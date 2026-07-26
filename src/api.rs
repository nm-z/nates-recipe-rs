use core::fmt;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeclarationErrorKind {
	EmptyValue,
	InvalidExclusion,
	InvalidSplit,
	InvalidLayer,
	InvalidActivation,
	InvalidBayes,
	InvalidLearningRate,
	InvalidMetric,
	InvalidTrainingConfiguration,
	InvalidInferenceConfiguration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationError {
	pub kind: DeclarationErrorKind,
	pub detail: String,
}

impl DeclarationError {
	fn new(kind: DeclarationErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for DeclarationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl std::error::Error for DeclarationError {}

pub type DeclarationResult<T> = Result<T, DeclarationError>;

pub trait IntoTargets {
	fn into_targets(self) -> Vec<String>;
}

impl IntoTargets for &str {
	fn into_targets(self) -> Vec<String> {
		vec![self.to_owned()]
	}
}

impl IntoTargets for String {
	fn into_targets(self) -> Vec<String> {
		vec![self]
	}
}

impl<const N: usize> IntoTargets for [&str; N] {
	fn into_targets(self) -> Vec<String> {
		self.into_iter().map(str::to_owned).collect()
	}
}

impl IntoTargets for &[&str] {
	fn into_targets(self) -> Vec<String> {
		self.iter().map(|value| (*value).to_owned()).collect()
	}
}

impl IntoTargets for Vec<String> {
	fn into_targets(self) -> Vec<String> {
		self
	}
}

/// Literal value used by a row-exclusion predicate.
///
/// Floating-point values retain their exact declaration bits so immutable
/// declarations continue to support structural equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionValue {
	Signed(i64),
	Unsigned(u64),
	FloatBits(u64),
	Boolean(bool),
	Text(String),
}

pub trait IntoConditionValue {
	fn into_condition_value(self) -> ConditionValue;
}

macro_rules! impl_signed_condition_value {
	($($integer:ty),+ $(,)?) => {
		$(
			impl IntoConditionValue for $integer {
				fn into_condition_value(self) -> ConditionValue {
					ConditionValue::Signed(i64::from(self))
				}
			}
		)+
	};
}

macro_rules! impl_unsigned_condition_value {
	($($integer:ty),+ $(,)?) => {
		$(
			impl IntoConditionValue for $integer {
				fn into_condition_value(self) -> ConditionValue {
					ConditionValue::Unsigned(u64::from(self))
				}
			}
		)+
	};
}

impl_signed_condition_value!(i8, i16, i32, i64);
impl_unsigned_condition_value!(u8, u16, u32, u64);

impl IntoConditionValue for isize {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::Signed(self as i64)
	}
}

impl IntoConditionValue for usize {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::Unsigned(self as u64)
	}
}

impl IntoConditionValue for f32 {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::FloatBits((self as f64).to_bits())
	}
}

impl IntoConditionValue for f64 {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::FloatBits(self.to_bits())
	}
}

impl IntoConditionValue for bool {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::Boolean(self)
	}
}

impl IntoConditionValue for &str {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::Text(self.to_owned())
	}
}

impl IntoConditionValue for String {
	fn into_condition_value(self) -> ConditionValue {
		ConditionValue::Text(self)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
	Equal,
	NotEqual,
	Less,
	LessOrEqual,
	Greater,
	GreaterOrEqual,
}

/// Typed predicate selecting rows that must be excluded from the dataset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition {
	column: String,
	operator: ComparisonOperator,
	value: ConditionValue,
}

impl Condition {
	#[must_use]
	pub(crate) fn new(
		column: impl Into<String>,
		operator: ComparisonOperator,
		value: impl IntoConditionValue,
	) -> Self {
		Self {
			column: column.into(),
			operator,
			value: value.into_condition_value(),
		}
	}

	fn validate(&self) -> DeclarationResult<()> {
		if self.column.is_empty() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidExclusion,
				"condition column name is empty",
			));
		}
		if matches!(self.value, ConditionValue::FloatBits(bits) if !f64::from_bits(bits).is_finite()) {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidExclusion,
				"condition comparison value must be finite",
			));
		}
		Ok(())
	}

	#[must_use]
	pub fn column(&self) -> &str {
		&self.column
	}

	#[must_use]
	pub const fn operator(&self) -> ComparisonOperator {
		self.operator
	}

	#[must_use]
	pub const fn value(&self) -> &ConditionValue {
		&self.value
	}
}

/// Implementation detail for the exported [`cond!`] macro.
#[doc(hidden)]
#[must_use]
pub fn __condition(
	column: impl Into<String>,
	operator: ComparisonOperator,
	value: impl IntoConditionValue,
) -> Condition {
	Condition::new(column, operator, value)
}

/// Construct a typed row-exclusion predicate without evaluating the column as
/// a Rust identifier.
#[macro_export]
macro_rules! cond {
	($column:ident <= $value:expr) => {
		$crate::__condition(
			stringify!($column),
			$crate::ComparisonOperator::LessOrEqual,
			$value,
		)
	};
	($column:ident >= $value:expr) => {
		$crate::__condition(
			stringify!($column),
			$crate::ComparisonOperator::GreaterOrEqual,
			$value,
		)
	};
	($column:ident == $value:expr) => {
		$crate::__condition(
			stringify!($column),
			$crate::ComparisonOperator::Equal,
			$value,
		)
	};
	($column:ident != $value:expr) => {
		$crate::__condition(
			stringify!($column),
			$crate::ComparisonOperator::NotEqual,
			$value,
		)
	};
	($column:ident < $value:expr) => {
		$crate::__condition(
			stringify!($column),
			$crate::ComparisonOperator::Less,
			$value,
		)
	};
	($column:ident > $value:expr) => {
		$crate::__condition(
			stringify!($column),
			$crate::ComparisonOperator::Greater,
			$value,
		)
	};
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exclusion {
	Column(String),
	Condition(Condition),
}

pub trait IntoExclusions {
	fn into_exclusions(self) -> Vec<Exclusion>;
}

impl IntoExclusions for &str {
	fn into_exclusions(self) -> Vec<Exclusion> {
		vec![Exclusion::Column(self.to_owned())]
	}
}

impl IntoExclusions for String {
	fn into_exclusions(self) -> Vec<Exclusion> {
		vec![Exclusion::Column(self)]
	}
}

impl<const N: usize> IntoExclusions for [&str; N] {
	fn into_exclusions(self) -> Vec<Exclusion> {
		self.into_iter()
			.map(|column| Exclusion::Column(column.to_owned()))
			.collect()
	}
}

impl IntoExclusions for &[&str] {
	fn into_exclusions(self) -> Vec<Exclusion> {
		self.iter()
			.map(|column| Exclusion::Column((*column).to_owned()))
			.collect()
	}
}

impl IntoExclusions for Vec<String> {
	fn into_exclusions(self) -> Vec<Exclusion> {
		self.into_iter().map(Exclusion::Column).collect()
	}
}

impl IntoExclusions for Condition {
	fn into_exclusions(self) -> Vec<Exclusion> {
		vec![Exclusion::Condition(self)]
	}
}

impl IntoExclusions for Exclusion {
	fn into_exclusions(self) -> Vec<Exclusion> {
		vec![self]
	}
}

impl IntoExclusions for Vec<Exclusion> {
	fn into_exclusions(self) -> Vec<Exclusion> {
		self
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataNormalization {
	ZScore,
	MinMax,
	L2Norm,
}

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
pub struct Data {
	sources: Vec<String>,
	targets: Vec<String>,
	exclusions: Vec<String>,
	condition_exclusions: Vec<Condition>,
	split_fraction_bits: Option<u32>,
	normalization: Option<DataNormalization>,
	deferred: Option<DeclarationError>,
}

impl Data {
	pub(crate) const fn empty() -> Self {
		Self {
			sources: Vec::new(),
			targets: Vec::new(),
			exclusions: Vec::new(),
			condition_exclusions: Vec::new(),
			split_fraction_bits: None,
			normalization: None,
			deferred: None,
		}
	}

	pub fn set(mut self, path: &str) -> Self {
		if path.is_empty() {
			self.defer(
				DeclarationErrorKind::EmptyValue,
				"data source path is empty",
			);
		} else {
			self.sources.push(path.to_owned());
		}
		crate::remember_recipe_data(self.clone());
		self
	}

	pub fn target(mut self, targets: impl IntoTargets) -> Self {
		let targets = targets.into_targets();
		if targets.is_empty() || targets.iter().any(String::is_empty) {
			self.defer(
				DeclarationErrorKind::EmptyValue,
				"target names must be nonempty",
			);
		} else {
			self.targets = targets;
		}
		crate::remember_recipe_data(self.clone());
		self
	}

	pub fn exclude(mut self, exclusions: impl IntoExclusions) -> Self {
		let exclusions = exclusions.into_exclusions();
		if exclusions.is_empty() {
			self.defer(
				DeclarationErrorKind::EmptyValue,
				"at least one column or condition exclusion is required",
			);
		}
		for exclusion in exclusions {
			match exclusion {
				Exclusion::Column(pattern) if pattern.is_empty() => self.defer(
					DeclarationErrorKind::EmptyValue,
					"excluded-column pattern is empty",
				),
				Exclusion::Column(pattern) => self.exclusions.push(pattern),
				Exclusion::Condition(condition) => match condition.validate() {
					Ok(()) => self.condition_exclusions.push(condition),
					Err(error) => self.defer(error.kind, error.detail),
				},
			}
		}
		crate::remember_recipe_data(self.clone());
		self
	}

	pub fn split(mut self, train_fraction: f64) -> Self {
		let narrowed = train_fraction as f32;
		if !train_fraction.is_finite()
			|| train_fraction <= 0.0
			|| train_fraction >= 1.0
			|| narrowed <= 0.0
			|| narrowed >= 1.0
		{
			self.defer(
				DeclarationErrorKind::InvalidSplit,
				format!("split fraction must be finite and in (0, 1), got {train_fraction}"),
			);
		} else {
			self.split_fraction_bits = Some(narrowed.to_bits());
		}
		crate::remember_recipe_data(self.clone());
		self
	}

	pub fn norm(mut self, normalization: DataNormalization) -> Self {
		self.normalization = Some(normalization);
		crate::remember_recipe_data(self.clone());
		self
	}

	pub(crate) fn validate(&self) -> DeclarationResult<()> {
		if let Some(error) = &self.deferred {
			return Err(error.clone());
		}
		if self.sources.is_empty() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::EmptyValue,
				"at least one data source is required",
			));
		}
		Ok(())
	}

	#[must_use]
	pub fn source(&self) -> &str {
		self.sources.first().map_or("", String::as_str)
	}

	#[must_use]
	pub fn sources(&self) -> &[String] {
		&self.sources
	}

	#[must_use]
	pub fn targets(&self) -> &[String] {
		&self.targets
	}

	#[must_use]
	pub fn exclusions(&self) -> &[String] {
		&self.exclusions
	}

	#[must_use]
	pub fn condition_exclusions(&self) -> &[Condition] {
		&self.condition_exclusions
	}

	#[must_use]
	pub fn split_fraction(&self) -> Option<f32> {
		self.split_fraction_bits.map(f32::from_bits)
	}

	#[must_use]
	pub const fn normalization(&self) -> Option<DataNormalization> {
		self.normalization
	}

	fn defer(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) {
		if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail));
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activation {
	#[default]
	Linear,
	Cosine,
	Exponential,
	Logarithm,
	Huber,
	Tangent,
	Relu,
	LeakyRelu,
	Sigmoid,
	Tanh,
	Selu,
	Gelu,
	Silu,
	Elu,
	PRelu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerNormalization {
	LayerNorm,
	BatchNorm,
}

#[allow(non_upper_case_globals)]
pub const layer_norm: LayerNormalization = LayerNormalization::LayerNorm;
#[allow(non_upper_case_globals)]
pub const batch_norm: LayerNormalization = LayerNormalization::BatchNorm;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerOperation {
	Activation(Activation),
	Normalization(LayerNormalization),
}

/// One operation inside a residual branch, retained in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualOperation {
	Layer { width: usize },
	Activation(Activation),
}

/// The shortcut rule implied by a residual branch's declared output width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualSkip {
	IdentityOrLinearProjection,
}

/// Declare a dense operation inside a residual branch.
#[must_use]
pub const fn layer(width: usize) -> ResidualOperation {
	ResidualOperation::Layer { width }
}

/// Declare a ReLU operation inside a residual branch.
#[must_use]
pub const fn relu() -> ResidualOperation {
	ResidualOperation::Activation(Activation::Relu)
}

trait IntoResidualBranch {
	fn into_residual_branch(self) -> Vec<ResidualOperation>;
}

impl IntoResidualBranch for ResidualOperation {
	fn into_residual_branch(self) -> Vec<ResidualOperation> {
		vec![self]
	}
}

impl<const N: usize> IntoResidualBranch for [ResidualOperation; N] {
	fn into_residual_branch(self) -> Vec<ResidualOperation> {
		self.into_iter().collect()
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForestBooster {
	Lgbm { depth: usize },
	Cbst { depth: usize },
	Xgbst { depth: usize },
}

impl ForestBooster {
	const fn depth(&self) -> usize {
		match self {
			Self::Lgbm { depth } | Self::Cbst { depth } | Self::Xgbst { depth } => *depth,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupCount {
	Derived,
	Exact(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupToNeuronRouting {
	Identity {
		width: usize,
	},
	Expand {
		groups: usize,
		neurons: usize,
		neurons_per_group: usize,
	},
	Contract {
		groups: usize,
		neurons: usize,
		groups_per_neuron: usize,
	},
	FullyConnected {
		groups: usize,
		neurons: usize,
	},
}

/// Connection from an immediately preceding grouped block into a dense layer.
///
/// Pooling derives the number of output groups from its input shape. K-means
/// knows the exact group count from its cluster declaration. Once that count is
/// resolved, [`Self::routing`] yields the shared contiguous or fully connected
/// routing rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupToNeuronConnection {
	groups: GroupCount,
	neurons: usize,
}

impl GroupToNeuronConnection {
	#[must_use]
	pub const fn groups(&self) -> GroupCount {
		self.groups
	}

	#[must_use]
	pub const fn neurons(&self) -> usize {
		self.neurons
	}

	/// Resolve routing after the grouped block's actual output count is known.
	///
	/// Expansion and contraction use contiguous ranges. Non-divisible widths use
	/// ordinary full connectivity. An exact group declaration returns `None` if
	/// `groups` disagrees with it.
	#[must_use]
	pub fn routing(&self, groups: usize) -> Option<GroupToNeuronRouting> {
		if groups == 0 || self.neurons == 0 {
			return None;
		}
		if let GroupCount::Exact(expected) = self.groups
			&& groups != expected
		{
			return None;
		}
		if groups == self.neurons {
			Some(GroupToNeuronRouting::Identity { width: groups })
		} else if self.neurons % groups == 0 {
			Some(GroupToNeuronRouting::Expand {
				groups,
				neurons: self.neurons,
				neurons_per_group: self.neurons / groups,
			})
		} else if groups % self.neurons == 0 {
			Some(GroupToNeuronRouting::Contract {
				groups,
				neurons: self.neurons,
				groups_per_neuron: groups / self.neurons,
			})
		} else {
			Some(GroupToNeuronRouting::FullyConnected {
				groups,
				neurons: self.neurons,
			})
		}
	}

	fn valid_for(self, groups: GroupCount) -> bool {
		self.neurons != 0 && self.groups == groups
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerSpec {
	Dense {
		units: usize,
		operations: Vec<LayerOperation>,
	},
	Perc {
		count: usize,
		operations: Vec<LayerOperation>,
	},
	Rnn {
		width: usize,
		operations: Vec<LayerOperation>,
	},
	Gru {
		width: usize,
		operations: Vec<LayerOperation>,
	},
	Lstm {
		width: usize,
		operations: Vec<LayerOperation>,
	},
	Convolution {
		filters: usize,
		kernel: usize,
		activation: Activation,
	},
	Pool {
		size: usize,
		group_to_neuron: Option<GroupToNeuronConnection>,
	},
	Lgbm {
		depth: usize,
	},
	Cbst {
		depth: usize,
	},
	Xgbst {
		depth: usize,
	},
	Forest {
		trees: usize,
		booster: Option<ForestBooster>,
	},
	KMeans {
		clusters: usize,
		group_to_neuron: Option<GroupToNeuronConnection>,
	},
	KnnPrediction {
		neighbors: usize,
	},
	KnnReduction {
		outputs: usize,
		neighbors: usize,
		operations: Vec<LayerOperation>,
	},
	Residual {
		branch: Vec<ResidualOperation>,
		output_width: usize,
		skip: ResidualSkip,
		operations: Vec<LayerOperation>,
	},
	Embedding {
		dimensions: usize,
		vocabulary: Option<usize>,
	},
	Attention {
		heads: usize,
	},
}

impl LayerSpec {
	fn validate(&self) -> DeclarationResult<()> {
		let valid = match self {
			Self::Dense { units, .. } => *units != 0,
			Self::Perc { count, .. } => *count != 0,
			Self::Rnn { width, .. } | Self::Gru { width, .. } | Self::Lstm { width, .. } => *width != 0,
			Self::Convolution {
				filters, kernel, ..
			} => *filters != 0 && *kernel != 0,
			Self::Pool {
				size,
				group_to_neuron,
			} => *size != 0 && group_to_neuron.is_none_or(|connection| connection.valid_for(GroupCount::Derived)),
			Self::Lgbm { depth } | Self::Cbst { depth } | Self::Xgbst { depth } => *depth != 0,
			Self::Forest { trees, booster } => {
				*trees != 0 && booster.as_ref().is_some_and(|booster| booster.depth() != 0)
			}
			Self::KMeans {
				clusters,
				group_to_neuron,
			} => {
				*clusters != 0
					&& group_to_neuron
						.is_none_or(|connection| connection.valid_for(GroupCount::Exact(*clusters)))
			}
			Self::KnnPrediction { neighbors } => *neighbors != 0,
			Self::KnnReduction {
				outputs, neighbors, ..
			} => *outputs != 0 && *neighbors != 0,
			Self::Residual {
				branch,
				output_width,
				skip: ResidualSkip::IdentityOrLinearProjection,
				..
			} => residual_output_width(branch) == Some(*output_width),
			Self::Embedding {
				dimensions,
				vocabulary,
			} => *dimensions != 0 && vocabulary.is_none_or(|value| value != 0),
			Self::Attention { heads } => *heads != 0,
		};
		if valid {
			Ok(())
		} else {
			Err(DeclarationError::new(
				DeclarationErrorKind::InvalidLayer,
				"model blocks must be complete, nonzero, and preserve every declared grouped routing",
			))
		}
	}
}

pub trait IntoLayer {
	fn into_layer(self) -> LayerSpec;
}

impl IntoLayer for usize {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense {
			units: self,
			operations: Vec::new(),
		}
	}
}

impl IntoLayer for LayerSpec {
	fn into_layer(self) -> LayerSpec {
		self
	}
}

trait IntoKnnSpec {
	fn into_knn_spec(self) -> LayerSpec;
}

impl IntoKnnSpec for usize {
	fn into_knn_spec(self) -> LayerSpec {
		LayerSpec::KnnPrediction { neighbors: self }
	}
}

impl IntoKnnSpec for [usize; 2] {
	fn into_knn_spec(self) -> LayerSpec {
		let [outputs, neighbors] = self;
		LayerSpec::KnnReduction {
			outputs,
			neighbors,
			operations: Vec::new(),
		}
	}
}

fn residual_output_width(branch: &[ResidualOperation]) -> Option<usize> {
	let mut output_width = None;
	for operation in branch {
		match operation {
			ResidualOperation::Layer { width: 0 } => return None,
			ResidualOperation::Layer { width } => output_width = Some(*width),
			ResidualOperation::Activation(_) => {}
		}
	}
	output_width
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loss {
	MeanSquaredError,
	MeanAbsoluteError,
	Huber,
	BinaryCrossEntropy,
	CrossEntropy,
	Focal,
}

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
pub const focal: Loss = Loss::Focal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grad {
	clip_bits: Option<u32>,
	invalid_clip_bits: Option<u64>,
}

/// Configure an explicit global gradient clipping norm for [`Model::grad`].
#[must_use]
pub fn clip(maximum_norm: f64) -> Grad {
	let narrowed = maximum_norm as f32;
	if maximum_norm.is_finite() && narrowed.is_finite() && narrowed > 0.0 {
		Grad {
			clip_bits: Some(narrowed.to_bits()),
			invalid_clip_bits: None,
		}
	} else {
		Grad {
			clip_bits: None,
			invalid_clip_bits: Some(maximum_norm.to_bits()),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Objective {
	Builtin(Loss),
	Reference(Box<Model>),
}

pub trait IntoObjective {
	fn into_objective(self) -> Objective;
}

impl IntoObjective for Loss {
	fn into_objective(self) -> Objective {
		Objective::Builtin(self)
	}
}

impl IntoObjective for &Model {
	fn into_objective(self) -> Objective {
		Objective::Reference(Box::new(self.clone()))
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Optimizer {
	AdamW,
}

#[allow(non_upper_case_globals)]
pub const adamw: Optimizer = Optimizer::AdamW;

/// One conditional dependency in a Bayesian model.
///
/// The edge direction is `parent -> child`; declaration order is retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesDependency {
	child: String,
	parents: Vec<String>,
}

impl BayesDependency {
	#[must_use]
	pub fn child(&self) -> &str {
		&self.child
	}

	#[must_use]
	pub fn parents(&self) -> &[String] {
		&self.parents
	}

	fn validate(&self) -> DeclarationResult<()> {
		if self.child.is_empty() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes,
				"Bayesian child name is empty",
			));
		}
		if self.parents.iter().any(String::is_empty) {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes,
				format!(
					"Bayesian dependency for {:?} contains an empty parent name",
					self.child
				),
			));
		}
		if self.parents.iter().any(|parent| parent == &self.child) {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes,
				format!(
					"Bayesian dependency {:?} cannot name itself as a parent",
					self.child
				),
			));
		}
		let unique = self.parents.iter().collect::<BTreeSet<_>>();
		if unique.len() != self.parents.len() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes,
				format!(
					"Bayesian dependency {:?} contains a duplicate parent",
					self.child
				),
			));
		}
		Ok(())
	}
}

const CHECKPOINT_MODEL_DECLARATION_CONFLICT: &str =
	"a checkpoint-backed model cannot also declare layers, Bayesian dependencies, a loss, or gradient policy";

/// Backend-neutral model declaration. It contains no runtime handles, loaded
/// weights, allocations, or mutable global registry entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
	layers: Vec<LayerSpec>,
	bayes_dependencies: Vec<BayesDependency>,
	objective: Option<Objective>,
	gradient_clip_bits: Option<u32>,
	weights_source: Option<String>,
	deferred: Option<DeclarationError>,
}

impl Model {
	#[must_use]
	pub(crate) const fn new() -> Self {
		Self {
			layers: Vec::new(),
			bayes_dependencies: Vec::new(),
			objective: None,
			gradient_clip_bits: None,
			weights_source: None,
			deferred: None,
		}
	}

	/// Declare a checkpoint-backed model.
	///
	/// Input width, feature schema, topology, and parameter shapes are read from
	/// the checkpoint during inference preparation; callers do not redeclare
	/// them here.
	pub fn load(mut self, path: &str) -> Self {
		if path.is_empty() {
			self.defer(
				DeclarationErrorKind::EmptyValue,
				"model checkpoint path is empty",
			);
		} else if self.has_inline_definition() {
			self.defer(
				DeclarationErrorKind::InvalidLayer,
				CHECKPOINT_MODEL_DECLARATION_CONFLICT,
			);
		} else if self.weights_source.is_some() {
			self.defer(
				DeclarationErrorKind::InvalidLayer,
				"a model accepts exactly one checkpoint source",
			);
		} else {
			self.weights_source = Some(path.to_owned());
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn layer(mut self, spec: impl IntoLayer) -> Self {
		let spec = spec.into_layer();
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			if let LayerSpec::Dense { units, .. } = &spec {
				match self.layers.last_mut() {
					Some(LayerSpec::Pool {
						group_to_neuron, ..
					}) => {
						*group_to_neuron = Some(GroupToNeuronConnection {
							groups: GroupCount::Derived,
							neurons: *units,
						});
					}
					Some(LayerSpec::KMeans {
						clusters,
						group_to_neuron,
					}) => {
						*group_to_neuron = Some(GroupToNeuronConnection {
							groups: GroupCount::Exact(*clusters),
							neurons: *units,
						});
					}
					_ => {}
				}
			}
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn embed(mut self, dimensions: usize) -> Self {
		let spec = LayerSpec::Embedding {
			dimensions,
			vocabulary: None,
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn vocab(mut self, vocabulary: usize) -> Self {
		if vocabulary == 0 {
			self.defer(
				DeclarationErrorKind::InvalidLayer,
				"embedding vocabulary must be nonzero",
			);
		} else {
			match self.layers.last_mut() {
				Some(LayerSpec::Embedding {
					vocabulary: current,
					..
				}) => *current = Some(vocabulary),
				_ => self.defer(
					DeclarationErrorKind::InvalidLayer,
					"vocab requires a preceding embedding block",
				),
			}
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn attn(mut self, heads: usize) -> Self {
		let spec = LayerSpec::Attention { heads };
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	/// Add one block containing `count` parallel perceptrons.
	pub fn perc(mut self, count: usize) -> Self {
		let spec = LayerSpec::Perc {
			count,
			operations: Vec::new(),
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	/// Add a recurrent neural-network block with `width` recurrent states.
	pub fn rnn(mut self, width: usize) -> Self {
		let spec = LayerSpec::Rnn {
			width,
			operations: Vec::new(),
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	/// Add a gated recurrent-unit block with `width` recurrent states.
	pub fn gru(mut self, width: usize) -> Self {
		let spec = LayerSpec::Gru {
			width,
			operations: Vec::new(),
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	/// Add a long short-term-memory block with `width` recurrent states.
	pub fn lstm(mut self, width: usize) -> Self {
		let spec = LayerSpec::Lstm {
			width,
			operations: Vec::new(),
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	/// Declare that `child` is conditionally modeled from `parents`.
	///
	/// Repeated calls append to one network in source order. Each child has one
	/// declaration, while a parent may feed any number of children. Parent nodes
	/// may be declared later or remain implicit roots. The resulting network
	/// must be acyclic.
	pub fn bayes(mut self, child: &str, parents: impl IntoTargets) -> Self {
		let dependency = BayesDependency {
			child: child.to_owned(),
			parents: parents.into_targets(),
		};
		self.bayes_dependencies.push(dependency);
		if let Err(error) = validate_bayes_network(&self.bayes_dependencies) {
			self.bayes_dependencies.pop();
			self.defer(error.kind, error.detail);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn conv(mut self, filters: usize, kernel: usize) -> Self {
		let spec = LayerSpec::Convolution {
			filters,
			kernel,
			activation: Activation::Linear,
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn pool(mut self, size: usize) -> Self {
		let spec = LayerSpec::Pool {
			size,
			group_to_neuron: None,
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn lgbm(self, depth: usize) -> Self {
		self.with_tree_booster(ForestBooster::Lgbm { depth })
	}

	pub fn cbst(self, depth: usize) -> Self {
		self.with_tree_booster(ForestBooster::Cbst { depth })
	}

	pub fn xgbst(self, depth: usize) -> Self {
		self.with_tree_booster(ForestBooster::Xgbst { depth })
	}

	pub fn forest(mut self, trees: usize) -> Self {
		if trees == 0 {
			self.defer(
				DeclarationErrorKind::InvalidLayer,
				"forest tree count must be nonzero",
			);
		} else {
			self.layers.push(LayerSpec::Forest {
				trees,
				booster: None,
			});
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn kmeans(mut self, clusters: usize) -> Self {
		let spec = LayerSpec::KMeans {
			clusters,
			group_to_neuron: None,
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	#[allow(private_bounds)]
	pub fn knn(mut self, spec: impl IntoKnnSpec) -> Self {
		let spec = spec.into_knn_spec();
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	#[allow(private_bounds)]
	pub fn residual(mut self, branch: impl IntoResidualBranch) -> Self {
		let branch = branch.into_residual_branch();
		if let Some(output_width) = residual_output_width(&branch) {
			self.layers.push(LayerSpec::Residual {
				branch,
				output_width,
				skip: ResidualSkip::IdentityOrLinearProjection,
				operations: Vec::new(),
			});
		} else {
			self.defer(
				DeclarationErrorKind::InvalidLayer,
				"a residual branch requires at least one layer and every declared layer width must be nonzero",
			);
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn relu(self) -> Self {
		self.with_last_activation(Activation::Relu)
	}

	pub fn leak(self) -> Self {
		self.with_last_activation(Activation::LeakyRelu)
	}

	pub fn sigmoid(self) -> Self {
		self.with_last_activation(Activation::Sigmoid)
	}

	pub fn tanh(self) -> Self {
		self.with_last_activation(Activation::Tanh)
	}

	pub fn selu(self) -> Self {
		self.with_last_activation(Activation::Selu)
	}

	pub fn gelu(self) -> Self {
		self.with_last_activation(Activation::Gelu)
	}

	pub fn silu(self) -> Self {
		self.with_last_activation(Activation::Silu)
	}

	pub fn elu(self) -> Self {
		self.with_last_activation(Activation::Elu)
	}

	pub fn prelu(self) -> Self {
		self.with_last_activation(Activation::PRelu)
	}

	pub fn cos(self) -> Self {
		self.with_last_activation(Activation::Cosine)
	}

	pub fn exp(self) -> Self {
		self.with_last_activation(Activation::Exponential)
	}

	/// Applies the signed logarithmic activation
	/// `sign(x) * ln(1 + abs(x))`.
	pub fn log(self) -> Self {
		self.with_last_activation(Activation::Logarithm)
	}

	pub fn huber(self) -> Self {
		self.with_last_activation(Activation::Huber)
	}

	pub fn tan(self) -> Self {
		self.with_last_activation(Activation::Tangent)
	}

	pub fn loss(mut self, objective: impl IntoObjective) -> Self {
		self.objective = Some(objective.into_objective());
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn grad(mut self, gradient: Grad) -> Self {
		match gradient.clip_bits {
			Some(bits) => self.gradient_clip_bits = Some(bits),
			None => {
				let maximum_norm = gradient
					.invalid_clip_bits
					.map(f64::from_bits)
					.unwrap_or(f64::NAN);
				self.defer(
					DeclarationErrorKind::InvalidTrainingConfiguration,
					format!(
						"gradient clipping norm must be finite, positive, and representable as f32, got {maximum_norm}"
					),
				);
			}
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub fn norm(mut self, normalization: LayerNormalization) -> Self {
		match self.layers.last_mut() {
			Some(
				LayerSpec::Dense { operations, .. }
				| LayerSpec::Perc { operations, .. }
				| LayerSpec::Rnn { operations, .. }
				| LayerSpec::Gru { operations, .. }
				| LayerSpec::Lstm { operations, .. }
				| LayerSpec::KnnReduction { operations, .. }
				| LayerSpec::Residual { operations, .. },
			) => {
				operations.push(LayerOperation::Normalization(normalization));
			}
			Some(
				LayerSpec::Convolution { .. }
				| LayerSpec::Pool { .. }
				| LayerSpec::Lgbm { .. }
				| LayerSpec::Cbst { .. }
				| LayerSpec::Xgbst { .. }
				| LayerSpec::Forest { .. }
				| LayerSpec::KMeans { .. }
				| LayerSpec::KnnPrediction { .. }
				| LayerSpec::Embedding { .. }
				| LayerSpec::Attention { .. },
			)
			| None => self.defer(
				DeclarationErrorKind::InvalidLayer,
				"layer normalization requires a preceding dense, perceptron, recurrent, KNN reduction, or residual block",
			),
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	pub(crate) fn validate(&self) -> DeclarationResult<()> {
		if let Some(error) = &self.deferred {
			return Err(error.clone());
		}
		if self.weights_source.is_some() && self.has_inline_definition() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidLayer,
				CHECKPOINT_MODEL_DECLARATION_CONFLICT,
			));
		}
		if self.layers.is_empty() && self.bayes_dependencies.is_empty() && self.weights_source.is_none() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidLayer,
				"a model requires at least one layer, Bayesian dependency, or declared weight source",
			));
		}
		for (index, layer) in self.layers.iter().enumerate() {
			layer.validate()?;
			let connection = match layer {
				LayerSpec::Pool {
					group_to_neuron, ..
				}
				| LayerSpec::KMeans {
					group_to_neuron, ..
				} => group_to_neuron.as_ref(),
				_ => None,
			};
			if let Some(connection) = connection {
				let has_exact_following_layer = matches!(
					self.layers.get(index + 1),
					Some(LayerSpec::Dense { units, .. }) if *units == connection.neurons()
				);
				if !has_exact_following_layer {
					return Err(DeclarationError::new(
						DeclarationErrorKind::InvalidLayer,
						"a grouped-to-dense connection must refer to the immediately following layer",
					));
				}
			}
		}
		validate_bayes_network(&self.bayes_dependencies)?;
		if let Some(Objective::Reference(model)) = &self.objective {
			model.validate()?;
		}
		Ok(())
	}

	#[must_use]
	pub fn layers(&self) -> &[LayerSpec] {
		&self.layers
	}

	#[must_use]
	pub fn bayes_dependencies(&self) -> &[BayesDependency] {
		&self.bayes_dependencies
	}

	#[must_use]
	pub const fn objective(&self) -> Option<&Objective> {
		self.objective.as_ref()
	}

	#[must_use]
	pub fn gradient_clip_value(&self) -> Option<f32> {
		self.gradient_clip_bits.map(f32::from_bits)
	}

	#[must_use]
	pub fn weights_source(&self) -> Option<&str> {
		self.weights_source.as_deref()
	}

	fn has_inline_definition(&self) -> bool {
		!self.layers.is_empty()
			|| !self.bayes_dependencies.is_empty()
			|| self.objective.is_some()
			|| self.gradient_clip_bits.is_some()
	}

	fn with_tree_booster(mut self, booster: ForestBooster) -> Self {
		let depth = booster.depth();
		if depth == 0 {
			self.defer(
				DeclarationErrorKind::InvalidLayer,
				"boosted-tree depth must be nonzero",
			);
		} else if let Some(LayerSpec::Forest {
			booster: pending @ None,
			..
		}) = self.layers.last_mut()
		{
			*pending = Some(booster);
		} else {
			self.layers.push(match booster {
				ForestBooster::Lgbm { depth } => LayerSpec::Lgbm { depth },
				ForestBooster::Cbst { depth } => LayerSpec::Cbst { depth },
				ForestBooster::Xgbst { depth } => LayerSpec::Xgbst { depth },
			});
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	fn with_last_activation(mut self, activation: Activation) -> Self {
		match self.layers.last_mut() {
			Some(
				LayerSpec::Dense { operations, .. }
				| LayerSpec::Perc { operations, .. }
				| LayerSpec::Rnn { operations, .. }
				| LayerSpec::Gru { operations, .. }
				| LayerSpec::Lstm { operations, .. }
				| LayerSpec::KnnReduction { operations, .. }
				| LayerSpec::Residual { operations, .. },
			) => {
				operations.push(LayerOperation::Activation(activation));
			}
			Some(LayerSpec::Convolution {
				activation: current,
				..
			}) => *current = activation,
			Some(
				LayerSpec::Pool { .. }
				| LayerSpec::Lgbm { .. }
				| LayerSpec::Cbst { .. }
				| LayerSpec::Xgbst { .. }
				| LayerSpec::Forest { .. }
				| LayerSpec::KMeans { .. }
				| LayerSpec::KnnPrediction { .. }
				| LayerSpec::Embedding { .. }
				| LayerSpec::Attention { .. },
			)
			| None => self.defer(
				DeclarationErrorKind::InvalidActivation,
				concat!(
					"activation methods require a preceding dense, perceptron, recurrent, convolution, KNN reduction,",
					" or residual block",
				),
			),
		}
		crate::remember_recipe_model(self.clone());
		self
	}

	fn defer(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) {
		if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail));
		}
	}
}

fn validate_bayes_network(dependencies: &[BayesDependency]) -> DeclarationResult<()> {
	let mut children = BTreeSet::new();
	for dependency in dependencies {
		dependency.validate()?;
		if !children.insert(dependency.child.as_str()) {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidBayes,
				format!(
					"Bayesian child {:?} is declared more than once",
					dependency.child
				),
			));
		}
	}
	if bayes_dependencies_have_cycle(dependencies) {
		return Err(DeclarationError::new(
			DeclarationErrorKind::InvalidBayes,
			"Bayesian dependencies contain a cycle",
		));
	}
	Ok(())
}

fn bayes_dependencies_have_cycle(dependencies: &[BayesDependency]) -> bool {
	dependencies.iter().any(|dependency| {
		dependency
			.parents
			.iter()
			.any(|parent| bayes_path_exists(dependencies, &dependency.child, parent))
	})
}

fn bayes_path_exists(dependencies: &[BayesDependency], from: &str, to: &str) -> bool {
	let mut pending = vec![from];
	let mut visited = BTreeSet::new();
	while let Some(current) = pending.pop() {
		if current == to {
			return true;
		}
		if !visited.insert(current) {
			continue;
		}
		for dependency in dependencies {
			if dependency.parents.iter().any(|parent| parent == current) {
				pending.push(dependency.child.as_str());
			}
		}
	}
	false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Metric {
	Loss,
	Accuracy,
	R2,
	AuRoc,
	AuPrc,
	Brier,
	CalibrationError,
	Epoch,
	LearningRate,
	Time,
	Device,
}

impl Metric {
	fn validate(self) -> DeclarationResult<()> {
		Ok(())
	}

	const fn inference_rejection(self) -> Option<&'static str> {
		match self {
			Self::Time | Self::Device => None,
			Self::Loss
			| Self::Accuracy
			| Self::R2
			| Self::AuRoc
			| Self::AuPrc
			| Self::Brier
			| Self::CalibrationError => Some("target-free inference has no target values for this metric"),
			Self::Epoch | Self::LearningRate => Some("inference has no training epoch or optimizer state for this metric"),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogItem {
	metric: Metric,
}

impl LogItem {
	fn validate(self) -> DeclarationResult<()> {
		self.metric.validate()
	}

	#[must_use]
	pub(crate) const fn metric(self) -> Metric {
		self.metric
	}
}

pub trait IntoLogItems {
	fn into_log_items(self) -> Vec<LogItem>;
}

impl IntoLogItems for LogItem {
	fn into_log_items(self) -> Vec<LogItem> {
		vec![self]
	}
}

impl<const N: usize> IntoLogItems for [LogItem; N] {
	fn into_log_items(self) -> Vec<LogItem> {
		self.into()
	}
}

impl IntoLogItems for Vec<LogItem> {
	fn into_log_items(self) -> Vec<LogItem> {
		self
	}
}

impl IntoLogItems for &[LogItem] {
	fn into_log_items(self) -> Vec<LogItem> {
		self.to_vec()
	}
}

#[allow(non_upper_case_globals)]
pub const Accuracy: LogItem = LogItem {
	metric: Metric::Accuracy,
};
#[allow(non_upper_case_globals)]
pub const R2: LogItem = LogItem { metric: Metric::R2 };
#[allow(non_upper_case_globals)]
pub const AuRoc: LogItem = LogItem {
	metric: Metric::AuRoc,
};
#[allow(non_upper_case_globals)]
pub const AuPrc: LogItem = LogItem {
	metric: Metric::AuPrc,
};
#[allow(non_upper_case_globals)]
pub const Brier: LogItem = LogItem {
	metric: Metric::Brier,
};
#[allow(non_upper_case_globals)]
pub const CalibrationError: LogItem = LogItem {
	metric: Metric::CalibrationError,
};
#[allow(non_upper_case_globals)]
pub const Epoch: LogItem = LogItem {
	metric: Metric::Epoch,
};
#[allow(non_upper_case_globals)]
pub const Lr: LogItem = LogItem {
	metric: Metric::LearningRate,
};
#[allow(non_upper_case_globals)]
pub const Time: LogItem = LogItem {
	metric: Metric::Time,
};
#[allow(non_upper_case_globals)]
pub const Device: LogItem = LogItem {
	metric: Metric::Device,
};
#[allow(non_upper_case_globals)]
pub const LossMetric: LogItem = LogItem {
	metric: Metric::Loss,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LearningRateSchedule {
	LinearDecay,
	CosineDecay,
	ExponentialDecay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EarlyStopping {
	metric: LogItem,
	patience: usize,
}

impl EarlyStopping {
	#[must_use]
	pub const fn metric(self) -> LogItem {
		self.metric
	}

	#[must_use]
	pub const fn patience(self) -> usize {
		self.patience
	}
}

pub trait SavePath {
	fn or_default(self) -> String;
}

impl SavePath for () {
	fn or_default(self) -> String {
		"model.ogdl".to_owned()
	}
}

impl SavePath for &str {
	fn or_default(self) -> String {
		self.to_owned()
	}
}

impl SavePath for String {
	fn or_default(self) -> String {
		self
	}
}

/// Static training policy. Calling builder methods does not probe hardware,
/// parse data, prepare a bundle, or start a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Train {
	batch_fraction_bits: Option<u32>,
	epochs: Option<usize>,
	learning_rate_bits: Option<u32>,
	warmup_epochs: Option<usize>,
	learning_rate_schedule: Option<LearningRateSchedule>,
	optimizer: Option<Optimizer>,
	early_stopping: Option<EarlyStopping>,
	log_every: Option<usize>,
	log: Vec<LogItem>,
	plot: Vec<LogItem>,
	resume: Option<String>,
	deferred: Option<DeclarationError>,
}

impl Train {
	#[must_use]
	pub(crate) const fn new() -> Self {
		Self {
			batch_fraction_bits: None,
			epochs: None,
			learning_rate_bits: None,
			warmup_epochs: None,
			learning_rate_schedule: None,
			optimizer: None,
			early_stopping: None,
			log_every: None,
			log: Vec::new(),
			plot: Vec::new(),
			resume: None,
			deferred: None,
		}
	}

	#[must_use]
	pub fn batch(mut self, fraction: f64) -> Self {
		let narrowed = fraction as f32;
		if !fraction.is_finite() || fraction <= 0.0 || fraction >= 1.0 || narrowed <= 0.0 || narrowed >= 1.0 {
			self.defer(format!(
				"training batch fraction must be finite and in (0, 1), got {fraction}"
			));
		} else {
			self.batch_fraction_bits = Some(narrowed.to_bits());
		}
		self
	}

	#[must_use]
	pub fn epochs(mut self, epochs: usize) -> Self {
		if epochs == 0 {
			self.defer("training epoch bound must be nonzero");
		} else {
			self.epochs = Some(epochs);
		}
		self
	}

	#[must_use]
	pub fn lr(mut self, rate: f64) -> Self {
		let rate_f32 = rate as f32;
		if !rate.is_finite() || !rate_f32.is_finite() || rate_f32 <= 0.0 {
			self.defer_kind(
				DeclarationErrorKind::InvalidLearningRate,
				format!(
					"training learning rate must be finite, positive, and representable as f32, got {rate}"
				),
			);
		} else {
			self.learning_rate_bits = Some(rate_f32.to_bits());
			self.learning_rate_schedule = Some(LearningRateSchedule::LinearDecay);
		}
		self
	}

	#[must_use]
	pub fn warmup(mut self, epochs: usize) -> Self {
		if epochs == 0 {
			self.defer("training warmup epoch bound must be nonzero");
		} else {
			self.warmup_epochs = Some(epochs);
		}
		self
	}

	#[must_use]
	pub const fn cos(mut self) -> Self {
		self.learning_rate_schedule = Some(LearningRateSchedule::CosineDecay);
		self
	}

	#[must_use]
	pub const fn exp(mut self) -> Self {
		self.learning_rate_schedule = Some(LearningRateSchedule::ExponentialDecay);
		self
	}

	#[must_use]
	pub const fn optimizer(mut self, optimizer: Optimizer) -> Self {
		self.optimizer = Some(optimizer);
		self
	}

	#[must_use]
	pub fn early_stop(mut self, metric: LogItem, patience: usize) -> Self {
		if let Err(error) = metric.validate() {
			self.defer_kind(error.kind, error.detail);
		} else if patience == 0 {
			self.defer("early-stopping patience must be nonzero");
		} else {
			self.early_stopping = Some(EarlyStopping { metric, patience });
		}
		self
	}

	#[must_use]
	pub fn log_every(mut self, interval: usize) -> Self {
		if interval == 0 {
			self.defer("training log interval must be nonzero");
		} else {
			self.log_every = Some(interval);
		}
		self
	}

	#[must_use]
	pub fn log(mut self, items: impl IntoLogItems) -> Self {
		for item in items.into_log_items() {
			if let Err(error) = item.validate() {
				self.defer_kind(error.kind, error.detail);
			} else {
				self.log.push(item);
			}
		}
		self
	}

	#[must_use]
	pub fn plot(mut self, items: impl IntoIterator<Item = LogItem>) -> Self {
		for item in items {
			if let Err(error) = item.validate() {
				self.defer_kind(error.kind, error.detail);
			} else {
				self.plot.push(item);
			}
		}
		self
	}

	#[must_use]
	pub fn resume(mut self, path: impl SavePath) -> Self {
		let path = path.or_default();
		if path.is_empty() {
			self.defer("training resume path is empty");
		} else {
			self.resume = Some(path);
		}
		self
	}

	pub(crate) fn validate(&self) -> DeclarationResult<()> {
		if let Some(error) = &self.deferred {
			return Err(error.clone());
		}
		if matches!((self.warmup_epochs, self.epochs), (Some(warmup), Some(epochs)) if warmup >= epochs) {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidTrainingConfiguration,
				"warmup epochs must be less than the total epoch bound",
			));
		}
		for item in self.log.iter().chain(&self.plot) {
			item.validate()?;
		}
		if let Some(early_stopping) = self.early_stopping {
			early_stopping.metric.validate()?;
		}
		Ok(())
	}

	#[must_use]
	pub fn batch_fraction(&self) -> Option<f32> {
		self.batch_fraction_bits.map(f32::from_bits)
	}

	#[must_use]
	pub const fn epoch_bound(&self) -> Option<usize> {
		self.epochs
	}

	#[must_use]
	pub const fn log_interval(&self) -> Option<usize> {
		self.log_every
	}

	#[must_use]
	pub fn learning_rate(&self) -> Option<f32> {
		self.learning_rate_bits.map(f32::from_bits)
	}

	#[must_use]
	pub const fn warmup_epoch_bound(&self) -> Option<usize> {
		self.warmup_epochs
	}

	#[must_use]
	pub const fn learning_rate_schedule(&self) -> Option<LearningRateSchedule> {
		self.learning_rate_schedule
	}

	#[must_use]
	pub const fn optimizer_spec(&self) -> Option<Optimizer> {
		self.optimizer
	}

	#[must_use]
	pub const fn early_stopping(&self) -> Option<EarlyStopping> {
		self.early_stopping
	}

	#[must_use]
	pub fn log_items(&self) -> &[LogItem] {
		&self.log
	}

	#[must_use]
	pub fn plot_items(&self) -> &[LogItem] {
		&self.plot
	}

	#[must_use]
	pub fn resume_source(&self) -> Option<&str> {
		self.resume.as_deref()
	}

	fn defer(&mut self, detail: impl Into<String>) {
		self.defer_kind(DeclarationErrorKind::InvalidTrainingConfiguration, detail);
	}

	fn defer_kind(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) {
		if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail));
		}
	}
}

/// Static inference policy and logging declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Infer {
	log: Vec<LogItem>,
	deferred: Option<DeclarationError>,
}

impl Infer {
	#[must_use]
	pub(crate) const fn new() -> Self {
		Self {
			log: Vec::new(),
			deferred: None,
		}
	}

	#[must_use]
	pub fn log(mut self, items: impl IntoLogItems) -> Self {
		for item in items.into_log_items() {
			if let Err(error) = item.validate() {
				if self.deferred.is_none() {
					self.deferred = Some(error);
				}
			} else {
				self.log.push(item);
				if let Some(detail) = item.metric().inference_rejection()
					&& self.deferred.is_none()
				{
					self.deferred = Some(DeclarationError::new(
						DeclarationErrorKind::InvalidInferenceConfiguration,
						format!("{:?}: {detail}", item.metric()),
					));
				}
			}
		}
		self
	}

	/// Resolve this policy against the immediately preceding `recipe.data(...)`
	/// and `recipe.model()` declarations.
	pub fn evaluate(&self) -> DeclarationResult<InferenceDeclaration> {
		let sequence = crate::take_recipe_inference_sequence();
		self.validate()?;
		let (data, model) = sequence.map_err(|detail| {
			DeclarationError::new(DeclarationErrorKind::InvalidInferenceConfiguration, detail)
		})?;
		data.validate()?;
		model.validate()?;
		Ok(InferenceDeclaration {
			model,
			data: Some(data),
			policy: self.clone(),
		})
	}

	pub(crate) fn validate(&self) -> DeclarationResult<()> {
		match &self.deferred {
			Some(error) => Err(error.clone()),
			None => Ok(()),
		}
	}

	#[must_use]
	pub fn log_items(&self) -> &[LogItem] {
		&self.log
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceDeclaration {
	model: Model,
	data: Option<Data>,
	policy: Infer,
}

impl InferenceDeclaration {
	#[must_use]
	pub const fn model(&self) -> &Model {
		&self.model
	}

	#[must_use]
	pub const fn data(&self) -> Option<&Data> {
		self.data.as_ref()
	}

	#[must_use]
	pub const fn policy(&self) -> &Infer {
		&self.policy
	}
}
