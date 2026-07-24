use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{
	Device, DeviceId, DirectedLink, DiscoveredDevice, DiscoveredLink, DiscoveryIdentity, DiscoveryProfile,
	DuplexMode, DuplexResourceId, LinkId, Machine, MachineId, Node, NodeId, NodeRole, Property, PropertyProvenance,
	Topology, TopologyIdentity, TransferLaneCount, TransportId, TransportKind,
};
use recipe_probe::{
	BenchmarkMetadata, CacheIdentity, LinkDuplex, MeasuredGpuOrigin, MeasuredMachineOrigin, MeasuredOrigins,
	MeasuredPeerBenchmark, MeasuredProfile, MeasuredProfileCodec, MeasuredRamOrigin, MeasuredStorageOrigin,
	PROFILE_SCHEMA,
};

use crate::error::{ClusterError, ClusterResult};
use crate::hash::cluster_digest;
use crate::model::{
	ClusterConfiguration, MeasuredNetworkPair, MemberProfileIdentity, PairKey, SubmittedProfile, index_submissions,
};

#[derive(Clone, Debug)]
pub(crate) struct PreparedMember {
	pub(crate) key: recipe_core::Label,
	pub(crate) identity: MemberProfileIdentity,
	pub(crate) profile: MeasuredProfile,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedNetworkPair {
	pub(crate) key: PairKey,
	pub(crate) first_device: DeviceId,
	pub(crate) second_device: DeviceId,
	pub(crate) kind: TransportKind,
	pub(crate) duplex: DuplexMode,
	pub(crate) first_to_second: Property<recipe_core::BytesPerSecond>,
	pub(crate) second_to_first: Property<recipe_core::BytesPerSecond>,
	pub(crate) first_to_second_lanes: TransferLaneCount,
	pub(crate) second_to_first_lanes: TransferLaneCount,
	pub(crate) asynchronous_submission: bool,
	pub(crate) first_memory_capacity: u64,
	pub(crate) first_memory_rate: u64,
	pub(crate) second_memory_capacity: u64,
	pub(crate) second_memory_rate: u64,
	pub(crate) evidence_session: recipe_core::Label,
	pub(crate) evidence_link: recipe_core::Label,
	pub(crate) evidence_remote_driver: recipe_core::Label,
	pub(crate) evidence_remote_firmware: recipe_core::Label,
	pub(crate) evidence_remote_stable_id: recipe_core::Label,
	pub(crate) evidence_remote_runtime_abi: recipe_core::Label,
	pub(crate) evidence_remote_machine_firmware: recipe_core::Label,
	pub(crate) evidence_remote_memory: recipe_core::Label,
	pub(crate) evidence_local_memory: recipe_core::Label,
	pub(crate) evidence_local_interface: recipe_core::Label,
	pub(crate) evidence_remote_interface: recipe_core::Label,
	pub(crate) benchmark: recipe_probe::BoundedBenchmarkPlan,
	pub(crate) evidence_outbound_rate: Property<recipe_core::BytesPerSecond>,
	pub(crate) evidence_inbound_rate: Property<recipe_core::BytesPerSecond>,
	pub(crate) benchmark_evidence: recipe_probe::PeerBenchmarkEvidence,
}

#[derive(Clone, Debug, Default)]
struct IdAllocator {
	next_machine: u64,
	next_node: u64,
	next_device: u64,
	next_link: u64,
	next_transport: u64,
	next_resource: u64,
}

impl IdAllocator {
	fn initialized() -> Self {
		Self {
			next_machine: 1,
			next_node: 1,
			next_device: 1,
			next_link: 1,
			next_transport: 1,
			next_resource: 1,
		}
	}

	fn machine(&mut self) -> ClusterResult<MachineId> {
		Ok(MachineId::new(take_id(&mut self.next_machine, "machine")?))
	}

	fn node(&mut self) -> ClusterResult<NodeId> {
		Ok(NodeId::new(take_id(&mut self.next_node, "node")?))
	}

	fn device(&mut self) -> ClusterResult<DeviceId> {
		Ok(DeviceId::new(take_id(&mut self.next_device, "device")?))
	}

	fn link(&mut self) -> ClusterResult<LinkId> {
		Ok(LinkId::new(take_id(&mut self.next_link, "link")?))
	}

	fn transport(&mut self) -> ClusterResult<TransportId> {
		Ok(TransportId::new(take_id(
			&mut self.next_transport,
			"transport",
		)?))
	}

	fn resource(&mut self) -> ClusterResult<DuplexResourceId> {
		Ok(DuplexResourceId::new(take_id(
			&mut self.next_resource,
			"duplex resource",
		)?))
	}
}

fn take_id(next: &mut u64, kind: &'static str) -> ClusterResult<u64> {
	let result = *next;
	*next = next
		.checked_add(1)
		.ok_or(ClusterError::IdentityExhausted(kind))?;
	Ok(result)
}

#[derive(Clone, Debug, Default)]
struct MemberRemap {
	machines: BTreeMap<MachineId, MachineId>,
	nodes: BTreeMap<NodeId, NodeId>,
	devices: BTreeMap<DeviceId, DeviceId>,
	links: BTreeMap<LinkId, LinkId>,
	transports: BTreeMap<TransportId, TransportId>,
	resources: BTreeMap<DuplexResourceId, DuplexResourceId>,
}

/// Assemble a complete measured cluster profile. Input order has no effect on
/// IDs or any resulting identity.
pub fn assemble_cluster(
	configuration: &ClusterConfiguration,
	submissions: Vec<SubmittedProfile>,
	network_measurements: Vec<MeasuredNetworkPair>,
) -> ClusterResult<MeasuredProfile> {
	let members = prepare_members(configuration, submissions)?;
	let benchmarks = common_benchmarks(&members)?;
	let network = resolve_network(configuration, &members, network_measurements, benchmarks)?;
	let topology_identity = TopologyIdentity::new(cluster_digest(
		"recipe-cluster-topology-v3",
		configuration,
		&members,
		&network,
		benchmarks,
	)?);
	let discovery_identity = DiscoveryIdentity::new(cluster_digest(
		"recipe-cluster-discovery-v3",
		configuration,
		&members,
		&network,
		benchmarks,
	)?);
	let profile_identity = cluster_digest(
		"recipe-cluster-profile-v3",
		configuration,
		&members,
		&network,
		benchmarks,
	)?;
	let profile = remap_profile(
		configuration,
		&members,
		&network,
		benchmarks,
		topology_identity,
		discovery_identity,
		CacheIdentity {
			schema: PROFILE_SCHEMA,
			digest: profile_identity,
		},
	)?;
	MeasuredProfileCodec::encode(&profile).map_err(|error| ClusterError::InvalidClusterProfile(error.to_string()))?;
	Ok(profile)
}

fn prepare_members(
	configuration: &ClusterConfiguration,
	submissions: Vec<SubmittedProfile>,
) -> ClusterResult<Vec<PreparedMember>> {
	let mut indexed = index_submissions(submissions)?;
	let mut members = Vec::with_capacity(configuration.members().len());
	let mut profile_identities = BTreeSet::new();
	let mut machine_names = BTreeSet::new();
	for spec in configuration.members() {
		let profile = indexed
			.remove(spec.key())
			.ok_or_else(|| ClusterError::MissingMember(spec.key().to_string()))?;
		let identity = MemberProfileIdentity::derive(&profile).map_err(|error| ClusterError::InvalidMember {
			member: spec.key().to_string(),
			message: error.to_string(),
		})?;
		ensure(
			identity == spec.expected(),
			ClusterError::StaleMember(spec.key().to_string()),
		)?;
		ensure(
			profile_identities.insert(identity.endpoint().profile()),
			ClusterError::DuplicateMember(spec.key().to_string()),
		)?;
		let machine_name = profile
			.topology
			.machines
			.first()
			.ok_or_else(|| ClusterError::InvalidMember {
				member: spec.key().to_string(),
				message: "profile contains no machine".to_owned(),
			})?
			.name
			.clone();
		ensure(
			machine_names.insert(machine_name.clone()),
			ClusterError::InvalidMember {
				member: spec.key().to_string(),
				message: format!("machine name {machine_name} is duplicated in cluster membership"),
			},
		)?;
		members.push(PreparedMember {
			key: spec.key().clone(),
			identity,
			profile,
		});
	}
	match indexed.first_key_value() {
		Some((key, _)) => Err(ClusterError::InvalidMember {
			member: key.to_string(),
			message: "profile was submitted by an unconfigured member".to_owned(),
		}),
		None => Ok(()),
	}?;
	Ok(members)
}

fn common_benchmarks(members: &[PreparedMember]) -> ClusterResult<BenchmarkMetadata> {
	let benchmarks = members
		.first()
		.ok_or_else(|| ClusterError::InvalidConfiguration("membership is empty".to_owned()))?
		.profile
		.benchmarks;
	for member in members.iter().skip(1) {
		ensure(
			member.profile.benchmarks == benchmarks,
			ClusterError::InvalidMember {
				member: member.key.to_string(),
				message: "benchmark metadata differs from the other cluster members".to_owned(),
			},
		)?;
	}
	Ok(benchmarks)
}

fn resolve_network(
	configuration: &ClusterConfiguration,
	members: &[PreparedMember],
	measurements: Vec<MeasuredNetworkPair>,
	benchmarks: BenchmarkMetadata,
) -> ClusterResult<Vec<ResolvedNetworkPair>> {
	let mut indexed = BTreeMap::new();
	for measurement in measurements {
		let key = measurement.pair_key()?;
		match indexed.insert(key.clone(), measurement) {
			Some(_) => Err(ClusterError::DuplicateNetworkMeasurement(key.display())),
			None => Ok(()),
		}?;
	}
	let member_index = members
		.iter()
		.map(|member| (member.key.clone(), member))
		.collect::<BTreeMap<_, _>>();
	let mut result = Vec::with_capacity(configuration.network_pairs().len());
	for spec in configuration.network_pairs() {
		let key = PairKey::new(spec.first().clone(), spec.second().clone())?;
		let measurement = indexed
			.remove(&key)
			.ok_or_else(|| ClusterError::MissingNetworkMeasurement(key.display()))?;
		result.push(resolve_pair(&member_index, key, measurement, benchmarks)?);
	}
	match indexed.first_key_value() {
		Some((key, _)) => Err(ClusterError::InvalidNetworkMeasurement(format!(
			"unconfigured pair {} supplied probe evidence",
			key.display()
		))),
		None => Ok(()),
	}?;
	Ok(result)
}

fn resolve_pair(
	members: &BTreeMap<recipe_core::Label, &PreparedMember>,
	key: PairKey,
	evidence: MeasuredNetworkPair,
	benchmarks: BenchmarkMetadata,
) -> ClusterResult<ResolvedNetworkPair> {
	let local = members
		.get(&evidence.local_member)
		.copied()
		.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement(format!("unknown local member {}", evidence.local_member))
		})?;
	let remote = members
		.get(&evidence.remote_member)
		.copied()
		.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement(format!("unknown remote member {}", evidence.remote_member))
		})?;
	ensure(
		evidence.session.local() == local.identity.endpoint()
			&& evidence.session.remote() == remote.identity.endpoint(),
		ClusterError::InvalidNetworkMeasurement(format!(
			"session identities do not match members {} and {}",
			local.key, remote.key
		)),
	)?;
	ensure(
		evidence.benchmark == benchmarks.network,
		ClusterError::InvalidNetworkMeasurement(format!(
			"pair {} used a stale or inconsistent benchmark plan",
			key.display()
		)),
	)?;
	let remote_machine_origin = remote.profile.origins.machines.first().ok_or_else(|| {
		ClusterError::InvalidNetworkMeasurement("remote profile has no machine origin".to_owned())
	})?;
	ensure(
		evidence.descriptor.machine == remote_machine_origin.fingerprint,
		ClusterError::InvalidNetworkMeasurement(format!(
			"peer descriptor fingerprint does not match member {}",
			remote.key
		)),
	)?;
	let local_device = resolve_gateway(
		local,
		&evidence.descriptor.local_memory_key,
		evidence.local_memory.capacity.get(),
		evidence.local_memory.transfer_rate.get(),
		"local",
	)?;
	let remote_device = resolve_gateway(
		remote,
		&evidence.descriptor.remote_memory_key,
		evidence.measurement.remote_memory_capacity.value.get(),
		evidence.measurement.remote_memory_rate.value.get(),
		"remote",
	)?;
	let outbound = evidence.measurement.outbound_rate.ok_or_else(|| {
		ClusterError::InvalidNetworkMeasurement("outbound throughput measurement is missing".to_owned())
	})?;
	let inbound = evidence.measurement.inbound_rate.ok_or_else(|| {
		ClusterError::InvalidNetworkMeasurement("inbound throughput measurement is missing".to_owned())
	})?;
	let duplex = match evidence.descriptor.duplex {
		LinkDuplex::Full => DuplexMode::Full,
		LinkDuplex::Half => DuplexMode::Half,
	};
	let local_memory = (
		evidence.local_memory.capacity.get(),
		evidence.local_memory.transfer_rate.get(),
	);
	let remote_memory = (
		evidence.measurement.remote_memory_capacity.value.get(),
		evidence.measurement.remote_memory_rate.value.get(),
	);
	let local_is_first = evidence.local_member == key.first;
	let (
		first_device,
		second_device,
		first_to_second,
		second_to_first,
		first_to_second_lanes,
		second_to_first_lanes,
		first_memory,
		second_memory,
	) = match local_is_first {
		true => (
			local_device,
			remote_device,
			outbound,
			inbound,
			evidence.descriptor.outbound_maximum_inflight,
			evidence.descriptor.inbound_maximum_inflight,
			local_memory,
			remote_memory,
		),
		false => (
			remote_device,
			local_device,
			inbound,
			outbound,
			evidence.descriptor.inbound_maximum_inflight,
			evidence.descriptor.outbound_maximum_inflight,
			remote_memory,
			local_memory,
		),
	};
	Ok(ResolvedNetworkPair {
		key,
		first_device,
		second_device,
		kind: evidence.descriptor.transport_kind,
		duplex,
		first_to_second,
		second_to_first,
		first_to_second_lanes,
		second_to_first_lanes,
		asynchronous_submission: evidence.descriptor.asynchronous_submission,
		first_memory_capacity: first_memory.0,
		first_memory_rate: first_memory.1,
		second_memory_capacity: second_memory.0,
		second_memory_rate: second_memory.1,
		evidence_session: evidence.descriptor.session_id,
		evidence_link: evidence.descriptor.link_identity,
		evidence_remote_driver: evidence.descriptor.remote_driver,
		evidence_remote_firmware: evidence.descriptor.remote_firmware,
		evidence_remote_stable_id: evidence.descriptor.machine.stable_id,
		evidence_remote_runtime_abi: evidence.descriptor.machine.runtime_abi,
		evidence_remote_machine_firmware: evidence.descriptor.machine.firmware,
		evidence_remote_memory: evidence.descriptor.remote_memory_key,
		evidence_local_memory: evidence.descriptor.local_memory_key,
		evidence_local_interface: evidence.descriptor.local_interface_key,
		evidence_remote_interface: evidence.descriptor.remote_interface_identity,
		benchmark: evidence.benchmark,
		evidence_outbound_rate: outbound,
		evidence_inbound_rate: inbound,
		benchmark_evidence: evidence.measurement.evidence,
	})
}

fn resolve_gateway(
	member: &PreparedMember,
	key: &recipe_core::Label,
	capacity: u64,
	transfer_rate: u64,
	side: &str,
) -> ClusterResult<DeviceId> {
	let origin = member
		.profile
		.origins
		.ram
		.iter()
		.find(|origin| &origin.key == key)
		.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement(format!(
				"{side} RAM origin key {key} is absent from member {}",
				member.key
			))
		})?;
	let device = member
		.profile
		.topology
		.device(origin.device)
		.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement(format!(
				"{side} RAM origin {key} references a missing device in member {}",
				member.key
			))
		})?;
	ensure(
		device.capacity.value.get() == capacity && device.transfer_rate.value.get() == transfer_rate,
		ClusterError::InvalidNetworkMeasurement(format!(
			"{side} RAM measurement does not match exact origin {key} in member {}",
			member.key
		)),
	)?;
	Ok(device.id)
}

#[derive(Clone, Debug, Default)]
struct ClusterCollections {
	machines: Vec<Machine>,
	origin_machines: Vec<MeasuredMachineOrigin>,
	nodes: Vec<Node>,
	devices: Vec<Device>,
	origin_ram: Vec<MeasuredRamOrigin>,
	origin_storage: Vec<MeasuredStorageOrigin>,
	origin_gpu: Vec<MeasuredGpuOrigin>,
	links: Vec<DirectedLink>,
	discovered_devices: Vec<DiscoveredDevice>,
	discovered_links: Vec<DiscoveredLink>,
}

fn remap_profile(
	configuration: &ClusterConfiguration,
	members: &[PreparedMember],
	network: &[ResolvedNetworkPair],
	benchmarks: BenchmarkMetadata,
	topology_identity: TopologyIdentity,
	discovery_identity: DiscoveryIdentity,
	cache_identity: CacheIdentity,
) -> ClusterResult<MeasuredProfile> {
	let mut allocator = IdAllocator::initialized();
	let mut mappings = BTreeMap::new();
	for member in members {
		mappings.insert(member.key.clone(), allocate_member(member, &mut allocator)?);
	}

	let mut collected = ClusterCollections::default();
	for member in members {
		let mapping = mappings
			.get(&member.key)
			.ok_or_else(|| ClusterError::InvalidClusterProfile("member remap is missing".to_owned()))?;
		remap_member(member, mapping, configuration.master(), &mut collected)?;
	}
	for pair in network {
		let first_mapping = mappings.get(&pair.key.first).ok_or_else(|| {
			ClusterError::InvalidClusterProfile(format!("member {} has no ID remap", pair.key.first))
		})?;
		let second_mapping = mappings.get(&pair.key.second).ok_or_else(|| {
			ClusterError::InvalidClusterProfile(format!("member {} has no ID remap", pair.key.second))
		})?;
		let from = mapped(
			&first_mapping.devices,
			pair.first_device,
			"network source device",
		)?;
		let to = mapped(
			&second_mapping.devices,
			pair.second_device,
			"network destination device",
		)?;
		push_network_pair(
			pair,
			from,
			to,
			&mut allocator,
			&mut collected.links,
			&mut collected.discovered_links,
		)?;
	}

	let topology = Topology {
		identity: topology_identity,
		machines: collected.machines,
		nodes: collected.nodes,
		devices: collected.devices,
		links: collected.links,
	};
	let discovery = DiscoveryProfile {
		identity: discovery_identity,
		topology: topology_identity,
		devices: collected.discovered_devices,
		links: collected.discovered_links,
	};
	let mut peer_benchmarks = network
		.iter()
		.map(|pair| MeasuredPeerBenchmark {
			session_id: pair.evidence_session.clone(),
			outbound_rate: pair.evidence_outbound_rate,
			inbound_rate: pair.evidence_inbound_rate,
			evidence: pair.benchmark_evidence,
		})
		.collect::<Vec<_>>();
	peer_benchmarks.sort_by(|left, right| left.session_id.cmp(&right.session_id));
	Ok(MeasuredProfile {
		schema: PROFILE_SCHEMA,
		cache_identity,
		origins: MeasuredOrigins {
			machines: collected.origin_machines,
			ram: collected.origin_ram,
			storage: collected.origin_storage,
			gpu: collected.origin_gpu,
		},
		benchmarks,
		peer_benchmarks,
		topology,
		discovery,
	})
}

fn allocate_member(member: &PreparedMember, allocator: &mut IdAllocator) -> ClusterResult<MemberRemap> {
	let mut result = MemberRemap::default();
	for machine in &member.profile.topology.machines {
		result.machines.insert(machine.id, allocator.machine()?);
	}
	for node in &member.profile.topology.nodes {
		result.nodes.insert(node.id, allocator.node()?);
	}
	for device in &member.profile.topology.devices {
		result.devices.insert(device.id, allocator.device()?);
	}
	for link in &member.profile.topology.links {
		result.links.insert(link.id, allocator.link()?);
	}
	let transports = member
		.profile
		.topology
		.links
		.iter()
		.map(|link| link.transport)
		.collect::<BTreeSet<_>>();
	for transport in transports {
		result.transports.insert(transport, allocator.transport()?);
	}
	let resources = member
		.profile
		.topology
		.links
		.iter()
		.map(|link| link.capacity_resource)
		.collect::<BTreeSet<_>>();
	for resource in resources {
		result.resources.insert(resource, allocator.resource()?);
	}
	Ok(result)
}

fn remap_member(
	member: &PreparedMember,
	mapping: &MemberRemap,
	master: &recipe_core::Label,
	collected: &mut ClusterCollections,
) -> ClusterResult<()> {
	for machine in &member.profile.topology.machines {
		collected.machines.push(Machine {
			id: mapped(&mapping.machines, machine.id, "machine")?,
			name: machine.name.clone(),
		});
	}
	for origin in &member.profile.origins.machines {
		collected.origin_machines.push(MeasuredMachineOrigin {
			machine: mapped(&mapping.machines, origin.machine, "machine origin")?,
			fingerprint: origin.fingerprint.clone(),
		});
	}
	for node in &member.profile.topology.nodes {
		let role = match (&member.key == master, node.role) {
			(true, NodeRole::Master) => NodeRole::Master,
			(true, NodeRole::Worker) | (false, NodeRole::Master | NodeRole::Worker) => NodeRole::Worker,
		};
		collected.nodes.push(Node {
			id: mapped(&mapping.nodes, node.id, "node")?,
			role,
			machine: mapped(&mapping.machines, node.machine, "node machine")?,
			devices: node
				.devices
				.iter()
				.map(|device| mapped(&mapping.devices, *device, "node device"))
				.collect::<ClusterResult<Vec<_>>>()?,
		});
	}
	for device in &member.profile.topology.devices {
		collected.devices.push(Device {
			id: mapped(&mapping.devices, device.id, "device")?,
			machine: mapped(&mapping.machines, device.machine, "device machine")?,
			kind: device.kind,
			capacity: device.capacity,
			transfer_rate: device.transfer_rate,
			calculation_rate: device.calculation_rate,
		});
	}
	for origin in &member.profile.origins.ram {
		collected.origin_ram.push(MeasuredRamOrigin {
			device: mapped(&mapping.devices, origin.device, "RAM origin")?,
			key: origin.key.clone(),
		});
	}
	for origin in &member.profile.origins.storage {
		collected.origin_storage.push(MeasuredStorageOrigin {
			device: mapped(&mapping.devices, origin.device, "storage origin")?,
			key: origin.key.clone(),
		});
	}
	for origin in &member.profile.origins.gpu {
		collected.origin_gpu.push(MeasuredGpuOrigin {
			device: mapped(&mapping.devices, origin.device, "GPU origin")?,
			key: origin.key.clone(),
		});
	}
	for link in &member.profile.topology.links {
		collected.links.push(DirectedLink {
			id: mapped(&mapping.links, link.id, "link")?,
			transport: mapped(&mapping.transports, link.transport, "transport")?,
			kind: link.kind,
			duplex: link.duplex,
			from: mapped(&mapping.devices, link.from, "link source")?,
			to: mapped(&mapping.devices, link.to, "link destination")?,
			bandwidth: link.bandwidth,
			maximum_inflight_transfers: link.maximum_inflight_transfers,
			capacity_resource: mapped(
				&mapping.resources,
				link.capacity_resource,
				"capacity resource",
			)?,
		});
	}
	for discovered in &member.profile.discovery.devices {
		collected.discovered_devices.push(DiscoveredDevice {
			device: mapped(&mapping.devices, discovered.device, "discovered device")?,
			available: discovered.available,
			maximum_submission_queues: discovered.maximum_submission_queues,
			total_capacity: discovered.total_capacity,
			transfer: discovered.transfer,
			calculation: discovered.calculation.clone(),
		});
	}
	for discovered in &member.profile.discovery.links {
		collected.discovered_links.push(DiscoveredLink {
			link: mapped(&mapping.links, discovered.link, "discovered link")?,
			available: discovered.available,
			bandwidth: discovered.bandwidth,
			maximum_inflight_transfers: discovered.maximum_inflight_transfers,
			asynchronous_submission: discovered.asynchronous_submission,
		});
	}
	Ok(())
}

fn mapped<K: Ord + Copy, V: Copy>(mapping: &BTreeMap<K, V>, old: K, kind: &str) -> ClusterResult<V> {
	mapping
		.get(&old)
		.copied()
		.ok_or_else(|| ClusterError::InvalidClusterProfile(format!("{kind} remap is missing")))
}

fn push_network_pair(
	pair: &ResolvedNetworkPair,
	from: DeviceId,
	to: DeviceId,
	allocator: &mut IdAllocator,
	links: &mut Vec<DirectedLink>,
	discovered_links: &mut Vec<DiscoveredLink>,
) -> ClusterResult<()> {
	let transport = allocator.transport()?;
	let forward_resource = allocator.resource()?;
	let reverse_resource = match pair.duplex {
		DuplexMode::Full => allocator.resource()?,
		DuplexMode::Half => forward_resource,
	};
	let forward_id = allocator.link()?;
	let reverse_id = allocator.link()?;
	let measured_forward_lanes = Property::new(pair.first_to_second_lanes, PropertyProvenance::Measured);
	let measured_reverse_lanes = Property::new(pair.second_to_first_lanes, PropertyProvenance::Measured);
	links.push(DirectedLink {
		id: forward_id,
		transport,
		kind: pair.kind,
		duplex: pair.duplex,
		from,
		to,
		bandwidth: pair.first_to_second,
		maximum_inflight_transfers: measured_forward_lanes,
		capacity_resource: forward_resource,
	});
	links.push(DirectedLink {
		id: reverse_id,
		transport,
		kind: pair.kind,
		duplex: pair.duplex,
		from: to,
		to: from,
		bandwidth: pair.second_to_first,
		maximum_inflight_transfers: measured_reverse_lanes,
		capacity_resource: reverse_resource,
	});
	discovered_links.push(DiscoveredLink {
		link: forward_id,
		available: true,
		bandwidth: pair.first_to_second,
		maximum_inflight_transfers: measured_forward_lanes,
		asynchronous_submission: pair.asynchronous_submission,
	});
	discovered_links.push(DiscoveredLink {
		link: reverse_id,
		available: true,
		bandwidth: pair.second_to_first,
		maximum_inflight_transfers: measured_reverse_lanes,
		asynchronous_submission: pair.asynchronous_submission,
	});
	Ok(())
}

/// Cluster-specific facade over the canonical measured-profile codec.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClusterProfileCodec;

impl ClusterProfileCodec {
	pub fn encode(profile: &MeasuredProfile) -> ClusterResult<Vec<u8>> {
		validate_cluster_shape(profile)?;
		MeasuredProfileCodec::encode(profile).map_err(|error| ClusterError::Codec(error.to_string()))
	}

	pub fn decode(bytes: &[u8]) -> ClusterResult<MeasuredProfile> {
		let profile =
			MeasuredProfileCodec::decode(bytes).map_err(|error| ClusterError::Codec(error.to_string()))?;
		validate_cluster_shape(&profile)?;
		Ok(profile)
	}
}

fn validate_cluster_shape(profile: &MeasuredProfile) -> ClusterResult<()> {
	ensure(
		profile.topology.machines.len() >= 2,
		ClusterError::InvalidClusterProfile("cluster profile contains fewer than two machines".to_owned()),
	)?;
	let machine_by_device = profile
		.topology
		.devices
		.iter()
		.map(|device| (device.id, device.machine))
		.collect::<BTreeMap<_, _>>();
	let mut adjacency = profile
		.topology
		.machines
		.iter()
		.map(|machine| (machine.id, BTreeSet::new()))
		.collect::<BTreeMap<_, _>>();
	for link in &profile.topology.links {
		let from = machine_by_device.get(&link.from);
		let to = machine_by_device.get(&link.to);
		match (from, to) {
			(Some(from), Some(to)) => match from == to {
				true => Ok(()),
				false => {
					adjacency.entry(*from).or_default().insert(*to);
					adjacency.entry(*to).or_default().insert(*from);
					Ok(())
				}
			},
			(Some(_), None) | (None, Some(_)) | (None, None) => Err(ClusterError::InvalidClusterProfile(
				"link references an unknown device".to_owned(),
			)),
		}?;
	}
	let first = profile
		.topology
		.machines
		.first()
		.ok_or_else(|| ClusterError::InvalidClusterProfile("cluster contains no machine".to_owned()))?
		.id;
	let mut reached = BTreeSet::from([first]);
	for _ in 0..profile.topology.machines.len() {
		let newly_reached = reached
			.iter()
			.flat_map(|machine| adjacency.get(machine).into_iter().flatten())
			.copied()
			.filter(|machine| !reached.contains(machine))
			.collect::<BTreeSet<_>>();
		reached.extend(newly_reached);
	}
	ensure(
		reached.len() == profile.topology.machines.len(),
		ClusterError::InvalidClusterProfile("inter-machine transport graph is disconnected".to_owned()),
	)
}

fn ensure(condition: bool, error: ClusterError) -> ClusterResult<()> {
	match condition {
		true => Ok(()),
		false => Err(error),
	}
}
