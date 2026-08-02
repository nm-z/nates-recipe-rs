use std::{
	collections::{BTreeMap, BTreeSet},
	time::Duration,
};

use recipe_core::{
	ByteCount, BytesPerSecond, CalculationCapability, Device, DeviceId, DeviceKind, Digest, DirectedLink,
	DiscoveredDevice, DiscoveredLink, DiscoveryIdentity, DiscoveryProfile, DuplexMode, DuplexResourceId,
	FlopsPerSecond, Label, LinkId, Machine, MachineId, Node, NodeId, NodeRole, Property, PropertyProvenance,
	TargetIdentity, Topology, TopologyIdentity, TransferCapability, TransferLaneCount, TransportId, TransportKind,
};
use sha2::{Digest as _, Sha256};

use crate::{
	error::{ProbeError, ProbeResult},
	model::{
		BenchmarkMetadata, BoundedBenchmarkPlan, CacheIdentity, DirectionalBenchmarkEvidence, MachineFingerprint,
		MeasuredGpuOrigin, MeasuredMachineOrigin, MeasuredOrigins, MeasuredPeerBenchmark, MeasuredProfile,
		MeasuredRamOrigin, MeasuredStorageOrigin, PEER_BENCHMARK_PROTOCOL_SCHEMA, PROFILE_CODEC_SCHEMA,
		PROFILE_SCHEMA, PeerBenchmarkEvidence, PeerDuplexExecution, PeerEndpointEvidence,
	},
	seed::CONTRACT_SCHEMA,
};

const MAGIC: &[u8; 16] = b"RECIPEPROFILE\0\0\0";
const CHECKSUM_BYTES: usize = 32;
const MAXIMUM_ITEMS: usize = 1_000_000;
const MAXIMUM_STRING_BYTES: usize = 1_048_576;
pub const MAXIMUM_PROFILE_BYTES: usize = 256 * 1024 * 1024;

/// Versioned canonical binary codec for validated measured profiles.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeasuredProfileCodec;

impl MeasuredProfileCodec {
	pub fn encode(profile: &MeasuredProfile) -> ProbeResult<Vec<u8>> {
		validate_profile(profile)?;
		encode_unchecked(profile)
	}

	pub fn decode(bytes: &[u8]) -> ProbeResult<MeasuredProfile> {
		if bytes.len() > MAXIMUM_PROFILE_BYTES {
			return Err(ProbeError::Cache(format!(
				"profile is {} bytes; limit is {MAXIMUM_PROFILE_BYTES}",
				bytes.len()
			)));
		}
		if bytes.len() < MAGIC.len() + 4 + CHECKSUM_BYTES {
			return Err(codec_error("profile is truncated"));
		}
		let payload_len = bytes.len() - CHECKSUM_BYTES;
		let (payload, encoded_checksum) = bytes.split_at(payload_len);
		let expected: [u8; CHECKSUM_BYTES] = Sha256::digest(payload).into();
		if encoded_checksum != expected {
			return Err(codec_error("profile checksum mismatch"));
		}
		let mut decoder = Decoder::new(payload);
		if decoder.raw(MAGIC.len(), "magic")? != MAGIC {
			return Err(codec_error("profile magic mismatch"));
		}
		let codec_schema = decoder.u32("codec_schema")?;
		if codec_schema != PROFILE_CODEC_SCHEMA {
			return Err(codec_error(format!(
				"unsupported codec schema {codec_schema}; expected {PROFILE_CODEC_SCHEMA}"
			)));
		}
		let schema = decoder.u32("profile.schema")?;
		let cache_schema = decoder.u32("cache.schema")?;
		let cache_digest = decoder.digest("cache.digest")?;
		let origins = decode_origins(&mut decoder)?;
		let benchmarks = decode_benchmark_metadata(&mut decoder)?;
		let peer_benchmarks = decode_peer_benchmarks(&mut decoder)?;
		let topology = decode_topology(&mut decoder)?;
		let discovery = decode_discovery(&mut decoder)?;
		if !decoder.is_finished() {
			return Err(codec_error(format!(
				"{} unconsumed payload bytes",
				decoder.remaining()
			)));
		}
		let profile = MeasuredProfile {
			schema,
			cache_identity: CacheIdentity {
				schema: cache_schema,
				digest: cache_digest,
			},
			origins,
			benchmarks,
			peer_benchmarks,
			topology,
			discovery,
		};
		validate_profile(&profile)?;
		Ok(profile)
	}
}

fn encode_unchecked(profile: &MeasuredProfile) -> ProbeResult<Vec<u8>> {
	let mut encoder = Encoder::default();
	encoder.raw(MAGIC);
	encoder.u32(PROFILE_CODEC_SCHEMA);
	encoder.u32(profile.schema);
	encoder.u32(profile.cache_identity.schema);
	encoder.digest(profile.cache_identity.digest);
	encode_origins(&mut encoder, &profile.origins)?;
	encode_benchmark_metadata(&mut encoder, profile.benchmarks)?;
	encode_peer_benchmarks(&mut encoder, &profile.peer_benchmarks)?;
	encode_topology(&mut encoder, &profile.topology)?;
	encode_discovery(&mut encoder, &profile.discovery)?;
	let checksum: [u8; CHECKSUM_BYTES] = Sha256::digest(&encoder.bytes).into();
	encoder.raw(&checksum);
	if encoder.bytes.len() > MAXIMUM_PROFILE_BYTES {
		return Err(ProbeError::Cache(format!(
			"encoded profile is {} bytes; limit is {MAXIMUM_PROFILE_BYTES}",
			encoder.bytes.len()
		)));
	}
	Ok(encoder.bytes)
}

pub fn validate_profile(profile: &MeasuredProfile) -> ProbeResult<()> {
	if profile.schema != PROFILE_SCHEMA {
		return Err(codec_error(format!(
			"profile schema {} is unsupported; expected {PROFILE_SCHEMA}",
			profile.schema
		)));
	}
	if profile.cache_identity.schema != PROFILE_SCHEMA {
		return Err(codec_error(format!(
			"cache schema {} is unsupported; expected {PROFILE_SCHEMA}",
			profile.cache_identity.schema
		)));
	}
	if profile.cache_identity.digest.is_zero() {
		return Err(codec_error("cache identity must not be zero"));
	}
	validate_benchmark_metadata(profile.benchmarks)?;
	validate_peer_benchmarks(profile)?;
	profile
		.topology
		.validate()
		.map_err(|errors| codec_error(format!("invalid topology: {errors}")))?;
	profile
		.topology
		.validate_scheduling_properties()
		.map_err(|errors| codec_error(format!("unschedulable topology: {errors}")))?;
	profile
		.discovery
		.validate(&profile.topology)
		.map_err(|errors| codec_error(format!("invalid discovery profile: {errors}")))?;
	validate_origins(profile)?;
	require_canonical_order(profile)?;
	require_exclusively_measured(profile)?;
	require_topology_discovery_match(profile)?;
	require_peer_topology_match(profile)?;
	Ok(())
}

fn validate_benchmark_metadata(metadata: BenchmarkMetadata) -> ProbeResult<()> {
	if metadata.seed_schema != CONTRACT_SCHEMA {
		return Err(codec_error(format!(
			"benchmark seed schema {} is unsupported; expected {CONTRACT_SCHEMA}",
			metadata.seed_schema
		)));
	}
	for (name, plan) in [
		("ram", metadata.ram),
		("storage", metadata.storage),
		("gpu", metadata.gpu),
		("network", metadata.network),
	] {
		if !plan.is_bounded() {
			return Err(codec_error(format!("{name} benchmark plan is unbounded")));
		}
		u64::try_from(plan.maximum_duration.as_nanos()).map_err(|_| {
			codec_error(format!(
				"{name} benchmark duration cannot be represented canonically"
			))
		})?;
	}
	Ok(())
}

fn validate_peer_benchmarks(profile: &MeasuredProfile) -> ProbeResult<()> {
	let expected_bytes = profile
		.benchmarks
		.network
		.buffer_bytes
		.get()
		.checked_mul(u64::from(profile.benchmarks.network.iterations))
		.ok_or_else(|| codec_error("network benchmark byte count overflowed"))?;
	let maximum_duration = u64::try_from(profile.benchmarks.network.maximum_duration.as_nanos())
		.map_err(|_| codec_error("network benchmark duration exceeds canonical representation"))?;
	for benchmark in &profile.peer_benchmarks {
		let evidence = benchmark.evidence;
		if evidence.protocol_schema != PEER_BENCHMARK_PROTOCOL_SCHEMA {
			return Err(codec_error(format!(
				"peer {} uses benchmark protocol {}, expected {}",
				benchmark.session_id, evidence.protocol_schema, PEER_BENCHMARK_PROTOCOL_SCHEMA
			)));
		}
		for (name, endpoint) in [
			("local", evidence.local_endpoint),
			("remote", evidence.remote_endpoint),
		] {
			if endpoint.machine.is_zero() || endpoint.profile.is_zero() {
				return Err(codec_error(format!(
					"peer {} {name} endpoint identity is zero",
					benchmark.session_id
				)));
			}
		}
		if evidence.local_endpoint.machine == evidence.remote_endpoint.machine {
			return Err(codec_error(format!(
				"peer {} benchmark endpoints identify the same machine",
				benchmark.session_id
			)));
		}
		for (direction, sample, rate) in [
			("outbound", evidence.outbound, benchmark.outbound_rate),
			("inbound", evidence.inbound, benchmark.inbound_rate),
		] {
			validate_directional_evidence(
				benchmark,
				direction,
				sample,
				expected_bytes,
				profile.benchmarks.network.iterations,
				maximum_duration,
			)?;
			let expected_rate = evidence_rate(sample)?;
			if rate.value != expected_rate {
				return Err(codec_error(format!(
					"peer {} {direction} rate {} differs from its duration-derived rate {}",
					benchmark.session_id,
					rate.value.get(),
					expected_rate.get()
				)));
			}
		}
	}
	Ok(())
}

fn validate_directional_evidence(
	benchmark: &MeasuredPeerBenchmark,
	direction: &str,
	evidence: DirectionalBenchmarkEvidence,
	expected_bytes: u64,
	expected_iterations: u32,
	maximum_duration: u64,
) -> ProbeResult<()> {
	let valid = evidence.total_bytes.get() == expected_bytes
		&& evidence.sample_count != 0
		&& evidence.sample_count == expected_iterations
		&& evidence.elapsed_nanoseconds != 0
		&& evidence.elapsed_nanoseconds <= maximum_duration
		&& evidence.minimum_sample_nanoseconds != 0
		&& evidence.minimum_sample_nanoseconds <= evidence.mean_sample_nanoseconds
		&& evidence.mean_sample_nanoseconds <= evidence.maximum_sample_nanoseconds
		&& evidence.maximum_sample_nanoseconds <= evidence.elapsed_nanoseconds;
	if valid {
		Ok(())
	} else {
		Err(codec_error(format!(
			"peer {} {direction} benchmark evidence is inconsistent",
			benchmark.session_id
		)))
	}
}

fn evidence_rate(evidence: DirectionalBenchmarkEvidence) -> ProbeResult<BytesPerSecond> {
	let rate = u128::from(evidence.total_bytes.get())
		.checked_mul(1_000_000_000)
		.and_then(|bytes| bytes.checked_div(u128::from(evidence.elapsed_nanoseconds)))
		.ok_or_else(|| codec_error("peer benchmark rate calculation overflowed"))?
		.clamp(1, u128::from(u64::MAX));
	BytesPerSecond::new(u64::try_from(rate).map_err(|_| codec_error("peer benchmark rate does not fit u64"))?)
		.map_err(|error| codec_error(format!("peer benchmark rate is invalid: {error}")))
}

fn validate_origins(profile: &MeasuredProfile) -> ProbeResult<()> {
	let machines = profile
		.topology
		.machines
		.iter()
		.map(|machine| (machine.id, machine))
		.collect::<BTreeMap<_, _>>();
	let mut origin_machines = BTreeSet::new();
	let mut stable_machine_ids = BTreeSet::new();
	for origin in &profile.origins.machines {
		let machine = machines.get(&origin.machine).copied().ok_or_else(|| {
			codec_error(format!(
				"machine origin references unknown machine {}",
				origin.machine
			))
		})?;
		require_origin(
			origin_machines.insert(origin.machine),
			format!("machine {} has duplicate origin metadata", origin.machine),
		)?;
		require_origin(
			machine.name == origin.fingerprint.hostname,
			format!(
				"machine {} name {} differs from fingerprint hostname {}",
				machine.id, machine.name, origin.fingerprint.hostname
			),
		)?;
		require_origin(
			stable_machine_ids.insert(origin.fingerprint.stable_id.clone()),
			format!(
				"stable machine identity {} appears more than once",
				origin.fingerprint.stable_id
			),
		)?;
	}
	for machine in &profile.topology.machines {
		require_origin(
			origin_machines.contains(&machine.id),
			format!("machine {} has no stable fingerprint origin", machine.id),
		)?;
	}

	let devices = profile
		.topology
		.devices
		.iter()
		.map(|device| (device.id, device))
		.collect::<BTreeMap<_, _>>();
	let mut origin_ram = BTreeSet::new();
	let mut scoped_keys = BTreeSet::new();
	for origin in &profile.origins.ram {
		let device = devices.get(&origin.device).copied().ok_or_else(|| {
			codec_error(format!(
				"RAM origin references unknown device {}",
				origin.device
			))
		})?;
		require_origin(
			device.kind == DeviceKind::Ram,
			format!(
				"device {} has RAM origin metadata but is {:?}",
				device.id, device.kind
			),
		)?;
		require_origin(
			origin_ram.insert(origin.device),
			format!("RAM device {} has duplicate origin metadata", origin.device),
		)?;
		require_origin(
			scoped_keys.insert((device.machine, origin.key.clone())),
			format!(
				"RAM origin key {} appears more than once on machine {}",
				origin.key, device.machine
			),
		)?;
	}
	for device in profile
		.topology
		.devices
		.iter()
		.filter(|device| device.kind == DeviceKind::Ram)
	{
		require_origin(
			origin_ram.contains(&device.id),
			format!("RAM device {} has no stable domain origin", device.id),
		)?;
	}

	let mut origin_storage = BTreeSet::new();
	let mut storage_keys = BTreeSet::new();
	for origin in &profile.origins.storage {
		let device = devices.get(&origin.device).copied().ok_or_else(|| {
			codec_error(format!(
				"storage origin references unknown device {}",
				origin.device
			))
		})?;
		require_origin(
			device.kind == DeviceKind::Disk,
			format!(
				"device {} has storage origin metadata but is {:?}",
				device.id, device.kind
			),
		)?;
		require_origin(
			origin_storage.insert(origin.device),
			format!(
				"storage device {} has duplicate origin metadata",
				origin.device
			),
		)?;
		require_origin(
			storage_keys.insert((device.machine, origin.key.clone())),
			format!(
				"storage origin key {} appears more than once on machine {}",
				origin.key, device.machine
			),
		)?;
	}
	for device in profile
		.topology
		.devices
		.iter()
		.filter(|device| device.kind == DeviceKind::Disk)
	{
		require_origin(
			origin_storage.contains(&device.id),
			format!("storage device {} has no stable domain origin", device.id),
		)?;
	}

	let mut origin_gpu = BTreeSet::new();
	let mut gpu_keys = BTreeSet::new();
	for origin in &profile.origins.gpu {
		let device = devices.get(&origin.device).copied().ok_or_else(|| {
			codec_error(format!(
				"GPU origin references unknown device {}",
				origin.device
			))
		})?;
		require_origin(
			device.kind == DeviceKind::GpuMemory,
			format!(
				"device {} has GPU origin metadata but is {:?}",
				device.id, device.kind
			),
		)?;
		require_origin(
			origin_gpu.insert(origin.device),
			format!("GPU device {} has duplicate origin metadata", origin.device),
		)?;
		require_origin(
			gpu_keys.insert((device.machine, origin.key.clone())),
			format!(
				"GPU origin key {} appears more than once on machine {}",
				origin.key, device.machine
			),
		)?;
	}
	for device in profile
		.topology
		.devices
		.iter()
		.filter(|device| device.kind == DeviceKind::GpuMemory)
	{
		require_origin(
			origin_gpu.contains(&device.id),
			format!("GPU device {} has no stable domain origin", device.id),
		)?;
	}
	Ok(())
}

fn require_origin(condition: bool, message: String) -> ProbeResult<()> {
	match condition {
		true => Ok(()),
		false => Err(codec_error(message)),
	}
}

fn require_canonical_order(profile: &MeasuredProfile) -> ProbeResult<()> {
	require_increasing(
		"origins.machines",
		profile.origins.machines.iter().map(|value| value.machine),
	)?;
	require_increasing(
		"origins.ram",
		profile.origins.ram.iter().map(|value| value.device),
	)?;
	require_increasing(
		"origins.storage",
		profile.origins.storage.iter().map(|value| value.device),
	)?;
	require_increasing(
		"origins.gpu",
		profile.origins.gpu.iter().map(|value| value.device),
	)?;
	require_increasing_ref(
		"peer_benchmarks",
		profile
			.peer_benchmarks
			.iter()
			.map(|value| &value.session_id),
	)?;
	require_increasing(
		"topology.machines",
		profile.topology.machines.iter().map(|value| value.id),
	)?;
	require_increasing(
		"topology.nodes",
		profile.topology.nodes.iter().map(|value| value.id),
	)?;
	require_increasing(
		"topology.devices",
		profile.topology.devices.iter().map(|value| value.id),
	)?;
	require_increasing(
		"topology.links",
		profile.topology.links.iter().map(|value| value.id),
	)?;
	for node in &profile.topology.nodes {
		require_increasing(
			&format!("topology.node[{}].devices", node.id),
			node.devices.iter().copied(),
		)?;
	}
	require_increasing(
		"discovery.devices",
		profile.discovery.devices.iter().map(|value| value.device),
	)?;
	require_increasing(
		"discovery.links",
		profile.discovery.links.iter().map(|value| value.link),
	)?;
	Ok(())
}

fn require_increasing<T: Ord + Copy>(name: &str, values: impl Iterator<Item = T>) -> ProbeResult<()> {
	let mut previous = None;
	for value in values {
		if previous.is_some_and(|previous| previous >= value) {
			return Err(codec_error(format!(
				"{name} is not in strict canonical order"
			)));
		}
		previous = Some(value);
	}
	Ok(())
}

fn require_increasing_ref<'a, T: Ord + ?Sized + 'a>(
	name: &str,
	values: impl Iterator<Item = &'a T>,
) -> ProbeResult<()> {
	let mut previous = None;
	for value in values {
		if previous.is_some_and(|previous| previous >= value) {
			return Err(codec_error(format!(
				"{name} is not in strict canonical order"
			)));
		}
		previous = Some(value);
	}
	Ok(())
}

fn require_exclusively_measured(profile: &MeasuredProfile) -> ProbeResult<()> {
	for benchmark in &profile.peer_benchmarks {
		require_measured(
			benchmark.outbound_rate.provenance,
			&format!("peer {} outbound rate", benchmark.session_id),
		)?;
		require_measured(
			benchmark.inbound_rate.provenance,
			&format!("peer {} inbound rate", benchmark.session_id),
		)?;
	}
	for device in &profile.topology.devices {
		require_measured(
			device.capacity.provenance,
			&format!("topology device {} capacity", device.id),
		)?;
		require_measured(
			device.transfer_rate.provenance,
			&format!("topology device {} transfer rate", device.id),
		)?;
		if let Some(rate) = device.calculation_rate {
			require_measured(
				rate.provenance,
				&format!("topology device {} calculation rate", device.id),
			)?;
		}
	}
	for link in &profile.topology.links {
		require_measured(
			link.bandwidth.provenance,
			&format!("topology link {} bandwidth", link.id),
		)?;
		require_measured(
			link.maximum_inflight_transfers.provenance,
			&format!("topology link {} concurrency", link.id),
		)?;
	}
	for device in &profile.discovery.devices {
		require_measured(
			device.total_capacity.provenance,
			&format!("discovery device {} capacity", device.device),
		)?;
		require_measured(
			device.transfer.rate.provenance,
			&format!("discovery device {} transfer rate", device.device),
		)?;
		require_measured(
			device.transfer.maximum_inflight_transfers.provenance,
			&format!("discovery device {} transfer concurrency", device.device),
		)?;
		if let Some(calculation) = &device.calculation {
			require_measured(
				calculation.rate.provenance,
				&format!("discovery device {} calculation rate", device.device),
			)?;
		}
	}
	for link in &profile.discovery.links {
		require_measured(
			link.bandwidth.provenance,
			&format!("discovery link {} bandwidth", link.link),
		)?;
		require_measured(
			link.maximum_inflight_transfers.provenance,
			&format!("discovery link {} concurrency", link.link),
		)?;
	}
	Ok(())
}

fn require_measured(provenance: PropertyProvenance, name: &str) -> ProbeResult<()> {
	if provenance == PropertyProvenance::Measured {
		Ok(())
	} else {
		Err(codec_error(format!(
			"{name} has {provenance:?} provenance; persisted probe profiles require Measured"
		)))
	}
}

fn require_topology_discovery_match(profile: &MeasuredProfile) -> ProbeResult<()> {
	for topology_device in &profile.topology.devices {
		let discovered = profile
			.discovery
			.devices
			.iter()
			.find(|value| value.device == topology_device.id)
			.ok_or_else(|| {
				codec_error(format!(
					"topology device {} is absent from discovery",
					topology_device.id
				))
			})?;
		if topology_device.capacity != discovered.total_capacity
			|| topology_device.transfer_rate != discovered.transfer.rate
		{
			return Err(codec_error(format!(
				"topology/discovery properties differ for device {}",
				topology_device.id
			)));
		}
		match (
			topology_device.calculation_rate,
			discovered.calculation.as_ref(),
		) {
			(None, None) => {}
			(Some(rate), Some(calculation)) if rate == calculation.rate => {}
			_ => {
				return Err(codec_error(format!(
					"topology/discovery calculation properties differ for device {}",
					topology_device.id
				)));
			}
		}
	}
	for topology_link in &profile.topology.links {
		let discovered = profile
			.discovery
			.links
			.iter()
			.find(|value| value.link == topology_link.id)
			.ok_or_else(|| {
				codec_error(format!(
					"topology link {} is absent from discovery",
					topology_link.id
				))
			})?;
		if topology_link.bandwidth != discovered.bandwidth
			|| topology_link.maximum_inflight_transfers != discovered.maximum_inflight_transfers
		{
			return Err(codec_error(format!(
				"topology/discovery directional properties differ for link {}",
				topology_link.id
			)));
		}
	}
	Ok(())
}

fn require_peer_topology_match(profile: &MeasuredProfile) -> ProbeResult<()> {
	let devices = profile
		.topology
		.devices
		.iter()
		.map(|device| (device.id, device))
		.collect::<BTreeMap<_, _>>();
	let mut transports = BTreeMap::<TransportId, Vec<&DirectedLink>>::new();
	for link in &profile.topology.links {
		let from = devices
			.get(&link.from)
			.ok_or_else(|| codec_error(format!("link {} has no source device", link.id)))?;
		let to = devices
			.get(&link.to)
			.ok_or_else(|| codec_error(format!("link {} has no destination device", link.id)))?;
		if from.machine != to.machine {
			transports.entry(link.transport).or_default().push(link);
		}
	}
	let mut peer_transports = Vec::with_capacity(transports.len());
	for (transport, directions) in transports {
		if directions.len() != 2 {
			return Err(codec_error(format!(
				"cross-machine transport {transport} has {} directions; exactly two are required",
				directions.len()
			)));
		}
		let forward = directions[0];
		let reverse = directions[1];
		if forward.from != reverse.to
			|| forward.to != reverse.from
			|| forward.kind != reverse.kind
			|| forward.duplex != reverse.duplex
		{
			return Err(codec_error(format!(
				"cross-machine transport {transport} is not one opposing directional pair"
			)));
		}
		peer_transports.push((forward, reverse));
	}
	if peer_transports.len() != profile.peer_benchmarks.len() {
		return Err(codec_error(format!(
			"{} cross-machine transports have {} peer benchmark records",
			peer_transports.len(),
			profile.peer_benchmarks.len()
		)));
	}

	let mut endpoint_pairs = BTreeSet::new();
	let mut used_transports = vec![false; peer_transports.len()];
	for benchmark in &profile.peer_benchmarks {
		let local = (
			benchmark.evidence.local_endpoint.machine,
			benchmark.evidence.local_endpoint.profile,
		);
		let remote = (
			benchmark.evidence.remote_endpoint.machine,
			benchmark.evidence.remote_endpoint.profile,
		);
		let pair = match local.cmp(&remote) {
			core::cmp::Ordering::Less => (local, remote),
			core::cmp::Ordering::Greater => (remote, local),
			core::cmp::Ordering::Equal => {
				return Err(codec_error(format!(
					"peer {} repeats one endpoint on both sides",
					benchmark.session_id
				)));
			}
		};
		if !endpoint_pairs.insert(pair) {
			return Err(codec_error(format!(
				"peer {} repeats an authenticated endpoint pair",
				benchmark.session_id
			)));
		}

		let matching = peer_transports
			.iter()
			.enumerate()
			.find(|(index, (forward, reverse))| {
				if used_transports[*index] {
					return false;
				}
				let rates_match = (forward.bandwidth == benchmark.outbound_rate
					&& reverse.bandwidth == benchmark.inbound_rate)
					|| (reverse.bandwidth == benchmark.outbound_rate
						&& forward.bandwidth == benchmark.inbound_rate);
				let execution_matches = match benchmark.evidence.execution {
					PeerDuplexExecution::Simultaneous => {
						forward.duplex == DuplexMode::Full
							&& forward.capacity_resource != reverse.capacity_resource
					}
					PeerDuplexExecution::Serialized => {
						forward.duplex == DuplexMode::Half
							&& forward.capacity_resource == reverse.capacity_resource
					}
				};
				rates_match && execution_matches
			})
			.map(|(index, _)| index)
			.ok_or_else(|| {
				codec_error(format!(
					"peer {} has no cross-machine transport matching its measured rates and duplex evidence",
					benchmark.session_id
				))
			})?;
		used_transports[matching] = true;
	}
	Ok(())
}

fn encode_origins(encoder: &mut Encoder, origins: &MeasuredOrigins) -> ProbeResult<()> {
	encoder.length(origins.machines.len(), "origins.machines")?;
	for origin in &origins.machines {
		encoder.u64(origin.machine.get());
		encoder.label(&origin.fingerprint.hostname)?;
		encoder.label(&origin.fingerprint.stable_id)?;
		encoder.label(&origin.fingerprint.runtime_abi)?;
		encoder.label(&origin.fingerprint.firmware)?;
	}
	encoder.length(origins.ram.len(), "origins.ram")?;
	for origin in &origins.ram {
		encoder.u64(origin.device.get());
		encoder.label(&origin.key)?;
	}
	encoder.length(origins.storage.len(), "origins.storage")?;
	for origin in &origins.storage {
		encoder.u64(origin.device.get());
		encoder.label(&origin.key)?;
	}
	encoder.length(origins.gpu.len(), "origins.gpu")?;
	for origin in &origins.gpu {
		encoder.u64(origin.device.get());
		encoder.label(&origin.key)?;
	}
	Ok(())
}

fn decode_origins(decoder: &mut Decoder<'_>) -> ProbeResult<MeasuredOrigins> {
	let machine_count = decoder.length("origins.machines")?;
	let mut machines = Vec::with_capacity(machine_count);
	for index in 0..machine_count {
		machines.push(MeasuredMachineOrigin {
			machine: MachineId::new(decoder.u64(&format!("origins.machines[{index}].machine"))?),
			fingerprint: MachineFingerprint {
				hostname: decoder.label(&format!("origins.machines[{index}].hostname"))?,
				stable_id: decoder.label(&format!("origins.machines[{index}].stable_id"))?,
				runtime_abi: decoder.label(&format!("origins.machines[{index}].runtime_abi"))?,
				firmware: decoder.label(&format!("origins.machines[{index}].firmware"))?,
			},
		});
	}
	let ram_count = decoder.length("origins.ram")?;
	let mut ram = Vec::with_capacity(ram_count);
	for index in 0..ram_count {
		ram.push(MeasuredRamOrigin {
			device: DeviceId::new(decoder.u64(&format!("origins.ram[{index}].device"))?),
			key: decoder.label(&format!("origins.ram[{index}].key"))?,
		});
	}
	let storage_count = decoder.length("origins.storage")?;
	let mut storage = Vec::with_capacity(storage_count);
	for index in 0..storage_count {
		storage.push(MeasuredStorageOrigin {
			device: DeviceId::new(decoder.u64(&format!("origins.storage[{index}].device"))?),
			key: decoder.label(&format!("origins.storage[{index}].key"))?,
		});
	}
	let gpu_count = decoder.length("origins.gpu")?;
	let mut gpu = Vec::with_capacity(gpu_count);
	for index in 0..gpu_count {
		gpu.push(MeasuredGpuOrigin {
			device: DeviceId::new(decoder.u64(&format!("origins.gpu[{index}].device"))?),
			key: decoder.label(&format!("origins.gpu[{index}].key"))?,
		});
	}
	Ok(MeasuredOrigins {
		machines,
		ram,
		storage,
		gpu,
	})
}

fn encode_benchmark_metadata(encoder: &mut Encoder, metadata: BenchmarkMetadata) -> ProbeResult<()> {
	encoder.u32(metadata.seed_schema);
	for plan in [
		metadata.ram,
		metadata.storage,
		metadata.gpu,
		metadata.network,
	] {
		encoder.u64(plan.buffer_bytes.get());
		encoder.u32(plan.iterations);
		encoder.u64(u64::try_from(plan.maximum_duration.as_nanos())
			.map_err(|_| codec_error("benchmark duration exceeds canonical representation"))?);
	}
	Ok(())
}

fn decode_benchmark_metadata(decoder: &mut Decoder<'_>) -> ProbeResult<BenchmarkMetadata> {
	let seed_schema = decoder.u32("benchmarks.seed_schema")?;
	let mut plan = |name: &str| -> ProbeResult<BoundedBenchmarkPlan> {
		Ok(BoundedBenchmarkPlan {
			buffer_bytes: ByteCount::new(decoder.u64(&format!("{name}.buffer_bytes"))?),
			iterations: decoder.u32(&format!("{name}.iterations"))?,
			maximum_duration: Duration::from_nanos(decoder.u64(&format!("{name}.maximum_duration_ns"))?),
		})
	};
	Ok(BenchmarkMetadata {
		seed_schema,
		ram: plan("benchmarks.ram")?,
		storage: plan("benchmarks.storage")?,
		gpu: plan("benchmarks.gpu")?,
		network: plan("benchmarks.network")?,
	})
}

fn encode_peer_benchmarks(encoder: &mut Encoder, benchmarks: &[MeasuredPeerBenchmark]) -> ProbeResult<()> {
	encoder.length(benchmarks.len(), "peer_benchmarks")?;
	for benchmark in benchmarks {
		encoder.label(&benchmark.session_id)?;
		encoder.property_rate(benchmark.outbound_rate);
		encoder.property_rate(benchmark.inbound_rate);
		encode_peer_evidence(encoder, benchmark.evidence);
	}
	Ok(())
}

fn encode_peer_evidence(encoder: &mut Encoder, evidence: PeerBenchmarkEvidence) {
	encoder.u16(evidence.protocol_schema);
	for endpoint in [evidence.local_endpoint, evidence.remote_endpoint] {
		encoder.digest(endpoint.machine);
		encoder.digest(endpoint.profile);
	}
	encoder.u8(match evidence.execution {
		PeerDuplexExecution::Simultaneous => 0,
		PeerDuplexExecution::Serialized => 1,
	});
	for direction in [evidence.outbound, evidence.inbound] {
		encoder.u64(direction.total_bytes.get());
		encoder.u64(direction.elapsed_nanoseconds);
		encoder.u32(direction.sample_count);
		encoder.u64(direction.minimum_sample_nanoseconds);
		encoder.u64(direction.maximum_sample_nanoseconds);
		encoder.u64(direction.mean_sample_nanoseconds);
		encoder.u128(direction.variance_nanoseconds_squared);
	}
}

fn decode_peer_benchmarks(decoder: &mut Decoder<'_>) -> ProbeResult<Vec<MeasuredPeerBenchmark>> {
	let count = decoder.length("peer_benchmarks")?;
	let mut benchmarks = Vec::with_capacity(count);
	for index in 0..count {
		benchmarks.push(MeasuredPeerBenchmark {
			session_id: decoder.label(&format!("peer_benchmarks[{index}].session_id"))?,
			outbound_rate: decoder.property_rate(&format!("peer_benchmarks[{index}].outbound_rate"))?,
			inbound_rate: decoder.property_rate(&format!("peer_benchmarks[{index}].inbound_rate"))?,
			evidence: decode_peer_evidence(decoder, index)?,
		});
	}
	Ok(benchmarks)
}

fn decode_peer_evidence(decoder: &mut Decoder<'_>, index: usize) -> ProbeResult<PeerBenchmarkEvidence> {
	let prefix = format!("peer_benchmarks[{index}].evidence");
	let protocol_schema = decoder.u16(&format!("{prefix}.protocol_schema"))?;
	let mut endpoint = |name: &str| -> ProbeResult<PeerEndpointEvidence> {
		Ok(PeerEndpointEvidence {
			machine: decoder.digest(&format!("{prefix}.{name}.machine"))?,
			profile: decoder.digest(&format!("{prefix}.{name}.profile"))?,
		})
	};
	let local_endpoint = endpoint("local_endpoint")?;
	let remote_endpoint = endpoint("remote_endpoint")?;
	let execution = match decoder.u8(&format!("{prefix}.execution"))? {
		0 => PeerDuplexExecution::Simultaneous,
		1 => PeerDuplexExecution::Serialized,
		value => return Err(codec_error(format!("invalid peer execution tag {value}"))),
	};
	let mut direction = |name: &str| -> ProbeResult<DirectionalBenchmarkEvidence> {
		Ok(DirectionalBenchmarkEvidence {
			total_bytes: ByteCount::new(decoder.u64(&format!("{prefix}.{name}.total_bytes"))?),
			elapsed_nanoseconds: decoder.u64(&format!("{prefix}.{name}.elapsed_nanoseconds"))?,
			sample_count: decoder.u32(&format!("{prefix}.{name}.sample_count"))?,
			minimum_sample_nanoseconds: decoder.u64(&format!("{prefix}.{name}.minimum_sample_nanoseconds"))?,
			maximum_sample_nanoseconds: decoder.u64(&format!("{prefix}.{name}.maximum_sample_nanoseconds"))?,
			mean_sample_nanoseconds: decoder.u64(&format!("{prefix}.{name}.mean_sample_nanoseconds"))?,
			variance_nanoseconds_squared: decoder
				.u128(&format!("{prefix}.{name}.variance_nanoseconds_squared"))?,
		})
	};
	Ok(PeerBenchmarkEvidence {
		protocol_schema,
		local_endpoint,
		remote_endpoint,
		execution,
		outbound: direction("outbound")?,
		inbound: direction("inbound")?,
	})
}

const fn transport_kind_tag(kind: TransportKind) -> u8 {
	match kind {
		TransportKind::Memory => 0,
		TransportKind::Pcie => 1,
		TransportKind::Sata => 2,
		TransportKind::Sas => 3,
		TransportKind::Nvme => 4,
		TransportKind::Ethernet => 5,
		TransportKind::Wlan => 6,
	}
}

fn decode_transport_kind(tag: u8) -> ProbeResult<TransportKind> {
	match tag {
		0 => Ok(TransportKind::Memory),
		1 => Ok(TransportKind::Pcie),
		2 => Ok(TransportKind::Sata),
		3 => Ok(TransportKind::Sas),
		4 => Ok(TransportKind::Nvme),
		5 => Ok(TransportKind::Ethernet),
		6 => Ok(TransportKind::Wlan),
		value => Err(codec_error(format!("invalid transport-kind tag {value}"))),
	}
}

const fn duplex_tag(duplex: DuplexMode) -> u8 {
	match duplex {
		DuplexMode::Half => 0,
		DuplexMode::Full => 1,
	}
}

fn decode_duplex(tag: u8) -> ProbeResult<DuplexMode> {
	match tag {
		0 => Ok(DuplexMode::Half),
		1 => Ok(DuplexMode::Full),
		value => Err(codec_error(format!("invalid duplex tag {value}"))),
	}
}

fn encode_topology(encoder: &mut Encoder, topology: &Topology) -> ProbeResult<()> {
	encoder.digest(topology.identity.digest());
	encoder.length(topology.machines.len(), "topology.machines")?;
	for machine in &topology.machines {
		encoder.u64(machine.id.get());
		encoder.label(&machine.name)?;
	}
	encoder.length(topology.nodes.len(), "topology.nodes")?;
	for node in &topology.nodes {
		encoder.u64(node.id.get());
		encoder.u8(match node.role {
			NodeRole::Master => 0,
			NodeRole::Worker => 1,
		});
		encoder.u64(node.machine.get());
		encoder.length(node.devices.len(), "node.devices")?;
		for device in &node.devices {
			encoder.u64(device.get());
		}
	}
	encoder.length(topology.devices.len(), "topology.devices")?;
	for device in &topology.devices {
		encoder.u64(device.id.get());
		encoder.u64(device.machine.get());
		encoder.u8(match device.kind {
			DeviceKind::GpuMemory => 0,
			DeviceKind::Ram => 1,
			DeviceKind::Disk => 2,
		});
		encoder.property_bytes(device.capacity);
		encoder.property_rate(device.transfer_rate);
		encoder.bool(device.calculation_rate.is_some());
		if let Some(rate) = device.calculation_rate {
			encoder.property_flops(rate);
		}
	}
	encoder.length(topology.links.len(), "topology.links")?;
	for link in &topology.links {
		encoder.u64(link.id.get());
		encoder.u64(link.transport.get());
		encoder.u8(transport_kind_tag(link.kind));
		encoder.u8(duplex_tag(link.duplex));
		encoder.u64(link.from.get());
		encoder.u64(link.to.get());
		encoder.property_rate(link.bandwidth);
		encoder.property_lanes(link.maximum_inflight_transfers);
		encoder.u64(link.capacity_resource.get());
	}
	Ok(())
}

fn decode_topology(decoder: &mut Decoder<'_>) -> ProbeResult<Topology> {
	let identity = TopologyIdentity::new(decoder.digest("topology.identity")?);
	let machine_count = decoder.length("topology.machines")?;
	let mut machines = Vec::with_capacity(machine_count);
	for index in 0..machine_count {
		machines.push(Machine {
			id: MachineId::new(decoder.u64(&format!("machines[{index}].id"))?),
			name: decoder.label(&format!("machines[{index}].name"))?,
		});
	}
	let node_count = decoder.length("topology.nodes")?;
	let mut nodes = Vec::with_capacity(node_count);
	for index in 0..node_count {
		let id = NodeId::new(decoder.u64(&format!("nodes[{index}].id"))?);
		let role = match decoder.u8(&format!("nodes[{index}].role"))? {
			0 => NodeRole::Master,
			1 => NodeRole::Worker,
			tag => return Err(codec_error(format!("invalid node role tag {tag}"))),
		};
		let machine = MachineId::new(decoder.u64(&format!("nodes[{index}].machine"))?);
		let device_count = decoder.length(&format!("nodes[{index}].devices"))?;
		let mut devices = Vec::with_capacity(device_count);
		for device_index in 0..device_count {
			devices.push(DeviceId::new(
				decoder.u64(&format!("nodes[{index}].devices[{device_index}]"))?,
			));
		}
		nodes.push(Node {
			id,
			role,
			machine,
			devices,
		});
	}
	let device_count = decoder.length("topology.devices")?;
	let mut devices = Vec::with_capacity(device_count);
	for index in 0..device_count {
		let id = DeviceId::new(decoder.u64(&format!("devices[{index}].id"))?);
		let machine = MachineId::new(decoder.u64(&format!("devices[{index}].machine"))?);
		let kind = match decoder.u8(&format!("devices[{index}].kind"))? {
			0 => DeviceKind::GpuMemory,
			1 => DeviceKind::Ram,
			2 => DeviceKind::Disk,
			tag => return Err(codec_error(format!("invalid device-kind tag {tag}"))),
		};
		let capacity = decoder.property_bytes(&format!("devices[{index}].capacity"))?;
		let transfer_rate = decoder.property_rate(&format!("devices[{index}].transfer_rate"))?;
		let calculation_rate = if decoder.bool(&format!("devices[{index}].has_calculation_rate"))? {
			Some(decoder.property_flops(&format!("devices[{index}].calculation_rate"))?)
		} else {
			None
		};
		devices.push(Device {
			id,
			machine,
			kind,
			capacity,
			transfer_rate,
			calculation_rate,
		});
	}
	let link_count = decoder.length("topology.links")?;
	let mut links = Vec::with_capacity(link_count);
	for index in 0..link_count {
		links.push(DirectedLink {
			id: LinkId::new(decoder.u64(&format!("links[{index}].id"))?),
			transport: TransportId::new(decoder.u64(&format!("links[{index}].transport"))?),
			kind: decode_transport_kind(decoder.u8(&format!("links[{index}].kind"))?)?,
			duplex: decode_duplex(decoder.u8(&format!("links[{index}].duplex"))?)?,
			from: DeviceId::new(decoder.u64(&format!("links[{index}].from"))?),
			to: DeviceId::new(decoder.u64(&format!("links[{index}].to"))?),
			bandwidth: decoder.property_rate(&format!("links[{index}].bandwidth"))?,
			maximum_inflight_transfers: decoder
				.property_lanes(&format!("links[{index}].maximum_inflight_transfers"))?,
			capacity_resource: DuplexResourceId::new(decoder.u64(&format!("links[{index}].capacity_resource"))?),
		});
	}
	Ok(Topology {
		identity,
		machines,
		nodes,
		devices,
		links,
	})
}

fn encode_discovery(encoder: &mut Encoder, discovery: &DiscoveryProfile) -> ProbeResult<()> {
	encoder.digest(discovery.identity.digest());
	encoder.digest(discovery.topology.digest());
	encoder.length(discovery.devices.len(), "discovery.devices")?;
	for device in &discovery.devices {
		encoder.u64(device.device.get());
		encoder.bool(device.available);
		encoder.u32(device.maximum_submission_queues);
		encoder.property_bytes(device.total_capacity);
		encoder.property_rate(device.transfer.rate);
		encoder.property_lanes(device.transfer.maximum_inflight_transfers);
		encoder.bool(device.transfer.asynchronous_submission);
		encoder.bool(device.transfer.overlaps_calculation);
		encoder.bool(device.calculation.is_some());
		if let Some(calculation) = &device.calculation {
			encoder.label(&calculation.target.backend)?;
			encoder.label(&calculation.target.architecture)?;
			encoder.label(&calculation.target.abi)?;
			encoder.property_flops(calculation.rate);
			encoder.bool(calculation.asynchronous_submission);
			encoder.u32(calculation.maximum_concurrent_tasks);
			encoder.u32(calculation.subgroup_lanes);
			encoder.u32(calculation.maximum_workgroup_lanes);
			encoder.u64(calculation.maximum_shared_memory_per_workgroup.get());
		}
	}
	encoder.length(discovery.links.len(), "discovery.links")?;
	for link in &discovery.links {
		encoder.u64(link.link.get());
		encoder.bool(link.available);
		encoder.property_rate(link.bandwidth);
		encoder.property_lanes(link.maximum_inflight_transfers);
		encoder.bool(link.asynchronous_submission);
	}
	Ok(())
}

fn decode_discovery(decoder: &mut Decoder<'_>) -> ProbeResult<DiscoveryProfile> {
	let identity = DiscoveryIdentity::new(decoder.digest("discovery.identity")?);
	let topology = TopologyIdentity::new(decoder.digest("discovery.topology")?);
	let device_count = decoder.length("discovery.devices")?;
	let mut devices = Vec::with_capacity(device_count);
	for index in 0..device_count {
		let device = DeviceId::new(decoder.u64(&format!("discovery.devices[{index}].id"))?);
		let available = decoder.bool(&format!("discovery.devices[{index}].available"))?;
		let maximum_submission_queues = decoder.u32(&format!(
			"discovery.devices[{index}].maximum_submission_queues"
		))?;
		let total_capacity = decoder.property_bytes(&format!("discovery.devices[{index}].capacity"))?;
		let transfer = TransferCapability {
			rate: decoder.property_rate(&format!("discovery.devices[{index}].transfer.rate"))?,
			maximum_inflight_transfers: decoder.property_lanes(&format!(
				"discovery.devices[{index}].transfer.maximum_inflight"
			))?,
			asynchronous_submission: decoder
				.bool(&format!("discovery.devices[{index}].transfer.asynchronous"))?,
			overlaps_calculation: decoder.bool(&format!("discovery.devices[{index}].transfer.overlap"))?,
		};
		let calculation = if decoder.bool(&format!("discovery.devices[{index}].has_calculation"))? {
			Some(CalculationCapability {
				target: TargetIdentity {
					backend: decoder.label(&format!("discovery.devices[{index}].target.backend"))?,
					architecture: decoder
						.label(&format!("discovery.devices[{index}].target.architecture"))?,
					abi: decoder.label(&format!("discovery.devices[{index}].target.abi"))?,
				},
				rate: decoder.property_flops(&format!("discovery.devices[{index}].calculation.rate"))?,
				asynchronous_submission: decoder.bool(&format!(
					"discovery.devices[{index}].calculation.asynchronous"
				))?,
				maximum_concurrent_tasks: decoder.u32(&format!(
					"discovery.devices[{index}].calculation.maximum_concurrent"
				))?,
				subgroup_lanes: decoder.u32(&format!(
					"discovery.devices[{index}].calculation.subgroup_lanes"
				))?,
				maximum_workgroup_lanes: decoder.u32(&format!(
					"discovery.devices[{index}].calculation.maximum_workgroup_lanes"
				))?,
				maximum_shared_memory_per_workgroup: ByteCount::new(decoder.u64(&format!(
					"discovery.devices[{index}].calculation.maximum_shared_memory_per_workgroup"
				))?),
			})
		} else {
			None
		};
		devices.push(DiscoveredDevice {
			device,
			available,
			maximum_submission_queues,
			total_capacity,
			transfer,
			calculation,
		});
	}
	let link_count = decoder.length("discovery.links")?;
	let mut links = Vec::with_capacity(link_count);
	for index in 0..link_count {
		links.push(DiscoveredLink {
			link: LinkId::new(decoder.u64(&format!("discovery.links[{index}].id"))?),
			available: decoder.bool(&format!("discovery.links[{index}].available"))?,
			bandwidth: decoder.property_rate(&format!("discovery.links[{index}].bandwidth"))?,
			maximum_inflight_transfers: decoder
				.property_lanes(&format!("discovery.links[{index}].maximum_inflight"))?,
			asynchronous_submission: decoder.bool(&format!("discovery.links[{index}].asynchronous"))?,
		});
	}
	Ok(DiscoveryProfile {
		identity,
		topology,
		devices,
		links,
	})
}

#[derive(Debug, Default)]
struct Encoder {
	bytes: Vec<u8>,
}

impl Encoder {
	fn raw(&mut self, bytes: &[u8]) { self.bytes.extend_from_slice(bytes); }

	fn u8(&mut self, value: u8) { self.bytes.push(value); }

	fn bool(&mut self, value: bool) { self.u8(u8::from(value)); }

	fn u16(&mut self, value: u16) { self.raw(&value.to_le_bytes()); }

	fn u32(&mut self, value: u32) { self.raw(&value.to_le_bytes()); }

	fn u64(&mut self, value: u64) { self.raw(&value.to_le_bytes()); }

	fn u128(&mut self, value: u128) { self.raw(&value.to_le_bytes()); }

	fn digest(&mut self, value: Digest) { self.raw(&value.bytes()); }

	fn length(&mut self, value: usize, name: &str) -> ProbeResult<()> {
		if value > MAXIMUM_ITEMS {
			return Err(codec_error(format!(
				"{name} contains {value} items; limit is {MAXIMUM_ITEMS}"
			)));
		}
		self.u32(u32::try_from(value).map_err(|_| codec_error(format!("{name} length does not fit u32")))?);
		Ok(())
	}

	fn label(&mut self, value: &Label) -> ProbeResult<()> {
		let bytes = value.as_str().as_bytes();
		if bytes.len() > MAXIMUM_STRING_BYTES {
			return Err(codec_error("label exceeds canonical length limit"));
		}
		self.u32(u32::try_from(bytes.len()).map_err(|_| codec_error("label length does not fit u32"))?);
		self.raw(bytes);
		Ok(())
	}

	fn provenance(&mut self, value: PropertyProvenance) {
		self.u8(match value {
			PropertyProvenance::Estimated => 0,
			PropertyProvenance::Measured => 1,
			PropertyProvenance::Override => 2,
		});
	}

	fn property_bytes(&mut self, value: Property<ByteCount>) {
		self.u64(value.value.get());
		self.provenance(value.provenance);
	}

	fn property_rate(&mut self, value: Property<BytesPerSecond>) {
		self.u64(value.value.get());
		self.provenance(value.provenance);
	}

	fn property_lanes(&mut self, value: Property<TransferLaneCount>) {
		self.u32(value.value.get());
		self.provenance(value.provenance);
	}

	fn property_flops(&mut self, value: Property<FlopsPerSecond>) {
		self.u64(value.value.get());
		self.provenance(value.provenance);
	}
}

#[derive(Debug)]
struct Decoder<'a> {
	bytes: &'a [u8],
	position: usize,
}

impl<'a> Decoder<'a> {
	const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

	fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.position) }

	fn is_finished(&self) -> bool { self.position == self.bytes.len() }

	fn raw(&mut self, count: usize, name: &str) -> ProbeResult<&'a [u8]> {
		let end = self
			.position
			.checked_add(count)
			.ok_or_else(|| codec_error(format!("{name} offset overflow")))?;
		let result = self
			.bytes
			.get(self.position..end)
			.ok_or_else(|| codec_error(format!("{name} is truncated")))?;
		self.position = end;
		Ok(result)
	}

	fn u8(&mut self, name: &str) -> ProbeResult<u8> { Ok(self.raw(1, name)?[0]) }

	fn bool(&mut self, name: &str) -> ProbeResult<bool> {
		match self.u8(name)? {
			0 => Ok(false),
			1 => Ok(true),
			value => {
				Err(codec_error(format!(
					"{name} has invalid boolean tag {value}"
				)))
			}
		}
	}

	fn u16(&mut self, name: &str) -> ProbeResult<u16> {
		let bytes: [u8; 2] = self
			.raw(2, name)?
			.try_into()
			.map_err(|_| codec_error(format!("{name} is truncated")))?;
		Ok(u16::from_le_bytes(bytes))
	}

	fn u32(&mut self, name: &str) -> ProbeResult<u32> {
		let bytes: [u8; 4] = self
			.raw(4, name)?
			.try_into()
			.map_err(|_| codec_error(format!("{name} is truncated")))?;
		Ok(u32::from_le_bytes(bytes))
	}

	fn u64(&mut self, name: &str) -> ProbeResult<u64> {
		let bytes: [u8; 8] = self
			.raw(8, name)?
			.try_into()
			.map_err(|_| codec_error(format!("{name} is truncated")))?;
		Ok(u64::from_le_bytes(bytes))
	}

	fn u128(&mut self, name: &str) -> ProbeResult<u128> {
		let bytes: [u8; 16] = self
			.raw(16, name)?
			.try_into()
			.map_err(|_| codec_error(format!("{name} is truncated")))?;
		Ok(u128::from_le_bytes(bytes))
	}

	fn digest(&mut self, name: &str) -> ProbeResult<Digest> {
		let bytes: [u8; 32] = self
			.raw(32, name)?
			.try_into()
			.map_err(|_| codec_error(format!("{name} is truncated")))?;
		Ok(Digest::new(bytes))
	}

	fn length(&mut self, name: &str) -> ProbeResult<usize> {
		let value = usize::try_from(self.u32(name)?)
			.map_err(|_| codec_error(format!("{name} length does not fit usize")))?;
		if value > MAXIMUM_ITEMS {
			return Err(codec_error(format!(
				"{name} contains {value} items; limit is {MAXIMUM_ITEMS}"
			)));
		}
		Ok(value)
	}

	fn label(&mut self, name: &str) -> ProbeResult<Label> {
		let length = usize::try_from(self.u32(&format!("{name}.length"))?)
			.map_err(|_| codec_error(format!("{name} length does not fit usize")))?;
		if length > MAXIMUM_STRING_BYTES {
			return Err(codec_error(format!(
				"{name} is {length} bytes; limit is {MAXIMUM_STRING_BYTES}"
			)));
		}
		let bytes = self.raw(length, name)?;
		let value =
			std::str::from_utf8(bytes).map_err(|error| codec_error(format!("{name} is not UTF-8: {error}")))?;
		Label::new(value).map_err(|error| codec_error(format!("{name}: {error}")))
	}

	fn provenance(&mut self, name: &str) -> ProbeResult<PropertyProvenance> {
		match self.u8(name)? {
			0 => Ok(PropertyProvenance::Estimated),
			1 => Ok(PropertyProvenance::Measured),
			2 => Ok(PropertyProvenance::Override),
			tag => {
				Err(codec_error(format!(
					"{name} has invalid provenance tag {tag}"
				)))
			}
		}
	}

	fn property_bytes(&mut self, name: &str) -> ProbeResult<Property<ByteCount>> {
		Ok(Property::new(
			ByteCount::new(self.u64(&format!("{name}.value"))?),
			self.provenance(&format!("{name}.provenance"))?,
		))
	}

	fn property_rate(&mut self, name: &str) -> ProbeResult<Property<BytesPerSecond>> {
		let raw = self.u64(&format!("{name}.value"))?;
		Ok(Property::new(
			BytesPerSecond::new(raw).map_err(|error| codec_error(format!("{name}: {error}")))?,
			self.provenance(&format!("{name}.provenance"))?,
		))
	}

	fn property_lanes(&mut self, name: &str) -> ProbeResult<Property<TransferLaneCount>> {
		let raw = self.u32(&format!("{name}.value"))?;
		Ok(Property::new(
			TransferLaneCount::new(raw).map_err(|error| codec_error(format!("{name}: {error}")))?,
			self.provenance(&format!("{name}.provenance"))?,
		))
	}

	fn property_flops(&mut self, name: &str) -> ProbeResult<Property<FlopsPerSecond>> {
		let raw = self.u64(&format!("{name}.value"))?;
		Ok(Property::new(
			FlopsPerSecond::new(raw).map_err(|error| codec_error(format!("{name}: {error}")))?,
			self.provenance(&format!("{name}.provenance"))?,
		))
	}
}

fn codec_error(message: impl Into<String>) -> ProbeError { ProbeError::Cache(format!("codec: {}", message.into())) }
