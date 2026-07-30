use std::time::Duration;

use recipe_core::{
	ByteCount, BytesPerSecond, CalculationCapability, Device, DeviceId, DeviceKind, Digest, DirectedLink,
	DiscoveredDevice, DiscoveredLink, DiscoveryIdentity, DiscoveryProfile, DuplexMode, DuplexResourceId,
	FlopsPerSecond, Label, LinkId, Machine, MachineId, Node, NodeId, NodeRole, Property, PropertyProvenance,
	TargetIdentity, Topology, TopologyIdentity, TransferCapability, TransferLaneCount, TransportId, TransportKind,
};
use recipe_probe::{
	BenchmarkMetadata, BoundedBenchmarkPlan, CONTRACT_SCHEMA, CacheIdentity, LinkDuplex, MachineFingerprint,
	MeasuredGpuOrigin, MeasuredMachineOrigin, MeasuredOrigins, MeasuredProfile, MeasuredRamOrigin,
	MeasuredStorageOrigin, PEER_BENCHMARK_PROTOCOL_SCHEMA, PROFILE_SCHEMA, PeerBenchmarkEvidence, PeerDescriptor,
	PeerDuplexExecution, PeerEndpointEvidence, PeerMeasurement,
};
use recipe_transport::{MeasuredLocalMemory, SessionIdentity};

use super::*;

fn label(value: &str) -> Label {
	must(Label::new(value))
}

fn rate(value: u64) -> BytesPerSecond {
	must(BytesPerSecond::new(value))
}

fn flops(value: u64) -> FlopsPerSecond {
	must(FlopsPerSecond::new(value))
}

fn lanes(value: u32) -> TransferLaneCount {
	must(TransferLaneCount::new(value))
}

fn must<T, E: core::fmt::Display>(result: Result<T, E>) -> T {
	match result {
		Ok(value) => value,
		Err(error) => panic!("test fixture construction failed: {error}"),
	}
}

fn some<T>(value: Option<T>) -> T {
	match value {
		Some(value) => value,
		None => panic!("test fixture expected a value"),
	}
}

fn must_error<T, E>(result: Result<T, E>) -> E {
	match result {
		Ok(_) => panic!("test expected an error"),
		Err(error) => error,
	}
}

fn measured<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Measured)
}

fn benchmark_plan() -> BoundedBenchmarkPlan {
	BoundedBenchmarkPlan {
		buffer_bytes: ByteCount::new(1_048_576),
		iterations: 4,
		maximum_duration: Duration::from_secs(2),
	}
}

fn benchmark_metadata() -> BenchmarkMetadata {
	let plan = benchmark_plan();
	BenchmarkMetadata {
		seed_schema: CONTRACT_SCHEMA,
		ram: plan,
		storage: plan,
		gpu: plan,
		network: plan,
	}
}

fn machine_fingerprint(index: u8, name: &str) -> MachineFingerprint {
	MachineFingerprint {
		hostname: label(name),
		stable_id: label(&format!("stable-{index}-{name}")),
		runtime_abi: label("linux-x86_64"),
		firmware: label("firmware-1"),
	}
}

fn single_machine_profile(index: u8, name: &str) -> MeasuredProfile {
	let machine = MachineId::new(1);
	let ram = DeviceId::new(1);
	let disk = DeviceId::new(2);
	let gpu = DeviceId::new(3);
	let ram_capacity = ByteCount::new(48_000_000_000 + u64::from(index) * 1_000_000);
	let ram_rate = rate(90_000_000_000 + u64::from(index) * 1_000_000);
	let disk_capacity = ByteCount::new(1_000_000_000_000 + u64::from(index) * 1_000_000);
	let disk_rate = rate(500_000_000 + u64::from(index) * 1_000_000);
	let gpu_capacity = ByteCount::new(12_000_000_000 + u64::from(index) * 1_000_000);
	let gpu_rate = rate(400_000_000_000 + u64::from(index) * 1_000_000);
	let gpu_flops = flops(300_000_000_000 + u64::from(index) * 1_000_000);
	let backend = match index.is_multiple_of(2) {
		true => "cuda",
		false => "hsa",
	};
	let target = TargetIdentity {
		backend: label(backend),
		architecture: label(&format!("gpu-{name}")),
		abi: label("recipe-owned-v1"),
	};
	let devices = vec![
		Device {
			id: ram,
			machine,
			kind: DeviceKind::Ram,
			capacity: measured(ram_capacity),
			transfer_rate: measured(ram_rate),
			calculation_rate: None,
		},
		Device {
			id: disk,
			machine,
			kind: DeviceKind::Disk,
			capacity: measured(disk_capacity),
			transfer_rate: measured(disk_rate),
			calculation_rate: None,
		},
		Device {
			id: gpu,
			machine,
			kind: DeviceKind::GpuMemory,
			capacity: measured(gpu_capacity),
			transfer_rate: measured(gpu_rate),
			calculation_rate: Some(measured(gpu_flops)),
		},
	];
	let links = vec![
		DirectedLink {
			id: LinkId::new(1),
			transport: TransportId::new(1),
			kind: TransportKind::Sata,
			duplex: DuplexMode::Half,
			from: ram,
			to: disk,
			bandwidth: measured(rate(450_000_000)),
			maximum_inflight_transfers: measured(lanes(1)),
			capacity_resource: DuplexResourceId::new(1),
		},
		DirectedLink {
			id: LinkId::new(2),
			transport: TransportId::new(1),
			kind: TransportKind::Sata,
			duplex: DuplexMode::Half,
			from: disk,
			to: ram,
			bandwidth: measured(rate(500_000_000)),
			maximum_inflight_transfers: measured(lanes(1)),
			capacity_resource: DuplexResourceId::new(1),
		},
		DirectedLink {
			id: LinkId::new(3),
			transport: TransportId::new(2),
			kind: TransportKind::Pcie,
			duplex: DuplexMode::Full,
			from: ram,
			to: gpu,
			bandwidth: measured(rate(12_000_000_000)),
			maximum_inflight_transfers: measured(lanes(2)),
			capacity_resource: DuplexResourceId::new(2),
		},
		DirectedLink {
			id: LinkId::new(4),
			transport: TransportId::new(2),
			kind: TransportKind::Pcie,
			duplex: DuplexMode::Full,
			from: gpu,
			to: ram,
			bandwidth: measured(rate(13_000_000_000)),
			maximum_inflight_transfers: measured(lanes(3)),
			capacity_resource: DuplexResourceId::new(3),
		},
	];
	let topology_identity = TopologyIdentity::new(Digest::new([index + 20; 32]));
	let topology = Topology {
		identity: topology_identity,
		machines: vec![Machine {
			id: machine,
			name: label(name),
		}],
		nodes: vec![Node {
			id: NodeId::new(1),
			role: NodeRole::Master,
			machine,
			devices: vec![ram, disk, gpu],
		}],
		devices,
		links,
	};
	let discovery = DiscoveryProfile {
		identity: DiscoveryIdentity::new(Digest::new([index + 40; 32])),
		topology: topology_identity,
		devices: vec![
			DiscoveredDevice {
				device: ram,
				available: true,
				maximum_submission_queues: 4,
				total_capacity: measured(ram_capacity),
				transfer: TransferCapability {
					rate: measured(ram_rate),
					maximum_inflight_transfers: measured(lanes(4)),
					asynchronous_submission: true,
					overlaps_calculation: true,
				},
				calculation: None,
			},
			DiscoveredDevice {
				device: disk,
				available: true,
				maximum_submission_queues: 1,
				total_capacity: measured(disk_capacity),
				transfer: TransferCapability {
					rate: measured(disk_rate),
					maximum_inflight_transfers: measured(lanes(1)),
					asynchronous_submission: true,
					overlaps_calculation: true,
				},
				calculation: None,
			},
			DiscoveredDevice {
				device: gpu,
				available: true,
				maximum_submission_queues: 2,
				total_capacity: measured(gpu_capacity),
				transfer: TransferCapability {
					rate: measured(gpu_rate),
					maximum_inflight_transfers: measured(lanes(2)),
					asynchronous_submission: true,
					overlaps_calculation: true,
				},
				calculation: Some(CalculationCapability {
					target,
					rate: measured(gpu_flops),
					asynchronous_submission: true,
					maximum_concurrent_tasks: 2,
					subgroup_lanes: 32,
					maximum_workgroup_lanes: 256,
					maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
				}),
			},
		],
		links: (1..=4)
			.map(|value| {
				let link = some(topology.link(LinkId::new(value)));
				DiscoveredLink {
					link: link.id,
					available: true,
					bandwidth: link.bandwidth,
					maximum_inflight_transfers: link.maximum_inflight_transfers,
					asynchronous_submission: true,
				}
			})
			.collect(),
	};
	MeasuredProfile {
		schema: PROFILE_SCHEMA,
		cache_identity: CacheIdentity {
			schema: PROFILE_SCHEMA,
			digest: Digest::new([index + 60; 32]),
		},
		origins: MeasuredOrigins {
			machines: vec![MeasuredMachineOrigin {
				machine,
				fingerprint: machine_fingerprint(index, name),
			}],
			ram: vec![MeasuredRamOrigin {
				device: ram,
				key: label("ram"),
			}],
			storage: vec![MeasuredStorageOrigin {
				device: disk,
				key: label("storage:root"),
			}],
			gpu: vec![MeasuredGpuOrigin {
				device: gpu,
				key: label(&format!("{backend}:gpu-{name}")),
			}],
		},
		benchmarks: benchmark_metadata(),
		peer_benchmarks: Vec::new(),
		topology,
		discovery,
	}
}

fn member_spec(key: &str, address: &str, profile: &MeasuredProfile) -> MemberSpec {
	MemberSpec::new(
		label(key),
		label(address),
		must(MemberProfileIdentity::derive(profile)),
	)
}

#[derive(Clone, Copy, Debug)]
struct EvidenceShape {
	kind: TransportKind,
	duplex: LinkDuplex,
	outbound: u64,
	inbound: u64,
	outbound_lanes: u32,
	inbound_lanes: u32,
}

fn evidence(
	local_key: &str,
	remote_key: &str,
	local: &MeasuredProfile,
	remote: &MeasuredProfile,
	shape: EvidenceShape,
) -> MeasuredNetworkPair {
	let local_identity = must(MemberProfileIdentity::derive(local));
	let remote_identity = must(MemberProfileIdentity::derive(remote));
	let session = must(SessionIdentity::new(
		local_identity.endpoint(),
		remote_identity.endpoint(),
	));
	let local_ram = some(local
		.topology
		.devices
		.iter()
		.find(|device| device.kind == DeviceKind::Ram));
	let remote_ram = some(remote
		.topology
		.devices
		.iter()
		.find(|device| device.kind == DeviceKind::Ram));
	let descriptor = PeerDescriptor {
		session_id: label(&format!("{local_key}-to-{remote_key}")),
		machine: remote.origins.machines[0].fingerprint.clone(),
		remote_memory_key: label("ram"),
		local_memory_key: label("ram"),
		local_interface_key: label("network-0"),
		remote_interface_identity: label(&format!("interface-{remote_key}")),
		remote_driver: label("driver-1"),
		remote_firmware: label("nic-firmware-1"),
		link_identity: label(&format!("link-{local_key}-{remote_key}")),
		transport_kind: shape.kind,
		duplex: shape.duplex,
		outbound_maximum_inflight: lanes(shape.outbound_lanes),
		inbound_maximum_inflight: lanes(shape.inbound_lanes),
		asynchronous_submission: true,
	};
	let local_memory = must(MeasuredLocalMemory::new(
		local_ram.capacity.value,
		local_ram.transfer_rate.value,
	));
	let plan = benchmark_plan();
	let (outbound_evidence, outbound_rate) = direction_evidence(plan, shape.outbound);
	let (inbound_evidence, inbound_rate) = direction_evidence(plan, shape.inbound);
	let measurement = PeerMeasurement {
		remote_memory_capacity: remote_ram.capacity,
		remote_memory_rate: remote_ram.transfer_rate,
		outbound_rate: Some(measured(outbound_rate)),
		inbound_rate: Some(measured(inbound_rate)),
		evidence: PeerBenchmarkEvidence {
			protocol_schema: PEER_BENCHMARK_PROTOCOL_SCHEMA,
			local_endpoint: PeerEndpointEvidence {
				machine: session.local().machine(),
				profile: session.local().profile(),
			},
			remote_endpoint: PeerEndpointEvidence {
				machine: session.remote().machine(),
				profile: session.remote().profile(),
			},
			execution: match shape.duplex {
				LinkDuplex::Full => PeerDuplexExecution::Simultaneous,
				LinkDuplex::Half => PeerDuplexExecution::Serialized,
			},
			outbound: outbound_evidence,
			inbound: inbound_evidence,
		},
	};
	must(MeasuredNetworkPair::from_probe(
		label(local_key),
		label(remote_key),
		session,
		descriptor,
		local_memory,
		measurement,
		plan,
	))
}

fn direction_evidence(
	plan: BoundedBenchmarkPlan,
	requested_rate: u64,
) -> (recipe_probe::DirectionalBenchmarkEvidence, BytesPerSecond) {
	let total_bytes = plan.buffer_bytes.get() * u64::from(plan.iterations);
	let elapsed_nanoseconds =
		u64::try_from((u128::from(total_bytes) * 1_000_000_000).div_ceil(u128::from(requested_rate))).unwrap();
	let actual_rate =
		rate(u64::try_from(u128::from(total_bytes) * 1_000_000_000 / u128::from(elapsed_nanoseconds)).unwrap());
	let mean = (elapsed_nanoseconds / u64::from(plan.iterations)).max(1);
	(
		recipe_probe::DirectionalBenchmarkEvidence {
			total_bytes: ByteCount::new(total_bytes),
			elapsed_nanoseconds,
			sample_count: plan.iterations,
			minimum_sample_nanoseconds: mean,
			maximum_sample_nanoseconds: mean,
			mean_sample_nanoseconds: mean,
			variance_nanoseconds_squared: 0,
		},
		actual_rate,
	)
}

fn fixture() -> (
	ClusterConfiguration,
	Vec<SubmittedProfile>,
	Vec<MeasuredNetworkPair>,
) {
	let alpha = single_machine_profile(1, "alpha");
	let beta = single_machine_profile(2, "beta");
	let gamma = single_machine_profile(3, "gamma");
	let ethernet = evidence(
		"alpha",
		"beta",
		&alpha,
		&beta,
		EvidenceShape {
			kind: TransportKind::Ethernet,
			duplex: LinkDuplex::Full,
			outbound: 125_000_000,
			inbound: 120_000_000,
			outbound_lanes: 2,
			inbound_lanes: 3,
		},
	);
	let wlan = evidence(
		"gamma",
		"beta",
		&gamma,
		&beta,
		EvidenceShape {
			kind: TransportKind::Wlan,
			duplex: LinkDuplex::Half,
			outbound: 70_000_000,
			inbound: 65_000_000,
			outbound_lanes: 1,
			inbound_lanes: 1,
		},
	);
	let configuration = must(ClusterConfiguration::new(
		label("alpha"),
		vec![
			member_spec("gamma", "gamma.example:7000", &gamma),
			member_spec("alpha", "alpha.example:7000", &alpha),
			member_spec("beta", "beta.example:7000", &beta),
		],
		vec![
			must(NetworkPairSpec::new(label("beta"), label("gamma"))),
			must(NetworkPairSpec::new(label("beta"), label("alpha"))),
		],
	));
	(
		configuration,
		vec![
			SubmittedProfile::new(label("beta"), beta),
			SubmittedProfile::new(label("gamma"), gamma),
			SubmittedProfile::new(label("alpha"), alpha),
		],
		vec![wlan, ethernet],
	)
}

#[test]
fn merges_three_complete_profiles_and_round_trips_codec() {
	let (configuration, submissions, measurements) = fixture();
	let profile = must(assemble_cluster(&configuration, submissions, measurements));

	assert_eq!(profile.topology.machines.len(), 3);
	assert_eq!(profile.topology.nodes.len(), 3);
	assert_eq!(profile.topology.devices.len(), 9);
	assert_eq!(profile.origins.machines.len(), 3);
	assert_eq!(profile.origins.ram.len(), 3);
	assert_eq!(profile.origins.storage.len(), 3);
	assert_eq!(profile.origins.gpu.len(), 3);
	assert_eq!(
		profile
			.topology
			.devices
			.iter()
			.filter(|device| device.kind == DeviceKind::GpuMemory)
			.count(),
		3
	);
	assert_eq!(
		profile
			.topology
			.devices
			.iter()
			.filter(|device| device.kind == DeviceKind::Disk)
			.count(),
		3
	);
	assert_eq!(
		profile
			.discovery
			.devices
			.iter()
			.filter(|device| device.calculation.is_some())
			.count(),
		3
	);
	let architectures = profile
		.discovery
		.devices
		.iter()
		.filter_map(|device| {
			device.calculation
				.as_ref()
				.map(|calculation| calculation.target.architecture.as_str())
		})
		.collect::<Vec<_>>();
	assert_eq!(architectures, vec!["gpu-alpha", "gpu-beta", "gpu-gamma"]);
	let master = profile
		.topology
		.nodes
		.iter()
		.filter(|node| node.role == NodeRole::Master)
		.collect::<Vec<_>>();
	assert_eq!(master.len(), 1);
	assert_eq!(
		some(profile.topology.machine(master[0].machine)).name,
		label("alpha")
	);

	let cross_machine = profile
		.topology
		.links
		.iter()
		.filter(|link| {
			some(profile.topology.device(link.from)).machine != some(profile.topology.device(link.to)).machine
		})
		.collect::<Vec<_>>();
	assert_eq!(cross_machine.len(), 4);
	let ethernet = cross_machine
		.iter()
		.copied()
		.filter(|link| link.kind == TransportKind::Ethernet)
		.collect::<Vec<_>>();
	assert_eq!(ethernet.len(), 2);
	assert_eq!(ethernet[0].duplex, DuplexMode::Full);
	assert_ne!(ethernet[0].capacity_resource, ethernet[1].capacity_resource);
	assert_eq!(ethernet[0].maximum_inflight_transfers.value, lanes(2));
	assert_eq!(ethernet[1].maximum_inflight_transfers.value, lanes(3));
	let wlan = cross_machine
		.iter()
		.copied()
		.filter(|link| link.kind == TransportKind::Wlan)
		.collect::<Vec<_>>();
	assert_eq!(wlan.len(), 2);
	assert_eq!(wlan[0].duplex, DuplexMode::Half);
	assert_eq!(wlan[0].capacity_resource, wlan[1].capacity_resource);
	assert!(profile.topology.validate().is_ok());
	assert!(profile.discovery.validate(&profile.topology).is_ok());

	let encoded = must(ClusterProfileCodec::encode(&profile));
	let decoded = must(ClusterProfileCodec::decode(&encoded));
	assert_eq!(decoded, profile);
}

#[test]
fn assembly_is_independent_of_configuration_and_input_order() {
	let (configuration, submissions, measurements) = fixture();
	let first = must(assemble_cluster(
		&configuration,
		submissions.clone(),
		measurements.clone(),
	));
	let reversed_configuration = must(ClusterConfiguration::new(
		configuration.master().clone(),
		configuration.members().iter().cloned().rev().collect(),
		configuration
			.network_pairs()
			.iter()
			.cloned()
			.rev()
			.collect(),
	));
	let second = must(assemble_cluster(
		&reversed_configuration,
		submissions.into_iter().rev().collect(),
		measurements.into_iter().rev().collect(),
	));
	assert_eq!(second, first);
	assert_eq!(second.cache_identity, first.cache_identity);
	assert_eq!(second.topology.identity, first.topology.identity);
	assert_eq!(second.discovery.identity, first.discovery.identity);
}

#[test]
fn stable_machine_endpoint_is_independent_of_measured_profile_content() {
	let mut profile = single_machine_profile(1, "alpha");
	let first = must(MemberProfileIdentity::derive(&profile));
	let ram_id = profile.origins.ram[0].device;
	let topology_ram = some(profile
		.topology
		.devices
		.iter_mut()
		.find(|device| device.id == ram_id));
	topology_ram.capacity = measured(ByteCount::new(topology_ram.capacity.value.get() + 1));
	let discovery_ram = some(profile
		.discovery
		.devices
		.iter_mut()
		.find(|device| device.device == ram_id));
	discovery_ram.total_capacity = topology_ram.capacity;
	let second = must(MemberProfileIdentity::derive(&profile));
	assert_eq!(first.endpoint().machine(), second.endpoint().machine());
	assert_ne!(first.endpoint().profile(), second.endpoint().profile());
}

#[test]
fn stale_member_content_is_rejected_even_when_embedded_ids_are_unchanged() {
	let (configuration, mut submissions, measurements) = fixture();
	let alpha = some(submissions
		.iter_mut()
		.find(|submission| submission.key() == &label("alpha")));
	alpha.profile_mut().origins.machines[0].fingerprint.firmware = label("firmware-2");
	let error = must_error(assemble_cluster(&configuration, submissions, measurements));
	assert_eq!(error, ClusterError::StaleMember("alpha".to_owned()));
}

#[test]
fn missing_and_duplicate_members_fail_closed() {
	let (configuration, mut submissions, measurements) = fixture();
	submissions.retain(|submission| submission.key() != &label("gamma"));
	let error = must_error(assemble_cluster(
		&configuration,
		submissions,
		measurements.clone(),
	));
	assert_eq!(error, ClusterError::MissingMember("gamma".to_owned()));

	let (_, mut submissions, _) = fixture();
	submissions.push(submissions[0].clone());
	let error = must_error(assemble_cluster(&configuration, submissions, measurements));
	assert_eq!(error, ClusterError::DuplicateMember("beta".to_owned()));
}

#[test]
fn missing_or_unconfigured_network_evidence_fails_closed() {
	let (configuration, submissions, mut measurements) = fixture();
	measurements.pop();
	let error = must_error(assemble_cluster(&configuration, submissions, measurements));
	assert!(matches!(error, ClusterError::MissingNetworkMeasurement(_)));

	let (configuration, submissions, mut measurements) = fixture();
	let duplicate = measurements[0].clone();
	measurements.push(duplicate);
	let error = must_error(assemble_cluster(&configuration, submissions, measurements));
	assert!(matches!(
		error,
		ClusterError::DuplicateNetworkMeasurement(_)
	));
}

#[test]
fn exact_ram_origin_key_is_required_even_when_measurements_match() {
	let (configuration, submissions, mut measurements) = fixture();
	measurements[0].descriptor.local_memory_key = label("wrong-ram-key");
	let error = must_error(assemble_cluster(&configuration, submissions, measurements));
	match error {
		ClusterError::InvalidNetworkMeasurement(message) => {
			assert!(message.contains("wrong-ram-key"));
			assert!(message.contains("absent"));
		}
		other => panic!("unexpected cluster error: {other}"),
	}
}

#[test]
fn exact_ram_key_disambiguates_identical_measured_domains() {
	let mut alpha = single_machine_profile(1, "alpha");
	let beta = single_machine_profile(2, "beta");
	let alternate_id = DeviceId::new(4);
	let mut alternate = some(alpha.topology.device(DeviceId::new(1))).clone();
	alternate.id = alternate_id;
	alpha.topology.devices.push(alternate);
	alpha.topology.nodes[0].devices.push(alternate_id);
	let mut alternate_discovery = some(alpha
		.discovery
		.devices
		.iter()
		.find(|device| device.device == DeviceId::new(1)))
	.clone();
	alternate_discovery.device = alternate_id;
	alpha.discovery.devices.push(alternate_discovery);
	alpha.origins.ram.push(MeasuredRamOrigin {
		device: alternate_id,
		key: label("ram-alternate"),
	});

	let network = evidence(
		"alpha",
		"beta",
		&alpha,
		&beta,
		EvidenceShape {
			kind: TransportKind::Ethernet,
			duplex: LinkDuplex::Full,
			outbound: 125_000_000,
			inbound: 120_000_000,
			outbound_lanes: 2,
			inbound_lanes: 2,
		},
	);
	let configuration = must(ClusterConfiguration::new(
		label("alpha"),
		vec![
			member_spec("alpha", "alpha.example:7000", &alpha),
			member_spec("beta", "beta.example:7000", &beta),
		],
		vec![must(NetworkPairSpec::new(label("alpha"), label("beta")))],
	));
	let profile = must(assemble_cluster(
		&configuration,
		vec![
			SubmittedProfile::new(label("beta"), beta),
			SubmittedProfile::new(label("alpha"), alpha),
		],
		vec![network],
	));
	let alpha_machine = some(profile
		.origins
		.machines
		.iter()
		.find(|origin| origin.fingerprint.hostname == label("alpha")))
	.machine;
	let selected = some(profile.origins.ram.iter().find(|origin| {
		origin.key == label("ram") && some(profile.topology.device(origin.device)).machine == alpha_machine
	}))
	.device;
	let alternate = some(profile.origins.ram.iter().find(|origin| {
		origin.key == label("ram-alternate")
			&& some(profile.topology.device(origin.device)).machine == alpha_machine
	}))
	.device;
	let outbound = some(profile.topology.links.iter().find(|link| {
		link.kind == TransportKind::Ethernet && some(profile.topology.device(link.from)).machine == alpha_machine
	}));
	assert_eq!(outbound.from, selected);
	assert_ne!(outbound.from, alternate);
}
