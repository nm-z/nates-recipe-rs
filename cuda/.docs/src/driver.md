# `cuda/src/driver.rs`: dynamic CUDA Driver owner

```toml
[module]
path = "cuda/src/driver.rs"
kind = "dynamic-cuda-driver-owner"
purpose = "Own one resolved CUDA Driver API table and the dynamic library that supplies it."
public_types = ["Driver", "ModuleLoadingMode"]
private_types = ["DriverInner", "LibraryOwner"]
loader_module = "cuda/src/ffi.rs"
error_module = "cuda/src/error.rs"
platform = "Linux, 64-bit CUDA Driver ABI"

[contract]
construction = "open one Driver library, resolve all required symbols, record optional symbols"
default_library_order = ["libcuda.so.1", "libcuda.so"]
required_symbol_policy = "missing required symbol rejects construction"
optional_symbol_policy = "missing optional symbol becomes an absent capability"
library_lifetime = "DynamicLibrary remains in the shared Arc until the last Driver clone and all Context clones are dropped"
status_success = 0
rich_error_text = "best effort through cuGetErrorName and cuGetErrorString"
live_loop_error_text = "numeric status only"
module_loading_mode_values = "1=Eager, 2=Lazy, other values are invalid"
retry_policy = "none"
fallback_policy = "only default soname open failure is aggregated"
```

This module is the owner boundary for the CUDA Driver API. It does not define
C signatures or dynamic-loader calls itself. Those details are in
[`ffi.rs`](../../src/ffi.rs), which supplies `Api`, `DynamicLibrary`, typed
function pointers, `DriverSymbol`, and `DriverCapabilities`. The module wraps
that table in a cloneable `Driver`, exposes the selected library name and
resolved capabilities, converts Driver status values into crate errors, and
provides the optional module-loading-mode query. Discovery, context, and
runtime modules use the crate-private `Api` fields through a `Driver` reference;
they do not load a second API table.

The crate root re-exports `Driver` and `ModuleLoadingMode` from this module
([`lib.rs`](../../src/lib.rs#L16-L37)). It also re-exports the capability and
symbol types owned by `ffi.rs`, so an advanced caller can inspect the resolved
surface without receiving a raw function pointer or a raw `dlopen` handle.

## Ownership and representation

The complete storage graph is:

```text
Driver (public, Clone)
└── inner: Arc<DriverInner>
    ├── api: Api                 (crate-private typed function pointers)
    └── library: LibraryOwner
        └── Dynamic(DynamicLibrary)
            ├── handle: NonNull<c_void>
            └── name: String
```

[`Driver`](../../src/driver.rs#L12-L24) contains only the `Arc`. `DriverInner`
keeps the `Api` and the `LibraryOwner` in the same allocation. The function
pointers therefore cannot outlive the `DynamicLibrary` that supplied them:
`DynamicLibrary` is moved into `DriverInner` only after `load_api()` has
resolved the table, and the library is dropped only after the final
`Arc<DriverInner>` owner goes away. There is no public constructor for
`DriverInner`, no public access to `Api`, and no public method that replaces a
library or symbol after construction.

`Driver` derives `Clone`, so cloning increments the `Arc` count and shares one
immutable API table. It has no custom `Drop` implementation. The final shared
owner drops `LibraryOwner::Dynamic`, whose `DynamicLibrary::drop` calls
`dlclose`; that destructor ignores the `dlclose` return value
([`ffi.rs`](../../src/ffi.rs#L445-L515)). A failed `Api::load` never constructs a
`Driver`, so the temporary library is dropped on the error path.

`Driver` implements `Debug` manually. The output is a debug struct containing
`loaded_library` and `capabilities`; it does not print function addresses or
the opaque handle ([`driver.rs`](../../src/driver.rs#L135-L142)).

## Public API contract

The following table is the exact public surface implemented in
`driver.rs`. `Result<T>` means `core::result::Result<T, CudaError>` from
[`error.rs`](../../src/error.rs#L1-L4).

| Method or type | Inputs | Success output | Failure or ownership rule |
| --- | --- | --- | --- |
| `Driver::load()` | No arguments. | One `Driver` using the first default soname that opens. | Returns `LibraryOpen` when both sonames fail, or `MissingRequiredSymbol` when the opened library cannot provide a required symbol. |
| `Driver::load_from_path(path: &str)` | One Rust string. The bytes must not contain an interior NUL because the string is converted to `CString`. | One `Driver` whose `loaded_library()` is the exact supplied string. | `InvalidLibraryPath` for an interior NUL; `LibraryOpen` for `dlopen` failure; `MissingRequiredSymbol` for an API-resolution failure. The path is not canonicalized by this module. |
| `Driver::loaded_library()` | Shared borrow of an existing `Driver`. | `&str` borrowing the stored `name`. | Infallible and non-mutating. It reports the requested path or soname, not a canonical filesystem path. |
| `Driver::capabilities()` | Shared borrow. | `&DriverCapabilities` borrowing the immutable capability set in `Api`. | Infallible. It does not perform a new `dlsym`. |
| `Driver::supports(symbol)` | One `DriverSymbol`. | `true` exactly when the symbol is in the set recorded during `Api::load`. | Infallible. It does not probe the library again. |
| `Driver::module_loading_mode()` | Shared borrow and no explicit argument. | `Ok(Some(Eager))` for raw mode `1`, `Ok(Some(Lazy))` for raw mode `2`, or `Ok(None)` when the optional symbol is absent. | A Driver status becomes `DriverCall`; a successful unknown raw mode becomes `InvalidDriverValue`. |
| `ModuleLoadingMode` | `Eager` or `Lazy`; derives `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq`. | Describes the raw mode returned by `cuModuleGetLoadingMode`. | It is not a setting API. This module only queries the driver. |

The crate-private methods are part of the caller contract even though they are
not exported:

| Method | Caller input | Behavior |
| --- | --- | --- |
| `Driver::from_library(library)` | An already-open `DynamicLibrary`. | Resolves `Api` from that handle, then stores both API and library in one `Arc`; any `Api::load` error is returned before a `Driver` exists. |
| `Driver::check(operation, status)` | A static operation label and one `CuResult`. | Returns `Ok(())` only for `CUDA_SUCCESS`; otherwise builds `CudaError::DriverCall` with numeric status and best-effort name and description text. |
| `Driver::check_status_only(operation, status)` | A static operation label and one `CuResult`. | Same success rule and error variant as `check`, but never asks optional error-text symbols and never allocates rich text. |
| `Driver::error_detail(status)` | One nonzero Driver status. | Independently requests the optional name and description; each result is `None` when its symbol is absent, its call fails, or it returns a null pointer. |
| `Driver::error_text(status, function)` | One status and one optional typed error-text function. | Converts the returned NUL-terminated C string with `CStr::to_string_lossy().into_owned()`. It does not retain the Driver-owned C pointer. |

`operation` is `&'static str`, so every error created by these methods carries
the fixed operation spelling selected by the caller. The Driver does not infer
an operation name from a function pointer and does not retry a failed call.

## Construction flow

The two public constructors share one path after opening the library:

```text
Driver::load
  -> DynamicLibrary::open_default
       -> DynamicLibrary::open("libcuda.so.1")
       -> DynamicLibrary::open("libcuda.so") if the first open failed
  -> Driver::from_library
       -> DynamicLibrary::load_api
            -> Api::load
                 -> resolve all required symbols
                 -> resolve all optional symbols
                 -> build DriverCapabilities
  -> Arc<DriverInner> { api, library }

Driver::load_from_path(path)
  -> DynamicLibrary::open(path)
  -> the same Driver::from_library path
```

`Driver::load` and `Driver::load_from_path` are the only construction entry
points. `from_library` is private, so callers cannot supply an alternate
resolver or a borrowed library. The only production `SymbolResolver`
implementation is `DynamicLibrary` in `ffi.rs`; the trait is private and does
not create a second public FFI path.

### Default soname selection

`DynamicLibrary::open_default()` attempts the exact ordered list
`["libcuda.so.1", "libcuda.so"]` ([`ffi.rs`](../../src/ffi.rs#L453-L466)).
For each candidate it calls `open` once. A successful first open returns
immediately and the second name is not consulted. A `CudaError::LibraryOpen`
from a failed candidate contributes its `attempts` entries to one aggregate
vector. Any other error is returned immediately rather than being hidden by a
fallback attempt. If both opens fail, the final `LibraryOpen` preserves both
candidate diagnostics in order.

The fallback applies only to `dlopen` failure. If one soname opens but its API
is missing a required symbol, `from_library` returns `MissingRequiredSymbol`;
it does not reopen the other soname.

### Explicit path opening

`DynamicLibrary::open(path)` first calls `CString::new(path)`. An interior NUL
is rejected as `CudaError::InvalidLibraryPath { path: path.to_owned() }` before
any loader call. Otherwise it clears the thread-local `dlerror`, calls
`dlopen(path, RTLD_NOW | RTLD_LOCAL)`, and wraps a non-null handle in
`NonNull<c_void>` ([`ffi.rs`](../../src/ffi.rs#L468-L493)).

On a null handle it reads one `dlerror` string. If no text is available, it
uses the literal detail `unknown dlopen error`. The result is
`CudaError::LibraryOpen { attempts: vec![format!("{path}: {detail}")] }`.
On success it stores the exact input path in `DynamicLibrary::name`; this is
the string later returned by `Driver::loaded_library()`.

No path existence check, regular-file check, canonicalization, digest, or
permission policy occurs here. The native-probe layer performs its configured
candidate selection and identity hashing before it passes a path into
`Driver::load_from_path` ([`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L86-L106)).

## Symbol resolution and capability recording

### Symbol inventory

`DriverSymbol` is `#[non_exhaustive]` and derives ordering and hashing. Its
`as_str()` result is used in diagnostics and its private `c_name()` result is
the NUL-terminated name passed to `dlsym` ([`ffi.rs`](../../src/ffi.rs#L91-L214)).
The current inventory contains 32 required and 4 optional symbols:

```toml
required_symbols = [
  "cuInit", "cuDriverGetVersion", "cuDeviceGetCount", "cuDeviceGet",
  "cuDeviceGetName", "cuDeviceGetUuid", "cuDeviceGetPCIBusId",
  "cuDeviceTotalMem_v2", "cuDeviceGetAttribute",
  "cuCtxCreate_v2", "cuCtxDestroy_v2", "cuCtxPushCurrent_v2",
  "cuCtxPopCurrent_v2", "cuModuleLoadData", "cuModuleUnload",
  "cuModuleGetFunction", "cuMemAlloc_v2", "cuMemFree_v2",
  "cuMemGetInfo_v2", "cuMemHostAlloc", "cuMemFreeHost",
  "cuStreamCreate", "cuStreamDestroy_v2", "cuStreamQuery",
  "cuEventCreate", "cuEventRecord", "cuEventQuery", "cuEventDestroy_v2",
  "cuMemcpyHtoDAsync_v2", "cuMemcpyDtoHAsync_v2", "cuMemcpyDtoDAsync_v2",
  "cuLaunchKernel",
]

optional_symbols = [
  "cuDeviceGetUuid_v2", "cuModuleGetLoadingMode",
  "cuGetErrorName", "cuGetErrorString",
]
```

The required list is the exact `REQUIRED_DRIVER_SYMBOLS` slice. The optional
list is the exact `OPTIONAL_DRIVER_SYMBOLS` slice. Versioned spellings such as
`cuCtxCreate_v2` and `cuMemcpyDtoHAsync_v2` are intentional. The module does
not resolve a symbol by prefix, alias, version range, or a CUDA Runtime API
name. The inventory has no `cuStreamSynchronize`, synchronous-copy,
cancellation, or HIP symbol; runtime completion uses stream and event queries
that are present in the list.

### Resolved field map

`Api` stores one typed field for each entry. The map below names the field,
the FFI alias, the C symbol, and the production caller. The aliases and
pointer signatures are defined in [`ffi.rs`](../../src/ffi.rs#L38-L89); the
driver module never casts a value at a call site after `Api::load` has built the
typed table.

| `Api` field | FFI alias | C symbol | Input and output shape | Direct caller |
| --- | --- | --- | --- | --- |
| `init` | `CuInit` | `cuInit` | `u32` flags in, `CuResult` out | `Discovery::discover` passes `0`. |
| `driver_get_version` | `CuDriverGetVersion` | `cuDriverGetVersion` | `*mut c_int` raw version out, `CuResult` out | `Discovery::discover`. |
| `device_get_count` | `CuDeviceGetCount` | `cuDeviceGetCount` | `*mut c_int` count out, `CuResult` out | `Discovery::discover`. |
| `device_get` | `CuDeviceGet` | `cuDeviceGet` | `*mut CuDevice` out and `c_int` ordinal in, `CuResult` out | `Discovery::discover_device`. |
| `device_get_name` | `CuDeviceGetName` | `cuDeviceGetName` | `*mut c_char` buffer, `c_int` capacity, `CuDevice` in, `CuResult` out | `Discovery::device_name`. |
| `device_get_uuid` | `CuDeviceGetUuid` | `cuDeviceGetUuid` | `*mut CuUuid` out and `CuDevice` in, `CuResult` out | `Discovery::device_uuid` fallback. |
| `device_get_uuid_v2` | `Option<CuDeviceGetUuid>` | `cuDeviceGetUuid_v2` | Same shape as `device_get_uuid`; optional | `Discovery::device_uuid` preferred path. |
| `device_get_pci_bus_id` | `CuDeviceGetPciBusId` | `cuDeviceGetPCIBusId` | `*mut c_char` buffer, `c_int` capacity, `CuDevice` in, `CuResult` out | `Discovery::device_pci_bus_id`. |
| `device_total_mem_v2` | `CuDeviceTotalMemV2` | `cuDeviceTotalMem_v2` | `*mut usize` bytes out and `CuDevice` in, `CuResult` out | `Discovery::device_memory`. |
| `device_get_attribute` | `CuDeviceGetAttribute` | `cuDeviceGetAttribute` | `*mut c_int` value out, `c_int` attribute and `CuDevice` in, `CuResult` out | `Discovery::device_attribute`. |
| `ctx_create_v2` | `CuCtxCreateV2` | `cuCtxCreate_v2` | `*mut CuContext` out, `u32` flags and `CuDevice` in, `CuResult` out | `Context::create`. |
| `ctx_destroy_v2` | `CuCtxDestroyV2` | `cuCtxDestroy_v2` | `CuContext` in, `CuResult` out | `Context::destroy`, `Context::drop`, and create-failure cleanup. |
| `ctx_push_current_v2` | `CuCtxPushCurrentV2` | `cuCtxPushCurrent_v2` | `CuContext` in, `CuResult` out | `Context::enter`. |
| `ctx_pop_current_v2` | `CuCtxPopCurrentV2` | `cuCtxPopCurrent_v2` | `*mut CuContext` popped handle out, `CuResult` out | `Context::create` verification and `ContextGuard::pop`. |
| `module_load_data` | `CuModuleLoadData` | `cuModuleLoadData` | `*mut CuModule` out and `*const c_void` cubin image in, `CuResult` out | `Module::load_cubin`. |
| `module_unload` | `CuModuleUnload` | `cuModuleUnload` | `CuModule` in, `CuResult` out | `Module::destroy` and `Module::drop`. |
| `module_get_function` | `CuModuleGetFunction` | `cuModuleGetFunction` | `*mut CuFunction` out, `CuModule` and `*const c_char` name in, `CuResult` out | `Module::function`. |
| `mem_alloc_v2` | `CuMemAllocV2` | `cuMemAlloc_v2` | `*mut CuDevicePtr` out and `usize` bytes in, `CuResult` out | `DeviceBuffer::allocate`. |
| `mem_free_v2` | `CuMemFreeV2` | `cuMemFree_v2` | `CuDevicePtr` in, `CuResult` out | `DeviceBuffer::destroy` and `DeviceBuffer::drop`. |
| `mem_get_info_v2` | `CuMemGetInfoV2` | `cuMemGetInfo_v2` | `*mut usize` free and total outputs, `CuResult` out | `Context::memory_info`. |
| `mem_host_alloc` | `CuMemHostAlloc` | `cuMemHostAlloc` | `*mut *mut c_void` out, `usize` bytes and `u32` flags in, `CuResult` out | `PinnedHostBuffer::allocate` passes flags `0`. |
| `mem_free_host` | `CuMemFreeHost` | `cuMemFreeHost` | `*mut c_void` in, `CuResult` out | `PinnedHostBuffer::destroy` and `PinnedHostBuffer::drop`. |
| `stream_create` | `CuStreamCreate` | `cuStreamCreate` | `*mut CuStream` out and `u32` flags in, `CuResult` out | `Stream::create_nonblocking` passes flag `1`. |
| `stream_destroy_v2` | `CuStreamDestroyV2` | `cuStreamDestroy_v2` | `CuStream` in, `CuResult` out | `Stream::close` and `Stream::drop`. |
| `stream_query` | `CuStreamQuery` | `cuStreamQuery` | `CuStream` in, `CuResult` out | `Stream::poll_idle` through `query_status`. |
| `event_create` | `CuEventCreate` | `cuEventCreate` | `*mut CuEvent` out and `u32` flags in, `CuResult` out | `Event::create_completion` passes flag `2`. |
| `event_record` | `CuEventRecord` | `cuEventRecord` | `CuEvent` and `CuStream` in, `CuResult` out | Event-backed copy and launch methods. |
| `event_query` | `CuEventQuery` | `cuEventQuery` | `CuEvent` in, `CuResult` out | `Pending::poll` through `query_status`. |
| `event_destroy_v2` | `CuEventDestroyV2` | `cuEventDestroy_v2` | `CuEvent` in, `CuResult` out | `Event::close` and `Event::drop`. |
| `memcpy_htod_async_v2` | `CuMemcpyHtoDAsyncV2` | `cuMemcpyHtoDAsync_v2` | Device pointer, host pointer, `usize` bytes, stream in, `CuResult` out | `Stream::copy_h2d`. |
| `memcpy_dtoh_async_v2` | `CuMemcpyDtoHAsyncV2` | `cuMemcpyDtoHAsync_v2` | Host pointer, device pointer, `usize` bytes, stream in, `CuResult` out | `Stream::copy_d2h` and `enqueue_d2h`. |
| `memcpy_dtod_async_v2` | `CuMemcpyDtoDAsyncV2` | `cuMemcpyDtoDAsync_v2` | Destination and source device pointers, `usize` bytes, stream in, `CuResult` out | `Stream::copy_d2d` and `enqueue_d2d`. |
| `launch_kernel` | `CuLaunchKernel` | `cuLaunchKernel` | Function, grid/block dimensions, shared bytes, stream, parameter pointer array, extra pointer in, `CuResult` out | `Stream::launch` and `enqueue_launch`; extra is always null. |
| `module_get_loading_mode` | `Option<CuModuleGetLoadingMode>` | `cuModuleGetLoadingMode` | `*mut c_int` mode out, `CuResult` out; optional | `Driver::module_loading_mode`. |
| `get_error_name` | `Option<CuGetErrorName>` | `cuGetErrorName` | `CuResult` status in, `*mut *const c_char` text out, `CuResult` out; optional | `Driver::error_text` through `check`. |
| `get_error_string` | `Option<CuGetErrorString>` | `cuGetErrorString` | `CuResult` status in, `*mut *const c_char` text out, `CuResult` out; optional | `Driver::error_text` through `check`. |

### `Api::load`

`DynamicLibrary::load_api()` delegates to `Api::load(&self)`
([`ffi.rs`](../../src/ffi.rs#L273-L414)). `Api::load` performs these phases in
source order:

1. It calls `required(resolver, symbol)` for every entry needed to initialize,
   discover, create contexts, load modules, allocate memory, create and query
   streams and events, enqueue copies, and launch kernels.
2. Only after all required calls succeed, it calls `optional(resolver, symbol)`
   for `DeviceGetUuidV2`, `ModuleGetLoadingMode`, `GetErrorName`, and
   `GetErrorString`.
3. It starts a `BTreeSet` with every entry in `REQUIRED_DRIVER_SYMBOLS`.
   Each optional symbol whose typed pointer is `Some` is inserted. The set is
   stored in `DriverCapabilities` beside the function pointers.
4. It returns one fully populated `Api` value. No later method performs a
   lookup or changes the set.

The required and optional function-pointer fields are typed aliases in
`ffi.rs`. All aliases use the CUDA Driver C ABI and return `CuResult`; handles
are represented as opaque pointers, and `CUdeviceptr` is `u64`. The raw aliases
and the `Api` fields remain crate-private. Only `DriverCapabilities` and
`DriverSymbol` cross the public facade.

`required<T>` calls the unsafe resolver once. `Ok(Some(pointer))` is converted
to the requested function-pointer type. `Ok(None)` becomes
`CudaError::MissingRequiredSymbol { symbol, detail: None }`; `Err(detail)`
becomes the same variant with `detail: Some(detail)`. Thus a missing symbol
and a `dlsym` failure are distinguishable in the stored error detail, while
both remain construction failures.

`optional<T>` calls the resolver once and uses `.ok().flatten()`. A null
pointer or any `dlsym` error becomes `None`. Optional lookup diagnostics are
therefore intentionally not returned as construction errors. The resulting
absence is observable through `Driver::supports` and the optional API methods.

`pointer_as_function` checks that the function-pointer type and
`*mut c_void` have equal sizes, then performs `mem::transmute_copy` in the
reviewed unsafe FFI boundary ([`ffi.rs`](../../src/ffi.rs#L416-L443)). The size
assertion is the only runtime representation check. The source does not
validate a function's ABI signature against metadata returned by the driver.

### Dynamic symbol lookup

`DynamicLibrary` implements the private `SymbolResolver` trait. `lookup`
clears the thread-local `dlerror`, calls `dlsym(handle, symbol.c_name())`, then
reads `dlerror` exactly once. A nonempty loader error returns `Err(detail)`;
otherwise the raw pointer is converted to `Option<NonNull<c_void>>`. A null
pointer with no loader error is `Ok(None)`, which is the required/optional
absence path described above ([`ffi.rs`](../../src/ffi.rs#L495-L507)).

`clear_dlerror` discards any previous thread-local loader error. `take_dlerror`
converts the returned C string with lossy UTF-8 and owns the resulting
`String`; it returns `None` when `dlerror` returns null
([`ffi.rs`](../../src/ffi.rs#L517-L531)). No loader detail is retained after
resolution except a required-symbol error detail or an open attempt detail.

## Capability view

`DriverCapabilities` stores one private `BTreeSet<DriverSymbol>`
([`ffi.rs`](../../src/ffi.rs#L258-L271)). Its public operations are:

| Operation | Contract |
| --- | --- |
| `supports(symbol)` | Membership test. It is O(log n) over the set and has no loader side effect. |
| `available_symbols()` | Returns an `ExactSizeIterator<Item = DriverSymbol>` borrowing the set. BTree ordering means symbols are yielded in enum order. The iterator length is the exact number of resolved required plus optional symbols. |

The set is initialized only by `Api::load`. Every successfully constructed
`Driver` has all 32 required symbols in the set. Optional membership is a
per-library observation and can differ between two separately loaded Drivers.
`Driver::capabilities()` borrows the set; `Discovery::discover()` clones it
into its immutable snapshot; `DeploymentIdentity::from_discovery()` clones
that snapshot again for artifact compatibility checks. No public API can add or
remove a capability after loading.

`DriverSymbol` is non-exhaustive. Callers that enumerate it must include a
wildcard arm in future source changes, and callers must not assume that the
current 36 names are permanent. The two exported slices are the source of
truth for required versus optional policy at the current version.

## Module-loading mode query

`Driver::module_loading_mode()` is the only operation in this module that
directly calls an optional function pointer. It reads
`self.inner.api.module_get_loading_mode` once:

```text
optional pointer absent
  -> Ok(None)

optional pointer present
  -> call cuModuleGetLoadingMode(&mut raw_mode)
       -> nonzero status: DriverCall with operation cuModuleGetLoadingMode
       -> raw_mode == 1: Ok(Some(ModuleLoadingMode::Eager))
       -> raw_mode == 2: Ok(Some(ModuleLoadingMode::Lazy))
       -> any other value: InvalidDriverValue { operation, detail }
```

The output is an observation, not a request to change CUDA's loading policy.
The raw output variable is a `c_int` initialized to zero. The method checks
the Driver status through `check`, so an available `cuGetErrorName` or
`cuGetErrorString` contributes rich text to a failure. It does not cache the
answer, and it does not treat an absent symbol as an error. No current
workspace caller invokes this method during discovery or execution; callers
that need this observation may issue it against an already loaded `Driver`.

## Status conversion and error text

### Rich path: `check`

`Driver::check(operation, status)` is the conversion used by initialization,
discovery, context creation and stack operations, memory observations, module
and resource creation or destruction, and the optional loading-mode query. The
algorithm is exact:

```text
if status == CUDA_SUCCESS (0):
    return Ok(())
else:
    name = error_text(status, api.get_error_name)
    description = error_text(status, api.get_error_string)
    return Err(CudaError::DriverCall(DriverCallError {
        operation,
        status: DriverStatus(status),
        name,
        description,
    }))
```

`error_detail` asks the two optional functions independently. A missing
function, a non-success status returned by the error-text function, or a null
output pointer yields `None` for that field. If a pointer is returned, the
Driver-owned C string is read as a NUL-terminated `CStr`, converted with lossy
UTF-8, and copied into a Rust `String`; the pointer is never freed or retained
by Recipe. A failure to obtain rich text never hides or replaces the original
numeric status.

`DriverCallError` stores the fixed operation label, `DriverStatus`, and both
optional text fields. Its `Display` form is
`<operation> failed with CUDA status <number>`, optionally followed by
`(<name>)` and `: <description>` ([`error.rs`](../../src/error.rs#L12-L30)).
`CudaError::DriverCall` exposes that value as its `source()`; loader and value
validation errors do not have a nested source ([`error.rs`](../../src/error.rs#L33-L141)).

### Bounded path: `check_status_only`

`check_status_only` has the same zero-versus-nonzero rule, but it constructs
`DriverCallError` with `name: None` and `description: None`. It does not call
`cuGetErrorName`, `cuGetErrorString`, allocate text, or perform another Driver
operation. The runtime submission and poll paths deliberately use this method
after realization so a live loop retains the exact numeric failure without
introducing rich-text work. This is a bounded error representation, not a
different success policy.

The status-only callers are the event-backed and enqueue-only methods in
[`runtime.rs`](../../src/runtime.rs#L405-L672), `Stream::poll_idle`,
`Pending::poll`, and `query_status` at the end of that file. `query_status`
maps exactly `CUDA_SUCCESS` to `Complete` and `CUDA_ERROR_NOT_READY` to
`Pending`; any other status is passed to `check_status_only` and returned as a
`DriverCall`.

## Direct caller trace

The following are all production source paths that call a `Driver` method or
read its crate-private API table. Higher layers that only carry a
`DeploymentIdentity` or a `CudaBinding` are listed separately because they
consume the snapshot produced by a `Driver`, not the live table itself.

| Source and function | Driver entry or fields used | Input, output, and lifetime effect |
| --- | --- | --- |
| [`cuda/src/discovery.rs:150-189`](../../src/discovery.rs#L150-L189), `Driver::discover` | `check`; `api.init`, `driver_get_version`, `device_get_count`; `capabilities().clone()` | Calls `cuInit(0)`, obtains a nonnegative version and device count, discovers every device, rejects duplicate UUIDs, and returns a fresh `Discovery` with a cloned capability set. The `Driver` remains the owner of the API; the snapshot owns copied values only. |
| [`cuda/src/discovery.rs:191-295`](../../src/discovery.rs#L191-L295), `discover_device` | `check`; `api.device_get` | Converts one raw ordinal to a `DeviceInfo`, retaining a crate-private raw `CUdevice` handle for later context creation. Every status is rich-checked immediately. |
| [`cuda/src/discovery.rs:297-428`](../../src/discovery.rs#L297-L428), device helpers | `api.device_get_pci_bus_id`, `device_get_name`, optional `device_get_uuid_v2`, required `device_get_uuid`, `device_total_mem_v2`, `device_get_attribute` | Reads fixed buffers and numeric attributes. Optional UUID v2 falls back only on `CUDA_ERROR_NOT_SUPPORTED`; all other v2 failures are returned. Malformed strings, negative attributes, invalid Boolean concurrency, and duplicate UUIDs become `CudaError` values rather than guessed data. |
| [`cuda/src/context.rs:102-134`](../../src/context.rs#L102-L134), `Context::create` | `check`; `api.ctx_create_v2`, `ctx_pop_current_v2`, `ctx_destroy_v2` on cleanup | Validates `ContextFlags`, creates one raw context, immediately pops and verifies the exact handle, then clones the `Driver` into `Context`. A failed pop or stack mismatch destroys the just-created raw context before returning. |
| [`cuda/src/context.rs:138-179`](../../src/context.rs#L138-L179), context methods | `check`; `api.ctx_push_current_v2`, `mem_get_info_v2`, `ctx_destroy_v2` | `enter` pushes an open context and returns a guard. `memory_info` enters, reads free and total bytes, leaves, then returns the observation. `close` consumes the context and destroys it. `Context::drop` destroys any remaining raw context while discarding the Driver status. |
| [`cuda/src/context.rs:206-239`](../../src/context.rs#L206-L239), `ContextGuard` | `check`; `api.ctx_pop_current_v2` | `leave` pops once and verifies that the returned handle equals the pushed handle. Guard drop performs the same pop best effort if still active. A pop status failure leaves `active` true during the failed call, so the drop path may attempt the pop again if the guard then unwinds. |
| [`cuda/src/runtime.rs:21-43`](../../src/runtime.rs#L21-L43), `with_current` | `Context::enter`, `context.driver()`, `ContextGuard::leave` | Every runtime call enters the context, invokes a closure with `&Driver`, then attempts to leave. If the operation fails, the leave result is discarded and the operation error is returned. If the operation succeeds, a leave error is returned. This is the single current-context routing helper. |
| [`cuda/src/runtime.rs:45-133`](../../src/runtime.rs#L45-L133), `Module` | `check`; `api.module_load_data`, `module_get_function`, `module_unload` | Loads only ELF cubin bytes, resolves a nonempty NUL-free entry name, and owns the raw module. `Function` borrows the module, so it cannot outlive it. Explicit unload consumes the module; `Drop` unloads best effort. |
| [`cuda/src/runtime.rs:145-235`](../../src/runtime.rs#L145-L235), `DeviceBuffer` | `check`; `api.mem_alloc_v2`, `mem_free_v2` | Rejects zero-byte allocations, checks a non-null returned device pointer, and stores the allocation length plus a context borrow. Explicit `free` consumes the buffer; `Drop` frees a remaining pointer best effort. |
| [`cuda/src/runtime.rs:238-327`](../../src/runtime.rs#L238-L327), `PinnedHostBuffer` | `check`; `api.mem_host_alloc`, `mem_free_host` | Rejects zero length, allocates pinned host memory, zero-initializes it, and exposes slices tied to the context lifetime. Explicit `free` consumes the buffer; `Drop` frees a remaining pointer best effort. |
| [`cuda/src/runtime.rs:370-403`](../../src/runtime.rs#L370-L403), `Stream` creation and idle query | `check` for create; `check_status_only` through `query_status` for query; `api.stream_create`, `stream_query` | Creates a nonblocking stream with flag `1`, verifies a non-null handle, and reports `Complete` or `Pending` from `cuStreamQuery`. Stream ownership borrows the context and ends through explicit `destroy` or best-effort `Drop`. |
| [`cuda/src/runtime.rs:405-572`](../../src/runtime.rs#L405-L572), copy methods | `check_status_only`; `api.memcpy_htod_async_v2`, `memcpy_dtoh_async_v2`, `memcpy_dtod_async_v2`, `event_record` | Checks same-context identity and byte ranges before enqueue. Event-backed copies record an event and return `Pending` carrying the event and operation borrow. Enqueue-only copies return `()` and require the caller to retain stream and allocations until `poll_idle` is terminal. |
| [`cuda/src/runtime.rs:574-673`](../../src/runtime.rs#L574-L673), launch methods | `check_status_only`; `api.launch_kernel`, `event_record` | Checks function, event, and keepalive context identity; passes launch dimensions and parameter pointers to `cuLaunchKernel`. `launch` records an event and returns a `Pending` token that keeps referenced Rust borrows live. `enqueue_launch` returns `()` and requires caller-owned lifetimes through stream idle. |
| [`cuda/src/runtime.rs:699-746`](../../src/runtime.rs#L699-L746), `Event` | `check`; `api.event_create`, `event_destroy_v2` | Creates a timing-disabled event with flag `2`, verifies non-null, and destroys explicitly or in `Drop`. Event creation and destruction use rich Driver status text. |
| [`cuda/src/runtime.rs:755-839`](../../src/runtime.rs#L755-L839), `Pending` and `query_status` | `check_status_only` through `with_current` and `query_status`; `api.event_query` | Polls one event without blocking, marks the token complete on terminal success, waits by repeated poll and `thread::yield_now`, returns the same token on timeout, and recycles the event only after a terminal poll. A pending token is `#[must_use]` and carries a phantom borrow for operation resources. |
| [`native-probe/src/cuda.rs:86-106`](../../native-probe/src/cuda.rs#L86-L106), `CudaBackend::open` | `Driver::load_from_path`, `Driver::discover` | Receives a path selected and identity-pinned by native-probe, loads one Driver, performs exhaustive discovery, and wraps CUDA errors as `ProbeError::Discovery`. If no NVIDIA PCI accelerator is present it returns `None` before loading. |
| [`native-probe/src/cuda.rs:236-446`](../../native-probe/src/cuda.rs#L236-L446), benchmark path | `Driver` is held in the tuple from `benchmark_open`; Context and runtime wrappers use its cloned table | Reopens and matches the exact measured device, creates a context and stream, verifies H2D, D2H, D2D, and a Recipe-owned cubin launch, and drives every `Pending` token to completion. The driver remains alive through context and resource teardown. |
| [`native-probe/src/bindings.rs:268-317`](../../native-probe/src/bindings.rs#L268-L317), `realize_cuda` | `CudaBackend::open`, `DeploymentIdentity::from_discovery`, `Context::create` | Reopens every measured CUDA device, derives identity from the same discovery snapshot, creates contexts, and stores them in callback-scoped `RealizedCuda` values. `CudaBinding` borrows each context, so no Driver or context handle escapes the higher-ranked preparation callback. |

There are no direct `Driver::load`, `Driver::module_loading_mode`, or
`Driver::supports` calls outside these paths in the current workspace.
`Driver::load_from_path` is the production constructor; `Driver::load` is the
library-default convenience path exposed to advanced callers.

## Indirect consumers of the capability snapshot

The live `Driver` table is intentionally not passed into planning or finalized
execution. Those layers consume immutable values derived from discovery:

1. `DeploymentIdentity::from_discovery` requires the supplied device's ordinal
   and UUID to occur in the same `Discovery`, then copies the Driver version,
   UUID, compute target, and `DriverCapabilities`
   ([`artifact.rs`](../../src/artifact.rs#L31-L55)). A device not in that
   snapshot returns `None` rather than creating an identity from unrelated
   fields.
2. `src/native_prepare.rs::cuda_spec` builds a `BTreeSet` from
   `REQUIRED_DRIVER_SYMBOLS` and rejects a deployment whose copied capability
   set lacks any required entry ([`src/native_prepare.rs`](../../../src/native_prepare.rs#L651-L702)).
   This is a policy check on the immutable deployment identity, not another
   `dlsym` call.
3. `prepare/src/production.rs::runtime_kind` embeds the measured Driver
   version range and required symbol set in `recipe_cuda::ArtifactIdentity`
   ([`prepare/src/production.rs`](../../../prepare/src/production.rs#L577-L607)).
   Artifact compatibility later compares those values with the deployment;
   it does not reopen or query a Driver.
4. `native-executor` receives borrowed `Context` values through
   `CudaBinding`. Its CUDA backend calls the runtime wrappers, maps
   `CudaError` to its own `Error::Cuda`, and never reaches `Driver::inner`
   ([`native-executor/src/error.rs`](../../../native-executor/src/error.rs#L229-L233)).
   Cross-backend staging similarly converts `CudaError` to
   `StagedBridgeError::Cuda` ([`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L157-L163)).

The root facade's `recipe::engine::cuda` re-export is presentation only
([`src/facade.rs`](../../../src/facade.rs#L17-L41)); it does not add a second
loader, cache, or compatibility policy.

## Lifetimes and thread safety

### Dynamic library and function table

`DynamicLibrary` contains a `NonNull<c_void>` from `dlopen` and the diagnostic
name. The source explicitly declares `unsafe impl Send` and `unsafe impl Sync`
for this type ([`ffi.rs`](../../src/ffi.rs#L445-L452)). `Api` contains only
typed `unsafe extern "C"` function pointers, `Option` wrappers for optional
ones, and an immutable `DriverCapabilities` set. Consequently `Driver`, whose
only field is `Arc<DriverInner>`, is automatically `Send` and `Sync` under the
current definitions, in addition to being `Clone`. The source does not add a
mutex around API calls or serialize access to the dynamic handle. Any
thread-safety behavior of the NVIDIA Driver itself remains the Driver API's
contract; this module only guarantees that the shared loader handle and table
remain alive and immutable.

The `Send` and `Sync` assertions are a reviewed FFI boundary, not a proof that
every opaque CUDA resource is thread-safe. `Context` deliberately opts out of
both auto traits with `PhantomData<Rc<()>>` and therefore cannot cross a thread
boundary. All runtime objects borrow a `Context`, so their lifetimes and trait
behavior are constrained by that context even though they use the shared
`Driver` internally.

### Context and runtime ownership

`Context::create` clones the caller's `Driver` into the context. A caller may
drop its original `Driver` after creation; the context's clone still owns the
`Arc`, `Api`, and library. Conversely, a caller may keep a separate `Driver`
clone after a context is closed; the library then remains loaded until that
last clone is dropped. `Context::drop` destroys the raw context before its
`driver` field is dropped, so the API table remains available for the best
effort destroy call.

`Module<'ctx>`, `DeviceBuffer<'ctx>`, `PinnedHostBuffer<'ctx>`, `Stream<'ctx>`,
and `Event<'ctx>` all store a borrow of the context. `Function<'module, 'ctx>`
stores a borrow of its module and therefore indirectly the context. A
`Pending<'op, 'ctx>` stores one event plus `PhantomData<&'op ()>`; the
operation borrow is represented at the type level even though the event field
does not contain the referenced buffers. This is what prevents a caller from
destroying or mutating borrowed resources before event completion when using
the event-backed methods.

The enqueue-only runtime methods return `()` and do not create a token. Their
unsafe contracts require the caller to keep every referenced resource and the
stream alive until `Stream::poll_idle()` reports `Complete`. The Driver module
does not invent a background waiter, cancellation path, or replacement event.

`Pending::wait` has no cancellation semantics. If its deadline expires, it
returns `WaitOutcome::TimedOut(Pending)` with the same event and operation
borrow. Dropping that token while the Driver may still access resources is a
caller contract violation. Native-probe's `complete_cuda` therefore continues
polling one timed-out token until it reaches completion, then reports the
benchmark deadline error ([`native-probe/src/cuda.rs`](../../native-probe/src/cuda.rs#L482-L517)).

### Current-context scope

The Driver table itself has no current-context state. `Context::enter` and
`ContextGuard::leave` call the resolved push/pop symbols, and runtime's
`with_current` wraps every direct resource operation in that pair. On an
operation error, `with_current` still attempts the pop but deliberately returns
the original operation error, discarding the pop result; on operation success,
the pop result is propagated. A guard dropped while active makes the same pop
call best effort. Thus the `Driver` is shareable, but the raw current-context
handle is constrained by non-Send/Sync `Context` ownership and by these
scoped guards.

## Error taxonomy at the Driver boundary

The complete `CudaError` variants are defined in
[`error.rs`](../../src/error.rs#L33-L71). Driver loading and status conversion
use the following variants directly:

| Error | Produced by | Exact payload and meaning |
| --- | --- | --- |
| `LibraryOpen { attempts }` | `DynamicLibrary::open` and `open_default` | One or two strings in attempt order. Each string is `<input path>: <dlerror text>` or uses `unknown dlopen error`. Default opening aggregates only open failures. |
| `InvalidLibraryPath { path }` | `DynamicLibrary::open` | The supplied path contained an interior NUL and could not become a C string. No `dlopen` call occurred. |
| `MissingRequiredSymbol { symbol, detail }` | `required` during `Api::load` | `symbol` is the human-readable `DriverSymbol::as_str()` value. `detail` is `None` for a null lookup without `dlerror`, or `Some` with the loader's `dlsym` error text. Construction stops at the first missing required symbol in source order. |
| `DriverCall(DriverCallError)` | `Driver::check`, `check_status_only`, and callers that pass their statuses through them | `operation` is the caller's static label; `status` is the exact nonzero `CuResult`; rich path may include optional name and description; status-only path always leaves both absent. |
| `InvalidDriverValue { operation, detail }` | `module_loading_mode` for unknown raw mode, and discovery/context/runtime callers that validate Driver output after a successful status | The Driver returned success but an output value violated the wrapper contract, such as a null created handle, negative count, malformed text, or unknown mode. |

The remaining variants are generated by downstream wrappers before or after a
Driver call: `InvalidInput` for caller-side lengths, ranges, names, launch
geometry, or timeout overflow; `InvalidDeviceName` and
`DuplicateDeviceUuid` for discovery identity; `InvalidContextFlags`,
`ContextStackMismatch`, and `ContextClosed` for context ownership;
`ResourceContextMismatch` for cross-context runtime resources. They preserve
the same `Result<T>` boundary and are not converted into loader errors.

`CudaError` is `#[non_exhaustive]`; code outside the crate must include a
wildcard arm when matching it. `Display` retains the operation and detail, and
`std::error::Error::source()` returns a `DriverCallError` only for the
`DriverCall` variant. Higher layers wrap the value without changing the
underlying status, for example `native-executor::Error::Cuda` and
`native-probe`'s formatted discovery or benchmark errors.

### Error precedence and cleanup

The Driver module does not add retries, alternate symbol names, or fallback
resources. The observable precedence is:

```text
path CString validation
  -> dlopen result
  -> required dlsym sequence
  -> optional dlsym observations
  -> Driver method call status
  -> wrapper output validation
```

When a wrapper has already acquired a raw resource and then detects an invalid
success output, it reports `InvalidDriverValue` and lets the surrounding owner
perform its documented best-effort cleanup. For example, `Context::create`
destroys a null-verified or stack-mismatched raw context before returning, and
runtime owners take their `Option<raw>` field before explicit destruction so a
second `free`, `destroy`, or `unload` reports `ContextClosed` instead of
calling the Driver twice.

## End-to-end production path

The live Driver owner participates in one measured, callback-scoped path:

```text
NativeProbeConfig
  -> native-probe candidate validation and library identity
  -> CudaBackend::open
       -> Driver::load_from_path(selected canonical UTF-8 path)
       -> Driver::discover
       -> exact GpuDescriptor / MeasuredProfile

with_native_execution_bindings
  -> reopen Driver and fresh Discovery
  -> match exact device UUID, ordinal, PCI identity, and target
  -> DeploymentIdentity::from_discovery
  -> Context::create (Context clones Driver)
  -> higher-ranked callback receives CudaBinding borrowing Context

recipe-prepare
  -> check copied required DriverCapabilities
  -> embed Driver version and symbol policy in ArtifactIdentity

native-executor
  -> load validated cubin modules and functions before the finalized loop
  -> allocate buffers, streams, events, and completion slots
  -> submit copies and launches through runtime wrappers
  -> poll event or stream status until terminal
  -> destroy resources and contexts while the Driver Arc remains alive
```

`CudaBackend::open` returns `(PinnedLibrary, Driver, Discovery)` only while
the probe owns that discovery result. `benchmark_device` retains the Driver
local while creating a context, stream, buffers, modules, events, and pending
tokens. Resource borrows ensure the Driver-backed context and library remain
valid for every asynchronous operation. The benchmark verifies transfer and
kernel output independently and drives a timed-out token to completion because
the Driver API exposes no cancellation symbol.

`realize_cuda` repeats the open and discovery sequence instead of reusing a
stale Driver. It derives each `DeploymentIdentity` from the same fresh
snapshot, creates one context per measured device, then gives only borrowed
`CudaBinding` values to a higher-ranked callback. The callback cannot return a
value borrowing those contexts, and after the callback returns the contexts
drop before their cloned Driver owners release the dynamic library.

The native executor's finalized state owns runtime objects and references the
borrowed contexts; it does not retain a `Driver` in a global catalog or reopen
one during a loop. Discovery, compilation, module loading, allocation, and
event creation remain pre-loop work. The Driver boundary only executes the
already-resolved C calls selected by those immutable plans.

## Invariants and non-goals

The source establishes these observable invariants:

1. A successful `Driver` has every current required symbol resolved and keeps
   the supplying library open for its entire shared lifetime.
2. Optional symbol absence is represented as `None` and a missing capability;
   it is never silently replaced by a differently named symbol.
3. `Driver::load` attempts default sonames in fixed order and aggregates only
   open failures. An API-resolution failure does not trigger another soname.
4. `loaded_library`, `capabilities`, and `supports` are read-only observations;
   none performs I/O or a new dynamic lookup.
5. Status zero is the only success status accepted by `check` and
   `check_status_only`. Every other status remains visible as a numeric
   `DriverStatus`, with optional rich text only on the rich path.
6. The module never interprets a C string after copying it and never frees a
   Driver-owned error-text pointer.
7. Driver loading and API resolution are not cached globally. Every explicit
   `load` or `load_from_path` creates its own `DynamicLibrary`, `Api`, and
   capability snapshot.
8. Runtime ownership, current-context discipline, asynchronous lifetimes, and
   artifact compatibility are enforced by the neighboring modules. This
   module does not add a global context, scheduler, compiler, cancellation
   worker, or fallback implementation.

The module intentionally does not promise that a CUDA context can cross
threads, that arbitrary C function-pointer signatures are safe, that the
Driver's internal calls are serialized, or that a timed-out operation has been
cancelled. Those assumptions are either rejected by the type/lifetime layer or
left to the reviewed CUDA Driver contract at the FFI boundary.

## Verification pointers

The direct source locations for this trace are:

| Contract area | Source |
| --- | --- |
| `Driver`, status conversion, and mode mapping | [`cuda/src/driver.rs:12-142`](../../src/driver.rs#L12-L142) |
| C ABI aliases, symbol inventory, `Api::load`, and capabilities | [`cuda/src/ffi.rs:10-443`](../../src/ffi.rs#L10-L443) |
| `dlopen`, `dlsym`, `dlerror`, and `dlclose` ownership | [`cuda/src/ffi.rs:445-531`](../../src/ffi.rs#L445-L531) |
| Error variants and formatting | [`cuda/src/error.rs:1-141`](../../src/error.rs#L1-L141) |
| Driver discovery and optional UUID fallback | [`cuda/src/discovery.rs:150-428`](../../src/discovery.rs#L150-L428) |
| Context clone, current stack, and thread-affinity marker | [`cuda/src/context.rs:95-240`](../../src/context.rs#L95-L240) |
| Runtime current-context routing and asynchronous calls | [`cuda/src/runtime.rs:21-839`](../../src/runtime.rs#L21-L839) |
| Native probe load, discovery, benchmark, and cleanup | [`native-probe/src/cuda.rs:75-522`](../../../native-probe/src/cuda.rs#L75-L522) |
| Callback-scoped context reopening | [`native-probe/src/bindings.rs:120-317`](../../../native-probe/src/bindings.rs#L120-L317) |
| Capability policy and artifact identity | [`src/native_prepare.rs:651-702`](../../../src/native_prepare.rs#L651-L702), [`prepare/src/production.rs:577-607`](../../../prepare/src/production.rs#L577-L607) |
| Runtime error propagation | [`native-executor/src/error.rs:229-233`](../../../native-executor/src/error.rs#L229-L233), [`native-executor/src/bridge.rs:157-163`](../../../native-executor/src/bridge.rs#L157-L163) |

Structural checks for this module are:

```text
cargo check -p recipe-cuda
cargo doc -p recipe-cuda --no-deps
```

These commands check Rust and documentation structure only. They do not prove
that a CUDA library is installed, that every listed symbol is present on a
deployment, or that asynchronous operations complete correctly. Runtime proof
comes from the production native-probe and complete CUDA acceptance paths,
which exercise the exact `Driver::load_from_path` to discovery, context,
resource, submit, poll, and teardown sequence described above.
