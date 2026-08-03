use core::{fmt, num::NonZeroU64}; use alloc::collections::{BTreeMap, BTreeSet}; use std::{ path::Path, };

use recipe_core::{ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId}; use recipe_ingest::{
	InferenceFeatureEncoding, InferenceFeatureSchema, InferencePrepareError, PreparedInferenceDataset,
	PreparedInferenceFeature, PreparedInferenceValues, RawTable, SemanticType, SourceError,
	SourceLimit, VectorEncoding, read_source_snapshot, }; use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather, IndexBounds, IndexMap,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarProgramBuilder,
	Scatter, ScatterConflict, Shape, Tensor, }; use recipe_ops::{
	CategoricalBayesInferenceRequest, IdentityNamespace, KnnAllOutputRequest, KnnOutputRequest,
	MaterializationRequest, NamedTensor, PreparedParameter, PreparedParameters, TreeEnsembleInferenceRequest,
	append_categorical_bayes_inference, append_knn_all_outputs, categorical_bayes_inference_requirements,
	knn_all_output_requirements, lower_scalar, materialize_composition, materialize_tree_ensemble_inference,
	operation_registry, prepare_channelwise_convolution_1d, prepare_channelwise_max_pool_1d,
	tree_ensemble_inference_requirements, };
use recipe_program::{IterationDomain, KernelIterationDomain, StaticCalculationProgram};

use crate::{ BayesModelArtifact, BayesModelDecodeLimits, CheckpointArtifact, CheckpointArtifactMetadata,
	CheckpointArtifactVector, CheckpointDecodeErrorKind, CheckpointDecodeLimits, CheckpointError, CheckpointPath,
	CheckpointTensorImage, CompiledFeatureSpan, DenseActivation, DenseDataNormalization, DenseFeatureLowering,
	DenseNormalization, DenseOperation, DenseOutputAdapter, DenseTask, KnnModelArtifact, KnnModelDecodeLimits,
	MAXIMUM_REDUCTION_TREE_LANES, checkpoint::decode_error, decode_bayes_model, decode_checkpoint, decode_knn_model,
	forward::{ ForwardActivation, GruForwardParameters, LstmForwardParameters, RecurrentForwardGraph,
		RecurrentGateParameters, add_program, causal_mask_program, divide_constant_program, divide_program,
		forbidden_aliases, head_major_source_index_program, l2_norm_program, l2_square_program,
		lower_gru_sequence, lower_lstm_sequence, lower_rnn_sequence, min_max_program, multiply_constant_program,
		multiply_program, square_program, subtract_program, z_score_program, }, };

#[path = "gguf_llama.rs"]
mod gguf_llama;

pub use gguf_llama::{
	GgufLlamaArtifact, GgufLlamaError, GgufLlamaErrorKind, GgufLlamaResult, PreparedGgufLlamaInference,
	compile_prepared_gguf_llama_inference, decode_gguf_llama, load_gguf_llama_model_file,
	prepare_gguf_llama_inference_table, };

const MATERIALIZATION_RESERVATION: u64 = 64; const WORKSPACE_LIMIT: ByteCount = ByteCount::new(u64::MAX);

#[derive(Debug)]
#[non_exhaustive]
pub enum InferencePreparationError { CheckpointSource(SourceError), Checkpoint(CheckpointError),
	GgufLlama(GgufLlamaError), Data(InferencePrepareError), InconsistentCheckpoint { feature: usize, source_vector: usize,
		detail: String, }, ArithmeticOverflow { detail: String, }, }

impl fmt::Display for InferencePreparationError {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CheckpointSource(error) => write!(formatter, "load checkpoint source: {error}"),
			Self::Checkpoint(error) => write!(formatter, "load checkpoint: {error}"),
			Self::GgufLlama(error) => write!(formatter, "load GGUF llama model: {error}"),
			Self::Data(error) => write!(formatter, "prepare inference data: {error}"),
			Self::InconsistentCheckpoint { feature, source_vector, detail, } => { write!( formatter,
					"inconsistent checkpoint inference.feature[{feature}].source-vector[{source_vector}]: {detail}"
				) }
			Self::ArithmeticOverflow { detail } => write!(formatter, "prepare inference data: {detail}"),
		} } }

impl core::error::Error for InferencePreparationError {
	#[inline]
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		match self { Self::CheckpointSource(error) => Some(error), Self::Checkpoint(error) => Some(error),
			Self::GgufLlama(error) => Some(error), Self::Data(error) => Some(error),
			Self::InconsistentCheckpoint { .. } | Self::ArithmeticOverflow { .. } => None, } } }

impl From<SourceError> for InferencePreparationError {
	#[inline]
	fn from(error: SourceError) -> Self { Self::CheckpointSource(error) } }

impl From<CheckpointError> for InferencePreparationError {
	#[inline]
	fn from(error: CheckpointError) -> Self { Self::Checkpoint(error) } }

impl From<InferencePrepareError> for InferencePreparationError {
	#[inline]
	fn from(error: InferencePrepareError) -> Self { Self::Data(error) } }

impl From<GgufLlamaError> for InferencePreparationError {
	#[inline]
	fn from(error: GgufLlamaError) -> Self { Self::GgufLlama(error) } }

pub type InferencePreparationResult<T> = Result<T, InferencePreparationError>;

#[derive(Clone, Debug)]
pub enum SemanticModelArtifact { Dense(CheckpointArtifact), Knn(KnnModelArtifact), Bayes(BayesModelArtifact), }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceCompileErrorKind { EmptyDataset, InconsistentCheckpoint, UnsupportedTopology, UnsupportedExtent,
	ArithmeticOverflow, IdentityExhausted, Language, Operation, Program, Ogdl, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceCompileError { kind: InferenceCompileErrorKind, detail: String, }

impl InferenceCompileError {
	#[must_use]
	#[inline]
	pub const fn kind(&self) -> InferenceCompileErrorKind { self.kind }

	#[must_use]
	#[inline]
	pub fn detail(&self) -> &str { &self.detail }

	pub(crate) fn new(kind: InferenceCompileErrorKind, detail: impl Into<String>) -> Self { Self { kind,
			detail: detail.into(), } } }

impl fmt::Display for InferenceCompileError {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	} }

impl core::error::Error for InferenceCompileError {}

impl From<recipe_language::LanguageError> for InferenceCompileError {
	#[inline]
	fn from(error: recipe_language::LanguageError) -> Self { Self::new(InferenceCompileErrorKind::Language, error.to_string()) }
}

impl From<recipe_language::OgdlCodecError> for InferenceCompileError {
	#[inline]
	fn from(error: recipe_language::OgdlCodecError) -> Self { Self::new(InferenceCompileErrorKind::Ogdl, error.to_string()) }
}

impl From<recipe_ops::OperationError> for InferenceCompileError {
	#[inline]
	fn from(error: recipe_ops::OperationError) -> Self { Self::new(InferenceCompileErrorKind::Operation, error.to_string()) }
}

impl From<recipe_program::ProgramError> for InferenceCompileError {
	#[inline]
	fn from(error: recipe_program::ProgramError) -> Self { Self::new(InferenceCompileErrorKind::Program, error.to_string()) }
}

pub type InferenceCompileResult<T> = Result<T, InferenceCompileError>;

pub(crate) struct BlockInferenceContext<'a> {
	pub input: ValueId, pub rows: u64, pub width: u64, pub logical_length: u64, pub logical_channels: u64,
	pub block_index: usize,
	pub layer_index: &'a mut usize,
	pub normalization_epsilon: f32, pub tree_lanes: u32, }

pub(crate) struct BlockInference { pub output: ValueId, pub width: u64, pub logical_length: u64,
	pub logical_channels: u64, }

pub(crate) trait InferenceBlock { fn token_vocabulary(&self) -> Option<u64> { None }

	fn output_width(&self) -> NonZeroU64;

	fn output_operations(&self) -> &[DenseOperation] { &[] }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference>; }

pub(crate) fn inference_block(block: &crate::CheckpointBlockImage) -> &dyn InferenceBlock { match block {
		crate::CheckpointBlockImage::Embedding(block) => block, crate::CheckpointBlockImage::Attention(block) => block,
		crate::CheckpointBlockImage::Rnn(block) => block, crate::CheckpointBlockImage::Gru(block) => block,
		crate::CheckpointBlockImage::Lstm(block) => block, crate::CheckpointBlockImage::Layer(block) => block,
		crate::CheckpointBlockImage::Convolution(block) => block, crate::CheckpointBlockImage::Pool(block) => block,
		crate::CheckpointBlockImage::KMeans(block) => block, crate::CheckpointBlockImage::Tree(block) => block,
		crate::CheckpointBlockImage::Residual(block) => block, } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceInputRole { Feature { feature: usize, source_vector: usize, }, FeatureNormalizationMask,
	DataNormalizationMean, DataNormalizationVariance, DataNormalizationMinimum, DataNormalizationMaximum, LayerWeight {
		layer: usize, }, LayerBias { layer: usize, }, LayerPRelu { layer: usize, occurrence: usize, }, EmbeddingTable {
		block: usize, }, AttentionQuery { block: usize, }, AttentionKey { block: usize, }, AttentionValue { block: usize, },
	AttentionOutput { block: usize, }, RnnInputWeight { block: usize, }, RnnRecurrentWeight { block: usize, }, RnnBias {
		block: usize, }, GruResetInputWeight { block: usize, }, GruResetRecurrentWeight { block: usize, }, GruResetBias {
		block: usize, }, GruUpdateInputWeight { block: usize, }, GruUpdateRecurrentWeight { block: usize, }, GruUpdateBias {
		block: usize, }, GruCandidateInputWeight { block: usize, }, GruCandidateRecurrentWeight { block: usize, },
	GruCandidateBias { block: usize, }, LstmInputGateInputWeight { block: usize, }, LstmInputGateRecurrentWeight {
		block: usize, }, LstmInputGateBias { block: usize, }, LstmForgetGateInputWeight { block: usize, },
	LstmForgetGateRecurrentWeight { block: usize, }, LstmForgetGateBias { block: usize, }, LstmOutputGateInputWeight {
		block: usize, }, LstmOutputGateRecurrentWeight { block: usize, }, LstmOutputGateBias { block: usize, },
	LstmCandidateInputWeight { block: usize, }, LstmCandidateRecurrentWeight { block: usize, }, LstmCandidateBias {
		block: usize, }, ConvolutionWindowIndices { block: usize, }, ConvolutionWeight { block: usize, }, ConvolutionBias {
		block: usize, }, ConvolutionPRelu { block: usize, occurrence: usize, }, PoolWindowIndices { block: usize, },
	PoolWinnerBases { block: usize, }, KMeansCentroids { block: usize, }, TreeSplitFeatures { block: usize, },
	TreeSplitThresholds { block: usize, }, TreeLeafValues { block: usize, }, ResidualProjectionWeight { block: usize, },
	ResidualBranchPRelu { block: usize, occurrence: usize, }, ResidualOutputPRelu { block: usize, occurrence: usize, },
	Temperature, KnnReferenceFeatures, KnnReferenceValues { output: usize, }, KnnReferenceKnown { output: usize, },
	BayesQueryParents { conditional: usize, }, BayesReferenceParents { conditional: usize, }, BayesReferenceChild {
		conditional: usize, }, BayesParentMultipliers { conditional: usize, }, BayesParentCardinalities { conditional: usize,
	}, BayesConcatenationLeftIndices { join: usize, }, BayesConcatenationRightIndices { join: usize, },
	BayesConcatenationSelectLeft { join: usize, }, GgufTokenIds, GgufTensor { tensor: usize, }, GgufRopePartnerIndices,
	GgufRopeCosines, GgufRopeSignedSines, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceExternalInput { role: InferenceInputRole, value: ValueId, dtype: DType, shape: Shape,
	bytes: Vec<u8>, }

impl InferenceExternalInput {
	#[must_use]
	#[inline]
	pub const fn role(&self) -> InferenceInputRole { self.role }

	#[must_use]
	#[inline]
	pub const fn value(&self) -> ValueId { self.value }

	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> DType { self.dtype }

	#[must_use]
	#[inline]
	pub const fn shape(&self) -> &Shape { &self.shape }

	#[must_use]
	#[inline]
	pub fn bytes(&self) -> &[u8] { &self.bytes } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferencePredictionKind { BinaryProbability, MulticlassProbabilities, Regression,
	MultiTargetBinaryProbabilities, JointTargetProbabilities, MultiTargetRegression, BayesProbabilities, TokenLogits, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceTask { Dense(DenseTask), BayesProbabilities { width: u64 }, TokenLogits { vocabulary: u64 }, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceOutputContract { value: ValueId, dtype: DType, target_dtypes: Vec<DType>, shape: Shape,
	kind: InferencePredictionKind, }

impl InferenceOutputContract {
	#[must_use]
	#[inline]
	pub const fn value(&self) -> ValueId { self.value }

	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> DType { self.dtype }

	#[must_use]
	#[inline]
	pub fn target_dtype(&self) -> DType { self.target_dtypes[0] }

	#[must_use]
	#[inline]
	pub fn target_dtypes(&self) -> &[DType] { &self.target_dtypes }

	#[must_use]
	#[inline]
	pub const fn shape(&self) -> &Shape { &self.shape }

	#[must_use]
	#[inline]
	pub const fn kind(&self) -> InferencePredictionKind { self.kind } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledInference { program: StaticCalculationProgram, external_inputs: Vec<InferenceExternalInput>,
	output: InferenceOutputContract, rows: u64, task: InferenceTask, output_adapter: Option<DenseOutputAdapter>, }

impl CompiledInference {
	#[must_use]
	#[inline]
	pub const fn graph(&self) -> &CalculationGraph { self.program.graph() }

	#[must_use]
	#[inline]
	pub const fn program(&self) -> &StaticCalculationProgram { &self.program }

	#[must_use]
	#[inline]
	pub fn external_inputs(&self) -> &[InferenceExternalInput] { &self.external_inputs }

	#[must_use]
	#[inline]
	pub const fn output(&self) -> &InferenceOutputContract { &self.output }

	#[must_use]
	#[inline]
	pub const fn rows(&self) -> u64 { self.rows }

	#[must_use]
	#[inline]
	pub const fn task(&self) -> InferenceTask { self.task }

	#[must_use]
	#[inline]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> { self.output_adapter } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnnInferencePredictionKind { NumericMean, DiscreteMode, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnInferenceOutputContract { value: ValueId, dtype: DType, shape: Shape, source_vector: usize,
	kind: KnnInferencePredictionKind, }

impl KnnInferenceOutputContract { pub(crate) fn new( value: ValueId, dtype: DType, shape: Shape, source_vector: usize,
		kind: KnnInferencePredictionKind, ) -> Self { Self { value, dtype, shape, source_vector, kind, } }

	#[must_use]
	#[inline]
	pub const fn value(&self) -> ValueId { self.value }

	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> DType { self.dtype }

	#[must_use]
	#[inline]
	pub const fn shape(&self) -> &Shape { &self.shape }

	#[must_use]
	#[inline]
	pub const fn source_vector(&self) -> usize { self.source_vector }

	#[must_use]
	#[inline]
	pub const fn kind(&self) -> KnnInferencePredictionKind { self.kind } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledKnnInference { program: StaticCalculationProgram, external_inputs: Vec<InferenceExternalInput>,
	outputs: Vec<KnnInferenceOutputContract>, rows: u64, }

impl CompiledKnnInference {
	#[must_use]
	#[inline]
	pub const fn graph(&self) -> &CalculationGraph { self.program.graph() }

	#[must_use]
	#[inline]
	pub const fn program(&self) -> &StaticCalculationProgram { &self.program }

	#[must_use]
	#[inline]
	pub fn external_inputs(&self) -> &[InferenceExternalInput] { &self.external_inputs }

	#[must_use]
	#[inline]
	pub fn outputs(&self) -> &[KnnInferenceOutputContract] { &self.outputs }

	#[must_use]
	#[inline]
	pub const fn rows(&self) -> u64 { self.rows } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedKnnInference { artifact: KnnModelArtifact, data: PreparedInferenceDataset, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedBayesInference { artifact: BayesModelArtifact, data: PreparedInferenceDataset, }

impl PreparedBayesInference {
	#[must_use]
	#[inline]
	pub const fn artifact(&self) -> &BayesModelArtifact { &self.artifact }

	#[must_use]
	#[inline]
	pub const fn data(&self) -> &PreparedInferenceDataset { &self.data }

	#[must_use]
	#[inline]
	pub fn into_artifact(self) -> BayesModelArtifact { self.artifact } }

impl PreparedKnnInference {
	#[must_use]
	#[inline]
	pub const fn artifact(&self) -> &KnnModelArtifact { &self.artifact }

	#[must_use]
	#[inline]
	pub const fn data(&self) -> &PreparedInferenceDataset { &self.data }

	#[must_use]
	#[inline]
	pub fn into_artifact(self) -> KnnModelArtifact { self.artifact } }

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedInference { checkpoint: CheckpointArtifact, data: PreparedInferenceDataset, }

impl PreparedInference {
	#[must_use]
	#[inline]
	pub const fn checkpoint(&self) -> &CheckpointArtifact { &self.checkpoint }

	#[must_use]
	#[inline]
	pub const fn data(&self) -> &PreparedInferenceDataset { &self.data }

	#[must_use]
	#[inline]
	pub fn features(&self) -> &[PreparedInferenceFeature] { self.data.features() }

	#[must_use]
	#[inline]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] { self.checkpoint.feature_spans() }

	#[must_use]
	#[inline]
	pub const fn normalization(&self) -> DenseDataNormalization { self.checkpoint.config().data_normalization }


	#[must_use]
	#[inline]
	pub fn feature_normalization_mask(&self) -> &[u32] { self.checkpoint.feature_normalization_mask() }

	#[must_use]
	#[inline]
	pub fn normalization_epsilon(&self) -> f32 { self.checkpoint.config().normalization_epsilon }

}

#[inline]
pub fn load_checkpoint_file( path: impl AsRef<Path>, limits: CheckpointDecodeLimits,
) -> InferencePreparationResult<CheckpointArtifact> { let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("checkpoint source-byte bound cannot be represented as u64: {error}"),
		})?; let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_checkpoint(source.bytes(), limits).map_err(Into::into) }

#[inline]
pub fn load_knn_model_file( path: impl AsRef<Path>, limits: KnnModelDecodeLimits,
) -> InferencePreparationResult<KnnModelArtifact> { let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("KNN model source-byte bound cannot be represented as u64: {error}"),
		})?; let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_knn_model(source.bytes(), limits).map_err(Into::into) }

#[inline]
pub fn load_bayes_model_file( path: impl AsRef<Path>, limits: BayesModelDecodeLimits,
) -> InferencePreparationResult<BayesModelArtifact> { let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("Bayesian model source-byte bound cannot be represented as u64: {error}"),
		})?; let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_bayes_model(source.bytes(), limits).map_err(Into::into) }

#[inline]
pub fn load_semantic_model_file( path: impl AsRef<Path>, checkpoint_limits: CheckpointDecodeLimits,
	knn_limits: KnnModelDecodeLimits, ) -> InferencePreparationResult<SemanticModelArtifact> {
	let source_bytes = checkpoint_limits.source_bytes.max(knn_limits.source_bytes);
	let source_bytes = source_bytes.max(BayesModelDecodeLimits::default().source_bytes); let source_bytes =
		u64::try_from(source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("semantic-model source-byte bound cannot be represented as u64: {error}"),
		})?; let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?; let root = source .bytes()
		.split(|byte| matches!(*byte, b'\n' | b'\t'))
		.next()
		.expect("a byte slice always has a first split segment");
	let root = core::str::from_utf8(root).map_err(|error| { decode_error( CheckpointDecodeErrorKind::InvalidUtf8,
			CheckpointPath::root(),
			format!("semantic-model root is not UTF-8: {error}"),
		) })?; match root {
		"recipe" => decode_checkpoint(source.bytes(), checkpoint_limits)
			.map(SemanticModelArtifact::Dense) .map_err(Into::into),
		"recipe-knn-model" => decode_knn_model(source.bytes(), knn_limits)
			.map(SemanticModelArtifact::Knn) .map_err(Into::into),
		"recipe-bayes-model" => decode_bayes_model(source.bytes(), BayesModelDecodeLimits::default())
			.map(SemanticModelArtifact::Bayes) .map_err(Into::into), other => Err(decode_error(
			CheckpointDecodeErrorKind::InvalidValue, CheckpointPath::root(),
			format!("unknown semantic-model root {other:?}"),
		) .into()), } }

#[inline]
pub fn prepare_knn_inference_table( artifact: KnnModelArtifact, table: &RawTable,
) -> InferencePreparationResult<PreparedKnnInference> { let schema = saved_feature_schema_from_parts(
		artifact.references().vectors(), artifact.references().feature_spans(), )?;
	let data = recipe_ingest::prepare_inference_table(table, &schema)?;
	validate_prepared_feature_spans(&data, artifact.references().feature_spans())?;
	Ok(PreparedKnnInference { artifact, data }) }

#[inline]
pub fn prepare_bayes_inference_table( artifact: BayesModelArtifact, table: &RawTable,
) -> InferencePreparationResult<PreparedBayesInference> { let mut schema = Vec::new(); let mut seen = BTreeSet::new();
	for conditional in artifact.conditionals() { for parent in conditional.parents() {
			if seen.insert(parent.source_index()) { schema.push(InferenceFeatureSchema::new( parent.source_index(),
					parent.name(), InferenceFeatureEncoding::CategoricalDictionary { dictionary: parent.dictionary().to_vec(), }, )); }
		} }
	let data = recipe_ingest::prepare_inference_table(table, &schema)?; Ok(PreparedBayesInference { artifact, data }) }

pub fn prepare_checkpoint_inference_table( checkpoint: CheckpointArtifact, table: &RawTable,
) -> InferencePreparationResult<PreparedInference> { let schema = saved_feature_schema(&checkpoint)?;
	let data = recipe_ingest::prepare_inference_table(table, &schema)?;
	validate_prepared_feature_spans(&data, checkpoint.feature_spans())?; Ok(PreparedInference { checkpoint, data }) }

pub fn compile_prepared_inference(prepared: &PreparedInference) -> InferenceCompileResult<CompiledInference> {
	let checkpoint = prepared.checkpoint(); if checkpoint.blocks().is_empty() { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"checkpoint retains no effective model blocks",
		)); }
	let rows = u64::try_from(prepared.data().rows()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("inference row count cannot be represented by u64: {error}"),
		) })?; if rows == 0 { return Err(InferenceCompileError::new( InferenceCompileErrorKind::EmptyDataset,
			"target-free inference requires at least one row",
		)); }
	let feature_width = u64::try_from(checkpoint.feature_width()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("saved dense feature width cannot be represented by u64: {error}"),
		) })?; if feature_width == 0 { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"saved dense feature width is zero",
		)); }

	let mut compiler = InferenceGraphCompiler::new(); let leading_vocabulary = checkpoint .blocks() .first()
		.and_then(|block| inference_block(block).token_vocabulary());
	let lowered = if let Some(vocabulary) = leading_vocabulary { compiler.compile_token_features( prepared.data(),
			checkpoint.feature_spans(), rows, feature_width, vocabulary, )? } else { compiler.compile_features( prepared.data(),
			checkpoint.feature_spans(), rows, feature_width, )? };
	let mut current = compiler.apply_data_normalization(checkpoint, lowered, rows, feature_width)?;
	let mut current_width = feature_width; let mut logical_length = feature_width; let mut logical_channels = 1u64;
	let mut layer_index = 0usize; for (block_index, block) in checkpoint.blocks().iter().enumerate() {
		let inference = inference_block(block).compile_inference( &mut compiler, BlockInferenceContext { input: current, rows,
				width: current_width, logical_length, logical_channels, block_index, layer_index: &mut layer_index,
				normalization_epsilon: checkpoint.config().normalization_epsilon,
				tree_lanes: checkpoint.config().reduction_tree_lanes, }, )?; current = inference.output;
		current_width = inference.width; logical_length = inference.logical_length;
		logical_channels = inference.logical_channels; }
	let expected_width = u64::try_from(checkpoint.task().output_width()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("saved task output width cannot be represented by u64: {error}"),
		) })?; if current_width != expected_width { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			format!("effective model blocks produce width {current_width}, saved task requires {expected_width}"),
		)); }
	if let Some(temperature) = checkpoint.temperature() { current = compiler.apply_temperature(current, temperature)?; }
	let (prediction, kind) = compiler.compile_prediction( current, checkpoint.task(), rows, current_width,
		checkpoint.config().reduction_tree_lanes, )?; let target_dtypes = checkpoint.target_dtypes().collect::<Vec<_>>();
	if target_dtypes.len() != checkpoint.task().target_count() { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"saved target vectors do not all have fixed calculation dtypes",
		)); }
	compiler.finish( prediction, kind, rows, InferenceTask::Dense(checkpoint.task()), target_dtypes,
		checkpoint.output_adapter(), ) }

#[inline]
pub fn compile_prepared_bayes_inference( prepared: &PreparedBayesInference,
) -> InferenceCompileResult<CompiledInference> { let rows = u64::try_from(prepared.data().rows()).map_err(|error| {
		InferenceCompileError::new( InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian inference row count cannot be represented by u64: {error}"),
		) })?; if rows == 0 { return Err(InferenceCompileError::new( InferenceCompileErrorKind::EmptyDataset,
			"categorical Bayesian inference requires at least one query row",
		)); }
	if prepared.artifact().conditionals().is_empty() { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"Bayesian semantic model retains no conditionals",
		)); }

	let mut compiler = InferenceGraphCompiler::new();
	let mut probabilities = Vec::with_capacity(prepared.artifact().conditionals().len()); let mut total_width = 0u64;
	for (conditional, references) in prepared.artifact().conditionals().iter().enumerate() {
		let width = u64::try_from(references.child_classes()).map_err(|error| { InferenceCompileError::new(
				InferenceCompileErrorKind::UnsupportedExtent,
				format!("Bayesian conditional {conditional} child class count does not fit u64: {error}"),
			) })?; let output = compile_bayes_conditional( &mut compiler, prepared.data(), references,
			prepared.artifact().smoothing(), conditional, rows, )?; total_width = total_width.checked_add(width).ok_or_else(|| {
			InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
				"Bayesian aggregate output width overflowed u64",
			) })?; probabilities.push((output, width)); }
	let prediction = concatenate_bayes_probabilities(&mut compiler, probabilities, rows)?; compiler.finish( prediction,
		InferencePredictionKind::BayesProbabilities, rows, InferenceTask::BayesProbabilities { width: total_width },
		vec![DType::I32; prepared.artifact().conditionals().len()], None, ) }

fn compile_bayes_conditional( compiler: &mut InferenceGraphCompiler, data: &PreparedInferenceDataset,
	references: &crate::BayesianCategoricalReferenceSet, smoothing: f32, conditional: usize, rows: u64,
) -> InferenceCompileResult<ValueId> { let reference_rows = u64::try_from(references.reference_rows()).map_err(|error| {
		InferenceCompileError::new( InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian conditional {conditional} reference row count does not fit u64: {error}"),
		) })?; let parent_count = u64::try_from(references.parents().len()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian parent count cannot be represented by u64: {error}"),
		) })?; let child_classes = u64::try_from(references.child_classes()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("Bayesian child class count cannot be represented by u64: {error}"),
		) })?; let requirements = categorical_bayes_inference_requirements( reference_rows, rows, parent_count,
		references.parent_configurations(), child_classes, )?; let query_elements = data .rows()
		.checked_mul(references.parents().len()) .ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"Bayesian query parent matrix size overflowed usize",
			) })?; let mut query_parent_codes = Vec::with_capacity(query_elements); for row in 0..data.rows() {
		for (parent_index, parent) in references.parents().iter().enumerate() { let feature = data .features() .iter()
				.find(|feature| { feature.schema().source_vector() == parent.source_index()
						&& feature.schema().name() == parent.name() }) .ok_or_else(|| { InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint, format!(
							"prepared Bayesian conditional {conditional} parent {parent_index} is absent"
						), ) })?; let PreparedInferenceValues::I32(values) = feature.values() else {
				return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
						"prepared Bayesian conditional {conditional} parent {parent_index} is not dictionary-coded I32"
					), )); }; let code = values.get(row).copied().ok_or_else(|| { InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint, format!(
						"prepared Bayesian conditional {conditional} parent {parent_index} is missing query row {row}"
					), ) })?; let cardinality = references.parent_cardinalities()[parent_index]; if code < 0 || code >= cardinality {
				return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
						"prepared Bayesian conditional {conditional} parent {parent_index} row {row} has out-of-range code {code}"
					), )); }
			query_parent_codes.push(code); } }

	let reference_parent_codes = compiler.external( InferenceInputRole::BayesReferenceParents { conditional }, DType::I32,
		shape(&[reference_rows, parent_count])?, references .parent_codes() .iter() .flat_map(|value| value.to_le_bytes())
			.collect(), )?; let reference_child_codes = compiler.external(
		InferenceInputRole::BayesReferenceChild { conditional }, DType::I32, shape(&[reference_rows])?, references
			.child_codes() .iter() .flat_map(|value| value.to_le_bytes()) .collect(), )?;
	let query_parent_codes = compiler.external( InferenceInputRole::BayesQueryParents { conditional }, DType::I32,
		shape(&[rows, parent_count])?, query_parent_codes .iter() .flat_map(|value| value.to_le_bytes()) .collect(), )?;
	let parent_multipliers = compiler.external( InferenceInputRole::BayesParentMultipliers { conditional }, DType::I32,
		shape(&[parent_count])?, references .parent_multipliers() .iter() .flat_map(|value| value.to_le_bytes()) .collect(),
	)?; let parent_cardinalities = compiler.external( InferenceInputRole::BayesParentCardinalities { conditional },
		DType::I32, shape(&[parent_count])?, references .parent_cardinalities() .iter() .flat_map(|value| value.to_le_bytes())
			.collect(), )?; let probabilities = compiler.tensor(DType::F32, shape(&[rows, child_classes])?)?;
	let first_value = compiler.next_value; let first_kernel = compiler.next_kernel; compiler.next_value = compiler
		.next_value .checked_add(requirements.intermediate_values) .ok_or_else(identity_exhausted)?;
	compiler.next_kernel = compiler .next_kernel .checked_add(requirements.kernels) .ok_or_else(identity_exhausted)?;
	let request = CategoricalBayesInferenceRequest {
		reference_parent_codes: bayes_request_input(&compiler, reference_parent_codes)?,
		reference_child_codes: bayes_request_input(&compiler, reference_child_codes)?,
		query_parent_codes: bayes_request_input(&compiler, query_parent_codes)?,
		parent_multipliers: bayes_request_input(&compiler, parent_multipliers)?,
		parent_cardinalities: bayes_request_input(&compiler, parent_cardinalities)?,
		probabilities: bayes_request_output(&compiler, probabilities)?, reference_rows, query_rows: rows, parent_count,
		parent_configurations: references.parent_configurations(), child_classes, smoothing,
		tree_lanes: MAXIMUM_REDUCTION_TREE_LANES, identity_namespace: IdentityNamespace::new( ValueId::new(first_value),
			requirements.intermediate_values, KernelTemplateId::new(first_kernel), requirements.kernels, ),
		workspace_limit: requirements.workspace_bytes, }; for tensor in compiler.tensors.values_mut() {
		tensor.external_input = compiler.external_input_ids.contains(&tensor.id);
		tensor.external_output = tensor.id == probabilities; }
	let mut graph = CalculationGraph { tensors: core::mem::take(&mut compiler.tensors) .into_values() .collect(),
		nodes: core::mem::take(&mut compiler.nodes), };
	let materialized = append_categorical_bayes_inference(&mut graph, &request)?; compiler.domains.extend( materialized
			.kernels .iter() .map(|kernel| KernelIterationDomain { kernel: *kernel, domain: IterationDomain::first(), }), );
	compiler.tensors = graph .tensors .into_iter() .map(|tensor| (tensor.id, tensor)) .collect();
	compiler.nodes = graph.nodes; Ok(probabilities) }

fn concatenate_bayes_probabilities( compiler: &mut InferenceGraphCompiler, probabilities: Vec<(ValueId, u64)>,
	rows: u64, ) -> InferenceCompileResult<ValueId> { let mut probabilities = probabilities.into_iter();
	let (mut combined, mut combined_width) = probabilities.next().ok_or_else(|| { InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"Bayesian inference has no probability outputs",
		) })?; for (join, (right, right_width)) in probabilities.enumerate() { let join = join + 1;
		let total_width = combined_width.checked_add(right_width).ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"Bayesian concatenated probability width overflowed u64",
			) })?; let left_elements = checked_product( &[rows, combined_width],
			"Bayesian left probability elements",
		)?;
		let right_elements = checked_product(&[rows, right_width], "Bayesian right probability elements")?;
		let total_elements = checked_product( &[rows, total_width],
			"Bayesian concatenated probability elements",
		)?;
		checked_i32(left_elements, "Bayesian left probability elements")?;
		checked_i32(right_elements, "Bayesian right probability elements")?;
		checked_i32(total_elements, "Bayesian concatenated probability elements")?;
		let capacity = usize::try_from(total_elements).map_err(|error| { InferenceCompileError::new(
				InferenceCompileErrorKind::UnsupportedExtent,
				format!("Bayesian concatenation table length does not fit usize: {error}"),
			) })?; let mut left_indices = Vec::with_capacity(capacity); let mut right_indices = Vec::with_capacity(capacity);
		let mut select_left = Vec::<i32>::with_capacity(capacity); for row in 0..rows { for column in 0..total_width {
				if column < combined_width { left_indices.push(checked_i32( row * combined_width + column,
						"Bayesian left concatenation index",
					)?); right_indices.push(0); select_left.push(1); } else { left_indices.push(0); right_indices.push(checked_i32(
						row * right_width + column - combined_width,
						"Bayesian right concatenation index",
					)?); select_left.push(0); } } }
		let table_shape = shape(&[total_elements])?; let left_indices = compiler.external(
			InferenceInputRole::BayesConcatenationLeftIndices { join }, DType::I32, table_shape.clone(), left_indices .iter()
				.flat_map(|value| value.to_le_bytes()) .collect(), )?; let right_indices = compiler.external(
			InferenceInputRole::BayesConcatenationRightIndices { join }, DType::I32, table_shape.clone(), right_indices .iter()
				.flat_map(|value| value.to_le_bytes()) .collect(), )?; let select_left = compiler.external(
			InferenceInputRole::BayesConcatenationSelectLeft { join }, DType::I32, table_shape.clone(), select_left .iter()
				.flat_map(|value| value.to_le_bytes()) .collect(), )?;
		let left = compiler.reinterpret_f32(combined, shape(&[left_elements])?)?;
		let right = compiler.reinterpret_f32(right, shape(&[right_elements])?)?;
		let concatenated = compiler.tensor(DType::F32, table_shape)?; let parameters = PreparedParameters::from([
			("rows".to_owned(), PreparedParameter::U64(rows)),
			(
				"left_columns".to_owned(),
				PreparedParameter::U64(combined_width), ), (
				"right_columns".to_owned(),
				PreparedParameter::U64(right_width), ), (
				"concatenation_tables_verified".to_owned(),
				PreparedParameter::Bool(true), ), ]); compiler.materialize(
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
			&parameters, )?; combined = compiler.reinterpret_f32(concatenated, shape(&[rows, total_width])?)?;
		combined_width = total_width; }
	Ok(combined) }

fn bayes_request_input(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone(); tensor.external_input = true; tensor.external_output = false;
	Ok(tensor) }

fn bayes_request_output(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone(); tensor.external_input = false; tensor.external_output = true;
	Ok(tensor) }

#[inline]
pub fn compile_prepared_knn_inference(prepared: &PreparedKnnInference) -> InferenceCompileResult<CompiledKnnInference> {
	let artifact = prepared.artifact(); if !artifact.operations().is_empty() { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedTopology,
			"post-KNN operations are undefined for independently typed numeric and discrete outputs",
		)); }
	let references = artifact.references(); let rows = u64::try_from(prepared.data().rows()).map_err(|error| {
		InferenceCompileError::new( InferenceCompileErrorKind::UnsupportedExtent,
			format!("KNN inference row count cannot be represented by u64: {error}"),
		) })?; if rows == 0 { return Err(InferenceCompileError::new( InferenceCompileErrorKind::EmptyDataset,
			"KNN inference requires at least one query row",
		)); }
	let reference_rows = u64::try_from(references.reference_rows()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("KNN reference row count cannot be represented by u64: {error}"),
		) })?; let feature_width = u64::try_from(references.feature_width()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("KNN feature width cannot be represented by u64: {error}"),
		) })?; if reference_rows == 0 || feature_width == 0 { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			"KNN model has an empty reference matrix",
		)); }

	let mut compiler = InferenceGraphCompiler::new(); let query_features = compiler.compile_features( prepared.data(),
		references.feature_spans(), rows, feature_width, )?; let reference_features = compiler.external(
		InferenceInputRole::KnnReferenceFeatures, DType::F32, shape(&[reference_rows, feature_width])?, references
			.reference_feature_bits() .iter() .flat_map(|bits| bits.to_le_bytes()) .collect(), )?;
	let (query_features, reference_features) = compiler.normalize_knn_features( [query_features, reference_features],
		[rows, reference_rows, feature_width], artifact.data_normalization(), references.normalization_mask(),
		MAXIMUM_REDUCTION_TREE_LANES, )?;

	let mut requests = Vec::with_capacity(references.outputs().len());
	let mut outputs = Vec::with_capacity(references.outputs().len());
	for (output_index, output) in references.outputs().iter().enumerate() { let known = compiler.external(
			InferenceInputRole::KnnReferenceKnown { output: output_index, }, DType::I32, shape(&[reference_rows])?,
			output.known() .iter() .flat_map(|value| value.to_le_bytes()) .collect(), )?;
		let prediction_shape = shape(&[rows, 1])?; let (request, contract) = if output.values().is_numeric() {
				let bits = output.values().numeric_f32_bits_storage(); let values = compiler.external(
					InferenceInputRole::KnnReferenceValues { output: output_index, }, DType::F32, shape(&[reference_rows])?,
					bits.iter().flat_map(|bits| bits.to_le_bytes()).collect(), )?;
				let prediction = compiler.tensor(DType::F32, prediction_shape.clone())?; ( KnnOutputRequest::Numeric {
						reference_values: knn_request_input(&compiler, values)?, known: knn_request_input(&compiler, known)?,
						predictions: knn_request_output(&compiler, prediction)?, known_references: output.known_references(), },
					KnnInferenceOutputContract::new( prediction, DType::F32, prediction_shape, output.schema().source_index(),
						KnnInferencePredictionKind::NumericMean, ), ) } else {
				let (codes, labels) = output.values().discrete_i32_storage(); let values = compiler.external(
					InferenceInputRole::KnnReferenceValues { output: output_index, }, DType::I32, shape(&[reference_rows])?,
					codes.iter().flat_map(|value| value.to_le_bytes()).collect(), )?;
				let prediction = compiler.tensor(DType::I32, prediction_shape.clone())?; ( KnnOutputRequest::Categorical {
						reference_codes: knn_request_input(&compiler, values)?, known: knn_request_input(&compiler, known)?,
						predictions: knn_request_output(&compiler, prediction)?, known_references: output.known_references(),
						classes: u64::try_from(labels.len()).map_err(|error| { InferenceCompileError::new(
								InferenceCompileErrorKind::UnsupportedExtent, format!(
									"KNN output {output_index} label count does not fit u64: {error}"
								), ) })?, }, KnnInferenceOutputContract::new( prediction, DType::I32, prediction_shape,
						output.schema().source_index(), KnnInferencePredictionKind::DiscreteMode, ), ) }; requests.push(request);
		outputs.push(contract); }

	let specs = references.operation_specs(); let requirements = knn_all_output_requirements( rows, reference_rows,
		feature_width, references.neighbors().get(), &specs, )?; let request = KnnAllOutputRequest {
		query_features: knn_request_input(&compiler, query_features)?,
		reference_features: knn_request_input(&compiler, reference_features)?, outputs: requests,
		neighbors: references.neighbors().get(), tree_lanes: MAXIMUM_REDUCTION_TREE_LANES,
		identity_namespace: IdentityNamespace::new( ValueId::new(compiler.next_value), requirements.intermediate_values,
			KernelTemplateId::new(compiler.next_kernel), requirements.kernels, ), workspace_limit: requirements.workspace_bytes,
	}; let output_ids = outputs .iter() .map(KnnInferenceOutputContract::value) .collect::<BTreeSet<_>>();
	for tensor in compiler.tensors.values_mut() { tensor.external_input = compiler.external_input_ids.contains(&tensor.id);
		tensor.external_output = output_ids.contains(&tensor.id); }
	let mut graph = CalculationGraph { tensors: compiler.tensors.into_values().collect(), nodes: compiler.nodes, };
	let materialized = append_knn_all_outputs(&mut graph, &request)?; compiler.domains.extend( materialized .kernels
			.iter() .map(|kernel| KernelIterationDomain { kernel: *kernel, domain: IterationDomain::first(), }), );
	graph.validate()?; let graph = CalculationGraph::from_ogdl(&graph.to_ogdl()?)?;
	let iterations = NonZeroU64::new(1).expect("one KNN inference iteration is nonzero");
	let program = StaticCalculationProgram::new(graph, iterations, compiler.domains)?;
	let program = StaticCalculationProgram::from_ogdl(&program.to_ogdl()?)?; Ok(CompiledKnnInference { program,
		external_inputs: compiler.external_inputs, outputs, rows, }) }

fn knn_request_input(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone(); tensor.external_input = true; tensor.external_output = false;
	Ok(tensor) }

fn knn_request_output(compiler: &InferenceGraphCompiler, value: ValueId) -> InferenceCompileResult<Tensor> {
	let mut tensor = compiler.tensor_ref(value)?.clone(); tensor.external_input = false; tensor.external_output = true;
	Ok(tensor) }

#[derive(Debug)]
pub(crate) struct InferenceGraphCompiler { tensors: BTreeMap<ValueId, Tensor>, nodes: Vec<CalculationNode>,
	domains: Vec<KernelIterationDomain>, next_value: u64, next_kernel: u64, external_inputs: Vec<InferenceExternalInput>,
	external_input_ids: BTreeSet<ValueId>, }

impl RecurrentForwardGraph for InferenceGraphCompiler { type Error = InferenceCompileError;

	fn zero_f32(&mut self, shape: Shape) -> InferenceCompileResult<ValueId> { InferenceGraphCompiler::zero_f32(self, shape) }

	fn gather_matrix_column( &mut self, matrix: ValueId, rows: u64, columns: u64, column: u64,
	) -> InferenceCompileResult<ValueId> {
		InferenceGraphCompiler::gather_matrix_column(self, matrix, rows, columns, column) }

	fn bias_free_linear( &mut self, input: ValueId, weight: ValueId, rows: u64, output_width: u64,
	) -> InferenceCompileResult<ValueId> {
		InferenceGraphCompiler::bias_free_linear(self, input, weight, rows, output_width) }

	fn elementwise_f32( &mut self, shape: Shape, inputs: Vec<ValueId>, program: recipe_core::ScalarProgram,
	) -> InferenceCompileResult<ValueId> { let output = self.tensor(DType::F32, shape)?;
		self.emit_elementwise(inputs, vec![output], program)?; Ok(output) }

	fn activate( &mut self, input: ValueId, activation: DenseActivation, shape: Shape,
	) -> InferenceCompileResult<ValueId> { self.apply_activation(input, activation, None, shape) } }

impl InferenceGraphCompiler { fn new() -> Self { Self { tensors: BTreeMap::new(), nodes: Vec::new(),
			domains: Vec::new(), next_value: 1, next_kernel: 1, external_inputs: Vec::new(), external_input_ids: BTreeSet::new(),
		} }

	fn tensor(&mut self, dtype: DType, shape: Shape) -> InferenceCompileResult<ValueId> { let value = self.next_value()?;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false)?; self.tensors.insert(value, tensor); Ok(value) }

	fn external( &mut self, role: InferenceInputRole, dtype: DType, shape: Shape, bytes: Vec<u8>,
	) -> InferenceCompileResult<ValueId> { let expected = shape.bytes(dtype)?.get();
		if u64::try_from(bytes.len()).ok() != Some(expected) { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"{role:?} provides {} bytes, tensor contract requires {expected}",
					bytes.len() ), )); }
		let value = self.tensor(dtype, shape.clone())?; self.external_input_ids.insert(value);
		self.external_inputs.push(InferenceExternalInput { role, value, dtype, shape, bytes, }); Ok(value) }

	fn external_checkpoint_tensor( &mut self, role: InferenceInputRole, image: &CheckpointTensorImage,
	) -> InferenceCompileResult<ValueId> { self.external( role, image.dtype(), shape(image.shape())?,
			image.bytes().to_vec(), ) }

	fn next_value(&mut self) -> InferenceCompileResult<ValueId> { let value = self.next_value;
		self.next_value = value.checked_add(1).ok_or_else(identity_exhausted)?; Ok(ValueId::new(value)) }

	fn next_kernel(&mut self) -> InferenceCompileResult<KernelTemplateId> { let kernel = self.next_kernel;
		self.next_kernel = kernel.checked_add(1).ok_or_else(identity_exhausted)?; Ok(KernelTemplateId::new(kernel)) }

	fn tensor_ref(&self, value: ValueId) -> InferenceCompileResult<&Tensor> { self.tensors.get(&value).ok_or_else(|| {
			InferenceCompileError::new( InferenceCompileErrorKind::Language,
				format!("inference compiler tensor {value} is absent"),
			) }) }

	fn emit( &mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind,
		alias_rules: Vec<PrimitiveAliasRule>, ) -> InferenceCompileResult<KernelTemplateId> { let id = self.next_kernel()?;
		self.nodes.push(CalculationNode { kernel: PrimitiveKernel { id, inputs, outputs, alias_rules, kind, }, });
		self.domains.push(KernelIterationDomain { kernel: id, domain: IterationDomain::first(), }); Ok(id) }

	fn emit_elementwise( &mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, program: recipe_core::ScalarProgram,
	) -> InferenceCompileResult<KernelTemplateId> { let aliases = forbidden_aliases(inputs.len(), outputs.len());
		self.emit( inputs, outputs, PrimitiveKind::Elementwise(Elementwise { program }), aliases, ) }

	fn emit_owned_scalar( &mut self, symbol: &str, inputs: Vec<ValueId>, outputs: Vec<ValueId>,
	) -> InferenceCompileResult<KernelTemplateId> { let descriptor = operation_registry().resolve_unique(symbol)?;
		let program = lower_scalar(descriptor)?; self.emit_elementwise(inputs, outputs, program) }

	fn reduce( &mut self, input: ValueId, output: ValueId, operator: ReduceOperator, axes: &[usize], keep_dimensions: bool,
		tree_lanes: u32, ) -> InferenceCompileResult<()> { self.emit( vec![input], vec![output],
			PrimitiveKind::Reduce(Reduce { operator, axes: AxisSet::new(axes.to_vec())?, keep_dimensions,
				result: ReduceResult::Value, tree_lanes, }), forbidden_aliases(1, 1), )?; Ok(()) }

	fn materialize( &mut self, symbol: &str,
		inputs: &[(&'static str, ValueId)],
		outputs: &[(&'static str, ValueId)],
		iteration_shape_input: &'static str,
		parameters: &PreparedParameters, ) -> InferenceCompileResult<()> { let mut input_tensors = inputs .iter()
			.map(|(_, value)| self.tensor_ref(*value).cloned()) .collect::<InferenceCompileResult<Vec<_>>>()?;
		for tensor in &mut input_tensors { tensor.external_input = true; tensor.external_output = false; }
		let mut output_tensors = outputs .iter() .map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<InferenceCompileResult<Vec<_>>>()?; for tensor in &mut output_tensors { tensor.external_input = false;
			tensor.external_output = true; }
		let named_inputs = inputs .iter() .zip(&input_tensors) .map(|((name, _), tensor)| NamedTensor::new(name, tensor))
			.collect::<Vec<_>>(); let named_outputs = outputs .iter() .zip(&output_tensors)
			.map(|((name, _), tensor)| NamedTensor::new(name, tensor)) .collect::<Vec<_>>(); let first_value = self.next_value;
		let first_kernel = self.next_kernel; self.next_value = self .next_value .checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?; self.next_kernel = self .next_kernel .checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?; let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?, &named_inputs, &named_outputs, iteration_shape_input, parameters,
			IdentityNamespace::new( ValueId::new(first_value), MATERIALIZATION_RESERVATION, KernelTemplateId::new(first_kernel),
				MATERIALIZATION_RESERVATION, ), WORKSPACE_LIMIT, ))?; for tensor in &materialized.graph().tensors {
			self.insert_tensor_contract(tensor.clone())?; }
		for node in &materialized.graph().nodes { self.nodes.push(node.clone()); self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id, domain: IterationDomain::first(), }); }
		Ok(()) }

	fn insert_tensor_contract(&mut self, mut tensor: Tensor) -> InferenceCompileResult<()> { tensor.external_input = false;
		tensor.external_output = false; match self.tensors.get(&tensor.id) { Some(existing) if existing.dtype == tensor.dtype
					&& existing.shape == tensor.shape
					&& existing.layout == tensor.layout
					&& existing.storage_bytes == tensor.storage_bytes =>
			{ Ok(()) }
			Some(_) => Err(InferenceCompileError::new( InferenceCompileErrorKind::Language, format!(
					"materialized tensor {} conflicts with an existing inference contract",
					tensor.id ), )), None => { self.tensors.insert(tensor.id, tensor); Ok(()) } } }

	fn compile_features( &mut self, data: &PreparedInferenceDataset, spans: &[CompiledFeatureSpan], rows: u64,
		feature_width: u64, ) -> InferenceCompileResult<ValueId> { if data.features().len() != spans.len() {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				"prepared feature count differs from saved dense spans",
			)); }
		let total_elements = rows.checked_mul(feature_width).ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"inference dense feature element count overflowed u64",
			) })?;
		require_i32_indexable(total_elements, "inference dense feature element count")?;
		let mut combined = self.zero_f32(shape(&[total_elements])?)?; let mut expected_start = 0u64;
		for (feature_index, (feature, span)) in data.features().iter().zip(spans).enumerate() {
			let start = u64::try_from(span.start()).map_err(|error| { InferenceCompileError::new(
					InferenceCompileErrorKind::UnsupportedExtent,
					format!("feature span start cannot be represented by u64: {error}"),
				) })?; let width = u64::try_from(span.width()).map_err(|error| { InferenceCompileError::new(
					InferenceCompileErrorKind::UnsupportedExtent,
					format!("feature span width cannot be represented by u64: {error}"),
				) })?; if start != expected_start || width == 0 || start.checked_add(width).is_none() {
				return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("saved feature span {feature_index} is not a nonempty contiguous dense span"),
				)); }
			expected_start = start + width; if expected_start > feature_width
				|| feature.schema().source_vector() != span.source_vector()
				|| feature.values().len() != data.rows()
			{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("prepared feature {feature_index} disagrees with its saved dense span"),
				)); }
			let raw_shape = shape(&[rows])?; let (dtype, bytes) = inference_feature_bytes(feature.values());
			let raw = self.external( InferenceInputRole::Feature { feature: feature_index, source_vector: span.source_vector(),
				}, dtype, raw_shape, bytes, )?; let block = match (span.lowering(), feature.values()) {
				(DenseFeatureLowering::NumericScalar, PreparedInferenceValues::I32(_)) if width == 1 => {
					let converted = self.tensor(DType::F32, shape(&[rows])?)?;
					self.emit_elementwise(vec![raw], vec![converted], checked_i32_to_f32_program()?)?; converted }
				(DenseFeatureLowering::NumericScalar, PreparedInferenceValues::F32Bits(_)) if width == 1 => raw, (
					DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, }, PreparedInferenceValues::I32(_),
				) if dictionary_width.checked_add(1) == Some(span.width()) && reserved_index == dictionary_width =>
				{ self.one_hot(raw, rows, width)? }
				_ => { return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
						format!("prepared feature {feature_index} has no matching saved lowering"),
					)); } }; combined = self.scatter_feature_block(combined, block, rows, feature_width, start, width)?; }
		if expected_start != feature_width { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"saved feature spans end at width {expected_start}, checkpoint declares {feature_width}"
				), )); }
		let dense_shape = shape(&[rows, feature_width])?; let identity = self.tensor(DType::I32, dense_shape.clone())?;
		self.emit( Vec::new(), vec![identity], PrimitiveKind::IndexMap(IndexMap { start: 0, element_step: 1,
				iteration_step: 0, modulus: None, }), Vec::new(), )?; let dense = self.tensor(DType::F32, dense_shape)?; self.emit(
			vec![combined, identity], vec![dense], PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }),
			forbidden_aliases(2, 1), )?; Ok(dense) }

	fn compile_token_features( &mut self, data: &PreparedInferenceDataset, spans: &[CompiledFeatureSpan], rows: u64,
		feature_width: u64, vocabulary: u64, ) -> InferenceCompileResult<ValueId> { if data.features().len() != spans.len() {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				"prepared token-position count differs from the saved embedding sequence",
			)); }
		let vocabulary = i32::try_from(vocabulary).map_err(|error| { InferenceCompileError::new(
				InferenceCompileErrorKind::UnsupportedExtent,
				format!("embedding vocabulary cannot be represented by int32: {error}"),
			) })?; let total_elements = rows.checked_mul(feature_width).ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"inference token element count overflowed u64",
			) })?;
		require_i32_indexable(total_elements, "inference token element count")?;
		let mut combined = self.zero_i32(shape(&[total_elements])?)?;
		for (feature_index, (feature, span)) in data.features().iter().zip(spans).enumerate() {
			let start = u64::try_from(span.start()).map_err(|error| { InferenceCompileError::new(
					InferenceCompileErrorKind::UnsupportedExtent,
					format!("token position cannot be represented by u64: {error}"),
				) })?; if start != feature_index as u64
				|| span.width() != 1 || span.lowering() != DenseFeatureLowering::NumericScalar
				|| feature.schema().source_vector() != span.source_vector()
				|| feature.values().len() != data.rows()
			{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("prepared token position {feature_index} disagrees with its saved sequence span"),
				)); }
			let PreparedInferenceValues::I32(values) = feature.values() else { return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("prepared token position {feature_index} did not retain exact int32 IDs"),
				)); }; if let Some((row, token)) = values .iter() .copied() .enumerate()
				.find(|(_, token)| *token < 0 || *token >= vocabulary)
			{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
						"inference token {token} at row {row}, position {feature_index} is outside 0..{vocabulary}"
					), )); }
			let raw = self.external( InferenceInputRole::Feature { feature: feature_index, source_vector: span.source_vector(),
				}, DType::I32, shape(&[rows])?, values.iter() .flat_map(|value| value.to_le_bytes()) .collect(), )?;
			combined = self.scatter_i32_feature_block(combined, raw, rows, feature_width, start)?; }
		if u64::try_from(spans.len()).ok() != Some(feature_width) { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"saved embedding feature spans do not cover the fixed sequence width",
			)); }
		let dense_shape = shape(&[rows, feature_width])?; let identity = self.identity_indices(dense_shape.clone())?;
		let dense = self.tensor(DType::I32, dense_shape)?; self.emit( vec![combined, identity], vec![dense],
			PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }), forbidden_aliases(2, 1), )?; Ok(dense) }

	fn zero_f32(&mut self, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let seed = self.tensor(DType::I32, output_shape.clone())?; self.emit( Vec::new(), vec![seed],
			PrimitiveKind::IndexMap(IndexMap { start: 0, element_step: 0, iteration_step: 0, modulus: None, }), Vec::new(), )?;
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(vec![seed], vec![output], zero_f32_program()?)?; Ok(output) }

	fn zero_i32(&mut self, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let output = self.tensor(DType::I32, output_shape)?; self.emit( Vec::new(), vec![output],
			PrimitiveKind::IndexMap(IndexMap { start: 0, element_step: 0, iteration_step: 0, modulus: None, }), Vec::new(), )?;
		Ok(output) }

	fn one_hot(&mut self, labels: ValueId, rows: u64, classes: u64) -> InferenceCompileResult<ValueId> {
		let classes_i32 = checked_i32(classes, "categorical one-hot width")?;
		let row_shape = shape(&[rows])?; let row_bases = self.tensor(DType::I32, row_shape.clone())?; self.emit( Vec::new(),
			vec![row_bases], PrimitiveKind::IndexMap(IndexMap { start: 0, element_step: classes_i32, iteration_step: 0,
				modulus: None, }), Vec::new(), )?; let destinations = self.tensor(DType::I32, row_shape.clone())?;
		let updates = self.tensor(DType::F32, row_shape)?; self.emit_elementwise( vec![labels, row_bases],
			vec![destinations, updates], checked_one_hot_update_program(classes_i32)?, )?;
		let elements = rows.checked_mul(classes).ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"categorical one-hot element count overflowed u64",
			) })?;
		require_i32_indexable(elements, "categorical one-hot element count")?;
		let base = self.zero_f32(shape(&[elements])?)?; let encoded = self.tensor(DType::F32, shape(&[elements])?)?;
		self.emit( vec![base, destinations, updates], vec![encoded], PrimitiveKind::Scatter(Scatter { axis: 0,
				bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), )?; Ok(encoded)
	}

	fn scatter_feature_block( &mut self, base: ValueId, block: ValueId, rows: u64, total_width: u64, start: u64,
		block_width: u64, ) -> InferenceCompileResult<ValueId> { let elements = rows.checked_mul(block_width).ok_or_else(|| {
			InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
				"feature block element count overflowed u64",
			) })?;
		require_i32_indexable(elements, "feature block element count")?;
		let positions = self.tensor(DType::I32, shape(&[elements])?)?; self.emit( Vec::new(), vec![positions],
			PrimitiveKind::IndexMap(IndexMap { start: 0, element_step: 1, iteration_step: 0, modulus: None, }), Vec::new(), )?;
		let destinations = self.tensor(DType::I32, shape(&[elements])?)?; self.emit_elementwise( vec![positions],
			vec![destinations], feature_destination_program(
				checked_i32(total_width, "dense feature width")?,
				checked_i32(start, "feature span start")?,
				checked_i32(block_width, "feature span width")?,
			)?, )?; let output_shape = self.tensor_ref(base)?.shape.clone(); let output = self.tensor(DType::F32, output_shape)?;
		self.emit( vec![base, destinations, block], vec![output], PrimitiveKind::Scatter(Scatter { axis: 0,
				bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), )?; Ok(output) }

	fn scatter_i32_feature_block( &mut self, base: ValueId, block: ValueId, rows: u64, total_width: u64, start: u64,
	) -> InferenceCompileResult<ValueId> { let positions = self.identity_indices(shape(&[rows])?)?;
		let destinations = self.tensor(DType::I32, shape(&[rows])?)?; self.emit_elementwise( vec![positions],
			vec![destinations], feature_destination_program(
				checked_i32(total_width, "token sequence width")?,
				checked_i32(start, "token position")?,
				1, )?, )?; let output = self.tensor(DType::I32, self.tensor_ref(base)?.shape.clone())?; self.emit(
			vec![base, destinations, block], vec![output], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), )?; Ok(output) }

	fn apply_data_normalization( &mut self, checkpoint: &CheckpointArtifact, input: ValueId, rows: u64, columns: u64,
	) -> InferenceCompileResult<ValueId> { if checkpoint.config().data_normalization == DenseDataNormalization::Identity {
			if !checkpoint.normalization().is_empty() { return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					"identity-input checkpoint unexpectedly retains fitted normalization tensors",
				)); }
			return Ok(input); }
		let mask = self.external( InferenceInputRole::FeatureNormalizationMask, DType::F32, shape(&[columns])?, checkpoint
				.feature_normalization_mask() .iter() .flat_map(|bits| bits.to_le_bytes()) .collect(), )?;
		let matrix_shape = shape(&[rows, columns])?; match checkpoint.config().data_normalization {
			DenseDataNormalization::Identity => unreachable!("handled before the normalization mask"),
			DenseDataNormalization::ZScore => { let [mean, variance] = checkpoint.normalization() else {
					return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
						"z-score checkpoint does not retain exactly mean and variance tensors",
					)); }; let mean = self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationMean, mean)?; let variance =
					self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationVariance, variance)?;
				let output = self.tensor(DType::F32, matrix_shape)?; self.emit_elementwise( vec![input, mean, variance, mask],
					vec![output], z_score_program(checkpoint.config().normalization_epsilon, true)?, )?; Ok(output) }
			DenseDataNormalization::MinMax => { let [minimum, maximum] = checkpoint.normalization() else {
					return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
						"min-max checkpoint does not retain exactly minimum and maximum tensors",
					)); }; let minimum =
					self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationMinimum, minimum)?; let maximum =
					self.external_checkpoint_tensor(InferenceInputRole::DataNormalizationMaximum, maximum)?;
				let output = self.tensor(DType::F32, matrix_shape)?; self.emit_elementwise( vec![input, minimum, maximum, mask],
					vec![output], min_max_program(checkpoint.config().normalization_epsilon, true)?, )?; Ok(output) }
			DenseDataNormalization::L2Norm => { if !checkpoint.normalization().is_empty() {
					return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
						"L2 checkpoint unexpectedly retains fitted normalization tensors",
					)); }
				let squares = self.tensor(DType::F32, matrix_shape.clone())?;
				self.emit_elementwise(vec![input, mask], vec![squares], l2_square_program(true)?)?;
				let norms_squared = self.tensor(DType::F32, shape(&[rows, 1])?)?; self.reduce( squares, norms_squared,
					ReduceOperator::Sum, &[1], true, checkpoint.config().reduction_tree_lanes, )?;
				let output = self.tensor(DType::F32, matrix_shape)?; self.emit_elementwise( vec![input, norms_squared, mask],
					vec![output], l2_norm_program(checkpoint.config().normalization_epsilon, true)?, )?; Ok(output) } } }

	fn normalize_knn_features( &mut self, values: [ValueId; 2], dimensions: [u64; 3],
		normalization: Option<DenseDataNormalization>, mask_bits: Option<&[u32]>, tree_lanes: u32,
	) -> InferenceCompileResult<(ValueId, ValueId)> { let [query, reference] = values;
		let [query_rows, reference_rows, columns] = dimensions; let Some(normalization) = normalization else {
			return Ok((query, reference)); }; let mask = mask_bits .map(|bits| { self.external(
					InferenceInputRole::FeatureNormalizationMask, DType::F32, shape(&[columns])?,
					bits.iter().flat_map(|bits| bits.to_le_bytes()).collect(), ) }) .transpose()?; const EPSILON: f32 = 1.0e-6;
		match normalization { DenseDataNormalization::Identity => Ok((query, reference)), DenseDataNormalization::ZScore => {
				let column_shape = shape(&[columns])?; let sums = self.tensor(DType::F32, column_shape.clone())?; self.reduce(
					reference, sums, ReduceOperator::Sum, &[0], false, tree_lanes, )?;
				let means = self.tensor(DType::F32, column_shape.clone())?; self.emit_elementwise( vec![sums], vec![means],
					divide_constant_program(reference_rows as f32)?, )?;
				let centered = self.tensor(DType::F32, shape(&[reference_rows, columns])?)?;
				self.emit_elementwise(vec![reference, means], vec![centered], subtract_program()?)?;
				let squares = self.tensor(DType::F32, shape(&[reference_rows, columns])?)?;
				self.emit_elementwise(vec![centered], vec![squares], square_program()?)?;
				let variance_sums = self.tensor(DType::F32, column_shape.clone())?; self.reduce( squares, variance_sums,
					ReduceOperator::Sum, &[0], false, tree_lanes, )?; let variances = self.tensor(DType::F32, column_shape)?;
				self.emit_elementwise( vec![variance_sums], vec![variances], divide_constant_program(reference_rows as f32)?, )?;
				Ok(( self.apply_knn_z_score([query, means, variances], [query_rows, columns], mask, EPSILON)?,
					self.apply_knn_z_score( [reference, means, variances], [reference_rows, columns], mask, EPSILON, )?, )) }
			DenseDataNormalization::MinMax => { let column_shape = shape(&[columns])?;
				let minimum = self.tensor(DType::F32, column_shape.clone())?; self.reduce( reference, minimum,
					ReduceOperator::Minimum, &[0], false, tree_lanes, )?; let maximum = self.tensor(DType::F32, column_shape)?;
				self.reduce( reference, maximum, ReduceOperator::Maximum, &[0], false, tree_lanes, )?; Ok((
					self.apply_knn_min_max([query, minimum, maximum], [query_rows, columns], mask, EPSILON)?, self.apply_knn_min_max(
						[reference, minimum, maximum], [reference_rows, columns], mask, EPSILON, )?, )) }
			DenseDataNormalization::L2Norm => Ok(( self.apply_knn_l2(query, query_rows, columns, mask, EPSILON, tree_lanes)?,
				self.apply_knn_l2( reference, reference_rows, columns, mask, EPSILON, tree_lanes, )?, )), } }

	fn apply_knn_z_score( &mut self, values: [ValueId; 3], dimensions: [u64; 2], mask: Option<ValueId>, epsilon: f32,
	) -> InferenceCompileResult<ValueId> { let [input, mean, variance] = values; let [rows, columns] = dimensions;
		let output = self.tensor(DType::F32, shape(&[rows, columns])?)?; let mut inputs = vec![input, mean, variance];
		if let Some(mask) = mask { inputs.push(mask); }
		self.emit_elementwise( inputs, vec![output], z_score_program(epsilon, mask.is_some())?, )?; Ok(output) }

	fn apply_knn_min_max( &mut self, values: [ValueId; 3], dimensions: [u64; 2], mask: Option<ValueId>, epsilon: f32,
	) -> InferenceCompileResult<ValueId> { let [input, minimum, maximum] = values; let [rows, columns] = dimensions;
		let output = self.tensor(DType::F32, shape(&[rows, columns])?)?; let mut inputs = vec![input, minimum, maximum];
		if let Some(mask) = mask { inputs.push(mask); }
		self.emit_elementwise( inputs, vec![output], min_max_program(epsilon, mask.is_some())?, )?; Ok(output) }

	fn apply_knn_l2( &mut self, input: ValueId, rows: u64, columns: u64, mask: Option<ValueId>, epsilon: f32,
		tree_lanes: u32, ) -> InferenceCompileResult<ValueId> { let matrix_shape = shape(&[rows, columns])?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?; let mut square_inputs = vec![input];
		if let Some(mask) = mask { square_inputs.push(mask); }
		self.emit_elementwise( square_inputs, vec![squares], l2_square_program(mask.is_some())?, )?;
		let norms_squared = self.tensor(DType::F32, shape(&[rows, 1])?)?; self.reduce( squares, norms_squared,
			ReduceOperator::Sum, &[1], true, tree_lanes, )?; let output = self.tensor(DType::F32, matrix_shape)?;
		let mut inputs = vec![input, norms_squared]; if let Some(mask) = mask { inputs.push(mask); }
		self.emit_elementwise( inputs, vec![output], l2_norm_program(epsilon, mask.is_some())?, )?; Ok(output) }

	pub(crate) fn compile_embedding( &mut self, block_index: usize, embedding: &crate::CheckpointEmbeddingImage,
		input: ValueId, rows: u64, input_width: u64, ) -> InferenceCompileResult<ValueId> {
		if embedding.sequence_length().get() != input_width { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"embedding block {block_index} sequence length {} differs from input width {input_width}",
					embedding.sequence_length() ), )); }
		self.require_tensor( input, DType::I32, &[rows, input_width],
			"embedding token matrix",
		)?; validate_checkpoint_parameter_image( embedding.table().parameter(),
			&[embedding.vocabulary().get(), embedding.dimensions().get()],
			"embedding table",
		)?; let table = self.external_checkpoint_tensor( InferenceInputRole::EmbeddingTable { block: block_index },
			embedding.table().parameter(), )?; let token_rows = rows.checked_mul(input_width).ok_or_else(|| {
			InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
				"embedding inference token count overflowed u64",
			) })?; let output_width = input_width .checked_mul(embedding.dimensions().get()) .ok_or_else(|| {
				InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
					"embedding inference output width overflowed u64",
				) })?; let indices = self.pack_i32_matrix_to_flat(input, rows, input_width)?; let gathered = self.tensor(
			DType::F32, shape(&[token_rows, embedding.dimensions().get()])?, )?; self.emit( vec![table, indices], vec![gathered],
			PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }), forbidden_aliases(2, 1), )?;
		let flat = self.pack_matrix_to_flat(gathered, token_rows, embedding.dimensions().get())?;
		let output_shape = shape(&[rows, output_width])?; let identity = self.identity_indices(output_shape.clone())?;
		let output = self.tensor(DType::F32, output_shape)?; self.emit( vec![flat, identity], vec![output],
			PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }), forbidden_aliases(2, 1), )?; Ok(output) }

	pub(crate) fn compile_attention( &mut self, block_index: usize, attention: &crate::CheckpointAttentionImage,
		input: ValueId, geometry: [u64; 4], tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		let [rows, input_width, logical_length, logical_channels] = geometry;
		if attention.sequence_length().get() != logical_length || attention.dimensions().get() != logical_channels
			|| attention .heads() .get() .checked_mul(attention.head_dimension().get()) != Some(logical_channels)
		{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("attention block {block_index} geometry differs from its preceding embedding"),
			)); }
		let expected_width = logical_length .checked_mul(logical_channels) .ok_or_else(|| { InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					"attention inference width overflowed u64",
				) })?; if input_width != expected_width { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"attention block {block_index} input width {input_width} differs from fixed sequence width {expected_width}"
				), )); }
		checked_i32( checked_product( &[rows, logical_length, logical_channels],
				"attention inference element count",
			)?,
			"attention inference element count",
		)?; let parameter_shape = [logical_channels, logical_channels]; for (name, parameter) in [
			("query", attention.query()),
			("key", attention.key()),
			("value", attention.value()),
			("output", attention.output()),
		] { validate_checkpoint_parameter_image( parameter.parameter(), &parameter_shape,
				&format!("attention {name} matrix"),
			)?; }
		let query_weight = self.external_checkpoint_tensor( InferenceInputRole::AttentionQuery { block: block_index },
			attention.query().parameter(), )?; let key_weight = self.external_checkpoint_tensor(
			InferenceInputRole::AttentionKey { block: block_index }, attention.key().parameter(), )?;
		let value_weight = self.external_checkpoint_tensor( InferenceInputRole::AttentionValue { block: block_index },
			attention.value().parameter(), )?; let output_weight = self.external_checkpoint_tensor(
			InferenceInputRole::AttentionOutput { block: block_index }, attention.output().parameter(), )?;

		let input_sequence = self.reinterpret_f32(input, shape(&[rows, logical_length, logical_channels])?)?;
		let query = self.attention_projection( input_sequence, query_weight, rows, logical_length, logical_channels, )?;
		let key = self.attention_projection( input_sequence, key_weight, rows, logical_length, logical_channels, )?;
		let value = self.attention_projection( input_sequence, value_weight, rows, logical_length, logical_channels, )?;
		let heads = attention.heads().get(); let head_dimension = attention.head_dimension().get();
		let head_shape = shape(&[rows, logical_length, heads, head_dimension])?;
		let query = self.reinterpret_f32(query, head_shape.clone())?;
		let key = self.reinterpret_f32(key, head_shape.clone())?; let value = self.reinterpret_f32(value, head_shape)?;
		let score_shape = shape(&[rows, heads, logical_length, logical_length])?;
		let scores = self.tensor(DType::F32, score_shape.clone())?; self.emit( vec![query, key], vec![scores],
			PrimitiveKind::Contraction(Contraction { batch_axes: vec![(0, 0), (2, 2)], contract_axes: vec![(3, 3)], }),
			forbidden_aliases(2, 1), )?; let scaled = self.tensor(DType::F32, score_shape)?; self.emit_elementwise( vec![scores],
			vec![scaled], multiply_constant_program(1.0 / (head_dimension as f32).sqrt())?, )?;
		let probabilities = self.causal_softmax(scaled, rows, heads, logical_length, tree_lanes)?; let context = self.tensor(
			DType::F32, shape(&[rows, heads, logical_length, head_dimension])?, )?; self.emit( vec![probabilities, value],
			vec![context], PrimitiveKind::Contraction(Contraction { batch_axes: vec![(0, 0), (1, 2)],
				contract_axes: vec![(3, 1)], }), forbidden_aliases(2, 1), )?;
		let context = self.head_major_to_sequence(context, rows, logical_length, heads, head_dimension)?;
		let context = self.reinterpret_f32(context, shape(&[rows, logical_length, logical_channels])?)?;
		let output = self.attention_projection( context, output_weight, rows, logical_length, logical_channels, )?;
		self.reinterpret_f32(output, shape(&[rows, expected_width])?) }

	pub(crate) fn compile_rnn( &mut self, block_index: usize, rnn: &crate::CheckpointRnnImage, input: ValueId, rows: u64,
		input_width: u64, ) -> InferenceCompileResult<ValueId> { if rnn.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"RNN block {block_index} sequence length {} differs from input width {input_width}",
					rnn.sequence_length() ), )); }
		self.require_tensor( input, DType::F32, &[rows, input_width],
			"vanilla-RNN input sequence",
		)?; let width = rnn.width().get(); validate_checkpoint_parameter_image( rnn.input_weight().parameter(), &[1, width],
			"RNN input weight",
		)?; validate_checkpoint_parameter_image( rnn.recurrent_weight().parameter(), &[width, width],
			"RNN recurrent weight",
		)?;
		validate_checkpoint_parameter_image(rnn.bias().parameter(), &[width], "RNN bias")?;
		let input_weight = self.external_checkpoint_tensor( InferenceInputRole::RnnInputWeight { block: block_index },
			rnn.input_weight().parameter(), )?; let recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::RnnRecurrentWeight { block: block_index }, rnn.recurrent_weight().parameter(), )?;
		let bias = self.external_checkpoint_tensor( InferenceInputRole::RnnBias { block: block_index },
			rnn.bias().parameter(), )?; Ok(lower_rnn_sequence( self, input, rows, input_width, width, RecurrentGateParameters {
				input_weight, recurrent_weight, bias, }, )? .0) }

	pub(crate) fn compile_gru( &mut self, block_index: usize, gru: &crate::CheckpointGruImage, input: ValueId, rows: u64,
		input_width: u64, ) -> InferenceCompileResult<ValueId> { if gru.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"GRU block {block_index} sequence length {} differs from input width {input_width}",
					gru.sequence_length() ), )); }
		self.require_tensor( input, DType::F32, &[rows, input_width],
			"GRU input sequence",
		)?; let width = gru.width().get(); for (name, parameter) in [
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
				gru.candidate_recurrent_weight(), ), ] { validate_checkpoint_parameter_image( parameter.parameter(),
				&[width, width],
				&format!("GRU {name}"),
			)?; }
		for (name, parameter) in [
			("reset bias", gru.reset_bias()),
			("update bias", gru.update_bias()),
			("candidate bias", gru.candidate_bias()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[width], &format!("GRU {name}"))?;
		}

		let reset_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruResetInputWeight { block: block_index }, gru.reset_input_weight().parameter(), )?;
		let reset_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruResetRecurrentWeight { block: block_index }, gru.reset_recurrent_weight().parameter(), )?;
		let reset_bias = self.external_checkpoint_tensor( InferenceInputRole::GruResetBias { block: block_index },
			gru.reset_bias().parameter(), )?; let update_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruUpdateInputWeight { block: block_index }, gru.update_input_weight().parameter(), )?;
		let update_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruUpdateRecurrentWeight { block: block_index }, gru.update_recurrent_weight().parameter(), )?;
		let update_bias = self.external_checkpoint_tensor( InferenceInputRole::GruUpdateBias { block: block_index },
			gru.update_bias().parameter(), )?; let candidate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruCandidateInputWeight { block: block_index }, gru.candidate_input_weight().parameter(), )?;
		let candidate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::GruCandidateRecurrentWeight { block: block_index }, gru.candidate_recurrent_weight().parameter(),
		)?; let candidate_bias = self.external_checkpoint_tensor( InferenceInputRole::GruCandidateBias { block: block_index },
			gru.candidate_bias().parameter(), )?;

		Ok(lower_gru_sequence( self, input, rows, input_width, width, GruForwardParameters { reset: RecurrentGateParameters {
					input_weight: reset_input_weight, recurrent_weight: reset_recurrent_weight, bias: reset_bias, },
				update: RecurrentGateParameters { input_weight: update_input_weight, recurrent_weight: update_recurrent_weight,
					bias: update_bias, }, candidate: RecurrentGateParameters { input_weight: candidate_input_weight,
					recurrent_weight: candidate_recurrent_weight, bias: candidate_bias, }, }, )? .0) }

	pub(crate) fn compile_lstm( &mut self, block_index: usize, lstm: &crate::CheckpointLstmImage, input: ValueId,
		rows: u64, input_width: u64, ) -> InferenceCompileResult<ValueId> { if lstm.sequence_length().get() != input_width {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"LSTM block {block_index} sequence length {} differs from input width {input_width}",
					lstm.sequence_length() ), )); }
		self.require_tensor( input, DType::F32, &[rows, input_width],
			"LSTM input sequence",
		)?; let width = lstm.width().get(); for (name, parameter) in [
			("input gate input weight", lstm.input_gate_input_weight()),
			("forget gate input weight", lstm.forget_gate_input_weight()),
			("output gate input weight", lstm.output_gate_input_weight()),
			("candidate input weight", lstm.candidate_input_weight()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[1, width], &format!("LSTM {name}"))?;
		}
		for (name, parameter) in [ (
				"input gate recurrent weight",
				lstm.input_gate_recurrent_weight(), ), (
				"forget gate recurrent weight",
				lstm.forget_gate_recurrent_weight(), ), (
				"output gate recurrent weight",
				lstm.output_gate_recurrent_weight(), ), (
				"candidate recurrent weight",
				lstm.candidate_recurrent_weight(), ), ] { validate_checkpoint_parameter_image( parameter.parameter(),
				&[width, width],
				&format!("LSTM {name}"),
			)?; }
		for (name, parameter) in [
			("input gate bias", lstm.input_gate_bias()),
			("forget gate bias", lstm.forget_gate_bias()),
			("output gate bias", lstm.output_gate_bias()),
			("candidate bias", lstm.candidate_bias()),
		] {
			validate_checkpoint_parameter_image(parameter.parameter(), &[width], &format!("LSTM {name}"))?;
		}

		let input_gate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmInputGateInputWeight { block: block_index }, lstm.input_gate_input_weight().parameter(), )?;
		let input_gate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmInputGateRecurrentWeight { block: block_index },
			lstm.input_gate_recurrent_weight().parameter(), )?; let input_gate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmInputGateBias { block: block_index }, lstm.input_gate_bias().parameter(), )?;
		let forget_gate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmForgetGateInputWeight { block: block_index }, lstm.forget_gate_input_weight().parameter(),
		)?; let forget_gate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmForgetGateRecurrentWeight { block: block_index },
			lstm.forget_gate_recurrent_weight().parameter(), )?; let forget_gate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmForgetGateBias { block: block_index }, lstm.forget_gate_bias().parameter(), )?;
		let output_gate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmOutputGateInputWeight { block: block_index }, lstm.output_gate_input_weight().parameter(),
		)?; let output_gate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmOutputGateRecurrentWeight { block: block_index },
			lstm.output_gate_recurrent_weight().parameter(), )?; let output_gate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmOutputGateBias { block: block_index }, lstm.output_gate_bias().parameter(), )?;
		let candidate_input_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmCandidateInputWeight { block: block_index }, lstm.candidate_input_weight().parameter(), )?;
		let candidate_recurrent_weight = self.external_checkpoint_tensor(
			InferenceInputRole::LstmCandidateRecurrentWeight { block: block_index },
			lstm.candidate_recurrent_weight().parameter(), )?; let candidate_bias = self.external_checkpoint_tensor(
			InferenceInputRole::LstmCandidateBias { block: block_index }, lstm.candidate_bias().parameter(), )?;

		Ok(lower_lstm_sequence( self, input, rows, input_width, width, LstmForwardParameters {
				input: RecurrentGateParameters { input_weight: input_gate_input_weight,
					recurrent_weight: input_gate_recurrent_weight, bias: input_gate_bias, }, forget: RecurrentGateParameters {
					input_weight: forget_gate_input_weight, recurrent_weight: forget_gate_recurrent_weight, bias: forget_gate_bias, },
				output: RecurrentGateParameters { input_weight: output_gate_input_weight,
					recurrent_weight: output_gate_recurrent_weight, bias: output_gate_bias, }, candidate: RecurrentGateParameters {
					input_weight: candidate_input_weight, recurrent_weight: candidate_recurrent_weight, bias: candidate_bias, }, }, )?
		.0) }

	fn gather_matrix_column( &mut self, matrix: ValueId, rows: u64, columns: u64, column: u64,
	) -> InferenceCompileResult<ValueId> { if column >= columns { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("matrix column {column} is outside width {columns}"),
			)); }
		self.require_tensor(matrix, DType::F32, &[rows, columns], "matrix column source")?;
		let index = self.tensor(DType::I32, shape(&[1])?)?; self.emit( Vec::new(), vec![index],
			PrimitiveKind::IndexMap(IndexMap {
				start: checked_i32(column, "matrix column")?,
				element_step: 0, iteration_step: 0, modulus: None, }), Vec::new(), )?;
		let gathered = self.tensor(DType::F32, shape(&[rows, 1])?)?; self.emit( vec![matrix, index], vec![gathered],
			PrimitiveKind::Gather(Gather { axis: 1, bounds: IndexBounds::Reject, }), forbidden_aliases(2, 1), )?; Ok(gathered) }

	fn attention_projection( &mut self, input: ValueId, weight: ValueId, rows: u64, sequence: u64, dimensions: u64,
	) -> InferenceCompileResult<ValueId> { let output = self.tensor(DType::F32, shape(&[rows, sequence, dimensions])?)?;
		self.emit( vec![input, weight], vec![output], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(2, 0)], }), forbidden_aliases(2, 1), )?; Ok(output) }

	pub(crate) fn compile_layer( &mut self, layer_index: usize, layer: &crate::CheckpointLayerImage, input: ValueId,
		dimensions: [u64; 2], normalization_epsilon: f32, tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		let [rows, input_width] = dimensions; let output_width = layer.declaration().width().get();
		validate_checkpoint_parameter_image( layer.weight().parameter(), &[input_width, output_width],
			"layer weight",
		)?;
		validate_checkpoint_parameter_image(layer.bias().parameter(), &[output_width], "layer bias")?;
		let weight = self.external_checkpoint_tensor( InferenceInputRole::LayerWeight { layer: layer_index },
			layer.weight().parameter(), )?; let bias = self.external_checkpoint_tensor(
			InferenceInputRole::LayerBias { layer: layer_index }, layer.bias().parameter(), )?;
		let output_shape = shape(&[rows, output_width])?; let preactivation = self.tensor(DType::F32, output_shape.clone())?;
		self.materialize(
			"gpu_linear_into",
			&[("input", input), ("weight", weight), ("bias", bias)],
			&[("output", preactivation)],
			"input",
			&PreparedParameters::new(), )?; let mut current = preactivation; let mut prelu = layer.prelu().iter().enumerate();
		for operation in layer.declaration().operations().iter().copied() { current = match operation {
				DenseOperation::Activation(activation) => { let alpha = if activation.learned_parameters() == 1 {
						let (occurrence, parameter) = prelu.next().ok_or_else(|| { InferenceCompileError::new(
								InferenceCompileErrorKind::InconsistentCheckpoint,
								format!("layer {layer_index} omitted a PReLU scalar"),
							) })?; Some(self.external_checkpoint_tensor( InferenceInputRole::LayerPRelu { layer: layer_index, occurrence, },
							parameter.parameter(), )?) } else { None };
					self.apply_activation(current, activation, alpha, output_shape.clone())? }
				DenseOperation::Normalization(normalization) => self.apply_model_normalization( current, normalization, rows,
					output_width, normalization_epsilon, tree_lanes, )?, }; }
		if prelu.next().is_some() { return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("layer {layer_index} retains an extra PReLU scalar"),
			)); }
		Ok(current) }

	pub(crate) fn compile_convolution( &mut self, block_index: usize, convolution: &crate::CheckpointConvolutionImage,
		input: ValueId, input_geometry: [u64; 4], normalization_epsilon: f32, tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> { let [rows, input_width, logical_length, logical_channels] = input_geometry;
		let geometry = convolution.geometry(); if geometry.input_length().get() != logical_length
			|| geometry.input_channels().get() != logical_channels
			|| geometry.input_width().map(NonZeroU64::get) != Some(input_width)
		{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"convolution block {block_index} input geometry disagrees with the preceding logical shape"
				), )); }
		let output_width = geometry.output_width().ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				format!("convolution block {block_index} output width overflowed u64"),
			) })?;
		self.require_tensor(input, DType::F32, &[rows, input_width], "convolution input")?;
		validate_checkpoint_parameter_image( convolution.weight().parameter(), &[ geometry.kernel().get(),
				geometry.input_channels().get(), geometry.filters().get(), ],
			"convolution weight",
		)?; validate_checkpoint_parameter_image( convolution.bias().parameter(), &[geometry.filters().get()],
			"convolution bias",
		)?; let preparation = prepare_channelwise_convolution_1d( rows, logical_length, logical_channels,
			geometry.filters().get(), geometry.kernel().get(), )?;
		if preparation.output_length() != geometry.output_length().get() { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("convolution block {block_index} preparation disagrees with its saved output length"),
			)); }
		let flat_input = self.pack_matrix_to_flat(input, rows, input_width)?; let window_indices = self.external_i32(
			InferenceInputRole::ConvolutionWindowIndices { block: block_index }, shape(&preparation.window_indices_shape())?,
			preparation.window_indices(), )?;
		let columns = self.tensor(DType::F32, shape(&preparation.window_indices_shape())?)?; self.emit(
			vec![flat_input, window_indices], vec![columns], PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject,
			}), forbidden_aliases(2, 1), )?; let weight = self.external_checkpoint_tensor(
			InferenceInputRole::ConvolutionWeight { block: block_index }, convolution.weight().parameter(), )?;
		let bias = self.external_checkpoint_tensor( InferenceInputRole::ConvolutionBias { block: block_index },
			convolution.bias().parameter(), )?; let contracted = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		self.emit( vec![columns, weight], vec![contracted], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(2, 0), (3, 1)], }), forbidden_aliases(2, 1), )?;
		let grouped = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		self.emit_elementwise(vec![contracted, bias], vec![grouped], add_program()?)?;
		let mut current = self.unpack_pool_to_matrix( grouped, rows, geometry.output_length().get(), geometry.filters().get(),
			output_width.get(), )?; let mut prelu = convolution.prelu().iter().enumerate();
		for operation in convolution.declaration().operations().iter().copied() {
			let alpha = if matches!(operation, DenseOperation::Activation(activation) if activation.learned_parameters() == 1) {
				let (occurrence, parameter) = prelu.next().ok_or_else(|| { InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						format!("convolution block {block_index} omitted a PReLU scalar"),
					) })?;
				validate_checkpoint_parameter_image(parameter.parameter(), &[1], "convolution PReLU scalar")?;
				Some(self.external_checkpoint_tensor( InferenceInputRole::ConvolutionPRelu { block: block_index, occurrence, },
					parameter.parameter(), )?) } else { None }; current = self.apply_saved_operation( current, operation, alpha,
				[rows, output_width.get()], normalization_epsilon, tree_lanes, )?; }
		if prelu.next().is_some() { return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("convolution block {block_index} retains an extra PReLU scalar"),
			)); }
		Ok(current) }

	pub(crate) fn compile_pool( &mut self, block_index: usize, pool: &crate::CheckpointPoolImage, input: ValueId,
		input_geometry: [u64; 4], tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		let [rows, input_width, logical_length, logical_channels] = input_geometry; if pool.input_width().get() != input_width
			|| pool.input_length().get() != logical_length
			|| pool.channels().get() != logical_channels
		{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"pool block {block_index} expects width {} as logical {} x {}, received width {input_width} as logical {logical_length} x {logical_channels}",
					pool.input_width(), pool.input_length(), pool.channels(), ), )); }
		let expected_input_width = logical_length .checked_mul(logical_channels) .ok_or_else(|| { InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					format!("pool block {block_index} input width overflowed u64"),
				) })?; let expected_output_width = pool .output_length() .get() .checked_mul(logical_channels) .ok_or_else(|| {
				InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
					format!("pool block {block_index} output width overflowed u64"),
				) })?; if expected_input_width != input_width || expected_output_width != pool.output_width().get() {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("pool block {block_index} cached widths disagree with its logical shape"),
			)); }
		self.require_tensor( input, DType::F32, &[rows, input_width],
			"maximum-pool input",
		)?; let preparation = prepare_channelwise_max_pool_1d(rows, logical_length, logical_channels, pool.size().get())?;
		if preparation.groups() != pool.output_length().get() { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("pool block {block_index} preparation disagrees with its saved output length"),
			)); }
		let flat_input = self.pack_matrix_to_flat(input, rows, input_width)?; let window_indices = self.external_i32(
			InferenceInputRole::PoolWindowIndices { block: block_index }, shape(&preparation.window_indices_shape())?,
			preparation.window_indices(), )?; let winner_bases = self.external_i32(
			InferenceInputRole::PoolWinnerBases { block: block_index }, shape(&preparation.output_shape())?,
			preparation.winner_bases(), )?; let pooled = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		let unused_winners = self.tensor(DType::I32, shape(&preparation.output_shape())?)?;
		let parameters = preparation.forward_parameters(u64::from(tree_lanes)); self.materialize(
			"recipe_max_pool_1d",
			&[
				("values", flat_input),
				("window_indices", window_indices),
				("winner_bases", winner_bases),
			],
			&[("pooled", pooled), ("winning_indices", unused_winners)],
			"values",
			&parameters, )?; self.unpack_pool_to_matrix( pooled, rows, pool.output_length().get(), logical_channels,
			pool.output_width().get(), ) }

	pub(crate) fn compile_kmeans( &mut self, block_index: usize, kmeans: &crate::CheckpointKMeansImage, input: ValueId,
		rows: u64, input_width: u64, tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		if kmeans.input_width().get() != input_width { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"K-means block {block_index} expects input width {}, received {input_width}",
					kmeans.input_width() ), )); }
		self.require_tensor(input, DType::F32, &[rows, input_width], "K-means input")?;
		validate_checkpoint_parameter_image( kmeans.centroids(), &[kmeans.clusters().get(), input_width],
			"K-means centroids",
		)?; let centroids = self.external_checkpoint_tensor( InferenceInputRole::KMeansCentroids { block: block_index },
			kmeans.centroids(), )?; let distances = self.tensor(DType::F32, shape(&[rows, kmeans.clusters().get()])?)?;
		let parameters = [
			("queries".to_owned(), PreparedParameter::U64(rows)),
			(
				"training_rows".to_owned(),
				PreparedParameter::U64(kmeans.clusters().get()), ),
			("dimensions".to_owned(), PreparedParameter::U64(input_width)),
			(
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)), ), ] .into_iter() .collect::<PreparedParameters>(); self.materialize(
			"gpu_pairwise_l2",
			&[("query", input), ("training", centroids)],
			&[("distances", distances)],
			"query",
			&parameters, )?; Ok(distances) }

	pub(crate) fn compile_tree( &mut self, block_index: usize, tree: &crate::CheckpointTreeImage, input: ValueId,
		rows: u64, input_width: u64, tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		if tree.input_width().get() != input_width { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"tree block {block_index} expects input width {}, received {input_width}",
					tree.input_width() ), )); }
		self.require_tensor(input, DType::F32, &[rows, input_width], "tree input")?;
		let declaration = tree.declaration(); let requirements = tree_ensemble_inference_requirements( rows, input_width,
			declaration.trees().get(), declaration.depth().get(), tree.output_width().get(), )?;
		if requirements.internal_nodes_per_tree != tree.internal_nodes_per_tree().get()
			|| requirements.leaves_per_tree != tree.leaves_per_tree().get()
		{ return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("tree block {block_index} cached extents differ from its declaration"),
			)); }
		let split_features = self.external_checkpoint_tensor( InferenceInputRole::TreeSplitFeatures { block: block_index },
			tree.split_features(), )?; let split_thresholds = self.external_checkpoint_tensor(
			InferenceInputRole::TreeSplitThresholds { block: block_index }, tree.split_thresholds(), )?;
		let leaf_values = self.external_checkpoint_tensor( InferenceInputRole::TreeLeafValues { block: block_index },
			tree.leaf_values().parameter(), )?; let flat_features = self.pack_matrix_to_flat(input, rows, input_width)?;
		let predictions = self.tensor(DType::F32, shape(&[rows, tree.output_width().get()])?)?;
		let first_value = self.next_value; let first_kernel = self.next_kernel; self.next_value = self .next_value
			.checked_add(requirements.intermediate_values) .ok_or_else(identity_exhausted)?; self.next_kernel = self .next_kernel
			.checked_add(requirements.kernels) .ok_or_else(identity_exhausted)?; let request = TreeEnsembleInferenceRequest {
			features: self.tensor_ref(flat_features)?.clone(), split_features: self.tensor_ref(split_features)?.clone(),
			split_thresholds: self.tensor_ref(split_thresholds)?.clone(), leaf_values: self.tensor_ref(leaf_values)?.clone(),
			predictions: self.tensor_ref(predictions)?.clone(), rows, feature_width: input_width,
			trees: declaration.trees().get(), depth: declaration.depth().get(), outputs: tree.output_width().get(), scale: 1.0,
			tree_lanes, identity_namespace: IdentityNamespace::new( ValueId::new(first_value), requirements.intermediate_values,
				KernelTemplateId::new(first_kernel), requirements.kernels, ), workspace_limit: requirements.workspace_bytes, };
		let materialized = materialize_tree_ensemble_inference(&request)?; for tensor in &materialized.graph.tensors {
			self.insert_tensor_contract(tensor.clone())?; }
		for node in &materialized.graph.nodes { self.nodes.push(node.clone()); self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id, domain: IterationDomain::first(), }); }
		Ok(predictions) }

	pub(crate) fn compile_residual( &mut self, residual: &crate::CheckpointResidualImage,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<ValueId> { let BlockInferenceContext { input, rows, width: input_width, block_index,
			layer_index, normalization_epsilon, tree_lanes, .. } = context;
		self.require_tensor(input, DType::F32, &[rows, input_width], "residual input")?;
		let mut branch = input; let mut branch_width = input_width; let mut retained_layer = false;
		let mut branch_prelu = residual.branch_prelu().iter().enumerate(); for step in residual.branch() { match step {
				crate::CheckpointResidualBranchImage::Layer(layer) => { retained_layer = true; branch = self.compile_layer(
						*layer_index, layer, branch, [rows, branch_width], normalization_epsilon, tree_lanes, )?;
					*layer_index = layer_index.checked_add(1).ok_or_else(identity_exhausted)?;
					branch_width = layer.declaration().width().get(); }
				crate::CheckpointResidualBranchImage::Operation(operation) => {
					let alpha = if matches!(operation, DenseOperation::Activation(activation) if activation.learned_parameters() == 1) {
						let (occurrence, parameter) = branch_prelu.next().ok_or_else(|| { InferenceCompileError::new(
								InferenceCompileErrorKind::InconsistentCheckpoint,
								format!("residual block {block_index} omitted a branch PReLU scalar"),
							) })?; validate_checkpoint_parameter_image( parameter.parameter(), &[1],
							"residual branch PReLU scalar",
						)?; Some(self.external_checkpoint_tensor( InferenceInputRole::ResidualBranchPRelu { block: block_index,
								occurrence, }, parameter.parameter(), )?) } else { None }; branch = self.apply_saved_operation( branch,
						*operation, alpha, [rows, branch_width], normalization_epsilon, tree_lanes, )?; } } }
		if branch_prelu.next().is_some() { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("residual block {block_index} retains an extra branch PReLU scalar"),
			)); }
		let output_width = residual.output_width().get(); if !retained_layer || branch_width != output_width {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("residual block {block_index} output width disagrees with its last branch layer"),
			)); }
		let skip = match residual.skip() {
			crate::CheckpointResidualSkipImage::Identity if input_width == output_width => input,
			crate::CheckpointResidualSkipImage::Projection(projection) if input_width != output_width => {
				validate_checkpoint_parameter_image( projection.parameter(), &[input_width, output_width],
					"residual projection weight",
				)?; let weight = self.external_checkpoint_tensor(
					InferenceInputRole::ResidualProjectionWeight { block: block_index }, projection.parameter(), )?;
				self.bias_free_linear(input, weight, rows, output_width)? }
			crate::CheckpointResidualSkipImage::Identity => { return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("residual block {block_index} width mismatch requires a projection"),
				)); }
			crate::CheckpointResidualSkipImage::Projection(_) => { return Err(InferenceCompileError::new(
					InferenceCompileErrorKind::InconsistentCheckpoint,
					format!("equal-width residual block {block_index} must use an identity skip"),
				)); } }; let mut current = self.exact_add(branch, skip)?;
		let mut output_prelu = residual.prelu().iter().enumerate(); for operation in residual.operations().iter().copied() {
			let alpha = if matches!(operation, DenseOperation::Activation(activation) if activation.learned_parameters() == 1) {
				let (occurrence, parameter) = output_prelu.next().ok_or_else(|| { InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						format!("residual block {block_index} omitted an output PReLU scalar"),
					) })?; validate_checkpoint_parameter_image( parameter.parameter(), &[1],
					"residual output PReLU scalar",
				)?; Some(self.external_checkpoint_tensor( InferenceInputRole::ResidualOutputPRelu { block: block_index, occurrence,
					}, parameter.parameter(), )?) } else { None }; current = self.apply_saved_operation( current, operation, alpha,
				[rows, output_width], normalization_epsilon, tree_lanes, )?; }
		if output_prelu.next().is_some() { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				format!("residual block {block_index} retains an extra output PReLU scalar"),
			)); }
		Ok(current) }

	fn apply_saved_operation( &mut self, input: ValueId, operation: DenseOperation, prelu: Option<ValueId>,
		dimensions: [u64; 2], normalization_epsilon: f32, tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		let [rows, width] = dimensions; match operation { DenseOperation::Activation(activation) => {
				self.apply_activation(input, activation, prelu, shape(&[rows, width])?) }
			DenseOperation::Normalization(normalization) => self.apply_model_normalization( input, normalization, rows, width,
				normalization_epsilon, tree_lanes, ), } }

	fn external_i32( &mut self, role: InferenceInputRole, shape: Shape, values: &[i32],
	) -> InferenceCompileResult<ValueId> { self.external( role, DType::I32, shape, values.iter()
				.flat_map(|value| value.to_le_bytes()) .collect(), ) }

	fn identity_indices(&mut self, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let indices = self.tensor(DType::I32, output_shape)?; self.emit( Vec::new(), vec![indices],
			PrimitiveKind::IndexMap(IndexMap { start: 0, element_step: 1, iteration_step: 0, modulus: None, }), Vec::new(), )?;
		Ok(indices) }

	fn pack_contiguous_f32_to_flat(&mut self, input: ValueId) -> InferenceCompileResult<ValueId> {
		let contract = self.tensor_ref(input)?.clone(); if contract.dtype != DType::F32 {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::Language,
				"attention tensor reinterpretation requires binary32 input",
			)); }
		if contract.shape.rank() == 1 { return Ok(input); }
		let elements = contract.shape.elements();
		checked_i32(elements, "attention tensor element count")?;
		let indices = self.identity_indices(contract.shape.clone())?; let base = self.zero_f32(shape(&[elements])?)?;
		let flat = self.tensor(DType::F32, shape(&[elements])?)?; self.emit( vec![base, indices, input], vec![flat],
			PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices, }),
			forbidden_aliases(3, 1), )?; Ok(flat) }

	fn reinterpret_f32(&mut self, input: ValueId, output_shape: Shape) -> InferenceCompileResult<ValueId> {
		let flat = self.pack_contiguous_f32_to_flat(input)?;
		if self.tensor_ref(flat)?.shape.elements() != output_shape.elements() { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"attention tensor reinterpretation changed its element count",
			)); }
		let indices = self.identity_indices(output_shape.clone())?; let output = self.tensor(DType::F32, output_shape)?;
		self.emit( vec![flat, indices], vec![output], PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }),
			forbidden_aliases(2, 1), )?; Ok(output) }

	fn head_major_to_sequence( &mut self, input: ValueId, rows: u64, sequence: u64, heads: u64, head_dimension: u64,
	) -> InferenceCompileResult<ValueId> { self.require_tensor( input, DType::F32,
			&[rows, heads, sequence, head_dimension],
			"inference head-major attention tensor",
		)?; let output_shape = shape(&[rows, sequence, heads, head_dimension])?;
		let positions = self.identity_indices(output_shape.clone())?;
		let indices = self.tensor(DType::I32, output_shape.clone())?; self.emit_elementwise( vec![positions], vec![indices],
			head_major_source_index_program(sequence, heads, head_dimension)?, )?;
		let flat = self.pack_contiguous_f32_to_flat(input)?; let output = self.tensor(DType::F32, output_shape)?; self.emit(
			vec![flat, indices], vec![output], PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }),
			forbidden_aliases(2, 1), )?; Ok(output) }

	fn causal_softmax( &mut self, scores: ValueId, rows: u64, heads: u64, sequence: u64, tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> {
		let attention_rows = checked_product(&[rows, heads, sequence], "attention softmax row count")?;
		let matrix_shape = shape(&[attention_rows, sequence])?;
		let scores = self.reinterpret_f32(scores, matrix_shape.clone())?;
		let positions = self.identity_indices(matrix_shape.clone())?;
		let mask = self.tensor(DType::I32, matrix_shape.clone())?;
		self.emit_elementwise(vec![positions], vec![mask], causal_mask_program(sequence)?)?;
		let softmax = self.tensor(DType::F32, matrix_shape)?; let parameters = [
			("rows".to_owned(), PreparedParameter::U64(attention_rows)),
			("columns".to_owned(), PreparedParameter::U64(sequence)),
			(
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)), ), (
				"causal_mask_verified".to_owned(),
				PreparedParameter::Bool(true), ), ] .into_iter() .collect::<PreparedParameters>(); self.materialize(
			"gpu_causal_softmax_rows",
			&[("values", scores), ("causal_mask", mask)],
			&[("softmax", softmax)],
			"values",
			&parameters, )?; self.reinterpret_f32(softmax, shape(&[rows, heads, sequence, sequence])?) }

	fn pack_matrix_to_flat(&mut self, input: ValueId, rows: u64, width: u64) -> InferenceCompileResult<ValueId> {
		self.require_tensor(input, DType::F32, &[rows, width], "pool matrix pack input")?;
		let elements = rows.checked_mul(width).ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"pool matrix pack element count overflowed u64",
			) })?; let matrix_indices = self.identity_indices(shape(&[rows, width])?)?;
		let base = self.zero_f32(shape(&[elements])?)?; let flat = self.tensor(DType::F32, shape(&[elements])?)?; self.emit(
			vec![base, matrix_indices, input], vec![flat], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), )?; Ok(flat) }

	fn pack_i32_matrix_to_flat(&mut self, input: ValueId, rows: u64, width: u64) -> InferenceCompileResult<ValueId> {
		self.require_tensor(input, DType::I32, &[rows, width], "int32 matrix pack input")?;
		let elements = rows.checked_mul(width).ok_or_else(|| { InferenceCompileError::new(
				InferenceCompileErrorKind::ArithmeticOverflow,
				"int32 matrix pack element count overflowed u64",
			) })?; let matrix_indices = self.identity_indices(shape(&[rows, width])?)?;
		let base = self.zero_i32(shape(&[elements])?)?; let flat = self.tensor(DType::I32, shape(&[elements])?)?; self.emit(
			vec![base, matrix_indices, input], vec![flat], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), )?; Ok(flat) }

	fn unpack_pool_to_matrix( &mut self, pooled: ValueId, rows: u64, groups: u64, channels: u64, output_width: u64,
	) -> InferenceCompileResult<ValueId> { self.require_tensor( pooled, DType::F32, &[rows, groups, channels],
			"pool grouped output",
		)?; if groups.checked_mul(channels) != Some(output_width) { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"pool grouped output width disagrees with its logical shape",
			)); }
		let group_indices = self.identity_indices(shape(&[groups, channels])?)?;
		let base = self.zero_f32(shape(&[rows, output_width])?)?;
		let output = self.tensor(DType::F32, shape(&[rows, output_width])?)?; self.emit( vec![base, group_indices, pooled],
			vec![output], PrimitiveKind::Scatter(Scatter { axis: 1, bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), )?; Ok(output) }

	fn bias_free_linear( &mut self, input: ValueId, weight: ValueId, rows: u64, output_width: u64,
	) -> InferenceCompileResult<ValueId> { let output = self.tensor(DType::F32, shape(&[rows, output_width])?)?; self.emit(
			vec![input, weight], vec![output], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(1, 0)], }), forbidden_aliases(2, 1), )?; Ok(output) }

	fn exact_add(&mut self, left: ValueId, right: ValueId) -> InferenceCompileResult<ValueId> {
		let left_tensor = self.tensor_ref(left)?; let left_dtype = left_tensor.dtype;
		let left_shape = left_tensor.shape.clone(); let right_tensor = self.tensor_ref(right)?;
		if left_dtype != right_tensor.dtype || left_shape != right_tensor.shape { return Err(InferenceCompileError::new(
				InferenceCompileErrorKind::InconsistentCheckpoint,
				"residual addition requires exactly equal dtypes and shapes",
			)); }
		let output = self.tensor(left_dtype, left_shape)?;
		self.emit_elementwise(vec![left, right], vec![output], add_program()?)?; Ok(output) }

	fn require_tensor( &self, value: ValueId, dtype: DType, extents: &[u64], role: &str, ) -> InferenceCompileResult<()> {
		let tensor = self.tensor_ref(value)?; if tensor.dtype != dtype || tensor.shape.extents() != extents {
			return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
					"{role} has {:?} shape {:?}, expected {dtype:?} shape {extents:?}",
					tensor.dtype, tensor.shape.extents(), ), )); }
		Ok(()) }

	fn apply_activation( &mut self, input: ValueId, activation: DenseActivation, prelu: Option<ValueId>,
		output_shape: Shape, ) -> InferenceCompileResult<ValueId> {
		let activation = activation.forward_kernel::<InferenceCompileError>()?;
		if matches!(activation, ForwardActivation::Identity) { return Ok(input); }
		let output = self.tensor(DType::F32, output_shape.clone())?; match activation {
			ForwardActivation::Identity => unreachable!("linear activation returned its input"),
			ForwardActivation::Owned(symbol) => { self.emit_owned_scalar(symbol, vec![input], vec![output])?; }
			ForwardActivation::Program(program) => { self.emit_elementwise(vec![input], vec![output], program)?; }
			ForwardActivation::SignedMagnitude(magnitude_symbol) => {
				let absolute = self.tensor(DType::F32, output_shape.clone())?;
				self.emit_owned_scalar("gpu_abs_into", vec![input], vec![absolute])?;
				let magnitude = self.tensor(DType::F32, output_shape.clone())?;
				self.emit_owned_scalar(magnitude_symbol, vec![absolute], vec![magnitude])?;
				let sign = self.tensor(DType::F32, output_shape)?;
				self.emit_owned_scalar("gpu_sign_into", vec![input], vec![sign])?;
				self.emit_elementwise(vec![sign, magnitude], vec![output], multiply_program()?)?; }
			ForwardActivation::PRelu(program) => { let alpha = prelu.ok_or_else(|| { InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"PReLU inference omitted its learned scalar",
					) })?; self.emit_elementwise(vec![input, alpha], vec![output], program)?; } }
		Ok(output) }

	fn apply_model_normalization( &mut self, input: ValueId, normalization: DenseNormalization, rows: u64, columns: u64,
		epsilon: f32, tree_lanes: u32, ) -> InferenceCompileResult<ValueId> {
		let (axis, statistic_shape, population, keep_dimensions) = match normalization {
			DenseNormalization::Layer => (1, shape(&[rows, 1])?, columns as f32, true),
			DenseNormalization::Batch => (0, shape(&[columns])?, rows as f32, false), };
		let matrix_shape = shape(&[rows, columns])?; let sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce( input, sums, ReduceOperator::Sum, &[axis], keep_dimensions, tree_lanes, )?;
		let means = self.tensor(DType::F32, statistic_shape.clone())?; self.emit_elementwise( vec![sums], vec![means],
			divide_constant_program(population)?, )?; let centered = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![input, means], vec![centered], subtract_program()?)?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![centered], vec![squares], square_program()?)?;
		let variance_sums = self.tensor(DType::F32, statistic_shape.clone())?; self.reduce( squares, variance_sums,
			ReduceOperator::Sum, &[axis], keep_dimensions, tree_lanes, )?;
		let variance = self.tensor(DType::F32, statistic_shape)?; self.emit_elementwise( vec![variance_sums], vec![variance],
			divide_constant_program(population)?, )?; let normalized = self.tensor(DType::F32, matrix_shape)?;
		self.emit_elementwise( vec![input, means, variance], vec![normalized], z_score_program(epsilon, false)?, )?;
		Ok(normalized) }

	fn apply_temperature( &mut self, logits: ValueId, temperature: &CheckpointTensorImage,
	) -> InferenceCompileResult<ValueId> {
		validate_checkpoint_parameter_image(temperature, &[1], "temperature")?;
		let temperature = self.external_checkpoint_tensor(InferenceInputRole::Temperature, temperature)?;
		let output_shape = self.tensor_ref(logits)?.shape.clone(); let scaled = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(vec![logits, temperature], vec![scaled], divide_program()?)?; Ok(scaled) }

	fn compile_prediction( &mut self, values: ValueId, task: DenseTask, rows: u64, columns: u64, tree_lanes: u32,
	) -> InferenceCompileResult<(ValueId, InferencePredictionKind)> { match task {
			DenseTask::BinaryClassification { .. } => { if columns != 1 { return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"binary inference logits do not have width one",
					)); }
				let probabilities = self.stable_sigmoid(values)?; Ok((probabilities, InferencePredictionKind::BinaryProbability)) }
			DenseTask::MulticlassClassification { class_count, .. } => { if u64::try_from(class_count).ok() != Some(columns) {
					return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
						"multiclass inference logit width differs from saved class count",
					)); }
				let probabilities = self.stable_softmax(values, rows, columns, tree_lanes)?; Ok(( probabilities,
					InferencePredictionKind::MulticlassProbabilities, )) }
			DenseTask::ScalarRegression { .. } => { if columns != 1 { return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"scalar regression output does not have width one",
					)); }
				Ok((values, InferencePredictionKind::Regression)) }
			DenseTask::MultiTargetBinaryClassification { target_count, .. } => {
				if u64::try_from(target_count).ok() != Some(columns) { return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"multi-target binary logit width differs from saved target count",
					)); }
				let probabilities = self.stable_sigmoid(values)?; Ok(( probabilities,
					InferencePredictionKind::MultiTargetBinaryProbabilities, )) }
			DenseTask::JointMulticlassClassification { target_count, .. } => {
				if u64::try_from(target_count).ok() != Some(columns) { return Err(InferenceCompileError::new(
						InferenceCompileErrorKind::InconsistentCheckpoint,
						"joint target logit width differs from saved target count",
					)); }
				let probabilities = self.stable_softmax(values, rows, columns, tree_lanes)?; Ok(( probabilities,
					InferencePredictionKind::JointTargetProbabilities, )) }
			DenseTask::MultiTargetRegression { target_count, .. } => { if u64::try_from(target_count).ok() != Some(columns) {
					return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint,
						"multi-target regression width differs from saved target count",
					)); }
				Ok((values, InferencePredictionKind::MultiTargetRegression)) } } }

	fn stable_sigmoid(&mut self, logits: ValueId) -> InferenceCompileResult<ValueId> {
		let output_shape = self.tensor_ref(logits)?.shape.clone();
		let exponent_argument = self.tensor(DType::F32, output_shape.clone())?; self.emit_elementwise( vec![logits],
			vec![exponent_argument], stable_sigmoid_exponent_program()?, )?;
		let exponent = self.tensor(DType::F32, output_shape.clone())?; self.emit_elementwise( vec![exponent_argument],
			vec![exponent], recipe_math::exp_with_gradual_underflow_program()?, )?;
		let probabilities = self.tensor(DType::F32, output_shape)?; self.emit_elementwise( vec![logits, exponent],
			vec![probabilities], stable_sigmoid_result_program()?, )?; Ok(probabilities) }

	fn stable_softmax( &mut self, logits: ValueId, rows: u64, classes: u64, tree_lanes: u32,
	) -> InferenceCompileResult<ValueId> { let matrix_shape = shape(&[rows, classes])?; let row_shape = shape(&[rows, 1])?;
		let maximum = self.tensor(DType::F32, row_shape.clone())?; self.reduce( logits, maximum, ReduceOperator::Maximum,
			&[1], true, tree_lanes, )?; let shifted = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![logits, maximum], vec![shifted], subtract_program()?)?;
		let exponent_input = self.tensor(DType::F32, matrix_shape.clone())?; self.emit_elementwise( vec![shifted],
			vec![exponent_input], softmax_exponent_input_program()?, )?;
		let exponentials = self.tensor(DType::F32, matrix_shape.clone())?; self.emit_elementwise( vec![exponent_input],
			vec![exponentials], recipe_math::exp_with_gradual_underflow_program()?, )?;
		let exponential_sum = self.tensor(DType::F32, row_shape)?; self.reduce( exponentials, exponential_sum,
			ReduceOperator::Sum, &[1], true, tree_lanes, )?; let probabilities = self.tensor(DType::F32, matrix_shape)?;
		self.emit_elementwise( vec![exponentials, exponential_sum], vec![probabilities], divide_program()?, )?;
		Ok(probabilities) }

	fn finish( mut self, prediction: ValueId, kind: InferencePredictionKind, rows: u64, task: InferenceTask,
		target_dtypes: Vec<DType>, output_adapter: Option<DenseOutputAdapter>,
	) -> InferenceCompileResult<CompiledInference> { let output_tensor = self.tensor_ref(prediction)?.clone();
		for tensor in self.tensors.values_mut() { tensor.external_input = self.external_input_ids.contains(&tensor.id);
			tensor.external_output = tensor.id == prediction; }
		let graph = CalculationGraph { tensors: self.tensors.into_values().collect(), nodes: self.nodes, }; graph.validate()?;
		let canonical = graph.to_ogdl()?; let graph = CalculationGraph::from_ogdl(&canonical)?;
		let iterations = NonZeroU64::new(1).expect("one inference iteration is nonzero");
		let program = StaticCalculationProgram::new(graph, iterations, self.domains)?; let program_text = program.to_ogdl()?;
		let program = StaticCalculationProgram::from_ogdl(&program_text)?; Ok(CompiledInference { program,
			external_inputs: self.external_inputs, output: InferenceOutputContract { value: prediction,
				dtype: output_tensor.dtype, target_dtypes, shape: output_tensor.shape, kind, }, rows, task, output_adapter, }) } }

fn inference_feature_bytes(values: &PreparedInferenceValues) -> (DType, Vec<u8>) { match values {
		PreparedInferenceValues::I32(values) => ( DType::I32, values.iter() .flat_map(|value| value.to_le_bytes()) .collect(),
		), PreparedInferenceValues::F32Bits(values) => ( DType::F32,
			values.iter().flat_map(|bits| bits.to_le_bytes()).collect(), ), } }

fn validate_checkpoint_parameter_image( image: &CheckpointTensorImage, expected_shape: &[u64], role: &str,
) -> InferenceCompileResult<()> { if image.dtype() != DType::F32 || image.shape() != expected_shape {
		return Err(InferenceCompileError::new( InferenceCompileErrorKind::InconsistentCheckpoint, format!(
				"{role} has {:?} shape {:?}, expected F32 shape {expected_shape:?}",
				image.dtype(), image.shape() ), )); }
	let expected_bytes = expected_shape .iter() .try_fold(1u64, |elements, extent| elements.checked_mul(*extent))
		.and_then(|elements| elements.checked_mul(u64::from(DType::F32.byte_width()))) .ok_or_else(|| {
			InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
				format!("{role} byte count overflowed u64"),
			) })?; if u64::try_from(image.bytes().len()).ok() != Some(expected_bytes) { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint, format!(
				"{role} provides {} bytes, expected {expected_bytes}",
				image.bytes().len() ), )); }
	Ok(()) }

fn shape(extents: &[u64]) -> InferenceCompileResult<Shape> { Ok(Shape::new(extents.to_vec())?) }

fn checked_product(values: &[u64], name: &str) -> InferenceCompileResult<u64> {
	values.iter().copied().try_fold(1u64, |product, value| { product.checked_mul(value).ok_or_else(|| {
			InferenceCompileError::new( InferenceCompileErrorKind::ArithmeticOverflow,
				format!("{name} overflowed u64"),
			) }) }) }

fn checked_i32(value: u64, name: &str) -> InferenceCompileResult<i32> { i32::try_from(value).map_err(|error| {
		InferenceCompileError::new( InferenceCompileErrorKind::UnsupportedExtent,
			format!("{name} {value} cannot be represented by int32: {error}"),
		) }) }

fn require_i32_indexable(value: u64, name: &str) -> InferenceCompileResult<()> {
	if value == 0 || value > i32::MAX as u64 { return Err(InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("{name} {value} must fit the nonempty checked int32 index domain"),
		)); }
	Ok(()) }

pub(crate) fn identity_exhausted() -> InferenceCompileError { InferenceCompileError::new(
		InferenceCompileErrorKind::IdentityExhausted,
		"deterministic inference graph identity space exhausted",
	) }

fn zero_f32_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let _seed = builder.input(DType::I32)?; let zero = builder.f32(0.0)?;
	Ok(builder.finish(&[zero])?) }

fn checked_i32_to_f32_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let input = builder.input(DType::I32)?;
	let converted = builder.unary(ScalarOpcode::ConvertI32ToF32, input)?;
	let round_trip = builder.unary(ScalarOpcode::ConvertF32ToI32, converted)?;
	let exact = builder.binary(ScalarOpcode::Equal, input, round_trip)?; let maximum = builder.i32(i32::MAX)?;
	let below_saturating_hole = builder.binary(ScalarOpcode::NotEqual, input, maximum)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, exact, below_saturating_hole)?;
	let _ = builder.unary(ScalarOpcode::Require, valid)?; Ok(builder.finish(&[converted])?) }

fn stable_sigmoid_exponent_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let logit = builder.input(DType::F32)?;
	let magnitude = builder.unary(ScalarOpcode::Absolute, logit)?;
	let exponent_argument = builder.unary(ScalarOpcode::Negate, magnitude)?; Ok(builder.finish(&[exponent_argument])?) }

fn stable_sigmoid_result_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let logit = builder.input(DType::F32)?;
	let exponent = builder.input(DType::F32)?; let zero = builder.f32(0.0)?; let one = builder.f32(1.0)?;
	let denominator = builder.binary(ScalarOpcode::Add, one, exponent)?;
	let positive = builder.binary(ScalarOpcode::Divide, one, denominator)?;
	let negative = builder.binary(ScalarOpcode::Divide, exponent, denominator)?;
	let nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, logit, zero)?;
	let probability = builder.ternary(ScalarOpcode::Select, nonnegative, positive, negative)?;
	Ok(builder.finish(&[probability])?) }

fn softmax_exponent_input_program() -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let shifted = builder.input(DType::F32)?; let zero = builder.f32(0.0)?;
	let nonpositive = builder.binary(ScalarOpcode::LessThanOrEqual, shifted, zero)?;
	let _ = builder.unary(ScalarOpcode::Require, nonpositive)?;
	let finite = builder.unary(ScalarOpcode::IsFinite, shifted)?; let true_underflow = builder.f32(-104.0)?;
	let exponent_input = builder.ternary(ScalarOpcode::Select, finite, shifted, true_underflow)?;
	Ok(builder.finish(&[exponent_input])?) }

fn checked_one_hot_update_program(classes: i32) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let label = builder.input(DType::I32)?;
	let row_base = builder.input(DType::I32)?; let zero = builder.i32(0)?; let classes = builder.i32(classes)?;
	let nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, label, zero)?;
	let below_width = builder.binary(ScalarOpcode::LessThan, label, classes)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, nonnegative, below_width)?;
	let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let destination = builder.binary(ScalarOpcode::Add, row_base, label)?; let one = builder.f32(1.0)?;
	Ok(builder.finish(&[destination, one])?) }

fn feature_destination_program( total_width: i32, start: i32, block_width: i32,
) -> InferenceCompileResult<recipe_core::ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?; let block_width = builder.i32(block_width)?;
	let row = builder.binary(ScalarOpcode::Divide, position, block_width)?;
	let column = builder.binary(ScalarOpcode::Remainder, position, block_width)?;
	let total_width = builder.i32(total_width)?;
	let row_offset = builder.binary(ScalarOpcode::Multiply, row, total_width)?; let start = builder.i32(start)?;
	let destination = builder.binary(ScalarOpcode::Add, row_offset, start)?;
	let destination = builder.binary(ScalarOpcode::Add, destination, column)?; Ok(builder.finish(&[destination])?) }

fn saved_feature_schema(checkpoint: &CheckpointArtifact) -> InferencePreparationResult<Vec<InferenceFeatureSchema>> { saved_feature_schema_from_parts(checkpoint.vectors(), checkpoint.feature_spans()) }

fn saved_feature_schema_from_parts( vectors: &[CheckpointArtifactVector], spans: &[CompiledFeatureSpan],
) -> InferencePreparationResult<Vec<InferenceFeatureSchema>> { spans.iter() .enumerate() .map(|(feature, span)| {
			let vector = vectors .iter() .find(|vector| vector.source_index() == span.source_vector())
				.ok_or_else(|| inconsistent_feature(feature, span, "saved feature vector is absent"))?;
			let encoding = match ( span.lowering(), vector.semantic_type(), vector.encoding(), vector.metadata(), ) { (
					DenseFeatureLowering::NumericScalar, SemanticType::Numeric, VectorEncoding::I32, CheckpointArtifactMetadata::None,
				) => InferenceFeatureEncoding::NumericI32, ( DenseFeatureLowering::NumericScalar, SemanticType::Numeric,
					VectorEncoding::F32, CheckpointArtifactMetadata::None, ) => InferenceFeatureEncoding::NumericF32, (
					DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, }, SemanticType::Categorical,
					VectorEncoding::DictionaryI32, CheckpointArtifactMetadata::Categorical { dictionary },
				) if dictionary_width == dictionary.len() && reserved_index == dictionary.len()
					&& span.width() == dictionary.len().saturating_add(1) =>
				{ InferenceFeatureEncoding::CategoricalDictionary { dictionary: dictionary.clone(), } }
				_ => { return Err(inconsistent_feature( feature, span,
						"saved vector schema and dense lowering are inconsistent",
					)); } }; Ok(InferenceFeatureSchema::new( span.source_vector(), vector.name(), encoding, )) }) .collect() }

fn validate_prepared_feature_spans( prepared: &PreparedInferenceDataset, spans: &[CompiledFeatureSpan],
) -> InferencePreparationResult<()> { if prepared.features().len() != spans.len() {
		return Err(InferencePreparationError::InconsistentCheckpoint { feature: prepared.features().len().min(spans.len()),
			source_vector: spans .get(prepared.features().len().min(spans.len())) .map_or(0, CompiledFeatureSpan::source_vector),
			detail: "prepared feature count differs from the saved span count".to_owned(),
		}); }
	for (feature, (prepared, span)) in prepared.features().iter().zip(spans).enumerate() {
		if prepared.schema().source_vector() != span.source_vector() { return Err(inconsistent_feature( feature, span,
				"prepared feature identity differs from the saved span",
			)); }
		match ( span.lowering(), prepared.schema().encoding(), prepared.values(), ) { ( DenseFeatureLowering::NumericScalar,
				InferenceFeatureEncoding::NumericI32, PreparedInferenceValues::I32(_), )
			| ( DenseFeatureLowering::NumericScalar, InferenceFeatureEncoding::NumericF32, PreparedInferenceValues::F32Bits(_),
			) if span.width() == 1 => {}
			( DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, },
				InferenceFeatureEncoding::CategoricalDictionary { dictionary }, PreparedInferenceValues::I32(_),
			) if dictionary_width == dictionary.len() && reserved_index == dictionary.len()
				&& span.width() == dictionary.len().saturating_add(1) => {}
			_ => { return Err(inconsistent_feature( feature, span,
					"prepared saved-schema values and feature span are inconsistent",
				)); } } }
	Ok(()) }

fn inconsistent_feature( feature: usize, span: &CompiledFeatureSpan, detail: impl Into<String>,
) -> InferencePreparationError { InferencePreparationError::InconsistentCheckpoint { feature,
		source_vector: span.source_vector(), detail: detail.into(), } }
