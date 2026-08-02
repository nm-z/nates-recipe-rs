 # Measured profile codec

 `probe/src/codec.rs` is the canonical, versioned binary codec for a fully
 measured `MeasuredProfile`. It is deliberately more than a serializer: both
 encode and decode are admission boundaries. Encoding validates the in-memory
 profile before producing bytes; decoding authenticates and parses bytes and
 then runs exactly the same profile validation. A byte sequence therefore
 represents a profile only if it is canonical, bounded, internally consistent,
 and composed entirely of measured production properties.

 The implementation is the authority for this document. In the current source
 `PROFILE_SCHEMA` and `PROFILE_CODEC_SCHEMA` are both `7`, and
 `PEER_BENCHMARK_PROTOCOL_SCHEMA` is `1` (`probe/src/model.rs:17-20`). The
 checked-in `probe/PROFILE_SCHEMA.md` still describes version 6; that prose is
 not the executable version contract. `CONTRACT_SCHEMA` for the benchmark seed
 is `1` (`probe/src/seed.rs:10`).

 ## Public surface and purpose

 `MeasuredProfileCodec` is a zero-sized, `Clone + Copy + Debug + Default` type
 with two public operations:

 * `encode(&MeasuredProfile) -> ProbeResult<Vec<u8>>`
 * `decode(&[u8]) -> ProbeResult<MeasuredProfile>`

 `validate_profile(&MeasuredProfile) -> ProbeResult<()>` is public from the
 module facade and is the shared admission operation used by the codec and by
 downstream preparation and resolution code. The codec's private helpers are
 paired by construction: every `encode_*` writes the fields consumed in the
 same order by its corresponding `decode_*`.

 The codec owns no persistent state. `Encoder` owns only the output `Vec<u8>`;
 `Decoder` owns a borrowed payload slice and a byte position. Identities in the
 profile remain content values. The codec does not allocate IDs, discover
 hardware, perform measurements, infer an origin from a capacity or rate, or
 choose a cache path.

 ## End-to-end role

 The profile lifecycle is:

 1. `ProbeEngine::inspect` performs host, GPU, and peer discovery, normalizes
    stable keys, and computes the current `CacheIdentity` from discovery and
    seed inputs (`probe/src/engine.rs:163-201`, `575-668`). It does not read a
    profile file.
 2. `ProbeEngine::probe` executes bounded RAM, storage, GPU, and peer
    measurements, builds measured topology, origins, and discovery capability,
    validates the core structures, constructs `MeasuredProfile`, then calls
    `validate_profile` before returning it (`probe/src/engine.rs:63-160`).
 3. `ProbeEngine::load_or_probe_and_store` computes that identity, asks the
    supplied `ProfileCache` to load it, and returns an exact decoded profile on
    a hit. A miss runs `probe_and_store`, whose store path calls codec encode
    before installation (`probe/src/engine.rs:212-237`). A malformed or stale
    cache is an error from the cache, not a silent fallback to another file.
 4. `ExplicitPathProfileCache` reads a bounded private regular file, calls
    `MeasuredProfileCodec::decode`, and then compares the decoded
    `profile.cache_identity` with the requested identity (`probe/src/cache.rs:83-138`).
    Store calls encode, treats an identical existing profile as a no-op, and
    refuses to replace a different one (`probe/src/cache.rs:142-195`). The
    atomic same-directory temporary-file and `NOREPLACE` installation are cache
    responsibilities; the codec supplies the authenticated bytes.
 5. The CLI's zero-argument `recipe probe` names the cache
    `measured-v<schema>-<lowercase-digest>.recipe-profile`, where the schema and
    digest come from `current_cache_identity` (`src/cli.rs:876-947`). Native
    preparation parses that filename into an expected identity, loads through
    `ExplicitPathProfileCache`, and fails if the file is absent, stale,
    malformed, or invalid (`src/native_prepare.rs:248-287`, `413-447`).
 6. `MeasuredProfile::resolve_local_inventory` validates the profile before
    associating current exhaustive host and GPU inventories with its retained
    origins. It uses exact stable keys and machine fingerprints only; capacity,
    product name, ordinal, and benchmark similarity are not selectors
    (`probe/src/resolve.rs:76-114`). Preparation and native realization also
    call `validate_profile` before consuming topology and discovery
    (`prepare/src/lib.rs:327-338`, `prepare/src/production.rs:949-972`).
 7. Cluster assembly uses this codec as the member-profile identity source:
    `MemberProfileIdentity::derive` hashes canonical encoded bytes, and
    `ClusterProfileCodec` adds cluster-shape checks around the same codec
    (`cluster/src/model.rs:27-63`, `cluster/src/assemble.rs:783-799`). Cluster
    assembly encodes its remapped result before returning it
    (`cluster/src/assemble.rs:128-172`).

 Consequently, the encoded profile is the hand-off between measured discovery,
 the private cache, cluster identity, exact local realization, and immutable
 planning. It is not a user-editable configuration or a best-effort snapshot.

 ## Wire envelope

 The byte stream is one fixed-order payload followed by a 32-byte checksum.
 There are no field tags, maps, alignment bytes, compression, or optional
 section tags. Version changes are intentionally incompatible.

 | Offset/order | Field | Encoding |
 | --- | --- | --- |
 | 1 | magic | 16 raw bytes, `RECIPEPROFILE\0\0\0` |
 | 2 | codec schema | little-endian `u32`, currently `7` |
 | 3 | profile schema | little-endian `u32`, must equal `PROFILE_SCHEMA` (`7`) |
 | 4 | cache schema | little-endian `u32`, must equal `PROFILE_SCHEMA` (`7`) |
 | 5 | cache digest | 32 raw digest bytes |
 | 6 | origins | `encode_origins` section, below |
 | 7 | benchmark metadata | `encode_benchmark_metadata` section |
 | 8 | peer benchmarks | `encode_peer_benchmarks` section |
 | 9 | topology | `encode_topology` section |
 | 10 | discovery | `encode_discovery` section |
 | 11 | checksum | SHA-256 of every byte from magic through the last discovery field |

 `MAGIC` is 16 bytes. `CHECKSUM_BYTES` is 32. The maximum complete encoded
 profile, including checksum, is `MAXIMUM_PROFILE_BYTES = 256 * 1024 * 1024`
 bytes. Each collection length is capped at `MAXIMUM_ITEMS = 1_000_000`; each
 label is capped at `MAXIMUM_STRING_BYTES = 1_048_576` UTF-8 bytes.

 ### Encoding and decoding envelope behavior

 `encode` calls `validate_profile` first, then `encode_unchecked`. The latter
 writes the fixed sequence, computes `Sha256::digest(&encoder.bytes)`, appends
 the 32-byte digest, checks the total size limit, and returns the vector
 (`probe/src/codec.rs:98-119`). The name `encode_unchecked` means only that the
 caller has already performed profile validation; its collection, label, and
 duration encoders still enforce wire bounds.

 `decode` first rejects input larger than the maximum. It then requires at least
 `MAGIC.len() + 4 + CHECKSUM_BYTES` bytes, treats the final 32 bytes as the
 encoded checksum, recomputes SHA-256 over the preceding payload, and rejects a
 mismatch. It parses the payload in fixed order, rejects a codec schema other
 than the current constant, rejects unconsumed payload bytes, builds the
 `MeasuredProfile`, and calls `validate_profile` before returning it
 (`probe/src/codec.rs:41-95`). The checksum is an integrity check, not a
 substitute for semantic validation: a valid checksum over invalid fields is
 still rejected.

 ## Primitive representation

 `Encoder` and `Decoder` are intentionally minimal and symmetric
 (`probe/src/codec.rs:1281-1500`):

 * `raw` copies bytes without framing.
 * `u8`, `u16`, `u32`, `u64`, and `u128` use their fixed width little-endian
   representation, except `u8`, which is one byte.
 * `bool` is encoded as `0` or `1`; decode rejects every other byte.
 * A `Digest` is exactly its 32 raw bytes, with no length prefix.
 * A collection `length` is a little-endian `u32`, after checking the one
   million item limit and conversion from `usize`.
 * A `Label` is a little-endian `u32` byte length followed by UTF-8 bytes. The
   encoder checks the one MiB canonical byte limit. The decoder checks that
   limit, validates UTF-8, and runs `Label::new`, so an empty or whitespace-only
   label is rejected by the `Label` constructor.
 * `Property<ByteCount>` is `u64 value` then one provenance byte.
 * `Property<BytesPerSecond>` is `u64 value` then provenance.
 * `Property<TransferLaneCount>` is `u32 value` then provenance.
 * `Property<FlopsPerSecond>` is `u64 value` then provenance.
 * Provenance tags are `Estimated = 0`, `Measured = 1`, and `Override = 2`.
   Unknown tags fail closed. `Property::new` itself stores the tag and value;
   the nonzero `BytesPerSecond`, `TransferLaneCount`, and `FlopsPerSecond`
   constructors reject zero values while decoding. `ByteCount` permits zero,
   with higher-level validation deciding whether a capacity is usable.

 `Decoder::raw` uses checked position arithmetic and a bounded slice. Every
 typed read therefore reports truncation rather than panicking. `length` and
 label length are checked before allocation or slicing. Numeric conversion
 errors, invalid UTF-8, invalid `Label`, invalid unit values, invalid
 provenance, and invalid booleans become a `ProbeError` from `codec_error`.

 ## Complete field order

 The following order is the file format. Changing a field's position, width,
 tag assignment, or presence marker requires a new codec schema.

 ### Origins

 `encode_origins` writes four independent vectors in this order:

 1. `origins.machines` count. Each item is `machine: u64`, then four labels in
    order: `fingerprint.hostname`, `fingerprint.stable_id`,
    `fingerprint.runtime_abi`, and `fingerprint.firmware`.
 2. `origins.ram` count. Each item is `device: u64`, then `key: Label`.
 3. `origins.storage` count. Each item is `device: u64`, then `key: Label`.
 4. `origins.gpu` count. Each item is `device: u64`, then `key: Label`.

 Decode reconstructs `MeasuredMachineOrigin`, `MeasuredRamOrigin`,
 `MeasuredStorageOrigin`, and `MeasuredGpuOrigin` with the same nesting and
 order (`probe/src/codec.rs:798-868`). Origins retain host-discovery identity
 that is intentionally absent from core topology objects. A RAM key is the
 measured local or remote memory-domain key, a storage key comes from the
 measured storage domain, and a GPU key comes from the native GPU descriptor.

 ### Benchmark metadata

 The section starts with `benchmarks.seed_schema: u32`. Four
 `BoundedBenchmarkPlan` records follow in exactly this order: `ram`, `storage`,
 `gpu`, `network`. Each record is `buffer_bytes: u64`, `iterations: u32`, and
 `maximum_duration_ns: u64`. Decode reconstructs `Duration::from_nanos` and
 `ByteCount` directly; profile validation later requires every plan to be
 bounded and its duration to fit the canonical `u64` nanosecond form
 (`probe/src/codec.rs:871-903`).

 ### Peer benchmarks

 A `peer_benchmarks` count precedes the vector. Each item is:

 1. `session_id: Label`.
 2. `outbound_rate: Property<BytesPerSecond>`.
 3. `inbound_rate: Property<BytesPerSecond>`.
 4. `evidence.protocol_schema: u16`.
 5. `evidence.local_endpoint.machine: Digest`, then
    `evidence.local_endpoint.profile: Digest`.
 6. `evidence.remote_endpoint.machine: Digest`, then
    `evidence.remote_endpoint.profile: Digest`.
 7. `evidence.execution: u8`, with `Simultaneous = 0` and `Serialized = 1`.
 8. `evidence.outbound`, seven fields in this order: `total_bytes: u64`,
    `elapsed_nanoseconds: u64`, `sample_count: u32`,
    `minimum_sample_nanoseconds: u64`, `maximum_sample_nanoseconds: u64`,
    `mean_sample_nanoseconds: u64`, and
    `variance_nanoseconds_squared: u128`.
 9. `evidence.inbound`, with the same seven fields in the same order.

 `encode_peer_evidence` writes endpoints as the local pair followed by the
 remote pair and directions as outbound followed by inbound. Decoder error
 paths include the item index and a field prefix for diagnosis
 (`probe/src/codec.rs:905-987`).

 ### Enum tags

 The codec uses explicit stable tags, not Rust discriminants:

 | Enum | `0` | `1` | additional |
 | --- | --- | --- | --- |
 | `PeerDuplexExecution` | `Simultaneous` | `Serialized` | reject |
 | `NodeRole` | `Master` | `Worker` | reject |
 | `DeviceKind` | `GpuMemory` | `Ram` | `2 = Disk`, otherwise reject |
 | `TransportKind` | `Memory` | `Pcie` | `2 = Sata`, `3 = Sas`, `4 = Nvme`, `5 = Ethernet`, `6 = Wlan`, otherwise reject |
 | `DuplexMode` | `Half` | `Full` | reject |
 | `PropertyProvenance` | `Estimated` | `Measured` | `2 = Override`, otherwise reject |

 ### Topology

 `encode_topology` begins with `topology.identity.digest()` as 32 raw bytes.
 It then writes:

 * `topology.machines` count; each machine is `id: u64`, `name: Label`.
 * `topology.nodes` count; each node is `id: u64`, role tag, `machine: u64`,
   a device-vector count, and each node device ID as `u64`.
 * `topology.devices` count; each device is `id: u64`, `machine: u64`, kind
   tag, capacity property, transfer-rate property, a one-byte
   `has_calculation_rate` marker, and, when true, a calculation-rate FLOPS
   property.
 * `topology.links` count; each directed link is `id: u64`, `transport: u64`,
   transport-kind tag, duplex tag, `from: u64`, `to: u64`, bandwidth property,
   maximum-inflight-transfers property, and `capacity_resource: u64`.

 Decoder reconstructs `TopologyIdentity`, `Machine`, `Node`, `Device`, and
 `DirectedLink` directly. The optional GPU calculation rate is controlled by
 the marker, not by a sentinel numeric value (`probe/src/codec.rs:1029-1164`).

 ### Discovery

 `encode_discovery` writes `discovery.identity.digest()` and
 `discovery.topology.digest()` as two 32-byte digests, then:

 * `discovery.devices` count. Each device is `device: u64`, `available: bool`,
   `maximum_submission_queues: u32`, total-capacity property, transfer-rate
   property, transfer maximum-inflight property, transfer asynchronous bool,
   transfer-overlap bool, and a `has_calculation` bool.
 * When `has_calculation` is true, the calculation capability is
   `target.backend: Label`, `target.architecture: Label`, `target.abi: Label`,
   FLOPS rate property, asynchronous-submission bool,
   `maximum_concurrent_tasks: u32`, `subgroup_lanes: u32`,
   `maximum_workgroup_lanes: u32`, and
   `maximum_shared_memory_per_workgroup: u64`.
 * `discovery.links` count. Each link is `link: u64`, `available: bool`,
   bandwidth property, maximum-inflight property, and asynchronous-submission
   bool.

 Decoder reconstructs `DiscoveryIdentity`, its `TopologyIdentity` owner, and
 the capability records (`probe/src/codec.rs:1166-1279`). The discovery
 topology digest is separate from, but must equal, the decoded topology's
 identity under `DiscoveryProfile::validate`.

 ## Validation before persistence

 `validate_profile` short-circuits in this order (`probe/src/codec.rs:121-157`):

 1. `profile.schema == PROFILE_SCHEMA`.
 2. `profile.cache_identity.schema == PROFILE_SCHEMA`.
 3. `profile.cache_identity.digest` is not zero.
 4. Benchmark metadata is valid and bounded.
 5. Every peer benchmark is internally consistent and derivable from the
    network plan.
 6. `Topology::validate()` succeeds.
 7. `Topology::validate_scheduling_properties()` succeeds.
 8. `DiscoveryProfile::validate(&profile.topology)` succeeds.
 9. Retained origins are complete, typed, unique, and linked to topology.
 10. All persisted vectors are in strict canonical order.
 11. Every persisted schedulable property has `Measured` provenance.
 12. Topology and discovery values agree exactly.
 13. Peer benchmark records have a one-to-one, evidence-compatible cross-machine
     topology transport.

 The first eight checks delegate core invariants as well as codec-specific
 checks. `Topology::validate` requires nonzero topology identity, unique IDs
 and machine names, known machine/device references, one master node, every
 device owned once, two opposing edges per transport, legal transport duplex,
 nonzero transport and capacity-resource IDs, and correct half/full resource
 sharing. `validate_scheduling_properties` rejects estimated properties. The
 discovery validator requires a nonzero discovery identity, the matching
 topology identity, exactly all required devices and links, availability,
 nonzero submission/capability limits, asynchronous transfer and calculation,
 schedulable measured or overridden properties, GPU-only calculation capability,
 and exact topology direction values. Core validation errors are converted to
 `ProbeError::Cache("codec: ...")` strings by the codec.

 ### Benchmark-plan checks

 `validate_benchmark_metadata` requires `seed_schema == CONTRACT_SCHEMA`, each
 plan's `is_bounded()` result to be true (nonzero buffer, iterations, and
 duration), and every duration to fit in a `u64` nanosecond field. It validates
 RAM, storage, GPU, and network plans independently.

 `validate_peer_benchmarks` computes
 `expected_bytes = network.buffer_bytes * network.iterations` with checked
 multiplication and converts the network maximum duration to `u64` nanoseconds.
 For each record it requires protocol schema `1`, nonzero machine and profile
 endpoint digests on both sides, and different endpoint machine digests. For
 outbound and inbound independently, `validate_directional_evidence` requires:

 * `total_bytes == expected_bytes`;
 * nonzero `sample_count` equal to network `iterations`;
 * nonzero elapsed time no greater than the plan maximum;
 * nonzero minimum sample time;
 * `minimum <= mean <= maximum <= elapsed`.

 The stored directional rate must equal `evidence_rate`, which computes
 `total_bytes * 1_000_000_000 / elapsed_nanoseconds` with checked `u128`
 arithmetic, clamps the result to `[1, u64::MAX]`, and constructs a nonzero
 `BytesPerSecond`. A mismatch is rejected even when the rate itself has a
 valid unit representation. The variance is persisted as evidence but is not
 independently recomputed by the codec.

 ### Origin checks

 `validate_origins` builds ID maps and checks each origin kind separately:

 * A machine origin must reference a topology machine exactly once. Its
   fingerprint hostname must equal that machine's topology name. Stable
   machine IDs must be unique across all machine origins, and every topology
   machine must have one origin.
 * A RAM origin must reference a known `DeviceKind::Ram` exactly once. Its key
   must be unique within the referenced machine, and every RAM device must have
   one origin.
 * A storage origin has the analogous requirements for
   `DeviceKind::Disk`.
 * A GPU origin has the analogous requirements for
   `DeviceKind::GpuMemory`.

 Unknown IDs, duplicate IDs, wrong device kinds, missing origins, duplicate
 machine-scoped keys, and a machine name/fingerprint mismatch all fail with a
 codec error. The key is the retained identity. Capacity and transfer rate are
 never used as an origin selector.

 ### Canonical order

 `require_canonical_order` requires strict increasing order, not merely sorted
 order with equal values allowed, for:

 * `origins.machines` by machine ID;
 * `origins.ram`, `origins.storage`, and `origins.gpu` by device ID;
 * `peer_benchmarks` by `session_id` label;
 * `topology.machines`, `topology.nodes`, `topology.devices`, and
   `topology.links` by their IDs;
 * every `topology.node[ID].devices` vector by device ID; and
 * `discovery.devices` by device ID and `discovery.links` by link ID.

 The generic `require_increasing` and `require_increasing_ref` helpers reject a
 value when the previous value is greater than or equal to it. Canonical order
 makes byte output deterministic and makes canonical encoded bytes safe as a
 content identity. The encoder does not sort; callers must construct or remap
 profiles in this order, and validation rejects otherwise.

 ### Measured-only persistence

 `require_exclusively_measured` rejects `Estimated` and `Override` provenance
 for every property that can be persisted into a production profile:

 * peer outbound and inbound rates;
 * topology device capacity, transfer rate, and optional calculation rate;
 * topology link bandwidth and concurrency;
 * discovery device capacity, transfer rate, transfer concurrency, and optional
   calculation rate; and
 * discovery link bandwidth and concurrency.

 This is stricter than core schedulability, which permits explicit overrides.
 A persisted probe profile is evidence from measurement, not a seed estimate or
 an operational override.

 ### Topology/discovery equality

 `require_topology_discovery_match` requires every topology device and link to
 have a discovery entry. Device capacity and transfer properties must compare
 equal as complete `Property` values, including provenance. Calculation must be
 absent in both records or present with equal rate properties. Link bandwidth
 and maximum-inflight properties must likewise compare equal. Availability,
 asynchronous flags, and other discovery capability fields are checked by the
 delegated discovery validator, not duplicated here.

 ### Peer transport/evidence equality

 `require_peer_topology_match` maps topology devices, groups only cross-machine
 directed links by `TransportId`, and requires each transport to have exactly
 two opposing directions with equal kind and duplex. The number of such
 transports must equal the number of peer benchmark records.

 For each benchmark, the pair of authenticated endpoint tuples
 `(machine_digest, profile_digest)` is canonicalized by tuple ordering and must
 be unique. A local and remote endpoint may not be equal. Each benchmark is
 matched to one unused cross-machine transport where either direction ordering
 agrees with outbound/inbound rates. Simultaneous evidence requires full duplex
 and distinct directional capacity resources. Serialized evidence requires
 half duplex and one shared capacity resource. No transport can satisfy two
 benchmark records. This binds the measured peer evidence to an actual
 cross-machine pair without deriving identity from measured throughput.

 ## Identity and hashing boundaries

 `CacheIdentity { schema, digest }` is serialized explicitly and must use the
 profile schema. The engine computes it with `CanonicalDigest::new("recipe-probe-cache-v7", PROFILE_SCHEMA)` over the seed contract,
 machine fingerprint, all discovered RAM/storage/network/GPU descriptors, and
 peer descriptors (`probe/src/engine.rs:575-668`). A changed relevant identity
 facet, driver, firmware, link, target, native queue limit, host key, or peer
 descriptor therefore changes the cache digest before measurements are reused.

 Topology and discovery identities are separate 32-byte digests. The engine
 currently constructs them with the domain strings `recipe-topology-v6` and
 `recipe-discovery-v6` while passing the current profile schema to
 `CanonicalDigest::new` (`probe/src/engine.rs:108-113`, `727-766`). Their raw
 digests are encoded in their respective sections. Discovery also stores the
 topology identity it belongs to, and delegated validation requires equality.

 A machine origin retains the stable machine fingerprint. RAM, storage, and
 GPU origins retain the exact domain key needed to reopen a current device.
 Peer evidence carries authenticated machine and complete-profile digests at
 both endpoints. The codec verifies nonzero and distinct endpoint identities
 and uniqueness of endpoint pairs, but does not recompute or authenticate a
 remote digest itself. That authentication is established by the peer protocol
 before the `MeasuredPeerBenchmark` reaches this boundary.

 `MeasuredProfile::is_cache_valid_for` performs only schema and exact
 `CacheIdentity` equality (`probe/src/model.rs:484-488`). File naming and cache
 identity checks are separate from the binary checksum. A file can have a
 correct checksum yet fail identity comparison or semantic validation.

 ## Failure behavior

 Codec-specific semantic and parse failures use
 `codec_error(message)`, which returns `ProbeError::Cache(format!("codec: {message}"))`.
 This preserves the public error family as a cache/profile failure while
 retaining a field-specific detail. Important direct messages include:

 * input or encoded output over `MAXIMUM_PROFILE_BYTES`;
 * profile truncated below the minimum envelope or at any typed field;
 * checksum mismatch;
 * magic mismatch;
 * unsupported codec schema;
 * unconsumed payload bytes;
 * collection or label limits and integer conversion overflow;
 * invalid booleans, provenance, enum tags, UTF-8, labels, or nonzero units;
 * unsupported profile or cache schema and zero cache digest;
 * unbounded plans, duration and byte-count overflow;
 * malformed peer protocol, endpoint, duration, sample, rate, or duplex
   evidence;
 * unknown, duplicate, missing, mis-typed, or non-canonical origins;
 * estimated or overridden persisted properties;
 * topology/discovery mismatches; and
 * cross-machine transport or peer-record mismatches.

 `decode` never returns a partially decoded profile. It only constructs the
 `MeasuredProfile` after all sections have parsed and the payload is exhausted,
 then validates it. There is no retry, alternate schema, newest-file fallback,
 ordinal fallback, or tolerance for trailing bytes. The cache layer may report
 filesystem and ownership errors as `ProbeError::Io` or `ProbeError::Cache`,
 but codec failures remain visible through the same `ProbeResult`.

 ## Canonical contract for callers

 Callers must provide a complete profile already arranged in canonical order,
 with nonzero identities, paired links, exact topology/discovery copies, and
 `Measured` properties. A caller that only wants to inspect bytes must still
 accept that decode validates production admissibility. A caller that needs a
 profile for preparation must use the decoded profile's retained origins and
 then perform current-inventory resolution; codec validity alone does not prove
 that today's hardware is the same hardware.

 The practical boundary is therefore:

 ```text
 discovery and bounded measurement
       -> MeasuredProfile construction
       -> validate_profile
       -> canonical encode + SHA-256 checksum
       -> private cache or cluster transport
       -> bounded decode + checksum/schema checks
       -> validate_profile again
       -> exact origin resolution
       -> topology/discovery planning and native realization
 ```

 A profile that does not pass every arrow is not a usable measured profile.
