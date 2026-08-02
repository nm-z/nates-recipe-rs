# Local profile resolution

`probe/src/resolve.rs` is the bridge between an immutable measured profile and
the current bare-metal inventories. A measured profile gives the rest of Recipe
stable topology and capability IDs. A host discovery pass and a native GPU
discovery pass give the current process borrowed descriptors. The resolver joins
those two views by the stable origin keys retained in the profile. It returns
the measured `MachineId` and `DeviceId` values together with references to the
current RAM, storage, and GPU records. Native preparation can therefore reopen
the exact resources represented by the profile without guessing from an ordinal,
product name, capacity, or benchmark result.

The operation is intentionally fail-closed. The resolver does not repair a
profile, rewrite IDs, choose a close match, or silently ignore an inventory
entry. A missing key, an extra key, a duplicate live key, a non-exhaustive GPU
enumeration, a changed machine fingerprint, a missing GPU calculation
capability, or a changed GPU target returns `ProbeError::InvalidProfile`.
Callers must run a new `recipe probe` when the current machine no longer has the
same measured identity.

## Public shapes and ownership

The module defines three small borrowed associations and one aggregate:

| Type | Stored association | Borrowed input | Accessors |
| --- | --- | --- | --- |
| `ResolvedRamDomain<'inventory>` | measured `DeviceId` plus one `&RamDomain` | `HostInventory::ram` | `device()`, `domain()` |
| `ResolvedStorageDomain<'inventory>` | measured `DeviceId` plus one `&StorageDomain` | `HostInventory::storage` | `device()`, `domain()` |
| `ResolvedGpuDevice<'inventory>` | measured `DeviceId` plus one `&GpuDescriptor` | `GpuInventory::devices` | `device()`, `descriptor()` |
| `ResolvedLocalInventory<'inventory>` | measured local `MachineId` and vectors of the three associations | the supplied host and GPU inventories | `machine()`, `ram()`, `storage()`, `gpu()` |

All four types are `Clone + Debug`; the per-device records are `Copy` because
they contain only a `DeviceId` and a shared reference. The result borrows the
inventories for the lifetime named by the call. It does not own or clone a
current descriptor, open a driver handle, allocate a buffer, benchmark, or
modify the profile or inventories. `ResolvedLocalInventory` is `Debug` and
owns only its vectors and the measured machine ID.

`probe/src/lib.rs` exposes the module and re-exports these types, so the
resolver is part of the public `recipe-probe` API. The measured side of the
join is `MeasuredProfile` in `probe/src/model.rs`:

```text
MeasuredProfile {
    schema,
    cache_identity,
    origins: MeasuredOrigins {
        machines: [MeasuredMachineOrigin { machine, fingerprint }],
        ram:     [MeasuredRamOrigin { device, key }],
        storage: [MeasuredStorageOrigin { device, key }],
        gpu:     [MeasuredGpuOrigin { device, key }],
    },
    benchmarks,
    peer_benchmarks,
    topology,
    discovery,
}
```

The resolver uses `schema`, `origins`, `topology`, and `discovery`. Benchmark
metadata and peer records are not used to select a local object, but they are
covered by the `validate_profile` call at the start of resolution. The profile
contains local and optional peer machines. Only entries whose topology object
belongs to the selected local machine become local resolved entries. A peer RAM
origin can remain in the profile without appearing in the returned local RAM
slice.

The live host side is `HostInventory` (`probe/src/model.rs:74-79`):

* `machine: MachineFingerprint` contains hostname, stable ID, runtime ABI, and
  firmware.
* `ram: Vec<RamDomain>` contains a stable `key`, capacity hint, link identity,
  and maximum in-flight transfers.
* `storage: Vec<StorageDomain>` contains a stable `key`, benchmark root,
  host-memory key, transport and duplex properties, and I/O limits.
* `network` is present for discovery and cache identity, but local resolution
  does not return or compare network interfaces.

The live GPU side is `GpuInventory` (`probe/src/model.rs:112-116`):

* `exhaustive` states whether the native backend proved that every visible GPU
  was enumerated.
* Each `GpuDescriptor` has a stable `key`, `host_memory_key`, `target`, native
  capacity and identity fields, transfer limits, and calculation capabilities.

The resolver reads only the GPU key, host-memory key, and target after the
profile-wide validation. Other descriptor fields still participate in current
cache identity computation before a cached profile is selected, but they are
not independently compared by `resolve_gpu`.

## Entry point and state transitions

`MeasuredProfile::resolve_local_inventory` (`probe/src/resolve.rs:76-114`) is
the only public operation in this module. Its transitions are ordered and
short-circuit on the first error:

```text
profile + HostInventory + GpuInventory
        |
        v
validate_profile(profile)
        |
        +-- invalid persisted profile -> InvalidProfile
        v
require gpus.exhaustive
        |
        +-- false -> InvalidProfile("current GPU inventory is not exhaustive")
        v
match the complete MachineFingerprint -> local MachineId
        |
        +-- no exact origin -> InvalidProfile
        v
resolve_ram(profile, host, local MachineId)
        |
        +-- duplicate, missing, extra, missing topology object, or wrong kind
        v
resolve_storage(profile, host, local MachineId, resolved RAM)
        |
        +-- duplicate, missing, extra, or host-memory reference not in RAM
        v
resolve_gpu(profile, gpus, local MachineId, resolved RAM)
        |
        +-- duplicate, missing, extra, host-memory, discovery, or target error
        v
ResolvedLocalInventory { machine, ram, storage, gpu }
```

The first `validate_profile` call is important. It proves the persisted
topology, discovery profile, and origin metadata are internally coherent before
the resolver uses `BTreeMap` construction and indexed lookups. It checks the
profile schema and cache schema, rejects a zero cache digest, validates bounded
benchmark metadata and peer evidence, validates topology structure and
schedulable measured properties, validates discovery against topology, and
checks origin references, canonical ordering, measured provenance, topology to
discovery property equality, and peer transport evidence
(`probe/src/codec.rs:121-156`). Any codec error is returned unchanged through
`ProbeResult`.

After that call, the resolver checks `GpuInventory::exhaustive`. Host discovery
is specified to return every usable RAM and mounted storage domain or an error,
but the resolver has no separate host `exhaustive` bit. The duplicate and exact
key checks below are the live host completeness guard.

The machine match scans `profile.origins.machines` and compares the entire
`MachineFingerprint` to `host.machine`, not just `stable_id`. On success it
keeps the profile's `MachineId`. The error text prints the current stable ID,
but hostname, runtime ABI, and firmware are also part of the equality test.

The three resource resolvers run in RAM, storage, GPU order. Storage and GPU
resolution receive the already resolved RAM slice so their `host_memory_key`
references are checked against current RAM objects, not against an inferred
profile entry. No later step runs after an earlier error.

## RAM resolution

`resolve_ram` (`probe/src/resolve.rs:117-158`) performs an exact set join.

1. It walks `profile.origins.ram`. For each origin it asks
   `profile.topology.device(origin.device)`. An origin is retained in the
   expected map only when that topology device exists and its `machine` equals
   the selected local `MachineId`. The map entry is `(origin.key.as_str(),
   origin.device)`.
2. It walks `host.ram` and builds the live map `(domain.key.as_str(), domain)`.
3. `require_unique_live_keys("RAM", host.ram.len(), live.len())` rejects a
   duplicate current key before any lookup can overwrite another descriptor.
4. `require_same_keys("RAM", &expected, &live)` computes both directions of
   set difference. Any measured local key absent from the current host is
   reported as `missing`; any newly visible current key is `unexpected`.
5. In the expected map's sorted key order, it retrieves the live domain, looks
   up the retained topology device, and verifies `topology.kind ==
   DeviceKind::Ram`. It then constructs `ResolvedRamDomain { device,
   domain }`.

`BTreeMap` gives deterministic key order in the returned vector. It is not an
identity heuristic. Profile validation already rejects duplicate origin keys
within one machine and unknown or mis-typed origin devices; the explicit kind
check keeps the resolution boundary readable and makes a malformed topology
failure local to the RAM path.

The live `RamDomain::key` is the only selector. Capacity, `capacity_hint`, link
identity, and transfer-lane count never select a RAM device. `device` in the
result is the topology ID retained by the profile, while `domain` is the current
host object that will be used for realization.

## Storage resolution

`resolve_storage` (`probe/src/resolve.rs:160-193`) repeats the same expected and
live key-set join for `profile.origins.storage` and `host.storage`:

1. Only storage origins whose topology device belongs to the selected local
   machine enter `expected`.
2. Live storage descriptors are indexed by their stable `StorageDomain::key`.
3. Duplicate live keys and missing or unexpected keys fail before indexing.
4. For each sorted key, `require_host_memory("storage", key,
   &domain.host_memory_key, ram)` verifies that the current storage descriptor
   refers to one of the resolved current RAM keys.
5. The result keeps the profile's storage `DeviceId` and borrows the current
   `StorageDomain`.

The host-memory check compares `Label` values against
`resolved.domain.key`. Thus it rejects a storage descriptor that names a RAM
key absent from the measured local machine, even if the profile's old storage
origin itself was valid. It does not compare the storage's capacity, path,
driver, firmware, transport, duplex, or asynchronous flags here. Those fields
are part of discovery and current cache identity upstream; they are not
fallback selectors at realization time.

## GPU resolution

`resolve_gpu` (`probe/src/resolve.rs:195-248`) performs the same local origin
and exact key-set join against `GpuInventory::devices`, then applies the GPU
specific checks:

1. Expected entries come from `profile.origins.gpu` whose topology device is on
   the selected local machine. The key is the retained
   `GpuDescriptor::key`; the value is its measured `DeviceId`.
2. Live entries are indexed by current descriptor key. Duplicate, missing, and
   unexpected keys fail. This check is in addition to the earlier exhaustive
   flag, so an exhaustive enumeration with a changed device set still fails.
3. `require_host_memory("GPU", key, &descriptor.host_memory_key, ram)` requires
   the GPU's current host-memory key to be one of the resolved current RAM
   domains.
4. `profile.discovery.device(device)` must exist. The profile codec normally
   guarantees this, but the resolver reports a local mismatch if it does not.
5. The retained discovered device must have a `calculation` capability. A
   transfer-only or otherwise incomplete profile cannot produce a resolved GPU.
6. The retained calculation target must equal the current descriptor's
   `TargetIdentity` exactly. A changed backend, architecture, or ABI is a
   mismatch even when the key remains the same.
7. The result stores the measured `DeviceId` and borrows the current descriptor.

The target comparison is the resolver's direct guard for native compilation
identity. The descriptor key is still the primary physical identity. No
capacity, product name, ordinal, PCI ordering, measured rate, or similarity
fallback is attempted. CUDA and HSA discovery construct keys from backend UUID
and canonical PCI identity, and construct targets from current backend and
architecture data (`native-probe/src/cuda.rs:132-171`,
`native-probe/src/hsa.rs:238-253`). A changed target therefore cannot be
silently attached to an old measured GPU ID.

## Shared helpers and error surface

The helper functions at the end of `resolve.rs` keep all mismatch errors in the
`InvalidProfile` branch of `ProbeError`:

* `profile_mismatch(detail)` wraps a detail string in
  `ProbeError::InvalidProfile`.
* `require_unique_live_keys(kind, original_count, unique_count)` compares the
  vector length with the map length. A shorter map means at least one duplicate
  stable key, and returns `current {kind} inventory contains duplicate stable
  keys`.
* `require_same_keys(kind, expected, live)` computes sorted `missing` and
  `unexpected` vectors. It returns
  `current {kind} origins differ from the measured profile: missing=...,` and
  `unexpected=...` when either side differs.
* `require_host_memory(kind, key, host_memory_key, ram)` searches the resolved
  RAM slice by current key. It returns
  `{kind} origin {key} references current RAM key {host_memory_key}, which is
  absent from the measured machine` when the relationship is broken.

The entry point adds these specific errors:

| Condition | Detail returned by the resolver |
| --- | --- |
| Current GPU enumeration did not prove completeness | `current GPU inventory is not exhaustive` |
| No origin has the exact current machine fingerprint | `current machine fingerprint <stable_id> has no exact profile origin` |
| Local RAM topology object disappeared | `RAM device <id> disappeared from topology` |
| RAM origin points at a non-RAM topology object | `RAM origin <key> resolves to <kind> device <id>` |
| Current storage or GPU key set differs | `current storage/GPU origins differ from the measured profile: ...` |
| Current storage or GPU points at absent RAM | helper message above |
| Retained GPU capability is absent | `GPU origin <key> resolves to device <id> without discovery capability` or `...without calculation capability` |
| Current GPU target differs | `GPU origin <key> target changed from <old> to <new>` |

`validate_profile` can return any earlier codec or core validation detail,
including unsupported schema, zero cache identity, invalid topology or
discovery identity, unschedulable or estimated properties, duplicate or missing
origins, non-canonical order, non-measured provenance, or topology and
discovery property disagreement. The resolver does not convert those errors
into a second error type.

The error type is intentionally not a cache miss signal. A stale or malformed
persisted file is rejected by the cache and codec layers before this function is
called; a changed live machine is an invalid profile for the current realization
and must be reprobed.

## Identity and invariant ledger

The following invariants explain why the join is exact:

* **Profile IDs are authoritative.** `DeviceId` and `MachineId` in the result
  always come from profile origins. The resolver never allocates a new ID.
* **Discovery keys are authoritative selectors.** RAM, storage, and GPU keys are
  retained from discovery, not derived from capacity, rates, names, or position.
  Keys are scoped by topology machine. The codec enforces uniqueness per kind
  and machine (`probe/src/codec.rs:282-452`).
* **The machine match is full-fingerprint equality.** Hostname, stable ID,
  runtime ABI, and firmware all participate in selecting the profile machine.
* **The local GPU set is complete.** The `exhaustive` flag must be true and the
  current key set must equal the profile's local key set. A partial diagnostic
  backend cannot realize a profile.
* **RAM relationships are current.** Storage and GPU descriptors must reference
  keys in the resolved current RAM slice. This protects the host-memory graph
  used by native bindings.
* **GPU calculation identity is current.** The profile's retained discovery
  capability and target must exist and the target must equal the current native
  target. A transfer-only or differently compiled GPU is not interchangeable.
* **The result remains borrowed.** The lifetime ties each resolved descriptor to
  the exact inventory used for the check. Downstream code must finish native
  reopening while those inventories and their owning probe/runtime remain alive.
* **Ordering is deterministic.** Expected and live maps are `BTreeMap`s, so the
  output order is stable by key. This is ordering for reproducibility, not a
  substitute for identity.
* **No mutation occurs.** The profile and inventories are shared references;
  the function performs validation and constructs a new association value only.

One subtle boundary is worth preserving: `resolve_local_inventory` checks a
current GPU target directly, but it does not recompute `CacheIdentity` or
compare every current descriptor field to the profile's hashed inputs. That
broader invalidation decision belongs to `ProbeEngine::current_cache_identity`
and the cache path. The resolver's job is the final key, relationship, and
native-target join after a profile has already been selected.

## How a measured profile supplies the resolver

### Seed and discovery inputs

`SeedContract` is a theoretical probe contract, not a device inventory. Its
parser requires schema 1 and kind `probe-seed-estimates`, all discovery and
benchmark policy gates enabled, every identity invalidation facet, and explicit
transport duplex/async declarations (`probe/src/seed.rs:62-246`). It contains
estimates such as RAM, disk, GPU, and network capacities and rates. The profile
resolver never reads those estimates directly.

`ProbeEngine::inspect` (`probe/src/engine.rs:163-201`) consumes the seed and
live discovery providers in this order:

1. `HostDiscovery::discover_host` returns a `HostInventory`; `normalize_host`
   sorts RAM, storage, and network by key, requires RAM and mounted storage,
   rejects duplicate keys, checks storage host-memory references, and requires
   asynchronous storage submission.
2. `GpuDiscovery::discover_all` returns `GpuInventory`. The engine rejects
   `exhaustive == false`, rejects an empty device list, sorts devices by key,
   and checks unique GPU keys, host-memory references, asynchronous submission,
   nonzero submission queues, and nonzero concurrent-task capacity.
3. Peer descriptors are collected and sorted if peer sessions were supplied;
   their machine and local-memory/interface references are validated.
4. `build_cache_identity` hashes the seed, complete host fingerprint and
   domains, every GPU identity and capability field, and peer descriptors into
   `CacheIdentity { schema: PROFILE_SCHEMA, digest }`.

The cache digest includes the exact fields that make a current inventory or
probe contract materially different. For GPUs this includes the key, target,
driver, runtime ABI, firmware, link, toolchain name/version/digest, queue and
task limits, workgroup limits, shared memory, transfer overlap, duplex, and
in-flight lanes (`probe/src/engine.rs:575-668`). The resolver itself receives
the already discovered values, not this digest.

### Fresh profile construction

`ProbeEngine::probe` (`probe/src/engine.rs:63-161`) follows inspection with
bounded benchmarks. `BenchmarkPlans::from_seed` derives bounded RAM, storage,
GPU, and network plans from seed estimates, clamps their byte sizes to the
engine's minimum and maximum, and records the resulting plans as benchmark
metadata. The seed values bound work; they do not become production rates.

The engine measures every local RAM, storage, and GPU key and every peer session,
requiring measured provenance and complete peer evidence. It then:

1. hashes current measurements with the cache identity into a topology digest;
2. assigns deterministic IDs starting with local `MachineId(1)`, then local RAM,
   storage, and GPU `DeviceId`s, followed by peer RAM and peer machine IDs;
3. builds and validates the `Topology` and its measured scheduling properties;
4. builds `MeasuredOrigins` from the exact host/GPU keys and peer remote-memory
   keys;
5. hashes the same measured values into a discovery digest and builds and
   validates `DiscoveryProfile`;
6. constructs `MeasuredProfile` with schema 7, cache identity, origins,
   benchmark metadata, peer records, topology, and discovery;
7. runs `codec::validate_profile` once more before returning it.

`build_origins` is the pairing the resolver later consumes
(`probe/src/engine.rs:846-899`). Local RAM origins retain each
`RamDomain::key`, storage origins retain each `StorageDomain::key`, and GPU
origins retain each `GpuDescriptor::key`, all next to the assigned topology
ID. Every topology machine receives a `MachineFingerprint`. The profile schema
contract requires one origin for every topology machine and every RAM, storage,
or GPU device, stable machine IDs to be unique, and kind-specific keys to be
unique within a machine (`probe/PROFILE_SCHEMA.md` and
`probe/src/codec.rs:282-452`).

## Cache and seed paths around resolution

The cache path decides whether a profile can be reused; resolution decides
whether that selected profile can be realized against live inventories.

### CLI `recipe probe`

`src/cli.rs:876-947` parses either an explicitly supplied contract or the
embedded `topology/contract.toml`, discovers the host once to choose the native
probe's host-memory key, constructs `NativeGpuProbe`, and creates
`ProbeEngine`. It asks `engine.current_cache_identity(&seed, &peers)` before
choosing the default filename
`measured-v<schema>-<lowercase-digest>.recipe-profile`.

It then creates `ExplicitPathProfileCache` and calls
`engine.load_or_probe_and_store`. That engine method (`probe/src/engine.rs:212-237`)
computes the current discovery-only identity, calls `ProfileCache::load`, and
returns the exact decoded profile on a hit. On `None` it runs the complete fresh
probe and stores only the validated profile. The CLI writes its active native
receipt and prints profile, source, cache identity, topology identity,
discovery identity, and object counts. Resolution is not part of the probe
measurement path itself; it runs later when preparation reopens the selected
profile.

### `ExplicitPathProfileCache`

`probe/src/cache.rs:25-196` is an opt-in single-file implementation of
`ProfileCache`. It requires an absolute file path and a canonical, real,
effective-user-owned parent directory with no group or other permissions. A
load rejects symlinks, non-regular files, insecure ownership or modes, races
between metadata and open, oversized files, codec checksum/magic/schema errors,
and a profile whose embedded `cache_identity` differs from the requested
identity. A missing target returns `Ok(None)` to the engine.

Store encodes and validates the profile, refuses to replace a different existing
profile, writes a private same-directory temporary file, syncs it, installs it
with `renameat(..., NOREPLACE)`, and syncs the parent directory. A concurrent
writer is accepted only when its resulting profile is byte-for-byte equal. No
newest-file, ordinal, or alternate-path fallback exists.

The codec's decode path validates origins and all profile invariants before a
cache hit can reach the resolver. Therefore a resolver caller can rely on the
profile's retained IDs and key uniqueness, while still checking live current
state.

### Native preparation cache path

`src/native_prepare.rs:260-287` can load one explicitly named measured profile
and immediately build a native target plan. The filename is parsed as
`measured-v<schema>-<64 lowercase hex>.recipe-profile`; the embedded identity
must match the filename and the secure cache load must succeed. The plan build
eventually enters `with_native_preparation_from_probe`, which invokes the
resolver.

The normal current path is
`with_current_native_preparation` (`src/native_prepare.rs:368-410`):

1. `current_native_inputs` rediscover the host and determine the exact current
   profile path and identity from the active receipt or a fresh discovery-only
   cache identity computation.
2. The exact path and identity are loaded through `ExplicitPathProfileCache`.
3. A thread-local `NativeGpuProbe` is reused only when its complete native
   configuration is unchanged.
4. `with_native_preparation_from_probe` calls native GPU discovery and then
   `profile.resolve_local_inventory(host, &inventory)`.

The current path never picks the newest profile file. It either loads the
identity-derived file or returns a profile-not-found, cache, discovery, or
invalid-profile error.

## Downstream callers and complete realization path

### Native GPU bindings

`native-probe/src/bindings.rs:120-234` calls the resolver from
`with_native_execution_bindings` after `NativeGpuProbe::discover_all`. It takes
the returned GPU associations, partitions them by exact target backend into
CUDA and HSA expected maps, and reopens native contexts or sessions. It checks
that every reopened backend key is expected, no duplicate is reopened, and all
expected keys were seen. `host_backend_config_from_inventory` consumes the
resolved RAM IDs and storage roots to build deterministic run-scoped host
bindings without performing I/O during construction.

The resolver's borrowed lifetime is what keeps current descriptors tied to the
probe inventory while these native bindings are built. The native probe owns
the concrete backend state, including the retained ROCr runtime, until the
callback and every borrowed binding have been destroyed.

### Root native preparation

`src/native_prepare.rs:327-366` performs the direct preparation sequence:

```text
NativeGpuProbe::discover_all
    -> MeasuredProfile::resolve_local_inventory
    -> owned_host_plan + owned_gpu_inventory
    -> require complete local calculation-device scope
    -> with_native_execution_bindings (resolver is checked again)
    -> build_scope
    -> NativePreparationScope
```

`owned_host_plan` copies only resolved machine, RAM IDs, storage IDs, and
benchmark roots. `owned_gpu_inventory` copies the resolved device IDs and
descriptors, sorted by profile `DeviceId`. `build_scope` then requires the
native binding device set to equal the measured local GPU set, creates exact
CUDA/HSA target specifications, deduplicates equivalent compiler targets, and
returns a scope whose callback cannot outlive the native handles. If any
resolver or reopening invariant fails, no partial preparation scope is exposed.

`load_native_preparation` uses this path after an explicit cache load, while
`build_native_target_plan` uses it when callers need owned build specifications
after the borrowed native scope is destroyed.

### Training and inference

The public native execution paths in `src/training.rs:1278-1337` and
`src/inference.rs:602-659` call `with_current_native_preparation`. Their
callbacks consume the resolved preparation scope to:

* derive runtime tuning from the profile and local machine;
* create run-scoped host backend resources from resolved RAM and storage;
* take exact CUDA/HSA bindings and build the staged cross-backend bridge;
* construct the native executor driver and deferred compiler from measured GPU
  targets and toolchains;
* prepare and execute the complete training or inference program.

Training additionally checks any resumed native bundle against current topology,
discovery, target, and toolchain identities before execution. Inference reports
the exact current GPU origin keys used by its preparation scope. The resolver is
therefore the point where a cached measured profile becomes a current-machine
execution input, before scheduling, artifact realization, or model work.

`prepare` (`prepare/src/lib.rs:315-338`) separately validates a supplied
`MeasuredProfile` and consumes its topology and discovery for planning. It does
not invent local associations. Any path that must reopen actual local devices
uses the resolver through native preparation first.

## End-to-end role

The complete profile lifecycle is:

```text
SeedContract (bounds and invalidation policy only)
    |
    v
HostDiscovery + exhaustive GpuDiscovery + optional PeerSession descriptors
    |
    v
ProbeEngine::current_cache_identity
    |                         \
    | cache hit                \ cache miss
    v                           v
secure codec load       bounded benchmarks -> topology/discovery/origins
    |                           |
    +-------------+-------------+
                  v
          validated MeasuredProfile
                  |
                  v
current host + current exhaustive GPU inventory
                  |
                  v
       resolve_local_inventory
                  |
                  v
measured IDs + borrowed current descriptors
                  |
                  v
exact native reopening and preparation
                  |
                  v
production scheduling and execution
```

The seed controls how a fresh profile is measured and which identity facets
invalidate its cache. The engine records measured values and stable origin
metadata. The cache preserves that exact validated object. The resolver proves
that the current machine still presents the same local identity set and native
target identity, then hands downstream code the measured IDs and current
objects needed to construct real backend resources. Any mismatch remains
visible as an error and directs the caller back to a fresh `recipe probe`.
