# `recipe-cluster`

`recipe-cluster` turns independently measured, identity-bound machine profiles
and explicitly measured peer links into one canonical `recipe_probe::MeasuredProfile`.
It is the assembly boundary for a distributed measured topology. It does not
discover hardware, perform a peer benchmark, choose a route, schedule a task,
compile an artifact, or realize a native backend.

The crate's source-level contract is visible in
[`src/lib.rs`](../src/lib.rs), [`src/model.rs`](../src/model.rs),
[`src/assemble.rs`](../src/assemble.rs), [`src/hash.rs`](../src/hash.rs), and
[`src/error.rs`](../src/error.rs). The five modules are private. The facade
re-exports only the deliberately public configuration, identity, evidence,
assembly, codec, error, and result types.

## Position in the workspace

```text
recipe-probe  ── measured per-machine profile ──┐
recipe-transport ─ authenticated peer evidence ──┤
recipe-core    ─ topology/identity value types ──┤
sha2          ─ canonical SHA-256 identities ────┘
                         │
                         ▼
                 recipe-cluster
                         │
                         ▼
              one measured cluster profile
                         │
                         ▼
  recipe-prepare → recipe-planner → recipe-scheduler
```

The Cargo manifest has exactly four direct dependencies: `recipe-core`,
`recipe-probe`, `recipe-transport`, and `sha2`. The root package depends on
`recipe-cluster` and re-exports it as `recipe::engine::cluster` in
[`src/facade.rs`](../../src/facade.rs#L17-L41). There is no in-tree call to
`assemble_cluster` or to any other cluster API. The reverse dependency tree is
therefore only `recipe-cluster → recipe`; the public engine re-export is the
current caller boundary. `recipe-cluster` has no dependency on planner or
scheduler, so the downstream relationship described below is a data handoff,
not a direct module call.

The crate forbids unsafe code and denies missing `Debug` implementations. It
contains no examples, tests, build script, runtime thread, socket listener, or
filesystem/cache implementation.

## What the crate owns

The crate owns five related responsibilities:

1. Validate membership declarations without allowing a user to enter hardware
   rates, capacities, lane counts, transport kinds, or topology objects.
2. Bind each submitted profile to the configured member through a canonical
   profile digest and a stable-machine digest.
3. Admit only authenticated, bounded, measured peer evidence and resolve it to
   exact RAM gateway devices in the two member profiles.
4. Rebase every member's local IDs into one deterministic global ID space,
   rewrite node roles to the configured master, and add measured cross-machine
   links.
5. Derive independent topology, discovery, and cache/profile identities, then
   pass the resulting profile through the canonical measured-profile codec.

The result is still a `MeasuredProfile`, not a cluster-specific execution plan.
The cluster-specific extra check is only that the profile has at least two
machines and a connected inter-machine graph.

## Module graph

```text
lib.rs
├── model.rs
│   ├── MemberProfileIdentity
│   ├── MemberSpec, NetworkPairSpec, ClusterConfiguration
│   ├── SubmittedProfile
│   └── MeasuredNetworkPair and peer-evidence validation
├── assemble.rs
│   ├── input indexing and member preparation
│   ├── network-pair resolution
│   ├── deterministic ID allocation/remapping
│   ├── profile construction
│   └── ClusterProfileCodec and cluster-shape validation
├── hash.rs
│   └── domain-separated canonical cluster digests
└── error.rs
    └── ClusterError and ClusterResult
```

`assemble.rs` is the only module that constructs a `MeasuredProfile`. It uses
private `PreparedMember` and `ResolvedNetworkPair` records to keep validated
intermediate facts separate from public input declarations. `hash.rs` imports
those records from `assemble.rs`, which keeps hash input construction tied to
the exact resolved assembly model instead of hashing an independently rebuilt
representation.

## Public data model

### `CLUSTER_SCHEMA`

[`CLUSTER_SCHEMA`](../src/model.rs#L13) is `4`. It is written into every
cluster digest by `CanonicalDigest::new`; it is not the binary profile codec
schema. The profile returned by assembly uses `recipe_probe::PROFILE_SCHEMA`
(`7`) and the probe codec's versioned binary format.

### `MemberProfileIdentity`

[`MemberProfileIdentity`](../src/model.rs#L15-L77) is the expected content-bound
identity for one configured member. `derive` first calls
`MeasuredProfileCodec::encode`, so a member identity cannot be derived from an
uncanonical, structurally invalid, or non-measured profile. It then requires
exactly one topology machine, exactly one machine origin, and a present first
origin.

It derives two endpoint digests:

- `profile` is SHA-256 of the complete canonical measured-profile bytes.
- `machine` is SHA-256 of the domain string
  `recipe-cluster-machine-identity-v3` followed by the retained stable machine
  ID bytes from the machine fingerprint.

Those digests are passed to `recipe_transport::EndpointIdentity::new`, which
rejects zero values. The identity also retains the member's cache, topology,
and discovery identities. The four fields are private; callers can only derive
them from a profile and read them through `endpoint`, `cache`, `topology`, and
`discovery`.

The one-machine requirement is intentional. A profile that already contains
peer machines is not a valid input member profile for this boundary. Peer
links enter cluster assembly as `MeasuredNetworkPair` records, not as an
already assembled remote topology hidden inside a member submission.

### `MemberSpec`

[`MemberSpec`](../src/model.rs#L79-L104) is one user-controlled membership
entry: a validated `Label` key, an opaque `Label` endpoint address, and an
expected `MemberProfileIdentity`. The address is not parsed, resolved, or used
to create transport sockets here. It is retained for canonical identity
hashing and for the caller's external endpoint policy. The expected identity
must normally be obtained with `MemberProfileIdentity::derive` from the
profile that the endpoint is expected to submit.

### `PairKey` and `NetworkPairSpec`

`PairKey` is private and orders its two member labels lexicographically. Equal
labels are rejected as a self-pair. Reversed declarations therefore represent
the same pair, and their canonical display form is `first<->second`.

[`NetworkPairSpec`](../src/model.rs#L133-L150) exposes only the canonical pair
labels. It contains no transport, bandwidth, capacity, duplex, or lane fields.

### `ClusterConfiguration`

[`ClusterConfiguration`](../src/model.rs#L152-L239) contains only:

- `master: Label`,
- sorted `MemberSpec` membership, and
- sorted `NetworkPairSpec` edges.

`new` enforces the complete declaration boundary:

- at least two members;
- unique member keys;
- unique endpoint address labels;
- unique endpoint machine digests;
- unique endpoint profile digests;
- the master is a member;
- every pair endpoint is a member;
- no duplicate canonical pair; and
- the undirected pair graph is connected.

Both member and pair vectors are sorted before storage. `require_connected`
starts at the first sorted key and repeatedly expands reached endpoints for
`keys.len()` passes. A connected spanning graph is sufficient; the
configuration does not require every possible member-to-member pair.

### `SubmittedProfile`

[`SubmittedProfile`](../src/model.rs#L274-L285) associates a member key with a
`MeasuredProfile`. Its constructor is intentionally infallible and performs no
validation. Assembly later consumes the values, indexes them by key, rejects
duplicate submissions, and validates the actual profile against the configured
identity.

### `MeasuredNetworkPair`

[`MeasuredNetworkPair`](../src/model.rs#L287-L394) is the only public cluster
input that carries numeric inter-machine evidence. The fields are crate-private
so callers must use `from_probe`:

```text
local member key
remote member key
authenticated SessionIdentity
PeerDescriptor
MeasuredLocalMemory
PeerMeasurement
BoundedBenchmarkPlan
```

The separation is architectural: membership configuration names endpoints,
while probe and transport result types supply capacities, rates, duplex mode,
lane counts, asynchronous capability, and benchmark observations.

## Peer evidence admission

`MeasuredNetworkPair::from_probe` performs the checks below before assembly can
see the record:

1. Local and remote member labels differ.
2. The benchmark plan is bounded and nonzero.
3. The descriptor advertises asynchronous submission.
4. The descriptor duplex equals `TransportKind::required_duplex()`.
5. The transport is `Ethernet` or `Wlan`; storage and local-device transports
   are not accepted as inter-machine links.
6. Remote memory capacity and rate have `Measured` provenance.
7. Both outbound and inbound throughput values exist and have `Measured`
   provenance.
8. `validate_peer_evidence` binds the evidence protocol schema and both endpoint
   machine/profile digests to the authenticated `SessionIdentity`.
9. Full duplex requires simultaneous evidence; half duplex requires serialized
   evidence.
10. For each direction, total bytes equal `buffer_bytes * iterations`, elapsed
    time is nonzero and within the plan, sample count equals the plan's
    iteration count, sample extrema are ordered and bounded by total elapsed
    time, and the stored rate equals the duration-derived bytes/second value.

Transport supplies the authenticated session and local memory result. In the
TCP implementation, frame headers carry both endpoint identities, sequence
numbers, payload length, and a SHA-256 payload digest. Full duplex measures both
directions simultaneously; half duplex serializes direction order by endpoint
machine digest. The cluster crate does not rerun or reinterpret this transport
protocol. It checks the resulting evidence again at its own boundary.

## Assembly pipeline

The public [`assemble_cluster`](../src/assemble.rs#L128-L173) function is a
single fail-closed pipeline:

```text
configuration + submissions + network measurements
        │
        ├─ prepare_members
        ├─ common_benchmarks
        ├─ resolve_network / resolve_pair / resolve_gateway
        ├─ derive topology, discovery, and profile digests
        ├─ remap_profile / add measured peer links
        └─ canonical MeasuredProfileCodec validation
                     │
                     ▼
              MeasuredProfile
```

### 1. `prepare_members`

`index_submissions` turns submissions into a `BTreeMap<Label,
MeasuredProfile>` and rejects duplicate keys. For every sorted configured
member, assembly:

- requires a submission;
- derives `MemberProfileIdentity`, converting profile codec failures into an
  `InvalidMember` message naming the member;
- requires exact equality with `MemberSpec::expected`;
- rejects duplicate submitted profile digests; and
- requires unique topology machine names.

After configured members are consumed, any remaining submission is an
unconfigured member and fails. The resulting `PreparedMember` retains the
configured key, derived identity, and owned measured profile.

### 2. `common_benchmarks`

The first prepared member supplies `BenchmarkMetadata`. Every other member must
have byte-for-byte equal metadata, including seed schema and all four bounded
RAM, storage, GPU, and network plans. Assembly never selects a plan or merges
different plans. This makes the network evidence comparable and gives the final
profile one authoritative benchmark metadata value.

### 3. `resolve_network`

Measurements are indexed by their canonical `PairKey`. Duplicate evidence for
one pair, missing evidence for a configured pair, and extra evidence for an
unconfigured pair are separate failures. The configured pair list is already
sorted by `ClusterConfiguration::new`, so the resolved vector is deterministic.

`resolve_pair` then:

- finds both evidence member keys in the prepared-member map;
- requires the session's local endpoint to equal the local member endpoint and
  its remote endpoint to equal the remote member endpoint;
- requires the evidence benchmark plan to equal common network metadata;
- requires the descriptor machine fingerprint to equal the remote profile's
  retained machine origin;
- resolves the descriptor's local and remote RAM keys to exact profile devices;
- checks the supplied capacities and transfer rates against those exact RAM
  devices; and
- normalizes outbound/inbound rates, lane counts, and memory facts into the
  lexicographically first and second pair members.

The resolved record also retains session, link, driver, firmware, stable-ID,
runtime-ABI, machine-firmware, memory-domain, interface, benchmark, rate, and
full peer-evidence labels for the identity hash. No rate or capacity is inferred
from a name, address, ordinal, or similarity.

`resolve_gateway` is deliberately key-based. It searches only
`MeasuredRamOrigin.key` in the selected member, verifies that the origin points
to an existing device, and requires exact equality of the measured capacity and
transfer rate. A peer descriptor cannot redirect a link to an arbitrary device
with matching performance.

### 4. Identity derivation

Assembly computes three domain-separated SHA-256 values over the same resolved
canonical input:

- `recipe-cluster-topology-v3` becomes `TopologyIdentity`;
- `recipe-cluster-discovery-v3` becomes `DiscoveryIdentity`; and
- `recipe-cluster-profile-v3` becomes the profile cache digest.

`CacheIdentity.schema` is `PROFILE_SCHEMA`, while `CLUSTER_SCHEMA` is included
inside all three cluster digests. A change to membership, address, expected
member identity, benchmark metadata, measured direction, duplex/resource
semantics, gateway, peer descriptor, or benchmark evidence changes the relevant
cluster identity.

### 5. `remap_profile`

Member-local IDs cannot be concatenated directly because every per-machine
profile may start its IDs at one. `IdAllocator` starts six independent global
spaces at one: machines, nodes, devices, links, transports, and duplex
resources. `allocate_member` creates a `MemberRemap` for all local machines,
nodes, devices, links, link transports, and link capacity resources. The maps
are populated in canonical profile order; transport and resource sets use
`BTreeSet` ordering.

Members are processed in sorted configuration-key order. The pair vector is
also sorted. Consequently, submission order, measurement order, and reversed
pair declaration order cannot change the allocated IDs or any digest input.
An overflow while allocating any identity returns `IdentityExhausted`.

`remap_member` copies each member's machines, machine origins, devices, RAM,
storage, and GPU origins, directed local links, and discovery records through
the remap maps. It also rewrites roles:

```text
configured master member + local Master node → Master
everything else                         → Worker
```

The profile codec and core topology validator still enforce that the final
profile has exactly one master. A malformed one-machine member that has no
master node therefore fails during final validation rather than being silently
repaired.

### 6. Adding measured network links

For every `ResolvedNetworkPair`, `push_network_pair` adds two opposing
`DirectedLink` values:

- one shared `TransportId`;
- `Full` duplex gets distinct forward and reverse `DuplexResourceId` values;
- `Half` duplex shares one capacity resource;
- each direction receives its own measured bandwidth and measured lane count;
- both directions are marked available and retain the measured asynchronous
  submission capability.

The link endpoints are the exact remapped RAM gateway devices selected above.
The result is the same bidirectional representation required by
`recipe_core::Topology::validate` and by the probe codec's cross-machine peer
checks.

### 7. Constructing and validating the profile

The final `MeasuredProfile` contains:

- `PROFILE_SCHEMA` and the cluster cache identity;
- remapped machine, RAM, storage, and GPU origins;
- common benchmark metadata;
- one `MeasuredPeerBenchmark` per resolved network pair, sorted by session ID;
- a topology with all local and added peer links; and
- a discovery profile tied to the new topology identity, with all device and
  link capabilities copied or added from measured evidence.

`assemble_cluster` then calls `MeasuredProfileCodec::encode` and maps any
failure to `InvalidClusterProfile`. This final call checks the complete probe
profile contract, not only the cluster-specific graph shape.

## Canonical hash format

[`hash.rs`](../src/hash.rs) defines `CanonicalDigest`. It starts SHA-256 with a
domain string and `CLUSTER_SCHEMA`. Strings and byte slices are length-prefixed
with a little-endian `u64`; integer fields use little-endian `u32`/`u64`, booleans
use one byte, and `Digest` values contribute their raw 32 bytes. Length
conversion overflow is reported as `InvalidClusterProfile`; the finished value
is a `recipe_core::Digest`.

For each digest domain, the serialized sequence is:

1. master label;
2. seed schema and all four benchmark plans, including duration nanoseconds;
3. for every sorted member, a `member` marker, key, address, expected machine
   and profile endpoint digests, expected cache schema/digest, expected topology
   and discovery digests, and the actual submitted profile endpoint profile
   digest;
4. for every sorted network pair, a `network-pair` marker, pair labels,
   transport and duplex tags, directional rates and provenance tags, lane
   counts, asynchronous flag, both gateway memory capacity/rate values, all
   retained evidence labels, the pair benchmark plan, measured evidence rates,
   and the complete `PeerBenchmarkEvidence`; and
5. the peer evidence protocol schema, endpoint machine/profile digests,
   simultaneous/serialized tag, and both directional byte/time/sample/variance
   records.

Transport-kind tags are `Memory=0`, `Pcie=1`, `Sata=2`, `Sas=3`, `Nvme=4`,
`Ethernet=5`, and `Wlan=6`. Duplex tags are `Half=0` and `Full=1`. Property
provenance tags are `Estimated=0`, `Measured=1`, and `Override=2`. The assembly
path only admits measured peer values, but provenance is still hashed so a
future explicitly permitted provenance change cannot collide with this one.

The member profile digest used by `MemberProfileIdentity` is different from
these cluster digests: it is SHA-256 over the probe codec's complete canonical
bytes. The machine endpoint digest is intentionally based only on the retained
stable machine ID, with its own domain string.

## `ClusterProfileCodec`

[`ClusterProfileCodec`](../src/assemble.rs#L783-L799) is a thin cluster facade,
not a second wire format:

- `encode` first applies `validate_cluster_shape`, then delegates to
  `MeasuredProfileCodec::encode` and maps the probe error to `ClusterError::Codec`;
- `decode` delegates to `MeasuredProfileCodec::decode`, applies the same shape
  check, and returns the profile; and
- the bytes are therefore exactly the canonical measured-profile bytes used by
  `recipe-probe` caches.

`validate_cluster_shape` adds only cluster facts before or after the general
codec validation:

- at least two topology machines;
- every topology link endpoint names a known device; and
- the undirected graph formed by links connecting different machines reaches
  every machine from the first machine.

Same-machine links do not contribute to this inter-machine adjacency. The
general codec still validates IDs, node ownership and master count, device
kinds, duplex/resource pairing, origins, canonical ordering, measured
provenance, topology/discovery equality, peer benchmark consistency, and all
discovery capabilities.

## Error surface

[`ClusterError`](../src/error.rs#L5-L66) is `non_exhaustive`, cloneable, and
`Eq`. Its variants and principal sources are:

| Variant | Meaning |
| --- | --- |
| `InvalidConfiguration` | Membership, pair, master, or connectivity declaration is invalid. |
| `MissingMember` | A configured key has no submitted profile. |
| `DuplicateMember` | A submission key, profile identity, or member key is repeated. |
| `StaleMember` | A submitted profile's derived identity differs from the expected identity. |
| `InvalidMember` | A member profile is malformed, has duplicate machine names, or is unconfigured. |
| `MissingNetworkMeasurement` | A configured canonical pair has no evidence. |
| `DuplicateNetworkMeasurement` | More than one evidence record resolves to one pair. |
| `InvalidNetworkMeasurement` | Session, descriptor, benchmark, gateway, provenance, duplex, or peer evidence fails. |
| `IdentityExhausted` | One global ID allocator counter cannot advance. |
| `InvalidClusterProfile` | A constructed or submitted profile violates canonical/profile/cluster shape rules. |
| `Codec` | The explicit codec facade failed while encoding or decoding bytes. |

`Display` adds stable human-readable context such as `member X did not submit a
measured profile`, `network pair A<->B has no measured probe evidence`, or
`invalid cluster profile: ...`. `ClusterResult<T>` is simply
`Result<T, ClusterError>`. There are no retry, substitute, guessed-address, or
fallback paths.

## Downstream handoff to planning and scheduling

The direct consumer boundary is `recipe-prepare`, not this crate. Its
`Preparer::prepare_program` receives a `&MeasuredProfile`, calls
`recipe_probe::validate_profile`, and passes only `profile.topology` and
`profile.discovery` into `recipe_planner::plan_program_candidates`. That means
cluster assembly must finish every identity, origin, capability, and measured
property before preparation begins.

`plan_program_candidates` validates the topology, scheduling properties,
discovery, reservations, and capacity before lowering. The planner uses the
cluster-produced values in several concrete ways:

- available GPU `DeviceKind::GpuMemory` entries with discovery calculation
  capabilities are the legal calculation placement choices;
- measured subgroup/workgroup/shared-memory capabilities form common lowering
  hardware;
- measured link topology supplies all directed routes considered for tensor
  copies;
- measured link bandwidth determines trial transfer times and route choice;
- measured link lane counts, duplex resources, device queue counts, and overlap
  capabilities constrain candidate scheduling; and
- candidate identity hashes both `TopologyIdentity` and `DiscoveryIdentity`.

When a transfer needs more than one physical hop, the planner enumerates a
directed route, creates one dependency-chained `TransferTask` per link, and
creates resident intermediate values. This is required because the scheduler
accepts an executor-visible internal transfer with at most one link.

`recipe_scheduler::schedule` independently validates the same topology and
discovery profile. It computes calculation duration from measured FLOP rate and
transfer duration from measured link bandwidth, reserves measured compute and
transfer lanes, contends opposing half-duplex links through their shared
capacity resource, and honors discovered transfer/calculation overlap. It
persists exact lane claims in the scheduled `Task`. `pack_arenas` then uses the
same topology's device IDs and capacity ledger.

The planner's `DraftPlan` and the scheduler's `StaticSchedule` do not accept a
`MeasuredProfile` directly. They rely on the profile's topology/discovery pair,
their matching identities, and the profile codec's guarantee that every
required device/link capability is present and measured. A cluster identity
change consequently changes planner candidate/draft identities and cannot be
silently reused by preparation or realization.

## Boundaries and non-responsibilities

The following are intentionally outside this crate:

- host/GPU enumeration and local benchmark execution, owned by `recipe-probe`
  and `recipe-native-probe`;
- TCP framing, authenticated session identities, deadlines, cancellation, and
  peer benchmark execution, owned by `recipe-transport`;
- current-inventory reopening by retained machine/domain/GPU keys, owned by
  probe resolution and native preparation;
- graph lowering, route enumeration, transfer-chain construction, candidate
  identity, and capacity-aware draft generation, owned by `recipe-planner`;
- static timing, lane/resource reservation, schedule windows, and arena packing,
  owned by `recipe-scheduler`; and
- artifact compilation, backend realization, stabilization, finalization, and
  execution, owned by `recipe-prepare` and the native executor crates.

Cluster configuration cannot override measured values. A missing profile,
stale identity, missing pair, malformed evidence, disconnected graph, unknown
gateway, or canonical codec violation is returned as an error. The real
profile and its evidence remain the sole source of facts passed to the
downstream measured scheduler.
