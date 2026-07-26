use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use recipe_core::{DType, IterationDomain, KernelTemplateId, MetricId, ValueId};
use recipe_ingest::{
	DenseMatrix, PartitionKind, PreparedDataset, PreparedValues, SemanticType, VectorEncoding, VectorMetadata,
	VectorRole, VectorSchema,
};
use recipe_language::{CalculationGraph, Shape};
use recipe_program::StaticCalculationProgram;

use crate::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseActivation {
	Linear,
	Cosine,
	Exponential,
	Logarithm,
	Huber,
	Tangent,
	Relu,
	Gelu,
	Silu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseNormalization {
	Layer,
	Batch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseOperation {
	Activation(DenseActivation),
	Normalization(DenseNormalization),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseBlockKind {
	Layer,
	Perc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseDataNormalization {
	ZScore,
	MinMax,
	L2Norm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseLoss {
	BinaryCrossEntropy,
	MeanSquaredError,
	MeanAbsoluteError,
	CrossEntropy,
	Huber,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LearningRateDecay {
	Linear,
	Cosine,
	Exponential,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseLayer {
	kind: DenseBlockKind,
	width: NonZeroU64,
	operations: Vec<DenseOperation>,
}

impl DenseLayer {
	#[must_use]
	pub fn new(width: NonZeroU64, activation: DenseActivation) -> Self {
		Self {
			kind: DenseBlockKind::Layer,
			width,
			operations: match activation {
				DenseActivation::Linear => Vec::new(),
				activation => vec![DenseOperation::Activation(activation)],
			},
		}
	}

	#[must_use]
	pub fn with_operations(width: NonZeroU64, operations: impl IntoIterator<Item = DenseOperation>) -> Self {
		Self::with_kind(DenseBlockKind::Layer, width, operations)
	}

	#[must_use]
	pub fn with_kind(
		kind: DenseBlockKind,
		width: NonZeroU64,
		operations: impl IntoIterator<Item = DenseOperation>,
	) -> Self {
		Self {
			kind,
			width,
			operations: operations.into_iter().collect(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> DenseBlockKind {
		self.kind
	}

	#[must_use]
	pub const fn width(&self) -> NonZeroU64 {
		self.width
	}

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] {
		&self.operations
	}
}

/// One ordered residual branch and the operations applied after its merge.
///
/// The final branch layer determines the residual output width. The compiler
/// uses an identity skip at that width and otherwise inserts a learned,
/// weight-only linear projection from the block input to that width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseResidual {
	branch: Vec<DenseResidualOperation>,
	operations: Vec<DenseOperation>,
}

/// One exact, ordered operation inside a residual branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseResidualOperation {
	Layer(DenseLayer),
	Operation(DenseOperation),
}

impl From<DenseLayer> for DenseResidualOperation {
	fn from(layer: DenseLayer) -> Self {
		Self::Layer(layer)
	}
}

impl From<DenseOperation> for DenseResidualOperation {
	fn from(operation: DenseOperation) -> Self {
		Self::Operation(operation)
	}
}

impl DenseResidual {
	#[must_use]
	pub fn new<T>(
		branch: impl IntoIterator<Item = T>,
		operations: impl IntoIterator<Item = DenseOperation>,
	) -> Self
	where
		T: Into<DenseResidualOperation>,
	{
		Self {
			branch: branch.into_iter().map(Into::into).collect(),
			operations: operations.into_iter().collect(),
		}
	}

	#[must_use]
	pub fn branch(&self) -> &[DenseResidualOperation] {
		&self.branch
	}

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] {
		&self.operations
	}

	#[must_use]
	pub fn output_width(&self) -> Option<NonZeroU64> {
		self.branch.iter().rev().find_map(|operation| match operation {
			DenseResidualOperation::Layer(layer) => Some(layer.width()),
			DenseResidualOperation::Operation(_) => None,
		})
	}
}

/// A topology-preserving dense training block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseBlock {
	Layer(DenseLayer),
	Residual(DenseResidual),
}

impl DenseBlock {
	#[must_use]
	pub fn output_width(&self) -> Option<NonZeroU64> {
		match self {
			Self::Layer(layer) => Some(layer.width()),
			Self::Residual(residual) => residual.output_width(),
		}
	}

	#[must_use]
	pub fn output_operations(&self) -> &[DenseOperation] {
		match self {
			Self::Layer(layer) => layer.operations(),
			Self::Residual(residual) => residual.operations(),
		}
	}
}

impl From<DenseLayer> for DenseBlock {
	fn from(layer: DenseLayer) -> Self {
		Self::Layer(layer)
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
}

impl DenseTask {
	#[must_use]
	pub const fn target_vector(self) -> usize {
		match self {
			Self::BinaryClassification { target_vector, .. }
			| Self::MulticlassClassification { target_vector, .. }
			| Self::ScalarRegression { target_vector } => target_vector,
		}
	}

	#[must_use]
	pub const fn output_width(self) -> usize {
		match self {
			Self::BinaryClassification { .. } | Self::ScalarRegression { .. } => 1,
			Self::MulticlassClassification { class_count, .. } => class_count,
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
		Self {
			source_width,
			target_width,
		}
	}

	#[must_use]
	pub const fn source_width(self) -> NonZeroU64 {
		self.source_width
	}

	#[must_use]
	pub const fn target_width(self) -> NonZeroU64 {
		self.target_width
	}
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
	pub(crate) const fn new(
		source_vector: usize,
		start: usize,
		width: usize,
		lowering: DenseFeatureLowering,
	) -> Self {
		Self {
			source_vector,
			start,
			width,
			lowering,
		}
	}

	#[must_use]
	pub const fn source_vector(&self) -> usize {
		self.source_vector
	}

	#[must_use]
	pub const fn start(&self) -> usize {
		self.start
	}

	#[must_use]
	pub const fn width(&self) -> usize {
		self.width
	}

	#[must_use]
	pub const fn lowering(&self) -> DenseFeatureLowering {
		self.lowering
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledDatasetSchema {
	vectors: Vec<VectorSchema>,
	features: Vec<CompiledFeatureSpan>,
	target: usize,
	input_width: usize,
	task: DenseTask,
}

impl CompiledDatasetSchema {
	pub(crate) fn from_prepared(
		dataset: &PreparedDataset,
		task: DenseTask,
		features: Vec<CompiledFeatureSpan>,
	) -> Self {
		let vectors = dataset
			.vectors()
			.iter()
			.map(|vector| vector.schema())
			.collect::<Vec<_>>();
		let target = task.target_vector();
		let input_width = features.iter().map(|feature| feature.width).sum();
		Self {
			vectors,
			features,
			target,
			input_width,
			task,
		}
	}

	#[must_use]
	pub fn vectors(&self) -> &[VectorSchema] {
		&self.vectors
	}

	#[must_use]
	pub fn features(&self) -> &[CompiledFeatureSpan] {
		&self.features
	}

	#[must_use]
	pub const fn target(&self) -> usize {
		self.target
	}

	#[must_use]
	pub const fn input_width(&self) -> usize {
		self.input_width
	}

	#[must_use]
	pub const fn task(&self) -> DenseTask {
		self.task
	}

	#[must_use]
	pub const fn output_width(&self) -> usize {
		self.task.output_width()
	}

	/// Decode one multiclass output index under the fitted target dictionary.
	///
	/// The final index is the explicit checkpoint-v5 route for a nonempty label
	/// absent from the fit partition. An out-of-range index, or a non-multiclass
	/// schema, has no categorical decoding.
	#[must_use]
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
		let target = self
			.vectors
			.iter()
			.find(|vector| vector.source_index() == target_vector && vector.role() == VectorRole::Target)?;
		let VectorMetadata::Categorical { dictionary } = target.metadata() else {
			return None;
		};
		if i32::try_from(class).ok() == Some(reserved_code) {
			return Some(DecodedMulticlassClass::ReservedUnseen);
		}
		dictionary
			.get(class)
			.map(|label| DecodedMulticlassClass::Label(label))
	}
}

/// Semantic decoding of one multiclass output index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedMulticlassClass<'a> {
	Label(&'a [u8]),
	ReservedUnseen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DenseFeaturePlan {
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
			.filter(|vector| vector.role() == VectorRole::Feature)
		{
			let (width, lowering, normalize) = match (
				feature.semantic_type(),
				feature.encoding(),
				feature.metadata(),
				feature.values(),
			) {
				(SemanticType::Numeric, VectorEncoding::I32, VectorMetadata::None, PreparedValues::I32(_))
				| (
					SemanticType::Numeric,
					VectorEncoding::F32,
					VectorMetadata::None,
					PreparedValues::F32Bits(_),
				) => (1, DenseFeatureLowering::NumericScalar, true),
				(
					SemanticType::Categorical,
					VectorEncoding::DictionaryI32,
					VectorMetadata::Categorical { dictionary },
					PreparedValues::I32(_),
				) => {
					validate_categorical_dictionary(feature.name(), dictionary)?;
					has_categorical = true;
					let reserved_index = dictionary.len();
					(
						reserved_index.checked_add(1).ok_or_else(|| {
							TrainingCompileError::new(
								TrainingCompileErrorKind::ArithmeticOverflow,
								"categorical one-hot width overflowed usize",
							)
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
				TrainingCompileError::new(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"lowered dense feature width overflowed usize",
				)
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
		Ok(Self {
			spans,
			input_width,
			normalization_mask: has_categorical.then_some(normalization_mask),
		})
	}

	#[must_use]
	pub(crate) fn spans(&self) -> &[CompiledFeatureSpan] {
		&self.spans
	}

	#[must_use]
	pub(crate) fn normalization_mask(&self) -> Option<&[u32]> {
		self.normalization_mask.as_deref()
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DensePartition {
	features: DenseMatrix,
	targets: DenseMatrix,
}

impl DensePartition {
	fn new(features: DenseMatrix, targets: DenseMatrix) -> TrainingCompileResult<Self> {
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
pub(crate) struct LoweredDenseDataset {
	train: DensePartition,
	validation: Option<DensePartition>,
}

impl LoweredDenseDataset {
	fn new(train: DensePartition, validation: Option<DensePartition>) -> TrainingCompileResult<Self> {
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

	pub(crate) fn from_prepared(
		dataset: &PreparedDataset,
		feature_plan: &DenseFeaturePlan,
		task: DenseTask,
	) -> TrainingCompileResult<Self> {
		let train = DensePartition::new(
			lower_dense_features(dataset, feature_plan, PartitionKind::Train)?,
			lower_dense_targets(dataset, task, PartitionKind::Train)?,
		)?;
		let validation = if dataset.validation().is_empty() {
			None
		} else {
			Some(DensePartition::new(
				lower_dense_features(dataset, feature_plan, PartitionKind::Validation)?,
				lower_dense_targets(dataset, task, PartitionKind::Validation)?,
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

fn lower_dense_targets(
	dataset: &PreparedDataset,
	task: DenseTask,
	partition_kind: PartitionKind,
) -> TrainingCompileResult<DenseMatrix> {
	let matrix = dataset.fixed_dense_matrix(VectorRole::Target, partition_kind)?;
	let DenseTask::MulticlassClassification {
		target_vector,
		class_count,
		reserved_code,
	} = task
	else {
		return Ok(matrix);
	};
	let target = dataset
		.vectors()
		.iter()
		.find(|vector| vector.source_index() == target_vector && vector.role() == VectorRole::Target)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("multiclass target vector {target_vector} is absent"),
			)
		})?;
	let PreparedValues::I32(values) = target.values() else {
		return Err(target_lowering_error(
			target.name(),
			0,
			"dictionary encoding does not contain int32 category codes",
		));
	};
	let reserved_index = usize::try_from(reserved_code).map_err(|_| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!("multiclass reserved code {reserved_code} is negative"),
		)
	})?;
	if class_count == 0 || reserved_index.checked_add(1) != Some(class_count) {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"multiclass class count does not include exactly one reserved unseen-label route",
		));
	}
	let DenseMatrix::I32 {
		rows,
		columns,
		values: matrix_values,
	} = &matrix
	else {
		return Err(target_lowering_error(
			target.name(),
			0,
			"dictionary target matrix does not retain int32 category codes",
		));
	};
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	if *rows != partition.len() || *columns != 1 || matrix_values.len() != partition.len() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!(
				"multiclass target codes require an int32 [rows, 1] matrix; got [{rows}, {columns}] with {} values",
				matrix_values.len()
			),
		));
	}
	for (retained_position, source_row) in partition
		.retained_positions()
		.iter()
		.copied()
		.zip(partition.source_rows().iter().copied())
	{
		let code = values
			.get(retained_position)
			.ok_or_else(|| target_lowering_error(target.name(), source_row, "category code is absent"))?
			.ok_or_else(|| target_lowering_error(target.name(), source_row, "target category is missing"))?;
		let category = usize::try_from(code).map_err(|_| {
			target_lowering_error(
				target.name(),
				source_row,
				format!("negative category code {code}"),
			)
		})?;
		if category >= class_count {
			return Err(target_lowering_error(
				target.name(),
				source_row,
				format!("category code {code} exceeds reserved code {reserved_code}"),
			));
		}
		if partition_kind == PartitionKind::Train && category == reserved_index {
			return Err(target_lowering_error(
				target.name(),
				source_row,
				"fit partition unexpectedly uses its validation-only reserved label route",
			));
		}
	}
	Ok(matrix)
}

fn target_lowering_error(name: &[u8], source_row: usize, detail: impl core::fmt::Display) -> TrainingCompileError {
	TrainingCompileError::new(
		TrainingCompileErrorKind::InvalidTargetMatrix,
		format!(
			"target {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(name)
		),
	)
}

fn lower_dense_features(
	dataset: &PreparedDataset,
	plan: &DenseFeaturePlan,
	partition_kind: PartitionKind,
) -> TrainingCompileResult<DenseMatrix> {
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
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"lowered dense feature element count overflowed usize",
			)
		})?;
	let features = dataset
		.vectors()
		.iter()
		.filter(|vector| vector.role() == VectorRole::Feature)
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
					output.push(lower_numeric_feature(
						feature,
						retained_position,
						source_row,
					)?);
				}
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				} => {
					let PreparedValues::I32(values) = feature.values() else {
						return Err(feature_lowering_error(
							feature.name(),
							source_row,
							"dictionary encoding does not contain int32 category codes",
						));
					};
					let code = values.get(retained_position).ok_or_else(|| {
						feature_lowering_error(
							feature.name(),
							source_row,
							"category code is absent from the prepared vector",
						)
					})?;
					let expected_width = dictionary_width.checked_add(1).ok_or_else(|| {
						feature_lowering_error(
							feature.name(),
							source_row,
							"categorical span omitted its reserved route",
						)
					})?;
					if span.width != expected_width || reserved_index != dictionary_width {
						return Err(feature_lowering_error(
							feature.name(),
							source_row,
							"categorical span reserved-route identity is inconsistent",
						));
					}
					let category = match code {
						Some(code) => {
							categorical_code_index(*code, dictionary_width, feature.name(), source_row)?
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
	Ok(DenseMatrix::F32Bits {
		rows: partition.len(),
		columns: plan.input_width,
		values: output,
	})
}

fn lower_numeric_feature(
	feature: &recipe_ingest::PreparedVector,
	retained_position: usize,
	source_row: usize,
) -> TrainingCompileResult<u32> {
	match feature.values() {
		PreparedValues::I32(values) => {
			let value = values
				.get(retained_position)
				.ok_or_else(|| {
					feature_lowering_error(
						feature.name(),
						source_row,
						"numeric value is absent from the prepared vector",
					)
				})?
				.ok_or_else(|| {
					feature_lowering_error(feature.name(), source_row, "numeric value is missing")
				})?;
			let converted = value as f32;
			if f64::from(converted) != f64::from(value) {
				return Err(feature_lowering_error(
					feature.name(),
					source_row,
					format!("int32 value {value} cannot be represented exactly by dense f32 calculation"),
				));
			}
			Ok(converted.to_bits())
		}
		PreparedValues::F32Bits(values) => values
			.get(retained_position)
			.ok_or_else(|| {
				feature_lowering_error(
					feature.name(),
					source_row,
					"numeric value is absent from the prepared vector",
				)
			})?
			.ok_or_else(|| feature_lowering_error(feature.name(), source_row, "numeric value is missing")),
		PreparedValues::VariableWidth(_) => Err(feature_lowering_error(
			feature.name(),
			source_row,
			"numeric feature unexpectedly contains variable-width values",
		)),
	}
}

fn validate_categorical_dictionary(name: &[u8], dictionary: &[Vec<u8>]) -> TrainingCompileResult<()> {
	if dictionary.is_empty() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			format!(
				"categorical feature {:?} has an empty dictionary",
				String::from_utf8_lossy(name)
			),
		));
	}
	let mut labels = std::collections::BTreeSet::new();
	if dictionary
		.iter()
		.any(|label| label.is_empty() || !labels.insert(label))
	{
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			format!(
				"categorical feature {:?} has an empty or duplicate dictionary label",
				String::from_utf8_lossy(name)
			),
		));
	}
	Ok(())
}

fn categorical_code_index(
	code: i32,
	known_width: usize,
	name: &[u8],
	source_row: usize,
) -> TrainingCompileResult<usize> {
	let index = usize::try_from(code).map_err(|_| {
		feature_lowering_error(
			name,
			source_row,
			format!("negative category code {code} violates dictionary encoding"),
		)
	})?;
	if index <= known_width {
		Ok(index)
	} else {
		Err(feature_lowering_error(
			name,
			source_row,
			format!("category code {code} exceeds dictionary and reserved index {known_width}"),
		))
	}
}

fn feature_lowering_error(name: &[u8], source_row: usize, detail: impl core::fmt::Display) -> TrainingCompileError {
	TrainingCompileError::new(
		TrainingCompileErrorKind::InvalidFeatureMatrix,
		format!(
			"feature {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(name)
		),
	)
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
	pub loss: DenseLoss,
	pub data_normalization: DenseDataNormalization,
	pub batch_size: NonZeroUsize,
	pub epochs: NonZeroU64,
	pub warmup_epochs: u64,
	pub learning_rate_decay: LearningRateDecay,
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

/// Request epoch-bound categorical cross-entropy and top-one accuracy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MulticlassValidationConfig;

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
pub struct DenseLayerState {
	pub weight: ParameterState,
	pub bias: ParameterState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseResidualState {
	pub branch: Vec<DenseLayerState>,
	pub projection: Option<ParameterState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseBlockState {
	Layer(DenseLayerState),
	Residual(DenseResidualState),
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
	ZScore(ZScoreState),
	MinMax(MinMaxState),
	L2Norm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingOutputs {
	pub batch_loss: ValueId,
	pub batch_loss_domain: IterationDomain,
	pub normalization: DataNormalizationState,
	pub blocks: Vec<DenseBlockState>,
	pub layers: Vec<DenseLayerState>,
	pub validation: Option<BinaryValidationOutputs>,
	pub multiclass_validation: Option<MulticlassValidationOutputs>,
	pub metric_bindings: Vec<TrainingMetricBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingMetricKind {
	BatchLoss,
	ValidationMeanBce,
	ValidationMeanCrossEntropy,
	Accuracy,
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
pub struct MulticlassMetricOutputs {
	pub mean_cross_entropy: ValueId,
	pub accuracy: ValueId,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MulticlassValidationOutputs {
	pub logits: ValueId,
	pub metrics: MulticlassMetricOutputs,
	pub metric_domain: IterationDomain,
}

#[derive(Clone, Debug)]
pub struct CompiledTraining {
	pub(crate) program: StaticCalculationProgram,
	pub(crate) external_inputs: Vec<OwnedExternalInput>,
	pub(crate) bounds: TrainingBounds,
	pub(crate) outputs: TrainingOutputs,
	pub(crate) dataset_schema: CompiledDatasetSchema,
	pub(crate) config: DenseTrainingConfig,
	pub(crate) blocks: Vec<DenseBlock>,
	pub(crate) layers: Vec<DenseLayer>,
	pub(crate) output_adapter: Option<DenseOutputAdapter>,
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

	#[must_use]
	pub const fn dataset_schema(&self) -> &CompiledDatasetSchema {
		&self.dataset_schema
	}

	#[must_use]
	pub const fn config(&self) -> &DenseTrainingConfig {
		&self.config
	}

	#[must_use]
	pub fn layers(&self) -> &[DenseLayer] {
		&self.layers
	}

	#[must_use]
	pub fn blocks(&self) -> &[DenseBlock] {
		&self.blocks
	}

	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> {
		self.output_adapter
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
	if targets.columns() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"dense training partition has no target columns",
		));
	}
	validate_matrix_storage(features, "feature")?;
	validate_matrix_storage(targets, "target")
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

pub(crate) fn validate_binary_targets(targets: &DenseMatrix) -> TrainingCompileResult<()> {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn categorical_codes_preserve_the_reserved_route_and_reject_corruption() {
		assert_eq!(categorical_code_index(0, 3, b"color", 7).unwrap(), 0);
		assert_eq!(categorical_code_index(2, 3, b"color", 7).unwrap(), 2);
		assert_eq!(categorical_code_index(3, 3, b"color", 7).unwrap(), 3);

		let negative = categorical_code_index(-1, 3, b"color", 7).unwrap_err();
		assert_eq!(
			negative.kind,
			TrainingCompileErrorKind::InvalidFeatureMatrix
		);
		assert!(negative.detail.contains("negative category code -1"));
		let overflow = categorical_code_index(4, 3, b"color", 7).unwrap_err();
		assert_eq!(
			overflow.kind,
			TrainingCompileErrorKind::InvalidFeatureMatrix
		);
		assert!(overflow.detail.contains("reserved index 3"));
	}
}
