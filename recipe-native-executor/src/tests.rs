use std::collections::BTreeSet;
use std::sync::Arc;

use recipe_core::*;
use recipe_cuda::{ComputeCapability, DriverSymbol, DriverVersion};
use recipe_executor::Backend;
use recipe_kernel::{BufferAccess, KernelAbi, KernelArgument};
use recipe_planner::{KernelPlacement, PlannedCandidate};

use crate::accounting::{completion_poll_call, submission_call};
use crate::{CandidateArtifact, Error, ExecutionPlan, RuntimeArtifact, RuntimeArtifactKind, RuntimeImage};

pub(crate) const GPU: DeviceId = DeviceId::new(1);
pub(crate) const INIT: TaskId = TaskId::new(1);
pub(crate) const CALCULATION: TaskId = TaskId::new(2);
pub(crate) const METRIC: TaskId = TaskId::new(3);
const ARTIFACT: ArtifactId = ArtifactId::new(1);
const QUEUE: QueueSlotId = QueueSlotId::new(1);
const COMPLETION: CompletionSlotId = CompletionSlotId::new(1);

fn digest(value: u8) -> Digest {
	Digest::new([value; 32])
}

fn success<T, E: std::fmt::Debug>(result: std::result::Result<T, E>) -> T {
	match result {
		Ok(value) => value,
		Err(error) => panic!("test setup failed: {error:?}"),
	}
}

fn present<T>(value: Option<T>) -> T {
	match value {
		Some(value) => value,
		None => panic!("test setup produced no value"),
	}
}

fn label(value: &str) -> Label {
	success(Label::new(value))
}

fn property<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Override)
}

fn target() -> TargetIdentity {
	TargetIdentity {
		backend: label("nvidia-cuda-driver"),
		architecture: label("sm_52"),
		abi: label("elf64-cubin"),
	}
}

fn kernel_abi() -> KernelAbi {
	KernelAbi {
		entry_symbol: "negate".to_owned(),
		arguments: vec![
			KernelArgument::Buffer {
				access: BufferAccess::Read,
				dtype: DType::F32,
				alignment: 4,
			},
			KernelArgument::Buffer {
				access: BufferAccess::Write,
				dtype: DType::F32,
				alignment: 4,
			},
			KernelArgument::ElementCount,
		],
		argument_bytes: 24,
		argument_alignment: 8,
		elements: success(ElementCount::new(4)),
		workgroup_lanes: 64,
	}
}

fn runtime_artifact(bytes: Arc<[u8]>, abi: KernelAbi) -> RuntimeArtifact {
	let image = RuntimeImage::new(bytes);
	let sha256 = image.digest().bytes();
	RuntimeArtifact::from_image(
		ARTIFACT,
		image,
		abi,
		RuntimeArtifactKind::Cuda {
			identity: recipe_cuda::ArtifactIdentity {
				sha256,
				target: ComputeCapability::new(5, 2),
				toolchain: recipe_cuda::ToolchainIdentity {
					zig_version: "0.17".to_owned(),
					llvm_version: "22".to_owned(),
					ptx_isa_version: "6.4".to_owned(),
					ptxas_version: "11.4".to_owned(),
					cuda_toolkit_version: "11.4".to_owned(),
					cubin_format: "elf64-cubin".to_owned(),
				},
				minimum_driver: success(DriverVersion::new(11, 0)),
				maximum_driver: Some(success(DriverVersion::new(11, 9))),
				required_driver_symbols: BTreeSet::from([DriverSymbol::Init, DriverSymbol::LaunchKernel]),
			},
		},
	)
}

#[derive(Debug)]
pub(crate) struct CandidateFixture {
	pub(crate) topology: Topology,
	pub(crate) discovery: DiscoveryProfile,
	pub(crate) candidate: PlannedCandidate,
	pub(crate) artifacts: Vec<CandidateArtifact>,
	pub(crate) reservations: ReservationLedger,
	pub(crate) capacity: CapacityLedger,
	pub(crate) bundle: FinalizedBundle,
}

pub(crate) fn candidate_fixture(bytes: &[u8]) -> CandidateFixture {
	let machine = MachineId::new(1);
	let topology_id = TopologyIdentity::new(digest(1));
	let topology = Topology {
		identity: topology_id,
		machines: vec![Machine {
			id: machine,
			name: label("machine"),
		}],
		nodes: vec![Node {
			id: NodeId::new(1),
			role: NodeRole::Master,
			machine,
			devices: vec![GPU],
		}],
		devices: vec![Device {
			id: GPU,
			machine,
			kind: DeviceKind::GpuMemory,
			capacity: property(ByteCount::new(12_000_000_000)),
			transfer_rate: property(success(BytesPerSecond::new(432_000_000_000))),
			calculation_rate: Some(property(success(FlopsPerSecond::new(380_000_000_000)))),
		}],
		links: vec![],
	};
	let discovery_id = DiscoveryIdentity::new(digest(2));
	let discovery = DiscoveryProfile {
		identity: discovery_id,
		topology: topology_id,
		devices: vec![DiscoveredDevice {
			device: GPU,
			available: true,
			maximum_submission_queues: 1,
			total_capacity: property(ByteCount::new(12_000_000_000)),
			transfer: TransferCapability {
				rate: property(success(BytesPerSecond::new(432_000_000_000))),
				maximum_inflight_transfers: property(success(TransferLaneCount::new(2))),
				asynchronous_submission: true,
				overlaps_calculation: true,
			},
			calculation: Some(CalculationCapability {
				target: target(),
				rate: property(success(FlopsPerSecond::new(380_000_000_000))),
				asynchronous_submission: true,
				maximum_concurrent_tasks: 1,
				subgroup_lanes: 32,
				maximum_workgroup_lanes: 256,
				maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
			}),
		}],
		links: vec![],
	};

	let scalar_input = ScalarValueId::new(1);
	let scalar_output = ScalarValueId::new(2);
	let kernel_id = KernelTemplateId::new(1);
	let kernel = KernelTemplate {
		id: kernel_id,
		index_space: success(IndexSpace::new(vec![success(ElementCount::new(4))])),
		inputs: vec![KernelInput {
			id: KernelInputId::new(1),
			dtype: DType::F32,
			access: success(StaticBufferAccess::linear(
				success(ElementCount::new(4)),
				DType::F32,
			)),
		}],
		outputs: vec![KernelOutput {
			id: KernelOutputId::new(1),
			dtype: DType::F32,
			access: success(StaticBufferAccess::linear(
				success(ElementCount::new(4)),
				DType::F32,
			)),
		}],
		program: ScalarProgram {
			inputs: vec![ScalarInput {
				id: scalar_input,
				dtype: DType::F32,
			}],
			constants: vec![],
			instructions: vec![ScalarInstruction {
				result: scalar_output,
				dtype: DType::F32,
				opcode: ScalarOpcode::Negate,
				operands: vec![scalar_input],
			}],
			outputs: vec![scalar_output],
		},
		alias_rules: vec![AliasRule {
			input: KernelInputId::new(1),
			output: KernelOutputId::new(1),
			permission: AliasPermission::MustAliasExact,
		}],
	};
	let artifact = ArtifactIdentity {
		id: ARTIFACT,
		digest: Digest::new(recipe_kernel::ArtifactDigest::of(bytes).bytes()),
		format: label("elf64-cubin"),
		target: target(),
		toolchain: ToolchainIdentity {
			name: label("recipe-owned"),
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
	let metric_id = MetricId::new(1);
	let metric_slot = MetricSlotId::new(1);
	let resources = ResourceManifest {
		queues: vec![QueueSlot {
			id: QUEUE,
			device: GPU,
		}],
		completions: vec![CompletionSlot {
			id: COMPLETION,
			device: GPU,
		}],
		metrics: vec![MetricSlot {
			id: metric_slot,
			metric: metric_id,
		}],
		pinned_staging: vec![DeviceBytes {
			device: GPU,
			bytes: ByteCount::new(16),
		}],
		scratch: vec![DeviceBytes {
			device: GPU,
			bytes: ByteCount::new(4),
		}],
	};
	let value = ValueId::new(1);
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
		values: vec![ValueSpec {
			id: value,
			dtype: DType::F32,
			bytes: ByteCount::new(16),
			device: GPU,
			producer: Some(INIT),
		}],
		kernels: vec![kernel],
		artifacts: vec![artifact.clone()],
		artifact_builds: Vec::new(),
		tasks: vec![
			Task {
				id: INIT,
				phase: RunPhase::Init,
				window: window(0, 1),
				dependencies: vec![],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::External,
					destination: TransferEndpoint::Device { device: GPU, value },
					bytes: ByteCount::new(16),
					route: vec![],
					lane_claims: vec![TransferLaneClaim::External {
						device: GPU,
						lane: 0,
					}],
					submission: SubmissionSlots {
						queue: QUEUE,
						completion: COMPLETION,
					},
				}),
			},
			Task {
				id: CALCULATION,
				phase: RunPhase::Loop,
				window: window(1, 2),
				dependencies: vec![INIT],
				kind: TaskKind::Calculation(CalculationTask {
					device: GPU,
					kernel_template: kernel_id,
					artifact: ARTIFACT,
					inputs: vec![value],
					outputs: vec![value],
					fault_flag: None,
					work: FlopCount::new(4),
					submission: SubmissionSlots {
						queue: QUEUE,
						completion: COMPLETION,
					},
				}),
			},
			Task {
				id: METRIC,
				phase: RunPhase::Loop,
				window: window(2, 3),
				dependencies: vec![CALCULATION],
				kind: TaskKind::Metric(MetricTask {
					purpose: MetricPurpose::User,
					metric: metric_id,
					value,
					slot: metric_slot,
					submission: SubmissionSlots {
						queue: QUEUE,
						completion: COMPLETION,
					},
				}),
			},
		],
		resources: resources.clone(),
		arena_objects: vec![ArenaObject {
			id: ArenaObjectId::new(1),
			device: GPU,
			bytes: ByteCount::new(16),
			alignment: ByteCount::new(16),
			lifetime: window(0, 4),
		}],
		value_bindings: vec![ValueBinding {
			value,
			object: ArenaObjectId::new(1),
			object_offset: ByteOffset::new(0),
		}],
		value_aliases: Vec::new(),
		init_images: vec![InitDataImage {
			device: GPU,
			image: value,
			bytes: ByteCount::new(16),
			members: vec![InitDataImageMember {
				logical: value,
				physical: value,
				dtype: DType::F32,
				bytes: ByteCount::new(16),
				image_offset: ByteOffset::new(0),
			}],
		}],
		releases: vec![ArenaRelease { device: GPU }],
	};
	let reservations = ReservationLedger {
		entries: vec![ReservationEntry {
			device: GPU,
			name: label("user"),
			bytes: EXACT_USER_RESERVATION,
			mechanism: ReservationMechanism::EnforcedQuota,
			evidence: ReservationEvidence::GpuDisplay {
				enabled_connectors: 1,
			},
		}],
	};
	let capacity = CapacityLedger {
		entries: vec![CapacityLedgerEntry {
			device: GPU,
			total: property(ByteCount::new(12_000_000_000)),
			runtime_overhead: property(ByteCount::ZERO),
			fragmentation: property(ByteCount::ZERO),
			safety_headroom: property(ByteCount::ZERO),
			recipe_usable: property(ByteCount::new(11_000_000_000)),
		}],
	};
	let realization = RealizationProfile {
		identity: RealizationIdentity::new(digest(7)),
		draft: draft_id,
		candidate,
		discovery: discovery_id,
		topology: topology_id,
		artifacts: vec![artifact],
		resources,
		reservations: reservations.clone(),
		capacity: capacity.clone(),
	};
	let layouts = vec![ArenaLayout {
		device: GPU,
		size: ByteCount::new(16),
		allocations: vec![ArenaAllocation {
			object: ArenaObjectId::new(1),
			offset: ByteOffset::new(0),
		}],
	}];
	let planned = PlannedCandidate {
		draft: draft.clone(),
		arena_layouts: layouts.clone(),
		makespan: Nanoseconds::new(3),
		placements: vec![KernelPlacement {
			kernel: kernel_id,
			device: GPU,
		}],
		stage_placements: Vec::new(),
		lowered_programs: Vec::new(),
		value_copies: Vec::new(),
		external_outputs: Vec::new(),
	};
	let runtime_bytes: Arc<[u8]> = Arc::from(bytes);
	let artifacts = vec![CandidateArtifact::new(
		realization.artifacts[0].clone(),
		runtime_artifact(runtime_bytes, kernel_abi()),
	)];
	let bundle = success(FinalizedBundle::finalize(
		BundleIdentity::new(digest(8)),
		&topology,
		&discovery,
		draft,
		realization,
		layouts,
	));
	CandidateFixture {
		topology,
		discovery,
		candidate: planned,
		artifacts,
		reservations,
		capacity,
		bundle,
	}
}

pub(crate) fn fixture(bytes: &[u8]) -> FinalizedBundle {
	candidate_fixture(bytes).bundle
}

fn deferred_build_fixture(bytes: &[u8]) -> FinalizedBundle {
	let fixture = candidate_fixture(bytes);
	let mut draft = fixture.candidate.draft.clone();
	let kernel_template = match &draft.tasks[1].kind {
		TaskKind::Calculation(calculation) => calculation.kernel_template,
		TaskKind::Transfer(_) | TaskKind::Metric(_) => panic!("fixture task 1 is not a calculation"),
	};
	let build = ArtifactBuildRecipe {
		artifact: ARTIFACT,
		kernel_template,
		source_kernel: kernel_template,
		provenance: ArtifactBuildProvenance {
			program_digest: digest(9),
			stage_ordinal: 0,
			contract_digest: digest(10),
		},
		bindings: vec![ArtifactBuildBinding {
			value: ValueId::new(1),
			dtype: DType::F32,
			access: ArtifactBuildAccess::ReadWrite,
			view: ArtifactBuildView {
				logical_extents: vec![4],
				offset_elements: 0,
				strides: vec![1],
				storage_bytes: ByteCount::new(16),
			},
		}],
		dispatch: ArtifactDispatchGeometry {
			logical_lanes: 4,
			workgroup_lanes: 64,
			workgroups: 1,
		},
		work: ArtifactWorkBounds {
			flops: FlopCount::new(4),
			integer_operations: 0,
			atomic_operations: 0,
		},
		fault_flag: None,
		resources: KernelResourceBounds {
			private_bytes_per_lane: ByteCount::ZERO,
			shared_bytes_per_workgroup: ByteCount::ZERO,
			scratch_bytes_per_dispatch: ByteCount::ZERO,
			maximum_workgroup_lanes: 64,
		},
	};
	draft.kernels.clear();
	draft.artifacts.clear();
	draft.artifact_builds = vec![build.clone()];

	let mut artifact = fixture.bundle.artifacts()[0].clone();
	artifact.build = Some(build.provenance);
	artifact.resources = build.resources;
	let realization = RealizationProfile {
		identity: RealizationIdentity::new(digest(11)),
		draft: draft.identity,
		candidate: draft.candidate,
		discovery: draft.discovery,
		topology: draft.topology,
		artifacts: vec![artifact],
		resources: draft.resources.clone(),
		reservations: fixture.reservations,
		capacity: fixture.capacity,
	};
	success(FinalizedBundle::finalize(
		BundleIdentity::new(digest(12)),
		&fixture.topology,
		&fixture.discovery,
		draft,
		realization,
		fixture.candidate.arena_layouts,
	))
}

#[test]
fn native_plan_uses_deferred_build_contract_without_a_scalar_template() {
	let bytes: Arc<[u8]> = Arc::from(&b"runtime-image"[..]);
	let bundle = deferred_build_fixture(&bytes);
	assert!(bundle.kernels().is_empty());
	assert!(bundle.artifact_build(ARTIFACT).is_some());
	success(ExecutionPlan::validate(
		&bundle,
		vec![runtime_artifact(Arc::clone(&bytes), kernel_abi())],
	));

	let mut wrong_access = kernel_abi();
	wrong_access.arguments[0] = KernelArgument::Buffer {
		access: BufferAccess::Write,
		dtype: DType::F32,
		alignment: 4,
	};
	let error = match ExecutionPlan::validate(&bundle, vec![runtime_artifact(bytes, wrong_access)]) {
		Ok(plan) => panic!("wrong deferred-stage ABI produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(error, Error::ArtifactMismatch { .. }));
}

#[test]
fn immutable_plan_uses_the_metric_submission_slots_persisted_by_the_planner() {
	let bytes: Arc<[u8]> = Arc::from(&b"runtime-image"[..]);
	let bundle = fixture(&bytes);
	let plan = success(ExecutionPlan::validate(
		&bundle,
		vec![runtime_artifact(Arc::clone(&bytes), kernel_abi())],
	));
	let metric = present(plan.submission(METRIC));
	assert_eq!(metric.device, GPU);
	assert_eq!(metric.slots.queue, QUEUE);
	assert_eq!(metric.slots.completion, COMPLETION);
}

#[test]
fn digest_and_fault_flag_mismatches_fail_closed_before_native_realization() {
	let bytes: Arc<[u8]> = Arc::from(&b"runtime-image"[..]);
	let bundle = fixture(&bytes);
	let wrong: Arc<[u8]> = Arc::from(&b"altered-image"[..]);
	let digest_error = match ExecutionPlan::validate(&bundle, vec![runtime_artifact(wrong, kernel_abi())]) {
		Ok(plan) => panic!("wrong digest produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(digest_error, Error::ArtifactMismatch { .. }));

	let mut abi = kernel_abi();
	abi.arguments.insert(2, KernelArgument::FaultFlag);
	abi.argument_bytes = 32;
	let fault_error = match ExecutionPlan::validate(&bundle, vec![runtime_artifact(Arc::clone(&bytes), abi)]) {
		Ok(plan) => panic!("fault flag produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(fault_error, Error::ArtifactMismatch { .. }));
}

#[test]
fn dynamic_run_id_has_one_canonical_native_abi_position() {
	let bytes: Arc<[u8]> = Arc::from(&b"runtime-image"[..]);
	let bundle = fixture(&bytes);

	let mut dynamic = kernel_abi();
	dynamic.arguments.insert(2, KernelArgument::RunId);
	dynamic.argument_bytes = 32;
	success(ExecutionPlan::validate(
		&bundle,
		vec![runtime_artifact(Arc::clone(&bytes), dynamic.clone())],
	));

	let mut misplaced = dynamic.clone();
	misplaced.arguments.swap(0, 2);
	let misplaced_error = match ExecutionPlan::validate(
		&bundle,
		vec![runtime_artifact(Arc::clone(&bytes), misplaced)],
	) {
		Ok(plan) => panic!("misplaced run id produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(misplaced_error, Error::ArtifactMismatch { .. }));

	let mut repeated = dynamic;
	repeated.arguments.insert(3, KernelArgument::RunId);
	repeated.argument_bytes = 40;
	let repeated_error = match ExecutionPlan::validate(&bundle, vec![runtime_artifact(bytes, repeated)]) {
		Ok(plan) => panic!("repeated run id produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(repeated_error, Error::ArtifactMismatch { .. }));
}

#[test]
fn loop_iteration_has_one_canonical_native_abi_position() {
	let bytes: Arc<[u8]> = Arc::from(&b"runtime-image"[..]);
	let bundle = fixture(&bytes);

	let mut dynamic = kernel_abi();
	dynamic.arguments.insert(2, KernelArgument::LoopIteration);
	dynamic.argument_bytes = 32;
	success(ExecutionPlan::validate(
		&bundle,
		vec![runtime_artifact(Arc::clone(&bytes), dynamic.clone())],
	));

	let mut misplaced = dynamic.clone();
	misplaced.arguments.swap(0, 2);
	let misplaced_error = match ExecutionPlan::validate(
		&bundle,
		vec![runtime_artifact(Arc::clone(&bytes), misplaced)],
	) {
		Ok(plan) => panic!("misplaced loop iteration produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(misplaced_error, Error::ArtifactMismatch { .. }));

	let mut repeated = dynamic;
	repeated.arguments.insert(3, KernelArgument::LoopIteration);
	repeated.argument_bytes = 40;
	let repeated_error = match ExecutionPlan::validate(&bundle, vec![runtime_artifact(bytes, repeated)]) {
		Ok(plan) => panic!("repeated loop iteration produced a plan: {plan:?}"),
		Err(error) => error,
	};
	assert!(matches!(repeated_error, Error::ArtifactMismatch { .. }));
}

#[test]
fn accounting_keeps_one_submit_and_one_poll_per_logical_action() {
	let work = recipe_executor::BackendWork::Metric(recipe_executor::MetricWork {
		task: METRIC,
		iteration: LoopIterations::ONE
			.iteration(0)
			.expect("one loop iteration has index zero"),
		metric: MetricId::new(1),
		slot: MetricSlotId::new(1),
		purpose: MetricPurpose::User,
		value: ResolvedValueLocation {
			value: ValueId::new(1),
			dtype: DType::F32,
			device: GPU,
			bytes: ByteCount::new(4),
			object: ArenaObjectId::new(1),
			object_offset: ByteOffset::new(0),
			arena_offset: ByteOffset::new(0),
		},
		submission: SubmissionSlots {
			queue: QUEUE,
			completion: COMPLETION,
		},
	});
	assert!(matches!(
		submission_call(&work),
		recipe_executor::PhysicalCall::SubmitMetric { task: METRIC, .. }
	));
	assert_eq!(
		completion_poll_call(METRIC, recipe_executor::PhysicalPollStatus::Complete,),
		recipe_executor::PhysicalCall::Poll {
			task: METRIC,
			status: recipe_executor::PhysicalPollStatus::Complete,
		}
	);
}

#[test]
fn native_adapters_implement_the_closed_executor_backend() {
	fn backend_size<B: Backend>() -> usize {
		core::mem::size_of::<B>()
	}

	assert_ne!(backend_size::<crate::cuda::CudaBackend<'static>>(), 0);
	assert_ne!(backend_size::<crate::hsa::HsaBackend<'static>>(), 0);
}
