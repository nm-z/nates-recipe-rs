use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{Digest, DiscoveryIdentity, Label, TopologyIdentity};
use recipe_probe::{
	BoundedBenchmarkPlan, CacheIdentity, LinkDuplex, MeasuredProfile, MeasuredProfileCodec,
	PEER_BENCHMARK_PROTOCOL_SCHEMA, PeerDescriptor, PeerDuplexExecution, PeerMeasurement,
};
use recipe_transport::{EndpointIdentity, MeasuredLocalMemory, SessionIdentity};
use sha2::{Digest as _, Sha256};

use crate::error::{ClusterError, ClusterResult};

pub const CLUSTER_SCHEMA: u32 = 4;

/// Content-bound identity advertised by one per-machine probe endpoint.
///
/// The endpoint machine digest covers the retained stable machine ID while the
/// endpoint profile digest covers the complete canonical measured profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberProfileIdentity {
	endpoint: EndpointIdentity,
	cache: CacheIdentity,
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
}

impl MemberProfileIdentity {
	pub fn derive(profile: &MeasuredProfile) -> ClusterResult<Self> {
		let encoded = MeasuredProfileCodec::encode(profile).map_err(|error| {
			ClusterError::InvalidClusterProfile(format!(
				"member profile failed canonical validation: {error}"
			))
		})?;
		ensure(
			profile.topology.machines.len() == 1,
			ClusterError::InvalidClusterProfile(format!(
				"per-machine profile must contain exactly one machine, found {}",
				profile.topology.machines.len()
			)),
		)?;
		ensure(
			profile.origins.machines.len() == 1,
			ClusterError::InvalidClusterProfile(format!(
				"per-machine profile must contain exactly one machine origin, found {}",
				profile.origins.machines.len()
			)),
		)?;
		let origin = profile.origins.machines.first().ok_or_else(|| {
			ClusterError::InvalidClusterProfile("per-machine profile has no machine origin".to_owned())
		})?;
		let profile_digest = sha256(&encoded);
		let mut machine = Sha256::new();
		machine.update(b"recipe-cluster-machine-identity-v3");
		machine.update(origin.fingerprint.stable_id.as_str().as_bytes());
		let machine_digest = Digest::new(<[u8; 32]>::from(machine.finalize()));
		let endpoint = EndpointIdentity::new(machine_digest, profile_digest)
			.map_err(|error| ClusterError::InvalidClusterProfile(error.to_string()))?;
		Ok(Self {
			endpoint,
			cache: profile.cache_identity,
			topology: profile.topology.identity,
			discovery: profile.discovery.identity,
		})
	}

	#[must_use]
	pub const fn endpoint(self) -> EndpointIdentity {
		self.endpoint
	}

	#[must_use]
	pub const fn cache(self) -> CacheIdentity {
		self.cache
	}

	#[must_use]
	pub const fn topology(self) -> TopologyIdentity {
		self.topology
	}

	#[must_use]
	pub const fn discovery(self) -> DiscoveryIdentity {
		self.discovery
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberSpec {
	key: Label,
	address: Label,
	expected: MemberProfileIdentity,
}

impl MemberSpec {
	#[must_use]
	pub const fn new(key: Label, address: Label, expected: MemberProfileIdentity) -> Self {
		Self {
			key,
			address,
			expected,
		}
	}

	#[must_use]
	pub const fn key(&self) -> &Label {
		&self.key
	}

	#[must_use]
	pub const fn address(&self) -> &Label {
		&self.address
	}

	#[must_use]
	pub const fn expected(&self) -> MemberProfileIdentity {
		self.expected
	}
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PairKey {
	pub(crate) first: Label,
	pub(crate) second: Label,
}

impl PairKey {
	pub(crate) fn new(first: Label, second: Label) -> ClusterResult<Self> {
		match first.cmp(&second) {
			core::cmp::Ordering::Less => Ok(Self { first, second }),
			core::cmp::Ordering::Greater => Ok(Self {
				first: second,
				second: first,
			}),
			core::cmp::Ordering::Equal => Err(ClusterError::InvalidConfiguration(format!(
				"network pair {first} references the same member twice"
			))),
		}
	}

	pub(crate) fn display(&self) -> String {
		format!("{}<->{}", self.first, self.second)
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkPairSpec {
	pair: PairKey,
}

impl NetworkPairSpec {
	pub fn new(first: Label, second: Label) -> ClusterResult<Self> {
		Ok(Self {
			pair: PairKey::new(first, second)?,
		})
	}

	#[must_use]
	pub const fn first(&self) -> &Label {
		&self.pair.first
	}

	#[must_use]
	pub const fn second(&self) -> &Label {
		&self.pair.second
	}
}

/// User-controlled cluster membership. It deliberately has no hardware,
/// capacity, bandwidth, lane-count, transport, or topology fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterConfiguration {
	master: Label,
	members: Vec<MemberSpec>,
	pairs: Vec<NetworkPairSpec>,
}

impl ClusterConfiguration {
	pub fn new(master: Label, mut members: Vec<MemberSpec>, mut pairs: Vec<NetworkPairSpec>) -> ClusterResult<Self> {
		ensure(
			members.len() >= 2,
			ClusterError::InvalidConfiguration("a cluster requires at least two measured members".to_owned()),
		)?;
		members.sort_by(|left, right| left.key.cmp(&right.key));
		let mut keys = BTreeSet::new();
		let mut addresses = BTreeSet::new();
		let mut machine_identities = BTreeSet::new();
		let mut profile_identities = BTreeSet::new();
		for member in &members {
			ensure(
				keys.insert(member.key.clone()),
				ClusterError::DuplicateMember(member.key.to_string()),
			)?;
			ensure(
				addresses.insert(member.address.clone()),
				ClusterError::InvalidConfiguration(format!(
					"endpoint address {} appears more than once",
					member.address
				)),
			)?;
			ensure(
				machine_identities.insert(member.expected.endpoint.machine()),
				ClusterError::InvalidConfiguration(format!(
					"member {} repeats an endpoint machine identity",
					member.key
				)),
			)?;
			ensure(
				profile_identities.insert(member.expected.endpoint.profile()),
				ClusterError::InvalidConfiguration(format!(
					"member {} repeats a measured profile identity",
					member.key
				)),
			)?;
		}
		ensure(
			keys.contains(&master),
			ClusterError::InvalidConfiguration(format!("master member {master} is not in membership")),
		)?;

		pairs.sort_by(|left, right| left.pair.cmp(&right.pair));
		let mut pair_keys = BTreeSet::new();
		for pair in &pairs {
			ensure(
				keys.contains(&pair.pair.first) && keys.contains(&pair.pair.second),
				ClusterError::InvalidConfiguration(format!(
					"network pair {} references a member outside membership",
					pair.pair.display()
				)),
			)?;
			ensure(
				pair_keys.insert(pair.pair.clone()),
				ClusterError::InvalidConfiguration(format!(
					"network pair {} appears more than once",
					pair.pair.display()
				)),
			)?;
		}
		require_connected(&keys, &pairs)?;

		Ok(Self {
			master,
			members,
			pairs,
		})
	}

	#[must_use]
	pub const fn master(&self) -> &Label {
		&self.master
	}

	#[must_use]
	pub fn members(&self) -> &[MemberSpec] {
		&self.members
	}

	#[must_use]
	pub fn network_pairs(&self) -> &[NetworkPairSpec] {
		&self.pairs
	}
}

fn require_connected(keys: &BTreeSet<Label>, pairs: &[NetworkPairSpec]) -> ClusterResult<()> {
	let first = keys
		.first()
		.ok_or_else(|| ClusterError::InvalidConfiguration("membership is empty".to_owned()))?
		.clone();
	let mut reached = BTreeSet::from([first]);
	for _ in 0..keys.len() {
		for pair in pairs {
			let newly_reached = match (
				reached.contains(&pair.pair.first),
				reached.contains(&pair.pair.second),
			) {
				(true, false) => Some(pair.pair.second.clone()),
				(false, true) => Some(pair.pair.first.clone()),
				(true, true) | (false, false) => None,
			};
			reached.extend(newly_reached);
		}
	}
	let missing = keys
		.iter()
		.filter(|key| !reached.contains(*key))
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(", ");
	ensure(
		&reached == keys,
		ClusterError::InvalidConfiguration(format!(
			"network-pair graph is disconnected; unreachable members: {missing}"
		)),
	)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedProfile {
	key: Label,
	profile: MeasuredProfile,
}

impl SubmittedProfile {
	#[must_use]
	pub const fn new(key: Label, profile: MeasuredProfile) -> Self {
		Self { key, profile }
	}

	pub(crate) fn into_parts(self) -> (Label, MeasuredProfile) {
		(self.key, self.profile)
	}

	#[cfg(test)]
	pub(crate) const fn key(&self) -> &Label {
		&self.key
	}

	#[cfg(test)]
	pub(crate) fn profile_mut(&mut self) -> &mut MeasuredProfile {
		&mut self.profile
	}
}

/// Probe-produced evidence for both directions of one established peer
/// session. Numeric fields are intentionally absent from cluster
/// configuration and enter assembly only through probe/transport result types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredNetworkPair {
	pub(crate) local_member: Label,
	pub(crate) remote_member: Label,
	pub(crate) session: SessionIdentity,
	pub(crate) descriptor: PeerDescriptor,
	pub(crate) local_memory: MeasuredLocalMemory,
	pub(crate) measurement: PeerMeasurement,
	pub(crate) benchmark: BoundedBenchmarkPlan,
}

impl MeasuredNetworkPair {
	pub fn from_probe(
		local_member: Label,
		remote_member: Label,
		session: SessionIdentity,
		descriptor: PeerDescriptor,
		local_memory: MeasuredLocalMemory,
		measurement: PeerMeasurement,
		benchmark: BoundedBenchmarkPlan,
	) -> ClusterResult<Self> {
		ensure(
			local_member != remote_member,
			ClusterError::InvalidNetworkMeasurement("a measured pair must connect distinct members".to_owned()),
		)?;
		ensure(
			benchmark.is_bounded(),
			ClusterError::InvalidNetworkMeasurement("network benchmark plan is unbounded".to_owned()),
		)?;
		ensure(
			descriptor.asynchronous_submission,
			ClusterError::InvalidNetworkMeasurement(
				"peer transport does not support asynchronous submission".to_owned(),
			),
		)?;
		let expected_duplex = match descriptor.transport_kind.required_duplex() {
			recipe_core::DuplexMode::Full => LinkDuplex::Full,
			recipe_core::DuplexMode::Half => LinkDuplex::Half,
		};
		ensure(
			descriptor.duplex == expected_duplex,
			ClusterError::InvalidNetworkMeasurement(format!(
				"{:?} requires {expected_duplex:?} duplex operation",
				descriptor.transport_kind
			)),
		)?;
		ensure(
			matches!(
				descriptor.transport_kind,
				recipe_core::TransportKind::Ethernet | recipe_core::TransportKind::Wlan
			),
			ClusterError::InvalidNetworkMeasurement(format!(
				"{:?} is not an inter-machine network transport",
				descriptor.transport_kind
			)),
		)?;
		for (name, provenance) in [
			(
				"remote memory capacity",
				measurement.remote_memory_capacity.provenance,
			),
			(
				"remote memory transfer rate",
				measurement.remote_memory_rate.provenance,
			),
		] {
			ensure(
				provenance == recipe_core::PropertyProvenance::Measured,
				ClusterError::InvalidNetworkMeasurement(format!("{name} does not have measured provenance")),
			)?;
		}
		let outbound = measurement.outbound_rate.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement("outbound throughput measurement is missing".to_owned())
		})?;
		let inbound = measurement.inbound_rate.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement("inbound throughput measurement is missing".to_owned())
		})?;
		ensure(
			matches!(
				(outbound.provenance, inbound.provenance),
				(
					recipe_core::PropertyProvenance::Measured,
					recipe_core::PropertyProvenance::Measured
				)
			),
			ClusterError::InvalidNetworkMeasurement(
				"directional throughput does not have measured provenance".to_owned(),
			),
		)?;
		validate_peer_evidence(session, descriptor.duplex, measurement, benchmark)?;
		Ok(Self {
			local_member,
			remote_member,
			session,
			descriptor,
			local_memory,
			measurement,
			benchmark,
		})
	}

	pub(crate) fn pair_key(&self) -> ClusterResult<PairKey> {
		PairKey::new(self.local_member.clone(), self.remote_member.clone())
	}
}

fn validate_peer_evidence(
	session: SessionIdentity,
	duplex: LinkDuplex,
	measurement: PeerMeasurement,
	benchmark: BoundedBenchmarkPlan,
) -> ClusterResult<()> {
	let evidence = measurement.evidence;
	let local = session.local();
	let remote = session.remote();
	ensure(
		evidence.protocol_schema == PEER_BENCHMARK_PROTOCOL_SCHEMA
			&& evidence.local_endpoint.machine == local.machine()
			&& evidence.local_endpoint.profile == local.profile()
			&& evidence.remote_endpoint.machine == remote.machine()
			&& evidence.remote_endpoint.profile == remote.profile(),
		ClusterError::InvalidNetworkMeasurement(
			"peer benchmark evidence is not bound to the authenticated session endpoints".to_owned(),
		),
	)?;
	let expected_execution = match duplex {
		LinkDuplex::Full => PeerDuplexExecution::Simultaneous,
		LinkDuplex::Half => PeerDuplexExecution::Serialized,
	};
	ensure(
		evidence.execution == expected_execution,
		ClusterError::InvalidNetworkMeasurement("peer benchmark execution mode contradicts link duplex".to_owned()),
	)?;
	let total_bytes = benchmark
		.buffer_bytes
		.get()
		.checked_mul(u64::from(benchmark.iterations))
		.ok_or_else(|| {
			ClusterError::InvalidNetworkMeasurement("peer benchmark byte count overflowed".to_owned())
		})?;
	let maximum_duration = u64::try_from(benchmark.maximum_duration.as_nanos()).map_err(|_| {
		ClusterError::InvalidNetworkMeasurement("peer benchmark duration exceeds canonical nanoseconds".to_owned())
	})?;
	let outbound = measurement.outbound_rate.ok_or_else(|| {
		ClusterError::InvalidNetworkMeasurement("outbound throughput measurement is missing".to_owned())
	})?;
	let inbound = measurement.inbound_rate.ok_or_else(|| {
		ClusterError::InvalidNetworkMeasurement("inbound throughput measurement is missing".to_owned())
	})?;
	for (direction, sample, rate) in [
		("outbound", evidence.outbound, outbound.value),
		("inbound", evidence.inbound, inbound.value),
	] {
		let derived_rate = u128::from(total_bytes)
			.checked_mul(1_000_000_000)
			.and_then(|bytes| bytes.checked_div(u128::from(sample.elapsed_nanoseconds.max(1))))
			.ok_or_else(|| {
				ClusterError::InvalidNetworkMeasurement(format!(
					"{direction} throughput calculation overflowed"
				))
			})?
			.clamp(1, u128::from(u64::MAX));
		ensure(
			sample.total_bytes.get() == total_bytes
				&& sample.elapsed_nanoseconds != 0
				&& sample.elapsed_nanoseconds <= maximum_duration
				&& sample.sample_count == benchmark.iterations
				&& sample.minimum_sample_nanoseconds != 0
				&& sample.minimum_sample_nanoseconds <= sample.mean_sample_nanoseconds
				&& sample.mean_sample_nanoseconds <= sample.maximum_sample_nanoseconds
				&& sample.maximum_sample_nanoseconds <= sample.elapsed_nanoseconds
				&& u128::from(rate.get()) == derived_rate,
			ClusterError::InvalidNetworkMeasurement(format!("{direction} benchmark evidence is inconsistent")),
		)?;
	}
	Ok(())
}

fn sha256(bytes: &[u8]) -> Digest {
	Digest::new(<[u8; 32]>::from(Sha256::digest(bytes)))
}

pub(crate) fn index_submissions(submissions: Vec<SubmittedProfile>) -> ClusterResult<BTreeMap<Label, MeasuredProfile>> {
	let mut indexed = BTreeMap::new();
	for submission in submissions {
		let (key, profile) = submission.into_parts();
		match indexed.insert(key.clone(), profile) {
			Some(_) => Err(ClusterError::DuplicateMember(key.to_string())),
			None => Ok(()),
		}?;
	}
	Ok(indexed)
}

fn ensure(condition: bool, error: ClusterError) -> ClusterResult<()> {
	match condition {
		true => Ok(()),
		false => Err(error),
	}
}
