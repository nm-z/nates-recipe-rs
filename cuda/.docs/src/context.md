# `recipe-cuda::context`

`context.rs` is the ownership and current-context boundary for the CUDA Driver
API. It turns one discovered `DeviceInfo` into one owned opaque `CUcontext`,
keeps that handle on its creating thread, and makes every context-sensitive
operation run with the exact context pushed on the current-context stack. The
module also owns the small, typed flag vocabulary used at creation time and
provides one live free/total-memory observation. It does not load the Driver,
discover devices, schedule work, allocate arenas, or implement a CUDA Runtime
API. Those responsibilities remain in [`driver.rs`](../../src/driver.rs),
[`discovery.rs`](../../src/discovery.rs), and [`runtime.rs`](../../src/runtime.rs).

The implementation is deliberately narrow. The only raw calls made directly
by this module are `cuCtxCreate_v2`, `cuCtxDestroy_v2`,
`cuCtxPushCurrent_v2`, `cuCtxPopCurrent_v2`, and `cuMemGetInfo_v2`
([`context.rs`](../../src/context.rs#L102-L179),
[`context.rs`](../../src/context.rs#L211-L225)). Modules, allocations, streams,
events, copies, and launches are implemented by `runtime.rs`, which uses this
module's guard and identity checks.

## Public surface at a glance

The module is private in `lib.rs`, but its principal types are re-exported by
the crate root. `MemoryInfo` is the public return type of
`Context::memory_info`, even though `lib.rs` does not list the type separately
in its `pub use` statement ([`lib.rs`](../../src/lib.rs#L16-L37)). The public
inputs and outputs are:

| Entry point | Input | Successful result | Context-related failures |
| --- | --- | --- | --- |
| `SchedulingPolicy::bits` | One policy value, used internally | The Driver scheduling bit | No fallible path; the mapping is exhaustive. |
| `ContextFlags::new` | A `SchedulingPolicy` | Flags with `map_host` and `local_memory_resize_to_max` disabled | None. |
| `ContextFlags::with_map_host` | Existing flags and a `bool` | Updated flags value | None. |
| `ContextFlags::with_local_memory_resize_to_max` | Existing flags and a `bool` | Updated flags value | None. |
| `ContextFlags::bits` | Existing flags | A complete `u32` Driver mask | None. |
| `ContextFlags::from_bits` | A raw `u32` mask | Validated typed flags | `InvalidContextFlags { bits }`. |
| `Context::create` | `&Driver`, `&DeviceInfo`, and typed flags | An open owner of one `CUcontext` | Invalid flags, Driver status, null-success handle, pop failure, or context-stack mismatch. |
| `Context::device` | A borrow of a live `Context` wrapper | Borrowed cloned `DeviceInfo` | None. The accessor does not inspect or mutate `raw`. |
| `Context::enter` | An open context | `ContextGuard` that has pushed this handle | `ContextClosed` or `DriverCall` from `cuCtxPushCurrent_v2`. |
| `Context::as_raw` | An open context | The opaque handle as `*mut c_void` | `ContextClosed`. |
| `Context::memory_info` | An open context | `MemoryInfo { free_bytes, total_bytes }` | `ContextClosed`, `DriverCall` from either stack call or `cuMemGetInfo_v2`, or `ContextStackMismatch`. |
| `Context::close` | Ownership of the context (`self`) | Unit after one explicit destroy attempt | `DriverCall` from `cuCtxDestroy_v2`; the private one-shot helper also guards an empty slot with `ContextClosed`. |
| `ContextGuard::leave` | An active guard (`self`) | Unit after one checked pop | `ContextClosed`, `DriverCall`, or `ContextStackMismatch`. `Drop` performs a best-effort retry only when the guard is still marked active. |

`Context` is not `Clone` or `Copy`. `ContextGuard` is a borrow of a
`Context`, and the guard dereferences to that context only while the guard is
alive. `MemoryInfo` is a copyable observation, not an allocation or a cache:

```rust
pub struct MemoryInfo {
    pub free_bytes: usize,
    pub total_bytes: usize,
}
```

Both counters are the values returned by the Driver at the instant of the
query. They are not promised to remain valid after the call
([`context.rs`](../../src/context.rs#L182-L187)).

## Driver and FFI prerequisites

### The handle types

`ffi.rs` keeps all CUDA C shapes crate-private. `CuContext` is an opaque
`*mut c_void`, `CuDevice` is a `c_int`, and Driver status values are `c_int`
([`ffi.rs`](../../src/ffi.rs#L10-L17)). The function-pointer declarations used by
this module are:

```text
CuCtxCreateV2  = unsafe extern "C" fn(*mut CuContext, c_uint, CuDevice) -> CuResult
CuCtxDestroyV2 = unsafe extern "C" fn(CuContext) -> CuResult
CuCtxPushCurrentV2 = unsafe extern "C" fn(CuContext) -> CuResult
CuCtxPopCurrentV2  = unsafe extern "C" fn(*mut CuContext) -> CuResult
CuMemGetInfoV2 = unsafe extern "C" fn(*mut usize, *mut usize) -> CuResult
```

The context aliases and signatures are resolved by `Api::load`, not linked at
compile time. All four context functions and `cuMemGetInfo_v2` are in
`REQUIRED_DRIVER_SYMBOLS`; a `Driver` cannot be constructed if any is missing
([`ffi.rs`](../../src/ffi.rs#L216-L249),
[`ffi.rs`](../../src/ffi.rs#L317-L410)). `Context` therefore never has an
optional context operation or a second implementation to use when a symbol is
absent.

### Driver lifetime and status conversion

`Driver::load` or `Driver::load_from_path` opens a dynamic library and stores
the resolved `Api` plus the library owner in an `Arc<DriverInner>`
([`driver.rs`](../../src/driver.rs#L12-L51)). Cloning the `Driver` stored in a
`Context` keeps both the function table and `libcuda.so` loaded for the whole
context lifetime. The context's thread-affinity marker is independent of the
Driver: a Driver is shareable, while a Context is intentionally not.

`Driver::check` is the status boundary used by `context.rs`. Zero
(`CUDA_SUCCESS`) returns `Ok(())`. Any other status becomes
`CudaError::DriverCall(DriverCallError)`, retaining the operation string and
numeric status and, when the optional `cuGetErrorName` and
`cuGetErrorString` symbols exist, their text
([`driver.rs`](../../src/driver.rs#L83-L132)). Context calls use this rich path,
so context setup, stack operations, and the memory query can report the Driver
operation as well as its raw failure status.

## Flag vocabulary and validation

The constants in `context.rs` are the R470-safe context bits. Their values are
fixed by the Driver ABI ([`context.rs`](../../src/context.rs#L11-L18)):

| Rust value | Driver bits | Meaning in `ContextFlags` |
| --- | ---: | --- |
| `SchedulingPolicy::Auto` | `0x00` | Driver-selected scheduling. This is the enum default. |
| `SchedulingPolicy::Spin` | `0x01` | Spin while waiting. |
| `SchedulingPolicy::Yield` | `0x02` | Yield while waiting. |
| `SchedulingPolicy::BlockingSync` | `0x04` | Blocking synchronization. |
| scheduling mask | `0x07` | The only bits inspected for the policy. |
| `map_host = true` | `0x08` | Enables the `CU_CTX_MAP_HOST` bit. |
| `local_memory_resize_to_max = true` | `0x10` | Enables `CU_CTX_LMEM_RESIZE_TO_MAX`. |

`ContextFlags` has exactly three public fields: the scheduling policy and the
two optional booleans. `new(policy)` initializes both optional bits to false;
the two `const` builder methods replace one boolean and return the whole value
([`context.rs`](../../src/context.rs#L40-L65)). `bits()` ORs those three encoded
choices and never emits a bit outside `KNOWN_CONTEXT_FLAGS`.

`from_bits(bits)` is the inverse validation boundary. It first rejects any bit
outside `0x1f`, then decodes `bits & 0x07`. Only `0x00`, `0x01`, `0x02`, and
`0x04` are valid policies; combinations such as `0x03`, `0x05`, `0x06`, or
`0x07` return `InvalidContextFlags`. The optional bits are converted to
booleans after the policy is accepted ([`context.rs`](../../src/context.rs#L76-L92)).

`Context::create` computes `flags.bits()` and immediately round-trips it
through `ContextFlags::from_bits` before entering the FFI. This is an internal
consistency check on the encoder as well as a guard against accidentally
passing an unsupported mask ([`context.rs`](../../src/context.rs#L102-L105)).
Because the public fields are typed, ordinary callers cannot construct an
unknown bit through the builder methods; `from_bits` remains available for
decoding externally stored masks.

The crate's default is therefore:

```text
ContextFlags::default()
  = scheduling: Auto, map_host: false, local_memory_resize_to_max: false
  = Driver mask 0x00000000
```

The bounded native benchmark deliberately chooses `Yield`, while production
reopening uses the default. Those choices are caller policy, not hidden
context behavior (see [Callers](#callers-and-lifetime-scopes)).

## Context ownership and creation

### Stored state

An open `Context` contains four fields
([`context.rs`](../../src/context.rs#L95-L100)):

| Field | Role and invariant |
| --- | --- |
| `driver: Driver` | A clone of the caller's shareable Driver. Its `Arc` keeps all resolved function pointers and the dynamic library alive. |
| `raw: Option<CuContext>` | `Some(handle)` means the wrapper owns one live Driver context. `None` means it has been explicitly consumed or has already been destroyed by an earlier cleanup path. |
| `device: DeviceInfo` | A cloned discovery snapshot used for identity and attributes. It is not a second Driver context and contains the crate-private raw `CUdevice` ordinal handle. |
| `_not_send_or_sync: PhantomData<Rc<()>>` | A zero-sized marker that makes the wrapper neither `Send` nor `Sync`, preserving CUDA current-context thread affinity at the Rust type boundary. |

The raw handle itself is intentionally absent from `Debug`. Debug output shows
the cloned device and whether `raw` is still present, then uses
`finish_non_exhaustive()` ([`context.rs`](../../src/context.rs#L197-L203)).

### Creation sequence

`Context::create(driver, device, flags)` is the sole constructor. Its exact
sequence is:

1. Encode and validate the typed flags.
2. Set a null `CuContext` output slot and call
   `driver.inner.api.ctx_create_v2(&mut raw, bits, device.handle)` through
   `Driver::check`.
3. Reject a successful call that left the output slot null with
   `InvalidDriverValue { operation: "cuCtxCreate_v2", detail: "success with a null context" }`.
4. Call `cuCtxPopCurrent_v2` immediately, because creation has placed the new
   context on the current thread's Driver context stack. Validate its status
   through `Driver::check` with operation text
   `"cuCtxPopCurrent_v2(after create)"`.
5. Compare the popped handle with the exact `raw` returned by creation. A
   different handle is `ContextStackMismatch`.
6. If the pop call fails or the handle differs, best-effort destroy the newly
   created `raw` directly through `ctx_destroy_v2`, ignore that cleanup status,
   and return the original pop or mismatch error.
7. On success, store the cloned Driver and `DeviceInfo`, `Some(raw)`, and the
   affinity marker in a new owner.

The implementation thus leaves the newly created context off the current
stack. Callers must use `enter()` for every later context-sensitive operation;
there is no ambient context retained by the `Context` object
([`context.rs`](../../src/context.rs#L103-L134)). A pre-existing current context is
not represented in Rust state, but the immediate pop is intended to restore
the stack to the state before construction. The code checks the exact popped
handle rather than assuming the Driver returned the one it created.

The input `DeviceInfo` must come from a successful `Driver::discover` snapshot
in normal production paths. Discovery obtains the raw `CUdevice` handle and
all identity/attribute fields before `Context::create` receives it
([`discovery.rs`](../../src/discovery.rs#L150-L189),
[`discovery.rs`](../../src/discovery.rs#L191-L295)). `Context::create` does not
re-probe the ordinal, UUID, PCI identity, or memory size.

### Creation failures and cleanup

There are three distinct validation layers:

* `InvalidContextFlags` is returned before any Driver context is created.
* `DriverCall` represents a nonzero status from `cuCtxCreate_v2` or the
  post-create `cuCtxPopCurrent_v2`. Rich optional Driver text is attached when
  the Driver exports the error-text functions.
* `InvalidDriverValue` represents a successful Driver status with a null
  context output. `ContextStackMismatch` represents a successful pop that
  returned another handle.

Every path after a non-null successful create owns `raw` locally. The two
post-create failure branches call `ctx_destroy_v2(raw)` before returning, so a
partially initialized context is not left for the caller. Cleanup errors are
deliberately ignored because the original pop or stack-integrity failure is the
reported result. No retry or alternate context constructor exists.

## Current-context guards and stack integrity

### Enter and leave

`Context::enter()` reads `raw`, returning `ContextClosed` if it is `None`, then
pushes exactly that handle with `cuCtxPushCurrent_v2`. A successful push yields
`ContextGuard { context: &self, active: true }`
([`context.rs`](../../src/context.rs#L136-L147)). The borrow ties the guard to the
owner and prevents the owner from being moved or consumed while the guard is
alive.

`ContextGuard::leave()` consumes the guard and calls its private `pop()`:

1. Read the expected handle from the borrowed `Context`; a closed owner gives
   `ContextClosed`.
2. Call `cuCtxPopCurrent_v2` through the same cloned Driver.
3. On Driver failure, return `DriverCall` while `active` remains true.
4. On success, set `active = false`, compare the returned handle with the
   expected one, and return `ContextStackMismatch` if they differ.
5. Return `Ok(())` only when the status succeeded and the exact handle was
   popped ([`context.rs`](../../src/context.rs#L206-L225)).

The `active` bit is changed only after a successful Driver pop. If a pop status
fails, `Drop` sees an active guard and makes one best-effort second `pop()`;
that cleanup result is ignored. If the status succeeds but the returned handle
is wrong, `active` is already false, so `Drop` does not pop again. This is
intentional error visibility and is why callers must preserve stack ownership
and use guards in LIFO order.

`ContextGuard` implements `Deref<Target = Context>`, allowing code that holds a
guard to pass it anywhere a `&Context` is accepted without exposing the raw
handle. Its `Drop` implementation only acts when `active` is true and never
panics or reports an error ([`context.rs`](../../src/context.rs#L228-L239)).
Nested guards are representable by Rust borrows, but the Driver stack must be
unwound in the same nesting order. An external Driver call that changes the
thread's current-context stack between `enter` and `leave` is observed as a
`ContextStackMismatch`; the module does not add a watcher or recovery path for
that invalid transition.

### `memory_info` is a complete scoped operation

`Context::memory_info()` is the one query implemented in this module. It:

1. Enters the exact context and retains the guard.
2. Initializes `free` and `total` to zero and calls
   `cuMemGetInfo_v2(&mut free, &mut total)` while the context is current.
3. Explicitly leaves the guard, checking the pop and the returned handle.
4. Propagates the query result and returns a `MemoryInfo` copy with the two
   `usize` counters ([`context.rs`](../../src/context.rs#L149-L166)).

The Driver's free and total values are not converted to a different unit and
are not cached. A failed query still attempts to leave the context before the
query error is returned. If leaving itself fails, that leave error is returned
at the `guard.leave()?` point before the stored query result is examined. This
ordering is part of the actual implementation and callers must treat a failed
stack restoration as a failure of the whole observation.

## Affinity, identity, and lifetime rules

### Thread affinity

`PhantomData<Rc<()>>` gives `Context` the auto-trait behavior of a non-thread-
safe marker without storing an `Rc` or changing the native representation. A
context, any guard, and every runtime object that borrows it therefore remain
on the creating thread. `Driver` remains cloneable and shareable because its
`Arc<DriverInner>` protects the loaded function table and library; sharing the
Driver does not share a current context.

The context's `DeviceInfo` is a value clone, so reading `device()` does not
borrow discovery's `Discovery` object. The raw `CUdevice` field in that clone
is used only by `Context::create`; later context-sensitive calls use the
`CUcontext` handle and current-context stack.

### Open/closed state

`raw: Option<CuContext>` is the single state bit. `Some` means operations may
be attempted; `None` means the native handle has been consumed. The state is
not independently synchronized because the type cannot cross threads and is
not shared through `Arc`.

* `as_raw()` and `enter()` return `ContextClosed` after the state becomes
  `None`.
* `close(self)` consumes the wrapper and calls `destroy()`, which takes the
  handle out before calling `cuCtxDestroy_v2`. The public consuming API cannot
  be called twice on the same value; its private `destroy` helper still guards
  an already-empty slot with `ContextClosed`.
* `Drop` takes any remaining raw handle and calls `cuCtxDestroy_v2` directly,
  ignoring the status. It does not push the context first, and it does not
  report an error because `Drop` cannot return one
  ([`context.rs`](../../src/context.rs#L172-L195)).

Taking the handle before the explicit destroy call makes cleanup one-shot even
when the Driver returns an error. In that error case the wrapper is already
closed and `Drop` does not issue a second destroy. This avoids duplicate native
ownership at the cost of retaining only the returned `DriverCall` status.

### Exact context identity

`same_context(left, right)` in `context.rs` is crate-private and uses
`core::ptr::eq(left, right)`, comparing the Rust `Context` object addresses,
not the raw handle, device ordinal, UUID, or compute capability
([`context.rs`](../../src/context.rs#L168-L170)). This is intentionally stricter
than device equality. Two separately created contexts for the same GPU cannot
be mixed, even if their `DeviceInfo` values are equal. Runtime methods call
this check before submitting a copy or launch and return
`ResourceContextMismatch` without touching the Driver when any participating
resource came from another `Context`.

## How `runtime.rs` consumes the context contract

`runtime.rs` centralizes the scoped push/pop operation in
`with_current(context, operation)`. It enters first, invokes the closure with
the context's crate-private `Driver`, then leaves the guard. If the operation
fails, it makes a best-effort leave and preserves the operation error. If the
operation succeeds, a leave failure is returned. This is the same stack
discipline as `memory_info`, with an explicit policy for preserving a native
operation failure ([`runtime.rs`](../../src/runtime.rs#L21-L35)).

All runtime owners carry a borrow of the same context lifetime. The following
table names the direct context boundary for each owner and operation family:

| Runtime owner or operation | Context use | Lifetime/identity consequence |
| --- | --- | --- |
| `Module::load_cubin` | `with_current` around `cuModuleLoadData`; rejects non-ELF input and null success | `Module<'ctx>` borrows the context for its entire native module lifetime. |
| `Module::function` | `with_current` around `cuModuleGetFunction`; rejects empty or NUL-containing names and null success | `Function<'module, 'ctx>` borrows its module and therefore its context. |
| `Module::unload` and `Drop` | `with_current` around `cuModuleUnload` | Module cleanup cannot outlive the context; `Drop` ignores status. |
| `DeviceBuffer::allocate` / `free` | `with_current` around `cuMemAlloc_v2` / `cuMemFree_v2` | Nonzero allocation and one-shot pointer state; `DeviceBuffer<'ctx>` keeps the context borrowed. |
| `PinnedHostBuffer::allocate` / `free` | `with_current` around `cuMemHostAlloc` / `cuMemFreeHost` | Nonzero, zero-initialized host storage; the pinned owner borrows the context. |
| `Stream::create_nonblocking`, `poll_idle`, and destroy | `with_current` around stream create/query/destroy | Stream operations use the context that created the stream. |
| Event creation/query/destroy | `with_current` around event create/query/destroy | Completion events remain tied to the creating context. |
| Event-backed copies | `same_context` for stream, source, destination, and completion event, then `with_current` around copy plus `cuEventRecord` | `Pending<'op, 'ctx>` borrows the operation resources and context until terminal completion. |
| Enqueue-only copies | `same_context` for both allocations, then `with_current` around the copy | Caller must retain allocations and stream until `poll_idle` reports complete. |
| Event-backed kernel launch | `same_context` for function, completion event, and every keepalive buffer, then `with_current` around launch plus event record | `Pending` keeps the operation borrow live; the context and module must remain open. |
| Enqueue-only launch | `same_context` for function and keepalive buffers, then `with_current` around launch | Caller must retain function, module, buffers, stream, and context until idle. |
| `Pending::poll` / `wait` / `recycle_event` | Event query runs in `with_current(event.context, ...)` | A timeout returns the same token; CUDA cancellation is not invented. |

The context guard is therefore the only way `runtime.rs` reaches a
context-sensitive Driver symbol after construction. Resource borrows make it
impossible to drop the context while a `Module`, `DeviceBuffer`, pinned
buffer, `Stream`, `Event`, `Function`, or pending operation is still in scope.
The runtime's unsafe methods add the remaining asynchronous requirement: the
caller must retain and drive event-backed `Pending` tokens, or retain all
enqueue-only resources until `poll_idle` is terminal
([`runtime.rs`](../../src/runtime.rs#L405-L673),
[`runtime.rs`](../../src/runtime.rs#L749-L821)).

`runtime.rs` uses the status-only Driver conversion for post-realization
submission and poll paths. Context creation, destruction, stack operations,
and `memory_info` use the rich `Driver::check` path directly from
`context.rs`; this keeps setup diagnostics detailed while keeping live-loop
submission allocation-free.

## Callers and lifetime scopes

`rg` over the workspace finds two context constructors and three classes of
consumers. No scheduler or declaration creates a context directly. The
production path always reopens a measured device, creates the context in the
native probe layer, and lends a borrow to preparation or execution.

### Native probe: bounded benchmark

`native-probe::cuda::CudaBackend::open` selects the configured Driver library,
calls `Driver::load_from_path`, and runs `Driver::discover`. The benchmark then
matches the exact previously described device and creates one context with
`ContextFlags::new(SchedulingPolicy::Yield)`
([`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L86-L106),
[`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L236-L252)).

`benchmark_device` keeps that context alive while it creates one nonblocking
stream, two pinned host buffers, two device buffers, completion events,
modules, functions, and pending tokens. Every allocation and submission passes
`&context`; all ranges are checked by `runtime.rs`, and the benchmark's
`complete_cuda` loop drives each event to terminal completion before the local
resource owners unwind ([`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L248-L342),
[`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L400-L445)). The
calculation helper reads `context.device()` for compute capability and maximum
threads, then loads and launches a cubin in that same context. Its verification
download also uses the same context borrow
([`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L345-L446),
[`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L466-L518)).

The benchmark's timeout path does not drop a live pending token. CUDA has no
cancellation primitive in this boundary, so `complete_cuda` continues polling
the same token with a capped cleanup delay and only returns the timeout error
after terminal completion. This protects the context-borrowed buffers from
being destroyed while the Driver still accesses them.

### Native probe: preparation/execution bindings

`native-probe::bindings::realize_cuda` reopens the current Driver and exact
discovery, matches every measured GPU key, derives a `DeploymentIdentity`, and
creates one `Context` per reopened device with `ContextFlags::default()`
(`Auto`, both optional bits disabled)
([`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L268-L316)).
Each context is stored in a `RealizedCuda` value. A `CudaBinding` then borrows
that context and carries the device id, deployment identity, submission-queue
ceiling, and display-connector count.

`with_native_execution_bindings` passes those borrowed bindings to one
higher-ranked callback. The callback type is
`for<'cuda, 'hsa> FnOnce(NativeExecutionBindings<'cuda, 'hsa>)`, so its result
cannot contain a borrow of the context. After the callback returns, the
binding vector and its `RealizedCuda` owners are dropped in the enclosing
scope, destroying contexts only after all resources borrowed by the callback
have gone away ([`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L22-L46),
[`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L120-L234)).
There is no ordinal, product-name, capacity, or performance fallback that
would pair a context with a different discovered device.

### Root native preparation

`src/native_prepare.rs` wraps the same higher-ranked binding callback in
`with_native_preparation`. It constructs a `NativePreparationScope` containing
the context-borrowing `NativeExecutionBindings`, host plan, and target plan,
then invokes one callback. The scope documentation explicitly states that CUDA
contexts and HSA sessions cannot escape this callback
([`src/native_prepare.rs`](../../../src/native_prepare.rs#L212-L246),
[`src/native_prepare.rs`](../../../src/native_prepare.rs#L305-L365)). This is the
root-library boundary used to construct a `LocalCandidateFactory`, native
executor driver, and candidate realizer without storing dynamic handles in
Recipe declarations.

### Native executor: binding and resources

`native-executor::CudaBinding<'context>` stores `&'context Context`. Its
`available_bytes()` method calls `Context::memory_info()` immediately before
resource realization and converts `free_bytes` to Recipe's `ByteCount`; a
counter that does not fit `u64` becomes an executor `ArenaMismatch`
([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L37-L92)).
`validate_binding` separately checks that the context's UUID and compute
capability match the binding's deployment identity before native resources are
created ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1633-L1646)).

`CudaResources::realize` receives bindings rather than creating contexts. For
each selected device, `realize_device` uses the borrowed context to create:

* one nonblocking `Stream` for every finalized queue slot;
* one timing-disabled completion `Event` for every finalized completion slot;
* pinned staging storage sized by the immutable resource manifest;
* optional device scratch storage;
* one `Module` per distinct cubin digest and `Function` values for logical
  entries, after artifact identity and ABI validation;
* four-byte pinned metric buffers and preallocated exit vectors.

All of those owners carry the same `'context` lifetime. Device arenas are
allocated later with `DeviceBuffer::allocate(resources.context, bytes)`, and
the executor's `CudaPending` tokens retain the context and operation resources
through terminal polling ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L186-L198),
[`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L348-L435),
[`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1665-L1903)).

For event-backed submissions, the executor converts
`Pending<'operation, 'context>` to `Pending<'static, 'context>` at one reviewed
adapter boundary. Only the operation borrow is erased; the context lifetime,
completion event, and resource-table ownership remain. The adapter's safety
argument is that `CudaResources` tracks active queue and completion-slot
ownership, refuses teardown until tokens are terminal, and deliberately leaks
an abandoned active native token rather than letting CUDA access a freed
context-borrowed resource ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L2188-L2201),
[`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1467-L1483)).

The finalized backend does not retain a global current context. Each submit,
poll, allocation, query, and teardown operation enters the context that owns
the selected resource. Same-context checks reject cross-context device-device
routes before `cuMemcpyDtoDAsync_v2` or `cuLaunchKernel` is reached. The
executor's contract layer therefore maps one finalized device to one exact
context and one set of borrowed native owners.

`destroy_devices` enforces the reverse lifetime order. It first requires every
stream to report `CompletionStatus::Complete`, then requires every completion
slot to be available, destroys events, destroys streams, drops function
holders, unloads modules, frees metric buffers and pinned staging, and finally
frees optional device scratch ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L2203-L2245)).
Only after this resource tree is gone may the outer native binding scope drop
the `Context` owners. An active stream or checked-out event is a teardown
failure, not a reason to destroy the context early.

### Staged cross-backend bridge

`native-executor::StagedCrossBackend` obtains the exact context from a
`CudaBinding` for each CUDA endpoint. During pre-realization it allocates a
pinned staging buffer, one nonblocking stream, and one completion event for
that endpoint ([`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L232-L333)).
At submission, a CUDA source leg uses `Stream::copy_d2h` from the executor arena
to its staging buffer; a CUDA destination leg uses `Stream::copy_h2d` from
staging into the destination arena. The event-backed `Pending` token is kept in
the bridge state and polled until terminal completion before the event is
recycled ([`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L1508-L1612)).
The bridge converts any `CudaError` into `StagedBridgeError::Cuda` without
changing the Driver status or adding a retry
([`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L157-L163)).

## Error model for context operations

`Result<T>` is the crate-wide alias for `core::result::Result<T, CudaError>`
([`error.rs`](../../src/error.rs#L1-L4)). The variants that can arise on the
context boundary are:

| Error | Where it arises | Meaning and side effect |
| --- | --- | --- |
| `InvalidContextFlags { bits }` | `ContextFlags::from_bits` and the constructor round-trip | Unknown bits or an invalid scheduling-bit combination. No native context has been created. |
| `DriverCall(DriverCallError)` | Any checked Driver call in create, enter, leave, memory query, or destroy | Nonzero status. The operation string and numeric status are retained; optional name/description text may be present. |
| `InvalidDriverValue { operation, detail }` | Null-success context from `cuCtxCreate_v2` | Driver claimed success but returned no handle. Creation returns without an owner. |
| `ContextStackMismatch` | Post-create pop or guard pop | Driver returned a handle different from the exact one created or pushed. Post-create creation destroys its raw handle first; a guard with a successful wrong-handle pop is already inactive. |
| `ContextClosed` | `enter`, `as_raw`, guard `pop`, or `close` after ownership has been consumed | The wrapper's `raw` slot is `None`; no Driver operation is attempted. |
| `ResourceContextMismatch { operation }` | `runtime.rs::same_context` before copy or launch | A stream, event, function, or buffer belongs to another Rust `Context` object. The Driver is not called. |

The full `CudaError` enum is non-exhaustive, so callers must preserve a
wildcard when matching it. Its `Display` text includes the exact operation for
Driver-value, input, and resource errors; `ContextStackMismatch` reports that
the Driver returned a context different from the one Recipe pushed, and
`ContextClosed` reports that the owner was already consumed
([`error.rs`](../../src/error.rs#L33-L71),
[`error.rs`](../../src/error.rs#L73-L128)).

When a context is dropped, `cuCtxDestroy_v2` failure cannot be surfaced. The
explicit `close` path should be used by code that must observe that status; it
still consumes the owner before the call, so a failed explicit destroy is not
retried by `Drop`. This one-shot behavior is also used by runtime objects and
is required for deterministic ownership rather than best-effort duplicate
cleanup.

## Invariants enforced at this boundary

The source and its callers rely on the following concrete invariants:

1. **One typed flag mask.** Only the four supported scheduling values and the
   two documented optional bits can reach `cuCtxCreate_v2`.
2. **Required FFI.** Context creation, destruction, push/pop, and memory info
   are required Driver symbols resolved before a `Driver` exists.
3. **Non-null success.** A successful context creation must write a non-null
   handle.
4. **Creation-stack restoration.** The handle returned by the immediate pop
   must equal the handle returned by creation.
5. **Scoped current context.** Every post-creation context-sensitive runtime
   call runs only between one successful push and one checked pop of the exact
   context. Destruction is the one-shot direct `cuCtxDestroy_v2` path.
6. **Exact stack ownership.** Guard leave compares the popped handle and marks
   the guard inactive only after the Driver pop succeeded.
7. **Thread affinity.** `Context` cannot be moved or shared across threads;
   `Driver` sharing does not weaken this rule.
8. **Borrowed resource lifetime.** Every module, allocation, stream, event,
   function, and pending operation borrowing a context must end before the
   context owner is dropped.
9. **Object identity.** Runtime same-context checks compare the exact Context
   object, not merely its device metadata or raw pointer value.
10. **No implicit cancellation.** A timeout retains the pending token and its
    context-borrowed resources until terminal completion.
11. **One-shot cleanup.** `close` and `Drop` take the raw slot before destroy;
    an owner never performs a second destroy after a failed first attempt.
12. **Observed memory only.** `MemoryInfo` is one live Driver observation and
    is not used as an implicit reservation or cache.

These invariants fit the larger Recipe lifecycle: discovery, context creation,
allocation, module loading, and event/queue creation happen before the
immutable execution loop; loop operations consume already-realized resources
and re-enter their owning context for each native call. The context module
does not add scheduling or model semantics to that lifecycle.

## Source map and verification

The implementation references most useful for a context review are:

| Concern | Source |
| --- | --- |
| Flag constants, typed flags, and validation | [`cuda/src/context.rs`](../../src/context.rs#L11-L93) |
| Context fields, creation, accessors, memory query, and explicit close | [`cuda/src/context.rs`](../../src/context.rs#L95-L187) |
| Drop, Debug, guard push/pop, and dereference | [`cuda/src/context.rs`](../../src/context.rs#L189-L240) |
| Opaque aliases and context Driver function pointers | [`cuda/src/ffi.rs`](../../src/ffi.rs#L10-L89) |
| Required symbol resolution and capability inventory | [`cuda/src/ffi.rs`](../../src/ffi.rs#L216-L414) |
| Driver `Arc` lifetime and rich status conversion | [`cuda/src/driver.rs`](../../src/driver.rs#L12-L132) |
| Device snapshot that supplies `DeviceInfo::handle` | [`cuda/src/discovery.rs`](../../src/discovery.rs#L124-L189) |
| Scoped runtime push/pop and resource context identity | [`cuda/src/runtime.rs`](../../src/runtime.rs#L21-L43) |
| Runtime context-borrowing owners and asynchronous calls | [`cuda/src/runtime.rs`](../../src/runtime.rs#L45-L133), [`cuda/src/runtime.rs`](../../src/runtime.rs#L145-L327), [`cuda/src/runtime.rs`](../../src/runtime.rs#L370-L673), [`cuda/src/runtime.rs`](../../src/runtime.rs#L699-L839) |
| Benchmark context and event completion | [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L236-L342), [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L466-L518) |
| Exact reopened contexts and higher-ranked binding scope | [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L22-L46), [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L120-L316) |
| Executor binding, resource realization, and teardown | [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L37-L92), [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1665-L1903), [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L2203-L2245) |
| CUDA half of staged cross-backend transfer | [`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L232-L333), [`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs#L1508-L1635) |
| Root preparation callback lifetime | [`src/native_prepare.rs`](../../../src/native_prepare.rs#L212-L246), [`src/native_prepare.rs`](../../../src/native_prepare.rs#L305-L365) |

`cargo check -p recipe-cuda` verifies the Rust and FFI shapes. It does not
prove Driver availability, current-context stack behavior, asynchronous
completion, or hardware correctness. Those claims require the real
`recipe probe` path and a complete CUDA workload on the target Linux 64-bit
Driver deployment, where the production callers above exercise context setup,
resource use, polling, and ordered teardown.
