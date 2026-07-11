use crate::Mat;
use pantry::encode::exclude_match;
use pantry::{Attr, Kind};

pub use pantry::encode::{Dataset, shuffle_split};

use crate::model::InferOnly;

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

pub struct Datasets {
	pub train: Dataset,
	pub test: Option<Dataset>,
}

struct PreparedSets {
	train: Dataset,
	test: Option<Dataset>,
	attrs: Vec<Attr>,
}

pub(crate) struct CollapsedOnehot {
	pub x: Mat,
	pub embed_cols: Vec<usize>,
	pub vocab: usize,
}

pub struct SafeTable {
	pub attrs: Vec<Attr>,
	pub rows: Vec<Vec<String>>,
}

struct TypeCount {
	label: &'static str,
	count: usize,
}

struct CardCount {
	card: usize,
	count: usize,
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
pub(crate) fn collapse_onehot(ds: &Dataset) -> CollapsedOnehot {
	let n = ds.x.nrows();
	let ncols = ds.x.ncols();
	let mut in_group: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
	for grp in &ds.onehot_groups {
		for c in grp.start..grp.start + grp.len {
			in_group.insert(c);
		}
	}
	let passthrough: Vec<usize> = (0..ncols).filter(|c| !in_group.contains(c)).collect();
	let n_cat = ds.onehot_groups.len();
	let new_ncols = passthrough.len() + n_cat;
	let mut data = vec![0.0f64; n * new_ncols];
	for new_j in 0..passthrough.len() {
		let orig_j = passthrough[new_j];
		for i in 0..n {
			data[i * new_ncols + new_j] = ds.x[[i, orig_j]];
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
			data[i * new_ncols + new_j] = (offset + c) as f64;
		}
		offset += grp.len;
	}
	let embed_cols: Vec<usize> = (embed_start..embed_start + n_cat).collect();
	let x = crate::ok_or_die(Mat::from_shape_vec([n, new_ncols], data), "collapse_onehot");
	CollapsedOnehot {
		x,
		embed_cols,
		vocab: offset,
	}
}

enum FileFormat {
	Arff,
	Safetensors,
	Table,
}

fn file_format(path: &str) -> FileFormat {
	let ext = std::path::Path::new(path)
		.extension()
		.and_then(|e| e.to_str());
	let arff = ext.filter(|e| *e == "arff").and(Some(FileFormat::Arff));
	let safet = ext
		.filter(|e| *e == "safetensors")
		.and(Some(FileFormat::Safetensors));
	arff.or(safet).unwrap_or(FileFormat::Table)
}

pub fn safetensors_to_table(path: &str) -> anyhow::Result<SafeTable> {
	let bytes =
		std::fs::read(path).map_err(|e| anyhow::anyhow!("safetensors: read {path}: {e}"))?;
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
				std::cmp::Ordering::Equal => name.clone(),
				std::cmp::Ordering::Less | std::cmp::Ordering::Greater => {
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
		match file_format(path) {
			FileFormat::Arff => {
				let table = crate::data::parse_arff(path);
				self.inner.attrs = table.attrs;
				self.inner.rows = table.rows;
			}
			FileFormat::Safetensors => match safetensors_to_table(path) {
				Ok(table) => {
					self.inner.attrs = table.attrs;
					self.inner.rows = table.rows;
				}
				Err(e) => self.inner.defer(e),
			},
			FileFormat::Table => match pantry::detect_kinds(path) {
				Ok(kinds) => self.inner.pre_kinds.extend(kinds),
				Err(e) => self.inner.defer(e),
			},
		}
		self
	}

	pub fn target(mut self, t: impl IntoTargets) -> Data {
		self.inner.target_names = t.into_targets();
		self.inner.target = self.inner.target_names.first().cloned().unwrap_or_default();
		let Some(tp) = self.inner.test_path.clone() else {
			return self;
		};
		let Ok(raw) = crate::data::read_raw_csv(std::path::Path::new(&tp)) else {
			return self;
		};
		match raw.headers.len().checked_sub(1) {
			None => return self,
			Some(_last) => {
				self.inner.raw_test_headers = Some(raw
					.headers
					.into_iter()
					.map(|h| h.trim().to_string())
					.collect());
				self.inner.raw_test_rows = Some(raw.rows);
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
		self.deferred.get_or_insert(e);
	}

	pub fn datasets(&self) -> Datasets {
		crate::ok_or_die(self.try_datasets(), "Data::datasets")
	}

	pub(crate) fn try_datasets(&self) -> anyhow::Result<Datasets> {
		let prepared = self.prepare()?;
		self.print_summary(&prepared.train, prepared.test.as_ref(), &prepared.attrs);
		Ok(Datasets {
			train: prepared.train,
			test: prepared.test,
		})
	}

	fn feature_type_counts(&self, attrs: &[Attr]) -> Vec<TypeCount> {
		let is_target = |name: &str| self.target_names.iter().any(|t| t == name);
		let is_excluded = |name: &str| self.exclude.iter().any(|p| exclude_match(p, name));
		let mut numeric = 0usize;
		let mut temporal = 0usize;
		let mut categorical = 0usize;
		let mut ordinal = 0usize;
		let mut text = 0usize;
		let mut image = 0usize;
		for a in attrs
			.iter()
			.filter(|a| !is_target(a.name.as_str()) && !is_excluded(a.name.as_str()))
		{
			match &a.kind {
				Kind::Numeric => numeric += 1,
				Kind::Temporal => temporal += 1,
				Kind::Categorical(_cats) => categorical += 1,
				Kind::Ordinal(_ord) => ordinal += 1,
				Kind::Text(_txt) => text += 1,
				Kind::Image => image += 1,
			}
		}
		[
			TypeCount {
				label: "numeric",
				count: numeric,
			},
			TypeCount {
				label: "temporal",
				count: temporal,
			},
			TypeCount {
				label: "categorical",
				count: categorical,
			},
			TypeCount {
				label: "ordinal",
				count: ordinal,
			},
			TypeCount {
				label: "text",
				count: text,
			},
			TypeCount {
				label: "image",
				count: image,
			},
		]
		.into_iter()
		.filter(|cand| cand.count > 0)
		.collect()
	}

	fn cat_cardinality_counts(&self, attrs: &[Attr]) -> Vec<CardCount> {
		let is_target = |name: &str| self.target_names.iter().any(|t| t == name);
		let is_excluded = |name: &str| self.exclude.iter().any(|p| exclude_match(p, name));
		let mut card: std::collections::BTreeMap<usize, usize> =
			std::collections::BTreeMap::new();
		for a in attrs
			.iter()
			.filter(|a| !is_target(a.name.as_str()) && !is_excluded(a.name.as_str()))
		{
			let Kind::Categorical(cats) = &a.kind else {
				continue;
			};
			*card.entry(cats.len()).or_default() += 1;
		}
		let mut out = Vec::new();
		for card_key in card.keys() {
			out.push(CardCount {
				card: *card_key,
				count: card[card_key],
			});
		}
		out
	}

	fn print_summary(&self, train: &Dataset, test: Option<&Dataset>, attrs: &[Attr]) {
		let disk_size = |path: &str| -> String {
			std::fs::metadata(path)
				.map(|m| crate::data::human_bytes(m.len() as usize))
				.unwrap_or_else(|_err| "?".into())
		};
		let short = |path: &str| -> String {
			let Ok(home) = std::env::var("HOME") else {
				return path.to_string();
			};
			let Some(rest) = path.strip_prefix(&home) else {
				return path.to_string();
			};
			format!("~{rest}")
		};
		let raw_cols = attrs.len();
		let types = self.feature_type_counts(attrs);
		let print_types = |indent: &str| {
			for tc in &types {
				eprintln!("{indent}{} {}", tc.count, tc.label);
			}
		};
		let set_rows = match self.split_frac {
			Some(_frac) => train.x.nrows() + test.map_or(0, |t| t.x.nrows()),
			None => train.x.nrows(),
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
		for _present in cards.first().into_iter() {
			eprintln!("    encoding");
		}
		for cc in &cards {
			let range: Vec<String> = (0..cc.card).map(|i| i.to_string()).collect();
			eprintln!("        {} × [{}]", cc.count, range.join(", "));
		}
		eprintln!("    {} features -> model", train.x.ncols(),);
		for test in test.into_iter() {
			match &self.test_path {
				Some(tp) => {
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
				}
				None => {
					for _frac in self.split_frac.iter() {
						eprintln!(
							"\x1b[32msplit\x1b[0m  {} train / {} test",
							train.x.nrows(),
							test.x.nrows(),
						);
					}
				}
			}
		}
		for t in &self.target_names {
			eprintln!("\x1b[32mtarget\x1b[0m  {t}");
		}
	}

	fn prepare(&self) -> anyhow::Result<PreparedSets> {
		self.deferred
			.as_ref()
			.map(|e| anyhow::anyhow!("{e:#}"))
			.map_or(Ok(()), Err)?;
		let mut prepared = match self.attrs.first() {
			None => self.prepare_table()?,
			Some(_first) => {
				let arff = self.prepare_arff()?;
				PreparedSets {
					train: arff.train,
					test: arff.test,
					attrs: self.attrs.clone(),
				}
			}
		};
		pantry::encode::clean_dataset(&mut prepared.train);
		for ds in prepared.test.as_mut().into_iter() {
			pantry::encode::clean_dataset(ds);
		}
		anyhow::ensure!(
			prepared.train.x.nrows() > 0,
			"dataset has 0 rows after NaN removal"
		);
		anyhow::ensure!(
			prepared.train.x.ncols() > 0,
			"dataset has 0 feature columns"
		);
		let k = prepared.train.n_targets;
		anyhow::ensure!(
			prepared.train.y.len() == prepared.train.x.nrows() * k,
			"x/y dimension mismatch: {} rows × {k} targets but y has {} elements",
			prepared.train.x.nrows(),
			prepared.train.y.len(),
		);
		Ok(prepared)
	}

	fn prepare_arff(&self) -> anyhow::Result<Datasets> {
		let names: Vec<String> = self.attrs.iter().map(|a| a.name.clone()).collect();
		let resolved = self.resolve_targets(&names, None)?;
		let targets: Vec<usize> = resolved
			.iter()
			.map(|t| {
				names.iter()
					.position(|n| n == t)
					.ok_or_else(|| anyhow::anyhow!("resolved from names"))
			})
			.collect::<anyhow::Result<Vec<usize>>>()?;
		let prepared = pantry::encode::prepare_arff_data(
			&self.attrs,
			&self.rows,
			&targets,
			&self.exclude,
			self.split_frac,
			self.test_path.as_deref(),
			&self.source_label(),
		)?;
		Ok(Datasets {
			train: prepared.train,
			test: prepared.test,
		})
	}

	fn prepare_table(&self) -> anyhow::Result<PreparedSets> {
		let prepared = pantry::encode::prepare_table_data(
			&self.sources,
			self.test_path.as_deref(),
			self.split_frac,
			&self.exclude,
			&self.source_label(),
			Some(self.pre_kinds.as_slice()),
			|s, t| self.resolve_targets(s, t),
		)?;
		Ok(PreparedSets {
			train: prepared.train,
			test: prepared.test,
			attrs: prepared.attrs,
		})
	}

	fn resolve_targets(
		&self,
		set_names: &[String],
		test_names: Option<&[String]>,
	) -> anyhow::Result<Vec<String>> {
		match self.target_names.first() {
			Some(_first) => self
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
				.collect(),
			None => match test_names {
				Some(tn) => match set_names.len().cmp(&(tn.len() + 1)) {
					std::cmp::Ordering::Equal => Ok(vec![set_names
						.last()
						.ok_or_else(|| anyhow::anyhow!("set has columns"))?
						.clone()]),
					std::cmp::Ordering::Less | std::cmp::Ordering::Greater => {
						Ok(Vec::new())
					}
				},
				None => Ok(Vec::new()),
			},
		}
	}
}

impl crate::model::RunData for DataInner {
	fn prepared<'a>(&'a self) -> anyhow::Result<crate::model::Prepared<'a>> {
		let sets = self.try_datasets()?;
		Ok(crate::model::Prepared::Owned(sets.train))
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
	fn infer_only(&self) -> InferOnly {
		InferOnly::Fit
	}
}

impl crate::model::RunData for Data {
	fn prepared<'a>(&'a self) -> anyhow::Result<crate::model::Prepared<'a>> {
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
	fn infer_only(&self) -> InferOnly {
		self.inner.infer_only()
	}
}
