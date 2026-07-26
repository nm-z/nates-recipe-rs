use core::fmt;
use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use recipe_core::{BundleIdentity, MetricId, RunId};
use recipe_executor::{ExitImage, MetricValue, RunJournal, Watchdog};
use recipe_native_executor::{LocalCandidateFactory, StagedCrossBackend};
use recipe_prepare::{
	NativeArtifactCatalog, NativeArtifactProvider, NativeCandidateRealizer, NativeExecutorDriver, Preparer,
};
use recipe_training::{
	AdamWConfig, BinaryValidationConfig, CheckpointError, CheckpointManifest, CompiledTraining,
	CompletedTrainingCheckpoint, CompletedTrainingExecution, DenseActivation, DenseBlock, DenseBlockKind,
	DenseDataNormalization, DenseLayer, DenseLoss, DenseNormalization, DenseOperation, DensePool, DenseResidual,
	DenseResidualOperation, DenseTrainingConfig, FinalTrainingMetric, LearningRateDecay, MulticlassValidationConfig,
	TrainingBounds, TrainingCompileError, TrainingExecutionLimits, TrainingMetricKind, TrainingMetricObserver,
	TrainingMetricSample, ValidationMetricStatus, bounded_training_metric_channel, compile_dense_training,
	compile_dense_training_with_binary_validation, compile_dense_training_with_blocks,
	compile_dense_training_with_blocks_and_binary_validation,
	compile_dense_training_with_blocks_and_multiclass_validation, compile_dense_training_with_multiclass_validation,
	prepare_and_execute_local_training, prepare_and_execute_local_training_with_observer,
};

use crate::api::{
	Activation, Data, DataNormalization, DeclarationError, LayerNormalization, LayerOperation, LayerSpec,
	LearningRateSchedule, Loss, Metric, Model, Objective, Optimizer, ResidualOperation, ResidualSkip, SavePath,
	Train,
};
use crate::data_prepare::{DataPreparationError, prepare_data};
use crate::native_prepare::{NativePreparationError, with_current_native_preparation};

const HOST_STAGING_BYTES_PER_WORKER: usize = 1 << 20;
const TRAINING_WATCHDOG_POLLS: u32 = 65_536;
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
#[derive(Debug)]
#[non_exhaustive]
pub enum TrainingError {
	Declaration(DeclarationError),
	Data(DataPreparationError),
	Compile(TrainingCompileError),
	Checkpoint(CheckpointError),
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
			Self::Declaration(error) => write_training_error(formatter, "invalid training declaration", error),
			Self::Data(error) => write_training_error(formatter, "prepare training data", error),
			Self::Compile(error) => write_training_error(formatter, "compile training graph", error),
			Self::Checkpoint(error) => write_training_error(formatter, "checkpoint trained model", error),
			Self::Native(error) => write_training_error(formatter, "prepare current native system", error),
			Self::Unsupported { detail } => {
				write_training_error(formatter, "unsupported training declaration", detail)
			}
			Self::Runtime { stage, detail } => write_training_error(formatter, stage, detail),
		}
	}
}

fn write_training_error(formatter: &mut fmt::Formatter<'_>, stage: &str, detail: impl fmt::Display) -> fmt::Result {
	formatter.write_str("training failed")?;
	write_ogdl_error_node(formatter, 1, stage)?;
	let detail = detail.to_string();
	for (index, node) in detail.split(": ").enumerate() {
		write_ogdl_error_node(formatter, index.saturating_add(2), node)?;
	}
	Ok(())
}

fn write_ogdl_error_node(formatter: &mut fmt::Formatter<'_>, depth: usize, text: &str) -> fmt::Result {
	formatter.write_str("\n")?;
	for _ in 0..depth {
		formatter.write_str("\t")?;
	}
	let text = text.replace(['\t', '\r', '\n'], " ");
	formatter.write_str(if text.is_empty() {
		"unspecified"
	} else {
		&text
	})
}

impl std::error::Error for TrainingError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Declaration(error) => Some(error),
			Self::Data(error) => Some(error),
			Self::Compile(error) => Some(error),
			Self::Checkpoint(error) => Some(error),
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

impl From<NativePreparationError> for TrainingError {
	fn from(error: NativePreparationError) -> Self {
		Self::Native(error)
	}
}

pub type TrainingResult<T> = Result<T, TrainingError>;

/// Fully exited training result plus its lightweight semantic checkpoint
/// manifest. This value is constructed only after native teardown completes.
#[derive(Clone, Debug)]
pub struct TrainingReport {
	checkpoint: CompletedTrainingCheckpoint,
	validation_status: ValidationMetricStatus,
}

impl TrainingReport {
	fn new(
		execution: CompletedTrainingExecution,
		manifest: CheckpointManifest,
		validation_status: ValidationMetricStatus,
	) -> TrainingResult<Self> {
		Ok(Self {
			checkpoint: CompletedTrainingCheckpoint::new(execution, manifest)?,
			validation_status,
		})
	}

	#[must_use]
	pub const fn run(&self) -> RunId {
		self.checkpoint.run()
	}

	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity {
		self.checkpoint.bundle()
	}

	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] {
		self.checkpoint.external_outputs()
	}

	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] {
		self.checkpoint.metrics()
	}

	/// Whether requested validation reporting had known target rows.
	#[must_use]
	pub const fn validation_status(&self) -> ValidationMetricStatus {
		self.validation_status
	}

	#[must_use]
	pub const fn journal(&self) -> &RunJournal {
		self.checkpoint.journal()
	}

	#[must_use]
	pub const fn checkpoint_manifest(&self) -> &CheckpointManifest {
		self.checkpoint.manifest()
	}

	/// Save a deterministic Recipe-owned OGDL model. Pass `()` to use
	/// `model.ogdl`.
	///
	/// # Panics
	///
	/// Panics when the checkpoint cannot be saved.
	pub fn save(self, path: impl SavePath) -> Self {
		if let Err(error) = self.try_save(path) {
			panic!("{error}");
		}
		self
	}

	fn try_save(&self, path: impl SavePath) -> TrainingResult<()> {
		self.checkpoint
			.save(path.or_default())
			.map_err(TrainingError::from)
	}

	#[must_use]
	pub fn into_execution(self) -> CompletedTrainingExecution {
		self.checkpoint.into_execution()
	}
}

#[derive(Debug)]
struct CompiledTrainingPackage {
	training: CompiledTraining,
	checkpoint: CheckpointManifest,
}

/// Translate public data, model, and policy declarations into Recipe's static
/// GPU training program.
///
/// This boundary loads and losslessly prepares the user dataset, but performs
/// no native probing, artifact compilation, allocation, or execution.
pub fn compile_training(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTraining> {
	compile_training_graph(policy, data, model)
}

fn compile_training_package(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTrainingPackage> {
	let training = compile_training_graph(policy, data, model)?;
	let checkpoint = CheckpointManifest::from_compiled(&training)?;
	Ok(CompiledTrainingPackage {
		training,
		checkpoint,
	})
}

fn compile_training_graph(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTraining> {
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
			DenseBlock::Pool(_) | DenseBlock::Residual(_) => None,
		})
		.collect::<Option<Vec<_>>>()
		.unwrap_or_default();
	let batch_fraction = policy
		.batch_fraction()
		.ok_or_else(|| TrainingError::unsupported("a finite batch fraction is required"))?;
	let batch_size = batch_size_from_fraction(prepared.train().len(), batch_fraction)?;
	let epochs = policy
		.epoch_bound()
		.ok_or_else(|| TrainingError::unsupported("a finite epoch bound is required"))?;
	let epochs = u64::try_from(epochs)
		.map_err(|error| TrainingError::unsupported(format!("epoch bound does not fit u64: {error}")))?;
	let warmup_epochs = u64::try_from(policy.warmup_epoch_bound().unwrap_or(0))
		.map_err(|error| TrainingError::unsupported(format!("warmup epoch bound does not fit u64: {error}")))?;
	let learning_rate = policy
		.learning_rate()
		.unwrap_or_else(|| AdamWConfig::default().learning_rate);
	let learning_rate_decay = match policy.learning_rate_schedule() {
		Some(LearningRateSchedule::LinearDecay) => LearningRateDecay::Linear,
		Some(LearningRateSchedule::CosineDecay) => LearningRateDecay::Cosine,
		Some(LearningRateSchedule::ExponentialDecay) => LearningRateDecay::Exponential,
		None => {
			return Err(TrainingError::unsupported(
				"an explicit learning-rate decay is required",
			));
		}
	};
	let data_normalization = match data.normalization() {
		Some(DataNormalization::ZScore) => DenseDataNormalization::ZScore,
		Some(DataNormalization::MinMax) => DenseDataNormalization::MinMax,
		Some(DataNormalization::L2Norm) => DenseDataNormalization::L2Norm,
		None => {
			return Err(TrainingError::unsupported(
				"dense training requires explicit data normalization",
			));
		}
	};
	let config = DenseTrainingConfig {
		layers,
		loss,
		data_normalization,
		batch_size: NonZeroUsize::new(batch_size)
			.ok_or_else(|| TrainingError::unsupported("batch size must be nonzero"))?,
		epochs: NonZeroU64::new(epochs).ok_or_else(|| TrainingError::unsupported("epoch bound must be nonzero"))?,
		warmup_epochs,
		learning_rate_decay,
		gradient_clip_norm: model.gradient_clip_value().unwrap_or(1.0),
		normalization_epsilon: 1.0e-6,
		reduction_tree_lanes: 256,
		random_seed: 0x7265_6369_7065,
		adamw: AdamWConfig {
			learning_rate,
			..AdamWConfig::default()
		},
	};
	let binary_validation = binary_validation_config(policy, loss)?;
	let multiclass_validation = multiclass_validation_config(policy, loss)?;
	let contains_structured_blocks = blocks
		.iter()
		.any(|block| !matches!(block, DenseBlock::Layer(_)));
	let training = match (
		contains_structured_blocks,
		binary_validation,
		multiclass_validation,
	) {
		(false, Some(validation), None) => {
			compile_dense_training_with_binary_validation(&prepared, &config, &validation)
				.map_err(TrainingError::from)
		}
		(false, None, Some(validation)) => {
			compile_dense_training_with_multiclass_validation(&prepared, &config, &validation)
				.map_err(TrainingError::from)
		}
		(false, None, None) => compile_dense_training(&prepared, &config).map_err(TrainingError::from),
		(true, Some(validation), None) => {
			compile_dense_training_with_blocks_and_binary_validation(&prepared, &config, &blocks, &validation)
				.map_err(TrainingError::from)
		}
		(true, None, Some(validation)) => {
			compile_dense_training_with_blocks_and_multiclass_validation(&prepared, &config, &blocks, &validation)
				.map_err(TrainingError::from)
		}
		(true, None, None) => {
			compile_dense_training_with_blocks(&prepared, &config, &blocks).map_err(TrainingError::from)
		}
		(_, Some(_), Some(_)) => Err(TrainingError::unsupported(
			"one training run cannot request binary and multiclass validation simultaneously",
		)),
	}?;
	Ok(training)
}

fn batch_size_from_fraction(rows: usize, fraction: f32) -> TrainingResult<usize> {
	if !fraction.is_finite() || fraction <= 0.0 || fraction >= 1.0 {
		return Err(TrainingError::unsupported(format!(
			"batch fraction must be finite and in (0, 1), got {fraction}"
		)));
	}
	let bits = fraction.to_bits();
	let exponent = (bits >> 23) & 0xff;
	let fraction_bits = bits & 0x7f_ffff;
	let (numerator, denominator_power) = if exponent == 0 {
		(u128::from(fraction_bits), 149_u32)
	} else {
		(
			u128::from((1 << 23) | fraction_bits),
			150_u32.saturating_sub(exponent),
		)
	};
	let product = u128::try_from(rows)
		.map_err(|error| TrainingError::unsupported(format!("training row count does not fit u128: {error}")))?
		.checked_mul(numerator)
		.ok_or_else(|| TrainingError::unsupported("batch fraction multiplication overflowed u128"))?;
	let derived = if denominator_power >= 128 {
		0
	} else {
		product >> denominator_power
	};
	usize::try_from(derived.max(1))
		.map_err(|error| TrainingError::unsupported(format!("derived batch size does not fit usize: {error}")))
}

impl Train {
	/// Compile and execute this declaration with the preceding `recipe.data(...)`
	/// and `recipe.model()` declarations.
	///
	/// Preparation performs all discovery validation, placement, artifact
	/// generation, loading, warming, allocation, and finalization before the
	/// singular external data image for each device is admitted.
	///
	/// # Panics
	///
	/// Panics when training cannot be compiled, prepared, or executed.
	pub fn run(&self) -> TrainingReport {
		match self.try_run() {
			Ok(report) => report,
			Err(error) => panic!("{error}"),
		}
	}

	fn try_run(&self) -> TrainingResult<TrainingReport> {
		let (data, model) = crate::take_recipe_sequence().map_err(TrainingError::unsupported)?;
		self.try_run_with(&data, &model)
	}

	fn try_run_with(&self, data: &Data, model: &Model) -> TrainingResult<TrainingReport> {
		let package = compile_training_package(self, data, model)?;
		let execution = execute_current_training(self, &package.training)?;
		TrainingReport::new(
			execution,
			package.checkpoint,
			package.training.outputs().validation_status,
		)
	}
}

fn execute_current_training(policy: &Train, training: &CompiledTraining) -> TrainingResult<CompletedTrainingExecution> {
	report_unavailable_validation(training)?;
	let presentations = live_metric_presentations(policy, training);
	if presentations.is_empty() {
		return execute_current_training_native(training, None);
	}
	let epoch_cadence = u64::try_from(policy.log_interval().unwrap_or(1))
		.ok()
		.and_then(NonZeroU64::new)
		.ok_or_else(|| {
			TrainingError::runtime(
				"configure live metrics",
				"log cadence does not fit nonzero u64",
			)
		})?;
	let capacity =
		NonZeroUsize::new(LIVE_METRIC_CHANNEL_CAPACITY).expect("the fixed live metric channel capacity is nonzero");
	let (mut observer, receiver) =
		bounded_training_metric_channel(capacity, presentations.keys().copied(), NonZeroU64::MIN);
	let presenter = spawn_live_metric_presenter(receiver, presentations, training.bounds(), epoch_cadence)?;
	let result = execute_current_training_native(training, Some(&mut observer));
	drop(observer);
	let _ = presenter.join();
	result
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

fn execute_current_training_native(
	training: &CompiledTraining,
	mut observer: Option<&mut TrainingMetricObserver>,
) -> TrainingResult<CompletedTrainingExecution> {
	let run = next_run_id();
	let watchdog = Watchdog::new(TRAINING_WATCHDOG_POLLS)
		.map_err(|error| TrainingError::runtime("configure training watchdog", error.to_string()))?;
	let limits = TrainingExecutionLimits::new(watchdog);
	let worker_threads = std::thread::available_parallelism()
		.map(NonZeroUsize::get)
		.unwrap_or(1)
		.min(8);
	let result = with_current_native_preparation(|profile, _config, scope| {
		let (bindings, host, targets) = scope.into_parts();
		let host = host
			.backend_config(run, worker_threads, HOST_STAGING_BYTES_PER_WORKER)
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let (cuda, hsa) = bindings.into_parts();
		let bridge = StagedCrossBackend::new(cuda.clone(), hsa.clone());
		let factory = LocalCandidateFactory::production(Some(host), cuda, hsa, bridge);
		let driver = NativeExecutorDriver::new(factory);
		let compiler = targets
			.deferred_compiler()
			.map_err(NativePreparationError::TargetSpecification)?;
		let realizer = NativeCandidateRealizer::new(profile, compiler, driver)
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let provider = NativeArtifactProvider::new(NativeArtifactCatalog::default());
		let mut preparer = Preparer::new(provider, realizer);
		let execution = match observer.as_deref_mut() {
			Some(observer) => prepare_and_execute_local_training_with_observer(
				training,
				profile,
				&mut preparer,
				run,
				limits,
				observer,
			),
			None => prepare_and_execute_local_training(training, profile, &mut preparer, run, limits),
		};
		Ok(execution)
	})?;
	result.map_err(|error| TrainingError::runtime("execute native training", error.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveMetricPresentation {
	label: String,
	width: usize,
}

fn live_metric_presentations(
	policy: &Train,
	training: &CompiledTraining,
) -> BTreeMap<MetricId, LiveMetricPresentation> {
	training
		.outputs()
		.metric_bindings
		.iter()
		.filter(|binding| {
			policy.log_items()
				.iter()
				.any(|item| log_selects(item.metric(), binding.kind))
		})
		.map(|binding| {
			(
				binding.metric,
				LiveMetricPresentation {
					label: metric_label(binding.kind),
					width: metric_width(binding.kind),
				},
			)
		})
		.collect()
}

fn log_selects(requested: Metric, available: TrainingMetricKind) -> bool {
	match requested {
		Metric::Loss => matches!(
			available,
			TrainingMetricKind::BatchLoss
				| TrainingMetricKind::ValidationMeanBce
				| TrainingMetricKind::ValidationMeanCrossEntropy
		),
		Metric::Accuracy => available == TrainingMetricKind::Accuracy,
		Metric::AuRoc => available == TrainingMetricKind::AuRoc,
		Metric::AuPrc => available == TrainingMetricKind::AuPrc,
		Metric::Brier => available == TrainingMetricKind::BrierScore,
		Metric::CalibrationError => available == TrainingMetricKind::ExpectedCalibrationError,
		Metric::R2 | Metric::Epoch | Metric::LearningRate | Metric::Time | Metric::Device => false,
	}
}

fn metric_label(metric: TrainingMetricKind) -> String {
	match metric {
		TrainingMetricKind::BatchLoss => "loss".to_owned(),
		TrainingMetricKind::ValidationMeanBce => "validation_loss".to_owned(),
		TrainingMetricKind::ValidationMeanCrossEntropy => "validation_loss".to_owned(),
		TrainingMetricKind::Accuracy => "accuracy".to_owned(),
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
		TrainingMetricKind::BatchLoss
		| TrainingMetricKind::ValidationMeanBce
		| TrainingMetricKind::ValidationMeanCrossEntropy => 7,
		TrainingMetricKind::Accuracy => 6,
		TrainingMetricKind::AuRoc
		| TrainingMetricKind::AuPrc
		| TrainingMetricKind::BrierScore
		| TrainingMetricKind::ExpectedCalibrationError
		| TrainingMetricKind::RecallAt { .. } => 6,
	}
}

fn spawn_live_metric_presenter(
	receiver: Receiver<TrainingMetricSample>,
	presentations: BTreeMap<MetricId, LiveMetricPresentation>,
	bounds: TrainingBounds,
	epoch_cadence: NonZeroU64,
) -> TrainingResult<JoinHandle<()>> {
	std::thread::Builder::new()
		.name("recipe-live-metrics".to_owned())
		.spawn(move || {
			let stdout = io::stdout();
			let mut output = stdout.lock();
			let mut rows = LiveMetricRows::new(
				presentations,
				bounds.batches_per_epoch,
				bounds.epochs,
				epoch_cadence,
			);
			while let Ok(sample) = receiver.recv() {
				if rows
					.push(
						sample.zero_based_iteration.index(),
						sample.epoch,
						sample.metric,
						sample.value,
					)
					.is_some_and(|row| write_live_metric_row(&mut output, &row).is_err())
				{
					break;
				}
			}
			if let Some(row) = rows.finish() {
				let _ = write_live_metric_row(&mut output, &row);
			}
		})
		.map_err(|error| TrainingError::runtime("start live metric presenter", error.to_string()))
}

fn write_live_metric_row(output: &mut impl Write, row: &str) -> io::Result<()> {
	writeln!(output, "{row}")?;
	output.flush()
}

#[derive(Debug)]
struct LiveMetricRows {
	presentations: BTreeMap<MetricId, LiveMetricPresentation>,
	batches_per_epoch: u64,
	epochs: NonZeroU64,
	epoch_cadence: NonZeroU64,
	pending_sample: Option<(u64, NonZeroU64)>,
	pending_values: BTreeMap<MetricId, MetricValue>,
}

impl LiveMetricRows {
	fn new(
		presentations: BTreeMap<MetricId, LiveMetricPresentation>,
		batches_per_epoch: u64,
		epochs: NonZeroU64,
		epoch_cadence: NonZeroU64,
	) -> Self {
		Self {
			presentations,
			batches_per_epoch,
			epochs,
			epoch_cadence,
			pending_sample: None,
			pending_values: BTreeMap::new(),
		}
	}

	fn push(&mut self, iteration: u64, epoch: NonZeroU64, metric: MetricId, value: MetricValue) -> Option<String> {
		let completed = if self
			.pending_sample
			.is_some_and(|(pending, _)| pending != iteration)
		{
			let completed = self.pending_sample.map(|(_, epoch)| self.render(epoch));
			self.pending_sample = None;
			self.pending_values.clear();
			completed
		} else {
			None
		};
		if !self.selects(iteration, epoch) {
			return completed;
		}
		self.pending_sample = Some((iteration, epoch));
		if self.presentations.contains_key(&metric) {
			self.pending_values.insert(metric, value);
		}
		completed
	}

	fn finish(&mut self) -> Option<String> {
		let (_, epoch) = self.pending_sample.take()?;
		let row = self.render(epoch);
		self.pending_values.clear();
		Some(row)
	}

	fn selects(&self, iteration: u64, epoch: NonZeroU64) -> bool {
		if self.batches_per_epoch == 0 || iteration % self.batches_per_epoch != self.batches_per_epoch - 1 {
			return false;
		}
		epoch.get() % self.epoch_cadence.get() == 0 || epoch == self.epochs
	}

	fn render(&self, epoch: NonZeroU64) -> String {
		let mut fields = Vec::with_capacity(self.pending_values.len().saturating_add(1));
		fields.push(live_metric_field("epoch", 5, &epoch.to_string(), 0));
		for (metric, presentation) in &self.presentations {
			let Some(value) = self.pending_values.get(metric) else {
				continue;
			};
			let rendered = render_live_metric_value(*value, presentation.width);
			fields.push(live_metric_field(
				&presentation.label,
				presentation.width,
				&rendered,
				fields.len(),
			));
		}
		fields.join("  ")
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

fn next_run_id() -> RunId {
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
		Some(Objective::Builtin(Loss::Focal)) => Err(TrainingError::unsupported(
			"focal loss has no dense training lowering",
		)),
		Some(Objective::Reference(_)) => Err(TrainingError::unsupported(
			"model-referenced objectives have no dense training lowering",
		)),
		None => Err(TrainingError::unsupported(
			"dense training requires an explicit built-in objective",
		)),
	}
}

fn require_supported_policy(policy: &Train) -> TrainingResult<()> {
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
	if policy.resume_source().is_some() {
		return Err(TrainingError::unsupported(
			"resume requires a finalized Recipe weight-image format",
		));
	}
	Ok(())
}

fn binary_validation_config(policy: &Train, loss: DenseLoss) -> TrainingResult<Option<BinaryValidationConfig>> {
	let mut requested = policy.early_stopping().is_some();
	for item in policy.log_items().iter().chain(policy.plot_items()) {
		match item.metric() {
			Metric::Loss => {}
			Metric::AuRoc | Metric::AuPrc | Metric::Brier | Metric::CalibrationError => {
				requested = true;
			}
			Metric::Epoch | Metric::LearningRate | Metric::Time | Metric::Device => {}
			Metric::Accuracy if loss == DenseLoss::CrossEntropy => {}
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
	if loss != DenseLoss::BinaryCrossEntropy {
		return Err(TrainingError::unsupported(
			"binary classification validation metrics, early stopping, and calibration require BCE",
		));
	}
	let bins = NonZeroU32::new(15).expect("the Recipe ECE default is nonzero");
	let mut validation = BinaryValidationConfig::new(bins, Vec::new());
	if let Some(early) = policy.early_stopping() {
		if early.metric().metric() != Metric::AuPrc {
			return Err(TrainingError::unsupported(
				"the current GPU early-stop latch is defined for AUPRC",
			));
		}
		let patience = u64::try_from(early.patience())
			.ok()
			.and_then(NonZeroU64::new)
			.ok_or_else(|| TrainingError::unsupported("early-stop patience does not fit a nonzero u64"))?;
		validation = validation.with_auprc_early_stopping(patience);
	}
	Ok(Some(validation))
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
	if loss != DenseLoss::CrossEntropy {
		return Err(TrainingError::unsupported(
			"accuracy validation requires categorical cross entropy",
		));
	}
	Ok(Some(MulticlassValidationConfig))
}

fn map_dense_block(layer: &LayerSpec) -> TrainingResult<DenseBlock> {
	match layer {
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
		| LayerSpec::KnnPrediction { .. }
		| LayerSpec::KnnReduction { .. }
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
		Activation::Huber => Ok(DenseActivation::Huber),
		Activation::Tangent => Ok(DenseActivation::Tangent),
		Activation::Relu => Ok(DenseActivation::Relu),
		Activation::Gelu => Ok(DenseActivation::Gelu),
		Activation::Silu => Ok(DenseActivation::Silu),
		other => Err(TrainingError::unsupported(format!(
			"dense activation {other:?} has no Recipe primitive training lowering"
		))),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn presentations() -> BTreeMap<MetricId, LiveMetricPresentation> {
		[
			(
				MetricId::new(1),
				LiveMetricPresentation {
					label: "loss".to_owned(),
					width: 7,
				},
			),
			(
				MetricId::new(2),
				LiveMetricPresentation {
					label: "auroc".to_owned(),
					width: 6,
				},
			),
		]
		.into_iter()
		.collect()
	}

	#[test]
	fn loss_logging_uses_batch_loss_without_requesting_validation() {
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
	fn accuracy_logging_selects_only_categorical_cross_entropy_validation() {
		let policy = Train::new().log([crate::api::Accuracy]);

		assert_eq!(
			binary_validation_config(&policy, DenseLoss::CrossEntropy).expect("valid policy"),
			None
		);
		assert_eq!(
			multiclass_validation_config(&policy, DenseLoss::CrossEntropy).expect("valid policy"),
			Some(MulticlassValidationConfig)
		);
		assert!(multiclass_validation_config(&policy, DenseLoss::MeanSquaredError).is_err());
	}

	#[test]
	fn public_losses_map_to_distinct_dense_compiler_losses() {
		for (loss, expected) in [
			(Loss::BinaryCrossEntropy, DenseLoss::BinaryCrossEntropy),
			(Loss::MeanSquaredError, DenseLoss::MeanSquaredError),
			(Loss::MeanAbsoluteError, DenseLoss::MeanAbsoluteError),
			(Loss::CrossEntropy, DenseLoss::CrossEntropy),
			(Loss::Huber, DenseLoss::Huber),
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
			.batch(0.5)
			.epochs(1)
			.lr(0.001)
			.cos();

		let package = compile_training_package(&policy, &data, &model).expect("residual package compiles");
		assert!(matches!(
			package.training.blocks()[0],
			DenseBlock::Residual(_)
		));
		assert_eq!(package.checkpoint.format_version(), 6);
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
	fn batch_fraction_derives_samples_after_data_preparation() {
		assert_eq!(batch_size_from_fraction(5, 0.5).expect("half batch"), 2);
		assert_eq!(
			batch_size_from_fraction(100, 0.25).expect("quarter batch"),
			25
		);
		assert_eq!(
			batch_size_from_fraction(3, f32::MIN_POSITIVE).expect("minimum positive batch"),
			1
		);
	}

	#[test]
	fn live_metric_row_preserves_legacy_colors_widths_and_precision() {
		let mut rows = LiveMetricRows::new(
			presentations(),
			43,
			NonZeroU64::new(2).expect("two epochs"),
			NonZeroU64::MIN,
		);
		assert_eq!(
			rows.push(41, NonZeroU64::MIN, MetricId::new(1), MetricValue::F32(9.0),),
			None,
		);
		assert_eq!(rows.pending_sample, None);
		assert_eq!(
			rows.push(
				42,
				NonZeroU64::MIN,
				MetricId::new(1),
				MetricValue::F32(0.7451),
			),
			None
		);
		assert_eq!(
			rows.push(
				42,
				NonZeroU64::MIN,
				MetricId::new(2),
				MetricValue::F32(0.5528),
			),
			None
		);

		let row = rows
			.push(
				43,
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
		assert_eq!(rows.finish(), None);
	}

	#[test]
	fn live_metric_epoch_cadence_keeps_configured_and_final_epochs() {
		let mut rows = LiveMetricRows::new(
			presentations(),
			2,
			NonZeroU64::new(5).expect("five epochs"),
			NonZeroU64::new(3).expect("every third epoch"),
		);
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
		assert_eq!(rows.pending_sample, None);
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
