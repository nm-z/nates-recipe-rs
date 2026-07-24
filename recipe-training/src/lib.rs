#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Static Recipe calculation-program compilation for dense binary training.
//!
//! This first vertical compiler owns dense/SILU/BCE-with-logits/AdamW
//! semantics. It emits only Recipe primitives, derives iteration-dependent
//! values on the GPU, and represents recurrent parameter storage with exact
//! alias contracts. Epoch-bound validation metrics, a GPU-resident early-stop
//! latch, and bounded post-training temperature scaling use the same static
//! lifecycle without loop-phase host transfers.

mod checkpoint;
mod compile;
mod error;
mod execute;
mod model;

pub use checkpoint::{
	CheckpointError, CheckpointManifest, CheckpointResult, CheckpointVectorSchema, CompletedTrainingCheckpoint,
};
pub use compile::{compile_dense_binary_training, compile_dense_binary_training_with_validation};
pub use error::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};
pub use execute::{
	CompletedTrainingExecution, FinalTrainingMetric, TrainingExecutionError, TrainingExecutionLimits,
	TrainingExecutionResult, TrainingMetricObserver, TrainingMetricObserverStats, bounded_training_metric_channel,
	build_training_device_images, prepare_and_execute_local_training,
	prepare_and_execute_local_training_with_observer,
};
pub use model::{
	AdamWConfig, BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledTraining,
	DenseActivation, DenseBinaryDataset, DenseLayer, DenseLayerState, DensePartition, DenseTrainingConfig,
	EarlyStoppingState, ExternalInputRole, OwnedExternalInput, ParameterState, REMAINING_UNSUPPORTED,
	RecallMetricOutput, TemperatureScalingConfig, TemperatureScalingState, TrainingBounds, TrainingMetricBinding,
	TrainingMetricKind, TrainingOutputs, UnsupportedTrainingFeature, ZScoreState,
};
