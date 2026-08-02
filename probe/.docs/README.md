# `recipe-probe`

This document describes the executable behavior of the `recipe-probe` crate in
this checkout. The crate is the bare-metal discovery and bounded measurement
boundary for Recipe. It does not accept a user-supplied device list or
production-rate table. A seed contract supplies only theoretical values used to
size finite benchmark operations. The probe discovers the current machine,
devices, links, and stable identities, measures the usable paths, and returns a
validated `MeasuredProfile` containing a `Topology`, a matching
`DiscoveryProfile`, benchmark metadata, peer evidence, and retained identity
origins (`probe/src/lib.rs:4-9`, `probe/src/model.rs:17-19,473-482`).

The crate is a Rust 2024 package named `recipe-probe`. Its only direct
dependencies are `recipe-core`, `rustix` (filesystem and process facilities),
and `sha2` (`probe/Cargo.toml:1-10`). `probe/src/lib.rs` forbids unsafe code, denies
missing `Debug` implementations, declares the nine modules below, and
glob-imports their public APIs (`probe/src/lib.rs:1-27`).

## Module map

| Module | Owned behavior and public boundary |
| --- | --- |
| `model` | Discovery inventories, GPU and host benchmark hooks, bounded plans, peer-session protocol types, cancellation/deadline control, measured profile/origin types, and the `ProfileCache` trait. |
| `seed` | Strict parser for the theoretical TOML-like seed contract. It validates schema, policy gates, cache invalidation facets, transports, and rejects inventory fields. |
| `engine` | `ProbeEngine`, which performs discovery, normalization, bounded measurements, cache identity calculation, profile construction, and end-to-end validation. |
| `local` | Linux `/proc` and `/sys` host discovery plus bounded RAM and mounted-storage benchmarks. |
| `codec` | Canonical binary encoding and decoding of a fully validated measured profile, with schema, checksum, size, origin, ordering, provenance, topology, and evidence checks. |
| `cache` | `ExplicitPathProfileCache`, an opt-in secure single-file cache with identity checking and no-replacement atomic installation. |
| `resolve` | Exact reassociation of a live exhaustive host/GPU inventory with the topology IDs and stable discovery keys retained in a profile. |
| `hash` | Internal length-prefixed SHA-256 helper used by cache, topology, and discovery identity domains. |
| `error` | Non-exhaustive `ProbeError` and `ProbeResult<T>` used by every probe boundary. |

There is no hidden global probe state in these modules. `ProbeEngine` borrows
four caller-owned trait objects for one operation. The concrete CLI and native
backend own runtime handles and filesystem paths outside this crate; the probe
crate receives them through the traits.

## Public model and state

### Schema and identity values

The source constants are `PROFILE_SCHEMA = 7`, `PROFILE_CODEC_SCHEMA = 7`, and
`PEER_BENCHMARK_PROTOCOL_SCHEMA = 1` (`probe/src/model.rs:17-19`). A
`CacheIdentity` is a `(schema, Digest)` pair. A `MeasuredProfile` stores its
profile schema, cache identity, `MeasuredOrigins`, `BenchmarkMetadata`, sorted
`MeasuredPeerBenchmark` records, the topology, and the discovery profile. Its
`is_cache_valid_for` method accepts only schema 7 plus exact identity equality;
the origin lookup methods are simple ID-indexed searches
(`probe/src/model.rs:419-535`).

The checked-in `probe/PROFILE_SCHEMA.md` still calls the format “version 6” and
mentions `measured-v6` filenames. The executable constants and codec in this
checkout enforce version 7, and the CLI formats its default filename from the
runtime identity schema. This README reports the code path as authoritative
(`probe/PROFILE_SCHEMA.md:1-44`, `probe/src/model.rs:17-19`,
`src/cli.rs:905-915`).

### Inventories and descriptors

`MachineFingerprint` contains hostname, stable machine ID, runtime ABI, and
firmware labels. A `HostInventory` contains one machine fingerprint, RAM
domains, mounted storage domains, and non-loopback network interfaces.
`RamDomain` carries a stable key, capacity hint, link identity, and an inflight
lane limit. `StorageDomain` additionally carries benchmark root, host RAM key,
name, driver, firmware, link identity, transport kind, duplex, read/write lane
limits, and asynchronous-submission capability. `NetworkInterface` carries a
key, name, address, driver, firmware, link identity, transport kind, duplex,
and asynchronous-submission flag (`probe/src/model.rs:21-85`).

`HostDiscovery::discover_host` must enumerate every locally usable RAM, mounted
storage, and network domain or return an error. A partial inventory is not a
valid success (`probe/src/model.rs:81-85`).

`GpuDescriptor` is the native-calculation identity and capability record. It
contains a stable key and name, host RAM key, `TargetIdentity`, capacity hint,
driver/runtime/firmware/link labels, transport and toolchain identities, duplex,
both host/device inflight limits, asynchronous-submission support, maximum
submission queues, maximum concurrent tasks, subgroup/workgroup limits, shared
memory capacity, and whether transfers overlap calculation. `GpuInventory`
contains the descriptors and an explicit `exhaustive` bit. `GpuDiscovery` is
required to cover every device visible to its native backend
(`probe/src/model.rs:87-122`).

### Plans and measurements

`BoundedBenchmarkPlan` has `buffer_bytes`, `iterations`, and
`maximum_duration`; `is_bounded` requires all three to be nonzero
(`probe/src/model.rs:124-136`). Host measurements report measured RAM capacity
and transfer rate, or measured storage capacity, read rate, and write rate.
GPU measurements report measured memory capacity, device-memory rate,
host-to-device rate, device-to-host rate, and calculation FLOP rate. The
`HostBenchmarkIo` and `GpuBenchmarkIo` traits are injection points, not
simulations: `ProbeEngine` calls them on every discovered domain/device and
rejects any returned property that is not marked `PropertyProvenance::Measured`
(`probe/src/model.rs:138-173`, `probe/src/engine.rs:72-91,429-461`).

### Peer protocol state

An explicit, established `PeerSession` is the sole source of measured network
throughput. Its descriptor identifies the session, remote machine and RAM,
local RAM and network interface, remote interface/driver/firmware, link,
transport, duplex, direction-specific inflight limits, and asynchronous
submission (`probe/src/model.rs:175-200,383-417`). A `PeerMeasurement` must
contain remote RAM capacity/rate, both directional rates, and structured
evidence. The optional rate fields exist at the trait boundary so incomplete
implementations can be rejected with `IncompletePeerMeasurement`; profile
construction requires both directions.

`PeerBenchmarkCancellation` is cloneable caller-owned state backed by an
`Arc<AtomicBool>`. `PeerBenchmarkControl` carries one absolute `Instant`
deadline and cancellation state. `for_plan` sets the deadline to now plus the
plan duration and reports an overflow as a benchmark error. `failure(phase)`
reports cancellation before deadline expiry, otherwise deadline expiry, or no
failure. Cancellation is advisory between framed operations; the deadline is
the hard upper bound (`probe/src/model.rs:202-274`).

Failure evidence names one of validation, begin exchange, ready exchange,
directional transfer, or completion exchange and one of cancelled, deadline,
identity, integrity, protocol, or transport. `PeerBenchmarkAttempt::Measured`
or `::Failed` is converted by `into_measurement`; a failed attempt always
becomes `ProbeError::Benchmark`, never a measured property
(`probe/src/model.rs:276-340`). The default `benchmark_controlled` checks the
control before calling the legacy `benchmark`, checks again at completion, and
maps a returned error to directional transport failure. Blocking sessions are
expected to override it so every operation observes the supplied deadline.

Peer evidence retains protocol schema, authenticated local/remote machine and
profile digests, simultaneous versus serialized execution, and per-direction
total bytes, elapsed nanoseconds, sample count, minimum, maximum, mean, and
integer variance (`probe/src/model.rs:342-381`).

## Seed contract

`SeedContract::read` reads a path and delegates to `parse`; parsing does not
modify the file or synthesize missing values (`probe/src/seed.rs:62-82`). The
accepted contract is schema 1 and kind `probe-seed-estimates`. It requires all
eleven estimates (Ethernet, disk, SATA, GPU VRAM, PCIe, GPU FLOP, GPU transfer,
RAM capacity, DDR, CPU reference FLOP, and RAM transfer), and the storage
reservation must be exactly 1,000,000,000 bytes. The command/environment must
be exactly `recipe probe` and `bare-metal`. All seven discovery, benchmark, and
measured-profile policy gates must be true. Cache invalidation must contain all
seven facets: machine, device, driver, runtime ABI, firmware, link, and
artifact toolchain (`probe/src/seed.rs:99-156`).

Transport tables are discovered from `transport.<name>.*` keys. At least one is
required, each must declare both directions and asynchronous issue, and duplex
must be `full` or `half`. Names are collected in a `BTreeSet`, so the resulting
transport vector is deterministic (`probe/src/seed.rs:158-245`).

The parser is a deliberately small assignment parser. It supports table
headers, quoted strings, comma-separated quoted string arrays, decimal unsigned
integers with underscores, booleans, and comments beginning with `#` outside a
quoted string. It supports a multiline array only until a line containing `]`.
It rejects malformed assignments, empty names, duplicate keys, unterminated
arrays, missing required keys, wrong string/boolean/integer forms, and every
unknown field. Unknown-field errors explicitly state that machine, device,
link, and production-rate inventory is discovered automatically
(`probe/src/seed.rs:249-472`).

The repository's default `topology/contract.toml` supplies the theoretical
values and transport duplex declarations. Those values influence only finite
buffer sizing; they are not copied into the measured topology as rates.

## `ProbeEngine` lifecycle

`ProbeEngine::new` stores references to a `HostDiscovery`, `GpuDiscovery`,
`HostBenchmarkIo`, and `GpuBenchmarkIo`. It has no internal mutable cache or
long-lived inventory (`probe/src/engine.rs:31-61`). The complete
`probe(seed, peer_sessions)` path is:

1. `inspect` discovers and normalizes the host, discovers all GPUs, obtains
   peer descriptors, validates identity relationships, sorts deterministic
   collections, and computes the current `CacheIdentity`.
2. `BenchmarkPlans::from_seed` converts seed estimates to four bounded plans.
3. Every host RAM domain, storage domain, GPU descriptor, and peer session is
   measured. Each result is checked for measured provenance and required
   fields, and peer evidence is checked against the plan.
4. Measurements are retained in sorted maps/vectors. Topology and discovery
   identity digests are computed, then topology, stable origins, and discovery
   capabilities are built.
5. Core topology validation, scheduling-property validation, discovery
   validation, and the complete codec/profile validator all run before the
   profile is returned.

The implementation is fail-fast: a discovery, benchmark, evidence, or
validation error stops the operation and no partial profile is returned
(`probe/src/engine.rs:63-161`).

### Inspection and admission checks

`inspect` calls `HostDiscovery::discover_host`, sorts RAM, storage, and network
by key, requires at least one RAM and one mounted storage domain, and rejects
duplicate keys. Every storage domain must reference a discovered RAM key and
declare asynchronous submission. GPU discovery must set `exhaustive = true`,
must return at least one device, and its keys are sorted and unique. Every GPU
must reference host RAM, be asynchronous, and report nonzero submission queues
and concurrent-task capacity (`probe/src/engine.rs:163-359`).

Peer descriptors are sorted by session ID and must have unique session IDs and
remote stable machine IDs. A peer cannot identify the local machine, must
reference an existing local RAM domain and network interface, must agree with
that interface's transport and duplex, and must declare asynchronous
submission (`probe/src/engine.rs:361-415`). The CLI currently passes an empty
peer slice, but callers such as transport and cluster assembly can supply
explicit sessions.

### Bounded plans

The engine uses a 4 KiB minimum buffer, a 64 MiB maximum buffer, eight
iterations, and a two-second maximum duration. Suggested bytes are calculated
as follows, then clamped to the minimum and maximum:

| Path | Suggested bytes |
| --- | --- |
| RAM | `seed.estimates.ram_capacity / 1024` |
| storage | `seed.estimates.disk_capacity / 16_384` |
| GPU | `seed.estimates.gpu_memory_capacity / 1024` |
| peer network | `seed.estimates.ethernet_rate / 8` |

The four plans and seed schema are retained as `BenchmarkMetadata`, so a
decoded profile records the exact bounds used for its measurements
(`probe/src/engine.rs:241-276`, `probe/src/model.rs:523-530`).

### Measurements and evidence

For each host RAM and storage domain the engine calls the host benchmark hook;
for each GPU it calls the native benchmark hook. A peer gets a fresh
`PeerBenchmarkControl` for the network plan and is run through
`benchmark_controlled`. RAM, storage, and GPU properties must all be measured.
Peer remote-memory properties and both directional rates must be measured, and
both rate options must be present (`probe/src/engine.rs:72-100,429-495`).

Peer evidence must use protocol schema 1, contain nonzero local and remote
machine/profile digests, and identify different endpoint machines. Full duplex
requires `Simultaneous`; half duplex requires `Serialized`. For each direction,
the engine requires `total_bytes = buffer_bytes * iterations`, a nonzero elapsed
time no longer than the plan duration, exactly the plan iteration count, and
ordered nonzero sample statistics. The stored rate must equal the duration-
derived byte rate (with arithmetic overflow rejected)
(`probe/src/engine.rs:496-569`).

### Cache identity and hash domains

`build_cache_identity` uses `CanonicalDigest::new("recipe-probe-cache-v7", 7)`.
It hashes the seed schema, every estimate and reservation, the invalidation
facets, and transport names/duplex values (the parser fixes every policy gate
to true), then the machine fingerprint, every RAM key and
capacity/link/lane property, every storage name/capacity/benchmark-root/host
RAM/driver/firmware/link/transport/duplex/asynchronous/read/write property,
every network key/name/address/driver/firmware/link/transport/duplex/
asynchronous property, every GPU descriptor field including target, toolchain,
capabilities, overlap, queue ceiling, and both transfer lane limits, and every
peer descriptor field. Collections are normalized before hashing. Any changed
machine, device, driver, runtime, firmware, link, toolchain, transport, or
seed-bound value therefore yields a different cache identity
(`probe/src/engine.rs:575-718`).

`CanonicalDigest` length-prefixes byte/string values, writes integers in
little-endian order, writes booleans as one byte, and finishes with SHA-256
(`probe/src/hash.rs:4-35`).

After measurements, `build_profile_digest` creates topology and discovery
digests from the cache digest plus every measured capacity/rate and full peer
evidence. The domain strings in this code are
`recipe-topology-v6` and `recipe-discovery-v6`; the enclosing profile/cache
schema remains 7. This distinction is part of the current digest bytes and is
not inferred from the prose schema note (`probe/src/engine.rs:727-786`).

### ID assignment, origins, topology, and discovery

IDs are regenerated deterministically from normalized order. The local machine
is `MachineId(1)` and peer machines are `2 + sorted peer index`. Device IDs
start at one and are allocated in RAM order, storage order, GPU order, then
peer-RAM order. The local node is `NodeId(1)` with `Master` role; peers become
worker nodes with IDs starting at two. These IDs are profile-local topology IDs,
not hardware identities (`probe/src/engine.rs:789-844`).

`MeasuredOrigins` preserves the stable machine fingerprint and the exact
discovery key for every RAM, storage, and GPU-memory device. Peer RAM origins
use the peer's retained remote-memory key. Origins are the bridge from core
numeric IDs to host/native identities; capacity, product name, ordinal, or rate
similarity is never used as a selector (`probe/src/engine.rs:846-899`,
`probe/src/model.rs:425-471`).

The topology contains local RAM, disk, and GPU-memory devices plus one remote
RAM device per measured peer. Device properties are measured. A disk's topology
transfer rate is the lower of measured read and write rates; a GPU carries its
measured calculation rate. Storage, GPU, and peer RAM each produce opposing
directed links. A full-duplex pair gets distinct capacity resources; a
half-duplex pair shares one resource. Link bandwidth and inflight limits are
measured, with descriptor lane limits supplying the values
(`probe/src/engine.rs:901-1125`).

The discovery profile mirrors every topology device and link. RAM and remote
RAM use their measured transfer rates and lane limits. Storage uses the lower
read/write rate and lane limit. GPU discovery carries the native target and all
calculation capability fields, including queue ceiling, concurrent-task limit,
subgroup/workgroup lanes, shared memory, and transfer overlap. Every discovered
object is marked available; asynchronous flags come from the descriptor. The
discovery profile points back to the topology identity
(`probe/src/engine.rs:1135-1267`).

## Local Linux implementation

`LocalSystemDiscovery` is the concrete `HostDiscovery` used by the CLI. Its
default roots are `/proc`, `/sys`, and `/etc`; `with_benchmark_roots` only
changes the ordered directories considered for placing temporary disk probe
files. A benchmark root never declares device identity, capacity, transport,
or production rate (`probe/src/local.rs:22-93`).

Machine identity is read from `/proc/sys/kernel/hostname`, the first usable
value of `/etc/machine-id` or `/sys/class/dmi/id/product_uuid`, and
`/proc/sys/kernel/osrelease`. Firmware joins any usable BIOS vendor, version,
and date values with `|`, or uses `firmware-unreported`. Empty required files
and missing stable identity are errors (`probe/src/local.rs:42-78,602-633`).

RAM discovery first enumerates numeric `/sys/devices/system/node/nodeN`
directories and parses each `meminfo` `MemTotal` in KiB, converting with a
checked multiply by 1024. If no NUMA node is available it reads `/proc/meminfo`
and emits `memory0`. RAM keys are node names or `memory0`; link identities are
`memory-link:<key>` and each domain starts with one inflight lane
(`probe/src/local.rs:95-143,584-600`).

Storage discovery parses `/proc/self/mountinfo`, follows `/sys/dev/block/<major>:<minor>`,
walks partition parents to a physical block device, and reads its sector count
and `dev` number. Zero-capacity mounts are ignored. Entries sharing a physical
path are grouped; inconsistent capacity or transport for one identity is an
error. Transport is inferred from the physical path (`/nvme` gives NVMe,
`/sas` gives SAS, otherwise SATA), with NVMe/SAS full duplex and SATA half
duplex. The stable domain key is `block:<physical major:minor>`, and all
storage domains reference the first discovered RAM key, one read lane, one
write lane, and asynchronous submission (`probe/src/local.rs:146-257`).

For each physical disk, `select_storage_benchmark_root` tries caller-provided
roots, `$HOME`, the current directory, and every mount path. It canonicalizes
and deduplicates candidates, requires a directory on the matching major/minor,
and probes a private temporary filename for write access. No suitable writable
directory is a discovery error. Access-test files are removed immediately
(`probe/src/local.rs:345-428`).

Network discovery enumerates `/sys/class/net`, excludes `lo`, and keys each
interface as `net:<ifindex>:<MAC>`. It reads the interface address, driver
symlink, and optional firmware data. Wireless interfaces are WLAN and half
duplex; all others are Ethernet and full duplex. Missing optional driver or
firmware surfaces become explicit `*-unreported` labels; required directory,
address, and ifindex failures are errors (`probe/src/local.rs:259-307`).

`LocalHostBenchmarks` rejects an unbounded plan. RAM allocates a source filled
with `0xa5`, copies to a destination for at most the iteration and duration
bounds, and computes bytes per second from completed iterations and elapsed
nanoseconds. Capacity is the discovered hint, marked measured. Storage creates
a unique `.recipe-probe-<pid>-<nonce>.tmp`, writes and `sync_data`s the bounded
buffer, reads it back, then measures available filesystem bytes with `statvfs`.
It validates elapsed work and removes the temporary file on drop. A zero
iteration or zero elapsed duration, overflow, or invalid rate is a benchmark
error (`probe/src/local.rs:430-582`).

## Native GPU boundary

`recipe-native-probe` implements the `GpuDiscovery` and `GpuBenchmarkIo` traits
for the production CLI. `NativeGpuProbe::new` validates a nonzero Recipe-owned
FMA chain and an absolute kernel scratch directory, then constructs CUDA and
ROCr/HSA backends. It marks this combined inventory exhaustive. The
CUDA-only and HSA-only diagnostic constructors intentionally mark inventories
non-exhaustive, so they cannot produce an accepted profile
(`native-probe/src/native.rs:31-118,221-275`).

`discover_all` asks every enabled backend for descriptors, merges and sorts
them by key, and rejects duplicate keys. `benchmark_gpu` first requires a
bounded plan, rediscoveries each backend, requires exactly one backend to emit a
descriptor equal to the originally discovered descriptor, and then delegates
to that backend. A GPU that disappears, changes identity, or is claimed by two
backends is an error rather than an absent device (`native-probe/src/native.rs:245-290`).

The CUDA and HSA backends derive descriptor identities from native driver/runtime
surfaces, PCI link/firmware data, targets, toolchains, queue and execution
capabilities. Their bounded benchmark paths allocate host/device buffers,
measure host-to-device, device-to-host, and device-memory transfers, compile and
launch a Recipe-owned FMA kernel through the pinned offline toolchain, verify
the output, and return all fields as measured properties. A bounded allocation
larger than discovered device capacity or any identity/runtime/kernel
verification failure is fatal (`native-probe/src/cuda.rs:236-466`,
`native-probe/src/hsa.rs:296-628`).

## Peer transport boundary

`recipe-transport::TcpPeerSession` is the concrete `PeerSession`. Its descriptor
is returned unchanged. `benchmark_controlled` runs a framed begin exchange,
ready exchange, full-duplex simultaneous or half-duplex serialized directional
transfers, and a completion exchange. It checks plan, duplex, payload bounds,
sequence, endpoint identity, protocol version, checksums, deadlines, and
cancellation. Per-frame timings are accumulated into the evidence structure,
and rates are derived from total bytes and elapsed duration. A transport failure
is returned as structured phase/kind evidence and then rejected by the engine
(`transport/src/probe.rs:60-337,395-520`).

## Profile validation invariants

`validate_profile` is the final gate used both by the engine and the codec. It
requires:

- profile and cache schema 7, a nonzero cache digest, and benchmark metadata
  with seed schema 1 and four bounded, canonically representable plans;
- protocol-1 peer evidence with nonzero, distinct authenticated endpoints;
- exact network byte counts and iteration counts, elapsed-time bounds, ordered
  sample statistics, and rates derivable from evidence;
- valid core topology and scheduling properties, and a discovery profile that
  validates against that topology;
- exactly one machine origin for every topology machine, matching the machine
  name and with unique stable machine IDs;
- exactly one correctly typed RAM, disk, and GPU origin for every corresponding
  topology device, with each origin key unique within its machine;
- strict increasing canonical order for origins, peer records, topology
  machines/nodes/devices/links and node device lists, and discovery devices/
  links;
- `Measured` provenance for every persisted capacity, transfer rate, FLOP rate,
  bandwidth, and concurrency property;
- exact topology/discovery equality for device and link measured properties;
- one opposing pair per cross-machine transport, with peer benchmark count,
  directional rates, endpoint-pair uniqueness, and full/half duplex resource
  shape matching the evidence.

The checks are implemented in `probe/src/codec.rs:121-156,159-280,282-796`.
They reject unknown, duplicate, missing, mis-typed, non-canonical, estimated,
or contradictory profile data. There is no profile-level recovery path.

## Exact inventory resolution

`MeasuredProfile::resolve_local_inventory(host, gpus)` first revalidates the
profile and requires an exhaustive current GPU inventory. It finds the local
machine only by exact `MachineFingerprint` equality. It then builds expected
RAM, storage, and GPU maps from retained origins and compares them with the
live key maps. Missing keys, unexpected newly visible keys, duplicate live
keys, a wrong device kind, an absent host-memory relationship, a missing GPU
calculation capability, or a changed GPU target returns `InvalidProfile`.

Capacity, product name, ordinal, and benchmark similarity are deliberately not
fallback selectors. The returned `ResolvedLocalInventory` borrows the live
domains/descriptors and exposes the matched machine ID plus resolved RAM,
storage, and GPU records (`probe/src/resolve.rs:52-114,117-297`). This is the
only supported bridge from a cached profile's numeric topology IDs back to the
current native inventory.

## Codec and persisted representation

`MeasuredProfileCodec::encode` validates first, then emits a canonical little-
endian binary payload followed by a SHA-256 checksum. The payload begins with
the 16-byte `RECIPEPROFILE` magic and codec schema 7, then profile schema,
cache schema/digest, origins, benchmark metadata, peer benchmarks/evidence,
topology, and discovery. Labels carry a length and UTF-8 bytes; properties
carry their numeric value plus provenance tags; transport, duplex, node role,
device kind, calculation presence, and boolean values use fixed tags
(`probe/src/codec.rs:25-119,798-1267`).

Decode rejects payloads larger than 256 MiB, truncated framing, checksum or
magic mismatch, unsupported codec schema, invalid tags, invalid UTF-8/labels,
lengths over one million items or 1 MiB labels, trailing bytes, and invalid
numeric wrappers. It reconstructs the profile and runs the complete validator
before returning it. The encoder applies the same 256 MiB limit and item/string
limits (`probe/src/codec.rs:41-95,1282-1502`).

The codec is intentionally the persistence boundary. Cluster assembly uses it
for member identity derivation and cluster profile exchange, and preparation
uses `validate_profile` before planning. A profile cannot be made acceptable by
changing only its serialized bytes.

## Secure single-file cache

`ExplicitPathProfileCache::new` requires an absolute path naming a file. It has
no default location and no directory-wide search. Before load or store it
requires the parent to be a canonical, real directory owned by the effective
user with no group/other permission bits. Existing targets must be regular
non-symlink files, have private permission bits, and be owned by the effective
user. The file is opened with `O_NOFOLLOW | O_CLOEXEC`, and device/inode are
compared before reading to detect replacement races. Size is bounded before
decode, and the decoded profile's cache identity must equal the requested
identity (`probe/src/cache.rs:25-139`).

Store encodes and validates the profile, treats an existing byte-equivalent
profile as success, and rejects an existing different profile. Otherwise it
creates a same-directory private temporary file with a process/atomic nonce,
writes and syncs it, installs it with `renameat(..., NOREPLACE)`, handles a
concurrent identical writer as success, and syncs the directory. Temporary
files are removed on every uncommitted drop. Existing data is never replaced
(`probe/src/cache.rs:142-249`).

`ProbeEngine::current_cache_identity` runs discovery and validation only. The
`load_or_probe_and_store` method computes that identity, attempts an exact cache
load, and probes/stores only when the cache returns `None`. A malformed,
insecure, stale, or different existing file is an error, not a cache miss.
`probe_and_store` always performs a fresh full probe and stores only its
fully-validated profile (`probe/src/engine.rs:203-238`).

## Errors

All public operations return `ProbeResult<T>`. `ProbeError` is non-exhaustive
and currently has these categories: line-aware contract errors, discovery
errors, benchmark errors, non-exhaustive GPU enumeration, missing measured
properties, incomplete peer direction, invalid measured profile, cache errors,
and path-qualified I/O errors. Display output preserves the category and,
where applicable, contract line, peer/direction, operation, and path
(`probe/src/error.rs:4-85`). Callers should propagate the category rather than
turning a failed transition into an estimated value or an absent device.

## End-to-end callers and role in Recipe

### `recipe probe`

The production CLI accepts `--contract`, `--profile`, native library/toolchain
paths, and otherwise loads the checked-in seed. It requires bare-metal mode,
creates private state and scratch directories, discovers the host, derives the
host RAM key for native configuration, constructs `NativeGpuProbe`, and builds
`ProbeEngine` with `LocalSystemDiscovery`, `NativeGpuProbe`, and
`LocalHostBenchmarks`. The current CLI passes no peer sessions. It computes the
identity before choosing an identity-named default path
`measured-v<schema>-<digest>.recipe-profile`, then calls
`load_or_probe_and_store`. On success it writes the active native receipt and
prints profile path, source, cache/topology/discovery identities, and counts
(`src/cli.rs:841-947`).

`current_native_inputs` repeats host discovery and uses the active receipt when
present, checking that its retained host RAM key still exists. Without a
receipt it constructs the same default native configuration and computes the
current identity to derive the profile path. It never selects a newest or
ordinally named profile (`src/cli.rs:949-1009`).

### Native preparation, training, and inference

`src/native_prepare.rs` loads an identity-named profile through
`ExplicitPathProfileCache`, validates its filename and exact cache identity,
and reopens native GPU inventory through `resolve_local_inventory`. It creates
owned host and target plans only after every local origin matches. The
`with_current_native_preparation` path loads the exact current profile, keeps
one `NativeGpuProbe` in thread-local state, rejects a changed native
configuration, and lends scoped CUDA/HSA bindings to one callback
(`src/native_prepare.rs:250-410`).

`Train::run` and native inference call that callback. They use the measured
profile's topology/discovery for preparation, derive runtime tuning from
measured capacities and lane limits, and pass the profile to
`recipe-prepare::Preparer`. The preparation boundary calls `validate_profile`
before planning and rejects the theoretical seed or any unmeasured/invalid
profile (`prepare/src/lib.rs:320-360`, `src/training.rs`, `src/inference.rs`).
The probe profile therefore supplies immutable measured facts and native
identity; it does not own scheduling, compilation, execution, or training
state.

### Peer transport and cluster assembly

`recipe-transport` implements the peer session used by the engine when a caller
has an established authenticated TCP connection. `recipe-cluster` accepts
measured member profiles, derives member endpoint identity from canonical codec
bytes and retained stable machine identity, validates the profile, remaps
member-local numeric IDs, and uses the canonical codec for cluster profile
serialization. It preserves peer benchmark evidence and builds cross-machine
links from measured rates, duplex, lane, and retained origin data
(`cluster/src/model.rs:1-70`, `cluster/src/assemble.rs:1-171,788-795`).

The root facade reexports the complete crate as `recipe::probe`, so library
callers can use the same traits and validators rather than a parallel discovery
API (`src/facade.rs:36`).

## What this crate does not do

- It does not trust seed estimates as production performance, enumerate devices
  from user declarations, or infer identity from capacity/rate similarity.
- It does not accept a partial host inventory, a non-exhaustive GPU inventory,
  a missing peer direction, or estimated profile properties.
- It does not own native CUDA/HSA handles, compile the production model graph,
  schedule Recipe tasks, allocate final execution resources, or run training.
  Those responsibilities remain in `recipe-native-probe`, preparation, and
  executor crates.
- It does not silently treat a changed device/runtime/toolchain, malformed cache,
  stale identity, failed benchmark, or invalid profile as a cache miss or
  fallback device.
- The default CLI path measures the local host and native GPUs only. Peer
  measurements are available through the explicit `PeerSession` API and are
  included only when a caller supplies established sessions.

The resulting measured profile is the handoff: discovery proves the current
hardware and stable origins, bounded backends provide measured capacities and
rates, codec validation makes the facts canonical and persistent, and exact
resolution reopens those same origins for preparation and execution.
