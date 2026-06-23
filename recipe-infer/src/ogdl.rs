//! The OGDL checkpoint codec: parse a saved-weights dump into one `Saved` block
//! per layer/neuron (read side, `load_ogdl`), and serialize a model's GPU buffers
//! back out (write side, `dump_ogdl`/`write_ogdl`/`saved_score`). Both halves live
//! here — serialization is inference-adjacent, not training: the trained params
//! are tensors, and turning tensors into the on-disk format needs nothing of
//! datasets or the training loop.

use crate::{Activation, LayerKind, LayerParams, Param, download_scalar, download_vec};

/// One parsed OGDL block, in layer/neuron order — the resume counterpart of the
/// per-layer save format. `Embed` is the flat [vocab*dim] token table; `Attn` holds
/// the four [d*d] projections and their (zero) [d] biases; `Dense` is one neuron's
/// weight row, bias, and optional learned PReLU slope `a`.
#[derive(Debug, PartialEq)]
pub enum Saved {
	Embed(Vec<f64>),
	Attn {
		wq: Vec<f64>,
		wk: Vec<f64>,
		wv: Vec<f64>,
		wo: Vec<f64>,
		bq: Vec<f64>,
		bk: Vec<f64>,
		bv: Vec<f64>,
		bo: Vec<f64>,
	},
	Dense {
		w: Vec<f64>,
		b: f64,
		a: Option<f64>,
	},
	Conv {
		w: Vec<f64>,
		b: Vec<f64>,
	},
}

impl Saved {
	/// Element count of this block (weights + biases), for the NaN-fraction report.
	pub fn len(&self) -> usize {
		match self {
			Saved::Embed(t) => t.len(),
			Saved::Attn {
				wq,
				wk,
				wv,
				wo,
				bq,
				bk,
				bv,
				bo,
			} => {
				wq.len()
					+ wk.len() + wv.len() + wo.len()
					+ bq.len() + bk.len() + bv.len()
					+ bo.len()
			}
			Saved::Dense { w, .. } => w.len() + 1,
			Saved::Conv { w, b } => w.len() + b.len(),
		}
	}
}

/// Parse an OGDL dump into one `Saved` block per layer/neuron, in save order
/// (embed table, attn projections+biases, or one dense neuron each). A missing
/// file is not an error: it just means "first run" — return empty so training
/// starts from random init and a later run can resume.
pub fn load_ogdl(path: &str) -> Vec<Saved> {
	let text = match std::fs::read_to_string(path) {
		Ok(t) => t,
		Err(_) => {
			eprintln!("no data in {path}, initialized random weights and biases");
			return Vec::new();
		}
	};
	load_ogdl_str(&text)
}

/// Parse OGDL checkpoint text into `Saved` blocks (the cwd-independent core of
/// `load_ogdl` — used by `Model::load` with `include_str!`-embedded weights).
/// The `ogdl` crate turns the text into a name/value tree; this fn interprets
/// what each top-level block means for model weights. The block layout is fixed
/// by `dump_ogdl`: a scalar metric header (`r2=`/`acc=`/…), then one bare-named
/// block per layer with its fields indented underneath.
pub fn load_ogdl_str(text: &str) -> Vec<Saved> {
	let vals = |s: &str| -> Vec<f64> {
		s.split_whitespace()
			.map(|t| t.parse::<f64>().expect("resume: parse value"))
			.collect()
	};
	let mut out: Vec<Saved> = Vec::new();
	for block in ogdl::Node::parse(text).children {
		// A top-level `key=value` line is the scalar metric header, not a weight
		// block — skip it whatever the metric is named.
		if block.value.is_some() {
			continue;
		}
		let field = |name: &str| -> Vec<f64> {
			block
				.children
				.iter()
				.find(|c| c.name == name)
				.and_then(|c| c.value.as_deref())
				.map_or_else(Vec::new, vals)
		};
		match block.name.as_str() {
			"embed" => {
				let mut rows: Vec<(usize, Vec<f64>)> = block
					.children
					.iter()
					.map(|c| {
						let v = c.value.as_deref().map_or_else(Vec::new, vals);
						(c.name.parse().expect("resume: embed row id"), v)
					})
					.collect();
				rows.sort_by_key(|(id, _)| *id);
				out.push(Saved::Embed(
					rows.into_iter().flat_map(|(_, v)| v).collect(),
				));
			}
			"attn" => out.push(Saved::Attn {
				wq: field("wq"),
				wk: field("wk"),
				wv: field("wv"),
				wo: field("wo"),
				bq: field("bq"),
				bk: field("bk"),
				bv: field("bv"),
				bo: field("bo"),
			}),
			name if name.starts_with("conv ") => {
				out.push(Saved::Conv { w: field("w"), b: field("b") })
			}
			// z{k}: one dense neuron — w row, scalar b, optional PReLU slope a.
			_ => {
				let mut w = Vec::new();
				let mut b = 0.0;
				let mut a = None;
				for c in &block.children {
					let v = c.value.as_deref().unwrap_or("");
					match c.name.as_str() {
						"w" => w = vals(v),
						"b" => b = v.trim().parse().expect("resume: dense b"),
						"a" => a = Some(v.trim().parse().expect("resume: dense a")),
						// Back-compat: the old format wrote one weight per line
						// (w1=, w2=, …) in order — append each to the vector.
						key if key.starts_with('w')
							&& key.len() > 1
							&& key[1..].chars().all(|c| c.is_ascii_digit()) =>
						{
							w.push(v.trim().parse().expect("resume: dense w{n}"));
						}
						key => panic!(
							"resume: unrecognized key '{key}' — incompatible checkpoint; rm the .ogdl to start fresh"
						),
					}
				}
				out.push(Saved::Dense { w, b, a });
			}
		}
	}
	out
}

/// One OGDL block per layer, in layer order: `embed` (one `{id}=` row per vocab
/// token), `attn` (`wq/wk/wv/wo` + `bq/bk/bv/bo`), `conv` (`w=`/`b=`), or one `z{k}`
/// block per dense neuron (`w=` row, `b=` scalar, plus `a=` for a PReLU layer's
/// learned slope). W rows are laid out to match `load_ogdl`'s distribution.
/// `filter: None` saves everything the model allocated (full checkpoint —
/// future-proof as new param kinds are added per layer below). `Some(parts)`
/// restricts to a subset. Each layer block downloads exactly the buffers it holds.
pub fn dump_ogdl(params: &[LayerParams], filter: Option<&[Param]>, key: &str, score: f64) -> String {
	let want_w = filter.map_or(true, |f| f.contains(&Param::W));
	let want_b = filter.map_or(true, |f| f.contains(&Param::B));
	let join = |v: &[f64]| {
		v.iter()
			.map(|x| x.to_string())
			.collect::<Vec<_>>()
			.join(" ")
	};
	let mut out = format!("{key}={score}\n");
	let mut z = 1;
	for p in params.iter() {
		match p.kind {
			LayerKind::Embed => {
				out.push_str("embed\n");
				if want_w {
					let table = download_vec(&p.w, p.vocab * p.dim);
					for id in 0..p.vocab {
						let row = &table[id * p.dim..(id + 1) * p.dim];
						out.push_str(&format!("    {id}={}\n", join(row)));
					}
				}
			}
			LayerKind::Attn => {
				out.push_str("attn\n");
				let dd = p.dim * p.dim;
				if want_w {
					for (nm, buf) in [
						("wq", &p.w),
						("wk", &p.wk),
						("wv", &p.wv),
						("wo", &p.wo),
					] {
						out.push_str(&format!(
							"    {nm}={}\n",
							join(&download_vec(buf, dd))
						));
					}
				}
				if want_b {
					// Bare attention has a single shared (zero) bias [d];
					// emit it as bq/bk/bv/bo for format completeness.
					let bias = download_vec(&p.b, p.dim);
					for nm in ["bq", "bk", "bv", "bo"] {
						out.push_str(&format!("    {nm}={}\n", join(&bias)));
					}
				}
			}
			LayerKind::Conv => {
				let lin = p.in_dim / p.conv_cin;
				let lout = (lin - p.conv_k) / p.conv_stride + 1;
				let cout = p.out_dim / lout;
				let w_count = cout * p.conv_cin * p.conv_k;
				out.push_str(&format!("conv {} {} {} {}\n", cout, p.conv_cin, p.conv_k, p.conv_stride));
				if want_w {
					let w = download_vec(&p.w, w_count);
					out.push_str(&format!("    w={}\n", join(&w)));
				}
				if want_b {
					let b = download_vec(&p.b, cout);
					out.push_str(&format!("    b={}\n", join(&b)));
				}
			}
			LayerKind::Dense => {
				let w = download_vec(&p.w, p.in_dim * p.out_dim);
				let b = download_vec(&p.b, p.out_dim);
				let slope = (p.act == Activation::PRelu)
					.then(|| download_scalar(&p.palpha));
				for j in 0..p.out_dim {
					out.push_str(&format!("z{z}\n"));
					if want_w {
						let row: Vec<f64> = (0..p.in_dim)
							.map(|i| w[i * p.out_dim + j])
							.collect();
						out.push_str(&format!("    w={}\n", join(&row)));
						if let Some(a) = slope {
							out.push_str(&format!("    a={a}\n"));
						}
					}
					if want_b {
						out.push_str(&format!("    b={}\n", b[j]));
					}
					z += 1;
				}
			}
		}
	}
	out
}

/// Write OGDL text, creating any missing parent dirs — saving should make the
/// file, not fail because the directory isn't there yet.
pub fn write_ogdl(path: &str, out: &str) {
	if let Some(parent) = std::path::Path::new(path).parent()
		&& !parent.as_os_str().is_empty()
	{
		std::fs::create_dir_all(parent)
			.unwrap_or_else(|e| panic!("save: mkdir {}: {e}", parent.display()));
	}
	std::fs::write(path, out).unwrap_or_else(|e| panic!("save: write {path}: {e}"));
}

/// Read the score recorded on the first line of a saved checkpoint (`{key}={score}`),
/// used by the best-only save guard. `None` if the file is absent or unparseable.
pub fn saved_score(path: &str, key: &str) -> Option<f64> {
	let text = std::fs::read_to_string(path).ok()?;
	for line in text.lines() {
		if let Some((k, v)) = line.trim().split_once('=')
			&& k.trim() == key
		{
			return v.trim().parse().ok();
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;

	// Host-only: the OGDL parser must read back the documented embed/attn/dense
	// format exactly (no GPU — pure file parse). Mirrors what dump_ogdl writes:
	// an embed table by token id, attn projections + zero biases, and dense
	// neurons with optional PReLU slope `a`.
	#[test]
	fn ogdl_format_roundtrips_host_side() {
		let path = std::env::temp_dir().join("nrs_ogdl_roundtrip.ogdl");
		let text = "\
r2=0.42
embed
    0=-0.0312 0.1847 -0.0551
    1=0.0892 -0.2104 0.0033
attn
    wq=1 2 3 4
    wk=5 6 7 8
    wv=9 10 11 12
    wo=13 14 15 16
    bq=0 0
    bk=0 0
    bv=0 0
    bo=0 0
z1
    w=0.01 -0.02 0.03
    b=0.001
z2
    w=0.04 0.05 0.06
    a=0.25
    b=0.002
";
		std::fs::write(&path, text).expect("write tmp ogdl");
		let parsed = load_ogdl(path.to_str().expect("utf8 path"));
		std::fs::remove_file(&path).ok();
		assert_eq!(parsed.len(), 4);
		assert_eq!(
			parsed[0],
			Saved::Embed(vec![-0.0312, 0.1847, -0.0551, 0.0892, -0.2104, 0.0033])
		);
		assert_eq!(
			parsed[1],
			Saved::Attn {
				wq: vec![1.0, 2.0, 3.0, 4.0],
				wk: vec![5.0, 6.0, 7.0, 8.0],
				wv: vec![9.0, 10.0, 11.0, 12.0],
				wo: vec![13.0, 14.0, 15.0, 16.0],
				bq: vec![0.0, 0.0],
				bk: vec![0.0, 0.0],
				bv: vec![0.0, 0.0],
				bo: vec![0.0, 0.0],
			}
		);
		assert_eq!(
			parsed[2],
			Saved::Dense {
				w: vec![0.01, -0.02, 0.03],
				b: 0.001,
				a: None
			}
		);
		assert_eq!(
			parsed[3],
			Saved::Dense {
				w: vec![0.04, 0.05, 0.06],
				b: 0.002,
				a: Some(0.25)
			}
		);
	}
}
