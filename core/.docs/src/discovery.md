# `recipe_core::discovery`

## Intent

```yaml
module: recipe_core::discovery
source: core/src/discovery.rs
role: immutable-capability-snapshot
authority: measured-device-and-link-capabilities
inputs: topology IDs and topology kinds
outputs: typed capabilities consumed by scheduling, lowering, preparation, and realization
state: owned values with no driver handles, host strings, or borrowed lifetimes
identity: DiscoveryIdentity plus the TopologyIdentity to which the snapshot belongs
```

This module is the small, dependency-free boundary between a topology and the
capabilities that may be used to execute on that topology. It does not discover
hardware, run benchmarks, assign IDs, choose placements, or serialize itself.
Those operations are owned by `probe`, `native-probe`, and `cluster`. The module
owns the resulting typed values and one validation pass that rejects a snapshot
which cannot describe every required topology object.

The profile is a snapshot, not a live query. A consumer borrows a
`DiscoveryProfile`; producers derive a new profile and `DiscoveryIdentity` when
the machine, driver, runtime, firmware, link, or measured property changes. The
Rust fields are public and therefore technically mutable, but the validated
handoff treats the value as immutable evidence. A missing profile record cannot
be substituted with a topology value or another device's capability. Consumers
may use topology values only after validation has established the required
topology/discovery relationship. Core validation establishes exact equality for
directional link bandwidth and lane values; the persisted probe validator also
establishes equality for duplicated device capacity, transfer-rate, and
calculation-rate properties.

## Public structure and data ownership

All fields below are `pub`, so construction is intentionally performed by the
probe and cluster crates rather than by a constructor in this module. The core
types own copies of values. `Property<T>` carries both a value and its explicit
`PropertyProvenance`; `Measured` and `Override` are schedulable, while
`Estimated` is not (`core/src/topology.rs:10-36`).
The module and these types are publicly re-exported by `core/src/lib.rs:10-31`.

| Type | Owned fields | Meaning and authority |
| --- | --- | --- |
| `CalculationCapability` (`core/src/discovery.rs:12-24`) | `target: TargetIdentity`; `rate: Property<FlopsPerSecond>`; `asynchronous_submission`; `maximum_concurrent_tasks`; `subgroup_lanes`; `maximum_workgroup_lanes`; `maximum_shared_memory_per_workgroup` | Calculation execution contract for one GPU-memory device. `target` is the backend, architecture, and ABI selected by native discovery. The rate is measured or explicitly overridden for core validation. The lane and shared-memory limits are native backend limits, not topology estimates. |
| `TransferCapability` (`core/src/discovery.rs:26-32`) | `rate: Property<BytesPerSecond>`; `maximum_inflight_transfers: Property<TransferLaneCount>`; `asynchronous_submission`; `overlaps_calculation` | Device-local or external transfer contract. `overlaps_calculation` is a capability bit used to decide whether transfers and calculations may occupy overlapping time windows. |
| `DiscoveredDevice` (`core/src/discovery.rs:34-42`) | `device: DeviceId`; `available`; `maximum_submission_queues`; `total_capacity: Property<ByteCount>`; `transfer: TransferCapability`; `calculation: Option<CalculationCapability>` | Capability record for one topology device. The topology owns the device kind and structural identity. Discovery owns whether it is available now, queue capacity, total capacity, transfer behavior, and optional GPU calculation behavior. |
| `DiscoveredLink` (`core/src/discovery.rs:44-51`) | `link: LinkId`; `available`; `bandwidth: Property<BytesPerSecond>`; `maximum_inflight_transfers: Property<TransferLaneCount>`; `asynchronous_submission` | Capability record for one directed topology link. The topology owns endpoints, transport, duplex resource, bandwidth, and lane values structurally; validation and persisted-profile checks require these directional values to agree. Discovery adds availability and asynchronous-submission evidence. |
| `DiscoveryProfile` (`core/src/discovery.rs:53-60`) | `identity: DiscoveryIdentity`; `topology: TopologyIdentity`; owned `Vec<DiscoveredDevice>`; owned `Vec<DiscoveredLink>` | Complete capability snapshot for all required topology devices and links. The profile does not own `Topology`, host inventory, benchmark metadata, stable origin labels, or native runtime objects. |

`DiscoveryProfile::device` (`core/src/discovery.rs:277-282`) is a linear lookup by
`DeviceId`. `validate` requires unique IDs first, so a valid profile has at most
one result for an ID. The method returns a borrowed record and does not mutate
the profile.

### Relationship to topology and identity types

`TopologyIdentity` and `DiscoveryIdentity` are typed wrappers over a 256-bit
`Digest` (`core/src/identity.rs:36-96`). Zero identities are invalid at this
boundary. `DeviceId` and `LinkId` are stable typed `u64` wrappers
(`core/src/ids.rs:3-50`), and the topology remains the authority for resolving
their kinds and endpoints. `TargetIdentity` is the artifact-facing backend,
architecture, and ABI tuple (`core/src/artifact.rs:9-15`).

`DiscoveryProfile::topology` is a foreign-key-like binding to
`Topology::identity`. It is not a second topology and cannot be used to recover
topology objects. The profile's `identity` describes the capability snapshot,
while `topology` says which structural snapshot those capabilities belong to.

## Core validation contract

`DiscoveryProfile::validate(&Topology)` accumulates all observed failures in a
`Validator` and returns `ValidationResult` (`core/src/discovery.rs:62-275`). A
caller must treat any `Err(ValidationErrors)` as a rejected profile. Errors carry
a machine-readable `ValidationCode`, a path, and a message
(`core/src/error.rs:3-180`); the pass does not silently repair or discard bad
entries.

### Profile identity and topology binding

1. `identity` must be nonzero, otherwise `InvalidIdentity` at `identity`.
2. `topology` must equal `topology.identity`, otherwise `IdentityMismatch` at
   `topology`.

The comparison is exact digest equality. A profile made for a different
topology is invalid even when its IDs happen to overlap.

### Device coverage and per-device invariants

The validator builds a `BTreeSet<DeviceId>` from `topology.devices`, then walks
every profile record (`core/src/discovery.rs:78-197`). For each
`DiscoveredDevice` at `devices[index]` it requires:

| Check | Failure code and path | Required fact |
| --- | --- | --- |
| ID references topology | `UnknownReference`, `devices[index].device` | The ID is in the topology device set. |
| ID is unique | `DuplicateId`, same path | No second profile record names the same device. |
| Required object is usable | `UnavailableRequiredObject`, `devices[index].available` | `available` is `true`. There is no accepted unavailable placeholder. |
| Queue submission exists | `UnsupportedCapability`, `devices[index].maximum_submission_queues` | Queue count is nonzero. |
| Capacity can schedule | `UnmeasuredProperty`, `devices[index].total_capacity` | Provenance is `Measured` or `Override`. |
| Device transfer rate can schedule | `UnmeasuredProperty`, `devices[index].transfer.rate` | Provenance is schedulable. |
| Device transfer concurrency can schedule | `UnmeasuredProperty`, `devices[index].transfer.maximum_inflight_transfers` | Provenance is schedulable. |
| Device transfer is asynchronous | `UnsupportedCapability`, `devices[index].transfer` | `transfer.asynchronous_submission` is `true`. |

When the ID resolves in the topology, the device kind controls calculation
capability:

- `DeviceKind::GpuMemory` requires `calculation: Some(...)`, then requires a
  schedulable calculation rate, asynchronous calculation submission, and a
  nonzero `maximum_concurrent_tasks`.
- `subgroup_lanes` must be nonzero and a power of two. A nonzero subgroup is a
  native SIMD/SIMT unit, not a tunable planner guess.
- `maximum_workgroup_lanes` must be at least `subgroup_lanes`.
- `maximum_shared_memory_per_workgroup` must be nonzero.
- `DeviceKind::Ram` and `DeviceKind::Disk` require
  `calculation: None`, otherwise `WrongKind` at
  `devices[index].calculation`.

After the record loop, every topology device must appear in the profile. A
missing record produces `MissingRequiredObject` at
`topology.devices[index]`. Unknown records are not ignored, and a duplicate
record is not treated as an update.

### Link coverage and directional consistency

The link pass is analogous (`core/src/discovery.rs:207-272`). It builds the
required `LinkId` set from `topology.links`, then requires for each
`DiscoveredLink`:

- the ID is known (`UnknownReference`) and unique (`DuplicateId`);
- `available` is true (`UnavailableRequiredObject`);
- `bandwidth` and `maximum_inflight_transfers` are schedulable
  (`UnmeasuredProperty`);
- asynchronous submission is true (`UnsupportedCapability`);
- when the topology link exists, discovery bandwidth equals
  `DirectedLink::bandwidth` and discovery concurrency equals
  `DirectedLink::maximum_inflight_transfers`, otherwise `ResourceMismatch`.

Every topology link must have one discovery record, or the pass emits
`MissingRequiredObject` at `topology.links[index]`. The link check is
directional. It does not compare a forward value to its reverse value, because
the topology permits direction-specific measurements.

### What core validation does not do

Core validation does not verify canonical vector order, benchmark metadata,
peer endpoint evidence, stable origin keys, or that all properties are strictly
`Measured`. Those are persisted measured-profile invariants in
`probe/src/codec.rs`. Core validation accepts `Override` as schedulable so a
caller can supply a deliberate override; the probe codec rejects that provenance
when writing a measured profile. It also does not require a numeric
`ByteCount` capacity to be nonzero, and it does not constrain the
`overlaps_calculation` boolean. The producer and the persisted-profile checks
own those stronger evidence requirements.

## Construction path: `probe`

`recipe-probe` owns inventory, benchmark execution, origin metadata, cache
identity, topology construction, and serialization. It re-exports the core
types but does not change their ownership (`probe/src/lib.rs:4-27`). Its
`MeasuredProfile` is the enclosing owned record:

```text
MeasuredProfile
  schema, cache_identity
  origins: stable machine/RAM/storage/GPU keys
  benchmarks: bounded seed-derived plans
  peer_benchmarks: authenticated directional evidence
  topology: Topology
  discovery: DiscoveryProfile
```

The profile model is defined at `probe/src/model.rs:419-535`. Origins are
deliberately outside `DiscoveryProfile`: they retain the host-facing strings
needed to resolve a future machine back to the stable topology IDs, while the
core topology and discovery structures remain free of those host strings.

### Probe engine sequence

`ProbeEngine::probe` (`probe/src/engine.rs:63-161`) executes this order:

1. `inspect` calls `HostDiscovery::discover_host`, normalizes and sorts RAM,
   storage, and network keys, requires RAM and mounted storage, requires an
   exhaustive GPU inventory, sorts GPU descriptors, and validates peer
   descriptors (`probe/src/engine.rs:163-201`, `286-427`).
2. It derives bounded RAM, storage, GPU, and network benchmark plans from seed
   estimates. Seed estimates only bound work; they are never copied into a
   production property (`probe/src/engine.rs:241-275`).
3. It benchmarks every discovered RAM domain, storage domain, GPU descriptor,
   and peer session. Each property must have `Measured` provenance. Peer
   measurements additionally require complete authenticated endpoint evidence,
   the correct full or half-duplex execution mode, exact byte/sample counts,
   duration bounds, and a rate derived from elapsed time
   (`probe/src/engine.rs:429-573`).
4. It hashes the cache inputs, measurement values, and peer evidence into a
   topology digest using domain `recipe-topology-v6`, and a discovery digest
   using domain `recipe-discovery-v6` (`probe/src/engine.rs:575-787`). The
   current profile schema is `7`; the digest domain strings are source-defined
   identity versions and must not be inferred from the schema number.
5. `build_topology` allocates deterministic IDs and constructs machines, nodes,
   devices, and opposing directed link pairs. It validates the topology before
   returning (`probe/src/engine.rs:901-1126`).
6. `build_origins` retains the exact machine and domain keys that were used to
   allocate those IDs (`probe/src/engine.rs:846-899`).
7. `build_discovery` copies measured values into `DiscoveredDevice` and
   `DiscoveredLink` records (`probe/src/engine.rs:1135-1276`), binds
   `topology: topology.identity`, and sets the discovery identity computed in
   step 4.
8. The engine validates topology structure, topology scheduling properties, and
   the discovery snapshot, then runs the complete codec validator over the
   assembled `MeasuredProfile` (`probe/src/engine.rs:121-160`).

### Exact device mapping produced by `build_discovery`

The mapping is direct and deterministic. IDs come from `assign_ids`, whose
ordered groups are local RAM, local storage, local GPUs, and peer RAM
(`probe/src/engine.rs:789-844`). All records are marked `available: true`.

- RAM: queue count and transfer lanes come from the RAM domain; capacity and
  transfer rate come from the measured RAM result; there is no calculation
  capability; transfer overlap is true.
- Storage: queue count and transfer lanes are the minimum of read and write
  limits; transfer rate is the minimum of measured read and write rates; there
  is no calculation capability; asynchronous submission comes from the storage
  descriptor; transfer overlap is true.
- GPU memory: queue count and native limits come from the GPU descriptor;
  capacity, memory rate, and calculation rate come from GPU measurements;
  `CalculationCapability` copies target, asynchronous submission, concurrent
  task count, subgroup width, workgroup limit, and shared-memory limit; transfer
  overlap comes from the descriptor.
- Peer RAM: queue count and transfer lanes are the minimum of outbound and
  inbound limits; capacity and rate come from remote-memory measurements; there
  is no calculation capability; asynchronous submission comes from the peer
  descriptor; transfer overlap is true.

For each topology link, `build_discovery` copies the topology bandwidth and
directional lane value and supplies the matching asynchronous-submission bit.
The source vector is assembled in the same storage, GPU, and peer pair order as
`build_topology`; the codec and profile validators catch any resulting missing
or mismatched link.

## Native GPU discovery boundary

`native-probe` implements the `GpuDiscovery` and `GpuBenchmarkIo` traits used by
`ProbeEngine` (`native-probe/src/native.rs:245-299`). It is the owner of backend
libraries, runtime handles, PCI/sysfs reads, native benchmark submissions, and
their failures. None of those handles or strings are stored in
`DiscoveryProfile`.

### Inventory and exact identity

`NativeGpuProbe::new` creates CUDA and HSA backends and marks the inventory
exhaustive. `cuda_diagnostic` and `hsa_diagnostic` intentionally mark a
single-backend inventory non-exhaustive, which cannot produce an accepted
measured profile (`native-probe/src/native.rs:33-86`). `discover_all` merges,
sorts, and rejects duplicate descriptor keys.

CUDA descriptor construction (`native-probe/src/cuda.rs:86-207`) requires PCI
identity to agree with driver attributes, canonicalizes the sysfs BDF, and
retains driver, firmware, PCIe-link, runtime, and pinned-toolchain identity
surfaces. It emits:

```text
key                 cuda:<driver UUID>@<canonical PCI BDF>
target.backend      nvidia-cuda-driver
target.abi          elf64-cubin
transport           PCIe, full duplex
host/device lanes   one each
queues              CUDA_MAXIMUM_SUBMISSION_QUEUES
calculation         one concurrent task, driver warp/workgroup/shared-memory limits
overlap             async engine count != 0 and concurrent kernels
```

HSA descriptor construction (`native-probe/src/hsa.rs:153-293`) requires a GPU
agent with a stable UUID, exact PCI address, kernel dispatch support, AMD ISA,
queue limits, GPU memory capacity, and KFD LDS capacity. It emits a key of
`hsa:<agent UUID>@<PCI address>`, target backend `amd-rocr-hsa`, an ELF64 AMDGPU
code-object ABI, full-duplex PCIe, one host/device lane each, queue and ISA
limits from ROCr, and transfer overlap from SDMA engine availability.

Both backends reopen and rediscover the exact expected descriptor before a GPU
benchmark. The benchmark submits real transfer and Recipe-owned FMA work,
verifies output, and returns measured capacity/rates. A descriptor that changes,
disappears, is ambiguous, or is claimed by two backends is a `ProbeError`, not a
fallback to an estimated capability.

`NativeGpuProbe::benchmark_gpu` first requires a bounded plan, rediscovering
each backend and requiring exactly one descriptor equal to the expected record.
The resulting `GpuMeasurement` is then consumed by `ProbeEngine`; a native
failure prevents `DiscoveryProfile` construction.

### Reopening a measured profile

`MeasuredProfile::resolve_local_inventory` (`probe/src/resolve.rs:76-115`) and
`native-probe::with_native_execution_bindings` (`native-probe/src/bindings.rs:120-234`)
are the preparation-time consumers of discovery origins. They:

- require a valid profile and exhaustive current GPU inventory;
- match the current machine fingerprint exactly;
- require the current RAM, storage, and GPU key sets to equal the retained
  origin key sets;
- require storage and GPU host-memory keys to resolve to current RAM keys;
- require each current GPU target to equal its retained discovery calculation
  target;
- reopen every current GPU by exact key and reject duplicates, missing keys,
  unsupported backends, changed descriptors, missing runtimes, or ambiguous HSA
  host allocators.

Capacity, product name, ordinal, and benchmark similarity are explicitly not
selectors. A mismatch means the measured profile is stale and a fresh probe is
required.

## Persisted profile and serialization

`MeasuredProfileCodec` (`probe/src/codec.rs:31-157`) is the serialization
authority for a complete measured profile. The current constants are
`PROFILE_SCHEMA = 7` and `PROFILE_CODEC_SCHEMA = 7`
(`probe/src/model.rs:17-19`). The format is a bounded little-endian binary
payload followed by a SHA-256 checksum:

```text
MAGIC (16 bytes: RECIPEPROFILE\0\0\0)
codec schema u32
profile schema u32
cache schema u32
cache digest (32 bytes)
origins
benchmark metadata
peer benchmark records and evidence
topology
discovery
SHA-256(payload) (32 bytes)
```

`encode` validates before writing and rejects an encoded result larger than
256 MiB. `decode` enforces the same size bound, minimum framing, checksum,
magic, exact codec schema, complete payload consumption, and then validates the
decoded profile. Lengths are capped at one million items and labels at one MiB;
invalid booleans, provenance tags, transport tags, duplex tags, UTF-8, unit
values, or truncated fields are cache/codec failures (`probe/src/codec.rs:1281-1502`).

The discovery portion is encoded after topology (`probe/src/codec.rs:1166-1279`):

```text
discovery.identity digest
discovery.topology digest
device count
  device ID, available, queue count
  total capacity property
  transfer rate property, transfer lanes property, async, overlap
  has-calculation flag
    target backend, architecture, ABI labels
    calculation rate property, async, max concurrent tasks
    subgroup lanes, max workgroup lanes, shared-memory bytes
link count
  link ID, available, bandwidth property, lane property, async
```

Properties serialize their numeric value followed by a provenance tag. The
decoder reconstructs typed nonzero rate and lane units, so a zero rate or lane
cannot enter a profile through bytes alone.

### Complete persisted-profile checks

`validate_profile` performs more than `DiscoveryProfile::validate`:

- profile and cache schemas must equal `PROFILE_SCHEMA`, and cache digest must
  be nonzero;
- all benchmark plans must be bounded and use the seed contract schema;
- peer evidence must use protocol schema 1, identify two nonzero machines,
  match its directional byte/sample/time bounds, and derive exactly its stored
  rates;
- topology and discovery properties must agree for every device and directed
  link (`require_topology_discovery_match`);
- every topology machine, RAM device, disk device, and GPU-memory device must
  have exactly one origin, with unique stable keys scoped by machine;
- origins, peer records, topology objects, discovery records, and node device
  lists must be in strict canonical order;
- every persisted scheduling property must have `Measured` provenance, not
  `Estimated` or `Override`;
- cross-machine links must form opposing directional pairs, and each peer
  benchmark must match one unused pair's endpoint evidence, measured rates, and
  full or half-duplex resource semantics.

These checks are why a valid `DiscoveryProfile` assembled directly in core is
not automatically a valid cached `MeasuredProfile`.

`ExplicitPathProfileCache` only accepts an absolute path whose parent and file
are private, owned by the effective user, canonical, and non-symlink. It decodes
and validates before returning a profile, checks the requested `CacheIdentity`,
and installs a new file without replacing a different existing profile
(`probe/src/cache.rs:25-195`).

`probe/PROFILE_SCHEMA.md` still describes schema and hash domains as version 6.
That prose is stale relative to the executable source: the current constants,
decoder gate, cache identity, and probe output all use schema 7
(`probe/src/model.rs:17-19`, `probe/src/codec.rs:121-136`). Consumers must use
the code-level schema constants and reject a profile whose schema does not equal
7; the markdown file is not a second compatibility authority.

## Cluster assembly

The cluster crate treats each submitted `MeasuredProfile` as a per-machine
measured member. `MemberProfileIdentity::derive` canonical-encodes the member,
hashes its stable machine ID into an endpoint machine identity, and retains its
cache, topology, and discovery identities (`cluster/src/model.rs:15-77`). A
member's expected `DiscoveryIdentity` is therefore part of cluster membership,
not user-entered hardware configuration.

`assemble_cluster` validates members and network evidence, computes separate
cluster topology, discovery, and profile digests, remaps all member IDs, and
constructs one cluster `DiscoveryProfile` (`cluster/src/assemble.rs:128-173`,
`491-578`). Member discovery records are cloned with remapped device and link
IDs. Inter-machine measurements create two available discovered links with
measured directional bandwidth and lane values, plus the descriptor's
asynchronous-submission bit (`cluster/src/assemble.rs:726-780`). The resulting
profile is passed through `MeasuredProfileCodec::encode` before assembly
returns.

Cluster identity hashing includes each member's expected discovery digest and
all network rates, lane limits, async/duplex evidence, endpoint identities, and
benchmark evidence (`cluster/src/hash.rs:50-108`). Any change to a member
discovery snapshot or measured network capability therefore changes the cluster
discovery identity. `ClusterProfileCodec` wraps the canonical codec and adds
the cluster shape requirement of at least two machines and a connected
inter-machine transport graph (`cluster/src/assemble.rs:783-859`).

Cluster failures remain explicit: missing, duplicate, stale, or invalid member
profiles; missing, duplicate, unconfigured, or invalid network measurements;
identity exhaustion; invalid cluster shape; and codec failures are represented
by `ClusterError` (`cluster/src/error.rs:3-66`). No cluster path invents a
capability when a member record is absent.

## Planner, scheduler, and preparation consumers

### Planner

`plan_program_candidates` validates topology, topology scheduling properties,
and `discovery.validate(topology)` before lowering or enumerating assignments
(`planner/src/planner.rs:220-323`). Discovery has four direct planning roles:

1. `common_lowering_hardware` selects available calculation capabilities and
   computes a common lowering envelope: maximum subgroup width, minimum
   workgroup limit, and minimum shared-memory capacity. No available calculation
   device is `InvalidDiscovery`; incompatible subgroup/workgroup limits are also
   `InvalidDiscovery` (`planner/src/planner.rs:436-471`).
2. `legal_choices` permits placement only on topology GPU-memory devices with a
   discovered calculation capability. An empty set is `NoCalculationDevice`
   (`planner/src/planner.rs:519-555`).
3. Deferred or prebuilt artifacts must target the exact discovered calculation
   `TargetIdentity` for their selected device. A mismatch is `InvalidArtifact`
   (`planner/src/planner.rs:1681-1716`).
4. Candidate identity hashes both topology and discovery digests, so changing
   measured capabilities invalidates a prior candidate
   (`planner/src/planner.rs:746-763`).

The planner also uses `maximum_submission_queues` when compacting submission
resources (`planner/src/planner.rs:2784-2796`). Discovery does not allocate the
queue slots; it supplies the measured upper bound used to compact them.
Planner failures include `InvalidDiscovery`, `NoCalculationDevice`,
`InvalidArtifact`, `NoRoute`, `CandidateInfeasible`, `NoViableCandidate`, and
identity collisions (`planner/src/error.rs:3-50`).

### Scheduler

The static scheduler validates discovery before preparing tasks
(`scheduler/src/static_schedule.rs:61-76`). It uses:

- calculation rate for `calculation_time_ceil` and maximum concurrent tasks for
  compute-lane resources;
- device transfer rate and lane count for external transfer duration and lane
  resources;
- directed-link bandwidth and lane count for each internal hop;
- `overlaps_calculation` to add a no-overlap resource when a device transfer
  cannot run concurrently with calculation;
- asynchronous capability and identity validity indirectly through the earlier
  profile validation.

An unavailable endpoint or missing capability yields a schedule error, not a
zero-duration task. Multi-hop internal transfers must already be lowered into
dependency-chained one-link tasks (`scheduler/src/static_schedule.rs:193-375`).

### Core plans and preparation

`DraftPlan::validate` appends discovery validation and requires its stored
`DiscoveryIdentity` to equal the current profile (`core/src/plan.rs:252-297`).
It checks every calculation artifact target against the selected device's
discovered target and every transfer lane claim against discovered lane limits
(`core/src/plan.rs:480-523`, `1022-1105`). Resource-contention validation uses
device transfer overlap and measured link/device concurrency
(`core/src/plan.rs:1242-1417`).

`RealizationProfile` and `FinalizedBundle` retain the discovery identity and
reject a changed profile before artifacts, reservations, or layouts can cross
the next lifecycle boundary (`core/src/plan.rs:2057-2228`, `2375-2516`). Native
preparation validates the profile, requires a build target for every discovered
calculation target, and snapshots the exact topology/discovery pair into its
candidate session (`prepare/src/production.rs:935-1019`). Native candidate
requests likewise reject invalid discovery, missing GPU capability, and target
mismatch before opening a driver session (`native-executor/src/candidate.rs:43-112`,
`333-407`, `571-612`).

The public `prepare` orchestrator validates the complete measured profile before
passing its topology and discovery references to reservation planning, artifact
resolution, planner search, and candidate realization (`prepare/src/lib.rs:327-377`).
Its optimistic planning capacity reads each `DiscoveredDevice::total_capacity`,
subtracts the exact reservation, and retains the same property provenance in
the provisional capacity ledger (`prepare/src/lib.rs:554-605`). It records the
discovery identity in realization and bundle hashes (`prepare/src/lib.rs:757-805`),
so a capacity observation cannot be attached to a different measured snapshot.

The local native executor owns physical capacity observations, not discovery
capability. `LocalPreparedSession` stores a cloned topology/discovery pair and
an initial capacity snapshot keyed by both identities (`native-executor/src/local.rs:880-910`,
`999-1003`). Candidate realization rejects an initial snapshot for another
topology or discovery, capacity observation rejects changed identities, and the
finalized bundle must retain the candidate's discovery identity
(`native-executor/src/local.rs:1439-1475`, `1575-1605`, `3279-3312`, `3430-3446`).
The local executor therefore uses discovery to bind observed capacity to the
same immutable preparation evidence; it does not rewrite capability values.

### Runtime, checkpoint, and reporting identity consumers

Discovery identity continues beyond preparation even where no capability field
is read:

- Native execution retains `RealizedNativeKernelSet { topology, discovery, ... }`
  so handed-off native images are tied to the measured system
  (`training/src/execute.rs:307-328`). Training resume rejects a supplied native
  kernel whose topology or discovery identity differs from the current profile,
  then separately checks target and toolchain identity
  (`src/training.rs:1278-1305`).
- Checkpoint native metadata stores and decodes `DiscoveryIdentity` alongside
  program, realization, topology, and kernels; a native realization without
  those fields or kernels is a decode failure
  (`training/src/checkpoint.rs:1062-1087`, `1681-1730`).
- The CLI prints cache, topology, and discovery digests from the validated
  profile, uses discovered calculation targets to build the active native
  receipt, and refuses a receipt whose retained host-memory origin is absent
  from current host discovery (`src/cli.rs:925-1008`, `1011-1028`).
- The hardware acceptance runner records the measured discovery digest as
  evidence, alongside the profile and topology digests
  (`acceptance/src/main.rs:181-205`).

These reporting and artifact records carry identity only. They do not make a
digest a substitute for loading and validating the complete `DiscoveryProfile`.

## Failure map and safe next action

| Boundary | Evidence that is rejected | Error surface | Required next action |
| --- | --- | --- | --- |
| Core profile validation | Unknown, duplicate, unavailable, unmeasured, wrong-kind, zero-capability, or topology-mismatched record | `ValidationErrors` with `ValidationCode` and path | Repair or reprobe the actual source profile. Do not add a fallback record. |
| Host/native probe | Incomplete inventory, missing runtime, changed PCI/driver identity, failed native submission, unverified output, incomplete peer measurement, or unbounded plan | `ProbeError::Discovery`, `Benchmark`, `IncompleteGpuEnumeration`, `MissingMeasurement`, or `IncompletePeerMeasurement` | Fix the current machine/backend/session or rerun `recipe probe`. |
| Persisted codec/cache | Bad framing/checksum/schema, noncanonical order, invalid origin, estimated/override persisted property, topology/discovery drift, or stale cache identity | `ProbeError::Cache` with `codec:` detail | Discard the invalid cache and produce a fresh fully measured profile. |
| Cluster assembly | Stale member identity, missing or duplicate network evidence, endpoint/rate/duplex contradiction, disconnected graph | `ClusterError` | Recollect the affected member or peer measurement and assemble again. |
| Planner/scheduler/preparation | No available calculation target, target mismatch, missing capability, route or lane violation, changed discovery identity | `PlannerError`, `ScheduleError`, validation errors, or typed preparation/candidate rejection | Replan or reprepare against the current complete profile. |

The common invariant is fail closed at the boundary that owns the evidence.
Discovery never manufactures capacity, changes topology, chooses a substitute
device, or downgrades a native failure to absence.
