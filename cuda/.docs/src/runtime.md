# CUDA runtime boundary

[`src/runtime.rs`](../../src/runtime.rs) is the small public wrapper
around Recipe's asynchronous CUDA Driver operations. It owns the Rust handles
for modules, kernel functions, device memory, pinned host memory, streams, and
completion events. It does not choose devices, compile kernels, schedule work,
or provide a second execution policy. The public root re-exports these values
from [`src/lib.rs`](../../src/lib.rs#L35-L38).

The runtime deliberately exposes only Driver API operations that Recipe uses:

| Runtime area | Driver operations | Runtime responsibility |
| --- | --- | --- |
| Module and function | `cuModuleLoadData`, `cuModuleGetFunction`, `cuModuleUnload` | Validate an ELF cubin, keep the module alive while its function borrows it, and unload exactly once. |
| Device memory | `cuMemAlloc_v2`, `cuMemFree_v2` | Own a nonzero allocation and check every byte range before forming an offset pointer. |
| Pinned host memory | `cuMemHostAlloc`, `cuMemFreeHost` | Own a nonzero page-locked host allocation, zero it on creation, and expose byte slices for staging. |
| Stream | `cuStreamCreate`, `cuStreamQuery`, `cuStreamDestroy_v2` | Create one nonblocking stream, query idle/completion, and close it. |
| Events | `cuEventCreate`, `cuEventRecord`, `cuEventQuery`, `cuEventDestroy_v2` | Create timing-disabled completion events and move them into `Pending` submissions. |
| Copies | `cuMemcpyHtoDAsync_v2`, `cuMemcpyDtoHAsync_v2`, `cuMemcpyDtoDAsync_v2` | Enqueue checked asynchronous copies, optionally recording an event after the copy. |
| Kernel launch | `cuLaunchKernel` | Pass the caller's ABI-shaped argument pointer array, launch with checked dimensions, and optionally record an event. |

There is no synchronous copy, global synchronize, stream-wait-event, or
cancellation wrapper. Completion is observed only with a stream query or an
event query. The absence of a cancellation primitive is why callers must keep
resources alive after a timeout and continue polling the same operation.

## Shared invariants

### Current context

Every Driver call in this module is made through `with_current` at
[`runtime.rs#L21-L35`](../../src/runtime.rs#L21-L35). The helper:

1. calls `Context::enter`, which pushes the context with
   `cuCtxPushCurrent_v2`;
2. invokes the operation closure with the context's loaded `Driver`;
3. pops the context with `ContextGuard::leave`.

When the operation fails, the helper attempts the pop but preserves the
operation error. When the operation succeeds, a pop failure is returned. A
context-stack mismatch therefore remains visible, while a failed operation is
not replaced by a secondary leave failure. If a context is already closed,
`enter` returns `CudaError::ContextClosed` before the Driver call.

Resource compatibility is checked by `same_context` at
[`runtime.rs#L37-L43`](../../src/runtime.rs#L37-L43). `Context::same_context`
uses pointer identity, so two `Context` values that happen to describe the
same CUDA device are still different runtime contexts unless all resources
borrow the exact same `Context` object. Copies and launches reject a mixed
context before entering the Driver.

### Status conversion

Creation, lookup, loading, and explicit destruction call `Driver::check`, which
preserves the numeric Driver status and asks optional error-name and
error-string symbols for rich text. Asynchronous submission and polling call
`Driver::check_status_only`, an allocation-free conversion used after
realization. `query_status` maps only `CUDA_SUCCESS` to
`CompletionStatus::Complete` and `CUDA_ERROR_NOT_READY` to
`CompletionStatus::Pending`; every other status is a `CudaError::DriverCall`.

Successful Driver calls are also checked for impossible null handles. A null
module, function, allocation pointer, stream, or event after a success status
becomes `CudaError::InvalidDriverValue`, rather than being stored as a usable
opaque handle.

### Handle state and cleanup

Each native owner stores its raw handle in an `Option`. Explicit destruction
takes the handle before calling the Driver, so a failed destruction is not
retried by a later `Drop`. `Drop` is a best-effort fallback: it enters the
borrowed context, calls the corresponding destroy operation, and discards any
error. If the context is already closed, that cleanup cannot re-enter it and
the error is likewise ignored. Callers that need an observable failure must
use the consuming `free`, `destroy`, or `unload` method while the context is
open.

The runtime types borrow their `Context` and are therefore tied to that
context's lifetime. `Function` additionally borrows its `Module`; a function
handle cannot normally outlive the module that resolved it.

## Object and lifetime map

| Type | Stored state | Creation | Explicit close | Borrow/lifetime rule |
| --- | --- | --- | --- | --- |
| `Module<'ctx>` | `&'ctx Context`, optional `CUmodule` | `Module::load_cubin` | `unload` | Owns the module image. `Function` borrows it. |
| `Function<'module, 'ctx>` | `&'module Module`, `CUfunction`, copied name | `Module::function` | None | Raw function is valid only while its module remains loaded. |
| `DeviceBuffer<'ctx>` | context, optional `CUdeviceptr`, byte length | `allocate` | `free` | Nonzero allocation; range checks produce offset pointers. |
| `PinnedHostBuffer<'ctx>` | context, optional `NonNull<u8>`, byte length | `allocate` | `free` | Nonzero zero-initialized host allocation; async H2D/D2H requires it. |
| `Stream<'ctx>` | context, optional `CUstream` | `create_nonblocking` | `destroy` | Owns one nonblocking queue and must stay live through queued work. |
| `Event<'ctx>` | context, optional `CUevent` | `create_completion` | `destroy` | Timing is disabled. Event-backed operations consume it. |
| `Pending<'op, 'ctx>` | optional event, completion bit, phantom `&'op` | Event-backed stream method | `poll`, `wait`, `recycle_event` | Must be driven to terminal completion before borrowed resources are released. |

The `'op` lifetime on `Pending` is intentionally represented by
`PhantomData<&'op ()>` rather than stored references. The unsafe stream methods
use that lifetime to make the caller retain the stream, buffers, function,
module, and argument storage. The implementation still requires the caller to
uphold the contract when raw pointers or lifetime erasure are used.

## Modules and kernel functions

### Loading a cubin

`Module::load_cubin(context, cubin)` at
[`runtime.rs#L50-L75`](../../src/runtime.rs#L50-L75) accepts only bytes
whose first four bytes are the ELF magic `0x7f ELF`. PTX and fat binaries are
rejected as `InvalidInput` before any Driver call. The bytes are passed to
`cuModuleLoadData` while the context is current. A non-null `CUmodule` is
stored with the same context borrow.

The byte slice itself is not retained after loading. CUDA has copied or
otherwise consumed the image according to the Driver contract by the time the
call returns; subsequent operation lifetime is represented by the module
handle, not by the source slice.

### Resolving a function

`Module::function(name)` first checks that the module is open, rejects an empty
name, and rejects a name containing an interior NUL because the Driver name is
passed as a `CString`. `cuModuleGetFunction` is called in the module context,
and a null success result is rejected. The returned `Function` stores a copied
Rust name for diagnostics and a raw `CUfunction` borrowed from the module.

`Function` has no independent destructor. Unloading the module invalidates its
function handles, so normal Rust borrowing prevents that order. The native
executor stores functions in `LoadedArtifact` values and keeps their modules in
stable `Box<Module>` values. It drops those function holders before unloading
the modules during teardown.

## Device and pinned host buffers

### Device buffers

`DeviceBuffer::allocate` at
[`runtime.rs#L151-L177`](../../src/runtime.rs#L151-L177) rejects zero
bytes, calls `cuMemAlloc_v2`, and rejects a null success pointer. `len()`
returns the requested byte length. `is_empty()` is always `false` because a
zero-byte allocation cannot exist. `device_ptr()` returns the base pointer or
`ContextClosed` after explicit destruction.

All asynchronous copies use `checked_pointer` at
[`runtime.rs#L187-L214`](../../src/runtime.rs#L187-L214). It checks
`offset + bytes` with overflow detection, requires the range to end at or
before `len`, converts the offset to `u64`, and checks pointer addition for
overflow. A zero-byte range is permitted when its offset is within the
allocation, including the one-past-end offset. The raw pointer is never
formed for an out-of-range request.

### Pinned host buffers

`PinnedHostBuffer::allocate` at
[`runtime.rs#L244-L272`](../../src/runtime.rs#L244-L272) has the same
nonzero rule. It calls `cuMemHostAlloc` with no allocation flags, rejects a
null pointer, and writes zero bytes across the allocation before returning.
`as_slice` and `as_mut_slice` expose the entire allocation through an unsafe
raw slice conversion after asserting that the owner is still open. Calling
those accessors after `free` is a programmer error and panics at the internal
`expect`, rather than returning a CUDA error.

Its range helper mirrors the device helper and returns a `NonNull<u8>` at the
requested offset. Pinned host storage is the source or destination type used
by the asynchronous host/device methods. The wrapper does not offer a regular
heap buffer overload, so callers cannot accidentally pass pageable memory to
these methods.

## Launch geometry and argument ABI

`Dim3::new(x, y, z)` at
[`runtime.rs#L329-L346`](../../src/runtime.rs#L329-L346) rejects any zero
axis. `LaunchConfig::new(grid, block)` stores both dimensions and sets dynamic
shared memory to zero. `with_dynamic_shared_memory` replaces that byte count.
No other hardware limit is checked here. The immutable plan and artifact
validation performed by higher layers are responsible for the cubin ABI,
workgroup limits, and element count.

The event-backed `Stream::launch` API receives a mutable slice of host-side
argument pointers. An empty slice is translated to a null CUDA parameter
pointer; otherwise the slice base is passed directly to `cuLaunchKernel`.
CUDA copies the argument values before the Driver call returns, but any device
allocation referenced by those values remains in use asynchronously. Every
such allocation must be listed in `keepalive`, which is context-checked and
whose references are tied to the returned `Pending` lifetime.

The caller must ensure that each parameter pointer is correctly aligned, that
its pointee type and size match the inspected cubin entry ABI, and that the
grid, block, and dynamic shared-memory values are valid for that kernel. The
Driver cannot validate Rust's host-side pointer typing.

## Streams

### Creation and idle polling

`Stream::create_nonblocking` at
[`runtime.rs#L375-L394`](../../src/runtime.rs#L375-L394) calls
`cuStreamCreate` with `CU_STREAM_NON_BLOCKING = 1` and rejects a null stream.
`poll_idle` at [`runtime.rs#L396-L403`](../../src/runtime.rs#L396-L403)
calls `cuStreamQuery` and returns the shared `CompletionStatus` mapping. It
does not synchronize, wait, or consume any queued work.

### Event-backed operations

The following methods enqueue the operation, record the supplied event on the
same stream after the operation, and return a `Pending` that owns that event:

| Method | Source and destination | Driver sequence | Borrowed resources retained by `'op` |
| --- | --- | --- | --- |
| `copy_h2d` | pinned host slice to device range | `cuMemcpyHtoDAsync_v2`, then `cuEventRecord` | stream, device destination, pinned source, and event context |
| `copy_d2h` | device range to mutable pinned host slice | `cuMemcpyDtoHAsync_v2`, then `cuEventRecord` | stream, device source, mutable host destination, and event context |
| `copy_d2d` | device range to device range | `cuMemcpyDtoDAsync_v2`, then `cuEventRecord` | stream, both device allocations, and event context |
| `launch` | cubin function plus ABI parameter pointers | `cuLaunchKernel`, then `cuEventRecord` | stream, function/module, parameter storage, keepalive buffers, and event context |

Each method first checks every involved context with `same_context`, then
checks the stream and event handles, then checks all byte ranges before
entering the Driver. The event is moved into the returned `Pending` only after
both Driver calls succeed. If either call fails, the error is returned and no
pending token is produced; the runtime does not attempt a rollback or submit a
replacement operation.

The methods are `unsafe` because the type system cannot prove CUDA's
asynchronous access contract. The caller must retain the returned token and
drive it to a terminal status before destroying or mutating any resource that
CUDA may still access. The Rust borrow carried by `Pending` is the intended
normal path for expressing that requirement.

### Enqueue-only operations

The following methods enqueue work without recording an event and return only
the Driver submission result:

| Method | Driver call | Completion obligation |
| --- | --- | --- |
| `enqueue_d2h` | `cuMemcpyDtoHAsync_v2` | Keep source, mutable destination, and stream live until `poll_idle` reports `Complete`. |
| `enqueue_d2d` | `cuMemcpyDtoDAsync_v2` | Keep both allocations and the stream live until `poll_idle` reports `Complete`. |
| `enqueue_launch` | `cuLaunchKernel` | Keep function/module, parameter storage, keepalive allocations, and stream live until `poll_idle` reports `Complete`. |

These methods perform the same context and range checks as their event-backed
counterparts. They do not provide a `Pending`, so the caller must provide the
larger lifetime discipline. Recipe's native executor uses this form for
calculation launches, internal D2D transfers, and metric readbacks, while its
resource object owns the arenas and pre-realized functions until stream polling
finishes.

There is intentionally no enqueue-only H2D method. Host-to-device admissions
and staged destination copies use an event so their pinned source and
completion slot have an explicit token.

## Events and completion tokens

### Events

`Event::create_completion` at
[`runtime.rs#L704-L723`](../../src/runtime.rs#L704-L723) calls
`cuEventCreate` with `CU_EVENT_DISABLE_TIMING = 2`. These events are completion
markers, not timing measurements. `destroy` consumes the owner and calls
`cuEventDestroy_v2`; `Drop` is the best-effort fallback.

An event-backed stream method consumes an `Event` by value. This prevents the
caller from recording the same event in two submissions at once. The event is
returned only by `Pending::wait` after complete or by
`Pending::recycle_event` after a terminal poll. Until then it belongs to the
submission.

### Status and state machine

`CompletionStatus` has exactly two states:

```text
Pending::new(event)
        |
        v
Pending --poll--> Pending       (cuEventQuery = CUDA_ERROR_NOT_READY)
        |
        +---------> Complete     (cuEventQuery = CUDA_SUCCESS)
```

After the first complete query, `Pending` sets an internal `complete` bit.
Later `poll` calls return `Complete` without another Driver query. A Driver
error leaves the result as an error; no status is invented for unknown return
codes.

`Pending` is marked `must_use` because dropping a still-pending token drops its
event and releases the conceptual operation borrow. Direct callers must not do
that. The token does not cancel CUDA work.

### Waiting with a deadline

`Pending::wait(timeout)` at
[`runtime.rs#L788-L805`](../../src/runtime.rs#L788-L805) computes a checked
`Instant` deadline. An overflow is `InvalidInput`. It polls first, returns
`WaitOutcome::Complete(event)` as soon as the event is terminal, and otherwise
returns `WaitOutcome::TimedOut(self)` once the deadline has elapsed. A zero
timeout therefore still performs one poll. `yield_now` provides cooperative
poll spacing; there is no sleep policy or cancellation path in this layer.

The timed-out branch returns the same `Pending` token, preserving its event and
phantom operation lifetime. Callers must continue polling it or otherwise keep
all referenced resources alive. `WaitOutcome` is also `must_use` for this
reason.

### Recycling an event

`Pending::recycle_event` at
[`runtime.rs#L807-L821`](../../src/runtime.rs#L807-L821) is for a token
already known to be terminal. It polls once, returns the owned event on
`Complete`, and returns `InvalidInput` if the event is still pending. Calling
it while pending consumes and drops the token, so callers must never use it as
a nonblocking probe. The native probe and the staged bridge use it to return
pre-created events to their reusable completion slots.

## Runtime callers

The runtime has three production callers. No test or alternate implementation
enters an internal runtime function.

### Native probe

[`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs) opens a real Driver
and context, creates one nonblocking stream, allocates pinned host and device
buffers, and exercises all three event-backed copy directions. Each operation
creates an event, calls `copy_h2d`, `copy_d2h`, or `copy_d2d`, then passes the
`Pending` to `complete_cuda`.

`complete_cuda` repeatedly calls `Pending::poll` with an exponential backoff.
On terminal completion it calls `recycle_event` and drops the returned event.
If the measurement deadline expires, CUDA cannot cancel the submission, so the
probe keeps polling that same token with a capped cleanup delay until it
completes. Only then does it return the benchmark timeout error. This keeps the
live pinned and device buffers valid until the GPU has stopped using them.

The probe then downloads the D2D destination through `copy_d2h` and compares
host bytes independently. For the calculation benchmark it builds and
inspects a real cubin, loads one `Module`, resolves its `Function`, builds a
`LaunchConfig`, and calls event-backed `launch` with `[input, output]` in
`keepalive`. The same completion helper drives the kernel, followed by a
verified device-to-host copy. Module, buffers, stream, and context are dropped
only after those terminal tokens have been recycled.

### Native CUDA executor

[`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs) realizes all
objects before the finalized run:

* one nonblocking `Stream` for each planned queue;
* one timing-disabled `Event` for each planned completion slot;
* one pinned staging buffer per CUDA device, sized by the resource manifest;
* optional scratch and per-arena `DeviceBuffer` allocations;
* one `Module` per distinct cubin digest and one borrowed `Function` per
  logical artifact entry;
* one four-byte pinned buffer per metric task; and
* host `Vec<u8>` egress storage for device-to-external exit transfers.

`DeviceResources::completions` tracks each event as either
`CompletionEvent::Available(Event)` or `Active { task }`. `take_event` changes
the slot to `Active` before submission. A second checkout returns
`CompletionBusy`; a missing slot returns `MissingCompletion`.

The executor's `CudaPending` adds task, phase, queue, completion-slot, and
action metadata around the runtime token. Its native state is `Ready`,
`Active(PendingSubmission::Event)`, `Active(PendingSubmission::Stream)`, or
`Terminal`:

| Backend work | Runtime call | Native pending form | Terminal action |
| --- | --- | --- | --- |
| Init admission, external bytes to a device | `copy_h2d` from prefilled pinned staging | event-backed `Pending` | return event to slot |
| Calculation | `enqueue_launch` using `ParameterBlock` | stream-polled | none |
| Internal same-device transfer | `enqueue_d2d` | stream-polled | none |
| Metric readback | `enqueue_d2h` into a four-byte pinned buffer | stream-polled | decode F32 or I32 little-endian bytes |
| Exit device to external | `copy_d2h` into pinned staging | event-backed `Pending` | copy staging into preallocated egress `Vec` and return event |

For event-backed submissions the executor widens `Pending<'op, 'ctx>` to
`Pending<'static, 'ctx>` with a reviewed `transmute`. The runtime token stores
only its event and phantom borrow; the executor retains the arenas, staging,
stream, function, module, and parameter blocks in `CudaResources` until the
token is terminal. This is a lifetime transfer, not an ownership transfer of
the CUDA objects.

`ParameterBlock` in
[`native-executor/src/cuda_ffi.rs`](../../../native-executor/src/cuda_ffi.rs)
keeps boxed `u64` argument values and a parallel boxed pointer array at stable
addresses. It resets and repopulates raw keepalive pointers for each
calculation, then calls `enqueue_launch`. Arena buffers stay owned by the
executor while the stream is active.

`CudaResources::poll` checks the native state, calls `Pending::poll` for an
event-backed submission or `Stream::poll_idle` for a stream-polled submission,
and maps the result to `BackendPoll`. A CUDA error poisons the backend and all
later operations return `BackendPoisoned`; there is no retry or replacement
submission. `finish_pending` uses `Pending::wait(Duration::ZERO)` after a
complete status to recover an event without blocking. If that defensive
zero-duration wait returns `TimedOut`, it restores the active token and reports
`Pending`. Once complete, the executor puts the event back in its slot,
performs the metric or egress action, and marks the token terminal.

Loop tokens are reusable. A terminal loop token is rearmed to `Ready` without
allocating a new event, and its action is reset. A warm pre-final token is
recycled only after terminal completion; `prepared_tasks` rejects duplicate
preparation or recycling. An active `CudaPending` dropped by the executor is
intentionally forgotten, not dropped, so its event cannot be destroyed while
CUDA may still be using it. This is the executor's abandoned-work safety
policy; normal successful paths always poll to terminal completion.

### Staged cross-backend bridge

[`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs) uses the
runtime for one CUDA leg of a host-staged transfer. During realization it
allocates a pinned staging buffer, a nonblocking stream, and one completion
event for each CUDA endpoint. The event is held as `ActiveLeg::CudaReady`.

When a CUDA source leg is submitted, the bridge consumes that ready event and
calls `copy_d2h` from the CUDA arena into the leg's pinned staging. A CUDA
destination leg similarly calls `copy_h2d` from pinned staging into the CUDA
arena. The resulting pending token is lifetime-erased only because the bridge
keeps the arena and staging resource alive until terminal polling.

The bridge state machine is:

```text
Ready
  -> Source       (CUDA D2H, HSA copy, or direct host read submitted)
  -> Middle       (host worker copies staging bytes)
  -> Destination  (CUDA H2D or HSA copy submitted when needed)
  -> Complete
```

`poll_leg` delegates CUDA active legs to `Pending::poll`. After completion,
`recycle_leg` or `rearm_leg` calls `recycle_event` and restores
`CudaReady(Event)`, allowing the same event and stream to serve the next loop
iteration. Polling a ready, host, or transition leg is a bridge state error,
not a fabricated CUDA status. Bridge teardown destroys each CUDA stream and
then frees its pinned staging buffer after token state has been quiesced by the
higher-level executor.

## Teardown order and failure behavior

The runtime's individual `Drop` implementations are intentionally best effort,
but the native executor performs an observable ordered teardown in
`destroy_devices`:

1. query every realized stream with `poll_idle`; a pending stream returns
   `CudaContract("CUDA stream remained active during teardown")`;
2. destroy every available completion event; an `Active` completion slot
   returns `ResourceContention` naming its task;
3. destroy streams explicitly;
4. drop loaded `Function` holders (`artifacts`) before unloading their modules;
5. unload each module;
6. free metric pinned buffers, the shared staging buffer, and optional scratch.

Per-arena `DeviceBuffer` values are released by the executor's arena lifecycle
before resource destruction. A failed stream query, active completion slot,
resource-context mismatch, or Driver destruction error remains visible to the
caller. No cleanup loop invents a success status, retries a failed Driver call,
or silently substitutes a different context.

The runtime-level failure classes are:

| Condition | Result |
| --- | --- |
| PTX/fat binary, empty or NUL kernel name, zero allocation, zero launch axis, invalid byte range, pointer overflow, or `Instant` deadline overflow | `CudaError::InvalidInput` |
| Operation uses a closed module, buffer, stream, event, or context | `CudaError::ContextClosed` |
| Stream, function, buffer, event, or completion event belongs to another `Context` object | `CudaError::ResourceContextMismatch` |
| Driver reports a nonzero status during create, load, lookup, copy, launch, query, record, or destroy | `CudaError::DriverCall` with rich or numeric-only detail according to the call path |
| Driver reports success with a null native handle | `CudaError::InvalidDriverValue` |
| `recycle_event` sees a nonterminal event | `CudaError::InvalidInput` |
| Context push/pop returns a different handle | `CudaError::ContextStackMismatch` |

The runtime does not own higher-level poisoning, task contracts, queue-slot
allocation, or transfer routing. Those checks belong to
`recipe-native-executor`. Once a runtime error crosses that boundary, the
executor either poisons the CUDA resource set, reports a bridge failure, or
keeps polling the same token when cancellation is impossible.

## Caller checklist

For a direct event-backed operation, the valid sequence is:

```text
open one Context
  -> allocate live DeviceBuffer/PinnedHostBuffer values
  -> create one completion Event
  -> enqueue copy or launch
  -> retain Pending and every referenced resource
  -> poll or wait until Complete
  -> recycle_event or consume the completed event
  -> destroy buffers, stream, module, and context
```

For an enqueue-only operation, replace the event and `Pending` steps with
`Stream::poll_idle`, and keep every referenced resource live until that query
returns `Complete`. A timeout is not permission to free anything. A dropped
active token is not cancellation. The caller must either finish the operation
or preserve the resources until the Driver reports completion.
