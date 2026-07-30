use core::fmt;
use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use recipe_core::{
	BundleIdentity, ByteCount, DeviceKind, DiscoveryIdentity, FlopCount, MachineId, MetricId, RunId, TargetIdentity,
	ToolchainIdentity, TopologyIdentity, calculation_time_ceil, transfer_time_ceil,
};
use recipe_executor::{ExitImage, LogicalEvent, MetricValue, Watchdog};
use recipe_ingest::{SourceError, SourceLimit, read_source_snapshot};
use recipe_language::CalculationGraph;
use recipe_native_executor::{LocalCandidateFactory, StagedCrossBackend};
use recipe_prepare::{
	NativeArtifactCatalog, NativeArtifactProvider, NativeCandidateRealizer, NativeExecutorDriver, Preparer,
};
use recipe_probe::MeasuredProfile;
use recipe_training::{
	AdamWConfig, BayesModelArtifact, BayesModelDecodeLimits, BayesianDependency, BinaryValidationConfig,
	CheckpointArtifact, CheckpointDecodeLimits, CheckpointError, CheckpointManifest, CompiledTraining,
	CompletedTrainingCheckpoint, CompletedTrainingExecution, DenseActivation, DenseAttention, DenseBlock,
	DenseBlockKind, DenseConvolution, DenseDataNormalization, DenseEmbedding, DenseGru, DenseKMeans, DenseLayer,
	DenseLoss, DenseLstm, DenseNormalization, DenseOperation, DensePool, DenseResidual, DenseResidualOperation,
	DenseRnn, DenseTrainingConfig, DenseTree, DenseTreeFamily, FinalTrainingMetric, InferencePreparationError,
	KnnModelArtifact, KnnModelDecodeLimits, LearningRateDecay, MAXIMUM_REDUCTION_TREE_LANES,
	MulticlassValidationConfig, NativeKernelFormat, RegressionValidationConfig, TrainingBounds, TrainingCompileError,
	TrainingExecutionControl, TrainingExecutionLimits, TrainingHorizon, TrainingMetricKind, TrainingMetricObserver,
	TrainingMetricSample, ValidationMetricStatus, apply_checkpoint_resume, bounded_training_metric_channel,
	compile_dense_training, compile_dense_training_with_binary_validation, compile_dense_training_with_blocks,
	compile_dense_training_with_blocks_and_binary_validation,
	compile_dense_training_with_blocks_and_multiclass_validation,
	compile_dense_training_with_blocks_and_regression_validation, compile_dense_training_with_multiclass_validation,
	compile_dense_training_with_regression_validation, compiled_training_program_digest, load_bayes_model_file,
	load_checkpoint_file, load_knn_model_file, prepare_and_execute_local_training_controlled,
	prepare_categorical_bayesian_reference_sets, prepare_knn_reference_set,
};

use crate::api::{
	Activation, Data, DataNormalization, DeclarationError, ForestBooster, LayerNormalization, LayerOperation,
	LayerSpec, LearningRateSchedule, Loss, Metric, Model, Objective, Optimizer, ResidualOperation, ResidualSkip,
	Train,
};
use crate::data_prepare::{DataPreparationError, prepare_data};
use crate::native_prepare::{NativePreparationError, with_current_native_preparation};

const LIVE_METRIC_CHANNEL_CAPACITY: usize = 256;
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
		Self::Unsupported {
			detail: detail.into(),
		}
	}

	pub(crate) fn runtime(stage: &'static str, detail: impl Into<String>) -> Self {
		Self::Runtime {
			stage,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for TrainingError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Declaration(error) => write!(formatter, "invalid training declaration: {error}"),
			Self::Data(error) => write!(formatter, "prepare training data: {error}"),
			Self::Compile(error) => write!(formatter, "compile training graph: {error}"),
			Self::Checkpoint(error) => write!(formatter, "save semantic model or native kernel: {error}"),
			Self::Resume(error) => write!(formatter, "load training resume model: {error}"),
			Self::NativeKernelSource(error) => write!(formatter, "load training resume kernel: {error}"),
			Self::Native(error) => write!(formatter, "prepare current native system: {error}"),
			Self::Unsupported { detail } => write!(formatter, "unsupported training declaration: {detail}"),
			Self::Runtime { stage, detail } => write!(formatter, "{stage}: {detail}"),
		}
	}
}

impl fmt::Debug for TrainingError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, formatter)
	}
}

impl std::error::Error for TrainingError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Declaration(error) => Some(error),
			Self::Data(error) => Some(error),
			Self::Compile(error) => Some(error),
			Self::Checkpoint(error) => Some(error),
			Self::Resume(error) => Some(error),
			Self::NativeKernelSource(error) => Some(error),
			Self::Native(error) => Some(error),
			Self::Unsupported { .. } | Self::Runtime { .. } => None,
		}
	}
}

impl From<DeclarationError> for TrainingError {
	fn from(error: DeclarationError) -> Self {
		Self::Declaration(error)
	}
}

impl From<DataPreparationError> for TrainingError {
	fn from(error: DataPreparationError) -> Self {
		Self::Data(error)
	}
}

impl From<TrainingCompileError> for TrainingError {
	fn from(error: TrainingCompileError) -> Self {
		Self::Compile(error)
	}
}

impl From<CheckpointError> for TrainingError {
	fn from(error: CheckpointError) -> Self {
		Self::Checkpoint(error)
	}
}

impl From<InferencePreparationError> for TrainingError {
	fn from(error: InferencePreparationError) -> Self {
		Self::Resume(error)
	}
}

impl From<NativePreparationError> for TrainingError {
	fn from(error: NativePreparationError) -> Self {
		Self::Native(error)
	}
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
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("TrainingReport")
			.field("kind", &self.kind())
			.field("run", &self.run())
			.field("bundle", &self.bundle())
			.field("external_output_count", &self.external_outputs().len())
			.field("metric_count", &self.metrics().len())
			.field("validation_status", &self.validation_status)
			.field("gracefully_stopped", &self.gracefully_stopped)
			.finish()
	}
}

impl TrainingReport {
	fn dense(
		execution: CompletedTrainingExecution,
		manifest: CheckpointManifest,
		validation_status: ValidationMetricStatus,
	) -> TrainingResult<Self> {
		let gracefully_stopped = execution
			.journal()
			.logical_events()
			.iter()
			.any(|event| matches!(event, LogicalEvent::LoopStopAccepted { .. }));
		Ok(Self {
			payload: TrainingReportPayload::Dense(CompletedTrainingCheckpoint::new(execution, manifest)?),
			validation_status,
			gracefully_stopped,
		})
	}

	const fn knn(model: KnnModelArtifact) -> Self {
		Self {
			payload: TrainingReportPayload::Knn(model),
			validation_status: ValidationMetricStatus::NotRequested,
			gracefully_stopped: false,
		}
	}

	const fn bayes(model: BayesModelArtifact) -> Self {
		Self {
			payload: TrainingReportPayload::Bayes(model),
			validation_status: ValidationMetricStatus::NotRequested,
			gracefully_stopped: false,
		}
	}

	#[must_use]
	pub const fn kind(&self) -> TrainingModelKind {
		match self.payload {
			TrainingReportPayload::Dense(_) => TrainingModelKind::Dense,
			TrainingReportPayload::Knn(_) => TrainingModelKind::Knn,
			TrainingReportPayload::Bayes(_) => TrainingModelKind::Bayes,
		}
	}

	/// Native dense run identity. KNN preparation has no native training run.
	#[must_use]
	pub const fn run(&self) -> Option<RunId> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => Some(checkpoint.run()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => None,
		}
	}

	/// Native dense bundle identity. KNN preparation has no execution bundle.
	#[must_use]
	pub const fn bundle(&self) -> Option<BundleIdentity> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => Some(checkpoint.bundle()),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => None,
		}
	}

	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => checkpoint.external_outputs(),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => &[],
		}
	}

	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => checkpoint.metrics(),
			TrainingReportPayload::Knn(_) | TrainingReportPayload::Bayes(_) => &[],
		}
	}

	/// Whether requested validation reporting had known target rows.
	#[must_use]
	pub const fn validation_status(&self) -> ValidationMetricStatus {
		self.validation_status
	}

	/// Whether a host stop request was accepted after a complete epoch.
	#[must_use]
	pub const fn gracefully_stopped(&self) -> bool {
		self.gracefully_stopped
	}

	/// Prepared KNN semantic model, or `None` for a dense training report.
	#[must_use]
	pub const fn knn_model(&self) -> Option<&KnnModelArtifact> {
		match &self.payload {
			TrainingReportPayload::Dense(_) | TrainingReportPayload::Bayes(_) => None,
			TrainingReportPayload::Knn(model) => Some(model),
		}
	}

	/// Prepared observed categorical Bayesian semantic model.
	#[must_use]
	pub const fn bayes_model(&self) -> Option<&BayesModelArtifact> {
		match &self.payload {
			TrainingReportPayload::Bayes(model) => Some(model),
			TrainingReportPayload::Dense(_) | TrainingReportPayload::Knn(_) => None,
		}
	}

	fn save_model(&self, path: impl AsRef<Path>) -> TrainingResult<()> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => checkpoint.save(path).map_err(TrainingError::from),
			TrainingReportPayload::Knn(model) => model.save(path).map_err(TrainingError::from),
			TrainingReportPayload::Bayes(model) => model.save(path).map_err(TrainingError::from),
		}
	}

	fn save_native_kernel(&self, path: impl AsRef<Path>, format: NativeKernelFormat) -> TrainingResult<()> {
		match &self.payload {
			TrainingReportPayload::Dense(checkpoint) => checkpoint
				.save_native_kernel(path, format)
				.map_err(TrainingError::from),
			TrainingReportPayload::Knn(_) => Err(TrainingError::unsupported(
				"KNN reference preparation has no native training kernel artifact",
			)),
			TrainingReportPayload::Bayes(_) => Err(TrainingError::unsupported(
				"categorical Bayesian observation preparation has no native training kernel artifact",
			)),
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

/// Translate public data, model, and policy declarations into Recipe's static
/// GPU training program.
///
/// This boundary loads and losslessly prepares the user dataset, but performs
/// no native probing, artifact compilation, allocation, or execution.
pub fn compile_training(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTraining> {
	compile_training_graph(policy, data, model).map(|(training, _)| training)
}

/// Prepare one loss-independent, all-declared-output KNN semantic model.
///
/// KNN has no optimizer update or learned dense objective. The training
/// partition becomes the immutable reference set; prediction calculation is
/// compiled when target-free query rows are supplied.
pub fn compile_knn_model(policy: &Train, data: &Data, model: &Model) -> TrainingResult<KnnModelArtifact> {
	policy.validate()?;
	data.validate()?;
	model.validate()?;
	let [
		LayerSpec::Knn {
			neighbors,
			operations,
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
	if policy.optimizer_spec().is_some()
		|| policy.learning_rate().is_some()
		|| policy.learning_rate_schedule().is_some()
		|| policy.warmup_epoch_bound().is_some()
		|| policy.epoch_bound().is_some()
	{
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
			.map_err(|error| TrainingError::runtime("inspect KNN resume model", error.to_string()))?;
		if exists {
			Some(load_knn_model_file(path, KnnModelDecodeLimits::default())?)
		} else {
			None
		}
	} else {
		None
	};
	let neighbors = u64::try_from(*neighbors)
		.map_err(|error| TrainingError::unsupported(format!("KNN neighbor count does not fit u64: {error}")))?;
	let neighbors = NonZeroU64::new(neighbors)
		.ok_or_else(|| TrainingError::unsupported("KNN neighbor count must be nonzero"))?;
	let prepared = prepare_data(data)?;
	let references = prepare_knn_reference_set(&prepared, neighbors)?;
	let normalization = data
		.normalization()
		.map(|normalization| match normalization {
			DataNormalization::ZScore => DenseDataNormalization::ZScore,
			DataNormalization::MinMax => DenseDataNormalization::MinMax,
			DataNormalization::L2Norm => DenseDataNormalization::L2Norm,
		});
	let current = KnnModelArtifact::new(references, normalization, map_dense_operations(operations)?)?;
	match resume {
		Some(saved) => saved.continue_with(current).map_err(Into::into),
		None => Ok(current),
	}
}

/// Prepare one or more observed categorical Bayesian target conditionals.
///
/// The semantic artifact retains exact observations and dictionaries. It does
/// not claim an optimizer loop: native inference performs histogramming and
/// Laplace posterior calculation.
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
	if policy.optimizer_spec().is_some()
		|| policy.learning_rate().is_some()
		|| policy.learning_rate_schedule().is_some()
		|| policy.warmup_epoch_bound().is_some()
		|| policy.epoch_bound().is_some()
	{
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
			.map_err(|error| TrainingError::runtime("inspect Bayesian resume model", error.to_string()))?;
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
		.map(|dependency| BayesianDependency::new(dependency.child(), dependency.parents()))
		.collect::<Vec<_>>();
	let current = BayesModelArtifact::from_conditionals(prepare_categorical_bayesian_reference_sets(
		&prepared,
		&dependencies,
	)?)?;
	match resume {
		Some(saved) => saved.continue_with(current).map_err(Into::into),
		None => Ok(current),
	}
}

fn compile_training_package(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTrainingPackage> {
	let (training, resume_checkpoint) = compile_training_graph(policy, data, model)?;
	let resume_native = load_resume_native_bundle(policy, &training, resume_checkpoint.as_ref())?;
	let checkpoint = CheckpointManifest::from_compiled(&training)?;
	Ok(CompiledTrainingPackage {
		training,
		checkpoint,
		resume_native,
	})
}

fn compile_training_graph(
	policy: &Train,
	data: &Data,
	model: &Model,
) -> TrainingResult<(CompiledTraining, Option<CheckpointArtifact>)> {
	policy.validate()?;
	data.validate()?;
	model.validate()?;
	let loss = require_supported_model(model)?;
	require_supported_policy(policy)?;

	let prepared = prepare_data(data)?;
	let blocks = model
		.layers()
		.iter()
		.map(map_dense_block)
		.collect::<TrainingResult<Vec<_>>>()?;
	let layers = blocks
		.iter()
		.map(|block| match block {
			DenseBlock::Layer(layer) => Some(layer.clone()),
			DenseBlock::Embedding(_)
			| DenseBlock::Attention(_)
			| DenseBlock::Rnn(_)
			| DenseBlock::Gru(_)
			| DenseBlock::Lstm(_)
			| DenseBlock::Convolution(_)
			| DenseBlock::Pool(_)
			| DenseBlock::KMeans(_)
			| DenseBlock::Tree(_)
			| DenseBlock::Residual(_) => None,
		})
		.collect::<Option<Vec<_>>>()
		.unwrap_or_default();
	let epochs = policy
		.epoch_bound()
		.map(|epochs| {
			u64::try_from(epochs)
				.map_err(|error| TrainingError::unsupported(format!("epoch bound does not fit u64: {error}")))
				.and_then(|epochs| {
					NonZeroU64::new(epochs)
						.map(TrainingHorizon::Finite)
						.ok_or_else(|| TrainingError::unsupported("epoch bound must be nonzero"))
				})
		})
		.transpose()?
		.unwrap_or(TrainingHorizon::Unbounded);
	let warmup_epochs = u64::try_from(policy.warmup_epoch_bound().unwrap_or(0))
		.map_err(|error| TrainingError::unsupported(format!("warmup epoch bound does not fit u64: {error}")))?;
	let learning_rate = policy
		.learning_rate()
		.unwrap_or_else(|| AdamWConfig::default().learning_rate);
	let learning_rate_decay = match (epochs, policy.learning_rate_schedule()) {
		(TrainingHorizon::Unbounded, Some(LearningRateSchedule::LinearDecay)) => LearningRateDecay::Constant,
		(TrainingHorizon::Unbounded, Some(LearningRateSchedule::CosineDecay)) => {
			return Err(TrainingError::unsupported(
				"cosine learning-rate decay requires `.epochs(...)` to define its endpoint",
			));
		}
		(TrainingHorizon::Unbounded, Some(LearningRateSchedule::ExponentialDecay)) => {
			return Err(TrainingError::unsupported(
				"exponential learning-rate decay requires `.epochs(...)` to define its endpoint",
			));
		}
		(TrainingHorizon::Finite(_), Some(LearningRateSchedule::LinearDecay)) => LearningRateDecay::Linear,
		(TrainingHorizon::Finite(_), Some(LearningRateSchedule::CosineDecay)) => LearningRateDecay::Cosine,
		(TrainingHorizon::Finite(_), Some(LearningRateSchedule::ExponentialDecay)) => {
			LearningRateDecay::Exponential
		}
		(_, None) => {
			return Err(TrainingError::unsupported(
				"an explicit learning-rate decay is required",
			));
		}
	};
	let has_embedding = matches!(blocks.first(), Some(DenseBlock::Embedding(_)));
	let data_normalization = match (has_embedding, data.normalization()) {
		(true, None) => DenseDataNormalization::Identity,
		(true, Some(_)) => {
			return Err(TrainingError::unsupported(
				"embedding consumes exact int32 token IDs; do not declare numeric data normalization",
			));
		}
		(false, Some(DataNormalization::ZScore)) => DenseDataNormalization::ZScore,
		(false, Some(DataNormalization::MinMax)) => DenseDataNormalization::MinMax,
		(false, Some(DataNormalization::L2Norm)) => DenseDataNormalization::L2Norm,
		(false, None) => {
			return Err(TrainingError::unsupported(
				"dense training requires explicit data normalization",
			));
		}
	};
	let config = DenseTrainingConfig {
		layers,
		loss,
		data_normalization,
		epochs,
		warmup_epochs,
		learning_rate_decay,
		gradient_clip_norm: model.gradient_clip_value(),
		normalization_epsilon: 1.0e-6,
		// This is the grammar's semantic ceiling. Primitive lowering selects the
		// actual tree width from realized reduction shape and measured hardware.
		reduction_tree_lanes: MAXIMUM_REDUCTION_TREE_LANES,
		random_seed: 0x7265_6369_7065,
		adamw: AdamWConfig {
			learning_rate,
			..AdamWConfig::default()
		},
	};
	let binary_validation = binary_validation_config(policy, loss)?;
	let multiclass_validation = multiclass_validation_config(policy, loss)?;
	let regression_validation = regression_validation_config(policy, loss)?;
	let contains_structured_blocks = blocks
		.iter()
		.any(|block| !matches!(block, DenseBlock::Layer(_)));
	let mut training = match (
		contains_structured_blocks,
		binary_validation,
		multiclass_validation,
		regression_validation,
	) {
		(false, Some(validation), None, None) => {
			compile_dense_training_with_binary_validation(&prepared, &config, &validation)
				.map_err(TrainingError::from)
		}
		(false, None, Some(validation), None) => {
			compile_dense_training_with_multiclass_validation(&prepared, &config, &validation)
				.map_err(TrainingError::from)
		}
		(false, None, None, Some(validation)) => {
			compile_dense_training_with_regression_validation(&prepared, &config, &validation)
				.map_err(TrainingError::from)
		}
		(false, None, None, None) => compile_dense_training(&prepared, &config).map_err(TrainingError::from),
		(true, Some(validation), None, None) => {
			compile_dense_training_with_blocks_and_binary_validation(&prepared, &config, &blocks, &validation)
				.map_err(TrainingError::from)
		}
		(true, None, Some(validation), None) => {
			compile_dense_training_with_blocks_and_multiclass_validation(&prepared, &config, &blocks, &validation)
				.map_err(TrainingError::from)
		}
		(true, None, None, Some(validation)) => {
			compile_dense_training_with_blocks_and_regression_validation(&prepared, &config, &blocks, &validation)
				.map_err(TrainingError::from)
		}
		(true, None, None, None) => {
			compile_dense_training_with_blocks(&prepared, &config, &blocks).map_err(TrainingError::from)
		}
		_ => Err(TrainingError::unsupported(
			"one training run cannot request multiple validation metric families simultaneously",
		)),
	}?;
	let mut resume_checkpoint = None;
	if let Some(source) = policy.resume_source() {
		let path = Path::new(source);
		let exists = path
			.try_exists()
			.map_err(|error| TrainingError::runtime("inspect training resume model", error.to_string()))?;
		if exists {
			let checkpoint = load_checkpoint_file(path, CheckpointDecodeLimits::default())?;
			apply_checkpoint_resume(&mut training, &checkpoint)?;
			resume_checkpoint = Some(checkpoint);
		}
	}
	Ok((training, resume_checkpoint))
}

fn load_resume_native_bundle(
	policy: &Train,
	training: &CompiledTraining,
	checkpoint: Option<&CheckpointArtifact>,
) -> TrainingResult<Option<ResumeNativeBundle>> {
	let Some(source) = policy.resume_kernel_source() else {
		return Ok(None);
	};
	let Some(checkpoint) = checkpoint else {
		return Ok(None);
	};
	let path = Path::new(source);
	let exists = path
		.try_exists()
		.map_err(|error| TrainingError::runtime("inspect training resume kernel", error.to_string()))?;
	if !exists {
		return Ok(None);
	}
	let format = match path.extension().and_then(|extension| extension.to_str()) {
		Some("cubin") => NativeKernelFormat::Cubin,
		Some("hsaco") => NativeKernelFormat::Hsaco,
		_ => unreachable!("validated native resume source has a known extension"),
	};
	let native = checkpoint.native_realization().ok_or_else(|| {
		CheckpointError::IncompatibleResume {
			detail: "the semantic model predates native realization metadata, so the supplied kernel cannot be authenticated"
				.to_owned(),
		}
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
		.filter(|kernel| kernel.format() == format)
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
	let source_limit = u64::try_from(CheckpointDecodeLimits::default().source_bytes)
		.map_err(|error| TrainingError::runtime("bound training resume kernel", error.to_string()))?;
	let snapshot = read_source_snapshot(
		path,
		SourceLimit::new(source_limit).map_err(TrainingError::NativeKernelSource)?,
	)
	.map_err(TrainingError::NativeKernelSource)?;
	if snapshot.digest() != kernel.digest() {
		return Err(CheckpointError::IncompatibleResume {
			detail:
				"the supplied native kernel bytes do not match the digest authenticated by the semantic model"
					.to_owned(),
		}
		.into());
	}
	Ok(Some(ResumeNativeBundle {
		topology: native.topology(),
		discovery: native.discovery(),
		target: kernel.target().clone(),
		toolchain: kernel.toolchain().clone(),
		bytes: Arc::from(snapshot.into_bytes()),
	}))
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
	pub fn run(&self) -> TrainingResult<TrainingReport> {
		self.try_run()
	}

	fn try_run(&self) -> TrainingResult<TrainingReport> {
		let (data, model) = crate::take_recipe_sequence().map_err(TrainingError::unsupported)?;
		self.try_run_with(&data, &model)
	}

	fn try_run_with(&self, data: &Data, model: &Model) -> TrainingResult<TrainingReport> {
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
			.any(|layer| matches!(layer, LayerSpec::Knn { .. }))
		{
			let report = TrainingReport::knn(compile_knn_model(self, data, model)?);
			if let Some(destination) = self.save_model_destination() {
				report.save_model(destination)?;
			}
			return Ok(report);
		}
		let package = compile_training_package(self, data, model)?;
		let interrupt = crate::signal::SigintGuard::install()
			.map_err(|error| TrainingError::runtime("install graceful SIGINT handler", error.to_string()))?;
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
				.and_then(|extension| extension.to_str())
			{
				Some("cubin") => NativeKernelFormat::Cubin,
				Some("hsaco") => NativeKernelFormat::Hsaco,
				_ => unreachable!("validated native kernel destination has a known extension"),
			};
			report.save_native_kernel(destination, format)?;
		}
		Ok(report)
	}
}

fn execute_current_training(
	policy: &Train,
	training: &CompiledTraining,
	resume_native: Option<&ResumeNativeBundle>,
	stop_requested: &std::sync::atomic::AtomicBool,
) -> TrainingResult<CompletedTrainingExecution> {
	report_unavailable_validation(training)?;
	let presentations = live_metric_presentations(policy, training);
	if presentations.is_empty() {
		let execution = execute_current_training_native(training, resume_native, None, stop_requested)?;
		report_static_training_metrics(policy, &execution)?;
		return Ok(execution);
	}
	let capacity =
		NonZeroUsize::new(LIVE_METRIC_CHANNEL_CAPACITY).expect("the fixed live metric channel capacity is nonzero");
	let (mut observer, receiver) = bounded_training_metric_channel(
		capacity,
		presentations.metrics.keys().copied(),
		NonZeroU64::MIN,
	);
	let presenter = spawn_live_metric_presenter(receiver, presentations, training.bounds())?;
	let execution = execute_current_training_native(training, resume_native, Some(&mut observer), stop_requested);
	drop(observer);
	let presentation = presenter
		.join()
		.map_err(|panic| TrainingError::runtime("join live metric presenter", thread_panic_detail(panic)))?;
	match (execution, presentation) {
		(Ok(execution), Ok(())) => {
			report_static_training_metrics(policy, &execution)?;
			Ok(execution)
		}
		(Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
		(Err(execution), Err(presentation)) => Err(TrainingError::runtime(
			"execute native training and present live metrics",
			format!("execution error: {execution}; presentation error: {presentation}"),
		)),
	}
}

fn report_unavailable_validation(training: &CompiledTraining) -> TrainingResult<()> {
	let Some(line) = crate::validation_reporting::unavailable_validation_line(training.outputs().validation_status)
	else {
		return Ok(());
	};
	let stdout = io::stdout();
	write_live_metric_row(&mut stdout.lock(), &line)
		.map_err(|error| TrainingError::runtime("report validation availability", error.to_string()))
}

fn report_static_training_metrics(policy: &Train, execution: &CompletedTrainingExecution) -> TrainingResult<()> {
	let log_device = policy
		.log_items()
		.iter()
		.any(|item| item.metric() == Metric::Device);
	let plot_device = policy
		.plot_items()
		.iter()
		.any(|item| item.metric() == Metric::Device);
	if !log_device && !plot_device {
		return Ok(());
	}
	let devices = execution
		.native_kernels()
		.kernels()
		.iter()
		.map(|kernel| {
			let target = kernel.target();
			format!("{}:{}:{}", target.backend, target.architecture, target.abi)
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
	let stdout = io::stdout();
	let mut output = stdout.lock();
	if log_device {
		write_live_metric_row(&mut output, &format!("device {devices}"))
			.map_err(|error| TrainingError::runtime("write device metric", error.to_string()))?;
	}
	if plot_device {
		write_live_metric_row(&mut output, &format!("plot device {devices}"))
			.map_err(|error| TrainingError::runtime("plot device metric", error.to_string()))?;
	}
	Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeRuntimeTuning {
	pub worker_threads: usize,
	pub staging_bytes_per_worker: usize,
	pub watchdog: Watchdog,
}

fn select_host_worker_threads(
	available_parallelism: usize,
	measured_host_lanes: usize,
	realized_opportunities: usize,
) -> usize {
	available_parallelism
		.min(measured_host_lanes)
		.min(realized_opportunities)
		.max(1)
}

fn select_host_staging_bytes(
	maximum_value_bytes: u64,
	measured_staging_scale: u64,
	local_ram_bytes: u64,
	worker_threads: u64,
) -> u64 {
	maximum_value_bytes
		.min(measured_staging_scale)
		.min(local_ram_bytes / worker_threads.max(1))
		.max(1)
}

pub(crate) fn derive_native_runtime_tuning(
	graph: &CalculationGraph,
	profile: &MeasuredProfile,
	machine: MachineId,
) -> TrainingResult<NativeRuntimeTuning> {
	let local_devices = profile
		.discovery
		.devices
		.iter()
		.filter(|discovered| {
			profile
				.topology
				.device(discovered.device)
				.is_some_and(|device| device.machine == machine)
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
			profile
				.topology
				.device(discovered.device)
				.is_some_and(|device| matches!(device.kind, DeviceKind::Ram | DeviceKind::Disk))
		})
		.try_fold(0_u64, |total, discovered| {
			let lanes = u64::from(
				discovered
					.transfer
					.maximum_inflight_transfers
					.value
					.get()
					.min(discovered.maximum_submission_queues),
			);
			total.checked_add(lanes)
		})
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"host lane count overflowed",
			)
		})?;
	let measured_host_lanes = usize::try_from(measured_host_lanes)
		.map_err(|error| TrainingError::runtime("derive native training tuning", error.to_string()))?
		.max(1);
	let available_parallelism = std::thread::available_parallelism()
		.map(NonZeroUsize::get)
		.map_err(|error| TrainingError::runtime("query host parallelism", error.to_string()))?;
	let worker_threads = select_host_worker_threads(
		available_parallelism,
		measured_host_lanes,
		realized_opportunities,
	);

	let maximum_value_bytes = graph
		.tensors
		.iter()
		.map(|tensor| tensor.storage_bytes)
		.max_by_key(|bytes| bytes.get())
		.unwrap_or(ByteCount::new(1));
	let external_input_bytes = graph
		.tensors
		.iter()
		.filter(|tensor| tensor.external_input)
		.try_fold(0_u64, |total, tensor| {
			total.checked_add(tensor.storage_bytes.get())
		})
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"external input image size overflowed",
			)
		})?;
	let external_output_bytes = graph
		.tensors
		.iter()
		.filter(|tensor| tensor.external_output)
		.try_fold(0_u64, |total, tensor| {
			total.checked_add(tensor.storage_bytes.get())
		})
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"external output image size overflowed",
			)
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
		.any(|device| device.machine == machine && device.kind == DeviceKind::Disk);
	let measured_staging_scale = match has_local_disk {
		true => profile.benchmarks.storage.buffer_bytes,
		false => profile.benchmarks.ram.buffer_bytes,
	};
	let local_ram_bytes = local_devices
		.iter()
		.filter(|discovered| {
			profile
				.topology
				.device(discovered.device)
				.is_some_and(|device| device.kind == DeviceKind::Ram)
		})
		.try_fold(0_u64, |total, discovered| {
			total.checked_add(discovered.total_capacity.value.get())
		})
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"measured RAM capacity overflowed",
			)
		})?;
	if local_ram_bytes == 0 {
		return Err(TrainingError::runtime(
			"derive native training tuning",
			"measured local machine has no RAM capacity for host staging",
		));
	}
	let worker_count = u64::try_from(worker_threads)
		.map_err(|error| TrainingError::runtime("derive native training tuning", error.to_string()))?;
	let staging_bytes = select_host_staging_bytes(
		maximum_value_bytes.get(),
		measured_staging_scale.get(),
		local_ram_bytes,
		worker_count,
	);
	let staging_bytes_per_worker = usize::try_from(staging_bytes)
		.map_err(|error| TrainingError::runtime("derive native training tuning", error.to_string()))?;

	let tensor_index = graph
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	let maximum_work = graph
		.nodes
		.iter()
		.map(|node| node.kernel.work(&tensor_index))
		.collect::<Result<Vec<_>, _>>()
		.map_err(|error| TrainingError::runtime("derive native training tuning", error.to_string()))?
		.into_iter()
		.max_by_key(|work| work.get())
		.unwrap_or(FlopCount::ZERO);
	let slowest_calculation_rate = local_devices
		.iter()
		.filter_map(|discovered| {
			discovered
				.calculation
				.as_ref()
				.map(|calculation| calculation.rate.value)
		})
		.min_by_key(|rate| rate.get())
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"measured local machine has no calculation rate",
			)
		})?;
	let calculation_duration = calculation_time_ceil(maximum_work, slowest_calculation_rate)
		.map_err(|error| TrainingError::runtime("derive native training tuning", error.to_string()))?;
	let slowest_device_transfer_rate = local_devices
		.iter()
		.map(|discovered| discovered.transfer.rate.value)
		.min_by_key(|rate| rate.get());
	let slowest_local_link_rate = profile
		.discovery
		.links
		.iter()
		.filter_map(|discovered| {
			let link = profile.topology.link(discovered.link)?;
			let source = profile.topology.device(link.from)?;
			let destination = profile.topology.device(link.to)?;
			(source.machine == machine && destination.machine == machine).then_some(discovered.bandwidth.value)
		})
		.min_by_key(|rate| rate.get());
	let slowest_transfer_rate = slowest_device_transfer_rate
		.into_iter()
		.chain(slowest_local_link_rate)
		.min_by_key(|rate| rate.get())
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"measured local machine has no transfer rate",
			)
		})?;
	let transfer_duration = transfer_time_ceil(maximum_transfer_bytes, slowest_transfer_rate)
		.map_err(|error| TrainingError::runtime("derive native training tuning", error.to_string()))?;
	let expected_operation = Duration::from_nanos(
		calculation_duration
			.get()
			.max(transfer_duration.get())
			.max(1),
	);
	let measured_concurrency = local_devices
		.iter()
		.try_fold(0_u64, |total, discovered| {
			let calculation = discovered
				.calculation
				.as_ref()
				.map_or(0_u64, |calculation| {
					u64::from(calculation.maximum_concurrent_tasks)
				});
			let transfer = u64::from(discovered.transfer.maximum_inflight_transfers.value.get());
			total.checked_add(calculation)?.checked_add(transfer)
		})
		.ok_or_else(|| {
			TrainingError::runtime(
				"derive native training tuning",
				"device concurrency overflowed",
			)
		})?
		.max(1)
		.min(u64::from(u32::MAX));
	let safety_multiplier = NonZeroU32::new(measured_concurrency as u32)
		.expect("measured concurrency was clamped to the nonzero u32 domain");
	let watchdog = Watchdog::for_expected_duration(expected_operation, safety_multiplier);

	Ok(NativeRuntimeTuning {
		worker_threads,
		staging_bytes_per_worker,
		watchdog,
	})
}

fn execute_current_training_native(
	training: &CompiledTraining,
	resume_native: Option<&ResumeNativeBundle>,
	mut observer: Option<&mut TrainingMetricObserver>,
	stop_requested: &std::sync::atomic::AtomicBool,
) -> TrainingResult<CompletedTrainingExecution> {
	let run = next_run_id();
	let result = with_current_native_preparation(|profile, _config, scope| {
		if let Some(native) = resume_native {
			if profile.topology.identity != native.topology || profile.discovery.identity != native.discovery {
				return Err(NativePreparationError::IdentityMismatch {
					detail: "supplied native kernel was realized for a different measured topology or discovery profile"
						.to_owned(),
				});
			}
			let matches_current_target = scope.targets().target_build_specs().iter().any(|spec| {
				spec.target_identity == native.target && spec.toolchain_identity == native.toolchain
			});
			if !matches_current_target {
				return Err(NativePreparationError::IdentityMismatch {
					detail: "supplied native kernel target or toolchain differs from the current measured target plan"
						.to_owned(),
				});
			}
		}
		let (bindings, host, targets) = scope.into_parts();
		let tuning = derive_native_runtime_tuning(training.graph(), profile, host.machine())
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let limits = TrainingExecutionLimits::new(tuning.watchdog);
		let host = host
			.backend_config(run, tuning.worker_threads, tuning.staging_bytes_per_worker)
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
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
		let realizer = NativeCandidateRealizer::new(profile, compiler, driver)
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let provider = NativeArtifactProvider::new(NativeArtifactCatalog::default());
		let mut preparer = Preparer::new(provider, realizer);
		let execution = prepare_and_execute_local_training_controlled(
			training,
			profile,
			&mut preparer,
			run,
			limits,
			observer.as_deref_mut(),
			TrainingExecutionControl::graceful_stop(stop_requested),
		);
		Ok(execution)
	})?;
	result.map_err(|error| TrainingError::runtime("execute native training", error.to_string()))
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
	fn requested(&self) -> bool {
		self.epoch_log_cadence.is_some() || self.epoch_plot || self.time_log_cadence.is_some() || self.time_plot
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LiveMetricPlan {
	metrics: BTreeMap<MetricId, LiveMetricPresentation>,
	host: LiveHostPresentation,
}

impl LiveMetricPlan {
	fn is_empty(&self) -> bool {
		self.metrics.is_empty()
	}
}

fn live_metric_presentations(policy: &Train, training: &CompiledTraining) -> LiveMetricPlan {
	let mut plan = LiveMetricPlan {
		host: LiveHostPresentation {
			epoch_log_cadence: policy.metric_log_interval(Metric::Epoch),
			epoch_plot: policy
				.plot_items()
				.iter()
				.any(|item| item.metric() == Metric::Epoch),
			time_log_cadence: policy.metric_log_interval(Metric::Time),
			time_plot: policy
				.plot_items()
				.iter()
				.any(|item| item.metric() == Metric::Time),
		},
		..LiveMetricPlan::default()
	};
	for binding in &training.outputs().metric_bindings {
		let Some(metric) = log_selects_binding_to_metric(binding.kind) else {
			continue;
		};
		let logged = policy
			.log_items()
			.iter()
			.any(|item| log_selects(item.metric(), binding.kind));
		let plotted = policy
			.plot_items()
			.iter()
			.any(|item| log_selects(item.metric(), binding.kind));
		if !logged && !plotted {
			continue;
		}
		plan.metrics.insert(
			binding.metric,
			LiveMetricPresentation {
				label: metric_label(binding.kind),
				width: metric_width(binding.kind),
				log_cadence: logged.then(|| policy.metric_log_interval(metric)).flatten(),
				plot: plotted,
			},
		);
	}
	if plan.host.requested()
		&& let Some(trigger) = training
			.outputs()
			.metric_bindings
			.iter()
			.find(|binding| binding.kind == TrainingMetricKind::TrainingLoss)
	{
		plan.metrics
			.entry(trigger.metric)
			.or_insert_with(|| LiveMetricPresentation {
				label: metric_label(trigger.kind),
				width: metric_width(trigger.kind),
				log_cadence: None,
				plot: false,
			});
	}
	plan
}

fn log_selects_binding_to_metric(binding: TrainingMetricKind) -> Option<Metric> {
	match binding {
		TrainingMetricKind::TrainingLoss
		| TrainingMetricKind::ValidationMeanBce
		| TrainingMetricKind::ValidationMeanCrossEntropy => Some(Metric::Loss),
		TrainingMetricKind::Accuracy => Some(Metric::Accuracy),
		TrainingMetricKind::R2 => Some(Metric::R2),
		TrainingMetricKind::LearningRate => Some(Metric::LearningRate),
		TrainingMetricKind::AuRoc => Some(Metric::AuRoc),
		TrainingMetricKind::AuPrc => Some(Metric::AuPrc),
		TrainingMetricKind::BrierScore => Some(Metric::Brier),
		TrainingMetricKind::ExpectedCalibrationError => Some(Metric::CalibrationError),
		TrainingMetricKind::RecallAt { .. } => Some(Metric::Loss),
	}
}

fn log_selects(requested: Metric, available: TrainingMetricKind) -> bool {
	match requested {
		Metric::Loss => matches!(
			available,
			TrainingMetricKind::TrainingLoss
				| TrainingMetricKind::ValidationMeanBce
				| TrainingMetricKind::ValidationMeanCrossEntropy
		),
		Metric::Accuracy => available == TrainingMetricKind::Accuracy,
		Metric::R2 => available == TrainingMetricKind::R2,
		Metric::AuRoc => available == TrainingMetricKind::AuRoc,
		Metric::AuPrc => available == TrainingMetricKind::AuPrc,
		Metric::Brier => available == TrainingMetricKind::BrierScore,
		Metric::CalibrationError => available == TrainingMetricKind::ExpectedCalibrationError,
		Metric::LearningRate => available == TrainingMetricKind::LearningRate,
		Metric::Epoch | Metric::Time | Metric::Device => false,
	}
}

fn metric_label(metric: TrainingMetricKind) -> String {
	match metric {
		TrainingMetricKind::TrainingLoss => "loss".to_owned(),
		TrainingMetricKind::ValidationMeanBce => "validation_loss".to_owned(),
		TrainingMetricKind::ValidationMeanCrossEntropy => "validation_loss".to_owned(),
		TrainingMetricKind::Accuracy => "accuracy".to_owned(),
		TrainingMetricKind::R2 => "r2".to_owned(),
		TrainingMetricKind::LearningRate => "lr".to_owned(),
		TrainingMetricKind::AuRoc => "auroc".to_owned(),
		TrainingMetricKind::AuPrc => "auprc".to_owned(),
		TrainingMetricKind::BrierScore => "brier".to_owned(),
		TrainingMetricKind::ExpectedCalibrationError => "calibration_error".to_owned(),
		TrainingMetricKind::RecallAt { threshold_bits } => {
			format!("recall@{}", f32::from_bits(threshold_bits))
		}
	}
}

const fn metric_width(metric: TrainingMetricKind) -> usize {
	match metric {
		TrainingMetricKind::TrainingLoss
		| TrainingMetricKind::ValidationMeanBce
		| TrainingMetricKind::ValidationMeanCrossEntropy => 7,
		TrainingMetricKind::Accuracy | TrainingMetricKind::R2 | TrainingMetricKind::LearningRate => 6,
		TrainingMetricKind::AuRoc
		| TrainingMetricKind::AuPrc
		| TrainingMetricKind::BrierScore
		| TrainingMetricKind::ExpectedCalibrationError
		| TrainingMetricKind::RecallAt { .. } => 6,
	}
}

fn spawn_live_metric_presenter(
	receiver: Receiver<TrainingMetricSample>,
	presentations: LiveMetricPlan,
	bounds: TrainingBounds,
) -> TrainingResult<JoinHandle<TrainingResult<()>>> {
	std::thread::Builder::new()
		.name("recipe-live-metrics".to_owned())
		.spawn(move || {
			let stdout = io::stdout();
			let mut output = stdout.lock();
			present_live_metrics(receiver, presentations, bounds, &mut output)
		})
		.map_err(|error| TrainingError::runtime("start live metric presenter", error.to_string()))
}

fn present_live_metrics(
	receiver: Receiver<TrainingMetricSample>,
	presentations: LiveMetricPlan,
	bounds: TrainingBounds,
	output: &mut impl Write,
) -> TrainingResult<()> {
	let mut rows = LiveMetricRows::new(presentations, bounds.epochs);
	while let Ok(sample) = receiver.recv() {
		if let Some(row) = rows.push(
			sample.zero_based_iteration.index(),
			sample.epoch,
			sample.metric,
			sample.value,
		) {
			write_live_metric_row(output, &row)
				.map_err(|error| TrainingError::runtime("write live metrics", error.to_string()))?;
		}
	}
	if let Some(row) = rows.finish() {
		write_live_metric_row(output, &row)
			.map_err(|error| TrainingError::runtime("write live metrics", error.to_string()))?;
	}
	for plot in rows.plot_lines() {
		write_live_metric_row(output, &plot)
			.map_err(|error| TrainingError::runtime("write live metric plot", error.to_string()))?;
	}
	Ok(())
}

fn thread_panic_detail(panic: Box<dyn std::any::Any + Send>) -> String {
	if let Some(message) = panic.downcast_ref::<&str>() {
		(*message).to_owned()
	} else if let Some(message) = panic.downcast_ref::<String>() {
		message.clone()
	} else {
		"non-string panic payload".to_owned()
	}
}

fn write_live_metric_row(output: &mut impl Write, row: &str) -> io::Result<()> {
	writeln!(output, "{row}")?;
	output.flush()
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
		Self {
			plan,
			epochs: epochs.into(),
			started: Instant::now(),
			pending_sample: None,
			pending_values: BTreeMap::new(),
			plots: BTreeMap::new(),
		}
	}

	fn push(&mut self, iteration: u64, epoch: NonZeroU64, metric: MetricId, value: MetricValue) -> Option<String> {
		let completed = if self
			.pending_sample
			.is_some_and(|(pending, _)| pending != iteration)
		{
			let completed = self
				.pending_sample
				.and_then(|(_, epoch)| self.render(epoch, false));
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
		completed
	}

	fn finish(&mut self) -> Option<String> {
		let (_, epoch) = self.pending_sample.take()?;
		let row = self.render(epoch, true);
		self.pending_values.clear();
		row
	}

	fn render(&mut self, epoch: NonZeroU64, final_flush: bool) -> Option<String> {
		let final_epoch = final_flush || self.epochs.bound().is_some_and(|bound| epoch == bound);
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
				presentation
					.plot
					.then(|| {
						self.pending_values
							.get(metric)
							.copied()
							.map(|value| (presentation.label.clone(), value))
					})
					.flatten()
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
			.is_some_and(|cadence| epoch.get() % cadence.get() == 0 || final_epoch);
		let time_due = self
			.plan
			.host
			.time_log_cadence
			.is_some_and(|cadence| epoch.get() % cadence.get() == 0 || final_epoch);
		if time_due {
			fields.push(live_metric_field("time", 9, &format!("{elapsed:.3}s"), 1));
		}
		for (metric, presentation) in &self.plan.metrics {
			let Some(value) = self.pending_values.get(metric) else {
				continue;
			};
			let log_due = presentation
				.log_cadence
				.is_some_and(|cadence| epoch.get() % cadence.get() == 0 || final_epoch);
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
		Some(fields.join("  "))
	}

	fn record_plot(&mut self, label: &str, value: f64) {
		self.plots.entry(label.to_owned()).or_default().push(value);
	}

	fn plot_lines(&self) -> Vec<String> {
		self.plots
			.iter()
			.map(|(label, series)| series.render(label))
			.collect()
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
			.filter(|value| value.is_finite())
			.collect::<Vec<_>>();
		let (minimum, maximum) = finite
			.iter()
			.copied()
			.fold(None, |range, value| match range {
				Some((minimum, maximum)) => Some((f64::min(minimum, value), f64::max(maximum, value))),
				None => Some((value, value)),
			})
			.unwrap_or((f64::NAN, f64::NAN));
		let levels = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
		let plot = self
			.values
			.iter()
			.map(|value| {
				if !value.is_finite() {
					'·'
				} else if minimum == maximum {
					levels[3]
				} else {
					let scaled = ((*value - minimum) / (maximum - minimum)) * (levels.len() - 1) as f64;
					levels[scaled.round() as usize]
				}
			})
			.collect::<String>();
		if minimum.is_finite() {
			format!(
				"plot {label} {plot}  range {minimum:.4}..{maximum:.4}  samples {}",
				self.total
			)
		} else {
			format!("plot {label} {plot}  range N/A  samples {}", self.total)
		}
	}
}

fn metric_value_f64(value: MetricValue) -> f64 {
	match value {
		MetricValue::F32(value) => f64::from(value),
		MetricValue::I32(value) => f64::from(value),
	}
}

fn render_live_metric_value(value: MetricValue, width: usize) -> String {
	match value {
		MetricValue::F32(value) if value.is_nan() => format!("{:>width$}", "N/A"),
		MetricValue::F32(value) => format!("{value:>width$.4}"),
		MetricValue::I32(value) => format!("{value:>width$}"),
	}
}

fn live_metric_field(label: &str, width: usize, value: &str, color: usize) -> String {
	let (red, green, blue) = LIVE_METRIC_PALETTE[color % LIVE_METRIC_PALETTE.len()];
	format!("\x1b[38;2;{red};{green};{blue}m{label}\x1b[0m {value:>width$}")
}

pub(crate) fn next_run_id() -> RunId {
	let sequence = NEXT_RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
	let epoch = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map_or(0, |duration| duration.as_nanos() as u64);
	let process = u64::from(std::process::id());
	let value = epoch ^ process.rotate_left(19) ^ sequence.rotate_left(41);
	RunId::new(value.max(1))
}

fn require_supported_model(model: &Model) -> TrainingResult<DenseLoss> {
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
		Some(Objective::Builtin(Loss::BinaryCrossEntropy)) => Ok(DenseLoss::BinaryCrossEntropy),
		Some(Objective::Builtin(Loss::MeanSquaredError)) => Ok(DenseLoss::MeanSquaredError),
		Some(Objective::Builtin(Loss::MeanAbsoluteError)) => Ok(DenseLoss::MeanAbsoluteError),
		Some(Objective::Builtin(Loss::CrossEntropy)) => Ok(DenseLoss::CrossEntropy),
		Some(Objective::Builtin(Loss::Huber)) => Ok(DenseLoss::Huber),
		Some(Objective::Builtin(Loss::Focal)) => Ok(DenseLoss::Focal),
		Some(Objective::Reference(_)) => Err(TrainingError::unsupported(
			"model-referenced objectives have no dense training lowering",
		)),
		None => Err(TrainingError::unsupported(
			"dense training requires an explicit built-in objective",
		)),
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
	Ok(())
}

fn binary_validation_config(policy: &Train, loss: DenseLoss) -> TrainingResult<Option<BinaryValidationConfig>> {
	let mut requested = false;
	for item in policy.log_items().iter().chain(policy.plot_items()) {
		match item.metric() {
			Metric::Loss => {}
			Metric::AuRoc | Metric::AuPrc | Metric::Brier | Metric::CalibrationError => {
				requested = true;
			}
			Metric::Epoch | Metric::LearningRate | Metric::Time | Metric::Device => {}
			Metric::Accuracy if matches!(loss, DenseLoss::BinaryCrossEntropy | DenseLoss::Focal) => {
				requested = true;
			}
			Metric::Accuracy if loss == DenseLoss::CrossEntropy => {}
			Metric::R2
				if matches!(
					loss,
					DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber
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
	if !matches!(loss, DenseLoss::BinaryCrossEntropy | DenseLoss::Focal) {
		return Err(TrainingError::unsupported(
			"binary classification validation metrics and calibration require BCE or focal loss",
		));
	}
	let bins = NonZeroU32::new(15).expect("the Recipe ECE default is nonzero");
	Ok(Some(BinaryValidationConfig::new(bins, Vec::new())))
}

fn multiclass_validation_config(policy: &Train, loss: DenseLoss) -> TrainingResult<Option<MulticlassValidationConfig>> {
	let requested = policy
		.log_items()
		.iter()
		.chain(policy.plot_items())
		.any(|item| item.metric() == Metric::Accuracy);
	if !requested {
		return Ok(None);
	}
	match loss {
		DenseLoss::CrossEntropy => Ok(Some(MulticlassValidationConfig)),
		DenseLoss::BinaryCrossEntropy | DenseLoss::Focal => Ok(None),
		_ => Err(TrainingError::unsupported(
			"accuracy validation requires binary or categorical cross entropy",
		)),
	}
}

fn regression_validation_config(policy: &Train, loss: DenseLoss) -> TrainingResult<Option<RegressionValidationConfig>> {
	let requested = policy
		.log_items()
		.iter()
		.chain(policy.plot_items())
		.any(|item| item.metric() == Metric::R2);
	if !requested {
		return Ok(None);
	}
	if !matches!(
		loss,
		DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber
	) {
		return Err(TrainingError::unsupported(
			"R2 validation requires a scalar regression loss",
		));
	}
	Ok(Some(RegressionValidationConfig))
}

fn map_dense_block(layer: &LayerSpec) -> TrainingResult<DenseBlock> {
	match layer {
		LayerSpec::Embedding {
			dimensions,
			vocabulary,
		} => {
			let vocabulary = vocabulary.ok_or_else(|| {
				TrainingError::unsupported("`.embed(dimensions)` requires an immediate `.vocab(vocabulary)`")
			})?;
			Ok(DenseBlock::Embedding(DenseEmbedding::new(
				map_dense_width(*dimensions, "embedding dimension")?,
				map_dense_width(vocabulary, "embedding vocabulary")?,
			)))
		}
		LayerSpec::Attention { heads } => Ok(DenseBlock::Attention(DenseAttention::new(map_dense_width(
			*heads,
			"attention head count",
		)?))),
		LayerSpec::Rnn { width, operations } => {
			if !operations.is_empty() {
				return Err(TrainingError::unsupported(
					"the first vanilla RNN case does not yet define chained recurrent-block operations",
				));
			}
			Ok(DenseBlock::Rnn(DenseRnn::new(map_dense_width(
				*width,
				"RNN hidden width",
			)?)))
		}
		LayerSpec::Convolution {
			filters,
			kernel,
			activation,
		} => Ok(DenseBlock::Convolution(DenseConvolution::new(
			map_dense_width(*filters, "convolution filter count")?,
			map_dense_width(*kernel, "convolution kernel width")?,
			map_dense_activation(*activation)?,
		))),
		LayerSpec::Pool {
			size,
			group_to_neuron,
		} => {
			let size = map_dense_width(*size, "pool size")?;
			let group_to_neuron = group_to_neuron
				.map(|connection| map_dense_width(connection.neurons(), "pool destination neuron count"))
				.transpose()?;
			Ok(DenseBlock::Pool(DensePool::new(size, group_to_neuron)))
		}
		LayerSpec::KMeans {
			clusters,
			group_to_neuron,
		} => {
			let clusters = map_dense_width(*clusters, "K-means cluster count")?;
			let group_to_neuron = group_to_neuron
				.map(|connection| map_dense_width(connection.neurons(), "K-means destination neuron count"))
				.transpose()?;
			Ok(DenseBlock::KMeans(DenseKMeans::new(
				clusters,
				group_to_neuron,
			)))
		}
		LayerSpec::Gru { width, operations } => {
			if !operations.is_empty() {
				return Err(TrainingError::unsupported(
					"the first GRU case does not yet define chained recurrent-block operations",
				));
			}
			Ok(DenseBlock::Gru(DenseGru::new(map_dense_width(
				*width,
				"GRU hidden width",
			)?)))
		}
		LayerSpec::Lstm { width, operations } => {
			if !operations.is_empty() {
				return Err(TrainingError::unsupported(
					"the first LSTM case does not yet define chained recurrent-block operations",
				));
			}
			Ok(DenseBlock::Lstm(DenseLstm::new(map_dense_width(
				*width,
				"LSTM hidden width",
			)?)))
		}
		LayerSpec::Lgbm { depth } => map_tree_block(DenseTreeFamily::LightGbm, 1, *depth),
		LayerSpec::Cbst { depth } => map_tree_block(DenseTreeFamily::CatBoost, 1, *depth),
		LayerSpec::Xgbst { depth } => map_tree_block(DenseTreeFamily::XGBoost, 1, *depth),
		LayerSpec::Forest { trees, booster } => {
			let booster = booster.as_ref().ok_or_else(|| {
				TrainingError::unsupported(
					"forest requires one nested `.lgbm`, `.cbst`, or `.xgbst` declaration",
				)
			})?;
			let (family, depth) = match booster {
				ForestBooster::Lgbm { depth } => (DenseTreeFamily::LightGbm, *depth),
				ForestBooster::Cbst { depth } => (DenseTreeFamily::CatBoost, *depth),
				ForestBooster::Xgbst { depth } => (DenseTreeFamily::XGBoost, *depth),
			};
			map_tree_block(family, *trees, depth)
		}
		LayerSpec::Residual {
			branch,
			output_width,
			skip: ResidualSkip::IdentityOrLinearProjection,
			operations,
		} => {
			let branch = branch
				.iter()
				.copied()
				.map(map_dense_residual_operation)
				.collect::<TrainingResult<Vec<_>>>()?;
			let operations = map_dense_operations(operations)?;
			let residual = DenseResidual::new(branch, operations);
			let declared_output_width = map_dense_width(*output_width, "residual output width")?;
			if residual.output_width() != Some(declared_output_width) {
				return Err(TrainingError::unsupported(format!(
					"residual branch resolves to output width {:?}, but the facade retained {}",
					residual.output_width().map(NonZeroU64::get),
					declared_output_width.get(),
				)));
			}
			Ok(DenseBlock::Residual(residual))
		}
		_ => map_dense_layer(layer).map(DenseBlock::Layer),
	}
}

fn map_tree_block(family: DenseTreeFamily, trees: usize, depth: usize) -> TrainingResult<DenseBlock> {
	let trees = map_dense_width(trees, "tree count")?;
	let depth = u32::try_from(depth)
		.map_err(|error| TrainingError::unsupported(format!("tree depth does not fit u32: {error}")))?;
	let depth = NonZeroU32::new(depth).ok_or_else(|| TrainingError::unsupported("tree depth must be nonzero"))?;
	Ok(DenseBlock::Tree(DenseTree::new(family, trees, depth)))
}

fn map_dense_residual_operation(operation: ResidualOperation) -> TrainingResult<DenseResidualOperation> {
	match operation {
		ResidualOperation::Layer { width } => Ok(DenseResidualOperation::Layer(DenseLayer::with_kind(
			DenseBlockKind::Layer,
			map_dense_width(width, "residual branch layer width")?,
			[],
		))),
		ResidualOperation::Activation(activation) => Ok(DenseResidualOperation::Operation(
			DenseOperation::Activation(map_dense_activation(activation)?),
		)),
	}
}

fn map_dense_layer(layer: &LayerSpec) -> TrainingResult<DenseLayer> {
	let (kind, units, operations) = match layer {
		LayerSpec::Dense { units, operations } => (DenseBlockKind::Layer, units, operations),
		LayerSpec::Perc { count, operations } => (DenseBlockKind::Perc, count, operations),
		LayerSpec::Rnn { .. }
		| LayerSpec::Gru { .. }
		| LayerSpec::Lstm { .. }
		| LayerSpec::Convolution { .. }
		| LayerSpec::Pool { .. }
		| LayerSpec::Lgbm { .. }
		| LayerSpec::Cbst { .. }
		| LayerSpec::Xgbst { .. }
		| LayerSpec::Forest { .. }
		| LayerSpec::KMeans { .. }
		| LayerSpec::Knn { .. }
		| LayerSpec::Embedding { .. }
		| LayerSpec::Attention { .. }
		| LayerSpec::Residual { .. } => {
			return Err(TrainingError::unsupported(
				"the current training compiler accepts layer, perc, and residual blocks only",
			));
		}
	};
	let width = map_dense_width(*units, "dense layer width")?;
	let operations = map_dense_operations(operations)?;
	Ok(DenseLayer::with_kind(kind, width, operations))
}

fn map_dense_width(width: usize, role: &str) -> TrainingResult<NonZeroU64> {
	let width = u64::try_from(width)
		.map_err(|error| TrainingError::unsupported(format!("{role} does not fit u64: {error}")))?;
	NonZeroU64::new(width).ok_or_else(|| TrainingError::unsupported(format!("{role} must be nonzero")))
}

fn map_dense_operations(operations: &[LayerOperation]) -> TrainingResult<Vec<DenseOperation>> {
	let operations = operations
		.iter()
		.copied()
		.map(|operation| match operation {
			LayerOperation::Activation(activation) => {
				map_dense_activation(activation).map(DenseOperation::Activation)
			}
			LayerOperation::Normalization(LayerNormalization::LayerNorm) => {
				Ok(DenseOperation::Normalization(DenseNormalization::Layer))
			}
			LayerOperation::Normalization(LayerNormalization::BatchNorm) => {
				Ok(DenseOperation::Normalization(DenseNormalization::Batch))
			}
		})
		.collect::<TrainingResult<Vec<_>>>()?;
	Ok(operations)
}

fn map_dense_activation(activation: Activation) -> TrainingResult<DenseActivation> {
	match activation {
		Activation::Linear => Ok(DenseActivation::Linear),
		Activation::Cosine => Ok(DenseActivation::Cosine),
		Activation::Exponential => Ok(DenseActivation::Exponential),
		Activation::Logarithm => Ok(DenseActivation::Logarithm),
		Activation::NaturalLogarithm => Ok(DenseActivation::NaturalLogarithm),
		Activation::Huber => Ok(DenseActivation::Huber),
		Activation::Tangent => Ok(DenseActivation::Tangent),
		Activation::Relu => Ok(DenseActivation::Relu),
		Activation::LeakyRelu => Ok(DenseActivation::LeakyRelu),
		Activation::Sigmoid => Ok(DenseActivation::Sigmoid),
		Activation::Tanh => Ok(DenseActivation::Tanh),
		Activation::Selu => Ok(DenseActivation::Selu),
		Activation::Gelu => Ok(DenseActivation::Gelu),
		Activation::Silu => Ok(DenseActivation::Silu),
		Activation::Elu => Ok(DenseActivation::Elu),
		Activation::PRelu => Ok(DenseActivation::PRelu),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct BrokenMetricWriter;

	impl Write for BrokenMetricWriter {
		fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
			Err(io::Error::new(
				io::ErrorKind::BrokenPipe,
				"closed metric output",
			))
		}

		fn flush(&mut self) -> io::Result<()> {
			Ok(())
		}
	}

	fn presentations() -> LiveMetricPlan {
		let metrics = [
			(
				MetricId::new(1),
				LiveMetricPresentation {
					label: "loss".to_owned(),
					width: 7,
					log_cadence: NonZeroU64::new(1),
					plot: false,
				},
			),
			(
				MetricId::new(2),
				LiveMetricPresentation {
					label: "auroc".to_owned(),
					width: 6,
					log_cadence: NonZeroU64::new(1),
					plot: false,
				},
			),
		]
		.into_iter()
		.collect();
		LiveMetricPlan {
			metrics,
			host: LiveHostPresentation::default(),
		}
	}

	#[test]
	fn host_runtime_tuning_tracks_capabilities_and_realized_work() {
		assert_eq!(select_host_worker_threads(32, 12, 100), 12);
		assert_eq!(select_host_worker_threads(4, 12, 100), 4);
		assert_eq!(select_host_worker_threads(32, 12, 3), 3);
		assert_eq!(
			select_host_staging_bytes(64 << 20, 8 << 20, 128 << 20, 4),
			8 << 20
		);
		assert_eq!(
			select_host_staging_bytes(2 << 20, 8 << 20, 128 << 20, 4),
			2 << 20
		);
		assert_eq!(
			select_host_staging_bytes(64 << 20, 64 << 20, 8 << 20, 4),
			2 << 20
		);
	}

	#[test]
	fn live_metric_presenter_propagates_writer_failure() {
		let (sender, receiver) = std::sync::mpsc::channel();
		sender.send(TrainingMetricSample {
			sequence: 1,
			epoch: NonZeroU64::MIN,
			zero_based_iteration: recipe_core::LoopIterations::ONE.iteration(0).unwrap(),
			task: recipe_core::TaskId::new(1),
			slot: recipe_core::MetricSlotId::new(1),
			metric: MetricId::new(1),
			value: MetricValue::F32(0.25),
		})
		.expect("send live metric sample");
		drop(sender);
		let bounds = TrainingBounds {
			train_rows: 1,
			epochs: TrainingHorizon::Finite(NonZeroU64::MIN),
			training_iterations: recipe_core::LoopIterations::ONE,
			calibration_iterations: 0,
			iterations: recipe_core::LoopIterations::ONE,
			warmup_iterations: 0,
		};
		let error = present_live_metrics(receiver, presentations(), bounds, &mut BrokenMetricWriter)
			.expect_err("closed output must fail live metric presentation");
		assert!(matches!(
			error,
			TrainingError::Runtime { stage: "write live metrics", detail } if detail.contains("closed metric output")
		));
	}

	#[test]
	fn loss_logging_uses_training_loss_without_requesting_validation() {
		let policy = Train::new().log([crate::api::Loss]);

		assert_eq!(
			binary_validation_config(&policy, DenseLoss::MeanSquaredError).expect("valid policy"),
			None
		);
		assert_eq!(
			multiclass_validation_config(&policy, DenseLoss::CrossEntropy).expect("valid policy"),
			None
		);
	}

	#[test]
	fn accuracy_logging_selects_binary_or_categorical_validation_from_the_loss() {
		let policy = Train::new().log([crate::api::Accuracy]);

		assert_eq!(
			binary_validation_config(&policy, DenseLoss::BinaryCrossEntropy).expect("valid policy"),
			Some(BinaryValidationConfig::new(
				NonZeroU32::new(15).expect("nonzero bins"),
				Vec::new()
			))
		);
		assert_eq!(
			binary_validation_config(&policy, DenseLoss::Focal).expect("valid policy"),
			Some(BinaryValidationConfig::new(
				NonZeroU32::new(15).expect("nonzero bins"),
				Vec::new()
			))
		);
		assert_eq!(
			binary_validation_config(&policy, DenseLoss::CrossEntropy).expect("valid policy"),
			None
		);
		assert_eq!(
			multiclass_validation_config(&policy, DenseLoss::BinaryCrossEntropy).expect("valid policy"),
			None
		);
		assert_eq!(
			multiclass_validation_config(&policy, DenseLoss::Focal).expect("valid policy"),
			None
		);
		assert_eq!(
			multiclass_validation_config(&policy, DenseLoss::CrossEntropy).expect("valid policy"),
			Some(MulticlassValidationConfig)
		);
		assert!(multiclass_validation_config(&policy, DenseLoss::MeanSquaredError).is_err());
	}

	#[test]
	fn r2_logging_selects_only_scalar_regression_validation() {
		let policy = Train::new().log([crate::api::R2]);

		assert_eq!(
			binary_validation_config(&policy, DenseLoss::MeanSquaredError).expect("valid policy"),
			None
		);
		assert_eq!(
			regression_validation_config(&policy, DenseLoss::MeanSquaredError).expect("valid policy"),
			Some(RegressionValidationConfig)
		);
		assert!(regression_validation_config(&policy, DenseLoss::BinaryCrossEntropy).is_err());
	}

	#[test]
	fn public_losses_map_to_distinct_dense_compiler_losses() {
		for (loss, expected) in [
			(Loss::BinaryCrossEntropy, DenseLoss::BinaryCrossEntropy),
			(Loss::MeanSquaredError, DenseLoss::MeanSquaredError),
			(Loss::MeanAbsoluteError, DenseLoss::MeanAbsoluteError),
			(Loss::CrossEntropy, DenseLoss::CrossEntropy),
			(Loss::Huber, DenseLoss::Huber),
			(Loss::Focal, DenseLoss::Focal),
		] {
			let model = Model::new()
				.layer(1)
				.loss(loss)
				.grad(crate::api::clip(0.75));
			assert_eq!(
				require_supported_model(&model).expect("supported loss"),
				expected
			);
			assert_eq!(model.gradient_clip_value(), Some(0.75));
		}
	}

	#[test]
	fn public_activations_map_to_their_canonical_dense_identity() {
		for (activation, expected) in [
			(Activation::Logarithm, DenseActivation::Logarithm),
			(
				Activation::NaturalLogarithm,
				DenseActivation::NaturalLogarithm,
			),
			(Activation::LeakyRelu, DenseActivation::LeakyRelu),
			(Activation::Sigmoid, DenseActivation::Sigmoid),
			(Activation::Tanh, DenseActivation::Tanh),
			(Activation::Selu, DenseActivation::Selu),
			(Activation::Elu, DenseActivation::Elu),
			(Activation::PRelu, DenseActivation::PRelu),
		] {
			assert_eq!(
				map_dense_activation(activation).expect("activation lowers"),
				expected
			);
		}
	}

	#[test]
	fn perc_lowers_as_a_distinct_dense_block_kind() {
		let model = Model::new().perc(30).silu();
		let block = map_dense_layer(&model.layers()[0]).expect("perc block lowers");

		assert_eq!(block.kind(), DenseBlockKind::Perc);
		assert_eq!(block.width().get(), 30);
		assert_eq!(
			block.operations(),
			[DenseOperation::Activation(DenseActivation::Silu)]
		);
	}

	#[test]
	fn pool_lowering_preserves_size_and_immediate_dense_routing() {
		let model = Model::new().pool(2).layer(8);
		let block = map_dense_block(&model.layers()[0]).expect("pool block lowers");
		let DenseBlock::Pool(pool) = block else {
			panic!("pool facade block lowered as another block kind");
		};

		assert_eq!(pool.size().get(), 2);
		assert_eq!(pool.group_to_neuron().map(NonZeroU64::get), Some(8));
		assert!(matches!(
			map_dense_block(&model.layers()[1]).expect("following dense layer lowers"),
			DenseBlock::Layer(layer) if layer.width().get() == 8
		));

		let unconnected = Model::new().pool(3);
		let DenseBlock::Pool(pool) = map_dense_block(&unconnected.layers()[0]).expect("terminal pool block lowers")
		else {
			panic!("terminal pool facade block lowered as another block kind");
		};
		assert_eq!(pool.size().get(), 3);
		assert_eq!(pool.group_to_neuron(), None);
	}

	#[test]
	fn residual_lowering_preserves_branch_and_post_add_operation_order() {
		let model = Model::new()
			.residual([
				crate::api::relu(),
				crate::api::layer(64),
				crate::api::relu(),
				crate::api::layer(32),
			])
			.norm(crate::api::layer_norm)
			.silu();
		let block = map_dense_block(&model.layers()[0]).expect("residual block lowers");
		let DenseBlock::Residual(residual) = block else {
			panic!("residual facade block lowered as an ordinary layer");
		};

		assert_eq!(residual.output_width().map(NonZeroU64::get), Some(32));
		assert_eq!(
			residual.branch(),
			[
				DenseResidualOperation::Operation(DenseOperation::Activation(DenseActivation::Relu)),
				DenseResidualOperation::Layer(DenseLayer::with_operations(
					NonZeroU64::new(64).unwrap(),
					[]
				)),
				DenseResidualOperation::Operation(DenseOperation::Activation(DenseActivation::Relu)),
				DenseResidualOperation::Layer(DenseLayer::with_operations(
					NonZeroU64::new(32).unwrap(),
					[]
				)),
			]
		);
		assert_eq!(
			residual.operations(),
			[
				DenseOperation::Normalization(DenseNormalization::Layer),
				DenseOperation::Activation(DenseActivation::Silu),
			]
		);
	}

	#[test]
	fn residual_training_package_constructs_a_structured_checkpoint_manifest() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-residual-package-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(&path, b"feature,target\n1,2\n2,3\n3,4\n4,5\n5,6\n6,7\n")
			.expect("write residual package fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(0.75);
		let model = Model::new()
			.residual([crate::api::relu(), crate::api::layer(4), crate::api::relu()])
			.layer(1)
			.loss(Loss::MeanSquaredError);
		let policy = Train::new()
			.optimizer(Optimizer::AdamW)
			.epochs(1)
			.lr(0.001)
			.cos();

		let package = compile_training_package(&policy, &data, &model).expect("residual package compiles");
		assert!(matches!(
			package.training.blocks()[0],
			DenseBlock::Residual(_)
		));
		assert_eq!(package.checkpoint.format_version(), 9);
	}

	#[test]
	fn public_embedding_and_attention_map_and_compile_without_numeric_token_normalization() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let model = Model::new()
			.embed(4)
			.vocab(8)
			.attn(2)
			.layer(1)
			.loss(Loss::MeanSquaredError);
		assert_eq!(
			map_dense_block(&model.layers()[0]).expect("embedding lowers"),
			DenseBlock::Embedding(DenseEmbedding::new(
				NonZeroU64::new(4).unwrap(),
				NonZeroU64::new(8).unwrap(),
			))
		);
		assert_eq!(
			map_dense_block(&model.layers()[1]).expect("attention lowers"),
			DenseBlock::Attention(DenseAttention::new(NonZeroU64::new(2).unwrap()))
		);
		assert!(matches!(
			map_dense_block(&Model::new().embed(4).layers()[0]),
			Err(TrainingError::Unsupported { detail }) if detail.contains("requires an immediate `.vocab")
		));

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-embedding-package-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(
			&path,
			b"token_zero,token_one,target\n0,1,0\n1,2,1\n2,3,0\n3,4,1\n4,5,0\n5,6,1\n6,7,0\n",
		)
		.expect("write embedding package fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.split(5.0 / 7.0);
		let policy = Train::new()
			.optimizer(Optimizer::AdamW)
			.epochs(1)
			.lr(0.001)
			.cos();
		let package = compile_training_package(&policy, &data, &model).expect("embedding package compiles");
		assert_eq!(package.checkpoint.format_version(), 13);
		assert_eq!(
			package.training.config().data_normalization,
			DenseDataNormalization::Identity
		);
		assert!(matches!(
			package.training.blocks(),
			[
				DenseBlock::Embedding(_),
				DenseBlock::Attention(_),
				DenseBlock::Layer(_)
			]
		));

		let normalized = data.clone().norm(DataNormalization::ZScore);
		assert!(matches!(
			compile_training_package(&policy, &normalized, &model),
			Err(TrainingError::Unsupported { detail }) if detail.contains("exact int32 token IDs")
		));
	}

	#[test]
	fn public_rnn_maps_numeric_columns_to_one_final_hidden_state() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let model = Model::new().rnn(4).layer(1).loss(Loss::MeanSquaredError);
		assert_eq!(
			map_dense_block(&model.layers()[0]).expect("RNN lowers"),
			DenseBlock::Rnn(DenseRnn::new(NonZeroU64::new(4).unwrap()))
		);
		assert!(matches!(
			map_dense_block(&Model::new().rnn(4).relu().layers()[0]),
			Err(TrainingError::Unsupported { detail }) if detail.contains("does not yet define chained")
		));

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-rnn-package-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(
			&path,
			b"step_zero,step_one,step_two,target\n1,2,3,1\n2,3,4,2\n3,4,5,3\n4,5,6,4\n5,6,7,5\n6,7,8,6\n7,8,9,7\n",
		)
		.expect("write RNN package fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(5.0 / 7.0);
		let policy = Train::new()
			.optimizer(Optimizer::AdamW)
			.epochs(1)
			.lr(0.001)
			.cos();
		let package = compile_training_package(&policy, &data, &model).expect("RNN package compiles");
		assert_eq!(package.checkpoint.format_version(), 14);
		assert!(matches!(
			package.training.blocks(),
			[DenseBlock::Rnn(_), DenseBlock::Layer(_)]
		));
	}

	#[test]
	fn public_gru_maps_numeric_columns_to_one_reset_before_final_hidden_state() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let model = Model::new().gru(4).layer(1).loss(Loss::MeanSquaredError);
		assert_eq!(
			map_dense_block(&model.layers()[0]).expect("GRU lowers"),
			DenseBlock::Gru(DenseGru::new(NonZeroU64::new(4).unwrap()))
		);
		assert!(matches!(
			map_dense_block(&Model::new().gru(4).relu().layers()[0]),
			Err(TrainingError::Unsupported { detail }) if detail.contains("does not yet define chained")
		));

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-gru-package-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(
			&path,
			b"step_zero,step_one,step_two,target\n1,2,3,1\n2,3,4,2\n3,4,5,3\n4,5,6,4\n5,6,7,5\n6,7,8,6\n7,8,9,7\n",
		)
		.expect("write GRU package fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(5.0 / 7.0);
		let policy = Train::new()
			.optimizer(Optimizer::AdamW)
			.epochs(1)
			.lr(0.001)
			.cos();
		let package = compile_training_package(&policy, &data, &model).expect("GRU package compiles");
		assert_eq!(package.checkpoint.format_version(), 15);
		assert!(matches!(
			package.training.blocks(),
			[DenseBlock::Gru(_), DenseBlock::Layer(_)]
		));
	}

	#[test]
	fn public_lstm_maps_numeric_columns_to_one_zero_cell_final_hidden_state() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let model = Model::new().lstm(4).layer(1).loss(Loss::MeanSquaredError);
		assert_eq!(
			map_dense_block(&model.layers()[0]).expect("LSTM lowers"),
			DenseBlock::Lstm(DenseLstm::new(NonZeroU64::new(4).unwrap()))
		);
		assert!(matches!(
			map_dense_block(&Model::new().lstm(4).relu().layers()[0]),
			Err(TrainingError::Unsupported { detail }) if detail.contains("does not yet define chained")
		));

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-lstm-package-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(
			&path,
			b"step_zero,step_one,step_two,target\n1,2,3,1\n2,3,4,2\n3,4,5,3\n4,5,6,4\n5,6,7,5\n6,7,8,6\n7,8,9,7\n",
		)
		.expect("write LSTM package fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(5.0 / 7.0);
		let policy = Train::new()
			.optimizer(Optimizer::AdamW)
			.epochs(1)
			.lr(0.001)
			.cos();
		let package = compile_training_package(&policy, &data, &model).expect("LSTM package compiles");
		assert_eq!(package.checkpoint.format_version(), 16);
		assert!(matches!(
			package.training.blocks(),
			[DenseBlock::Lstm(_), DenseBlock::Layer(_)]
		));
	}

	#[test]
	fn public_tree_and_forest_forms_compile_as_terminal_v12_models() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-tree-package-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(
			&path,
			b"feature_one,feature_two,target\n1,10,1\n2,20,0\n3,30,1\n4,40,0\n5,50,1\n6,60,0\n7,70,1\n",
		)
		.expect("write tree package fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(5.0 / 7.0);
		let policy = Train::new()
			.optimizer(Optimizer::AdamW)
			.epochs(1)
			.lr(0.001)
			.cos();
		let declarations = [
			(
				Model::new().lgbm(2).loss(Loss::MeanSquaredError),
				DenseTree::new(
					DenseTreeFamily::LightGbm,
					NonZeroU64::MIN,
					NonZeroU32::new(2).unwrap(),
				),
			),
			(
				Model::new().cbst(2).loss(Loss::MeanSquaredError),
				DenseTree::new(
					DenseTreeFamily::CatBoost,
					NonZeroU64::MIN,
					NonZeroU32::new(2).unwrap(),
				),
			),
			(
				Model::new().xgbst(2).loss(Loss::MeanSquaredError),
				DenseTree::new(
					DenseTreeFamily::XGBoost,
					NonZeroU64::MIN,
					NonZeroU32::new(2).unwrap(),
				),
			),
			(
				Model::new().forest(2).lgbm(2).loss(Loss::MeanSquaredError),
				DenseTree::new(
					DenseTreeFamily::LightGbm,
					NonZeroU64::new(2).unwrap(),
					NonZeroU32::new(2).unwrap(),
				),
			),
			(
				Model::new().forest(2).cbst(2).loss(Loss::MeanSquaredError),
				DenseTree::new(
					DenseTreeFamily::CatBoost,
					NonZeroU64::new(2).unwrap(),
					NonZeroU32::new(2).unwrap(),
				),
			),
			(
				Model::new().forest(2).xgbst(2).loss(Loss::MeanSquaredError),
				DenseTree::new(
					DenseTreeFamily::XGBoost,
					NonZeroU64::new(2).unwrap(),
					NonZeroU32::new(2).unwrap(),
				),
			),
		];

		for (model, expected) in declarations {
			let package = compile_training_package(&policy, &data, &model).expect("tree package compiles");
			assert_eq!(package.training.blocks(), [DenseBlock::Tree(expected)]);
			assert_eq!(package.checkpoint.format_version(), 12);
		}
	}

	#[test]
	fn specialized_blocks_are_never_silently_treated_as_dense_layers() {
		let bayes = Model::new()
			.bayes("housing", ["age", "job"])
			.loss(Loss::BinaryCrossEntropy);
		assert!(matches!(
			require_supported_model(&bayes),
			Err(TrainingError::Unsupported { detail })
				if detail.contains("Bayesian dependencies require a Bayesian model compiler")
		));

		for model in [
			Model::new().rnn(30),
			Model::new().gru(30),
			Model::new().lstm(30),
			Model::new().conv(12, 3),
			Model::new().pool(2),
			Model::new().lgbm(4),
			Model::new().cbst(4),
			Model::new().xgbst(4),
			Model::new().forest(20).lgbm(4),
			Model::new().kmeans(4),
		] {
			assert!(matches!(
				map_dense_layer(&model.layers()[0]),
				Err(TrainingError::Unsupported { detail })
					if detail.contains("accepts layer, perc, and residual blocks only")
			));
		}
	}

	#[test]
	fn knn_compilation_prepares_every_target_without_dense_training_policy() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(std::path::PathBuf);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				let _ = std::fs::remove_file(&self.0);
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-knn-compilation-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(
			&path,
			b"feature,class,numeric\n1,red,1.5\n2,blue,2.5\n3,red,3.5\n4,blue,4.5\n",
		)
		.expect("write KNN compilation fixture");
		let _fixture = RemoveOnDrop(path.clone());
		let data = Data::empty()
			.set(path.to_str().expect("temporary path is UTF-8"))
			.target(["class", "numeric"])
			.norm(DataNormalization::ZScore)
			.split(0.75);
		let artifact =
			compile_knn_model(&Train::new(), &data, &Model::new().knn(2)).expect("standalone KNN model compiles");

		assert_eq!(artifact.references().neighbors().get(), 2);
		assert_eq!(artifact.references().reference_rows(), 3);
		assert_eq!(artifact.references().outputs().len(), 2);
		assert_eq!(
			artifact.references().outputs()[0].schema().source_index(),
			1
		);
		assert_eq!(
			artifact.references().outputs()[1].schema().source_index(),
			2
		);
		assert_eq!(
			artifact.data_normalization(),
			Some(DenseDataNormalization::ZScore)
		);
		assert!(artifact.operations().is_empty());
	}

	#[test]
	fn knn_compilation_keeps_missing_resume_fresh_and_appends_an_existing_reference_set() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(Vec<std::path::PathBuf>);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				for path in &self.0 {
					let _ = std::fs::remove_file(path);
				}
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let csv = std::env::temp_dir().join(format!(
			"recipe-knn-resume-{}-{sequence}.csv",
			std::process::id(),
		));
		let model_path = csv.with_extension("ogdl");
		std::fs::write(&csv, b"feature,target\n1,a\n2,b\n3,a\n4,b\n").expect("write KNN resume fixture");
		let _fixture = RemoveOnDrop(vec![csv.clone(), model_path.clone()]);
		let data = Data::empty()
			.set(csv.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.split(0.75);
		let model = Model::new().knn(1);
		let policy = Train::new().resume(model_path.to_str().expect("temporary path is UTF-8"));

		let artifact = compile_knn_model(&policy, &data, &model).expect("missing KNN resume starts fresh");
		artifact.save(&model_path).expect("save existing KNN model");
		let continued = compile_knn_model(&policy, &data, &model).expect("existing KNN model continues");
		assert_eq!(continued.references().reference_rows(), 6);
		assert_eq!(
			continued.references().reference_source_rows(),
			[0, 1, 2, 0, 1, 2]
		);
		assert_eq!(
			continued.references().reference_feature_bits(),
			[
				1.0_f32.to_bits(),
				2.0_f32.to_bits(),
				3.0_f32.to_bits(),
				1.0_f32.to_bits(),
				2.0_f32.to_bits(),
				3.0_f32.to_bits(),
			]
		);
	}

	#[test]
	fn knn_compilation_rejects_optimizer_and_native_training_artifacts_before_data_io() {
		let data = Data::empty()
			.set("not-read.csv")
			.target("target")
			.split(0.75);
		let model = Model::new().knn(1);
		let policy = Train::new().optimizer(Optimizer::AdamW);
		let error = compile_knn_model(&policy, &data, &model).expect_err("KNN has no optimizer");
		assert!(matches!(
			error,
			TrainingError::Unsupported { detail } if detail.contains("has no optimizer")
		));

		let policy = Train::new().__recipe_save_pair("model.ogdl", "kernel.cubin");
		let error = compile_knn_model(&policy, &data, &model).expect_err("KNN has no training kernel");
		assert!(matches!(
			error,
			TrainingError::Unsupported { detail } if detail.contains("does not realize a native training kernel")
		));
	}

	#[test]
	fn knn_run_prepares_and_saves_without_fabricating_native_training_evidence() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(Vec<std::path::PathBuf>);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				for path in &self.0 {
					let _ = std::fs::remove_file(path);
				}
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let csv = std::env::temp_dir().join(format!(
			"recipe-knn-run-{}-{sequence}.csv",
			std::process::id(),
		));
		let destination = csv.with_extension("ogdl");
		std::fs::write(&csv, b"feature,target\n1,a\n2,b\n3,a\n4,b\n").expect("write KNN run fixture");
		let _fixture = RemoveOnDrop(vec![csv.clone(), destination.clone()]);
		let data = Data::empty()
			.set(csv.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.split(0.75);
		let policy = Train::new().save(destination.to_str().expect("temporary path is UTF-8"));
		let report = policy
			.try_run_with(&data, &Model::new().knn(1))
			.expect("KNN run prepares and saves");

		assert_eq!(report.kind(), TrainingModelKind::Knn);
		assert_eq!(report.run(), None);
		assert_eq!(report.bundle(), None);
		assert!(report.external_outputs().is_empty());
		assert!(report.metrics().is_empty());
		assert_eq!(
			report.validation_status(),
			ValidationMetricStatus::NotRequested
		);
		assert!(!report.gracefully_stopped());
		let model = report
			.knn_model()
			.expect("KNN report retains its semantic model");
		assert_eq!(
			std::fs::read(&destination).expect("read saved KNN model"),
			model.encode().unwrap()
		);
	}

	#[test]
	fn bayesian_compilation_saves_exact_observations_and_appends_an_existing_model() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(Vec<std::path::PathBuf>);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				for path in &self.0 {
					let _ = std::fs::remove_file(path);
				}
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let csv = std::env::temp_dir().join(format!(
			"recipe-bayes-resume-{}-{sequence}.csv",
			std::process::id(),
		));
		let model_path = csv.with_extension("ogdl");
		std::fs::write(
			&csv,
			b"weather,wind,play\nsun,breeze,otter\nsun,gale,falcon\nrain,breeze,otter\nrain,gale,falcon\nsun,breeze,otter\n",
		)
		.expect("write Bayesian resume fixture");
		let _fixture = RemoveOnDrop(vec![csv.clone(), model_path.clone()]);
		let data = Data::empty()
			.set(csv.to_str().expect("temporary path is UTF-8"))
			.target("play")
			.split(0.8);
		let model = Model::new().bayes("play", ["weather", "wind"]);
		let policy = Train::new().resume(model_path.to_str().expect("temporary path is UTF-8"));

		let artifact = compile_bayes_model(&policy, &data, &model).expect("missing Bayesian resume starts fresh");
		assert_eq!(artifact.references().reference_rows(), 4);
		artifact
			.save(&model_path)
			.expect("save Bayesian semantic model");
		let continued = compile_bayes_model(&policy, &data, &model).expect("existing Bayesian model continues");
		assert_eq!(continued.references().reference_rows(), 8);
		assert_eq!(
			continued.references().reference_source_rows(),
			[0, 1, 2, 3, 0, 1, 2, 3]
		);
	}

	#[test]
	fn bayesian_run_saves_without_fabricating_optimizer_or_native_training_evidence() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

		struct RemoveOnDrop(Vec<std::path::PathBuf>);

		impl Drop for RemoveOnDrop {
			fn drop(&mut self) {
				for path in &self.0 {
					let _ = std::fs::remove_file(path);
				}
			}
		}

		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let csv = std::env::temp_dir().join(format!(
			"recipe-bayes-run-{}-{sequence}.csv",
			std::process::id(),
		));
		let destination = csv.with_extension("ogdl");
		std::fs::write(
			&csv,
			b"weather,wind,play\nsun,breeze,otter\nsun,gale,falcon\nrain,breeze,otter\nrain,gale,falcon\nsun,breeze,otter\n",
		)
		.expect("write Bayesian run fixture");
		let _fixture = RemoveOnDrop(vec![csv.clone(), destination.clone()]);
		let data = Data::empty()
			.set(csv.to_str().expect("temporary path is UTF-8"))
			.target("play")
			.split(0.8);
		let policy = Train::new().save(destination.to_str().expect("temporary path is UTF-8"));
		let report = policy
			.try_run_with(&data, &Model::new().bayes("play", ["weather", "wind"]))
			.expect("Bayesian run prepares and saves");

		assert_eq!(report.kind(), TrainingModelKind::Bayes);
		assert_eq!(report.run(), None);
		assert_eq!(report.bundle(), None);
		assert!(report.external_outputs().is_empty());
		assert!(report.metrics().is_empty());
		let model = report
			.bayes_model()
			.expect("Bayesian report retains its semantic model");
		assert_eq!(
			std::fs::read(&destination).expect("read saved Bayesian model"),
			model.encode().unwrap()
		);

		let error = compile_bayes_model(
			&Train::new().optimizer(Optimizer::AdamW),
			&data,
			&Model::new().bayes("play", ["weather", "wind"]),
		)
		.expect_err("Bayesian preparation has no optimizer");
		assert!(matches!(
			error,
			TrainingError::Unsupported { detail } if detail.contains("has no optimizer")
		));
	}

	#[test]
	fn training_uses_one_full_partition_update_per_epoch() {
		let csv = std::env::temp_dir().join("recipe-full-partition-training-fixture.csv");
		let data_path = csv.to_str().expect("temporary path is UTF-8");
		std::fs::write(&csv, "feature,target\n1,0\n2,1\n3,0\n4,1\n").expect("write training fixture");
		let data = Data::empty()
			.set(data_path)
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(0.8);
		let model = Model::new().layer(1).loss(Loss::BinaryCrossEntropy);
		let policy = Train::new()
			.epochs(1)
			.lr(0.001)
			.cos()
			.optimizer(Optimizer::AdamW);
		let package = compile_training_package(&policy, &data, &model).expect("compile");
		assert_eq!(package.training.bounds().train_rows, 3);
		assert_eq!(
			package.training.bounds().training_iterations,
			recipe_core::LoopIterations::ONE
		);
		let _ = std::fs::remove_file(&csv);
	}

	#[test]
	fn omitted_epoch_bound_compiles_as_unbounded_with_constant_post_warmup_rate() {
		static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);
		let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let csv = std::env::temp_dir().join(format!(
			"recipe-unbounded-training-{}-{sequence}.csv",
			std::process::id(),
		));
		std::fs::write(&csv, "feature,target\n1,0\n2,1\n3,0\n4,1\n").expect("write unbounded training fixture");
		let data = Data::empty()
			.set(csv.to_str().expect("temporary path is UTF-8"))
			.target("target")
			.norm(DataNormalization::ZScore)
			.split(0.8);
		let model = Model::new().layer(1).loss(Loss::BinaryCrossEntropy);
		let policy = Train::new().lr(0.001).warmup(2).optimizer(Optimizer::AdamW);
		let package = compile_training_package(&policy, &data, &model).expect("compile unbounded training");
		assert_eq!(package.training.config().epochs, TrainingHorizon::Unbounded);
		assert_eq!(
			package.training.config().learning_rate_decay,
			LearningRateDecay::Constant
		);
		assert_eq!(
			package.training.program().iterations(),
			recipe_core::LoopIterations::UNBOUNDED
		);
		for schedule in [
			Train::new().lr(0.001).cos().optimizer(Optimizer::AdamW),
			Train::new().lr(0.001).exp().optimizer(Optimizer::AdamW),
		] {
			let error = compile_training_package(&schedule, &data, &model)
				.expect_err("endpoint-dependent decay without epochs must fail");
			assert!(matches!(
				error,
				TrainingError::Unsupported { detail } if detail.contains("requires `.epochs(...)` to define its endpoint")
			));
		}
		let _ = std::fs::remove_file(&csv);
	}

	#[test]
	fn live_metric_row_preserves_legacy_colors_widths_and_precision() {
		let mut rows = LiveMetricRows::new(presentations(), NonZeroU64::new(2).expect("two epochs"));
		assert_eq!(
			rows.push(
				0,
				NonZeroU64::MIN,
				MetricId::new(1),
				MetricValue::F32(0.7451),
			),
			None
		);
		assert_eq!(
			rows.push(
				0,
				NonZeroU64::MIN,
				MetricId::new(2),
				MetricValue::F32(0.5528),
			),
			None
		);

		let row = rows
			.push(
				1,
				NonZeroU64::new(2).unwrap(),
				MetricId::new(1),
				MetricValue::F32(1.0),
			)
			.expect("the next iteration completes the epoch-final row");

		assert_eq!(
			row,
			concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     1  ",
				"\u{1b}[38;2;39;125;255mloss\u{1b}[0m  0.7451  ",
				"\u{1b}[38;2;0;174;107mauroc\u{1b}[0m 0.5528",
			)
		);
		assert_eq!(
			rows.finish(),
			Some(concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     2  ",
				"\u{1b}[38;2;39;125;255mloss\u{1b}[0m  1.0000",
			)
			.to_owned())
		);
	}

	#[test]
	fn live_metric_epoch_cadence_keeps_configured_and_final_epochs() {
		let mut presentations = presentations();
		for presentation in presentations.metrics.values_mut() {
			presentation.log_cadence = NonZeroU64::new(3);
		}
		let mut rows = LiveMetricRows::new(presentations, NonZeroU64::new(5).expect("five epochs"));
		assert_eq!(
			rows.push(
				3,
				NonZeroU64::new(2).unwrap(),
				MetricId::new(1),
				MetricValue::F32(1.0),
			),
			None,
			"epoch two is not selected",
		);
		assert_eq!(
			rows.pending_sample,
			Some((3, NonZeroU64::new(2).unwrap())),
			"off-cadence state is retained so an accepted stop can flush it",
		);
		assert_eq!(
			rows.push(
				5,
				NonZeroU64::new(3).unwrap(),
				MetricId::new(1),
				MetricValue::F32(0.75),
			),
			None,
			"epoch three is selected",
		);
		assert_eq!(
			rows.push(
				9,
				NonZeroU64::new(5).unwrap(),
				MetricId::new(1),
				MetricValue::F32(0.5),
			),
			Some(concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     3  ",
				"\u{1b}[38;2;39;125;255mloss\u{1b}[0m  0.7500",
			)
			.to_owned(),),
			"the selected third epoch is completed before the final epoch",
		);
		assert_eq!(
			rows.pending_sample,
			Some((9, NonZeroU64::new(5).unwrap())),
			"the final epoch is selected even off cadence"
		);
		assert_eq!(
			rows.finish(),
			Some(concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     5  ",
				"\u{1b}[38;2;39;125;255mloss\u{1b}[0m  0.5000",
			)
			.to_owned(),),
		);
	}

	#[test]
	fn unbounded_live_metrics_flush_the_last_completed_epoch_off_cadence() {
		let mut presentations = presentations();
		for presentation in presentations.metrics.values_mut() {
			presentation.log_cadence = NonZeroU64::new(3);
		}
		let mut rows = LiveMetricRows::new(presentations, TrainingHorizon::Unbounded);
		assert_eq!(
			rows.push(
				1,
				NonZeroU64::new(2).unwrap(),
				MetricId::new(1),
				MetricValue::F32(0.625),
			),
			None,
		);
		assert_eq!(
			rows.finish(),
			Some(concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     2  ",
				"\u{1b}[38;2;39;125;255mloss\u{1b}[0m  0.6250",
			)
			.to_owned()),
		);
	}

	#[test]
	fn epoch_and_time_logging_use_a_hidden_full_partition_tick() {
		let trigger = MetricId::new(1);
		let plan = LiveMetricPlan {
			metrics: [(
				trigger,
				LiveMetricPresentation {
					label: "loss".to_owned(),
					width: 7,
					log_cadence: None,
					plot: false,
				},
			)]
			.into_iter()
			.collect(),
			host: LiveHostPresentation {
				epoch_log_cadence: NonZeroU64::new(2),
				time_log_cadence: NonZeroU64::new(2),
				..LiveHostPresentation::default()
			},
		};
		let mut rows = LiveMetricRows::new(plan, NonZeroU64::new(3).unwrap());
		assert_eq!(
			rows.push(
				0,
				NonZeroU64::new(1).unwrap(),
				trigger,
				MetricValue::F32(1.0)
			),
			None
		);
		assert_eq!(
			rows.push(
				1,
				NonZeroU64::new(2).unwrap(),
				trigger,
				MetricValue::F32(0.5)
			),
			None
		);
		let row = rows
			.push(
				2,
				NonZeroU64::new(3).unwrap(),
				trigger,
				MetricValue::F32(0.25),
			)
			.expect("epoch two host metrics are due");
		assert!(row.contains("epoch"));
		assert!(row.contains("    2"));
		assert!(row.contains("time"));
		assert!(!row.contains("loss"));
		let final_row = rows.finish().expect("final epoch is always due");
		assert!(final_row.contains("    3"));
	}

	#[test]
	fn plotting_executes_as_a_bounded_terminal_sparkline_without_implied_logging() {
		let metric = MetricId::new(1);
		let plan = LiveMetricPlan {
			metrics: [(
				metric,
				LiveMetricPresentation {
					label: "loss".to_owned(),
					width: 7,
					log_cadence: None,
					plot: true,
				},
			)]
			.into_iter()
			.collect(),
			host: LiveHostPresentation::default(),
		};
		let mut rows = LiveMetricRows::new(plan, NonZeroU64::new(70).unwrap());
		for epoch in 1..=70 {
			assert_eq!(
				rows.push(
					epoch - 1,
					NonZeroU64::new(epoch).unwrap(),
					metric,
					MetricValue::F32(epoch as f32),
				),
				None,
				"plot-only declarations must not masquerade as logs",
			);
		}
		assert_eq!(rows.finish(), None);
		let plots = rows.plot_lines();
		assert_eq!(plots.len(), 1);
		assert!(plots[0].starts_with("plot loss "));
		assert!(plots[0].contains("samples 70"));
		assert_eq!(rows.plots["loss"].values.len(), LIVE_PLOT_WIDTH);
	}

	#[test]
	fn live_metric_value_uses_legacy_nan_and_integer_padding() {
		assert_eq!(
			render_live_metric_value(MetricValue::F32(f32::NAN), 7),
			"    N/A"
		);
		assert_eq!(
			render_live_metric_value(MetricValue::F32(123_456.79), 7),
			"123456.7891"
		);
		assert_eq!(render_live_metric_value(MetricValue::I32(42), 5), "   42");
	}
}
