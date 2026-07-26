use core::fmt::Debug;
use std::error::Error;
use std::path::Path;

use recipe_core::{FinalizedBundle, RunId};
use recipe_executor::DeviceImage;
use recipe_ingest::{DistilledDataset, RawTable};
use recipe_native_executor::{
	CandidateCrossBackendTransfer, CrossBackendTransfer, LocalPreparedSession, ValidatedCandidateSession,
};
use recipe_prepare::{ArtifactProvider, CandidateRealizer, PreparedNativeSession, Preparer};
use recipe_probe::MeasuredProfile;
use recipe_training::{
	CheckpointArtifact, CheckpointDecodeLimits, CompiledInference, CompletedInferenceExecution,
	InferenceCompileError, InferenceCompileErrorKind, InferenceCompileResult, InferenceExecutionError,
	InferenceExecutionLimits, InferenceExecutionResult, InferenceExternalInput, InferenceInputRole,
	InferenceOutputContract, InferencePrediction, InferencePredictionKind, InferencePreparationError,
	InferencePreparationResult, InferenceRunFailure, PreparedInference, build_inference_device_images,
	compile_prepared_inference, load_and_prepare_checkpoint_inference, load_checkpoint_file,
	prepare_and_execute_local_inference, prepare_checkpoint_inference, prepare_checkpoint_inference_table,
};

type CompileEntry = fn(&PreparedInference) -> InferenceCompileResult<CompiledInference>;
type PrepareDatasetEntry = fn(CheckpointArtifact, &DistilledDataset) -> InferencePreparationResult<PreparedInference>;
type PrepareTableEntry = fn(CheckpointArtifact, &RawTable) -> InferencePreparationResult<PreparedInference>;
type BuildImagesEntry = fn(&CompiledInference, &FinalizedBundle) -> InferenceExecutionResult<Vec<DeviceImage>>;

fn assert_debug<T: Debug>() {}

fn assert_error<T: Error + Send + Sync + 'static>() {}

#[allow(dead_code)]
fn external_native_execution_entry<'cuda, 'hsa, A, R, Bridge>(
	inference: &CompiledInference,
	profile: &MeasuredProfile,
	preparer: &mut Preparer<A, R>,
	run: RunId,
	limits: InferenceExecutionLimits,
) -> InferenceExecutionResult<CompletedInferenceExecution>
where
	A: ArtifactProvider,
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
	R: CandidateRealizer<
			A::Catalog,
			Session = PreparedNativeSession<
				ValidatedCandidateSession<
					LocalPreparedSession<
						'cuda,
						'hsa,
						Bridge,
						<Bridge as CrossBackendTransfer<'cuda, 'hsa>>::Resource,
					>,
				>,
			>,
		>,
{
	prepare_and_execute_local_inference::<A, R, Bridge>(inference, profile, preparer, run, limits)
}

#[test]
fn inference_compiler_and_execution_api_is_reexported_at_the_crate_boundary() {
	let _: CompileEntry = compile_prepared_inference;
	let _: PrepareDatasetEntry = prepare_checkpoint_inference;
	let _: PrepareTableEntry = prepare_checkpoint_inference_table;
	let _load = |path: &Path, limits: CheckpointDecodeLimits| load_checkpoint_file(path, limits);
	let _load_and_prepare = |path: &Path, limits: CheckpointDecodeLimits, data: &DistilledDataset| {
		load_and_prepare_checkpoint_inference(path, limits, data)
	};
	let _: BuildImagesEntry = build_inference_device_images;

	assert_debug::<CompiledInference>();
	assert_debug::<PreparedInference>();
	assert_debug::<InferenceExternalInput>();
	assert_debug::<InferenceOutputContract>();
	assert_debug::<InferencePrediction>();
	assert_debug::<CompletedInferenceExecution>();
	assert_debug::<InferenceRunFailure>();
	assert_error::<InferenceCompileError>();
	assert_error::<InferencePreparationError>();
	assert_error::<InferenceExecutionError>();

	let _: InferenceCompileResult<()> = Ok(());
	let _: InferencePreparationResult<()> = Ok(());
	let _: InferenceExecutionResult<()> = Ok(());
	let _ = InferenceCompileErrorKind::UnsupportedTopology;
	let _ = InferenceInputRole::FeatureNormalizationMask;
	let _ = InferencePredictionKind::Regression;
}
