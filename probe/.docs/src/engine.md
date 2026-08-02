# Probe engine

`recipe_probe::engine::ProbeEngine` is the orchestration boundary for one
bare-metal probe. It combines a discovered host inventory, an exhaustive GPU
inventory, established peer descriptors, and completed bounded measurements
into one immutable, measured `MeasuredProfile`. The engine owns ordering,
validation, identity construction, and translation into the core topology and
discovery representations. It does not own a vendor runtime, a storage cache,
or a default machine inventory. Those are supplied through the traits below.

The implementation is in [`probe/src/engine.rs`](../../src/engine.rs). The
input and output types are in [`probe/src/model.rs`](../../src/model.rs), the
native GPU adapter is in `native-probe/src/native.rs`, and the CLI caller is
`src/cli.rs::run_probe`.

## Boundary and purpose

The probe is the source of production topology properties. Seed values are
theoretical estimates used only to size bounded benchmark work. They are not
copied into the profile as capacities, rates, queue limits, or capabilities.
Every `Property<T>` scalar that the engine puts in the returned topology or
discovery profile is marked `PropertyProvenance::Measured`; boolean flags and
integer capability fields are copied from the discovered descriptors.

The engine has four injected capabilities:

| Field | Trait | Responsibility at the boundary |
| --- | --- | --- |
| `host_discovery` | `HostDiscovery` | Return the current machine fingerprint and all local RAM, mounted storage, and network domains. |
| `gpu_discovery` | `GpuDiscovery` | Return every GPU visible to the native calculation backends and say whether enumeration is exhaustive. |
| `host_benchmarks` | `HostBenchmarkIo` | Measure each discovered RAM and storage domain with a supplied bounded plan. |
| `gpu_benchmarks` | `GpuBenchmarkIo` | Measure each discovered GPU with a supplied bounded plan. |

`ProbeEngine<'a>` only borrows these capabilities. It retains no inventory,
measurement, or cache state between calls. A call performs fresh discovery and
fresh measurement unless the caller chooses the cache-aware method, in which
case the discovery-only identity check can return an already validated profile
before benchmark submission.

## Public methods

`ProbeEngine::new` stores the four borrowed capability objects. It performs no
I/O and no validation.

`ProbeEngine::probe(seed, peer_sessions)` runs the complete discovery,
measurement, profile construction, and validation pipeline. It returns either a
fully validated `MeasuredProfile` or a `ProbeError`; there is no partial
profile result.

`ProbeEngine::current_cache_identity(seed, peer_sessions)` runs only the
discovery part of the pipeline (`inspect`). It sorts and validates the current
host, GPU, and peer descriptors and returns the exact `CacheIdentity` that a
fresh probe would use. It does not create benchmark plans, call a benchmark
backend, validate a measured profile, or access a cache.

`ProbeEngine::load_or_probe_and_store(seed, peer_sessions, cache)` first calls
`current_cache_identity`. If `cache.load(identity)` returns `Some(profile)`,
that profile is returned and no benchmark is run. If the cache returns `None`,
the method calls `probe_and_store` and therefore performs a fresh probe before
storing it. A cache implementation is responsible for validating the profile
it loads; `ExplicitPathProfileCache` does so through the profile codec.

`ProbeEngine::probe_and_store(seed, peer_sessions, cache)` always runs
`probe`, then calls `cache.store(&profile)`. A store failure is returned after
the successful probe, and no profile is reported as stored by the engine.

## Call graph at the real CLI boundary

The production path is the following. The engine does not select a different
path based on a benchmark result.

```text
recipe probe
  -> src/cli.rs::run_probe
     -> require_bare_metal
     -> SeedContract::read or SeedContract::parse(topology/contract.toml)
     -> LocalSystemDiscovery::discover_host
     -> NativeProbeConfig and NativeGpuProbe::new
     -> LocalHostBenchmarks
     -> ProbeEngine::new(host, native, host_benchmarks, native)
     -> current_cache_identity(seed, no peer sessions)
     -> ExplicitPathProfileCache
     -> load_or_probe_and_store
          -> cache hit: return validated profile
          -> cache miss: probe -> benchmark -> validate -> cache.store
     -> write active-native-v1 receipt
     -> print profile and identity summary
```

The CLI uses no peer sessions today, so the normal `recipe probe` profile has
one local machine and no worker nodes. The engine itself supports established
peer sessions and constructs worker nodes when the caller supplies them.

## Discovery-only inspection

`inspect` establishes the stable input order and the cache identity before any
benchmark is submitted.

### Host

1. `host_discovery.discover_host()` is called.
2. `normalize_host` sorts RAM domains, storage domains, and network interfaces
   by their stable `Label` keys.
3. At least one RAM domain and one mounted storage domain are required. Network
   may be empty when no peer is being described.
4. RAM, storage, and network keys must each be unique.
5. Every storage domain must reference a known RAM key and declare
   `asynchronous_submission`.

The host inventory carries a `MachineFingerprint` with hostname, stable
machine ID, runtime ABI, and firmware identity. The engine does not synthesize
any of these fields.

### GPU inventory

`gpu_discovery.discover_all()` is called once during `inspect`. The returned
`GpuInventory` must set `exhaustive` to `true`, otherwise the engine returns
`ProbeError::IncompleteGpuEnumeration`. An empty exhaustive inventory is also
an error because a measured profile must contain at least one GPU calculation
device.

Descriptors are sorted by `GpuDescriptor::key`. The sorted set must have unique
keys. Each GPU must reference a host RAM key, support asynchronous submission,
report at least one submission queue, and report at least one concurrent task.
The engine does not choose a GPU by ordinal, name, capacity, or benchmark
similarity.

### Peer descriptors

For every supplied `PeerSession`, `descriptor()` is called. The resulting
`(session, descriptor)` pairs are sorted by `session_id` and validated as one
set:

* session IDs and remote stable machine IDs are unique;
* a peer machine cannot have the local machine's stable ID;
* the peer's local RAM and local network-interface keys must exist in the host
  inventory;
* the peer transport kind and duplex must equal the referenced local
  interface's kind and duplex; and
* the peer must support asynchronous submission.

Peer sessions are not inferred from host network interfaces. A session is a
caller-owned, established control/data path, and only that session can supply
peer throughput evidence.

### Cache identity

After the above validation, `build_cache_identity` constructs a
`CacheIdentity { schema: PROFILE_SCHEMA, digest }`. The canonical digest domain
is `recipe-probe-cache-v7` and the schema is currently `7`.

The digest includes, in deterministic order:

* seed schema, every numeric seed estimate, the storage reservation, every
  invalidation facet, and each transport seed's name and duplex;
* local machine hostname, stable ID, runtime ABI, and firmware;
* each RAM key, capacity hint, link identity, and maximum in-flight count;
* each storage key, name, capacity hint, benchmark root, host RAM key, driver,
  firmware, link identity, transport kind, asynchronous flag, duplex, read
  limit, and write limit;
* each network key, name, address, driver, firmware, link identity, transport
  kind, asynchronous flag, and duplex;
* each GPU key, name, capacity hint, host RAM key, target backend,
  architecture and ABI, driver, runtime ABI, firmware, link identity, transport
  kind, toolchain name/version/digest, asynchronous flag, queue and task limits,
  subgroup and workgroup limits, shared-memory limit, transfer-overlap flag,
  duplex, and directional in-flight limits; and
* each peer session ID, remote machine fingerprint, remote and local memory
  keys, local and remote interface identities, remote driver and firmware, link
  identity, transport kind, asynchronous flag, duplex, and directional
  in-flight limits.

`CanonicalDigest` length-prefixes strings and byte arrays and writes integer
values in little-endian form before SHA-256 finalization. The seed policy
booleans are validated when a `SeedContract` is parsed, but they are not fields
hashed by `hash_seed`; the digest uses the seed values and identity facets
listed above.

## Bounded benchmark plans

`BenchmarkPlans::from_seed` derives one plan for each measurement class. Integer
division happens before clamping:

| Class | Suggested buffer |
| --- | --- |
| RAM | `seed.estimates.ram_capacity / 1024` |
| Storage | `seed.estimates.disk_capacity / 16_384` |
| GPU | `seed.estimates.gpu_memory_capacity / 1024` |
| Network | `seed.estimates.ethernet_rate / 8` |

`bounded_plan` clamps every suggestion to the inclusive range 4 KiB through
64 MiB, uses eight iterations, and sets a two-second maximum duration. The
resulting `BoundedBenchmarkPlan` is copied into `MeasuredProfile::benchmarks`
with the seed schema. The engine does not use the other seed estimates as
runtime rates and does not make a plan depend on the measured capacity of a
device discovered later.

## Measurement lifecycle

After `inspect` and plan creation, `probe` measures in this order:

1. every sorted host RAM domain with `benchmark_ram(domain, plans.ram)`;
2. every sorted host storage domain with `benchmark_storage(domain,
   plans.storage)`;
3. every sorted GPU descriptor with `benchmark_gpu(device, plans.gpu)`; and
4. every sorted peer session with a fresh `PeerBenchmarkControl` and
   `benchmark_controlled(plans.network, &control)`.

Each result is validated before it enters the internal `Measurements` value.
RAM requires measured capacity and transfer rate. Storage requires measured
capacity, read rate, and write rate. GPU requires measured capacity,
calculation rate, memory rate, host-to-device rate, and device-to-host rate.
`require_measured` checks provenance only; the backend owns the measurement
operation and its value-level checks.

Peer measurements additionally require measured remote memory capacity and
remote memory rate, both measured directional throughputs, and internally
consistent authenticated evidence. The evidence must:

* use `PEER_BENCHMARK_PROTOCOL_SCHEMA` (`1`);
* contain nonzero local/remote machine and profile digests for two distinct
  machines;
* use simultaneous execution for a full-duplex descriptor and serialized
  execution for a half-duplex descriptor;
* report exactly `plan.buffer_bytes * plan.iterations` bytes;
* report a nonzero elapsed duration no greater than the two-second plan bound;
* report exactly the plan iteration count;
* order minimum, mean, maximum, and total sample durations consistently; and
* have each stored rate exactly equal to the duration-derived rate.

`PeerBenchmarkControl::for_plan` creates a fresh absolute deadline from the
plan duration and a fresh cancellation token for each session. The default
`PeerSession::benchmark_controlled` checks cancellation/deadline before the
attempt, calls the legacy `benchmark`, checks the deadline again, and converts
an error into structured transport failure evidence. A peer implementation
with blocking operations can override this method and apply the supplied
deadline to every framed operation. `PeerBenchmarkAttempt::into_measurement`
turns a failed attempt into `ProbeError::Benchmark`; failed attempts cannot
become measured properties.

## Measurement-derived identities

Once all measurements have passed their class-specific checks, the engine
keeps them in:

```text
Measurements {
    ram: BTreeMap<Label, RamMeasurement>,
    storage: BTreeMap<Label, StorageMeasurement>,
    gpu: BTreeMap<Label, GpuMeasurement>,
    peers: Vec<(PeerDescriptor, PeerMeasurement)>,
}
```

`build_profile_digest("recipe-topology-v6", cache_identity, measurements)` and
`build_profile_digest("recipe-discovery-v6", cache_identity, measurements)`
hash the cache digest, all measured scalar values, and complete peer evidence.
The two domains produce independent `TopologyIdentity` and
`DiscoveryIdentity` values. The domain labels remain `v6` in the implementation
even though the profile schema and codec schema are `7`; they are separate
hash-domain strings, not the profile schema field.

The same sorted inputs are assigned deterministic IDs by `assign_ids`:

* local machine is `MachineId(1)`;
* local RAM devices receive `DeviceId`s first, then local storage, then GPUs;
* peer RAM devices follow the local devices in sorted peer order;
* peer machines receive `MachineId`s `2`, `3`, and so on in sorted peer order;
* local node is `NodeId(1)`, and peer worker nodes follow in the same order;
* links, transports, and duplex resources each start at `1` and are allocated
  in storage, GPU, then peer-link order.

`build_origins` retains identity provenance separately from the core topology:
local and peer machine fingerprints, local RAM keys, peer remote-memory keys,
local storage keys, and local GPU keys. Origins are keyed by stable discovered
identities, never by capacity, rate, ordinal position, or product name.

## Topology construction

`build_topology` receives the topology identity, normalized inventories, and
validated measurements.

### Machines, nodes, and devices

The first machine is the local machine, named by its hostname. Every peer
descriptor adds one machine named by the peer hostname. The local node is a
`NodeRole::Master` containing all local RAM, disk, and GPU-memory device IDs.
Each peer adds a `NodeRole::Worker` containing exactly its remote RAM device.

The local devices are built as follows:

* RAM uses measured RAM capacity and transfer rate and has no calculation rate.
* Disk uses measured capacity and the lower of measured read and write rates.
* GPU memory uses measured capacity, measured GPU memory rate, and the measured
  calculation rate.

Each peer RAM device uses measured remote capacity and remote memory rate and
has no calculation rate. All device properties are copied with their measured
provenance.

### Directed links and duplex resources

`LinkBuilder::push_pair` creates two opposing `DirectedLink`s for each physical
pair:

* host RAM to storage uses measured write rate and the reverse uses measured
  read rate;
* host RAM to GPU uses measured host-to-device rate and the reverse uses
  measured device-to-host rate; and
* local RAM to peer RAM uses measured outbound rate and the reverse uses
  measured inbound rate.

Both directions share one `TransportId` and one transport kind. Full duplex
uses two distinct `DuplexResourceId`s; half duplex shares one resource. The
duplex enum is converted from the probe model's `LinkDuplex`, so topology
validation can enforce the core transport contract. Directional concurrency
limits come from the discovered domain or descriptor and are marked measured.

The topology is validated immediately inside `build_topology`, then validated
again by `probe` for the final profile. Core validation checks nonzero and
unique identities, machine/device/node ownership, exactly one master, complete
reverse link pairs, transport/duplex consistency, and valid capacity-resource
ownership. `validate_scheduling_properties` separately rejects any estimated
capacity, rate, or concurrency property.

## Discovery profile construction

`build_discovery` uses the same assigned device and link IDs as the topology.
Every emitted object is marked available.

* RAM transfer capability uses its measured transfer rate, discovered maximum
  in-flight count, asynchronous submission, and `overlaps_calculation: true`.
* Storage transfer capability uses the lower measured read/write rate and the
  lower discovered read/write concurrency. Its asynchronous flag comes from
  the storage domain.
* GPU transfer capability uses measured GPU memory rate and the lower of the
  host-to-device and device-to-host concurrency limits. Its asynchronous and
  overlap flags come from the GPU descriptor. Its calculation capability
  carries the descriptor target, measured FLOP rate, asynchronous flag,
  concurrent-task limit, subgroup width, workgroup limit, and shared-memory
  limit.
* Peer RAM transfer capability uses measured remote memory rate and the lower
  outbound/inbound concurrency. Its asynchronous flag comes from the peer
  descriptor and it overlaps calculation.

For links, the engine creates two asynchronous flags per storage, GPU, and
peer pair, then zips those flags with the topology's directed-link vector. The
ordering is the same as `LinkBuilder`, so each directed link receives the
matching domain or descriptor asynchronous flag. Link bandwidth and
concurrency are copied from the topology.

The discovery identity is independent from, but tied to, the topology identity
through `DiscoveryProfile::topology`. Core discovery validation requires a
nonzero identity, exact topology identity, one available discovery record for
every topology device and link, nonzero queue support, asynchronous transfer
and calculation capabilities, measured scheduling properties, and capability
shape appropriate to each device kind.

## Final profile and validation order

After topology and discovery construction, `probe` performs the following
checks in order:

1. `topology.validate()`;
2. `topology.validate_scheduling_properties()`;
3. `discovery.validate(&topology)`;
4. construction of `MeasuredProfile` with `PROFILE_SCHEMA`, cache identity,
   origins, benchmark metadata, peer benchmark records, topology, and discovery;
5. `crate::codec::validate_profile(&profile)`.

The final codec validation is stricter than the individual engine checks. It
requires schema and cache schema `7`, a nonzero cache digest, bounded benchmark
metadata, canonical ordering, exclusively measured persisted properties,
complete machine/RAM/storage/GPU origins, exact topology/discovery property
agreement, and peer evidence that matches one opposing cross-machine link pair
with the measured rates and duplex execution mode.

The returned value is therefore:

```text
MeasuredProfile {
    schema: PROFILE_SCHEMA,
    cache_identity,
    origins,
    benchmarks,
    peer_benchmarks,
    topology,
    discovery,
}
```

No benchmark journal, temporary file, kernel artifact, or intermediate profile
is returned by the engine. Those concerns belong to the injected benchmark
implementation or to the caller's cache and native-preparation layers.

## Production backend selection and measurement ownership

The engine itself is backend-neutral. The production CLI supplies
`NativeGpuProbe` for both GPU discovery and GPU measurement. That adapter is
constructed with both `CudaBackend` and `HsaBackend` and reports an exhaustive
inventory.

`NativeGpuProbe::discover_all` calls each configured backend, concatenates its
descriptors, sorts by key, rejects duplicate keys, and returns the configured
exhaustive flag. `NativeGpuProbe::new` sets that flag to `true`. The
`cuda_diagnostic` and `hsa_diagnostic` constructors intentionally set it to
`false`, so `ProbeEngine::inspect` rejects those inventories instead of
publishing a partial profile.

Each native backend treats absent hardware differently from a broken runtime:
if no PCI accelerator for that vendor exists, a missing library produces an
empty backend result. If hardware exists, a missing configured library, load
failure, or failed exhaustive enumeration is an error. This prevents hardware
from being silently treated as absent.

For each GPU benchmark, `NativeGpuProbe::benchmark_gpu` first rejects an
unbounded plan. It rediscoveries every backend and compares each candidate
descriptor for exact equality with the descriptor captured by `inspect`. It
requires exactly one backend owner, rejects an identity change, and delegates
the bounded plan to that owner. CUDA and HSA therefore cannot be selected by a
name or ordinal fallback.

The concrete measurements are still owned by the backends:

* `LocalHostBenchmarks::benchmark_ram` copies a bounded in-memory buffer for up
  to the plan iteration and duration limits and returns the domain's discovered
  capacity hint plus the observed copy rate, both measured.
* `LocalHostBenchmarks::benchmark_storage` creates a unique temporary file in
  the discovered benchmark root, performs bounded synced writes and reads,
  measures each direction, reads filesystem available bytes for capacity, and
  removes the temporary file on drop.
* `CudaBackend` and `HsaBackend` allocate bounded host/device buffers, time
  host-to-device, device-to-host, and device-to-device copies, build and submit
  a Recipe-owned dependent-f32-FMA kernel, verify copied and computed output,
  and return measured capacity and rates. A completion deadline is applied to
  each bounded operation.

The engine sees only the `RamMeasurement`, `StorageMeasurement`, and
`GpuMeasurement` values returned by these boundaries. It validates provenance
and cross-field peer evidence, then translates the results without re-running
or reinterpreting the backend operations.

## Host discovery used by the CLI

`LocalSystemDiscovery` is the default host implementation used by
`run_probe`. It reads procfs and sysfs, not a user-supplied hardware table:

* machine identity comes from `/proc/sys/kernel/hostname`, `/etc/machine-id`
  or DMI product UUID, `/proc/sys/kernel/osrelease`, and available DMI firmware
  fields;
* RAM domains come from NUMA node `meminfo` files, with `/proc/meminfo` as the
  fallback;
* mounted filesystems are grouped by their physical block device, partitions
  are reduced to the physical device, and transport/duplex are derived from
  the physical path (`nvme`, `sas`, otherwise SATA); and
* non-loopback network interfaces come from `/sys/class/net` with address,
  ifindex, driver, firmware, wireless transport, and duplex identity.

Storage benchmark roots are selected only from explicitly configured roots,
`HOME`, the current directory, and mounted paths after canonicalization and a
write/delete access test. A root chooses where a temporary file is placed; it
does not override a discovered device identity or rate.

## Cache-aware end-to-end role

The default CLI derives an identity-named profile path from the discovery-only
`CacheIdentity` (`measured-v<schema>-<lowercase digest>.recipe-profile`) unless
`--profile` supplies an explicit path. `ExplicitPathProfileCache` then loads a
regular, private, identity-matching profile through `MeasuredProfileCodec` or
stores a newly measured profile atomically without replacing a different file.

On a cache hit, the engine's output is the decoded, fully validated profile and
the CLI writes a fresh active-native receipt from it. On a miss, all discovery,
measurement, topology/discovery construction, and final validation complete
before the cache store. The CLI prints the profile path, whether it used a
validated cache or fresh measurement, and the cache, topology, discovery,
machine, device, and directed-link identities/counts.

This makes the engine the one transition from current bare-metal state to the
immutable measured input consumed by planning and native preparation. Later
realization resolves the current inventory by the retained origin keys and
fails if the exhaustive current identities no longer match; it does not infer
or repair a profile produced by this engine.

## Failure surface and fail-closed invariants

The engine returns the following errors from the conditions it can observe:

| Error | Engine-visible causes |
| --- | --- |
| `Discovery` | Missing required host domains, duplicate or contradictory identities, unknown memory/interface references, non-asynchronous domains, zero queue/task limits, peer identity conflicts, backend discovery failure, or malformed local discovery data. |
| `IncompleteGpuEnumeration` | `GpuInventory::exhaustive` is false. |
| `Benchmark` | Backend benchmark failure, invalid plan/deadline, overflow or no-work accounting, invalid peer authentication/evidence, or a failed peer attempt converted by `into_measurement`. |
| `MissingMeasurement` | A RAM, storage, GPU, or peer property has `Estimated` or `Override` provenance where the engine requires `Measured`. |
| `IncompletePeerMeasurement` | A peer did not return measured outbound or inbound throughput. |
| `InvalidProfile` | Constructed topology, scheduling properties, discovery, or final profile validation failed. The engine wraps the failing validation text with the affected stage. |
| `Io` | A discovery or benchmark implementation reports a path operation failure through `ProbeError::io`. |
| `Cache` | A caller-supplied cache reports a cache or codec failure. The engine propagates it from `load` or `store`. |

The following invariants are deliberate and are not fallback opportunities:

* an incomplete GPU inventory never becomes a partial profile;
* missing or changed device identities never select by ordinal, name,
  capacity, or rate similarity;
* every physical link is represented by exactly two opposing directed links;
* full and half duplex resources are represented distinctly and validated;
* measured properties, not seed estimates, drive every production topology and
  discovery field;
* peer throughput requires authenticated endpoint evidence and exact bounded
  accounting;
* all identity and output ordering is deterministic for equal discovered input;
* the cache-aware path either returns a cache implementation's validated exact
  identity or performs a complete fresh probe; and
* no profile is returned from `probe` until topology, scheduling, discovery,
  origin, canonical-order, peer-link, and measured-provenance checks pass.

Several `expect` calls in topology construction are downstream of these
invariants. For example, peer directional rates are unwrapped only after
`validate_peer_measurement`, and map lookups use keys that `inspect` and the
measurement loops have already established. They are not alternate error
handling paths.

## State and ownership summary

The engine's in-memory lifecycle is intentionally short-lived:

```text
borrowed capability objects
  -> Inspection (owned normalized descriptors and cache identity)
  -> BenchmarkPlans (copyable bounds)
  -> Measurements (owned measured values)
  -> Topology + DiscoveryProfile + origins
  -> final MeasuredProfile
  -> optional caller-owned ProfileCache
```

Discovery and measurement side effects belong to the injected implementations.
The engine does not retain open files, native handles, peer cancellation tokens,
temporary benchmark buffers, or cache paths. A successful return is the only
engine-owned state that leaves the call, and that state is immutable by
convention through the public profile data structures.
