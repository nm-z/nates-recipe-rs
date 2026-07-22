use ogdl::log::{Errored, Write, data};

use crate::data::DirGroup;
use crate::{Attr, Kind, Mat, Vec1};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error;
use std::fmt;
use std::fs;
use std::mem;
use std::path::Path;

pub struct Dataset {
	pub x: Mat,
	pub y: Vec1,
	pub source: String,
	pub n_targets: usize,
	pub has_target: bool,
	pub text_cols: Vec<usize>,
	pub onehot_groups: Vec<GroupSpan>,
}

#[derive(Clone, Copy)]
pub struct GroupSpan {
	pub start: usize,
	pub len: usize,
}

fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
	s.split(|c: char| !c.is_alphanumeric())
		.filter(|t| !t.is_empty())
		.map(|t| t.to_ascii_lowercase())
}

fn file_stem(s: &str) -> &str {
	Path::new(s)
		.file_stem()
		.and_then(|x| x.to_str())
		.unwrap_or(s)
}

fn cell(row: &[String], j: usize) -> &str {
	row.get(j).map_or("", |s| s.as_str())
}

fn date_to_f64(s: &str) -> f64 {
	let parts: Vec<&str> = s.split(['-', '/', 'T', ' ']).collect();
	if parts.len() >= 3
		&& let (Ok(y), Ok(m), Ok(d)) = (
			parts[0].parse::<f64>(),
			parts[1].parse::<f64>(),
			parts[2].parse::<f64>(),
		) {
		return y * 365.25 + m * 30.44 + d;
	}
	f64::NAN
}

pub(crate) fn is_missing(c: &str) -> bool {
	c.is_empty()
		|| matches!(
			c,
			"NA" | "NaN"
				| "nan" | "N/A" | "NULL"
				| "null" | "None" | "none"
				| "?" | "." | "-"
		)
}

fn col_vocab(rows: &[Vec<String>], j: usize) -> Vec<String> {
	let set = rows
		.par_iter()
		.fold(HashSet::<String>::new, |mut acc, row| {
			acc.extend(tokenize(cell(row, j)));
			acc
		})
		.reduce(HashSet::<String>::new, |a, b| {
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
		.collect::<HashSet<_>>()
		.into_iter()
		.collect();
	cats.sort_unstable();
	cats
}

fn infer_attrs(
	headers: &[String],
	rows: &[Vec<String>],
	known: Option<&[Attr]>,
	pre: Option<&[ColKind]>,
) -> anyhow::Result<Vec<Attr>> {
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
	let preds: Vec<usize> = match pre {
		Some(pre) => {
			if pre.len() != headers.len()
				|| pre.iter().zip(headers).any(|(c, name)| &c.header != name)
			{
				Write::err(format!(
					"detect_kinds drift: precomputed [{}] != parsed [{}]",
					pre.iter()
						.map(|c| c.header.as_str())
						.collect::<Vec<_>>()
						.join(", "),
					headers
						.iter()
						.map(String::as_str)
						.collect::<Vec<_>>()
						.join(", "),
				))?;
			}
			to_predict.iter().map(|&j| pre[j].kind).collect()
		}
		None if to_predict.is_empty() => Vec::new(),
		None => {
			Write::err(format!(
				"column(s) [{}] have no load-time kind and no train-schema attr — detection runs at Data::load, never mid-run",
				to_predict
					.iter()
					.map(|&j| headers[j].as_str())
					.collect::<Vec<_>>()
					.join(", "),
			))?;
			Vec::new()
		}
	};
	let mut pred = HashMap::new();
	for (i, &j) in to_predict.iter().enumerate() {
		pred.insert(j, preds[i]);
	}
	Ok(headers
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
					crate::KIND_CATEGORICAL => {
						Kind::Categorical(distinct_sorted(rows, j))
					}
					crate::KIND_ORDINAL => Kind::Ordinal(distinct_sorted(rows, j)),
					crate::KIND_TEXT => Kind::Text(col_vocab(rows, j)),
					_ => Kind::Image,
				}
			};
			Attr {
				name: name.clone(),
				kind,
			}
		})
		.collect())
}

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
			let names = (0..seq_len)
				.map(|s| format!("{}#t{s}", attr.name))
				.collect();
			let per_row: Vec<Vec<f64>> = rows
				.par_iter()
				.map(|row| {
					let mut ids = vec![0.0f64; seq_len];
					for (s, tok) in tokenize(cell(row, ai)).take(seq_len).enumerate() {
						ids[s] =
							vocab.binary_search(&tok).map_or(0.0, |p| (p + 1) as f64);
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
		Kind::Image => (Vec::new(), Vec::new()),
	}
}

fn encode(
	attrs: &[Attr],
	rows: &[Vec<String>],
	targets: &[usize],
	skip: &[bool],
) -> anyhow::Result<(Vec<String>, Mat, Vec1, usize)> {
	let n = rows.len();

	let is_target = |ai: usize| targets.contains(&ai);
	let is_skip = |ai: usize| skip.get(ai).copied().unwrap_or(false);
	let text_seq_lens: Vec<usize> = attrs
		.iter()
		.enumerate()
		.map(|(ai, a)| match &a.kind {
			Kind::Text(_) if !is_skip(ai) => {
				let raw = rows
					.iter()
					.map(|row| tokenize(cell(row, ai)).count())
					.max()
					.unwrap_or(1);
				let capped = raw.min(crate::TEXT_CONTEXT);
				if raw > capped {
					Write::line(
						data,
						&format!(
							"    context  {} tokens/{} → capped to {capped}",
							raw, a.name
						),
					);
				}
				capped
			}
			_ => 0,
		})
		.collect();
	let width = |ai: usize, a: &Attr| match &a.kind {
		Kind::Numeric | Kind::Temporal | Kind::Ordinal(_) => 1,
		Kind::Categorical(c) => c.len(),
		Kind::Text(_) => text_seq_lens[ai].max(1),
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
	admit_ceiling(n, proj_w, "encoded", &top)?;

	let mut names: Vec<String> = Vec::with_capacity(proj_w);
	let w = proj_w;
	let mut dat = alloc_matrix(n, w, "encoded")?;
	let mut jc = 0usize;
	let mut tcols: Vec<Vec<Vec<f64>>> = vec![Vec::new(); targets.len()];
	for (ai, attr) in attrs.iter().enumerate() {
		if is_skip(ai) && !is_target(ai) {
			continue;
		}
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
				for (cn, col) in cnames.into_iter().zip(ccols) {
					names.push(cn);
					for (i, v) in col.iter().enumerate() {
						dat[i * w + jc] = *v;
					}
					jc += 1;
				}
			}
		}
	}
	let ycols: Vec<Vec<f64>> = tcols.into_iter().flatten().collect();
	let k = ycols.len();
	let mut ydata = vec![0.0f64; n * k];
	for (j, col) in ycols.iter().enumerate() {
		for (i, v) in col.iter().enumerate() {
			ydata[i * k + j] = *v;
		}
	}
	Ok((
		names,
		Mat::from_shape_vec((n, w), dat)
			.map_err(|e| Errored::new(format!("encode: reshape: {e}")))?,
		Vec1::from(ydata),
		k,
	))
}

fn oom_pair(name: &str, val: &str) -> String {
	format!("{name}: {val}")
}

#[derive(Debug)]
pub struct CeilingExceeded {
	pub label: String,
	pub rows: usize,
	pub cols: usize,
	pub need: usize,
	pub cap: usize,
}

impl fmt::Display for CeilingExceeded {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{} buffer exceeds memory ceiling: {} rows × {} cols × 8B = {} (ceiling {})",
			self.label,
			self.rows,
			self.cols,
			crate::data::human_bytes(self.need),
			crate::data::human_bytes(self.cap),
		)
	}
}

impl error::Error for CeilingExceeded {}

fn admit_ceiling(
	n: usize,
	w: usize,
	label: &str,
	top_cols: &[(&str, usize)],
) -> anyhow::Result<()> {
	let bytes = n.saturating_mul(w).saturating_mul(mem::size_of::<f64>());
	let spill = Path::new(".recipe_spill");
	let full = match recipe_infer::tiered::admit(bytes, 0, 0, spill) {
		Ok(bud) if bytes > bud.ram_data => recipe_infer::tiered::Full {
			need: bytes,
			cap: bud.ram_data,
		},
		Ok(_) => return Ok(()),
		Err(f) => f,
	};
	let hb = crate::data::human_bytes;
	let cols_bytes = |c: usize| c.saturating_mul(n).saturating_mul(8);
	let tokens_cols: usize = top_cols
		.iter()
		.filter(|(nm, _)| nm.contains("#t"))
		.map(|(_, c)| *c)
		.sum();
	let onehot_cols: usize = top_cols
		.iter()
		.filter(|(_, c)| *c > 1)
		.map(|(_, c)| *c)
		.sum();
	let scalar_cols: usize = top_cols
		.iter()
		.filter(|(nm, c)| *c == 1 && !nm.contains("#t"))
		.map(|(_, c)| *c)
		.sum();
	let mut autopsy: Vec<(&str, usize)> = [
		("tokens", tokens_cols),
		("scalar", scalar_cols),
		("onehot", onehot_cols),
	]
	.into_iter()
	.filter(|(_, c)| *c > 0)
	.collect();
	autopsy.sort_by(|a, b| cols_bytes(b.1).cmp(&cols_bytes(a.1)));
	let mut line = vec![oom_pair("OVER CEILING", label)];
	line.extend(
		autopsy
			.iter()
			.map(|(nm, c)| oom_pair(nm, &format!("{} ({c})", hb(cols_bytes(*c))))),
	);
	let mut bases: BTreeMap<&str, usize> = BTreeMap::new();
	for (nm, _) in top_cols.iter().filter(|(nm, _)| nm.contains("#t")) {
		*bases.entry(nm.split("#t").next().unwrap_or(nm))
			.or_insert(0) += 1;
	}
	line.push(oom_pair("rows", &n.to_string()));
	line.push(oom_pair("ceiling", &hb(full.cap)));
	line.push(oom_pair("over", &hb(full.need.saturating_sub(full.cap))));
	if let Some((base, seq)) = bases.into_iter().max_by_key(|(_, c)| *c) {
		line.push(oom_pair("widest", &format!("{base}×{seq}")));
	}
	Write::error(line.join(", "));
	Err(CeilingExceeded {
		label: label.to_string(),
		rows: n,
		cols: w,
		need: full.need,
		cap: full.cap,
	}
	.into())
}

fn alloc_matrix(n: usize, w: usize, label: &str) -> anyhow::Result<Vec<f64>> {
	let cells = n.saturating_mul(w);
	let mut dat: Vec<f64> = Vec::new();
	if dat.try_reserve_exact(cells).is_err() {
		let spill = Path::new(".recipe_spill");
		let cap = recipe_infer::tiered::admit(0, 0, 0, spill)
			.map(|b| b.ram_data)
			.unwrap_or(0);
		return Err(CeilingExceeded {
			label: label.to_string(),
			rows: n,
			cols: w,
			need: cells.saturating_mul(mem::size_of::<f64>()),
			cap,
		}
		.into());
	}
	dat.resize(cells, 0.0);
	Ok(dat)
}

type Schema = BTreeMap<String, Vec<Attr>>;

pub struct GroupKinds {
	pub name: String,
	pub cols: Vec<ColKind>,
}

pub struct ColKind {
	pub header: String,
	pub kind: usize,
}
pub type PreKinds = Vec<GroupKinds>;

fn group_pre<'a>(pre: Option<&'a [GroupKinds]>, name: &str) -> Option<&'a [ColKind]> {
	pre.and_then(|p| p.iter().find(|g| g.name == name).map(|g| g.cols.as_slice()))
}

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
	fn select(&mut self, keep: &[String]) -> anyhow::Result<Mat> {
		let n = self.samples;
		let w = keep.len();
		let idx: HashMap<&str, usize> = self
			.names
			.iter()
			.enumerate()
			.map(|(i, s)| (s.as_str(), i))
			.collect();
		let picks: Vec<(usize, usize)> = keep
			.iter()
			.map(|name| self.sources[idx[name.as_str()]])
			.collect();

		if w > 0 {
			let mi = picks[0].0;
			let cols: Vec<usize> = picks.iter().map(|&(_, c)| c).collect();
			let compact = picks.iter().all(|&(m, _)| m == mi)
				&& self.gather[mi].len() == n
				&& self.gather[mi]
					.iter()
					.enumerate()
					.all(|(i, g)| *g == Some(i))
				&& cols.windows(2).all(|p| p[0] < p[1])
				&& self.mats[mi].is_standard_layout();
			if compact {
				let big_w = self.mats[mi].ncols();
				let (mut buf, _) = mem::take(&mut self.mats[mi]).into_raw_vec_and_offset();
				for i in 0..n {
					let (ib, iw) = (i * big_w, i * w);
					for (jc, &c) in cols.iter().enumerate() {
						buf[iw + jc] = buf[ib + c];
					}
				}
				buf.truncate(n * w);
				return Ok(Mat::from_shape_vec((n, w), buf)
					.map_err(|e| Errored::new(format!("select reshape: {e}")))?);
			}
		}

		let mut by_col: BTreeMap<&str, usize> = BTreeMap::new();
		for name in keep {
			*by_col
				.entry(name.split('=').next().unwrap_or(name))
				.or_insert(0) += 1;
		}
		let top: Vec<(&str, usize)> = by_col.into_iter().collect();
		admit_ceiling(n, w, "selection", &top)?;
		let mut dat = alloc_matrix(n, w, "selection")?;
		for (jc, name) in keep.iter().enumerate() {
			let (mi, col) = self.sources[idx[name.as_str()]];
			let m = &self.mats[mi];
			let g = &self.gather[mi];
			for i in 0..n {
				dat[i * w + jc] = g[i].map_or(f64::NAN, |r| m[[r, col]]);
			}
		}
		Ok(Mat::from_shape_vec((n, w), dat)
			.map_err(|e| Errored::new(format!("select reshape: {e}")))?)
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
	pre: Option<&[ColKind]>,
) -> anyhow::Result<(Vec<String>, Mat, Vec1, usize)> {
	Ok(match g {
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
				pre,
			)?;
			schema.insert(name.clone(), attrs.clone());

			let skip = exclude_mask(&attrs, name, exclude);
			let (fnames, x, y, k) = encode(&attrs, cells, target_cols, &skip)?;
			let names = fnames.iter().map(|f| namespaced(name, f)).collect();
			(names, x, y, k)
		}
		DirGroup::Image {
			name, dim, pixels, ..
		} => {
			let n = pixels.len();
			let mut dat = vec![0.0f64; n * dim];
			for (i, px) in pixels.iter().enumerate() {
				for (j, v) in px.iter().take(*dim).enumerate() {
					dat[i * dim + j] = *v;
				}
			}
			let names = (0..*dim)
				.map(|i| namespaced(name, &format!("px{i}")))
				.collect();
			let x = Mat::from_shape_vec((n, *dim), dat)
				.map_err(|e| Errored::new(format!("image reshape: {e}")))?;
			(names, x, Vec1::zeros(n), 0)
		}
	})
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
	pre: Option<&[GroupKinds]>,
) -> anyhow::Result<(Assembled, Schema)> {
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
					_ => anyhow::bail!(
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
		group_pre(pre, group_name(&groups[sample_idx])),
	)?;
	let s_hashes = group_hashes(&groups[sample_idx]);
	let n = s_x.nrows();
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

	let mut s_count: HashMap<&str, usize> = HashMap::new();
	for h in s_hashes {
		*s_count.entry(h.as_str()).or_insert(0) += 1;
	}
	let mut s_pos = vec![0usize; n];
	{
		let mut seen: HashMap<&str, usize> = HashMap::new();
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

		if let (
			DirGroup::Image {
				hashes: g_hashes, ..
			},
			Some(cells),
		) = (g, sample_cells)
		{
			let key_set: HashSet<&str> = g_hashes.iter().map(String::as_str).collect();
			let ncols = cells.first().map_or(0, Vec::len);
			let (mut best_col, mut best) = (0usize, 0usize);
			for c in 0..ncols {
				let hits = cells
					.iter()
					.filter(|r| {
						key_set.contains(file_stem(r.get(c).map_or("", String::as_str)))
					})
					.count();
				if hits > best {
					best = hits;
					best_col = c;
				}
			}
			if best > 0 {
				let by_key: HashMap<&str, usize> = g_hashes
					.iter()
					.enumerate()
					.map(|(i, h)| (h.as_str(), i))
					.collect();
				let (g_names, g_x, _gy, _gk) = encode_group(
					g,
					&mut schema,
					schema_in,
					&[],
					exclude,
					group_pre(pre, group_name(g)),
				)?;
				let src: Vec<Option<usize>> = (0..n)
					.map(|i| {
						by_key.get(file_stem(
							cells[i].get(best_col).map_or("", String::as_str),
						))
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
		let mut by_hash: HashMap<&str, Vec<usize>> = HashMap::new();
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

		let (g_names, g_x, _gy, _gk) = encode_group(
			g,
			&mut schema,
			schema_in,
			&[],
			exclude,
			group_pre(pre, group_name(g)),
		)?;

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

	Ok((
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
	))
}

fn group_name(g: &DirGroup) -> &str {
	match g {
		DirGroup::Table { name, .. } | DirGroup::Image { name, .. } => name,
	}
}

pub fn exclude_mask(attrs: &[Attr], group: &str, exclude: &[String]) -> Vec<bool> {
	attrs.iter()
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

pub struct Split {
	pub train: Dataset,
	pub test: Dataset,
}

pub fn shuffle_split(
	x: &Mat,
	y: &Vec1,
	k: usize,
	train_frac: f64,
	source: &str,
	text_cols: &[usize],
	onehot_groups: &[GroupSpan],
) -> Split {
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
			x: Mat::from_shape_vec((sel.len(), cols), xd).unwrap_or_else(|e| {
				Write::error(format!("split: x reshape: {e}"));
				return Mat::zeros((sel.len(), cols));
			}),
			y: Vec1::from(yd),
			source: source.to_string(),
			n_targets: k,
			has_target: k > 0,
			text_cols: text_cols.to_vec(),
			onehot_groups: onehot_groups.to_vec(),
		}
	};
	Split {
		train: take(&idx[..n_train]),
		test: take(&idx[n_train..]),
	}
}

fn text_col_indices(feats: &[String]) -> Vec<usize> {
	feats.iter()
		.enumerate()
		.filter(|(_, n)| n.contains("#t"))
		.map(|(i, _)| i)
		.collect()
}

fn onehot_group_indices(feats: &[String]) -> Vec<GroupSpan> {
	let mut groups = Vec::new();
	let mut i = 0;
	while i < feats.len() {
		if let Some(eq) = feats[i].find('=') {
			let prefix = &feats[i][..=eq];
			let start = i;
			while i < feats.len() && feats[i].starts_with(prefix) {
				i += 1;
			}
			groups.push(GroupSpan {
				start,
				len: i - start,
			});
		} else {
			i += 1;
		}
	}
	groups
}

pub enum Nan {
	Drop,
	ImputeMean,
	Error,
}

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
			if !v.iter().all(|x| x.is_finite()) {
				Write::error(format!(
					"NaN/inf in '{name}' — no missing values allowed here"
				));
			}
			return (0..v.len()).collect();
		}
		Nan::Drop => (0..v.len()).filter(|&i| v[i].is_finite()).collect(),
	}
}

pub fn clean_dataset(d: &mut Dataset) -> anyhow::Result<()> {
	let k = d.n_targets;
	let n = d.x.nrows();
	let src = crate::data::short_path(&d.source);
	if d.x.ncols() == 0 {
		return Err(Errored::new(format!(
			"no columns found, check delimiter  {src}  →  {n} row(s) × 0 column(s)"
		))
		.into());
	}
	if d.y.len() < n * k {
		return Err(Errored::new(format!(
			"{k} target column(s) but {} target value(s)  {src}  →  {n} row(s) × {k} target(s) needs {} value(s)",
			d.y.len(),
			n * k
		))
		.into());
	}
	let mut keep: Vec<usize> = (0..n).collect();
	for j in 0..k {
		let mut col: Vec<f64> = (0..n).map(|i| d.y[i * k + j]).collect();
		let kj = nan_clean(&mut col, Nan::Drop, "target");
		keep.retain(|i| kj.binary_search(i).is_ok());
	}
	if keep.len() < n {
		Write::line(
			data,
			&format!(
				"nan  dropped {} row(s) with a missing target",
				n - keep.len()
			),
		);
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
	return Ok(());
}

pub fn prepare_arff_data(
	attrs: &[Attr],
	rows: &[Vec<String>],
	targets: &[usize],
	exclude: &[String],
	split_frac: Option<f64>,
	test_path: Option<&str>,
	source_label: &str,
) -> anyhow::Result<PreparedArff> {
	let skip = exclude_mask(attrs, "", exclude);
	let (names, x, y, enc_k) = encode(attrs, rows, targets, &skip)?;
	let k = enc_k.max(1);
	let tc = text_col_indices(&names);
	let oh = onehot_group_indices(&names);
	let (train, test) = if let Some(frac) = split_frac {
		let Split {
			train: tr,
			test: te,
		} = shuffle_split(&x, &y, k, frac, source_label, &tc, &oh);
		(tr, Some(te))
	} else if let Some(tp) = test_path {
		let trows = crate::data::parse_arff(tp).rows;
		let (_, tx, ty, _) = encode(attrs, &trows, targets, &skip)?;
		(
			Dataset {
				x,
				y,
				source: source_label.to_string(),
				n_targets: k,
				has_target: k > 0,
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
	};
	Ok(PreparedArff { train, test })
}

pub struct PreparedArff {
	pub train: Dataset,
	pub test: Option<Dataset>,
}

pub fn prepare_table_data(
	sources: &[String],
	test_path: Option<&str>,
	split_frac: Option<f64>,
	exclude: &[String],
	source_label: &str,
	pre: Option<&[GroupKinds]>,
	resolve: impl Fn(&[String], Option<&[String]>) -> anyhow::Result<Vec<String>>,
) -> anyhow::Result<PreparedTable> {
	let set_groups: Vec<DirGroup> = sources
		.iter()
		.map(|s| crate::data::load_groups(s))
		.collect::<anyhow::Result<Vec<Vec<DirGroup>>>>()?
		.into_iter()
		.flatten()
		.collect();
	let set_tnames = table_names(&set_groups);

	let test_groups = match test_path {
		Some(tp) => Some((crate::data::load_groups(tp)?, tp.to_string())),
		None => None,
	};
	let test_tnames: Option<Vec<String>> = match (&test_groups, split_frac) {
		(Some((g, _)), _) => Some(table_names(g)),
		(None, Some(_)) => Some(set_tnames.clone()),
		(None, None) => None,
	};
	let t = resolve(&set_tnames, test_tnames.as_deref())?;

	let (mut set, schema) = assemble(&set_groups, &t, None, None, exclude, pre)?;
	let flat_attrs: Vec<Attr> = schema.values().flat_map(|v| v.iter().cloned()).collect();
	let k = set.n_targets;
	let keep = |name: &str| !exclude.iter().any(|p| exclude_match(p, name));

	if let Some((tg, tp)) = &test_groups {
		let (mut test, _) = assemble(
			tg,
			&t,
			Some(&schema),
			Some(&set.sample_group),
			exclude,
			None,
		)?;
		let test_has_target =
			!t.is_empty() && t.iter().all(|tgt| test.names.iter().any(|n| n == tgt));
		let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
		let tc = text_col_indices(&feats);
		let oh = onehot_group_indices(&feats);
		let train = Dataset {
			x: set.select(&feats)?,
			y: set.y,
			source: source_label.to_string(),
			n_targets: k,
			has_target: k > 0,
			text_cols: tc.clone(),
			onehot_groups: oh.clone(),
		};
		let test_feats: Vec<String> = test.names.iter().filter(|n| keep(n)).cloned().collect();
		let oh_test = onehot_group_indices(&test_feats);
		let testds = Dataset {
			x: test.select(&test_feats)?,
			y: test.y,
			source: tp.clone(),
			n_targets: test.n_targets,
			has_target: test_has_target,
			text_cols: tc,
			onehot_groups: oh_test,
		};
		return Ok(PreparedTable {
			train,
			test: Some(testds),
			attrs: flat_attrs,
		});
	}

	let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
	let x = set.select(&feats)?;
	let tc = text_col_indices(&feats);
	let oh = onehot_group_indices(&feats);
	if let Some(frac) = split_frac {
		let Split {
			train: tr,
			test: te,
		} = shuffle_split(&x, &set.y, k, frac, source_label, &tc, &oh);
		return Ok(PreparedTable {
			train: tr,
			test: Some(te),
			attrs: flat_attrs,
		});
	}
	Ok(PreparedTable {
		train: Dataset {
			x,
			y: set.y,
			source: source_label.to_string(),
			n_targets: k,
			has_target: true,
			text_cols: tc,
			onehot_groups: oh,
		},
		test: None,
		attrs: flat_attrs,
	})
}

pub struct PreparedTable {
	pub train: Dataset,
	pub test: Option<Dataset>,
	pub attrs: Vec<Attr>,
}

pub struct CollapsedOnehot {
	pub x: Mat,
	pub embed_cols: Vec<usize>,
	pub vocab: usize,
}

pub struct SafeTable {
	pub attrs: Vec<Attr>,
	pub rows: Vec<Vec<String>>,
}

pub fn collapse_onehot(ds: &Dataset) -> anyhow::Result<CollapsedOnehot> {
	let n = ds.x.nrows();
	let ncols = ds.x.ncols();
	let mut in_group: BTreeSet<usize> = BTreeSet::new();
	for grp in &ds.onehot_groups {
		for c in grp.start..grp.start + grp.len {
			in_group.insert(c);
		}
	}
	let passthrough: Vec<usize> = (0..ncols).filter(|c| !in_group.contains(c)).collect();
	let n_cat = ds.onehot_groups.len();
	let new_ncols = passthrough.len() + n_cat;
	let mut dat = vec![0.0f64; n * new_ncols];
	for new_j in 0..passthrough.len() {
		let orig_j = passthrough[new_j];
		for i in 0..n {
			dat[i * new_ncols + new_j] = ds.x[[i, orig_j]];
		}
	}
	let embed_start = passthrough.len();
	let mut offset = 0usize;
	let spans = &ds.onehot_groups;
	for g in 0..spans.len() {
		let grp = &spans[g];
		let new_j = embed_start + g;
		for i in 0..n {
			let Some(c) = (0..grp.len).find(|&c| ds.x[[i, grp.start + c]] > 0.5) else {
				continue;
			};
			dat[i * new_ncols + new_j] = (offset + c) as f64;
		}
		offset += grp.len;
	}
	let embed_cols: Vec<usize> = (embed_start..embed_start + n_cat).collect();
	let x = Mat::from_shape_vec([n, new_ncols], dat)
		.map_err(|e| anyhow::anyhow!("collapse_onehot: {e:#}"))?;
	Ok(CollapsedOnehot {
		x,
		embed_cols,
		vocab: offset,
	})
}

pub fn safetensors_to_table(path: &str) -> anyhow::Result<SafeTable> {
	let bytes = fs::read(path).map_err(|e| anyhow::anyhow!("safetensors: read {path}: {e}"))?;
	let tensors = recipe_infer::safetensors::parse_safetensors_shaped(&bytes)
		.map_err(|e| anyhow::anyhow!("safetensors: {path}: {e}"))?;
	anyhow::ensure!(!tensors.is_empty(), "safetensors: {path} has no tensors");
	let n = {
		let head = &tensors[0];
		let hname = &head.name;
		head.shape.first().copied().ok_or_else(|| {
			anyhow::anyhow!("safetensors: tensor '{hname}' has no leading row dim")
		})?
	};
	let mut attrs = Vec::new();
	let mut cols: Vec<Vec<f64>> = Vec::new();
	for t in &tensors {
		let name = &t.name;
		let shape = &t.shape;
		let vals = &t.values;
		let leading = shape.first().copied().unwrap_or(0);
		anyhow::ensure!(
			leading == n,
			"safetensors: tensor '{name}' leading dim {leading} != {n}"
		);
		let width = shape.iter().skip(1).product::<usize>().max(1);
		for c in 0..width {
			let aname = match width.cmp(&1) {
				Ordering::Equal => name.clone(),
				Ordering::Less | Ordering::Greater => {
					format!("{name}:{c}")
				}
			};
			attrs.push(Attr {
				name: aname,
				kind: Kind::Numeric,
			});
			cols.push((0..n).map(|i| vals[i * width + c]).collect());
		}
	}
	let rows = (0..n)
		.map(|i| cols.iter().map(|col| format!("{}", col[i])).collect())
		.collect();
	Ok(SafeTable { attrs, rows })
}
