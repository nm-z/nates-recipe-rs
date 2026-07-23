use recipe_core::{Digest, DuplexMode, PropertyProvenance, TransportKind};
use recipe_probe::{BenchmarkMetadata, BoundedBenchmarkPlan};
use sha2::{Digest as _, Sha256};

use crate::assemble::{PreparedMember, ResolvedNetworkPair};
use crate::error::{ClusterError, ClusterResult};
use crate::model::{CLUSTER_SCHEMA, ClusterConfiguration};

#[derive(Clone, Debug)]
struct CanonicalDigest {
	hasher: Sha256,
}

impl CanonicalDigest {
	fn new(domain: &str) -> ClusterResult<Self> {
		let mut result = Self {
			hasher: Sha256::new(),
		};
		result.string(domain)?;
		result.u32(CLUSTER_SCHEMA);
		Ok(result)
	}

	fn bytes(&mut self, value: &[u8]) -> ClusterResult<()> {
		let length = u64::try_from(value.len()).map_err(|error| {
			ClusterError::InvalidClusterProfile(format!("identity input is too large: {error}"))
		})?;
		self.u64(length);
		self.hasher.update(value);
		Ok(())
	}

	fn string(&mut self, value: &str) -> ClusterResult<()> {
		self.bytes(value.as_bytes())
	}

	fn digest(&mut self, value: Digest) {
		self.hasher.update(value.bytes());
	}

	fn u8(&mut self, value: u8) {
		self.hasher.update([value]);
	}

	fn u32(&mut self, value: u32) {
		self.hasher.update(value.to_le_bytes());
	}

	fn u64(&mut self, value: u64) {
		self.hasher.update(value.to_le_bytes());
	}

	fn bool(&mut self, value: bool) {
		self.u8(u8::from(value));
	}

	fn finish(self) -> Digest {
		Digest::new(<[u8; 32]>::from(self.hasher.finalize()))
	}
}

pub(crate) fn cluster_digest(
	domain: &str,
	configuration: &ClusterConfiguration,
	members: &[PreparedMember],
	network: &[ResolvedNetworkPair],
	benchmarks: BenchmarkMetadata,
) -> ClusterResult<Digest> {
	let mut digest = CanonicalDigest::new(domain)?;
	digest.string(configuration.master().as_str())?;
	hash_benchmarks(&mut digest, benchmarks)?;
	for (spec, member) in configuration.members().iter().zip(members) {
		digest.string("member")?;
		digest.string(spec.key().as_str())?;
		digest.string(spec.address().as_str())?;
		let expected = spec.expected();
		digest.digest(expected.endpoint().machine());
		digest.digest(expected.endpoint().profile());
		digest.u32(expected.cache().schema);
		digest.digest(expected.cache().digest);
		digest.digest(expected.topology().digest());
		digest.digest(expected.discovery().digest());
		digest.digest(member.identity.endpoint().profile());
	}
	for pair in network {
		digest.string("network-pair")?;
		digest.string(pair.key.first.as_str())?;
		digest.string(pair.key.second.as_str())?;
		digest.u8(transport_kind_tag(pair.kind));
		digest.u8(duplex_tag(pair.duplex));
		digest.u64(pair.first_to_second.value.get());
		digest.u8(provenance_tag(pair.first_to_second.provenance));
		digest.u64(pair.second_to_first.value.get());
		digest.u8(provenance_tag(pair.second_to_first.provenance));
		digest.u32(pair.first_to_second_lanes.get());
		digest.u32(pair.second_to_first_lanes.get());
		digest.bool(pair.asynchronous_submission);
		digest.u64(pair.first_memory_capacity);
		digest.u64(pair.first_memory_rate);
		digest.u64(pair.second_memory_capacity);
		digest.u64(pair.second_memory_rate);
		digest.string(pair.evidence_session.as_str())?;
		digest.string(pair.evidence_link.as_str())?;
		digest.string(pair.evidence_remote_driver.as_str())?;
		digest.string(pair.evidence_remote_firmware.as_str())?;
		digest.string(pair.evidence_remote_stable_id.as_str())?;
		digest.string(pair.evidence_remote_runtime_abi.as_str())?;
		digest.string(pair.evidence_remote_machine_firmware.as_str())?;
		digest.string(pair.evidence_remote_memory.as_str())?;
		digest.string(pair.evidence_local_memory.as_str())?;
		digest.string(pair.evidence_local_interface.as_str())?;
		digest.string(pair.evidence_remote_interface.as_str())?;
		hash_benchmark(&mut digest, pair.benchmark)?;
		digest.u64(pair.evidence_outbound_rate.value.get());
		digest.u8(provenance_tag(pair.evidence_outbound_rate.provenance));
		digest.u64(pair.evidence_inbound_rate.value.get());
		digest.u8(provenance_tag(pair.evidence_inbound_rate.provenance));
		hash_peer_evidence(&mut digest, pair.benchmark_evidence);
	}
	Ok(digest.finish())
}

fn hash_peer_evidence(digest: &mut CanonicalDigest, evidence: recipe_probe::PeerBenchmarkEvidence) {
	digest.u32(u32::from(evidence.protocol_schema));
	for endpoint in [evidence.local_endpoint, evidence.remote_endpoint] {
		digest.digest(endpoint.machine);
		digest.digest(endpoint.profile);
	}
	digest.u8(match evidence.execution {
		recipe_probe::PeerDuplexExecution::Simultaneous => 0,
		recipe_probe::PeerDuplexExecution::Serialized => 1,
	});
	for direction in [evidence.outbound, evidence.inbound] {
		digest.u64(direction.total_bytes.get());
		digest.u64(direction.elapsed_nanoseconds);
		digest.u32(direction.sample_count);
		digest.u64(direction.minimum_sample_nanoseconds);
		digest.u64(direction.maximum_sample_nanoseconds);
		digest.u64(direction.mean_sample_nanoseconds);
		digest.hasher
			.update(direction.variance_nanoseconds_squared.to_le_bytes());
	}
}

fn hash_benchmarks(digest: &mut CanonicalDigest, metadata: BenchmarkMetadata) -> ClusterResult<()> {
	digest.u32(metadata.seed_schema);
	for plan in [
		metadata.ram,
		metadata.storage,
		metadata.gpu,
		metadata.network,
	] {
		hash_benchmark(digest, plan)?;
	}
	Ok(())
}

fn hash_benchmark(digest: &mut CanonicalDigest, plan: BoundedBenchmarkPlan) -> ClusterResult<()> {
	digest.u64(plan.buffer_bytes.get());
	digest.u32(plan.iterations);
	digest.bytes(&plan.maximum_duration.as_nanos().to_le_bytes())
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

const fn duplex_tag(duplex: DuplexMode) -> u8 {
	match duplex {
		DuplexMode::Half => 0,
		DuplexMode::Full => 1,
	}
}

const fn provenance_tag(provenance: PropertyProvenance) -> u8 {
	match provenance {
		PropertyProvenance::Estimated => 0,
		PropertyProvenance::Measured => 1,
		PropertyProvenance::Override => 2,
	}
}
