# `cluster/src/assemble.rs`

## Scope

`assemble.rs` is the deterministic composition boundary for a measured cluster.
It accepts already measured, already identity-bearing per-machine profiles and
separately authenticated peer-probe results, then produces one canonical
`recipe_probe::MeasuredProfile` containing the global topology, discovery
capabilities, retained provenance, and peer benchmark evidence.

The module does not discover hardware, run a benchmark, choose a transport, or
invent a capacity or rate. It only checks that the supplied evidence agrees
with the user-controlled membership declaration, allocates one global identity
space, and copies measured values into the canonical profile representation.
The module is private; `cluster/src/lib.rs` re-exports only
`assemble_cluster` and `ClusterProfileCodec`.

The central operation is:

```text
ClusterConfiguration
        + Vec<SubmittedProfile>
        + Vec<MeasuredNetworkPair>
        -> prepare_members
        -> common_benchmarks
        -> resolve_network
        -> three domain-separated cluster digests
        -> remap_profile
        -> canonical MeasuredProfileCodec validation
        -> MeasuredProfile
```

Every fallible operation is fail-fast. There are no retries, alternate
selectors, guessed values, or fallback identities.

## Public entry points and call graph

### `assemble_cluster` (`assemble.rs:128-173`)

`assemble_cluster` is the only constructor for a complete cluster profile in
this module. Its call graph is deliberately linear:

1. `prepare_members` indexes and validates all member submissions.
2. `common_benchmarks` requires one identical benchmark metadata value across
   all member profiles.
3. `resolve_network` indexes every peer measurement, checks that it exactly
   covers the configured pair set, and turns each measurement into a
   `ResolvedNetworkPair` in configuration order.
4. `cluster_digest` is called with three different domains to produce the
   topology identity, discovery identity, and profile cache digest.
5. `remap_profile` allocates a global ID space, copies member data, adds the
   inter-machine links, and constructs the final `MeasuredProfile`.
6. `MeasuredProfileCodec::encode` validates the complete result. The encoded
   bytes are intentionally discarded. This call is a validation gate, not a
   persistence operation.

The returned profile is the only successful output. Each stage keeps its own
error boundary: member identity failures are wrapped as `InvalidMember`,
network binding failures remain `InvalidNetworkMeasurement`, and the final
canonical codec failure is converted to `InvalidClusterProfile`.

### `ClusterProfileCodec` (`assemble.rs:783-799`)

`ClusterProfileCodec` is a cluster-specific facade over the canonical probe
profile codec:

```text
encode(profile)
  -> validate_cluster_shape(profile)
  -> MeasuredProfileCodec::encode(profile)

decode(bytes)
  -> MeasuredProfileCodec::decode(bytes)
  -> validate_cluster_shape(profile)
  -> profile
```

Unlike the final validation inside `assemble_cluster`, this facade preserves a
separate `ClusterError::Codec` category for codec failures. It is useful when a
caller wants to persist or load a complete cluster profile and still require
the additional multi-machine shape constraint.

There are currently no in-tree call sites for either public entry point beyond
the re-exports in `cluster/src/lib.rs` and the `recipe::engine::cluster`
facade in `src/facade.rs`. The workspace documents `recipe-cluster` as the
assembly crate, but callers are expected to invoke these APIs explicitly.

## Inputs and their ownership boundaries

### `ClusterConfiguration`

`ClusterConfiguration` is defined in `cluster/src/model.rs:152-239`. It is
deliberately only membership and connectivity policy. It contains:

- one master member label;
- `MemberSpec` values, each with a member key, an endpoint address, and the
  expected `MemberProfileIdentity`;
- `NetworkPairSpec` values naming the required undirected inter-member links.

It contains no device IDs, capacities, bandwidths, lane counts, transport
families, or topology objects. `ClusterConfiguration::new` sorts members by
key and pairs by their normalized `PairKey`; rejects fewer than two members,
duplicate keys or addresses, repeated machine or profile identities, a master
outside membership, pairs outside membership, duplicate pairs, and a
disconnected pair graph. Therefore the assembly code can iterate the exposed
slices as canonical configuration order rather than preserving caller vector
order.

`assemble_cluster` does not reconstruct or revalidate this configuration. The
constructor is the configuration validation boundary. A configuration value
reaching assembly is assumed to have passed those checks.

### `SubmittedProfile`

`SubmittedProfile` (`model.rs:274-285`) is a public `(Label,
MeasuredProfile)` wrapper. Its fields are private, so external callers use
`SubmittedProfile::new`. `index_submissions` consumes the wrappers and builds a
`BTreeMap<Label, MeasuredProfile>`; duplicate labels fail before any profile is
used.

The submitted profile must be a canonical measured profile and must describe
exactly one machine with exactly one machine origin. That shape is enforced by
`MemberProfileIdentity::derive` (`model.rs:27-64`) before assembly accepts the
member. `ProbeEngine::probe` (`probe/src/engine.rs:63-160`) adds a machine,
remote RAM device, and worker node for each supplied peer session, so a
per-machine submission normally comes from a probe with no peer sessions (or
an equivalent one-machine producer). A profile containing peer-session
machines is not valid at this boundary. Peer evidence enters the cluster
through `MeasuredNetworkPair`, not through extra machines in a member
submission.

### `MeasuredNetworkPair`

`MeasuredNetworkPair` (`model.rs:287-394`) is the public carrier for one
authenticated, two-direction peer measurement. Its fields are private; callers
use `MeasuredNetworkPair::from_probe` with:

| Input | Source and purpose |
| --- | --- |
| `local_member`, `remote_member` | Configuration labels identifying the two endpoints. They must differ. |
| `SessionIdentity` | Authenticated local and remote `EndpointIdentity` values. |
| `PeerDescriptor` | Remote machine fingerprint, RAM/interface discovery keys, transport family, duplex mode, directional lane limits, asynchronous-submission capability, and evidence labels. |
| `MeasuredLocalMemory` | The local RAM capacity and transfer rate observed by the peer session. |
| `PeerMeasurement` | Remote RAM properties, optional directional throughput properties, and structured benchmark evidence. |
| `BoundedBenchmarkPlan` | The exact network plan used by both directions of the benchmark. |

`from_probe` performs the first validation layer: distinct members, a bounded
plan, asynchronous submission, transport-kind/duplex compatibility, an
inter-machine network kind (`Ethernet` or `Wlan`), measured provenance for
remote memory and both rates, and endpoint-bound, duration-consistent peer
evidence. `assemble.rs` performs the second layer because only assembly knows
which configured member each authenticated endpoint is supposed to represent.

`PeerSession` implementations in `recipe_probe` and `recipe_transport` are
the intended producers of `PeerDescriptor`, `PeerMeasurement`, and
`SessionIdentity`. `TcpPeerSession` implements the trait, while
`MeasuredLocalMemory` is re-exported by `recipe_transport`. There is no
in-repository orchestration that calls `MeasuredNetworkPair::from_probe`; the
cluster API is the boundary for such a caller.

## Internal state used during assembly

### `PreparedMember` (`assemble.rs:23-28`)

`PreparedMember` holds the configured key, the derived and checked
`MemberProfileIdentity`, and the owned canonical member `MeasuredProfile`.
The vector is in `ClusterConfiguration::members()` order.

### `ResolvedNetworkPair` (`assemble.rs:30-61`)

This is the normalized, assembly-ready representation of one configured pair.
Its fields fall into four groups:

| Group | Fields and use |
| --- | --- |
| Topology direction | `key`, `first_device`, `second_device`, `kind`, `duplex`, `first_to_second`, `second_to_first`, `first_to_second_lanes`, `second_to_first_lanes`, and `asynchronous_submission`. These become two global `DirectedLink` values and two `DiscoveredLink` values. |
| Gateway identity evidence | `first_memory_capacity`, `first_memory_rate`, `second_memory_capacity`, and `second_memory_rate`. These are checked against the exact RAM origins and then retained for identity hashing. They are not copied into the final topology because the RAM devices already carry those properties. |
| Descriptor provenance | `evidence_session`, `evidence_link`, `evidence_remote_driver`, `evidence_remote_firmware`, `evidence_remote_stable_id`, `evidence_remote_runtime_abi`, `evidence_remote_machine_firmware`, `evidence_remote_memory`, `evidence_local_memory`, `evidence_local_interface`, and `evidence_remote_interface`. These bind the resulting identity to the peer descriptor. They are consumed by `cluster_digest`, not emitted as separate topology fields. |
| Benchmark evidence | `benchmark`, `evidence_outbound_rate`, `evidence_inbound_rate`, and `benchmark_evidence`. The rates and evidence become `MeasuredPeerBenchmark` records; the whole set also contributes to the profile digest. |

### `IdAllocator` and `MemberRemap` (`assemble.rs:63-126`)

`IdAllocator::initialized` starts six independent counters at `1`, so every
generated `MachineId`, `NodeId`, `DeviceId`, `LinkId`, `TransportId`, and
`DuplexResourceId` is nonzero. `take_id` returns the current value and uses
checked increment. If incrementing would overflow, it returns
`ClusterError::IdentityExhausted` for the relevant kind rather than wrapping.

`MemberRemap` maps each member profile's local IDs to the new global IDs. It
has one map for each of the six ID classes. A transport or capacity resource
is mapped once per unique source ID, even when several local links share it.

### `ClusterCollections` (`assemble.rs:477-489`)

This temporary accumulator owns the output vectors while member data and
network data are appended:

- `machines`, `nodes`, and `devices` become `Topology` fields;
- `links` becomes the complete local plus inter-machine directed-link list;
- `origin_machines`, `origin_ram`, `origin_storage`, and `origin_gpu` become
  `MeasuredOrigins`;
- `discovered_devices` and `discovered_links` become `DiscoveryProfile` fields.

The accumulator does not retain configuration labels. Labels are used to
resolve and order data, while topology references use the newly allocated IDs.

## Assembly pipeline

### 1. Index and validate member submissions: `prepare_members`

`prepare_members` (`assemble.rs:175-236`) executes the following exact steps:

1. Consume `submissions` through `index_submissions`. The `BTreeMap` rejects
   duplicate submitted keys with `ClusterError::DuplicateMember`.
2. Allocate vectors sized to the configured member count and empty sets for
   profile endpoint digests and machine names.
3. For each configured `MemberSpec`, in sorted configuration order:
   - remove the profile under `spec.key()`; absence returns
     `ClusterError::MissingMember`;
   - derive `MemberProfileIdentity` by canonical-encoding the profile, requiring
     exactly one topology machine and one machine origin;
   - compare the derived identity byte-for-byte with `spec.expected()`;
     disagreement returns `ClusterError::StaleMember`;
   - reject a repeated actual endpoint profile digest with
     `ClusterError::DuplicateMember`;
   - read the first topology machine and reject a profile with no machine using
     `ClusterError::InvalidMember`;
   - reject a repeated machine name with `InvalidMember`;
   - append a `PreparedMember`.
4. After configured members are removed, inspect the first remaining map entry.
   Any leftover submission is an unconfigured member and returns
   `InvalidMember`; an empty map completes the stage.

The order of the original `submissions` vector has no effect because indexing
is by key and output order follows the already sorted configuration.

`MemberProfileIdentity::derive` binds four values from the member profile:

- the endpoint machine digest is SHA-256 of the stable machine ID under the
  `recipe-cluster-machine-identity-v3` domain;
- the endpoint profile digest is SHA-256 of the canonical encoded profile;
- the cache identity is copied from the profile;
- topology and discovery identities are copied from the profile.

The endpoint constructor rejects zero machine or profile digests. Any member
codec or shape error is wrapped as `InvalidMember` with the configured member
key.

### 2. Require one benchmark plan: `common_benchmarks`

`common_benchmarks` (`assemble.rs:238-254`) copies the first prepared profile's
`BenchmarkMetadata`, then compares the complete value with every remaining
profile. A mismatch is `InvalidMember` for the member that differs. An empty
prepared vector is an `InvalidConfiguration`, although a valid
`ClusterConfiguration` already requires at least two members.

This shared value is later used for two independent purposes:

- `benchmarks.network` is the exact plan that every `MeasuredNetworkPair` must
  report;
- all four plan values and the seed schema become part of every cluster digest.

No benchmark is executed or modified here.

### 3. Index and resolve peer measurements: `resolve_network`

`resolve_network` (`assemble.rs:256-292`) first builds a `BTreeMap<PairKey,
MeasuredNetworkPair>` from the supplied vector. `MeasuredNetworkPair::pair_key`
normalizes local and remote labels through `PairKey::new`, so reversed
observations address the same undirected pair. A duplicate normalized key is
`DuplicateNetworkMeasurement`.

It then builds a label-to-`PreparedMember` index and iterates the configured
network pairs in sorted order. For each pair it:

1. constructs the normalized configuration key;
2. removes the matching measurement, returning
   `MissingNetworkMeasurement` if absent;
3. calls `resolve_pair` and appends the normalized result.

After all configured pairs are consumed, any remaining measurement is evidence
for an unconfigured pair and returns `InvalidNetworkMeasurement`. Thus the
measurement set must be exactly equal to the configured pair set, not merely a
superset.

#### `resolve_pair`

`resolve_pair` (`assemble.rs:294-436`) binds one evidence record to the two
configured member profiles:

1. Look up `evidence.local_member` and `evidence.remote_member`; unknown labels
   fail as `InvalidNetworkMeasurement`.
2. Require `evidence.session.local()` to equal the local member's derived
   endpoint and `evidence.session.remote()` to equal the remote member's
   derived endpoint. This is the assembly-time check that authenticated session
   identities name the configured members.
3. Require `evidence.benchmark == common_benchmarks(...).network`. A stale or
   different plan is rejected even if all numeric evidence is otherwise valid.
4. Require a remote machine origin and compare the complete
   `PeerDescriptor.machine` fingerprint with that origin. This binds the peer
   descriptor to the profile selected for `remote_member`.
5. Resolve the local and remote RAM gateway devices through `resolve_gateway`.
6. Require both optional throughput measurements to be present.
7. Convert `LinkDuplex::Full/Half` to the corresponding core
   `DuplexMode::Full/Half`.
8. Normalize direction. `PairKey` orders labels lexicographically, but the
   measured session may have been opened in either direction:

   | Condition | `first` side | `second` side |
   | --- | --- | --- |
   | `evidence.local_member == key.first` | local device, outbound rate, outbound lanes, local RAM values | remote device, inbound rate, inbound lanes, remote RAM values |
   | otherwise | remote device, inbound rate, inbound lanes, remote RAM values | local device, outbound rate, outbound lanes, local RAM values |

9. Copy the normalized topology values, memory values, descriptor provenance,
   benchmark plan, rates, and structured peer evidence into `ResolvedNetworkPair`.

The `false` orientation branch is required because transport measurement and
cluster ordering use different notions of direction. Full-duplex transport
measures both directions simultaneously; half-duplex transport serializes the
direction whose endpoint machine digest sorts first, while `PairKey` sorts the
configured member labels. `resolve_pair` converts either transport orientation
to the label-ordered topology direction. It does not reverse the
`SessionIdentity` or rewrite the evidence; only the topology-facing
first/second fields are normalized. The original endpoint ordering remains in
the hashed peer evidence.

#### `resolve_gateway`

`resolve_gateway` (`assemble.rs:438-475`) never selects a RAM device by size,
rate, ordinal, or proximity. It:

1. finds the exact `MeasuredRamOrigin.key` in the selected member profile;
2. follows that origin to its topology device;
3. compares the device's capacity and transfer-rate values with the values
   carried by the peer evidence;
4. returns the existing member-local `DeviceId`.

Missing origin keys, missing devices, or any capacity/rate mismatch return
`InvalidNetworkMeasurement`. The `side` argument only affects the diagnostic
text (`local` or `remote`). The memory values are retained in the resolved
pair for identity hashing, while the output topology keeps one canonical RAM
device per origin.

### 4. Derive identities: `cluster_digest`

The assembly stage calls `cluster_digest` (`hash.rs:50-109`) three times:

| Output | Domain string | Destination |
| --- | --- | --- |
| topology identity | `recipe-cluster-topology-v3` | `Topology.identity` |
| discovery identity | `recipe-cluster-discovery-v3` | `DiscoveryProfile.identity` |
| profile identity | `recipe-cluster-profile-v3` | `CacheIdentity.digest` |

Each digest starts with a length-prefixed domain string and `CLUSTER_SCHEMA`,
then hashes the following canonical sequence:

1. the configured master label;
2. shared benchmark seed schema and all four bounded plans, including buffer
   bytes, iteration count, and canonical nanoseconds for maximum duration;
3. each configured member and its prepared profile, in configuration order:
   member key, endpoint address, every expected identity component (machine and
   profile endpoint digests, cache schema and digest, topology digest, and
   discovery digest), and the actual member endpoint profile digest;
4. each resolved network pair, in normalized pair order: pair labels,
   transport and duplex tags, both directional rate values and provenance,
   both lane counts, asynchronous-submission flag, both gateway capacity/rate
   tuples, every retained descriptor evidence label, the bounded peer plan,
   both measured evidence rates and provenance, and all structured peer evidence
   fields (protocol, endpoint digests, execution mode, and per-direction sample
   statistics).

The three domain strings are the only intentional difference between the
identity calculations. Any change to membership, expected identity, measured
profile digest, peer descriptor, benchmark plan, gateway measurement, rate,
lane count, or benchmark evidence changes the relevant digest. The profile
identity is stored with `PROFILE_SCHEMA` in `CacheIdentity`.

### 5. Allocate and remap the global profile: `remap_profile`

`remap_profile` (`assemble.rs:491-579`) constructs one global ID namespace and
one output collection:

1. Initialize `IdAllocator`.
2. For each prepared member, call `allocate_member` and store its
   `MemberRemap` under the member key.
3. For each prepared member, look up its mapping and call `remap_member`.
4. For each resolved network pair, look up both member mappings, map the
   normalized endpoint device IDs, and call `push_network_pair`.
5. Move the collections into `Topology` and `DiscoveryProfile` values.
6. Convert each resolved pair to a `MeasuredPeerBenchmark`, then sort those
   records by `session_id`.
7. Return a `MeasuredProfile` with `PROFILE_SCHEMA`, the supplied cache identity,
   collected origins, shared benchmarks, sorted peer benchmarks, topology, and
   discovery.

Missing member mappings or endpoint mappings are `InvalidClusterProfile`; they
represent an internal inconsistency between the earlier allocation and remap
passes, not a selector fallback.

#### `allocate_member`

`allocate_member` (`assemble.rs:581-616`) walks one canonical member profile
and allocates fresh IDs for every machine, node, device, and local link. It
collects the source transport IDs and capacity-resource IDs in `BTreeSet`s so
each unique source identity receives one destination identity. Allocation is
performed in source profile order for machines, nodes, devices, and links, and
in sorted source-ID order for transport/resource sets. Because the allocator is
shared across members, no two member profiles can retain colliding numeric IDs.

Inter-machine transports are not allocated here. They are created later by
`push_network_pair`, after the local profile namespace has been reserved.

#### `remap_member`

`remap_member` (`assemble.rs:618-717`) copies all member-owned data and rewrites
every ID reference through `mapped`:

| Source data | Output behavior |
| --- | --- |
| `topology.machines` | New machine IDs, original machine names. |
| `origins.machines` | New machine IDs, original complete machine fingerprints. |
| `topology.nodes` | New node and machine/device IDs. Role is normalized using the configured master key. |
| `topology.devices` | New device/machine IDs, original kind and all measured capacity, transfer, and optional calculation properties. |
| `origins.ram`, `.storage`, `.gpu` | New device IDs, original stable discovery keys. |
| `topology.links` | New link, transport, endpoint, and capacity-resource IDs; original kind, duplex, bandwidth, and lane properties. |
| `discovery.devices` | New device IDs; original availability and every transfer/calculation capability. |
| `discovery.links` | New link IDs; original availability, bandwidth, lane count, and asynchronous-submission flag. |

Node roles are intentionally rewritten rather than trusted blindly:

- a node from the configured master member is `Master` only if its source role
  is `Master`;
- a worker-role node on the master member remains `Worker`;
- every node from a non-master member becomes `Worker`, even if the source
  incorrectly labels it `Master`.

The canonical codec later enforces exactly one master node and all other
topology ownership rules. `remap_member` itself does not add a second rejection
branch for impossible source roles.

`mapped` (`assemble.rs:719-724`) is the single lookup helper. A missing entry
returns `InvalidClusterProfile` with the object kind, so a malformed source
reference cannot silently survive remapping.

#### `push_network_pair`

`push_network_pair` (`assemble.rs:726-780`) adds the global inter-machine
transport represented by one resolved pair:

1. Allocate one `TransportId` shared by both directions.
2. Allocate one forward capacity resource. For full duplex allocate a distinct
   reverse resource; for half duplex reuse the forward resource.
3. Allocate two `LinkId` values.
4. Wrap the directional lane counts as measured `Property<TransferLaneCount>`
   values.
5. Append forward and reverse `DirectedLink` values with opposite endpoints,
   directional measured bandwidth, directional measured lane limits, the
   resolved transport kind and duplex mode, and the appropriate resource.
6. Append two available `DiscoveredLink` values carrying the same directional
   bandwidth and lane properties plus the descriptor's asynchronous-submission
   flag.

No `DiscoveredDevice` is created here. Network gateways are existing RAM
devices from the two member profiles, and the discovery devices for those RAM
domains were already copied by `remap_member`.

The resulting link pair satisfies the core topology contract: one transport,
two opposing directed edges, shared capacity for half duplex, and independent
capacity resources for full duplex. The later canonical codec also requires
the link properties and discovery properties to match exactly.

### 6. Canonical result validation

After `remap_profile`, `assemble_cluster` calls
`recipe_probe::MeasuredProfileCodec::encode` (`assemble.rs:171`). The codec
validation order is `validate_profile`'s schema/cache checks, benchmark
metadata, peer evidence, core topology, topology scheduling properties,
discovery, origins, canonical ordering, measured provenance, topology/discovery
property equality, and peer-topology matching. It validates, among other
things:

- profile and cache schema values and a nonzero cache digest;
- bounded benchmark metadata and canonical durations;
- peer endpoint identities, directional evidence, and evidence-derived rates;
- core topology identity, unique IDs, references, node ownership, transport
  pairing, required duplex/resource relationships, and schedulable properties;
- discovery identity, topology identity match, complete device/link coverage,
  measured capabilities, and topology/discovery property equality;
- machine, RAM, storage, and GPU origin coverage and uniqueness;
- strict canonical ordering of all origin, topology, discovery, and peer vectors;
- measured-only provenance for persisted profile values;
- one-to-one matching between cross-machine transports and peer benchmark
  records, including endpoint pair uniqueness, directional rates, and duplex
  execution mode.

The bytes returned by `encode` are not stored. A failure is mapped to
`ClusterError::InvalidClusterProfile`, so the assembly boundary never returns a
partially validated profile.

## Resulting `MeasuredProfile`

`remap_profile` fills every field as follows:

| Field | Value in the assembled profile |
| --- | --- |
| `schema` | `recipe_probe::PROFILE_SCHEMA`. |
| `cache_identity` | `CacheIdentity { schema: PROFILE_SCHEMA, digest: profile_identity }`, where `profile_identity` is the profile-domain cluster digest. |
| `origins.machines` | All member machine fingerprints with globally remapped machine IDs. |
| `origins.ram`, `.storage`, `.gpu` | All member origin keys with globally remapped device IDs. |
| `benchmarks` | The one common `BenchmarkMetadata` copied from the member profiles. |
| `peer_benchmarks` | One record per configured network pair, carrying session ID, measured outbound/inbound rates, and structured benchmark evidence, sorted by session ID. |
| `topology.identity` | The topology-domain digest. |
| `topology.machines`, `.nodes`, `.devices`, `.links` | Member data plus one opposing link pair per configured network pair, all using global IDs. |
| `discovery.identity` | The discovery-domain digest. |
| `discovery.topology` | Exactly the assembled topology identity. |
| `discovery.devices` | Remapped member discovery devices. |
| `discovery.links` | Remapped member links plus the two available discovered directions for every network pair. |

The descriptor's driver, firmware, interface, and stable identity labels are
not separate `MeasuredProfile` fields. They still affect the profile, topology,
and discovery digests through `ResolvedNetworkPair`, so changing peer identity
evidence cannot leave the assembled identity unchanged.

## Cluster-specific codec shape check

`validate_cluster_shape` (`assemble.rs:801-859`) is intentionally narrower than
the canonical probe validator. It adds the two properties that distinguish a
cluster profile from a single-machine profile:

1. Require at least two topology machines.
2. Build a device-to-machine map and an undirected machine adjacency map.
3. Inspect every topology link. Unknown endpoint devices fail with
   `InvalidClusterProfile`. Intra-machine links do not add adjacency; links
   between different machines add both directions.
4. Start a reachability set at the first topology machine and repeatedly extend
   it through adjacency for `machines.len()` rounds.
5. Require that every topology machine is reached. A disconnected
   inter-machine transport graph returns `InvalidClusterProfile`.

`ClusterProfileCodec::encode` runs this shape check before the canonical codec;
`decode` runs the canonical codec first and the shape check second. The shape
check does not replace canonical topology validation: it relies on the codec to
reject duplicate machine IDs, unknown machine references, invalid transport
pairs, missing discovery records, invalid roles, and unschedulable properties.

### Canonical binary boundary

The delegated `MeasuredProfileCodec` is a versioned binary codec, not a
structural pass-through. On encode it validates first, writes the profile codec
schema, profile/cache identities, origins, benchmark metadata, peer records,
topology, and discovery in their canonical order, then appends a SHA-256
checksum. It enforces item and label limits and a maximum encoded profile size.

On decode it rejects an oversized or truncated buffer, verifies the checksum and
magic bytes, requires the supported codec schema, decodes every field, rejects
unconsumed payload bytes, and runs the same full profile validation before
returning. `ClusterProfileCodec` maps any such delegated error to
`ClusterError::Codec`; the assembly constructor maps its final encode error to
`InvalidClusterProfile` because it is using the codec only as a result gate.

## Failure catalog

The following are the complete failure families emitted by this module or its
direct dependencies:

| Boundary | Failure variants or condition |
| --- | --- |
| Configuration construction | `InvalidConfiguration` for too few members, duplicate keys/addresses/identities, invalid master, invalid or duplicate pairs, and a disconnected pair graph. |
| Submission indexing | `DuplicateMember` for repeated submitted keys. |
| Member matching | `MissingMember` for a configured member without a submission; `InvalidMember` for failed canonical validation, missing machine, or duplicate machine name; `StaleMember` for an identity mismatch; `DuplicateMember` for repeated actual profile endpoint digests; leftover submissions are `InvalidMember`. |
| Shared metadata | `InvalidConfiguration` for an empty prepared set; `InvalidMember` for differing benchmark metadata. |
| Network set matching | `PairKey::new` rejects a same-member pair as `InvalidConfiguration`; otherwise `DuplicateNetworkMeasurement`, `MissingNetworkMeasurement`, or `InvalidNetworkMeasurement` cover duplicate, missing, and unconfigured evidence. |
| Network binding | `InvalidNetworkMeasurement` for unknown member labels, session endpoint mismatch, stale plan, missing or mismatched remote fingerprint, missing RAM origins/devices, RAM capacity/rate mismatch, absent throughput, or any impossible resolved evidence. |
| Identity hashing | `InvalidClusterProfile` if canonical digest input cannot be represented or hashed. |
| ID allocation and remapping | `IdentityExhausted` on checked counter overflow; `InvalidClusterProfile` for a missing member or object remap. |
| Final assembled profile | `InvalidClusterProfile` wrapping any `MeasuredProfileCodec::encode` validation failure. |
| Cluster codec facade | `Codec` wrapping canonical encode/decode failures; `InvalidClusterProfile` for fewer than two machines, unknown link devices, or a disconnected inter-machine graph. |

All errors stop the current operation. No invalid profile is returned for
inspection or later repair.

## Determinism and ordering guarantees

The function-level comment on `assemble_cluster` promises that input order does
not affect IDs or identity. That follows from several concrete rules:

- `ClusterConfiguration::new` sorts member and pair declarations.
- `index_submissions` uses a `BTreeMap`, then `prepare_members` follows the
  sorted configuration.
- `PairKey::new` normalizes the two labels, and `resolve_network` indexes
  measurements by normalized key.
- `IdAllocator` is consumed in deterministic member/profile order; source
  transport and resource sets are sorted before allocation.
- `cluster_digest` receives members and resolved pairs in those canonical
  orders.
- `peer_benchmarks` is explicitly sorted by session ID before the output
  profile is validated.
- `MeasuredProfileCodec` requires strict increasing order for every persisted
  vector and rejects a noncanonical member profile before its identity is
  derived.

Consequently, reversing a submission vector or reversing a measurement vector
does not change IDs or identity. Opening a valid peer session in the opposite
local direction produces the same normalized topology direction when its
corresponding evidence is equivalent, but the digest remains bound to the
session descriptor and the local/remote ordering inside the hashed peer
evidence. Orientation reversal is therefore not, by itself, an identity
equivalence claim.

## Callers and downstream consumers

The current workspace has no direct invocation of `assemble_cluster` or
`ClusterProfileCodec`; `rg` finds only their definitions and public
re-exports. The intended boundary is nevertheless explicit:

1. A caller probes each machine into a canonical one-machine `MeasuredProfile`
   and derives its `MemberProfileIdentity` for `MemberSpec.expected`.
2. The caller establishes each configured peer session, obtains its descriptor,
   local memory, directional measurement, and bounded plan, then constructs a
   `MeasuredNetworkPair` with `from_probe`.
3. The caller invokes `assemble_cluster` once all exact inputs are available.
4. The returned profile can be persisted with `ClusterProfileCodec`, validated
   with the canonical probe codec, and passed to downstream preparation and
   planning APIs.

Downstream crates consume the resulting `MeasuredProfile` through its
`topology` and `discovery` fields rather than through cluster-specific helper
state. The concrete path is:

```text
MeasuredProfile
  -> recipe_prepare::Preparer::prepare_program
       -> recipe_probe::validate_profile
       -> reservation_plan(topology, discovery)
       -> recipe_planner::plan_program_candidates
       -> recipe_scheduler::schedule(topology, discovery, tasks)
       -> native candidate realization and finalization
  -> training execution helpers (same profile argument)
  -> native preparation / resolve_local_inventory (stable origin matching)
```

`Preparer::prepare_program` validates the profile before using its topology and
discovery, then passes those exact references to reservation planning and
candidate planning. The planner validates both objects again before lowering
programs and enumerating placements. The scheduler validates them before using
measured bandwidth, lane counts, duplex resources, and discovered overlap
capabilities to schedule transfers and calculations. The native-executor
candidate request validates topology, discovery, Draft, reservations, and
artifact identities before realization. Native preparation associates the
current host and GPU inventories with the assembled origin keys, while training
and inference execution paths consume the same canonical objects. The cluster
crate owns assembly and identity binding; those consumers own reservations,
planning, realization, and execution.

## Source map

| Region | Responsibility |
| --- | --- |
| `assemble.rs:23-126` | Prepared/resolved transient state, global ID allocation, and remap tables. |
| `assemble.rs:128-173` | Public deterministic assembly entry point. |
| `assemble.rs:175-254` | Member indexing and common benchmark metadata. |
| `assemble.rs:256-475` | Pair indexing, endpoint/evidence checks, orientation normalization, and RAM gateway resolution. |
| `assemble.rs:477-580` | Output collection and measured profile construction. |
| `assemble.rs:581-724` | Per-member ID allocation, field-by-field remapping, and remap lookup errors. |
| `assemble.rs:726-780` | Inter-machine transport/link and discovery-link construction. |
| `assemble.rs:783-799` | Public cluster codec facade. |
| `assemble.rs:801-866` | Multi-machine connected-shape validation and shared `ensure` helper. |
| `cluster/src/model.rs:13-272` | Cluster schema, identities, membership sorting, pair normalization, and connectivity checks. |
| `cluster/src/model.rs:287-466` | Peer measurement construction and evidence validation. |
| `cluster/src/hash.rs:11-177` | Domain-separated canonical digest implementation used by assembly. |
| `recipe_probe::codec` | Canonical profile validation invoked by member identity derivation, final assembly, and the cluster codec facade. |
| `prepare/src/lib.rs:329-377` | Validates the assembled profile, then passes its topology and discovery to reservation and candidate planning. |
| `planner/src/planner.rs:222-250` | Revalidates topology/discovery before lowering and placement enumeration. |
| `scheduler/src/static_schedule.rs:61-75` | Revalidates topology/discovery before using measured links, duplex resources, and capabilities. |
| `native-executor/src/candidate.rs:43-68` | Validates topology/discovery at the native candidate realization boundary. |
| `probe/src/resolve.rs:76-114` | Reassociates the current host/GPU inventory to assembled origin keys without fallback selectors. |
| `src/native_prepare.rs:260-339` | Loads exact cached profiles and resolves local native bindings from retained origins. |
| `training/src/execute.rs:1203-1228,1321-1347,2178-2209` | Supplies the same measured profile to local inference and training preparation. |
