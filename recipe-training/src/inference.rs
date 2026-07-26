use core::fmt;
use core::num::NonZeroU64;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_ingest::{
	DistilledDataset, InferenceFeatureEncoding, InferenceFeatureSchema, InferencePrepareError,
	PreparedInferenceDataset, PreparedInferenceFeature, PreparedInferenceValues, RawTable, SemanticType, SourceError,
	SourceLimit, VectorEncoding, read_source_snapshot,
};
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Elementwise, Gather, IndexBounds, IndexMap, PrimitiveAliasRule,
	PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarProgramBuilder, Scatter,
	ScatterConflict, Shape, Tensor,
};
use recipe_ops::{
	IdentityNamespace, MaterializationRequest, NamedTensor, PreparedParameters, lower_scalar,
	materialize_composition, operation_registry,
};
use recipe_program::{IterationDomain, KernelIterationDomain, StaticCalculationProgram};

use crate::{
	CheckpointArtifact, CheckpointArtifactMetadata, CheckpointDecodeLimits, CheckpointError, CheckpointTensorImage,
	CompiledFeatureSpan, DenseActivation, DenseDataNormalization, DenseFeatureLowering, DenseNormalization,
	DenseOperation, DenseOutputAdapter, DenseTask, decode_checkpoint,
};

const FLAT_CHECKPOINT_FORMAT_VERSION: u32 = 5;
const MATERIALIZATION_RESERVATION: u64 = 64;
const WORKSPACE_LIMIT: ByteCount = ByteCount::new(u64::MAX);

/// Failure to load a checkpoint or prepare schema-bound inference features.
#[derive(Debug)]
#[non_exhaustive]
pub enum InferencePreparationError {
	CheckpointSource(SourceError),
	Checkpoint(CheckpointError),
	Data(InferencePrepareError),
	InconsistentCheckpoint {
		feature: usize,
		source_vector: usize,
		detail: String,
	},
	ArithmeticOverflow {
		detail: String,
	},
}

impl fmt::Display for InferencePreparationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CheckpointSource(error) => write!(formatter, "load checkpoint source: {error}"),
			Self::Checkpoint(error) => write!(formatter, "load checkpoint: {error}"),
			Self::Data(error) => write!(formatter, "prepare inference data: {error}"),
			Self::InconsistentCheckpoint {
				feature,
				source_vector,
				detail,
			} => write!(
				formatter,
				"inconsistent checkpoint inference.feature[{feature}].source-vector[{source_vector}]: {detail}"
			),
			Self::ArithmeticOverflow { detail } => write!(formatter, "prepare inference data: {detail}"),
		}
	}
}

impl std::error::Error for InferencePreparationError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::CheckpointSource(error) => Some(error),
			Self::Checkpoint(error) => Some(error),
			Self::Data(error) => Some(error),
			Self::InconsistentCheckpoint { .. } | Self::ArithmeticOverflow { .. } => None,
		}
	}
}

impl From<SourceError> for InferencePreparationError {
	fn from(error: SourceError) -> Self {
		Self::CheckpointSource(error)
	}
}

impl From<CheckpointError> for InferencePreparationError {
	fn from(error: CheckpointError) -> Self {
		Self::Checkpoint(error)
	}
}

impl From<InferencePrepareError> for InferencePreparationError {
	fn from(error: InferencePrepareError) -> Self {
		Self::Data(error)
	}
}

pub type InferencePreparationResult<T> = Result<T, InferencePreparationError>;

/// Stable class of target-free inference graph compilation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceCompileErrorKind {
	EmptyDataset,
	InconsistentCheckpoint,
	UnsupportedTopology,
	UnsupportedExtent,
	ArithmeticOverflow,
	IdentityExhausted,
	Language,
	Operation,
	Program,
	Ogdl,
}

/// Typed failure to compile a prepared checkpoint and rows into one static
/// target-free inference calculation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceCompileError {
	kind: InferenceCompileErrorKind,
	detail: String,
}

impl InferenceCompileError {
	#[must_use]
	pub const fn kind(&self) -> InferenceCompileErrorKind {
		self.kind
	}

	#[must_use]
	pub fn detail(&self) -> &str {
		&self.detail
	}

	fn new(kind: InferenceCompileErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for InferenceCompileError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl std::error::Error for InferenceCompileError {}

impl From<recipe_language::LanguageError> for InferenceCompileError {
	fn from(error: recipe_language::LanguageError) -> Self {
		Self::new(InferenceCompileErrorKind::Language, error.to_string())
	}
}

impl From<recipe_language::OgdlCodecError> for InferenceCompileError {
	fn from(error: recipe_language::OgdlCodecError) -> Self {
		Self::new(InferenceCompileErrorKind::Ogdl, error.to_string())
	}
}

impl From<recipe_ops::OperationError> for InferenceCompileError {
	fn from(error: recipe_ops::OperationError) -> Self {
		Self::new(InferenceCompileErrorKind::Operation, error.to_string())
	}
}

impl From<recipe_program::ProgramError> for InferenceCompileError {
	fn from(error: recipe_program::ProgramError) -> Self {
		Self::new(InferenceCompileErrorKind::Program, error.to_string())
	}
}

pub type InferenceCompileResult<T> = Result<T, InferenceCompileError>;

/// Semantic role of one immutable host image admitted to an inference graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceInputRole {
	Feature {
		feature: usize,
		source_vector: usize,
	},
	FeatureNormalizationMask,
	DataNormalizationMean,
	DataNormalizationVariance,
	DataNormalizationMinimum,
	DataNormalizationMaximum,
	LayerWeight {
		layer: usize,
	},
	LayerBias {
		layer: usize,
	},
	Temperature,
}

/// One typed external input and the exact bytes copied to its device value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceExternalInput {
	role: InferenceInputRole,
	value: ValueId,
	dtype: DType,
	shape: Shape,
	bytes: Vec<u8>,
}

impl InferenceExternalInput {
	#[must_use]
	pub const fn role(&self) -> InferenceInputRole {
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
}

/// Interpretation of the single inference egress matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferencePredictionKind {
	BinaryProbability,
	MulticlassProbabilities,
	Regression,
}

/// Exact tensor contract for the one public inference prediction output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceOutputContract {
	value: ValueId,
	dtype: DType,
	shape: Shape,
	kind: InferencePredictionKind,
}

impl InferenceOutputContract {
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
	pub const fn kind(&self) -> InferencePredictionKind {
		self.kind
	}
}

/// One deterministic, target-free, single-iteration inference program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledInference {
	program: StaticCalculationProgram,
	external_inputs: Vec<InferenceExternalInput>,
	output: InferenceOutputContract,
	rows: u64,
	task: DenseTask,
	output_adapter: Option<DenseOutputAdapter>,
}

impl CompiledInference {
	#[must_use]
	pub const fn graph(&self) -> &CalculationGraph {
		self.program.graph()
	}

	#[must_use]
	pub const fn program(&self) -> &StaticCalculationProgram {
		&self.program
	}

	#[must_use]
	pub fn external_inputs(&self) -> &[InferenceExternalInput] {
		&self.external_inputs
	}

	#[must_use]
	pub const fn output(&self) -> &InferenceOutputContract {
		&self.output
	}

	#[must_use]
	pub const fn rows(&self) -> u64 {
		self.rows
	}

	#[must_use]
	pub const fn task(&self) -> DenseTask {
		self.task
	}

	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> {
		self.output_adapter
	}
}

/// A decoded model and its unnormalized, schema-bound inference rows.
///
/// The exact fitted normalization tensors remain in `checkpoint`; no host-side
/// numeric conversion, one-hot expansion, normalization, or model calculation
/// has been performed.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedInference {
	checkpoint: CheckpointArtifact,
	data: PreparedInferenceDataset,
}

impl PreparedInference {
	#[must_use]
	pub const fn checkpoint(&self) -> &CheckpointArtifact {
		&self.checkpoint
	}

	#[must_use]
	pub const fn data(&self) -> &PreparedInferenceDataset {
		&self.data
	}

	#[must_use]
	pub fn features(&self) -> &[PreparedInferenceFeature] {
		self.data.features()
	}

	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] {
		self.checkpoint.feature_spans()
	}

	#[must_use]
	pub const fn normalization(&self) -> DenseDataNormalization {
		self.checkpoint.config().data_normalization
	}

	#[must_use]
	pub fn normalization_tensors(&self) -> &[CheckpointTensorImage] {
		self.checkpoint.normalization()
	}

	#[must_use]
	pub fn feature_normalization_mask(&self) -> &[u32] {
		self.checkpoint.feature_normalization_mask()
	}

	#[must_use]
	pub fn normalization_epsilon(&self) -> f32 {
		self.checkpoint.config().normalization_epsilon
	}

	#[must_use]
	pub fn into_checkpoint(self) -> CheckpointArtifact {
		self.checkpoint
	}
}

/// Read and decode one checkpoint from a bounded regular-file snapshot.
///
/// The file is opened and read only during this preparation call. Its admitted
/// bytes are then decoded by the strict versioned checkpoint decoder.
///
/// # Errors
///
/// Returns a typed source error for a zero or unrepresentable bound, an I/O
/// failure, a non-regular source, or a file exceeding the bound. Strict
/// versioned decoding errors retain their checkpoint paths.
pub fn load_checkpoint_file(
	path: impl AsRef<Path>,
	limits: CheckpointDecodeLimits,
) -> InferencePreparationResult<CheckpointArtifact> {
	let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("checkpoint source-byte bound cannot be represented as u64: {error}"),
		})?;
	let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_checkpoint(source.bytes(), limits).map_err(Into::into)
}

/// Prepare a newly distilled dataset under one decoded checkpoint's schema.
///
/// Only saved feature names are read. Source columns may be reordered, target
/// columns may be absent, and unrelated columns are ignored. Saved categorical
/// dictionaries and their v5 reserved route are reused exactly.
///
/// # Errors
///
/// Returns a typed schema/data error or a checked dense-lowering failure.
pub fn prepare_checkpoint_inference(
	checkpoint: CheckpointArtifact,
	dataset: &DistilledDataset,
) -> InferencePreparationResult<PreparedInference> {
	prepare_checkpoint_inference_table(checkpoint, dataset.table())
}

/// Prepare a framed table under one decoded checkpoint's schema.
///
/// This lower-level boundary is useful when a caller already owns an admitted
/// [`RawTable`]. It has the same semantics as [`prepare_checkpoint_inference`].
///
/// # Errors
///
/// Returns a typed schema/data error or a checked dense-lowering failure.
pub fn prepare_checkpoint_inference_table(
	checkpoint: CheckpointArtifact,
	table: &RawTable,
) -> InferencePreparationResult<PreparedInference> {
	let schema = saved_feature_schema(&checkpoint)?;
	let data = recipe_ingest::prepare_inference_table(table, &schema)?;
	validate_prepared_feature_spans(&data, checkpoint.feature_spans())?;
	Ok(PreparedInference { checkpoint, data })
}

/// Load a bounded checkpoint and prepare a newly distilled dataset in one call.
///
/// # Errors
///
/// Returns any checkpoint source, strict decoding, schema application, or dense
/// lowering failure.
pub fn load_and_prepare_checkpoint_inference(
	path: impl AsRef<Path>,
	limits: CheckpointDecodeLimits,
	dataset: &DistilledDataset,
) -> InferencePreparationResult<PreparedInference> {
	let checkpoint = load_checkpoint_file(path, limits)?;
	prepare_checkpoint_inference(checkpoint, dataset)
}

/// Compile prepared rows and the saved checkpoint state into one target-free
/// static calculation program.
///
/// Raw saved-schema feature tensors remain external inputs. Numeric conversion,
/// categorical one-hot expansion, dense concatenation, saved data
/// normalization, every effective layer, and prediction interpretation are all
/// emitted as Recipe calculations. Checkpoint optimizer moments are never
/// admitted to this graph.
///
/// Checkpoint v5's effective flat layer list already contains any synthetic
/// output adapter, so that list is traversed exactly once. Structured v6
/// topology is rejected explicitly until its distinct inference compiler is
/// available; it is never flattened.
///
/// # Errors
///
/// Returns a typed topology, extent, checkpoint-consistency, graph, operation,
/// or static-program failure.
pub fn compile_prepared_inference(prepared: &PreparedInference) -> InferenceCompileResult<CompiledInference> {
	let checkpoint = prepared.checkpoint();
	if checkpoint.format_version() != FLAT_CHECKPOINT_FORMAT_VERSION {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedTopology,
			format!(
				"checkpoint v{} retains structured blocks; bounded target-free inference currently requires flat checkpoint \
				 v{FLAT_CHECKPOINT_FORMAT_VERSION}",
				checkpoint.format_version()
			),
		));
	}
	if checkpoint.layers().is_empty()
		|| checkpoint.blocks().len() != checkpoint.layers().len()
		|| checkpoint
			.blocks()
			.iter()
			.any(|block| !matches!(block, crate::CheckpointBlockImage::Layer(_)))
	{
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"flat checkpoint does not retain one effective layer image per block",
		));
	}
	let rows = u64::try_from(prepared.data().rows()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("inference row count cannot be represented by u64: {error}"),
		)
	})?;
	if rows == 0 {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::EmptyDataset,
			"target-free inference requires at least one row",
		));
	}
	let feature_width = u64::try_from(checkpoint.feature_width()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("saved dense feature width cannot be represented by u64: {error}"),
		)
	})?;
	if feature_width == 0 {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"saved dense feature width is zero",
		));
	}

	let mut compiler = InferenceGraphCompiler::new();
	let lowered = compiler.compile_features(
		prepared.data(),
		checkpoint.feature_spans(),
		rows,
		feature_width,
	)?;
	let mut current = compiler.apply_data_normalization(checkpoint, lowered, rows, feature_width)?;
	let mut current_width = feature_width;
	for (layer_index, layer) in checkpoint.layers().iter().enumerate() {
		current = compiler.compile_layer(
			layer_index,
			layer,
			current,
			rows,
			current_width,
			checkpoint.config().normalization_epsilon,
			checkpoint.config().reduction_tree_lanes,
		)?;
		current_width = layer.declaration().width().get();
	}
	let expected_width = u64::try_from(checkpoint.task().output_width()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("saved task output width cannot be represented by u64: {error}"),
		)
	})?;
	if current_width != expected_width {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			format!("effective flat layers produce width {current_width}, saved task requires {expected_width}"),
		));
	}
	if let Some(temperature) = checkpoint.temperature() {
		current = compiler.apply_temperature(current, temperature)?;
	}
	let (prediction, kind) = compiler.compile_prediction(
		current,
		checkpoint.task(),
		rows,
		current_width,
		checkpoint.config().reduction_tree_lanes,
	)?;
	compiler.finish(
		prediction,
		kind,
		rows,
		checkpoint.task(),
		checkpoint.output_adapter(),
	)
}

#[derive(Debug)]
struct InferenceGraphCompiler {
	tensors: BTreeMap<ValueId, Tensor>,
	nodes: Vec<CalculationNode>,
	domains: Vec<KernelIterationDomain>,
	next_value: u64,
	next_kernel: u64,
	external_inputs: Vec<InferenceExternalInput>,
	external_input_ids: BTreeSet<ValueId>,
}

impl InferenceGraphCompiler {
	fn new() -> Self {
		Self {
			tensors: BTreeMap::new(),
			nodes: Vec::new(),
			domains: Vec::new(),
			next_value: 1,
			next_kernel: 1,
			external_inputs: Vec::new(),
			external_input_ids: BTreeSet::new(),
		}
	}

	fn tensor(&mut self, dtype: DType, shape: Shape) -> InferenceCompileResult<ValueId> {
		let value = self.next_value()?;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false)?;
		self.tensors.insert(value, tensor);
		Ok(value)
	}

	fn external(
		&mut self,
		role: InferenceInputRole,
		dtype: DType,
		shape: Shape,
		bytes: Vec<u8>,
	) -> InferenceCompileResult<ValueId> {
		let expected = shape.bytes(dtype)?.get();
		if u64::try_from(bytes.len()).ok() != Some(expected) {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"{role:?} provides {} bytes, tensor contract requires {expected}",
					bytes.len()
				),
			));
		}
		let value = self.tensor(dtype, shape.clone())?;
		self.external_input_ids.insert(value);
		self.external_inputs.push(InferenceExternalInput {
			role,
			value,
			dtype,
			shape,
			bytes,
		});
		Ok(value)
	}

	fn external_checkpoint_tensor(
		&mut self,
		role: InferenceInputRole,
		image: &CheckpointTensorImage,
	) -> InferenceCompileResult<ValueId> {
		self.external(
			role,
			image.dtype(),
			shape(image.shape())?,
			image.bytes().to_vec(),
		)
	}

	fn next_value(&mut self) -> InferenceCompileResult<ValueId> {
		let value = self.next_value;
		self.next_value = value.checked_add(1).ok_or_else(identity_exhausted)?;
		Ok(ValueId::new(value))
	}

	fn next_kernel(&mut self) -> InferenceCompileResult<KernelTemplateId> {
		let kernel = self.next_kernel;
		self.next_kernel = kernel.checked_add(1).ok_or_else(identity_exhausted)?;
		Ok(KernelTemplateId::new(kernel))
	}

	fn tensor_ref(&self, value: ValueId) -> InferenceCompileResult<&Tensor> {
		self.tensors.get(&value).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::Language,
				format!("inference compiler tensor {value} is absent"),
			)
		})
	}

	fn emit(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		kind: PrimitiveKind,
		alias_rules: Vec<PrimitiveAliasRule>,
	) -> InferenceCompileResult<KernelTemplateId> {
		let id = self.next_kernel()?;
		self.nodes.push(CalculationNode {
			kernel: PrimitiveKernel {
				id,
				inputs,
				outputs,
				alias_rules,
				kind,
			},
		});
		self.domains.push(KernelIterationDomain {
			kernel: id,
			domain: IterationDomain::first(),
		});
		Ok(id)
	}

	fn emit_elementwise(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		program: recipe_core::ScalarProgram,
	) -> InferenceCompileResult<KernelTemplateId> {
		let aliases = forbidden_aliases(inputs.len(), outputs.len());
		self.emit(
			inputs,
			outputs,
			PrimitiveKind::Elementwise(Elementwise { program }),
			aliases,
		)
	}

	fn emit_owned_scalar(
		&mut self,
		symbol: &str,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
	) -> InferenceCompileResult<KernelTemplateId> {
		let descriptor = operation_registry().resolve_unique(symbol)?;
		let program = lower_scalar(descriptor)?;
		self.emit_elementwise(inputs, outputs, program)
	}

	fn reduce(
		&mut self,
		input: ValueId,
		output: ValueId,
		operator: ReduceOperator,
		axes: &[usize],
		keep_dimensions: bool,
		tree_lanes: u32,
	) -> InferenceCompileResult<()> {
		self.emit(
			vec![input],
			vec![output],
			PrimitiveKind::Reduce(Reduce {
				operator,
				axes: AxisSet::new(axes.to_vec())?,
				keep_dimensions,
				result: ReduceResult::Value,
				tree_lanes,
			}),
			forbidden_aliases(1, 1),
		)?;
		Ok(())
	}

	fn materialize(
		&mut self,
		symbol: &str,
		inputs: &[(&'static str, ValueId)],
		outputs: &[(&'static str, ValueId)],
		iteration_shape_input: &'static str,
	) -> InferenceCompileResult<()> {
		let mut input_tensors = inputs
			.iter()
			.map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<InferenceCompileResult<Vec<_>>>()?;
		for tensor in &mut input_tensors {
			tensor.external_input = true;
			tensor.external_output = false;
		}
		let mut output_tensors = outputs
			.iter()
			.map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<InferenceCompileResult<Vec<_>>>()?;
		for tensor in &mut output_tensors {
			tensor.external_input = false;
			tensor.external_output = true;
		}
		let named_inputs = inputs
			.iter()
			.zip(&input_tensors)
			.map(|((name, _), tensor)| NamedTensor::new(name, tensor))
			.collect::<Vec<_>>();
		let named_outputs = outputs
			.iter()
			.zip(&output_tensors)
			.map(|((name, _), tensor)| NamedTensor::new(name, tensor))
			.collect::<Vec<_>>();
		let first_value = self.next_value;
		let first_kernel = self.next_kernel;
		self.next_value = self
			.next_value
			.checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?;
		self.next_kernel = self
			.next_kernel
			.checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?;
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&named_inputs,
			&named_outputs,
			iteration_shape_input,
			&PreparedParameters::new(),
			IdentityNamespace::new(
				ValueId::new(first_value),
				MATERIALIZATION_RESERVATION,
				KernelTemplateId::new(first_kernel),
				MATERIALIZATION_RESERVATION,
			),
			WORKSPACE_LIMIT,
		))?;
		for tensor in &materialized.graph().tensors {
			self.insert_tensor_contract(tensor.clone())?;
		}
		for node in &materialized.graph().nodes {
			self.nodes.push(node.clone());
			self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id,
				domain: IterationDomain::first(),
			});
		}
		Ok(())
	}

	fn insert_tensor_contract(&mut self, mut tensor: Tensor) -> InferenceCompileResult<()> {
		tensor.external_input = false;
		tensor.external_output = false;
		match self.tensors.get(&tensor.id) {
			Some(existing)
				if existing.dtype == tensor.dtype
					&& existing.shape == tensor.shape
					&& existing.layout == tensor.layout
					&& existing.storage_bytes == tensor.storage_bytes =>
			{
				Ok(())
			}
			Some(_) => Err(InferenceCompileError::new(
				InferenceCompileErrorKind::Language,
				format!(
					"materialized tensor {} conflicts with an existing inference contract",
					tensor.id
				),
			)),
			None => {
				self.tensors.insert(tensor.id, tensor);
				Ok(())
			}
		}
	}

	fn compile_features(
		&mut self,
		data: &PreparedInferenceDataset,
		spans: &[CompiledFeatureSpan],
		rows: u64,
		feature_width: u64,
	) -> InferenceCompileResult<ValueId> {
		if data.features().len() != spans.len() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"prepared feature count differs from saved dense spans",
			));
		}
		let total_elements = rows.checked_mul(feature_width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"inference dense feature element count overflowed u64",
			)
		})?;
		require_i32_indexable(total_elements, "inference dense feature element count")?;
		let mut combined = self.zero_f32(shape(&[total_elements])?)?;
		let mut expected_start = 0_u64;
		for (feature_index, (feature, span)) in data.features().iter().zip(spans).enumerate() {
			let start = u64::try_from(span.start()).map_err(|error| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::UnsupportedExtent,
					format!("feature span start cannot be represented by u64: {error}"),
				)
			})?;
			let width = u64::try_from(span.width()).map_err(|error| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::UnsupportedExtent,
					format!("feature span width cannot be represented by u64: {error}"),
				)
			})?;
			if start != expected_start || width == 0 || start.checked_add(width).is_none() {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("saved feature span {feature_index} is not a nonempty contiguous dense span"),
				));
			}
			expected_start = start + width;
			if expected_start > feature_width
				|| feature.schema().source_vector() != span.source_vector()
				|| feature.values().len() != data.rows()
			{
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("prepared feature {feature_index} disagrees with its saved dense span"),
				));
			}
			let raw_shape = shape(&[rows])?;
			let (dtype, bytes) = inference_feature_bytes(feature.values());
			let raw = self.external(
				InferenceInputRole::Feature {
					feature: feature_index,
					source_vector: span.source_vector(),
				},
				dtype,
				raw_shape,
				bytes,
			)?;
			let block = match (span.lowering(), feature.values()) {
				(DenseFeatureLowering::NumericScalar, PreparedInferenceValues::I32(_)) if width == 1 => {
					let converted = self.tensor(DType::F32, shape(&[rows])?)?;
					self.emit_elementwise(vec![raw], vec![converted], checked_i32_to_f32_program()?)?;
					converted
				}
				(DenseFeatureLowering::NumericScalar, PreparedInferenceValues::F32Bits(_)) if width == 1 => raw,
				(
					DenseFeatureLowering::CategoricalOneHot {
						dictionary_width,
						reserved_index,
					},
					PreparedInferenceValues::I32(_),
				) if dictionary_width.checked_add(1) == Some(span.width())
					&& reserved_index == dictionary_width =>
				{
					self.one_hot(raw, rows, width)?
				}
				_ => {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						format!("prepared feature {feature_index} has no matching saved lowering"),
					));
				}
			};
			combined = self.scatter_feature_block(combined, block, rows, feature_width, start, width)?;
		}
		if expected_start != feature_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"saved feature spans end at width {expected_start}, checkpoint declares {feature_width}"
				),
			));
		}
		let dense_shape = shape(&[rows, feature_width])?;
		let identity = self.tensor(DType::I32, dense_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![identity],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
		let dense = self.tensor(DType::F32, dense_shape)?;
		self.emit(
			vec![combined, identity],
			vec![dense],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(dense)
	}

	fn zero_f32(&mut self, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let seed = self.tensor(DType::I32, output_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(vec![seed], vec![output], zero_f32_program()?)?;
		Ok(output)
	}

	fn one_hot(&mut self, labels: ValueId, rows: u64, classes: u64) -> InferenceCompileResult<ValueId> {
		let classes_i32 = checked_i32(classes, "categorical one-hot width")?;
		let row_shape = shape(&[rows])?;
		let row_bases = self.tensor(DType::I32, row_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![row_bases],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: classes_i32,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
		let destinations = self.tensor(DType::I32, row_shape.clone())?;
		let updates = self.tensor(DType::F32, row_shape)?;
		self.emit_elementwise(
			vec![labels, row_bases],
			vec![destinations, updates],
			checked_one_hot_update_program(classes_i32)?,
		)?;
		let elements = rows.checked_mul(classes).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"categorical one-hot element count overflowed u64",
			)
		})?;
		require_i32_indexable(elements, "categorical one-hot element count")?;
		let base = self.zero_f32(shape(&[elements])?)?;
		let encoded = self.tensor(DType::F32, shape(&[elements])?)?;
		self.emit(
			vec![base, destinations, updates],
			vec![encoded],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
		)?;
		Ok(encoded)
	}

	fn scatter_feature_block(
		&mut self,
		base: ValueId,
		block: ValueId,
		rows: u64,
		total_width: u64,
		start: u64,
		block_width: u64,
	) -> InferenceCompileResult<ValueId> {
		let elements = rows.checked_mul(block_width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"feature block element count overflowed u64",
			)
		})?;
		require_i32_indexable(elements, "feature block element count")?;
		let positions = self.tensor(DType::I32, shape(&[elements])?)?;
		self.emit(
			Vec::new(),
			vec![positions],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
		let destinations = self.tensor(DType::I32, shape(&[elements])?)?;
		self.emit_elementwise(
			vec![positions],
			vec![destinations],
			feature_destination_program(
				checked_i32(total_width, "dense feature width")?,
				checked_i32(start, "feature span start")?,
				checked_i32(block_width, "feature span width")?,
			)?,
		)?;
		let output_shape = self.tensor_ref(base)?.shape.clone();
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit(
			vec![base, destinations, block],
			vec![output],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
		)?;
		Ok(output)
	}

	fn apply_data_normalization(
		&mut self,
		checkpoint: &CheckpointArtifact,
		input: ValueId,
		rows: u64,
		columns: u64,
	) -> InferenceCompileResult<ValueId> {
		let mask = self.external(
			InferenceInputRole::FeatureNormalizationMask,
			DType::F32,
			shape(&[columns])?,
			checkpoint
				.feature_normalization_mask()
				.iter()
				.flat_map(|bits| bits.to_le_bytes())
				.collect(),
		)?;
		let matrix_shape = shape(&[rows, columns])?;
		match checkpoint.config().data_normalization {
			DenseDataNormalization::ZScore => {
				let [mean, variance] = checkpoint.normalization() else {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"z-score checkpoint does not retain exactly mean and variance tensors",
					));
				};
				let mean = self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationMean, mean)?;
				let variance =
					self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationVariance, variance)?;
				let output = self.tensor(DType::F32, matrix_shape)?;
				self.emit_elementwise(
					vec![input, mean, variance, mask],
					vec![output],
					z_score_program(checkpoint.config().normalization_epsilon, true)?,
				)?;
				Ok(output)
			}
			DenseDataNormalization::MinMax => {
				let [minimum, maximum] = checkpoint.normalization() else {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"min-max checkpoint does not retain exactly minimum and maximum tensors",
					));
				};
				let minimum =
					self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationMinimum, minimum)?;
				let maximum =
					self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationMaximum, maximum)?;
				let output = self.tensor(DType::F32, matrix_shape)?;
				self.emit_elementwise(
					vec![input, minimum, maximum, mask],
					vec![output],
					min_max_program(checkpoint.config().normalization_epsilon, true)?,
				)?;
				Ok(output)
			}
			DenseDataNormalization::L2Norm => {
				if !checkpoint.normalization().is_empty() {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"L2 checkpoint unexpectedly retains fitted normalization tensors",
					));
				}
				let squares = self.tensor(DType::F32, matrix_shape.clone())?;
				self.emit_elementwise(vec![input, mask], vec![squares], l2_square_program(true)?)?;
				let norms_squared = self.tensor(DType::F32, shape(&[rows, 1])?)?;
				self.reduce(
					squares,
					norms_squared,
					ReduceOperator::Sum,
					&[1],
					true,
					checkpoint.config().reduction_tree_lanes,
				)?;
				let output = self.tensor(DType::F32, matrix_shape)?;
				self.emit_elementwise(
					vec![input, norms_squared, mask],
					vec![output],
					l2_norm_program(checkpoint.config().normalization_epsilon, true)?,
				)?;
				Ok(output)
			}
		}
	}

	fn compile_layer(
		&mut self,
		layer_index: usize,
		layer: &crate::CheckpointLayerImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		normalization_epsilon: f32,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let output_width = layer.declaration().width().get();
		validate_checkpoint_parameter_image(
			layer.weight().parameter(),
			&[input_width, output_width],
			"layer weight",
		)?;
		validate_checkpoint_parameter_image(layer.bias().parameter(), &[output_width], "layer bias")?;
		let weight = self.external_checkpoint_tensor(
			InferenceInputRole::LayerWeight { layer: layer_index },
			layer.weight().parameter(),
		)?;
		let bias = self.external_checkpoint_tensor(
			InferenceInputRole::LayerBias { layer: layer_index },
			layer.bias().parameter(),
		)?;
		let output_shape = shape(&[rows, output_width])?;
		let preactivation = self.tensor(DType::F32, output_shape.clone())?;
		self.materialize(
			"gpu_linear_into",
			&[("input", input), ("weight", weight), ("bias", bias)],
			&[("output", preactivation)],
			"input",
		)?;
		let mut current = preactivation;
		for operation in layer.declaration().operations().iter().copied() {
			current = match operation {
				DenseOperation::Activation(activation) => {
					self.apply_activation(current, activation, output_shape.clone())?
				}
				DenseOperation::Normalization(normalization) => self.apply_model_normalization(
					current,
					normalization,
					rows,
					output_width,
					normalization_epsilon,
					tree_lanes,
				)?,
			};
		}
		Ok(current)
	}

	fn apply_activation(
		&mut self,
		input: ValueId,
		activation: DenseActivation,
		output_shape: Shape,
	) -> InferenceCompileResult<ValueId> {
		if activation == DenseActivation::Linear {
			return Ok(input);
		}
		let output = self.tensor(DType::F32, output_shape.clone())?;
		match activation {
			DenseActivation::Linear => {}
			DenseActivation::Cosine => {
				self.emit_owned_scalar("gpu_cos", vec![input], vec![output])?;
			}
			DenseActivation::Exponential => {
				self.emit_owned_scalar("gpu_exp", vec![input], vec![output])?;
			}
			DenseActivation::Logarithm => {
				let absolute = self.tensor(DType::F32, output_shape.clone())?;
				self.emit_owned_scalar("gpu_abs_into", vec![input], vec![absolute])?;
				let magnitude = self.tensor(DType::F32, output_shape.clone())?;
				self.emit_owned_scalar("gpu_log1p", vec![absolute], vec![magnitude])?;
				let sign = self.tensor(DType::F32, output_shape)?;
				self.emit_owned_scalar("gpu_sign_into", vec![input], vec![sign])?;
				self.emit_elementwise(vec![sign, magnitude], vec![output], multiply_program()?)?;
			}
			DenseActivation::Huber => {
				self.emit_elementwise(vec![input], vec![output], huber_activation_program()?)?;
			}
			DenseActivation::Tangent => {
				self.emit_owned_scalar("gpu_tan", vec![input], vec![output])?;
			}
			DenseActivation::Relu => {
				self.emit_owned_scalar("gpu_relu_into", vec![input], vec![output])?;
			}
			DenseActivation::Gelu => {
				self.emit_owned_scalar("gpu_gelu_into", vec![input], vec![output])?;
			}
			DenseActivation::Silu => {
				self.emit_owned_scalar("gpu_silu_into", vec![input], vec![output])?;
			}
		}
		Ok(output)
	}

	fn apply_model_normalization(
		&mut self,
		input: ValueId,
		normalization: DenseNormalization,
		rows: u64,
		columns: u64,
		epsilon: f32,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let (axis, statistic_shape, population, keep_dimensions) = match normalization {
			DenseNormalization::Layer => (1, shape(&[rows, 1])?, columns as f32, true),
			DenseNormalization::Batch => (0, shape(&[columns])?, rows as f32, false),
		};
		let matrix_shape = shape(&[rows, columns])?;
		let sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce(
			input,
			sums,
			ReduceOperator::Sum,
			&[axis],
			keep_dimensions,
			tree_lanes,
		)?;
		let means = self.tensor(DType::F32, statistic_shape.clone())?;
		self.emit_elementwise(
			vec![sums],
			vec![means],
			divide_constant_program(population)?,
		)?;
		let centered = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![input, means], vec![centered], subtract_program()?)?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![centered], vec![squares], square_program()?)?;
		let variance_sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce(
			squares,
			variance_sums,
			ReduceOperator::Sum,
			&[axis],
			keep_dimensions,
			tree_lanes,
		)?;
		let variance = self.tensor(DType::F32, statistic_shape)?;
		self.emit_elementwise(
			vec![variance_sums],
			vec![variance],
			divide_constant_program(population)?,
		)?;
		let normalized = self.tensor(DType::F32, matrix_shape)?;
		self.emit_elementwise(
			vec![input, means, variance],
			vec![normalized],
			z_score_program(epsilon, false)?,
		)?;
		Ok(normalized)
	}

	fn apply_temperature(
		&mut self,
		logits: ValueId,
		temperature: &CheckpointTensorImage,
	) -> InferenceCompileResult<ValueId> {
		validate_checkpoint_parameter_image(temperature, &[1], "temperature")?;
		let temperature = self.external_checkpoint_tensor(InferenceInputRole::Temperature, temperature)?;
		let output_shape = self.tensor_ref(logits)?.shape.clone();
		let scaled = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(vec![logits, temperature], vec![scaled], divide_program()?)?;
		Ok(scaled)
	}

	fn compile_prediction(
		&mut self,
		values: ValueId,
		task: DenseTask,
		rows: u64,
		columns: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<(ValueId, InferencePredictionKind)> {
		match task {
			DenseTask::BinaryClassification { .. } => {
				if columns != 1 {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"binary inference logits do not have width one",
					));
				}
				let probabilities = self.stable_sigmoid(values)?;
				Ok((probabilities, InferencePredictionKind::BinaryProbability))
			}
			DenseTask::MulticlassClassification { class_count, .. } => {
				if u64::try_from(class_count).ok() != Some(columns) {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"multiclass inference logit width differs from saved class count",
					));
				}
				let probabilities = self.stable_softmax(values, rows, columns, tree_lanes)?;
				Ok((
					probabilities,
					InferencePredictionKind::MulticlassProbabilities,
				))
			}
			DenseTask::ScalarRegression { .. } => {
				if columns != 1 {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"scalar regression output does not have width one",
					));
				}
				Ok((values, InferencePredictionKind::Regression))
			}
		}
	}

	fn stable_sigmoid(&mut self, logits: ValueId) -> InferenceCompileResult<ValueId> {
		let output_shape = self.tensor_ref(logits)?.shape.clone();
		let exponent_argument = self.tensor(DType::F32, output_shape.clone())?;
		self.emit_elementwise(
			vec![logits],
			vec![exponent_argument],
			stable_sigmoid_exponent_program()?,
		)?;
		let exponent = self.tensor(DType::F32, output_shape.clone())?;
		self.emit_elementwise(
			vec![exponent_argument],
			vec![exponent],
			recipe_math::exp_with_gradual_underflow_program()?,
		)?;
		let probabilities = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(
			vec![logits, exponent],
			vec![probabilities],
			stable_sigmoid_result_program()?,
		)?;
		Ok(probabilities)
	}

	fn stable_softmax(
		&mut self,
		logits: ValueId,
		rows: u64,
		classes: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let matrix_shape = shape(&[rows, classes])?;
		let row_shape = shape(&[rows, 1])?;
		let maximum = self.tensor(DType::F32, row_shape.clone())?;
		self.reduce(
			logits,
			maximum,
			ReduceOperator::Maximum,
			&[1],
			true,
			tree_lanes,
		)?;
		let shifted = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![logits, maximum], vec![shifted], subtract_program()?)?;
		let exponent_input = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![shifted],
			vec![exponent_input],
			softmax_exponent_input_program()?,
		)?;
		let exponentials = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![exponent_input],
			vec![exponentials],
			recipe_math::exp_with_gradual_underflow_program()?,
		)?;
		let exponential_sum = self.tensor(DType::F32, row_shape)?;
		self.reduce(
			exponentials,
			exponential_sum,
			ReduceOperator::Sum,
			&[1],
			true,
			tree_lanes,
		)?;
		let probabilities = self.tensor(DType::F32, matrix_shape)?;
		self.emit_elementwise(
			vec![exponentials, exponential_sum],
			vec![probabilities],
			divide_program()?,
		)?;
		Ok(probabilities)
	}

	fn finish(
		mut self,
		prediction: ValueId,
		kind: InferencePredictionKind,
		rows: u64,
		task: DenseTask,
		output_adapter: Option<DenseOutputAdapter>,
	) -> InferenceCompileResult<CompiledInference> {
		let output_tensor = self.tensor_ref(prediction)?.clone();
		for tensor in self.tensors.values_mut() {
			tensor.external_input = self.external_input_ids.contains(&tensor.id);
			tensor.external_output = tensor.id == prediction;
		}
		let graph = CalculationGraph {
			tensors: self.tensors.into_values().collect(),
			nodes: self.nodes,
		};
		graph.validate()?;
		let canonical = graph.to_ogdl()?;
		let graph = CalculationGraph::from_ogdl(&canonical)?;
		let iterations = NonZeroU64::new(1).expect("one inference iteration is nonzero");
		let program = StaticCalculationProgram::new(graph, iterations, self.domains)?;
		let program_text = program.to_ogdl()?;
		let program = StaticCalculationProgram::from_ogdl(&program_text)?;
		Ok(CompiledInference {
			program,
			external_inputs: self.external_inputs,
			output: InferenceOutputContract {
				value: prediction,
				dtype: output_tensor.dtype,
				shape: output_tensor.shape,
				kind,
			},
			rows,
			task,
			output_adapter,
		})
	}
}

fn inference_feature_bytes(values: &PreparedInferenceValues) -> (DType, Vec<u8>) {
	match values {
		PreparedInferenceValues::I32(values) => (
			DType::I32,
			values.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		),
		PreparedInferenceValues::F32Bits(values) => (
			DType::F32,
			values.iter().flat_map(|bits| bits.to_le_bytes()).collect(),
		),
	}
}

fn validate_checkpoint_parameter_image(
	image: &CheckpointTensorImage,
	expected_shape: &[u64],
	role: &str,
) -> InferenceCompileResult<()> {
	if image.dtype() != DType::F32 || image.shape() != expected_shape {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			format!(
				"{role} has {:?} shape {:?}, expected F32 shape {expected_shape:?}",
				image.dtype(),
				image.shape()
			),
		));
	}
	let expected_bytes = expected_shape
		.iter()
		.try_fold(1_u64, |elements, extent| elements.checked_mul(*extent))
		.and_then(|elements| elements.checked_mul(u64::from(DType::F32.byte_width())))
		.ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				format!("{role} byte count overflowed u64"),
			)
		})?;
	if u64::try_from(image.bytes().len()).ok() != Some(expected_bytes) {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			format!(
				"{role} provides {} bytes, expected {expected_bytes}",
				image.bytes().len()
			),
		));
	}
	Ok(())
}

fn shape(extents: &[u64]) -> InferenceCompileResult<Shape> {
	Ok(Shape::new(extents.to_vec())?)
}

fn checked_i32(value: u64, name: &str) -> InferenceCompileResult<i32> {
	i32::try_from(value).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("{name} {value} cannot be represented by int32: {error}"),
		)
	})
}

fn require_i32_indexable(value: u64, name: &str) -> InferenceCompileResult<()> {
	if value == 0 || value > i32::MAX as u64 {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("{name} {value} must fit the nonempty checked int32 index domain"),
		));
	}
	Ok(())
}

fn identity_exhausted() -> InferenceCompileError {
	InferenceCompileError::new(
		InferenceCompileErrorKind::IdentityExhausted,
		"deterministic inference graph identity space exhausted",
	)
}

fn forbidden_aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.flat_map(|input| {
			(0..outputs).map(move |output| PrimitiveAliasRule {
				input,
				output,
				permission: AliasPermission::Forbidden,
			})
		})
		.collect()
}

fn zero_f32_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let _seed = builder.input(DType::I32)?;
	let zero = builder.f32(0.0)?;
	Ok(builder.finish(&[zero])?)
}

fn checked_i32_to_f32_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let input = builder.input(DType::I32)?;
	let converted = builder.unary(ScalarOpcode::ConvertI32ToF32, input)?;
	let round_trip = builder.unary(ScalarOpcode::ConvertF32ToI32, converted)?;
	let exact = builder.binary(ScalarOpcode::Equal, input, round_trip)?;
	let maximum = builder.i32(i32::MAX)?;
	let below_saturating_hole = builder.binary(ScalarOpcode::NotEqual, input, maximum)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, exact, below_saturating_hole)?;
	let _ = builder.unary(ScalarOpcode::Require, valid)?;
	Ok(builder.finish(&[converted])?)
}

fn stable_sigmoid_exponent_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let logit = builder.input(DType::F32)?;
	let magnitude = builder.unary(ScalarOpcode::Absolute, logit)?;
	let exponent_argument = builder.unary(ScalarOpcode::Negate, magnitude)?;
	Ok(builder.finish(&[exponent_argument])?)
}

fn stable_sigmoid_result_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let logit = builder.input(DType::F32)?;
	let exponent = builder.input(DType::F32)?;
	let zero = builder.f32(0.0)?;
	let one = builder.f32(1.0)?;
	let denominator = builder.binary(ScalarOpcode::Add, one, exponent)?;
	let positive = builder.binary(ScalarOpcode::Divide, one, denominator)?;
	let negative = builder.binary(ScalarOpcode::Divide, exponent, denominator)?;
	let nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, logit, zero)?;
	let probability = builder.ternary(ScalarOpcode::Select, nonnegative, positive, negative)?;
	Ok(builder.finish(&[probability])?)
}

fn softmax_exponent_input_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let shifted = builder.input(DType::F32)?;
	let zero = builder.f32(0.0)?;
	let nonpositive = builder.binary(ScalarOpcode::LessThanOrEqual, shifted, zero)?;
	let _ = builder.unary(ScalarOpcode::Require, nonpositive)?;
	let finite = builder.unary(ScalarOpcode::IsFinite, shifted)?;
	let true_underflow = builder.f32(-104.0)?;
	let exponent_input = builder.ternary(ScalarOpcode::Select, finite, shifted, true_underflow)?;
	Ok(builder.finish(&[exponent_input])?)
}

fn checked_one_hot_update_program(classes: i32) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let label = builder.input(DType::I32)?;
	let row_base = builder.input(DType::I32)?;
	let zero = builder.i32(0)?;
	let classes = builder.i32(classes)?;
	let nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, label, zero)?;
	let below_width = builder.binary(ScalarOpcode::LessThan, label, classes)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, nonnegative, below_width)?;
	let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let destination = builder.binary(ScalarOpcode::Add, row_base, label)?;
	let one = builder.f32(1.0)?;
	Ok(builder.finish(&[destination, one])?)
}

fn feature_destination_program(
	total_width: i32,
	start: i32,
	block_width: i32,
) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let block_width = builder.i32(block_width)?;
	let row = builder.binary(ScalarOpcode::Divide, position, block_width)?;
	let column = builder.binary(ScalarOpcode::Remainder, position, block_width)?;
	let total_width = builder.i32(total_width)?;
	let row_offset = builder.binary(ScalarOpcode::Multiply, row, total_width)?;
	let start = builder.i32(start)?;
	let destination = builder.binary(ScalarOpcode::Add, row_offset, start)?;
	let destination = builder.binary(ScalarOpcode::Add, destination, column)?;
	Ok(builder.finish(&[destination])?)
}

fn multiply_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn divide_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let numerator = builder.input(DType::F32)?;
	let denominator = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
	Ok(builder.finish(&[result])?)
}

fn subtract_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Subtract, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn square_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, value, value)?;
	Ok(builder.finish(&[result])?)
}

fn divide_constant_program(divisor: f32) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let divisor = builder.f32(divisor)?;
	let result = builder.binary(ScalarOpcode::Divide, value, divisor)?;
	Ok(builder.finish(&[result])?)
}

fn z_score_program(epsilon: f32, masked: bool) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mean = builder.input(DType::F32)?;
	let variance = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let epsilon = builder.f32(epsilon)?;
	let variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, variance)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, mean)?;
	let mut normalized = builder.binary(ScalarOpcode::Divide, centered, standard_deviation)?;
	if let Some(mask) = mask {
		let zero = builder.f32(0.0)?;
		let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
		normalized = builder.ternary(ScalarOpcode::Select, normalize, normalized, value)?;
	}
	Ok(builder.finish(&[normalized])?)
}

fn min_max_program(epsilon: f32, masked: bool) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let minimum = builder.input(DType::F32)?;
	let maximum = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let range = builder.binary(ScalarOpcode::Subtract, maximum, minimum)?;
	let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, range, epsilon)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, minimum)?;
	let mut normalized = builder.binary(ScalarOpcode::Divide, centered, denominator)?;
	if let Some(mask) = mask {
		let zero = builder.f32(0.0)?;
		let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
		normalized = builder.ternary(ScalarOpcode::Select, normalize, normalized, value)?;
	}
	Ok(builder.finish(&[normalized])?)
}

fn l2_square_program(masked: bool) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let mut square = builder.binary(ScalarOpcode::Multiply, value, value)?;
	if let Some(mask) = mask {
		square = builder.binary(ScalarOpcode::Multiply, square, mask)?;
	}
	Ok(builder.finish(&[square])?)
}

fn l2_norm_program(epsilon: f32, masked: bool) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let norm_squared = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let epsilon = builder.f32(epsilon)?;
	let norm_squared = builder.binary(ScalarOpcode::Maximum, norm_squared, epsilon)?;
	let norm = builder.unary(ScalarOpcode::SquareRoot, norm_squared)?;
	let mut normalized = builder.binary(ScalarOpcode::Divide, value, norm)?;
	if let Some(mask) = mask {
		let zero = builder.f32(0.0)?;
		let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
		normalized = builder.ternary(ScalarOpcode::Select, normalize, normalized, value)?;
	}
	Ok(builder.finish(&[normalized])?)
}

fn huber_activation_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let absolute = builder.unary(ScalarOpcode::Absolute, value)?;
	let one = builder.f32(1.0)?;
	let quadratic_domain = builder.binary(ScalarOpcode::LessThanOrEqual, absolute, one)?;
	let square = builder.binary(ScalarOpcode::Multiply, value, value)?;
	let half = builder.f32(0.5)?;
	let quadratic = builder.binary(ScalarOpcode::Multiply, half, square)?;
	let linear = builder.binary(ScalarOpcode::Subtract, absolute, half)?;
	let result = builder.ternary(ScalarOpcode::Select, quadratic_domain, quadratic, linear)?;
	Ok(builder.finish(&[result])?)
}

fn saved_feature_schema(checkpoint: &CheckpointArtifact) -> InferencePreparationResult<Vec<InferenceFeatureSchema>> {
	checkpoint
		.feature_spans()
		.iter()
		.enumerate()
		.map(|(feature, span)| {
			let vector = checkpoint
				.vectors()
				.iter()
				.find(|vector| vector.source_index() == span.source_vector())
				.ok_or_else(|| inconsistent_feature(feature, span, "saved feature vector is absent"))?;
			let encoding = match (
				span.lowering(),
				vector.semantic_type(),
				vector.encoding(),
				vector.metadata(),
			) {
				(
					DenseFeatureLowering::NumericScalar,
					SemanticType::Numeric,
					VectorEncoding::I32,
					CheckpointArtifactMetadata::None,
				) => InferenceFeatureEncoding::NumericI32,
				(
					DenseFeatureLowering::NumericScalar,
					SemanticType::Numeric,
					VectorEncoding::F32,
					CheckpointArtifactMetadata::None,
				) => InferenceFeatureEncoding::NumericF32,
				(
					DenseFeatureLowering::CategoricalOneHot {
						dictionary_width,
						reserved_index,
					},
					SemanticType::Categorical,
					VectorEncoding::DictionaryI32,
					CheckpointArtifactMetadata::Categorical { dictionary },
				) if dictionary_width == dictionary.len()
					&& reserved_index == dictionary.len()
					&& span.width() == dictionary.len().saturating_add(1) =>
				{
					InferenceFeatureEncoding::CategoricalDictionary {
						dictionary: dictionary.clone(),
					}
				}
				_ => {
					return Err(inconsistent_feature(
						feature,
						span,
						"saved vector schema and dense lowering are inconsistent",
					));
				}
			};
			Ok(InferenceFeatureSchema::new(
				span.source_vector(),
				vector.name(),
				encoding,
			))
		})
		.collect()
}

fn validate_prepared_feature_spans(
	prepared: &PreparedInferenceDataset,
	spans: &[CompiledFeatureSpan],
) -> InferencePreparationResult<()> {
	if prepared.features().len() != spans.len() {
		return Err(InferencePreparationError::InconsistentCheckpoint {
			feature: prepared.features().len().min(spans.len()),
			source_vector: spans
				.get(prepared.features().len().min(spans.len()))
				.map_or(0, CompiledFeatureSpan::source_vector),
			detail: "prepared feature count differs from the saved span count".to_owned(),
		});
	}
	for (feature, (prepared, span)) in prepared.features().iter().zip(spans).enumerate() {
		if prepared.schema().source_vector() != span.source_vector() {
			return Err(inconsistent_feature(
				feature,
				span,
				"prepared feature identity differs from the saved span",
			));
		}
		match (
			span.lowering(),
			prepared.schema().encoding(),
			prepared.values(),
		) {
			(
				DenseFeatureLowering::NumericScalar,
				InferenceFeatureEncoding::NumericI32,
				PreparedInferenceValues::I32(_),
			)
			| (
				DenseFeatureLowering::NumericScalar,
				InferenceFeatureEncoding::NumericF32,
				PreparedInferenceValues::F32Bits(_),
			) if span.width() == 1 => {}
			(
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				},
				InferenceFeatureEncoding::CategoricalDictionary { dictionary },
				PreparedInferenceValues::I32(_),
			) if dictionary_width == dictionary.len()
				&& reserved_index == dictionary.len()
				&& span.width() == dictionary.len().saturating_add(1) => {}
			_ => {
				return Err(inconsistent_feature(
					feature,
					span,
					"prepared saved-schema values and feature span are inconsistent",
				));
			}
		}
	}
	Ok(())
}

fn inconsistent_feature(
	feature: usize,
	span: &CompiledFeatureSpan,
	detail: impl Into<String>,
) -> InferencePreparationError {
	InferencePreparationError::InconsistentCheckpoint {
		feature,
		source_vector: span.source_vector(),
		detail: detail.into(),
	}
}

#[cfg(test)]
pub(crate) mod test_support {
	use recipe_language::{ContiguousOrder, TensorLayout};

	use super::*;

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub(crate) enum InferenceBoundaryFault {
		Valid,
		DuplicateSemanticRole,
		InputIsUnproducedPrediction,
		NonCanonicalInputLayout,
		PaddedInputStorage,
		NonCanonicalOutputLayout,
		PaddedOutputStorage,
		I32Prediction,
	}

	pub(crate) fn inference_boundary_allowed_roles() -> [InferenceInputRole; 9] {
		[
			InferenceInputRole::Feature {
				feature: 0,
				source_vector: 0,
			},
			InferenceInputRole::FeatureNormalizationMask,
			InferenceInputRole::DataNormalizationMean,
			InferenceInputRole::DataNormalizationVariance,
			InferenceInputRole::DataNormalizationMinimum,
			InferenceInputRole::DataNormalizationMaximum,
			InferenceInputRole::LayerWeight { layer: 0 },
			InferenceInputRole::LayerBias { layer: 0 },
			InferenceInputRole::Temperature,
		]
	}

	pub(crate) fn compiled_inference_boundary_role_fixture(role: InferenceInputRole) -> CompiledInference {
		let role = match role {
			InferenceInputRole::Feature { .. }
			| InferenceInputRole::FeatureNormalizationMask
			| InferenceInputRole::DataNormalizationMean
			| InferenceInputRole::DataNormalizationVariance
			| InferenceInputRole::DataNormalizationMinimum
			| InferenceInputRole::DataNormalizationMaximum
			| InferenceInputRole::LayerWeight { .. }
			| InferenceInputRole::LayerBias { .. }
			| InferenceInputRole::Temperature => role,
		};
		compiled_inference_boundary_fixture_with_role(InferenceBoundaryFault::Valid, role)
	}

	pub(crate) fn compiled_inference_boundary_fixture(fault: InferenceBoundaryFault) -> CompiledInference {
		compiled_inference_boundary_fixture_with_role(
			fault,
			InferenceInputRole::Feature {
				feature: 0,
				source_vector: 0,
			},
		)
	}

	fn compiled_inference_boundary_fixture_with_role(
		fault: InferenceBoundaryFault,
		role: InferenceInputRole,
	) -> CompiledInference {
		let mut compiler = InferenceGraphCompiler::new();
		let dtype = match fault {
			InferenceBoundaryFault::I32Prediction => DType::I32,
			_ => DType::F32,
		};
		let shape = Shape::new(vec![2, 1]).unwrap();
		let bytes = vec![0; usize::try_from(shape.bytes(dtype).unwrap().get()).unwrap()];
		let input = compiler
			.external(role, dtype, shape.clone(), bytes.clone())
			.unwrap();

		if fault == InferenceBoundaryFault::DuplicateSemanticRole {
			compiler
				.external(role, dtype, shape.clone(), bytes)
				.unwrap();
		}
		if fault == InferenceBoundaryFault::NonCanonicalInputLayout {
			compiler.tensors.get_mut(&input).unwrap().layout =
				TensorLayout::contiguous(&shape, ContiguousOrder::ColumnMajor).unwrap();
		}
		if fault == InferenceBoundaryFault::PaddedInputStorage {
			compiler.tensors.get_mut(&input).unwrap().storage_bytes =
				ByteCount::new(shape.bytes(dtype).unwrap().get() + u64::from(dtype.byte_width()));
		}

		let prediction = if fault == InferenceBoundaryFault::InputIsUnproducedPrediction {
			input
		} else {
			let prediction = compiler.tensor(dtype, shape.clone()).unwrap();
			if fault == InferenceBoundaryFault::NonCanonicalOutputLayout {
				compiler.tensors.get_mut(&prediction).unwrap().layout =
					TensorLayout::contiguous(&shape, ContiguousOrder::ColumnMajor).unwrap();
			}
			if fault == InferenceBoundaryFault::PaddedOutputStorage {
				compiler.tensors.get_mut(&prediction).unwrap().storage_bytes =
					ByteCount::new(shape.bytes(dtype).unwrap().get() + u64::from(dtype.byte_width()));
			}
			let mut builder = ScalarProgramBuilder::new().unwrap();
			let value = builder.input(dtype).unwrap();
			let program = builder.finish(&[value]).unwrap();
			compiler
				.emit_elementwise(vec![input], vec![prediction], program)
				.unwrap();
			prediction
		};

		compiler
			.finish(
				prediction,
				InferencePredictionKind::Regression,
				2,
				DenseTask::ScalarRegression { target_vector: 0 },
				None,
			)
			.unwrap()
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::fs;
	use std::sync::atomic::{AtomicU64, Ordering};

	use recipe_core::{ScalarLiteral, ScalarProgram, ScalarValueId};
	use recipe_ingest::{
		Delimiter, HeaderMode, InferencePrepareErrorKind, IngestLimits, TableRequest, distill_dataset, parse_table,
	};

	use super::*;

	#[derive(Debug)]
	struct TestPath(std::path::PathBuf);

	impl TestPath {
		fn new(name: &str) -> Self {
			static NEXT: AtomicU64 = AtomicU64::new(1);
			let path = std::env::temp_dir().join(format!(
				"recipe-training-inference-{}-{}-{name}",
				std::process::id(),
				NEXT.fetch_add(1, Ordering::Relaxed)
			));
			Self(path)
		}
	}

	impl Drop for TestPath {
		fn drop(&mut self) {
			let _ = fs::remove_file(&self.0);
			let _ = fs::remove_dir_all(&self.0);
		}
	}

	fn table(source: &[u8]) -> RawTable {
		parse_table(
			source,
			TableRequest::new(
				Delimiter::Comma,
				HeaderMode::Present,
				IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
			),
		)
		.unwrap()
	}

	#[derive(Clone, Copy, Debug)]
	enum TestScalar {
		F32(f32),
		I32(i32),
	}

	impl TestScalar {
		fn f32(self) -> f32 {
			match self {
				Self::F32(value) => value,
				Self::I32(_) => panic!("expected f32 test value"),
			}
		}

		fn i32(self) -> i32 {
			match self {
				Self::I32(value) => value,
				Self::F32(_) => panic!("expected i32 test value"),
			}
		}
	}

	#[derive(Clone, Copy, Debug)]
	struct TestEvaluation {
		output: f32,
		faulted: bool,
	}

	fn evaluate_test_program(program: &ScalarProgram, inputs: &[f32]) -> TestEvaluation {
		assert_eq!(program.inputs.len(), inputs.len());
		let mut values = BTreeMap::<ScalarValueId, TestScalar>::new();
		for (input, value) in program.inputs.iter().zip(inputs) {
			assert_eq!(input.dtype, DType::F32);
			values.insert(input.id, TestScalar::F32(*value));
		}
		for constant in &program.constants {
			let value = match constant.value {
				ScalarLiteral::F32Bits(bits) => TestScalar::F32(f32::from_bits(bits)),
				ScalarLiteral::I32(value) => TestScalar::I32(value),
			};
			values.insert(constant.id, value);
		}

		let mut faulted = false;
		for instruction in &program.instructions {
			let operands = instruction
				.operands
				.iter()
				.map(|operand| values[operand])
				.collect::<Vec<_>>();
			let unary = || operands[0];
			let binary = || (operands[0], operands[1]);
			let value = match instruction.opcode {
				ScalarOpcode::Add => {
					let (left, right) = binary();
					match (left, right) {
						(TestScalar::F32(left), TestScalar::F32(right)) => TestScalar::F32(left + right),
						(TestScalar::I32(left), TestScalar::I32(right)) => {
							TestScalar::I32(left.wrapping_add(right))
						}
						_ => panic!("mixed test add"),
					}
				}
				ScalarOpcode::Multiply => {
					let (left, right) = binary();
					TestScalar::F32(left.f32() * right.f32())
				}
				ScalarOpcode::Divide => {
					let (left, right) = binary();
					TestScalar::F32(left.f32() / right.f32())
				}
				ScalarOpcode::Negate => TestScalar::F32(-unary().f32()),
				ScalarOpcode::Absolute => TestScalar::F32(unary().f32().abs()),
				ScalarOpcode::Maximum => {
					let (left, right) = binary();
					TestScalar::F32(left.f32().max(right.f32()))
				}
				ScalarOpcode::Fma => TestScalar::F32(
					operands[0]
						.f32()
						.mul_add(operands[1].f32(), operands[2].f32()),
				),
				ScalarOpcode::LessThan => {
					let (left, right) = binary();
					let result = match (left, right) {
						(TestScalar::F32(left), TestScalar::F32(right)) => left < right,
						(TestScalar::I32(left), TestScalar::I32(right)) => left < right,
						_ => panic!("mixed test comparison"),
					};
					TestScalar::I32(i32::from(result))
				}
				ScalarOpcode::LessThanOrEqual => {
					let (left, right) = binary();
					let result = match (left, right) {
						(TestScalar::F32(left), TestScalar::F32(right)) => left <= right,
						(TestScalar::I32(left), TestScalar::I32(right)) => left <= right,
						_ => panic!("mixed test comparison"),
					};
					TestScalar::I32(i32::from(result))
				}
				ScalarOpcode::GreaterThanOrEqual => {
					let (left, right) = binary();
					TestScalar::I32(i32::from(left.f32() >= right.f32()))
				}
				ScalarOpcode::Select => match operands[0].i32() != 0 {
					true => operands[1],
					false => operands[2],
				},
				ScalarOpcode::ShiftLeft => {
					let (left, right) = binary();
					TestScalar::I32(left.i32().wrapping_shl(right.i32() as u32))
				}
				ScalarOpcode::BitcastI32ToF32 => TestScalar::F32(f32::from_bits(unary().i32() as u32)),
				ScalarOpcode::Require => {
					faulted |= unary().i32() == 0;
					unary()
				}
				ScalarOpcode::IsFinite => TestScalar::I32(i32::from(unary().f32().is_finite())),
				ScalarOpcode::RoundNearestEven => TestScalar::F32(unary().f32().round_ties_even()),
				ScalarOpcode::ConvertF32ToI32 => TestScalar::I32(unary().f32() as i32),
				opcode => panic!("unsupported inference test opcode {opcode:?}"),
			};
			values.insert(instruction.result, value);
		}

		assert_eq!(program.outputs.len(), 1);
		TestEvaluation {
			output: values[&program.outputs[0]].f32(),
			faulted,
		}
	}

	#[test]
	fn saved_spans_align_raw_numeric_and_reserved_categorical_routes_without_host_calculation() {
		let schema = [
			InferenceFeatureSchema::new(3, b"amount", InferenceFeatureEncoding::NumericI32),
			InferenceFeatureSchema::new(
				8,
				b"color",
				InferenceFeatureEncoding::CategoricalDictionary {
					dictionary: vec![b"blue".to_vec(), b"red".to_vec()],
				},
			),
		];
		let prepared =
			recipe_ingest::prepare_inference_table(&table(b"color,amount\nred,10\npurple,20\n"), &schema)
				.unwrap();
		let spans = [
			CompiledFeatureSpan::new(3, 0, 1, DenseFeatureLowering::NumericScalar),
			CompiledFeatureSpan::new(
				8,
				1,
				3,
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width: 2,
					reserved_index: 2,
				},
			),
		];
		validate_prepared_feature_spans(&prepared, &spans).unwrap();
		assert_eq!(
			prepared.features()[0].values(),
			&PreparedInferenceValues::I32(vec![10, 20])
		);
		assert_eq!(
			prepared.features()[1].values(),
			&PreparedInferenceValues::I32(vec![1, 2])
		);
	}

	#[test]
	fn checkpoint_file_loader_enforces_regular_file_and_source_bound() {
		let directory = TestPath::new("directory");
		fs::create_dir(&directory.0).unwrap();
		let error = load_checkpoint_file(&directory.0, CheckpointDecodeLimits::default()).unwrap_err();
		assert!(matches!(
			error,
			InferencePreparationError::CheckpointSource(_)
		));

		let file = TestPath::new("large.ogdl");
		fs::write(&file.0, b"12345").unwrap();
		let mut limits = CheckpointDecodeLimits::default();
		limits.source_bytes = 4;
		let error = load_checkpoint_file(&file.0, limits).unwrap_err();
		assert!(matches!(
			error,
			InferencePreparationError::CheckpointSource(_)
		));
	}

	#[test]
	fn loaded_v5_artifact_prepares_reordered_target_free_rows_and_retains_normalization() {
		let file = TestPath::new("valid.ogdl");
		fs::write(
			&file.0,
			crate::checkpoint::encoded_test_checkpoint_fixture(),
		)
		.unwrap();
		let checkpoint = load_checkpoint_file(&file.0, CheckpointDecodeLimits::default()).unwrap();
		let data_file = TestPath::new("inference.csv");
		fs::write(&data_file.0, b"extra,\"feature\nbytes\"\nignored,1.5\n").unwrap();
		let dataset = distill_dataset(
			&data_file.0,
			IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
		)
		.unwrap();
		let prepared = prepare_checkpoint_inference(checkpoint, &dataset).unwrap();
		assert_eq!(
			prepared.features()[0].values(),
			&PreparedInferenceValues::F32Bits(vec![1.5f32.to_bits()])
		);
		assert_eq!(prepared.feature_spans().len(), 1);
		assert_eq!(prepared.normalization(), DenseDataNormalization::ZScore);
		assert_eq!(prepared.normalization_tensors().len(), 2);
		assert_eq!(prepared.feature_normalization_mask(), &[1.0f32.to_bits()]);
		assert_eq!(prepared.normalization_epsilon().to_bits(), 0x3586_37bd);
	}

	#[test]
	fn loaded_v5_dictionary_routes_unseen_and_missing_values_without_refitting() {
		let file = TestPath::new("categorical.ogdl");
		fs::write(
			&file.0,
			crate::checkpoint::encoded_categorical_feature_test_checkpoint_fixture(),
		)
		.unwrap();
		let checkpoint = load_checkpoint_file(&file.0, CheckpointDecodeLimits::default()).unwrap();
		let data_file = TestPath::new("categorical-inference.csv");
		fs::write(
			&data_file.0,
			b"extra,\"feature\nbytes\"\nignored,red\nignored,purple\nignored,\"\"\n",
		)
		.unwrap();
		let dataset = distill_dataset(
			&data_file.0,
			IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
		)
		.unwrap();
		let prepared = prepare_checkpoint_inference(checkpoint, &dataset).unwrap();
		assert_eq!(
			prepared.features()[0].values(),
			&PreparedInferenceValues::I32(vec![1, 2, 2])
		);
		assert_eq!(prepared.feature_spans()[0].start(), 0);
		assert_eq!(prepared.feature_spans()[0].width(), 3);
		assert_eq!(
			prepared.feature_normalization_mask(),
			&[0.0f32.to_bits(), 0.0f32.to_bits(), 0.0f32.to_bits()]
		);
	}

	#[test]
	fn loaded_v5_schema_rejects_a_missing_required_feature_with_its_typed_path() {
		let file = TestPath::new("missing-feature.ogdl");
		fs::write(
			&file.0,
			crate::checkpoint::encoded_test_checkpoint_fixture(),
		)
		.unwrap();
		let checkpoint = load_checkpoint_file(&file.0, CheckpointDecodeLimits::default()).unwrap();
		let error = prepare_checkpoint_inference_table(checkpoint, &table(b"extra\n1\n")).unwrap_err();
		let InferencePreparationError::Data(error) = error else {
			panic!("expected typed inference data error");
		};
		assert_eq!(
			error.kind(),
			InferencePrepareErrorKind::MissingRequiredFeature
		);
		let path = error.path().unwrap();
		assert_eq!(path.feature(), 0);
		assert_eq!(path.source_vector(), 0);
		assert_eq!(path.column(), b"feature\nbytes");
		assert_eq!(path.source_row(), None);
	}

	#[test]
	fn checkpoint_schema_errors_remain_typed_through_training_boundary() {
		let schema = [InferenceFeatureSchema::new(
			4,
			b"required",
			InferenceFeatureEncoding::NumericF32,
		)];
		let error = recipe_ingest::prepare_inference_table(&table(b"other\n1\n"), &schema).unwrap_err();
		let error = InferencePreparationError::from(error);
		let InferencePreparationError::Data(error) = error else {
			panic!("expected typed inference data error");
		};
		assert_eq!(
			error.kind(),
			InferencePrepareErrorKind::MissingRequiredFeature
		);
		assert_eq!(error.path().unwrap().source_vector(), 4);
	}

	fn prepared_fixture(checkpoint: &[u8], source: &[u8]) -> PreparedInference {
		let checkpoint = decode_checkpoint(checkpoint, CheckpointDecodeLimits::default()).unwrap();
		prepare_checkpoint_inference_table(checkpoint, &table(source)).unwrap()
	}

	fn external_output_count(compiled: &CompiledInference) -> usize {
		compiled
			.graph()
			.tensors
			.iter()
			.filter(|tensor| tensor.external_output)
			.count()
	}

	#[test]
	fn binary_inference_compiles_one_target_free_probability_output_deterministically() {
		let encoded = crate::checkpoint::encoded_test_checkpoint_fixture();
		let prepared = prepared_fixture(
			&encoded,
			b"extra,\"feature\nbytes\"\nignored,1.5\nignored,2.5\n",
		);
		let first = compile_prepared_inference(&prepared).unwrap();
		let second = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(first.rows(), 2);
		assert_eq!(
			first.output().kind(),
			InferencePredictionKind::BinaryProbability
		);
		assert_eq!(first.output().dtype(), DType::F32);
		assert_eq!(first.output().shape().extents(), [2, 1]);
		assert_eq!(external_output_count(&first), 1);
		assert_eq!(first.program().iterations().get(), 1);
		assert!(
			first.program()
				.domains()
				.iter()
				.all(|domain| domain.domain == IterationDomain::first())
		);
		assert_eq!(
			first.program().to_ogdl().unwrap(),
			second.program().to_ogdl().unwrap()
		);
		assert!(
			first.graph()
				.nodes
				.iter()
				.all(|node| !matches!(node.kernel.kind, PrimitiveKind::Random(_)))
		);
		let gradual_underflow_exp = recipe_math::exp_with_gradual_underflow_program().unwrap();
		assert!(first.graph().nodes.iter().any(|node| {
			matches!(
				&node.kernel.kind,
				PrimitiveKind::Elementwise(map) if map.program == gradual_underflow_exp
			)
		}));
		assert_eq!(
			first.external_inputs()
				.iter()
				.map(InferenceExternalInput::role)
				.collect::<Vec<_>>(),
			[
				InferenceInputRole::Feature {
					feature: 0,
					source_vector: 0,
				},
				InferenceInputRole::FeatureNormalizationMask,
				InferenceInputRole::DataNormalizationMean,
				InferenceInputRole::DataNormalizationVariance,
				InferenceInputRole::LayerWeight { layer: 0 },
				InferenceInputRole::LayerBias { layer: 0 },
			]
		);
		let feature = &first.external_inputs()[0];
		assert_eq!(feature.dtype(), DType::F32);
		assert_eq!(feature.shape().extents(), [2]);
		assert_eq!(
			feature.bytes(),
			[
				1.5f32.to_le_bytes().as_slice(),
				2.5f32.to_le_bytes().as_slice(),
			]
			.concat()
		);
	}

	#[test]
	fn categorical_inference_one_hot_and_dense_assembly_are_device_calculations() {
		let encoded = crate::checkpoint::encoded_categorical_feature_test_checkpoint_fixture();
		let prepared = prepared_fixture(
			&encoded,
			b"extra,\"feature\nbytes\"\nignored,red\nignored,purple\nignored,\"\"\n",
		);
		let compiled = compile_prepared_inference(&prepared).unwrap();
		let feature = compiled
			.external_inputs()
			.iter()
			.find(|input| matches!(input.role(), InferenceInputRole::Feature { .. }))
			.unwrap();
		assert_eq!(feature.dtype(), DType::I32);
		assert_eq!(feature.shape().extents(), [3]);
		assert_eq!(
			feature.bytes(),
			[1_i32, 2, 2]
				.into_iter()
				.flat_map(i32::to_le_bytes)
				.collect::<Vec<_>>()
		);
		assert!(
			compiled
				.external_inputs()
				.iter()
				.all(|input| { !(input.dtype() == DType::F32 && input.shape().extents() == [3, 3]) })
		);
		assert!(
			compiled
				.graph()
				.nodes
				.iter()
				.filter(|node| matches!(node.kernel.kind, PrimitiveKind::Scatter(_)))
				.count() >= 2
		);
		assert_eq!(compiled.output().shape().extents(), [3, 1]);
		assert_eq!(external_output_count(&compiled), 1);
	}

	#[test]
	fn multiclass_inference_runs_each_effective_adapter_layer_once_then_stable_softmax() {
		let encoded = crate::checkpoint::encoded_multiclass_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n2.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(
			compiled.output().kind(),
			InferencePredictionKind::MulticlassProbabilities
		);
		assert_eq!(compiled.output().shape().extents(), [2, 4]);
		assert!(compiled.output_adapter().is_some());
		assert_eq!(
			compiled
				.graph()
				.nodes
				.iter()
				.filter(|node| matches!(node.kernel.kind, PrimitiveKind::Contraction(_)))
				.count(),
			2
		);
		let reductions = compiled
			.graph()
			.nodes
			.iter()
			.filter_map(|node| match &node.kernel.kind {
				PrimitiveKind::Reduce(reduce) => Some(reduce.operator),
				_ => None,
			})
			.collect::<Vec<_>>();
		assert!(reductions.contains(&ReduceOperator::Maximum));
		assert!(reductions.contains(&ReduceOperator::Sum));
		let gradual_underflow_exp = recipe_math::exp_with_gradual_underflow_program().unwrap();
		assert!(compiled.graph().nodes.iter().any(|node| {
			matches!(
				&node.kernel.kind,
				PrimitiveKind::Elementwise(map) if map.program == gradual_underflow_exp
			)
		}));
		assert_eq!(
			compiled
				.external_inputs()
				.iter()
				.filter(|input| matches!(input.role(), InferenceInputRole::LayerWeight { .. }))
				.count(),
			2
		);
		assert_eq!(
			compiled
				.external_inputs()
				.iter()
				.filter(|input| matches!(input.role(), InferenceInputRole::LayerBias { .. }))
				.count(),
			2
		);
		assert_eq!(external_output_count(&compiled), 1);
	}

	#[test]
	fn regression_inference_exposes_the_raw_effective_model_output() {
		let encoded = crate::checkpoint::encoded_regression_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n2.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(
			compiled.output().kind(),
			InferencePredictionKind::Regression
		);
		assert_eq!(compiled.output().shape().extents(), [2, 1]);
		assert!(
			compiled
				.graph()
				.nodes
				.last()
				.unwrap()
				.kernel
				.outputs
				.contains(&compiled.output().value())
		);
		assert_eq!(external_output_count(&compiled), 1);
	}

	#[test]
	fn saved_binary_temperature_is_a_parameter_input_to_device_scaling() {
		let encoded = crate::checkpoint::encoded_calibrated_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		let temperature = compiled
			.external_inputs()
			.iter()
			.find(|input| input.role() == InferenceInputRole::Temperature)
			.unwrap();
		assert_eq!(temperature.dtype(), DType::F32);
		assert_eq!(temperature.shape().extents(), [1]);
		assert_eq!(temperature.bytes(), 2.0f32.to_le_bytes());
		assert!(compiled.graph().nodes.iter().any(|node| {
			node.kernel.inputs.contains(&temperature.value())
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Elementwise(map)
						if map
							.program
							.instructions
							.iter()
							.any(|instruction| instruction.opcode == ScalarOpcode::Divide)
				)
		}));
	}

	#[test]
	fn prediction_tail_programs_preserve_subnormal_probabilities_without_legacy_range_faults() {
		let sigmoid_exponent = stable_sigmoid_exponent_program().unwrap();
		assert_eq!(
			sigmoid_exponent
				.instructions
				.iter()
				.map(|instruction| instruction.opcode)
				.collect::<Vec<_>>(),
			[ScalarOpcode::Absolute, ScalarOpcode::Negate]
		);
		let sigmoid_result = stable_sigmoid_result_program().unwrap();
		assert!(
			sigmoid_result
				.instructions
				.iter()
				.any(|instruction| instruction.opcode == ScalarOpcode::Select)
		);
		assert!(
			sigmoid_result
				.instructions
				.iter()
				.all(|instruction| instruction.opcode != ScalarOpcode::Require)
		);
		let gradual_underflow_exp = recipe_math::exp_with_gradual_underflow_program().unwrap();
		let exponent_argument = evaluate_test_program(&sigmoid_exponent, &[-100.0]);
		assert!(!exponent_argument.faulted);
		assert_eq!(exponent_argument.output, -100.0);
		let exponent = evaluate_test_program(&gradual_underflow_exp, &[exponent_argument.output]);
		assert!(!exponent.faulted);
		assert!(exponent.output.is_subnormal());
		assert!(exponent.output > 0.0);
		let sigmoid = evaluate_test_program(&sigmoid_result, &[-100.0, exponent.output]);
		assert!(!sigmoid.faulted);
		assert_eq!(sigmoid.output.to_bits(), exponent.output.to_bits());

		let softmax_exponent_input = softmax_exponent_input_program().unwrap();
		let exp_zero = evaluate_test_program(&gradual_underflow_exp, &[0.0]);
		let gap_ninety = evaluate_test_program(&softmax_exponent_input, &[-90.0]);
		let gap_one_hundred = evaluate_test_program(&softmax_exponent_input, &[-100.0]);
		let exp_gap_ninety = evaluate_test_program(&gradual_underflow_exp, &[gap_ninety.output]);
		let exp_gap_one_hundred = evaluate_test_program(&gradual_underflow_exp, &[gap_one_hundred.output]);
		for evaluation in [exp_zero, exp_gap_ninety, exp_gap_one_hundred] {
			assert!(!evaluation.faulted);
		}
		let sum = exp_zero.output + exp_gap_ninety.output + exp_gap_one_hundred.output;
		let probability_ninety = exp_gap_ninety.output / sum;
		let probability_one_hundred = exp_gap_one_hundred.output / sum;
		assert!(probability_ninety.is_subnormal());
		assert!(probability_one_hundred.is_subnormal());
		assert!(probability_ninety > probability_one_hundred);
		assert!(probability_one_hundred > 0.0);

		let finite_logit_shift = -f32::MAX - f32::MAX;
		assert_eq!(finite_logit_shift, f32::NEG_INFINITY);
		let overflowed_shift = evaluate_test_program(&softmax_exponent_input, &[finite_logit_shift]);
		assert!(!overflowed_shift.faulted);
		assert_eq!(overflowed_shift.output, -104.0);
		let overflowed_tail = evaluate_test_program(&gradual_underflow_exp, &[overflowed_shift.output]);
		assert!(!overflowed_tail.faulted);
		assert_eq!(overflowed_tail.output.to_bits(), 0.0_f32.to_bits());
		assert!(evaluate_test_program(&softmax_exponent_input, &[f32::INFINITY]).faulted);
		assert!(evaluate_test_program(&softmax_exponent_input, &[f32::NAN]).faulted);

		let true_underflow = evaluate_test_program(&gradual_underflow_exp, &[-104.0]);
		assert!(!true_underflow.faulted);
		assert_eq!(true_underflow.output.to_bits(), 0.0_f32.to_bits());
		let v5_i32_conversion = checked_i32_to_f32_program().unwrap();
		assert_eq!(
			v5_i32_conversion
				.instructions
				.iter()
				.map(|instruction| instruction.opcode)
				.collect::<Vec<_>>(),
			[
				ScalarOpcode::ConvertI32ToF32,
				ScalarOpcode::ConvertF32ToI32,
				ScalarOpcode::Equal,
				ScalarOpcode::NotEqual,
				ScalarOpcode::BitAnd,
				ScalarOpcode::Require,
			]
		);
	}

	#[test]
	fn structured_residual_checkpoint_fails_with_typed_unsupported_topology() {
		let encoded = crate::checkpoint::encoded_residual_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n");
		let error = compile_prepared_inference(&prepared).unwrap_err();
		assert_eq!(error.kind(), InferenceCompileErrorKind::UnsupportedTopology);
		assert!(error.detail().contains("structured blocks"));
		assert!(error.detail().contains("v6"));
	}
}
