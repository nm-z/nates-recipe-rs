use std::collections::BTreeMap;
use std::fmt::{self, Write};

use recipe_core::{
	ArtifactId, BundleIdentity, ByteCount, DeviceId, FinalizedBundle, KernelTemplateId, LinkId, MetricId,
	MetricPurpose, MetricSlotId, ResolvedTransferEndpoint, ResolvedValueLocation, RunId, RunPhase, ScheduleWindow,
	SubmissionSlots, Task, TaskId, TaskKind, TransferEndpoint, TransferLaneClaim, ValueId,
};

use crate::backend::{
	ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork,
	MAX_PHYSICAL_CALLS_PER_OPERATION, MetricWork, PendingRequest, PhysicalCall, PhysicalCallBatch, TransferWork,
	WorkClass,
};
use crate::error::{BackendMessage, BackendOperation, ExecutorError, JournalStream, Result};
use crate::metrics::{MetricMailbox, MetricSample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Watchdog {
	max_nonprogress_polls: u32,
}

impl Watchdog {
	pub fn new(max_nonprogress_polls: u32) -> Result<Self> {
		match max_nonprogress_polls {
			0 => Err(ExecutorError::InvalidWatchdog {
				max_nonprogress_polls,
			}),
			_ => Ok(Self {
				max_nonprogress_polls,
			}),
		}
	}

	#[must_use]
	pub const fn max_nonprogress_polls(self) -> u32 {
		self.max_nonprogress_polls
	}
}

impl Default for Watchdog {
	fn default() -> Self {
		Self {
			max_nonprogress_polls: 1_024,
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
	MetricPublished {
		task: TaskId,
		slot: MetricSlotId,
		replaced_unconsumed: bool,
	},
	FaultChecked {
		calculation: TaskId,
		readback: TaskId,
		value: ValueId,
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

	pub(crate) fn recommended(bundle: &FinalizedBundle, watchdog: Watchdog) -> Result<Self> {
		let task_count = bundle.tasks().len();
		let arena_count = bundle.arena_layouts().len();
		let metric_count = bundle
			.tasks()
			.iter()
			.filter(|task| matches!(task.kind, TaskKind::Metric(_)))
			.count();
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
			8,
			checked_capacity_mul(arena_count, 2)?,
			checked_capacity_mul(task_count, 2)?,
			metric_count,
		])?;

		// Every non-poll operation may report a full inline batch. Poll records
		// receive one terminal record per task plus a declared watchdog stretch
		// across all task slots. More verbose backends fail closed at this bound.
		let fixed_backend_operations = checked_capacity_sum(&[
			2,
			checked_capacity_mul(arena_count, 2)?,
			checked_capacity_mul(task_count, 2)?,
			exit_image_count,
		])?;
		let fixed_physical = checked_capacity_mul(fixed_backend_operations, MAX_PHYSICAL_CALLS_PER_OPERATION)?;
		let watchdog_polls = checked_capacity_mul(
			task_count,
			usize::try_from(watchdog.max_nonprogress_polls).map_err(|conversion_error| {
				debug_assert!(false, "u32 watchdog did not fit usize: {conversion_error}");
				ExecutorError::PreparationCapacityOverflow
			})?,
		)?;
		let physical_calls = checked_capacity_sum(&[
			fixed_physical,
			task_count,
			watchdog_polls,
			MAX_PHYSICAL_CALLS_PER_OPERATION,
		])?;
		Ok(Self::new(logical_events, physical_calls))
	}
}

#[derive(Clone, Debug)]
pub struct RunJournal {
	logical_events: Vec<LogicalEvent>,
	physical_calls: Vec<PhysicalCall>,
	declared: JournalCapacity,
}

impl RunJournal {
	pub(crate) fn with_capacity(declared: JournalCapacity) -> Self {
		Self {
			logical_events: Vec::with_capacity(declared.logical_events),
			physical_calls: Vec::with_capacity(declared.physical_calls),
			declared,
		}
	}

	pub(crate) fn record_logical(&mut self, event: LogicalEvent) -> Result<()> {
		match self.logical_events.len() < self.declared.logical_events {
			true => {
				self.logical_events.push(event);
				Ok(())
			}
			false => Err(ExecutorError::JournalCapacityExceeded {
				stream: JournalStream::Logical,
				capacity: self.declared.logical_events,
			}),
		}
	}

	pub(crate) fn record_physical(&mut self, calls: PhysicalCallBatch) -> Result<()> {
		let required = self
			.physical_calls
			.len()
			.checked_add(calls.len())
			.ok_or(ExecutorError::PreparationCapacityOverflow)?;
		match required <= self.declared.physical_calls {
			true => {
				for call in calls.iter() {
					self.physical_calls.push(call);
				}
				Ok(())
			}
			false => Err(ExecutorError::JournalCapacityExceeded {
				stream: JournalStream::Physical,
				capacity: self.declared.physical_calls,
			}),
		}
	}

	#[must_use]
	pub fn logical_events(&self) -> &[LogicalEvent] {
		&self.logical_events
	}

	#[must_use]
	pub fn physical_calls(&self) -> &[PhysicalCall] {
		&self.physical_calls
	}

	#[must_use]
	pub const fn declared_capacity(&self) -> JournalCapacity {
		self.declared
	}

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
	fn active_mut(&mut self) -> Result<&mut R> {
		match self {
			Self::Active(resource) => Ok(resource),
			Self::Taken => Err(ExecutorError::LifecycleInvariant {
				detail: "backend resource was already consumed",
			}),
		}
	}

	fn consume(&mut self) -> Result<R> {
		match core::mem::replace(self, Self::Taken) {
			Self::Active(resource) => Ok(resource),
			Self::Taken => Err(ExecutorError::LifecycleInvariant {
				detail: "backend resource was already consumed",
			}),
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
		let journal_capacity = JournalCapacity::recommended(&bundle, watchdog)?;
		Self::prepare_with_journal_capacity(run_id, bundle, backend, watchdog, journal_capacity)
	}

	pub fn prepare_with_journal_capacity(
		run_id: RunId,
		bundle: FinalizedBundle,
		mut backend: B,
		watchdog: Watchdog,
		journal_capacity: JournalCapacity,
	) -> Result<Self> {
		let prepared = PreparedPhases::new(&bundle)?;
		let completed = CompletionLedger::new(bundle.tasks());
		let exit_image_capacity = prepared.exit.external_exit_count();
		let fault_resets = prepared.fault_resets();
		let mut journal = RunJournal::with_capacity(journal_capacity);
		let mut physical_calls = PhysicalCallBatch::new();
		let bind_result = backend.bind_resources(&bundle, &mut physical_calls);
		let resource = backend_value(
			&mut journal,
			BackendOperation::BindResources,
			physical_calls,
			bind_result,
		)?;
		let mut resource = resource;
		let init_phase = realize_phase(
			prepared.init,
			RunPhase::Init,
			&mut backend,
			&mut resource,
			&mut journal,
		)?;
		let loop_phase = realize_phase(
			prepared.loop_phase,
			RunPhase::Loop,
			&mut backend,
			&mut resource,
			&mut journal,
		)?;
		let exit_phase = realize_phase(
			prepared.exit,
			RunPhase::Exit,
			&mut backend,
			&mut resource,
			&mut journal,
		)?;
		let metrics = MetricMailbox::new(&bundle.resources().metrics, bundle.tasks());
		journal.record_logical(LogicalEvent::Prepared {
			run: run_id,
			bundle: bundle.identity(),
		})?;
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

	pub fn initialize(mut self, images: impl IntoIterator<Item = DeviceImage>) -> Result<InitializedRun<B>> {
		let images = validate_images(&self.core.bundle, images, &self.fault_resets)?;
		for layout_index in 0..self.core.bundle.arena_layouts().len() {
			let layout = &self.core.bundle.arena_layouts()[layout_index];
			let device = layout.device;
			let bytes = layout.size;
			let operation = BackendOperation::AllocateArena { device };
			let mut physical_calls = PhysicalCallBatch::new();
			let result = {
				let resource = self.core.resource.active_mut()?;
				self.core
					.backend
					.allocate_arena(resource, layout, &mut physical_calls)
			};
			let arena = backend_value(&mut self.core.journal, operation, physical_calls, result)?;
			let previous = self.core.arenas.insert(device, arena);
			debug_assert!(previous.is_none(), "finalized layouts have unique devices");
			self.core
				.journal
				.record_logical(LogicalEvent::ArenaAllocated { device, bytes })?;
		}

		run_phase_blocking(&mut self.core, &mut self.init_phase, Some(&images))?;
		self.core
			.journal
			.record_logical(LogicalEvent::Initialized {
				run: self.core.run_id,
			})?;
		Ok(InitializedRun {
			core: self.core,
			loop_phase: self.loop_phase,
			exit_phase: self.exit_phase,
		})
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal {
		&self.core.journal
	}
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
	pub fn start_loop(mut self) -> Result<RunningRun<B>> {
		self.core
			.journal
			.record_logical(LogicalEvent::LoopStarted {
				run: self.core.run_id,
			})?;
		Ok(RunningRun {
			core: self.core,
			phase: self.loop_phase,
			exit_phase: self.exit_phase,
			failure: None,
			completion_recorded: false,
		})
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal {
		&self.core.journal
	}
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
	pub fn poll(&mut self) -> Result<LoopStatus> {
		let failure_check = self.failure.map(Err::<(), ExecutorError>).transpose()?;
		debug_assert!(failure_check.is_none());
		match poll_phase_once(&mut self.core, &mut self.phase, None) {
			Ok(true) => {
				let false = self.completion_recorded else {
					return Ok(LoopStatus::Complete);
				};
				self.core
					.journal
					.record_logical(LogicalEvent::LoopCompleted {
						run: self.core.run_id,
					})?;
				self.completion_recorded = true;
				Ok(LoopStatus::Complete)
			}
			Ok(false) => Ok(LoopStatus::Pending),
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
		let terminal = std::iter::repeat_with(|| self.poll()).find_map(|poll| match poll {
			Ok(LoopStatus::Pending) => None,
			Ok(LoopStatus::Complete) => Some(Ok(())),
			Err(error) => Some(Err(error)),
		});
		let Some(result) = terminal else {
			unreachable!("watchdog-bounded polling always reaches a terminal result");
		};
		result
	}

	pub fn try_take_metric(&mut self, slot: MetricSlotId) -> Option<MetricSample> {
		self.core.metrics.try_take(slot)
	}

	#[must_use]
	pub fn metric_mailbox(&self) -> &MetricMailbox {
		&self.core.metrics
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal {
		&self.core.journal
	}

	#[must_use]
	pub fn capacities(&self) -> RuntimeCapacities {
		let journal = self.core.journal.allocated_capacity();
		RuntimeCapacities {
			phase_slots: self.phase.slots.capacity(),
			completion_entries: self.core.completed.entries.capacity(),
			logical_journal: journal.logical_events,
			physical_journal: journal.physical_calls,
		}
	}

	pub fn into_exited_loop(self) -> std::result::Result<ExitedLoop<B>, Box<Self>> {
		match self.phase.complete && self.failure.is_none() {
			true => Ok(ExitedLoop {
				core: self.core,
				exit_phase: self.exit_phase,
			}),
			false => Err(Box::new(self)),
		}
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
	pub fn exit(mut self) -> Result<ExitedRun<B>> {
		run_phase_blocking(&mut self.core, &mut self.exit_phase, None)?;

		let arenas = core::mem::take(&mut self.core.arenas);
		for (device, arena) in arenas {
			let mut physical_calls = PhysicalCallBatch::new();
			let result = {
				let resource = self.core.resource.active_mut()?;
				self.core
					.backend
					.release_arena(resource, device, arena, &mut physical_calls)
			};
			backend_value(
				&mut self.core.journal,
				BackendOperation::ReleaseArena { device },
				physical_calls,
				result,
			)?;
			self.core
				.journal
				.record_logical(LogicalEvent::ArenaReleased { device })?;
		}

		let resource = self.core.resource.consume()?;
		let mut physical_calls = PhysicalCallBatch::new();
		let result = self
			.core
			.backend
			.destroy_resources(resource, &mut physical_calls);
		backend_value(
			&mut self.core.journal,
			BackendOperation::DestroyResources,
			physical_calls,
			result,
		)?;
		self.core.journal.record_logical(LogicalEvent::Exited {
			run: self.core.run_id,
		})?;

		Ok(ExitedRun {
			run_id: self.core.run_id,
			bundle: self.core.bundle.identity(),
			backend: self.core.backend,
			metrics: self.core.metrics,
			exit_images: self.core.exit_images,
			journal: self.core.journal,
		})
	}

	pub fn try_take_metric(&mut self, slot: MetricSlotId) -> Option<MetricSample> {
		self.core.metrics.try_take(slot)
	}

	#[must_use]
	pub fn metric_mailbox(&self) -> &MetricMailbox {
		&self.core.metrics
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal {
		&self.core.journal
	}
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
	pub const fn run_id(&self) -> RunId {
		self.run_id
	}

	#[must_use]
	pub const fn bundle_identity(&self) -> BundleIdentity {
		self.bundle
	}

	#[must_use]
	pub fn journal(&self) -> &RunJournal {
		&self.journal
	}

	#[must_use]
	pub fn exit_images(&self) -> &[ExitImage] {
		&self.exit_images
	}

	pub fn try_take_metric(&mut self, slot: MetricSlotId) -> Option<MetricSample> {
		self.metrics.try_take(slot)
	}

	pub fn into_parts(self) -> (B, MetricMailbox, Vec<ExitImage>, RunJournal) {
		(self.backend, self.metrics, self.exit_images, self.journal)
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
}

impl<P: fmt::Debug> fmt::Debug for PhaseState<P> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("PhaseState")
			.field("phase", &self.phase)
			.field("slot_count", &self.slots.len())
			.field("nonprogress_polls", &self.nonprogress_polls)
			.field("complete", &self.complete)
			.finish()
	}
}

#[derive(Clone, Debug)]
struct PreparedTask {
	id: TaskId,
	window: ScheduleWindow,
	dependencies: Vec<TaskId>,
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
	},
}

impl PreparedTask {
	fn new(bundle: &FinalizedBundle, task: &Task) -> Result<Self> {
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
			(TaskKind::Metric(metric), RunPhase::Loop) => PreparedWork::Metric {
				purpose: metric.purpose,
				metric: metric.metric,
				slot: metric.slot,
				value: resolve_value(bundle, task.id, metric.value)?,
			},
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
			work,
		})
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
			| PreparedWork::Transfer { submission, .. } => Some(submission),
			PreparedWork::Metric { .. } => None,
		}
	}

	fn backend_work<'a>(
		&'a self,
		run: RunId,
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
			} => Ok(BackendWork::Calculation(CalculationWork {
				task: self.id,
				run,
				device: *device,
				kernel_template: *kernel_template,
				artifact: *artifact,
				submission: *submission,
				inputs,
				outputs,
				fault_flag: *fault_flag,
			})),
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
			} => Ok(BackendWork::Metric(MetricWork {
				task: self.id,
				purpose: *purpose,
				metric: *metric,
				slot: *slot,
				value: *value,
			})),
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
			} => Some(FaultReset {
				task: self.id,
				location,
			}),
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
			.map(|task| CompletionEntry {
				task: task.id,
				complete: false,
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

	fn completed_count(&self) -> usize {
		self.entries.iter().filter(|entry| entry.complete).count()
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
		false => Err(ExecutorError::BackendProtocol {
			task,
			detail: "fault flag and finalized init image do not share one exact arena object",
		}),
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

#[cfg(test)]
pub(crate) fn validate_images_with_fault_reset(
	bundle: &FinalizedBundle,
	images: impl IntoIterator<Item = DeviceImage>,
	task: TaskId,
	location: ResolvedValueLocation,
) -> Result<BTreeMap<DeviceId, Vec<u8>>> {
	validate_images(bundle, images, &[FaultReset { task, location }]).map(|images| {
		images.into_iter()
			.map(|(key, bytes)| (key.device, bytes))
			.collect()
	})
}

#[cfg(test)]
pub(crate) fn fault_reset_range_for_test(
	task: TaskId,
	image: ResolvedValueLocation,
	fault: ResolvedValueLocation,
	image_bytes: ByteCount,
) -> Result<core::ops::Range<usize>> {
	fault_reset_range(task, image, fault, image_bytes)
}

fn run_phase_blocking<B: Backend>(
	core: &mut RunCore<B>,
	state: &mut PhaseState<B::Pending>,
	images: Option<&BTreeMap<InitImageKey, Vec<u8>>>,
) -> Result<()> {
	let terminal = std::iter::repeat_with(|| poll_phase_once(core, state, images)).find_map(|poll| match poll {
		Ok(false) => None,
		Ok(true) => Some(Ok(())),
		Err(error) => Some(Err(error)),
	});
	let Some(result) = terminal else {
		unreachable!("watchdog-bounded phase polling always reaches a terminal result");
	};
	result
}

fn poll_phase_once<B: Backend>(
	core: &mut RunCore<B>,
	state: &mut PhaseState<B::Pending>,
	images: Option<&BTreeMap<InitImageKey, Vec<u8>>>,
) -> Result<bool> {
	let false = state.complete else {
		return Ok(true);
	};

	let mut made_progress = false;
	for index in 0..state.slots.len() {
		let runnable = {
			let slot = &state.slots[index];
			slot.status == SlotStatus::Remaining
				&& slot
					.task
					.dependencies
					.iter()
					.all(|dependency| core.completed.contains(*dependency))
				&& state.slots.iter().all(|active| {
					active.status != SlotStatus::Pending || active.task.window.overlaps(slot.task.window)
				})
		};
		let true = runnable else {
			continue;
		};
		submit_slot(core, state.phase, &mut state.slots[index], images)?;
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
						complete_slot(core, state.phase, slot, metric)?;
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
			return Ok(true);
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
	Ok(false)
}

fn submit_slot<B: Backend>(
	core: &mut RunCore<B>,
	phase: RunPhase,
	slot: &mut TaskSlot<B::Pending>,
	images: Option<&BTreeMap<InitImageKey, Vec<u8>>>,
) -> Result<()> {
	let work = slot.task.backend_work(core.run_id, images)?;
	let class = work.class();
	let mut physical_calls = PhysicalCallBatch::new();
	let result = {
		let resource = core.resource.active_mut()?;
		core.backend.submit(
			resource,
			ArenaSet::new(&core.arenas),
			&mut slot.pending,
			work,
			&mut physical_calls,
		)
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
			},
			WorkClass::Metric,
			Some(metric_value),
		) => match purpose {
			MetricPurpose::User => {
				let replaced = core
					.metrics
					.publish(slot.task.id, *metric_slot, *metric, metric_value)?;
				core.journal.record_logical(LogicalEvent::MetricPublished {
					task: slot.task.id,
					slot: *metric_slot,
					replaced_unconsumed: replaced,
				})?;
			}
			MetricPurpose::FaultReadback { calculation } => match metric_value {
				crate::MetricValue::I32(0) => {
					core.journal.record_logical(LogicalEvent::FaultChecked {
						calculation: *calculation,
						readback: slot.task.id,
						value: location.value,
					})?;
				}
				crate::MetricValue::I32(code) => {
					return Err(ExecutorError::DeviceFault {
						calculation: *calculation,
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
			},
		},
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
	let BackendWork::ExitTransfer(work) = slot.task.backend_work(core.run_id, None)? else {
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
		true => core.exit_images.push(ExitImage {
			task: slot.task.id,
			source,
			bytes: image,
		}),
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

fn checked_capacity_mul(left: usize, right: usize) -> Result<usize> {
	left.checked_mul(right)
		.ok_or(ExecutorError::PreparationCapacityOverflow)
}
