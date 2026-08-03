pub mod data_prepare; pub mod inference; pub mod native_prepare; mod signal; mod source_frontend; pub mod training;
mod validation_reporting;

pub use data_prepare::{
	DEFAULT_DATA_FIELD_BYTES, DEFAULT_DATA_FIELDS_PER_RECORD, DEFAULT_DATA_RECORDS, DEFAULT_DATA_SOURCE_BYTES,
	DataPreparationError, DataPreparationResult, distill_data, distill_data_with_limits, prepare_data,
	prepare_data_with_limits, select_target_free_data, }; pub use inference::{
	CompiledModelInference, InferenceError, InferenceModelKind, InferenceReport, InferenceResult, compile_inference, };
pub use native_prepare::{
	NativeDeviceBuildTarget, NativeHostPlan, NativePreparationError, NativePreparationResult, NativePreparationScope,
	NativeTargetPlan, with_current_native_preparation, };
pub use recipe_native_executor::{NativeBackendKind, NativeDeviceExecutionEvidence, NativeExecutionEvidence};
pub use recipe_training::{
	BayesModelArtifact, KnnInferencePrediction, KnnInferencePredictionKind, KnnLabelValue, KnnModelArtifact,
	NativeKernelFormat, RealizedNativeKernel, RealizedNativeKernelSet, TrainingExecutionEvidence,
	ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason, }; pub use training::{
	TrainingError, TrainingModelKind, TrainingReport, TrainingResult, compile_bayes_model, compile_knn_model, };

include!("facade.rs");
