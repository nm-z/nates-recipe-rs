use core::fmt;
use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use recipe_core::{BundleIdentity, MetricId, RunId};
use recipe_executor::{ExitImage, MetricSample, MetricValue, RunJournal, Watchdog};
use recipe_native_executor::{LocalCandidateFactory, StagedCrossBackend};
use recipe_prepare::{
	NativeArtifactCatalog, NativeArtifactProvider, NativeCandidateRealizer, NativeExecutorDriver, Preparer,
};
use recipe_training::{
	AdamWConfig, BinaryValidationConfig, CheckpointError, CheckpointManifest, CompiledTraining,
	CompletedTrainingCheckpoint, CompletedTrainingExecution, DenseActivation, DenseBinaryDataset, DenseLayer,
	DenseTrainingConfig, FinalTrainingMetric, TemperatureScalingConfig, TrainingBounds, TrainingCompileError,
	TrainingExecutionLimits, TrainingMetricKind, TrainingMetricObserver, bounded_training_metric_channel,
	compile_dense_binary_training, compile_dense_binary_training_with_validation, prepare_and_execute_local_training,
	prepare_and_execute_local_training_with_observer,
};

use crate::api::{
	Activation, Calibration, Data, DeclarationError, LayerSpec, LearningRateSchedule, Loss, Metric, Model,
	Normalization, Objective, Optimizer, SavePath, Train,
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
			Self::Declaration(error) => write!(formatter, "invalid training declaration: {error}"),
			Self::Data(error) => write!(formatter, "prepare training data: {error}"),
			Self::Compile(error) => write!(formatter, "compile training graph: {error}"),
			Self::Checkpoint(error) => write!(formatter, "checkpoint trained model: {error}"),
			Self::Native(error) => write!(formatter, "prepare current native system: {error}"),
			Self::Unsupported { detail } => write!(formatter, "unsupported training declaration: {detail}"),
			Self::Runtime { stage, detail } => write!(formatter, "{stage}: {detail}"),
		}
	}
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
}

impl TrainingReport {
	fn new(execution: CompletedTrainingExecution, manifest: CheckpointManifest) -> TrainingResult<Self> {
		Ok(Self {
			checkpoint: CompletedTrainingCheckpoint::new(execution, manifest)?,
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
	pub fn save(&self, path: impl SavePath) -> TrainingResult<()> {
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
	Ok(compile_training_package(policy, data, model)?.training)
}

fn compile_training_package(policy: &Train, data: &Data, model: &Model) -> TrainingResult<CompiledTrainingPackage> {
	policy.declare(data, model)?;
	require_supported_model(model)?;
	require_supported_policy(policy)?;

	let prepared = prepare_data(data)?;
	let dataset = DenseBinaryDataset::from_prepared(&prepared)?;
	let layers = model
		.layers()
		.iter()
		.map(map_dense_layer)
		.collect::<TrainingResult<Vec<_>>>()?;
	let batch_size = policy
		.batch_size_value()
		.ok_or_else(|| TrainingError::unsupported("a finite batch_size is required"))?;
	let epochs = policy
		.epoch_bound()
		.ok_or_else(|| TrainingError::unsupported("a finite epoch bound is required"))?;
	let epochs = u64::try_from(epochs)
		.map_err(|error| TrainingError::unsupported(format!("epoch bound does not fit u64: {error}")))?;
	let warmup_epochs = u64::try_from(policy.warmup_epoch_bound().unwrap_or(0))
		.map_err(|error| TrainingError::unsupported(format!("warmup epoch bound does not fit u64: {error}")))?;
	let learning_rate = policy
		.learning_rate()
		.or_else(|| model.learning_rate())
		.unwrap_or_else(|| AdamWConfig::default().learning_rate);
	let config = DenseTrainingConfig {
		layers,
		batch_size: NonZeroUsize::new(batch_size)
			.ok_or_else(|| TrainingError::unsupported("batch size must be nonzero"))?,
		epochs: NonZeroU64::new(epochs).ok_or_else(|| TrainingError::unsupported("epoch bound must be nonzero"))?,
		warmup_epochs,
		gradient_clip_norm: policy.gradient_clip_value().unwrap_or(1.0),
		normalization_epsilon: 1.0e-6,
		reduction_tree_lanes: 256,
		random_seed: 0x7265_6369_7065,
		adamw: AdamWConfig {
			learning_rate,
			..AdamWConfig::default()
		},
	};
	let training = match binary_validation_config(policy)? {
		Some(validation) => compile_dense_binary_training_with_validation(&dataset, &config, &validation)
			.map_err(TrainingError::from),
		None => compile_dense_binary_training(&dataset, &config).map_err(TrainingError::from),
	}?;
	let checkpoint = CheckpointManifest::from_compiled(&prepared, &config, &training)?;
	Ok(CompiledTrainingPackage {
		training,
		checkpoint,
	})
}

impl Train {
	/// Compile and execute this declaration against the current exact measured
	/// machine profile.
	///
	/// Preparation performs all discovery validation, placement, artifact
	/// generation, loading, warming, allocation, and finalization before the
	/// singular external data image for each device is admitted.
	pub fn run(&self, data: &Data, model: &Model) -> TrainingResult<TrainingReport> {
		let package = compile_training_package(self, data, model)?;
		let execution = execute_current_training(self, &package.training)?;
		TrainingReport::new(execution, package.checkpoint)
	}
}

fn execute_current_training(policy: &Train, training: &CompiledTraining) -> TrainingResult<CompletedTrainingExecution> {
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
			TrainingMetricKind::BatchLoss | TrainingMetricKind::ValidationMeanBce
		),
		Metric::AuRoc => available == TrainingMetricKind::AuRoc,
		Metric::AuPrc => available == TrainingMetricKind::AuPrc,
		Metric::Brier => available == TrainingMetricKind::BrierScore,
		Metric::CalibrationError => available == TrainingMetricKind::ExpectedCalibrationError,
		Metric::RecallAt { threshold_bits } => {
			let threshold_bits = (f64::from_bits(threshold_bits) as f32).to_bits();
			matches!(
				available,
				TrainingMetricKind::RecallAt {
					threshold_bits: available,
				} if available == threshold_bits
			)
		}
		Metric::Accuracy | Metric::R2 | Metric::Epoch | Metric::LearningRate | Metric::Time | Metric::Device => {
			false
		}
	}
}

fn metric_label(metric: TrainingMetricKind) -> String {
	match metric {
		TrainingMetricKind::BatchLoss => "loss".to_owned(),
		TrainingMetricKind::ValidationMeanBce => "validation_loss".to_owned(),
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
		TrainingMetricKind::BatchLoss | TrainingMetricKind::ValidationMeanBce => 7,
		TrainingMetricKind::AuRoc
		| TrainingMetricKind::AuPrc
		| TrainingMetricKind::BrierScore
		| TrainingMetricKind::ExpectedCalibrationError
		| TrainingMetricKind::RecallAt { .. } => 6,
	}
}

fn spawn_live_metric_presenter(
	receiver: Receiver<MetricSample>,
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
					.push(sample.iteration.index(), sample.metric, sample.value)
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
	pending_iteration: Option<u64>,
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
			pending_iteration: None,
			pending_values: BTreeMap::new(),
		}
	}

	fn push(&mut self, iteration: u64, metric: MetricId, value: MetricValue) -> Option<String> {
		let completed = if self
			.pending_iteration
			.is_some_and(|pending| pending != iteration)
		{
			let completed = self.pending_iteration.map(|pending| self.render(pending));
			self.pending_iteration = None;
			self.pending_values.clear();
			completed
		} else {
			None
		};
		if !self.selects(iteration) {
			return completed;
		}
		self.pending_iteration = Some(iteration);
		if self.presentations.contains_key(&metric) {
			self.pending_values.insert(metric, value);
		}
		completed
	}

	fn finish(&mut self) -> Option<String> {
		let iteration = self.pending_iteration.take()?;
		let row = self.render(iteration);
		self.pending_values.clear();
		Some(row)
	}

	fn selects(&self, iteration: u64) -> bool {
		if self.batches_per_epoch == 0 || iteration % self.batches_per_epoch != self.batches_per_epoch - 1 {
			return false;
		}
		let epoch = iteration / self.batches_per_epoch;
		epoch % self.epoch_cadence.get() == 0 || epoch.saturating_add(1) == self.epochs.get()
	}

	fn render(&self, iteration: u64) -> String {
		let epoch = iteration / self.batches_per_epoch.max(1);
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

fn require_supported_model(model: &Model) -> TrainingResult<()> {
	if model.weights_source().is_some() {
		return Err(TrainingError::unsupported(
			"loading an existing weight image is not part of the dense training compiler",
		));
	}
	if model.normalization() != Some(Normalization::ZScore) {
		return Err(TrainingError::unsupported(
			"dense training currently requires explicit z-score normalization",
		));
	}
	if model.objective() != Some(&Objective::Builtin(Loss::BinaryCrossEntropy)) {
		return Err(TrainingError::unsupported(
			"dense binary training requires the built-in BCE objective",
		));
	}
	if model.optimizer_spec() != Some(Optimizer::AdamW) {
		return Err(TrainingError::unsupported(
			"dense binary training requires the Recipe-owned AdamW optimizer",
		));
	}
	Ok(())
}

fn require_supported_policy(policy: &Train) -> TrainingResult<()> {
	if policy.learning_rate_schedule() != Some(LearningRateSchedule::CosineDecay) {
		return Err(TrainingError::unsupported(
			"dense training currently requires an explicit cosine-decay schedule",
		));
	}
	if policy.resume_source().is_some() {
		return Err(TrainingError::unsupported(
			"resume requires a finalized Recipe weight-image format",
		));
	}
	if !policy.nodes().is_empty() {
		return Err(TrainingError::unsupported(
			"explicit multi-node placement is not yet connected to the native training runner",
		));
	}
	Ok(())
}

fn binary_validation_config(policy: &Train) -> TrainingResult<Option<BinaryValidationConfig>> {
	let mut requested = policy.early_stopping().is_some() || policy.calibration().is_some();
	let mut recall_thresholds = Vec::new();
	for item in policy.log_items().iter().chain(policy.plot_items()) {
		match item.metric() {
			Metric::Loss | Metric::AuRoc | Metric::AuPrc | Metric::Brier | Metric::CalibrationError => {
				requested = true;
			}
			Metric::RecallAt { .. } => {
				requested = true;
				let threshold = item
					.metric()
					.recall_threshold()
					.expect("RecallAt always exposes its threshold") as f32;
				if !recall_thresholds.contains(&threshold) {
					recall_thresholds.push(threshold);
				}
			}
			Metric::Epoch | Metric::LearningRate | Metric::Time | Metric::Device => {}
			Metric::Accuracy | Metric::R2 => {
				return Err(TrainingError::unsupported(format!(
					"metric {:?} is not defined for the current binary validation graph",
					item.metric()
				)));
			}
		}
	}
	if !requested {
		return Ok(None);
	}
	let bins = NonZeroU32::new(15).expect("the Recipe ECE default is nonzero");
	let mut validation = BinaryValidationConfig::new(bins, recall_thresholds);
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
	if let Some(calibration) = policy.calibration() {
		match calibration {
			Calibration::TemperatureScaling => {
				validation = validation.with_temperature_scaling(TemperatureScalingConfig::default());
			}
		}
	}
	Ok(Some(validation))
}

fn map_dense_layer(layer: &LayerSpec) -> TrainingResult<DenseLayer> {
	let LayerSpec::Dense { units, activation } = layer else {
		return Err(TrainingError::unsupported(
			"the current training compiler accepts dense layers only",
		));
	};
	let width = u64::try_from(*units)
		.map_err(|error| TrainingError::unsupported(format!("dense layer width does not fit u64: {error}")))?;
	let width =
		NonZeroU64::new(width).ok_or_else(|| TrainingError::unsupported("dense layer width must be nonzero"))?;
	let activation = match activation {
		Activation::Linear => DenseActivation::Linear,
		Activation::Silu => DenseActivation::Silu,
		other => {
			return Err(TrainingError::unsupported(format!(
				"dense activation {other:?} has no Recipe primitive training lowering"
			)));
		}
	};
	Ok(DenseLayer::new(width, activation))
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
	fn live_metric_row_preserves_legacy_colors_widths_and_precision() {
		let mut rows = LiveMetricRows::new(
			presentations(),
			43,
			NonZeroU64::new(2).expect("two epochs"),
			NonZeroU64::MIN,
		);
		assert_eq!(rows.push(41, MetricId::new(1), MetricValue::F32(9.0)), None);
		assert_eq!(rows.pending_iteration, None);
		assert_eq!(
			rows.push(42, MetricId::new(1), MetricValue::F32(0.7451)),
			None
		);
		assert_eq!(
			rows.push(42, MetricId::new(2), MetricValue::F32(0.5528)),
			None
		);

		let row = rows
			.push(43, MetricId::new(1), MetricValue::F32(1.0))
			.expect("the next iteration completes the epoch-final row");

		assert_eq!(
			row,
			concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     0  ",
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
			rows.push(3, MetricId::new(1), MetricValue::F32(1.0)),
			None,
			"epoch one is not selected",
		);
		assert_eq!(rows.pending_iteration, None);
		assert_eq!(
			rows.push(9, MetricId::new(1), MetricValue::F32(0.5)),
			None,
			"the final epoch is selected even off cadence",
		);
		assert_eq!(
			rows.finish(),
			Some(concat!(
				"\u{1b}[38;2;242;40;60mepoch\u{1b}[0m     4  ",
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
