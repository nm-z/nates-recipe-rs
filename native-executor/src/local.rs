//! Heterogeneous local execution over host, CUDA, and HSA partitions.

use core::fmt;
use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error as StdError,
};

use recipe_core::{ArenaLayout, ArtifactIdentity, BundleIdentity, ByteCount, CapacityLedger, CapacityLedgerEntry, DeviceId, DiscoveryIdentity, DiscoveryProfile, FinalizedBundle, LoopIteration, Property, PropertyProvenance, RealizationIdentity, RealizationProfile, ReservationEvidence, ReservationLedger, ResolvedTransferEndpoint, RunPhase, SubmissionSlots, TaskId, TaskKind, Topology, TopologyIdentity, TransferEndpoint};
use recipe_executor::{ArenaSet, Backend, BackendPoll, BackendWork, PendingRequest, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus, TransferWork, WorkClass, sealed};
use recipe_host::{Arena as HostArena, HostArenaLookup, HostBackend, HostBackendConfig, HostPending, HostPreparedResources, HostResources};
use recipe_planner::PlannedCandidate;

use crate::{
	CudaBackend, CudaBinding, Error, HsaBackend, HsaBinding, NativeExecutionEvidence, RuntimeArtifact,
	candidate::{CandidateFailure, CandidateRealizationRequest, CandidateSessionFactory},
	cuda::{CudaArena, CudaArenaLookup, CudaPending, CudaPreparedResources, CudaResources},
	hsa::{HsaArena, HsaArenaLookup, HsaPending, HsaPreparedResources, HsaResources},
};

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

	fn bind(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<Self::Resource, Self::Error>;

	fn prepare_pending(&mut self, resource: &mut Self::Resource, request: PendingRequest) -> Result<Self::Pending, Self::Error>;

	fn submit(&mut self, resource: &mut Self::Resource, arenas: LocalArenaSet<'_, '_, 'cuda, 'hsa>, pending: &mut Self::Pending, class: WorkClass, work: TransferWork<'_>) -> Result<(), Self::Error>;

	fn poll(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending) -> Result<BackendPoll, Self::Error>;

	/// Whether terminal transfer tokens can be reset in place for another
	/// execution of the same finalized loop task.
	#[must_use]
	fn supports_loop_repetition(&self) -> bool { false }

	/// Leaves a never-submitted loop token ready or rearms a terminal token
	/// without allocating or replacing its pre-realized resources.
	fn rearm_loop_pending(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending) -> Result<(), Self::Error> {
		let _ = (resource, pending);
		Ok(())
	}

	fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error>;
}

/// Pre-final realization extension for cross-backend one-hop transfers.
///
/// Implementations must create every registration, staging allocation, queue,
/// and completion object in `realize_candidate`. `validate_handoff` may inspect
/// immutable finalized addresses but may not allocate or replace `resource`.
pub trait CandidateCrossBackendTransfer<'cuda, 'hsa>: CrossBackendTransfer<'cuda, 'hsa> + Clone {
	fn realize_candidate(&mut self, candidate: &PlannedCandidate, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<Self::Resource, Self::Error>;

	fn validate_handoff(&mut self, resource: &Self::Resource, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<(), Self::Error>;

	/// Returns one terminal warm-pass token to the exact pre-final resource
	/// that created it.
	fn recycle_candidate_pending(&mut self, resource: &mut Self::Resource, pending: Self::Pending) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossBackendUnavailable {
	task: TaskId,
}

impl CrossBackendUnavailable {
	#[must_use]
	pub const fn task(self) -> TaskId { self.task }
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

fn consume_context<T>(context: T) { drop(context); }

/// Bridge policy for homogeneous deployments. It fails bind closed when a
/// finalized bundle contains any cross-backend transfer.
#[derive(Clone, Copy, Debug, Default)]
pub struct RejectCrossBackend;

impl CrossBackendTransfer<'_, '_> for RejectCrossBackend {
	type Error = CrossBackendUnavailable;
	type Pending = ();
	type Resource = ();

	fn bind(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<Self::Resource, Self::Error> {
		consume_context((bundle, devices));
		match tasks.iter().next().copied() {
			Some(task) => Err(CrossBackendUnavailable { task }),
			None => Ok(()),
		}
	}

	fn prepare_pending(&mut self, resource: &mut Self::Resource, request: PendingRequest) -> Result<Self::Pending, Self::Error> {
		consume_context(resource);
		Err(CrossBackendUnavailable { task: request.task })
	}

	fn submit(&mut self, resource: &mut Self::Resource, arenas: LocalArenaSet<'_, '_, '_, '_>, pending: &mut Self::Pending, class: WorkClass, work: TransferWork<'_>) -> Result<(), Self::Error> {
		consume_context((resource, arenas, pending, class));
		Err(CrossBackendUnavailable { task: work.task })
	}

	fn poll(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending) -> Result<BackendPoll, Self::Error> {
		consume_context((resource, pending));
		Err(CrossBackendUnavailable {
			task: TaskId::new(0),
		})
	}

	fn supports_loop_repetition(&self) -> bool { true }

	fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error> {
		let () = resource;
		Ok(())
	}
}

impl CandidateCrossBackendTransfer<'_, '_> for RejectCrossBackend {
	fn realize_candidate(&mut self, candidate: &PlannedCandidate, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<Self::Resource, Self::Error> {
		consume_context((candidate, devices));
		match tasks.iter().next().copied() {
			Some(task) => Err(CrossBackendUnavailable { task }),
			None => Ok(()),
		}
	}

	fn validate_handoff(&mut self, resource: &Self::Resource, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<(), Self::Error> {
		consume_context((resource, bundle, devices));
		match tasks.iter().next().copied() {
			Some(task) => Err(CrossBackendUnavailable { task }),
			None => Ok(()),
		}
	}

	fn recycle_candidate_pending(&mut self, resource: &mut Self::Resource, pending: Self::Pending) -> Result<(), Self::Error> {
		consume_context((resource, pending));
		Err(CrossBackendUnavailable {
			task: TaskId::new(0),
		})
	}
}

#[derive(Debug)]
pub enum LocalError<BridgeError> {
	CapacityMismatch {
		device: DeviceId,
		detail: &'static str,
	},
	BackendState(&'static str),
	Host(recipe_host::Error),
	Native(Error),
	Bridge(BridgeError),
}

impl<BridgeError: fmt::Display> fmt::Display for LocalError<BridgeError> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CapacityMismatch { device, detail } => {
				write!(
					formatter,
					"local capacity observation for device {device} is invalid: {detail}"
				)
			}
			Self::BackendState(detail) => write!(formatter, "local backend state is invalid: {detail}"),
			Self::Host(error) => write!(formatter, "host partition failed: {error}"),
			Self::Native(error) => write!(formatter, "native partition failed: {error}"),
			Self::Bridge(error) => write!(formatter, "cross-backend bridge failed: {error}"),
		}
	}
}

impl<BridgeError> StdError for LocalError<BridgeError>
where BridgeError: StdError + 'static
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Host(error) => Some(error),
			Self::Native(error) => Some(error),
			Self::Bridge(error) => Some(error),
			Self::CapacityMismatch { .. } | Self::BackendState(_) => None,
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
where E: StdError + 'static
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
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>
{
	type Error = PreparedBridgeError<Bridge::Error>;
	type Pending = Bridge::Pending;
	type Resource = Bridge::Resource;

	fn bind(&mut self, bundle: &FinalizedBundle, tasks: &BTreeSet<TaskId>, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<Self::Resource, Self::Error> {
		debug_assert!(*tasks == self.tasks && *devices == self.devices);
		match self.handoff_validated {
			true => Ok(()),
			false => {
				Err(PreparedBridgeError::State(
					"prepared bridge handoff was not validated before finalized bind",
				))
			}
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

	fn prepare_pending(&mut self, resource: &mut Self::Resource, request: PendingRequest) -> Result<Self::Pending, Self::Error> {
		self.bridge
			.prepare_pending(resource, request)
			.map_err(PreparedBridgeError::Bridge)
	}

	fn submit(&mut self, resource: &mut Self::Resource, arenas: LocalArenaSet<'_, '_, 'cuda, 'hsa>, pending: &mut Self::Pending, class: WorkClass, work: TransferWork<'_>) -> Result<(), Self::Error> {
		self.bridge
			.submit(resource, arenas, pending, class, work)
			.map_err(PreparedBridgeError::Bridge)
	}

	fn poll(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending) -> Result<BackendPoll, Self::Error> {
		self.bridge
			.poll(resource, pending)
			.map_err(PreparedBridgeError::Bridge)
	}

	fn supports_loop_repetition(&self) -> bool { self.bridge.supports_loop_repetition() }

	fn rearm_loop_pending(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending) -> Result<(), Self::Error> {
		self.bridge
			.rearm_loop_pending(resource, pending)
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

type PreparedLocalBackend<'cuda, 'hsa, Bridge> = LocalBackend<'cuda, 'hsa, PreparedBridge<Bridge, <Bridge as CrossBackendTransfer<'cuda, 'hsa>>::Resource>>;

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
		_bundle: FinalizedBundle,
		resources: Box<LocalResources<'cuda, 'hsa, BridgeResource>>,
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
		submission: SubmissionSlots,
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
			Self::Init { submission, .. } | Self::Calculation { submission, .. } | Self::Transfer { submission, .. } | Self::Metric { submission, .. } => Some(*submission),
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
			Self::Init { .. } | Self::Calculation { .. } | Self::Transfer { .. } | Self::Metric { .. } => None,
		}
	}

	fn backend_work<'work>(&'work self, task: TaskId, run: recipe_core::RunId, iteration: LoopIteration, images: &'work BTreeMap<DeviceId, Vec<u8>>) -> BackendWork<'work> {
		match self {
			Self::Init {
				device,
				destination,
				bytes,
				submission,
			} => {
				let image = &images[device];
				BackendWork::InitAdmission(recipe_executor::InitAdmissionWork {
					task,
					destination: *destination,
					bytes: *bytes,
					submission: *submission,
					image,
				})
			}
			Self::Calculation {
				device,
				kernel_template,
				artifact,
				submission,
				inputs,
				outputs,
				fault_flag,
			} => {
				BackendWork::Calculation(recipe_executor::CalculationWork {
					task,
					run,
					iteration,
					device: *device,
					kernel_template: *kernel_template,
					artifact: *artifact,
					submission: *submission,
					inputs,
					outputs,
					fault_flag: *fault_flag,
				})
			}
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
					WorkClass::InternalTransfer => BackendWork::InternalTransfer(transfer),
					WorkClass::ExitTransfer => BackendWork::ExitTransfer(transfer),
					WorkClass::InitAdmission | WorkClass::Calculation | WorkClass::Metric => unreachable!(),
				}
			}
			Self::Metric {
				purpose,
				metric,
				slot,
				value,
				submission,
			} => {
				BackendWork::Metric(recipe_executor::MetricWork {
					task,
					iteration,
					purpose: *purpose,
					metric: *metric,
					slot: *slot,
					value: *value,
					submission: *submission,
				})
			}
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
	initial_capacity: InitialCapacitySnapshot,
	anchored_capacity: Option<CapacityLedger>,
	partitions: Partitions,
	bridge: Bridge,
	physical: LocalPreparedPhysical<'cuda, 'hsa, BridgeResource>,
	stabilization: StabilizationState,
}

impl<Bridge: fmt::Debug, BridgeResource: fmt::Debug> fmt::Debug for LocalPreparedSession<'_, '_, Bridge, BridgeResource> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("LocalPreparedSession")
			.field("topology", &self.topology.identity)
			.field("discovery", &self.discovery.identity)
			.field("candidate", &self.candidate.draft.candidate)
			.field("artifact_count", &self.artifacts.len())
			.field("reservations", &self.reservations)
			.field("initial_capacity", &self.initial_capacity)
			.field("anchored_capacity", &self.anchored_capacity)
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
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>
{
	fn warm_maximum_concurrency(&mut self, session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>, candidate: &PlannedCandidate, pass: u32) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>>;

	fn capacity_snapshot(&mut self, session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>, topology: &Topology, discovery: &DiscoveryProfile) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableLocalStabilizer;

impl<'cuda, 'hsa, Bridge> LocalCandidateStabilizer<'cuda, 'hsa, Bridge> for UnavailableLocalStabilizer
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>
{
	fn warm_maximum_concurrency(&mut self, session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>, candidate: &PlannedCandidate, pass: u32) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> {
		consume_context((session, candidate, pass));
		Err(CandidateFailure::PreFinalRealizationUnavailable {
			detail: "native maximum-concurrency warm execution is not implemented",
		})
	}

	fn capacity_snapshot(&mut self, session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>, topology: &Topology, discovery: &DiscoveryProfile) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>> {
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
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>
{
	fn warm_maximum_concurrency(&mut self, session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>, candidate: &PlannedCandidate, pass: u32) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> { session.execute_warm_pass(candidate, pass) }

	fn capacity_snapshot(&mut self, session: &mut LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>, topology: &Topology, discovery: &DiscoveryProfile) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>> { session.observe_capacity(topology, discovery) }
}

/// Concrete pre-final local physical resource factory.
#[derive(Clone, Debug, PartialEq, Eq)]
struct InitialCapacitySnapshot {
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
	available: BTreeMap<DeviceId, ByteCount>,
}

#[derive(Debug)]
pub struct LocalCandidateFactory<'cuda, 'hsa, Bridge, Stabilizer = UnavailableLocalStabilizer> {
	host: Option<HostBackendConfig>,
	cuda_bindings: Vec<CudaBinding<'cuda>>,
	hsa_bindings: Vec<HsaBinding<'hsa>>,
	bridge: Bridge,
	stabilizer: Stabilizer,
	initial_capacity: Option<InitialCapacitySnapshot>,
}

impl<'cuda, 'hsa, Bridge, Stabilizer> LocalCandidateFactory<'cuda, 'hsa, Bridge, Stabilizer> {
	#[must_use]
	pub fn new(host: Option<HostBackendConfig>, cuda_bindings: Vec<CudaBinding<'cuda>>, hsa_bindings: Vec<HsaBinding<'hsa>>, bridge: Bridge, stabilizer: Stabilizer) -> Self {
		Self {
			host,
			cuda_bindings,
			hsa_bindings,
			bridge,
			stabilizer,
			initial_capacity: None,
		}
	}
}

impl<'cuda, 'hsa, Bridge> LocalCandidateFactory<'cuda, 'hsa, Bridge, UnavailableLocalStabilizer> {}

impl<'cuda, 'hsa, Bridge> LocalCandidateFactory<'cuda, 'hsa, Bridge, NativeLocalStabilizer> {
	/// Constructs the production factory that warms and measures the exact
	/// pre-final local resources.
	#[must_use]
	pub fn production(host: Option<HostBackendConfig>, cuda_bindings: Vec<CudaBinding<'cuda>>, hsa_bindings: Vec<HsaBinding<'hsa>>, bridge: Bridge) -> Self {
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
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa>
{
	fn execute_warm_pass(&mut self, candidate: &PlannedCandidate, pass: u32) -> Result<(), CandidateFailure<LocalError<Bridge::Error>>> {
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
			LocalPreparedPhysical::Candidate { .. } | LocalPreparedPhysical::Transition | LocalPreparedPhysical::Destroyed => {
				return Err(CandidateFailure::Fatal(LocalError::BackendState(
					"warm local resources are unavailable",
				)));
			}
		};
		if arenas.is_empty() {
			for layout in &candidate.arena_layouts {
				let arena = allocate_warm_arena(resources, layout).map_err(CandidateFailure::Fatal)?;
				let prior = arenas.insert(layout.device, arena);
				debug_assert!(prior.is_none());
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
			Some(resources) => {
				match resources.bind_candidate(&bundle, &self.partitions.host) {
					Ok(resources) => Some(resources),
					Err(error) => {
						let failure = LocalError::Host(error);
						let failure = cleanup_prepared_native_host(hsa, cuda, None, failure);
						let failure = cleanup_bridge_resource(&mut self.bridge, bridge_resource, failure);
						self.physical = LocalPreparedPhysical::Destroyed;
						return Err(CandidateFailure::Fatal(failure));
					}
				}
			}
			None => None,
		};
		let cuda = match cuda.bind_candidate(&bundle, &self.partitions.cuda) {
			Ok(resources) => resources,
			Err(error) => {
				let mut failure = LocalError::Native(error);
				failure = cleanup_error(failure, hsa.destroy().map_err(LocalError::Native));
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
				failure = cleanup_error(failure, cuda.destroy().map_err(LocalError::Native));
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
		let tasks = prepare_warm_tasks(&bundle);
		let images = prepare_warm_images(&bundle);
		let exits = prepare_warm_exits(&tasks);
		self.physical = LocalPreparedPhysical::Warm {
			_bundle: bundle,
			resources: Box::new(resources),
			arenas: BTreeMap::new(),
			tasks,
			images,
			exits,
		};
		Ok(())
	}

	fn observe_capacity(&mut self, topology: &Topology, discovery: &DiscoveryProfile) -> Result<CapacityLedger, CandidateFailure<LocalError<Bridge::Error>>> {
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
			LocalPreparedPhysical::Candidate { .. } | LocalPreparedPhysical::Transition | LocalPreparedPhysical::Destroyed => {
				return Err(CandidateFailure::Fatal(LocalError::BackendState(
					"capacity observation requires a complete physical warm trace",
				)));
			}
		};
		let warm_arenas = core::mem::take(arenas);
		release_warm_arenas(warm_arenas).map_err(CandidateFailure::Fatal)?;
		anchor_capacity_snapshot(&mut self.anchored_capacity, || {
			observe_capacity_ledger(
				topology,
				discovery,
				&self.reservations,
				&self.initial_capacity,
				resources,
			)
		})
		.map_err(CandidateFailure::Fatal)
	}

	pub fn into_backend(mut self, bundle: &FinalizedBundle) -> Result<PreparedLocalBackend<'cuda, 'hsa, Bridge>, LocalError<Bridge::Error>> {
		let validation = self.validate_handoff(bundle);
		match validation {
			Ok(()) => {
				let physical = core::mem::replace(&mut self.physical, LocalPreparedPhysical::Transition);
				let LocalPreparedPhysical::Warm {
					resources, arenas, ..
				} = physical
				else {
					return Err(LocalError::BackendState(
						"observed local session has no warmed physical resources",
					));
				};
				match arenas.is_empty() {
					true => Ok(()),
					false => {
						Err(LocalError::BackendState(
							"warm candidate arenas were not released before final handoff",
						))
					}
				}?;
				let Self {
					partitions, bridge, ..
				} = self;
				let LocalResources {
					host,
					cuda,
					hsa,
					bridge: bridge_resource,
					..
				} = *resources;
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
					native_evidence: NativeExecutionEvidence::default(),
				})
			}
			Err(validation_error) => {
				match self.destroy() {
					Ok(()) => Err(validation_error),
					Err(teardown_error) => Err(teardown_error),
				}
			}
		}
	}

	fn validate_handoff(&mut self, bundle: &FinalizedBundle) -> Result<(), LocalError<Bridge::Error>> {
		match self.stabilization {
			StabilizationState::Observed { .. } => Ok(()),
			StabilizationState::Realized | StabilizationState::Warmed { .. } => {
				Err(LocalError::BackendState(
					"candidate resources were not observed after their final warm pass",
				))
			}
		}?;
		validate_prepared_identity(&self.candidate, &self.artifacts, &self.reservations, bundle);
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
			false => {
				Err(LocalError::BackendState(
					"capacity was observed before warm candidate arenas were released",
				))
			}
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
			Some(host) => {
				host.validate_handoff(bundle, &self.partitions.host)
					.map_err(LocalError::Host)
			}
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
		let Self { mut bridge, .. } = self;
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
				first = retain_first(first, destroy_warm_resources(&mut bridge, *resources));
				match first {
					Some(error) => Err(error),
					None => Ok(()),
				}
			}
			LocalPreparedPhysical::Transition => {
				Err(LocalError::BackendState(
					"local candidate resources are already in transition",
				))
			}
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

	fn reservation_evidence(&self, device: DeviceId) -> Result<ReservationEvidence, Self::Error> {
		Ok(reservation_evidence_for_device(
			device,
			self.host.as_ref(),
			&self.cuda_bindings,
			&self.hsa_bindings,
		))
	}

	fn realize_candidate(&mut self, request: CandidateRealizationRequest<'_>) -> Result<Self::Session, CandidateFailure<Self::Error>> {
		request.validate().map_err(|error| {
			CandidateFailure::CandidateRejected {
				detail: error.to_string(),
			}
		})?;
		let declared_devices = declared_devices(self.host.as_ref(), &self.cuda_bindings, &self.hsa_bindings);
		let partitions = classify_candidate(request.candidate, &declared_devices);
		let initial_capacity = match &self.initial_capacity {
			Some(snapshot) if snapshot.topology == request.topology.identity && snapshot.discovery == request.discovery.identity => snapshot.clone(),
			Some(_) => {
				return Err(CandidateFailure::Fatal(LocalError::BackendState(
					"local factory initial capacity belongs to another topology or discovery",
				)));
			}
			None => {
				let snapshot = capture_initial_capacity::<Bridge::Error>(
					request.topology,
					request.discovery,
					&partitions.devices,
					self.host.as_ref(),
					&self.cuda_bindings,
					&self.hsa_bindings,
				)
				.map_err(CandidateFailure::Fatal)?;
				self.initial_capacity = Some(snapshot.clone());
				snapshot
			}
		};
		validate_initial_headroom(request.reservations, &initial_capacity).map_err(CandidateFailure::Fatal)?;
		let (cuda_artifacts, hsa_artifacts, identities) = partition_candidate_artifacts(request, &partitions).map_err(CandidateFailure::Fatal)?;

		let host = match self.host.clone() {
			Some(config) => {
				Some(HostBackend::prepare_candidate(
					&request.candidate.draft,
					request.reservations,
					config,
					&partitions.host,
				)
				.map_err(LocalError::Host)
				.map_err(CandidateFailure::Fatal)?)
			}
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
		let bridge_resource = match bridge.realize_candidate(request.candidate, &partitions.bridge, &partitions.devices) {
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
			initial_capacity,
			anchored_capacity: None,
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

	fn warm_maximum_concurrency(&mut self, session: &mut Self::Session, candidate: &PlannedCandidate, pass: u32) -> Result<(), CandidateFailure<Self::Error>> {
		let expected_pass = match session.stabilization {
			StabilizationState::Realized => Some(1),
			StabilizationState::Observed { pass } => pass.checked_add(1),
			StabilizationState::Warmed { .. } => None,
		};
		match expected_pass == Some(pass) && *candidate == session.candidate {
			true => Ok(()),
			false => {
				Err(CandidateFailure::CandidateRejected {
					detail: "warm trace candidate or pass order differs from the realized local session".to_owned(),
				})
			}
		}?;
		self.stabilizer
			.warm_maximum_concurrency(session, candidate, pass)?;
		session.stabilization = StabilizationState::Warmed { pass };
		Ok(())
	}

	fn capacity_snapshot(&mut self, session: &mut Self::Session, topology: &Topology, discovery: &DiscoveryProfile) -> Result<CapacityLedger, CandidateFailure<Self::Error>> {
		match topology.identity == session.candidate.draft.topology && discovery.identity == session.candidate.draft.discovery {
			true => Ok(()),
			false => {
				Err(CandidateFailure::CandidateRejected {
					detail: "capacity snapshot topology or discovery differs from the local candidate".to_owned(),
				})
			}
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

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error> { session.destroy() }
}

#[derive(Debug)]
pub struct LocalBackend<'cuda, 'hsa, Bridge> {
	host: Option<HostBackend>,
	cuda: CudaBackend<'cuda>,
	hsa: HsaBackend<'hsa>,
	bridge: Bridge,
	declared_devices: Vec<(DeviceId, LocalDeviceClass)>,
	native_evidence: NativeExecutionEvidence,
}

impl<'cuda, 'hsa, Bridge> LocalBackend<'cuda, 'hsa, Bridge> {
	#[must_use]
	pub fn new(host: Option<HostBackendConfig>, cuda_bindings: Vec<CudaBinding<'cuda>>, cuda_artifacts: Vec<RuntimeArtifact>, hsa_bindings: Vec<HsaBinding<'hsa>>, hsa_artifacts: Vec<RuntimeArtifact>, bridge: Bridge) -> Self {
		let declared_devices = declared_devices(host.as_ref(), &cuda_bindings, &hsa_bindings);
		Self {
			host: host.map(HostBackend::new),
			cuda: CudaBackend::new(cuda_bindings, cuda_artifacts),
			hsa: HsaBackend::new(hsa_bindings, hsa_artifacts),
			bridge,
			declared_devices,
			native_evidence: NativeExecutionEvidence::default(),
		}
	}

	#[must_use]
	pub const fn native_evidence(&self) -> &NativeExecutionEvidence { &self.native_evidence }
}

impl<Bridge> sealed::Sealed for LocalBackend<'_, '_, Bridge> {}

impl<'cuda, 'hsa, Bridge> Backend for LocalBackend<'cuda, 'hsa, Bridge>
where Bridge: CrossBackendTransfer<'cuda, 'hsa>
{
	type Arena = LocalArena<'cuda, 'hsa>;
	type Error = LocalError<Bridge::Error>;
	type Pending = LocalPending<'cuda, 'hsa, Bridge::Pending>;
	type Resource = LocalResources<'cuda, 'hsa, Bridge::Resource>;

	const MAX_NON_POLL_PHYSICAL_CALLS: usize = 1;

	fn bind_resources(&mut self, bundle: &FinalizedBundle, physical_calls: &mut PhysicalCallBatch) -> Result<Self::Resource, Self::Error> {
		record(physical_calls, PhysicalCall::BindResources);
		let partitions = classify(bundle, &self.declared_devices);
		let host = match &mut self.host {
			Some(host) => {
				Some(host
					.bind_partition(bundle, &partitions.host)
					.map_err(LocalError::Host)?)
			}
			None => {
				debug_assert!(partitions.host.is_empty());
				None
			}
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

	fn prepare_pending(&mut self, resource: &mut Self::Resource, request: PendingRequest, physical_calls: &mut PhysicalCallBatch) -> Result<Self::Pending, Self::Error> {
		record(physical_calls, PhysicalCall::PreparePending {
			task: request.task,
		});
		match resource.tasks[&request.task] {
			TaskOwner::Host => {
				let host_resource = resource.host.as_mut().unwrap();
				self.host
					.as_mut()
					.unwrap()
					.prepare_partition(host_resource, request)
					.map(LocalPending::Host)
					.map_err(LocalError::Host)
			}
			TaskOwner::Cuda => {
				resource
					.cuda
					.prepare_pending(request)
					.map(LocalPending::Cuda)
					.map_err(LocalError::Native)
			}
			TaskOwner::Hsa => {
				resource
					.hsa
					.prepare_pending(request)
					.map(LocalPending::Hsa)
					.map_err(LocalError::Native)
			}
			TaskOwner::Bridge => {
				self.bridge
					.prepare_pending(&mut resource.bridge, request)
					.map(|pending| {
						LocalPending::Bridge {
							task: request.task,
							pending,
						}
					})
					.map_err(LocalError::Bridge)
			}
		}
	}

	fn allocate_arena(&mut self, resource: &mut Self::Resource, layout: &ArenaLayout, physical_calls: &mut PhysicalCallBatch) -> Result<Self::Arena, Self::Error> {
		record(physical_calls, PhysicalCall::AllocateArena {
			device: layout.device,
			bytes: layout.size,
		});
		match resource.devices[&layout.device] {
			LocalDeviceClass::Host => {
				let host_resource = resource.host.as_mut().unwrap();
				self.host
					.as_mut()
					.unwrap()
					.allocate_partition(host_resource, layout)
					.map(LocalArena::Host)
					.map_err(LocalError::Host)
			}
			LocalDeviceClass::Cuda => {
				resource
					.cuda
					.allocate_arena(layout)
					.map(LocalArena::Cuda)
					.map_err(LocalError::Native)
			}
			LocalDeviceClass::Hsa => {
				resource
					.hsa
					.allocate_arena(layout)
					.map(LocalArena::Hsa)
					.map_err(LocalError::Native)
			}
		}
	}

	fn supports_loop_repetition(&self) -> bool { self.bridge.supports_loop_repetition() }

	fn supports_same_queue_pipelining(&self, resource: &Self::Resource, task: TaskId) -> bool { resource.tasks.get(&task) == Some(&TaskOwner::Cuda) }

	fn submit(&mut self, resource: &mut Self::Resource, arenas: ArenaSet<'_, Self::Arena>, pending: &mut Self::Pending, work: BackendWork<'_>, physical_calls: &mut PhysicalCallBatch) -> Result<(), Self::Error> {
		record(physical_calls, crate::accounting::submission_call(&work));
		let task = work.task();
		let lookup = ProjectedArenas { arenas: &arenas };
		match pending {
			LocalPending::Host(pending) => {
				ensure_owner(resource, task, TaskOwner::Host);
				let host_resource = resource.host.as_mut().unwrap();
				self.host
					.as_mut()
					.unwrap()
					.submit_partition(host_resource, &lookup, pending, work)
					.map_err(LocalError::Host)
			}
			LocalPending::Cuda(pending) => {
				ensure_owner(resource, task, TaskOwner::Cuda);
				resource
					.cuda
					.submit(&lookup, pending, work)
					.map_err(LocalError::Native)
			}
			LocalPending::Hsa(pending) => {
				ensure_owner(resource, task, TaskOwner::Hsa);
				resource
					.hsa
					.submit(&lookup, pending, work)
					.map_err(LocalError::Native)
			}
			LocalPending::Bridge {
				task: pending_task,
				pending,
			} => {
				ensure_owner(resource, task, TaskOwner::Bridge);
				debug_assert_eq!(*pending_task, task);
				let (class, transfer) = match work {
					BackendWork::InternalTransfer(work) => (WorkClass::InternalTransfer, work),
					BackendWork::ExitTransfer(work) => (WorkClass::ExitTransfer, work),
					BackendWork::InitAdmission(_) | BackendWork::Calculation(_) | BackendWork::Metric(_) => {
						unreachable!()
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

	fn submit_loop_iteration(&mut self, resource: &mut Self::Resource, arenas: ArenaSet<'_, Self::Arena>, pending: &mut Self::Pending, _iteration: LoopIteration, work: BackendWork<'_>, physical_calls: &mut PhysicalCallBatch) -> Result<(), Self::Error> {
		let task = work.task();
		match pending {
			LocalPending::Host(pending) => {
				ensure_owner(resource, task, TaskOwner::Host);
				resource
					.host
					.as_mut()
					.unwrap()
					.prepare_loop_pending(pending)
					.map_err(LocalError::Host)?;
			}
			LocalPending::Cuda(pending) => {
				ensure_owner(resource, task, TaskOwner::Cuda);
				resource
					.cuda
					.prepare_loop_pending(pending)
					.map_err(LocalError::Native)?;
			}
			LocalPending::Hsa(pending) => {
				ensure_owner(resource, task, TaskOwner::Hsa);
				resource
					.hsa
					.prepare_loop_pending(pending)
					.map_err(LocalError::Native)?;
			}
			LocalPending::Bridge {
				task: pending_task,
				pending,
			} => {
				ensure_owner(resource, task, TaskOwner::Bridge);
				debug_assert_eq!(*pending_task, task);
				self.bridge
					.rearm_loop_pending(&mut resource.bridge, pending)
					.map_err(LocalError::Bridge)?;
			}
		}
		self.submit(resource, arenas, pending, work, physical_calls)
	}

	fn poll(&mut self, resource: &mut Self::Resource, pending: &mut Self::Pending, physical_calls: &mut PhysicalCallBatch) -> Result<BackendPoll, Self::Error> {
		let task = pending.task();
		let result = match pending {
			LocalPending::Host(pending) => {
				let host_resource = resource.host.as_mut().unwrap();
				self.host
					.as_mut()
					.unwrap()
					.poll_partition(host_resource, pending)
					.map_err(LocalError::Host)
			}
			LocalPending::Cuda(pending) => resource.cuda.poll(pending).map_err(LocalError::Native),
			LocalPending::Hsa(pending) => {
				resource
					.hsa
					.poll_pending(pending)
					.map_err(LocalError::Native)
			}
			LocalPending::Bridge { pending, .. } => {
				self.bridge
					.poll(&mut resource.bridge, pending)
					.map_err(LocalError::Bridge)
			}
		};
		let status = match &result {
			Ok(BackendPoll::Pending) => PhysicalPollStatus::Pending,
			Ok(BackendPoll::Complete { .. }) => PhysicalPollStatus::Complete,
			Err(LocalError::CapacityMismatch { .. } | LocalError::BackendState(_) | LocalError::Host(_) | LocalError::Native(_) | LocalError::Bridge(_)) => PhysicalPollStatus::Failed,
		};
		record(
			physical_calls,
			crate::accounting::completion_poll_call(task, status),
		);
		result
	}

	fn collect_exit(&mut self, resource: &mut Self::Resource, arenas: ArenaSet<'_, Self::Arena>, pending: &mut Self::Pending, work: TransferWork<'_>, destination: &mut [u8], physical_calls: &mut PhysicalCallBatch) -> Result<(), Self::Error> {
		record(physical_calls, PhysicalCall::CollectExit {
			task: work.task,
			bytes: work.bytes,
		});
		let lookup = ProjectedArenas { arenas: &arenas };
		match pending {
			LocalPending::Host(pending) => {
				let host_resource = resource.host.as_mut().unwrap();
				self.host
					.as_mut()
					.unwrap()
					.collect_partition(host_resource, pending, work, destination)
					.map_err(LocalError::Host)
			}
			LocalPending::Cuda(pending) => {
				resource
					.cuda
					.collect_exit(pending, work, destination)
					.map_err(LocalError::Native)
			}
			LocalPending::Hsa(pending) => {
				resource
					.hsa
					.collect_exit(&lookup, pending, work, destination)
					.map_err(LocalError::Native)
			}
			LocalPending::Bridge { .. } => unreachable!("finalized external exit has one backend owner"),
		}
	}

	fn release_arena(&mut self, resource: &mut Self::Resource, device: DeviceId, arena: Self::Arena, physical_calls: &mut PhysicalCallBatch) -> Result<(), Self::Error> {
		record(physical_calls, PhysicalCall::ReleaseArena { device });
		let expected = resource.devices[&device];
		debug_assert!(arena.device() == device && arena.class() == expected);
		match arena {
			LocalArena::Host(arena) => {
				let host_resource = resource.host.as_mut().unwrap();
				self.host
					.as_mut()
					.unwrap()
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

	fn destroy_resources(&mut self, resource: Self::Resource, physical_calls: &mut PhysicalCallBatch) -> Result<(), Self::Error> {
		record(physical_calls, PhysicalCall::DestroyResources);
		let mut device_evidence = resource.cuda.execution_evidence();
		device_evidence.extend(resource.hsa.execution_evidence());
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
			Some(host_resource) => {
				match &mut self.host {
					Some(host) => {
						host.destroy_partition(host_resource)
							.map_err(LocalError::Host)
					}
					None => {
						Err(LocalError::BackendState(
							"bound host resources outlived the host partition",
						))
					}
				}
			}
			None => Ok(()),
		};
		first = retain_first(first, host_result);
		match first {
			Some(error) => Err(error),
			None => {
				self.native_evidence = NativeExecutionEvidence::completed(device_evidence);
				Ok(())
			}
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

fn reservation_evidence_for_device(device: DeviceId, host: Option<&HostBackendConfig>, cuda_bindings: &[CudaBinding<'_>], hsa_bindings: &[HsaBinding<'_>]) -> ReservationEvidence {
	let mut evidence = None;
	for binding in host
		.into_iter()
		.flat_map(HostBackendConfig::bindings)
		.filter(|binding| binding.device() == device)
	{
		let _ = binding;
		let prior = evidence.replace(ReservationEvidence::NonGpu);
		debug_assert!(prior.is_none());
	}
	for binding in cuda_bindings
		.iter()
		.filter(|binding| binding.device() == device)
	{
		let candidate = ReservationEvidence::GpuDisplay {
			enabled_connectors: binding.enabled_display_connectors(),
		};
		let prior = evidence.replace(candidate);
		debug_assert!(prior.is_none());
	}
	for binding in hsa_bindings
		.iter()
		.filter(|binding| binding.device() == device)
	{
		let candidate = ReservationEvidence::GpuDisplay {
			enabled_connectors: binding.enabled_display_connectors(),
		};
		let prior = evidence.replace(candidate);
		debug_assert!(prior.is_none());
	}
	evidence.unwrap()
}

fn capture_initial_capacity<BridgeError>(topology: &Topology, discovery: &DiscoveryProfile, devices: &BTreeMap<DeviceId, LocalDeviceClass>, host: Option<&HostBackendConfig>, cuda_bindings: &[CudaBinding<'_>], hsa_bindings: &[HsaBinding<'_>]) -> Result<InitialCapacitySnapshot, LocalError<BridgeError>> {
	let mut available = BTreeMap::new();
	for device in &topology.devices {
		let bytes = match devices[&device.id] {
			LocalDeviceClass::Host => {
				host.unwrap()
					.available_bytes(device.id)
					.map_err(LocalError::Host)?
			}
			LocalDeviceClass::Cuda => {
				cuda_bindings
					.iter()
					.find(|binding| binding.device() == device.id)
					.unwrap()
					.available_bytes()
					.map_err(LocalError::Native)?
			}
			LocalDeviceClass::Hsa => {
				hsa_bindings
					.iter()
					.find(|binding| binding.device() == device.id)
					.unwrap()
					.available_bytes()
					.map_err(LocalError::Native)?
			}
		};
		let prior = available.insert(device.id, bytes);
		debug_assert!(prior.is_none());
	}
	Ok(InitialCapacitySnapshot {
		topology: topology.identity,
		discovery: discovery.identity,
		available,
	})
}

fn validate_initial_headroom<BridgeError>(reservations: &ReservationLedger, initial: &InitialCapacitySnapshot) -> Result<(), LocalError<BridgeError>> {
	for (device, available) in &initial.available {
		let reservation = reservations.entry(*device).unwrap();
		if *available < reservation.bytes {
			return Err(LocalError::CapacityMismatch {
				device: *device,
				detail: "initial available bytes are smaller than required user headroom",
			});
		}
	}
	Ok(())
}

fn declared_devices(host: Option<&HostBackendConfig>, cuda_bindings: &[CudaBinding<'_>], hsa_bindings: &[HsaBinding<'_>]) -> Vec<(DeviceId, LocalDeviceClass)> {
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

fn classify_candidate(candidate: &PlannedCandidate, declared: &[(DeviceId, LocalDeviceClass)]) -> Partitions {
	let devices = validate_device_owners(
		candidate
			.arena_layouts
			.iter()
			.map(|layout| layout.device)
			.collect(),
		declared,
	);
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
			TaskKind::Calculation(calculation) => {
				match result.devices[&calculation.device] {
					LocalDeviceClass::Cuda => TaskOwner::Cuda,
					LocalDeviceClass::Hsa => TaskOwner::Hsa,
					LocalDeviceClass::Host => unreachable!(),
				}
			}
			TaskKind::Metric(metric) => {
				let device = values[&metric.value];
				class_owner(result.devices[&device])
			}
			TaskKind::Transfer(transfer) => candidate_transfer_owner(transfer.source, transfer.destination, &result.devices),
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
				debug_assert!(matches!(&task.kind, TaskKind::Transfer(transfer) if transfer.route.len() == 1));
				result.bridge.insert(task.id);
			}
		}
	}
	result
}

fn partition_candidate_artifacts<BridgeError>(request: CandidateRealizationRequest<'_>, partitions: &Partitions) -> Result<PartitionedArtifacts, LocalError<BridgeError>> {
	let mut cuda_ids = BTreeSet::new();
	let mut hsa_ids = BTreeSet::new();
	for task in &request.candidate.draft.tasks {
		let TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		match partitions.tasks[&task.id] {
			TaskOwner::Cuda => {
				cuda_ids.insert(calculation.artifact);
			}
			TaskOwner::Hsa => {
				hsa_ids.insert(calculation.artifact);
			}
			TaskOwner::Host | TaskOwner::Bridge => unreachable!(),
		}
	}
	match cuda_ids.is_disjoint(&hsa_ids) {
		true => Ok(()),
		false => {
			Err(LocalError::BackendState(
				"one runtime artifact is assigned to both native GPU backends",
			))
		}
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

fn candidate_transfer_owner(source: TransferEndpoint, destination: TransferEndpoint, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> TaskOwner {
	match (source, destination) {
		(TransferEndpoint::External, TransferEndpoint::Device { device, .. }) | (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => class_owner(devices[&device]),
		(
			TransferEndpoint::Device { device: source, .. },
			TransferEndpoint::Device {
				device: destination,
				..
			},
		) => device_transfer_owner(source, destination, devices),
		(TransferEndpoint::External, TransferEndpoint::External) => unreachable!(),
	}
}

fn validate_device_owners(required: BTreeSet<DeviceId>, declared: &[(DeviceId, LocalDeviceClass)]) -> BTreeMap<DeviceId, LocalDeviceClass> {
	let mut devices = BTreeMap::new();
	for (device, class) in declared {
		let prior = devices.insert(*device, *class);
		debug_assert!(prior.is_none());
	}
	debug_assert!(required.iter().all(|device| devices.contains_key(device)));
	debug_assert!(devices.keys().all(|device| required.contains(device)));
	devices
}

fn device_transfer_owner(source: DeviceId, destination: DeviceId, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> TaskOwner {
	let source_class = devices[&source];
	let destination_class = devices[&destination];
	match (source_class, destination_class) {
		(LocalDeviceClass::Host, LocalDeviceClass::Host) => TaskOwner::Host,
		(LocalDeviceClass::Hsa, LocalDeviceClass::Hsa) => TaskOwner::Hsa,
		(LocalDeviceClass::Cuda, LocalDeviceClass::Cuda) => {
			match source == destination {
				true => TaskOwner::Cuda,
				false => TaskOwner::Bridge,
			}
		}
		(LocalDeviceClass::Host, LocalDeviceClass::Cuda | LocalDeviceClass::Hsa) | (LocalDeviceClass::Cuda, LocalDeviceClass::Host | LocalDeviceClass::Hsa) | (LocalDeviceClass::Hsa, LocalDeviceClass::Host | LocalDeviceClass::Cuda) => TaskOwner::Bridge,
	}
}

fn provisional_warm_bundle(topology: &Topology, discovery: &DiscoveryProfile, candidate: &PlannedCandidate, artifacts: &[ArtifactIdentity], reservations: &ReservationLedger) -> Result<FinalizedBundle, String> {
	let mut entries = Vec::with_capacity(topology.devices.len());
	for device in &topology.devices {
		let discovered = discovery.device(device.id).unwrap();
		let reservation = reservations.entry(device.id).unwrap();
		let usable = discovered
			.total_capacity
			.value
			.checked_sub(reservation.bytes)
			.unwrap();
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

fn prepare_warm_tasks(bundle: &FinalizedBundle) -> Vec<WarmTask> {
	bundle.tasks()
		.iter()
		.map(|task| {
			let work = match (&task.kind, task.phase) {
				(TaskKind::Calculation(calculation), RunPhase::Loop) => {
					WarmWork::Calculation {
						device: calculation.device,
						kernel_template: calculation.kernel_template,
						artifact: calculation.artifact,
						submission: calculation.submission,
						inputs: resolve_warm_values(bundle, &calculation.inputs),
						outputs: resolve_warm_values(bundle, &calculation.outputs),
						fault_flag: calculation
							.fault_flag
							.map(|value| resolve_warm_value(bundle, value)),
					}
				}
				(TaskKind::Metric(metric), RunPhase::Loop) => {
					WarmWork::Metric {
						purpose: metric.purpose,
						metric: metric.metric,
						slot: metric.slot,
						value: resolve_warm_value(bundle, metric.value),
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
						unreachable!();
					};
					let endpoints = bundle.transfer_endpoints(task.id).unwrap();
					let ResolvedTransferEndpoint::Device(destination) = endpoints.destination else {
						unreachable!();
					};
					WarmWork::Init {
						device,
						destination,
						bytes: transfer.bytes,
						submission: transfer.submission,
					}
				}
				(TaskKind::Transfer(transfer), RunPhase::Init | RunPhase::Loop) => {
					let endpoints = bundle.transfer_endpoints(task.id).unwrap();
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
					let endpoints = bundle.transfer_endpoints(task.id).unwrap();
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
					unreachable!()
				}
			};
			WarmTask {
				id: task.id,
				phase: task.phase,
				window: task.window,
				dependencies: task.dependencies.clone(),
				work,
			}
		})
		.collect()
}

fn resolve_warm_values(bundle: &FinalizedBundle, values: &[recipe_core::ValueId]) -> Vec<recipe_core::ResolvedValueLocation> {
	values.iter()
		.map(|value| resolve_warm_value(bundle, *value))
		.collect()
}

fn resolve_warm_value(bundle: &FinalizedBundle, value: recipe_core::ValueId) -> recipe_core::ResolvedValueLocation { bundle.value_location(value).copied().unwrap() }

fn prepare_warm_images(bundle: &FinalizedBundle) -> BTreeMap<DeviceId, Vec<u8>> {
	bundle.init_images()
		.iter()
		.map(|manifest| {
			let bytes = usize::try_from(manifest.bytes.get()).unwrap();
			(manifest.device, vec![0; bytes])
		})
		.collect()
}

fn prepare_warm_exits(tasks: &[WarmTask]) -> BTreeMap<TaskId, Vec<u8>> {
	tasks.iter()
		.filter_map(|task| {
			task.work
				.external_exit_bytes()
				.map(|bytes| (task.id, bytes))
		})
		.map(|(task, bytes)| {
			let bytes = usize::try_from(bytes.get()).unwrap();
			(task, vec![0; bytes])
		})
		.collect()
}

fn allocate_warm_arena<'cuda, 'hsa, BridgeResource, BridgeError>(resources: &mut LocalResources<'cuda, 'hsa, BridgeResource>, layout: &ArenaLayout) -> Result<LocalArena<'cuda, 'hsa>, LocalError<BridgeError>> {
	match resources.devices[&layout.device] {
		LocalDeviceClass::Host => {
			resources
				.host
				.as_ref()
				.unwrap()
				.allocate_arena(layout)
				.map(LocalArena::Host)
				.map_err(LocalError::Host)
		}
		LocalDeviceClass::Cuda => {
			resources
				.cuda
				.allocate_arena(layout)
				.map(LocalArena::Cuda)
				.map_err(LocalError::Native)
		}
		LocalDeviceClass::Hsa => {
			resources
				.hsa
				.allocate_arena(layout)
				.map(LocalArena::Hsa)
				.map_err(LocalError::Native)
		}
	}
}

fn run_warm_trace<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>, arenas: &BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>, tasks: &[WarmTask], images: &BTreeMap<DeviceId, Vec<u8>>, exits: &mut BTreeMap<TaskId, Vec<u8>>, pass: u32) -> Result<(), LocalError<Bridge::Error>>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
	const INITIAL_IDLE_DELAY: std::time::Duration = std::time::Duration::from_micros(50);
	const MAXIMUM_IDLE_DELAY: std::time::Duration = std::time::Duration::from_millis(2);

	let run = recipe_core::RunId::new(u64::from(pass));
	let iteration = recipe_core::LoopIterations::ONE.iteration(0).unwrap();
	let mut states = vec![WarmTaskState::Remaining; tasks.len()];
	let mut completed = BTreeSet::new();
	let mut pending = BTreeMap::new();
	for phase in [RunPhase::Init, RunPhase::Loop, RunPhase::Exit] {
		let mut idle_polls = 0_u32;
		let mut idle_delay = INITIAL_IDLE_DELAY;
		loop {
			let mut progressed = false;
			for (index, task) in tasks.iter().enumerate() {
				let runnable = task.phase == phase
					&& states[index] == WarmTaskState::Remaining
					&& task
						.dependencies
						.iter()
						.all(|dependency| completed.contains(dependency))
					&& tasks
						.iter()
						.enumerate()
						.all(|(active_index, active)| states[active_index] != WarmTaskState::Pending || active.window.overlaps(task.window));
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
						let work = task.work.backend_work(task.id, run, iteration, images);
						submit_warm_work(bridge, resources, arenas, &mut token, work)?;
						let prior = pending.insert(task.id, token);
						debug_assert!(prior.is_none());
						states[index] = WarmTaskState::Pending;
						progressed = true;
					}
				}
			}
			for (index, task) in tasks.iter().enumerate() {
				if task.phase != phase || states[index] != WarmTaskState::Pending {
					continue;
				}
				let poll = {
					let token = pending.get_mut(&task.id).unwrap();
					poll_warm_pending(bridge, resources, token)?
				};
				match poll {
					BackendPoll::Pending => {}
					BackendPoll::Complete { .. } => {
						let mut token = pending.remove(&task.id).unwrap();
						if let Some(destination) = exits.get_mut(&task.id) {
							let work = task.work.backend_work(task.id, run, iteration, images);
							let BackendWork::ExitTransfer(transfer) = work else {
								unreachable!()
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
			if !remaining {
				break;
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
						false => {
							std::thread::sleep(idle_delay);
							idle_delay = idle_delay.saturating_mul(2).min(MAXIMUM_IDLE_DELAY);
						}
					}
				}
				(true, false | true) => {
					idle_polls = 0;
					idle_delay = INITIAL_IDLE_DELAY;
				}
			}
		}
	}
	Ok(())
}

fn prepare_warm_pending<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>, request: PendingRequest) -> Result<LocalPending<'cuda, 'hsa, Bridge::Pending>, LocalError<Bridge::Error>>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
	match resources.tasks.get(&request.task).copied() {
		Some(TaskOwner::Host) => {
			resources
				.host
				.as_mut()
				.unwrap()
				.prepare_pending(request)
				.map(LocalPending::Host)
				.map_err(LocalError::Host)
		}
		Some(TaskOwner::Cuda) => {
			resources
				.cuda
				.prepare_pending(request)
				.map(LocalPending::Cuda)
				.map_err(LocalError::Native)
		}
		Some(TaskOwner::Hsa) => {
			resources
				.hsa
				.prepare_pending(request)
				.map(LocalPending::Hsa)
				.map_err(LocalError::Native)
		}
		Some(TaskOwner::Bridge) => {
			bridge.prepare_pending(&mut resources.bridge, request)
				.map(|pending| {
					LocalPending::Bridge {
						task: request.task,
						pending,
					}
				})
				.map_err(LocalError::Bridge)
		}
		None => unreachable!("warm request names a finalized task"),
	}
}

fn submit_warm_work<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>, arenas: &BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>, pending: &mut LocalPending<'cuda, 'hsa, Bridge::Pending>, work: BackendWork<'_>) -> Result<(), LocalError<Bridge::Error>>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
	let task = work.task();
	let arena_set = ArenaSet::new(arenas);
	let lookup = ProjectedArenas { arenas: &arena_set };
	match pending {
		LocalPending::Host(pending) => {
			ensure_owner(resources, task, TaskOwner::Host);
			resources
				.host
				.as_mut()
				.unwrap()
				.submit(&lookup, pending, work)
				.map_err(LocalError::Host)
		}
		LocalPending::Cuda(pending) => {
			ensure_owner(resources, task, TaskOwner::Cuda);
			resources
				.cuda
				.submit(&lookup, pending, work)
				.map_err(LocalError::Native)
		}
		LocalPending::Hsa(pending) => {
			ensure_owner(resources, task, TaskOwner::Hsa);
			resources
				.hsa
				.submit(&lookup, pending, work)
				.map_err(LocalError::Native)
		}
		LocalPending::Bridge {
			task: pending_task,
			pending,
		} => {
			ensure_owner(resources, task, TaskOwner::Bridge);
			debug_assert_eq!(*pending_task, task);
			let (class, transfer) = match work {
				BackendWork::InternalTransfer(transfer) => (WorkClass::InternalTransfer, transfer),
				BackendWork::ExitTransfer(transfer) => (WorkClass::ExitTransfer, transfer),
				BackendWork::InitAdmission(_) | BackendWork::Calculation(_) | BackendWork::Metric(_) => {
					unreachable!()
				}
			};
			bridge.submit(
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

fn poll_warm_pending<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>, pending: &mut LocalPending<'cuda, 'hsa, Bridge::Pending>) -> Result<BackendPoll, LocalError<Bridge::Error>>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
	match pending {
		LocalPending::Host(pending) => {
			resources
				.host
				.as_mut()
				.unwrap()
				.poll_pending(pending)
				.map_err(LocalError::Host)
		}
		LocalPending::Cuda(pending) => resources.cuda.poll(pending).map_err(LocalError::Native),
		LocalPending::Hsa(pending) => {
			resources
				.hsa
				.poll_pending(pending)
				.map_err(LocalError::Native)
		}
		LocalPending::Bridge { pending, .. } => {
			bridge.poll(&mut resources.bridge, pending)
				.map_err(LocalError::Bridge)
		}
	}
}

fn collect_warm_exit<'cuda, 'hsa, BridgeResource, BridgePending, BridgeError>(resources: &mut LocalResources<'cuda, 'hsa, BridgeResource>, arenas: &BTreeMap<DeviceId, LocalArena<'cuda, 'hsa>>, pending: &mut LocalPending<'cuda, 'hsa, BridgePending>, work: TransferWork<'_>, destination: &mut [u8]) -> Result<(), LocalError<BridgeError>> {
	let arena_set = ArenaSet::new(arenas);
	let lookup = ProjectedArenas { arenas: &arena_set };
	match pending {
		LocalPending::Host(pending) => {
			resources
				.host
				.as_ref()
				.unwrap()
				.collect_exit(pending, work, destination)
				.map_err(LocalError::Host)
		}
		LocalPending::Cuda(pending) => {
			resources
				.cuda
				.collect_exit(pending, work, destination)
				.map_err(LocalError::Native)
		}
		LocalPending::Hsa(pending) => {
			resources
				.hsa
				.collect_exit(&lookup, pending, work, destination)
				.map_err(LocalError::Native)
		}
		LocalPending::Bridge { .. } => unreachable!("finalized warm exit has one backend owner"),
	}
}

fn recycle_warm_pending<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resources: &mut LocalResources<'cuda, 'hsa, Bridge::Resource>, pending: LocalPending<'cuda, 'hsa, Bridge::Pending>) -> Result<(), LocalError<Bridge::Error>>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
	match pending {
		LocalPending::Host(pending) => {
			resources
				.host
				.as_mut()
				.unwrap()
				.recycle_pending(pending)
				.map_err(LocalError::Host)
		}
		LocalPending::Cuda(pending) => {
			resources
				.cuda
				.recycle_pending(pending)
				.map_err(LocalError::Native)
		}
		LocalPending::Hsa(pending) => {
			resources
				.hsa
				.recycle_pending(pending)
				.map_err(LocalError::Native)
		}
		LocalPending::Bridge { pending, .. } => {
			bridge.recycle_candidate_pending(&mut resources.bridge, pending)
				.map_err(LocalError::Bridge)
		}
	}
}

fn release_warm_arenas<BridgeError>(arenas: BTreeMap<DeviceId, LocalArena<'_, '_>>) -> Result<(), LocalError<BridgeError>> {
	let mut first = None;
	for (device, arena) in arenas {
		let result = match arena {
			LocalArena::Host(arena) => arena.close().map_err(LocalError::Host),
			LocalArena::Cuda(arena) => arena.release().map_err(LocalError::Native),
			LocalArena::Hsa(arena) => arena.release().map_err(LocalError::Native),
		};
		match result {
			Ok(()) => {}
			Err(error) => {
				match first {
					Some(_) => drop(error),
					None => first = Some(error),
				}
			}
		}
		consume_context(device);
	}
	match first {
		Some(error) => Err(error),
		None => Ok(()),
	}
}

fn destroy_warm_resources<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resources: LocalResources<'cuda, 'hsa, Bridge::Resource>) -> Result<(), LocalError<Bridge::Error>>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
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

/// Anchor one conservative session quota after the first complete warm
/// allocate/execute/free cycle. That observation includes persistent
/// Recipe-owned modules, queues, staging, driver allocations, and one
/// production-representative arena cycle. Later system/display or allocator
/// counter drift cannot rewrite the finalized scheduler contract.
fn anchor_capacity_snapshot<E>(anchored: &mut Option<CapacityLedger>, observe: impl FnOnce() -> Result<CapacityLedger, E>) -> Result<CapacityLedger, E> {
	if let Some(snapshot) = anchored {
		return Ok(snapshot.clone());
	}
	let snapshot = observe()?;
	*anchored = Some(snapshot.clone());
	Ok(snapshot)
}

fn observe_capacity_ledger<BridgeError, BridgeResource>(topology: &Topology, discovery: &DiscoveryProfile, reservations: &ReservationLedger, initial_capacity: &InitialCapacitySnapshot, resources: &LocalResources<'_, '_, BridgeResource>) -> Result<CapacityLedger, LocalError<BridgeError>> {
	if initial_capacity.topology != topology.identity || initial_capacity.discovery != discovery.identity {
		return Err(LocalError::BackendState(
			"capacity observation differs from its immutable init snapshot",
		));
	}
	let mut entries = Vec::with_capacity(topology.devices.len());
	for device in &topology.devices {
		debug_assert!(discovery.device(device.id).is_some());
		let initial = initial_capacity.available.get(&device.id).copied().unwrap();
		let reservation = reservations.entry(device.id).unwrap();
		let available = match resources.devices[&device.id] {
			LocalDeviceClass::Host => {
				resources
					.host
					.as_ref()
					.unwrap()
					.available_bytes(device.id)
					.map_err(LocalError::Host)?
			}
			LocalDeviceClass::Cuda => {
				resources
					.cuda
					.available_bytes(device.id)
					.map_err(LocalError::Native)?
			}
			LocalDeviceClass::Hsa => {
				resources
					.hsa
					.available_bytes(device.id)
					.map_err(LocalError::Native)?
			}
		};
		let (overhead, usable) = account_live_capacity(initial, reservation.bytes, available).ok_or(LocalError::CapacityMismatch {
			device: device.id,
			detail: "live available bytes fell below required user headroom",
		})?;
		entries.push(CapacityLedgerEntry {
			device: device.id,
			total: Property::new(initial, PropertyProvenance::Measured),
			runtime_overhead: Property::new(overhead, PropertyProvenance::Measured),
			fragmentation: Property::new(ByteCount::ZERO, PropertyProvenance::Measured),
			safety_headroom: Property::new(ByteCount::ZERO, PropertyProvenance::Measured),
			recipe_usable: Property::new(usable, PropertyProvenance::Measured),
		});
	}
	Ok(CapacityLedger { entries })
}

fn account_live_capacity(initial: ByteCount, headroom: ByteCount, live: ByteCount) -> Option<(ByteCount, ByteCount)> {
	let capped_live = live.min(initial);
	let overhead = initial.checked_sub(capped_live)?;
	let usable = capped_live.checked_sub(headroom)?;
	Some((overhead, usable))
}

fn cleanup_bridge_resource<'cuda, 'hsa, Bridge>(bridge: &mut Bridge, resource: Bridge::Resource, error: LocalError<Bridge::Error>) -> LocalError<Bridge::Error>
where Bridge: CandidateCrossBackendTransfer<'cuda, 'hsa> {
	cleanup_error(error, bridge.destroy(resource).map_err(LocalError::Bridge))
}

fn cleanup_warm_host<BridgeError>(host: Option<HostResources>, error: LocalError<BridgeError>) -> LocalError<BridgeError> {
	let result = match host {
		Some(host) => host.destroy().map_err(LocalError::Host),
		None => Ok(()),
	};
	cleanup_error(error, result)
}

fn cleanup_prepared_host<BridgeError>(host: Option<HostPreparedResources>, error: LocalError<BridgeError>) -> LocalError<BridgeError> {
	let host_result = match host {
		Some(resources) => resources.destroy().map_err(LocalError::Host),
		None => Ok(()),
	};
	cleanup_error(error, host_result)
}

fn cleanup_prepared_cuda_host<BridgeError>(cuda: CudaPreparedResources<'_>, host: Option<HostPreparedResources>, error: LocalError<BridgeError>) -> LocalError<BridgeError> {
	let first = cleanup_error(error, cuda.destroy().map_err(LocalError::Native));
	cleanup_prepared_host(host, first)
}

fn cleanup_prepared_native_host<BridgeError>(hsa: HsaPreparedResources<'_>, cuda: CudaPreparedResources<'_>, host: Option<HostPreparedResources>, error: LocalError<BridgeError>) -> LocalError<BridgeError> {
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

fn validate_prepared_identity(candidate: &PlannedCandidate, artifacts: &[ArtifactIdentity], reservations: &ReservationLedger, bundle: &FinalizedBundle) {
	debug_assert!(bundle.topology() == candidate.draft.topology && bundle.discovery() == candidate.draft.discovery && bundle.draft() == candidate.draft.identity && bundle.candidate() == candidate.draft.candidate);
	debug_assert!(bundle.tasks() == candidate.draft.tasks && bundle.kernels() == candidate.draft.kernels && bundle.artifact_builds() == candidate.draft.artifact_builds && bundle.resources() == &candidate.draft.resources && bundle.init_images() == candidate.draft.init_images && bundle.reservations() == reservations);
	let prepared_artifacts = artifacts
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	let finalized_artifacts = bundle
		.artifacts()
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	debug_assert_eq!(prepared_artifacts, finalized_artifacts);
}

fn classify(bundle: &FinalizedBundle, declared: &[(DeviceId, LocalDeviceClass)]) -> Partitions {
	let finalized_devices = bundle
		.arena_layouts()
		.iter()
		.map(|layout| layout.device)
		.collect::<BTreeSet<_>>();
	let devices = validate_device_owners(finalized_devices, declared);

	let mut result = Partitions {
		devices,
		tasks: BTreeMap::new(),
		host: BTreeSet::new(),
		cuda: BTreeSet::new(),
		hsa: BTreeSet::new(),
		bridge: BTreeSet::new(),
	};
	for task in bundle.tasks() {
		let owner = match &task.kind {
			TaskKind::Calculation(calculation) => {
				match result.devices[&calculation.device] {
					LocalDeviceClass::Cuda => TaskOwner::Cuda,
					LocalDeviceClass::Hsa => TaskOwner::Hsa,
					LocalDeviceClass::Host => unreachable!(),
				}
			}
			TaskKind::Metric(metric) => {
				let location = bundle.value_location(metric.value).unwrap();
				class_owner(result.devices[&location.device])
			}
			TaskKind::Transfer(_) => {
				let endpoints = bundle.transfer_endpoints(task.id).unwrap();
				transfer_owner(endpoints.source, endpoints.destination, &result.devices)
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
				debug_assert!(matches!(&task.kind, TaskKind::Transfer(transfer) if transfer.route.len() == 1));
				result.bridge.insert(task.id);
			}
		}
	}
	result
}

fn transfer_owner(source: ResolvedTransferEndpoint, destination: ResolvedTransferEndpoint, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> TaskOwner {
	match (source, destination) {
		(ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::Device(destination)) => class_owner(devices[&destination.device]),
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::External) => class_owner(devices[&source.device]),
		(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => device_transfer_owner(source.device, destination.device, devices),
		(ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::External) => unreachable!(),
	}
}

const fn class_owner(class: LocalDeviceClass) -> TaskOwner {
	match class {
		LocalDeviceClass::Host => TaskOwner::Host,
		LocalDeviceClass::Cuda => TaskOwner::Cuda,
		LocalDeviceClass::Hsa => TaskOwner::Hsa,
	}
}

fn ensure_owner<BridgeResource>(resource: &LocalResources<'_, '_, BridgeResource>, task: TaskId, expected: TaskOwner) {
	debug_assert_eq!(resource.tasks[&task], expected);
}

fn record(batch: &mut PhysicalCallBatch, call: PhysicalCall) {
	let result = batch.try_push(call);
	debug_assert!(result.is_ok());
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
