# CUDA error model

[`cuda/src/error.rs`](../../src/error.rs) is the single error vocabulary for
the `recipe-cuda` crate. The crate does not use the CUDA Runtime API. It loads
the Linux CUDA Driver ABI, validates driver-owned values, and exposes one
typed result boundary:

```rust
pub type Result<T> = core::result::Result<T, CudaError>;
```

The module is private inside `recipe-cuda`, but
[`cuda/src/lib.rs`](../../src/lib.rs:16-38) reexports `CudaError`,
`DriverCallError`, `DriverStatus`, and `Result`. The enum is
`#[non_exhaustive]`, so callers must not assume that the current variant list
is permanent. Every error is `Clone`, `Debug`, `Eq`, and `PartialEq`; no error
is a status-only Boolean or a recoverable fallback.

## Error flow

The runtime path has four boundaries. Each boundary keeps the failure from
being mistaken for a missing device or a successful asynchronous operation:

```text
dlopen/dlsym and CUDA Driver calls
    -> recipe-cuda::Result<T, CudaError>
    -> native-probe::ProbeError (discovery or benchmark text)
    -> native preparation or native-executor error
    -> probe, training, inference, or CLI failure text
```

The first arrow is typed. The later wrappers are deliberately owned by their
respective crates and are described below. In particular, a missing CUDA
library is not converted to `Ok(None)` when NVIDIA PCI hardware is present,
and a driver status is not converted to a fake completion.

## The two driver detail types

### `DriverStatus`

[`DriverStatus`](../../src/error.rs:6-10) preserves the raw signed CUDA
status integer. Its `Display` implementation is exactly `CUDA status N`, so
unknown or newer driver codes remain diagnosable instead of being collapsed
to an invented Rust enum.

### `DriverCallError`

[`DriverCallError`](../../src/error.rs:12-31) records:

| Field | Meaning |
| --- | --- |
| `operation` | The static CUDA operation label supplied by the caller, such as `cuDeviceGetCount` or `cuLaunchKernel`. |
| `status` | The exact numeric status wrapped in `DriverStatus`. |
| `name` | Optional text returned by `cuGetErrorName`. |
| `description` | Optional text returned by `cuGetErrorString`. |

Its display form is `<operation> failed with CUDA status N`, followed by
`(name)` when present and `: description` when present
([`error.rs:20-29`](../../src/error.rs:20-29)). It implements
`std::error::Error`, but has no nested source. The driver status and optional
texts are the complete cause payload.

## `CudaError` variants

The following table names every variant in the current enum and the source
that constructs it. The implementation links are the authoritative behavior;
the wording in the table describes the observed consequence of returning that
variant.

| Variant | Construction sites and cause | Display and consequence |
| --- | --- | --- |
| `LibraryOpen { attempts }` | [`ffi.rs:454-466`](../../src/ffi.rs:454-466) aggregates failed default candidates `libcuda.so.1` and `libcuda.so`; [`ffi.rs:474-486`](../../src/ffi.rs:474-486) creates one attempt from `dlerror` when an explicit path cannot be opened. | Displays `unable to open a CUDA Driver library` followed by each `path: dlerror` attempt. `Driver::load` or `load_from_path` stops before symbol loading, discovery, or context creation. |
| `InvalidLibraryPath { path }` | [`ffi.rs:468-473`](../../src/ffi.rs:468-473) maps `CString::new(path)` failure, which means the configured path contains an interior NUL. | Displays the path with debug quoting. No `dlopen` is attempted. |
| `MissingRequiredSymbol { symbol, detail }` | [`ffi.rs:416-432`](../../src/ffi.rs:416-432) is used by `Api::load` for every symbol in [`REQUIRED_DRIVER_SYMBOLS`](../../src/ffi.rs:216-249). `Ok(None)` means no symbol and gives `None`; a resolver error gives `Some(detail)`. | Displays `CUDA Driver is missing required symbol NAME`, optionally with the resolver detail. API construction fails, so no public `Driver` is returned. Optional symbols are not errors: [`ffi.rs:435-438`](../../src/ffi.rs:435-438) drops optional lookup failures. |
| `DriverCall(DriverCallError)` | [`driver.rs:83-94`](../../src/driver.rs:83-94) is the rich conversion used by `Driver::check`; [`driver.rs:96-109`](../../src/driver.rs:96-109) is the allocation-free conversion used by `check_status_only`. | Delegates to `DriverCallError`. The operation that received a non-success CUDA status fails immediately. Rich name and description text is available only through `check`; submit and poll paths retain the exact numeric status but intentionally omit allocations and lookup text. |
| `InvalidDriverValue { operation, detail }` | Used whenever the Driver reports a value that violates Recipe's ABI or representation contract: driver version and device count in [`discovery.rs:71-92`](../../src/discovery.rs:71-92) and [`discovery.rs:151-169`](../../src/discovery.rs:151-169); Boolean attributes in [`discovery.rs:211-224`](../../src/discovery.rs:211-224); malformed PCI text, memory width, and negative attributes in [`discovery.rs:297-324`](../../src/discovery.rs:297-324) and [`discovery.rs:379-402`](../../src/discovery.rs:379-402); unknown module-loading mode in [`driver.rs:63-80`](../../src/driver.rs:63-80); successful calls returning null context, module, function, device pointer, host pointer, stream, or event in [`context.rs:103-115`](../../src/context.rs:103-115) and [`runtime.rs:51-109`](../../src/runtime.rs:51-109), [`runtime.rs:152-171`](../../src/runtime.rs:152-171), [`runtime.rs:244-263`](../../src/runtime.rs:244-263), [`runtime.rs:375-394`](../../src/runtime.rs:375-394), and [`runtime.rs:704-723`](../../src/runtime.rs:704-723). | Displays `<operation> returned an invalid value: detail`. The constructor that observed the bad value returns no partially initialized resource or discovery record. |
| `InvalidInput { operation, detail }` | Rejects caller data before a Driver call: non-ELF cubin bytes in [`runtime.rs:51-57`](../../src/runtime.rs:51-57); NUL or empty kernel names in [`runtime.rs:77-90`](../../src/runtime.rs:77-90); zero-byte allocations in [`runtime.rs:152-158`](../../src/runtime.rs:152-158) and [`runtime.rs:244-251`](../../src/runtime.rs:244-251); out-of-bounds or overflowing buffer ranges and offsets in [`runtime.rs:187-213`](../../src/runtime.rs:187-213) and [`runtime.rs:290-305`](../../src/runtime.rs:290-305); zero launch dimensions in [`runtime.rs:336-345`](../../src/runtime.rs:336-345); an `Instant` deadline overflow or recycling a still-pending event in [`runtime.rs:788-821`](../../src/runtime.rs:788-821). | Displays `invalid input for OPERATION: detail`. The operation is rejected locally, so no asynchronous work is enqueued for that call. |
| `InvalidDeviceName { ordinal, detail }` | [`discovery.rs:328-359`](../../src/discovery.rs:328-359) rejects a device name that is not NUL-terminated within the fixed 256-byte buffer, is not UTF-8, or becomes empty after trimming. | Displays `CUDA device ORDINAL returned an invalid name: detail`. The whole discovery result fails rather than retaining an unusable device. |
| `DuplicateDeviceUuid { first_ordinal, second_ordinal }` | [`discovery.rs:171-181`](../../src/discovery.rs:171-181) detects a repeated UUID while building one discovery snapshot. | Displays both ordinals. Discovery stops and returns no accepted inventory, preventing ambiguous device identity. |
| `InvalidContextFlags { bits }` | [`context.rs:76-92`](../../src/context.rs:76-92) rejects unknown bits or a scheduling mask that is not one of the four R470-safe policies. `Context::create` round-trips its own bits through this validator at [`context.rs:103-105`](../../src/context.rs:103-105). | Displays the complete bit set as `invalid R470-safe CUDA context flag set 0x...`. Context creation does not reach `cuCtxCreate_v2`. |
| `ContextStackMismatch` | [`context.rs:117-126`](../../src/context.rs:117-126) rejects a different context returned by the pop performed after creation; [`context.rs:214-224`](../../src/context.rs:214-224) rejects a different context returned by a `ContextGuard`. | Displays that CUDA returned a context different from the one Recipe pushed. The newly created context is destroyed on the creation mismatch path, and the guard reports the mismatch after the pop. |
| `ContextClosed` | Every `Option`-backed handle uses this sentinel after ownership is consumed: context entry, raw access, destroy, and guard pop in [`context.rs:138-149`](../../src/context.rs:138-149) and [`context.rs:172-225`](../../src/context.rs:172-225); module, device and host buffers, streams, events, and all async submission handles in [`runtime.rs:77-120`](../../src/runtime.rs:77-120), [`runtime.rs:183-223`](../../src/runtime.rs:183-223), [`runtime.rs:290-315`](../../src/runtime.rs:290-315), [`runtime.rs:396-427`](../../src/runtime.rs:396-427), [`runtime.rs:602-603`](../../src/runtime.rs:602-603), [`runtime.rs:675-735`](../../src/runtime.rs:675-735), and [`runtime.rs:771-821`](../../src/runtime.rs:771-821). | Displays `CUDA context is already closed`. A successful or failed explicit `destroy` takes the raw handle first, so a later call cannot retry it and returns `ContextClosed`. |
| `ResourceContextMismatch { operation }` | [`runtime.rs:37-43`](../../src/runtime.rs:37-43) compares context identity before every copy, event record, and kernel launch. The checks are used by the H2D, D2H, D2D, and launch methods at [`runtime.rs:420-603`](../../src/runtime.rs:420-603). | Displays that `operation` received resources from different contexts. The wrapper refuses the operation before calling CUDA, preventing a cross-context pointer or event submission. |

`DriverCall` is the only `CudaError` variant with a nested source. The
`source` implementation in [`error.rs:132-139`](../../src/error.rs:132-139)
returns the contained `DriverCallError` only for that variant and returns
`None` for every validation, identity, and lifecycle variant. This is why a
caller must inspect the variant fields, not only an error-chain walk, to
distinguish malformed driver output from a closed resource.

## Status conversion and optional driver text

The raw constants are defined in [`ffi.rs:19-21`](../../src/ffi.rs:19-21):

| Numeric status | Meaning in this crate |
| --- | --- |
| `0` (`CUDA_SUCCESS`) | A Driver call succeeded. `Driver::check` and `check_status_only` return `Ok(())`. |
| `600` (`CUDA_ERROR_NOT_READY`) | Only a stream or event query may use this as normal progress. [`runtime.rs:830-838`](../../src/runtime.rs:830-838) maps it to `CompletionStatus::Pending`; it is not a `CudaError`. |
| `801` (`CUDA_ERROR_NOT_SUPPORTED`) | A `cuDeviceGetUuid_v2` result means that the optional v2 UUID query is unavailable for this driver/device. [`discovery.rs:362-376`](../../src/discovery.rs:362-376) falls back to required `cuDeviceGetUuid`. Any other v2 failure becomes `DriverCall`. |

All other nonzero statuses become `DriverCall`. `Driver::check` first calls
the optional error-name and error-string functions through
[`driver.rs:111-132`](../../src/driver.rs:111-132). If an optional function is
absent, itself returns a non-success status, or returns a null text pointer,
that piece of detail is simply `None`; it does not replace the original
status or create a second error. `module_loading_mode` has the same optional
surface behavior: an absent `cuModuleGetLoadingMode` returns `Ok(None)` at
[`driver.rs:63-66`](../../src/driver.rs:63-66), while only raw modes `1` and
`2` are accepted at [`driver.rs:67-80`](../../src/driver.rs:67-80).

`check_status_only` exists for post-realization submission and polling. Its
comment and implementation at [`driver.rs:96-109`](../../src/driver.rs:96-109)
make the allocation boundary explicit: live-loop failures preserve the exact
numeric status but do not query text. It is used for copies, event recording,
kernel launches, and nonblocking resource operations throughout
[`runtime.rs:428-672`](../../src/runtime.rs:428-672).

### `DriverCall` operation inventory

Every `DriverCall` operation label comes from one of the following call sites.
The rich `check` labels cover discovery, setup, and explicit teardown:

| Source | Operation labels |
| --- | --- |
| [`driver.rs:63-70`](../../src/driver.rs:63-70) | `cuModuleGetLoadingMode` |
| [`context.rs:107-177`](../../src/context.rs:107-177) | `cuCtxCreate_v2`, `cuCtxPopCurrent_v2(after create)`, `cuCtxPushCurrent_v2`, `cuMemGetInfo_v2`, `cuCtxDestroy_v2` |
| [`context.rs:217-219`](../../src/context.rs:217-219) | `cuCtxPopCurrent_v2` |
| [`discovery.rs:151-395`](../../src/discovery.rs:151-395) | `cuInit`, `cuDriverGetVersion`, `cuDeviceGetCount`, `cuDeviceGet`, `cuDeviceGetPCIBusId`, `cuDeviceGetName`, `cuDeviceGetUuid_v2`, `cuDeviceGetUuid`, `cuDeviceTotalMem_v2`, and `cuDeviceGetAttribute(...)` for each named CUDA attribute |
| [`runtime.rs:60-127`](../../src/runtime.rs:60-127) | `cuModuleLoadData`, `cuModuleGetFunction`, `cuModuleUnload` |
| [`runtime.rs:161-230`](../../src/runtime.rs:161-230) | `cuMemAlloc_v2`, `cuMemFree_v2` |
| [`runtime.rs:254-321`](../../src/runtime.rs:254-321) | `cuMemHostAlloc`, `cuMemFreeHost` |
| [`runtime.rs:379-388`](../../src/runtime.rs:379-388) | `cuStreamCreate` |
| [`runtime.rs:680-741`](../../src/runtime.rs:680-741) | `cuStreamDestroy_v2`, `cuEventCreate`, `cuEventDestroy_v2` |

The allocation-free `check_status_only` labels cover live submissions and
queries:

| Source | Operation labels |
| --- | --- |
| [`runtime.rs:429-476`](../../src/runtime.rs:429-476) | `cuMemcpyHtoDAsync_v2`, `cuMemcpyDtoHAsync_v2`, `cuEventRecord` |
| [`runtime.rs:503-569`](../../src/runtime.rs:503-569) | `cuMemcpyDtoHAsync_v2`, `cuMemcpyDtoDAsync_v2` |
| [`runtime.rs:610-671`](../../src/runtime.rs:610-671) | `cuLaunchKernel`, with `cuEventRecord` on event-backed launch |
| [`runtime.rs:777-835`](../../src/runtime.rs:777-835) | `cuEventQuery` and `cuStreamQuery`; `CUDA_ERROR_NOT_READY` is progress, all other non-success values become `DriverCall` |

Thus an operation name in a rendered `DriverCall` is always one of these
static labels or a discovery attribute label. It is not a user-provided
string and cannot identify a different hidden implementation.

`query_status` is the only status-to-progress mapping. A successful query is
`Complete`, `CUDA_ERROR_NOT_READY` is `Pending`, and every other value is
passed to `check_status_only` and therefore returned as `DriverCall`
([`runtime.rs:771-838`](../../src/runtime.rs:771-838)). A pending submission is
not a failure: `Pending::wait` returns `WaitOutcome::TimedOut` with ownership
of the live token when its deadline expires, and `Pending::recycle_event`
returns `InvalidInput` only when called before terminal completion.

## Construction and propagation by source module

### Dynamic loading and FFI

`DynamicLibrary::open_default` tries the two configured sonames in order and
aggregates only `LibraryOpen` attempts. `open(path)` turns an interior NUL
into `InvalidLibraryPath`, `dlopen` failure into one `LibraryOpen` attempt,
and a successful handle into a `DynamicLibrary`
([`ffi.rs:445-493`](../../src/ffi.rs:445-493)). `Api::load` requires every
entry in the required symbol list. `required` preserves whether lookup
returned no pointer or a resolver detail; `optional` intentionally ignores
lookup errors and stores `None` for optional capabilities
([`ffi.rs:317-438`](../../src/ffi.rs:317-438)).

`Driver::load` and `Driver::load_from_path` propagate those results with `?`
before constructing `Driver` ([`driver.rs:32-51`](../../src/driver.rs:32-51)).
`Driver::discover` then propagates status and representation failures through
initialization, version, count, UUID, name, PCI, memory, and attribute
collection. A duplicate UUID or any invalid device field aborts the entire
`Discovery` value, not just one device
([`discovery.rs:150-188`](../../src/discovery.rs:150-188)).

### Context ownership

`Context::create` validates flags, calls `cuCtxCreate_v2`, rejects a null
success, pops the newly current context, and verifies that the popped handle
is exactly the created handle. If the pop call fails or returns a different
handle, it makes one best-effort raw destroy call and returns the original
driver or stack error ([`context.rs:102-133`](../../src/context.rs:102-133)).

`Context::memory_info` always attempts to leave its guard. If the memory query
fails, a leave failure is ignored so the original query error remains; if the
query succeeds, a leave failure is propagated
([`context.rs:151-166`](../../src/context.rs:151-166)). `ContextGuard::drop`,
`Context::drop`, and every runtime resource `Drop` implementation discard
driver failures because `Drop` cannot return a `Result`
([`context.rs:189-239`](../../src/context.rs:189-239) and
[`runtime.rs:123-132`](../../src/runtime.rs:123-133),
[`runtime.rs:226-235`](../../src/runtime.rs:226-235),
[`runtime.rs:317-326`](../../src/runtime.rs:317-326),
[`runtime.rs:687-696`](../../src/runtime.rs:687-696), and
[`runtime.rs:737-747`](../../src/runtime.rs:737-747)). Explicit `close`,
`free`, `destroy`, or `unload` methods are the only teardown paths that expose
the `DriverCall` to a caller.

### Runtime validation and asynchronous calls

`with_current` pushes the exact context, runs one operation, and then pops it
([`runtime.rs:21-35`](../../src/runtime.rs:21-35)). An operation error wins
over a pop error; a successful operation propagates a pop error. Every copy
and launch validates context identity, handle state, buffer ranges, and event
state before entering `with_current`. The Driver call and, for event-backed
operations, the event record are sequenced in one closure. If either
`check_status_only` call fails, the submission returns `DriverCall` and no
`Pending` token is produced.

When submission succeeds, the returned `Pending` owns the completion event
and carries a phantom borrow of the operation resources. Its `poll` method
returns `Pending` or `Complete` through the status mapping above. Once an
event reaches `Complete`, later polls return `Complete` without another Driver
call; `wait` takes the event only for a completed outcome
([`runtime.rs:755-805`](../../src/runtime.rs:755-805)). The public safety
contracts require callers to retain and drive a pending token to terminal
completion. Dropping an event-backed token while the GPU still accesses its
buffers violates that contract even though the Rust `Drop` implementation
cannot report the resulting driver cleanup status.

## Propagation into the native probe

`native-probe/src/cuda.rs` is the first adapter above `recipe-cuda`:

* `CudaBackend::open` maps `Driver::load_from_path` to
  `ProbeError::Discovery("load CUDA Driver: ...")` and `Driver::discover` to
  `ProbeError::Discovery("exhaustive CUDA discovery: ...")`
  ([`native-probe/src/cuda.rs:86-106`](../../../native-probe/src/cuda.rs:86-106)).
* `discover`, `benchmark`, and exact-device reopening propagate those
  `ProbeError` values through the `GpuDiscovery` and `GpuBenchmarkIo`
  boundaries. `NativeGpuProbe::discover_all` and `benchmark_gpu` use `?`
  without treating a failed existing NVIDIA backend as an absent backend
  ([`native-probe/src/native.rs:33-39`](../../../native-probe/src/native.rs:33-39),
  [`native-probe/src/native.rs:245-298`](../../../native-probe/src/native.rs:245-298)).
* Native benchmark resource operations use
  `map_err(cuda_benchmark_error)`. That function renders
  `ProbeError::Benchmark("CUDA native operation: {error}")`
  ([`native-probe/src/cuda.rs:248-322`](../../../native-probe/src/cuda.rs:248-322),
  [`native-probe/src/cuda.rs:400-518`](../../../native-probe/src/cuda.rs:400-518),
  [`native-probe/src/cuda.rs:520-522`](../../../native-probe/src/cuda.rs:520-522)).
  It covers context, stream, allocation, event, module, function, launch,
  copy, poll, and event-recycling failures. Timeout is a separate benchmark
  message after the still-pending operation reaches completion.
* Preparation-scoped reopening calls `backend.open()?` and wraps context
  creation as a discovery string in [`bindings.rs:268-316`](../../../native-probe/src/bindings.rs:268-316).
  `with_native_execution_bindings` therefore returns `ProbeError`, not a
  live `CudaError`, to its callback boundary
  ([`bindings.rs:130-215`](../../../native-probe/src/bindings.rs:130-215)).

`ProbeError` stores `Discovery` and `Benchmark` as `String` and implements an
empty `std::error::Error` source chain
([`probe/src/error.rs:4-26`](../../../probe/src/error.rs:4-26),
[`probe/src/error.rs:47-85`](../../../probe/src/error.rs:47-85)). Once a
`CudaError` crosses this adapter, its typed fields are therefore available in
the rendered text but not as a nested `source()` value.

## Propagation into native execution

The direct CUDA backend in `native-executor` converts the type with
[`impl From<recipe_cuda::CudaError> for Error`](../../../native-executor/src/error.rs:229-233):

```text
CudaError
    -> native_executor::Error::Cuda
    -> LocalError::Native
    -> TrainingExecutionError::NativeHandoff or ExecutorError::Backend
    -> public training/inference result
```

`Error::Cuda` displays `CUDA Driver operation failed: ...`, but
`native-executor::Error` has an empty `source()` implementation
([`native-executor/src/error.rs:202-232`](../../../native-executor/src/error.rs:202-232)).
The CUDA detail remains in the display string, while a standard error-chain
walk stops at `Error`.

The conversion sites are the real resource paths, not test shims:

| Native-executor path | CUDA operations that can produce `CudaError` | Resulting behavior |
| --- | --- | --- |
| `CudaBinding::available_bytes` and `CudaResources::available_bytes` | `Context::memory_info` | `?` converts to `Error::Cuda`; free-byte representation errors remain native `ArenaMismatch` ([`cuda.rs:75-83`](../../../native-executor/src/cuda.rs:75-83), [`cuda.rs:945-958`](../../../native-executor/src/cuda.rs:945-958)). |
| `CudaResources::realize` and `realize_device` | Stream and event creation, pinned/device allocation, module load and function lookup, and all artifact/resource validation | A preparation failure prevents a resource bundle from being handed to the executor ([`cuda.rs:348-421`](../../../native-executor/src/cuda.rs:348-421), [`cuda.rs:1680-1870`](../../../native-executor/src/cuda.rs:1680-1870)). |
| `allocate_arena`, `submit_*`, `release_arena`, and `destroy_devices` | Device allocation, dimension validation, H2D/D2D/D2H copies, kernel launch, pointer lookup, and explicit frees/destroys | `?` converts to `Error::Cuda`; the executor records the backend operation that failed ([`cuda.rs:423-510`](../../../native-executor/src/cuda.rs:423-510), [`cuda.rs:654-860`](../../../native-executor/src/cuda.rs:654-860), [`cuda.rs:1335-1366`](../../../native-executor/src/cuda.rs:1335-1366), [`cuda.rs:2210-2243`](../../../native-executor/src/cuda.rs:2210-2243)). |
| `CudaResources::poll` | `Pending::poll` or `Stream::poll_idle` | A CUDA error is converted and passed to `poison`; the current poll fails and `ensure_healthy` returns `BackendPoisoned` on later operations ([`cuda.rs:554-577`](../../../native-executor/src/cuda.rs:554-577), [`cuda.rs:988-998`](../../../native-executor/src/cuda.rs:988-998)). |
| `CudaResources::submit` | Any submission helper, including copy, event record, or launch | Any error sets `poisoned = true` before returning it ([`cuda.rs:480-510`](../../../native-executor/src/cuda.rs:480-510)). |
| `ParameterBlock::enqueue` | The real `Stream::enqueue_launch` call used by calculation submission | The thin FFI helper returns `recipe_cuda::Result<()>`; `submit_calculation` propagates it into `Error::Cuda` ([`cuda_ffi.rs:48-65`](../../../native-executor/src/cuda_ffi.rs:48-65), [`cuda.rs:701-737`](../../../native-executor/src/cuda.rs:701-737)). |

The CUDA backend's `CudaPending` drop policy intentionally forgets an active
native token instead of destroying its event while the GPU may still use live
buffers ([`native-executor/src/cuda.rs:1467-1483`](../../../native-executor/src/cuda.rs:1467-1483)). This is why a failed asynchronous operation can leave a poisoned
backend and an unreclaimed live token rather than submitting replacement work.

Cross-backend staging has a separate typed wrapper. Resource creation and
source/destination copies use `?` into `StagedBridgeError::Cuda`; its display
prefix is `cross-backend CUDA staging failed: ...`, and its `source()` returns
the original `CudaError` ([`bridge.rs:44-78`](../../../native-executor/src/bridge.rs:44-78),
[`bridge.rs:137-165`](../../../native-executor/src/bridge.rs:137-165),
[`bridge.rs:305-331`](../../../native-executor/src/bridge.rs:305-331),
[`bridge.rs:1391-1410`](../../../native-executor/src/bridge.rs:1391-1410),
[`bridge.rs:1508-1581`](../../../native-executor/src/bridge.rs:1508-1581),
and [`bridge.rs:1614-1627`](../../../native-executor/src/bridge.rs:1614-1627)). A
cross-backend poll or teardown therefore travels as
`LocalError::Bridge`, while the direct CUDA backend travels as
`LocalError::Native` ([`local.rs:341-468`](../../../native-executor/src/local.rs:341-468),
[`local.rs:1930-1976`](../../../native-executor/src/local.rs:1930-1976)).

The executor's `backend_value` adapter renders any backend error into a fixed
96-byte `BackendMessage` and creates `ExecutorError::Backend` with the
operation (`BindResources`, `AllocateArena`, `PreparePending`, `Submit`,
`Poll`, `CollectExit`, `ReleaseArena`, or `DestroyResources`)
([`executor/src/error.rs:5-89`](../../../executor/src/error.rs:5-89),
[`executor/src/executor.rs:2693-2715`](../../../executor/src/executor.rs:2693-2715)).
This preserves a bounded human-readable CUDA detail and operation label but
does not preserve `CudaError` as a typed nested source. The executor still
retains its run failure, journal, and any ordered cleanup error; inference
exposes that evidence through `InferenceRunFailure`.

## Public training and inference consequences

The final user-facing wrapper depends on where the error occurs:

* During probe or preparation, `ProbeError` is wrapped by
  `NativePreparationError::Probe`, then by `TrainingError::Native` or
  `InferenceError::Native`. The displayed prefixes are `prepare current native
  system: native probe/profile validation failed: ...`
  ([`native_prepare.rs:34-89`](../../../src/native_prepare.rs:34-89),
  [`training.rs:108-141`](../../../src/training.rs:108-141),
  [`inference.rs:63-89`](../../../src/inference.rs:63-89)). The nested
  `ProbeError` has no source beyond its text.
* During the native training handoff, `LocalError` is boxed in
  `TrainingExecutionError::NativeHandoff`, whose display is `native training
  handoff failed: ...` and whose source is the boxed local error
  ([`training/src/execute.rs:687-867`](../../../training/src/execute.rs:687-867),
  [`training/src/execute.rs:2962-2965`](../../../training/src/execute.rs:2962-2965)).
  A direct CUDA backend reaches `Error::Cuda`; a staged cross-backend path can
  retain `StagedBridgeError::Cuda` and its source.
* During an already running training or inference lifecycle, executor backend
  conversion produces `ExecutorError::Backend`. Training wraps the resulting
  execution error into `TrainingError::Runtime` with stage `execute native
  training`, so the final public training error is text-only at that outer
  boundary ([`src/training.rs:1313-1337`](../../../src/training.rs:1313-1337)).
  Inference retains `InferenceExecutionError::Executor` and its run failure,
  so the displayed prefix is `execute native inference: inference execution
  failed: ...` and cleanup evidence remains queryable
  ([`training/src/execute.rs:973-1177`](../../../training/src/execute.rs:973-1177),
  [`src/inference.rs:602-659`](../../../src/inference.rs:602-659)).

The practical rule is therefore: inspect `CudaError` directly at the
`recipe-cuda` boundary, preserve the rendered detail when using probe or the
executor, and distinguish a normal `Pending` status from a `DriverCall`.
Neither the probe nor the executor is allowed to reinterpret a non-success
CUDA status as device absence, successful completion, or a retry opportunity.
