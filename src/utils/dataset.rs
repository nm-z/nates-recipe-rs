use crate::Mat;
use pantry::encode::exclude_match;
use pantry::{Attr, Kind};

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
			self.prepare_table()
		} else {
			let (tr, te) = self.prepare_arff();
			(tr, te, self.attrs.clone())
		};
		pantry::encode::report_nans(&train, test.as_ref());
		pantry::encode::drop_nan_samples(&mut train);
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
