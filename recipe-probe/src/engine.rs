use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use recipe_core::{
	ByteCount, BytesPerSecond, CalculationCapability, Device, DeviceId, DeviceKind, DirectedLink, DiscoveredDevice,
	DiscoveredLink, DiscoveryIdentity, DiscoveryProfile, DuplexMode, DuplexResourceId, Label, LinkId, Machine,
	MachineId, Node, NodeId, NodeRole, Property, PropertyProvenance, Topology, TopologyIdentity, TransferCapability,
	TransferLaneCount, TransportId, TransportKind,
};

use crate::error::{ProbeError, ProbeResult};
use crate::hash::CanonicalDigest;
use crate::model::{
	BenchmarkMetadata, BoundedBenchmarkPlan, CacheIdentity, GpuBenchmarkIo, GpuDescriptor, GpuDiscovery,
	GpuMeasurement, HostBenchmarkIo, HostDiscovery, HostInventory, LinkDuplex, MeasuredGpuOrigin,
	MeasuredMachineOrigin, MeasuredOrigins, MeasuredPeerBenchmark, MeasuredProfile, MeasuredRamOrigin,
	MeasuredStorageOrigin, PEER_BENCHMARK_PROTOCOL_SCHEMA, PROFILE_SCHEMA, PeerDescriptor, PeerDuplexExecution,
	PeerMeasurement, PeerSession, RamMeasurement, StorageMeasurement,
};
use crate::seed::{IdentityFacet, SeedContract, SeedDuplex};

const MAXIMUM_BUFFER_BYTES: u64 = 64 * 1024 * 1024;
const MINIMUM_BUFFER_BYTES: u64 = 4 * 1024;
const DEFAULT_ITERATIONS: u32 = 8;
const MAXIMUM_BENCHMARK_DURATION: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct ProbeEngine<'a> {
	host_discovery: &'a dyn HostDiscovery,
	gpu_discovery: &'a dyn GpuDiscovery,
	host_benchmarks: &'a dyn HostBenchmarkIo,
	gpu_benchmarks: &'a dyn GpuBenchmarkIo,
}

#[derive(Debug)]
struct Inspection<'a> {
	host: HostInventory,
	gpus: Vec<GpuDescriptor>,
	peers: Vec<(&'a dyn PeerSession, PeerDescriptor)>,
	cache_identity: CacheIdentity,
}

impl<'a> ProbeEngine<'a> {
	#[must_use]
	pub const fn new(
		host_discovery: &'a dyn HostDiscovery,
		gpu_discovery: &'a dyn GpuDiscovery,
		host_benchmarks: &'a dyn HostBenchmarkIo,
		gpu_benchmarks: &'a dyn GpuBenchmarkIo,
	) -> Self {
		Self {
			host_discovery,
			gpu_discovery,
			host_benchmarks,
			gpu_benchmarks,
		}
	}

	pub fn probe(&self, seed: &SeedContract, peer_sessions: &[&dyn PeerSession]) -> ProbeResult<MeasuredProfile> {
		let Inspection {
			host,
			gpus,
			peers,
			cache_identity,
		} = self.inspect(seed, peer_sessions)?;
		let plans = BenchmarkPlans::from_seed(seed);

		let mut ram_measurements = BTreeMap::new();
		for domain in &host.ram {
			let measurement = self.host_benchmarks.benchmark_ram(domain, plans.ram)?;
			validate_ram_measurement(&domain.key, measurement)?;
			ram_measurements.insert(domain.key.clone(), measurement);
		}
		let mut storage_measurements = BTreeMap::new();
		for domain in &host.storage {
			let measurement = self
				.host_benchmarks
				.benchmark_storage(domain, plans.storage)?;
			validate_storage_measurement(&domain.key, measurement)?;
			storage_measurements.insert(domain.key.clone(), measurement);
		}
		let mut gpu_measurements = BTreeMap::new();
		for device in &gpus {
			let measurement = self.gpu_benchmarks.benchmark_gpu(device, plans.gpu)?;
			validate_gpu_measurement(&device.key, measurement)?;
			gpu_measurements.insert(device.key.clone(), measurement);
		}
		let mut peer_measurements = Vec::new();
		for (session, descriptor) in peers {
			let measurement = session.benchmark(plans.network)?;
			validate_peer_measurement(&descriptor, measurement, plans.network)?;
			peer_measurements.push((descriptor, measurement));
		}

		let measurements = Measurements {
			ram: ram_measurements,
			storage: storage_measurements,
			gpu: gpu_measurements,
			peers: peer_measurements,
		};
		let topology_digest = build_profile_digest("recipe-topology-v5", cache_identity, &measurements);
		let topology_identity = TopologyIdentity::new(topology_digest);
		let topology = build_topology(topology_identity, &host, &gpus, &measurements)?;
		let origins = build_origins(&host, &gpus, &measurements);
		let discovery_digest = build_profile_digest("recipe-discovery-v5", cache_identity, &measurements);
		let discovery = build_discovery(
			DiscoveryIdentity::new(discovery_digest),
			&topology,
			&host,
			&gpus,
			&measurements,
		)?;

		topology
			.validate()
			.map_err(|errors| ProbeError::InvalidProfile(format!("topology validation failed: {errors}")))?;
		topology
			.validate_scheduling_properties()
			.map_err(|errors| {
				ProbeError::InvalidProfile(format!("topology scheduling validation failed: {errors}"))
			})?;
		discovery
			.validate(&topology)
			.map_err(|errors| ProbeError::InvalidProfile(format!("discovery validation failed: {errors}")))?;

		let profile = MeasuredProfile {
			schema: PROFILE_SCHEMA,
			cache_identity,
			origins,
			benchmarks: plans.metadata(seed.schema),
			peer_benchmarks: measurements
				.peers
				.iter()
				.map(|(descriptor, measurement)| MeasuredPeerBenchmark {
					session_id: descriptor.session_id.clone(),
					outbound_rate: measurement
						.outbound_rate
						.expect("peer measurement was validated"),
					inbound_rate: measurement
						.inbound_rate
						.expect("peer measurement was validated"),
					evidence: measurement.evidence,
				})
				.collect(),
			topology,
			discovery,
		};
		crate::codec::validate_profile(&profile).map_err(|error| {
			ProbeError::InvalidProfile(format!("constructed profile validation failed: {error}"))
		})?;
		Ok(profile)
	}

	fn inspect<'b>(
		&self,
		seed: &SeedContract,
		peer_sessions: &'b [&'b dyn PeerSession],
	) -> ProbeResult<Inspection<'b>> {
		let mut host = self.host_discovery.discover_host()?;
		normalize_host(&mut host)?;
		let mut gpu_inventory = self.gpu_discovery.discover_all()?;
		if !gpu_inventory.exhaustive {
			return Err(ProbeError::IncompleteGpuEnumeration);
		}
		if gpu_inventory.devices.is_empty() {
			return Err(ProbeError::Discovery(
				"no GPU calculation device was discovered".to_owned(),
			));
		}
		gpu_inventory
			.devices
			.sort_by(|left, right| left.key.cmp(&right.key));
		validate_gpu_inventory(&host, &gpu_inventory.devices)?;

		let mut peers = peer_sessions
			.iter()
			.map(|session| {
				session
					.descriptor()
					.map(|descriptor| (*session, descriptor))
			})
			.collect::<ProbeResult<Vec<_>>>()?;
		peers.sort_by(|left, right| left.1.session_id.cmp(&right.1.session_id));
		validate_peer_descriptors(&host, &peers)?;
		let cache_identity = build_cache_identity(seed, &host, &gpu_inventory.devices, &peers);
		Ok(Inspection {
			host,
			gpus: gpu_inventory.devices,
			peers,
			cache_identity,
		})
	}

	/// Computes the exact current cache key using discovery only.
	pub fn current_cache_identity(
		&self,
		seed: &SeedContract,
		peer_sessions: &[&dyn PeerSession],
	) -> ProbeResult<CacheIdentity> {
		Ok(self.inspect(seed, peer_sessions)?.cache_identity)
	}

	/// Returns an exact cached profile when valid, otherwise probes and stores a
	/// new one through the explicitly supplied cache.
	pub fn load_or_probe_and_store(
		&self,
		seed: &SeedContract,
		peer_sessions: &[&dyn PeerSession],
		cache: &dyn crate::model::ProfileCache,
	) -> ProbeResult<MeasuredProfile> {
		let identity = self.current_cache_identity(seed, peer_sessions)?;
		if let Some(profile) = cache.load(identity)? {
			return Ok(profile);
		}
		self.probe_and_store(seed, peer_sessions, cache)
	}

	/// Runs a fresh probe and persists only the fully validated measured profile
	/// through an explicitly supplied cache implementation.
	pub fn probe_and_store(
		&self,
		seed: &SeedContract,
		peer_sessions: &[&dyn PeerSession],
		cache: &dyn crate::model::ProfileCache,
	) -> ProbeResult<MeasuredProfile> {
		let profile = self.probe(seed, peer_sessions)?;
		cache.store(&profile)?;
		Ok(profile)
	}
}

#[derive(Clone, Copy, Debug)]
struct BenchmarkPlans {
	ram: BoundedBenchmarkPlan,
	storage: BoundedBenchmarkPlan,
	gpu: BoundedBenchmarkPlan,
	network: BoundedBenchmarkPlan,
}

impl BenchmarkPlans {
	fn from_seed(seed: &SeedContract) -> Self {
		Self {
			ram: bounded_plan(seed.estimates.ram_capacity.get() / 1024),
			storage: bounded_plan(seed.estimates.disk_capacity.get() / 16_384),
			gpu: bounded_plan(seed.estimates.gpu_memory_capacity.get() / 1024),
			network: bounded_plan(seed.estimates.ethernet_rate.get() / 8),
		}
	}

	const fn metadata(self, seed_schema: u32) -> BenchmarkMetadata {
		BenchmarkMetadata {
			seed_schema,
			ram: self.ram,
			storage: self.storage,
			gpu: self.gpu,
			network: self.network,
		}
	}
}

fn bounded_plan(suggested_bytes: u64) -> BoundedBenchmarkPlan {
	BoundedBenchmarkPlan {
		buffer_bytes: ByteCount::new(suggested_bytes.clamp(MINIMUM_BUFFER_BYTES, MAXIMUM_BUFFER_BYTES)),
		iterations: DEFAULT_ITERATIONS,
		maximum_duration: MAXIMUM_BENCHMARK_DURATION,
	}
}

#[derive(Debug)]
struct Measurements {
	ram: BTreeMap<Label, RamMeasurement>,
	storage: BTreeMap<Label, StorageMeasurement>,
	gpu: BTreeMap<Label, GpuMeasurement>,
	peers: Vec<(PeerDescriptor, PeerMeasurement)>,
}

fn normalize_host(host: &mut HostInventory) -> ProbeResult<()> {
	host.ram.sort_by(|left, right| left.key.cmp(&right.key));
	host.storage.sort_by(|left, right| left.key.cmp(&right.key));
	host.network.sort_by(|left, right| left.key.cmp(&right.key));
	if host.ram.is_empty() {
		return Err(ProbeError::Discovery(
			"host discovery returned no memory domains".to_owned(),
		));
	}
	if host.storage.is_empty() {
		return Err(ProbeError::Discovery(
			"host discovery returned no mounted storage domains".to_owned(),
		));
	}
	unique_labels("RAM domain", host.ram.iter().map(|value| &value.key))?;
	unique_labels(
		"storage domain",
		host.storage.iter().map(|value| &value.key),
	)?;
	unique_labels(
		"network interface",
		host.network.iter().map(|value| &value.key),
	)?;
	for storage in &host.storage {
		if !host
			.ram
			.iter()
			.any(|ram| ram.key == storage.host_memory_key)
		{
			return Err(ProbeError::Discovery(format!(
				"storage {} references unknown memory domain {}",
				storage.key, storage.host_memory_key
			)));
		}
		if !storage.asynchronous_submission {
			return Err(ProbeError::Discovery(format!(
				"storage {} has no asynchronous submission path",
				storage.key
			)));
		}
	}
	Ok(())
}

fn validate_gpu_inventory(host: &HostInventory, devices: &[GpuDescriptor]) -> ProbeResult<()> {
	unique_labels("GPU", devices.iter().map(|value| &value.key))?;
	for device in devices {
		if !host.ram.iter().any(|ram| ram.key == device.host_memory_key) {
			return Err(ProbeError::Discovery(format!(
				"GPU {} references unknown memory domain {}",
				device.key, device.host_memory_key
			)));
		}
		if !device.asynchronous_submission {
			return Err(ProbeError::Discovery(format!(
				"GPU {} has no asynchronous submission path",
				device.key
			)));
		}
		if device.maximum_concurrent_tasks == 0 {
			return Err(ProbeError::Discovery(format!(
				"GPU {} reports zero concurrent task capacity",
				device.key
			)));
		}
	}
	Ok(())
}

fn validate_peer_descriptors(host: &HostInventory, peers: &[(&dyn PeerSession, PeerDescriptor)]) -> ProbeResult<()> {
	unique_labels(
		"peer session",
		peers.iter().map(|value| &value.1.session_id),
	)?;
	unique_labels(
		"peer machine",
		peers.iter().map(|value| &value.1.machine.stable_id),
	)?;
	for (_, peer) in peers {
		if peer.machine.stable_id == host.machine.stable_id {
			return Err(ProbeError::Discovery(format!(
				"peer session {} resolves to the local machine",
				peer.session_id
			)));
		}
		if !host.ram.iter().any(|ram| ram.key == peer.local_memory_key) {
			return Err(ProbeError::Discovery(format!(
				"peer {} references unknown local memory domain {}",
				peer.session_id, peer.local_memory_key
			)));
		}
		let interface = host
			.network
			.iter()
			.find(|interface| interface.key == peer.local_interface_key);
		if interface.is_none() {
			return Err(ProbeError::Discovery(format!(
				"peer {} references unknown local interface {}",
				peer.session_id, peer.local_interface_key
			)));
		}
		if let Some(interface) = interface {
			if peer.transport_kind != interface.transport_kind {
				return Err(ProbeError::Discovery(format!(
					"peer {} transport kind {:?} contradicts local interface kind {:?}",
					peer.session_id, peer.transport_kind, interface.transport_kind
				)));
			}
			if peer.duplex != interface.duplex {
				return Err(ProbeError::Discovery(format!(
					"peer {} duplex {:?} contradicts local interface duplex {:?}",
					peer.session_id, peer.duplex, interface.duplex
				)));
			}
		}
		if !peer.asynchronous_submission {
			return Err(ProbeError::Discovery(format!(
				"peer {} has no asynchronous submission path",
				peer.session_id
			)));
		}
	}
	Ok(())
}

fn unique_labels<'a>(kind: &str, values: impl Iterator<Item = &'a Label>) -> ProbeResult<()> {
	let mut seen = BTreeSet::new();
	for value in values {
		if !seen.insert(value.clone()) {
			return Err(ProbeError::Discovery(format!(
				"{kind} identity {value} appears more than once"
			)));
		}
	}
	Ok(())
}

fn validate_ram_measurement(key: &Label, measurement: RamMeasurement) -> ProbeResult<()> {
	require_measured(&measurement.capacity, &format!("RAM {key} capacity"))?;
	require_measured(
		&measurement.transfer_rate,
		&format!("RAM {key} transfer rate"),
	)
}

fn validate_storage_measurement(key: &Label, measurement: StorageMeasurement) -> ProbeResult<()> {
	require_measured(&measurement.capacity, &format!("storage {key} capacity"))?;
	require_measured(&measurement.read_rate, &format!("storage {key} read rate"))?;
	require_measured(
		&measurement.write_rate,
		&format!("storage {key} write rate"),
	)
}

fn validate_gpu_measurement(key: &Label, measurement: GpuMeasurement) -> ProbeResult<()> {
	require_measured(&measurement.capacity, &format!("GPU {key} capacity"))?;
	require_measured(
		&measurement.calculation_rate,
		&format!("GPU {key} calculation rate"),
	)?;
	require_measured(&measurement.memory_rate, &format!("GPU {key} memory rate"))?;
	require_measured(
		&measurement.host_to_device_rate,
		&format!("GPU {key} host-to-device rate"),
	)?;
	require_measured(
		&measurement.device_to_host_rate,
		&format!("GPU {key} device-to-host rate"),
	)
}

fn validate_peer_measurement(
	descriptor: &PeerDescriptor,
	measurement: PeerMeasurement,
	plan: BoundedBenchmarkPlan,
) -> ProbeResult<()> {
	require_measured(
		&measurement.remote_memory_capacity,
		&format!("peer {} remote memory capacity", descriptor.session_id),
	)?;
	require_measured(
		&measurement.remote_memory_rate,
		&format!("peer {} remote memory rate", descriptor.session_id),
	)?;
	let outbound = measurement
		.outbound_rate
		.ok_or_else(|| ProbeError::IncompletePeerMeasurement {
			peer: descriptor.session_id.to_string(),
			direction: "outbound",
		})?;
	let inbound = measurement
		.inbound_rate
		.ok_or_else(|| ProbeError::IncompletePeerMeasurement {
			peer: descriptor.session_id.to_string(),
			direction: "inbound",
		})?;
	require_measured(
		&outbound,
		&format!("peer {} outbound throughput", descriptor.session_id),
	)?;
	require_measured(
		&inbound,
		&format!("peer {} inbound throughput", descriptor.session_id),
	)?;
	let evidence = measurement.evidence;
	if evidence.protocol_schema != PEER_BENCHMARK_PROTOCOL_SCHEMA
		|| evidence.local_endpoint.machine.is_zero()
		|| evidence.local_endpoint.profile.is_zero()
		|| evidence.remote_endpoint.machine.is_zero()
		|| evidence.remote_endpoint.profile.is_zero()
		|| evidence.local_endpoint.machine == evidence.remote_endpoint.machine
	{
		return Err(ProbeError::Benchmark(format!(
			"peer {} supplied invalid authenticated endpoint evidence",
			descriptor.session_id
		)));
	}
	let expected_execution = match descriptor.duplex {
		LinkDuplex::Full => PeerDuplexExecution::Simultaneous,
		LinkDuplex::Half => PeerDuplexExecution::Serialized,
	};
	if evidence.execution != expected_execution {
		return Err(ProbeError::Benchmark(format!(
			"peer {} benchmark execution {:?} contradicts {:?} duplex",
			descriptor.session_id, evidence.execution, descriptor.duplex
		)));
	}
	validate_directional_evidence(
		descriptor,
		"outbound",
		evidence.outbound,
		outbound.value,
		plan,
	)?;
	validate_directional_evidence(descriptor, "inbound", evidence.inbound, inbound.value, plan)
}

fn validate_directional_evidence(
	descriptor: &PeerDescriptor,
	direction: &str,
	evidence: crate::model::DirectionalBenchmarkEvidence,
	rate: BytesPerSecond,
	plan: BoundedBenchmarkPlan,
) -> ProbeResult<()> {
	let total_bytes = plan
		.buffer_bytes
		.get()
		.checked_mul(u64::from(plan.iterations))
		.ok_or_else(|| ProbeError::Benchmark("peer benchmark byte count overflowed".to_owned()))?;
	let maximum_duration = u64::try_from(plan.maximum_duration.as_nanos())
		.map_err(|_| ProbeError::Benchmark("peer benchmark duration exceeds u64 nanoseconds".to_owned()))?;
	let derived_rate = u128::from(total_bytes)
		.checked_mul(1_000_000_000)
		.and_then(|bytes| bytes.checked_div(u128::from(evidence.elapsed_nanoseconds.max(1))))
		.ok_or_else(|| ProbeError::Benchmark("peer benchmark rate calculation overflowed".to_owned()))?
		.clamp(1, u128::from(u64::MAX));
	let valid = evidence.total_bytes.get() == total_bytes
		&& evidence.elapsed_nanoseconds != 0
		&& evidence.elapsed_nanoseconds <= maximum_duration
		&& evidence.sample_count == plan.iterations
		&& evidence.minimum_sample_nanoseconds != 0
		&& evidence.minimum_sample_nanoseconds <= evidence.mean_sample_nanoseconds
		&& evidence.mean_sample_nanoseconds <= evidence.maximum_sample_nanoseconds
		&& evidence.maximum_sample_nanoseconds <= evidence.elapsed_nanoseconds
		&& u128::from(rate.get()) == derived_rate;
	if valid {
		Ok(())
	} else {
		Err(ProbeError::Benchmark(format!(
			"peer {} {direction} benchmark evidence is inconsistent",
			descriptor.session_id
		)))
	}
}

fn require_measured<T>(property: &Property<T>, name: &str) -> ProbeResult<()> {
	if property.provenance == PropertyProvenance::Measured {
		Ok(())
	} else {
		Err(ProbeError::MissingMeasurement(name.to_owned()))
	}
}

fn build_cache_identity(
	seed: &SeedContract,
	host: &HostInventory,
	gpus: &[GpuDescriptor],
	peers: &[(&dyn PeerSession, PeerDescriptor)],
) -> CacheIdentity {
	let mut digest = CanonicalDigest::new("recipe-probe-cache-v5", PROFILE_SCHEMA);
	hash_seed(&mut digest, seed);
	hash_machine(&mut digest, &host.machine);
	for ram in &host.ram {
		digest.string("ram");
		digest.string(ram.key.as_str());
		digest.u64(ram.capacity_hint.get());
		digest.string(ram.link_identity.as_str());
		digest.u64(u64::from(ram.maximum_inflight_transfers.get()));
	}
	for storage in &host.storage {
		digest.string("storage");
		digest.string(storage.key.as_str());
		digest.string(storage.name.as_str());
		digest.u64(storage.capacity_hint.get());
		digest.string(&storage.benchmark_root.to_string_lossy());
		digest.string(storage.host_memory_key.as_str());
		digest.string(storage.driver.as_str());
		digest.string(storage.firmware.as_str());
		digest.string(storage.link_identity.as_str());
		hash_transport_kind(&mut digest, storage.transport_kind);
		digest.bool(storage.asynchronous_submission);
		digest.bool(storage.duplex == LinkDuplex::Full);
		digest.u64(u64::from(storage.maximum_concurrent_reads.get()));
		digest.u64(u64::from(storage.maximum_concurrent_writes.get()));
	}
	for interface in &host.network {
		digest.string("network");
		digest.string(interface.key.as_str());
		digest.string(interface.name.as_str());
		digest.string(interface.address.as_str());
		digest.string(interface.driver.as_str());
		digest.string(interface.firmware.as_str());
		digest.string(interface.link_identity.as_str());
		hash_transport_kind(&mut digest, interface.transport_kind);
		digest.bool(interface.asynchronous_submission);
		digest.bool(interface.duplex == LinkDuplex::Full);
	}
	for gpu in gpus {
		digest.string("gpu");
		digest.string(gpu.key.as_str());
		digest.string(gpu.name.as_str());
		digest.u64(gpu.capacity_hint.get());
		digest.string(gpu.host_memory_key.as_str());
		digest.string(gpu.target.backend.as_str());
		digest.string(gpu.target.architecture.as_str());
		digest.string(gpu.target.abi.as_str());
		digest.string(gpu.driver.as_str());
		digest.string(gpu.runtime_abi.as_str());
		digest.string(gpu.firmware.as_str());
		digest.string(gpu.link_identity.as_str());
		hash_transport_kind(&mut digest, gpu.transport_kind);
		digest.string(gpu.toolchain.name.as_str());
		digest.string(gpu.toolchain.version.as_str());
		digest.digest(gpu.toolchain.digest);
		digest.bool(gpu.asynchronous_submission);
		digest.u64(u64::from(gpu.maximum_concurrent_tasks));
		digest.bool(gpu.transfer_overlaps_calculation);
		digest.bool(gpu.duplex == LinkDuplex::Full);
		digest.u64(u64::from(gpu.host_to_device_maximum_inflight.get()));
		digest.u64(u64::from(gpu.device_to_host_maximum_inflight.get()));
	}
	for (_, peer) in peers {
		digest.string("peer");
		digest.string(peer.session_id.as_str());
		hash_machine(&mut digest, &peer.machine);
		digest.string(peer.remote_memory_key.as_str());
		digest.string(peer.local_memory_key.as_str());
		digest.string(peer.local_interface_key.as_str());
		digest.string(peer.remote_interface_identity.as_str());
		digest.string(peer.remote_driver.as_str());
		digest.string(peer.remote_firmware.as_str());
		digest.string(peer.link_identity.as_str());
		hash_transport_kind(&mut digest, peer.transport_kind);
		digest.bool(peer.asynchronous_submission);
		digest.bool(peer.duplex == LinkDuplex::Full);
		digest.u64(u64::from(peer.outbound_maximum_inflight.get()));
		digest.u64(u64::from(peer.inbound_maximum_inflight.get()));
	}
	CacheIdentity {
		schema: PROFILE_SCHEMA,
		digest: digest.finish(),
	}
}

fn hash_transport_kind(digest: &mut CanonicalDigest, kind: TransportKind) {
	digest.string(match kind {
		TransportKind::Memory => "memory",
		TransportKind::Pcie => "pcie",
		TransportKind::Sata => "sata",
		TransportKind::Sas => "sas",
		TransportKind::Nvme => "nvme",
		TransportKind::Ethernet => "ethernet",
		TransportKind::Wlan => "wlan",
	});
}

fn hash_seed(digest: &mut CanonicalDigest, seed: &SeedContract) {
	digest.u64(u64::from(seed.schema));
	for value in [
		seed.estimates.ethernet_rate.get(),
		seed.estimates.disk_capacity.get(),
		seed.estimates.sata_rate.get(),
		seed.estimates.gpu_memory_capacity.get(),
		seed.estimates.pcie_rate.get(),
		seed.estimates.gpu_calculation_rate.get(),
		seed.estimates.gpu_memory_rate.get(),
		seed.estimates.ram_capacity.get(),
		seed.estimates.ram_rate.get(),
		seed.estimates.cpu_reference_rate.get(),
		seed.estimates.ram_transfer_rate.get(),
		seed.reservation_per_storage_device.get(),
	] {
		digest.u64(value);
	}
	for facet in &seed.invalidation {
		digest.string(match facet {
			IdentityFacet::Machine => "machine",
			IdentityFacet::Device => "device",
			IdentityFacet::Driver => "driver",
			IdentityFacet::RuntimeAbi => "runtime-abi",
			IdentityFacet::Firmware => "firmware",
			IdentityFacet::Link => "link",
			IdentityFacet::ArtifactToolchain => "artifact-toolchain",
		});
	}
	for transport in &seed.transports {
		digest.string(transport.name.as_str());
		digest.string(match transport.duplex {
			SeedDuplex::Full => "full",
			SeedDuplex::Half => "half",
		});
	}
}

fn hash_machine(digest: &mut CanonicalDigest, machine: &crate::model::MachineFingerprint) {
	digest.string(machine.hostname.as_str());
	digest.string(machine.stable_id.as_str());
	digest.string(machine.runtime_abi.as_str());
	digest.string(machine.firmware.as_str());
}

fn build_profile_digest(domain: &str, cache: CacheIdentity, measurements: &Measurements) -> recipe_core::Digest {
	let mut digest = CanonicalDigest::new(domain, PROFILE_SCHEMA);
	digest.digest(cache.digest);
	for (key, measurement) in &measurements.ram {
		digest.string("ram");
		digest.string(key.as_str());
		digest.u64(measurement.capacity.value.get());
		digest.u64(measurement.transfer_rate.value.get());
	}
	for (key, measurement) in &measurements.storage {
		digest.string("storage");
		digest.string(key.as_str());
		digest.u64(measurement.capacity.value.get());
		digest.u64(measurement.read_rate.value.get());
		digest.u64(measurement.write_rate.value.get());
	}
	for (key, measurement) in &measurements.gpu {
		digest.string("gpu");
		digest.string(key.as_str());
		digest.u64(measurement.capacity.value.get());
		digest.u64(measurement.calculation_rate.value.get());
		digest.u64(measurement.memory_rate.value.get());
		digest.u64(measurement.host_to_device_rate.value.get());
		digest.u64(measurement.device_to_host_rate.value.get());
	}
	for (peer, measurement) in &measurements.peers {
		digest.string("peer");
		digest.string(peer.session_id.as_str());
		digest.u64(measurement.remote_memory_capacity.value.get());
		digest.u64(measurement.remote_memory_rate.value.get());
		if let Some(outbound) = measurement.outbound_rate {
			digest.u64(outbound.value.get());
		}
		if let Some(inbound) = measurement.inbound_rate {
			digest.u64(inbound.value.get());
		}
		hash_peer_evidence(&mut digest, measurement.evidence);
	}
	digest.finish()
}

fn hash_peer_evidence(digest: &mut CanonicalDigest, evidence: crate::model::PeerBenchmarkEvidence) {
	digest.u64(u64::from(evidence.protocol_schema));
	for endpoint in [evidence.local_endpoint, evidence.remote_endpoint] {
		digest.digest(endpoint.machine);
		digest.digest(endpoint.profile);
	}
	digest.bool(matches!(
		evidence.execution,
		PeerDuplexExecution::Simultaneous
	));
	for direction in [evidence.outbound, evidence.inbound] {
		digest.u64(direction.total_bytes.get());
		digest.u64(direction.elapsed_nanoseconds);
		digest.u64(u64::from(direction.sample_count));
		digest.u64(direction.minimum_sample_nanoseconds);
		digest.u64(direction.maximum_sample_nanoseconds);
		digest.u64(direction.mean_sample_nanoseconds);
		digest.bytes(&direction.variance_nanoseconds_squared.to_le_bytes());
	}
}

#[derive(Debug)]
struct DeviceMaps {
	local_machine: MachineId,
	ram: BTreeMap<Label, DeviceId>,
	storage: BTreeMap<Label, DeviceId>,
	gpu: BTreeMap<Label, DeviceId>,
	peer_ram: BTreeMap<Label, DeviceId>,
	peer_machine: BTreeMap<Label, MachineId>,
}

fn assign_ids(host: &HostInventory, gpus: &[GpuDescriptor], measurements: &Measurements) -> DeviceMaps {
	let mut next_device = 1u64;
	let mut assign = || {
		let id = DeviceId::new(next_device);
		next_device += 1;
		id
	};
	let ram = host
		.ram
		.iter()
		.map(|domain| (domain.key.clone(), assign()))
		.collect();
	let storage = host
		.storage
		.iter()
		.map(|domain| (domain.key.clone(), assign()))
		.collect();
	let gpu = gpus
		.iter()
		.map(|device| (device.key.clone(), assign()))
		.collect();
	let peer_ram = measurements
		.peers
		.iter()
		.map(|(peer, _)| (peer.session_id.clone(), assign()))
		.collect();
	let peer_machine = measurements
		.peers
		.iter()
		.enumerate()
		.map(|(index, (peer, _))| {
			(
				peer.session_id.clone(),
				MachineId::new(u64::try_from(index).unwrap_or(u64::MAX) + 2),
			)
		})
		.collect();
	DeviceMaps {
		local_machine: MachineId::new(1),
		ram,
		storage,
		gpu,
		peer_ram,
		peer_machine,
	}
}

fn build_origins(host: &HostInventory, gpus: &[GpuDescriptor], measurements: &Measurements) -> MeasuredOrigins {
	let ids = assign_ids(host, gpus, measurements);
	let mut machines = vec![MeasuredMachineOrigin {
		machine: ids.local_machine,
		fingerprint: host.machine.clone(),
	}];
	for (peer, _) in &measurements.peers {
		machines.push(MeasuredMachineOrigin {
			machine: ids.peer_machine[&peer.session_id],
			fingerprint: peer.machine.clone(),
		});
	}
	let mut ram = host
		.ram
		.iter()
		.map(|domain| MeasuredRamOrigin {
			device: ids.ram[&domain.key],
			key: domain.key.clone(),
		})
		.collect::<Vec<_>>();
	for (peer, _) in &measurements.peers {
		ram.push(MeasuredRamOrigin {
			device: ids.peer_ram[&peer.session_id],
			key: peer.remote_memory_key.clone(),
		});
	}
	let storage = host
		.storage
		.iter()
		.map(|domain| MeasuredStorageOrigin {
			device: ids.storage[&domain.key],
			key: domain.key.clone(),
		})
		.collect();
	let gpu = gpus
		.iter()
		.map(|device| MeasuredGpuOrigin {
			device: ids.gpu[&device.key],
			key: device.key.clone(),
		})
		.collect();
	MeasuredOrigins {
		machines,
		ram,
		storage,
		gpu,
	}
}

fn build_topology(
	identity: TopologyIdentity,
	host: &HostInventory,
	gpus: &[GpuDescriptor],
	measurements: &Measurements,
) -> ProbeResult<Topology> {
	let ids = assign_ids(host, gpus, measurements);
	let mut machines = vec![Machine {
		id: ids.local_machine,
		name: host.machine.hostname.clone(),
	}];
	for (peer, _) in &measurements.peers {
		machines.push(Machine {
			id: ids.peer_machine[&peer.session_id],
			name: peer.machine.hostname.clone(),
		});
	}
	let mut local_devices = Vec::new();
	let mut devices = Vec::new();
	for domain in &host.ram {
		let id = ids.ram[&domain.key];
		local_devices.push(id);
		let measurement = measurements.ram[&domain.key];
		devices.push(Device {
			id,
			machine: ids.local_machine,
			kind: DeviceKind::Ram,
			capacity: measurement.capacity,
			transfer_rate: measurement.transfer_rate,
			calculation_rate: None,
		});
	}
	for domain in &host.storage {
		let id = ids.storage[&domain.key];
		local_devices.push(id);
		let measurement = measurements.storage[&domain.key];
		devices.push(Device {
			id,
			machine: ids.local_machine,
			kind: DeviceKind::Disk,
			capacity: measurement.capacity,
			transfer_rate: measured(min_rate(
				measurement.read_rate.value,
				measurement.write_rate.value,
			)),
			calculation_rate: None,
		});
	}
	for descriptor in gpus {
		let id = ids.gpu[&descriptor.key];
		local_devices.push(id);
		let measurement = measurements.gpu[&descriptor.key];
		devices.push(Device {
			id,
			machine: ids.local_machine,
			kind: DeviceKind::GpuMemory,
			capacity: measurement.capacity,
			transfer_rate: measurement.memory_rate,
			calculation_rate: Some(measurement.calculation_rate),
		});
	}
	let mut nodes = vec![Node {
		id: NodeId::new(1),
		role: NodeRole::Master,
		machine: ids.local_machine,
		devices: local_devices,
	}];
	for (index, (peer, measurement)) in measurements.peers.iter().enumerate() {
		let device = ids.peer_ram[&peer.session_id];
		let machine = ids.peer_machine[&peer.session_id];
		devices.push(Device {
			id: device,
			machine,
			kind: DeviceKind::Ram,
			capacity: measurement.remote_memory_capacity,
			transfer_rate: measurement.remote_memory_rate,
			calculation_rate: None,
		});
		nodes.push(Node {
			id: NodeId::new(u64::try_from(index).unwrap_or(u64::MAX) + 2),
			role: NodeRole::Worker,
			machine,
			devices: vec![device],
		});
	}

	let mut link_builder = LinkBuilder::default();
	for domain in &host.storage {
		let measurement = measurements.storage[&domain.key];
		link_builder.push_pair(
			ids.ram[&domain.host_memory_key],
			ids.storage[&domain.key],
			DirectionalLinkMeasurement {
				bandwidth: measurement.write_rate,
				maximum_inflight: domain.maximum_concurrent_writes,
			},
			DirectionalLinkMeasurement {
				bandwidth: measurement.read_rate,
				maximum_inflight: domain.maximum_concurrent_reads,
			},
			domain.transport_kind,
			domain.duplex,
		);
	}
	for descriptor in gpus {
		let measurement = measurements.gpu[&descriptor.key];
		link_builder.push_pair(
			ids.ram[&descriptor.host_memory_key],
			ids.gpu[&descriptor.key],
			DirectionalLinkMeasurement {
				bandwidth: measurement.host_to_device_rate,
				maximum_inflight: descriptor.host_to_device_maximum_inflight,
			},
			DirectionalLinkMeasurement {
				bandwidth: measurement.device_to_host_rate,
				maximum_inflight: descriptor.device_to_host_maximum_inflight,
			},
			descriptor.transport_kind,
			descriptor.duplex,
		);
	}
	for (peer, measurement) in &measurements.peers {
		link_builder.push_pair(
			ids.ram[&peer.local_memory_key],
			ids.peer_ram[&peer.session_id],
			DirectionalLinkMeasurement {
				bandwidth: measurement
					.outbound_rate
					.expect("validated before topology build"),
				maximum_inflight: peer.outbound_maximum_inflight,
			},
			DirectionalLinkMeasurement {
				bandwidth: measurement
					.inbound_rate
					.expect("validated before topology build"),
				maximum_inflight: peer.inbound_maximum_inflight,
			},
			peer.transport_kind,
			peer.duplex,
		);
	}
	let topology = Topology {
		identity,
		machines,
		nodes,
		devices,
		links: link_builder.links,
	};
	topology
		.validate()
		.map_err(|errors| ProbeError::InvalidProfile(format!("constructed topology is invalid: {errors}")))?;
	Ok(topology)
}

#[derive(Debug)]
struct LinkBuilder {
	links: Vec<DirectedLink>,
	next_link: u64,
	next_transport: u64,
	next_resource: u64,
}

impl Default for LinkBuilder {
	fn default() -> Self {
		Self {
			links: Vec::new(),
			next_link: 1,
			next_transport: 1,
			next_resource: 1,
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct DirectionalLinkMeasurement {
	bandwidth: Property<BytesPerSecond>,
	maximum_inflight: TransferLaneCount,
}

impl LinkBuilder {
	fn push_pair(
		&mut self,
		from: DeviceId,
		to: DeviceId,
		forward: DirectionalLinkMeasurement,
		reverse: DirectionalLinkMeasurement,
		kind: TransportKind,
		duplex: LinkDuplex,
	) {
		let transport = TransportId::new(self.next_transport);
		self.next_transport += 1;
		let forward_resource = DuplexResourceId::new(self.next_resource);
		self.next_resource += 1;
		let reverse_resource = if duplex == LinkDuplex::Full {
			let id = DuplexResourceId::new(self.next_resource);
			self.next_resource += 1;
			id
		} else {
			forward_resource
		};
		self.links.push(DirectedLink {
			id: LinkId::new(self.next_link),
			transport,
			kind,
			duplex: duplex_mode(duplex),
			from,
			to,
			bandwidth: forward.bandwidth,
			maximum_inflight_transfers: measured(forward.maximum_inflight),
			capacity_resource: forward_resource,
		});
		self.next_link += 1;
		self.links.push(DirectedLink {
			id: LinkId::new(self.next_link),
			transport,
			kind,
			duplex: duplex_mode(duplex),
			from: to,
			to: from,
			bandwidth: reverse.bandwidth,
			maximum_inflight_transfers: measured(reverse.maximum_inflight),
			capacity_resource: reverse_resource,
		});
		self.next_link += 1;
	}
}

const fn duplex_mode(duplex: LinkDuplex) -> DuplexMode {
	match duplex {
		LinkDuplex::Full => DuplexMode::Full,
		LinkDuplex::Half => DuplexMode::Half,
	}
}

fn build_discovery(
	identity: DiscoveryIdentity,
	topology: &Topology,
	host: &HostInventory,
	gpus: &[GpuDescriptor],
	measurements: &Measurements,
) -> ProbeResult<DiscoveryProfile> {
	let ids = assign_ids(host, gpus, measurements);
	let mut devices = Vec::new();
	for domain in &host.ram {
		let measurement = measurements.ram[&domain.key];
		devices.push(DiscoveredDevice {
			device: ids.ram[&domain.key],
			available: true,
			total_capacity: measurement.capacity,
			transfer: TransferCapability {
				rate: measurement.transfer_rate,
				maximum_inflight_transfers: measured(domain.maximum_inflight_transfers),
				asynchronous_submission: true,
				overlaps_calculation: true,
			},
			calculation: None,
		});
	}
	for domain in &host.storage {
		let measurement = measurements.storage[&domain.key];
		devices.push(DiscoveredDevice {
			device: ids.storage[&domain.key],
			available: true,
			total_capacity: measurement.capacity,
			transfer: TransferCapability {
				rate: measured(min_rate(
					measurement.read_rate.value,
					measurement.write_rate.value,
				)),
				maximum_inflight_transfers: measured(min_lanes(
					domain.maximum_concurrent_reads,
					domain.maximum_concurrent_writes,
				)),
				asynchronous_submission: domain.asynchronous_submission,
				overlaps_calculation: true,
			},
			calculation: None,
		});
	}
	for descriptor in gpus {
		let measurement = measurements.gpu[&descriptor.key];
		devices.push(DiscoveredDevice {
			device: ids.gpu[&descriptor.key],
			available: true,
			total_capacity: measurement.capacity,
			transfer: TransferCapability {
				rate: measurement.memory_rate,
				maximum_inflight_transfers: measured(min_lanes(
					descriptor.host_to_device_maximum_inflight,
					descriptor.device_to_host_maximum_inflight,
				)),
				asynchronous_submission: descriptor.asynchronous_submission,
				overlaps_calculation: descriptor.transfer_overlaps_calculation,
			},
			calculation: Some(CalculationCapability {
				target: descriptor.target.clone(),
				rate: measurement.calculation_rate,
				asynchronous_submission: descriptor.asynchronous_submission,
				maximum_concurrent_tasks: descriptor.maximum_concurrent_tasks,
			}),
		});
	}
	for (peer, measurement) in &measurements.peers {
		devices.push(DiscoveredDevice {
			device: ids.peer_ram[&peer.session_id],
			available: true,
			total_capacity: measurement.remote_memory_capacity,
			transfer: TransferCapability {
				rate: measurement.remote_memory_rate,
				maximum_inflight_transfers: measured(min_lanes(
					peer.outbound_maximum_inflight,
					peer.inbound_maximum_inflight,
				)),
				asynchronous_submission: peer.asynchronous_submission,
				overlaps_calculation: true,
			},
			calculation: None,
		});
	}
	let mut link_async = Vec::new();
	for domain in &host.storage {
		link_async.push(domain.asynchronous_submission);
		link_async.push(domain.asynchronous_submission);
	}
	for descriptor in gpus {
		link_async.push(descriptor.asynchronous_submission);
		link_async.push(descriptor.asynchronous_submission);
	}
	for (peer, _) in &measurements.peers {
		link_async.push(peer.asynchronous_submission);
		link_async.push(peer.asynchronous_submission);
	}
	let links = topology
		.links
		.iter()
		.zip(link_async)
		.map(|(link, asynchronous_submission)| DiscoveredLink {
			link: link.id,
			available: true,
			bandwidth: link.bandwidth,
			maximum_inflight_transfers: link.maximum_inflight_transfers,
			asynchronous_submission,
		})
		.collect();
	Ok(DiscoveryProfile {
		identity,
		topology: topology.identity,
		devices,
		links,
	})
}

fn measured<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Measured)
}

fn min_rate(left: BytesPerSecond, right: BytesPerSecond) -> BytesPerSecond {
	if left <= right { left } else { right }
}

fn min_lanes(left: TransferLaneCount, right: TransferLaneCount) -> TransferLaneCount {
	if left <= right { left } else { right }
}

#[cfg(test)]
mod tests {
	use std::fs;
	use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
	use std::path::{Path, PathBuf};
	use std::sync::atomic::{AtomicU64, Ordering};

	use super::*;
	use recipe_core::{Digest, FlopsPerSecond, ToolchainIdentity};

	use crate::cache::ExplicitPathProfileCache;
	use crate::codec::MeasuredProfileCodec;
	use crate::model::{GpuInventory, MachineFingerprint, NetworkInterface, ProfileCache, RamDomain, StorageDomain};

	fn label(value: &str) -> Label {
		Label::new(value).unwrap()
	}

	fn measured_bytes(value: u64) -> Property<ByteCount> {
		measured(ByteCount::new(value))
	}

	fn measured_rate(value: u64) -> Property<BytesPerSecond> {
		measured(BytesPerSecond::new(value).unwrap())
	}

	fn measured_flops(value: u64) -> Property<FlopsPerSecond> {
		measured(FlopsPerSecond::new(value).unwrap())
	}

	fn lanes(value: u32) -> TransferLaneCount {
		TransferLaneCount::new(value).unwrap()
	}

	#[derive(Debug)]
	struct FakeHost;

	impl HostDiscovery for FakeHost {
		fn discover_host(&self) -> ProbeResult<HostInventory> {
			Ok(HostInventory {
				machine: MachineFingerprint {
					hostname: label("local-machine"),
					stable_id: label("machine-id-1"),
					runtime_abi: label("kernel-1"),
					firmware: label("firmware-1"),
				},
				ram: vec![RamDomain {
					key: label("ram-0"),
					capacity_hint: ByteCount::new(64_000_000_000),
					link_identity: label("ram-link-0"),
					maximum_inflight_transfers: lanes(2),
				}],
				storage: vec![StorageDomain {
					key: label("disk-0"),
					name: label("disk"),
					benchmark_root: "/not-used-by-fake".into(),
					capacity_hint: ByteCount::new(2_000_000_000_000),
					host_memory_key: label("ram-0"),
					driver: label("disk-driver-1"),
					firmware: label("disk-firmware-1"),
					link_identity: label("disk-link-1"),
					transport_kind: TransportKind::Sata,
					duplex: LinkDuplex::Half,
					maximum_concurrent_reads: lanes(2),
					maximum_concurrent_writes: lanes(2),
					asynchronous_submission: true,
				}],
				network: vec![NetworkInterface {
					key: label("net-0"),
					name: label("net"),
					address: label("00:11:22:33:44:55"),
					driver: label("net-driver-1"),
					firmware: label("net-firmware-1"),
					link_identity: label("net-link-1"),
					transport_kind: TransportKind::Ethernet,
					duplex: LinkDuplex::Full,
					asynchronous_submission: true,
				}],
			})
		}
	}

	#[derive(Debug)]
	struct FakeGpuDiscovery {
		driver: &'static str,
		exhaustive: bool,
	}

	impl GpuDiscovery for FakeGpuDiscovery {
		fn discover_all(&self) -> ProbeResult<GpuInventory> {
			let device = |key: &str, arch: &str| GpuDescriptor {
				key: label(key),
				name: label(key),
				host_memory_key: label("ram-0"),
				target: recipe_core::TargetIdentity {
					backend: label("backend"),
					architecture: label(arch),
					abi: label("abi-1"),
				},
				capacity_hint: ByteCount::new(16_000_000_000),
				driver: label(self.driver),
				runtime_abi: label("runtime-1"),
				firmware: label("gpu-firmware-1"),
				link_identity: label(&format!("pcie-{key}")),
				transport_kind: TransportKind::Pcie,
				toolchain: ToolchainIdentity {
					name: label("toolchain"),
					version: label("1"),
					digest: Digest::new([9; 32]),
				},
				duplex: LinkDuplex::Full,
				host_to_device_maximum_inflight: lanes(2),
				device_to_host_maximum_inflight: lanes(2),
				asynchronous_submission: true,
				maximum_concurrent_tasks: 2,
				transfer_overlaps_calculation: true,
			};
			Ok(GpuInventory {
				exhaustive: self.exhaustive,
				devices: vec![device("gpu-1", "arch-1"), device("gpu-0", "arch-0")],
			})
		}
	}

	#[derive(Debug)]
	struct FakeHostBenchmarks {
		provenance: PropertyProvenance,
	}

	impl HostBenchmarkIo for FakeHostBenchmarks {
		fn benchmark_ram(
			&self,
			_domain: &crate::model::RamDomain,
			_plan: BoundedBenchmarkPlan,
		) -> ProbeResult<RamMeasurement> {
			Ok(RamMeasurement {
				capacity: Property::new(ByteCount::new(64_000_000_000), self.provenance),
				transfer_rate: Property::new(
					BytesPerSecond::new(100_000_000_000).unwrap(),
					self.provenance,
				),
			})
		}

		fn benchmark_storage(
			&self,
			_domain: &crate::model::StorageDomain,
			_plan: BoundedBenchmarkPlan,
		) -> ProbeResult<StorageMeasurement> {
			Ok(StorageMeasurement {
				capacity: Property::new(ByteCount::new(2_000_000_000_000), self.provenance),
				read_rate: Property::new(BytesPerSecond::new(700_000_000).unwrap(), self.provenance),
				write_rate: Property::new(BytesPerSecond::new(600_000_000).unwrap(), self.provenance),
			})
		}
	}

	#[derive(Debug)]
	struct FakeGpuBenchmarks;

	impl GpuBenchmarkIo for FakeGpuBenchmarks {
		fn benchmark_gpu(
			&self,
			_device: &GpuDescriptor,
			_plan: BoundedBenchmarkPlan,
		) -> ProbeResult<GpuMeasurement> {
			Ok(GpuMeasurement {
				capacity: measured_bytes(16_000_000_000),
				calculation_rate: measured_flops(400_000_000_000),
				memory_rate: measured_rate(500_000_000_000),
				host_to_device_rate: measured_rate(15_000_000_000),
				device_to_host_rate: measured_rate(14_000_000_000),
			})
		}
	}

	#[derive(Debug)]
	struct FakePeer {
		complete: bool,
	}

	impl PeerSession for FakePeer {
		fn descriptor(&self) -> ProbeResult<PeerDescriptor> {
			Ok(PeerDescriptor {
				session_id: label("peer-session-1"),
				machine: MachineFingerprint {
					hostname: label("remote-machine"),
					stable_id: label("machine-id-2"),
					runtime_abi: label("kernel-2"),
					firmware: label("firmware-2"),
				},
				remote_memory_key: label("remote-ram-0"),
				local_memory_key: label("ram-0"),
				local_interface_key: label("net-0"),
				remote_interface_identity: label("remote-net-0"),
				remote_driver: label("remote-net-driver"),
				remote_firmware: label("remote-net-firmware"),
				link_identity: label("peer-link-1"),
				transport_kind: TransportKind::Ethernet,
				duplex: LinkDuplex::Full,
				outbound_maximum_inflight: lanes(2),
				inbound_maximum_inflight: lanes(2),
				asynchronous_submission: true,
			})
		}

		fn benchmark(&self, plan: BoundedBenchmarkPlan) -> ProbeResult<PeerMeasurement> {
			let outbound = fake_peer_direction(plan, 1_000_000);
			let inbound = fake_peer_direction(plan, 1_200_000);
			Ok(PeerMeasurement {
				remote_memory_capacity: measured_bytes(32_000_000_000),
				remote_memory_rate: measured_rate(80_000_000_000),
				outbound_rate: Some(measured_rate(rate_from_evidence(outbound))),
				inbound_rate: self
					.complete
					.then(|| measured_rate(rate_from_evidence(inbound))),
				evidence: crate::model::PeerBenchmarkEvidence {
					protocol_schema: PEER_BENCHMARK_PROTOCOL_SCHEMA,
					local_endpoint: crate::model::PeerEndpointEvidence {
						machine: Digest::new([1; 32]),
						profile: Digest::new([2; 32]),
					},
					remote_endpoint: crate::model::PeerEndpointEvidence {
						machine: Digest::new([3; 32]),
						profile: Digest::new([4; 32]),
					},
					execution: PeerDuplexExecution::Simultaneous,
					outbound,
					inbound,
				},
			})
		}
	}

	fn fake_peer_direction(
		plan: BoundedBenchmarkPlan,
		elapsed_nanoseconds: u64,
	) -> crate::model::DirectionalBenchmarkEvidence {
		let total_bytes = plan
			.buffer_bytes
			.get()
			.checked_mul(u64::from(plan.iterations))
			.unwrap();
		let mean = (elapsed_nanoseconds / u64::from(plan.iterations)).max(1);
		crate::model::DirectionalBenchmarkEvidence {
			total_bytes: ByteCount::new(total_bytes),
			elapsed_nanoseconds,
			sample_count: plan.iterations,
			minimum_sample_nanoseconds: mean,
			maximum_sample_nanoseconds: mean,
			mean_sample_nanoseconds: mean,
			variance_nanoseconds_squared: 0,
		}
	}

	fn rate_from_evidence(evidence: crate::model::DirectionalBenchmarkEvidence) -> u64 {
		u64::try_from(
			u128::from(evidence.total_bytes.get()) * 1_000_000_000 / u128::from(evidence.elapsed_nanoseconds),
		)
		.unwrap()
	}

	fn seed() -> SeedContract {
		SeedContract::parse(include_str!("../../topology/contract.toml")).unwrap()
	}

	fn engine<'a>(gpu: &'a FakeGpuDiscovery, host_benchmarks: &'a FakeHostBenchmarks) -> ProbeEngine<'a> {
		ProbeEngine::new(&FakeHost, gpu, host_benchmarks, &FakeGpuBenchmarks)
	}

	#[derive(Debug)]
	struct TestDirectory(PathBuf);

	impl TestDirectory {
		fn new() -> Self {
			static NEXT: AtomicU64 = AtomicU64::new(1);
			for _ in 0..64 {
				let path = std::env::temp_dir().join(format!(
					"recipe-probe-test-{}-{}",
					std::process::id(),
					NEXT.fetch_add(1, Ordering::Relaxed)
				));
				match fs::create_dir(&path) {
					Ok(()) => {
						fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
						return Self(path);
					}
					Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
					Err(error) => panic!("create test directory: {error}"),
				}
			}
			panic!("could not allocate test directory");
		}

		fn path(&self) -> &Path {
			&self.0
		}
	}

	impl Drop for TestDirectory {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.0);
		}
	}

	#[test]
	fn automatically_enumerates_every_backend_gpu() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let profile = engine(&gpu, &benchmarks).probe(&seed(), &[]).unwrap();
		assert_eq!(
			profile
				.topology
				.devices
				.iter()
				.filter(|device| device.kind == DeviceKind::GpuMemory)
				.count(),
			2
		);
		assert!(
			profile
				.topology
				.devices
				.iter()
				.all(|device| device.capacity.provenance == PropertyProvenance::Measured)
		);
		assert_eq!(
			profile.topology.machines.len(),
			1,
			"a discovered interface alone must not fabricate a measured peer link"
		);
		assert_eq!(
			profile
				.origins
				.storage
				.iter()
				.map(|origin| origin.key.as_str())
				.collect::<Vec<_>>(),
			vec!["disk-0"]
		);
		assert_eq!(
			profile
				.origins
				.gpu
				.iter()
				.map(|origin| origin.key.as_str())
				.collect::<Vec<_>>(),
			vec!["gpu-0", "gpu-1"]
		);
		for device in &profile.topology.devices {
			match device.kind {
				DeviceKind::Ram => assert_eq!(
					profile
						.ram_origin(device.id)
						.map(|origin| origin.key.as_str()),
					Some("ram-0")
				),
				DeviceKind::Disk => assert_eq!(
					profile
						.storage_origin(device.id)
						.map(|origin| origin.key.as_str()),
					Some("disk-0")
				),
				DeviceKind::GpuMemory => assert!(profile.gpu_origin(device.id).is_some()),
			}
		}
		let host = FakeHost.discover_host().unwrap();
		let live_gpus = gpu.discover_all().unwrap();
		let resolved = profile.resolve_local_inventory(&host, &live_gpus).unwrap();
		assert_eq!(resolved.machine(), profile.topology.machines[0].id);
		assert_eq!(resolved.ram().len(), 1);
		assert_eq!(resolved.storage().len(), 1);
		assert_eq!(resolved.gpu().len(), 2);
	}

	#[test]
	fn theoretical_seed_values_only_bound_benchmarks_and_never_become_profile_values() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let contract = seed();
		let profile = engine(&gpu, &benchmarks).probe(&contract, &[]).unwrap();

		let ram = profile
			.topology
			.devices
			.iter()
			.find(|device| device.kind == DeviceKind::Ram)
			.unwrap();
		assert_eq!(ram.capacity.value.get(), 64_000_000_000);
		assert_ne!(ram.capacity.value, contract.estimates.ram_capacity);
		assert_eq!(ram.transfer_rate.value.get(), 100_000_000_000);
		assert_ne!(
			ram.transfer_rate.value,
			contract.estimates.ram_transfer_rate
		);

		let storage = profile
			.topology
			.devices
			.iter()
			.find(|device| device.kind == DeviceKind::Disk)
			.unwrap();
		assert_eq!(storage.capacity.value.get(), 2_000_000_000_000);
		assert_ne!(storage.capacity.value, contract.estimates.disk_capacity);

		for gpu_device in profile
			.topology
			.devices
			.iter()
			.filter(|device| device.kind == DeviceKind::GpuMemory)
		{
			assert_eq!(gpu_device.capacity.value.get(), 16_000_000_000);
			assert_ne!(
				gpu_device.capacity.value,
				contract.estimates.gpu_memory_capacity
			);
			assert_eq!(
				gpu_device.calculation_rate.unwrap().value.get(),
				400_000_000_000
			);
			assert_ne!(
				gpu_device.calculation_rate.unwrap().value,
				contract.estimates.gpu_calculation_rate
			);
			assert_eq!(gpu_device.transfer_rate.value.get(), 500_000_000_000);
			assert_ne!(
				gpu_device.transfer_rate.value,
				contract.estimates.gpu_memory_rate
			);
		}
	}

	#[test]
	fn local_inventory_resolution_never_falls_back_to_similar_hardware() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let profile = engine(&gpu, &benchmarks).probe(&seed(), &[]).unwrap();
		let host = FakeHost.discover_host().unwrap();
		let mut live_gpus = gpu.discover_all().unwrap();
		live_gpus.devices[0].key = label("gpu-similar");
		assert!(profile.resolve_local_inventory(&host, &live_gpus).is_err());
		let mut duplicate_gpus = gpu.discover_all().unwrap();
		let duplicate = duplicate_gpus.devices[0].clone();
		duplicate_gpus.devices.push(duplicate);
		assert!(
			profile
				.resolve_local_inventory(&host, &duplicate_gpus)
				.is_err()
		);

		let mut live_host = host;
		live_host.storage[0].key = label("disk-similar");
		assert!(
			profile
				.resolve_local_inventory(&live_host, &gpu.discover_all().unwrap())
				.is_err()
		);
	}

	#[test]
	fn deterministic_inputs_produce_deterministic_profile_ids() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let engine = engine(&gpu, &benchmarks);
		let first = engine.probe(&seed(), &[]).unwrap();
		let second = engine.probe(&seed(), &[]).unwrap();
		assert_eq!(first.cache_identity, second.cache_identity);
		assert_eq!(first.topology.identity, second.topology.identity);
		assert_eq!(first.discovery.identity, second.discovery.identity);
	}

	#[test]
	fn changed_driver_invalidates_cache_identity() {
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let first_gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let second_gpu = FakeGpuDiscovery {
			driver: "driver-2",
			exhaustive: true,
		};
		let first = engine(&first_gpu, &benchmarks).probe(&seed(), &[]).unwrap();
		let second = engine(&second_gpu, &benchmarks)
			.probe(&seed(), &[])
			.unwrap();
		assert_ne!(first.cache_identity, second.cache_identity);
		assert!(!first.is_cache_valid_for(second.cache_identity));
	}

	#[test]
	fn changed_stable_ram_origin_invalidates_cache_identity() {
		let mut host = FakeHost.discover_host().unwrap();
		let gpu_discovery = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let mut gpus = gpu_discovery.discover_all().unwrap().devices;
		gpus.sort_by(|left, right| left.key.cmp(&right.key));
		let first = build_cache_identity(&seed(), &host, &gpus, &[]);
		host.ram[0].key = label("ram-renamed");
		host.storage[0].host_memory_key = label("ram-renamed");
		for gpu in &mut gpus {
			gpu.host_memory_key = label("ram-renamed");
		}
		let second = build_cache_identity(&seed(), &host, &gpus, &[]);
		assert_ne!(first, second);
		assert_eq!(first.schema, PROFILE_SCHEMA);
		assert_eq!(second.schema, PROFILE_SCHEMA);
	}

	#[test]
	fn estimate_only_benchmark_results_fail_closed() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let estimates = FakeHostBenchmarks {
			provenance: PropertyProvenance::Estimated,
		};
		let error = engine(&gpu, &estimates).probe(&seed(), &[]).unwrap_err();
		assert!(matches!(error, ProbeError::MissingMeasurement(_)));
	}

	#[test]
	fn incomplete_peer_measurement_fails_closed() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let peer = FakePeer { complete: false };
		let error = engine(&gpu, &benchmarks)
			.probe(&seed(), &[&peer])
			.unwrap_err();
		assert!(matches!(
			error,
			ProbeError::IncompletePeerMeasurement {
				direction: "inbound",
				..
			}
		));
	}

	#[test]
	fn complete_explicit_peer_adds_measured_bidirectional_link() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let peer = FakePeer { complete: true };
		let profile = engine(&gpu, &benchmarks).probe(&seed(), &[&peer]).unwrap();
		assert_eq!(profile.topology.machines.len(), 2);
		assert_eq!(profile.origins.machines.len(), 2);
		assert_eq!(
			profile
				.origins
				.machines
				.iter()
				.map(|origin| origin.fingerprint.stable_id.as_str())
				.collect::<Vec<_>>(),
			vec!["machine-id-1", "machine-id-2"]
		);
		assert_eq!(
			profile
				.origins
				.ram
				.iter()
				.map(|origin| origin.key.as_str())
				.collect::<Vec<_>>(),
			vec!["ram-0", "remote-ram-0"]
		);
		assert!(
			profile
				.topology
				.links
				.iter()
				.all(|link| link.bandwidth.provenance == PropertyProvenance::Measured)
		);
	}

	#[test]
	fn canonical_codec_round_trips_complete_profile_without_loss() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let peer = FakePeer { complete: true };
		let profile = engine(&gpu, &benchmarks).probe(&seed(), &[&peer]).unwrap();
		let encoded = MeasuredProfileCodec::encode(&profile).unwrap();
		let decoded = MeasuredProfileCodec::decode(&encoded).unwrap();
		assert_eq!(decoded, profile);
		assert_eq!(decoded.benchmarks.seed_schema, seed().schema);
	}

	fn assert_codec_rejects_semantic_tamper(profile: &MeasuredProfile) {
		assert!(MeasuredProfileCodec::encode(profile).is_err());
		let bytes = crate::codec::encode_unchecked_for_test(profile).unwrap();
		assert!(MeasuredProfileCodec::decode(&bytes).is_err());
	}

	#[test]
	fn cached_peer_evidence_is_bound_to_measured_rate_and_duplex() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let peer = FakePeer { complete: true };
		let profile = engine(&gpu, &benchmarks).probe(&seed(), &[&peer]).unwrap();

		let mut wrong_execution = profile.clone();
		wrong_execution.peer_benchmarks[0].evidence.execution = PeerDuplexExecution::Serialized;
		assert_codec_rejects_semantic_tamper(&wrong_execution);

		let mut wrong_evidence = profile.clone();
		wrong_evidence.peer_benchmarks[0]
			.evidence
			.outbound
			.elapsed_nanoseconds += 1;
		assert_codec_rejects_semantic_tamper(&wrong_evidence);

		let mut missing_evidence = profile.clone();
		missing_evidence.peer_benchmarks.clear();
		assert_codec_rejects_semantic_tamper(&missing_evidence);

		let mut wrong_topology_rate = profile;
		let remote_machine = wrong_topology_rate.topology.machines[1].id;
		let remote_ram = wrong_topology_rate
			.topology
			.devices
			.iter()
			.find(|device| device.machine == remote_machine && device.kind == DeviceKind::Ram)
			.unwrap()
			.id;
		let link = wrong_topology_rate
			.topology
			.links
			.iter_mut()
			.find(|link| link.to == remote_ram)
			.unwrap();
		link.bandwidth = measured_rate(link.bandwidth.value.get() + 1);
		let discovery_link = wrong_topology_rate
			.discovery
			.links
			.iter_mut()
			.find(|candidate| candidate.link == link.id)
			.unwrap();
		discovery_link.bandwidth = link.bandwidth;
		assert_codec_rejects_semantic_tamper(&wrong_topology_rate);
	}

	#[test]
	fn codec_rejects_estimated_or_wrong_schema_profiles() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let mut profile = engine(&gpu, &benchmarks).probe(&seed(), &[]).unwrap();
		profile.topology.devices[0].capacity.provenance = PropertyProvenance::Estimated;
		assert!(MeasuredProfileCodec::encode(&profile).is_err());
		let estimated_bytes = crate::codec::encode_unchecked_for_test(&profile).unwrap();
		assert!(MeasuredProfileCodec::decode(&estimated_bytes).is_err());
		profile.topology.devices[0].capacity.provenance = PropertyProvenance::Measured;
		let ram_origin = profile.origins.ram.pop().unwrap();
		assert!(MeasuredProfileCodec::encode(&profile).is_err());
		profile.origins.ram.push(ram_origin);
		let storage_origin = profile.origins.storage.pop().unwrap();
		assert!(MeasuredProfileCodec::encode(&profile).is_err());
		profile.origins.storage.push(storage_origin);
		let gpu_origin = profile.origins.gpu.pop().unwrap();
		assert!(MeasuredProfileCodec::encode(&profile).is_err());
		profile.origins.gpu.push(gpu_origin);
		profile.schema += 1;
		assert!(MeasuredProfileCodec::encode(&profile).is_err());
	}

	#[test]
	fn explicit_cache_atomically_round_trips_with_private_permissions() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let directory = TestDirectory::new();
		let path = directory.path().join("measured.profile");
		let cache = ExplicitPathProfileCache::new(&path).unwrap();
		let profile = engine(&gpu, &benchmarks)
			.load_or_probe_and_store(&seed(), &[], &cache)
			.unwrap();
		let cached = engine(&gpu, &benchmarks)
			.load_or_probe_and_store(&seed(), &[], &cache)
			.unwrap();
		assert_eq!(cached, profile);
		let loaded = cache.load(profile.cache_identity).unwrap().unwrap();
		assert_eq!(loaded, profile);
		assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
		let entries = fs::read_dir(directory.path()).unwrap().count();
		assert_eq!(entries, 1, "temporary file must be removed after rename");
	}

	#[test]
	fn cache_rejects_corruption_stale_identity_symlinks_and_overwrite() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: true,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};

		let stale_directory = TestDirectory::new();
		let stale_path = stale_directory.path().join("measured.profile");
		let stale_cache = ExplicitPathProfileCache::new(&stale_path).unwrap();
		let profile = engine(&gpu, &benchmarks).probe(&seed(), &[]).unwrap();
		stale_cache.store(&profile).unwrap();
		let stale_identity = CacheIdentity {
			schema: profile.cache_identity.schema,
			digest: Digest::new([0x55; 32]),
		};
		assert!(stale_cache.load(stale_identity).is_err());
		let mut different = profile.clone();
		different.cache_identity = stale_identity;
		assert!(stale_cache.store(&different).is_err());
		assert_eq!(
			stale_cache.load(profile.cache_identity).unwrap().unwrap(),
			profile
		);

		let corrupt_directory = TestDirectory::new();
		let corrupt_path = corrupt_directory.path().join("measured.profile");
		let corrupt_cache = ExplicitPathProfileCache::new(&corrupt_path).unwrap();
		corrupt_cache.store(&profile).unwrap();
		let mut bytes = fs::read(&corrupt_path).unwrap();
		let middle = bytes.len() / 2;
		bytes[middle] ^= 1;
		fs::write(&corrupt_path, bytes).unwrap();
		assert!(corrupt_cache.load(profile.cache_identity).is_err());

		let symlink_directory = TestDirectory::new();
		let victim = symlink_directory.path().join("victim");
		fs::write(&victim, b"not a profile").unwrap();
		fs::set_permissions(&victim, fs::Permissions::from_mode(0o600)).unwrap();
		let link = symlink_directory.path().join("measured.profile");
		symlink(&victim, &link).unwrap();
		let symlink_cache = ExplicitPathProfileCache::new(link).unwrap();
		assert!(symlink_cache.load(profile.cache_identity).is_err());
	}

	#[test]
	fn exhaustive_enumeration_is_mandatory() {
		let gpu = FakeGpuDiscovery {
			driver: "driver-1",
			exhaustive: false,
		};
		let benchmarks = FakeHostBenchmarks {
			provenance: PropertyProvenance::Measured,
		};
		let error = engine(&gpu, &benchmarks).probe(&seed(), &[]).unwrap_err();
		assert_eq!(error, ProbeError::IncompleteGpuEnumeration);
	}
}
