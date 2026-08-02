# Native bindings

<!--
Intent: specify the exact identity, realization, lifetime, and consumer
contract implemented by native-probe/src/bindings.rs. This module does not
serialize a runtime handle, select a fallback device, or own a production
execution loop. It reopens the measured GPU origins, creates one scoped CUDA
or HSA binding per current GPU, and lends those bindings to one callback.
-->

`native-probe/src/bindings.rs` is the preparation boundary between a validated
`MeasuredProfile` and the native executor. It has two products:

* `host_backend_config_from_inventory` converts the exact resolved RAM and
  storage origins into a host backend configuration. It derives deterministic,
  run-scoped disk arena paths but performs no I/O.
* `with_native_execution_bindings` reopens every current GPU by the stable key
  retained by `recipe probe`, realizes the native context or session, and lends
  `CudaBinding` and `HsaBinding` values to one higher-ranked callback.

The function never chooses an ordinal, product name, capacity, performance
nearest neighbour, newest cache file, or other fallback. A missing measured
origin, an extra current GPU, a changed target, an unavailable native runtime,
or an ambiguous HSA host allocator is a failure.

## Intent and parseable contract

The following is the contract implemented by this module. The field names are
Rust field and accessor names, not a second wire format.

```text
document: recipe_native_probe.bindings
source: native-probe/src/bindings.rs
kind: scoped-exact-native-binding
authority:
  - native-probe/src/bindings.rs
  - native-probe/src/native.rs
  - native-probe/src/cuda.rs
  - native-probe/src/hsa.rs
  - probe/src/resolve.rs
  - probe/src/codec.rs
  - native-executor/src/cuda.rs
  - native-executor/src/hsa.rs
  - src/native_prepare.rs

supported_backends:
  cuda:
    descriptor_target_backend: nvidia-cuda-driver
    binding_type: recipe_native_executor::CudaBinding
    live_owner: recipe_cuda::Context
  hsa:
    descriptor_target_backend: amd-rocr-hsa
    binding_type: recipe_native_executor::HsaBinding
    live_owner: recipe_hsa::Session

NativeExecutionBindings<'cuda, 'hsa>:
  fields:
    machine: MachineId
    cuda: Vec<CudaBinding<'cuda>>
    hsa: Vec<HsaBinding<'hsa>>
  construction: private; only with_native_execution_bindings constructs it
  accessors: [machine, cuda, hsa, into_parts]
  lifetime_rule: every context/session/allocator borrow ends with the callback
  serialization: none; contexts, sessions, and bindings are preparation state

host_backend_config_from_inventory(inventory, run, worker_threads,
                                   staging_bytes_per_worker):
  ram: one HostDeviceBinding::Ram per resolved RAM origin
  storage: one HostDeviceBinding::Disk per resolved storage origin
  disk_path: <benchmark_root>/.recipe-run-<run>-device-<DeviceId>-arena
  side_effects: none during construction
  later_side_effect: HostBackend creates disk files with create_new

with_native_execution_bindings(probe, profile, host, operation):
  input_preconditions:
    - probe is the NativeGpuProbe that owns the current backend state
    - profile is a validate_profile-approved measured profile
    - host is the current HostInventory from exhaustive host discovery
  sequence:
    - probe.discover_all
    - profile.resolve_local_inventory(host, inventory)
    - partition_expected by descriptor.target.backend
    - reopen expected CUDA origins, if any
    - reopen expected HSA origins, if any
    - invoke operation exactly once with NativeExecutionBindings
  success: operation result is returned unchanged
  failure: ProbeError::Discovery or the upstream profile/discovery error
  fallback: none for an expected origin; empty backend is allowed only when
            the current probe proves no origin for that backend exists
```

The source map is deliberately local and precise:

| Source range | Contract surface |
| --- | --- |
| `bindings.rs:22-47` | `NativeExecutionBindings` storage, lifetimes, and accessors. |
| `bindings.rs:49-85` | Private expected and realized records, plus HSA allocator errors. |
| `bindings.rs:87-118` | Pure host binding and deterministic disk path construction. |
| `bindings.rs:120-234` | Complete current-inventory resolution and callback scope. |
| `bindings.rs:236-266` | Backend partition and display-connector observation. |
| `bindings.rs:268-317` | CUDA reopen, deployment identity, context creation, and set check. |
| `bindings.rs:319-414` | HSA agent filtering, allocator selection, session creation, and set check. |
| `bindings.rs:416-482` | Host allocator predicate, exact NUMA selection, and error conversion. |

## Identity chain

There are three different identities in this boundary. They must not be
collapsed:

```text
current native descriptor key (Label/String)
  -> measured origin key (MeasuredGpuOrigin.key)
  -> topology DeviceId (MeasuredGpuOrigin.device)
  -> native binding device field (CudaBinding.device or HsaBinding.device)
```

`GpuDescriptor::key` is the stable discovery identity. `DeviceId` is a Recipe
topology identifier assigned by the probe engine and is not a CUDA ordinal or
an HSA agent handle. `DeploymentIdentity` is a current CUDA deployment
snapshot used for artifact compatibility. `target_id` on `HsaBinding` is the
current exact AMDGPU ISA string used to build and validate HSA artifacts.

### Stable GPU origins

`NativeGpuProbe::discover_all` calls both native backend adapters, joins their
`GpuDescriptor` values, sorts by `descriptor.key`, and rejects adjacent
duplicate keys (`native-probe/src/native.rs:245-264`). A normal
`NativeGpuProbe::new` marks the inventory exhaustive. The deliberately
single-backend diagnostic constructors mark it non-exhaustive, so
`MeasuredProfile::resolve_local_inventory` rejects such an inventory before a
binding can be realized.

The native library selector is part of this identity boundary. Each configured
candidate must be absolute. Missing candidates are skipped, canonical duplicate
paths are skipped, and the first existing canonical regular file is selected
after its bytes are hashed. A present but invalid candidate, canonicalization
failure, read failure, or a hardware vendor with no existing configured
candidate is a discovery error (`native-probe/src/identity.rs:22-73`). The
selected path and digest are incorporated into the descriptor runtime or
toolchain identity. This is why a changed library binary cannot silently reuse
an old measured origin.

The measurement producer applies a stricter whole-record check than the
binding reopen map. `NativeGpuProbe::benchmark_gpu` rediscovers each backend,
requires exactly one backend whose fresh `GpuDescriptor` equals the requested
descriptor, and rejects multiple exact owners or no owner
(`native-probe/src/native.rs:267-298`). CUDA and HSA benchmark adapters use the
same current descriptor construction before allocating or submitting their
bounded native benchmark. Binding realization then uses the profile key and
current runtime checks to make the preparation handoff, while the normal
identity-named cache path has already hashed the complete descriptor record.

The two native descriptor producers use exact PCI and runtime identities:

| Backend | Stable key | Target fields | Other binding-relevant descriptor data |
| --- | --- | --- | --- |
| CUDA Driver | `cuda:{DeviceUuid}@{domain:04x}:{bus:02x}:{device:02x}.{function}` | backend `nvidia-cuda-driver`, architecture `sm_{major}{minor}`, ABI `elf64-cubin` | current Driver version and capabilities, device UUID, PCI surface digests, PTX toolchain identity, queue ceiling, subgroup/workgroup limits, shared memory, and transfer overlap. |
| ROCr/HSA | `hsa:{AgentUuid}@{domain:04x}:{bus:02x}:{device:02x}.{function}` | backend `amd-rocr-hsa`, architecture exact `IsaTarget`, ABI `elf64-amdgpu-code-object-v{version}` | ROCr and AMD extension versions, agent UUID and NUMA identity, PCI surface digests, exact ISA/toolchain, queue limits, wavefront/workgroup limits, LDS capacity, and SDMA overlap. |

#### CUDA descriptor production

`CudaBackend::descriptor` (`native-probe/src/cuda.rs:109-208`) parses the
Driver's textual PCI bus ID and checks domain, bus, and device against Driver
attributes. The function number must be in `0..=7`. The parsed values are
rendered in one lowercase, zero-padded BDF for sysfs lookup and for the stable
key. The descriptor then hashes the current PCI driver, firmware, and link
surfaces, incorporates the pinned Driver library digest and pinned toolchain,
and records the Driver-reported target and queue/capability limits.

`CudaBackend::matching_device` is the benchmark reopen path
(`native-probe/src/cuda.rs:210-234`). It compares complete `GpuDescriptor`
values and rejects zero or multiple matches. `realize_cuda` uses the stable
key map after `resolve_local_inventory` has already checked the profile target;
it never falls back to the ordinal if a key is absent.

#### HSA descriptor production

`HsaBackend::descriptor` (`native-probe/src/hsa.rs:153-294`) returns `None` for
non-GPU agents. A GPU must advertise kernel dispatch, a value UUID, an exact
PCI address, AMD capability properties, queue limits, a usable exact target,
ISA/workgroup limits, a KFD node, and a pinned toolchain. `exact_target`
(`native-probe/src/hsa.rs:633-683`) accepts one non-generic AMD target, or one
unique generic target when no specific target exists. Multiple specific or
ambiguous generic targets are rejected. The key retains the complete agent
UUID and canonical PCI address.

The HSA descriptor contains the target and queue values later copied into an
`HsaBinding`. The binding also recomputes `exact_target` from the same current
agent before session creation, so a target string is never inferred from a
product name or ordinal.

### Profile association

`MeasuredProfile::resolve_local_inventory` is the profile-to-live association
used by `with_native_execution_bindings` (`probe/src/resolve.rs:76-114`). It:

1. validates the complete measured profile;
2. requires `GpuInventory.exhaustive == true`;
3. matches the current `MachineFingerprint` exactly to one measured machine;
4. requires current RAM, storage, and GPU key sets to equal the profile's
   machine-scoped origin key sets, with no duplicate live key;
5. verifies topology kinds and every GPU or storage host-memory key; and
6. requires each current GPU calculation target to equal the target retained
   in `profile.discovery`.

Malformed or internally inconsistent profile data fails the initial
`validate_profile` call as `ProbeError::Cache` with a `codec:` detail. Missing
keys, unexpected keys, duplicate live keys, a changed machine, a changed
target, a missing discovery capability, or a missing host-memory origin are
`ProbeError::InvalidProfile`. Capacity, product name, ordinal, benchmark
similarity, and performance are not selectors. The resolver does not compare
every `GpuDescriptor` field to a serialized descriptor, because the measured
profile stores the origin key and measured topology/discovery records rather
than a `GpuDescriptor` object.

## Descriptor and profile serialization

Native bindings are not serialized. `NativeExecutionBindings`,
`CudaBinding`, `HsaBinding`, `Context`, `Session`, `DeploymentIdentity`,
`HostBackendConfig`, and `DiskFileSpec` have no codec implementation in this
workspace. Their vectors, references, native handles, host allocator indices,
display connector count, queue packet count, and current deployment snapshot
exist only during preparation.

The persisted identity that leads to a later binding is the measured profile:

```text
MeasuredGpuOrigin {
    device: DeviceId,
    key: Label,
}
```

`ProbeEngine::build_origins` copies each current descriptor key into
`MeasuredGpuOrigin.key` and pairs it with the newly assigned topology
`DeviceId` (`probe/src/engine.rs:846-898`). `MeasuredProfileCodec` encodes
`origins.gpu` as a bounded count followed by each device ID and label
(`probe/src/codec.rs:798-860`). The same codec encodes the measured discovery
device records, including the calculation target backend, architecture, and
ABI (`probe/src/codec.rs:1166-1279`). It never encodes a native context or
session.

Those persisted discovery records also carry the measured
`maximum_submission_queues` and calculation limits. `realize_cuda` and
`realize_hsa` deliberately copy queue limits from the current descriptor into
the live binding rather than trusting the serialized number. The downstream
native executor compares the current binding limit with the finalized request;
if the current limit is too low, realization fails, and no stale serialized
limit is used.

The profile binary is versioned and checksummed, not a binding snapshot:

```text
RECIPEPROFILE\0\0\0       # 16-byte magic
u32 codec_schema           # currently PROFILE_CODEC_SCHEMA = 7
u32 profile.schema
u32 cache.schema
Digest cache.digest
origins, benchmark metadata, peers, topology, discovery
SHA-256(payload)           # 32-byte trailer
```

Decode rejects an oversized or truncated buffer, checksum or magic mismatch,
an unsupported codec schema, trailing payload bytes, and any profile that
fails `validate_profile`. Encode validates before writing and enforces the
256 MiB profile limit. The origin validator requires one origin for every
topology machine and every RAM, disk, and GPU-memory device, unique
machine-scoped origin keys, matching device kinds, and strict canonical ID
order (`probe/src/codec.rs:121-156,282-529`).

The identity-named cache path is checked by the preparation caller, not by
`bindings.rs`. `src/native_prepare.rs::with_current_native_preparation`
loads the profile whose filename and embedded `CacheIdentity` match the
current discovery-only identity, then calls this module. The direct
`with_native_execution_bindings` entry point assumes its `MeasuredProfile`
argument has already crossed the profile/cache boundary and starts by
rediscovering current hardware.

The current cache identity includes every `GpuDescriptor` identity and
capability field, including key, name, capacity, host-memory key, target,
driver, runtime ABI, firmware, link, transport, toolchain, duplex, lane
limits, queue/task limits, subgroup/workgroup limits, shared-memory limit, and
transfer-overlap flag (`probe/src/engine.rs:575-668`). Thus a normal
identity-named preparation call notices descriptor drift before this binding
scope is entered. The binding function itself still performs its own exact
key, target, backend, runtime, and native handle checks.

## Host backend configuration

`host_backend_config_from_inventory` (`bindings.rs:87-118`) consumes only the
resolved current inventory. For each `ResolvedRamDomain`, it emits:

```text
HostDeviceBinding::Ram { device: resolved.device() }
```

For each `ResolvedStorageDomain`, it emits:

```text
stem = .recipe-run-{run}-device-{resolved.device()}
path = resolved.domain().benchmark_root / "{stem}-arena"
HostDeviceBinding::Disk {
    device: resolved.device(),
    arena: DiskFileSpec::new(path),
}
```

The RAM and storage iteration order is the order of the resolver's
machine-scoped `BTreeMap` keys. `HostBackendConfig::new` then enforces a
nonzero worker count, nonzero staging capacity, unique `DeviceId` values, and
globally unique disk paths (`host/src/backend.rs:55-83,1286-1303`). A bad
`DiskFileSpec` path or host configuration is a `recipe_host::Error`, not a
`ProbeError`.

This function performs no filesystem operation. Later `HostBackend` arena
realization calls `OpenOptions::create_new`, fallocate, and sync for each disk
arena (`host/src/arena.rs:196-223`). An existing run-scoped path therefore
fails without overwrite. Disk backing removes its file on close or drop
(`host/src/arena.rs:79-85,225-233`). The path is a run resource, not a
serialized model or profile artifact.

`host_backend_config_from_inventory` is re-exported by
`native-probe/src/lib.rs`, but no current production call site invokes that
helper directly. The root preparation layer instead uses
`NativeHostPlan::backend_config` (`src/native_prepare.rs:141-163`), which
constructs the same RAM bindings and run-scoped disk paths from its owned
resolved plan. The two entry points have the same host backend contract; the
native binding callback itself is the production path that is currently
consumed by training and inference.

## Reopen algorithm

The complete successful path is:

```text
NativeGpuProbe::discover_all()
  -> GpuInventory { exhaustive, devices: Vec<GpuDescriptor> }
MeasuredProfile::resolve_local_inventory(host, inventory)
  -> ResolvedLocalInventory { machine, ram, storage, gpu }
partition_expected(probe, resolved.gpu())
  -> (BTreeMap<CUDA key, ExpectedGpu>, BTreeMap<HSA key, ExpectedGpu>)
realize_cuda(cuda_backend, expected_cuda)
  -> Vec<RealizedCuda>
realize_hsa(hsa_backend, system, agents, expected_hsa)
  -> RealizedHsaSet { host_allocators, devices }
construct CudaBinding and HsaBinding vectors
  -> operation(NativeExecutionBindings)
```

The two backend maps are keyed by the exact descriptor key. Each
`ExpectedGpu` retains the profile's `DeviceId`, the current key string, and a
fresh count of enabled DRM display connectors. The `DeviceId` is never parsed
from the key. `partition_expected` accepts only the two exact backend strings
and rejects duplicate keys in its destination map (`bindings.rs:236-266`).

`NativeGpuProbe::enabled_display_connectors` extracts the suffix after the
last `@`, requires a canonical 12-byte PCI BDF shape, and counts directories
whose connector state is exactly `enabled`; `disabled` contributes zero. A
missing DRM directory means zero. Invalid BDF text, missing PCI paths,
enumeration or read failures, invalid connector state, and a `u32` count
overflow are discovery failures (`native-probe/src/native.rs:106-219`). The
count is current reservation evidence and is not read from the profile.

`HsaBackend::with_runtime` applies the same absence rule to ROCr. It checks
the AMD PCI surface before opening the runtime, retains one `PinnedLibrary` and
one `Runtime` in the backend, and requires the selected path and digest to
remain equal on later calls. Once an AMD accelerator has been observed, a
runtime disappearance, library disappearance, or library identity change is a
hard discovery error, not a transition to an empty HSA vector
(`native-probe/src/hsa.rs:91-149`). The HSA sessions borrowed by this module
therefore always refer to the retained runtime state owned by the supplied
probe.

### Backend absence matrix

`NativeGpuProbe::new` always carries both backend adapters, but each adapter
may report no matching PCI accelerator. Diagnostic probes can carry only one
adapter. The binding boundary handles the cases as follows:

| Current probe state | Expected origins for backend | Result |
| --- | --- | --- |
| backend adapter is absent | empty | an empty binding vector is allowed |
| backend adapter is absent | nonempty | `ProbeError::Discovery`, measured origins exist but the backend is unavailable |
| HSA adapter exists but `with_runtime` finds no AMD accelerator | empty HSA set | callback is invoked with no HSA bindings |
| HSA adapter exists but `with_runtime` finds no AMD accelerator | nonempty HSA set | `ProbeError::Discovery`, no AMD runtime surface was reopened |
| runtime opens and exposes an unexpected GPU | any expected set | unexpected-device discovery error, never silently ignored |

If the HSA backend is present and its runtime closure returns an error, that
error is propagated. The empty-vector fallback is used only for the explicit
no-accelerator result and only when no HSA origin is expected.

## CUDA realization

`realize_cuda` (`bindings.rs:268-317`) opens the configured CUDA Driver
through `CudaBackend::open`. If no NVIDIA PCI accelerator exists, `open`
returns `None`; this is accepted only for an empty expected map. With hardware
present, a missing library, a load failure, or an exhaustive discovery failure
is fatal.

For every `DeviceInfo` in the current Driver discovery snapshot it:

1. builds a current `GpuDescriptor` from the same pinned library and
   discovery snapshot;
2. looks up that descriptor's key in `expected_cuda`; an unmeasured current
   device is an immediate discovery error;
3. rejects a duplicate key in the reopened Driver snapshot;
4. creates `DeploymentIdentity::from_discovery(&discovery, device)`;
5. creates a `recipe_cuda::Context` with `ContextFlags::default()` for that
   exact `DeviceInfo`; and
6. retains the profile `DeviceId`, current descriptor queue ceiling, and fresh
   display connector count in `RealizedCuda`.

`DeploymentIdentity::from_discovery` copies the current Driver version, device
UUID, compute capability, and resolved Driver symbol capabilities only when a
discovery record with the same ordinal and UUID exists (`cuda/src/artifact.rs:31-55`).
The context is created from that same `DeviceInfo`, so a context handle cannot
be paired with a different deployment record. Context creation validates and
balances the Driver context stack; failure is converted to
`ProbeError::Discovery` with the measured key.

`require_all_reopened("CUDA", expected, seen)` then rejects every expected key
that was not observed. The realized vector is sorted by `DeviceId`, not by
Driver ordinal or key, before bindings are constructed. Each binding receives
the borrowed context, a clone of the deployment identity, the current queue
ceiling, and the current display connector count.

CUDA reopening errors include:

```text
measured CUDA GPU origins exist but no NVIDIA Driver surface was reopened
reopened CUDA device <key> was not present in the measured profile
CUDA Driver reopened duplicate device <key>
CUDA deployment identity for <key> is not part of its discovery snapshot
create CUDA context for measured device <key>: <cuda error>
CUDA did not reopen measured GPU origins [<missing keys>]
```

The descriptor construction can additionally fail on malformed or
inconsistent PCI identity, missing PCI surface files, invalid target or
toolchain identity, or any Driver discovery value that cannot be represented.
Those errors are returned before a `CudaBinding` is made.

## HSA realization

`realize_hsa` (`bindings.rs:319-414`) runs inside the one retained ROCr
runtime owned by `HsaBackend::with_runtime`. It consumes the current
`DiscoveredAgent` vector and separates CPU agents from non-CPU agents. CPU
agents are retained only when `supports_host_allocation` is true:

```text
there exists a MemorySegment::Global pool with runtime_allocation != None
and KERNARG_INITIALIZATION
and there exists a MemorySegment::Global pool with runtime_allocation != None
and (FINE_GRAINED or EXTENDED_SCOPE_FINE_GRAINED)
```

The predicate intentionally tests the raw global flags through
`MemoryPoolFlags::contains`; it does not guess a pool or use capacity as a
fallback. Retained host allocators are sorted by the complete identity tuple
`(numa_node_id, uuid, name, vendor_name)`. Their sorted positions are the
indices stored in `RealizedHsa.host_allocator` and later used to borrow the
selected `DiscoveredAgent`.

Each non-CPU agent is passed to `HsaBackend::descriptor`:

* `None` for a DSP or AIE agent is skipped because it is not a GPU
  calculation device.
* A GPU descriptor with no expected key is an unexpected current GPU error.
* A repeated expected key is a duplicate ROCr device error.

For an expected GPU the function recomputes the exact target string, requires
an advertised queue, copies `minimum_packets` and `maximum_queues`, and
selects a host allocator by NUMA node. It then consumes that exact GPU agent
with `DiscoveredAgent::into_session`, which also checks that the runtime is
active, the agent is a GPU with kernel dispatch, and every ISA has an exact
AMDGPU target (`hsa/src/session.rs:145-199`). The result retains the profile
`DeviceId`, `Session`, allocator index, exact target string, HSA code-object
version from the configured backend, queue packet count, queue ceiling, and
display connector count.

### Exact NUMA allocator selection

`select_host_allocator` is intentionally total and fail-closed:

| Matching allocators | Selection |
| --- | --- |
| exactly one allocator with `numa_node == gpu_numa_node` | that sorted index |
| no same-NUMA allocator and exactly one retained allocator total | index `0` fallback |
| no retained allocator | `Missing` |
| no same-NUMA allocator and more than one retained allocator | `AmbiguousFallback { count }` |
| two or more same-NUMA allocators | `AmbiguousSameNuma { count }` |

There is no nearest-NUMA or first-match policy when the choice is ambiguous.
The errors become these discovery messages:

```text
measured HSA device <key> has no CPU agent with allocatable fine-grained and kernarg pools
measured HSA device <key> has <count> same-NUMA CPU host allocators; exact binding is ambiguous
measured HSA device <key> has no same-NUMA CPU host allocator and <count> fallback allocators; exact binding is ambiguous
```

After all agents are processed, `require_all_reopened("HSA", expected, seen)`
rejects missing measured keys and the device vector is sorted by `DeviceId`.
The returned allocator vector is kept beside the devices until every HSA
binding borrow and the callback have ended.

HSA reopening errors include runtime discovery failure, malformed or changed
agent descriptor identity, missing queue, allocator ambiguity, session
creation failure, an unexpected or duplicate GPU key, and a missing measured
GPU key. All are `ProbeError::Discovery` at this module boundary.

## Binding values and consumers

### `NativeExecutionBindings`

The constructor fields are private. The only public observations are:

```text
machine() -> MachineId
cuda() -> &[CudaBinding<'cuda>]
hsa() -> &[HsaBinding<'hsa>]
into_parts(self) -> (Vec<CudaBinding<'cuda>>, Vec<HsaBinding<'hsa>>)
```

The vectors are sorted by topology `DeviceId` because both realization helpers
sort their temporary records before constructing the bindings. The type is
`Clone`: cloning clones the binding references and metadata, not a CUDA
context or an HSA session. The lifetimes still prevent either clone from
outliving the callback's native owners.

### CUDA binding

`recipe_native_executor::CudaBinding` stores `DeviceId`, a borrowed
`Context`, `DeploymentIdentity`, maximum submission queues, and enabled display
connectors (`native-executor/src/cuda.rs:38-105`). Its public methods expose the
device, deployment, queue ceiling, connector count, and a current free-memory
observation. The context accessor is crate-private. The native executor uses
the context to create streams, events, pinned staging, device arenas, and
kernel modules. It validates every runtime CUDA artifact against the binding's
deployment identity, including target, driver range, and required Driver
symbols (`native-executor/src/cuda.rs:1744-1789` and
`cuda/src/artifact.rs:124-188`).
Before device resources are realized it also checks that the context's
discovered UUID equals `deployment.device_uuid` and that the context compute
capability equals `deployment.target` (`native-executor/src/cuda.rs:1633-1646`).

### HSA binding

`recipe_native_executor::HsaBinding` stores `DeviceId`, a borrowed GPU
`Session`, a borrowed selected CPU `DiscoveredAgent`, exact `target_id`, code
object version, queue packet count, maximum submission queues, and enabled
display connectors (`native-executor/src/hsa.rs:32-122`). Its public methods
expose the device, target, code-object version, queue values, connector count,
and current available memory. `allocate_host_fine` allocates through the exact
selected CPU allocator and grants the GPU session access. Session and
allocator accessors are crate-private.

The native HSA backend rechecks that the allocator is a CPU agent with an
allocatable kernarg pool and fine-grained pool, and that the session advertises
the binding's exact target (`native-executor/src/hsa.rs:1692-1738`). It uses
`queue_packets` when creating single-producer queues and uses the allocator
for kernarg, metric, and staging allocations. A binding that passed probe
realization can still fail later if the native executor observes a violated
session or allocator invariant; that later failure is not hidden by this
module.

The public `maximum_submission_queues()` accessors on both native binding
types have no in-workspace call site, but the native-executor modules access
the private field directly while realizing each device and pass it to
`ensure_submission_queue_capacity` (`native-executor/src/cuda.rs:1633-1691`,
`native-executor/src/hsa.rs:400-439`). Requested queue slots still come from
the finalized resource manifest. The binding layer does not use the value as a
queue-count fallback.

### Preparation and execution consumers

The root preparation boundary consumes the values as follows:

1. `src/native_prepare.rs::build_scope` first requires the binding machine to
   equal the resolved profile machine (`native_prepare.rs:543-571`). It indexes
   CUDA and HSA bindings by `DeviceId`, rejects duplicates, and requires the
   union of both maps to equal the measured local GPU set with no device in
   both classes.
2. For a CUDA descriptor it consumes `binding.deployment()` to derive the
   exact SM target and Driver policy. For HSA it requires
   `descriptor.target.architecture == binding.target_id()` and the configured
   code-object version to equal `binding.code_object_version()`
   (`native_prepare.rs:651-744`).
3. It moves the scoped bindings, host plan, and target plan into
   `LocalCandidateFactory::production`. Training and inference both call
   `into_parts`, clone the two vectors for `StagedCrossBackend`, and retain the
   originals for the local native factory (`src/training.rs:1303-1325`,
   `src/inference.rs:613-636`).
4. `StagedCrossBackend::validate_bindings` rejects duplicate `DeviceId` values,
   requires each CUDA binding to own a CUDA-class device and each HSA binding
   to own an HSA-class device, and performs the same checks again at final
   handoff (`native-executor/src/bridge.rs:186-229,1303-1322`).
5. The local factory uses the binding device IDs to classify arenas and tasks,
   obtains reservation evidence from the connector count, and takes initial
   capacity snapshots through `CudaBinding::available_bytes` or
   `HsaBinding::available_bytes` (`native-executor/src/local.rs:2145-2244`).

The callback is invoked while every borrowed context, session, host allocator,
and retained ROCr runtime is valid. Returning an owned target plan is allowed
because `NativePreparationScope::into_targets` drops the bindings before it
returns. Returning a value that borrows a binding is rejected by the
higher-ranked callback type.

## Validation and failure ownership

The errors are layered. A later layer must not be read as evidence that an
earlier identity check succeeded.

| Stage | Validation or failure | Error boundary |
| --- | --- | --- |
| Native config and discovery | nonzero FMA chain, absolute scratch root, library selection, PCI surface, Driver/ROCr load, exhaustive enumeration | `ProbeError::Discovery` from `NativeGpuProbe` and backend adapters |
| Current inventory association | profile validation, exhaustive flag, exact machine, key sets, duplicate keys, topology kind, host-memory relation, target equality | `ProbeError::Cache` for profile validation, otherwise `ProbeError::InvalidProfile` from `resolve_local_inventory` |
| Display observation | malformed BDF, missing PCI/DRM path, invalid connector state, count overflow | `ProbeError::Discovery` or `ProbeError::Io` |
| Backend partition | unsupported target backend or duplicate expected key | `ProbeError::Discovery` |
| CUDA realization | unexpected or missing key, duplicate key, deployment mismatch, context creation | `ProbeError::Discovery` |
| HSA realization | unexpected or missing key, unsupported GPU descriptor, missing queue, allocator ambiguity, session creation | `ProbeError::Discovery` |
| Host configuration | zero worker or staging capacity, duplicate device, duplicate disk path, invalid file path | `recipe_host::Error` |
| Root preparation | machine or binding set mismatch, target or toolchain mismatch | `NativePreparationError::IdentityMismatch`, `Probe`, or target/lowering variants |
| Native executor | duplicate or missing class owner, capacity mismatch, artifact incompatibility, session or queue fault | `native_executor::LocalError` or native backend error |

`binding_error` is the one conversion helper in this module. It maps a detail
string to `ProbeError::Discovery`; it does not retry, downgrade, or substitute
another backend. Errors from the callback itself are returned unchanged.

## Lifetime and invariant ledger

These are the invariants that make a successful binding scope meaningful:

```text
I1  Every binding device is a DeviceId from the measured profile, never a
    newly assigned ID and never a Driver ordinal or HSA raw handle.
I2  The current key set equals the machine-scoped measured key set. Both
    missing and extra current GPU origins fail.
I3  Descriptor.target.backend is exactly one of the two supported backend
    strings. Unsupported backends have no binding implementation.
I4  CUDA deployment identity and Context are built from one current Driver
    discovery snapshot and one DeviceInfo record.
I5  Every HSA GPU session has one exact target and one selected, allocatable
    CPU host allocator. Ambiguous NUMA choices fail.
I6  CUDA and HSA binding vectors contain no duplicate DeviceId, are sorted by
    DeviceId, and are disjoint by backend class.
I7  Enabled display connectors, queue ceilings, queue packet counts, target
    strings, and deployment capabilities are current observations. They are
    not reconstructed from serialized profile text.
I8  The operation callback is invoked at most once, and only after all
    realization checks pass. Its native borrows cannot escape the callback.
I9  Contexts, sessions, agents, and temporary realization records are dropped
    after the callback. The probe retains the ROCr runtime only according to
    its own backend lifetime; no handle enters a declaration or dynamic
    placement cache.
I10 No fallback selector, retry, alternate implementation, or partial binding
    vector exists. A failed transition remains visible to the caller.
```

The `expect` calls around `operation.take()` represent the internal at-most-once
callback invariant. They are unreachable when the function is used through its
public signature and are not recovery paths for a hardware failure.

## End-to-end use

The production path is therefore:

```text
recipe probe
  -> current exhaustive descriptors and measured profile
  -> identity-named profile cache
with_current_native_preparation / with_native_preparation
  -> load and validate profile
  -> NativeGpuProbe::discover_all
  -> resolve_local_inventory
  -> with_native_execution_bindings
  -> exact CUDA contexts and HSA sessions
  -> host backend config and native target plan
  -> LocalCandidateFactory::production
  -> pre-loop preparation and warm capacity observation
  -> immutable init -> loop -> exit execution
```

This module proves only the scoped native binding step and the pure host
configuration step. It does not prove artifact compatibility, reservation
headroom, queue health, or end-to-end numerical correctness. Those are checked
by the downstream preparation and executor layers while they consume the
bindings.
