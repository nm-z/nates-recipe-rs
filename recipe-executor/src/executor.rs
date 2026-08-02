use std::{
	collections::BTreeMap,
	fmt::{self, Write},
	num::NonZeroU32,
	time::Duration,
};

use recipe_core::{
	ArtifactId, BundleIdentity, ByteCount, DeviceId, FinalizedBundle, IterationDomain, KernelTemplateId, LinkId,
	LoopIteration, MetricId, MetricPurpose, MetricSlotId, ResolvedTransferEndpoint, ResolvedValueLocation, RunId,
	RunPhase, ScheduleWindow, SubmissionSlots, Task, TaskId, TaskKind, TransferEndpoint, TransferLaneClaim, ValueId,
};

use crate::{
	backend::{
		ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork,
		MAX_PHYSICAL_CALLS_PER_OPERATION, MetricWork, PendingRequest, PhysicalCall, PhysicalCallBatch,
		TransferWork, WorkClass,
	},
	error::{BackendMessage, BackendOperation, ExecutorError, JournalStream, Result},
	metrics::{MetricMailbox, MetricSample},
};

const RUN_LIFECYCLE_LOGICAL_EVENTS: usize = 6;
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

	fn reset(&mut self) { self.next_delay = BLOCKING_POLL_INITIAL_DELAY; }

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

#[derive(Clone, Copy, Debug)]
struct PhasePoll {
	complete: bool,
	made_progress: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Watchdog {
	max_nonprogress_polls: u32,
}

impl Watchdog {
	pub fn new(max_nonprogress_polls: u32) -> Result<Self> {
		match max_nonprogress_polls {
			0 => {
				Err(ExecutorError::InvalidWatchdog {
					max_nonprogress_polls,
				})
			}
			_ => {
				Ok(Self {
					max_nonprogress_polls,
				})
			}
		}
	}

	#[must_use]
	pub const fn max_nonprogress_polls(self) -> u32 { self.max_nonprogress_polls }

	/// Convert a measured upper-bound operation duration into the number of
	/// nonprogress polls admitted by the executor's blocking backoff.
	///
	/// The extra maximum-delay interval prevents a zero- or sub-poll estimate
	/// from expiring on the first asynchronous observation. Saturation retains
	/// a finite watchdog even when the duration exceeds the `u32` poll domain.
	#[must_use]
	pub fn for_expected_duration(expected: Duration, safety_multiplier: NonZeroU32) -> Self {
		let target = expected
			.as_nanos()
			.saturating_mul(u128::from(safety_multiplier.get()))
			.saturating_add(BLOCKING_POLL_MAXIMUM_DELAY.as_nanos());
		let mut elapsed = 0_u128;
		let mut delay = BLOCKING_POLL_INITIAL_DELAY;
		let mut sleeps = 0_u128;
		while elapsed < target && delay < BLOCKING_POLL_MAXIMUM_DELAY {
			elapsed = elapsed.saturating_add(delay.as_nanos());
			delay = delay.saturating_mul(2).min(BLOCKING_POLL_MAXIMUM_DELAY);
			sleeps += 1;
		}
		if elapsed < target {
			let remaining = target - elapsed;
			let maximum_delay = BLOCKING_POLL_MAXIMUM_DELAY.as_nanos();
			sleeps = sleeps.saturating_add(remaining.saturating_add(maximum_delay - 1) / maximum_delay);
		}
		let polls = sleeps.saturating_add(1).min(u128::from(u32::MAX)) as u32;
		Self {
			max_nonprogress_polls: polls.max(1),
		}
	}
}

/// Packed external image for one required device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceImage {
	pub device: DeviceId,
	pub image: ValueId,
	pub bytes: Vec<u8>,
}

impl DeviceImage {
	#[must_use]
	pub fn new(device: DeviceId, image: ValueId, bytes: Vec<u8>) -> Self {
		Self {
			device,
			image,
			bytes,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct InitImageKey {
	device: DeviceId,
	image: ValueId,
	bytes: ByteCount,
}

/// One external egress image produced by a finalized exit transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitImage {
	pub task: TaskId,
	pub source: ResolvedValueLocation,
	pub bytes: Vec<u8>,
}

/// Logical contract events, separate from backend-reported physical calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalEvent {
	Prepared {
		run: RunId,
		bundle: BundleIdentity,
	},
	ArenaAllocated {
		device: DeviceId,
		bytes: ByteCount,
	},
	InitAdmission {
		task: TaskId,
		device: DeviceId,
		bytes: ByteCount,
	},
	TaskSubmitted {
		phase: RunPhase,
		task: TaskId,
		class: WorkClass,
	},
	TaskCompleted {
		phase: RunPhase,
		task: TaskId,
	},
	ExternalTransferSubmitted {
		phase: RunPhase,
		task: TaskId,
		direction: crate::worker::ExternalTransferDirection,
		bytes: ByteCount,
	},
	ExternalTransferCompleted {
		phase: RunPhase,
		task: TaskId,
		direction: crate::worker::ExternalTransferDirection,
		bytes: ByteCount,
	},
	Initialized {
		run: RunId,
	},
	LoopStarted {
		run: RunId,
	},
	LoopIterationStarted {
		run: RunId,
		iteration: LoopIteration,
	},
	MetricPublished {
		task: TaskId,
		slot: MetricSlotId,
		replaced_unconsumed: bool,
	},
	FaultChecked {
		readback: TaskId,
		value: ValueId,
	},
	LoopIterationCompleted {
		run: RunId,
		iteration: LoopIteration,
	},
	LoopStopAccepted {
		run: RunId,
		after_iteration: LoopIteration,
	},
	LoopCompleted {
		run: RunId,
	},
	LoopFailed {
		run: RunId,
	},
	WorkerQuiesced {
		run: RunId,
	},
	ArenaReleased {
		device: DeviceId,
	},
	Exited {
		run: RunId,
	},
}

/// Declared fixed journal bounds for one run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalCapacity {
	pub logical_events: usize,
	pub physical_calls: usize,
}

impl JournalCapacity {
	#[must_use]
	pub const fn new(logical_events: usize, physical_calls: usize) -> Self {
		Self {
			logical_events,
			physical_calls,
		}
	}

	/// Derive fixed journal bounds from the exact finalized task graph.
	///
	/// Pending completion polls are represented by one ordered marker and one
	/// exact counter per finalized task, so this bound is independent of the
	/// watchdog stretch.
	pub fn for_bundle<B: Backend>(bundle: &FinalizedBundle) -> Result<Self> {
		let retained_loop_iterations = 1_u64;
		Self::for_bundle_retaining::<B>(bundle, retained_loop_iterations)
	}

	pub(crate) fn for_bundle_retaining<B: Backend>(
		bundle: &FinalizedBundle,
		retained_loop_iterations: u64,
	) -> Result<Self> {
		if B::MAX_NON_POLL_PHYSICAL_CALLS == 0 || B::MAX_NON_POLL_PHYSICAL_CALLS > MAX_PHYSICAL_CALLS_PER_OPERATION
		{
			return Err(ExecutorError::PreparationCapacityOverflow);
		}
		let task_count = bundle.tasks().len();
		let retained_loop_iteration_count =
			usize::try_from(retained_loop_iterations).map_err(|_| ExecutorError::PreparationCapacityOverflow)?;
		let loop_task_count = bundle
			.tasks()
			.iter()
			.filter(|task| task.phase == RunPhase::Loop)
			.count();
		let non_loop_task_count = task_count
			.checked_sub(loop_task_count)
			.ok_or(ExecutorError::PreparationCapacityOverflow)?;
		let loop_task_completions = checked_capacity_mul(loop_task_count, retained_loop_iteration_count)?;
		let active_loop_tasks = bundle
			.tasks()
			.iter()
			.filter(|task| task.phase == RunPhase::Loop)
			.try_fold(0_usize, |total, task| {
				let domain = bundle
					.iteration_domain(task.id)
					.ok_or(ExecutorError::PreparationCapacityOverflow)?;
				total.checked_add(iteration_domain_activations_before(
					domain,
					retained_loop_iterations,
				)?)
				.ok_or(ExecutorError::PreparationCapacityOverflow)
			})?;
		let task_executions = checked_capacity_sum(&[non_loop_task_count, active_loop_tasks])?;
		let arena_count = bundle.arena_layouts().len();
		let metric_emissions = bundle
			.tasks()
			.iter()
			.filter(|task| task.phase == RunPhase::Loop && matches!(task.kind, TaskKind::Metric(_)))
			.try_fold(0_usize, |total, task| {
				let domain = bundle
					.iteration_domain(task.id)
					.ok_or(ExecutorError::PreparationCapacityOverflow)?;
				total.checked_add(iteration_domain_activations_before(
					domain,
					retained_loop_iterations,
				)?)
				.ok_or(ExecutorError::PreparationCapacityOverflow)
			})?;
		let exit_image_count = bundle
			.tasks()
			.iter()
			.filter(|task| {
				task.phase == RunPhase::Exit
					&& matches!(
						task.kind,
						TaskKind::Transfer(recipe_core::TransferTask {
							destination: TransferEndpoint::External,
							..
						})
					)
			})
			.count();

		let logical_events = checked_capacity_sum(&[
			// Prepared, Initialized, LoopStarted, optional LoopStopAccepted,
			// LoopCompleted, and Exited.
			RUN_LIFECYCLE_LOGICAL_EVENTS,
			checked_capacity_mul(arena_count, 2)?,
			checked_capacity_mul(non_loop_task_count, 2)?,
			loop_task_completions,
			active_loop_tasks,
			metric_emissions,
			checked_capacity_mul(retained_loop_iteration_count, 2)?,
		])?;

		// Every non-poll operation may report a full inline batch. A completed
		// task contributes one terminal poll record, while arbitrarily many
		// pending polls consume only its fixed counter and first-marker slot.
		let fixed_backend_operations = checked_capacity_sum(&[
			2,
			checked_capacity_mul(arena_count, 2)?,
			task_count,
			task_executions,
			exit_image_count,
		])?;
		let fixed_physical = checked_capacity_mul(fixed_backend_operations, B::MAX_NON_POLL_PHYSICAL_CALLS)?;
		let physical_calls = checked_capacity_sum(&[fixed_physical, task_executions, task_count])?;
		Ok(Self::new(logical_events, physical_calls))
	}
}

/// Exact number of pending completion polls reported for one finalized task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingPollCount {
	pub task: TaskId,
	pub count: u128,
}

/// Bounded production-journal accounting across the complete run.
///
/// Production retains ordered detail for the first loop iteration plus every
/// non-repeating lifecycle event. These counters preserve how much activity was
/// observed and compacted without allocating in proportion to the loop bound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JournalSummary {
	pub logical_events_observed: u128,
	pub logical_events_compacted: u128,
	pub physical_calls_observed: u128,
	pub physical_calls_compacted: u128,
}

#[derive(Clone, Debug)]
pub struct RunJournal {
	logical_events: Vec<LogicalEvent>,
	physical_calls: Vec<PhysicalCall>,
	pending_polls: Vec<PendingPollCount>,
	loop_tasks: Vec<TaskId>,
	current_loop_iteration: Option<LoopIteration>,
	retain_all_loop_iterations: bool,
	summary: JournalSummary,
	declared: JournalCapacity,
}

impl RunJournal {
	pub(crate) fn with_capacity(declared: JournalCapacity, tasks: &[Task]) -> Self {
		Self::with_loop_detail(declared, tasks, false)
	}

	pub(crate) fn with_loop_detail(
		declared: JournalCapacity,
		tasks: &[Task],
		retain_all_loop_iterations: bool,
	) -> Self {
		let mut pending_polls = tasks
			.iter()
			.map(|task| {
				PendingPollCount {
					task: task.id,
					count: 0,
				}
			})
			.collect::<Vec<_>>();
		pending_polls.sort_unstable_by_key(|entry| entry.task);
		let mut loop_tasks = tasks
			.iter()
			.filter_map(|task| (task.phase == RunPhase::Loop).then_some(task.id))
			.collect::<Vec<_>>();
		loop_tasks.sort_unstable();
		Self {
			logical_events: Vec::with_capacity(declared.logical_events),
			physical_calls: Vec::with_capacity(declared.physical_calls),
			pending_polls,
			loop_tasks,
			current_loop_iteration: None,
			retain_all_loop_iterations,
			summary: JournalSummary::default(),
			declared,
		}
	}

	pub(crate) fn record_logical(&mut self, event: LogicalEvent) -> Result<()> {
		self.summary.logical_events_observed = self.summary.logical_events_observed.saturating_add(1);
		if let LogicalEvent::LoopIterationStarted { iteration, .. } = &event {
			self.current_loop_iteration = Some(*iteration);
		}
		if !self.retains_repeated_loop_detail() && repeated_loop_event(&event) {
			self.summary.logical_events_compacted = self.summary.logical_events_compacted.saturating_add(1);
			return Ok(());
		}
		match self.logical_events.len() < self.declared.logical_events {
			true => {
				self.logical_events.push(event);
				Ok(())
			}
			false => {
				Err(ExecutorError::JournalCapacityExceeded {
					stream: JournalStream::Logical,
					capacity: self.declared.logical_events,
				})
			}
		}
	}

	fn retains_repeated_loop_detail(&self) -> bool {
		self.retain_all_loop_iterations
			|| self
				.current_loop_iteration
				.is_none_or(|iteration| iteration.index() == 0)
	}

	fn is_loop_task(&self, task: TaskId) -> bool { self.loop_tasks.binary_search(&task).is_ok() }

	fn retains_physical_call(&self, call: PhysicalCall) -> bool {
		self.retains_repeated_loop_detail() || !self.is_repeated_loop_physical(call)
	}

	fn is_repeated_loop_physical(&self, call: PhysicalCall) -> bool {
		let task = match call {
			PhysicalCall::SubmitCalculation { task }
			| PhysicalCall::SubmitInternalTransfer { task }
			| PhysicalCall::SubmitMetric { task, .. }
			| PhysicalCall::SubmitExternalIngress { task, .. }
			| PhysicalCall::SubmitExternalEgress { task, .. }
			| PhysicalCall::AcknowledgeExternalEgress { task }
			| PhysicalCall::Poll { task, .. } => Some(task),
			PhysicalCall::BindResources
			| PhysicalCall::PreparePending { .. }
			| PhysicalCall::PrepareExternal { .. }
			| PhysicalCall::AllocateArena { .. }
			| PhysicalCall::AdmissionChunk { .. }
			| PhysicalCall::SubmitExitTransfer { .. }
			| PhysicalCall::QuiesceWorker
			| PhysicalCall::CollectExit { .. }
			| PhysicalCall::ReleaseArena { .. }
			| PhysicalCall::DestroyResources => None,
		};
		task.is_some_and(|task| self.is_loop_task(task))
	}

	pub(crate) fn record_physical(&mut self, calls: PhysicalCallBatch) -> Result<()> {
		let mut pending_batch = [None::<(TaskId, u8)>; MAX_PHYSICAL_CALLS_PER_OPERATION];
		let mut pending_batch_len = 0_usize;
		let mut retained_calls = [None::<PhysicalCall>; MAX_PHYSICAL_CALLS_PER_OPERATION];
		let mut retained_len = 0_usize;
		for call in calls.iter() {
			let mut retain = self.retains_physical_call(call);
			if let PhysicalCall::Poll {
				task,
				status: crate::backend::PhysicalPollStatus::Pending,
			} = call
			{
				let index = self
					.pending_polls
					.binary_search_by_key(&task, |entry| entry.task)
					.map_err(|_| {
						ExecutorError::BackendProtocol {
							task,
							detail: "pending poll names a task absent from the finalized bundle",
						}
					})?;
				let existing = pending_batch[..pending_batch_len]
					.iter_mut()
					.find_map(|entry| entry.as_mut().filter(|(candidate, _)| *candidate == task));
				match existing {
					Some((_, count)) => {
						*count = count
							.checked_add(1)
							.ok_or(ExecutorError::PendingPollCountOverflow { task })?;
						retain = false;
					}
					None => {
						pending_batch[pending_batch_len] = Some((task, 1));
						pending_batch_len += 1;
						retain &= self.pending_polls[index].count == 0;
					}
				}
			}
			if retain {
				retained_calls[retained_len] = Some(call);
				retained_len += 1;
			}
		}
		for (task, increment) in pending_batch[..pending_batch_len].iter().flatten() {
			let index = self
				.pending_polls
				.binary_search_by_key(task, |entry| entry.task)
				.map_err(|_| {
					ExecutorError::BackendProtocol {
						task: *task,
						detail: "pending poll names a task absent from the finalized bundle",
					}
				})?;
			self.pending_polls[index]
				.count
				.checked_add(u128::from(*increment))
				.ok_or(ExecutorError::PendingPollCountOverflow { task: *task })?;
		}
		let required = self
			.physical_calls
			.len()
			.checked_add(retained_len)
			.ok_or(ExecutorError::PreparationCapacityOverflow)?;
		if required > self.declared.physical_calls {
			return Err(ExecutorError::JournalCapacityExceeded {
				stream: JournalStream::Physical,
				capacity: self.declared.physical_calls,
			});
		}
		for (task, increment) in pending_batch[..pending_batch_len].iter().flatten() {
			let index = self
				.pending_polls
				.binary_search_by_key(task, |entry| entry.task)
				.map_err(|_| {
					ExecutorError::BackendProtocol {
						task: *task,
						detail: "pending poll names a task absent from the finalized bundle",
					}
				})?;
			self.pending_polls[index].count = self.pending_polls[index]
				.count
				.checked_add(u128::from(*increment))
				.ok_or(ExecutorError::PendingPollCountOverflow { task: *task })?;
		}
		self.physical_calls
			.extend(retained_calls[..retained_len].iter().flatten().copied());
		self.summary.physical_calls_observed = self
			.summary
			.physical_calls_observed
			.saturating_add(calls.len() as u128);
		self.summary.physical_calls_compacted = self
			.summary
			.physical_calls_compacted
			.saturating_add(calls.len().saturating_sub(retained_len) as u128);
		Ok(())
	}

	#[must_use]
	pub fn logical_events(&self) -> &[LogicalEvent] { &self.logical_events }

	#[must_use]
	/// Ordered non-pending calls, terminal polls, and the first pending marker
	/// for each task. Use [`Self::pending_poll_counts`] for exact pending totals.
	pub fn physical_calls(&self) -> &[PhysicalCall] { &self.physical_calls }

	/// Fixed task-indexed pending-poll counters.
	///
	/// [`Self::physical_calls`] retains the first pending marker for each task;
	/// this table preserves the exact number of pending polls across every
	/// activation without growing the ordered journal.
	#[must_use]
	pub fn pending_poll_counts(&self) -> &[PendingPollCount] { &self.pending_polls }

	#[must_use]
	pub fn pending_poll_count(&self, task: TaskId) -> Option<u128> {
		self.pending_polls
			.binary_search_by_key(&task, |entry| entry.task)
			.ok()
			.map(|index| self.pending_polls[index].count)
	}

	#[must_use]
	pub const fn summary(&self) -> JournalSummary { self.summary }

	#[must_use]
	pub const fn declared_capacity(&self) -> JournalCapacity { self.declared }

	#[must_use]
	pub fn allocated_capacity(&self) -> JournalCapacity {
		JournalCapacity::new(
			self.logical_events.capacity(),
			self.physical_calls.capacity(),
		)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeCapacities {
	pub phase_slots: usize,
	pub completion_entries: usize,
	pub logical_journal: usize,
	pub physical_journal: usize,
	pub pending_poll_entries: usize,
}

struct RunCore<B: Backend> {
	run_id: RunId,
	bundle: FinalizedBundle,
	backend: B,
	resource: ResourceState<B::Resource>,
	arenas: BTreeMap<DeviceId, B::Arena>,
	completed: CompletionLedger,
	metrics: MetricMailbox,
	exit_images: Vec<ExitImage>,
	exit_image_capacity: usize,
	journal: RunJournal,
	watchdog: Watchdog,
}

enum ResourceState<R> {
	Active(R),
	Taken,
}

impl<R> ResourceState<R> {
	fn active(&self) -> Result<&R> {
		match self {
			Self::Active(resource) => Ok(resource),
			Self::Taken => {
				Err(ExecutorError::LifecycleInvariant {
					detail: "backend resource was already consumed",
				})
			}
		}
	}

	fn active_mut(&mut self) -> Result<&mut R> {
		match self {
			Self::Active(resource) => Ok(resource),
			Self::Taken => {
				Err(ExecutorError::LifecycleInvariant {
					detail: "backend resource was already consumed",
				})
			}
		}
	}

	fn consume(&mut self) -> Result<R> {
		match core::mem::replace(self, Self::Taken) {
			Self::Active(resource) => Ok(resource),
			Self::Taken => {
				Err(ExecutorError::LifecycleInvariant {
					detail: "backend resource was already consumed",
				})
			}
		}
	}
}

impl<B: Backend> fmt::Debug for RunCore<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("RunCore")
			.field("run_id", &self.run_id)
			.field("bundle", &self.bundle.identity())
			.field("arena_count", &self.arenas.len())
			.field("completed_tasks", &self.completed.completed_count())
			.field("exit_images", &self.exit_images.len())
			.field("journal", &self.journal)
			.field("watchdog", &self.watchdog)
			.finish_non_exhaustive()
	}
}

/// One failed run with native-backend ownership and bounded execution evidence
/// preserved after best-effort teardown.
///
/// `cleanup_error` is the first teardown failure observed after `error`.
/// Teardown still attempts every remaining arena release and resource
/// destruction in lifecycle order.
pub struct RunFailure<B: Backend> {
	run_id: RunId,
	bundle: BundleIdentity,
	error: ExecutorError,
	cleanup_error: Option<ExecutorError>,
	backend: B,
	journal: Option<RunJournal>,
}

impl<B: Backend> fmt::Debug for RunFailure<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("RunFailure")
			.field("run_id", &self.run_id)
			.field("bundle", &self.bundle)
			.field("error", &self.error)
			.field("cleanup_error", &self.cleanup_error)
			.field("journal", &self.journal)
			.finish_non_exhaustive()
	}
}

impl<B: Backend> RunFailure<B> {
	#[must_use]
	pub const fn run_id(&self) -> RunId { self.run_id }

	#[must_use]
	pub const fn bundle_identity(&self) -> BundleIdentity { self.bundle }

	#[must_use]
	pub const fn error(&self) -> ExecutorError { self.error }

	#[must_use]
	pub const fn cleanup_error(&self) -> Option<ExecutorError> { self.cleanup_error }

	#[must_use]
	pub const fn journal(&self) -> Option<&RunJournal> { self.journal.as_ref() }

	#[must_use]
	pub fn into_parts(self) -> RunFailureParts<B> {
		RunFailureParts {
			run_id: self.run_id,
			bundle: self.bundle,
			error: self.error,
			cleanup_error: self.cleanup_error,
			backend: self.backend,
			journal: self.journal,
		}
	}
}

/// Owned components of a failed run, including the recovered backend.
pub struct RunFailureParts<B: Backend> {
	pub run_id: RunId,
	pub bundle: BundleIdentity,
	pub error: ExecutorError,
	pub cleanup_error: Option<ExecutorError>,
	pub backend: B,
	pub journal: Option<RunJournal>,
}

impl<B: Backend> fmt::Debug for RunFailureParts<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("RunFailureParts")
			.field("run_id", &self.run_id)
			.field("bundle", &self.bundle)
			.field("error", &self.error)
			.field("cleanup_error", &self.cleanup_error)
			.field("journal", &self.journal)
			.finish_non_exhaustive()
	}
}

/// A finalized run whose backend resources and every pending token are already
/// realized, but whose arenas and external data images have not been admitted.
pub struct PreparedRun<B: Backend> {
	core: RunCore<B>,
	init_phase: PhaseState<B::Pending>,
	loop_phase: PhaseState<B::Pending>,
	exit_phase: PhaseState<B::Pending>,
	fault_resets: Vec<FaultReset>,
}

impl<B: Backend> fmt::Debug for PreparedRun<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("PreparedRun")
			.field("core", &self.core)
			.field("init_phase", &self.init_phase)
			.field("loop_phase", &self.loop_phase)
			.field("exit_phase", &self.exit_phase)
			.finish()
	}
}

impl<B: Backend> PreparedRun<B> {
	pub fn prepare(run_id: RunId, bundle: FinalizedBundle, backend: B, watchdog: Watchdog) -> Result<Self> {
		Self::prepare_recoverable(run_id, bundle, backend, watchdog).map_err(|failure| failure.error())
	}

	pub fn prepare_recoverable(
		run_id: RunId,
		bundle: FinalizedBundle,
		backend: B,
		watchdog: Watchdog,
	) -> std::result::Result<Self, RunFailure<B>> {
		let journal_capacity = match JournalCapacity::for_bundle::<B>(&bundle) {
			Ok(capacity) => capacity,
			Err(error) => return Err(unstarted_failure(run_id, bundle.identity(), backend, error)),
		};
		Self::prepare_with_journal_capacity_recoverable(run_id, bundle, backend, watchdog, journal_capacity)
	}

	pub fn prepare_with_journal_capacity(
		run_id: RunId,
		bundle: FinalizedBundle,
		backend: B,
		watchdog: Watchdog,
		journal_capacity: JournalCapacity,
	) -> Result<Self> {
		Self::prepare_with_journal_capacity_recoverable(run_id, bundle, backend, watchdog, journal_capacity)
			.map_err(|failure| failure.error())
	}

	pub fn prepare_with_journal_capacity_recoverable(
		run_id: RunId,
		bundle: FinalizedBundle,
		mut backend: B,
		watchdog: Watchdog,
		journal_capacity: JournalCapacity,
	) -> std::result::Result<Self, RunFailure<B>> {
		let bundle_identity = bundle.identity();
		if (bundle.loop_iterations().is_unbounded()
			|| bundle
				.loop_iterations()
				.finite()
				.is_some_and(|iterations| iterations.get() > 1))
			&& !backend.supports_loop_repetition()
		{
			return Err(unstarted_failure(
				run_id,
				bundle_identity,
				backend,
				ExecutorError::LoopRepetitionUnsupported {
					iterations: bundle.loop_iterations(),
				},
			));
		}
		let mut journal = RunJournal::with_loop_detail(journal_capacity, bundle.tasks(), false);
		let prepared = match PreparedPhases::new(&bundle) {
			Ok(prepared) => prepared,
			Err(error) => {
				return Err(RunFailure {
					run_id,
					bundle: bundle_identity,
					error,
					cleanup_error: None,
					backend,
					journal: Some(journal),
				});
			}
		};
		let completed = CompletionLedger::new(bundle.tasks());
		let exit_image_capacity = prepared.exit.external_exit_count();
		let fault_resets = prepared.fault_resets();
		let mut physical_calls = PhysicalCallBatch::new();
		let bind_result = backend.bind_resources(&bundle, &mut physical_calls);
		let mut resource = match backend_value(
			&mut journal,
			BackendOperation::BindResources,
			physical_calls,
			bind_result,
		) {
			Ok(resource) => resource,
			Err(error) => {
				return Err(RunFailure {
					run_id,
					bundle: bundle_identity,
					error,
					cleanup_error: None,
					backend,
					journal: Some(journal),
				});
			}
		};
		let init_phase = match realize_phase(
			prepared.init,
			RunPhase::Init,
			&mut backend,
			&mut resource,
			&mut journal,
		) {
			Ok(phase) => phase,
			Err(error) => {
				return Err(prepared_resource_failure(
					run_id,
					bundle_identity,
					backend,
					resource,
					journal,
					error,
				));
			}
		};
		let loop_phase = match realize_phase(
			prepared.loop_phase,
			RunPhase::Loop,
			&mut backend,
			&mut resource,
			&mut journal,
		) {
			Ok(phase) => phase,
			Err(error) => {
				drop(init_phase);
				return Err(prepared_resource_failure(
					run_id,
					bundle_identity,
					backend,
					resource,
					journal,
					error,
				));
			}
		};
		let exit_phase = match realize_phase(
			prepared.exit,
			RunPhase::Exit,
			&mut backend,
			&mut resource,
			&mut journal,
		) {
			Ok(phase) => phase,
			Err(error) => {
				drop((init_phase, loop_phase));
				return Err(prepared_resource_failure(
					run_id,
					bundle_identity,
					backend,
					resource,
					journal,
					error,
				));
			}
		};
		let metrics = MetricMailbox::new(&bundle.resources().metrics, bundle.tasks());
		if let Err(error) = journal.record_logical(LogicalEvent::Prepared {
			run: run_id,
			bundle: bundle_identity,
		}) {
			drop((init_phase, loop_phase, exit_phase));
			return Err(prepared_resource_failure(
				run_id,
				bundle_identity,
				backend,
				resource,
				journal,
				error,
			));
		}
		Ok(Self {
			core: RunCore {
				run_id,
				bundle,
				backend,
				resource: ResourceState::Active(resource),
				arenas: BTreeMap::new(),
				completed,
				metrics,
				exit_images: Vec::with_capacity(exit_image_capacity),
				exit_image_capacity,
				journal,
				watchdog,
			},
			init_phase,
			loop_phase,
			exit_phase,
			fault_resets,
		})
	}

	pub fn initialize(self, images: impl IntoIterator<Item = DeviceImage>) -> Result<InitializedRun<B>> {
		self.initialize_recoverable(images)
			.map_err(|failure| failure.error())
	}

	pub fn initialize_recoverable(
		mut self,
		images: impl IntoIterator<Item = DeviceImage>,
	) -> std::result::Result<InitializedRun<B>, RunFailure<B>> {
		let images = match validate_images(&self.core.bundle, images, &self.fault_resets) {
			Ok(images) => images,
			Err(error) => return Err(self.into_failure(error)),
		};
		for layout_index in 0..self.core.bundle.arena_layouts().len() {
			let layout = &self.core.bundle.arena_layouts()[layout_index];
			let device = layout.device;
			let bytes = layout.size;
			let operation = BackendOperation::AllocateArena { device };
			let mut physical_calls = PhysicalCallBatch::new();
			let result = match self.core.resource.active_mut() {
				Ok(resource) => {
					self.core
						.backend
						.allocate_arena(resource, layout, &mut physical_calls)
				}
				Err(error) => return Err(self.into_failure(error)),
			};
			let arena = match backend_value(&mut self.core.journal, operation, physical_calls, result) {
				Ok(arena) => arena,
				Err(error) => return Err(self.into_failure(error)),
			};
			let previous = self.core.arenas.insert(device, arena);
			debug_assert!(previous.is_none(), "finalized layouts have unique devices");
			if let Err(error) = self
				.core
				.journal
				.record_logical(LogicalEvent::ArenaAllocated { device, bytes })
			{
				return Err(self.into_failure(error));
			}
		}

		if let Err(error) = run_phase_blocking(&mut self.core, &mut self.init_phase, Some(&images)) {
			return Err(self.into_failure(error));
		}
		if let Err(error) = self.core.journal.record_logical(LogicalEvent::Initialized {
			run: self.core.run_id,
		}) {
			return Err(self.into_failure(error));
		}
		Ok(InitializedRun {
			core: self.core,
			loop_phase: self.loop_phase,
			exit_phase: self.exit_phase,
		})
	}

	fn into_failure(self, error: ExecutorError) -> RunFailure<B> {
		let Self {
			core,
			init_phase,
			loop_phase,
			exit_phase,
			fault_resets: _,
		} = self;
		drop((init_phase, loop_phase, exit_phase));
		failed_core(core, error)
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal { &self.core.journal }
}

/// A run whose fixed arenas exist and whose single logical data image per
/// device has reached terminal completion.
pub struct InitializedRun<B: Backend> {
	core: RunCore<B>,
	loop_phase: PhaseState<B::Pending>,
	exit_phase: PhaseState<B::Pending>,
}

impl<B: Backend> fmt::Debug for InitializedRun<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("InitializedRun")
			.field("core", &self.core)
			.field("loop_phase", &self.loop_phase)
			.field("exit_phase", &self.exit_phase)
			.finish()
	}
}

impl<B: Backend> InitializedRun<B> {
	pub fn start_loop(self) -> Result<RunningRun<B>> {
		self.start_loop_recoverable()
			.map_err(|failure| failure.error())
	}

	pub fn start_loop_recoverable(mut self) -> std::result::Result<RunningRun<B>, RunFailure<B>> {
		let iteration = match self.core.bundle.loop_iterations().iteration(0) {
			Some(iteration) => iteration,
			None => {
				return Err(self.into_failure(ExecutorError::LifecycleInvariant {
					detail: "finalized loop count did not contain iteration zero",
				}));
			}
		};
		self.loop_phase.begin_loop_iteration(iteration);
		if let Err(error) = self.core.journal.record_logical(LogicalEvent::LoopStarted {
			run: self.core.run_id,
		}) {
			return Err(self.into_failure(error));
		}
		if let Err(error) = self
			.core
			.journal
			.record_logical(LogicalEvent::LoopIterationStarted {
				run: self.core.run_id,
				iteration,
			}) {
			return Err(self.into_failure(error));
		}
		Ok(RunningRun {
			core: self.core,
			phase: self.loop_phase,
			exit_phase: self.exit_phase,
			failure: None,
			completion_recorded: false,
		})
	}

	fn into_failure(self, error: ExecutorError) -> RunFailure<B> {
		let Self {
			core,
			loop_phase,
			exit_phase,
		} = self;
		drop((loop_phase, exit_phase));
		failed_core(core, error)
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal { &self.core.journal }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopStatus {
	Pending,
	Complete,
}

/// The only live-loop capabilities: nonblocking progress, bounded waiting,
/// latest-value metric consumption, and journal inspection.
///
/// Every task slot, dependency bit, backend pending token, work slice, and
/// journal allocation reachable by [`Self::poll`] is fixed before `init`.
pub struct RunningRun<B: Backend> {
	core: RunCore<B>,
	phase: PhaseState<B::Pending>,
	exit_phase: PhaseState<B::Pending>,
	failure: Option<ExecutorError>,
	completion_recorded: bool,
}

impl<B: Backend> fmt::Debug for RunningRun<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("RunningRun")
			.field("core", &self.core)
			.field("phase", &self.phase)
			.field("failure", &self.failure)
			.finish()
	}
}

impl<B: Backend> RunningRun<B> {
	/// Performs one bounded, nonblocking scheduler/poll pass.
	pub fn poll(&mut self) -> Result<LoopStatus> { self.poll_with_progress().map(|(status, _)| status) }

	/// Performs one bounded, nonblocking scheduler/poll pass and reports
	/// whether that pass submitted or completed any work.
	pub fn poll_with_progress(&mut self) -> Result<(LoopStatus, bool)> { self.poll_with_progress_or_stop(|| false) }

	/// Performs one bounded scheduler/poll pass and accepts a graceful stop
	/// only after the active loop iteration reaches terminal completion.
	///
	/// `stop_requested` is evaluated at that safe boundary, immediately before
	/// another iteration could begin. An accepted stop preserves the completed
	/// iteration's state and makes [`Self::into_exited_loop`] legal.
	pub fn poll_with_progress_or_stop(
		&mut self,
		stop_requested: impl FnOnce() -> bool,
	) -> Result<(LoopStatus, bool)> {
		let failure_check = self.failure.map(Err::<(), ExecutorError>).transpose()?;
		debug_assert!(failure_check.is_none());
		match poll_phase_once(&mut self.core, &mut self.phase, None) {
			Ok(PhasePoll {
				complete: true,
				made_progress,
			}) => {
				let false = self.completion_recorded else {
					return Ok((LoopStatus::Complete, made_progress));
				};
				let iteration = self
					.phase
					.loop_iteration
					.ok_or(ExecutorError::LifecycleInvariant {
						detail: "completed loop phase has no active iteration",
					})?;
				self.core
					.journal
					.record_logical(LogicalEvent::LoopIterationCompleted {
						run: self.core.run_id,
						iteration,
					})?;
				let stop_requested = stop_requested();
				let next =
					if stop_requested {
						None
					} else {
						let next_index = iteration.index().checked_add(1).ok_or(
							ExecutorError::LifecycleInvariant {
								detail: "loop iteration index exhausted the u64 coordinate space",
							},
						)?;
						iteration.total().iteration(next_index)
					};
				if let Some(next) = next {
					self.core.completed.reset_phase(RunPhase::Loop);
					self.phase.begin_loop_iteration(next);
					self.core
						.journal
						.record_logical(LogicalEvent::LoopIterationStarted {
							run: self.core.run_id,
							iteration: next,
						})?;
					return Ok((LoopStatus::Pending, true));
				}
				if stop_requested {
					self.core
						.journal
						.record_logical(LogicalEvent::LoopStopAccepted {
							run: self.core.run_id,
							after_iteration: iteration,
						})?;
				}
				self.core
					.journal
					.record_logical(LogicalEvent::LoopCompleted {
						run: self.core.run_id,
					})?;
				self.completion_recorded = true;
				Ok((LoopStatus::Complete, true))
			}
			Ok(PhasePoll {
				complete: false,
				made_progress,
			}) => Ok((LoopStatus::Pending, made_progress)),
			Err(error) => {
				let reported = match self.core.journal.record_logical(LogicalEvent::LoopFailed {
					run: self.core.run_id,
				}) {
					Ok(()) => error,
					Err(journal_error) => journal_error,
				};
				self.failure = Some(reported);
				Err(reported)
			}
		}
	}

	/// Drives nonblocking polls until completion or a bounded failure.
	pub fn wait(&mut self) -> Result<()> {
		let mut backoff = BlockingPollBackoff::new();
		loop {
			let (status, made_progress) = self.poll_with_progress()?;
			if status == LoopStatus::Complete {
				return Ok(());
			}
			match made_progress {
				true => backoff.reset(),
				false => backoff.wait(),
			}
		}
	}

	pub fn try_take_metric(&mut self, slot: MetricSlotId) -> Option<MetricSample> { self.core.metrics.try_take(slot) }

	#[must_use]
	pub fn metric_mailbox(&self) -> &MetricMailbox { &self.core.metrics }

	#[must_use]
	pub fn journal(&self) -> &RunJournal { &self.core.journal }

	#[must_use]
	pub fn capacities(&self) -> RuntimeCapacities {
		let journal = self.core.journal.allocated_capacity();
		RuntimeCapacities {
			phase_slots: self.phase.slots.capacity(),
			completion_entries: self.core.completed.entries.capacity(),
			logical_journal: journal.logical_events,
			physical_journal: journal.physical_calls,
			pending_poll_entries: self.core.journal.pending_polls.capacity(),
		}
	}

	#[must_use]
	pub fn current_iteration(&self) -> LoopIteration {
		self.phase
			.loop_iteration
			.expect("a running loop always has one active iteration")
	}

	pub fn into_exited_loop(self) -> std::result::Result<ExitedLoop<B>, Box<Self>> {
		match self.phase.complete && self.failure.is_none() {
			true => {
				Ok(ExitedLoop {
					core: self.core,
					exit_phase: self.exit_phase,
				})
			}
			false => Err(Box::new(self)),
		}
	}

	/// Ends a run that cannot legally reach [`ExitedLoop`], preserving its
	/// primary failure and attempting ordered native teardown.
	#[must_use]
	pub fn fail(self, error: ExecutorError) -> RunFailure<B> {
		let Self {
			core,
			phase,
			exit_phase,
			failure: _,
			completion_recorded: _,
		} = self;
		drop((phase, exit_phase));
		failed_core(core, error)
	}
}

/// A run with no remaining loop work. Exit transfers and arena teardown are now
/// legal, while loop ingress remains impossible.
pub struct ExitedLoop<B: Backend> {
	core: RunCore<B>,
	exit_phase: PhaseState<B::Pending>,
}

impl<B: Backend> fmt::Debug for ExitedLoop<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("ExitedLoop")
			.field("core", &self.core)
			.field("exit_phase", &self.exit_phase)
			.finish()
	}
}

impl<B: Backend> ExitedLoop<B> {
	pub fn exit(self) -> Result<ExitedRun<B>> { self.exit_recoverable().map_err(|failure| failure.error()) }

	pub fn exit_recoverable(mut self) -> std::result::Result<ExitedRun<B>, RunFailure<B>> {
		if let Err(error) = run_phase_blocking(&mut self.core, &mut self.exit_phase, None) {
			let Self { core, exit_phase } = self;
			drop(exit_phase);
			return Err(failed_core(core, error));
		}
		let Self {
			mut core,
			exit_phase,
		} = self;
		drop(exit_phase);
		let (error, cleanup_error) = teardown_resources(&mut core);
		if let Some(error) = error {
			return Err(run_failure(core, error, cleanup_error));
		}
		if let Err(error) = core
			.journal
			.record_logical(LogicalEvent::Exited { run: core.run_id })
		{
			return Err(run_failure(core, error, None));
		}

		Ok(ExitedRun {
			run_id: core.run_id,
			bundle: core.bundle.identity(),
			backend: core.backend,
			metrics: core.metrics,
			exit_images: core.exit_images,
			journal: core.journal,
		})
	}

	pub fn try_take_metric(&mut self, slot: MetricSlotId) -> Option<MetricSample> { self.core.metrics.try_take(slot) }

	#[must_use]
	pub fn metric_mailbox(&self) -> &MetricMailbox { &self.core.metrics }

	#[must_use]
	pub fn journal(&self) -> &RunJournal { &self.core.journal }
}

/// Fully exited run. Backend ownership and external result images may now be
/// recovered for a subsequent lifecycle or caller-controlled persistence.
pub struct ExitedRun<B: Backend> {
	run_id: RunId,
	bundle: BundleIdentity,
	backend: B,
	metrics: MetricMailbox,
	exit_images: Vec<ExitImage>,
	journal: RunJournal,
}

impl<B: Backend> fmt::Debug for ExitedRun<B> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("ExitedRun")
			.field("run_id", &self.run_id)
			.field("bundle", &self.bundle)
			.field("metrics", &self.metrics)
			.field("exit_images", &self.exit_images)
			.field("journal", &self.journal)
			.finish_non_exhaustive()
	}
}

impl<B: Backend> ExitedRun<B> {
	#[must_use]
	pub const fn run_id(&self) -> RunId { self.run_id }

	#[must_use]
	pub const fn bundle_identity(&self) -> BundleIdentity { self.bundle }

	#[must_use]
	pub fn journal(&self) -> &RunJournal { &self.journal }

	#[must_use]
	pub fn exit_images(&self) -> &[ExitImage] { &self.exit_images }

	pub fn try_take_metric(&mut self, slot: MetricSlotId) -> Option<MetricSample> { self.metrics.try_take(slot) }

	pub fn into_parts(self) -> (B, MetricMailbox, Vec<ExitImage>, RunJournal) {
		(self.backend, self.metrics, self.exit_images, self.journal)
	}
}

fn unstarted_failure<B: Backend>(
	run_id: RunId,
	bundle: BundleIdentity,
	backend: B,
	error: ExecutorError,
) -> RunFailure<B> {
	RunFailure {
		run_id,
		bundle,
		error,
		cleanup_error: None,
		backend,
		journal: None,
	}
}

fn prepared_resource_failure<B: Backend>(
	run_id: RunId,
	bundle: BundleIdentity,
	mut backend: B,
	resource: B::Resource,
	mut journal: RunJournal,
	error: ExecutorError,
) -> RunFailure<B> {
	let mut physical_calls = PhysicalCallBatch::new();
	let result = backend.destroy_resources(resource, &mut physical_calls);
	let cleanup_error = backend_value(
		&mut journal,
		BackendOperation::DestroyResources,
		physical_calls,
		result,
	)
	.err();
	RunFailure {
		run_id,
		bundle,
		error,
		cleanup_error,
		backend,
		journal: Some(journal),
	}
}

fn failed_core<B: Backend>(mut core: RunCore<B>, error: ExecutorError) -> RunFailure<B> {
	let (cleanup_error, _) = teardown_resources(&mut core);
	run_failure(core, error, cleanup_error)
}

fn run_failure<B: Backend>(
	core: RunCore<B>,
	error: ExecutorError,
	cleanup_error: Option<ExecutorError>,
) -> RunFailure<B> {
	RunFailure {
		run_id: core.run_id,
		bundle: core.bundle.identity(),
		error,
		cleanup_error,
		backend: core.backend,
		journal: Some(core.journal),
	}
}

fn teardown_resources<B: Backend>(core: &mut RunCore<B>) -> (Option<ExecutorError>, Option<ExecutorError>) {
	let mut error = None;
	let mut cleanup_error = None;
	let arenas = core::mem::take(&mut core.arenas);
	for (device, arena) in arenas {
		let mut physical_calls = PhysicalCallBatch::new();
		let result = match core.resource.active_mut() {
			Ok(resource) => {
				core.backend
					.release_arena(resource, device, arena, &mut physical_calls)
			}
			Err(reported) => {
				record_teardown_error(&mut error, &mut cleanup_error, reported);
				continue;
			}
		};
		match backend_value(
			&mut core.journal,
			BackendOperation::ReleaseArena { device },
			physical_calls,
			result,
		) {
			Ok(()) => {
				if let Err(reported) = core
					.journal
					.record_logical(LogicalEvent::ArenaReleased { device })
				{
					record_teardown_error(&mut error, &mut cleanup_error, reported);
				}
			}
			Err(reported) => record_teardown_error(&mut error, &mut cleanup_error, reported),
		}
	}

	match core.resource.consume() {
		Ok(resource) => {
			let mut physical_calls = PhysicalCallBatch::new();
			let result = core
				.backend
				.destroy_resources(resource, &mut physical_calls);
			if let Err(reported) = backend_value(
				&mut core.journal,
				BackendOperation::DestroyResources,
				physical_calls,
				result,
			) {
				record_teardown_error(&mut error, &mut cleanup_error, reported);
			}
		}
		Err(reported) => record_teardown_error(&mut error, &mut cleanup_error, reported),
	}
	(error, cleanup_error)
}

fn record_teardown_error(
	error: &mut Option<ExecutorError>,
	cleanup_error: &mut Option<ExecutorError>,
	reported: ExecutorError,
) {
	if error.is_none() {
		*error = Some(reported);
	} else if cleanup_error.is_none() {
		*cleanup_error = Some(reported);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotStatus {
	Remaining,
	Pending,
	Complete,
}

struct TaskSlot<P> {
	task: PreparedTask,
	pending: P,
	status: SlotStatus,
}

impl<P: fmt::Debug> fmt::Debug for TaskSlot<P> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("TaskSlot")
			.field("task", &self.task.id)
			.field("pending", &self.pending)
			.field("status", &self.status)
			.finish()
	}
}

struct PhaseState<P> {
	phase: RunPhase,
	slots: Vec<TaskSlot<P>>,
	nonprogress_polls: u32,
	complete: bool,
	loop_iteration: Option<LoopIteration>,
}

impl<P: fmt::Debug> fmt::Debug for PhaseState<P> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("PhaseState")
			.field("phase", &self.phase)
			.field("slot_count", &self.slots.len())
			.field("nonprogress_polls", &self.nonprogress_polls)
			.field("complete", &self.complete)
			.field("loop_iteration", &self.loop_iteration)
			.finish()
	}
}

impl<P> PhaseState<P> {
	fn begin_loop_iteration(&mut self, iteration: LoopIteration) {
		debug_assert_eq!(self.phase, RunPhase::Loop);
		debug_assert!(
			self.loop_iteration.is_none()
				|| self
					.slots
					.iter()
					.all(|slot| slot.status == SlotStatus::Complete),
			"a subsequent loop iteration begins only after every slot completed"
		);
		for slot in &mut self.slots {
			slot.status = SlotStatus::Remaining;
		}
		self.nonprogress_polls = 0;
		self.complete = self.slots.is_empty();
		self.loop_iteration = Some(iteration);
	}
}

#[derive(Clone, Debug)]
struct PreparedTask {
	id: TaskId,
	window: ScheduleWindow,
	dependencies: Vec<TaskId>,
	iteration_domain: Option<IterationDomain>,
	work: PreparedWork,
}

#[derive(Clone, Debug)]
enum PreparedWork {
	InitAdmission {
		device: DeviceId,
		destination: ResolvedValueLocation,
		bytes: ByteCount,
		submission: SubmissionSlots,
	},
	Calculation {
		device: DeviceId,
		kernel_template: KernelTemplateId,
		artifact: ArtifactId,
		submission: SubmissionSlots,
		inputs: Vec<ResolvedValueLocation>,
		outputs: Vec<ResolvedValueLocation>,
		fault_flag: Option<ResolvedValueLocation>,
	},
	Transfer {
		class: WorkClass,
		source: ResolvedTransferEndpoint,
		destination: ResolvedTransferEndpoint,
		bytes: ByteCount,
		route: Vec<LinkId>,
		lane_claims: Vec<TransferLaneClaim>,
		submission: SubmissionSlots,
	},
	Metric {
		purpose: MetricPurpose,
		metric: MetricId,
		slot: MetricSlotId,
		value: ResolvedValueLocation,
		submission: SubmissionSlots,
	},
}

impl PreparedTask {
	fn new(bundle: &FinalizedBundle, task: &Task) -> Result<Self> {
		let iteration_domain = match task.phase {
			RunPhase::Loop => {
				Some(bundle
					.iteration_domain(task.id)
					.ok_or(ExecutorError::LifecycleInvariant {
						detail: "finalized loop task has no iteration domain",
					})?)
			}
			RunPhase::Init | RunPhase::Exit => None,
		};
		let work = match (&task.kind, task.phase) {
			(TaskKind::Calculation(calculation), RunPhase::Loop) => {
				let inputs = resolve_values(bundle, task.id, &calculation.inputs)?;
				let outputs = resolve_values(bundle, task.id, &calculation.outputs)?;
				let fault_flag = calculation
					.fault_flag
					.map(|value| resolve_value(bundle, task.id, value))
					.transpose()?;
				PreparedWork::Calculation {
					device: calculation.device,
					kernel_template: calculation.kernel_template,
					artifact: calculation.artifact,
					submission: calculation.submission,
					inputs,
					outputs,
					fault_flag,
				}
			}
			(TaskKind::Metric(metric), RunPhase::Loop) => {
				PreparedWork::Metric {
					purpose: metric.purpose,
					metric: metric.metric,
					slot: metric.slot,
					value: resolve_value(bundle, task.id, metric.value)?,
					submission: metric.submission,
				}
			}
			(TaskKind::Transfer(transfer), RunPhase::Init)
				if matches!(
					(transfer.source, transfer.destination),
					(TransferEndpoint::External, TransferEndpoint::Device { .. })
				) =>
			{
				let TransferEndpoint::Device { device, .. } = transfer.destination else {
					unreachable!("guard established device destination");
				};
				let endpoints = resolve_transfer_endpoints(bundle, task.id)?;
				let ResolvedTransferEndpoint::Device(destination) = endpoints.destination else {
					return Err(ExecutorError::BackendProtocol {
						task: task.id,
						detail: "finalized admission has no resolved device destination",
					});
				};
				PreparedWork::InitAdmission {
					device,
					destination,
					bytes: transfer.bytes,
					submission: transfer.submission,
				}
			}
			(TaskKind::Transfer(transfer), RunPhase::Init | RunPhase::Loop)
				if matches!(
					(transfer.source, transfer.destination),
					(
						TransferEndpoint::Device { .. },
						TransferEndpoint::Device { .. }
					)
				) =>
			{
				let endpoints = resolve_transfer_endpoints(bundle, task.id)?;
				PreparedWork::Transfer {
					class: WorkClass::InternalTransfer,
					source: endpoints.source,
					destination: endpoints.destination,
					bytes: transfer.bytes,
					route: transfer.route.clone(),
					lane_claims: transfer.lane_claims.clone(),
					submission: transfer.submission,
				}
			}
			(TaskKind::Transfer(transfer), RunPhase::Exit)
				if matches!(
					(transfer.source, transfer.destination),
					(
						TransferEndpoint::Device { .. },
						TransferEndpoint::Device { .. }
					) | (TransferEndpoint::Device { .. }, TransferEndpoint::External)
				) =>
			{
				let endpoints = resolve_transfer_endpoints(bundle, task.id)?;
				PreparedWork::Transfer {
					class: WorkClass::ExitTransfer,
					source: endpoints.source,
					destination: endpoints.destination,
					bytes: transfer.bytes,
					route: transfer.route.clone(),
					lane_claims: transfer.lane_claims.clone(),
					submission: transfer.submission,
				}
			}
			(TaskKind::Calculation(_), _) => {
				return Err(ExecutorError::InvalidPhaseTask {
					phase: task.phase,
					task: task.id,
					detail: "calculations are legal only in the loop",
				});
			}
			(TaskKind::Metric(_), _) => {
				return Err(ExecutorError::InvalidPhaseTask {
					phase: task.phase,
					task: task.id,
					detail: "metrics are legal only in the loop",
				});
			}
			(TaskKind::Transfer(_), RunPhase::Loop) => {
				return Err(ExecutorError::InvalidPhaseTask {
					phase: task.phase,
					task: task.id,
					detail: "loop transfers must be internal",
				});
			}
			(TaskKind::Transfer(_), RunPhase::Init) => {
				return Err(ExecutorError::InvalidPhaseTask {
					phase: task.phase,
					task: task.id,
					detail: "init transfer is neither admission nor internal movement",
				});
			}
			(TaskKind::Transfer(_), RunPhase::Exit) => {
				return Err(ExecutorError::InvalidPhaseTask {
					phase: task.phase,
					task: task.id,
					detail: "exit cannot admit external data",
				});
			}
		};
		Ok(Self {
			id: task.id,
			window: task.window,
			dependencies: task.dependencies.clone(),
			iteration_domain,
			work,
		})
	}

	fn active_on(&self, iteration: LoopIteration) -> bool {
		self.iteration_domain
			.is_some_and(|domain| domain.contains(iteration.index()))
	}

	const fn class(&self) -> WorkClass {
		match self.work {
			PreparedWork::InitAdmission { .. } => WorkClass::InitAdmission,
			PreparedWork::Calculation { .. } => WorkClass::Calculation,
			PreparedWork::Transfer { class, .. } => class,
			PreparedWork::Metric { .. } => WorkClass::Metric,
		}
	}

	const fn submission(&self) -> Option<SubmissionSlots> {
		match self.work {
			PreparedWork::InitAdmission { submission, .. }
			| PreparedWork::Calculation { submission, .. }
			| PreparedWork::Transfer { submission, .. }
			| PreparedWork::Metric { submission, .. } => Some(submission),
		}
	}

	fn backend_work<'a>(
		&'a self,
		run: RunId,
		iteration: Option<LoopIteration>,
		images: Option<&'a BTreeMap<InitImageKey, Vec<u8>>>,
	) -> Result<BackendWork<'a>> {
		match &self.work {
			PreparedWork::InitAdmission {
				device,
				destination,
				bytes,
				submission,
			} => {
				let images = images.ok_or(ExecutorError::MissingAdmission { device: *device })?;
				let key = InitImageKey {
					device: *device,
					image: destination.value,
					bytes: *bytes,
				};
				let image = images
					.get(&key)
					.ok_or(ExecutorError::MissingAdmission { device: *device })?;
				Ok(BackendWork::InitAdmission(InitAdmissionWork {
					task: self.id,
					destination: *destination,
					bytes: *bytes,
					submission: *submission,
					image,
				}))
			}
			PreparedWork::Calculation {
				device,
				kernel_template,
				artifact,
				submission,
				inputs,
				outputs,
				fault_flag,
			} => {
				let iteration = iteration.ok_or(ExecutorError::LifecycleInvariant {
					detail: "calculation work has no active loop iteration",
				})?;
				Ok(BackendWork::Calculation(CalculationWork {
					task: self.id,
					run,
					iteration,
					device: *device,
					kernel_template: *kernel_template,
					artifact: *artifact,
					submission: *submission,
					inputs,
					outputs,
					fault_flag: *fault_flag,
				}))
			}
			PreparedWork::Transfer {
				class,
				source,
				destination,
				bytes,
				route,
				lane_claims,
				submission,
			} => {
				let transfer = TransferWork {
					task: self.id,
					source: *source,
					destination: *destination,
					bytes: *bytes,
					route,
					lane_claims,
					submission: *submission,
				};
				match class {
					WorkClass::InternalTransfer => Ok(BackendWork::InternalTransfer(transfer)),
					WorkClass::ExitTransfer => Ok(BackendWork::ExitTransfer(transfer)),
					_ => unreachable!("prepared transfer has a transfer work class"),
				}
			}
			PreparedWork::Metric {
				purpose,
				metric,
				slot,
				value,
				submission,
			} => {
				let iteration = iteration.ok_or(ExecutorError::LifecycleInvariant {
					detail: "metric work has no active loop iteration",
				})?;
				Ok(BackendWork::Metric(MetricWork {
					task: self.id,
					iteration,
					purpose: *purpose,
					metric: *metric,
					slot: *slot,
					value: *value,
					submission: *submission,
				}))
			}
		}
	}

	fn external_exit(&self) -> Option<(ResolvedValueLocation, ByteCount)> {
		match self.work {
			PreparedWork::Transfer {
				class: WorkClass::ExitTransfer,
				source: ResolvedTransferEndpoint::Device(source),
				destination: ResolvedTransferEndpoint::External,
				bytes,
				..
			} => Some((source, bytes)),
			_ => None,
		}
	}

	fn fault_reset(&self) -> Option<FaultReset> {
		match self.work {
			PreparedWork::Calculation {
				fault_flag: Some(location),
				..
			} => {
				Some(FaultReset {
					task: self.id,
					location,
				})
			}
			_ => None,
		}
	}
}

#[derive(Debug)]
struct PreparedPhase {
	tasks: Vec<PreparedTask>,
}

impl PreparedPhase {
	fn new(bundle: &FinalizedBundle, phase: RunPhase) -> Result<Self> {
		let mut tasks = bundle
			.tasks()
			.iter()
			.filter(|task| task.phase == phase)
			.map(|task| PreparedTask::new(bundle, task))
			.collect::<Result<Vec<_>>>()?;
		tasks.sort_by_key(|task| (task.window.start, task.id));
		Ok(Self { tasks })
	}

	fn external_exit_count(&self) -> usize {
		self.tasks
			.iter()
			.filter(|task| task.external_exit().is_some())
			.count()
	}
}

#[derive(Debug)]
struct PreparedPhases {
	init: PreparedPhase,
	loop_phase: PreparedPhase,
	exit: PreparedPhase,
}

impl PreparedPhases {
	fn new(bundle: &FinalizedBundle) -> Result<Self> {
		Ok(Self {
			init: PreparedPhase::new(bundle, RunPhase::Init)?,
			loop_phase: PreparedPhase::new(bundle, RunPhase::Loop)?,
			exit: PreparedPhase::new(bundle, RunPhase::Exit)?,
		})
	}

	fn fault_resets(&self) -> Vec<FaultReset> {
		let mut resets = self
			.loop_phase
			.tasks
			.iter()
			.filter_map(PreparedTask::fault_reset)
			.collect::<Vec<_>>();
		resets.sort_by_key(|reset| reset.location.value);
		resets.dedup_by_key(|reset| reset.location.value);
		resets
	}
}

#[derive(Clone, Copy, Debug)]
struct FaultReset {
	task: TaskId,
	location: ResolvedValueLocation,
}

#[derive(Debug)]
struct CompletionEntry {
	task: TaskId,
	phase: RunPhase,
	complete: bool,
}

#[derive(Debug)]
struct CompletionLedger {
	entries: Vec<CompletionEntry>,
}

impl CompletionLedger {
	fn new(tasks: &[Task]) -> Self {
		let mut entries = tasks
			.iter()
			.map(|task| {
				CompletionEntry {
					task: task.id,
					phase: task.phase,
					complete: false,
				}
			})
			.collect::<Vec<_>>();
		entries.sort_by_key(|entry| entry.task);
		Self { entries }
	}

	fn contains(&self, task: TaskId) -> bool {
		self.entries
			.binary_search_by_key(&task, |entry| entry.task)
			.is_ok_and(|index| self.entries[index].complete)
	}

	fn mark(&mut self, task: TaskId) -> Result<()> {
		let index = self
			.entries
			.binary_search_by_key(&task, |entry| entry.task)
			.map_err(|search_index| {
				debug_assert!(search_index <= self.entries.len());
				ExecutorError::BackendProtocol {
					task,
					detail: "completion names a task outside the fixed ledger",
				}
			})?;
		self.entries[index].complete = true;
		Ok(())
	}

	fn completed_count(&self) -> usize { self.entries.iter().filter(|entry| entry.complete).count() }

	fn reset_phase(&mut self, phase: RunPhase) {
		for entry in &mut self.entries {
			if entry.phase == phase {
				entry.complete = false;
			}
		}
	}
}

fn realize_phase<B: Backend>(
	prepared: PreparedPhase,
	phase: RunPhase,
	backend: &mut B,
	resource: &mut B::Resource,
	journal: &mut RunJournal,
) -> Result<PhaseState<B::Pending>> {
	let mut slots = Vec::with_capacity(prepared.tasks.len());
	for task in prepared.tasks {
		let request = PendingRequest {
			task: task.id,
			phase,
			class: task.class(),
			submission: task.submission(),
		};
		let mut physical_calls = PhysicalCallBatch::new();
		let result = backend.prepare_pending(resource, request, &mut physical_calls);
		let pending = backend_value(
			journal,
			BackendOperation::PreparePending { task: task.id },
			physical_calls,
			result,
		)?;
		slots.push(TaskSlot {
			task,
			pending,
			status: SlotStatus::Remaining,
		});
	}
	let complete = slots.is_empty();
	Ok(PhaseState {
		phase,
		slots,
		nonprogress_polls: 0,
		complete,
		loop_iteration: None,
	})
}

fn validate_images(
	bundle: &FinalizedBundle,
	images: impl IntoIterator<Item = DeviceImage>,
	fault_resets: &[FaultReset],
) -> Result<BTreeMap<InitImageKey, Vec<u8>>> {
	let mut provided = BTreeMap::new();
	for image in images {
		let device = image.device;
		let None = provided.insert(device, image) else {
			return Err(ExecutorError::DuplicateAdmission { device });
		};
	}

	let mut expected = BTreeMap::new();
	for manifest in bundle.init_images() {
		let key = InitImageKey {
			device: manifest.device,
			image: manifest.image,
			bytes: manifest.bytes,
		};
		let None = expected.insert(manifest.device, key) else {
			return Err(ExecutorError::DuplicateAdmission {
				device: manifest.device,
			});
		};
	}

	for layout in bundle.arena_layouts() {
		expected
			.get(&layout.device)
			.ok_or(ExecutorError::MissingAdmission {
				device: layout.device,
			})?;
	}

	let mut validated = BTreeMap::new();
	for (device, key) in &expected {
		let supplied = provided
			.remove(device)
			.ok_or(ExecutorError::MissingAdmission { device: *device })?;
		if supplied.image != key.image {
			return Err(ExecutorError::AdmissionImageMismatch {
				device: *device,
				expected: key.image,
				actual: supplied.image,
			});
		}
		let actual = ByteCount::new(
			u64::try_from(supplied.bytes.len()).map_err(|conversion_error| {
				debug_assert!(
					false,
					"host image length did not fit u64: {conversion_error}"
				);
				ExecutorError::PreparationCapacityOverflow
			})?,
		);
		if actual != key.bytes {
			return Err(ExecutorError::AdmissionSizeMismatch {
				device: *device,
				expected: key.bytes,
				actual,
			});
		}
		validated.insert(*key, supplied.bytes);
	}
	match provided.keys().next().copied() {
		Some(device) => Err(ExecutorError::UnexpectedAdmission { device }),
		None => Ok(()),
	}?;

	for reset in fault_resets {
		let key = expected
			.get(&reset.location.device)
			.ok_or(ExecutorError::MissingAdmission {
				device: reset.location.device,
			})?;
		let image_location = bundle
			.value_location(key.image)
			.ok_or(ExecutorError::BackendProtocol {
				task: reset.task,
				detail: "finalized init image has no resolved value location",
			})?;
		let range = fault_reset_range(reset.task, *image_location, reset.location, key.bytes)?;
		let bytes = validated
			.get_mut(key)
			.ok_or(ExecutorError::MissingAdmission {
				device: reset.location.device,
			})?;
		let target = bytes.get_mut(range).ok_or(ExecutorError::BackendProtocol {
			task: reset.task,
			detail: "fault flag lies outside its device init image",
		})?;
		target.fill(0);
	}
	Ok(validated)
}

fn fault_reset_range(
	task: TaskId,
	image: ResolvedValueLocation,
	fault: ResolvedValueLocation,
	image_bytes: ByteCount,
) -> Result<core::ops::Range<usize>> {
	match image.device == fault.device && image.object == fault.object && image.bytes == image_bytes {
		true => Ok(()),
		false => {
			Err(ExecutorError::BackendProtocol {
				task,
				detail: "fault flag and finalized init image do not share one exact arena object",
			})
		}
	}?;
	let relative_offset = fault
		.arena_offset
		.get()
		.checked_sub(image.arena_offset.get())
		.ok_or(ExecutorError::BackendProtocol {
			task,
			detail: "fault flag precedes its finalized init image",
		})?;
	let start = usize::try_from(relative_offset).map_err(|conversion_error| {
		debug_assert!(
			false,
			"fault flag offset did not fit usize: {conversion_error}"
		);
		ExecutorError::BackendProtocol {
			task,
			detail: "fault flag offset does not fit the host address space",
		}
	})?;
	let end = start
		.checked_add(core::mem::size_of::<i32>())
		.ok_or(ExecutorError::BackendProtocol {
			task,
			detail: "fault flag byte range overflowed",
		})?;
	Ok(start..end)
}

fn run_phase_blocking<B: Backend>(
	core: &mut RunCore<B>,
	state: &mut PhaseState<B::Pending>,
	images: Option<&BTreeMap<InitImageKey, Vec<u8>>>,
) -> Result<()> {
	let mut backoff = BlockingPollBackoff::new();
	loop {
		let poll = poll_phase_once(core, state, images)?;
		if poll.complete {
			return Ok(());
		}
		match poll.made_progress {
			true => backoff.reset(),
			false => backoff.wait(),
		}
	}
}

fn poll_phase_once<B: Backend>(
	core: &mut RunCore<B>,
	state: &mut PhaseState<B::Pending>,
	images: Option<&BTreeMap<InitImageKey, Vec<u8>>>,
) -> Result<PhasePoll> {
	let false = state.complete else {
		return Ok(PhasePoll {
			complete: true,
			made_progress: false,
		});
	};

	let mut made_progress = false;
	if let Some(iteration) = state.loop_iteration {
		for index in 0..state.slots.len() {
			let inactive_and_ready = {
				let slot = &state.slots[index];
				slot.status == SlotStatus::Remaining
					&& !slot.task.active_on(iteration)
					&& slot
						.task
						.dependencies
						.iter()
						.all(|dependency| core.completed.contains(*dependency))
			};
			if !inactive_and_ready {
				continue;
			}
			let task = state.slots[index].task.id;
			core.completed.mark(task)?;
			core.journal.record_logical(LogicalEvent::TaskCompleted {
				phase: RunPhase::Loop,
				task,
			})?;
			state.slots[index].status = SlotStatus::Complete;
			made_progress = true;
		}
	}
	for index in 0..state.slots.len() {
		let runnable = {
			let slot = &state.slots[index];
			let resource = core.resource.active()?;
			let pipeline_queue = (state.phase == RunPhase::Loop
				&& core
					.backend
					.supports_same_queue_pipelining(resource, slot.task.id))
			.then(|| slot.task.submission().map(|submission| submission.queue))
			.flatten();
			slot.status == SlotStatus::Remaining
				&& state
					.loop_iteration
					.is_none_or(|iteration| slot.task.active_on(iteration))
				&& slot.task.dependencies.iter().all(|dependency| {
					core.completed.contains(*dependency)
						|| state.slots.iter().any(|issued| {
							issued.task.id == *dependency
								&& issued.status == SlotStatus::Pending && core
								.backend
								.supports_same_queue_pipelining(resource, issued.task.id)
								&& issued.task.submission().map(|submission| submission.queue)
									== pipeline_queue
						})
				}) && state.slots.iter().all(|active| {
				active.status != SlotStatus::Pending
					|| active.task.window.overlaps(slot.task.window)
					|| active.task.submission().map(|submission| submission.queue) == pipeline_queue
			})
		};
		let true = runnable else {
			continue;
		};
		submit_slot(
			core,
			state.phase,
			state.loop_iteration,
			&mut state.slots[index],
			images,
		)?;
		made_progress = true;
	}

	for slot in &mut state.slots {
		match slot.status {
			SlotStatus::Pending => {
				let mut physical_calls = PhysicalCallBatch::new();
				let result = {
					let resource = core.resource.active_mut()?;
					core.backend
						.poll(resource, &mut slot.pending, &mut physical_calls)
				};
				let poll = backend_value(
					&mut core.journal,
					BackendOperation::Poll { task: slot.task.id },
					physical_calls,
					result,
				)?;
				match poll {
					BackendPoll::Pending => {}
					BackendPoll::Complete { metric } => {
						complete_slot(core, state.phase, state.loop_iteration, slot, metric)?;
						slot.status = SlotStatus::Complete;
						made_progress = true;
					}
				}
			}
			SlotStatus::Remaining | SlotStatus::Complete => {}
		}
	}

	let has_remaining = state
		.slots
		.iter()
		.any(|slot| slot.status == SlotStatus::Remaining);
	let has_pending = state
		.slots
		.iter()
		.any(|slot| slot.status == SlotStatus::Pending);
	match (has_remaining, has_pending) {
		(false, false) => {
			state.complete = true;
			return Ok(PhasePoll {
				complete: true,
				made_progress,
			});
		}
		(true, false) if !made_progress => {
			return Err(ExecutorError::SchedulerStalled { phase: state.phase });
		}
		_ => {}
	}

	match made_progress {
		true => state.nonprogress_polls = 0,
		false => {
			state.nonprogress_polls = state.nonprogress_polls.saturating_add(1);
			let false = state.nonprogress_polls >= core.watchdog.max_nonprogress_polls else {
				return Err(ExecutorError::WatchdogExpired {
					phase: state.phase,
					nonprogress_polls: state.nonprogress_polls,
				});
			};
		}
	}
	Ok(PhasePoll {
		complete: false,
		made_progress,
	})
}

fn submit_slot<B: Backend>(
	core: &mut RunCore<B>,
	phase: RunPhase,
	iteration: Option<LoopIteration>,
	slot: &mut TaskSlot<B::Pending>,
	images: Option<&BTreeMap<InitImageKey, Vec<u8>>>,
) -> Result<()> {
	let work = slot.task.backend_work(core.run_id, iteration, images)?;
	let class = work.class();
	let mut physical_calls = PhysicalCallBatch::new();
	let result = {
		let resource = core.resource.active_mut()?;
		match (phase, iteration) {
			(RunPhase::Loop, Some(iteration)) => {
				core.backend.submit_loop_iteration(
					resource,
					ArenaSet::new(&core.arenas),
					&mut slot.pending,
					iteration,
					work,
					&mut physical_calls,
				)
			}
			(RunPhase::Loop, None) => {
				return Err(ExecutorError::LifecycleInvariant {
					detail: "loop task submission has no active iteration",
				});
			}
			(RunPhase::Init | RunPhase::Exit, _) => {
				core.backend.submit(
					resource,
					ArenaSet::new(&core.arenas),
					&mut slot.pending,
					work,
					&mut physical_calls,
				)
			}
		}
	};
	backend_value(
		&mut core.journal,
		BackendOperation::Submit { task: slot.task.id },
		physical_calls,
		result,
	)?;

	match slot.task.work {
		PreparedWork::InitAdmission { device, bytes, .. } => {
			core.journal.record_logical(LogicalEvent::InitAdmission {
				task: slot.task.id,
				device,
				bytes,
			})?;
		}
		_ => {
			core.journal.record_logical(LogicalEvent::TaskSubmitted {
				phase,
				task: slot.task.id,
				class,
			})?;
		}
	}
	slot.status = SlotStatus::Pending;
	Ok(())
}

fn complete_slot<B: Backend>(
	core: &mut RunCore<B>,
	phase: RunPhase,
	iteration: Option<LoopIteration>,
	slot: &mut TaskSlot<B::Pending>,
	metric_value: Option<crate::MetricValue>,
) -> Result<()> {
	let exit_collection = slot
		.task
		.external_exit()
		.map(|(source, bytes)| collect_exit_image(core, slot, source, bytes))
		.transpose()?;
	debug_assert_eq!(
		exit_collection.is_some(),
		slot.task.external_exit().is_some()
	);

	match (&slot.task.work, slot.task.class(), metric_value) {
		(
			PreparedWork::Metric {
				purpose,
				metric,
				slot: metric_slot,
				value: location,
				..
			},
			WorkClass::Metric,
			Some(metric_value),
		) => {
			match purpose {
				MetricPurpose::User => {
					let iteration = iteration.ok_or(ExecutorError::LifecycleInvariant {
						detail: "metric completion has no active loop iteration",
					})?;
					let replaced = core.metrics.publish(
						iteration,
						slot.task.id,
						*metric_slot,
						*metric,
						metric_value,
					)?;
					core.journal.record_logical(LogicalEvent::MetricPublished {
						task: slot.task.id,
						slot: *metric_slot,
						replaced_unconsumed: replaced,
					})?;
				}
				MetricPurpose::FaultReadback => {
					match metric_value {
						crate::MetricValue::I32(0) => {
							core.journal.record_logical(LogicalEvent::FaultChecked {
								readback: slot.task.id,
								value: location.value,
							})?;
						}
						crate::MetricValue::I32(code) => {
							return Err(ExecutorError::DeviceFault {
								readback: slot.task.id,
								value: location.value,
								code,
							});
						}
						crate::MetricValue::F32(_) => {
							return Err(ExecutorError::BackendProtocol {
								task: slot.task.id,
								detail: "fault readback completed with a non-int32 value",
							});
						}
					}
				}
			}
		}
		(PreparedWork::Metric { .. }, WorkClass::Metric, None) => {
			return Err(ExecutorError::BackendProtocol {
				task: slot.task.id,
				detail: "metric task completed without a metric value",
			});
		}
		(_, WorkClass::Metric, _) => {
			return Err(ExecutorError::BackendProtocol {
				task: slot.task.id,
				detail: "non-metric task was tagged as metric work",
			});
		}
		(_, _, Some(_)) => {
			return Err(ExecutorError::BackendProtocol {
				task: slot.task.id,
				detail: "non-metric task completed with a metric value",
			});
		}
		(_, _, None) => {}
	}
	core.completed.mark(slot.task.id)?;
	core.journal.record_logical(LogicalEvent::TaskCompleted {
		phase,
		task: slot.task.id,
	})?;
	Ok(())
}

fn collect_exit_image<B: Backend>(
	core: &mut RunCore<B>,
	slot: &mut TaskSlot<B::Pending>,
	source: ResolvedValueLocation,
	bytes: ByteCount,
) -> Result<()> {
	let length = usize::try_from(bytes.get()).map_err(|conversion_error| {
		debug_assert!(
			false,
			"exit image byte count did not fit usize: {conversion_error}"
		);
		ExecutorError::ExitImageTooLarge {
			task: slot.task.id,
			bytes,
		}
	})?;
	let mut image = Vec::new();
	image.try_reserve_exact(length)
		.map_err(|allocation_error| {
			debug_assert!(
				false,
				"exit image allocation failed unexpectedly: {allocation_error}"
			);
			ExecutorError::ExitImageAllocationFailed {
				task: slot.task.id,
				bytes,
			}
		})?;
	image.resize(length, 0);
	let BackendWork::ExitTransfer(work) = slot.task.backend_work(core.run_id, None, None)? else {
		return Err(ExecutorError::BackendProtocol {
			task: slot.task.id,
			detail: "external exit did not prepare exit-transfer work",
		});
	};
	let mut physical_calls = PhysicalCallBatch::new();
	let result = {
		let resource = core.resource.active_mut()?;
		core.backend.collect_exit(
			resource,
			ArenaSet::new(&core.arenas),
			&mut slot.pending,
			work,
			&mut image,
			&mut physical_calls,
		)
	};
	backend_value(
		&mut core.journal,
		BackendOperation::CollectExit { task: slot.task.id },
		physical_calls,
		result,
	)?;
	match core.exit_images.len() < core.exit_image_capacity {
		true => {
			core.exit_images.push(ExitImage {
				task: slot.task.id,
				source,
				bytes: image,
			})
		}
		false => {
			return Err(ExecutorError::BackendProtocol {
				task: slot.task.id,
				detail: "external exit exceeded its precomputed result slots",
			});
		}
	}
	Ok(())
}

fn resolve_values(bundle: &FinalizedBundle, task: TaskId, values: &[ValueId]) -> Result<Vec<ResolvedValueLocation>> {
	values.iter()
		.map(|value| resolve_value(bundle, task, *value))
		.collect()
}

fn resolve_transfer_endpoints(
	bundle: &FinalizedBundle,
	task: TaskId,
) -> Result<recipe_core::ResolvedTransferEndpoints> {
	bundle.transfer_endpoints(task)
		.ok_or(ExecutorError::BackendProtocol {
			task,
			detail: "finalized transfer references an endpoint without a resolved location",
		})
}

fn resolve_value(bundle: &FinalizedBundle, task: TaskId, value: ValueId) -> Result<ResolvedValueLocation> {
	bundle.value_location(value)
		.copied()
		.ok_or(ExecutorError::BackendProtocol {
			task,
			detail: "finalized task references a value without a resolved location",
		})
}

fn backend_value<T, E: std::error::Error>(
	journal: &mut RunJournal,
	operation: BackendOperation,
	physical_calls: PhysicalCallBatch,
	result: std::result::Result<T, E>,
) -> Result<T> {
	journal.record_physical(physical_calls)?;
	match result {
		Ok(value) => Ok(value),
		Err(error) => {
			let mut message = BackendMessage::default();
			match write!(&mut message, "{error}") {
				Ok(()) => {}
				Err(format_error) => {
					debug_assert!(
						message.was_truncated(),
						"unexpected backend error formatting failure: {format_error}"
					);
				}
			}
			Err(ExecutorError::Backend { operation, message })
		}
	}
}

fn checked_capacity_sum(values: &[usize]) -> Result<usize> {
	values.iter().try_fold(0_usize, |sum, value| {
		sum.checked_add(*value)
			.ok_or(ExecutorError::PreparationCapacityOverflow)
	})
}

fn iteration_domain_activations_before(domain: IterationDomain, retained_end: u64) -> Result<usize> {
	let end_exclusive = domain
		.end_exclusive()
		.map_or(retained_end, |end| end.min(retained_end));
	if end_exclusive <= domain.first_iteration() {
		return Ok(0);
	}
	let span = end_exclusive
		.checked_sub(domain.first_iteration())
		.ok_or(ExecutorError::PreparationCapacityOverflow)?;
	let stride = domain.stride().get();
	let activations = span
		.checked_add(stride - 1)
		.and_then(|value| value.checked_div(stride))
		.ok_or(ExecutorError::PreparationCapacityOverflow)?;
	usize::try_from(activations).map_err(|_| ExecutorError::PreparationCapacityOverflow)
}

fn repeated_loop_event(event: &LogicalEvent) -> bool {
	matches!(
		event,
		LogicalEvent::TaskSubmitted {
			phase: RunPhase::Loop,
			..
		} | LogicalEvent::TaskCompleted {
			phase: RunPhase::Loop,
			..
		} | LogicalEvent::LoopIterationStarted { .. }
			| LogicalEvent::MetricPublished { .. }
			| LogicalEvent::FaultChecked { .. }
			| LogicalEvent::LoopIterationCompleted { .. }
	)
}

fn checked_capacity_mul(left: usize, right: usize) -> Result<usize> {
	left.checked_mul(right)
		.ok_or(ExecutorError::PreparationCapacityOverflow)
}
