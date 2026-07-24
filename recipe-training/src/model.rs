use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use recipe_core::{DType, IterationDomain, KernelTemplateId, MetricId, ValueId};
use recipe_ingest::{DenseMatrix, PartitionKind, PreparedDataset, VectorRole};
use recipe_language::{CalculationGraph, Shape};
use recipe_program::StaticCalculationProgram;

use crate::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseActivation {
	Linear,
	Silu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseLayer {
	width: NonZeroU64,
	activation: DenseActivation,
}

impl DenseLayer {
	#[must_use]
	pub const fn new(width: NonZeroU64, activation: DenseActivation) -> Self {
		Self { width, activation }
	}

	#[must_use]
	pub const fn width(self) -> NonZeroU64 {
		self.width
	}

	#[must_use]
	pub const fn activation(self) -> DenseActivation {
		self.activation
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DensePartition {
	features: DenseMatrix,
	targets: DenseMatrix,
}

impl DensePartition {
	pub fn new(features: DenseMatrix, targets: DenseMatrix) -> TrainingCompileResult<Self> {
		validate_partition(&features, &targets)?;
		Ok(Self { features, targets })
	}

	#[must_use]
	pub const fn features(&self) -> &DenseMatrix {
		&self.features
	}

	#[must_use]
	pub const fn targets(&self) -> &DenseMatrix {
		&self.targets
	}

	#[must_use]
	pub const fn rows(&self) -> usize {
		self.features.rows()
	}

	#[must_use]
	pub const fn feature_columns(&self) -> usize {
		self.features.columns()
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseBinaryDataset {
	train: DensePartition,
	validation: Option<DensePartition>,
}

impl DenseBinaryDataset {
	pub fn new(train: DensePartition, validation: Option<DensePartition>) -> TrainingCompileResult<Self> {
		if let Some(validation) = &validation
			&& validation.feature_columns() != train.feature_columns()
		{
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!(
					"validation has {} feature columns, training has {}",
					validation.feature_columns(),
					train.feature_columns()
				),
			));
		}
		Ok(Self { train, validation })
	}

	pub fn from_prepared(dataset: &PreparedDataset) -> TrainingCompileResult<Self> {
		let train = DensePartition::new(
			dataset.fixed_dense_matrix(VectorRole::Feature, PartitionKind::Train)?,
			dataset.fixed_dense_matrix(VectorRole::Target, PartitionKind::Train)?,
		)?;
		let validation = if dataset.validation().is_empty() {
			None
		} else {
			Some(DensePartition::new(
				dataset.fixed_dense_matrix(VectorRole::Feature, PartitionKind::Validation)?,
				dataset.fixed_dense_matrix(VectorRole::Target, PartitionKind::Validation)?,
			)?)
		};
		Self::new(train, validation)
	}

	#[must_use]
	pub const fn train(&self) -> &DensePartition {
		&self.train
	}

	#[must_use]
	pub const fn validation(&self) -> Option<&DensePartition> {
		self.validation.as_ref()
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdamWConfig {
	pub learning_rate: f32,
	pub beta_one: f32,
	pub beta_two: f32,
	pub epsilon: f32,
	pub weight_decay: f32,
}

impl Default for AdamWConfig {
	fn default() -> Self {
		Self {
			learning_rate: 1.0e-4,
			beta_one: 0.9,
			beta_two: 0.999,
			epsilon: 1.0e-8,
			weight_decay: 0.01,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct DenseTrainingConfig {
	pub layers: Vec<DenseLayer>,
	pub batch_size: NonZeroUsize,
	pub epochs: NonZeroU64,
	pub warmup_epochs: u64,
	pub gradient_clip_norm: f32,
	pub normalization_epsilon: f32,
	pub reduction_tree_lanes: u32,
	pub random_seed: u64,
	pub adamw: AdamWConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryValidationConfig {
	calibration_bins: NonZeroU32,
	recall_threshold_bits: Vec<u32>,
	early_stopping_patience: Option<NonZeroU64>,
	temperature_scaling: Option<TemperatureScalingConfig>,
}

impl BinaryValidationConfig {
	#[must_use]
	pub fn new(calibration_bins: NonZeroU32, recall_thresholds: impl IntoIterator<Item = f32>) -> Self {
		Self {
			calibration_bins,
			recall_threshold_bits: recall_thresholds.into_iter().map(f32::to_bits).collect(),
			early_stopping_patience: None,
			temperature_scaling: None,
		}
	}

	#[must_use]
	pub fn with_auprc_early_stopping(mut self, patience: NonZeroU64) -> Self {
		self.early_stopping_patience = Some(patience);
		self
	}

	#[must_use]
	pub fn with_temperature_scaling(mut self, config: TemperatureScalingConfig) -> Self {
		self.temperature_scaling = Some(config);
		self
	}

	#[must_use]
	pub const fn calibration_bins(&self) -> NonZeroU32 {
		self.calibration_bins
	}

	#[must_use]
	pub fn recall_thresholds(&self) -> impl ExactSizeIterator<Item = f32> + '_ {
		self.recall_threshold_bits
			.iter()
			.copied()
			.map(f32::from_bits)
	}

	#[must_use]
	pub const fn early_stopping_patience(&self) -> Option<NonZeroU64> {
		self.early_stopping_patience
	}

	#[must_use]
	pub const fn temperature_scaling(&self) -> Option<TemperatureScalingConfig> {
		self.temperature_scaling
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemperatureScalingConfig {
	pub iterations: NonZeroU64,
	pub learning_rate: f32,
	pub minimum_temperature: f32,
	pub maximum_temperature: f32,
}

impl Default for TemperatureScalingConfig {
	fn default() -> Self {
		Self {
			iterations: NonZeroU64::new(64).expect("64 is nonzero"),
			learning_rate: 0.01,
			minimum_temperature: 0.05,
			maximum_temperature: 20.0,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingBounds {
	pub train_rows: u64,
	pub batch_size: u64,
	pub batches_per_epoch: u64,
	pub padded_rows_per_epoch: u64,
	pub epochs: NonZeroU64,
	pub training_iterations: NonZeroU64,
	pub calibration_iterations: u64,
	pub iterations: NonZeroU64,
	pub warmup_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedExternalInput {
	role: ExternalInputRole,
	value: ValueId,
	dtype: DType,
	shape: Shape,
	bytes: Vec<u8>,
}

impl OwnedExternalInput {
	#[must_use]
	pub const fn role(&self) -> ExternalInputRole {
		self.role
	}

	#[must_use]
	pub const fn value(&self) -> ValueId {
		self.value
	}

	#[must_use]
	pub const fn dtype(&self) -> DType {
		self.dtype
	}

	#[must_use]
	pub const fn shape(&self) -> &Shape {
		&self.shape
	}

	#[must_use]
	pub fn bytes(&self) -> &[u8] {
		&self.bytes
	}

	pub(crate) fn new(role: ExternalInputRole, value: ValueId, dtype: DType, shape: Shape, bytes: Vec<u8>) -> Self {
		Self {
			role,
			value,
			dtype,
			shape,
			bytes,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalInputRole {
	TrainFeatures,
	TrainTargets,
	ValidationFeatures,
	ValidationTargets,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterState {
	pub initial_parameter: ValueId,
	pub updated_parameter: ValueId,
	pub initial_first_moment: ValueId,
	pub updated_first_moment: ValueId,
	pub initial_second_moment: ValueId,
	pub updated_second_moment: ValueId,
	pub update_kernel: KernelTemplateId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseLayerState {
	pub weight: ParameterState,
	pub bias: ParameterState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZScoreState {
	pub mean: ValueId,
	pub variance: ValueId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingOutputs {
	pub batch_loss: ValueId,
	pub batch_loss_domain: IterationDomain,
	pub normalization: ZScoreState,
	pub layers: Vec<DenseLayerState>,
	pub validation: Option<BinaryValidationOutputs>,
	pub metric_bindings: Vec<TrainingMetricBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingMetricKind {
	BatchLoss,
	ValidationMeanBce,
	AuRoc,
	AuPrc,
	BrierScore,
	ExpectedCalibrationError,
	RecallAt { threshold_bits: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingMetricBinding {
	pub kind: TrainingMetricKind,
	pub metric: MetricId,
	pub value: ValueId,
	pub domain: IterationDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecallMetricOutput {
	pub threshold_bits: u32,
	pub value: ValueId,
}

impl RecallMetricOutput {
	#[must_use]
	pub const fn threshold(self) -> f32 {
		f32::from_bits(self.threshold_bits)
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryMetricOutputs {
	pub mean_bce: ValueId,
	pub auroc: ValueId,
	pub auprc: ValueId,
	pub brier_score: ValueId,
	pub expected_calibration_error: ValueId,
	pub recall_at: Vec<RecallMetricOutput>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EarlyStoppingState {
	pub initial_best_auprc: ValueId,
	pub updated_best_auprc: ValueId,
	pub initial_stale_epochs: ValueId,
	pub updated_stale_epochs: ValueId,
	pub initial_stopped: ValueId,
	pub updated_stopped: ValueId,
	pub update_kernel: KernelTemplateId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemperatureScalingState {
	pub initial_temperature: ValueId,
	pub updated_temperature: ValueId,
	pub update_kernel: KernelTemplateId,
	pub iterations: NonZeroU64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryValidationOutputs {
	pub logits: ValueId,
	pub metrics: BinaryMetricOutputs,
	pub metric_domain: IterationDomain,
	pub early_stopping: Option<EarlyStoppingState>,
	pub temperature_scaling: Option<TemperatureScalingState>,
}

#[derive(Clone, Debug)]
pub struct CompiledTraining {
	pub(crate) program: StaticCalculationProgram,
	pub(crate) external_inputs: Vec<OwnedExternalInput>,
	pub(crate) bounds: TrainingBounds,
	pub(crate) outputs: TrainingOutputs,
}

impl CompiledTraining {
	#[must_use]
	pub const fn graph(&self) -> &CalculationGraph {
		self.program.graph()
	}

	#[must_use]
	pub const fn program(&self) -> &StaticCalculationProgram {
		&self.program
	}

	#[must_use]
	pub fn external_inputs(&self) -> &[OwnedExternalInput] {
		&self.external_inputs
	}

	#[must_use]
	pub const fn bounds(&self) -> TrainingBounds {
		self.bounds
	}

	#[must_use]
	pub const fn outputs(&self) -> &TrainingOutputs {
		&self.outputs
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedTrainingFeature {
	DynamicLoopShortening,
}

pub const REMAINING_UNSUPPORTED: &[UnsupportedTrainingFeature] = &[UnsupportedTrainingFeature::DynamicLoopShortening];

fn validate_partition(features: &DenseMatrix, targets: &DenseMatrix) -> TrainingCompileResult<()> {
	if features.rows() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::EmptyDataset,
			"dense training partition has no rows",
		));
	}
	if features.columns() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			"dense feature matrix has no columns",
		));
	}
	if features.rows() != targets.rows() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InconsistentRows,
			format!(
				"feature matrix has {} rows, target matrix has {}",
				features.rows(),
				targets.rows()
			),
		));
	}
	if targets.columns() != 1 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!(
				"binary classification requires exactly one target column, got {}",
				targets.columns()
			),
		));
	}
	validate_matrix_storage(features, "feature")?;
	validate_matrix_storage(targets, "target")?;
	validate_binary_targets(targets)
}

fn validate_matrix_storage(matrix: &DenseMatrix, role: &str) -> TrainingCompileResult<()> {
	let expected = matrix.rows().checked_mul(matrix.columns()).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			format!("{role} element count overflowed usize"),
		)
	})?;
	let actual = match matrix {
		DenseMatrix::I32 { values, .. } => values.len(),
		DenseMatrix::F32Bits { values, .. } => {
			if let Some(bits) = values
				.iter()
				.copied()
				.find(|bits| !f32::from_bits(*bits).is_finite())
			{
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidFeatureMatrix,
					format!("{role} matrix contains non-finite f32 bits {bits:#010x}"),
				));
			}
			values.len()
		}
	};
	if actual != expected {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			format!("{role} matrix stores {actual} values, expected {expected}"),
		));
	}
	Ok(())
}

fn validate_binary_targets(targets: &DenseMatrix) -> TrainingCompileResult<()> {
	let invalid = match targets {
		DenseMatrix::I32 { values, .. } => values
			.iter()
			.copied()
			.find(|value| !matches!(value, 0 | 1))
			.map(|value| value.to_string()),
		DenseMatrix::F32Bits { values, .. } => values
			.iter()
			.copied()
			.map(f32::from_bits)
			.find(|value| *value != 0.0 && *value != 1.0)
			.map(|value| value.to_string()),
	};
	if let Some(value) = invalid {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!("binary target matrix contains {value}; only exact zero and one are accepted"),
		));
	}
	Ok(())
}
