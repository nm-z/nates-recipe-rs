use core::{
	fmt,
	num::{NonZeroU32, NonZeroU64, NonZeroUsize},
};
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	path::Path,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
		mpsc::Receiver,
	},
	thread::JoinHandle,
	time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use recipe_core::{BundleIdentity, ByteCount, DeviceKind, DiscoveryIdentity, FlopCount, MachineId, MetricId, RunId, TargetIdentity, ToolchainIdentity, TopologyIdentity, calculation_time_ceil, transfer_time_ceil};
use recipe_executor::{ExitImage, LogicalEvent, MetricValue, RunJournal, Watchdog};
use recipe_ingest::{SourceError, SourceLimit, read_source_snapshot};
use recipe_language::CalculationGraph;
use recipe_native_executor::{LocalCandidateFactory, StagedCrossBackend};
use recipe_prepare::{NativeArtifactCatalog, NativeArtifactProvider, NativeCandidateRealizer, NativeExecutorDriver, Preparer};
use recipe_probe::MeasuredProfile;
use recipe_training::{
	AdamWConfig, BayesModelArtifact, BayesModelDecodeLimits, BayesianDependency, BinaryValidationConfig, CheckpointArtifact, CheckpointDecodeLimits, CheckpointError, CheckpointManifest, CompiledTraining, CompletedTrainingCheckpoint, CompletedTrainingExecution, FinalTrainingMetric, InferencePreparationError, KnnModelArtifact, KnnModelDecodeLimits, MAXIMUM_REDUCTION_TREE_LANES, MulticlassValidationConfig, NativeKernelFormat, RealizedNativeKernelSet, RegressionValidationConfig, TrainingBounds, TrainingCompileError, TrainingExecutionControl, TrainingExecutionEvidence, TrainingExecutionLimits,
	TrainingHorizon, TrainingMetricKind, TrainingMetricObserver, TrainingMetricSample, ValidationMetricStatus, apply_checkpoint_resume, bounded_training_metric_channel, compile_training, compiled_training_program_digest, load_bayes_model_file, load_checkpoint_file, load_knn_model_file, prepare_and_execute_local_training_controlled, prepare_categorical_bayesian_reference_sets, prepare_knn_reference_set,
};
use write::{always, line};

use crate::{
	api::{Block, Data, DataNormalization, DeclarationError, LearningRateSchedule, Loss, Metric, Model, Objective, Optimizer, Train},
	data_prepare::{DataPreparationError, prepare_data},
	native_prepare::{NativePreparationError, with_current_native_preparation},
};

const LIVE_METRIC_CHANNEL_CAPACITY: NonZeroUsize = NonZeroUsize::MIN.saturating_add(255);
const LIVE_METRIC_PALETTE: [(u8, u8, u8); 12] = [
	(242, 40, 60),
	(39, 125, 255),
	(0, 174, 107),
	(255, 194, 0),
	(215, 46, 130),
	(135, 90, 251),
	(255, 122, 0),
	(91, 192, 235),
	(157, 121, 188),
	(46, 83, 57),
	(3, 252, 186),
	(194, 1, 20),
];

static NEXT_RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// The public training boundary failed before or during execution.
#[non_exhaustive]
pub enum TrainingError {
	Declaration(DeclarationError),
	Data(DataPreparationError),
	Compile(TrainingCompileError),
	Checkpoint(CheckpointError),
	Resume(InferencePreparationError),
	NativeKernelSource(SourceError),
	Native(NativePreparationError),
	Unsupported { detail: String },
	Runtime { stage: &'static str, detail: String },
}

impl TrainingError {
	pub(crate) fn unsupported(detail: impl Into<String>) -> Self {
		return Self::Unsupported {
			detail: detail.into(),
		};
	}

	pub(crate) fn runtime(stage: &'static str, detail: impl Into<String>) -> Self {
		return Self::Runtime {
			stage,
			detail: detail.into(),
		};
	}
}

impl fmt::Display for TrainingError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Declaration(error) => return write!(f, "invalid training declaration: {error}"),
			Self::Data(error) => return write!(f, "prepare training data: {error}"),
			Self::Compile(error) => return write!(f, "compile training graph: {error}"),
			Self::Checkpoint(error) => return write!(f, "save semantic model or native kernel: {error}"),
			Self::Resume(error) => return write!(f, "load training resume model: {error}"),
			Self::NativeKernelSource(error) => return write!(f, "load training resume kernel: {error}"),
			Self::Native(error) => return write!(f, "prepare current native system: {error}"),
			Self::Unsupported { detail } => {
				return write!(f, "unsupported training declaration: {detail}");
			}
			Self::Runtime { stage, detail } => return write!(f, "{stage}: {detail}"),
		}
	}
}

impl fmt::Debug for TrainingError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { return fmt::Display::fmt(self, f); }
}

impl std::error::Error for TrainingError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Declaration(error) => return Some(error),
			Self::Data(error) => return Some(error),
			Self::Compile(error) => return Some(error),
			Self::Checkpoint(error) => return Some(error),
			Self::Resume(error) => return Some(error),
			Self::NativeKernelSource(error) => return Some(error),
			Self::Native(error) => return Some(error),
			Self::Unsupported { .. } | Self::Runtime { .. } => return None,
		}
	}
}

impl From<DeclarationError> for TrainingError {
	fn from(error: DeclarationError) -> Self { return Self::Declaration(error); }
}

impl From<DataPreparationError> for TrainingError {
	fn from(error: DataPreparationError) -> Self { return Self::Data(error); }
}

impl From<TrainingCompileError> for TrainingError {
	fn from(error: TrainingCompileError) -> Self { return Self::Compile(error); }
}

impl From<CheckpointError> for TrainingError {
	fn from(error: CheckpointError) -> Self { return Self::Checkpoint(error); }
}

impl From<InferencePreparationError> for TrainingError {
	fn from(error: InferencePreparationError) -> Self { return Self::Resume(error); }
}

impl From<NativePreparationError> for TrainingError {
	fn from(error: NativePreparationError) -> Self { return Self::Native(error); }
}

pub type TrainingResult<T> = Result<T, TrainingError>;

/// Kind of completed public training/preparation declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingModelKind {
	Dense,
	Knn,
	Bayes,
}

#[derive(Clone)]
enum TrainingReportPayload {
	Dense(CompletedTrainingCheckpoint),
	Knn(KnnModelArtifact),
	Bayes(BayesModelArtifact),
}

/// Completed model preparation. Dense reports are constructed only after
/// native teardown; KNN reports contain the immutable reference model and do
/// not pretend that an optimizer loop or native training run occurred.
#[derive(Clone)]
pub struct TrainingReport {
	payload: TrainingReportPayload,
	validation_status: ValidationMetricStatus,
	gracefully_stopped: bool,
}

impl fmt::Debug for TrainingReport {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return f
			.debug_struct("TrainingReport")
			.field("kind", &self.kind())
			.field("run", &self.run())
			.field("bundle", &self.bundle())
			.field("external_output_count", &self.external_outputs().len())
			.field("metric_count", &self.metrics().len())
			.field("validation_status", &self.validation_status)
			.field("gracefully_stopped", &self.gracefully_stopped)
			.finish_non_exhaustive();
	}
}

impl TrainingReport {
	fn dense(execution: CompletedTrainingExecution, manifest: CheckpointManifest, validation_status: ValidationMetricStatus) -> TrainingResult<Self> {
		let gracefully_stopped = execution
			.journal()
			.logical_events()
			.iter()
			.any(|event| matches!(event, LogicalEvent::LoopStopAccepted { .. }));
		return Ok(Self {
			payload: TrainingReportPayload::Dense(CompletedTrainingCheckpoint::new(execution, manifest)?),
			validation_status,
			gracefully_stopped,
		});
	}

	const fn knn(model: KnnModelArtifact) -> Self {
		return Self {
			payload: TrainingReportPayload::Knn(model),
			validation_status: ValidationMetricStatus::NotRequested,
			gracefully_stopped: false,
		};
	}

	const fn bayes(model: BayesModelArtifact) -> Self {
		return Self {
			payload: TrainingReportPayload::Bayes(model),
			validation_status: ValidationMetricStatus::NotRequested,
			gracefully_stopped: false,
		};
	}

	#[must_use]
	pub const fn kind(&self) -> TrainingModelKind {
		match self.payload {
			TrainingReportPayload::Dense(_) => return TrainingModelKind::Dense,
			TrainingReportPayload::Knn(_) => return TrainingModelKind::Knn,
			TrainingReportPayload::Bayes(_) => return TrainingModelKind::Bayes,
		}
	}

	/// Native dense run identity. KNN preparation has no native training run.
	#[must_use]
	pub const fn run(&self) -> Option<RunId> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return Some(checkpoint.run()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return None,
		}
	}

	/// Native dense bundle identity. KNN preparation has no execution bundle.
	#[must_use]
	pub const fn bundle(&self) -> Option<BundleIdentity> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return Some(checkpoint.bundle()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return None,
		}
	}

	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return checkpoint.external_outputs(),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return &[],
		}
	}

	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return checkpoint.metrics(),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return &[],
		}
	}

	/// Completed native lifecycle journal for dense training. Reference-only
	/// KNN and Bayesian preparation have no optimizer execution to report.
	#[must_use]
	pub const fn journal(&self) -> Option<&RunJournal> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return Some(checkpoint.journal()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return None,
		}
	}

	/// Exact bundled native images built, loaded, warmed, and torn down by a
	/// dense training run.
	#[must_use]
	pub const fn native_kernels(&self) -> Option<&RealizedNativeKernelSet> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return Some(checkpoint.native_kernels()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return None,
		}
	}

	/// Driver-derived native image, entry, queue, completion, allocation, and
	/// teardown evidence for dense training.
	#[must_use]
	pub const fn native_evidence(&self) -> Option<&recipe_native_executor::NativeExecutionEvidence> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return Some(checkpoint.native_evidence()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return None,
		}
	}

	/// Full-partition and actual executor submission evidence for dense training.
	#[must_use]
	pub const fn training_evidence(&self) -> Option<&TrainingExecutionEvidence> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => return Some(checkpoint.training_evidence()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => return None,
		}
	}

	/// Whether requested validation reporting had known target rows.
	#[must_use]
	pub const fn validation_status(&self) -> ValidationMetricStatus { return self.validation_status; }

	/// Whether a host stop request was accepted after a complete epoch.
	#[must_use]
	pub const fn gracefully_stopped(&self) -> bool { return self.gracefully_stopped; }

	fn save_model(&self, path: impl AsRef<Path>) -> TrainingResult<()> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => {
				return checkpoint.save(path).map_err(TrainingError::from);
			}
			TrainingReportPayload::Knn(model) => return model.save(path).map_err(TrainingError::from),
			TrainingReportPayload::Bayes(model) => return model.save(path).map_err(TrainingError::from),
		}
	}

	fn save_native_kernel(&self, path: impl AsRef<Path>, format: NativeKernelFormat) -> TrainingResult<()> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => {
				return checkpoint
					.save_native_kernel(path, format)
					.map_err(TrainingError::from);
			}
			TrainingReportPayload::Knn(_) => {
				return Err(TrainingError::unsupported(
					"KNN reference preparation has no native training kernel artifact",
				));
			}
			TrainingReportPayload::Bayes(_) => {
				return Err(TrainingError::unsupported(
					"categorical Bayesian observation preparation has no native training kernel artifact",
				));
			}
		}
	}
}

#[derive(Debug)]
struct CompiledTrainingPackage {
	training: CompiledTraining,
	checkpoint: CheckpointManifest,
	resume_native: Option<ResumeNativeBundle>,
}

#[derive(Clone, Debug)]
struct ResumeNativeBundle {
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
	target: TargetIdentity,
	toolchain: ToolchainIdentity,
	bytes: Arc<[u8]>,
}

/// Prepare one loss-independent, all-declared-output KNN semantic model.
///
/// KNN has no optimizer update or learned dense objective. The training
/// partition becomes the immutable reference set; prediction calculation is
/// compiled when target-free query rows are supplied.
///
/// # Errors
///
/// Returns the first invalid declaration, data preparation, resume compatibility, or KNN artifact construction error.
pub fn compile_knn_model(policy: &Train, data: &Data, model: &Model) -> TrainingResult<KnnModelArtifact> {
	policy.validate()?;
	data.validate()?;
	model.validate()?;
	let [
		Block::Knn {
			neighbors,
			functions,
		},
	] = model.layers()
	else {
		return Err(TrainingError::unsupported(
			"KNN preparation requires exactly one `.knn(neighbors)` model block",
		));
	};
	if !model.bayes_dependencies().is_empty() || model.weights_source().is_some() {
		return Err(TrainingError::unsupported(
			"KNN preparation cannot combine Bayesian dependencies or a loaded model declaration",
		));
	}
	if model.objective().is_some() || model.gradient_clip_value().is_some() {
		return Err(TrainingError::unsupported(
			"KNN feature reduction has no loss objective or gradient policy",
		));
	}
	if policy.optimizer_spec().is_some() || policy.learning_rate().is_some() || policy.learning_rate_schedule().is_some() || policy.warmup_epoch_bound().is_some() || policy.epoch_bound().is_some() {
		return Err(TrainingError::unsupported(
			"KNN reference preparation has no optimizer, learning rate, warmup, or epoch loop",
		));
	}
	if !policy.log_items().is_empty() || !policy.plot_items().is_empty() {
		return Err(TrainingError::unsupported(
			"KNN reference preparation has no iterative training metrics",
		));
	}
	if policy.resume_kernel_source().is_some() || policy.save_kernel_destination().is_some() {
		return Err(TrainingError::unsupported(
			"KNN reference preparation does not realize a native training kernel artifact",
		));
	}
	let resume = if let Some(source) = policy.resume_source() {
		let path = Path::new(source);
		let exists = path
			.try_exists()
			.map_err(|error| return TrainingError::runtime("inspect KNN resume model", error.to_string()))?;
		if exists {
			Some(load_knn_model_file(path, KnnModelDecodeLimits::default())?)
		} else {
			None
		}
	} else {
		None
	};
	let neighbors = u64::try_from(*neighbors).map_err(|error| {
		return TrainingError::unsupported(format!("KNN neighbor count does not fit u64: {error}"));
	})?;
	let neighbors = NonZeroU64::new(neighbors).ok_or_else(|| return TrainingError::unsupported("KNN neighbor count must be nonzero"))?;
	let prepared = prepare_data(data)?;
	let references = prepare_knn_reference_set(&prepared, neighbors)?;
	let normalization = data.normalization().map(|normalization| {
		match normalization {
			DataNormalization::Identity => return DataNormalization::Identity,
			DataNormalization::ZScore => return DataNormalization::ZScore,
			DataNormalization::MinMax => return DataNormalization::MinMax,
			DataNormalization::L2Norm => return DataNormalization::L2Norm,
		}
	});
	let current = KnnModelArtifact::new(references, normalization, functions.clone())?;
	match resume {
		Some(saved) => return saved.continue_with(current).map_err(Into::into),
		None => return Ok(current),
	}
}

/// Prepare one or more observed categorical Bayesian target conditionals.
///
/// The semantic artifact retains exact observations and dictionaries. It does
/// not claim an optimizer loop: native inference performs histogramming and
/// Laplace posterior calculation.
///
/// # Errors
///
/// Returns the first invalid declaration, unsupported model combination, data preparation, or Bayesian artifact error.
pub fn compile_bayes_model(policy: &Train, data: &Data, model: &Model) -> TrainingResult<BayesModelArtifact> {
	policy.validate()?;
	data.validate()?;
	model.validate()?;
	if model.bayes_dependencies().is_empty() {
		return Err(TrainingError::unsupported(
			"Bayesian preparation requires at least one `.bayes(child, parents)` declaration",
		));
	}
	if !model.layers().is_empty() || model.weights_source().is_some() {
		return Err(TrainingError::unsupported(
			"observed categorical Bayesian preparation cannot combine layer blocks or a loaded model declaration",
		));
	}
	if model.objective().is_some() || model.gradient_clip_value().is_some() {
		return Err(TrainingError::unsupported(
			"the observed categorical posterior owns its likelihood and has no generic loss or gradient policy",
		));
	}
	if data.normalization().is_some() {
		return Err(TrainingError::unsupported(
			"categorical Bayesian node identities cannot be numerically normalized",
		));
	}
	if policy.optimizer_spec().is_some() || policy.learning_rate().is_some() || policy.learning_rate_schedule().is_some() || policy.warmup_epoch_bound().is_some() || policy.epoch_bound().is_some() {
		return Err(TrainingError::unsupported(
			"observed categorical Bayesian preparation has no optimizer, learning rate, warmup, or epoch loop",
		));
	}
	if !policy.log_items().is_empty() || !policy.plot_items().is_empty() {
		return Err(TrainingError::unsupported(
			"observed categorical Bayesian preparation has no iterative training metrics",
		));
	}
	if policy.resume_kernel_source().is_some() || policy.save_kernel_destination().is_some() {
		return Err(TrainingError::unsupported(
			"observed categorical Bayesian preparation does not realize a native training kernel artifact",
		));
	}
	let resume = if let Some(source) = policy.resume_source() {
		let path = Path::new(source);
		let exists = path
			.try_exists()
			.map_err(|error| return TrainingError::runtime("inspect Bayesian resume model", error.to_string()))?;
		if exists {
			Some(load_bayes_model_file(
				path,
				BayesModelDecodeLimits::default(),
			)?)
		} else {
			None
		}
	} else {
		None
	};
	let prepared = prepare_data(data)?;
	let dependencies = model
		.bayes_dependencies()
		.iter()
		.map(|dependency| return BayesianDependency::new(dependency.child(), dependency.parents()))
		.collect::<Vec<_>>();
	let current = BayesModelArtifact::from_conditionals(prepare_categorical_bayesian_reference_sets(
		&prepared,
		&dependencies,
	)?)?;
	match resume {
		Some(saved) => return saved.continue_with(current).map_err(Into::into),
		None => return Ok(current),
	}
}

fn compile_training_package(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTrainingPackage> {
	let (training, resume_checkpoint) = compile_training_graph(policy, data, model)?;
	let resume_native = load_resume_native_bundle(policy, &training, resume_checkpoint.as_ref())?;
	let checkpoint = CheckpointManifest::from_compiled(&training)?;
	return Ok(CompiledTrainingPackage {
		training,
		checkpoint,
		resume_native,
	});
}

fn compile_training_graph(policy: &Train, data: &Data, model: &Model) -> TrainingResult<(CompiledTraining, Option<CheckpointArtifact>)> {
	policy.validate()?;
	data.validate()?;
	model.validate()?;
	let loss = require_supported_model(model)?;
	require_supported_policy(policy)?;

	let prepared = prepare_data(data)?;
	let blocks = model.layers().to_vec();
	let epochs = policy
		.epoch_bound()
		.map(|epochs| {
			let epochs = u64::try_from(epochs).map_err(|error| {
				return TrainingError::unsupported(format!("epoch bound does not fit u64: {error}"));
			})?;
			return NonZeroU64::new(epochs)
				.map(TrainingHorizon::Finite)
				.ok_or_else(|| return TrainingError::unsupported("epoch bound must be nonzero"));
		})
		.transpose()?
		.unwrap_or(TrainingHorizon::Unbounded);
	let warmup_epochs = u64::try_from(policy.warmup_epoch_bound().unwrap_or(0)).map_err(|error| {
		return TrainingError::unsupported(format!("warmup epoch bound does not fit u64: {error}"));
	})?;
	let learning_rate = policy
		.learning_rate()
		.unwrap_or_else(|| return AdamWConfig::default().learning_rate);
	let learning_rate_decay = match (epochs, policy.learning_rate_schedule()) {
		(TrainingHorizon::Unbounded, Some(LearningRateSchedule::Linear)) | (_, Some(LearningRateSchedule::Constant)) => LearningRateSchedule::Constant,
		(TrainingHorizon::Unbounded, Some(LearningRateSchedule::Cosine)) => {
			return Err(TrainingError::unsupported(
				"cosine learning-rate decay requires `.epochs(...)` to define its endpoint",
			));
		}
		(TrainingHorizon::Unbounded, Some(LearningRateSchedule::Exponential)) => {
			return Err(TrainingError::unsupported(
				"exponential learning-rate decay requires `.epochs(...)` to define its endpoint",
			));
		}
		(TrainingHorizon::Finite(_), Some(LearningRateSchedule::Linear)) => LearningRateSchedule::Linear,
		(TrainingHorizon::Finite(_), Some(LearningRateSchedule::Cosine)) => LearningRateSchedule::Cosine,
		(TrainingHorizon::Finite(_), Some(LearningRateSchedule::Exponential)) => LearningRateSchedule::Exponential,
		(_, None) => {
			return Err(TrainingError::unsupported(
				"an explicit learning-rate decay is required",
			));
		}
	};
	let has_embedding = matches!(blocks.first(), Some(Block::Embedding { .. }));
	let data_normalization = match (has_embedding, data.normalization()) {
		(true, None) => DataNormalization::Identity,
		(true, Some(_)) => {
			return Err(TrainingError::unsupported(
				"embedding consumes exact int32 token IDs; do not declare numeric data normalization",
			));
		}
		(false, Some(DataNormalization::ZScore)) => DataNormalization::ZScore,
		(false, Some(DataNormalization::MinMax)) => DataNormalization::MinMax,
		(false, Some(DataNormalization::L2Norm)) => DataNormalization::L2Norm,
		(false, Some(DataNormalization::Identity)) => DataNormalization::Identity,
		(false, None) => {
			return Err(TrainingError::unsupported(
				"dense training requires explicit data normalization",
			));
		}
	};
	let binary_validation = binary_validation_config(policy, loss)?;
	let multiclass_validation = multiclass_validation_config(policy, loss)?;
	let regression_validation = regression_validation_config(policy, loss)?;
	if usize::from(binary_validation.is_some()) + usize::from(multiclass_validation.is_some()) + usize::from(regression_validation.is_some()) > 1 {
		return Err(TrainingError::unsupported(
			"one training run cannot request multiple validation metric families simultaneously",
		));
	}
	let mut training = compile_training(
		&prepared,
		&blocks,
		loss,
		data_normalization,
		epochs,
		warmup_epochs,
		learning_rate_decay,
		model.gradient_clip_value(),
		1.0e-6,
		MAXIMUM_REDUCTION_TREE_LANES,
		0x7265_6369_7065,
		AdamWConfig {
			learning_rate,
			..AdamWConfig::default()
		},
		binary_validation.as_ref(),
		multiclass_validation.as_ref(),
		regression_validation.as_ref(),
	)?;
	let mut resume_checkpoint = None;
	if let Some(source) = policy.resume_source() {
		let path = Path::new(source);
		let exists = path
			.try_exists()
			.map_err(|error| return TrainingError::runtime("inspect training resume model", error.to_string()))?;
		if exists {
			let checkpoint = load_checkpoint_file(path, CheckpointDecodeLimits::default())?;
			apply_checkpoint_resume(&mut training, &checkpoint)?;
			resume_checkpoint = Some(checkpoint);
		}
	}
	return Ok((training, resume_checkpoint));
}

fn load_resume_native_bundle(policy: &Train, training: &CompiledTraining, checkpoint: Option<&CheckpointArtifact>) -> TrainingResult<Option<ResumeNativeBundle>> {
	let Some(source) = policy.resume_kernel_source() else {
		return Ok(None);
	};
	let Some(checkpoint) = checkpoint else {
		return Ok(None);
	};
	let path = Path::new(source);
	let exists = path
		.try_exists()
		.map_err(|error| return TrainingError::runtime("inspect training resume kernel", error.to_string()))?;
	if !exists {
		return Ok(None);
	}
	let format = match path
		.extension()
		.and_then(|extension| return extension.to_str())
	{
		Some("cubin") => NativeKernelFormat::Cubin,
		Some("hsaco") => NativeKernelFormat::Hsaco,
		_ => unreachable!("validated native resume source has a known extension"),
	};
	let native = checkpoint.native_realization().ok_or_else(|| {
		return CheckpointError::IncompatibleResume {
			detail: "the semantic model predates native realization metadata, so the supplied kernel cannot be authenticated".to_owned(),
		};
	})?;
	let current_program = compiled_training_program_digest(training)?;
	if native.program() != current_program {
		return Err(CheckpointError::IncompatibleResume {
			detail: "the supplied native kernel was realized for a different static training program".to_owned(),
		}
		.into());
	}
	let matching = native
		.kernels()
		.iter()
		.filter(|kernel| return kernel.format() == format)
		.collect::<Vec<_>>();
	let kernel = match matching.as_slice() {
		[kernel] => *kernel,
		[] => {
			return Err(CheckpointError::IncompatibleResume {
				detail: format!(
					"the semantic model has no authenticated .{} realization",
					format.extension()
				),
			}
			.into());
		}
		kernels => {
			return Err(CheckpointError::IncompatibleResume {
				detail: format!(
					"the semantic model has {} authenticated .{} realizations; one resume path is ambiguous",
					kernels.len(),
					format.extension()
				),
			}
			.into());
		}
	};
	let source_limit = u64::try_from(CheckpointDecodeLimits::default().source_bytes).map_err(|error| return TrainingError::runtime("bound training resume kernel", error.to_string()))?;
	let snapshot = read_source_snapshot(
		path,
		SourceLimit::new(source_limit).map_err(TrainingError::NativeKernelSource)?,
	)
	.map_err(TrainingError::NativeKernelSource)?;
	if snapshot.digest() != kernel.digest() {
		return Err(CheckpointError::IncompatibleResume {
			detail: "the supplied native kernel bytes do not match the digest authenticated by the semantic model".to_owned(),
		}
		.into());
	}
	return Ok(Some(ResumeNativeBundle {
		topology: native.topology(),
		discovery: native.discovery(),
		target: kernel.target().clone(),
		toolchain: kernel.toolchain().clone(),
		bytes: Arc::from(snapshot.into_bytes()),
	}));
}

impl Train {
	/// Compile and execute this declaration with the preceding `recipe.data(...)`
	/// and `recipe.model()` declarations.
	///
	/// Preparation performs all discovery validation, placement, artifact
	/// generation, loading, warming, allocation, and finalization before the
	/// singular external data image for each device is admitted.
	///
	/// Expected declaration, preparation, execution, and artifact-write failures
	/// are returned as typed Recipe errors.
	///
	/// # Errors
	///
	/// Returns the first declaration, preparation, execution, resume, or requested artifact-write failure.
	pub fn run(&self) -> TrainingResult<TrainingReport> {
		let (data, model) = crate::take_recipe_training_sequence().map_err(TrainingError::unsupported)?;
		return self.try_run_with(&data, &model);
	}

	/// Source-frontend lowering target for `.run(&model, &data)`.
	#[doc(hidden)]
	pub fn __recipe_run_with(&self, model: &Model, data: &Data) -> TrainingResult<TrainingReport> { return self.try_run_with(data, model); }

	fn try_run_with(&self, data: &Data, model: &Model) -> TrainingResult<TrainingReport> {
		crate::api::select_log_output(self.log_items());
		if !model.bayes_dependencies().is_empty() {
			let report = TrainingReport::bayes(compile_bayes_model(self, data, model)?);
			if let Some(destination) = self.save_model_destination() {
				report.save_model(destination)?;
			}
			return Ok(report);
		}
		if model
			.layers()
			.iter()
			.any(|layer| matches!(layer, Block::Knn { .. }))
		{
			let report = TrainingReport::knn(compile_knn_model(self, data, model)?);
			if let Some(destination) = self.save_model_destination() {
				report.save_model(destination)?;
			}
			return Ok(report);
		}
		let package = compile_training_package(self, data, model)?;
		let interrupt = crate::signal::SigintGuard::install().map_err(|error| {
			return TrainingError::runtime("install graceful SIGINT handler", error.to_string());
		})?;
		let execution = execute_current_training(
			self,
			&package.training,
			package.resume_native.as_ref(),
			interrupt.request_flag(),
		)?;
		let report = TrainingReport::dense(
			execution,
			package.checkpoint,
			package.training.outputs().validation_status,
		)?;
		if let Some(destination) = self.save_model_destination() {
			report.save_model(destination)?;
		}
		if let Some(destination) = self.save_kernel_destination() {
			let format = match Path::new(destination)
				.extension()
				.and_then(|extension| return extension.to_str())
			{
				Some("cubin") => NativeKernelFormat::Cubin,
				Some("hsaco") => NativeKernelFormat::Hsaco,
				_ => unreachable!("validated native kernel destination has a known extension"),
			};
			report.save_native_kernel(destination, format)?;
		}
		return Ok(report);
	}
}

fn execute_current_training(policy: &Train, training: &CompiledTraining, resume_native: Option<&ResumeNativeBundle>, stop_requested: &std::sync::atomic::AtomicBool) -> TrainingResult<CompletedTrainingExecution> {
	report_unavailable_validation(training);
	let presentations = live_metric_presentations(policy, training);
	if presentations.is_empty() {
		let execution = execute_current_training_native(training, resume_native, None, stop_requested)?;
		report_static_training_metrics(policy, &execution);
		return Ok(execution);
	}
	let (mut observer, receiver) = bounded_training_metric_channel(
		LIVE_METRIC_CHANNEL_CAPACITY,
		presentations.metrics.keys().copied(),
		NonZeroU64::MIN,
	);
	let presenter = spawn_live_metric_presenter(receiver, presentations, training.bounds())?;
	let execution = execute_current_training_native(training, resume_native, Some(&mut observer), stop_requested);
	drop(observer);
	presenter.join().map_err(|panic| {
		return TrainingError::runtime(
			"join live metric presenter",
			thread_panic_detail(panic.as_ref()),
		);
	})?;
	let execution = execution?;
	report_static_training_metrics(policy, &execution);
	return Ok(execution);
}

fn report_unavailable_validation(training: &CompiledTraining) {
	let Some(line) = crate::validation_reporting::unavailable_validation_line(training.outputs().validation_status) else {
		return;
	};
	always(line);
}

fn report_static_training_metrics(policy: &Train, execution: &CompletedTrainingExecution) {
	let log_device = policy
		.log_items()
		.iter()
		.any(|item| return item.metric() == Metric::Device);
	let plot_device = policy
		.plot_items()
		.iter()
		.any(|item| return item.metric() == Metric::Device);
	if !log_device && !plot_device {
		return;
	}
	let devices = execution
		.native_kernels()
		.kernels()
		.iter()
		.map(|kernel| {
			let target = kernel.target();
			return format!("{}:{}:{}", target.backend, target.architecture, target.abi);
		})
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect::<Vec<_>>()
		.join(",");
	let devices = if devices.is_empty() {
		"unavailable".to_owned()
	} else {
		devices
	};
	if log_device {
		line(write::device, format!("device {devices}"));
	}
	if plot_device {
		always(format!("plot device {devices}"));
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeRuntimeTuning {
	pub worker_threads: usize,
	pub staging_bytes_per_worker: usize,
	pub watchdog: Watchdog,
}

fn select_host_worker_threads(available_parallelism: usize, measured_host_lanes: usize, realized_opportunities: usize) -> usize {
	return available_parallelism
		.min(measured_host_lanes)
		.min(realized_opportunities)
		.max(1);
}

fn select_host_staging_bytes(maximum_value_bytes: u64, measured_staging_scale: u64, local_ram_bytes: u64, worker_threads: u64) -> u64 {
	return maximum_value_bytes
		.min(measured_staging_scale)
		.min(local_ram_bytes / worker_threads.max(1))
		.max(1);
}

pub(crate) fn derive_native_runtime_tuning(graph: &CalculationGraph, profile: &MeasuredProfile, machine: MachineId) -> TrainingResult<NativeRuntimeTuning> {
	let local_devices = profile
		.discovery
		.devices
		.iter()
		.filter(|discovered| {
			return profile
				.topology
				.device(discovered.device)
				.is_some_and(|device| return device.machine == machine);
		})
		.collect::<Vec<_>>();
	if local_devices.is_empty() {
		return Err(TrainingError::runtime(
			"derive native training tuning",
			"measured local machine has no discovered devices",
		));
	}

	let realized_opportunities = graph.tensors.len().max(1);
	let measured_host_lanes = local_devices
		.iter()
		.filter(|discovered| {
			return profile
				.topology
				.device(discovered.device)
				.is_some_and(|device| matches!(device.kind, DeviceKind::Ram | DeviceKind::Disk));
		})
		.try_fold(0u64, |total, discovered| {
			let lanes = u64::from(
				discovered
					.transfer
					.maximum_inflight_transfers
					.value
					.get()
					.min(discovered.maximum_submission_queues),
			);
			return total.checked_add(lanes);
		})
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"host lane count overflowed",
			);
		})?;
	let measured_host_lanes = usize::try_from(measured_host_lanes)
		.map_err(|error| return TrainingError::runtime("derive native training tuning", error.to_string()))?
		.max(1);
	let available_parallelism = std::thread::available_parallelism()
		.map(NonZeroUsize::get)
		.map_err(|error| return TrainingError::runtime("query host parallelism", error.to_string()))?;
	let worker_threads = select_host_worker_threads(
		available_parallelism,
		measured_host_lanes,
		realized_opportunities,
	);

	let maximum_value_bytes = graph
		.tensors
		.iter()
		.map(|tensor| return tensor.storage_bytes)
		.max_by_key(|bytes| return bytes.get())
		.unwrap_or(ByteCount::new(1));
	let external_input_bytes = graph
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_input)
		.try_fold(0u64, |total, tensor| {
			return total.checked_add(tensor.storage_bytes.get());
		})
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"external input image size overflowed",
			);
		})?;
	let external_output_bytes = graph
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_output)
		.try_fold(0u64, |total, tensor| {
			return total.checked_add(tensor.storage_bytes.get());
		})
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"external output image size overflowed",
			);
		})?;
	let maximum_transfer_bytes = ByteCount::new(
		maximum_value_bytes
			.get()
			.max(external_input_bytes)
			.max(external_output_bytes),
	);
	let has_local_disk = profile
		.topology
		.devices
		.iter()
		.any(|device| return device.machine == machine && device.kind == DeviceKind::Disk);
	let measured_staging_scale = if has_local_disk {
		profile.benchmarks.storage.buffer_bytes
	} else {
		profile.benchmarks.ram.buffer_bytes
	};
	let local_ram_bytes = local_devices
		.iter()
		.filter(|discovered| {
			return profile
				.topology
				.device(discovered.device)
				.is_some_and(|device| return device.kind == DeviceKind::Ram);
		})
		.try_fold(0u64, |total, discovered| {
			return total.checked_add(discovered.total_capacity.value.get());
		})
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"measured RAM capacity overflowed",
			);
		})?;
	if local_ram_bytes == 0 {
		return Err(TrainingError::runtime(
			"derive native training tuning",
			"measured local machine has no RAM capacity for host staging",
		));
	}
	let worker_count = u64::try_from(worker_threads).map_err(|error| return TrainingError::runtime("derive native training tuning", error.to_string()))?;
	let staging_bytes = select_host_staging_bytes(
		maximum_value_bytes.get(),
		measured_staging_scale.get(),
		local_ram_bytes,
		worker_count,
	);
	let staging_bytes_per_worker = usize::try_from(staging_bytes).map_err(|error| return TrainingError::runtime("derive native training tuning", error.to_string()))?;

	let tensor_index = graph
		.tensors
		.iter()
		.map(|tensor| return (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	let maximum_work = graph
		.nodes
		.iter()
		.map(|node| return node.kernel.work(&tensor_index))
		.collect::<Result<Vec<_>, _>>()
		.map_err(|error| return TrainingError::runtime("derive native training tuning", error.to_string()))?
		.into_iter()
		.max_by_key(|work| return work.get())
		.unwrap_or(FlopCount::ZERO);
	let slowest_calculation_rate = local_devices
		.iter()
		.filter_map(|discovered| {
			return discovered
				.calculation
				.as_ref()
				.map(|calculation| return calculation.rate.value);
		})
		.min_by_key(|rate| return rate.get())
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"measured local machine has no calculation rate",
			);
		})?;
	let calculation_duration = calculation_time_ceil(maximum_work, slowest_calculation_rate).map_err(|error| return TrainingError::runtime("derive native training tuning", error.to_string()))?;
	let slowest_device_transfer_rate = local_devices
		.iter()
		.map(|discovered| return discovered.transfer.rate.value)
		.min_by_key(|rate| return rate.get());
	let slowest_local_link_rate = profile
		.discovery
		.links
		.iter()
		.filter_map(|discovered| {
			let link = profile.topology.link(discovered.link)?;
			let source = profile.topology.device(link.from)?;
			let destination = profile.topology.device(link.to)?;
			return (source.machine == machine && destination.machine == machine).then_some(discovered.bandwidth.value);
		})
		.min_by_key(|rate| return rate.get());
	let slowest_transfer_rate = slowest_device_transfer_rate
		.into_iter()
		.chain(slowest_local_link_rate)
		.min_by_key(|rate| return rate.get())
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"measured local machine has no transfer rate",
			);
		})?;
	let transfer_duration = transfer_time_ceil(maximum_transfer_bytes, slowest_transfer_rate).map_err(|error| return TrainingError::runtime("derive native training tuning", error.to_string()))?;
	let expected_operation = Duration::from_nanos(
		calculation_duration
			.get()
			.max(transfer_duration.get())
			.max(1),
	);
	let measured_concurrency = local_devices
		.iter()
		.try_fold(0u64, |total, discovered| {
			let calculation = discovered.calculation.as_ref().map_or(0u64, |calculation| {
				return u64::from(calculation.maximum_concurrent_tasks);
			});
			let transfer = u64::from(discovered.transfer.maximum_inflight_transfers.value.get());
			return total.checked_add(calculation)?.checked_add(transfer);
		})
		.ok_or_else(|| {
			return TrainingError::runtime(
				"derive native training tuning",
				"device concurrency overflowed",
			);
		})?
		.max(1)
		.min(u64::from(u32::MAX));
	let measured_concurrency = u32::try_from(measured_concurrency).map_err(|error| {
		return TrainingError::runtime("derive native training tuning", error.to_string());
	})?;
	let safety_multiplier = NonZeroU32::MIN.saturating_add(measured_concurrency - 1);
	let watchdog = Watchdog::for_expected_duration(expected_operation, safety_multiplier);

	return Ok(NativeRuntimeTuning {
		worker_threads,
		staging_bytes_per_worker,
		watchdog,
	});
}

fn execute_current_training_native(training: &CompiledTraining, resume_native: Option<&ResumeNativeBundle>, observer: Option<&mut TrainingMetricObserver>, stop_requested: &std::sync::atomic::AtomicBool) -> TrainingResult<CompletedTrainingExecution> {
	let run = next_run_id();
	let result = with_current_native_preparation(|profile, _config, scope| {
		if let Some(native) = resume_native {
			if profile.topology.identity != native.topology || profile.discovery.identity != native.discovery {
				return Err(NativePreparationError::IdentityMismatch {
					detail: "supplied native kernel was realized for a different measured topology or discovery profile".to_owned(),
				});
			}
			let matches_current_target = scope.targets().target_build_specs().iter().any(|spec| {
				return spec.target_identity == native.target && spec.toolchain_identity == native.toolchain;
			});
			if !matches_current_target {
				return Err(NativePreparationError::IdentityMismatch {
					detail: "supplied native kernel target or toolchain differs from the current measured target plan".to_owned(),
				});
			}
		}
		let (bindings, host, targets) = scope.into_parts();
		let tuning = derive_native_runtime_tuning(training.graph(), profile, host.machine()).map_err(|error| return NativePreparationError::LocalConfiguration(error.to_string()))?;
		let limits = TrainingExecutionLimits::new(tuning.watchdog);
		let host = host
			.backend_config(run, tuning.worker_threads, tuning.staging_bytes_per_worker)
			.map_err(|error| return NativePreparationError::LocalConfiguration(error.to_string()))?;
		let (cuda, hsa) = bindings.into_parts();
		let bridge = StagedCrossBackend::new(cuda.clone(), hsa.clone());
		let factory = LocalCandidateFactory::production(Some(host), cuda, hsa, bridge);
		let driver = NativeExecutorDriver::new(factory);
		let mut compiler = targets
			.deferred_compiler()
			.map_err(NativePreparationError::TargetSpecification)?;
		if let Some(native) = resume_native {
			compiler = compiler
				.with_prebuilt_bundle(native.target.clone(), Arc::clone(&native.bytes))
				.map_err(NativePreparationError::TargetSpecification)?;
		}
		let realizer = NativeCandidateRealizer::new(profile, compiler, driver).map_err(|error| return NativePreparationError::LocalConfiguration(error.to_string()))?;
		let provider = NativeArtifactProvider::new(NativeArtifactCatalog::default());
		let mut preparer = Preparer::new(provider, realizer);
		let execution = prepare_and_execute_local_training_controlled(
			training,
			profile,
			&mut preparer,
			run,
			limits,
			observer,
			TrainingExecutionControl::graceful_stop(stop_requested),
		);
		return Ok(execution);
	})?;
	return result.map_err(|error| return TrainingError::runtime("execute native training", error.to_string()));
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveMetricPresentation {
	label: String,
	width: usize,
	log_cadence: Option<NonZeroU64>,
	plot: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LiveHostPresentation {
	epoch_log_cadence: Option<NonZeroU64>,
	epoch_plot: bool,
	time_log_cadence: Option<NonZeroU64>,
	time_plot: bool,
}

impl LiveHostPresentation {
	const fn requested(&self) -> bool { return self.epoch_log_cadence.is_some() || self.epoch_plot || self.time_log_cadence.is_some() || self.time_plot; }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LiveMetricPlan {
	metrics: BTreeMap<MetricId, LiveMetricPresentation>,
	host: LiveHostPresentation,
}

impl LiveMetricPlan {
	fn is_empty(&self) -> bool { return self.metrics.is_empty(); }
}

fn live_metric_presentations(policy: &Train, training: &CompiledTraining) -> LiveMetricPlan {
	let mut plan = LiveMetricPlan {
		host: LiveHostPresentation {
			epoch_log_cadence: policy.metric_log_interval(Metric::Epoch),
			epoch_plot: policy
				.plot_items()
				.iter()
				.any(|item| return item.metric() == Metric::Epoch),
			time_log_cadence: policy.metric_log_interval(Metric::Time),
			time_plot: policy
				.plot_items()
				.iter()
				.any(|item| return item.metric() == Metric::Time),
		},
		..LiveMetricPlan::default()
	};
	for binding in &training.outputs().metric_bindings {
		let metric = log_selects_binding_to_metric(binding.kind);
		let logged = policy
			.log_items()
			.iter()
			.any(|item| return log_selects(item.metric(), binding.kind));
		let plotted = policy
			.plot_items()
			.iter()
			.any(|item| return log_selects(item.metric(), binding.kind));
		if !logged && !plotted {
			continue;
		}
		plan.metrics.insert(binding.metric, LiveMetricPresentation {
			label: metric_label(binding.kind),
			width: metric_width(binding.kind),
			log_cadence: logged
				.then(|| return policy.metric_log_interval(metric))
				.flatten(),
			plot: plotted,
		});
	}
	if plan.host.requested()
		&& let Some(trigger) = training
			.outputs()
			.metric_bindings
			.iter()
			.find(|binding| return binding.kind == TrainingMetricKind::TrainingLoss)
	{
		plan.metrics.entry(trigger.metric).or_insert_with(|| {
			return LiveMetricPresentation {
				label: metric_label(trigger.kind),
				width: metric_width(trigger.kind),
				log_cadence: None,
				plot: false,
			};
		});
	}
	return plan;
}

const fn log_selects_binding_to_metric(binding: TrainingMetricKind) -> Metric {
	match binding {
		TrainingMetricKind::TrainingLoss | TrainingMetricKind::ValidationMeanBce | TrainingMetricKind::ValidationMeanCrossEntropy | TrainingMetricKind::RecallAt { .. } => return Metric::Loss,
		TrainingMetricKind::Accuracy => return Metric::Accuracy,
		TrainingMetricKind::R2 => return Metric::R2,
		TrainingMetricKind::LearningRate => return Metric::LearningRate,
		TrainingMetricKind::AuRoc => return Metric::AuRoc,
		TrainingMetricKind::AuPrc => return Metric::AuPrc,
		TrainingMetricKind::BrierScore => return Metric::Brier,
		TrainingMetricKind::ExpectedCalibrationError => return Metric::CalibrationError,
	}
}

fn log_selects(requested: Metric, available: TrainingMetricKind) -> bool {
	match requested {
		Metric::Loss => {
			return matches!(
				available,
				TrainingMetricKind::TrainingLoss | TrainingMetricKind::ValidationMeanBce | TrainingMetricKind::ValidationMeanCrossEntropy
			);
		}
		Metric::Accuracy => return available == TrainingMetricKind::Accuracy,
		Metric::R2 => return available == TrainingMetricKind::R2,
		Metric::AuRoc => return available == TrainingMetricKind::AuRoc,
		Metric::AuPrc => return available == TrainingMetricKind::AuPrc,
		Metric::Brier => return available == TrainingMetricKind::BrierScore,
		Metric::CalibrationError => return available == TrainingMetricKind::ExpectedCalibrationError,
		Metric::LearningRate => return available == TrainingMetricKind::LearningRate,
		Metric::Epoch | Metric::Time | Metric::Device => return false,
	}
}

fn metric_label(metric: TrainingMetricKind) -> String {
	match metric {
		TrainingMetricKind::TrainingLoss => return "loss".to_owned(),
		TrainingMetricKind::ValidationMeanBce | TrainingMetricKind::ValidationMeanCrossEntropy => {
			return "validation_loss".to_owned();
		}
		TrainingMetricKind::Accuracy => return "accuracy".to_owned(),
		TrainingMetricKind::R2 => return "r2".to_owned(),
		TrainingMetricKind::LearningRate => return "lr".to_owned(),
		TrainingMetricKind::AuRoc => return "auroc".to_owned(),
		TrainingMetricKind::AuPrc => return "auprc".to_owned(),
		TrainingMetricKind::BrierScore => return "brier".to_owned(),
		TrainingMetricKind::ExpectedCalibrationError => return "calibration_error".to_owned(),
		TrainingMetricKind::RecallAt { threshold_bits } => {
			return format!("recall@{}", f32::from_bits(threshold_bits));
		}
	}
}

const fn metric_width(metric: TrainingMetricKind) -> usize {
	match metric {
		TrainingMetricKind::TrainingLoss | TrainingMetricKind::ValidationMeanBce | TrainingMetricKind::ValidationMeanCrossEntropy => return 7,
		TrainingMetricKind::Accuracy | TrainingMetricKind::R2 | TrainingMetricKind::LearningRate | TrainingMetricKind::AuRoc | TrainingMetricKind::AuPrc | TrainingMetricKind::BrierScore | TrainingMetricKind::ExpectedCalibrationError | TrainingMetricKind::RecallAt { .. } => return 6,
	}
}

fn spawn_live_metric_presenter(receiver: Receiver<TrainingMetricSample>, presentations: LiveMetricPlan, bounds: TrainingBounds) -> TrainingResult<JoinHandle<()>> {
	return std::thread::Builder::new()
		.name("recipe-live-metrics".to_owned())
		.spawn(move || return present_live_metrics(&receiver, presentations, bounds))
		.map_err(|error| return TrainingError::runtime("start live metric presenter", error.to_string()));
}

fn present_live_metrics(receiver: &Receiver<TrainingMetricSample>, presentations: LiveMetricPlan, bounds: TrainingBounds) {
	let mut rows = LiveMetricRows::new(presentations, bounds.epochs);
	while let Ok(sample) = receiver.recv() {
		if let Some(row) = rows.push(
			sample.zero_based_iteration.index(),
			sample.epoch,
			sample.metric,
			sample.value,
		) {
			always(row);
		}
	}
	if let Some(row) = rows.finish() {
		always(row);
	}
	for plot in rows.plot_lines() {
		always(plot);
	}
}

fn thread_panic_detail(panic: &dyn std::any::Any) -> String {
	if let Some(message) = panic.downcast_ref::<&str>() {
		return (*message).to_owned();
	}
	if let Some(message) = panic.downcast_ref::<String>() {
		return message.clone();
	}
	return "non-string panic payload".to_owned();
}

#[derive(Debug)]
struct LiveMetricRows {
	plan: LiveMetricPlan,
	epochs: TrainingHorizon,
	started: Instant,
	pending_sample: Option<(u64, NonZeroU64)>,
	pending_values: BTreeMap<MetricId, MetricValue>,
	plots: BTreeMap<String, BoundedPlotSeries>,
}

impl LiveMetricRows {
	fn new(plan: LiveMetricPlan, epochs: impl Into<TrainingHorizon>) -> Self {
		return Self {
			plan,
			epochs: epochs.into(),
			started: Instant::now(),
			pending_sample: None,
			pending_values: BTreeMap::new(),
			plots: BTreeMap::new(),
		};
	}

	fn push(&mut self, iteration: u64, epoch: NonZeroU64, metric: MetricId, value: MetricValue) -> Option<String> {
		let completed = if self
			.pending_sample
			.is_some_and(|(pending, _)| return pending != iteration)
		{
			let completed = self
				.pending_sample
				.and_then(|(_, epoch)| return self.render(epoch, false));
			self.pending_sample = None;
			self.pending_values.clear();
			completed
		} else {
			None
		};
		self.pending_sample = Some((iteration, epoch));
		if self.plan.metrics.contains_key(&metric) {
			self.pending_values.insert(metric, value);
		}
		return completed;
	}

	fn finish(&mut self) -> Option<String> {
		let (_, epoch) = self.pending_sample.take()?;
		let row = self.render(epoch, true);
		self.pending_values.clear();
		return row;
	}

	fn render(&mut self, epoch: NonZeroU64, final_flush: bool) -> Option<String> {
		let final_epoch = final_flush
			|| self
				.epochs
				.bound()
				.is_some_and(|bound| return epoch == bound);
		let elapsed = self.started.elapsed().as_secs_f64();
		if self.plan.host.epoch_plot {
			self.record_plot("epoch", epoch.get() as f64);
		}
		if self.plan.host.time_plot {
			self.record_plot("time", elapsed);
		}
		let plotted = self
			.plan
			.metrics
			.iter()
			.filter_map(|(metric, presentation)| {
				return presentation
					.plot
					.then(|| {
						return self
							.pending_values
							.get(metric)
							.copied()
							.map(|value| return (presentation.label.clone(), value));
					})
					.flatten();
			})
			.collect::<Vec<_>>();
		for (label, value) in plotted {
			self.record_plot(&label, metric_value_f64(value));
		}

		let mut fields = Vec::with_capacity(self.pending_values.len().saturating_add(2));
		let epoch_due = self
			.plan
			.host
			.epoch_log_cadence
			.is_some_and(|cadence| return epoch.get().is_multiple_of(cadence.get()) || final_epoch);
		let time_due = self
			.plan
			.host
			.time_log_cadence
			.is_some_and(|cadence| return epoch.get().is_multiple_of(cadence.get()) || final_epoch);
		if time_due {
			fields.push(live_metric_field("time", 9, &format!("{elapsed:.3}s"), 1));
		}
		for (metric, presentation) in &self.plan.metrics {
			let Some(value) = self.pending_values.get(metric) else {
				continue;
			};
			let log_due = presentation
				.log_cadence
				.is_some_and(|cadence| return epoch.get().is_multiple_of(cadence.get()) || final_epoch);
			if !log_due {
				continue;
			}
			let rendered = render_live_metric_value(*value, presentation.width);
			fields.push(live_metric_field(
				&presentation.label,
				presentation.width,
				&rendered,
				fields.len().saturating_add(1),
			));
		}
		if fields.is_empty() && !epoch_due {
			return None;
		}
		fields.insert(0, live_metric_field("epoch", 5, &epoch.to_string(), 0));
		return Some(fields.join("  "));
	}

	fn record_plot(&mut self, label: &str, value: f64) { self.plots.entry(label.to_owned()).or_default().push(value); }

	fn plot_lines(&self) -> Vec<String> {
		return self
			.plots
			.iter()
			.map(|(label, series)| return series.render(label))
			.collect();
	}
}

const LIVE_PLOT_WIDTH: usize = 64;

#[derive(Debug, Default)]
struct BoundedPlotSeries {
	values: VecDeque<f64>,
	total: u64,
}

impl BoundedPlotSeries {
	fn push(&mut self, value: f64) {
		self.total = self.total.saturating_add(1);
		if self.values.len() == LIVE_PLOT_WIDTH {
			self.values.pop_front();
		}
		self.values.push_back(value);
	}

	fn render(&self, label: &str) -> String {
		let finite = self
			.values
			.iter()
			.copied()
			.filter(|value| return value.is_finite())
			.collect::<Vec<_>>();
		let (minimum, maximum) = finite
			.iter()
			.copied()
			.fold(None, |range, value| {
				match range {
					Some((minimum, maximum)) => {
						return Some((f64::min(minimum, value), f64::max(maximum, value)));
					}
					None => return Some((value, value)),
				}
			})
			.unwrap_or((f64::NAN, f64::NAN));
		let levels = [
			'\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
		];
		let plot = self
			.values
			.iter()
			.map(|value| {
				if !value.is_finite() {
					return '\u{b7}';
				}
				if minimum.to_bits() == maximum.to_bits() {
					return levels[3];
				}
				let scaled = ((*value - minimum) / (maximum - minimum)) * (levels.len() - 1) as f64;
				return levels[scaled.round() as usize];
			})
			.collect::<String>();
		if minimum.is_finite() {
			return format!(
				"plot {label} {plot}  range {minimum:.4}..{maximum:.4}  samples {}",
				self.total
			);
		}
		return format!("plot {label} {plot}  range N/A  samples {}", self.total);
	}
}

fn metric_value_f64(value: MetricValue) -> f64 {
	match value {
		MetricValue::F32(value) => return f64::from(value),
		MetricValue::I32(value) => return f64::from(value),
	}
}

fn render_live_metric_value(value: MetricValue, width: usize) -> String {
	match value {
		MetricValue::F32(value) if value.is_nan() => return format!("{:>width$}", "N/A"),
		MetricValue::F32(value) => return format!("{value:>width$.4}"),
		MetricValue::I32(value) => return format!("{value:>width$}"),
	}
}

fn live_metric_field(label: &str, width: usize, value: &str, color: usize) -> String {
	let (red, green, blue) = LIVE_METRIC_PALETTE[color % LIVE_METRIC_PALETTE.len()];
	return format!("\x1b[38;2;{red};{green};{blue}m{label}\x1b[0m {value:>width$}");
}

pub(crate) fn next_run_id() -> RunId {
	let sequence = NEXT_RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
	let epoch = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map_or(0, |duration| {
			return duration.as_secs() ^ u64::from(duration.subsec_nanos()).rotate_left(32);
		});
	let process = u64::from(std::process::id());
	let value = epoch ^ process.rotate_left(19) ^ sequence.rotate_left(41);
	return RunId::new(value.max(1));
}

fn require_supported_model(model: &Model) -> TrainingResult<Loss> {
	if model.weights_source().is_some() {
		return Err(TrainingError::unsupported(
			"loading an existing weight image is not part of the dense training compiler",
		));
	}
	if !model.bayes_dependencies().is_empty() {
		return Err(TrainingError::unsupported(
			"Bayesian dependencies require a Bayesian model compiler and cannot be ignored by dense training",
		));
	}
	match model.objective() {
		Some(Objective::Builtin(Loss::BinaryCrossEntropy)) => return Ok(Loss::BinaryCrossEntropy),
		Some(Objective::Builtin(Loss::MeanSquaredError)) => return Ok(Loss::MeanSquaredError),
		Some(Objective::Builtin(Loss::MeanAbsoluteError)) => return Ok(Loss::MeanAbsoluteError),
		Some(Objective::Builtin(Loss::CrossEntropy)) => return Ok(Loss::CrossEntropy),
		Some(Objective::Builtin(Loss::Huber)) => return Ok(Loss::Huber),
		Some(Objective::Builtin(Loss::Focal)) => return Ok(Loss::Focal),
		Some(Objective::Reference(_)) => {
			return Err(TrainingError::unsupported(
				"model-referenced objectives have no dense training lowering",
			));
		}
		None => {
			return Err(TrainingError::unsupported(
				"dense training requires an explicit built-in objective",
			));
		}
	}
}

fn require_supported_policy(policy: &Train) -> TrainingResult<()> {
	if policy.resume_kernel_source().is_some() && policy.resume_source().is_none() {
		return Err(TrainingError::unsupported(
			"a native resume kernel requires a semantic .ogdl model",
		));
	}
	if policy.optimizer_spec() != Some(Optimizer::AdamW) {
		return Err(TrainingError::unsupported(
			"dense training requires the Recipe-owned AdamW optimizer on the training declaration",
		));
	}
	if policy.learning_rate_schedule().is_none() {
		return Err(TrainingError::unsupported(
			"dense training requires an explicit learning-rate decay",
		));
	}
	return Ok(());
}

fn binary_validation_config(policy: &Train, loss: Loss) -> TrainingResult<Option<BinaryValidationConfig>> {
	let mut requested = false;
	for item in policy.log_items().iter().chain(policy.plot_items()) {
		match item.metric() {
			Metric::Loss | Metric::Epoch | Metric::LearningRate | Metric::Time | Metric::Device => {}
			Metric::AuRoc | Metric::AuPrc | Metric::Brier | Metric::CalibrationError => {
				requested = true;
			}
			Metric::Accuracy if matches!(loss, Loss::BinaryCrossEntropy | Loss::Focal) => {
				requested = true;
			}
			Metric::Accuracy if loss == Loss::CrossEntropy => {}
			Metric::R2
				if matches!(
					loss,
					Loss::MeanSquaredError | Loss::MeanAbsoluteError | Loss::Huber
				) => {}
			Metric::Accuracy | Metric::R2 => {
				return Err(TrainingError::unsupported(format!(
					"metric {:?} is not defined for the current training objective",
					item.metric()
				)));
			}
		}
	}
	if !requested {
		return Ok(None);
	}
	if !matches!(loss, Loss::BinaryCrossEntropy | Loss::Focal) {
		return Err(TrainingError::unsupported(
			"binary classification validation metrics and calibration require BCE or focal loss",
		));
	}
	let bins = NonZeroU32::MIN.saturating_add(14);
	return Ok(Some(BinaryValidationConfig::new(bins, Vec::new())));
}

fn multiclass_validation_config(policy: &Train, loss: Loss) -> TrainingResult<Option<MulticlassValidationConfig>> {
	let requested = policy
		.log_items()
		.iter()
		.chain(policy.plot_items())
		.any(|item| return item.metric() == Metric::Accuracy);
	if !requested {
		return Ok(None);
	}
	match loss {
		Loss::CrossEntropy => return Ok(Some(MulticlassValidationConfig)),
		Loss::BinaryCrossEntropy | Loss::Focal => return Ok(None),
		Loss::MeanSquaredError | Loss::MeanAbsoluteError | Loss::Huber => {
			return Err(TrainingError::unsupported(
				"accuracy validation requires binary or categorical cross entropy",
			));
		}
	}
}

fn regression_validation_config(policy: &Train, loss: Loss) -> TrainingResult<Option<RegressionValidationConfig>> {
	let requested = policy
		.log_items()
		.iter()
		.chain(policy.plot_items())
		.any(|item| return item.metric() == Metric::R2);
	if !requested {
		return Ok(None);
	}
	if !matches!(
		loss,
		Loss::MeanSquaredError | Loss::MeanAbsoluteError | Loss::Huber
	) {
		return Err(TrainingError::unsupported(
			"R2 validation requires a scalar regression loss",
		));
	}
	return Ok(Some(RegressionValidationConfig));
}
