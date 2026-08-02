# `cuda/src/lib.rs`: the `recipe-cuda` crate root

This file is the public-facade map for `recipe-cuda`. The implementation lives
in the seven private modules declared by [`cuda/src/lib.rs`](../../src/lib.rs).
The crate is Recipe's reviewed, dependency-clean boundary to the NVIDIA CUDA
Driver API. It loads the Linux Driver shared object, discovers exact devices,
creates explicit contexts, validates CUDA artifact identities, and owns the
native objects needed for asynchronous copies and kernel launches.

The crate root does not implement a second runtime. It only applies the target
contract, declares the module graph, and re-exports the selected public types.
Every operation described below is implemented in the linked module. This is
an implementation map, not a promise that a missing CUDA deployment can be
emulated or bypassed.

## Crate and platform contract

The manifest names the package `recipe-cuda`, version `0.1.0`, edition 2024,
and describes it as dependency-clean CUDA Driver API scaffolding. Its only
dependencies are `libc = "0.2"` for the Linux dynamic loader and
`sha2 = "0.10"` for cubin digest validation ([`cuda/Cargo.toml`](../../Cargo.toml)).
There are no features, build scripts, generated bindings, static CUDA link
inputs, binaries, or examples in this package.

The root attributes and compile gates are exact:

| Source | Current rule |
| --- | --- |
| `#![allow(unsafe_code)]` | Unsafe is allowed only at the reviewed CUDA Driver and `dlopen`/`dlsym` FFI boundary. |
| `cfg_attr(not(target_os = "linux"), allow(dead_code, unused_imports))` | Keeps inactive Linux-loader code quiet while a target is being checked. |
| `compile_error!("recipe-cuda currently supports Linux libcuda.so deployments only")` | Any non-Linux target is rejected at compile time. |
| `compile_error!("recipe-cuda requires a 64-bit CUDA Driver ABI")` | Any non-64-bit target is rejected at compile time. |

The `cfg_attr` is not a portability implementation. The compile error remains
the authoritative platform boundary. The package uses the Driver API only. It
does not use the CUDA Runtime API, HIP, cuBLAS, cuDNN, NCCL, or another vendor
operation library.

## Root module graph

The declarations at lines 16 through 22 of `lib.rs` are private. Their public
surface is exposed only through the explicit `pub use` groups at lines 24
through 38.

```text
recipe_cuda::lib.rs
├── artifact  -> artifact/deployment identity and compatibility checks
├── context   -> context flags, current-context guard, memory observation
├── discovery -> exact Driver/device snapshot and typed identity values
├── driver    -> loaded Driver owner, capability query, status conversion
├── error     -> crate-wide Result and typed failure values
├── ffi       -> opaque Driver handles, function pointers, loader, symbol sets
└── runtime   -> modules, buffers, streams, events, launches, pending work
```

The dependency direction is deliberately one-way at the public boundary:

```text
ffi -> error
driver -> ffi + error
discovery -> driver + ffi + error
context -> driver + discovery + error + ffi
runtime -> context + driver + error + ffi
artifact -> discovery + ffi
lib.rs -> all modules through reexports
```

`ffi` is the only module that contains C signatures or `libc` loader details.
`driver` owns the resolved function table and keeps its library alive.
`discovery` and `context` use the Driver through crate-private fields.
`runtime` enters a current context for each Driver operation and checks that
all participating resources belong to the same `Context`. `artifact` has no
native handle and never loads a module.

### Exact reexport surface

The following is the complete root surface. A name absent from this table is
not reexported by `recipe_cuda`, even if its declaration is `pub` inside a
private module.

| Root path | Source module | Public names and role |
| --- | --- | --- |
| `recipe_cuda::artifact` values | `artifact.rs` | `ToolchainIdentity`, `ArtifactIdentity`, `DeploymentIdentity`, `ArtifactField`, `ArtifactIssue`, `ArtifactCompatibilityError`, `validate_artifact_compatibility`. |
| `recipe_cuda::context` values | `context.rs` | `Context`, `ContextFlags`, `ContextGuard`, `SchedulingPolicy`. `MemoryInfo` is public as the return type of `Context::memory_info`, but is not separately reexported at the crate root. |
| `recipe_cuda::discovery` values | `discovery.rs` | `ComputeCapability`, `DeviceAttributes`, `DeviceInfo`, `DeviceOrdinal`, `DeviceUuid`, `Discovery`, `DriverVersion`. |
| `recipe_cuda::driver` values | `driver.rs` | `Driver`, `ModuleLoadingMode`. |
| `recipe_cuda::error` values | `error.rs` | `CudaError`, `DriverCallError`, `DriverStatus`, and `Result<T>`. |
| `recipe_cuda::ffi` inventory | `ffi.rs` | `DriverCapabilities`, `DriverSymbol`, `REQUIRED_DRIVER_SYMBOLS`, `OPTIONAL_DRIVER_SYMBOLS`. No raw C handle or function pointer is exported. |
| `recipe_cuda::runtime` values | `runtime.rs` | `Module`, `Function`, `DeviceBuffer`, `PinnedHostBuffer`, `Dim3`, `LaunchConfig`, `Stream`, `Event`, `Pending`, `CompletionStatus`, `WaitOutcome`. |

The root does not reexport the private modules themselves. Direct users can
write `recipe_cuda::Driver::load()` and `recipe_cuda::Stream::create_nonblocking`
without knowing the implementation file. The root `recipe` package exposes the
same crate under [`recipe::engine::cuda`](../../../src/facade.rs#L17-L41), while
workspace crates that need the boundary depend on `recipe-cuda` directly.

## Public value domains

### Artifact and deployment identity

[`artifact.rs`](../../src/artifact.rs) owns identity data that must be checked
before a native image is loaded:

```text
ToolchainIdentity {
    zig_version: String,
    llvm_version: String,
    ptx_isa_version: String,
    ptxas_version: String,
    cuda_toolkit_version: String,
    cubin_format: String,
}

ArtifactIdentity {
    sha256: [u8; 32],
    target: ComputeCapability,
    toolchain: ToolchainIdentity,
    minimum_driver: DriverVersion,
    maximum_driver: Option<DriverVersion>,
    required_driver_symbols: BTreeSet<DriverSymbol>,
}

DeploymentIdentity {
    driver_version: DriverVersion,
    device_uuid: DeviceUuid,
    target: ComputeCapability,
    driver_capabilities: DriverCapabilities,
}
```

`DeploymentIdentity::from_discovery(discovery, device)` returns `Some` only if
the borrowed device's ordinal and UUID both occur in that exact `Discovery`
snapshot. Otherwise it returns `None`; it never manufactures an identity from a
device borrowed from another snapshot. The UUID and target come from the
device, while the driver version and resolved capability set come from the
snapshot.

`validate_artifact_compatibility(cubin, expected, observed, deployment)` is a
pure, accumulating check. Its observed order is:

1. Reject an empty image with `EmptyArtifact`.
2. Require the ELF magic bytes `0x7f 45 4c 46`, otherwise add
   `InvalidCubinMagic`.
3. Compute SHA-256 and compare it with `observed.sha256`, adding
   `DigestMismatch` when they differ.
4. Compare `expected` and `observed` for `Sha256`, `Target`, all six
   toolchain fields, `MinimumDriver`, `MaximumDriver`, and
   `RequiredDriverSymbols`, adding one `IdentityFieldMismatch` per mismatch.
5. Reject blank toolchain strings with `EmptyIdentityField`.
6. Reject an observed maximum driver below its minimum with
   `InvalidDriverRange`.
7. Require the observed compute target to equal the deployed target.
8. Require the deployed driver to be at least the observed minimum and, when
   present, no greater than the observed maximum.
9. Require every observed required symbol to be present in the deployed
   `DriverCapabilities`.

The function returns `Ok(())` only when the issue vector is empty. Otherwise it
returns `ArtifactCompatibilityError { issues }`, preserving every observed
issue for the caller. This error is separate from `CudaError` because no Driver
call is made by artifact validation.

`ArtifactField` is the closed field vocabulary `Sha256`, `Target`,
`ZigVersion`, `LlvmVersion`, `PtxIsaVersion`, `PtxasVersion`,
`CudaToolkitVersion`, `CubinFormat`, `MinimumDriver`, `MaximumDriver`, and
`RequiredDriverSymbols`. `ArtifactIssue` additionally carries the concrete
values for digest, target, driver range, driver version, and missing-symbol
failures. `ArtifactCompatibilityError` implements `Display` and
`std::error::Error`; its display text contains the number of issues, while the
typed `issues` field remains the source of detail.

### Discovery values and exact device snapshots

[`discovery.rs`](../../src/discovery.rs) converts raw Driver values into types
used by probing and native preparation:

| Type | Concrete representation and invariant |
| --- | --- |
| `DeviceOrdinal` | Private `i32`; `get()` exposes the Driver ordinal. Only discovery constructs it. |
| `DeviceUuid` | Exactly `[u8; 16]`; `from_bytes` preserves bytes and `as_bytes` borrows them. `Display` uses the `GPU-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` form. |
| `ComputeCapability` | Public `major` and `minor` `u32` fields; `new` stores them and `Display` renders `sm_Mm`. |
| `DriverVersion` | Private nonnegative raw `i32`; `from_raw` rejects negatives, `new(major, minor)` checked-encodes `major * 1000 + minor * 10`, and `raw`, `major`, and `minor` expose the components. |
| `DeviceAttributes` | Async-engine count, concurrent-kernel flag, warp size, block and shared-memory limits, PCI domain/bus/device IDs, core and memory clocks, memory-bus width, and multiprocessor count. |
| `DeviceInfo` | Ordinal, UUID, name, compute capability, exact Driver PCI string, total bytes, attributes, and one crate-private raw `CuDevice` handle. |
| `Discovery` | One `DriverVersion`, one `DriverCapabilities` snapshot, and an ordered `Vec<DeviceInfo>`; `device(ordinal)` performs exact ordinal lookup. |

`Driver::discover()` is implemented in this module. It calls `cuInit(0)`,
`cuDriverGetVersion`, and `cuDeviceGetCount`, rejects a negative count, then
discovers every ordinal. Each device requires a valid Driver name (NUL within
256 bytes, UTF-8, and nonempty after trimming), a UUID, an exact PCI string in
the `dddd:bb:dd.f` grammar, total memory that fits `u64`, and every listed
attribute as a nonnegative `u32`. A Boolean concurrent-kernel attribute must be
exactly `0` or `1`. `cuDeviceGetUuid_v2` is used when present; a
`CUDA_ERROR_NOT_SUPPORTED` result falls back to required `cuDeviceGetUuid`,
while any other v2 failure is returned. Duplicate UUIDs fail the complete
snapshot with `DuplicateDeviceUuid`. The snapshot copies the Driver's
capability inventory; later optional symbol lookups cannot mutate it.

### Driver owner and capability inventory

[`driver.rs`](../../src/driver.rs) defines cloneable `Driver`. Internally it
stores an `Arc<DriverInner>` containing both the resolved `Api` and its
`DynamicLibrary`, so every clone keeps the shared object open. `Driver::load()`
tries `libcuda.so.1` and then `libcuda.so`. `Driver::load_from_path(path)`
opens only the requested path. Both paths resolve all required symbols before
returning a Driver. `loaded_library()` reports the selected name.

`ModuleLoadingMode` has `Eager` and `Lazy`. `Driver::module_loading_mode()`
returns `Ok(None)` when optional `cuModuleGetLoadingMode` is absent, maps raw
values `1` and `2` to those variants, and rejects every other value as an
invalid Driver value.

[`ffi.rs`](../../src/ffi.rs) exposes a typed, non-exhaustive `DriverSymbol`
inventory without exposing the raw function pointers. `as_str()` gives each
symbol's C name. The required set is:

```text
cuInit
cuDriverGetVersion
cuDeviceGetCount
cuDeviceGet
cuDeviceGetName
cuDeviceGetUuid
cuDeviceGetPCIBusId
cuDeviceTotalMem_v2
cuDeviceGetAttribute
cuCtxCreate_v2
cuCtxDestroy_v2
cuCtxPushCurrent_v2
cuCtxPopCurrent_v2
cuModuleLoadData
cuModuleUnload
cuModuleGetFunction
cuMemAlloc_v2
cuMemFree_v2
cuMemGetInfo_v2
cuMemHostAlloc
cuMemFreeHost
cuStreamCreate
cuStreamDestroy_v2
cuStreamQuery
cuEventCreate
cuEventRecord
cuEventQuery
cuEventDestroy_v2
cuMemcpyHtoDAsync_v2
cuMemcpyDtoHAsync_v2
cuMemcpyDtoDAsync_v2
cuLaunchKernel
```

The optional set is exactly:

```text
cuDeviceGetUuid_v2
cuModuleGetLoadingMode
cuGetErrorName
cuGetErrorString
```

`DriverCapabilities::supports(symbol)` tests the resolved set and
`available_symbols()` returns its sorted exact-size iterator. Required symbols
are always present on a successfully constructed Driver. Optional absence is a
capability fact, not a request to reopen another library or invent a fallback
implementation.

### Context ownership

[`context.rs`](../../src/context.rs) owns one raw `CUcontext` and a cloned
Driver. `ContextFlags` is an explicit R470-safe bit set:

```text
ContextFlags {
    scheduling: SchedulingPolicy,       // Auto, Spin, Yield, BlockingSync
    map_host: bool,
    local_memory_resize_to_max: bool,
}
```

`ContextFlags::new`, `with_map_host`, `with_local_memory_resize_to_max`, and
`bits` build the mask. `from_bits` rejects unknown bits and invalid scheduling
combinations with `InvalidContextFlags`.

`Context::create(driver, device, flags)` validates the mask, calls
`cuCtxCreate_v2`, rejects a successful null handle, immediately pops the newly
created current context, and verifies that the popped handle is exactly the
created handle. `Context::enter()` pushes that handle and returns a
`ContextGuard`; `ContextGuard::leave()` pops and verifies the same handle.
Dropping an active guard attempts the same pop and suppresses a drop-time
failure. `Context` is intentionally not `Send` or `Sync` because it carries
`PhantomData<Rc<()>>`; the current-context stack is therefore thread-affine.

`Context::memory_info()` enters and leaves the context around
`cuMemGetInfo_v2` and returns `MemoryInfo { free_bytes, total_bytes }`.
`Context::device()` returns the owned discovery value, `as_raw()` exposes the
raw pointer only as a checked result, and consuming `close()` destroys the
context. A second close or any operation after destruction returns
`ContextClosed`. `Drop` calls `cuCtxDestroy_v2` best effort when explicit close
was not used.

### Runtime objects and asynchronous work

[`runtime.rs`](../../src/runtime.rs) contains context-borrowing owners. Every
Driver call runs through `with_current`, which pushes the context, invokes the
operation, then pops and verifies it. Every resource pair is checked by
`same_context` before submission. This gives the following ownership chain:

```text
Context
├── Module<'ctx>
│   └── Function<'module, 'ctx>
├── DeviceBuffer<'ctx>
├── PinnedHostBuffer<'ctx>
├── Stream<'ctx>
└── Event<'ctx>
       └── Pending<'op, 'ctx> (operation borrow plus completion event)
```

The public runtime methods are:

| Type and entry point | Behavior and validation |
| --- | --- |
| `Module::load_cubin(context, cubin)` | Requires ELF cubin bytes, not PTX or a fat binary; loads with `cuModuleLoadData` and rejects a null success handle. |
| `Module::function(name)` | Requires a nonempty name without NUL, resolves `cuModuleGetFunction`, and returns a function borrowing the module. |
| `Module::unload()` | Consuming `cuModuleUnload`; `Drop` is best effort. |
| `Function::name()` | Borrows the entry name retained by the function. |
| `DeviceBuffer::allocate(context, len)` | Requires nonzero bytes, calls `cuMemAlloc_v2`, rejects a null pointer, and records the exact length. `len`, `is_empty` (always false for a live allocation), `device_ptr`, and consuming `free` expose or release it. Ranges are checked for addition, allocation bounds, pointer conversion, and pointer overflow. |
| `PinnedHostBuffer::allocate(context, len)` | Requires nonzero bytes, calls `cuMemHostAlloc`, rejects a null pointer, zeroes the allocation, and provides `len`, `is_empty`, `as_slice`, `as_mut_slice`, and consuming `free`. Ranges use the same checked rules. |
| `Dim3::new(x, y, z)` | Rejects every zero axis with `InvalidInput`; all axes are retained as `u32`. |
| `LaunchConfig::new(grid, block)` / `with_dynamic_shared_memory(bytes)` | Stores launch geometry and defaults dynamic shared memory to zero. |
| `Stream::create_nonblocking(context)` | Calls `cuStreamCreate` with the nonblocking flag and rejects a null success handle. |
| `Stream::poll_idle()` | Queries `cuStreamQuery`, mapping success to `Complete`, `CUDA_ERROR_NOT_READY` to `Pending`, and all other statuses to a numeric Driver error. |
| `Stream::copy_h2d(...)` | Bounds-checks same-context pinned source and device destination, enqueues `cuMemcpyHtoDAsync_v2`, records the supplied event, and returns an event-backed `Pending`. |
| `Stream::copy_d2h(...)` | Bounds-checks same-context device source and mutable pinned destination, enqueues `cuMemcpyDtoHAsync_v2`, records the supplied event, and returns `Pending`. |
| `Stream::enqueue_d2h(...)` | Enqueues the same D2H operation without an event. The caller must retain both allocations and poll the stream to completion. |
| `Stream::copy_d2d(...)` | Bounds-checks same-context device ranges, enqueues `cuMemcpyDtoDAsync_v2`, records the supplied event, and returns `Pending`. |
| `Stream::enqueue_d2d(...)` | Enqueues D2D without an event; the caller owns retention until `poll_idle` is complete. |
| `Stream::launch(...)` | Checks function, event, and keepalive contexts, passes the parameter pointer array and `LaunchConfig` to `cuLaunchKernel`, records the event, and returns `Pending`. The caller must match the cubin ABI and retain every referenced allocation in `keepalive`. |
| `Stream::enqueue_launch(...)` | The same launch without an event. The function, module, stream, arguments, and referenced allocations must remain live until `poll_idle` is complete. |
| `Stream::destroy()` | Consuming `cuStreamDestroy_v2`; `Drop` is best effort. |
| `Event::create_completion(context)` | Creates a timing-disabled event and rejects a null success handle. |
| `Event::destroy()` | Consuming `cuEventDestroy_v2`; `Drop` is best effort. |
| `Pending::poll()` | Nonblocking event query. Once complete, every later poll returns `Complete` without another Driver query. |
| `Pending::wait(timeout)` | Polls until completion or the deadline, yielding between polls. A completed token returns its event in `WaitOutcome::Complete`; a deadline returns the same token in `WaitOutcome::TimedOut`. An `Instant` overflow is `InvalidInput`. |
| `Pending::recycle_event()` | Requires a terminal event query and returns the original event for reuse. Calling it while pending is `InvalidInput`. |

`CompletionStatus` is the two-state `Pending`/`Complete` result used by both
event and stream queries. `WaitOutcome` is the two-state ownership-preserving
result of a timed wait. The `#[must_use]` annotations on `Pending` and
`WaitOutcome::TimedOut` are deliberate: dropping an event-backed pending token
while the Driver may still access borrowed allocations violates the CUDA
lifetime contract. There is no cancellation primitive and no hidden
synchronization or replacement submission in this crate.

## Load-to-teardown lifecycle

The complete production path is the following state sequence. Higher layers
may choose when to run each state, but they cannot skip an identity or lifetime
check by entering an internal function.

```text
unloaded
  -> Driver::load / Driver::load_from_path
       dlopen -> dlsym required symbols -> optional capabilities -> Driver
  -> Driver::discover
       cuInit -> driver version/count -> every DeviceInfo -> Discovery
  -> DeploymentIdentity::from_discovery
       exact ordinal+UUID membership -> deployment identity
  -> Context::create
       create -> verify create-time pop -> thread-affine Context
  -> validate_artifact_compatibility
       ELF, SHA-256, identity, target, driver range, required symbols
  -> Module::load_cubin -> Module::function
       stable module owner -> borrowed entry function
  -> allocate DeviceBuffer/PinnedHostBuffer, Stream, Event
       all native resources borrow the same Context
  -> Stream copy/launch
       Driver enqueue -> event record or stream-only completion -> Pending
  -> Pending::poll / Pending::wait / Stream::poll_idle
       Pending or Complete, with timeout preserving the token
  -> recycle completion event or collect output
  -> explicit destroy/free, then Drop best-effort cleanup
       Event/Stream/Module/Buffers -> Context -> Driver library
```

The crate does not compile kernels, select a placement, create Recipe arena
layouts, interpret `BackendWork`, or mutate a scheduler plan. Preparation
chooses those facts before this boundary. This crate realizes and checks the
native facts it is given.

## Error protocol

`Result<T>` is an alias for `core::result::Result<T, CudaError>`. `CudaError`
is `#[non_exhaustive]`, implements `Display` and `std::error::Error`, and
retains a `DriverCallError` source for Driver-status failures. The complete
current variants and their boundaries are:

| Variant | Produced by | Meaning |
| --- | --- | --- |
| `LibraryOpen { attempts }` | `DynamicLibrary::open_default`, `open` | Every attempted soname/path failed; attempts retain loader text. |
| `InvalidLibraryPath { path }` | `Driver::load_from_path` | The path contains an interior NUL and cannot be passed to `dlopen`. |
| `MissingRequiredSymbol { symbol, detail }` | `Api::load` | One required Driver symbol is absent or `dlsym` returned an error. Optional absence is not this error. |
| `DriverCall(DriverCallError)` | rich discovery/realization checks and numeric runtime checks | A Driver status was nonzero. `operation` and `DriverStatus(i32)` are always retained; optional name/description text is present only when queried successfully. |
| `InvalidDriverValue { operation, detail }` | discovery, context creation, mode query, resource creation | A successful Driver call returned an impossible value, such as a negative count, malformed string, unknown mode, or null success handle. |
| `InvalidInput { operation, detail }` | constructors and runtime range/ABI boundaries | Caller input is invalid: NUL or empty names, zero allocation/dimension, out-of-bounds range, pointer overflow, or timeout overflow. |
| `InvalidDeviceName { ordinal, detail }` | `Driver::discover` | Device name was not NUL-terminated, not UTF-8, or empty after trimming. |
| `DuplicateDeviceUuid { first_ordinal, second_ordinal }` | `Driver::discover` | Two discovered ordinals reported the same UUID, so the snapshot is rejected. |
| `InvalidContextFlags { bits }` | `ContextFlags::from_bits`, `Context::create` | Unknown bits or an invalid scheduling mask were requested. |
| `ContextStackMismatch` | context create/enter/leave | `cuCtxPopCurrent_v2` returned a different handle than Recipe pushed. |
| `ContextClosed` | any owner after explicit close/free/destroy | The native handle was already taken. |
| `ResourceContextMismatch { operation }` | every multi-resource runtime submission | Resources belong to different `Context` objects. |

`DriverStatus` is a transparent typed wrapper around the numeric CUDA status.
`DriverCallError` stores the operation string, status, optional Driver error
name, and optional Driver description. Discovery and pre-loop realization call
the rich `Driver::check`, while post-realization submit and poll paths call the
allocation-free `check_status_only`. This keeps live-loop error handling
bounded while retaining the exact status.

Artifact failures intentionally remain a separate typed protocol:
`ArtifactCompatibilityError { issues: Vec<ArtifactIssue> }`. Native executor
errors wrap it as an artifact mismatch, while Driver call failures remain
`CudaError::DriverCall` and can be inspected through the source chain.

## Direct consumers and ownership boundaries

The current workspace has exactly these direct Rust consumers of the
`recipe-cuda` package (the set is obtained from `rg -l 'recipe_cuda'`):

| Consumer | What it owns from `recipe-cuda` | Boundary it must not move into this crate |
| --- | --- | --- |
| [`src/facade.rs`](../../../src/facade.rs#L17-L41) | Reexports the package as `recipe::engine::cuda` for advanced callers. | It does not wrap or duplicate CUDA behavior. |
| [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L75-L107) | Loads a configured Driver path, runs `Driver::discover`, builds measured GPU descriptors, creates a `Context`, allocates pinned/device buffers, measures H2D/D2H/D2D and a Recipe-owned cubin launch, and drives `Pending` to completion. | Probe policy, PCI sysfs identity, kernel compilation, benchmark bounds, and measured-profile serialization stay in `recipe-native-probe`. |
| [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L268-L317) | Reopens the exact measured CUDA devices, derives `DeploymentIdentity` from the matching snapshot, creates default-flag contexts, and lends them through lifetime-scoped `CudaBinding` values. | Exact profile matching and HSA binding lifetime stay in the probe crate. |
| [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L15-L33) | Uses `Context`, `DeploymentIdentity`, allocations, modules/functions, streams/events, pending tokens, launch geometry, and artifact compatibility to realize `CudaResources` and implement the backend lifecycle. | Plans, task contracts, arenas, queue ceilings, artifact ABI inspection, and `BackendWork` interpretation stay in `recipe-native-executor`. |
| [`native-executor/src/cuda_ffi.rs`](../../../native-executor/src/cuda_ffi.rs#L1-L67) | Converts validated `KernelAbi` values into `DeviceBuffer` keepalive references, `LaunchConfig`, and `Stream::enqueue_launch` calls. | Parameter block construction and ABI validation remain executor concerns. |
| [`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L18-L24) | Uses CUDA pinned staging buffers, nonblocking streams, completion events, and `Pending` tokens for one-hop cross-backend staged transfers. | Host worker threads, transfer contracts, HSA legs, and local-backend routing remain bridge concerns. |
| [`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs#L19-L28) | Stores `recipe_cuda::ArtifactIdentity` inside `RuntimeArtifactKind::Cuda` and validates the identity/ABI relationship before realization. | Backend-neutral artifact IDs, kernel ABI inspection, and finalized bundle contracts remain executor concerns. |
| [`native-executor/src/error.rs`](../../../native-executor/src/error.rs#L80-L90) | Wraps `CudaError` in the executor's `Error::Cuda` branch and preserves it as a source. | Executor protocol and poisoned-backend errors remain separate variants. |
| [`prepare/src/production.rs`](../../../prepare/src/production.rs#L19-L28) | Uses `ComputeCapability`, `DriverSymbol`, `DriverVersion`, and `ToolchainIdentity` to define `CudaArtifactPolicy`; constructs `recipe_cuda::ArtifactIdentity` for each realized cubin. | Candidate enumeration, compiler realization, and immutable preparation remain in `recipe-prepare`. |
| [`src/native_prepare.rs`](../../../src/native_prepare.rs#L9-L16) | Checks `DeploymentIdentity` capabilities against `REQUIRED_DRIVER_SYMBOLS`, derives the NVIDIA target, and builds the current CUDA toolchain identity. | Measured profile reopening, local scope, and public training/inference handoff remain in the root package. |

The backend-neutral crates (`recipe-core`, planner, scheduler, executor, and
the public declaration frontends) do not depend on CUDA handles. They exchange
typed targets, artifacts, tasks, and measurements with the native layers. A
native call reaches this crate only after those layers have selected a CUDA
device, a target, an image, a queue, and the required lifetime.

The complete cross-crate flow is therefore:

```text
recipe probe
  -> recipe-native-probe: Driver + Discovery + measured descriptor
  -> measured profile/cache

public train/infer preparation
  -> recipe-native-probe: exact reopen + DeploymentIdentity + Context
  -> recipe-prepare: CUDA ArtifactIdentity and RuntimeArtifact
  -> recipe-native-executor: module/buffer/stream/event realization
  -> recipe-cuda: checked Driver operations and completion status
  -> recipe-native-executor: BackendPoll and authoritative state
  -> public report / teardown
```

No consumer is allowed to construct a raw `CUcontext`, call `dlopen`, resolve
a Driver symbol independently, or bypass `Context`'s current-stack guard.
There is one CUDA Driver implementation and one root reexport surface.

## Non-goals and review invariants

The crate root and its modules deliberately do not own:

- Recipe operation semantics, graph construction, primitive lowering, kernel
  compilation, PTX assembly, or cubin inspection.
- Hardware probing policy, PCI sysfs reads, measured profile caches, scheduler
  placement, queue-count policy, arena layout, or cross-backend route choice.
- A global context, a background executor, a log file, cancellation, retries,
  alternate library implementations, or substitute host/device state.
- CUDA Runtime API, HIP, or vendor math/collective libraries.

The safety and lifecycle invariants visible from `lib.rs` are concrete:

1. The crate can only build on Linux and 64-bit targets.
2. A Driver is returned only after all required symbols have been resolved and
   the dynamic library is owned by the same shared `Arc` as the API table.
3. Discovery returns every current device with validated identity and rejects
   duplicate UUIDs instead of selecting one.
4. A Context is thread-affine, and every Driver call is made with that context
   current and then popped with handle verification.
5. Runtime resources borrow their Context; functions borrow their Module;
   event-backed submissions retain operation borrows until terminal completion.
6. Every multi-resource operation rejects a different Context before entering
   the Driver.
7. Explicit destruction is consuming and `Drop` is only a best-effort cleanup
   path. A failed drop is not converted into a second runtime or fallback.
8. Artifact compatibility is checked before native module loading and reports
   all observed issues in one typed value.

These invariants are the reason higher layers can treat this crate as a small
native boundary: Recipe owns meaning, planning, and authoritative state, while
`recipe-cuda` owns only the exact Linux CUDA Driver handles and operations that
have been selected for it.
