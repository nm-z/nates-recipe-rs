pub mod data_prepare;
pub mod native_prepare;
mod source_frontend;
pub mod training;
mod validation_reporting;

pub use data_prepare::{
	DEFAULT_DATA_FIELD_BYTES, DEFAULT_DATA_FIELDS_PER_RECORD, DEFAULT_DATA_RECORDS, DEFAULT_DATA_SOURCE_BYTES,
	DataPreparationError, DataPreparationResult, prepare_data, prepare_data_with_limits,
};
pub use native_prepare::{
	LoadedNativePreparation, NativeDeviceBuildTarget, NativeHostPlan, NativePreparationError,
	NativePreparationResult, NativePreparationScope, NativeTargetPlan, build_native_target_plan,
	load_cached_measured_profile, load_native_preparation, with_current_native_preparation, with_native_preparation,
};
pub use recipe_training::{ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason};
pub use training::{TrainingError, TrainingReport, TrainingResult, compile_training};

include!("facade.rs");
