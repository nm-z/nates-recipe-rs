use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{
	AliasPermission, ArtifactIdentity, ByteCount, BytesPerSecond, CalculationCapability, CapacityLedger,
	CapacityLedgerEntry, DType, Device, DeviceId, DeviceKind, Digest, DirectedLink, DiscoveredDevice, DiscoveredLink,
	DiscoveryIdentity, DiscoveryProfile, DuplexMode, DuplexResourceId, EXACT_USER_RESERVATION, FlopsPerSecond,
	KernelResourceBounds, KernelTemplateId, Label, LinkId, Machine, MachineId, Node, NodeId, NodeRole, Property,
	PropertyProvenance, ReservationEntry, ReservationLedger, ReservationMechanism, ScalarInput, ScalarInstruction,
	ScalarOpcode, ScalarProgram, ScalarValueId, TargetIdentity, ToolchainIdentity, Topology, TopologyIdentity,
	TransferCapability, TransferLaneClaim, TransferLaneCount, TransportId, TransportKind, ValueId,
};
use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather,
	Histogram, IndexBounds, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey,
	RandomMap, Reduce, ReduceOperator, ReduceResult, Scan, ScanMode, Scatter, ScatterConflict, Shape, Sort,
	SortDirection, Tensor,
};
use recipe_planner::{PlannerErrorKind, plan_candidates};

fn label(value: &str) -> Label {
	Label::new(value).unwrap()
}

fn measured<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Measured)
}

fn target(name: &str) -> TargetIdentity {
	TargetIdentity {
		backend: label(name),
		architecture: label(&format!("{name}-arch")),
		abi: label("abi-1"),
	}
}

#[derive(Debug)]
struct Fixture {
	topology: Topology,
	discovery: DiscoveryProfile,
	reservations: ReservationLedger,
	capacity: CapacityLedger,
	target_a: TargetIdentity,
	target_b: TargetIdentity,
}

fn fixture() -> Fixture {
	let machine_a = MachineId::new(1);
	let machine_b = MachineId::new(2);
	let gpu_a = DeviceId::new(1);
	let ram_a = DeviceId::new(2);
	let gpu_b = DeviceId::new(3);
	let ram_b = DeviceId::new(4);
	let topology_id = TopologyIdentity::new(Digest::new([1; 32]));
	let target_a = target("native-a");
	let target_b = target("native-b");
	let device = |id: DeviceId, machine: MachineId, kind: DeviceKind, calculation: bool| Device {
		id,
		machine,
		kind,
		capacity: measured(ByteCount::new(10_000_000_000)),
		transfer_rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
		calculation_rate: calculation.then(|| measured(FlopsPerSecond::new(1_000_000_000).unwrap())),
	};
	let link = |id, transport, kind, from, to, resource| DirectedLink {
		id: LinkId::new(id),
		transport: TransportId::new(transport),
		kind,
		duplex: DuplexMode::Full,
		from,
		to,
		bandwidth: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
		maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
		capacity_resource: DuplexResourceId::new(resource),
	};
	let links = vec![
		link(1, 1, TransportKind::Pcie, ram_a, gpu_a, 1),
		link(2, 1, TransportKind::Pcie, gpu_a, ram_a, 2),
		link(3, 2, TransportKind::Pcie, ram_b, gpu_b, 3),
		link(4, 2, TransportKind::Pcie, gpu_b, ram_b, 4),
		link(5, 3, TransportKind::Ethernet, ram_a, ram_b, 5),
		link(6, 3, TransportKind::Ethernet, ram_b, ram_a, 6),
	];
	let topology = Topology {
		identity: topology_id,
		machines: vec![
			Machine {
				id: machine_a,
				name: label("machine-a"),
			},
			Machine {
				id: machine_b,
				name: label("machine-b"),
			},
		],
		nodes: vec![
			Node {
				id: NodeId::new(1),
				role: NodeRole::Master,
				machine: machine_a,
				devices: vec![gpu_a, ram_a],
			},
			Node {
				id: NodeId::new(2),
				role: NodeRole::Worker,
				machine: machine_b,
				devices: vec![gpu_b, ram_b],
			},
		],
		devices: vec![
			device(gpu_a, machine_a, DeviceKind::GpuMemory, true),
			device(ram_a, machine_a, DeviceKind::Ram, false),
			device(gpu_b, machine_b, DeviceKind::GpuMemory, true),
			device(ram_b, machine_b, DeviceKind::Ram, false),
		],
		links,
	};
	let discovered_device = |id, calculation: Option<(TargetIdentity, u32)>| DiscoveredDevice {
		device: id,
		available: true,
		total_capacity: measured(ByteCount::new(10_000_000_000)),
		transfer: TransferCapability {
			rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
			maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
			asynchronous_submission: true,
			overlaps_calculation: true,
		},
		calculation: calculation.map(|(target, lanes)| CalculationCapability {
			target,
			rate: measured(FlopsPerSecond::new(1_000_000_000).unwrap()),
			asynchronous_submission: true,
			maximum_concurrent_tasks: lanes,
		}),
	};
	let discovery = DiscoveryProfile {
		identity: DiscoveryIdentity::new(Digest::new([2; 32])),
		topology: topology_id,
		devices: vec![
			discovered_device(gpu_a, Some((target_a.clone(), 2))),
			discovered_device(ram_a, None),
			discovered_device(gpu_b, Some((target_b.clone(), 2))),
			discovered_device(ram_b, None),
		],
		links: topology
			.links
			.iter()
			.map(|link| DiscoveredLink {
				link: link.id,
				available: true,
				bandwidth: link.bandwidth,
				maximum_inflight_transfers: link.maximum_inflight_transfers,
				asynchronous_submission: true,
			})
			.collect(),
	};
	let reservations = ReservationLedger {
		entries: topology
			.devices
			.iter()
			.map(|device| ReservationEntry {
				device: device.id,
				name: label(&format!("user-{}", device.id.get())),
				bytes: EXACT_USER_RESERVATION,
				mechanism: ReservationMechanism::HeldAllocation,
			})
			.collect(),
	};
	let zero = measured(ByteCount::ZERO);
	let capacity = CapacityLedger {
		entries: topology
			.devices
			.iter()
			.map(|device| CapacityLedgerEntry {
				device: device.id,
				total: measured(ByteCount::new(10_000_000_000)),
				runtime_overhead: zero,
				fragmentation: zero,
				safety_headroom: zero,
				recipe_usable: measured(ByteCount::new(8_000_000_000)),
			})
			.collect(),
	};
	Fixture {
		topology,
		discovery,
		reservations,
		capacity,
		target_a,
		target_b,
	}
}

fn three_gpu_fixture() -> (Fixture, TargetIdentity) {
	let mut fixture = fixture();
	let gpu_a = DeviceId::new(1);
	let gpu_b = DeviceId::new(3);
	let gpu_c = DeviceId::new(5);
	let machine_c = MachineId::new(2);
	let target_c = target("native-c");
	fixture.topology.nodes[1].devices.push(gpu_c);
	fixture.topology.devices.push(Device {
		id: gpu_c,
		machine: machine_c,
		kind: DeviceKind::GpuMemory,
		capacity: measured(ByteCount::new(10_000_000_000)),
		transfer_rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
		calculation_rate: Some(measured(FlopsPerSecond::new(1_000_000_000).unwrap())),
	});
	let link = |id, transport, kind, from, to, bandwidth, resource| DirectedLink {
		id: LinkId::new(id),
		transport: TransportId::new(transport),
		kind,
		duplex: DuplexMode::Full,
		from,
		to,
		bandwidth: measured(BytesPerSecond::new(bandwidth).unwrap()),
		maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
		capacity_resource: DuplexResourceId::new(resource),
	};
	fixture.topology.links = vec![
		link(1, 1, TransportKind::Ethernet, gpu_a, gpu_b, 1, 1),
		link(2, 1, TransportKind::Ethernet, gpu_b, gpu_a, 1, 2),
		link(3, 2, TransportKind::Pcie, gpu_b, gpu_c, 100, 3),
		link(4, 2, TransportKind::Pcie, gpu_c, gpu_b, 1, 4),
		link(5, 3, TransportKind::Ethernet, gpu_a, gpu_c, 10, 5),
		link(6, 3, TransportKind::Ethernet, gpu_c, gpu_a, 10, 6),
	];
	fixture.discovery.devices.push(DiscoveredDevice {
		device: gpu_c,
		available: true,
		total_capacity: measured(ByteCount::new(10_000_000_000)),
		transfer: TransferCapability {
			rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
			maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
			asynchronous_submission: true,
			overlaps_calculation: true,
		},
		calculation: Some(CalculationCapability {
			target: target_c.clone(),
			rate: measured(FlopsPerSecond::new(1_000_000_000).unwrap()),
			asynchronous_submission: true,
			maximum_concurrent_tasks: 2,
		}),
	});
	fixture.discovery.links = fixture
		.topology
		.links
		.iter()
		.map(|link| DiscoveredLink {
			link: link.id,
			available: true,
			bandwidth: link.bandwidth,
			maximum_inflight_transfers: link.maximum_inflight_transfers,
			asynchronous_submission: true,
		})
		.collect();
	fixture.reservations.entries.push(ReservationEntry {
		device: gpu_c,
		name: label("user-5"),
		bytes: EXACT_USER_RESERVATION,
		mechanism: ReservationMechanism::HeldAllocation,
	});
	fixture.capacity.entries.push(CapacityLedgerEntry {
		device: gpu_c,
		total: measured(ByteCount::new(10_000_000_000)),
		runtime_overhead: measured(ByteCount::ZERO),
		fragmentation: measured(ByteCount::ZERO),
		safety_headroom: measured(ByteCount::ZERO),
		recipe_usable: measured(ByteCount::new(8_000_000_000)),
	});
	(fixture, target_c)
}

fn tensor(id: u64, external_input: bool, external_output: bool) -> Tensor {
	typed_tensor(id, DType::F32, &[16], external_input, external_output)
}

fn typed_tensor(id: u64, dtype: DType, shape: &[u64], external_input: bool, external_output: bool) -> Tensor {
	Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(shape.to_vec()).unwrap(),
		external_input,
		external_output,
	)
	.unwrap()
}

fn primitive_graph(
	id: u64,
	tensors: Vec<Tensor>,
	inputs: &[u64],
	outputs: &[u64],
	kind: PrimitiveKind,
) -> CalculationGraph {
	let alias_rules = inputs
		.iter()
		.enumerate()
		.flat_map(|(input, _)| {
			outputs
				.iter()
				.enumerate()
				.map(move |(output, _)| PrimitiveAliasRule {
					input,
					output,
					permission: AliasPermission::Forbidden,
				})
		})
		.collect();
	CalculationGraph {
		tensors,
		nodes: vec![CalculationNode {
			kernel: PrimitiveKernel {
				id: KernelTemplateId::new(id),
				inputs: inputs.iter().copied().map(ValueId::new).collect(),
				outputs: outputs.iter().copied().map(ValueId::new).collect(),
				alias_rules,
				kind,
			},
		}],
	}
}

fn unary(id: u64, input: u64, output: u64) -> CalculationNode {
	let scalar_input = ScalarValueId::new(1);
	let scalar_output = ScalarValueId::new(2);
	CalculationNode {
		kernel: PrimitiveKernel {
			id: KernelTemplateId::new(id),
			inputs: vec![ValueId::new(input)],
			outputs: vec![ValueId::new(output)],
			alias_rules: vec![PrimitiveAliasRule {
				input: 0,
				output: 0,
				permission: AliasPermission::Forbidden,
			}],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: ScalarProgram {
					inputs: vec![ScalarInput {
						id: scalar_input,
						dtype: recipe_core::DType::F32,
					}],
					constants: Vec::new(),
					instructions: vec![ScalarInstruction {
						result: scalar_output,
						dtype: recipe_core::DType::F32,
						opcode: ScalarOpcode::Negate,
						operands: vec![scalar_input],
					}],
					outputs: vec![scalar_output],
				},
			}),
		},
	}
}

fn binary(id: u64, left: u64, right: u64, output: u64) -> CalculationNode {
	let scalar_left = ScalarValueId::new(1);
	let scalar_right = ScalarValueId::new(2);
	let scalar_output = ScalarValueId::new(3);
	CalculationNode {
		kernel: PrimitiveKernel {
			id: KernelTemplateId::new(id),
			inputs: vec![ValueId::new(left), ValueId::new(right)],
			outputs: vec![ValueId::new(output)],
			alias_rules: vec![
				PrimitiveAliasRule {
					input: 0,
					output: 0,
					permission: AliasPermission::Forbidden,
				},
				PrimitiveAliasRule {
					input: 1,
					output: 0,
					permission: AliasPermission::Forbidden,
				},
			],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: ScalarProgram {
					inputs: vec![
						ScalarInput {
							id: scalar_left,
							dtype: recipe_core::DType::F32,
						},
						ScalarInput {
							id: scalar_right,
							dtype: recipe_core::DType::F32,
						},
					],
					constants: Vec::new(),
					instructions: vec![ScalarInstruction {
						result: scalar_output,
						dtype: recipe_core::DType::F32,
						opcode: ScalarOpcode::Add,
						operands: vec![scalar_left, scalar_right],
					}],
					outputs: vec![scalar_output],
				},
			}),
		},
	}
}

fn chain_graph() -> CalculationGraph {
	CalculationGraph {
		tensors: vec![
			tensor(1, true, false),
			tensor(2, false, false),
			tensor(3, false, true),
		],
		nodes: vec![unary(1, 1, 2), unary(2, 2, 3)],
	}
}

fn one_node_graph() -> CalculationGraph {
	CalculationGraph {
		tensors: vec![tensor(1, true, false), tensor(2, false, true)],
		nodes: vec![unary(1, 1, 2)],
	}
}

fn faulting_graph() -> CalculationGraph {
	let mut graph = one_node_graph();
	let PrimitiveKind::Elementwise(elementwise) = &mut graph.nodes[0].kernel.kind else {
		panic!("fixture kernel must be elementwise");
	};
	let finite = ScalarValueId::new(3);
	let required = ScalarValueId::new(4);
	elementwise.program.instructions.extend([
		ScalarInstruction {
			result: finite,
			dtype: recipe_core::DType::I32,
			opcode: ScalarOpcode::IsFinite,
			operands: vec![ScalarValueId::new(1)],
		},
		ScalarInstruction {
			result: required,
			dtype: recipe_core::DType::I32,
			opcode: ScalarOpcode::Require,
			operands: vec![finite],
		},
	]);
	graph
}

fn artifact(id: u64, kernel: u64, target: TargetIdentity) -> ArtifactIdentity {
	ArtifactIdentity {
		id: recipe_core::ArtifactId::new(id),
		digest: Digest::new([u8::try_from(id).unwrap(); 32]),
		format: label("recipe-object"),
		target,
		toolchain: ToolchainIdentity {
			name: label("recipe-toolchain"),
			version: label("1"),
			digest: Digest::new([99; 32]),
		},
		entry_symbol: label(&format!("kernel_{kernel}")),
		kernel_template: KernelTemplateId::new(kernel),
		resources: KernelResourceBounds {
			private_bytes_per_lane: ByteCount::ZERO,
			shared_bytes_per_workgroup: ByteCount::ZERO,
			scratch_bytes_per_dispatch: ByteCount::new(64),
			maximum_workgroup_lanes: 256,
		},
		build: None,
	}
}

#[test]
fn inserts_cross_machine_route_and_complete_lifecycle_contract() {
	let mut fixture = fixture();
	let direct = |id, from, to, resource| DirectedLink {
		id: LinkId::new(id),
		transport: TransportId::new(4),
		kind: TransportKind::Ethernet,
		duplex: DuplexMode::Full,
		from,
		to,
		bandwidth: measured(BytesPerSecond::new(1).unwrap()),
		maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
		capacity_resource: DuplexResourceId::new(resource),
	};
	fixture.topology.links.extend([
		direct(7, DeviceId::new(1), DeviceId::new(3), 7),
		direct(8, DeviceId::new(3), DeviceId::new(1), 8),
	]);
	fixture.discovery.links.extend(
		fixture.topology.links[6..]
			.iter()
			.map(|link| DiscoveredLink {
				link: link.id,
				available: true,
				bandwidth: link.bandwidth,
				maximum_inflight_transfers: link.maximum_inflight_transfers,
				asynchronous_submission: true,
			}),
	);
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 2, fixture.target_b.clone()),
	];
	let search = plan_candidates(
		&chain_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let planned = search
		.ranked_candidates()
		.iter()
		.find(|candidate| {
			candidate
				.placements
				.iter()
				.map(|placement| (placement.kernel, placement.device))
				.collect::<BTreeMap<_, _>>()
				== BTreeMap::from([
					(KernelTemplateId::new(1), DeviceId::new(1)),
					(KernelTemplateId::new(2), DeviceId::new(3)),
				])
		})
		.unwrap();
	planned
		.draft
		.validate(&fixture.topology, &fixture.discovery)
		.unwrap();
	assert_eq!(planned.draft.releases.len(), fixture.topology.devices.len());
	let init_counts = planned
		.draft
		.tasks
		.iter()
		.filter(|task| task.phase == recipe_core::RunPhase::Init)
		.count();
	assert_eq!(init_counts, fixture.topology.devices.len());
	let init_end = planned
		.draft
		.tasks
		.iter()
		.filter(|task| task.phase == recipe_core::RunPhase::Init)
		.map(|task| task.window.end)
		.max()
		.unwrap();
	let loop_start = planned
		.draft
		.tasks
		.iter()
		.filter(|task| task.phase == recipe_core::RunPhase::Loop)
		.map(|task| task.window.start)
		.min()
		.unwrap();
	let loop_end = planned
		.draft
		.tasks
		.iter()
		.filter(|task| task.phase == recipe_core::RunPhase::Loop)
		.map(|task| task.window.end)
		.max()
		.unwrap();
	let exit_start = planned
		.draft
		.tasks
		.iter()
		.filter(|task| task.phase == recipe_core::RunPhase::Exit)
		.map(|task| task.window.start)
		.min()
		.unwrap();
	assert!(init_end <= loop_start);
	assert!(loop_end <= exit_start);
	let transfers = planned
		.draft
		.tasks
		.iter()
		.filter_map(|task| match &task.kind {
			recipe_core::TaskKind::Transfer(transfer) if task.phase == recipe_core::RunPhase::Loop => {
				Some((task, transfer))
			}
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(transfers.len(), 3);
	let expected_hops = [
		(DeviceId::new(1), DeviceId::new(2), LinkId::new(2)),
		(DeviceId::new(2), DeviceId::new(4), LinkId::new(5)),
		(DeviceId::new(4), DeviceId::new(3), LinkId::new(3)),
	];
	for (index, ((task, transfer), (source, destination, link))) in transfers.iter().zip(expected_hops).enumerate() {
		assert_eq!(transfer.route, vec![link]);
		assert_eq!(
			transfer.lane_claims,
			vec![TransferLaneClaim::Link { link, lane: 0 }]
		);
		let (
			recipe_core::TransferEndpoint::Device {
				device: actual_source,
				value: source_value,
			},
			recipe_core::TransferEndpoint::Device {
				device: actual_destination,
				value: destination_value,
			},
		) = (transfer.source, transfer.destination)
		else {
			panic!("loop transfer must have two resident endpoints");
		};
		assert_eq!(actual_source, source);
		assert_eq!(actual_destination, destination);
		let destination_spec = planned
			.draft
			.values
			.iter()
			.find(|value| value.id == destination_value)
			.unwrap();
		assert_eq!(destination_spec.device, destination);
		assert_eq!(destination_spec.dtype, DType::F32);
		assert_eq!(destination_spec.bytes, ByteCount::new(64));
		assert_eq!(destination_spec.producer, Some(task.id));
		assert!(
			planned
				.draft
				.value_bindings
				.iter()
				.any(|binding| binding.value == destination_value)
		);
		let queue = planned
			.draft
			.resources
			.queues
			.iter()
			.find(|queue| queue.id == transfer.submission.queue)
			.unwrap();
		assert_eq!(queue.device, source);
		if index != 0 {
			let (previous_task, previous_transfer) = transfers[index - 1];
			let recipe_core::TransferEndpoint::Device {
				value: previous_destination,
				..
			} = previous_transfer.destination
			else {
				panic!("preceding hop must have a resident destination");
			};
			assert_eq!(source_value, previous_destination);
			assert!(task.dependencies.contains(&previous_task.id));
			assert!(previous_task.window.end <= task.window.start);
		}
	}
	let final_copy = planned
		.value_copies
		.iter()
		.find(|copy| copy.logical == ValueId::new(2) && copy.device == DeviceId::new(3))
		.unwrap();
	let recipe_core::TransferEndpoint::Device {
		value: final_destination,
		..
	} = transfers[2].1.destination
	else {
		panic!("final hop must have a resident destination");
	};
	assert_eq!(final_copy.physical, final_destination);
	for (_, transfer) in &transfers[..2] {
		let recipe_core::TransferEndpoint::Device { value, .. } = transfer.destination else {
			panic!("intermediate hop must have a resident destination");
		};
		assert!(
			!planned
				.value_copies
				.iter()
				.any(|copy| copy.physical == value)
		);
	}
	assert!(planned.draft.tasks.iter().all(|task| match &task.kind {
		recipe_core::TaskKind::Transfer(transfer) => transfer.route.len() <= 1,
		recipe_core::TaskKind::Calculation(_) | recipe_core::TaskKind::Metric(_) => true,
	}));
	assert_eq!(planned.arena_layouts.len(), fixture.topology.devices.len());
	assert_eq!(
		planned
			.placements
			.iter()
			.map(|placement| (placement.kernel, placement.device))
			.collect::<BTreeMap<_, _>>(),
		BTreeMap::from([
			(KernelTemplateId::new(1), DeviceId::new(1)),
			(KernelTemplateId::new(2), DeviceId::new(3)),
		])
	);

	let rerun = plan_candidates(
		&chain_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let repeated = rerun
		.ranked_candidates()
		.iter()
		.find(|candidate| candidate.placements == planned.placements)
		.unwrap();
	assert_eq!(repeated.draft.identity, planned.draft.identity);
	assert_eq!(repeated.draft.values, planned.draft.values);
	assert_eq!(repeated.draft.tasks, planned.draft.tasks);
	assert_eq!(repeated.arena_layouts, planned.arena_layouts);
}

#[test]
fn checked_programs_receive_a_preallocated_device_fault_flag() {
	let fixture = fixture();
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 1, fixture.target_b.clone()),
	];
	let search = plan_candidates(
		&faulting_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert!(!search.ranked_candidates().is_empty());
	for planned in search.ranked_candidates() {
		let (calculation_task, calculation) = planned
			.draft
			.tasks
			.iter()
			.find_map(|task| match &task.kind {
				recipe_core::TaskKind::Calculation(calculation) => Some((task, calculation)),
				_ => None,
			})
			.unwrap();
		let fault_flag = calculation.fault_flag.unwrap();
		let spec = planned
			.draft
			.values
			.iter()
			.find(|value| value.id == fault_flag)
			.unwrap();
		assert_eq!(spec.device, calculation.device);
		assert_eq!(spec.dtype, recipe_core::DType::I32);
		assert_eq!(spec.bytes, ByteCount::new(4));
		let producer = spec.producer.unwrap();
		assert!(planned.draft.tasks.iter().any(|task| {
			task.id == producer
				&& task.phase == recipe_core::RunPhase::Init
				&& matches!(task.kind, recipe_core::TaskKind::Transfer(_))
		}));
		let (readback_task, readback) = planned
			.draft
			.tasks
			.iter()
			.find_map(|task| match &task.kind {
				recipe_core::TaskKind::Metric(metric)
					if metric.purpose
						== recipe_core::MetricPurpose::FaultReadback {
							calculation: calculation_task.id,
						} =>
				{
					Some((task, metric))
				}
				_ => None,
			})
			.unwrap();
		assert_eq!(readback.value, fault_flag);
		assert!(readback_task.dependencies.contains(&calculation_task.id));
		assert!(calculation_task.window.end <= readback_task.window.start);
		assert_eq!(
			planned
				.draft
				.resources
				.metrics
				.iter()
				.find(|slot| slot.id == readback.slot)
				.map(|slot| slot.metric),
			Some(readback.metric)
		);
		assert_eq!(
			planned
				.draft
				.tasks
				.iter()
				.filter(|task| {
					matches!(
						&task.kind,
						recipe_core::TaskKind::Metric(metric) if metric.slot == readback.slot
					)
				})
				.count(),
			1
		);
	}
}

#[test]
fn exhaustively_ranks_and_never_reissues_a_rejected_identity() {
	let fixture = fixture();
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 1, fixture.target_a.clone()),
		artifact(3, 1, fixture.target_b.clone()),
	];
	let mut search = plan_candidates(
		&one_node_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert!(!search.ranked_candidates().is_empty());
	assert!(search.ranked_candidates().iter().all(|candidate| {
		candidate.stage_placements.len() == 1
			&& candidate.draft.artifacts.is_empty()
			&& candidate.draft.artifact_builds.len() == 1
	}));
	let ranked = search
		.ranked_candidates()
		.iter()
		.map(|candidate| candidate.draft.candidate)
		.collect::<Vec<_>>();
	let first = search.next_candidate().unwrap().draft.candidate;
	search.reject(first).unwrap();
	let remaining = std::iter::from_fn(|| {
		search.next_candidate()
			.map(|candidate| candidate.draft.candidate)
	})
	.collect::<Vec<_>>();
	assert!(!remaining.is_empty());
	assert!(remaining.iter().all(|identity| *identity != first));
	assert!(ranked.contains(&first));
	assert!(remaining.iter().all(|identity| ranked.contains(identity)));
	assert!(search.next_candidate().is_none());

	let mut shuffled_artifacts = artifacts.clone();
	shuffled_artifacts.reverse();
	let rerun = plan_candidates(
		&one_node_graph(),
		&fixture.topology,
		&fixture.discovery,
		&shuffled_artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert_eq!(
		ranked,
		rerun.ranked_candidates()
			.iter()
			.map(|candidate| candidate.draft.candidate)
			.collect::<Vec<_>>()
	);
}

#[test]
fn rejects_only_capacity_infeasible_assignments() {
	let mut fixture = fixture();
	fixture
		.capacity
		.entries
		.iter_mut()
		.find(|entry| entry.device == DeviceId::new(1))
		.unwrap()
		.recipe_usable = measured(ByteCount::new(8));
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 1, fixture.target_b.clone()),
	];
	let search = plan_candidates(
		&one_node_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert_eq!(search.ranked_candidates().len(), 1);
	assert_eq!(
		search.ranked_candidates()[0].placements[0].device,
		DeviceId::new(3)
	);
}

#[test]
fn time_disjoint_auxiliary_peaks_share_capacity() {
	let mut fixture = fixture();
	fixture
		.capacity
		.entries
		.iter_mut()
		.find(|entry| entry.device == DeviceId::new(1))
		.unwrap()
		.recipe_usable = measured(ByteCount::new(192));
	let artifacts = [artifact(1, 1, fixture.target_a.clone())];
	let search = plan_candidates(
		&one_node_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let planned = search
		.ranked_candidates()
		.iter()
		.find(|candidate| candidate.placements[0].device == DeviceId::new(1))
		.unwrap();
	assert_eq!(
		planned
			.draft
			.resources
			.pinned_staging
			.iter()
			.find(|entry| entry.device == DeviceId::new(1))
			.unwrap()
			.bytes,
		ByteCount::new(64)
	);
	assert!(planned.draft.resources.scratch.is_empty());
	assert_eq!(
		planned
			.arena_layouts
			.iter()
			.find(|layout| layout.device == DeviceId::new(1))
			.unwrap()
			.size,
		ByteCount::new(128)
	);
}

#[test]
fn semantic_kernel_changes_participate_in_candidate_and_draft_identity() {
	let fixture = fixture();
	let artifacts = [artifact(1, 1, fixture.target_a.clone())];
	let baseline_search = plan_candidates(
		&one_node_graph(),
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let baseline = &baseline_search.ranked_candidates()[0].draft;
	let mut changed = one_node_graph();
	let PrimitiveKind::Elementwise(elementwise) = &mut changed.nodes[0].kernel.kind else {
		panic!("test graph contains an elementwise kernel");
	};
	elementwise.program.instructions[0].opcode = ScalarOpcode::Absolute;
	let changed_search = plan_candidates(
		&changed,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let changed = &changed_search.ranked_candidates()[0].draft;
	assert_ne!(baseline.candidate, changed.candidate);
	assert_ne!(baseline.identity, changed.identity);

	let mut changed_alias = one_node_graph();
	changed_alias.nodes[0].kernel.alias_rules[0].permission = AliasPermission::MayAliasExact;
	let changed_alias_search = plan_candidates(
		&changed_alias,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let changed_alias = &changed_alias_search.ranked_candidates()[0].draft;
	assert_ne!(baseline.candidate, changed_alias.candidate);
	assert_ne!(baseline.identity, changed_alias.identity);
}

#[test]
fn candidate_local_schedule_overflow_does_not_abort_exhaustive_search() {
	let mut fixture = fixture();
	let large_capacity = ByteCount::new(1_000_000_000_000);
	for device in &mut fixture.topology.devices {
		device.capacity = measured(large_capacity);
	}
	for device in &mut fixture.discovery.devices {
		device.total_capacity = measured(large_capacity);
		if device.device == DeviceId::new(1) {
			device.calculation.as_mut().unwrap().rate = measured(FlopsPerSecond::new(1).unwrap());
		}
	}
	for entry in &mut fixture.capacity.entries {
		entry.total = measured(large_capacity);
		entry.recipe_usable = measured(ByteCount::new(900_000_000_000));
	}
	let shape = Shape::new(vec![20_000_000_000]).unwrap();
	let graph = CalculationGraph {
		tensors: vec![
			Tensor::contiguous(
				ValueId::new(1),
				recipe_core::DType::F32,
				shape.clone(),
				true,
				false,
			)
			.unwrap(),
			Tensor::contiguous(ValueId::new(2), recipe_core::DType::F32, shape, false, true).unwrap(),
		],
		nodes: vec![unary(1, 1, 2)],
	};
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 1, fixture.target_b.clone()),
	];
	let search = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert_eq!(search.ranked_candidates().len(), 1);
	assert_eq!(
		search.ranked_candidates()[0].placements[0].device,
		DeviceId::new(3)
	);
}

#[test]
fn must_alias_writer_waits_for_other_readers_of_the_old_value() {
	let fixture = fixture();
	let mut writer = unary(1, 1, 2);
	writer.kernel.alias_rules[0].permission = AliasPermission::MustAliasExact;
	let graph = CalculationGraph {
		tensors: vec![
			tensor(1, true, false),
			tensor(2, false, true),
			tensor(3, false, true),
		],
		nodes: vec![writer, unary(2, 1, 3)],
	};
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 2, fixture.target_a.clone()),
	];
	let search = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let planned = search.ranked_candidates().iter().find(|candidate| {
		let writer_device = candidate
			.stage_placements
			.iter()
			.find(|stage| stage.source_kernel == KernelTemplateId::new(1))
			.map(|stage| stage.device);
		let reader_device = candidate
			.stage_placements
			.iter()
			.find(|stage| stage.source_kernel == KernelTemplateId::new(2))
			.map(|stage| stage.device);
		writer_device == reader_device
	});
	let planned = match planned {
		Some(planned) => planned,
		None => panic!("same-device placement must be enumerated"),
	};
	let writer_stage = planned
		.stage_placements
		.iter()
		.find(|stage| stage.source_kernel == KernelTemplateId::new(1))
		.unwrap()
		.kernel_template;
	let reader_stage = planned
		.stage_placements
		.iter()
		.find(|stage| stage.source_kernel == KernelTemplateId::new(2))
		.unwrap()
		.kernel_template;
	let writer = planned
		.draft
		.tasks
		.iter()
		.find(|task| {
			matches!(
					&task.kind,
					recipe_core::TaskKind::Calculation(calculation)
						if calculation.kernel_template == writer_stage
			)
		})
		.unwrap();
	let reader = planned
		.draft
		.tasks
		.iter()
		.find(|task| {
			matches!(
					&task.kind,
					recipe_core::TaskKind::Calculation(calculation)
						if calculation.kernel_template == reader_stage
			)
		})
		.unwrap();
	assert!(writer.dependencies.contains(&reader.id));
	assert!(reader.window.end <= writer.window.start);
	let writer_calculation = match &writer.kind {
		recipe_core::TaskKind::Calculation(calculation) => calculation,
		_ => panic!("writer is a calculation task"),
	};
	let input_binding = planned
		.draft
		.value_bindings
		.iter()
		.find(|binding| binding.value == writer_calculation.inputs[0])
		.unwrap();
	let output_binding = planned
		.draft
		.value_bindings
		.iter()
		.find(|binding| binding.value == writer_calculation.outputs[0])
		.unwrap();
	assert_eq!(input_binding.object, output_binding.object);
	assert_eq!(input_binding.object_offset, output_binding.object_offset);
}

#[test]
fn source_selection_accounts_for_readiness_and_route_contention() {
	let (fixture, target_c) = three_gpu_fixture();
	let graph = CalculationGraph {
		tensors: vec![
			tensor(1, true, false),
			tensor(2, false, false),
			tensor(3, false, true),
			tensor(4, false, true),
		],
		nodes: vec![unary(1, 1, 2), unary(2, 2, 3), unary(3, 2, 4)],
	};
	let artifacts = [
		artifact(1, 1, fixture.target_a.clone()),
		artifact(2, 2, fixture.target_b.clone()),
		artifact(3, 3, target_c),
	];
	let search = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let planned = search
		.ranked_candidates()
		.iter()
		.find(|candidate| {
			candidate
				.placements
				.iter()
				.map(|placement| (placement.kernel, placement.device))
				.collect::<BTreeMap<_, _>>()
				== BTreeMap::from([
					(KernelTemplateId::new(1), DeviceId::new(1)),
					(KernelTemplateId::new(2), DeviceId::new(3)),
					(KernelTemplateId::new(3), DeviceId::new(5)),
				])
		})
		.unwrap();
	let transfer = planned
		.draft
		.tasks
		.iter()
		.find_map(|task| {
			let recipe_core::TaskKind::Transfer(transfer) = &task.kind else {
				return None;
			};
			match transfer.destination {
				recipe_core::TransferEndpoint::Device { device, .. }
					if task.phase == recipe_core::RunPhase::Loop && device == DeviceId::new(5) =>
				{
					Some(transfer)
				}
				_ => None,
			}
		})
		.unwrap();
	assert!(
		matches!(
			transfer.source,
			recipe_core::TransferEndpoint::Device {
				device,
				..
			} if device == DeviceId::new(1)
		),
		"{transfer:?}"
	);
	assert_eq!(transfer.route, vec![LinkId::new(5)]);
}

#[test]
fn rejects_estimates_missing_capacity_and_missing_artifacts() {
	let mut estimated_fixture = fixture();
	let artifacts = [artifact(1, 1, estimated_fixture.target_a.clone())];
	estimated_fixture.topology.devices[0].capacity.provenance = PropertyProvenance::Estimated;
	let error = plan_candidates(
		&one_node_graph(),
		&estimated_fixture.topology,
		&estimated_fixture.discovery,
		&artifacts,
		&estimated_fixture.reservations,
		&estimated_fixture.capacity,
	)
	.unwrap_err();
	assert_eq!(error.kind, PlannerErrorKind::InvalidTopology);

	let mut capacity_fixture = fixture();
	capacity_fixture.capacity.entries.pop();
	let error = plan_candidates(
		&one_node_graph(),
		&capacity_fixture.topology,
		&capacity_fixture.discovery,
		&artifacts,
		&capacity_fixture.reservations,
		&capacity_fixture.capacity,
	)
	.unwrap_err();
	assert_eq!(error.kind, PlannerErrorKind::InvalidCapacity);

	let fixture = fixture();
	let deferred = plan_candidates(
		&one_node_graph(),
		&fixture.topology,
		&fixture.discovery,
		&[],
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert!(
		deferred.ranked_candidates().iter().all(|candidate| {
			candidate.draft.artifacts.is_empty() && candidate.draft.artifact_builds.len() == 1
		})
	);
}

#[test]
fn zero_element_programs_are_honest_no_dispatch_candidates() {
	let fixture = fixture();
	let graph = CalculationGraph {
		tensors: vec![
			Tensor::contiguous(
				ValueId::new(1),
				recipe_core::DType::F32,
				Shape::new(vec![0]).unwrap(),
				true,
				false,
			)
			.unwrap(),
			Tensor::contiguous(
				ValueId::new(2),
				recipe_core::DType::F32,
				Shape::new(vec![0]).unwrap(),
				false,
				true,
			)
			.unwrap(),
		],
		nodes: vec![unary(1, 1, 2)],
	};
	let artifacts = [artifact(1, 1, fixture.target_a.clone())];
	let search = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	assert!(search.ranked_candidates().iter().all(|candidate| {
		candidate.stage_placements.is_empty()
			&& candidate.draft.artifact_builds.is_empty()
			&& candidate
				.draft
				.tasks
				.iter()
				.all(|task| !matches!(task.kind, recipe_core::TaskKind::Calculation(_)))
	}));
}

#[test]
fn preserves_offset_stride_and_storage_in_kernel_addressing() {
	let fixture = fixture();
	let mut graph = one_node_graph();
	graph.tensors[0].layout.offset_elements = 1;
	graph.tensors[0].storage_bytes = ByteCount::new(68);
	let artifacts = [artifact(1, 1, fixture.target_a.clone())];
	let search = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let kernel = &search.ranked_candidates()[0].draft.kernels[0];
	assert_eq!(kernel.inputs[0].access.offset_elements, 1);
	assert_eq!(kernel.inputs[0].access.strides, vec![1]);
	assert_eq!(kernel.inputs[0].access.storage_bytes, ByteCount::new(68));
}

#[test]
fn lowers_right_aligned_input_broadcasts_to_zero_strides() {
	let fixture = fixture();
	let shape = |extents| Shape::new(extents).unwrap();
	let graph = CalculationGraph {
		tensors: vec![
			Tensor::contiguous(
				ValueId::new(1),
				recipe_core::DType::F32,
				shape(vec![3]),
				true,
				false,
			)
			.unwrap(),
			Tensor::contiguous(
				ValueId::new(2),
				recipe_core::DType::F32,
				shape(vec![2, 3]),
				true,
				false,
			)
			.unwrap(),
			Tensor::contiguous(
				ValueId::new(3),
				recipe_core::DType::F32,
				shape(vec![2, 3]),
				false,
				true,
			)
			.unwrap(),
		],
		nodes: vec![binary(1, 1, 2, 3)],
	};
	let artifacts = [artifact(1, 1, fixture.target_a.clone())];
	let search = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap();
	let kernel = &search.ranked_candidates()[0].draft.kernels[0];
	assert_eq!(kernel.inputs[0].access.strides, vec![0, 1]);
	assert_eq!(kernel.inputs[1].access.strides, vec![3, 1]);
	assert_eq!(kernel.outputs[0].access.strides, vec![3, 1]);
}

#[test]
fn rejects_must_alias_when_static_mappings_differ() {
	let fixture = fixture();
	let mut graph = one_node_graph();
	graph.tensors[0].layout.offset_elements = 1;
	graph.tensors[0].storage_bytes = ByteCount::new(68);
	graph.nodes[0].kernel.alias_rules[0].permission = AliasPermission::MustAliasExact;
	let artifacts = [artifact(1, 1, fixture.target_a.clone())];
	let error = plan_candidates(
		&graph,
		&fixture.topology,
		&fixture.discovery,
		&artifacts,
		&fixture.reservations,
		&fixture.capacity,
	)
	.unwrap_err();
	assert_eq!(error.kind, PlannerErrorKind::InvalidGraph);
}

#[test]
fn every_primitive_family_reaches_an_exact_stage_build_contract() {
	let fixture = fixture();
	let cases = vec![
		(
			"reduce",
			primitive_graph(
				201,
				vec![
					typed_tensor(1, DType::F32, &[2, 9], true, false),
					typed_tensor(2, DType::F32, &[2], false, true),
				],
				&[1],
				&[2],
				PrimitiveKind::Reduce(Reduce {
					operator: ReduceOperator::Sum,
					axes: AxisSet::new(vec![1]).unwrap(),
					keep_dimensions: false,
					result: ReduceResult::Value,
					tree_lanes: 4,
				}),
			),
			2,
		),
		(
			"scan",
			primitive_graph(
				202,
				vec![
					typed_tensor(1, DType::F32, &[2, 35], true, false),
					typed_tensor(2, DType::F32, &[2, 35], false, true),
				],
				&[1],
				&[2],
				PrimitiveKind::Scan(Scan {
					operator: ReduceOperator::Sum,
					axis: 1,
					mode: ScanMode::Inclusive,
					reverse: false,
					tree_lanes: 4,
				}),
			),
			2,
		),
		(
			"contraction",
			primitive_graph(
				203,
				vec![
					typed_tensor(1, DType::F32, &[2, 3, 5], true, false),
					typed_tensor(2, DType::F32, &[2, 5, 7], true, false),
					typed_tensor(3, DType::F32, &[2, 3, 7], false, true),
				],
				&[1, 2],
				&[3],
				PrimitiveKind::Contraction(Contraction {
					batch_axes: vec![(0, 0)],
					contract_axes: vec![(2, 1)],
				}),
			),
			1,
		),
		(
			"gather",
			primitive_graph(
				204,
				vec![
					typed_tensor(1, DType::F32, &[4, 9], true, false),
					typed_tensor(2, DType::I32, &[3], true, false),
					typed_tensor(3, DType::F32, &[4, 3], false, true),
				],
				&[1, 2],
				&[3],
				PrimitiveKind::Gather(Gather {
					axis: 1,
					bounds: IndexBounds::Reject,
				}),
			),
			1,
		),
		(
			"scatter",
			primitive_graph(
				205,
				vec![
					typed_tensor(1, DType::F32, &[4, 9], true, false),
					typed_tensor(2, DType::I32, &[3], true, false),
					typed_tensor(3, DType::F32, &[4, 3], true, false),
					typed_tensor(4, DType::F32, &[4, 9], false, true),
				],
				&[1, 2, 3],
				&[4],
				PrimitiveKind::Scatter(Scatter {
					axis: 1,
					bounds: IndexBounds::Reject,
					conflict: ScatterConflict::Atomic {
						operation: AtomicOperation::Add,
						ordering: AtomicOrdering::Relaxed,
					},
				}),
			),
			1,
		),
		(
			"histogram",
			primitive_graph(
				206,
				vec![
					typed_tensor(1, DType::I32, &[33], true, false),
					typed_tensor(2, DType::I32, &[7], false, true),
				],
				&[1],
				&[2],
				PrimitiveKind::Histogram(Histogram {
					bins: 7,
					weighted: false,
					ordering: AtomicOrdering::SequentiallyConsistent,
				}),
			),
			2,
		),
		(
			"sort",
			primitive_graph(
				207,
				vec![
					typed_tensor(1, DType::F32, &[3, 7], true, false),
					typed_tensor(2, DType::F32, &[3, 7], false, true),
				],
				&[1],
				&[2],
				PrimitiveKind::Sort(Sort {
					axis: 1,
					direction: SortDirection::Ascending,
					stable: true,
					emit_indices: false,
				}),
			),
			6,
		),
		(
			"random",
			primitive_graph(
				208,
				vec![typed_tensor(1, DType::F32, &[1024], false, true)],
				&[],
				&[1],
				PrimitiveKind::Random(RandomMap {
					distribution: RandomDistribution::UniformF32,
					key: RandomKey {
						seed_low: 1,
						seed_high: 2,
						stream: 3,
					},
					philox_rounds: 10,
				}),
			),
			1,
		),
	];
	for (name, graph, minimum_stage_count) in cases {
		let search = plan_candidates(
			&graph,
			&fixture.topology,
			&fixture.discovery,
			&[],
			&fixture.reservations,
			&fixture.capacity,
		)
		.unwrap_or_else(|error| panic!("{name} planning failed: {error}"));
		let planned = &search.ranked_candidates()[0];
		planned
			.draft
			.validate(&fixture.topology, &fixture.discovery)
			.unwrap_or_else(|errors| panic!("{name} draft validation failed: {errors}"));
		let program = &planned.lowered_programs[0];
		assert!(program.stages.len() >= minimum_stage_count, "{name}");
		assert_eq!(
			planned.stage_placements.len(),
			program.stages.len(),
			"{name}"
		);
		assert_eq!(
			planned.draft.artifact_builds.len(),
			program.stages.len(),
			"{name}"
		);
		let templates = planned
			.stage_placements
			.iter()
			.map(|placement| placement.kernel_template)
			.collect::<BTreeSet<_>>();
		assert_eq!(templates.len(), program.stages.len(), "{name}");
		assert!(!templates.contains(&program.source_kernel), "{name}");
		for stage in &program.stages {
			let placement = planned
				.stage_placements
				.iter()
				.find(|placement| placement.stage_ordinal == stage.id.get())
				.unwrap_or_else(|| panic!("{name} omitted stage {}", stage.id.get()));
			let build = planned
				.draft
				.artifact_builds
				.iter()
				.find(|build| build.artifact == placement.artifact)
				.unwrap_or_else(|| panic!("{name} omitted build {}", stage.id.get()));
			build.validate()
				.unwrap_or_else(|errors| panic!("{name} emitted invalid build: {errors}"));
			assert_eq!(build.kernel_template, placement.kernel_template, "{name}");
			assert_eq!(build.source_kernel, program.source_kernel, "{name}");
			assert_eq!(build.provenance.stage_ordinal, stage.id.get(), "{name}");
			assert_eq!(
				build.provenance.program_digest,
				Digest::new(program.digest.bytes()),
				"{name}"
			);
			assert_eq!(build.artifact.get(), build.kernel_template.get(), "{name}");
			assert_eq!(build.work.flops, stage.resources.flops, "{name}");
			assert_eq!(
				build.work.integer_operations, stage.resources.integer_operations,
				"{name}"
			);
			assert_eq!(
				build.work.atomic_operations, stage.resources.atomic_operations,
				"{name}"
			);
			assert_eq!(build.fault_flag.is_some(), stage.fault.is_some(), "{name}");
			let task = planned
				.draft
				.tasks
				.iter()
				.find(|task| {
					matches!(
						&task.kind,
						recipe_core::TaskKind::Calculation(calculation)
							if calculation.kernel_template == placement.kernel_template
								&& calculation.artifact == build.artifact
					)
				})
				.unwrap_or_else(|| panic!("{name} omitted calculation {}", stage.id.get()));
			for dependency in &stage.dependencies {
				let dependency_template = planned
					.stage_placements
					.iter()
					.find(|placement| placement.stage_ordinal == dependency.get())
					.map(|placement| placement.kernel_template)
					.unwrap_or_else(|| panic!("{name} omitted dependency stage"));
				let dependency_task = planned
					.draft
					.tasks
					.iter()
					.find(|task| {
						matches!(
							&task.kind,
							recipe_core::TaskKind::Calculation(calculation)
								if calculation.kernel_template == dependency_template
						)
					})
					.unwrap_or_else(|| panic!("{name} omitted dependency calculation"));
				assert!(task.dependencies.contains(&dependency_task.id), "{name}");
			}
			for binding in &build.bindings {
				let value = planned
					.draft
					.values
					.iter()
					.find(|value| value.id == binding.value)
					.unwrap_or_else(|| panic!("{name} build references an absent value"));
				assert_eq!(value.device, placement.device, "{name}");
				assert_eq!(value.dtype, binding.dtype, "{name}");
				assert_eq!(value.bytes, binding.view.storage_bytes, "{name}");
			}
		}
	}
}
