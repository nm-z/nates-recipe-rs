use core::{
	fmt,
	num::{NonZeroU32, NonZeroU64},
};

use recipe_core::{Block, DType, IterationDomain, KernelTemplateId, LoopIterations, MetricId, ValueId};
use recipe_ingest::{DenseMatrix, PartitionKind, PreparedDataset, PreparedValues, SemanticType, VectorEncoding, VectorMetadata, VectorRole, VectorSchema};
use recipe_language::{CalculationGraph, Shape};
use recipe_program::StaticCalculationProgram;

use crate::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};

pub const MAXIMUM_REDUCTION_TREE_LANES: u32 = 1_024;

pub const fn exact_i32_as_f32(value: i32) -> Option<f32> {
	const SIGN_MASK: u32 = 1u32 << (u32::BITS - 1);
	const EXPONENT_BIAS: u32 = 127;
	const FRACTION_BITS: u32 = f32::MANTISSA_DIGITS - 1;

	if value == 0 {
		return Some(0.0);
	}
	let magnitude = value.unsigned_abs();
	let exponent = (u32::BITS - 1) - magnitude.leading_zeros();
	let discarded_bits = exponent.saturating_sub(FRACTION_BITS);
	if discarded_bits != 0 && magnitude & ((1u32 << discarded_bits) - 1) != 0 {
		return None;
	}
	let significand = if exponent <= FRACTION_BITS {
		magnitude << (FRACTION_BITS - exponent)
	} else {
		magnitude >> discarded_bits
	};
	let sign = if value.is_negative() { SIGN_MASK } else { 0 };
	let exponent_bits = (exponent + EXPONENT_BIAS) << FRACTION_BITS;
	let fraction_mask = (1u32 << FRACTION_BITS) - 1;
	return Some(f32::from_bits(
		sign | exponent_bits | (significand & fraction_mask),
	));
}

pub const fn f32_from_u64(value: u64) -> f32 {
	const FRACTION_BITS: u32 = f32::MANTISSA_DIGITS - 1;
	const EXPONENT_BIAS: u32 = 127;
	if value == 0 {
		return 0.0;
	}
	let mut exponent = (u64::BITS - 1) - value.leading_zeros();
	let significand = if exponent <= FRACTION_BITS {
		value << (FRACTION_BITS - exponent)
	} else {
		let shift = exponent - FRACTION_BITS;
		let mut significand = value >> shift;
		let remainder = value & ((1u64 << shift) - 1);
		let halfway = 1u64 << (shift - 1);
		if remainder > halfway || (remainder == halfway && significand & 1 != 0) {
			significand += 1;
			if significand == 1u64 << f32::MANTISSA_DIGITS {
				significand >>= 1;
				exponent += 1;
			}
		}
		significand
	};
	let bytes = significand.to_le_bytes();
	let significand = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
	let bits = ((exponent + EXPONENT_BIAS) << FRACTION_BITS) | (significand & ((1u32 << FRACTION_BITS) - 1));
	return f32::from_bits(bits);
}

fn is_binary_target(value: f32) -> bool { return value.classify() == core::num::FpCategory::Zero || value.to_bits() == 1.0f32.to_bits(); }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingHorizon {
	Finite(NonZeroU64),
	Unbounded,
}

impl TrainingHorizon {
	#[must_use]
	#[inline]
	pub const fn finite(epochs: NonZeroU64) -> Self { return Self::Finite(epochs); }

	#[must_use]
	#[inline]
	pub const fn unbounded() -> Self { return Self::Unbounded; }

	#[must_use]
	#[inline]
	pub const fn bound(self) -> Option<NonZeroU64> {
		match self {
			Self::Finite(epochs) => return Some(epochs),
			Self::Unbounded => return None,
		}
	}

	#[must_use]
	#[inline]
	pub const fn is_unbounded(self) -> bool { return matches!(self, Self::Unbounded); }

	#[must_use]
	#[inline]
	pub const fn loop_iterations(self) -> LoopIterations {
		match self {
			Self::Finite(epochs) => return LoopIterations::Finite(epochs),
			Self::Unbounded => return LoopIterations::Unbounded,
		}
	}
}

impl From<NonZeroU64> for TrainingHorizon {
	#[inline]
	fn from(epochs: NonZeroU64) -> Self { return Self::Finite(epochs); }
}

impl fmt::Display for TrainingHorizon {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			Self::Finite(epochs) => return epochs.fmt(f),
			Self::Unbounded => return f.write_str("unbounded"),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseConvolutionGeometry {
	input_length: NonZeroU64,
	input_channels: NonZeroU64,
	output_length: NonZeroU64,
	filters: NonZeroU64,
	kernel: NonZeroU64,
}

impl DenseConvolutionGeometry {
	#[must_use]
	#[inline]
	pub const fn new(input_length: NonZeroU64, input_channels: NonZeroU64, output_length: NonZeroU64, filters: NonZeroU64, kernel: NonZeroU64) -> Self {
		return Self {
			input_length,
			input_channels,
			output_length,
			filters,
			kernel,
		};
	}

	#[must_use]
	#[inline]
	pub const fn input_length(self) -> NonZeroU64 { return self.input_length; }

	#[must_use]
	#[inline]
	pub const fn input_channels(self) -> NonZeroU64 { return self.input_channels; }

	#[must_use]
	#[inline]
	pub const fn output_length(self) -> NonZeroU64 { return self.output_length; }

	#[must_use]
	#[inline]
	pub const fn filters(self) -> NonZeroU64 { return self.filters; }

	#[must_use]
	#[inline]
	pub const fn kernel(self) -> NonZeroU64 { return self.kernel; }

	#[must_use]
	#[inline]
	pub fn input_width(self) -> Option<NonZeroU64> {
		return self
			.input_length
			.get()
			.checked_mul(self.input_channels.get())
			.and_then(NonZeroU64::new);
	}

	#[must_use]
	#[inline]
	pub fn output_width(self) -> Option<NonZeroU64> {
		return self
			.output_length
			.get()
			.checked_mul(self.filters.get())
			.and_then(NonZeroU64::new);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseTask {
	BinaryClassification {
		target_vector: usize,
		positive_code: i32,
	},
	MulticlassClassification {
		target_vector: usize,
		class_count: usize,
		reserved_code: i32,
	},
	ScalarRegression {
		target_vector: usize,
	},
	MultiTargetBinaryClassification {
		first_target_vector: usize,
		target_count: usize,
	},
	JointMulticlassClassification {
		first_target_vector: usize,
		target_count: usize,
	},
	MultiTargetRegression {
		first_target_vector: usize,
		target_count: usize,
	},
}

impl DenseTask {
	#[must_use]
	#[inline]
	pub const fn target_vector(self) -> usize {
		match self {
			Self::BinaryClassification { target_vector, .. } | Self::MulticlassClassification { target_vector, .. } | Self::ScalarRegression { target_vector } => return target_vector,
			Self::MultiTargetBinaryClassification {
				first_target_vector,
				..
			}
			| Self::JointMulticlassClassification {
				first_target_vector,
				..
			}
			| Self::MultiTargetRegression {
				first_target_vector,
				..
			} => return first_target_vector,
		}
	}

	#[must_use]
	#[inline]
	pub const fn target_count(self) -> usize {
		match self {
			Self::BinaryClassification { .. } | Self::MulticlassClassification { .. } | Self::ScalarRegression { .. } => return 1,
			Self::MultiTargetBinaryClassification { target_count, .. } | Self::JointMulticlassClassification { target_count, .. } | Self::MultiTargetRegression { target_count, .. } => return target_count,
		}
	}

	#[must_use]
	#[inline]
	pub const fn uses_target_matrix(self) -> bool {
		return matches!(
			self,
			Self::MultiTargetBinaryClassification { .. } | Self::JointMulticlassClassification { .. } | Self::MultiTargetRegression { .. }
		);
	}

	#[must_use]
	#[inline]
	pub const fn output_width(self) -> usize {
		match self {
			Self::BinaryClassification { .. } | Self::ScalarRegression { .. } => return 1,
			Self::MulticlassClassification { class_count, .. } => return class_count,
			Self::MultiTargetBinaryClassification { target_count, .. } | Self::JointMulticlassClassification { target_count, .. } | Self::MultiTargetRegression { target_count, .. } => return target_count,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseOutputAdapter {
	source_width: NonZeroU64,
	target_width: NonZeroU64,
}

impl DenseOutputAdapter {
	pub(crate) const fn new(source_width: NonZeroU64, target_width: NonZeroU64) -> Self {
		return Self {
			source_width,
			target_width,
		};
	}

	#[must_use]
	#[inline]
	pub const fn source_width(self) -> NonZeroU64 { return self.source_width; }

	#[must_use]
	#[inline]
	pub const fn target_width(self) -> NonZeroU64 { return self.target_width; }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseFeatureLowering {
	NumericScalar,
	CategoricalOneHot {
		dictionary_width: usize,
		reserved_index: usize,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledFeatureSpan {
	source_vector: usize,
	start: usize,
	width: usize,
	lowering: DenseFeatureLowering,
}

impl CompiledFeatureSpan {
	pub(crate) const fn new(source_vector: usize, start: usize, width: usize, lowering: DenseFeatureLowering) -> Self {
		return Self {
			source_vector,
			start,
			width,
			lowering,
		};
	}

	#[must_use]
	#[inline]
	pub const fn source_vector(&self) -> usize { return self.source_vector; }

	#[must_use]
	#[inline]
	pub const fn start(&self) -> usize { return self.start; }

	#[must_use]
	#[inline]
	pub const fn width(&self) -> usize { return self.width; }

	#[must_use]
	#[inline]
	pub const fn lowering(&self) -> DenseFeatureLowering { return self.lowering; }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledDatasetSchema {
	vectors: Vec<VectorSchema>,
	features: Vec<CompiledFeatureSpan>,
	targets: Vec<usize>,
	target_dtypes: Vec<DType>,
	input_width: usize,
	task: DenseTask,
}

impl CompiledDatasetSchema {
	pub(crate) fn from_prepared(dataset: &PreparedDataset, task: DenseTask, features: Vec<CompiledFeatureSpan>) -> TrainingCompileResult<Self> {
		let vectors = dataset
			.vectors()
			.iter()
			.map(|vector| return vector.schema())
			.collect::<Vec<_>>();
		let targets = dataset.target_source_indices().to_vec();
		if targets.len() != task.target_count() || targets.first().copied() != Some(task.target_vector()) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!(
					"declared target order {targets:?} disagrees with task primary target {} and count {}",
					task.target_vector(),
					task.target_count()
				),
			));
		}
		let target_dtypes = targets
			.iter()
			.copied()
			.map(|target| {
				let target_schema = vectors
					.iter()
					.find(|vector| {
						return vector.source_index() == target && vector.role() == VectorRole::Target;
					})
					.ok_or_else(|| {
						return TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidTargetMatrix,
							format!("dense target vector {target} is absent from the compiled schema"),
						);
					})?;
				return target_schema.encoding().dtype().ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						format!(
							"dense target vector {target} uses variable-width {:?} storage without a typed target lowering",
							target_schema.encoding()
						),
					);
				});
			})
			.collect::<TrainingCompileResult<Vec<_>>>()?;
		let input_width = features.iter().map(|feature| return feature.width).sum();
		return Ok(Self {
			vectors,
			features,
			targets,
			target_dtypes,
			input_width,
			task,
		});
	}

	#[must_use]
	#[inline]
	pub fn vectors(&self) -> &[VectorSchema] { return &self.vectors; }

	#[must_use]
	#[inline]
	pub fn features(&self) -> &[CompiledFeatureSpan] { return &self.features; }

	#[must_use]
	#[inline]
	pub fn target(&self) -> usize { return self.targets[0]; }

	#[must_use]
	#[inline]
	pub fn targets(&self) -> &[usize] { return &self.targets; }

	#[must_use]
	#[inline]
	pub fn target_dtype(&self) -> DType { return self.target_dtypes[0]; }

	#[must_use]
	#[inline]
	pub fn target_dtypes(&self) -> &[DType] { return &self.target_dtypes; }

	#[must_use]
	#[inline]
	pub const fn input_width(&self) -> usize { return self.input_width; }

	#[must_use]
	#[inline]
	pub const fn task(&self) -> DenseTask { return self.task; }

	#[must_use]
	#[inline]
	pub const fn output_width(&self) -> usize { return self.task.output_width(); }

	#[must_use]
	#[inline]
	pub fn decode_multiclass_class(&self, class: usize) -> Option<DecodedMulticlassClass<'_>> {
		let DenseTask::MulticlassClassification {
			target_vector,
			class_count,
			reserved_code,
		} = self.task
		else {
			return None;
		};
		if class >= class_count {
			return None;
		}
		let target = self.vectors.iter().find(|vector| {
			return vector.source_index() == target_vector && vector.role() == VectorRole::Target;
		})?;
		let dictionary = target.metadata().categorical_dictionary()?;
		if i32::try_from(class).ok() == Some(reserved_code) {
			return Some(DecodedMulticlassClass::ReservedUnseen);
		}
		return dictionary
			.get(class)
			.map(|label| return DecodedMulticlassClass::Label(label));
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedMulticlassClass<'a> {
	Label(&'a [u8]),
	ReservedUnseen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseFeaturePlan {
	spans: Vec<CompiledFeatureSpan>,
	input_width: usize,
	normalization_mask: Option<Vec<u32>>,
}

impl DenseFeaturePlan {
	pub(crate) fn from_prepared(dataset: &PreparedDataset) -> TrainingCompileResult<Self> {
		let mut spans = Vec::new();
		let mut normalization_mask = Vec::new();
		let mut input_width = 0usize;
		let mut has_categorical = false;
		for feature in dataset
			.vectors()
			.iter()
			.filter(|vector| return vector.role() == VectorRole::Feature)
		{
			let (width, lowering, normalize) = match (
				feature.semantic_type(),
				feature.encoding(),
				feature.metadata(),
				feature.values(),
			) {
				(SemanticType::Numeric, VectorEncoding::I32, VectorMetadata::None, PreparedValues::I32(_)) | (SemanticType::Numeric, VectorEncoding::F32, VectorMetadata::None, PreparedValues::F32Bits(_)) => (1, DenseFeatureLowering::NumericScalar, true),
				(SemanticType::Categorical, VectorEncoding::DictionaryI32, VectorMetadata::Categorical { dictionary }, PreparedValues::I32(_)) => {
					let mut labels = alloc::collections::BTreeSet::new();
					if dictionary
						.iter()
						.any(|label| return label.is_empty() || !labels.insert(label))
					{
						return Err(TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidFeatureMatrix,
							format!(
								"categorical feature {:?} has an empty or duplicate dictionary label",
								String::from_utf8_lossy(feature.name())
							),
						));
					}
					has_categorical = true;
					let reserved_index = dictionary.len();
					(
						reserved_index.checked_add(1).ok_or_else(|| {
							return TrainingCompileError::new(
								TrainingCompileErrorKind::ArithmeticOverflow,
								"categorical one-hot width overflowed usize",
							);
						})?,
						DenseFeatureLowering::CategoricalOneHot {
							dictionary_width: dictionary.len(),
							reserved_index,
						},
						false,
					)
				}
				_ => {
					return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidFeatureMatrix,
						format!(
							"feature {:?} classified {:?}/{:?} does not yet have a dedicated semantic dense lowering",
							String::from_utf8_lossy(feature.name()),
							feature.semantic_type(),
							feature.encoding()
						),
					));
				}
			};
			let start = input_width;
			input_width = input_width.checked_add(width).ok_or_else(|| {
				return TrainingCompileError::new(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"lowered dense feature width overflowed usize",
				);
			})?;
			spans.push(CompiledFeatureSpan::new(
				feature.source_index(),
				start,
				width,
				lowering,
			));
			normalization_mask.resize(
				input_width,
				if normalize { 1.0f32 } else { 0.0f32 }.to_bits(),
			);
		}
		if spans.is_empty() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				"dense feature lowering requires at least one feature vector",
			));
		}
		return Ok(Self {
			spans,
			input_width,
			normalization_mask: has_categorical.then_some(normalization_mask),
		});
	}

	#[must_use]
	pub(crate) fn spans(&self) -> &[CompiledFeatureSpan] { return &self.spans; }

	#[must_use]
	pub(crate) fn normalization_mask(&self) -> Option<&[u32]> { return self.normalization_mask.as_deref(); }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DensePartition {
	features: DenseMatrix,
	targets: DenseMatrix,
	target_observations: Vec<TargetObservation>,
}

impl DensePartition {
	fn new(features: DenseMatrix, targets: DenseMatrix, target_observations: Vec<TargetObservation>) -> TrainingCompileResult<Self> {
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
		if targets.columns() == 0 {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"dense training partition has no target columns",
			));
		}
		validate_matrix_storage(&features, "feature")?;
		validate_matrix_storage(&targets, "target")?;
		if target_observations.len() != targets.rows() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InconsistentRows,
				format!(
					"target matrix has {} rows but target observation state has {} rows",
					targets.rows(),
					target_observations.len()
				),
			));
		}
		return Ok(Self {
			features,
			targets,
			target_observations,
		});
	}

	#[must_use]
	pub const fn features(&self) -> &DenseMatrix { return &self.features; }

	#[must_use]
	pub const fn targets(&self) -> &DenseMatrix { return &self.targets; }

	#[must_use]
	pub const fn rows(&self) -> usize { return self.features.rows(); }

	#[must_use]
	pub const fn feature_columns(&self) -> usize { return self.features.columns(); }

	#[must_use]
	pub fn all_targets_known(&self) -> bool {
		return self
			.target_observations
			.iter()
			.all(|observation| return *observation == TargetObservation::Known);
	}

	pub(crate) fn accepted_updates_per_epoch(&self) -> usize { return usize::from(self.target_observations.contains(&TargetObservation::Known)); }

	pub(crate) fn target_supervision(&self) -> DenseMatrix {
		return DenseMatrix::F32Bits {
			rows: self.rows(),
			columns: 1,
			values: self
				.target_observations
				.iter()
				.map(|observation| {
					match *observation {
						TargetObservation::Known => return 1.0f32.to_bits(),
						TargetObservation::Missing | TargetObservation::Unseen => return 0.0f32.to_bits(),
					}
				})
				.collect(),
		};
	}

	fn known_only(self) -> TrainingCompileResult<Option<Self>> {
		if self.all_targets_known() {
			return Ok(Some(self));
		}
		let retained_rows = self
			.target_observations
			.iter()
			.enumerate()
			.filter_map(|(row, observation)| return (*observation == TargetObservation::Known).then_some(row))
			.collect::<Vec<_>>();
		if retained_rows.is_empty() {
			return Ok(None);
		}
		let features = compact_dense_rows(&self.features, &retained_rows, "validation feature")?;
		let targets = compact_dense_rows(&self.targets, &retained_rows, "validation target")?;
		return Self::new(features, targets, vec![
			TargetObservation::Known;
			retained_rows.len()
		])
		.map(Some);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetObservation {
	Known,
	Missing,
	Unseen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoweredDenseTargets {
	matrix: DenseMatrix,
	observations: Vec<TargetObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredDenseDataset {
	train: DensePartition,
	validation: Option<DensePartition>,
	validation_split_rows: usize,
}

impl LoweredDenseDataset {
	pub(crate) fn from_prepared(dataset: &PreparedDataset, feature_plan: &DenseFeaturePlan, task: DenseTask) -> TrainingCompileResult<Self> {
		let train_targets = lower_dense_targets(dataset, task, PartitionKind::Train)?;
		let train = DensePartition::new(
			lower_dense_features(dataset, feature_plan, PartitionKind::Train)?,
			train_targets.matrix,
			train_targets.observations,
		)?;
		let validation_split_rows = dataset.validation().len();
		let validation = if dataset.validation().is_empty() {
			None
		} else {
			let validation_targets = lower_dense_targets(dataset, task, PartitionKind::Validation)?;
			DensePartition::new(
				lower_dense_features(dataset, feature_plan, PartitionKind::Validation)?,
				validation_targets.matrix,
				validation_targets.observations,
			)?
			.known_only()?
		};
		if let Some(validation) = validation.as_ref()
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
		return Ok(Self {
			train,
			validation,
			validation_split_rows,
		});
	}

	#[must_use]
	pub const fn train(&self) -> &DensePartition { return &self.train; }

	#[must_use]
	pub const fn validation(&self) -> Option<&DensePartition> { return self.validation.as_ref(); }

	#[must_use]
	pub const fn validation_split_rows(&self) -> usize { return self.validation_split_rows; }
}

fn lower_dense_targets(dataset: &PreparedDataset, task: DenseTask, partition_kind: PartitionKind) -> TrainingCompileResult<LoweredDenseTargets> {
	if task.uses_target_matrix() {
		return lower_multi_dense_targets(dataset, task, partition_kind);
	}
	let target_vector = task.target_vector();
	let target = dataset
		.vectors()
		.iter()
		.find(|vector| return vector.source_index() == target_vector && vector.role() == VectorRole::Target)
		.ok_or_else(|| {
			return TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("dense target vector {target_vector} is absent"),
			);
		})?;
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	match target.values() {
		PreparedValues::I32(values) => {
			let mut lowered = Vec::with_capacity(partition.len());
			let mut observations = Vec::with_capacity(partition.len());
			for (retained_position, source_row) in partition
				.retained_positions()
				.iter()
				.copied()
				.zip(partition.source_rows().iter().copied())
			{
				let value = values.get(retained_position).ok_or_else(|| {
					return target_lowering_error(target.name(), source_row, "target value is absent");
				})?;
				let (value, observation) = lower_i32_target(target, task, *value, source_row)?;
				lowered.push(value);
				observations.push(observation);
			}
			return Ok(LoweredDenseTargets {
				matrix: DenseMatrix::I32 {
					rows: partition.len(),
					columns: 1,
					values: lowered,
				},
				observations,
			});
		}
		PreparedValues::F32Bits(values) => {
			if matches!(task, DenseTask::MulticlassClassification { .. }) {
				return Err(target_lowering_error(
					target.name(),
					0,
					"dictionary target does not retain int32 category codes",
				));
			}
			let mut lowered = Vec::with_capacity(partition.len());
			let mut observations = Vec::with_capacity(partition.len());
			for (retained_position, source_row) in partition
				.retained_positions()
				.iter()
				.copied()
				.zip(partition.source_rows().iter().copied())
			{
				let value = values.get(retained_position).ok_or_else(|| {
					return target_lowering_error(target.name(), source_row, "target value is absent");
				})?;
				let (bits, observation) = match *value {
					Some(bits) => {
						let value = f32::from_bits(bits);
						if !value.is_finite() {
							return Err(target_lowering_error(
								target.name(),
								source_row,
								format!("target contains non-finite f32 bits {bits:#010x}"),
							));
						}
						if matches!(task, DenseTask::BinaryClassification { .. }) && !is_binary_target(value) {
							return Err(target_lowering_error(
								target.name(),
								source_row,
								format!("binary target matrix contains {value}; only exact zero and one are accepted"),
							));
						}
						(bits, TargetObservation::Known)
					}
					None => (0.0f32.to_bits(), TargetObservation::Missing),
				};
				lowered.push(bits);
				observations.push(observation);
			}
			return Ok(LoweredDenseTargets {
				matrix: DenseMatrix::F32Bits {
					rows: partition.len(),
					columns: 1,
					values: lowered,
				},
				observations,
			});
		}
		PreparedValues::VariableWidth(_) => {
			return Err(target_lowering_error(
				target.name(),
				0,
				"dense target cannot use variable-width storage",
			));
		}
	}
}

fn lower_multi_dense_targets(dataset: &PreparedDataset, task: DenseTask, partition_kind: PartitionKind) -> TrainingCompileResult<LoweredDenseTargets> {
	let target_count = task.target_count();
	if target_count < 2 || dataset.target_source_indices().len() != target_count {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!(
				"multi-target task requires at least two declared targets and exact order width {target_count}, found {:?}",
				dataset.target_source_indices()
			),
		));
	}
	let targets = dataset
		.target_source_indices()
		.iter()
		.copied()
		.map(|source_index| {
			return dataset
				.vectors()
				.iter()
				.find(|vector| {
					return vector.source_index() == source_index && vector.role() == VectorRole::Target;
				})
				.ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						format!("declared target vector {source_index} is absent"),
					);
				});
		})
		.collect::<TrainingCompileResult<Vec<_>>>()?;
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	let capacity = partition.len().checked_mul(target_count).ok_or_else(|| {
		return TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"multi-target matrix element count overflowed usize",
		);
	})?;
	let mut lowered = Vec::with_capacity(capacity);
	let mut observations = Vec::with_capacity(partition.len());
	for (retained_position, source_row) in partition
		.retained_positions()
		.iter()
		.copied()
		.zip(partition.source_rows().iter().copied())
	{
		let row_start = lowered.len();
		let mut row_known = true;
		for target in &targets {
			let value = match target.values() {
				PreparedValues::I32(values) => {
					let value = values.get(retained_position).ok_or_else(|| {
						return target_lowering_error(target.name(), source_row, "target value is absent");
					})?;
					match *value {
						Some(value) => {
							let Some(converted) = exact_i32_as_f32(value) else {
								return Err(target_lowering_error(
									target.name(),
									source_row,
									format!("int32 target {value} is not exactly representable as binary32"),
								));
							};
							Some(converted)
						}
						None => None,
					}
				}
				PreparedValues::F32Bits(values) => {
					let bits = values.get(retained_position).ok_or_else(|| {
						return target_lowering_error(target.name(), source_row, "target value is absent");
					})?;
					bits.map(f32::from_bits)
				}
				PreparedValues::VariableWidth(_) => {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						"multi-target dense objectives require fixed numeric storage",
					));
				}
			};
			match value {
				Some(value) if value.is_finite() => lowered.push(value.to_bits()),
				Some(value) => {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						format!("target contains non-finite value {value}"),
					));
				}
				None => {
					row_known = false;
					lowered.push(0.0f32.to_bits());
				}
			}
		}
		let row = &mut lowered[row_start..];
		if !row_known {
			row.fill(0.0f32.to_bits());
			observations.push(TargetObservation::Missing);
			continue;
		}
		match task {
			DenseTask::MultiTargetBinaryClassification { .. } => {
				if let Some((column, value)) = row
					.iter()
					.copied()
					.map(f32::from_bits)
					.enumerate()
					.find(|entry| return !is_binary_target(entry.1))
				{
					return Err(target_lowering_error(
						targets[column].name(),
						source_row,
						format!("binary target matrix contains {value}; only exact zero and one are accepted"),
					));
				}
			}
			DenseTask::JointMulticlassClassification { .. } => {
				let mut ones = 0usize;
				for (column, value) in row.iter().copied().map(f32::from_bits).enumerate() {
					if value.to_bits() == 1.0f32.to_bits() {
						ones += 1;
						continue;
					}
					if value.classify() != core::num::FpCategory::Zero {
						return Err(target_lowering_error(
							targets[column].name(),
							source_row,
							format!("joint cross-entropy target contains {value}; rows must be exact one-hot vectors"),
						));
					}
				}
				if ones != 1 {
					return Err(target_lowering_error(
						targets[0].name(),
						source_row,
						format!("joint cross-entropy row has {ones} active targets; exactly one is required"),
					));
				}
			}
			DenseTask::MultiTargetRegression { .. } => {}
			DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. } | DenseTask::ScalarRegression { .. } => {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					"singular task reached multi-target lowering",
				));
			}
		}
		observations.push(TargetObservation::Known);
	}
	return Ok(LoweredDenseTargets {
		matrix: DenseMatrix::F32Bits {
			rows: partition.len(),
			columns: target_count,
			values: lowered,
		},
		observations,
	});
}

fn lower_i32_target(target: &recipe_ingest::PreparedVector, task: DenseTask, value: Option<i32>, source_row: usize) -> TrainingCompileResult<(i32, TargetObservation)> {
	let Some(value) = value else {
		return Ok((0, TargetObservation::Missing));
	};
	match task {
		DenseTask::ScalarRegression { .. } => return Ok((value, TargetObservation::Known)),
		DenseTask::BinaryClassification { positive_code, .. } => {
			if let Some(dictionary) = target.metadata().categorical_dictionary() {
				let expected_positive_code = match dictionary.len() {
					0 => -1,
					1 => 0,
					2 => 1,
					count => {
						return Err(target_lowering_error(
							target.name(),
							source_row,
							format!("categorical BCE target has unsupported label count {count}"),
						));
					}
				};
				if positive_code != expected_positive_code {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						format!("categorical BCE positive code {positive_code} disagrees with expected binding {expected_positive_code}"),
					));
				}
				let category = usize::try_from(value).map_err(|_error| {
					return target_lowering_error(
						target.name(),
						source_row,
						format!("negative category code {value}"),
					);
				})?;
				if category == dictionary.len() {
					return Ok((0, TargetObservation::Unseen));
				}
				if category > dictionary.len() {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						format!(
							"category code {value} exceeds reserved code {}",
							dictionary.len()
						),
					));
				}
				return Ok((i32::from(value == positive_code), TargetObservation::Known));
			}
			if !matches!(value, 0 | 1) {
				return Err(target_lowering_error(
					target.name(),
					source_row,
					format!("binary target matrix contains {value}; only exact zero and one are accepted"),
				));
			}
			return Ok((value, TargetObservation::Known));
		}
		DenseTask::MulticlassClassification {
			class_count,
			reserved_code,
			..
		} => {
			let reserved_index = usize::try_from(reserved_code).map_err(|_error| {
				return TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					format!("multiclass reserved code {reserved_code} is negative"),
				);
			})?;
			if class_count == 0 || reserved_index.checked_add(1) != Some(class_count) {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					"multiclass class count does not include exactly one reserved unseen-label route",
				));
			}
			let category = usize::try_from(value).map_err(|_error| {
				return target_lowering_error(
					target.name(),
					source_row,
					format!("negative category code {value}"),
				);
			})?;
			if category == reserved_index {
				return Ok((0, TargetObservation::Unseen));
			}
			if category > reserved_index {
				return Err(target_lowering_error(
					target.name(),
					source_row,
					format!("category code {value} exceeds reserved code {reserved_code}"),
				));
			}
			return Ok((value, TargetObservation::Known));
		}
		DenseTask::MultiTargetBinaryClassification { .. } | DenseTask::JointMulticlassClassification { .. } | DenseTask::MultiTargetRegression { .. } => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"multi-target task reached singular int32 target lowering",
			));
		}
	}
}

fn compact_dense_rows(matrix: &DenseMatrix, rows: &[usize], role: &str) -> TrainingCompileResult<DenseMatrix> {
	let columns = matrix.columns();
	let capacity = rows.len().checked_mul(columns).ok_or_else(|| {
		return TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			format!("{role} compaction element count overflowed usize"),
		);
	})?;
	match matrix {
		DenseMatrix::I32 { values, .. } => {
			let mut compacted = Vec::with_capacity(capacity);
			for row in rows.iter().copied() {
				let start = row.checked_mul(columns).ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row offset overflowed usize"),
					);
				})?;
				let end = start.checked_add(columns).ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row end overflowed usize"),
					);
				})?;
				compacted.extend_from_slice(values.get(start..end).ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::InconsistentRows,
						format!("{role} row {row} is absent from dense storage"),
					);
				})?);
			}
			return Ok(DenseMatrix::I32 {
				rows: rows.len(),
				columns,
				values: compacted,
			});
		}
		DenseMatrix::F32Bits { values, .. } => {
			let mut compacted = Vec::with_capacity(capacity);
			for row in rows.iter().copied() {
				let start = row.checked_mul(columns).ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row offset overflowed usize"),
					);
				})?;
				let end = start.checked_add(columns).ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row end overflowed usize"),
					);
				})?;
				compacted.extend_from_slice(values.get(start..end).ok_or_else(|| {
					return TrainingCompileError::new(
						TrainingCompileErrorKind::InconsistentRows,
						format!("{role} row {row} is absent from dense storage"),
					);
				})?);
			}
			return Ok(DenseMatrix::F32Bits {
				rows: rows.len(),
				columns,
				values: compacted,
			});
		}
	}
}

fn target_lowering_error(name: &[u8], source_row: usize, detail: impl core::fmt::Display) -> TrainingCompileError {
	return TrainingCompileError::new(
		TrainingCompileErrorKind::InvalidTargetMatrix,
		format!(
			"target {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(name)
		),
	);
}

pub fn lower_dense_features(dataset: &PreparedDataset, plan: &DenseFeaturePlan, partition_kind: PartitionKind) -> TrainingCompileResult<DenseMatrix> {
	if plan.normalization_mask.is_none() {
		return Ok(dataset.fixed_dense_matrix(VectorRole::Feature, partition_kind)?);
	}
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	let capacity = partition
		.len()
		.checked_mul(plan.input_width)
		.ok_or_else(|| {
			return TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"lowered dense feature element count overflowed usize",
			);
		})?;
	let features = dataset
		.vectors()
		.iter()
		.filter(|vector| return vector.role() == VectorRole::Feature)
		.collect::<Vec<_>>();
	if features.len() != plan.spans.len() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			"dense feature plan no longer matches its prepared vectors",
		));
	}
	let mut output = Vec::with_capacity(capacity);
	for (retained_position, source_row) in partition
		.retained_positions()
		.iter()
		.copied()
		.zip(partition.source_rows().iter().copied())
	{
		for (feature, span) in features.iter().copied().zip(&plan.spans) {
			if feature.source_index() != span.source_vector {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidFeatureMatrix,
					"dense feature plan source identity no longer matches its prepared vector",
				));
			}
			match span.lowering {
				DenseFeatureLowering::NumericScalar => {
					let bits = match feature.values() {
						PreparedValues::I32(values) => {
							let value = values
								.get(retained_position)
								.ok_or_else(|| {
									return feature_lowering_error(
										feature.name(),
										source_row,
										"numeric value is absent from the prepared vector",
									);
								})?
								.ok_or_else(|| {
									return feature_lowering_error(feature.name(), source_row, "numeric value is missing");
								})?;
							let Some(converted) = exact_i32_as_f32(value) else {
								return Err(feature_lowering_error(
									feature.name(),
									source_row,
									format!("int32 value {value} cannot be represented exactly by dense f32 calculation"),
								));
							};
							converted.to_bits()
						}
						PreparedValues::F32Bits(values) => {
							values.get(retained_position)
								.ok_or_else(|| {
									return feature_lowering_error(
										feature.name(),
										source_row,
										"numeric value is absent from the prepared vector",
									);
								})?
								.ok_or_else(|| {
									return feature_lowering_error(feature.name(), source_row, "numeric value is missing");
								})?
						}
						PreparedValues::VariableWidth(_) => {
							return Err(feature_lowering_error(
								feature.name(),
								source_row,
								"numeric feature unexpectedly contains variable-width values",
							));
						}
					};
					output.push(bits);
				}
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				} => {
					let values = feature.values().i32_values().ok_or_else(|| {
						return feature_lowering_error(
							feature.name(),
							source_row,
							"dictionary encoding does not contain int32 category codes",
						);
					})?;
					let code = values.get(retained_position).ok_or_else(|| {
						return feature_lowering_error(
							feature.name(),
							source_row,
							"category code is absent from the prepared vector",
						);
					})?;
					let expected_width = dictionary_width.checked_add(1).ok_or_else(|| {
						return feature_lowering_error(
							feature.name(),
							source_row,
							"categorical span omitted its reserved route",
						);
					})?;
					if span.width != expected_width || reserved_index != dictionary_width {
						return Err(feature_lowering_error(
							feature.name(),
							source_row,
							"categorical span reserved-route identity is inconsistent",
						));
					}
					let category = match *code {
						Some(code) => {
							let category = usize::try_from(code).map_err(|_error| {
								return feature_lowering_error(
									feature.name(),
									source_row,
									format!("negative category code {code} violates dictionary encoding"),
								);
							})?;
							if category > dictionary_width {
								return Err(feature_lowering_error(
									feature.name(),
									source_row,
									format!("category code {code} exceeds dictionary and reserved index {dictionary_width}"),
								));
							}
							category
						}
						None => reserved_index,
					};
					let start = output.len();
					output.resize(start + span.width, 0.0f32.to_bits());
					output[start + category] = 1.0f32.to_bits();
				}
			}
		}
	}
	return Ok(DenseMatrix::F32Bits {
		rows: partition.len(),
		columns: plan.input_width,
		values: output,
	});
}

fn feature_lowering_error(name: &[u8], source_row: usize, detail: impl core::fmt::Display) -> TrainingCompileError {
	return TrainingCompileError::new(
		TrainingCompileErrorKind::InvalidFeatureMatrix,
		format!(
			"feature {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(name)
		),
	);
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
	#[inline]
	fn default() -> Self {
		return Self {
			learning_rate: 1.0e-4,
			beta_one: 0.9,
			beta_two: 0.999,
			epsilon: 1.0e-8,
			weight_decay: 0.01,
		};
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryValidationConfig {
	calibration_bins: NonZeroU32,
	recall_threshold_bits: Vec<u32>,
	temperature_scaling: Option<TemperatureScalingConfig>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MulticlassValidationConfig;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegressionValidationConfig;

impl BinaryValidationConfig {
	#[must_use]
	#[inline]
	pub fn new(calibration_bins: NonZeroU32, recall_thresholds: impl IntoIterator<Item = f32>) -> Self {
		return Self {
			calibration_bins,
			recall_threshold_bits: recall_thresholds.into_iter().map(f32::to_bits).collect(),
			temperature_scaling: None,
		};
	}

	#[must_use]
	#[inline]
	pub const fn calibration_bins(&self) -> NonZeroU32 { return self.calibration_bins; }

	#[must_use]
	#[inline]
	pub fn recall_thresholds(&self) -> impl ExactSizeIterator<Item = f32> + '_ {
		return self
			.recall_threshold_bits
			.iter()
			.copied()
			.map(f32::from_bits);
	}

	#[must_use]
	#[inline]
	pub const fn temperature_scaling(&self) -> Option<TemperatureScalingConfig> { return self.temperature_scaling; }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemperatureScalingConfig {
	pub iterations: NonZeroU64,
	pub learning_rate: f32,
	pub minimum_temperature: f32,
	pub maximum_temperature: f32,
}

impl Default for TemperatureScalingConfig {
	#[inline]
	fn default() -> Self {
		return Self {
			iterations: NonZeroU64::MIN.saturating_add(63),
			learning_rate: 0.01,
			minimum_temperature: 0.05,
			maximum_temperature: 20.0,
		};
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingBounds {
	pub train_rows: u64,
	pub epochs: TrainingHorizon,
	pub training_iterations: LoopIterations,
	pub calibration_iterations: u64,
	pub iterations: LoopIterations,
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
	#[inline]
	pub const fn role(&self) -> ExternalInputRole { return self.role; }

	#[must_use]
	#[inline]
	pub const fn value(&self) -> ValueId { return self.value; }

	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> DType { return self.dtype; }

	#[must_use]
	#[inline]
	pub const fn shape(&self) -> &Shape { return &self.shape; }

	#[must_use]
	#[inline]
	pub fn bytes(&self) -> &[u8] { return &self.bytes; }

	pub(crate) const fn new(role: ExternalInputRole, value: ValueId, dtype: DType, shape: Shape, bytes: Vec<u8>) -> Self {
		return Self {
			role,
			value,
			dtype,
			shape,
			bytes,
		};
	}

	pub(crate) fn replace_bytes(&mut self, bytes: &[u8]) {
		self.bytes.clear();
		self.bytes.extend_from_slice(bytes);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalInputRole {
	TrainFeatures,
	TrainTargets,
	TrainTargetSupervision,
	ResumeEnabled,
	ResumeParameter { ordinal: usize },
	ResumeFirstMoment { ordinal: usize },
	ResumeSecondMoment { ordinal: usize },
	ResumeKMeansCentroids { block: usize },
	ResumeTreeSplitFeatures { block: usize },
	ResumeTreeSplitThresholds { block: usize },
	ResumeTreeLeafValues { block: usize },
	TrainingPoolWindowIndices { block: usize },
	TrainingPoolWinnerBases { block: usize },
	TrainingPoolGradientBatchIndices { block: usize },
	TrainingConvolutionWindowIndices { block: usize },
	TrainingConvolutionInputGradientIndices { block: usize },
	TrainingConvolutionInputGradientValidity { block: usize },
	ValidationFeatures,
	ValidationTargets,
	ValidationPoolWindowIndices { block: usize },
	ValidationPoolWinnerBases { block: usize },
	ValidationConvolutionWindowIndices { block: usize },
	FeatureNormalizationMask,
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
pub struct ZScoreState {
	pub mean: ValueId,
	pub variance: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinMaxState {
	pub minimum: ValueId,
	pub maximum: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataNormalizationState {
	Identity,
	ZScore(ZScoreState),
	MinMax(MinMaxState),
	L2Norm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OptimizerProgressState {
	pub apply_update: ValueId,
	pub initial_accepted_updates: ValueId,
	pub updated_accepted_updates: ValueId,
	pub update_kernel: KernelTemplateId,
	pub initial_beta_one_power: ValueId,
	pub updated_beta_one_power: ValueId,
	pub initial_beta_two_power: ValueId,
	pub updated_beta_two_power: ValueId,
	pub beta_update_kernel: KernelTemplateId,
	pub accepted_updates_per_epoch: u64,
	pub maximum_accepted_updates: Option<u64>,
	pub accepted_update_counter_limit: u64,
	pub warmup_accepted_updates: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationMetricFamily {
	Binary,
	Multiclass,
	Regression,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationUnavailableReason {
	NoKnownTargets,
	SingleKnownClass { known_rows: NonZeroU64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationMetricStatus {
	NotRequested,
	Available {
		family: ValidationMetricFamily,
		known_rows: NonZeroU64,
	},
	Unavailable {
		family: ValidationMetricFamily,
		reason: ValidationUnavailableReason,
		split_rows: u64,
	},
}

#[derive(Clone, Debug)]
pub struct TrainingOutputs {
	pub training_loss: ValueId,
	pub training_loss_domain: IterationDomain,
	pub normalization: DataNormalizationState,
	pub optimizer_progress: Option<OptimizerProgressState>,
	pub blocks: Vec<Box<dyn crate::compile::RealizedBlock>>,
	pub validation: Option<BinaryValidationOutputs>,
	pub multiclass_validation: Option<MulticlassValidationOutputs>,
	pub regression_validation: Option<RegressionValidationOutputs>,
	pub validation_status: ValidationMetricStatus,
	pub metric_bindings: Vec<TrainingMetricBinding>,
}

impl TrainingOutputs {
	pub(crate) fn visit_parameter_states(&self, mut visit: impl FnMut(ParameterState)) {
		for block in &self.blocks {
			block.visit_parameter_states(&mut visit);
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingMetricKind {
	TrainingLoss,
	LearningRate,
	ValidationMeanBce,
	ValidationMeanCrossEntropy,
	Accuracy,
	R2,
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
	#[inline]
	pub const fn threshold(self) -> f32 { return f32::from_bits(self.threshold_bits); }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryMetricOutputs {
	pub mean_bce: ValueId,
	pub accuracy: ValueId,
	pub auroc: ValueId,
	pub auprc: ValueId,
	pub brier_score: ValueId,
	pub expected_calibration_error: ValueId,
	pub recall_at: Vec<RecallMetricOutput>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MulticlassMetricOutputs {
	pub mean_cross_entropy: ValueId,
	pub accuracy: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegressionMetricOutputs {
	pub r2: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegressionValidationOutputs {
	pub predictions: ValueId,
	pub metrics: RegressionMetricOutputs,
	pub metric_domain: IterationDomain,
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
	pub temperature_scaling: Option<TemperatureScalingState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MulticlassValidationOutputs {
	pub logits: ValueId,
	pub metrics: MulticlassMetricOutputs,
	pub metric_domain: IterationDomain,
}

#[derive(Clone, Debug)]
pub struct CompiledTrainingParts {
	pub program: StaticCalculationProgram,
	pub external_inputs: Vec<OwnedExternalInput>,
	pub bounds: TrainingBounds,
	pub outputs: TrainingOutputs,
	pub dataset_schema: CompiledDatasetSchema,
	pub blocks: Vec<Block>,
	pub output_adapter: Option<DenseOutputAdapter>,
}

#[derive(Clone, Debug)]
pub struct CompiledTraining {
	parts: CompiledTrainingParts,
}

impl From<CompiledTrainingParts> for CompiledTraining {
	fn from(parts: CompiledTrainingParts) -> Self { return Self { parts }; }
}

impl CompiledTraining {
	#[must_use]
	#[inline]
	pub const fn graph(&self) -> &CalculationGraph { return self.parts.program.graph(); }

	#[must_use]
	#[inline]
	pub const fn program(&self) -> &StaticCalculationProgram { return &self.parts.program; }

	#[must_use]
	#[inline]
	pub fn external_inputs(&self) -> &[OwnedExternalInput] { return &self.parts.external_inputs; }

	pub(crate) fn external_inputs_mut(&mut self) -> &mut [OwnedExternalInput] { return &mut self.parts.external_inputs; }

	#[must_use]
	#[inline]
	pub const fn bounds(&self) -> TrainingBounds { return self.parts.bounds; }

	#[must_use]
	#[inline]
	pub const fn outputs(&self) -> &TrainingOutputs { return &self.parts.outputs; }

	#[must_use]
	#[inline]
	pub const fn dataset_schema(&self) -> &CompiledDatasetSchema { return &self.parts.dataset_schema; }

	#[must_use]
	#[inline]
	pub fn blocks(&self) -> &[Block] { return &self.parts.blocks; }

	#[must_use]
	#[inline]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> { return self.parts.output_adapter; }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedTrainingFeature {
	DynamicLoopShortening,
	ExactOptimizerResume,
}

pub const REMAINING_UNSUPPORTED: &[UnsupportedTrainingFeature] = &[
	UnsupportedTrainingFeature::DynamicLoopShortening,
	UnsupportedTrainingFeature::ExactOptimizerResume,
];

fn validate_matrix_storage(matrix: &DenseMatrix, role: &str) -> TrainingCompileResult<()> {
	let expected = matrix.rows().checked_mul(matrix.columns()).ok_or_else(|| {
		return TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			format!("{role} element count overflowed usize"),
		);
	})?;
	let actual = match matrix {
		DenseMatrix::I32 { values, .. } => values.len(),
		DenseMatrix::F32Bits { values, .. } => {
			if let Some(bits) = values
				.iter()
				.copied()
				.find(|bits| return !f32::from_bits(*bits).is_finite())
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
	return Ok(());
}

pub fn validate_binary_targets(targets: &DenseMatrix) -> TrainingCompileResult<()> {
	let invalid = match targets {
		DenseMatrix::I32 { values, .. } => {
			values.iter()
				.copied()
				.find(|value| return !matches!(value, 0 | 1))
				.map(|value| return value.to_string())
		}
		DenseMatrix::F32Bits { values, .. } => {
			values.iter()
				.copied()
				.map(f32::from_bits)
				.find(|value| return !is_binary_target(*value))
				.map(|value| return value.to_string())
		}
	};
	if let Some(value) = invalid {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!("binary target matrix contains {value}; only exact zero and one are accepted"),
		));
	}
	return Ok(());
}
