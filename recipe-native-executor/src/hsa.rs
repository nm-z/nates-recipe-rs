//! ROCr/HSA resource realization and submission bridge.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::time::Duration;

use recipe_core::{
	ArenaLayout, ArtifactId, BundleIdentity, ByteCount, CompletionSlotId, DeviceId, DraftPlan, FinalizedBundle,
	InitDataImage, LinkId, QueueSlotId, ReservationLedger, ReservationMechanism, ResolvedTransferEndpoint,
	ResolvedValueLocation, ResourceManifest, RunPhase, SubmissionSlots, Task, TaskId, TaskKind, TransferEndpoint,
	TransferLaneClaim, ValueId,
};
use recipe_executor::{
	ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork, MetricValue, MetricWork,
	PendingRequest, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus, TransferWork, WorkClass, sealed,
};
use recipe_hsa::{
	Allocation, DispatchGeometry, Executable, Kernel, PollStatus, PreparedPending, Queue, QueueConfig, QueueKind,
	QueueProgress, Session,
};
use recipe_kernel::{KernelArgument, inspect_hsaco};

use crate::plan::InitImageContract;
use crate::{Error, ExecutionPlan, Result, RuntimeArtifactKind};

#[derive(Clone)]
pub struct HsaBinding<'scope> {
	device: DeviceId,
	session: &'scope Session<'scope>,
	target_id: String,
	code_object_version: u8,
	queue_packets: u32,
}

impl<'scope> HsaBinding<'scope> {
	#[must_use]
	pub fn new(
		device: DeviceId,
		session: &'scope Session<'scope>,
		target_id: String,
		code_object_version: u8,
		queue_packets: u32,
	) -> Self {
		Self {
			device,
			session,
			target_id,
			code_object_version,
			queue_packets,
		}
	}

	#[must_use]
	pub const fn device(&self) -> DeviceId {
		self.device
	}

	pub(crate) const fn session(&self) -> &'scope Session<'scope> {
		self.session
	}
}

impl fmt::Debug for HsaBinding<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaBinding")
			.field("device", &self.device)
			.field("target_id", &self.target_id)
			.field("code_object_version", &self.code_object_version)
			.field("queue_packets", &self.queue_packets)
			.finish_non_exhaustive()
	}
}

pub(crate) trait HsaArenaLookup<'scope> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'scope>>;
}

impl<'scope> HsaArenaLookup<'scope> for BTreeMap<DeviceId, HsaArena<'scope>> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'scope>> {
		self.get(&device)
	}
}

impl<'borrow, 'scope> HsaArenaLookup<'scope> for ArenaSet<'borrow, HsaArena<'scope>> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'scope>> {
		self.get(device)
	}
}

pub struct HsaArena<'scope> {
	device: DeviceId,
	allocation: Allocation<'scope>,
}

impl<'scope> HsaArena<'scope> {
	pub(crate) fn device(&self) -> DeviceId {
		self.device
	}

	pub(crate) const fn allocation(&self) -> &Allocation<'scope> {
		&self.allocation
	}

	pub fn bytes(&self) -> usize {
		self.allocation.len()
	}

	pub(crate) fn release(self) -> Result<()> {
		self.allocation.close().map_err(Error::from)
	}
}

impl fmt::Debug for HsaArena<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaArena")
			.field("device", &self.device)
			.field("bytes", &self.allocation.len())
			.field("pool", &self.allocation.pool_index())
			.finish_non_exhaustive()
	}
}

struct LoadedArtifact<'scope> {
	kernel: Kernel<'scope, 'scope>,
	executable: Executable<'scope, 'scope>,
	abi: recipe_kernel::KernelAbi,
}

impl LoadedArtifact<'_> {
	fn close(self) -> Result<()> {
		let Self {
			kernel,
			executable,
			abi,
		} = self;
		drop(kernel);
		drop(abi);
		executable.close().map_err(Error::from)
	}
}

impl fmt::Debug for LoadedArtifact<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("LoadedArtifact")
			.field("entry", &self.abi.entry_symbol)
			.field("metadata", self.kernel.metadata())
			.finish()
	}
}

struct KernargSlot<'scope> {
	allocation: Allocation<'scope>,
	bytes: Vec<u8>,
}

impl fmt::Debug for KernargSlot<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("KernargSlot")
			.field("allocation_bytes", &self.allocation.len())
			.field("host_bytes", &self.bytes.len())
			.finish_non_exhaustive()
	}
}

#[derive(Clone, Copy, Debug)]
enum CompletionState {
	Available,
	Active { task: TaskId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HsaTaskContract {
	phase: RunPhase,
	class: WorkClass,
	submission: Option<SubmissionSlots>,
	admission: Option<InitImageContract>,
	route: Vec<LinkId>,
	lane_claims: Vec<TransferLaneClaim>,
}

struct DeviceResources<'scope> {
	session: &'scope Session<'scope>,
	queues: BTreeMap<QueueSlotId, Queue<'scope, 'scope>>,
	completions: BTreeMap<CompletionSlotId, CompletionState>,
	artifacts: BTreeMap<ArtifactId, LoadedArtifact<'scope>>,
	kernargs: BTreeMap<CompletionSlotId, KernargSlot<'scope>>,
	metric_buffers: BTreeMap<TaskId, Allocation<'scope>>,
	staging: Allocation<'scope>,
	admission: Option<InitImageContract>,
	egress: BTreeMap<TaskId, Vec<u8>>,
	scratch: Option<Allocation<'scope>>,
	reservation: Allocation<'scope>,
}

impl fmt::Debug for DeviceResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("DeviceResources")
			.field("agent", &self.session.description().identity)
			.field("queue_count", &self.queues.len())
			.field("completion_count", &self.completions.len())
			.field("artifact_count", &self.artifacts.len())
			.field("kernarg_count", &self.kernargs.len())
			.field("metric_buffer_count", &self.metric_buffers.len())
			.field("staging_bytes", &self.staging.len())
			.field("admission", &self.admission)
			.field("egress_count", &self.egress.len())
			.field("scratch_bytes", &self.scratch.as_ref().map(Allocation::len))
			.field("reservation_bytes", &self.reservation.len())
			.finish()
	}
}

pub struct HsaResources<'scope> {
	plan: ExecutionPlan,
	devices: BTreeMap<DeviceId, DeviceResources<'scope>>,
	contracts: BTreeMap<TaskId, HsaTaskContract>,
	prepared_tasks: BTreeSet<TaskId>,
	pending_pool: BTreeMap<TaskId, PreparedPending<'scope, 'scope>>,
	poisoned: bool,
}

pub(crate) struct HsaPreparedResources<'scope> {
	handoff: HsaPreparedHandoff,
	devices: BTreeMap<DeviceId, DeviceResources<'scope>>,
	pending: BTreeMap<TaskId, PreparedPending<'scope, 'scope>>,
}

#[derive(Debug)]
enum HsaPreparedHandoff {
	Candidate(Vec<crate::RuntimeArtifact>),
	Finalized {
		bundle: BundleIdentity,
		tasks: BTreeSet<TaskId>,
		plan: ExecutionPlan,
		contracts: BTreeMap<TaskId, HsaTaskContract>,
	},
}

#[derive(Debug)]
enum HsaBackendState<'scope> {
	Ready {
		bindings: Vec<HsaBinding<'scope>>,
		artifacts: Vec<crate::RuntimeArtifact>,
	},
	Prepared(HsaPreparedResources<'scope>),
	Warmed(HsaResources<'scope>),
	Bound,
}

#[derive(Debug)]
pub struct HsaBackend<'scope> {
	state: HsaBackendState<'scope>,
}

impl<'scope> HsaBackend<'scope> {
	pub fn new(bindings: Vec<HsaBinding<'scope>>, artifacts: Vec<crate::RuntimeArtifact>) -> Self {
		Self {
			state: HsaBackendState::Ready {
				bindings,
				artifacts,
			},
		}
	}

	pub(crate) const fn from_prepared(resources: HsaPreparedResources<'scope>) -> Self {
		Self {
			state: HsaBackendState::Prepared(resources),
		}
	}

	pub(crate) const fn from_warmed(resources: HsaResources<'scope>) -> Self {
		Self {
			state: HsaBackendState::Warmed(resources),
		}
	}

	pub(crate) fn bind_partition(
		&mut self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
	) -> Result<HsaResources<'scope>> {
		let prior = core::mem::replace(&mut self.state, HsaBackendState::Bound);
		match prior {
			HsaBackendState::Ready {
				bindings,
				artifacts,
			} => {
				let devices = bindings.iter().map(HsaBinding::device).collect();
				let plan = ExecutionPlan::validate_partition(bundle, artifacts, devices, tasks)?;
				HsaResources::realize(bundle, plan, bindings, Some(tasks))
			}
			HsaBackendState::Prepared(resources) => resources.bind(bundle, tasks),
			HsaBackendState::Warmed(mut resources) => {
				resources.validate_handoff(bundle, tasks)?;
				Ok(resources)
			}
			HsaBackendState::Bound => Err(Error::BackendState {
				backend: "HSA",
				detail: "resources may be bound only once",
			}),
		}
	}
}

impl sealed::Sealed for HsaBackend<'_> {}

impl fmt::Debug for HsaResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaResources")
			.field("plan", &self.plan)
			.field("devices", &self.devices)
			.field("contracts", &self.contracts)
			.field("prepared_task_count", &self.prepared_tasks.len())
			.field("poisoned", &self.poisoned)
			.finish()
	}
}

impl fmt::Debug for HsaPreparedResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaPreparedResources")
			.field("handoff", &self.handoff)
			.field("devices", &self.devices)
			.finish()
	}
}

impl<'scope> HsaResources<'scope> {
	pub(crate) fn realize(
		bundle: &FinalizedBundle,
		plan: ExecutionPlan,
		bindings: Vec<HsaBinding<'scope>>,
		tasks: Option<&BTreeSet<TaskId>>,
	) -> Result<Self> {
		let mut binding_by_device = BTreeMap::new();
		for binding in bindings {
			let device = binding.device;
			ensure(
				binding_by_device.insert(device, binding).is_none(),
				Error::DuplicateDevice { device },
			)?;
		}
		for device in plan.devices() {
			ensure(
				binding_by_device.contains_key(&device),
				Error::MissingDevice { device },
			)?;
		}
		reject_unexpected_device(
			binding_by_device
				.keys()
				.copied()
				.find(|device| !plan.devices().any(|planned| planned == *device)),
		)?;

		let mut devices = BTreeMap::new();
		let runtime_by_id = plan
			.runtime_artifacts()
			.map(|runtime| (runtime.id, runtime.clone()))
			.collect::<BTreeMap<_, _>>();
		let value_devices = bundle
			.value_locations()
			.iter()
			.map(|location| (location.value, location.device))
			.collect::<BTreeMap<_, _>>();
		let scoped_tasks = bundle
			.tasks()
			.iter()
			.filter(|task| tasks.is_none_or(|selected| selected.contains(&task.id)))
			.cloned()
			.collect::<Vec<_>>();
		for (device, binding) in &binding_by_device {
			validate_binding(binding)?;
			devices.insert(
				*device,
				realize_device(
					bundle.resources(),
					bundle.reservations(),
					&scoped_tasks,
					&value_devices,
					bundle.init_images(),
					&runtime_by_id,
					binding,
				)?,
			);
		}
		let contracts = task_contracts(bundle, tasks)?;
		let pending_pool = prepare_pending_pool(&devices, &scoped_tasks, &value_devices)?;
		Ok(Self {
			plan,
			devices,
			contracts,
			prepared_tasks: BTreeSet::new(),
			pending_pool,
			poisoned: false,
		})
	}

	pub(crate) fn allocate_arena(&self, layout: &ArenaLayout) -> Result<HsaArena<'scope>> {
		let owner = self
			.devices
			.get(&layout.device)
			.ok_or(Error::MissingDevice {
				device: layout.device,
			})?;
		let allocation = owner
			.session
			.allocate_coarse(bytes_to_usize(layout.size.get(), "HSA arena size")?)?;
		for resources in self.devices.values() {
			resources.session.grant_access(&allocation)?;
		}
		Ok(HsaArena {
			device: layout.device,
			allocation,
		})
	}

	pub(crate) fn prepare_pending(&mut self, request: PendingRequest) -> Result<HsaPending<'scope>> {
		self.ensure_healthy()?;
		let contract = self.contracts.get(&request.task).ok_or(Error::Protocol {
			task: request.task,
			detail: "pending request names no finalized HSA task",
		})?;
		ensure(
			request.phase == contract.phase
				&& request.class == contract.class
				&& request.submission == contract.submission,
			Error::Protocol {
				task: request.task,
				detail: "pending request differs from the finalized HSA task contract",
			},
		)?;
		let planned = self.plan.submission(request.task).ok_or(Error::Protocol {
			task: request.task,
			detail: "task has no immutable native submission assignment",
		})?;
		let device = self
			.devices
			.get(&planned.device)
			.ok_or(Error::MissingDevice {
				device: planned.device,
			})?;
		ensure(
			device.queues.contains_key(&planned.slots.queue)
				&& device.completions.contains_key(&planned.slots.completion),
			Error::Protocol {
				task: request.task,
				detail: "prepared task references an unrealized HSA submission slot",
			},
		)?;
		ensure(
			!self.prepared_tasks.contains(&request.task),
			Error::Protocol {
				task: request.task,
				detail: "HSA pending token was prepared more than once",
			},
		)?;
		let native = self.pending_pool.remove(&request.task).ok_or(Error::Protocol {
			task: request.task,
			detail: "HSA pre-final pending token is absent",
		})?;
		ensure(
			self.prepared_tasks.insert(request.task),
			Error::Protocol {
				task: request.task,
				detail: "HSA pending token was prepared more than once",
			},
		)?;
		Ok(HsaPending::ready(request, planned, native))
	}

	pub(crate) fn submit(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		pending: &mut HsaPending<'scope>,
		work: BackendWork<'_>,
	) -> Result<()> {
		self.ensure_healthy()?;
		let task = work.task();
		let planned = self.plan.submission(task).ok_or(Error::Protocol {
			task,
			detail: "task has no immutable native submission assignment",
		})?;
		pending.validate_ready(&work, planned)?;
		self.validate_work_contract(&work)?;
		let result = match work {
			BackendWork::InitAdmission(work) => self.submit_admission(arenas, planned, work, &mut pending.native),
			BackendWork::InternalTransfer(work) => {
				self.submit_internal_transfer(arenas, planned, &work, &mut pending.native)
			}
			BackendWork::Calculation(work) => {
				self.submit_calculation(arenas, planned, &work, &mut pending.native)
			}
			BackendWork::Metric(work) => self.submit_metric(arenas, planned, work, &mut pending.native),
			BackendWork::ExitTransfer(work) => {
				self.submit_exit_transfer(arenas, planned, &work, &mut pending.native)
			}
		};
		match result {
			Ok(action) => {
				pending.activate(action);
				Ok(())
			}
			Err(error) => {
				self.poisoned = true;
				Err(error)
			}
		}
	}

	pub(crate) fn poll_pending(&mut self, pending: &mut HsaPending<'scope>) -> Result<BackendPoll> {
		self.ensure_healthy()?;
		validate_active(self.device_mut(pending.device)?, pending)?;
		ensure(
			matches!(pending.state, HsaPendingState::Active),
			Error::Protocol {
				task: pending.task,
				detail: "HSA pending token is not active",
			},
		)?;
		let status = match pending.native.poll() {
			Ok(status) => status,
			Err(error) => return self.poison(Error::from(error)),
		};
		match status {
			PollStatus::Pending => Ok(BackendPoll::Pending),
			PollStatus::Complete => self.finish_pending(pending),
		}
	}

	pub fn take_egress(&mut self, task: TaskId) -> Option<Vec<u8>> {
		self.devices
			.values_mut()
			.find_map(|device| device.egress.remove(&task))
	}

	pub(crate) fn collect_exit(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		pending: &HsaPending<'scope>,
		work: TransferWork<'_>,
		destination: &mut [u8],
	) -> Result<()> {
		self.ensure_healthy()?;
		self.validate_work_contract(&BackendWork::ExitTransfer(work))?;
		pending.validate_collected_exit(&work, destination.len())?;
		let ResolvedTransferEndpoint::Device(source_location) = work.source else {
			return Err(Error::UnsupportedTransfer {
				task: work.task,
				detail: "HSA exit collection has no device source",
			});
		};
		checked_arena(arenas, source_location, work.bytes.get())?;
		let device = self.device_mut(pending.device)?;
		let source = device.egress.get(&work.task).ok_or(Error::Protocol {
			task: work.task,
			detail: "completed HSA exit has no preallocated host result",
		})?;
		ensure(
			source.len() == destination.len(),
			Error::Protocol {
				task: work.task,
				detail: "completed HSA exit size differs from caller storage",
			},
		)?;
		destination.copy_from_slice(source);
		Ok(())
	}

	fn validate_work_contract(&self, work: &BackendWork<'_>) -> Result<()> {
		let task = work.task();
		let contract = self.contracts.get(&task).ok_or(Error::Protocol {
			task,
			detail: "submitted work names no finalized HSA task",
		})?;
		ensure(
			work.class() == contract.class && work_submission(work) == contract.submission,
			Error::Protocol {
				task,
				detail: "submitted work class or slots differ from the finalized HSA task",
			},
		)?;
		match work {
			BackendWork::InitAdmission(admission) => ensure(
				contract.admission
					== Some(InitImageContract {
						device: admission.destination.device,
						image: admission.destination.value,
						bytes: admission.bytes,
					}),
				Error::Protocol {
					task,
					detail: "submitted HSA admission differs from the finalized init-image manifest",
				},
			),
			BackendWork::InternalTransfer(transfer) | BackendWork::ExitTransfer(transfer) => ensure(
				transfer.route == contract.route && transfer.lane_claims == contract.lane_claims,
				Error::Protocol {
					task,
					detail: "submitted HSA route or lane claims differ from the finalized transfer",
				},
			),
			BackendWork::Calculation(_) | BackendWork::Metric(_) => Ok(()),
		}
	}

	pub(crate) fn destroy(self) -> Result<()> {
		self.ensure_healthy()?;
		destroy_devices(self.devices)
	}

	fn submit_admission(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		planned: crate::PlannedSubmission,
		work: InitAdmissionWork<'_>,
		native: &mut PreparedPending<'scope, 'scope>,
	) -> Result<PendingAction> {
		ensure(
			planned.device == work.destination.device && work.submission == planned.slots,
			Error::Protocol {
				task: work.task,
				detail: "HSA admission device or slots differ from immutable submission",
			},
		)?;
		let arena = checked_arena(arenas, work.destination, work.bytes.get())?;
		let device = self.device_mut(planned.device)?;
		let bytes = bytes_to_usize(work.bytes.get(), "HSA admission byte count")?;
		ensure(
			device.admission
				== Some(InitImageContract {
					device: work.destination.device,
					image: work.destination.value,
					bytes: work.bytes,
				}) && work.image.len() == bytes
				&& device.staging.len() >= bytes,
			Error::Protocol {
				task: work.task,
				detail: "admission image or fine staging size differs from finalized bytes",
			},
		)?;
		let copy = unsafe {
			// SAFETY: `staging` was allocated from a discovered fine-grained
			// host-accessible pool, and no operation is using it during init.
			device.staging.copy_from_host(0, work.image)
		};
		copy?;
		let destination_offset = offset_to_usize(work.destination.arena_offset.get())?;
		ensure_queue(device, work.task, planned.slots.queue)?;
		claim_completion(&mut device.completions, work.task, planned.slots.completion)?;
		let submission = device.session.copy_async_prepared(
			&arena.allocation,
			destination_offset,
			&device.staging,
			0,
			bytes,
			native,
		);
		finish_submission_claim(
			&mut device.completions,
			work.task,
			planned.slots.completion,
			submission,
		)?;
		Ok(PendingAction::None)
	}

	fn submit_internal_transfer(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		planned: crate::PlannedSubmission,
		work: &TransferWork,
		native: &mut PreparedPending<'scope, 'scope>,
	) -> Result<PendingAction> {
		let (source, destination) = device_endpoints(work)?;
		ensure(
			source.device == planned.device && work.submission == planned.slots,
			Error::Protocol {
				task: work.task,
				detail: "HSA transfer source or slots differ from immutable submission",
			},
		)?;
		let source_arena = checked_arena(arenas, source, work.bytes.get())?;
		let destination_arena = checked_arena(arenas, destination, work.bytes.get())?;
		let source_offset = offset_to_usize(source.arena_offset.get())?;
		let destination_offset = offset_to_usize(destination.arena_offset.get())?;
		let bytes = bytes_to_usize(work.bytes.get(), "HSA internal copy byte count")?;
		let device = self.device_mut(planned.device)?;
		ensure_queue(device, work.task, planned.slots.queue)?;
		claim_completion(&mut device.completions, work.task, planned.slots.completion)?;
		let submission = device.session.copy_async_prepared(
			&destination_arena.allocation,
			destination_offset,
			&source_arena.allocation,
			source_offset,
			bytes,
			native,
		);
		finish_submission_claim(
			&mut device.completions,
			work.task,
			planned.slots.completion,
			submission,
		)?;
		Ok(PendingAction::None)
	}

	fn submit_exit_transfer(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		planned: crate::PlannedSubmission,
		work: &TransferWork,
		native: &mut PreparedPending<'scope, 'scope>,
	) -> Result<PendingAction> {
		let ResolvedTransferEndpoint::Device(source) = work.source else {
			return Err(Error::UnsupportedTransfer {
				task: work.task,
				detail: "HSA exit transfer has no device source",
			});
		};
		ensure(
			source.device == planned.device && work.submission == planned.slots,
			Error::UnsupportedTransfer {
				task: work.task,
				detail: "HSA exit source or slots differ from immutable submission",
			},
		)?;
		let source_arena = checked_arena(arenas, source, work.bytes.get())?;
		let bytes = bytes_to_usize(work.bytes.get(), "HSA egress byte count")?;
		let source_offset = offset_to_usize(source.arena_offset.get())?;
		match work.destination {
			ResolvedTransferEndpoint::External => {
				let device = self.device_mut(planned.device)?;
				ensure(
					device.staging.len() >= bytes && device.egress.contains_key(&work.task),
					Error::Protocol {
						task: work.task,
						detail: "HSA egress staging was not pre-realized at finalized size",
					},
				)?;
				ensure_queue(device, work.task, planned.slots.queue)?;
				claim_completion(&mut device.completions, work.task, planned.slots.completion)?;
				let submission = device.session.copy_async_prepared(
					&device.staging,
					0,
					&source_arena.allocation,
					source_offset,
					bytes,
					native,
				);
				finish_submission_claim(
					&mut device.completions,
					work.task,
					planned.slots.completion,
					submission,
				)?;
				Ok(PendingAction::Egress { bytes })
			}
			ResolvedTransferEndpoint::Device(destination) => {
				let destination_arena = checked_arena(arenas, destination, work.bytes.get())?;
				let destination_offset = offset_to_usize(destination.arena_offset.get())?;
				let device = self.device_mut(planned.device)?;
				ensure_queue(device, work.task, planned.slots.queue)?;
				claim_completion(&mut device.completions, work.task, planned.slots.completion)?;
				let submission = device.session.copy_async_prepared(
					&destination_arena.allocation,
					destination_offset,
					&source_arena.allocation,
					source_offset,
					bytes,
					native,
				);
				finish_submission_claim(
					&mut device.completions,
					work.task,
					planned.slots.completion,
					submission,
				)?;
				Ok(PendingAction::None)
			}
		}
	}

	fn submit_calculation(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		planned: crate::PlannedSubmission,
		work: &CalculationWork,
		native: &mut PreparedPending<'scope, 'scope>,
	) -> Result<PendingAction> {
		ensure(
			work.device == planned.device && work.submission == planned.slots,
			Error::Protocol {
				task: work.task,
				detail: "HSA calculation device or slots differ from immutable submission",
			},
		)?;
		let geometry = {
			let device = self.device_mut(planned.device)?;
			let artifact = device
				.artifacts
				.get(&work.artifact)
				.ok_or(Error::MissingArtifact {
					artifact: work.artifact,
				})?;
			let slot = device
				.kernargs
				.get_mut(&planned.slots.completion)
				.ok_or(Error::MissingCompletion {
					task: work.task,
					completion: planned.slots.completion,
				})?;
			fill_kernarg(slot, arenas, work, &artifact.abi)?;
			let queue = device
				.queues
				.get(&planned.slots.queue)
				.ok_or(Error::MissingQueue {
					task: work.task,
					queue: planned.slots.queue,
				})?;
			let progress = queue.progress_capacity(1, NonZeroU32::MIN)?;
			ensure(
				matches!(progress, QueueProgress::Ready { .. }),
				Error::ResourceContention {
					task: work.task,
					detail: "HSA AQL queue is backpressured",
				},
			)?;
			let grid = u32_from_u64(artifact.abi.elements.get(), "HSA grid dimension")?;
			let lanes = u16_from_u32(artifact.abi.workgroup_lanes, "HSA workgroup dimension")?;
			let geometry = DispatchGeometry::one_dimensional(grid, lanes);
			ensure(
				geometry.grid[0] == grid && geometry.workgroup[0] == lanes,
				Error::Protocol {
					task: work.task,
					detail: "HSA dispatch geometry changed during preflight",
				},
			)?;
			geometry
		};
		let device = self.device_mut(planned.device)?;
		let artifact = device
			.artifacts
			.get(&work.artifact)
			.ok_or(Error::MissingArtifact {
				artifact: work.artifact,
			})?;
		let slot = device
			.kernargs
			.get(&planned.slots.completion)
			.ok_or(Error::MissingCompletion {
				task: work.task,
				completion: planned.slots.completion,
			})?;
		let queue = device
			.queues
			.get(&planned.slots.queue)
			.ok_or(Error::MissingQueue {
				task: work.task,
				queue: planned.slots.queue,
			})?;
		claim_completion(&mut device.completions, work.task, planned.slots.completion)?;
		let submission = queue.dispatch_prepared(&artifact.kernel, Some(&slot.allocation), geometry, native);
		finish_submission_claim(
			&mut device.completions,
			work.task,
			planned.slots.completion,
			submission,
		)?;
		Ok(PendingAction::None)
	}

	fn submit_metric(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		planned: crate::PlannedSubmission,
		work: MetricWork,
		native: &mut PreparedPending<'scope, 'scope>,
	) -> Result<PendingAction> {
		ensure(
			work.value.device == planned.device && work.value.bytes.get() == 4,
			Error::ValueMismatch {
				value: work.value.value,
				detail: "HSA metric requires one four-byte value on the submission device",
			},
		)?;
		let arena = checked_arena(arenas, work.value, 4)?;
		let device = self.device_mut(planned.device)?;
		ensure_queue(device, work.task, planned.slots.queue)?;
		let metric_buffer = device
			.metric_buffers
			.get(&work.task)
			.ok_or(Error::Protocol {
				task: work.task,
				detail: "HSA metric buffer was not pre-realized",
			})?;
		ensure(
			metric_buffer.len() == 4,
			Error::Protocol {
				task: work.task,
				detail: "HSA metric buffer is not four bytes",
			},
		)?;
		let source_offset = offset_to_usize(work.value.arena_offset.get())?;
		claim_completion(&mut device.completions, work.task, planned.slots.completion)?;
		let submission = device.session.copy_async_prepared(
			metric_buffer,
			0,
			&arena.allocation,
			source_offset,
			4,
			native,
		);
		finish_submission_claim(
			&mut device.completions,
			work.task,
			planned.slots.completion,
			submission,
		)?;
		Ok(PendingAction::Metric {
			dtype: work.value.dtype,
		})
	}

	fn finish_pending(&mut self, pending: &mut HsaPending<'scope>) -> Result<BackendPoll> {
		let action_result = {
			let device = self.device_mut(pending.device)?;
			release_completion(&mut device.completions, pending.task, pending.completion)?;
			pending.state = HsaPendingState::Terminal;
			finish_action(device, pending.task, pending.action)
		};
		match action_result {
			Ok(metric) => Ok(BackendPoll::Complete { metric }),
			Err(error) => self.poison(error),
		}
	}

	pub(crate) fn recycle_pending(&mut self, mut pending: HsaPending<'scope>) -> Result<()> {
		self.ensure_healthy()?;
		ensure(
			pending.state == HsaPendingState::Terminal && self.prepared_tasks.remove(&pending.task),
			Error::Protocol {
				task: pending.task,
				detail: "only one terminal HSA pending token may be recycled",
			},
		)?;
		pending.native.reset()?;
		let task = pending.task;
		ensure(
			self.pending_pool.insert(task, pending.native).is_none(),
			Error::Protocol {
				task,
				detail: "HSA pending pool already contains the recycled task",
			},
		)
	}

	pub(crate) fn available_bytes(&self, device: DeviceId) -> Result<ByteCount> {
		let resources = self
			.devices
			.get(&device)
			.ok_or(Error::MissingDevice { device })?;
		Ok(ByteCount::new(resources.session.available_memory_bytes()?))
	}

	pub(crate) fn validate_handoff(
		&mut self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
	) -> Result<()> {
		self.ensure_healthy()?;
		ensure(
			self.prepared_tasks.is_empty() && self.pending_pool.keys().copied().eq(tasks.iter().copied()),
			Error::BackendState {
				backend: "HSA",
				detail: "warm HSA pending tokens were not all recycled",
			},
		)?;
		for (device, resources) in &self.devices {
			let finalized = bundle.init_image(*device).map(InitImageContract::from);
			ensure(
				resources.admission == finalized,
				Error::ArenaMismatch {
					device: *device,
					detail: "finalized HSA init-image manifest differs from warm admission",
				},
			)?;
		}
		let runtime_artifacts = self.plan.runtime_artifacts().cloned().collect();
		let devices = self.devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts, devices, tasks)?;
		let contracts = task_contracts(bundle, Some(tasks))?;
		self.plan = plan;
		self.contracts = contracts;
		Ok(())
	}

	fn device_mut(&mut self, device: DeviceId) -> Result<&mut DeviceResources<'scope>> {
		self.devices
			.get_mut(&device)
			.ok_or(Error::MissingDevice { device })
	}

	pub(crate) fn ensure_healthy(&self) -> Result<()> {
		match self.poisoned {
			true => Err(Error::BackendPoisoned { backend: "HSA" }),
			false => Ok(()),
		}
	}

	fn poison<T>(&mut self, error: Error) -> Result<T> {
		self.poisoned = true;
		Err(error)
	}
}

impl<'scope> HsaPreparedResources<'scope> {
	pub(crate) fn realize(
		draft: &DraftPlan,
		runtime_artifacts: Vec<crate::RuntimeArtifact>,
		reservations: &ReservationLedger,
		bindings: Vec<HsaBinding<'scope>>,
		tasks: &BTreeSet<TaskId>,
	) -> Result<Self> {
		let mut binding_by_device = BTreeMap::new();
		for binding in bindings {
			let device = binding.device;
			ensure(
				binding_by_device.insert(device, binding).is_none(),
				Error::DuplicateDevice { device },
			)?;
		}
		let scoped_tasks = draft
			.tasks
			.iter()
			.filter(|task| tasks.contains(&task.id))
			.cloned()
			.collect::<Vec<_>>();
		for task in &scoped_tasks {
			let TaskKind::Calculation(calculation) = &task.kind else {
				continue;
			};
			ensure(
				binding_by_device.contains_key(&calculation.device),
				Error::MissingDevice {
					device: calculation.device,
				},
			)?;
		}
		let artifact_ids = scoped_tasks
			.iter()
			.filter_map(|task| match &task.kind {
				TaskKind::Calculation(calculation) => Some(calculation.artifact),
				TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
			})
			.collect::<BTreeSet<_>>();
		let mut runtime_by_id = BTreeMap::new();
		for runtime in runtime_artifacts {
			let artifact = runtime.id;
			ensure(
				artifact_ids.contains(&artifact),
				Error::UnexpectedArtifact { artifact },
			)?;
			ensure(
				runtime_by_id.insert(artifact, runtime).is_none(),
				Error::DuplicateArtifact { artifact },
			)?;
		}
		for artifact in &artifact_ids {
			ensure(
				runtime_by_id.contains_key(artifact),
				Error::MissingArtifact {
					artifact: *artifact,
				},
			)?;
		}
		let value_devices = draft
			.values
			.iter()
			.map(|value| (value.id, value.device))
			.collect::<BTreeMap<_, _>>();
		let mut devices = BTreeMap::new();
		for (device, binding) in &binding_by_device {
			validate_binding(binding)?;
			devices.insert(
				*device,
				realize_device(
					&draft.resources,
					reservations,
					&scoped_tasks,
					&value_devices,
					&draft.init_images,
					&runtime_by_id,
					binding,
				)?,
			);
		}
		let pending = prepare_pending_pool(&devices, &scoped_tasks, &value_devices)?;
		Ok(Self {
			handoff: HsaPreparedHandoff::Candidate(runtime_by_id.into_values().collect()),
			devices,
			pending,
		})
	}

	pub(crate) fn bind(self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<HsaResources<'scope>> {
		let Self {
			handoff,
			devices,
			pending,
		} = self;
		let (plan, contracts) = match handoff {
			HsaPreparedHandoff::Finalized {
				bundle: prepared_bundle,
				tasks: prepared_tasks,
				plan,
				contracts,
			} => match prepared_bundle == bundle.identity() && prepared_tasks == *tasks {
				true => Ok((plan, contracts)),
				false => Err(Error::BackendState {
					backend: "HSA",
					detail: "finalized partition differs from its prepared handoff",
				}),
			},
			HsaPreparedHandoff::Candidate(_) => Err(Error::BackendState {
				backend: "HSA",
				detail: "candidate resources were not validated for finalized handoff",
			}),
		}?;
		Ok(HsaResources {
			plan,
			devices,
			contracts,
			prepared_tasks: BTreeSet::new(),
			pending_pool: pending,
			poisoned: false,
		})
	}

	pub(crate) fn bind_candidate(
		self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
	) -> Result<HsaResources<'scope>> {
		let Self {
			handoff,
			devices,
			pending,
		} = self;
		let runtime_artifacts = match handoff {
			HsaPreparedHandoff::Candidate(runtime_artifacts) => runtime_artifacts,
			HsaPreparedHandoff::Finalized { .. } => {
				return Err(Error::BackendState {
					backend: "HSA",
					detail: "finalized HSA resources cannot be rebound as a warm candidate",
				});
			}
		};
		ensure(
			pending.keys().copied().eq(tasks.iter().copied()),
			Error::BackendState {
				backend: "HSA",
				detail: "warm HSA partition differs from its pre-final pending pool",
			},
		)?;
		for (device, resources) in &devices {
			let warm = bundle.init_image(*device).map(InitImageContract::from);
			ensure(
				resources.admission == warm,
				Error::ArenaMismatch {
					device: *device,
					detail: "warm HSA init-image manifest differs from prepared admission",
				},
			)?;
		}
		let planned_devices = devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts, planned_devices, tasks)?;
		let contracts = task_contracts(bundle, Some(tasks))?;
		Ok(HsaResources {
			plan,
			devices,
			contracts,
			prepared_tasks: BTreeSet::new(),
			pending_pool: pending,
			poisoned: false,
		})
	}

	pub(crate) fn validate_handoff(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<()> {
		let runtime_artifacts = match &self.handoff {
			HsaPreparedHandoff::Candidate(runtime_artifacts) => runtime_artifacts,
			HsaPreparedHandoff::Finalized { .. } => {
				return Err(Error::BackendState {
					backend: "HSA",
					detail: "candidate handoff was validated more than once",
				});
			}
		};
		for (device, resources) in &self.devices {
			let finalized = bundle.init_image(*device).map(InitImageContract::from);
			ensure(
				resources.admission == finalized,
				Error::ArenaMismatch {
					device: *device,
					detail: "finalized HSA init-image manifest differs from its prepared admission",
				},
			)?;
		}
		let devices = self.devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts.clone(), devices, tasks)?;
		let contracts = task_contracts(bundle, Some(tasks))?;
		self.handoff = HsaPreparedHandoff::Finalized {
			bundle: bundle.identity(),
			tasks: tasks.clone(),
			plan,
			contracts,
		};
		Ok(())
	}

	pub(crate) fn destroy(self) -> Result<()> {
		let Self {
			devices, pending, ..
		} = self;
		drop(pending);
		destroy_devices(devices)
	}
}

impl<'scope> Backend for HsaBackend<'scope> {
	type Arena = HsaArena<'scope>;
	type Error = Error;
	type Pending = HsaPending<'scope>;
	type Resource = HsaResources<'scope>;

	fn bind_resources(
		&mut self,
		bundle: &FinalizedBundle,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Resource, Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::BindResources)?;
		let prior = core::mem::replace(&mut self.state, HsaBackendState::Bound);
		match prior {
			HsaBackendState::Ready {
				bindings,
				artifacts,
			} => {
				let plan = ExecutionPlan::validate(bundle, artifacts)?;
				HsaResources::realize(bundle, plan, bindings, None)
			}
			HsaBackendState::Prepared(resources) => {
				let tasks = bundle.tasks().iter().map(|task| task.id).collect();
				resources.bind(bundle, &tasks)
			}
			HsaBackendState::Warmed(mut resources) => {
				let tasks = bundle.tasks().iter().map(|task| task.id).collect();
				resources.validate_handoff(bundle, &tasks)?;
				Ok(resources)
			}
			HsaBackendState::Bound => Err(Error::BackendState {
				backend: "HSA",
				detail: "resources may be bound only once",
			}),
		}
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Pending, Self::Error> {
		crate::accounting::record(
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
	) -> std::result::Result<Self::Arena, Self::Error> {
		crate::accounting::record(
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
	) -> std::result::Result<(), Self::Error> {
		let call = crate::accounting::submission_call(&work);
		crate::accounting::record(physical_calls, call)?;
		resource.submit(&arenas, pending, work)
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<BackendPoll, Self::Error> {
		let task = pending.task();
		let result = resource.poll_pending(pending);
		let physical_status = match &result {
			Ok(BackendPoll::Pending) => PhysicalPollStatus::Pending,
			Ok(BackendPoll::Complete { .. }) => PhysicalPollStatus::Complete,
			Err(error) => match error {
				Error::DuplicateArtifact { .. }
				| Error::MissingArtifact { .. }
				| Error::UnexpectedArtifact { .. }
				| Error::ArtifactMismatch { .. }
				| Error::DuplicateDevice { .. }
				| Error::MissingDevice { .. }
				| Error::UnexpectedDevice { .. }
				| Error::MissingQueue { .. }
				| Error::MissingCompletion { .. }
				| Error::NoMetricSubmission { .. }
				| Error::ResourceContention { .. }
				| Error::CompletionBusy { .. }
				| Error::ArenaMismatch { .. }
				| Error::ValueMismatch { .. }
				| Error::UnsupportedTransfer { .. }
				| Error::UnsupportedLoopContract { .. }
				| Error::BackendState { .. }
				| Error::BackendPoisoned { .. }
				| Error::IntegerOverflow { .. }
				| Error::PhysicalAccountingOverflow
				| Error::CudaContract(_)
				| Error::Cuda(_)
				| Error::Hsa(_)
				| Error::Kernel(_)
				| Error::Protocol { .. } => PhysicalPollStatus::Failed,
			},
		};
		crate::accounting::record(
			physical_calls,
			crate::accounting::completion_poll_call(task, physical_status),
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
	) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(
			physical_calls,
			PhysicalCall::CollectExit {
				task: work.task,
				bytes: work.bytes,
			},
		)?;
		resource.collect_exit(&arenas, pending, work, destination)
	}

	fn release_arena(
		&mut self,
		resource: &mut Self::Resource,
		device: DeviceId,
		arena: Self::Arena,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::ReleaseArena { device })?;
		match resource.ensure_healthy() {
			Ok(()) => match arena.device() == device {
				true => arena.release(),
				false => Err(Error::ArenaMismatch {
					device,
					detail: "released HSA arena belongs to another device",
				}),
			},
			Err(error) => Err(error),
		}
	}

	fn destroy_resources(
		&mut self,
		resource: Self::Resource,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::DestroyResources)?;
		resource.destroy()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAction {
	None,
	Metric { dtype: recipe_core::DType },
	Egress { bytes: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HsaPendingState {
	Ready,
	Active,
	Terminal,
}

pub struct HsaPending<'scope> {
	task: TaskId,
	device: DeviceId,
	queue: QueueSlotId,
	completion: CompletionSlotId,
	class: WorkClass,
	native: PreparedPending<'scope, 'scope>,
	action: PendingAction,
	state: HsaPendingState,
}

impl<'scope> HsaPending<'scope> {
	fn ready(
		request: PendingRequest,
		planned: crate::PlannedSubmission,
		native: PreparedPending<'scope, 'scope>,
	) -> Self {
		Self {
			task: request.task,
			device: planned.device,
			queue: planned.slots.queue,
			completion: planned.slots.completion,
			class: request.class,
			native,
			action: PendingAction::None,
			state: HsaPendingState::Ready,
		}
	}

	fn validate_ready(&self, work: &BackendWork<'_>, planned: crate::PlannedSubmission) -> Result<()> {
		ensure(
			self.state == HsaPendingState::Ready
				&& self.task == work.task()
				&& self.class == work.class()
				&& self.device == planned.device
				&& self.queue == planned.slots.queue
				&& self.completion == planned.slots.completion,
			Error::Protocol {
				task: work.task(),
				detail: "submitted HSA work differs from its prepared pending token",
			},
		)
	}

	fn activate(&mut self, action: PendingAction) {
		self.action = action;
		self.state = HsaPendingState::Active;
	}

	fn validate_collected_exit(&self, work: &TransferWork<'_>, destination_bytes: usize) -> Result<()> {
		let bytes = bytes_to_usize(work.bytes.get(), "HSA exit result size")?;
		ensure(
			self.task == work.task
				&& self.class == WorkClass::ExitTransfer
				&& self.state == HsaPendingState::Terminal
				&& self.queue == work.submission.queue
				&& self.completion == work.submission.completion
				&& self.action == PendingAction::Egress { bytes }
				&& destination_bytes == bytes,
			Error::Protocol {
				task: work.task,
				detail: "HSA exit collection differs from its completed prepared task",
			},
		)
	}

	pub(crate) fn task(&self) -> TaskId {
		self.task
	}
}

impl fmt::Debug for HsaPending<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaPending")
			.field("task", &self.task)
			.field("device", &self.device)
			.field("queue", &self.queue)
			.field("completion", &self.completion)
			.field("class", &self.class)
			.field("action", &self.action)
			.field("state", &self.state)
			.finish_non_exhaustive()
	}
}

fn validate_binding(binding: &HsaBinding<'_>) -> Result<()> {
	let has_target = binding
		.session
		.description()
		.isas
		.iter()
		.filter_map(|isa| isa.amd_target.as_ref())
		.any(|target| target.as_str() == binding.target_id);
	ensure(
		has_target,
		Error::ArtifactMismatch {
			artifact: ArtifactId::new(0),
			detail: format!(
				"HSA session does not advertise exact target {:?}",
				binding.target_id
			),
		},
	)
}

fn prepare_pending_pool<'scope>(
	devices: &BTreeMap<DeviceId, DeviceResources<'scope>>,
	tasks: &[Task],
	value_devices: &BTreeMap<ValueId, DeviceId>,
) -> Result<BTreeMap<TaskId, PreparedPending<'scope, 'scope>>> {
	let mut pending = BTreeMap::new();
	for task in tasks {
		let device = candidate_task_device(task, value_devices)?;
		let resources = devices
			.get(&device)
			.ok_or(Error::MissingDevice { device })?;
		let token = resources.session.prepare_pending(2, 0)?;
		ensure(
			pending.insert(task.id, token).is_none(),
			Error::Protocol {
				task: task.id,
				detail: "HSA candidate task appears more than once",
			},
		)?;
	}
	Ok(pending)
}

fn candidate_task_device(task: &Task, value_devices: &BTreeMap<ValueId, DeviceId>) -> Result<DeviceId> {
	match &task.kind {
		TaskKind::Calculation(calculation) => Ok(calculation.device),
		TaskKind::Metric(metric) => value_devices
			.get(&metric.value)
			.copied()
			.ok_or(Error::ValueMismatch {
				value: metric.value,
				detail: "HSA candidate metric value has no device",
			}),
		TaskKind::Transfer(transfer) => match transfer.source {
			TransferEndpoint::Device { device, .. } => Ok(device),
			TransferEndpoint::External => match transfer.destination {
				TransferEndpoint::Device { device, .. } => Ok(device),
				TransferEndpoint::External => Err(Error::Protocol {
					task: task.id,
					detail: "HSA candidate transfer has no device endpoint",
				}),
			},
		},
	}
}

fn realize_device<'scope>(
	resources: &ResourceManifest,
	reservations: &ReservationLedger,
	tasks: &[Task],
	value_devices: &BTreeMap<ValueId, DeviceId>,
	init_images: &[InitDataImage],
	runtime_artifacts: &BTreeMap<ArtifactId, crate::RuntimeArtifact>,
	binding: &HsaBinding<'scope>,
) -> Result<DeviceResources<'scope>> {
	let device = binding.device;
	let queue_ids = tasks
		.iter()
		.filter_map(task_submission)
		.map(|submission| submission.queue)
		.collect::<BTreeSet<_>>();
	let completion_ids = tasks
		.iter()
		.filter_map(task_submission)
		.map(|submission| submission.completion)
		.collect::<BTreeSet<_>>();
	let queues = resources
		.queues
		.iter()
		.filter(|slot| slot.device == device && queue_ids.contains(&slot.id))
		.map(|slot| {
			Ok((
				slot.id,
				binding.session.create_queue(QueueConfig::new(
					binding.queue_packets,
					QueueKind::SingleProducer,
				))?,
			))
		})
		.collect::<Result<BTreeMap<_, _>>>()?;
	let completions = resources
		.completions
		.iter()
		.filter(|slot| slot.device == device && completion_ids.contains(&slot.id))
		.map(|slot| (slot.id, CompletionState::Available))
		.collect();
	let staging_bytes = resources
		.pinned_staging
		.iter()
		.find(|entry| entry.device == device)
		.ok_or(Error::MissingDevice { device })?
		.bytes
		.get();
	let staging = binding
		.session
		.allocate_fine(bytes_to_usize(staging_bytes, "HSA fine staging size")?)?;
	binding.session.grant_access(&staging)?;
	let admission = init_images
		.iter()
		.find(|manifest| manifest.device == device)
		.map(InitImageContract::from);
	match admission {
		Some(admission) => ensure(
			admission.bytes.get() <= staging_bytes,
			Error::ArenaMismatch {
				device,
				detail: "HSA init image exceeds its pre-realized fine staging",
			},
		),
		None => Err(Error::MissingDevice { device }),
	}?;
	let scratch = match resources
		.scratch
		.iter()
		.find(|entry| entry.device == device)
		.map(|entry| entry.bytes.get())
	{
		Some(0) | None => None,
		Some(bytes) => Some(binding
			.session
			.allocate_coarse(bytes_to_usize(bytes, "HSA scratch size")?)?),
	};
	let reservation = reservations
		.entry(device)
		.ok_or(Error::MissingDevice { device })?;
	ensure(
		reservation.mechanism == ReservationMechanism::HeldAllocation,
		Error::ArenaMismatch {
			device,
			detail: "HSA bridge requires the finalized held-allocation reservation mechanism",
		},
	)?;
	let reservation = binding.session.allocate_coarse(bytes_to_usize(
		reservation.bytes.get(),
		"HSA user reservation",
	)?)?;

	let artifact_ids = tasks
		.iter()
		.filter_map(|task| match &task.kind {
			TaskKind::Calculation(calculation) => (calculation.device == device).then_some(calculation.artifact),
			TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
		})
		.collect::<BTreeSet<_>>();
	let mut artifacts = BTreeMap::new();
	for artifact_id in artifact_ids {
		let runtime = runtime_artifacts
			.get(&artifact_id)
			.ok_or(Error::MissingArtifact {
				artifact: artifact_id,
			})?;
		let RuntimeArtifactKind::Hsa {
			target_id,
			code_object_version,
		} = &runtime.kind
		else {
			return Err(Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "HSA device was assigned a non-HSA artifact".to_owned(),
			});
		};
		ensure(
			target_id == &binding.target_id && *code_object_version == binding.code_object_version,
			Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "HSACO target or code-object version differs from the HSA binding".to_owned(),
			},
		)?;
		let inspection = inspect_hsaco(
			&runtime.bytes,
			target_id,
			*code_object_version,
			&runtime.abi,
		)?;
		ensure(
			inspection.kernel.name == runtime.abi.entry_symbol,
			Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "inspected HSACO entry differs from immutable ABI".to_owned(),
			},
		)?;
		let executable = binding.session.load_hsaco(&runtime.bytes)?;
		let kernel = executable.kernel(&runtime.abi.entry_symbol)?;
		ensure(
			kernel.metadata().kernarg_segment_size >= runtime.abi.argument_bytes
				&& kernel.metadata().kernarg_segment_alignment >= runtime.abi.argument_alignment,
			Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "loaded HSA kernel metadata is smaller than the inspected ABI".to_owned(),
			},
		)?;
		artifacts.insert(
			artifact_id,
			LoadedArtifact {
				kernel,
				executable,
				abi: runtime.abi.clone(),
			},
		);
	}

	let kernarg_sizes = kernarg_sizes(tasks, device, &artifacts)?;
	let mut kernargs = BTreeMap::new();
	for (slot, bytes) in kernarg_sizes {
		let allocation = binding.session.allocate_kernarg(bytes)?;
		binding.session.grant_access(&allocation)?;
		kernargs.insert(
			slot,
			KernargSlot {
				allocation,
				bytes: vec![0_u8; bytes],
			},
		);
	}
	let metric_buffers = tasks
		.iter()
		.filter_map(|task| match &task.kind {
			TaskKind::Metric(metric) => match value_devices.get(&metric.value) {
				Some(value_device) => (*value_device == device).then_some(task.id),
				None => None,
			},
			TaskKind::Calculation(_) | TaskKind::Transfer(_) => None,
		})
		.map(|task| Ok((task, binding.session.allocate_fine(4)?)))
		.collect::<Result<BTreeMap<_, _>>>()?;
	for allocation in metric_buffers.values() {
		binding.session.grant_access(allocation)?;
	}
	let egress = tasks
		.iter()
		.filter_map(|task| match &task.kind {
			TaskKind::Transfer(transfer) => {
				let source = match transfer.source {
					recipe_core::TransferEndpoint::Device { device, .. } => Some(device),
					recipe_core::TransferEndpoint::External => None,
				};
				(task.phase == RunPhase::Exit
					&& source == Some(device) && matches!(
					transfer.destination,
					recipe_core::TransferEndpoint::External
				))
				.then_some((task.id, transfer.bytes.get()))
			}
			TaskKind::Calculation(_) | TaskKind::Metric(_) => None,
		})
		.map(|(task, bytes)| Ok((task, vec![0_u8; bytes_to_usize(bytes, "HSA egress size")?])))
		.collect::<Result<BTreeMap<_, _>>>()?;

	Ok(DeviceResources {
		session: binding.session,
		queues,
		completions,
		artifacts,
		kernargs,
		metric_buffers,
		staging,
		admission,
		egress,
		scratch,
		reservation,
	})
}

fn task_contracts(
	bundle: &FinalizedBundle,
	selected: Option<&BTreeSet<TaskId>>,
) -> Result<BTreeMap<TaskId, HsaTaskContract>> {
	let mut contracts = BTreeMap::new();
	for task in bundle.tasks().iter().filter(|task| match selected {
		Some(selected) => selected.contains(&task.id),
		None => true,
	}) {
		let (class, submission, admission, route, lane_claims) = match &task.kind {
			TaskKind::Calculation(calculation) => {
				ensure(
					task.phase == RunPhase::Loop,
					Error::Protocol {
						task: task.id,
						detail: "HSA calculation is not assigned to the loop phase",
					},
				)?;
				(
					WorkClass::Calculation,
					Some(calculation.submission),
					None,
					Vec::new(),
					Vec::new(),
				)
			}
			TaskKind::Metric(_) => {
				ensure(
					task.phase == RunPhase::Loop,
					Error::Protocol {
						task: task.id,
						detail: "HSA metric is not assigned to the loop phase",
					},
				)?;
				(WorkClass::Metric, None, None, Vec::new(), Vec::new())
			}
			TaskKind::Transfer(transfer) => {
				let class = transfer_work_class(task.id, task.phase, transfer.source, transfer.destination)?;
				let admission = match class {
					WorkClass::InitAdmission => {
						let endpoints = bundle.transfer_endpoints(task.id).ok_or(Error::Protocol {
							task: task.id,
							detail: "HSA admission has no finalized endpoints",
						})?;
						let ResolvedTransferEndpoint::Device(destination) = endpoints.destination else {
							return Err(Error::Protocol {
								task: task.id,
								detail: "HSA admission has no finalized device image",
							});
						};
						let manifest = bundle
							.init_image(destination.device)
							.ok_or(Error::Protocol {
								task: task.id,
								detail: "HSA admission device has no finalized init-image manifest",
							})?;
						let contract = InitImageContract::from(manifest);
						ensure(
							contract.image == destination.value && contract.bytes == transfer.bytes,
							Error::Protocol {
								task: task.id,
								detail: "HSA admission differs from the finalized init-image manifest",
							},
						)?;
						Some(contract)
					}
					WorkClass::InternalTransfer | WorkClass::ExitTransfer => None,
					WorkClass::Calculation | WorkClass::Metric => {
						return Err(Error::Protocol {
							task: task.id,
							detail: "HSA transfer has a non-transfer work class",
						});
					}
				};
				(
					class,
					Some(transfer.submission),
					admission,
					transfer.route.clone(),
					transfer.lane_claims.clone(),
				)
			}
		};
		let contract = HsaTaskContract {
			phase: task.phase,
			class,
			submission,
			admission,
			route,
			lane_claims,
		};
		ensure(
			contracts.insert(task.id, contract).is_none(),
			Error::Protocol {
				task: task.id,
				detail: "HSA task contract appears more than once",
			},
		)?;
	}
	Ok(contracts)
}

fn transfer_work_class(
	task: TaskId,
	phase: RunPhase,
	source: recipe_core::TransferEndpoint,
	destination: recipe_core::TransferEndpoint,
) -> Result<WorkClass> {
	use recipe_core::TransferEndpoint::{Device, External};

	match (phase, source, destination) {
		(RunPhase::Init, External, Device { .. }) => Ok(WorkClass::InitAdmission),
		(RunPhase::Init | RunPhase::Loop, Device { .. }, Device { .. }) => Ok(WorkClass::InternalTransfer),
		(RunPhase::Exit, Device { .. }, Device { .. } | External) => Ok(WorkClass::ExitTransfer),
		(RunPhase::Init, External, External)
		| (RunPhase::Init, Device { .. }, External)
		| (RunPhase::Loop, External, External | Device { .. })
		| (RunPhase::Loop, Device { .. }, External)
		| (RunPhase::Exit, External, External | Device { .. }) => Err(Error::Protocol {
			task,
			detail: "HSA transfer phase or endpoint class is invalid",
		}),
	}
}

fn work_submission(work: &BackendWork<'_>) -> Option<SubmissionSlots> {
	match work {
		BackendWork::InitAdmission(work) => Some(work.submission),
		BackendWork::Calculation(work) => Some(work.submission),
		BackendWork::InternalTransfer(work) | BackendWork::ExitTransfer(work) => Some(work.submission),
		BackendWork::Metric(_) => None,
	}
}

fn task_submission(task: &Task) -> Option<SubmissionSlots> {
	match &task.kind {
		TaskKind::Calculation(calculation) => Some(calculation.submission),
		TaskKind::Transfer(transfer) => Some(transfer.submission),
		TaskKind::Metric(_) => None,
	}
}

fn kernarg_sizes(
	tasks: &[Task],
	device: DeviceId,
	artifacts: &BTreeMap<ArtifactId, LoadedArtifact<'_>>,
) -> Result<BTreeMap<CompletionSlotId, usize>> {
	let mut result = BTreeMap::new();
	for task in tasks {
		let TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		match calculation.device == device {
			true => {
				let artifact = artifacts
					.get(&calculation.artifact)
					.ok_or(Error::MissingArtifact {
						artifact: calculation.artifact,
					})?;
				let bytes = usize::try_from(artifact.kernel.metadata().kernarg_segment_size)
					.map_err(kernarg_size_error)?;
				result.entry(calculation.submission.completion)
					.and_modify(|prior: &mut usize| *prior = (*prior).max(bytes))
					.or_insert(bytes);
			}
			false => continue,
		}
	}
	Ok(result)
}

fn fill_kernarg<'scope>(
	slot: &mut KernargSlot<'_>,
	arenas: &impl HsaArenaLookup<'scope>,
	work: &CalculationWork,
	abi: &recipe_kernel::KernelAbi,
) -> Result<()> {
	ensure(
		slot.bytes.len() >= usize::try_from(abi.argument_bytes).map_err(kernarg_size_error)?,
		Error::ResourceContention {
			task: work.task,
			detail: "preallocated HSA kernarg slot is too small",
		},
	)?;
	slot.bytes.fill(0);
	let mut locations = work.inputs.iter().chain(work.outputs.iter()).copied();
	let mut has_fault_argument = false;
	for (argument_index, argument) in abi.arguments.iter().enumerate() {
		let value = match argument {
			KernelArgument::Buffer { .. } => {
				let location = locations.next().ok_or(Error::Protocol {
					task: work.task,
					detail: "HSA calculation operands differ from the validated artifact ABI",
				})?;
				let arena = checked_arena(arenas, location, location.bytes.get())?;
				let base = u64::try_from(arena.allocation.as_ptr().addr()).map_err(pointer_size_error)?;
				base.checked_add(location.arena_offset.get())
					.ok_or(Error::IntegerOverflow {
						field: "HSA kernarg device pointer",
					})?
			}
			KernelArgument::RunId => work.run.get(),
			KernelArgument::ElementCount => abi.elements.get(),
			KernelArgument::FaultFlag => {
				has_fault_argument = true;
				let location = work.fault_flag.ok_or(Error::Protocol {
					task: work.task,
					detail: "HSA checked calculation has no resolved fault-flag location",
				})?;
				ensure(
					location.dtype == recipe_core::DType::I32 && location.bytes.get() == 4,
					Error::ValueMismatch {
						value: location.value,
						detail: "HSA fault flag must be one resolved int32 value",
					},
				)?;
				let arena = checked_arena(arenas, location, 4)?;
				let base = u64::try_from(arena.allocation.as_ptr().addr()).map_err(pointer_size_error)?;
				base.checked_add(location.arena_offset.get())
					.ok_or(Error::IntegerOverflow {
						field: "HSA fault-flag device pointer",
					})?
			}
		};
		let offset = argument_index
			.checked_mul(8)
			.ok_or(Error::IntegerOverflow {
				field: "HSA kernarg byte offset",
			})?;
		let end = offset.checked_add(8).ok_or(Error::IntegerOverflow {
			field: "HSA kernarg byte range",
		})?;
		let destination = slot
			.bytes
			.get_mut(offset..end)
			.ok_or(Error::IntegerOverflow {
				field: "HSA kernarg byte range",
			})?;
		destination.copy_from_slice(&value.to_le_bytes());
	}
	ensure(
		locations.next().is_none() && has_fault_argument == work.fault_flag.is_some(),
		Error::Protocol {
			task: work.task,
			detail: "HSA calculation operands or fault flag differ from the validated artifact ABI",
		},
	)?;
	let copy = unsafe {
		// SAFETY: kernarg allocations are host-accessible by HSA definition,
		// and strict submission has not published an AQL packet.
		slot.allocation.copy_from_host(0, &slot.bytes)
	};
	copy.map_err(Error::from)
}

fn checked_arena<'arena, 'scope>(
	arenas: &'arena impl HsaArenaLookup<'scope>,
	location: ResolvedValueLocation,
	bytes: u64,
) -> Result<&'arena HsaArena<'scope>> {
	let arena = arenas.arena(location.device).ok_or(Error::MissingDevice {
		device: location.device,
	})?;
	ensure(
		arena.device == location.device,
		Error::ArenaMismatch {
			device: location.device,
			detail: "HSA arena identity differs from resolved value",
		},
	)?;
	let end = location
		.arena_offset
		.get()
		.checked_add(bytes)
		.ok_or(Error::IntegerOverflow {
			field: "resolved HSA arena range",
		})?;
	let arena_bytes = u64::try_from(arena.allocation.len()).map_err(pointer_size_error)?;
	ensure(
		end <= arena_bytes,
		Error::ValueMismatch {
			value: location.value,
			detail: "resolved range exceeds HSA arena",
		},
	)?;
	Ok(arena)
}

fn device_endpoints(work: &TransferWork) -> Result<(ResolvedValueLocation, ResolvedValueLocation)> {
	match (work.source, work.destination) {
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => {
			Ok((source, destination))
		}
		(ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::External)
		| (ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::Device(_))
		| (ResolvedTransferEndpoint::Device(_), ResolvedTransferEndpoint::External) => Err(Error::UnsupportedTransfer {
			task: work.task,
			detail: "internal transfer does not have two resolved HSA endpoints",
		}),
	}
}

fn claim_completion(
	completions: &mut BTreeMap<CompletionSlotId, CompletionState>,
	task: TaskId,
	completion: CompletionSlotId,
) -> Result<()> {
	let slot = completions
		.get_mut(&completion)
		.ok_or(Error::MissingCompletion { task, completion })?;
	match core::mem::replace(slot, CompletionState::Active { task }) {
		CompletionState::Available => Ok(()),
		CompletionState::Active { task: owner } => Err(Error::CompletionBusy {
			backend: "HSA",
			task,
			completion,
			owner,
		}),
	}
}

fn release_completion(
	completions: &mut BTreeMap<CompletionSlotId, CompletionState>,
	task: TaskId,
	completion: CompletionSlotId,
) -> Result<()> {
	let slot = completions
		.get_mut(&completion)
		.ok_or(Error::MissingCompletion { task, completion })?;
	ensure(
		completion_owned(slot, task),
		Error::Protocol {
			task,
			detail: "HSA completion slot belongs to another task",
		},
	)?;
	*slot = CompletionState::Available;
	Ok(())
}

fn finish_submission_claim(
	completions: &mut BTreeMap<CompletionSlotId, CompletionState>,
	task: TaskId,
	completion: CompletionSlotId,
	submission: recipe_hsa::Result<()>,
) -> Result<()> {
	match submission {
		Ok(()) => Ok(()),
		Err(error) => {
			release_completion(completions, task, completion)?;
			Err(Error::from(error))
		}
	}
}

fn ensure_queue(device: &DeviceResources<'_>, task: TaskId, queue: QueueSlotId) -> Result<()> {
	ensure(
		device.queues.contains_key(&queue),
		Error::MissingQueue { task, queue },
	)
}

fn validate_active(device: &DeviceResources<'_>, pending: &HsaPending<'_>) -> Result<()> {
	let state = device
		.completions
		.get(&pending.completion)
		.ok_or(Error::MissingCompletion {
			task: pending.task,
			completion: pending.completion,
		})?;
	ensure(
		completion_owned(state, pending.task),
		Error::Protocol {
			task: pending.task,
			detail: "HSA pending token is not registered active",
		},
	)
}

fn finish_action(device: &mut DeviceResources<'_>, task: TaskId, action: PendingAction) -> Result<Option<MetricValue>> {
	match action {
		PendingAction::None => Ok(None),
		PendingAction::Metric { dtype } => {
			let buffer = device.metric_buffers.get(&task).ok_or(Error::Protocol {
				task,
				detail: "completed HSA metric has no preallocated result buffer",
			})?;
			ensure(
				buffer.len() == 4,
				Error::Protocol {
					task,
					detail: "completed HSA metric buffer is not four bytes",
				},
			)?;
			let mut bytes = [0_u8; 4];
			let copy = unsafe {
				// SAFETY: the prepared asynchronous copy reached terminal
				// system-scope completion before this host read.
				buffer.copy_to_host(0, &mut bytes)
			};
			copy?;
			match dtype {
				recipe_core::DType::F32 => Ok(Some(MetricValue::F32(f32::from_le_bytes(bytes)))),
				recipe_core::DType::I32 => Ok(Some(MetricValue::I32(i32::from_le_bytes(bytes)))),
			}
		}
		PendingAction::Egress { bytes } => {
			let output = device.egress.get_mut(&task).ok_or(Error::Protocol {
				task,
				detail: "completed HSA egress has no preallocated output",
			})?;
			ensure(
				output.len() == bytes && device.staging.len() >= bytes,
				Error::Protocol {
					task,
					detail: "completed HSA egress size differs from preallocated output",
				},
			)?;
			let copy = unsafe {
				// SAFETY: the async copy reached terminal system-scope
				// completion before this host read of fine-grained staging.
				device.staging.copy_to_host(0, output)
			};
			copy?;
			Ok(None)
		}
	}
}

fn completion_owned(state: &CompletionState, task: TaskId) -> bool {
	match state {
		CompletionState::Active { task: owner } => *owner == task,
		CompletionState::Available => false,
	}
}

fn destroy_devices(devices: BTreeMap<DeviceId, DeviceResources<'_>>) -> Result<()> {
	for resources in devices.values() {
		for state in resources.completions.values() {
			ensure(
				matches!(state, CompletionState::Available),
				Error::ResourceContention {
					task: TaskId::new(0),
					detail: "HSA resources still have active completion slots",
				},
			)?;
		}
		resources
			.session
			.drain_retirements(Duration::from_millis(10))?;
	}
	for (_, device) in devices {
		for (_, queue) in device.queues {
			queue.close()?;
		}
		for (_, artifact) in device.artifacts {
			artifact.close()?;
		}
		for (_, slot) in device.kernargs {
			slot.allocation.close()?;
		}
		for (_, allocation) in device.metric_buffers {
			allocation.close()?;
		}
		device.staging.close()?;
		free_optional_allocation(device.scratch)?;
		device.reservation.close()?;
	}
	Ok(())
}

fn free_optional_allocation(allocation: Option<Allocation<'_>>) -> Result<()> {
	match allocation {
		Some(allocation) => allocation.close().map_err(Error::from),
		None => Ok(()),
	}
}

fn bytes_to_usize(bytes: u64, field: &'static str) -> Result<usize> {
	match usize::try_from(bytes) {
		Ok(value) => Ok(value),
		Err(..) => Err(Error::IntegerOverflow { field }),
	}
}

fn offset_to_usize(offset: u64) -> Result<usize> {
	bytes_to_usize(offset, "HSA arena offset")
}

fn u32_from_u64(value: u64, field: &'static str) -> Result<u32> {
	match u32::try_from(value) {
		Ok(value) => Ok(value),
		Err(..) => Err(Error::IntegerOverflow { field }),
	}
}

fn u16_from_u32(value: u32, field: &'static str) -> Result<u16> {
	match u16::try_from(value) {
		Ok(value) => Ok(value),
		Err(..) => Err(Error::IntegerOverflow { field }),
	}
}

fn kernarg_size_error(error: core::num::TryFromIntError) -> Error {
	debug_assert!(std::error::Error::source(&error).is_none());
	Error::IntegerOverflow {
		field: "HSA kernarg size",
	}
}

fn pointer_size_error(error: core::num::TryFromIntError) -> Error {
	debug_assert!(std::error::Error::source(&error).is_none());
	Error::IntegerOverflow {
		field: "HSA pointer or arena size",
	}
}

fn ensure(valid: bool, error: Error) -> Result<()> {
	match valid {
		true => Ok(()),
		false => Err(error),
	}
}

fn reject_unexpected_device(device: Option<DeviceId>) -> Result<()> {
	match device {
		Some(device) => Err(Error::UnexpectedDevice { device }),
		None => Ok(()),
	}
}
