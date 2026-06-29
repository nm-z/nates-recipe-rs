use crate::Mat;
use pantry::encode::exclude_match;
use pantry::{Attr, Kind};
use std::cell::RefCell;

pub use pantry::encode::{Dataset, shuffle_split};

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

/// A lazy description of a dataset: which sources, target, split, exclusions.
/// Nothing is loaded or encoded until `Train::run` (or `datasets()`) asks for it
/// — so building a `Data`, even many of them, costs only the config it holds.
/// `Data` describes; `Train` executes.
///
/// The config lives in a heap-pinned [`DataInner`] so the builder's by-value
/// moves don't shift its address; the live-data registry holds a raw pointer to
/// it for the no-argument `.run()` / `.run(model)` resolution.
pub struct Data {
	pub(crate) inner: Box<DataInner>,
}

#[doc(hidden)]
pub struct DataInner {
	pub target: &'static str,
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

impl std::ops::Deref for Data {
	type Target = DataInner;
	fn deref(&self) -> &DataInner {
		&self.inner
	}
}
impl std::ops::DerefMut for Data {
	fn deref_mut(&mut self) -> &mut DataInner {
		&mut self.inner
	}
}
impl Drop for Data {
	fn drop(&mut self) {
		let r: &dyn crate::model::RunData = &*self.inner;
		deregister_data(r as *const dyn crate::model::RunData);
	}
}

// Every live `Data` registers the stable address of its heap config here, so
// `.run(())` / `.run(model)` can resolve "the Data in scope" with no argument.
thread_local! {
	static DATAS: RefCell<Vec<*const dyn crate::model::RunData>> =
		const { RefCell::new(Vec::new()) };
}
fn register_data(p: *const dyn crate::model::RunData) {
	DATAS.with(|d| d.borrow_mut().push(p));
}
fn deregister_data(p: *const dyn crate::model::RunData) {
	DATAS.with(|d| d.borrow_mut().retain(|&x| !std::ptr::addr_eq(x, p)));
}
/// The one live `Data`, or a clear panic when zero or several exist — backs the
/// no-argument `.run()` / `.run(model)` "use what's in scope" resolution.
pub(crate) fn the_data() -> *const dyn crate::model::RunData {
	DATAS.with(|d| {
		let d = d.borrow();
		match d.len() {
			1 => d[0],
			0 => panic!("run(): no Data in scope — build one with Data::load()…, or pass it: train.run((model, data))"),
			n => panic!("run(): {n} Datasets in scope — ambiguous; pass the one to run, e.g. train.run((model, data))"),
		}
	})
}

/// The `Dataset → Mat` seam for the embed-on-categoricals path: collapse each
/// one-hot group back to a single integer-index column (each category a unique
/// id, offset across groups) so an `embed` layer can look them up directly.
/// Returns `(collapsed matrix, the new embed-column indices, total vocab size)`.
/// Lives here, not in `pantry`, because it is the one place inference needs to
/// know what a `Dataset` is — and that knowledge stays up in this crate.
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

fn is_arff(path: &str) -> bool {
	std::path::Path::new(path)
		.extension()
		.and_then(|e| e.to_str())
		== Some("arff")
}

fn is_safetensors(path: &str) -> bool {
	std::path::Path::new(path).extension().and_then(|e| e.to_str()) == Some("safetensors")
}

/// A `.safetensors` source as a numeric table: each tensor's leading dim is the row
/// count, its trailing dims flatten to columns (`name` for a 1-D tensor, `name:c` per
/// column above that). Every column is `Numeric`; `.target(name)` selects which tensor
/// is the target, the rest are features. Feeds the same arff encode path.
fn safetensors_to_table(path: &str) -> (Vec<Attr>, Vec<Vec<String>>) {
	let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("safetensors: read {path}: {e}"));
	let tensors = recipe_infer::safetensors::parse_safetensors_shaped(&bytes)
		.unwrap_or_else(|e| panic!("safetensors: {path}: {e}"));
	assert!(!tensors.is_empty(), "safetensors: {path} has no tensors");
	let n = tensors[0].1.first().copied().unwrap_or_else(|| {
		panic!("safetensors: tensor '{}' has no leading row dim", tensors[0].0)
	});
	let mut attrs = Vec::new();
	let mut cols: Vec<Vec<f64>> = Vec::new();
	for (name, shape, vals) in &tensors {
		let leading = shape.first().copied().unwrap_or(0);
		assert_eq!(leading, n, "safetensors: tensor '{name}' leading dim {leading} != {n}");
		let width = shape.iter().skip(1).product::<usize>().max(1);
		for c in 0..width {
			let aname = if width == 1 { name.clone() } else { format!("{name}:{c}") };
			attrs.push(Attr { name: aname, kind: Kind::Numeric });
			cols.push((0..n).map(|i| vals[i * width + c]).collect());
		}
	}
	let rows = (0..n)
		.map(|i| cols.iter().map(|col| format!("{}", col[i])).collect())
		.collect();
	(attrs, rows)
}

impl Data {
	pub fn load() -> Data {
		let inner = Box::new(DataInner {
			target: "",
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
		});
		let r: &dyn crate::model::RunData = &*inner;
		register_data(r as *const dyn crate::model::RunData);
		Data { inner }
	}

	pub fn set(mut self, path: &str) -> Data {
		self.inner.sources.push(path.to_string());
		if is_arff(path) {
			let (attrs, rows) = crate::data::parse_arff(path);
			self.inner.attrs = attrs;
			self.inner.rows = rows;
		} else if is_safetensors(path) {
			let (attrs, rows) = safetensors_to_table(path);
			self.inner.attrs = attrs;
			self.inner.rows = rows;
		}
		self
	}

	pub fn target(mut self, t: impl IntoTargets) -> Data {
		self.inner.target_names = t.into_targets();
		self.inner.target = self
			.inner
			.target_names
			.first()
			.map_or("", |s| Box::leak(s.clone().into_boxed_str()));
		if !self.inner.attrs.is_empty() {
			let attrs = &self.inner.attrs;
			let targets = self
				.inner
				.target_names
				.iter()
				.map(|name| {
					attrs
						.iter()
						.position(|a| a.name == *name)
						.unwrap_or_else(|| {
							panic!("Data::target: no attribute named '{name}'")
						})
				})
				.collect();
			self.inner.targets = targets;
		}
		if let Some(tp) = &self.inner.test_path {
			if let Ok(text) = std::fs::read_to_string(tp) {
				let mut lines = text.lines();
				if let Some(header_line) = lines.next() {
					let headers = crate::data::split_fields(header_line)
						.into_iter()
						.map(|s| s.trim().to_string())
						.collect();
					let rows = lines
						.filter(|l| !l.trim().is_empty())
						.map(|l| crate::data::split_fields(l))
						.collect();
					self.inner.raw_test_headers = Some(headers);
					self.inner.raw_test_rows = Some(rows);
				}
			}
		}
		self
	}

	pub fn test(mut self, path: &str) -> Data {
		self.inner.test_path = Some(path.to_string());
		self
	}

	pub fn exclude(mut self, pattern: &str) -> Data {
		self.inner.exclude.push(pattern.to_string());
		self
	}

	pub fn split(mut self, train_frac: f64) -> Data {
		assert!(
			(0.0..1.0).contains(&train_frac),
			"split fraction must be in (0, 1), got {train_frac}",
		);
		self.inner.split_frac = Some(train_frac);
		self
	}
}

impl DataInner {
	fn source_label(&self) -> String {
		self.sources.join(", ")
	}

	/// Materialize this description into encoded `(train, Option<test>)` datasets,
	/// printing the summary as it goes. This is the ONLY place loading + encoding
	/// happens; `Train::run` calls it per run so exactly one dataset is resident at
	/// a time. Public so the CLI / tests can force materialization explicitly.
	pub fn datasets(&self) -> (Dataset, Option<Dataset>) {
		let (train, test, attrs) = self.prepare();
		self.print_summary(&train, test.as_ref(), &attrs);
		(train, test)
	}

	fn feature_type_counts(&self, attrs: &[Attr]) -> Vec<(&'static str, usize)> {
		let is_target = |name: &str| self.target_names.iter().any(|t| t == name);
		let is_excluded = |name: &str| self.exclude.iter().any(|p| exclude_match(p, name));
		let (mut numeric, mut temporal, mut categorical, mut ordinal, mut text, mut image) =
			(0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
		for a in attrs {
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

	fn cat_cardinality_counts(&self, attrs: &[Attr]) -> Vec<(usize, usize)> {
		let is_target = |name: &str| self.target_names.iter().any(|t| t == name);
		let is_excluded = |name: &str| self.exclude.iter().any(|p| exclude_match(p, name));
		let mut card: std::collections::BTreeMap<usize, usize> =
			std::collections::BTreeMap::new();
		for a in attrs {
			if is_target(&a.name) || is_excluded(&a.name) {
				continue;
			}
			if let Kind::Categorical(cats) = &a.kind {
				*card.entry(cats.len()).or_default() += 1;
			}
		}
		card.into_iter().collect()
	}

	fn print_summary(&self, train: &Dataset, test: Option<&Dataset>, attrs: &[Attr]) {
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
		let raw_cols = attrs.len();
		let types = self.feature_type_counts(attrs);
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
			train.x.nrows() + test.map_or(0, |t| t.x.nrows())
		} else {
			train.x.nrows()
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
		let cards = self.cat_cardinality_counts(attrs);
		if !cards.is_empty() {
			eprintln!("    encoding");
			for (card, count) in &cards {
				let range: Vec<String> = (0..*card).map(|i| i.to_string()).collect();
				eprintln!("        {count} × [{}]", range.join(", "));
			}
		}
		eprintln!("    {} features → model", train.x.ncols(),);
		if let Some(test) = test {
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
					train.x.nrows(),
					test.x.nrows(),
				);
			}
		}
		for t in &self.target_names {
			eprintln!("\x1b[32mtarget\x1b[0m  {t}");
		}
	}

	fn prepare(&self) -> (Dataset, Option<Dataset>, Vec<Attr>) {
		let (mut train, mut test, attrs) = if self.attrs.is_empty() {
			self.prepare_table()
		} else {
			let (tr, te) = self.prepare_arff();
			(tr, te, self.attrs.clone())
		};
		// The ONE NaN call site: each dataset's column-vectors are cleaned once here
		// as they enter the pipeline (missing-target rows dropped, feature NaNs
		// imputed). After this nothing downstream handles NaN again.
		pantry::encode::clean_dataset(&mut train);
		if let Some(t) = test.as_mut() {
			pantry::encode::clean_dataset(t);
		}
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
		pantry::encode::prepare_arff_data(
			&self.attrs,
			&self.rows,
			&self.targets,
			&self.exclude,
			self.split_frac,
			self.test_path.as_deref(),
			&self.source_label(),
		)
	}

	fn prepare_table(&self) -> (Dataset, Option<Dataset>, Vec<Attr>) {
		pantry::encode::prepare_table_data(
			&self.sources,
			self.test_path.as_deref(),
			self.split_frac,
			&self.exclude,
			&self.source_label(),
			|s, t| self.resolve_targets(s, t),
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

impl crate::model::RunData for DataInner {
	fn prepared(&self) -> crate::model::Prepared<'_> {
		let (train, _test) = self.datasets();
		crate::model::Prepared::Owned(train)
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

// `Data` forwards to its inner so an explicit `train.run((&model, &data))` (the
// only form a loop over several datasets can use) accepts a `&Data` directly,
// while the live-data registry holds the heap-pinned `DataInner`.
impl crate::model::RunData for Data {
	fn prepared(&self) -> crate::model::Prepared<'_> {
		self.inner.prepared()
	}
	fn target_names(&self) -> Vec<String> {
		self.inner.target_names()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		self.inner.raw_rows()
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		self.inner.raw_headers()
	}
	fn infer_only(&self) -> bool {
		self.inner.infer_only()
	}
}

#[cfg(test)]
mod safetensors_source_tests {
	use super::*;

	// Build a tiny .safetensors image (x: [3,2] F64 features, y: [3] F64 target), write
	// it to a temp file, and load it through the public Data builder. Host-only — encode
	// builds an ndarray Mat, no GPU. Proves .set("*.safetensors") is a real Data source.
	#[test]
	fn data_load_reads_safetensors_source() {
		let header = concat!(
			r#"{"x":{"dtype":"F64","shape":[3,2],"data_offsets":[0,48]},"#,
			r#""y":{"dtype":"F64","shape":[3],"data_offsets":[48,72]}}"#,
		);
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
		bytes.extend_from_slice(header.as_bytes());
		for v in [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0] {
			bytes.extend_from_slice(&v.to_le_bytes());
		}
		for v in [10.0f64, 20.0, 30.0] {
			bytes.extend_from_slice(&v.to_le_bytes());
		}
		let path = std::env::temp_dir().join("recipe_st_source_test.safetensors");
		std::fs::write(&path, &bytes).expect("write temp safetensors");
		let p = path.to_str().expect("temp path utf8");

		let (attrs, rows) = safetensors_to_table(p);
		assert_eq!(
			attrs.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
			vec!["x:0", "x:1", "y"]
		);
		assert!(attrs.iter().all(|a| matches!(a.kind, Kind::Numeric)));
		assert_eq!(rows.len(), 3);
		assert_eq!(rows[0], vec!["1", "2", "10"]);
		assert_eq!(rows[2], vec!["5", "6", "30"]);

		let data = Data::load().set(p).split(0.66).target("y");
		let (set, test) = data.datasets();
		assert_eq!(set.x.ncols(), 2, "two feature columns (x:0, x:1)");
		assert_eq!(set.n_targets, 1, "single target (y)");
		let total = set.x.nrows() + test.as_ref().map_or(0, |t| t.x.nrows());
		assert_eq!(total, 3, "all rows preserved across the split");
		let _ = std::fs::remove_file(&path);
	}

	// Run the real thing: feed an actual model weight shard ($ST_FILE) to Data::load().
	// Not a dataset — whatever happens (panic on heterogeneous tensor dims, or success)
	// is the experiment. Ignored by default; run with ST_FILE set.
	#[test]
	#[ignore = "set ST_FILE to a real .safetensors weight shard"]
	fn data_load_on_real_safetensors_shard() {
		let p = std::env::var("ST_FILE").expect("set ST_FILE");
		let d = Data::load().set(&p);
		let (set, _) = d.datasets();
		eprintln!("loaded: {} rows × {} cols", set.x.nrows(), set.x.ncols());
	}
}
