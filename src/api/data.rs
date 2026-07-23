use crate::Mat;
use ogdl::log::{Write, data};
use pantry::encode::exclude_match;
use pantry::{Attr, Kind};
use std::cell::{OnceCell, RefCell};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::mem;
use std::ops::Deref;

pub use pantry::encode::{Dataset, shuffle_split};

use crate::api::InferOnly;

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

thread_local! {
	static PARKED_DATA: RefCell<Option<Box<DataInner>>> =
		const { RefCell::new(None) };
}

impl Drop for Data {
	fn drop(&mut self) {
		let inner = mem::replace(&mut self.inner, Box::new(DataInner::blank()));
		PARKED_DATA.with(|slot| slot.borrow_mut().replace(inner));
	}
}

pub(crate) fn parked_data() -> Data {
	let inner = PARKED_DATA.with(|slot| slot.borrow_mut().take());
	let inner = inner.unwrap_or_else(|| {
		Write::error("run: no data configured — chain recipe.data(path) before run(data, …)");
		Box::new(DataInner::blank())
	});
	Data { inner }
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

struct RawTest {
	headers: Vec<String>,
	rows: Vec<Vec<String>>,
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
	pub(crate) parsed: Vec<pantry::formats::UsrData>,
	sources: Vec<String>,
	test_path: Option<String>,
	split_frac: Option<f64>,
	exclude: Vec<String>,
	test_raw: OnceCell<Option<RawTest>>,
	pre_kinds: pantry::encode::PreKinds,
	deferred: Option<anyhow::Error>,
}

impl DataInner {
	fn blank() -> DataInner {
		DataInner {
			target: String::new(),
			target_names: Vec::new(),
			parsed: Vec::new(),
			sources: Vec::new(),
			test_path: None,
			split_frac: None,
			exclude: Vec::new(),
			test_raw: OnceCell::new(),
			pre_kinds: Vec::new(),
			deferred: None,
		}
	}
}

impl Deref for Data {
	type Target = DataInner;
	fn deref(&self) -> &DataInner {
		&self.inner
	}
}
fn empty_dataset() -> Dataset {
	Dataset {
		x: Mat::zeros([0, 0]),
		y: crate::Vec1::zeros(0),
		source: String::new(),
		n_targets: 0,
		has_target: false,
		text_cols: Vec::new(),
		onehot_groups: Vec::new(),
	}
}

fn attrs_from_group(group: &pantry::formats::DataGroup) -> Vec<Attr> {
	use pantry::formats::DataType;
	return group
		.columns
		.iter()
		.map(|v| {
			let vals: &[String] = v.values.text().unwrap_or(&[]);
			let kind = match v.kind {
				DataType::Numeric => Kind::Numeric,
				DataType::Temporal => Kind::Temporal,
				DataType::Image => Kind::Image,
				DataType::Categoric => {
					let mut cats: Vec<String> = vals
						.iter()
						.filter(|s| !s.is_empty())
						.cloned()
						.collect::<std::collections::BTreeSet<_>>()
						.into_iter()
						.collect();
					cats.sort();
					Kind::Categorical(cats)
				}
				DataType::Ordinal => {
					let mut ord: Vec<String> = vals
						.iter()
						.filter(|s| !s.is_empty())
						.cloned()
						.collect::<std::collections::BTreeSet<_>>()
						.into_iter()
						.collect();
					ord.sort_by(|a, b| match (a.parse::<i64>(), b.parse::<i64>()) {
						(Ok(x), Ok(y)) => x.cmp(&y),
						_ => a.cmp(b),
					});
					Kind::Ordinal(ord)
				}
				DataType::Text => {
					let vocab: Vec<String> = vals
						.iter()
						.flat_map(|s| s.split(|c: char| !c.is_alphanumeric()))
						.filter(|t| !t.is_empty())
						.map(String::from)
						.collect::<std::collections::BTreeSet<_>>()
						.into_iter()
						.collect();
					Kind::Text(vocab)
				}
			};
			Attr {
				name: v.name.clone(),
				kind,
			}
		})
		.collect();
}

impl Data {
	pub fn load(path: &str) -> Data {
		let dat = Data {
			inner: Box::new(DataInner::blank()),
		};
		dat.set(path)
	}

	pub fn set(mut self, path: &str) -> Data {
		self.inner.sources.push(path.to_string());
		match pantry::formats::load(path) {
			Ok(mut usr) => {
				let typed = pantry::formats::assign_kinds(&mut usr, recipe_runtime::execute::detector_forward);
				match typed {
					Ok(()) => self.inner.parsed.push(usr),
					Err(e) => self.inner.defer(e),
				}
			}
			Err(e) => self.inner.defer(e),
		}
		self
	}

	pub fn target(mut self, t: impl IntoTargets) -> Data {
		self.inner.target_names = t.into_targets();
		self.inner.target = self.inner.target_names.first().cloned().unwrap_or_default();
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
		if !(0.0..1.0).contains(&train_frac) {
			Write::error(format!(
				"split fraction must be in (0, 1), got {train_frac}"
			));
			return self;
		}
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

	fn raw_test(&self) -> Option<&RawTest> {
		return self
			.test_raw
			.get_or_init(|| {
				let tp = self.test_path.as_ref()?;
				let usr = pantry::formats::load(tp).ok()?;
				let names: Vec<String> = self
					.parsed
					.iter()
					.flat_map(|u| u.groups.iter())
					.flat_map(|g| g.columns.iter())
					.map(|c| c.name.clone())
					.collect();
				let group = usr.related_group(&names).ok()?;
				let rows = group.text_rows().ok()?;
				let headers: Vec<String> = group
					.header_names()
					.iter()
					.map(|h| h.trim().to_string())
					.collect();
				if headers.is_empty() {
					return None;
				}
				return Some(RawTest { headers, rows });
			})
			.as_ref();
	}

	pub fn datasets(&self) -> Datasets {
		self.try_datasets().unwrap_or_else(|e| {
			Write::error(format!("Data::datasets: {e:#}"));
			Datasets {
				train: empty_dataset(),
				test: None,
			}
		})
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
		let mut card: BTreeMap<usize, usize> = BTreeMap::new();
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
			fs::metadata(path)
				.map(|m| crate::data::human_bytes(m.len() as usize))
				.unwrap_or_else(|_err| "?".into())
		};
		let short = |path: &str| -> String {
			let Ok(home) = env::var("HOME") else {
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
				Write::line(data, &format!("{indent}{} {}", tc.count, tc.label));
			}
		};
		let set_rows = match self.split_frac {
			Some(_frac) => train.x.nrows() + test.map_or(0, |t| t.x.nrows()),
			None => train.x.nrows(),
		};
		for src in &self.sources {
			Write::line(data, &format!("set  {}", short(src)));
			Write::line(data, &format!("    {}", disk_size(src)));
		}
		Write::line(data, &format!("    {} rows  {} cols", set_rows, raw_cols));
		print_types("        ");
		for ex in &self.exclude {
			Write::line(data, &format!("    excluded  {ex}"));
		}
		let cards = self.cat_cardinality_counts(attrs);
		for _present in cards.first().into_iter() {
			Write::line(data, "    encoding");
		}
		for cc in &cards {
			let range: Vec<String> = (0..cc.card).map(|i| i.to_string()).collect();
			Write::line(
				data,
				&format!("        {} × [{}]", cc.count, range.join(", ")),
			);
		}
		Write::line(data, &format!("    {} features -> model", train.x.ncols()));
		for test in test.into_iter() {
			match &self.test_path {
				Some(tp) => {
					let test_raw_cols = self.raw_test().map_or(raw_cols, |t| t.headers.len());
					let test_raw_rows = self.raw_test().map_or(test.x.nrows(), |t| t.rows.len());
					Write::line(data, &format!("test  {}", short(tp)));
					Write::line(
						data,
						&format!(
							"    {} rows  {} cols  {}",
							test_raw_rows,
							test_raw_cols,
							disk_size(tp),
						),
					);
					print_types("        ");
					Write::line(data, &format!("    {} features -> model", test.x.ncols()));
				}
				None => {
					for _frac in self.split_frac.iter() {
						Write::line(
							data,
							&format!("split  {} train / {} test", train.x.nrows(), test.x.nrows(),),
						);
					}
				}
			}
		}
		for t in &self.target_names {
			Write::line(data, &format!("target  {t}"));
		}
	}

	fn prepare(&self) -> anyhow::Result<PreparedSets> {
		self.deferred
			.as_ref()
			.map(|e| anyhow::anyhow!("{e:#}"))
			.map_or(Ok(()), Err)?;
		let mut groups_iter = self.parsed.iter().flat_map(|u| u.groups.iter());
		let first = groups_iter.next();
		let second = groups_iter.next();
		let mut prepared = match (first, second) {
			(None, _) => self.prepare_table()?,
			(Some(group), None) if group.columns.iter().all(|c| c.values.text().is_some()) => {
				let attrs = attrs_from_group(group);
				let rows = group.text_rows()?;
				let sets = self.prepare_encoded(&attrs, &rows)?;
				PreparedSets {
					train: sets.train,
					test: sets.test,
					attrs,
				}
			}
			_ => {
				let (set_groups, pre) = pantry::formats::to_dir_groups(&self.parsed)?;
				let test_groups = match self.test_path.as_deref() {
					Some(tp) => {
						let tusr = pantry::formats::load(tp)?;
						let (tg, _tpre) = pantry::formats::to_dir_groups(std::slice::from_ref(&tusr))?;
						Some((tg, tp.to_string()))
					}
					None => None,
				};
				let table = pantry::encode::prepare_groups_data(
					set_groups,
					test_groups,
					self.split_frac,
					&self.exclude,
					&self.source_label(),
					Some(pre.as_slice()),
					|s, t| self.resolve_targets(s, t),
				)?;
				PreparedSets {
					train: table.train,
					test: table.test,
					attrs: table.attrs,
				}
			}
		};
		pantry::encode::clean_dataset(&mut prepared.train)?;
		for ds in prepared.test.as_mut().into_iter() {
			pantry::encode::clean_dataset(ds)?;
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

	fn prepare_encoded(&self, attrs: &[Attr], rows: &[Vec<String>]) -> anyhow::Result<Datasets> {
		let names: Vec<String> = attrs.iter().map(|a| a.name.clone()).collect();
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
			attrs,
			rows,
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

	fn resolve_targets(&self, set_names: &[String], test_names: Option<&[String]>) -> anyhow::Result<Vec<String>> {
		match self.target_names.first() {
			Some(_first) => self
				.target_names
				.iter()
				.map(|want| {
					set_names
						.iter()
						.find(|n| {
							n.as_str() == want
								|| n.ends_with(&format!(":{want}")) || n.rsplit(':').next()
								== Some(want.as_str())
						})
						.cloned()
						.ok_or_else(|| {
							anyhow::anyhow!(
								"target '{want}' not found; available columns: {}",
								column_list(set_names)
							)
						})
				})
				.collect(),
			None => match test_names {
				Some(tn) => match set_names.len().cmp(&(tn.len() + 1)) {
					Ordering::Equal => Ok(vec![set_names
						.last()
						.ok_or_else(|| anyhow::anyhow!("set has columns"))?
						.clone()]),
					Ordering::Less | Ordering::Greater => Ok(Vec::new()),
				},
				None => Ok(Vec::new()),
			},
		}
	}
}

fn column_list(names: &[String]) -> String {
	let show = 20usize;
	let width = 48usize;
	let mut shown: Vec<String> = names
		.iter()
		.take(show)
		.map(|n| {
			let clean: String = n
				.chars()
				.map(|c| if c.is_control() { ' ' } else { c })
				.take(width)
				.collect();
			if n.chars().count() > width {
				return format!("{clean}...");
			}
			return clean;
		})
		.collect();
	if names.len() > show {
		shown.push(format!("... and {} more", names.len() - show));
	}
	return shown.join(", ");
}

impl crate::api::RunData for DataInner {
	fn prepared<'a>(&'a self) -> anyhow::Result<crate::api::Prepared<'a>> {
		let sets = self.try_datasets()?;
		Ok(crate::api::Prepared::Owned(sets.train))
	}
	fn target_names(&self) -> Vec<String> {
		self.target_names.clone()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		self.raw_test().map(|t| t.rows.clone())
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		self.raw_test().map(|t| t.headers.clone())
	}
	fn infer_only(&self) -> InferOnly {
		InferOnly::Fit
	}
}

impl crate::api::RunData for Data {
	fn prepared<'a>(&'a self) -> anyhow::Result<crate::api::Prepared<'a>> {
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
