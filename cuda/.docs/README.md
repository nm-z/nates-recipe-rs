# `recipe-cuda`

`recipe-cuda` is Recipe's reviewed, dependency-clean boundary to the NVIDIA
CUDA Driver API. It dynamically opens `libcuda.so.1` or `libcuda.so`, resolves
the Driver symbols needed by Recipe, discovers exact devices, creates explicit
CUDA contexts, validates CUDA artifact identities, and owns the native objects
needed for asynchronous copies and kernel launches. It does not use the CUDA
Runtime API, HIP, cuBLAS, cuDNN, NCCL, or any other vendor operation library.

The crate is deliberately below scheduling and execution policy. It does not
compile kernels, choose a placement, create Recipe arena layouts, or interpret
`BackendWork`. `recipe-native-probe` uses it to reopen and measure a real
device. `recipe-prepare` turns its discovery and artifact identities into an
immutable candidate. `recipe-native-executor` translates that candidate into
pre-realized CUDA resources and uses this crate for each native operation.

## Build and platform contract

The [manifest](../Cargo.toml) declares package `recipe-cuda` version `0.1.0`,
edition `2024`, description `Dependency-clean CUDA Driver API backend
scaffolding for Recipe`, MIT license, and the repository
`https://github.com/nm-z/nates-recipe-rs`. It has only two dependencies:

| Dependency | Boundary role |
| --- | --- |
| `libc = "0.2"` | Linux `dlopen`, `dlsym`, `dlerror`, and `dlclose` calls. |
| `sha2 = "0.10"` | SHA-256 checking of a cubin byte image. |

The manifest has no explicit binary, example, test, feature, or library target
section, so Cargo uses the default `src/lib.rs` library target. Workspace lint
settings come from the root manifest; this crate adds no package-local runtime
configuration.

There are no feature flags, build scripts, generated bindings, or static CUDA
link inputs. [The crate root](../src/lib.rs#L1-L14) rejects every target other
than Linux and every pointer width other than 64 bits at compile time. Unsafe
code is allowed only in this crate because the raw Driver calls and dynamic
loader are the reviewed FFI boundary. The root `recipe` facade exposes the
crate at `recipe::engine::cuda` ([`src/facade.rs`](../../src/facade.rs#L17-L41));
the workspace consumers can also depend on `recipe-cuda` directly.

No scheduler, global context, compiler, background worker, logging system, or
test-only implementation is owned here. All successful runtime claims require
the production probe or a complete real workload on an NVIDIA Driver
deployment. `cargo check -p recipe-cuda` checks the Rust and FFI shapes only.

## Module graph and public surface

The private implementation modules are declared in
[`lib.rs`](../src/lib.rs#L16-L22) and are re-exported through one small public
facade ([`lib.rs`](../src/lib.rs#L24-L37)). The dependency direction is:

```text
lib.rs
├── error.rs       status and crate-wide error values
├── ffi.rs         opaque Driver types, function pointers, dlopen/dlsym
├── driver.rs      loaded API owner and status conversion
├── discovery.rs   Driver::discover and exact device snapshot
├── context.rs     context flags, current-context guards, memory observation
├── artifact.rs    artifact/deployment identities and compatibility checks
└── runtime.rs     modules, buffers, streams, events, and pending work
```

`ffi` is the only module that knows C signatures or `libc` loader details.
`driver` owns the loaded `Api` and keeps the library alive. `discovery` and
`context` use that driver through the crate-private fields. `runtime` routes
every operation through a current-context guard and checks resource context
identity before calling the Driver. `artifact` uses discovery and FFI identity
types but never loads a module itself. The root exports these public types:

| Area | Root exports |
| --- | --- |
| Artifacts | `ArtifactIdentity`, `ToolchainIdentity`, `DeploymentIdentity`, `ArtifactField`, `ArtifactIssue`, `ArtifactCompatibilityError`, and `validate_artifact_compatibility`. |
| Context | `Context`, `ContextFlags`, `ContextGuard`, and `SchedulingPolicy`. `MemoryInfo` is the public return type of `Context::memory_info`, although its private module is not re-exported by name. |
| Discovery | `DeviceOrdinal`, `DeviceUuid`, `ComputeCapability`, `DriverVersion`, `DeviceAttributes`, `DeviceInfo`, and `Discovery`. |
| Driver | `Driver` and `ModuleLoadingMode`. |
| FFI inventory | `DriverCapabilities`, `DriverSymbol`, `REQUIRED_DRIVER_SYMBOLS`, and `OPTIONAL_DRIVER_SYMBOLS`. |
| Runtime | `Module`, `Function`, `DeviceBuffer`, `PinnedHostBuffer`, `Dim3`, `LaunchConfig`, `Stream`, `Event`, `Pending`, `CompletionStatus`, and `WaitOutcome`. |
| Errors | `CudaError`, `DriverCallError`, `DriverStatus`, and the `Result<T>` alias. |

### Public input/output inventory

The root exports are intentionally thin wrappers around one input and one
output boundary. No method below accepts a scheduler, compiler, callback for
domain work, or implicit global state.

| Entry point | Inputs | Output and ownership |
| --- | --- | --- |
| `Driver::load` | No arguments; tries the two default library sonames. | A cloneable `Driver` whose `Arc` keeps the dynamic library and resolved function table alive. |
| `Driver::load_from_path` | One UTF-8 Rust `&str` path, rejected if it contains NUL. | The same `Driver` owner, with `loaded_library()` returning the selected path. |
| `Driver::capabilities` / `supports` | A `DriverSymbol` for `supports`. | Borrowed resolved capability set or one boolean; no new lookup occurs. |
| `Driver::discover` | The loaded `Driver`; no device selector. | One immutable `Discovery` snapshot containing version, capabilities, and every device. |
| `DeploymentIdentity::from_discovery` | A snapshot and a borrowed `DeviceInfo`. | `Some(DeploymentIdentity)` only for matching ordinal and UUID, otherwise `None`. |
| `Context::create` | `&Driver`, `&DeviceInfo`, and validated `ContextFlags`. | A context owning one raw `CUcontext` plus cloned Driver and device identity. |
| `Context::enter` / `ContextGuard::leave` | An open context. | A scoped current-context guard; `leave` consumes the guard and verifies the popped handle. |
| `Context::memory_info` | An open context. | One live `MemoryInfo` observation with free and total bytes. |
| `Context::device` / `Context::as_raw` / `Context::close` | An open context for the first two; ownership of the context for `close`. | Borrowed `DeviceInfo`, raw context pointer or `ContextClosed`, and consuming destruction respectively. |
| `Module::load_cubin` / `Module::function` | A context and ELF cubin bytes, then a nonempty NUL-free entry name. | A module owner and a function borrowing that module. |
| `Module::unload`, allocation `free`, `Stream::destroy`, `Event::destroy` | Ownership of the corresponding open native object. | Consuming explicit destruction, with `Drop` as best-effort fallback. |
| `DeviceBuffer::allocate` / `PinnedHostBuffer::allocate` | An open context and nonzero byte length. | A context-borrowing allocation owner with explicit and drop cleanup. |
| `Stream` copy/launch methods | Same-context resources, checked ranges, launch geometry, and, for event-backed methods, one supplied `Event`. | Either a `Pending` token that owns the completion event or a queued operation requiring `poll_idle`. |
| `Event::create_completion` | An open context. | One timing-disabled completion event. |
| `Pending::poll` / `wait` / `recycle_event` | A still-owned submission token, with an optional timeout for `wait`. | Nonblocking status, a complete event or still-owned timed-out token, or a reusable terminal event. |
| `validate_artifact_compatibility` | Cubin bytes, expected and observed identities, and a deployment identity. | `Ok(())` or an accumulated `ArtifactCompatibilityError`; no Driver object is opened. |

The accessors on the value types are similarly non-mutating: ordinals expose
their raw integer, UUIDs expose their original 16 bytes, compute capabilities
expose major/minor, Driver versions expose raw/major/minor, and `Discovery::device`
performs an exact ordinal lookup. `ContextGuard` dereferences to its context
only while the guard is active. `Function::name`, allocation lengths and byte
slices expose state without transferring the underlying native owner.

The value constructors define the remaining validation boundary:

| Value | Constructor behavior |
| --- | --- |
| `DeviceOrdinal` | Has no public constructor; discovery creates it from the Driver ordinal. |
| `DeviceUuid` | `from_bytes([u8; 16])` preserves all bytes without validation. |
| `ComputeCapability` | `new(major, minor)` stores both `u32` values without probing the device. |
| `DriverVersion` | `from_raw(i32)` rejects negatives; `new(major, minor)` uses checked `1000/10` encoding. |
| `ContextFlags` | `new(policy)` starts with both optional bits disabled; builder methods set them; `from_bits` validates the complete mask. |
| `Dim3` | `new(x, y, z)` rejects any zero axis. |
| `LaunchConfig` | `new(grid, block)` sets dynamic shared memory to zero; `with_dynamic_shared_memory` replaces that value. |

## Driver API FFI boundary

### Raw representation

[`ffi.rs`](../src/ffi.rs#L10-L89) maps Driver handles to opaque pointers and
maps `CUdevice` to `c_int`, `CUdeviceptr` to `u64`, and Driver status values to
`c_int`; `CuUuid` is a `repr(C)` 16-byte byte array. The function pointer
signatures cover initialization and discovery,
context stack operations, module and function loading, device and pinned-host
memory, streams and events, asynchronous host/device and device/device copies,
and `cuLaunchKernel`. The only completion constants interpreted here are
`CUDA_SUCCESS` (`0`), `CUDA_ERROR_NOT_READY` (`600`), and
`CUDA_ERROR_NOT_SUPPORTED` (`801`). Device attribute constants are the numeric
Driver values needed to build the measured descriptor: compute capability,
concurrency, warp and block limits, shared memory, clocks, PCI identity,
asynchronous engines, and multiprocessor count.

The private Rust aliases at this boundary are `CuResult = c_int`,
`CuDevice = c_int`, `CuContext = *mut c_void`, `CuDevicePtr = u64`, and
`CuModule`, `CuFunction`, `CuStream`, and `CuEvent` as `*mut c_void`. The
Driver function-pointer aliases (`CuInit`, `CuDriverGetVersion`,
`CuDeviceGetCount`, the device/context/module/memory/stream/event/copy aliases,
`CuLaunchKernel`, and the optional loading/error aliases) all return the same
`CuResult` status and remain crate-private; no raw C type escapes the public
facade.

The discovery attribute constants are the exact Driver values: max threads per
block `1`, max shared memory per block `8`, warp size `10`, core clock `13`,
multiprocessor count `16`, concurrent kernels `31`, PCI bus/device `33/34`,
memory clock `36`, global memory bus width `37`, async engine count `40`, PCI
domain `50`, and compute capability major/minor `75/76`.

`DriverSymbol` is the closed, ordered symbol inventory. `as_str()` supplies the
human-readable name and `c_name()` supplies the NUL-terminated name passed to
`dlsym` ([`ffi.rs`](../src/ffi.rs#L91-L214)). The required and optional sets are
explicit:

```text
required:
  cuInit, cuDriverGetVersion, cuDeviceGetCount, cuDeviceGet,
  cuDeviceGetName, cuDeviceGetUuid, cuDeviceGetPCIBusId,
  cuDeviceTotalMem_v2, cuDeviceGetAttribute,
  cuCtxCreate_v2, cuCtxDestroy_v2, cuCtxPushCurrent_v2, cuCtxPopCurrent_v2,
  cuModuleLoadData, cuModuleUnload, cuModuleGetFunction,
  cuMemAlloc_v2, cuMemFree_v2, cuMemGetInfo_v2,
  cuMemHostAlloc, cuMemFreeHost,
  cuStreamCreate, cuStreamDestroy_v2, cuStreamQuery,
  cuEventCreate, cuEventRecord, cuEventQuery, cuEventDestroy_v2,
  cuMemcpyHtoDAsync_v2, cuMemcpyDtoHAsync_v2, cuMemcpyDtoDAsync_v2,
  cuLaunchKernel

optional:
  cuDeviceGetUuid_v2, cuModuleGetLoadingMode,
  cuGetErrorName, cuGetErrorString
```

`DriverCapabilities` stores the symbols that were actually resolved. Required
symbols are always present if a `Driver` was constructed; optional symbols are
reported through `supports` and `available_symbols` and are never silently
treated as required by the loader. `DriverSymbol` is `#[non_exhaustive]`, so
callers must not assume this inventory can never grow. `available_symbols`
returns a sorted, exact-size iterator over the private `BTreeSet`; no public
method can insert a capability after loading.

The inventory intentionally contains no global context synchronize, stream
synchronize, synchronous copy, or host-side cancellation symbol. Completion is
therefore represented only by event or stream queries, and higher layers must
retain the corresponding pending token or enqueue-only resources until those
queries report terminal completion. It also has no `cuStreamWaitEvent` or
dependency-graph API; dependency order and cross-backend routing are finalized
by `recipe_executor` and the native bridge before this crate is called.

### Dynamic loading and lifetime

[`DynamicLibrary::open_default`](../src/ffi.rs#L445-L466) tries
`libcuda.so.1` first and `libcuda.so` second. `open(path)` rejects an embedded
NUL, clears the thread-local `dlerror`, calls
`dlopen(path, RTLD_NOW | RTLD_LOCAL)`, and preserves the selected path for
diagnostics. `lookup` calls `dlsym` and converts a `dlerror` into a detail
string. A failed default search returns one `CudaError::LibraryOpen` containing
all candidate attempts. `DynamicLibrary` closes its handle in `Drop` and is
marked `Send` and `Sync` because the underlying handle is kept open for the
entire shared driver lifetime; the `dlclose` return value is ignored during
drop.

The two-name fallback applies only to `dlopen` failure. Once one soname opens,
`Api::load` resolves that handle; a missing required symbol returns
`MissingRequiredSymbol` and does not silently reopen the other soname.

`Api::load` resolves every required function before it returns and resolves
optional functions independently ([`ffi.rs`](../src/ffi.rs#L317-L414)). A
missing required pointer returns `MissingRequiredSymbol`; an optional lookup
failure simply produces `None`. `pointer_as_function` checks that the function
pointer and data pointer have the same size before the reviewed
`transmute_copy`. `Driver::from_library` stores the `Api` and library in one
`Arc<DriverInner>`, so cloned `Driver` values share the function table and keep
the dynamic library loaded ([`driver.rs`](../src/driver.rs#L12-L51)). The
resulting `Driver` is `Clone`, `Send`, and `Sync`; `Context` is the deliberately
thread-affine object that prevents a raw current-context handle from crossing
threads.

The internal `SymbolResolver` trait keeps `Api::load` independent of the
loader implementation. In production its only implementation is
`DynamicLibrary`; the trait is not exported and cannot become a second public
FFI path. `required<T>` distinguishes a null symbol from a `dlsym` error and
preserves the symbol name in both cases. `optional<T>` intentionally collapses
either absence or lookup detail into `None`, because optional capabilities are
observed through `DriverCapabilities` rather than required for construction.

### Status conversion

`Driver::check` turns a nonzero status into `CudaError::DriverCall` and, when
available, asks `cuGetErrorName` and `cuGetErrorString` for rich text
([`driver.rs`](../src/driver.rs#L83-L94)). `check_status_only` intentionally
does not allocate or query optional text. Runtime submission and poll paths use
this exact numeric-status path; discovery and pre-loop realization use the
rich path. This keeps live-loop error handling bounded while preserving the
Driver status that caused the failure.

`module_loading_mode` is an optional capability query. Missing
`cuModuleGetLoadingMode` returns `Ok(None)`, raw value `1` means `Eager`, raw
value `2` means `Lazy`, and every other value is
`InvalidDriverValue` ([`driver.rs`](../src/driver.rs#L63-L81)).

## Discovery and deployment identity

### Value types

[`discovery.rs`](../src/discovery.rs#L24-L148) keeps the values used by the
measured profile strongly typed:

* `DeviceOrdinal` wraps the Driver ordinal and exposes only `get()`.
* `DeviceUuid` is exactly 16 bytes and formats as
  `GPU-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` with lowercase hexadecimal bytes.
  `from_bytes` and `as_bytes` do not rewrite the bytes.
* `ComputeCapability` stores `major` and `minor` and formats as `sm_Mm`.
* `DriverVersion` stores the Driver's raw integer and displays
  `<major>.<minor> (<raw>)`. `from_raw` rejects a negative value.
  `new(major, minor)` encodes `major * 1000 + minor * 10` with checked
  arithmetic; `major` and `minor` decode the same representation.
* `DeviceAttributes` records async engines, concurrent kernels, warp size,
  maximum block threads, maximum shared memory per block, PCI domain/bus/device,
  core and memory clocks, memory bus width, and multiprocessor count.
* `DeviceInfo` combines ordinal, UUID, validated name, compute capability,
  exact Driver PCI string, total memory, attributes, and a crate-private raw
  `CUdevice` handle.
* `Discovery` is one snapshot containing Driver version, resolved capabilities,
  and all `DeviceInfo` values. `device(ordinal)` performs an exact lookup.

### Driver::discover sequence

[`Driver::discover`](../src/discovery.rs#L150-L189) is the only discovery
entrypoint. It executes this sequence against the loaded API:

```text
cuInit(0)
  -> cuDriverGetVersion
  -> cuDeviceGetCount
  -> for each raw ordinal: cuDeviceGet and all device fields
  -> reject duplicate UUIDs
  -> return Discovery
```

Discovery is not cached in `Driver`; each call repeats `cuInit`, version/count
queries, and all device field reads, returning a fresh `Discovery` value with a
fresh capability clone.

A negative device count is an invalid Driver value. Each device query is
checked immediately. The Driver name uses a 256-byte C buffer and must be
NUL-terminated, valid UTF-8, and nonempty after trimming
([`discovery.rs`](../src/discovery.rs#L328-L360)). The PCI identity uses a
32-byte C buffer and must be UTF-8, NUL-terminated, and exactly
`dddd:bb:dd.f`, with hexadecimal domain, bus, and device components and one
decimal function digit ([`discovery.rs`](../src/discovery.rs#L297-L326),
[`discovery.rs`](../src/discovery.rs#L406-L428)). A malformed name or PCI value
is not normalized or guessed.

UUID discovery prefers optional `cuDeviceGetUuid_v2`; a status of
`CUDA_ERROR_NOT_SUPPORTED` deliberately falls back to required
`cuDeviceGetUuid`. Any other v2 error is returned. The concurrent-kernel
attribute must be exactly `0` or `1`; all numeric attributes must fit `u32`.
The returned UUID is inserted into a `BTreeMap`; duplicate UUIDs fail the
whole snapshot with `DuplicateDeviceUuid` rather than selecting one ordinal.

### DeploymentIdentity

`DeploymentIdentity::from_discovery` is an existence check across one snapshot,
not a fresh probe. It accepts a `DeviceInfo` only when both its ordinal and UUID
match a device in the supplied `Discovery`, then copies the snapshot Driver
version and capabilities plus the device UUID and target
([`artifact.rs`](../src/artifact.rs#L31-L55)). This is the identity lent to a
native executor context. It prevents a context or artifact from being paired
with a device that was not part of the exact reopening snapshot.

## Context ownership and current-context discipline

### Flags

[`ContextFlags`](../src/context.rs#L11-L93) represents only the R470-safe bits:
one scheduling policy (`Auto`, `Spin`, `Yield`, or `BlockingSync`), `map_host`,
and `local_memory_resize_to_max`. `bits()` produces the Driver mask. `from_bits`
rejects unknown bits and invalid combinations instead of preserving opaque
policy. `ContextFlags::new`, `with_map_host`, and
`with_local_memory_resize_to_max` build a value without contacting the Driver.

The exact mask is scheduling `Auto = 0x00`, `Spin = 0x01`, `Yield = 0x02`,
or `BlockingSync = 0x04`, plus `map_host = 0x08` and
`local_memory_resize_to_max = 0x10`; the known mask is `0x1f`. Scheduling bits
are matched as one exact value, so combinations such as `0x03` are rejected.

### Creation, entry, and destruction

`Context::create(driver, device, flags)` validates the bit set, calls
`cuCtxCreate_v2`, rejects a null success handle, immediately pops the context
with `cuCtxPopCurrent_v2`, and verifies that the popped handle is exactly the
one returned by creation ([`context.rs`](../src/context.rs#L102-L134)). A pop
error or stack mismatch destroys the just-created raw context before returning
the error. The context stores a cloned `Driver`, a cloned `DeviceInfo`, and an
`Option<CUcontext>` so `close` can consume the live resource exactly once.
Creation does not re-query `cuDeviceGet` or prove that a `DeviceInfo` handle
came from the same `Driver` object; the raw handle field is crate-private, so
the production caller must pass the device from the matching `discover`
snapshot. `recipe-native-probe` enforces that pairing while reopening measured
bindings.

`Context` carries `PhantomData<Rc<()>>`; it is intentionally neither `Send` nor
`Sync`. A context is entered only through `enter()`, which pushes its handle
and returns a `ContextGuard`. `ContextGuard::leave` pops and compares the exact
handle. Its `Drop` implementation attempts the same pop if the caller did not
leave explicitly. A closed context yields `ContextClosed` from `enter`,
`as_raw`, memory operations, and runtime resource operations. `close` and every
resource `Drop` path attempt the Driver destroy function and ignore a destroy
error during unwinding.

`runtime::with_current` is the common operation wrapper
([`runtime.rs`](../src/runtime.rs#L21-L43)): enter the context, execute the
operation, then call `ContextGuard::leave` exactly once. If the operation
fails, the leave result is deliberately discarded and the operation's original
error is kept; the wrapper does not create a second cleanup or retry path.
`same_context` compares the actual `Context` object identity, not just a device
ordinal or UUID. Every module, allocation, stream, event, and launch argument
must therefore belong to the exact context used by the operation.

`Context::memory_info` enters the context, queries `cuMemGetInfo_v2`, leaves,
and returns one `MemoryInfo { free_bytes, total_bytes }` observation
([`context.rs`](../src/context.rs#L151-L187)). The native executor uses this
for measured capacity and headroom checks; this crate does not reserve or
subtract scheduler policy from the counters.

## Artifact identity and compatibility

### Identity domains

[`artifact.rs`](../src/artifact.rs#L11-L55) keeps build and deployment identity
separate:

* `ToolchainIdentity` records six nonempty strings: Zig, LLVM, PTX ISA,
  `ptxas`, CUDA toolkit, and cubin format versions.
* `ArtifactIdentity` records the SHA-256 of the exact cubin bytes, target
  `ComputeCapability`, toolchain identity, minimum Driver version, optional
  maximum Driver version, and the exact required `DriverSymbol` set.
* `DeploymentIdentity` records the reopened Driver version, device UUID,
  target capability, and resolved Driver capabilities.

This `recipe_cuda::ArtifactIdentity` is the native Driver-compatibility
identity, not `recipe_core::ArtifactIdentity` used by the finalized graph. The
preparation layer stores both and validates their digest, target, entry, and
ABI relationship before the executor receives a `RuntimeArtifact`.

`validate_artifact_compatibility(cubin, expected, observed, deployment)` is a
pure check. It does not open a module or mutate a context. It accumulates every
observed issue before returning, rather than returning the first mismatch. Its
order is:

1. Reject an empty byte slice and require the ELF magic `0x7fELF`.
2. Compute SHA-256 over the complete byte slice and compare it with
   `observed.sha256`.
3. Compare every expected and observed identity field: digest, target, all six
   toolchain strings, driver range, and required symbol set.
4. Reject toolchain or cubin-format strings that are empty after trimming.
5. Reject an observed maximum Driver lower than its minimum.
6. Require the artifact target and reopened deployment target to match.
7. Require the deployed Driver to lie within the observed range.
8. Require every observed required symbol to be present in deployment
   capabilities.

The result is `Ok(())` only when the issue vector is empty. The check validates
the byte envelope and identity contract, not the full cubin ELF or kernel ABI.
The native executor performs the additional `inspect_cubin` target and entry
inspection before loading a module. `ArtifactCompatibilityError::issues` is
public so callers can retain the complete `ArtifactIssue` values, including
digest, identity, target, range, and missing-symbol details. Its `Display`
implementation reports only the number of collected issues; callers should
inspect `issues` for the actionable fields.
The validator intentionally does not compare a cubin to a device UUID because
the artifact contract is target and Driver based; UUID membership is enforced
when `DeploymentIdentity` is built and when the native executor validates its
context binding.

`ArtifactField` names each identity field used by the comparison:
`Sha256`, `Target`, `ZigVersion`, `LlvmVersion`, `PtxIsaVersion`,
`PtxasVersion`, `CudaToolkitVersion`, `CubinFormat`, `MinimumDriver`,
`MaximumDriver`, and `RequiredDriverSymbols`. The complete `ArtifactIssue`
set is `EmptyArtifact`, `InvalidCubinMagic`, `DigestMismatch`,
`IdentityFieldMismatch`, `EmptyIdentityField`, `InvalidDriverRange`,
`DeviceTargetMismatch`, `DriverTooOld`, `DriverTooNew`, and
`MissingDriverSymbol`. No missing kernel path is treated as a CUDA driver
error here; recompilation and artifact selection are owned by the preparation
layer.

## Runtime resource model

All native resource objects (`Module`, `Function`, `DeviceBuffer`,
`PinnedHostBuffer`, `Stream`, `Event`, and `Pending`) borrow one `Context`
directly or through their parent module; value-only launch and completion
types do not own a Driver handle. Each raw resource handle is stored in an
`Option` so an explicit close consumes it and `Drop` cannot destroy the same
handle twice. Driver calls are wrapped in `with_current`; context mismatches,
closed handles, null success outputs, and out-of-range byte ranges fail before
an unsafe call.

The lifecycle boundary is phase-specific:

| Phase | CUDA crate responsibility | Native executor ownership |
| --- | --- | --- |
| Discovery | Load API, initialize Driver, query immutable device snapshot. | Match measured profile and retain `DeploymentIdentity`. |
| Preparation and realization | Create contexts, allocate pinned/device resources, load cubins, resolve entries, create streams/events. | Validate immutable plan, reservations, ABI, and artifact identity before `LoopStarted`. |
| Init | Expose event-backed H2D admission through `Stream::copy_h2d`. | Supply finalized init image and arena contract. |
| Loop | Expose nonblocking kernel, D2D, and metric operations plus event/stream polling. | Submit only finalized task contracts and rearm terminal loop tokens. |
| Exit | Expose event-backed D2H egress and completion event reuse. | Copy preallocated egress bytes and release arenas in order. |
| Teardown | Destroy events, streams, modules, and allocations after terminal completion. | Reject active tokens, poisoned state, or mismatched ownership. |

`recipe-cuda` performs no scalar math and has no host calculation callback. It
only launches the already-realized cubin ABI and moves bytes. The native
executor's finalized ABI admits Recipe's GPU-owned `f32` and `i32` payloads;
the only host-side typed interpretation in this CUDA path is the four-byte
metric readback after a completed transfer.

These boundaries mirror Recipe's normative system contract: discovery,
compilation or Driver JIT, module loading, argument storage, resource-pool
creation, and queue/event realization finish before `init`; the loop cannot
place, route, compile, load, allocate, resize, discover, or change topology
([`system-contract.md`](../../system-contract.md#L152-L179)). External data
enters through the single finalized init admission and leaves through finalized
exit egress; loop transfers move only pre-owned or scheduled bytes. Scheduler
rates and placements come from the measured profile, not from this crate or the
seed estimates in `topology/contract.toml`.

### Modules and functions

`Module::load_cubin(context, bytes)` accepts a nonempty byte slice beginning
with ELF magic and calls `cuModuleLoadData` while the context is current. It
explicitly rejects PTX and non-ELF input at this boundary, rejects a null module
on a success status, and owns `cuModuleUnload` through `unload` or `Drop`
([`runtime.rs`](../src/runtime.rs#L45-L133)). It does not parse the cubin or
choose its loading mode. `Module::function(name)` rejects an empty name or an
embedded NUL, calls `cuModuleGetFunction`, rejects a null success handle, and
returns a `Function` borrowing the module. `Function::name` returns the owned
entry spelling. The lifetime relation prevents a function from outliving its
module in safe Rust.

### Device and pinned-host memory

`DeviceBuffer::allocate` calls `cuMemAlloc_v2` for a nonzero byte count and
rejects a null success pointer. It exposes `len`, `is_empty` (always `false`
because zero allocation is rejected), `device_ptr`, and consuming `free`
([`runtime.rs`](../src/runtime.rs#L145-L236)). `checked_pointer` uses checked
offset plus length arithmetic, verifies the range stays within the allocation,
and checks conversion and device-pointer overflow before a copy or launch.
`device_ptr` returns the raw `CUdeviceptr` value as `u64` while the allocation
is open; callers cannot construct a typed pointer that bypasses the
range/context checks.

`PinnedHostBuffer::allocate` calls `cuMemHostAlloc` with no extra flags for a
nonzero length, rejects a null success pointer, and zero-fills the allocation
before exposing `as_slice` or `as_mut_slice`. It has the same checked range
rules, reports `is_empty() == false` for every live allocation, and has
explicit/drop destruction through `cuMemFreeHost`
([`runtime.rs`](../src/runtime.rs#L238-L327)). Slice access expects the buffer
to still be open. Both allocation types borrow the context for their complete
lifetimes, so Rust ownership prevents a context from being dropped first.

### Launch geometry

`Dim3::new(x, y, z)` rejects a zero axis. `LaunchConfig::new` combines a grid
and block and defaults dynamic shared memory to zero;
`with_dynamic_shared_memory` changes only that byte count
([`runtime.rs`](../src/runtime.rs#L329-L368)). The crate does not check the
device's maximum block or shared-memory limits here; discovery and the
immutable kernel ABI supply those constraints before the native executor calls
the launch boundary.

### Streams, copies, and launches

`Stream::create_nonblocking` calls `cuStreamCreate` with
`CU_STREAM_NON_BLOCKING = 1`. `poll_idle` maps `cuStreamQuery` through the
completion status conversion. A stream can issue these operations:

| Method | Driver work | Completion ownership |
| --- | --- | --- |
| `copy_h2d` | `cuMemcpyHtoDAsync_v2`, then `cuEventRecord` | Returns `Pending` owning the supplied event and borrows both allocations. |
| `copy_d2h` | `cuMemcpyDtoHAsync_v2`, then `cuEventRecord` | Returns `Pending` owning the event and the mutable host destination borrow. |
| `enqueue_d2h` | `cuMemcpyDtoHAsync_v2` only | Caller must retain stream and allocations until `poll_idle` is complete. |
| `copy_d2d` | `cuMemcpyDtoDAsync_v2`, then `cuEventRecord` | Returns `Pending` owning the event and both allocation borrows. |
| `enqueue_d2d` | `cuMemcpyDtoDAsync_v2` only | Caller must retain stream and allocations until `poll_idle` is complete. |
| `launch` | `cuLaunchKernel`, then `cuEventRecord` | Returns `Pending`; keepalive buffers, function, module, and stream remain borrowed until terminal completion. CUDA copies the parameter values before the call returns. |
| `enqueue_launch` | `cuLaunchKernel` only | Caller must retain function, module, allocations, and stream until `poll_idle`; parameter values only need to be valid for the Driver call. |

The copy methods check that all resources and the event have the exact same
context and that each offset/length range is valid. The unsafe launch methods
accept a mutable array of pointers to host-side argument values. The caller,
not the Driver, must prove that pointer alignment, pointee types and sizes,
argument order, cubin ABI, and launch geometry agree. CUDA copies those values
before either call returns. `launch` additionally requires every referenced
allocation in `keepalive`, which keeps the Rust borrows alive in the returned
token; `enqueue_launch` instead requires the caller to retain those resources
until `poll_idle`. The Driver's `extra` launch parameter is passed as null;
an empty parameter slice is passed as a null parameter pointer, and there is no
dynamic launch-parameter dependency path.

`Stream::destroy` and `Drop` call `cuStreamDestroy_v2`. A stream is not
implicitly synchronized by this crate; the caller must first reach a terminal
poll when using an enqueue-only operation.

### Events and pending work

`Event::create_completion` calls `cuEventCreate` with
`CU_EVENT_DISABLE_TIMING = 2`, rejects a null success handle, and destroys through
`destroy` or `Drop` ([`runtime.rs`](../src/runtime.rs#L699-L747)). Events are
completion objects, not timing samples.

`CompletionStatus` has exactly `Pending` and `Complete`. `cuEventQuery` and
`cuStreamQuery` map status `CUDA_ERROR_NOT_READY` to `Pending`, success to
`Complete`, and every other status to a numeric `DriverCallError`.

`Pending<'op, 'ctx>` is `#[must_use]`, owns one event, records a terminal bit,
and carries a phantom borrow for the operation's referenced Rust resources
([`runtime.rs`](../src/runtime.rs#L749-L828)). `poll` is nonblocking and marks
the token complete when its event reaches `Complete`. `wait(timeout)` repeatedly
polls and yields the host thread until the deadline; an overflowing
`Instant::checked_add` is `InvalidInput`. It returns `WaitOutcome::Complete`
with the event or `WaitOutcome::TimedOut` with the same still-owned pending
token. A timed-out token must not be dropped while the Driver may still access
its resources. `recycle_event` calls one terminal poll and returns the same
event only after completion, which lets warm execution reuse pre-created event
objects without creating replacements.

## End-to-end caller and ownership flow

The direct workspace dependency edges are deliberately few:

| Consumer manifest | Source edge | Responsibility at this boundary |
| --- | --- | --- |
| [`Cargo.toml`](../../Cargo.toml#L56-L66) | [`src/facade.rs`](../../src/facade.rs#L17-L41) | Re-export the crate as `recipe::engine::cuda` for advanced callers. |
| [`native-probe/Cargo.toml`](../../native-probe/Cargo.toml#L1-L13) | [`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L75-L107), [`bindings.rs`](../../native-probe/src/bindings.rs#L268-L317), and [`build.rs`](../../native-probe/build.rs#L20-L33) | Load and discover the configured Driver, benchmark transfers and kernels, reopen exact contexts for preparation, and include CUDA source in the probe identity digest. |
| [`prepare/Cargo.toml`](../../prepare/Cargo.toml#L1-L14) | [`src/native_prepare.rs`](../../src/native_prepare.rs#L651-L702) and [`prepare/src/production.rs`](../../prepare/src/production.rs#L177-L190) | Build the measured CUDA target and immutable Driver/artifact policy. |
| [`native-executor/Cargo.toml`](../../native-executor/Cargo.toml#L1-L14) | [`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L37-L92) | Own pre-realized streams, events, modules, buffers, pending tokens, and finalized task submission. |

No other workspace crate directly imports `recipe-cuda`; higher layers reach it
through these explicit preparation and execution edges. The root facade's
re-export is presentation only and does not create a second CUDA implementation.

The production path has one native context lifetime and one direction of
ownership transfer:

```text
probe configuration
  -> recipe-native-probe::CudaBackend::open
       Driver::load_from_path -> Driver::discover
  -> exact GpuDescriptor and measured profile
  -> recipe-native-probe::bindings::realize_cuda
       reopen Driver/discovery -> DeploymentIdentity -> Context::create
  -> recipe-prepare::cuda_spec/runtime_kind
       measured target + toolchain + Driver symbol policy
       -> RuntimeArtifactKind::Cuda { recipe_cuda::ArtifactIdentity }
  -> recipe-native-executor candidate realization
       validate identity -> load modules/functions -> pre-create queues/events/buffers
  -> finalized recipe_executor::Backend
       bind arenas -> submit init/calc/transfer/metric/exit -> poll -> destroy
```

### `recipe-native-probe`

[`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L75-L107) first
checks for an NVIDIA PCI accelerator and selects the configured Driver library.
It always uses `Driver::load_from_path`, then `discover`; a present NVIDIA
device without a configured library is a discovery failure. Descriptor
construction cross-checks the Driver PCI string against Driver attributes,
canonicalizes the BDF for Linux sysfs, records the exact `sm_Mm` target and
Driver version, and carries the measured warp, block, shared-memory,
concurrency, asynchronous-engine, and queue limits into `GpuDescriptor`
([`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L109-L207)). It
does not substitute ordinal, product name, or capacity when reopening a
profile.

Before a benchmark, `matching_device` requires exactly one reopened descriptor
equal to the measured descriptor; a duplicate match or a disappeared/changed
device is a benchmark failure rather than a nearest-device fallback.

The CUDA probe input is an explicit `CudaProbeConfig`: ordered library
candidate paths, a PTX ISA number encoded as `major * 10 + minor`, the PCI
sysfs root, the host-memory label, and the pinned kernel build configuration.
Before `Driver::load_from_path`,
`selected_library` requires absolute candidates, accepts only regular files or
symlinks, canonicalizes the first existing candidate, hashes its bytes, and
deduplicates canonical paths ([`native-probe/src/config.rs`](../../native-probe/src/config.rs#L6-L20),
[`native-probe/src/identity.rs`](../../native-probe/src/identity.rs#L16-L73)).
The CUDA crate receives that canonical UTF-8 path; it does not itself inspect,
hash, or select among configured candidates.
The probe build script also includes `../cuda/src` and `../cuda/Cargo.toml` in
its sorted source-digest input, together with native build manifests, selected
environment variables, and the rustc identity. A change to this Driver
boundary therefore changes the pinned native-probe source identity and emits a
new `RECIPE_NATIVE_PROBE_SOURCE_DIGEST`
([`native-probe/build.rs`](../../native-probe/build.rs#L20-L135)).

The bounded benchmark creates one `Context` with
`ContextFlags::new(SchedulingPolicy::Yield)` and one nonblocking stream, allocates
pinned host and device buffers, verifies H2D, D2H, and D2D copies, builds a
Recipe-owned cubin, loads its module and entry, launches it with an inspected
ABI, downloads the result, and verifies the output
([`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L248-L446)). Each
event-backed submission is driven to terminal completion. A benchmark timeout
does not cancel CUDA work because the Driver has no cancellation primitive; the
probe continues polling the same token at a bounded cleanup cadence before it
returns a timeout error ([`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L466-L522)).

[`native-probe/src/bindings.rs`](../../native-probe/src/bindings.rs#L268-L317)
reopens every measured CUDA device, constructs `DeploymentIdentity` from that
exact discovery, creates a `Context` with `ContextFlags::default()` (`Auto`
and both optional bits disabled), and lends the context through a `CudaBinding`
only for the preparation/execution callback. The context cannot escape that
borrow scope into a later dynamic placement path.
The root native-preparation adapter rejects duplicate CUDA binding device IDs
before constructing target specifications, so one measured device cannot be
represented by two context owners.

### `recipe-prepare`

`recipe-prepare` owns the immutable build/runtime policy. Its
`CudaArtifactPolicy` stores the CUDA toolchain identity, Driver version range,
and required symbol set ([`prepare/src/production.rs`](../../prepare/src/production.rs#L177-L190)).
`cuda_spec` requires the measured backend `nvidia-cuda-driver`, ABI
`elf64-cubin`, architecture matching the reopened compute capability, and every
required symbol in `DeploymentIdentity::driver_capabilities`
([`src/native_prepare.rs`](../../src/native_prepare.rs#L651-L702)). The policy
does not copy a code-side fallback for a missing Driver feature.

When a realized cubin becomes a native runtime image,
`runtime_kind` constructs `recipe_cuda::ArtifactIdentity` from the exact image
digest, `ComputeCapability`, toolchain, Driver range, and required symbols
([`prepare/src/production.rs`](../../prepare/src/production.rs#L577-L607)).
This identity is stored beside the backend-neutral finalized artifact identity;
the compiler and Driver handles are not retained in the finalized session.

### `recipe-native-executor`

The native executor is the owner of Recipe execution policy. Its public module
documentation states that it validates an immutable `ExecutionPlan`, realizes
Driver resources before the loop, owns exact device arenas, and translates the
closed `BackendWork` set into native asynchronous operations
([`native-executor/src/lib.rs`](../../native-executor/src/lib.rs#L1-L11)).
The CUDA-specific adapter in
[`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L37-L92)
wraps one borrowed `Context` in `CudaBinding` with the exact deployment
identity, device id, bounded queue ceiling, and display-connector count. Its
`available_bytes` method reads `Context::memory_info` immediately before
candidate resource realization. The display-connector count is carried for
preparation headroom policy; `recipe-cuda` itself neither discovers nor
subtracts that policy from Driver memory counters.
After resources are realized, `CudaResources::available_bytes` can issue the
same live context query for the selected device; neither value is a cached
catalog estimate.

`CudaBackend` is a one-shot state machine: `Ready` owns bindings and runtime
images, `Prepared` owns a pre-final candidate, `Warmed` owns resources carried
through stabilization, and `Bound` rejects a second bind
([`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L229-L323)).
The direct `Prepared` handoff remains implemented for finalized prepared
resources, although the current production path carries warmed resources into
`bind_resources`; both paths validate the unchanged bundle and selected task
partition before exposing `CudaResources`.
The Driver API has no finite stream-count attribute, so the adapter's public
`CUDA_MAXIMUM_SUBMISSION_QUEUES` ceiling is the explicit value `32`; a
finalized plan requesting more queues is rejected rather than treated as
unbounded capacity. The adapter reports loop repetition and same-queue
pipelining support to `recipe_executor`, declares
`MAX_NON_POLL_PHYSICAL_CALLS = 1`, and keeps physical calls bounded at the
backend boundary.
`CudaResources::realize` rejects duplicate, missing, or unexpected devices,
checks the bounded submission-queue requirement, validates each binding, and
pre-realizes all device state before `LoopStarted`
([`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L348-L421)).
Binding validation requires the context's discovered UUID and compute capability
to equal its `DeploymentIdentity`; resource realization also requires an
`EnforcedQuota` reservation for every selected device. The init-image byte
count must fit the preallocated pinned staging buffer, while zero-sized or
absent scratch remains absent rather than becoming a dummy allocation.

Each `DeviceResources` value owns the resources that the finalized plan names:

* one `Stream` for each selected finalized queue slot on the device;
* one reusable completion `Event` for each selected completion slot;
* one loaded `Module` per distinct cubin digest and one resolved `Function`
  per logical artifact entry;
* one pre-sized launch `ParameterBlock` for each calculation completion slot;
* one four-byte pinned buffer for each metric whose value resides on the device;
* pinned staging for init admission and exit egress;
* an exact init-image manifest, one preallocated egress byte vector for each
  device-to-external exit task, and optional device scratch.

`CudaBackend::execution_evidence` exposes per-device structural counts for
image loads (`modules.len()`), entry lookups (`artifacts.len()`), queues,
completion objects, and persistent allocations (metric buffers plus staging
and optional scratch). These counts document the realized lifecycle; they are
not a substitute for measured output correctness.

`realize_device` requires `RuntimeArtifactKind::Cuda`, runs
`validate_artifact_compatibility`, inspects the cubin for the deployment SM and
entry symbol, groups identical digests, loads each image once with
`Module::load_cubin`, and resolves every entry before execution
([`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L1671-L1847)).
The current executor call passes the runtime CUDA identity as both `expected`
and `observed` after its backend-neutral `ExecutionPlan` has checked the
finalized identity ([`native-executor/src/plan.rs`](../../native-executor/src/plan.rs#L227-L352));
the deployment argument is still independently checked for target, Driver
range, digest, and required symbols.
Distinct artifact IDs sharing one digest must also carry byte-for-byte equal
images; a digest collision with different bytes is rejected. Equal images are
loaded once per device and each logical entry is resolved from that stable
module. Modules remain in stable `Box` allocations so the borrowed `Function`
values stay valid. Teardown drops functions before modules and unloads each
module after all streams and events are quiescent.

Before those loads, `ExecutionPlan` checks the runtime id, image digest, entry
symbol, target ABI, nonzero workgroup width, and finalized maximum workgroup
width. Its calculation ABI validation additionally derives the expected element
count and read/write operands, requires matching dtype, access, backing-byte
count, power-of-two alignment, and rejects repeated dynamic arguments. A
finalized fault flag, when present, must be an aligned four-byte device `i32`
and must occupy the ABI fault-flag slot. Optional `RunId` and loop-iteration
arguments occur at their canonical suffix positions, and `ElementCount` must
be the final argument ([`native-executor/src/plan.rs`](../../native-executor/src/plan.rs#L261-L352),
[`native-executor/src/plan.rs`](../../native-executor/src/plan.rs#L354-L640)).
This validation is why `fill_invocation` can build one preallocated parameter
block without probing or adapting an ABI in the loop.
The same plan derives each task's immutable native device and queue/completion
slots: calculations use their assigned device, transfers use the resolved
device endpoint, metrics use their finalized value location, and every slot is
validated against the bundle before a pending token can be prepared.
`native-executor::ParameterBlock` owns boxed `u64` argument values, a matching
boxed pointer array, and a preallocated raw-pointer keepalive list. Submission
resets and repopulates those slots, retains every referenced arena buffer, and
then calls `Stream::enqueue_launch`; the only unsafe slice reconstruction is
confined to that adapter ([`native-executor/src/cuda_ffi.rs`](../../native-executor/src/cuda_ffi.rs#L7-L65)).
The launch grid is checked as a ceil-divided element count over ABI workgroup
lanes and must fit `u32`; `Dim3::new` then rejects any zero grid or block axis.

The native operation mapping is fixed by the finalized work class:

| Finalized work | `recipe-cuda` operation | Completion path |
| --- | --- | --- |
| Init admission | `Stream::copy_h2d` from pinned staging to a device arena | Event-backed `Pending`. |
| Calculation | `Stream::enqueue_launch` after `ParameterBlock` ABI/range validation | Stream idle query. |
| Same-context internal transfer | `Stream::enqueue_d2d` | Stream idle query; cross-context routes are rejected. |
| Metric | `Stream::enqueue_d2h` of exactly four bytes to a pre-realized pinned buffer | Stream idle query, then decode as little-endian `f32` or `i32`. |
| Exit egress | `Stream::copy_d2h` from a device arena to pinned staging | Event-backed `Pending`, then copy the preallocated egress vector. |

Exit collection accepts only a terminal `ExitTransfer` token whose task,
device, queue, completion slot, external destination, and byte length equal the
finalized work. It copies from the device's preallocated egress vector into the
caller-provided destination; a missing vector or size mismatch is a protocol
error, not a second download path.
Metric submission similarly requires the resolved value to be exactly four
bytes on the planned submission device and to have a pre-realized pinned result
buffer.
Init admission requires the destination device, image id, image byte count,
arena range, and staging capacity to match the immutable init-image contract.

`CudaResources::prepare_pending` checks the immutable task contract, submission
slots, and realized queue/event slots before returning a `CudaPending` token.
The resource's `prepared_tasks` set rejects preparing the same task twice and
is cleared only by terminal `recycle_pending`.
`submit` validates the same contract and poisons the backend on a native
submission error. `poll` converts a native Driver error into the poisoned
backend state, then uses event polling for event-backed operations and
`Stream::poll_idle` for stream-backed operations. Loop tokens may be rearmed
only after terminal completion; completion events are recycled rather than
recreated. `CudaPending` carries the task, phase, work class, device, queue,
completion slot, and native state, so a pending token cannot be applied to a
different finalized task ([`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L1382-L1449)).
Polling also verifies that the active token still owns its registered queue or
completion slot; an available event or missing queue is a protocol/resource
error rather than an inferred completion.
For loop submissions, a `Ready` token is consumed once, a terminal token is
rearmed for the next iteration, and an active token is rejected as a protocol
error.

The contract builder admits only phase-valid transfer shapes: init external to
device is `InitAdmission`, init or loop device to device is
`InternalTransfer`, and exit device to device or device to external is
`ExitTransfer`. Other endpoint and phase combinations fail as a protocol
error before any stream is touched. Admission also has to equal the finalized
init-image manifest, while internal and exit transfers must carry the exact
finalized route and transfer-lane claims. Each realized CUDA device also needs
the scheduler's `EnforcedQuota` reservation; a theoretical or missing quota is
not converted into a native allocation. Contract construction also rejects
duplicate task identities, missing resolved admission endpoints, and missing
init-image manifests before any task can be submitted.

The adapter erases only the operation lifetime on event-backed tokens when it
hands them to the executor. That one `transmute` is safe only because the
resource table owns every arena and function, the active completion slot is
tracked by task, and teardown refuses to proceed while a token is active
([`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L2188-L2201)).
An abandoned in-flight token is leaked by the executor's cleanup contract
rather than allowing CUDA to access freed Rust resources.

This is also the native-executor integration contract: strict-loop storage is
caller-owned, all CUDA queues, modules, functions, completion events, metric
buffers, egress buffers, pinned staging, and scratch are realized before
`Finalize`, and the finalized handoff attaches offsets without loading a second
copy. A candidate mismatch is fatal and destroys the realized resources in the
defined order; there is no alternate CUDA path
([`native-executor/INTEGRATION_REQUIRED.md`](../../native-executor/INTEGRATION_REQUIRED.md#L1-L61)).
The dependency-neutral `CandidateSessionFactory` requires every exact artifact,
reservation, and resource-manifest object during `realize_candidate`, exercises
the candidate's maximum-concurrency trace on each numbered warm pass, takes a
capacity snapshot only after a complete pass, and deterministically destroys
partial or complete sessions ([`native-executor/src/candidate.rs`](../../native-executor/src/candidate.rs#L243-L278)).

Native CUDA teardown is ordered and observable. `destroy_devices` first queries
every queue and requires `CompletionStatus::Complete`; an active stream returns
`CudaContract`. It then requires every completion slot to be `Available`,
destroys those events, destroys the streams, drops the loaded function holders,
unloads each distinct module, frees metric buffers, frees pinned staging, and
finally frees optional device scratch
([`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L2203-L2245)).
`CudaResources::destroy` first rejects a poisoned backend, so a native failure
remains visible instead of being hidden by a best-effort alternate teardown
path. `CudaPending::Drop` changes its native state to terminal; if an active
submission is dropped before terminal polling, it deliberately forgets the
native token, preserving the CUDA resource lifetime rather than issuing an
unsafe destroy.

`LocalBackend` composes this adapter with host, HSA, and the staged bridge. It
projects one finalized arena map into the selected child, routes CUDA tasks to
`CudaResources`, and keeps physical call accounting at the composite boundary
([`native-executor/src/local.rs`](../../native-executor/src/local.rs#L1611-L1867)).
`LocalBackend::new` receives the optional host configuration, CUDA bindings and
runtime artifacts, HSA bindings and artifacts, and one cross-backend bridge;
it constructs the child `CudaBackend` in `Ready` state and derives one declared
device-class map before any bundle is bound.
During `bind_resources`, local partitioning assigns each finalized task to its
single host, CUDA, HSA, or bridge owner; a CUDA pending token is rejected if the
local owner map names another class or device.
`StagedCrossBackend` uses pinned CUDA staging, one nonblocking `Stream`, and one
completion `Event` per CUDA transfer leg before the loop
([`native-executor/src/bridge.rs`](../../native-executor/src/bridge.rs#L169-L333));
this is the only CUDA surface used for a cross-backend one-hop transfer. A CUDA
source leg performs `copy_d2h` into its pinned staging and a CUDA destination
leg performs `copy_h2d` from that staging; each leg retains its event-backed
pending token until terminal polling and recycles the same event for the next
loop iteration ([`native-executor/src/bridge.rs`](../../native-executor/src/bridge.rs#L1514-L1568)).
Composite destruction attempts bridge, HSA, CUDA, and host children in its
deterministic order; CUDA resources therefore remain owned by the child rather
than being duplicated by the composite backend
([`native-executor/src/local.rs`](../../native-executor/src/local.rs#L2064-L2109)).

Errors retain their layer of origin. `native-probe` prefixes load and discovery
failures as probe discovery errors and prefixes benchmark failures as native
benchmark errors ([`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L101-L105),
[`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L520-L522)).
`recipe-native-executor` converts `CudaError` into its `Error::Cuda` variant
([`native-executor/src/error.rs`](../../native-executor/src/error.rs#L83-L84),
[`native-executor/src/error.rs`](../../native-executor/src/error.rs#L231-L232)),
while the staged bridge converts it into `StagedBridgeError::Cuda`
([`native-executor/src/bridge.rs`](../../native-executor/src/bridge.rs#L157-L163));
neither conversion changes the numeric Driver status or replaces the operation
with a retry. The root preparation path reports binding and identity failures
as `NativePreparationError::Probe` and candidate realization failures through
its `CandidateFailure`/native-preparation wrapper; it does not suppress the
source detail. Native executor teardown and contract failures are kept distinct
from `Error::Cuda`: `CudaContract` identifies an active stream or invalid
resource teardown, `BackendPoisoned` identifies a prior native failure, and
`SubmissionQueueLimitExceeded` retains the requested and maximum queue counts.

## Invariants and constraints

These are the properties enforced by this crate or required by its callers:

1. **Exact platform:** production support is Linux, 64-bit CUDA Driver ABI;
   unsupported targets fail compilation.
2. **Driver-only boundary:** all native calls use resolved CUDA Driver symbols;
   no CUDA Runtime API, HIP, or vendor math library can enter through this
   crate.
3. **Required symbols first:** a `Driver` cannot exist with a missing required
   symbol. Optional symbols are capabilities, not fallbacks for required work.
4. **Current context:** every context-sensitive call enters and leaves the
   exact `Context`; resources from a different context return
   `ResourceContextMismatch` before the Driver call.
5. **Context stack integrity:** create and guard leave operations verify the
   exact handle that was pushed or created.
6. **Nonzero resources:** device and pinned allocations, launch dimensions,
   kernel names, and module byte images have explicit nonempty checks.
7. **Bounds and overflow:** every byte range, pointer offset, Driver-version
   encoding, host-size conversion, and grid calculation is checked before FFI.
8. **Artifact identity:** SHA-256, ELF envelope, target, toolchain, Driver
   range, and required symbols must agree with the deployment; full cubin ABI
   inspection remains a native-executor responsibility.
9. **Asynchronous lifetime:** a pending token or enqueue-only operation must be
   driven to terminal stream/event completion before any referenced allocation,
   module, function, event, or stream is destroyed.
10. **No implicit cancellation:** CUDA work has no cancellation path in this
    boundary. A timeout retains the same token until completion.
11. **Immutable execution:** discovery, compilation, allocation, module load,
    and event/queue creation are pre-loop work. Finalized loop submissions only
    consume the resources and contracts prepared for them.
12. **One-shot ownership:** explicit `free`, `destroy`, `unload`, and `close`
    consume their object; `Drop` is best-effort cleanup for the remaining raw
    handle and never reports a second destroy as success.

## Error model

`Result<T>` is `core::result::Result<T, CudaError>`
([`error.rs`](../src/error.rs#L1-L4)). `DriverCallError` preserves the operation,
numeric `DriverStatus`, and optional name/description text. `CudaError` is
non-exhaustive and currently contains:

| Variant group | Variants and meaning |
| --- | --- |
| Loader | `LibraryOpen { attempts }`, `InvalidLibraryPath { path }`, `MissingRequiredSymbol { symbol, detail }`. |
| Driver values and calls | `DriverCall(DriverCallError)`, `InvalidDriverValue { operation, detail }`. |
| Caller input | `InvalidInput { operation, detail }`, `InvalidContextFlags { bits }`. |
| Discovery identity | `InvalidDeviceName { ordinal, detail }`, `DuplicateDeviceUuid { first_ordinal, second_ordinal }`. |
| Context lifetime | `ContextStackMismatch`, `ContextClosed`. |
| Resource ownership | `ResourceContextMismatch { operation }`. |

`CudaError` implements `Display` and `std::error::Error`; only
`DriverCall` exposes a source error (`DriverCallError`). Driver text is
optional because older deployments may not export the two optional error-text
symbols. Runtime errors remain visible as the exact numeric status when those
symbols are absent or when the status-only path is used.

`DriverStatus` formats as `CUDA status <raw>`. `DriverCallError` formats as
`<operation> failed with <status>`, then appends the optional Driver name in
parentheses and optional description after a colon. The other `CudaError`
messages include the operation and validation detail, so conversion layers can
add backend, task, or artifact context without losing the original status.

Artifact validation deliberately has its own `ArtifactCompatibilityError`
instead of collapsing identity failures into `CudaError`. Its public `issues`
vector preserves all `ArtifactIssue` values for preparation and executor
diagnostics. Native-executor and probe layers wrap these errors with their
task, artifact, or backend context; this crate does not invent a retry or
alternate resource path.

## Verification pointers

The source locations above are the authoritative implementation references.
Useful structural checks are:

| Requirement | Direct evidence |
| --- | --- |
| Package metadata and dependency graph | [`cuda/Cargo.toml`](../Cargo.toml), `cargo tree -p recipe-cuda --depth 2`. |
| Platform and unsafe boundary | [`cuda/src/lib.rs`](../src/lib.rs#L1-L14). |
| Raw ABI, symbol gating, and loader ownership | [`cuda/src/ffi.rs`](../src/ffi.rs#L10-L531), [`cuda/src/driver.rs`](../src/driver.rs#L12-L142). |
| Device discovery and deployment identity | [`cuda/src/discovery.rs`](../src/discovery.rs#L24-L428), [`cuda/src/artifact.rs`](../src/artifact.rs#L31-L55). |
| Context stack and memory observation | [`cuda/src/context.rs`](../src/context.rs#L11-L240). |
| Artifact compatibility | [`cuda/src/artifact.rs`](../src/artifact.rs#L57-L259). |
| Native modules, memory, streams, events, and pending lifetimes | [`cuda/src/runtime.rs`](../src/runtime.rs#L21-L839). |
| Production Driver reopening and benchmark | [`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L75-L522), [`native-probe/src/bindings.rs`](../../native-probe/src/bindings.rs#L268-L317). |
| Preparation policy and runtime CUDA identity | [`src/native_prepare.rs`](../../src/native_prepare.rs#L651-L702), [`prepare/src/production.rs`](../../prepare/src/production.rs#L177-L607). |
| Finalized native ownership and operation mapping | [`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs#L348-L2245), [`native-executor/src/plan.rs`](../../native-executor/src/plan.rs#L227-L690). |
| Normative phase and interface constraints | [`system-contract.md`](../../system-contract.md#L152-L191), [`system-contract.md`](../../system-contract.md#L274-L292). |

```text
cargo tree -p recipe-cuda --depth 2
cargo check -p recipe-cuda
cargo doc -p recipe-cuda --no-deps
```

The first command should show only direct dependency lines for `libc` and
`sha2` below this crate (plus their transitive `sha2` support crates). The
second command proves the Linux 64-bit Rust/FFI shape, not Driver availability
or asynchronous correctness. Real evidence comes from the production
`recipe probe` path and the complete CUDA acceptance workflow, which exercise
`Driver::load_from_path`, `discover`, context creation, artifact loading,
asynchronous submission, polling, egress, and ordered teardown on the actual
deployment.
