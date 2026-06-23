use crate::{Mat, Vec1};
use pantry::{Attr, Kind};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
	s.split(|c: char| !c.is_alphanumeric())
		.filter(|t| !t.is_empty())
		.map(|t| t.to_ascii_lowercase())
}

pub trait IntoTargets {
	fn into_targets(self) -> Vec<String>;
}
impl IntoTargets for &str {
	fn into_targets(self) -> Vec<String> {
		vec![self.to_string()]
	}
}
impl<const N: usize> IntoTargets for [&str; N] {
	fn into_targets(self) -> Vec<String> {
		self.iter().map(|s| s.to_string()).collect()
	}
}
impl IntoTargets for &[&str] {
	fn into_targets(self) -> Vec<String> {
		self.iter().map(|s| s.to_string()).collect()
	}
}

pub struct Data {
	pub target: &'static str,
	pub set: Dataset,
	pub test: Option<Dataset>,
	target_names: Vec<String>,
	pub(crate) attrs: Vec<Attr>,
	rows: Vec<Vec<String>>,
	targets: Vec<usize>,
	sources: Vec<String>,
	test_path: Option<String>,
	split_frac: Option<f64>,
	exclude: Vec<String>,
	raw_test_rows: Option<Vec<Vec<String>>>,
	raw_test_headers: Option<Vec<String>>,
}

pub struct Dataset {
	pub x: Mat,
	pub y: Vec1,
	pub source: String,
	pub n_targets: usize,
	pub has_target: bool,
	pub text_cols: Vec<usize>,
	pub(crate) onehot_groups: Vec<(usize, usize)>,
}

/// The `Dataset → Mat` seam for the embed-on-categoricals path: collapse each
/// one-hot group back to a single integer-index column (each category a unique
/// id, offset across groups) so an `embed` layer can look them up directly.
/// Returns `(collapsed matrix, the new embed-column indices, total vocab size)`.
/// Lives here, not in `recipe-infer`, because it is the one place inference needs
/// to know what a `Dataset` is — and that knowledge stays up in this crate.
pub(crate) fn collapse_onehot(ds: &Dataset) -> (Mat, Vec<usize>, usize) {
	let n = ds.x.nrows();
	let ncols = ds.x.ncols();
	let mut in_group = vec![false; ncols];
	for &(start, len) in &ds.onehot_groups {
		for c in start..start + len {
			in_group[c] = true;
		}
	}
	let passthrough: Vec<usize> = (0..ncols).filter(|c| !in_group[*c]).collect();
	let n_cat = ds.onehot_groups.len();
	let new_ncols = passthrough.len() + n_cat;
	let mut data = vec![0.0f64; n * new_ncols];
	for (new_j, &orig_j) in passthrough.iter().enumerate() {
		for i in 0..n {
			data[i * new_ncols + new_j] = ds.x[[i, orig_j]];
		}
	}
	let embed_start = passthrough.len();
	let mut offset = 0usize;
	for (g, &(start, len)) in ds.onehot_groups.iter().enumerate() {
		let new_j = embed_start + g;
		for i in 0..n {
			for c in 0..len {
				if ds.x[[i, start + c]] > 0.5 {
					data[i * new_ncols + new_j] = (offset + c) as f64;
					break;
				}
			}
		}
		offset += len;
	}
	let embed_cols: Vec<usize> = (embed_start..embed_start + n_cat).collect();
	let x = Mat::from_shape_vec((n, new_ncols), data).expect("collapse_onehot");
	(x, embed_cols, offset)
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
	let preds = pantry::predict_kinds(&cols);
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
					pantry::KIND_NUMERIC => Kind::Numeric,
					pantry::KIND_TEMPORAL => Kind::Temporal,
					pantry::KIND_CATEGORICAL => Kind::Categorical(distinct_sorted(rows, j)),
					pantry::KIND_ORDINAL => Kind::Ordinal(distinct_sorted(rows, j)),
					pantry::KIND_TEXT => Kind::Text(col_vocab(rows, j)),
					_ => Kind::Image,
				}
			};
			Attr { name: name.clone(), kind }
		})
		.collect()
}

fn encode(
	attrs: &[Attr],
	rows: &[Vec<String>],
	targets: &[usize],
	skip: &[bool],
) -> (Vec<String>, Mat, Vec1) {
	let n = rows.len();
	let k = targets.len();

	let is_target = |ai: usize| targets.contains(&ai);
	let is_skip = |ai: usize| skip.get(ai).copied().unwrap_or(false);
	let text_seq_lens: Vec<usize> = attrs
		.iter()
		.enumerate()
		.map(|(ai, a)| match &a.kind {
			Kind::Text(_) if !is_target(ai) && !is_skip(ai) => rows
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
		Kind::Image => panic!(
			"image column '{}' not yet supported — .exclude(\"{}\")",
			a.name, a.name
		),
	};
	let proj_w: usize = attrs
		.iter()
		.enumerate()
		.filter(|(ai, _)| !is_target(*ai) && !is_skip(*ai))
		.map(|(ai, a)| width(ai, a))
		.sum();
	let bytes = n
		.saturating_mul(proj_w)
		.saturating_mul(std::mem::size_of::<f64>());
	let avail = pantry::available_ram_bytes();
	if bytes > avail / 10 * 9 {
		let mut top: Vec<(&str, usize)> = attrs
			.iter()
			.enumerate()
			.filter(|(ai, a)| !is_target(*ai) && !is_skip(*ai) && width(*ai, a) > 1)
			.map(|(ai, a)| (a.name.as_str(), width(ai, a)))
			.collect();
		top.sort_by(|a, b| b.1.cmp(&a.1));
		eprintln!("\x1b[1;31mencoded matrix too large for RAM\x1b[0m");
		eprintln!(
			"    {n} rows × {proj_w} cols × 8B = {} (available {})",
			crate::data::human_bytes(bytes),
			crate::data::human_bytes(avail)
		);
		eprintln!("    biggest one-hot expansions:");
		for (col, cnt) in top.iter().take(5) {
			eprintln!("        {col}  →  {cnt} columns");
		}
		panic!(
			"encoded matrix too large for RAM: {n} rows × {proj_w} cols × 8B = {} (available {})",
			crate::data::human_bytes(bytes),
			crate::data::human_bytes(avail)
		);
	}
	let mut names: Vec<String> = Vec::with_capacity(proj_w);
	let mut cols: Vec<Vec<f64>> = Vec::with_capacity(proj_w);
	let mut y = vec![0.0f64; n * k];
	for (ai, attr) in attrs.iter().enumerate() {
		if let Some(tj) = targets.iter().position(|&t| t == ai) {
			match &attr.kind {
				Kind::Categorical(cats) | Kind::Ordinal(cats) => {
					for (r, row) in rows.iter().enumerate() {
						let v = cell(row, ai);
						y[r * k + tj] = cats
							.iter()
							.position(|c| c == v)
							.map_or(f64::NAN, |p| p as f64);
					}
				}
				Kind::Numeric => {
					for (r, row) in rows.iter().enumerate() {
						y[r * k + tj] =
							cell(row, ai).parse::<f64>().unwrap_or(f64::NAN);
					}
				}
				Kind::Temporal => {
					for (r, row) in rows.iter().enumerate() {
						let c = cell(row, ai);
						y[r * k + tj] =
							c.parse::<f64>().unwrap_or_else(|_| date_to_f64(c));
					}
				}
				Kind::Text(_) => {
					panic!("target '{}' is free text — not a valid target", attr.name);
				}
				Kind::Image => {
					panic!("target '{}' is image data — not a valid target", attr.name);
				}
			}
			continue;
		}
		if is_skip(ai) {
			continue;
		}
		match &attr.kind {
			Kind::Numeric => {
				names.push(attr.name.clone());
				let mut col = vec![0.0f64; n];
				for (r, row) in rows.iter().enumerate() {
					col[r] = cell(row, ai).parse::<f64>().unwrap_or(f64::NAN);
				}
				cols.push(col);
			}
			Kind::Temporal => {
				names.push(attr.name.clone());
				let mut col = vec![0.0f64; n];
				for (r, row) in rows.iter().enumerate() {
					let c = cell(row, ai);
					col[r] = c.parse::<f64>().unwrap_or_else(|_| date_to_f64(c));
				}
				cols.push(col);
			}
			Kind::Categorical(cats) => {
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
			}
			Kind::Ordinal(cats) => {
				names.push(attr.name.clone());
				let mut col = vec![f64::NAN; n];
				for (r, row) in rows.iter().enumerate() {
					let v = cell(row, ai);
					if let Some(p) = cats.iter().position(|c| c == v) {
						col[r] = p as f64;
					}
				}
				cols.push(col);
			}
			Kind::Text(vocab) => {
				let seq_len: usize = rows
					.iter()
					.map(|row| tokenize(cell(row, ai)).count())
					.max()
					.unwrap_or(1);
				let base = cols.len();
				for s in 0..seq_len {
					names.push(format!("{}#t{s}", attr.name));
					cols.push(vec![0.0f64; n]);
				}
				let per_row: Vec<Vec<f64>> = rows
					.par_iter()
					.map(|row| {
						let mut ids = vec![0.0f64; seq_len];
						for (s, tok) in
							tokenize(cell(row, ai)).take(seq_len).enumerate()
						{
							ids[s] = vocab
								.binary_search(&tok)
								.map_or(0.0, |p| (p + 1) as f64);
						}
						ids
					})
					.collect();
				for (r, ids) in per_row.iter().enumerate() {
					for s in 0..seq_len {
						cols[base + s][r] = ids[s];
					}
				}
			}
			Kind::Image => {
				panic!(
					"image column '{}' not yet supported — .exclude() it",
					attr.name
				);
			}
		}
	}
	let w = cols.len();
	let mut data = vec![0.0f64; n * w];
	for (j, col) in cols.iter().enumerate() {
		for (i, v) in col.iter().enumerate() {
			data[i * w + j] = *v;
		}
	}
	(
		names,
		Mat::from_shape_vec((n, w), data).expect("encode: reshape"),
		Vec1::from(y),
	)
}

use crate::data::DirGroup;

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

		let bytes = n
			.saturating_mul(w)
			.saturating_mul(std::mem::size_of::<f64>());
		let avail = pantry::available_ram_bytes();
		if bytes > avail / 10 * 9 {
			let mut by_col: std::collections::BTreeMap<&str, usize> =
				std::collections::BTreeMap::new();
			for name in keep {
				*by_col
					.entry(name.split('=').next().unwrap_or(name))
					.or_insert(0) += 1;
			}
			let mut top: Vec<(&&str, &usize)> = by_col.iter().collect();
			top.sort_by(|a, b| b.1.cmp(a.1));
			eprintln!("\x1b[1;31mencoded matrix too large for RAM\x1b[0m");
			eprintln!(
				"    {n} rows × {w} cols × 8B = {} (available {})",
				crate::data::human_bytes(bytes),
				crate::data::human_bytes(avail)
			);
			eprintln!("    biggest one-hot expansions:");
			for (col, cnt) in top.iter().take(5) {
				eprintln!("        {col}  →  {cnt} columns");
			}
			panic!(
				"selection matrix too large for RAM: {n} rows × {w} cols × 8B = {} (available {})",
				crate::data::human_bytes(bytes),
				crate::data::human_bytes(avail)
			);
		}
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
) -> (Vec<String>, Mat, Vec1) {
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
			let (fnames, x, y) = encode(&attrs, cells, target_cols, &skip);
			let names = fnames.iter().map(|f| namespaced(name, f)).collect();
			(names, x, y)
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
			(names, x, Vec1::zeros(n))
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
	let n_targets = target_cols.len();

	let (s_names, s_x, y) = encode_group(
		&groups[sample_idx],
		&mut schema,
		schema_in,
		&target_cols,
		exclude,
	);
	let s_hashes = group_hashes(&groups[sample_idx]);
	let n = s_x.nrows();

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

		if matches!(g, DirGroup::Image { .. }) {
			skipped.push(format!(
                        "{}: image group ({} wells) kept out of the feature matrix — one copy per well, not duplicated into rows",
                        group_name(g), by_hash.len()
                  ));
			continue;
		}
		let (g_names, g_x, _gy) = encode_group(g, &mut schema, schema_in, &[], exclude);

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

fn exclude_mask(attrs: &[Attr], group: &str, exclude: &[String]) -> Vec<bool> {
	attrs.iter()
		.map(|a| {
			let nm = namespaced(group, &a.name);
			exclude.iter().any(|p| exclude_match(p, &nm))
		})
		.collect()
}

fn exclude_match(pattern: &str, name: &str) -> bool {
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

impl Data {
	pub fn load() -> Data {
		Data {
			target: "",
			set: Dataset {
				x: crate::Mat::default((0, 0)),
				y: crate::Vec1::default(0),
				source: String::new(),
				n_targets: 0,
				has_target: false,
				text_cols: Vec::new(),
				onehot_groups: Vec::new(),
			},
			test: None,
			target_names: Vec::new(),
			attrs: Vec::new(),
			rows: Vec::new(),
			targets: Vec::new(),
			sources: Vec::new(),
			test_path: None,
			split_frac: None,
			exclude: Vec::new(),
			raw_test_rows: None,
			raw_test_headers: None,
		}
	}

	fn source_label(&self) -> String {
		self.sources.join(", ")
	}

	pub fn set(mut self, path: &str) -> Data {
		self.sources.push(path.to_string());
		if is_arff(path) {
			let (attrs, rows) = crate::data::parse_arff(path);
			self.attrs = attrs;
			self.rows = rows;
		}
		self
	}

	pub fn target(mut self, t: impl IntoTargets) -> Data {
		self.target_names = t.into_targets();
		self.target = self
			.target_names
			.first()
			.map_or("", |s| Box::leak(s.clone().into_boxed_str()));
		if !self.attrs.is_empty() {
			self.targets = self
				.target_names
				.iter()
				.map(|name| {
					self.attrs
						.iter()
						.position(|a| a.name == *name)
						.unwrap_or_else(|| {
							panic!("Data::target: no attribute named '{name}'")
						})
				})
				.collect();
		}
		if let Some(tp) = &self.test_path {
			if let Ok(text) = std::fs::read_to_string(tp) {
				let mut lines = text.lines();
				if let Some(header_line) = lines.next() {
					self.raw_test_headers = Some(crate::data::split_fields(header_line)
						.into_iter()
						.map(|s| s.trim().to_string())
						.collect());
					self.raw_test_rows = Some(lines
						.filter(|l| !l.trim().is_empty())
						.map(|l| crate::data::split_fields(l))
						.collect());
				}
			}
		}
		let (train, test, attrs) = self.prepare();
		self.set = train;
		self.test = test;
		self.attrs = attrs;
		self.print_summary();
		self
	}

	fn feature_type_counts(&self) -> Vec<(&'static str, usize)> {
		let is_target = |name: &str| self.target_names.iter().any(|t| t == name);
		let is_excluded = |name: &str| self.exclude.iter().any(|p| exclude_match(p, name));
		let (mut numeric, mut temporal, mut categorical, mut ordinal, mut text, mut image) =
			(0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
		for a in &self.attrs {
			if is_target(&a.name) || is_excluded(&a.name) {
				continue;
			}
			match &a.kind {
				Kind::Numeric => numeric += 1,
				Kind::Temporal => temporal += 1,
				Kind::Categorical(_) => categorical += 1,
				Kind::Ordinal(_) => ordinal += 1,
				Kind::Text(_) => text += 1,
				Kind::Image => image += 1,
			}
		}
		let mut out = Vec::new();
		if numeric > 0 {
			out.push(("numeric", numeric));
		}
		if temporal > 0 {
			out.push(("temporal", temporal));
		}
		if categorical > 0 {
			out.push(("categorical", categorical));
		}
		if ordinal > 0 {
			out.push(("ordinal", ordinal));
		}
		if text > 0 {
			out.push(("text", text));
		}
		if image > 0 {
			out.push(("image", image));
		}
		out
	}

	fn cat_cardinality_counts(&self) -> Vec<(usize, usize)> {
		let is_target = |name: &str| self.target_names.iter().any(|t| t == name);
		let is_excluded = |name: &str| self.exclude.iter().any(|p| exclude_match(p, name));
		let mut card: std::collections::BTreeMap<usize, usize> =
			std::collections::BTreeMap::new();
		for a in &self.attrs {
			if is_target(&a.name) || is_excluded(&a.name) {
				continue;
			}
			if let Kind::Categorical(cats) = &a.kind {
				*card.entry(cats.len()).or_default() += 1;
			}
		}
		card.into_iter().collect()
	}

	fn print_summary(&self) {
		let disk_size = |path: &str| -> String {
			std::fs::metadata(path)
				.map(|m| crate::data::human_bytes(m.len() as usize))
				.unwrap_or_else(|_| "?".into())
		};
		let short = |path: &str| -> String {
			if let Some(home) = std::env::var("HOME").ok() {
				if let Some(rest) = path.strip_prefix(&home) {
					return format!("~{rest}");
				}
			}
			path.to_string()
		};
		let raw_cols = self.attrs.len();
		let types = self.feature_type_counts();
		let print_types = |indent: &str| {
			if types.len() == 1 {
				eprintln!("{indent}{} {}", types[0].1, types[0].0);
			} else {
				for (kind, count) in &types {
					eprintln!("{indent}{count} {kind}");
				}
			}
		};
		let set_rows = if self.split_frac.is_some() {
			self.set.x.nrows() + self.test.as_ref().map_or(0, |t| t.x.nrows())
		} else {
			self.set.x.nrows()
		};
		for src in &self.sources {
			eprintln!("\x1b[32mset\x1b[0m  {}", short(src),);
			eprintln!("    {}", disk_size(src),);
		}
		eprintln!("    {} rows  {} cols", set_rows, raw_cols,);
		print_types("        ");
		for ex in &self.exclude {
			eprintln!("    excluded  {ex}");
		}
		let cards = self.cat_cardinality_counts();
		if !cards.is_empty() {
			eprintln!("    encoding");
			for (card, count) in &cards {
				let range: Vec<String> = (0..*card).map(|i| i.to_string()).collect();
				eprintln!("        {count} × [{}]", range.join(", "));
			}
		}
		eprintln!("    {} features → model", self.set.x.ncols(),);
		if let Some(test) = &self.test {
			if let Some(tp) = &self.test_path {
				let test_raw_cols =
					self.raw_test_headers.as_ref().map_or(raw_cols, |h| h.len());
				let test_raw_rows = self
					.raw_test_rows
					.as_ref()
					.map_or(test.x.nrows(), |r| r.len());
				eprintln!("\x1b[32mtest\x1b[0m  {}", short(tp),);
				eprintln!(
					"    {} rows  {} cols  {}",
					test_raw_rows,
					test_raw_cols,
					disk_size(tp),
				);
				print_types("        ");
				eprintln!("    {} features → model", test.x.ncols(),);
			} else if self.split_frac.is_some() {
				eprintln!(
					"\x1b[32msplit\x1b[0m  {} train / {} test",
					self.set.x.nrows(),
					test.x.nrows(),
				);
			}
		}
		for t in &self.target_names {
			eprintln!("\x1b[32mtarget\x1b[0m  {t}");
		}
	}

	pub fn test(mut self, path: &str) -> Data {
		self.test_path = Some(path.to_string());
		self
	}

	pub fn exclude(mut self, pattern: &str) -> Data {
		self.exclude.push(pattern.to_string());
		self
	}

	pub fn split(mut self, train_frac: f64) -> Data {
		assert!(
			(0.0..1.0).contains(&train_frac),
			"split fraction must be in (0, 1), got {train_frac}",
		);
		self.split_frac = Some(train_frac);
		self
	}

	fn prepare(&self) -> (Dataset, Option<Dataset>, Vec<Attr>) {
		let (mut train, test, attrs) = if self.attrs.is_empty() {
			let (tr, te, a) = self.prepare_table();
			(tr, te, a)
		} else {
			let (tr, te) = self.prepare_arff();
			(tr, te, self.attrs.clone())
		};
		report_nans(&train, test.as_ref());
		drop_nan_samples(&mut train);
		assert!(train.x.nrows() > 0, "dataset has 0 rows after NaN removal");
		assert!(train.x.ncols() > 0, "dataset has 0 feature columns");
		let k = train.n_targets.max(1);
		assert_eq!(
			train.y.len(),
			train.x.nrows() * k,
			"x/y dimension mismatch: {} rows × {k} targets but y has {} elements",
			train.x.nrows(),
			train.y.len(),
		);
		(train, test, attrs)
	}

	fn prepare_arff(&self) -> (Dataset, Option<Dataset>) {
		let k = self.targets.len().max(1);
		let skip = exclude_mask(&self.attrs, "", &self.exclude);
		let (names, x, y) = encode(&self.attrs, &self.rows, &self.targets, &skip);
		let tc = text_col_indices(&names);
		let oh = onehot_group_indices(&names);
		if let Some(frac) = self.split_frac {
			let (tr, te) = shuffle_split(&x, &y, k, frac, &self.source_label(), &tc, &oh);
			(tr, Some(te))
		} else if let Some(tp) = &self.test_path {
			let (_, trows) = crate::data::parse_arff(tp);
			let (_, tx, ty) = encode(&self.attrs, &trows, &self.targets, &skip);
			(
				Dataset {
					x,
					y,
					source: self.source_label(),
					n_targets: k,
					has_target: true,
					text_cols: tc.clone(),
					onehot_groups: oh.clone(),
				},
				Some(Dataset {
					x: tx,
					y: ty,
					source: tp.clone(),
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
					source: self.source_label(),
					n_targets: k,
					has_target: true,
					text_cols: tc,
					onehot_groups: oh,
				},
				None,
			)
		}
	}

	fn prepare_table(&self) -> (Dataset, Option<Dataset>, Vec<Attr>) {
		let set_groups: Vec<DirGroup> = self
			.sources
			.iter()
			.flat_map(|s| crate::data::load_groups(s))
			.collect();
		let set_tnames = table_names(&set_groups);

		let test_groups = self
			.test_path
			.as_ref()
			.map(|tp| (crate::data::load_groups(tp), tp.clone()));
		let test_tnames: Option<Vec<String>> = match (&test_groups, self.split_frac) {
			(Some((g, _)), _) => Some(table_names(g)),
			(None, Some(_)) => Some(set_tnames.clone()),
			(None, None) => None,
		};
		let t = self.resolve_targets(&set_tnames, test_tnames.as_deref());

		let (set, schema) = assemble(&set_groups, &t, None, None, &self.exclude);
		let flat_attrs: Vec<Attr> = schema.values().flat_map(|v| v.iter().cloned()).collect();
		let k = set.n_targets;
		let keep = |name: &str| !self.exclude.iter().any(|p| exclude_match(p, name));

		if let Some((tg, tp)) = &test_groups {
			let (test, _) = assemble(
				tg,
				&t,
				Some(&schema),
				Some(&set.sample_group),
				&self.exclude,
			);
			let test_has_target =
				!t.is_empty() && t.iter().all(|tgt| test.names.iter().any(|n| n == tgt));
			let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
			let tc = text_col_indices(&feats);
			let oh = onehot_group_indices(&feats);
			let train = Dataset {
				x: set.select(&feats),
				y: set.y,
				source: self.source_label(),
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
				source: (*tp).clone(),
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
		if let Some(frac) = self.split_frac {
			let (tr, te) =
				shuffle_split(&x, &set.y, k.max(1), frac, &self.source_label(), &tc, &oh);

			return (tr, Some(te), flat_attrs);
		}
		(
			Dataset {
				x,
				y: set.y,
				source: self.source_label(),
				n_targets: k,
				has_target: true,
				text_cols: tc,
				onehot_groups: oh,
			},
			None,
			flat_attrs,
		)
	}

	fn resolve_targets(
		&self,
		set_names: &[String],
		test_names: Option<&[String]>,
	) -> Vec<String> {
		if !self.target_names.is_empty() {
			return self
				.target_names
				.iter()
				.map(|want| {
					set_names
						.iter()
						.find(|n| {
							n.as_str() == want
								|| n.ends_with(&format!(":{want}")) || n
								.rsplit(':')
								.next() == Some(
								want.as_str(),
							)
						})
						.cloned()
						.unwrap_or_else(|| {
							let avail: Vec<&str> =
								set_names.iter().map(|s| s.as_str()).collect();
							panic!(
								"target '{want}' not found — available columns: {}",
								avail.join(", ")
							);
						})
				})
				.collect();
		}
		if let Some(tn) = test_names {
			if set_names.len() == tn.len() + 1 {
				return vec![set_names.last().expect("set has columns").clone()];
			}
		}
		Vec::new()
	}
}

fn col_after(c: &str) -> &str {
	c.split_once(':').map_or(c, |(_, s)| s)
}

fn is_arff(path: &str) -> bool {
	std::path::Path::new(path)
		.extension()
		.and_then(|e| e.to_str())
		== Some("arff")
}

pub(crate) fn drop_nan_samples(train: &mut Dataset) {
	let n = train.x.nrows();
	let k = train.n_targets.max(1);
	let keep: Vec<usize> = (0..n)
		.filter(|&i| {
			(0..k).all(|j| !train.y[i * k + j].is_nan())
				&& train.x.row(i).iter().all(|v| !v.is_nan())
		})
		.collect();
	let dropped = n - keep.len();
	if dropped == 0 {
		return;
	}
	train.x = train.x.select(ndarray::Axis(0), &keep);
	let mut yd = Vec::with_capacity(keep.len() * k);
	for &i in &keep {
		for j in 0..k {
			yd.push(train.y[i * k + j]);
		}
	}
	train.y = Vec1::from(yd);
	eprintln!(
		"\x1b[32mhandled\x1b[0m\n    train\n        dropped {dropped} {} (NaN)",
		if dropped == 1 { "sample" } else { "samples" }
	);
}

pub(crate) fn nan_stats(d: &Dataset) -> (usize, usize, usize) {
	let cells = d.x.iter().filter(|v| v.is_nan()).count();
	let rows =
		d.x.outer_iter()
			.filter(|r| r.iter().any(|v| v.is_nan()))
			.count();
	let target = d.y.iter().filter(|v| v.is_nan()).count();
	(cells, rows, target)
}

fn report_nans(train: &Dataset, test: Option<&Dataset>) {
	let (tf, tr, tt) = nan_stats(train);
	let (ef, er, et) = test.map(nan_stats).unwrap_or((0, 0, 0));
	if tf + tt + ef + et == 0 {
		return;
	}
	let rows = |n: usize| if n == 1 { "row" } else { "rows" };
	eprintln!("\x1b[1;31mnans\x1b[0m");
	if tf > 0 || tt > 0 {
		eprintln!("    train");
		if tf > 0 {
			eprintln!("        {tf} in features ({tr} {})", rows(tr));
		}
		if tt > 0 {
			eprintln!("        {tt} in target");
		}
	}
	if ef > 0 || et > 0 {
		eprintln!("    test");
		if ef > 0 {
			eprintln!("        {ef} in features ({er} {})", rows(er));
		}
		if et > 0 {
			eprintln!("        {et} in target");
		}
	}
}

fn shuffle_split(
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
	feats.iter()
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

impl crate::model::RunData for Data {
	fn dataset(&self) -> &Dataset {
		&self.set
	}
	fn target_names(&self) -> Vec<String> {
		self.target_names.clone()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		self.raw_test_rows.clone()
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		self.raw_test_headers.clone()
	}
	fn infer_only(&self) -> bool {
		false
	}
}
