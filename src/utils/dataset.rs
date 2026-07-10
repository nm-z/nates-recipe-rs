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
	pub(crate) inner: Box<DataInner>,
}

#[doc(hidden)]
pub struct DataInner {
	pub target: String,
	target_names: Vec<String>,
	pub(crate) attrs: Vec<Attr>,
	rows: Vec<Vec<String>>,
	sources: Vec<String>,
	test_path: Option<String>,
	split_frac: Option<f64>,
	exclude: Vec<String>,
	raw_test_rows: Option<Vec<Vec<String>>>,
	raw_test_headers: Option<Vec<String>>,
	pre_kinds: pantry::encode::PreKinds,
	deferred: Option<anyhow::Error>,
}

impl std::ops::Deref for Data {
	type Target = DataInner;
	fn deref(&self) -> &DataInner {
		&self.inner
	}
}
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
	let r = Mat::from_shape_vec((n, new_ncols), data);
	assert!(r.is_ok(), "collapse_onehot: {}", r.as_ref().err().map(|e| e.to_string()).unwrap_or_default());
	let Ok(x) = r else { loop {} };
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

pub fn safetensors_to_table(path: &str) -> anyhow::Result<(Vec<Attr>, Vec<Vec<String>>)> {
	let bytes = std::fs::read(path).map_err(|e| anyhow::anyhow!("safetensors: read {path}: {e}"))?;
	let tensors = recipe_infer::safetensors::parse_safetensors_shaped(&bytes)
		.map_err(|e| anyhow::anyhow!("safetensors: {path}: {e}"))?;
	anyhow::ensure!(!tensors.is_empty(), "safetensors: {path} has no tensors");
	let n = tensors[0].1.first().copied().ok_or_else(|| {
		anyhow::anyhow!("safetensors: tensor '{}' has no leading row dim", tensors[0].0)
	})?;
	let mut attrs = Vec::new();
	let mut cols: Vec<Vec<f64>> = Vec::new();
	for (name, shape, vals) in &tensors {
		let leading = shape.first().copied().unwrap_or(0);
		anyhow::ensure!(leading == n, "safetensors: tensor '{name}' leading dim {leading} != {n}");
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
	Ok((attrs, rows))
}

impl Data {
	pub fn load(path: &str) -> Data {
		let data = Data {
			inner: Box::new(DataInner {
				target: String::new(),
				target_names: Vec::new(),
				attrs: Vec::new(),
				rows: Vec::new(),
				sources: Vec::new(),
				test_path: None,
				split_frac: None,
				exclude: Vec::new(),
				raw_test_rows: None,
				raw_test_headers: None,
				pre_kinds: Vec::new(),
				deferred: None,
			}),
		};
		data.set(path)
	}

	pub fn set(mut self, path: &str) -> Data {
		self.inner.sources.push(path.to_string());
		if is_arff(path) {
			let (attrs, rows) = crate::data::parse_arff(path);
			self.inner.attrs = attrs;
			self.inner.rows = rows;
		} else if is_safetensors(path) {
			match safetensors_to_table(path) {
				Ok((attrs, rows)) => {
					self.inner.attrs = attrs;
					self.inner.rows = rows;
				}
				Err(e) => self.inner.defer(e),
			}
		} else {
			match pantry::detect_kinds(path) {
				Ok(kinds) => self.inner.pre_kinds.extend(kinds),
				Err(e) => self.inner.defer(e),
			}
		}
		self
	}

	pub fn target(mut self, t: impl IntoTargets) -> Data {
		self.inner.target_names = t.into_targets();
		self.inner.target = self.inner.target_names.first().cloned().unwrap_or_default();
		if let Some(tp) = &self.inner.test_path {
			if let Ok((headers, rows)) = crate::data::read_raw_csv(std::path::Path::new(tp)) {
				if !headers.is_empty() {
					self.inner.raw_test_headers =
						Some(headers.into_iter().map(|h| h.trim().to_string()).collect());
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

	fn defer(&mut self, e: anyhow::Error) {
		if self.deferred.is_none() {
			self.deferred = Some(e);
		}
	}

	pub fn datasets(&self) -> (Dataset, Option<Dataset>) {
		let r = self.try_datasets();
		assert!(r.is_ok(), "Data::datasets: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let Ok(v) = r else { loop {} };
		v
	}

	pub(crate) fn try_datasets(&self) -> anyhow::Result<(Dataset, Option<Dataset>)> {
		let (train, test, attrs) = self.prepare()?;
		self.print_summary(&train, test.as_ref(), &attrs);
		Ok((train, test))
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
		eprintln!("    {} features -> model", train.x.ncols(),);
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
				eprintln!("    {} features -> model", test.x.ncols(),);
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

	fn prepare(&self) -> anyhow::Result<(Dataset, Option<Dataset>, Vec<Attr>)> {
		if let Some(e) = &self.deferred {
			anyhow::bail!("{e:#}");
		}
		let (mut train, mut test, attrs) = if self.attrs.is_empty() {
			self.prepare_table()?
		} else {
			let (tr, te) = self.prepare_arff()?;
			(tr, te, self.attrs.clone())
		};
		pantry::encode::clean_dataset(&mut train);
		if let Some(t) = test.as_mut() {
			pantry::encode::clean_dataset(t);
		}
		anyhow::ensure!(train.x.nrows() > 0, "dataset has 0 rows after NaN removal");
		anyhow::ensure!(train.x.ncols() > 0, "dataset has 0 feature columns");
		let k = train.n_targets;
		anyhow::ensure!(
			train.y.len() == train.x.nrows() * k,
			"x/y dimension mismatch: {} rows × {k} targets but y has {} elements",
			train.x.nrows(),
			train.y.len(),
		);
		Ok((train, test, attrs))
	}

	fn prepare_arff(&self) -> anyhow::Result<(Dataset, Option<Dataset>)> {
		let names: Vec<String> = self.attrs.iter().map(|a| a.name.clone()).collect();
		let resolved = self.resolve_targets(&names, None)?;
		let targets: Vec<usize> = resolved
			.iter()
			.map(|t| names.iter().position(|n| n == t).ok_or_else(|| anyhow::anyhow!("resolved from names")))
			.collect::<anyhow::Result<Vec<_>>>()?;
		pantry::encode::prepare_arff_data(
			&self.attrs,
			&self.rows,
			&targets,
			&self.exclude,
			self.split_frac,
			self.test_path.as_deref(),
			&self.source_label(),
		)
	}

	fn prepare_table(&self) -> anyhow::Result<(Dataset, Option<Dataset>, Vec<Attr>)> {
		pantry::encode::prepare_table_data(
			&self.sources,
			self.test_path.as_deref(),
			self.split_frac,
			&self.exclude,
			&self.source_label(),
			Some(self.pre_kinds.as_slice()),
			|s, t| self.resolve_targets(s, t),
		)
	}

	fn resolve_targets(
		&self,
		set_names: &[String],
		test_names: Option<&[String]>,
	) -> anyhow::Result<Vec<String>> {
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
						.ok_or_else(|| {
							let avail: Vec<&str> =
								set_names.iter().map(|s| s.as_str()).collect();
							anyhow::anyhow!(
								"target '{want}' not found — available columns: {}",
								avail.join(", ")
							)
						})
				})
				.collect();
		}
		if let Some(tn) = test_names {
			if set_names.len() == tn.len() + 1 {
				return Ok(vec![set_names.last().ok_or_else(|| anyhow::anyhow!("set has columns"))?.clone()]);
			}
		}
		Ok(Vec::new())
	}
}

impl crate::model::RunData for DataInner {
	fn prepared(&self) -> anyhow::Result<crate::model::Prepared<'_>> {
		let (train, _test) = self.try_datasets()?;
		Ok(crate::model::Prepared::Owned(train))
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

impl crate::model::RunData for Data {
	fn prepared(&self) -> anyhow::Result<crate::model::Prepared<'_>> {
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

