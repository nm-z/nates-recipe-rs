use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use recipe_core::{
	ArenaLayout, BundleIdentity, ByteCount, DType, DeviceId, DraftPlan, FinalizedBundle, MetricId, MetricSlotId,
	ReservationLedger, ResolvedTransferEndpoint, ResolvedValueLocation, RunPhase, SubmissionSlots, TaskId, TaskKind,
	TransferEndpoint, TransferLaneClaim, ValueId,
};
use rustix::fs::statvfs;
use recipe_executor::{
	ArenaSet, Backend, BackendPoll, BackendWork, MetricValue, PendingRequest, PhysicalCall, PhysicalCallBatch,
	PhysicalPollStatus, TransferWork, WorkClass, sealed,
};

use crate::{
	Arena, DiskFileSpec, Error, PendingCopy, PollStatus, Reservation, Result, Runtime, RuntimeConfig, SlotCapacity,
};

fn require(condition: bool, error: Error) -> Result<()> {
	match condition {
		true => Ok(()),
		false => Err(error),
	}
}

fn consume_context<T>(context: T) {
	drop(context);
}

/// One caller-resolved host storage binding.
///
/// Paths are selected during preparation. Disk files are created with
/// `create_new` when resources are bound or arenas are initialized; they are
/// never discovered or selected from the live loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostDeviceBinding {
	Ram {
		device: DeviceId,
	},
	Disk {
		device: DeviceId,
		arena: DiskFileSpec,
		reservation: DiskFileSpec,
	},
}

impl HostDeviceBinding {
	#[must_use]
	pub const fn device(&self) -> DeviceId {
		match self {
			Self::Ram { device } | Self::Disk { device, .. } => *device,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostBackendConfig {
	worker_threads: usize,
	staging_bytes_per_slot: usize,
	bindings: Vec<HostDeviceBinding>,
}

impl HostBackendConfig {
	pub fn new(
		worker_threads: usize,
		staging_bytes_per_slot: usize,
		bindings: Vec<HostDeviceBinding>,
	) -> Result<Self> {
		if worker_threads == 0 {
			return Err(Error::InvalidConfiguration(
				"host backend worker count must be nonzero",
			));
		}
		if staging_bytes_per_slot == 0 {
			return Err(Error::InvalidConfiguration(
				"host backend staging capacity must be nonzero",
			));
		}
		validate_bindings(&bindings)?;
		Ok(Self {
			worker_threads,
			staging_bytes_per_slot,
			bindings,
		})
	}

	#[must_use]
	pub const fn worker_threads(&self) -> usize {
		self.worker_threads
	}

	#[must_use]
	pub const fn staging_bytes_per_slot(&self) -> usize {
		self.staging_bytes_per_slot
	}

	#[must_use]
	pub fn bindings(&self) -> &[HostDeviceBinding] {
		&self.bindings
	}
}

#[derive(Debug)]
enum BackendState {
	Ready(HostBackendConfig),
	Prepared(HostPreparedResources),
	Warmed(HostResources),
	Bound,
}

/// Recipe executor adapter for RAM and disk-only task subsets.
///
/// Calculations deliberately fail closed: calculation payloads belong on the
/// native GPU adapters. A mixed backend can compose this adapter with CUDA,
/// HSA and remote lanes with the same preallocated pending tokens.
#[derive(Debug)]
pub struct HostBackend {
	state: BackendState,
}

impl HostBackend {
	#[must_use]
	pub const fn new(config: HostBackendConfig) -> Self {
		Self {
			state: BackendState::Ready(config),
		}
	}

	#[doc(hidden)]
	#[must_use]
	pub const fn from_prepared(resources: HostPreparedResources) -> Self {
		Self {
			state: BackendState::Prepared(resources),
		}
	}

	#[doc(hidden)]
	#[must_use]
	pub const fn from_warmed(resources: HostResources) -> Self {
		Self {
			state: BackendState::Warmed(resources),
		}
	}

	/// Binds the host-owned partition of a heterogeneous finalized bundle.
	///
	/// This is an integration surface for Recipe's composite local backend. It
	/// deliberately performs no physical-call accounting; the composite emits
	/// the single logical backend record for the enclosing operation.
	#[doc(hidden)]
	pub fn bind_partition(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<HostResources> {
		let state = core::mem::replace(&mut self.state, BackendState::Bound);
		match state {
			BackendState::Ready(config) => HostResources::realize_partition(bundle, config, tasks),
			BackendState::Prepared(resources) => resources.bind(bundle, tasks),
			BackendState::Warmed(mut resources) => {
				resources.validate_handoff(bundle, tasks)?;
				Ok(resources)
			}
			BackendState::Bound => Err(Error::BackendState("resources may be bound only once")),
		}
	}

	#[doc(hidden)]
	pub fn prepare_partition(
		&mut self,
		resource: &mut HostResources,
		request: PendingRequest,
	) -> Result<HostPending> {
		resource.prepare_pending(request)
	}

	#[doc(hidden)]
	pub fn allocate_partition(&mut self, resource: &mut HostResources, layout: &ArenaLayout) -> Result<Arena> {
		resource.allocate_arena(layout)
	}

	#[doc(hidden)]
	pub fn submit_partition<L: HostArenaLookup + ?Sized>(
		&mut self,
		resource: &mut HostResources,
		arenas: &L,
		pending: &mut HostPending,
		work: BackendWork<'_>,
	) -> Result<()> {
		resource.submit(arenas, pending, work)
	}

	#[doc(hidden)]
	pub fn poll_partition(&mut self, resource: &mut HostResources, pending: &mut HostPending) -> Result<BackendPoll> {
		resource.poll_pending(pending)
	}

	#[doc(hidden)]
	pub fn collect_partition(
		&mut self,
		resource: &mut HostResources,
		pending: &HostPending,
		work: TransferWork<'_>,
		destination: &mut [u8],
	) -> Result<()> {
		resource.collect_exit(pending, work, destination)
	}

	#[doc(hidden)]
	pub fn release_partition(&mut self, resource: &mut HostResources, device: DeviceId, arena: Arena) -> Result<()> {
		resource.ensure_healthy()?;
		if arena.device() != device {
			return Err(Error::Protocol {
				task: TaskId::new(0),
				detail: "released host arena belongs to another device",
			});
		}
		arena.close()
	}

	#[doc(hidden)]
	pub fn destroy_partition(&mut self, resource: HostResources) -> Result<()> {
		resource.destroy()
	}

	#[doc(hidden)]
	pub fn prepare_candidate(
		draft: &DraftPlan,
		reservations: &ReservationLedger,
		config: HostBackendConfig,
		tasks: &BTreeSet<TaskId>,
	) -> Result<HostPreparedResources> {
		HostPreparedResources::realize(draft, reservations, config, tasks)
	}
}

impl sealed::Sealed for HostBackend {}

/// Allocation-free lookup used by Recipe's heterogeneous local backend.
///
/// Implementations must return only the host arena owned by `device`.
#[doc(hidden)]
pub trait HostArenaLookup {
	fn host_arena(&self, device: DeviceId) -> Option<&Arena>;
}

impl HostArenaLookup for ArenaSet<'_, Arena> {
	fn host_arena(&self, device: DeviceId) -> Option<&Arena> {
		self.get(device)
	}
}

#[derive(Clone, Debug)]
enum ExpectedWork {
	Init {
		image: ValueId,
		destination: ResolvedValueLocation,
		bytes: ByteCount,
		submission: SubmissionSlots,
	},
	Transfer {
		class: WorkClass,
		source: ResolvedTransferEndpoint,
		destination: ResolvedTransferEndpoint,
		bytes: ByteCount,
		route: Vec<recipe_core::LinkId>,
		lane_claims: Vec<TransferLaneClaim>,
		submission: SubmissionSlots,
	},
	Metric {
		metric: MetricId,
		slot: MetricSlotId,
		value: ResolvedValueLocation,
	},
}

impl ExpectedWork {
	const fn class(&self) -> WorkClass {
		match self {
			Self::Init { .. } => WorkClass::InitAdmission,
			Self::Transfer { class, .. } => *class,
			Self::Metric { .. } => WorkClass::Metric,
		}
	}

	const fn submission(&self) -> Option<SubmissionSlots> {
		match self {
			Self::Init { submission, .. } | Self::Transfer { submission, .. } => Some(*submission),
			Self::Metric { .. } => None,
		}
	}

	const fn admission(&self) -> Option<InitImageContract> {
		match self {
			Self::Init {
				image,
				destination,
				bytes,
				..
			} => Some(InitImageContract {
				device: destination.device,
				image: *image,
				bytes: *bytes,
			}),
			Self::Transfer { .. } | Self::Metric { .. } => None,
		}
	}

	const fn staging(&self) -> Option<(DeviceId, ByteCount, PendingAction)> {
		match self {
			Self::Init {
				destination, bytes, ..
			} => Some((destination.device, *bytes, PendingAction::None)),
			Self::Transfer {
				class: WorkClass::ExitTransfer,
				source: ResolvedTransferEndpoint::Device(source),
				destination: ResolvedTransferEndpoint::External,
				bytes,
				..
			} => Some((source.device, *bytes, PendingAction::Egress)),
			Self::Metric { value, .. } => Some((
				value.device,
				ByteCount::new(4),
				PendingAction::Metric { dtype: value.dtype },
			)),
			Self::Transfer { .. } => None,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InitImageContract {
	device: DeviceId,
	image: ValueId,
	bytes: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAction {
	None,
	Metric { dtype: DType },
	Egress,
}

pub struct HostPending {
	task: TaskId,
	class: WorkClass,
	copy: PendingCopy,
	staging: Option<Arena>,
	admission: Option<InitImageContract>,
	action: PendingAction,
	submitted: bool,
	terminal: bool,
}

impl fmt::Debug for HostPending {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HostPending")
			.field("task", &self.task)
			.field("class", &self.class)
			.field("copy", &self.copy)
			.field("staging", &self.staging)
			.field("admission", &self.admission)
			.field("action", &self.action)
			.field("submitted", &self.submitted)
			.field("terminal", &self.terminal)
			.finish()
	}
}

impl HostPending {
	#[must_use]
	pub const fn task(&self) -> TaskId {
		self.task
	}
}

pub struct HostResources {
	runtime: Runtime,
	reservations: BTreeMap<DeviceId, Reservation>,
	bindings: BTreeMap<DeviceId, HostDeviceBinding>,
	contracts: BTreeMap<TaskId, ExpectedWork>,
	prepared: BTreeSet<TaskId>,
	pending: PendingResources,
	poisoned: bool,
}

#[derive(Debug)]
enum PendingResources {
	Deferred,
	Prepared(BTreeMap<TaskId, HostPending>),
}

#[doc(hidden)]
pub struct HostPreparedResources {
	runtime: Runtime,
	reservations: BTreeMap<DeviceId, Reservation>,
	bindings: BTreeMap<DeviceId, HostDeviceBinding>,
	pending: BTreeMap<TaskId, HostPending>,
	handoff: HostPreparedHandoff,
}

#[derive(Debug)]
enum HostPreparedHandoff {
	Candidate,
	Finalized {
		bundle: BundleIdentity,
		tasks: BTreeSet<TaskId>,
		contracts: BTreeMap<TaskId, ExpectedWork>,
	},
}

impl fmt::Debug for HostResources {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HostResources")
			.field("runtime", &self.runtime)
			.field("reservation_count", &self.reservations.len())
			.field("bindings", &self.bindings)
			.field("contract_count", &self.contracts.len())
			.field("prepared", &self.prepared)
			.field("pending", &self.pending)
			.field("poisoned", &self.poisoned)
			.finish()
	}
}

impl fmt::Debug for HostPreparedResources {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HostPreparedResources")
			.field("runtime", &self.runtime)
			.field("reservation_count", &self.reservations.len())
			.field("bindings", &self.bindings)
			.field("pending", &self.pending)
			.field("handoff", &self.handoff)
			.finish()
	}
}

impl HostPreparedResources {
	fn realize(
		draft: &DraftPlan,
		reservations: &ReservationLedger,
		config: HostBackendConfig,
		tasks: &BTreeSet<TaskId>,
	) -> Result<Self> {
		for task in tasks {
			require(
				draft.tasks.iter().any(|candidate| candidate.id == *task),
				Error::Protocol {
					task: *task,
					detail: "host candidate partition names an absent task",
				},
			)?;
		}
		let bindings = index_bindings(config.bindings)?;
		let candidate_devices = draft
			.arena_objects
			.iter()
			.map(|object| object.device)
			.collect::<BTreeSet<_>>();
		for device in bindings.keys() {
			if !candidate_devices.contains(device) {
				return Err(Error::InvalidConfiguration(
					"host binding names a device absent from the candidate arena objects",
				));
			}
		}
		let slots = SlotCapacity::new(tasks.len().max(1))?;
		let runtime = Runtime::new(RuntimeConfig::new(
			config.worker_threads,
			slots,
			config.staging_bytes_per_slot,
		)?)?;
		let reservations = match allocate_reservations_from_ledger(reservations, &bindings) {
			Ok(reservations) => reservations,
			Err(error) => {
				drop(runtime.close());
				return Err(error);
			}
		};
		let pending = match prepare_candidate_pending(&runtime, draft, tasks) {
			Ok(pending) => pending,
			Err(error) => {
				drop(destroy_host_resources(runtime, reservations));
				return Err(error);
			}
		};
		Ok(Self {
			runtime,
			reservations,
			bindings,
			pending,
			handoff: HostPreparedHandoff::Candidate,
		})
	}

	fn bind(self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<HostResources> {
		let Self {
			runtime,
			reservations,
			bindings,
			pending,
			handoff,
		} = self;
		let contracts = match handoff {
			HostPreparedHandoff::Finalized {
				bundle: prepared_bundle,
				tasks: prepared_tasks,
				contracts,
			} => match prepared_bundle == bundle.identity() && prepared_tasks == *tasks {
				true => Ok(contracts),
				false => Err(Error::BackendState(
					"finalized host partition differs from its prepared handoff",
				)),
			},
			HostPreparedHandoff::Candidate => Err(Error::BackendState(
				"candidate host resources were not validated for finalized handoff",
			)),
		}?;
		Ok(HostResources {
			runtime,
			reservations,
			bindings,
			contracts,
			prepared: BTreeSet::new(),
			pending: PendingResources::Prepared(pending),
			poisoned: false,
		})
	}

	#[doc(hidden)]
	pub fn bind_candidate(self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<HostResources> {
		let Self {
			runtime,
			reservations,
			bindings,
			pending,
			handoff,
		} = self;
		match handoff {
			HostPreparedHandoff::Candidate => Ok(()),
			HostPreparedHandoff::Finalized { .. } => Err(Error::BackendState(
				"finalized host resources cannot be rebound as a warm candidate",
			)),
		}?;
		validate_bundle_devices(bundle, &bindings, false)?;
		match pending.keys().copied().eq(tasks.iter().copied()) {
			true => Ok(()),
			false => Err(Error::BackendState(
				"warm host partition differs from its pre-final pending pool",
			)),
		}?;
		let contracts = task_contracts(bundle, Some(tasks))?;
		for (task, prepared) in &pending {
			let expected = contracts.get(task).ok_or(Error::Protocol {
				task: *task,
				detail: "warm host task is absent from the candidate contract",
			})?;
			require(
				prepared.admission == expected.admission(),
				Error::Protocol {
					task: *task,
					detail: "warm init-image manifest differs from prepared host admission",
				},
			)?;
		}
		Ok(HostResources {
			runtime,
			reservations,
			bindings,
			contracts,
			prepared: BTreeSet::new(),
			pending: PendingResources::Prepared(pending),
			poisoned: false,
		})
	}

	#[doc(hidden)]
	pub fn validate_handoff(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<()> {
		match self.handoff {
			HostPreparedHandoff::Candidate => Ok(()),
			HostPreparedHandoff::Finalized { .. } => Err(Error::BackendState(
				"candidate host handoff was validated more than once",
			)),
		}?;
		validate_bundle_devices(bundle, &self.bindings, false)?;
		match self.pending.keys().copied().eq(tasks.iter().copied()) {
			true => Ok(()),
			false => Err(Error::BackendState(
				"finalized host partition differs from its pre-final pending pool",
			)),
		}?;
		let contracts = task_contracts(bundle, Some(tasks))?;
		for (task, pending) in &self.pending {
			let expected = contracts.get(task).ok_or(Error::Protocol {
				task: *task,
				detail: "finalized host contract is absent from the prepared pending pool",
			})?;
			require(
				pending.admission == expected.admission(),
				Error::Protocol {
					task: *task,
					detail: "finalized init-image manifest differs from the prepared host admission",
				},
			)?;
		}
		self.handoff = HostPreparedHandoff::Finalized {
			bundle: bundle.identity(),
			tasks: tasks.clone(),
			contracts,
		};
		Ok(())
	}

	#[doc(hidden)]
	pub fn destroy(self) -> Result<()> {
		let Self {
			runtime,
			reservations,
			pending,
			..
		} = self;
		drop(pending);
		destroy_host_resources(runtime, reservations)
	}
}

impl HostResources {
	fn realize(bundle: &FinalizedBundle, config: HostBackendConfig) -> Result<Self> {
		Self::realize_scoped(bundle, config, None)
	}

	fn realize_partition(
		bundle: &FinalizedBundle,
		config: HostBackendConfig,
		tasks: &BTreeSet<TaskId>,
	) -> Result<Self> {
		Self::realize_scoped(bundle, config, Some(tasks))
	}

	fn realize_scoped(
		bundle: &FinalizedBundle,
		config: HostBackendConfig,
		tasks: Option<&BTreeSet<TaskId>>,
	) -> Result<Self> {
		let bindings = index_bindings(config.bindings)?;
		validate_bundle_devices(bundle, &bindings, tasks.is_none())?;
		let contracts = task_contracts(bundle, tasks)?;
		let slots = SlotCapacity::new(contracts.len().max(1))?;
		let runtime = Runtime::new(RuntimeConfig::new(
			config.worker_threads,
			slots,
			config.staging_bytes_per_slot,
		)?)?;
		let reservations = match allocate_reservations(bundle, &bindings) {
			Ok(reservations) => reservations,
			Err(error) => {
				drop(runtime.close());
				return Err(error);
			}
		};
		Ok(Self {
			runtime,
			reservations,
			bindings,
			contracts,
			prepared: BTreeSet::new(),
			pending: PendingResources::Deferred,
			poisoned: false,
		})
	}

	fn ensure_healthy(&self) -> Result<()> {
		if self.poisoned {
			Err(Error::Poisoned)
		} else {
			Ok(())
		}
	}

	#[doc(hidden)]
	pub fn prepare_pending(&mut self, request: PendingRequest) -> Result<HostPending> {
		self.ensure_healthy()?;
		let expected = self.contracts.get(&request.task).ok_or(Error::Protocol {
			task: request.task,
			detail: "pending request names no finalized host task",
		})?;
		if request.class != expected.class()
			|| request.submission != expected.submission()
			|| !phase_accepts(request.phase, expected.class())
		{
			return Err(Error::Protocol {
				task: request.task,
				detail: "pending request differs from the finalized host task",
			});
		}
		if !self.prepared.insert(request.task) {
			return Err(Error::Protocol {
				task: request.task,
				detail: "host pending token was prepared more than once",
			});
		}
		match &mut self.pending {
			PendingResources::Prepared(pending) => {
				let prepared = pending.remove(&request.task).ok_or(Error::Protocol {
					task: request.task,
					detail: "pre-final host pending resource is absent",
				})?;
				match prepared.task == request.task
					&& prepared.class == request.class
					&& prepared.admission == expected.admission()
				{
					true => Ok(prepared),
					false => Err(Error::Protocol {
						task: request.task,
						detail: "pre-final host pending resource differs from finalized request",
					}),
				}
			}
			PendingResources::Deferred => {
				let copy = self.runtime.prepare_copy()?;
				let (staging, action) = match expected.staging() {
					Some((device, bytes, action)) => (Some(Arena::ram(device, bytes)?), action),
					None => (None, PendingAction::None),
				};
				Ok(HostPending {
					task: request.task,
					class: request.class,
					copy,
					staging,
					admission: expected.admission(),
					action,
					submitted: false,
					terminal: false,
				})
			}
		}
	}

	#[doc(hidden)]
	pub fn allocate_arena(&self, layout: &ArenaLayout) -> Result<Arena> {
		self.ensure_healthy()?;
		match self
			.bindings
			.get(&layout.device)
			.ok_or(Error::MissingDevice(layout.device))?
		{
			HostDeviceBinding::Ram { .. } => Arena::ram(layout.device, layout.size),
			HostDeviceBinding::Disk { arena, .. } => Arena::disk(layout.device, arena.clone(), layout.size),
		}
	}

	#[doc(hidden)]
	pub fn submit<L: HostArenaLookup + ?Sized>(
		&mut self,
		arenas: &L,
		pending: &mut HostPending,
		work: BackendWork<'_>,
	) -> Result<()> {
		self.ensure_healthy()?;
		let task = work.task();
		let submitted_class = work.class();
		if pending.submitted || pending.terminal || pending.task != task {
			return Err(Error::Protocol {
				task,
				detail: "host pending token is not ready for this task",
			});
		}
		let expected = self.contracts.get(&task).ok_or(Error::Protocol {
			task,
			detail: "submitted work names no finalized host task",
		})?;
		if expected.class() != submitted_class || pending.class != submitted_class {
			return Err(Error::Protocol {
				task,
				detail: "submitted work class differs from the finalized host task",
			});
		}
		let result = match (expected, work) {
			(
				ExpectedWork::Init {
					image,
					destination,
					bytes,
					submission,
				},
				BackendWork::InitAdmission(work),
			) => {
				let image_bytes = u64::try_from(work.image.len()).map_err(|error| {
					consume_context(error);
					Error::RangeOverflow
				})?;
				if work.destination.value != *image
					|| work.destination != *destination
					|| work.bytes != *bytes || work.submission != *submission
					|| image_bytes != bytes.get()
					|| pending.admission != expected.admission()
				{
					return Err(Error::Protocol {
						task: work.task,
						detail: "host admission differs from its finalized contract",
					});
				}
				let staging = pending.staging.as_ref().ok_or(Error::Protocol {
					task: work.task,
					detail: "host admission has no preallocated staging arena",
				})?;
				staging.write_exact(0, work.image)?;
				let destination_arena = checked_arena(arenas, *destination, bytes.get())?;
				pending.copy.submit(
					staging,
					0,
					destination_arena,
					destination.arena_offset.get(),
					*bytes,
				)
			}
			(
				ExpectedWork::Transfer {
					class,
					source,
					destination,
					bytes,
					route,
					lane_claims,
					submission,
				},
				BackendWork::InternalTransfer(work) | BackendWork::ExitTransfer(work),
			) => {
				validate_transfer(
					work,
					TransferExpectation {
						submitted_class,
						class: *class,
						source: *source,
						destination: *destination,
						bytes: *bytes,
						route,
						lane_claims,
						submission: *submission,
					},
				)?;
				submit_transfer(arenas, pending, work)
			}
			(
				ExpectedWork::Metric {
					metric,
					slot,
					value,
				},
				BackendWork::Metric(work),
			) => {
				if work.metric != *metric || work.slot != *slot || work.value != *value {
					return Err(Error::Protocol {
						task: work.task,
						detail: "host metric differs from its finalized contract",
					});
				}
				let source = checked_arena(arenas, *value, 4)?;
				let staging = pending.staging.as_ref().ok_or(Error::Protocol {
					task: work.task,
					detail: "host metric has no preallocated staging arena",
				})?;
				pending.copy.submit(
					source,
					value.arena_offset.get(),
					staging,
					0,
					ByteCount::new(4),
				)
			}
			(_, BackendWork::Calculation(work)) => Err(Error::UnsupportedWork {
				task: work.task,
				detail: "payload calculations require a CUDA or HSA GPU adapter",
			}),
			(unexpected_contract, unexpected_work) => {
				consume_context((unexpected_contract, unexpected_work));
				Err(Error::Protocol {
					task,
					detail: "submitted host work variant differs from its finalized contract",
				})
			}
		};
		match result {
			Ok(()) => {
				pending.submitted = true;
				Ok(())
			}
			Err(error) => {
				self.poisoned = true;
				Err(error)
			}
		}
	}

	#[doc(hidden)]
	pub fn poll_pending(&mut self, pending: &mut HostPending) -> Result<BackendPoll> {
		self.ensure_healthy()?;
		if !pending.submitted || pending.terminal {
			return Err(Error::Protocol {
				task: pending.task,
				detail: "host pending token is not active",
			});
		}
		match pending.copy.poll() {
			Ok(PollStatus::Pending) => Ok(BackendPoll::Pending),
			Ok(PollStatus::Complete) => {
				let metric = match pending.action {
					PendingAction::None | PendingAction::Egress => None,
					PendingAction::Metric { dtype } => {
						let staging = pending.staging.as_ref().ok_or(Error::Protocol {
							task: pending.task,
							detail: "completed host metric has no staging arena",
						})?;
						let mut bytes = [0_u8; 4];
						staging.read_exact(0, &mut bytes)?;
						Some(match dtype {
							DType::F32 => MetricValue::F32(f32::from_le_bytes(bytes)),
							DType::I32 => MetricValue::I32(i32::from_le_bytes(bytes)),
						})
					}
				};
				pending.terminal = true;
				Ok(BackendPoll::Complete { metric })
			}
			Err(error) => {
				self.poisoned = true;
				Err(error)
			}
		}
	}

	#[doc(hidden)]
	pub fn collect_exit(&self, pending: &HostPending, work: TransferWork<'_>, destination: &mut [u8]) -> Result<()> {
		self.ensure_healthy()?;
		let destination_bytes = u64::try_from(destination.len()).map_err(|error| {
			consume_context(error);
			Error::RangeOverflow
		})?;
		if !pending.terminal
			|| pending.task != work.task
			|| pending.action != PendingAction::Egress
			|| destination_bytes != work.bytes.get()
		{
			return Err(Error::Protocol {
				task: work.task,
				detail: "host exit collection differs from its completed task",
			});
		}
		let expected = self.contracts.get(&work.task).ok_or(Error::Protocol {
			task: work.task,
			detail: "collected host exit has no finalized contract",
		})?;
		let ExpectedWork::Transfer {
			class,
			source,
			destination: expected_destination,
			bytes,
			route,
			lane_claims,
			submission,
		} = expected
		else {
			return Err(Error::Protocol {
				task: work.task,
				detail: "collected host exit is not a finalized transfer",
			});
		};
		validate_transfer(
			work,
			TransferExpectation {
				submitted_class: WorkClass::ExitTransfer,
				class: *class,
				source: *source,
				destination: *expected_destination,
				bytes: *bytes,
				route,
				lane_claims,
				submission: *submission,
			},
		)?;
		if !matches!(work.destination, ResolvedTransferEndpoint::External) {
			return Err(Error::Protocol {
				task: work.task,
				detail: "host exit collection has no external destination",
			});
		}
		pending
			.staging
			.as_ref()
			.ok_or(Error::Protocol {
				task: work.task,
				detail: "completed host exit has no staging arena",
			})?
			.read_exact(0, destination)
	}

	#[doc(hidden)]
	pub fn recycle_pending(&mut self, mut pending: HostPending) -> Result<()> {
		self.ensure_healthy()?;
		require(
			pending.terminal && self.prepared.remove(&pending.task),
			Error::Protocol {
				task: pending.task,
				detail: "only one terminal host pending token may be recycled",
			},
		)?;
		pending.copy.reset()?;
		pending.submitted = false;
		pending.terminal = false;
		let task = pending.task;
		match &mut self.pending {
			PendingResources::Prepared(pool) => require(
				pool.insert(task, pending).is_none(),
				Error::Protocol {
					task,
					detail: "host pending pool already contains the recycled task",
				},
			),
			PendingResources::Deferred => Err(Error::BackendState(
				"warm host resources have no pre-final pending pool",
			)),
		}
	}

	#[doc(hidden)]
	pub fn available_bytes(&self, device: DeviceId) -> Result<ByteCount> {
		let binding = self.bindings.get(&device).ok_or(Error::MissingDevice(device))?;
		match binding {
			HostDeviceBinding::Ram { .. } => available_ram_bytes(),
			HostDeviceBinding::Disk { arena, .. } => available_disk_bytes(arena.path()),
		}
	}

	#[doc(hidden)]
	pub fn validate_handoff(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<()> {
		self.ensure_healthy()?;
		require(
			self.prepared.is_empty(),
			Error::BackendState("warm host pending tokens were not all recycled"),
		)?;
		validate_bundle_devices(bundle, &self.bindings, false)?;
		let pool = match &self.pending {
			PendingResources::Prepared(pool) => pool,
			PendingResources::Deferred => {
				return Err(Error::BackendState(
					"warm host resources lost their pre-final pending pool",
				));
			}
		};
		require(
			pool.keys().copied().eq(tasks.iter().copied()),
			Error::BackendState("final host partition differs from its warm pending pool"),
		)?;
		let contracts = task_contracts(bundle, Some(tasks))?;
		for (task, pending) in pool {
			let expected = contracts.get(task).ok_or(Error::Protocol {
				task: *task,
				detail: "final host task is absent from its warm pending pool",
			})?;
			require(
				pending.admission == expected.admission(),
				Error::Protocol {
					task: *task,
					detail: "final host admission differs from its warm manifest",
				},
			)?;
		}
		self.contracts = contracts;
		Ok(())
	}

	#[doc(hidden)]
	pub fn destroy(self) -> Result<()> {
		let HostResources {
			runtime,
			reservations,
			pending,
			..
		} = self;
		drop(pending);
		destroy_host_resources(runtime, reservations)
	}
}

impl Backend for HostBackend {
	type Arena = Arena;
	type Error = Error;
	type Pending = HostPending;
	type Resource = HostResources;

	fn bind_resources(
		&mut self,
		bundle: &FinalizedBundle,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Resource> {
		record(physical_calls, PhysicalCall::BindResources)?;
		let state = core::mem::replace(&mut self.state, BackendState::Bound);
		match state {
			BackendState::Ready(config) => HostResources::realize(bundle, config),
			BackendState::Prepared(resources) => {
				let tasks = bundle.tasks().iter().map(|task| task.id).collect();
				resources.bind(bundle, &tasks)
			}
			BackendState::Warmed(mut resources) => {
				let tasks = bundle.tasks().iter().map(|task| task.id).collect();
				resources.validate_handoff(bundle, &tasks)?;
				Ok(resources)
			}
			BackendState::Bound => Err(Error::BackendState("resources may be bound only once")),
		}
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Pending> {
		record(
			physical_calls,
			PhysicalCall::PreparePending { task: request.task },
		)?;
		resource.prepare_pending(request)
	}

	fn allocate_arena(
		&mut self,
		resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Arena> {
		record(
			physical_calls,
			PhysicalCall::AllocateArena {
				device: layout.device,
				bytes: layout.size,
			},
		)?;
		resource.allocate_arena(layout)
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<()> {
		record(physical_calls, submission_call(&work))?;
		resource.submit(&arenas, pending, work)
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<BackendPoll> {
		let result = resource.poll_pending(pending);
		let status = match &result {
			Ok(BackendPoll::Pending) => PhysicalPollStatus::Pending,
			Ok(BackendPoll::Complete { .. }) => PhysicalPollStatus::Complete,
			Err(error) => match error {
				Error::InvalidConfiguration(_)
				| Error::InvalidPath
				| Error::DuplicateDevice(_)
				| Error::MissingDevice(_)
				| Error::WrongDeviceKind(_)
				| Error::BackendState(_)
				| Error::Protocol { .. }
				| Error::UnsupportedWork { .. }
				| Error::PhysicalReportOverflow
				| Error::RangeOverflow
				| Error::OutOfBounds { .. }
				| Error::SlotCapacityExhausted
				| Error::SlotBusy
				| Error::InvalidPendingState
				| Error::WorkerFailed { .. }
				| Error::Io { .. }
				| Error::ThreadSpawn(_)
				| Error::ThreadPanicked
				| Error::Poisoned => PhysicalPollStatus::Failed,
			},
		};
		record(
			physical_calls,
			PhysicalCall::Poll {
				task: pending.task,
				status,
			},
		)?;
		result
	}

	fn collect_exit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: TransferWork<'_>,
		destination: &mut [u8],
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<()> {
		consume_context(arenas);
		record(
			physical_calls,
			PhysicalCall::CollectExit {
				task: work.task,
				bytes: work.bytes,
			},
		)?;
		resource.collect_exit(pending, work, destination)
	}

	fn release_arena(
		&mut self,
		resource: &mut Self::Resource,
		device: DeviceId,
		arena: Self::Arena,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<()> {
		record(physical_calls, PhysicalCall::ReleaseArena { device })?;
		resource.ensure_healthy()?;
		if arena.device() != device {
			return Err(Error::Protocol {
				task: TaskId::new(0),
				detail: "released host arena belongs to another device",
			});
		}
		arena.close()
	}

	fn destroy_resources(&mut self, resource: Self::Resource, physical_calls: &mut PhysicalCallBatch) -> Result<()> {
		record(physical_calls, PhysicalCall::DestroyResources)?;
		resource.destroy()
	}
}

fn validate_bindings(bindings: &[HostDeviceBinding]) -> Result<()> {
	let mut devices = BTreeSet::new();
	let mut paths = BTreeSet::new();
	for binding in bindings {
		let device = binding.device();
		if !devices.insert(device) {
			return Err(Error::DuplicateDevice(device));
		}
		if let HostDeviceBinding::Disk {
			arena, reservation, ..
		} = binding && (arena.path() == reservation.path()
			|| !paths.insert(arena.path().to_owned())
			|| !paths.insert(reservation.path().to_owned()))
		{
			return Err(Error::InvalidConfiguration(
				"host disk arena and reservation paths must be globally unique",
			));
		}
	}
	Ok(())
}

fn index_bindings(bindings: Vec<HostDeviceBinding>) -> Result<BTreeMap<DeviceId, HostDeviceBinding>> {
	validate_bindings(&bindings)?;
	Ok(bindings
		.into_iter()
		.map(|binding| (binding.device(), binding))
		.collect())
}

fn validate_bundle_devices(
	bundle: &FinalizedBundle,
	bindings: &BTreeMap<DeviceId, HostDeviceBinding>,
	require_complete_coverage: bool,
) -> Result<()> {
	let layouts = bundle
		.arena_layouts()
		.iter()
		.map(|layout| layout.device)
		.collect::<BTreeSet<_>>();
	if require_complete_coverage {
		for device in &layouts {
			if !bindings.contains_key(device) {
				return Err(Error::MissingDevice(*device));
			}
		}
	}
	for device in bindings.keys() {
		if !layouts.contains(device) {
			return Err(Error::InvalidConfiguration(
				"host binding names a device absent from the finalized bundle",
			));
		}
	}
	Ok(())
}

fn allocate_reservations(
	bundle: &FinalizedBundle,
	bindings: &BTreeMap<DeviceId, HostDeviceBinding>,
) -> Result<BTreeMap<DeviceId, Reservation>> {
	allocate_reservations_from_ledger(bundle.reservations(), bindings)
}

fn allocate_reservations_from_ledger(
	ledger: &ReservationLedger,
	bindings: &BTreeMap<DeviceId, HostDeviceBinding>,
) -> Result<BTreeMap<DeviceId, Reservation>> {
	let mut reservations = BTreeMap::new();
	for (device, binding) in bindings {
		let entry = ledger.entry(*device).ok_or(Error::MissingDevice(*device))?;
		if entry.mechanism != recipe_core::ReservationMechanism::HeldAllocation {
			release_reservations(reservations);
			return Err(Error::InvalidConfiguration(
				"host backend requires a held-allocation reservation",
			));
		}
		let reservation = match binding {
			HostDeviceBinding::Ram { .. } => Reservation::ram(*device, entry.bytes),
			HostDeviceBinding::Disk { reservation, .. } => {
				Reservation::disk(*device, reservation.clone(), entry.bytes)
			}
		};
		match reservation {
			Ok(reservation) => {
				reservations.insert(*device, reservation);
			}
			Err(error) => {
				release_reservations(reservations);
				return Err(error);
			}
		}
	}
	Ok(reservations)
}

fn destroy_host_resources(runtime: Runtime, reservations: BTreeMap<DeviceId, Reservation>) -> Result<()> {
	let mut first = runtime.close().err();
	for (_, reservation) in reservations {
		if let Err(error) = reservation.release()
			&& first.is_none()
		{
			first = Some(error);
		}
	}
	match first {
		Some(error) => Err(error),
		None => Ok(()),
	}
}

fn available_ram_bytes() -> Result<ByteCount> {
	let contents = fs::read_to_string("/proc/meminfo").map_err(|error| Error::Io {
		operation: "read /proc/meminfo",
		kind: error.kind(),
	})?;
	let kilobytes = contents
		.lines()
		.find_map(|line| {
			let value = line.strip_prefix("MemAvailable:")?;
			value.split_whitespace().next()?.parse::<u64>().ok()
		})
		.ok_or(Error::InvalidConfiguration(
			"/proc/meminfo has no numeric MemAvailable field",
		))?;
	let bytes = kilobytes.checked_mul(1024).ok_or(Error::RangeOverflow)?;
	Ok(ByteCount::new(bytes))
}

fn available_disk_bytes(path: &Path) -> Result<ByteCount> {
	let directory = path.parent().ok_or(Error::InvalidPath)?;
	let statistics = statvfs(directory).map_err(|error| Error::Io {
		operation: "query disk capacity",
		kind: error.kind(),
	})?;
	let bytes = statistics
		.f_bavail
		.checked_mul(statistics.f_frsize)
		.ok_or(Error::RangeOverflow)?;
	Ok(ByteCount::new(bytes))
}

fn release_reservations(reservations: BTreeMap<DeviceId, Reservation>) {
	for (_, reservation) in reservations {
		drop(reservation.release());
	}
}

fn prepare_candidate_pending(
	runtime: &Runtime,
	draft: &DraftPlan,
	selected: &BTreeSet<TaskId>,
) -> Result<BTreeMap<TaskId, HostPending>> {
	let values = draft
		.values
		.iter()
		.map(|value| (value.id, (value.device, value.dtype)))
		.collect::<BTreeMap<_, _>>();
	let mut pending = BTreeMap::new();
	for task in draft
		.tasks
		.iter()
		.filter(|task| selected.contains(&task.id))
	{
		let (class, staging, admission) = match &task.kind {
			TaskKind::Calculation(_) => {
				return Err(Error::Protocol {
					task: task.id,
					detail: "host candidate partition contains a calculation",
				});
			}
			TaskKind::Metric(metric) => {
				let (device, dtype) = values.get(&metric.value).copied().ok_or(Error::Protocol {
					task: task.id,
					detail: "host candidate metric value is absent",
				})?;
				(
					WorkClass::Metric,
					Some((device, ByteCount::new(4), PendingAction::Metric { dtype })),
					None,
				)
			}
			TaskKind::Transfer(transfer) => {
				let class =
					candidate_transfer_class(task.id, task.phase, transfer.source, transfer.destination)?;
				let staging = candidate_transfer_staging(
					task.id,
					class,
					transfer.source,
					transfer.destination,
					transfer.bytes,
				)?;
				let admission = match (class, transfer.destination) {
					(WorkClass::InitAdmission, TransferEndpoint::Device { device, value }) => {
						let manifest = draft
							.init_images
							.iter()
							.find(|manifest| manifest.device == device)
							.ok_or(Error::Protocol {
								task: task.id,
								detail: "host candidate admission has no init-image manifest",
							})?;
						require(
							manifest.image == value && manifest.bytes == transfer.bytes,
							Error::Protocol {
								task: task.id,
								detail: "host candidate admission differs from its init-image manifest",
							},
						)?;
						Some(InitImageContract {
							device,
							image: value,
							bytes: transfer.bytes,
						})
					}
					(WorkClass::InitAdmission, TransferEndpoint::External) => {
						return Err(Error::Protocol {
							task: task.id,
							detail: "host candidate admission has no device image destination",
						});
					}
					(
						WorkClass::InternalTransfer | WorkClass::ExitTransfer,
						TransferEndpoint::External | TransferEndpoint::Device { .. },
					) => None,
					(
						WorkClass::Calculation | WorkClass::Metric,
						TransferEndpoint::External | TransferEndpoint::Device { .. },
					) => {
						return Err(Error::Protocol {
							task: task.id,
							detail: "host candidate transfer has a non-transfer class",
						});
					}
				};
				(class, staging, admission)
			}
		};
		let copy = runtime.prepare_copy()?;
		let (staging, action) = match staging {
			Some((device, bytes, action)) => (Some(Arena::ram(device, bytes)?), action),
			None => (None, PendingAction::None),
		};
		let resource = HostPending {
			task: task.id,
			class,
			copy,
			staging,
			admission,
			action,
			submitted: false,
			terminal: false,
		};
		reject_duplicate(
			pending.insert(task.id, resource),
			Error::Protocol {
				task: task.id,
				detail: "host candidate task appears more than once",
			},
		)?;
	}
	Ok(pending)
}

fn reject_duplicate<T>(prior: Option<T>, error: Error) -> Result<()> {
	match prior {
		Some(value) => {
			drop(value);
			Err(error)
		}
		None => Ok(()),
	}
}

fn candidate_transfer_class(
	task: TaskId,
	phase: RunPhase,
	source: TransferEndpoint,
	destination: TransferEndpoint,
) -> Result<WorkClass> {
	match (phase, source, destination) {
		(RunPhase::Init, TransferEndpoint::External, TransferEndpoint::Device { .. }) => {
			Ok(WorkClass::InitAdmission)
		}
		(RunPhase::Init | RunPhase::Loop, TransferEndpoint::Device { .. }, TransferEndpoint::Device { .. }) => {
			Ok(WorkClass::InternalTransfer)
		}
		(
			RunPhase::Exit,
			TransferEndpoint::Device { .. },
			TransferEndpoint::Device { .. } | TransferEndpoint::External,
		) => Ok(WorkClass::ExitTransfer),
		(RunPhase::Init, TransferEndpoint::External, TransferEndpoint::External)
		| (RunPhase::Init, TransferEndpoint::Device { .. }, TransferEndpoint::External)
		| (
			RunPhase::Loop,
			TransferEndpoint::External,
			TransferEndpoint::External | TransferEndpoint::Device { .. },
		)
		| (RunPhase::Loop, TransferEndpoint::Device { .. }, TransferEndpoint::External)
		| (
			RunPhase::Exit,
			TransferEndpoint::External,
			TransferEndpoint::External | TransferEndpoint::Device { .. },
		) => Err(Error::Protocol {
			task,
			detail: "host candidate transfer phase or endpoint class is invalid",
		}),
	}
}

fn candidate_transfer_staging(
	task: TaskId,
	class: WorkClass,
	source: TransferEndpoint,
	destination: TransferEndpoint,
	bytes: ByteCount,
) -> Result<Option<(DeviceId, ByteCount, PendingAction)>> {
	match (class, source, destination) {
		(WorkClass::InitAdmission, TransferEndpoint::External, TransferEndpoint::Device { device, .. }) => {
			Ok(Some((device, bytes, PendingAction::None)))
		}
		(WorkClass::ExitTransfer, TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => {
			Ok(Some((device, bytes, PendingAction::Egress)))
		}
		(
			WorkClass::InternalTransfer | WorkClass::ExitTransfer,
			TransferEndpoint::Device { .. },
			TransferEndpoint::Device { .. },
		) => Ok(None),
		(
			WorkClass::Calculation | WorkClass::Metric,
			TransferEndpoint::External | TransferEndpoint::Device { .. },
			TransferEndpoint::External | TransferEndpoint::Device { .. },
		)
		| (
			WorkClass::InitAdmission | WorkClass::InternalTransfer,
			TransferEndpoint::External | TransferEndpoint::Device { .. },
			TransferEndpoint::External,
		)
		| (WorkClass::InitAdmission, TransferEndpoint::Device { .. }, TransferEndpoint::Device { .. })
		| (
			WorkClass::InternalTransfer | WorkClass::ExitTransfer,
			TransferEndpoint::External,
			TransferEndpoint::Device { .. },
		)
		| (WorkClass::ExitTransfer, TransferEndpoint::External, TransferEndpoint::External) => Err(Error::Protocol {
			task,
			detail: "host candidate transfer staging contract is invalid",
		}),
	}
}

fn task_contracts(
	bundle: &FinalizedBundle,
	selected: Option<&BTreeSet<TaskId>>,
) -> Result<BTreeMap<TaskId, ExpectedWork>> {
	let mut contracts = BTreeMap::new();
	for task in bundle.tasks() {
		if selected.is_some_and(|selected| !selected.contains(&task.id)) {
			continue;
		}
		let expected = match &task.kind {
			TaskKind::Calculation(_) => {
				return Err(Error::UnsupportedWork {
					task: task.id,
					detail: "host-only backend cannot bind a GPU calculation task",
				});
			}
			TaskKind::Metric(metric) => ExpectedWork::Metric {
				metric: metric.metric,
				slot: metric.slot,
				value: *bundle.value_location(metric.value).ok_or(Error::Protocol {
					task: task.id,
					detail: "metric value has no finalized location",
				})?,
			},
			TaskKind::Transfer(transfer) => {
				let endpoints = bundle.transfer_endpoints(task.id).ok_or(Error::Protocol {
					task: task.id,
					detail: "transfer has no finalized endpoints",
				})?;
				let class = transfer_class(task.id, task.phase, endpoints.source, endpoints.destination)?;
				match class {
					WorkClass::InitAdmission => {
						let ResolvedTransferEndpoint::Device(destination) = endpoints.destination else {
							return Err(Error::Protocol {
								task: task.id,
								detail: "host admission has no device destination",
							});
						};
						let manifest = bundle
							.init_image(destination.device)
							.ok_or(Error::Protocol {
								task: task.id,
								detail: "host admission device has no finalized init-image manifest",
							})?;
						require(
							manifest.image == destination.value && manifest.bytes == transfer.bytes,
							Error::Protocol {
								task: task.id,
								detail: "host admission differs from the finalized init-image manifest",
							},
						)?;
						ExpectedWork::Init {
							image: manifest.image,
							destination,
							bytes: transfer.bytes,
							submission: transfer.submission,
						}
					}
					WorkClass::InternalTransfer | WorkClass::ExitTransfer => {
						if transfer.route.len() > 1 {
							return Err(Error::UnsupportedWork {
								task: task.id,
								detail: "host backend requires planner-expanded one-hop transfers",
							});
						}
						ExpectedWork::Transfer {
							class,
							source: endpoints.source,
							destination: endpoints.destination,
							bytes: transfer.bytes,
							route: transfer.route.clone(),
							lane_claims: transfer.lane_claims.clone(),
							submission: transfer.submission,
						}
					}
					WorkClass::Calculation | WorkClass::Metric => {
						return Err(Error::Protocol {
							task: task.id,
							detail: "host transfer received a non-transfer work class",
						});
					}
				}
			}
		};
		if contracts.insert(task.id, expected).is_some() {
			return Err(Error::Protocol {
				task: task.id,
				detail: "host task appears more than once",
			});
		}
	}
	Ok(contracts)
}

fn transfer_class(
	task: TaskId,
	phase: RunPhase,
	source: ResolvedTransferEndpoint,
	destination: ResolvedTransferEndpoint,
) -> Result<WorkClass> {
	use ResolvedTransferEndpoint::{Device, External};
	match (phase, source, destination) {
		(RunPhase::Init, External, Device(_)) => Ok(WorkClass::InitAdmission),
		(RunPhase::Init | RunPhase::Loop, Device(_), Device(_)) => Ok(WorkClass::InternalTransfer),
		(RunPhase::Exit, Device(_), Device(_) | External) => Ok(WorkClass::ExitTransfer),
		(RunPhase::Init, External, External)
		| (RunPhase::Init, Device(_), External)
		| (RunPhase::Loop, External, External | Device(_))
		| (RunPhase::Loop, Device(_), External)
		| (RunPhase::Exit, External, External | Device(_)) => Err(Error::Protocol {
			task,
			detail: "host transfer phase or endpoint class is invalid",
		}),
	}
}

const fn phase_accepts(phase: RunPhase, class: WorkClass) -> bool {
	match class {
		WorkClass::InitAdmission => matches!(phase, RunPhase::Init),
		WorkClass::Calculation | WorkClass::Metric => matches!(phase, RunPhase::Loop),
		WorkClass::InternalTransfer => {
			matches!(phase, RunPhase::Init | RunPhase::Loop)
		}
		WorkClass::ExitTransfer => matches!(phase, RunPhase::Exit),
	}
}

fn checked_arena(
	arenas: &(impl HostArenaLookup + ?Sized),
	location: ResolvedValueLocation,
	bytes: u64,
) -> Result<&Arena> {
	let arena = arenas
		.host_arena(location.device)
		.ok_or(Error::MissingDevice(location.device))?;
	let end = location
		.arena_offset
		.get()
		.checked_add(bytes)
		.ok_or(Error::RangeOverflow)?;
	if arena.device() != location.device || end > arena.bytes().get() {
		return Err(Error::OutOfBounds {
			device: location.device,
		});
	}
	Ok(arena)
}

struct TransferExpectation<'a> {
	submitted_class: WorkClass,
	class: WorkClass,
	source: ResolvedTransferEndpoint,
	destination: ResolvedTransferEndpoint,
	bytes: ByteCount,
	route: &'a [recipe_core::LinkId],
	lane_claims: &'a [TransferLaneClaim],
	submission: SubmissionSlots,
}

fn validate_transfer(work: TransferWork<'_>, expected: TransferExpectation<'_>) -> Result<()> {
	if expected.submitted_class != expected.class
		|| work.source != expected.source
		|| work.destination != expected.destination
		|| work.bytes != expected.bytes
		|| work.route != expected.route
		|| work.lane_claims != expected.lane_claims
		|| work.submission != expected.submission
	{
		return Err(Error::Protocol {
			task: work.task,
			detail: "host transfer differs from its finalized contract",
		});
	}
	Ok(())
}

fn submit_transfer(
	arenas: &(impl HostArenaLookup + ?Sized),
	pending: &mut HostPending,
	work: TransferWork<'_>,
) -> Result<()> {
	match (work.source, work.destination) {
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => {
			let source_arena = checked_arena(arenas, source, work.bytes.get())?;
			let destination_arena = checked_arena(arenas, destination, work.bytes.get())?;
			pending.copy.submit(
				source_arena,
				source.arena_offset.get(),
				destination_arena,
				destination.arena_offset.get(),
				work.bytes,
			)
		}
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::External) => {
			let source_arena = checked_arena(arenas, source, work.bytes.get())?;
			let staging = pending.staging.as_ref().ok_or(Error::Protocol {
				task: work.task,
				detail: "host egress has no preallocated staging arena",
			})?;
			pending.copy.submit(
				source_arena,
				source.arena_offset.get(),
				staging,
				0,
				work.bytes,
			)
		}
		(ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::External)
		| (ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::Device(_)) => Err(Error::Protocol {
			task: work.task,
			detail: "host transfer has unsupported resolved endpoints",
		}),
	}
}

fn submission_call(work: &BackendWork<'_>) -> PhysicalCall {
	match work {
		BackendWork::InitAdmission(work) => PhysicalCall::AdmissionChunk {
			task: work.task,
			device: work.destination.device,
			bytes: work.bytes,
			chunk_index: 0,
		},
		BackendWork::Calculation(work) => PhysicalCall::SubmitCalculation { task: work.task },
		BackendWork::InternalTransfer(work) => PhysicalCall::SubmitInternalTransfer { task: work.task },
		BackendWork::Metric(work) => PhysicalCall::SubmitMetric {
			task: work.task,
			slot: work.slot,
		},
		BackendWork::ExitTransfer(work) => PhysicalCall::SubmitExitTransfer { task: work.task },
	}
}

fn record(batch: &mut PhysicalCallBatch, call: PhysicalCall) -> Result<()> {
	match batch.try_push(call) {
		Ok(()) => Ok(()),
		Err(overflow) => {
			consume_context(overflow);
			Err(Error::PhysicalReportOverflow)
		}
	}
}
