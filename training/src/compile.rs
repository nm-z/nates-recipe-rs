use core::num::NonZeroU64; use alloc::collections::{BTreeMap, BTreeSet};

use recipe_core::{ AliasPermission, ByteCount, DType, KernelTemplateId, LoopIterations, MetricId, ScalarOpcode, ValueId,
}; use recipe_ingest::{
	DenseMatrix, PreparedDataset, PreparedValues, SemanticType, VectorEncoding, VectorMetadata, VectorRole, };
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather, IndexBounds, IndexMap,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce,
	ReduceOperator, ReduceResult, ScalarExpression, ScalarProgramBuilder, Scatter, ScatterConflict, Shape, Tensor, };
use recipe_ops::{
	BinaryClassificationMetricRequest, ChannelwiseConvolution1dPreparation, ChannelwiseMaxPool1dPreparation,
	IdentityNamespace, KMeansInitializationRequest, KMeansLloydRequest, MaterializationRequest, NamedTensor,
	PreparedParameter, PreparedParameters, RecallAtOutput, TreeEnsembleInferenceRequest, binary_metric_requirements,
	canonical_elu_backward_program, canonical_focal_with_logits_program, canonical_leaky_relu_backward_program,
	canonical_prelu_backward_program, canonical_selu_backward_program, kmeans_initialization_requirements,
	kmeans_lloyd_requirements, lower_scalar, materialize_binary_classification_metrics, materialize_composition,
	materialize_kmeans_initialization, materialize_kmeans_lloyd_step, materialize_tree_ensemble_inference,
	operation_registry, prepare_channelwise_convolution_1d, prepare_channelwise_max_pool_1d,
	tree_ensemble_inference_requirements, };
use recipe_program::{IterationDomain, KernelIterationDomain, MetricEmission, StaticCalculationProgram};

use crate::{
	BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledDatasetSchema, CompiledTraining,
	DataNormalizationState, DenseActivation, DenseAttention, DenseAttentionState, DenseBlock, DenseBlockState,
	DenseConvolution, DenseConvolutionGeometry, DenseConvolutionState, DenseDataNormalization, DenseEmbedding,
	DenseEmbeddingState, DenseGroupToNeuronRouting, DenseGru, DenseGruState, DenseKMeans, DenseKMeansState,
	DenseLayer, DenseLayerState, DenseLoss, DenseLstm, DenseLstmState, DenseNormalization, DenseOperation,
	DensePool, DensePoolState, DenseResidual, DenseResidualOperation, DenseResidualState,
	DenseRnn, DenseRnnState, DenseTask, DenseTrainingConfig, DenseTree, DenseTreeFamily, DenseTreeState,
	ExternalInputRole, LearningRateDecay, MinMaxState, MulticlassMetricOutputs, MulticlassValidationConfig,
	MulticlassValidationOutputs, OptimizerProgressState, OwnedExternalInput, ParameterState, RecallMetricOutput,
	RegressionMetricOutputs, RegressionValidationConfig, RegressionValidationOutputs, TemperatureScalingConfig,
	TemperatureScalingState, TrainingBounds, TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult,
	TrainingHorizon, TrainingMetricBinding, TrainingMetricKind, TrainingOutputs, ValidationMetricFamily,
	ValidationMetricStatus, ValidationUnavailableReason, ZScoreState, model::CompiledTrainingParts, forward::{
		ForwardActivation, GruForwardParameters, GruStepValues, LstmForwardParameters, LstmStepValues,
		RecurrentForwardGraph, RecurrentGateParameters, RnnStepValues, causal_mask_program,
		divide_constant_program, divide_program, forbidden_aliases, head_major_source_index_program,
		l2_norm_program, l2_square_program, lower_gru_sequence, lower_lstm_sequence,
		lower_rnn_sequence, min_max_program, multiply_constant_program, multiply_program, square_program,
		subtract_program, sum_program, z_score_program, },
	model::{DenseFeaturePlan, LoweredDenseDataset, validate_binary_targets}, };

#[path = "blocks/mod.rs"]
mod blocks;

const MATERIALIZATION_RESERVATION: u64 = 64; const WORKSPACE_LIMIT: ByteCount = ByteCount::new(u64::MAX);

#[derive(Clone, Debug)]
struct LayerValues { declaration: DenseLayer, input: ValueId, weight: InitialParameter, forward_weight: ValueId,
	routing_mask: Option<ValueId>, bias: InitialParameter, operations: Vec<OperationValues>, }

#[derive(Clone, Copy, Debug)]
struct EmbeddingValues { declaration: DenseEmbedding, indices: ValueId, table: InitialParameter,
	sequence_length: NonZeroU64, dimensions: NonZeroU64, vocabulary: NonZeroU64, }

#[derive(Clone, Copy, Debug)]
struct AttentionValues { declaration: DenseAttention, input: ValueId, queries: ValueId, keys: ValueId, values: ValueId,
	probabilities: ValueId, context: ValueId, query: InitialParameter, key: InitialParameter, value: InitialParameter,
	output: InitialParameter, sequence_length: NonZeroU64, dimensions: NonZeroU64, heads: NonZeroU64,
	head_dimension: NonZeroU64, }

#[derive(Clone, Copy, Debug)]
struct AttentionForwardParameters { query: ValueId, key: ValueId, value: ValueId, output: ValueId, }

#[derive(Clone, Copy, Debug)]
struct AttentionForwardValues { input: ValueId, queries: ValueId, keys: ValueId, values: ValueId,
	probabilities: ValueId, context: ValueId, }

#[derive(Clone, Debug)]
struct RnnValues { declaration: DenseRnn, steps: Vec<RnnStepValues>, input_weight: InitialParameter,
	recurrent_weight: InitialParameter, bias: InitialParameter, sequence_length: NonZeroU64, width: NonZeroU64, }

#[derive(Clone, Debug)]
struct GruValues { declaration: DenseGru, steps: Vec<GruStepValues>, reset_input_weight: InitialParameter,
	reset_recurrent_weight: InitialParameter, reset_bias: InitialParameter, update_input_weight: InitialParameter,
	update_recurrent_weight: InitialParameter, update_bias: InitialParameter, candidate_input_weight: InitialParameter,
	candidate_recurrent_weight: InitialParameter, candidate_bias: InitialParameter, sequence_length: NonZeroU64,
	width: NonZeroU64, }

#[derive(Clone, Debug)]
struct LstmValues { declaration: DenseLstm, steps: Vec<LstmStepValues>, input_gate_input_weight: InitialParameter,
	input_gate_recurrent_weight: InitialParameter, input_gate_bias: InitialParameter,
	forget_gate_input_weight: InitialParameter, forget_gate_recurrent_weight: InitialParameter,
	forget_gate_bias: InitialParameter, output_gate_input_weight: InitialParameter,
	output_gate_recurrent_weight: InitialParameter, output_gate_bias: InitialParameter,
	candidate_input_weight: InitialParameter, candidate_recurrent_weight: InitialParameter,
	candidate_bias: InitialParameter, sequence_length: NonZeroU64, width: NonZeroU64, }

#[derive(Clone, Copy, Debug)]
struct RecurrentGateBackwardValues { input_weight: ValueId, recurrent_weight: ValueId, bias: ValueId,
	previous_hidden: ValueId, }

#[derive(Clone, Debug)]
struct ConvolutionValues { declaration: DenseConvolution, geometry: DenseConvolutionGeometry,
	preparation: ChannelwiseConvolution1dPreparation, columns: ValueId, output_group_indices: ValueId,
	weight: InitialParameter, bias: InitialParameter, operations: Vec<OperationValues>, }

#[derive(Clone, Debug)]
struct PoolValues { declaration: DensePool, state: DensePoolState, preparation: ChannelwiseMaxPool1dPreparation,
	winners: ValueId, gradient_batch_indices: ValueId, input_matrix_indices: ValueId, output_group_indices: ValueId, }

#[derive(Clone, Copy, Debug)]
struct KMeansValues { declaration: DenseKMeans, input: ValueId, distances: ValueId, state: DenseKMeansState, }

#[derive(Clone, Debug)]
struct TreeValues { declaration: DenseTree, input_width: NonZeroU64, output_width: NonZeroU64,
	internal_nodes_per_tree: NonZeroU64, leaves_per_tree: NonZeroU64, split_features: ValueId, split_thresholds: ValueId,
	leaf_values: InitialParameter, leaf_indices: ValueId, }

#[derive(Clone, Debug)]
struct ResidualValues { declaration: DenseResidual, input: ValueId, branch: Vec<ResidualBranchValues>,
	projection: Option<InitialParameter>, operations: Vec<OperationValues>, }

#[derive(Clone, Debug)]
enum ResidualBranchValues { Layer(LayerValues), Operation(OperationValues), }

#[derive(Clone, Copy, Debug)]
struct OperationValues { operation: DenseOperation, input: ValueId, output: ValueId, variance: Option<ValueId>,
	parameter: Option<InitialParameter>, }

#[derive(Clone, Copy)]
enum InputGradientRequirement { Omit, Required, }

#[derive(Clone, Copy)]
struct BlockBackwardContext { gradient: ValueId, input_gradient: InputGradientRequirement, block_index: usize,
	partition_rows: u64, validity: ValueId, valid_count: ValueId, epsilon: f32, tree_lanes: u32, }

struct BlockBackward { input_gradient: Option<ValueId>, parameters: Vec<ParameterGradient>, }

#[derive(Clone, Copy)]
pub(crate) struct BlockForwardContext<'a> {
	input: ValueId, targets: ValueId, logical: LogicalFeatureShape,
	routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>, target_width: u64, task: DenseTask, partition_rows: u64,
	validity: ValueId, valid_count: ValueId, block_index: usize,
	config: &'a DenseTrainingConfig,
}

pub(crate) struct BlockForward { output: ValueId, logical: LogicalFeatureShape,
	routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>, tape: Box<dyn CompiledBlock>, }

#[derive(Clone, Copy)]
pub(crate) struct BlockValidationContext<'a> {
	input: ValueId, logical: LogicalFeatureShape, routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>, rows: u64,
	block_index: usize,
	config: &'a DenseTrainingConfig,
	domain: IterationDomain, }

pub(crate) struct BlockValidation { output: ValueId, logical: LogicalFeatureShape,
	routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>, }

pub(crate) trait RealizedBlock: core::fmt::Debug { fn clone_box(&self) -> Box<dyn RealizedBlock>;
	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState));
	fn legacy_layer_state(&self) -> Option<&DenseLayerState> { None }
	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation>; fn checkpoint( &self, training: &CompiledTraining, index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock>; }

#[derive(Clone, Debug)]
struct RealizedEmbedding { declaration: DenseEmbedding, state: DenseEmbeddingState, }

#[derive(Clone, Debug)]
struct RealizedAttention { declaration: DenseAttention, state: DenseAttentionState, }

#[derive(Clone, Debug)]
struct RealizedRnn { declaration: DenseRnn, state: DenseRnnState, }

#[derive(Clone, Debug)]
struct RealizedGru { declaration: DenseGru, state: DenseGruState, }

#[derive(Clone, Debug)]
struct RealizedLstm { declaration: DenseLstm, state: DenseLstmState, }

#[derive(Clone, Debug)]
struct RealizedLayer { declaration: DenseLayer, state: DenseLayerState, }

#[derive(Clone, Debug)]
struct RealizedConvolution { declaration: DenseConvolution, state: DenseConvolutionState, }

#[derive(Clone, Copy, Debug)]
struct RealizedPool { declaration: DensePool, state: DensePoolState, }

#[derive(Clone, Copy, Debug)]
struct RealizedKMeans { declaration: DenseKMeans, state: DenseKMeansState, }

#[derive(Clone, Copy, Debug)]
struct RealizedTree { declaration: DenseTree, state: DenseTreeState, }

#[derive(Clone, Debug)]
struct RealizedResidual { declaration: DenseResidual, state: DenseResidualState, }

pub(crate) trait DeclaredBlock: core::fmt::Debug { fn clone_box(&self) -> Box<dyn DeclaredBlock>;
	fn output_width(&self) -> Option<NonZeroU64> { None }
	fn output_operations(&self) -> &[DenseOperation] { &[] }
	fn legacy_layer(&self) -> Option<&DenseLayer> { None }
	fn leading_embedding(&self) -> Option<DenseEmbedding> { None }
	fn leading_recurrent_name(&self) -> Option<&'static str> { None }
	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward>; }

trait CompiledBlock { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward>;

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState>; }

#[derive(Clone, Copy, Debug)]
struct NormalizedValues { normalized: ValueId, variance: ValueId, }

#[derive(Clone, Copy, Debug)]
struct SoftmaxValues { maximum: ValueId, logarithmic_sum: ValueId, exponentials: ValueId, exponential_sum: ValueId, }

#[derive(Clone, Copy, Debug)]
struct InitialParameter { value: ValueId, first_moment: ValueId, second_moment: ValueId, }

#[derive(Clone, Debug)]
struct GradientPair { weight: ValueId, bias: ValueId, prelu: Vec<ValueId>, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LogicalFeatureShape { length: NonZeroU64, channels: NonZeroU64, }

impl LogicalFeatureShape { fn from_width(width: u64) -> TrainingCompileResult<Self> {
		let length = NonZeroU64::new(width).expect("validated logical feature width is nonzero");
		Ok(Self { length, channels: NonZeroU64::MIN, }) }

	fn width(self) -> TrainingCompileResult<u64> { self.length .get() .checked_mul(self.channels.get()) .ok_or_else(|| {
				TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
					"logical feature width overflowed u64",
				) }) }

	fn embedded(self, embedding: DenseEmbedding) -> TrainingCompileResult<Self> { let embedded = Self {
			length: self.length, channels: embedding.dimensions(), }; embedded.width()?; Ok(embedded) }

	fn attended(self, attention: DenseAttention) -> TrainingCompileResult<(Self, NonZeroU64)> {
		let head_dimension = NonZeroU64::new(self.channels.get() / attention.heads().get())
			.expect("nonzero divisible attention head dimension");
		Ok((self, head_dimension)) }

	fn recurrent(self, width: NonZeroU64) -> TrainingCompileResult<Self> { Ok(Self { length: width,
			channels: NonZeroU64::MIN, }) }

	fn pooled(self, pool: DensePool) -> TrainingCompileResult<(Self, DensePoolState)> {
		let output_length = NonZeroU64::new(self.length.get().div_ceil(pool.size().get()))
			.expect("validated pool output is nonzero");
		let state = DensePoolState::new(self.length, self.channels, output_length); Ok(( Self { length: output_length,
				channels: self.channels, }, state, )) }

	fn convolved(self, convolution: &DenseConvolution) -> TrainingCompileResult<(Self, DenseConvolutionGeometry)> {
		let output_length = NonZeroU64::new(self.length.get() - convolution.kernel().get() + 1)
			.expect("validated convolution output length is nonzero");
		let geometry = DenseConvolutionGeometry::new( self.length, self.channels, output_length, convolution.filters(),
			convolution.kernel(), ); Ok(( Self { length: output_length, channels: convolution.filters(), }, geometry, )) } }

type ParameterGradient = ValueId;

struct ParameterUpdates<'a> {
	clipped: &'a [ValueId],
	cursor: usize, learning_rate: ValueId, beta_one_power: ValueId, beta_two_power: ValueId, apply_update: Option<ValueId>,
	config: &'a DenseTrainingConfig,
}

impl<'a> ParameterUpdates<'a> {
	fn new(
		clipped: &'a [ValueId],
		learning_rate: ValueId, beta_one_power: ValueId, beta_two_power: ValueId, apply_update: Option<ValueId>,
		config: &'a DenseTrainingConfig,
	) -> Self { Self { clipped, cursor: 0, learning_rate, beta_one_power, beta_two_power, apply_update, config, } }

	fn take(&mut self) -> ValueId { let gradient = self.clipped[self.cursor]; self.cursor += 1; gradient }

	fn apply( &mut self, compiler: &mut GraphCompiler, initial: InitialParameter,
	) -> TrainingCompileResult<ParameterState> { let gradient = self.take(); compiler.adamw_update(gradient, initial, self)
	} }

#[derive(Clone, Copy, Debug)]
struct ZScoreValues { normalized: ValueId, mean: ValueId, variance: ValueId, }

#[derive(Clone, Copy, Debug)]
struct MinMaxValues { normalized: ValueId, minimum: ValueId, maximum: ValueId, }

#[derive(Clone, Copy, Debug)]
struct ValidationValues { features: ValueId, targets: ValueId, rows: u64, }

#[derive(Clone, Copy, Debug)]
struct AcceptedUpdatePlan { per_epoch: u64, maximum: Option<u64>, warmup: u64, counter_limit: u64, }

#[inline]
pub fn compile_dense_training( dataset: &PreparedDataset, config: &DenseTrainingConfig,
) -> TrainingCompileResult<CompiledTraining> { let blocks = flat_dense_blocks(config);
	compile_dense_training_impl(dataset, config, &blocks, None, None, None) }

#[inline]
pub fn compile_dense_training_with_blocks( dataset: &PreparedDataset, config: &DenseTrainingConfig,
	blocks: &[DenseBlock], ) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, None, None, None) }

#[inline]
pub fn compile_dense_training_with_blocks_and_binary_validation( dataset: &PreparedDataset,
	config: &DenseTrainingConfig, blocks: &[DenseBlock], validation: &BinaryValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, Some(validation), None, None) }

#[inline]
pub fn compile_dense_training_with_blocks_and_multiclass_validation( dataset: &PreparedDataset,
	config: &DenseTrainingConfig, blocks: &[DenseBlock], validation: &MulticlassValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, None, Some(validation), None) }

#[inline]
pub fn compile_dense_training_with_blocks_and_regression_validation( dataset: &PreparedDataset,
	config: &DenseTrainingConfig, blocks: &[DenseBlock], validation: &RegressionValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, None, None, Some(validation)) }

#[inline]
pub fn compile_dense_training_with_binary_validation( dataset: &PreparedDataset, config: &DenseTrainingConfig,
	validation: &BinaryValidationConfig, ) -> TrainingCompileResult<CompiledTraining> {
	let blocks = flat_dense_blocks(config);
	compile_dense_training_impl(dataset, config, &blocks, Some(validation), None, None) }

#[inline]
pub fn compile_dense_training_with_multiclass_validation( dataset: &PreparedDataset, config: &DenseTrainingConfig,
	validation: &MulticlassValidationConfig, ) -> TrainingCompileResult<CompiledTraining> {
	let blocks = flat_dense_blocks(config);
	compile_dense_training_impl(dataset, config, &blocks, None, Some(validation), None) }

#[inline]
pub fn compile_dense_training_with_regression_validation( dataset: &PreparedDataset, config: &DenseTrainingConfig,
	validation: &RegressionValidationConfig, ) -> TrainingCompileResult<CompiledTraining> {
	let blocks = flat_dense_blocks(config);
	compile_dense_training_impl(dataset, config, &blocks, None, None, Some(validation)) }

fn flat_dense_blocks(config: &DenseTrainingConfig) -> Vec<DenseBlock> { config.layers .iter() .cloned()
		.map(DenseBlock::from) .collect() }

fn compile_dense_training_impl( prepared: &PreparedDataset, config: &DenseTrainingConfig,
	declared_blocks: &[DenseBlock], validation_config: Option<&BinaryValidationConfig>,
	multiclass_validation_config: Option<&MulticlassValidationConfig>,
	regression_validation_config: Option<&RegressionValidationConfig>, ) -> TrainingCompileResult<CompiledTraining> {
	let task = resolve_dense_task(prepared, config.loss)?; let embedding = declared_blocks .first()
		.and_then(|block| block.declaration().leading_embedding()); let recurrent_name = declared_blocks .first()
		.and_then(|block| block.declaration().leading_recurrent_name());
	let feature_plan = DenseFeaturePlan::from_prepared(prepared)?;
	let dataset = LoweredDenseDataset::from_prepared(prepared, &feature_plan, task)?;
	validate_lowered_dataset(&dataset, task, embedding.is_none())?; if let Some(embedding) = embedding {
		validate_embedding_dataset(prepared, &dataset, embedding)?; }
	if let Some(name) = recurrent_name { validate_recurrent_dataset(prepared, name)?; }
	let blocks = declared_blocks.to_vec(); let output_adapter = None; let layers = blocks .iter()
		.map(|block| block.declaration().legacy_layer().cloned()) .collect::<Option<Vec<_>>>() .unwrap_or_default();
	let dataset_schema = CompiledDatasetSchema::from_prepared(prepared, task, feature_plan.spans().to_vec())?;
	let validation_status = validate_validation_config( &dataset, validation_config, multiclass_validation_config,
		regression_validation_config, )?;
	let validation_available = matches!(validation_status, ValidationMetricStatus::Available { .. });
	let retain_validation_inputs = !matches!( validation_status, ValidationMetricStatus::Unavailable { .. } );
	let calibration_iterations = if validation_available { validation_config
			.and_then(BinaryValidationConfig::temperature_scaling) .map_or(0, |calibration| calibration.iterations.get())
	} else { 0 }; let bounds = training_bounds( dataset.train().rows(), config.epochs, config.warmup_epochs,
		calibration_iterations, )?; let masked_targets = !dataset.train().all_targets_known();
	let accepted_update_plan = masked_targets .then(|| { accepted_update_plan( if masked_targets {
					dataset.train().accepted_updates_per_epoch() } else { 1 }, config, &bounds, ) }) .transpose()?;
	let mut compiler = GraphCompiler::new(bounds.iterations, bounds.training_iterations)?;

	let train_features = compiler.external_matrix(ExternalInputRole::TrainFeatures, dataset.train().features())?;
	let train_targets = compiler.external_matrix(ExternalInputRole::TrainTargets, dataset.train().targets())?;
	let train_target_supervision = masked_targets .then(|| dataset.train().target_supervision()) .as_ref()
		.map(|supervision| compiler.external_matrix(ExternalInputRole::TrainTargetSupervision, supervision)) .transpose()?;
	let validation_inputs = retain_validation_inputs .then(|| dataset.validation()) .flatten()
		.map(|validation| -> TrainingCompileResult<_> { Ok((
				compiler.external_matrix(ExternalInputRole::ValidationFeatures, validation.features())?,
				compiler.external_matrix(ExternalInputRole::ValidationTargets, validation.targets())?,
				u64::try_from(validation.rows()).map_err(|error| { TrainingCompileError::new(
						TrainingCompileErrorKind::UnsupportedExtent,
						format!("validation rows cannot be represented by u64: {error}"),
					) })?, )) }) .transpose()?; let feature_normalization_mask = feature_plan .normalization_mask()
		.map(|mask| compiler.external_f32_vector(ExternalInputRole::FeatureNormalizationMask, mask)) .transpose()?;

	let feature_columns = u64::try_from(dataset.train().feature_columns()).map_err(|error| { TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("feature width cannot be represented by u64: {error}"),
		) })?; let target_code_columns = u64::try_from(dataset.train().targets().columns()).map_err(|error| {
		TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
			format!("target code width cannot be represented by u64: {error}"),
		) })?; let output_columns = u64::try_from(task.output_width()).map_err(|error| { TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("task output width cannot be represented by u64: {error}"),
		) })?; let partition_rows = bounds.train_rows; let full_feature_shape = shape(&[partition_rows, feature_columns])?;
	let full_target_shape = shape(&[partition_rows, target_code_columns])?;
	let partition_output_shape = shape(&[partition_rows, output_columns])?;
	let partition_row_shape = shape(&[partition_rows, 1])?; let scalar_shape = shape(&[1])?;

	let converted_features = if embedding.is_some() { train_features } else { compiler.convert_matrix_if_i32(
			train_features, full_feature_shape.clone(), IterationDomain::first(), )? }; let prepared_targets = match task {
		DenseTask::MulticlassClassification { .. } => train_targets, DenseTask::BinaryClassification { .. }
		| DenseTask::ScalarRegression { .. }
		| DenseTask::MultiTargetBinaryClassification { .. }
		| DenseTask::JointMulticlassClassification { .. }
		| DenseTask::MultiTargetRegression { .. } => {
			compiler.convert_matrix_if_i32(train_targets, full_target_shape, IterationDomain::first())? } };
	let validation_features = validation_inputs .as_ref() .map(|(features, _, rows)| -> TrainingCompileResult<_> {
			let feature_shape = shape(&[*rows, feature_columns])?; let features = if embedding.is_some() { *features } else {
				compiler.convert_matrix_if_i32(*features, feature_shape.clone(), IterationDomain::first())? };
			Ok((features, feature_shape)) }) .transpose()?;
	let (normalized_features, normalization, normalized_validation_features) = match config.data_normalization {
		DenseDataNormalization::Identity => ( converted_features, DataNormalizationState::Identity,
			validation_features.map(|(features, _)| features), ), DenseDataNormalization::ZScore => {
			let training = compiler.z_score( converted_features, full_feature_shape, [feature_columns, partition_rows],
				config.normalization_epsilon, config.reduction_tree_lanes, feature_normalization_mask, )?;
			let validation = validation_features .map(|(features, feature_shape)| { compiler.apply_z_score(
						[features, training.mean, training.variance], feature_shape, config.normalization_epsilon,
						IterationDomain::first(), feature_normalization_mask, ) }) .transpose()?; ( training.normalized,
				DataNormalizationState::ZScore(ZScoreState { mean: training.mean, variance: training.variance, }), validation, ) }
		DenseDataNormalization::MinMax => { let training = compiler.min_max( converted_features, full_feature_shape,
				feature_columns, config.normalization_epsilon, config.reduction_tree_lanes, feature_normalization_mask, )?;
			let validation = validation_features .map(|(features, feature_shape)| { compiler.apply_min_max(
						[features, training.minimum, training.maximum], feature_shape, config.normalization_epsilon,
						IterationDomain::first(), feature_normalization_mask, ) }) .transpose()?; ( training.normalized,
				DataNormalizationState::MinMax(MinMaxState { minimum: training.minimum, maximum: training.maximum, }), validation, )
		}
		DenseDataNormalization::L2Norm => { let training = compiler.l2_norm( converted_features, full_feature_shape,
				config.normalization_epsilon, config.reduction_tree_lanes, IterationDomain::first(), feature_normalization_mask, )?;
			let validation = validation_features .map(|(features, feature_shape)| { compiler.l2_norm( features, feature_shape,
						config.normalization_epsilon, config.reduction_tree_lanes, IterationDomain::first(), feature_normalization_mask, )
				}) .transpose()?; (training, DataNormalizationState::L2Norm, validation) } };
	let validation_values = match (validation_inputs, normalized_validation_features) {
		(Some((_, targets, rows)), Some(features)) => { let target_shape = shape(&[rows, target_code_columns])?;
			let targets = match task { DenseTask::MulticlassClassification { .. } => targets,
				DenseTask::BinaryClassification { .. }
				| DenseTask::ScalarRegression { .. }
				| DenseTask::MultiTargetBinaryClassification { .. }
				| DenseTask::JointMulticlassClassification { .. }
				| DenseTask::MultiTargetRegression { .. } => {
					compiler.convert_matrix_if_i32(targets, target_shape, IterationDomain::first())? } }; Some(ValidationValues {
				features, targets, rows, }) }
		(None, None) => None, _ => { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix,
				"validation feature normalization did not preserve the validation partition",
			)); } };

	let mask_index = compiler.index_map( partition_row_shape.clone(), IndexMap { start: 0, element_step: 1,
			iteration_step: 0, modulus: None, }, IterationDomain::first(), )?;
	let validity = compiler.tensor(DType::F32, partition_row_shape.clone())?; compiler.emit_elementwise( vec![mask_index],
		vec![validity],
		validity_program(checked_i32(partition_rows, "training partition rows")?)?,
		IterationDomain::first(), )?; let valid_count = compiler.tensor(DType::F32, scalar_shape.clone())?;
	compiler.reduce_sum( validity, valid_count, &[0, 1], config.reduction_tree_lanes, IterationDomain::first(), )?;
	let (supervision, supervised_count, update_signal) = if let Some(train_target_supervision) = train_target_supervision {
			let known_count = compiler.tensor(DType::F32, scalar_shape.clone())?; compiler.reduce_sum( train_target_supervision,
				known_count, &[0, 1], config.reduction_tree_lanes, IterationDomain::first(), )?;
			let safe_known_count = compiler.tensor(DType::F32, scalar_shape.clone())?; compiler.emit_elementwise(
				vec![known_count], vec![safe_known_count], safe_count_program()?, IterationDomain::first(), )?; (
				train_target_supervision, safe_known_count, Some(known_count), ) } else { ( validity, valid_count,
				accepted_update_plan.map(|_| valid_count), ) };

	let partition_features = normalized_features; let partition_targets = prepared_targets;
	let objective_denominator = if task.uses_target_matrix() && config.loss != DenseLoss::CrossEntropy {
		let denominator = compiler.tensor(DType::F32, scalar_shape.clone())?; compiler.emit_elementwise(
			vec![supervised_count], vec![denominator], multiply_constant_program(output_columns as f32)?,
			compiler.training_domain, )?; denominator } else { supervised_count };

	let (current, _, block_values) = compiler.compile_training_blocks(&blocks, BlockForwardContext {
		input: partition_features, targets: partition_targets, logical: LogicalFeatureShape::from_width(feature_columns)?,
		routing: None, target_width: output_columns, task, partition_rows, validity: supervision,
		valid_count: supervised_count, block_index: 0, config, })?;
	let safe_current = compiler.mask_f32_with_zero(current, supervision, compiler.training_domain)?;
	let safe_partition_targets = if matches!(task, DenseTask::MulticlassClassification { .. }) {
		compiler.mask_i32_with_zero(partition_targets, supervision, compiler.training_domain)? } else {
		compiler.mask_f32_with_zero(partition_targets, supervision, compiler.training_domain)? };

	let losses = compiler.tensor(DType::F32, partition_output_shape.clone())?;
	let loss_gradient = compiler.tensor(DType::F32, partition_output_shape.clone())?; match config.loss {
		DenseLoss::BinaryCrossEntropy => { compiler.materialize(
				"gpu_bce_with_logits",
				&[
					("logits", safe_current),
					("targets", safe_partition_targets),
				],
				&[("losses", losses), ("gradients", loss_gradient)],
				"logits",
				&PreparedParameters::new(), compiler.training_domain, )?; }
		DenseLoss::Focal => { compiler.emit_elementwise( vec![safe_current, safe_partition_targets],
				vec![losses, loss_gradient], canonical_focal_with_logits_program()?, compiler.training_domain, )?; }
		DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber => { compiler.emit_elementwise(
				vec![safe_current, safe_partition_targets], vec![losses, loss_gradient], pointwise_loss_program(config.loss)?,
				compiler.training_domain, )?; }
		DenseLoss::CrossEntropy => { if matches!(task, DenseTask::JointMulticlassClassification { .. }) {
				compiler.cross_entropy_with_dense_targets( safe_current, safe_partition_targets, [losses, loss_gradient],
					[partition_rows, output_columns], config.reduction_tree_lanes, compiler.training_domain, )?; } else {
				compiler.cross_entropy_with_logits( safe_current, safe_partition_targets, [losses, loss_gradient],
					[partition_rows, output_columns], config.reduction_tree_lanes, compiler.training_domain, )?; } } }
	let normalized_gradient = compiler.tensor(DType::F32, partition_output_shape.clone())?; compiler.emit_elementwise(
		vec![loss_gradient, supervision, objective_denominator], vec![normalized_gradient], masked_mean_program()?,
		compiler.training_domain, )?; let masked_losses = compiler.tensor(DType::F32, partition_output_shape)?;
	compiler.emit_elementwise( vec![losses, supervision], vec![masked_losses], masked_zero_f32_program()?,
		compiler.training_domain, )?; let loss_sum = compiler.tensor(DType::F32, scalar_shape.clone())?; compiler.reduce_sum(
		masked_losses, loss_sum, &[0, 1], config.reduction_tree_lanes, compiler.training_domain, )?;
	let training_loss = compiler.tensor(DType::F32, scalar_shape.clone())?; compiler.emit_elementwise(
		vec![loss_sum, objective_denominator], vec![training_loss], divide_program()?, compiler.training_domain, )?;
	let parameter_gradients = compiler.backward_blocks( &block_values, BlockBackwardContext {
			gradient: normalized_gradient, input_gradient: InputGradientRequirement::Omit, block_index: 0, partition_rows,
			validity: supervision, valid_count: supervised_count, epsilon: config.normalization_epsilon,
			tree_lanes: config.reduction_tree_lanes, }, )?; let flattened_gradients = parameter_gradients.clone();
	let optimizer_gradients = match config.gradient_clip_norm { Some(maximum_norm) => compiler.global_clip(
			&flattened_gradients, maximum_norm, config.reduction_tree_lanes, )?, None => flattened_gradients, };

	let (learning_rate, beta_one_power, beta_two_power, optimizer_progress) =
		compiler.dynamic_adam_scalars(config, &bounds, update_signal.zip(accepted_update_plan))?;
	let updates = ParameterUpdates::new( &optimizer_gradients, learning_rate, beta_one_power, beta_two_power,
		optimizer_progress.map(|progress| progress.apply_update), config, ); let block_states = compiler.update_blocks(
		&block_values, updates, )?; let layer_states = block_states .iter()
		.map(|state| state.realized().legacy_layer_state().cloned()) .collect::<Option<Vec<_>>>() .unwrap_or_default();
	let validation = match (validation_config, validation_values) {
		(Some(validation_config), Some(validation_values)) => Some(compiler.compile_validation( validation_values, config,
			validation_config, &block_states, &bounds, )?), (Some(_), None)
			if matches!( validation_status, ValidationMetricStatus::Unavailable { .. } ) =>
		{ None }
		(Some(_), None) => { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
				"validation was requested without validation values",
			)); }
		(None, _) => None, }; let multiclass_validation = match (multiclass_validation_config, validation_values) {
		(Some(_), Some(validation_values)) => Some(compiler.compile_multiclass_validation( validation_values, config,
			&block_states, &bounds, task, )?), (Some(_), None)
			if matches!( validation_status, ValidationMetricStatus::Unavailable { .. } ) =>
		{ None }
		(Some(_), None) => { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
				"multiclass validation was requested without validation values",
			)); }
		(None, _) => None, }; let regression_validation = match (regression_validation_config, validation_values) {
		(Some(_), Some(validation_values)) => {
			Some(compiler.compile_regression_validation(validation_values, config, &block_states, &bounds)?) }
		(Some(_), None) if matches!( validation_status, ValidationMetricStatus::Unavailable { .. } ) =>
		{ None }
		(Some(_), None) => { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
				"R2 validation was requested without validation values",
			)); }
		(None, _) => None, };

	let training_loss_domain = compiler.training_domain; let metric_bindings = training_metric_bindings( training_loss,
		training_loss_domain, learning_rate, validation.as_ref(), multiclass_validation.as_ref(),
		regression_validation.as_ref(), )?; match normalization { DataNormalizationState::Identity => {}
		DataNormalizationState::ZScore(state) => { compiler .external_outputs .extend([state.mean, state.variance]); }
		DataNormalizationState::MinMax(state) => { compiler .external_outputs .extend([state.minimum, state.maximum]); }
		DataNormalizationState::L2Norm => {} }
	let outputs = TrainingOutputs { training_loss, training_loss_domain, normalization, optimizer_progress,
		blocks: block_states, layers: layer_states, validation, multiclass_validation, regression_validation,
		validation_status, metric_bindings, }; for tensor in compiler.tensors.values_mut() {
		tensor.external_input = compiler.external_input_ids.contains(&tensor.id);
		tensor.external_output = compiler.external_outputs.contains(&tensor.id); }
	let metrics = outputs .metric_bindings .iter() .map(|binding| MetricEmission { metric: binding.metric,
			value: binding.value, domain: binding.domain, }) .collect(); let graph = CalculationGraph {
		tensors: compiler.tensors.into_values().collect(), nodes: compiler.nodes, }; graph.validate()?;
	let canonical = graph.to_ogdl()?; let graph = CalculationGraph::from_ogdl(&canonical)?;
	let program = StaticCalculationProgram::new_with_metrics( graph, compiler.iterations, compiler.domains, metrics, )?;
	let program_text = program.to_ogdl()?; let program = StaticCalculationProgram::from_ogdl(&program_text)?;
	Ok(CompiledTraining::from(CompiledTrainingParts { program, external_inputs: compiler.external_inputs, bounds, outputs,
		dataset_schema, config: config.clone(), blocks, layers, output_adapter, })) }

fn validate_recurrent_dataset(dataset: &PreparedDataset, name: &str) -> TrainingCompileResult<()> {
	for feature in dataset .vectors() .iter() .filter(|vector| vector.role() == VectorRole::Feature)
	{ if feature.semantic_type() != SemanticType::Numeric || !matches!( feature.encoding(),
				VectorEncoding::I32 | VectorEncoding::F32 ) || !matches!(feature.metadata(), VectorMetadata::None)
		{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix, format!(
					"the first {name} case requires one numeric scalar per feature-column time step; feature {:?} is {:?}/{:?}",
					String::from_utf8_lossy(feature.name()), feature.semantic_type(), feature.encoding(), ), )); } }
	Ok(()) }

fn resolve_dense_task(dataset: &PreparedDataset, loss: DenseLoss) -> TrainingCompileResult<DenseTask> {
	let targets = dataset .target_source_indices() .iter() .copied() .map(|source_index| { dataset .vectors() .iter()
				.find(|vector| vector.source_index() == source_index && vector.role() == VectorRole::Target) .ok_or_else(|| {
					TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
						format!("declared dense target vector {source_index} is absent"),
					) }) }) .collect::<TrainingCompileResult<Vec<_>>>()?; if targets.is_empty() { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"dense training requires at least one typed target vector",
		)); }
	if targets.len() > 1 { for target in &targets { if target.semantic_type() != SemanticType::Numeric
				|| !matches!(target.encoding(), VectorEncoding::I32 | VectorEncoding::F32)
			{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix, format!(
						"multi-target objective requires homogeneous numeric target meaning; target {:?} is {:?}/{:?}",
						String::from_utf8_lossy(target.name()), target.semantic_type(), target.encoding() ), )); } }
		let first_target_vector = targets[0].source_index(); let target_count = targets.len(); return Ok(match loss {
			DenseLoss::BinaryCrossEntropy | DenseLoss::Focal => DenseTask::MultiTargetBinaryClassification { first_target_vector,
				target_count, }, DenseLoss::CrossEntropy => DenseTask::JointMulticlassClassification { first_target_vector,
				target_count, }, DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber => {
				DenseTask::MultiTargetRegression { first_target_vector, target_count, } } }); }
	let target = targets[0];
	let classified = format!("{:?}/{:?}", target.semantic_type(), target.encoding());
	match loss { DenseLoss::BinaryCrossEntropy | DenseLoss::Focal => {
			let positive_code = match (target.semantic_type(), target.encoding(), target.metadata()) {
				(SemanticType::Numeric, VectorEncoding::I32 | VectorEncoding::F32, _) => Some(1), ( SemanticType::Categorical,
					VectorEncoding::DictionaryI32, recipe_ingest::VectorMetadata::Categorical { dictionary },
				) if dictionary.len() <= 2 => Some(match dictionary.len() { 0 => -1, 1 => 0, 2 => 1, _ => unreachable!(), }),
				_ => None, }; let Some(positive_code) = positive_code else { return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix, format!(
						"target {:?} classified {classified} is not an explicit numeric 0/1 or at-most-two-category binary target",
						String::from_utf8_lossy(target.name()) ), )); }; Ok(DenseTask::BinaryClassification {
				target_vector: target.source_index(), positive_code, }) }
		DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber => {
			if target.semantic_type() != SemanticType::Numeric
				|| !matches!(target.encoding(), VectorEncoding::I32 | VectorEncoding::F32)
			{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix, format!(
						"target {:?} classified {classified} is incompatible with scalar regression",
						String::from_utf8_lossy(target.name()) ), )); }
			Ok(DenseTask::ScalarRegression { target_vector: target.source_index(), }) }
		DenseLoss::CrossEntropy => { let ( SemanticType::Categorical, VectorEncoding::DictionaryI32,
				recipe_ingest::VectorMetadata::Categorical { dictionary },
			) = (target.semantic_type(), target.encoding(), target.metadata())
			else { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix, format!(
						"target {:?} classified {classified} is incompatible with categorical cross entropy",
						String::from_utf8_lossy(target.name()) ), )); }; let class_count = dictionary.len().checked_add(1).ok_or_else(|| {
				TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
					"categorical class count overflowed while adding the reserved unseen-label route",
				) })?; let reserved_code = i32::try_from(dictionary.len()).map_err(|error| { TrainingCompileError::new(
					TrainingCompileErrorKind::UnsupportedExtent,
					format!("categorical reserved code cannot be represented by i32: {error}"),
				) })?; Ok(DenseTask::MulticlassClassification { target_vector: target.source_index(), class_count, reserved_code, })
		} } }

fn validate_lowered_dataset( dataset: &LoweredDenseDataset, task: DenseTask, require_exact_f32_features: bool,
) -> TrainingCompileResult<()> { let expected_target_dtype = match dataset.train().targets() {
		DenseMatrix::I32 { .. } => DType::I32, DenseMatrix::F32Bits { .. } => DType::F32, };
	for partition in core::iter::once(dataset.train()).chain(dataset.validation()) {
		if partition.targets().columns() != task.target_count() { return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix, format!(
					"dense task requires {} target-code columns, got {}",
					task.target_count(), partition.targets().columns(), ), )); }
		if partition.targets().dtype() != expected_target_dtype { return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix, format!(
					"target dtype changed between prepared partitions: training is {expected_target_dtype:?}, partition is {:?}",
					partition.targets().dtype() ), )); }
		if require_exact_f32_features {
			validate_exact_i32_to_f32(partition.features(), "feature")?;
		}
		match task { DenseTask::BinaryClassification { .. } => {
				validate_exact_i32_to_f32(partition.targets(), "target")?;
				validate_binary_targets(partition.targets())?; }
			DenseTask::ScalarRegression { .. } => {
				validate_exact_i32_to_f32(partition.targets(), "target")?;
			}
			DenseTask::MulticlassClassification { .. } => { if !matches!(partition.targets(), DenseMatrix::I32 { .. }) {
					return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
						"multiclass categorical targets must retain dictionary int32 codes",
					)); } }
			DenseTask::MultiTargetBinaryClassification { .. } => {
				if !matches!(partition.targets(), DenseMatrix::F32Bits { .. }) { return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						"multi-target binary targets must lower to one binary32 matrix",
					)); }
				validate_binary_targets(partition.targets())?; }
			DenseTask::JointMulticlassClassification { .. } | DenseTask::MultiTargetRegression { .. } => {
				if !matches!(partition.targets(), DenseMatrix::F32Bits { .. }) { return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						"multi-target values must lower to one binary32 matrix",
					)); } } } }
	Ok(()) }

fn validate_embedding_dataset( prepared: &PreparedDataset, dataset: &LoweredDenseDataset, embedding: DenseEmbedding,
) -> TrainingCompileResult<()> { if embedding.vocabulary().get() > i32::MAX as u64 {
		return Err(TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
			"embedding vocabulary exceeds Recipe's checked int32 token-index domain",
		)); }
	let features = prepared .vectors() .iter() .filter(|vector| vector.role() == VectorRole::Feature) .collect::<Vec<_>>();
	if features.is_empty() { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix,
			"embedding requires at least one fixed token position",
		)); }
	for feature in features { if !matches!( ( feature.semantic_type(), feature.encoding(), feature.metadata(),
				feature.values(), ), ( SemanticType::Numeric, VectorEncoding::I32, VectorMetadata::None, PreparedValues::I32(_), )
		) { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix, format!(
					"embedding token position {:?} must be an exact numeric int32 feature",
					String::from_utf8_lossy(feature.name()) ), )); } }
	let vocabulary = i32::try_from(embedding.vocabulary().get()).map_err(|error| { TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("embedding vocabulary cannot be represented by int32: {error}"),
		) })?; for (partition_name, partition) in [
		("training", dataset.train()),
		(
			"validation",
			dataset.validation().unwrap_or(dataset.train()), ), ] {
		let DenseMatrix::I32 { values, .. } = partition.features() else { return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!("{partition_name} embedding tokens did not retain int32 storage"),
			)); }; if let Some((position, token)) = values .iter() .copied() .enumerate()
			.find(|(_, token)| *token < 0 || *token >= vocabulary)
		{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix, format!(
					"{partition_name} embedding token {token} at flattened position {position} is outside 0..{vocabulary}"
				), )); }
		if dataset.validation().is_none() { break; } }
	Ok(()) }

fn validate_exact_i32_to_f32(matrix: &DenseMatrix, role: &str) -> TrainingCompileResult<()> {
	let DenseMatrix::I32 { values, .. } = matrix else { return Ok(()); }; if let Some(value) = values .iter() .copied()
		.find(|value| f64::from(*value as f32) != f64::from(*value))
	{ return Err(TrainingCompileError::new(
			if role == "feature" {
				TrainingCompileErrorKind::InvalidFeatureMatrix } else { TrainingCompileErrorKind::InvalidTargetMatrix },
			format!("{role} int32 value {value} cannot be represented exactly by dense f32 calculation"),
		)); }
	Ok(()) }

fn push_layer_gradients(flattened: &mut Vec<ParameterGradient>, gradient: &GradientPair) { flattened.extend([
		gradient.weight, gradient.bias, ]); flattened.extend(gradient.prelu.iter().copied()); }

fn validation_prelu_parameter<'a>(
	operation: DenseOperation,
	states: &mut impl Iterator<Item = &'a ParameterState>,
) -> Option<ValueId> { match operation {
		DenseOperation::Activation(activation) if activation.learned_parameters() == 1 => Some(states .next()
			.expect("realized PReLU state")
			.updated_parameter), _ => None, } }

fn validate_validation_config( dataset: &LoweredDenseDataset, binary: Option<&BinaryValidationConfig>,
	multiclass: Option<&MulticlassValidationConfig>, regression: Option<&RegressionValidationConfig>,
) -> TrainingCompileResult<ValidationMetricStatus> { if regression.is_some() { if dataset.validation_split_rows() == 0 {
			return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
				"R2 validation requires a prepared validation partition",
			)); }
		return validation_metric_status(dataset, ValidationMetricFamily::Regression); }
	if multiclass.is_some() { if dataset.validation_split_rows() == 0 { return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"multiclass validation metrics require a prepared validation partition",
			)); }
		return validation_metric_status(dataset, ValidationMetricFamily::Multiclass); }
	let Some(config) = binary else { return Ok(ValidationMetricStatus::NotRequested); };
	if dataset.validation_split_rows() == 0 { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"validation metrics require a prepared validation partition",
		)); }
	let thresholds = config.recall_thresholds().collect::<Vec<_>>(); if let Some((index, threshold)) = thresholds .iter()
		.copied() .enumerate() .find(|(_, threshold)| !threshold.is_finite() || !(0.0..=1.0).contains(threshold))
	{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidOptimizer,
			format!("validation recall threshold {index} ({threshold}) must be finite and in [0, 1]"),
		)); }
	let distinct = thresholds .iter() .map(|threshold| threshold.to_bits()) .collect::<BTreeSet<_>>();
	if distinct.len() != thresholds.len() { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"validation recall thresholds must have distinct f32 bit patterns",
		)); }
	if let Some(validation) = dataset.validation() { binary_metric_requirements(
			u64::try_from(validation.rows()).map_err(|error| { TrainingCompileError::new(
					TrainingCompileErrorKind::UnsupportedExtent,
					format!("validation rows cannot be represented by u64: {error}"),
				) })?, thresholds.len(), config.calibration_bins().get(), )?; }
	if let Some(temperature) = config.temperature_scaling() {
		if !temperature.learning_rate.is_finite() || temperature.learning_rate <= 0.0 { return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"temperature-scaling learning rate must be finite and positive",
			)); }
		if !temperature.minimum_temperature.is_finite() || !temperature.maximum_temperature.is_finite()
			|| temperature.minimum_temperature <= 0.0
			|| temperature.minimum_temperature >= temperature.maximum_temperature
		{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidOptimizer,
				"temperature bounds must be finite, positive, and strictly increasing",
			)); } }
	binary_validation_metric_status(dataset) }

fn binary_validation_metric_status(dataset: &LoweredDenseDataset) -> TrainingCompileResult<ValidationMetricStatus> {
	let status = validation_metric_status(dataset, ValidationMetricFamily::Binary)?;
	let ValidationMetricStatus::Available { known_rows, .. } = status else { return Ok(status); };
	let validation = dataset.validation().ok_or_else(|| { TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"available binary validation omitted its known target rows",
		) })?; let columns = validation.targets().columns(); let every_column_has_both_classes = match validation.targets() {
		DenseMatrix::I32 { values, .. } => (0..columns).all(|column| {
			let values = values.iter().skip(column).step_by(columns);
			values.clone().any(|value| *value == 0) && values.clone().any(|value| *value == 1) }),
		DenseMatrix::F32Bits { values, .. } => (0..columns).all(|column| {
			let values = values.iter().skip(column).step_by(columns); values.clone().any(|bits| f32::from_bits(*bits) == 0.0)
				&& values.clone().any(|bits| f32::from_bits(*bits) == 1.0) }), }; if every_column_has_both_classes {
		return Ok(status); }
	let split_rows = u64::try_from(dataset.validation_split_rows()).map_err(|error| { TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("validation split rows cannot be represented by u64: {error}"),
		) })?; Ok(ValidationMetricStatus::Unavailable { family: ValidationMetricFamily::Binary,
		reason: ValidationUnavailableReason::SingleKnownClass { known_rows }, split_rows, }) }

fn validation_metric_status( dataset: &LoweredDenseDataset, family: ValidationMetricFamily,
) -> TrainingCompileResult<ValidationMetricStatus> {
	let split_rows = u64::try_from(dataset.validation_split_rows()).map_err(|error| { TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("validation split rows cannot be represented by u64: {error}"),
		) })?; let Some(validation) = dataset.validation() else { return Ok(ValidationMetricStatus::Unavailable { family,
			reason: ValidationUnavailableReason::NoKnownTargets, split_rows, }); };
	let known_rows = u64::try_from(validation.rows()) .ok() .and_then(NonZeroU64::new) .ok_or_else(|| {
			TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
				"known validation target rows cannot be represented as nonzero u64",
			) })?; Ok(ValidationMetricStatus::Available { family, known_rows }) }

fn training_bounds( rows: usize, epochs: TrainingHorizon, warmup_epochs: u64, calibration_iterations: u64,
) -> TrainingCompileResult<TrainingBounds> { let train_rows = u64::try_from(rows).map_err(|error| {
		TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
			format!("training row count cannot be represented by u64: {error}"),
		) })?; let train_rows = NonZeroU64::new(train_rows).ok_or_else(|| { TrainingCompileError::new(
			TrainingCompileErrorKind::EmptyDataset,
			"training requires a nonempty prepared partition",
		) })?; let training_iterations = epochs.loop_iterations(); let iterations = match epochs {
		TrainingHorizon::Finite(training_iterations) => { if training_iterations.get() > i32::MAX as u64 {
				return Err(TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
					"GPU-derived training schedule currently requires at most i32::MAX training iterations",
				)); }
			let iterations = training_iterations .get() .checked_add(calibration_iterations) .and_then(NonZeroU64::new)
				.ok_or_else(|| { TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
						"training plus calibration iteration count overflowed",
					) })?; LoopIterations::Finite(iterations) }
		TrainingHorizon::Unbounded => { if calibration_iterations != 0 { return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidOptimizer,
					"post-training calibration is undefined for an unbounded training phase",
				)); }
			LoopIterations::Unbounded } }; if train_rows.get() > i32::MAX as u64 { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			"training rows exceed the int32 IndexMap domain",
		)); }
	let warmup_iterations = warmup_epochs; Ok(TrainingBounds { train_rows: train_rows.get(), epochs, training_iterations,
		calibration_iterations, iterations, warmup_iterations, }) }

fn accepted_update_plan( accepted_updates_per_epoch: usize, config: &DenseTrainingConfig, bounds: &TrainingBounds,
) -> TrainingCompileResult<AcceptedUpdatePlan> {
	let per_epoch = u64::try_from(accepted_updates_per_epoch).map_err(|error| { TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("accepted updates per epoch cannot be represented by u64: {error}"),
		) })?; if per_epoch > 1 { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidOptimizer,
			"accepted updates per epoch exceed the single full-partition update",
		)); }
	let maximum = config .epochs .bound() .map(|epochs| { per_epoch.checked_mul(epochs.get()).ok_or_else(|| {
				TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
					"maximum accepted optimizer update count overflowed u64",
				) }) }) .transpose()?; let warmup = per_epoch.checked_mul(config.warmup_epochs).ok_or_else(|| {
		TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
			"warmup accepted optimizer update count overflowed u64",
		) })?; if let Some(maximum) = maximum { let physical_maximum = bounds .training_iterations .finite()
			.expect("finite training horizon has finite physical iterations")
			.get(); if maximum > physical_maximum || warmup > maximum { return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"accepted optimizer update bounds exceed physical training bounds",
			)); } }
	let counter_limit = maximum.unwrap_or(warmup).max(1); if counter_limit > i32::MAX as u64 {
		return Err(TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
			"accepted optimizer update counter exceeds the int32 schedule domain",
		)); }
	Ok(AcceptedUpdatePlan { per_epoch, maximum, warmup, counter_limit, }) }

#[derive(Debug)]
pub(crate) struct GraphCompiler { tensors: BTreeMap<ValueId, Tensor>, nodes: Vec<CalculationNode>,
	domains: Vec<KernelIterationDomain>, next_value: u64, next_kernel: u64, next_weight_stream: u64,
	next_parameter_input: usize, iterations: LoopIterations, training_domain: IterationDomain,
	resume_enabled: Option<ValueId>, external_inputs: Vec<OwnedExternalInput>, external_input_ids: BTreeSet<ValueId>,
	external_outputs: BTreeSet<ValueId>, }

struct TrainingForwardGraph<'a> {
	compiler: &'a mut GraphCompiler,
	domain: IterationDomain, }

impl RecurrentForwardGraph for TrainingForwardGraph<'_> {
	type Error = TrainingCompileError;

	fn zero_f32(&mut self, shape: Shape) -> TrainingCompileResult<ValueId> { self.compiler.zero_f32_tensor(shape) }

	fn gather_matrix_column( &mut self, matrix: ValueId, rows: u64, columns: u64, column: u64,
	) -> TrainingCompileResult<ValueId> { self.compiler .gather_matrix_column(matrix, rows, columns, column, self.domain) }

	fn bias_free_linear( &mut self, input: ValueId, weight: ValueId, rows: u64, output_width: u64,
	) -> TrainingCompileResult<ValueId> { self.compiler .bias_free_linear(input, weight, rows, output_width, self.domain) }

	fn elementwise_f32( &mut self, shape: Shape, inputs: Vec<ValueId>, program: recipe_core::ScalarProgram,
	) -> TrainingCompileResult<ValueId> { self.compiler .elementwise_f32(shape, inputs, program, self.domain) }

	fn activate( &mut self, input: ValueId, activation: DenseActivation, shape: Shape,
	) -> TrainingCompileResult<ValueId> { self.compiler .apply_activation(input, activation, None, shape, self.domain) } }

enum ScalarOperation {
	Owned(&'static str),
	Program(recipe_core::ScalarProgram), }

enum BackwardActivation { Cosine, Input(ScalarOperation), Output(ScalarOperation), PRelu, }

impl DenseActivation { fn backward_kernel(self) -> TrainingCompileResult<Option<BackwardActivation>> {
		Ok(Some(match self { Self::Linear => return Ok(None), Self::Cosine => BackwardActivation::Cosine,
			Self::Exponential => BackwardActivation::Output(ScalarOperation::Program(multiply_program()?)),
			Self::Logarithm => BackwardActivation::Input(ScalarOperation::Program(signed_logarithm_backward_program()?)),
			Self::NaturalLogarithm => BackwardActivation::Input(ScalarOperation::Program(natural_logarithm_backward_program()?)),
			Self::LegacySignedLogOnePlus => BackwardActivation::Input(ScalarOperation::Program(signed_log_one_plus_backward_program()?)),
			Self::Huber => BackwardActivation::Input(ScalarOperation::Program(huber_activation_backward_program()?)),
			Self::Tangent => BackwardActivation::Output(ScalarOperation::Program(tangent_activation_backward_program()?)),
			Self::Relu => BackwardActivation::Input(ScalarOperation::Owned("gpu_relu_backward_into")),
			Self::LeakyRelu => BackwardActivation::Input(ScalarOperation::Program(canonical_leaky_relu_backward_program()?)),
			Self::Sigmoid => BackwardActivation::Output(ScalarOperation::Owned("gpu_sigmoid_backward_into")),
			Self::Tanh => BackwardActivation::Output(ScalarOperation::Owned("gpu_tanh_backward_into")),
			Self::Selu => BackwardActivation::Input(ScalarOperation::Program(canonical_selu_backward_program()?)),
			Self::Gelu => BackwardActivation::Input(ScalarOperation::Owned("gpu_gelu_backward_into")),
			Self::Silu => BackwardActivation::Input(ScalarOperation::Owned("gpu_silu_backward_into")),
			Self::Elu => BackwardActivation::Input(ScalarOperation::Program(canonical_elu_backward_program()?)),
			Self::PRelu => BackwardActivation::PRelu, })) } }

impl GraphCompiler {
	fn new(iterations: LoopIterations, training_iterations: LoopIterations) -> TrainingCompileResult<Self> {
		let training_domain = IterationDomain::every(training_iterations); let mut compiler = Self { tensors: BTreeMap::new(),
			nodes: Vec::new(), domains: Vec::new(), next_value: 1, next_kernel: 1, next_weight_stream: 0,
			next_parameter_input: 0, iterations, training_domain, resume_enabled: None, external_inputs: Vec::new(),
			external_input_ids: BTreeSet::new(), external_outputs: BTreeSet::new(), };
		let resume_enabled = compiler.external_i32_tensor(ExternalInputRole::ResumeEnabled, shape(&[1])?, &[0])?;
		compiler.resume_enabled = Some(resume_enabled); Ok(compiler) }

	fn tensor(&mut self, dtype: DType, shape: Shape) -> TrainingCompileResult<ValueId> { let value = self.next_value()?;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false)?; self.tensors.insert(value, tensor); Ok(value) }

	fn external_matrix(&mut self, role: ExternalInputRole, matrix: &DenseMatrix) -> TrainingCompileResult<ValueId> {
		let rows = u64::try_from(matrix.rows()).map_err(|error| { TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} rows cannot be represented by u64: {error}"),
			) })?; let columns = u64::try_from(matrix.columns()).map_err(|error| { TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} columns cannot be represented by u64: {error}"),
			) })?; let shape = shape(&[rows, columns])?; let (dtype, bytes) = matrix_bytes(matrix);
		let expected_bytes = shape.bytes(dtype)?.get(); if u64::try_from(bytes.len()).ok() != Some(expected_bytes) {
			return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix, format!(
					"{role:?} encodes {} bytes, tensor contract requires {expected_bytes}",
					bytes.len() ), )); }
		let value = self.tensor(dtype, shape.clone())?; self.external_input_ids.insert(value); self.external_inputs
			.push(OwnedExternalInput::new(role, value, dtype, shape, bytes)); Ok(value) }

	fn external_f32_vector(&mut self, role: ExternalInputRole, values: &[u32]) -> TrainingCompileResult<ValueId> {
		if values.is_empty() { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!("{role:?} cannot be empty"),
			)); }
		if let Some(bits) = values .iter() .copied() .find(|bits| !f32::from_bits(*bits).is_finite())
		{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!("{role:?} contains non-finite f32 bits {bits:#010x}"),
			)); }
		let columns = u64::try_from(values.len()).map_err(|error| { TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} width cannot be represented by u64: {error}"),
			) })?; self.external_f32_tensor(role, shape(&[columns])?, values) }

	fn external_f32_tensor( &mut self, role: ExternalInputRole, tensor_shape: Shape, values: &[u32],
	) -> TrainingCompileResult<ValueId> { if let Some(bits) = values .iter() .copied()
			.find(|bits| !f32::from_bits(*bits).is_finite())
		{ return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!("{role:?} contains non-finite f32 bits {bits:#010x}"),
			)); }
		let bytes = values .iter() .flat_map(|bits| bits.to_le_bytes()) .collect::<Vec<_>>();
		let value = self.tensor(DType::F32, tensor_shape.clone())?; self.external_input_ids.insert(value);
		self.external_inputs.push(OwnedExternalInput::new( role, value, DType::F32, tensor_shape, bytes, )); Ok(value) }

	fn external_f32_zeros(&mut self, role: ExternalInputRole, tensor_shape: Shape) -> TrainingCompileResult<ValueId> {
		let byte_count = tensor_shape.bytes(DType::F32)?.get(); let byte_count = usize::try_from(byte_count).map_err(|error| {
			TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} byte count cannot be represented by usize: {error}"),
			) })?; let value = self.tensor(DType::F32, tensor_shape.clone())?; self.external_input_ids.insert(value);
		self.external_inputs.push(OwnedExternalInput::new( role, value, DType::F32, tensor_shape, vec![0; byte_count], ));
		Ok(value) }

	fn external_i32_tensor( &mut self, role: ExternalInputRole, tensor_shape: Shape, values: &[i32],
	) -> TrainingCompileResult<ValueId> { let bytes = values .iter() .flat_map(|value| value.to_le_bytes())
			.collect::<Vec<_>>(); let value = self.tensor(DType::I32, tensor_shape.clone())?;
		self.external_input_ids.insert(value); self.external_inputs.push(OwnedExternalInput::new( role, value, DType::I32,
			tensor_shape, bytes, )); Ok(value) }

	fn external_i32_zeros(&mut self, role: ExternalInputRole, tensor_shape: Shape) -> TrainingCompileResult<ValueId> {
		let elements = usize::try_from(tensor_shape.elements()).map_err(|error| { TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} element count cannot be represented by usize: {error}"),
			) })?; self.external_i32_tensor(role, tensor_shape, &vec![0; elements]) }

	fn convert_matrix_if_i32( &mut self, input: ValueId, output_shape: Shape, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { if self.tensor_ref(input)?.dtype == DType::F32 { return Ok(input); }
		self.elementwise_f32(output_shape, vec![input], convert_i32_program()?, domain) }

	fn z_score( &mut self, input: ValueId, matrix_shape: Shape, dimensions: [u64; 2], epsilon: f32, tree_lanes: u32,
		normalization_mask: Option<ValueId>, ) -> TrainingCompileResult<ZScoreValues> { let [columns, rows] = dimensions;
		let column_shape = shape(&[columns])?; let sums = self.sum_tensor( input, column_shape.clone(), &[0], tree_lanes,
			IterationDomain::first(), )?; let means = self.elementwise_f32( column_shape.clone(), vec![sums],
			divide_constant_program(rows as f32)?, IterationDomain::first(), )?; let centered = self.elementwise_f32(
			matrix_shape.clone(), vec![input, means], subtract_program()?, IterationDomain::first(), )?;
		let squares = self.elementwise_f32( matrix_shape.clone(), vec![centered], square_program()?, IterationDomain::first(),
		)?; let variance_sums = self.sum_tensor( squares, column_shape.clone(), &[0], tree_lanes, IterationDomain::first(),
		)?; let variances = self.elementwise_f32( column_shape, vec![variance_sums], divide_constant_program(rows as f32)?,
			IterationDomain::first(), )?; let normalized = self.apply_z_score( [input, means, variances], matrix_shape, epsilon,
			IterationDomain::first(), normalization_mask, )?; Ok(ZScoreValues { normalized, mean: means, variance: variances, })
	}

	fn apply_z_score( &mut self, values: [ValueId; 3], output_shape: Shape, epsilon: f32, domain: IterationDomain,
		normalization_mask: Option<ValueId>, ) -> TrainingCompileResult<ValueId> { let [input, mean, variance] = values;
		let mut inputs = vec![input, mean, variance]; if let Some(mask) = normalization_mask { inputs.push(mask); }
		self.elementwise_f32( output_shape, inputs, z_score_program(epsilon, normalization_mask.is_some())?, domain, ) }

	fn min_max( &mut self, input: ValueId, matrix_shape: Shape, columns: u64, epsilon: f32, tree_lanes: u32,
		normalization_mask: Option<ValueId>, ) -> TrainingCompileResult<MinMaxValues> { let column_shape = shape(&[columns])?;
		let minimum = self.tensor(DType::F32, column_shape.clone())?; self.reduce_value( [input, minimum],
			ReduceOperator::Minimum, &[0], false, tree_lanes, IterationDomain::first(), )?;
		let maximum = self.tensor(DType::F32, column_shape)?; self.reduce_value( [input, maximum], ReduceOperator::Maximum,
			&[0], false, tree_lanes, IterationDomain::first(), )?; let normalized = self.apply_min_max(
			[input, minimum, maximum], matrix_shape, epsilon, IterationDomain::first(), normalization_mask, )?; Ok(MinMaxValues {
			normalized, minimum, maximum, }) }

	fn apply_min_max( &mut self, values: [ValueId; 3], output_shape: Shape, epsilon: f32, domain: IterationDomain,
		normalization_mask: Option<ValueId>, ) -> TrainingCompileResult<ValueId> { let [input, minimum, maximum] = values;
		let mut inputs = vec![input, minimum, maximum]; if let Some(mask) = normalization_mask { inputs.push(mask); }
		self.elementwise_f32( output_shape, inputs, min_max_program(epsilon, normalization_mask.is_some())?, domain, ) }

	fn l2_norm( &mut self, input: ValueId, matrix_shape: Shape, epsilon: f32, tree_lanes: u32, domain: IterationDomain,
		normalization_mask: Option<ValueId>, ) -> TrainingCompileResult<ValueId> {
		let rows = matrix_shape.extents().first().copied().ok_or_else(|| { TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				"L2 normalization requires a matrix",
			) })?; let mut square_inputs = vec![input]; if let Some(mask) = normalization_mask { square_inputs.push(mask); }
		let squares = self.elementwise_f32( matrix_shape.clone(), square_inputs,
			l2_square_program(normalization_mask.is_some())?, domain, )?;
		let norms_squared = self.tensor(DType::F32, shape(&[rows, 1])?)?; self.reduce_value( [squares, norms_squared],
			ReduceOperator::Sum, &[1], true, tree_lanes, domain, )?; let mut normalize_inputs = vec![input, norms_squared];
		if let Some(mask) = normalization_mask { normalize_inputs.push(mask); }
		self.elementwise_f32( matrix_shape, normalize_inputs, l2_norm_program(epsilon, normalization_mask.is_some())?, domain,
		) }

	fn apply_activation( &mut self, input: ValueId, activation: DenseActivation, prelu: Option<ValueId>,
		output_shape: Shape, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		let activation = activation.forward_kernel::<TrainingCompileError>()?;
		if matches!(activation, ForwardActivation::Identity) { return Ok(input); }
		let output = self.tensor(DType::F32, output_shape.clone())?; let (inputs, operation) = match activation {
			ForwardActivation::Identity => unreachable!("linear activation returned its input"),
			ForwardActivation::Owned(symbol) => (vec![input], ScalarOperation::Owned(symbol)),
			ForwardActivation::Program(program) => (vec![input], ScalarOperation::Program(program)),
			ForwardActivation::SignedMagnitude(magnitude_symbol) => { let absolute =
					self.owned_scalar_f32(output_shape.clone(), "gpu_abs_into", vec![input], domain)?;
				let magnitude = self.owned_scalar_f32( output_shape.clone(), magnitude_symbol, vec![absolute], domain, )?;
				let sign = self.owned_scalar_f32(output_shape, "gpu_sign_into", vec![input], domain)?;
				self.emit_elementwise( vec![sign, magnitude], vec![output], multiply_program()?, domain, )?; return Ok(output); }
			ForwardActivation::PRelu(program) => {
				let alpha = prelu.expect("compiled PReLU parameter");
				(vec![input, alpha], ScalarOperation::Program(program)) } };
		self.emit_scalar_operation(inputs, vec![output], operation, domain)?; Ok(output) }

	fn mask_f32_with_zero( &mut self, input: ValueId, validity: ValueId, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let input_shape = self.tensor_ref(input)?.shape.clone(); self.elementwise_f32(
			input_shape, vec![input, validity], masked_zero_f32_program()?, domain, ) }

	fn mask_i32_with_zero( &mut self, input: ValueId, validity: ValueId, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let input_shape = self.tensor_ref(input)?.shape.clone();
		let output = self.tensor(DType::I32, input_shape)?; self.emit_elementwise( vec![input, validity], vec![output],
			masked_zero_i32_program()?, domain, )?; Ok(output) }

	fn cross_entropy_with_logits( &mut self, logits: ValueId, targets: ValueId, outputs: [ValueId; 2],
		dimensions: [u64; 2], tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<()> {
		let [rows, classes] = dimensions;
		let classes_i32 = checked_i32(classes, "categorical class count")?;
		let class_indices = self.index_map( shape(&[1, classes])?, IndexMap { start: 0, element_step: 1, iteration_step: 0,
				modulus: None, }, IterationDomain::first(), )?;
		let softmax = self.stable_softmax_values(logits, rows, classes, tree_lanes, domain)?; self.emit_elementwise( vec![
				logits, targets, class_indices, softmax.maximum, softmax.logarithmic_sum, softmax.exponentials,
				softmax.exponential_sum, ], Vec::from(outputs), cross_entropy_with_logits_program(classes_i32)?, domain, )?; Ok(())
	}

	fn cross_entropy_with_dense_targets( &mut self, logits: ValueId, targets: ValueId, outputs: [ValueId; 2],
		dimensions: [u64; 2], tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<()> {
		let [rows, classes] = dimensions;
		let softmax = self.stable_softmax_values(logits, rows, classes, tree_lanes, domain)?; self.emit_elementwise( vec![
				logits, targets, softmax.maximum, softmax.logarithmic_sum, softmax.exponentials, softmax.exponential_sum, ],
			Vec::from(outputs), cross_entropy_with_dense_targets_program()?, domain, )?; Ok(()) }

	fn stable_softmax_values( &mut self, logits: ValueId, rows: u64, classes: u64, tree_lanes: u32,
		domain: IterationDomain, ) -> TrainingCompileResult<SoftmaxValues> { let matrix_shape = shape(&[rows, classes])?;
		let row_shape = shape(&[rows, 1])?; let maximum = self.tensor(DType::F32, row_shape.clone())?; self.reduce_value(
			[logits, maximum], ReduceOperator::Maximum, &[1], true, tree_lanes, domain, )?; let shifted = self.elementwise_f32(
			matrix_shape.clone(), vec![logits, maximum], subtract_program()?, domain, )?;
		let exponentials = self.owned_scalar_f32(matrix_shape, "gpu_exp", vec![shifted], domain)?;
		let exponential_sum = self.tensor(DType::F32, row_shape.clone())?; self.reduce_value( [exponentials, exponential_sum],
			ReduceOperator::Sum, &[1], true, tree_lanes, domain, )?;
		let logarithmic_sum = self.owned_scalar_f32(row_shape, "gpu_log_into", vec![exponential_sum], domain)?;
		Ok(SoftmaxValues { maximum, logarithmic_sum, exponentials, exponential_sum, }) }

	fn normalize_training( &mut self, input: ValueId, normalization: DenseNormalization, dimensions: [u64; 2],
		validity: [ValueId; 2], epsilon: f32, tree_lanes: u32, ) -> TrainingCompileResult<NormalizedValues> {
		let [rows, columns] = dimensions; let [validity, valid_count] = validity; match normalization {
			DenseNormalization::Layer => self.normalize_unmasked( input, normalization, [rows, columns], epsilon, tree_lanes,
				self.training_domain, ), DenseNormalization::Batch => self.normalize_masked_batch( input, [rows, columns], validity,
				valid_count, epsilon, tree_lanes, ), } }

	fn normalize_validation( &mut self, input: ValueId, normalization: DenseNormalization, dimensions: [u64; 2],
		epsilon: f32, tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<NormalizedValues> {
		self.normalize_unmasked(input, normalization, dimensions, epsilon, tree_lanes, domain) }

	fn normalize_unmasked( &mut self, input: ValueId, normalization: DenseNormalization, dimensions: [u64; 2],
		epsilon: f32, tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<NormalizedValues> {
		let [rows, columns] = dimensions; let (axis, population, keep_dimensions) = match normalization {
			DenseNormalization::Layer => (1, columns as f32, true), DenseNormalization::Batch => (0, rows as f32, false), };
		let matrix_shape = shape(&[rows, columns])?; let statistic_shape = if keep_dimensions { shape(&[rows, 1])? } else {
			shape(&[columns])? }; let sums = self.tensor(DType::F32, statistic_shape.clone())?; self.reduce_value( [input, sums],
			ReduceOperator::Sum, &[axis], keep_dimensions, tree_lanes, domain, )?; let means = self.elementwise_f32(
			statistic_shape.clone(), vec![sums], divide_constant_program(population)?, domain, )?;
		let centered = self.elementwise_f32( matrix_shape.clone(), vec![input, means], subtract_program()?, domain, )?;
		let squares = self.elementwise_f32( matrix_shape.clone(), vec![centered], square_program()?, domain, )?;
		let variance_sums = self.tensor(DType::F32, statistic_shape.clone())?; self.reduce_value( [squares, variance_sums],
			ReduceOperator::Sum, &[axis], keep_dimensions, tree_lanes, domain, )?; let variance = self.elementwise_f32(
			statistic_shape, vec![variance_sums], divide_constant_program(population)?, domain, )?;
		let normalized = self.apply_z_score([input, means, variance], matrix_shape, epsilon, domain, None)?;
		Ok(NormalizedValues { normalized, variance, }) }

	fn normalize_masked_batch( &mut self, input: ValueId, dimensions: [u64; 2], validity: ValueId, valid_count: ValueId,
		epsilon: f32, tree_lanes: u32, ) -> TrainingCompileResult<NormalizedValues> { let [rows, columns] = dimensions;
		let matrix_shape = shape(&[rows, columns])?; let statistic_shape = shape(&[columns])?;
		let safe_input = self.mask_f32_with_zero(input, validity, self.training_domain)?; let sums = self.sum_tensor(
			safe_input, statistic_shape.clone(), &[0], tree_lanes, self.training_domain, )?; let means = self.elementwise_f32(
			statistic_shape.clone(), vec![sums, valid_count], divide_program()?, self.training_domain, )?;
		let unmasked_centered = self.elementwise_f32( matrix_shape.clone(), vec![safe_input, means], subtract_program()?,
			self.training_domain, )?; let centered = self.mask_f32_with_zero(unmasked_centered, validity, self.training_domain)?;
		let squares = self.elementwise_f32( matrix_shape.clone(), vec![centered], square_program()?, self.training_domain, )?;
		let masked_squares = self.elementwise_f32( matrix_shape.clone(), vec![squares, validity], masked_zero_f32_program()?,
			self.training_domain, )?; let variance_sums = self.sum_tensor( masked_squares, statistic_shape.clone(), &[0],
			tree_lanes, self.training_domain, )?; let variance = self.elementwise_f32( statistic_shape,
			vec![variance_sums, valid_count], divide_program()?, self.training_domain, )?; let normalized = self.apply_z_score(
			[safe_input, means, variance], matrix_shape, epsilon, self.training_domain, None, )?;
		let normalized = self.mask_f32_with_zero(normalized, validity, self.training_domain)?; Ok(NormalizedValues {
			normalized, variance, }) }

	fn backward_activation( &mut self, gradient: ValueId, input: ValueId, output: ValueId, activation: DenseActivation,
	) -> TrainingCompileResult<ValueId> { let Some(kernel) = activation.backward_kernel()? else { return Ok(gradient) };
		let output_shape = self.tensor_ref(gradient)?.shape.clone();
		let activation_gradient = self.tensor(DType::F32, output_shape.clone())?; let (inputs, operation) = match kernel {
			BackwardActivation::Cosine => {
				let sine = self.owned_scalar_f32(output_shape, "gpu_sin", vec![input], self.training_domain)?;
				(vec![gradient, sine], ScalarOperation::Program(negative_multiply_program()?)) }
			BackwardActivation::Input(operation) => (vec![gradient, input], operation),
			BackwardActivation::Output(operation) => (vec![gradient, output], operation),
			BackwardActivation::PRelu => unreachable!("PReLU uses its parameterized backward path"),
		}; self.emit_scalar_operation( inputs, vec![activation_gradient], operation, self.training_domain, )?;
		Ok(activation_gradient) }

	fn backward_normalization( &mut self, gradient: ValueId, operation: OperationValues, validity: ValueId,
		valid_count: ValueId, epsilon: f32, tree_lanes: u32, ) -> TrainingCompileResult<ValueId> {
		let DenseOperation::Normalization(normalization) = operation.operation else {
			unreachable!("normalization backward receives a normalization tape entry")
		}; let normalized = operation.output;
		let variance = operation.variance.expect("compiled normalization tape");
		let matrix_shape = self.tensor_ref(gradient)?.shape.clone();
		let gradient = self.mask_f32_with_zero(gradient, validity, self.training_domain)?;
		let normalized = self.mask_f32_with_zero(normalized, validity, self.training_domain)?;
		let extents = matrix_shape.extents(); let rows = extents[0]; let columns = extents[1];
		let (axis, keep_dimensions, statistic_shape) = match normalization {
			DenseNormalization::Layer => (1, true, shape(&[rows, 1])?),
			DenseNormalization::Batch => (0, false, shape(&[columns])?), };
		let gradient_sums = self.tensor(DType::F32, statistic_shape.clone())?; self.reduce_value( [gradient, gradient_sums],
			ReduceOperator::Sum, &[axis], keep_dimensions, tree_lanes, self.training_domain, )?;
		let (gradient_mean_inputs, gradient_mean_program) = match normalization { DenseNormalization::Layer => (
				vec![gradient_sums], divide_constant_program(columns as f32)?, ),
			DenseNormalization::Batch => (vec![gradient_sums, valid_count], divide_program()?), };
		let gradient_means = self.elementwise_f32( statistic_shape.clone(), gradient_mean_inputs, gradient_mean_program,
			self.training_domain, )?; let gradient_products = self.elementwise_f32( matrix_shape.clone(),
			vec![gradient, normalized], multiply_program()?, self.training_domain, )?;
		let product_sums = self.tensor(DType::F32, statistic_shape.clone())?; self.reduce_value(
			[gradient_products, product_sums], ReduceOperator::Sum, &[axis], keep_dimensions, tree_lanes, self.training_domain,
		)?; let (product_mean_inputs, product_mean_program) = match normalization {
			DenseNormalization::Layer => (vec![product_sums], divide_constant_program(columns as f32)?),
			DenseNormalization::Batch => (vec![product_sums, valid_count], divide_program()?), };
		let product_means = self.elementwise_f32( statistic_shape.clone(), product_mean_inputs, product_mean_program,
			self.training_domain, )?; let inverse_standard_deviation = self.elementwise_f32( statistic_shape, vec![variance],
			inverse_standard_deviation_program(epsilon)?, self.training_domain, )?; let unmasked = self.elementwise_f32(
			matrix_shape.clone(), vec![ gradient, gradient_means, normalized, product_means, inverse_standard_deviation, ],
			normalization_backward_program()?, self.training_domain, )?; if normalization == DenseNormalization::Layer {
			return Ok(unmasked); }
		self.elementwise_f32( matrix_shape, vec![unmasked, validity], masked_zero_f32_program()?, self.training_domain, ) }

	fn compile_training_blocks( &mut self, blocks: &[DenseBlock],
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<(ValueId, u64, Vec<Box<dyn CompiledBlock>>)> { let mut current = context.input;
		let mut logical = context.logical; let mut preceding_routing = context.routing;
		let mut values: Vec<Box<dyn CompiledBlock>> = Vec::with_capacity(blocks.len());
		for (block_index, block) in blocks.iter().enumerate() { let forward = block.declaration().compile_forward( self,
				BlockForwardContext { input: current, logical, routing: preceding_routing, block_index, ..context }, )?;
			current = forward.output; logical = forward.logical; preceding_routing = forward.routing; values.push(forward.tape);
		}
		Ok((current, logical.width()?, values)) }

	fn tree_target_matrix( &mut self, targets: ValueId, supervision: ValueId, rows: u64, outputs: NonZeroU64,
		task: DenseTask, ) -> TrainingCompileResult<ValueId> {
		if !matches!(task, DenseTask::MulticlassClassification { .. }) { self.require_tensor( targets, DType::F32,
				&[rows, outputs.get()],
				"tree target matrix",
			)?; return Ok(targets); }
		self.require_tensor( targets, DType::I32, &[rows, 1],
			"tree categorical target codes",
		)?; let class_positions = self.index_map( shape(&[1, outputs.get()])?, IndexMap { start: 0, element_step: 1,
				iteration_step: 0, modulus: None, }, IterationDomain::first(), )?; let one_hot = self.elementwise_f32(
			shape(&[rows, outputs.get()])?, vec![targets, class_positions, supervision], tree_one_hot_target_program()?,
			IterationDomain::first(), )?; Ok(one_hot) }

	fn bootstrap_tree_rows( &mut self, values: [ValueId; 3], dimensions: [u64; 3], random_seed: u64, tree: u64,
		domain: IterationDomain, ) -> TrainingCompileResult<(ValueId, ValueId, ValueId)> {
		let [input, targets, supervision] = values; let [rows, features, outputs] = dimensions;
		let sample_indices = self.tensor(DType::I32, shape(&[rows])?)?; self.emit( Vec::new(), vec![sample_indices],
			PrimitiveKind::Random(RandomMap { distribution: RandomDistribution::UniformI32 { low: 0,
					high_exclusive: checked_i32(rows, "tree bootstrap row count")?,
				}, key: RandomKey { seed_low: random_seed, seed_high: random_seed.rotate_left(29) ^ 0x9e37_79b9_7f4a_7c15,
					stream: tree.checked_add(1).ok_or_else(identity_exhausted)?, }, philox_rounds: 10, }), Vec::new(), domain, )?;
		let sampled_input = self.gather(input, sample_indices, shape(&[rows, features])?, 0, domain)?;
		let sampled_targets = self.gather(targets, sample_indices, shape(&[rows, outputs])?, 0, domain)?;
		let sampled_supervision = self.gather(supervision, sample_indices, shape(&[rows, 1])?, 0, domain)?;
		Ok((sampled_input, sampled_targets, sampled_supervision)) }

	fn supervised_global_feature_thresholds( &mut self, values: [ValueId; 3], dimensions: [u64; 2], tree_lanes: u32,
		domain: IterationDomain, ) -> TrainingCompileResult<ValueId> { let [input, supervision, feature_positions] = values;
		let [rows, features] = dimensions; let feature_contributions = self.tensor(DType::F32, shape(&[rows, features])?)?;
		let count_contributions = self.tensor(DType::F32, shape(&[rows, features])?)?; self.emit_elementwise(
			vec![input, supervision], vec![feature_contributions], multiply_program()?, domain, )?; self.emit_elementwise(
			vec![supervision, feature_positions], vec![count_contributions], repeat_f32_with_i32_program()?, domain, )?;
		let feature_sums = self.tensor(DType::F32, shape(&[features])?)?;
		let feature_counts = self.tensor(DType::F32, shape(&[features])?)?; self.reduce_sum( feature_contributions,
			feature_sums, &[0], tree_lanes, domain, )?; self.reduce_sum( count_contributions, feature_counts, &[0], tree_lanes,
			domain, )?; let thresholds = self.elementwise_f32( shape(&[features])?, vec![feature_sums, feature_counts],
			tree_safe_mean_program()?, domain, )?; Ok(thresholds) }

	fn compile_lightgbm_best_leaf_candidate( &mut self, values: [ValueId; 8], dimensions: [u64; 4], tree_lanes: u32,
		domain: IterationDomain, ) -> TrainingCompileResult<(ValueId, ValueId, ValueId, ValueId)> {
		let [input, targets, supervision, target_cube, nodes, feature_positions, output_positions, output_cube_positions] = values;
		let [rows, features, outputs, internal_nodes] = dimensions;
		let node_features = checked_product(&[internal_nodes, features], "LightGBM node-feature count")?;
		let node_outputs = checked_product(&[internal_nodes, outputs], "LightGBM node-output count")?;
		let node_feature_outputs = checked_product( &[internal_nodes, features, outputs],
			"LightGBM node-feature-output count",
		)?; let active_nodes = self.tensor(DType::I32, shape(&[rows, 1])?)?;
		let active_supervision = self.tensor(DType::F32, shape(&[rows, 1])?)?; self.emit_elementwise(
			vec![nodes, supervision], vec![active_nodes, active_supervision], tree_lightgbm_active_row_program(internal_nodes)?,
			domain, )?; let candidate_indices = self.tensor(DType::I32, shape(&[rows, features])?)?; self.emit_elementwise(
			vec![active_nodes, feature_positions], vec![candidate_indices], tree_candidate_index_program(features)?, domain, )?;

		let feature_contributions = self.tensor(DType::F32, shape(&[rows, features])?)?;
		let count_contributions = self.tensor(DType::F32, shape(&[rows, features])?)?; self.emit_elementwise(
			vec![input, active_supervision], vec![feature_contributions], multiply_program()?, domain, )?; self.emit_elementwise(
			vec![active_supervision, feature_positions], vec![count_contributions], repeat_f32_with_i32_program()?, domain, )?;
		let feature_sums = self.deterministic_segment_sum( candidate_indices, feature_contributions, node_features, rows,
			tree_lanes, domain, )?; let candidate_counts = self.deterministic_segment_sum( candidate_indices,
			count_contributions, node_features, rows, tree_lanes, domain, )?; let candidate_thresholds = self.elementwise_f32(
			shape(&[node_features])?, vec![feature_sums, candidate_counts], tree_safe_mean_program()?, domain, )?;
		let row_thresholds = self.gather( candidate_thresholds, candidate_indices, shape(&[rows, features])?, 0, domain, )?;
		let left_weights = self.elementwise_f32( shape(&[rows, features])?, vec![input, row_thresholds, active_supervision],
			tree_left_weight_program()?, domain, )?;

		let parent_counts = self.deterministic_segment_sum( active_nodes, active_supervision, internal_nodes, rows,
			tree_lanes, domain, )?; let parent_indices = self.tensor(DType::I32, shape(&[rows, outputs])?)?;
		self.emit_elementwise( vec![active_nodes, output_positions], vec![parent_indices],
			tree_parent_output_index_program(outputs)?, domain, )?; let masked_targets = self.elementwise_f32(
			shape(&[rows, outputs])?, vec![targets, active_supervision], multiply_program()?, domain, )?;
		let parent_sums = self.deterministic_segment_sum( parent_indices, masked_targets, node_outputs, rows, tree_lanes,
			domain, )?;

		let (flat_left_weights, _) = self.pack_matrix_to_flat(left_weights, rows, features, domain)?;
		let left_weight_cube = self.gather_flat_as(flat_left_weights, shape(&[rows, features, 1])?, domain)?;
		let left_target_contributions = self.elementwise_f32( shape(&[rows, features, outputs])?,
			vec![target_cube, left_weight_cube], multiply_program()?, domain, )?; let (flat_candidate_indices, _) =
			self.pack_i32_matrix_to_flat(candidate_indices, rows, features, domain)?; let candidate_index_cube =
			self.gather_flat_as(flat_candidate_indices, shape(&[rows, features, 1])?, domain)?;
		let left_output_indices = self.tensor(DType::I32, shape(&[rows, features, outputs])?)?; self.emit_elementwise(
			vec![candidate_index_cube, output_cube_positions], vec![left_output_indices], tree_leaf_index_program(outputs)?,
			domain, )?; let left_sums = self.deterministic_segment_sum( left_output_indices, left_target_contributions,
			node_feature_outputs, rows, tree_lanes, domain, )?; let left_counts = self.deterministic_segment_sum(
			candidate_indices, left_weights, node_features, rows, tree_lanes, domain, )?;

		let parent_sum_cube = self.gather_flat_as(parent_sums, shape(&[internal_nodes, 1, outputs])?, domain)?;
		let parent_count_cube = self.gather_flat_as(parent_counts, shape(&[internal_nodes, 1, 1])?, domain)?;
		let left_sum_cube = self.gather_flat_as( left_sums, shape(&[internal_nodes, features, outputs])?, domain, )?;
		let left_count_cube = self.gather_flat_as(left_counts, shape(&[internal_nodes, features, 1])?, domain)?;
		let gain_contributions = self.elementwise_f32( shape(&[internal_nodes, features, outputs])?, vec![ left_sum_cube,
				parent_sum_cube, left_count_cube, parent_count_cube, ], tree_variance_gain_program(f32::MIN)?, domain, )?;
		let gains = self.sum_tensor( gain_contributions, shape(&[internal_nodes, features])?, &[2], tree_lanes, domain, )?;
		let flat_gains = self.pack_contiguous_tensor_to_flat(gains, domain)?;
		let best_gain = self.tensor(DType::F32, shape(&[1])?)?; let best_candidate = self.tensor(DType::I32, shape(&[1])?)?;
		self.emit( vec![flat_gains], vec![best_gain, best_candidate], PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Maximum, axes: AxisSet::new(vec![0])?, keep_dimensions: false,
				result: ReduceResult::ValueAndIndex, tree_lanes, }), forbidden_aliases(1, 2), domain, )?;
		let best_node = self.tensor(DType::I32, shape(&[1])?)?; let best_feature = self.tensor(DType::I32, shape(&[1])?)?;
		self.emit_elementwise( vec![best_candidate], vec![best_node, best_feature],
			tree_lightgbm_candidate_choice_program(features)?, domain, )?; let best_threshold = self.gather(
			candidate_thresholds, best_candidate, shape(&[1])?, 0, domain, )?;
		Ok((best_gain, best_node, best_feature, best_threshold)) }

	fn compile_supervised_lightgbm_tree_structure( &mut self, values: [ValueId; 3], rows: u64, widths: [NonZeroU64; 3],
		declaration: DenseTree, tree_lanes: u32, random_seed: u64, ) -> TrainingCompileResult<(ValueId, ValueId)> {
		let [input, targets, supervision] = values; let [input_width, output_width, internal_nodes_per_tree] = widths;
		let domain = IterationDomain::first(); let features = input_width.get(); let outputs = output_width.get();
		let trees = declaration.trees().get(); let internal_nodes = internal_nodes_per_tree.get();
		let split_elements = checked_product(&[trees, internal_nodes], "LightGBM split structure size")?;
		let mut split_features = self.zero_i32_tensor(shape(&[split_elements])?)?;
		let threshold_seed = self.zero_i32_tensor(shape(&[split_elements])?)?;
		let mut split_thresholds = self.tensor(DType::F32, shape(&[split_elements])?)?; self.emit_elementwise(
			vec![threshold_seed], vec![split_thresholds], constant_f32_from_i32_program(f32::MAX)?, domain, )?;
		let feature_positions = self.index_map( shape(&[1, features])?, IndexMap { start: 0, element_step: 1,
				iteration_step: 0, modulus: None, }, domain, )?; let output_positions_flat = self.index_map( shape(&[outputs])?,
			IndexMap { start: 0, element_step: 1, iteration_step: 0, modulus: None, }, domain, )?;
		let output_positions = self.gather_flat_as(output_positions_flat, shape(&[1, outputs])?, domain)?;
		let output_cube_positions = self.gather_flat_as(output_positions_flat, shape(&[1, 1, outputs])?, domain)?;
		let maximum_splits = internal_nodes.min(rows.saturating_sub(1));

		for tree in 0..trees { let (tree_input, tree_targets, tree_supervision) = if trees == 1 {
				(input, targets, supervision) } else { self.bootstrap_tree_rows( [input, targets, supervision],
					[rows, features, outputs], random_seed, tree, domain, )? };
			let (flat_features, _) = self.pack_matrix_to_flat(tree_input, rows, features, domain)?;
			let (flat_targets, _) = self.pack_matrix_to_flat(tree_targets, rows, outputs, domain)?;
			let target_cube = self.gather_flat_as(flat_targets, shape(&[rows, 1, outputs])?, domain)?;
			let row_positions = self.index_map( shape(&[rows, 1])?, IndexMap { start: 0, element_step: 1, iteration_step: 0,
					modulus: None, }, domain, )?; let mut nodes = self.zero_i32_tensor(shape(&[rows, 1])?)?;
			for _ in 0..maximum_splits { let (best_gain, best_node, best_feature, best_threshold) = self
					.compile_lightgbm_best_leaf_candidate( [ tree_input, tree_targets, tree_supervision, target_cube, nodes,
							feature_positions, output_positions, output_cube_positions, ], [rows, features, outputs, internal_nodes],
						tree_lanes, domain, )?; let enabled = self.tensor(DType::I32, shape(&[1])?)?; self.emit_elementwise(
					vec![best_gain], vec![enabled], tree_lightgbm_positive_gain_program()?, domain, )?;
				let tree_offset = tree * internal_nodes; let destination = self.tensor(DType::I32, shape(&[1])?)?;
				self.emit_elementwise( vec![best_node], vec![destination], tree_lightgbm_destination_program(tree_offset)?, domain,
				)?; let old_feature = self.gather(split_features, destination, shape(&[1])?, 0, domain)?;
				let old_threshold = self.gather(split_thresholds, destination, shape(&[1])?, 0, domain)?;
				let written_feature = self.tensor(DType::I32, shape(&[1])?)?;
				let written_threshold = self.tensor(DType::F32, shape(&[1])?)?; self.emit_elementwise(
					vec![old_feature, best_feature, enabled], vec![written_feature], tree_lightgbm_select_i32_program()?, domain, )?;
				self.emit_elementwise( vec![old_threshold, best_threshold, enabled], vec![written_threshold],
					tree_lightgbm_select_f32_program()?, domain, )?;
				let next_split_features = self.tensor(DType::I32, shape(&[split_elements])?)?; self.emit(
					vec![split_features, destination, written_feature], vec![next_split_features], PrimitiveKind::Scatter(Scatter {
						axis: 0, bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1),
					domain, )?; let next_split_thresholds = self.tensor(DType::F32, shape(&[split_elements])?)?; self.emit(
					vec![split_thresholds, destination, written_threshold], vec![next_split_thresholds],
					PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices,
					}), forbidden_aliases(3, 1), domain, )?; split_features = next_split_features;
				split_thresholds = next_split_thresholds;

				let feature_indices = self.tensor(DType::I32, shape(&[rows, 1])?)?; self.emit_elementwise(
					vec![row_positions, best_feature], vec![feature_indices], tree_row_feature_index_program(features)?, domain, )?;
				let selected_values = self.gather( flat_features, feature_indices, shape(&[rows, 1])?, 0, domain, )?;
				let next_nodes = self.tensor(DType::I32, shape(&[rows, 1])?)?; self.emit_elementwise(
					vec![selected_values, best_threshold, nodes, best_node, enabled], vec![next_nodes],
					tree_lightgbm_next_node_program(internal_nodes)?, domain, )?; nodes = next_nodes; } }
		Ok((split_features, split_thresholds)) }

	fn compile_supervised_tree_structure( &mut self, values: [ValueId; 3], rows: u64, widths: [NonZeroU64; 3],
		declaration: DenseTree, tree_lanes: u32, random_seed: u64, ) -> TrainingCompileResult<(ValueId, ValueId)> {
		let [input, targets, supervision] = values; let [input_width, output_width, internal_nodes_per_tree] = widths;
		if declaration.family() == DenseTreeFamily::LightGbm { return self.compile_supervised_lightgbm_tree_structure( values,
				rows, widths, declaration, tree_lanes, random_seed, ); }
		let domain = IterationDomain::first(); let features = input_width.get(); let outputs = output_width.get();
		let trees = declaration.trees().get(); let split_elements = checked_product( &[trees, internal_nodes_per_tree.get()],
			"tree split structure size",
		)?; let mut split_features = self.zero_i32_tensor(shape(&[split_elements])?)?;
		let mut split_thresholds = self.zero_f32_tensor(shape(&[split_elements])?)?; let feature_positions = self.index_map(
			shape(&[1, features])?, IndexMap { start: 0, element_step: 1, iteration_step: 0, modulus: None, }, domain, )?;
		let output_positions_flat = self.index_map( shape(&[outputs])?, IndexMap { start: 0, element_step: 1,
				iteration_step: 0, modulus: None, }, domain, )?;
		let output_positions = self.gather_flat_as(output_positions_flat, shape(&[1, outputs])?, domain)?;
		let output_cube_positions = self.gather_flat_as(output_positions_flat, shape(&[1, 1, outputs])?, domain)?;

		for tree in 0..trees { let (tree_input, tree_targets, tree_supervision) = if trees == 1 {
				(input, targets, supervision) } else { self.bootstrap_tree_rows( [input, targets, supervision],
					[rows, features, outputs], random_seed, tree, domain, )? };
			let (flat_features, _) = self.pack_matrix_to_flat(tree_input, rows, features, domain)?;
			let (flat_targets, _) = self.pack_matrix_to_flat(tree_targets, rows, outputs, domain)?;
			let target_cube = self.gather_flat_as(flat_targets, shape(&[rows, 1, outputs])?, domain)?;
			let catboost_thresholds = if declaration.family() == DenseTreeFamily::CatBoost {
				Some(self.supervised_global_feature_thresholds( [tree_input, tree_supervision, feature_positions], [rows, features],
					tree_lanes, domain, )?) } else { None }; let mut nodes = self.zero_i32_tensor(shape(&[rows, 1])?)?;
			for depth in 0..declaration.depth().get() { let level_nodes = 1u64 << depth; let level_base = level_nodes - 1;
				let node_features = checked_product(&[level_nodes, features], "tree node-feature count")?;
				let node_outputs = checked_product(&[level_nodes, outputs], "tree node-output count")?;
				let node_feature_outputs = checked_product( &[level_nodes, features, outputs],
					"tree node-feature-output count",
				)?; let relative_nodes = self.tensor(DType::I32, shape(&[rows, 1])?)?; self.emit_elementwise( vec![nodes],
					vec![relative_nodes], tree_relative_node_program(level_base, level_nodes)?, domain, )?;
				let candidate_indices = self.tensor(DType::I32, shape(&[rows, features])?)?; self.emit_elementwise(
					vec![relative_nodes, feature_positions], vec![candidate_indices], tree_candidate_index_program(features)?, domain,
				)?; let candidate_thresholds = if let Some(global_thresholds) = catboost_thresholds {
					let candidate_features = self.index_map( shape(&[node_features])?, IndexMap { start: 0, element_step: 1,
							iteration_step: 0,
							modulus: Some(checked_i32(features, "CatBoost feature count")?),
						}, domain, )?; self.gather( global_thresholds, candidate_features, shape(&[node_features])?, 0, domain, )?
				} else { let feature_contributions = self.tensor(DType::F32, shape(&[rows, features])?)?;
					let count_contributions = self.tensor(DType::F32, shape(&[rows, features])?)?; self.emit_elementwise(
						vec![tree_input, tree_supervision], vec![feature_contributions], multiply_program()?, domain, )?;
					self.emit_elementwise( vec![tree_supervision, feature_positions], vec![count_contributions],
						repeat_f32_with_i32_program()?, domain, )?; let feature_sums = self.deterministic_segment_sum( candidate_indices,
						feature_contributions, node_features, rows, tree_lanes, domain, )?;
					let candidate_counts = self.deterministic_segment_sum( candidate_indices, count_contributions, node_features, rows,
						tree_lanes, domain, )?; let thresholds = self.elementwise_f32( shape(&[node_features])?,
						vec![feature_sums, candidate_counts], tree_safe_mean_program()?, domain, )?; thresholds };
				let row_thresholds = self.gather( candidate_thresholds, candidate_indices, shape(&[rows, features])?, 0, domain, )?;
				let left_weights = self.elementwise_f32( shape(&[rows, features])?,
					vec![tree_input, row_thresholds, tree_supervision], tree_left_weight_program()?, domain, )?;

				let parent_counts = self.deterministic_segment_sum( relative_nodes, tree_supervision, level_nodes, rows, tree_lanes,
					domain, )?; let parent_indices = self.tensor(DType::I32, shape(&[rows, outputs])?)?; self.emit_elementwise(
					vec![relative_nodes, output_positions], vec![parent_indices], tree_parent_output_index_program(outputs)?, domain,
				)?; let masked_targets = self.elementwise_f32( shape(&[rows, outputs])?, vec![tree_targets, tree_supervision],
					multiply_program()?, domain, )?; let parent_sums = self.deterministic_segment_sum( parent_indices, masked_targets,
					node_outputs, rows, tree_lanes, domain, )?;

				let (flat_left_weights, _) = self.pack_matrix_to_flat(left_weights, rows, features, domain)?; let left_weight_cube =
					self.gather_flat_as(flat_left_weights, shape(&[rows, features, 1])?, domain)?;
				let left_target_contributions = self.elementwise_f32( shape(&[rows, features, outputs])?,
					vec![target_cube, left_weight_cube], multiply_program()?, domain, )?; let (flat_candidate_indices, _) =
					self.pack_i32_matrix_to_flat(candidate_indices, rows, features, domain)?; let candidate_index_cube =
					self.gather_flat_as(flat_candidate_indices, shape(&[rows, features, 1])?, domain)?;
				let left_output_indices = self.tensor(DType::I32, shape(&[rows, features, outputs])?)?; self.emit_elementwise(
					vec![candidate_index_cube, output_cube_positions], vec![left_output_indices], tree_leaf_index_program(outputs)?,
					domain, )?; let left_sums = self.deterministic_segment_sum( left_output_indices, left_target_contributions,
					node_feature_outputs, rows, tree_lanes, domain, )?; let left_counts = self.deterministic_segment_sum(
					candidate_indices, left_weights, node_features, rows, tree_lanes, domain, )?;

				let parent_sum_cube = self.gather_flat_as(parent_sums, shape(&[level_nodes, 1, outputs])?, domain)?;
				let parent_count_cube = self.gather_flat_as(parent_counts, shape(&[level_nodes, 1, 1])?, domain)?;
				let left_sum_cube = self.gather_flat_as(left_sums, shape(&[level_nodes, features, outputs])?, domain)?;
				let left_count_cube = self.gather_flat_as(left_counts, shape(&[level_nodes, features, 1])?, domain)?;
				let gain_contributions = self.elementwise_f32( shape(&[level_nodes, features, outputs])?, vec![ left_sum_cube,
						parent_sum_cube, left_count_cube, parent_count_cube, ],
					tree_variance_gain_program(if declaration.family() == DenseTreeFamily::CatBoost { 0.0 } else { f32::MIN })?,
					domain, )?; let gains = self.sum_tensor( gain_contributions, shape(&[level_nodes, features])?, &[2], tree_lanes,
					domain, )?; let (best_features, selected_thresholds) = if let Some(global_thresholds) = catboost_thresholds
				{ let feature_gains = self.sum_tensor(gains, shape(&[features])?, &[0], tree_lanes, domain)?;
					let shared_gain = self.tensor(DType::F32, shape(&[1])?)?;
					let shared_feature = self.tensor(DType::I32, shape(&[1])?)?; self.emit( vec![feature_gains],
						vec![shared_gain, shared_feature], PrimitiveKind::Reduce(Reduce { operator: ReduceOperator::Maximum,
							axes: AxisSet::new(vec![0])?, keep_dimensions: false, result: ReduceResult::ValueAndIndex, tree_lanes, }),
						forbidden_aliases(1, 2), domain, )?; let shared_indices = self.zero_i32_tensor(shape(&[level_nodes])?)?;
					let best_features = self.gather( shared_feature, shared_indices, shape(&[level_nodes])?, 0, domain, )?;
					let selected_thresholds = self.gather( global_thresholds, best_features, shape(&[level_nodes])?, 0, domain, )?;
					(best_features, selected_thresholds) } else { let best_gains = self.tensor(DType::F32, shape(&[level_nodes])?)?;
					let best_features = self.tensor(DType::I32, shape(&[level_nodes])?)?; self.emit( vec![gains],
						vec![best_gains, best_features], PrimitiveKind::Reduce(Reduce { operator: ReduceOperator::Maximum,
							axes: AxisSet::new(vec![1])?, keep_dimensions: false, result: ReduceResult::ValueAndIndex, tree_lanes, }),
						forbidden_aliases(1, 2), domain, )?; let node_positions = self.index_map( shape(&[level_nodes])?, IndexMap {
							start: 0, element_step: 1, iteration_step: 0, modulus: None, }, domain, )?;
					let selected_threshold_indices = self.tensor(DType::I32, shape(&[level_nodes])?)?; self.emit_elementwise(
						vec![node_positions, best_features], vec![selected_threshold_indices], tree_candidate_index_program(features)?,
						domain, )?; let selected_thresholds = self.gather( candidate_thresholds, selected_threshold_indices,
						shape(&[level_nodes])?, 0, domain, )?; (best_features, selected_thresholds) };
				let destination_start = tree * internal_nodes_per_tree.get() + level_base; let split_destinations = self.index_map(
					shape(&[level_nodes])?, IndexMap {
						start: checked_i32(destination_start, "tree split destination")?,
						element_step: 1, iteration_step: 0, modulus: None, }, domain, )?;
				let next_split_features = self.tensor(DType::I32, shape(&[split_elements])?)?; self.emit(
					vec![split_features, split_destinations, best_features], vec![next_split_features],
					PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices,
					}), forbidden_aliases(3, 1), domain, )?;
				let next_split_thresholds = self.tensor(DType::F32, shape(&[split_elements])?)?; self.emit(
					vec![split_thresholds, split_destinations, selected_thresholds], vec![next_split_thresholds],
					PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices,
					}), forbidden_aliases(3, 1), domain, )?; split_features = next_split_features;
				split_thresholds = next_split_thresholds;

				let selected_features_by_row = self.gather(best_features, relative_nodes, shape(&[rows, 1])?, 0, domain)?;
				let selected_thresholds_by_row = self.gather( selected_thresholds, relative_nodes, shape(&[rows, 1])?, 0, domain,
				)?; let row_positions = self.index_map( shape(&[rows, 1])?, IndexMap { start: 0, element_step: 1, iteration_step: 0,
						modulus: None, }, domain, )?; let feature_indices = self.tensor(DType::I32, shape(&[rows, 1])?)?;
				self.emit_elementwise( vec![row_positions, selected_features_by_row], vec![feature_indices],
					tree_row_feature_index_program(features)?, domain, )?; let selected_values = self.gather( flat_features,
					feature_indices, shape(&[rows, 1])?, 0, domain, )?; let next_nodes = self.tensor(DType::I32, shape(&[rows, 1])?)?;
				self.emit_elementwise( vec![selected_values, selected_thresholds_by_row, nodes], vec![next_nodes],
					tree_next_node_program(internal_nodes_per_tree.get())?, domain, )?; nodes = next_nodes; } }
		Ok((split_features, split_thresholds)) }

	fn materialize_tree_predictions( &mut self, values: [ValueId; 4], rows: u64, widths: [NonZeroU64; 2],
		declaration: DenseTree, tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		let [flat_features, split_features, split_thresholds, leaf_values] = values; let [input_width, output_width] = widths;
		let requirements = tree_ensemble_inference_requirements( rows, input_width.get(), declaration.trees().get(),
			declaration.depth().get(), output_width.get(), )?;
		let predictions = self.tensor(DType::F32, shape(&[rows, output_width.get()])?)?; let first_value = self.next_value;
		let first_kernel = self.next_kernel; self.next_value = self .next_value .checked_add(requirements.intermediate_values)
			.ok_or_else(identity_exhausted)?; self.next_kernel = self .next_kernel .checked_add(requirements.kernels)
			.ok_or_else(identity_exhausted)?; let request = TreeEnsembleInferenceRequest {
			features: self.tensor_ref(flat_features)?.clone(), split_features: self.tensor_ref(split_features)?.clone(),
			split_thresholds: self.tensor_ref(split_thresholds)?.clone(), leaf_values: self.tensor_ref(leaf_values)?.clone(),
			predictions: self.tensor_ref(predictions)?.clone(), rows, feature_width: input_width.get(),
			trees: declaration.trees().get(), depth: declaration.depth().get(), outputs: output_width.get(), scale: 1.0,
			tree_lanes, identity_namespace: IdentityNamespace::new( ValueId::new(first_value), requirements.intermediate_values,
				KernelTemplateId::new(first_kernel), requirements.kernels, ), workspace_limit: requirements.workspace_bytes, };
		let materialized = materialize_tree_ensemble_inference(&request)?;
		self.insert_materialized_graph(&materialized.graph, domain)?; Ok(predictions) }

	fn route_tree_leaf_indices( &mut self, values: [ValueId; 3], rows: u64, widths: [NonZeroU64; 4],
		declaration: DenseTree, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		let [flat_features, split_features, split_thresholds] = values;
		let [input_width, output_width, internal_nodes_per_tree, leaves_per_tree] = widths;
		let trees = declaration.trees().get(); let pair_shape = shape(&[rows, trees, 1])?;
		let pair_positions = self.index_map( pair_shape.clone(), IndexMap { start: 0, element_step: 1, iteration_step: 0,
				modulus: None, }, domain, )?; let mut nodes = self.index_map( pair_shape.clone(), IndexMap { start: 0,
				element_step: 0, iteration_step: 0, modulus: None, }, domain, )?; for _ in 0..declaration.depth().get() {
			let split_indices = self.tensor(DType::I32, pair_shape.clone())?; self.emit_elementwise( vec![pair_positions, nodes],
				vec![split_indices], tree_split_index_program(trees, internal_nodes_per_tree.get())?, domain, )?;
			let selected_features = self.gather(split_features, split_indices, pair_shape.clone(), 0, domain)?;
			let selected_thresholds = self.gather( split_thresholds, split_indices, pair_shape.clone(), 0, domain, )?;
			let feature_indices = self.tensor(DType::I32, pair_shape.clone())?; self.emit_elementwise(
				vec![pair_positions, selected_features], vec![feature_indices],
				tree_feature_index_program(trees, input_width.get())?, domain, )?; let selected_values = self.gather( flat_features,
				feature_indices, pair_shape.clone(), 0, domain, )?; let next_nodes = self.tensor(DType::I32, pair_shape.clone())?;
			self.emit_elementwise( vec![selected_values, selected_thresholds, nodes], vec![next_nodes],
				tree_next_node_program(internal_nodes_per_tree.get())?, domain, )?; nodes = next_nodes; }
		let leaf_bases = self.tensor(DType::I32, pair_shape)?; self.emit_elementwise( vec![pair_positions, nodes],
			vec![leaf_bases], tree_leaf_base_program( trees, internal_nodes_per_tree.get(), leaves_per_tree.get(),
				output_width.get(), )?, domain, )?; let output_offsets = self.index_map( shape(&[1, 1, output_width.get()])?,
			IndexMap { start: 0, element_step: 1, iteration_step: 0, modulus: None, }, domain, )?;
		let leaf_indices = self.tensor(DType::I32, shape(&[rows, trees, output_width.get()])?)?; self.emit_elementwise(
			vec![leaf_bases, output_offsets], vec![leaf_indices], tree_leaf_index_program(output_width.get())?, domain, )?;
		Ok(leaf_indices) }

	fn compile_embedding_forward( &mut self, input: ValueId, rows: u64, sequence: u64, dimensions: u64, table: ValueId,
		domain: IterationDomain, ) -> TrainingCompileResult<(ValueId, ValueId)> { self.require_tensor( input, DType::I32,
			&[rows, sequence],
			"embedding token matrix",
		)?;
		let token_rows = checked_product(&[rows, sequence], "embedding token count")?;
		let (indices, _) = self.pack_i32_matrix_to_flat(input, rows, sequence, domain)?;
		let gathered = self.gather(table, indices, shape(&[token_rows, dimensions])?, 0, domain)?;
		let flat = self.pack_contiguous_tensor_to_flat(gathered, domain)?;
		let output = self.gather_flat_as(flat, shape(&[rows, sequence * dimensions])?, domain)?; Ok((output, indices)) }

	fn compile_attention_forward( &mut self, input: ValueId, geometry: [u64; 5], parameters: AttentionForwardParameters,
		tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<(ValueId, AttentionForwardValues)> {
		let [rows, sequence, dimensions, heads, head_dimension] = geometry;
		let input_sequence = self.reinterpret_tensor(input, shape(&[rows, sequence, dimensions])?, domain)?;
		let query = self.attention_projection( input_sequence, parameters.query, rows, sequence, dimensions, domain, )?;
		let key = self.attention_projection( input_sequence, parameters.key, rows, sequence, dimensions, domain, )?;
		let value = self.attention_projection( input_sequence, parameters.value, rows, sequence, dimensions, domain, )?;
		let head_shape = shape(&[rows, sequence, heads, head_dimension])?;
		let queries = self.reinterpret_tensor(query, head_shape.clone(), domain)?;
		let keys = self.reinterpret_tensor(key, head_shape.clone(), domain)?;
		let values = self.reinterpret_tensor(value, head_shape, domain)?;
		let score_shape = shape(&[rows, heads, sequence, sequence])?;
		let scores = self.tensor(DType::F32, score_shape.clone())?; self.emit( vec![queries, keys], vec![scores],
			PrimitiveKind::Contraction(Contraction { batch_axes: vec![(0, 0), (2, 2)], contract_axes: vec![(3, 3)], }),
			forbidden_aliases(2, 1), domain, )?; let scaled = self.elementwise_f32( score_shape, vec![scores],
			multiply_constant_program(1.0 / (head_dimension as f32).sqrt())?, domain, )?;
		let probabilities = self.causal_softmax(scaled, [rows, heads, sequence], tree_lanes, domain)?;
		let head_context = self.tensor(DType::F32, shape(&[rows, heads, sequence, head_dimension])?)?; self.emit(
			vec![probabilities, values], vec![head_context], PrimitiveKind::Contraction(Contraction {
				batch_axes: vec![(0, 0), (1, 2)], contract_axes: vec![(3, 1)], }), forbidden_aliases(2, 1), domain, )?;
		let context_heads = self.head_major_to_sequence(head_context, rows, sequence, heads, head_dimension, domain)?;
		let context = self.reinterpret_tensor(context_heads, shape(&[rows, sequence, dimensions])?, domain)?;
		let projected = self.attention_projection( context, parameters.output, rows, sequence, dimensions, domain, )?; Ok((
			self.reinterpret_tensor(projected, shape(&[rows, sequence * dimensions])?, domain)?, AttentionForwardValues {
				input: input_sequence, queries, keys, values, probabilities, context, }, )) }

	fn gather_matrix_column( &mut self, matrix: ValueId, rows: u64, columns: u64, column: u64, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		self.require_tensor(matrix, DType::F32, &[rows, columns], "matrix column source")?;
		let index = self.index_map( shape(&[1])?, IndexMap {
				start: checked_i32(column, "matrix column")?,
				element_step: 0, iteration_step: 0, modulus: None, }, IterationDomain::first(), )?;
		let gathered = self.gather(matrix, index, shape(&[rows, 1])?, 1, domain)?; Ok(gathered) }

	fn attention_projection( &mut self, input: ValueId, weight: ValueId, rows: u64, sequence_length: u64, dimensions: u64,
		domain: IterationDomain, ) -> TrainingCompileResult<ValueId> { self.require_tensor( input, DType::F32,
			&[rows, sequence_length, dimensions],
			"attention projection input",
		)?; self.require_tensor( weight, DType::F32, &[dimensions, dimensions],
			"attention projection weight",
		)?; let projected = self.tensor(DType::F32, shape(&[rows, sequence_length, dimensions])?)?; self.emit(
			vec![input, weight], vec![projected], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(2, 0)], }), forbidden_aliases(2, 1), domain, )?; Ok(projected) }

	fn compile_convolution_forward( &mut self, values: [ValueId; 3], rows: u64, geometry: DenseConvolutionGeometry,
		window_role: ExternalInputRole, domain: IterationDomain, ) -> TrainingCompileResult<( ValueId,
		ChannelwiseConvolution1dPreparation, ValueId, ValueId, )> { let [input, weight, bias] = values;
		let input_width = geometry .input_width()
			.expect("validated convolution input width");
		let output_width = geometry .output_width()
			.expect("validated convolution output width");
		self.require_tensor( input, DType::F32, &[rows, input_width.get()],
			"convolution input",
		)?; self.require_tensor( weight, DType::F32, &[ geometry.kernel().get(), geometry.input_channels().get(),
				geometry.filters().get(), ],
			"convolution weight",
		)?; self.require_tensor( bias, DType::F32, &[geometry.filters().get()],
			"convolution bias",
		)?; let preparation = prepare_channelwise_convolution_1d( rows, geometry.input_length().get(),
			geometry.input_channels().get(), geometry.filters().get(), geometry.kernel().get(), )?;
		let (flat_input, _) = self.pack_matrix_to_flat(input, rows, input_width.get(), domain)?;
		let window_indices = self.external_i32_tensor( window_role, shape(&preparation.window_indices_shape())?,
			preparation.window_indices(), )?; let columns = self.gather( flat_input, window_indices,
			shape(&preparation.window_indices_shape())?, 0, domain, )?;
		let contracted = self.tensor(DType::F32, shape(&preparation.output_shape())?)?; self.emit( vec![columns, weight],
			vec![contracted], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(2, 0), (3, 1)], }), forbidden_aliases(2, 1), domain, )?; let grouped = self.elementwise_f32(
			shape(&preparation.output_shape())?, vec![contracted, bias], sum_program(2)?, domain, )?;
		let (output, group_indices) = self.unpack_pool_to_matrix( grouped, rows, geometry.output_length().get(),
			geometry.filters().get(), output_width.get(), domain, )?; Ok((output, preparation, columns, group_indices)) }

	fn compile_pool_forward( &mut self, input: ValueId, geometry: [u64; 2], state: DensePoolState,
		roles: [ExternalInputRole; 2], tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<( ValueId,
		ChannelwiseMaxPool1dPreparation, ValueId, ValueId, ValueId, )> { let [rows, pool_size] = geometry;
		let [window_role, winner_role] = roles;
		let input_width = state.input_width().expect("validated pool input width");
		let output_width = state.output_width().expect("validated pool output width");
		self.require_tensor( input, DType::F32, &[rows, input_width.get()],
			"maximum-pool input",
		)?; let preparation = prepare_channelwise_max_pool_1d( rows, state.input_length().get(), state.channels().get(),
			pool_size, )?; let (flat_input, input_indices) = self.pack_matrix_to_flat(input, rows, input_width.get(), domain)?;
		let window_indices = self.external_i32_tensor( window_role, shape(&preparation.window_indices_shape())?,
			preparation.window_indices(), )?; let winner_bases = self.external_i32_tensor( winner_role,
			shape(&preparation.output_shape())?, preparation.winner_bases(), )?;
		let pooled = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		let winners = self.tensor(DType::I32, shape(&preparation.output_shape())?)?; self.materialize(
			"recipe_max_pool_1d",
			&[
				("values", flat_input),
				("window_indices", window_indices),
				("winner_bases", winner_bases),
			],
			&[("pooled", pooled), ("winning_indices", winners)],
			"values",
			&preparation.forward_parameters(u64::from(tree_lanes)), domain, )?;
		let (output, group_indices) = self.unpack_pool_to_matrix( pooled, rows, state.output_length().get(),
			state.channels().get(), output_width.get(), domain, )?;
		Ok((output, preparation, winners, input_indices, group_indices)) }

	fn compile_training_operation( &mut self, input: ValueId, operation: DenseOperation, width: u64,
		context: (u64, ValueId, ValueId, &DenseTrainingConfig), ) -> TrainingCompileResult<(ValueId, OperationValues)> {
		let (partition_rows, validity, valid_count, config) = context; let output_shape = shape(&[partition_rows, width])?;
		let safe_input = self.mask_f32_with_zero(input, validity, self.training_domain)?; let parameter = match operation {
			DenseOperation::Activation(activation) if activation.learned_parameters() == 1 => {
				Some(self.initialize_constant_parameter(shape(&[1])?, 0.25)?) }
			DenseOperation::Activation(_) | DenseOperation::Normalization(_) => None, };
		let (raw_output, variance) = match operation { DenseOperation::Activation(activation) => ( self.apply_activation(
					safe_input, activation, parameter.map(|parameter| parameter.value), output_shape, self.training_domain, )?, None,
			), DenseOperation::Normalization(normalization) => { let normalized = self.normalize_training( safe_input,
					normalization, [partition_rows, width], [validity, valid_count], config.normalization_epsilon,
					config.reduction_tree_lanes, )?; (normalized.normalized, Some(normalized.variance)) } };
		let output = self.mask_f32_with_zero(raw_output, validity, self.training_domain)?; Ok(( output, OperationValues {
				operation, input: safe_input, output, variance, parameter, }, )) }

	fn bias_free_linear( &mut self, input: ValueId, weight: ValueId, rows: u64, output_width: u64, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let output = self.tensor(DType::F32, shape(&[rows, output_width])?)?; self.emit(
			vec![input, weight], vec![output], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(1, 0)], }), forbidden_aliases(2, 1), domain, )?; Ok(output) }

	fn exact_add( &mut self, left: ValueId, right: ValueId, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		let left_dtype = self.tensor_ref(left)?.dtype; let left_shape = self.tensor_ref(left)?.shape.clone();
		let output = self.tensor(left_dtype, left_shape)?;
		self.emit_elementwise(vec![left, right], vec![output], sum_program(2)?, domain)?; Ok(output) }

	fn require_tensor( &self, value: ValueId, _dtype: DType, _extents: &[u64], _role: &str,
	) -> TrainingCompileResult<()> { self.tensor_ref(value)?; Ok(()) }

	fn identity_indices(&mut self, tensor_shape: Shape) -> TrainingCompileResult<ValueId> {
		checked_i32(tensor_shape.elements(), "identity index element count")?;
		let indices = self.index_map( tensor_shape, IndexMap { start: 0, element_step: 1, iteration_step: 0, modulus: None, },
			IterationDomain::first(), )?; Ok(indices) }

	fn reinterpret_tensor( &mut self, input: ValueId, output_shape: Shape, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let flat = self.pack_contiguous_tensor_to_flat(input, domain)?;
		self.gather_flat_as(flat, output_shape, domain) }

	fn head_major_to_sequence( &mut self, input: ValueId, rows: u64, sequence_length: u64, heads: u64, head_dimension: u64,
		domain: IterationDomain, ) -> TrainingCompileResult<ValueId> { self.require_tensor( input, DType::F32,
			&[rows, heads, sequence_length, head_dimension],
			"head-major attention tensor",
		)?; let output_shape = shape(&[rows, sequence_length, heads, head_dimension])?;
		let positions = self.identity_indices(output_shape.clone())?;
		let indices = self.tensor(DType::I32, output_shape.clone())?; self.emit_elementwise( vec![positions], vec![indices],
			head_major_source_index_program(sequence_length, heads, head_dimension)?, IterationDomain::first(), )?;
		let flat = self.pack_contiguous_tensor_to_flat(input, domain)?;
		let output = self.gather(flat, indices, output_shape, 0, domain)?; Ok(output) }

	fn sequence_to_head_major( &mut self, input: ValueId, rows: u64, sequence_length: u64, heads: u64, head_dimension: u64,
		domain: IterationDomain, ) -> TrainingCompileResult<ValueId> { self.require_tensor( input, DType::F32,
			&[rows, sequence_length, heads, head_dimension],
			"sequence-major attention tensor",
		)?; let output_shape = shape(&[rows, heads, sequence_length, head_dimension])?;
		let positions = self.identity_indices(output_shape.clone())?;
		let indices = self.tensor(DType::I32, output_shape.clone())?; self.emit_elementwise( vec![positions], vec![indices],
			sequence_major_source_index_program(sequence_length, heads, head_dimension)?, IterationDomain::first(), )?;
		let flat = self.pack_contiguous_tensor_to_flat(input, domain)?;
		let output = self.gather(flat, indices, output_shape, 0, domain)?; Ok(output) }

	fn causal_softmax( &mut self, scores: ValueId, geometry: [u64; 3], tree_lanes: u32, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let [rows, heads, sequence_length] = geometry; self.require_tensor( scores,
			DType::F32, &[rows, heads, sequence_length, sequence_length],
			"causal-attention scores",
		)?;
		let attention_rows = checked_product(&[rows, heads, sequence_length], "causal softmax row count")?;
		let row_shape = shape(&[attention_rows, sequence_length])?;
		let values = self.reinterpret_tensor(scores, row_shape.clone(), domain)?;
		let positions = self.identity_indices(row_shape.clone())?; let mask = self.tensor(DType::I32, row_shape.clone())?;
		self.emit_elementwise( vec![positions], vec![mask], causal_mask_program(sequence_length)?, IterationDomain::first(),
		)?; let softmax = self.tensor(DType::F32, row_shape)?; let parameters = [
			("rows".to_owned(), PreparedParameter::U64(attention_rows)),
			(
				"columns".to_owned(),
				PreparedParameter::U64(sequence_length), ), (
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)), ), (
				"causal_mask_verified".to_owned(),
				PreparedParameter::Bool(true), ), ] .into_iter() .collect::<PreparedParameters>(); self.materialize(
			"gpu_causal_softmax_rows",
			&[("values", values), ("causal_mask", mask)],
			&[("softmax", softmax)],
			"values",
			&parameters, domain, )?; self.reinterpret_tensor( softmax, shape(&[rows, heads, sequence_length, sequence_length])?,
			domain, ) }

	fn zero_f32_tensor(&mut self, tensor_shape: Shape) -> TrainingCompileResult<ValueId> { let seed = self.index_map(
			tensor_shape.clone(), IndexMap { start: 0, element_step: 0, iteration_step: 0, modulus: None, },
			IterationDomain::first(), )?; let output = self.elementwise_f32( tensor_shape, vec![seed], zero_outputs_program(1)?,
			IterationDomain::first(), )?; Ok(output) }

	fn zero_i32_tensor(&mut self, tensor_shape: Shape) -> TrainingCompileResult<ValueId> { let output = self.index_map(
			tensor_shape, IndexMap { start: 0, element_step: 0, iteration_step: 0, modulus: None, }, IterationDomain::first(),
		)?; Ok(output) }

	fn gather_flat_as( &mut self, flat: ValueId, output_shape: Shape, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let indices = self.identity_indices(output_shape.clone())?;
		self.gather(flat, indices, output_shape, 0, domain) }

	fn deterministic_segment_sum( &mut self, indices: ValueId, updates: ValueId, segments: u64,
		maximum_segment_length: u64, tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		let index_contract = self.tensor_ref(indices)?.clone(); let elements = index_contract.shape.elements();
		let flat_indices = self.pack_contiguous_tensor_to_flat(indices, domain)?;
		let flat_updates = self.pack_contiguous_tensor_to_flat(updates, domain)?;
		let output = self.tensor(DType::F32, shape(&[segments])?)?; let parameters = [
			("elements".to_owned(), PreparedParameter::U64(elements)),
			("segments".to_owned(), PreparedParameter::U64(segments)),
			(
				"maximum_segment_length".to_owned(),
				PreparedParameter::U64(maximum_segment_length), ), (
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)), ), ] .into_iter() .collect::<PreparedParameters>(); self.materialize(
			"gpu_segment_sum",
			&[("values", flat_updates), ("segment_ids", flat_indices)],
			&[("sums", output)],
			"values",
			&parameters, domain, )?; Ok(output) }

	fn pack_contiguous_tensor_to_flat( &mut self, input: ValueId, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let contract = self.tensor_ref(input)?.clone(); if contract.shape.rank() == 1 {
			return Ok(input); }
		let elements = contract.shape.elements(); let indices = self.identity_indices(contract.shape.clone())?;
		let flat_shape = shape(&[elements])?; let base = match contract.dtype {
			DType::F32 => self.zero_f32_tensor(flat_shape.clone())?, DType::I32 => self.zero_i32_tensor(flat_shape.clone())?, };
		let flat = self.tensor(contract.dtype, flat_shape)?; self.emit( vec![base, indices, input], vec![flat],
			PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices, }),
			forbidden_aliases(3, 1), domain, )?; Ok(flat) }

	fn pack_matrix_to_flat( &mut self, input: ValueId, rows: u64, width: u64, domain: IterationDomain,
	) -> TrainingCompileResult<(ValueId, ValueId)> { self.pack_matrix(input, rows, width, DType::F32, domain) }

	fn pack_i32_matrix_to_flat( &mut self, input: ValueId, rows: u64, width: u64, domain: IterationDomain,
	) -> TrainingCompileResult<(ValueId, ValueId)> { self.pack_matrix(input, rows, width, DType::I32, domain) }

	fn pack_matrix( &mut self, input: ValueId, rows: u64, width: u64, dtype: DType, domain: IterationDomain,
	) -> TrainingCompileResult<(ValueId, ValueId)> {
		self.require_tensor(input, dtype, &[rows, width], "matrix pack input")?;
		let elements = checked_product(&[rows, width], "matrix pack element count")?;
		let matrix_indices = self.identity_indices(shape(&[rows, width])?)?; let flat_shape = shape(&[elements])?;
		let base = match dtype { DType::F32 => self.zero_f32_tensor(flat_shape.clone())?,
			DType::I32 => self.zero_i32_tensor(flat_shape.clone())?, }; let flat = self.tensor(dtype, flat_shape)?; self.emit(
			vec![base, matrix_indices, input], vec![flat], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), domain, )?; Ok((flat, matrix_indices)) }

	fn unpack_pool_to_matrix( &mut self, pooled: ValueId, partition_rows: u64, groups: u64, channels: u64,
		output_width: u64, domain: IterationDomain, ) -> TrainingCompileResult<(ValueId, ValueId)> { self.require_tensor(
			pooled, DType::F32, &[partition_rows, groups, channels],
			"pool grouped output",
		)?; let group_indices = self.identity_indices(shape(&[groups, channels])?)?;
		let base = self.zero_f32_tensor(shape(&[partition_rows, output_width])?)?;
		let output = self.tensor(DType::F32, shape(&[partition_rows, output_width])?)?; self.emit(
			vec![base, group_indices, pooled], vec![output], PrimitiveKind::Scatter(Scatter { axis: 1,
				bounds: IndexBounds::Reject, conflict: ScatterConflict::UniqueIndices, }), forbidden_aliases(3, 1), domain, )?;
		Ok((output, group_indices)) }

	fn group_routing_mask( &mut self, input_width: u64, output_width: u64, channels: NonZeroU64,
		routing: DenseGroupToNeuronRouting, ) -> TrainingCompileResult<ValueId> {
		let (groups, neurons) = routing_extents(routing); let _ = (groups, neurons);
		let weight_shape = shape(&[input_width, output_width])?; let positions = self.identity_indices(weight_shape.clone())?;
		let mask = self.elementwise_f32( weight_shape, vec![positions], group_routing_mask_program(channels, routing)?,
			IterationDomain::first(), )?; Ok(mask) }

	fn initialize_weight( &mut self, parameter_shape: Shape, fan_in: u64, seed: u64,
	) -> TrainingCompileResult<InitialParameter> { let resumed = self.resume_parameter_inputs(parameter_shape.clone())?;
		let stream = self.next_weight_stream; self.next_weight_stream = stream.checked_add(1).ok_or_else(identity_exhausted)?;
		let random = self.tensor(DType::F32, parameter_shape.clone())?; self.emit( Vec::new(), vec![random],
			PrimitiveKind::Random(RandomMap { distribution: RandomDistribution::NormalF32, key: RandomKey { seed_low: seed,
					seed_high: seed.rotate_left(29) ^ 0x9e37_79b9_7f4a_7c15, stream, }, philox_rounds: 10, }), Vec::new(),
			IterationDomain::first(), )?; let fresh_value = self.tensor(DType::F32, parameter_shape.clone())?;
		let scale = (2.0f32 / fan_in as f32).sqrt(); self.emit_elementwise( vec![random], vec![fresh_value],
			multiply_constant_program(scale)?, IterationDomain::first(), )?;
		let (fresh_first_moment, fresh_second_moment) = self.initialize_zero_pair(parameter_shape.clone())?;
		self.select_resume_parameter( InitialParameter { value: fresh_value, first_moment: fresh_first_moment,
				second_moment: fresh_second_moment, }, resumed, parameter_shape, ) }

	fn initialize_zero_parameter(&mut self, parameter_shape: Shape) -> TrainingCompileResult<InitialParameter> { self.initialize_parameter(parameter_shape, zero_outputs_program(3)?) }

	fn initialize_constant_parameter( &mut self, parameter_shape: Shape, value: f32,
	) -> TrainingCompileResult<InitialParameter> {
		self.initialize_parameter(parameter_shape, constant_parameter_program(value)?) }

	fn initialize_parameter( &mut self, parameter_shape: Shape, program: recipe_core::ScalarProgram,
	) -> TrainingCompileResult<InitialParameter> { let resumed = self.resume_parameter_inputs(parameter_shape.clone())?;
		let seed = self.index_map( parameter_shape.clone(), IndexMap { start: 0, element_step: 0, iteration_step: 0,
				modulus: None, }, IterationDomain::first(), )?; let fresh_value = self.tensor(DType::F32, parameter_shape.clone())?;
		let fresh_first_moment = self.tensor(DType::F32, parameter_shape.clone())?;
		let fresh_second_moment = self.tensor(DType::F32, parameter_shape.clone())?; self.emit_elementwise( vec![seed],
			vec![fresh_value, fresh_first_moment, fresh_second_moment], program, IterationDomain::first(), )?;
		self.select_resume_parameter( InitialParameter { value: fresh_value, first_moment: fresh_first_moment,
				second_moment: fresh_second_moment, }, resumed, parameter_shape, ) }

	fn resume_parameter_inputs(&mut self, parameter_shape: Shape) -> TrainingCompileResult<InitialParameter> {
		let ordinal = self.next_parameter_input;
		self.next_parameter_input = ordinal.checked_add(1).ok_or_else(identity_exhausted)?; Ok(InitialParameter {
			value: self.external_f32_zeros( ExternalInputRole::ResumeParameter { ordinal }, parameter_shape.clone(), )?,
			first_moment: self.external_f32_zeros( ExternalInputRole::ResumeFirstMoment { ordinal }, parameter_shape.clone(), )?,
			second_moment: self.external_f32_zeros( ExternalInputRole::ResumeSecondMoment { ordinal }, parameter_shape, )?, }) }

	fn select_resume_parameter( &mut self, fresh: InitialParameter, resumed: InitialParameter, parameter_shape: Shape,
	) -> TrainingCompileResult<InitialParameter> { Ok(InitialParameter {
			value: self.select_resume_tensor(fresh.value, resumed.value, parameter_shape.clone())?,
			first_moment: self.select_resume_tensor( fresh.first_moment, resumed.first_moment, parameter_shape.clone(), )?,
			second_moment: self.select_resume_tensor( fresh.second_moment, resumed.second_moment, parameter_shape, )?, }) }

	fn select_resume_tensor( &mut self, fresh: ValueId, resumed: ValueId, tensor_shape: Shape,
	) -> TrainingCompileResult<ValueId> { self.select_resume( fresh, resumed, tensor_shape, DType::F32,
			resume_tensor_program(DType::F32)?, ) }

	fn select_resume_i32_tensor( &mut self, fresh: ValueId, resumed: ValueId, tensor_shape: Shape,
	) -> TrainingCompileResult<ValueId> { self.select_resume( fresh, resumed, tensor_shape, DType::I32,
			resume_tensor_program(DType::I32)?, ) }

	fn select_resume( &mut self, fresh: ValueId, resumed: ValueId, tensor_shape: Shape, dtype: DType,
		program: recipe_core::ScalarProgram, ) -> TrainingCompileResult<ValueId> {
		let resume_enabled = self.resume_enabled.expect("compiler resume selector");
		let selected = self.tensor(dtype, tensor_shape)?; self.emit_elementwise( vec![fresh, resumed, resume_enabled],
			vec![selected], program, IterationDomain::first(), )?; Ok(selected) }

	fn initialize_zero_pair(&mut self, parameter_shape: Shape) -> TrainingCompileResult<(ValueId, ValueId)> {
		let seed = self.index_map( parameter_shape.clone(), IndexMap { start: 0, element_step: 0, iteration_step: 0,
				modulus: None, }, IterationDomain::first(), )?; let first = self.tensor(DType::F32, parameter_shape.clone())?;
		let second = self.tensor(DType::F32, parameter_shape)?; self.emit_elementwise( vec![seed], vec![first, second],
			zero_outputs_program(2)?, IterationDomain::first(), )?; Ok((first, second)) }

	fn compile_validation_blocks( &mut self, states: &[DenseBlockState], input: ValueId, rows: u64,
		config: &DenseTrainingConfig, domain: IterationDomain, ) -> TrainingCompileResult<(ValueId, u64)> {
		let mut current = input;
		let mut logical = LogicalFeatureShape::from_width(self.matrix_width(input, "validation input")?)?;
		let mut routing = None; for (block_index, state) in states.iter().enumerate() {
			let validation = state.realized().compile_validation( self, BlockValidationContext { input: current, logical,
					routing, rows, block_index, config, domain, }, )?; current = validation.output; logical = validation.logical;
			routing = validation.routing; }
		Ok((current, logical.width()?)) }

	fn apply_validation_operation( &mut self, input: ValueId, operation: DenseOperation, prelu: Option<ValueId>,
		dimensions: [u64; 2], config: &DenseTrainingConfig, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		let [rows, width] = dimensions; match operation { DenseOperation::Activation(activation) => {
				self.apply_activation(input, activation, prelu, shape(&[rows, width])?, domain) }
			DenseOperation::Normalization(normalization) => Ok(self .normalize_validation( input, normalization, [rows, width],
					config.normalization_epsilon, config.reduction_tree_lanes, domain, )? .normalized), } }

	fn compile_validation( &mut self, validation: ValidationValues, training_config: &DenseTrainingConfig,
		validation_config: &BinaryValidationConfig, block_states: &[DenseBlockState], bounds: &TrainingBounds,
	) -> TrainingCompileResult<BinaryValidationOutputs> {
		let validation_domain = IterationDomain::every(bounds.training_iterations);
		let (current, columns) = self.compile_validation_blocks( block_states, validation.features, validation.rows,
			training_config, validation_domain, )?; let logits = current; let matrix_shape = shape(&[validation.rows, columns])?;
		let probabilities_matrix = self.owned_scalar_f32( matrix_shape.clone(),
			"gpu_sigmoid_into",
			vec![logits], validation_domain, )?; let losses_matrix = self.tensor(DType::F32, matrix_shape.clone())?;
		let unused_gradient = self.tensor(DType::F32, matrix_shape)?; match training_config.loss {
			DenseLoss::BinaryCrossEntropy => self.materialize(
				"gpu_bce_with_logits",
				&[("logits", logits), ("targets", validation.targets)],
				&[("losses", losses_matrix), ("gradients", unused_gradient)],
				"logits",
				&PreparedParameters::new(), validation_domain, )?, DenseLoss::Focal => { self.emit_elementwise(
					vec![logits, validation.targets], vec![losses_matrix, unused_gradient], canonical_focal_with_logits_program()?,
					validation_domain, )?; }
			_ => unreachable!("validated binary objective"),
		}

		let metrics = if columns == 1 { let probabilities = self.matrix_column_vector( probabilities_matrix,
				[validation.rows, columns, 0], training_config.reduction_tree_lanes, validation_domain, )?;
			let targets = self.matrix_column_vector( validation.targets, [validation.rows, columns, 0],
				training_config.reduction_tree_lanes, validation_domain, )?; let losses = self.matrix_column_vector( losses_matrix,
				[validation.rows, columns, 0], training_config.reduction_tree_lanes, validation_domain, )?;
			self.materialize_binary_metrics( [probabilities, targets, losses], validation.rows,
				training_config.reduction_tree_lanes, validation_config, validation_domain, )? } else {
			self.materialize_multi_target_binary_metrics( [probabilities_matrix, validation.targets, losses_matrix],
				[validation.rows, columns], training_config.reduction_tree_lanes, validation_config, validation_domain, )? };
		let temperature_scaling = validation_config .temperature_scaling() .map(|temperature| {
				self.compile_temperature_scaling( logits, validation.targets, validation.rows, training_config.reduction_tree_lanes,
					temperature, bounds, ) }) .transpose()?; Ok(BinaryValidationOutputs { logits, metrics,
			metric_domain: validation_domain, temperature_scaling, }) }

	fn compile_multiclass_validation( &mut self, validation: ValidationValues, training_config: &DenseTrainingConfig,
		block_states: &[DenseBlockState], bounds: &TrainingBounds, task: DenseTask,
	) -> TrainingCompileResult<MulticlassValidationOutputs> {
		let validation_domain = IterationDomain::every(bounds.training_iterations);
		let (current, classes) = self.compile_validation_blocks( block_states, validation.features, validation.rows,
			training_config, validation_domain, )?; let logits = current; let matrix_shape = shape(&[validation.rows, classes])?;
		let losses = self.tensor(DType::F32, matrix_shape.clone())?;
		let unused_gradient = self.tensor(DType::F32, matrix_shape)?;
		if matches!(task, DenseTask::JointMulticlassClassification { .. }) { self.cross_entropy_with_dense_targets( logits,
				validation.targets, [losses, unused_gradient], [validation.rows, classes], training_config.reduction_tree_lanes,
				validation_domain, )?; } else { self.cross_entropy_with_logits( logits, validation.targets,
				[losses, unused_gradient], [validation.rows, classes], training_config.reduction_tree_lanes, validation_domain, )?;
		}
		let scalar_shape = shape(&[1])?; let loss_sum = self.sum_tensor( losses, scalar_shape.clone(), &[0, 1],
			training_config.reduction_tree_lanes, validation_domain, )?; let mean_cross_entropy = self.elementwise_f32(
			scalar_shape.clone(), vec![loss_sum], divide_constant_program(validation.rows as f32)?, validation_domain, )?;

		let parameters = [(
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(training_config.reduction_tree_lanes)), )] .into_iter()
		.collect::<PreparedParameters>(); let accuracy = self.tensor(DType::F32, scalar_shape)?;
		if matches!(task, DenseTask::JointMulticlassClassification { .. }) { self.materialize(
				"gpu_argmax_accuracy_into",
				&[("predictions", logits), ("targets", validation.targets)],
				&[("accuracy", accuracy)],
				"predictions",
				&parameters, validation_domain, )?; } else {
			let target_vector = self.tensor(DType::I32, shape(&[validation.rows])?)?; self.reduce_sum( validation.targets,
				target_vector, &[1], training_config.reduction_tree_lanes, validation_domain, )?;
			let correct_count = self.tensor(DType::F32, shape(&[1])?)?; self.materialize(
				"gpu_accuracy",
				&[("predictions", logits), ("targets", target_vector)],
				&[("correct_count", correct_count)],
				"predictions",
				&parameters, validation_domain, )?; self.emit_elementwise( vec![correct_count], vec![accuracy],
				divide_constant_program(validation.rows as f32)?, validation_domain, )?; }
		Ok(MulticlassValidationOutputs { logits, metrics: MulticlassMetricOutputs { mean_cross_entropy, accuracy, },
			metric_domain: validation_domain, }) }

	fn compile_regression_validation( &mut self, validation: ValidationValues, training_config: &DenseTrainingConfig,
		block_states: &[DenseBlockState], bounds: &TrainingBounds, ) -> TrainingCompileResult<RegressionValidationOutputs> {
		let validation_domain = IterationDomain::every(bounds.training_iterations);
		let (predictions, output_width) = self.compile_validation_blocks( block_states, validation.features, validation.rows,
			training_config, validation_domain, )?; let matrix_shape = shape(&[validation.rows, output_width])?;
		let scalar_shape = shape(&[1])?; let residuals = self.elementwise_f32( matrix_shape.clone(),
			vec![predictions, validation.targets], subtract_program()?, validation_domain, )?;
		let squared_residuals = self.elementwise_f32( matrix_shape.clone(), vec![residuals], square_program()?,
			validation_domain, )?; let residual_sum_of_squares = self.sum_tensor( squared_residuals, scalar_shape.clone(),
			&[0, 1], training_config.reduction_tree_lanes, validation_domain, )?;

		let target_sum = self.tensor(DType::F32, scalar_shape.clone())?; self.reduce_sum( validation.targets, target_sum,
			&[0, 1], training_config.reduction_tree_lanes, validation_domain, )?;
		let target_mean = self.tensor(DType::F32, scalar_shape.clone())?; self.emit_elementwise( vec![target_sum],
			vec![target_mean], divide_constant_program(validation.rows.checked_mul(output_width).ok_or_else(|| {
				TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
					"R2 target population overflowed u64",
				) })? as f32)?, validation_domain, )?; let centered_targets = self.elementwise_f32( matrix_shape.clone(),
			vec![validation.targets, target_mean], subtract_program()?, validation_domain, )?;
		let squared_centered_targets = self.elementwise_f32( matrix_shape, vec![centered_targets], square_program()?,
			validation_domain, )?; let total_sum_of_squares = self.sum_tensor( squared_centered_targets, scalar_shape.clone(),
			&[0, 1], training_config.reduction_tree_lanes, validation_domain, )?; let r2 = self.elementwise_f32( scalar_shape,
			vec![residual_sum_of_squares, total_sum_of_squares], r2_program()?, validation_domain, )?;
		Ok(RegressionValidationOutputs { predictions, metrics: RegressionMetricOutputs { r2 },
			metric_domain: validation_domain, }) }

	fn matrix_column_vector( &mut self, matrix: ValueId, geometry: [u64; 3], tree_lanes: u32, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let [rows, columns, column] = geometry; if column >= columns {
			return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("matrix column {column} is outside width {columns}"),
			)); }
		let dtype = { let tensor = self.tensor_ref(matrix)?; if tensor.shape.extents() != [rows, columns] {
				return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix, format!(
						"matrix column extraction expected shape [{rows}, {columns}], found {:?}",
						tensor.shape.extents() ), )); }
			tensor.dtype }; let index = self.index_map( shape(&[1])?, IndexMap {
				start: checked_i32(column, "matrix column")?,
				element_step: 0, iteration_step: 0, modulus: None, }, IterationDomain::first(), )?;
		let gathered = self.gather(matrix, index, shape(&[rows, 1])?, 1, domain)?;
		let vector = self.tensor(dtype, shape(&[rows])?)?; self.reduce_sum(gathered, vector, &[1], tree_lanes, domain)?;
		Ok(vector) }

	fn materialize_multi_target_binary_metrics( &mut self, values: [ValueId; 3], geometry: [u64; 2], tree_lanes: u32,
		config: &BinaryValidationConfig, domain: IterationDomain, ) -> TrainingCompileResult<BinaryMetricOutputs> {
		let [probabilities, targets, losses] = values; let [rows, columns] = geometry;
		let target_count = usize::try_from(columns).map_err(|error| { TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("multi-target metric width cannot be represented by usize: {error}"),
			) })?; let mut per_target = Vec::with_capacity(target_count); for column in 0..columns {
			let geometry = [rows, columns, column];
			let probabilities = self.matrix_column_vector(probabilities, geometry, tree_lanes, domain)?;
			let targets = self.matrix_column_vector(targets, geometry, tree_lanes, domain)?;
			let losses = self.matrix_column_vector(losses, geometry, tree_lanes, domain)?;
			per_target.push(self.materialize_binary_metrics( [probabilities, targets, losses], rows, tree_lanes, config, domain,
			)?); }
		let mean = |compiler: &mut Self, values: Vec<ValueId>| compiler.mean_scalar_values(values, domain);
		let mean_bce = mean( self, per_target.iter().map(|metrics| metrics.mean_bce).collect(), )?; let auroc = mean( self,
			per_target.iter().map(|metrics| metrics.auroc).collect(), )?; let auprc = mean( self,
			per_target.iter().map(|metrics| metrics.auprc).collect(), )?; let brier_score = mean( self, per_target .iter()
				.map(|metrics| metrics.brier_score) .collect(), )?; let expected_calibration_error = mean( self, per_target .iter()
				.map(|metrics| metrics.expected_calibration_error) .collect(), )?; let recall_at = config .recall_thresholds()
			.enumerate() .map(|(index, threshold)| -> TrainingCompileResult<_> { Ok(RecallMetricOutput {
					threshold_bits: threshold.to_bits(), value: self.mean_scalar_values( per_target .iter()
							.map(|metrics| metrics.recall_at[index].value) .collect(), domain, )?, }) })
			.collect::<TrainingCompileResult<Vec<_>>>()?; let accuracy = self.tensor(DType::F32, shape(&[1])?)?;
		let parameters = [(
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(tree_lanes)), )] .into_iter() .collect::<PreparedParameters>(); self.materialize(
			"gpu_argmax_accuracy_into",
			&[("predictions", probabilities), ("targets", targets)],
			&[("accuracy", accuracy)],
			"predictions",
			&parameters, domain, )?; Ok(BinaryMetricOutputs { mean_bce, accuracy, auroc, auprc, brier_score,
			expected_calibration_error, recall_at, }) }

	fn mean_scalar_values( &mut self, values: Vec<ValueId>, domain: IterationDomain, ) -> TrainingCompileResult<ValueId> {
		if values.is_empty() { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
				"scalar metric mean requires at least one target value",
			)); }
		let count = values.len(); self.elementwise_f32( shape(&[1])?, values, mean_scalar_values_program(count)?, domain, ) }

	fn materialize_binary_metrics( &mut self, values: [ValueId; 3], examples: u64, tree_lanes: u32,
		config: &BinaryValidationConfig, domain: IterationDomain, ) -> TrainingCompileResult<BinaryMetricOutputs> {
		let [probabilities, targets, losses] = values; let scalar_shape = shape(&[1])?;
		let mean_bce = self.tensor(DType::F32, scalar_shape.clone())?;
		let accuracy = self.tensor(DType::F32, scalar_shape.clone())?;
		let auroc = self.tensor(DType::F32, scalar_shape.clone())?;
		let auprc = self.tensor(DType::F32, scalar_shape.clone())?;
		let brier_score = self.tensor(DType::F32, scalar_shape.clone())?;
		let expected_calibration_error = self.tensor(DType::F32, scalar_shape.clone())?; let recall_at = config
			.recall_thresholds() .map(|threshold| -> TrainingCompileResult<_> { Ok(RecallMetricOutput {
					threshold_bits: threshold.to_bits(), value: self.tensor(DType::F32, scalar_shape.clone())?, }) })
			.collect::<TrainingCompileResult<Vec<_>>>()?;
		let requirements = binary_metric_requirements(examples, recall_at.len(), config.calibration_bins().get())?;
		let first_value = self.next_value; let first_kernel = self.next_kernel; self.next_value = self .next_value
			.checked_add(requirements.intermediate_values) .ok_or_else(identity_exhausted)?; self.next_kernel = self .next_kernel
			.checked_add(requirements.kernels) .ok_or_else(identity_exhausted)?;
		let request = BinaryClassificationMetricRequest { probabilities: self.tensor_ref(probabilities)?.clone(),
			targets: self.tensor_ref(targets)?.clone(), per_element_bce: self.tensor_ref(losses)?.clone(),
			mean_bce: self.tensor_ref(mean_bce)?.clone(), auroc: self.tensor_ref(auroc)?.clone(),
			auprc: self.tensor_ref(auprc)?.clone(), brier_score: self.tensor_ref(brier_score)?.clone(),
			expected_calibration_error: self.tensor_ref(expected_calibration_error)?.clone(), recall_at: recall_at .iter()
				.map(|recall| { Ok(RecallAtOutput::new( recall.threshold(), self.tensor_ref(recall.value)?.clone(), )) })
				.collect::<TrainingCompileResult<Vec<_>>>()?, calibration_bins: config.calibration_bins().get(), tree_lanes,
			identity_namespace: IdentityNamespace::new( ValueId::new(first_value), requirements.intermediate_values,
				KernelTemplateId::new(first_kernel), requirements.kernels, ), workspace_limit: requirements.workspace_bytes, };
		let materialized = materialize_binary_classification_metrics(&request)?;
		self.insert_materialized_graph(&materialized.graph, domain)?; let accuracy_parameters = [(
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(tree_lanes)), )] .into_iter() .collect::<PreparedParameters>(); self.materialize(
			"gpu_accuracy_into",
			&[("predictions", probabilities), ("targets", targets)],
			&[("accuracy", accuracy)],
			"predictions",
			&accuracy_parameters, domain, )?; Ok(BinaryMetricOutputs { mean_bce, accuracy, auroc, auprc, brier_score,
			expected_calibration_error, recall_at, }) }

	fn compile_temperature_scaling( &mut self, logits: ValueId, targets: ValueId, examples: u64, tree_lanes: u32,
		config: TemperatureScalingConfig, bounds: &TrainingBounds, ) -> TrainingCompileResult<TemperatureScalingState> {
		let training_iterations = bounds .training_iterations .finite() .ok_or_else(|| { TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidOptimizer,
					"temperature scaling requires a finite training horizon",
				) })? .get(); let iterations = bounds .iterations .finite()
			.expect("finite training with temperature scaling has finite total iterations")
			.get(); let domain = IterationDomain::new(training_iterations, iterations, 1).ok_or_else(|| {
			TrainingCompileError::new( TrainingCompileErrorKind::InvalidOptimizer,
				"temperature-scaling domain must be nonempty",
			) })?; let scalar_shape = shape(&[1])?; let matrix_shape = shape(&[examples, 1])?; let seed = self.index_map(
			scalar_shape.clone(), IndexMap { start: 0, element_step: 0, iteration_step: 0, modulus: None, },
			IterationDomain::first(), )?; let initial_temperature = self.elementwise_f32( scalar_shape.clone(), vec![seed],
			constant_f32_from_i32_program(1.0)?, IterationDomain::first(), )?; let scaled_logits = self.elementwise_f32(
			matrix_shape.clone(), vec![logits, initial_temperature], divide_program()?, domain, )?;
		let unused_losses = self.tensor(DType::F32, matrix_shape.clone())?;
		let scaled_gradient = self.tensor(DType::F32, matrix_shape.clone())?; self.materialize(
			"gpu_bce_with_logits",
			&[("logits", scaled_logits), ("targets", targets)],
			&[("losses", unused_losses), ("gradients", scaled_gradient)],
			"logits",
			&PreparedParameters::new(), domain, )?; let contributions = self.elementwise_f32( matrix_shape,
			vec![scaled_gradient, logits, initial_temperature], temperature_gradient_program(examples as f32)?, domain, )?;
		let temperature_gradient = self.sum_tensor( contributions, scalar_shape.clone(), &[0, 1], tree_lanes, domain, )?;
		let updated_temperature = self.tensor(DType::F32, scalar_shape)?; let update_kernel = self.emit(
			vec![initial_temperature, temperature_gradient], vec![updated_temperature], PrimitiveKind::Elementwise(Elementwise {
				program: temperature_update_program(config)?, }), vec![ PrimitiveAliasRule { input: 0, output: 0,
					permission: AliasPermission::MustAliasExact, }, PrimitiveAliasRule { input: 1, output: 0,
					permission: AliasPermission::Forbidden, }, ], domain, )?; self.external_outputs.insert(updated_temperature);
		Ok(TemperatureScalingState { initial_temperature, updated_temperature, update_kernel, iterations: config.iterations,
		}) }

	fn backward_blocks( &mut self, blocks: &[Box<dyn CompiledBlock>], mut context: BlockBackwardContext,
	) -> TrainingCompileResult<Vec<ParameterGradient>> { let mut gradients = Vec::with_capacity(blocks.len());
		for (block_index, block) in blocks.iter().enumerate().rev() { let input_gradient = match block_index {
				0 => InputGradientRequirement::Omit, _ => InputGradientRequirement::Required, }; let backward = block.backward(
				self, BlockBackwardContext { gradient: context.gradient, input_gradient, block_index, ..context }, )?;
			match backward.input_gradient { Some(input_gradient) => context.gradient = input_gradient, None => {} }
			gradients.push(backward.parameters); }
		Ok(gradients.into_iter().rev().flatten().collect()) }

	fn gru_matrix_gradient( &mut self, input: ValueId, gradient: ValueId, input_width: u64, output_width: u64,
	) -> TrainingCompileResult<ValueId> { let result = self.tensor(DType::F32, shape(&[input_width, output_width])?)?;
		self.emit( vec![input, gradient], vec![result], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(0, 0)], }), forbidden_aliases(2, 1), self.training_domain, )?; Ok(result) }

	fn gru_bias_gradient( &mut self, gradient: ValueId, width: u64, tree_lanes: u32, ) -> TrainingCompileResult<ValueId> {
		let result = self.sum_tensor( gradient, shape(&[width])?, &[0], tree_lanes, self.training_domain, )?; Ok(result) }

	fn gru_projection_input_gradient( &mut self, gradient: ValueId, weight: ValueId, rows: u64, width: u64,
	) -> TrainingCompileResult<ValueId> { let result = self.tensor(DType::F32, shape(&[rows, width])?)?; self.emit(
			vec![gradient, weight], vec![result], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(1, 1)], }), forbidden_aliases(2, 1), self.training_domain, )?; Ok(result) }

	fn recurrent_gate_backward( &mut self, values: [ValueId; 4], geometry: [u64; 2], tree_lanes: u32,
	) -> TrainingCompileResult<RecurrentGateBackwardValues> {
		let [input, previous_hidden, preactivation_gradient, recurrent_weight] = values; let [rows, width] = geometry;
		Ok(RecurrentGateBackwardValues { input_weight: self.gru_matrix_gradient(input, preactivation_gradient, 1, width)?,
			recurrent_weight: self.gru_matrix_gradient(previous_hidden, preactivation_gradient, width, width)?,
			bias: self.gru_bias_gradient(preactivation_gradient, width, tree_lanes)?,
			previous_hidden: self.gru_projection_input_gradient( preactivation_gradient, recurrent_weight, rows, width, )?, }) }

	fn accumulate_recurrent_gradient( &mut self, accumulated: &mut Option<ValueId>, next: ValueId,
	) -> TrainingCompileResult<()> { *accumulated = Some(match *accumulated {
			Some(previous) => self.exact_add(previous, next, self.training_domain)?, None => next, }); Ok(()) }

	fn attention_head_gradient_to_sequence( &mut self, gradient: ValueId, rows: u64, sequence: u64, heads: u64,
		head_dimension: u64, dimensions: u64, ) -> TrainingCompileResult<ValueId> {
		let sequence_heads = self.head_major_to_sequence( gradient, rows, sequence, heads, head_dimension,
			self.training_domain, )?; self.reinterpret_tensor( sequence_heads, shape(&[rows, sequence, dimensions])?,
			self.training_domain, ) }

	fn attention_weight_gradient( &mut self, input: ValueId, gradient: ValueId, dimensions: u64, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let weight_gradient = self.tensor(DType::F32, shape(&[dimensions, dimensions])?)?; self.emit( vec![input, gradient],
			vec![weight_gradient], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(0, 0), (1, 1)], }), forbidden_aliases(2, 1), domain, )?; Ok(weight_gradient) }

	fn attention_input_gradient( &mut self, values: [ValueId; 2], geometry: [u64; 3], domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let [gradient, weight] = values; let [rows, sequence, dimensions] = geometry;
		let input_gradient = self.tensor(DType::F32, shape(&[rows, sequence, dimensions])?)?; self.emit(
			vec![gradient, weight], vec![input_gradient], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
				contract_axes: vec![(2, 1)], }), forbidden_aliases(2, 1), domain, )?; Ok(input_gradient) }

	fn backward_operation( &mut self, gradient: ValueId, operation: OperationValues, validity: ValueId,
		valid_count: ValueId, epsilon: f32, tree_lanes: u32, ) -> TrainingCompileResult<(ValueId, Option<ValueId>)> {
		let gradient = self.mask_f32_with_zero(gradient, validity, self.training_domain)?;
		let result = match operation.operation {
			DenseOperation::Activation(activation) if activation.learned_parameters() == 1 => {
				let parameter = operation.parameter.expect("compiled PReLU tape");
				let output_shape = self.tensor_ref(gradient)?.shape.clone();
				let input_gradient = self.tensor(DType::F32, output_shape.clone())?;
				let element_gradient = self.tensor(DType::F32, output_shape.clone())?; self.emit_elementwise(
					vec![gradient, operation.input, parameter.value], vec![input_gradient, element_gradient],
					canonical_prelu_backward_program()?, self.training_domain, )?;
				let parameter_gradient = self.tensor(DType::F32, shape(&[1])?)?;
				let axes = (0..output_shape.rank()).collect::<Vec<_>>(); self.reduce_sum( element_gradient, parameter_gradient,
					&axes, tree_lanes, self.training_domain, )?; (input_gradient, Some(parameter_gradient)) }
			DenseOperation::Activation(activation) => (
				self.backward_activation(gradient, operation.input, operation.output, activation)?, None, ),
			DenseOperation::Normalization(_) => ( self.backward_normalization( gradient, operation, validity, valid_count,
					epsilon, tree_lanes, )?, None, ), }; Ok(( self.mask_f32_with_zero(result.0, validity, self.training_domain)?,
			result.1, )) }

	fn update_blocks( &mut self, blocks: &[Box<dyn CompiledBlock>],
		mut updates: ParameterUpdates<'_>,
	) -> TrainingCompileResult<Vec<DenseBlockState>> { let states = blocks .iter()
			.map(|block| block.optimize(self, &mut updates)) .collect::<TrainingCompileResult<Vec<_>>>()?; Ok(states) }

	fn update_operation_parameters( &mut self, operations: &[OperationValues],
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<Vec<ParameterState>> { operations .iter() .filter_map(|operation| operation.parameter)
			.map(|parameter| updates.apply(self, parameter)) .collect() }

	fn global_clip( &mut self, gradients: &[ValueId], maximum_norm: f32, tree_lanes: u32,
	) -> TrainingCompileResult<Vec<ValueId>> { let scalar_shape = shape(&[1])?;
		let mut squared_norms = Vec::with_capacity(gradients.len()); for gradient in gradients.iter().copied() {
			let gradient_shape = self.tensor_ref(gradient)?.shape.clone(); let squares = self.elementwise_f32(
				gradient_shape.clone(), vec![gradient], square_program()?, self.training_domain, )?;
			let norm = self.tensor(DType::F32, scalar_shape.clone())?; let axes = (0..gradient_shape.rank()).collect::<Vec<_>>();
			self.reduce_sum(squares, norm, &axes, tree_lanes, self.training_domain)?; squared_norms.push(norm); }
		let global_squared_norm = self.elementwise_f32( scalar_shape.clone(), squared_norms, sum_program(gradients.len())?,
			self.training_domain, )?; let scale = self.elementwise_f32( scalar_shape, vec![global_squared_norm],
			clip_scale_program(maximum_norm)?, self.training_domain, )?; gradients .iter() .copied() .map(|gradient| {
				let clipped = self.elementwise_f32( self.tensor_ref(gradient)?.shape.clone(), vec![gradient, scale],
					multiply_program()?, self.training_domain, )?; Ok(clipped) }) .collect() }

	fn dynamic_adam_scalars( &mut self, config: &DenseTrainingConfig, bounds: &TrainingBounds,
		accepted: Option<(ValueId, AcceptedUpdatePlan)>,
	) -> TrainingCompileResult<(ValueId, ValueId, ValueId, Option<OptimizerProgressState>)> {
		let scalar_shape = shape(&[1])?; let (step_i32, update_state, schedule_warmup, schedule_updates) =
			if let Some((known_count, plan)) = accepted { let apply_update = self.tensor(DType::I32, scalar_shape.clone())?;
				self.emit_elementwise( vec![known_count], vec![apply_update], optimizer_update_gate_program()?,
					self.training_domain, )?; let initial_accepted_updates = self.index_map( scalar_shape.clone(), IndexMap { start: 0,
						element_step: 0, iteration_step: 0, modulus: None, }, IterationDomain::first(), )?;
				let updated_accepted_updates = self.tensor(DType::I32, scalar_shape.clone())?; let update_kernel = self.emit(
					vec![initial_accepted_updates, apply_update], vec![updated_accepted_updates],
					PrimitiveKind::Elementwise(Elementwise { program: accepted_update_program(plan.counter_limit)?, }),
					exact_single_recurrence_aliases(2), self.training_domain, )?; ( updated_accepted_updates, Some(( apply_update,
						initial_accepted_updates, updated_accepted_updates, update_kernel, plan, )), plan.warmup,
					plan.maximum.map(|maximum| maximum.max(1)), ) } else {
				let step_i32 = if bounds.training_iterations.is_unbounded() { let initial_step = self.index_map(
						scalar_shape.clone(), IndexMap { start: 0, element_step: 0, iteration_step: 0, modulus: None, },
						IterationDomain::first(), )?; let step = self.tensor(DType::I32, scalar_shape.clone())?; self.emit(
						vec![initial_step], vec![step], PrimitiveKind::Elementwise(Elementwise {
							program: saturating_step_program(bounds.warmup_iterations.max(1))?, }), exact_single_recurrence_aliases(1),
						self.training_domain, )?; step } else { let step = self.index_map( scalar_shape.clone(), IndexMap { start: 1,
							element_step: 0, iteration_step: 1, modulus: None, }, self.training_domain, )?; step }; ( step_i32, None,
					bounds.warmup_iterations, bounds.training_iterations .finite() .map(|iterations| iterations.get()), ) };
		let warmup = self.tensor(DType::F32, scalar_shape.clone())?;
		let progress = self.tensor(DType::F32, scalar_shape.clone())?;
		let remaining_fraction = self.tensor(DType::F32, scalar_shape.clone())?;
		let schedule_program = match schedule_updates {
			Some(schedule_updates) => schedule_inputs_program(schedule_warmup, schedule_updates)?,
			None => unbounded_schedule_inputs_program(schedule_warmup)?, }; self.emit_elementwise( vec![step_i32],
			vec![warmup, progress, remaining_fraction], schedule_program, self.training_domain, )?;
		let decay = match config.learning_rate_decay { LearningRateDecay::Constant => { let decay = self.elementwise_f32(
					scalar_shape.clone(), vec![remaining_fraction], constant_one_program()?, self.training_domain, )?; decay }
			LearningRateDecay::Linear => remaining_fraction, LearningRateDecay::Cosine => { let angle = self.elementwise_f32(
					scalar_shape.clone(), vec![remaining_fraction], cosine_angle_program()?, self.training_domain, )?;
				let sine = self.owned_scalar_f32( scalar_shape.clone(),
					"gpu_sin",
					vec![angle], self.training_domain, )?; let decay = self.elementwise_f32( scalar_shape.clone(),
					vec![sine, remaining_fraction], cosine_decay_program()?, self.training_domain, )?; decay }
			LearningRateDecay::Exponential => { let argument = self.elementwise_f32( scalar_shape.clone(),
					vec![remaining_fraction], exponential_decay_argument_program()?, self.training_domain, )?;
				let curve = self.owned_scalar_f32( scalar_shape.clone(),
					"gpu_expm1",
					vec![argument], self.training_domain, )?; let decay = self.elementwise_f32( scalar_shape.clone(),
					vec![curve, remaining_fraction], exponential_decay_program()?, self.training_domain, )?; decay } };
		let learning_rate = self.tensor(DType::F32, scalar_shape.clone())?;
		let mut learning_rate_inputs = vec![decay, warmup]; if let Some((apply_update, ..)) = update_state {
			learning_rate_inputs.push(apply_update); }
		self.emit_elementwise( learning_rate_inputs, vec![learning_rate],
			learning_rate_program(config.adamw.learning_rate, update_state.is_some())?, self.training_domain, )?;
		let beta_seed = self.index_map( scalar_shape.clone(), IndexMap { start: 0, element_step: 0, iteration_step: 0,
				modulus: None, }, IterationDomain::first(), )?;
		let initial_beta_one_power = self.tensor(DType::F32, scalar_shape.clone())?;
		let initial_beta_two_power = self.tensor(DType::F32, scalar_shape.clone())?; self.emit_elementwise( vec![beta_seed],
			vec![initial_beta_one_power, initial_beta_two_power], adam_beta_power_initial_program()?, IterationDomain::first(),
		)?; let beta_one_power = self.tensor(DType::F32, scalar_shape.clone())?;
		let beta_two_power = self.tensor(DType::F32, scalar_shape)?;
		let mut beta_inputs = vec![initial_beta_one_power, initial_beta_two_power];
		if let Some((apply_update, ..)) = update_state { beta_inputs.push(apply_update); }
		let beta_update_kernel = self.emit( beta_inputs, vec![beta_one_power, beta_two_power],
			PrimitiveKind::Elementwise(Elementwise { program: adam_beta_power_update_program( config.adamw.beta_one,
					config.adamw.beta_two, update_state.is_some(), )?, }), if update_state.is_some() {
				exact_gated_pair_recurrence_aliases() } else { exact_pair_recurrence_aliases() }, self.training_domain, )?;
		let optimizer_progress = update_state.map(
			|(apply_update, initial_accepted_updates, updated_accepted_updates, update_kernel, plan)| { OptimizerProgressState {
					apply_update, initial_accepted_updates, updated_accepted_updates, update_kernel, initial_beta_one_power,
					updated_beta_one_power: beta_one_power, initial_beta_two_power, updated_beta_two_power: beta_two_power,
					beta_update_kernel, accepted_updates_per_epoch: plan.per_epoch, maximum_accepted_updates: plan.maximum,
					accepted_update_counter_limit: plan.counter_limit, warmup_accepted_updates: plan.warmup, } }, ); Ok((
			learning_rate, beta_one_power, beta_two_power, optimizer_progress, )) }

	fn adamw_update( &mut self, gradient: ValueId, initial: InitialParameter,
		updates: &ParameterUpdates<'_>,
	) -> TrainingCompileResult<ParameterState> { let learning_rate = updates.learning_rate;
		let beta_one_power = updates.beta_one_power; let beta_two_power = updates.beta_two_power;
		let apply_update = updates.apply_update; let config = updates.config;
		let parameter_shape = self.tensor_ref(initial.value)?.shape.clone();
		let updated_first_moment = self.tensor(DType::F32, parameter_shape.clone())?;
		let updated_second_moment = self.tensor(DType::F32, parameter_shape.clone())?;
		let updated_parameter = self.tensor(DType::F32, parameter_shape)?; let inputs = vec![ gradient, initial.value,
			initial.first_moment, initial.second_moment, learning_rate, beta_one_power, beta_two_power, ];
		let mut inputs = inputs; if let Some(apply_update) = apply_update { inputs.push(apply_update); }
		let outputs = vec![ updated_first_moment, updated_second_moment, updated_parameter, ];
		let aliases = exact_adam_aliases(inputs.len()); let update_kernel = self.emit( inputs, outputs,
			PrimitiveKind::Elementwise(Elementwise { program: adamw_program( config.adamw.beta_one, config.adamw.beta_two,
					config.adamw.epsilon, config.adamw.weight_decay, apply_update.is_some(), )?, }), aliases, self.training_domain, )?;
		self.external_outputs.extend([ updated_parameter, updated_first_moment, updated_second_moment, ]); Ok(ParameterState {
			initial_parameter: initial.value, updated_parameter, initial_first_moment: initial.first_moment,
			updated_first_moment, initial_second_moment: initial.second_moment, updated_second_moment, update_kernel, }) }

	fn reduce_sum( &mut self, input: ValueId, output: ValueId, axes: &[usize], tree_lanes: u32, domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> { self.reduce_value( [input, output], ReduceOperator::Sum, axes, false,
			tree_lanes, domain, ) }

	fn sum_tensor( &mut self, input: ValueId, shape: Shape, axes: &[usize], tree_lanes: u32, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let dtype = self.tensor_ref(input)?.dtype;
		let output = self.tensor(dtype, shape)?; self.reduce_sum(input, output, axes, tree_lanes, domain)?; Ok(output) }

	fn index_map(&mut self, shape: Shape, map: IndexMap, domain: IterationDomain) -> TrainingCompileResult<ValueId> {
		let output = self.tensor(DType::I32, shape)?; self.emit( Vec::new(), vec![output], PrimitiveKind::IndexMap(map),
			Vec::new(), domain, )?; Ok(output) }

	fn gather( &mut self, source: ValueId, indices: ValueId, shape: Shape, axis: usize, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let dtype = self.tensor_ref(source)?.dtype;
		let output = self.tensor(dtype, shape)?; self.emit( vec![source, indices], vec![output],
			PrimitiveKind::Gather(Gather { axis, bounds: IndexBounds::Reject, }), forbidden_aliases(2, 1), domain, )?; Ok(output)
	}

	fn reduce_value( &mut self, values: [ValueId; 2], operator: ReduceOperator, axes: &[usize], keep_dimensions: bool,
		tree_lanes: u32, domain: IterationDomain, ) -> TrainingCompileResult<KernelTemplateId> { let [input, output] = values;
		self.emit( vec![input], vec![output], PrimitiveKind::Reduce(Reduce { operator, axes: AxisSet::new(axes.to_vec())?,
				keep_dimensions, result: ReduceResult::Value, tree_lanes, }), forbidden_aliases(1, 1), domain, ) }

	fn emit_owned_scalar( &mut self, symbol: &str, inputs: Vec<ValueId>, outputs: Vec<ValueId>, domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> { let descriptor = operation_registry().resolve_unique(symbol)?;
		let program = lower_scalar(descriptor)?; self.emit_elementwise(inputs, outputs, program, domain) }

	fn emit_scalar_operation( &mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, operation: ScalarOperation,
		domain: IterationDomain, ) -> TrainingCompileResult<KernelTemplateId> { match operation {
			ScalarOperation::Owned(symbol) => self.emit_owned_scalar(symbol, inputs, outputs, domain),
			ScalarOperation::Program(program) => self.emit_elementwise(inputs, outputs, program, domain), } }

	fn emit_elementwise( &mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, program: recipe_core::ScalarProgram,
		domain: IterationDomain, ) -> TrainingCompileResult<KernelTemplateId> {
		let aliases = forbidden_aliases(inputs.len(), outputs.len()); self.emit( inputs, outputs,
			PrimitiveKind::Elementwise(Elementwise { program }), aliases, domain, ) }

	fn elementwise_f32( &mut self, shape: Shape, inputs: Vec<ValueId>, program: recipe_core::ScalarProgram,
		domain: IterationDomain, ) -> TrainingCompileResult<ValueId> { let output = self.tensor(DType::F32, shape)?;
		self.emit_elementwise(inputs, vec![output], program, domain)?; Ok(output) }

	fn owned_scalar_f32( &mut self, shape: Shape, symbol: &str, inputs: Vec<ValueId>, domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> { let output = self.tensor(DType::F32, shape)?;
		self.emit_owned_scalar(symbol, inputs, vec![output], domain)?; Ok(output) }

	fn emit( &mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind,
		alias_rules: Vec<PrimitiveAliasRule>, domain: IterationDomain, ) -> TrainingCompileResult<KernelTemplateId> {
		let id = self.next_kernel()?; self.nodes.push(CalculationNode { kernel: PrimitiveKernel { id, inputs, outputs,
				alias_rules, kind, }, }); self.domains .push(KernelIterationDomain { kernel: id, domain }); Ok(id) }

	fn materialize( &mut self, symbol: &str,
		inputs: &[(&'static str, ValueId)],
		outputs: &[(&'static str, ValueId)],
		iteration_shape_input: &'static str,
		parameters: &PreparedParameters, domain: IterationDomain, ) -> TrainingCompileResult<()> {
		let mut input_tensors = inputs .iter() .map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<TrainingCompileResult<Vec<_>>>()?; for tensor in &mut input_tensors { tensor.external_input = true;
			tensor.external_output = false; }
		let mut output_tensors = outputs .iter() .map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<TrainingCompileResult<Vec<_>>>()?; for tensor in &mut output_tensors { tensor.external_input = false;
			tensor.external_output = true; }
		let named_inputs = inputs .iter() .zip(&input_tensors) .map(|((name, _), tensor)| NamedTensor::new(name, tensor))
			.collect::<Vec<_>>(); let named_outputs = outputs .iter() .zip(&output_tensors)
			.map(|((name, _), tensor)| NamedTensor::new(name, tensor)) .collect::<Vec<_>>(); let first_value = self.next_value;
		let first_kernel = self.next_kernel; self.next_value = self .next_value .checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?; self.next_kernel = self .next_kernel .checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?; let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?, &named_inputs, &named_outputs, iteration_shape_input, parameters,
			IdentityNamespace::new( ValueId::new(first_value), MATERIALIZATION_RESERVATION, KernelTemplateId::new(first_kernel),
				MATERIALIZATION_RESERVATION, ), WORKSPACE_LIMIT, ))?; self.insert_materialized_graph(materialized.graph(), domain)?;
		Ok(()) }

	fn insert_materialized_graph( &mut self, graph: &CalculationGraph, domain: IterationDomain,
	) -> TrainingCompileResult<()> { for tensor in &graph.tensors { self.insert_tensor_contract(tensor.clone())?; }
		for node in &graph.nodes { self.nodes.push(node.clone()); self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id, domain, }); }
		Ok(()) }

	fn insert_tensor_contract(&mut self, mut tensor: Tensor) -> TrainingCompileResult<()> { tensor.external_input = false;
		tensor.external_output = false; match self.tensors.get(&tensor.id) { Some(existing) if existing.dtype == tensor.dtype
					&& existing.shape == tensor.shape
					&& existing.layout == tensor.layout
					&& existing.storage_bytes == tensor.storage_bytes =>
			{ Ok(()) }
			Some(_) => Err(TrainingCompileError::new( TrainingCompileErrorKind::Language, format!(
					"materialized tensor {} conflicts with an existing contract",
					tensor.id ), )), None => { self.tensors.insert(tensor.id, tensor); Ok(()) } } }

	fn tensor_ref(&self, value: ValueId) -> TrainingCompileResult<&Tensor> { self.tensors.get(&value).ok_or_else(|| {
			TrainingCompileError::new( TrainingCompileErrorKind::Language,
				format!("compiler tensor {value} is absent"),
			) }) }

	fn matrix_width(&self, value: ValueId, _role: &str) -> TrainingCompileResult<u64> {
		let extents = self.tensor_ref(value)?.shape.extents(); Ok(extents[1]) }

	fn next_value(&mut self) -> TrainingCompileResult<ValueId> { let value = self.next_value;
		self.next_value = value.checked_add(1).ok_or_else(identity_exhausted)?; Ok(ValueId::new(value)) }

	fn next_kernel(&mut self) -> TrainingCompileResult<KernelTemplateId> { let kernel = self.next_kernel;
		self.next_kernel = kernel.checked_add(1).ok_or_else(identity_exhausted)?; Ok(KernelTemplateId::new(kernel)) }

}

fn matrix_bytes(matrix: &DenseMatrix) -> (DType, Vec<u8>) { match matrix { DenseMatrix::I32 { values, .. } => (
			DType::I32, values.iter() .flat_map(|value| value.to_le_bytes()) .collect(), ),
		DenseMatrix::F32Bits { values, .. } => ( DType::F32, values.iter() .flat_map(|value| value.to_le_bytes()) .collect(),
		), } }

fn shape(extents: &[u64]) -> TrainingCompileResult<Shape> { Ok(Shape::new(extents.to_vec())?) }

fn checked_product(values: &[u64], name: &str) -> TrainingCompileResult<u64> {
	values.iter().copied().try_fold(1u64, |product, value| { product.checked_mul(value).ok_or_else(|| {
			TrainingCompileError::new( TrainingCompileErrorKind::ArithmeticOverflow,
				format!("{name} overflowed u64"),
			) }) }) }

fn checked_i32(value: u64, name: &str) -> TrainingCompileResult<i32> { i32::try_from(value).map_err(|error| {
		TrainingCompileError::new( TrainingCompileErrorKind::UnsupportedExtent,
			format!("{name} {value} cannot be represented by int32: {error}"),
		) }) }

fn training_metric_bindings( training_loss: ValueId, training_loss_domain: IterationDomain, learning_rate: ValueId,
	validation: Option<&BinaryValidationOutputs>, multiclass_validation: Option<&MulticlassValidationOutputs>,
	regression_validation: Option<&RegressionValidationOutputs>, ) -> TrainingCompileResult<Vec<TrainingMetricBinding>> {
	let mut bindings = Vec::new(); push_metric_binding( &mut bindings, TrainingMetricKind::TrainingLoss, training_loss,
		training_loss_domain, )?; push_metric_binding( &mut bindings, TrainingMetricKind::LearningRate, learning_rate,
		training_loss_domain, )?; if let Some(validation) = validation { for (kind, value) in [ (
				TrainingMetricKind::ValidationMeanBce, validation.metrics.mean_bce, ),
			(TrainingMetricKind::Accuracy, validation.metrics.accuracy), (TrainingMetricKind::AuRoc, validation.metrics.auroc),
			(TrainingMetricKind::AuPrc, validation.metrics.auprc), ( TrainingMetricKind::BrierScore,
				validation.metrics.brier_score, ), ( TrainingMetricKind::ExpectedCalibrationError,
				validation.metrics.expected_calibration_error, ), ] {
			push_metric_binding(&mut bindings, kind, value, validation.metric_domain)?; }
		for recall in &validation.metrics.recall_at { push_metric_binding( &mut bindings, TrainingMetricKind::RecallAt {
					threshold_bits: recall.threshold_bits, }, recall.value, validation.metric_domain, )?; } }
	if let Some(validation) = multiclass_validation { for (kind, value) in [ (
				TrainingMetricKind::ValidationMeanCrossEntropy, validation.metrics.mean_cross_entropy, ),
			(TrainingMetricKind::Accuracy, validation.metrics.accuracy), ] {
			push_metric_binding(&mut bindings, kind, value, validation.metric_domain)?; } }
	if let Some(validation) = regression_validation { push_metric_binding( &mut bindings, TrainingMetricKind::R2,
			validation.metrics.r2, validation.metric_domain, )?; }
	Ok(bindings) }

fn push_metric_binding( bindings: &mut Vec<TrainingMetricBinding>, kind: TrainingMetricKind, value: ValueId,
	domain: IterationDomain, ) -> TrainingCompileResult<()> { let ordinal = u64::try_from(bindings.len()) .ok()
		.and_then(|index| index.checked_add(1)) .ok_or_else(identity_exhausted)?; bindings.push(TrainingMetricBinding { kind,
		metric: MetricId::new(ordinal), value, domain, }); Ok(()) }

fn resume_tensor_program(dtype: DType) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let fresh = builder.input(dtype)?; let resumed = builder.input(dtype)?;
	let enabled = builder.input(DType::I32)?;
	let selected = builder.ternary(ScalarOpcode::Select, enabled, resumed, fresh)?; Ok(builder.finish(&[selected])?) }

fn tree_one_hot_target_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let target = builder.input(DType::I32)?;
	let class = builder.input(DType::I32)?; let supervision = builder.input(DType::F32)?;
	let equal = builder.binary(ScalarOpcode::Equal, target, class)?;
	let indicator = builder.unary(ScalarOpcode::ConvertI32ToF32, equal)?;
	let value = builder.binary(ScalarOpcode::Multiply, indicator, supervision)?; Ok(builder.finish(&[value])?) }

fn tree_relative_node_program(level_base: u64, level_nodes: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let node = builder.input(DType::I32)?;
	let base = builder.i32(checked_i32(level_base, "tree level base")?)?;
	let count = builder.i32(checked_i32(level_nodes, "tree level node count")?)?;
	let relative = builder.binary(ScalarOpcode::Subtract, node, base)?; let zero = builder.i32(0)?;
	let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, relative, zero)?;
	let upper = builder.binary(ScalarOpcode::LessThan, relative, count)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, lower, upper)?; let _ = builder.unary(ScalarOpcode::Require, valid)?;
	Ok(builder.finish(&[relative])?) }

fn tree_lightgbm_active_row_program(internal_nodes: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let node = builder.input(DType::I32)?;
	let supervision = builder.input(DType::F32)?; let zero = builder.i32(0)?;
	let limit = builder.i32(checked_i32(internal_nodes, "LightGBM internal-node count")?)?;
	let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, node, zero)?;
	let upper = builder.binary(ScalarOpcode::LessThan, node, limit)?;
	let active = builder.binary(ScalarOpcode::BitAnd, lower, upper)?;
	let safe_node = builder.ternary(ScalarOpcode::Select, active, node, zero)?;
	let active_weight = builder.unary(ScalarOpcode::ConvertI32ToF32, active)?;
	let active_supervision = builder.binary(ScalarOpcode::Multiply, supervision, active_weight)?;
	Ok(builder.finish(&[safe_node, active_supervision])?) }

fn tree_lightgbm_candidate_choice_program(features: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let candidate = builder.input(DType::I32)?;
	let features = builder.i32(checked_i32(features, "LightGBM feature width")?)?;
	let node = builder.binary(ScalarOpcode::Divide, candidate, features)?;
	let feature = builder.binary(ScalarOpcode::Remainder, candidate, features)?; Ok(builder.finish(&[node, feature])?) }

fn tree_lightgbm_positive_gain_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gain = builder.input(DType::F32)?; let zero = builder.f32(0.0)?;
	let positive = builder.binary(ScalarOpcode::GreaterThan, gain, zero)?; Ok(builder.finish(&[positive])?) }

fn tree_lightgbm_destination_program(tree_offset: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let node = builder.input(DType::I32)?;
	let offset = builder.i32(checked_i32(tree_offset, "LightGBM tree split offset")?)?;
	let destination = builder.binary(ScalarOpcode::Add, offset, node)?; Ok(builder.finish(&[destination])?) }

fn tree_lightgbm_select_i32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let previous = builder.input(DType::I32)?;
	let candidate = builder.input(DType::I32)?; let enabled = builder.input(DType::I32)?;
	let selected = builder.ternary(ScalarOpcode::Select, enabled, candidate, previous)?; Ok(builder.finish(&[selected])?) }

fn tree_lightgbm_select_f32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let previous = builder.input(DType::F32)?;
	let candidate = builder.input(DType::F32)?; let enabled = builder.input(DType::I32)?;
	let selected = builder.ternary(ScalarOpcode::Select, enabled, candidate, previous)?; Ok(builder.finish(&[selected])?) }

fn tree_lightgbm_next_node_program(internal_nodes: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let threshold = builder.input(DType::F32)?; let node = builder.input(DType::I32)?;
	let selected_node = builder.input(DType::I32)?; let enabled = builder.input(DType::I32)?; let zero = builder.i32(0)?;
	let internal_nodes = builder.i32(checked_i32(internal_nodes, "LightGBM internal-node count")?)?;
	let selected_lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, selected_node, zero)?;
	let selected_upper = builder.binary(ScalarOpcode::LessThan, selected_node, internal_nodes)?;
	let selected_valid = builder.binary(ScalarOpcode::BitAnd, selected_lower, selected_upper)?;
	let _ = builder.unary(ScalarOpcode::Require, selected_valid)?;
	let finite_value = builder.unary(ScalarOpcode::IsFinite, value)?;
	let finite_threshold = builder.unary(ScalarOpcode::IsFinite, threshold)?;
	let finite = builder.binary(ScalarOpcode::BitAnd, finite_value, finite_threshold)?;
	let _ = builder.unary(ScalarOpcode::Require, finite)?;
	let matches = builder.binary(ScalarOpcode::Equal, node, selected_node)?;
	let split = builder.binary(ScalarOpcode::BitAnd, enabled, matches)?;
	let right = builder.binary(ScalarOpcode::GreaterThan, value, threshold)?; let two = builder.i32(2)?;
	let one = builder.i32(1)?; let doubled = builder.binary(ScalarOpcode::Multiply, selected_node, two)?;
	let left = builder.binary(ScalarOpcode::Add, doubled, one)?;
	let child = builder.binary(ScalarOpcode::Add, left, right)?;
	let next = builder.ternary(ScalarOpcode::Select, split, child, node)?; Ok(builder.finish(&[next])?) }

fn tree_candidate_index_program(features: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let node = builder.input(DType::I32)?;
	let feature = builder.input(DType::I32)?;
	let features = builder.i32(checked_i32(features, "tree feature width")?)?;
	let base = builder.binary(ScalarOpcode::Multiply, node, features)?;
	let index = builder.binary(ScalarOpcode::Add, base, feature)?; Ok(builder.finish(&[index])?) }

fn repeat_f32_with_i32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let _index = builder.input(DType::I32)?; Ok(builder.finish(&[value])?) }

fn tree_safe_mean_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let sum = builder.input(DType::F32)?;
	let count = builder.input(DType::F32)?; let zero = builder.f32(0.0)?; let one = builder.f32(1.0)?;
	let positive = builder.binary(ScalarOpcode::GreaterThan, count, zero)?;
	let denominator = builder.ternary(ScalarOpcode::Select, positive, count, one)?;
	let mean = builder.binary(ScalarOpcode::Divide, sum, denominator)?; Ok(builder.finish(&[mean])?) }

fn tree_left_weight_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let feature = builder.input(DType::F32)?;
	let threshold = builder.input(DType::F32)?; let supervision = builder.input(DType::F32)?;
	let left = builder.binary(ScalarOpcode::LessThanOrEqual, feature, threshold)?;
	let left = builder.unary(ScalarOpcode::ConvertI32ToF32, left)?;
	let weight = builder.binary(ScalarOpcode::Multiply, left, supervision)?; Ok(builder.finish(&[weight])?) }

fn tree_parent_output_index_program(outputs: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let node = builder.input(DType::I32)?;
	let output = builder.input(DType::I32)?;
	let outputs = builder.i32(checked_i32(outputs, "tree output width")?)?;
	let base = builder.binary(ScalarOpcode::Multiply, node, outputs)?;
	let index = builder.binary(ScalarOpcode::Add, base, output)?; Ok(builder.finish(&[index])?) }

fn tree_variance_gain_program(invalid_gain: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let left_sum = builder.input(DType::F32)?;
	let parent_sum = builder.input(DType::F32)?; let left_count = builder.input(DType::F32)?;
	let parent_count = builder.input(DType::F32)?;
	let right_sum = builder.binary(ScalarOpcode::Subtract, parent_sum, left_sum)?;
	let right_count = builder.binary(ScalarOpcode::Subtract, parent_count, left_count)?; let zero = builder.f32(0.0)?;
	let one = builder.f32(1.0)?; let left_valid = builder.binary(ScalarOpcode::GreaterThan, left_count, zero)?;
	let right_valid = builder.binary(ScalarOpcode::GreaterThan, right_count, zero)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, left_valid, right_valid)?;
	let left_square = builder.binary(ScalarOpcode::Multiply, left_sum, left_sum)?;
	let right_square = builder.binary(ScalarOpcode::Multiply, right_sum, right_sum)?;
	let parent_square = builder.binary(ScalarOpcode::Multiply, parent_sum, parent_sum)?;
	let left_denominator = builder.binary(ScalarOpcode::Add, left_count, one)?;
	let right_denominator = builder.binary(ScalarOpcode::Add, right_count, one)?;
	let parent_denominator = builder.binary(ScalarOpcode::Add, parent_count, one)?;
	let left_score = builder.binary(ScalarOpcode::Divide, left_square, left_denominator)?;
	let right_score = builder.binary(ScalarOpcode::Divide, right_square, right_denominator)?;
	let parent_score = builder.binary(ScalarOpcode::Divide, parent_square, parent_denominator)?;
	let children = builder.binary(ScalarOpcode::Add, left_score, right_score)?;
	let gain = builder.binary(ScalarOpcode::Subtract, children, parent_score)?; let invalid = builder.f32(invalid_gain)?;
	let gain = builder.ternary(ScalarOpcode::Select, valid, gain, invalid)?; Ok(builder.finish(&[gain])?) }

fn tree_row_feature_index_program(features: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let row = builder.input(DType::I32)?;
	let feature = builder.input(DType::I32)?;
	let features = builder.i32(checked_i32(features, "tree feature width")?)?;
	let base = builder.binary(ScalarOpcode::Multiply, row, features)?;
	let index = builder.binary(ScalarOpcode::Add, base, feature)?; Ok(builder.finish(&[index])?) }

fn tree_split_index_program(trees: u64, internal_nodes: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let pair = builder.input(DType::I32)?;
	let node = builder.input(DType::I32)?;
	let trees = builder.i32(checked_i32(trees, "tree count")?)?;
	let internal_nodes = builder.i32(checked_i32(internal_nodes, "tree internal-node count")?)?;
	let zero = builder.i32(0)?; let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, node, zero)?;
	let upper = builder.binary(ScalarOpcode::LessThan, node, internal_nodes)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, lower, upper)?; let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let tree = builder.binary(ScalarOpcode::Remainder, pair, trees)?;
	let base = builder.binary(ScalarOpcode::Multiply, tree, internal_nodes)?;
	let index = builder.binary(ScalarOpcode::Add, base, node)?; Ok(builder.finish(&[index])?) }

fn tree_feature_index_program(trees: u64, feature_width: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let pair = builder.input(DType::I32)?;
	let feature = builder.input(DType::I32)?;
	let trees = builder.i32(checked_i32(trees, "tree count")?)?;
	let feature_width = builder.i32(checked_i32(feature_width, "tree feature width")?)?;
	let zero = builder.i32(0)?; let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, feature, zero)?;
	let upper = builder.binary(ScalarOpcode::LessThan, feature, feature_width)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, lower, upper)?; let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let row = builder.binary(ScalarOpcode::Divide, pair, trees)?;
	let base = builder.binary(ScalarOpcode::Multiply, row, feature_width)?;
	let index = builder.binary(ScalarOpcode::Add, base, feature)?; Ok(builder.finish(&[index])?) }

fn tree_next_node_program(internal_nodes: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let threshold = builder.input(DType::F32)?; let node = builder.input(DType::I32)?;
	let internal_nodes = builder.i32(checked_i32(internal_nodes, "tree internal-node count")?)?;
	let zero = builder.i32(0)?; let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, node, zero)?;
	let upper = builder.binary(ScalarOpcode::LessThan, node, internal_nodes)?;
	let valid_node = builder.binary(ScalarOpcode::BitAnd, lower, upper)?;
	let finite_value = builder.unary(ScalarOpcode::IsFinite, value)?;
	let finite_threshold = builder.unary(ScalarOpcode::IsFinite, threshold)?;
	let finite = builder.binary(ScalarOpcode::BitAnd, finite_value, finite_threshold)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, valid_node, finite)?;
	let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let right = builder.binary(ScalarOpcode::GreaterThan, value, threshold)?; let two = builder.i32(2)?;
	let one = builder.i32(1)?; let doubled = builder.binary(ScalarOpcode::Multiply, node, two)?;
	let left = builder.binary(ScalarOpcode::Add, doubled, one)?;
	let next = builder.binary(ScalarOpcode::Add, left, right)?; Ok(builder.finish(&[next])?) }

fn tree_leaf_base_program( trees: u64, internal_nodes: u64, leaves: u64, outputs: u64,
) -> TrainingCompileResult<recipe_core::ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let pair = builder.input(DType::I32)?; let node = builder.input(DType::I32)?;
	let trees = builder.i32(checked_i32(trees, "tree count")?)?;
	let internal_nodes = builder.i32(checked_i32(internal_nodes, "tree internal-node count")?)?;
	let leaves = builder.i32(checked_i32(leaves, "tree leaf count")?)?;
	let outputs = builder.i32(checked_i32(outputs, "tree output width")?)?;
	let leaf_limit = builder.binary(ScalarOpcode::Add, internal_nodes, leaves)?;
	let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, node, internal_nodes)?;
	let upper = builder.binary(ScalarOpcode::LessThan, node, leaf_limit)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, lower, upper)?; let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let tree = builder.binary(ScalarOpcode::Remainder, pair, trees)?;
	let leaf = builder.binary(ScalarOpcode::Subtract, node, internal_nodes)?;
	let tree_base = builder.binary(ScalarOpcode::Multiply, tree, leaves)?;
	let tree_leaf = builder.binary(ScalarOpcode::Add, tree_base, leaf)?;
	let base = builder.binary(ScalarOpcode::Multiply, tree_leaf, outputs)?; Ok(builder.finish(&[base])?) }

fn tree_leaf_index_program(outputs: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let base = builder.input(DType::I32)?;
	let output = builder.input(DType::I32)?;
	let outputs = builder.i32(checked_i32(outputs, "tree output width")?)?;
	let zero = builder.i32(0)?; let lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, output, zero)?;
	let upper = builder.binary(ScalarOpcode::LessThan, output, outputs)?;
	let valid = builder.binary(ScalarOpcode::BitAnd, lower, upper)?; let _ = builder.unary(ScalarOpcode::Require, valid)?;
	let index = builder.binary(ScalarOpcode::Add, base, output)?; Ok(builder.finish(&[index])?) }

fn identity_exhausted() -> TrainingCompileError { TrainingCompileError::new(
		TrainingCompileErrorKind::IdentityExhausted,
		"deterministic graph identity space exhausted",
	) }

fn exact_adam_aliases(inputs: usize) -> Vec<PrimitiveAliasRule> { (0..inputs) .flat_map(|input| {
			(0..3).map(move |output| { let exact = matches!((input, output), (1, 2) | (2, 0) | (3, 1)); PrimitiveAliasRule {
					input, output, permission: if exact { AliasPermission::MustAliasExact } else { AliasPermission::Forbidden }, } })
		}) .collect() }

fn exact_single_recurrence_aliases(inputs: usize) -> Vec<PrimitiveAliasRule> { (0..inputs)
		.map(|input| PrimitiveAliasRule { input, output: 0, permission: if input == 0 { AliasPermission::MustAliasExact
			} else { AliasPermission::Forbidden }, }) .collect() }

fn exact_pair_recurrence_aliases() -> Vec<PrimitiveAliasRule> { (0..2).flat_map(|input| {
		(0..2).map(move |output| PrimitiveAliasRule { input, output, permission: if input == output {
				AliasPermission::MustAliasExact } else { AliasPermission::Forbidden }, }) }) .collect() }

fn exact_gated_pair_recurrence_aliases() -> Vec<PrimitiveAliasRule> { (0..3).flat_map(|input| {
		(0..2).map(move |output| PrimitiveAliasRule { input, output, permission: if input == output {
				AliasPermission::MustAliasExact } else { AliasPermission::Forbidden }, }) }) .collect() }

fn sequence_major_source_index_program( sequence_length: u64, heads: u64, head_dimension: u64,
) -> TrainingCompileResult<recipe_core::ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let sequence = builder.i32(checked_i32(sequence_length, "attention sequence length")?)?;
	let heads = builder.i32(checked_i32(heads, "attention head count")?)?;
	let dimension = builder.i32(checked_i32(head_dimension, "attention head dimension")?)?;
	let channel = builder.binary(ScalarOpcode::Remainder, position, dimension)?;
	let packed = builder.binary(ScalarOpcode::Divide, position, dimension)?;
	let token = builder.binary(ScalarOpcode::Remainder, packed, sequence)?;
	let head_packed = builder.binary(ScalarOpcode::Divide, packed, sequence)?;
	let head = builder.binary(ScalarOpcode::Remainder, head_packed, heads)?;
	let row = builder.binary(ScalarOpcode::Divide, head_packed, heads)?;
	let source = builder.binary(ScalarOpcode::Multiply, row, sequence)?;
	let source = builder.binary(ScalarOpcode::Add, source, token)?;
	let source = builder.binary(ScalarOpcode::Multiply, source, heads)?;
	let source = builder.binary(ScalarOpcode::Add, source, head)?;
	let source = builder.binary(ScalarOpcode::Multiply, source, dimension)?;
	let source = builder.binary(ScalarOpcode::Add, source, channel)?; Ok(builder.finish(&[source])?) }

fn convert_i32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::I32)?;
	let converted = builder.unary(ScalarOpcode::ConvertI32ToF32, value)?; Ok(builder.finish(&[converted])?) }

fn constant_f32_from_i32_program(value: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let _seed = builder.input(DType::I32)?;
	let value = builder.f32(value)?; Ok(builder.finish(&[value])?) }

fn validity_program(rows: i32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let position = builder.input(DType::I32)?;
	let row_count = builder.i32(rows)?; let valid = builder.binary(ScalarOpcode::LessThan, position, row_count)?;
	let valid = builder.unary(ScalarOpcode::ConvertI32ToF32, valid)?; Ok(builder.finish(&[valid])?) }

fn zero_outputs_program(outputs: usize) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let _seed = builder.input(DType::I32)?; let zeros = (0..outputs)
		.map(|_| builder.f32(0.0)) .collect::<Result<Vec<_>, _>>()?; Ok(builder.finish(&zeros)?) }

fn constant_parameter_program(value: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let _seed = builder.input(DType::I32)?;
	let value = builder.f32(value)?; let first_moment = builder.f32(0.0)?; let second_moment = builder.f32(0.0)?;
	Ok(builder.finish(&[value, first_moment, second_moment])?) }

fn routing_extents(routing: DenseGroupToNeuronRouting) -> (NonZeroU64, NonZeroU64) { match routing {
		DenseGroupToNeuronRouting::Identity { width } => (width, width), DenseGroupToNeuronRouting::Expand {
			groups, neurons, .. }
		| DenseGroupToNeuronRouting::Contract { groups, neurons, .. }
		| DenseGroupToNeuronRouting::FullyConnected { groups, neurons } => (groups, neurons), } }

fn group_routing_mask_program( channels: NonZeroU64, routing: DenseGroupToNeuronRouting,
) -> TrainingCompileResult<recipe_core::ScalarProgram> { let (_, neurons) = routing_extents(routing);
	let group_stride = channels.get().checked_mul(neurons.get()).ok_or_else(|| { TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"group routing row stride overflowed u64",
		) })?;
	let group_stride = checked_i32(group_stride, "group routing row stride")?;
	let neurons_i32 = checked_i32(neurons.get(), "group routing neuron count")?;
	let mut builder = ScalarProgramBuilder::new()?; let position = builder.input(DType::I32)?;
	let group_stride = builder.i32(group_stride)?; let neurons_value = builder.i32(neurons_i32)?;
	let group = builder.binary(ScalarOpcode::Divide, position, group_stride)?;
	let neuron = builder.binary(ScalarOpcode::Remainder, position, neurons_value)?; let allowed = match routing {
		DenseGroupToNeuronRouting::Identity { .. } => builder.binary(ScalarOpcode::Equal, group, neuron)?,
		DenseGroupToNeuronRouting::Expand { neurons_per_group, .. } => { let divisor = builder.i32(checked_i32(
				neurons_per_group.get(),
				"neurons per routed group",
			)?)?; let neuron_group = builder.binary(ScalarOpcode::Divide, neuron, divisor)?;
			builder.binary(ScalarOpcode::Equal, group, neuron_group)? }
		DenseGroupToNeuronRouting::Contract { groups_per_neuron, .. } => { let divisor = builder.i32(checked_i32(
				groups_per_neuron.get(),
				"groups per routed neuron",
			)?)?; let neuron_group = builder.binary(ScalarOpcode::Divide, group, divisor)?;
			builder.binary(ScalarOpcode::Equal, neuron, neuron_group)? }
		DenseGroupToNeuronRouting::FullyConnected { .. } => builder.i32(1)?, };
	let allowed = builder.unary(ScalarOpcode::ConvertI32ToF32, allowed)?; Ok(builder.finish(&[allowed])?) }

fn gru_hidden_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gradient = builder.input(DType::F32)?;
	let candidate = builder.input(DType::F32)?; let update = builder.input(DType::F32)?;
	let previous = builder.input(DType::F32)?; let one = builder.f32(1.0)?;
	let candidate_scale = builder.binary(ScalarOpcode::Subtract, one, update)?;
	let candidate_gradient = builder.binary(ScalarOpcode::Multiply, gradient, candidate_scale)?;
	let difference = builder.binary(ScalarOpcode::Subtract, previous, candidate)?;
	let update_gradient = builder.binary(ScalarOpcode::Multiply, gradient, difference)?;
	let previous_gradient = builder.binary(ScalarOpcode::Multiply, gradient, update)?;
	Ok(builder.finish(&[candidate_gradient, update_gradient, previous_gradient])?) }

fn gru_reset_product_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gradient = builder.input(DType::F32)?;
	let reset = builder.input(DType::F32)?; let previous = builder.input(DType::F32)?;
	let reset_gradient = builder.binary(ScalarOpcode::Multiply, gradient, previous)?;
	let previous_gradient = builder.binary(ScalarOpcode::Multiply, gradient, reset)?;
	Ok(builder.finish(&[reset_gradient, previous_gradient])?) }

fn lstm_hidden_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gradient = builder.input(DType::F32)?;
	let output_gate = builder.input(DType::F32)?; let cell_activation = builder.input(DType::F32)?;
	let output_gate_gradient = builder.binary(ScalarOpcode::Multiply, gradient, cell_activation)?;
	let cell_activation_gradient = builder.binary(ScalarOpcode::Multiply, gradient, output_gate)?;
	Ok(builder.finish(&[output_gate_gradient, cell_activation_gradient])?) }

fn lstm_cell_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gradient = builder.input(DType::F32)?;
	let forget = builder.input(DType::F32)?; let previous_cell = builder.input(DType::F32)?;
	let input = builder.input(DType::F32)?; let candidate = builder.input(DType::F32)?;
	let forget_gradient = builder.binary(ScalarOpcode::Multiply, gradient, previous_cell)?;
	let previous_cell_gradient = builder.binary(ScalarOpcode::Multiply, gradient, forget)?;
	let input_gradient = builder.binary(ScalarOpcode::Multiply, gradient, candidate)?;
	let candidate_gradient = builder.binary(ScalarOpcode::Multiply, gradient, input)?; Ok(builder.finish(&[
		forget_gradient, previous_cell_gradient, input_gradient, candidate_gradient, ])?) }

fn masked_zero_f32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let validity = builder.input(DType::F32)?; let zero = builder.f32(0.0)?;
	let supervised = builder.binary(ScalarOpcode::GreaterThan, validity, zero)?;
	let masked = builder.ternary(ScalarOpcode::Select, supervised, value, zero)?; Ok(builder.finish(&[masked])?) }

fn masked_zero_i32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::I32)?;
	let validity = builder.input(DType::F32)?; let zero_f32 = builder.f32(0.0)?;
	let supervised = builder.binary(ScalarOpcode::GreaterThan, validity, zero_f32)?; let zero_integer = builder.i32(0)?;
	let masked = builder.ternary(ScalarOpcode::Select, supervised, value, zero_integer)?; Ok(builder.finish(&[masked])?) }

fn negative_multiply_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?; let product = builder.binary(ScalarOpcode::Multiply, left, right)?;
	let result = builder.unary(ScalarOpcode::Negate, product)?; Ok(builder.finish(&[result])?) }

fn pointwise_loss_program(loss: DenseLoss) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let prediction = builder.input(DType::F32)?;
	let target = builder.input(DType::F32)?; let prediction_is_finite = builder.unary(ScalarOpcode::IsFinite, prediction)?;
	let target_is_finite = builder.unary(ScalarOpcode::IsFinite, target)?;
	let _ = builder.unary(ScalarOpcode::Require, prediction_is_finite)?;
	let _ = builder.unary(ScalarOpcode::Require, target_is_finite)?;
	let difference = builder.binary(ScalarOpcode::Subtract, prediction, target)?;

	let (loss, gradient) = match loss { DenseLoss::MeanSquaredError => {
			let loss = builder.binary(ScalarOpcode::Multiply, difference, difference)?; let two = builder.f32(2.0)?;
			let gradient = builder.binary(ScalarOpcode::Multiply, two, difference)?; (loss, gradient) }
		DenseLoss::MeanAbsoluteError => { let loss = builder.unary(ScalarOpcode::Absolute, difference)?;
			let zero = builder.f32(0.0)?; let one = builder.f32(1.0)?; let negative_one = builder.f32(-1.0)?;
			let positive = builder.binary(ScalarOpcode::GreaterThan, difference, zero)?;
			let negative = builder.binary(ScalarOpcode::LessThan, difference, zero)?;
			let positive_or_zero = builder.ternary(ScalarOpcode::Select, positive, one, zero)?; let gradient = builder.ternary(
				ScalarOpcode::Select, negative, negative_one, positive_or_zero, )?; (loss, gradient) }
		DenseLoss::CrossEntropy => unreachable!("cross entropy uses its logits program"),
		DenseLoss::Huber => { let absolute = builder.unary(ScalarOpcode::Absolute, difference)?; let one = builder.f32(1.0)?;
			let quadratic_domain = builder.binary(ScalarOpcode::LessThan, absolute, one)?;
			let square = builder.binary(ScalarOpcode::Multiply, difference, difference)?; let half = builder.f32(0.5)?;
			let quadratic = builder.binary(ScalarOpcode::Multiply, half, square)?;
			let linear = builder.binary(ScalarOpcode::Subtract, absolute, half)?;
			let loss = builder.ternary(ScalarOpcode::Select, quadratic_domain, quadratic, linear)?;
			let negative_one = builder.f32(-1.0)?;
			let lower_bounded = builder.binary(ScalarOpcode::Maximum, difference, negative_one)?;
			let gradient = builder.binary(ScalarOpcode::Minimum, lower_bounded, one)?; (loss, gradient) }
		DenseLoss::BinaryCrossEntropy | DenseLoss::Focal => {
			unreachable!("binary objectives use their logits programs")
		} }; Ok(builder.finish(&[loss, gradient])?) }

fn cross_entropy_with_logits_program(classes: i32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if classes <= 0 { return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
			"categorical class count must be positive",
		)); }
	let mut builder = ScalarProgramBuilder::new()?; let logit = builder.input(DType::F32)?;
	let target = builder.input(DType::I32)?; let class_index = builder.input(DType::I32)?;
	let maximum = builder.input(DType::F32)?; let logarithmic_sum = builder.input(DType::F32)?;
	let exponential = builder.input(DType::F32)?; let exponential_sum = builder.input(DType::F32)?; for value in [ logit,
		maximum, logarithmic_sum, exponential, exponential_sum, ] {
		let finite = builder.unary(ScalarOpcode::IsFinite, value)?; let _ = builder.unary(ScalarOpcode::Require, finite)?; }
	let zero = builder.i32(0)?; let class_count = builder.i32(classes)?;
	let target_nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, target, zero)?;
	let target_below_class_count = builder.binary(ScalarOpcode::LessThan, target, class_count)?;
	let target_in_range = builder.binary( ScalarOpcode::BitAnd, target_nonnegative, target_below_class_count, )?;
	let _ = builder.unary(ScalarOpcode::Require, target_in_range)?;
	let target_indicator = builder.binary(ScalarOpcode::Equal, target, class_index)?;
	let target_indicator = builder.unary(ScalarOpcode::ConvertI32ToF32, target_indicator)?;
	let log_partition = builder.binary(ScalarOpcode::Add, maximum, logarithmic_sum)?;
	let negative_log_probability = builder.binary(ScalarOpcode::Subtract, log_partition, logit)?;
	let loss = builder.binary( ScalarOpcode::Multiply, target_indicator, negative_log_probability, )?;
	let probability = builder.binary(ScalarOpcode::Divide, exponential, exponential_sum)?;
	let gradient = builder.binary(ScalarOpcode::Subtract, probability, target_indicator)?;
	Ok(builder.finish(&[loss, gradient])?) }

fn cross_entropy_with_dense_targets_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let logit = builder.input(DType::F32)?;
	let target = builder.input(DType::F32)?; let maximum = builder.input(DType::F32)?;
	let logarithmic_sum = builder.input(DType::F32)?; let exponential = builder.input(DType::F32)?;
	let exponential_sum = builder.input(DType::F32)?; for value in [ logit, target, maximum, logarithmic_sum, exponential,
		exponential_sum, ] { let finite = builder.unary(ScalarOpcode::IsFinite, value)?;
		let _ = builder.unary(ScalarOpcode::Require, finite)?; }
	let zero = builder.f32(0.0)?; let one = builder.f32(1.0)?;
	let target_nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, target, zero)?;
	let target_at_most_one = builder.binary(ScalarOpcode::LessThanOrEqual, target, one)?;
	let target_in_range = builder.binary(ScalarOpcode::BitAnd, target_nonnegative, target_at_most_one)?;
	let _ = builder.unary(ScalarOpcode::Require, target_in_range)?;
	let log_partition = builder.binary(ScalarOpcode::Add, maximum, logarithmic_sum)?;
	let negative_log_probability = builder.binary(ScalarOpcode::Subtract, log_partition, logit)?;
	let loss = builder.binary(ScalarOpcode::Multiply, target, negative_log_probability)?;
	let probability = builder.binary(ScalarOpcode::Divide, exponential, exponential_sum)?;
	let gradient = builder.binary(ScalarOpcode::Subtract, probability, target)?; Ok(builder.finish(&[loss, gradient])?) }

fn signed_logarithm_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let upstream = builder.input(DType::F32)?;
	let value = builder.input(DType::F32)?; let zero = builder.f32(0.0)?;
	let nonzero = builder.binary(ScalarOpcode::NotEqual, value, zero)?;
	let _ = builder.unary(ScalarOpcode::Require, nonzero)?; let absolute = builder.unary(ScalarOpcode::Absolute, value)?;
	let result = builder.binary(ScalarOpcode::Divide, upstream, absolute)?; Ok(builder.finish(&[result])?) }

fn natural_logarithm_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let upstream = builder.input(DType::F32)?;
	let value = builder.input(DType::F32)?; let zero = builder.f32(0.0)?;
	let positive = builder.binary(ScalarOpcode::GreaterThan, value, zero)?;
	let _ = builder.unary(ScalarOpcode::Require, positive)?;
	let result = builder.binary(ScalarOpcode::Divide, upstream, value)?; Ok(builder.finish(&[result])?) }

fn signed_log_one_plus_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let upstream = builder.input(DType::F32)?;
	let value = builder.input(DType::F32)?; let absolute = builder.unary(ScalarOpcode::Absolute, value)?;
	let one = builder.f32(1.0)?; let denominator = builder.binary(ScalarOpcode::Add, one, absolute)?;
	let result = builder.binary(ScalarOpcode::Divide, upstream, denominator)?; Ok(builder.finish(&[result])?) }

fn huber_activation_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let upstream = builder.input(DType::F32)?;
	let value = builder.input(DType::F32)?; let negative_one = builder.f32(-1.0)?; let one = builder.f32(1.0)?;
	let lower_bounded = builder.binary(ScalarOpcode::Maximum, value, negative_one)?;
	let derivative = builder.binary(ScalarOpcode::Minimum, lower_bounded, one)?;
	let result = builder.binary(ScalarOpcode::Multiply, upstream, derivative)?; Ok(builder.finish(&[result])?) }

fn tangent_activation_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let upstream = builder.input(DType::F32)?;
	let tangent = builder.input(DType::F32)?; let square = builder.binary(ScalarOpcode::Multiply, tangent, tangent)?;
	let one = builder.f32(1.0)?; let derivative = builder.binary(ScalarOpcode::Add, one, square)?;
	let result = builder.binary(ScalarOpcode::Multiply, upstream, derivative)?; Ok(builder.finish(&[result])?) }

fn kmeans_distance_gradient_scale_program(epsilon: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gradient = builder.input(DType::F32)?;
	let distance = builder.input(DType::F32)?; let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, distance, epsilon)?;
	let scaled = builder.binary(ScalarOpcode::Divide, gradient, denominator)?; Ok(builder.finish(&[scaled])?) }

fn r2_program() -> TrainingCompileResult<recipe_core::ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let residual_sum_of_squares = builder.input(DType::F32)?; let total_sum_of_squares = builder.input(DType::F32)?;
	let unexplained = builder.binary( ScalarOpcode::Divide, residual_sum_of_squares, total_sum_of_squares, )?;
	let one = builder.f32(1.0)?; let r2 = builder.binary(ScalarOpcode::Subtract, one, unexplained)?;
	Ok(builder.finish(&[r2])?) }

fn safe_count_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let count = builder.input(DType::F32)?; let one = builder.f32(1.0)?;
	let safe = builder.binary(ScalarOpcode::Maximum, count, one)?; Ok(builder.finish(&[safe])?) }

fn mean_scalar_values_program(count: usize) -> TrainingCompileResult<recipe_core::ScalarProgram> { if count == 0 {
		return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidTargetMatrix,
			"scalar mean requires at least one input",
		)); }
	let mut builder = ScalarProgramBuilder::new()?; let mut sum = builder.input(DType::F32)?; for _ in 1..count {
		let value = builder.input(DType::F32)?; sum = builder.binary(ScalarOpcode::Add, sum, value)?; }
	let divisor = builder.f32(count as f32)?; let mean = builder.binary(ScalarOpcode::Divide, sum, divisor)?;
	Ok(builder.finish(&[mean])?) }

fn inverse_standard_deviation_program(epsilon: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let variance = builder.input(DType::F32)?;
	let epsilon = builder.f32(epsilon)?; let variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, variance)?; let one = builder.f32(1.0)?;
	let inverse = builder.binary(ScalarOpcode::Divide, one, standard_deviation)?; Ok(builder.finish(&[inverse])?) }

fn normalization_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let gradient = builder.input(DType::F32)?;
	let gradient_mean = builder.input(DType::F32)?; let normalized = builder.input(DType::F32)?;
	let gradient_normalized_mean = builder.input(DType::F32)?; let inverse_standard_deviation = builder.input(DType::F32)?;
	let centered_gradient = builder.binary(ScalarOpcode::Subtract, gradient, gradient_mean)?;
	let projection = builder.binary(ScalarOpcode::Multiply, normalized, gradient_normalized_mean)?;
	let adjusted = builder.binary(ScalarOpcode::Subtract, centered_gradient, projection)?;
	let result = builder.binary(ScalarOpcode::Multiply, adjusted, inverse_standard_deviation)?;
	Ok(builder.finish(&[result])?) }

fn masked_mean_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let valid = builder.input(DType::F32)?; let valid_count = builder.input(DType::F32)?; let zero = builder.f32(0.0)?;
	let supervised = builder.binary(ScalarOpcode::GreaterThan, valid, zero)?;
	let safe_value = builder.ternary(ScalarOpcode::Select, supervised, value, zero)?;
	let normalized = builder.binary(ScalarOpcode::Divide, safe_value, valid_count)?;
	let masked = builder.ternary(ScalarOpcode::Select, supervised, normalized, zero)?; Ok(builder.finish(&[masked])?) }

fn clip_scale_program(maximum_norm: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let squared_norm = builder.input(DType::F32)?;
	let norm = builder.unary(ScalarOpcode::SquareRoot, squared_norm)?; let epsilon = builder.f32(f32::EPSILON)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, norm, epsilon)?; let maximum = builder.f32(maximum_norm)?;
	let ratio = builder.binary(ScalarOpcode::Divide, maximum, denominator)?; let one = builder.f32(1.0)?;
	let scale = builder.binary(ScalarOpcode::Minimum, one, ratio)?; Ok(builder.finish(&[scale])?) }

fn schedule_inputs_program( warmup_iterations: u64, total_iterations: u64,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if total_iterations == 0 || total_iterations > i32::MAX as u64 || warmup_iterations > total_iterations {
		return Err(TrainingCompileError::new( TrainingCompileErrorKind::InvalidOptimizer,
			"training schedule requires 1..=i32::MAX updates and warmup no longer than training",
		)); }
	let warmup_iterations_i32 = warmup_iterations as i32; let mut builder = ScalarProgramBuilder::new()?;
	let step = builder.input(DType::I32)?; let one = builder.f32(1.0)?; let zero_i32 = builder.i32(0)?;
	let warmup = if warmup_iterations == 0 { one } else { let warmup_end = builder.i32(warmup_iterations_i32)?;
		let warmup_step = builder.binary(ScalarOpcode::Minimum, step, warmup_end)?;
		exact_nonnegative_i32_ratio(&mut builder, warmup_step, warmup_iterations_i32)? };
	let decay_start = warmup_iterations.max(1); let (progress, remaining_fraction) = if total_iterations <= decay_start {
		(builder.f32(0.0)?, one) } else { let decay_start = builder.i32(decay_start as i32)?;
		let elapsed = builder.binary(ScalarOpcode::Subtract, step, decay_start)?;
		let elapsed = builder.binary(ScalarOpcode::Maximum, elapsed, zero_i32)?;
		let decay_iterations = total_iterations.saturating_sub(warmup_iterations.max(1));
		let decay_end = builder.i32(decay_iterations as i32)?;
		let elapsed = builder.binary(ScalarOpcode::Minimum, elapsed, decay_end)?;
		let remaining = builder.binary(ScalarOpcode::Subtract, decay_end, elapsed)?; (
			exact_nonnegative_i32_ratio(&mut builder, elapsed, decay_iterations as i32)?,
			exact_nonnegative_i32_ratio(&mut builder, remaining, decay_iterations as i32)?, ) };
	Ok(builder.finish(&[warmup, progress, remaining_fraction])?) }

fn unbounded_schedule_inputs_program(warmup_iterations: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if warmup_iterations > i32::MAX as u64 { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"unbounded training warmup requires at most i32::MAX updates",
		)); }
	let mut builder = ScalarProgramBuilder::new()?; let step = builder.input(DType::I32)?; let one = builder.f32(1.0)?;
	let warmup = if warmup_iterations == 0 { one } else { let warmup_end = builder.i32(warmup_iterations as i32)?;
		let warmup_step = builder.binary(ScalarOpcode::Minimum, step, warmup_end)?;
		exact_nonnegative_i32_ratio(&mut builder, warmup_step, warmup_iterations as i32)? }; let progress = builder.f32(0.0)?;
	Ok(builder.finish(&[warmup, progress, one])?) }

fn exact_nonnegative_i32_ratio( builder: &mut ScalarProgramBuilder, numerator: ScalarExpression, denominator: i32,
) -> TrainingCompileResult<ScalarExpression> { const LIMB_BITS: i32 = 12; const LIMB_MASK: i32 = (1 << LIMB_BITS) - 1;
	const LIMB_SCALE: f32 = (1 << LIMB_BITS) as f32; if denominator <= 0 { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"schedule ratio denominator must be positive",
		)); }

	if denominator <= LIMB_MASK { let terminal = builder.i32(denominator)?;
		let terminal = builder.binary(ScalarOpcode::Equal, numerator, terminal)?;
		let numerator = builder.unary(ScalarOpcode::ConvertI32ToF32, numerator)?;
		let denominator = builder.f32(denominator as f32)?;
		let ratio = builder.binary(ScalarOpcode::Divide, numerator, denominator)?; let one = builder.f32(1.0)?;
		return Ok(builder.ternary(ScalarOpcode::Select, terminal, one, ratio)?); }

	let terminal = builder.i32(denominator)?; let terminal = builder.binary(ScalarOpcode::Equal, numerator, terminal)?;
	let shift = builder.i32(LIMB_BITS)?; let mask = builder.i32(LIMB_MASK)?;
	let numerator_high = builder.binary(ScalarOpcode::ShiftRightLogical, numerator, shift)?;
	let numerator_low = builder.binary(ScalarOpcode::BitAnd, numerator, mask)?;
	let numerator_high = builder.unary(ScalarOpcode::ConvertI32ToF32, numerator_high)?;
	let numerator_low = builder.unary(ScalarOpcode::ConvertI32ToF32, numerator_low)?;
	let limb_scale = builder.f32(LIMB_SCALE)?;
	let numerator_low = builder.binary(ScalarOpcode::Divide, numerator_low, limb_scale)?;

	let denominator_high = builder.f32((denominator >> LIMB_BITS) as f32)?;
	let denominator_low = builder.f32((denominator & LIMB_MASK) as f32 / LIMB_SCALE)?;
	let quotient = builder.binary(ScalarOpcode::Divide, numerator_high, denominator_high)?;
	let negative_quotient = builder.unary(ScalarOpcode::Negate, quotient)?; let residual = builder.ternary(
		ScalarOpcode::Fma, negative_quotient, denominator_high, numerator_high, )?;
	let residual = builder.binary(ScalarOpcode::Add, residual, numerator_low)?; let residual = builder.ternary(
		ScalarOpcode::Fma, negative_quotient, denominator_low, residual, )?;
	let denominator = builder.binary(ScalarOpcode::Add, denominator_high, denominator_low)?;
	let correction = builder.binary(ScalarOpcode::Divide, residual, denominator)?;
	let ratio = builder.binary(ScalarOpcode::Add, quotient, correction)?; let zero = builder.f32(0.0)?;
	let ratio = builder.binary(ScalarOpcode::Maximum, zero, ratio)?; let one = builder.f32(1.0)?;
	let ratio = builder.binary(ScalarOpcode::Minimum, one, ratio)?;
	Ok(builder.ternary(ScalarOpcode::Select, terminal, one, ratio)?) }

fn cosine_angle_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let remaining_fraction = builder.input(DType::F32)?;
	let half_pi = builder.f32(core::f32::consts::FRAC_PI_2)?;
	let angle = builder.binary(ScalarOpcode::Multiply, remaining_fraction, half_pi)?; Ok(builder.finish(&[angle])?) }

fn cosine_decay_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let sine = builder.input(DType::F32)?;
	let remaining_fraction = builder.input(DType::F32)?; let decay = builder.binary(ScalarOpcode::Multiply, sine, sine)?;
	let zero = builder.f32(0.0)?; let one = builder.f32(1.0)?;
	let at_start = builder.binary(ScalarOpcode::Equal, remaining_fraction, one)?;
	let at_end = builder.binary(ScalarOpcode::Equal, remaining_fraction, zero)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_start, one, decay)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_end, zero, decay)?; Ok(builder.finish(&[decay])?) }

fn exponential_decay_argument_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let remaining_fraction = builder.input(DType::F32)?;
	let rate = builder.f32(5.0)?; let argument = builder.binary(ScalarOpcode::Multiply, rate, remaining_fraction)?;
	Ok(builder.finish(&[argument])?) }

fn exponential_decay_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let curve = builder.input(DType::F32)?;
	let remaining_fraction = builder.input(DType::F32)?; let end = builder.f32((-5.0f32).exp())?;
	let one = builder.f32(1.0)?; let denominator = builder.binary(ScalarOpcode::Subtract, one, end)?;
	let scale = builder.binary(ScalarOpcode::Divide, end, denominator)?;
	let decay = builder.binary(ScalarOpcode::Multiply, scale, curve)?; let zero = builder.f32(0.0)?;
	let at_start = builder.binary(ScalarOpcode::Equal, remaining_fraction, one)?;
	let at_end = builder.binary(ScalarOpcode::Equal, remaining_fraction, zero)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_start, one, decay)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_end, zero, decay)?; Ok(builder.finish(&[decay])?) }

fn learning_rate_program(base_learning_rate: f32, gated: bool) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let decay = builder.input(DType::F32)?;
	let warmup = builder.input(DType::F32)?; let enabled = gated.then(|| builder.input(DType::I32)).transpose()?;
	let factor = builder.binary(ScalarOpcode::Multiply, warmup, decay)?; let base = builder.f32(base_learning_rate)?;
	let candidate = builder.binary(ScalarOpcode::Multiply, base, factor)?;
	let learning_rate = if let Some(enabled) = enabled { let zero = builder.f32(0.0)?;
		builder.ternary(ScalarOpcode::Select, enabled, candidate, zero)? } else { candidate };
	Ok(builder.finish(&[learning_rate])?) }

fn constant_one_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let _coordinate = builder.input(DType::F32)?;
	let one = builder.f32(1.0)?; Ok(builder.finish(&[one])?) }

fn optimizer_update_gate_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let known_count = builder.input(DType::F32)?;
	let zero_f32 = builder.f32(0.0)?; let enabled = builder.binary(ScalarOpcode::GreaterThan, known_count, zero_f32)?;
	Ok(builder.finish(&[enabled])?) }

fn accepted_update_program(counter_limit: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if counter_limit == 0 || counter_limit > i32::MAX as u64 { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"accepted-update counter limit must be in 1..=i32::MAX",
		)); }
	let mut builder = ScalarProgramBuilder::new()?; let accepted = builder.input(DType::I32)?;
	let enabled = builder.input(DType::I32)?; let prior_limit = builder.i32(counter_limit as i32 - 1)?;
	let bounded_prior = builder.binary(ScalarOpcode::Minimum, accepted, prior_limit)?; let one = builder.i32(1)?;
	let candidate = builder.binary(ScalarOpcode::Add, bounded_prior, one)?;
	let updated = builder.ternary(ScalarOpcode::Select, enabled, candidate, accepted)?; Ok(builder.finish(&[updated])?) }

fn saturating_step_program(counter_limit: u64) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if counter_limit == 0 || counter_limit > i32::MAX as u64 { return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"schedule counter limit must be in 1..=i32::MAX",
		)); }
	let mut builder = ScalarProgramBuilder::new()?; let step = builder.input(DType::I32)?;
	let prior_limit = builder.i32(counter_limit as i32 - 1)?;
	let bounded_prior = builder.binary(ScalarOpcode::Minimum, step, prior_limit)?; let one = builder.i32(1)?;
	let updated = builder.binary(ScalarOpcode::Add, bounded_prior, one)?; Ok(builder.finish(&[updated])?) }

fn adam_beta_power_initial_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let _seed = builder.input(DType::I32)?;
	let beta_one_power = builder.f32(1.0)?; let beta_two_power = builder.f32(1.0)?;
	Ok(builder.finish(&[beta_one_power, beta_two_power])?) }

fn adam_beta_power_update_program( beta_one: f32, beta_two: f32, gated: bool,
) -> TrainingCompileResult<recipe_core::ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let beta_one_power = builder.input(DType::F32)?; let beta_two_power = builder.input(DType::F32)?;
	let enabled = gated.then(|| builder.input(DType::I32)).transpose()?; let beta_one = builder.f32(beta_one)?;
	let beta_two = builder.f32(beta_two)?;
	let candidate_one = builder.binary(ScalarOpcode::Multiply, beta_one_power, beta_one)?;
	let candidate_two = builder.binary(ScalarOpcode::Multiply, beta_two_power, beta_two)?;
	let (updated_one, updated_two) = if let Some(enabled) = enabled { (
			builder.ternary(ScalarOpcode::Select, enabled, candidate_one, beta_one_power)?,
			builder.ternary(ScalarOpcode::Select, enabled, candidate_two, beta_two_power)?, ) } else {
		(candidate_one, candidate_two) }; Ok(builder.finish(&[updated_one, updated_two])?) }

fn adamw_program( beta_one: f32, beta_two: f32, epsilon: f32, weight_decay: f32, gated: bool,
) -> TrainingCompileResult<recipe_core::ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let gradient = builder.input(DType::F32)?; let weight = builder.input(DType::F32)?;
	let first_moment = builder.input(DType::F32)?; let second_moment = builder.input(DType::F32)?;
	let learning_rate = builder.input(DType::F32)?; let beta_one_power = builder.input(DType::F32)?;
	let beta_two_power = builder.input(DType::F32)?; let enabled = gated.then(|| builder.input(DType::I32)).transpose()?;

	let one = builder.f32(1.0)?; let beta_one = builder.f32(beta_one)?; let beta_two = builder.f32(beta_two)?;
	let one_minus_beta_one = builder.binary(ScalarOpcode::Subtract, one, beta_one)?;
	let one_minus_beta_two = builder.binary(ScalarOpcode::Subtract, one, beta_two)?;

	let retained_first = builder.binary(ScalarOpcode::Multiply, beta_one, first_moment)?;
	let gradient_first = builder.binary(ScalarOpcode::Multiply, one_minus_beta_one, gradient)?;
	let candidate_first = builder.binary(ScalarOpcode::Add, retained_first, gradient_first)?;

	let retained_second = builder.binary(ScalarOpcode::Multiply, beta_two, second_moment)?;
	let gradient_squared = builder.binary(ScalarOpcode::Multiply, gradient, gradient)?;
	let gradient_second = builder.binary(ScalarOpcode::Multiply, one_minus_beta_two, gradient_squared)?;
	let candidate_second = builder.binary(ScalarOpcode::Add, retained_second, gradient_second)?;

	let (safe_beta_one_power, safe_beta_two_power) = if let Some(enabled) = enabled { (
			builder.ternary(ScalarOpcode::Select, enabled, beta_one_power, beta_one)?,
			builder.ternary(ScalarOpcode::Select, enabled, beta_two_power, beta_two)?, ) } else {
		(beta_one_power, beta_two_power) };
	let first_correction = builder.binary(ScalarOpcode::Subtract, one, safe_beta_one_power)?;
	let second_correction = builder.binary(ScalarOpcode::Subtract, one, safe_beta_two_power)?;
	let corrected_first = builder.binary(ScalarOpcode::Divide, candidate_first, first_correction)?;
	let corrected_second = builder.binary(ScalarOpcode::Divide, candidate_second, second_correction)?;
	let root_second = builder.unary(ScalarOpcode::SquareRoot, corrected_second)?; let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Add, root_second, epsilon)?;
	let normalized = builder.binary(ScalarOpcode::Divide, corrected_first, denominator)?;
	let adaptive_step = builder.binary(ScalarOpcode::Multiply, learning_rate, normalized)?;

	let weight_decay = builder.f32(weight_decay)?;
	let decay = builder.binary(ScalarOpcode::Multiply, learning_rate, weight_decay)?;
	let decay = builder.binary(ScalarOpcode::Multiply, decay, weight)?;
	let decayed_weight = builder.binary(ScalarOpcode::Subtract, weight, decay)?;
	let candidate_weight = builder.binary(ScalarOpcode::Subtract, decayed_weight, adaptive_step)?;
	let (updated_first, updated_second, updated_weight) = if let Some(enabled) = enabled { (
			builder.ternary(ScalarOpcode::Select, enabled, candidate_first, first_moment)?, builder.ternary(
				ScalarOpcode::Select, enabled, candidate_second, second_moment, )?,
			builder.ternary(ScalarOpcode::Select, enabled, candidate_weight, weight)?, ) } else {
		(candidate_first, candidate_second, candidate_weight) };
	Ok(builder.finish(&[updated_first, updated_second, updated_weight])?) }

fn temperature_gradient_program(population: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let scaled_logit_gradient = builder.input(DType::F32)?;
	let logit = builder.input(DType::F32)?; let temperature = builder.input(DType::F32)?;
	let product = builder.binary(ScalarOpcode::Multiply, scaled_logit_gradient, logit)?;
	let numerator = builder.unary(ScalarOpcode::Negate, product)?;
	let temperature_squared = builder.binary(ScalarOpcode::Multiply, temperature, temperature)?;
	let population = builder.f32(population)?;
	let denominator = builder.binary(ScalarOpcode::Multiply, temperature_squared, population)?;
	let contribution = builder.binary(ScalarOpcode::Divide, numerator, denominator)?; Ok(builder.finish(&[contribution])?)
}

fn temperature_update_program(config: TemperatureScalingConfig) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let temperature = builder.input(DType::F32)?;
	let gradient = builder.input(DType::F32)?; let learning_rate = builder.f32(config.learning_rate)?;
	let step = builder.binary(ScalarOpcode::Multiply, learning_rate, gradient)?;
	let candidate = builder.binary(ScalarOpcode::Subtract, temperature, step)?;
	let minimum = builder.f32(config.minimum_temperature)?;
	let bounded = builder.binary(ScalarOpcode::Maximum, candidate, minimum)?;
	let maximum = builder.f32(config.maximum_temperature)?;
	let bounded = builder.binary(ScalarOpcode::Minimum, bounded, maximum)?; Ok(builder.finish(&[bounded])?) }
