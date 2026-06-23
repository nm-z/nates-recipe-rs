//! The OGDL checkpoint codec (read side): parse a saved-weights dump into one
//! `Saved` block per layer/neuron, in save order. The write side (`dump_ogdl`)
//! lives in the training crate; this half is all that inference needs to resume.

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
pub fn load_ogdl_str(text: &str) -> Vec<Saved> {
	let vals = |s: &str| -> Vec<f64> {
		s.split_whitespace()
			.map(|t| t.parse::<f64>().expect("resume: parse value"))
			.collect()
	};
	// A block accumulates several lines before it's complete, so collect into a
	// mutable `cur` and flush it on the next header (and at EOF).
	enum Cur {
		Embed(Vec<(usize, Vec<f64>)>),
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
	let flush = |cur: Option<Cur>, out: &mut Vec<Saved>| match cur {
		None => {}
		Some(Cur::Embed(mut rows)) => {
			rows.sort_by_key(|(id, _)| *id);
			out.push(Saved::Embed(
				rows.into_iter().flat_map(|(_, v)| v).collect(),
			));
		}
		Some(Cur::Attn {
			wq,
			wk,
			wv,
			wo,
			bq,
			bk,
			bv,
			bo,
		}) => {
			out.push(Saved::Attn {
				wq,
				wk,
				wv,
				wo,
				bq,
				bk,
				bv,
				bo,
			});
		}
		Some(Cur::Dense { w, b, a }) => out.push(Saved::Dense { w, b, a }),
		Some(Cur::Conv { w, b }) => out.push(Saved::Conv { w, b }),
	};
	let mut out: Vec<Saved> = Vec::new();
	let mut cur: Option<Cur> = None;
	for line in text.lines() {
		let t = line.trim();
		if t.is_empty() {
			continue;
		}
		match t.split_once('=') {
			// Bare token = block header: flush the previous block, open a new one.
			None => {
				flush(cur.take(), &mut out);
				cur = Some(if t == "embed" {
					Cur::Embed(Vec::new())
				} else if t == "attn" {
					Cur::Attn {
						wq: vec![],
						wk: vec![],
						wv: vec![],
						wo: vec![],
						bq: vec![],
						bk: vec![],
						bv: vec![],
						bo: vec![],
					}
				} else if t.starts_with("conv ") {
					Cur::Conv { w: vec![], b: vec![] }
				} else {
					Cur::Dense {
						w: Vec::new(),
						b: 0.0,
						a: None,
					} // z{k}
				});
			}
			Some((k, _)) if matches!(k.trim(), "r2" | "acc") => {}
			Some((k, v)) => {
				let key = k.trim();
				match cur
					.as_mut()
					.expect("resume: value line before any block header")
				{
					Cur::Embed(rows) => {
						rows.push((
							key.parse().expect("resume: embed row id"),
							vals(v),
						));
					}
					Cur::Attn {
						wq,
						wk,
						wv,
						wo,
						bq,
						bk,
						bv,
						bo,
					} => match key {
						"wq" => *wq = vals(v),
						"wk" => *wk = vals(v),
						"wv" => *wv = vals(v),
						"wo" => *wo = vals(v),
						"bq" => *bq = vals(v),
						"bk" => *bk = vals(v),
						"bv" => *bv = vals(v),
						"bo" => *bo = vals(v),
						_ => panic!("resume: unknown attn key {key}"),
					},
					Cur::Conv { w, b } => match key {
						"w" => *w = vals(v),
						"b" => *b = vals(v),
						_ => panic!("resume: unknown conv key {key}"),
					},
					Cur::Dense { w, b, a } => match key {
						"b" => *b = v.trim().parse().expect("resume: dense b"),
						"a" => {
							*a = Some(v
								.trim()
								.parse()
								.expect("resume: dense a"))
						}
						"w" => *w = vals(v),
						// Back-compat: the old format wrote one weight per line
						// (w1=, w2=, …) in order — append each to the vector.
						_ if key.starts_with('w')
							&& key[1..].chars().all(|c| c.is_ascii_digit())
							&& key.len() > 1 =>
						{
							w.push(v
								.trim()
								.parse()
								.expect("resume: dense w{n}"));
						}
						_ => {
							panic!(
								"resume: unrecognized key '{key}' — incompatible checkpoint; rm the .ogdl to start fresh"
							);
						}
					},
				}
			}
		}
	}
	flush(cur.take(), &mut out);
	out
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
