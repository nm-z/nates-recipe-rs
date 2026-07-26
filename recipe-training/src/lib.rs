#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Static Recipe calculation-program compilation for dense training.
//!
//! The compiler owns ordered dense activations and normalizations, data
//! normalization, selectable losses, learning-rate schedules, and AdamW
//! semantics. It emits only Recipe primitives, derives iteration-dependent
//! values on the GPU, and represents recurrent parameter storage with exact
//! alias contracts. Epoch-bound validation metrics, a GPU-resident early-stop
//! latch, and bounded post-training temperature scaling use the same static
//! lifecycle without loop-phase host transfers.

mod checkpoint;
mod compile;
mod error;
mod execute;
mod inference;
mod model;
pub mod bayes;

pub use checkpoint::{
	CheckpointArtifact, CheckpointArtifactMetadata, CheckpointArtifactVector, CheckpointDecodeError,
	CheckpointDecodeErrorKind, CheckpointDecodeLimits, CheckpointError, CheckpointImageMetadata,
	CheckpointBlockImage, CheckpointLayerImage, CheckpointManifest, CheckpointParameterImage, CheckpointPath,
	CheckpointPathSegment, CheckpointResidualBranchImage, CheckpointResidualImage, CheckpointResidualSkipImage,
	CheckpointResult, CheckpointTensorImage, CheckpointVectorSchema, CompletedTrainingCheckpoint, decode_checkpoint,
};
pub use compile::{
	compile_dense_training, compile_dense_training_with_binary_validation, compile_dense_training_with_blocks,
	compile_dense_training_with_blocks_and_binary_validation,
	compile_dense_training_with_blocks_and_multiclass_validation, compile_dense_training_with_multiclass_validation,
};
pub use error::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};
pub use execute::{
	CompletedTrainingExecution, FinalTrainingMetric, TrainingExecutionError, TrainingExecutionLimits,
	TrainingExecutionResult, TrainingMetricObserver, TrainingMetricObserverStats, TrainingMetricSample,
	bounded_training_metric_channel, build_training_device_images, prepare_and_execute_local_training,
	prepare_and_execute_local_training_with_observer,
};
pub use inference::{
	InferencePreparationError, InferencePreparationResult, PreparedInference, load_and_prepare_checkpoint_inference,
	load_checkpoint_file, prepare_checkpoint_inference, prepare_checkpoint_inference_table,
};
pub use model::{
	AdamWConfig, BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledDatasetSchema,
	CompiledFeatureSpan, CompiledTraining, DataNormalizationState, DecodedMulticlassClass, DenseActivation, DenseBlock,
	DenseBlockKind, DenseBlockState, DenseDataNormalization, DenseFeatureLowering, DenseLayer, DenseLayerState, DenseLoss,
	DenseNormalization, DenseOperation, DenseOutputAdapter, DenseResidual, DenseResidualOperation, DenseResidualState,
	DenseTask, DenseTrainingConfig, EarlyStoppingState, ExternalInputRole,
	LearningRateDecay, MinMaxState, MulticlassMetricOutputs, MulticlassValidationConfig,
	MulticlassValidationOutputs, OwnedExternalInput, ParameterState, REMAINING_UNSUPPORTED, RecallMetricOutput,
	TemperatureScalingConfig, TemperatureScalingState, TrainingBounds, TrainingMetricBinding, TrainingMetricKind,
	TrainingOutputs, UnsupportedTrainingFeature, ZScoreState,
};
