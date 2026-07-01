//! All predictor/target encoding: detected column `Kind`s → numeric matrices,
//! the multi-file assemble/join, RAM guards, NaN handling, and the train/test
//! split. Pure data work — turns parsed rows (from `data`) into a `Dataset`. The
//! trainer crate above interprets what that `Dataset` means for a model; this
//! module knows nothing of models, GPUs, or the forward pass.

use crate::data::DirGroup;
use crate::{Attr, Kind, Mat, Vec1};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

/// The encoded numeric result handed to the trainer: feature matrix `x`, flat
/// `n*n_targets` target vector `y`, and the column metadata the model needs.
pub struct Dataset {
	pub x: Mat,
	pub y: Vec1,
	pub source: String,
	pub n_targets: usize,
	pub has_target: bool,
	pub text_cols: Vec<usize>,
	pub onehot_groups: Vec<(usize, usize)>,
}

fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
	s.split(|c: char| !c.is_alphanumeric())
		.filter(|t| !t.is_empty())
		.map(|t| t.to_ascii_lowercase())
}

/// Filename → stem (drop directory + extension) so a CSV cell like
/// `train/train_0001.png` matches an image vector keyed by `train_0001`.
fn file_stem(s: &str) -> &str {
	std::path::Path::new(s)
		.file_stem()
		.and_then(|x| x.to_str())
		.unwrap_or(s)
}

fn cell(row: &[String], j: usize) -> &str {
	row.get(j).map_or("", |s| s.as_str())
}

fn date_to_f64(s: &str) -> f64 {
	let parts: Vec<&str> = s
		.split(|c: char| c == '-' || c == '/' || c == 'T' || c == ' ')
		.collect();
	if parts.len() >= 3 {
		if let (Ok(y), Ok(m), Ok(d)) = (
			parts[0].parse::<f64>(),
			parts[1].parse::<f64>(),
			parts[2].parse::<f64>(),
		) {
			return y * 365.25 + m * 30.44 + d;
		}
	}
	f64::NAN
}

fn is_missing(c: &str) -> bool {
	c.is_empty()
		|| matches!(
			c,
			"NA" | "NaN" | "nan" | "N/A" | "NULL" | "null" | "None" | "none" | "?" | "." | "-"
		)
}

fn col_vocab(rows: &[Vec<String>], j: usize) -> Vec<String> {
	let set = rows
		.par_iter()
		.fold(std::collections::HashSet::<String>::new, |mut acc, row| {
			acc.extend(tokenize(cell(row, j)));
			acc
		})
		.reduce(std::collections::HashSet::<String>::new, |a, b| {
			let (mut big, small) = if a.len() >= b.len() { (a, b) } else { (b, a) };
			big.extend(small);
			big
		});
	let mut vocab: Vec<String> = set.into_iter().collect();
	vocab.par_sort_unstable();
	vocab
}

fn distinct_sorted(rows: &[Vec<String>], j: usize) -> Vec<String> {
	let mut cats: Vec<String> = rows
		.iter()
		.map(|row| cell(row, j))
		.filter(|c| !is_missing(c))
		.map(str::to_string)
		.collect::<std::collections::HashSet<_>>()
		.into_iter()
		.collect();
	cats.sort_unstable();
	cats
}

fn infer_attrs(headers: &[String], rows: &[Vec<String>], known: Option<&[Attr]>) -> Vec<Attr> {
	let non_empty: Vec<Vec<&str>> = (0..headers.len())
		.map(|j| {
			rows.iter()
				.map(|row| cell(row, j))
				.filter(|c| !is_missing(c))
				.collect()
		})
		.collect();
	let to_predict: Vec<usize> = (0..headers.len())
		.filter(|&j| known.and_then(|k| k.get(j)).is_none() && !non_empty[j].is_empty())
		.collect();
	let cols: Vec<Vec<&str>> = to_predict.iter().map(|&j| non_empty[j].clone()).collect();
	let preds = crate::predict_kinds(&cols);
	let mut pred = std::collections::HashMap::new();
	for (i, &j) in to_predict.iter().enumerate() {
		pred.insert(j, preds[i]);
	}
	headers
		.iter()
		.enumerate()
		.map(|(j, name)| {
			let kind = if let Some(sa) = known.and_then(|k| k.get(j)) {
				sa.kind.clone()
			} else if non_empty[j].is_empty() {
				Kind::Numeric
			} else {
				match pred[&j] {
					crate::KIND_NUMERIC => Kind::Numeric,
					crate::KIND_TEMPORAL => Kind::Temporal,
					crate::KIND_CATEGORICAL => Kind::Categorical(distinct_sorted(rows, j)),
					crate::KIND_ORDINAL => Kind::Ordinal(distinct_sorted(rows, j)),
					crate::KIND_TEXT => Kind::Text(col_vocab(rows, j)),
					_ => Kind::Image,
				}
			};
			Attr { name: name.clone(), kind }
		})
		.collect()
}

/// Encode one column purely by its `Kind` — identical whether the column is a
/// feature or a target. Role only decides where the produced columns are routed
/// (X vs Y), never how they're encoded.
fn encode_kind(
	attr: &Attr,
	rows: &[Vec<String>],
	ai: usize,
	seq_len: usize,
) -> (Vec<String>, Vec<Vec<f64>>) {
	let n = rows.len();
	match &attr.kind {
		Kind::Numeric => {
			let mut col = vec![0.0f64; n];
			for (r, row) in rows.iter().enumerate() {
				col[r] = cell(row, ai).parse::<f64>().unwrap_or(f64::NAN);
			}
			(vec![attr.name.clone()], vec![col])
		}
		Kind::Temporal => {
			let mut col = vec![0.0f64; n];
			for (r, row) in rows.iter().enumerate() {
				let c = cell(row, ai);
				col[r] = c.parse::<f64>().unwrap_or_else(|_| date_to_f64(c));
			}
			(vec![attr.name.clone()], vec![col])
		}
		Kind::Categorical(cats) => {
			let mut names = Vec::with_capacity(cats.len());
			let mut cols = Vec::with_capacity(cats.len());
			for cat in cats {
				names.push(format!("{}={cat}", attr.name));
				let mut col = vec![0.0f64; n];
				for (r, row) in rows.iter().enumerate() {
					if cell(row, ai) == cat {
						col[r] = 1.0;
					}
				}
				cols.push(col);
			}
			(names, cols)
		}
		Kind::Ordinal(cats) => {
			let mut col = vec![f64::NAN; n];
			for (r, row) in rows.iter().enumerate() {
				let v = cell(row, ai);
				if let Some(p) = cats.iter().position(|c| c == v) {
					col[r] = p as f64;
				}
			}
			(vec![attr.name.clone()], vec![col])
		}
		Kind::Text(vocab) => {
			let names = (0..seq_len).map(|s| format!("{}#t{s}", attr.name)).collect();
			let per_row: Vec<Vec<f64>> = rows
				.par_iter()
				.map(|row| {
					let mut ids = vec![0.0f64; seq_len];
					for (s, tok) in tokenize(cell(row, ai)).take(seq_len).enumerate() {
						ids[s] = vocab.binary_search(&tok).map_or(0.0, |p| (p + 1) as f64);
					}
					ids
				})
				.collect();
			let mut cols = vec![vec![0.0f64; n]; seq_len];
			for (r, ids) in per_row.iter().enumerate() {
				for s in 0..seq_len {
					cols[s][r] = ids[s];
				}
			}
			(names, cols)
		}
		// An image column holds filenames — it is the JOIN KEY into an image vector
		// (handled in `assemble`), not a feature itself, so it emits no columns.
		Kind::Image => (Vec::new(), Vec::new()),
	}
}

fn encode(
	attrs: &[Attr],
	rows: &[Vec<String>],
	targets: &[usize],
	skip: &[bool],
) -> (Vec<String>, Mat, Vec1, usize) {
	let n = rows.len();

	let is_target = |ai: usize| targets.contains(&ai);
	let is_skip = |ai: usize| skip.get(ai).copied().unwrap_or(false);
	let text_seq_lens: Vec<usize> = attrs
		.iter()
		.enumerate()
		.map(|(ai, a)| match &a.kind {
			Kind::Text(_) if !is_skip(ai) => rows
				.iter()
				.map(|row| tokenize(cell(row, ai)).count())
				.max()
				.unwrap_or(1),
			_ => 0,
		})
		.collect();
	let width = |ai: usize, a: &Attr| match &a.kind {
		Kind::Numeric | Kind::Temporal | Kind::Ordinal(_) => 1,
		Kind::Categorical(c) => c.len(),
		Kind::Text(_) => text_seq_lens[ai].max(1),
		// Image columns are join keys, not features — zero feature width.
		Kind::Image => 0,
	};
	let proj_w: usize = attrs
		.iter()
		.enumerate()
		.filter(|(ai, _)| !is_target(*ai) && !is_skip(*ai))
		.map(|(ai, a)| width(ai, a))
		.sum();
	let top: Vec<(&str, usize)> = attrs
		.iter()
		.enumerate()
		.filter(|(ai, a)| !is_target(*ai) && !is_skip(*ai) && width(*ai, a) > 1)
		.map(|(ai, a)| (a.name.as_str(), width(ai, a)))
		.collect();
	check_ram(n, proj_w, "encoded", &top);

	let mut names: Vec<String> = Vec::with_capacity(proj_w);
	let mut cols: Vec<Vec<f64>> = Vec::with_capacity(proj_w);
	let mut tcols: Vec<Vec<Vec<f64>>> = vec![Vec::new(); targets.len()];
	for (ai, attr) in attrs.iter().enumerate() {
		if is_skip(ai) && !is_target(ai) {
			continue;
		}
		// A categorical TARGET encodes to ONE class-index column (0..N-1), not a
		// one-hot — the trainer expands it to the model's output width for CE (so a
		// declared class count above what the data shows still works). Features keep
		// their one-hot encoding (role decides target-index vs feature-one-hot here).
		let (cnames, ccols) = match (&attr.kind, is_target(ai)) {
			(Kind::Categorical(cats), true) => {
				let mut col = vec![f64::NAN; n];
				for (r, row) in rows.iter().enumerate() {
					if let Some(p) = cats.iter().position(|c| c == cell(row, ai)) {
						col[r] = p as f64;
					}
				}
				(vec![attr.name.clone()], vec![col])
			}
			_ => encode_kind(attr, rows, ai, width(ai, attr)),
		};
		match targets.iter().position(|&t| t == ai) {
			Some(tj) => tcols[tj] = ccols,
			None => {
				names.extend(cnames);
				cols.extend(ccols);
			}
		}
	}
	let ycols: Vec<Vec<f64>> = tcols.into_iter().flatten().collect();
	let k = ycols.len();
	let w = cols.len();
	let mut data = vec![0.0f64; n * w];
	for (j, col) in cols.iter().enumerate() {
		for (i, v) in col.iter().enumerate() {
			data[i * w + j] = *v;
		}
	}
	let mut ydata = vec![0.0f64; n * k];
	for (j, col) in ycols.iter().enumerate() {
		for (i, v) in col.iter().enumerate() {
			ydata[i * k + j] = *v;
		}
	}
	(
		names,
		Mat::from_shape_vec((n, w), data).expect("encode: reshape"),
		Vec1::from(ydata),
		k,
	)
}

fn oom_pair(name: &str, val: &str) -> String {
	format!("\x1b[1;31m{name}:\x1b[0m \x1b[1m{val}\x1b[0m")
}

/// Single RAM guard for both the per-group encode and the cross-group selection:
/// if `n × w × 8B` would exceed 90% of available memory, print a one-line memory
/// autopsy (largest bucket first) built from `top_cols`, then panic. `label`
/// names the matrix in the message ("encoded" vs "selection").
fn check_ram(n: usize, w: usize, label: &str, top_cols: &[(&str, usize)]) {
	let bytes = n
		.saturating_mul(w)
		.saturating_mul(std::mem::size_of::<f64>());
	let avail = crate::available_ram_bytes();
	if bytes <= avail / 10 * 9 {
		return;
	}
	let hb = crate::data::human_bytes;
	let cols_bytes = |c: usize| c.saturating_mul(n).saturating_mul(8);
	let tokens_cols: usize = top_cols
		.iter()
		.filter(|(nm, _)| nm.contains("#t"))
		.map(|(_, c)| *c)
		.sum();
	let onehot_cols: usize = top_cols.iter().filter(|(_, c)| *c > 1).map(|(_, c)| *c).sum();
	let scalar_cols: usize = top_cols
		.iter()
		.filter(|(nm, c)| *c == 1 && !nm.contains("#t"))
		.map(|(_, c)| *c)
		.sum();
	let mut autopsy: Vec<(&str, usize)> =
		[("tokens", tokens_cols), ("scalar", scalar_cols), ("onehot", onehot_cols)]
			.into_iter()
			.filter(|(_, c)| *c > 0)
			.collect();
	autopsy.sort_by(|a, b| cols_bytes(b.1).cmp(&cols_bytes(a.1)));
	let mut line: Vec<String> = autopsy
		.iter()
		.map(|(nm, c)| oom_pair(nm, &format!("{} ({c})", hb(cols_bytes(*c)))))
		.collect();
	let mut bases: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
	for (nm, _) in top_cols.iter().filter(|(nm, _)| nm.contains("#t")) {
		*bases.entry(nm.split("#t").next().unwrap_or(nm)).or_insert(0) += 1;
	}
	line.push(oom_pair("rows", &n.to_string()));
	line.push(oom_pair("free", &hb(avail)));
	line.push(oom_pair("over", &hb(bytes.saturating_sub(avail))));
	if let Some((base, seq)) = bases.into_iter().max_by_key(|(_, c)| *c) {
		line.push(oom_pair("widest", &format!("{base}×{seq}")));
	}
	eprintln!("{}", line.join(", "));
	panic!(
		"{label} matrix too large for RAM: {n} rows × {w} cols × 8B = {} (available {})",
		crate::data::human_bytes(bytes),
		crate::data::human_bytes(avail)
	);
}

type Schema = std::collections::BTreeMap<String, Vec<Attr>>;

struct Assembled {
	names: Vec<String>,
	sources: Vec<(usize, usize)>,
	mats: Vec<Mat>,
	gather: Vec<Vec<Option<usize>>>,
	y: Vec1,
	n_targets: usize,
	samples: usize,
	#[allow(dead_code)]
	skipped: Vec<String>,
	sample_group: String,
}

impl Assembled {
	fn select(&self, keep: &[String]) -> Mat {
		let n = self.samples;
		let w = keep.len();
		let mut by_col: std::collections::BTreeMap<&str, usize> =
			std::collections::BTreeMap::new();
		for name in keep {
			*by_col
				.entry(name.split('=').next().unwrap_or(name))
				.or_insert(0) += 1;
		}
		let top: Vec<(&str, usize)> = by_col.into_iter().collect();
		check_ram(n, w, "selection", &top);
		let idx: std::collections::HashMap<&str, usize> = self
			.names
			.iter()
			.enumerate()
			.map(|(i, s)| (s.as_str(), i))
			.collect();
		let mut data = vec![0.0f64; n * w];
		for (jc, name) in keep.iter().enumerate() {
			let (mi, col) = self.sources[idx[name.as_str()]];
			let m = &self.mats[mi];
			let g = &self.gather[mi];
			for i in 0..n {
				data[i * w + jc] = g[i].map_or(f64::NAN, |r| m[[r, col]]);
			}
		}
		Mat::from_shape_vec((n, w), data).expect("select reshape")
	}
}

fn namespaced(group: &str, col: &str) -> String {
	if group.is_empty() {
		col.to_string()
	} else {
		format!("{group}:{col}")
	}
}

fn table_names(groups: &[DirGroup]) -> Vec<String> {
	let mut out = Vec::new();
	for g in groups {
		if let DirGroup::Table { name, headers, .. } = g {
			for h in headers {
				out.push(namespaced(name, h));
			}
		}
	}
	out
}

fn encode_group(
	g: &DirGroup,
	schema: &mut Schema,
	schema_in: Option<&Schema>,
	target_cols: &[usize],
	exclude: &[String],
) -> (Vec<String>, Mat, Vec1, usize) {
	match g {
		DirGroup::Table {
			name,
			headers,
			cells,
			..
		} => {
			let attrs = infer_attrs(
				headers,
				cells,
				schema_in.and_then(|s| s.get(name)).map(Vec::as_slice),
			);
			schema.insert(name.clone(), attrs.clone());

			let skip = exclude_mask(&attrs, name, exclude);
			let (fnames, x, y, k) = encode(&attrs, cells, target_cols, &skip);
			let names = fnames.iter().map(|f| namespaced(name, f)).collect();
			(names, x, y, k)
		}
		DirGroup::Image {
			name, dim, pixels, ..
		} => {
			let n = pixels.len();
			let mut data = vec![0.0f64; n * dim];
			for (i, px) in pixels.iter().enumerate() {
				for (j, v) in px.iter().take(*dim).enumerate() {
					data[i * dim + j] = *v;
				}
			}
			let names = (0..*dim)
				.map(|i| namespaced(name, &format!("px{i}")))
				.collect();
			let x = Mat::from_shape_vec((n, *dim), data).expect("image reshape");
			(names, x, Vec1::zeros(n), 0)
		}
	}
}

fn group_hashes(g: &DirGroup) -> &[String] {
	match g {
		DirGroup::Table { hashes, .. } | DirGroup::Image { hashes, .. } => hashes,
	}
}

fn assemble(
	groups: &[DirGroup],
	targets: &[String],
	schema_in: Option<&Schema>,
	sample_hint: Option<&str>,
	exclude: &[String],
) -> (Assembled, Schema) {
	let mut schema: Schema = Schema::new();

	let mut sample_idx = 0usize;
	let mut target_cols: Vec<usize> = Vec::new();
	if !targets.is_empty() {
		for (gi, g) in groups.iter().enumerate() {
			if let DirGroup::Table { name, headers, .. } = g {
				let cols: Vec<usize> = targets
					.iter()
					.filter_map(|t| {
						headers.iter().position(|h| namespaced(name, h) == *t)
					})
					.collect();
				if !cols.is_empty() {
					sample_idx = gi;
					target_cols = cols;
					break;
				}
			}
		}
	}

	if target_cols.is_empty() {
		let by_hint = sample_hint.and_then(|h| groups.iter().position(|g| group_name(g) == h));
		sample_idx = match by_hint {
			Some(i) => i,
			None => {
				let tables: Vec<usize> = groups
					.iter()
					.enumerate()
					.filter(|(_, g)| matches!(g, DirGroup::Table { .. }))
					.map(|(i, _)| i)
					.collect();
				match (groups.len(), tables.as_slice()) {
					(1, _) => 0,
					(_, [only]) => *only,
					_ => panic!(
						"multiple groups and no resolvable target — name it with .target() so the sample file is known"
					),
				}
			}
		};
	}
	let (s_names, s_x, y, n_targets) = encode_group(
		&groups[sample_idx],
		&mut schema,
		schema_in,
		&target_cols,
		exclude,
	);
	let s_hashes = group_hashes(&groups[sample_idx]);
	let n = s_x.nrows();
	// Raw sample cells — an image vector joins by matching the filename it holds in
	// one of these columns (the "column of filenames" in the user's abstraction).
	let sample_cells: Option<&[Vec<String>]> = match &groups[sample_idx] {
		DirGroup::Table { cells, .. } => Some(cells.as_slice()),
		_ => None,
	};

	let mut names: Vec<String> = Vec::new();
	let mut sources: Vec<(usize, usize)> = Vec::new();
	let mut mats: Vec<Mat> = Vec::new();
	let mut gather: Vec<Vec<Option<usize>>> = Vec::new();
	let mut skipped: Vec<String> = Vec::new();

	for (j, nm) in s_names.iter().enumerate() {
		names.push(nm.clone());
		sources.push((0, j));
	}
	gather.push((0..n).map(Some).collect());
	mats.push(s_x);

	let mut s_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
	for h in s_hashes {
		*s_count.entry(h.as_str()).or_insert(0) += 1;
	}
	let mut s_pos = vec![0usize; n];
	{
		let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
		for (i, h) in s_hashes.iter().enumerate() {
			let c = seen.entry(h.as_str()).or_insert(0);
			s_pos[i] = *c;
			*c += 1;
		}
	}

	for (gi, g) in groups.iter().enumerate() {
		if gi == sample_idx {
			continue;
		}

		// Image vector ⋈ filename column: a dir of files is a vector indexed by
		// filename; a sample column of filenames is a vector of those keys. Pick the
		// sample column whose cell stems best match this image vector's filenames,
		// then gather each row's image by that key (index = filename, data = image).
		if let (DirGroup::Image { hashes: g_hashes, .. }, Some(cells)) = (g, sample_cells) {
			let key_set: std::collections::HashSet<&str> =
				g_hashes.iter().map(String::as_str).collect();
			let ncols = cells.first().map_or(0, Vec::len);
			let (mut best_col, mut best) = (0usize, 0usize);
			for c in 0..ncols {
				let hits = cells
					.iter()
					.filter(|r| key_set.contains(file_stem(r.get(c).map_or("", String::as_str))))
					.count();
				if hits > best {
					best = hits;
					best_col = c;
				}
			}
			if best > 0 {
				let by_key: std::collections::HashMap<&str, usize> =
					g_hashes.iter().enumerate().map(|(i, h)| (h.as_str(), i)).collect();
				let (g_names, g_x, _gy, _gk) =
					encode_group(g, &mut schema, schema_in, &[], exclude);
				let src: Vec<Option<usize>> = (0..n)
					.map(|i| {
						by_key
							.get(file_stem(cells[i].get(best_col).map_or("", String::as_str)))
							.copied()
					})
					.collect();
				let mi = mats.len();
				for (j, nm) in g_names.iter().enumerate() {
					names.push(nm.clone());
					sources.push((mi, j));
				}
				gather.push(src);
				mats.push(g_x);
				continue;
			}
		}

		let g_hashes = group_hashes(g);
		let mut by_hash: std::collections::HashMap<&str, Vec<usize>> =
			std::collections::HashMap::new();
		for (gi2, h) in g_hashes.iter().enumerate() {
			by_hash.entry(h.as_str()).or_default().push(gi2);
		}

		let shares = by_hash.keys().any(|h| s_count.contains_key(*h));
		if !shares {
			skipped.push(format!(
                        "{}: shares no join key with the sample group — separate table, use .exclude() or join manually",
                        group_name(g)
                  ));
			continue;
		}
		let all_one = by_hash.values().all(|v| v.len() == 1);

		let aligns = s_count
			.iter()
			.all(|(h, &sc)| by_hash.get(*h).is_none_or(|v| v.len() == sc));
		if !all_one && !aligns {
			skipped.push(format!(
                        "{}: {} rows across {} hashes don't hash-align to the sample group ({} samples) — excluded, use .exclude() or join manually",
                        group_name(g), g_hashes.len(), by_hash.len(), n
                  ));
			continue;
		}

		let (g_names, g_x, _gy, _gk) = encode_group(g, &mut schema, schema_in, &[], exclude);

		let src: Vec<Option<usize>> = (0..n)
			.map(|i| {
				let h = s_hashes[i].as_str();
				by_hash.get(h).and_then(|v| {
					if all_one {
						v.first().copied()
					} else {
						v.get(s_pos[i]).copied()
					}
				})
			})
			.collect();
		let mi = mats.len();
		for (j, nm) in g_names.iter().enumerate() {
			names.push(nm.clone());
			sources.push((mi, j));
		}
		gather.push(src);
		mats.push(g_x);
	}

	(
		Assembled {
			names,
			sources,
			mats,
			gather,
			y,
			n_targets,
			samples: n,
			skipped,
			sample_group: group_name(&groups[sample_idx]).to_string(),
		},
		schema,
	)
}

fn group_name(g: &DirGroup) -> &str {
	match g {
		DirGroup::Table { name, .. } | DirGroup::Image { name, .. } => name,
	}
}

pub fn exclude_mask(attrs: &[Attr], group: &str, exclude: &[String]) -> Vec<bool> {
	attrs
		.iter()
		.map(|a| {
			let nm = namespaced(group, &a.name);
			exclude.iter().any(|p| exclude_match(p, &nm))
		})
		.collect()
}

pub fn exclude_match(pattern: &str, name: &str) -> bool {
	if pattern == name {
		return true;
	}
	if let Some(g) = pattern.strip_suffix(":*") {
		return name.split_once(':').map(|(ng, _)| ng) == Some(g);
	}
	if name.split_once(':').map(|(ng, _)| ng) == Some(pattern) {
		return true;
	}
	col_after(name) == pattern
}

fn col_after(c: &str) -> &str {
	c.split_once(':').map_or(c, |(_, s)| s)
}

pub fn shuffle_split(
	x: &Mat,
	y: &Vec1,
	k: usize,
	train_frac: f64,
	source: &str,
	text_cols: &[usize],
	onehot_groups: &[(usize, usize)],
) -> (Dataset, Dataset) {
	let n = x.nrows();
	let mut idx: Vec<usize> = (0..n).collect();
	idx.shuffle(&mut ChaCha8Rng::seed_from_u64(42));
	let n_train = (n as f64 * train_frac).round() as usize;
	let cols = x.ncols();
	let take = |sel: &[usize]| -> Dataset {
		let mut xd = Vec::with_capacity(sel.len() * cols);
		let mut yd = Vec::with_capacity(sel.len() * k);
		for &i in sel {
			xd.extend(x.row(i).iter().copied());
			yd.extend((0..k).map(|j| y[i * k + j]));
		}
		Dataset {
			x: Mat::from_shape_vec((sel.len(), cols), xd).expect("split: x reshape"),
			y: Vec1::from(yd),
			source: source.to_string(),
			n_targets: k,
			has_target: true,
			text_cols: text_cols.to_vec(),
			onehot_groups: onehot_groups.to_vec(),
		}
	};
	(take(&idx[..n_train]), take(&idx[n_train..]))
}

fn text_col_indices(feats: &[String]) -> Vec<usize> {
	feats
		.iter()
		.enumerate()
		.filter(|(_, n)| n.contains("#t"))
		.map(|(i, _)| i)
		.collect()
}

fn onehot_group_indices(feats: &[String]) -> Vec<(usize, usize)> {
	let mut groups = Vec::new();
	let mut i = 0;
	while i < feats.len() {
		if let Some(eq) = feats[i].find('=') {
			let prefix = &feats[i][..=eq];
			let start = i;
			while i < feats.len() && feats[i].starts_with(prefix) {
				i += 1;
			}
			groups.push((start, i - start));
		} else {
			i += 1;
		}
	}
	groups
}

/// The single NaN strategy: `Drop` reports the finite rows (caller removes them
/// from every column), `ImputeMean` fills NaN with the column's finite mean in
/// place, `Error` panics on any NaN.
pub enum Nan {
	Drop,
	ImputeMean,
	Error,
}

/// THE one NaN-handling function — applied to a single column-vector. Returns the
/// row indices to keep (every row except for `Drop`, which keeps only finite ones).
pub fn nan_clean(v: &mut [f64], strategy: Nan, name: &str) -> Vec<usize> {
	match strategy {
		Nan::ImputeMean => {
			let (mut sum, mut cnt) = (0.0f64, 0usize);
			for &x in v.iter() {
				if x.is_finite() {
					sum += x;
					cnt += 1;
				}
			}
			let mean = if cnt > 0 { sum / cnt as f64 } else { 0.0 };
			for x in v.iter_mut() {
				if !x.is_finite() {
					*x = mean;
				}
			}
			(0..v.len()).collect()
		}
		Nan::Error => {
			assert!(
				v.iter().all(|x| x.is_finite()),
				"NaN/inf in '{name}' — no missing values allowed here"
			);
			(0..v.len()).collect()
		}
		Nan::Drop => (0..v.len()).filter(|&i| v[i].is_finite()).collect(),
	}
}

/// The ONE call site: apply the NaN policy once per column-vector as a dataset
/// enters the numeric pipeline. Targets use `Drop` (a missing label can't be
/// invented); features use `ImputeMean`. Afterwards the matrix holds no NaN, so
/// nothing downstream handles NaN again.
pub fn clean_dataset(d: &mut Dataset) {
	let k = d.n_targets.max(1);
	let n = d.x.nrows();
	let mut keep: Vec<usize> = (0..n).collect();
	for j in 0..k {
		let mut col: Vec<f64> = (0..n).map(|i| d.y[i * k + j]).collect();
		let kj = nan_clean(&mut col, Nan::Drop, "target");
		keep.retain(|i| kj.binary_search(i).is_ok());
	}
	if keep.len() < n {
		eprintln!("\x1b[32mnan\x1b[0m  dropped {} row(s) with a missing target", n - keep.len());
		d.x = d.x.select(ndarray::Axis(0), &keep);
		let mut yd = Vec::with_capacity(keep.len() * k);
		for &i in &keep {
			for j in 0..k {
				yd.push(d.y[i * k + j]);
			}
		}
		d.y = Vec1::from(yd);
	}
	let (rows, cols) = (d.x.nrows(), d.x.ncols());
	for j in 0..cols {
		let mut col: Vec<f64> = (0..rows).map(|i| d.x[(i, j)]).collect();
		nan_clean(&mut col, Nan::ImputeMean, "feature");
		for i in 0..rows {
			d.x[(i, j)] = col[i];
		}
	}
}

/// ARFF / pre-parsed path: `attrs` + `rows` are already in hand (the loader ran
/// up in the builder), so encode directly, optionally splitting or encoding a
/// separate test file the same way.
pub fn prepare_arff_data(
	attrs: &[Attr],
	rows: &[Vec<String>],
	targets: &[usize],
	exclude: &[String],
	split_frac: Option<f64>,
	test_path: Option<&str>,
	source_label: &str,
) -> (Dataset, Option<Dataset>) {
	let skip = exclude_mask(attrs, "", exclude);
	let (names, x, y, enc_k) = encode(attrs, rows, targets, &skip);
	let k = enc_k.max(1);
	let tc = text_col_indices(&names);
	let oh = onehot_group_indices(&names);
	if let Some(frac) = split_frac {
		let (tr, te) = shuffle_split(&x, &y, k, frac, source_label, &tc, &oh);
		(tr, Some(te))
	} else if let Some(tp) = test_path {
		let (_, trows) = crate::data::parse_arff(tp);
		let (_, tx, ty, _) = encode(attrs, &trows, targets, &skip);
		(
			Dataset {
				x,
				y,
				source: source_label.to_string(),
				n_targets: k,
				has_target: true,
				text_cols: tc.clone(),
				onehot_groups: oh.clone(),
			},
			Some(Dataset {
				x: tx,
				y: ty,
				source: tp.to_string(),
				n_targets: k,
				has_target: true,
				text_cols: tc,
				onehot_groups: oh,
			}),
		)
	} else {
		(
			Dataset {
				x,
				y,
				source: source_label.to_string(),
				n_targets: k,
				has_target: true,
				text_cols: tc,
				onehot_groups: oh,
			},
			None,
		)
	}
}

/// Table path (CSV/dir/zip): load groups, resolve targets (via the caller's
/// `resolve` closure, which lives up in the builder), assemble + join, select
/// the kept feature columns, and optionally split or align a separate test set.
pub fn prepare_table_data(
	sources: &[String],
	test_path: Option<&str>,
	split_frac: Option<f64>,
	exclude: &[String],
	source_label: &str,
	resolve: impl Fn(&[String], Option<&[String]>) -> Vec<String>,
) -> (Dataset, Option<Dataset>, Vec<Attr>) {
	let set_groups: Vec<DirGroup> = sources
		.iter()
		.flat_map(|s| crate::data::load_groups(s))
		.collect();
	let set_tnames = table_names(&set_groups);

	let test_groups = test_path.map(|tp| (crate::data::load_groups(tp), tp.to_string()));
	let test_tnames: Option<Vec<String>> = match (&test_groups, split_frac) {
		(Some((g, _)), _) => Some(table_names(g)),
		(None, Some(_)) => Some(set_tnames.clone()),
		(None, None) => None,
	};
	let t = resolve(&set_tnames, test_tnames.as_deref());

	let (set, schema) = assemble(&set_groups, &t, None, None, exclude);
	let flat_attrs: Vec<Attr> = schema.values().flat_map(|v| v.iter().cloned()).collect();
	let k = set.n_targets;
	let keep = |name: &str| !exclude.iter().any(|p| exclude_match(p, name));

	if let Some((tg, tp)) = &test_groups {
		let (test, _) = assemble(
			tg,
			&t,
			Some(&schema),
			Some(&set.sample_group),
			exclude,
		);
		let test_has_target =
			!t.is_empty() && t.iter().all(|tgt| test.names.iter().any(|n| n == tgt));
		let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
		let tc = text_col_indices(&feats);
		let oh = onehot_group_indices(&feats);
		let train = Dataset {
			x: set.select(&feats),
			y: set.y,
			source: source_label.to_string(),
			n_targets: k,
			has_target: true,
			text_cols: tc.clone(),
			onehot_groups: oh.clone(),
		};
		let test_feats: Vec<String> =
			test.names.iter().filter(|n| keep(n)).cloned().collect();
		let oh_test = onehot_group_indices(&test_feats);
		let testds = Dataset {
			x: test.select(&test_feats),
			y: test.y,
			source: tp.clone(),
			n_targets: test.n_targets,
			has_target: test_has_target,
			text_cols: tc,
			onehot_groups: oh_test,
		};
		return (train, Some(testds), flat_attrs);
	}

	let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
	let x = set.select(&feats);
	let tc = text_col_indices(&feats);
	let oh = onehot_group_indices(&feats);
	if let Some(frac) = split_frac {
		let (tr, te) = shuffle_split(&x, &set.y, k.max(1), frac, source_label, &tc, &oh);
		return (tr, Some(te), flat_attrs);
	}
	(
		Dataset {
			x,
			y: set.y,
			source: source_label.to_string(),
			n_targets: k,
			has_target: true,
			text_cols: tc,
			onehot_groups: oh,
		},
		None,
		flat_attrs,
	)
}
