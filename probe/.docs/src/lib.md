# `recipe-probe` crate root

`recipe-probe` is Recipe's bare-metal discovery and bounded-measurement
boundary. It converts an explicit seed contract and current host, GPU, and
peer observations into one `MeasuredProfile`. The profile contains the
measured `Topology`, measured `DiscoveryProfile`, benchmark evidence, and the
stable origins needed to reopen those objects later. A seed value is only a
bound for probe work. It is never emitted as a production rate or used as a
hardware inventory.

The crate is deliberately free of unsafe code (`probe/src/lib.rs:1`) and has
no operating-system or GPU implementation hidden behind a global singleton.
Discovery, benchmark I/O, peer sessions, and cache storage are traits or
explicit values supplied by the caller. The concrete Linux host implementation
is in `local`; the native GPU implementation lives in the sibling
`recipe-native-probe` crate and implements this crate's traits.

## Root module and exports

The root (`probe/src/lib.rs`) declares these modules:

| Module | Root visibility | Role |
| --- | --- | --- |
| `cache` | `pub mod`, wildcard re-exported | Secure explicit-path profile cache. |
| `codec` | `pub mod`, wildcard re-exported | Canonical binary profile codec and full profile validator. |
| `engine` | `pub mod`, wildcard re-exported | Discovery, bounded benchmark orchestration, identity hashing, and profile construction. |
| `error` | `pub mod`, wildcard re-exported | `ProbeError` and `ProbeResult`. |
| `hash` | `pub mod`, not wildcard re-exported | Crate-private SHA-256 canonical digest builder. Its methods are `pub(crate)`, so it is an implementation module rather than a caller API. |
| `local` | `pub mod`, not wildcard re-exported | Linux `/proc`, `/sys`, and `/etc` discovery plus host RAM and disk benchmarks. Callers use `recipe_probe::local::...`. |
| `model` | `pub mod`, wildcard re-exported | Inventory, benchmark, peer-control, profile, origin, and cache traits and data types. |
| `resolve` | `pub mod`, wildcard re-exported | Exact re-association of a live inventory with profile IDs. It also adds `MeasuredProfile::resolve_local_inventory`. |
| `seed` | `pub mod`, wildcard re-exported | Strict seed-contract parser and policy/estimate types. |

The wildcard exports make the normal API available directly as
`recipe_probe::ProbeEngine`, `recipe_probe::MeasuredProfile`,
`recipe_probe::SeedContract`, `recipe_probe::MeasuredProfileCodec`, and so on.
`LocalSystemDiscovery` and `LocalHostBenchmarks` remain under the `local`
namespace to keep the concrete Linux choice explicit. `CanonicalDigest` is
not a public construction mechanism.

`Cargo.toml` depends only on `recipe-core`, `rustix` (filesystem/process
facilities), and `sha2`. Core owns IDs, topology, discovery types, measured
property provenance, units, and identity wrappers. Probe owns how current
hardware is discovered, measured, validated, and persisted.

## Module responsibilities and public surface

### `model`: the dependency-injected vocabulary

The model module defines the values exchanged by all other modules. Schema
constants are `PROFILE_SCHEMA = 7`, `PROFILE_CODEC_SCHEMA = 7`, and
`PEER_BENCHMARK_PROTOCOL_SCHEMA = 1`.

Discovery inputs are represented by:

- `MachineFingerprint`, with hostname, stable machine ID, runtime ABI, and
  firmware labels.
- `RamDomain`, `StorageDomain`, and `NetworkInterface`, which carry stable
  discovery keys, capacity hints, link identity, transport/duplex, and
  submission limits. Storage also carries its benchmark directory, host-memory
  key, driver, and firmware. Network interfaces are identity inputs even
  though local network throughput is measured through an explicit peer session.
- `HostInventory`, containing one machine fingerprint and vectors of RAM,
  mounted storage, and network domains.
- `GpuDescriptor` and `GpuInventory`. A descriptor carries the stable GPU key,
  `TargetIdentity`, host-memory key, capacity hint, driver/runtime/firmware,
  toolchain identity, transport details, queue/task/subgroup/workgroup
  limits, and overlap capability. `GpuInventory::exhaustive` is a proof bit,
  not a hint.

The injected discovery traits are:

```text
HostDiscovery::discover_host() -> ProbeResult<HostInventory>
GpuDiscovery::discover_all()  -> ProbeResult<GpuInventory>
```

An implementation must enumerate every usable local domain, or return an error.
The engine rejects a GPU inventory whose `exhaustive` bit is false and never
builds an accepted profile from a partial list.

Benchmark inputs and outputs are:

- `BoundedBenchmarkPlan`, with nonzero `buffer_bytes`, `iterations`, and
  `maximum_duration`. `is_bounded` is the shared basic guard.
- `RamMeasurement` (`capacity`, `transfer_rate`), `StorageMeasurement`
  (`capacity`, `read_rate`, `write_rate`), and `GpuMeasurement` (`capacity`,
  calculation rate, memory rate, and both host/device transfer rates). Every
  `Property` in a persisted profile must have `Measured` provenance.
- `HostBenchmarkIo`, with `benchmark_ram` and `benchmark_storage`, and
  `GpuBenchmarkIo`, with `benchmark_gpu`. The engine calls each once per
  discovered domain using a plan derived from the seed.

Peer probing has a richer control and evidence model:

- `PeerDescriptor` names the session, remote machine/memory/interface, local
  memory/interface, remote driver/firmware, link identity, transport, duplex,
  direction limits, and asynchronous-submission capability.
- `PeerMeasurement` carries measured remote memory properties, optional
  outbound/inbound rates, and `PeerBenchmarkEvidence`.
- `PeerBenchmarkCancellation` is cloneable caller-owned `Arc<AtomicBool>`
  state. `cancel` uses release ordering and `is_cancelled` uses acquire
  ordering.
- `PeerBenchmarkControl` combines an absolute `Instant` deadline with the
  cancellation state. `for_plan` sets the deadline to now plus the plan's
  duration and reports an `Instant` overflow. `failure` reports cancellation
  before deadline expiry, then a deadline failure after expiry.
- `PeerBenchmarkPhase` records validation, begin exchange, ready exchange,
  directional transfer, or completion exchange. `PeerBenchmarkFailureKind`
  distinguishes cancellation, deadline, identity, integrity, protocol, and
  transport failures. `PeerBenchmarkFailure` retains protocol schema, phase,
  kind, and detail.
- `PeerBenchmarkAttempt` is either `Measured(PeerMeasurement)` or
  `Failed(PeerBenchmarkFailure)`. `into_measurement` is the only conversion
  used by profile construction and turns a failed attempt into
  `ProbeError::Benchmark`; failure evidence cannot masquerade as measured
  data.
- `PeerDuplexExecution`, `PeerEndpointEvidence`,
  `DirectionalBenchmarkEvidence`, and `PeerBenchmarkEvidence` retain the
  authenticated endpoint digests, simultaneous versus serialized execution,
  byte count, elapsed time, sample statistics, and protocol schema required to
  audit a rate.
- `MeasuredPeerBenchmark` is the persisted session ID, two measured rates, and
  the complete evidence record.

`PeerSession` is the only source of network throughput. It exposes
`descriptor`, `benchmark`, and `benchmark_controlled`. The default controlled
method checks the control at validation and completion, invokes `benchmark`,
and maps an error to a directional transport failure. Blocking transports are
expected to override it so every I/O operation receives the supplied deadline.
`recipe-transport::TcpPeerSession` is the production implementation: it uses
framed begin/ready/transfer/complete exchanges, authenticates both endpoint
identities, and returns the structured evidence consumed by `ProbeEngine`.

Profile and identity types are:

- `CacheIdentity { schema, digest }`, ordered and hashable. The digest is the
  canonical identity of the seed and discovered inputs, before measurements.
- `MeasuredMachineOrigin`, `MeasuredRamOrigin`, `MeasuredStorageOrigin`, and
  `MeasuredGpuOrigin`. Origins retain the exact discovery strings that core's
  topology intentionally omits. RAM, storage, and GPU keys are scoped to their
  topology machine and come from the discovery key, never from capacity,
  ordinal, product name, or rate similarity.
- `MeasuredOrigins`, the four canonical origin vectors.
- `BenchmarkMetadata`, retaining the seed schema and the exact bounded plan
  for RAM, storage, GPU, and network work.
- `MeasuredProfile`, containing `schema`, `cache_identity`, `origins`,
  `benchmarks`, `peer_benchmarks`, `topology`, and `discovery`.
- `ProfileCache`, the explicit `load(CacheIdentity)` and
  `store(&MeasuredProfile)` boundary used by the engine. A cache may return
  `None` for an absent file, but a present malformed or stale profile is an
  error, not a cache miss.

`MeasuredProfile` provides `is_cache_valid_for`, four origin lookup helpers,
and, through `resolve`, `resolve_local_inventory`. The lookup helpers are
non-mutating; profile construction and validation happen before a profile is
returned or stored.

### `seed`: strict theoretical input

`SeedContract::read` reads a path and delegates to `SeedContract::parse`.
`parse` is a deliberately small TOML-shaped parser, not a general TOML
dependency. It supports sections, comments outside quoted strings, continued
string arrays, and duplicate-key detection, then rejects unknown fields.

The public seed values are:

- `SeedEstimates`: Ethernet and SATA rates, disk/RAM/GPU capacities, PCIe and
  memory rates, GPU and CPU reference FLOP rates, and RAM transfer rate.
- `ProbePolicy`: discovery, capacity/calculation/transfer benchmark, and
  measured-profile-for-prepare gates. All seven gates must be `true`.
- `IdentityFacet`: machine, device, driver, runtime ABI, firmware, link, and
  artifact-toolchain invalidation facets.
- `SeedDuplex` and `TransportSeed`.
- `SeedContract`, with schema, estimates, a fixed
  `reservation.bytes_per_storage_device` of exactly one billion bytes, policy,
  invalidation set, and transport seeds.

Parsing requires schema `1`, kind `"probe-seed-estimates"`, command
`"recipe probe"`, environment `"bare-metal"`, every invalidation facet, and
at least one transport. Each transport must declare both directions and async
issue, with `full` or `half` duplex. The contract has no machine, device, or
production-rate inventory fields. Invalid syntax and values carry an optional
source line in `ProbeError::Contract`.

### `local`: concrete Linux host path

`LocalSystemDiscovery::default` uses `/proc`, `/sys`, and `/etc`, with no
benchmark-root override. `with_benchmark_roots` changes only the ordered list
of already-created writable directories considered for temporary disk probes;
it does not declare a device or a rate.

`HostDiscovery::discover_host` reads the hostname, the first available stable
machine ID (`/etc/machine-id` then DMI product UUID), kernel release, and
available firmware fields. RAM comes from NUMA `node*/meminfo` when present,
otherwise `/proc/meminfo`. Storage is derived from mounted filesystems in
`/proc/self/mountinfo`, grouped back to physical block devices through sysfs,
and classified as NVMe, SAS, or SATA with full/half duplex. Network discovery
enumerates `/sys/class/net` except `lo`, captures address, ifindex, driver and
firmware, and classifies wired versus wireless transport and duplex.

The implementation fails rather than returning a partial host inventory. It
requires a RAM domain, mounted storage, valid stable labels, and a writable
benchmark directory on the physical disk. Temporary access-test files and
benchmark files are removed on all normal and error paths.

`LocalHostBenchmarks` implements `HostBenchmarkIo`. RAM copies a fixed buffer
for at most the plan's iterations and duration. Storage writes and syncs a
temporary file, then seeks and reads it for the same bound, and measures
available filesystem bytes with `statvfs`. `require_bounded` rejects zero
buffer, iteration, or duration. Rates are derived from completed bytes and
elapsed nanoseconds; no completed iteration or measurable elapsed time is an
error.

### `engine`: one orchestration boundary

`ProbeEngine<'a>` borrows exactly four implementations:

```text
HostDiscovery, GpuDiscovery, HostBenchmarkIo, GpuBenchmarkIo
```

`ProbeEngine::new` stores those references. Its public methods are:

- `probe(seed, peer_sessions)`: perform a fresh complete probe and return a
  validated `MeasuredProfile`.
- `current_cache_identity(seed, peer_sessions)`: run discovery and validation
  only, returning the exact cache identity without benchmarking.
- `load_or_probe_and_store(seed, peer_sessions, cache)`: compute the current
  identity, ask the explicit cache for that identity, and return it when
  present. On `None`, perform `probe_and_store`.
- `probe_and_store(seed, peer_sessions, cache)`: fresh `probe`, then `store`;
  the cache never receives a partially built profile.

The private `BenchmarkPlans::from_seed` derives one plan per resource class:
RAM uses the RAM-capacity estimate divided by 1024, storage uses disk capacity
divided by 16,384, GPU uses VRAM capacity divided by 1024, and network uses
Ethernet rate divided by 8. Each suggestion is clamped to 4 KiB through 64
MiB, with eight iterations and a two-second maximum duration. The resulting
plans, including the seed schema, are retained in `BenchmarkMetadata`.

The `probe` call graph is:

```text
inspect
  discover_host -> normalize_host
  discover_all -> require exhaustive, nonempty GPU inventory
  PeerSession::descriptor for each supplied session
  validate host, GPU, and peer references and uniqueness
  build_cache_identity
benchmark every RAM, storage, GPU, and peer domain
  validate measured provenance and peer evidence
build_profile_digest("recipe-topology-v6", ...)
  assign deterministic IDs -> build_topology
build_origins
build_profile_digest("recipe-discovery-v6", ...)
  build_discovery from the topology IDs and measured capabilities
validate topology, scheduling properties, discovery, and full profile
return MeasuredProfile
```

The digest labels above are implementation domain names. The cache identity
domain is `recipe-probe-cache-v7`; all are SHA-256 domains created by
`CanonicalDigest` and include the profile schema. `CanonicalDigest` length
prefixes byte strings, writes integers little-endian, writes booleans as one
byte, and includes nested digests, preventing ambiguous concatenations.

`inspect` sorts RAM, storage, network, GPU, and peer records by stable key
before identity hashing. It rejects empty RAM or mounted storage, unknown
host-memory references, duplicate keys, non-asynchronous storage/GPU/peer
paths, zero GPU queue/task capacities, a peer that names the local machine,
and local interface transport/duplex contradictions.

Each measured property must be `PropertyProvenance::Measured`. A peer must
provide both directions and nonzero authenticated endpoint digests. Full
duplex evidence must say `Simultaneous`; half duplex must say `Serialized`.
Directional evidence must exactly account for `buffer_bytes * iterations`, a
nonzero elapsed time no greater than the plan duration, the expected sample
count, monotonic sample statistics, and a rate derived from elapsed time.

Topology construction assigns machine ID 1 to the local host and IDs 2 and
upward to sorted peers. Device IDs are assigned in RAM, storage, GPU, then
peer-RAM order. A local node is a master; each peer becomes a worker node.
Every storage, GPU, and peer relationship becomes two directed links sharing
one transport ID. Full duplex links get distinct capacity resources; half
duplex links share one resource. Link bandwidth and inflight limits are
measured values. Storage topology uses the lower of read and write rates for
the device transfer rate, while the directional links retain each measured
direction. Discovery mirrors those IDs and adds transfer/calculation
capabilities, queue limits, async submission, and overlap flags.

The engine validates both core topology and scheduling properties, then the
discovery profile against that topology, and finally calls
`validate_profile`. Consequently `Ok(profile)` means the cache/codec contract
also holds, not merely that discovery returned data.

### `codec`: persistence format and validation gate

`MeasuredProfileCodec::encode` first calls `validate_profile`, then emits one
canonical little-endian byte stream. The stream is:

```text
16-byte MAGIC = "RECIPEPROFILE\0\0\0"
u32 codec schema
u32 profile schema
u32 cache schema
32-byte cache digest
origins
benchmark metadata
peer benchmarks and evidence
topology
discovery profile
32-byte SHA-256 checksum of all preceding bytes
```

`decode` enforces the 256 MiB `MAXIMUM_PROFILE_BYTES` limit, minimum framing,
checksum, magic, codec schema, complete consumption, and then calls
`validate_profile` on the decoded value. Length fields are limited to one
million items and labels to one MiB. Enum, boolean, provenance, UTF-8, and
numeric unit tags are checked while decoding. Codec failures are surfaced as
`ProbeError::Cache` messages prefixed with `codec:`.

`validate_profile` is the profile's central invariant gate. It requires profile
and cache schema 7, a nonzero cache digest, schema-1 bounded benchmark plans,
valid core topology and scheduling properties, a discovery profile that
matches the topology, stable origins for every machine and resource, strict
canonical ordering, measured provenance for every persisted rate/capacity/
concurrency property, and exact topology/discovery property equality. It also
proves each peer record matches exactly one opposing cross-machine link pair,
including endpoint evidence, rates, duplex execution, and shared versus
separate capacity resources.

### `cache`: explicit, secure profile state

`ExplicitPathProfileCache::new` accepts only an absolute path naming a file;
there is no default location. `path` returns that path. Before reading or
writing, the parent must be a canonical real directory owned by the effective
user with no group or other permission bits. A cache target must be a regular,
non-symlink file with the same ownership and private permissions. Reads use
`O_NOFOLLOW | O_CLOEXEC`, compare opened device/inode metadata with the earlier
`symlink_metadata`, enforce the codec size limit, decode, and require the
requested `CacheIdentity` exactly.

`ProfileCache::store` encodes and validates first. An existing identical
profile is a no-op; an existing different profile is an error. A new value is
written to a same-directory `create_new` temporary file with mode `0600`,
`sync_all`ed, installed with `renameat(..., NOREPLACE)`, and followed by a
parent-directory sync. `PendingFile::Drop` removes an uncommitted temporary
file. This is atomic installation without replacement, not a newest-file or
ordinal cache policy.

### `resolve`: exact current-inventory reopening

`ResolvedRamDomain`, `ResolvedStorageDomain`, and `ResolvedGpuDevice` pair a
borrowed current descriptor with the profile's `DeviceId`. Their accessors
return the ID and the original borrowed `RamDomain`, `StorageDomain`, or
`GpuDescriptor`. `ResolvedLocalInventory` exposes the exact local machine ID
and slices of those three resolved collections.

`MeasuredProfile::resolve_local_inventory(host, gpus)` first validates the
profile and requires an exhaustive current GPU inventory. It matches the live
machine by complete `MachineFingerprint`, then matches RAM, storage, and GPU
sets by the retained origin keys. Duplicate live keys, missing keys, newly
visible keys, wrong topology device kinds, missing host-memory domains, and
GPU target changes all fail with `ProbeError::InvalidProfile`. There is no
fallback to ordinal, product name, capacity, or benchmark similarity. A GPU
also must retain a discovery calculation capability whose `TargetIdentity`
equals the live descriptor.

## Cache identity, profile identity, and state transitions

The engine maintains three related but distinct identities:

1. `CacheIdentity` is discovery-only. It hashes the seed schema and all
   estimates, reservation, invalidation facets, transport seeds, the machine
   fingerprint, every RAM/storage/network/GPU descriptor field that can affect
   realization, the GPU toolchain identity, and every peer descriptor field.
   A changed machine, device, driver, runtime, firmware, link, or toolchain
   input therefore changes the identity before a benchmark runs.
2. `TopologyIdentity` is built after measurements and hashes the cache digest
   plus measured RAM, storage, GPU, and peer rates/capacities and complete peer
   evidence in the topology domain.
3. `DiscoveryIdentity` is built from the same measured inputs in a separate
   discovery domain. `DiscoveryProfile::topology` points back to the topology
   identity.

The normal state transition is:

```text
seed + current discovery
  -> CacheIdentity
  -> cache load(identity)
       -> Some(valid profile): return it
       -> None: benchmark and construct
  -> MeasuredProfile validation
  -> cache store(profile)
  -> downstream exact resolution and preparation
```

`ExplicitPathProfileCache` treats a stale identity, malformed bytes, insecure
path, or different existing profile as an error. It does not silently turn a
present but invalid cache into a fresh probe. The engine's `None` branch is
reserved for absence.

## Error surface and invariant failures

All fallible APIs return `ProbeResult<T> = Result<T, ProbeError>`. The
`#[non_exhaustive]` `ProbeError` variants are:

| Variant | Meaning |
| --- | --- |
| `Contract { line, message }` | Seed syntax, schema, policy, or unknown-field failure; `line` is retained when known. |
| `Discovery(String)` | Host/GPU/peer enumeration or descriptor inconsistency. |
| `Benchmark(String)` | Invalid bounds, no measurable work, overflow, failed peer attempt, or inconsistent evidence. |
| `IncompleteGpuEnumeration` | GPU discovery did not prove exhaustive coverage. |
| `MissingMeasurement(String)` | A returned property was estimated or overridden instead of measured. |
| `IncompletePeerMeasurement { peer, direction }` | Peer omitted outbound or inbound throughput. |
| `InvalidProfile(String)` | Topology, discovery, origins, identity resolution, or profile invariants failed. |
| `Cache(String)` | Cache policy or codec framing/validation failure. |
| `Io { operation, path, message }` | Filesystem operation failure with operation and path context. |

`ProbeError::contract` and `ProbeError::io` are constructors for the two
structured forms. `Display` prefixes errors with `contract`, `discovery`,
`benchmark`, `invalid measured profile`, `profile cache`, or the filesystem
operation, so CLI callers can report one useful message without inspecting
implementation details.

## End-to-end role in Recipe

The public `recipe` facade re-exports this crate as `recipe::engine::probe`.
The production command path in `src/cli.rs` is:

1. `recipe probe` reads the explicit contract or the checked-in
   `topology/contract.toml` through `SeedContract`.
2. It creates a private state and scratch directory, then uses
   `LocalSystemDiscovery::with_benchmark_roots` to discover the host and select
   the host-memory key for native configuration.
3. `recipe-native-probe::NativeGpuProbe` is constructed from explicit backend
   libraries and compiler paths. It implements both `GpuDiscovery` and
   `GpuBenchmarkIo`, revalidating exact backend/hardware ownership for every
   benchmark.
4. `ProbeEngine::new` receives the host discovery, native GPU probe, local host
   benchmarks, and native GPU benchmarks. The zero-argument CLI supplies no
   peer sessions.
5. The CLI computes the identity, chooses an identity-named profile path unless
   `--profile` was supplied, creates `ExplicitPathProfileCache`, and calls
   `load_or_probe_and_store`.
6. On success it writes the active-native receipt and prints profile,
   cache/topology/discovery identities, and object counts. The receipt is a
   CLI-owned handoff, not part of the profile artifact.

Preparation and execution consume the profile rather than the seed.
`src/native_prepare.rs::load_cached_measured_profile` derives the expected identity
from the identity-named filename and loads it through `ExplicitPathProfileCache`.
`with_native_preparation` discovers current GPUs, calls
`resolve_local_inventory`, realizes exact CUDA/HSA bindings, and lends them to
one callback whose result cannot borrow dynamic handles. `with_current_native_
preparation` reuses one thread-local initialized native probe while rebuilding
per-run resources.

`recipe-native-probe::with_native_execution_bindings` follows the same exact
resolution rule before opening native contexts. `recipe-prepare` calls
`validate_profile` before planning and consumes only the profile's topology
and discovery data. Its native candidate realizer retains those identities
while compiling and realizing artifacts. The scheduler consumes measured
topology routes and discovery concurrency, and native executors retain the
topology/discovery identities through candidate, warm-pass, and loop state.
`recipe-transport::TcpPeerSession` can supply the peer-session side when a
distributed probe is used. `recipe-cluster` uses `MeasuredProfileCodec` to
encode and decode assembled cluster profiles, preserving the same validation
and identity rules.

Thus the crate's end-to-end contract is narrow and complete:

```text
explicit seed + exhaustive current discovery
  -> bounded real measurements
  -> measured topology/discovery plus stable origins
  -> canonical validated cache bytes
  -> exact current-inventory resolution
  -> preparation, scheduling, and native execution
```

No caller can turn a theoretical estimate, partial inventory, stale cache,
unrelated device, or failed peer attempt into a production measured profile.
