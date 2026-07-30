use std::cell::RefCell;
use std::fmt;
use std::net::{TcpListener, TcpStream};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use recipe_core::*;
use recipe_executor::{
	ArenaSet as ExecutorArenaSet, Backend as ExecutorBackend, BackendPoll as ExecutorBackendPoll,
	BackendWork as ExecutorBackendWork, ExternalTransferDirection as ExecutorTransferDirection,
	ExternalTransferPoll as ExecutorTransferPoll, MetricValue as ExecutorMetricValue,
	PendingRequest as ExecutorPendingRequest, PhysicalCall as ExecutorPhysicalCall,
	PhysicalCallBatch as ExecutorPhysicalCallBatch, PhysicalCallBatchOverflow as ExecutorPhysicalCallBatchOverflow,
	PhysicalPollStatus as ExecutorPhysicalPollStatus, TransferWork as ExecutorTransferWork,
	WorkClass as ExecutorWorkClass, WorkerAssignment, WorkerBackend, WorkerExternalTransfer, WorkerProjection,
};
use recipe_executor::{Watchdog as ExecutorWatchdog, sealed::Sealed as ExecutorSealed};
use recipe_transport::{ChannelCapacities, EndpointIdentity, ProtocolLimits, RuntimeChannel, SessionIdentity};
use sha2::{Digest as _, Sha256};

use crate::*;

const MASTER_RAM: DeviceId = DeviceId::new(1);
const WORKER_GPU: DeviceId = DeviceId::new(2);
const INIT_MASTER: TaskId = TaskId::new(1);
const INIT_WORKER: TaskId = TaskId::new(2);
const MASTER_TO_WORKER: TaskId = TaskId::new(3);
const WORKER_TO_MASTER: TaskId = TaskId::new(4);
const METRIC_ONE: TaskId = TaskId::new(5);
const METRIC_TWO: TaskId = TaskId::new(6);
const RUN: RunId = RunId::new(41);
const IMAGE: [u8; 16] = [0x5a; 16];
const MASTER_DATA: [u8; 16] = [0x31; 16];
const WORKER_DATA: [u8; 16] = [0xa7; 16];
const TEST_DRIVER_FAULT: DriverFault = DriverFault::new(91, 0xfeed);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaultPoint {
	Prepare,
	InitChunk,
	ReceiveData,
	Release,
	Finish,
}

fn digest(value: u8) -> Digest {
	Digest::new([value; 32])
}

fn success<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
	match result {
		Ok(value) => value,
		Err(error) => panic!("test setup failed: {error:?}"),
	}
}

fn label(value: &str) -> Label {
	success(Label::new(value))
}

fn measured<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Measured)
}

fn window(start: u64, end: u64) -> ScheduleWindow {
	ScheduleWindow {
		start: Nanoseconds::new(start),
		end: Nanoseconds::new(end),
	}
}

fn transfer_capability(rate: u64) -> TransferCapability {
	TransferCapability {
		rate: measured(success(BytesPerSecond::new(rate))),
		maximum_inflight_transfers: measured(success(TransferLaneCount::new(2))),
		asynchronous_submission: true,
		overlaps_calculation: true,
	}
}

fn finalized_fixture(bundle_digest: u8, mode: DuplexMode, payload_bytes: u64) -> (Topology, FinalizedBundle) {
	let master_machine = MachineId::new(1);
	let worker_machine = MachineId::new(2);
	let topology_id = TopologyIdentity::new(digest(1));
	let discovery_id = DiscoveryIdentity::new(digest(2));
	let (transport_kind, forward_resource, reverse_resource) = match mode {
		DuplexMode::Full => (
			TransportKind::Ethernet,
			DuplexResourceId::new(1),
			DuplexResourceId::new(2),
		),
		DuplexMode::Half => (
			TransportKind::Wlan,
			DuplexResourceId::new(9),
			DuplexResourceId::new(9),
		),
	};
	let target = TargetIdentity {
		backend: label("recipe-owned-native"),
		architecture: label("test-gpu"),
		abi: label("test-abi"),
	};
	let topology = Topology {
		identity: topology_id,
		machines: vec![
			Machine {
				id: master_machine,
				name: label("master-machine"),
			},
			Machine {
				id: worker_machine,
				name: label("worker-machine"),
			},
		],
		nodes: vec![
			Node {
				id: NodeId::new(1),
				role: NodeRole::Master,
				machine: master_machine,
				devices: vec![MASTER_RAM],
			},
			Node {
				id: NodeId::new(2),
				role: NodeRole::Worker,
				machine: worker_machine,
				devices: vec![WORKER_GPU],
			},
		],
		devices: vec![
			Device {
				id: MASTER_RAM,
				machine: master_machine,
				kind: DeviceKind::Ram,
				capacity: measured(ByteCount::new(2_000_000_000)),
				transfer_rate: measured(success(BytesPerSecond::new(90_000_000_000))),
				calculation_rate: None,
			},
			Device {
				id: WORKER_GPU,
				machine: worker_machine,
				kind: DeviceKind::GpuMemory,
				capacity: measured(ByteCount::new(2_000_000_000)),
				transfer_rate: measured(success(BytesPerSecond::new(432_000_000_000))),
				calculation_rate: Some(measured(success(FlopsPerSecond::new(380_000_000_000)))),
			},
		],
		links: vec![
			DirectedLink {
				id: LinkId::new(1),
				transport: TransportId::new(1),
				kind: transport_kind,
				duplex: mode,
				from: MASTER_RAM,
				to: WORKER_GPU,
				bandwidth: measured(success(BytesPerSecond::new(125_000_000))),
				maximum_inflight_transfers: measured(success(TransferLaneCount::new(2))),
				capacity_resource: forward_resource,
			},
			DirectedLink {
				id: LinkId::new(2),
				transport: TransportId::new(1),
				kind: transport_kind,
				duplex: mode,
				from: WORKER_GPU,
				to: MASTER_RAM,
				bandwidth: measured(success(BytesPerSecond::new(125_000_000))),
				maximum_inflight_transfers: measured(success(TransferLaneCount::new(2))),
				capacity_resource: reverse_resource,
			},
		],
	};
	let discovery = DiscoveryProfile {
		identity: discovery_id,
		topology: topology_id,
		devices: vec![
			DiscoveredDevice {
				device: MASTER_RAM,
				available: true,
				maximum_submission_queues: 64,
				total_capacity: measured(ByteCount::new(2_000_000_000)),
				transfer: transfer_capability(90_000_000_000),
				calculation: None,
			},
			DiscoveredDevice {
				device: WORKER_GPU,
				available: true,
				maximum_submission_queues: 64,
				total_capacity: measured(ByteCount::new(2_000_000_000)),
				transfer: transfer_capability(432_000_000_000),
				calculation: Some(CalculationCapability {
					target,
					rate: measured(success(FlopsPerSecond::new(380_000_000_000))),
					asynchronous_submission: true,
					maximum_concurrent_tasks: 2,
					subgroup_lanes: 32,
					maximum_workgroup_lanes: 256,
					maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
				}),
			},
		],
		links: vec![
			DiscoveredLink {
				link: LinkId::new(1),
				available: true,
				bandwidth: measured(success(BytesPerSecond::new(125_000_000))),
				maximum_inflight_transfers: measured(success(TransferLaneCount::new(2))),
				asynchronous_submission: true,
			},
			DiscoveredLink {
				link: LinkId::new(2),
				available: true,
				bandwidth: measured(success(BytesPerSecond::new(125_000_000))),
				maximum_inflight_transfers: measured(success(TransferLaneCount::new(2))),
				asynchronous_submission: true,
			},
		],
	};
	let master_value = ValueId::new(1);
	let worker_value = ValueId::new(2);
	let master_queue = QueueSlotId::new(1);
	let worker_queue = QueueSlotId::new(2);
	let master_completion = CompletionSlotId::new(1);
	let worker_completion = CompletionSlotId::new(2);
	let metric = MetricId::new(1);
	let metric_slot = MetricSlotId::new(1);
	let resources = ResourceManifest {
		queues: vec![
			QueueSlot {
				id: master_queue,
				device: MASTER_RAM,
			},
			QueueSlot {
				id: worker_queue,
				device: WORKER_GPU,
			},
		],
		completions: vec![
			CompletionSlot {
				id: master_completion,
				device: MASTER_RAM,
			},
			CompletionSlot {
				id: worker_completion,
				device: WORKER_GPU,
			},
		],
		metrics: vec![MetricSlot {
			id: metric_slot,
			metric,
		}],
		pinned_staging: vec![],
		scratch: vec![],
	};
	let draft_id = DraftIdentity::new(digest(3));
	let candidate = CandidateIdentity::new(digest(4));
	let draft = DraftPlan {
		identity: draft_id,
		candidate,
		discovery: discovery_id,
		topology: topology_id,
		values: vec![
			ValueSpec {
				id: master_value,
				dtype: DType::F32,
				bytes: ByteCount::new(payload_bytes),
				device: MASTER_RAM,
				producer: Some(INIT_MASTER),
			},
			ValueSpec {
				id: worker_value,
				dtype: DType::F32,
				bytes: ByteCount::new(payload_bytes),
				device: WORKER_GPU,
				producer: Some(INIT_WORKER),
			},
		],
		kernels: vec![],
		artifacts: vec![],
		artifact_builds: vec![],
		tasks: vec![
			Task {
				id: INIT_MASTER,
				phase: RunPhase::Init,
				window: window(0, 1),
				dependencies: vec![],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::External,
					destination: TransferEndpoint::Device {
						device: MASTER_RAM,
						value: master_value,
					},
					bytes: ByteCount::new(payload_bytes),
					route: vec![],
					lane_claims: vec![TransferLaneClaim::External {
						device: MASTER_RAM,
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: master_queue,
						completion: master_completion,
					},
				}),
			},
			Task {
				id: INIT_WORKER,
				phase: RunPhase::Init,
				window: window(0, 1),
				dependencies: vec![],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::External,
					destination: TransferEndpoint::Device {
						device: WORKER_GPU,
						value: worker_value,
					},
					bytes: ByteCount::new(payload_bytes),
					route: vec![],
					lane_claims: vec![TransferLaneClaim::External {
						device: WORKER_GPU,
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: worker_queue,
						completion: worker_completion,
					},
				}),
			},
			Task {
				id: MASTER_TO_WORKER,
				phase: RunPhase::Loop,
				window: window(1, 2),
				dependencies: vec![INIT_MASTER, INIT_WORKER],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::Device {
						device: MASTER_RAM,
						value: master_value,
					},
					destination: TransferEndpoint::Device {
						device: WORKER_GPU,
						value: worker_value,
					},
					bytes: ByteCount::new(payload_bytes),
					route: vec![LinkId::new(1)],
					lane_claims: vec![TransferLaneClaim::Link {
						link: LinkId::new(1),
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: master_queue,
						completion: master_completion,
					},
				}),
			},
			Task {
				id: WORKER_TO_MASTER,
				phase: RunPhase::Loop,
				window: window(2, 3),
				dependencies: vec![MASTER_TO_WORKER],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::Device {
						device: WORKER_GPU,
						value: worker_value,
					},
					destination: TransferEndpoint::Device {
						device: MASTER_RAM,
						value: master_value,
					},
					bytes: ByteCount::new(payload_bytes),
					route: vec![LinkId::new(2)],
					lane_claims: vec![TransferLaneClaim::Link {
						link: LinkId::new(2),
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: worker_queue,
						completion: worker_completion,
					},
				}),
			},
			Task {
				id: METRIC_ONE,
				phase: RunPhase::Loop,
				window: window(3, 4),
				dependencies: vec![WORKER_TO_MASTER],
				kind: TaskKind::Metric(MetricTask {
					purpose: MetricPurpose::User,
					metric,
					value: worker_value,
					slot: metric_slot,
					submission: SubmissionSlots {
						queue: worker_queue,
						completion: worker_completion,
					},
				}),
			},
			Task {
				id: METRIC_TWO,
				phase: RunPhase::Loop,
				window: window(4, 5),
				dependencies: vec![METRIC_ONE],
				kind: TaskKind::Metric(MetricTask {
					purpose: MetricPurpose::User,
					metric,
					value: worker_value,
					slot: metric_slot,
					submission: SubmissionSlots {
						queue: worker_queue,
						completion: worker_completion,
					},
				}),
			},
		],
		resources: resources.clone(),
		arena_objects: vec![
			ArenaObject {
				id: ArenaObjectId::new(1),
				device: MASTER_RAM,
				bytes: ByteCount::new(payload_bytes),
				alignment: ByteCount::new(16),
				lifetime: window(0, 5),
			},
			ArenaObject {
				id: ArenaObjectId::new(2),
				device: WORKER_GPU,
				bytes: ByteCount::new(payload_bytes),
				alignment: ByteCount::new(16),
				lifetime: window(0, 5),
			},
		],
		value_bindings: vec![
			ValueBinding {
				value: master_value,
				object: ArenaObjectId::new(1),
				object_offset: ByteOffset::new(0),
			},
			ValueBinding {
				value: worker_value,
				object: ArenaObjectId::new(2),
				object_offset: ByteOffset::new(0),
			},
		],
		value_aliases: vec![],
		init_images: vec![
			InitDataImage {
				device: MASTER_RAM,
				image: master_value,
				bytes: ByteCount::new(payload_bytes),
				members: vec![InitDataImageMember {
					logical: master_value,
					physical: master_value,
					dtype: DType::F32,
					bytes: ByteCount::new(payload_bytes),
					image_offset: ByteOffset::new(0),
				}],
			},
			InitDataImage {
				device: WORKER_GPU,
				image: worker_value,
				bytes: ByteCount::new(payload_bytes),
				members: vec![InitDataImageMember {
					logical: worker_value,
					physical: worker_value,
					dtype: DType::F32,
					bytes: ByteCount::new(payload_bytes),
					image_offset: ByteOffset::new(0),
				}],
			},
		],
		releases: vec![
			ArenaRelease { device: MASTER_RAM },
			ArenaRelease { device: WORKER_GPU },
		],
	};
	let reservations = ReservationLedger {
		entries: vec![
			ReservationEntry {
				device: MASTER_RAM,
				name: label("master-user-reservation"),
				bytes: EXACT_USER_RESERVATION,
				mechanism: ReservationMechanism::EnforcedQuota,
				evidence: ReservationEvidence::NonGpu,
			},
			ReservationEntry {
				device: WORKER_GPU,
				name: label("worker-user-reservation"),
				bytes: EXACT_USER_RESERVATION,
				mechanism: ReservationMechanism::HeldAllocation,
				evidence: ReservationEvidence::GpuDisplay {
					enabled_connectors: 1,
				},
			},
		],
	};
	let capacity = CapacityLedger {
		entries: [MASTER_RAM, WORKER_GPU]
			.into_iter()
			.map(|device| CapacityLedgerEntry {
				device,
				total: measured(ByteCount::new(2_000_000_000)),
				runtime_overhead: measured(ByteCount::ZERO),
				fragmentation: measured(ByteCount::ZERO),
				safety_headroom: measured(ByteCount::ZERO),
				recipe_usable: measured(ByteCount::new(1_000_000_000)),
			})
			.collect(),
	};
	let realization = RealizationProfile {
		identity: RealizationIdentity::new(digest(5)),
		draft: draft_id,
		candidate,
		discovery: discovery_id,
		topology: topology_id,
		artifacts: vec![],
		resources,
		reservations,
		capacity,
	};
	let layouts = vec![
		ArenaLayout {
			device: MASTER_RAM,
			size: ByteCount::new(payload_bytes),
			allocations: vec![ArenaAllocation {
				object: ArenaObjectId::new(1),
				offset: ByteOffset::new(0),
			}],
		},
		ArenaLayout {
			device: WORKER_GPU,
			size: ByteCount::new(payload_bytes),
			allocations: vec![ArenaAllocation {
				object: ArenaObjectId::new(2),
				offset: ByteOffset::new(0),
			}],
		},
	];
	let bundle = success(FinalizedBundle::finalize(
		BundleIdentity::new(digest(bundle_digest)),
		&topology,
		&discovery,
		draft,
		realization,
		layouts,
	));
	(topology, bundle)
}

fn limits() -> RemoteLimits {
	success(RemoteLimits::new(
		512,
		success(ManifestLimits::new(4, 16, 4, 8)),
		success(RuntimeSlots::new(2, 2, 2)),
	))
}

fn program(mode: DuplexMode, bundle_digest: u8) -> ProvisionedProgram {
	let (topology, bundle) = finalized_fixture(bundle_digest, mode, 16);
	success(ProvisionedProgram::from_bundle(
		&bundle,
		&topology,
		&[WORKER_GPU],
		limits(),
	))
}

fn endpoints() -> (EndpointIdentity, EndpointIdentity) {
	(
		success(EndpointIdentity::new(digest(20), digest(21))),
		success(EndpointIdentity::new(digest(22), digest(23))),
	)
}

fn channels(control_slots: usize) -> (RuntimeChannel, RuntimeChannel) {
	let listener = success(TcpListener::bind(("127.0.0.1", 0)));
	let address = success(listener.local_addr());
	let master_stream = success(TcpStream::connect(address));
	let (worker_stream, _) = success(listener.accept());
	let (master_endpoint, worker_endpoint) = endpoints();
	let protocol = success(ProtocolLimits::new(512, Duration::from_secs(2)));
	let capacities = success(ChannelCapacities::new(control_slots, 2, 2, 512));
	(
		success(RuntimeChannel::new(
			master_stream,
			success(SessionIdentity::new(master_endpoint, worker_endpoint)),
			protocol,
			capacities,
		)),
		success(RuntimeChannel::new(
			worker_stream,
			success(SessionIdentity::new(worker_endpoint, master_endpoint)),
			protocol,
			capacities,
		)),
	)
}

#[derive(Debug)]
struct TestDriver {
	prepared: bool,
	prepares: usize,
	image: [u8; 16],
	active_tasks: [Option<TaskId>; 2],
	received: [u8; 16],
	init_transfer: Option<(DeviceId, usize, bool)>,
	init_image_transfer: Option<(DeviceId, usize, bool)>,
	incoming_transfer: Option<(TaskId, usize, bool)>,
	outgoing_transfer: Option<(TaskId, usize, bool)>,
	arena_active: bool,
	releases: usize,
	fatal_cleanups: usize,
	finished: bool,
	fault_at: Option<FaultPoint>,
	cleanup_fault: Option<DriverFault>,
	cleanup_observer: Option<Arc<AtomicUsize>>,
}

impl TestDriver {
	fn new() -> Self {
		Self {
			prepared: false,
			prepares: 0,
			image: [0; 16],
			active_tasks: [None; 2],
			received: [0; 16],
			init_transfer: None,
			init_image_transfer: None,
			incoming_transfer: None,
			outgoing_transfer: None,
			arena_active: false,
			releases: 0,
			fatal_cleanups: 0,
			finished: false,
			fault_at: None,
			cleanup_fault: None,
			cleanup_observer: None,
		}
	}

	fn failing(fault_at: FaultPoint) -> Self {
		Self {
			fault_at: Some(fault_at),
			..Self::new()
		}
	}

	fn observed_failure(fault_at: FaultPoint, observer: Arc<AtomicUsize>) -> Self {
		Self {
			fault_at: Some(fault_at),
			cleanup_observer: Some(observer),
			..Self::new()
		}
	}

	fn cleanup_failure(fault_at: FaultPoint, cleanup_fault: DriverFault) -> Self {
		Self {
			fault_at: Some(fault_at),
			cleanup_fault: Some(cleanup_fault),
			..Self::new()
		}
	}
}

impl WorkerDriver for TestDriver {
	fn prepare(&mut self, run: RunId, _program: &ProvisionedProgram) -> Result<(), DriverFault> {
		if self.fault_at == Some(FaultPoint::Prepare) {
			return Err(TEST_DRIVER_FAULT);
		}
		assert_ne!(run.get(), 0);
		self.prepared = true;
		self.prepares += 1;
		self.finished = false;
		Ok(())
	}

	fn begin_init_image(&mut self, device: DeviceId, bytes: ByteCount, _digest: Digest) -> Result<(), DriverFault> {
		assert!(self.prepared);
		assert_eq!(device, WORKER_GPU);
		assert_eq!(bytes, ByteCount::new(16));
		self.arena_active = true;
		Ok(())
	}

	fn begin_init_chunk(&mut self, device: DeviceId, offset: u64, bytes: &[u8]) -> Result<(), DriverFault> {
		if self.fault_at == Some(FaultPoint::InitChunk) {
			return Err(TEST_DRIVER_FAULT);
		}
		assert_eq!(device, WORKER_GPU);
		assert!(self.init_transfer.is_none());
		let start = success(usize::try_from(offset));
		self.image[start..start + bytes.len()].copy_from_slice(bytes);
		self.init_transfer = Some((device, bytes.len(), false));
		Ok(())
	}

	fn poll_init_chunk(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault> {
		let (active_device, bytes, polled) = self.init_transfer.expect("active init transfer");
		assert_eq!(device, active_device);
		if !polled {
			self.init_transfer = Some((active_device, bytes, true));
			return Ok(DriverTransferPoll::Pending);
		}
		self.init_transfer = None;
		Ok(DriverTransferPoll::Complete { bytes })
	}

	fn finish_init_image(&mut self, device: DeviceId) -> Result<(), DriverFault> {
		assert_eq!(device, WORKER_GPU);
		assert!(self.init_image_transfer.is_none());
		self.init_image_transfer = Some((device, self.image.len(), false));
		Ok(())
	}

	fn poll_init_image(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault> {
		let (active_device, bytes, polled) = self.init_image_transfer.expect("active init image");
		assert_eq!(device, active_device);
		if !polled {
			self.init_image_transfer = Some((active_device, bytes, true));
			return Ok(DriverTransferPoll::Pending);
		}
		self.init_image_transfer = None;
		Ok(DriverTransferPoll::Complete { bytes })
	}

	fn submit_task(&mut self, task: TaskId) -> Result<(), DriverFault> {
		let slot = self
			.active_tasks
			.iter_mut()
			.find(|slot| slot.is_none())
			.expect("test driver task capacity");
		*slot = Some(task);
		Ok(())
	}

	fn poll_task(&mut self, task: TaskId) -> Result<DriverPoll, DriverFault> {
		let slot = self
			.active_tasks
			.iter_mut()
			.find(|slot| **slot == Some(task))
			.expect("submitted test task");
		*slot = None;
		Ok(DriverPoll::Complete {
			metric: Some(RemoteMetricValue::I32(success(i32::try_from(task.get())))),
		})
	}

	fn begin_receive_user_data(&mut self, task: TaskId, bytes: &[u8]) -> Result<(), DriverFault> {
		if self.fault_at == Some(FaultPoint::ReceiveData) {
			return Err(TEST_DRIVER_FAULT);
		}
		assert_eq!(task, MASTER_TO_WORKER);
		assert!(self.incoming_transfer.is_none());
		self.received.copy_from_slice(bytes);
		self.incoming_transfer = Some((task, bytes.len(), false));
		Ok(())
	}

	fn poll_receive_user_data(&mut self, task: TaskId) -> Result<DriverTransferPoll, DriverFault> {
		let (active_task, bytes, polled) = self.incoming_transfer.expect("active incoming transfer");
		assert_eq!(task, active_task);
		if !polled {
			self.incoming_transfer = Some((active_task, bytes, true));
			return Ok(DriverTransferPoll::Pending);
		}
		self.incoming_transfer = None;
		Ok(DriverTransferPoll::Complete { bytes })
	}

	fn begin_produce_user_data(&mut self, task: TaskId, destination: &mut [u8]) -> Result<(), DriverFault> {
		assert_eq!(task, WORKER_TO_MASTER);
		assert!(self.outgoing_transfer.is_none());
		destination.copy_from_slice(&WORKER_DATA);
		self.outgoing_transfer = Some((task, destination.len(), false));
		Ok(())
	}

	fn poll_produce_user_data(&mut self, task: TaskId) -> Result<DriverTransferPoll, DriverFault> {
		let (active_task, bytes, polled) = self.outgoing_transfer.expect("active outgoing transfer");
		assert_eq!(task, active_task);
		if !polled {
			self.outgoing_transfer = Some((active_task, bytes, true));
			return Ok(DriverTransferPoll::Pending);
		}
		self.outgoing_transfer = None;
		Ok(DriverTransferPoll::Complete { bytes })
	}

	fn user_data_acked(&mut self, task: TaskId) -> Result<(), DriverFault> {
		assert_eq!(task, WORKER_TO_MASTER);
		Ok(())
	}

	fn cancel(&mut self, _reason: u32) -> Result<(), DriverFault> {
		self.active_tasks.fill(None);
		self.init_transfer = None;
		self.init_image_transfer = None;
		self.incoming_transfer = None;
		self.outgoing_transfer = None;
		Ok(())
	}

	fn begin_exit(&mut self) -> Result<(), DriverFault> {
		Ok(())
	}

	fn release_arena(&mut self, device: DeviceId) -> Result<(), DriverFault> {
		if self.fault_at == Some(FaultPoint::Release) {
			return Err(TEST_DRIVER_FAULT);
		}
		assert_eq!(device, WORKER_GPU);
		assert!(self.arena_active);
		self.arena_active = false;
		self.releases += 1;
		Ok(())
	}

	fn finish(&mut self) -> Result<(), DriverFault> {
		if self.fault_at == Some(FaultPoint::Finish) {
			return Err(TEST_DRIVER_FAULT);
		}
		self.finished = true;
		Ok(())
	}

	fn cleanup_after_fault(&mut self, primary: DriverFault) -> Result<(), DriverFault> {
		self.active_tasks.fill(None);
		self.init_transfer = None;
		self.init_image_transfer = None;
		self.incoming_transfer = None;
		self.outgoing_transfer = None;
		if self.arena_active {
			self.arena_active = false;
			self.releases += 1;
		}
		self.finished = true;
		self.fatal_cleanups += 1;
		if let Some(observer) = &self.cleanup_observer {
			observer.fetch_add(1, Ordering::SeqCst);
		}
		match self.cleanup_fault {
			Some(cleanup_fault) => {
				assert_eq!(primary, TEST_DRIVER_FAULT);
				Err(cleanup_fault)
			}
			None => Ok(()),
		}
	}
}

#[derive(Debug, Default)]
struct ExecutorObserver {
	binds: AtomicUsize,
	local_prepares: AtomicUsize,
	external_prepares: AtomicUsize,
	quiesces: AtomicUsize,
	releases: AtomicUsize,
	destroys: AtomicUsize,
}

#[derive(Clone, Debug)]
struct ExecutorFakeBackend {
	observer: Arc<ExecutorObserver>,
	fail_ingress: bool,
}

impl ExecutorFakeBackend {
	fn new(observer: Arc<ExecutorObserver>) -> Self {
		Self {
			observer,
			fail_ingress: false,
		}
	}

	fn failing_ingress(observer: Arc<ExecutorObserver>) -> Self {
		Self {
			observer,
			fail_ingress: true,
		}
	}
}

impl ExecutorSealed for ExecutorFakeBackend {}

#[derive(Debug)]
struct ExecutorFakeResource {
	active: usize,
}

#[derive(Debug)]
struct ExecutorFakeArena {
	device: DeviceId,
	bytes: RefCell<Box<[u8]>>,
}

#[derive(Debug)]
struct ExecutorFakePending {
	task: TaskId,
	class: ExecutorWorkClass,
	active: bool,
	polled: bool,
	metric: Option<ExecutorMetricValue>,
}

#[derive(Debug)]
struct ExecutorFakeExternalPending {
	task: TaskId,
	direction: ExecutorTransferDirection,
	bytes: usize,
	active: bool,
	polled: bool,
	awaiting_ack: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutorFakeError {
	CallOverflow,
	Contract(&'static str),
	IngressFault,
}

impl fmt::Display for ExecutorFakeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CallOverflow => formatter.write_str("fake physical call batch overflow"),
			Self::Contract(detail) => write!(formatter, "fake executor contract failure: {detail}"),
			Self::IngressFault => formatter.write_str("injected external ingress failure"),
		}
	}
}

impl std::error::Error for ExecutorFakeError {}

fn executor_record(calls: &mut ExecutorPhysicalCallBatch, call: ExecutorPhysicalCall) -> Result<(), ExecutorFakeError> {
	calls.try_push(call)
		.map_err(|ExecutorPhysicalCallBatchOverflow| ExecutorFakeError::CallOverflow)
}

fn arena_range(
	location: ResolvedValueLocation,
	buffer_len: usize,
) -> Result<std::ops::Range<usize>, ExecutorFakeError> {
	let start = usize::try_from(location.arena_offset.get())
		.map_err(|_| ExecutorFakeError::Contract("arena offset does not fit the test host"))?;
	let bytes = usize::try_from(location.bytes.get())
		.map_err(|_| ExecutorFakeError::Contract("value size does not fit the test host"))?;
	let end = start
		.checked_add(bytes)
		.ok_or(ExecutorFakeError::Contract("arena range overflowed"))?;
	if end > buffer_len {
		return Err(ExecutorFakeError::Contract("value exceeds the fake arena"));
	}
	Ok(start..end)
}

impl ExecutorBackend for ExecutorFakeBackend {
	type Arena = ExecutorFakeArena;
	type Resource = ExecutorFakeResource;
	type Pending = ExecutorFakePending;
	type Error = ExecutorFakeError;

	fn bind_resources(
		&mut self,
		_bundle: &FinalizedBundle,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<Self::Resource, Self::Error> {
		self.observer.binds.fetch_add(1, Ordering::SeqCst);
		executor_record(physical_calls, ExecutorPhysicalCall::BindResources)?;
		Ok(ExecutorFakeResource { active: 0 })
	}

	fn prepare_pending(
		&mut self,
		_resource: &mut Self::Resource,
		request: ExecutorPendingRequest,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<Self::Pending, Self::Error> {
		self.observer.local_prepares.fetch_add(1, Ordering::SeqCst);
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::PreparePending { task: request.task },
		)?;
		Ok(ExecutorFakePending {
			task: request.task,
			class: request.class,
			active: false,
			polled: false,
			metric: None,
		})
	}

	fn allocate_arena(
		&mut self,
		_resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<Self::Arena, Self::Error> {
		let bytes = usize::try_from(layout.size.get())
			.map_err(|_| ExecutorFakeError::Contract("arena size does not fit the test host"))?;
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::AllocateArena {
				device: layout.device,
				bytes: layout.size,
			},
		)?;
		Ok(ExecutorFakeArena {
			device: layout.device,
			bytes: RefCell::new(vec![0; bytes].into_boxed_slice()),
		})
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ExecutorArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: ExecutorBackendWork<'_>,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		if pending.active || pending.task != work.task() || pending.class != work.class() {
			return Err(ExecutorFakeError::Contract(
				"local pending token differs from submitted work",
			));
		}
		pending.metric = match work {
			ExecutorBackendWork::InitAdmission(work) => {
				let arena = arenas
					.get(work.destination.device)
					.ok_or(ExecutorFakeError::Contract("init arena is absent"))?;
				let mut arena_bytes = arena.bytes.borrow_mut();
				let range = arena_range(work.destination, arena_bytes.len())?;
				if work.image.len() != range.len()
					|| work.bytes.get() != u64::try_from(work.image.len()).unwrap_or(0)
				{
					return Err(ExecutorFakeError::Contract("init image length differs"));
				}
				arena_bytes[range].copy_from_slice(work.image);
				executor_record(
					physical_calls,
					ExecutorPhysicalCall::AdmissionChunk {
						task: work.task,
						device: work.destination.device,
						bytes: work.bytes,
						chunk_index: 0,
					},
				)?;
				None
			}
			ExecutorBackendWork::Metric(work) => {
				executor_record(
					physical_calls,
					ExecutorPhysicalCall::SubmitMetric {
						task: work.task,
						slot: work.slot,
					},
				)?;
				Some(ExecutorMetricValue::F32(match work.task {
					METRIC_ONE => 1.0,
					METRIC_TWO => 2.0,
					_ => return Err(ExecutorFakeError::Contract("unexpected metric task")),
				}))
			}
			ExecutorBackendWork::Calculation(work) => {
				executor_record(
					physical_calls,
					ExecutorPhysicalCall::SubmitCalculation { task: work.task },
				)?;
				None
			}
			ExecutorBackendWork::InternalTransfer(work) => {
				executor_record(
					physical_calls,
					ExecutorPhysicalCall::SubmitInternalTransfer { task: work.task },
				)?;
				None
			}
			ExecutorBackendWork::ExitTransfer(work) => {
				executor_record(
					physical_calls,
					ExecutorPhysicalCall::SubmitExitTransfer { task: work.task },
				)?;
				None
			}
		};
		pending.active = true;
		pending.polled = false;
		resource.active = resource
			.active
			.checked_add(1)
			.ok_or(ExecutorFakeError::Contract("active count overflowed"))?;
		Ok(())
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<ExecutorBackendPoll, Self::Error> {
		if !pending.active {
			return Err(ExecutorFakeError::Contract("local pending token is idle"));
		}
		if !pending.polled {
			pending.polled = true;
			executor_record(
				physical_calls,
				ExecutorPhysicalCall::Poll {
					task: pending.task,
					status: ExecutorPhysicalPollStatus::Pending,
				},
			)?;
			return Ok(ExecutorBackendPoll::Pending);
		}
		pending.active = false;
		resource.active = resource
			.active
			.checked_sub(1)
			.ok_or(ExecutorFakeError::Contract("active count underflowed"))?;
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::Poll {
				task: pending.task,
				status: ExecutorPhysicalPollStatus::Complete,
			},
		)?;
		Ok(ExecutorBackendPoll::Complete {
			metric: pending.metric,
		})
	}

	fn collect_exit(
		&mut self,
		_resource: &mut Self::Resource,
		arenas: ExecutorArenaSet<'_, Self::Arena>,
		_pending: &mut Self::Pending,
		work: ExecutorTransferWork<'_>,
		destination: &mut [u8],
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		let ResolvedTransferEndpoint::Device(source) = work.source else {
			return Err(ExecutorFakeError::Contract("exit source is not a device"));
		};
		let arena = arenas
			.get(source.device)
			.ok_or(ExecutorFakeError::Contract("exit arena is absent"))?;
		let arena_bytes = arena.bytes.borrow();
		let range = arena_range(source, arena_bytes.len())?;
		if destination.len() != range.len() {
			return Err(ExecutorFakeError::Contract(
				"exit destination length differs",
			));
		}
		destination.copy_from_slice(&arena_bytes[range]);
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::CollectExit {
				task: work.task,
				bytes: work.bytes,
			},
		)
	}

	fn release_arena(
		&mut self,
		resource: &mut Self::Resource,
		device: DeviceId,
		arena: Self::Arena,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		if resource.active != 0 || arena.device != device {
			return Err(ExecutorFakeError::Contract(
				"arena release preceded quiescence or completion",
			));
		}
		self.observer.releases.fetch_add(1, Ordering::SeqCst);
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::ReleaseArena { device },
		)
	}

	fn destroy_resources(
		&mut self,
		resource: Self::Resource,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		if resource.active != 0 {
			return Err(ExecutorFakeError::Contract(
				"resource destruction preceded quiescence",
			));
		}
		self.observer.destroys.fetch_add(1, Ordering::SeqCst);
		executor_record(physical_calls, ExecutorPhysicalCall::DestroyResources)
	}
}

impl WorkerBackend for ExecutorFakeBackend {
	type ExternalPending = ExecutorFakeExternalPending;

	fn bind_worker_resources(
		&mut self,
		bundle: &FinalizedBundle,
		projection: &WorkerProjection,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<Self::Resource, Self::Error> {
		if projection.devices() != [WORKER_GPU] || bundle.init_image(WORKER_GPU).is_none() {
			return Err(ExecutorFakeError::Contract("worker projection differs"));
		}
		self.bind_resources(bundle, physical_calls)
	}

	fn prepare_external(
		&mut self,
		_resource: &mut Self::Resource,
		transfer: &WorkerExternalTransfer,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<Self::ExternalPending, Self::Error> {
		self.observer
			.external_prepares
			.fetch_add(1, Ordering::SeqCst);
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::PrepareExternal {
				task: transfer.task(),
			},
		)?;
		Ok(ExecutorFakeExternalPending {
			task: transfer.task(),
			direction: transfer.direction(),
			bytes: usize::try_from(transfer.bytes().get())
				.map_err(|_| ExecutorFakeError::Contract("external size does not fit the test host"))?,
			active: false,
			polled: false,
			awaiting_ack: false,
		})
	}

	fn begin_external_ingress(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ExecutorArenaSet<'_, Self::Arena>,
		pending: &mut Self::ExternalPending,
		transfer: &WorkerExternalTransfer,
		bytes: &[u8],
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		if self.fail_ingress {
			return Err(ExecutorFakeError::IngressFault);
		}
		if pending.active
			|| pending.awaiting_ack
			|| pending.task != transfer.task()
			|| pending.direction != ExecutorTransferDirection::Ingress
		{
			return Err(ExecutorFakeError::Contract("ingress pending token differs"));
		}
		let arena = arenas
			.get(transfer.local().device)
			.ok_or(ExecutorFakeError::Contract("ingress arena is absent"))?;
		let mut arena_bytes = arena.bytes.borrow_mut();
		let range = arena_range(transfer.local(), arena_bytes.len())?;
		if bytes.len() != range.len() {
			return Err(ExecutorFakeError::Contract("ingress length differs"));
		}
		arena_bytes[range].copy_from_slice(bytes);
		pending.active = true;
		pending.polled = false;
		resource.active = resource
			.active
			.checked_add(1)
			.ok_or(ExecutorFakeError::Contract("active count overflowed"))?;
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::SubmitExternalIngress {
				task: transfer.task(),
				bytes: transfer.bytes(),
			},
		)
	}

	fn begin_external_egress(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ExecutorArenaSet<'_, Self::Arena>,
		pending: &mut Self::ExternalPending,
		transfer: &WorkerExternalTransfer,
		destination: &mut [u8],
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		if pending.active
			|| pending.awaiting_ack
			|| pending.task != transfer.task()
			|| pending.direction != ExecutorTransferDirection::Egress
		{
			return Err(ExecutorFakeError::Contract("egress pending token differs"));
		}
		let arena = arenas
			.get(transfer.local().device)
			.ok_or(ExecutorFakeError::Contract("egress arena is absent"))?;
		let arena_bytes = arena.bytes.borrow();
		let range = arena_range(transfer.local(), arena_bytes.len())?;
		if destination.len() != range.len() {
			return Err(ExecutorFakeError::Contract("egress length differs"));
		}
		destination.copy_from_slice(&arena_bytes[range]);
		pending.active = true;
		pending.polled = false;
		resource.active = resource
			.active
			.checked_add(1)
			.ok_or(ExecutorFakeError::Contract("active count overflowed"))?;
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::SubmitExternalEgress {
				task: transfer.task(),
				bytes: transfer.bytes(),
			},
		)
	}

	fn poll_external(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::ExternalPending,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<ExecutorTransferPoll, Self::Error> {
		if !pending.active {
			return Err(ExecutorFakeError::Contract(
				"external pending token is idle",
			));
		}
		if !pending.polled {
			pending.polled = true;
			executor_record(
				physical_calls,
				ExecutorPhysicalCall::Poll {
					task: pending.task,
					status: ExecutorPhysicalPollStatus::Pending,
				},
			)?;
			return Ok(ExecutorTransferPoll::Pending);
		}
		pending.active = false;
		pending.awaiting_ack = pending.direction == ExecutorTransferDirection::Egress;
		resource.active = resource
			.active
			.checked_sub(1)
			.ok_or(ExecutorFakeError::Contract("active count underflowed"))?;
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::Poll {
				task: pending.task,
				status: ExecutorPhysicalPollStatus::Complete,
			},
		)?;
		Ok(ExecutorTransferPoll::Complete {
			bytes: pending.bytes,
		})
	}

	fn acknowledge_external_egress(
		&mut self,
		_resource: &mut Self::Resource,
		pending: &mut Self::ExternalPending,
		transfer: &WorkerExternalTransfer,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		if !pending.awaiting_ack || pending.task != transfer.task() {
			return Err(ExecutorFakeError::Contract("egress acknowledgment differs"));
		}
		pending.awaiting_ack = false;
		executor_record(
			physical_calls,
			ExecutorPhysicalCall::AcknowledgeExternalEgress {
				task: transfer.task(),
			},
		)
	}

	fn quiesce_worker(
		&mut self,
		resource: &mut Self::Resource,
		physical_calls: &mut ExecutorPhysicalCallBatch,
	) -> Result<(), Self::Error> {
		resource.active = 0;
		self.observer.quiesces.fetch_add(1, Ordering::SeqCst);
		executor_record(physical_calls, ExecutorPhysicalCall::QuiesceWorker)
	}
}

fn complete_worker<C, D>(
	channel: C,
	program: ProvisionedProgram,
	run_id: RunId,
	driver: D,
) -> RemoteResult<WorkerComplete<D>>
where
	C: Into<RemoteChannel>,
	D: WorkerDriver,
{
	let (master_endpoint, worker_endpoint) = endpoints();
	let identity = success(RemoteIdentity::new(worker_endpoint, master_endpoint));
	let mut handshake = WorkerHandshake::new(channel, identity, limits(), program, run_id, driver)?;
	let mut init = loop {
		match handshake.progress()? {
			Advance::Pending(next) => handshake = next,
			Advance::Ready(next) => break next,
		}
		thread::yield_now();
	};
	let mut run = loop {
		match init.progress()? {
			Advance::Pending(next) => init = next,
			Advance::Ready(next) => break next,
		}
		thread::yield_now();
	};
	let mut exit = loop {
		match run.progress()? {
			WorkerRunProgress::Running { session, .. } => run = session,
			WorkerRunProgress::Exit(next) => break next,
			WorkerRunProgress::Cancelled(mut cancelled) => loop {
				match cancelled.progress()? {
					Advance::Pending(next) => cancelled = next,
					Advance::Ready(complete) => return Ok(complete),
				}
			},
		}
		thread::yield_now();
	};
	loop {
		match exit.progress()? {
			WorkerExitProgress::Exiting { session, .. } => exit = session,
			WorkerExitProgress::Complete(complete) => return Ok(complete),
			WorkerExitProgress::Cancelled(mut cancelled) => loop {
				match cancelled.progress()? {
					Advance::Pending(next) => cancelled = next,
					Advance::Ready(complete) => return Ok(complete),
				}
			},
		}
		thread::yield_now();
	}
}

fn spawn_worker(channel: RuntimeChannel, program: ProvisionedProgram) -> JoinHandle<RemoteResult<TestDriver>> {
	thread::spawn(move || {
		let complete = complete_worker(channel, program, RUN, TestDriver::new())?;
		let (_, driver) = complete.into_parts();
		Ok(driver)
	})
}

fn master_run_for<C: Into<RemoteChannel>>(
	channel: C,
	program: ProvisionedProgram,
	run_id: RunId,
) -> RemoteResult<MasterRun> {
	let (master_endpoint, worker_endpoint) = endpoints();
	let identity = success(RemoteIdentity::new(master_endpoint, worker_endpoint));
	let mut handshake = MasterHandshake::new(channel, identity, limits(), program, run_id)?;
	let mut init = loop {
		match handshake.progress()? {
			Advance::Pending(next) => handshake = next,
			Advance::Ready(next) => break next,
		}
		thread::yield_now();
	};
	init.queue_image(WORKER_GPU, Box::new(IMAGE))?;
	loop {
		match init.progress()? {
			Advance::Pending(next) => init = next,
			Advance::Ready(run) => return Ok(run),
		}
		thread::yield_now();
	}
}

fn master_run(channel: RuntimeChannel, program: ProvisionedProgram) -> RemoteResult<MasterRun> {
	master_run_for(channel, program, RUN)
}

fn assert_test_driver_fault(error: RemoteError) {
	assert!(matches!(
		error,
		RemoteError::Driver(fault) if fault == TEST_DRIVER_FAULT
	));
}

#[test]
fn terminal_driver_fault_is_sent_only_after_local_cleanup() {
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(2);
	let worker_program = provisioned.clone();
	let cleanup = Arc::new(AtomicUsize::new(0));
	let worker_cleanup = Arc::clone(&cleanup);
	let worker = thread::spawn(move || {
		complete_worker(
			worker_channel,
			worker_program,
			RUN,
			TestDriver::observed_failure(FaultPoint::Prepare, worker_cleanup),
		)
	});
	let master_error = master_run(master_channel, provisioned).expect_err("prepare fault must reach the master");
	assert_test_driver_fault(master_error);
	assert_eq!(cleanup.load(Ordering::SeqCst), 1);
	let worker_error = worker
		.join()
		.expect("worker thread")
		.expect_err("worker returns the reported terminal fault");
	assert_test_driver_fault(worker_error);
}

#[test]
fn cleanup_failure_is_composed_with_the_primary_fault() {
	const CLEANUP_FAULT: DriverFault = DriverFault::new(92, 0xc1ea);
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(2);
	let worker_program = provisioned.clone();
	let worker = thread::spawn(move || {
		complete_worker(
			worker_channel,
			worker_program,
			RUN,
			TestDriver::cleanup_failure(FaultPoint::Prepare, CLEANUP_FAULT),
		)
	});
	let expected = DriverFault::cleanup_failed(TEST_DRIVER_FAULT, CLEANUP_FAULT);
	let master_error = master_run(master_channel, provisioned).expect_err("cleanup fault must reach the master");
	assert!(matches!(
		master_error,
		RemoteError::Driver(fault) if fault == expected
	));
	assert_eq!(expected.code, DriverFaultCode::FATAL_CLEANUP_FAILED.get());
	assert_eq!(
		expected.detail,
		u64::from(TEST_DRIVER_FAULT.code) << 32 | u64::from(CLEANUP_FAULT.code)
	);
	let worker_error = worker
		.join()
		.expect("worker thread")
		.expect_err("worker returns the composite cleanup fault");
	assert!(matches!(
		worker_error,
		RemoteError::Driver(fault) if fault == expected
	));
}

#[derive(Debug, Default)]
struct LoopObservation {
	completed: [bool; 4],
	metric_count: usize,
	data_seen: bool,
}

fn observe_event(master: &mut MasterRun, observation: &mut LoopObservation, event: MasterRunEvent) -> RemoteResult<()> {
	match event {
		MasterRunEvent::TaskComplete(task) => {
			let index = match task {
				MASTER_TO_WORKER => 0,
				WORKER_TO_MASTER => 1,
				METRIC_ONE => 2,
				METRIC_TWO => 3,
				_ => panic!("unexpected completed task {task}"),
			};
			observation.completed[index] = true;
		}
		MasterRunEvent::DataReady { task, slot, bytes } => {
			assert_eq!(task, WORKER_TO_MASTER);
			assert_eq!(bytes, WORKER_DATA.len());
			let (_, data) = master.data(slot).expect("data slot");
			assert_eq!(data, WORKER_DATA);
			master.release_data(slot)?;
			observation.data_seen = true;
		}
		MasterRunEvent::MetricReady { slot, sample } => {
			assert!(sample.task == METRIC_ONE || sample.task == METRIC_TWO);
			assert_eq!(master.take_metric(slot), Some(sample));
			observation.metric_count += 1;
		}
		MasterRunEvent::TaskFailed { task, fault } => {
			panic!("task {task} failed unexpectedly: {fault:?}");
		}
	}
	Ok(())
}

fn drive_loop(mut master: MasterRun, mut observation: LoopObservation) -> RemoteResult<MasterExit> {
	for _ in 0..200_000 {
		if let Some(event) = master.progress()? {
			observe_event(&mut master, &mut observation, event)?;
		}
		if observation.completed.into_iter().all(|complete| complete)
			&& observation.data_seen
			&& observation.metric_count == 2
		{
			return master.begin_exit();
		}
		thread::yield_now();
	}
	panic!("remote loop did not complete");
}

fn drive_exit(mut master: MasterExit) -> RemoteResult<MasterComplete> {
	for _ in 0..200_000 {
		master.progress()?;
		if master.is_complete() {
			return master.into_complete();
		}
		thread::yield_now();
	}
	panic!("remote exit did not complete");
}

fn executor_driver(
	backend: ExecutorFakeBackend,
	bundle_digest: u8,
) -> (
	ProvisionedProgram,
	ExecutorWorkerDriver<ExecutorFakeBackend>,
) {
	let (topology, bundle) = finalized_fixture(bundle_digest, DuplexMode::Full, 16);
	let program = success(ProvisionedProgram::from_bundle(
		&bundle,
		&topology,
		&[WORKER_GPU],
		limits(),
	));
	let driver = success(ExecutorWorkerDriver::new(
		bundle,
		&topology,
		WorkerAssignment::new(MachineId::new(2), NodeId::new(2)),
		&program,
		backend,
		success(ExecutorWatchdog::new(8)),
	));
	(program, driver)
}

fn wait_executor_task(
	master: &mut MasterRun,
	expected_task: TaskId,
	expected_data: Option<&[u8]>,
	expected_metric: Option<RemoteMetricValue>,
) -> RemoteResult<()> {
	let mut saw_data = expected_data.is_none();
	let mut saw_metric = expected_metric.is_none();
	for _ in 0..200_000 {
		let Some(event) = master.progress()? else {
			thread::yield_now();
			continue;
		};
		match event {
			MasterRunEvent::TaskComplete(task) if task == expected_task => {
				if !saw_data || !saw_metric {
					return Err(RemoteError::Protocol {
						detail: "executor task completed before its data or metric report",
					});
				}
				return Ok(());
			}
			MasterRunEvent::DataReady { task, slot, bytes } if task == expected_task => {
				let expected = expected_data.ok_or(RemoteError::Protocol {
					detail: "executor emitted unexpected data",
				})?;
				let (_, actual) = master.data(slot).ok_or(RemoteError::Protocol {
					detail: "executor data slot disappeared",
				})?;
				if bytes != expected.len() || actual != expected {
					return Err(RemoteError::Protocol {
						detail: "executor data differs from the resident worker value",
					});
				}
				master.release_data(slot)?;
				saw_data = true;
			}
			MasterRunEvent::MetricReady { slot, sample } if sample.task == expected_task => {
				if Some(sample.value) != expected_metric {
					return Err(RemoteError::Protocol {
						detail: "executor metric differs from the local task completion",
					});
				}
				if master.take_metric(slot) != Some(sample) {
					return Err(RemoteError::Protocol {
						detail: "executor metric slot disappeared",
					});
				}
				saw_metric = true;
			}
			MasterRunEvent::TaskFailed { task, fault } if task == expected_task => {
				return Err(RemoteError::Driver(fault));
			}
			_ => {
				return Err(RemoteError::Protocol {
					detail: "executor reported an event for an uncommanded task",
				});
			}
		}
	}
	panic!("executor-backed remote task did not complete");
}

fn complete_executor_master(channel: RuntimeChannel, program: ProvisionedProgram) -> RemoteResult<MasterComplete> {
	let mut master = master_run(channel, program)?;
	master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA)?;
	wait_executor_task(&mut master, MASTER_TO_WORKER, None, None)?;
	master.request_user_data(WORKER_TO_MASTER)?;
	wait_executor_task(&mut master, WORKER_TO_MASTER, Some(&MASTER_DATA), None)?;
	master.submit_task(METRIC_ONE)?;
	wait_executor_task(
		&mut master,
		METRIC_ONE,
		None,
		Some(RemoteMetricValue::F32(1.0)),
	)?;
	master.submit_task(METRIC_TWO)?;
	wait_executor_task(
		&mut master,
		METRIC_TWO,
		None,
		Some(RemoteMetricValue::F32(2.0)),
	)?;
	drive_exit(master.begin_exit()?)
}

fn complete_master<C: Into<RemoteChannel>>(
	channel: C,
	program: ProvisionedProgram,
	run_id: RunId,
) -> RemoteResult<MasterComplete> {
	let mut master = master_run_for(channel, program, run_id)?;
	master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA)?;
	master.request_user_data(WORKER_TO_MASTER)?;
	master.submit_task(METRIC_ONE)?;
	master.submit_task(METRIC_TWO)?;
	drive_exit(drive_loop(master, LoopObservation::default())?)
}

#[test]
fn driver_faults_reach_the_master_before_init_or_handshake_can_hang() {
	for fault_at in [FaultPoint::Prepare, FaultPoint::InitChunk] {
		let provisioned = program(DuplexMode::Full, 8);
		let (master_channel, worker_channel) = channels(4);
		let worker_program = provisioned.clone();
		let worker = thread::spawn(move || {
			complete_worker(
				worker_channel,
				worker_program,
				RUN,
				TestDriver::failing(fault_at),
			)
		});
		let master_error = master_run(master_channel, provisioned).expect_err("master must receive worker fault");
		assert_test_driver_fault(master_error);
		let worker_error = worker
			.join()
			.expect("worker thread")
			.expect_err("worker must return its reported fault");
		assert_test_driver_fault(worker_error);
	}
}

#[test]
fn loop_transfer_driver_fault_reaches_the_master() {
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(4);
	let worker_program = provisioned.clone();
	let worker = thread::spawn(move || {
		complete_worker(
			worker_channel,
			worker_program,
			RUN,
			TestDriver::failing(FaultPoint::ReceiveData),
		)
	});
	let mut master = success(master_run(master_channel, provisioned));
	success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
	let master_error = loop {
		match master.progress() {
			Ok(_) => thread::yield_now(),
			Err(error) => break error,
		}
	};
	assert_test_driver_fault(master_error);
	let worker_error = worker
		.join()
		.expect("worker thread")
		.expect_err("worker must return its reported fault");
	assert_test_driver_fault(worker_error);
}

#[test]
fn exit_release_and_finish_faults_reach_the_master_before_terminal_ack() {
	for fault_at in [FaultPoint::Release, FaultPoint::Finish] {
		let provisioned = program(DuplexMode::Full, 8);
		let (master_channel, worker_channel) = channels(4);
		let worker_program = provisioned.clone();
		let worker = thread::spawn(move || {
			complete_worker(
				worker_channel,
				worker_program,
				RUN,
				TestDriver::failing(fault_at),
			)
		});
		let mut master = success(master_run(master_channel, provisioned));
		success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
		success(master.request_user_data(WORKER_TO_MASTER));
		success(master.submit_task(METRIC_ONE));
		success(master.submit_task(METRIC_TWO));
		let mut exit = success(drive_loop(master, LoopObservation::default()));
		let master_error = loop {
			match exit.progress() {
				Ok(_) => thread::yield_now(),
				Err(error) => break error,
			}
		};
		assert_test_driver_fault(master_error);
		let worker_error = worker
			.join()
			.expect("worker thread")
			.expect_err("worker must return its reported fault");
		assert_test_driver_fault(worker_error);
	}
}

#[test]
fn provisioning_derives_cross_transfers_from_the_finalized_measured_topology() {
	let program = program(DuplexMode::Full, 8);
	assert_eq!(program.transfers().len(), 2);
	let master_to_worker = program
		.transfers()
		.iter()
		.find(|transfer| transfer.task == MASTER_TO_WORKER)
		.expect("master-to-worker transfer");
	assert_eq!(master_to_worker.direction, DataDirection::MasterToWorker);
	assert_eq!(master_to_worker.schedule, 1);
	assert_eq!(master_to_worker.bytes, ByteCount::new(16));
	assert_eq!(
		master_to_worker.claims.as_ref(),
		[DuplexClaim {
			resource: DuplexResourceId::new(1),
			mode: DuplexMode::Full,
		}]
	);
	let worker_to_master = program
		.transfers()
		.iter()
		.find(|transfer| transfer.task == WORKER_TO_MASTER)
		.expect("worker-to-master transfer");
	assert_eq!(worker_to_master.direction, DataDirection::WorkerToMaster);
	assert_eq!(worker_to_master.schedule, 0);
	assert_eq!(
		worker_to_master.claims.as_ref(),
		[DuplexClaim {
			resource: DuplexResourceId::new(2),
			mode: DuplexMode::Full,
		}]
	);
}

#[test]
fn provisioning_rejects_a_topology_identity_other_than_the_finalized_profile() {
	let (mut topology, bundle) = finalized_fixture(8, DuplexMode::Full, 16);
	topology.identity = TopologyIdentity::new(digest(99));
	let error = ProvisionedProgram::from_bundle(&bundle, &topology, &[WORKER_GPU], limits())
		.expect_err("topology identity mismatch must fail");
	assert!(matches!(error, RemoteError::ManifestMismatch(_)));
}

#[test]
fn session_rejects_identity_claims_that_differ_from_the_connected_transport() {
	let (master_channel, worker_channel) = channels(2);
	drop(worker_channel);
	let (master_endpoint, worker_endpoint) = endpoints();
	let wrong_identity = success(RemoteIdentity::new(worker_endpoint, master_endpoint));
	let error = match MasterHandshake::new(
		master_channel,
		wrong_identity,
		limits(),
		program(DuplexMode::Full, 8),
		RUN,
	) {
		Ok(_) => panic!("mismatched identity reached the handshake"),
		Err(error) => error,
	};
	assert!(matches!(error, RemoteError::InvalidConfiguration(_)));
}

#[test]
fn tcp_lifecycle_preserves_exact_init_bidirectional_data_metrics_and_release() {
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(4);
	let worker = spawn_worker(worker_channel, provisioned.clone());
	let mut master = success(master_run(master_channel, provisioned));
	let capacity = master.capacities();

	success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
	success(master.request_user_data(WORKER_TO_MASTER));
	success(master.submit_task(METRIC_ONE));
	success(master.submit_task(METRIC_TWO));
	assert_eq!(master.capacities(), capacity);

	let complete = success(drive_exit(success(drive_loop(
		master,
		LoopObservation::default(),
	))));
	assert!(!complete.was_cancelled());
	assert_eq!(complete.run_id(), RUN);
	let driver = success(worker.join().expect("worker thread"));
	assert_eq!(driver.image, IMAGE);
	assert_eq!(driver.received, MASTER_DATA);
	assert_eq!(driver.releases, 1);
	assert_eq!(driver.fatal_cleanups, 0);
	assert!(driver.finished);
}

#[test]
fn executor_driver_runs_only_the_exact_worker_projection() {
	let observer = Arc::new(ExecutorObserver::default());
	let (provisioned, driver) = executor_driver(ExecutorFakeBackend::new(Arc::clone(&observer)), 8);
	assert_eq!(driver.projection().devices(), [WORKER_GPU]);
	assert_eq!(
		driver.projection().task_role(INIT_WORKER),
		Some(recipe_executor::WorkerTaskRole::InitAdmission)
	);
	assert_eq!(
		driver.projection().task_role(MASTER_TO_WORKER),
		Some(recipe_executor::WorkerTaskRole::ExternalIngress)
	);
	assert_eq!(
		driver.projection().task_role(WORKER_TO_MASTER),
		Some(recipe_executor::WorkerTaskRole::ExternalEgress)
	);
	assert_eq!(
		driver.projection().task_role(METRIC_ONE),
		Some(recipe_executor::WorkerTaskRole::Local)
	);
	assert_eq!(driver.projection().task_role(INIT_MASTER), None);

	let (master_channel, worker_channel) = channels(4);
	let worker_program = provisioned.clone();
	let worker = thread::spawn(move || {
		let complete = complete_worker(worker_channel, worker_program, RUN, driver)?;
		let (_, driver) = complete.into_parts();
		Ok::<_, RemoteError>(driver)
	});
	let complete = success(complete_executor_master(master_channel, provisioned));
	assert!(!complete.was_cancelled());
	let driver = success(worker.join().expect("executor worker thread"));
	assert_eq!(driver.active_run(), None);
	assert_eq!(observer.binds.load(Ordering::SeqCst), 1);
	assert_eq!(observer.local_prepares.load(Ordering::SeqCst), 3);
	assert_eq!(observer.external_prepares.load(Ordering::SeqCst), 2);
	assert_eq!(observer.quiesces.load(Ordering::SeqCst), 0);
	assert_eq!(observer.releases.load(Ordering::SeqCst), 1);
	assert_eq!(observer.destroys.load(Ordering::SeqCst), 1);
}

#[test]
fn executor_driver_rejects_foreign_and_dependency_violating_commands() {
	let observer = Arc::new(ExecutorObserver::default());
	let (program, mut driver) = executor_driver(ExecutorFakeBackend::new(Arc::clone(&observer)), 8);
	success(driver.prepare(RUN, &program));
	let image_digest = Digest::new(Sha256::digest(IMAGE).into());
	success(driver.begin_init_image(
		WORKER_GPU,
		ByteCount::new(u64::try_from(IMAGE.len()).unwrap()),
		image_digest,
	));
	success(driver.begin_init_chunk(WORKER_GPU, 0, &IMAGE));
	assert_eq!(
		success(driver.poll_init_chunk(WORKER_GPU)),
		DriverTransferPoll::Complete { bytes: IMAGE.len() }
	);
	success(driver.finish_init_image(WORKER_GPU));
	assert_eq!(
		success(driver.poll_init_image(WORKER_GPU)),
		DriverTransferPoll::Pending
	);
	assert_eq!(
		success(driver.poll_init_image(WORKER_GPU)),
		DriverTransferPoll::Complete { bytes: IMAGE.len() }
	);

	let foreign = driver
		.submit_task(INIT_MASTER)
		.expect_err("foreign master task must not enter the worker executor");
	assert_eq!(
		foreign.code,
		DriverFaultCode::EXECUTOR_OPERATION_FAILED.get()
	);
	assert_eq!(foreign.detail, INIT_MASTER.get());

	let mut destination = [0_u8; 16];
	let dependency = driver
		.begin_produce_user_data(WORKER_TO_MASTER, &mut destination)
		.expect_err("egress must wait for its projected ingress dependency");
	assert_eq!(
		dependency.code,
		DriverFaultCode::EXECUTOR_OPERATION_FAILED.get()
	);
	assert_eq!(dependency.detail, WORKER_TO_MASTER.get());
	success(driver.cleanup_after_fault(dependency));
	assert_eq!(observer.quiesces.load(Ordering::SeqCst), 1);
	assert_eq!(observer.releases.load(Ordering::SeqCst), 1);
	assert_eq!(observer.destroys.load(Ordering::SeqCst), 1);
}

#[test]
fn executor_driver_construction_proves_assignment_and_program_identity() {
	let observer = Arc::new(ExecutorObserver::default());
	let (topology, bundle) = finalized_fixture(8, DuplexMode::Full, 16);
	let (_, other_bundle) = finalized_fixture(9, DuplexMode::Full, 16);
	let other_program = success(ProvisionedProgram::from_bundle(
		&other_bundle,
		&topology,
		&[WORKER_GPU],
		limits(),
	));
	let mismatch = ExecutorWorkerDriver::new(
		bundle.clone(),
		&topology,
		WorkerAssignment::new(MachineId::new(2), NodeId::new(2)),
		&other_program,
		ExecutorFakeBackend::new(Arc::clone(&observer)),
		success(ExecutorWatchdog::new(8)),
	)
	.expect_err("program from another finalized bundle must be rejected");
	assert!(matches!(
		mismatch,
		ExecutorDriverBuildError::ProgramMismatch(_)
	));

	let program = success(ProvisionedProgram::from_bundle(
		&bundle,
		&topology,
		&[WORKER_GPU],
		limits(),
	));
	let assignment = ExecutorWorkerDriver::new(
		bundle,
		&topology,
		WorkerAssignment::new(MachineId::new(1), NodeId::new(1)),
		&program,
		ExecutorFakeBackend::new(observer),
		success(ExecutorWatchdog::new(8)),
	)
	.expect_err("master node must not be projected as a worker");
	assert!(matches!(
		assignment,
		ExecutorDriverBuildError::Projection(
			recipe_executor::WorkerProjectionError::NodeIsNotWorker(node)
		) if node == NodeId::new(1)
	));
}

#[test]
fn executor_driver_fault_quiesces_releases_and_destroys_before_reporting() {
	let observer = Arc::new(ExecutorObserver::default());
	let (provisioned, driver) = executor_driver(
		ExecutorFakeBackend::failing_ingress(Arc::clone(&observer)),
		8,
	);
	let (master_channel, worker_channel) = channels(4);
	let worker_program = provisioned.clone();
	let worker = thread::spawn(move || complete_worker(worker_channel, worker_program, RUN, driver));
	let mut master = success(master_run(master_channel, provisioned));
	success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
	let master_fault = loop {
		match master.progress() {
			Ok(_) => thread::yield_now(),
			Err(RemoteError::Driver(fault)) => break fault,
			Err(error) => panic!("unexpected master failure: {error:?}"),
		}
	};
	assert_eq!(
		master_fault.code,
		DriverFaultCode::EXECUTOR_OPERATION_FAILED.get()
	);
	let worker_error = worker
		.join()
		.expect("executor worker thread")
		.expect_err("injected ingress fault must terminate the worker");
	assert!(matches!(
		worker_error,
		RemoteError::Driver(fault) if fault == master_fault
	));
	assert_eq!(observer.quiesces.load(Ordering::SeqCst), 1);
	assert_eq!(observer.releases.load(Ordering::SeqCst), 1);
	assert_eq!(observer.destroys.load(Ordering::SeqCst), 1);
}

#[test]
fn executor_driver_drop_cleans_an_active_session_after_disconnect() {
	let observer = Arc::new(ExecutorObserver::default());
	let (provisioned, driver) = executor_driver(ExecutorFakeBackend::new(Arc::clone(&observer)), 8);
	let (master_channel, worker_channel) = channels(4);
	let worker_program = provisioned.clone();
	let worker = thread::spawn(move || complete_worker(worker_channel, worker_program, RUN, driver));
	let master = success(master_run(master_channel, provisioned));
	drop(master);
	let worker_error = worker
		.join()
		.expect("executor worker thread")
		.expect_err("disconnected master must terminate the worker");
	assert!(matches!(
		worker_error,
		RemoteError::Transport(_) | RemoteError::Poisoned
	));
	assert_eq!(observer.quiesces.load(Ordering::SeqCst), 1);
	assert_eq!(observer.releases.load(Ordering::SeqCst), 1);
	assert_eq!(observer.destroys.load(Ordering::SeqCst), 1);
}

#[test]
fn cancellation_quiesces_active_work_and_releases_every_worker_arena() {
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(4);
	let worker = spawn_worker(worker_channel, provisioned.clone());
	let mut master = success(master_run(master_channel, provisioned));
	success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
	success(master.request_user_data(WORKER_TO_MASTER));
	success(master.submit_task(METRIC_ONE));
	let mut cancelling = master.cancel(CancelReason::new(7));
	let complete = loop {
		match success(cancelling.progress()) {
			Advance::Pending(next) => cancelling = next,
			Advance::Ready(complete) => break complete,
		}
		thread::yield_now();
	};
	assert!(complete.was_cancelled());
	let driver = success(worker.join().expect("worker thread"));
	assert_eq!(driver.releases, 1);
	assert_eq!(driver.fatal_cleanups, 0);
	assert!(driver.active_tasks.iter().all(Option::is_none));
	assert!(driver.incoming_transfer.is_none());
	assert!(driver.outgoing_transfer.is_none());
	assert!(driver.finished);
}

#[test]
fn one_connection_runs_two_monotonic_lifecycles_without_reusing_schedules() {
	let next_run = RunId::new(RUN.get() + 1);
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(4);
	let worker_program = provisioned.clone();
	let worker = thread::spawn(move || -> RemoteResult<(RemoteChannel, TestDriver)> {
		let first = complete_worker(
			worker_channel,
			worker_program.clone(),
			RUN,
			TestDriver::new(),
		)?;
		let (channel, driver) = first.into_parts();
		let second = complete_worker(channel, worker_program, next_run, driver)?;
		Ok(second.into_parts())
	});

	let first = success(complete_master(master_channel, provisioned.clone(), RUN));
	assert_eq!(first.run_id(), RUN);
	let second = success(complete_master(
		first.into_channel(),
		provisioned.clone(),
		next_run,
	));
	assert_eq!(second.run_id(), next_run);
	let master_channel = second.into_channel();
	let (worker_channel, driver) = success(worker.join().expect("worker thread"));
	assert_eq!(driver.prepares, 2);
	assert_eq!(driver.releases, 2);
	assert!(driver.finished);
	drop(worker_channel);

	let (master_endpoint, worker_endpoint) = endpoints();
	let identity = success(RemoteIdentity::new(master_endpoint, worker_endpoint));
	let error = match MasterHandshake::new(master_channel, identity, limits(), provisioned, next_run) {
		Ok(_) => panic!("reused run id reached the handshake"),
		Err(error) => error,
	};
	assert!(matches!(error, RemoteError::InvalidConfiguration(_)));
}

#[test]
fn shared_half_duplex_token_serializes_opposing_transfers() {
	let provisioned = program(DuplexMode::Half, 8);
	let (master_channel, worker_channel) = channels(4);
	let worker = spawn_worker(worker_channel, provisioned.clone());
	let mut master = success(master_run(master_channel, provisioned));

	success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
	assert!(matches!(
		master.request_user_data(WORKER_TO_MASTER),
		Err(RemoteError::Backpressured("half-duplex capacity token"))
	));
	let mut first_complete = false;
	for _ in 0..200_000 {
		if let Some(MasterRunEvent::TaskComplete(MASTER_TO_WORKER)) = success(master.progress()) {
			first_complete = true;
			break;
		}
		thread::yield_now();
	}
	assert!(first_complete);
	success(master.request_user_data(WORKER_TO_MASTER));
	success(master.submit_task(METRIC_ONE));
	success(master.submit_task(METRIC_TWO));
	let mut observation = LoopObservation::default();
	observation.completed[0] = true;
	let complete = success(drive_exit(success(drive_loop(master, observation))));
	assert!(!complete.was_cancelled());
	let driver = success(worker.join().expect("worker thread"));
	assert_eq!(driver.releases, 1);
}

#[test]
fn fixed_control_capacity_backpressures_and_can_be_retried() {
	let provisioned = program(DuplexMode::Full, 8);
	let (master_channel, worker_channel) = channels(1);
	let worker = spawn_worker(worker_channel, provisioned.clone());
	let mut master = success(master_run(master_channel, provisioned));

	success(master.submit_task(METRIC_ONE));
	assert!(matches!(
		master.submit_task(METRIC_TWO),
		Err(RemoteError::Backpressured("control lane"))
	));
	let mut observation = LoopObservation::default();
	for _ in 0..200_000 {
		if let Some(event) = success(master.progress()) {
			success(observe_event(&mut master, &mut observation, event));
		}
		match master.submit_task(METRIC_TWO) {
			Ok(()) => break,
			Err(RemoteError::Backpressured("control lane")) => thread::yield_now(),
			Err(error) => panic!("unexpected retry error: {error}"),
		}
	}
	success(master.send_user_data(MASTER_TO_WORKER, &MASTER_DATA));
	for _ in 0..200_000 {
		match master.request_user_data(WORKER_TO_MASTER) {
			Ok(()) => break,
			Err(RemoteError::Backpressured("control lane")) => {
				if let Some(event) = success(master.progress()) {
					success(observe_event(&mut master, &mut observation, event));
				}
			}
			Err(error) => panic!("unexpected request retry error: {error}"),
		}
	}
	let complete = success(drive_exit(success(drive_loop(master, observation))));
	assert!(!complete.was_cancelled());
	success(worker.join().expect("worker thread"));
}

#[test]
fn exact_program_digest_mismatch_fails_before_prepare() {
	let (master_channel, worker_channel) = channels(2);
	let worker_program = program(DuplexMode::Full, 9);
	let worker: JoinHandle<RemoteResult<()>> = thread::spawn(move || {
		let (master_endpoint, worker_endpoint) = endpoints();
		let identity = success(RemoteIdentity::new(worker_endpoint, master_endpoint));
		let mut handshake = WorkerHandshake::new(
			worker_channel,
			identity,
			limits(),
			worker_program,
			RUN,
			TestDriver::new(),
		)?;
		loop {
			match handshake.progress()? {
				Advance::Pending(next) => handshake = next,
				Advance::Ready(_) => panic!("mismatched worker reached init"),
			}
			thread::yield_now();
		}
	});
	let (master_endpoint, worker_endpoint) = endpoints();
	let identity = success(RemoteIdentity::new(master_endpoint, worker_endpoint));
	let mut master = success(MasterHandshake::new(
		master_channel,
		identity,
		limits(),
		program(DuplexMode::Full, 8),
		RUN,
	));
	for _ in 0..200_000 {
		match master.progress() {
			Ok(Advance::Pending(next)) => master = next,
			Ok(Advance::Ready(_)) => panic!("mismatched master reached init"),
			Err(_) => break,
		}
		thread::yield_now();
	}
	let worker_error = worker
		.join()
		.expect("worker thread")
		.expect_err("worker must reject mismatched program");
	assert!(matches!(worker_error, RemoteError::ManifestMismatch(_)));
}

#[test]
fn disconnected_peer_is_reported_and_poisoned() {
	let (master_channel, worker_channel) = channels(2);
	drop(worker_channel);
	let (master_endpoint, worker_endpoint) = endpoints();
	let identity = success(RemoteIdentity::new(master_endpoint, worker_endpoint));
	let mut master = success(MasterHandshake::new(
		master_channel,
		identity,
		limits(),
		program(DuplexMode::Full, 8),
		RUN,
	));
	for _ in 0..200_000 {
		match master.progress() {
			Ok(Advance::Pending(next)) => master = next,
			Ok(Advance::Ready(_)) => panic!("disconnected master reached init"),
			Err(RemoteError::Transport(_)) => return,
			Err(error) => panic!("unexpected disconnect error: {error}"),
		}
		thread::yield_now();
	}
	panic!("disconnect was not observed");
}

#[test]
fn provisioning_rejects_cross_payload_that_cannot_fit_the_codec() {
	let (topology, bundle) = finalized_fixture(8, DuplexMode::Full, 500);
	let error = ProvisionedProgram::from_bundle(&bundle, &topology, &[WORKER_GPU], limits())
		.expect_err("oversized payload must fail during provisioning");
	assert!(matches!(
		error,
		RemoteError::InvalidConfiguration(_) | RemoteError::CapacityExhausted(_)
	));
}
