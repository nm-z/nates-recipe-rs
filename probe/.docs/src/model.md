# Probe model: discovery, measurement, and measured profiles

`probe/src/model.rs` is the data and trait boundary for Recipe's bare-metal
probe. It does not read hardware, choose benchmark sizes, build topology IDs,
serialize bytes, or write a cache. Those operations live in the engine, local
and native adapters, the peer transport, the codec, and the cache. The model
types make the boundaries explicit:

1. discovery adapters return an exhaustive *current inventory*;
2. benchmark adapters return `Property<T>` values whose provenance is
   explicitly `Measured`;
3. `ProbeEngine` combines both with a seed contract into one immutable
   `MeasuredProfile`;
4. the profile carries the paired `recipe_core::Topology` and
   `recipe_core::DiscoveryProfile`, plus the stable-origin table needed to
   resolve those numeric IDs back to current hardware; and
5. the canonical codec and `ProfileCache` make the profile reusable only when
   its schema, checksum, identity, topology, discovery data, and measured
   evidence still agree.

The public model is re-exported by `probe/src/lib.rs`. The source links below
point to the implementation that actually constructs or consumes each value.

## Version constants and provenance

`PROFILE_SCHEMA` is `7`. It versions the semantic measured profile and is the
schema stored in `MeasuredProfile.schema` and `CacheIdentity.schema`.
`PROFILE_CODEC_SCHEMA` is also `7`; it versions the binary framing and field
order. A codec version change is independent of the semantic schema even though
the current values are equal. `PEER_BENCHMARK_PROTOCOL_SCHEMA` is `1`; it is
stored in peer evidence and in structured peer failures. These constants are at
the top of [`model.rs`](../../src/model.rs#L17-L19).

All resource values use `recipe_core` newtypes. `ByteCount` can be zero,
whereas `BytesPerSecond`, `FlopsPerSecond`, and `TransferLaneCount` reject zero
at construction. `Label` rejects empty or whitespace-only strings, and
`Digest` has an explicit all-zero value. A `Property<T>` retains both the value
and `PropertyProvenance`; only `Measured` or `Override` values are schedulable,
but a persisted probe profile is stricter and requires `Measured` everywhere.
The core definitions are in [`topology.rs`](../../../core/src/topology.rs#L10-L36),
[`units.rs`](../../../core/src/units.rs#L3-L89), and
[`identity.rs`](../../../core/src/identity.rs#L3-L51).

The plain inventory, descriptor, measurement, origin, metadata, and profile
structures in this module have no associated `new` constructor. Callers create
them with struct literals after their adapter-specific validation. The model's
associated constructors are the small control and failure constructors listed
below: `PeerBenchmarkControl::new`, `PeerBenchmarkControl::for_plan`, and
`PeerBenchmarkFailure::new`. `PeerBenchmarkCancellation` uses its derived
`Default`; `PeerBenchmarkAttempt` and `PeerDuplexExecution` are constructed by
selecting their enum variants. `ProbeEngine`, `ExplicitPathProfileCache`, and
the native or transport adapters construct these model values outside this
file, then the engine and codec enforce the cross-field invariants.

## Current discovery representations

These structures describe what adapters found *now*. They are not persisted
profiles and they do not contain topology IDs. The stable textual keys are the
bridge that `MeasuredOrigins` later records.

### `MachineFingerprint`

[`MachineFingerprint`](../../src/model.rs#L21-L27) identifies one machine for
cache invalidation and profile resolution:

| Field | Meaning and use |
| --- | --- |
| `hostname: Label` | Human-readable machine name. It becomes the local `Topology::machines[].name` and must agree with the persisted origin hostname. |
| `stable_id: Label` | Machine identity used to distinguish machines and to reject resolving a profile on another host. It is hashed into the probe cache identity. |
| `runtime_abi: Label` | Operating-system or runtime ABI identity. It participates in the cache key but is not a topology display name. |
| `firmware: Label` | Firmware identity. It participates in the cache key and is retained in the origin. |

`LocalSystemDiscovery::discover_host` reads these values from procfs, machine-id
or DMI, and DMI firmware files. A missing firmware surface is represented by a
nonempty `firmware-unreported` label, not by an empty label
([`local.rs`](../../src/local.rs#L42-L78)). Peer descriptors carry the same
shape for the remote machine.

### RAM, storage, network, and duplex

[`RamDomain`](../../src/model.rs#L29-L35) is one locally usable RAM domain:

| Field | Meaning |
| --- | --- |
| `key` | Stable discovery key. It is the only selector used later to reassociate a live RAM domain with a topology `DeviceId`. |
| `capacity_hint` | Discovery hint used to size a bounded RAM benchmark. It is not a production schedule property. |
| `link_identity` | Stable link identity for cache invalidation and diagnostics. |
| `maximum_inflight_transfers` | Adapter-reported transfer lane bound. The engine copies it into measured topology/discovery concurrency. |

[`LinkDuplex`](../../src/model.rs#L37-L41) has exactly `Full` and `Half`.
`ProbeEngine` maps these to core `DuplexMode`; a full-duplex pair gets two
capacity resources, while a half-duplex pair shares one
([`engine.rs`](../../src/engine.rs#L1080-L1133)).

[`StorageDomain`](../../src/model.rs#L43-L58) describes a mounted physical
storage domain:

| Field | Meaning |
| --- | --- |
| `key` | Stable block-domain key, used for origins and cache identity. |
| `name` | Display and discovery identity, also hashed. |
| `benchmark_root: PathBuf` | Existing directory on this physical device where the bounded temporary benchmark file is created. It is operational input, not a declared capacity or rate. |
| `capacity_hint` | Seed-sized capacity hint; the benchmark later measures available capacity. |
| `host_memory_key` | RAM key through which the disk is connected. The engine rejects an unknown key. |
| `driver`, `firmware` | Driver and firmware identities, both cache-key inputs. |
| `link_identity` | Stable physical-link identity, used in cache invalidation. |
| `transport_kind` | Core transport family, for example `Sata`, `Sas`, or `Nvme`. |
| `duplex` | Physical half/full mode. The core topology validator checks it against the transport family. |
| `maximum_concurrent_reads`, `maximum_concurrent_writes` | Direction-specific lane bounds. Topology links retain the respective direction. Discovery conservatively uses the smaller bound for a bidirectional device capability. |
| `asynchronous_submission` | Required to be true by engine normalization and core discovery validation. |

[`NetworkInterface`](../../src/model.rs#L60-L71) is the local side of a peer
transport. It has `key`, display `name`, `address`, `driver`, `firmware`, and
`link_identity` labels, plus `transport_kind`, `duplex`, and
`asynchronous_submission`. Network interfaces are sorted and uniqueness-checked
for cache identity, but they do not become topology devices unless an explicit
`PeerSession` uses one.

[`HostInventory`](../../src/model.rs#L73-L79) groups one local
`MachineFingerprint` with `ram`, `storage`, and `network` vectors. The
`HostDiscovery` trait requires `discover_host() -> ProbeResult<HostInventory>`
and documents the all-or-error rule: every locally usable RAM, mounted storage,
and network domain must be enumerated, or the adapter returns an error
([`model.rs`](../../src/model.rs#L81-L85)). `ProbeEngine::inspect` sorts the
three vectors by key, rejects empty RAM or storage, rejects duplicate keys,
checks every storage memory reference and asynchronous flag, and then uses the
normalized inventory for both identity hashing and measurement
([`engine.rs`](../../src/engine.rs#L163-L200),
[`engine.rs`](../../src/engine.rs#L286-L327)).

The production `LocalSystemDiscovery` implementation reads procfs and sysfs,
groups mounted partitions by physical block device, excludes loopback, and
sets asynchronous submission true for the discovered storage and network
domains. Its RAM fallback is one `memory0` domain when NUMA nodes are absent
([`local.rs`](../../src/local.rs#L95-L143),
[`local.rs`](../../src/local.rs#L146-L257),
[`local.rs`](../../src/local.rs#L259-L307)).

### GPU discovery

[`GpuDescriptor`](../../src/model.rs#L87-L110) is the complete identity and
capability descriptor supplied by the native calculation backend:

| Field | Meaning |
| --- | --- |
| `key` | Stable GPU key. It is the origin selector and cache-key component. |
| `name` | Display name and cache-key component. |
| `host_memory_key` | Local RAM domain connected to the GPU. It must resolve. |
| `target: TargetIdentity` | Backend, architecture, and ABI used to compile native calculations. It is retained in discovery capability and checked again before native preparation. |
| `capacity_hint` | Seed-sized VRAM hint for the bounded GPU benchmark. |
| `driver`, `runtime_abi`, `firmware`, `link_identity` | Exact driver/runtime/firmware/link identities used for cache invalidation and diagnostics. |
| `transport_kind` | Usually `Pcie` for the native backends, but modeled as the core transport family. |
| `toolchain` | Exact compiler/toolchain name, version, and digest. It is hashed into the cache identity because a changed toolchain invalidates native preparation. |
| `duplex` | Host-to-device and device-to-host link mode. |
| `host_to_device_maximum_inflight`, `device_to_host_maximum_inflight` | Directional transfer lane bounds. |
| `asynchronous_submission` | Required to be true for accepted GPU discovery. |
| `maximum_submission_queues` | Native submission queue capacity, required nonzero. |
| `maximum_concurrent_tasks` | Maximum active GPU calculations, required nonzero. |
| `subgroup_lanes` | Native SIMD/SIMT subgroup width. Core discovery requires a nonzero power of two. |
| `maximum_workgroup_lanes` | Largest native workgroup; it must contain at least one subgroup. |
| `maximum_shared_memory_per_workgroup` | Nonzero workgroup-local/shared-memory limit. |
| `transfer_overlaps_calculation` | Whether transfer capability may overlap calculation. |

[`GpuInventory`](../../src/model.rs#L112-L116) carries an `exhaustive` proof
and the device vector. `GpuDiscovery::discover_all()` must return this whole
inventory ([`model.rs`](../../src/model.rs#L118-L122)). The engine refuses a
false `exhaustive` flag, an empty device vector, duplicate keys, missing host
RAM references, synchronous submission, zero queues, or zero concurrent tasks
([`engine.rs`](../../src/engine.rs#L168-L182),
[`engine.rs`](../../src/engine.rs#L330-L358)).

`native-probe::NativeGpuProbe` is the production implementation. Its normal
constructor combines CUDA and HSA backends and marks the inventory exhaustive;
the CUDA-only and HSA-only diagnostic constructors deliberately mark it
non-exhaustive and therefore cannot produce an accepted profile. It sorts
backend output and rejects duplicate keys. For measurement it rediscovers the
exact descriptor on each backend, rejects zero, changed, or multiply claimed
devices, then delegates to the owning backend
([`native.rs`](../../../native-probe/src/native.rs#L33-L86),
[`native.rs`](../../../native-probe/src/native.rs#L245-L298)).

## Bounded plans and local measurements

[`BoundedBenchmarkPlan`](../../src/model.rs#L124-L135) is a copyable request
with three fields:

| Field | Meaning |
| --- | --- |
| `buffer_bytes: ByteCount` | Payload or benchmark buffer size. |
| `iterations: u32` | Maximum requested iterations or samples. |
| `maximum_duration: Duration` | Absolute wall-clock bound for the attempt. |

`is_bounded()` returns true exactly when all three values are nonzero. It does
not enforce the native transport's additional iteration, payload, or total-byte
limits. `ProbeEngine::BenchmarkPlans::from_seed` derives one plan each for RAM,
storage, GPU, and network from seed estimates, clamps each suggested buffer to
the engine's minimum and maximum, and fixes the iteration and duration bounds.
The four plans are copied into `BenchmarkMetadata` so the persisted profile
records what was actually requested ([`engine.rs`](../../src/engine.rs#L241-L276)).

[`RamMeasurement`](../../src/model.rs#L138-L142) has measured `capacity` and
`transfer_rate`. [`StorageMeasurement`](../../src/model.rs#L144-L149) has
measured `capacity`, `read_rate`, and `write_rate`. [`GpuMeasurement`](../../src/model.rs#L161-L168)
has measured `capacity`, calculation FLOP rate, device-memory rate, and
host-to-device and device-to-host rates. `HostBenchmarkIo` receives a domain
and a plan and returns the corresponding measurement; `GpuBenchmarkIo` receives
a descriptor and a plan and returns a GPU measurement
([`model.rs`](../../src/model.rs#L151-L173)).

`ProbeEngine::probe` calls all local and GPU benchmark hooks after inspection.
Every returned property must have `PropertyProvenance::Measured`; estimated or
override values return `ProbeError::MissingMeasurement`. The local benchmark
implementation copies RAM buffers, writes and reads a temporary storage file,
and derives rates from observed elapsed time. A zero-iteration or zero-duration
result is a benchmark error, not an estimated fallback
([`engine.rs`](../../src/engine.rs#L429-L461),
[`local.rs`](../../src/local.rs#L430-L507),
[`local.rs`](../../src/local.rs#L557-L580)). Native GPU backends run their own
bounded native kernels and return the same `GpuMeasurement` shape.

## Peer session model

Peer types model one already-established transport session. Discovery does not
invent peers from network interfaces.

### Descriptor and measurement

[`PeerDescriptor`](../../src/model.rs#L175-L191) contains:

| Field | Meaning |
| --- | --- |
| `session_id` | Stable session key and persisted benchmark record key. |
| `machine` | Fingerprint of the remote machine. A peer may not resolve to the local machine. |
| `remote_memory_key` | Stable key for the remote RAM domain measured through this session. |
| `local_memory_key` | Local RAM domain used by the session. It must exist in `HostInventory`. |
| `local_interface_key` | Local network interface key. It must exist and agree on transport and duplex. |
| `remote_interface_identity`, `remote_driver`, `remote_firmware` | Remote link, driver, and firmware identities. All are cache-key inputs. |
| `link_identity` | Stable link identity for cache invalidation. |
| `transport_kind`, `duplex` | Physical transport and full/half mode. |
| `outbound_maximum_inflight`, `inbound_maximum_inflight` | Directional remote transfer lane limits. |
| `asynchronous_submission` | Required true by engine validation and transport construction. |

[`PeerMeasurement`](../../src/model.rs#L193-L200) combines measured remote
memory capacity and rate, optional measured outbound and inbound throughput,
and authenticated `PeerBenchmarkEvidence`. The options are deliberate: an
adapter may report an incomplete attempt, but `ProbeEngine` refuses either
direction missing with `ProbeError::IncompletePeerMeasurement` before profile
construction.

### Cancellation, deadlines, and structured failure

[`PeerBenchmarkCancellation`](../../src/model.rs#L202-L216) is a cloneable
`Arc<AtomicBool>`. `cancel()` stores true with release ordering and
`is_cancelled()` loads it with acquire ordering. It is advisory between framed
operations; the absolute deadline remains the hard bound.

[`PeerBenchmarkControl`](../../src/model.rs#L218-L274) contains a private
`absolute_deadline: Instant` and the shared cancellation token. Its constructor
and accessors are:

| Method | Behavior |
| --- | --- |
| `new(absolute_deadline, cancellation)` | Const constructor, no validation. |
| `for_plan(plan)` | Starts a fresh control at `Instant::now() + plan.maximum_duration`; returns `ProbeError::Benchmark` if `Instant::checked_add` overflows. It does not itself reject an unbounded plan. |
| `absolute_deadline()` | Returns the fixed deadline. |
| `cancellation()` | Returns the shared cancellation handle. |
| `failure(phase)` | Returns `Cancelled` first if cancellation is set, then `Deadline` when now is at or past the deadline, otherwise `None`. The supplied phase is retained in the returned failure. |

[`PeerBenchmarkPhase`](../../src/model.rs#L276-L283) records one of
`Validation`, `BeginExchange`, `ReadyExchange`, `DirectionalTransfer`, or
`CompletionExchange`. [`PeerBenchmarkFailureKind`](../../src/model.rs#L285-L293)
classifies terminal evidence as `Cancelled`, `Deadline`, `Identity`,
`Integrity`, `Protocol`, or `Transport`.

[`PeerBenchmarkFailure`](../../src/model.rs#L295-L313) stores the peer protocol
schema, phase, kind, and human-readable detail. `new` stamps
`PEER_BENCHMARK_PROTOCOL_SCHEMA` and converts any `Into<String>` detail.
[`PeerBenchmarkAttempt`](../../src/model.rs#L316-L340) is either
`Measured(PeerMeasurement)` or `Failed(PeerBenchmarkFailure)`. Its
`into_measurement()` passes through only the measured variant. A failed attempt
becomes `ProbeError::Benchmark` containing phase, kind, and detail, so failure
evidence cannot masquerade as a measured property.

### Duplex and authenticated evidence

[`PeerDuplexExecution`](../../src/model.rs#L342-L346) distinguishes
`Simultaneous` full-duplex transfer from `Serialized` half-duplex transfer.
[`PeerEndpointEvidence`](../../src/model.rs#L348-L352) contains a machine digest
and a complete profile digest for each endpoint. Both digests must be nonzero
and the two machine digests must differ.

[`DirectionalBenchmarkEvidence`](../../src/model.rs#L354-L363) records one
direction's accounting:

| Field | Meaning and validation |
| --- | --- |
| `total_bytes` | Must equal `plan.buffer_bytes * plan.iterations`. |
| `elapsed_nanoseconds` | Nonzero and no greater than the plan duration. |
| `sample_count` | Exactly the plan iteration count. |
| `minimum_sample_nanoseconds`, `maximum_sample_nanoseconds`, `mean_sample_nanoseconds` | Nonzero ordered statistics, with `min <= mean <= max <= elapsed`. |
| `variance_nanoseconds_squared` | u128 variance retained for evidence and digesting. It is not used to derive the rate. |

[`PeerBenchmarkEvidence`](../../src/model.rs#L365-L373) combines protocol
schema, local and remote endpoint evidence, execution mode, and outbound and
inbound directional evidence. [`MeasuredPeerBenchmark`](../../src/model.rs#L375-L381)
is the persisted projection: session ID, measured outbound and inbound rates,
and the evidence. It intentionally does not persist the mutable descriptor.

`PeerSession` requires `descriptor()` and the simple `benchmark(plan)` method.
Its default `benchmark_controlled` checks cancellation/deadline at validation,
calls `benchmark`, checks again at completion, and maps any returned
`ProbeError` to a transport failure in the directional-transfer phase. A
blocking implementation is expected to override it so every I/O operation gets
the supplied absolute deadline ([`model.rs`](../../src/model.rs#L383-L417)).
`transport::TcpPeerSession` does override it. It authenticates both endpoints,
checks begin/ready/complete frames, uses concurrent transfer for full duplex
and serialized transfer for half duplex, computes the statistics above, and
returns measured properties and evidence
([`transport/src/probe.rs`](../../../transport/src/probe.rs#L93-L330),
[`transport/src/probe.rs`](../../../transport/src/probe.rs#L351-L418),
[`transport/src/probe.rs`](../../../transport/src/probe.rs#L530-L556)).

## Persisted identity and origins

[`CacheIdentity`](../../src/model.rs#L419-L423) is an ordered pair of the
semantic cache `schema` and a 256-bit `digest`. The engine's
`build_cache_identity` hashes the seed schema and all seed estimates,
reservation, invalidation facets, and transport seeds, then the normalized
machine fingerprint, every RAM/storage/network/GPU descriptor field that can
change hardware or native compilation, and every peer descriptor field. It
does not contain benchmark results. Therefore discovery-only identity can be
computed before spending time on measurements and is the exact key used by
`load_or_probe_and_store` ([`engine.rs`](../../src/engine.rs#L203-L237),
[`engine.rs`](../../src/engine.rs#L575-L725)).

The origin records retain the stable discovery identity that core topology
objects intentionally omit:

| Type | Fields and source |
| --- | --- |
| `MeasuredMachineOrigin` | `machine: MachineId` plus the complete `MachineFingerprint`. The topology machine name must equal `fingerprint.hostname`, and each stable machine ID must be unique. |
| `MeasuredRamOrigin` | `device: DeviceId` plus a RAM `key`, from `RamDomain::key` or a peer's `remote_memory_key`. The key is scoped by the device's machine, never guessed from capacity or rate. |
| `MeasuredStorageOrigin` | `device: DeviceId` plus the exact `StorageDomain::key`, scoped by machine. |
| `MeasuredGpuOrigin` | `device: DeviceId` plus the exact `GpuDescriptor::key`, scoped by machine. |

These definitions and their identity comments are in
[`model.rs`](../../src/model.rs#L425-L461). [`MeasuredOrigins`](../../src/model.rs#L463-L471)
groups the four vectors. `ProbeEngine::build_origins` emits one local machine,
one origin per peer machine, local RAM origins followed by peer RAM origins,
then storage and GPU origins. The vectors are deterministic because inspection
sorts local keys and peer sessions first
([`engine.rs`](../../src/engine.rs#L846-L899)).

## `MeasuredProfile`, paired core representations, and metadata

[`MeasuredProfile`](../../src/model.rs#L473-L482) is the complete immutable
output of a successful probe:

| Field | Role |
| --- | --- |
| `schema` | Must equal `PROFILE_SCHEMA`. |
| `cache_identity` | Discovery-only identity used for exact cache selection. |
| `origins` | Stable machine and domain keys that map current inventory back to numeric core IDs. |
| `benchmarks` | The four bounded plans and seed schema used to create this profile. |
| `peer_benchmarks` | Canonical authenticated network evidence and both measured directions for every cross-machine transport. |
| `topology: Topology` | Measured physical/logical graph used by planning and scheduling. |
| `discovery: DiscoveryProfile` | Measured per-device and per-link capability snapshot paired to the same topology identity. |

The two core representations are deliberately paired rather than duplicated
models with independent identities:

- `Topology` contains `identity`, ordered machines, nodes, devices, and
  directed links. A device has a machine, kind (`GpuMemory`, `Ram`, or `Disk`),
  measured capacity and transfer rate, and an optional measured calculation
  rate. A directed link has transport and duplex identities, endpoints,
  measured bandwidth and lane count, and a capacity resource. Core validation
  requires one master, owned devices, reverse link pairs, transport-consistent
  duplex resources, and nonzero identities
  ([`topology.rs`](../../../core/src/topology.rs#L46-L141),
  [`topology.rs`](../../../core/src/topology.rs#L143-L418)).
- `DiscoveryProfile` contains a separate `identity`, the `TopologyIdentity` it
  belongs to, and one `DiscoveredDevice` and `DiscoveredLink` capability record
  for every topology object. Device records preserve submission queues,
  availability, transfer capability, and optional GPU calculation capability;
  links preserve availability, bandwidth, lanes, and asynchronous submission.
  Validation requires exact topology identity, complete unique coverage,
  available objects, asynchronous operation, schedulable properties, and
  topology/discovery property equality
  ([`discovery.rs`](../../../core/src/discovery.rs#L34-L60),
  [`discovery.rs`](../../../core/src/discovery.rs#L62-L275)).
- `MeasuredOrigins` is the association layer. Numeric `MachineId` and
  `DeviceId` values are generated for this profile, while origins retain the
  stable keys needed to prove that a later live inventory is the same hardware.

`MeasuredProfile::is_cache_valid_for(current)` is a small, non-validating
predicate: it checks only `self.schema == PROFILE_SCHEMA` and equality of
`cache_identity`. The actual cache reader must decode and call full
`validate_profile` before returning a profile. The four lookup methods
`machine_origin`, `ram_origin`, `storage_origin`, and `gpu_origin` perform a
linear ID lookup in the corresponding origin vector and return `Option<&...>`;
they do not infer, synthesize, or fall back to another origin
([`model.rs`](../../src/model.rs#L484-L521)).

[`BenchmarkMetadata`](../../src/model.rs#L523-L530) records `seed_schema` and
the RAM, storage, GPU, and network `BoundedBenchmarkPlan` values. It preserves
probe provenance and allows codec validation to reject an unbounded or
non-canonical plan on load.

`ProfileCache` is the injection boundary for persistence. `load(identity)` may
return `None` for a missing file or one validated `MeasuredProfile`; `store`
accepts only a profile. The trait has no default path, no newest-file policy,
and no replacement semantics ([`model.rs`](../../src/model.rs#L532-L535)).

## How `ProbeEngine` constructs the paired profile

`ProbeEngine::new` stores references to exactly four adapters: host discovery,
GPU discovery, host benchmarks, and GPU benchmarks
([`engine.rs`](../../src/engine.rs#L31-L61)). Its real construction path is:

1. `inspect` calls `HostDiscovery::discover_host`, normalizes and validates the
   complete host inventory, calls `GpuDiscovery::discover_all`, requires an
   exhaustive nonempty GPU inventory, sorts GPU descriptors, validates their
   host-memory and asynchronous capabilities, obtains every peer descriptor,
   sorts sessions by `session_id`, validates local interface/memory and remote
   machine identity, then computes `CacheIdentity`.
2. `BenchmarkPlans::from_seed` creates four bounded plans. The engine benchmarks
   every RAM domain, storage domain, GPU descriptor, and peer session. Peer
   attempts use a fresh `PeerBenchmarkControl` and must become a measured
   `PeerMeasurement`; all measurement provenance and peer evidence are checked
   before insertion into the internal `Measurements` maps.
3. `build_profile_digest("recipe-topology-v6", ...)` hashes the cache digest,
   measured RAM/storage/GPU values, peer rates, and all peer evidence to form a
   `TopologyIdentity`. A second domain string, `recipe-discovery-v6`, produces
   the `DiscoveryIdentity` from the same measured input.
4. `assign_ids` gives the local machine ID `1`, peer machine IDs in sorted
   session order starting at `2`, and sequential device IDs for local RAM,
   local storage, local GPUs, then peer RAM. The master node is node `1`; peer
   nodes are workers starting at node `2`.
5. `build_topology` emits measured local RAM, disk, GPU-memory, and peer-RAM
   devices. Disk transfer rate is the measured minimum of read and write; GPU
   memory transfer is the measured memory rate. `LinkBuilder::push_pair` emits
   exactly two opposing directed links per storage, GPU, and peer transport,
   sharing a capacity resource for half duplex and using distinct resources for
   full duplex.
6. `build_discovery` emits one measured capability for every topology device
   and link. RAM, storage, GPU, and peer capabilities use the appropriate
   measured values. Storage and peer device concurrency is conservatively the
   minimum of opposing lane limits. GPU calculation capability copies target,
   rate, queue/concurrency, subgroup, workgroup, and shared-memory limits from
   the descriptor. Link asynchronous flags are paired in topology-link order.
7. The engine validates topology structure and scheduling properties, validates
   discovery against topology, constructs `MeasuredProfile`, converts each
   peer measurement to `MeasuredPeerBenchmark` (the optional directions are
   safe to unwrap only because validation just succeeded), and runs the full
   codec validator once more before returning.

The engine's implementation is in
[`engine.rs`](../../src/engine.rs#L63-L160),
[`engine.rs`](../../src/engine.rs#L799-L1053), and
[`engine.rs`](../../src/engine.rs#L1135-L1276). No seed estimate, descriptor
capacity hint, ordinal, product name, or benchmark similarity is used as a
post-probe identity selector.

## Resolution back to live hardware

`MeasuredProfile::resolve_local_inventory` is the only model-level
reassociation operation. It first runs full `validate_profile`, requires an
exhaustive current GPU inventory, finds the machine whose complete fingerprint
equals `host.machine`, and then resolves RAM, storage, and GPU by exact stable
keys. It returns `ResolvedLocalInventory` containing the profile's machine ID
and borrowed `ResolvedRamDomain`, `ResolvedStorageDomain`, and
`ResolvedGpuDevice` records, each retaining the profile ID plus a reference to
the current descriptor ([`resolve.rs`](../../src/resolve.rs#L10-L114)).

The resolver rejects every mismatch:

- duplicate live keys;
- missing or newly visible RAM, storage, or GPU keys;
- a profile origin that points to a missing topology device or wrong device
  kind;
- storage or GPU descriptors whose `host_memory_key` is absent from resolved
  RAM;
- a GPU whose measured discovery target differs from the current descriptor;
- a non-exhaustive GPU inventory; or
- a machine fingerprint with no exact origin.

There is no capacity, product-name, ordinal, performance, newest-cache, or
partial-inventory fallback. Native preparation calls this resolver before
opening CUDA/HSA bindings, and the preparation scope cannot outlive those
borrowed bindings ([`native_prepare.rs`](../../../src/native_prepare.rs#L248-L339),
[`bindings.rs`](../../../native-probe/src/bindings.rs#L120-L147)).

## Validation and error boundaries

### Engine-time validation

The engine reports adapter and measurement failures as `ProbeError` values. The
important model-facing variants are:

- `Discovery(String)` for malformed or incomplete host, GPU, or peer
  descriptors;
- `IncompleteGpuEnumeration` when `GpuInventory.exhaustive` is false;
- `MissingMeasurement(String)` when a local measurement is not `Measured`;
- `IncompletePeerMeasurement { peer, direction }` when an optional peer rate is
  absent;
- `Benchmark(String)` for deadline, transport, endpoint, duplex, or evidence
  inconsistency; and
- `InvalidProfile(String)` when constructed topology, discovery, or the final
  profile fails validation.

The enum and display forms are in [`error.rs`](../../src/error.rs#L4-L85).
Peer endpoint evidence must use protocol schema `1`, nonzero distinct machine
digests, and an execution mode matching `LinkDuplex`. For each direction the
engine independently checks byte count, duration, sample count, ordered sample
statistics, and the rate derived from bytes divided by elapsed nanoseconds
([`engine.rs`](../../src/engine.rs#L463-L573)).

### Canonical profile validation

`validate_profile` is the codec's semantic gate and is called by both encode and
decode. It requires:

1. profile and cache schemas equal `PROFILE_SCHEMA`, and a nonzero cache digest;
2. every benchmark metadata seed schema equal to `CONTRACT_SCHEMA`, every plan
   bounded, and every duration representable as canonical u64 nanoseconds;
3. peer protocol schema, endpoint identities, duplex execution, byte accounting,
   sample statistics, and duration-derived rates consistent with metadata;
4. topology structure and scheduling properties valid;
5. discovery valid for exactly that topology;
6. one origin for every topology machine, RAM device, storage device, and GPU
   device, with the correct kind, unique machine-scoped stable keys, and
   matching machine names;
7. strict increasing order for origins, peer benchmarks, topology vectors,
   node-device lists, and discovery vectors;
8. every persisted property provenance exactly `Measured`, never `Estimated` or
   `Override`;
9. exact equality between topology and discovery capability values; and
10. exactly one opposing pair for every cross-machine transport, with peer
    evidence matching its rates and full/half resource semantics.

These checks are implemented in
[`codec.rs`](../../src/codec.rs#L121-L156), with peer checks at
[`codec.rs`](../../src/codec.rs#L184-L280), origin checks at
[`codec.rs`](../../src/codec.rs#L282-L460), ordering and provenance checks at
[`codec.rs`](../../src/codec.rs#L462-L626), and topology/discovery/peer pairing
checks at [`codec.rs`](../../src/codec.rs#L628-L796). Any failure is a
`ProbeError::Cache` whose display prefix identifies the codec or profile
failure. A profile that merely parses is not accepted.

## Canonical serialization and cache consumers

`MeasuredProfileCodec` is a versioned canonical binary codec, not serde and not
a text format. `encode(profile)` validates first, emits the fields in one fixed
order, appends a SHA-256 checksum, and enforces a 256 MiB output limit. `decode`
checks size, framing, checksum, magic, codec schema, every typed field, and
trailing bytes, reconstructs `MeasuredProfile`, then calls `validate_profile`
before returning ([`codec.rs`](../../src/codec.rs#L25-L119)).

The byte order is little endian. The payload layout is:

1. 16-byte `RECIPEPROFILE` magic, codec schema, profile schema, cache schema,
   and 32-byte cache digest;
2. origin arrays for machines, RAM, storage, and GPU, including every
   fingerprint label and stable key;
3. benchmark seed schema and four plans, each as buffer bytes, iterations, and
   duration nanoseconds;
4. peer benchmark count, session labels, rate values plus provenance, endpoint
   digests, execution tag, and both directional evidence records;
5. topology identity and ordered machines, nodes, devices, and links; and
6. discovery identity, topology identity, ordered device capabilities and link
   capabilities, followed by a 32-byte SHA-256 checksum over all preceding
   bytes.

Lengths are u32 and limited to `MAXIMUM_ITEMS` (one million), labels are UTF-8
with a one MiB limit, and rates/lane counts are reconstructed through their
nonzero core constructors. Boolean, provenance, transport, duplex, node-role,
and device-kind tags reject unknown values. These bounds and primitive codecs
are in [`codec.rs`](../../src/codec.rs#L1281-L1502).

`ExplicitPathProfileCache` is the concrete `ProfileCache`. Its constructor
requires an absolute path naming a file. Before load or store it requires a
canonical, real, effective-user-owned parent directory with no group or other
permissions. Existing targets must be regular non-symlink files with the same
ownership and private permissions. Loading is bounded, uses `O_NOFOLLOW`,
checks the opened inode, decodes the canonical profile, and rejects a stale
`CacheIdentity`; a missing target returns `Ok(None)`. Storing encodes first,
returns success for an identical existing profile, rejects a different existing
profile, writes a private same-directory temporary file, fsyncs it, installs it
with `renameat(..., NOREPLACE)`, and fsyncs the directory. It never overwrites a
different profile or searches for a newest file
([`cache.rs`](../../src/cache.rs#L25-L195)).

The CLI's `recipe probe` computes discovery identity, chooses either the caller
path or an identity-named private profile path, calls
`load_or_probe_and_store`, and prints whether the result came from a validated
cache or fresh measurement ([`cli.rs`](../../../src/cli.rs#L876-L947)). Native
preparation parses the identity from an identity-named path and uses the same
cache reader; absence, stale identity, bad permissions, checksum, schema, or
profile validation is a hard preparation error
([`native_prepare.rs`](../../../src/native_prepare.rs#L248-L271)).

## Downstream consumers and the complete lifecycle

The model has one authoritative path from hardware to scheduling:

| Consumer | How it uses the model |
| --- | --- |
| `recipe` CLI | Builds `ProbeEngine` from local and native adapters, obtains a cache identity, loads or measures one `MeasuredProfile`, and records the active native receipt. The receipt stores the profile path, `CacheIdentity`, host-memory origin, and pinned native libraries/toolchain inputs. On the next run, `profile_uses_backend` inspects `profile.discovery.devices[].calculation.target.backend` to reopen only the required CUDA or HSA library set ([`cli.rs`](../../../src/cli.rs#L97-L113), [`cli.rs`](../../../src/cli.rs#L1011-L1048), [`cli.rs`](../../../src/cli.rs#L1283-L1289)). |
| `recipe-transport` | Implements `PeerSession` and produces authenticated `PeerMeasurement` and `PeerBenchmarkEvidence`; it never writes a profile itself. |
| `recipe-cluster` | Accepts one validated measured profile per member, derives content-bound member identities from canonical profile bytes, remaps member topologies and peer measurements into a new multi-machine `MeasuredProfile`, and validates it through the same `MeasuredProfileCodec`. `ClusterProfileCodec` adds cluster shape checks after canonical decode ([`cluster/src/model.rs`](../../../cluster/src/model.rs#L15-L76), [`cluster/src/assemble.rs`](../../../cluster/src/assemble.rs#L130-L172), [`cluster/src/assemble.rs`](../../../cluster/src/assemble.rs#L783-L830)). |
| `recipe-probe` resolver and `native-probe` bindings | Resolve current RAM/storage/GPU descriptors by origins before lending exact native handles to preparation. |
| `recipe-prepare` | Rejects the seed contract at this boundary, validates the measured profile, and passes only its paired topology and discovery views to reservations, artifact resolution, planning, realization, and finalization ([`prepare/src/lib.rs`](../../../prepare/src/lib.rs#L315-L368)). |
| native preparation | Checks every measured GPU target against compiler targets and reopens exact CUDA/HSA devices before creating native target plans ([`prepare/src/production.rs`](../../../prepare/src/production.rs#L935-L972)). |
| training and execution | Calls preparation with the same profile for local training and inference. Native training derives staging bytes from the storage or RAM benchmark plan, local RAM capacities, calculation rates, transfer rates, and measured concurrency; native resume rejects a kernel whose topology or discovery identity, target, or toolchain differs from the current profile. The execution layer then consumes the finalized topology and discovery data without rediscovering or mutating the profile ([`src/training.rs`](../../../src/training.rs#L1140-L1276), [`training/src/execute.rs`](../../../training/src/execute.rs#L1283-L1305), [`training/src/execute.rs`](../../../training/src/execute.rs#L2092-L2132)). |

The profile is therefore a preparation snapshot, not a live mutable registry.
Its identity and origin records prove which machine and devices were measured;
its topology and discovery pair supplies the complete measured graph and
capability surface; its benchmark metadata explains the measurement bounds; and
its peer records preserve the authenticated evidence required to trust
cross-machine rates. A changed machine, device, driver, runtime, firmware,
link, toolchain, stable key, or target changes the discovery cache identity and
requires a new profile. A fresh probe may also record different measured rates
and evidence under the same discovery identity when the caller explicitly
chooses to replace or remove the old cache file, but ordinary cache loading
does not silently remeasure. No consumer may substitute a seed estimate, stale
cache, ordinal device, or partial inventory as a fallback.
