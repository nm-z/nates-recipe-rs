# `recipe-native-probe` crate facade

Source: [`native-probe/src/lib.rs`](../../src/lib.rs)

```toml
[module]
path = "native-probe/src/lib.rs"
kind = "private-module-crate-facade"
intent = "Discover and measure every local CUDA and ROCr/HSA GPU through the native driver APIs, then reopen exact measured origins for one bounded preparation scope."
purpose = "Own the production GPU half of recipe_probe::ProbeEngine and the preparation-scoped bridge from a validated MeasuredProfile to borrowed recipe_native_executor bindings."
structure = "Seven private implementation modules behind one explicit root re-export surface."
private_modules = ["benchmark", "bindings", "config", "cuda", "hsa", "identity", "native"]
public_modules = []
state = "per-probe owned backends; HSA runtime retained by the probe after first open"
runtime_side_effects = "native library loading, driver/ROCr discovery, PCI and DRM reads, bounded GPU submissions, kernel realization, and preparation-scoped context/session creation"
exported_artifacts = "none; benchmark kernels are temporary probe work"

[boundary]
input = "explicit NativeProbeConfig, current PCI/sysfs and native runtime state, recipe_probe::BoundedBenchmarkPlan, and validated profile/host inventory for binding reopen"
discovery_output = "recipe_probe::GpuInventory with sorted, exact GpuDescriptor values and an exhaustive flag"
benchmark_output = "recipe_probe::GpuMeasurement whose capacity, transfer rates, and calculation rate are all Measured"
binding_output = "NativeExecutionBindings<'cuda, 'hsa> borrowed only for a higher-ranked callback"
host_output = "recipe_host::HostBackendConfig built from a ResolvedLocalInventory without filesystem I/O"
failure = "recipe_probe::ProbeError; missing optional vendor hardware is empty only when PCI preflight finds no matching accelerator"

[identity]
library = "first existing absolute candidate after canonicalization, deduplicated by canonical path, SHA-256 pinned"
gpu = "backend-specific stable UUID plus canonical PCI BDF, with driver, firmware, link, runtime, and toolchain identity surfaces"
reopen = "exact retained profile keys and identities only; no ordinal, product-name, capacity, or benchmark-similarity fallback"
toolchain = "Recipe source digest, backend target configuration, release label, and required pinned tool digests"

[ownership]
config = "callers own the input NativeProbeConfig; backend constructors clone the fields they need"
probe = "NativeGpuProbe owns backend adapters, PCI root, and any retained HSA runtime"
descriptors = "discover_all returns owned descriptors; no native runtime handle escapes descriptor data"
bindings = "CUDA contexts and HSA sessions are borrowed into NativeExecutionBindings and cannot outlive its callback"
host = "host_backend_config_from_inventory returns owned host configuration; run-scoped disk paths are consumed later by recipe_host"
```

## Intent and boundary

`recipe-native-probe` is the native, bare-metal GPU side of Recipe's general
`recipe-probe` engine. It is deliberately not a second profile engine. The
crate implements `recipe_probe::GpuDiscovery` and
`recipe_probe::GpuBenchmarkIo` for [`NativeGpuProbe`], so the ordinary probe
pipeline can use one host discovery implementation, one native GPU source, and
one host benchmark implementation. The profile engine remains responsible for
seed policy, cache identity, topology construction, profile validation, and
profile persistence.

The crate also owns the later preparation bridge. Given the same probe that
produced a profile, `with_native_execution_bindings` re-discovers the current
machine, resolves the profile's stable origins, recreates exact CUDA contexts
and HSA sessions, and lends backend-neutral `recipe_native_executor` bindings
to one callback. It does not create a dynamic placement policy or store a
runtime handle in a declaration.

Every native property placed in a `GpuDescriptor` or `GpuMeasurement` comes
from the current native discovery or from a completed native submission. Seed
values supplied to `ProbeEngine` bound the benchmark plan only. This crate does
not turn theoretical seed values into measured capacities or rates.

The root facade has no global mutable registry, background worker, socket, or
public submodule. `NativeGpuProbe` owns state per instance. The only retained
long-lived native runtime is the HSA `Runtime` held by its private backend;
contexts, sessions, allocations, streams, queues, modules, and events created
for one discovery, benchmark, or binding scope are owned by that operation and
are dropped or closed at its boundary.

## Crate build surface

`native-probe/Cargo.toml` declares package `recipe-native-probe` version
`0.1.0`, library target `recipe_native_probe`, Rust edition 2024, and MIT
license. It has no feature flags, binary target, or optional dependencies. Its
runtime dependencies are the workspace crates `recipe-core`, `recipe-cuda`,
`recipe-hsa`, `recipe-host`, `recipe-kernel`, `recipe-native-executor`, and
`recipe-probe`, plus `sha2`. The build dependency is `sha2`.

`native-probe/build.rs` computes the compile-time
`RECIPE_NATIVE_PROBE_SOURCE_DIGEST` from the native-probe source, the relevant
workspace implementation and manifest files, and the Rust compiler/build
identity. It emits Cargo rerun markers for those inputs and environment values.
`identity::backend_toolchain_identity` mixes that digest into every backend
toolchain identity. The build script therefore participates in identity
evidence, but it does not discover hardware, run a benchmark, or create a
runtime fallback. The package denies unsafe operations inside unsafe functions
and denies undocumented unsafe blocks through its manifest lint settings.

## Root module structure

`lib.rs:10-16` declares every implementation module with `mod`, not `pub mod`.
Consequently `recipe_native_probe::cuda::CudaBackend`, the private benchmark
helpers, and the identity helpers are not importable public paths. The explicit
re-export list at `lib.rs:18-20` is the complete crate-root API:

| root item | source | role and boundary |
| --- | --- | --- |
| `BackendLibrary` | `config.rs:6-13` | Ordered absolute library candidates for one optional native runtime. The first existing canonical target is selected and hashed. |
| `CudaProbeConfig` | `config.rs:15-20` | CUDA Driver candidate list plus PTX ISA encoded as major times ten plus minor. |
| `HsaProbeConfig` | `config.rs:22-26` | ROCr/HSA candidate list plus nonzero code-object version. |
| `KernelBuildConfig` | `config.rs:28-36` | Pinned offline compiler toolchain, release label, absolute scratch parent, and dependent f32 FMA chain length. |
| `NativeProbeConfig` | `config.rs:38-47` | Host RAM origin key, explicit PCI sysfs root, CUDA configuration, HSA configuration, and shared kernel build configuration. |
| `NativeGpuProbe` | `native.rs:40-44` | Production or deliberately non-exhaustive CUDA/HSA discovery and benchmark adapter. |
| `NativeExecutionBindings<'cuda, 'hsa>` | `bindings.rs:22-47` | Machine ID plus vectors of borrowed executor `CudaBinding` and `HsaBinding` values. |
| `host_backend_config_from_inventory` | `bindings.rs:87-118` | Pure construction of RAM and run-scoped disk host bindings from an exact resolved inventory. |
| `with_native_execution_bindings` | `bindings.rs:120-234` | Exact profile-bound GPU reopen and higher-ranked callback scope. |

`ProbeResult<T>` in the signatures above is the result alias owned by
`recipe_probe`; this crate does not re-export that alias or `ProbeError`. The
root crate is available to advanced callers as `recipe::engine::native_probe`
through [`src/facade.rs`](../../src/facade.rs), where the root facade aliases
the complete `recipe_native_probe` crate. The application CLI and native
preparation module import the crate directly.

## Public configuration declarations

All five configuration types derive `Clone`, `Debug`, `PartialEq`, and `Eq`.
Their fields are public, so the caller constructs one explicit value and owns
its paths and labels. Construction of a configuration is not native discovery;
validation that depends on hardware or files happens when a probe is opened.

| type | fields | checked meaning |
| --- | --- | --- |
| `BackendLibrary` | `candidates: Vec<PathBuf>` | Candidates are tried in order. A missing candidate is skipped. Every candidate must be absolute, and an existing candidate must be a regular file or symlink to a regular file. Once a vendor accelerator is present, no existing usable runtime is an error. |
| `CudaProbeConfig` | `library: BackendLibrary`, `ptx_isa: u16` | `ptx_isa` is passed into `recipe_kernel::NvidiaTarget`; it is not inferred from a product name. |
| `HsaProbeConfig` | `library: BackendLibrary`, `code_object_version: u8` | A zero code-object version is rejected by `HsaBackend::new`. |
| `KernelBuildConfig` | `toolchain: OfflineToolchain`, `release: Label`, `scratch_parent: PathBuf`, `fma_chain_length: u16` | `scratch_parent` must be absolute and `fma_chain_length` must be nonzero. The backend-specific toolchain identity additionally requires LLVM verifier and codegen plus `lld` for AMD or `ptxas` for NVIDIA. |
| `NativeProbeConfig` | `host_memory_key: Label`, `pci_sysfs_root: PathBuf`, `cuda`, `hsa`, `kernels` | `host_memory_key` is copied into each GPU descriptor as the host RAM origin. The PCI root is explicit and is checked for absolute form when read. |

The library-candidate rule has an intentional two-part boundary. A machine
without a matching PCI accelerator may have no selected library and contributes
an empty backend. A machine with a matching accelerator must have a selected
library, and load, hashing, or exhaustive enumeration failure is fatal. A
present library on a machine with no matching vendor hardware is not evidence
that the backend participates.

## `NativeGpuProbe` public API

`NativeGpuProbe` has private fields `backends`, `exhaustive`, and
`pci_sysfs_root`. Its `Debug` implementation exposes only the backend count and
is intentionally non-exhaustive. The three constructors are the only ways to
select backend composition:

| constructor | backend set | `GpuInventory::exhaustive` | intended use |
| --- | --- | --- | --- |
| `NativeGpuProbe::new(config)` | CUDA and HSA adapters | `true` | The only constructor accepted by normal measured-profile construction. |
| `NativeGpuProbe::cuda_diagnostic(config)` | CUDA adapter only | `false` | Isolate CUDA discovery or benchmarking while another required device is unavailable. |
| `NativeGpuProbe::hsa_diagnostic(config)` | HSA adapter only | `false` | Isolate HSA discovery or benchmarking while another required device is unavailable. |

All three constructors return `ProbeResult<Self>`. They reject an empty FMA
chain or relative kernel scratch parent before backend state is created; HSA
construction also rejects code-object version zero. The diagnostic forms are
not a profile shortcut. `ProbeEngine::inspect` rejects their inventory with
`ProbeError::IncompleteGpuEnumeration`, even if a diagnostic discovers one
valid device.

### `GpuDiscovery::discover_all`

The trait implementation at `native.rs:245-264` performs one pass over the
private backend vector. Each adapter returns owned `GpuDescriptor` values. The
facade sorts all descriptors lexically by `GpuDescriptor::key` and rejects a
duplicate key before returning:

```text
NativeGpuProbe::discover_all()
    -> ProbeResult<GpuInventory { exhaustive, devices: sorted unique descriptors }>
```

The `exhaustive` bit is `true` only for `new` and `false` for either diagnostic
constructor. The method does not return host RAM, storage, or network domains.
It also does not benchmark, cache, or persist the descriptors.

Each CUDA descriptor binds the exact Driver device to its PCI BDF and sysfs
surface. Its target backend is `nvidia-cuda-driver`, ABI is `elf64-cubin`,
transport is PCIe full duplex, and the descriptor carries current capacity,
driver/runtime/firmware/link identities, transfer lane limits, queue limits,
workgroup and subgroup limits, shared-memory capacity, and whether transfers
can overlap calculation. Each HSA descriptor follows the same shape with
backend `amd-rocr-hsa`, ABI `elf64-amdgpu-code-object-v<version>`, an exact
ROCr AMDGPU target, current HSA/KFD/PCI surfaces, and queue and ISA limits.
The host RAM key in both cases is the configured `host_memory_key`.

### `GpuBenchmarkIo::benchmark_gpu`

The trait implementation at `native.rs:267-298` first rejects an unbounded
`BoundedBenchmarkPlan`. It then calls each enabled backend's discovery again
and selects exactly one backend whose freshly constructed descriptor equals the
caller-provided descriptor. No matching owner means the GPU identity changed;
more than one owner is an error. Only after this exact ownership check does it
submit the benchmark.

The result is a `GpuMeasurement` with all five properties marked
`PropertyProvenance::Measured`:

```text
capacity
calculation_rate
memory_rate
host_to_device_rate
device_to_host_rate
```

`benchmark.rs:20-92` applies the plan deadline and iteration bound to every
submission. It refuses zero work, zero duration, unbounded plans, overflowed
work counters, zero elapsed time, and rates that do not fit the core units.
The compute buffer is capped at four MiB by the private benchmark helper and is
aligned to f32 width. This cap is a native-probe implementation bound, not a
public partial-batch or model-training setting.

The shared calculation workload is created at `benchmark.rs:95-160` as a
Recipe-owned `KernelTemplate` with one f32 input, one f32 output, a forbidden
input/output alias, and exactly the configured number of dependent FMA
instructions per element. Input bytes are filled deterministically, and the
first output element must be finite and bitwise different from the input
element (`benchmark.rs:163-185`). The backend must compile, load, submit, and
verify this native kernel before reporting a calculation rate.

## Private module map

| module | source responsibility | public leakage |
| --- | --- | --- |
| `benchmark` | Bounded timing, transfer and FLOP rate arithmetic, f32 buffer sizing, deterministic input, Recipe FMA template construction, output verification, measured-property wrapping, and nonzero capacity checks. | None. `TimedWork` and all helpers are `pub(crate)`. |
| `bindings` | Public binding envelope and host-config constructor; profile-to-current-inventory resolution; exact CUDA and HSA reopen; display-connector counting; HSA CPU allocator selection. | Only `NativeExecutionBindings`, `host_backend_config_from_inventory`, and `with_native_execution_bindings` are re-exported. |
| `config` | Public configuration structs used by all backends. | All five config structs are re-exported; the module itself is private. |
| `cuda` | CUDA Driver library selection, exhaustive discovery, PCI identity comparison, descriptor construction, bounded H2D/D2H/D2D copies, Recipe-owned cubin lowering/build/load/launch, completion polling, and measured rates. | `CudaBackend` and every helper are private to this crate. |
| `hsa` | ROCr runtime retention, exhaustive agent and ISA discovery, descriptor construction, KFD/PCI identity surfaces, bounded HSA copies, Recipe-owned HSACO lowering/build/load/dispatch, completion polling, and measured rates. | `HsaBackend` and every helper are private to this crate. |
| `identity` | Absolute candidate validation, canonical library hashing, backend toolchain/source digest construction, PCI vendor preflight, driver/firmware/link surface digests, and library identity strings. | `PinnedLibrary`, `PciSurface`, and all identity functions are crate-private. |
| `native` | Backend trait and CUDA/HSA sum type; public `NativeGpuProbe`; sorted aggregate discovery; exact backend ownership during benchmark; PCI DRM display-connector count. | Only `NativeGpuProbe` is re-exported. |

The private modules depend on the workspace's ownership layers in one direction:

```text
NativeProbeConfig
    -> CudaBackend / HsaBackend
        -> recipe_cuda / recipe_hsa discovery and native resources
        -> recipe_kernel lowering and offline ArtifactBuilder
        -> benchmark helpers
    -> NativeGpuProbe (GpuDiscovery + GpuBenchmarkIo)
        -> recipe_probe::ProbeEngine
    -> bindings
        -> recipe_probe::ResolvedLocalInventory
        -> recipe_native_executor::{CudaBinding, HsaBinding}
```

`identity` supplies stable evidence to both backend descriptor builders and
`bindings` reopen checks. `benchmark` supplies no discovery and no backend
selection; it only operates on a bounded plan and native submission closure.

## Backend discovery and measurement boundaries

### CUDA path

`cuda.rs` opens only after `identity::pci_accelerator_present` finds an NVIDIA
accelerator class under the configured PCI root. It selects and hashes the
first existing absolute Driver-library candidate, loads the CUDA Driver API,
and requests exhaustive Driver discovery. Descriptor construction parses the
Driver PCI bus ID, compares its numeric domain/bus/device fields with Driver
attributes, canonicalizes the BDF for sysfs, and digests the PCI driver,
firmware, and link surface. It derives an `NvidiaTarget` from the current SM
capability and configured PTX ISA, validates it, and incorporates the pinned
toolchain and native-probe source digest into `ToolchainIdentity`.

For measurement, CUDA creates a yielding context and nonblocking stream,
allocates pinned host source/destination buffers plus two device buffers, and
times H2D, D2H, and D2D copies. Each copy is polled to terminal completion and
then copied back for byte-for-byte verification. Calculation lowering uses the
same f32 FMA template as HSA, realizes a cubin through the pinned
`recipe_kernel::ArtifactBuilder`, checks the inspected entry symbol
`recipe_probe_fma_f32`, launches the explicit one-input/one-output/i64 ABI, and
verifies the changed finite output before calculating FLOPs per second.

CUDA has no cancellation primitive. If a submission passes its benchmark
deadline, `complete_cuda` continues polling the already-submitted operation at
a capped cleanup backoff until it reaches terminal completion, then returns a
deadline error. It never frees the borrowed buffers early and never submits a
replacement operation.

### HSA path

`hsa.rs` opens only after AMD accelerator preflight. The backend retains one
`Runtime` and its pinned library identity in a `RefCell`; subsequent calls must
observe the same canonical library and digest. It exhaustively discovers ROCr
agents and system versions. GPU descriptors require kernel dispatch support, a
stable value UUID, an exact PCI address, one unambiguous artifact target, AMD
capability data, queue limits, ISA limits, and KFD LDS capacity. Capacity uses
the largest allocatable GPU coarse-grained global pool, falling back to the
reported available memory only when no such pool exists.

For measurement, HSA chooses a CPU memory agent, allocates fine-grained host
source/destination plus coarse-grained GPU source/destination, grants access in
both directions, and times H2D, D2H, and D2D asynchronous copies with byte
verification. It lowers and realizes an HSACO with the pinned toolchain, checks
the inspected kernel symbol and kernarg size against ROCr metadata, writes the
three explicit u64 arguments (input pointer, output pointer, element count),
dispatches a one-dimensional queue workload, verifies output, and computes the
measured FLOP rate.

If AMD hardware disappears after a runtime was retained, or the selected ROCr
library identity changes, the backend returns an error. A missing runtime is
empty only before initialization and only when no AMD accelerator is present.
As with CUDA, a timed-out HSA operation remains live and is polled to terminal
completion before a deadline error is returned.

### Identity surfaces

`identity.rs` makes every runtime decision explicit:

1. `selected_library` rejects relative candidates, accepts missing candidates
   as absent, canonicalizes existing files and symlinks, deduplicates targets,
   hashes the selected target bytes, and returns the first selected identity.
2. `backend_toolchain_identity` hashes a domain tag, backend name, release,
   target configuration, compile-time `RECIPE_NATIVE_PROBE_SOURCE_DIGEST`,
   and the paths and artifact digests of required pinned tools. AMD requires
   `elf_linker`; NVIDIA requires `ptx_assembler`.
3. `pci_accelerator_present` reads vendor and class files under an absolute PCI
   root and treats class major `0x03` or `0x12` as an accelerator.
4. `pci_surface` requires a resolvable driver link and digests readable kernel,
   module, firmware, PCIe-link, and NUMA surface files. Missing optional
   firmware or link files are represented in the digest, not replaced with a
   guessed value.

The build script computes the source digest from the native-probe source and
the relevant workspace implementation and manifest inputs. The digest is
therefore part of the measured target identity without making build metadata a
runtime fallback.

## Preparation-scoped binding API

### `NativeExecutionBindings`

`NativeExecutionBindings<'cuda, 'hsa>` owns its vectors but borrows the native
resources inside each element. Its public methods are:

| method | result | ownership meaning |
| --- | --- | --- |
| `machine(&self) -> MachineId` | copied machine ID | Identifies the exact profile machine associated with the bindings. |
| `cuda(&self) -> &[CudaBinding<'cuda>]` | borrowed CUDA slice | Exposes exact device, deployment, queue, and display-connector data without moving contexts. |
| `hsa(&self) -> &[HsaBinding<'hsa>]` | borrowed HSA slice | Exposes exact device, target, code-object, queue, and display-connector data without moving sessions. |
| `into_parts(self) -> (Vec<CudaBinding<'cuda>>, Vec<HsaBinding<'hsa>>)` | moved vectors | Transfers the binding envelopes while retaining their original borrow constraints. |

The type derives `Clone` and `Debug`, but cloning clones only the binding
envelopes and their native references. It does not duplicate a CUDA context or
HSA session. `CudaBinding` and `HsaBinding` are owned by
`recipe_native_executor`; this crate constructs them and does not reimplement
their execution behavior.

### `host_backend_config_from_inventory`

The function accepts an exact `ResolvedLocalInventory`, a `RunId`, worker
thread count, and staging bytes per worker. It emits one `HostDeviceBinding::Ram`
for every resolved RAM origin and one `HostDeviceBinding::Disk` for every
resolved storage origin. A storage binding's arena path is the deterministic
child `<benchmark_root>/.recipe-run-<run>-device-<device>-arena`.

The function constructs `DiskFileSpec` and `HostBackendConfig` only. It does
not create, truncate, inspect, or delete an arena file. The later host backend
uses exclusive `create_new` realization, so an overlapping run ID fails rather
than overwriting another run. This function cannot accept an unresolved host
inventory or choose a storage root by capacity, path order, or recency.

### `with_native_execution_bindings`

The callback boundary is the central ownership contract:

```text
with_native_execution_bindings(
    probe: &NativeGpuProbe,
    profile: &MeasuredProfile,
    host: &HostInventory,
    operation: for<'cuda, 'hsa> FnOnce(NativeExecutionBindings<'cuda, 'hsa>)
) -> ProbeResult<T>
```

The implementation at `bindings.rs:130-234` performs this ordered sequence:

1. Call `probe.discover_all()` and require the resulting inventory to resolve
   against `profile.resolve_local_inventory(host, &inventory)`. That validates
   the profile and requires exact current machine fingerprint and complete RAM,
   storage, and GPU key sets. The profile's device IDs, not live ordinals, are
   retained.
2. Partition resolved GPUs by the descriptor target backend strings
   `nvidia-cuda-driver` and `amd-rocr-hsa`. Count enabled DRM connectors from
   the canonical PCI BDF suffix in each origin. Any other backend string or
   duplicate resolved key is an error.
3. Reopen every expected CUDA device through the probe's CUDA backend. A fresh
   Driver discovery must produce no unmeasured device, no duplicate measured
   key, and every expected key. Each device receives a new context and the
   deployment identity retained by that same discovery snapshot.
4. Reopen every expected HSA device through the probe's retained ROCr runtime.
   CPU agents are filtered to those with allocatable fine-grained and kernarg
   pools. A GPU must have exactly one same-NUMA allocator, or exactly one
   fallback allocator when no same-NUMA allocator exists. Missing or ambiguous
   allocators are errors. Each expected GPU receives a new HSA session and its
   exact ISA target, code-object version, queue packet count, queue limit, and
   display-connector count.
5. Invoke the callback once with the machine ID and sorted native binding
   vectors. The callback runs while every borrowed context, session, host
   allocator, and retained runtime is alive. Its higher-ranked lifetime prevents
   a returned `T` from borrowing those native resources.
6. Drop the callback's binding envelope and native resources at scope exit.
   There is no partial binding result and no fallback if one measured origin
   cannot be reopened.

The absence cases are precise. A probe with no HSA backend may invoke the
callback with an empty HSA vector only when the profile has no HSA origins. A
probe with no CUDA backend may do the same for CUDA. If the profile requires a
backend that the probe does not own, the call fails.

## End-to-end consumers

The workspace has a small, explicit set of production consumers. The crate is
not called through an internal test adapter or a replacement backend.

| consumer | call path | responsibility that remains outside this crate |
| --- | --- | --- |
| `src/cli.rs::run_probe` (`cli.rs:876-946`) | Build `NativeProbeConfig` from CLI paths and the discovered host RAM key, call `NativeGpuProbe::new`, pass the same value as both `ProbeEngine`'s GPU discovery and GPU benchmark implementation, compute cache identity, then `load_or_probe_and_store`. | CLI owns seed loading, private state/profile paths, cache receipt capture, and user-facing output. `ProbeEngine` owns profile assembly and cache persistence. |
| `src/cli.rs::current_native_inputs` (`cli.rs:949-1009`) | Reopen the active receipt or derive the current identity with a temporary `NativeGpuProbe`; return config, host inventory, profile path, identity, and a fresh native probe. | CLI owns receipt validation and current-profile path selection. |
| `src/native_prepare.rs::with_native_preparation` (`native_prepare.rs:317-365`) | Clone config into `NativeGpuProbe::new`, discover and resolve the exact local profile, call `with_native_execution_bindings`, and turn the borrowed bindings into a preparation scope with compiler target specs. | `native_prepare` owns profile loading, target policy, artifact compiler specifications, host plan ownership, and the callback's higher-level preparation result. |
| `src/native_prepare.rs::with_current_native_preparation` (`native_prepare.rs:368-410`) | Reuse a thread-local probe only when its config is unchanged, load the identity-named profile, and call the same binding bridge. | `native_prepare` owns receipt-backed profile selection and thread-local probe retention. |
| `src/facade.rs::engine::native_probe` (`facade.rs:18-37`) | Re-export the complete crate for advanced `recipe` users. | The root facade owns namespace composition; this crate still owns all native probing behavior. |

There are no other production call sites of `recipe_native_probe` in the
workspace. Training and inference consume the prepared bindings through
`native_prepare` and `recipe_native_executor`; they do not call private backend
methods or construct a second native probe path.

The normal CLI flow is therefore:

```text
recipe probe
    -> LocalSystemDiscovery::discover_host
    -> NativeProbeConfig
    -> NativeGpuProbe::new
    -> ProbeEngine::current_cache_identity
         -> NativeGpuProbe::discover_all
    -> ProbeEngine::load_or_probe_and_store
         -> NativeGpuProbe::discover_all
         -> NativeGpuProbe::benchmark_gpu for every exact descriptor
         -> measured profile validation and cache store
    -> active native receipt
```

The normal preparation flow is:

```text
MeasuredProfile + HostInventory + NativeProbeConfig
    -> NativeGpuProbe::new or retained exact probe
    -> NativeGpuProbe::discover_all
    -> MeasuredProfile::resolve_local_inventory
    -> with_native_execution_bindings
         -> exact CUDA contexts and HSA sessions
         -> NativeExecutionBindings callback
    -> native_prepare builds owned host and compiler plans
    -> callback scope destroys borrowed native handles
```

## Fail-closed invariants

The following facts are part of the crate boundary and must remain observable:

1. Normal profile construction uses an exhaustive `NativeGpuProbe`. A
   CUDA-only or HSA-only diagnostic cannot be smuggled into an accepted profile
   by changing a status bit downstream.
2. A vendor runtime is optional only when its vendor accelerator is absent from
   explicit PCI preflight. Hardware with a missing, changed, unloadable, or
   partially enumerable runtime is an error.
3. Every GPU key is unique and every benchmark descriptor is re-discovered and
   compared in full before submission. A changed identity is never matched by
   ordinal, name, capacity, performance, or list position.
4. Descriptor and measurement properties are current native evidence. The
   benchmark returns only after transfer bytes and Recipe-owned calculation
   output are verified.
5. Toolchain and library paths are canonicalized and hashed. The source digest,
   target configuration, release label, and pinned tools participate in the
   target identity used by profile preparation.
6. Native GPU work is bounded by the supplied benchmark plan. Completion
   helpers preserve live native allocations until each submitted operation is
   terminal, even when the plan deadline is exceeded.
7. Reopen uses the same profile and probe identity that produced discovery.
   Complete local origin sets, backend-specific identities, deployment data,
   HSA allocator choice, and display-connector state must agree. There is no
   partial scope.
8. Native contexts, sessions, allocators, and executor bindings do not escape
   the higher-ranked callback. The crate does not expose a dynamic native handle
   as a declaration field or cache it outside the owning preparation scope.
9. Host arena configuration is deterministic and construction-only. File I/O
   and exclusive arena creation remain in `recipe_host` realization.
10. Probe kernels are Recipe-owned temporary benchmark artifacts. This crate
    does not export `.cubin`, `.hsaco`, profile, journal, plan, or cache files.

## Non-responsibilities

`recipe-native-probe` intentionally does not:

- parse `topology/contract.toml`, choose benchmark durations or iterations, or
  build a `MeasuredProfile` itself;
- discover host RAM, mounted storage, network interfaces, or peer sessions;
- persist or decode profile caches, active receipts, or user artifacts;
- select a GPU by ordinal, product name, capacity, benchmark score, or display
  preference;
- allocate execution arenas, create candidate plans, schedule tasks, or run the
  finalized `init -> loop -> exit` executor lifecycle;
- expose CUDA or HSA runtime handles outside the preparation callback;
- provide a vendor runtime fallback, alternate compiler, compatibility shim, or
  retry path when native identity or completion fails.

These boundaries keep discovery and completed native measurement in this crate,
profile policy in `recipe_probe`, preparation and compiler-target ownership in
`native_prepare`, and finalized asynchronous execution in
`recipe_native_executor`.
