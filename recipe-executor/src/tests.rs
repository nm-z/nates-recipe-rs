use std::collections::BTreeMap;
use std::fmt;

use recipe_core::*;

use crate::backend::sealed::Sealed;
use crate::*;

const GPU: DeviceId = DeviceId::new(1);
const RAM: DeviceId = DeviceId::new(2);
const INIT_GPU: TaskId = TaskId::new(1);
const INIT_RAM: TaskId = TaskId::new(2);
const CALCULATION: TaskId = TaskId::new(3);
const INTERNAL_TRANSFER: TaskId = TaskId::new(4);
const METRIC_OLD: TaskId = TaskId::new(5);
const METRIC_NEW: TaskId = TaskId::new(6);
const EXIT_GPU: TaskId = TaskId::new(7);
const FAULT_READBACK: TaskId = TaskId::new(8);
const METRIC_SLOT: MetricSlotId = MetricSlotId::new(1);
const FAULT_SLOT: MetricSlotId = MetricSlotId::new(2);

#[derive(Clone, Debug)]
enum FakeStep {
	Pending,
	Complete,
	Fail(&'static str),
}

#[derive(Debug)]
struct FakeBackend {
	scripts: BTreeMap<TaskId, Vec<FakeStep>>,
	metric_values: BTreeMap<TaskId, MetricValue>,
	next_resource: u64,
	completed_lifecycles: u64,
}

impl FakeBackend {
	fn new() -> Self {
		Self {
			scripts: BTreeMap::new(),
			metric_values: BTreeMap::new(),
			next_resource: 1,
			completed_lifecycles: 0,
		}
	}

	fn with_script(mut self, task: TaskId, script: impl IntoIterator<Item = FakeStep>) -> Self {
		self.scripts.insert(task, script.into_iter().collect());
		self
	}

	fn with_metric(mut self, task: TaskId, value: MetricValue) -> Self {
		self.metric_values.insert(task, value);
		self
	}
}

fn record_physical(physical_calls: &mut PhysicalCallBatch, call: PhysicalCall) -> std::result::Result<(), FakeError> {
	physical_calls.try_push(call).map_err(|error| match error {
		PhysicalCallBatchOverflow => FakeError("physical call batch overflow"),
	})
}

impl Sealed for FakeBackend {}

#[derive(Debug)]
struct FakeResource {
	generation: u64,
}

#[derive(Debug)]
struct FakeArena {
	device: DeviceId,
	bytes: ByteCount,
}

#[derive(Debug)]
struct FakePending {
	task: TaskId,
	class: WorkClass,
	steps: Vec<FakeStep>,
	next_step: usize,
	metric: Option<MetricValue>,
}

#[derive(Debug)]
struct FakeError(&'static str);

impl fmt::Display for FakeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.0)
	}
}

impl std::error::Error for FakeError {}

impl Backend for FakeBackend {
	type Arena = FakeArena;
	type Resource = FakeResource;
	type Pending = FakePending;
	type Error = FakeError;

	fn bind_resources(
		&mut self,
		_bundle: &FinalizedBundle,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Resource, Self::Error> {
		let generation = self.next_resource;
		self.next_resource += 1;
		record_physical(physical_calls, PhysicalCall::BindResources)?;
		Ok(FakeResource { generation })
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Pending, Self::Error> {
		assert_ne!(resource.generation, 0);
		let steps = self
			.scripts
			.get(&request.task)
			.cloned()
			.unwrap_or_else(|| vec![FakeStep::Complete]);
		record_physical(
			physical_calls,
			PhysicalCall::PreparePending { task: request.task },
		)?;
		Ok(FakePending {
			task: request.task,
			class: request.class,
			steps,
			next_step: 0,
			metric: None,
		})
	}

	fn allocate_arena(
		&mut self,
		resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Arena, Self::Error> {
		assert_ne!(resource.generation, 0);
		record_physical(
			physical_calls,
			PhysicalCall::AllocateArena {
				device: layout.device,
				bytes: layout.size,
			},
		)?;
		Ok(FakeArena {
			device: layout.device,
			bytes: layout.size,
		})
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		assert_ne!(resource.generation, 0);
		assert_eq!(arenas.len(), 2);
		let task = work.task();
		let class = work.class();
		assert_eq!(pending.task, task);
		assert_eq!(pending.class, class);
		pending.next_step = 0;
		let metric = match work {
			BackendWork::InitAdmission(work) => {
				let device = work.destination.device;
				assert_eq!(arenas.get(device).map(|arena| arena.device), Some(device));
				assert_eq!(work.destination.arena_offset, ByteOffset::new(0));
				let split = work.image.len() / 2;
				let first = ByteCount::new(u64::try_from(split).unwrap());
				let second = ByteCount::new(u64::try_from(work.image.len() - split).unwrap());
				record_physical(
					physical_calls,
					PhysicalCall::AdmissionChunk {
						task,
						device,
						bytes: first,
						chunk_index: 0,
					},
				)?;
				record_physical(
					physical_calls,
					PhysicalCall::AdmissionChunk {
						task,
						device,
						bytes: second,
						chunk_index: 1,
					},
				)?;
				None
			}
			BackendWork::Calculation(work) => {
				assert_eq!(work.artifact, ArtifactId::new(1));
				assert_eq!(work.inputs.len(), 1);
				assert_eq!(work.outputs.len(), 1);
				assert_eq!(work.inputs[0], work.outputs[0]);
				if let Some(fault_flag) = work.fault_flag {
					assert_eq!(fault_flag.dtype, DType::I32);
					assert_eq!(fault_flag.device, GPU);
					assert_eq!(fault_flag.bytes, ByteCount::new(4));
				}
				record_physical(physical_calls, PhysicalCall::SubmitCalculation { task })?;
				None
			}
			BackendWork::InternalTransfer(work) => {
				let ResolvedTransferEndpoint::Device(source) = work.source else {
					panic!("internal transfer source must be resolved");
				};
				let ResolvedTransferEndpoint::Device(destination) = work.destination else {
					panic!("internal transfer destination must be resolved");
				};
				assert_eq!(source.device, GPU);
				assert_eq!(source.value, ValueId::new(1));
				assert_eq!(source.arena_offset, ByteOffset::new(0));
				assert_eq!(destination.device, RAM);
				assert_eq!(destination.value, ValueId::new(2));
				assert_eq!(destination.arena_offset, ByteOffset::new(0));
				assert_eq!(work.route, &[LinkId::new(2)]);
				record_physical(
					physical_calls,
					PhysicalCall::SubmitInternalTransfer { task },
				)?;
				None
			}
			BackendWork::Metric(work) => {
				let metric = Some(self
					.metric_values
					.get(&task)
					.copied()
					.unwrap_or(match work.purpose {
						MetricPurpose::User if task == METRIC_OLD => MetricValue::F32(1.0),
						MetricPurpose::User => MetricValue::F32(2.0),
						MetricPurpose::FaultReadback { .. } => MetricValue::I32(0),
					}));
				record_physical(
					physical_calls,
					PhysicalCall::SubmitMetric {
						task,
						slot: work.slot,
					},
				)?;
				metric
			}
			BackendWork::ExitTransfer(_) => {
				record_physical(physical_calls, PhysicalCall::SubmitExitTransfer { task })?;
				None
			}
		};
		pending.metric = metric;
		Ok(())
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<BackendPoll, Self::Error> {
		assert_ne!(resource.generation, 0);
		let step = pending
			.steps
			.get(pending.next_step)
			.cloned()
			.unwrap_or(FakeStep::Complete);
		pending.next_step = pending.next_step.saturating_add(1);
		match step {
			FakeStep::Pending => {
				record_physical(
					physical_calls,
					PhysicalCall::Poll {
						task: pending.task,
						status: PhysicalPollStatus::Pending,
					},
				)?;
				Ok(BackendPoll::Pending)
			}
			FakeStep::Complete => {
				record_physical(
					physical_calls,
					PhysicalCall::Poll {
						task: pending.task,
						status: PhysicalPollStatus::Complete,
					},
				)?;
				Ok(BackendPoll::Complete {
					metric: (pending.class == WorkClass::Metric)
						.then_some(pending.metric)
						.flatten(),
				})
			}
			FakeStep::Fail(message) => {
				record_physical(
					physical_calls,
					PhysicalCall::Poll {
						task: pending.task,
						status: PhysicalPollStatus::Failed,
					},
				)?;
				Err(FakeError(message))
			}
		}
	}

	fn collect_exit(
		&mut self,
		resource: &mut Self::Resource,
		_arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: TransferWork<'_>,
		destination: &mut [u8],
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		assert_ne!(resource.generation, 0);
		assert_eq!(pending.task, work.task);
		destination.fill(0xA5);
		record_physical(
			physical_calls,
			PhysicalCall::CollectExit {
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
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		assert_ne!(resource.generation, 0);
		assert_eq!(arena.device, device);
		assert!(arena.bytes >= ByteCount::new(16));
		record_physical(physical_calls, PhysicalCall::ReleaseArena { device })
	}

	fn destroy_resources(
		&mut self,
		resource: Self::Resource,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		assert_ne!(resource.generation, 0);
		self.completed_lifecycles += 1;
		record_physical(physical_calls, PhysicalCall::DestroyResources)
	}
}

fn digest(value: u8) -> Digest {
	Digest::new([value; 32])
}

fn label(value: &str) -> Label {
	Label::new(value).unwrap()
}

fn property<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Override)
}

fn build_fixture(checked: bool) -> FinalizedBundle {
	let machine = MachineId::new(1);
	let topology_id = TopologyIdentity::new(digest(1));
	let target = TargetIdentity {
		backend: label("fake"),
		architecture: label("test-gpu"),
		abi: label("recipe-test-v1"),
	};
	let topology = Topology {
		identity: topology_id,
		machines: vec![Machine {
			id: machine,
			name: label("test-machine"),
		}],
		nodes: vec![Node {
			id: NodeId::new(1),
			role: NodeRole::Master,
			machine,
			devices: vec![GPU, RAM],
		}],
		devices: vec![
			Device {
				id: GPU,
				machine,
				kind: DeviceKind::GpuMemory,
				capacity: property(ByteCount::new(12_000_000_000)),
				transfer_rate: property(BytesPerSecond::new(432_000_000_000).unwrap()),
				calculation_rate: Some(property(FlopsPerSecond::new(380_000_000_000).unwrap())),
			},
			Device {
				id: RAM,
				machine,
				kind: DeviceKind::Ram,
				capacity: property(ByteCount::new(48_000_000_000)),
				transfer_rate: property(BytesPerSecond::new(90_000_000_000).unwrap()),
				calculation_rate: None,
			},
		],
		links: vec![
			DirectedLink {
				id: LinkId::new(1),
				transport: TransportId::new(1),
				kind: TransportKind::Pcie,
				duplex: DuplexMode::Full,
				from: RAM,
				to: GPU,
				bandwidth: property(BytesPerSecond::new(16_000_000_000).unwrap()),
				maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
				capacity_resource: DuplexResourceId::new(1),
			},
			DirectedLink {
				id: LinkId::new(2),
				transport: TransportId::new(1),
				kind: TransportKind::Pcie,
				duplex: DuplexMode::Full,
				from: GPU,
				to: RAM,
				bandwidth: property(BytesPerSecond::new(16_000_000_000).unwrap()),
				maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
				capacity_resource: DuplexResourceId::new(2),
			},
		],
	};

	let discovery_id = DiscoveryIdentity::new(digest(2));
	let discovery = DiscoveryProfile {
		identity: discovery_id,
		topology: topology_id,
		devices: vec![
			DiscoveredDevice {
				device: GPU,
				available: true,
				total_capacity: property(ByteCount::new(12_000_000_000)),
				transfer: TransferCapability {
					rate: property(BytesPerSecond::new(432_000_000_000).unwrap()),
					maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
					asynchronous_submission: true,
					overlaps_calculation: true,
				},
				calculation: Some(CalculationCapability {
					target: target.clone(),
					rate: property(FlopsPerSecond::new(380_000_000_000).unwrap()),
					asynchronous_submission: true,
					maximum_concurrent_tasks: 2,
				}),
			},
			DiscoveredDevice {
				device: RAM,
				available: true,
				total_capacity: property(ByteCount::new(48_000_000_000)),
				transfer: TransferCapability {
					rate: property(BytesPerSecond::new(90_000_000_000).unwrap()),
					maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
					asynchronous_submission: true,
					overlaps_calculation: true,
				},
				calculation: None,
			},
		],
		links: vec![
			DiscoveredLink {
				link: LinkId::new(1),
				available: true,
				bandwidth: property(BytesPerSecond::new(16_000_000_000).unwrap()),
				maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
				asynchronous_submission: true,
			},
			DiscoveredLink {
				link: LinkId::new(2),
				available: true,
				bandwidth: property(BytesPerSecond::new(16_000_000_000).unwrap()),
				maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
				asynchronous_submission: true,
			},
		],
	};

	let scalar_input = ScalarValueId::new(1);
	let scalar_output = ScalarValueId::new(2);
	let finite = ScalarValueId::new(3);
	let required = ScalarValueId::new(4);
	let kernel_id = KernelTemplateId::new(1);
	let mut instructions = vec![ScalarInstruction {
		result: scalar_output,
		dtype: DType::F32,
		opcode: ScalarOpcode::Negate,
		operands: vec![scalar_input],
	}];
	if checked {
		instructions.extend([
			ScalarInstruction {
				result: finite,
				dtype: DType::I32,
				opcode: ScalarOpcode::IsFinite,
				operands: vec![scalar_output],
			},
			ScalarInstruction {
				result: required,
				dtype: DType::I32,
				opcode: ScalarOpcode::Require,
				operands: vec![finite],
			},
		]);
	}
	let kernel = KernelTemplate {
		id: kernel_id,
		index_space: IndexSpace::new(vec![ElementCount::new(4).unwrap()]).unwrap(),
		inputs: vec![KernelInput {
			id: KernelInputId::new(1),
			dtype: DType::F32,
			access: recipe_core::StaticBufferAccess::linear(ElementCount::new(4).unwrap(), DType::F32).unwrap(),
		}],
		outputs: vec![KernelOutput {
			id: KernelOutputId::new(1),
			dtype: DType::F32,
			access: recipe_core::StaticBufferAccess::linear(ElementCount::new(4).unwrap(), DType::F32).unwrap(),
		}],
		program: ScalarProgram {
			inputs: vec![ScalarInput {
				id: scalar_input,
				dtype: DType::F32,
			}],
			constants: vec![],
			instructions,
			outputs: vec![scalar_output],
		},
		alias_rules: vec![AliasRule {
			input: KernelInputId::new(1),
			output: KernelOutputId::new(1),
			permission: AliasPermission::MustAliasExact,
		}],
	};
	let artifact_id = ArtifactId::new(1);
	let artifact = ArtifactIdentity {
		id: artifact_id,
		digest: digest(3),
		format: label("test-object"),
		target,
		toolchain: ToolchainIdentity {
			name: label("test-compiler"),
			version: label("1"),
			digest: digest(4),
		},
		entry_symbol: label("negate"),
		kernel_template: kernel_id,
		resources: KernelResourceBounds {
			private_bytes_per_lane: ByteCount::ZERO,
			shared_bytes_per_workgroup: ByteCount::ZERO,
			scratch_bytes_per_dispatch: ByteCount::ZERO,
			maximum_workgroup_lanes: 256,
		},
		build: None,
	};

	let queue_gpu = QueueSlotId::new(1);
	let queue_ram = QueueSlotId::new(2);
	let queue_transfer = QueueSlotId::new(3);
	let completion_gpu = CompletionSlotId::new(1);
	let completion_ram = CompletionSlotId::new(2);
	let completion_transfer = CompletionSlotId::new(3);
	let metric_id = MetricId::new(1);
	let fault_metric_id = MetricId::new(2);
	let mut metric_slots = vec![MetricSlot {
		id: METRIC_SLOT,
		metric: metric_id,
	}];
	if checked {
		metric_slots.push(MetricSlot {
			id: FAULT_SLOT,
			metric: fault_metric_id,
		});
	}
	let resources = ResourceManifest {
		queues: vec![
			QueueSlot {
				id: queue_gpu,
				device: GPU,
			},
			QueueSlot {
				id: queue_ram,
				device: RAM,
			},
			QueueSlot {
				id: queue_transfer,
				device: GPU,
			},
		],
		completions: vec![
			CompletionSlot {
				id: completion_gpu,
				device: GPU,
			},
			CompletionSlot {
				id: completion_ram,
				device: RAM,
			},
			CompletionSlot {
				id: completion_transfer,
				device: GPU,
			},
		],
		metrics: metric_slots,
		pinned_staging: vec![],
		scratch: vec![],
	};

	let value_gpu = ValueId::new(1);
	let value_ram = ValueId::new(2);
	let gpu_image = ValueId::new(3);
	let fault_flag = ValueId::new(4);
	let fault_value = checked.then_some(fault_flag);
	let admission_value = if checked { gpu_image } else { value_gpu };
	let gpu_admission_bytes = if checked {
		ByteCount::new(20)
	} else {
		ByteCount::new(16)
	};
	let mut values = vec![
		ValueSpec {
			id: value_gpu,
			dtype: DType::F32,
			bytes: ByteCount::new(16),
			device: GPU,
			producer: Some(INIT_GPU),
		},
		ValueSpec {
			id: value_ram,
			dtype: DType::F32,
			bytes: ByteCount::new(16),
			device: RAM,
			producer: Some(INIT_RAM),
		},
	];
	if checked {
		values.extend([
			ValueSpec {
				id: gpu_image,
				dtype: DType::I32,
				bytes: ByteCount::new(20),
				device: GPU,
				producer: Some(INIT_GPU),
			},
			ValueSpec {
				id: fault_flag,
				dtype: DType::I32,
				bytes: ByteCount::new(4),
				device: GPU,
				producer: Some(INIT_GPU),
			},
		]);
	}
	let window = |start, end| ScheduleWindow {
		start: Nanoseconds::new(start),
		end: Nanoseconds::new(end),
	};
	let draft_id = DraftIdentity::new(digest(5));
	let candidate = CandidateIdentity::new(digest(6));
	let draft = DraftPlan {
		identity: draft_id,
		candidate,
		discovery: discovery_id,
		topology: topology_id,
		values,
		kernels: vec![kernel],
		artifacts: vec![artifact.clone()],
		artifact_builds: Vec::new(),
		tasks: vec![
			Task {
				id: INIT_GPU,
				phase: RunPhase::Init,
				window: window(0, 1),
				dependencies: vec![],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::External,
					destination: TransferEndpoint::Device {
						device: GPU,
						value: admission_value,
					},
					bytes: gpu_admission_bytes,
					route: vec![],
					lane_claims: vec![TransferLaneClaim::External {
						device: GPU,
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: queue_gpu,
						completion: completion_gpu,
					},
				}),
			},
			Task {
				id: INIT_RAM,
				phase: RunPhase::Init,
				window: window(0, 1),
				dependencies: vec![],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::External,
					destination: TransferEndpoint::Device {
						device: RAM,
						value: value_ram,
					},
					bytes: ByteCount::new(16),
					route: vec![],
					lane_claims: vec![TransferLaneClaim::External {
						device: RAM,
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: queue_ram,
						completion: completion_ram,
					},
				}),
			},
			Task {
				id: CALCULATION,
				phase: RunPhase::Loop,
				window: window(1, 3),
				dependencies: vec![INIT_GPU],
				kind: TaskKind::Calculation(CalculationTask {
					device: GPU,
					kernel_template: kernel_id,
					artifact: artifact_id,
					inputs: vec![value_gpu],
					outputs: vec![value_gpu],
					fault_flag: fault_value,
					work: FlopCount::new(4),
					submission: SubmissionSlots {
						queue: queue_gpu,
						completion: completion_gpu,
					},
				}),
			},
			Task {
				id: INTERNAL_TRANSFER,
				phase: RunPhase::Loop,
				window: window(1, 3),
				dependencies: vec![INIT_GPU, INIT_RAM],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::Device {
						device: GPU,
						value: value_gpu,
					},
					destination: TransferEndpoint::Device {
						device: RAM,
						value: value_ram,
					},
					bytes: ByteCount::new(16),
					route: vec![LinkId::new(2)],
					lane_claims: vec![TransferLaneClaim::Link {
						link: LinkId::new(2),
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: queue_transfer,
						completion: completion_transfer,
					},
				}),
			},
		]
		.into_iter()
		.chain(checked.then_some(Task {
			id: FAULT_READBACK,
			phase: RunPhase::Loop,
			window: window(3, 4),
			dependencies: vec![CALCULATION],
			kind: TaskKind::Metric(MetricTask {
				purpose: MetricPurpose::FaultReadback {
					calculation: CALCULATION,
				},
				metric: fault_metric_id,
				value: fault_flag,
				slot: FAULT_SLOT,
			}),
		}))
		.chain([
			Task {
				id: METRIC_OLD,
				phase: RunPhase::Loop,
				window: if checked { window(4, 5) } else { window(3, 4) },
				dependencies: vec![if checked { FAULT_READBACK } else { CALCULATION }],
				kind: TaskKind::Metric(MetricTask {
					purpose: MetricPurpose::User,
					metric: metric_id,
					value: value_gpu,
					slot: METRIC_SLOT,
				}),
			},
			Task {
				id: METRIC_NEW,
				phase: RunPhase::Loop,
				window: if checked { window(5, 6) } else { window(4, 5) },
				dependencies: vec![METRIC_OLD],
				kind: TaskKind::Metric(MetricTask {
					purpose: MetricPurpose::User,
					metric: metric_id,
					value: value_gpu,
					slot: METRIC_SLOT,
				}),
			},
			Task {
				id: EXIT_GPU,
				phase: RunPhase::Exit,
				window: if checked { window(6, 7) } else { window(5, 6) },
				dependencies: vec![METRIC_NEW],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::Device {
						device: GPU,
						value: value_gpu,
					},
					destination: TransferEndpoint::External,
					bytes: ByteCount::new(16),
					route: vec![],
					lane_claims: vec![TransferLaneClaim::External {
						device: GPU,
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: queue_transfer,
						completion: completion_transfer,
					},
				}),
			},
		])
		.collect(),
		resources: resources.clone(),
		arena_objects: vec![
			ArenaObject {
				id: ArenaObjectId::new(1),
				device: GPU,
				bytes: gpu_admission_bytes,
				alignment: ByteCount::new(16),
				lifetime: if checked { window(0, 7) } else { window(0, 6) },
			},
			ArenaObject {
				id: ArenaObjectId::new(2),
				device: RAM,
				bytes: ByteCount::new(16),
				alignment: ByteCount::new(16),
				lifetime: window(0, 6),
			},
		],
		value_bindings: vec![
			ValueBinding {
				value: value_gpu,
				object: ArenaObjectId::new(1),
				object_offset: ByteOffset::new(0),
			},
			ValueBinding {
				value: value_ram,
				object: ArenaObjectId::new(2),
				object_offset: ByteOffset::new(0),
			},
		]
		.into_iter()
		.chain(
			checked
				.then_some([
					ValueBinding {
						value: gpu_image,
						object: ArenaObjectId::new(1),
						object_offset: ByteOffset::new(0),
					},
					ValueBinding {
						value: fault_flag,
						object: ArenaObjectId::new(1),
						object_offset: ByteOffset::new(16),
					},
				])
				.into_iter()
				.flatten(),
		)
		.collect(),
		value_aliases: Vec::new(),
		init_images: vec![
			InitDataImage {
				device: GPU,
				image: admission_value,
				bytes: gpu_admission_bytes,
				members: vec![InitDataImageMember {
					logical: value_gpu,
					physical: value_gpu,
					dtype: DType::F32,
					bytes: ByteCount::new(16),
					image_offset: ByteOffset::new(0),
				}],
			},
			InitDataImage {
				device: RAM,
				image: value_ram,
				bytes: ByteCount::new(16),
				members: vec![InitDataImageMember {
					logical: value_ram,
					physical: value_ram,
					dtype: DType::F32,
					bytes: ByteCount::new(16),
					image_offset: ByteOffset::new(0),
				}],
			},
		],
		releases: vec![ArenaRelease { device: GPU }, ArenaRelease { device: RAM }],
	};

	let reservations = ReservationLedger {
		entries: vec![
			ReservationEntry {
				device: GPU,
				name: label("user-gpu"),
				bytes: EXACT_USER_RESERVATION,
				mechanism: ReservationMechanism::HeldAllocation,
			},
			ReservationEntry {
				device: RAM,
				name: label("user-ram"),
				bytes: EXACT_USER_RESERVATION,
				mechanism: ReservationMechanism::EnforcedQuota,
			},
		],
	};
	let zero = property(ByteCount::ZERO);
	let capacity = CapacityLedger {
		entries: vec![
			CapacityLedgerEntry {
				device: GPU,
				total: property(ByteCount::new(12_000_000_000)),
				runtime_overhead: zero,
				fragmentation: zero,
				safety_headroom: zero,
				recipe_usable: property(ByteCount::new(11_000_000_000)),
			},
			CapacityLedgerEntry {
				device: RAM,
				total: property(ByteCount::new(48_000_000_000)),
				runtime_overhead: zero,
				fragmentation: zero,
				safety_headroom: zero,
				recipe_usable: property(ByteCount::new(47_000_000_000)),
			},
		],
	};
	let realization = RealizationProfile {
		identity: RealizationIdentity::new(digest(7)),
		draft: draft_id,
		candidate,
		discovery: discovery_id,
		topology: topology_id,
		artifacts: vec![artifact],
		resources,
		reservations,
		capacity,
	};
	let layouts = vec![
		ArenaLayout {
			device: GPU,
			size: gpu_admission_bytes,
			allocations: vec![ArenaAllocation {
				object: ArenaObjectId::new(1),
				offset: ByteOffset::new(0),
			}],
		},
		ArenaLayout {
			device: RAM,
			size: ByteCount::new(16),
			allocations: vec![ArenaAllocation {
				object: ArenaObjectId::new(2),
				offset: ByteOffset::new(0),
			}],
		},
	];

	FinalizedBundle::finalize(
		BundleIdentity::new(digest(8)),
		&topology,
		&discovery,
		draft,
		realization,
		layouts,
	)
	.unwrap()
}

fn fixture() -> FinalizedBundle {
	build_fixture(false)
}

fn fault_fixture() -> FinalizedBundle {
	build_fixture(true)
}

fn images() -> Vec<DeviceImage> {
	vec![
		DeviceImage::new(GPU, ValueId::new(1), vec![1; 16]),
		DeviceImage::new(RAM, ValueId::new(2), vec![2; 16]),
	]
}

fn fault_images() -> Vec<DeviceImage> {
	vec![
		DeviceImage::new(GPU, ValueId::new(3), vec![1; 20]),
		DeviceImage::new(RAM, ValueId::new(2), vec![2; 16]),
	]
}

fn initialized(backend: FakeBackend, watchdog: Watchdog, run_id: u64) -> InitializedRun<FakeBackend> {
	PreparedRun::prepare(RunId::new(run_id), fixture(), backend, watchdog)
		.unwrap()
		.initialize(images())
		.unwrap()
}

fn fault_initialized(backend: FakeBackend, watchdog: Watchdog, run_id: u64) -> InitializedRun<FakeBackend> {
	PreparedRun::prepare(RunId::new(run_id), fault_fixture(), backend, watchdog)
		.unwrap()
		.initialize(fault_images())
		.unwrap()
}

fn complete_loop(initialized: InitializedRun<FakeBackend>) -> ExitedLoop<FakeBackend> {
	let mut running = initialized.start_loop().unwrap();
	running.wait().unwrap();
	running
		.into_exited_loop()
		.unwrap_or_else(|_| panic!("completed loop did not transition"))
}

#[test]
fn fixed_arenas_one_logical_admission_and_ordered_cleanup() {
	let backend = FakeBackend::new()
		.with_script(INIT_GPU, [FakeStep::Pending, FakeStep::Complete])
		.with_script(INIT_RAM, [FakeStep::Pending, FakeStep::Complete]);
	let initialized = initialized(backend, Watchdog::new(4).unwrap(), 1);
	let journal = initialized.journal();

	assert_eq!(
		journal
			.logical_events()
			.iter()
			.filter(|event| matches!(event, LogicalEvent::ArenaAllocated { .. }))
			.count(),
		2
	);
	assert_eq!(
		journal
			.logical_events()
			.iter()
			.filter(|event| matches!(event, LogicalEvent::InitAdmission { .. }))
			.count(),
		2
	);
	assert_eq!(
		journal
			.physical_calls()
			.iter()
			.filter(|call| matches!(call, PhysicalCall::AdmissionChunk { .. }))
			.count(),
		4
	);
	let gpu_polls = journal
		.physical_calls()
		.iter()
		.filter_map(|call| match call {
			PhysicalCall::Poll { task, status } if *task == INIT_GPU => Some(*status),
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(
		gpu_polls,
		[PhysicalPollStatus::Pending, PhysicalPollStatus::Complete]
	);

	let exited = complete_loop(initialized).exit().unwrap();
	assert_eq!(
		exited.journal()
			.logical_events()
			.iter()
			.filter(|event| matches!(event, LogicalEvent::ArenaReleased { .. }))
			.count(),
		2
	);
	assert!(matches!(
		&exited.journal().physical_calls()[exited.journal().physical_calls().len() - 3..],
		[
			PhysicalCall::ReleaseArena { device: GPU },
			PhysicalCall::ReleaseArena { device: RAM },
			PhysicalCall::DestroyResources
		]
	));
}

#[test]
fn independent_ready_tasks_are_submitted_before_either_is_polled() {
	let backend = FakeBackend::new()
		.with_script(CALCULATION, [FakeStep::Pending, FakeStep::Complete])
		.with_script(INTERNAL_TRANSFER, [FakeStep::Pending, FakeStep::Complete]);
	let initialized = initialized(backend, Watchdog::new(4).unwrap(), 2);
	let mut running = initialized.start_loop().unwrap();
	assert_eq!(running.poll().unwrap(), LoopStatus::Pending);

	let calls = running.journal().physical_calls();
	let calculation = calls
		.iter()
		.position(|call| matches!(call, PhysicalCall::SubmitCalculation { task: CALCULATION }))
		.unwrap();
	let transfer = calls
		.iter()
		.position(|call| {
			matches!(
				call,
				PhysicalCall::SubmitInternalTransfer {
					task: INTERNAL_TRANSFER
				}
			)
		})
		.unwrap();
	let first_loop_poll = calls
		.iter()
		.position(|call| {
			matches!(
				call,
				PhysicalCall::Poll {
					task: CALCULATION | INTERNAL_TRANSFER,
					..
				}
			)
		})
		.unwrap();
	assert!(calculation < first_loop_poll);
	assert!(transfer < first_loop_poll);
	running.wait().unwrap();
}

#[test]
fn injected_asynchronous_failure_is_typed_and_retains_running_state() {
	let backend = FakeBackend::new().with_script(CALCULATION, [FakeStep::Fail("injected async fault")]);
	let initialized = initialized(backend, Watchdog::new(4).unwrap(), 3);
	let mut running = initialized.start_loop().unwrap();
	let error = running.poll().unwrap_err();
	assert_eq!(
		error,
		ExecutorError::Backend {
			operation: BackendOperation::Poll { task: CALCULATION },
			message: BackendMessage::new("injected async fault"),
		}
	);
	assert!(matches!(
		running.journal().physical_calls().last(),
		Some(PhysicalCall::Poll {
			task: CALCULATION,
			status: PhysicalPollStatus::Failed
		})
	));
	assert!(running.into_exited_loop().is_err());
}

#[test]
fn watchdog_detects_bounded_nonprogress() {
	let pending = std::iter::repeat_n(FakeStep::Pending, 8);
	let backend = FakeBackend::new()
		.with_script(CALCULATION, pending.clone())
		.with_script(INTERNAL_TRANSFER, pending);
	let initialized = initialized(backend, Watchdog::new(2).unwrap(), 4);
	let mut running = initialized.start_loop().unwrap();
	assert_eq!(running.poll().unwrap(), LoopStatus::Pending);
	assert_eq!(running.poll().unwrap(), LoopStatus::Pending);
	assert_eq!(
		running.poll().unwrap_err(),
		ExecutorError::WatchdogExpired {
			phase: RunPhase::Loop,
			nonprogress_polls: 2,
		}
	);
}

#[test]
fn running_loop_never_allocates_or_admits_external_data() {
	let initialized = initialized(FakeBackend::new(), Watchdog::new(4).unwrap(), 5);
	let allocations_before = initialized
		.journal()
		.physical_calls()
		.iter()
		.filter(|call| matches!(call, PhysicalCall::AllocateArena { .. }))
		.count();
	let admissions_before = initialized
		.journal()
		.logical_events()
		.iter()
		.filter(|event| matches!(event, LogicalEvent::InitAdmission { .. }))
		.count();
	let exited_loop = complete_loop(initialized);
	assert_eq!(
		exited_loop
			.journal()
			.physical_calls()
			.iter()
			.filter(|call| matches!(call, PhysicalCall::AllocateArena { .. }))
			.count(),
		allocations_before
	);
	assert_eq!(
		exited_loop
			.journal()
			.logical_events()
			.iter()
			.filter(|event| matches!(event, LogicalEvent::InitAdmission { .. }))
			.count(),
		admissions_before
	);
}

#[test]
fn metric_slots_coalesce_to_the_latest_unconsumed_value() {
	let initialized = initialized(FakeBackend::new(), Watchdog::new(4).unwrap(), 6);
	let mut exited_loop = complete_loop(initialized);
	assert_eq!(exited_loop.metric_mailbox().capacity(), 1);
	assert_eq!(exited_loop.metric_mailbox().pending_len(), 1);
	let sample = exited_loop.try_take_metric(METRIC_SLOT).unwrap();
	assert_eq!(sample.task, METRIC_NEW);
	assert_eq!(sample.value, MetricValue::F32(2.0));
	let replacements = exited_loop
		.journal()
		.logical_events()
		.iter()
		.filter_map(|event| match event {
			LogicalEvent::MetricPublished {
				replaced_unconsumed,
				..
			} => Some(*replaced_unconsumed),
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(replacements, [false, true]);
}

#[test]
fn zero_fault_readback_gates_dependents_without_entering_the_user_mailbox() {
	let initialized = fault_initialized(FakeBackend::new(), Watchdog::new(4).unwrap(), 13);
	let mut exited_loop = complete_loop(initialized);
	assert_eq!(exited_loop.metric_mailbox().capacity(), 1);
	assert_eq!(exited_loop.metric_mailbox().pending_len(), 1);
	let sample = exited_loop.try_take_metric(METRIC_SLOT).unwrap();
	assert_eq!(sample.task, METRIC_NEW);
	assert_eq!(sample.value, MetricValue::F32(2.0));

	let journal = exited_loop.journal();
	assert!(journal.logical_events().iter().any(|event| {
		matches!(
			event,
			LogicalEvent::FaultChecked {
				calculation: CALCULATION,
				readback: FAULT_READBACK,
				value,
			} if *value == ValueId::new(4)
		)
	}));
	let readback_complete = journal
		.physical_calls()
		.iter()
		.position(|call| {
			matches!(
				call,
				PhysicalCall::Poll {
					task: FAULT_READBACK,
					status: PhysicalPollStatus::Complete,
				}
			)
		})
		.unwrap();
	let dependent_submit = journal
		.physical_calls()
		.iter()
		.position(|call| {
			matches!(
				call,
				PhysicalCall::SubmitMetric {
					task: METRIC_OLD,
					..
				}
			)
		})
		.unwrap();
	assert!(readback_complete < dependent_submit);
}

#[test]
fn nonzero_fault_readback_fails_closed_before_dependents_submit() {
	let backend = FakeBackend::new().with_metric(FAULT_READBACK, MetricValue::I32(7));
	let initialized = fault_initialized(backend, Watchdog::new(4).unwrap(), 14);
	let mut running = initialized.start_loop().unwrap();
	let error = running.wait().unwrap_err();
	assert_eq!(
		error,
		ExecutorError::DeviceFault {
			calculation: CALCULATION,
			readback: FAULT_READBACK,
			value: ValueId::new(4),
			code: 7,
		}
	);
	assert!(!running.journal().physical_calls().iter().any(|call| {
		matches!(
			call,
			PhysicalCall::SubmitMetric {
				task: METRIC_OLD,
				..
			}
		)
	}));
	assert!(!running.journal().logical_events().iter().any(|event| {
		matches!(
			event,
			LogicalEvent::TaskCompleted {
				task: FAULT_READBACK,
				..
			} | LogicalEvent::FaultChecked {
				readback: FAULT_READBACK,
				..
			}
		)
	}));
	assert!(running.into_exited_loop().is_err());
}

#[test]
fn backend_can_execute_repeated_full_lifecycles() {
	let first = complete_loop(initialized(
		FakeBackend::new(),
		Watchdog::new(4).unwrap(),
		7,
	))
	.exit()
	.unwrap();
	let (backend, _, _, first_journal) = first.into_parts();
	assert!(matches!(
		first_journal.logical_events().last(),
		Some(LogicalEvent::Exited { run }) if *run == RunId::new(7)
	));

	let second = complete_loop(initialized(backend, Watchdog::new(4).unwrap(), 8))
		.exit()
		.unwrap();
	let (backend, _, _, second_journal) = second.into_parts();
	assert_eq!(backend.completed_lifecycles, 2);
	assert_eq!(backend.next_resource, 3);
	assert!(matches!(
		second_journal.logical_events().last(),
		Some(LogicalEvent::Exited { run }) if *run == RunId::new(8)
	));
}

#[test]
fn init_images_are_exact_and_unique() {
	let prepared = PreparedRun::prepare(
		RunId::new(9),
		fixture(),
		FakeBackend::new(),
		Watchdog::new(4).unwrap(),
	)
	.unwrap();
	let error = prepared
		.initialize([
			DeviceImage::new(GPU, ValueId::new(1), vec![0; 16]),
			DeviceImage::new(RAM, ValueId::new(2), vec![0; 15]),
		])
		.unwrap_err();
	assert_eq!(
		error,
		ExecutorError::AdmissionSizeMismatch {
			device: RAM,
			expected: ByteCount::new(16),
			actual: ByteCount::new(15),
		}
	);
}

#[test]
fn init_images_preserve_the_finalized_image_identity() {
	let prepared = PreparedRun::prepare(
		RunId::new(90),
		fixture(),
		FakeBackend::new(),
		Watchdog::new(4).unwrap(),
	)
	.unwrap();
	let mut supplied = images();
	supplied[0].image = ValueId::new(99);
	let error = prepared.initialize(supplied).unwrap_err();
	assert_eq!(
		error,
		ExecutorError::AdmissionImageMismatch {
			device: GPU,
			expected: ValueId::new(1),
			actual: ValueId::new(99),
		}
	);
}

#[test]
fn loop_progress_preserves_every_preallocated_capacity() {
	let backend = FakeBackend::new()
		.with_script(CALCULATION, [FakeStep::Pending, FakeStep::Complete])
		.with_script(INTERNAL_TRANSFER, [FakeStep::Pending, FakeStep::Complete]);
	let initialized = initialized(backend, Watchdog::new(4).unwrap(), 10);
	let mut running = initialized.start_loop().unwrap();
	let capacities = running.capacities();

	assert_eq!(running.poll().unwrap(), LoopStatus::Pending);
	assert_eq!(running.capacities(), capacities);
	assert_eq!(running.poll().unwrap(), LoopStatus::Pending);
	assert_eq!(running.capacities(), capacities);
	assert_eq!(running.poll().unwrap(), LoopStatus::Pending);
	assert_eq!(running.capacities(), capacities);
	assert_eq!(running.poll().unwrap(), LoopStatus::Complete);
	assert_eq!(running.capacities(), capacities);
}

#[test]
fn external_exit_bytes_are_returned_as_typed_images() {
	let exited = complete_loop(initialized(
		FakeBackend::new(),
		Watchdog::new(4).unwrap(),
		11,
	))
	.exit()
	.unwrap();
	assert_eq!(exited.exit_images().len(), 1);
	let image = &exited.exit_images()[0];
	assert_eq!(image.task, EXIT_GPU);
	assert_eq!(image.source.device, GPU);
	assert_eq!(image.source.value, ValueId::new(1));
	assert_eq!(image.bytes, vec![0xA5; 16]);
	assert!(exited.journal().physical_calls().iter().any(|call| {
		matches!(
			call,
			PhysicalCall::CollectExit {
				task: EXIT_GPU,
				bytes
			} if *bytes == ByteCount::new(16)
		)
	}));
}

#[test]
fn declared_journal_exhaustion_fails_closed() {
	let error = PreparedRun::prepare_with_journal_capacity(
		RunId::new(12),
		fixture(),
		FakeBackend::new(),
		Watchdog::new(4).unwrap(),
		JournalCapacity::new(64, 1),
	)
	.unwrap_err();
	assert_eq!(
		error,
		ExecutorError::JournalCapacityExceeded {
			stream: JournalStream::Physical,
			capacity: 1,
		}
	);
}

#[test]
fn physical_call_batches_reject_capacity_overflow() {
	assert_eq!(
		PhysicalCallBatch::try_from_array([PhysicalCall::BindResources; MAX_PHYSICAL_CALLS_PER_OPERATION + 1]),
		Err(PhysicalCallBatchOverflow)
	);
}

#[test]
fn caller_bytes_cannot_preseed_a_fault_flag() {
	let bundle = fault_fixture();
	let location = bundle.value_location(ValueId::new(4)).copied().unwrap();
	let validated =
		crate::executor::validate_images_with_fault_reset(&bundle, fault_images(), CALCULATION, location).unwrap();
	assert_eq!(&validated[&GPU][..16], &[1; 16]);
	assert_eq!(&validated[&GPU][16..], &[0, 0, 0, 0]);
}

#[test]
fn fault_reset_is_relative_to_the_finalized_image_origin() {
	let image = ResolvedValueLocation {
		value: ValueId::new(3),
		dtype: DType::I32,
		device: GPU,
		bytes: ByteCount::new(20),
		object: ArenaObjectId::new(7),
		object_offset: ByteOffset::new(0),
		arena_offset: ByteOffset::new(64),
	};
	let fault = ResolvedValueLocation {
		value: ValueId::new(4),
		dtype: DType::I32,
		device: GPU,
		bytes: ByteCount::new(4),
		object: ArenaObjectId::new(7),
		object_offset: ByteOffset::new(16),
		arena_offset: ByteOffset::new(80),
	};
	let range = crate::executor::fault_reset_range_for_test(CALCULATION, image, fault, ByteCount::new(20)).unwrap();
	assert_eq!(range, 16..20);
}
