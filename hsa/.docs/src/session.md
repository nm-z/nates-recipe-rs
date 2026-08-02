# `Session`, memory, executable, and queue ownership

## Scope

`Session<'runtime>` is the realized ownership scope for one discovered AMD GPU
agent. The type is declared in [`hsa/src/session.rs`](../../src/session.rs),
while its allocation, executable, signal, and dispatch methods are implemented
in [`hsa/src/execution.rs`](../../src/execution.rs). This page follows those
two modules through the public `recipe-hsa` boundary and the callers in
`native-probe` and `native-executor`.

The session is deliberately not a process-global device object. It borrows one
active [`Runtime`](runtime.md), retains the exact raw GPU agent and discovered
memory-pool ordering, and shares one permanent asynchronous-fault record with
all queues and completion tokens created from it. The public crate re-exports
`Session`, `Queue`, `QueueConfig`, and `QueueFault` from `hsa/src/lib.rs`.

The implementation is host-thread confined. `Session` owns an `Rc`-backed
`SignalPool` with `RefCell` state, and queues own `Rc<QueueCore>` values. The
queue API therefore exposes a single-producer publication discipline instead of
claiming thread-safe queue submission. Rustdoc consequently reports `Session`
and `Queue` as `!Send` and `!Sync`; `QueueFault` is a copyable diagnostic value,
not the ownership scope.

## Ownership graph and lifetimes

| Object | Created by | Retains or borrows | Meaning at drop |
| --- | --- | --- | --- |
| `Runtime` | `Runtime::open` | `Arc<Api>` and the loaded ROCr library | Balances `hsa_init` with `hsa_shut_down` when its active flag is still set. |
| `DiscoveredAgent<'runtime>` | `Runtime::discover` | `&'runtime Runtime`, a raw agent, copied raw pool handles, and owned descriptions | Drops ordinary Rust data. Agent and pool handles are not separately destroyed by this crate. |
| `Session<'runtime>` | `DiscoveredAgent::into_session` | `&'runtime Runtime`, one exact GPU agent, the raw pool vector, `AgentDescription`, `Arc<SharedFault>`, and `SignalPool` | Has no custom `Drop`; the signal pool performs signal and deferred-retirement teardown. |
| `Queue<'session, 'runtime>` | `Session::create_queue` | `&'session Session` and an `Rc<QueueCore>` | `Queue::close` or `QueueCore::Drop` calls `hsa_queue_destroy`. |
| `Allocation<'runtime>` | `Session` or `DiscoveredAgent` allocation methods | `Rc<AllocationInner>` and `Arc<Api>`; only the runtime lifetime is carried in the type | The unique owner calls `hsa_amd_memory_pool_free`; a failed free is not retried. |
| `Executable<'session, 'runtime>` | `Session::load_hsaco` | `Rc<ExecutableInner>` plus a phantom borrow of the session | Destroys the executable, then its code-object reader, then releases HSACO backing bytes. |
| `Kernel<'session, 'runtime>` | `Executable::kernel` | `Rc<ExecutableInner>` and a phantom session borrow | Keeps its executable alive until the kernel is dropped. |
| `PreparedPending<'session, 'runtime>` or `Pending<'session, 'runtime>` | `Session::prepare_pending`, copy methods, or queue dispatch | `SignalPool` plus signal records and operation keepalives | Completed resources are released or signals are recycled. Incomplete work enters deferred retirement. |

The Rust lifetimes enforce the important edges. A queue, executable, kernel,
dependency, or pending token cannot safely outlive its session. An allocation
returned by a session is intentionally only runtime-scoped: it can be shared by
the host allocator and GPU session, and its `Rc<AllocationInner>` is retained by
pending operations until the device no longer references it. The runtime borrow
also prevents `Runtime::close` while any discovery record, session, or allocation
is live.

`raw_pools` and `description.memory_pools` are parallel vectors produced by
discovery. A pool index is meaningful only against that exact pair. The raw
`HsaMemoryPool` values are copied handles, not allocation owners requiring a
pool-destroy call.

## Constructing a session

### Input and admission checks

`DiscoveredAgent::into_session` consumes the discovery record at
`hsa/src/session.rs:145-200`.

1. It calls `Runtime::ensure_active`. A closed runtime returns
   `Error::RuntimeClosed` before any agent validation.
2. It requires `description.device_type == DeviceType::Gpu`; a CPU, DSP, or
   AIE record returns `Error::UnsupportedAgent` with reason `the agent is not a
   GPU`.
3. It requires the raw feature bits to include
   `AGENT_FEATURE_KERNEL_DISPATCH`; otherwise the reason is `the GPU does not
   report kernel-dispatch support`.
4. It requires at least one discovered ISA and every ISA to have an exact
   `amd_target`. Missing or non-AMDGPU identities return
   `Error::UnsupportedAgent` with reason `the GPU lacks an exact AMD ISA
   identity`.

These checks use the immutable discovery description. No queue, allocation,
executable, or HSA signal has been created when an admission check fails, so the
consumed record is cleaned up by ordinary Rust drops.

Queue capability, allocatable pool flags, and queue-count limits are not
revalidated here. They remain part of the immutable description and are checked
at `create_queue` or allocation time, which permits a valid GPU session to exist
even when a particular optional queue or pool operation is unavailable.

### Timestamp and state creation

After admission, `into_session` queries
`SYSTEM_INFO_TIMESTAMP_FREQUENCY` through `hsa_system_get_info`. The successful
`u64` value is stored in the new `SignalPool` and later converts bounded host
durations into ROCr signal-wait ticks. A status failure is returned as the
annotated `Error::Hsa` from `Api::check`; the `MaybeUninit` output is read only
after that success check.

The constructor then creates one `Arc<SharedFault>` and one
`SignalPool::new(Arc<Api>, Arc<SharedFault>, timestamp_frequency_hz)`. It moves
the runtime borrow, raw agent, raw pools, and description into:

```text
Session {
    runtime,
    raw_agent,
    raw_pools,
    description,
    fault,
    signals,
}
```

`SignalPool::new` only creates Rust ownership state. Device-visible completion
signals are acquired lazily by `prepare_pending`, `copy_async`, and
`dispatch_after`. There is no session constructor side effect that allocates
device memory or creates a queue.

## Session API surface

The following table records the public methods and their actual checks. The
absence of a fault check is intentional and matters after an asynchronous queue
callback has poisoned the session.

The complete session-facing signature index is:

```rust
DiscoveredAgent::into_session(self) -> Result<Session<'runtime>>
Session::description(&self) -> &AgentDescription
Session::available_memory_bytes(&self) -> Result<u64>
Session::fault(&self) -> Option<QueueFault>
Session::ensure_healthy(&self) -> Result<()>
Session::create_queue(&self, config: QueueConfig) -> Result<Queue<'_, '_>>
Session::allocate(&self, pool_index: usize, size: usize) -> Result<Allocation<'runtime>>
Session::allocate_coarse(&self, size: usize) -> Result<Allocation<'runtime>>
Session::allocate_fine(&self, size: usize) -> Result<Allocation<'runtime>>
Session::allocate_kernarg(&self, size: usize) -> Result<Allocation<'runtime>>
Session::grant_access(&self, allocation: &Allocation<'_>) -> Result<()>
Session::grant_access_exact_set(
    &self,
    allocation: &Allocation<'_>,
    sessions: &[&Session<'runtime>],
    agents: &[&DiscoveredAgent<'runtime>],
) -> Result<()>
Session::poll_retirements(&self) -> RetirementReport
Session::drain_retirements(&self, timeout: Duration) -> Result<RetirementReport>
Session::load_hsaco<'session>(&'session self, hsaco: &[u8])
    -> Result<Executable<'session, 'runtime>>
Session::prepare_pending<'session>(
    &'session self,
    allocation_capacity: usize,
    dependency_capacity: usize,
) -> Result<PreparedPending<'session, 'runtime>>
Session::copy_async_prepared(
    &self,
    destination: &Allocation<'runtime>, destination_offset: usize,
    source: &Allocation<'runtime>, source_offset: usize, size: usize,
    pending: &mut PreparedPending<'_, 'runtime>,
) -> Result<()>
Session::copy_async<'session>(
    &'session self,
    destination: &Allocation<'runtime>, destination_offset: usize,
    source: &Allocation<'runtime>, source_offset: usize, size: usize,
) -> Result<Pending<'session, 'runtime>>
```

These signatures retain the literal argument order used by the implementation.

| Method | Runtime-active check | Fault check | Result or side effect |
| --- | --- | --- | --- |
| `description()` | No | No | Borrows the immutable `AgentDescription`. |
| `available_memory_bytes()` | Yes | Yes | Queries the current `AMD_AGENT_INFO_MEMORY_AVAIL` counter for this exact agent. |
| `fault()` | No | No | Returns the newest `QueueFault`, or `None` before the first callback. |
| `ensure_healthy()` | No | No | Returns `Ok(())` or `Error::SessionPoisoned`. |
| `create_queue(config)` | Yes | Yes | Validates discovered queue capability and returns a live `Queue`. |
| `allocate(index, size)` and `allocate_coarse/fine/kernarg(size)` | Yes | No | Allocates from the exact discovered pool and returns an `Allocation`. |
| `grant_access(allocation)` | Yes | No | Replaces direct access with this session's agent plus ROCr's required owner. |
| `grant_access_exact_set(allocation, sessions, agents)` | Yes | No | Validates one runtime and replaces direct access with the sorted, deduplicated set. |
| `poll_retirements()` | No | No | Performs one nonblocking deferred-retirement pass and returns `RetirementReport`. |
| `drain_retirements(timeout)` | No | No | Polls and waits in bounded chunks until drained, timed out, or poisoned. |
| `load_hsaco(bytes)` | Yes | Yes, before and after realization | Creates, loads, freezes, and returns a session-scoped `Executable`. |
| `prepare_pending(allocation_capacity, dependency_capacity)` | Yes | Yes | Acquires one signal and reserves fixed keepalive vectors before submission. |
| `copy_async(...)` | Yes | Yes | Submits an AMD asynchronous copy and returns a `Pending` token. |
| `copy_async_prepared(..., pending)` | Yes | Yes | Uses an already realized token and mutates it to active state. |

The queue methods perform their own active and health checks through the queue's
borrowed session. `Allocation::copy_from_host` and `copy_to_host` are unsafe,
range-checked pointer copies and do not query runtime or fault state.

## Permanent queue fault state

### Shared record

`SharedFault` is the callback-safe state at
`hsa/src/session.rs:29-92`:

```text
poisoned: AtomicBool
status: AtomicI32
source_queue_id: AtomicU64
source_known: AtomicBool
epoch: AtomicU64
_wait_lock: Mutex<()>
wake: Condvar
```

`new()` initializes `poisoned=false`, `status=0`, `source_known=false`, and
`epoch=0`. `record(status, source_queue_id)` stores the newest numeric status,
sets or clears the source-id marker, increments the epoch, publishes
`poisoned=true` with release ordering, and calls `wake.notify_all()`. The
callback path takes no lock and allocates no memory. The mutex and condition
variable are retained in the shared state, but no public session method blocks
on them; polling and bounded signal waits are the current wait mechanisms.

`snapshot()` first acquires `poisoned`. If it is false it returns `None`; if it
is true it reads the latest status, optional source queue id, and epoch into a
`QueueFault`:

| Field | Meaning |
| --- | --- |
| `status: i32` | The most recently recorded ROCr queue callback status. |
| `source_queue_id: Option<u64>` | The source queue's `HsaQueue.id` when ROCr supplied a non-null queue pointer. |
| `epoch: u64` | Number of callback records after initialization, identifying the observed fault generation. |

`check()` maps a snapshot to `Error::SessionPoisoned { status,
source_queue_id, epoch }`. Poisoning is permanent. There is no reset method and
new queues, executable loads, copies, prepared tokens, and dispatches reject a
poisoned session. `fault()` remains available for diagnosis even after poison.

### Callback lifetime

Every queue receives a `Box<QueueCallbackContext>` containing an `Arc` clone of
the session fault. ROCr receives a stable raw pointer to that box. The callback
returns immediately for null data, otherwise reads the live source queue id for
the duration ROCr guarantees and calls `SharedFault::record`.

The box is retained in `QueueCore.callback` until `hsa_queue_destroy` returns.
This is the reason queue teardown drops the callback only after a successful
destroy, and leaks the box when destroy fails. The leak is deliberate: ROCr may
still invoke the callback after reporting a destruction error, so freeing the
context would create a use-after-free.

### Health checks and pending work

`Pending::poll` and `Dependency::poll` check the fault when the signal is still
positive and again when it reaches zero. A queue callback can therefore fail a
logical operation before its completion signal becomes terminal. In that case
the operation stays nonterminal and its `Drop` path moves the signal and every
referenced allocation, executable, queue, and dependency into deferred
retirement. It never releases a resource that the device may still reference.

## Memory allocation and access lifecycle

### Pool selection and allocation

`Session::allocate` and its convenience methods are in
`hsa/src/execution.rs:525-708`.

`allocate_from` performs the following sequence:

1. Check that the runtime is active.
2. Resolve the same index in `raw_pools` and `description.memory_pools`.
   Out-of-range indices return `InvalidMemoryPoolIndex`.
3. Require `runtime_allocation` metadata. A discovered pool that is not
   runtime-allocatable returns `MemoryPoolNotAllocatable`.
4. Reject zero bytes and a size above
   `maximum_aggregate_allocation_bytes`, when ROCr supplied that maximum, with
   `InvalidAllocationSize`.
5. Call `hsa_amd_memory_pool_allocate(pool.handle, size, 0, &pointer)`.
   A non-success status is checked as `hsa_amd_memory_pool_allocate`; a
   successful null pointer is `NullAllocation`.
6. Create `AllocationInner` with the exact owner agent, pool index, optional
   global flags, and `accessible_agents = [owner.handle]`.

The convenience predicates select the first matching global, allocatable pool:

| Method | Required global flag |
| --- | --- |
| `allocate_coarse` | `COARSE_GRAINED` |
| `allocate_fine` | `FINE_GRAINED` or `EXTENDED_SCOPE_FINE_GRAINED` |
| `allocate_kernarg` | `KERNARG_INITIALIZATION` |

No pool is guessed when a predicate has no match. The result is
`NoMatchingMemoryPool { kind }`. A session allocation is owned by its GPU
agent. The corresponding `DiscoveredAgent` methods are used for CPU host
allocators and produce allocations owned by that CPU agent.

### Access grants

`grant_access` calls `hsa_amd_agents_allow_access` with one explicit agent, the
session or discovered-agent handle. ROCr treats this as replacement, not
accumulation: the supplied agents and the allocation's pool owner remain the
direct-access set. The in-memory `accessible_agents` vector is updated only
after the HSA call succeeds, using exact numeric handles.

The low-level helper rejects an empty set and agent counts that do not fit the
ABI `u32`. A failed grant leaves the prior recorded access set unchanged. The
public convenience method does not perform a session fault check, so callers
must use `ensure_healthy` when they need a health gate before a grant.

`grant_access_exact_set` accepts `&[&Session]` and `&[&DiscoveredAgent]`. It
requires every object to point at the same `Runtime` and the same `Arc<Api>` as
the session performing the grant. It then sorts by raw handle, removes
duplicates, and calls the same replacement API. ROCr still retains the pool
owner even when the supplied exact list is empty; this implementation rejects
an empty list through the low-level count check rather than inventing an
additional access agent.

### Host copies and closure

`Allocation::copy_from_host` and `copy_to_host` use
`check_range(buffer, offset, size, capacity)` with checked addition before
calling `ptr::copy_nonoverlapping`. The caller must guarantee that the selected
pool is host-writable or host-readable, that the host and allocation regions do
not overlap, and that no device operation concurrently accesses the range.
`copy_to_host` additionally requires a system-scope producer completion.

`Allocation::close(self)` consumes the handle. A unique `Rc` calls
`AllocationInner::destroy`, which takes the pointer out of its `Option` before
calling `hsa_amd_memory_pool_free`. A shared `Rc` is dropped and returns
`ResourceBusy { resource: "HSA memory allocation" }`. `Drop` calls the same
destroy routine but discards its result. If ROCr reports a free failure, the
pointer has already been removed, so the destructor does not retry it.

Asynchronous copies retain both endpoint `AllocationInner` values in
`PendingKeepalive._allocations`. This keeps pointers valid until the completion
signal reaches a terminal value, including the deferred-retirement path after a
queue fault or timeout.

## Executable and kernel lifecycle

### Loading an in-memory HSACO

`Session::load_hsaco` is implemented at `hsa/src/execution.rs:974-1060` and
does not read a file. It receives a nonempty byte slice, copies it into an
`Arc<[u8]>`, and creates an `ExecutableInner` containing the backing bytes,
`Arc<Api>`, the exact session agent, and empty reader and executable slots.

The realization sequence is ordered and each later object depends on the prior
one:

1. `hsa_code_object_reader_create_from_memory` receives the stable Arc-backed
   bytes. A failure returns immediately; no reader handle was recorded.
2. The successful reader is stored in `inner.reader`.
3. `hsa_executable_create_alt` uses the discovered agent profile,
   `DEFAULT_FLOAT_ROUNDING_MODE_NEAR`, and null options. If it fails,
   `inner.destroy()` destroys the reader and releases or deliberately leaks its
   backing bytes according to the reader-destroy result.
4. The executable handle is stored in `inner.executable`.
5. `hsa_executable_load_agent_code_object` loads the reader for the exact raw
   session agent. On failure, `inner.destroy()` destroys the executable first,
   then the reader.
6. `hsa_executable_freeze` freezes the loaded executable. On failure, the same
   ordered cleanup runs.
7. `Session::ensure_healthy` is checked after freeze. If a queue callback
   poisoned the session during realization, the local `ExecutableInner` is
   dropped and performs the ordered cleanup before the poison error is returned.

The loaded-code-object result handle is not retained separately. ROCr keeps the
loaded image attached to the executable, whose reader and backing bytes remain
alive in `ExecutableInner`.

### Executable and kernel ownership

`Executable` wraps `Rc<ExecutableInner>` and carries a phantom borrow of the
session. `Executable::kernel(name)` rejects interior NULs, looks up the named
symbol for the exact agent, and reads the symbol type and kernel metadata. It
requires a kernel symbol kind, a nonzero kernel object, and a nonzero power-of-
two kernarg alignment. The returned `Kernel` owns another `Rc` clone and exposes:

| Kernel output | Source of value |
| --- | --- |
| `name()` | The caller's symbol name, copied into an owned `String`. |
| `metadata().kernarg_segment_size` | `EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_SIZE`. |
| `metadata().kernarg_segment_alignment` | `EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_ALIGNMENT`, validated above. |
| `metadata().group_segment_size` | `EXECUTABLE_SYMBOL_INFO_KERNEL_GROUP_SEGMENT_SIZE`. |
| `metadata().private_segment_size` | `EXECUTABLE_SYMBOL_INFO_KERNEL_PRIVATE_SEGMENT_SIZE`. |
| `metadata().dynamic_callstack` | `EXECUTABLE_SYMBOL_INFO_KERNEL_DYNAMIC_CALLSTACK`, decoded as a checked ROCr boolean. |

`Executable::close(self)` succeeds only when its `Rc` is unique. A live
`Kernel`, a pending dispatch keepalive, or another executable owner yields
`ResourceBusy { resource: "HSA executable" }`; the dropped Rc remains with the
other owner. `ExecutableInner::destroy` always destroys the HSA executable
before the code-object reader. If executable destruction fails, it takes and
forgets the reader backing rather than risk releasing bytes ROCr may still use.
If reader destruction fails, it likewise forgets the HSACO Arc. `Drop` invokes
this best-effort destroy and suppresses its returned error.

The native executor groups artifacts by content digest, loads one executable
per distinct HSACO image, and resolves each logical kernel from that shared
executable. `LoadedArtifact.kernel` is dropped before its executable map during
resource destruction, so explicit `Executable::close` calls can succeed when
there are no pending keepalives.

## Completion signals and deferred retirement

Although the pool implementation lives in `execution.rs`, it is part of the
session ownership contract. `SignalPool` owns one `Rc<SignalPoolInner>` with the
API, shared fault record, timestamp frequency, and a `RefCell<SignalPoolState>`:

```text
SignalPoolState {
    available: Vec<HsaSignal>,
    retired: Vec<RetiredResource<Rc<SignalRecord>, PendingKeepalive>>,
    retirement_reservations: usize,
    fatal_signal: Option<i64>,
}
```

`SignalRecord` stores a weak pool link, API Arc, raw signal, terminal value, and
a reservation marker. `PendingKeepalive` can retain one queue, one executable,
any allocations, and dependency signal records. Resources are released before
the completion signal is recycled or destroyed.

### Acquire and prepare

`SignalPool::acquire` first collects completed retired records. It rejects a
previous negative signal through `AsyncSignal`, reserves enough retired-vector
capacity for every active reservation plus the new signal, then reuses an
available signal or creates a new signal initialized to one. A failed create or
zero handle releases the reservation and returns the typed error.

The pool's `fatal_signal` gate is separate from `SharedFault`: a negative
completion observed by `Pending::poll` or retirement collection prevents future
signal acquisition, while `Session::ensure_healthy` reports only asynchronous
queue-callback poison. Callers therefore must handle both `SessionPoisoned` and
`AsyncSignal` results.

`Session::prepare_pending(allocation_capacity, dependency_capacity)` performs
the active and health checks, acquires one signal, and `try_reserve_exact`s the
two keepalive vectors. A reserve failure releases the unsubmitted signal. The
result is a `PreparedPending` in `Ready` state. It contains all host storage
needed by the live submit path and can therefore be reused without signal
creation or vector growth inside `init` or the loop.

`PreparedPending::begin` consumes the ready state temporarily, verifies that
both vector capacities meet the operation's required counts, clears prior
keepalive entries, and returns the signal plus mutable keepalive. Insufficient
capacity restores `Ready` and returns `InvalidDispatch`. A successful submit
transitions to `Active(Pending)`; an unsubmitted ready token returns its signal
to the pool on drop. `reset()` only rearms a terminal token by restoring its
signal to one and clearing terminal state. A pending or consumed token is not
silently reused.

### Poll, wait, and drop

Signals classify as positive `Pending`, zero `Complete`, and negative
`Failed(value)`.

* A complete `Pending` marks the signal terminal, releases its reservation,
  drops queue, executable, allocation, and dependency keepalives, then checks
  the shared fault. The terminal result is cached.
* A negative signal calls `SignalPool::fail`, marks the signal terminal, records
  the first fatal signal, releases keepalive resources, and returns
  `Error::AsyncSignal { value }`.
* A positive signal plus a queue fault returns `SessionPoisoned` while leaving
  the token nonterminal. Its `Drop` path therefore retains the device-visible
  references in the deferred set.
* Dropping an incomplete token calls `SignalPool::retire`. The signal and
  keepalive remain in `retired` until a nonpositive completion value is observed.
  Dropping a terminal token releases the keepalive and lets `SignalRecord::Drop`
  recycle a zero signal or destroy a negative one.

`Dependency` clones the signal record without consuming it. Polling a dependency
does not recycle the signal, and it checks the same session fault after reading
the signal value. Dependency records are retained by dependent dispatch
keepalives until the dependent completion is terminal.

`poll_retirements()` scans the retired vector until no nonpositive signal is
found. It reports reclaimed records, pending records, the first negative signal,
and current queue poison. `drain_retirements(timeout)` repeats that pass and
waits only on the first retired signal for at most one millisecond per bounded
chunk. A timeout or a poisoned session with unresolved records returns
`DeferredRetirement` and leaves the set intact for a later call. If the set is
empty, a recorded negative signal returns `AsyncSignal`; an empty, merely
poisoned report is otherwise returned as a drained report. The method itself
does not call `ensure_active` or `ensure_healthy`.

`SignalPoolInner::Drop` destroys available signals. For retired records it first
loads each signal: terminal records release their keepalives and signal, while
positive unresolved records are forgotten as a unit and counted. A nonzero
count emits a terminal `eprintln!` diagnostic instructing callers to run
`Session::drain_retirements` before dropping the session. `SignalRecord::Drop`
also emits a diagnostic rather than destroying a positive signal found outside
the retirement set. These paths intentionally prefer a visible resource leak to
releasing a signal or allocation that the device may still reference.

## Queue creation and ownership

### Configuration and validation

`QueueConfig` is a copyable value with `size_packets`, `kind`,
`private_segment_size`, and `group_segment_size`. `QueueConfig::new(size,
kind)` sets both segment-size fields to `u32::MAX`, the ROCr request for its
normal limit. `Session::create_queue` at `hsa/src/session.rs:230-335` then:

1. Checks runtime activity and permanent session health.
2. Requires `description.queue`; an agent with no user-mode queue capability
   returns `UnsupportedAgent`.
3. Requires an inclusive packet-size range and a power-of-two request.
   Violations return `InvalidQueueSize` with the discovered minimum and
   maximum.
4. Enforces the advertised queue kind. A `SingleProducer` advertisement accepts
   only `SingleProducer`; `MultiProducer` accepts `MultiProducer` or
   `SingleProducer`; `Cooperative` accepts only `Cooperative`.
5. Allocates a boxed callback context, passes its stable pointer to
   `hsa_queue_create`, and checks the returned status.

The API call is allowed to choose normal private and group segment limits when
the configuration fields are `u32::MAX`. The request is still checked against
the exact discovered queue capability before calling ROCr.

### Returned queue checks

A successful queue creation must return a non-null raw pointer. `NullQueue` is
returned if ROCr reports success with a null pointer. For a non-null result the
implementation reads `base_address`, `size`, `kind`, and `features`, requiring:

* a non-null packet-ring base address;
* a nonzero power-of-two actual size;
* `AGENT_FEATURE_KERNEL_DISPATCH` in the returned feature bits; and
* a recognized raw queue kind.

An invalid result is destroyed immediately. If that destroy succeeds, the
callback box is dropped. If it fails, the callback box is leaked because ROCr may
still retain the callback pointer. The returned error is
`InvalidQueueReturned { kind, features, size, base_is_null }`; a destroy failure
does not replace that invalid-return diagnostic.

For a valid result, `QueueCore` stores the API Arc, raw queue pointer, callback
box, the requested `config.kind`, and the exact session agent. ROCr may report
the agent's multi-producer protocol in its read-only queue field even when a
single-producer queue was requested. The confined Rust API retains the requested
discipline and only permits safe AQL publication when `kind()` is
`SingleProducer`.

### Close and automatic destruction

`Queue::close(self)` takes the queue's `Option<Rc<QueueCore>>` and attempts
`Rc::try_unwrap`:

* If this is the unique owner, `QueueCore::destroy` replaces the raw pointer with
  null before calling `hsa_queue_destroy`. A successful status drops the
  callback box and returns `Ok(())`.
* If another owner exists, normally a pending keepalive, the queue Rc is
  dropped and `ResourceBusy { resource: "HSA queue" }` is returned. The other
  owner remains responsible for eventual destruction.

If `hsa_queue_destroy` fails, the raw pointer has already been removed from the
core and the callback box is leaked. `QueueCore::Drop` calls the same destroy
routine and suppresses its result. This ensures at-most-one destroy call and
never frees callback context while ROCr's ownership is ambiguous.

Queue accessors `id()`, `size_packets()`, and `kind()` read the live core;
`session()` returns the borrowed session. The private `core()` accessor asserts
that the queue has not been consumed by `close`.

## Queue publication and dispatch

### Ring publication invariant

`HsaQueueIo` is the concrete `QueueIo` implementation. Queue creation proves a
non-null power-of-two 64-byte ring. Occupancy loads the write index with relaxed
ordering and the read index with acquire ordering. `enqueue_packets` rejects a
full ring before writing any packet and returns `QueueFull { write_index,
read_index, size }`; it does not advance the write index on backpressure.

For each packet, publication is ordered as:

1. Write the 64-byte packet body with an invalid header.
2. Release-store the final valid packet header.
3. Release-store the next queue write index.
4. Release-store the doorbell signal.

Only a host-confined single producer may use this sequence. The slot index is
the queue index masked by `size - 1`, so wraparound is defined by the validated
power-of-two ring. Kernel and barrier packet headers request system-scope
acquire and release fences, so the packet body, queue index, doorbell, and
system-visible operation share one ordered publication boundary.

### Geometry and kernarg validation

`validate_geometry` requires one, two, or three dimensions; nonzero grid and
workgroup dimensions; one in every unused dimension; workgroup dimensions no
larger than grid dimensions; and checked products for total workgroup and grid
cardinality. Every exact discovered ISA must accept the totals and each per-
dimension limit. Violations return `InvalidDispatch`.

For a kernel with a zero kernarg segment size, `None` is valid and produces a
null kernarg pointer. Otherwise the supplied allocation must be from a
discovered kernarg-capable pool, be accessible by the queue agent, meet the
metadata size, and satisfy the power-of-two alignment. A missing required
allocation or any mismatch is `InvalidDispatch`. Dynamic-callstack kernels
also require a nonzero explicit dynamic private-byte allowance, and static plus
dynamic group/private sizes must not overflow their AQL fields.

### Prepared dispatch

`Queue::dispatch_prepared` is the production live-loop path. It checks runtime
activity and health, verifies that the prepared token belongs to the queue's
session signal pool, requires `SingleProducer`, requires the kernel executable
to target the queue's exact agent, validates geometry and kernarg metadata, and
calls `PreparedPending::begin` with one optional kernarg keepalive slot.

Before enqueue it places the queue core, executable, and optional kernarg
allocation into the token keepalive, builds one kernel packet, and calls
`enqueue_packets`. A queue-full or other enqueue error restores the token to
`Ready` with all keepalives cleared. After successful publication it activates
the token. If a callback poisoned the session between publication and the final
health check, it retires the active token and returns the poison error.

`Queue::dispatch` is the non-prepared convenience wrapper. It delegates to
`dispatch_after` with no dependencies and returns a normal `Pending` token.

### Dependent dispatch

`Queue::dispatch_after` uses the same queue, agent, geometry, kernarg, and health
checks, then verifies that every `Dependency` belongs to the same signal pool
and polls each dependency before publication. A negative dependency therefore
fails before a barrier packet can be published.

Dependencies are lowered into a deterministic barrier-AND tree with a fan-in
of five. `dependent_dispatch_packet_count(n)` returns the number of barrier
packets plus one kernel packet and errors if the count cannot fit `u32`. A
required count larger than the queue ring returns `InvalidProgressRequest`.

The method acquires one final completion signal and one signal per barrier
packet. Any acquisition failure releases every unsubmitted signal. It builds
all vectors and packet bodies before reserving a queue slot. The pending
keepalive retains the queue, executable, optional kernarg allocation, original
dependency records, and barrier signal records. On enqueue failure it drops the
keepalive and releases all signals as unsubmitted. On success it releases the
barrier signals' retirement reservations, retains them through the dependency
keepalive, and returns a normal `Pending`. A poison observed immediately after
publication moves the final completion signal and keepalive into deferred
retirement.

### Capacity probe

`Queue::progress_capacity(required_packets, probe_budget)` performs only bounded
occupancy reads. Zero requirements or requirements larger than the ring return
`InvalidProgressRequest`. Up to the nonzero probe budget, it reports
`QueueProgress::Ready { available_packets, probes }` as soon as enough capacity
is observed, otherwise `Backpressured { available_packets, required_packets,
probes }`. It never sleeps, blocks, writes packet memory, or advances the queue.

## Native callers and complete resource paths

### Standalone `execute_smoke`

`hsa/examples/execute_smoke.rs` opens and discovers a runtime, consumes one GPU
agent into a session, and keeps a CPU discovered agent for host allocations. It
allocates CPU fine, GPU coarse, and CPU fine source/device/destination buffers,
grants each cross-agent access, initializes the host source, and waits for
host-to-device and device-to-host `copy_async` tokens. It independently reads
the CPU fine destination after system-scope completion and compares bytes.

When `RECIPE_HSA_SMOKE_COPY_HSACO` and `RECIPE_HSA_SMOKE_SYMBOL` are set, it
also loads an in-memory HSACO, resolves its kernel, allocates output and CPU
kernarg storage, grants GPU access, creates a minimum-size single-producer
queue, submits a prerequisite copy, and publishes a dependent dispatch through
`dispatch_after`. It waits for dispatch and prerequisite completion, closes the
queue, verifies a download, then drops the kernel and closes the executable and
allocations. The required path prints
`ROCr fine→coarse→fine asynchronous copy smoke passed`; the optional path
also prints `optional HSACO kernel dispatch smoke passed`. A mismatch or any
typed HSA error reaches the boxed process result and exits unsuccessfully.

### Native probe benchmark

`native-probe/src/hsa.rs` retains one `Runtime` in `HsaBackend` and runs each
discovery or benchmark inside `with_runtime`. The benchmark rediscoveries an
exact GPU, consumes that `DiscoveredAgent` into a session, allocates host and
GPU buffers, grants both directions of access, and times real asynchronous
copies. Its calculation benchmark lowers a real HSACO, calls `load_hsaco`, reads
kernel metadata, allocates and initializes CPU kernarg memory, creates a
single-producer queue, and uses `Queue::dispatch` with a normal `Pending`.

The benchmark waits each token to terminal state before host verification. It
then closes the queue, drops the kernel, closes the executable and kernarg, and
lets the remaining allocations and session leave the callback scope. No runtime
or session is exported into a later placement path. The benchmark's measured
capacity and transfer/calculation rates are outputs of the surrounding probe,
not cached session state.

### Native execution bindings

`native-probe/src/bindings.rs` reopens exact measured HSA agents, chooses a
same-NUMA CPU host allocator, and calls `into_session` for every measured GPU.
`NativeExecutionBindings` lends `HsaBinding` values to one callback while the
probe retains the runtime, discovered host allocators, and sessions. The
callback scope is required: each binding stores `&Session` and
`&DiscoveredAgent`, and all queues, executables, allocations, and pending tokens
must be destroyed before the runtime state can be released.

`HsaBinding::available_bytes` delegates to the session's live memory counter.
`allocate_host_fine` allocates from the selected CPU agent and immediately calls
`Session::grant_access` for the bound GPU.

### Production `native-executor` resources

`native-executor/src/hsa.rs` receives these borrowed bindings while realizing a
finalized plan. `realize_device` uses each binding session to:

* create one `Queue` per planned queue slot, always with
  `QueueKind::SingleProducer` and the configured packet count;
* allocate host fine-grained staging and grant GPU access;
* allocate optional GPU coarse scratch;
* load one `Executable` per distinct HSACO digest and resolve each artifact's
  `Kernel`;
* allocate CPU kernarg slots, grant GPU access, and preallocate host metric
  buffers; and
* prepare one `PreparedPending` token per finalized task with two allocation
  keepalive slots and no dependency slots.

The static resource maps retain queue, artifact, executable, kernarg, metric,
staging, egress, and scratch ownership. Init admission, internal transfers,
exit transfers, four-byte metrics, and calculations all submit through the
same prepared token boundary. Transfers call `copy_async_prepared`; calculation
tasks fill a preallocated kernarg slot and call `dispatch_prepared`. No signal,
queue, executable, or vector capacity is realized by the live submission after
the pre-loop preparation pass.

If any validation, queue creation, allocation, HSACO load, symbol lookup, or
prepared-token reservation fails during `realize` or `realize_device`, the
partially built maps and locals unwind immediately. Their Rust drops invoke the
same queue, executable, signal, and allocation cleanup paths; the failed
candidate is not retained as a usable backend. `execution_evidence()` reports
the realized per-device counts of image loads, entry lookups, queues,
completion objects, and persistent allocations. Those counts describe the
already-realized maps and do not create or probe another session object.

The cross-backend staged bridge in `native-executor/src/bridge.rs` follows the
same rule. An HSA leg allocates fine-grained staging and calls
`Session::prepare_pending(2, 0)` before execution. Source and destination legs
then call `copy_async_prepared`, retaining staging and arena allocations until
the prepared token is terminal.

### Production destruction order

`HsaResources::destroy` first requires the backend to be healthy and drops its
prepared pending-token pool. `destroy_devices` then checks that every completion
slot is available and calls `Session::drain_retirements(10 ms)` for each session.
Only after deferred work is drained does it:

1. close every queue;
2. drop loaded artifact kernels;
3. close shared executables;
4. close kernarg allocations;
5. close four-byte metric buffers;
6. close fine-grained staging; and
7. close optional scratch allocations.

Any health, active-completion, retirement, busy-resource, or HSA teardown error
stops this explicit sequence. Rust drops still run for the consumed maps, and
the underlying best-effort destructors preserve the no-use-after-free rules
described above.

## Failure and cleanup matrix

| Failure point | Returned error or output | Cleanup invariant |
| --- | --- | --- |
| Runtime closed during session construction or queue/allocation operation | `RuntimeClosed` | No HSA object is created by the rejected operation. |
| Discovered record is not an eligible GPU | `UnsupportedAgent` | The consumed discovery record drops ordinary Rust data only. |
| Timestamp query fails | `Hsa` for `hsa_system_get_info(TIMESTAMP_FREQUENCY)` | No session is returned and no signal has been created. |
| Invalid pool index, pool capability, or size | `InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, `NoMatchingMemoryPool`, or `InvalidAllocationSize` | Allocation API is not called. |
| Allocation reports success with null pointer | `NullAllocation` | The invalid pointer is not wrapped. |
| Access grant fails | HSA status error | `accessible_agents` is not changed. |
| Queue size or kind rejected before ROCr | `InvalidQueueSize` or `UnsupportedQueueKind` | Callback context is never allocated. |
| Queue create status fails | HSA status error | The local callback box drops; ROCr is expected not to own a queue after failure. |
| Queue create succeeds with null or invalid fields | `NullQueue` or `InvalidQueueReturned` | Newly returned queue is destroyed. Callback context is dropped only after successful destroy, otherwise leaked. |
| Queue close sees pending Rc keepalive | `ResourceBusy { resource: "HSA queue" }` | The other Rc owner keeps the queue and callback alive. |
| Queue destroy reports failure | HSA status error from explicit close, ignored by `Drop` | Raw pointer is cleared and callback context is leaked to avoid a callback UAF. |
| Empty HSACO or bad reader/executable/load/freeze operation | `EmptyCodeObject` or operation-specific `Hsa` | `ExecutableInner::destroy` orders executable, reader, and backing cleanup; failed destruction leaks ambiguous backing. |
| Kernel lookup is not a kernel, has zero object, or bad alignment | `SymbolNotKernel` or `InvalidKernel` | The executable remains owned by its Rc and can still be closed later. |
| Async copy API fails before publication | HSA status error | Unsubmitted signal is terminalized and recycled or destroyed; endpoint keepalives drop. |
| Successful submission followed by queue poison | `SessionPoisoned` | Active token is retired; signal and all referenced resources remain deferred. |
| Queue ring has insufficient capacity | `QueueFull` or `QueueProgress::Backpressured` | No packet body, header, write index, or doorbell is published for the rejected enqueue. |
| Dependency belongs to another session or is already negative | `InvalidDispatch` or `AsyncSignal` | No dependent barrier is published; every newly acquired signal is released. |
| Incomplete token is dropped | No immediate result | `SignalPool` owns it in the explicit retirement set until terminal completion. |
| Retirement times out or remains poisoned | `DeferredRetirement` | Retired records stay intact for a later poll or drain. |
| Session or signal pool drops with positive unresolved work | Terminal `eprintln!` diagnostic | The signal and keepalive are forgotten as a unit instead of being released unsafely. |
| Allocation or executable close sees shared Rc | `ResourceBusy` | The surviving owner retains the underlying resource. |
| Allocation free, reader destroy, or executable destroy fails in `Drop` | Drop has no returned error | The implementation does not retry after ownership has become ambiguous; diagnostics or intentional backing leaks preserve safety. |

## Invariants for callers

* Build a session only from an active runtime and a discovery record whose GPU,
  kernel-dispatch, and exact ISA checks pass.
* Treat `Session::fault()` as a permanent health indicator. Once it returns a
  fault, do not attempt to recover by creating another queue or token from that
  session.
* Use the exact discovered pool index or one of the three flag-based selectors;
  never infer a pool from ordinal data outside the session's description.
* Grant access in both directions required by an asynchronous copy. The copy
  preflight checks that each endpoint owner can directly access the other
  endpoint allocation.
* Keep host pointer copies behind their documented unsafe guarantees and only
  read device-produced bytes after a system-scope terminal completion.
* Drop or poll every `Pending` token to terminal state, and explicitly drain
  deferred retirements before session teardown when a timeout or queue fault can
  leave device work unresolved.
* Keep queues single-producer for the safe AQL methods. `MultiProducer` and
  `Cooperative` queues can be created only when discovery advertises the
  compatible kind, but `dispatch` and `dispatch_after` reject them.
* Keep kernel, executable, kernarg, queue, and allocation owners alive through
  the complete operation. The pending keepalive is the device-lifetime proof
  used by the execution boundary, not a substitute for caller validation.
* In the production executor, realize all queue, signal, vector, executable,
  kernarg, staging, and metric capacity before `init`; use the prepared methods
  in the loop and let the static resource destroy order release them afterward.

## Source map

| Contract area | Implementation |
| --- | --- |
| Fault record and queue configuration | `hsa/src/session.rs:22-133` |
| Session fields and `into_session` | `hsa/src/session.rs:135-200` |
| Dynamic memory counter, health, and queue construction | `hsa/src/session.rs:202-335` |
| Queue core destruction and public accessors | `hsa/src/session.rs:338-409` |
| Signal pool, pending keepalives, retirement, and prepared tokens | `hsa/src/execution.rs:29-390`, `1282-1499` |
| Allocation creation, grants, and session allocation methods | `hsa/src/execution.rs:394-739` |
| HSACO, executable, and kernel ownership | `hsa/src/execution.rs:772-1060` |
| Async copies | `hsa/src/execution.rs:1460-1651` |
| Queue ring publication, geometry, and dispatch | `hsa/src/execution.rs:1653-2305` |
| Direct smoke caller | `hsa/examples/execute_smoke.rs:1-136` |
| Probe realization and benchmark caller | `native-probe/src/hsa.rs:307-568`, `738-830` |
| Borrowed production bindings | `native-probe/src/bindings.rs:65-234`, `319-414` |
| Static production resource realization and teardown | `native-executor/src/hsa.rs:384-711`, `1808-2050`, `2571-2606` |
| Cross-backend staged HSA legs | `native-executor/src/bridge.rs:305-333`, `1508-1581` |
