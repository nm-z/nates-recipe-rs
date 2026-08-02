# `recipe-cluster` crate facade

The package is `recipe-cluster` version `0.1.0` (library target
`recipe_cluster`, Rust edition 2024, MIT license). It has no feature flags,
binary target, build script, or optional dependency.

`cluster/src/lib.rs` is the public boundary for deterministic assembly of a
complete measured cluster. The crate does not discover hardware, open a
transport, run a benchmark, select an endpoint, or infer a device from a
capacity or rate. It accepts already measured per-machine probe profiles and
explicitly session-bound peer evidence, checks that the evidence belongs to
the configured members, then emits one canonical `recipe_probe::MeasuredProfile`
whose topology and discovery graph cover every configured machine.

The crate-level attributes are `forbid(unsafe_code)` and
`deny(missing_debug_implementations)`. The four implementation modules are
private. Callers use the types and functions re-exported at the crate root.
Assembly is synchronous and has no global mutable state, background task,
socket, filesystem access, or logging side effect. Its observable effects are
consuming the two input vectors, allocating the returned graph and digest
buffers, and returning either one complete profile or one `ClusterError`.

## Public surface

The root re-exports the following items:

| Item | Role | Main source |
| --- | --- | --- |
| `CLUSTER_SCHEMA` | `u32` version mixed into every cluster identity digest | `model.rs` |
| `MemberProfileIdentity` | Content-bound identity for one per-machine profile | `model.rs` |
| `MemberSpec` | Configured member key, endpoint label, and expected identity | `model.rs` |
| `NetworkPairSpec` | One canonical, undirected member-pair declaration | `model.rs` |
| `ClusterConfiguration` | Validated master, membership, and connected pair graph | `model.rs` |
| `SubmittedProfile` | A member key paired with its measured profile | `model.rs` |
| `MeasuredNetworkPair` | Probe and transport evidence for both directions of one peer session | `model.rs` |
| `assemble_cluster` | Merge profiles and peer evidence into one measured cluster profile | `assemble.rs` |
| `ClusterProfileCodec` | Cluster-shape facade over the canonical measured-profile codec | `assemble.rs` |
| `ClusterError` and `ClusterResult<T>` | Fail-closed error vocabulary and result alias | `error.rs` |

The implementation types `PreparedMember`, `ResolvedNetworkPair`,
`PairKey`, `IdAllocator`, `MemberRemap`, `ClusterCollections`, and all helper
functions remain crate-private. There is no public `cluster` module to import
from this crate, only these root-level exports. The root `recipe` crate exposes
the complete crate again as `recipe::engine::cluster` for advanced callers;
there are no other in-workspace call sites of the cluster API at present.

### Identity and configuration declarations

`MemberProfileIdentity::derive(&MeasuredProfile)` first sends the profile
through `recipe_probe::MeasuredProfileCodec::encode`. That canonical validation
must succeed before an identity can be derived. It then requires exactly one
topology machine and exactly one machine origin. The endpoint machine digest is
`SHA-256("recipe-cluster-machine-identity-v3" || stable_machine_id)`, and the
endpoint profile digest is the SHA-256 digest of the canonical profile bytes.
Both digests are passed to `recipe_transport::EndpointIdentity::new`, which
rejects zero digests. The returned value also retains the profile's cache,
topology, and discovery identities. The four accessors are:

```text
derive(profile) -> ClusterResult<MemberProfileIdentity>
endpoint(self) -> EndpointIdentity
cache(self) -> CacheIdentity
topology(self) -> TopologyIdentity
discovery(self) -> DiscoveryIdentity
```

`MemberSpec::new(key, address, expected)` is infallible because its arguments
are already typed `recipe_core::Label` and `MemberProfileIdentity`. `key`,
`address`, and `expected` return borrowed labels or a copied identity. The
address is a configuration label only. The cluster crate never resolves it or
uses it to create a socket.

`NetworkPairSpec::new(first, second)` canonicalizes the two labels into lexical
order. It returns `InvalidConfiguration` when both labels are the same. Its
`first()` and `second()` accessors expose the canonical order. Reversing the
arguments therefore describes the same pair and cannot create a second pair.

`ClusterConfiguration::new(master, members, pairs)` owns and normalizes the
configuration. It requires at least two members, sorts members by key, and
rejects duplicate keys, duplicate addresses, repeated endpoint machine
identities, repeated endpoint profile identities, and a master that is not in
membership. It sorts pair declarations by their canonical `PairKey`, rejects
pairs outside membership and duplicate pairs, and requires the pair graph to
reach every member from the lexically first member. The configuration contains
no hardware, topology, capacity, bandwidth, transport, duplex, or lane-count
fields. Its accessors are `master()`, `members()`, and `network_pairs()`.

`SubmittedProfile::new(key, profile)` is a small owned envelope. Construction
does not validate the profile; `assemble_cluster` validates it while checking
the corresponding `MemberSpec`. The profile is consumed by assembly, and the
envelope has no public accessor because submitted data is only meaningful in
the configured merge.

### Peer evidence declaration

`MeasuredNetworkPair::from_probe` is the only public constructor for network
evidence. It receives:

```text
from_probe(
    local_member: Label,
    remote_member: Label,
    session: recipe_transport::SessionIdentity,
    descriptor: recipe_probe::PeerDescriptor,
    local_memory: recipe_transport::MeasuredLocalMemory,
    measurement: recipe_probe::PeerMeasurement,
    benchmark: recipe_probe::BoundedBenchmarkPlan,
) -> ClusterResult<MeasuredNetworkPair>
```

The fields are private so callers cannot bypass the evidence checks. The
constructor requires distinct member labels, a bounded benchmark plan,
asynchronous submission, and an Ethernet or WLAN transport whose descriptor
duplex equals `TransportKind::required_duplex()`. Remote capacity and transfer
rate, plus both directional throughput properties, must have `Measured`
provenance and both throughput directions must be present.

The constructor also binds the retained peer benchmark evidence to the
identity-bound session endpoints and protocol schema. Full duplex requires
simultaneous execution evidence; half duplex requires serialized evidence. For
each direction, the evidence must have exactly `buffer_bytes * iterations`
bytes (with checked multiplication), a nonzero elapsed time no greater than
the plan duration, the exact iteration count, ordered nonzero sample timing
statistics, and a rate equal to the duration-derived bytes-per-second value.
`pair_key()` is crate-private and canonicalizes the two member labels for
assembly indexing.

### Assembly and codec entry points

`assemble_cluster(&ClusterConfiguration, Vec<SubmittedProfile>,
Vec<MeasuredNetworkPair>)` returns `ClusterResult<MeasuredProfile>`. It consumes
the two evidence vectors, never mutates the configuration, and has no partial
success result. `ClusterProfileCodec` is a zero-sized `Clone + Copy + Debug +
Default` facade with:

```text
ClusterProfileCodec::encode(&MeasuredProfile) -> ClusterResult<Vec<u8>>
ClusterProfileCodec::decode(&[u8]) -> ClusterResult<MeasuredProfile>
```

The codec first enforces cluster shape, then delegates all binary encoding,
decoding, checksums, schema checks, canonical ordering, measured provenance,
origin checks, topology/discovery consistency, and peer evidence checks to
`recipe_probe::MeasuredProfileCodec`. It does not introduce another wire
format. The inherited canonical profile reader and writer enforce the probe
codec's bounded item/string/profile sizes and 256 MiB maximum encoded profile
size; `ClusterProfileCodec` does not enlarge those limits.

For API callers, the constructor and accessor surface is intentionally small:

| Type | Fallible constructor | Infallible constructors and accessors |
| --- | --- | --- |
| `MemberProfileIdentity` | `derive(&MeasuredProfile)` | `endpoint`, `cache`, `topology`, `discovery` are `const`, by-value accessors and marked `#[must_use]` |
| `MemberSpec` | none | `const new`, `key`, `address`, and `expected`; accessors are marked `#[must_use]` |
| `NetworkPairSpec` | `new(Label, Label)` | `first` and `second` are canonical `const` accessors marked `#[must_use]` |
| `ClusterConfiguration` | `new(Label, Vec<MemberSpec>, Vec<NetworkPairSpec>)` | `master` is a `const` accessor and `members`/`network_pairs` return borrowed slices; all are marked `#[must_use]` |
| `SubmittedProfile` | none | `const new(Label, MeasuredProfile)`; its consuming `into_parts` helper is private |
| `MeasuredNetworkPair` | `from_probe(...)` | no public field or accessor; `pair_key` is private |
| `ClusterProfileCodec` | `encode` and `decode` | zero-sized `Clone + Copy + Debug + Default` value |

All public model fields are encapsulated. `MemberSpec::new` is intentionally
infallible because it accepts already typed labels and a derived identity;
`SubmittedProfile` is the only public envelope whose embedded measured profile
is not validated at construction. That profile enters the canonical validator
immediately when assembly starts.

The public model envelopes derive `Clone`, `Debug`, `PartialEq`, and `Eq` as
shown by the source. `MemberProfileIdentity` additionally derives `Copy`, and
`ClusterProfileCodec` is a zero-sized `Copy` facade. No model type exposes a
mutable public field or a custom serialization trait.

## Module map

`lib.rs` declares `assemble`, `error`, `hash`, and `model` with `mod`, not
`pub mod`, so implementation details cannot leak through the module tree.

* `model.rs` defines `CLUSTER_SCHEMA`, public declarations and evidence
  envelopes, canonical pair keys, profile identity derivation, peer evidence
  validation, and submission indexing.
* `assemble.rs` owns the complete merge, deterministic ID remapping, topology
  and discovery construction, cross-machine link creation, peer benchmark
  ordering, and cluster-shape validation used by the codec facade.
* `hash.rs` owns the canonical SHA-256 stream used for topology, discovery, and
  profile identities.
* `error.rs` owns the non-exhaustive `ClusterError` enum and the
  `ClusterResult<T> = Result<T, ClusterError>` alias.

## Construction flow

The implementation is intentionally a straight, fail-closed pipeline. Each
step below must complete before the next one runs.

The internal state progression is:

```text
raw labels/evidence
    -> ClusterConfiguration
    -> PreparedMember[] + common BenchmarkMetadata
    -> ResolvedNetworkPair[]
    -> topology/discovery/profile digests
    -> remapped Topology + DiscoveryProfile + origins/peer evidence
    -> canonical MeasuredProfile
```

Only the final state crosses the public assembly return boundary. The
intermediate state types are private, which prevents callers from bypassing a
phase or injecting a partially resolved pair into the remapper.

### 1. Validate and canonicalize configuration

Configuration construction performs the membership and graph checks described
above. Sorting at this boundary establishes the order used everywhere else:
member keys and pair keys are canonical before assembly begins. The graph check
is a repeated reachability expansion over the configured pair list. A pair is
an undirected declaration for connectivity, even though the final topology
will contain two directed links.

### 2. Index and authenticate member profiles

`prepare_members` first indexes submitted envelopes in a `BTreeMap<Label,
MeasuredProfile>`. Duplicate submission keys produce `DuplicateMember`. Every
configured key must then remove one profile, or assembly returns
`MissingMember`; any profile left after configured keys are processed is an
`InvalidMember` for an unconfigured submitter.

For each configured profile, `MemberProfileIdentity::derive` runs canonical
profile validation and derives the machine/profile endpoint identity. The
derived identity must equal the exact `MemberSpec::expected()` value,
otherwise the profile is `StaleMember`. Endpoint profile digests must be unique
across members, and machine names taken from the sole profile machine must be
unique. A duplicate profile digest or duplicate machine name is rejected before
any IDs are allocated. `common_benchmarks` then requires byte-for-byte equality
of `BenchmarkMetadata` across all prepared members. Benchmark metadata is a
cluster-wide invariant, not a per-member choice.

### 3. Index and resolve peer measurements

`resolve_network` indexes each `MeasuredNetworkPair` by its canonical pair key.
Duplicate evidence for the same unordered pair produces
`DuplicateNetworkMeasurement`. Every configured pair must remove exactly one
measurement, or `MissingNetworkMeasurement` is returned. Any remaining
measurement is evidence for an unconfigured pair and returns
`InvalidNetworkMeasurement`.

`resolve_pair` performs the checks that require both member profiles:

1. `local_member` and `remote_member` must be configured.
2. The session's local and remote `EndpointIdentity` values must equal the
   identities derived for those two members.
3. The pair benchmark plan must equal the common `benchmarks.network` plan.
4. The descriptor's remote machine fingerprint must equal the remote profile's
   retained machine origin.
5. The descriptor's local and remote RAM keys must resolve to RAM origins in
   the corresponding profiles, and each capacity/rate pair must exactly equal
   the measured gateway device property. Capacity and rate are consistency
   checks, never identity selectors.
6. Both throughput properties must be present. The descriptor duplex is mapped
   to the core `DuplexMode`.

The configured `PairKey` is lexical. If evidence was collected with the
lexically first member local, outbound and inbound measurements map directly
to first-to-second and second-to-first. If the session was collected in the
reverse direction, the two rates, lane counts, and gateway devices are swapped
so the resolved pair still has a canonical first-to-second orientation. The
session ID, descriptor, benchmark, and peer evidence are retained for
identity hashing and the final `MeasuredPeerBenchmark` records.

### 4. Bind content identities

After member and network checks, `assemble_cluster` computes three independent
digests with the same canonical input stream and different domain strings:

```text
recipe-cluster-topology-v3
recipe-cluster-discovery-v3
recipe-cluster-profile-v3
```

`CanonicalDigest` starts with a length-prefixed domain and the little-endian
`CLUSTER_SCHEMA`. It then hashes the master key, all common benchmark plan
fields, each sorted member's key/address and expected identity components, the
actual member endpoint profile digest, and every sorted network pair. Network
input includes transport and duplex tags, directional rates and provenance,
lane counts, asynchronous submission, both gateway memory measurements, all
retained descriptor identity strings, the benchmark plan, measured rates, and
the complete peer evidence: protocol schema, endpoint digests, execution mode,
and every directional timing statistic including `u128` variance.

The stream is length-prefixed for strings and byte slices, uses little-endian
integer encodings, and tags transport, duplex, and property-provenance enums.
Only an oversized identity input can fail hashing, reported as
`InvalidClusterProfile`. The cluster stream does not add remapped IDs or source
vector positions as separate fields; source IDs and canonical ordering remain
covered by each member's endpoint profile digest. The configuration and
canonical sorting therefore make identities independent of submission or
evidence-vector order while still changing whenever a measured fact, identity,
endpoint label, benchmark plan, or configured graph changes.

### 5. Remap each member into one global profile

`remap_profile` allocates fresh IDs from six independent counters, all starting
at one: machines, nodes, devices, links, transports, and duplex capacity
resources. Members are processed in sorted configuration order. For each
member, `allocate_member` maps every original machine, node, device, link,
transport, and resource ID. Mapping failures are internal profile errors, not
fallbacks.

`remap_member` then copies each per-machine object and translates every
reference:

* machines retain their names; machine origins retain fingerprints;
* nodes retain their device lists, but the role is normalized so the configured
  master member's original master node is the sole master and every other node
  is a worker;
* devices retain kind, capacity, transfer rate, and optional calculation rate;
* RAM, storage, and GPU origin keys are preserved while their device IDs are
  translated;
* directed internal links retain transport kind, duplex, directional measured
  properties, and resource relationships;
* discovered-device capabilities and discovered-link capabilities are copied
  with translated IDs.

The member profiles are expected to be canonical per-machine profiles. Their
internal IDs can differ between machines; this remap is what makes one global
profile possible without trusting those local ID namespaces.

### 6. Add inter-machine links and discovery

For each resolved network pair, `push_network_pair` creates one new transport
identity, two new link identities, and either one shared capacity resource for
half duplex or two independent resources for full duplex. The links connect
the resolved RAM gateway devices in both directions. Directional bandwidth is
the resolved measured property. Lane counts are wrapped as measured
`Property<TransferLaneCount>` values, and both discovered-link records are
marked available with the descriptor's asynchronous-submission capability.

The final `Topology` contains the newly computed topology identity and all
member plus inter-machine machines, nodes, devices, and links. The final
`DiscoveryProfile` contains the newly computed discovery identity, the same
topology identity, all remapped member capability records, and the two
discovered records for every network link.

### 7. Assemble and canonically validate the result

The output profile has `recipe_probe::PROFILE_SCHEMA`, a `CacheIdentity` whose
schema is the probe profile schema and whose digest is the cluster profile
digest, all remapped measured origins, the common benchmark metadata, the
network peer benchmark records sorted by session ID, and the completed topology
and discovery profiles. `assemble_cluster` finally calls
`MeasuredProfileCodec::encode`; this validates the whole profile and returns
`InvalidClusterProfile` on any codec failure. A successful return is therefore
both an assembled graph and a canonical measured-profile value.

That final pass is not redundant serialization. It proves that remapping kept
every core reference valid, that copied topology and discovery properties still
match, that fresh full/half-duplex resources have the required relationship,
that every retained origin still names the correct object, and that the new
cross-machine links have exactly the peer evidence required by the canonical
probe contract. The assembler has no alternate output path when this proof
fails.

## Cluster shape and persisted profiles

`validate_cluster_shape` is the extra invariant used by
`ClusterProfileCodec`. It requires at least two topology machines, maps every
device ID to its machine, rejects links that reference unknown devices, and
builds an undirected machine adjacency graph from links that cross machine
boundaries. Reachability from the first machine must include every machine.
Same-machine links do not contribute to this graph. The canonical probe codec
still owns the stricter object, duplex, provenance, origin, ordering, and
peer-topology validations.

On `encode`, shape validation runs before the canonical probe codec. On
`decode`, the bytes are first decoded and fully validated by
`MeasuredProfileCodec`, then the resulting value is checked for at least two
machines and a connected inter-machine graph. Shape failures and remap failures
are `InvalidClusterProfile`; binary codec failures are wrapped as `Codec`.

The output has the core topology contract for bidirectional links: opposing
edges share one transport identity; half-duplex edges share one capacity
resource, and full-duplex edges use distinct resources. The codec additionally
requires exactly two opposing edges per cross-machine transport and exactly one
matching peer benchmark for each cross-machine transport. This is why peer
evidence cannot be replaced by a nominal network rate or a topology-only
declaration.

## Failure ownership

`ClusterError` is `#[non_exhaustive]` and its `Display` implementation keeps the
boundary that detected the failure visible:

| Variant | Detected by | Meaning |
| --- | --- | --- |
| `InvalidConfiguration` | `NetworkPairSpec` or `ClusterConfiguration` | Pair shape, membership, identity uniqueness, master, or graph is invalid |
| `MissingMember` | `prepare_members` | A configured key submitted no profile |
| `DuplicateMember` | submission indexing, configuration, or profile identity checks | A member key or profile identity appears more than once |
| `StaleMember` | `prepare_members` | Derived profile identity differs from the configured expectation |
| `InvalidMember` | `prepare_members` or common benchmark checks | Profile is malformed for a member, extra, has duplicate machine name, or has inconsistent benchmark metadata |
| `MissingNetworkMeasurement` | `resolve_network` | A configured pair has no evidence |
| `DuplicateNetworkMeasurement` | `resolve_network` | More than one evidence value canonicalizes to a pair |
| `InvalidNetworkMeasurement` | `MeasuredNetworkPair` validation or `resolve_pair` | Session, descriptor, benchmark, provenance, gateway, orientation, or evidence data is inconsistent |
| `IdentityExhausted` | `IdAllocator` | A fresh object ID counter cannot increment; the current labels are `machine`, `node`, `device`, `link`, `transport`, and `duplex resource` |
| `InvalidClusterProfile` | identity derivation, hashing, remap, shape, or final assembly validation | A profile cannot satisfy the cluster or measured-profile contract |
| `Codec` | `ClusterProfileCodec` | Canonical binary encode/decode failed |

The current `Display` implementation prefixes messages with enough context to
identify the owning boundary in a human-readable log: `invalid cluster
configuration`, `member ...`, `network pair ...`, `invalid inter-machine
measurement`, `... identity space is exhausted`, `invalid cluster profile`, and
`cluster profile codec`. The enum is non-exhaustive, so callers should match
known variants plus a wildcard rather than treating this text as a machine
protocol.

`recipe_core::Label::new`, `recipe_probe::ProbeError`, and
`recipe_transport::TransportError` remain owned by their respective crates
before callers construct the cluster envelopes. The cluster crate receives no
socket or discovery error and cannot retry a failed benchmark. Every fallible
operation uses `?`; the first failing cluster phase is returned and no partial
profile escapes. The delegated core and probe validators may aggregate several
object-level validation findings into their one wrapped codec message, but
they do not cause assembly to continue to a later phase.

## Dependency and consumer boundaries

`cluster/Cargo.toml` has four direct dependencies: path packages
`recipe-core`, `recipe-probe`, and `recipe-transport`, plus `sha2` `^0.10.9`.

* `recipe-core` supplies `Label`, digests and identity wrappers, typed stable
  IDs, topology and discovery graph objects, transport/duplex enums, measured
  properties, and unit types.
* `recipe-probe` supplies `MeasuredProfile`, the profile schema and canonical
  codec, benchmark metadata and plans, machine and device origin records, peer
  descriptors and evidence, and the `PeerMeasurement` model.
* `recipe-transport` supplies nonzero `EndpointIdentity` and identity-validating
  `SessionIdentity`, plus `MeasuredLocalMemory` for peer gateway checks. The
  cluster crate does not depend on TCP implementation details.
* `sha2` supplies SHA-256 for member endpoint profile digests, stable machine
  digests, and cluster identity streams.

The transport boundary is explicit. `recipe_transport::TcpPeerSession` is built
by a caller with an already connected stream, exact endpoint identities, a
descriptor, and local memory. Its `PeerSession` methods produce a descriptor
and bounded `PeerMeasurement`; the caller passes those values, the session
identity, member labels, and benchmark plan to `MeasuredNetworkPair::from_probe`.
The cluster crate neither chooses the peer nor establishes security.

The probe boundary is likewise explicit. `recipe_probe::ProbeEngine` discovers
and measures one host, constructs a canonical `MeasuredProfile`, retains
machine fingerprints and RAM/storage/GPU origin keys, and can use peer sessions
to collect network evidence. Cluster assembly does not invoke `ProbeEngine`.
The member identity check requires the submitted profile to contain exactly one
machine and one machine origin, so a member submission is a per-machine
snapshot; inter-machine links enter through `MeasuredNetworkPair`.
The resulting multi-machine profile is therefore not itself a valid
`MemberProfileIdentity::derive` input. Endpoint identities are for the
per-machine admission boundary; the assembled profile's topology, discovery,
and cache identities belong to the cluster boundary.

The source-level handoff is consequently:

```text
one host: ProbeEngine::probe (without peer sessions)
        -> SubmittedProfile::new(member_key, profile)
        -> MemberProfileIdentity::derive(profile)
        -> MemberSpec::new(member_key, address, expected_identity)

one identity-bound peer session:
        descriptor + bounded PeerMeasurement + SessionIdentity
        -> MeasuredNetworkPair::from_probe(...)

all members and configured pairs:
        ClusterConfiguration + SubmittedProfile values + MeasuredNetworkPair values
        -> assemble_cluster
        -> canonical multi-machine MeasuredProfile
```

Calling `ProbeEngine::probe` with peer sessions can produce a valid probe
profile, but that profile intentionally contains more than one machine and is
not a legal member submission. The separate handoff prevents an endpoint from
claiming a topology assembled from a different machine's evidence.

The only direct workspace package consumer of `recipe-cluster` is the root
`recipe` package, which lists it as a dependency and re-exports it under
`recipe::engine::cluster`. There are no current root, planner, scheduler,
remote, or training call sites that automatically construct a cluster. A
caller must orchestrate profile collection, expected identity configuration,
peer session measurement, and the merge explicitly.

The resulting `MeasuredProfile` is the same typed input accepted by the next
pipeline stages. In particular, `recipe_prepare::Preparer::prepare` and
`prepare_program` validate the profile with `recipe_probe::validate_profile`
before using its topology and discovery for reservation planning, artifact
resolution, scheduling, realization, and finalization. The root native
preparation path also loads canonical measured profiles and resolves current
RAM, storage, and GPU inventories by retained origins. These consumers own
preparation and hardware failures; cluster assembly owns only the identity,
evidence, merge, and cluster-shape boundary described here.

The root training and inference paths pass the same profile through
`Preparer::prepare_program`, derive local tuning from measured devices and
same-machine links, and compare retained topology/discovery identities when a
native kernel or bundle is resumed. A cluster profile can therefore carry the
complete machine graph while each local preparation scope selects the current
machine by its retained origins. None of these execution paths calls a private
cluster helper or reconstructs a network fact from the profile.

## Non-responsibilities

The facade intentionally does not:

* read TOML, cache files, or profile files;
* discover machines, devices, drivers, firmware, RAM domains, or network
  interfaces;
* resolve `MemberSpec::address()` or create/listen on a socket;
* run, cancel, or retry peer benchmarks;
* estimate missing rates, capacities, lane counts, duplex mode, or transport
  kind;
* select a route, schedule a calculation, allocate a runtime arena, compile a
  kernel, or launch a worker;
* accept a stale profile, failed peer attempt, nominal network rate, or
  duplicate identity as a fallback; or
* mutate an already assembled `MeasuredProfile`.

Those operations belong to probe, transport, preparation, planning, scheduling,
and execution crates. `recipe-cluster` is the narrow typed join between
per-machine measured facts and the complete machine graph.

The fail-closed choices are concrete rather than policy prose:

| Missing or conflicting fact | Cluster response |
| --- | --- |
| Profile key absent | `MissingMember`; no empty machine is synthesized |
| Extra or duplicate profile key | `InvalidMember` or `DuplicateMember`; no submission is ignored |
| Expected profile identity differs | `StaleMember`; expected config is not rewritten |
| Pair evidence absent | `MissingNetworkMeasurement`; no seed or nominal network rate is used |
| Pair evidence duplicated or unconfigured | `DuplicateNetworkMeasurement` or `InvalidNetworkMeasurement`; no merge of directions occurs |
| Session endpoint or remote fingerprint mismatch | `InvalidNetworkMeasurement`; no label or address is trusted as a substitute |
| RAM origin key absent or properties differ | `InvalidNetworkMeasurement`; no capacity/rate heuristic chooses another device |
| Wrong transport, duplex, execution mode, or async capability | `InvalidNetworkMeasurement`; no hidden serialization or alternate transport is selected |
| ID allocator exhausted | `IdentityExhausted`; no ID wrapping occurs |
| Cluster hash input too large | `InvalidClusterProfile`; no digest truncation occurs |
| Canonical output validation fails | `InvalidClusterProfile`; no partial bytes or profile escape |

## Trust transitions

The cluster boundary has four explicit trust transitions:

1. **Policy to manifest.** `ClusterConfiguration::new` accepts only labels,
   expected content identities, and a connected pair graph. It cannot carry a
   user-supplied rate, capacity, lane count, or transport override.
2. **Probe output to member.** Canonical profile validation, the one-machine
   shape check, and exact `MemberProfileIdentity` equality turn one submitted
   profile into an admitted member. A submitted profile never enrolls itself
   by changing the expected identity.
3. **Session evidence to link.** `MeasuredNetworkPair::from_probe` binds peer
   timing evidence to a session; `resolve_pair` binds the session, fingerprint,
   benchmark plan, and RAM-domain keys to two admitted members. No nominal rate
   or value-based gateway inference crosses this boundary.
4. **Resolved facts to global graph.** Domain-separated digests and fresh
   global IDs bind the accepted policy/evidence manifest to one topology and
   discovery graph. The canonical measured-profile codec is the final check
   before the graph is exposed to preparation.

Every transition is one-way in this crate. A later phase cannot alter policy,
replace a measurement, or repair an earlier identity mismatch.

## Function-level validation matrix

The following order is observable from the implementation and is useful when
assigning a failure to its owner. The cluster crate does not reorder these
checks to guess at a likely repair.

| Entry point or helper | Checks performed before it returns |
| --- | --- |
| `MemberProfileIdentity::derive` | Canonical probe-profile encoding, one machine, one machine origin, nonzero endpoint machine/profile digests, and retention of the source cache/topology/discovery identities |
| `NetworkPairSpec::new` | Distinct labels and lexical pair normalization |
| `ClusterConfiguration::new` | Minimum membership, canonical member and pair order, key/address/machine/profile uniqueness, master membership, pair membership, pair uniqueness, and graph connectivity |
| `MeasuredNetworkPair::from_probe` | Distinct endpoints, bounded plan, asynchronous transport, Ethernet/WLAN kind, required duplex, measured remote memory and throughput, identity-bound endpoint evidence, duplex execution mode, and exact duration-derived directional evidence |
| `prepare_members` | One submission per configured key, no missing or extra keys, expected identity equality, unique submitted profile digests, unique profile machine names, and profile shape errors reported as member errors |
| `common_benchmarks` | Exact equality of all four benchmark plans and the seed schema metadata |
| `resolve_network` | One measurement per canonical configured pair, complete configured coverage, and no extra evidence |
| `resolve_pair` | Member lookup, session-to-member endpoint equality, network plan equality, remote fingerprint equality, exact RAM-origin gateway resolution, exact memory property agreement, both directional rates, and canonical orientation |
| `remap_profile` | Complete ID maps for every referenced member object, translated references, normalized node roles, paired network links, and a final measured profile with cluster identities |
| `ClusterProfileCodec::encode` | Cluster machine count and cross-machine connectivity, then every canonical probe codec rule |
| `ClusterProfileCodec::decode` | Binary checksum/schema/structure and every canonical probe codec rule, then cluster machine count and cross-machine connectivity |

The helper `ensure` has no recovery semantics. It returns `Ok(())` only for a
true condition and returns the supplied error unchanged for false. The same
shape is used by model validation, peer evidence validation, assembly, and
cluster-shape checking.

## Output data lineage

The assembled profile deliberately keeps a distinction between semantic
topology, discovery capabilities, identity provenance, and benchmark evidence:

| Output region | Source and transformation |
| --- | --- |
| `topology.machines` | Every member machine cloned in sorted member order, with fresh global `MachineId` values and original machine names |
| `topology.nodes` | Every member node cloned with fresh IDs and references; role rewritten from the configured master key so only the selected master member can own `NodeRole::Master` |
| `topology.devices` | Every member device cloned with fresh IDs, preserving kind and measured capacity/transfer/calculation properties; network links reuse the RAM devices already declared by the corresponding member profiles |
| `topology.links` | All remapped member-internal links followed by two fresh directed links for each resolved network pair |
| `origins` | Machine fingerprints and RAM/storage/GPU discovery keys copied with translated IDs; keys are never reconstructed from rates or ordinals |
| `discovery.devices` and `discovery.links` | Member capability snapshots copied with translated IDs plus available asynchronous records for each new network direction |
| `peer_benchmarks` | One record per network evidence item, retaining session ID, measured rates, endpoint digests, execution mode, and directional timing statistics, sorted by session ID |
| `topology.identity` | Cluster SHA-256 digest under the topology domain |
| `discovery.identity` | Cluster SHA-256 digest under the discovery domain, with `discovery.topology` set to the same topology identity |
| `cache_identity` | Probe `PROFILE_SCHEMA` paired with the cluster SHA-256 digest under the profile domain |
| `benchmarks` | The single exact `BenchmarkMetadata` value shared by all members |

The phrase "remote RAM gateway" in the table means the device selected by the
retained RAM-domain key in `resolve_gateway`. The assembler does not synthesize
a RAM device from a peer's capacity. A remote gateway must already be a device
in the corresponding member profile and its measured capacity and transfer
rate must agree with the peer evidence.

For the current one-machine member contract, the object counts have direct
lineage. The output machine count is the member count, the device count is the
sum of member device counts, and the node count is the sum of member node
counts. Internal link and discovery-link counts are summed from members, then
two directed topology links and two discovered-link records are added per
configured network pair. No device or node is synthesized for a network pair.
Internal transport/resource identities are remapped one-for-one; each network
pair adds one transport and one resource for half duplex or two resources for
full duplex. The peer benchmark count is exactly the network-pair count.

## Determinism and orientation details

There are three independent ordering boundaries:

1. `ClusterConfiguration::new` sorts member and pair declarations.
2. `index_submissions` and `resolve_network` use ordered maps, so vector order
   from the caller cannot affect lookup.
3. `remap_profile` allocates IDs in sorted member and pair order, while the
   final peer benchmark vector is sorted by session ID.

The canonical measured-profile codec also requires strictly increasing IDs in
all profile vectors. Consequently, a profile with arbitrary source vector
order is not silently normalized by assembly: it fails member identity
derivation at the canonical codec boundary. A successful merge is independent
of the order in which otherwise valid submissions and pair evidence vectors
were supplied.

Pair labels are canonicalized before lookup, but session direction is not
discarded. Suppose the configured key is `alpha<->beta` and a caller measured
the session with `beta` local. `resolve_pair` maps the session's outbound rate
to the `beta`-to-`alpha` directed link, maps inbound to `alpha`-to-`beta`, and
swaps gateway IDs and lane counts for the canonical `first=alpha,
second=beta` representation. The original endpoint digests and directional
timing evidence remain in the peer benchmark record. This preserves both a
stable topology orientation and the evidence's identity-bound observation.

Full and half duplex differ only in the capacity resource relationship, not in
the existence of the two directed links. Full duplex allocates a resource for
each direction and requires simultaneous peer evidence. Half duplex reuses one
resource and requires serialized evidence. `TransportKind::required_duplex`
and the canonical topology validator enforce the same relationship at both the
input and output boundaries.

## Schema and identity version boundaries

`CLUSTER_SCHEMA` is currently `4`. It versions the cluster hash stream only.
The output profile and its binary codec use `recipe_probe::PROFILE_SCHEMA`,
currently `7`; the peer evidence protocol uses
`PEER_BENCHMARK_PROTOCOL_SCHEMA`, currently `1`. A change in any of these
versioned domains is expected to invalidate incompatible identities or bytes.

The member endpoint machine identity intentionally hashes only the retained
stable machine ID under its own `v3` domain. Hostname, runtime ABI, firmware,
and all measured topology data are covered by the canonical profile digest that
forms the endpoint profile identity. Cluster topology, discovery, and cache
identities are separate domain-separated digests, so a consumer can compare the
identity domain it owns without confusing it with the other two.

`ClusterProfileCodec` does not change these schema values and does not add a
cluster-specific file extension. If a caller persists the returned bytes, the
path and storage policy remain the caller's responsibility. A missing or stale
cache is not an assembly fallback because this crate has no cache reader.

The codec facade is a structural and canonical-profile validator, not a
recomputation service for arbitrary manually assembled identities. It checks
that identities are present and nonzero through the canonical probe validators,
but it has no configuration, member expectations, or peer-session manifest
with which to recompute a cluster digest. Authenticating a profile against a
specific cluster therefore requires the original `ClusterConfiguration`,
member identities, and peer evidence to be supplied to `assemble_cluster`.

## Error precedence and edge behavior

The implementation's ordered checks also define which failure is reported when
an input violates more than one rule:

* `ClusterConfiguration::new` checks the minimum member count before any
  duplicate or master checks, then validates members before pair declarations,
  and checks connectivity last. A disconnected graph is therefore only
  reported after all pair references are valid and unique.
* `NetworkPairSpec::new` rejects a same-label pair immediately. A reversed pair
  is normalized, so a later duplicate check in `ClusterConfiguration::new`
  reports the canonical `first<->second` pair.
* `prepare_members` indexes all submissions before validating profiles. A
  duplicate submission key is reported before any profile identity is derived;
  after indexing, configured members are processed in sorted key order, so the
  first missing or invalid configured member determines the result. Extra
  submissions are checked only after all configured members succeed.
* Profile identity derivation runs canonical probe validation before its
  explicit one-machine and one-origin checks. A malformed profile therefore
  reports the canonical validation failure; a valid multi-machine profile
  reports the per-machine shape error.
* `resolve_network` indexes and duplicate-checks every evidence item before it
  looks for missing configured pairs. It then resolves configured pairs in
  canonical order and reports an extra unconfigured pair only after all required
  pairs have resolved.
* `resolve_pair` checks member labels, identity-bound endpoint identities,
  benchmark-plan equality, remote fingerprint, gateway keys and properties, and
  finally directional throughput presence in that order. The first violated
  relationship is returned without trying a different member, key, or rate.
* Identity hashing and remapping happen only after all member and network
  evidence checks. A digest-size or allocator failure cannot be confused with
  stale evidence. The final canonical profile encode is the last validation
  before success.

Peer session IDs are not the lookup key for network pairs, so two different
configured pairs can carry the same descriptor session label through the
resolution phase. The output peer benchmark sort then has a non-strict key and
the final canonical codec rejects it as `InvalidClusterProfile`; the assembler
does not rename or merge session IDs.

This ordering is intentionally not a recovery strategy. It gives callers one
specific failing boundary and leaves the underlying evidence unchanged for the
caller to correct. No duplicate, reverse-direction, missing-profile, or missing
gateway case is silently dropped.

No random, time-derived, process-derived, or global counter participates in
assembly. Repeating a call with equal configuration, canonical member profiles,
and equal evidence produces equal fresh IDs, profile objects, encoded bytes,
and three identity digests. Equality of the evidence vectors' input order is
not required because both vectors are indexed before remapping.

For a three-member chain such as `m1<->m2` and `m2<->m3`, configuration order
is `m1`, `m2`, `m3` even if callers submit vectors in another order. If the
`m2` session was measured with `m2` local for `m1<->m2`, the resolver swaps its
outbound/inbound values into the canonical `m1`-to-`m2` direction. The selected
master key, not source node roles or vector order, decides which remapped node
is the sole master. Changing the master or any measured evidence changes the
domain-separated cluster identities; merely changing submission order does not.
