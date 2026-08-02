# `hsa/src/loader.rs`: the ROCr dynamic ABI boundary

```toml
[module]
path = "hsa/src/loader.rs"
kind = "private-posix-dynamic-abi-loader"
intent = "Open one ROCr shared library, resolve the exact HSA entry points used by recipe-hsa, and keep the defining handle alive for every typed call."
purpose = "Turn a configured soname or path into one immutable, shareable Api without linking recipe-hsa against a system ROCr installation."
state = "one NonNull<c_void> dlopen handle, its display name, and 32 unsafe extern C function pointers"
public_surface = "crate-private Library and Api; Runtime is the only public owner"

[boundary]
inputs = ["OsStr soname or filesystem path", "static NUL-terminated symbol names", "ROCr HSA status values"]
outputs = ["Result<Api>", "Result<()> from check and check_status_only", "typed Error values"]
loader_flags = ["RTLD_NOW", "RTLD_LOCAL"]
diagnostic_source = "thread-local dlerror copied immediately into String"
abi_source = "hsa/src/abi.rs declarations reviewed against hsa.h and hsa_ext_amd.h from ROCm 7.2"

[lifetime]
handle_owner = "Library inside Api"
shared_owner = "Arc<Api> cloned by Runtime and native resource owners"
runtime_balance = "Runtime::open calls hsa_init once; close or Drop calls hsa_shut_down once"
child_rule = "Rust borrows keep Discovery, Session, Queue, Allocation, Executable, and Pending scopes under Runtime"

[non_goals]
linking = "no link-time ROCr dependency and no system-wide singleton"
selection = "no soname search, symbol version negotiation, retry, or alternate implementation inside loader.rs"
semantics = "no topology interpretation, allocation policy, scheduling, packet construction, or kernel validation"
platform = "no Windows loader path; the source uses the Unix OsStrExt and libc dlopen API"
```

`loader` is a private module of `hsa/src/lib.rs:12-19`. It is the only source
of raw dynamic-loader calls in `recipe-hsa`. The module does not expose
`Library` or `Api` outside the crate. Public callers use `Runtime`, which owns
an `Arc<Api>` and supplies the discovered agents and scoped execution objects
that eventually invoke these pointers.

## Boundary and call graph

The normal public path is:

```text
Runtime::open_default() or Runtime::open(path)
  -> Api::load(path)
     -> Library::open(path)
        -> dlopen(RTLD_NOW | RTLD_LOCAL)
     -> Library::symbol(...) x 32
        -> dlsym + dlerror check
        -> transmute to an abi.rs function-pointer alias
  -> hsa_init()
  -> Runtime { api: Arc<Api>, active: true }
```

`Runtime::open_default` (`hsa/src/runtime.rs:22-32`) tries
`libhsa-runtime64.so.1`, then `libhsa-runtime64.so`. It advances to the next
candidate only for `Error::LibraryOpen`. A missing symbol, an ABI operation
failure, or an `hsa_init` failure is returned immediately. The loader itself
does not iterate candidates.

`Runtime::open` (`hsa/src/runtime.rs:34-50`) accepts any `AsRef<OsStr>`, so the
argument can be a soname understood by the system loader or an explicit path.
It constructs the complete `Api` before calling `hsa_init`. If initialization
fails, the `Arc<Api>` is dropped while returning the error, which closes the
library handle without calling `hsa_shut_down` because no successful
initialization reference exists.

There are three relevant caller families:

| caller | path into the loader | lifetime consequence |
| --- | --- | --- |
| `hsa/examples/discover.rs:4-12` | `Runtime::open_default`, then `runtime.discover()` | The `Discovery` borrow ends before `runtime.close()`. |
| `hsa/examples/execute_smoke.rs:6-134` | Open, discover, turn an agent into a `Session`, allocate/copy or load HSACO, then close children and the runtime | `Session`, allocations, queues, executables, and pending tokens are all destroyed before `Runtime::close()`. |
| `native-probe/src/hsa.rs:112-149` | `HsaBackend::with_runtime` selects a pinned `BackendLibrary`, calls `Runtime::open` once, stores it in `RefCell<Option<HsaRuntimeState>>`, and reuses it | The probe retains both the selected library identity and the initialized `Runtime` for later discovery and bounded benchmark calls. |

The CLI supplies the pinned candidate path through
`src/cli.rs:1037-1043,1095-1099`; default native candidates are listed at
`src/cli.rs:1816-1823`. `native-probe/src/native.rs:245-298` calls the HSA
backend for discovery and benchmark ownership, and
`native-probe/src/bindings.rs:179-215` keeps the callback and all borrowed HSA
bindings inside one `with_runtime` scope. `native-executor` receives those
bindings, not `Api` directly. No caller can bypass `Runtime` to construct an
`Api` or invoke a loader function without the crate-private boundary.

## Structure

| source region | item | role |
| --- | --- | --- |
| `hsa/src/loader.rs:22-25` | `Library` | Owns a non-null `dlopen` handle and the lossy display copy of the requested path. |
| `hsa/src/loader.rs:27-56` | `Library::open` | Converts `OsStr` bytes to a temporary NUL-terminated `CString`, clears the calling thread's old loader error, opens with eager local binding, and converts a null handle to `Error::LibraryOpen`. |
| `hsa/src/loader.rs:58-88` | `Library::symbol` | Resolves one static symbol, checks `dlerror` before interpreting the address, rejects a null address, and returns an untyped address for the ABI transmute. |
| `hsa/src/loader.rs:91-98` | `Drop for Library` | Calls the one matching `dlclose`; the return value is ignored. |
| `hsa/src/loader.rs:101-107` | unsafe `Send` and `Sync` impls | State the POSIX assumptions that permit the immutable handle to be retained and used by `Arc<Api>` owners across threads. |
| `hsa/src/loader.rs:109-130` | `clear_dlerror`, `take_dlerror` | Encapsulate the thread-local `dlerror` protocol. `take_dlerror` copies the C string before any later loader call. |
| `hsa/src/loader.rs:132-166` | `Api` | Stores `_library` plus one typed pointer for each required HSA operation. The field is not optional: a complete API is all-or-error. |
| `hsa/src/loader.rs:168-270` | `Api::load` | Opens once, expands the `symbol!` macro for all 32 names, transmutes each address to the alias declared in `abi.rs`, and constructs the immutable table. |
| `hsa/src/loader.rs:272-312` | status conversion | `check` returns rich status text when safe, `check_status_only` returns only the numeric status for realized submit paths, and `status_message` calls `hsa_status_string`. |

The only mutable loader state is the thread-local diagnostic state maintained
by the platform loader. After `Api::load` returns, no code performs symbol
lookup again. Function-pointer fields are immutable, and `_library` remains
part of the same owner for as long as any `Api` exists.

## Opening a library

`Library::open` takes an `&OsStr`, uses `OsStrExt::as_bytes`, and records
`path.to_string_lossy().into_owned()` as `display_name`
(`hsa/src/loader.rs:28-30`). The display copy is for diagnostics only. A path
with non-UTF-8 bytes can still be passed to `dlopen`; its error text uses the
lossy display representation.

`CString::new(path.as_bytes())` is the first fallible boundary. An interior
NUL cannot be represented as one C path and becomes
`Error::PathContainsNul { path: display_name }`. The temporary `CString`
outlives the `dlopen` call, satisfying the pointer lifetime requirement. The
loader does not canonicalize, stat, or probe the path before calling the
platform.

The call is:

```text
clear_dlerror()
dlopen(path.as_ptr(), RTLD_NOW | RTLD_LOCAL)
```

`RTLD_NOW` requires relocation and dependency resolution while opening, so an
unusable dependency fails at this boundary instead of during a later first
call. `RTLD_LOCAL` keeps the symbols local to this handle and does not publish
them to unrelated libraries. A null return is checked with `NonNull::new`.
Before returning `Error::LibraryOpen`, the implementation calls
`take_dlerror()` and copies that thread-local diagnostic. If the loader has no
diagnostic, it uses the exact fallback text
`"dynamic loader returned a null handle"`.

On success, `Library` stores the `NonNull<c_void>` handle and the display name.
There is exactly one `dlclose` in its `Drop` implementation. If symbol
resolution fails later, normal Rust unwinding drops this partially constructed
`Library`, so no half-populated `Api` escapes and the handle is not retained.

## Symbol resolution

`Library::symbol` accepts both an English symbol name and a separate static
byte string ending in NUL (`hsa/src/loader.rs:58-88`). The `debug_assert` checks
the terminator in debug builds. All call sites pass ASCII literals generated
by `concat!($name, "\\0")`; no caller-controlled symbol text reaches `dlsym`.

Each lookup follows this order:

1. Clear the prior thread-local diagnostic with `clear_dlerror`.
2. Call `dlsym(self.handle, name_with_nul)`. The handle is valid for the
   entire lookup because it belongs to the current `Library`.
3. Call `take_dlerror` immediately. POSIX requires this check because a null
   address alone is not a sufficient error test.
4. If a diagnostic exists, return `Error::MissingSymbol` with the library
   display name, the static symbol name, and the copied detail.
5. If no diagnostic exists but the address is null, return the same error with
   `"symbol resolved to a null function address"` as detail.
6. Otherwise return the untyped `*mut c_void` address.

`Api::load` expands the one local macro at
`hsa/src/loader.rs:172-179`. The macro performs the lookup and then
`core::mem::transmute::<*mut c_void, $kind>`. The transmute is unsafe because
Rust cannot check a C function's ABI from a dynamic address. The safety
contract is that the public ROCr header's spelling and calling convention are
exactly represented by the `$kind` alias in `hsa/src/abi.rs`. A wrong soname
that happens to export a same-named function is not detected beyond the ABI
contract; there is no runtime version negotiation.

The macro uses `?`, so the first missing symbol stops the sequence. Rust then
drops the local `Library`. Every required symbol is therefore present before
`Api` becomes observable.

### Unsafe assumptions at the loader boundary

The unsafe blocks in this file are small, but each one establishes a different
invariant:

| operation | source | required fact |
| --- | --- | --- |
| `dlopen` | `loader.rs:38-41` | The temporary `CString` is NUL terminated and live for the call. A non-null result is a new handle owned by this `Library`. |
| `dlsym` | `loader.rs:63-69` | The handle came from a successful `dlopen`, and the byte string is static, ASCII, and NUL terminated. The handle remains loaded while every resolved pointer can be used. |
| `dlerror` clear/copy | `loader.rs:109-129` | `dlerror` is called on the current thread. A non-null result is a loader-owned NUL-terminated string and is copied before another loader operation can replace it. |
| address transmute | `loader.rs:174-178` | The exported C symbol has exactly the signature, calling convention, enum representation, pointer mutability, and return type represented by the selected `abi.rs` alias. |
| `dlclose` | `loader.rs:91-97` | This `Library` owns the one matching handle, and no HSA function pointer is called after the final `Api` owner releases the handle. |
| status text | `loader.rs:303-310` | `hsa_status_string` is called only after successful `hsa_init`; a successful output pointer is runtime-owned, immutable, NUL terminated, and copied immediately. |

The loader does not turn these assumptions into runtime probes. ABI layout and
target-width checks are compile-time declarations in `abi.rs` and `lib.rs`;
object-specific pointer and output-storage checks remain at the call sites in
discovery, session, and execution.

## The 32 resolved entry points

The aliases below are the exact fields in `Api` and the exact `unsafe extern
"C"` declarations in `hsa/src/abi.rs:219-284`. `HsaStatus` and `HsaInfo` are
both `i32`; opaque HSA handles are `#[repr(C)]` structs containing one `u64`.
The `c_void` outputs are written by ROCr into caller-owned `MaybeUninit`
storage in the discovery and execution modules.

### Lifecycle and information

| C symbol | `abi.rs` alias | shape and direct callers |
| --- | --- | --- |
| `hsa_init` | `HsaInit` | `fn() -> HsaStatus`; called only by `Runtime::open` after all symbols resolve. |
| `hsa_shut_down` | `HsaShutDown` | `fn() -> HsaStatus`; called by `Runtime::close` or `Runtime::drop` for the one successful initialization reference. |
| `hsa_status_string` | `HsaStatusString` | `fn(HsaStatus, *mut *const c_char) -> HsaStatus`; used by `Api::status_message` after initialization to turn a numeric failure into runtime-owned text. |

### System, agent, ISA, and memory-pool discovery

| C symbol | `abi.rs` alias | shape and direct callers |
| --- | --- | --- |
| `hsa_system_get_info` | `HsaSystemGetInfo` | `fn(HsaInfo, *mut c_void) -> HsaStatus`; `discovery::system_info` reads HSA and AMD extension versions and timestamp frequency, while `DiscoveredAgent::into_session` rereads timestamp frequency. |
| `hsa_iterate_agents` | `HsaIterateAgents` | `fn(AgentCallback, *mut c_void) -> HsaStatus`; `discovery::iterate_agents` passes a synchronous stack collector. |
| `hsa_agent_get_info` | `HsaAgentGetInfo` | `fn(HsaAgent, HsaInfo, *mut c_void) -> HsaStatus`; `discovery::agent_info` reads identity, queue limits, GPU properties, and versions, and `Session::available_memory_bytes` rereads AMD available memory. |
| `hsa_agent_iterate_isas` | `HsaAgentIterateIsas` | `fn(HsaAgent, IsaCallback, *mut c_void) -> HsaStatus`; `discovery::iterate_isas` collects the agent's ISA handles synchronously. |
| `hsa_isa_get_info_alt` | `HsaIsaGetInfoAlt` | `fn(HsaIsa, HsaInfo, *mut c_void) -> HsaStatus`; `discovery::isa_info` queries machine models, profiles, rounding modes, dimensions, and limits, with one direct name-buffer call for the reported ISA spelling. |
| `hsa_amd_agent_iterate_memory_pools` | `HsaAmdAgentIterateMemoryPools` | `fn(HsaAgent, MemoryPoolCallback, *mut c_void) -> HsaStatus`; `discovery::iterate_memory_pools` collects pool handles synchronously. |
| `hsa_amd_memory_pool_get_info` | `HsaAmdMemoryPoolGetInfo` | `fn(HsaMemoryPool, HsaInfo, *mut c_void) -> HsaStatus`; `discovery::pool_info` reads required pool properties and `pool_info_optional` treats `STATUS_ERROR_INVALID_ARGUMENT` as an absent optional attribute. |

### Queue creation, destruction, and indices

| C symbol | `abi.rs` alias | shape and direct callers |
| --- | --- | --- |
| `hsa_queue_create` | `HsaQueueCreate` | `fn(HsaAgent, u32, u32, Option<QueueErrorCallback>, *mut c_void, u32, u32, *mut *mut HsaQueue) -> HsaStatus`; `Session::create_queue` supplies the discovered agent, validated size and kind, a boxed asynchronous fault context, and optional segment sizes. |
| `hsa_queue_destroy` | `HsaQueueDestroy` | `fn(*mut HsaQueue) -> HsaStatus`; `QueueCore::destroy` calls it exactly once and drops the callback context only after success. |
| `hsa_queue_load_read_index_scacquire` | `HsaQueueLoadReadIndexScAcquire` | `fn(*const HsaQueue) -> u64`; `HsaQueueIo` uses it for bounded queue occupancy/backpressure. |
| `hsa_queue_load_write_index_relaxed` | `HsaQueueLoadWriteIndexRelaxed` | `fn(*const HsaQueue) -> u64`; `HsaQueueIo` reads the producer index before selecting ring slots. |
| `hsa_queue_store_write_index_screlease` | `HsaQueueStoreWriteIndexScRelease` | `fn(*const HsaQueue, u64) -> ()`; `HsaQueueIo` publishes a fully written packet batch to ROCr. |

### Code objects and executable symbols

| C symbol | `abi.rs` alias | shape and direct callers |
| --- | --- | --- |
| `hsa_code_object_reader_create_from_memory` | `HsaCodeObjectReaderCreateFromMemory` | `fn(*const c_void, usize, *mut HsaCodeObjectReader) -> HsaStatus`; `Session::load_hsaco` keeps an `Arc<[u8]>` alive while ROCr reads the in-memory HSACO. |
| `hsa_code_object_reader_destroy` | `HsaCodeObjectReaderDestroy` | `fn(HsaCodeObjectReader) -> HsaStatus`; `ExecutableInner::destroy` calls it after destroying the executable. |
| `hsa_executable_create_alt` | `HsaExecutableCreateAlt` | `fn(i32, i32, *const c_char, *mut HsaExecutable) -> HsaStatus`; `Session::load_hsaco` passes the discovered profile, nearest rounding mode, null options, and output storage. |
| `hsa_executable_destroy` | `HsaExecutableDestroy` | `fn(HsaExecutable) -> HsaStatus`; `ExecutableInner::destroy` calls it before reader destruction. |
| `hsa_executable_load_agent_code_object` | `HsaExecutableLoadAgentCodeObject` | `fn(HsaExecutable, HsaAgent, HsaCodeObjectReader, *const c_char, *mut HsaLoadedCodeObject) -> HsaStatus`; `Session::load_hsaco` loads the reader for the exact discovered agent with null options. |
| `hsa_executable_freeze` | `HsaExecutableFreeze` | `fn(HsaExecutable, *const c_char) -> HsaStatus`; `Session::load_hsaco` freezes the loaded executable before exposing it. |
| `hsa_executable_get_symbol_by_name` | `HsaExecutableGetSymbolByName` | `fn(HsaExecutable, *const c_char, *const HsaAgent, *mut HsaExecutableSymbol) -> HsaStatus`; `Executable::kernel` resolves one caller-supplied NUL-free kernel name for the bound agent. |
| `hsa_executable_symbol_get_info` | `HsaExecutableSymbolGetInfo` | `fn(HsaExecutableSymbol, HsaInfo, *mut c_void) -> HsaStatus`; `executable_symbol_info` reads kind, kernel object, kernarg size/alignment, group/private segment sizes, and dynamic-callstack state. |

### AMD memory and asynchronous copies

| C symbol | `abi.rs` alias | shape and direct callers |
| --- | --- | --- |
| `hsa_amd_memory_pool_allocate` | `HsaAmdMemoryPoolAllocate` | `fn(HsaMemoryPool, usize, u32, *mut *mut c_void) -> HsaStatus`; `allocate_from` calls it only after pool and size validation, with flags zero. |
| `hsa_amd_memory_pool_free` | `HsaAmdMemoryPoolFree` | `fn(*mut c_void) -> HsaStatus`; `AllocationInner::destroy` calls it once for each live pointer. |
| `hsa_amd_agents_allow_access` | `HsaAmdAgentsAllowAccess` | `fn(u32, *const HsaAgent, *const u32, *const c_void) -> HsaStatus`; `grant_access` replaces an allocation's direct-access set using discovered agents, null reserved flags, and the allocation pointer. |
| `hsa_amd_memory_async_copy` | `HsaAmdMemoryAsyncCopy` | `fn(*mut c_void, HsaAgent, *const c_void, HsaAgent, usize, u32, *const HsaSignal, HsaSignal) -> HsaStatus`; `Session::copy_async` and the prepared-copy path submit validated ranges with no dependency array and an initialized completion signal. |

### Signals

| C symbol | `abi.rs` alias | shape and direct callers |
| --- | --- | --- |
| `hsa_signal_create` | `HsaSignalCreate` | `fn(i64, u32, *const HsaAgent, *mut HsaSignal) -> HsaStatus`; `SignalPool::acquire` creates a signal at value one with no consumer restriction. |
| `hsa_signal_destroy` | `HsaSignalDestroy` | `fn(HsaSignal) -> HsaStatus`; signal-record, available-pool, and deferred-retirement teardown destroy only terminal or unused signals. |
| `hsa_signal_load_scacquire` | `HsaSignalLoadScAcquire` | `fn(HsaSignal) -> i64`; dependency polling, pending completion, retirement collection, and destruction read the device-visible completion value. |
| `hsa_signal_store_screlease` | `HsaSignalStoreScRelease` | `fn(HsaSignal, i64) -> ()`; signal recycling restores value one, and queue doorbells publish the write index through the same loaded pointer. |
| `hsa_signal_wait_scacquire` | `HsaSignalWaitScAcquire` | `fn(HsaSignal, i32, i64, u64, i32) -> i64`; bounded waits use the `LT 1` condition and active-wait hint for pending operations and retirement cleanup. |

The loader does not resolve AQL packet functions. Packet layouts and atomic
publication are represented by `HsaKernelDispatchPacket`,
`HsaBarrierAndPacket`, and `HsaQueue` in `abi.rs`; `execution.rs` writes and
publishes those layouts using the queue and signal pointers above.

## ABI declarations and callback contracts

`hsa/src/abi.rs:1-5` records the ABI source of truth: public `hsa.h` and
`hsa_ext_amd.h` as reviewed against ROCm 7.2. C enum parameters use `i32`,
fixed-width queue fields use their explicit `u32` or `u64` types, and every
function pointer is `unsafe extern "C"`. `hsa/src/lib.rs:7-10` rejects a
non-64-bit target because this crate implements only the 64-bit HSA large-model
ABI. The two AQL packet structs have compile-time 64-byte size assertions at
`abi.rs:211-212`.

The eight opaque handle structures (`HsaAgent`, `HsaIsa`, `HsaSignal`,
`HsaMemoryPool`, `HsaCodeObjectReader`, `HsaExecutable`,
`HsaLoadedCodeObject`, and `HsaExecutableSymbol`) are `#[repr(C)]` one-word
values. `HsaQueue` is a readable C layout containing kind, feature bits, base
address, doorbell signal, ring size, reserved storage, and queue id. The queue
error callback reads only the id field supplied by this layout.

Four callback aliases are declared beside the function pointers:

| alias | C callback shape | owner and duration |
| --- | --- | --- |
| `AgentCallback` | `unsafe extern "C" fn(HsaAgent, *mut c_void) -> HsaStatus` | `discovery::collect_agent`; the pointer targets a stack `AgentCollector` and is valid only during synchronous `hsa_iterate_agents`. |
| `IsaCallback` | `unsafe extern "C" fn(HsaIsa, *mut c_void) -> HsaStatus` | `discovery::collect_isa`; the stack `IsaCollector` is valid only during `hsa_agent_iterate_isas`. |
| `MemoryPoolCallback` | `unsafe extern "C" fn(HsaMemoryPool, *mut c_void) -> HsaStatus` | `discovery::collect_pool`; the stack `PoolCollector` is valid only during `hsa_amd_agent_iterate_memory_pools`. |
| `QueueErrorCallback` | `unsafe extern "C" fn(HsaStatus, *mut HsaQueue, *mut c_void)` | `session::queue_error_callback`; the boxed `QueueCallbackContext` must remain stable until `hsa_queue_destroy` returns. |

The three discovery collectors check for null data, push handles inside
`catch_unwind(AssertUnwindSafe(...))`, and convert a panic into
`Error::CallbackPanicked` after ROCr returns (`hsa/src/discovery.rs:759-883`).
The pointer is never retained by ROCr beyond the documented synchronous call.

The queue callback is different. ROCr may invoke it asynchronously, so
`Session::create_queue` boxes a context containing `Arc<SharedFault>` and
passes its stable address (`hsa/src/session.rs:265-284`). The callback does no
allocation and takes no lock: it records the numeric status, optional source
queue id, and an epoch with atomics, marks the session poisoned, and wakes a
condition variable (`session.rs:80-116`). `QueueCore::destroy` drops the box
only after a successful destroy. If destruction fails, it leaks the box because
the runtime may still issue a callback (`session.rs:346-364`). This is a
resource-lifetime rule around a loader-resolved callback, not a loader retry.

## Status conversion and error behavior

`Api::check(operation, status)` is the common result boundary
(`hsa/src/loader.rs:272-281`):

* `STATUS_SUCCESS` (`0`) returns `Ok(())`.
* Every other status returns `Error::Hsa { operation, status,
  message: self.status_message(status) }`.
* `operation` is a static caller label, so callers identify the exact ABI
  operation or attribute query without allocating a second operation name.

`status_message` (`loader.rs:299-312`) calls the resolved
`hsa_status_string` pointer and supplies an output `*const c_char`. It is used
only after a successful `hsa_init`, when the ROCr runtime is initialized. A
non-success status from `hsa_status_string`, or a null returned string pointer,
produces `None`. A successful pointer is copied with `CStr::from_ptr` and
`to_string_lossy`, so the `Error` owns its text and does not retain ROCr-owned
storage.

`Api::check_status_only` (`loader.rs:283-297`) has the same success test but
always sets `message: None`. Realized submission paths use it when rich text
would allocate in the live loop. The numeric status remains visible through
`Error::Hsa` and is not replaced by a retry or a generic success value.

The loader-specific and immediate runtime errors are:

| error | produced by | exact boundary meaning |
| --- | --- | --- |
| `Error::PathContainsNul` | `Library::open` | The requested `OsStr` contains an interior NUL and cannot be passed as one C path. |
| `Error::LibraryOpen` | `Library::open` | `dlopen` returned null. The path display and copied `dlerror` detail are retained. |
| `Error::MissingSymbol` | `Library::symbol` | `dlsym` reported an error or returned a null address without an error. The first missing required symbol aborts `Api::load`. |
| `Error::Hsa` | `Api::check`, `check_status_only`, or `Runtime::close` | ROCr returned a nonzero `HsaStatus`; message text is optional as described above. |
| `Error::RuntimeClosed` | `Runtime::ensure_active` before a caller uses a closed runtime | The loader table may still be held by the `Runtime` value, but no further HSA operation is allowed through that value. |

`Runtime::open` intentionally reports an `hsa_init` failure with
`message: None` (`runtime.rs:39-48`). Calling `hsa_status_string` before a
successful initialization is not a valid ROCr operation, so this is not a
missing diagnostic and is not retried. `Runtime::close` marks `active = false`
before invoking `hsa_shut_down`; it reports a returned error through
`Api::check`. `Drop` performs the same one shutdown call but cannot return its
status (`runtime.rs:60-89`).

Errors from later callers retain this same `Hsa` conversion. For example,
allocation, executable, queue, and signal destruction report `Hsa` from an
explicit `close`, while their `Drop` paths discard the status after preserving
the safety invariant. An asynchronous negative signal is represented as
`Error::AsyncSignal`, and a queue callback is represented as
`Error::SessionPoisoned`; neither is confused with a loader failure.

## Ownership, lifetime, and threading

The ownership chain is deliberate:

```text
Library handle
  -> Api { _library, typed pointers }
     -> Arc<Api> in Runtime
        -> Arc<Api> in SignalPool, QueueCore, AllocationInner, ExecutableInner,
           SignalRecord, and deferred Pending keepalives
```

`Library` has explicit unsafe `Send` and `Sync` implementations
(`loader.rs:101-107`). Their stated assumptions are that POSIX dynamic-loader
handles support cross-thread use, symbol lookup state is immutable after
construction, and one `dlclose` occurs only after all `Arc<Api>` owners are
gone. `clear_dlerror` and `take_dlerror` operate on the calling thread's
diagnostic state, so a lookup's clear, `dlsym`, and copy are kept adjacent in
one call. `Api` has no mutable symbol table or lazy-resolution lock.

The `Arc<Api>` clones in execution objects are a second safety net for the
function-pointer lifetime during asynchronous work. The Rust types also carry
the stronger semantic boundary: `Discovery<'runtime>`,
`DiscoveredAgent<'runtime>`, and `Session<'runtime>` borrow `Runtime`;
`Allocation<'runtime>` and `Pending<'session, 'runtime>` carry runtime/session
`PhantomData`; `Queue` and `Executable` borrow their session. Therefore the
safe API requires child scopes to end before `Runtime::close` can be called.
The handle is not unloaded merely because a `Runtime` borrow is temporarily
unused.

The dynamic loader being `Sync` does not make every HSA object thread-safe.
`Session`, `Queue`, `Allocation`, `Executable`, `Pending`, and signal pools use
`Rc`, `RefCell`, or `Cell` and are intentionally host-thread-confined. The
asynchronous queue callback is the exception at the C boundary: it may run on a
ROCr thread, but it touches only the stable boxed context and atomic
`SharedFault` state. It invokes no loader diagnostic conversion, allocates no
`String`, and calls no `dlerror`; the synchronous discovery callbacks may grow
their own stack-borrowed collector vectors but never retain their callback data.

When an incomplete `Pending` token is dropped, the signal and every referenced
queue, executable, allocation, and dependency move to the session's deferred
retirement set (`hsa/src/execution.rs:1267-1278`). The set polls through the
same `Arc<Api>` only after a terminal signal value. If a signal remains
positive at final drop, the code retains the device-visible references and
emits a terminal leak diagnostic instead of destroying through an unloaded
library. This is why runtime shutdown must follow ordered session teardown.

## Caller responsibilities and operation phases

The loader provides pointers; callers provide the phase and object invariants.

### Discovery phase

`runtime::discover` first checks `Runtime::ensure_active`, then
`discovery::discover` reads system versions and timestamp frequency, collects
all agents, and expands each agent into exact identity, ISA, queue, and memory
pool descriptions (`hsa/src/discovery.rs:154-188`). The generic helpers
`system_info`, `agent_info`, `isa_info`, and `pool_info` use `MaybeUninit<T>`
and call `Api::check` before assuming ROCr initialized the output
(`discovery.rs:653-723`). Optional pool attributes accept only
`STATUS_ERROR_INVALID_ARGUMENT` as absence; all other statuses remain errors.

Discovery callbacks are synchronous and stack-borrowed. Their collectors
reserve and own returned vectors, so no pointer or callback data escapes the
`iterate_*` call. A callback panic is converted before `Api::check` handles the
ROCr status, preserving the C ABI no-unwind boundary.

### Session and queue realization

`DiscoveredAgent::into_session` rereads timestamp frequency, clones the
runtime's `Arc<Api>` into a `SignalPool`, and retains a `&Runtime`
(`hsa/src/session.rs:145-198`). `Session::create_queue` validates discovered
queue limits and producer kind before calling `hsa_queue_create`. It validates
the returned queue pointer, ring base, power-of-two size, kernel-dispatch bit,
and kind. An invalid result is destroyed immediately; callback context is
dropped only if that destroy succeeds.

`QueueCore` owns the raw queue pointer and the callback box. Its `Drop` path
calls `hsa_queue_destroy`, reports an explicit error through `Api::check` when
possible, and leaks callback context on ambiguous destruction. `HsaQueueIo`
uses the loaded queue-index pointers and the queue's validated C layout to
populate 64-byte packets, publish release headers and write indices, and ring
the doorbell with `signal_store_screlease`.

### Memory and executable realization

`allocate_from` validates the discovered pool, nonzero size, and optional
maximum before `hsa_amd_memory_pool_allocate`; the returned pointer must be
non-null. `grant_access` passes only handles from the same active runtime and
updates the host-side exact-access record after a successful HSA call
(`hsa/src/execution.rs:525-620`). `AllocationInner` retains the API and calls
`hsa_amd_memory_pool_free` once.

`Session::load_hsaco` rejects an empty byte slice, stores the bytes in an
`Arc<[u8]>`, creates a reader, creates an executable, loads the reader for the
exact agent, and freezes the executable (`execution.rs:974-1058`). Any failure
destroys already-created objects in dependency order. `ExecutableInner` keeps
the HSACO backing bytes until the reader is destroyed; if destruction fails it
leaks the backing bytes rather than allowing ROCr to read freed memory.

`Executable::kernel` resolves a NUL-free name, verifies the symbol kind, reads
the kernel object and metadata, and rejects a zero object or invalid alignment
(`execution.rs:845-943`). These are semantic execution errors, not dynamic
symbol errors. The name passed here is an HSA executable symbol name, distinct
from the loader's fixed `hsa_*` names.

### Live submission phase

`SignalPool` creates or reuses completion signals. Normal `Pending::poll` and
`wait`, dependency polling, deferred retirement, queue doorbells, and copy
submission all call the resolved signal pointers. `Session::copy_async` uses
`Api::check`; the prepared-copy path uses `check_status_only` so the finalized
loop does not allocate a status message (`execution.rs:1500-1649`). Queue
submission similarly uses the queue-index pointers and no loader lookup.

The live phase does not create a library, resolve a symbol, initialize ROCr,
or unload ROCr. Native preparation and HSACO realization happen before the
finalized loop; loader and runtime ownership therefore remain stable while the
loop submits work.

## Cleanup and failure ordering

There are two distinct teardown layers:

1. HSA object teardown calls the loaded destroy/free pointers. Explicit
   `close` methods return `Error::Hsa` or `Error::ResourceBusy`; `Drop` methods
   cannot report errors and preserve device safety by retaining or leaking
   ambiguous resources.
2. `Runtime::close` balances `hsa_init` with `hsa_shut_down`, then the final
   `Arc<Api>` drop runs `Library::drop` and `dlclose`.

The library cannot be unloaded while a `SignalRecord`, `QueueCore`,
`AllocationInner`, or `ExecutableInner` still owns an `Arc<Api>`. The API's
raw pointers are never copied into an owner without the handle owner or an
equivalent `Arc<Api>` keepalive. Conversely, `hsa_shut_down` is not a
replacement for `dlclose`: shutdown ends the initialized runtime reference,
while `dlclose` releases the code that supplied every function pointer.

No teardown path retries a failed HSA destroy, opens another library, or calls a
statically linked substitute. A failed queue destroy retains callback context;
a failed executable or reader destroy retains backing state; an unresolved
device signal is retained and reported. Those outcomes remain visible to the
caller rather than being hidden by a loader fallback.

## Scope limits

`loader.rs` deliberately stops at a typed function table and status adapter. It
does not:

* choose an AMD device, ISA, memory pool, queue size, or producer discipline;
* parse HSA attributes or turn raw handles into `AgentDescription` values;
* allocate host or device memory, load HSACO, create executable objects, or
  submit AQL itself;
* validate a kernel symbol, packet geometry, access set, or completion result;
* create a process-global runtime, keep a background thread, or expose raw
  `dlopen`/`dlsym` handles publicly;
* retry a failed open, skip a missing symbol, or substitute a different ABI.

Those responsibilities belong to `runtime.rs`, `discovery.rs`, `session.rs`,
and `execution.rs`. Keeping the boundary narrow makes each unsafe call site
state its own object, pointer, and phase invariant while the loader supplies
one stable ABI surface for all of them.

## Structural validation

`recipe-hsa` depends only on `libc` for this boundary (`hsa/Cargo.toml:8-12`);
the `live-hsa` feature gates the executable smoke example, not the loader
implementation. `cargo check -p recipe-hsa` checks that the private module,
ABI aliases, and all call sites remain structurally valid. A live
`cargo run --features live-hsa --example execute_smoke` additionally exercises
the complete open, initialize, discovery, allocation, copy, optional HSACO,
queue, signal, shutdown, and `dlclose` path on an installed ROCr runtime, but
hardware availability is outside this documentation module.
