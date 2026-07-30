#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Static Recipe calculation-program compilation for dense training.
//!
//! The compiler owns ordered dense activations and normalizations, data
//! normalization, selectable losses, learning-rate schedules, and AdamW
//! semantics. It emits only Recipe primitives, derives iteration-dependent
//! values on the GPU, and represents recurrent parameter storage with exact
//! alias contracts. Epoch-bound validation metrics and bounded post-training
//! temperature scaling use the same static lifecycle without loop-phase host
//! transfers.

pub mod bayes;
mod bayes_checkpoint;
mod checkpoint;
mod compile;
mod error;
mod execute;
mod inference;
mod knn;
mod knn_checkpoint;
mod model;

pub use bayes::{
	BayesianCategoricalReferenceSet, BayesianCategoricalSchema, BayesianDependency, BayesianNodeId,
	BayesianNodeSchema, BayesianNodeSource, BayesianSchemaError, BayesianSchemaErrorKind, BayesianSchemaPathSegment,
	CATEGORICAL_BAYES_SMOOTHING, ResolvedBayesianDependency, ResolvedBayesianSchema,
	prepare_categorical_bayesian_reference_set, prepare_categorical_bayesian_reference_sets, resolve_bayesian_schema,
};
pub use bayes_checkpoint::{BayesModelArtifact, BayesModelDecodeLimits, decode_bayes_model};
pub use checkpoint::{
	CheckpointArtifact, CheckpointArtifactMetadata, CheckpointArtifactVector, CheckpointAttentionImage,
	CheckpointBlockImage, CheckpointConvolutionImage, CheckpointDecodeError, CheckpointDecodeErrorKind,
	CheckpointDecodeLimits, CheckpointEmbeddingImage, CheckpointError, CheckpointGruImage, CheckpointImageMetadata,
	CheckpointKMeansImage, CheckpointLayerImage, CheckpointLstmImage, CheckpointManifest, CheckpointNativeKernel,
	CheckpointNativeRealization, CheckpointParameterImage, CheckpointPath, CheckpointPathSegment,
	CheckpointPoolImage, CheckpointResidualBranchImage, CheckpointResidualImage, CheckpointResidualSkipImage,
	CheckpointResult, CheckpointRnnImage, CheckpointTensorImage, CheckpointTreeImage, CheckpointVectorSchema,
	CompletedTrainingCheckpoint, apply_checkpoint_resume, compiled_training_program_digest, decode_checkpoint,
};
pub use compile::{
	compile_dense_training, compile_dense_training_with_binary_validation, compile_dense_training_with_blocks,
	compile_dense_training_with_blocks_and_binary_validation,
	compile_dense_training_with_blocks_and_multiclass_validation,
	compile_dense_training_with_blocks_and_regression_validation, compile_dense_training_with_multiclass_validation,
	compile_dense_training_with_regression_validation,
};
pub use error::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};
pub use execute::{
	CompletedInferenceExecution, CompletedKnnInferenceExecution, CompletedTrainingExecution, FinalTrainingMetric,
	InferenceExecutionError, InferenceExecutionLimits, InferenceExecutionResult, InferencePrediction,
	InferenceRunFailure, KnnInferencePrediction, NativeKernelFormat, RealizedNativeKernel, RealizedNativeKernelSet,
	TrainingExecutionControl, TrainingExecutionError, TrainingExecutionLimits, TrainingExecutionResult,
	TrainingMetricObserver, TrainingMetricObserverStats, TrainingMetricSample, bounded_training_metric_channel,
	build_inference_device_images, build_knn_inference_device_images, build_training_device_images,
	prepare_and_execute_local_inference, prepare_and_execute_local_knn_inference, prepare_and_execute_local_training,
	prepare_and_execute_local_training_controlled, prepare_and_execute_local_training_with_observer,
};
pub use inference::{
	CompiledInference, CompiledKnnInference, GgufLlamaArtifact, GgufLlamaError, GgufLlamaErrorKind, GgufLlamaResult,
	InferenceCompileError, InferenceCompileErrorKind, InferenceCompileResult, InferenceExternalInput,
	InferenceInputRole, InferenceOutputContract, InferencePredictionKind, InferencePreparationError,
	InferencePreparationResult, InferenceTask, KnnInferenceOutputContract, KnnInferencePredictionKind,
	PreparedBayesInference, PreparedGgufLlamaInference, PreparedInference, PreparedKnnInference,
	SemanticModelArtifact, compile_prepared_bayes_inference, compile_prepared_gguf_llama_inference,
	compile_prepared_inference, compile_prepared_knn_inference, decode_gguf_llama, load_and_prepare_bayes_inference,
	load_and_prepare_checkpoint_inference, load_and_prepare_knn_inference, load_bayes_model_file,
	load_checkpoint_file, load_gguf_llama_model_file, load_knn_model_file, load_semantic_model_file,
	prepare_bayes_inference_table, prepare_checkpoint_inference, prepare_checkpoint_inference_table,
	prepare_gguf_llama_inference_table, prepare_knn_inference_table,
};
pub use knn::{KnnLabelValue, KnnReferenceOutput, KnnReferenceSet, KnnReferenceValues, prepare_knn_reference_set};
pub use knn_checkpoint::{KnnModelArtifact, KnnModelDecodeLimits, decode_knn_model};
pub use model::{
	AdamWConfig, BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledDatasetSchema,
	CompiledFeatureSpan, CompiledTraining, DataNormalizationState, DecodedMulticlassClass, DenseActivation,
	DenseAttention, DenseAttentionState, DenseBlock, DenseBlockKind, DenseBlockState, DenseConvolution,
	DenseConvolutionGeometry, DenseConvolutionState, DenseDataNormalization, DenseEmbedding, DenseEmbeddingState,
	DenseFeatureLowering, DenseGroupToNeuronRouting, DenseGru, DenseGruState, DenseKMeans, DenseKMeansState,
	DenseLayer, DenseLayerState, DenseLoss, DenseLstm, DenseLstmState, DenseNormalization, DenseOperation,
	DenseOutputAdapter, DensePool, DensePoolGroupOrder, DensePoolState, DensePoolWinnerContract, DenseResidual,
	DenseResidualOperation, DenseResidualState, DenseRnn, DenseRnnState, DenseTask, DenseTrainingConfig, DenseTree,
	DenseTreeFamily, DenseTreeState, ExternalInputRole, LearningRateDecay, MAXIMUM_REDUCTION_TREE_LANES, MinMaxState,
	MulticlassMetricOutputs, MulticlassValidationConfig, MulticlassValidationOutputs, OptimizerProgressState,
	OwnedExternalInput, ParameterState, REMAINING_UNSUPPORTED, RecallMetricOutput, RegressionMetricOutputs,
	RegressionValidationConfig, RegressionValidationOutputs, TemperatureScalingConfig, TemperatureScalingState,
	TrainingBounds, TrainingHorizon, TrainingMetricBinding, TrainingMetricKind, TrainingOutputs,
	UnsupportedTrainingFeature, ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason,
	ZScoreState,
};
