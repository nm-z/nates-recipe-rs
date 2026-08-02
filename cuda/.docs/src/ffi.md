# CUDA Driver FFI

`cuda/src/ffi.rs` is Recipe's one reviewed CUDA Driver ABI boundary. It does
not link the CUDA Runtime API, HIP, or a vendor operation library. The module
declares the C-compatible scalar, opaque-handle, UUID, status, attribute, and
function-pointer types needed by the rest of `recipe-cuda`; it resolves those
functions from a Linux `libcuda.so` handle at run time; and it records which
optional and required symbols were present. The higher-level `Driver`,
`Discovery`, `Context`, and runtime-resource types are responsible for checking
the values returned by these calls. This file does not itself invoke a CUDA
operation after loading it.

The source is [the `ffi` module](../../src/ffi.rs). `cuda/src/lib.rs` enables
`unsafe_code` only with the reason “reviewed CUDA Driver API and dlopen/dlsym
FFI boundary”, rejects non-Linux targets, and rejects non-64-bit targets. Those
compile-time constraints are part of the ABI contract: the declarations below
are for the 64-bit Linux CUDA Driver ABI, not a portable binding generator.

## Boundary and ownership

The module has two layers:

1. `Api` owns typed copies of every resolved function pointer. It is private to
   the crate and is reached by `DriverInner` in `driver.rs`.
2. `DynamicLibrary` owns the `dlopen` handle. A `DriverInner` stores both the
   `Api` and its `LibraryOwner`, so the handle remains open for as long as any
   `Driver` clone can call an API pointer. `Driver` puts that inner value in an
   `Arc`; contexts and all resource wrappers retain a `Driver` or a reference to
   a context, which keeps the handle alive through their native calls.

The raw calls are deliberately not exported as a general-purpose C binding.
The only public FFI-derived values are `DriverSymbol`,
`DriverCapabilities`, and the required/optional symbol slices, re-exported by
`cuda/src/lib.rs`. All other aliases, `Api`, `DynamicLibrary`, and the resolver
trait are crate-private.

## ABI types

Every function pointer below is `unsafe extern "C"`. The aliases make the
expected ABI explicit at the load site and keep every caller's cast-free call
site in one `Api` field. `c_int`, `c_uint`, `c_char`, `c_void`, `usize`, and
`u64` have the following roles:

| Alias | Representation | Driver meaning | Consumers |
| --- | --- | --- | --- |
| `CuResult` | `c_int` | CUDA status code, `0` means success | `Driver::check`, `Driver::check_status_only`, `query_status` |
| `CuDevice` | `c_int` | CUDA device ordinal/handle | discovery, context creation, `DeviceInfo::handle` |
| `CuContext` | `*mut c_void` | Opaque CUDA context handle | `Context` |
| `CuDevicePtr` | `u64` | 64-bit CUDA device address, represented as an integer rather than a dereferenceable Rust pointer | `DeviceBuffer`, async copy and launch calls |
| `CuModule` | `*mut c_void` | Opaque loaded module handle | `Module` |
| `CuFunction` | `*mut c_void` | Opaque module entry handle | `Function`, kernel launch |
| `CuStream` | `*mut c_void` | Opaque asynchronous stream handle | `Stream` |
| `CuEvent` | `*mut c_void` | Opaque completion-event handle | `Event`, `Pending` |

`CuContext`, `CuModule`, `CuFunction`, `CuStream`, and `CuEvent` are opaque
addresses. Rust never dereferences them, performs arithmetic on them, or
assumes a layout. A non-null result is checked before a wrapper is created.
`CuDevicePtr` is not dereferenced by Rust either. It is offset only after the
runtime wrapper has checked an allocation range and an integer overflow.

`CuUuid` is the only declared C-layout value:

```text
#[repr(C)]
struct CuUuid { bytes: [u8; 16] }
```

The driver writes the sixteen UUID bytes into this value. Discovery copies the
array into its `DeviceUuid`; no byte order or textual parsing is performed at
the FFI layer.

## Status and attribute constants

The module declares only status values that have behavior at a caller. Other
driver status numbers remain opaque and are carried through `CuResult`.

| Constant | Value | Meaning and use |
| --- | ---: | --- |
| `CUDA_SUCCESS` | `0` | A successful driver call. `Driver::check` and `check_status_only` accept it; discovery accepts it from `cuDeviceGetUuid_v2`; stream/event query maps it to `CompletionStatus::Complete`. |
| `CUDA_ERROR_NOT_READY` | `600` | A nonblocking stream or event query reports work still in flight. `runtime::query_status` maps it to `CompletionStatus::Pending`; it is not treated as a failed submission. |
| `CUDA_ERROR_NOT_SUPPORTED` | `801` | The optional `cuDeviceGetUuid_v2` exists but declined this device. Discovery then calls the required `cuDeviceGetUuid`; any other v2 status is returned as a driver failure. |

The device-attribute constants are the CUDA Driver attribute IDs used by the
exhaustive discovery pass. `discovery.rs` passes each ID to
`cuDeviceGetAttribute`, requires a non-negative result, and stores the value in
`DeviceAttributes`:

| Constant | Value | `DeviceAttributes` field |
| --- | ---: | --- |
| `CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_BLOCK` | `1` | `maximum_threads_per_block` |
| `CU_DEVICE_ATTRIBUTE_MAX_SHARED_MEMORY_PER_BLOCK` | `8` | `maximum_shared_memory_per_block_bytes` |
| `CU_DEVICE_ATTRIBUTE_WARP_SIZE` | `10` | `warp_size` |
| `CU_DEVICE_ATTRIBUTE_CLOCK_RATE` | `13` | `core_clock_khz` |
| `CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT` | `16` | `multiprocessor_count` |
| `CU_DEVICE_ATTRIBUTE_CONCURRENT_KERNELS` | `31` | `concurrent_kernels`, which accepts only `0` or `1` |
| `CU_DEVICE_ATTRIBUTE_PCI_BUS_ID` | `33` | `pci_bus_id` |
| `CU_DEVICE_ATTRIBUTE_PCI_DEVICE_ID` | `34` | `pci_device_id` |
| `CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE` | `36` | `memory_clock_khz` |
| `CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH` | `37` | `global_memory_bus_width_bits` |
| `CU_DEVICE_ATTRIBUTE_ASYNC_ENGINE_COUNT` | `40` | `async_engine_count` |
| `CU_DEVICE_ATTRIBUTE_PCI_DOMAIN_ID` | `50` | `pci_domain_id` |
| `CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR` | `75` | `ComputeCapability::major` |
| `CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR` | `76` | `ComputeCapability::minor` |

These IDs are discovery policy, not user configuration. Their values are
intrinsic to the CUDA Driver ABI and therefore remain in the FFI module.

## Function-pointer declarations

The following tables list every `unsafe extern "C"` alias in `ffi.rs`. The
pointer direction is significant: output pointers must point to writable,
adequately sized storage for the duration of the call; input strings and image
bytes must remain valid for the call.

### Initialization and device discovery

| Alias, exported symbol | ABI signature after aliases | Caller and postcondition |
| --- | --- | --- |
| `CuInit`, `cuInit` | `(c_uint) -> CuResult` | `Driver::discover` passes flags `0` before every discovery snapshot. A non-success status stops discovery. |
| `CuDriverGetVersion`, `cuDriverGetVersion` | `(*mut c_int) -> CuResult` | Discovery supplies one writable `c_int`, then `DriverVersion::from_raw` rejects a negative version. |
| `CuDeviceGetCount`, `cuDeviceGetCount` | `(*mut c_int) -> CuResult` | Discovery supplies one writable count, rejects a negative count, and uses it to size the device vector and ordinal loop. |
| `CuDeviceGet`, `cuDeviceGet` | `(*mut CuDevice, c_int) -> CuResult` | For each ordinal, discovery supplies writable handle storage. The resulting handle is retained in `DeviceInfo` and is valid for later device queries and context creation. |
| `CuDeviceGetName`, `cuDeviceGetName` | `(*mut c_char, c_int, CuDevice) -> CuResult` | Discovery passes a 256-byte zeroed buffer and its `c_int` length. It requires a NUL within the buffer, valid UTF-8, and a non-empty trimmed name. |
| `CuDeviceGetUuid`, `cuDeviceGetUuid` | `(*mut CuUuid, CuDevice) -> CuResult` | Required UUID fallback. Discovery supplies `CuUuid` storage and copies the bytes into `DeviceUuid`. |
| `CuDeviceGetUuid`, `cuDeviceGetUuid_v2` | `(*mut CuUuid, CuDevice) -> CuResult` | The v2 symbol uses the same ABI and is optional. When present, discovery tries it first; success wins, status `CUDA_ERROR_NOT_SUPPORTED` falls back to the required symbol, and any other failure aborts. |
| `CuDeviceGetPciBusId`, `cuDeviceGetPCIBusId` | `(*mut c_char, c_int, CuDevice) -> CuResult` | Discovery passes a 32-byte zeroed buffer, validates NUL termination, UTF-8, and the exact `dddd:bb:dd.f` form, then retains the string. |
| `CuDeviceTotalMemV2`, `cuDeviceTotalMem_v2` | `(*mut usize, CuDevice) -> CuResult` | Discovery supplies writable byte-count storage and converts it to `u64`, rejecting a conversion failure. |
| `CuDeviceGetAttribute`, `cuDeviceGetAttribute` | `(*mut c_int, c_int, CuDevice) -> CuResult` | Discovery supplies writable value storage and one attribute ID from the table above. Every result must fit `u32`; the Boolean concurrent-kernel attribute additionally must be `0` or `1`. |

### Context and memory-capacity queries

| Alias, exported symbol | ABI signature after aliases | Caller and postcondition |
| --- | --- | --- |
| `CuCtxCreateV2`, `cuCtxCreate_v2` | `(*mut CuContext, c_uint, CuDevice) -> CuResult` | `Context::create` passes validated context flag bits and a discovered device handle. It rejects a null success result, pops the newly current context, and destroys it if the pop fails or returns a different handle. |
| `CuCtxDestroyV2`, `cuCtxDestroy_v2` | `(CuContext) -> CuResult` | `Context::destroy`, `Context::drop`, and failed-create cleanup release an owned context. `Context` takes its raw option before calling so explicit close and drop cannot destroy twice. Drop ignores the status because it cannot return an error. |
| `CuCtxPushCurrentV2`, `cuCtxPushCurrent_v2` | `(CuContext) -> CuResult` | `Context::enter` makes one context current for a scoped operation. `runtime::with_current` uses this before every resource or submission operation. |
| `CuCtxPopCurrentV2`, `cuCtxPopCurrent_v2` | `(*mut CuContext) -> CuResult` | `Context::create` removes the driver's implicit current context; `ContextGuard::leave` removes an entered context and checks that the returned handle equals the expected context. A mismatch is a `ContextStackMismatch`. |
| `CuMemGetInfoV2`, `cuMemGetInfo_v2` | `(*mut usize, *mut usize) -> CuResult` | `Context::memory_info` enters the context, supplies free and total output storage, leaves the context, and returns both counters. `CudaBinding::available_bytes` converts free bytes to Recipe units. |

### Modules and functions

| Alias, exported symbol | ABI signature after aliases | Caller and postcondition |
| --- | --- | --- |
| `CuModuleLoadData`, `cuModuleLoadData` | `(*mut CuModule, *const c_void) -> CuResult` | `Module::load_cubin` accepts only bytes beginning with ELF magic, passes the image pointer while the slice is live, and rejects a null success handle. The wrapper owns the module until unload/drop. |
| `CuModuleUnload`, `cuModuleUnload` | `(CuModule) -> CuResult` | `Module::unload` and `Drop` release the module in its context. A `Function` borrows its `Module`, so the module cannot be unloaded while the function reference is used. |
| `CuModuleGetFunction`, `cuModuleGetFunction` | `(*mut CuFunction, CuModule, *const c_char) -> CuResult` | `Module::function` rejects an empty or interior-NUL name, passes a temporary `CString`, and rejects a null success handle. The returned `Function` stores the Rust name for diagnostics and borrows the module. |

### Allocation and asynchronous resources

| Alias, exported symbol | ABI signature after aliases | Caller and postcondition |
| --- | --- | --- |
| `CuMemAllocV2`, `cuMemAlloc_v2` | `(*mut CuDevicePtr, usize) -> CuResult` | `DeviceBuffer::allocate` rejects zero bytes, supplies pointer storage, rejects address `0` after success, and records the allocation length for later range checks. |
| `CuMemFreeV2`, `cuMemFree_v2` | `(CuDevicePtr) -> CuResult` | `DeviceBuffer::free` and `Drop` release the one owned device allocation after taking its option. Destruction runs with the allocation's context current. Drop ignores the status. |
| `CuMemHostAlloc`, `cuMemHostAlloc` | `(*mut *mut c_void, usize, c_uint) -> CuResult` | `PinnedHostBuffer::allocate` rejects zero bytes, requests flags `0`, rejects a null success pointer, zero-initializes the returned range, and exposes it only as checked Rust slices. |
| `CuMemFreeHost`, `cuMemFreeHost` | `(*mut c_void) -> CuResult` | `PinnedHostBuffer::free` and `Drop` release the pinned host allocation after taking its option. Drop ignores the status. |
| `CuStreamCreate`, `cuStreamCreate` | `(*mut CuStream, c_uint) -> CuResult` | `Stream::create_nonblocking` requests `CU_STREAM_NON_BLOCKING` (`1`, declared in `runtime.rs`), rejects a null success handle, and associates the stream with one context. |
| `CuStreamDestroyV2`, `cuStreamDestroy_v2` | `(CuStream) -> CuResult` | `Stream::destroy` and `Drop` release the stream after taking its option. Drop ignores the status. |
| `CuStreamQuery`, `cuStreamQuery` | `(CuStream) -> CuResult` | `Stream::poll_idle` enters the context and maps `CUDA_SUCCESS` to complete and `CUDA_ERROR_NOT_READY` to pending. Other statuses become `CudaError::DriverCall` without allocating error text. |
| `CuEventCreate`, `cuEventCreate` | `(*mut CuEvent, c_uint) -> CuResult` | `Event::create_completion` requests `CU_EVENT_DISABLE_TIMING` (`2`, declared in `runtime.rs`), rejects a null success handle, and stores the context association. |
| `CuEventRecord`, `cuEventRecord` | `(CuEvent, CuStream) -> CuResult` | Every event-backed copy or launch records its completion event after the enqueue on the same stream. The event is moved into `Pending`. |
| `CuEventQuery`, `cuEventQuery` | `(CuEvent) -> CuResult` | `Pending::poll` enters the event's context and maps success/not-ready as above. Once complete, the Rust token remembers completion and does not query again. |
| `CuEventDestroyV2`, `cuEventDestroy_v2` | `(CuEvent) -> CuResult` | `Event::destroy`, `Pending` completion, and `Drop` release the event. A pending token owns the event until terminal completion. |

### Copies and kernel launch

| Alias, exported symbol | ABI signature after aliases | Caller and postcondition |
| --- | --- | --- |
| `CuMemcpyHtoDAsyncV2`, `cuMemcpyHtoDAsync_v2` | `(CuDevicePtr, *const c_void, usize, CuStream) -> CuResult` | `Stream::copy_h2d` checks one same-context device destination and pinned host source, bounds-checks both ranges, enqueues the copy, then records the supplied event. |
| `CuMemcpyDtoHAsyncV2`, `cuMemcpyDtoHAsync_v2` | `(*mut c_void, CuDevicePtr, usize, CuStream) -> CuResult` | `Stream::copy_d2h` checks a mutable pinned destination and device source, records an event for `Pending`; `Stream::enqueue_d2h` intentionally omits the event and requires its caller to retain both allocations and the stream until `poll_idle` is complete. |
| `CuMemcpyDtoDAsyncV2`, `cuMemcpyDtoDAsync_v2` | `(CuDevicePtr, CuDevicePtr, usize, CuStream) -> CuResult` | `Stream::copy_d2d` checks both same-context device ranges and records an event; `Stream::enqueue_d2d` omits the event and relies on its caller's stream-idle lifetime guarantee. |
| `CuLaunchKernel`, `cuLaunchKernel` | `(CuFunction, c_uint, c_uint, c_uint, c_uint, c_uint, c_uint, c_uint, CuStream, *mut *mut c_void, *mut *mut c_void) -> CuResult` | `Stream::launch` and `enqueue_launch` pass grid x/y/z, block x/y/z, dynamic shared-memory bytes, stream, a nullable parameter-array pointer, and a null `extra` pointer. `Dim3::new` rejects zero dimensions. `launch` records an event; `enqueue_launch` relies on stream idle. |

For `CuLaunchKernel`, each element of the parameter array is a pointer to host
storage containing one argument value, not the value itself. The driver copies
those values before the call returns. The cubin ABI, pointee sizes and
alignment, launch dimensions, and the lifetime of every referenced device
allocation must therefore be established by the caller. The FFI layer cannot
infer or validate any of those facts.

### Optional mode and error text

| Alias, exported symbol | ABI signature after aliases | Caller and postcondition |
| --- | --- | --- |
| `CuModuleGetLoadingMode`, `cuModuleGetLoadingMode` | `(*mut c_int) -> CuResult` | `Driver::module_loading_mode` returns `Ok(None)` when the symbol is absent. If present, it accepts only raw values `1` (eager) and `2` (lazy), otherwise it returns `InvalidDriverValue`. |
| `CuGetErrorName`, `cuGetErrorName` | `(CuResult, *mut *const c_char) -> CuResult` | `Driver::error_text` uses it only when present, requires its own call to succeed and the output pointer to be non-null, then copies the NUL-terminated C text into a Rust `String`. |
| `CuGetErrorString`, `cuGetErrorString` | `(CuResult, *mut *const c_char) -> CuResult` | Same rules as `CuGetErrorName`, producing an optional human-readable description. Name and description are independent capabilities. |

## Symbol identity and capability model

`DriverSymbol` is the single symbolic inventory. It is `Copy`, ordered, and
non-exhaustive so it can be used as a `BTreeSet` key and extended without
promising that the enum is closed. `as_str` returns the exported symbol name for
diagnostics and `c_name` returns the same name as a compile-time NUL-terminated
`CStr` for `dlsym`. The two matches are intentionally parallel. Adding a symbol
requires updating both matches and its required/optional policy.

The required slice contains the 32 symbols needed to construct contexts,
discover devices, load modules, allocate memory, create asynchronous
resources, enqueue copies, and launch Recipe kernels:

```text
Init, DriverGetVersion, DeviceGetCount, DeviceGet, DeviceGetName,
DeviceGetUuid, DeviceGetPciBusId, DeviceTotalMemV2, DeviceGetAttribute,
CtxCreateV2, CtxDestroyV2, CtxPushCurrentV2, CtxPopCurrentV2,
ModuleLoadData, ModuleUnload, ModuleGetFunction,
MemAllocV2, MemFreeV2, MemGetInfoV2, MemHostAlloc, MemFreeHost,
StreamCreate, StreamDestroyV2, StreamQuery,
EventCreate, EventRecord, EventQuery, EventDestroyV2,
MemcpyHtoDAsyncV2, MemcpyDtoHAsyncV2, MemcpyDtoDAsyncV2, LaunchKernel
```

The optional slice contains four symbols whose absence has a defined behavior:

```text
DeviceGetUuidV2, ModuleGetLoadingMode, GetErrorName, GetErrorString
```

`Api::load` begins with the required slice's explicit sequence. It stops at the
first missing required symbol and returns `CudaError::MissingRequiredSymbol`.
It then probes each optional symbol. Optional lookup errors, including a
`dlsym` error string, are treated as absence rather than preventing the driver
from loading. The resulting function options and a capability set are built
together, so `DriverCapabilities` describes the exact `Api` value rather than
an independently guessed version.

`DriverCapabilities` stores a `BTreeSet<DriverSymbol>`:

* `supports(symbol)` is an exact membership query.
* `available_symbols()` yields the sorted set and reports its exact size.
* `from_available` is crate-private and is called only while constructing an
  `Api`.

The set initially contains every required symbol, then contains only optional
symbols whose function pointer is `Some`. `Discovery` copies this value into
its public snapshot. `DeploymentIdentity` copies it again for a specific
device. Artifact compatibility compares the required-symbol set recorded in a
native artifact and reports `MissingDriverSymbol` for every symbol absent from
the reopened deployment. Preparation also builds the required set from
`REQUIRED_DRIVER_SYMBOLS` and rejects a measured deployment that omits one.

## Resolution and dynamic loading

### `SymbolResolver` and `Api::load`

`SymbolResolver` has one unsafe operation:

```text
unsafe fn lookup(
    &self,
    symbol: DriverSymbol,
) -> Result<Option<NonNull<c_void>>, String>
```

The resolver promises that `Some(pointer)` is a non-null address for the
function named by the `DriverSymbol`, using the exact ABI declared by the
corresponding alias. `None` means that no symbol was found and no loader error
was reported. `Err(detail)` means the lookup operation itself reported a
loader-specific failure.

`required<T>` performs one unsafe lookup. It converts `Some` with
`pointer_as_function`, maps `None` to a missing-symbol error without detail, and
maps `Err(detail)` to the same error with the loader detail. `optional<T>`
performs the same lookup and conversion but collapses both `None` and `Err` to
`None`.

`pointer_as_function` is the narrowest and most important representation
assumption in the module:

1. `T` must be one of the declared function-pointer aliases and therefore be
   `Copy`.
2. The function pointer and `*mut c_void` must have equal size. The function
   asserts this before `mem::transmute_copy`.
3. The pointer must have come from the exact symbol lookup for `T`; the size
   check cannot prove an ABI match.

The target is restricted to the reviewed 64-bit Linux ABI so this
function/data-pointer representation assumption is explicit. A changed
platform or a generated binding with a different calling convention must not
reuse this conversion without a new review.

### `DynamicLibrary`

`DynamicLibrary::open(path)` converts the Rust path with `CString::new`. An
interior NUL fails before entering libc with
`CudaError::InvalidLibraryPath`. Otherwise it clears any stale `dlerror`, calls
`dlopen(path, RTLD_NOW | RTLD_LOCAL)`, and stores a non-null handle and the
original path. A null handle is reported as `CudaError::LibraryOpen` with the
current `dlerror` text, or the literal `unknown dlopen error` if libc supplied
none.

`open_default` tries these names in order:

```text
libcuda.so.1
libcuda.so
```

It accumulates the attempt text from each `LibraryOpen` error and returns one
combined `LibraryOpen` only after both fail. A different error is returned
immediately. `Driver::load` uses this default search; the native probe and
reopening path use `Driver::load_from_path`, which calls `open` for the selected
identity-pinned library.

`DynamicLibrary::lookup` clears `dlerror`, calls `dlsym` with the `CStr` from
`DriverSymbol::c_name`, reads `dlerror` immediately, and returns an error if it
was set. If no loader error occurred, a null result becomes `Ok(None)` and a
non-null result becomes `Ok(Some(NonNull<c_void>))`. `take_dlerror` copies the
libc-owned NUL-terminated text to a Rust `String` before another dynamic-loader
operation can invalidate it.

`Drop` calls `dlclose` and intentionally ignores its status. No API call is made
from `Drop`; the `Api` contains plain function-pointer values. The unsafe
`Send` and `Sync` implementations assert that the immutable loader handle and
its resolved function addresses can be shared through `Arc<DriverInner>`.
They do not make CUDA context state global or thread-safe. Context currentness
remains an explicit per-call invariant enforced by `Context::enter` and
`runtime::with_current`.

## Direct callers inside `recipe-cuda`

The FFI module has no independent execution loop. These modules are the direct
consumers of its aliases, constants, and `Api` fields.

### `driver.rs`

`Driver::load` calls `DynamicLibrary::open_default`; `load_from_path` calls
`open`; and `from_library` calls `load_api`. The resulting `DriverInner` keeps
the `Api` and `DynamicLibrary` together. `Driver::capabilities` and
`Driver::supports` expose the capability snapshot, while `loaded_library`
exposes only the retained path string.

`module_loading_mode` reads the optional `Api::module_get_loading_mode` and
maps raw values `1` and `2`. `check` is used during discovery, setup,
allocation, and destruction, and on failure calls `error_detail`, which probes
the optional name and description functions. `check_status_only` is used on
post-realization submit and poll paths so a live-loop failure preserves the
exact numeric `CuResult` without allocating optional driver text. Both helpers
turn nonzero statuses into `CudaError::DriverCall` with a `DriverStatus`.

### `discovery.rs`

`Driver::discover` calls the initialization, version, count, and per-device
function pointers listed in the discovery table. It validates every output
before constructing `Discovery`, detects duplicate UUIDs, and copies
`Api::capabilities` into the snapshot. `device_uuid` exercises the optional v2
fallback described above. The attribute constants are consumed only here.

### `context.rs`

`Context::create`, `Context::enter`, `ContextGuard::leave`, `Context::memory_info`,
and context destruction call the context and memory-info fields. Creation
validates flag bits before entering the ABI, confirms that the driver popped the
same context it created, and cleans up on all failed post-create checks.
`Context` is intentionally not `Send` or `Sync` (`PhantomData<Rc<()>>`). A
`ContextGuard` is a scoped current-context proof; its drop path attempts a pop
if the caller did not explicitly leave.

### `runtime.rs`

The runtime wrappers are the complete operation surface over `Api`:

* `Module` calls module load, function lookup, and unload.
* `DeviceBuffer` calls device allocation and free, and bounds-checks every
  offset and byte count before creating a `CuDevicePtr` for a copy.
* `PinnedHostBuffer` calls pinned allocation and free, zeroes the allocation,
  and exposes checked slices.
* `Stream` calls stream creation, query, destruction, the three async copy
  functions, kernel launch, and event recording.
* `Event` calls event creation and destruction.
* `Pending` calls event query and keeps the completion event plus an operation
  lifetime marker until it reaches a terminal state.

Every operation that uses an owned context calls `with_current`, which pushes
the context, performs the operation, and pops it before returning. A failed
operation still attempts the pop, returning the original operation error. The
same-context checks reject resources from different context objects before any
raw call. Event-backed methods return `Pending`; while that token is retained,
its phantom operation borrow keeps the referenced Rust resources live. The
caller contract requires driving the token to terminal completion before those
resources or the event are recycled.

`native-executor/src/cuda_ffi.rs` is the narrow adapter for kernel argument
storage. Its `ParameterBlock` owns aligned `u64` values and pointers into that
storage, retains device-buffer references, and calls `Stream::enqueue_launch`.
The adapter's unsafe lifetime reconstruction is valid only while its arena
buffers remain retained until the executor observes stream idle.

## Callers outside `recipe-cuda`

The following callers reach the ABI indirectly through the public wrappers or
through the capability value. None accesses `Api` or `DynamicLibrary` directly.

| Caller | FFI-derived path |
| --- | --- |
| `native-probe/src/cuda.rs` | Opens a configured library through `Driver::load_from_path`, discovers it, creates a context and nonblocking stream, allocates pinned/device buffers, benchmarks H2D, D2H, D2D, and a Recipe-owned cubin launch, polls `Pending`, and verifies copied/calculated bytes. This is the first real hardware path through every required allocation, stream, event, copy, and launch declaration. |
| `native-probe/src/bindings.rs` | Reopens measured CUDA devices, builds `DeploymentIdentity::from_discovery`, and creates one context per matched device. |
| `native-executor/src/cuda.rs` | Uses `Context::memory_info` for admission, allocates arenas, staging and metric buffers, creates streams/events, loads realized cubins, looks up entry functions, and submits init transfers, calculation launches, internal D2D transfers, four-byte metric D2H transfers, and exit D2H transfers. It polls either event-backed `Pending` tokens or stream idle and retains all resources until terminal completion. |
| `native-executor/src/bridge.rs` | Creates CUDA staging buffers, streams, and completion events for staged host/device legs, then uses event-backed H2D and D2H operations and recycles only terminal events. |
| `cuda/src/artifact.rs` | Uses `DriverSymbol` and `DriverCapabilities` to bind artifact identity to the exact required-symbol set and report missing symbols during compatibility validation. It does not make a driver call. |
| `src/native_prepare.rs` | Collects `REQUIRED_DRIVER_SYMBOLS` into the CUDA runtime policy and rejects a deployment whose copied discovery capabilities do not support one. |
| `prepare/src/production.rs` | Stores `DriverSymbol` in `CudaArtifactPolicy::required_driver_symbols`, serializing the CUDA artifact's required ABI identity into preparation policy. |
| `cuda/src/lib.rs` and `src/facade.rs` | Re-export the capability and symbol inventory, and expose the rest of `recipe-cuda` under the root facade's `cuda` module. |

The separate `native-executor/src/cuda_ffi.rs` name is not this module. It
contains only launch-parameter storage and calls the safe-resource wrapper;
it does not resolve symbols or declare a second CUDA Driver ABI.

## Safety invariants at the FFI edge

The following facts are required before an `unsafe extern "C"` call. The
wrappers establish most of them; the documented unsafe methods expose the
remaining caller obligations.

### Pointer and representation invariants

* Every output pointer points to writable storage of the exact C representation
  and remains valid for the complete call. This applies to counts, handles,
  attributes, memory sizes, buffers, UUIDs, and error-text pointers.
* Every C string passed into the driver is NUL-terminated and contains no
  interior NUL. `CString::new` enforces this for library paths and kernel names;
  compile-time `c"..."` values enforce it for `dlsym` names.
* Every returned opaque handle is checked for null before ownership is recorded.
  A success status with a null context, module, function, stream, event, host
  pointer, or device pointer is an `InvalidDriverValue` error.
* Device-pointer arithmetic is integer checked against the allocation length
  and against `u64` overflow. Host-pointer arithmetic is performed only after
  the range check on the pinned allocation.
* `CuUuid` is written only into its `#[repr(C)]` sixteen-byte storage. Its bytes
  are copied before the local value goes out of scope.
* A `dlsym` result is converted only from `NonNull<c_void>`. The conversion to a
  typed function pointer is guarded by the equal-size assertion and the
  platform's reviewed ABI assumption.

### Library and symbol lifetime

* The `DynamicLibrary` handle outlives every `Api` call. `DriverInner` owns the
  handle next to the function pointers, and `Arc` clones keep that inner value
  alive while contexts, resources, or executor bindings use it.
* `SymbolResolver::lookup` must not return a pointer from a handle that will be
  closed before the resulting `Api` is dropped. A future resolver must preserve
  this same ownership relationship.
* `dlerror` text is copied immediately. No intervening loader operation may be
  inserted between `dlerror` and `CStr::from_ptr` in `take_dlerror`.
* `dlclose` failure is observable only to the dynamic loader. `Drop` cannot
  return an error, so it deliberately ignores the result after all Rust-owned
  API operations have ended.

### Context and resource lifetime

* CUDA operations are context-current operations. `with_current` pushes the
  wrapper's context and pops it on every path. `ContextGuard` checks that the
  driver returned the same handle it expected.
* Every resource wrapper carries the exact `Context` reference that created it.
  `same_context` compares those wrapper identities before a copy, event record,
  or launch. Cross-context resources fail with `ResourceContextMismatch` before
  the driver sees a pointer.
* Modules outlive their `Function` borrows. Device and pinned-host allocations
  outlive any pending operation that references their ranges. Streams and
  events outlive their queued work.
* Explicit `free`, `unload`, `destroy`, and `close` methods take the raw option
  before calling the driver, making repeated destruction a `ContextClosed`
  error rather than a second raw call. `Drop` follows the same take-first rule
  but ignores driver status.
* Event-backed `copy_*` and `launch` methods return a `Pending` whose phantom
  operation borrow prevents the referenced Rust values from ending early. The
  caller must poll or wait to terminal completion. `Pending::wait` returns a
  `TimedOut(Pending)` value, not a dropped operation, because CUDA Driver API
  has no cancellation primitive.
* Event-less `enqueue_*` methods are unsafe specifically because the wrapper
  cannot return a borrow-owning token. Their caller must retain allocations,
  functions, modules, and the stream until `poll_idle` reports complete.

### Launch and asynchronous-copy contracts

* H2D sources and D2H destinations are pinned host allocations created by the
  wrapper. A caller must not substitute an ordinary host slice for the raw
  pointer expected by the asynchronous Driver API.
* Copy offsets and byte counts are checked independently against both source
  and destination allocation lengths. A zero-byte allocation is rejected at
  allocation time; a zero-byte range is not given special success behavior.
* Kernel `parameters` must be mutable storage of pointers to correctly aligned
  argument values. The pointers and pointees must match the realized cubin
  entry ABI exactly. `keepalive` must list every device allocation referenced by
  those argument values. The driver copies argument values before returning,
  but device memory remains in use until the queued kernel completes.
* `LaunchConfig` carries nonzero grid and block dimensions and a caller-supplied
  dynamic shared-memory byte count. The FFI declaration does not validate
  occupancy, register use, cubin target, or argument semantics; artifact
  inspection and preparation establish those facts before the call.
* Event recording follows a successful enqueue. If the enqueue succeeds but
  event recording fails, the method returns the recording error while the
  caller's ownership and cleanup path still retain the stream and resources.

### Status and output validation

`Driver::check` is the rich path used before realization or outside the live
loop. It converts a nonzero `CuResult` to `DriverCallError` and asks the
optional error-name/string functions for best-effort text. A missing optional
text function, a failed text lookup, or a null text pointer leaves that field
`None`; it never hides the original numeric status.

`Driver::check_status_only` is the allocation-free path used after resources
are realized, including submissions, event records, and polling. It preserves
the numeric status with `name: None` and `description: None`. `query_status`
handles the one non-error asynchronous status, `CUDA_ERROR_NOT_READY`, before
delegating all other values to `check_status_only`.

Output values are validated at the first typed boundary:

* negative version, device count, or attribute values become
  `InvalidDriverValue`;
* Boolean concurrent-kernel values other than `0` and `1` become
  `InvalidDriverValue`;
* unterminated or invalid UTF-8 device name and PCI strings become their
  dedicated validation errors;
* duplicate device UUIDs become `DuplicateDeviceUuid`;
* unknown module loading modes become `InvalidDriverValue`;
* null success handles become `InvalidDriverValue`;
* a missing required symbol or loader detail becomes
  `MissingRequiredSymbol`;
* library-open and path-conversion failures remain `LibraryOpen` or
  `InvalidLibraryPath`.

No fallback substitutes a required symbol, invents a status, or converts an
invalid output into a default. The only ABI fallback is the documented
`cuDeviceGetUuid_v2` to `cuDeviceGetUuid` path for the driver's explicit
`CUDA_ERROR_NOT_SUPPORTED` result. Optional diagnostic text is best effort by
design, while the operation's status is always retained.

## End-to-end flows

### Load, resolve, and discover

```text
Driver::load or Driver::load_from_path
    -> DynamicLibrary::open(_default)
    -> DynamicLibrary::lookup for each DriverSymbol
    -> required/optional conversion into Api
    -> Driver::discover
    -> cuInit, version, count, and per-device queries
    -> validated Discovery plus DriverCapabilities
```

Failure before `Api` construction is a library-open, path, lookup, or missing
required-symbol error. Failure after construction is a driver status or an
invalid driver-output error. Optional symbols never change whether the base
driver loads, but they do change the capability snapshot and the behavior of
UUID fallback, loading-mode reporting, and error detail.

### Prepare and execute

The native probe first follows the load/discover path, then uses the returned
device attributes to build and benchmark a real cubin. Later native binding
reopens the identity-pinned library, repeats discovery, creates contexts, and
derives `DeploymentIdentity` with the exact capability set. Preparation records
the required symbol set in the CUDA artifact policy. The native executor then
loads the already validated cubin, obtains its function, realizes allocations,
and submits only through the context-current runtime wrappers.

This division keeps symbol resolution and discovery in pre-loop preparation.
The finalized execution loop uses already-resolved function pointers, bounded
allocation-free status conversion, streams, events, and immutable artifacts.

### Teardown

Resource drops release events, streams, allocations, modules, contexts, and
finally the dynamic library through their ownership chain. Explicit teardown
can report a driver status; drop paths attempt the same native release but
cannot return an error. A pending operation must reach terminal completion
before any of its borrowed resources can be dropped, which is why the executor
polls every event-backed token and waits for every event-less stream submission
before phase teardown.

## Maintenance rules for this boundary

When the CUDA Driver surface changes, update one coherent inventory:

1. add or change the ABI alias and its exact parameter contract;
2. add the `DriverSymbol` variant and both `as_str` and `c_name` arms;
3. classify it in exactly one of `REQUIRED_DRIVER_SYMBOLS` or
   `OPTIONAL_DRIVER_SYMBOLS`;
4. add the corresponding `Api` field and load step;
5. update the capability and artifact-identity policy if the symbol affects
   deployment compatibility;
6. document every direct caller and its pointer, context, and lifetime proof;
7. preserve the Linux 64-bit constraint unless the representation and loader
   review is redone.

Do not add a second dynamic loader, a static fallback declaration, a duplicate
symbol list, or an alternate implementation. The lists, enum mapping, `Api`,
and `DynamicLibrary` are the single source of truth for this FFI boundary.
