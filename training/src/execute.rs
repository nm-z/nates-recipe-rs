use alloc::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};
use core::{
	error::Error as StdError,
	fmt,
	num::{NonZeroU64, NonZeroUsize},
	sync::atomic::{AtomicBool, Ordering},
	time::Duration,
};
use std::{
	sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
	time::Instant,
};

use recipe_core::{ArtifactIdentity, BundleIdentity, ByteCount, DType, DeviceId, DeviceKind, Digest, DiscoveryIdentity, FinalizedBundle, InitDataImage, LoopIterations, MetricId, MetricPurpose, MetricSlotId, RealizationIdentity, ResolvedTransferEndpoint, ResolvedValueLocation, RunId, RunPhase, TargetIdentity, TaskId, TaskKind, ToolchainIdentity, TopologyIdentity, TransferEndpoint, ValueId};
use recipe_executor::{Backend, DeviceImage, ExecutorError, ExitImage, LogicalEvent, LoopStatus, MetricSample, PhysicalCall, PreparedRun, RunFailure, RunJournal, Watchdog};
use recipe_language::{ContiguousOrder, TensorLayout};
use recipe_native_executor::{CandidateCrossBackendTransfer, CrossBackendTransfer, LocalError, LocalPreparedSession, NativeExecutionEvidence, RuntimeArtifactKind, ValidatedCandidateSession};
use recipe_prepare::{ArtifactProvider, CandidateRealizer, NativeArtifact, PrepareError, PreparedNativeSession, Preparer};
use recipe_probe::MeasuredProfile;

use crate::{CompiledInference, CompiledKnnInference, CompiledTraining, DenseTask, InferenceExternalInput, InferenceInputRole, InferenceOutputContract, InferencePredictionKind, InferenceTask, KnnInferenceOutputContract, KnnInferencePredictionKind, OwnedExternalInput, TrainingBounds};

pub type TrainingExecutionResult<T> = Result<T, TrainingExecutionError>;
pub type InferenceExecutionResult<T> = Result<T, InferenceExecutionError>;

const BLOCKING_POLL_INITIAL_DELAY: Duration = Duration::from_micros(50);
const BLOCKING_POLL_MAXIMUM_DELAY: Duration = Duration::from_millis(2);

#[derive(Clone, Copy, Debug)]
struct BlockingPollBackoff {
	next_delay: Duration,
}

impl BlockingPollBackoff {
	const fn new() -> Self {
		return Self {
			next_delay: BLOCKING_POLL_INITIAL_DELAY,
		};
	}

	const fn reset(&mut self) { self.next_delay = BLOCKING_POLL_INITIAL_DELAY; }

	fn wait(&mut self) {
		std::thread::sleep(self.next_delay);
		self.advance();
	}

	fn advance(&mut self) {
		self.next_delay = self
			.next_delay
			.saturating_mul(2)
			.min(BLOCKING_POLL_MAXIMUM_DELAY);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingExecutionLimits {
	pub watchdog: Watchdog,
}

impl TrainingExecutionLimits {
	#[must_use]
	#[inline]
	pub const fn new(watchdog: Watchdog) -> Self { return Self { watchdog }; }
}

#[derive(Clone, Copy, Debug)]
pub struct TrainingExecutionControl<'a> {
	stop_requested: Option<&'a AtomicBool>,
}

impl<'a> TrainingExecutionControl<'a> {
	#[must_use]
	#[inline]
	pub const fn graceful_stop(stop_requested: &'a AtomicBool) -> Self {
		return Self {
			stop_requested: Some(stop_requested),
		};
	}

	fn stop_requested(self) -> bool {
		return self
			.stop_requested
			.is_some_and(|requested| return requested.load(Ordering::Acquire));
	}

	const fn has_stop_source(self) -> bool { return self.stop_requested.is_some(); }
}

const fn validate_training_execution_control(iterations: LoopIterations, control: TrainingExecutionControl<'_>) -> TrainingExecutionResult<()> {
	if iterations.is_unbounded() && !control.has_stop_source() {
		return Err(TrainingExecutionError::UnboundedTrainingRequiresStopControl);
	}
	return Ok(());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InferenceExecutionLimits {
	pub watchdog: Watchdog,
}

impl InferenceExecutionLimits {
	#[must_use]
	#[inline]
	pub const fn new(watchdog: Watchdog) -> Self { return Self { watchdog }; }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingMetricObserverStats {
	pub observed: u64,
	pub cadence_eligible: u64,
	pub delivered: u64,
	pub dropped: u64,
}

#[derive(Debug)]
pub struct TrainingMetricObserver {
	sender: SyncSender<TrainingMetricSample>,
	selected: BTreeMap<MetricId, u64>,
	cadence: NonZeroU64,
	stats: TrainingMetricObserverStats,
}

impl TrainingMetricObserver {
	#[must_use]
	#[inline]
	pub const fn stats(&self) -> TrainingMetricObserverStats { return self.stats; }

	fn try_observe(&mut self, sample: &TrainingMetricSample) {
		let Some(observed) = self.selected.get_mut(&sample.metric) else {
			return;
		};
		*observed = observed.saturating_add(1);
		self.stats.observed = self.stats.observed.saturating_add(1);
		if !observed.is_multiple_of(self.cadence.get()) {
			return;
		}
		self.stats.cadence_eligible = self.stats.cadence_eligible.saturating_add(1);
		match self.sender.try_send(sample.clone()) {
			Ok(()) => self.stats.delivered = self.stats.delivered.saturating_add(1),
			Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
				self.stats.dropped = self.stats.dropped.saturating_add(1);
			}
		}
	}
}

#[must_use]
#[inline]
pub fn bounded_training_metric_channel(capacity: NonZeroUsize, selected: impl IntoIterator<Item = MetricId>, cadence: NonZeroU64) -> (TrainingMetricObserver, Receiver<TrainingMetricSample>) {
	let (sender, receiver) = sync_channel(capacity.get());
	let observer = TrainingMetricObserver {
		sender,
		selected: selected
			.into_iter()
			.map(|metric| return (metric, 0))
			.collect(),
		cadence,
		stats: TrainingMetricObserverStats::default(),
	};
	return (observer, receiver);
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingMetricSample {
	pub sequence: u64,
	pub epoch: NonZeroU64,
	pub zero_based_iteration: recipe_core::LoopIteration,
	pub task: TaskId,
	pub slot: MetricSlotId,
	pub metric: MetricId,
	pub value: recipe_executor::MetricValue,
}

impl TrainingMetricSample {
	const fn from_executor(sample: &MetricSample) -> Self {
		return Self {
			sequence: sample.sequence,
			epoch: NonZeroU64::MIN.saturating_add(sample.iteration.index()),
			zero_based_iteration: sample.iteration,
			task: sample.task,
			slot: sample.slot,
			metric: sample.metric,
			value: sample.value,
		};
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinalTrainingMetric {
	pub slot: MetricSlotId,
	pub metric: MetricId,
	pub sample: Option<TrainingMetricSample>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeKernelFormat {
	Cubin,
	Hsaco,
}

impl NativeKernelFormat {
	#[must_use]
	#[inline]
	pub const fn extension(self) -> &'static str {
		match self {
			Self::Cubin => return "cubin",
			Self::Hsaco => return "hsaco",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealizedNativeKernel {
	format: NativeKernelFormat,
	target: TargetIdentity,
	toolchain: ToolchainIdentity,
	digest: Digest,
	bytes: Arc<[u8]>,
	entries: Vec<ArtifactIdentity>,
}

impl RealizedNativeKernel {
	#[must_use]
	#[inline]
	pub const fn format(&self) -> NativeKernelFormat { return self.format; }

	#[must_use]
	#[inline]
	pub const fn target(&self) -> &TargetIdentity { return &self.target; }

	#[must_use]
	#[inline]
	pub const fn toolchain(&self) -> &ToolchainIdentity { return &self.toolchain; }

	#[must_use]
	#[inline]
	pub const fn digest(&self) -> Digest { return self.digest; }

	#[must_use]
	#[inline]
	pub fn bytes(&self) -> &[u8] { return &self.bytes; }

	#[must_use]
	#[inline]
	pub fn entries(&self) -> &[ArtifactIdentity] { return &self.entries; }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealizedNativeKernelSet {
	realization: RealizationIdentity,
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
	kernels: Vec<RealizedNativeKernel>,
}

impl RealizedNativeKernelSet {
	#[must_use]
	#[inline]
	pub const fn realization(&self) -> RealizationIdentity { return self.realization; }

	#[must_use]
	#[inline]
	pub const fn topology(&self) -> TopologyIdentity { return self.topology; }

	#[must_use]
	#[inline]
	pub const fn discovery(&self) -> DiscoveryIdentity { return self.discovery; }

	#[must_use]
	#[inline]
	pub fn kernels(&self) -> &[RealizedNativeKernel] { return &self.kernels; }
}

fn retain_realized_native_kernels(realization: &recipe_core::RealizationProfile, artifacts: Vec<NativeArtifact>) -> RealizedNativeKernelSet {
	let mut kernels = Vec::<RealizedNativeKernel>::new();
	for artifact in artifacts {
		let identity = artifact.identity().clone();
		let runtime = artifact.runtime();
		let format = match *runtime.kind() {
			RuntimeArtifactKind::Cuda { .. } => NativeKernelFormat::Cubin,
			RuntimeArtifactKind::Hsa { .. } => NativeKernelFormat::Hsaco,
		};
		if let Some(kernel) = kernels.iter_mut().find(|kernel| {
			return kernel.format == format && kernel.target == identity.target && kernel.toolchain == identity.toolchain && kernel.digest == identity.digest && kernel.bytes.as_ref() == runtime.bytes().as_ref();
		}) {
			kernel.entries.push(identity);
			continue;
		}
		kernels.push(RealizedNativeKernel {
			format,
			target: identity.target.clone(),
			toolchain: identity.toolchain.clone(),
			digest: identity.digest,
			bytes: Arc::clone(runtime.bytes()),
			entries: vec![identity],
		});
	}
	return RealizedNativeKernelSet {
		realization: realization.identity,
		topology: realization.topology,
		discovery: realization.discovery,
		kernels,
	};
}

#[derive(Clone, Debug)]
pub struct CompletedTrainingExecution {
	run: RunId,
	bundle: BundleIdentity,
	external_outputs: Vec<ExitImage>,
	external_output_values: Vec<ValueId>,
	metrics: Vec<FinalTrainingMetric>,
	native_kernels: RealizedNativeKernelSet,
	native_evidence: NativeExecutionEvidence,
	training_evidence: TrainingExecutionEvidence,
	journal: RunJournal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingExecutionEvidence {
	bounds: TrainingBounds,
	logical_updates_per_epoch: u64,
	optimizer_parameter_kernels: usize,
	optimizer_parameter_tasks: usize,
	optimizer_parameter_submissions: usize,
	loop_iterations_started: usize,
	loop_iterations_completed: usize,
	loop_calculation_tasks: usize,
	non_gpu_calculation_tasks: usize,
	non_gpu_calculation_submissions: usize,
	logical_events_compacted: u128,
	physical_calls_compacted: u128,
}

impl TrainingExecutionEvidence {
	#[must_use]
	#[inline]
	pub const fn bounds(&self) -> TrainingBounds { return self.bounds; }

	#[must_use]
	#[inline]
	pub const fn logical_updates_per_epoch(&self) -> u64 { return self.logical_updates_per_epoch; }

	#[must_use]
	#[inline]
	pub const fn optimizer_parameter_kernels(&self) -> usize { return self.optimizer_parameter_kernels; }

	#[must_use]
	#[inline]
	pub const fn optimizer_parameter_tasks(&self) -> usize { return self.optimizer_parameter_tasks; }

	#[must_use]
	#[inline]
	pub const fn optimizer_parameter_submissions(&self) -> usize { return self.optimizer_parameter_submissions; }

	#[must_use]
	#[inline]
	pub const fn loop_iterations_started(&self) -> usize { return self.loop_iterations_started; }

	#[must_use]
	#[inline]
	pub const fn loop_iterations_completed(&self) -> usize { return self.loop_iterations_completed; }

	#[must_use]
	#[inline]
	pub const fn loop_calculation_tasks(&self) -> usize { return self.loop_calculation_tasks; }

	#[must_use]
	#[inline]
	pub const fn non_gpu_calculation_tasks(&self) -> usize { return self.non_gpu_calculation_tasks; }

	#[must_use]
	#[inline]
	pub const fn non_gpu_calculation_submissions(&self) -> usize { return self.non_gpu_calculation_submissions; }

	#[must_use]
	#[inline]
	pub const fn logical_events_compacted(&self) -> u128 { return self.logical_events_compacted; }

	#[must_use]
	#[inline]
	pub const fn physical_calls_compacted(&self) -> u128 { return self.physical_calls_compacted; }
}

impl CompletedTrainingExecution {
	#[must_use]
	#[inline]
	pub const fn run(&self) -> RunId { return self.run; }

	#[must_use]
	#[inline]
	pub const fn bundle(&self) -> BundleIdentity { return self.bundle; }

	#[must_use]
	#[inline]
	pub fn external_outputs(&self) -> &[ExitImage] { return &self.external_outputs; }

	#[must_use]
	#[inline]
	pub fn external_output_values(&self) -> &[ValueId] { return &self.external_output_values; }

	#[must_use]
	#[inline]
	pub fn metrics(&self) -> &[FinalTrainingMetric] { return &self.metrics; }

	#[must_use]
	#[inline]
	pub const fn native_kernels(&self) -> &RealizedNativeKernelSet { return &self.native_kernels; }

	#[must_use]
	#[inline]
	pub const fn native_evidence(&self) -> &NativeExecutionEvidence { return &self.native_evidence; }

	#[must_use]
	#[inline]
	pub const fn training_evidence(&self) -> &TrainingExecutionEvidence { return &self.training_evidence; }

	#[must_use]
	#[inline]
	pub const fn journal(&self) -> &RunJournal { return &self.journal; }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferencePrediction {
	contract: InferenceOutputContract,
	bytes: Vec<u8>,
}

impl InferencePrediction {
	#[must_use]
	#[inline]
	pub const fn contract(&self) -> &InferenceOutputContract { return &self.contract; }

	#[must_use]
	#[inline]
	pub fn bytes(&self) -> &[u8] { return &self.bytes; }

	#[must_use]
	#[inline]
	pub fn into_parts(self) -> (InferenceOutputContract, Vec<u8>) { return (self.contract, self.bytes); }
}

#[derive(Clone, Debug)]
pub struct CompletedInferenceExecution {
	run: RunId,
	bundle: BundleIdentity,
	prediction: InferencePrediction,
	native_kernels: RealizedNativeKernelSet,
	native_evidence: NativeExecutionEvidence,
	elapsed: Duration,
	journal: RunJournal,
}

impl CompletedInferenceExecution {
	#[must_use]
	#[inline]
	pub const fn run(&self) -> RunId { return self.run; }

	#[must_use]
	#[inline]
	pub const fn bundle(&self) -> BundleIdentity { return self.bundle; }

	#[must_use]
	#[inline]
	pub const fn prediction(&self) -> &InferencePrediction { return &self.prediction; }

	#[must_use]
	#[inline]
	pub const fn native_kernels(&self) -> &RealizedNativeKernelSet { return &self.native_kernels; }

	#[must_use]
	#[inline]
	pub const fn native_evidence(&self) -> &NativeExecutionEvidence { return &self.native_evidence; }

	#[must_use]
	#[inline]
	pub const fn elapsed(&self) -> Duration { return self.elapsed; }

	#[must_use]
	#[inline]
	pub const fn journal(&self) -> &RunJournal { return &self.journal; }

	#[must_use]
	#[inline]
	pub fn into_parts(
		self,
	) -> (
		RunId,
		BundleIdentity,
		InferencePrediction,
		RealizedNativeKernelSet,
		NativeExecutionEvidence,
		Duration,
		RunJournal,
	) {
		return (
			self.run,
			self.bundle,
			self.prediction,
			self.native_kernels,
			self.native_evidence,
			self.elapsed,
			self.journal,
		);
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnInferencePrediction {
	contract: KnnInferenceOutputContract,
	bytes: Vec<u8>,
}

impl KnnInferencePrediction {
	#[must_use]
	#[inline]
	pub const fn contract(&self) -> &KnnInferenceOutputContract { return &self.contract; }

	#[must_use]
	#[inline]
	pub fn bytes(&self) -> &[u8] { return &self.bytes; }

	#[must_use]
	#[inline]
	pub fn into_parts(self) -> (KnnInferenceOutputContract, Vec<u8>) { return (self.contract, self.bytes); }
}

#[derive(Clone, Debug)]
pub struct CompletedKnnInferenceExecution {
	run: RunId,
	bundle: BundleIdentity,
	predictions: Vec<KnnInferencePrediction>,
	native_evidence: NativeExecutionEvidence,
	elapsed: Duration,
	journal: RunJournal,
}

impl CompletedKnnInferenceExecution {
	#[must_use]
	#[inline]
	pub const fn run(&self) -> RunId { return self.run; }

	#[must_use]
	#[inline]
	pub const fn bundle(&self) -> BundleIdentity { return self.bundle; }

	#[must_use]
	#[inline]
	pub fn predictions(&self) -> &[KnnInferencePrediction] { return &self.predictions; }

	#[must_use]
	#[inline]
	pub const fn journal(&self) -> &RunJournal { return &self.journal; }

	#[must_use]
	#[inline]
	pub const fn elapsed(&self) -> Duration { return self.elapsed; }

	#[must_use]
	#[inline]
	pub const fn native_evidence(&self) -> &NativeExecutionEvidence { return &self.native_evidence; }

	#[must_use]
	#[inline]
	pub fn into_parts(
		self,
	) -> (
		RunId,
		BundleIdentity,
		Vec<KnnInferencePrediction>,
		NativeExecutionEvidence,
		Duration,
		RunJournal,
	) {
		return (
			self.run,
			self.bundle,
			self.predictions,
			self.native_evidence,
			self.elapsed,
			self.journal,
		);
	}
}

#[derive(Clone, Debug)]
pub struct InferenceRunFailure {
	run: RunId,
	bundle: BundleIdentity,
	journal: Option<RunJournal>,
	cleanup_error: Option<ExecutorError>,
}

impl InferenceRunFailure {
	#[must_use]
	#[inline]
	pub const fn run(&self) -> RunId { return self.run; }

	#[must_use]
	#[inline]
	pub const fn bundle(&self) -> BundleIdentity { return self.bundle; }

	#[must_use]
	#[inline]
	pub const fn journal(&self) -> Option<&RunJournal> { return self.journal.as_ref(); }

	#[must_use]
	#[inline]
	pub const fn cleanup_error(&self) -> Option<ExecutorError> { return self.cleanup_error; }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum TrainingExecutionError {
	Preparation(PrepareError),
	NativeHandoff(Box<dyn StdError + Send + Sync>),
	Executor(ExecutorError),
	DuplicateExternalInput {
		value: ValueId,
	},
	DuplicateInitDevice {
		device: DeviceId,
	},
	DuplicateImageMember {
		device: DeviceId,
		value: ValueId,
	},
	MissingExternalInput {
		device: DeviceId,
		value: ValueId,
	},
	ImageMemberDTypeMismatch {
		device: DeviceId,
		value: ValueId,
	},
	ImageMemberSizeMismatch {
		device: DeviceId,
		value: ValueId,
		expected: ByteCount,
		actual: ByteCount,
	},
	ImageSizeUnsupported {
		device: DeviceId,
		bytes: ByteCount,
	},
	ImageMemberOutOfBounds {
		device: DeviceId,
		value: ValueId,
	},
	ImageMembersOverlap {
		device: DeviceId,
		first: ValueId,
		second: ValueId,
	},
	LoopExternalTransfer {
		task: TaskId,
	},
	ExternalOutputMapping {
		detail: String,
	},
	InvalidTrainingBounds {
		detail: &'static str,
	},
	UnboundedTrainingRequiresStopControl,
	LoopDidNotReachTerminalState,
}

impl fmt::Display for TrainingExecutionError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Preparation(error) => return write!(f, "training preparation failed: {error}"),
			Self::NativeHandoff(error) => return write!(f, "native training handoff failed: {error}"),
			Self::Executor(error) => return write!(f, "training execution failed: {error}"),
			Self::DuplicateExternalInput { value } => {
				return write!(f, "training external input {value} appears more than once");
			}
			Self::DuplicateInitDevice { device } => {
				return write!(f, "finalized init image repeats device {device}");
			}
			Self::DuplicateImageMember { device, value } => {
				return write!(
					f,
					"device {device} init image repeats logical input {value}"
				);
			}
			Self::MissingExternalInput { device, value } => {
				return write!(
					f,
					"device {device} init image requires absent external input {value}"
				);
			}
			Self::ImageMemberDTypeMismatch { device, value } => {
				return write!(
					f,
					"device {device} init member {value} has a different dtype than its training input"
				);
			}
			Self::ImageMemberSizeMismatch {
				device,
				value,
				expected,
				actual,
			} => {
				return write!(
					f,
					"device {device} init member {value} needs {} bytes, training input provides {}",
					expected.get(),
					actual.get()
				);
			}
			Self::ImageSizeUnsupported { device, bytes } => {
				return write!(
					f,
					"device {device} init image size {} does not fit this host",
					bytes.get()
				);
			}
			Self::ImageMemberOutOfBounds { device, value } => {
				return write!(
					f,
					"device {device} init member {value} lies outside its finalized image"
				);
			}
			Self::ImageMembersOverlap {
				device,
				first,
				second,
			} => {
				return write!(
					f,
					"device {device} init members {first} and {second} overlap"
				);
			}
			Self::LoopExternalTransfer { task } => {
				return write!(
					f,
					"finalized loop task {task} attempts an external data transfer"
				);
			}
			Self::ExternalOutputMapping { detail } => {
				return write!(
					f,
					"finalized checkpoint output mapping is invalid: {detail}"
				);
			}
			Self::InvalidTrainingBounds { detail } => {
				return write!(f, "compiled training bounds are invalid: {detail}");
			}
			Self::UnboundedTrainingRequiresStopControl => {
				return f.write_str("unbounded training requires an explicit graceful-stop control source");
			}
			Self::LoopDidNotReachTerminalState => {
				return f.write_str("training wait returned before the loop reached terminal completion");
			}
		}
	}
}

impl StdError for TrainingExecutionError {
	#[inline]
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Preparation(error) => return Some(error),
			Self::NativeHandoff(error) => return Some(error.as_ref()),
			Self::Executor(error) => return Some(error),
			Self::DuplicateExternalInput { .. } | Self::DuplicateInitDevice { .. } | Self::DuplicateImageMember { .. } | Self::MissingExternalInput { .. } | Self::ImageMemberDTypeMismatch { .. } | Self::ImageMemberSizeMismatch { .. } | Self::ImageSizeUnsupported { .. } | Self::ImageMemberOutOfBounds { .. } | Self::ImageMembersOverlap { .. } | Self::LoopExternalTransfer { .. } | Self::ExternalOutputMapping { .. } | Self::InvalidTrainingBounds { .. } | Self::UnboundedTrainingRequiresStopControl | Self::LoopDidNotReachTerminalState => return None,
		}
	}
}

impl From<PrepareError> for TrainingExecutionError {
	#[inline]
	fn from(error: PrepareError) -> Self { return Self::Preparation(error); }
}

impl From<ExecutorError> for TrainingExecutionError {
	#[inline]
	fn from(error: ExecutorError) -> Self { return Self::Executor(error); }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum InferenceExecutionError {
	Preparation(PrepareError),
	NativeHandoff(Box<dyn StdError + Send + Sync>),
	Executor {
		source: Box<ExecutorError>,
		failure: Box<InferenceRunFailure>,
	},
	PostExitValidation {
		source: Box<Self>,
		failure: Box<InferenceRunFailure>,
	},
	InvalidLoopIterations {
		actual: LoopIterations,
	},
	InvalidInferenceBoundary {
		detail: String,
	},
	DuplicateExternalInput {
		value: ValueId,
	},
	UnboundExternalInput {
		value: ValueId,
	},
	ExternalInputByteSizeMismatch {
		value: ValueId,
		expected: ByteCount,
		actual: ByteCount,
	},
	DuplicateInitDevice {
		device: DeviceId,
	},
	DuplicateImageMember {
		device: DeviceId,
		value: ValueId,
	},
	UnexpectedImageMember {
		device: DeviceId,
		value: ValueId,
	},
	ImageMemberDTypeMismatch {
		device: DeviceId,
		value: ValueId,
	},
	ImageMemberSizeMismatch {
		device: DeviceId,
		value: ValueId,
		expected: ByteCount,
		actual: ByteCount,
	},
	ImageSizeUnsupported {
		device: DeviceId,
		bytes: ByteCount,
	},
	ImageMemberOutOfBounds {
		device: DeviceId,
		value: ValueId,
	},
	ImageMembersOverlap {
		device: DeviceId,
		first: ValueId,
		second: ValueId,
	},
	LoopExternalTransfer {
		task: TaskId,
	},
	MissingPredictionOutput,
	DuplicatePredictionOutput {
		value: ValueId,
	},
	UnexpectedPredictionOutput {
		task: TaskId,
		value: Option<ValueId>,
	},
	PredictionOutputImagesOverlap {
		first: TaskId,
		second: TaskId,
	},
	PredictionOutputDTypeMismatch {
		expected: DType,
		actual: DType,
	},
	PredictionOutputSizeMismatch {
		expected: ByteCount,
		actual: ByteCount,
	},
	PredictionOutputSourceMismatch {
		task: TaskId,
	},
	LoopDidNotReachTerminalState,
}

impl InferenceExecutionError {
	#[must_use]
	#[inline]
	pub const fn run_failure(&self) -> Option<&InferenceRunFailure> {
		match self {
			Self::Executor { failure, .. } | Self::PostExitValidation { failure, .. } => {
				return Some(failure);
			}
			Self::Preparation(_)
			| Self::NativeHandoff(_)
			| Self::InvalidLoopIterations { .. }
			| Self::InvalidInferenceBoundary { .. }
			| Self::DuplicateExternalInput { .. }
			| Self::UnboundExternalInput { .. }
			| Self::ExternalInputByteSizeMismatch { .. }
			| Self::DuplicateInitDevice { .. }
			| Self::DuplicateImageMember { .. }
			| Self::UnexpectedImageMember { .. }
			| Self::ImageMemberDTypeMismatch { .. }
			| Self::ImageMemberSizeMismatch { .. }
			| Self::ImageSizeUnsupported { .. }
			| Self::ImageMemberOutOfBounds { .. }
			| Self::ImageMembersOverlap { .. }
			| Self::LoopExternalTransfer { .. }
			| Self::MissingPredictionOutput
			| Self::DuplicatePredictionOutput { .. }
			| Self::UnexpectedPredictionOutput { .. }
			| Self::PredictionOutputImagesOverlap { .. }
			| Self::PredictionOutputDTypeMismatch { .. }
			| Self::PredictionOutputSizeMismatch { .. }
			| Self::PredictionOutputSourceMismatch { .. }
			| Self::LoopDidNotReachTerminalState => return None,
		}
	}
}

impl fmt::Display for InferenceExecutionError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Preparation(error) => return write!(f, "inference preparation failed: {error}"),
			Self::NativeHandoff(error) => return write!(f, "native inference handoff failed: {error}"),
			Self::Executor { source, .. } => return write!(f, "inference execution failed: {source}"),
			Self::PostExitValidation { source, .. } => {
				return write!(
					f,
					"completed inference prediction validation failed: {source}"
				);
			}
			Self::InvalidLoopIterations { actual } => {
				return write!(
					f,
					"compiled inference must execute exactly one loop iteration, got {actual}"
				);
			}
			Self::InvalidInferenceBoundary { detail } => {
				return write!(f, "compiled inference boundary is invalid: {detail}");
			}
			Self::DuplicateExternalInput { value } => {
				return write!(f, "inference external input {value} appears more than once");
			}
			Self::UnboundExternalInput { value } => {
				return write!(
					f,
					"inference external input {value} is absent from every finalized init image"
				);
			}
			Self::ExternalInputByteSizeMismatch {
				value,
				expected,
				actual,
			} => {
				return write!(
					f,
					"inference external input {value} shape requires {} bytes, input provides {}",
					expected.get(),
					actual.get()
				);
			}
			Self::DuplicateInitDevice { device } => {
				return write!(f, "finalized inference init image repeats device {device}");
			}
			Self::DuplicateImageMember { device, value } => {
				return write!(
					f,
					"device {device} inference init image repeats logical input {value}"
				);
			}
			Self::UnexpectedImageMember { device, value } => {
				return write!(
					f,
					"device {device} inference init image names undeclared input {value}"
				);
			}
			Self::ImageMemberDTypeMismatch { device, value } => {
				return write!(
					f,
					"device {device} init member {value} has a different dtype than its inference input"
				);
			}
			Self::ImageMemberSizeMismatch {
				device,
				value,
				expected,
				actual,
			} => {
				return write!(
					f,
					"device {device} init member {value} needs {} bytes, inference input provides {}",
					expected.get(),
					actual.get()
				);
			}
			Self::ImageSizeUnsupported { device, bytes } => {
				return write!(
					f,
					"device {device} inference init image size {} does not fit this host",
					bytes.get()
				);
			}
			Self::ImageMemberOutOfBounds { device, value } => {
				return write!(
					f,
					"device {device} inference init member {value} lies outside its finalized image"
				);
			}
			Self::ImageMembersOverlap {
				device,
				first,
				second,
			} => {
				return write!(
					f,
					"device {device} inference init members {first} and {second} overlap"
				);
			}
			Self::LoopExternalTransfer { task } => {
				return write!(
					f,
					"finalized inference loop task {task} attempts an external data transfer"
				);
			}
			Self::MissingPredictionOutput => {
				return f.write_str("finalized inference prediction output is missing");
			}
			Self::DuplicatePredictionOutput { value } => {
				return write!(
					f,
					"finalized inference prediction {value} appears more than once"
				);
			}
			Self::UnexpectedPredictionOutput { task, value } => {
				match *value {
					Some(value) => {
						return write!(
							f,
							"finalized inference exit task {task} names unexpected logical output {value}"
						);
					}
					None => {
						return write!(f, "finalized inference has unexpected exit task {task}");
					}
				}
			}
			Self::PredictionOutputImagesOverlap { first, second } => {
				return write!(
					f,
					"finalized inference output images from tasks {first} and {second} overlap"
				);
			}
			Self::PredictionOutputDTypeMismatch { expected, actual } => {
				return write!(
					f,
					"finalized inference prediction dtype is {actual}, expected {expected}"
				);
			}
			Self::PredictionOutputSizeMismatch { expected, actual } => {
				return write!(
					f,
					"finalized inference prediction has {} bytes, expected {}",
					actual.get(),
					expected.get()
				);
			}
			Self::PredictionOutputSourceMismatch { task } => {
				return write!(
					f,
					"completed inference output task {task} has a different physical source than its finalized plan"
				);
			}
			Self::LoopDidNotReachTerminalState => {
				return f.write_str("bounded inference wait returned before the one-iteration loop reached completion");
			}
		}
	}
}

impl StdError for InferenceExecutionError {
	#[inline]
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Preparation(error) => return Some(error),
			Self::NativeHandoff(error) => return Some(error.as_ref()),
			Self::Executor { source, .. } => return Some(source),
			Self::PostExitValidation { source, .. } => return Some(source.as_ref()),
			Self::InvalidLoopIterations { .. }
			| Self::InvalidInferenceBoundary { .. }
			| Self::DuplicateExternalInput { .. }
			| Self::UnboundExternalInput { .. }
			| Self::ExternalInputByteSizeMismatch { .. }
			| Self::DuplicateInitDevice { .. }
			| Self::DuplicateImageMember { .. }
			| Self::UnexpectedImageMember { .. }
			| Self::ImageMemberDTypeMismatch { .. }
			| Self::ImageMemberSizeMismatch { .. }
			| Self::ImageSizeUnsupported { .. }
			| Self::ImageMemberOutOfBounds { .. }
			| Self::ImageMembersOverlap { .. }
			| Self::LoopExternalTransfer { .. }
			| Self::MissingPredictionOutput
			| Self::DuplicatePredictionOutput { .. }
			| Self::UnexpectedPredictionOutput { .. }
			| Self::PredictionOutputImagesOverlap { .. }
			| Self::PredictionOutputDTypeMismatch { .. }
			| Self::PredictionOutputSizeMismatch { .. }
			| Self::PredictionOutputSourceMismatch { .. }
			| Self::LoopDidNotReachTerminalState => return None,
		}
	}
}

impl From<PrepareError> for InferenceExecutionError {
	#[inline]
	fn from(error: PrepareError) -> Self { return Self::Preparation(error); }
}

#[inline]
pub fn build_inference_device_images(inference: &CompiledInference, bundle: &FinalizedBundle) -> InferenceExecutionResult<Vec<DeviceImage>> {
	validate_compiled_inference_boundary(inference)?;
	return pack_inference_device_images(inference.external_inputs(), bundle.init_images());
}

#[inline]
pub fn prepare_and_execute_local_inference<'cuda, 'hsa, A, R, Bridge>(inference: &CompiledInference, profile: &MeasuredProfile, preparer: &mut Preparer<A, R>, run: RunId, limits: InferenceExecutionLimits) -> InferenceExecutionResult<CompletedInferenceExecution>
where
	A: ArtifactProvider,
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
	R: CandidateRealizer<A::Catalog, Session = PreparedNativeSession<ValidatedCandidateSession<LocalPreparedSession<'cuda, 'hsa, Bridge, <Bridge as CrossBackendTransfer<'cuda, 'hsa>>::Resource>>>>,
{
	validate_compiled_inference_boundary(inference)?;
	let prepared_system = preparer.prepare_program(inference.program(), profile)?;
	let bundle_iterations = prepared_system.bundle().loop_iterations();
	if bundle_iterations != LoopIterations::ONE {
		return Err(InferenceExecutionError::InvalidLoopIterations {
			actual: bundle_iterations,
		});
	}
	reject_inference_loop_external_transfers(prepared_system.bundle())?;
	reject_inference_user_metrics(prepared_system.bundle())?;
	let images = build_inference_device_images(inference, prepared_system.bundle())?;
	let output_plan = map_inference_output(
		inference,
		prepared_system.bundle(),
		prepared_system.external_outputs(),
	)?;
	let (bundle, realization, _catalog, native_session) = prepared_system.into_parts();
	let (validated_session, artifacts) = native_session.into_parts();
	let native_kernels = retain_realized_native_kernels(&realization, artifacts);
	let local_session = validated_session.into_inner();
	let backend = local_session
		.into_backend(&bundle)
		.map_err(native_inference_handoff_error::<Bridge::Error>)?;

	let run_state = PreparedRun::prepare_recoverable(run, bundle, backend, limits.watchdog).map_err(inference_executor_failure)?;
	let initialized = run_state
		.initialize_recoverable(images)
		.map_err(inference_executor_failure)?;
	let loop_started = Instant::now();
	let mut running = initialized
		.start_loop_recoverable()
		.map_err(inference_executor_failure)?;
	let mut backoff = BlockingPollBackoff::new();
	loop {
		let (status, made_progress) = match running.poll_with_progress() {
			Ok(progress) => progress,
			Err(error) => return Err(inference_executor_failure(running.fail(error))),
		};
		if status == LoopStatus::Complete {
			break;
		}
		if made_progress {
			backoff.reset();
		} else {
			backoff.wait();
		}
	}
	let elapsed = loop_started.elapsed();
	let exited_loop = match running.into_exited_loop() {
		Ok(exited_loop) => exited_loop,
		Err(running) => {
			let error = ExecutorError::LifecycleInvariant {
				detail: "bounded inference wait returned before the one-iteration loop reached completion",
			};
			return Err(inference_executor_failure((*running).fail(error)));
		}
	};
	let exited = exited_loop
		.exit_recoverable()
		.map_err(inference_executor_failure)?;
	let bundle_identity = exited.bundle_identity();
	let (backend, _mailbox, external_outputs, journal) = exited.into_parts();
	let native_evidence = backend.native_evidence().clone();
	let prediction = match collect_inference_prediction(inference.output(), output_plan, external_outputs) {
		Ok(prediction) => prediction,
		Err(source) => {
			return Err(post_exit_inference_failure(
				source,
				run,
				bundle_identity,
				journal,
			));
		}
	};
	return Ok(CompletedInferenceExecution {
		run,
		bundle: bundle_identity,
		prediction,
		native_kernels,
		native_evidence,
		elapsed,
		journal,
	});
}

#[inline]
pub fn build_knn_inference_device_images(inference: &CompiledKnnInference, bundle: &FinalizedBundle) -> InferenceExecutionResult<Vec<DeviceImage>> {
	validate_compiled_knn_inference_boundary(inference)?;
	return pack_inference_device_images(inference.external_inputs(), bundle.init_images());
}

#[inline]
pub fn prepare_and_execute_local_knn_inference<'cuda, 'hsa, A, R, Bridge>(inference: &CompiledKnnInference, profile: &MeasuredProfile, preparer: &mut Preparer<A, R>, run: RunId, limits: InferenceExecutionLimits) -> InferenceExecutionResult<CompletedKnnInferenceExecution>
where
	A: ArtifactProvider,
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
	R: CandidateRealizer<A::Catalog, Session = PreparedNativeSession<ValidatedCandidateSession<LocalPreparedSession<'cuda, 'hsa, Bridge, <Bridge as CrossBackendTransfer<'cuda, 'hsa>>::Resource>>>>,
{
	validate_compiled_knn_inference_boundary(inference)?;
	let prepared_system = preparer.prepare_program(inference.program(), profile)?;
	let bundle_iterations = prepared_system.bundle().loop_iterations();
	if bundle_iterations != LoopIterations::ONE {
		return Err(InferenceExecutionError::InvalidLoopIterations {
			actual: bundle_iterations,
		});
	}
	reject_inference_loop_external_transfers(prepared_system.bundle())?;
	reject_inference_user_metrics(prepared_system.bundle())?;
	let images = build_knn_inference_device_images(inference, prepared_system.bundle())?;
	let output_plan = map_knn_inference_outputs(
		inference,
		prepared_system.bundle(),
		prepared_system.external_outputs(),
	)?;
	let (bundle, _realization, _catalog, native_session) = prepared_system.into_parts();
	let (validated_session, _artifacts) = native_session.into_parts();
	let local_session = validated_session.into_inner();
	let backend = local_session
		.into_backend(&bundle)
		.map_err(native_inference_handoff_error::<Bridge::Error>)?;

	let run_state = PreparedRun::prepare_recoverable(run, bundle, backend, limits.watchdog).map_err(inference_executor_failure)?;
	let initialized = run_state
		.initialize_recoverable(images)
		.map_err(inference_executor_failure)?;
	let loop_started = Instant::now();
	let mut running = initialized
		.start_loop_recoverable()
		.map_err(inference_executor_failure)?;
	let mut backoff = BlockingPollBackoff::new();
	loop {
		let (status, made_progress) = match running.poll_with_progress() {
			Ok(progress) => progress,
			Err(error) => return Err(inference_executor_failure(running.fail(error))),
		};
		if status == LoopStatus::Complete {
			break;
		}
		if made_progress {
			backoff.reset();
		} else {
			backoff.wait();
		}
	}
	let elapsed = loop_started.elapsed();
	let exited_loop = match running.into_exited_loop() {
		Ok(exited_loop) => exited_loop,
		Err(running) => {
			let error = ExecutorError::LifecycleInvariant {
				detail: "bounded KNN inference wait returned before the one-iteration loop reached completion",
			};
			return Err(inference_executor_failure((*running).fail(error)));
		}
	};
	let exited = exited_loop
		.exit_recoverable()
		.map_err(inference_executor_failure)?;
	let bundle_identity = exited.bundle_identity();
	let (backend, _mailbox, external_outputs, journal) = exited.into_parts();
	let native_evidence = backend.native_evidence().clone();
	let predictions = match collect_knn_inference_predictions(inference.outputs(), &output_plan, external_outputs) {
		Ok(predictions) => predictions,
		Err(source) => {
			return Err(post_exit_inference_failure(
				source,
				run,
				bundle_identity,
				journal,
			));
		}
	};
	return Ok(CompletedKnnInferenceExecution {
		run,
		bundle: bundle_identity,
		predictions,
		native_evidence,
		elapsed,
		journal,
	});
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FinalizedInferenceOutput {
	task: TaskId,
	logical: ValueId,
	source: ResolvedValueLocation,
	bytes: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InferenceInputImage<'a> {
	value: ValueId,
	dtype: DType,
	bytes: &'a [u8],
}

fn validate_compiled_inference_boundary(inference: &CompiledInference) -> InferenceExecutionResult<()> {
	let iterations = inference.program().iterations();
	if iterations != LoopIterations::ONE {
		return Err(InferenceExecutionError::InvalidLoopIterations { actual: iterations });
	}
	if !inference.program().metrics().is_empty() {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: "target-free inference cannot declare metric emissions".to_owned(),
		});
	}

	let graph = inference.graph();
	graph.validate().map_err(|error| {
		return InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("calculation graph is invalid: {error}"),
		};
	})?;
	let graph_inputs = graph
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_input)
		.map(|tensor| return tensor.id)
		.collect::<BTreeSet<_>>();
	let mut declared_inputs = BTreeSet::new();
	let mut declared_roles = Vec::new();
	for input in inference.external_inputs() {
		if !inference_input_role_is_allowed(input.role()) {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"external input {} has a training-only semantic role",
					input.value()
				),
			});
		}
		if declared_roles.contains(&input.role()) {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"external input semantic role {:?} appears more than once",
					input.role()
				),
			});
		}
		declared_roles.push(input.role());
		if !declared_inputs.insert(input.value()) {
			return Err(InferenceExecutionError::DuplicateExternalInput {
				value: input.value(),
			});
		}
		let expected = input.shape().bytes(input.dtype()).map_err(|error| {
			return InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"external input {} has an invalid typed shape: {error}",
					input.value()
				),
			};
		})?;
		let actual = host_byte_count(input.bytes().len());
		if actual != expected {
			return Err(InferenceExecutionError::ExternalInputByteSizeMismatch {
				value: input.value(),
				expected,
				actual,
			});
		}
		let tensor = graph
			.tensors
			.iter()
			.find(|tensor| return tensor.id == input.value())
			.ok_or_else(|| {
				return InferenceExecutionError::InvalidInferenceBoundary {
					detail: format!(
						"declared external input {} is absent from the graph",
						input.value()
					),
				};
			})?;
		if !tensor.external_input || tensor.dtype != input.dtype() || tensor.shape != *input.shape() {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"declared external input {} differs from its graph tensor dtype, shape, or boundary role",
					input.value()
				),
			});
		}
		validate_canonical_boundary_tensor(tensor, "external input")?;
	}
	if declared_inputs != graph_inputs {
		let missing = graph_inputs
			.difference(&declared_inputs)
			.copied()
			.collect::<Vec<_>>();
		let unexpected = declared_inputs
			.difference(&graph_inputs)
			.copied()
			.collect::<Vec<_>>();
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("external input boundary differs (missing {missing:?}, unexpected {unexpected:?})"),
		});
	}

	let graph_outputs = graph
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_output)
		.collect::<Vec<_>>();
	if let Some(value) = graph_inputs.iter().find(|value| {
		return graph_outputs
			.iter()
			.any(|tensor| return tensor.id == **value);
	}) {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("value {value} is both an inference input and prediction output"),
		});
	}
	let [output_tensor] = graph_outputs.as_slice() else {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!(
				"inference graph has {} external outputs; exactly one prediction is required",
				graph_outputs.len()
			),
		});
	};
	let output = inference.output();
	if output.dtype() != DType::F32 || output_tensor.dtype != DType::F32 {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: "inference predictions must use F32 dtype".to_owned(),
		});
	}
	validate_canonical_boundary_tensor(output_tensor, "prediction output")?;
	if !graph
		.nodes
		.iter()
		.any(|node| return node.kernel.outputs.contains(&output.value()))
	{
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!(
				"prediction output {} has no calculation producer",
				output.value()
			),
		});
	}
	if output_tensor.id != output.value() || output_tensor.dtype != output.dtype() || output_tensor.shape != *output.shape() {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: "prediction contract differs from the sole external graph tensor".to_owned(),
		});
	}
	let (expected_width, expected_kind) = match inference.task() {
		InferenceTask::TokenLogits { vocabulary } => (vocabulary, InferencePredictionKind::TokenLogits),
		InferenceTask::BayesProbabilities { width } => (width, InferencePredictionKind::BayesProbabilities),
		InferenceTask::Dense(DenseTask::BinaryClassification { .. }) => (1u64, InferencePredictionKind::BinaryProbability),
		InferenceTask::Dense(DenseTask::MulticlassClassification { class_count, .. }) => {
			(
				u64::try_from(class_count).map_err(|_error| {
					return InferenceExecutionError::InvalidInferenceBoundary {
						detail: "multiclass output width does not fit u64".to_owned(),
					};
				})?,
				InferencePredictionKind::MulticlassProbabilities,
			)
		}
		InferenceTask::Dense(DenseTask::ScalarRegression { .. }) => (1u64, InferencePredictionKind::Regression),
		InferenceTask::Dense(DenseTask::MultiTargetBinaryClassification { target_count, .. }) => {
			(
				u64::try_from(target_count).map_err(|_error| {
					return InferenceExecutionError::InvalidInferenceBoundary {
						detail: "multi-target binary output width does not fit u64".to_owned(),
					};
				})?,
				InferencePredictionKind::MultiTargetBinaryProbabilities,
			)
		}
		InferenceTask::Dense(DenseTask::JointMulticlassClassification { target_count, .. }) => {
			(
				u64::try_from(target_count).map_err(|_error| {
					return InferenceExecutionError::InvalidInferenceBoundary {
						detail: "joint target output width does not fit u64".to_owned(),
					};
				})?,
				InferencePredictionKind::JointTargetProbabilities,
			)
		}
		InferenceTask::Dense(DenseTask::MultiTargetRegression { target_count, .. }) => {
			(
				u64::try_from(target_count).map_err(|_error| {
					return InferenceExecutionError::InvalidInferenceBoundary {
						detail: "multi-target regression output width does not fit u64".to_owned(),
					};
				})?,
				InferencePredictionKind::MultiTargetRegression,
			)
		}
	};
	if output.kind() != expected_kind || output.shape().extents() != [inference.rows(), expected_width] {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: "prediction kind or matrix shape differs from the compiled inference task".to_owned(),
		});
	}
	return Ok(());
}

fn validate_compiled_knn_inference_boundary(inference: &CompiledKnnInference) -> InferenceExecutionResult<()> {
	let iterations = inference.program().iterations();
	if iterations != LoopIterations::ONE {
		return Err(InferenceExecutionError::InvalidLoopIterations { actual: iterations });
	}
	if !inference.program().metrics().is_empty() {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: "KNN inference cannot declare metric emissions".to_owned(),
		});
	}
	let graph = inference.graph();
	graph.validate().map_err(|error| {
		return InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("KNN calculation graph is invalid: {error}"),
		};
	})?;
	let graph_inputs = graph
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_input)
		.map(|tensor| return tensor.id)
		.collect::<BTreeSet<_>>();
	let mut declared_inputs = BTreeSet::new();
	let mut declared_roles = Vec::new();
	for input in inference.external_inputs() {
		if !inference_input_role_is_allowed(input.role()) {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"KNN external input {} has an invalid semantic role",
					input.value()
				),
			});
		}
		if declared_roles.contains(&input.role()) {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"KNN external input semantic role {:?} appears more than once",
					input.role()
				),
			});
		}
		declared_roles.push(input.role());
		if !declared_inputs.insert(input.value()) {
			return Err(InferenceExecutionError::DuplicateExternalInput {
				value: input.value(),
			});
		}
		let expected = input.shape().bytes(input.dtype()).map_err(|error| {
			return InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"KNN external input {} has an invalid typed shape: {error}",
					input.value()
				),
			};
		})?;
		let actual = host_byte_count(input.bytes().len());
		if actual != expected {
			return Err(InferenceExecutionError::ExternalInputByteSizeMismatch {
				value: input.value(),
				expected,
				actual,
			});
		}
		let tensor = graph
			.tensors
			.iter()
			.find(|tensor| return tensor.id == input.value())
			.ok_or_else(|| {
				return InferenceExecutionError::InvalidInferenceBoundary {
					detail: format!(
						"declared KNN external input {} is absent from the graph",
						input.value()
					),
				};
			})?;
		if !tensor.external_input || tensor.dtype != input.dtype() || tensor.shape != *input.shape() {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"declared KNN external input {} differs from its graph tensor",
					input.value()
				),
			});
		}
		validate_canonical_boundary_tensor(tensor, "KNN external input")?;
	}
	if declared_inputs != graph_inputs {
		let missing = graph_inputs
			.difference(&declared_inputs)
			.copied()
			.collect::<Vec<_>>();
		let unexpected = declared_inputs
			.difference(&graph_inputs)
			.copied()
			.collect::<Vec<_>>();
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("KNN external input boundary differs (missing {missing:?}, unexpected {unexpected:?})"),
		});
	}
	if inference.outputs().is_empty() {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: "KNN inference has no declared outputs".to_owned(),
		});
	}
	let mut output_values = BTreeSet::new();
	let mut output_sources = BTreeSet::new();
	for output in inference.outputs() {
		if !output_values.insert(output.value()) || !output_sources.insert(output.source_vector()) {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: "KNN output values and target source identities must be unique".to_owned(),
			});
		}
		if graph_inputs.contains(&output.value()) {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"value {} is both a KNN input and prediction output",
					output.value()
				),
			});
		}
		let expected_dtype = match output.kind() {
			KnnInferencePredictionKind::NumericMean => DType::F32,
			KnnInferencePredictionKind::DiscreteMode => DType::I32,
		};
		if output.dtype() != expected_dtype || output.shape().extents() != [inference.rows(), 1] {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"KNN output {} has a dtype, aggregation kind, or shape mismatch",
					output.value()
				),
			});
		}
		let tensor = graph
			.tensors
			.iter()
			.find(|tensor| return tensor.id == output.value())
			.ok_or_else(|| {
				return InferenceExecutionError::InvalidInferenceBoundary {
					detail: format!("KNN output {} is absent from the graph", output.value()),
				};
			})?;
		if !tensor.external_output || tensor.external_input || tensor.dtype != output.dtype() || tensor.shape != *output.shape() {
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!(
					"KNN output {} differs from its graph tensor",
					output.value()
				),
			});
		}
		validate_canonical_boundary_tensor(tensor, "KNN prediction output")?;
		if !graph
			.nodes
			.iter()
			.any(|node| return node.kernel.outputs.contains(&output.value()))
		{
			return Err(InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!("KNN output {} has no calculation producer", output.value()),
			});
		}
	}
	let graph_outputs = graph
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_output)
		.map(|tensor| return tensor.id)
		.collect::<BTreeSet<_>>();
	if graph_outputs != output_values {
		let missing = graph_outputs
			.difference(&output_values)
			.copied()
			.collect::<Vec<_>>();
		let unexpected = output_values
			.difference(&graph_outputs)
			.copied()
			.collect::<Vec<_>>();
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("KNN output boundary differs (missing {missing:?}, unexpected {unexpected:?})"),
		});
	}
	return Ok(());
}

const fn inference_input_role_is_allowed(role: InferenceInputRole) -> bool {
	match role {
		InferenceInputRole::Feature { .. }
		| InferenceInputRole::FeatureNormalizationMask
		| InferenceInputRole::DataNormalizationMean
		| InferenceInputRole::DataNormalizationVariance
		| InferenceInputRole::DataNormalizationMinimum
		| InferenceInputRole::DataNormalizationMaximum
		| InferenceInputRole::LayerWeight { .. }
		| InferenceInputRole::LayerBias { .. }
		| InferenceInputRole::LayerPRelu { .. }
		| InferenceInputRole::EmbeddingTable { .. }
		| InferenceInputRole::AttentionQuery { .. }
		| InferenceInputRole::AttentionKey { .. }
		| InferenceInputRole::AttentionValue { .. }
		| InferenceInputRole::AttentionOutput { .. }
		| InferenceInputRole::RnnInputWeight { .. }
		| InferenceInputRole::RnnRecurrentWeight { .. }
		| InferenceInputRole::RnnBias { .. }
		| InferenceInputRole::GruResetInputWeight { .. }
		| InferenceInputRole::GruResetRecurrentWeight { .. }
		| InferenceInputRole::GruResetBias { .. }
		| InferenceInputRole::GruUpdateInputWeight { .. }
		| InferenceInputRole::GruUpdateRecurrentWeight { .. }
		| InferenceInputRole::GruUpdateBias { .. }
		| InferenceInputRole::GruCandidateInputWeight { .. }
		| InferenceInputRole::GruCandidateRecurrentWeight { .. }
		| InferenceInputRole::GruCandidateBias { .. }
		| InferenceInputRole::LstmInputGateInputWeight { .. }
		| InferenceInputRole::LstmInputGateRecurrentWeight { .. }
		| InferenceInputRole::LstmInputGateBias { .. }
		| InferenceInputRole::LstmForgetGateInputWeight { .. }
		| InferenceInputRole::LstmForgetGateRecurrentWeight { .. }
		| InferenceInputRole::LstmForgetGateBias { .. }
		| InferenceInputRole::LstmOutputGateInputWeight { .. }
		| InferenceInputRole::LstmOutputGateRecurrentWeight { .. }
		| InferenceInputRole::LstmOutputGateBias { .. }
		| InferenceInputRole::LstmCandidateInputWeight { .. }
		| InferenceInputRole::LstmCandidateRecurrentWeight { .. }
		| InferenceInputRole::LstmCandidateBias { .. }
		| InferenceInputRole::ConvolutionWindowIndices { .. }
		| InferenceInputRole::ConvolutionWeight { .. }
		| InferenceInputRole::ConvolutionBias { .. }
		| InferenceInputRole::ConvolutionPRelu { .. }
		| InferenceInputRole::PoolWindowIndices { .. }
		| InferenceInputRole::PoolWinnerBases { .. }
		| InferenceInputRole::KMeansCentroids { .. }
		| InferenceInputRole::TreeSplitFeatures { .. }
		| InferenceInputRole::TreeSplitThresholds { .. }
		| InferenceInputRole::TreeLeafValues { .. }
		| InferenceInputRole::ResidualProjectionWeight { .. }
		| InferenceInputRole::ResidualBranchPRelu { .. }
		| InferenceInputRole::ResidualOutputPRelu { .. }
		| InferenceInputRole::Temperature
		| InferenceInputRole::KnnReferenceFeatures
		| InferenceInputRole::KnnReferenceValues { .. }
		| InferenceInputRole::KnnReferenceKnown { .. }
		| InferenceInputRole::BayesQueryParents { .. }
		| InferenceInputRole::BayesReferenceParents { .. }
		| InferenceInputRole::BayesReferenceChild { .. }
		| InferenceInputRole::BayesParentMultipliers { .. }
		| InferenceInputRole::BayesParentCardinalities { .. }
		| InferenceInputRole::BayesConcatenationLeftIndices { .. }
		| InferenceInputRole::BayesConcatenationRightIndices { .. }
		| InferenceInputRole::BayesConcatenationSelectLeft { .. }
		| InferenceInputRole::GgufTokenIds
		| InferenceInputRole::GgufTensor { .. }
		| InferenceInputRole::GgufRopePartnerIndices
		| InferenceInputRole::GgufRopeCosines
		| InferenceInputRole::GgufRopeSignedSines => return true,
	}
}

fn validate_canonical_boundary_tensor(tensor: &recipe_language::Tensor, boundary: &'static str) -> InferenceExecutionResult<()> {
	let expected_layout = TensorLayout::contiguous(&tensor.shape, ContiguousOrder::RowMajor).map_err(|error| {
		return InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("{boundary} {} has an invalid shape: {error}", tensor.id),
		};
	})?;
	if tensor.layout != expected_layout {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!(
				"{boundary} {} is not canonical contiguous row-major",
				tensor.id
			),
		});
	}
	let expected_storage = tensor.shape.bytes(tensor.dtype).map_err(|error| {
		return InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!(
				"{boundary} {} has an invalid typed shape: {error}",
				tensor.id
			),
		};
	})?;
	if tensor.storage_bytes != expected_storage {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!(
				"{boundary} {} storage is {} bytes, expected exactly {}",
				tensor.id,
				tensor.storage_bytes.get(),
				expected_storage.get()
			),
		});
	}
	return Ok(());
}

fn pack_inference_device_images(inputs: &[InferenceExternalInput], manifests: &[InitDataImage]) -> InferenceExecutionResult<Vec<DeviceImage>> {
	let inputs = inputs
		.iter()
		.map(|input| {
			return InferenceInputImage {
				value: input.value(),
				dtype: input.dtype(),
				bytes: input.bytes(),
			};
		})
		.collect::<Vec<_>>();
	return pack_inference_input_images(&inputs, manifests);
}

fn pack_inference_input_images(inputs: &[InferenceInputImage<'_>], manifests: &[InitDataImage]) -> InferenceExecutionResult<Vec<DeviceImage>> {
	let mut by_value = BTreeMap::new();
	for input in inputs {
		if by_value.insert(input.value, input).is_some() {
			return Err(InferenceExecutionError::DuplicateExternalInput { value: input.value });
		}
	}
	let mut bound = BTreeSet::new();
	let mut devices = BTreeSet::new();
	let mut result = Vec::with_capacity(manifests.len());
	for manifest in manifests {
		if !devices.insert(manifest.device) {
			return Err(InferenceExecutionError::DuplicateInitDevice {
				device: manifest.device,
			});
		}
		let image_len = usize::try_from(manifest.bytes.get()).map_err(|_error| {
			return InferenceExecutionError::ImageSizeUnsupported {
				device: manifest.device,
				bytes: manifest.bytes,
			};
		})?;
		let mut bytes = vec![0u8; image_len];
		let mut members = manifest.members.iter().collect::<Vec<_>>();
		members.sort_by_key(|member| return (member.image_offset, member.logical));
		let mut logical = BTreeSet::new();
		let mut previous: Option<(ValueId, u64)> = None;
		for member in members {
			if !logical.insert(member.logical) {
				return Err(InferenceExecutionError::DuplicateImageMember {
					device: manifest.device,
					value: member.logical,
				});
			}
			let input = by_value
				.get(&member.logical)
				.copied()
				.ok_or(InferenceExecutionError::UnexpectedImageMember {
					device: manifest.device,
					value: member.logical,
				})?;
			bound.insert(member.logical);
			if input.dtype != member.dtype {
				return Err(InferenceExecutionError::ImageMemberDTypeMismatch {
					device: manifest.device,
					value: member.logical,
				});
			}
			let actual = host_byte_count(input.bytes.len());
			if actual != member.bytes {
				return Err(InferenceExecutionError::ImageMemberSizeMismatch {
					device: manifest.device,
					value: member.logical,
					expected: member.bytes,
					actual,
				});
			}
			let start = member.image_offset.get();
			let end = start
				.checked_add(member.bytes.get())
				.ok_or(InferenceExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				})?;
			if end > manifest.bytes.get() {
				return Err(InferenceExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				});
			}
			if let Some((prior, prior_end)) = previous
				&& start < prior_end
			{
				return Err(InferenceExecutionError::ImageMembersOverlap {
					device: manifest.device,
					first: prior,
					second: member.logical,
				});
			}
			let start = usize::try_from(start).map_err(|_error| {
				return InferenceExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				};
			})?;
			let end = usize::try_from(end).map_err(|_error| {
				return InferenceExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				};
			})?;
			let destination = bytes
				.get_mut(start..end)
				.ok_or(InferenceExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				})?;
			destination.copy_from_slice(input.bytes);
			previous = Some((member.logical, u64::try_from(end).unwrap_or(u64::MAX)));
		}
		result.push(DeviceImage::new(manifest.device, manifest.image, bytes));
	}
	for value in by_value.keys().copied() {
		if !bound.contains(&value) {
			return Err(InferenceExecutionError::UnboundExternalInput { value });
		}
	}
	result.sort_by_key(|image| return image.device);
	return Ok(result);
}

fn host_byte_count(length: usize) -> ByteCount { return ByteCount::new(u64::try_from(length).unwrap_or(u64::MAX)); }

#[inline]
pub fn build_training_device_images(training: &CompiledTraining, bundle: &FinalizedBundle) -> TrainingExecutionResult<Vec<DeviceImage>> { return pack_device_images(training.external_inputs(), bundle.init_images()); }

pub fn prepare_and_execute_local_training_controlled<'cuda, 'hsa, A, R, Bridge>(training: &CompiledTraining, profile: &MeasuredProfile, preparer: &mut Preparer<A, R>, run: RunId, limits: TrainingExecutionLimits, mut observer: Option<&mut TrainingMetricObserver>, control: TrainingExecutionControl<'_>) -> TrainingExecutionResult<CompletedTrainingExecution>
where
	A: ArtifactProvider,
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
	R: CandidateRealizer<A::Catalog, Session = PreparedNativeSession<ValidatedCandidateSession<LocalPreparedSession<'cuda, 'hsa, Bridge, <Bridge as CrossBackendTransfer<'cuda, 'hsa>>::Resource>>>>,
{
	validate_training_execution_control(training.program().iterations(), control)?;
	let prepared_system = preparer.prepare_program(training.program(), profile)?;
	reject_loop_external_transfers(prepared_system.bundle())?;
	let planned_training_evidence = PlannedTrainingExecutionEvidence::new(training, profile, prepared_system.bundle());
	let images = build_training_device_images(training, prepared_system.bundle())?;
	let external_output_tasks = map_external_output_tasks(
		training,
		prepared_system.bundle(),
		prepared_system.external_outputs(),
	)?;
	let mut metrics = user_metric_slots(prepared_system.bundle())
		.into_iter()
		.map(|(slot, metric)| {
			return FinalTrainingMetric {
				slot,
				metric,
				sample: None,
			};
		})
		.collect::<Vec<_>>();
	let (bundle, realization, _catalog, native_session) = prepared_system.into_parts();
	let (validated_session, artifacts) = native_session.into_parts();
	let native_kernels = retain_realized_native_kernels(&realization, artifacts);
	let local_session = validated_session.into_inner();
	let backend = local_session
		.into_backend(&bundle)
		.map_err(native_handoff_error::<Bridge::Error>)?;

	let initialized = PreparedRun::prepare(run, bundle, backend, limits.watchdog)?.initialize(images)?;
	let mut running = initialized.start_loop()?;
	let mut backoff = BlockingPollBackoff::new();
	loop {
		let (status, made_progress) = running.poll_with_progress_or_stop(|| return control.stop_requested())?;
		drain_user_metrics(
			&mut metrics,
			|slot| return running.try_take_metric(slot),
			observer.as_deref_mut(),
		);
		if status == LoopStatus::Complete {
			break;
		}
		if made_progress {
			backoff.reset();
		} else {
			backoff.wait();
		}
	}
	let mut exited_loop = running
		.into_exited_loop()
		.map_err(|_running| return TrainingExecutionError::LoopDidNotReachTerminalState)?;
	drain_user_metrics(
		&mut metrics,
		|slot| return exited_loop.try_take_metric(slot),
		observer.as_deref_mut(),
	);
	let exited = exited_loop.exit()?;
	let bundle_identity = exited.bundle_identity();
	let (backend, mut mailbox, mut external_outputs, journal) = exited.into_parts();
	let native_evidence = backend.native_evidence().clone();
	external_outputs.sort_by_key(|image| return image.task);
	let external_output_values = external_outputs
		.iter()
		.map(|image| {
			return external_output_tasks
				.get(&image.task)
				.copied()
				.ok_or_else(|| {
					return TrainingExecutionError::ExternalOutputMapping {
						detail: format!("completed exit task {} has no logical tensor", image.task),
					};
				});
		})
		.collect::<TrainingExecutionResult<Vec<_>>>()?;
	drain_user_metrics(&mut metrics, |slot| return mailbox.try_take(slot), observer);
	let training_evidence = planned_training_evidence.complete(&journal);
	return Ok(CompletedTrainingExecution {
		run,
		bundle: bundle_identity,
		external_outputs,
		external_output_values,
		metrics,
		native_kernels,
		native_evidence,
		training_evidence,
		journal,
	});
}

#[derive(Debug)]
struct PlannedTrainingExecutionEvidence {
	bounds: TrainingBounds,
	logical_updates_per_epoch: u64,
	optimizer_parameter_kernels: usize,
	optimizer_parameter_tasks: BTreeSet<TaskId>,
	loop_calculation_tasks: usize,
	non_gpu_calculation_tasks: BTreeSet<TaskId>,
}

impl PlannedTrainingExecutionEvidence {
	fn new(training: &CompiledTraining, profile: &MeasuredProfile, bundle: &FinalizedBundle) -> Self {
		let optimizer = training.outputs().optimizer_progress;
		let mut update_kernels = BTreeSet::new();
		training.outputs().visit_parameter_states(|state| {
			update_kernels.insert(state.update_kernel);
		});
		let optimizer_parameter_tasks = bundle
			.tasks()
			.iter()
			.filter_map(|task| {
				match &task.kind {
					TaskKind::Calculation(calculation)
						if task.phase == RunPhase::Loop
							&& bundle
								.artifact_build(calculation.artifact)
								.is_some_and(|build| {
									return update_kernels.contains(&build.source_kernel);
								}) =>
					{
						return Some(task.id);
					}
					TaskKind::Calculation(_) | TaskKind::Transfer(_) | TaskKind::Metric(_) => return None,
				}
			})
			.collect();
		let non_gpu_calculation_tasks = bundle
			.tasks()
			.iter()
			.filter_map(|task| {
				match &task.kind {
					TaskKind::Calculation(calculation)
						if profile
							.topology
							.device(calculation.device)
							.is_none_or(|device| return device.kind != DeviceKind::GpuMemory) =>
					{
						return Some(task.id);
					}
					TaskKind::Calculation(_) | TaskKind::Transfer(_) | TaskKind::Metric(_) => return None,
				}
			})
			.collect();
		let loop_calculation_tasks = bundle
			.tasks()
			.iter()
			.filter(|task| return task.phase == RunPhase::Loop && matches!(task.kind, TaskKind::Calculation(_)))
			.count();
		return Self {
			bounds: training.bounds(),
			logical_updates_per_epoch: optimizer.map_or(1, |progress| return progress.accepted_updates_per_epoch),
			optimizer_parameter_kernels: update_kernels.len(),
			optimizer_parameter_tasks,
			loop_calculation_tasks,
			non_gpu_calculation_tasks,
		};
	}

	fn complete(self, journal: &RunJournal) -> TrainingExecutionEvidence {
		let optimizer_parameter_submissions = journal
			.physical_calls()
			.iter()
			.filter(|call| matches!(call, PhysicalCall::SubmitCalculation { task } if self.optimizer_parameter_tasks.contains(task)))
			.count();
		let non_gpu_calculation_submissions = journal
			.physical_calls()
			.iter()
			.filter(|call| matches!(call, PhysicalCall::SubmitCalculation { task } if self.non_gpu_calculation_tasks.contains(task)))
			.count();
		let loop_iterations_started = journal
			.logical_events()
			.iter()
			.filter(|event| matches!(event, LogicalEvent::LoopIterationStarted { .. }))
			.count();
		let loop_iterations_completed = journal
			.logical_events()
			.iter()
			.filter(|event| matches!(event, LogicalEvent::LoopIterationCompleted { .. }))
			.count();
		let summary = journal.summary();
		return TrainingExecutionEvidence {
			bounds: self.bounds,
			logical_updates_per_epoch: self.logical_updates_per_epoch,
			optimizer_parameter_kernels: self.optimizer_parameter_kernels,
			optimizer_parameter_tasks: self.optimizer_parameter_tasks.len(),
			optimizer_parameter_submissions,
			loop_iterations_started,
			loop_iterations_completed,
			loop_calculation_tasks: self.loop_calculation_tasks,
			non_gpu_calculation_tasks: self.non_gpu_calculation_tasks.len(),
			non_gpu_calculation_submissions,
			logical_events_compacted: summary.logical_events_compacted,
			physical_calls_compacted: summary.physical_calls_compacted,
		};
	}
}

fn map_external_output_tasks(training: &CompiledTraining, bundle: &FinalizedBundle, planned_outputs: impl IntoIterator<Item = (TaskId, ValueId, DeviceId, ValueId)>) -> TrainingExecutionResult<BTreeMap<TaskId, ValueId>> {
	let expected = training
		.graph()
		.tensors
		.iter()
		.filter(|tensor| return tensor.external_output)
		.map(|tensor| return tensor.id)
		.collect::<BTreeSet<_>>();
	let mut mapped = BTreeMap::new();
	let mut seen = BTreeSet::new();
	for (task, logical, device, physical) in planned_outputs {
		if !expected.contains(&logical) {
			return Err(TrainingExecutionError::ExternalOutputMapping {
				detail: format!("planned exit task {task} names non-checkpoint tensor {logical}"),
			});
		}
		if mapped.insert(task, logical).is_some() {
			return Err(TrainingExecutionError::ExternalOutputMapping {
				detail: format!("planned exit task {task} appears more than once"),
			});
		}
		if !seen.insert(logical) {
			return Err(TrainingExecutionError::ExternalOutputMapping {
				detail: format!("logical checkpoint tensor {logical} has more than one exit task"),
			});
		}
		let finalized = bundle
			.tasks()
			.iter()
			.find(|candidate| return candidate.id == task)
			.ok_or_else(|| {
				return TrainingExecutionError::ExternalOutputMapping {
					detail: format!("planned exit task {task} is absent from the finalized bundle"),
				};
			})?;
		if finalized.phase != RunPhase::Exit
			|| !matches!(
				&finalized.kind,
				TaskKind::Transfer(recipe_core::TransferTask {
					destination: TransferEndpoint::External,
					..
				})
			) {
			return Err(TrainingExecutionError::ExternalOutputMapping {
				detail: format!("planned output task {task} is not a finalized exit egress"),
			});
		}
		let endpoints = bundle.transfer_endpoints(task).ok_or_else(|| {
			return TrainingExecutionError::ExternalOutputMapping {
				detail: format!("exit task {task} has no finalized endpoints"),
			};
		})?;
		let ResolvedTransferEndpoint::Device(source) = endpoints.source else {
			return Err(TrainingExecutionError::ExternalOutputMapping {
				detail: format!("exit task {task} has no physical device source"),
			});
		};
		if source.device != device || source.value != physical {
			return Err(TrainingExecutionError::ExternalOutputMapping {
				detail: format!(
					"exit task {task} planned source {physical} on device {device}, finalized as {} on device {}",
					source.value, source.device
				),
			});
		}
	}
	let finalized_tasks = bundle
		.tasks()
		.iter()
		.filter(|task| {
			return task.phase == RunPhase::Exit
				&& matches!(
					&task.kind,
					TaskKind::Transfer(recipe_core::TransferTask {
						destination: TransferEndpoint::External,
						..
					})
				);
		})
		.map(|task| return task.id)
		.collect::<BTreeSet<_>>();
	let planned_tasks = mapped.keys().copied().collect::<BTreeSet<_>>();
	if planned_tasks != finalized_tasks {
		let missing = finalized_tasks
			.difference(&planned_tasks)
			.copied()
			.collect::<Vec<_>>();
		let unexpected = planned_tasks
			.difference(&finalized_tasks)
			.copied()
			.collect::<Vec<_>>();
		return Err(TrainingExecutionError::ExternalOutputMapping {
			detail: format!("exit-task boundary differs (missing {missing:?}, unexpected {unexpected:?})"),
		});
	}
	if seen != expected {
		let missing = expected.difference(&seen).copied().collect::<Vec<_>>();
		let unexpected = seen.difference(&expected).copied().collect::<Vec<_>>();
		return Err(TrainingExecutionError::ExternalOutputMapping {
			detail: format!("checkpoint boundary differs (missing {missing:?}, unexpected {unexpected:?})"),
		});
	}
	return Ok(mapped);
}

fn map_inference_output(inference: &CompiledInference, bundle: &FinalizedBundle, planned_outputs: impl IntoIterator<Item = (TaskId, ValueId, DeviceId, ValueId)>) -> InferenceExecutionResult<FinalizedInferenceOutput> {
	let output = inference.output();
	let expected_bytes = output.shape().bytes(output.dtype()).map_err(|error| {
		return InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("prediction contract has an invalid typed shape: {error}"),
		};
	})?;
	let mut descriptors = Vec::new();
	let mut tasks = BTreeSet::new();
	let mut logical_values = BTreeSet::new();
	for (task, logical, device, physical) in planned_outputs {
		if !tasks.insert(task) {
			return Err(InferenceExecutionError::DuplicatePredictionOutput { value: logical });
		}
		if !logical_values.insert(logical) {
			return Err(InferenceExecutionError::DuplicatePredictionOutput { value: logical });
		}
		if logical != output.value() {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		}
		let finalized = bundle
			.tasks()
			.iter()
			.find(|candidate| return candidate.id == task)
			.ok_or(InferenceExecutionError::MissingPredictionOutput)?;
		let TaskKind::Transfer(transfer) = &finalized.kind else {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		};
		if finalized.phase != RunPhase::Exit || transfer.destination != TransferEndpoint::External {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		}
		let endpoints = bundle
			.transfer_endpoints(task)
			.ok_or(InferenceExecutionError::MissingPredictionOutput)?;
		let ResolvedTransferEndpoint::Device(source) = endpoints.source else {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		};
		if source.device != device || source.value != physical {
			return Err(InferenceExecutionError::PredictionOutputSourceMismatch { task });
		}
		descriptors.push(FinalizedInferenceOutput {
			task,
			logical,
			source,
			bytes: transfer.bytes,
		});
	}

	for left_index in 0..descriptors.len() {
		for right in &descriptors[left_index + 1..] {
			let left = descriptors[left_index];
			if output_locations_overlap(left.source, right.source) {
				return Err(InferenceExecutionError::PredictionOutputImagesOverlap {
					first: left.task,
					second: right.task,
				});
			}
		}
	}

	let finalized_tasks = bundle
		.tasks()
		.iter()
		.filter(|task| {
			return task.phase == RunPhase::Exit
				&& matches!( &task.kind,
					TaskKind::Transfer(transfer) if transfer.destination == TransferEndpoint::External );
		})
		.map(|task| return task.id)
		.collect::<BTreeSet<_>>();
	if let Some(task) = finalized_tasks.difference(&tasks).next().copied() {
		return Err(InferenceExecutionError::UnexpectedPredictionOutput { task, value: None });
	}
	if tasks.difference(&finalized_tasks).next().is_some() || descriptors.is_empty() {
		return Err(InferenceExecutionError::MissingPredictionOutput);
	}
	let [descriptor] = descriptors.as_slice() else {
		return Err(InferenceExecutionError::DuplicatePredictionOutput {
			value: output.value(),
		});
	};
	if descriptor.logical != output.value() {
		return Err(InferenceExecutionError::UnexpectedPredictionOutput {
			task: descriptor.task,
			value: Some(descriptor.logical),
		});
	}
	if descriptor.source.dtype != output.dtype() {
		return Err(InferenceExecutionError::PredictionOutputDTypeMismatch {
			expected: output.dtype(),
			actual: descriptor.source.dtype,
		});
	}
	for actual in [descriptor.source.bytes, descriptor.bytes] {
		if actual != expected_bytes {
			return Err(InferenceExecutionError::PredictionOutputSizeMismatch {
				expected: expected_bytes,
				actual,
			});
		}
	}
	return Ok(*descriptor);
}

fn map_knn_inference_outputs(inference: &CompiledKnnInference, bundle: &FinalizedBundle, planned_outputs: impl IntoIterator<Item = (TaskId, ValueId, DeviceId, ValueId)>) -> InferenceExecutionResult<Vec<FinalizedInferenceOutput>> {
	let contracts = inference
		.outputs()
		.iter()
		.map(|output| return (output.value(), output))
		.collect::<BTreeMap<_, _>>();
	let expected_values = contracts.keys().copied().collect::<BTreeSet<_>>();
	let mut descriptors = BTreeMap::new();
	let mut tasks = BTreeSet::new();
	for (task, logical, device, physical) in planned_outputs {
		if !tasks.insert(task) || descriptors.contains_key(&logical) {
			return Err(InferenceExecutionError::DuplicatePredictionOutput { value: logical });
		}
		let contract = contracts
			.get(&logical)
			.copied()
			.ok_or(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			})?;
		let finalized = bundle
			.tasks()
			.iter()
			.find(|candidate| return candidate.id == task)
			.ok_or(InferenceExecutionError::MissingPredictionOutput)?;
		let TaskKind::Transfer(transfer) = &finalized.kind else {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		};
		if finalized.phase != RunPhase::Exit || transfer.destination != TransferEndpoint::External {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		}
		let endpoints = bundle
			.transfer_endpoints(task)
			.ok_or(InferenceExecutionError::MissingPredictionOutput)?;
		let ResolvedTransferEndpoint::Device(source) = endpoints.source else {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task,
				value: Some(logical),
			});
		};
		if source.device != device || source.value != physical {
			return Err(InferenceExecutionError::PredictionOutputSourceMismatch { task });
		}
		if source.dtype != contract.dtype() {
			return Err(InferenceExecutionError::PredictionOutputDTypeMismatch {
				expected: contract.dtype(),
				actual: source.dtype,
			});
		}
		let expected_bytes = contract.shape().bytes(contract.dtype()).map_err(|error| {
			return InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!("KNN prediction contract has an invalid typed shape: {error}"),
			};
		})?;
		for actual in [source.bytes, transfer.bytes] {
			if actual != expected_bytes {
				return Err(InferenceExecutionError::PredictionOutputSizeMismatch {
					expected: expected_bytes,
					actual,
				});
			}
		}
		descriptors.insert(logical, FinalizedInferenceOutput {
			task,
			logical,
			source,
			bytes: transfer.bytes,
		});
	}

	let finalized_tasks = bundle
		.tasks()
		.iter()
		.filter(|task| {
			return task.phase == RunPhase::Exit
				&& matches!( &task.kind,
					TaskKind::Transfer(transfer) if transfer.destination == TransferEndpoint::External );
		})
		.map(|task| return task.id)
		.collect::<BTreeSet<_>>();
	if let Some(task) = finalized_tasks.difference(&tasks).next().copied() {
		return Err(InferenceExecutionError::UnexpectedPredictionOutput { task, value: None });
	}
	if tasks != finalized_tasks || descriptors.keys().copied().collect::<BTreeSet<_>>() != expected_values {
		return Err(InferenceExecutionError::MissingPredictionOutput);
	}
	let ordered = inference
		.outputs()
		.iter()
		.map(|output| {
			return descriptors
				.get(&output.value())
				.copied()
				.ok_or(InferenceExecutionError::MissingPredictionOutput);
		})
		.collect::<InferenceExecutionResult<Vec<_>>>()?;
	for left_index in 0..ordered.len() {
		for right in &ordered[left_index + 1..] {
			let left = ordered[left_index];
			if output_locations_overlap(left.source, right.source) {
				return Err(InferenceExecutionError::PredictionOutputImagesOverlap {
					first: left.task,
					second: right.task,
				});
			}
		}
	}
	return Ok(ordered);
}

fn collect_knn_inference_predictions(contracts: &[KnnInferenceOutputContract], plans: &[FinalizedInferenceOutput], images: Vec<ExitImage>) -> InferenceExecutionResult<Vec<KnnInferencePrediction>> {
	if contracts.len() != plans.len() || images.len() != plans.len() || plans.is_empty() {
		return Err(InferenceExecutionError::MissingPredictionOutput);
	}
	let fallback = contracts[0].value();
	let mut by_task = BTreeMap::new();
	for image in images {
		let task = image.task;
		if by_task.insert(task, image).is_some() {
			return Err(InferenceExecutionError::DuplicatePredictionOutput { value: fallback });
		}
	}
	let image_refs = by_task.values().collect::<Vec<_>>();
	for left_index in 0..image_refs.len() {
		for right in &image_refs[left_index + 1..] {
			let left = image_refs[left_index];
			if output_locations_overlap(left.source, right.source) {
				return Err(InferenceExecutionError::PredictionOutputImagesOverlap {
					first: left.task,
					second: right.task,
				});
			}
		}
	}
	let mut predictions = Vec::with_capacity(contracts.len());
	for (contract, plan) in contracts.iter().zip(plans) {
		if plan.logical != contract.value() {
			return Err(InferenceExecutionError::UnexpectedPredictionOutput {
				task: plan.task,
				value: Some(plan.logical),
			});
		}
		let image = by_task
			.remove(&plan.task)
			.ok_or(InferenceExecutionError::MissingPredictionOutput)?;
		if image.source.dtype != contract.dtype() {
			return Err(InferenceExecutionError::PredictionOutputDTypeMismatch {
				expected: contract.dtype(),
				actual: image.source.dtype,
			});
		}
		let expected_bytes = contract.shape().bytes(contract.dtype()).map_err(|error| {
			return InferenceExecutionError::InvalidInferenceBoundary {
				detail: format!("KNN prediction contract has an invalid typed shape: {error}"),
			};
		})?;
		for actual in [image.source.bytes, host_byte_count(image.bytes.len())] {
			if actual != expected_bytes {
				return Err(InferenceExecutionError::PredictionOutputSizeMismatch {
					expected: expected_bytes,
					actual,
				});
			}
		}
		if image.source != plan.source {
			return Err(InferenceExecutionError::PredictionOutputSourceMismatch { task: image.task });
		}
		predictions.push(KnnInferencePrediction {
			contract: contract.clone(),
			bytes: image.bytes,
		});
	}
	if let Some((task, _)) = by_task.into_iter().next() {
		return Err(InferenceExecutionError::UnexpectedPredictionOutput { task, value: None });
	}
	return Ok(predictions);
}

fn collect_inference_prediction(contract: &InferenceOutputContract, plan: FinalizedInferenceOutput, images: Vec<ExitImage>) -> InferenceExecutionResult<InferencePrediction> {
	let expected_bytes = contract.shape().bytes(contract.dtype()).map_err(|error| {
		return InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("prediction contract has an invalid typed shape: {error}"),
		};
	})?;
	let bytes = validate_completed_prediction_images(
		contract.value(),
		contract.dtype(),
		expected_bytes,
		plan,
		images,
	)?;
	return Ok(InferencePrediction {
		contract: contract.clone(),
		bytes,
	});
}

fn validate_completed_prediction_images(expected_value: ValueId, expected_dtype: DType, expected_bytes: ByteCount, plan: FinalizedInferenceOutput, mut images: Vec<ExitImage>) -> InferenceExecutionResult<Vec<u8>> {
	if images.is_empty() {
		return Err(InferenceExecutionError::MissingPredictionOutput);
	}
	images.sort_by_key(|image| return image.task);
	let mut tasks = BTreeSet::new();
	for image in &images {
		if !tasks.insert(image.task) {
			return Err(InferenceExecutionError::DuplicatePredictionOutput {
				value: expected_value,
			});
		}
	}
	for left_index in 0..images.len() {
		for right in &images[left_index + 1..] {
			let left = &images[left_index];
			if output_locations_overlap(left.source, right.source) {
				return Err(InferenceExecutionError::PredictionOutputImagesOverlap {
					first: left.task,
					second: right.task,
				});
			}
		}
	}
	let [image] = images.as_slice() else {
		let unexpected = images
			.iter()
			.find(|image| return image.task != plan.task)
			.map_or(plan.task, |image| return image.task);
		return Err(InferenceExecutionError::UnexpectedPredictionOutput {
			task: unexpected,
			value: None,
		});
	};
	if image.task != plan.task {
		return Err(InferenceExecutionError::UnexpectedPredictionOutput {
			task: image.task,
			value: None,
		});
	}
	if image.source.dtype != expected_dtype {
		return Err(InferenceExecutionError::PredictionOutputDTypeMismatch {
			expected: expected_dtype,
			actual: image.source.dtype,
		});
	}
	for actual in [image.source.bytes, host_byte_count(image.bytes.len())] {
		if actual != expected_bytes {
			return Err(InferenceExecutionError::PredictionOutputSizeMismatch {
				expected: expected_bytes,
				actual,
			});
		}
	}
	if image.source != plan.source {
		return Err(InferenceExecutionError::PredictionOutputSourceMismatch { task: image.task });
	}
	let image = images
		.pop()
		.ok_or(InferenceExecutionError::MissingPredictionOutput)?;
	return Ok(image.bytes);
}

fn output_locations_overlap(left: ResolvedValueLocation, right: ResolvedValueLocation) -> bool {
	if left.device != right.device {
		return false;
	}
	let left_start = left.arena_offset.get();
	let right_start = right.arena_offset.get();
	let Some(left_end) = left_start.checked_add(left.bytes.get()) else {
		return true;
	};
	let Some(right_end) = right_start.checked_add(right.bytes.get()) else {
		return true;
	};
	return left_start < right_end && right_start < left_end;
}

fn drain_user_metrics(metrics: &mut [FinalTrainingMetric], mut take: impl FnMut(MetricSlotId) -> Option<MetricSample>, mut observer: Option<&mut TrainingMetricObserver>) {
	for metric in metrics {
		let Some(sample) = take(metric.slot) else {
			continue;
		};
		debug_assert_eq!(
			sample.metric, metric.metric,
			"metric slot returned a different metric identity"
		);
		let sample = TrainingMetricSample::from_executor(&sample);
		if let Some(observer) = observer.as_deref_mut() {
			observer.try_observe(&sample);
		}
		if metric
			.sample
			.as_ref()
			.is_none_or(|retained| return sample.sequence > retained.sequence)
		{
			metric.sample = Some(sample);
		}
	}
}

fn native_handoff_error<E>(error: LocalError<E>) -> TrainingExecutionError
where E: StdError + Send + Sync + 'static {
	return TrainingExecutionError::NativeHandoff(Box::new(error));
}

fn native_inference_handoff_error<E>(error: LocalError<E>) -> InferenceExecutionError
where E: StdError + Send + Sync + 'static {
	return InferenceExecutionError::NativeHandoff(Box::new(error));
}

fn inference_executor_failure<B: Backend>(failure: RunFailure<B>) -> InferenceExecutionError {
	let parts = failure.into_parts();
	let failure = InferenceRunFailure {
		run: parts.run_id,
		bundle: parts.bundle,
		journal: parts.journal,
		cleanup_error: parts.cleanup_error,
	};
	drop(parts.backend);
	return InferenceExecutionError::Executor {
		source: Box::new(parts.error),
		failure: Box::new(failure),
	};
}

fn post_exit_inference_failure(source: InferenceExecutionError, run: RunId, bundle: BundleIdentity, journal: RunJournal) -> InferenceExecutionError {
	return InferenceExecutionError::PostExitValidation {
		source: Box::new(source),
		failure: Box::new(InferenceRunFailure {
			run,
			bundle,
			journal: Some(journal),
			cleanup_error: None,
		}),
	};
}

fn reject_loop_external_transfers(bundle: &FinalizedBundle) -> TrainingExecutionResult<()> {
	for task in bundle
		.tasks()
		.iter()
		.filter(|task| return task.phase == RunPhase::Loop)
	{
		let TaskKind::Transfer(transfer) = &task.kind else {
			continue;
		};
		if matches!(transfer.source, TransferEndpoint::External) || matches!(transfer.destination, TransferEndpoint::External) {
			return Err(TrainingExecutionError::LoopExternalTransfer { task: task.id });
		}
	}
	return Ok(());
}

fn reject_inference_loop_external_transfers(bundle: &FinalizedBundle) -> InferenceExecutionResult<()> {
	if let Some(task) = inference_loop_external_transfer(bundle.tasks()) {
		return Err(InferenceExecutionError::LoopExternalTransfer { task });
	}
	return Ok(());
}

fn inference_loop_external_transfer(tasks: &[recipe_core::Task]) -> Option<TaskId> {
	return tasks
		.iter()
		.filter(|task| return task.phase == RunPhase::Loop)
		.find_map(|task| {
			match &task.kind {
				TaskKind::Transfer(transfer) if matches!(transfer.source, TransferEndpoint::External) || matches!(transfer.destination, TransferEndpoint::External) => {
					return Some(task.id);
				}
				TaskKind::Calculation(_) | TaskKind::Metric(_) | TaskKind::Transfer(_) => return None,
			}
		});
}

fn reject_inference_user_metrics(bundle: &FinalizedBundle) -> InferenceExecutionResult<()> {
	if let Some(task) = bundle.tasks().iter().find(|task| {
		return matches!( &task.kind,
			TaskKind::Metric(metric) if metric.purpose == MetricPurpose::User );
	}) {
		return Err(InferenceExecutionError::InvalidInferenceBoundary {
			detail: format!("finalized inference task {} emits a user metric", task.id),
		});
	}
	return Ok(());
}

fn user_metric_slots(bundle: &FinalizedBundle) -> Vec<(MetricSlotId, MetricId)> {
	let mut slots = bundle
		.tasks()
		.iter()
		.filter_map(|task| {
			match &task.kind {
				TaskKind::Metric(metric) if metric.purpose == MetricPurpose::User => {
					return Some((metric.slot, metric.metric));
				}
				TaskKind::Calculation(_) | TaskKind::Transfer(_) | TaskKind::Metric(_) => return None,
			}
		})
		.collect::<Vec<_>>();
	slots.sort_unstable();
	slots.dedup();
	return slots;
}

fn pack_device_images(inputs: &[OwnedExternalInput], manifests: &[InitDataImage]) -> TrainingExecutionResult<Vec<DeviceImage>> {
	let mut by_value = BTreeMap::new();
	for input in inputs {
		if by_value.insert(input.value(), input).is_some() {
			return Err(TrainingExecutionError::DuplicateExternalInput {
				value: input.value(),
			});
		}
	}
	let mut devices = BTreeSet::new();
	let mut result = Vec::with_capacity(manifests.len());
	for manifest in manifests {
		if !devices.insert(manifest.device) {
			return Err(TrainingExecutionError::DuplicateInitDevice {
				device: manifest.device,
			});
		}
		let image_len = usize::try_from(manifest.bytes.get()).map_err(|_error| {
			return TrainingExecutionError::ImageSizeUnsupported {
				device: manifest.device,
				bytes: manifest.bytes,
			};
		})?;
		let mut bytes = vec![0u8; image_len];
		let mut members = manifest.members.iter().collect::<Vec<_>>();
		members.sort_by_key(|member| return (member.image_offset, member.logical));
		let mut logical = BTreeSet::new();
		let mut previous: Option<(ValueId, u64)> = None;
		for member in members {
			if !logical.insert(member.logical) {
				return Err(TrainingExecutionError::DuplicateImageMember {
					device: manifest.device,
					value: member.logical,
				});
			}
			let input = by_value
				.get(&member.logical)
				.copied()
				.ok_or(TrainingExecutionError::MissingExternalInput {
					device: manifest.device,
					value: member.logical,
				})?;
			if input.dtype() != member.dtype {
				return Err(TrainingExecutionError::ImageMemberDTypeMismatch {
					device: manifest.device,
					value: member.logical,
				});
			}
			let actual = ByteCount::new(u64::try_from(input.bytes().len()).map_err(|_error| {
				return TrainingExecutionError::ImageMemberSizeMismatch {
					device: manifest.device,
					value: member.logical,
					expected: member.bytes,
					actual: ByteCount::new(u64::MAX),
				};
			})?);
			if actual != member.bytes {
				return Err(TrainingExecutionError::ImageMemberSizeMismatch {
					device: manifest.device,
					value: member.logical,
					expected: member.bytes,
					actual,
				});
			}
			let start = member.image_offset.get();
			let end = start
				.checked_add(member.bytes.get())
				.ok_or(TrainingExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				})?;
			if end > manifest.bytes.get() {
				return Err(TrainingExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				});
			}
			if let Some((prior, prior_end)) = previous
				&& start < prior_end
			{
				return Err(TrainingExecutionError::ImageMembersOverlap {
					device: manifest.device,
					first: prior,
					second: member.logical,
				});
			}
			let start = usize::try_from(start).map_err(|_error| {
				return TrainingExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				};
			})?;
			let end_index = usize::try_from(end).map_err(|_error| {
				return TrainingExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				};
			})?;
			let destination = bytes
				.get_mut(start..end_index)
				.ok_or(TrainingExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				})?;
			destination.copy_from_slice(input.bytes());
			previous = Some((member.logical, end));
		}
		result.push(DeviceImage::new(manifest.device, manifest.image, bytes));
	}
	result.sort_by_key(|image| return image.device);
	return Ok(result);
}
