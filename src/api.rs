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
	pub fn new(column: impl Into<String>, operator: ComparisonOperator, value: impl IntoConditionValue) -> Self {
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

/// Construct a typed row-exclusion predicate without evaluating the column as
/// a Rust identifier.
#[macro_export]
macro_rules! cond {
	($column:ident <= $value:expr) => {
		$crate::Condition::new(
			stringify!($column),
			$crate::ComparisonOperator::LessOrEqual,
			$value,
		)
	};
	($column:ident >= $value:expr) => {
		$crate::Condition::new(
			stringify!($column),
			$crate::ComparisonOperator::GreaterOrEqual,
			$value,
		)
	};
	($column:ident == $value:expr) => {
		$crate::Condition::new(
			stringify!($column),
			$crate::ComparisonOperator::Equal,
			$value,
		)
	};
	($column:ident != $value:expr) => {
		$crate::Condition::new(
			stringify!($column),
			$crate::ComparisonOperator::NotEqual,
			$value,
		)
	};
	($column:ident < $value:expr) => {
		$crate::Condition::new(
			stringify!($column),
			$crate::ComparisonOperator::Less,
			$value,
		)
	};
	($column:ident > $value:expr) => {
		$crate::Condition::new(
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

/// Immutable data-ingestion declaration.
///
/// Construction records paths and preprocessing policy only. Filesystem reads,
/// parsing, encoding, and the single per-device data-image admission occur in
/// the preparation and init lifecycle, never in these builder calls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Data {
	sources: Vec<String>,
	targets: Vec<String>,
	test_source: Option<String>,
	exclusions: Vec<String>,
	condition_exclusions: Vec<Condition>,
	split_fraction_bits: Option<u32>,
	deferred: Option<DeclarationError>,
}

impl Data {
	#[must_use]
	pub fn load(path: &str) -> Self {
		Self {
			sources: Vec::new(),
			targets: Vec::new(),
			test_source: None,
			exclusions: Vec::new(),
			condition_exclusions: Vec::new(),
			split_fraction_bits: None,
			deferred: None,
		}
		.set(path)
	}

	#[must_use]
	pub fn set(mut self, path: &str) -> Self {
		if path.is_empty() {
			self.defer(
				DeclarationErrorKind::EmptyValue,
				"data source path is empty",
			);
		} else {
			self.sources.push(path.to_owned());
		}
		self
	}

	#[must_use]
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
		self
	}

	#[must_use]
	pub fn test(mut self, path: &str) -> Self {
		if path.is_empty() {
			self.defer(
				DeclarationErrorKind::EmptyValue,
				"test source path is empty",
			);
		} else {
			self.test_source = Some(path.to_owned());
		}
		self
	}

	#[must_use]
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
		self
	}

	#[must_use]
	pub fn split(mut self, train_fraction: f64) -> Self {
		let narrowed = train_fraction as f32;
		if !train_fraction.is_finite() || !(0.0..1.0).contains(&train_fraction) || !(0.0..1.0).contains(&narrowed) {
			self.defer(
				DeclarationErrorKind::InvalidSplit,
				format!("split fraction must be finite and in (0, 1), got {train_fraction}"),
			);
		} else {
			self.split_fraction_bits = Some(narrowed.to_bits());
		}
		self
	}

	pub fn validate(&self) -> DeclarationResult<()> {
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
	pub fn test_source(&self) -> Option<&str> {
		self.test_source.as_deref()
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerSpec {
	Dense {
		units: usize,
		activation: Activation,
	},
	Convolution {
		filters: usize,
		kernel: usize,
		stride: usize,
		activation: Activation,
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
			Self::Convolution {
				filters,
				kernel,
				stride,
				..
			} => *filters != 0 && *kernel != 0 && *stride != 0,
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
				"layer dimensions, heads, filters, kernels, and strides must be nonzero",
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
			activation: Activation::Linear,
		}
	}
}

impl IntoLayer for LayerSpec {
	fn into_layer(self) -> LayerSpec {
		self
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseSpec {
	units: usize,
	activation: Activation,
}

impl DenseSpec {
	#[must_use]
	pub const fn new(units: usize) -> Self {
		Self {
			units,
			activation: Activation::Linear,
		}
	}

	#[must_use]
	pub const fn activation(mut self, activation: Activation) -> Self {
		self.activation = activation;
		self
	}
}

impl IntoLayer for DenseSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense {
			units: self.units,
			activation: self.activation,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbedSpec {
	dimensions: usize,
	vocabulary: Option<usize>,
}

#[must_use]
pub const fn embed(dimensions: usize) -> EmbedSpec {
	EmbedSpec {
		dimensions,
		vocabulary: None,
	}
}

impl EmbedSpec {
	#[must_use]
	pub const fn vocab(mut self, vocabulary: usize) -> Self {
		self.vocabulary = Some(vocabulary);
		self
	}
}

impl IntoLayer for EmbedSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Embedding {
			dimensions: self.dimensions,
			vocabulary: self.vocabulary,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttnSpec {
	heads: usize,
}

#[must_use]
pub const fn attn(heads: usize) -> AttnSpec {
	AttnSpec { heads }
}

impl IntoLayer for AttnSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Attention { heads: self.heads }
	}
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
pub const mse: Loss = Loss::MeanSquaredError;
#[allow(non_upper_case_globals)]
pub const mae: Loss = Loss::MeanAbsoluteError;
#[allow(non_upper_case_globals)]
pub const huber: Loss = Loss::Huber;
#[allow(non_upper_case_globals)]
pub const bce: Loss = Loss::BinaryCrossEntropy;
#[allow(non_upper_case_globals)]
pub const ce: Loss = Loss::CrossEntropy;
#[allow(non_upper_case_globals)]
pub const focal: Loss = Loss::Focal;

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
pub enum Normalization {
	ZScore,
}

#[allow(non_upper_case_globals)]
pub const z_score: Normalization = Normalization::ZScore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Optimizer {
	AdamW,
}

#[allow(non_upper_case_globals)]
pub const adamw: Optimizer = Optimizer::AdamW;

/// Backend-neutral model declaration. It contains no runtime handles, loaded
/// weights, allocations, or mutable global registry entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
	layers: Vec<LayerSpec>,
	objective: Option<Objective>,
	normalization: Option<Normalization>,
	optimizer: Option<Optimizer>,
	learning_rate_bits: Option<u32>,
	weights_source: Option<String>,
	input_width: Option<usize>,
	deferred: Option<DeclarationError>,
}

impl Model {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			layers: Vec::new(),
			objective: None,
			normalization: None,
			optimizer: None,
			learning_rate_bits: None,
			weights_source: None,
			input_width: None,
			deferred: None,
		}
	}

	#[must_use]
	pub fn load(weights: &str, mut prototype: Self, input_width: usize) -> Self {
		if weights.is_empty() || input_width == 0 {
			prototype.defer(
				DeclarationErrorKind::EmptyValue,
				"model weight path and input width must be nonempty",
			);
		} else {
			prototype.weights_source = Some(weights.to_owned());
			prototype.input_width = Some(input_width);
		}
		prototype
	}

	#[must_use]
	pub fn layer(mut self, spec: impl IntoLayer) -> Self {
		let spec = spec.into_layer();
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		self
	}

	#[must_use]
	pub fn conv(mut self, filters: usize, kernel: usize, stride: usize) -> Self {
		let spec = LayerSpec::Convolution {
			filters,
			kernel,
			stride,
			activation: Activation::Linear,
		};
		if let Err(error) = spec.validate() {
			self.defer(error.kind, error.detail);
		} else {
			self.layers.push(spec);
		}
		self
	}

	#[must_use]
	pub fn relu(self) -> Self {
		self.with_last_activation(Activation::Relu)
	}

	#[must_use]
	pub fn leak(self) -> Self {
		self.with_last_activation(Activation::LeakyRelu)
	}

	#[must_use]
	pub fn sigmoid(self) -> Self {
		self.with_last_activation(Activation::Sigmoid)
	}

	#[must_use]
	pub fn tanh(self) -> Self {
		self.with_last_activation(Activation::Tanh)
	}

	#[must_use]
	pub fn selu(self) -> Self {
		self.with_last_activation(Activation::Selu)
	}

	#[must_use]
	pub fn gelu(self) -> Self {
		self.with_last_activation(Activation::Gelu)
	}

	#[must_use]
	pub fn silu(self) -> Self {
		self.with_last_activation(Activation::Silu)
	}

	#[must_use]
	pub fn elu(self) -> Self {
		self.with_last_activation(Activation::Elu)
	}

	#[must_use]
	pub fn prelu(self) -> Self {
		self.with_last_activation(Activation::PRelu)
	}

	#[must_use]
	pub fn loss(mut self, objective: impl IntoObjective) -> Self {
		self.objective = Some(objective.into_objective());
		self
	}

	#[must_use]
	pub const fn norm(mut self, normalization: Normalization) -> Self {
		self.normalization = Some(normalization);
		self
	}

	#[must_use]
	pub const fn optimizer(mut self, optimizer: Optimizer) -> Self {
		self.optimizer = Some(optimizer);
		self
	}

	#[must_use]
	pub fn lr(mut self, rate: f64) -> Self {
		let rate_f32 = rate as f32;
		if !rate.is_finite() || !rate_f32.is_finite() || rate_f32 <= 0.0 {
			self.defer(
				DeclarationErrorKind::InvalidLearningRate,
				format!("learning rate must be finite, positive, and representable as f32, got {rate}"),
			);
		} else {
			self.learning_rate_bits = Some(rate_f32.to_bits());
		}
		self
	}

	pub fn validate(&self) -> DeclarationResult<()> {
		if let Some(error) = &self.deferred {
			return Err(error.clone());
		}
		if self.layers.is_empty() && self.weights_source.is_none() {
			return Err(DeclarationError::new(
				DeclarationErrorKind::InvalidLayer,
				"a model requires at least one layer or a declared weight source",
			));
		}
		for layer in &self.layers {
			layer.validate()?;
		}
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
	pub const fn objective(&self) -> Option<&Objective> {
		self.objective.as_ref()
	}

	#[must_use]
	pub const fn normalization(&self) -> Option<Normalization> {
		self.normalization
	}

	#[must_use]
	pub const fn optimizer_spec(&self) -> Option<Optimizer> {
		self.optimizer
	}

	#[must_use]
	pub fn learning_rate(&self) -> Option<f32> {
		self.learning_rate_bits.map(f32::from_bits)
	}

	#[must_use]
	pub fn weights_source(&self) -> Option<&str> {
		self.weights_source.as_deref()
	}

	#[must_use]
	pub const fn input_width(&self) -> Option<usize> {
		self.input_width
	}

	fn with_last_activation(mut self, activation: Activation) -> Self {
		match self.layers.last_mut() {
			Some(LayerSpec::Dense {
				activation: current,
				..
			})
			| Some(LayerSpec::Convolution {
				activation: current,
				..
			}) => *current = activation,
			Some(LayerSpec::Embedding { .. } | LayerSpec::Attention { .. }) | None => self.defer(
				DeclarationErrorKind::InvalidActivation,
				"activation methods require a preceding dense or convolution layer",
			),
		}
		self
	}

	fn defer(&mut self, kind: DeclarationErrorKind, detail: impl Into<String>) {
		if self.deferred.is_none() {
			self.deferred = Some(DeclarationError::new(kind, detail));
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
	Loss,
	Accuracy,
	R2,
	AuRoc,
	AuPrc,
	Brier,
	CalibrationError,
	RecallAt { threshold_bits: u64 },
	Epoch,
	LearningRate,
	Time,
	Device,
}

impl Metric {
	fn validate(self) -> DeclarationResult<()> {
		if let Self::RecallAt { threshold_bits } = self {
			let threshold = f64::from_bits(threshold_bits);
			if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
				return Err(DeclarationError::new(
					DeclarationErrorKind::InvalidMetric,
					format!("recall threshold must be finite and in [0, 1], got {threshold}"),
				));
			}
		}
		Ok(())
	}

	#[must_use]
	pub fn recall_threshold(self) -> Option<f64> {
		match self {
			Self::RecallAt { threshold_bits } => Some(f64::from_bits(threshold_bits)),
			_ => None,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogItem {
	Metric(Metric),
}

impl LogItem {
	fn validate(self) -> DeclarationResult<()> {
		match self {
			Self::Metric(metric) => metric.validate(),
		}
	}

	#[must_use]
	pub const fn metric(self) -> Metric {
		match self {
			Self::Metric(metric) => metric,
		}
	}
}

#[allow(non_upper_case_globals)]
pub const Accuracy: LogItem = LogItem::Metric(Metric::Accuracy);
#[allow(non_upper_case_globals)]
pub const R2: LogItem = LogItem::Metric(Metric::R2);
#[allow(non_upper_case_globals)]
pub const AuRoc: LogItem = LogItem::Metric(Metric::AuRoc);
#[allow(non_upper_case_globals)]
pub const AuPrc: LogItem = LogItem::Metric(Metric::AuPrc);
#[allow(non_upper_case_globals)]
pub const Brier: LogItem = LogItem::Metric(Metric::Brier);
#[allow(non_upper_case_globals)]
pub const CalibrationError: LogItem = LogItem::Metric(Metric::CalibrationError);
#[allow(non_upper_case_globals)]
pub const Epoch: LogItem = LogItem::Metric(Metric::Epoch);
#[allow(non_upper_case_globals)]
pub const Lr: LogItem = LogItem::Metric(Metric::LearningRate);
#[allow(non_upper_case_globals)]
pub const Time: LogItem = LogItem::Metric(Metric::Time);
#[allow(non_upper_case_globals)]
pub const Device: LogItem = LogItem::Metric(Metric::Device);
#[allow(non_upper_case_globals)]
pub const LossMetric: LogItem = LogItem::Metric(Metric::Loss);
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

#[allow(non_snake_case)]
#[must_use]
pub const fn RecallAt(threshold: f64) -> LogItem {
	LogItem::Metric(Metric::RecallAt {
		threshold_bits: threshold.to_bits(),
	})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LearningRateSchedule {
	CosineDecay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calibration {
	TemperatureScaling,
}

#[allow(non_upper_case_globals)]
pub const TemperatureScaling: Calibration = Calibration::TemperatureScaling;

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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Train {
	batch_size: Option<usize>,
	epochs: Option<usize>,
	learning_rate_bits: Option<u32>,
	warmup_epochs: Option<usize>,
	learning_rate_schedule: Option<LearningRateSchedule>,
	gradient_clip_bits: Option<u32>,
	early_stopping: Option<EarlyStopping>,
	calibration: Option<Calibration>,
	log_every: Option<usize>,
	log: Vec<LogItem>,
	plot: Vec<LogItem>,
	resume: Option<String>,
	nodes: Vec<String>,
	deferred: Option<DeclarationError>,
}

impl Train {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			batch_size: None,
			epochs: None,
			learning_rate_bits: None,
			warmup_epochs: None,
			learning_rate_schedule: None,
			gradient_clip_bits: None,
			early_stopping: None,
			calibration: None,
			log_every: None,
			log: Vec::new(),
			plot: Vec::new(),
			resume: None,
			nodes: Vec::new(),
			deferred: None,
		}
	}

	#[must_use]
	pub fn batch_size(mut self, batch_size: usize) -> Self {
		if batch_size == 0 {
			self.defer("training batch size must be nonzero");
		} else {
			self.batch_size = Some(batch_size);
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
			self.defer(format!(
				"training learning rate must be finite, positive, and representable as f32, got {rate}"
			));
		} else {
			self.learning_rate_bits = Some(rate_f32.to_bits());
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
	pub const fn cosine_decay(mut self) -> Self {
		self.learning_rate_schedule = Some(LearningRateSchedule::CosineDecay);
		self
	}

	#[must_use]
	pub fn gradient_clip(mut self, maximum_norm: f64) -> Self {
		let maximum_norm_f32 = maximum_norm as f32;
		if !maximum_norm.is_finite() || !maximum_norm_f32.is_finite() || maximum_norm_f32 <= 0.0 {
			self.defer(format!(
				"gradient clipping norm must be finite, positive, and representable as f32, got {maximum_norm}"
			));
		} else {
			self.gradient_clip_bits = Some(maximum_norm_f32.to_bits());
		}
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
	pub const fn calibrate(mut self, calibration: Calibration) -> Self {
		self.calibration = Some(calibration);
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
	pub fn log(mut self, items: impl IntoIterator<Item = LogItem>) -> Self {
		for item in items {
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

	#[must_use]
	pub fn net<'a>(mut self, nodes: impl IntoIterator<Item = &'a str>) -> Self {
		let mut unique = BTreeSet::new();
		for node in nodes {
			if node.is_empty() || !unique.insert(node) {
				self.defer("network node aliases must be nonempty and unique");
				break;
			}
			self.nodes.push(node.to_owned());
		}
		self
	}

	pub fn declare(&self, data: &Data, model: &Model) -> DeclarationResult<TrainingDeclaration> {
		self.validate()?;
		data.validate()?;
		model.validate()?;
		Ok(TrainingDeclaration {
			data: data.clone(),
			model: model.clone(),
			policy: self.clone(),
		})
	}

	pub fn validate(&self) -> DeclarationResult<()> {
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
	pub const fn batch_size_value(&self) -> Option<usize> {
		self.batch_size
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
	pub fn gradient_clip_value(&self) -> Option<f32> {
		self.gradient_clip_bits.map(f32::from_bits)
	}

	#[must_use]
	pub const fn early_stopping(&self) -> Option<EarlyStopping> {
		self.early_stopping
	}

	#[must_use]
	pub const fn calibration(&self) -> Option<Calibration> {
		self.calibration
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

	#[must_use]
	pub fn nodes(&self) -> &[String] {
		&self.nodes
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingDeclaration {
	data: Data,
	model: Model,
	policy: Train,
}

impl TrainingDeclaration {
	#[must_use]
	pub const fn data(&self) -> &Data {
		&self.data
	}

	#[must_use]
	pub const fn model(&self) -> &Model {
		&self.model
	}

	#[must_use]
	pub const fn policy(&self) -> &Train {
		&self.policy
	}
}

/// Static inference policy and logging declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Infer {
	log: Vec<LogItem>,
	deferred: Option<DeclarationError>,
}

impl Infer {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			log: Vec::new(),
			deferred: None,
		}
	}

	#[must_use]
	pub fn log(mut self, items: impl IntoIterator<Item = LogItem>) -> Self {
		self.log.extend(items);
		self
	}

	pub fn declare(&self, model: &Model) -> DeclarationResult<InferenceDeclaration> {
		self.validate()?;
		model.validate()?;
		Ok(InferenceDeclaration {
			model: model.clone(),
			data: None,
			policy: self.clone(),
		})
	}

	pub fn evaluate(&self, data: &Data, model: &Model) -> DeclarationResult<InferenceDeclaration> {
		self.validate()?;
		data.validate()?;
		model.validate()?;
		Ok(InferenceDeclaration {
			model: model.clone(),
			data: Some(data.clone()),
			policy: self.clone(),
		})
	}

	pub fn validate(&self) -> DeclarationResult<()> {
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
