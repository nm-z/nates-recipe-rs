//! ROCr/HSA resource realization and submission bridge.

use core::fmt;
use std::{
	collections::{BTreeMap, BTreeSet},
	num::NonZeroU32,
	time::Duration,
};

use recipe_core::{
	ArenaLayout, ArtifactId, BundleIdentity, ByteCount, CompletionSlotId, DeviceId, DraftPlan, FinalizedBundle,
	InitDataImage, KernelResourceBounds, LinkId, LoopIteration, QueueSlotId, ReservationLedger, ReservationMechanism,
	ResolvedTransferEndpoint, ResolvedValueLocation, ResourceManifest, RunPhase, SubmissionSlots, Task, TaskId,
	TaskKind, TransferEndpoint, TransferLaneClaim, ValueId,
};
use recipe_executor::{
	ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork, MetricValue, MetricWork,
	PendingRequest, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus, TransferWork, WorkClass, sealed,
};
use recipe_hsa::{
	Allocation, DeviceType, DiscoveredAgent, DispatchGeometry, Executable, Kernel, MemoryPoolFlags, MemorySegment,
	PollStatus, PreparedPending, Queue, QueueConfig, QueueKind, QueueProgress, Session,
};
use recipe_kernel::{ArtifactDigest, KernelArgument, inspect_hsaco_bundle};

use crate::{
	Error, ExecutionPlan, NativeBackendKind, NativeDeviceExecutionEvidence, Result, RuntimeArtifactKind,
	plan::InitImageContract,
};

#[derive(Clone, Debug)]
pub struct HsaBindingProperties {
	target_id: String,
	code_object_version: u8,
	queue_packets: u32,
	maximum_submission_queues: u32,
	enabled_display_connectors: u32,
}

impl HsaBindingProperties {
	#[must_use]
	pub fn new(
		target_id: String,
		code_object_version: u8,
		queue_packets: u32,
		maximum_submission_queues: u32,
		enabled_display_connectors: u32,
	) -> Self {
		Self {
			target_id,
			code_object_version,
			queue_packets,
			maximum_submission_queues,
			enabled_display_connectors,
		}
	}
}

#[derive(Clone)]
pub struct HsaBinding<'scope> {
	device: DeviceId,
	session: &'scope Session<'scope>,
	host_allocator: &'scope DiscoveredAgent<'scope>,
	properties: HsaBindingProperties,
}

impl<'scope> HsaBinding<'scope> {
	#[must_use]
	pub fn new(
		device: DeviceId,
		session: &'scope Session<'scope>,
		host_allocator: &'scope DiscoveredAgent<'scope>,
		properties: HsaBindingProperties,
	) -> Self {
		Self {
			device,
			session,
			host_allocator,
			properties,
		}
	}

	#[must_use]
	pub const fn device(&self) -> DeviceId { self.device }

	#[must_use]
	pub fn target_id(&self) -> &str { &self.properties.target_id }

	#[must_use]
	pub const fn code_object_version(&self) -> u8 { self.properties.code_object_version }

	#[must_use]
	pub const fn queue_packets(&self) -> u32 { self.properties.queue_packets }

	#[must_use]
	pub const fn maximum_submission_queues(&self) -> u32 { self.properties.maximum_submission_queues }

	#[must_use]
	pub const fn enabled_display_connectors(&self) -> u32 { self.properties.enabled_display_connectors }

	/// Queries ROCr's current available-memory counter for this exact binding
	/// before any candidate resources are realized.
	pub fn available_bytes(&self) -> Result<ByteCount> { Ok(ByteCount::new(self.session.available_memory_bytes()?)) }

	/// Allocate host-accessible fine-grained memory through this GPU's exact
	/// CPU allocator and grant the bound GPU access.
	pub fn allocate_host_fine(&self, size: usize) -> recipe_hsa::Result<Allocation<'scope>> {
		let allocation = self.host_allocator.allocate_fine(size)?;
		self.session.grant_access(&allocation)?;
		Ok(allocation)
	}

	pub(crate) const fn session(&self) -> &'scope Session<'scope> { self.session }

	pub(crate) const fn host_allocator(&self) -> &'scope DiscoveredAgent<'scope> { self.host_allocator }
}

impl fmt::Debug for HsaBinding<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaBinding")
			.field("device", &self.device)
			.field(
				"host_allocator",
				&self.host_allocator.description().identity,
			)
			.field("target_id", &self.properties.target_id)
			.field("code_object_version", &self.properties.code_object_version)
			.field("queue_packets", &self.properties.queue_packets)
			.field(
				"maximum_submission_queues",
				&self.properties.maximum_submission_queues,
			)
			.field(
				"enabled_display_connectors",
				&self.properties.enabled_display_connectors,
			)
			.finish_non_exhaustive()
	}
}

pub(crate) trait HsaArenaLookup<'scope> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'scope>>;
}

impl<'scope> HsaArenaLookup<'scope> for BTreeMap<DeviceId, HsaArena<'scope>> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'scope>> { self.get(&device) }
}

impl<'borrow, 'scope> HsaArenaLookup<'scope> for ArenaSet<'borrow, HsaArena<'scope>> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'scope>> { self.get(device) }
}

pub struct HsaArena<'scope> {
	device: DeviceId,
	allocation: Allocation<'scope>,
}

impl<'scope> HsaArena<'scope> {
	pub(crate) fn device(&self) -> DeviceId { self.device }

	pub(crate) const fn allocation(&self) -> &Allocation<'scope> { &self.allocation }

	pub fn bytes(&self) -> usize { self.allocation.len() }

	pub(crate) fn release(self) -> Result<()> { self.allocation.close().map_err(Error::from) }
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
	abi: recipe_kernel::KernelAbi,
	resources: Option<HsaArtifactResourceEnvelope>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HsaArtifactResourceEnvelope {
	finalized: KernelResourceBounds,
	dynamic_private_bytes: u32,
}

impl fmt::Debug for LoadedArtifact<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("LoadedArtifact")
			.field("entry", &self.abi.entry_symbol)
			.field("metadata", self.kernel.metadata())
			.field("resources", &self.resources)
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
	host_allocator: &'scope DiscoveredAgent<'scope>,
	queues: BTreeMap<QueueSlotId, Queue<'scope, 'scope>>,
	completions: BTreeMap<CompletionSlotId, CompletionState>,
	artifacts: BTreeMap<ArtifactId, LoadedArtifact<'scope>>,
	executables: BTreeMap<ArtifactDigest, Executable<'scope, 'scope>>,
	kernargs: BTreeMap<CompletionSlotId, KernargSlot<'scope>>,
	metric_buffers: BTreeMap<TaskId, Allocation<'scope>>,
	staging: Allocation<'scope>,
	admission: Option<InitImageContract>,
	egress: BTreeMap<TaskId, Vec<u8>>,
	scratch: Option<Allocation<'scope>>,
}

impl fmt::Debug for DeviceResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("DeviceResources")
			.field("agent", &self.session.description().identity)
			.field(
				"host_allocator",
				&self.host_allocator.description().identity,
			)
			.field("queue_count", &self.queues.len())
			.field("completion_count", &self.completions.len())
			.field("artifact_count", &self.artifacts.len())
			.field("executable_count", &self.executables.len())
			.field("kernarg_count", &self.kernargs.len())
			.field("metric_buffer_count", &self.metric_buffers.len())
			.field("staging_bytes", &self.staging.len())
			.field("admission", &self.admission)
			.field("egress_count", &self.egress.len())
			.field("scratch_bytes", &self.scratch.as_ref().map(Allocation::len))
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
	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently hands off warmed resources"
	)]
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
	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently hands off warmed resources"
	)]
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

	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently hands off warmed resources"
	)]
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
			HsaBackendState::Bound => {
				Err(Error::BackendState {
					backend: "HSA",
					detail: "resources may be bound only once",
				})
			}
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
			let prior = binding_by_device.insert(device, binding);
			debug_assert!(prior.is_none());
		}
		for device in plan.devices() {
			debug_assert!(binding_by_device.contains_key(&device));
		}
		debug_assert!(
			binding_by_device
				.keys()
				.all(|device| plan.devices().any(|planned| planned == *device))
		);

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
		for binding in binding_by_device.values() {
			debug_assert!(
				requested_submission_queue_count(bundle.resources(), &scoped_tasks, binding.device)
					<= binding.maximum_submission_queues() as usize
			);
			validate_binding(binding)?;
		}
		for (device, binding) in &binding_by_device {
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
		bind_finalized_artifact_resources(&mut devices, &plan)?;
		let contracts = task_contracts(bundle, tasks);
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
		let owner = &self.devices[&layout.device];
		let allocation = owner
			.session
			.allocate_coarse(bytes_to_usize(layout.size.get()))?;
		let sessions = self
			.devices
			.values()
			.map(|resources| resources.session)
			.collect::<Vec<_>>();
		owner.session
			.grant_access_exact_set(&allocation, &sessions, &[owner.host_allocator])?;
		Ok(HsaArena {
			device: layout.device,
			allocation,
		})
	}

	pub(crate) fn prepare_pending(&mut self, request: PendingRequest) -> Result<HsaPending<'scope>> {
		self.ensure_healthy()?;
		let contract = &self.contracts[&request.task];
		debug_assert!(
			request.phase == contract.phase
				&& request.class == contract.class
				&& request.submission == contract.submission
		);
		let planned = self.plan.submission(request.task).unwrap();
		let device = &self.devices[&planned.device];
		debug_assert!(
			device.queues.contains_key(&planned.slots.queue)
				&& device.completions.contains_key(&planned.slots.completion)
		);
		debug_assert!(!self.prepared_tasks.contains(&request.task));
		let native = self.pending_pool.remove(&request.task).unwrap();
		let inserted = self.prepared_tasks.insert(request.task);
		debug_assert!(inserted);
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
		let planned = self.plan.submission(task).unwrap();
		pending.validate_ready(&work, planned);
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
				if submission_error_requires_poison(&error) {
					self.poisoned = true;
				}
				Err(error)
			}
		}
	}

	pub(crate) fn poll_pending(&mut self, pending: &mut HsaPending<'scope>) -> Result<BackendPoll> {
		self.ensure_healthy()?;
		validate_active(self.device_mut(pending.device), pending);
		debug_assert!(matches!(pending.state, HsaPendingState::Active));
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

	pub(crate) fn execution_evidence(&self) -> Vec<NativeDeviceExecutionEvidence> {
		self.devices
			.iter()
			.map(|(&device, resources)| {
				NativeDeviceExecutionEvidence {
					device,
					backend: NativeBackendKind::Hsa,
					image_loads: resources.executables.len(),
					entry_lookups: resources.artifacts.len(),
					queues: resources.queues.len(),
					completion_objects: resources.completions.len(),
					persistent_allocations: resources.kernargs.len()
						+ resources.metric_buffers.len()
						+ usize::from(resources.scratch.is_some())
						+ 1,
				}
			})
			.collect()
	}

	pub(crate) fn collect_exit(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		pending: &HsaPending<'scope>,
		work: TransferWork<'_>,
		destination: &mut [u8],
	) -> Result<()> {
		self.ensure_healthy()?;
		pending.validate_collected_exit(&work, destination.len());
		let ResolvedTransferEndpoint::Device(source_location) = work.source else {
			unreachable!()
		};
		checked_arena(arenas, source_location, work.bytes.get());
		let device = self.device_mut(pending.device);
		let source = &device.egress[&work.task];
		debug_assert_eq!(source.len(), destination.len());
		destination.copy_from_slice(source);
		Ok(())
	}

	pub(crate) fn destroy(self) -> Result<()> {
		self.ensure_healthy()?;
		let Self {
			devices,
			pending_pool,
			..
		} = self;
		// Prepared completion tokens may retain the executable used by their
		// last dispatch. Release those Rc keepalives before explicitly closing
		// each shared executable.
		drop(pending_pool);
		destroy_devices(devices)
	}

	fn submit_admission(
		&mut self,
		arenas: &impl HsaArenaLookup<'scope>,
		planned: crate::PlannedSubmission,
		work: InitAdmissionWork<'_>,
		native: &mut PreparedPending<'scope, 'scope>,
	) -> Result<PendingAction> {
		debug_assert!(planned.device == work.destination.device && work.submission == planned.slots);
		let arena = checked_arena(arenas, work.destination, work.bytes.get());
		let device = self.device_mut(planned.device);
		let bytes = bytes_to_usize(work.bytes.get());
		debug_assert!(
			device.admission
				== Some(InitImageContract {
					device: work.destination.device,
					image: work.destination.value,
					bytes: work.bytes,
				}) && work.image.len() == bytes
				&& device.staging.len() >= bytes,
		);
		let copy = unsafe {
			// SAFETY: `staging` was allocated from a discovered fine-grained
			// host-accessible pool, and no operation is using it during init.
			device.staging.copy_from_host(0, work.image)
		};
		copy?;
		let destination_offset = offset_to_usize(work.destination.arena_offset.get());
		ensure_queue(device, planned.slots.queue);
		claim_completion(&mut device.completions, work.task, planned.slots.completion);
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
		let (source, destination) = device_endpoints(work);
		debug_assert!(source.device == planned.device && work.submission == planned.slots);
		let source_arena = checked_arena(arenas, source, work.bytes.get());
		let destination_arena = checked_arena(arenas, destination, work.bytes.get());
		let source_offset = offset_to_usize(source.arena_offset.get());
		let destination_offset = offset_to_usize(destination.arena_offset.get());
		let bytes = bytes_to_usize(work.bytes.get());
		let device = self.device_mut(planned.device);
		ensure_queue(device, planned.slots.queue);
		claim_completion(&mut device.completions, work.task, planned.slots.completion);
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
			unreachable!()
		};
		debug_assert!(source.device == planned.device && work.submission == planned.slots);
		let source_arena = checked_arena(arenas, source, work.bytes.get());
		let bytes = bytes_to_usize(work.bytes.get());
		let source_offset = offset_to_usize(source.arena_offset.get());
		match work.destination {
			ResolvedTransferEndpoint::External => {
				let device = self.device_mut(planned.device);
				debug_assert!(device.staging.len() >= bytes && device.egress.contains_key(&work.task));
				ensure_queue(device, planned.slots.queue);
				claim_completion(&mut device.completions, work.task, planned.slots.completion);
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
				let destination_arena = checked_arena(arenas, destination, work.bytes.get());
				let destination_offset = offset_to_usize(destination.arena_offset.get());
				let device = self.device_mut(planned.device);
				ensure_queue(device, planned.slots.queue);
				claim_completion(&mut device.completions, work.task, planned.slots.completion);
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
		debug_assert!(work.device == planned.device && work.submission == planned.slots);
		let geometry = {
			let device = self.device_mut(planned.device);
			let artifact = &device.artifacts[&work.artifact];
			let dynamic_private_bytes = artifact.resources.as_ref().unwrap().dynamic_private_bytes;
			let slot = device.kernargs.get_mut(&planned.slots.completion).unwrap();
			fill_kernarg(slot, arenas, work, &artifact.abi)?;
			let queue = &device.queues[&planned.slots.queue];
			let progress = queue.progress_capacity(1, NonZeroU32::MIN)?;
			ensure(
				matches!(progress, QueueProgress::Ready { .. }),
				Error::ResourceContention {
					task: work.task,
					detail: "HSA AQL queue is backpressured",
				},
			)?;
			let workgroup_lanes = NonZeroU32::new(artifact.abi.workgroup_lanes).unwrap();
			let grid = hsa_grid_size(artifact.abi.elements.get(), workgroup_lanes)?;
			let lanes = u16_from_u32(workgroup_lanes.get(), "HSA workgroup dimension")?;
			let mut geometry = DispatchGeometry::one_dimensional(grid, lanes);
			geometry.dynamic_private_bytes = dynamic_private_bytes;
			debug_assert!(geometry.grid[0] == grid
					&& geometry.workgroup[0] == lanes
					&& geometry.dynamic_private_bytes == dynamic_private_bytes
					&& u64::from(grid) >= artifact.abi.elements.get());
			geometry
		};
		let device = self.device_mut(planned.device);
		let artifact = &device.artifacts[&work.artifact];
		let slot = &device.kernargs[&planned.slots.completion];
		let queue = &device.queues[&planned.slots.queue];
		claim_completion(&mut device.completions, work.task, planned.slots.completion);
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
		debug_assert!(work.value.device == planned.device && work.value.bytes.get() == 4);
		let arena = checked_arena(arenas, work.value, 4);
		let device = self.device_mut(planned.device);
		ensure_queue(device, planned.slots.queue);
		let metric_buffer = &device.metric_buffers[&work.task];
		debug_assert_eq!(metric_buffer.len(), 4);
		let source_offset = offset_to_usize(work.value.arena_offset.get());
		claim_completion(&mut device.completions, work.task, planned.slots.completion);
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
			let device = self.device_mut(pending.device);
			release_completion(&mut device.completions, pending.task, pending.completion);
			pending.state = HsaPendingState::Terminal;
			finish_action(device, pending.task, pending.action)
		};
		match action_result {
			Ok(metric) => Ok(BackendPoll::Complete { metric }),
			Err(error) => self.poison(error),
		}
	}

	pub(crate) fn rearm_pending(&mut self, pending: &mut HsaPending<'scope>) -> Result<()> {
		self.ensure_healthy()?;
		debug_assert!(pending.phase == RunPhase::Loop && pending.state == HsaPendingState::Terminal);
		pending.native.reset()?;
		pending.action = PendingAction::None;
		pending.state = HsaPendingState::Ready;
		Ok(())
	}

	pub(crate) fn prepare_loop_pending(&mut self, pending: &mut HsaPending<'scope>) -> Result<()> {
		self.ensure_healthy()?;
		debug_assert_eq!(pending.phase, RunPhase::Loop);
		match pending.state.loop_submission_action() {
			Some(LoopSubmissionAction::UsePrepared) => Ok(()),
			Some(LoopSubmissionAction::Rearm) => self.rearm_pending(pending),
			None => unreachable!(),
		}
	}

	pub(crate) fn recycle_pending(&mut self, mut pending: HsaPending<'scope>) -> Result<()> {
		self.ensure_healthy()?;
		let prepared = self.prepared_tasks.remove(&pending.task);
		debug_assert!(pending.state == HsaPendingState::Terminal && prepared);
		pending.native.reset()?;
		let task = pending.task;
		let prior = self.pending_pool.insert(task, pending.native);
		debug_assert!(prior.is_none());
		Ok(())
	}

	pub(crate) fn available_bytes(&self, device: DeviceId) -> Result<ByteCount> {
		let resources = &self.devices[&device];
		Ok(ByteCount::new(resources.session.available_memory_bytes()?))
	}

	pub(crate) fn validate_handoff(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<()> {
		self.ensure_healthy()?;
		debug_assert!(self.prepared_tasks.is_empty() && self.pending_pool.keys().copied().eq(tasks.iter().copied()));
		for (device, resources) in &self.devices {
			let finalized = bundle.init_image(*device).map(InitImageContract::from);
			debug_assert_eq!(resources.admission, finalized);
		}
		let runtime_artifacts = self.plan.runtime_artifacts().cloned().collect();
		let devices = self.devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts, devices, tasks)?;
		bind_finalized_artifact_resources(&mut self.devices, &plan)?;
		let contracts = task_contracts(bundle, Some(tasks));
		self.plan = plan;
		self.contracts = contracts;
		Ok(())
	}

	fn device_mut(&mut self, device: DeviceId) -> &mut DeviceResources<'scope> {
		self.devices.get_mut(&device).unwrap()
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
			let prior = binding_by_device.insert(device, binding);
			debug_assert!(prior.is_none());
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
			debug_assert!(binding_by_device.contains_key(&calculation.device));
		}
		let artifact_ids = scoped_tasks
			.iter()
			.filter_map(|task| {
				match &task.kind {
					TaskKind::Calculation(calculation) => Some(calculation.artifact),
					TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
				}
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
			mut devices,
			pending,
		} = self;
		let (plan, contracts) = match handoff {
			HsaPreparedHandoff::Finalized {
				bundle: prepared_bundle,
				tasks: prepared_tasks,
				plan,
				contracts,
			} => {
				debug_assert!(prepared_bundle == bundle.identity() && prepared_tasks == *tasks);
				Ok((plan, contracts))
			}
			HsaPreparedHandoff::Candidate(_) => {
				Err(Error::BackendState {
					backend: "HSA",
					detail: "candidate resources were not validated for finalized handoff",
				})
			}
		}?;
		bind_finalized_artifact_resources(&mut devices, &plan)?;
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
			mut devices,
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
		debug_assert!(pending.keys().copied().eq(tasks.iter().copied()));
		for (device, resources) in &devices {
			let warm = bundle.init_image(*device).map(InitImageContract::from);
			debug_assert_eq!(resources.admission, warm);
		}
		let planned_devices = devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts, planned_devices, tasks)?;
		bind_finalized_artifact_resources(&mut devices, &plan)?;
		let contracts = task_contracts(bundle, Some(tasks));
		Ok(HsaResources {
			plan,
			devices,
			contracts,
			prepared_tasks: BTreeSet::new(),
			pending_pool: pending,
			poisoned: false,
		})
	}

	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently validates warmed resources"
	)]
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
			debug_assert_eq!(resources.admission, finalized);
		}
		let devices = self.devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts.clone(), devices, tasks)?;
		bind_finalized_artifact_resources(&mut self.devices, &plan)?;
		let contracts = task_contracts(bundle, Some(tasks));
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

	const MAX_NON_POLL_PHYSICAL_CALLS: usize = 1;

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
			HsaBackendState::Bound => {
				Err(Error::BackendState {
					backend: "HSA",
					detail: "resources may be bound only once",
				})
			}
		}
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Pending, Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::PreparePending {
			task: request.task,
		})?;
		resource.prepare_pending(request)
	}

	fn allocate_arena(
		&mut self,
		resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Arena, Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::AllocateArena {
			device: layout.device,
			bytes: layout.size,
		})?;
		resource.allocate_arena(layout)
	}

	fn supports_loop_repetition(&self) -> bool { true }

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

	fn submit_loop_iteration(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		_iteration: LoopIteration,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		let call = crate::accounting::submission_call(&work);
		crate::accounting::record(physical_calls, call)?;
		resource.prepare_loop_pending(pending)?;
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
			Err(error) => {
				match error {
					Error::DuplicateArtifact { .. }
					| Error::MissingArtifact { .. }
					| Error::UnexpectedArtifact { .. }
					| Error::ArtifactMismatch { .. }
					| Error::DuplicateDevice { .. }
					| Error::MissingDevice { .. }
					| Error::UnexpectedDevice { .. }
					| Error::MissingQueue { .. }
					| Error::MissingCompletion { .. }
					| Error::ResourceContention { .. }
					| Error::CompletionBusy { .. }
					| Error::ArenaMismatch { .. }
					| Error::ValueMismatch { .. }
					| Error::UnsupportedTransfer { .. }
					| Error::UnsupportedLoopContract { .. }
					| Error::BackendState { .. }
					| Error::BackendPoisoned { .. }
					| Error::SubmissionQueueLimitExceeded { .. }
					| Error::IntegerOverflow { .. }
					| Error::PhysicalAccountingOverflow
					| Error::CudaContract(_)
					| Error::Cuda(_)
					| Error::HsaSymbolLookup { .. }
					| Error::Hsa(_)
					| Error::Kernel(_)
					| Error::Protocol { .. } => PhysicalPollStatus::Failed,
				}
			}
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
		crate::accounting::record(physical_calls, PhysicalCall::CollectExit {
			task: work.task,
			bytes: work.bytes,
		})?;
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
		resource.ensure_healthy()?;
		debug_assert_eq!(arena.device(), device);
		arena.release()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopSubmissionAction {
	UsePrepared,
	Rearm,
}

impl HsaPendingState {
	const fn loop_submission_action(self) -> Option<LoopSubmissionAction> {
		match self {
			Self::Ready => Some(LoopSubmissionAction::UsePrepared),
			Self::Terminal => Some(LoopSubmissionAction::Rearm),
			Self::Active => None,
		}
	}
}

pub struct HsaPending<'scope> {
	task: TaskId,
	phase: RunPhase,
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
			phase: request.phase,
			device: planned.device,
			queue: planned.slots.queue,
			completion: planned.slots.completion,
			class: request.class,
			native,
			action: PendingAction::None,
			state: HsaPendingState::Ready,
		}
	}

	fn validate_ready(&self, work: &BackendWork<'_>, planned: crate::PlannedSubmission) {
		debug_assert!(
			self.state == HsaPendingState::Ready
				&& self.task == work.task()
				&& self.class == work.class()
				&& self.device == planned.device
				&& self.queue == planned.slots.queue
				&& self.completion == planned.slots.completion
		);
	}

	fn activate(&mut self, action: PendingAction) {
		self.action = action;
		self.state = HsaPendingState::Active;
	}

	fn validate_collected_exit(&self, work: &TransferWork<'_>, destination_bytes: usize) {
		let bytes = bytes_to_usize(work.bytes.get());
		debug_assert!(
			self.task == work.task
				&& self.class == WorkClass::ExitTransfer
				&& self.state == HsaPendingState::Terminal
				&& self.queue == work.submission.queue
				&& self.completion == work.submission.completion
				&& self.action == PendingAction::Egress { bytes }
				&& destination_bytes == bytes,
		);
	}

	pub(crate) fn task(&self) -> TaskId { self.task }
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
	let allocator = binding.host_allocator().description();
	ensure(
		allocator.device_type == DeviceType::Cpu,
		Error::ArtifactMismatch {
			artifact: ArtifactId::new(0),
			detail: "HSA kernarg allocator is not a CPU agent".to_owned(),
		},
	)?;
	let has_kernarg_pool = allocator.memory_pools.iter().any(|pool| {
		pool.segment == MemorySegment::Global
			&& pool.runtime_allocation.is_some()
			&& pool
				.global_flags
				.is_some_and(|flags| flags.contains(MemoryPoolFlags::KERNARG_INITIALIZATION))
	});
	ensure(has_kernarg_pool, Error::ArtifactMismatch {
		artifact: ArtifactId::new(0),
		detail: "HSA CPU kernarg allocator has no allocatable kernarg pool".to_owned(),
	})?;
	let has_fine_pool = allocator.memory_pools.iter().any(|pool| {
		pool.segment == MemorySegment::Global
			&& pool.runtime_allocation.is_some()
			&& pool.global_flags.is_some_and(|flags| {
				flags.contains(MemoryPoolFlags::FINE_GRAINED)
					|| flags.contains(MemoryPoolFlags::EXTENDED_SCOPE_FINE_GRAINED)
			})
	});
	ensure(has_fine_pool, Error::ArtifactMismatch {
		artifact: ArtifactId::new(0),
		detail: "HSA CPU host allocator has no allocatable fine-grained pool".to_owned(),
	})?;
	let has_target = binding
		.session
		.description()
		.isas
		.iter()
		.filter_map(|isa| isa.amd_target.as_ref())
		.any(|target| target.as_str() == binding.target_id());
	ensure(has_target, Error::ArtifactMismatch {
		artifact: ArtifactId::new(0),
		detail: format!(
			"HSA session does not advertise exact target {:?}",
			binding.target_id()
		),
	})
}

fn validate_reservation(reservations: &ReservationLedger, device: DeviceId) -> Result<()> {
	let reservation = reservations.entry(device).unwrap();
	require_enforced_quota(device, reservation.mechanism)
}

fn require_enforced_quota(device: DeviceId, mechanism: ReservationMechanism) -> Result<()> {
	ensure(
		mechanism == ReservationMechanism::EnforcedQuota,
		Error::ArenaMismatch {
			device,
			detail: "HSA bridge requires a scheduler-enforced quota",
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
		let device = candidate_task_device(task, value_devices);
		let resources = &devices[&device];
		let token = resources.session.prepare_pending(2, 0)?;
		let prior = pending.insert(task.id, token);
		debug_assert!(prior.is_none());
	}
	Ok(pending)
}

fn candidate_task_device(task: &Task, value_devices: &BTreeMap<ValueId, DeviceId>) -> DeviceId {
	match &task.kind {
		TaskKind::Calculation(calculation) => calculation.device,
		TaskKind::Metric(metric) => value_devices[&metric.value],
		TaskKind::Transfer(transfer) => {
			match transfer.source {
				TransferEndpoint::Device { device, .. } => device,
				TransferEndpoint::External => {
					match transfer.destination {
						TransferEndpoint::Device { device, .. } => device,
						TransferEndpoint::External => unreachable!(),
					}
				}
			}
		}
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
	validate_reservation(reservations, device)?;
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
					binding.queue_packets(),
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
		.unwrap()
		.bytes
		.get();
	let staging = binding.allocate_host_fine(bytes_to_usize(staging_bytes))?;
	let admission = InitImageContract::from(init_images
		.iter()
		.find(|manifest| manifest.device == device)
		.unwrap());
	debug_assert!(admission.bytes.get() <= staging_bytes);
	let scratch = match resources
		.scratch
		.iter()
		.find(|entry| entry.device == device)
		.map(|entry| entry.bytes.get())
	{
		Some(0) | None => None,
		Some(bytes) => {
			Some(binding
				.session
				.allocate_coarse(bytes_to_usize(bytes))?)
		}
	};
	let artifact_ids = tasks
		.iter()
		.filter_map(|task| {
			match &task.kind {
				TaskKind::Calculation(calculation) => {
					(calculation.device == device).then_some(calculation.artifact)
				}
				TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
			}
		})
		.collect::<BTreeSet<_>>();
	let mut bundles = BTreeMap::<ArtifactDigest, Vec<(ArtifactId, &crate::RuntimeArtifact)>>::new();
	for artifact_id in artifact_ids {
		let runtime = &runtime_artifacts[&artifact_id];
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
			target_id.as_str() == binding.target_id() && *code_object_version == binding.code_object_version(),
			Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "HSACO target or code-object version differs from the HSA binding".to_owned(),
			},
		)?;
		let digest = runtime.digest();
		if let Some((_, first)) = bundles.get(&digest).and_then(|entries| entries.first()) {
			ensure(
				first.bytes.as_ref() == runtime.bytes.as_ref(),
				Error::ArtifactMismatch {
					artifact: artifact_id,
					detail: "distinct HSACO images produced the same content digest".to_owned(),
				},
			)?;
		}
		bundles
			.entry(digest)
			.or_default()
			.push((artifact_id, runtime));
	}

	// A runtime artifact identifies one logical entry point, while its byte
	// image may be the device target's shared multi-entry HSACO. Own one
	// executable per distinct image and resolve every logical stage from it.
	// `artifacts` is declared after `executables` so unwind drops Kernel Rc
	// owners before the underlying executable owner.
	let mut executables = BTreeMap::new();
	let mut artifacts = BTreeMap::new();
	for (digest, entries) in bundles {
		let (_, first_runtime) = entries
			.first()
			.expect("an HSACO bundle group is created only for an artifact");
		let inspections = inspect_hsaco_bundle(
			&first_runtime.bytes,
			binding.target_id(),
			binding.code_object_version(),
			entries.iter().map(|(_, runtime)| &runtime.abi),
		)?;
		let executable = binding.session.load_hsaco(&first_runtime.bytes)?;
		for ((artifact_id, runtime), inspection) in entries.into_iter().zip(inspections) {
			ensure(
				inspection.kernel.name == runtime.abi.entry_symbol,
				Error::ArtifactMismatch {
					artifact: artifact_id,
					detail: "inspected HSACO entry differs from immutable ABI".to_owned(),
				},
			)?;
			let runtime_symbol = inspection.kernel.symbol;
			let kernel = executable.kernel(&runtime_symbol).map_err(|source| {
				Error::HsaSymbolLookup {
					artifact: artifact_id,
					abi_entry: runtime.abi.entry_symbol.clone(),
					runtime_symbol: runtime_symbol.clone(),
					source,
				}
			})?;
			ensure(
				kernel.metadata().kernarg_segment_size >= runtime.abi.argument_bytes
					&& kernel.metadata().kernarg_segment_alignment >= runtime.abi.argument_alignment,
				Error::ArtifactMismatch {
					artifact: artifact_id,
					detail: "loaded HSA kernel metadata is smaller than the inspected ABI".to_owned(),
				},
			)?;
			artifacts.insert(artifact_id, LoadedArtifact {
				kernel,
				abi: runtime.abi.clone(),
				resources: None,
			});
		}
		executables.insert(digest, executable);
	}

	let kernarg_sizes = kernarg_sizes(tasks, device, &artifacts)?;
	let mut kernargs = BTreeMap::new();
	for (slot, bytes) in kernarg_sizes {
		let allocation = binding.host_allocator().allocate_kernarg(bytes)?;
		binding.session.grant_access(&allocation)?;
		kernargs.insert(slot, KernargSlot {
			allocation,
			bytes: vec![0_u8; bytes],
		});
	}
	let metric_buffers = tasks
		.iter()
		.filter_map(|task| {
			match &task.kind {
				TaskKind::Metric(metric) => (value_devices[&metric.value] == device).then_some(task.id),
				TaskKind::Calculation(_) | TaskKind::Transfer(_) => None,
			}
		})
		.map(|task| Ok((task, binding.allocate_host_fine(4)?)))
		.collect::<Result<BTreeMap<_, _>>>()?;
	let egress = tasks
		.iter()
		.filter_map(|task| {
			match &task.kind {
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
			}
		})
		.map(|(task, bytes)| (task, vec![0_u8; bytes_to_usize(bytes)]))
		.collect::<BTreeMap<_, _>>();

	Ok(DeviceResources {
		session: binding.session,
		host_allocator: binding.host_allocator(),
		queues,
		completions,
		artifacts,
		executables,
		kernargs,
		metric_buffers,
		staging,
		admission: Some(admission),
		egress,
		scratch,
	})
}

fn bind_finalized_artifact_resources(
	devices: &mut BTreeMap<DeviceId, DeviceResources<'_>>,
	plan: &ExecutionPlan,
) -> Result<()> {
	for device in devices.values_mut() {
		for (artifact, loaded) in &mut device.artifacts {
			let contract = plan.artifact_contract(*artifact).unwrap();
			debug_assert_eq!(loaded.abi, contract.runtime.abi);
			let resources = hsa_artifact_resource_envelope(
				*artifact,
				&contract.identity.resources,
				loaded.kernel.metadata().dynamic_callstack,
			)?;
			match &loaded.resources {
				Some(prior) => {
					debug_assert_eq!(prior, &resources)
				}
				None => loaded.resources = Some(resources),
			}
		}
	}
	Ok(())
}

fn hsa_artifact_resource_envelope(
	artifact: ArtifactId,
	finalized: &KernelResourceBounds,
	dynamic_callstack: bool,
) -> Result<HsaArtifactResourceEnvelope> {
	let dynamic_private_bytes = match dynamic_callstack {
		false => 0,
		true => {
			let private_bytes = finalized.private_bytes_per_lane.get();
			ensure(private_bytes != 0, Error::ArtifactMismatch {
				artifact,
				detail: "dynamic-callstack HSA kernel has a zero finalized private-byte bound".to_owned(),
			})?;
			u32::try_from(private_bytes).map_err(|_| {
				Error::ArtifactMismatch {
					artifact,
					detail: "dynamic-callstack HSA kernel private-byte bound exceeds the AQL field"
						.to_owned(),
				}
			})?
		}
	};
	Ok(HsaArtifactResourceEnvelope {
		finalized: finalized.clone(),
		dynamic_private_bytes,
	})
}

fn task_contracts(
	bundle: &FinalizedBundle,
	selected: Option<&BTreeSet<TaskId>>,
) -> BTreeMap<TaskId, HsaTaskContract> {
	let mut contracts = BTreeMap::new();
	for task in bundle.tasks().iter().filter(|task| {
		match selected {
			Some(selected) => selected.contains(&task.id),
			None => true,
		}
	}) {
		let (class, submission, admission, route, lane_claims) = match &task.kind {
			TaskKind::Calculation(calculation) => {
				debug_assert_eq!(task.phase, RunPhase::Loop);
				(
					WorkClass::Calculation,
					Some(calculation.submission),
					None,
					Vec::new(),
					Vec::new(),
				)
			}
			TaskKind::Metric(metric) => {
				debug_assert_eq!(task.phase, RunPhase::Loop);
				(
					WorkClass::Metric,
					Some(metric.submission),
					None,
					Vec::new(),
					Vec::new(),
				)
			}
			TaskKind::Transfer(transfer) => {
				let class = transfer_work_class(task.phase, transfer.source, transfer.destination);
				let admission = match class {
					WorkClass::InitAdmission => {
						let endpoints = bundle.transfer_endpoints(task.id).unwrap();
						let ResolvedTransferEndpoint::Device(destination) = endpoints.destination else {
							unreachable!("finalized admission has a device image");
						};
						let manifest = bundle.init_image(destination.device).unwrap();
						let contract = InitImageContract::from(manifest);
						debug_assert!(
							contract.image == destination.value && contract.bytes == transfer.bytes,
						);
						Some(contract)
					}
					WorkClass::InternalTransfer | WorkClass::ExitTransfer => None,
					WorkClass::Calculation | WorkClass::Metric => unreachable!(),
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
		let prior = contracts.insert(task.id, contract);
		debug_assert!(prior.is_none());
	}
	contracts
}

fn transfer_work_class(
	phase: RunPhase,
	source: recipe_core::TransferEndpoint,
	destination: recipe_core::TransferEndpoint,
) -> WorkClass {
	use recipe_core::TransferEndpoint::{Device, External};

	match (phase, source, destination) {
		(RunPhase::Init, External, Device { .. }) => WorkClass::InitAdmission,
		(RunPhase::Init | RunPhase::Loop, Device { .. }, Device { .. }) => WorkClass::InternalTransfer,
		(RunPhase::Exit, Device { .. }, Device { .. } | External) => WorkClass::ExitTransfer,
		(RunPhase::Init, External, External)
		| (RunPhase::Init, Device { .. }, External)
		| (RunPhase::Loop, External, External | Device { .. })
		| (RunPhase::Loop, Device { .. }, External)
		| (RunPhase::Exit, External, External | Device { .. }) => unreachable!(),
	}
}

fn task_submission(task: &Task) -> Option<SubmissionSlots> {
	match &task.kind {
		TaskKind::Calculation(calculation) => Some(calculation.submission),
		TaskKind::Transfer(transfer) => Some(transfer.submission),
		TaskKind::Metric(metric) => Some(metric.submission),
	}
}

fn requested_submission_queue_count(resources: &ResourceManifest, tasks: &[Task], device: DeviceId) -> usize {
	let queue_ids = tasks
		.iter()
		.filter_map(task_submission)
		.map(|submission| submission.queue)
		.collect::<BTreeSet<_>>();
	resources
		.queues
		.iter()
		.filter(|slot| slot.device == device && queue_ids.contains(&slot.id))
		.count()
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
				let artifact = &artifacts[&calculation.artifact];
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
	debug_assert!(slot.bytes.len() >= usize::try_from(abi.argument_bytes).unwrap());
	slot.bytes.fill(0);
	let mut locations = work.inputs.iter().chain(work.outputs.iter()).copied();
	let mut has_fault_argument = false;
	for (argument_index, argument) in abi.arguments.iter().enumerate() {
		let value = match argument {
			KernelArgument::Buffer { .. } => {
				let location = locations.next().unwrap();
				let arena = checked_arena(arenas, location, location.bytes.get());
				let base = u64::try_from(arena.allocation.as_ptr().addr()).unwrap();
				base.checked_add(location.arena_offset.get()).unwrap()
			}
			KernelArgument::RunId => work.run.get(),
			KernelArgument::LoopIteration => work.iteration.index(),
			KernelArgument::ElementCount => abi.elements.get(),
			KernelArgument::FaultFlag => {
				has_fault_argument = true;
				let location = work.fault_flag.unwrap();
				debug_assert!(location.dtype == recipe_core::DType::I32 && location.bytes.get() == 4);
				let arena = checked_arena(arenas, location, 4);
				let base = u64::try_from(arena.allocation.as_ptr().addr()).unwrap();
				base.checked_add(location.arena_offset.get()).unwrap()
			}
		};
		let offset = argument_index.checked_mul(8).unwrap();
		let end = offset.checked_add(8).unwrap();
		let destination = &mut slot.bytes[offset..end];
		destination.copy_from_slice(&value.to_le_bytes());
	}
	debug_assert!(locations.next().is_none() && has_fault_argument == work.fault_flag.is_some());
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
) -> &'arena HsaArena<'scope> {
	let arena = arenas.arena(location.device).unwrap();
	debug_assert_eq!(arena.device, location.device);
	let end = location
		.arena_offset
		.get()
		.checked_add(bytes)
		.unwrap();
	let arena_bytes = u64::try_from(arena.allocation.len()).unwrap();
	debug_assert!(end <= arena_bytes);
	arena
}

fn device_endpoints(work: &TransferWork) -> (ResolvedValueLocation, ResolvedValueLocation) {
	match (work.source, work.destination) {
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => {
			(source, destination)
		}
		_ => unreachable!("finalized internal transfer has two device endpoints"),
	}
}

fn claim_completion(
	completions: &mut BTreeMap<CompletionSlotId, CompletionState>,
	task: TaskId,
	completion: CompletionSlotId,
) {
	let slot = completions.get_mut(&completion).unwrap();
	match core::mem::replace(slot, CompletionState::Active { task }) {
		CompletionState::Available => {}
		CompletionState::Active { .. } => unreachable!(),
	}
}

fn release_completion(
	completions: &mut BTreeMap<CompletionSlotId, CompletionState>,
	task: TaskId,
	completion: CompletionSlotId,
) {
	let slot = completions.get_mut(&completion).unwrap();
	debug_assert!(completion_owned(slot, task));
	*slot = CompletionState::Available;
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
			release_completion(completions, task, completion);
			Err(Error::from(error))
		}
	}
}

fn submission_error_requires_poison(error: &Error) -> bool {
	match error {
		Error::BackendPoisoned { .. }
		| Error::Hsa(recipe_hsa::Error::SessionPoisoned { .. })
		| Error::Hsa(recipe_hsa::Error::DeferredRetirement { poisoned: true, .. }) => true,
		Error::Hsa(recipe_hsa::Error::AsyncSignal { value }) => *value < 0,
		_ => false,
	}
}

fn ensure_queue(device: &DeviceResources<'_>, queue: QueueSlotId) {
	debug_assert!(device.queues.contains_key(&queue));
}

fn validate_active(device: &DeviceResources<'_>, pending: &HsaPending<'_>) {
	let state = &device.completions[&pending.completion];
	debug_assert!(completion_owned(state, pending.task));
}

fn finish_action(device: &mut DeviceResources<'_>, task: TaskId, action: PendingAction) -> Result<Option<MetricValue>> {
	match action {
		PendingAction::None => Ok(None),
		PendingAction::Metric { dtype } => {
			let buffer = &device.metric_buffers[&task];
			debug_assert_eq!(buffer.len(), 4);
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
			let output = device.egress.get_mut(&task).unwrap();
			debug_assert!(output.len() == bytes && device.staging.len() >= bytes);
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
			debug_assert!(matches!(state, CompletionState::Available));
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
			drop(artifact);
		}
		for (_, executable) in device.executables {
			executable.close().map_err(Error::from)?;
		}
		for (_, slot) in device.kernargs {
			slot.allocation.close()?;
		}
		for (_, allocation) in device.metric_buffers {
			allocation.close()?;
		}
		device.staging.close()?;
		free_optional_allocation(device.scratch)?;
	}
	Ok(())
}

fn free_optional_allocation(allocation: Option<Allocation<'_>>) -> Result<()> {
	match allocation {
		Some(allocation) => allocation.close().map_err(Error::from),
		None => Ok(()),
	}
}

fn bytes_to_usize(bytes: u64) -> usize { usize::try_from(bytes).unwrap() }

fn offset_to_usize(offset: u64) -> usize { bytes_to_usize(offset) }

fn u32_from_u64(value: u64, field: &'static str) -> Result<u32> {
	match u32::try_from(value) {
		Ok(value) => Ok(value),
		Err(..) => Err(Error::IntegerOverflow { field }),
	}
}

fn hsa_grid_size(elements: u64, workgroup_lanes: NonZeroU32) -> Result<u32> {
	let lanes = u64::from(workgroup_lanes.get());
	let workgroups = elements
		.checked_add(lanes - 1)
		.ok_or(Error::IntegerOverflow {
			field: "HSA grid rounding",
		})? / lanes;
	let padded_lanes = workgroups
		.checked_mul(lanes)
		.ok_or(Error::IntegerOverflow {
			field: "HSA padded grid dimension",
		})?;
	u32_from_u64(padded_lanes, "HSA padded grid dimension")
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

fn ensure(valid: bool, error: Error) -> Result<()> {
	match valid {
		true => Ok(()),
		false => Err(error),
	}
}
