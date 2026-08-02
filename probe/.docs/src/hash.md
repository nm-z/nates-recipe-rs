# probe::hash

probe/src/hash.rs contains the probe crate's one canonical incremental SHA-256
writer, CanonicalDigest. It is crate-private. probe/src/engine.rs is the only
caller: it uses this writer to derive the discovery-only cache identity and the
measured topology and discovery identities. The writer does not describe a
profile by itself. The exact call order and the fields selected by the engine
are the identity schema.

This is a probe-specific implementation. Other crates have types with similar
names and different encodings. In particular, the cluster and planner hash
modules are not interchangeable with this writer.

## Writer shape

~~~text
CanonicalDigest {
    hasher: sha2::Sha256,
}
~~~

The type derives Clone and Debug; it has no public visibility outside the crate.
Its methods are:

| method | operation | result |
| --- | --- | --- |
| new(domain, schema) | starts a fresh SHA-256 state and appends the domain string followed by the schema | a writer |
| string(value) | encodes the UTF-8 bytes of value through bytes | () |
| bytes(value) | appends an eight-byte length and the bytes | () |
| u64(value) | appends the integer in little-endian order | () |
| bool(value) | appends one byte, 0 for false or 1 for true | () |
| digest(value) | appends the 32 raw bytes of a recipe_core::Digest | () |
| finish() | finalizes SHA-256 and wraps the 32-byte output in Digest | recipe_core::Digest |

All methods append to the same state in the order called. There is no automatic
field name, type tag, record count, delimiter, or error result. The callers
supply any semantic tags and rely on their fixed schemas. finish consumes the
writer, so no input can be appended after finalization.

## Exact byte encoding

The notation below is useful when reproducing an identity:

~~~text
LP(b) = u64_le(len(b)) || b
U64(n) = n.to_le_bytes()
BOOL(v) = [0] or [1]
DIGEST(d) = d.bytes()
~~~

CanonicalDigest::new(domain, schema) initializes the hash with

~~~text
SHA256(LP(domain.as_bytes()) || U64(u64::from(schema)))
~~~

string uses the string's exact UTF-8 representation. bytes casts the slice
length to u64, writes that length in little-endian order, then writes the slice
unchanged. u64 writes exactly eight bytes, bool exactly one, and digest exactly
32. A digest is raw bytes, not a length-prefixed value. finish converts the
SHA-256 finalizer's [u8; 32] into recipe_core::Digest::new; that constructor
does not add a marker or reject a zero digest.

The length prefix is present only for values passed through bytes (including
strings). Numeric and digest fields are distinguished by their fixed position
in the relevant engine routine, not by bytes emitted by this module. Changing
the order, domain, schema, conversion, or field selection changes the identity
schema and therefore requires an intentional schema or domain change.

## Ordering before hashing

The writer preserves caller order, so the engine establishes order before it
calls the writer:

- normalize_host sorts RAM domains, storage domains, and network interfaces by
  key and rejects duplicate keys. GPU descriptors are sorted by key after
  exhaustive discovery. Peer descriptors are sorted by session ID and their
  session IDs are unique.
- The engine stores RAM, storage, and GPU measurements in BTreeMaps. The profile
  digest therefore visits each measured map in key order. Peer measurements
  remain a vector in the already sorted session-ID order.
- SeedContract.invalidation is a BTreeSet, so its entries are visited in the
  enum's order (Machine, Device, Driver, RuntimeAbi, Firmware, Link,
  ArtifactToolchain). SeedContract.transports is a vector and is hashed in its
  supplied order. The TOML parser constructs that vector from a BTreeSet of
  names, which makes parsed contracts lexicographic by transport name, but a
  caller that constructs a SeedContract directly still controls the vector
  order.

The hash routines do not sort or validate their arguments themselves. They are
called only after the discovery and measurement checks in ProbeEngine have
established the ordering and uniqueness assumptions.

## Cache identity

build_cache_identity starts with:

~~~text
domain = "recipe-probe-cache-v7"
schema = PROFILE_SCHEMA (currently 7)
~~~

It then appends the following values, in this exact order. Every textual item
uses string; every numeric item uses u64; booleans use bool; and the one
toolchain digest uses digest.

### Seed contract

hash_seed appends:

1. seed.schema.
2. The twelve values in this fixed order:
   estimates.ethernet_rate, estimates.disk_capacity,
   estimates.sata_rate, estimates.gpu_memory_capacity,
   estimates.pcie_rate, estimates.gpu_calculation_rate,
   estimates.gpu_memory_rate, estimates.ram_capacity, estimates.ram_rate,
   estimates.cpu_reference_rate, estimates.ram_transfer_rate, and
   reservation_per_storage_device (the unit wrappers are reduced with get()).
3. Each invalidation facet, in BTreeSet order, using these strings:
   Machine -> "machine", Device -> "device", Driver -> "driver",
   RuntimeAbi -> "runtime-abi", Firmware -> "firmware",
   Link -> "link", and ArtifactToolchain -> "artifact-toolchain".
4. Each transport in vector order: its name, then "full" or "half" for its
   SeedDuplex.

SeedContract.policy is not hashed. The parser currently requires every
discovery, benchmark, and measured-profile gate to be enabled, but the policy
field itself is not an input to this digest. Transport direction and issue
settings are likewise not fields of TransportSeed and are not encoded here.

### Local machine

hash_machine appends the host fingerprint's hostname, stable_id, runtime_abi,
and firmware, in that order.

### RAM domains

For each normalized HostInventory.ram entry, the engine appends:

~~~text
"ram"
domain.key
domain.capacity_hint
domain.link_identity
domain.maximum_inflight_transfers
~~~

There is no separate count. The "ram" tag and the fixed field sequence frame
each record. A RAM domain's measured capacity and rate are intentionally not
present in the cache identity; measurements are added only to the profile
identities after probing.

### Storage domains

For each normalized HostInventory.storage entry, the sequence is:

~~~text
"storage"
domain.key
domain.name
domain.capacity_hint
domain.benchmark_root.to_string_lossy()
domain.host_memory_key
domain.driver
domain.firmware
domain.link_identity
transport kind ("memory", "pcie", "sata", "sas", "nvme", "ethernet", or "wlan")
domain.asynchronous_submission
domain.duplex == LinkDuplex::Full
domain.maximum_concurrent_reads
domain.maximum_concurrent_writes
~~~

The benchmark root is hashed as the exact PathBuf::to_string_lossy result, not
as platform-native path bytes. The transport mapping is implemented by
hash_transport_kind; it is textual and separate from the codec's numeric
transport tags.

### Network interfaces

For each normalized HostInventory.network entry, the sequence is:

~~~text
"network"
interface.key
interface.name
interface.address
interface.driver
interface.firmware
interface.link_identity
transport kind
interface.asynchronous_submission
interface.duplex == LinkDuplex::Full
~~~

NetworkInterface has no maximum-inflight fields, so none are hashed. The
interface list itself is part of the cache key even when no peer session uses a
particular interface.

### GPU descriptors

For each exhaustively discovered, key-sorted GPU descriptor, the sequence is:

~~~text
"gpu"
device.key
device.name
device.capacity_hint
device.host_memory_key
device.target.backend
device.target.architecture
device.target.abi
device.driver
device.runtime_abi
device.firmware
device.link_identity
transport kind
device.toolchain.name
device.toolchain.version
device.toolchain.digest
device.asynchronous_submission
device.maximum_submission_queues
device.maximum_concurrent_tasks
device.subgroup_lanes
device.maximum_workgroup_lanes
device.maximum_shared_memory_per_workgroup
device.transfer_overlaps_calculation
device.duplex == LinkDuplex::Full
device.host_to_device_maximum_inflight
device.device_to_host_maximum_inflight
~~~

The GpuInventory.exhaustive flag is checked before hashing but is not itself
encoded. A changed native submission-queue ceiling, concurrency limit,
subgroup/workgroup capability, shared-memory limit, transfer overlap behavior,
toolchain identity, or any other listed descriptor field therefore invalidates
the cache before a benchmark is reused.

### Peer descriptors

For each peer descriptor sorted by session_id, the sequence is:

~~~text
"peer"
peer.session_id
hash_machine(peer.machine)
peer.remote_memory_key
peer.local_memory_key
peer.local_interface_key
peer.remote_interface_identity
peer.remote_driver
peer.remote_firmware
peer.link_identity
transport kind
peer.asynchronous_submission
peer.duplex == LinkDuplex::Full
peer.outbound_maximum_inflight
peer.inbound_maximum_inflight
~~~

The peer session object or transport handle is not hashed. Only its descriptor is
part of the discovery-only cache identity.

After this sequence, build_cache_identity returns
CacheIdentity { schema: PROFILE_SCHEMA, digest: finish() }. The cache digest
therefore answers: "Will the same seed and discovered identity justify reusing
the measured profile?" It does not claim that any measured rate is unchanged.

## Measured topology and discovery identities

After the cache identity is computed and every benchmark is validated,
build_profile_digest is called twice:

~~~text
build_profile_digest("recipe-topology-v6", cache_identity, measurements)
build_profile_digest("recipe-discovery-v6", cache_identity, measurements)
~~~

Both calls use PROFILE_SCHEMA (currently 7), append the cache digest as raw
32 bytes, then append measured values. The two domains deliberately produce
different digests for the same bytes. The source names are therefore worth
noting: the profile schema constant is 7 while these two domain strings still
contain v6.

The measured sections are visited in this order:

1. **RAM map.** For each key in Measurements.ram (BTreeMap order):
   "ram", key, measured capacity, measured transfer rate.
2. **Storage map.** For each key in Measurements.storage: "storage", key,
   measured capacity, measured read rate, measured write rate.
3. **GPU map.** For each key in Measurements.gpu: "gpu", key, measured
   capacity, calculation rate, memory rate, host-to-device rate, and
   device-to-host rate.
4. **Peers.** For each (PeerDescriptor, PeerMeasurement) in sorted session-ID
   order: "peer", session ID, remote-memory capacity, remote-memory rate, the
   outbound rate if Some, the inbound rate if Some, and
   hash_peer_evidence(measurement.evidence).

There is no explicit option tag around the two peer rates. The probe engine
validates both directions as present and measured before this routine is called,
so a constructed Measurements with a missing rate is outside the normal profile
path. Provenance markers are also not hashed; the engine and codec require the
values used to build a persisted profile to have Measured provenance.

### Peer evidence encoding

hash_peer_evidence appends the complete authenticated and timing evidence in a
fixed order:

1. protocol_schema widened to u64.
2. The local endpoint, then the remote endpoint. Each endpoint contributes its
   machine digest followed by its profile digest, both as raw 32-byte values.
3. One boolean: true for PeerDuplexExecution::Simultaneous, false for
   Serialized.
4. The outbound direction, then inbound direction. Each contributes total_bytes,
   elapsed_nanoseconds, sample_count widened to u64,
   minimum_sample_nanoseconds, maximum_sample_nanoseconds,
   mean_sample_nanoseconds, and
   bytes(&variance_nanoseconds_squared.to_le_bytes()).

The variance call emits a 16-byte little-endian u128 preceded by an eight-byte
length prefix containing 16. This is different from a raw digest or a plain
u128 write and is part of the profile identity schema.

The resulting digest is stored as TopologyIdentity or DiscoveryIdentity. It is
not recomputed from serialized profile bytes by the codec. Serialized profiles
carry the identities and the cache identity; validation checks their schema,
nonzero status, and structural relationships, while cache lookup compares the
stored cache identity with the newly discovered one.

## Callers and consumers

The complete identity path is:

1. ProbeEngine::inspect discovers and normalizes the host and GPU inventory,
   obtains peer descriptors, validates references and capabilities, and
   computes the cache identity with build_cache_identity.
2. ProbeEngine::current_cache_identity exposes that exact discovery-only result.
   The CLI uses its schema and lowercase hexadecimal digest in the default
   measured-v<schema>-<digest>.recipe-profile filename.
3. ProbeEngine::load_or_probe_and_store asks the supplied ProfileCache for that
   identity. ExplicitPathProfileCache::load decodes and validates the profile,
   then rejects a file whose embedded CacheIdentity differs from the requested
   one. A cache miss runs probe and stores the fully validated profile; the
   cache never substitutes a newest or merely similar profile.
4. On a fresh probe, benchmark measurements and peer evidence are validated,
   then the topology and discovery digests are created from the cache digest
   plus those measurements. The resulting MeasuredProfile retains all three
   identity values: cache_identity, topology.identity, and discovery.identity.
5. MeasuredProfile::is_cache_valid_for is the model-level predicate for the same
   comparison: profile schema must equal PROFILE_SCHEMA and the complete
   CacheIdentity must equal the current one.
6. MeasuredProfileCodec serializes the cache schema/digest and the topology and
   discovery identities. It rejects an unsupported profile or cache schema, a
   zero cache digest, zero topology/discovery identities, malformed peer
   evidence, noncanonical ordering, or a topology/discovery relationship that
   does not validate. It does not derive a replacement digest.
7. The native-preparation boundary parses the identity from the exact cache
   filename, loads that exact CacheIdentity, and reopens the retained local
   origins. The active native receipt stores the same profile identity so a
   later training or preparation call cannot silently switch profiles.
8. The topology and discovery identities flow into prepare, planner, executor,
   remote, and training plans. Those components use them as immutable membership
   checks for schedules, candidates, bundles, and runtime sessions; they do not
   call CanonicalDigest directly.

Cluster assembly can wrap a per-machine measured profile in its own endpoint and
cluster identities. Its hash writer is a separate implementation and does not
change the probe encoding described here.

## Invariants and failure boundaries

CanonicalDigest itself cannot fail: all methods return (), new accepts any domain
and schema, and the length conversion is unchecked. It neither checks that a
domain is known nor rejects zero input digests. Correctness is supplied by the
fixed engine schemas and by validation around the call sites.

Before build_cache_identity, discovery must have produced a nonpartial host, an
exhaustive nonempty GPU inventory, unique keys, valid local-memory references,
asynchronous submission paths, nonzero GPU queue and task capacities, and peer
descriptors that agree with local interfaces and are not the local machine.
Before build_profile_digest, every RAM, storage, GPU, and peer measurement must
be present and Measured; peer evidence must have the expected protocol,
authenticated nonzero distinct endpoints, duplex-consistent execution, bounded
elapsed time, complete samples, and a rate derived from its bytes and duration.
Thus the hash routines consume established state rather than repairing or
filtering invalid state.

The deterministic assumptions are consequently:

- the domain and schema are explicit and fixed at each call site;
- strings are UTF-8 and paths use to_string_lossy where specified;
- all numeric fields are little-endian, with explicit widening to u64 where
  needed;
- collection ordering is normalized before hashing;
- identity digests are raw fixed 32-byte values; and
- identity construction is one-way SHA-256, while profile and cache validation
  is performed by the surrounding engine, codec, and cache layers.

The end-to-end role is therefore narrow and intentional: discovery determines a
stable cache key, measured execution extends that key into topology and
discovery identities, and those identities gate reuse and every later planning
or execution boundary. The hash writer supplies only the canonical bytes and
SHA-256 state; it does not own discovery, measurement, persistence, or runtime
authorization.
