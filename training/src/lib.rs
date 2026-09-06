#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Static Recipe calculation-program compilation for dense training.
//!
//! The compiler owns ordered dense activations and normalizations, data
//! normalization, selectable losses, learning-rate schedules, and `AdamW`
//! semantics. It emits only Recipe primitives, derives iteration-dependent
//! values on the GPU, and represents recurrent parameter storage with exact
//! alias contracts. Epoch-bound validation metrics and bounded post-training
//! temperature scaling use the same static lifecycle without loop-phase host
//! transfers.

extern crate alloc;

pub use recipe_core::{Activation, Block, DataNormalization, Function, LearningRateSchedule, Loss, Normalization, Optimizer, TreeFamily};

/// Bayesian declaration resolution and observed categorical preparation.
pub mod bayes;
/// Bayesian semantic-model encoding and decoding.
mod bayes_checkpoint;
/// Dense training checkpoint encoding and resume application.
mod checkpoint;
/// Dense training graph compilation.
mod compile;
/// Training compilation error types.
mod error;
/// Prepared training and inference execution.
mod execute;
/// Dense forward-pass lowering.
mod forward;
/// Semantic-model inference compilation and preparation.
mod inference;
/// K-nearest-neighbor reference preparation.
mod knn;
/// K-nearest-neighbor semantic-model encoding and decoding.
mod knn_checkpoint;
/// Dense model declarations and compiled state.
mod model;

pub use bayes::{BayesianCategoricalReferenceSet, BayesianCategoricalSchema, BayesianDependency, BayesianNodeId, BayesianNodeSchema, BayesianNodeSource, BayesianSchemaError, BayesianSchemaErrorKind, BayesianSchemaPathSegment, CATEGORICAL_BAYES_SMOOTHING, ResolvedBayesianDependency, ResolvedBayesianSchema, prepare_categorical_bayesian_reference_sets, resolve_bayesian_schema};
pub use bayes_checkpoint::{BayesModelArtifact, BayesModelDecodeLimits, decode_bayes_model};
pub use checkpoint::{CheckpointArtifact, CheckpointDecodeError, CheckpointDecodeErrorKind, CheckpointDecodeLimits, CheckpointError, CheckpointManifest, CheckpointNativeKernel, CheckpointNativeRealization, CheckpointParameterImage, CheckpointPath, CheckpointPathSegment, CheckpointResult, CheckpointTensorImage, CompletedTrainingCheckpoint, apply_checkpoint_resume, compiled_training_program_digest, decode_checkpoint};
pub use compile::{RealizedBlock, compile_training};
pub use error::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};
pub use execute::{
	CompletedInferenceExecution, CompletedKnnInferenceExecution, CompletedTrainingExecution, FinalTrainingMetric, InferenceExecutionError, InferenceExecutionLimits, InferenceExecutionResult, InferencePrediction, InferenceRunFailure, KnnInferencePrediction, NativeKernelFormat, RealizedNativeKernel, RealizedNativeKernelSet, TrainingExecutionControl, TrainingExecutionError, TrainingExecutionEvidence, TrainingExecutionLimits, TrainingExecutionResult, TrainingMetricObserver, TrainingMetricObserverStats, TrainingMetricSample, bounded_training_metric_channel, build_inference_device_images,
	build_knn_inference_device_images, build_training_device_images, prepare_and_execute_local_inference, prepare_and_execute_local_knn_inference, prepare_and_execute_local_training_controlled,
};
pub use inference::{
	CompiledInference, CompiledKnnInference, GgufLlamaArtifact, GgufLlamaError, GgufLlamaErrorKind, GgufLlamaResult, InferenceCompileError, InferenceCompileErrorKind, InferenceCompileResult, InferenceExternalInput, InferenceInputRole, InferenceOutputContract, InferencePredictionKind, InferencePreparationError, InferencePreparationResult, InferenceTask, KnnInferenceOutputContract, KnnInferencePredictionKind, PreparedBayesInference, PreparedGgufLlamaInference, PreparedInference, PreparedKnnInference, SemanticModelArtifact, compile_prepared_bayes_inference,
	compile_prepared_gguf_llama_inference, compile_prepared_inference, compile_prepared_knn_inference, decode_gguf_llama, load_bayes_model_file, load_checkpoint_file, load_gguf_llama_model_file, load_knn_model_file, load_semantic_model_file, prepare_bayes_inference_table, prepare_checkpoint_inference_table, prepare_gguf_llama_inference_table, prepare_knn_inference_table,
};
pub use knn::{KnnLabelValue, KnnReferenceOutput, KnnReferenceSet, KnnReferenceValues, prepare_knn_reference_set};
pub use knn_checkpoint::{KnnModelArtifact, KnnModelDecodeLimits, decode_knn_model};
pub use model::{
	AdamWConfig, BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledDatasetSchema, CompiledFeatureSpan, CompiledTraining, DataNormalizationState, DecodedMulticlassClass, DenseConvolutionGeometry, DenseFeatureLowering, DenseOutputAdapter, DenseTask, ExternalInputRole, MAXIMUM_REDUCTION_TREE_LANES, MinMaxState, MulticlassMetricOutputs, MulticlassValidationConfig, MulticlassValidationOutputs, OptimizerProgressState, OwnedExternalInput, ParameterState, REMAINING_UNSUPPORTED, RecallMetricOutput, RegressionMetricOutputs, RegressionValidationConfig,
	RegressionValidationOutputs, TemperatureScalingConfig, TemperatureScalingState, TrainingBounds, TrainingHorizon, TrainingMetricBinding, TrainingMetricKind, TrainingOutputs, UnsupportedTrainingFeature, ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason, ZScoreState,
};
