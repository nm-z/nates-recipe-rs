use core::fmt;
use core::num::{NonZeroU64, NonZeroUsize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::time::Duration;

use recipe_core::{
	BundleIdentity, ByteCount, DeviceId, FinalizedBundle, InitDataImage, MetricId, MetricPurpose, MetricSlotId,
	ResolvedTransferEndpoint, RunId, RunPhase, TaskId, TaskKind, TransferEndpoint, ValueId,
};
use recipe_executor::{
	DeviceImage, ExecutorError, ExitImage, LoopStatus, MetricSample, PreparedRun, RunJournal, Watchdog,
};
use recipe_native_executor::{
	CandidateCrossBackendTransfer, CrossBackendTransfer, LocalError, LocalPreparedSession, ValidatedCandidateSession,
};
use recipe_prepare::{ArtifactProvider, CandidateRealizer, PrepareError, PreparedNativeSession, Preparer};
use recipe_probe::MeasuredProfile;

use crate::{CompiledTraining, OwnedExternalInput};

pub type TrainingExecutionResult<T> = Result<T, TrainingExecutionError>;

const BLOCKING_POLL_INITIAL_DELAY: Duration = Duration::from_micros(50);
const BLOCKING_POLL_MAXIMUM_DELAY: Duration = Duration::from_millis(2);

#[derive(Clone, Copy, Debug)]
struct BlockingPollBackoff {
	next_delay: Duration,
}

impl BlockingPollBackoff {
	const fn new() -> Self {
		Self {
			next_delay: BLOCKING_POLL_INITIAL_DELAY,
		}
	}

	fn reset(&mut self) {
		self.next_delay = BLOCKING_POLL_INITIAL_DELAY;
	}

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

/// Caller-selected executor watchdog.
///
/// Journal bounds are derived only after preparation produces the exact
/// [`FinalizedBundle`]; pending poll counts use fixed task-indexed storage and
/// therefore do not multiply this watchdog into host allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingExecutionLimits {
	pub watchdog: Watchdog,
}

impl TrainingExecutionLimits {
	#[must_use]
	pub const fn new(watchdog: Watchdog) -> Self {
		Self { watchdog }
	}
}

/// Nonblocking observer accounting. A dropped sample never delays executor
/// polling and does not affect the newest sample retained in the final report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingMetricObserverStats {
	pub observed: u64,
	pub cadence_eligible: u64,
	pub delivered: u64,
	pub dropped: u64,
}

/// Producer half of a bounded live-training metric channel.
///
/// Each selected metric has an independent emission counter. Only every
/// `cadence`th sample is offered to the channel. Offering uses `try_send`, so a
/// full or disconnected consumer can never backpressure executor polling.
#[derive(Debug)]
pub struct TrainingMetricObserver {
	sender: SyncSender<MetricSample>,
	selected: BTreeMap<MetricId, u64>,
	cadence: NonZeroU64,
	stats: TrainingMetricObserverStats,
}

impl TrainingMetricObserver {
	#[must_use]
	pub const fn stats(&self) -> TrainingMetricObserverStats {
		self.stats
	}

	fn try_observe(&mut self, sample: &MetricSample) {
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

/// Create a bounded live metric channel without giving its consumer any
/// capability over the running calculation.
///
/// The observer ignores metrics absent from `selected`. Channel saturation
/// drops only the due live notification; final execution evidence still
/// retains the newest sample for every planned user metric.
#[must_use]
pub fn bounded_training_metric_channel(
	capacity: NonZeroUsize,
	selected: impl IntoIterator<Item = MetricId>,
	cadence: NonZeroU64,
) -> (TrainingMetricObserver, Receiver<MetricSample>) {
	let (sender, receiver) = sync_channel(capacity.get());
	let observer = TrainingMetricObserver {
		sender,
		selected: selected.into_iter().map(|metric| (metric, 0)).collect(),
		cadence,
		stats: TrainingMetricObserverStats::default(),
	};
	(observer, receiver)
}

/// One planned user metric and the newest sample retained across the live loop.
///
/// `sample` is `None` only when its statically planned task never activated.
#[derive(Clone, Debug, PartialEq)]
pub struct FinalTrainingMetric {
	pub slot: MetricSlotId,
	pub metric: MetricId,
	pub sample: Option<MetricSample>,
}

/// Fully exited execution evidence. Native resources have been destroyed
/// before this value is returned.
#[derive(Clone, Debug)]
pub struct CompletedTrainingExecution {
	run: RunId,
	bundle: BundleIdentity,
	external_outputs: Vec<ExitImage>,
	external_output_values: Vec<ValueId>,
	metrics: Vec<FinalTrainingMetric>,
	journal: RunJournal,
}

impl CompletedTrainingExecution {
	#[must_use]
	pub const fn run(&self) -> RunId {
		self.run
	}

	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity {
		self.bundle
	}

	/// Exact finalized egress images. Their task and resolved source preserve
	/// the physical output identity selected by the planner.
	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] {
		&self.external_outputs
	}

	/// Logical tensor identity corresponding to each physical exit image.
	///
	/// Entries are aligned by index with [`Self::external_outputs`]. Physical
	/// source identities remain available on each [`ExitImage`].
	#[must_use]
	pub fn external_output_values(&self) -> &[ValueId] {
		&self.external_output_values
	}

	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] {
		&self.metrics
	}

	#[must_use]
	pub const fn journal(&self) -> &RunJournal {
		&self.journal
	}

	/// Decompose execution evidence while preserving the logical identity for
	/// every physical exit image. The two returned vectors are index-aligned.
	#[must_use]
	pub fn into_parts(
		self,
	) -> (
		RunId,
		BundleIdentity,
		Vec<ExitImage>,
		Vec<ValueId>,
		Vec<FinalTrainingMetric>,
		RunJournal,
	) {
		(
			self.run,
			self.bundle,
			self.external_outputs,
			self.external_output_values,
			self.metrics,
			self.journal,
		)
	}
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
	LoopDidNotReachTerminalState,
}

impl fmt::Display for TrainingExecutionError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Preparation(error) => write!(formatter, "training preparation failed: {error}"),
			Self::NativeHandoff(error) => write!(formatter, "native training handoff failed: {error}"),
			Self::Executor(error) => write!(formatter, "training execution failed: {error}"),
			Self::DuplicateExternalInput { value } => {
				write!(
					formatter,
					"training external input {value} appears more than once"
				)
			}
			Self::DuplicateInitDevice { device } => {
				write!(formatter, "finalized init image repeats device {device}")
			}
			Self::DuplicateImageMember { device, value } => {
				write!(
					formatter,
					"device {device} init image repeats logical input {value}"
				)
			}
			Self::MissingExternalInput { device, value } => {
				write!(
					formatter,
					"device {device} init image requires absent external input {value}"
				)
			}
			Self::ImageMemberDTypeMismatch { device, value } => {
				write!(
					formatter,
					"device {device} init member {value} has a different dtype than its training input"
				)
			}
			Self::ImageMemberSizeMismatch {
				device,
				value,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"device {device} init member {value} needs {} bytes, training input provides {}",
					expected.get(),
					actual.get()
				)
			}
			Self::ImageSizeUnsupported { device, bytes } => {
				write!(
					formatter,
					"device {device} init image size {} does not fit this host",
					bytes.get()
				)
			}
			Self::ImageMemberOutOfBounds { device, value } => {
				write!(
					formatter,
					"device {device} init member {value} lies outside its finalized image"
				)
			}
			Self::ImageMembersOverlap {
				device,
				first,
				second,
			} => {
				write!(
					formatter,
					"device {device} init members {first} and {second} overlap"
				)
			}
			Self::LoopExternalTransfer { task } => {
				write!(
					formatter,
					"finalized loop task {task} attempts an external data transfer"
				)
			}
			Self::ExternalOutputMapping { detail } => {
				write!(
					formatter,
					"finalized checkpoint output mapping is invalid: {detail}"
				)
			}
			Self::LoopDidNotReachTerminalState => formatter
				.write_str("bounded training wait returned before the loop reached terminal completion"),
		}
	}
}

impl StdError for TrainingExecutionError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Preparation(error) => Some(error),
			Self::NativeHandoff(error) => Some(error.as_ref()),
			Self::Executor(error) => Some(error),
			Self::DuplicateExternalInput { .. }
			| Self::DuplicateInitDevice { .. }
			| Self::DuplicateImageMember { .. }
			| Self::MissingExternalInput { .. }
			| Self::ImageMemberDTypeMismatch { .. }
			| Self::ImageMemberSizeMismatch { .. }
			| Self::ImageSizeUnsupported { .. }
			| Self::ImageMemberOutOfBounds { .. }
			| Self::ImageMembersOverlap { .. }
			| Self::LoopExternalTransfer { .. }
			| Self::ExternalOutputMapping { .. }
			| Self::LoopDidNotReachTerminalState => None,
		}
	}
}

impl From<PrepareError> for TrainingExecutionError {
	fn from(error: PrepareError) -> Self {
		Self::Preparation(error)
	}
}

impl From<ExecutorError> for TrainingExecutionError {
	fn from(error: ExecutorError) -> Self {
		Self::Executor(error)
	}
}

/// Pack Recipe's owned external inputs into exactly one finalized admission
/// image per device.
///
/// Gaps and fault-buffer bytes remain zero. A logical input may be copied into
/// multiple device images, but it is uploaded only through each device's
/// singular init admission. Inputs absent from every manifest are unused by
/// the selected candidate and cause no upload.
pub fn build_training_device_images(
	training: &CompiledTraining,
	bundle: &FinalizedBundle,
) -> TrainingExecutionResult<Vec<DeviceImage>> {
	pack_device_images(training.external_inputs(), bundle.init_images())
}

/// Build/realize the exact static program and execute its complete native
/// `init -> loop -> exit` lifecycle.
///
/// The caller supplies a production-configured [`Preparer`] created inside the
/// native binding scope. This function invokes preparation itself, consumes
/// the same warmed local session at Finalize handoff, admits one image per
/// finalized device, and exposes only post-exit outputs and newest user metric
/// samples. The live loop has no API for data/file ingress or egress.
pub fn prepare_and_execute_local_training<'cuda, 'hsa, A, R, Bridge>(
	training: &CompiledTraining,
	profile: &MeasuredProfile,
	preparer: &mut Preparer<A, R>,
	run: RunId,
	limits: TrainingExecutionLimits,
) -> TrainingExecutionResult<CompletedTrainingExecution>
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
	prepare_and_execute_local_training_inner(training, profile, preparer, run, limits, None)
}

/// Execute a complete local training lifecycle while offering selected live
/// metrics to a bounded nonblocking observer.
///
/// Observer saturation or disconnection drops live notifications without
/// changing calculation progress or final metric retention.
pub fn prepare_and_execute_local_training_with_observer<'cuda, 'hsa, A, R, Bridge>(
	training: &CompiledTraining,
	profile: &MeasuredProfile,
	preparer: &mut Preparer<A, R>,
	run: RunId,
	limits: TrainingExecutionLimits,
	observer: &mut TrainingMetricObserver,
) -> TrainingExecutionResult<CompletedTrainingExecution>
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
	prepare_and_execute_local_training_inner(training, profile, preparer, run, limits, Some(observer))
}

fn prepare_and_execute_local_training_inner<'cuda, 'hsa, A, R, Bridge>(
	training: &CompiledTraining,
	profile: &MeasuredProfile,
	preparer: &mut Preparer<A, R>,
	run: RunId,
	limits: TrainingExecutionLimits,
	mut observer: Option<&mut TrainingMetricObserver>,
) -> TrainingExecutionResult<CompletedTrainingExecution>
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
	let prepared_system = preparer.prepare_program(training.program(), profile)?;
	reject_loop_external_transfers(prepared_system.bundle())?;
	let images = build_training_device_images(training, prepared_system.bundle())?;
	let external_output_tasks = map_external_output_tasks(
		training,
		prepared_system.bundle(),
		prepared_system.external_outputs(),
	)?;
	let mut metrics = user_metric_slots(prepared_system.bundle())
		.into_iter()
		.map(|(slot, metric)| FinalTrainingMetric {
			slot,
			metric,
			sample: None,
		})
		.collect::<Vec<_>>();
	let (bundle, _realization, _catalog, native_session) = prepared_system.into_parts();
	let (validated_session, _artifacts) = native_session.into_parts();
	let local_session = validated_session.into_inner();
	let backend = local_session
		.into_backend(&bundle)
		.map_err(native_handoff_error::<Bridge::Error>)?;

	let initialized = PreparedRun::prepare(run, bundle, backend, limits.watchdog)?.initialize(images)?;
	let mut running = initialized.start_loop()?;
	let mut backoff = BlockingPollBackoff::new();
	loop {
		let (status, made_progress) = running.poll_with_progress()?;
		drain_user_metrics(
			&mut metrics,
			|slot| running.try_take_metric(slot),
			observer.as_deref_mut(),
		);
		if status == LoopStatus::Complete {
			break;
		}
		match made_progress {
			true => backoff.reset(),
			false => backoff.wait(),
		}
	}
	let mut exited_loop = running
		.into_exited_loop()
		.map_err(|_running| TrainingExecutionError::LoopDidNotReachTerminalState)?;
	drain_user_metrics(
		&mut metrics,
		|slot| exited_loop.try_take_metric(slot),
		observer.as_deref_mut(),
	);
	let exited = exited_loop.exit()?;
	let bundle_identity = exited.bundle_identity();
	let (_backend, mut mailbox, mut external_outputs, journal) = exited.into_parts();
	external_outputs.sort_by_key(|image| image.task);
	let external_output_values = external_outputs
		.iter()
		.map(|image| {
			external_output_tasks
				.get(&image.task)
				.copied()
				.ok_or_else(|| TrainingExecutionError::ExternalOutputMapping {
					detail: format!("completed exit task {} has no logical tensor", image.task),
				})
		})
		.collect::<TrainingExecutionResult<Vec<_>>>()?;
	drain_user_metrics(
		&mut metrics,
		|slot| mailbox.try_take(slot),
		observer.as_deref_mut(),
	);
	Ok(CompletedTrainingExecution {
		run,
		bundle: bundle_identity,
		external_outputs,
		external_output_values,
		metrics,
		journal,
	})
}

fn map_external_output_tasks(
	training: &CompiledTraining,
	bundle: &FinalizedBundle,
	planned_outputs: impl IntoIterator<Item = (TaskId, ValueId, DeviceId, ValueId)>,
) -> TrainingExecutionResult<BTreeMap<TaskId, ValueId>> {
	let expected = training
		.graph()
		.tensors
		.iter()
		.filter(|tensor| tensor.external_output)
		.map(|tensor| tensor.id)
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
			.find(|candidate| candidate.id == task)
			.ok_or_else(|| TrainingExecutionError::ExternalOutputMapping {
				detail: format!("planned exit task {task} is absent from the finalized bundle"),
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
		let endpoints =
			bundle.transfer_endpoints(task)
				.ok_or_else(|| TrainingExecutionError::ExternalOutputMapping {
					detail: format!("exit task {task} has no finalized endpoints"),
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
			task.phase == RunPhase::Exit
				&& matches!(
					&task.kind,
					TaskKind::Transfer(recipe_core::TransferTask {
						destination: TransferEndpoint::External,
						..
					})
				)
		})
		.map(|task| task.id)
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
	Ok(mapped)
}

fn drain_user_metrics(
	metrics: &mut [FinalTrainingMetric],
	mut take: impl FnMut(MetricSlotId) -> Option<MetricSample>,
	mut observer: Option<&mut TrainingMetricObserver>,
) {
	for metric in metrics {
		let Some(sample) = take(metric.slot) else {
			continue;
		};
		debug_assert_eq!(sample.metric, metric.metric);
		if let Some(observer) = observer.as_deref_mut() {
			observer.try_observe(&sample);
		}
		if metric
			.sample
			.as_ref()
			.is_none_or(|retained| sample.sequence > retained.sequence)
		{
			metric.sample = Some(sample);
		}
	}
}

fn native_handoff_error<E>(error: LocalError<E>) -> TrainingExecutionError
where
	E: StdError + Send + Sync + 'static,
{
	TrainingExecutionError::NativeHandoff(Box::new(error))
}

fn reject_loop_external_transfers(bundle: &FinalizedBundle) -> TrainingExecutionResult<()> {
	for task in bundle
		.tasks()
		.iter()
		.filter(|task| task.phase == RunPhase::Loop)
	{
		let TaskKind::Transfer(transfer) = &task.kind else {
			continue;
		};
		if matches!(transfer.source, TransferEndpoint::External)
			|| matches!(transfer.destination, TransferEndpoint::External)
		{
			return Err(TrainingExecutionError::LoopExternalTransfer { task: task.id });
		}
	}
	Ok(())
}

fn user_metric_slots(bundle: &FinalizedBundle) -> Vec<(MetricSlotId, MetricId)> {
	let mut slots = bundle
		.tasks()
		.iter()
		.filter_map(|task| match &task.kind {
			TaskKind::Metric(metric) if metric.purpose == MetricPurpose::User => {
				Some((metric.slot, metric.metric))
			}
			_ => None,
		})
		.collect::<Vec<_>>();
	slots.sort_unstable();
	slots.dedup();
	slots
}

fn pack_device_images(
	inputs: &[OwnedExternalInput],
	manifests: &[InitDataImage],
) -> TrainingExecutionResult<Vec<DeviceImage>> {
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
		let image_len =
			usize::try_from(manifest.bytes.get()).map_err(|_| TrainingExecutionError::ImageSizeUnsupported {
				device: manifest.device,
				bytes: manifest.bytes,
			})?;
		let mut bytes = vec![0u8; image_len];
		let mut members = manifest.members.iter().collect::<Vec<_>>();
		members.sort_by_key(|member| (member.image_offset, member.logical));
		let mut logical = BTreeSet::new();
		let mut previous: Option<(ValueId, u64)> = None;
		for member in members {
			if !logical.insert(member.logical) {
				return Err(TrainingExecutionError::DuplicateImageMember {
					device: manifest.device,
					value: member.logical,
				});
			}
			let input =
				by_value
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
			let actual = ByteCount::new(u64::try_from(input.bytes().len()).map_err(|_| {
				TrainingExecutionError::ImageMemberSizeMismatch {
					device: manifest.device,
					value: member.logical,
					expected: member.bytes,
					actual: ByteCount::new(u64::MAX),
				}
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
			let end = start.checked_add(member.bytes.get()).ok_or(
				TrainingExecutionError::ImageMemberOutOfBounds {
					device: manifest.device,
					value: member.logical,
				},
			)?;
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
			let start = usize::try_from(start).map_err(|_| TrainingExecutionError::ImageMemberOutOfBounds {
				device: manifest.device,
				value: member.logical,
			})?;
			let end_index = usize::try_from(end).map_err(|_| TrainingExecutionError::ImageMemberOutOfBounds {
				device: manifest.device,
				value: member.logical,
			})?;
			let destination =
				bytes.get_mut(start..end_index)
					.ok_or(TrainingExecutionError::ImageMemberOutOfBounds {
						device: manifest.device,
						value: member.logical,
					})?;
			destination.copy_from_slice(input.bytes());
			previous = Some((member.logical, end));
		}
		result.push(DeviceImage::new(manifest.device, manifest.image, bytes));
	}
	result.sort_by_key(|image| image.device);
	Ok(result)
}

#[cfg(test)]
mod tests {
	use recipe_core::{ByteOffset, DType, InitDataImageMember, LoopIterations, MetricId, MetricSlotId};
	use recipe_executor::{MetricSample, MetricValue};
	use recipe_language::Shape;

	use super::*;
	use crate::ExternalInputRole;

	#[test]
	fn training_nonprogress_backoff_caps_and_progress_resets_it() {
		let mut backoff = BlockingPollBackoff::new();
		assert_eq!(backoff.next_delay, BLOCKING_POLL_INITIAL_DELAY);
		for _ in 0..16 {
			backoff.advance();
		}
		assert_eq!(backoff.next_delay, BLOCKING_POLL_MAXIMUM_DELAY);
		backoff.reset();
		assert_eq!(backoff.next_delay, BLOCKING_POLL_INITIAL_DELAY);
	}

	fn input(value: u64, bytes: &[u8]) -> OwnedExternalInput {
		OwnedExternalInput::new(
			ExternalInputRole::TrainFeatures,
			ValueId::new(value),
			DType::F32,
			Shape::new(vec![u64::try_from(bytes.len() / 4).unwrap()]).unwrap(),
			bytes.to_vec(),
		)
	}

	fn member(logical: u64, physical: u64, bytes: u64, offset: u64) -> InitDataImageMember {
		InitDataImageMember {
			logical: ValueId::new(logical),
			physical: ValueId::new(physical),
			dtype: DType::F32,
			bytes: ByteCount::new(bytes),
			image_offset: ByteOffset::new(offset),
		}
	}

	fn sample(sequence: u64, metric: u64, slot: u64, iteration: u64) -> MetricSample {
		let iterations = LoopIterations::new(16).unwrap();
		MetricSample {
			sequence,
			iteration: iterations.iteration(iteration).unwrap(),
			task: TaskId::new(slot),
			slot: MetricSlotId::new(slot),
			metric: MetricId::new(metric),
			value: MetricValue::F32(sequence as f32),
		}
	}

	#[test]
	fn packs_one_zero_filled_image_per_device_from_finalized_offsets() {
		let inputs = vec![
			input(1, &[1, 2, 3, 4]),
			input(2, &[5, 6, 7, 8, 9, 10, 11, 12]),
			input(99, &[13, 14, 15, 16]),
		];
		let manifests = vec![
			InitDataImage {
				device: DeviceId::new(2),
				image: ValueId::new(200),
				bytes: ByteCount::new(16),
				members: vec![member(2, 202, 8, 8), member(1, 201, 4, 0)],
			},
			InitDataImage {
				device: DeviceId::new(1),
				image: ValueId::new(100),
				bytes: ByteCount::new(8),
				members: vec![member(1, 101, 4, 4)],
			},
		];
		let images = pack_device_images(&inputs, &manifests).unwrap();
		assert_eq!(images.len(), 2);
		assert_eq!(images[0].device, DeviceId::new(1));
		assert_eq!(images[0].image, ValueId::new(100));
		assert_eq!(images[0].bytes, [0, 0, 0, 0, 1, 2, 3, 4]);
		assert_eq!(images[1].device, DeviceId::new(2));
		assert_eq!(
			images[1].bytes,
			[1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8, 9, 10, 11, 12]
		);
	}

	#[test]
	fn rejects_missing_mismatched_and_overlapping_members() {
		let inputs = vec![input(1, &[1, 2, 3, 4])];
		let missing = [InitDataImage {
			device: DeviceId::new(1),
			image: ValueId::new(100),
			bytes: ByteCount::new(4),
			members: vec![member(2, 102, 4, 0)],
		}];
		assert!(matches!(
			pack_device_images(&inputs, &missing),
			Err(TrainingExecutionError::MissingExternalInput { .. })
		));

		let wrong_size = [InitDataImage {
			device: DeviceId::new(1),
			image: ValueId::new(100),
			bytes: ByteCount::new(8),
			members: vec![member(1, 101, 8, 0)],
		}];
		assert!(matches!(
			pack_device_images(&inputs, &wrong_size),
			Err(TrainingExecutionError::ImageMemberSizeMismatch { .. })
		));

		let overlap_inputs = vec![input(1, &[1, 2, 3, 4]), input(2, &[5, 6, 7, 8])];
		let overlap = [InitDataImage {
			device: DeviceId::new(1),
			image: ValueId::new(100),
			bytes: ByteCount::new(8),
			members: vec![member(1, 101, 4, 0), member(2, 102, 4, 2)],
		}];
		assert!(matches!(
			pack_device_images(&overlap_inputs, &overlap),
			Err(TrainingExecutionError::ImageMembersOverlap { .. })
		));
	}

	#[test]
	fn live_drain_never_loses_the_newest_final_sample_when_observer_is_full() {
		let (mut observer, receiver) = bounded_training_metric_channel(
			NonZeroUsize::new(1).unwrap(),
			[MetricId::new(7)],
			NonZeroU64::MIN,
		);
		let mut metrics = vec![FinalTrainingMetric {
			slot: MetricSlotId::new(11),
			metric: MetricId::new(7),
			sample: None,
		}];
		let mut first = Some(sample(1, 7, 11, 0));
		drain_user_metrics(&mut metrics, |_| first.take(), Some(&mut observer));
		let mut second = Some(sample(2, 7, 11, 1));
		drain_user_metrics(&mut metrics, |_| second.take(), Some(&mut observer));

		assert_eq!(metrics[0].sample.as_ref().unwrap().sequence, 2);
		assert_eq!(receiver.try_recv().unwrap().sequence, 1);
		assert_eq!(
			observer.stats(),
			TrainingMetricObserverStats {
				observed: 2,
				cadence_eligible: 2,
				delivered: 1,
				dropped: 1,
			}
		);
	}

	#[test]
	fn bounded_observer_filters_metrics_and_counts_cadence_before_try_send() {
		let (mut observer, receiver) = bounded_training_metric_channel(
			NonZeroUsize::new(4).unwrap(),
			[MetricId::new(3)],
			NonZeroU64::new(2).unwrap(),
		);
		for sequence in 0..4 {
			observer.try_observe(&sample(sequence, 3, 9, sequence));
			observer.try_observe(&sample(sequence + 10, 4, 10, sequence));
		}
		assert_eq!(
			receiver
				.try_iter()
				.map(|sample| sample.sequence)
				.collect::<Vec<_>>(),
			[1, 3]
		);
		assert_eq!(
			observer.stats(),
			TrainingMetricObserverStats {
				observed: 4,
				cadence_eligible: 2,
				delivered: 2,
				dropped: 0,
			}
		);
	}
}
