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
	AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather, IndexBounds, IndexMap,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarProgramBuilder,
	Scatter, ScatterConflict, Shape, Tensor,
};
use recipe_ops::{
	CategoricalBayesInferenceRequest, IdentityNamespace, KnnAllOutputRequest, KnnOutputRequest,
	MaterializationRequest, NamedTensor, PreparedParameter, PreparedParameters, TreeEnsembleInferenceRequest,
	append_categorical_bayes_inference, append_knn_all_outputs, canonical_elu_program, canonical_leaky_relu_program,
	canonical_prelu_program, canonical_selu_program, categorical_bayes_inference_requirements,
	knn_all_output_requirements, lower_scalar, materialize_composition, materialize_tree_ensemble_inference,
	operation_registry, prepare_channelwise_convolution_1d, prepare_channelwise_max_pool_1d,
	tree_ensemble_inference_requirements,
};
use recipe_program::{IterationDomain, KernelIterationDomain, StaticCalculationProgram};

use crate::checkpoint::decode_error;
use crate::{
	BayesModelArtifact, BayesModelDecodeLimits, CheckpointArtifact, CheckpointArtifactMetadata,
	CheckpointArtifactVector, CheckpointDecodeErrorKind, CheckpointDecodeLimits, CheckpointError, CheckpointPath,
	CheckpointTensorImage, CompiledFeatureSpan, DenseActivation, DenseDataNormalization, DenseFeatureLowering,
	DenseNormalization, DenseOperation, DenseOutputAdapter, DenseTask, KnnModelArtifact, KnnModelDecodeLimits,
	KnnReferenceValues, MAXIMUM_REDUCTION_TREE_LANES, decode_bayes_model, decode_checkpoint, decode_knn_model,
};

#[path = "gguf_llama.rs"]
mod gguf_llama;

pub use gguf_llama::{
	GgufLlamaArtifact, GgufLlamaError, GgufLlamaErrorKind, GgufLlamaResult, PreparedGgufLlamaInference,
	compile_prepared_gguf_llama_inference, decode_gguf_llama, load_gguf_llama_model_file,
	prepare_gguf_llama_inference_table,
};

const MATERIALIZATION_RESERVATION: u64 = 64;
const WORKSPACE_LIMIT: ByteCount = ByteCount::new(u64::MAX);

/// Failure to load a checkpoint or prepare schema-bound inference features.
#[derive(Debug)]
#[non_exhaustive]
pub enum InferencePreparationError {
	CheckpointSource(SourceError),
	Checkpoint(CheckpointError),
	GgufLlama(GgufLlamaError),
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
			Self::GgufLlama(error) => write!(formatter, "load GGUF llama model: {error}"),
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
			Self::GgufLlama(error) => Some(error),
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

impl From<GgufLlamaError> for InferencePreparationError {
	fn from(error: GgufLlamaError) -> Self {
		Self::GgufLlama(error)
	}
}

pub type InferencePreparationResult<T> = Result<T, InferencePreparationError>;

/// One decoded Recipe-owned semantic model selected by its strict document
/// root. Model-family dispatch never falls back from one decoder to another.
#[derive(Clone, Debug)]
pub enum SemanticModelArtifact {
	Dense(CheckpointArtifact),
	Knn(KnnModelArtifact),
	Bayes(BayesModelArtifact),
}

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
	LayerPRelu {
		layer: usize,
		occurrence: usize,
	},
	EmbeddingTable {
		block: usize,
	},
	AttentionQuery {
		block: usize,
	},
	AttentionKey {
		block: usize,
	},
	AttentionValue {
		block: usize,
	},
	AttentionOutput {
		block: usize,
	},
	RnnInputWeight {
		block: usize,
	},
	RnnRecurrentWeight {
		block: usize,
	},
	RnnBias {
		block: usize,
	},
	GruResetInputWeight {
		block: usize,
	},
	GruResetRecurrentWeight {
		block: usize,
	},
	GruResetBias {
		block: usize,
	},
	GruUpdateInputWeight {
		block: usize,
	},
	GruUpdateRecurrentWeight {
		block: usize,
	},
	GruUpdateBias {
		block: usize,
	},
	GruCandidateInputWeight {
		block: usize,
	},
	GruCandidateRecurrentWeight {
		block: usize,
	},
	GruCandidateBias {
		block: usize,
	},
	LstmInputGateInputWeight {
		block: usize,
	},
	LstmInputGateRecurrentWeight {
		block: usize,
	},
	LstmInputGateBias {
		block: usize,
	},
	LstmForgetGateInputWeight {
		block: usize,
	},
	LstmForgetGateRecurrentWeight {
		block: usize,
	},
	LstmForgetGateBias {
		block: usize,
	},
	LstmOutputGateInputWeight {
		block: usize,
	},
	LstmOutputGateRecurrentWeight {
		block: usize,
	},
	LstmOutputGateBias {
		block: usize,
	},
	LstmCandidateInputWeight {
		block: usize,
	},
	LstmCandidateRecurrentWeight {
		block: usize,
	},
	LstmCandidateBias {
		block: usize,
	},
	ConvolutionWindowIndices {
		block: usize,
	},
	ConvolutionWeight {
		block: usize,
	},
	ConvolutionBias {
		block: usize,
	},
	ConvolutionPRelu {
		block: usize,
		occurrence: usize,
	},
	PoolWindowIndices {
		block: usize,
	},
	PoolWinnerBases {
		block: usize,
	},
	KMeansCentroids {
		block: usize,
	},
	TreeSplitFeatures {
		block: usize,
	},
	TreeSplitThresholds {
		block: usize,
	},
	TreeLeafValues {
		block: usize,
	},
	ResidualProjectionWeight {
		block: usize,
	},
	ResidualBranchPRelu {
		block: usize,
		occurrence: usize,
	},
	ResidualOutputPRelu {
		block: usize,
		occurrence: usize,
	},
	Temperature,
	KnnReferenceFeatures,
	KnnReferenceValues {
		output: usize,
	},
	KnnReferenceKnown {
		output: usize,
	},
	BayesQueryParents {
		conditional: usize,
	},
	BayesReferenceParents {
		conditional: usize,
	},
	BayesReferenceChild {
		conditional: usize,
	},
	BayesParentMultipliers {
		conditional: usize,
	},
	BayesParentCardinalities {
		conditional: usize,
	},
	BayesConcatenationLeftIndices {
		join: usize,
	},
	BayesConcatenationRightIndices {
		join: usize,
	},
	BayesConcatenationSelectLeft {
		join: usize,
	},
	GgufTokenIds,
	GgufTensor {
		tensor: usize,
	},
	GgufRopePartnerIndices,
	GgufRopeCosines,
	GgufRopeSignedSines,
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
	MultiTargetBinaryProbabilities,
	JointTargetProbabilities,
	MultiTargetRegression,
	/// Row-major probability blocks for repeated observed categorical Bayesian
	/// target declarations.
	BayesProbabilities,
	/// Raw vocabulary logits for every position in one exact token stream.
	TokenLogits,
}

/// Semantic calculation represented by one compiled inference program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceTask {
	Dense(DenseTask),
	BayesProbabilities { width: u64 },
	TokenLogits { vocabulary: u64 },
}

/// Exact tensor contract for the one public inference prediction output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceOutputContract {
	value: ValueId,
	dtype: DType,
	target_dtypes: Vec<DType>,
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

	/// First dtype selected by semantic distillation. This compatibility accessor
	/// is the complete target dtype only for singular-target tasks.
	#[must_use]
	pub fn target_dtype(&self) -> DType {
		self.target_dtypes[0]
	}

	/// Source target dtypes in the saved declaration order. Prediction tensors
	/// remain one f32 matrix regardless of these source representations.
	#[must_use]
	pub fn target_dtypes(&self) -> &[DType] {
		&self.target_dtypes
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
	task: InferenceTask,
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
	pub const fn task(&self) -> InferenceTask {
		self.task
	}

	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> {
		self.output_adapter
	}
}

/// Aggregation represented by one independently typed KNN prediction tensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnnInferencePredictionKind {
	NumericMean,
	DiscreteMode,
}

/// Exact tensor and source-target identity for one KNN prediction output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnInferenceOutputContract {
	value: ValueId,
	dtype: DType,
	shape: Shape,
	source_vector: usize,
	kind: KnnInferencePredictionKind,
}

impl KnnInferenceOutputContract {
	pub(crate) fn new(
		value: ValueId,
		dtype: DType,
		shape: Shape,
		source_vector: usize,
		kind: KnnInferencePredictionKind,
	) -> Self {
		Self {
			value,
			dtype,
			shape,
			source_vector,
			kind,
		}
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
	pub const fn source_vector(&self) -> usize {
		self.source_vector
	}

	#[must_use]
	pub const fn kind(&self) -> KnnInferencePredictionKind {
		self.kind
	}
}

/// One deterministic, target-free KNN program with one egress tensor per
/// declared target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledKnnInference {
	program: StaticCalculationProgram,
	external_inputs: Vec<InferenceExternalInput>,
	outputs: Vec<KnnInferenceOutputContract>,
	rows: u64,
}

impl CompiledKnnInference {
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
	pub fn outputs(&self) -> &[KnnInferenceOutputContract] {
		&self.outputs
	}

	#[must_use]
	pub const fn rows(&self) -> u64 {
		self.rows
	}
}

/// A decoded KNN model and target-free rows bound to its saved feature schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedKnnInference {
	artifact: KnnModelArtifact,
	data: PreparedInferenceDataset,
}

/// One observed categorical Bayesian model bound to target-free query rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedBayesInference {
	artifact: BayesModelArtifact,
	data: PreparedInferenceDataset,
}

impl PreparedBayesInference {
	#[must_use]
	pub const fn artifact(&self) -> &BayesModelArtifact {
		&self.artifact
	}

	#[must_use]
	pub const fn data(&self) -> &PreparedInferenceDataset {
		&self.data
	}

	#[must_use]
	pub fn into_artifact(self) -> BayesModelArtifact {
		self.artifact
	}
}

impl PreparedKnnInference {
	#[must_use]
	pub const fn artifact(&self) -> &KnnModelArtifact {
		&self.artifact
	}

	#[must_use]
	pub const fn data(&self) -> &PreparedInferenceDataset {
		&self.data
	}

	#[must_use]
	pub fn into_artifact(self) -> KnnModelArtifact {
		self.artifact
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

/// Read and decode one bounded KNN semantic model from a regular-file
/// snapshot.
pub fn load_knn_model_file(
	path: impl AsRef<Path>,
	limits: KnnModelDecodeLimits,
) -> InferencePreparationResult<KnnModelArtifact> {
	let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("KNN model source-byte bound cannot be represented as u64: {error}"),
		})?;
	let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_knn_model(source.bytes(), limits).map_err(Into::into)
}

/// Read and decode one bounded observed categorical Bayesian semantic model.
pub fn load_bayes_model_file(
	path: impl AsRef<Path>,
	limits: BayesModelDecodeLimits,
) -> InferencePreparationResult<BayesModelArtifact> {
	let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("Bayesian model source-byte bound cannot be represented as u64: {error}"),
		})?;
	let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_bayes_model(source.bytes(), limits).map_err(Into::into)
}

/// Read one bounded semantic `.ogdl` snapshot and dispatch its strict decoder
/// from the document root.
///
/// The root probe only examines the first line. The selected decoder remains
/// solely responsible for complete syntax, version, canonicality, and
/// model-family validation.
pub fn load_semantic_model_file(
	path: impl AsRef<Path>,
	checkpoint_limits: CheckpointDecodeLimits,
	knn_limits: KnnModelDecodeLimits,
) -> InferencePreparationResult<SemanticModelArtifact> {
	let source_bytes = checkpoint_limits.source_bytes.max(knn_limits.source_bytes);
	let source_bytes = source_bytes.max(BayesModelDecodeLimits::default().source_bytes);
	let source_bytes =
		u64::try_from(source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("semantic-model source-byte bound cannot be represented as u64: {error}"),
		})?;
	let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	let root = source
		.bytes()
		.split(|byte| matches!(*byte, b'\n' | b'\t'))
		.next()
		.expect("a byte slice always has a first split segment");
	let root = core::str::from_utf8(root).map_err(|error| {
		decode_error(
			CheckpointDecodeErrorKind::InvalidUtf8,
			CheckpointPath::root(),
			format!("semantic-model root is not UTF-8: {error}"),
		)
	})?;
	match root {
		"recipe" => decode_checkpoint(source.bytes(), checkpoint_limits)
			.map(SemanticModelArtifact::Dense)
			.map_err(Into::into),
		"recipe-knn-model" => decode_knn_model(source.bytes(), knn_limits)
			.map(SemanticModelArtifact::Knn)
			.map_err(Into::into),
		"recipe-bayes-model" => decode_bayes_model(source.bytes(), BayesModelDecodeLimits::default())
			.map(SemanticModelArtifact::Bayes)
			.map_err(Into::into),
		other => Err(decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			CheckpointPath::root(),
			format!("unknown semantic-model root {other:?}"),
		)
		.into()),
	}
}

/// Prepare a target-free table under one saved KNN feature schema.
pub fn prepare_knn_inference_table(
	artifact: KnnModelArtifact,
	table: &RawTable,
) -> InferencePreparationResult<PreparedKnnInference> {
	let schema = saved_feature_schema_from_parts(
		artifact.references().vectors(),
		artifact.references().feature_spans(),
	)?;
	let data = recipe_ingest::prepare_inference_table(table, &schema)?;
	validate_prepared_feature_spans(&data, artifact.references().feature_spans())?;
	Ok(PreparedKnnInference { artifact, data })
}

/// Prepare target-free query rows under the union of all saved categorical
/// Bayesian parent schemas. First occurrence across conditional/parent order
/// defines the physical feature order; shared parents are read once.
pub fn prepare_bayes_inference_table(
	artifact: BayesModelArtifact,
	table: &RawTable,
) -> InferencePreparationResult<PreparedBayesInference> {
	let mut schema = Vec::new();
	let mut seen = BTreeSet::new();
	for conditional in artifact.conditionals() {
		for parent in conditional.parents() {
			if seen.insert(parent.source_index()) {
				schema.push(InferenceFeatureSchema::new(
					parent.source_index(),
					parent.name(),
					InferenceFeatureEncoding::CategoricalDictionary {
						dictionary: parent.dictionary().to_vec(),
					},
				));
			}
		}
	}
	let data = recipe_ingest::prepare_inference_table(table, &schema)?;
	Ok(PreparedBayesInference { artifact, data })
}

/// Load a categorical Bayesian model and prepare a newly distilled dataset in
/// one bounded call.
pub fn load_and_prepare_bayes_inference(
	path: impl AsRef<Path>,
	limits: BayesModelDecodeLimits,
	dataset: &DistilledDataset,
) -> InferencePreparationResult<PreparedBayesInference> {
	let artifact = load_bayes_model_file(path, limits)?;
	prepare_bayes_inference_table(artifact, dataset.table())
}

/// Load a KNN model and prepare a newly distilled dataset in one bounded call.
pub fn load_and_prepare_knn_inference(
	path: impl AsRef<Path>,
	limits: KnnModelDecodeLimits,
	dataset: &DistilledDataset,
) -> InferencePreparationResult<PreparedKnnInference> {
	let artifact = load_knn_model_file(path, limits)?;
	prepare_knn_inference_table(artifact, dataset.table())
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
/// Every checkpoint version retains its effective topology in `blocks`,
/// including any synthetic output adapter. That canonical list is traversed
/// exactly once without flattening pool or residual structure.
///
/// # Errors
///
/// Returns a typed topology, extent, checkpoint-consistency, graph, operation,
/// or static-program failure.
pub fn compile_prepared_inference(prepared: &PreparedInference) -> InferenceCompileResult<CompiledInference> {
	let checkpoint = prepared.checkpoint();
	if checkpoint.blocks().is_empty() {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"checkpoint retains no effective model blocks",
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
	let leading_embedding = checkpoint.blocks().first().and_then(|block| match block {
		crate::CheckpointBlockImage::Embedding(embedding) => Some(embedding),
		_ => None,
	});
	let lowered = if let Some(embedding) = leading_embedding {
		compiler.compile_token_features(
			prepared.data(),
			checkpoint.feature_spans(),
			rows,
			feature_width,
			embedding.vocabulary().get(),
		)?
	} else {
		compiler.compile_features(
			prepared.data(),
			checkpoint.feature_spans(),
			rows,
			feature_width,
		)?
	};
	let mut current = compiler.apply_data_normalization(checkpoint, lowered, rows, feature_width)?;
	let mut current_width = feature_width;
	let mut logical_length = feature_width;
	let mut logical_channels = 1_u64;
	let mut layer_index = 0_usize;
	for (block_index, block) in checkpoint.blocks().iter().enumerate() {
		match block {
			crate::CheckpointBlockImage::Embedding(embedding) => {
				current = compiler.compile_embedding(block_index, embedding, current, rows, current_width)?;
				current_width = embedding
					.sequence_length()
					.get()
					.checked_mul(embedding.dimensions().get())
					.ok_or_else(|| {
						InferenceCompileError::new(
							InferenceCompileErrorKind::ArithmeticOverflow,
							"embedding inference output width overflowed u64",
						)
					})?;
				logical_length = embedding.sequence_length().get();
				logical_channels = embedding.dimensions().get();
			}
			crate::CheckpointBlockImage::Attention(attention) => {
				current = compiler.compile_attention(
					block_index,
					attention,
					current,
					rows,
					current_width,
					logical_length,
					logical_channels,
					checkpoint.config().reduction_tree_lanes,
				)?;
				current_width = attention
					.sequence_length()
					.get()
					.checked_mul(attention.dimensions().get())
					.ok_or_else(|| {
						InferenceCompileError::new(
							InferenceCompileErrorKind::ArithmeticOverflow,
							"attention inference output width overflowed u64",
						)
					})?;
				logical_length = attention.sequence_length().get();
				logical_channels = attention.dimensions().get();
			}
			crate::CheckpointBlockImage::Rnn(rnn) => {
				current = compiler.compile_rnn(block_index, rnn, current, rows, current_width)?;
				current_width = rnn.width().get();
				logical_length = current_width;
				logical_channels = 1;
			}
			crate::CheckpointBlockImage::Gru(gru) => {
				current = compiler.compile_gru(block_index, gru, current, rows, current_width)?;
				current_width = gru.width().get();
				logical_length = current_width;
				logical_channels = 1;
			}
			crate::CheckpointBlockImage::Lstm(lstm) => {
				current = compiler.compile_lstm(block_index, lstm, current, rows, current_width)?;
				current_width = lstm.width().get();
				logical_length = current_width;
				logical_channels = 1;
			}
			crate::CheckpointBlockImage::Layer(layer) => {
				current = compiler.compile_layer(
					layer_index,
					layer,
					current,
					rows,
					current_width,
					checkpoint.config().normalization_epsilon,
					checkpoint.config().reduction_tree_lanes,
				)?;
				layer_index = layer_index.checked_add(1).ok_or_else(identity_exhausted)?;
				current_width = layer.declaration().width().get();
				logical_length = current_width;
				logical_channels = 1;
			}
			crate::CheckpointBlockImage::Convolution(convolution) => {
				current = compiler.compile_convolution(
					block_index,
					convolution,
					current,
					rows,
					current_width,
					logical_length,
					logical_channels,
					checkpoint.config().normalization_epsilon,
					checkpoint.config().reduction_tree_lanes,
				)?;
				current_width = convolution
					.geometry()
					.output_width()
					.expect("validated convolution output width is nonzero")
					.get();
				logical_length = convolution.geometry().output_length().get();
				logical_channels = convolution.geometry().filters().get();
			}
			crate::CheckpointBlockImage::Pool(pool) => {
				current = compiler.compile_pool(
					block_index,
					pool,
					current,
					rows,
					current_width,
					logical_length,
					logical_channels,
					checkpoint.config().reduction_tree_lanes,
				)?;
				current_width = pool.output_width().get();
				logical_length = pool.output_length().get();
				logical_channels = pool.channels().get();
			}
			crate::CheckpointBlockImage::KMeans(kmeans) => {
				current = compiler.compile_kmeans(
					block_index,
					kmeans,
					current,
					rows,
					current_width,
					checkpoint.config().reduction_tree_lanes,
				)?;
				current_width = kmeans.clusters().get();
				logical_length = current_width;
				logical_channels = 1;
			}
			crate::CheckpointBlockImage::Tree(tree) => {
				current = compiler.compile_tree(
					block_index,
					tree,
					current,
					rows,
					current_width,
					checkpoint.config().reduction_tree_lanes,
				)?;
				current_width = tree.output_width().get();
				logical_length = current_width;
				logical_channels = 1;
			}
			crate::CheckpointBlockImage::Residual(residual) => {
				current = compiler.compile_residual(
					block_index,
					residual,
					current,
					rows,
					current_width,
					&mut layer_index,
					checkpoint.config().normalization_epsilon,
					checkpoint.config().reduction_tree_lanes,
				)?;
				current_width = residual.output_width().get();
				logical_length = current_width;
				logical_channels = 1;
			}
		}
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
			format!("effective model blocks produce width {current_width}, saved task requires {expected_width}"),
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
	let target_dtypes = checkpoint.target_dtypes().collect::<Vec<_>>();
	if target_dtypes.len() != checkpoint.task().target_count() {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"saved target vectors do not all have fixed calculation dtypes",
		));
	}
	compiler.finish(
		prediction,
		kind,
		rows,
		InferenceTask::Dense(checkpoint.task()),
		target_dtypes,
		checkpoint.output_adapter(),
	)
}

/// Compile every repeated observed categorical target conditional in
/// declaration order.
///
/// Each conditional retains its own saved observations, query-parent order,
/// mixed-radix packing, native histogram, and Laplace posterior. The resulting
/// f32 matrices are concatenated row-wise into one output whose adjacent class
/// ranges follow repeated `.bayes(...)` order.
pub fn compile_prepared_bayes_inference(
	prepared: &PreparedBayesInference,
) -> InferenceCompileResult<CompiledInference> {
	let rows = u64::try_from(prepared.data().rows()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian inference row count cannot be represented by u64: {error}"),
		)
	})?;
	if rows == 0 {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::EmptyDataset,
			"categorical Bayesian inference requires at least one query row",
		));
	}
	if prepared.artifact().conditionals().is_empty() {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"Bayesian semantic model retains no conditionals",
		));
	}

	let mut compiler = InferenceGraphCompiler::new();
	let mut probabilities = Vec::with_capacity(prepared.artifact().conditionals().len());
	let mut total_width = 0_u64;
	for (conditional, references) in prepared.artifact().conditionals().iter().enumerate() {
		let width = u64::try_from(references.child_classes()).map_err(|error| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::UnsupportedExtent,
				format!("Bayesian conditional {conditional} child class count does not fit u64: {error}"),
			)
		})?;
		let output = compile_bayes_conditional(
			&mut compiler,
			prepared.data(),
			references,
			prepared.artifact().smoothing(),
			conditional,
			rows,
		)?;
		total_width = total_width.checked_add(width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"Bayesian aggregate output width overflowed u64",
			)
		})?;
		probabilities.push((output, width));
	}
	let prediction = concatenate_bayes_probabilities(&mut compiler, probabilities, rows)?;
	compiler.finish(
		prediction,
		InferencePredictionKind::BayesProbabilities,
		rows,
		InferenceTask::BayesProbabilities { width: total_width },
		vec![DType::I32; prepared.artifact().conditionals().len()],
		None,
	)
}

fn compile_bayes_conditional(
	compiler: &mut InferenceGraphCompiler,
	data: &PreparedInferenceDataset,
	references: &crate::BayesianCategoricalReferenceSet,
	smoothing: f32,
	conditional: usize,
	rows: u64,
) -> InferenceCompileResult<ValueId> {
	let reference_rows = u64::try_from(references.reference_rows()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian conditional {conditional} reference row count does not fit u64: {error}"),
		)
	})?;
	let parent_count = u64::try_from(references.parents().len()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian parent count cannot be represented by u64: {error}"),
		)
	})?;
	let child_classes = u64::try_from(references.child_classes()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian child class count cannot be represented by u64: {error}"),
		)
	})?;
	let requirements = categorical_bayes_inference_requirements(
		reference_rows,
		rows,
		parent_count,
		references.parent_configurations(),
		child_classes,
	)?;
	let query_elements = data
		.rows()
		.checked_mul(references.parents().len())
		.ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"Bayesian query parent matrix size overflowed usize",
			)
		})?;
	let mut query_parent_codes = Vec::with_capacity(query_elements);
	for row in 0..data.rows() {
		for (parent_index, parent) in references.parents().iter().enumerate() {
			let feature = data
				.features()
				.iter()
				.find(|feature| {
					feature.schema().source_vector() == parent.source_index()
						&& feature.schema().name() == parent.name()
				})
				.ok_or_else(|| {
					InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						format!(
							"prepared Bayesian conditional {conditional} parent {parent_index} is absent"
						),
					)
				})?;
			let PreparedInferenceValues::I32(values) = feature.values() else {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!(
						"prepared Bayesian conditional {conditional} parent {parent_index} is not dictionary-coded I32"
					),
				));
			};
			let code = values.get(row).copied().ok_or_else(|| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!(
						"prepared Bayesian conditional {conditional} parent {parent_index} is missing query row {row}"
					),
				)
			})?;
			let cardinality = references.parent_cardinalities()[parent_index];
			if code < 0 || code >= cardinality {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!(
						"prepared Bayesian conditional {conditional} parent {parent_index} row {row} has out-of-range code {code}"
					),
				));
			}
			query_parent_codes.push(code);
		}
	}

	let reference_parent_codes = compiler.external(
		InferenceInputRole::BayesReferenceParents { conditional },
		DType::I32,
		shape(&[reference_rows, parent_count])?,
		references
			.parent_codes()
			.iter()
			.flat_map(|value| value.to_le_bytes())
			.collect(),
	)?;
	let reference_child_codes = compiler.external(
		InferenceInputRole::BayesReferenceChild { conditional },
		DType::I32,
		shape(&[reference_rows])?,
		references
			.child_codes()
			.iter()
			.flat_map(|value| value.to_le_bytes())
			.collect(),
	)?;
	let query_parent_codes = compiler.external(
		InferenceInputRole::BayesQueryParents { conditional },
		DType::I32,
		shape(&[rows, parent_count])?,
		query_parent_codes
			.iter()
			.flat_map(|value| value.to_le_bytes())
			.collect(),
	)?;
	let parent_multipliers = compiler.external(
		InferenceInputRole::BayesParentMultipliers { conditional },
		DType::I32,
		shape(&[parent_count])?,
		references
			.parent_multipliers()
			.iter()
			.flat_map(|value| value.to_le_bytes())
			.collect(),
	)?;
	let parent_cardinalities = compiler.external(
		InferenceInputRole::BayesParentCardinalities { conditional },
		DType::I32,
		shape(&[parent_count])?,
		references
			.parent_cardinalities()
			.iter()
			.flat_map(|value| value.to_le_bytes())
			.collect(),
	)?;
	let probabilities = compiler.tensor(DType::F32, shape(&[rows, child_classes])?)?;
	let first_value = compiler.next_value;
	let first_kernel = compiler.next_kernel;
	compiler.next_value = compiler
		.next_value
		.checked_add(requirements.intermediate_values)
		.ok_or_else(identity_exhausted)?;
	compiler.next_kernel = compiler
		.next_kernel
		.checked_add(requirements.kernels)
		.ok_or_else(identity_exhausted)?;
	let request = CategoricalBayesInferenceRequest {
		reference_parent_codes: bayes_request_input(&compiler, reference_parent_codes)?,
		reference_child_codes: bayes_request_input(&compiler, reference_child_codes)?,
		query_parent_codes: bayes_request_input(&compiler, query_parent_codes)?,
		parent_multipliers: bayes_request_input(&compiler, parent_multipliers)?,
		parent_cardinalities: bayes_request_input(&compiler, parent_cardinalities)?,
		probabilities: bayes_request_output(&compiler, probabilities)?,
		reference_rows,
		query_rows: rows,
		parent_count,
		parent_configurations: references.parent_configurations(),
		child_classes,
		smoothing,
		tree_lanes: MAXIMUM_REDUCTION_TREE_LANES,
		identity_namespace: IdentityNamespace::new(
			ValueId::new(first_value),
			requirements.intermediate_values,
			KernelTemplateId::new(first_kernel),
			requirements.kernels,
		),
		workspace_limit: requirements.workspace_bytes,
	};
	for tensor in compiler.tensors.values_mut() {
		tensor.external_input = compiler.external_input_ids.contains(&tensor.id);
		tensor.external_output = tensor.id == probabilities;
	}
	let mut graph = CalculationGraph {
		tensors: core::mem::take(&mut compiler.tensors)
			.into_values()
			.collect(),
		nodes: core::mem::take(&mut compiler.nodes),
	};
	let materialized = append_categorical_bayes_inference(&mut graph, &request)?;
	compiler.domains.extend(
		materialized
			.kernels
			.iter()
			.map(|kernel| KernelIterationDomain {
				kernel: *kernel,
				domain: IterationDomain::first(),
			}),
	);
	compiler.tensors = graph
		.tensors
		.into_iter()
		.map(|tensor| (tensor.id, tensor))
		.collect();
	compiler.nodes = graph.nodes;
	Ok(probabilities)
}

fn concatenate_bayes_probabilities(
	compiler: &mut InferenceGraphCompiler,
	probabilities: Vec<(ValueId, u64)>,
	rows: u64,
) -> InferenceCompileResult<ValueId> {
	let mut probabilities = probabilities.into_iter();
	let (mut combined, mut combined_width) = probabilities.next().ok_or_else(|| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"Bayesian inference has no probability outputs",
		)
	})?;
	for (join, (right, right_width)) in probabilities.enumerate() {
		let join = join + 1;
		let total_width = combined_width.checked_add(right_width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"Bayesian concatenated probability width overflowed u64",
			)
		})?;
		let left_elements = checked_product(
			&[rows, combined_width],
			"Bayesian left probability elements",
		)?;
		let right_elements = checked_product(&[rows, right_width], "Bayesian right probability elements")?;
		let total_elements = checked_product(
			&[rows, total_width],
			"Bayesian concatenated probability elements",
		)?;
		checked_i32(left_elements, "Bayesian left probability elements")?;
		checked_i32(right_elements, "Bayesian right probability elements")?;
		checked_i32(total_elements, "Bayesian concatenated probability elements")?;
		let capacity = usize::try_from(total_elements).map_err(|error| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::UnsupportedExtent,
				format!("Bayesian concatenation table length does not fit usize: {error}"),
			)
		})?;
		let mut left_indices = Vec::with_capacity(capacity);
		let mut right_indices = Vec::with_capacity(capacity);
		let mut select_left = Vec::<i32>::with_capacity(capacity);
		for row in 0..rows {
			for column in 0..total_width {
				if column < combined_width {
					left_indices.push(checked_i32(
						row * combined_width + column,
						"Bayesian left concatenation index",
					)?);
					right_indices.push(0);
					select_left.push(1);
				} else {
					left_indices.push(0);
					right_indices.push(checked_i32(
						row * right_width + column - combined_width,
						"Bayesian right concatenation index",
					)?);
					select_left.push(0);
				}
			}
		}
		let table_shape = shape(&[total_elements])?;
		let left_indices = compiler.external(
			InferenceInputRole::BayesConcatenationLeftIndices { join },
			DType::I32,
			table_shape.clone(),
			left_indices
				.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		)?;
		let right_indices = compiler.external(
			InferenceInputRole::BayesConcatenationRightIndices { join },
			DType::I32,
			table_shape.clone(),
			right_indices
				.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		)?;
		let select_left = compiler.external(
			InferenceInputRole::BayesConcatenationSelectLeft { join },
			DType::I32,
			table_shape.clone(),
			select_left
				.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		)?;
		let left = compiler.reinterpret_f32(combined, shape(&[left_elements])?)?;
		let right = compiler.reinterpret_f32(right, shape(&[right_elements])?)?;
		let concatenated = compiler.tensor(DType::F32, table_shape)?;
		let parameters = PreparedParameters::from([
			("rows".to_owned(), PreparedParameter::U64(rows)),
			(
				"left_columns".to_owned(),
				PreparedParameter::U64(combined_width),
			),
			(
				"right_columns".to_owned(),
				PreparedParameter::U64(right_width),
			),
			(
				"concatenation_tables_verified".to_owned(),
				PreparedParameter::Bool(true),
			),
		]);
		compiler.materialize(
			"gpu_concat_into",
			&[
				("left", left),
				("right", right),
				("left_indices", left_indices),
				("right_indices", right_indices),
				("select_left", select_left),
			],
			&[("concatenated", concatenated)],
			"left",
			&parameters,
		)?;
		combined = compiler.reinterpret_f32(concatenated, shape(&[rows, total_width])?)?;
		combined_width = total_width;
	}
	Ok(combined)
}

fn bayes_request_input(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone();
	tensor.external_input = true;
	tensor.external_output = false;
	Ok(tensor)
}

fn bayes_request_output(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone();
	tensor.external_input = false;
	tensor.external_output = true;
	Ok(tensor)
}

/// Compile one all-declared-output KNN inference program.
///
/// Distance calculation, optional feature normalization, neighbor selection,
/// numeric means, and discrete modes are all device calculations. Post-KNN
/// scalar or normalization operations remain rejected because applying one
/// numeric transform to heterogeneous semantic outputs is not defined by the
/// public declaration.
pub fn compile_prepared_knn_inference(prepared: &PreparedKnnInference) -> InferenceCompileResult<CompiledKnnInference> {
	let artifact = prepared.artifact();
	if !artifact.operations().is_empty() {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedTopology,
			"post-KNN operations are undefined for independently typed numeric and discrete outputs",
		));
	}
	let references = artifact.references();
	let rows = u64::try_from(prepared.data().rows()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("KNN inference row count cannot be represented by u64: {error}"),
		)
	})?;
	if rows == 0 {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::EmptyDataset,
			"KNN inference requires at least one query row",
		));
	}
	let reference_rows = u64::try_from(references.reference_rows()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("KNN reference row count cannot be represented by u64: {error}"),
		)
	})?;
	let feature_width = u64::try_from(references.feature_width()).map_err(|error| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("KNN feature width cannot be represented by u64: {error}"),
		)
	})?;
	if reference_rows == 0 || feature_width == 0 {
		return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"KNN model has an empty reference matrix",
		));
	}

	let mut compiler = InferenceGraphCompiler::new();
	let query_features = compiler.compile_features(
		prepared.data(),
		references.feature_spans(),
		rows,
		feature_width,
	)?;
	let reference_features = compiler.external(
		InferenceInputRole::KnnReferenceFeatures,
		DType::F32,
		shape(&[reference_rows, feature_width])?,
		references
			.reference_feature_bits()
			.iter()
			.flat_map(|bits| bits.to_le_bytes())
			.collect(),
	)?;
	let (query_features, reference_features) = compiler.normalize_knn_features(
		query_features,
		reference_features,
		rows,
		reference_rows,
		feature_width,
		artifact.data_normalization(),
		references.normalization_mask(),
		MAXIMUM_REDUCTION_TREE_LANES,
	)?;

	let mut requests = Vec::with_capacity(references.outputs().len());
	let mut outputs = Vec::with_capacity(references.outputs().len());
	for (output_index, output) in references.outputs().iter().enumerate() {
		let known = compiler.external(
			InferenceInputRole::KnnReferenceKnown {
				output: output_index,
			},
			DType::I32,
			shape(&[reference_rows])?,
			output.known()
				.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		)?;
		let prediction_shape = shape(&[rows, 1])?;
		let (request, contract) = match output.values() {
			KnnReferenceValues::NumericF32Bits(bits) => {
				let values = compiler.external(
					InferenceInputRole::KnnReferenceValues {
						output: output_index,
					},
					DType::F32,
					shape(&[reference_rows])?,
					bits.iter().flat_map(|bits| bits.to_le_bytes()).collect(),
				)?;
				let prediction = compiler.tensor(DType::F32, prediction_shape.clone())?;
				(
					KnnOutputRequest::Numeric {
						reference_values: knn_request_input(&compiler, values)?,
						known: knn_request_input(&compiler, known)?,
						predictions: knn_request_output(&compiler, prediction)?,
						known_references: output.known_references(),
					},
					KnnInferenceOutputContract::new(
						prediction,
						DType::F32,
						prediction_shape,
						output.schema().source_index(),
						KnnInferencePredictionKind::NumericMean,
					),
				)
			}
			KnnReferenceValues::DiscreteI32 { codes, labels } => {
				let values = compiler.external(
					InferenceInputRole::KnnReferenceValues {
						output: output_index,
					},
					DType::I32,
					shape(&[reference_rows])?,
					codes.iter().flat_map(|value| value.to_le_bytes()).collect(),
				)?;
				let prediction = compiler.tensor(DType::I32, prediction_shape.clone())?;
				(
					KnnOutputRequest::Categorical {
						reference_codes: knn_request_input(&compiler, values)?,
						known: knn_request_input(&compiler, known)?,
						predictions: knn_request_output(&compiler, prediction)?,
						known_references: output.known_references(),
						classes: u64::try_from(labels.len()).map_err(|error| {
							InferenceCompileError::new(
								InferenceCompileErrorKind::UnsupportedExtent,
								format!(
									"KNN output {output_index} label count does not fit u64: {error}"
								),
							)
						})?,
					},
					KnnInferenceOutputContract::new(
						prediction,
						DType::I32,
						prediction_shape,
						output.schema().source_index(),
						KnnInferencePredictionKind::DiscreteMode,
					),
				)
			}
		};
		requests.push(request);
		outputs.push(contract);
	}

	let specs = references.operation_specs();
	let requirements = knn_all_output_requirements(
		rows,
		reference_rows,
		feature_width,
		references.neighbors().get(),
		&specs,
	)?;
	let request = KnnAllOutputRequest {
		query_features: knn_request_input(&compiler, query_features)?,
		reference_features: knn_request_input(&compiler, reference_features)?,
		outputs: requests,
		neighbors: references.neighbors().get(),
		tree_lanes: MAXIMUM_REDUCTION_TREE_LANES,
		identity_namespace: IdentityNamespace::new(
			ValueId::new(compiler.next_value),
			requirements.intermediate_values,
			KernelTemplateId::new(compiler.next_kernel),
			requirements.kernels,
		),
		workspace_limit: requirements.workspace_bytes,
	};
	let output_ids = outputs
		.iter()
		.map(KnnInferenceOutputContract::value)
		.collect::<BTreeSet<_>>();
	for tensor in compiler.tensors.values_mut() {
		tensor.external_input = compiler.external_input_ids.contains(&tensor.id);
		tensor.external_output = output_ids.contains(&tensor.id);
	}
	let mut graph = CalculationGraph {
		tensors: compiler.tensors.into_values().collect(),
		nodes: compiler.nodes,
	};
	let materialized = append_knn_all_outputs(&mut graph, &request)?;
	compiler.domains.extend(
		materialized
			.kernels
			.iter()
			.map(|kernel| KernelIterationDomain {
				kernel: *kernel,
				domain: IterationDomain::first(),
			}),
	);
	graph.validate()?;
	let graph = CalculationGraph::from_ogdl(&graph.to_ogdl()?)?;
	let iterations = NonZeroU64::new(1).expect("one KNN inference iteration is nonzero");
	let program = StaticCalculationProgram::new(graph, iterations, compiler.domains)?;
	let program = StaticCalculationProgram::from_ogdl(&program.to_ogdl()?)?;
	Ok(CompiledKnnInference {
		program,
		external_inputs: compiler.external_inputs,
		outputs,
		rows,
	})
}

fn knn_request_input(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone();
	tensor.external_input = true;
	tensor.external_output = false;
	Ok(tensor)
}

fn knn_request_output(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone();
	tensor.external_input = false;
	tensor.external_output = true;
	Ok(tensor)
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
		parameters: &PreparedParameters,
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
			parameters,
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

	fn compile_token_features(
		&mut self,
		data: &PreparedInferenceDataset,
		spans: &[CompiledFeatureSpan],
		rows: u64,
		feature_width: u64,
		vocabulary: u64,
	) -> InferenceCompileResult<ValueId> {
		if data.features().len() != spans.len() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"prepared token-position count differs from the saved embedding sequence",
			));
		}
		let vocabulary = i32::try_from(vocabulary).map_err(|error| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::UnsupportedExtent,
				format!("embedding vocabulary cannot be represented by int32: {error}"),
			)
		})?;
		let total_elements = rows.checked_mul(feature_width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"inference token element count overflowed u64",
			)
		})?;
		require_i32_indexable(total_elements, "inference token element count")?;
		let mut combined = self.zero_i32(shape(&[total_elements])?)?;
		for (feature_index, (feature, span)) in data.features().iter().zip(spans).enumerate() {
			let start = u64::try_from(span.start()).map_err(|error| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::UnsupportedExtent,
					format!("token position cannot be represented by u64: {error}"),
				)
			})?;
			if start != feature_index as u64
				|| span.width() != 1 || span.lowering() != DenseFeatureLowering::NumericScalar
				|| feature.schema().source_vector() != span.source_vector()
				|| feature.values().len() != data.rows()
			{
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("prepared token position {feature_index} disagrees with its saved sequence span"),
				));
			}
			let PreparedInferenceValues::I32(values) = feature.values() else {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("prepared token position {feature_index} did not retain exact int32 IDs"),
				));
			};
			if let Some((row, token)) = values
				.iter()
				.copied()
				.enumerate()
				.find(|(_, token)| *token < 0 || *token >= vocabulary)
			{
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!(
						"inference token {token} at row {row}, position {feature_index} is outside 0..{vocabulary}"
					),
				));
			}
			let raw = self.external(
				InferenceInputRole::Feature {
					feature: feature_index,
					source_vector: span.source_vector(),
				},
				DType::I32,
				shape(&[rows])?,
				values.iter()
					.flat_map(|value| value.to_le_bytes())
					.collect(),
			)?;
			combined = self.scatter_i32_feature_block(combined, raw, rows, feature_width, start)?;
		}
		if u64::try_from(spans.len()).ok() != Some(feature_width) {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"saved embedding feature spans do not cover the fixed sequence width",
			));
		}
		let dense_shape = shape(&[rows, feature_width])?;
		let identity = self.identity_indices(dense_shape.clone())?;
		let dense = self.tensor(DType::I32, dense_shape)?;
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

	fn zero_i32(&mut self, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let output = self.tensor(DType::I32, output_shape)?;
		self.emit(
			Vec::new(),
			vec![output],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
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

	fn scatter_i32_feature_block(
		&mut self,
		base: ValueId,
		block: ValueId,
		rows: u64,
		total_width: u64,
		start: u64,
	) -> InferenceCompileResult<ValueId> {
		let positions = self.identity_indices(shape(&[rows])?)?;
		let destinations = self.tensor(DType::I32, shape(&[rows])?)?;
		self.emit_elementwise(
			vec![positions],
			vec![destinations],
			feature_destination_program(
				checked_i32(total_width, "token sequence width")?,
				checked_i32(start, "token position")?,
				1,
			)?,
		)?;
		let output = self.tensor(DType::I32, self.tensor_ref(base)?.shape.clone())?;
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
		if checkpoint.config().data_normalization == DenseDataNormalization::Identity {
			if !checkpoint.normalization().is_empty() {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					"identity-input checkpoint unexpectedly retains fitted normalization tensors",
				));
			}
			return Ok(input);
		}
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
			DenseDataNormalization::Identity => unreachable!("handled before the normalization mask"),
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

	#[allow(clippy::too_many_arguments)]
	fn normalize_knn_features(
		&mut self,
		query: ValueId,
		reference: ValueId,
		query_rows: u64,
		reference_rows: u64,
		columns: u64,
		normalization: Option<DenseDataNormalization>,
		mask_bits: Option<&[u32]>,
		tree_lanes: u32,
	) -> InferenceCompileResult<(ValueId, ValueId)> {
		let Some(normalization) = normalization else {
			return Ok((query, reference));
		};
		let mask = mask_bits
			.map(|bits| {
				self.external(
					InferenceInputRole::FeatureNormalizationMask,
					DType::F32,
					shape(&[columns])?,
					bits.iter().flat_map(|bits| bits.to_le_bytes()).collect(),
				)
			})
			.transpose()?;
		const EPSILON: f32 = 1.0e-6;
		match normalization {
			DenseDataNormalization::Identity => Ok((query, reference)),
			DenseDataNormalization::ZScore => {
				let column_shape = shape(&[columns])?;
				let sums = self.tensor(DType::F32, column_shape.clone())?;
				self.reduce(
					reference,
					sums,
					ReduceOperator::Sum,
					&[0],
					false,
					tree_lanes,
				)?;
				let means = self.tensor(DType::F32, column_shape.clone())?;
				self.emit_elementwise(
					vec![sums],
					vec![means],
					divide_constant_program(reference_rows as f32)?,
				)?;
				let centered = self.tensor(DType::F32, shape(&[reference_rows, columns])?)?;
				self.emit_elementwise(vec![reference, means], vec![centered], subtract_program()?)?;
				let squares = self.tensor(DType::F32, shape(&[reference_rows, columns])?)?;
				self.emit_elementwise(vec![centered], vec![squares], square_program()?)?;
				let variance_sums = self.tensor(DType::F32, column_shape.clone())?;
				self.reduce(
					squares,
					variance_sums,
					ReduceOperator::Sum,
					&[0],
					false,
					tree_lanes,
				)?;
				let variances = self.tensor(DType::F32, column_shape)?;
				self.emit_elementwise(
					vec![variance_sums],
					vec![variances],
					divide_constant_program(reference_rows as f32)?,
				)?;
				Ok((
					self.apply_knn_z_score(query, means, variances, query_rows, columns, mask, EPSILON)?,
					self.apply_knn_z_score(
						reference,
						means,
						variances,
						reference_rows,
						columns,
						mask,
						EPSILON,
					)?,
				))
			}
			DenseDataNormalization::MinMax => {
				let column_shape = shape(&[columns])?;
				let minimum = self.tensor(DType::F32, column_shape.clone())?;
				self.reduce(
					reference,
					minimum,
					ReduceOperator::Minimum,
					&[0],
					false,
					tree_lanes,
				)?;
				let maximum = self.tensor(DType::F32, column_shape)?;
				self.reduce(
					reference,
					maximum,
					ReduceOperator::Maximum,
					&[0],
					false,
					tree_lanes,
				)?;
				Ok((
					self.apply_knn_min_max(query, minimum, maximum, query_rows, columns, mask, EPSILON)?,
					self.apply_knn_min_max(
						reference,
						minimum,
						maximum,
						reference_rows,
						columns,
						mask,
						EPSILON,
					)?,
				))
			}
			DenseDataNormalization::L2Norm => Ok((
				self.apply_knn_l2(query, query_rows, columns, mask, EPSILON, tree_lanes)?,
				self.apply_knn_l2(
					reference,
					reference_rows,
					columns,
					mask,
					EPSILON,
					tree_lanes,
				)?,
			)),
		}
	}

	fn apply_knn_z_score(
		&mut self,
		input: ValueId,
		mean: ValueId,
		variance: ValueId,
		rows: u64,
		columns: u64,
		mask: Option<ValueId>,
		epsilon: f32,
	) -> InferenceCompileResult<ValueId> {
		let output = self.tensor(DType::F32, shape(&[rows, columns])?)?;
		let mut inputs = vec![input, mean, variance];
		if let Some(mask) = mask {
			inputs.push(mask);
		}
		self.emit_elementwise(
			inputs,
			vec![output],
			z_score_program(epsilon, mask.is_some())?,
		)?;
		Ok(output)
	}

	fn apply_knn_min_max(
		&mut self,
		input: ValueId,
		minimum: ValueId,
		maximum: ValueId,
		rows: u64,
		columns: u64,
		mask: Option<ValueId>,
		epsilon: f32,
	) -> InferenceCompileResult<ValueId> {
		let output = self.tensor(DType::F32, shape(&[rows, columns])?)?;
		let mut inputs = vec![input, minimum, maximum];
		if let Some(mask) = mask {
			inputs.push(mask);
		}
		self.emit_elementwise(
			inputs,
			vec![output],
			min_max_program(epsilon, mask.is_some())?,
		)?;
		Ok(output)
	}

	fn apply_knn_l2(
		&mut self,
		input: ValueId,
		rows: u64,
		columns: u64,
		mask: Option<ValueId>,
		epsilon: f32,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let matrix_shape = shape(&[rows, columns])?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		let mut square_inputs = vec![input];
		if let Some(mask) = mask {
			square_inputs.push(mask);
		}
		self.emit_elementwise(
			square_inputs,
			vec![squares],
			l2_square_program(mask.is_some())?,
		)?;
		let norms_squared = self.tensor(DType::F32, shape(&[rows, 1])?)?;
		self.reduce(
			squares,
			norms_squared,
			ReduceOperator::Sum,
			&[1],
			true,
			tree_lanes,
		)?;
		let output = self.tensor(DType::F32, matrix_shape)?;
		let mut inputs = vec![input, norms_squared];
		if let Some(mask) = mask {
			inputs.push(mask);
		}
		self.emit_elementwise(
			inputs,
			vec![output],
			l2_norm_program(epsilon, mask.is_some())?,
		)?;
		Ok(output)
	}

	fn compile_embedding(
		&mut self,
		block_index: usize,
		embedding: &crate::CheckpointEmbeddingImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
	) -> InferenceCompileResult<ValueId> {
		if embedding.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"embedding block {block_index} sequence length {} differs from input width {input_width}",
					embedding.sequence_length()
				),
			));
		}
		self.require_tensor(
			input,
			DType::I32,
			&[rows, input_width],
			"embedding token matrix",
		)?;
		validate_checkpoint_parameter_image(
			embedding.table().parameter(),
			&[embedding.vocabulary().get(), embedding.dimensions().get()],
			"embedding table",
		)?;
		let table = self.external_checkpoint_tensor(
			InferenceInputRole::EmbeddingTable { block: block_index },
			embedding.table().parameter(),
		)?;
		let token_rows = rows.checked_mul(input_width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"embedding inference token count overflowed u64",
			)
		})?;
		let output_width = input_width
			.checked_mul(embedding.dimensions().get())
			.ok_or_else(|| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					"embedding inference output width overflowed u64",
				)
			})?;
		let indices = self.pack_i32_matrix_to_flat(input, rows, input_width)?;
		let gathered = self.tensor(
			DType::F32,
			shape(&[token_rows, embedding.dimensions().get()])?,
		)?;
		self.emit(
			vec![table, indices],
			vec![gathered],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		let flat = self.pack_matrix_to_flat(gathered, token_rows, embedding.dimensions().get())?;
		let output_shape = shape(&[rows, output_width])?;
		let identity = self.identity_indices(output_shape.clone())?;
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit(
			vec![flat, identity],
			vec![output],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(output)
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_attention(
		&mut self,
		block_index: usize,
		attention: &crate::CheckpointAttentionImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		logical_length: u64,
		logical_channels: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		if attention.sequence_length().get() != logical_length
			|| attention.dimensions().get() != logical_channels
			|| attention
				.heads()
				.get()
				.checked_mul(attention.head_dimension().get())
				!= Some(logical_channels)
		{
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("attention block {block_index} geometry differs from its preceding embedding"),
			));
		}
		let expected_width = logical_length
			.checked_mul(logical_channels)
			.ok_or_else(|| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					"attention inference width overflowed u64",
				)
			})?;
		if input_width != expected_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"attention block {block_index} input width {input_width} differs from fixed sequence width {expected_width}"
				),
			));
		}
		checked_i32(
			checked_product(
				&[rows, logical_length, logical_channels],
				"attention inference element count",
			)?,
			"attention inference element count",
		)?;
		let parameter_shape = [logical_channels, logical_channels];
		for (name, parameter) in [
			("query", attention.query()),
			("key", attention.key()),
			("value", attention.value()),
			("output", attention.output()),
		] {
			validate_checkpoint_parameter_image(
				parameter.parameter(),
				&parameter_shape,
				&format!("attention {name} matrix"),
			)?;
		}
		let query_weight = self.external_checkpoint_tensor(
			InferenceInputRole::AttentionQuery { block: block_index },
			attention.query().parameter(),
		)?;
		let key_weight = self.external_checkpoint_tensor(
			InferenceInputRole::AttentionKey { block: block_index },
			attention.key().parameter(),
		)?;
		let value_weight = self.external_checkpoint_tensor(
			InferenceInputRole::AttentionValue { block: block_index },
			attention.value().parameter(),
		)?;
		let output_weight = self.external_checkpoint_tensor(
			InferenceInputRole::AttentionOutput { block: block_index },
			attention.output().parameter(),
		)?;

		let input_sequence = self.reinterpret_f32(input, shape(&[rows, logical_length, logical_channels])?)?;
		let query = self.attention_projection(
			input_sequence,
			query_weight,
			rows,
			logical_length,
			logical_channels,
		)?;
		let key = self.attention_projection(
			input_sequence,
			key_weight,
			rows,
			logical_length,
			logical_channels,
		)?;
		let value = self.attention_projection(
			input_sequence,
			value_weight,
			rows,
			logical_length,
			logical_channels,
		)?;
		let heads = attention.heads().get();
		let head_dimension = attention.head_dimension().get();
		let head_shape = shape(&[rows, logical_length, heads, head_dimension])?;
		let query = self.reinterpret_f32(query, head_shape.clone())?;
		let key = self.reinterpret_f32(key, head_shape.clone())?;
		let value = self.reinterpret_f32(value, head_shape)?;
		let score_shape = shape(&[rows, heads, logical_length, logical_length])?;
		let scores = self.tensor(DType::F32, score_shape.clone())?;
		self.emit(
			vec![query, key],
			vec![scores],
			PrimitiveKind::Contraction(Contraction {
				batch_axes: vec![(0, 0), (2, 2)],
				contract_axes: vec![(3, 3)],
			}),
			forbidden_aliases(2, 1),
		)?;
		let scaled = self.tensor(DType::F32, score_shape)?;
		self.emit_elementwise(
			vec![scores],
			vec![scaled],
			multiply_constant_program(1.0 / (head_dimension as f32).sqrt())?,
		)?;
		let probabilities = self.causal_softmax(scaled, rows, heads, logical_length, tree_lanes)?;
		let context = self.tensor(
			DType::F32,
			shape(&[rows, heads, logical_length, head_dimension])?,
		)?;
		self.emit(
			vec![probabilities, value],
			vec![context],
			PrimitiveKind::Contraction(Contraction {
				batch_axes: vec![(0, 0), (1, 2)],
				contract_axes: vec![(3, 1)],
			}),
			forbidden_aliases(2, 1),
		)?;
		let context = self.head_major_to_sequence(context, rows, logical_length, heads, head_dimension)?;
		let context = self.reinterpret_f32(context, shape(&[rows, logical_length, logical_channels])?)?;
		let output = self.attention_projection(
			context,
			output_weight,
			rows,
			logical_length,
			logical_channels,
		)?;
		self.reinterpret_f32(output, shape(&[rows, expected_width])?)
	}

	fn compile_rnn(
		&mut self,
		block_index: usize,
		rnn: &crate::CheckpointRnnImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
	) -> InferenceCompileResult<ValueId> {
		if rnn.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"RNN block {block_index} sequence length {} differs from input width {input_width}",
					rnn.sequence_length()
				),
			));
		}
		self.require_tensor(
			input,
			DType::F32,
			&[rows, input_width],
			"vanilla-RNN input sequence",
		)?;
		let width = rnn.width().get();
		validate_checkpoint_parameter_image(
			rnn.input_weight().parameter(),
			&[1, width],
			"RNN input weight",
		)?;
		validate_checkpoint_parameter_image(
			rnn.recurrent_weight().parameter(),
			&[width, width],
			"RNN recurrent weight",
		)?;
		validate_checkpoint_parameter_image(rnn.bias().parameter(), &[width], "RNN bias")?;
		let input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::RnnInputWeight { block: block_index },
			rnn.input_weight().parameter(),
		)?;
		let recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::RnnRecurrentWeight { block: block_index },
			rnn.recurrent_weight().parameter(),
		)?;
		let bias = self.external_checkpoint_tensor(
			InferenceInputRole::RnnBias { block: block_index },
			rnn.bias().parameter(),
		)?;
		let mut hidden = self.zero_f32(shape(&[rows, width])?)?;
		for step in 0..input_width {
			let step_input = self.gather_matrix_column(input, rows, input_width, step)?;
			let input_projection = self.bias_free_linear(step_input, input_weight, rows, width)?;
			let recurrent_projection = self.bias_free_linear(hidden, recurrent_weight, rows, width)?;
			let preactivation = self.tensor(DType::F32, shape(&[rows, width])?)?;
			self.emit_elementwise(
				vec![input_projection, recurrent_projection, bias],
				vec![preactivation],
				sum_program(3)?,
			)?;
			hidden = self.apply_activation(
				preactivation,
				DenseActivation::Tanh,
				None,
				shape(&[rows, width])?,
			)?;
		}
		Ok(hidden)
	}

	fn compile_gru(
		&mut self,
		block_index: usize,
		gru: &crate::CheckpointGruImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
	) -> InferenceCompileResult<ValueId> {
		if gru.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"GRU block {block_index} sequence length {} differs from input width {input_width}",
					gru.sequence_length()
				),
			));
		}
		self.require_tensor(
			input,
			DType::F32,
			&[rows, input_width],
			"GRU input sequence",
		)?;
		let width = gru.width().get();
		for (name, parameter) in [
			("reset input weight", gru.reset_input_weight()),
			("update input weight", gru.update_input_weight()),
			("candidate input weight", gru.candidate_input_weight()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[1, width], &format!("GRU {name}"))?;
		}
		for (name, parameter) in [
			("reset recurrent weight", gru.reset_recurrent_weight()),
			("update recurrent weight", gru.update_recurrent_weight()),
			(
				"candidate recurrent weight",
				gru.candidate_recurrent_weight(),
			),
		] {
			validate_checkpoint_parameter_image(
				parameter.parameter(),
				&[width, width],
				&format!("GRU {name}"),
			)?;
		}
		for (name, parameter) in [
			("reset bias", gru.reset_bias()),
			("update bias", gru.update_bias()),
			("candidate bias", gru.candidate_bias()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[width], &format!("GRU {name}"))?;
		}

		let reset_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruResetInputWeight { block: block_index },
			gru.reset_input_weight().parameter(),
		)?;
		let reset_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruResetRecurrentWeight { block: block_index },
			gru.reset_recurrent_weight().parameter(),
		)?;
		let reset_bias = self.external_checkpoint_tensor(
			InferenceInputRole::GruResetBias { block: block_index },
			gru.reset_bias().parameter(),
		)?;
		let update_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruUpdateInputWeight { block: block_index },
			gru.update_input_weight().parameter(),
		)?;
		let update_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruUpdateRecurrentWeight { block: block_index },
			gru.update_recurrent_weight().parameter(),
		)?;
		let update_bias = self.external_checkpoint_tensor(
			InferenceInputRole::GruUpdateBias { block: block_index },
			gru.update_bias().parameter(),
		)?;
		let candidate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruCandidateInputWeight { block: block_index },
			gru.candidate_input_weight().parameter(),
		)?;
		let candidate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruCandidateRecurrentWeight { block: block_index },
			gru.candidate_recurrent_weight().parameter(),
		)?;
		let candidate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::GruCandidateBias { block: block_index },
			gru.candidate_bias().parameter(),
		)?;

		let hidden_shape = shape(&[rows, width])?;
		let mut hidden = self.zero_f32(hidden_shape.clone())?;
		for step in 0..input_width {
			let step_input = self.gather_matrix_column(input, rows, input_width, step)?;
			let reset_input_projection = self.bias_free_linear(step_input, reset_input_weight, rows, width)?;
			let reset_recurrent_projection =
				self.bias_free_linear(hidden, reset_recurrent_weight, rows, width)?;
			let reset_preactivation = self.tensor(DType::F32, hidden_shape.clone())?;
			self.emit_elementwise(
				vec![
					reset_input_projection,
					reset_recurrent_projection,
					reset_bias,
				],
				vec![reset_preactivation],
				sum_program(3)?,
			)?;
			let reset = self.apply_activation(
				reset_preactivation,
				DenseActivation::Sigmoid,
				None,
				hidden_shape.clone(),
			)?;

			let update_input_projection = self.bias_free_linear(step_input, update_input_weight, rows, width)?;
			let update_recurrent_projection =
				self.bias_free_linear(hidden, update_recurrent_weight, rows, width)?;
			let update_preactivation = self.tensor(DType::F32, hidden_shape.clone())?;
			self.emit_elementwise(
				vec![
					update_input_projection,
					update_recurrent_projection,
					update_bias,
				],
				vec![update_preactivation],
				sum_program(3)?,
			)?;
			let update = self.apply_activation(
				update_preactivation,
				DenseActivation::Sigmoid,
				None,
				hidden_shape.clone(),
			)?;

			let reset_hidden = self.tensor(DType::F32, hidden_shape.clone())?;
			self.emit_elementwise(vec![reset, hidden], vec![reset_hidden], multiply_program()?)?;
			let candidate_input_projection =
				self.bias_free_linear(step_input, candidate_input_weight, rows, width)?;
			let candidate_recurrent_projection =
				self.bias_free_linear(reset_hidden, candidate_recurrent_weight, rows, width)?;
			let candidate_preactivation = self.tensor(DType::F32, hidden_shape.clone())?;
			self.emit_elementwise(
				vec![
					candidate_input_projection,
					candidate_recurrent_projection,
					candidate_bias,
				],
				vec![candidate_preactivation],
				sum_program(3)?,
			)?;
			let candidate = self.apply_activation(
				candidate_preactivation,
				DenseActivation::Tanh,
				None,
				hidden_shape.clone(),
			)?;
			let next_hidden = self.tensor(DType::F32, hidden_shape.clone())?;
			self.emit_elementwise(
				vec![candidate, update, hidden],
				vec![next_hidden],
				gru_hidden_program()?,
			)?;
			hidden = next_hidden;
		}
		Ok(hidden)
	}

	fn compile_lstm(
		&mut self,
		block_index: usize,
		lstm: &crate::CheckpointLstmImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
	) -> InferenceCompileResult<ValueId> {
		if lstm.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"LSTM block {block_index} sequence length {} differs from input width {input_width}",
					lstm.sequence_length()
				),
			));
		}
		self.require_tensor(
			input,
			DType::F32,
			&[rows, input_width],
			"LSTM input sequence",
		)?;
		let width = lstm.width().get();
		for (name, parameter) in [
			("input gate input weight", lstm.input_gate_input_weight()),
			("forget gate input weight", lstm.forget_gate_input_weight()),
			("output gate input weight", lstm.output_gate_input_weight()),
			("candidate input weight", lstm.candidate_input_weight()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[1, width], &format!("LSTM {name}"))?;
		}
		for (name, parameter) in [
			(
				"input gate recurrent weight",
				lstm.input_gate_recurrent_weight(),
			),
			(
				"forget gate recurrent weight",
				lstm.forget_gate_recurrent_weight(),
			),
			(
				"output gate recurrent weight",
				lstm.output_gate_recurrent_weight(),
			),
			(
				"candidate recurrent weight",
				lstm.candidate_recurrent_weight(),
			),
		] {
			validate_checkpoint_parameter_image(
				parameter.parameter(),
				&[width, width],
				&format!("LSTM {name}"),
			)?;
		}
		for (name, parameter) in [
			("input gate bias", lstm.input_gate_bias()),
			("forget gate bias", lstm.forget_gate_bias()),
			("output gate bias", lstm.output_gate_bias()),
			("candidate bias", lstm.candidate_bias()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[width], &format!("LSTM {name}"))?;
		}

		let input_gate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmInputGateInputWeight { block: block_index },
			lstm.input_gate_input_weight().parameter(),
		)?;
		let input_gate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmInputGateRecurrentWeight { block: block_index },
			lstm.input_gate_recurrent_weight().parameter(),
		)?;
		let input_gate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmInputGateBias { block: block_index },
			lstm.input_gate_bias().parameter(),
		)?;
		let forget_gate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmForgetGateInputWeight { block: block_index },
			lstm.forget_gate_input_weight().parameter(),
		)?;
		let forget_gate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmForgetGateRecurrentWeight { block: block_index },
			lstm.forget_gate_recurrent_weight().parameter(),
		)?;
		let forget_gate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmForgetGateBias { block: block_index },
			lstm.forget_gate_bias().parameter(),
		)?;
		let output_gate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmOutputGateInputWeight { block: block_index },
			lstm.output_gate_input_weight().parameter(),
		)?;
		let output_gate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmOutputGateRecurrentWeight { block: block_index },
			lstm.output_gate_recurrent_weight().parameter(),
		)?;
		let output_gate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmOutputGateBias { block: block_index },
			lstm.output_gate_bias().parameter(),
		)?;
		let candidate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmCandidateInputWeight { block: block_index },
			lstm.candidate_input_weight().parameter(),
		)?;
		let candidate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmCandidateRecurrentWeight { block: block_index },
			lstm.candidate_recurrent_weight().parameter(),
		)?;
		let candidate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmCandidateBias { block: block_index },
			lstm.candidate_bias().parameter(),
		)?;

		let state_shape = shape(&[rows, width])?;
		let mut hidden = self.zero_f32(state_shape.clone())?;
		let mut cell = self.zero_f32(state_shape.clone())?;
		for step in 0..input_width {
			let step_input = self.gather_matrix_column(input, rows, input_width, step)?;
			let input_gate = self.compile_lstm_gate(
				step_input,
				hidden,
				input_gate_input_weight,
				input_gate_recurrent_weight,
				input_gate_bias,
				DenseActivation::Sigmoid,
				rows,
				width,
				&state_shape,
			)?;
			let forget_gate = self.compile_lstm_gate(
				step_input,
				hidden,
				forget_gate_input_weight,
				forget_gate_recurrent_weight,
				forget_gate_bias,
				DenseActivation::Sigmoid,
				rows,
				width,
				&state_shape,
			)?;
			let output_gate = self.compile_lstm_gate(
				step_input,
				hidden,
				output_gate_input_weight,
				output_gate_recurrent_weight,
				output_gate_bias,
				DenseActivation::Sigmoid,
				rows,
				width,
				&state_shape,
			)?;
			let candidate = self.compile_lstm_gate(
				step_input,
				hidden,
				candidate_input_weight,
				candidate_recurrent_weight,
				candidate_bias,
				DenseActivation::Tanh,
				rows,
				width,
				&state_shape,
			)?;
			let next_cell = self.tensor(DType::F32, state_shape.clone())?;
			self.emit_elementwise(
				vec![forget_gate, cell, input_gate, candidate],
				vec![next_cell],
				lstm_cell_program()?,
			)?;
			let cell_activation =
				self.apply_activation(next_cell, DenseActivation::Tanh, None, state_shape.clone())?;
			let next_hidden = self.tensor(DType::F32, state_shape.clone())?;
			self.emit_elementwise(
				vec![output_gate, cell_activation],
				vec![next_hidden],
				multiply_program()?,
			)?;
			cell = next_cell;
			hidden = next_hidden;
		}
		Ok(hidden)
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_lstm_gate(
		&mut self,
		step_input: ValueId,
		hidden: ValueId,
		input_weight: ValueId,
		recurrent_weight: ValueId,
		bias: ValueId,
		activation: DenseActivation,
		rows: u64,
		width: u64,
		state_shape: &Shape,
	) -> InferenceCompileResult<ValueId> {
		let input_projection = self.bias_free_linear(step_input, input_weight, rows, width)?;
		let recurrent_projection = self.bias_free_linear(hidden, recurrent_weight, rows, width)?;
		let preactivation = self.tensor(DType::F32, state_shape.clone())?;
		self.emit_elementwise(
			vec![input_projection, recurrent_projection, bias],
			vec![preactivation],
			sum_program(3)?,
		)?;
		self.apply_activation(preactivation, activation, None, state_shape.clone())
	}

	fn gather_matrix_column(
		&mut self,
		matrix: ValueId,
		rows: u64,
		columns: u64,
		column: u64,
	) -> InferenceCompileResult<ValueId> {
		if column >= columns {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("matrix column {column} is outside width {columns}"),
			));
		}
		self.require_tensor(matrix, DType::F32, &[rows, columns], "matrix column source")?;
		let index = self.tensor(DType::I32, shape(&[1])?)?;
		self.emit(
			Vec::new(),
			vec![index],
			PrimitiveKind::IndexMap(IndexMap {
				start: checked_i32(column, "matrix column")?,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
		let gathered = self.tensor(DType::F32, shape(&[rows, 1])?)?;
		self.emit(
			vec![matrix, index],
			vec![gathered],
			PrimitiveKind::Gather(Gather {
				axis: 1,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(gathered)
	}

	fn attention_projection(
		&mut self,
		input: ValueId,
		weight: ValueId,
		rows: u64,
		sequence: u64,
		dimensions: u64,
	) -> InferenceCompileResult<ValueId> {
		let output = self.tensor(DType::F32, shape(&[rows, sequence, dimensions])?)?;
		self.emit(
			vec![input, weight],
			vec![output],
			PrimitiveKind::Contraction(Contraction {
				batch_axes: Vec::new(),
				contract_axes: vec![(2, 0)],
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(output)
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
			&PreparedParameters::new(),
		)?;
		let mut current = preactivation;
		let mut prelu = layer.prelu().iter().enumerate();
		for operation in layer.declaration().operations().iter().copied() {
			current = match operation {
				DenseOperation::Activation(activation) => {
					let alpha = if activation == DenseActivation::PRelu {
						let (occurrence, parameter) = prelu.next().ok_or_else(|| {
							InferenceCompileError::new(
								InferenceCompileErrorKind::InconsistentCheckpoint,
								format!("layer {layer_index} omitted a PReLU scalar"),
							)
						})?;
						Some(self.external_checkpoint_tensor(
							InferenceInputRole::LayerPRelu {
								layer: layer_index,
								occurrence,
							},
							parameter.parameter(),
						)?)
					} else {
						None
					};
					self.apply_activation(current, activation, alpha, output_shape.clone())?
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
		if prelu.next().is_some() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("layer {layer_index} retains an extra PReLU scalar"),
			));
		}
		Ok(current)
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_convolution(
		&mut self,
		block_index: usize,
		convolution: &crate::CheckpointConvolutionImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		logical_length: u64,
		logical_channels: u64,
		normalization_epsilon: f32,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let geometry = convolution.geometry();
		if geometry.input_length().get() != logical_length
			|| geometry.input_channels().get() != logical_channels
			|| geometry.input_width().map(NonZeroU64::get) != Some(input_width)
		{
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"convolution block {block_index} input geometry disagrees with the preceding logical shape"
				),
			));
		}
		let output_width = geometry.output_width().ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				format!("convolution block {block_index} output width overflowed u64"),
			)
		})?;
		self.require_tensor(input, DType::F32, &[rows, input_width], "convolution input")?;
		validate_checkpoint_parameter_image(
			convolution.weight().parameter(),
			&[
				geometry.kernel().get(),
				geometry.input_channels().get(),
				geometry.filters().get(),
			],
			"convolution weight",
		)?;
		validate_checkpoint_parameter_image(
			convolution.bias().parameter(),
			&[geometry.filters().get()],
			"convolution bias",
		)?;
		let preparation = prepare_channelwise_convolution_1d(
			rows,
			logical_length,
			logical_channels,
			geometry.filters().get(),
			geometry.kernel().get(),
		)?;
		if preparation.output_length() != geometry.output_length().get() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("convolution block {block_index} preparation disagrees with its saved output length"),
			));
		}
		let flat_input = self.pack_matrix_to_flat(input, rows, input_width)?;
		let window_indices = self.external_i32(
			InferenceInputRole::ConvolutionWindowIndices { block: block_index },
			shape(&preparation.window_indices_shape())?,
			preparation.window_indices(),
		)?;
		let columns = self.tensor(DType::F32, shape(&preparation.window_indices_shape())?)?;
		self.emit(
			vec![flat_input, window_indices],
			vec![columns],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		let weight = self.external_checkpoint_tensor(
			InferenceInputRole::ConvolutionWeight { block: block_index },
			convolution.weight().parameter(),
		)?;
		let bias = self.external_checkpoint_tensor(
			InferenceInputRole::ConvolutionBias { block: block_index },
			convolution.bias().parameter(),
		)?;
		let contracted = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		self.emit(
			vec![columns, weight],
			vec![contracted],
			PrimitiveKind::Contraction(Contraction {
				batch_axes: Vec::new(),
				contract_axes: vec![(2, 0), (3, 1)],
			}),
			forbidden_aliases(2, 1),
		)?;
		let grouped = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		self.emit_elementwise(vec![contracted, bias], vec![grouped], add_program()?)?;
		let mut current = self.unpack_pool_to_matrix(
			grouped,
			rows,
			geometry.output_length().get(),
			geometry.filters().get(),
			output_width.get(),
		)?;
		let mut prelu = convolution.prelu().iter().enumerate();
		for operation in convolution.declaration().operations().iter().copied() {
			let alpha = if operation == DenseOperation::Activation(DenseActivation::PRelu) {
				let (occurrence, parameter) = prelu.next().ok_or_else(|| {
					InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						format!("convolution block {block_index} omitted a PReLU scalar"),
					)
				})?;
				validate_checkpoint_parameter_image(parameter.parameter(), &[1], "convolution PReLU scalar")?;
				Some(self.external_checkpoint_tensor(
					InferenceInputRole::ConvolutionPRelu {
						block: block_index,
						occurrence,
					},
					parameter.parameter(),
				)?)
			} else {
				None
			};
			current = self.apply_saved_operation(
				current,
				operation,
				alpha,
				rows,
				output_width.get(),
				normalization_epsilon,
				tree_lanes,
			)?;
		}
		if prelu.next().is_some() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("convolution block {block_index} retains an extra PReLU scalar"),
			));
		}
		Ok(current)
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_pool(
		&mut self,
		block_index: usize,
		pool: &crate::CheckpointPoolImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		logical_length: u64,
		logical_channels: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		if pool.input_width().get() != input_width
			|| pool.input_length().get() != logical_length
			|| pool.channels().get() != logical_channels
		{
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"pool block {block_index} expects width {} as logical {} x {}, received width {input_width} as logical {logical_length} x {logical_channels}",
					pool.input_width(),
					pool.input_length(),
					pool.channels(),
				),
			));
		}
		let expected_input_width = logical_length
			.checked_mul(logical_channels)
			.ok_or_else(|| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					format!("pool block {block_index} input width overflowed u64"),
				)
			})?;
		let expected_output_width = pool
			.output_length()
			.get()
			.checked_mul(logical_channels)
			.ok_or_else(|| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					format!("pool block {block_index} output width overflowed u64"),
				)
			})?;
		if expected_input_width != input_width || expected_output_width != pool.output_width().get() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("pool block {block_index} cached widths disagree with its logical shape"),
			));
		}
		self.require_tensor(
			input,
			DType::F32,
			&[rows, input_width],
			"maximum-pool input",
		)?;
		let preparation =
			prepare_channelwise_max_pool_1d(rows, logical_length, logical_channels, pool.size().get())?;
		if preparation.groups() != pool.output_length().get() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("pool block {block_index} preparation disagrees with its saved output length"),
			));
		}
		let flat_input = self.pack_matrix_to_flat(input, rows, input_width)?;
		let window_indices = self.external_i32(
			InferenceInputRole::PoolWindowIndices { block: block_index },
			shape(&preparation.window_indices_shape())?,
			preparation.window_indices(),
		)?;
		let winner_bases = self.external_i32(
			InferenceInputRole::PoolWinnerBases { block: block_index },
			shape(&preparation.output_shape())?,
			preparation.winner_bases(),
		)?;
		let pooled = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		let unused_winners = self.tensor(DType::I32, shape(&preparation.output_shape())?)?;
		let parameters = preparation.forward_parameters(u64::from(tree_lanes));
		self.materialize(
			"recipe_max_pool_1d",
			&[
				("values", flat_input),
				("window_indices", window_indices),
				("winner_bases", winner_bases),
			],
			&[("pooled", pooled), ("winning_indices", unused_winners)],
			"values",
			&parameters,
		)?;
		self.unpack_pool_to_matrix(
			pooled,
			rows,
			pool.output_length().get(),
			logical_channels,
			pool.output_width().get(),
		)
	}

	fn compile_kmeans(
		&mut self,
		block_index: usize,
		kmeans: &crate::CheckpointKMeansImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		if kmeans.input_width().get() != input_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"K-means block {block_index} expects input width {}, received {input_width}",
					kmeans.input_width()
				),
			));
		}
		self.require_tensor(input, DType::F32, &[rows, input_width], "K-means input")?;
		validate_checkpoint_parameter_image(
			kmeans.centroids(),
			&[kmeans.clusters().get(), input_width],
			"K-means centroids",
		)?;
		let centroids = self.external_checkpoint_tensor(
			InferenceInputRole::KMeansCentroids { block: block_index },
			kmeans.centroids(),
		)?;
		let distances = self.tensor(DType::F32, shape(&[rows, kmeans.clusters().get()])?)?;
		let parameters = [
			("queries".to_owned(), PreparedParameter::U64(rows)),
			(
				"training_rows".to_owned(),
				PreparedParameter::U64(kmeans.clusters().get()),
			),
			("dimensions".to_owned(), PreparedParameter::U64(input_width)),
			(
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)),
			),
		]
		.into_iter()
		.collect::<PreparedParameters>();
		self.materialize(
			"gpu_pairwise_l2",
			&[("query", input), ("training", centroids)],
			&[("distances", distances)],
			"query",
			&parameters,
		)?;
		Ok(distances)
	}

	fn compile_tree(
		&mut self,
		block_index: usize,
		tree: &crate::CheckpointTreeImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		if tree.input_width().get() != input_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"tree block {block_index} expects input width {}, received {input_width}",
					tree.input_width()
				),
			));
		}
		self.require_tensor(input, DType::F32, &[rows, input_width], "tree input")?;
		let declaration = tree.declaration();
		let requirements = tree_ensemble_inference_requirements(
			rows,
			input_width,
			declaration.trees().get(),
			declaration.depth().get(),
			tree.output_width().get(),
		)?;
		if requirements.internal_nodes_per_tree != tree.internal_nodes_per_tree().get()
			|| requirements.leaves_per_tree != tree.leaves_per_tree().get()
		{
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("tree block {block_index} cached extents differ from its declaration"),
			));
		}
		let split_features = self.external_checkpoint_tensor(
			InferenceInputRole::TreeSplitFeatures { block: block_index },
			tree.split_features(),
		)?;
		let split_thresholds = self.external_checkpoint_tensor(
			InferenceInputRole::TreeSplitThresholds { block: block_index },
			tree.split_thresholds(),
		)?;
		let leaf_values = self.external_checkpoint_tensor(
			InferenceInputRole::TreeLeafValues { block: block_index },
			tree.leaf_values().parameter(),
		)?;
		let flat_features = self.pack_matrix_to_flat(input, rows, input_width)?;
		let predictions = self.tensor(DType::F32, shape(&[rows, tree.output_width().get()])?)?;
		let first_value = self.next_value;
		let first_kernel = self.next_kernel;
		self.next_value = self
			.next_value
			.checked_add(requirements.intermediate_values)
			.ok_or_else(identity_exhausted)?;
		self.next_kernel = self
			.next_kernel
			.checked_add(requirements.kernels)
			.ok_or_else(identity_exhausted)?;
		let request = TreeEnsembleInferenceRequest {
			features: self.tensor_ref(flat_features)?.clone(),
			split_features: self.tensor_ref(split_features)?.clone(),
			split_thresholds: self.tensor_ref(split_thresholds)?.clone(),
			leaf_values: self.tensor_ref(leaf_values)?.clone(),
			predictions: self.tensor_ref(predictions)?.clone(),
			rows,
			feature_width: input_width,
			trees: declaration.trees().get(),
			depth: declaration.depth().get(),
			outputs: tree.output_width().get(),
			scale: 1.0,
			tree_lanes,
			identity_namespace: IdentityNamespace::new(
				ValueId::new(first_value),
				requirements.intermediate_values,
				KernelTemplateId::new(first_kernel),
				requirements.kernels,
			),
			workspace_limit: requirements.workspace_bytes,
		};
		let materialized = materialize_tree_ensemble_inference(&request)?;
		for tensor in &materialized.graph.tensors {
			self.insert_tensor_contract(tensor.clone())?;
		}
		for node in &materialized.graph.nodes {
			self.nodes.push(node.clone());
			self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id,
				domain: IterationDomain::first(),
			});
		}
		Ok(predictions)
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_residual(
		&mut self,
		block_index: usize,
		residual: &crate::CheckpointResidualImage,
		input: ValueId,
		rows: u64,
		input_width: u64,
		layer_index: &mut usize,
		normalization_epsilon: f32,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		self.require_tensor(input, DType::F32, &[rows, input_width], "residual input")?;
		let mut branch = input;
		let mut branch_width = input_width;
		let mut retained_layer = false;
		let mut branch_prelu = residual.branch_prelu().iter().enumerate();
		for step in residual.branch() {
			match step {
				crate::CheckpointResidualBranchImage::Layer(layer) => {
					retained_layer = true;
					branch = self.compile_layer(
						*layer_index,
						layer,
						branch,
						rows,
						branch_width,
						normalization_epsilon,
						tree_lanes,
					)?;
					*layer_index = layer_index.checked_add(1).ok_or_else(identity_exhausted)?;
					branch_width = layer.declaration().width().get();
				}
				crate::CheckpointResidualBranchImage::Operation(operation) => {
					let alpha = if *operation == DenseOperation::Activation(DenseActivation::PRelu) {
						let (occurrence, parameter) = branch_prelu.next().ok_or_else(|| {
							InferenceCompileError::new(
								InferenceCompileErrorKind::InconsistentCheckpoint,
								format!("residual block {block_index} omitted a branch PReLU scalar"),
							)
						})?;
						validate_checkpoint_parameter_image(
							parameter.parameter(),
							&[1],
							"residual branch PReLU scalar",
						)?;
						Some(self.external_checkpoint_tensor(
							InferenceInputRole::ResidualBranchPRelu {
								block: block_index,
								occurrence,
							},
							parameter.parameter(),
						)?)
					} else {
						None
					};
					branch = self.apply_saved_operation(
						branch,
						*operation,
						alpha,
						rows,
						branch_width,
						normalization_epsilon,
						tree_lanes,
					)?;
				}
			}
		}
		if branch_prelu.next().is_some() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("residual block {block_index} retains an extra branch PReLU scalar"),
			));
		}
		let output_width = residual.output_width().get();
		if !retained_layer || branch_width != output_width {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("residual block {block_index} output width disagrees with its last branch layer"),
			));
		}
		let skip = match residual.skip() {
			crate::CheckpointResidualSkipImage::Identity if input_width == output_width => input,
			crate::CheckpointResidualSkipImage::Projection(projection) if input_width != output_width => {
				validate_checkpoint_parameter_image(
					projection.parameter(),
					&[input_width, output_width],
					"residual projection weight",
				)?;
				let weight = self.external_checkpoint_tensor(
					InferenceInputRole::ResidualProjectionWeight { block: block_index },
					projection.parameter(),
				)?;
				self.bias_free_linear(input, weight, rows, output_width)?
			}
			crate::CheckpointResidualSkipImage::Identity => {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("residual block {block_index} width mismatch requires a projection"),
				));
			}
			crate::CheckpointResidualSkipImage::Projection(_) => {
				return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("equal-width residual block {block_index} must use an identity skip"),
				));
			}
		};
		let mut current = self.exact_add(branch, skip)?;
		let mut output_prelu = residual.prelu().iter().enumerate();
		for operation in residual.operations().iter().copied() {
			let alpha = if operation == DenseOperation::Activation(DenseActivation::PRelu) {
				let (occurrence, parameter) = output_prelu.next().ok_or_else(|| {
					InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						format!("residual block {block_index} omitted an output PReLU scalar"),
					)
				})?;
				validate_checkpoint_parameter_image(
					parameter.parameter(),
					&[1],
					"residual output PReLU scalar",
				)?;
				Some(self.external_checkpoint_tensor(
					InferenceInputRole::ResidualOutputPRelu {
						block: block_index,
						occurrence,
					},
					parameter.parameter(),
				)?)
			} else {
				None
			};
			current = self.apply_saved_operation(
				current,
				operation,
				alpha,
				rows,
				output_width,
				normalization_epsilon,
				tree_lanes,
			)?;
		}
		if output_prelu.next().is_some() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("residual block {block_index} retains an extra output PReLU scalar"),
			));
		}
		Ok(current)
	}

	#[allow(clippy::too_many_arguments)]
	fn apply_saved_operation(
		&mut self,
		input: ValueId,
		operation: DenseOperation,
		prelu: Option<ValueId>,
		rows: u64,
		width: u64,
		normalization_epsilon: f32,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		match operation {
			DenseOperation::Activation(activation) => {
				self.apply_activation(input, activation, prelu, shape(&[rows, width])?)
			}
			DenseOperation::Normalization(normalization) => self.apply_model_normalization(
				input,
				normalization,
				rows,
				width,
				normalization_epsilon,
				tree_lanes,
			),
		}
	}

	fn external_i32(
		&mut self,
		role: InferenceInputRole,
		shape: Shape,
		values: &[i32],
	) -> InferenceCompileResult<ValueId> {
		self.external(
			role,
			DType::I32,
			shape,
			values.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		)
	}

	fn identity_indices(&mut self, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let indices = self.tensor(DType::I32, output_shape)?;
		self.emit(
			Vec::new(),
			vec![indices],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
		)?;
		Ok(indices)
	}

	fn pack_contiguous_f32_to_flat(&mut self, input: ValueId) -> InferenceCompileResult<ValueId> {
		let contract = self.tensor_ref(input)?.clone();
		if contract.dtype != DType::F32 {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::Language,
				"attention tensor reinterpretation requires binary32 input",
			));
		}
		if contract.shape.rank() == 1 {
			return Ok(input);
		}
		let elements = contract.shape.elements();
		checked_i32(elements, "attention tensor element count")?;
		let indices = self.identity_indices(contract.shape.clone())?;
		let base = self.zero_f32(shape(&[elements])?)?;
		let flat = self.tensor(DType::F32, shape(&[elements])?)?;
		self.emit(
			vec![base, indices, input],
			vec![flat],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
		)?;
		Ok(flat)
	}

	fn reinterpret_f32(&mut self, input: ValueId, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let flat = self.pack_contiguous_f32_to_flat(input)?;
		if self.tensor_ref(flat)?.shape.elements() != output_shape.elements() {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"attention tensor reinterpretation changed its element count",
			));
		}
		let indices = self.identity_indices(output_shape.clone())?;
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit(
			vec![flat, indices],
			vec![output],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(output)
	}

	fn head_major_to_sequence(
		&mut self,
		input: ValueId,
		rows: u64,
		sequence: u64,
		heads: u64,
		head_dimension: u64,
	) -> InferenceCompileResult<ValueId> {
		self.require_tensor(
			input,
			DType::F32,
			&[rows, heads, sequence, head_dimension],
			"inference head-major attention tensor",
		)?;
		let output_shape = shape(&[rows, sequence, heads, head_dimension])?;
		let positions = self.identity_indices(output_shape.clone())?;
		let indices = self.tensor(DType::I32, output_shape.clone())?;
		self.emit_elementwise(
			vec![positions],
			vec![indices],
			head_major_source_index_program(sequence, heads, head_dimension)?,
		)?;
		let flat = self.pack_contiguous_f32_to_flat(input)?;
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit(
			vec![flat, indices],
			vec![output],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(output)
	}

	fn causal_softmax(
		&mut self,
		scores: ValueId,
		rows: u64,
		heads: u64,
		sequence: u64,
		tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let attention_rows = checked_product(&[rows, heads, sequence], "attention softmax row count")?;
		let matrix_shape = shape(&[attention_rows, sequence])?;
		let scores = self.reinterpret_f32(scores, matrix_shape.clone())?;
		let positions = self.identity_indices(matrix_shape.clone())?;
		let mask = self.tensor(DType::I32, matrix_shape.clone())?;
		self.emit_elementwise(vec![positions], vec![mask], causal_mask_program(sequence)?)?;
		let softmax = self.tensor(DType::F32, matrix_shape)?;
		let parameters = [
			("rows".to_owned(), PreparedParameter::U64(attention_rows)),
			("columns".to_owned(), PreparedParameter::U64(sequence)),
			(
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)),
			),
			(
				"causal_mask_verified".to_owned(),
				PreparedParameter::Bool(true),
			),
		]
		.into_iter()
		.collect::<PreparedParameters>();
		self.materialize(
			"gpu_causal_softmax_rows",
			&[("values", scores), ("causal_mask", mask)],
			&[("softmax", softmax)],
			"values",
			&parameters,
		)?;
		self.reinterpret_f32(softmax, shape(&[rows, heads, sequence, sequence])?)
	}

	fn pack_matrix_to_flat(&mut self, input: ValueId, rows: u64, width: u64) -> InferenceCompileResult<ValueId> {
		self.require_tensor(input, DType::F32, &[rows, width], "pool matrix pack input")?;
		let elements = rows.checked_mul(width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"pool matrix pack element count overflowed u64",
			)
		})?;
		let matrix_indices = self.identity_indices(shape(&[rows, width])?)?;
		let base = self.zero_f32(shape(&[elements])?)?;
		let flat = self.tensor(DType::F32, shape(&[elements])?)?;
		self.emit(
			vec![base, matrix_indices, input],
			vec![flat],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
		)?;
		Ok(flat)
	}

	fn pack_i32_matrix_to_flat(&mut self, input: ValueId, rows: u64, width: u64) -> InferenceCompileResult<ValueId> {
		self.require_tensor(input, DType::I32, &[rows, width], "int32 matrix pack input")?;
		let elements = rows.checked_mul(width).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"int32 matrix pack element count overflowed u64",
			)
		})?;
		let matrix_indices = self.identity_indices(shape(&[rows, width])?)?;
		let base = self.zero_i32(shape(&[elements])?)?;
		let flat = self.tensor(DType::I32, shape(&[elements])?)?;
		self.emit(
			vec![base, matrix_indices, input],
			vec![flat],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
		)?;
		Ok(flat)
	}

	fn unpack_pool_to_matrix(
		&mut self,
		pooled: ValueId,
		rows: u64,
		groups: u64,
		channels: u64,
		output_width: u64,
	) -> InferenceCompileResult<ValueId> {
		self.require_tensor(
			pooled,
			DType::F32,
			&[rows, groups, channels],
			"pool grouped output",
		)?;
		if groups.checked_mul(channels) != Some(output_width) {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"pool grouped output width disagrees with its logical shape",
			));
		}
		let group_indices = self.identity_indices(shape(&[groups, channels])?)?;
		let base = self.zero_f32(shape(&[rows, output_width])?)?;
		let output = self.tensor(DType::F32, shape(&[rows, output_width])?)?;
		self.emit(
			vec![base, group_indices, pooled],
			vec![output],
			PrimitiveKind::Scatter(Scatter {
				axis: 1,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
		)?;
		Ok(output)
	}

	fn bias_free_linear(
		&mut self,
		input: ValueId,
		weight: ValueId,
		rows: u64,
		output_width: u64,
	) -> InferenceCompileResult<ValueId> {
		let output = self.tensor(DType::F32, shape(&[rows, output_width])?)?;
		self.emit(
			vec![input, weight],
			vec![output],
			PrimitiveKind::Contraction(Contraction {
				batch_axes: Vec::new(),
				contract_axes: vec![(1, 0)],
			}),
			forbidden_aliases(2, 1),
		)?;
		Ok(output)
	}

	fn exact_add(&mut self, left: ValueId, right: ValueId) -> InferenceCompileResult<ValueId> {
		let left_tensor = self.tensor_ref(left)?;
		let left_dtype = left_tensor.dtype;
		let left_shape = left_tensor.shape.clone();
		let right_tensor = self.tensor_ref(right)?;
		if left_dtype != right_tensor.dtype || left_shape != right_tensor.shape {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"residual addition requires exactly equal dtypes and shapes",
			));
		}
		let output = self.tensor(left_dtype, left_shape)?;
		self.emit_elementwise(vec![left, right], vec![output], add_program()?)?;
		Ok(output)
	}

	fn require_tensor(
		&self,
		value: ValueId,
		dtype: DType,
		extents: &[u64],
		role: &str,
	) -> InferenceCompileResult<()> {
		let tensor = self.tensor_ref(value)?;
		if tensor.dtype != dtype || tensor.shape.extents() != extents {
			return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!(
					"{role} has {:?} shape {:?}, expected {dtype:?} shape {extents:?}",
					tensor.dtype,
					tensor.shape.extents(),
				),
			));
		}
		Ok(())
	}

	fn apply_activation(
		&mut self,
		input: ValueId,
		activation: DenseActivation,
		prelu: Option<ValueId>,
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
				self.emit_owned_scalar("gpu_log_into", vec![absolute], vec![magnitude])?;
				let sign = self.tensor(DType::F32, output_shape)?;
				self.emit_owned_scalar("gpu_sign_into", vec![input], vec![sign])?;
				self.emit_elementwise(vec![sign, magnitude], vec![output], multiply_program()?)?;
			}
			DenseActivation::NaturalLogarithm => {
				self.emit_owned_scalar("gpu_log_into", vec![input], vec![output])?;
			}
			DenseActivation::LegacySignedLogOnePlus => {
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
			DenseActivation::LeakyRelu => {
				self.emit_elementwise(vec![input], vec![output], canonical_leaky_relu_program()?)?;
			}
			DenseActivation::Sigmoid => {
				self.emit_owned_scalar("gpu_sigmoid_into", vec![input], vec![output])?;
			}
			DenseActivation::Tanh => {
				self.emit_owned_scalar("gpu_tanh_into", vec![input], vec![output])?;
			}
			DenseActivation::Selu => {
				self.emit_elementwise(vec![input], vec![output], canonical_selu_program()?)?;
			}
			DenseActivation::Gelu => {
				self.emit_owned_scalar("gpu_gelu_into", vec![input], vec![output])?;
			}
			DenseActivation::Silu => {
				self.emit_owned_scalar("gpu_silu_into", vec![input], vec![output])?;
			}
			DenseActivation::Elu => {
				self.emit_elementwise(vec![input], vec![output], canonical_elu_program()?)?;
			}
			DenseActivation::PRelu => {
				let alpha = prelu.ok_or_else(|| {
					InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"PReLU inference omitted its learned scalar",
					)
				})?;
				self.emit_elementwise(vec![input, alpha], vec![output], canonical_prelu_program()?)?;
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
			DenseTask::MultiTargetBinaryClassification { target_count, .. } => {
				if u64::try_from(target_count).ok() != Some(columns) {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"multi-target binary logit width differs from saved target count",
					));
				}
				let probabilities = self.stable_sigmoid(values)?;
				Ok((
					probabilities,
					InferencePredictionKind::MultiTargetBinaryProbabilities,
				))
			}
			DenseTask::JointMulticlassClassification { target_count, .. } => {
				if u64::try_from(target_count).ok() != Some(columns) {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"joint target logit width differs from saved target count",
					));
				}
				let probabilities = self.stable_softmax(values, rows, columns, tree_lanes)?;
				Ok((
					probabilities,
					InferencePredictionKind::JointTargetProbabilities,
				))
			}
			DenseTask::MultiTargetRegression { target_count, .. } => {
				if u64::try_from(target_count).ok() != Some(columns) {
					return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"multi-target regression width differs from saved target count",
					));
				}
				Ok((values, InferencePredictionKind::MultiTargetRegression))
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
		task: InferenceTask,
		target_dtypes: Vec<DType>,
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
				target_dtypes,
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

fn checked_product(values: &[u64], name: &str) -> InferenceCompileResult<u64> {
	values.iter().copied().try_fold(1_u64, |product, value| {
		product.checked_mul(value).ok_or_else(|| {
			InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				format!("{name} overflowed u64"),
			)
		})
	})
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

fn causal_mask_program(sequence_length: u64) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let sequence = builder.i32(checked_i32(
		sequence_length,
		"causal-attention sequence length",
	)?)?;
	let key = builder.binary(ScalarOpcode::Remainder, position, sequence)?;
	let row = builder.binary(ScalarOpcode::Divide, position, sequence)?;
	let query = builder.binary(ScalarOpcode::Remainder, row, sequence)?;
	let visible = builder.binary(ScalarOpcode::LessThanOrEqual, key, query)?;
	Ok(builder.finish(&[visible])?)
}

fn head_major_source_index_program(
	sequence_length: u64,
	heads: u64,
	head_dimension: u64,
) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let sequence = builder.i32(checked_i32(sequence_length, "attention sequence length")?)?;
	let heads = builder.i32(checked_i32(heads, "attention head count")?)?;
	let dimension = builder.i32(checked_i32(head_dimension, "attention head dimension")?)?;
	let channel = builder.binary(ScalarOpcode::Remainder, position, dimension)?;
	let packed = builder.binary(ScalarOpcode::Divide, position, dimension)?;
	let head = builder.binary(ScalarOpcode::Remainder, packed, heads)?;
	let sequence_packed = builder.binary(ScalarOpcode::Divide, packed, heads)?;
	let token = builder.binary(ScalarOpcode::Remainder, sequence_packed, sequence)?;
	let row = builder.binary(ScalarOpcode::Divide, sequence_packed, sequence)?;
	let source = builder.binary(ScalarOpcode::Multiply, row, heads)?;
	let source = builder.binary(ScalarOpcode::Add, source, head)?;
	let source = builder.binary(ScalarOpcode::Multiply, source, sequence)?;
	let source = builder.binary(ScalarOpcode::Add, source, token)?;
	let source = builder.binary(ScalarOpcode::Multiply, source, dimension)?;
	let source = builder.binary(ScalarOpcode::Add, source, channel)?;
	Ok(builder.finish(&[source])?)
}

fn multiply_constant_program(constant: f32) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let constant = builder.f32(constant)?;
	let result = builder.binary(ScalarOpcode::Multiply, value, constant)?;
	Ok(builder.finish(&[result])?)
}

fn multiply_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn gru_hidden_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let candidate = builder.input(DType::F32)?;
	let update = builder.input(DType::F32)?;
	let previous = builder.input(DType::F32)?;
	let difference = builder.binary(ScalarOpcode::Subtract, previous, candidate)?;
	let retained = builder.binary(ScalarOpcode::Multiply, update, difference)?;
	let hidden = builder.binary(ScalarOpcode::Add, candidate, retained)?;
	Ok(builder.finish(&[hidden])?)
}

fn lstm_cell_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let forget = builder.input(DType::F32)?;
	let previous_cell = builder.input(DType::F32)?;
	let input = builder.input(DType::F32)?;
	let candidate = builder.input(DType::F32)?;
	let retained = builder.binary(ScalarOpcode::Multiply, forget, previous_cell)?;
	let admitted = builder.binary(ScalarOpcode::Multiply, input, candidate)?;
	let cell = builder.binary(ScalarOpcode::Add, retained, admitted)?;
	Ok(builder.finish(&[cell])?)
}

fn add_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Add, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn sum_program(inputs: usize) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let mut values = Vec::with_capacity(inputs);
	for _ in 0..inputs {
		values.push(builder.input(DType::F32)?);
	}
	let mut sum = *values.first().ok_or_else(|| {
		InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"scalar sum requires at least one input",
		)
	})?;
	for value in values.into_iter().skip(1) {
		sum = builder.binary(ScalarOpcode::Add, sum, value)?;
	}
	Ok(builder.finish(&[sum])?)
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
	saved_feature_schema_from_parts(checkpoint.vectors(), checkpoint.feature_spans())
}

fn saved_feature_schema_from_parts(
	vectors: &[CheckpointArtifactVector],
	spans: &[CompiledFeatureSpan],
) -> InferencePreparationResult<Vec<InferenceFeatureSchema>> {
	spans.iter()
		.enumerate()
		.map(|(feature, span)| {
			let vector = vectors
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

	pub(crate) fn inference_boundary_allowed_roles() -> [InferenceInputRole; 68] {
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
			InferenceInputRole::LayerPRelu {
				layer: 0,
				occurrence: 0,
			},
			InferenceInputRole::EmbeddingTable { block: 0 },
			InferenceInputRole::AttentionQuery { block: 0 },
			InferenceInputRole::AttentionKey { block: 0 },
			InferenceInputRole::AttentionValue { block: 0 },
			InferenceInputRole::AttentionOutput { block: 0 },
			InferenceInputRole::RnnInputWeight { block: 0 },
			InferenceInputRole::RnnRecurrentWeight { block: 0 },
			InferenceInputRole::RnnBias { block: 0 },
			InferenceInputRole::GruResetInputWeight { block: 0 },
			InferenceInputRole::GruResetRecurrentWeight { block: 0 },
			InferenceInputRole::GruResetBias { block: 0 },
			InferenceInputRole::GruUpdateInputWeight { block: 0 },
			InferenceInputRole::GruUpdateRecurrentWeight { block: 0 },
			InferenceInputRole::GruUpdateBias { block: 0 },
			InferenceInputRole::GruCandidateInputWeight { block: 0 },
			InferenceInputRole::GruCandidateRecurrentWeight { block: 0 },
			InferenceInputRole::GruCandidateBias { block: 0 },
			InferenceInputRole::LstmInputGateInputWeight { block: 0 },
			InferenceInputRole::LstmInputGateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmInputGateBias { block: 0 },
			InferenceInputRole::LstmForgetGateInputWeight { block: 0 },
			InferenceInputRole::LstmForgetGateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmForgetGateBias { block: 0 },
			InferenceInputRole::LstmOutputGateInputWeight { block: 0 },
			InferenceInputRole::LstmOutputGateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmOutputGateBias { block: 0 },
			InferenceInputRole::LstmCandidateInputWeight { block: 0 },
			InferenceInputRole::LstmCandidateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmCandidateBias { block: 0 },
			InferenceInputRole::ConvolutionWindowIndices { block: 0 },
			InferenceInputRole::ConvolutionWeight { block: 0 },
			InferenceInputRole::ConvolutionBias { block: 0 },
			InferenceInputRole::ConvolutionPRelu {
				block: 0,
				occurrence: 0,
			},
			InferenceInputRole::PoolWindowIndices { block: 0 },
			InferenceInputRole::PoolWinnerBases { block: 0 },
			InferenceInputRole::KMeansCentroids { block: 0 },
			InferenceInputRole::TreeSplitFeatures { block: 0 },
			InferenceInputRole::TreeSplitThresholds { block: 0 },
			InferenceInputRole::TreeLeafValues { block: 0 },
			InferenceInputRole::ResidualProjectionWeight { block: 0 },
			InferenceInputRole::ResidualBranchPRelu {
				block: 0,
				occurrence: 0,
			},
			InferenceInputRole::ResidualOutputPRelu {
				block: 0,
				occurrence: 0,
			},
			InferenceInputRole::Temperature,
			InferenceInputRole::KnnReferenceFeatures,
			InferenceInputRole::KnnReferenceValues { output: 0 },
			InferenceInputRole::KnnReferenceKnown { output: 0 },
			InferenceInputRole::BayesQueryParents { conditional: 0 },
			InferenceInputRole::BayesReferenceParents { conditional: 0 },
			InferenceInputRole::BayesReferenceChild { conditional: 0 },
			InferenceInputRole::BayesParentMultipliers { conditional: 0 },
			InferenceInputRole::BayesParentCardinalities { conditional: 0 },
			InferenceInputRole::BayesConcatenationLeftIndices { join: 1 },
			InferenceInputRole::BayesConcatenationRightIndices { join: 1 },
			InferenceInputRole::BayesConcatenationSelectLeft { join: 1 },
			InferenceInputRole::GgufTokenIds,
			InferenceInputRole::GgufTensor { tensor: 0 },
			InferenceInputRole::GgufRopePartnerIndices,
			InferenceInputRole::GgufRopeCosines,
			InferenceInputRole::GgufRopeSignedSines,
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
			| InferenceInputRole::LayerPRelu { .. }
			| InferenceInputRole::EmbeddingTable { .. }
			| InferenceInputRole::AttentionQuery { .. }
			| InferenceInputRole::AttentionKey { .. }
			| InferenceInputRole::AttentionValue { .. }
			| InferenceInputRole::AttentionOutput { .. }
			| InferenceInputRole::RnnInputWeight { .. }
			| InferenceInputRole::RnnRecurrentWeight { .. }
			| InferenceInputRole::RnnBias { .. }
			| InferenceInputRole::GruResetInputWeight { .. }
			| InferenceInputRole::GruResetRecurrentWeight { .. }
			| InferenceInputRole::GruResetBias { .. }
			| InferenceInputRole::GruUpdateInputWeight { .. }
			| InferenceInputRole::GruUpdateRecurrentWeight { .. }
			| InferenceInputRole::GruUpdateBias { .. }
			| InferenceInputRole::GruCandidateInputWeight { .. }
			| InferenceInputRole::GruCandidateRecurrentWeight { .. }
			| InferenceInputRole::GruCandidateBias { .. }
			| InferenceInputRole::LstmInputGateInputWeight { .. }
			| InferenceInputRole::LstmInputGateRecurrentWeight { .. }
			| InferenceInputRole::LstmInputGateBias { .. }
			| InferenceInputRole::LstmForgetGateInputWeight { .. }
			| InferenceInputRole::LstmForgetGateRecurrentWeight { .. }
			| InferenceInputRole::LstmForgetGateBias { .. }
			| InferenceInputRole::LstmOutputGateInputWeight { .. }
			| InferenceInputRole::LstmOutputGateRecurrentWeight { .. }
			| InferenceInputRole::LstmOutputGateBias { .. }
			| InferenceInputRole::LstmCandidateInputWeight { .. }
			| InferenceInputRole::LstmCandidateRecurrentWeight { .. }
			| InferenceInputRole::LstmCandidateBias { .. }
			| InferenceInputRole::ConvolutionWindowIndices { .. }
			| InferenceInputRole::ConvolutionWeight { .. }
			| InferenceInputRole::ConvolutionBias { .. }
			| InferenceInputRole::ConvolutionPRelu { .. }
			| InferenceInputRole::PoolWindowIndices { .. }
			| InferenceInputRole::PoolWinnerBases { .. }
			| InferenceInputRole::KMeansCentroids { .. }
			| InferenceInputRole::TreeSplitFeatures { .. }
			| InferenceInputRole::TreeSplitThresholds { .. }
			| InferenceInputRole::TreeLeafValues { .. }
			| InferenceInputRole::ResidualProjectionWeight { .. }
			| InferenceInputRole::ResidualBranchPRelu { .. }
			| InferenceInputRole::ResidualOutputPRelu { .. }
			| InferenceInputRole::Temperature
			| InferenceInputRole::KnnReferenceFeatures
			| InferenceInputRole::KnnReferenceValues { .. }
			| InferenceInputRole::KnnReferenceKnown { .. }
			| InferenceInputRole::BayesQueryParents { .. }
			| InferenceInputRole::BayesReferenceParents { .. }
			| InferenceInputRole::BayesReferenceChild { .. }
			| InferenceInputRole::BayesParentMultipliers { .. }
			| InferenceInputRole::BayesParentCardinalities { .. }
			| InferenceInputRole::BayesConcatenationLeftIndices { .. }
			| InferenceInputRole::BayesConcatenationRightIndices { .. }
			| InferenceInputRole::BayesConcatenationSelectLeft { .. }
			| InferenceInputRole::GgufTokenIds
			| InferenceInputRole::GgufTensor { .. }
			| InferenceInputRole::GgufRopePartnerIndices
			| InferenceInputRole::GgufRopeCosines
			| InferenceInputRole::GgufRopeSignedSines => role,
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
				InferenceTask::Dense(DenseTask::ScalarRegression { target_vector: 0 }),
				vec![DType::F32],
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
		assert_eq!(first.output().target_dtype(), DType::F32);
		assert_eq!(first.output().shape().extents(), [2, 1]);
		assert_eq!(external_output_count(&first), 1);
		assert_eq!(
			first.program().iterations(),
			recipe_core::LoopIterations::ONE
		);
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
	fn prelu_inference_admits_saved_scalars_in_occurrence_order() {
		let encoded = crate::checkpoint::encoded_prelu_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();

		let slopes = compiled
			.external_inputs()
			.iter()
			.filter(|input| matches!(input.role(), InferenceInputRole::LayerPRelu { .. }))
			.collect::<Vec<_>>();
		assert_eq!(slopes.len(), 2);
		assert_eq!(
			slopes.iter().map(|input| input.role()).collect::<Vec<_>>(),
			[
				InferenceInputRole::LayerPRelu {
					layer: 0,
					occurrence: 0,
				},
				InferenceInputRole::LayerPRelu {
					layer: 0,
					occurrence: 1,
				},
			]
		);
		assert_eq!(slopes[0].bytes(), 0.25f32.to_le_bytes());
		assert_eq!(slopes[1].bytes(), 0.5f32.to_le_bytes());

		let slope_values = slopes
			.iter()
			.map(|input| input.value())
			.collect::<BTreeSet<_>>();
		let expected_program = canonical_prelu_program().unwrap();
		let applications = compiled
			.graph()
			.nodes
			.iter()
			.filter(|node| {
				matches!(
					&node.kernel.kind,
					PrimitiveKind::Elementwise(elementwise)
						if elementwise.program == expected_program
							&& node.kernel.inputs.get(1).is_some_and(|value| slope_values.contains(value))
				)
			})
			.count();
		assert_eq!(applications, 2);
	}

	#[test]
	fn logarithm_inference_preserves_signed_natural_and_legacy_program_identities() {
		let encoded = crate::checkpoint::encoded_logarithm_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		let signed_and_natural = ScalarProgram::try_from(recipe_math::MathFunction::Log).unwrap();
		let legacy = ScalarProgram::try_from(recipe_math::MathFunction::LogOnePlus).unwrap();

		let signed_and_natural_count = compiled
			.graph()
			.nodes
			.iter()
			.filter(|node| {
				matches!(
					&node.kernel.kind,
					PrimitiveKind::Elementwise(elementwise) if elementwise.program == signed_and_natural
				)
			})
			.count();
		let legacy_count = compiled
			.graph()
			.nodes
			.iter()
			.filter(|node| {
				matches!(
					&node.kernel.kind,
					PrimitiveKind::Elementwise(elementwise) if elementwise.program == legacy
				)
			})
			.count();

		assert_eq!(signed_and_natural_count, 2);
		assert_eq!(legacy_count, 1);
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
	fn embedding_inference_keeps_token_ids_int32_and_gathers_the_saved_table() {
		let encoded = crate::checkpoint::encoded_embedding_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n3\n7\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		let feature = compiled
			.external_inputs()
			.iter()
			.find(|input| matches!(input.role(), InferenceInputRole::Feature { .. }))
			.expect("raw token feature exists");
		assert_eq!(feature.dtype(), DType::I32);
		assert_eq!(feature.shape().extents(), [2]);
		let table = compiled
			.external_inputs()
			.iter()
			.find(|input| {
				matches!(
					input.role(),
					InferenceInputRole::EmbeddingTable { block: 0 }
				)
			})
			.expect("saved embedding table exists");
		assert_eq!(table.dtype(), DType::F32);
		assert_eq!(table.shape().extents(), [8, 2]);
		assert!(
			compiled
				.external_inputs()
				.iter()
				.all(|input| input.role() != InferenceInputRole::FeatureNormalizationMask)
		);
		assert!(compiled.graph().nodes.iter().any(|node| {
			matches!(node.kernel.kind, PrimitiveKind::Gather(_))
				&& node.kernel.inputs.first() == Some(&table.value())
		}));
		assert_eq!(compiled.output().shape().extents(), [2, 1]);
		assert_eq!(external_output_count(&compiled), 1);

		let out_of_range = prepared_fixture(&encoded, b"\"feature\nbytes\"\n8\n");
		let error = compile_prepared_inference(&out_of_range).unwrap_err();
		assert_eq!(
			error.kind(),
			InferenceCompileErrorKind::InconsistentCheckpoint
		);
		assert!(error.detail().contains("outside 0..8"));
	}

	#[test]
	fn attention_inference_loads_all_projections_and_materializes_causal_softmax() {
		let encoded = crate::checkpoint::encoded_attention_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n3\n7\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		for role in [
			InferenceInputRole::AttentionQuery { block: 1 },
			InferenceInputRole::AttentionKey { block: 1 },
			InferenceInputRole::AttentionValue { block: 1 },
			InferenceInputRole::AttentionOutput { block: 1 },
		] {
			let parameter = compiled
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.expect("saved attention projection exists");
			assert_eq!(parameter.dtype(), DType::F32);
			assert_eq!(parameter.shape().extents(), [2, 2]);
		}
		assert!(compiled.graph().nodes.iter().any(|node| {
			matches!(
				&node.kernel.kind,
				PrimitiveKind::Contraction(contraction)
					if contraction.batch_axes == [(0, 0), (2, 2)]
						&& contraction.contract_axes == [(3, 3)]
			)
		}));
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
		assert_eq!(compiled.output().shape().extents(), [2, 1]);
		assert_eq!(external_output_count(&compiled), 1);
	}

	#[test]
	fn rnn_inference_loads_recurrent_parameters_and_returns_only_the_final_hidden_state() {
		let encoded = crate::checkpoint::encoded_rnn_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n2.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		let recurrent_weight = compiled
			.external_inputs()
			.iter()
			.find(|input| input.role() == InferenceInputRole::RnnRecurrentWeight { block: 0 })
			.expect("saved RNN recurrent weight exists");
		assert_eq!(recurrent_weight.dtype(), DType::F32);
		assert_eq!(recurrent_weight.shape().extents(), [2, 2]);
		for (role, expected_shape) in [
			(InferenceInputRole::RnnInputWeight { block: 0 }, &[1, 2][..]),
			(InferenceInputRole::RnnBias { block: 0 }, &[2][..]),
		] {
			let parameter = compiled
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.expect("saved RNN parameter exists");
			assert_eq!(parameter.shape().extents(), expected_shape);
		}
		assert!(compiled.graph().nodes.iter().any(|node| {
			matches!(node.kernel.kind, PrimitiveKind::Contraction(_))
				&& node.kernel.inputs.get(1) == Some(&recurrent_weight.value())
		}));
		assert_eq!(compiled.output().shape().extents(), [2, 1]);
		assert_eq!(external_output_count(&compiled), 1);
	}

	#[test]
	fn gru_inference_loads_all_gates_and_returns_only_the_final_hidden_state() {
		let encoded = crate::checkpoint::encoded_gru_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n2.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		for (role, expected_shape) in [
			(
				InferenceInputRole::GruResetInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::GruResetRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(InferenceInputRole::GruResetBias { block: 0 }, &[2][..]),
			(
				InferenceInputRole::GruUpdateInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::GruUpdateRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(InferenceInputRole::GruUpdateBias { block: 0 }, &[2][..]),
			(
				InferenceInputRole::GruCandidateInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::GruCandidateRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(InferenceInputRole::GruCandidateBias { block: 0 }, &[2][..]),
		] {
			let parameter = compiled
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.expect("saved GRU parameter exists");
			assert_eq!(parameter.dtype(), DType::F32);
			assert_eq!(parameter.shape().extents(), expected_shape);
		}
		for role in [
			InferenceInputRole::GruResetRecurrentWeight { block: 0 },
			InferenceInputRole::GruUpdateRecurrentWeight { block: 0 },
			InferenceInputRole::GruCandidateRecurrentWeight { block: 0 },
		] {
			let recurrent_weight = compiled
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.expect("saved GRU recurrent weight exists");
			assert!(compiled.graph().nodes.iter().any(|node| {
				matches!(node.kernel.kind, PrimitiveKind::Contraction(_))
					&& node.kernel.inputs.get(1) == Some(&recurrent_weight.value())
			}));
		}
		assert_eq!(compiled.output().shape().extents(), [2, 1]);
		assert_eq!(external_output_count(&compiled), 1);
	}

	#[test]
	fn lstm_inference_loads_all_gates_and_returns_only_the_final_hidden_state() {
		let encoded = crate::checkpoint::encoded_lstm_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n2.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();
		for (role, expected_shape) in [
			(
				InferenceInputRole::LstmInputGateInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::LstmInputGateRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(InferenceInputRole::LstmInputGateBias { block: 0 }, &[2][..]),
			(
				InferenceInputRole::LstmForgetGateInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::LstmForgetGateRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(
				InferenceInputRole::LstmForgetGateBias { block: 0 },
				&[2][..],
			),
			(
				InferenceInputRole::LstmOutputGateInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::LstmOutputGateRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(
				InferenceInputRole::LstmOutputGateBias { block: 0 },
				&[2][..],
			),
			(
				InferenceInputRole::LstmCandidateInputWeight { block: 0 },
				&[1, 2][..],
			),
			(
				InferenceInputRole::LstmCandidateRecurrentWeight { block: 0 },
				&[2, 2][..],
			),
			(InferenceInputRole::LstmCandidateBias { block: 0 }, &[2][..]),
		] {
			let parameter = compiled
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.expect("saved LSTM parameter exists");
			assert_eq!(parameter.dtype(), DType::F32);
			assert_eq!(parameter.shape().extents(), expected_shape);
		}
		for role in [
			InferenceInputRole::LstmInputGateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmForgetGateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmOutputGateRecurrentWeight { block: 0 },
			InferenceInputRole::LstmCandidateRecurrentWeight { block: 0 },
		] {
			let recurrent_weight = compiled
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.expect("saved LSTM recurrent weight exists");
			assert!(compiled.graph().nodes.iter().any(|node| {
				matches!(node.kernel.kind, PrimitiveKind::Contraction(_))
					&& node.kernel.inputs.get(1) == Some(&recurrent_weight.value())
			}));
		}
		assert_eq!(compiled.output().shape().extents(), [2, 1]);
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
		assert_eq!(compiled.output().target_dtype(), DType::F32);
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
	fn multi_target_inference_preserves_saved_width_and_prediction_interpretation() {
		for (task, expected_kind) in [
			(
				DenseTask::MultiTargetBinaryClassification {
					first_target_vector: 2,
					target_count: 3,
				},
				InferencePredictionKind::MultiTargetBinaryProbabilities,
			),
			(
				DenseTask::JointMulticlassClassification {
					first_target_vector: 2,
					target_count: 3,
				},
				InferencePredictionKind::JointTargetProbabilities,
			),
			(
				DenseTask::MultiTargetRegression {
					first_target_vector: 2,
					target_count: 3,
				},
				InferencePredictionKind::MultiTargetRegression,
			),
		] {
			let encoded = crate::checkpoint::encoded_multi_target_test_checkpoint_fixture(task);
			let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n2.5\n");
			assert_eq!(prepared.checkpoint().target_source_indices(), [2, 1, 3]);
			let compiled = compile_prepared_inference(&prepared).unwrap();

			assert_eq!(compiled.task(), InferenceTask::Dense(task));
			assert_eq!(compiled.output().kind(), expected_kind);
			assert_eq!(compiled.output().dtype(), DType::F32);
			assert_eq!(compiled.output().target_dtype(), DType::F32);
			assert_eq!(compiled.output().target_dtypes(), [DType::F32; 3]);
			assert_eq!(compiled.output().shape().extents(), [2, 3]);
			assert_eq!(external_output_count(&compiled), 1);
			compiled.graph().validate().unwrap();
			compiled.program().validate().unwrap();
		}
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
	fn structured_residual_checkpoint_compiles_branch_projection_merge_and_adapter() {
		let encoded = crate::checkpoint::encoded_residual_test_checkpoint_fixture();
		let prepared = prepared_fixture(&encoded, b"\"feature\nbytes\"\n1.5\n");
		let compiled = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(compiled.output().shape().extents(), [1, 1]);
		assert!(
			compiled
				.external_inputs()
				.iter()
				.any(|input| { input.role() == InferenceInputRole::ResidualProjectionWeight { block: 0 } })
		);
		assert!(
			compiled
				.external_inputs()
				.iter()
				.any(|input| { input.role() == InferenceInputRole::LayerWeight { layer: 0 } })
		);
		assert!(
			compiled
				.external_inputs()
				.iter()
				.any(|input| { input.role() == InferenceInputRole::LayerWeight { layer: 1 } })
		);
		assert!(
			compiled
				.graph()
				.nodes
				.iter()
				.filter(|node| matches!(node.kernel.kind, PrimitiveKind::Contraction(_)))
				.count() >= 3
		);
	}

	#[test]
	fn structured_pool_checkpoint_compiles_saved_shape_and_bounded_index_images() {
		let encoded = crate::checkpoint::encoded_pool_test_checkpoint_fixture();
		let prepared = prepared_fixture(
			&encoded,
			b"feature-0,feature-1,feature-2,feature-3,feature-4,feature-5\n1,2,3,4,5,6\n",
		);
		let first = compile_prepared_inference(&prepared).unwrap();
		let second = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(first.output().shape().extents(), [1, 1]);
		let windows = first
			.external_inputs()
			.iter()
			.find(|input| input.role() == InferenceInputRole::PoolWindowIndices { block: 0 })
			.unwrap();
		assert_eq!(windows.dtype(), DType::I32);
		assert_eq!(windows.shape().extents(), [1, 3, 1, 2]);
		let bases = first
			.external_inputs()
			.iter()
			.find(|input| input.role() == InferenceInputRole::PoolWinnerBases { block: 0 })
			.unwrap();
		assert_eq!(bases.shape().extents(), [1, 3, 1]);
		assert_eq!(
			first.program().to_ogdl().unwrap(),
			second.program().to_ogdl().unwrap()
		);
	}

	#[test]
	fn structured_kmeans_checkpoint_compiles_saved_centroid_distances() {
		let encoded = crate::checkpoint::encoded_kmeans_test_checkpoint_fixture();
		let prepared = prepared_fixture(
			&encoded,
			b"feature-0,feature-1,feature-2,feature-3\n1,2,3,4\n",
		);
		let first = compile_prepared_inference(&prepared).unwrap();
		let second = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(first.output().shape().extents(), [1, 1]);
		let centroids = first
			.external_inputs()
			.iter()
			.find(|input| input.role() == InferenceInputRole::KMeansCentroids { block: 0 })
			.expect("saved K-means centroid input exists");
		assert_eq!(centroids.dtype(), DType::F32);
		assert_eq!(centroids.shape().extents(), [3, 4]);
		assert!(
			first.graph()
				.nodes
				.iter()
				.any(|node| matches!(node.kernel.kind, PrimitiveKind::Contraction(_)))
		);
		assert_eq!(
			first.program().to_ogdl().unwrap(),
			second.program().to_ogdl().unwrap()
		);
	}

	#[test]
	fn structured_convolution_checkpoint_compiles_saved_geometry_and_parameter_images() {
		let encoded = crate::checkpoint::encoded_convolution_test_checkpoint_fixture();
		let prepared = prepared_fixture(
			&encoded,
			b"feature-0,feature-1,feature-2,feature-3\n1,2,3,4\n",
		);
		let first = compile_prepared_inference(&prepared).unwrap();
		let second = compile_prepared_inference(&prepared).unwrap();

		assert_eq!(first.output().shape().extents(), [1, 1]);
		for (role, shape) in [
			(
				InferenceInputRole::ConvolutionWindowIndices { block: 0 },
				&[1, 3, 2, 1][..],
			),
			(
				InferenceInputRole::ConvolutionWeight { block: 0 },
				&[2, 1, 2][..],
			),
			(InferenceInputRole::ConvolutionBias { block: 0 }, &[2][..]),
			(
				InferenceInputRole::ConvolutionWindowIndices { block: 1 },
				&[1, 2, 2, 2][..],
			),
			(
				InferenceInputRole::ConvolutionWeight { block: 1 },
				&[2, 2, 3][..],
			),
			(InferenceInputRole::ConvolutionBias { block: 1 }, &[3][..]),
			(
				InferenceInputRole::ConvolutionPRelu {
					block: 1,
					occurrence: 0,
				},
				&[1][..],
			),
		] {
			let input = first
				.external_inputs()
				.iter()
				.find(|input| input.role() == role)
				.unwrap_or_else(|| panic!("missing saved convolution input {role:?}"));
			assert_eq!(input.shape().extents(), shape);
		}
		assert!(
			first.external_inputs()
				.iter()
				.any(|input| { input.role() == InferenceInputRole::PoolWindowIndices { block: 2 } })
		);
		assert_eq!(
			first.program().to_ogdl().unwrap(),
			second.program().to_ogdl().unwrap()
		);
	}
}
