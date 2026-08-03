use core::fmt; use std::collections::BTreeSet; use std::num::NonZeroUsize;
pub use recipe_core::{Activation, Block, DataNormalization, Function, LearningRateSchedule, Loss, Normalization, Optimizer,
	TreeFamily};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeclarationErrorKind { EmptyValue, InvalidExclusion, InvalidSplit, InvalidLayer, InvalidActivation,
	InvalidBayes, InvalidLearningRate, InvalidMetric, InvalidTrainingConfiguration, InvalidInferenceConfiguration, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationError { pub kind: DeclarationErrorKind, pub detail: String, }

impl DeclarationError { fn new(kind: DeclarationErrorKind, detail: impl Into<String>) -> Self { Self { kind,
			detail: detail.into(), } } }

impl fmt::Display for DeclarationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	} }

impl std::error::Error for DeclarationError {}

pub type DeclarationResult<T> = Result<T, DeclarationError>;

pub trait IntoTargets { fn into_targets(self) -> Vec<String>; }

impl IntoTargets for &str { fn into_targets(self) -> Vec<String> { vec![self.to_owned()] } }

impl IntoTargets for String { fn into_targets(self) -> Vec<String> { vec![self] } }

impl<const N: usize> IntoTargets for [&str; N] {
	fn into_targets(self) -> Vec<String> { self.into_iter().map(str::to_owned).collect() } }

impl IntoTargets for &[&str] {
	fn into_targets(self) -> Vec<String> { self.iter().map(|value| (*value).to_owned()).collect() } }

impl IntoTargets for Vec<String> { fn into_targets(self) -> Vec<String> { self } }

/// Literal value used by a row-exclusion predicate.
///
/// Floating-point values retain their exact declaration bits so immutable
/// declarations continue to support structural equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionValue { Signed(i64), Unsigned(u64), FloatBits(u64), Boolean(bool), Text(String), }

pub trait IntoConditionValue { fn into_condition_value(self) -> ConditionValue; }

macro_rules! impl_signed_condition_value { ($($integer:ty),+ $(,)?) => { $( impl IntoConditionValue for $integer {
				fn into_condition_value(self) -> ConditionValue { ConditionValue::Signed(i64::from(self)) } } )+ }; }

macro_rules! impl_unsigned_condition_value { ($($integer:ty),+ $(,)?) => { $( impl IntoConditionValue for $integer {
				fn into_condition_value(self) -> ConditionValue { ConditionValue::Unsigned(u64::from(self)) } } )+ }; }

impl_signed_condition_value!(i8, i16, i32, i64); impl_unsigned_condition_value!(u8, u16, u32, u64);

impl IntoConditionValue for isize {
	fn into_condition_value(self) -> ConditionValue { ConditionValue::Signed(self as i64) } }

impl IntoConditionValue for usize {
	fn into_condition_value(self) -> ConditionValue { ConditionValue::Unsigned(self as u64) } }

impl IntoConditionValue for f32 {
	fn into_condition_value(self) -> ConditionValue { ConditionValue::FloatBits((self as f64).to_bits()) } }

impl IntoConditionValue for f64 {
	fn into_condition_value(self) -> ConditionValue { ConditionValue::FloatBits(self.to_bits()) } }

impl IntoConditionValue for bool { fn into_condition_value(self) -> ConditionValue { ConditionValue::Boolean(self) } }

impl IntoConditionValue for &str {
	fn into_condition_value(self) -> ConditionValue { ConditionValue::Text(self.to_owned()) } }

impl IntoConditionValue for String { fn into_condition_value(self) -> ConditionValue { ConditionValue::Text(self) } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator { Equal, NotEqual, Less, LessOrEqual, Greater, GreaterOrEqual, }

/// Typed predicate selecting rows that must be excluded from the dataset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition { column: String, operator: ComparisonOperator, value: ConditionValue, }

impl Condition {
	#[must_use]
	pub(crate) fn new( column: impl Into<String>, operator: ComparisonOperator, value: impl IntoConditionValue, ) -> Self {
		Self { column: column.into(), operator, value: value.into_condition_value(), } }

	fn validate(&self) -> DeclarationResult<()> { if self.column.is_empty() { return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidExclusion,
				"condition column name is empty",
			)); }
		if matches!(self.value, ConditionValue::FloatBits(bits) if !f64::from_bits(bits).is_finite()) {
			return Err(DeclarationError::new( DeclarationErrorKind::InvalidExclusion,
				"condition comparison value must be finite",
			)); }
		Ok(()) }

	#[must_use]
	pub fn column(&self) -> &str { &self.column }

	#[must_use]
	pub const fn operator(&self) -> ComparisonOperator { self.operator }

	#[must_use]
	pub const fn value(&self) -> &ConditionValue { &self.value } }

/// Implementation detail for the exported [`cond!`] macro.
#[doc(hidden)]
#[must_use]
pub fn __condition( column: impl Into<String>, operator: ComparisonOperator, value: impl IntoConditionValue,
) -> Condition { Condition::new(column, operator, value) }

/// Construct a typed row-exclusion predicate without evaluating the column as
/// a Rust identifier.
#[macro_export]
macro_rules! cond { ($column:ident <= $value:expr) => { $crate::__condition( stringify!($column),
			$crate::ComparisonOperator::LessOrEqual, $value, ) }; ($column:ident >= $value:expr) => { $crate::__condition(
			stringify!($column), $crate::ComparisonOperator::GreaterOrEqual, $value, ) }; ($column:ident == $value:expr) => {
		$crate::__condition( stringify!($column), $crate::ComparisonOperator::Equal, $value, ) };
	($column:ident != $value:expr) => { $crate::__condition( stringify!($column), $crate::ComparisonOperator::NotEqual,
			$value, ) }; ($column:ident < $value:expr) => { $crate::__condition( stringify!($column),
			$crate::ComparisonOperator::Less, $value, ) }; ($column:ident > $value:expr) => { $crate::__condition(
			stringify!($column), $crate::ComparisonOperator::Greater, $value, ) }; }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exclusion { Column(String), Condition(Condition), }

pub trait IntoExclusions { fn into_exclusions(self) -> Vec<Exclusion>; }

impl IntoExclusions for &str { fn into_exclusions(self) -> Vec<Exclusion> { vec![Exclusion::Column(self.to_owned())] } }

impl IntoExclusions for String { fn into_exclusions(self) -> Vec<Exclusion> { vec![Exclusion::Column(self)] } }

impl<const N: usize> IntoExclusions for [&str; N] { fn into_exclusions(self) -> Vec<Exclusion> { self.into_iter()
			.map(|column| Exclusion::Column(column.to_owned())) .collect() } }

impl IntoExclusions for &[&str] { fn into_exclusions(self) -> Vec<Exclusion> { self.iter()
			.map(|column| Exclusion::Column((*column).to_owned())) .collect() } }

impl IntoExclusions for Vec<String> {
	fn into_exclusions(self) -> Vec<Exclusion> { self.into_iter().map(Exclusion::Column).collect() } }

impl IntoExclusions for Condition { fn into_exclusions(self) -> Vec<Exclusion> { vec![Exclusion::Condition(self)] } }

impl IntoExclusions for Exclusion { fn into_exclusions(self) -> Vec<Exclusion> { vec![self] } }

impl IntoExclusions for Vec<Exclusion> { fn into_exclusions(self) -> Vec<Exclusion> { self } }

#[allow(non_upper_case_globals)]
pub const z_score: DataNormalization = DataNormalization::ZScore;
#[allow(non_upper_case_globals)]
pub const min_max: DataNormalization = DataNormalization::MinMax;
#[allow(non_upper_case_globals)]
pub const l2_norm: DataNormalization = DataNormalization::L2Norm;

/// Immutable data-ingestion declaration.
///
/// Construction records paths and preprocessing policy only. Filesystem reads,
/// parsing, encoding, and the single per-device data-image admission occur in
/// the preparation and init lifecycle, never in these builder calls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Data { sources: Vec<String>, targets: Vec<String>, exclusions: Vec<String>,
	condition_exclusions: Vec<Condition>, split_fraction_bits: Option<u32>, normalization: Option<DataNormalization>,
	deferred: Option<DeclarationError>, }

impl Data { pub(crate) const fn empty() -> Self { Self { sources: Vec::new(), targets: Vec::new(),
			exclusions: Vec::new(), condition_exclusions: Vec::new(), split_fraction_bits: None, normalization: None,
			deferred: None, } }

	pub fn set(mut self, path: &str) -> Self { if path.is_empty() { self.defer( DeclarationErrorKind::EmptyValue,
				"data source path is empty",
			); } else { self.sources.push(path.to_owned()); }
		crate::remember_recipe_data(self.clone()); self }

	pub fn target(mut self, targets: impl IntoTargets) -> Self { let targets = targets.into_targets();
		if targets.is_empty() || targets.iter().any(String::is_empty) { self.defer( DeclarationErrorKind::EmptyValue,
				"target names must be nonempty",
			); } else { self.targets = targets; }
		crate::remember_recipe_data(self.clone()); self }

	pub fn exclude(mut self, exclusions: impl IntoExclusions) -> Self { let exclusions = exclusions.into_exclusions();
		if exclusions.is_empty() { self.defer( DeclarationErrorKind::EmptyValue,
				"at least one column or condition exclusion is required",
			); }
		for exclusion in exclusions { match exclusion { Exclusion::Column(pattern) if pattern.is_empty() => self.defer(
					DeclarationErrorKind::EmptyValue,
					"excluded-column pattern is empty",
				), Exclusion::Column(pattern) => self.exclusions.push(pattern),
				Exclusion::Condition(condition) => match condition.validate() { Ok(()) => self.condition_exclusions.push(condition),
					Err(error) => self.defer(error.kind, error.detail), }, } }
		crate::remember_recipe_data(self.clone()); self }

	pub fn split(mut self, train_fraction: f64) -> Self { let narrowed = train_fraction as f32;
		if !train_fraction.is_finite() || train_fraction <= 0.0 || train_fraction >= 1.0 || narrowed <= 0.0 || narrowed >= 1.0
		{ self.defer( DeclarationErrorKind::InvalidSplit,
				format!("split fraction must be finite and in (0, 1), got {train_fraction}"),
			); } else { self.split_fraction_bits = Some(narrowed.to_bits()); }
		crate::remember_recipe_data(self.clone()); self }

	pub fn norm(mut self, normalization: DataNormalization) -> Self { self.normalization = Some(normalization);
		crate::remember_recipe_data(self.clone()); self }

	pub(crate) fn validate(&self) -> DeclarationResult<()> { if let Some(error) = &self.deferred {
			return Err(error.clone()); }
		if self.sources.is_empty() { return Err(DeclarationError::new( DeclarationErrorKind::EmptyValue,
				"at least one data source is required",
			)); }
		Ok(()) }

	#[must_use]
	pub fn source(&self) -> &str { self.sources.first().map_or("", String::as_str) }

	#[must_use]
	pub fn sources(&self) -> &[String] { &self.sources }

	#[must_use]
	pub fn targets(&self) -> &[String] { &self.targets }

	#[must_use]
	pub fn exclusions(&self) -> &[String] { &self.exclusions }

	#[must_use]
	pub fn condition_exclusions(&self) -> &[Condition] { &self.condition_exclusions }

	#[must_use]
	pub fn split_fraction(&self) -> Option<f32> { self.split_fraction_bits.map(f32::from_bits) }

	#[must_use]
	pub const fn normalization(&self) -> Option<DataNormalization> { self.normalization }

	fn defer(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) { if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail)); } } }

#[allow(non_upper_case_globals)]
pub const layer_norm: Function = Function::Normalization(Normalization::Layer);
#[allow(non_upper_case_globals)]
pub const batch_norm: Function = Function::Normalization(Normalization::Batch);

#[must_use]
pub const fn layer(width: usize) -> Block { Block::layer(width) }

#[must_use]
pub const fn conv(filters: usize, kernel: usize) -> Block { Block::convolution(filters, kernel) }

pub trait IntoLayer { fn into_layer(self) -> Block; }

impl IntoLayer for usize { fn into_layer(self) -> Block { Block::layer(self) } }

impl IntoLayer for Block { fn into_layer(self) -> Block { self } }

fn validate_block(block: &Block) -> DeclarationResult<()> { let valid = match block {
		Block::Layer { width, .. } | Block::Rnn { width, .. } | Block::Gru { width, .. } | Block::Lstm { width, .. } => {
			*width != 0 }
		Block::Convolution { filters, kernel, .. } => *filters != 0 && *kernel != 0,
		Block::Pool { size, group_to_neuron } => *size != 0 && group_to_neuron.is_none_or(|width| width != 0),
		Block::KMeans { clusters, group_to_neuron } => *clusters != 0 && group_to_neuron.is_none_or(|width| width != 0),
		Block::Embedding { dimensions, vocabulary } => *dimensions != 0 && vocabulary.is_none_or(|value| value != 0),
		Block::Attention { heads } => *heads != 0,
		Block::Tree { trees, depth, .. } => *trees != 0 && *depth != 0,
		Block::Knn { neighbors, .. } => *neighbors != 0,
		Block::Residual { branch, .. } => !branch.is_empty() && branch.iter().all(|block| validate_block(block).is_ok()),
	}; if valid { Ok(()) } else { Err(DeclarationError::new( DeclarationErrorKind::InvalidLayer,
			"model blocks must be complete and nonzero", )) } }

#[allow(non_upper_case_globals)]
/// Mean squared error: `(z - y)²`.
pub const mse: Loss = Loss::MeanSquaredError;
#[allow(non_upper_case_globals)]
/// Mean absolute error: `abs(z - y)`.
pub const mae: Loss = Loss::MeanAbsoluteError;
#[allow(non_upper_case_globals)]
/// Unit-delta Huber loss: `(z - y)² / 2` below unit absolute
/// error and `abs(z - y) - 1/2` otherwise.
pub const huber: Loss = Loss::Huber;
#[allow(non_upper_case_globals)]
/// Binary cross entropy evaluated by the training compiler from logits.
pub const bce: Loss = Loss::BinaryCrossEntropy;
#[allow(non_upper_case_globals)]
/// Multiclass cross entropy evaluated from logits using the target class and
/// a numerically stable log-softmax. The compiler derives the class width from
/// the categorical target contract.
pub const ce: Loss = Loss::CrossEntropy;
#[allow(non_upper_case_globals)]
/// Binary focal loss evaluated from logits with Recipe's fixed historical
/// `alpha = 0.25` and `gamma = 2.0`. Targets must be exact zero or one.
pub const focal: Loss = Loss::Focal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grad { clip_bits: Option<u32>, invalid_clip_bits: Option<u64>, }

/// Configure an explicit global gradient clipping norm for [`Model::grad`].
#[must_use]
pub fn clip(maximum_norm: f64) -> Grad { let narrowed = maximum_norm as f32;
	if maximum_norm.is_finite() && narrowed.is_finite() && narrowed > 0.0 { Grad { clip_bits: Some(narrowed.to_bits()),
			invalid_clip_bits: None, } } else { Grad { clip_bits: None, invalid_clip_bits: Some(maximum_norm.to_bits()), } } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Objective { Builtin(Loss), Reference(Box<Model>), }

pub trait IntoObjective { fn into_objective(self) -> Objective; }

impl IntoObjective for Loss { fn into_objective(self) -> Objective { Objective::Builtin(self) } }

impl IntoObjective for &Model { fn into_objective(self) -> Objective { Objective::Reference(Box::new(self.clone())) } }

#[allow(non_upper_case_globals)]
pub const adamw: Optimizer = Optimizer::AdamW;

/// One conditional dependency in a Bayesian model.
///
/// The edge direction is `parent -> child`; declaration order is retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesDependency { child: String, parents: Vec<String>, }

impl BayesDependency {
	#[must_use]
	pub fn child(&self) -> &str { &self.child }

	#[must_use]
	pub fn parents(&self) -> &[String] { &self.parents }

	fn validate(&self) -> DeclarationResult<()> { if self.child.is_empty() { return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes,
				"Bayesian child name is empty",
			)); }
		if self.parents.iter().any(String::is_empty) { return Err(DeclarationError::new( DeclarationErrorKind::InvalidBayes,
				format!(
					"Bayesian dependency for {:?} contains an empty parent name",
					self.child ), )); }
		if self.parents.iter().any(|parent| parent == &self.child) { return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes, format!(
					"Bayesian dependency {:?} cannot name itself as a parent",
					self.child ), )); }
		let unique = self.parents.iter().collect::<BTreeSet<_>>(); if unique.len() != self.parents.len() {
			return Err(DeclarationError::new( DeclarationErrorKind::InvalidBayes, format!(
					"Bayesian dependency {:?} contains a duplicate parent",
					self.child ), )); }
		Ok(()) } }

const CHECKPOINT_MODEL_DECLARATION_CONFLICT: &str =
	"a checkpoint-backed model cannot also declare layers, Bayesian dependencies, a loss, or gradient policy";

/// Backend-neutral model declaration. It contains no runtime handles, loaded
/// weights, allocations, or mutable global registry entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model { layers: Vec<Block>, bayes_dependencies: Vec<BayesDependency>, objective: Option<Objective>,
	gradient_clip_bits: Option<u32>, weights_source: Option<String>, deferred: Option<DeclarationError>, }

impl Model {
	#[must_use]
	pub(crate) const fn new() -> Self { Self { layers: Vec::new(), bayes_dependencies: Vec::new(), objective: None,
			gradient_clip_bits: None, weights_source: None, deferred: None, } }

	/// Declare a checkpoint-backed model.
	///
	/// Input width, feature schema, topology, and parameter shapes are read from
	/// the checkpoint during inference preparation; callers do not redeclare
	/// them here.
	pub fn load(mut self, path: &str) -> Self { if path.is_empty() { self.defer( DeclarationErrorKind::EmptyValue,
				"model checkpoint path is empty",
			); } else if self.has_inline_definition() { self.defer( DeclarationErrorKind::InvalidLayer,
				CHECKPOINT_MODEL_DECLARATION_CONFLICT, ); } else if self.weights_source.is_some() { self.defer(
				DeclarationErrorKind::InvalidLayer,
				"a model accepts exactly one checkpoint source",
			); } else { self.weights_source = Some(path.to_owned()); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn layer(mut self, spec: impl IntoLayer) -> Self { let spec = spec.into_layer();
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else {
			if let Block::Layer { width, .. } = &spec { match self.layers.last_mut() { Some(Block::Pool {
						group_to_neuron, .. }) | Some(Block::KMeans { group_to_neuron, .. }) => {
						*group_to_neuron = Some(*width); }
					_ => {} } }
			self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn embed(mut self, dimensions: usize) -> Self { let spec = Block::Embedding { dimensions, vocabulary: None, };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn vocab(mut self, vocabulary: usize) -> Self { if vocabulary == 0 { self.defer(
				DeclarationErrorKind::InvalidLayer,
				"embedding vocabulary must be nonzero",
			); } else { match self.layers.last_mut() { Some(Block::Embedding { vocabulary: current, ..
				}) => *current = Some(vocabulary), _ => self.defer( DeclarationErrorKind::InvalidLayer,
					"vocab requires a preceding embedding block",
				), } }
		crate::remember_recipe_model(self.clone()); self }

	pub fn attn(mut self, heads: usize) -> Self { let spec = Block::Attention { heads };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	/// Add one block containing `count` parallel perceptrons.
	pub fn perc(mut self, count: usize) -> Self { let spec = Block::Layer { width: count, functions: Vec::new(), };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	/// Add a recurrent neural-network block with `width` recurrent states.
	pub fn rnn(mut self, width: usize) -> Self { let spec = Block::Rnn { width, functions: Vec::new(), };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	/// Add a gated recurrent-unit block with `width` recurrent states.
	pub fn gru(mut self, width: usize) -> Self { let spec = Block::Gru { width, functions: Vec::new(), };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	/// Add a long short-term-memory block with `width` recurrent states.
	pub fn lstm(mut self, width: usize) -> Self { let spec = Block::Lstm { width, functions: Vec::new(), };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	/// Declare that `child` is conditionally modeled from `parents`.
	///
	/// Repeated calls append to one network in source order. Each child has one
	/// declaration, while a parent may feed any number of children. Parent nodes
	/// may be declared later or remain implicit roots. The resulting network
	/// must be acyclic.
	///
	/// The current executable repeated-call instrument binds every child to the
	/// corresponding declared data target and requires every parent to remain an
	/// observed categorical inference feature. It does not reinterpret a target
	/// child as sampled or marginalized evidence for another conditional.
	pub fn bayes(mut self, child: &str, parents: impl IntoTargets) -> Self { let dependency = BayesDependency {
			child: child.to_owned(), parents: parents.into_targets(), }; self.bayes_dependencies.push(dependency);
		if let Err(error) = validate_bayes_network(&self.bayes_dependencies) { self.bayes_dependencies.pop();
			self.defer(error.kind, error.detail); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn conv(mut self, filters: usize, kernel: usize) -> Self { let spec = Block::Convolution { filters, kernel,
			functions: Vec::new(), }; if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail);
		} else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn pool(mut self, size: usize) -> Self { let spec = Block::Pool { size, group_to_neuron: None, };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn lgbm(self, depth: usize) -> Self { self.with_tree_family(TreeFamily::LightGbm, depth) }

	pub fn cbst(self, depth: usize) -> Self { self.with_tree_family(TreeFamily::CatBoost, depth) }

	pub fn xgbst(self, depth: usize) -> Self { self.with_tree_family(TreeFamily::XGBoost, depth) }

	pub fn forest(mut self, trees: usize) -> Self { if trees == 0 { self.defer( DeclarationErrorKind::InvalidLayer,
				"forest tree count must be nonzero",
			); } else { self.layers.push(Block::Tree { family: TreeFamily::LightGbm, trees, depth: 0, }); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn kmeans(mut self, clusters: usize) -> Self { let spec = Block::KMeans { clusters, group_to_neuron: None, };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn knn(mut self, neighbors: usize) -> Self { let spec = Block::Knn { neighbors, functions: Vec::new(), };
		if let Err(error) = validate_block(&spec) { self.defer(error.kind, error.detail); } else { self.layers.push(spec); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn residual(mut self, branch: impl IntoIterator<Item = Block>) -> Self { let branch = branch.into_iter().collect();
		let spec = Block::Residual { branch, functions: Vec::new(), }; if validate_block(&spec).is_ok() { self.layers.push(spec);
		} else { self.defer(
				DeclarationErrorKind::InvalidLayer,
				"a residual branch requires at least one layer and every declared layer width must be nonzero",
			); }
		crate::remember_recipe_model(self.clone()); self }

	pub fn relu(self) -> Self { self.with_last_activation(Activation::Relu) }

	pub fn leak(self) -> Self { self.with_last_activation(Activation::LeakyRelu) }

	pub fn sigmoid(self) -> Self { self.with_last_activation(Activation::Sigmoid) }

	pub fn tanh(self) -> Self { self.with_last_activation(Activation::Tanh) }

	pub fn selu(self) -> Self { self.with_last_activation(Activation::Selu) }

	pub fn gelu(self) -> Self { self.with_last_activation(Activation::Gelu) }

	pub fn silu(self) -> Self { self.with_last_activation(Activation::Silu) }

	pub fn elu(self) -> Self { self.with_last_activation(Activation::Elu) }

	/// Appends a parametric ReLU with one independently learned scalar slope.
	/// Every ordered occurrence starts at `0.25` and owns distinct optimizer and
	/// checkpoint state.
	pub fn prelu(self) -> Self { self.with_last_activation(Activation::PRelu) }

	pub fn cos(self) -> Self { self.with_last_activation(Activation::Cosine) }

	pub fn exp(self) -> Self { self.with_last_activation(Activation::Exponential) }

	/// Applies `sign(x) * ln(abs(x))` to each finite, nonzero value.
	/// Positive and negative zero are rejected through Recipe's device fault
	/// boundary.
	pub fn log(self) -> Self { self.with_last_activation(Activation::Logarithm) }

	/// Applies the ordinary natural logarithm to each finite, strictly positive
	/// value. Zero and negative values are rejected through Recipe's device fault
	/// boundary.
	pub fn ln(self) -> Self { self.with_last_activation(Activation::NaturalLogarithm) }

	pub fn huber(self) -> Self { self.with_last_activation(Activation::Huber) }

	pub fn tan(self) -> Self { self.with_last_activation(Activation::Tangent) }

	pub fn loss(mut self, objective: impl IntoObjective) -> Self { self.objective = Some(objective.into_objective());
		crate::remember_recipe_model(self.clone()); self }

	pub fn grad(mut self, gradient: Grad) -> Self { match gradient.clip_bits {
			Some(bits) => self.gradient_clip_bits = Some(bits), None => { let maximum_norm = gradient .invalid_clip_bits
					.map(f64::from_bits) .unwrap_or(f64::NAN); self.defer( DeclarationErrorKind::InvalidTrainingConfiguration, format!(
						"gradient clipping norm must be finite, positive, and representable as f32, got {maximum_norm}"
					), ); } }
		crate::remember_recipe_model(self.clone()); self }

	pub fn norm(mut self, normalization: Function) -> Self { match self.layers.last_mut() { Some(
				Block::Layer { functions, .. }
				| Block::Rnn { functions, .. }
				| Block::Gru { functions, .. }
				| Block::Lstm { functions, .. }
				| Block::Residual { functions, .. }, ) => { functions.push(normalization); }
			Some(Block::Knn { .. }) => self.defer( DeclarationErrorKind::InvalidLayer,
				"normalization cannot follow terminal all-output KNN reduction",
			), Some( Block::Convolution { .. } | Block::Pool { .. }
				| Block::Tree { .. }
				| Block::KMeans { .. }
				| Block::Embedding { .. }
				| Block::Attention { .. }, )
			| None => self.defer( DeclarationErrorKind::InvalidLayer,
				"layer normalization requires a preceding dense, perceptron, recurrent, or residual block",
			), }
		crate::remember_recipe_model(self.clone()); self }

	pub(crate) fn validate(&self) -> DeclarationResult<()> { if let Some(error) = &self.deferred {
			return Err(error.clone()); }
		if self.weights_source.is_some() && self.has_inline_definition() { return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidLayer, CHECKPOINT_MODEL_DECLARATION_CONFLICT, )); }
		if self.layers.is_empty() && self.bayes_dependencies.is_empty() && self.weights_source.is_none() {
			return Err(DeclarationError::new( DeclarationErrorKind::InvalidLayer,
				"a model requires at least one layer, Bayesian dependency, or declared weight source",
			)); }
		if let Some((index, Block::Knn { functions, .. })) = self .layers .iter() .enumerate()
			.find(|(_, layer)| matches!(layer, Block::Knn { .. }))
		{ if self.layers.len() != 1 || index != 0 { return Err(DeclarationError::new( DeclarationErrorKind::InvalidLayer,
					"`.knn(neighbors)` is one standalone terminal all-output model and cannot compose with other blocks",
				)); }
			if !functions.is_empty() { return Err(DeclarationError::new( DeclarationErrorKind::InvalidLayer,
					"terminal all-output KNN reduction cannot contain activation or normalization operations",
				)); } }
		for (index, layer) in self.layers.iter().enumerate() { validate_block(layer)?; let connection = match layer {
				Block::Pool { group_to_neuron, .. }
				| Block::KMeans { group_to_neuron, .. } => group_to_neuron.as_ref(), _ => None, };
			if let Some(connection) = connection { let has_exact_following_layer = matches!( self.layers.get(index + 1),
					Some(Block::Layer { width, .. }) if width == connection ); if !has_exact_following_layer {
					return Err(DeclarationError::new( DeclarationErrorKind::InvalidLayer,
						"a grouped-to-dense connection must refer to the immediately following layer",
					)); } } }
		validate_bayes_network(&self.bayes_dependencies)?; if let Some(Objective::Reference(model)) = &self.objective {
			model.validate()?; }
		Ok(()) }

	#[must_use]
	pub fn layers(&self) -> &[Block] { &self.layers }

	#[must_use]
	pub fn bayes_dependencies(&self) -> &[BayesDependency] { &self.bayes_dependencies }

	#[must_use]
	pub const fn objective(&self) -> Option<&Objective> { self.objective.as_ref() }

	#[must_use]
	pub fn gradient_clip_value(&self) -> Option<f32> { self.gradient_clip_bits.map(f32::from_bits) }

	#[must_use]
	pub fn weights_source(&self) -> Option<&str> { self.weights_source.as_deref() }

	fn has_inline_definition(&self) -> bool { !self.layers.is_empty() || !self.bayes_dependencies.is_empty()
			|| self.objective.is_some()
			|| self.gradient_clip_bits.is_some() }

	fn with_tree_family(mut self, family: TreeFamily, depth: usize) -> Self { if depth == 0 {
			self.defer( DeclarationErrorKind::InvalidLayer,
				"boosted-tree depth must be nonzero",
			); } else if let Some(Block::Tree { family: pending_family, depth: pending_depth @ 0, .. }) = self.layers.last_mut()
		{ *pending_family = family; *pending_depth = depth; } else {
			self.layers.push(Block::Tree { family, trees: 1, depth }); }
		crate::remember_recipe_model(self.clone()); self }

	fn with_last_activation(mut self, activation: Activation) -> Self { match self.layers.last_mut() { Some(
				Block::Layer { functions, .. }
				| Block::Rnn { functions, .. }
				| Block::Gru { functions, .. }
				| Block::Lstm { functions, .. }
				| Block::Residual { functions, .. }, ) => { functions.push(Function::Activation(activation)); }
			Some(Block::Knn { .. }) => self.defer( DeclarationErrorKind::InvalidActivation,
				"activation cannot follow terminal all-output KNN reduction",
			), Some(Block::Convolution { functions, .. }) => functions.push(Function::Activation(activation)), Some( Block::Pool { .. }
				| Block::Tree { .. }
				| Block::KMeans { .. }
				| Block::Embedding { .. }
				| Block::Attention { .. }, )
			| None => self.defer( DeclarationErrorKind::InvalidActivation, concat!(
					"activation methods require a preceding dense, perceptron, recurrent, convolution,",
					" or residual block",
				), ), }
		crate::remember_recipe_model(self.clone()); self }

	fn defer(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) { if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail)); } } }

fn validate_bayes_network(dependencies: &[BayesDependency]) -> DeclarationResult<()> {
	let mut children = BTreeSet::new(); for dependency in dependencies { dependency.validate()?;
		if !children.insert(dependency.child.as_str()) { return Err(DeclarationError::new( DeclarationErrorKind::InvalidBayes,
				format!(
					"Bayesian child {:?} is declared more than once",
					dependency.child ), )); } }
	if bayes_dependencies_have_cycle(dependencies) { return Err(DeclarationError::new( DeclarationErrorKind::InvalidBayes,
			"Bayesian dependencies contain a cycle",
		)); }
	Ok(()) }

fn bayes_dependencies_have_cycle(dependencies: &[BayesDependency]) -> bool { dependencies.iter().any(|dependency| {
		dependency .parents .iter() .any(|parent| bayes_path_exists(dependencies, &dependency.child, parent)) }) }

fn bayes_path_exists(dependencies: &[BayesDependency], from: &str, to: &str) -> bool { let mut pending = vec![from];
	let mut visited = BTreeSet::new(); while let Some(current) = pending.pop() { if current == to { return true; }
		if !visited.insert(current) { continue; }
		for dependency in dependencies { if dependency.parents.iter().any(|parent| parent == current) {
				pending.push(dependency.child.as_str()); } } }
	false }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Metric { Loss, Accuracy, R2, AuRoc, AuPrc, Brier, CalibrationError, Epoch, LearningRate, Time, Device, }

impl Metric { fn validate(self) -> DeclarationResult<()> { Ok(()) }

	const fn inference_rejection(self) -> Option<&'static str> {
		match self { Self::Time | Self::Device => None, Self::Loss | Self::Accuracy | Self::R2 | Self::AuRoc | Self::AuPrc
			| Self::Brier
			| Self::CalibrationError => Some("target-free inference has no target values for this metric"),
			Self::Epoch | Self::LearningRate => {
				Some("inference has no training epoch or optimizer state for this metric")
			} } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogItem { metric: Metric, }

impl LogItem { fn validate(self) -> DeclarationResult<()> { self.metric.validate() }

	#[must_use]
	pub(crate) const fn metric(self) -> Metric { self.metric } }

pub trait IntoLogItems { fn into_log_items(self) -> Vec<LogItem>; }

impl IntoLogItems for LogItem { fn into_log_items(self) -> Vec<LogItem> { vec![self] } }

impl<const N: usize> IntoLogItems for [LogItem; N] { fn into_log_items(self) -> Vec<LogItem> { self.into() } }

impl IntoLogItems for Vec<LogItem> { fn into_log_items(self) -> Vec<LogItem> { self } }

impl IntoLogItems for &[LogItem] { fn into_log_items(self) -> Vec<LogItem> { self.to_vec() } }

#[allow(non_upper_case_globals)]
pub const Accuracy: LogItem = LogItem { metric: Metric::Accuracy, };
#[allow(non_upper_case_globals)]
pub const R2: LogItem = LogItem { metric: Metric::R2 };
#[allow(non_upper_case_globals)]
pub const AuRoc: LogItem = LogItem { metric: Metric::AuRoc, };
#[allow(non_upper_case_globals)]
pub const AuPrc: LogItem = LogItem { metric: Metric::AuPrc, };
#[allow(non_upper_case_globals)]
pub const Brier: LogItem = LogItem { metric: Metric::Brier, };
#[allow(non_upper_case_globals)]
pub const CalibrationError: LogItem = LogItem { metric: Metric::CalibrationError, };
#[allow(non_upper_case_globals)]
pub const Epoch: LogItem = LogItem { metric: Metric::Epoch, };
#[allow(non_upper_case_globals)]
pub const Lr: LogItem = LogItem { metric: Metric::LearningRate, };
#[allow(non_upper_case_globals)]
pub const Time: LogItem = LogItem { metric: Metric::Time, };
#[allow(non_upper_case_globals)]
pub const Device: LogItem = LogItem { metric: Metric::Device, };
#[allow(non_upper_case_globals)]
pub const LossMetric: LogItem = LogItem { metric: Metric::Loss, };
#[allow(non_upper_case_globals)]
pub const Loss: LogItem = LossMetric;
#[allow(non_upper_case_globals)]
pub const Acc: LogItem = Accuracy;
#[allow(non_upper_case_globals)]
pub const loss: LogItem = LossMetric;
#[allow(non_upper_case_globals)]
pub const accuracy: LogItem = Accuracy;
#[allow(non_upper_case_globals)]
pub const epoch: LogItem = Epoch;
#[allow(non_upper_case_globals)]
pub const lr: LogItem = Lr;
#[allow(non_upper_case_globals)]
pub const time: LogItem = Time;
#[allow(non_upper_case_globals)]
pub const r2: LogItem = R2;
#[allow(non_upper_case_globals)]
pub const device: LogItem = Device;

#[derive(Clone, Debug, PartialEq, Eq)]
struct LogDeclaration { items: Vec<LogItem>, every: NonZeroUsize, }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TrainingArtifactDeclaration { model: Option<String>, kernel: Option<String>, }

/// Static training policy. Calling builder methods does not probe hardware,
/// parse data, prepare a bundle, or start a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Train { epochs: Option<usize>, learning_rate_bits: Option<u32>, warmup_epochs: Option<usize>,
	learning_rate_schedule: Option<LearningRateSchedule>, optimizer: Option<Optimizer>, log: Vec<LogItem>,
	log_declarations: Vec<LogDeclaration>, plot: Vec<LogItem>, resume: TrainingArtifactDeclaration, resume_declared: bool,
	save: TrainingArtifactDeclaration, save_declared: bool, deferred: Option<DeclarationError>, }

impl Train {
	#[must_use]
	pub(crate) const fn new() -> Self { Self { epochs: None, learning_rate_bits: None, warmup_epochs: None,
			learning_rate_schedule: None, optimizer: None, log: Vec::new(), log_declarations: Vec::new(), plot: Vec::new(),
			resume: TrainingArtifactDeclaration { model: None, kernel: None, }, resume_declared: false,
			save: TrainingArtifactDeclaration { model: None, kernel: None, }, save_declared: false, deferred: None, } }

	#[must_use]
	pub fn epochs(mut self, epochs: usize) -> Self { if epochs == 0 {
			self.defer("training epoch bound must be nonzero");
		} else { self.epochs = Some(epochs); }
		self }

	#[must_use]
	pub fn lr(mut self, rate: f64) -> Self { let rate_f32 = rate as f32;
		if !rate.is_finite() || !rate_f32.is_finite() || rate_f32 <= 0.0 { self.defer_kind(
				DeclarationErrorKind::InvalidLearningRate, format!(
					"training learning rate must be finite, positive, and representable as f32, got {rate}"
				), ); } else { self.learning_rate_bits = Some(rate_f32.to_bits());
			self.learning_rate_schedule = Some(LearningRateSchedule::Linear); }
		self }

	#[must_use]
	pub fn warmup(mut self, epochs: usize) -> Self { if epochs == 0 {
			self.defer("training warmup epoch bound must be nonzero");
		} else { self.warmup_epochs = Some(epochs); }
		self }

	#[must_use]
	pub const fn cos(mut self) -> Self { self.learning_rate_schedule = Some(LearningRateSchedule::Cosine); self }

	#[must_use]
	pub const fn exp(mut self) -> Self { self.learning_rate_schedule = Some(LearningRateSchedule::Exponential); self }

	#[must_use]
	pub const fn optimizer(mut self, optimizer: Optimizer) -> Self { self.optimizer = Some(optimizer); self }

	#[must_use]
	pub fn every(mut self, interval: usize) -> Self { let Some(every) = NonZeroUsize::new(interval) else {
			self.defer("training log interval must be nonzero");
			return self; }; let Some(last) = self.log_declarations.last_mut() else {
			self.defer("training log interval must follow a .log call");
			return self; }; last.every = every; self }

	#[must_use]
	pub fn log(mut self, items: impl IntoLogItems) -> Self { let mut declared = Vec::new();
		for item in items.into_log_items() { if let Err(error) = item.validate() { self.defer_kind(error.kind, error.detail);
			} else { self.log.push(item); declared.push(item); } }
		if !declared.is_empty() { self.log_declarations.push(LogDeclaration {
				every: NonZeroUsize::new(1).expect("default training log interval is one"),
				items: declared, }); }
		self }

	#[must_use]
	pub fn plot(mut self, items: impl IntoIterator<Item = LogItem>) -> Self { for item in items {
			if let Err(error) = item.validate() { self.defer_kind(error.kind, error.detail); } else { self.plot.push(item); } }
		self }

	#[must_use]
	pub fn resume(mut self, path: impl Into<String>) -> Self { let path = path.into(); if self.resume_declared {
			self.defer(
				"training accepts one resume declaration; use the literal two-path form for model plus kernel",
			); return self; }
		self.resume_declared = true; if path.is_empty() {
			self.defer("training resume path is empty");
		} else if artifact_path_extension(&path) != Some("ogdl") {
			self.defer("training resume requires a semantic .ogdl model as its first argument");
		} else { self.resume.model = Some(path); }
		self }

	/// Source-frontend lowering target for the literal
	/// `.resume("model.ogdl", "kernel.cubin")` form.
	#[doc(hidden)]
	#[must_use]
	pub fn __recipe_resume_pair(mut self, model: impl Into<String>, kernel: impl Into<String>) -> Self {
		let model = model.into(); let kernel = kernel.into(); if self.resume_declared {
			self.defer("training accepts one resume declaration");
			return self; }
		self.resume_declared = true;
		if model.is_empty() || artifact_path_extension(&model) != Some("ogdl") {
			self.defer("the first training resume path must be a semantic .ogdl model");
		} else if kernel.is_empty() || !is_native_kernel_extension(artifact_path_extension(&kernel)) {
			self.defer("the second training resume path must be a .cubin or .hsaco native kernel");
		} else { self.resume.model = Some(model); self.resume.kernel = Some(kernel); }
		self }

	/// Declare one artifact path before execution. The extension selects the
	/// semantic model or native-kernel artifact.
	#[must_use]
	pub fn save(mut self, path: impl Into<String>) -> Self { let path = path.into(); if self.save_declared { self.defer(
				"training accepts one save declaration; use the literal two-path form to save both artifacts",
			); return self; }
		self.save_declared = true; if path.is_empty() {
			self.defer("training save path is empty");
		} else { match artifact_path_extension(&path) {
				Some("ogdl") => self.save.model = Some(path),
				Some("cubin" | "hsaco") => self.save.kernel = Some(path),
				_ => self.defer("training save path must end in .ogdl, .cubin, or .hsaco"),
			} }
		self }

	/// Source-frontend lowering target for the literal
	/// `.save("model.ogdl", "kernel.cubin")` form.
	#[doc(hidden)]
	#[must_use]
	pub fn __recipe_save_pair(mut self, model: impl Into<String>, kernel: impl Into<String>) -> Self {
		let model = model.into(); let kernel = kernel.into(); if self.save_declared {
			self.defer("training accepts one save declaration");
			return self; }
		self.save_declared = true;
		if model.is_empty() || artifact_path_extension(&model) != Some("ogdl") {
			self.defer("the first training save path must be a semantic .ogdl model");
		} else if kernel.is_empty() || !is_native_kernel_extension(artifact_path_extension(&kernel)) {
			self.defer("the second training save path must be a .cubin or .hsaco native kernel");
		} else { self.save.model = Some(model); self.save.kernel = Some(kernel); }
		self }

	pub(crate) fn validate(&self) -> DeclarationResult<()> { if let Some(error) = &self.deferred {
			return Err(error.clone()); }
		if matches!((self.warmup_epochs, self.epochs), (Some(warmup), Some(epochs)) if warmup >= epochs) {
			return Err(DeclarationError::new( DeclarationErrorKind::InvalidTrainingConfiguration,
				"warmup epochs must be less than the total epoch bound",
			)); }
		for item in self.log.iter().chain(&self.plot) { item.validate()?; }
		Ok(()) }

	#[must_use]
	pub const fn epoch_bound(&self) -> Option<usize> { self.epochs }

	#[must_use]
	pub(crate) fn metric_log_interval(&self, metric: Metric) -> Option<std::num::NonZeroU64> {
		self.log_declarations.iter().rev().find_map(|declaration| {
			if declaration.items.iter().any(|item| item.metric() == metric) {
				Some(std::num::NonZeroU64::new(declaration.every.get() as u64).expect("every is nonzero"))
			} else { None } }) }

	#[must_use]
	pub fn learning_rate(&self) -> Option<f32> { self.learning_rate_bits.map(f32::from_bits) }

	#[must_use]
	pub const fn warmup_epoch_bound(&self) -> Option<usize> { self.warmup_epochs }

	#[must_use]
	pub const fn learning_rate_schedule(&self) -> Option<LearningRateSchedule> { self.learning_rate_schedule }

	#[must_use]
	pub const fn optimizer_spec(&self) -> Option<Optimizer> { self.optimizer }

	#[must_use]
	pub fn log_items(&self) -> &[LogItem] { &self.log }

	#[must_use]
	pub fn plot_items(&self) -> &[LogItem] { &self.plot }

	#[must_use]
	pub fn resume_source(&self) -> Option<&str> { self.resume.model.as_deref() }

	pub(crate) fn resume_kernel_source(&self) -> Option<&str> { self.resume.kernel.as_deref() }

	pub(crate) fn save_model_destination(&self) -> Option<&str> { self.save.model.as_deref() }

	pub(crate) fn save_kernel_destination(&self) -> Option<&str> { self.save.kernel.as_deref() }

	fn defer(&mut self, detail: impl Into<String>) {
		self.defer_kind(DeclarationErrorKind::InvalidTrainingConfiguration, detail); }

	fn defer_kind(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) { if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail)); } } }

fn artifact_path_extension(path: &str) -> Option<&str> { std::path::Path::new(path) .extension()
		.and_then(|extension| extension.to_str()) }

fn is_native_kernel_extension(extension: Option<&str>) -> bool { matches!(extension, Some("cubin" | "hsaco")) }

/// Static inference policy and logging declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Infer { log: Vec<LogItem>, deferred: Option<DeclarationError>, }

impl Infer {
	#[must_use]
	pub(crate) const fn new() -> Self { Self { log: Vec::new(), deferred: None, } }

	#[must_use]
	pub fn log(mut self, items: impl IntoLogItems) -> Self { for item in items.into_log_items() {
			if let Err(error) = item.validate() { if self.deferred.is_none() { self.deferred = Some(error); } } else {
				self.log.push(item); if let Some(detail) = item.metric().inference_rejection() && self.deferred.is_none()
				{ self.deferred = Some(DeclarationError::new( DeclarationErrorKind::InvalidInferenceConfiguration,
						format!("{:?}: {detail}", item.metric()),
					)); } } }
		self }

	pub(crate) fn resolve_declaration(&self) -> DeclarationResult<InferenceDeclaration> {
		let sequence = crate::take_recipe_inference_sequence(); self.validate()?;
		let (data, model) = sequence.map_err(|detail| {
			DeclarationError::new(DeclarationErrorKind::InvalidInferenceConfiguration, detail) })?; data.validate()?;
		model.validate()?; Ok(InferenceDeclaration { model, data: Some(data), policy: self.clone(), }) }

	/// Compile and execute target-free inference against the immediately
	/// preceding data and checkpoint-model declarations, then print the exact
	/// prediction rows after native teardown.
	pub fn evaluate(&self) -> crate::inference::InferenceResult<crate::inference::InferenceReport> {
		let declaration = self.resolve_declaration()?; crate::inference::evaluate_inference_declaration(&declaration) }

	pub(crate) fn validate(&self) -> DeclarationResult<()> { match &self.deferred { Some(error) => Err(error.clone()),
			None => Ok(()), } }

	#[must_use]
	pub fn log_items(&self) -> &[LogItem] { &self.log } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceDeclaration { model: Model, data: Option<Data>, policy: Infer, }

impl InferenceDeclaration {
	#[must_use]
	pub const fn model(&self) -> &Model { &self.model }

	#[must_use]
	pub const fn data(&self) -> Option<&Data> { self.data.as_ref() }

	#[must_use]
	pub const fn policy(&self) -> &Infer { &self.policy } }
