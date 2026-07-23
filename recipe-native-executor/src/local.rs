//! Heterogeneous local execution over host, CUDA, and HSA partitions.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;

use recipe_core::{
	ArenaLayout, ArtifactIdentity, BundleIdentity, ByteCount, CapacityLedger, CapacityLedgerEntry, DeviceId,
	DiscoveryProfile, FinalizedBundle, Property, PropertyProvenance, RealizationIdentity, RealizationProfile,
	ReservationLedger, ResolvedTransferEndpoint, RunPhase, SubmissionSlots, Task, TaskId, TaskKind, Topology,
	TransferEndpoint,
};
use recipe_executor::{
	ArenaSet, Backend, BackendPoll, BackendWork, PendingRequest, PhysicalCall, PhysicalCallBatch,
	PhysicalCallBatchOverflow, PhysicalPollStatus, TransferWork, WorkClass, sealed,
};
use recipe_host::{
	Arena as HostArena, HostArenaLookup, HostBackend, HostBackendConfig, HostPending, HostPreparedResources,
	HostResources,
};
use recipe_planner::PlannedCandidate;

use crate::candidate::{CandidateFailure, CandidateRealizationRequest, CandidateSessionFactory};
use crate::cuda::{CudaArena, CudaArenaLookup, CudaPending, CudaPreparedResources, CudaResources};
use crate::hsa::{HsaArena, HsaArenaLookup, HsaPending, HsaPreparedResources, HsaResources};
use crate::{CudaBackend, CudaBinding, Error, HsaBackend, HsaBinding, RuntimeArtifact};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocalDeviceClass {
	Host,
	Cuda,
	Hsa,
}

#[derive(Debug)]
pub enum LocalArena<'cuda, 'hsa> {
	Host(HostArena),
	Cuda(CudaArena<'cuda>),
	Hsa(HsaArena<'hsa>),
}

impl LocalArena<'_, '_> {
	#[must_use]
	pub const fn class(&self) -> LocalDeviceClass {
		match self {
			Self::Host(_) => LocalDeviceClass::Host,
			Self::Cuda(_) => LocalDeviceClass::Cuda,
			Self::Hsa(_) => LocalDeviceClass::Hsa,
		}
	}

	#[must_use]
	pub fn device(&self) -> DeviceId {
		match self {
			Self::Host(arena) => arena.device(),
			Self::Cuda(arena) => arena.device(),
			Self::Hsa(arena) => arena.device(),
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub enum LocalArenaRef<'arena, 'cuda, 'hsa> {
	Host(&'arena HostArena),
	Cuda(&'arena CudaArena<'cuda>),
	Hsa(&'arena HsaArena<'hsa>),
}

impl LocalArenaRef<'_, '_, '_> {
	#[must_use]
	pub const fn class(self) -> LocalDeviceClass {
		match self {
			Self::Host(_) => LocalDeviceClass::Host,
			Self::Cuda(_) => LocalDeviceClass::Cuda,
			Self::Hsa(_) => LocalDeviceClass::Hsa,
		}
	}

	#[must_use]
	pub fn device(self) -> DeviceId {
		match self {
			Self::Host(arena) => arena.device(),
			Self::Cuda(arena) => arena.device(),
			Self::Hsa(arena) => arena.device(),
		}
	}
}

/// Read-only access to all arenas during one cross-backend submission.
///
/// The view borrows the executor's immutable arena map. Constructing and
/// querying it performs no allocation.
#[derive(Debug)]
pub struct LocalArenaSet<'view, 'arenas, 'cuda, 'hsa> {
	arenas: &'view ArenaSet<'arenas, LocalArena<'cuda, 'hsa>>,
}

impl<'view, 'arenas, 'cuda, 'hsa> LocalArenaSet<'view, 'arenas, 'cuda, 'hsa> {
	#[must_use]
	pub fn get(&self, device: DeviceId) -> Option<LocalArenaRef<'_, 'cuda, 'hsa>> {
		match self.arenas.get(device) {
			Some(LocalArena::Host(arena)) => Some(LocalArenaRef::Host(arena)),
			Some(LocalArena::Cuda(arena)) => Some(LocalArenaRef::Cuda(arena)),
			Some(LocalArena::Hsa(arena)) => Some(LocalArenaRef::Hsa(arena)),
			None => None,
		}
	}

	pub fn iter(&self) -> impl ExactSizeIterator<Item = (DeviceId, LocalArenaRef<'_, 'cuda, 'hsa>)> {
		self.arenas.iter().map(|(device, arena)| {
			let arena = match arena {
				LocalArena::Host(arena) => LocalArenaRef::Host(arena),
				LocalArena::Cuda(arena) => LocalArenaRef::Cuda(arena),
				LocalArena::Hsa(arena) => LocalArenaRef::Hsa(arena),
			};
			(device, arena)
		})
	}
}

/// Pre-realized implementation of one-hop transfers crossing local backend
/// ownership boundaries.
///
/// `bind` and `prepare_pending` are the only methods permitted to allocate,
/// register memory, create queues, or grow storage. `submit` and `poll` must be
/// nonblocking and allocation-free. The composite passes only finalized,
/// planner-expanded one-hop device-to-device transfers to this interface.
pub trait CrossBackendTransfer<'cuda, 'hsa>: fmt::Debug {
	type Resource: fmt::Debug;
	type Pending: fmt::Debug;
	type Error: StdError + Send + Sync + 'static;

	fn bind(
		&mut self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error>;

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
	) -> Result<Self::Pending, Self::Error>;

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: LocalArenaSet<'_, '_, 'cuda, 'hsa>,
		pending: &mut Self::Pending,
		class: WorkClass,
		work: TransferWork<'_>,
	) -> Result<(), Self::Error>;

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
	) -> Result<BackendPoll, Self::Error>;

	fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error>;
}

/// Pre-final realization extension for cross-backend one-hop transfers.
///
/// Implementations must create every registration, staging allocation, queue,
/// and completion object in `realize_candidate`. `validate_handoff` may inspect
/// immutable finalized addresses but may not allocate or replace `resource`.
pub trait CandidateCrossBackendTransfer<'cuda, 'hsa>: CrossBackendTransfer<'cuda, 'hsa> + Clone {
	fn realize_candidate(
		&mut self,
		candidate: &PlannedCandidate,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error>;

	fn validate_handoff(
		&mut self,
		resource: &Self::Resource,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<(), Self::Error>;

	/// Returns one terminal warm-pass token to the exact pre-final resource
	/// that created it.
	fn recycle_candidate_pending(
		&mut self,
		resource: &mut Self::Resource,
		pending: Self::Pending,
	) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossBackendUnavailable {
	task: TaskId,
}

impl CrossBackendUnavailable {
	#[must_use]
	pub const fn task(self) -> TaskId {
		self.task
	}
}

impl fmt::Display for CrossBackendUnavailable {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"cross-backend transfer task {} has no pre-realized bridge",
			self.task
		)
	}
}

impl StdError for CrossBackendUnavailable {}

fn consume_context<T>(context: T) {
	drop(context);
}

/// Bridge policy for homogeneous deployments. It fails bind closed when a
/// finalized bundle contains any cross-backend transfer.
#[derive(Clone, Copy, Debug, Default)]
pub struct RejectCrossBackend;

impl CrossBackendTransfer<'_, '_> for RejectCrossBackend {
	type Error = CrossBackendUnavailable;
	type Pending = ();
	type Resource = ();

	fn bind(
		&mut self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error> {
		consume_context((bundle, devices));
		match tasks.iter().next().copied() {
			Some(task) => Err(CrossBackendUnavailable { task }),
			None => Ok(()),
		}
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
	) -> Result<Self::Pending, Self::Error> {
		consume_context(resource);
		Err(CrossBackendUnavailable { task: request.task })
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: LocalArenaSet<'_, '_, '_, '_>,
		pending: &mut Self::Pending,
		class: WorkClass,
		work: TransferWork<'_>,
	) -> Result<(), Self::Error> {
		consume_context((resource, arenas, pending, class));
		Err(CrossBackendUnavailable { task: work.task })
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
	) -> Result<BackendPoll, Self::Error> {
		consume_context((resource, pending));
		Err(CrossBackendUnavailable {
			task: TaskId::new(0),
		})
	}

	fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error> {
		let () = resource;
		Ok(())
	}
}

impl CandidateCrossBackendTransfer<'_, '_> for RejectCrossBackend {
	fn realize_candidate(
		&mut self,
		candidate: &PlannedCandidate,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error> {
		consume_context((candidate, devices));
		match tasks.iter().next().copied() {
			Some(task) => Err(CrossBackendUnavailable { task }),
			None => Ok(()),
		}
	}

	fn validate_handoff(
		&mut self,
		resource: &Self::Resource,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<(), Self::Error> {
		consume_context((resource, bundle, devices));
		match tasks.iter().next().copied() {
			Some(task) => Err(CrossBackendUnavailable { task }),
			None => Ok(()),
		}
	}

	fn recycle_candidate_pending(
		&mut self,
		resource: &mut Self::Resource,
		pending: Self::Pending,
	) -> Result<(), Self::Error> {
		consume_context((resource, pending));
		Err(CrossBackendUnavailable {
			task: TaskId::new(0),
		})
	}
}

#[derive(Debug)]
pub enum LocalError<BridgeError> {
	DuplicateDevice { device: DeviceId },
	MissingDevice { device: DeviceId },
	UnexpectedDevice { device: DeviceId },
	UnsupportedCalculation { task: TaskId },
	UnsupportedRoute { task: TaskId, detail: &'static str },
	TaskNotAssigned { task: TaskId },
	ArenaOwnerMismatch { device: DeviceId },
	PendingOwnerMismatch { task: TaskId },
	CapacityMismatch { device: DeviceId, detail: &'static str },
	BackendState(&'static str),
	PhysicalAccountingOverflow,
	Host(recipe_host::Error),
	Native(Error),
	Bridge(BridgeError),
}

impl<BridgeError: fmt::Display> fmt::Display for LocalError<BridgeError> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DuplicateDevice { device } => {
				write!(
					formatter,
					"local device {device} has more than one backend owner"
				)
			}
			Self::MissingDevice { device } => {
				write!(
					formatter,
					"finalized local device {device} has no backend owner"
				)
			}
			Self::UnexpectedDevice { device } => {
				write!(
					formatter,
					"local backend binding {device} has no finalized arena"
				)
			}
			Self::UnsupportedCalculation { task } => {
				write!(
					formatter,
					"calculation task {task} is not owned by a GPU backend"
				)
			}
			Self::UnsupportedRoute { task, detail } => {
				write!(
					formatter,
					"local transfer task {task} is unsupported: {detail}"
				)
			}
			Self::TaskNotAssigned { task } => {
				write!(
					formatter,
					"local task {task} has no immutable backend assignment"
				)
			}
			Self::ArenaOwnerMismatch { device } => {
				write!(
					formatter,
					"arena {device} differs from its immutable backend owner"
				)
			}
			Self::PendingOwnerMismatch { task } => {
				write!(
					formatter,
					"task {task} differs from its prepared backend token"
				)
			}
			Self::CapacityMismatch { device, detail } => {
				write!(
					formatter,
					"local capacity observation for device {device} is invalid: {detail}"
				)
			}
			Self::BackendState(detail) => write!(formatter, "local backend state is invalid: {detail}"),
			Self::PhysicalAccountingOverflow => {
				formatter.write_str("local backend exceeded fixed physical-call accounting")
			}
			Self::Host(error) => write!(formatter, "host partition failed: {error}"),
			Self::Native(error) => write!(formatter, "native partition failed: {error}"),
			Self::Bridge(error) => write!(formatter, "cross-backend bridge failed: {error}"),
		}
	}
}

impl<BridgeError> StdError for LocalError<BridgeError>
where
	BridgeError: StdError + 'static,
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Host(error) => Some(error),
			Self::Native(error) => Some(error),
			Self::Bridge(error) => Some(error),
			Self::DuplicateDevice { .. }
			| Self::MissingDevice { .. }
			| Self::UnexpectedDevice { .. }
			| Self::UnsupportedCalculation { .. }
			| Self::UnsupportedRoute { .. }
			| Self::TaskNotAssigned { .. }
			| Self::ArenaOwnerMismatch { .. }
			| Self::PendingOwnerMismatch { .. }
			| Self::CapacityMismatch { .. }
			| Self::BackendState(_)
			| Self::PhysicalAccountingOverflow => None,
		}
	}
}

#[derive(Debug)]
pub enum PreparedBridgeError<E> {
	State(&'static str),
	Bridge(E),
}

impl<E: fmt::Display> fmt::Display for PreparedBridgeError<E> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::State(detail) => write!(formatter, "prepared bridge state is invalid: {detail}"),
			Self::Bridge(error) => write!(formatter, "prepared bridge failed: {error}"),
		}
	}
}

impl<E> StdError for PreparedBridgeError<E>
where
	E: StdError + 'static,
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Bridge(error) => Some(error),
			Self::State(_) => None,
		}
	}
}

#[derive(Debug)]
enum PreparedBridgeResource<R> {
	Available(R),
	Consumed,
}

/// Cross-backend driver whose exact physical resource was created before
/// Finalize and can be handed to one backend bind exactly once.
#[derive(Debug)]
pub struct PreparedBridge<Bridge, Resource> {
	bridge: Bridge,
	resource: PreparedBridgeResource<Resource>,
	tasks: BTreeSet<TaskId>,
	devices: BTreeMap<DeviceId, LocalDeviceClass>,
	handoff_validated: bool,
}

impl<'cuda, 'hsa, Bridge> CrossBackendTransfer<'cuda, 'hsa> for PreparedBridge<Bridge, Bridge::Resource>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	type Error = PreparedBridgeError<Bridge::Error>;
	type Pending = Bridge::Pending;
	type Resource = Bridge::Resource;

	fn bind(
		&mut self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error> {
		match *tasks == self.tasks && *devices == self.devices {
			true => Ok(()),
			false => Err(PreparedBridgeError::State(
				"finalized bridge partition differs from its candidate",
			)),
		}?;
		match self.handoff_validated {
			true => Ok(()),
			false => Err(PreparedBridgeError::State(
				"prepared bridge handoff was not validated before finalized bind",
			)),
		}?;
		let prepared = match &self.resource {
			PreparedBridgeResource::Available(resource) => resource,
			PreparedBridgeResource::Consumed => {
				return Err(PreparedBridgeError::State(
					"prepared bridge resource was already consumed",
				));
			}
		};
		consume_context((prepared, bundle));
		let resource = match core::mem::replace(&mut self.resource, PreparedBridgeResource::Consumed) {
			PreparedBridgeResource::Available(resource) => resource,
			PreparedBridgeResource::Consumed => {
				return Err(PreparedBridgeError::State(
					"prepared bridge resource was already consumed",
				));
			}
		};
		Ok(resource)
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
	) -> Result<Self::Pending, Self::Error> {
		self.bridge
			.prepare_pending(resource, request)
			.map_err(PreparedBridgeError::Bridge)
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: LocalArenaSet<'_, '_, 'cuda, 'hsa>,
		pending: &mut Self::Pending,
		class: WorkClass,
		work: TransferWork<'_>,
	) -> Result<(), Self::Error> {
		self.bridge
			.submit(resource, arenas, pending, class, work)
			.map_err(PreparedBridgeError::Bridge)
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
	) -> Result<BackendPoll, Self::Error> {
		self.bridge
			.poll(resource, pending)
			.map_err(PreparedBridgeError::Bridge)
	}

	fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error> {
		self.bridge
			.destroy(resource)
			.map_err(PreparedBridgeError::Bridge)
	}
}

#[derive(Debug)]
pub struct LocalResources<'cuda, 'hsa, BridgeResource> {
	devices: BTreeMap<DeviceId, LocalDeviceClass>,
	tasks: BTreeMap<TaskId, TaskOwner>,
	host: Option<HostResources>,
	cuda: CudaResources<'cuda>,
	hsa: HsaResources<'hsa>,
	bridge: BridgeResource,
}

#[derive(Debug)]
pub enum LocalPending<'cuda, 'hsa, BridgePending> {
	Host(HostPending),
	Cuda(CudaPending<'cuda>),
	Hsa(HsaPending<'hsa>),
	Bridge {
		task: TaskId,
		pending: BridgePending,
	},
}

impl<BridgePending> LocalPending<'_, '_, BridgePending> {
	fn task(&self) -> TaskId {
		match self {
			Self::Host(pending) => pending.task(),
			Self::Cuda(pending) => pending.task(),
			Self::Hsa(pending) => pending.task(),
			Self::Bridge { task, .. } => *task,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskOwner {
	Host,
	Cuda,
	Hsa,
	Bridge,
}

#[derive(Clone, Debug)]
struct Partitions {
	devices: BTreeMap<DeviceId, LocalDeviceClass>,
	tasks: BTreeMap<TaskId, TaskOwner>,
	host: BTreeSet<TaskId>,
	cuda: BTreeSet<TaskId>,
	hsa: BTreeSet<TaskId>,
	bridge: BTreeSet<TaskId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StabilizationState {
	Realized,
	Warmed { pass: u32 },
	Observed { pass: u32 },
}

type PreparedLocalBackend<'cuda, 'hsa, Bridge> =
	LocalBackend<'cuda, 'hsa, PreparedBridge<Bridge, <Bridge as CrossBackendTransfer<'cuda, 'hsa>>::Resource>>;

type PartitionedArtifacts = (
	Vec<RuntimeArtifact>,
	Vec<RuntimeArtifact>,
	Vec<ArtifactIdentity>,
);

#[derive(Debug)]
enum LocalPreparedPhysical<'cuda, 'hsa, BridgeResource> {
	Candidate {
		host: Option<HostPreparedResources>,
		cuda: CudaPreparedResources<'cuda>,
		hsa: HsaPreparedResources<'hsa>,
		bridge: BridgeResource,
	},
	Warm {
		bundle: FinalizedBundle,
		resources: LocalResources<'cuda, 'hsa, BridgeResource>,
		arenas: BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>,
		tasks: Vec<WarmTask>,
		images: BTreeMap<DeviceId, Vec<u8>>,
		exits: BTreeMap<TaskId, Vec<u8>>,
	},
	Transition,
	Destroyed,
}

#[derive(Clone, Debug)]
struct WarmTask {
	id: TaskId,
	phase: RunPhase,
	window: recipe_core::ScheduleWindow,
	dependencies: Vec<TaskId>,
	work: WarmWork,
}

#[derive(Clone, Debug)]
enum WarmWork {
	Init {
		device: DeviceId,
		destination: recipe_core::ResolvedValueLocation,
		bytes: ByteCount,
		submission: SubmissionSlots,
	},
	Calculation {
		device: DeviceId,
		kernel_template: recipe_core::KernelTemplateId,
		artifact: recipe_core::ArtifactId,
		submission: SubmissionSlots,
		inputs: Vec<recipe_core::ResolvedValueLocation>,
		outputs: Vec<recipe_core::ResolvedValueLocation>,
		fault_flag: Option<recipe_core::ResolvedValueLocation>,
	},
	Transfer {
		class: WorkClass,
		source: ResolvedTransferEndpoint,
		destination: ResolvedTransferEndpoint,
		bytes: ByteCount,
		route: Vec<recipe_core::LinkId>,
		lane_claims: Vec<recipe_core::TransferLaneClaim>,
		submission: SubmissionSlots,
	},
	Metric {
		purpose: recipe_core::MetricPurpose,
		metric: recipe_core::MetricId,
		slot: recipe_core::MetricSlotId,
		value: recipe_core::ResolvedValueLocation,
	},
}

impl WarmWork {
	const fn class(&self) -> WorkClass {
		match self {
			Self::Init { .. } => WorkClass::InitAdmission,
			Self::Calculation { .. } => WorkClass::Calculation,
			Self::Transfer { class, .. } => *class,
			Self::Metric { .. } => WorkClass::Metric,
		}
	}

	const fn submission(&self) -> Option<SubmissionSlots> {
		match self {
			Self::Init { submission, .. }
			| Self::Calculation { submission, .. }
			| Self::Transfer { submission, .. } => Some(*submission),
			Self::Metric { .. } => None,
		}
	}

	const fn external_exit_bytes(&self) -> Option<ByteCount> {
		match self {
			Self::Transfer {
				class: WorkClass::ExitTransfer,
				destination: ResolvedTransferEndpoint::External,
				bytes,
				..
			} => Some(*bytes),
			Self::Init { .. }
			| Self::Calculation { .. }
			| Self::Transfer { .. }
			| Self::Metric { .. } => None,
		}
	}

	fn backend_work<'work, BridgeError>(
		&'work self,
		task: TaskId,
		run: recipe_core::RunId,
		images: &'work BTreeMap<DeviceId, Vec<u8>>,
	) -> Result<BackendWork<'work>, LocalError<BridgeError>> {
		match self {
			Self::Init {
				device,
				destination,
				bytes,
				submission,
			} => {
				let image = images.get(device).ok_or(LocalError::BackendState(
					"warm admission image is absent",
				))?;
				Ok(BackendWork::InitAdmission(recipe_executor::InitAdmissionWork {
					task,
					destination: *destination,
					bytes: *bytes,
					submission: *submission,
					image,
				}))
			}
			Self::Calculation {
				device,
				kernel_template,
				artifact,
				submission,
				inputs,
				outputs,
				fault_flag,
			} => Ok(BackendWork::Calculation(recipe_executor::CalculationWork {
				task,
				run,
				device: *device,
				kernel_template: *kernel_template,
				artifact: *artifact,
				submission: *submission,
				inputs,
				outputs,
				fault_flag: *fault_flag,
			})),
			Self::Transfer {
				class,
				source,
				destination,
				bytes,
				route,
				lane_claims,
				submission,
			} => {
				let transfer = TransferWork {
					task,
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
					WorkClass::InitAdmission | WorkClass::Calculation | WorkClass::Metric => {
						Err(LocalError::BackendState(
							"warm transfer has a non-transfer work class",
						))
					}
				}
			}
			Self::Metric {
				purpose,
				metric,
				slot,
				value,
			} => Ok(BackendWork::Metric(recipe_executor::MetricWork {
				task,
				purpose: *purpose,
				metric: *metric,
				slot: *slot,
				value: *value,
			})),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WarmTaskState {
	Remaining,
	Pending,
	Complete,
}

/// Physical local resources created from one pre-final candidate.
///
/// The session is intentionally consumed by [`Self::into_backend`]. Its child
/// backend states then permit exactly one finalized bind.
pub struct LocalPreparedSession<'cuda, 'hsa, Bridge, BridgeResource> {
	topology: Topology,
	discovery: DiscoveryProfile,
	candidate: PlannedCandidate,
	artifacts: Vec<ArtifactIdentity>,
	reservations: ReservationLedger,
	partitions: Partitions,
	bridge: Bridge,
	physical: LocalPreparedPhysical<'cuda, 'hsa, BridgeResource>,
	stabilization: StabilizationState,
}

impl<Bridge: fmt::Debug, BridgeResource: fmt::Debug> fmt::Debug
	for LocalPreparedSession<'_, '_, Bridge, BridgeResource>
{
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("LocalPreparedSession")
			.field("topology", &self.topology.identity)
			.field("discovery", &self.discovery.identity)
			.field("candidate", &self.candidate.draft.candidate)
			.field("artifact_count", &self.artifacts.len())
			.field("reservations", &self.reservations)
			.field("partitions", &self.partitions)
			.field("bridge", &self.bridge)
			.field("physical", &self.physical)
			.field("stabilization", &self.stabilization)
			.finish()
	}
}

/// Driver for the candidate's real maximum-concurrency warm trace and
/// post-trace capacity observation.
pub trait LocalCandidateStabilizer<'cuda, 'hsa, Bridge>: fmt::Debug
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	fn warm_maximum_concurrency(
		&mut self,
		session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>>;

	fn capacity_snapshot(
		&mut self,
		session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableLocalStabilizer;

impl<'cuda, 'hsa, Bridge> LocalCandidateStabilizer<'cuda, 'hsa, Bridge> for UnavailableLocalStabilizer
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	fn warm_maximum_concurrency(
		&mut self,
		session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> {
		consume_context((session, candidate, pass));
		Err(CandidateFailure::PreFinalRealizationUnavailable {
			detail: "native maximum-concurrency warm execution is not implemented",
		})
	}

	fn capacity_snapshot(
		&mut self,
		session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>> {
		consume_context((session, topology, discovery));
		Err(CandidateFailure::PreFinalRealizationUnavailable {
			detail: "post-realization native capacity observation is not implemented",
		})
	}
}

/// Production stabilizer over the exact resources owned by a
/// [`LocalPreparedSession`].
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeLocalStabilizer;

impl<'cuda, 'hsa, Bridge> LocalCandidateStabilizer<'cuda, 'hsa, Bridge> for NativeLocalStabilizer
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	fn warm_maximum_concurrency(
		&mut self,
		session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> {
		session.execute_warm_pass(candidate, pass)
	}

	fn capacity_snapshot(
		&mut self,
		session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>> {
		session.observe_capacity(topology, discovery)
	}
}

/// Concrete pre-final local physical resource factory.
#[derive(Debug)]
pub struct LocalCandidateFactory<'cuda, 'hsa, Bridge, Stabilizer = UnavailableLocalStabilizer> {
	host: Option<HostBackendConfig>,
	cuda_bindings: Vec<CudaBinding<'cuda>>,
	hsa_bindings: Vec<HsaBinding<'hsa>>,
	bridge: Bridge,
	stabilizer: Stabilizer,
}

impl<'cuda, 'hsa, Bridge, Stabilizer> LocalCandidateFactory<'cuda, 'hsa, Bridge, Stabilizer> {
	#[must_use]
	pub fn new(
		host: Option<HostBackendConfig>,
		cuda_bindings: Vec<CudaBinding<'cuda>>,
		hsa_bindings: Vec<HsaBinding<'hsa>>,
		bridge: Bridge,
		stabilizer: Stabilizer,
	) -> Self {
		Self {
			host,
			cuda_bindings,
			hsa_bindings,
			bridge,
			stabilizer,
		}
	}
}

impl<'cuda, 'hsa, Bridge> LocalCandidateFactory<'cuda, 'hsa, Bridge, UnavailableLocalStabilizer> {
	#[must_use]
	pub fn fail_closed(
		host: Option<HostBackendConfig>,
		cuda_bindings: Vec<CudaBinding<'cuda>>,
		hsa_bindings: Vec<HsaBinding<'hsa>>,
		bridge: Bridge,
	) -> Self {
		Self::new(
			host,
			cuda_bindings,
			hsa_bindings,
			bridge,
			UnavailableLocalStabilizer,
		)
	}
}

impl<'cuda, 'hsa, Bridge> LocalCandidateFactory<'cuda, 'hsa, Bridge, NativeLocalStabilizer> {
	/// Constructs the production factory that warms and measures the exact
	/// pre-final local resources.
	#[must_use]
	pub fn production(
		host: Option<HostBackendConfig>,
		cuda_bindings: Vec<CudaBinding<'cuda>>,
		hsa_bindings: Vec<HsaBinding<'hsa>>,
		bridge: Bridge,
	) -> Self {
		Self::new(
			host,
			cuda_bindings,
			hsa_bindings,
			bridge,
			NativeLocalStabilizer,
		)
	}
}

impl<'cuda, 'hsa, Bridge> LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	fn execute_warm_pass(
		&mut self,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> {
		match candidate == &self.candidate {
			true => Ok(()),
			false => {
				return Err(CandidateFailure::CandidateRejected {
					detail: "warm execution candidate differs from its realized resources".to_owned(),
				});
			}
		}?;
		self.activate_warm_resources()?;
		let (resources, arenas, tasks, images, exits) = match &mut self.physical {
			LocalPreparedPhysical::Warm {
				resources,
				arenas,
				tasks,
				images,
				exits,
				..
			} => (resources, arenas, tasks, images, exits),
			LocalPreparedPhysical::Candidate { .. }
			| LocalPreparedPhysical::Transition
			| LocalPreparedPhysical::Destroyed => {
				return Err(CandidateFailure::Fatal(LocalError::BackendState(
					"warm local resources are unavailable",
				)));
			}
		};
		if arenas.is_empty() {
			for layout in &candidate.arena_layouts {
				let arena = allocate_warm_arena(resources, layout).map_err(CandidateFailure::Fatal)?;
				match arenas.insert(layout.device, arena) {
					None => {}
					Some(duplicate) => {
						let device = duplicate.device();
						drop(duplicate);
						return Err(CandidateFailure::Fatal(LocalError::DuplicateDevice { device }));
					}
				}
			}
		}
		run_warm_trace(
			&mut self.bridge,
			resources,
			arenas,
			tasks,
			images,
			exits,
			pass,
		)
		.map_err(CandidateFailure::Fatal)
	}

	fn activate_warm_resources(&mut self) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> {
		match self.physical {
			LocalPreparedPhysical::Warm { .. } => return Ok(()),
			LocalPreparedPhysical::Candidate { .. } => {}
			LocalPreparedPhysical::Transition | LocalPreparedPhysical::Destroyed => {
				return Err(CandidateFailure::Fatal(LocalError::BackendState(
					"candidate resources cannot enter warm execution",
				)));
			}
		}
		let bundle = provisional_warm_bundle(
			&self.topology,
			&self.discovery,
			&self.candidate,
			&self.artifacts,
			&self.reservations,
		)
		.map_err(|detail| CandidateFailure::CandidateRejected { detail })?;
		let physical = core::mem::replace(&mut self.physical, LocalPreparedPhysical::Transition);
		let LocalPreparedPhysical::Candidate {
			host,
			cuda,
			hsa,
			bridge: bridge_resource,
		} = physical
		else {
			return Err(CandidateFailure::Fatal(LocalError::BackendState(
				"candidate resources changed during warm activation",
			)));
		};
		let host = match host {
			Some(resources) => match resources.bind_candidate(&bundle, &self.partitions.host) {
				Ok(resources) => Some(resources),
				Err(error) => {
					let failure = LocalError::Host(error);
					let failure = cleanup_prepared_native_host(hsa, cuda, None, failure);
					let failure = cleanup_bridge_resource(&mut self.bridge, bridge_resource, failure);
					self.physical = LocalPreparedPhysical::Destroyed;
					return Err(CandidateFailure::Fatal(failure));
				}
			},
			None => None,
		};
		let cuda = match cuda.bind_candidate(&bundle, &self.partitions.cuda) {
			Ok(resources) => resources,
			Err(error) => {
				let mut failure = LocalError::Native(error);
				failure = cleanup_error(
					failure,
					hsa.destroy().map_err(LocalError::Native),
				);
				failure = cleanup_warm_host(host, failure);
				failure = cleanup_bridge_resource(&mut self.bridge, bridge_resource, failure);
				self.physical = LocalPreparedPhysical::Destroyed;
				return Err(CandidateFailure::Fatal(failure));
			}
		};
		let hsa = match hsa.bind_candidate(&bundle, &self.partitions.hsa) {
			Ok(resources) => resources,
			Err(error) => {
				let mut failure = LocalError::Native(error);
				failure = cleanup_error(
					failure,
					cuda.destroy().map_err(LocalError::Native),
				);
				failure = cleanup_warm_host(host, failure);
				failure = cleanup_bridge_resource(&mut self.bridge, bridge_resource, failure);
				self.physical = LocalPreparedPhysical::Destroyed;
				return Err(CandidateFailure::Fatal(failure));
			}
		};
		let resources = LocalResources {
			devices: self.partitions.devices.clone(),
			tasks: self.partitions.tasks.clone(),
			host,
			cuda,
			hsa,
			bridge: bridge_resource,
		};
		let tasks = prepare_warm_tasks::<Bridge::Error>(&bundle)
			.map_err(CandidateFailure::Fatal)?;
		let images = prepare_warm_images::<Bridge::Error>(&bundle)
			.map_err(CandidateFailure::Fatal)?;
		let exits = prepare_warm_exits::<Bridge::Error>(&tasks)
			.map_err(CandidateFailure::Fatal)?;
		self.physical = LocalPreparedPhysical::Warm {
			bundle,
			resources,
			arenas: BTreeMap::new(),
			tasks,
			images,
			exits,
		};
		Ok(())
	}

	fn observe_capacity(
		&mut self,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>> {
		match topology == &self.topology && discovery == &self.discovery {
			true => Ok(()),
			false => {
				return Err(CandidateFailure::CandidateRejected {
					detail: "capacity observation profile differs from realized local hardware".to_owned(),
				});
			}
		}?;
		let (resources, arenas) = match &mut self.physical {
			LocalPreparedPhysical::Warm {
				resources, arenas, ..
			} => (resources, arenas),
			LocalPreparedPhysical::Candidate { .. }
			| LocalPreparedPhysical::Transition
			| LocalPreparedPhysical::Destroyed => {
				return Err(CandidateFailure::Fatal(LocalError::BackendState(
					"capacity observation requires a complete physical warm trace",
				)));
			}
		};
		let warm_arenas = core::mem::take(arenas);
		release_warm_arenas(warm_arenas)
			.map_err(CandidateFailure::Fatal)?;
		observe_capacity_ledger(topology, discovery, &self.reservations, resources)
			.map_err(CandidateFailure::Fatal)
	}

	pub fn into_backend(
		mut self,
		bundle: &FinalizedBundle,
	) -> Result<PreparedLocalBackend<'cuda, 'hsa, Bridge>, LocalError<Bridge::Error>> {
		let validation = self.validate_handoff(bundle);
		match validation {
			Ok(()) => {
				let physical =
					core::mem::replace(&mut self.physical, LocalPreparedPhysical::Transition);
				let LocalPreparedPhysical::Warm {
					resources,
					arenas,
					..
				} = physical
				else {
					return Err(LocalError::BackendState(
						"observed local session has no warmed physical resources",
					));
				};
				match arenas.is_empty() {
					true => Ok(()),
					false => Err(LocalError::BackendState(
						"warm candidate arenas were not released before final handoff",
					)),
				}?;
				let Self {
					partitions,
					bridge,
					..
				} = self;
				let LocalResources {
					host,
					cuda,
					hsa,
					bridge: bridge_resource,
					..
				} = resources;
				let declared_devices = partitions
					.devices
					.iter()
					.map(|(device, class)| (*device, *class))
					.collect();
				Ok(LocalBackend {
					host: host.map(HostBackend::from_warmed),
					cuda: CudaBackend::from_warmed(cuda),
					hsa: HsaBackend::from_warmed(hsa),
					bridge: PreparedBridge {
						bridge,
						resource: PreparedBridgeResource::Available(bridge_resource),
						tasks: partitions.bridge,
						devices: partitions.devices,
						handoff_validated: true,
					},
					declared_devices,
				})
			}
			Err(validation_error) => match self.destroy() {
				Ok(()) => Err(validation_error),
				Err(teardown_error) => Err(teardown_error),
			},
		}
	}

	fn validate_handoff(&mut self, bundle: &FinalizedBundle) -> Result<(), LocalError<Bridge::Error>> {
		match self.stabilization {
			StabilizationState::Observed { .. } => Ok(()),
			StabilizationState::Realized | StabilizationState::Warmed { .. } => Err(LocalError::BackendState(
				"candidate resources were not observed after their final warm pass",
			)),
		}?;
		validate_prepared_identity(&self.candidate, &self.artifacts, &self.reservations, bundle)?;
		let LocalPreparedPhysical::Warm {
			resources, arenas, ..
		} = &mut self.physical
		else {
			return Err(LocalError::BackendState(
				"final handoff requires one warmed local resource set",
			));
		};
		match arenas.is_empty() {
			true => Ok(()),
			false => Err(LocalError::BackendState(
				"capacity was observed before warm candidate arenas were released",
			)),
		}?;
		self.bridge
			.validate_handoff(
				&resources.bridge,
				bundle,
				&self.partitions.bridge,
				&self.partitions.devices,
			)
			.map_err(LocalError::Bridge)?;
		match &mut resources.host {
			Some(host) => host
				.validate_handoff(bundle, &self.partitions.host)
				.map_err(LocalError::Host),
			None => Ok(()),
		}?;
		resources
			.cuda
			.validate_handoff(bundle, &self.partitions.cuda)
			.map_err(LocalError::Native)?;
		resources
			.hsa
			.validate_handoff(bundle, &self.partitions.hsa)
			.map_err(LocalError::Native)
	}

	fn destroy(mut self) -> Result<(), LocalError<Bridge::Error>> {
		let physical = core::mem::replace(&mut self.physical, LocalPreparedPhysical::Transition);
		let Self {
			mut bridge,
			..
		} = self;
		match physical {
			LocalPreparedPhysical::Candidate {
				host,
				cuda,
				hsa,
				bridge: bridge_resource,
			} => {
				let mut first = bridge
					.destroy(bridge_resource)
					.map_err(LocalError::Bridge)
					.err();
				first = retain_first(first, hsa.destroy().map_err(LocalError::Native));
				first = retain_first(first, cuda.destroy().map_err(LocalError::Native));
				let host_result = match host {
					Some(host) => host.destroy().map_err(LocalError::Host),
					None => Ok(()),
				};
				first = retain_first(first, host_result);
				match first {
					Some(error) => Err(error),
					None => Ok(()),
				}
			}
			LocalPreparedPhysical::Warm {
				resources, arenas, ..
			} => {
				let mut first = release_warm_arenas(arenas).err();
				first = retain_first(first, destroy_warm_resources(&mut bridge, resources));
				match first {
					Some(error) => Err(error),
					None => Ok(()),
				}
			}
			LocalPreparedPhysical::Transition => Err(LocalError::BackendState(
				"local candidate resources are already in transition",
			)),
			LocalPreparedPhysical::Destroyed => Ok(()),
		}
	}
}

impl<'cuda, 'hsa, Bridge, Stabilizer> CandidateSessionFactory for LocalCandidateFactory<'cuda, 'hsa, Bridge, Stabilizer>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> + Clone,
	Stabilizer: LocalCandidateStabilizer<'cuda, 'hsa, Bridge>,
{
	type Error = LocalError<Bridge::Error>;
	type Session = LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>;

	fn realize_candidate(
		&mut self,
		request: CandidateRealizationRequest<'_>,
	) -> Result<Self::Session, CandidateFailure<Self::Error>> {
		request
			.validate()
			.map_err(|error| CandidateFailure::CandidateRejected {
				detail: error.to_string(),
			})?;
		let declared_devices = declared_devices(self.host.as_ref(), &self.cuda_bindings, &self.hsa_bindings);
		let partitions = classify_candidate::<Bridge::Error>(request.candidate, &declared_devices)
			.map_err(CandidateFailure::Fatal)?;
		let (cuda_artifacts, hsa_artifacts, identities) =
			partition_candidate_artifacts(request, &partitions).map_err(CandidateFailure::Fatal)?;

		let host = match self.host.clone() {
			Some(config) => Some(HostBackend::prepare_candidate(
				&request.candidate.draft,
				request.reservations,
				config,
				&partitions.host,
			)
			.map_err(LocalError::Host)
			.map_err(CandidateFailure::Fatal)?),
			None => None,
		};
		let cuda = match CudaPreparedResources::realize(
			&request.candidate.draft,
			cuda_artifacts,
			request.reservations,
			self.cuda_bindings.clone(),
			&partitions.cuda,
		) {
			Ok(resources) => resources,
			Err(error) => {
				let failure = cleanup_prepared_host(host, LocalError::Native(error));
				return Err(CandidateFailure::Fatal(failure));
			}
		};
		let hsa = match HsaPreparedResources::realize(
			&request.candidate.draft,
			hsa_artifacts,
			request.reservations,
			self.hsa_bindings.clone(),
			&partitions.hsa,
		) {
			Ok(resources) => resources,
			Err(error) => {
				let failure = cleanup_prepared_cuda_host(cuda, host, LocalError::Native(error));
				return Err(CandidateFailure::Fatal(failure));
			}
		};
		let mut bridge = self.bridge.clone();
		let bridge_resource =
			match bridge.realize_candidate(request.candidate, &partitions.bridge, &partitions.devices) {
				Ok(resource) => resource,
				Err(error) => {
					let failure = cleanup_prepared_native_host(hsa, cuda, host, LocalError::Bridge(error));
					return Err(CandidateFailure::Fatal(failure));
				}
			};
		Ok(LocalPreparedSession {
			topology: request.topology.clone(),
			discovery: request.discovery.clone(),
			candidate: request.candidate.clone(),
			artifacts: identities,
			reservations: request.reservations.clone(),
			partitions,
			bridge,
			physical: LocalPreparedPhysical::Candidate {
				host,
				cuda,
				hsa,
				bridge: bridge_resource,
			},
			stabilization: StabilizationState::Realized,
		})
	}

	fn warm_maximum_concurrency(
		&mut self,
		session: &mut Self::Session,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<Self::Error>> {
		let expected_pass = match session.stabilization {
			StabilizationState::Realized => Some(1),
			StabilizationState::Observed { pass } => pass.checked_add(1),
			StabilizationState::Warmed { .. } => None,
		};
		match expected_pass == Some(pass) && *candidate == session.candidate {
			true => Ok(()),
			false => Err(CandidateFailure::CandidateRejected {
				detail: "warm trace candidate or pass order differs from the realized local session".to_owned(),
			}),
		}?;
		self.stabilizer
			.warm_maximum_concurrency(session, candidate, pass)?;
		session.stabilization = StabilizationState::Warmed { pass };
		Ok(())
	}

	fn capacity_snapshot(
		&mut self,
		session: &mut Self::Session,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<Self::Error>> {
		match topology.identity == session.candidate.draft.topology
			&& discovery.identity == session.candidate.draft.discovery
		{
			true => Ok(()),
			false => Err(CandidateFailure::CandidateRejected {
				detail: "capacity snapshot topology or discovery differs from the local candidate".to_owned(),
			}),
		}?;
		let pass = match session.stabilization {
			StabilizationState::Warmed { pass } => pass,
			StabilizationState::Realized | StabilizationState::Observed { .. } => {
				return Err(CandidateFailure::CandidateRejected {
					detail: "capacity snapshot requires one new complete warm pass".to_owned(),
				});
			}
		};
		let snapshot = self
			.stabilizer
			.capacity_snapshot(session, topology, discovery)?;
		session.stabilization = StabilizationState::Observed { pass };
		Ok(snapshot)
	}

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error> {
		session.destroy()
	}
}

#[derive(Debug)]
pub struct LocalBackend<'cuda, 'hsa, Bridge> {
	host: Option<HostBackend>,
	cuda: CudaBackend<'cuda>,
	hsa: HsaBackend<'hsa>,
	bridge: Bridge,
	declared_devices: Vec<(DeviceId, LocalDeviceClass)>,
}

impl<'cuda, 'hsa, Bridge> LocalBackend<'cuda, 'hsa, Bridge> {
	#[must_use]
	pub fn new(
		host: Option<HostBackendConfig>,
		cuda_bindings: Vec<CudaBinding<'cuda>>,
		cuda_artifacts: Vec<RuntimeArtifact>,
		hsa_bindings: Vec<HsaBinding<'hsa>>,
		hsa_artifacts: Vec<RuntimeArtifact>,
		bridge: Bridge,
	) -> Self {
		let declared_devices = declared_devices(host.as_ref(), &cuda_bindings, &hsa_bindings);
		Self {
			host: host.map(HostBackend::new),
			cuda: CudaBackend::new(cuda_bindings, cuda_artifacts),
			hsa: HsaBackend::new(hsa_bindings, hsa_artifacts),
			bridge,
			declared_devices,
		}
	}
}

impl<Bridge> sealed::Sealed for LocalBackend<'_, '_, Bridge> {}

impl<'cuda, 'hsa, Bridge> Backend for LocalBackend<'cuda, 'hsa, Bridge>
where
	Bridge: CrossBackendTransfer<'cuda, 'hsa>,
{
	type Arena = LocalArena<'cuda, 'hsa>;
	type Error = LocalError<Bridge::Error>;
	type Pending = LocalPending<'cuda, 'hsa, Bridge::Pending>;
	type Resource = LocalResources<'cuda, 'hsa, Bridge::Resource>;

	fn bind_resources(
		&mut self,
		bundle: &FinalizedBundle,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Resource, Self::Error> {
		record(physical_calls, PhysicalCall::BindResources)?;
		let partitions = classify(bundle, &self.declared_devices)?;
		let host = match &mut self.host {
			Some(host) => Some(host
				.bind_partition(bundle, &partitions.host)
				.map_err(LocalError::Host)?),
			None => match partitions.host.iter().next() {
				None => None,
				Some(_) => {
					return Err(LocalError::BackendState(
						"host tasks exist without a configured host partition",
					));
				}
			},
		};
		let cuda = self
			.cuda
			.bind_partition(bundle, &partitions.cuda)
			.map_err(LocalError::Native)?;
		let hsa = self
			.hsa
			.bind_partition(bundle, &partitions.hsa)
			.map_err(LocalError::Native)?;
		let bridge = self
			.bridge
			.bind(bundle, &partitions.bridge, &partitions.devices)
			.map_err(LocalError::Bridge)?;
		Ok(LocalResources {
			devices: partitions.devices,
			tasks: partitions.tasks,
			host,
			cuda,
			hsa,
			bridge,
		})
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Pending, Self::Error> {
		record(
			physical_calls,
			PhysicalCall::PreparePending { task: request.task },
		)?;
		match resource.tasks.get(&request.task).copied() {
			Some(TaskOwner::Host) => {
				let host_resource = resource.host.as_mut().ok_or(LocalError::BackendState(
					"host task has no bound host resources",
				))?;
				self.host
					.as_mut()
					.ok_or(LocalError::BackendState("host partition is absent"))?
					.prepare_partition(host_resource, request)
					.map(LocalPending::Host)
					.map_err(LocalError::Host)
			}
			Some(TaskOwner::Cuda) => resource
				.cuda
				.prepare_pending(request)
				.map(LocalPending::Cuda)
				.map_err(LocalError::Native),
			Some(TaskOwner::Hsa) => resource
				.hsa
				.prepare_pending(request)
				.map(LocalPending::Hsa)
				.map_err(LocalError::Native),
			Some(TaskOwner::Bridge) => self
				.bridge
				.prepare_pending(&mut resource.bridge, request)
				.map(|pending| LocalPending::Bridge {
					task: request.task,
					pending,
				})
				.map_err(LocalError::Bridge),
			None => Err(LocalError::TaskNotAssigned { task: request.task }),
		}
	}

	fn allocate_arena(
		&mut self,
		resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Arena, Self::Error> {
		record(
			physical_calls,
			PhysicalCall::AllocateArena {
				device: layout.device,
				bytes: layout.size,
			},
		)?;
		match resource.devices.get(&layout.device).copied() {
			Some(LocalDeviceClass::Host) => {
				let host_resource = resource.host.as_mut().ok_or(LocalError::BackendState(
					"host device has no bound host resources",
				))?;
				self.host
					.as_mut()
					.ok_or(LocalError::BackendState("host partition is absent"))?
					.allocate_partition(host_resource, layout)
					.map(LocalArena::Host)
					.map_err(LocalError::Host)
			}
			Some(LocalDeviceClass::Cuda) => resource
				.cuda
				.allocate_arena(layout)
				.map(LocalArena::Cuda)
				.map_err(LocalError::Native),
			Some(LocalDeviceClass::Hsa) => resource
				.hsa
				.allocate_arena(layout)
				.map(LocalArena::Hsa)
				.map_err(LocalError::Native),
			None => Err(LocalError::MissingDevice {
				device: layout.device,
			}),
		}
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		record(physical_calls, crate::accounting::submission_call(&work))?;
		let task = work.task();
		let lookup = ProjectedArenas { arenas: &arenas };
		match pending {
			LocalPending::Host(pending) => {
				ensure_owner(resource, task, TaskOwner::Host)?;
				let host_resource = resource.host.as_mut().ok_or(LocalError::BackendState(
					"host task has no bound host resources",
				))?;
				self.host
					.as_mut()
					.ok_or(LocalError::BackendState("host partition is absent"))?
					.submit_partition(host_resource, &lookup, pending, work)
					.map_err(LocalError::Host)
			}
			LocalPending::Cuda(pending) => {
				ensure_owner(resource, task, TaskOwner::Cuda)?;
				resource
					.cuda
					.submit(&lookup, pending, work)
					.map_err(LocalError::Native)
			}
			LocalPending::Hsa(pending) => {
				ensure_owner(resource, task, TaskOwner::Hsa)?;
				resource
					.hsa
					.submit(&lookup, pending, work)
					.map_err(LocalError::Native)
			}
			LocalPending::Bridge {
				task: pending_task,
				pending,
			} => {
				ensure_owner(resource, task, TaskOwner::Bridge)?;
				match *pending_task == task {
					true => Ok(()),
					false => Err(LocalError::PendingOwnerMismatch { task }),
				}?;
				let (class, transfer) = match work {
					BackendWork::InternalTransfer(work) => (WorkClass::InternalTransfer, work),
					BackendWork::ExitTransfer(work) => (WorkClass::ExitTransfer, work),
					BackendWork::InitAdmission(_) | BackendWork::Calculation(_) | BackendWork::Metric(_) => {
						return Err(LocalError::PendingOwnerMismatch { task });
					}
				};
				self.bridge
					.submit(
						&mut resource.bridge,
						LocalArenaSet { arenas: &arenas },
						pending,
						class,
						transfer,
					)
					.map_err(LocalError::Bridge)
			}
		}
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<BackendPoll, Self::Error> {
		let task = pending.task();
		let result = match pending {
			LocalPending::Host(pending) => {
				let host_resource = resource.host.as_mut().ok_or(LocalError::BackendState(
					"host task has no bound host resources",
				))?;
				self.host
					.as_mut()
					.ok_or(LocalError::BackendState("host partition is absent"))?
					.poll_partition(host_resource, pending)
					.map_err(LocalError::Host)
			}
			LocalPending::Cuda(pending) => resource.cuda.poll(pending).map_err(LocalError::Native),
			LocalPending::Hsa(pending) => resource
				.hsa
				.poll_pending(pending)
				.map_err(LocalError::Native),
			LocalPending::Bridge { pending, .. } => self
				.bridge
				.poll(&mut resource.bridge, pending)
				.map_err(LocalError::Bridge),
		};
		let status = match &result {
			Ok(BackendPoll::Pending) => PhysicalPollStatus::Pending,
			Ok(BackendPoll::Complete { .. }) => PhysicalPollStatus::Complete,
			Err(
				LocalError::DuplicateDevice { .. }
				| LocalError::MissingDevice { .. }
				| LocalError::UnexpectedDevice { .. }
				| LocalError::UnsupportedCalculation { .. }
				| LocalError::UnsupportedRoute { .. }
				| LocalError::TaskNotAssigned { .. }
				| LocalError::ArenaOwnerMismatch { .. }
				| LocalError::PendingOwnerMismatch { .. }
				| LocalError::CapacityMismatch { .. }
				| LocalError::BackendState(_)
				| LocalError::PhysicalAccountingOverflow
				| LocalError::Host(_)
				| LocalError::Native(_)
				| LocalError::Bridge(_),
			) => PhysicalPollStatus::Failed,
		};
		record(
			physical_calls,
			crate::accounting::completion_poll_call(task, status),
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
	) -> Result<(), Self::Error> {
		record(
			physical_calls,
			PhysicalCall::CollectExit {
				task: work.task,
				bytes: work.bytes,
			},
		)?;
		let lookup = ProjectedArenas { arenas: &arenas };
		match pending {
			LocalPending::Host(pending) => {
				let host_resource = resource.host.as_mut().ok_or(LocalError::BackendState(
					"host task has no bound host resources",
				))?;
				self.host
					.as_mut()
					.ok_or(LocalError::BackendState("host partition is absent"))?
					.collect_partition(host_resource, pending, work, destination)
					.map_err(LocalError::Host)
			}
			LocalPending::Cuda(pending) => resource
				.cuda
				.collect_exit(pending, work, destination)
				.map_err(LocalError::Native),
			LocalPending::Hsa(pending) => resource
				.hsa
				.collect_exit(&lookup, pending, work, destination)
				.map_err(LocalError::Native),
			LocalPending::Bridge { .. } => Err(LocalError::UnsupportedRoute {
				task: work.task,
				detail: "cross-backend transfers cannot have an external endpoint",
			}),
		}
	}

	fn release_arena(
		&mut self,
		resource: &mut Self::Resource,
		device: DeviceId,
		arena: Self::Arena,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		record(physical_calls, PhysicalCall::ReleaseArena { device })?;
		let expected = resource
			.devices
			.get(&device)
			.copied()
			.ok_or(LocalError::MissingDevice { device })?;
		match arena.device() == device && arena.class() == expected {
			true => Ok(()),
			false => Err(LocalError::ArenaOwnerMismatch { device }),
		}?;
		match arena {
			LocalArena::Host(arena) => {
				let host_resource = resource.host.as_mut().ok_or(LocalError::BackendState(
					"host device has no bound host resources",
				))?;
				self.host
					.as_mut()
					.ok_or(LocalError::BackendState("host partition is absent"))?
					.release_partition(host_resource, device, arena)
					.map_err(LocalError::Host)
			}
			LocalArena::Cuda(arena) => {
				resource.cuda.ensure_healthy().map_err(LocalError::Native)?;
				arena.release().map_err(LocalError::Native)
			}
			LocalArena::Hsa(arena) => {
				resource.hsa.ensure_healthy().map_err(LocalError::Native)?;
				arena.release().map_err(LocalError::Native)
			}
		}
	}

	fn destroy_resources(
		&mut self,
		resource: Self::Resource,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		record(physical_calls, PhysicalCall::DestroyResources)?;
		let LocalResources {
			host,
			cuda,
			hsa,
			bridge,
			..
		} = resource;
		let mut first = self
			.bridge
			.destroy(bridge)
			.map_err(LocalError::Bridge)
			.err();
		first = retain_first(first, hsa.destroy().map_err(LocalError::Native));
		first = retain_first(first, cuda.destroy().map_err(LocalError::Native));
		let host_result = match host {
			Some(host_resource) => match &mut self.host {
				Some(host) => host
					.destroy_partition(host_resource)
					.map_err(LocalError::Host),
				None => Err(LocalError::BackendState(
					"bound host resources outlived the host partition",
				)),
			},
			None => Ok(()),
		};
		first = retain_first(first, host_result);
		match first {
			Some(error) => Err(error),
			None => Ok(()),
		}
	}
}

#[derive(Debug)]
struct ProjectedArenas<'view, 'arenas, 'cuda, 'hsa> {
	arenas: &'view ArenaSet<'arenas, LocalArena<'cuda, 'hsa>>,
}

impl HostArenaLookup for ProjectedArenas<'_, '_, '_, '_> {
	fn host_arena(&self, device: DeviceId) -> Option<&HostArena> {
		match self.arenas.get(device) {
			Some(LocalArena::Host(arena)) => Some(arena),
			Some(LocalArena::Cuda(_) | LocalArena::Hsa(_)) | None => None,
		}
	}
}

impl<'cuda> CudaArenaLookup<'cuda> for ProjectedArenas<'_, '_, 'cuda, '_> {
	fn arena(&self, device: DeviceId) -> Option<&CudaArena<'cuda>> {
		match self.arenas.get(device) {
			Some(LocalArena::Cuda(arena)) => Some(arena),
			Some(LocalArena::Host(_) | LocalArena::Hsa(_)) | None => None,
		}
	}
}

impl<'hsa> HsaArenaLookup<'hsa> for ProjectedArenas<'_, '_, '_, 'hsa> {
	fn arena(&self, device: DeviceId) -> Option<&HsaArena<'hsa>> {
		match self.arenas.get(device) {
			Some(LocalArena::Hsa(arena)) => Some(arena),
			Some(LocalArena::Host(_) | LocalArena::Cuda(_)) | None => None,
		}
	}
}

fn declared_devices(
	host: Option<&HostBackendConfig>,
	cuda_bindings: &[CudaBinding<'_>],
	hsa_bindings: &[HsaBinding<'_>],
) -> Vec<(DeviceId, LocalDeviceClass)> {
	let mut declared = Vec::new();
	declared.extend(host.into_iter().flat_map(|config| {
		config.bindings()
			.iter()
			.map(|binding| (binding.device(), LocalDeviceClass::Host))
	}));
	declared.extend(
		cuda_bindings
			.iter()
			.map(|binding| (binding.device(), LocalDeviceClass::Cuda)),
	);
	declared.extend(
		hsa_bindings
			.iter()
			.map(|binding| (binding.device(), LocalDeviceClass::Hsa)),
	);
	declared
}

fn classify_candidate<BridgeError>(
	candidate: &PlannedCandidate,
	declared: &[(DeviceId, LocalDeviceClass)],
) -> Result<Partitions, LocalError<BridgeError>> {
	let devices = validate_device_owners(
		candidate
			.arena_layouts
			.iter()
			.map(|layout| layout.device)
			.collect(),
		declared,
	)?;
	let values = candidate
		.draft
		.values
		.iter()
		.map(|value| (value.id, value.device))
		.collect::<BTreeMap<_, _>>();
	let mut result = Partitions {
		devices,
		tasks: BTreeMap::new(),
		host: BTreeSet::new(),
		cuda: BTreeSet::new(),
		hsa: BTreeSet::new(),
		bridge: BTreeSet::new(),
	};
	for task in &candidate.draft.tasks {
		let owner = match &task.kind {
			TaskKind::Calculation(calculation) => match result.devices.get(&calculation.device) {
				Some(LocalDeviceClass::Cuda) => TaskOwner::Cuda,
				Some(LocalDeviceClass::Hsa) => TaskOwner::Hsa,
				Some(LocalDeviceClass::Host) | None => {
					return Err(LocalError::UnsupportedCalculation { task: task.id });
				}
			},
			TaskKind::Metric(metric) => {
				let device = values
					.get(&metric.value)
					.copied()
					.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
				class_owner(
					result.devices
						.get(&device)
						.copied()
						.ok_or(LocalError::MissingDevice { device })?,
				)
			}
			TaskKind::Transfer(transfer) => {
				match transfer.route.len() {
					0 | 1 => Ok(()),
					2.. => Err(LocalError::UnsupportedRoute {
						task: task.id,
						detail: "executor-visible transfers must be planner-expanded to one hop",
					}),
				}?;
				candidate_transfer_owner::<BridgeError>(
					task.id,
					transfer.source,
					transfer.destination,
					&result.devices,
				)?
			}
		};
		result.tasks.insert(task.id, owner);
		match owner {
			TaskOwner::Host => {
				result.host.insert(task.id);
			}
			TaskOwner::Cuda => {
				result.cuda.insert(task.id);
			}
			TaskOwner::Hsa => {
				result.hsa.insert(task.id);
			}
			TaskOwner::Bridge => {
				let route = match &task.kind {
					TaskKind::Transfer(transfer) => &transfer.route,
					TaskKind::Calculation(_) | TaskKind::Metric(_) => {
						return Err(LocalError::UnsupportedRoute {
							task: task.id,
							detail: "only transfers may use the cross-backend bridge",
						});
					}
				};
				match route.len() {
					1 => Ok(()),
					0 | 2.. => Err(LocalError::UnsupportedRoute {
						task: task.id,
						detail: "cross-backend transfers require exactly one candidate link",
					}),
				}?;
				result.bridge.insert(task.id);
			}
		}
	}
	validate_partition_slots(&candidate.draft.tasks, &result)?;
	Ok(result)
}

fn partition_candidate_artifacts<BridgeError>(
	request: CandidateRealizationRequest<'_>,
	partitions: &Partitions,
) -> Result<PartitionedArtifacts, LocalError<BridgeError>> {
	let mut cuda_ids = BTreeSet::new();
	let mut hsa_ids = BTreeSet::new();
	for task in &request.candidate.draft.tasks {
		let TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		match partitions.tasks.get(&task.id) {
			Some(TaskOwner::Cuda) => {
				cuda_ids.insert(calculation.artifact);
			}
			Some(TaskOwner::Hsa) => {
				hsa_ids.insert(calculation.artifact);
			}
			Some(TaskOwner::Host | TaskOwner::Bridge) | None => {
				return Err(LocalError::UnsupportedCalculation { task: task.id });
			}
		}
	}
	match cuda_ids.is_disjoint(&hsa_ids) {
		true => Ok(()),
		false => Err(LocalError::BackendState(
			"one runtime artifact is assigned to both native GPU backends",
		)),
	}?;

	let mut cuda_artifacts = Vec::new();
	let mut hsa_artifacts = Vec::new();
	let mut identities = Vec::with_capacity(request.artifacts.len());
	for artifact in request.artifacts {
		let id = artifact.identity().id;
		let destination = match (cuda_ids.remove(&id), hsa_ids.remove(&id)) {
			(true, false) => &mut cuda_artifacts,
			(false, true) => &mut hsa_artifacts,
			(false, false) | (true, true) => {
				return Err(LocalError::Native(Error::UnexpectedArtifact {
					artifact: id,
				}));
			}
		};
		destination.push(artifact.runtime().clone());
		identities.push(artifact.identity().clone());
	}
	match cuda_ids
		.iter()
		.next()
		.or_else(|| hsa_ids.iter().next())
		.copied()
	{
		Some(artifact) => Err(LocalError::Native(Error::MissingArtifact { artifact })),
		None => Ok((cuda_artifacts, hsa_artifacts, identities)),
	}
}

fn candidate_transfer_owner<BridgeError>(
	task: TaskId,
	source: TransferEndpoint,
	destination: TransferEndpoint,
	devices: &BTreeMap<DeviceId, LocalDeviceClass>,
) -> Result<TaskOwner, LocalError<BridgeError>> {
	match (source, destination) {
		(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
		| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => devices
			.get(&device)
			.copied()
			.map(class_owner)
			.ok_or(LocalError::MissingDevice { device }),
		(
			TransferEndpoint::Device { device: source, .. },
			TransferEndpoint::Device {
				device: destination,
				..
			},
		) => device_transfer_owner(source, destination, devices),
		(TransferEndpoint::External, TransferEndpoint::External) => Err(LocalError::UnsupportedRoute {
			task,
			detail: "external-to-external transfers have no local owner",
		}),
	}
}

fn validate_device_owners<BridgeError>(
	required: BTreeSet<DeviceId>,
	declared: &[(DeviceId, LocalDeviceClass)],
) -> Result<BTreeMap<DeviceId, LocalDeviceClass>, LocalError<BridgeError>> {
	let mut devices = BTreeMap::new();
	for (device, class) in declared {
		match devices.insert(*device, *class) {
			Some(_) => Err(LocalError::DuplicateDevice { device: *device }),
			None => Ok(()),
		}?;
	}
	for device in &required {
		match devices.get(device) {
			Some(_) => Ok(()),
			None => Err(LocalError::MissingDevice { device: *device }),
		}?;
	}
	for device in devices.keys() {
		match required.get(device) {
			Some(_) => Ok(()),
			None => Err(LocalError::UnexpectedDevice { device: *device }),
		}?;
	}
	Ok(devices)
}

fn validate_partition_slots<BridgeError>(
	tasks: &[Task],
	partitions: &Partitions,
) -> Result<(), LocalError<BridgeError>> {
	let mut queues = BTreeMap::new();
	let mut completions = BTreeMap::new();
	for task in tasks {
		let owner = partitions
			.tasks
			.get(&task.id)
			.copied()
			.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
		let Some(submission) = local_task_submission(task) else {
			continue;
		};
		validate_slot_owner(
			&mut queues,
			submission.queue,
			owner,
			"one queue slot is shared by different local backend owners",
		)?;
		validate_slot_owner(
			&mut completions,
			submission.completion,
			owner,
			"one completion slot is shared by different local backend owners",
		)?;
	}
	Ok(())
}

fn validate_slot_owner<Slot: Ord + Copy, BridgeError>(
	owners: &mut BTreeMap<Slot, TaskOwner>,
	slot: Slot,
	owner: TaskOwner,
	detail: &'static str,
) -> Result<(), LocalError<BridgeError>> {
	match owners.insert(slot, owner) {
		Some(prior) => match prior == owner {
			true => Ok(()),
			false => Err(LocalError::BackendState(detail)),
		},
		None => Ok(()),
	}
}

fn local_task_submission(task: &Task) -> Option<SubmissionSlots> {
	match &task.kind {
		TaskKind::Calculation(calculation) => Some(calculation.submission),
		TaskKind::Transfer(transfer) => Some(transfer.submission),
		TaskKind::Metric(_) => None,
	}
}

fn device_transfer_owner<BridgeError>(
	source: DeviceId,
	destination: DeviceId,
	devices: &BTreeMap<DeviceId, LocalDeviceClass>,
) -> Result<TaskOwner, LocalError<BridgeError>> {
	let source_class = devices
		.get(&source)
		.copied()
		.ok_or(LocalError::MissingDevice { device: source })?;
	let destination_class = devices
		.get(&destination)
		.copied()
		.ok_or(LocalError::MissingDevice {
			device: destination,
		})?;
	match (source_class, destination_class) {
		(LocalDeviceClass::Host, LocalDeviceClass::Host) => Ok(TaskOwner::Host),
		(LocalDeviceClass::Hsa, LocalDeviceClass::Hsa) => Ok(TaskOwner::Hsa),
		(LocalDeviceClass::Cuda, LocalDeviceClass::Cuda) => match source == destination {
			true => Ok(TaskOwner::Cuda),
			false => Ok(TaskOwner::Bridge),
		},
		(LocalDeviceClass::Host, LocalDeviceClass::Cuda | LocalDeviceClass::Hsa)
		| (LocalDeviceClass::Cuda, LocalDeviceClass::Host | LocalDeviceClass::Hsa)
		| (LocalDeviceClass::Hsa, LocalDeviceClass::Host | LocalDeviceClass::Cuda) => Ok(TaskOwner::Bridge),
	}
}

fn provisional_warm_bundle(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	candidate: &PlannedCandidate,
	artifacts: &[ArtifactIdentity],
	reservations: &ReservationLedger,
) -> Result<FinalizedBundle, String> {
	let mut entries = Vec::with_capacity(topology.devices.len());
	for device in &topology.devices {
		let discovered = discovery
			.device(device.id)
			.ok_or_else(|| format!("warm device {} has no discovery capacity", device.id))?;
		let reservation = reservations
			.entry(device.id)
			.ok_or_else(|| format!("warm device {} has no held reservation", device.id))?;
		let usable = discovered
			.total_capacity
			.value
			.checked_sub(reservation.bytes)
			.ok_or_else(|| {
				format!(
					"warm device {} is smaller than its exact held reservation",
					device.id
				)
			})?;
		let provenance = discovered.total_capacity.provenance;
		entries.push(CapacityLedgerEntry {
			device: device.id,
			total: discovered.total_capacity,
			runtime_overhead: Property::new(ByteCount::ZERO, provenance),
			fragmentation: Property::new(ByteCount::ZERO, provenance),
			safety_headroom: Property::new(ByteCount::ZERO, provenance),
			recipe_usable: Property::new(usable, provenance),
		});
	}
	let realization = RealizationProfile {
		identity: RealizationIdentity::new(candidate.draft.candidate.digest()),
		draft: candidate.draft.identity,
		candidate: candidate.draft.candidate,
		discovery: candidate.draft.discovery,
		topology: candidate.draft.topology,
		artifacts: artifacts.to_vec(),
		resources: candidate.draft.resources.clone(),
		reservations: reservations.clone(),
		capacity: CapacityLedger { entries },
	};
	FinalizedBundle::finalize(
		BundleIdentity::new(candidate.draft.identity.digest()),
		topology,
		discovery,
		candidate.draft.clone(),
		realization,
		candidate.arena_layouts.clone(),
	)
	.map_err(|errors| format!("candidate warm address resolution failed: {errors}"))
}

fn prepare_warm_tasks<BridgeError>(bundle: &FinalizedBundle) -> Result<Vec<WarmTask>, LocalError<BridgeError>> {
	bundle
		.tasks()
		.iter()
		.map(|task| {
			let work = match (&task.kind, task.phase) {
				(TaskKind::Calculation(calculation), RunPhase::Loop) => WarmWork::Calculation {
					device: calculation.device,
					kernel_template: calculation.kernel_template,
					artifact: calculation.artifact,
					submission: calculation.submission,
					inputs: resolve_warm_values(bundle, task.id, &calculation.inputs)?,
					outputs: resolve_warm_values(bundle, task.id, &calculation.outputs)?,
					fault_flag: calculation
						.fault_flag
						.map(|value| resolve_warm_value(bundle, task.id, value))
						.transpose()?,
				},
				(TaskKind::Metric(metric), RunPhase::Loop) => WarmWork::Metric {
					purpose: metric.purpose,
					metric: metric.metric,
					slot: metric.slot,
					value: resolve_warm_value(bundle, task.id, metric.value)?,
				},
				(TaskKind::Transfer(transfer), RunPhase::Init)
					if matches!(
						(transfer.source, transfer.destination),
						(TransferEndpoint::External, TransferEndpoint::Device { .. })
					) =>
				{
					let TransferEndpoint::Device { device, .. } = transfer.destination else {
						return Err(LocalError::BackendState(
							"warm admission has no device destination",
						));
					};
					let endpoints = bundle
						.transfer_endpoints(task.id)
						.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
					let ResolvedTransferEndpoint::Device(destination) = endpoints.destination else {
						return Err(LocalError::BackendState(
							"warm admission has no resolved destination",
						));
					};
					WarmWork::Init {
						device,
						destination,
						bytes: transfer.bytes,
						submission: transfer.submission,
					}
				}
				(TaskKind::Transfer(transfer), RunPhase::Init | RunPhase::Loop) => {
					let endpoints = bundle
						.transfer_endpoints(task.id)
						.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
					WarmWork::Transfer {
						class: WorkClass::InternalTransfer,
						source: endpoints.source,
						destination: endpoints.destination,
						bytes: transfer.bytes,
						route: transfer.route.clone(),
						lane_claims: transfer.lane_claims.clone(),
						submission: transfer.submission,
					}
				}
				(TaskKind::Transfer(transfer), RunPhase::Exit) => {
					let endpoints = bundle
						.transfer_endpoints(task.id)
						.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
					WarmWork::Transfer {
						class: WorkClass::ExitTransfer,
						source: endpoints.source,
						destination: endpoints.destination,
						bytes: transfer.bytes,
						route: transfer.route.clone(),
						lane_claims: transfer.lane_claims.clone(),
						submission: transfer.submission,
					}
				}
				(TaskKind::Calculation(_) | TaskKind::Metric(_), RunPhase::Init | RunPhase::Exit) => {
					return Err(LocalError::BackendState(
						"warm task kind is illegal in its phase",
					));
				}
			};
			Ok(WarmTask {
				id: task.id,
				phase: task.phase,
				window: task.window,
				dependencies: task.dependencies.clone(),
				work,
			})
		})
		.collect()
}

fn resolve_warm_values<BridgeError>(
	bundle: &FinalizedBundle,
	task: TaskId,
	values: &[recipe_core::ValueId],
) -> Result<Vec<recipe_core::ResolvedValueLocation>, LocalError<BridgeError>> {
	values
		.iter()
		.map(|value| resolve_warm_value(bundle, task, *value))
		.collect()
}

fn resolve_warm_value<BridgeError>(
	bundle: &FinalizedBundle,
	task: TaskId,
	value: recipe_core::ValueId,
) -> Result<recipe_core::ResolvedValueLocation, LocalError<BridgeError>> {
	bundle
		.value_location(value)
		.copied()
		.ok_or(LocalError::TaskNotAssigned { task })
}

fn prepare_warm_images<BridgeError>(
	bundle: &FinalizedBundle,
) -> Result<BTreeMap<DeviceId, Vec<u8>>, LocalError<BridgeError>> {
	bundle
		.init_images()
		.iter()
		.map(|manifest| {
			let bytes = usize::try_from(manifest.bytes.get()).map_err(|_| LocalError::CapacityMismatch {
				device: manifest.device,
				detail: "warm init image does not fit the host address space",
			})?;
			Ok((manifest.device, vec![0; bytes]))
		})
		.collect()
}

fn prepare_warm_exits<BridgeError>(
	tasks: &[WarmTask],
) -> Result<BTreeMap<TaskId, Vec<u8>>, LocalError<BridgeError>> {
	tasks
		.iter()
		.filter_map(|task| {
			task.work
				.external_exit_bytes()
				.map(|bytes| (task.id, bytes))
		})
		.map(|(task, bytes)| {
			let bytes = usize::try_from(bytes.get()).map_err(|_| LocalError::BackendState(
				"warm exit image does not fit the host address space",
			))?;
			Ok((task, vec![0; bytes]))
		})
		.collect()
}

fn allocate_warm_arena<'cuda, 'hsa, BridgeResource, BridgeError>(
	resources: &mut LocalResources<'cuda, 'hsa, BridgeResource>,
	layout: &ArenaLayout,
) -> Result<LocalArena<'cuda, 'hsa>, LocalError<BridgeError>> {
	match resources.devices.get(&layout.device).copied() {
		Some(LocalDeviceClass::Host) => resources
			.host
			.as_ref()
			.ok_or(LocalError::BackendState(
				"warm host device has no realized host resources",
			))?
			.allocate_arena(layout)
			.map(LocalArena::Host)
			.map_err(LocalError::Host),
		Some(LocalDeviceClass::Cuda) => resources
			.cuda
			.allocate_arena(layout)
			.map(LocalArena::Cuda)
			.map_err(LocalError::Native),
		Some(LocalDeviceClass::Hsa) => resources
			.hsa
			.allocate_arena(layout)
			.map(LocalArena::Hsa)
			.map_err(LocalError::Native),
		None => Err(LocalError::MissingDevice {
			device: layout.device,
		}),
	}
}

fn run_warm_trace<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>,
	arenas: &BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>,
	tasks: &[WarmTask],
	images: &BTreeMap<DeviceId, Vec<u8>>,
	exits: &mut BTreeMap<TaskId, Vec<u8>>,
	pass: u32,
) -> Result<(), LocalError<Bridge::Error>>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	let run = recipe_core::RunId::new(u64::from(pass));
	let mut states = vec![WarmTaskState::Remaining; tasks.len()];
	let mut completed = BTreeSet::new();
	let mut pending = BTreeMap::new();
	for phase in [RunPhase::Init, RunPhase::Loop, RunPhase::Exit] {
		let mut idle_polls = 0_u32;
		loop {
			let mut progressed = false;
			for (index, task) in tasks.iter().enumerate() {
				let runnable = task.phase == phase
					&& states[index] == WarmTaskState::Remaining
					&& task
						.dependencies
						.iter()
						.all(|dependency| completed.contains(dependency))
					&& tasks.iter().enumerate().all(|(active_index, active)| {
						states[active_index] != WarmTaskState::Pending || active.window.overlaps(task.window)
					});
				match runnable {
					false => {}
					true => {
						let request = PendingRequest {
							task: task.id,
							phase: task.phase,
							class: task.work.class(),
							submission: task.work.submission(),
						};
						let mut token = prepare_warm_pending(bridge, resources, request)?;
						let work = task.work.backend_work(task.id, run, images)?;
						submit_warm_work(bridge, resources, arenas, &mut token, work)?;
						match pending.insert(task.id, token) {
							None => {}
							Some(duplicate) => {
								drop(duplicate);
								return Err(LocalError::PendingOwnerMismatch { task: task.id });
							}
						}
						states[index] = WarmTaskState::Pending;
						progressed = true;
					}
				}
			}
			for (index, task) in tasks.iter().enumerate() {
				match task.phase == phase && states[index] == WarmTaskState::Pending {
					false => continue,
					true => {}
				}
				let poll = {
					let token = pending
						.get_mut(&task.id)
						.ok_or(LocalError::PendingOwnerMismatch { task: task.id })?;
					poll_warm_pending(bridge, resources, token)?
				};
				match poll {
					BackendPoll::Pending => {}
					BackendPoll::Complete { .. } => {
						let mut token = pending
							.remove(&task.id)
							.ok_or(LocalError::PendingOwnerMismatch { task: task.id })?;
						if let Some(destination) = exits.get_mut(&task.id) {
							let work = task.work.backend_work::<Bridge::Error>(task.id, run, images)?;
							let BackendWork::ExitTransfer(transfer) = work else {
								return Err(LocalError::PendingOwnerMismatch { task: task.id });
							};
							collect_warm_exit(resources, arenas, &mut token, transfer, destination)?;
						}
						recycle_warm_pending(bridge, resources, token)?;
						states[index] = WarmTaskState::Complete;
						completed.insert(task.id);
						progressed = true;
					}
				}
			}
			let remaining = tasks
				.iter()
				.enumerate()
				.any(|(index, task)| task.phase == phase && states[index] != WarmTaskState::Complete);
			match remaining {
				false => break,
				true => {}
			}
			match (progressed, pending.is_empty()) {
				(false, true) => {
					return Err(LocalError::BackendState(
						"maximum-concurrency warm scheduler stalled",
					));
				}
				(false, false) => {
					idle_polls = idle_polls.saturating_add(1);
					match idle_polls == 10_000_000 {
						true => {
							return Err(LocalError::BackendState(
								"maximum-concurrency warm trace did not reach terminal completion",
							));
						}
						false => std::thread::yield_now(),
					}
				}
				(true, false | true) => idle_polls = 0,
			}
		}
	}
	Ok(())
}

fn prepare_warm_pending<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>,
	request: PendingRequest,
) -> Result<LocalPending<'cuda, 'hsa, Bridge::Pending>, LocalError<Bridge::Error>>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	match resources.tasks.get(&request.task).copied() {
		Some(TaskOwner::Host) => resources
			.host
			.as_mut()
			.ok_or(LocalError::BackendState(
				"warm host task has no realized resources",
			))?
			.prepare_pending(request)
			.map(LocalPending::Host)
			.map_err(LocalError::Host),
		Some(TaskOwner::Cuda) => resources
			.cuda
			.prepare_pending(request)
			.map(LocalPending::Cuda)
			.map_err(LocalError::Native),
		Some(TaskOwner::Hsa) => resources
			.hsa
			.prepare_pending(request)
			.map(LocalPending::Hsa)
			.map_err(LocalError::Native),
		Some(TaskOwner::Bridge) => bridge
			.prepare_pending(&mut resources.bridge, request)
			.map(|pending| LocalPending::Bridge {
				task: request.task,
				pending,
			})
			.map_err(LocalError::Bridge),
		None => Err(LocalError::TaskNotAssigned { task: request.task }),
	}
}

fn submit_warm_work<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>,
	arenas: &BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>,
	pending: &mut LocalPending<'cuda, 'hsa, Bridge::Pending>,
	work: BackendWork<'_>,
) -> Result<(), LocalError<Bridge::Error>>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	let task = work.task();
	let arena_set = ArenaSet::new(arenas);
	let lookup = ProjectedArenas { arenas: &arena_set };
	match pending {
		LocalPending::Host(pending) => {
			ensure_owner(resources, task, TaskOwner::Host)?;
			resources
				.host
				.as_mut()
				.ok_or(LocalError::BackendState(
					"warm host task has no realized resources",
				))?
				.submit(&lookup, pending, work)
				.map_err(LocalError::Host)
		}
		LocalPending::Cuda(pending) => {
			ensure_owner(resources, task, TaskOwner::Cuda)?;
			resources
				.cuda
				.submit(&lookup, pending, work)
				.map_err(LocalError::Native)
		}
		LocalPending::Hsa(pending) => {
			ensure_owner(resources, task, TaskOwner::Hsa)?;
			resources
				.hsa
				.submit(&lookup, pending, work)
				.map_err(LocalError::Native)
		}
		LocalPending::Bridge {
			task: pending_task,
			pending,
		} => {
			ensure_owner(resources, task, TaskOwner::Bridge)?;
			match *pending_task == task {
				true => Ok(()),
				false => Err(LocalError::PendingOwnerMismatch { task }),
			}?;
			let (class, transfer) = match work {
				BackendWork::InternalTransfer(transfer) => (WorkClass::InternalTransfer, transfer),
				BackendWork::ExitTransfer(transfer) => (WorkClass::ExitTransfer, transfer),
				BackendWork::InitAdmission(_) | BackendWork::Calculation(_) | BackendWork::Metric(_) => {
					return Err(LocalError::PendingOwnerMismatch { task });
				}
			};
			bridge
				.submit(
					&mut resources.bridge,
					LocalArenaSet { arenas: &arena_set },
					pending,
					class,
					transfer,
				)
				.map_err(LocalError::Bridge)
		}
	}
}

fn poll_warm_pending<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>,
	pending: &mut LocalPending<'cuda, 'hsa, Bridge::Pending>,
) -> Result<BackendPoll, LocalError<Bridge::Error>>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	match pending {
		LocalPending::Host(pending) => resources
			.host
			.as_mut()
			.ok_or(LocalError::BackendState(
				"warm host task has no realized resources",
			))?
			.poll_pending(pending)
			.map_err(LocalError::Host),
		LocalPending::Cuda(pending) => resources
			.cuda
			.poll(pending)
			.map_err(LocalError::Native),
		LocalPending::Hsa(pending) => resources
			.hsa
			.poll_pending(pending)
			.map_err(LocalError::Native),
		LocalPending::Bridge { pending, .. } => bridge
			.poll(&mut resources.bridge, pending)
			.map_err(LocalError::Bridge),
	}
}

fn collect_warm_exit<'cuda, 'hsa, BridgeResource, BridgePending, BridgeError>(
	resources: &mut LocalResources<'cuda, 'hsa, BridgeResource>,
	arenas: &BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>,
	pending: &mut LocalPending<'cuda, 'hsa, BridgePending>,
	work: TransferWork<'_>,
	destination: &mut [u8],
) -> Result<(), LocalError<BridgeError>> {
	let arena_set = ArenaSet::new(arenas);
	let lookup = ProjectedArenas { arenas: &arena_set };
	match pending {
		LocalPending::Host(pending) => resources
			.host
			.as_ref()
			.ok_or(LocalError::BackendState(
				"warm host exit has no realized resources",
			))?
			.collect_exit(pending, work, destination)
			.map_err(LocalError::Host),
		LocalPending::Cuda(pending) => resources
			.cuda
			.collect_exit(pending, work, destination)
			.map_err(LocalError::Native),
		LocalPending::Hsa(pending) => resources
			.hsa
			.collect_exit(&lookup, pending, work, destination)
			.map_err(LocalError::Native),
		LocalPending::Bridge { .. } => Err(LocalError::UnsupportedRoute {
			task: work.task,
			detail: "warm cross-backend transfer cannot have an external endpoint",
		}),
	}
}

fn recycle_warm_pending<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>,
	pending: LocalPending<'cuda, 'hsa, Bridge::Pending>,
) -> Result<(), LocalError<Bridge::Error>>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	match pending {
		LocalPending::Host(pending) => resources
			.host
			.as_mut()
			.ok_or(LocalError::BackendState(
				"warm host task has no realized resources",
			))?
			.recycle_pending(pending)
			.map_err(LocalError::Host),
		LocalPending::Cuda(pending) => resources
			.cuda
			.recycle_pending(pending)
			.map_err(LocalError::Native),
		LocalPending::Hsa(pending) => resources
			.hsa
			.recycle_pending(pending)
			.map_err(LocalError::Native),
		LocalPending::Bridge { pending, .. } => bridge
			.recycle_candidate_pending(&mut resources.bridge, pending)
			.map_err(LocalError::Bridge),
	}
}

fn release_warm_arenas<BridgeError>(
	arenas: BTreeMap<DeviceId, LocalArena<'_, '_>>,
) -> Result<(), LocalError<BridgeError>> {
	let mut first = None;
	for (device, arena) in arenas {
		let result = match arena {
			LocalArena::Host(arena) => arena.close().map_err(LocalError::Host),
			LocalArena::Cuda(arena) => arena.release().map_err(LocalError::Native),
			LocalArena::Hsa(arena) => arena.release().map_err(LocalError::Native),
		};
		match result {
			Ok(()) => {}
			Err(error) => match first {
				Some(_) => drop(error),
				None => first = Some(error),
			},
		}
		consume_context(device);
	}
	match first {
		Some(error) => Err(error),
		None => Ok(()),
	}
}

fn destroy_warm_resources<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resources: LocalResources<'cuda, 'hsa, Bridge::Resource>,
) -> Result<(), LocalError<Bridge::Error>>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	let LocalResources {
		host,
		cuda,
		hsa,
		bridge: bridge_resource,
		..
	} = resources;
	let mut first = bridge
		.destroy(bridge_resource)
		.map_err(LocalError::Bridge)
		.err();
	first = retain_first(first, hsa.destroy().map_err(LocalError::Native));
	first = retain_first(first, cuda.destroy().map_err(LocalError::Native));
	let host_result = match host {
		Some(host) => host.destroy().map_err(LocalError::Host),
		None => Ok(()),
	};
	first = retain_first(first, host_result);
	match first {
		Some(error) => Err(error),
		None => Ok(()),
	}
}

fn observe_capacity_ledger<BridgeError, BridgeResource>(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	reservations: &ReservationLedger,
	resources: &LocalResources<'_, '_, BridgeResource>,
) -> Result<CapacityLedger, LocalError<BridgeError>> {
	let mut entries = Vec::with_capacity(topology.devices.len());
	for device in &topology.devices {
		let discovered = discovery
			.device(device.id)
			.ok_or(LocalError::CapacityMismatch {
				device: device.id,
				detail: "device has no measured discovery capacity",
			})?;
		let reservation = reservations
			.entry(device.id)
			.ok_or(LocalError::CapacityMismatch {
				device: device.id,
				detail: "device has no exact held reservation",
			})?;
		let available = match resources.devices.get(&device.id).copied() {
			Some(LocalDeviceClass::Host) => resources
				.host
				.as_ref()
				.ok_or(LocalError::CapacityMismatch {
					device: device.id,
					detail: "host capacity has no realized host resource",
				})?
				.available_bytes(device.id)
				.map_err(LocalError::Host)?,
			Some(LocalDeviceClass::Cuda) => resources
				.cuda
				.available_bytes(device.id)
				.map_err(LocalError::Native)?,
			Some(LocalDeviceClass::Hsa) => resources
				.hsa
				.available_bytes(device.id)
				.map_err(LocalError::Native)?,
			None => {
				return Err(LocalError::CapacityMismatch {
					device: device.id,
					detail: "device has no local capacity owner",
				});
			}
		};
		let after_reservation = discovered
			.total_capacity
			.value
			.checked_sub(reservation.bytes)
			.ok_or(LocalError::CapacityMismatch {
				device: device.id,
				detail: "measured total is smaller than its exact reservation",
			})?;
		let overhead = after_reservation
			.checked_sub(available)
			.ok_or(LocalError::CapacityMismatch {
				device: device.id,
				detail: "live available bytes exceed measured total minus reservation",
			})?;
		entries.push(CapacityLedgerEntry {
			device: device.id,
			total: discovered.total_capacity,
			runtime_overhead: Property::new(overhead, PropertyProvenance::Measured),
			fragmentation: Property::new(ByteCount::ZERO, PropertyProvenance::Measured),
			safety_headroom: Property::new(ByteCount::ZERO, PropertyProvenance::Measured),
			recipe_usable: Property::new(available, PropertyProvenance::Measured),
		});
	}
	Ok(CapacityLedger { entries })
}

fn cleanup_bridge_resource<'cuda, 'hsa, Bridge>(
	bridge: &mut Bridge,
	resource: Bridge::Resource,
	error: LocalError<Bridge::Error>,
) -> LocalError<Bridge::Error>
where
	Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>,
{
	cleanup_error(error, bridge.destroy(resource).map_err(LocalError::Bridge))
}

fn cleanup_warm_host<BridgeError>(
	host: Option<HostResources>,
	error: LocalError<BridgeError>,
) -> LocalError<BridgeError> {
	let result = match host {
		Some(host) => host.destroy().map_err(LocalError::Host),
		None => Ok(()),
	};
	cleanup_error(error, result)
}

fn cleanup_prepared_host<BridgeError>(
	host: Option<HostPreparedResources>,
	error: LocalError<BridgeError>,
) -> LocalError<BridgeError> {
	let host_result = match host {
		Some(resources) => resources.destroy().map_err(LocalError::Host),
		None => Ok(()),
	};
	cleanup_error(error, host_result)
}

fn cleanup_prepared_cuda_host<BridgeError>(
	cuda: CudaPreparedResources<'_>,
	host: Option<HostPreparedResources>,
	error: LocalError<BridgeError>,
) -> LocalError<BridgeError> {
	let first = cleanup_error(error, cuda.destroy().map_err(LocalError::Native));
	cleanup_prepared_host(host, first)
}

fn cleanup_prepared_native_host<BridgeError>(
	hsa: HsaPreparedResources<'_>,
	cuda: CudaPreparedResources<'_>,
	host: Option<HostPreparedResources>,
	error: LocalError<BridgeError>,
) -> LocalError<BridgeError> {
	let first = cleanup_error(error, hsa.destroy().map_err(LocalError::Native));
	cleanup_prepared_cuda_host(cuda, host, first)
}

fn cleanup_error<E>(first: E, result: Result<(), E>) -> E {
	match result {
		Ok(()) => first,
		Err(discarded) => {
			drop(discarded);
			first
		}
	}
}

fn validate_prepared_identity<BridgeError>(
	candidate: &PlannedCandidate,
	artifacts: &[ArtifactIdentity],
	reservations: &ReservationLedger,
	bundle: &FinalizedBundle,
) -> Result<(), LocalError<BridgeError>> {
	match bundle.topology() == candidate.draft.topology
		&& bundle.discovery() == candidate.draft.discovery
		&& bundle.draft() == candidate.draft.identity
		&& bundle.candidate() == candidate.draft.candidate
	{
		true => Ok(()),
		false => Err(LocalError::BackendState(
			"finalized bundle identity differs from its prepared candidate",
		)),
	}?;
	match bundle.tasks() == candidate.draft.tasks
		&& bundle.kernels() == candidate.draft.kernels
		&& bundle.artifact_builds() == candidate.draft.artifact_builds
		&& bundle.resources() == &candidate.draft.resources
		&& bundle.init_images() == candidate.draft.init_images
		&& bundle.reservations() == reservations
	{
		true => Ok(()),
		false => Err(LocalError::BackendState(
			"finalized plan or resources differ from their prepared candidate",
		)),
	}?;
	let prepared_artifacts = artifacts
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	let finalized_artifacts = bundle
		.artifacts()
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	match prepared_artifacts == finalized_artifacts {
		true => Ok(()),
		false => Err(LocalError::BackendState(
			"finalized artifacts differ from the loaded candidate images",
		)),
	}
}

fn classify<BridgeError>(
	bundle: &FinalizedBundle,
	declared: &[(DeviceId, LocalDeviceClass)],
) -> Result<Partitions, LocalError<BridgeError>> {
	let finalized_devices = bundle
		.arena_layouts()
		.iter()
		.map(|layout| layout.device)
		.collect::<BTreeSet<_>>();
	let devices = validate_device_owners(finalized_devices, declared)?;

	let mut result = Partitions {
		devices,
		tasks: BTreeMap::new(),
		host: BTreeSet::new(),
		cuda: BTreeSet::new(),
		hsa: BTreeSet::new(),
		bridge: BTreeSet::new(),
	};
	for task in bundle.tasks() {
		let owner =
			match &task.kind {
				TaskKind::Calculation(calculation) => match result.devices.get(&calculation.device) {
					Some(LocalDeviceClass::Cuda) => TaskOwner::Cuda,
					Some(LocalDeviceClass::Hsa) => TaskOwner::Hsa,
					Some(LocalDeviceClass::Host) | None => {
						return Err(LocalError::UnsupportedCalculation { task: task.id });
					}
				},
				TaskKind::Metric(metric) => {
					let location = bundle
						.value_location(metric.value)
						.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
					class_owner(result.devices.get(&location.device).copied().ok_or(
						LocalError::MissingDevice {
							device: location.device,
						},
					)?)
				}
				TaskKind::Transfer(transfer) => {
					match transfer.route.len() {
						0 | 1 => Ok(()),
						2.. => Err(LocalError::UnsupportedRoute {
							task: task.id,
							detail: "executor-visible transfers must be planner-expanded to one hop",
						}),
					}?;
					let endpoints = bundle
						.transfer_endpoints(task.id)
						.ok_or(LocalError::TaskNotAssigned { task: task.id })?;
					transfer_owner::<BridgeError>(
						task.id,
						endpoints.source,
						endpoints.destination,
						&result.devices,
					)?
				}
			};
		result.tasks.insert(task.id, owner);
		match owner {
			TaskOwner::Host => {
				result.host.insert(task.id);
			}
			TaskOwner::Cuda => {
				result.cuda.insert(task.id);
			}
			TaskOwner::Hsa => {
				result.hsa.insert(task.id);
			}
			TaskOwner::Bridge => {
				let route = match &task.kind {
					TaskKind::Transfer(transfer) => &transfer.route,
					TaskKind::Calculation(_) | TaskKind::Metric(_) => {
						return Err(LocalError::UnsupportedRoute {
							task: task.id,
							detail: "only transfers may use the cross-backend bridge",
						});
					}
				};
				match route.len() {
					1 => Ok(()),
					0 | 2.. => Err(LocalError::UnsupportedRoute {
						task: task.id,
						detail: "cross-backend transfers require exactly one finalized link",
					}),
				}?;
				result.bridge.insert(task.id);
			}
		}
	}
	validate_partition_slots(bundle.tasks(), &result)?;
	Ok(result)
}

fn transfer_owner<BridgeError>(
	task: TaskId,
	source: ResolvedTransferEndpoint,
	destination: ResolvedTransferEndpoint,
	devices: &BTreeMap<DeviceId, LocalDeviceClass>,
) -> Result<TaskOwner, LocalError<BridgeError>> {
	match (source, destination) {
		(ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::Device(destination)) => devices
			.get(&destination.device)
			.copied()
			.map(class_owner)
			.ok_or(LocalError::MissingDevice {
				device: destination.device,
			}),
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::External) => devices
			.get(&source.device)
			.copied()
			.map(class_owner)
			.ok_or(LocalError::MissingDevice {
				device: source.device,
			}),
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => {
			device_transfer_owner(source.device, destination.device, devices)
		}
		(ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::External) => {
			Err(LocalError::UnsupportedRoute {
				task,
				detail: "external-to-external transfers have no local owner",
			})
		}
	}
}

const fn class_owner(class: LocalDeviceClass) -> TaskOwner {
	match class {
		LocalDeviceClass::Host => TaskOwner::Host,
		LocalDeviceClass::Cuda => TaskOwner::Cuda,
		LocalDeviceClass::Hsa => TaskOwner::Hsa,
	}
}

fn ensure_owner<BridgeError, BridgeResource>(
	resource: &LocalResources<'_, '_, BridgeResource>,
	task: TaskId,
	expected: TaskOwner,
) -> Result<(), LocalError<BridgeError>> {
	match resource.tasks.get(&task).copied() {
		Some(actual) => match actual == expected {
			true => Ok(()),
			false => Err(LocalError::PendingOwnerMismatch { task }),
		},
		None => Err(LocalError::TaskNotAssigned { task }),
	}
}

fn record<BridgeError>(batch: &mut PhysicalCallBatch, call: PhysicalCall) -> Result<(), LocalError<BridgeError>> {
	match batch.try_push(call) {
		Ok(()) => Ok(()),
		Err(PhysicalCallBatchOverflow) => Err(LocalError::PhysicalAccountingOverflow),
	}
}

fn retain_first<E>(first: Option<E>, result: Result<(), E>) -> Option<E> {
	match (first, result) {
		(None, Err(error)) => Some(error),
		(Some(existing), Err(discarded)) => {
			drop(discarded);
			Some(existing)
		}
		(first, Ok(())) => first,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use recipe_core::{
		ArenaObjectId, ByteCount, ByteOffset, DType, ResolvedValueLocation, RunPhase, SubmissionSlots, ValueId,
	};

	use crate::tests::{CALCULATION, GPU, INIT, METRIC, candidate_fixture, fixture};

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	struct FakeError(&'static str);

	impl fmt::Display for FakeError {
		fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			formatter.write_str(self.0)
		}
	}

	impl StdError for FakeError {}

	#[derive(Debug)]
	struct FakeResource {
		tasks: BTreeSet<TaskId>,
	}

	#[derive(Debug)]
	struct FakePending {
		task: TaskId,
		polls: u8,
	}

	#[derive(Clone, Debug, Default)]
	struct FakeBridge {
		binds: usize,
		realizes: usize,
		validates: usize,
		prepares: usize,
		destroys: usize,
		fail_validation: bool,
	}

	impl CrossBackendTransfer<'_, '_> for FakeBridge {
		type Error = FakeError;
		type Pending = FakePending;
		type Resource = FakeResource;

		fn bind(
			&mut self,
			bundle: &FinalizedBundle,
			tasks: &BTreeSet<TaskId>,
			devices: &BTreeMap<DeviceId, LocalDeviceClass>,
		) -> Result<Self::Resource, Self::Error> {
			consume_context((bundle, devices));
			self.binds += 1;
			Ok(FakeResource {
				tasks: tasks.clone(),
			})
		}

		fn prepare_pending(
			&mut self,
			resource: &mut Self::Resource,
			request: PendingRequest,
		) -> Result<Self::Pending, Self::Error> {
			match resource.tasks.get(&request.task) {
				Some(_) => Ok(()),
				None => Err(FakeError("task was not bound")),
			}?;
			self.prepares += 1;
			Ok(FakePending {
				task: request.task,
				polls: 0,
			})
		}

		fn submit(
			&mut self,
			resource: &mut Self::Resource,
			arenas: LocalArenaSet<'_, '_, '_, '_>,
			pending: &mut Self::Pending,
			class: WorkClass,
			work: TransferWork<'_>,
		) -> Result<(), Self::Error> {
			consume_context((resource, arenas));
			match pending.task == work.task
				&& matches!(class, WorkClass::InternalTransfer | WorkClass::ExitTransfer)
			{
				true => Ok(()),
				false => Err(FakeError("submission differs from prepared transfer")),
			}
		}

		fn poll(
			&mut self,
			resource: &mut Self::Resource,
			pending: &mut Self::Pending,
		) -> Result<BackendPoll, Self::Error> {
			consume_context(resource);
			pending.polls += 1;
			match pending.polls {
				1 => Ok(BackendPoll::Pending),
				0 | 2..=u8::MAX => Ok(BackendPoll::Complete { metric: None }),
			}
		}

		fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error> {
			consume_context(resource);
			self.destroys += 1;
			Ok(())
		}
	}

	impl CandidateCrossBackendTransfer<'_, '_> for FakeBridge {
		fn realize_candidate(
			&mut self,
			candidate: &PlannedCandidate,
			tasks: &BTreeSet<TaskId>,
			devices: &BTreeMap<DeviceId, LocalDeviceClass>,
		) -> Result<Self::Resource, Self::Error> {
			consume_context((candidate, devices));
			self.realizes += 1;
			Ok(FakeResource {
				tasks: tasks.clone(),
			})
		}

		fn validate_handoff(
			&mut self,
			resource: &Self::Resource,
			bundle: &FinalizedBundle,
			tasks: &BTreeSet<TaskId>,
			devices: &BTreeMap<DeviceId, LocalDeviceClass>,
		) -> Result<(), Self::Error> {
			consume_context((bundle, devices));
			self.validates += 1;
			match !self.fail_validation && resource.tasks == *tasks {
				true => Ok(()),
				false => Err(FakeError("prepared bridge validation failed")),
			}
		}

		fn recycle_candidate_pending(
			&mut self,
			resource: &mut Self::Resource,
			pending: Self::Pending,
		) -> Result<(), Self::Error> {
			match resource.tasks.contains(&pending.task) && pending.polls > 0 {
				true => Ok(()),
				false => Err(FakeError("candidate bridge token is not terminal or owned")),
			}
		}
	}

	fn location(device: DeviceId) -> ResolvedValueLocation {
		ResolvedValueLocation {
			value: ValueId::new(device.get()),
			dtype: DType::F32,
			device,
			bytes: ByteCount::new(16),
			object: ArenaObjectId::new(device.get()),
			object_offset: ByteOffset::new(0),
			arena_offset: ByteOffset::new(0),
		}
	}

	fn success<T, E: fmt::Debug>(result: Result<T, E>) -> T {
		match result {
			Ok(value) => value,
			Err(error) => panic!("test setup failed: {error:?}"),
		}
	}

	#[test]
	fn homogeneous_cuda_bundle_assigns_every_task_to_cuda() {
		let bundle = fixture(b"local-partition");
		let partitions = success(classify::<FakeError>(
			&bundle,
			&[(GPU, LocalDeviceClass::Cuda)],
		));
		assert_eq!(partitions.cuda, BTreeSet::from([INIT, CALCULATION, METRIC]));
		assert!(partitions.host.is_empty());
		assert!(partitions.hsa.is_empty());
		assert!(partitions.bridge.is_empty());
	}

	#[test]
	fn calculations_bound_to_host_fail_before_resource_realization() {
		let bundle = fixture(b"local-host-rejection");
		let error = match classify::<FakeError>(&bundle, &[(GPU, LocalDeviceClass::Host)]) {
			Ok(partitions) => panic!("host calculation produced partitions: {partitions:?}"),
			Err(error) => error,
		};
		assert!(matches!(
			error,
			LocalError::UnsupportedCalculation { task: CALCULATION }
		));
	}

	#[test]
	fn device_ownership_must_be_disjoint_and_exact() {
		let bundle = fixture(b"local-ownership");
		let duplicate = match classify::<FakeError>(
			&bundle,
			&[(GPU, LocalDeviceClass::Cuda), (GPU, LocalDeviceClass::Hsa)],
		) {
			Ok(partitions) => panic!("duplicate binding produced partitions: {partitions:?}"),
			Err(error) => error,
		};
		assert!(matches!(
			duplicate,
			LocalError::DuplicateDevice { device: GPU }
		));

		let missing = match classify::<FakeError>(&bundle, &[]) {
			Ok(partitions) => panic!("missing binding produced partitions: {partitions:?}"),
			Err(error) => error,
		};
		assert!(matches!(missing, LocalError::MissingDevice { device: GPU }));

		let extra_device = DeviceId::new(99);
		let unexpected = match classify::<FakeError>(
			&bundle,
			&[
				(GPU, LocalDeviceClass::Cuda),
				(extra_device, LocalDeviceClass::Host),
			],
		) {
			Ok(partitions) => panic!("unexpected binding produced partitions: {partitions:?}"),
			Err(error) => error,
		};
		match unexpected {
			LocalError::UnexpectedDevice { device } => assert_eq!(device, extra_device),
			other => panic!("unexpected binding error: {other:?}"),
		}
	}

	#[test]
	fn one_hop_transfer_owner_matrix_is_fail_closed() {
		let host = DeviceId::new(1);
		let cuda_a = DeviceId::new(2);
		let cuda_b = DeviceId::new(3);
		let hsa_a = DeviceId::new(4);
		let hsa_b = DeviceId::new(5);
		let devices = BTreeMap::from([
			(host, LocalDeviceClass::Host),
			(cuda_a, LocalDeviceClass::Cuda),
			(cuda_b, LocalDeviceClass::Cuda),
			(hsa_a, LocalDeviceClass::Hsa),
			(hsa_b, LocalDeviceClass::Hsa),
		]);
		let external = ResolvedTransferEndpoint::External;
		let endpoint = |device| ResolvedTransferEndpoint::Device(location(device));

		assert_eq!(
			success(transfer_owner::<FakeError>(
				TaskId::new(1),
				external,
				endpoint(cuda_a),
				&devices,
			)),
			TaskOwner::Cuda
		);
		assert_eq!(
			success(transfer_owner::<FakeError>(
				TaskId::new(2),
				endpoint(host),
				external,
				&devices,
			)),
			TaskOwner::Host
		);
		assert_eq!(
			success(transfer_owner::<FakeError>(
				TaskId::new(3),
				endpoint(cuda_a),
				endpoint(cuda_a),
				&devices,
			)),
			TaskOwner::Cuda
		);
		assert_eq!(
			success(transfer_owner::<FakeError>(
				TaskId::new(4),
				endpoint(cuda_a),
				endpoint(cuda_b),
				&devices,
			)),
			TaskOwner::Bridge
		);
		assert_eq!(
			success(transfer_owner::<FakeError>(
				TaskId::new(5),
				endpoint(hsa_a),
				endpoint(hsa_b),
				&devices,
			)),
			TaskOwner::Hsa
		);
		assert_eq!(
			success(transfer_owner::<FakeError>(
				TaskId::new(6),
				endpoint(host),
				endpoint(hsa_a),
				&devices,
			)),
			TaskOwner::Bridge
		);
		let unsupported = transfer_owner::<FakeError>(TaskId::new(7), external, external, &devices);
		assert!(matches!(
			unsupported,
			Err(LocalError::UnsupportedRoute { .. })
		));
	}

	#[test]
	fn fake_bridge_realizes_tokens_before_nonblocking_polls() {
		let bundle = fixture(b"local-fake-bridge");
		let task = TaskId::new(77);
		let tasks = BTreeSet::from([task]);
		let devices = BTreeMap::from([(GPU, LocalDeviceClass::Cuda)]);
		let mut bridge = FakeBridge::default();
		let mut resource = success(bridge.bind(&bundle, &tasks, &devices));
		let mut pending = success(bridge.prepare_pending(
			&mut resource,
			PendingRequest {
				task,
				phase: RunPhase::Loop,
				class: WorkClass::InternalTransfer,
				submission: Some(SubmissionSlots {
					queue: recipe_core::QueueSlotId::new(77),
					completion: recipe_core::CompletionSlotId::new(77),
				}),
			},
		));
		assert_eq!(
			success(bridge.poll(&mut resource, &mut pending)),
			BackendPoll::Pending
		);
		assert_eq!(
			success(bridge.poll(&mut resource, &mut pending)),
			BackendPoll::Complete { metric: None }
		);
		success(bridge.destroy(resource));
		assert_eq!((bridge.binds, bridge.prepares, bridge.destroys), (1, 1, 1));
	}

	#[test]
	fn rejecting_bridge_reports_the_first_finalized_cross_task() {
		let bundle = fixture(b"local-reject-bridge");
		let first = TaskId::new(41);
		let tasks = BTreeSet::from([TaskId::new(42), first]);
		let devices = BTreeMap::from([(GPU, LocalDeviceClass::Cuda)]);
		let error = match RejectCrossBackend.bind(&bundle, &tasks, &devices) {
			Ok(()) => panic!("rejecting bridge accepted cross-backend work"),
			Err(error) => error,
		};
		assert_eq!(error.task(), first);
	}

	#[test]
	fn candidate_and_finalized_partitions_are_identical() {
		let fixture = candidate_fixture(b"local-candidate-partition");
		let candidate = success(classify_candidate::<FakeError>(
			&fixture.candidate,
			&[(GPU, LocalDeviceClass::Cuda)],
		));
		let finalized = success(classify::<FakeError>(
			&fixture.bundle,
			&[(GPU, LocalDeviceClass::Cuda)],
		));
		assert_eq!(candidate.devices, finalized.devices);
		assert_eq!(candidate.tasks, finalized.tasks);
		assert_eq!(candidate.host, finalized.host);
		assert_eq!(candidate.cuda, finalized.cuda);
		assert_eq!(candidate.hsa, finalized.hsa);
		assert_eq!(candidate.bridge, finalized.bridge);
	}

	#[test]
	fn prepared_identity_rejects_a_different_artifact_image() {
		let prepared = candidate_fixture(b"local-prepared-identity");
		let finalized = candidate_fixture(b"local-finalized-identity");
		let artifacts = prepared
			.artifacts
			.iter()
			.map(|artifact| artifact.identity().clone())
			.collect::<Vec<_>>();
		let error = validate_prepared_identity::<FakeError>(
			&prepared.candidate,
			&artifacts,
			&prepared.reservations,
			&finalized.bundle,
		);
		assert!(matches!(error, Err(LocalError::BackendState(_))));
	}

	#[test]
	fn prepared_bridge_hands_the_same_resource_to_bind_once() {
		let bundle = fixture(b"local-prepared-bridge");
		let task = TaskId::new(77);
		let tasks = BTreeSet::from([task]);
		let devices = BTreeMap::from([(GPU, LocalDeviceClass::Cuda)]);
		let mut bridge = FakeBridge::default();
		let resource = success(bridge.realize_candidate(
			&candidate_fixture(b"local-prepared-bridge").candidate,
			&tasks,
			&devices,
		));
		success(bridge.validate_handoff(&resource, &bundle, &tasks, &devices));
		let mut prepared = PreparedBridge {
			bridge,
			resource: PreparedBridgeResource::Available(resource),
			tasks: tasks.clone(),
			devices: devices.clone(),
			handoff_validated: true,
		};
		let resource = success(prepared.bind(&bundle, &tasks, &devices));
		let duplicate = prepared.bind(&bundle, &tasks, &devices);
		assert!(matches!(duplicate, Err(PreparedBridgeError::State(_))));
		success(prepared.destroy(resource));
		assert_eq!(
			(
				prepared.bridge.binds,
				prepared.bridge.realizes,
				prepared.bridge.validates,
				prepared.bridge.destroys,
			),
			(0, 1, 1, 1),
		);
	}

	#[test]
	fn prepared_bridge_mismatch_does_not_consume_the_resource() {
		let fixture = candidate_fixture(b"local-prepared-bridge-mismatch");
		let task = TaskId::new(77);
		let tasks = BTreeSet::from([task]);
		let devices = BTreeMap::from([(GPU, LocalDeviceClass::Cuda)]);
		let mut bridge = FakeBridge::default();
		let resource = success(bridge.realize_candidate(&fixture.candidate, &tasks, &devices));
		success(bridge.validate_handoff(&resource, &fixture.bundle, &tasks, &devices));
		let mut prepared = PreparedBridge {
			bridge,
			resource: PreparedBridgeResource::Available(resource),
			tasks: tasks.clone(),
			devices: devices.clone(),
			handoff_validated: true,
		};
		let mismatch = prepared.bind(&fixture.bundle, &BTreeSet::new(), &devices);
		assert!(matches!(mismatch, Err(PreparedBridgeError::State(_))));
		let resource = success(prepared.bind(&fixture.bundle, &tasks, &devices));
		success(prepared.destroy(resource));
		assert_eq!(
			(prepared.bridge.validates, prepared.bridge.destroys),
			(1, 1)
		);
	}

	#[test]
	fn prepared_bridge_validation_failure_does_not_authorize_or_consume_handoff() {
		let fixture = candidate_fixture(b"local-prepared-bridge-validation");
		let task = TaskId::new(77);
		let tasks = BTreeSet::from([task]);
		let devices = BTreeMap::from([(GPU, LocalDeviceClass::Cuda)]);
		let mut bridge = FakeBridge {
			fail_validation: true,
			..FakeBridge::default()
		};
		let resource = success(bridge.realize_candidate(&fixture.candidate, &tasks, &devices));
		let mut prepared = PreparedBridge {
			bridge,
			resource: PreparedBridgeResource::Available(resource),
			tasks: tasks.clone(),
			devices: devices.clone(),
			handoff_validated: false,
		};
		let validation = prepared.bind(&fixture.bundle, &tasks, &devices);
		assert!(matches!(validation, Err(PreparedBridgeError::State(_))));
		let resource = match &prepared.resource {
			PreparedBridgeResource::Available(resource) => resource,
			PreparedBridgeResource::Consumed => panic!("failed validation consumed the bridge resource"),
		};
		let validation = prepared
			.bridge
			.validate_handoff(resource, &fixture.bundle, &tasks, &devices);
		assert!(validation.is_err());
		prepared.bridge.fail_validation = false;
		let resource = match &prepared.resource {
			PreparedBridgeResource::Available(resource) => resource,
			PreparedBridgeResource::Consumed => panic!("failed validation consumed the bridge resource"),
		};
		success(
			prepared
				.bridge
				.validate_handoff(resource, &fixture.bundle, &tasks, &devices),
		);
		prepared.handoff_validated = true;
		let resource = success(prepared.bind(&fixture.bundle, &tasks, &devices));
		success(prepared.destroy(resource));
		assert_eq!(
			(prepared.bridge.validates, prepared.bridge.destroys),
			(2, 1)
		);
	}
}
