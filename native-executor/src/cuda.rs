//! CUDA Driver resource realization and submission bridge.

use core::fmt;
use std::{
	collections::{BTreeMap, BTreeSet},
	time::Duration,
};

use recipe_core::{ArenaLayout, ArtifactId, BundleIdentity, ByteCount, CompletionSlotId, DeviceId, DraftPlan, FinalizedBundle, InitDataImage, LinkId, LoopIteration, QueueSlotId, ReservationLedger, ReservationMechanism, ResolvedTransferEndpoint, ResolvedValueLocation, ResourceManifest, RunPhase, SubmissionSlots, Task, TaskId, TaskKind, TransferLaneClaim, ValueId};
use recipe_cuda::{CompletionStatus, Context, DeploymentIdentity, DeviceBuffer, Event, Function, LaunchConfig, Module, Pending, PinnedHostBuffer, Stream, WaitOutcome, validate_artifact_compatibility};
use recipe_executor::{ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork, MetricValue, MetricWork, PendingRequest, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus, TransferWork, WorkClass, sealed};
use recipe_kernel::{ArtifactDigest, KernelArgument, inspect_cubin};

use crate::{Error, ExecutionPlan, NativeBackendKind, NativeDeviceExecutionEvidence, Result, RuntimeArtifactKind, cuda_ffi::ParameterBlock, plan::InitImageContract};

/// CUDA Driver API exposes no finite device stream-count attribute. Recipe
/// therefore reports and enforces this bounded native-executor ceiling rather
/// than claiming an unbounded or hardware-measured stream capacity.
pub const CUDA_MAXIMUM_SUBMISSION_QUEUES: u32 = 32;

fn consume_context<T>(context: T) { drop(context); }

#[derive(Clone)]
pub struct CudaBinding<'context> {
	device: DeviceId,
	context: &'context Context,
	deployment: DeploymentIdentity,
	maximum_submission_queues: u32,
	enabled_display_connectors: u32,
}

impl<'context> CudaBinding<'context> {
	#[must_use]
	pub const fn new(device: DeviceId, context: &'context Context, deployment: DeploymentIdentity, maximum_submission_queues: u32, enabled_display_connectors: u32) -> Self {
		Self {
			device,
			context,
			deployment,
			maximum_submission_queues,
			enabled_display_connectors,
		}
	}

	#[must_use]
	pub const fn device(&self) -> DeviceId { self.device }

	/// Exact deployment identity observed while this binding's CUDA context
	/// was reopened.
	#[must_use]
	pub const fn deployment(&self) -> &DeploymentIdentity { &self.deployment }

	#[must_use]
	pub const fn maximum_submission_queues(&self) -> u32 { self.maximum_submission_queues }

	#[must_use]
	pub const fn enabled_display_connectors(&self) -> u32 { self.enabled_display_connectors }

	/// Queries the CUDA driver's current free-memory counter for this exact
	/// binding before any candidate resources are realized.
	pub fn available_bytes(&self) -> Result<ByteCount> {
		let info = self.context.memory_info()?;
		let bytes = u64::try_from(info.free_bytes).map_err(|_| {
			Error::ArenaMismatch {
				device: self.device,
				detail: "CUDA free-memory counter does not fit Recipe byte units",
			}
		})?;
		Ok(ByteCount::new(bytes))
	}

	pub(crate) const fn context(&self) -> &'context Context { self.context }
}

impl fmt::Debug for CudaBinding<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("CudaBinding")
			.field("device", &self.device)
			.field("deployment", &self.deployment)
			.field("maximum_submission_queues", &self.maximum_submission_queues)
			.field(
				"enabled_display_connectors",
				&self.enabled_display_connectors,
			)
			.finish_non_exhaustive()
	}
}

pub(crate) trait CudaArenaLookup<'context> {
	fn arena(&self, device: DeviceId) -> Option<&CudaArena<'context>>;
}

impl<'context> CudaArenaLookup<'context> for BTreeMap<DeviceId, CudaArena<'context>> {
	fn arena(&self, device: DeviceId) -> Option<&CudaArena<'context>> { self.get(&device) }
}

impl<'borrow, 'context> CudaArenaLookup<'context> for ArenaSet<'borrow, CudaArena<'context>> {
	fn arena(&self, device: DeviceId) -> Option<&CudaArena<'context>> { self.get(device) }
}

pub struct CudaArena<'context> {
	device: DeviceId,
	buffer: DeviceBuffer<'context>,
}

impl<'context> CudaArena<'context> {
	pub(crate) fn device(&self) -> DeviceId { self.device }

	pub(crate) const fn buffer(&self) -> &DeviceBuffer<'context> { &self.buffer }

	pub fn bytes(&self) -> usize { self.buffer.len() }

	pub(crate) fn release(self) -> Result<()> { self.buffer.free().map_err(Error::from) }
}

impl fmt::Debug for CudaArena<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("CudaArena")
			.field("device", &self.device)
			.field("bytes", &self.buffer.len())
			.finish_non_exhaustive()
	}
}

struct LoadedArtifact<'context> {
	function: Function<'context, 'context>,
	abi: recipe_kernel::KernelAbi,
}

impl fmt::Debug for LoadedArtifact<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("LoadedArtifact")
			.field("entry", &self.abi.entry_symbol)
			.finish()
	}
}

enum CompletionEvent<'context> {
	Available(Event<'context>),
	Active { task: TaskId },
}

enum NativePendingState<'context> {
	Ready,
	Active(PendingSubmission<'context>),
	Terminal,
}

enum PendingSubmission<'context> {
	Event(Pending<'static, 'context>),
	Stream,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TaskContract {
	phase: RunPhase,
	class: WorkClass,
	submission: Option<SubmissionSlots>,
	admission: Option<InitImageContract>,
	route: Vec<LinkId>,
	lane_claims: Vec<TransferLaneClaim>,
}

struct DeviceResources<'context> {
	context: &'context Context,
	queues: BTreeMap<QueueSlotId, Stream<'context>>,
	completions: BTreeMap<CompletionSlotId, CompletionEvent<'context>>,
	artifacts: BTreeMap<ArtifactId, LoadedArtifact<'context>>,
	modules: BTreeMap<ArtifactDigest, Box<Module<'context>>>,
	invocations: BTreeMap<CompletionSlotId, ParameterBlock>,
	metric_buffers: BTreeMap<TaskId, PinnedHostBuffer<'context>>,
	staging: PinnedHostBuffer<'context>,
	admission: Option<InitImageContract>,
	egress: BTreeMap<TaskId, Vec<u8>>,
	scratch: Option<DeviceBuffer<'context>>,
}

impl fmt::Debug for DeviceResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("DeviceResources")
			.field("device", &self.context.device().uuid)
			.field("queue_count", &self.queues.len())
			.field("completion_count", &self.completions.len())
			.field("artifact_count", &self.artifacts.len())
			.field("module_count", &self.modules.len())
			.field("metric_buffer_count", &self.metric_buffers.len())
			.field("staging_bytes", &self.staging.len())
			.field("admission", &self.admission)
			.field("egress_count", &self.egress.len())
			.field(
				"scratch_bytes",
				&self.scratch.as_ref().map(DeviceBuffer::len),
			)
			.finish()
	}
}

pub struct CudaResources<'context> {
	plan: ExecutionPlan,
	contracts: BTreeMap<TaskId, TaskContract>,
	prepared_tasks: BTreeSet<TaskId>,
	devices: BTreeMap<DeviceId, DeviceResources<'context>>,
	poisoned: bool,
}

pub(crate) struct CudaPreparedResources<'context> {
	handoff: CudaPreparedHandoff,
	devices: BTreeMap<DeviceId, DeviceResources<'context>>,
}

#[derive(Debug)]
enum CudaPreparedHandoff {
	Candidate(Vec<crate::RuntimeArtifact>),
	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently hands off warmed resources"
	)]
	Finalized {
		bundle: BundleIdentity,
		tasks: BTreeSet<TaskId>,
		plan: ExecutionPlan,
		contracts: BTreeMap<TaskId, TaskContract>,
	},
}

#[derive(Debug)]
enum CudaBackendState<'context> {
	Ready {
		bindings: Vec<CudaBinding<'context>>,
		artifacts: Vec<crate::RuntimeArtifact>,
	},
	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently hands off warmed resources"
	)]
	Prepared(CudaPreparedResources<'context>),
	Warmed(CudaResources<'context>),
	Bound,
}

#[derive(Debug)]
pub struct CudaBackend<'context> {
	state: CudaBackendState<'context>,
}

impl<'context> CudaBackend<'context> {
	pub fn new(bindings: Vec<CudaBinding<'context>>, artifacts: Vec<crate::RuntimeArtifact>) -> Self {
		Self {
			state: CudaBackendState::Ready {
				bindings,
				artifacts,
			},
		}
	}

	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently hands off warmed resources"
	)]
	pub(crate) const fn from_prepared(resources: CudaPreparedResources<'context>) -> Self {
		Self {
			state: CudaBackendState::Prepared(resources),
		}
	}

	pub(crate) const fn from_warmed(resources: CudaResources<'context>) -> Self {
		Self {
			state: CudaBackendState::Warmed(resources),
		}
	}

	pub(crate) fn bind_partition(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<CudaResources<'context>> {
		let prior = core::mem::replace(&mut self.state, CudaBackendState::Bound);
		match prior {
			CudaBackendState::Ready {
				bindings,
				artifacts,
			} => {
				let devices = bindings.iter().map(CudaBinding::device).collect();
				let plan = ExecutionPlan::validate_partition(bundle, artifacts, devices, tasks)?;
				CudaResources::realize(bundle, plan, bindings, Some(tasks))
			}
			CudaBackendState::Prepared(resources) => resources.bind(bundle, tasks),
			CudaBackendState::Warmed(mut resources) => {
				resources.validate_handoff(bundle, tasks)?;
				Ok(resources)
			}
			CudaBackendState::Bound => {
				Err(Error::BackendState {
					backend: "CUDA",
					detail: "resources may be bound only once",
				})
			}
		}
	}
}

impl sealed::Sealed for CudaBackend<'_> {}

impl fmt::Debug for CudaResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("CudaResources")
			.field("plan", &self.plan)
			.field("devices", &self.devices)
			.field("poisoned", &self.poisoned)
			.finish()
	}
}

impl fmt::Debug for CudaPreparedResources<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("CudaPreparedResources")
			.field("handoff", &self.handoff)
			.field("devices", &self.devices)
			.finish()
	}
}

impl<'context> CudaResources<'context> {
	pub(crate) fn realize(bundle: &FinalizedBundle, plan: ExecutionPlan, bindings: Vec<CudaBinding<'context>>, tasks: Option<&BTreeSet<TaskId>>) -> Result<Self> {
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
			debug_assert!(requested_submission_queue_count(bundle.resources(), &scoped_tasks, binding.device) <= binding.maximum_submission_queues as usize);
		}
		for (device, binding) in binding_by_device {
			validate_binding(&binding)?;
			let resources = realize_device(
				bundle.resources(),
				bundle.reservations(),
				&scoped_tasks,
				&value_devices,
				bundle.init_images(),
				&runtime_by_id,
				&binding,
			)?;
			devices.insert(device, resources);
		}
		let contracts = task_contracts(bundle, tasks);
		Ok(Self {
			plan,
			contracts,
			prepared_tasks: BTreeSet::new(),
			devices,
			poisoned: false,
		})
	}

	pub(crate) fn allocate_arena(&self, layout: &ArenaLayout) -> Result<CudaArena<'context>> {
		let resources = &self.devices[&layout.device];
		let bytes = bytes_to_usize(layout.size.get());
		let buffer = DeviceBuffer::allocate(resources.context, bytes)?;
		Ok(CudaArena {
			device: layout.device,
			buffer,
		})
	}

	pub(crate) fn prepare_pending(&mut self, request: PendingRequest) -> Result<CudaPending<'context>> {
		self.ensure_healthy()?;
		let contract = &self.contracts[&request.task];
		debug_assert!(request.phase == contract.phase && request.class == contract.class && request.submission == contract.submission);
		let planned = self.plan.submission(request.task).unwrap();
		let resources = &self.devices[&planned.device];
		debug_assert!(
			resources.queues.contains_key(&planned.slots.queue)
				&& resources
					.completions
					.contains_key(&planned.slots.completion)
		);
		let inserted = self.prepared_tasks.insert(request.task);
		debug_assert!(inserted);
		Ok(CudaPending::ready(request, planned))
	}

	pub(crate) fn submit(&mut self, arenas: &impl CudaArenaLookup<'context>, pending: &mut CudaPending<'context>, work: BackendWork<'_>) -> Result<()> {
		self.ensure_healthy()?;
		let task = work.task();
		let planned = self.plan.submission(task).unwrap();
		pending.validate_ready(&work, planned);
		let result = match work {
			BackendWork::InitAdmission(work) => self.submit_admission(arenas, planned, work),
			BackendWork::Calculation(work) => self.submit_calculation(arenas, planned, &work),
			BackendWork::InternalTransfer(work) => self.submit_internal_transfer(arenas, planned, &work),
			BackendWork::Metric(work) => self.submit_metric(arenas, planned, work),
			BackendWork::ExitTransfer(work) => self.submit_exit_transfer(arenas, planned, &work),
		};
		match result {
			Ok((native, action)) => {
				pending.activate(native, action);
				Ok(())
			}
			Err(error) => {
				self.poisoned = true;
				Err(error)
			}
		}
	}

	pub(crate) fn poll(&mut self, pending: &mut CudaPending<'context>) -> Result<BackendPoll> {
		self.ensure_healthy()?;
		let device = self.device_mut(pending.device);
		validate_active(device, pending);
		let status = match &mut pending.native {
			NativePendingState::Active(PendingSubmission::Event(native)) => native.poll(),
			NativePendingState::Active(PendingSubmission::Stream) => queue(&device.queues, pending.queue).poll_idle(),
			NativePendingState::Ready | NativePendingState::Terminal => unreachable!(),
		};
		let status = match status {
			Ok(status) => status,
			Err(error) => return self.poison(Error::from(error)),
		};
		match status {
			CompletionStatus::Pending => Ok(BackendPoll::Pending),
			CompletionStatus::Complete => self.finish_pending(pending),
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
					backend: NativeBackendKind::Cuda,
					image_loads: resources.modules.len(),
					entry_lookups: resources.artifacts.len(),
					queues: resources.queues.len(),
					completion_objects: resources.completions.len(),
					persistent_allocations: resources.metric_buffers.len() + usize::from(resources.scratch.is_some()) + 1,
				}
			})
			.collect()
	}

	pub(crate) fn collect_exit(&mut self, pending: &CudaPending<'context>, work: TransferWork<'_>, destination: &mut [u8]) -> Result<()> {
		self.ensure_healthy()?;
		let ResolvedTransferEndpoint::Device(source_location) = work.source else {
			unreachable!()
		};
		debug_assert!(pending.task == work.task && pending.class == WorkClass::ExitTransfer && pending.terminal && pending.device == source_location.device && pending.queue == work.submission.queue && pending.completion == work.submission.completion && matches!(work.destination, ResolvedTransferEndpoint::External) && destination.len() == bytes_to_usize(work.bytes.get()));
		let device = self.device_mut(pending.device);
		let source = &device.egress[&work.task];
		debug_assert_eq!(source.len(), destination.len());
		destination.copy_from_slice(source);
		Ok(())
	}

	pub(crate) fn destroy(self) -> Result<()> {
		self.ensure_healthy()?;
		destroy_devices(self.devices)
	}

	fn submit_admission(&mut self, arenas: &impl CudaArenaLookup<'context>, planned: crate::PlannedSubmission, work: InitAdmissionWork<'_>) -> Result<(PendingSubmission<'context>, PendingAction)> {
		debug_assert_eq!(planned.device, work.destination.device);
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
				&& device.staging.len() >= bytes
		);
		device.staging.as_mut_slice()[..bytes].copy_from_slice(work.image);
		let event = take_event(&mut device.completions, work.task, planned.slots.completion);
		let stream = queue(&device.queues, planned.slots.queue);
		// SAFETY: the arena and staging ranges were bounds-checked above. The
		// pending token remains owned until polling reports a terminal state.
		let native = unsafe {
			stream.copy_h2d(
				&arena.buffer,
				offset_to_usize(work.destination.arena_offset.get()),
				&device.staging,
				0,
				bytes,
				event,
			)
		}?;
		Ok((
			PendingSubmission::Event(erase_operation_lifetime(native)),
			PendingAction::None,
		))
	}

	fn submit_calculation(&mut self, arenas: &impl CudaArenaLookup<'context>, planned: crate::PlannedSubmission, work: &CalculationWork) -> Result<(PendingSubmission<'context>, PendingAction)> {
		debug_assert_eq!(planned.device, work.device);
		let device = self.device_mut(planned.device);
		let artifact = &device.artifacts[&work.artifact];
		let invocation = device
			.invocations
			.get_mut(&planned.slots.completion)
			.unwrap();
		fill_invocation(invocation, arenas, work, &artifact.abi)?;
		let stream = queue(&device.queues, planned.slots.queue);
		let grid = grid_size(artifact.abi.elements.get(), artifact.abi.workgroup_lanes)?;
		let config = LaunchConfig::new(
			recipe_cuda::Dim3::new(grid, 1, 1)?,
			recipe_cuda::Dim3::new(artifact.abi.workgroup_lanes, 1, 1)?,
		);
		// SAFETY: immutable-plan validation checked the cubin ABI and launch
		// shape; `fill_invocation` checked every argument and arena range.
		unsafe {
			invocation.enqueue(
				stream,
				&artifact.function,
				config,
				artifact.abi.arguments.len(),
			)
		}?;
		Ok((PendingSubmission::Stream, PendingAction::None))
	}

	fn submit_internal_transfer(&mut self, arenas: &impl CudaArenaLookup<'context>, planned: crate::PlannedSubmission, work: &TransferWork) -> Result<(PendingSubmission<'context>, PendingAction)> {
		let (source, destination) = device_endpoints(work);
		debug_assert!(source.device == destination.device && source.device == planned.device);
		let source_arena = checked_arena(arenas, source, work.bytes.get());
		let destination_arena = checked_arena(arenas, destination, work.bytes.get());
		let device = self.device_mut(planned.device);
		let stream = queue(&device.queues, planned.slots.queue);
		// SAFETY: both ranges were bounds-checked and belong to the stream's
		// context. The pending token retains their lifetime through completion.
		unsafe {
			stream.enqueue_d2d(
				&destination_arena.buffer,
				offset_to_usize(destination.arena_offset.get()),
				&source_arena.buffer,
				offset_to_usize(source.arena_offset.get()),
				bytes_to_usize(work.bytes.get()),
			)
		}?;
		Ok((PendingSubmission::Stream, PendingAction::None))
	}

	fn submit_metric(&mut self, arenas: &impl CudaArenaLookup<'context>, planned: crate::PlannedSubmission, work: MetricWork) -> Result<(PendingSubmission<'context>, PendingAction)> {
		debug_assert!(work.value.bytes.get() == 4 && work.value.device == planned.device);
		let arena = checked_arena(arenas, work.value, 4);
		let device = self.device_mut(planned.device);
		let metric_buffer = device.metric_buffers.get_mut(&work.task).unwrap();
		let stream = queue(&device.queues, planned.slots.queue);
		// SAFETY: the four-byte source range and pinned destination were checked
		// above. The pending token remains live until the event is terminal.
		unsafe {
			stream.enqueue_d2h(
				metric_buffer,
				0,
				&arena.buffer,
				offset_to_usize(work.value.arena_offset.get()),
				4,
			)
		}?;
		Ok((PendingSubmission::Stream, PendingAction::Metric {
			dtype: work.value.dtype,
		}))
	}

	fn submit_exit_transfer(&mut self, arenas: &impl CudaArenaLookup<'context>, planned: crate::PlannedSubmission, work: &TransferWork) -> Result<(PendingSubmission<'context>, PendingAction)> {
		let ResolvedTransferEndpoint::Device(source) = work.source else {
			unreachable!("finalized exit transfer has a device source");
		};
		debug_assert!(matches!(work.destination, ResolvedTransferEndpoint::External) && source.device == planned.device);
		let source_arena = checked_arena(arenas, source, work.bytes.get());
		let device = self.device_mut(planned.device);
		let bytes = bytes_to_usize(work.bytes.get());
		debug_assert!(device.staging.len() >= bytes && device.egress.contains_key(&work.task));
		let event = take_event(&mut device.completions, work.task, planned.slots.completion);
		let stream = queue(&device.queues, planned.slots.queue);
		// SAFETY: the finalized source range and pre-realized pinned destination
		// were bounds-checked. Pending ownership lasts through terminal polling.
		let native = unsafe {
			stream.copy_d2h(
				&mut device.staging,
				0,
				&source_arena.buffer,
				offset_to_usize(source.arena_offset.get()),
				bytes,
				event,
			)
		}?;
		Ok((
			PendingSubmission::Event(erase_operation_lifetime(native)),
			PendingAction::Egress { bytes },
		))
	}

	fn finish_pending(&mut self, pending: &mut CudaPending<'context>) -> Result<BackendPoll> {
		let submission = match core::mem::replace(&mut pending.native, NativePendingState::Terminal) {
			NativePendingState::Active(native) => native,
			NativePendingState::Ready | NativePendingState::Terminal => unreachable!(),
		};
		if let PendingSubmission::Event(native) = submission {
			let event = match native.wait(Duration::ZERO)? {
				WaitOutcome::Complete(event) => event,
				WaitOutcome::TimedOut(native) => {
					pending.native = NativePendingState::Active(PendingSubmission::Event(native));
					return Ok(BackendPoll::Pending);
				}
			};
			let device = self.device_mut(pending.device);
			let slot = device.completions.get_mut(&pending.completion).unwrap();
			let prior = core::mem::replace(slot, CompletionEvent::Available(event));
			debug_assert!(completion_owned(&prior, pending.task));
		}
		let device = self.device_mut(pending.device);
		let metric = finish_action(device, pending.task, pending.action)?;
		pending.terminal = true;
		Ok(BackendPoll::Complete { metric })
	}

	pub(crate) fn rearm_pending(&mut self, pending: &mut CudaPending<'context>) -> Result<()> {
		self.ensure_healthy()?;
		debug_assert!(pending.phase == RunPhase::Loop && pending.terminal && matches!(pending.native, NativePendingState::Terminal),);
		pending.native = NativePendingState::Ready;
		pending.action = PendingAction::None;
		pending.terminal = false;
		Ok(())
	}

	pub(crate) fn prepare_loop_pending(&mut self, pending: &mut CudaPending<'context>) -> Result<()> {
		self.ensure_healthy()?;
		debug_assert_eq!(pending.phase, RunPhase::Loop);
		match pending.loop_submission_action() {
			Some(LoopSubmissionAction::UsePrepared) => Ok(()),
			Some(LoopSubmissionAction::Rearm) => self.rearm_pending(pending),
			None => unreachable!(),
		}
	}

	pub(crate) fn recycle_pending(&mut self, pending: CudaPending<'context>) -> Result<()> {
		self.ensure_healthy()?;
		let prepared = self.prepared_tasks.remove(&pending.task);
		debug_assert!(pending.terminal && matches!(pending.native, NativePendingState::Terminal) && prepared,);
		Ok(())
	}

	pub(crate) fn available_bytes(&self, device: DeviceId) -> Result<ByteCount> {
		let resources = &self.devices[&device];
		let info = resources.context.memory_info()?;
		let bytes = u64::try_from(info.free_bytes).map_err(|_| {
			Error::ArenaMismatch {
				device,
				detail: "CUDA free-memory counter does not fit Recipe byte units",
			}
		})?;
		Ok(ByteCount::new(bytes))
	}

	pub(crate) fn validate_handoff(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<()> {
		self.ensure_healthy()?;
		debug_assert!(self.prepared_tasks.is_empty());
		for (device, resources) in &self.devices {
			let finalized = bundle.init_image(*device).map(InitImageContract::from);
			debug_assert_eq!(resources.admission, finalized);
		}
		let runtime_artifacts = self.plan.runtime_artifacts().cloned().collect();
		let devices = self.devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts, devices, tasks)?;
		let contracts = task_contracts(bundle, Some(tasks));
		self.plan = plan;
		self.contracts = contracts;
		Ok(())
	}

	fn device_mut(&mut self, device: DeviceId) -> &mut DeviceResources<'context> { self.devices.get_mut(&device).unwrap() }

	pub(crate) fn ensure_healthy(&self) -> Result<()> {
		match self.poisoned {
			true => Err(Error::BackendPoisoned { backend: "CUDA" }),
			false => Ok(()),
		}
	}

	fn poison<T>(&mut self, error: Error) -> Result<T> {
		self.poisoned = true;
		Err(error)
	}
}

impl<'context> CudaPreparedResources<'context> {
	pub(crate) fn realize(draft: &DraftPlan, runtime_artifacts: Vec<crate::RuntimeArtifact>, reservations: &ReservationLedger, bindings: Vec<CudaBinding<'context>>, tasks: &BTreeSet<TaskId>) -> Result<Self> {
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
		for (device, binding) in binding_by_device {
			validate_binding(&binding)?;
			let resources = realize_device(
				&draft.resources,
				reservations,
				&scoped_tasks,
				&value_devices,
				&draft.init_images,
				&runtime_by_id,
				&binding,
			)?;
			devices.insert(device, resources);
		}
		Ok(Self {
			handoff: CudaPreparedHandoff::Candidate(runtime_by_id.into_values().collect()),
			devices,
		})
	}

	pub(crate) fn bind(self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<CudaResources<'context>> {
		let Self { handoff, devices } = self;
		let (plan, contracts) = match handoff {
			CudaPreparedHandoff::Finalized {
				bundle: prepared_bundle,
				tasks: prepared_tasks,
				plan,
				contracts,
			} => {
				debug_assert!(prepared_bundle == bundle.identity() && prepared_tasks == *tasks);
				Ok((plan, contracts))
			}
			CudaPreparedHandoff::Candidate(_) => {
				Err(Error::BackendState {
					backend: "CUDA",
					detail: "candidate resources were not validated for finalized handoff",
				})
			}
		}?;
		Ok(CudaResources {
			plan,
			contracts,
			prepared_tasks: BTreeSet::new(),
			devices,
			poisoned: false,
		})
	}

	pub(crate) fn bind_candidate(self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<CudaResources<'context>> {
		let Self { handoff, devices } = self;
		let runtime_artifacts = match handoff {
			CudaPreparedHandoff::Candidate(runtime_artifacts) => runtime_artifacts,
			CudaPreparedHandoff::Finalized { .. } => {
				return Err(Error::BackendState {
					backend: "CUDA",
					detail: "finalized CUDA resources cannot be rebound as a warm candidate",
				});
			}
		};
		for (device, resources) in &devices {
			let warm = bundle.init_image(*device).map(InitImageContract::from);
			debug_assert_eq!(resources.admission, warm);
		}
		let planned_devices = devices.keys().copied().collect();
		let plan = ExecutionPlan::validate_partition(bundle, runtime_artifacts, planned_devices, tasks)?;
		let contracts = task_contracts(bundle, Some(tasks));
		Ok(CudaResources {
			plan,
			contracts,
			prepared_tasks: BTreeSet::new(),
			devices,
			poisoned: false,
		})
	}

	#[allow(
		dead_code,
		reason = "retained for direct finalized prepared-resource handoff; production currently validates warmed resources"
	)]
	pub(crate) fn validate_handoff(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>) -> Result<()> {
		let runtime_artifacts = match &self.handoff {
			CudaPreparedHandoff::Candidate(runtime_artifacts) => runtime_artifacts,
			CudaPreparedHandoff::Finalized { .. } => {
				return Err(Error::BackendState {
					backend: "CUDA",
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
		let contracts = task_contracts(bundle, Some(tasks));
		self.handoff = CudaPreparedHandoff::Finalized {
			bundle: bundle.identity(),
			tasks: tasks.clone(),
			plan,
			contracts,
		};
		Ok(())
	}

	pub(crate) fn destroy(self) -> Result<()> { destroy_devices(self.devices) }
}

impl<'context> Backend for CudaBackend<'context> {
	type Arena = CudaArena<'context>;
	type Error = Error;
	type Pending = CudaPending<'context>;
	type Resource = CudaResources<'context>;

	const MAX_NON_POLL_PHYSICAL_CALLS: usize = 1;

	fn bind_resources(&mut self, bundle: &FinalizedBundle, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<Self::Resource, Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::BindResources)?;
		let prior = core::mem::replace(&mut self.state, CudaBackendState::Bound);
		match prior {
			CudaBackendState::Ready {
				bindings,
				artifacts,
			} => {
				let plan = ExecutionPlan::validate(bundle, artifacts)?;
				CudaResources::realize(bundle, plan, bindings, None)
			}
			CudaBackendState::Prepared(resources) => {
				let tasks = bundle.tasks().iter().map(|task| task.id).collect();
				resources.bind(bundle, &tasks)
			}
			CudaBackendState::Warmed(mut resources) => {
				let tasks = bundle.tasks().iter().map(|task| task.id).collect();
				resources.validate_handoff(bundle, &tasks)?;
				Ok(resources)
			}
			CudaBackendState::Bound => {
				Err(Error::BackendState {
					backend: "CUDA",
					detail: "resources may be bound only once",
				})
			}
		}
	}

	fn prepare_pending(&mut self, resource: &mut Self::Resource, request: PendingRequest, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<Self::Pending, Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::PreparePending {
			task: request.task,
		})?;
		resource.prepare_pending(request)
	}

	fn allocate_arena(&mut self, resource: &mut Self::Resource, layout: &ArenaLayout, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<Self::Arena, Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::AllocateArena {
			device: layout.device,
			bytes: layout.size,
		})?;
		resource.allocate_arena(layout)
	}

	fn supports_loop_repetition(&self) -> bool { true }

	fn supports_same_queue_pipelining(&self, _resource: &Self::Resource, _task: TaskId) -> bool { true }

	fn submit(&mut self, resource: &mut Self::Resource, arenas: ArenaSet<'_, Self::Arena>, pending: &mut Self::Pending, work: BackendWork<'_>, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(physical_calls, crate::accounting::submission_call(&work))?;
		resource.submit(&arenas, pending, work)
	}

	fn submit_loop_iteration(&mut self, resource: &mut Self::Resource, arenas: ArenaSet<'_, Self::Arena>, pending: &mut Self::Pending, _iteration: LoopIteration, work: BackendWork<'_>, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(physical_calls, crate::accounting::submission_call(&work))?;
		resource.prepare_loop_pending(pending)?;
		resource.submit(&arenas, pending, work)
	}

	fn poll(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<BackendPoll, Self::Error> {
		let task = pending.task();
		match resource.poll(pending) {
			Ok(status) => {
				let physical_status = match status {
					BackendPoll::Pending => PhysicalPollStatus::Pending,
					BackendPoll::Complete { .. } => PhysicalPollStatus::Complete,
				};
				crate::accounting::record(
					physical_calls,
					crate::accounting::completion_poll_call(task, physical_status),
				)?;
				Ok(status)
			}
			Err(error) => {
				crate::accounting::record(
					physical_calls,
					crate::accounting::completion_poll_call(task, PhysicalPollStatus::Failed),
				)?;
				Err(error)
			}
		}
	}

	fn collect_exit(&mut self, resource: &mut Self::Resource, arenas: ArenaSet<'_, Self::Arena>, pending: &mut Self::Pending, work: TransferWork<'_>, destination: &mut [u8], physical_calls: &mut PhysicalCallBatch) -> std::result::Result<(), Self::Error> {
		consume_context(arenas);
		crate::accounting::record(physical_calls, PhysicalCall::CollectExit {
			task: work.task,
			bytes: work.bytes,
		})?;
		resource.collect_exit(pending, work, destination)
	}

	fn release_arena(&mut self, resource: &mut Self::Resource, device: DeviceId, arena: Self::Arena, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::ReleaseArena { device })?;
		resource.ensure_healthy()?;
		debug_assert_eq!(arena.device(), device);
		arena.release()
	}

	fn destroy_resources(&mut self, resource: Self::Resource, physical_calls: &mut PhysicalCallBatch) -> std::result::Result<(), Self::Error> {
		crate::accounting::record(physical_calls, PhysicalCall::DestroyResources)?;
		resource.destroy()
	}
}

#[derive(Clone, Copy, Debug)]
enum PendingAction {
	None,
	Metric { dtype: recipe_core::DType },
	Egress { bytes: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopSubmissionAction {
	UsePrepared,
	Rearm,
}

pub struct CudaPending<'context> {
	task: TaskId,
	phase: RunPhase,
	class: WorkClass,
	device: DeviceId,
	queue: QueueSlotId,
	completion: CompletionSlotId,
	native: NativePendingState<'context>,
	action: PendingAction,
	terminal: bool,
}

impl<'context> CudaPending<'context> {
	fn ready(request: PendingRequest, planned: crate::PlannedSubmission) -> Self {
		Self {
			task: request.task,
			phase: request.phase,
			class: request.class,
			device: planned.device,
			queue: planned.slots.queue,
			completion: planned.slots.completion,
			native: NativePendingState::Ready,
			action: PendingAction::None,
			terminal: false,
		}
	}

	fn validate_ready(&self, work: &BackendWork<'_>, planned: crate::PlannedSubmission) {
		let submission_matches = match work {
			BackendWork::InitAdmission(work) => work.submission == planned.slots,
			BackendWork::Calculation(work) => work.submission == planned.slots,
			BackendWork::InternalTransfer(work) | BackendWork::ExitTransfer(work) => work.submission == planned.slots,
			BackendWork::Metric(work) => work.submission == planned.slots,
		};
		debug_assert!(self.task == work.task() && self.class == work.class() && self.device == planned.device && self.queue == planned.slots.queue && self.completion == planned.slots.completion && submission_matches && matches!(self.native, NativePendingState::Ready) && !self.terminal);
	}

	fn activate(&mut self, native: PendingSubmission<'context>, action: PendingAction) {
		debug_assert!(matches!(self.native, NativePendingState::Ready));
		self.native = NativePendingState::Active(native);
		self.action = action;
	}

	fn loop_submission_action(&self) -> Option<LoopSubmissionAction> {
		match (&self.native, self.terminal) {
			(NativePendingState::Ready, false) => Some(LoopSubmissionAction::UsePrepared),
			(NativePendingState::Terminal, true) => Some(LoopSubmissionAction::Rearm),
			(NativePendingState::Ready | NativePendingState::Active(_) | NativePendingState::Terminal, _) => None,
		}
	}

	pub(crate) fn task(&self) -> TaskId { self.task }
}

impl fmt::Debug for CudaPending<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("CudaPending")
			.field("task", &self.task)
			.field("phase", &self.phase)
			.field("class", &self.class)
			.field("device", &self.device)
			.field("queue", &self.queue)
			.field("completion", &self.completion)
			.field("action", &self.action)
			.field("terminal", &self.terminal)
			.finish_non_exhaustive()
	}
}

impl Drop for CudaPending<'_> {
	fn drop(&mut self) {
		let native = core::mem::replace(&mut self.native, NativePendingState::Terminal);
		match (self.terminal, native) {
			(false, NativePendingState::Active(native)) => {
				std::mem::forget(native);
			}
			(false, NativePendingState::Ready) => {
				debug_assert!(!self.terminal);
			}
			(true, native) => drop(native),
			(false, NativePendingState::Terminal) => {
				debug_assert!(matches!(self.native, NativePendingState::Terminal));
			}
		}
	}
}

fn task_contracts(bundle: &FinalizedBundle, selected: Option<&BTreeSet<TaskId>>) -> BTreeMap<TaskId, TaskContract> {
	let mut contracts = BTreeMap::new();
	for task in bundle.tasks().iter().filter(|task| {
		match selected {
			Some(selected) => selected.contains(&task.id),
			None => true,
		}
	}) {
		let (class, submission, admission, route, lane_claims) = match &task.kind {
			TaskKind::Calculation(calculation) => {
				(
					WorkClass::Calculation,
					Some(calculation.submission),
					None,
					Vec::new(),
					Vec::new(),
				)
			}
			TaskKind::Metric(metric) => {
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
						debug_assert!(contract.image == destination.value && contract.bytes == transfer.bytes);
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
		let prior = contracts.insert(task.id, TaskContract {
			phase: task.phase,
			class,
			submission,
			admission,
			route,
			lane_claims,
		});
		debug_assert!(prior.is_none());
	}
	contracts
}

fn transfer_work_class(phase: RunPhase, source: recipe_core::TransferEndpoint, destination: recipe_core::TransferEndpoint) -> WorkClass {
	use recipe_core::TransferEndpoint::{Device, External};

	match (phase, source, destination) {
		(RunPhase::Init, External, Device { .. }) => WorkClass::InitAdmission,
		(RunPhase::Init | RunPhase::Loop, Device { .. }, Device { .. }) => WorkClass::InternalTransfer,
		(RunPhase::Exit, Device { .. }, Device { .. } | External) => WorkClass::ExitTransfer,
		(RunPhase::Init, External, External) | (RunPhase::Init, Device { .. }, External) | (RunPhase::Loop, External, External | Device { .. }) | (RunPhase::Loop, Device { .. }, External) | (RunPhase::Exit, External, External | Device { .. }) => unreachable!(),
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

fn validate_binding(binding: &CudaBinding<'_>) -> Result<()> {
	ensure(
		binding.context.device().uuid == binding.deployment.device_uuid && binding.context.device().compute_capability == binding.deployment.target,
		Error::ArtifactMismatch {
			artifact: ArtifactId::new(0),
			detail: format!(
				"CUDA context device {} does not match deployment {}",
				binding.context.device().uuid,
				binding.deployment.device_uuid
			),
		},
	)
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
			detail: "CUDA bridge requires a scheduler-enforced quota",
		},
	)
}

fn realize_device<'context>(resources: &ResourceManifest, reservations: &ReservationLedger, tasks: &[Task], value_devices: &BTreeMap<ValueId, DeviceId>, init_images: &[InitDataImage], runtime_artifacts: &BTreeMap<ArtifactId, crate::RuntimeArtifact>, binding: &CudaBinding<'context>) -> Result<DeviceResources<'context>> {
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
		.map(|slot| Ok((slot.id, Stream::create_nonblocking(binding.context)?)))
		.collect::<Result<BTreeMap<_, _>>>()?;
	let completions = resources
		.completions
		.iter()
		.filter(|slot| slot.device == device && completion_ids.contains(&slot.id))
		.map(|slot| {
			Ok((
				slot.id,
				CompletionEvent::Available(Event::create_completion(binding.context)?),
			))
		})
		.collect::<Result<BTreeMap<_, _>>>()?;
	let staging_bytes = resources
		.pinned_staging
		.iter()
		.find(|entry| entry.device == device)
		.unwrap()
		.bytes
		.get();
	let staging = PinnedHostBuffer::allocate(binding.context, bytes_to_usize(staging_bytes))?;
	let admission = InitImageContract::from(
		init_images
			.iter()
			.find(|manifest| manifest.device == device)
			.unwrap(),
	);
	debug_assert!(admission.bytes.get() <= staging_bytes);
	let scratch = match resources
		.scratch
		.iter()
		.find(|entry| entry.device == device)
		.map(|entry| entry.bytes.get())
	{
		Some(0) | None => None,
		Some(bytes) => {
			Some(DeviceBuffer::allocate(
				binding.context,
				bytes_to_usize(bytes),
			)?)
		}
	};
	let artifact_ids = tasks
		.iter()
		.filter_map(|task| {
			match &task.kind {
				TaskKind::Calculation(calculation) => (calculation.device == device).then_some(calculation.artifact),
				TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
			}
		})
		.collect::<BTreeSet<_>>();
	let mut bundles = BTreeMap::<ArtifactDigest, Vec<(ArtifactId, &crate::RuntimeArtifact)>>::new();
	for artifact_id in artifact_ids {
		let runtime = &runtime_artifacts[&artifact_id];
		let RuntimeArtifactKind::Cuda { identity } = &runtime.kind else {
			return Err(Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "CUDA device was assigned a non-CUDA artifact".to_owned(),
			});
		};
		validate_artifact_compatibility(&runtime.bytes, identity, identity, &binding.deployment).map_err(|error| {
			Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: error.to_string(),
			}
		})?;
		let sm = binding
			.deployment
			.target
			.major
			.checked_mul(10)
			.ok_or(Error::IntegerOverflow {
				field: "CUDA compute capability",
			})?;
		let sm = sm
			.checked_add(binding.deployment.target.minor)
			.ok_or(Error::IntegerOverflow {
				field: "CUDA compute capability",
			})?;
		let inspection = inspect_cubin(
			&runtime.bytes,
			u8_from_u32(sm, "CUDA compute capability")?,
			&runtime.abi.entry_symbol,
		)?;
		ensure(
			inspection.entry_symbol == runtime.abi.entry_symbol,
			Error::ArtifactMismatch {
				artifact: artifact_id,
				detail: "inspected cubin entry differs from immutable ABI".to_owned(),
			},
		)?;
		let digest = runtime.digest();
		if let Some((_, first)) = bundles.get(&digest).and_then(|entries| entries.first()) {
			ensure(
				first.bytes.as_ref() == runtime.bytes.as_ref(),
				Error::ArtifactMismatch {
					artifact: artifact_id,
					detail: "distinct cubin images produced the same content digest".to_owned(),
				},
			)?;
		}
		bundles
			.entry(digest)
			.or_default()
			.push((artifact_id, runtime));
	}

	// Runtime artifacts retain per-stage identity and ABI while multiple
	// entries may share one target cubin. Keep each module in a stable Box,
	// load it once per distinct image, and resolve all logical functions from
	// that shared module. `artifacts` is dropped before `modules` on unwind and
	// explicit teardown follows the same order.
	let mut modules = BTreeMap::new();
	let mut artifacts = BTreeMap::new();
	for (digest, entries) in bundles {
		let (_, first_runtime) = entries
			.first()
			.expect("a cubin bundle group is created only for an artifact");
		modules.insert(
			digest,
			Box::new(Module::load_cubin(binding.context, &first_runtime.bytes)?),
		);
		let module = modules
			.get(&digest)
			.expect("the just-loaded cubin module remains in the module table");
		for (artifact_id, runtime) in entries {
			let function = module.function(&runtime.abi.entry_symbol)?;
			// SAFETY: `module` lives in a stable Box owned by `modules`.
			// `artifacts` is dropped before `modules` during both unwind and
			// explicit teardown, and loaded functions never escape resources.
			let function = unsafe { core::mem::transmute::<Function<'_, 'context>, Function<'context, 'context>>(function) };
			artifacts.insert(artifact_id, LoadedArtifact {
				function,
				abi: runtime.abi.clone(),
			});
		}
	}

	let invocations = invocation_sizes(tasks, device, &artifacts)
		.into_iter()
		.map(|(slot, count)| (slot, ParameterBlock::new(count)))
		.collect();
	let metric_buffers = tasks
		.iter()
		.filter_map(|task| {
			match &task.kind {
				TaskKind::Metric(metric) => (value_devices[&metric.value] == device).then_some(task.id),
				TaskKind::Calculation(_) | TaskKind::Transfer(_) => None,
			}
		})
		.map(|task| Ok((task, PinnedHostBuffer::allocate(binding.context, 4)?)))
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
		context: binding.context,
		queues,
		completions,
		artifacts,
		modules,
		invocations,
		metric_buffers,
		staging,
		admission: Some(admission),
		egress,
		scratch,
	})
}

fn invocation_sizes(tasks: &[Task], device: DeviceId, artifacts: &BTreeMap<ArtifactId, LoadedArtifact<'_>>) -> BTreeMap<CompletionSlotId, usize> {
	let mut result = BTreeMap::new();
	for task in tasks {
		let TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		match calculation.device == device {
			true => {
				let count = artifacts[&calculation.artifact].abi.arguments.len();
				result.entry(calculation.submission.completion)
					.and_modify(|prior: &mut usize| *prior = (*prior).max(count))
					.or_insert(count);
			}
			false => continue,
		}
	}
	result
}

fn fill_invocation<'context>(invocation: &mut ParameterBlock, arenas: &impl CudaArenaLookup<'context>, work: &CalculationWork, abi: &recipe_kernel::KernelAbi) -> Result<()> {
	debug_assert!(invocation.len() >= abi.arguments.len());
	invocation.reset_keepalive();
	let mut locations = work.inputs.iter().chain(work.outputs.iter()).copied();
	let mut fault_consumed = false;
	for (index, argument) in abi.arguments.iter().enumerate() {
		let value = match argument {
			KernelArgument::Buffer { .. } => {
				let location = locations.next().unwrap();
				let arena = checked_arena(arenas, location, location.bytes.get());
				let base = arena.buffer.device_ptr()?;
				let pointer = base.checked_add(location.arena_offset.get()).unwrap();
				// The pointer is used only to build a temporary keepalive
				// reference for the driver call. Arena ownership outlives every
				// pending token.
				invocation.retain(&arena.buffer);
				pointer
			}
			KernelArgument::FaultFlag => {
				let location = work.fault_flag.unwrap();
				debug_assert!(!fault_consumed);
				fault_consumed = true;
				let arena = checked_arena(arenas, location, location.bytes.get());
				let base = arena.buffer.device_ptr()?;
				let pointer = base.checked_add(location.arena_offset.get()).unwrap();
				invocation.retain(&arena.buffer);
				pointer
			}
			KernelArgument::RunId => work.run.get(),
			KernelArgument::LoopIteration => work.iteration.index(),
			KernelArgument::ElementCount => abi.elements.get(),
		};
		invocation.set_value(index, value);
	}
	debug_assert!(locations.next().is_none() && fault_consumed == work.fault_flag.is_some());
	Ok(())
}

fn checked_arena<'arena, 'context>(arenas: &'arena impl CudaArenaLookup<'context>, location: ResolvedValueLocation, bytes: u64) -> &'arena CudaArena<'context> {
	let arena = arenas.arena(location.device).unwrap();
	debug_assert_eq!(arena.device, location.device);
	let end = location.arena_offset.get().checked_add(bytes).unwrap();
	let arena_bytes = u64::try_from(arena.buffer.len()).unwrap();
	debug_assert!(end <= arena_bytes);
	arena
}

fn device_endpoints(work: &TransferWork) -> (ResolvedValueLocation, ResolvedValueLocation) {
	match (work.source, work.destination) {
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => (source, destination),
		_ => unreachable!("finalized internal transfer has two device endpoints"),
	}
}

fn take_event<'context>(completions: &mut BTreeMap<CompletionSlotId, CompletionEvent<'context>>, task: TaskId, completion: CompletionSlotId) -> Event<'context> {
	let slot = completions.get_mut(&completion).unwrap();
	match core::mem::replace(slot, CompletionEvent::Active { task }) {
		CompletionEvent::Available(event) => event,
		CompletionEvent::Active { .. } => unreachable!(),
	}
}

fn validate_active(device: &DeviceResources<'_>, pending: &CudaPending<'_>) {
	if matches!(
		pending.native,
		NativePendingState::Active(PendingSubmission::Stream)
	) {
		debug_assert!(device.queues.contains_key(&pending.queue));
		return;
	}
	let state = &device.completions[&pending.completion];
	debug_assert!(completion_owned(state, pending.task));
}

fn completion_owned(state: &CompletionEvent<'_>, task: TaskId) -> bool {
	match state {
		CompletionEvent::Available(_) => false,
		CompletionEvent::Active { task: owner } => *owner == task,
	}
}

fn queue<'borrow, 'context>(queues: &'borrow BTreeMap<QueueSlotId, Stream<'context>>, slot: QueueSlotId) -> &'borrow Stream<'context> { &queues[&slot] }

fn finish_action(device: &mut DeviceResources<'_>, task: TaskId, action: PendingAction) -> Result<Option<MetricValue>> {
	match action {
		PendingAction::None => Ok(None),
		PendingAction::Metric { dtype } => {
			let buffer = &device.metric_buffers[&task];
			let bytes: [u8; 4] = buffer.as_slice()[..4].try_into().unwrap();
			match dtype {
				recipe_core::DType::F32 => Ok(Some(MetricValue::F32(f32::from_le_bytes(bytes)))),
				recipe_core::DType::I32 => Ok(Some(MetricValue::I32(i32::from_le_bytes(bytes)))),
			}
		}
		PendingAction::Egress { bytes } => {
			let output = device.egress.get_mut(&task).unwrap();
			debug_assert!(output.len() == bytes && device.staging.len() >= bytes);
			output.copy_from_slice(&device.staging.as_slice()[..bytes]);
			Ok(None)
		}
	}
}

fn grid_size(elements: u64, lanes: u32) -> Result<u32> {
	let lanes_u64 = u64::from(lanes);
	let numerator = elements
		.checked_add(lanes_u64.saturating_sub(1))
		.ok_or(Error::IntegerOverflow {
			field: "CUDA grid numerator",
		})?;
	match u32::try_from(numerator / lanes_u64) {
		Ok(value) => Ok(value),
		Err(..) => {
			Err(Error::IntegerOverflow {
				field: "CUDA grid dimension",
			})
		}
	}
}

fn bytes_to_usize(bytes: u64) -> usize { usize::try_from(bytes).unwrap() }

fn offset_to_usize(offset: u64) -> usize { bytes_to_usize(offset) }

fn erase_operation_lifetime<'operation, 'context>(pending: Pending<'operation, 'context>) -> Pending<'static, 'context> {
	type ErasedPending<'context> = Pending<'static, 'context>;
	// SAFETY: CUDA's token stores the completion event plus a phantom operation
	// borrow. `CudaResources` records active ownership, refuses arena/resource
	// teardown before terminal completion, and leaks an abandoned live token.
	let erased = unsafe { core::mem::transmute::<Pending<'operation, 'context>, ErasedPending<'context>>(pending) };
	debug_assert_eq!(
		core::mem::size_of_val(&erased),
		core::mem::size_of::<Pending<'static, 'context>>()
	);
	erased
}

fn destroy_devices(devices: BTreeMap<DeviceId, DeviceResources<'_>>) -> Result<()> {
	for (_, device) in devices {
		for stream in device.queues.values() {
			ensure(
				stream.poll_idle()? == CompletionStatus::Complete,
				Error::CudaContract("CUDA stream remained active during teardown"),
			)?;
		}
		debug_assert!(
			device.completions
				.values()
				.all(|event| matches!(event, CompletionEvent::Available(..)))
		);
		for (_, event) in device.completions {
			match event {
				CompletionEvent::Available(event) => event.destroy()?,
				CompletionEvent::Active { .. } => {}
			}
		}
		for (_, stream) in device.queues {
			stream.destroy()?;
		}
		for (_, artifact) in device.artifacts {
			drop(artifact);
		}
		for (_, module) in device.modules {
			module.unload().map_err(Error::from)?;
		}
		for (_, buffer) in device.metric_buffers {
			buffer.free()?;
		}
		device.staging.free()?;
		free_optional_buffer(device.scratch)?;
	}
	Ok(())
}

fn free_optional_buffer(buffer: Option<DeviceBuffer<'_>>) -> Result<()> {
	match buffer {
		Some(buffer) => buffer.free().map_err(Error::from),
		None => Ok(()),
	}
}

fn u8_from_u32(value: u32, field: &'static str) -> Result<u8> {
	match u8::try_from(value) {
		Ok(value) => Ok(value),
		Err(..) => Err(Error::IntegerOverflow { field }),
	}
}

fn ensure(valid: bool, error: Error) -> Result<()> {
	match valid {
		true => Ok(()),
		false => Err(error),
	}
}
