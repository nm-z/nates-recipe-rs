use crate::{Mat, Vec1};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

/// A string column with more than this many distinct values is treated as free
/// TEXT (tokenized → embedding-id sequence) rather than a CATEGORICAL (one-hot).
/// model names, states, yes/no → one-hot; prompts, responses → text.
const ONEHOT_MAX: usize = 256;

/// Fixed token-sequence length per text column: each text cell is tokenized and
/// the first SEQ_LEN token ids are emitted as SEQ_LEN integer columns (id 0 =
/// pad/out-of-vocab), padded when short. The `embed` layer looks these ids up.
const SEQ_LEN: usize = 32;

/// Tokenize text into lowercased alphanumeric tokens (whitespace/punctuation are
/// separators). Used both to build a column's vocabulary and to emit token ids.
fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
	s.split(|c: char| !c.is_alphanumeric())
		.filter(|t| !t.is_empty())
		.map(|t| t.to_ascii_lowercase())
}

#[derive(Clone)]
enum Kind {
	Numeric,
	Nominal(Vec<String>),
	// Free text: the column's token vocabulary (sorted distinct tokens). A token's
	// id is its index+1 (0 = pad / out-of-vocab). Encoded as SEQ_LEN id columns,
	// consumed by an `embed` layer — never one-hot (that OOMs at ~n columns).
	Text(Vec<String>),
}

#[derive(Clone)]
struct Attr {
	name: String,
	kind: Kind,
}

/// Accepts one target column (`"y"`) or several (`["a","b","c"]`) for `.target`.
/// Rust methods aren't variadic, so multiple columns come in as an array/slice.
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

/// Data loader: CSV, ARFF, or a directory of correlated files.
///
/// ```rust,no_run
/// # use nates_recipe::*;
/// let data = Data::load()
///     .set("train.csv")
///     .test("test.csv")
///     .exclude("Id")
///     .target("Price");
/// ```
pub struct Data {
	pub target: &'static str,
	pub set: Dataset,
	pub test: Option<Dataset>,
	target_names: Vec<String>,
	attrs: Vec<Attr>,
	rows: Vec<Vec<String>>,
	targets: Vec<usize>,
	source: String,
	test_path: Option<String>,
	split_frac: Option<f64>,
	exclude: Vec<String>,
	raw_test_rows: Option<Vec<Vec<String>>>,
	raw_test_headers: Option<Vec<String>>,
}

pub struct Dataset {
	pub x: Mat,
	// Targets, flat row-major: n*n_targets values, row r's targets at [r*k .. r*k+k].
	// For a single target this is just the n-long column (k=1), unchanged.
	pub y: Vec1,
	pub source: String,
	// Number of target columns (k). Output layer must have this many units.
	pub n_targets: usize,
	// True when the target column(s) were present in this set (train always; a
	// Kaggle test.csv has no target → false → eval skips scoring, still predicts).
	pub has_target: bool,
	// Indices of `x` columns that are token-id columns (from free-text columns,
	// named `*#t{s}`). An `embed` first layer consumes ONLY these — the rest
	// (numeric/one-hot) are not token ids and would explode the vocab.
	pub text_cols: Vec<usize>,
}

/// Split one ARFF/CSV line into trimmed, unquoted fields, respecting single-quote quoting.
fn split_fields(line: &str) -> Vec<String> {
	let mut out = Vec::new();
	let mut cur = String::new();
	let mut quoted = false;
	for c in line.chars() {
		match c {
			'\'' => quoted = !quoted,
			',' if !quoted => {
				out.push(cur.trim().to_string());
				cur.clear();
			}
			_ => cur.push(c),
		}
	}
	out.push(cur.trim().to_string());
	out
}

/// Read + parse an ARFF file into (attributes, data rows). Exits on read error.
fn parse_arff(path: &str) -> (Vec<Attr>, Vec<Vec<String>>) {
	let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
		if e.kind() == std::io::ErrorKind::NotFound {
			let cwd = std::env::current_dir()
				.map(|p| p.display().to_string())
				.unwrap_or_else(|_| ".".to_string());
			let name = std::path::Path::new(path)
				.file_name()
				.and_then(|s| s.to_str())
				.unwrap_or(path);
			panic!("couldn't find '{name}' in {cwd}");
		} else {
			panic!("Data: cannot read {path}: {e}");
		}
	});
	let mut attrs = Vec::new();
	let mut rows = Vec::new();
	let mut in_data = false;
	for raw in text.lines() {
		let line = raw.trim();
		if line.is_empty() || line.starts_with('%') {
			continue;
		}
		if in_data {
			rows.push(split_fields(line));
			continue;
		}
		let lower = line.to_ascii_lowercase();
		if lower.starts_with("@attribute") {
			attrs.push(parse_attribute(line));
		} else if lower.starts_with("@data") {
			in_data = true;
		}
	}
	assert!(!attrs.is_empty(), "Data: no @attribute lines in {path}");
	assert!(!rows.is_empty(), "Data: no @data rows in {path}");
	(attrs, rows)
}

/// Cell `j` of a raw row as `&str` (missing/short → "").
fn cell(row: &[String], j: usize) -> &str {
	row.get(j).map_or("", |s| s.as_str())
}

/// Infer an ARFF-style schema from raw rows: a column is `Numeric` if every
/// non-empty cell parses as f64, else `Nominal` with its sorted distinct values.
/// Categories are taken from whatever rows are passed (always the SET, so a test
/// file is later encoded against the same category list).
fn infer_attrs(headers: &[String], rows: &[Vec<String>], known: Option<&[Attr]>) -> Vec<Attr> {
	let attrs: Vec<Attr> = headers
		.iter()
		.enumerate()
		.map(|(j, name)| {
			// A column already in the SET schema (a test source sharing a train
			// column) reuses the train Kind verbatim — same vocab/categories AND
			// no re-inference, so the heavy token-vocab build never runs twice.
			if let Some(sa) = known.and_then(|k| k.iter().find(|s| s.name == *name)) {
				return Attr {
					name: name.clone(),
					kind: sa.kind.clone(),
				};
			}
			// Numeric test in parallel; rayon's `all` short-circuits on the first
			// non-numeric cell, so a text column bails almost immediately.
			let numeric = rows
				.par_iter()
				.all(|row| cell(row, j).is_empty() || cell(row, j).parse::<f64>().is_ok());
			let kind = if numeric {
				Kind::Numeric
			} else {
				// Distinct non-empty cells, counted with an early exit at
				// ONEHOT_MAX: a free-text column trips the limit after a few
				// hundred rows, so the full distinct set (millions of long
				// strings) is never built. Borrow &str — no per-cell clone.
				let mut distinct: std::collections::HashSet<&str> =
					std::collections::HashSet::new();
				let mut overflow = false;
				for row in rows {
					let c = cell(row, j);
					if c.is_empty() {
						continue;
					}
					distinct.insert(c);
					if distinct.len() > ONEHOT_MAX {
						overflow = true;
						break;
					}
				}
				if !overflow {
					let mut cats: Vec<String> =
						distinct.into_iter().map(|c| c.to_string()).collect();
					cats.sort_unstable();
					Kind::Nominal(cats)
				} else {
					// Free text: build the token vocabulary (sorted distinct
					// tokens across all cells) for embedding-id encoding. The
					// serial BTreeSet insert was the load bottleneck (~millions
					// of inserts over the long-text columns); collect per-thread
					// HashSets across rows, union, then a single parallel sort.
					let set = rows
						.par_iter()
						.fold(
							std::collections::HashSet::<String>::new,
							|mut acc, row| {
								acc.extend(tokenize(cell(row, j)));
								acc
							},
						)
						.reduce(std::collections::HashSet::<String>::new, |a, b| {
							let (mut big, small) =
								if a.len() >= b.len() { (a, b) } else { (b, a) };
							big.extend(small);
							big
						});
					let mut vocab: Vec<String> = set.into_iter().collect();
					vocab.par_sort_unstable();
					Kind::Text(vocab)
				}
			};
			Attr {
				name: name.clone(),
				kind,
			}
		})
		.collect();
	attrs
}

/// Encode raw `rows` against `attrs` into `(feature_names, X, y)`. Feature
/// numerics pass through (blank/unparseable → NaN); feature nominals one-hot as
/// `name=cat`. Each `targets` column (if any) is label-encoded for a Nominal kind
/// and parsed for a Numeric kind; a blank/unseen value → NaN. `y` is flat,
/// row-major n*k (k = targets.len()), so row r's targets are y[r*k .. r*k+k].
/// With `targets = []`, every column is a feature and `y` is empty.
fn encode(
	attrs: &[Attr],
	rows: &[Vec<String>],
	targets: &[usize],
	skip: &[bool],
) -> (Vec<String>, Mat, Vec1) {
	let n = rows.len();
	let k = targets.len();
	// RAM guard BEFORE any one-hot column is allocated: project the feature width
	// from each KEPT column's cardinality (Nominal → one col per category). A
	// free-text/ID column explodes to ~n columns → an n×n dense matrix. If that
	// won't fit, fail clean naming the biggest expansions and exit — no silent
	// cap, the user `.exclude()`s the offenders and decides.
	let is_target = |ai: usize| targets.contains(&ai);
	let is_skip = |ai: usize| skip.get(ai).copied().unwrap_or(false);
	let width = |a: &Attr| match &a.kind {
		Kind::Numeric => 1,
		Kind::Nominal(c) => c.len(),
		Kind::Text(_) => SEQ_LEN,
	};
	let proj_w: usize = attrs
		.iter()
		.enumerate()
		.filter(|(ai, _)| !is_target(*ai) && !is_skip(*ai))
		.map(|(_, a)| width(a))
		.sum();
	let bytes = n
		.saturating_mul(proj_w)
		.saturating_mul(std::mem::size_of::<f64>());
	let avail = available_ram_bytes();
	if bytes > avail / 10 * 9 {
		let mut top: Vec<(&str, usize)> = attrs
			.iter()
			.enumerate()
			.filter(|(ai, a)| !is_target(*ai) && !is_skip(*ai) && width(a) > 1)
			.map(|(_, a)| (a.name.as_str(), width(a)))
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
				Kind::Nominal(cats) => {
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
				Kind::Text(_) => {
					panic!("target '{}' is free text — not a valid target", attr.name);
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
			Kind::Nominal(cats) => {
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
			// Free text → SEQ_LEN token-id columns (id = vocab index+1, 0 = pad/OOV).
			// The `embed` layer turns each id into a learned vector; never one-hot.
			Kind::Text(vocab) => {
				let base = cols.len();
				for s in 0..SEQ_LEN {
					names.push(format!("{}#t{s}", attr.name));
					cols.push(vec![0.0f64; n]);
				}
				// Tokenize + vocab-lookup per row in parallel (each row is
				// independent), then scatter the SEQ_LEN ids into the columns.
				let per_row: Vec<[f64; SEQ_LEN]> = rows
					.par_iter()
					.map(|row| {
						let mut ids = [0.0f64; SEQ_LEN];
						for (s, tok) in
							tokenize(cell(row, ai)).take(SEQ_LEN).enumerate()
						{
							ids[s] = vocab
								.binary_search(&tok)
								.map_or(0.0, |p| (p + 1) as f64);
						}
						ids
					})
					.collect();
				for (r, ids) in per_row.iter().enumerate() {
					for s in 0..SEQ_LEN {
						cols[base + s][r] = ids[s];
					}
				}
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

/// Per-group inferred schema, so a `.test` source encodes against the SET's
/// categories (keyed by group name; `""` for a single un-grouped file).
type Schema = std::collections::BTreeMap<String, Vec<Attr>>;

/// A source assembled into one training table: namespaced feature `names`, design
/// matrix `x`, target `y`, the `samples` count, and any groups `skipped` because
/// their rows couldn't be hash-aligned to the sample group.
struct Assembled {
	names: Vec<String>,
	// Parallel to `names`: (matrix index, column) locating each feature's values.
	sources: Vec<(usize, usize)>,
	// Per-source-matrix encoded values at that group's own row count (mats[0] is
	// the sample group, n rows; others are un-broadcast group matrices).
	mats: Vec<Mat>,
	// Per matrix, per-sample source row (None → NaN). gather[0] is identity.
	gather: Vec<Vec<Option<usize>>>,
	// Targets, flat row-major n*n_targets.
	y: Vec1,
	n_targets: usize,
	samples: usize,
	#[allow(dead_code)]
	skipped: Vec<String>,
	sample_group: String,
}

impl Assembled {
	/// Materialize the `keep` columns (in order) into a dense `n × keep.len()`
	/// matrix, gathering each from its group matrix via that group's per-sample
	/// index. Only kept columns are built — a dropped/excluded group costs nothing.
	fn select(&self, keep: &[String]) -> Mat {
		let n = self.samples;
		let w = keep.len();
		// The dense matrix is n×w×8B. If that won't fit in RAM, fail clean BEFORE
		// allocating — and name the columns whose one-hot expansion blew it up
		// (text/ID columns become one column per distinct value). No silent cap:
		// the user excludes the offending columns and decides.
		let bytes = n
			.saturating_mul(w)
			.saturating_mul(std::mem::size_of::<f64>());
		let avail = available_ram_bytes();
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

/// Available RAM in bytes (Linux MemAvailable). usize::MAX if it can't be read,
/// so the guard never blocks a legitimate run on a parse failure.
pub(crate) fn available_ram_bytes() -> usize {
	std::fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(usize::MAX, |kb| kb.saturating_mul(1024))
}

/// Namespaced feature name: bare for an un-grouped file, `group:col` for a dir.
fn namespaced(group: &str, col: &str) -> String {
	if group.is_empty() {
		col.to_string()
	} else {
		format!("{group}:{col}")
	}
}

/// Namespaced names of every TABLE column across the groups — the only target
/// candidates (a pixel is never a target), used for target resolution.
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

/// Load a path into raw groups: a directory → one group per file type; a single
/// file → one un-grouped Table (no hash). Encoding happens later, in `assemble`.
fn load_groups(path: &str) -> Vec<DirGroup> {
	if std::path::Path::new(path).is_dir() {
		crate::data::load_dir_groups(path).expect("load dir")
	} else {
		let (headers, cells) =
			crate::data::read_raw_csv(std::path::Path::new(path)).expect("read csv");
		let hashes = vec![String::new(); cells.len()];
		vec![DirGroup::Table {
			name: String::new(),
			headers,
			hashes,
			cells,
		}]
	}
}

/// Encode one group into `(namespaced_names, X, y)`. A Table uses its (set-inferred)
/// schema; the target column is encoded into `y` only for the sample group
/// (`target_col = Some`). An Image is already numeric. Hashes are read separately
/// via `group_hashes` so a skipped group never needs encoding.
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
			// Infer from THIS source's own headers (so a test with fewer/extra
			// columns is encoded by name, not position), but reuse the SET's
			// category lists for shared columns so one-hot columns match up.
			let attrs = infer_attrs(
				headers,
				cells,
				schema_in.and_then(|s| s.get(name)).map(Vec::as_slice),
			);
			schema.insert(name.clone(), attrs.clone());
			// Excluded attrs are dropped HERE, before one-hot expansion, so a
			// high-cardinality text/ID column is never materialized (no OOM).
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

/// Per-row hashes of a group (the sample-correlation ids), borrowed cheaply.
fn group_hashes(g: &DirGroup) -> &[String] {
	match g {
		DirGroup::Table { hashes, .. } | DirGroup::Image { hashes, .. } => hashes,
	}
}

/// Assemble raw `groups` into one table. The group owning `target` defines the
/// samples (its rows, at full resolution); every other group is joined by hash —
/// a 1-row-per-hash group broadcasts onto the sample rows, an equal-rows-per-hash
/// group aligns by within-hash position, and a group whose row counts don't match
/// is reported in `skipped` and left out (never aggregated). `schema_in` (the set
/// schema) is reused so a test source encodes against the same categories.
fn assemble(
	groups: &[DirGroup],
	targets: &[String],
	schema_in: Option<&Schema>,
	sample_hint: Option<&str>,
	exclude: &[String],
) -> (Assembled, Schema) {
	let mut schema: Schema = Schema::new();

	// The sample group is the table holding the target column(s); collect every
	// target's column index within it (matched by namespaced name).
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
	// Target columns absent here (e.g. an unlabeled test source): keep the SET's
	// sample group via `sample_hint`; else the sole group / sole table group.
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

	// Encode the sample group (carries the target). Its matrix backs mats[0] with
	// an identity gather. Other groups push their OWN (un-broadcast) matrix plus a
	// per-sample gather index — values are materialized later, only for kept
	// columns, so a dropped/excluded image group never broadcasts (no blow-up).
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

	// Sample-group hash bookkeeping: count per hash + each row's within-hash position.
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
		// Decide the join from hashes ALONE — never encode a group we'll skip
		// (a mismatched 5M-row group must not be materialized just to be dropped).
		let g_hashes = group_hashes(g);
		let mut by_hash: std::collections::HashMap<&str, Vec<usize>> =
			std::collections::HashMap::new();
		for (gi2, h) in g_hashes.iter().enumerate() {
			by_hash.entry(h.as_str()).or_default().push(gi2);
		}
		// Unrelated table: shares no hash with the sample group (a separate table
		// in a relational dump — e.g. Cities.csv vs SampleSubmission.csv). Report
		// it, don't fabricate NaN columns by "joining" on nothing.
		let shares = by_hash.keys().any(|h| s_count.contains_key(*h));
		if !shares {
			skipped.push(format!(
                        "{}: shares no join key with the sample group — separate table, use .exclude() or join manually",
                        group_name(g)
                  ));
			continue;
		}
		let all_one = by_hash.values().all(|v| v.len() == 1);
		// Aligns if, for every sample hash present in this group, counts match.
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
		// An image group is one image per well: broadcasting its pixels into every
		// sample row would duplicate it thousands of times (the 125 GB blow-up).
		// Keep it OUT of the dense matrix — it stays one copy per well, linked by
		// hash, for an index-based GPU path, not copied into rows here.
		if matches!(g, DirGroup::Image { .. }) {
			skipped.push(format!(
                        "{}: image group ({} wells) kept out of the feature matrix — one copy per well, not duplicated into rows",
                        group_name(g), by_hash.len()
                  ));
			continue;
		}
		let (g_names, g_x, _gy) = encode_group(g, &mut schema, schema_in, &[], exclude);
		// Per-sample source row in this group (or None → NaN), broadcast (1/hash)
		// or position-aligned (equal counts). Values gathered lazily in `select`.
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

/// A group's display name (`""` un-grouped file shows as its path elsewhere).
fn group_name(g: &DirGroup) -> &str {
	match g {
		DirGroup::Table { name, .. } | DirGroup::Image { name, .. } => name,
	}
}

/// Per-attr drop mask for one group: true where the attr's namespaced name matches
/// any `.exclude` pattern. Applied before one-hot expansion so excluded columns are
/// never built (the only place a high-cardinality text/ID column can be stopped
/// before it OOMs).
fn is_id_column(name: &str) -> bool {
	let bare = name.rsplit_once(':').map(|(_, c)| c).unwrap_or(name);
	bare.eq_ignore_ascii_case("id")
}

fn exclude_mask(attrs: &[Attr], group: &str, exclude: &[String]) -> Vec<bool> {
	attrs.iter()
		.map(|a| {
			let nm = namespaced(group, &a.name);
			if is_id_column(&nm) {
				return true;
			}
			exclude.iter().any(|p| exclude_match(p, &nm))
		})
		.collect()
}

/// Does exclude `pattern` match feature `name`? Exact, `group:*` glob, group name,
/// or bare header (the part after `:`).
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
	/// Start a data builder. Call `.set(path)` to load the predictors file.
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
			},
			test: None,
			target_names: Vec::new(),
			attrs: Vec::new(),
			rows: Vec::new(),
			targets: Vec::new(),
			source: String::new(),
			test_path: None,
			split_frac: None,
			exclude: Vec::new(),
			raw_test_rows: None,
			raw_test_headers: None,
		}
	}

	/// Load the predictors. Accepts anything: an ARFF file (parsed now, schema
	/// known up front), or a CSV file / directory of hash-correlated samples
	/// (loaded as a named table in `prepare`). The `.target` is found in the set.
	pub fn set(mut self, path: &str) -> Data {
		self.source = path.to_string();
		if is_arff(path) {
			let (attrs, rows) = parse_arff(path);
			self.attrs = attrs;
			self.rows = rows;
		}
		self
	}

	/// Name the target column(s): `.target("y")` or `.target(["a","b","c"])`.
	/// For an ARFF set each is resolved to its attribute index now; for a tabular
	/// set/dir they're matched against column names in `prepare`.
	pub fn target(mut self, t: impl IntoTargets) -> Data {
		self.target_names = t.into_targets();
		self.target = self.target_names.first().map_or("", |s| Box::leak(s.clone().into_boxed_str()));
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
					self.raw_test_headers = Some(
						split_fields(header_line).into_iter().map(|s| s.trim().to_string()).collect(),
					);
					self.raw_test_rows = Some(
						lines
							.filter(|l| !l.trim().is_empty())
							.map(|l| split_fields(l))
							.collect(),
					);
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
		let is_excluded = |name: &str| {
			is_id_column(name) || self.exclude.iter().any(|p| exclude_match(p, name))
		};
		let (mut numeric, mut categorical, mut text) = (0usize, 0usize, 0usize);
		for a in &self.attrs {
			if is_target(&a.name) || is_excluded(&a.name) {
				continue;
			}
			match &a.kind {
				Kind::Numeric => numeric += 1,
				Kind::Nominal(cats) => categorical += cats.len(),
				Kind::Text(_) => text += SEQ_LEN,
			}
		}
		let mut out = Vec::new();
		if numeric > 0 {
			out.push(("numeric", numeric));
		}
		if categorical > 0 {
			out.push(("categorical", categorical));
		}
		if text > 0 {
			out.push(("text", text));
		}
		out
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
		let types = self.feature_type_counts();
		let print_types = |indent: &str| {
			if types.len() == 1 {
				eprintln!("{indent}{} {} features", types[0].1, types[0].0);
			} else {
				for (kind, count) in &types {
					eprintln!("{indent}{count} {kind}");
				}
			}
		};
		eprintln!(
			"\x1b[32mset\x1b[0m  {}",
			short(&self.source),
		);
		eprintln!(
			"    {} rows  {} cols  {}",
			self.set.x.nrows(),
			self.set.x.ncols() + self.set.n_targets.max(1),
			disk_size(&self.source),
		);
		print_types("    ");
		for ex in &self.exclude {
			eprintln!("    excluded  {ex}");
		}
		if let Some(test) = &self.test {
			if let Some(tp) = &self.test_path {
				eprintln!(
					"\x1b[32mtest\x1b[0m  {}",
					short(tp),
				);
				eprintln!(
					"    {} rows  {} cols  {}",
					test.x.nrows(),
					test.x.ncols() + test.n_targets.max(1),
					disk_size(tp),
				);
				print_types("    ");
			} else if self.split_frac.is_some() {
				let total = self.set.x.nrows() + test.x.nrows();
				eprintln!(
					"\x1b[32msplit\x1b[0m  {}/{} rows (train/test from {})",
					self.set.x.nrows(),
					test.x.nrows(),
					total,
				);
			}
		}
		for t in &self.target_names {
			eprintln!("\x1b[32mtarget\x1b[0m  {t}");
		}
	}

	/// Use a separate pre-split test file (encoded with the train schema).
	pub fn test(mut self, path: &str) -> Data {
		self.test_path = Some(path.to_string());
		self
	}

	/// Drop features matching `pattern`: an exact column name, a `group:*` glob,
	/// a group name, or a bare header (matches that column in any group).
	pub fn exclude(mut self, pattern: &str) -> Data {
		self.exclude.push(pattern.to_string());
		self
	}

	/// Hold out `1 - train_frac` of the `.set` file as the test set.
	pub fn split(mut self, train_frac: f64) -> Data {
		assert!(
			(0.0..1.0).contains(&train_frac),
			"split fraction must be in (0, 1), got {train_frac}",
		);
		self.split_frac = Some(train_frac);
		self
	}

	/// Build a train `Dataset` and an optional test `Dataset`. An ARFF set keeps
	/// its self-describing schema path; anything else (CSV file or directory, for
	/// both `.set` and `.test`) goes through the unified named-table path that
	/// auto-detects features + target, aligns train↔test on shared columns, and
	/// prints its interpretation.
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

	/// ARFF set: encode against the declared schema; a `.test` ARFF is encoded
	/// with that same schema (so it aligns by construction).
	fn prepare_arff(&self) -> (Dataset, Option<Dataset>) {
		let k = self.targets.len().max(1);
		let skip = exclude_mask(&self.attrs, "", &self.exclude);
		let (names, x, y) = encode(&self.attrs, &self.rows, &self.targets, &skip);
		let tc = text_col_indices(&names);
		if let Some(frac) = self.split_frac {
			let (tr, te) = shuffle_split(&x, &y, k, frac, &self.source, &tc);
			(tr, Some(te))
		} else if let Some(tp) = &self.test_path {
			let (_, trows) = parse_arff(tp);
			let (_, tx, ty) = encode(&self.attrs, &trows, &self.targets, &skip);
			(
				Dataset {
					x,
					y,
					source: self.source.clone(),
					n_targets: k,
					has_target: true,
					text_cols: tc.clone(),
				},
				Some(Dataset {
					x: tx,
					y: ty,
					source: tp.clone(),
					n_targets: k,
					has_target: true,
					text_cols: tc,
				}),
			)
		} else {
			(
				Dataset {
					x,
					y,
					source: self.source.clone(),
					n_targets: k,
					has_target: true,
					text_cols: tc,
				},
				None,
			)
		}
	}

	/// Unified path for CSV files and directories. Each source is parsed into raw
	/// groups, then `assemble`d into one table where the group owning `.target`
	/// defines the samples and the rest are hash-joined. Train/test align on the
	/// shared (namespaced) feature columns; `.exclude` patterns are dropped. The
	/// target is `.target` (matched by name) or, when a test exists, the lone
	/// train-only column.
	fn prepare_table(&self) -> (Dataset, Option<Dataset>, Vec<Attr>) {
		let set_groups = load_groups(&self.source);
		let set_tnames = table_names(&set_groups);

		let test_groups = self
			.test_path
			.as_ref()
			.map(|tp| (load_groups(tp), tp.clone()));
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
			// A Kaggle test.csv legitimately omits the target(s) — that's the
			// thing to predict. has_target only when ALL target cols are present.
			let test_has_target =
				!t.is_empty() && t.iter().all(|tgt| test.names.iter().any(|n| n == tgt));
			let tset: std::collections::HashSet<&str> =
				test.names.iter().map(|s| s.as_str()).collect();
			let feats: Vec<String> = set
				.names
				.iter()
				.filter(|n| tset.contains(n.as_str()) && keep(n))
				.cloned()
				.collect();
			assert!(
				!feats.is_empty(),
				"set ({}) and test ({tp}) share no feature columns — data is non-correlated",
				self.source
			);
			let tc = text_col_indices(&feats);
			let train = Dataset {
				x: set.select(&feats),
				y: set.y,
				source: self.source.clone(),
				n_targets: k,
				has_target: true,
				text_cols: tc.clone(),
			};
			let testds = Dataset {
				x: test.select(&feats),
				y: test.y,
				source: (*tp).clone(),
				n_targets: test.n_targets,
				has_target: test_has_target,
				text_cols: tc,
			};
			return (train, Some(testds), flat_attrs);
		}

		let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
		let x = set.select(&feats);
		let tc = text_col_indices(&feats);
		if let Some(frac) = self.split_frac {
			let (tr, te) = shuffle_split(&x, &set.y, k.max(1), frac, &self.source, &tc);

			return (tr, Some(te), flat_attrs);
		}
		(
			Dataset {
				x,
				y: set.y,
				source: self.source.clone(),
				n_targets: k,
				has_target: true,
				text_cols: tc,
			},
			None,
			flat_attrs,
		)
	}

	/// Resolve the target column name. `.target` wins (matched exactly, as a
	/// `group:name`, or by the trailing `:name`); otherwise, when a test set is
	/// present, a single column that exists in the set but not the test is taken
	/// as the target (the thing to predict). Ambiguous (many train-only columns)
	/// or absent → `None`.
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
			let tset: std::collections::HashSet<&str> =
				tn.iter().map(|s| s.as_str()).collect();
			let only: Vec<&String> = set_names
				.iter()
				.filter(|n| !tset.contains(n.as_str()))
				.collect();
			if only.len() == 1 {
				return vec![only[0].clone()];
			}
		}
		Vec::new()
	}
}

fn col_after(c: &str) -> &str {
	c.split_once(':').map_or(c, |(_, s)| s)
}

/// Whether `path` is an ARFF file (by extension) — the only self-describing
/// format that bypasses the named-table path.
fn is_arff(path: &str) -> bool {
	std::path::Path::new(path)
		.extension()
		.and_then(|e| e.to_str())
		== Some("arff")
}

/// Drop train samples carrying any NaN — in the target or any feature. Test is
/// left intact (its samples must still be predicted). A reasonable default until
/// feature imputation is added.
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

/// `(NaN cells in features, feature rows touched, NaN cells in target)`.
pub(crate) fn nan_stats(d: &Dataset) -> (usize, usize, usize) {
	let cells = d.x.iter().filter(|v| v.is_nan()).count();
	let rows =
		d.x.outer_iter()
			.filter(|r| r.iter().any(|v| v.is_nan()))
			.count();
	let target = d.y.iter().filter(|v| v.is_nan()).count();
	(cells, rows, target)
}

/// OGDL warning of where NaNs live in the final train/test data — printed only
/// when some exist. Detection only; handling is decided by the caller.
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

/// Shuffled `train_frac`/`1-train_frac` row split (seed 42). `y` is flat row-major
/// n*k (k targets); each selected row carries its whole k-wide target slice.
fn shuffle_split(
	x: &Mat,
	y: &Vec1,
	k: usize,
	train_frac: f64,
	source: &str,
	text_cols: &[usize],
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
		}
	};
	(take(&idx[..n_train]), take(&idx[n_train..]))
}

/// Indices of feature columns that are token-id columns (from free text, named
/// `*#t{s}`) — the columns an `embed` layer consumes.
fn text_col_indices(feats: &[String]) -> Vec<usize> {
	feats.iter()
		.enumerate()
		.filter(|(_, n)| n.contains("#t"))
		.map(|(i, _)| i)
		.collect()
}

/// Parse one `@attribute 'name' { a, b }` or `@attribute name real` line.
fn parse_attribute(line: &str) -> Attr {
	let rest = line["@attribute".len()..].trim();
	let (name, spec) = if let Some(r) = rest.strip_prefix('\'') {
		let end = r.find('\'').expect("attribute: unterminated quoted name");
		(r[..end].to_string(), r[end + 1..].trim())
	} else {
		let end = rest
			.find(char::is_whitespace)
			.expect("attribute: missing type");
		(rest[..end].to_string(), rest[end..].trim())
	};
	let kind = if spec.starts_with('{') {
		let inner = spec.trim_start_matches('{').trim_end_matches('}');
		Kind::Nominal(split_fields(inner))
	} else {
		Kind::Numeric
	};
	Attr { name, kind }
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
}
