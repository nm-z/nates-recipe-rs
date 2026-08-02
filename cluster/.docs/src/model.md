# `cluster/src/model.rs`

## Intent and ownership

`model.rs` is the typed boundary for assembling several independently probed
machines into one measured cluster profile. It owns the user-supplied
membership declarations, the expected identity of each member profile, the
unordered identity of a configured network pair, submitted per-machine
profiles, and authenticated peer-probe evidence. It validates those values at
the point where they cross into cluster assembly. It does not discover
hardware, perform a probe, choose a route, lower a calculation, or schedule a
task.

The module deliberately keeps measured hardware facts out of
`ClusterConfiguration`. A configuration names members and pair endpoints only;
capacities, rates, lane counts, duplex behavior, and transport kinds come from
`MeasuredProfile` values or `MeasuredNetworkPair` probe results. The public
crate facade re-exports the model types and `assemble_cluster` from
`recipe::engine::cluster` (`src/facade.rs`), while the cluster crate itself
depends only on `recipe-core`, `recipe-probe`, `recipe-transport`, and `sha2`.
There are currently no workspace call sites that invoke `assemble_cluster`.
The caller is expected to assemble a profile and then pass that profile to the
normal preparation boundary.

The ownership split is:

| Concern | Owner | Model connection |
| --- | --- | --- |
| Member and pair declarations | `model.rs` | `ClusterConfiguration`, `MemberSpec`, `NetworkPairSpec` |
| Per-member identity | `model.rs` | `MemberProfileIdentity::derive` |
| Probe evidence admission | `model.rs` | `MeasuredNetworkPair::from_probe` |
| Canonical cluster digest | `hash.rs` | Consumes model values plus assembly intermediates |
| Global ID allocation and profile construction | `assemble.rs` | Consumes every model value and returns `MeasuredProfile` |
| Binary profile validation and encoding | `recipe-probe` codec, with a cluster shape facade in `assemble.rs` | `ClusterProfileCodec` |
| Planning and scheduling | `recipe-planner`, `recipe-scheduler` | Consume the returned profile's topology and discovery snapshots |

Public construction is intentionally narrow: `MemberProfileIdentity` has only
`derive`, `MemberSpec` and `NetworkPairSpec` have only `new`,
`ClusterConfiguration` has only `new`, `SubmittedProfile` has only `new`, and
`MeasuredNetworkPair` admits evidence only through `from_probe`. `PairKey`,
`index_submissions`,
`validate_peer_evidence`, `sha256`, and `ensure` remain crate-private. Private
fields prevent a caller from bypassing the checks that make these values safe
for assembly.

## External contracts used by the model

The following types are not defined here, but their contracts are part of the
model boundary:

* `recipe_core::Label` is nonempty and ordered lexicographically. All member,
  address, and pair ordering in this module is label ordering.
* `recipe_probe::MeasuredProfileCodec::encode` first validates a profile and
  then emits canonical bytes. The codec checks profile schema, nonzero cache,
  topology and discovery validity, origin references, canonical vector order,
  measured-only scheduling properties, topology/discovery equality, and peer
  topology correspondence. `MemberProfileIdentity::derive` uses this codec
  before hashing a member profile.
* `recipe_transport::EndpointIdentity::new` rejects zero machine or profile
  digests. `SessionIdentity::new` rejects two endpoints with the same machine
  digest. `MeasuredLocalMemory` and the unit types reject invalid zero values
  at their own constructors.
* `BoundedBenchmarkPlan::is_bounded` means nonzero buffer, iteration count, and
  duration. The peer transport adds its own byte and iteration limits. The
  cluster model requires the bounded predicate and checks the complete
  evidence against the exact plan.
* `PropertyProvenance::Measured` is required for peer memory and directional
  throughput evidence. The assembled topology and discovery snapshots carry
  measured properties, so they can pass the production scheduling validators.

## Type inventory

### `CLUSTER_SCHEMA`

`CLUSTER_SCHEMA` is the cluster digest schema integer, currently `4`. It is
written by `hash::CanonicalDigest::new` after the length-prefixed digest domain
and therefore participates in all three assembly digests. It is distinct from
`recipe_probe::PROFILE_SCHEMA` (currently `7`) and from the `v3` suffixes in
the cluster hash domains. Changing any of these version markers changes the
identity or makes an old profile unacceptable; none is a runtime tuning value.

### `MemberProfileIdentity`

```text
MemberProfileIdentity {
    endpoint: EndpointIdentity { machine: Digest, profile: Digest },
    cache: CacheIdentity,
    topology: TopologyIdentity,
    discovery: DiscoveryIdentity,
}
```

This is the expected content-bound identity of one submitted member. It is
`Copy`, but it does not implement a separate hash algorithm. Equality compares
all four fields; `hash.rs` serializes the fields explicitly.

`MemberProfileIdentity::derive(profile)` performs these operations in order:

1. It runs `MeasuredProfileCodec::encode(profile)`. Any canonical profile
   failure is returned as `InvalidClusterProfile("member profile failed
   canonical validation: ...")`.
2. It requires exactly one `profile.topology.machines` entry and exactly one
   `profile.origins.machines` entry. It also requires that the origin exists,
   although the preceding codec validation normally guarantees that fact.
3. It hashes the complete canonical profile bytes with SHA-256 to form the
   endpoint profile digest.
4. It hashes the domain string `recipe-cluster-machine-identity-v3` followed by
   the origin fingerprint's stable machine ID to form the endpoint machine
   digest. Hostname, runtime ABI, and firmware do not enter this machine digest;
   they remain in the canonical profile and in the cluster digest inputs.
5. It constructs `EndpointIdentity` from the two digests and copies the
   profile's `cache_identity`, `topology.identity`, and `discovery.identity`.

The four `const` accessors return copies. The endpoint is used to authenticate
peer sessions and to prevent two configured members from naming one machine or
one profile. The cache, topology, and discovery identities are retained for
configuration hashing and for downstream profile matching. A stale submitted
profile is rejected by comparing this entire value with `MemberSpec::expected`.
Because the endpoint profile digest covers the complete codec byte stream, the
source profile's canonical IDs and vector contents are identity inputs too. The
later cluster remap deliberately does not reuse those local IDs as global IDs.
The one-machine shape is intentional: a member submission cannot hide peer
machines inside its topology. Cross-machine links enter this boundary only as
`MeasuredNetworkPair` evidence and are added during cluster remapping.

The identity fields have this complete dataflow:

| Identity field | Source | Direct consumers |
| --- | --- | --- |
| `endpoint.machine` | SHA-256 of stable machine ID with the cluster machine domain | Configuration uniqueness, authenticated session endpoints, peer evidence binding, cluster hash |
| `endpoint.profile` | SHA-256 of canonical `MeasuredProfileCodec` bytes | Configuration uniqueness, stale-member check, session endpoints, cluster hash, and the assembled profile digest input |
| `cache` | Submitted profile's `CacheIdentity` | Cluster hash and final profile cache identity input |
| `topology` | Submitted profile's `TopologyIdentity` | Cluster hash and expected topology provenance carried into preparation |
| `discovery` | Submitted profile's `DiscoveryIdentity` | Cluster hash and expected discovery provenance carried into preparation |

### `MemberSpec`

`MemberSpec` is one configuration membership record:

| Field | Meaning | Use |
| --- | --- | --- |
| `key: Label` | Stable configuration name for the member | Indexes `SubmittedProfile`, pair endpoints, and canonical hashes |
| `address: Label` | User-provided endpoint address label | Uniqueness check and canonical cluster hash; no address parsing or network probing |
| `expected: MemberProfileIdentity` | Identity captured when membership was configured | Rejects stale or unexpected submitted profiles |

`new` and the three accessors are the complete public API. The type has no
hardware or capacity fields. `ClusterConfiguration::new` sorts records by key,
rejects duplicate keys and addresses, and rejects duplicate expected endpoint
machine or profile digests before storing the records.

### `PairKey`

`PairKey` is crate-private and is the canonical unordered pair used by assembly
maps and sets. `PairKey::new(first, second)` compares the labels and stores the
lesser label in `first`; a reversed input is normalized rather than rejected.
Equal labels return `InvalidConfiguration` with a same-member message. The
type derives `Ord` so `BTreeMap` and `BTreeSet` operations do not depend on
caller order. `display` renders the canonical pair as `first<->second` for
errors. It is used by `NetworkPairSpec`, by `MeasuredNetworkPair::pair_key`,
and by `assemble::resolve_network`.

### `NetworkPairSpec`

This public wrapper is the configuration declaration for one measured
inter-machine pair. `new` delegates to `PairKey::new`, so self-pairs fail at
construction and reversed endpoints become one canonical pair. `first` and
`second` expose the normalized labels. It contains no rates, transport kind,
duplex mode, or memory measurements.

### `ClusterConfiguration`

`ClusterConfiguration` contains only:

```text
master: Label
members: Vec<MemberSpec>
pairs: Vec<NetworkPairSpec>
```

`new(master, members, pairs)` consumes and canonicalizes both vectors. Its
invariants are:

1. At least two members are present.
2. Member keys are unique.
3. Endpoint address labels are unique.
4. Expected endpoint machine digests are unique.
5. Expected endpoint profile digests are unique.
6. The master key belongs to membership.
7. Every pair endpoint belongs to membership.
8. No canonical pair is repeated.
9. The undirected graph of configured pairs reaches every member.

The uniqueness checks use `BTreeSet`s. Pairs and members are sorted before
validation is complete, which makes all later assembly order independent. The
connectivity check starts at the first sorted member and repeatedly extends the
reached set across either endpoint of every pair. A disconnected graph returns
`InvalidConfiguration` listing unreachable members. The helper's empty-set
error is retained even though the public constructor already requires two
members. The configuration requires a connected spanning pair graph, not a
complete graph of every possible member-to-member edge.

The accessors expose the canonical master, member slice, and pair slice. They
are borrowed, so callers cannot mutate the validated configuration. The
configuration itself is consumed by `hash::cluster_digest` and by all three
assembly stages: member preparation, network resolution, and global remapping.

### `SubmittedProfile`

`SubmittedProfile` is the caller-owned association of a member key and a
`recipe_probe::MeasuredProfile`. `new` stores both values without doing work;
`into_parts` is crate-private and is used only by `index_submissions`.

`index_submissions` moves every submission into a `BTreeMap<Label,
MeasuredProfile>`. Inserting an already present key returns
`DuplicateMember`; a missing configured key becomes `MissingMember`, and a
leftover unconfigured key becomes `InvalidMember` during `prepare_members`.
The profile is not trusted merely because it is wrapped. Assembly derives and
checks its `MemberProfileIdentity`, then requires the per-machine shape.

### `MeasuredNetworkPair`

This type is the model's only public admission point for an established peer
measurement. Its fields are crate-visible rather than publicly mutable:

| Field | Source and purpose |
| --- | --- |
| `local_member`, `remote_member` | Configuration labels identifying the probe direction |
| `session` | Authenticated `SessionIdentity` whose endpoint digests bind the evidence |
| `descriptor` | `PeerDescriptor` containing session ID, remote fingerprint, memory/interface keys, transport, duplex, lane counts, and asynchronous capability |
| `local_memory` | Measured local RAM capacity and transfer rate from the transport handshake |
| `measurement` | Remote RAM properties, both directional rates, and structured benchmark evidence |
| `benchmark` | Exact bounded plan used for this peer attempt |

The `PartialEq` value includes all evidence, not just the directional rates.
That matters because `assemble::resolve_network` and `hash::cluster_digest`
must not silently discard the session, descriptor, or timing provenance.

#### `MeasuredNetworkPair::from_probe`

Construction performs the following checks and returns
`InvalidNetworkMeasurement` for every rejection:

* local and remote member labels differ;
* the benchmark plan is bounded and nonzero;
* the descriptor advertises asynchronous submission;
* the descriptor transport is `Ethernet` or `Wlan`, the only inter-machine
  transport kinds accepted here;
* the descriptor's `LinkDuplex` agrees with
  `TransportKind::required_duplex` (`Ethernet` is full duplex and `Wlan` is
  half duplex in the core contract);
* remote memory capacity and remote memory transfer rate have
  `PropertyProvenance::Measured`;
* both optional directional rates are present and both have measured
  provenance; and
* `validate_peer_evidence` accepts the authenticated endpoint and timing
  record.

`pair_key` is crate-private and canonicalizes the two member labels through
`PairKey::new`. It is the key used to match this evidence to one
`NetworkPairSpec`, regardless of which peer initiated the probe.
The constructor does not yet know whether those labels are configured members
or whether descriptor RAM keys and fingerprints belong to their submitted
profiles. `assemble::resolve_pair` performs those cross-object checks after all
member identities have been prepared.

#### Peer evidence consistency

`validate_peer_evidence` binds the record to the supplied session by requiring:

* `evidence.protocol_schema == PEER_BENCHMARK_PROTOCOL_SCHEMA`;
* local endpoint machine and profile digests equal `session.local()`;
* remote endpoint machine and profile digests equal `session.remote()`; and
* full duplex uses `PeerDuplexExecution::Simultaneous`, while half duplex uses
  `PeerDuplexExecution::Serialized`.

It then derives `total_bytes` with checked multiplication of
`benchmark.buffer_bytes` and `benchmark.iterations`, and converts the maximum
duration to canonical `u64` nanoseconds. For each outbound and inbound sample,
the accepted rate is exactly:

```text
clamp(total_bytes * 1_000_000_000 / max(elapsed_nanoseconds, 1), 1, u64::MAX)
```

The sample must report that byte count, a nonzero elapsed duration no greater
than the plan maximum, exactly the plan iteration count, nonzero minimum sample
duration, ordered minimum <= mean <= maximum, maximum <= total elapsed, and the
derived rate. Overflow or any mismatch is an
`InvalidNetworkMeasurement` error. Variance is retained and later hashed, but
the model does not impose an additional statistical relation on it.

After admission, the evidence dataflow is fixed:

```text
MeasuredNetworkPair
  ├─ PairKey(local_member, remote_member) -> resolve_network indexing
  ├─ SessionIdentity + PeerBenchmarkEvidence -> authenticated endpoint checks
  ├─ PeerDescriptor RAM keys + memory values -> resolve_gateway device selection
  ├─ directional rates/lanes/duplex -> ResolvedNetworkPair -> DirectedLink values
  ├─ descriptor.async -> DiscoveredLink.asynchronous_submission
  └─ descriptor/session/evidence strings and timing -> cluster identity hash and peer_benchmarks
```

## Assembly lifecycle and model ownership

`assemble::assemble_cluster(configuration, submissions, network_measurements)`
is the only operation that turns these model values into one global
`MeasuredProfile`. The exact flow is:

### 1. Prepare members

`index_submissions` rejects duplicate keys. For every canonical
`MemberSpec`, `prepare_members` removes the submitted profile, derives its
`MemberProfileIdentity`, and compares it with `spec.expected()`.

It then rejects a duplicate submitted profile digest, rejects a duplicated
machine name, and stores a private `PreparedMember { key, identity, profile }`.
After all configured members are consumed, any remaining submission is an
unconfigured-member `InvalidMember`. The profile identity derivation wraps
identity and codec failures as `InvalidMember { member, message }`, while an
identity mismatch is `StaleMember`.

### 2. Require common benchmark metadata

`common_benchmarks` copies the first member's complete `BenchmarkMetadata` and
requires exact equality for every other member. This keeps one authoritative
benchmark plan in the assembled profile. An empty prepared set is an
`InvalidConfiguration` error.

### 3. Resolve network evidence

`resolve_network` indexes all `MeasuredNetworkPair` values by canonical
`PairKey`, rejecting duplicates. It then walks the canonical configured pair
list:

* no evidence for a configured pair is `MissingNetworkMeasurement`;
* evidence left over after all configured pairs is an unconfigured-pair
  `InvalidNetworkMeasurement`; and
* `resolve_pair` checks both member labels, requires the `SessionIdentity`
  local and remote endpoints to equal the corresponding prepared member
  endpoint identities, and requires the evidence benchmark plan to equal the
  common network plan.

`resolve_pair` also requires the descriptor's remote machine fingerprint to
equal the remote member's retained machine origin. It resolves the descriptor
local and remote RAM keys through `MeasuredRamOrigin`, verifies the referenced
devices exist, and verifies that each capacity and transfer rate exactly equals
the measured origin device. Missing keys, missing devices, and value mismatch
are all `InvalidNetworkMeasurement` errors.

The evidence can arrive in either direction. If the local member is the
canonical first label, outbound and inbound rates, lane counts, and memory
values are retained as first-to-second and second-to-first. If the local member
is the second label, those values are swapped. The result is a private
`ResolvedNetworkPair` with canonical endpoint order plus all descriptor strings,
the exact plan, directional properties, and complete benchmark evidence.

### 4. Derive identities and hash all semantic inputs

Assembly calls `hash::cluster_digest` three times with the domain strings:

```text
recipe-cluster-topology-v3
recipe-cluster-discovery-v3
recipe-cluster-profile-v3
```

Each digest begins with the length-prefixed domain and `CLUSTER_SCHEMA`, then
hashes the master label, all four benchmark plans and seed schema, every sorted
member's key/address/expected identity fields plus the actual submitted profile
endpoint profile digest, and every resolved network pair. Pair hashing includes
canonical labels, transport and duplex tags, both directional rates and
provenance tags, lane counts, both gateway capacities and rates, session and
descriptor identity strings, the benchmark plan, both measured rates and their
provenance, and the full authenticated timing evidence. Strings and duration
byte arrays are length-prefixed; numeric values use little-endian encoding.

`hash_peer_evidence` additionally includes protocol schema, both endpoint
machine/profile digests, the execution mode, and every directional timing field
including variance. Thus a digest changes when a measured property, endpoint,
plan, descriptor, or evidence record changes. Because configuration vectors are
sorted and network values are indexed by canonical pair, submission order does
not change any digest.
Per-member numeric IDs and the resolved `first_device`/`second_device` values
are intentionally not serialized. The canonical member labels, retained RAM
origin keys, exact gateway capacities/rates, and peer evidence bind those
selections; global IDs are allocated only after hashing.
Only the pair's numeric direction, lane, and gateway fields are normalized to
canonical first/second order. Session IDs, descriptor strings, and the
local/remote endpoint order inside `PeerBenchmarkEvidence` remain exactly as
authenticated and are hashed in that orientation.

The topology and discovery identities are wrappers around their respective
domain digests. The profile digest becomes `CacheIdentity { schema:
PROFILE_SCHEMA, digest: profile_identity }`.
`MeasuredProfile::is_cache_valid_for` and the explicit profile cache compare
that schema and digest exactly, so the assembled profile digest is a cache key,
not a diagnostic label. A changed member, address, benchmark, gateway, or peer
evidence record cannot reuse the prior assembled cache entry.

### 5. Remap local IDs into one global profile

`remap_profile` starts `IdAllocator` counters at one for machines, nodes,
devices, links, transports, and duplex resources. It allocates a
`MemberRemap` for each sorted member by walking the member's canonical profile
arrays. Every old machine, node, device, link, transport, and capacity resource
therefore gets a deterministic new ID. Missing remap entries return
`InvalidClusterProfile`; counter overflow returns `IdentityExhausted` with the
identity kind.

`remap_member` copies machines and machine origins, translates every node and
device reference, preserves measured device properties, translates RAM/storage/
GPU origin keys, and translates topology and discovery links. Node roles are
normalized here: a `NodeRole::Master` survives only for a node belonging to the
configured master member, and every other node is a worker. This is where local
profiles become one cluster-wide master/worker topology.

For each resolved network pair, `push_network_pair` maps the canonical first
and second gateway devices and appends two directed links. The links share one
transport identity. Full duplex allocates distinct forward and reverse
capacity resources; half duplex intentionally reuses one resource. Directional
bandwidth and lane counts are marked measured. Matching `DiscoveredLink`
records are appended with `available: true` and the descriptor's asynchronous
submission flag.

The final `MeasuredProfile` contains:

```text
schema: PROFILE_SCHEMA
cache_identity: profile digest
origins: remapped machine, RAM, storage, and GPU origin records
benchmarks: common benchmark metadata
peer_benchmarks: one record per resolved pair, sorted by session ID
topology: global topology with topology identity
discovery: global capabilities with discovery identity and the same topology identity
```

Peer benchmark records are explicitly sorted by `session_id` because the probe
codec requires strict canonical ordering and rejects duplicate session IDs.
`assemble_cluster` then runs the
canonical `MeasuredProfileCodec::encode` once more. A final codec error is
returned as `InvalidClusterProfile`.

The allocator and all collection sorting make the resulting IDs and identities
independent of the order of submitted profiles or network measurements. They
remain dependent on the canonical member keys, profile contents, pair evidence,
and configured addresses, as required by the hash contract.

## `ClusterProfileCodec` and cluster shape

`ClusterProfileCodec` is a cluster-specific facade over the canonical probe
codec. `encode` and `decode` both call `validate_cluster_shape` in addition to
the probe codec's complete profile validation. The shape validator requires:

1. at least two machines;
2. every topology link endpoint to reference a known device; and
3. the undirected graph formed by cross-machine links to connect every machine.

Same-machine links do not add inter-machine adjacency. A disconnected shape is
`InvalidClusterProfile("inter-machine transport graph is disconnected")`.
Unknown link devices, no machine, and fewer than two machines have their own
`InvalidClusterProfile` messages. Binary or canonical failures from
`MeasuredProfileCodec` are wrapped as `Codec` by this facade. `assemble_cluster`
uses the canonical codec directly because its construction path already
establishes the cluster shape; callers decoding arbitrary bytes should use
`ClusterProfileCodec` when the extra cluster graph constraint is required.

The inherited probe/core validation is part of the output contract even though
it is implemented outside this file. It requires a nonzero topology and
discovery identity, exact `DiscoveryProfile.topology == Topology.identity`,
unique and owned machines/nodes/devices/links, one master node, valid device
kind and calculation capability combinations, opposing directional links with
the required transport duplex/resource relationship, and complete discovery
records whose measured values equal the topology's directional values. Origins
must point to the right machine or device kind, use unique machine-scoped keys,
and cover every required domain. Persisted vectors and node device lists must
be strictly canonical by ID, and every scheduling property must have measured
provenance. Peer benchmark records must match one unused cross-machine link
pair, its endpoint identities, rates, and full or half-duplex resource
semantics. These checks explain why a shape-valid profile can still be rejected
as `InvalidClusterProfile` or `Codec`.

## Identity and planner/scheduler consumers

The cluster crate has no dependency on `recipe-planner` or
`recipe-scheduler`, so model types do not call either subsystem directly. The
downstream boundary is the assembled `MeasuredProfile`:

```text
assemble_cluster -> MeasuredProfile
                 -> Preparer::prepare_program
                 -> plan_program_candidates
                 -> lower_candidate -> scheduler::schedule
                 -> immutable Draft/FinalizedBundle
```

`recipe_prepare::Preparer::prepare_program` first invokes
`recipe_probe::validate_profile`. It then passes only `profile.topology` and
`profile.discovery` into reservation planning, artifact resolution, and
`recipe_planner::plan_program_candidates`. A malformed cluster profile is
reported as `PrepareErrorKind::InvalidMeasuredProfile` before planning.

The planner validates topology structure and scheduling properties, validates
that discovery belongs to the same topology identity, and uses the measured
profile as follows:

* `Topology.devices` and `DiscoveryProfile.devices` determine legal GPU
  placements and the common lowering hardware.
* `TopologyIdentity` and `DiscoveryIdentity` enter candidate identity and draft
  hashing. A candidate cannot silently move to a different assembled profile.
* Link endpoints, measured bandwidth, duplex resources, and directional lane
  counts lower internal transfer chains and arena contracts.
* Discovery calculation rates, concurrent-task limits, transfer rates, queues,
  asynchronous submission, and overlap capability constrain candidates and
  their estimated makespans.

`lower_candidate` calls `recipe_scheduler::schedule(topology, discovery,
tasks)`. The scheduler validates both snapshots again, computes calculation
duration from measured calculation rate, computes transfer duration from the
selected measured link bandwidth, allocates measured transfer lanes, and
serializes opposing links only when they share the half-duplex capacity
resource. `shortest_route` orders equal measured-duration paths by link IDs.
While lowering a cross-device copy, the planner enumerates directed topology
routes, builds one hop task per link, and uses trial scheduler calls to compare
their measured end time and makespan before committing the selected chain. The
final scheduler call then writes the executor-visible windows and lane claims.
Planner maps scheduler failures to `NoRoute`, `DependencyConflict`,
`CandidateInfeasible`, or `Schedule` as appropriate. The model's
`InvalidNetworkMeasurement` errors occur earlier, while these downstream errors
describe a valid profile that cannot lower a particular program assignment.

Preparation retains the same topology and discovery snapshots in native
realizers and rejects a candidate whose draft identities do not match them.
The raw `SubmittedProfile` wrappers and `MeasuredNetworkPair` evidence are not
planner inputs; they have already served their purpose as assembly provenance,
identity material, and canonical cache/profile data. The scheduler consumes the
global topology and discovery properties produced from that evidence. Later
executor, native-session, and training paths likewise compare the typed
topology/discovery identities captured in their bundles where both snapshots
are present; worker projections at minimum compare the topology identity. None
of these consumers recomputes the private cluster hash.

## Error inventory by stage

`ClusterError` is a `#[non_exhaustive]` `Clone + Debug + PartialEq + Eq`
error enum re-exported with `ClusterResult<T> = Result<T, ClusterError>`.
Its `Display` implementation is the only presentation layer; lower-level probe,
transport, codec, planner, and scheduler details are preserved as text only at
the explicit mapping boundaries.

All fallible model and assembly helpers return `ClusterResult<T>` and use
`ensure` for boolean invariants. No invalid event is converted into a fallback
profile.

| Error | Emitted by model or assembly stage | Meaning |
| --- | --- | --- |
| `InvalidConfiguration` | `PairKey`, `ClusterConfiguration`, connectivity, empty benchmark set | Self-pair, duplicate/unknown configuration, missing master, or disconnected pair graph |
| `MissingMember` | `prepare_members` | Configured member has no submission |
| `DuplicateMember` | `index_submissions`, member identity checks | Submission key or member profile identity is repeated |
| `StaleMember` | `prepare_members` | Submitted identity differs from the configured expected identity |
| `InvalidMember { member, message }` | `prepare_members`, extra submissions, common benchmarks | Canonical profile failure, wrong per-machine shape, duplicate machine name, benchmark mismatch, or unconfigured submitter |
| `MissingNetworkMeasurement` | `resolve_network` | Configured pair has no evidence |
| `DuplicateNetworkMeasurement` | `resolve_network` | More than one evidence record maps to one canonical pair |
| `InvalidNetworkMeasurement` | `MeasuredNetworkPair`, `resolve_pair`, gateway resolution | Evidence is unbounded, unauthenticated, stale, not measured, mismatched, missing, or physically inconsistent |
| `IdentityExhausted` | `IdAllocator` in assembly | A global machine, node, device, link, transport, or resource counter overflowed |
| `InvalidClusterProfile` | identity derivation, remapping, shape validation, final codec validation | A profile cannot be used as a member or assembled cluster, or a required ID/reference/graph invariant is absent |
| `Codec` | `ClusterProfileCodec::encode/decode` | Canonical binary codec rejected bytes or a profile after cluster-shape validation |

The `Display` implementation preserves these stages in user-facing messages,
for example `member worker-a submitted a stale or unexpected measured profile`
and `network pair worker-a<->worker-b has no measured probe evidence`. Errors
from planner and scheduler are separate crate error types after the profile has
crossed this boundary.

## Source map

| Region | Responsibility |
| --- | --- |
| `model.rs:13-77` | Cluster schema and canonical per-machine identity |
| `model.rs:79-150` | Membership records and canonical unordered pairs |
| `model.rs:152-272` | Configuration sorting, uniqueness, membership references, and connectivity |
| `model.rs:274-285` | Submitted profile wrapper and submission indexing boundary |
| `model.rs:287-394` | Peer measurement carrier and constructor checks |
| `model.rs:396-466` | Authenticated peer evidence and derived-rate validation |
| `model.rs:468-487` | SHA-256 profile helper, submission map helper, and boolean invariant helper |
| `assemble.rs:128-173` | Public assembly entry point and domain-separated identity installation |
| `assemble.rs:175-475` | Member preparation, common benchmark selection, pair resolution, and RAM gateway lookup |
| `assemble.rs:491-780` | Deterministic global ID remapping and measured network-link insertion |
| `assemble.rs:783-859` | Canonical codec facade and connected multi-machine shape check |
| `hash.rs:11-177` | Canonical cluster digest byte stream |
| `error.rs:3-66` | Public `ClusterError` categories and display propagation |

## Non-negotiable invariants for changes

* Keep `ClusterConfiguration` declarative. Do not add measured rates,
  capacities, lane counts, transport handles, or topology objects to it.
* Preserve canonical label and pair ordering. Input order independence is part
  of both ID allocation and digest identity.
* Keep machine identity derived from the retained stable machine ID and profile
  identity derived from canonical profile bytes. Do not substitute a hostname,
  capacity heuristic, or generated runtime ID.
* Admit only authenticated, bounded, measured peer evidence and preserve both
  directions plus timing evidence through the hash and profile.
* Preserve the distinction between full and half duplex capacity resources.
* Keep the final topology and discovery identities equal to the domain-separated
  assembly digests and keep discovery's topology identity equal to topology's.
* Do not bypass `MeasuredProfileCodec` or the cluster shape validator when
  exposing a new encode/decode path.
* Continue to pass the resulting measured snapshots through the real
  preparation, planner, and scheduler boundaries. Model construction alone is
  not runtime scheduling evidence.
