use recipe_core::{Block, DataNormalization, LearningRateSchedule, Loss};
use recipe_ingest::PreparedDataset;

use crate::{AdamWConfig, BinaryValidationConfig, CompiledTraining, MulticlassValidationConfig, ParameterState, RegressionValidationConfig, TrainingCompileResult, TrainingHorizon};

pub trait RealizedBlock: core::fmt::Debug {
	fn clone_box(&self) -> Box<dyn RealizedBlock>;
	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState));
}

impl Clone for Box<dyn RealizedBlock> {
	fn clone(&self) -> Self { return self.clone_box(); }
}

pub fn compile_training(_dataset: &PreparedDataset, _blocks: &[Block], _loss: Loss, _data_normalization: DataNormalization, _epochs: TrainingHorizon, _warmup_epochs: u64, _learning_rate_schedule: LearningRateSchedule, _gradient_clip_norm: Option<f32>, _normalization_epsilon: f32, _reduction_tree_lanes: u32, _random_seed: u64, _adamw: AdamWConfig, _binary_validation: Option<&BinaryValidationConfig>, _multiclass_validation: Option<&MulticlassValidationConfig>, _regression_validation: Option<&RegressionValidationConfig>) -> TrainingCompileResult<CompiledTraining> {
	todo!("unify: was Dense*, now Block")
}
