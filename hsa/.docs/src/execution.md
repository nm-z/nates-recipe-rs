# `recipe_hsa::execution`

```yaml
document: recipe_hsa.execution
source: hsa/src/execution.rs
kind: ROCr/HSA execution ownership and submission contract
authority:
  - hsa/src/execution.rs
  - hsa/src/session.rs
  - hsa/src/runtime.rs
  - hsa/src/abi.rs
  - hsa/src/loader.rs
  - native-executor/src/hsa.rs
  - native-executor/src/bridge.rs
  - native-probe/src/hsa.rs
  - hsa/examples/execute_smoke.rs
```

This document explains the complete execution boundary owned by
`hsa/src/execution.rs`. It follows discovered ROCr objects from session setup
through memory access, code-object loading, asynchronous copy, AQL packet
publication, completion observation, output collection, and teardown. The
source line references are anchors for the current checkout. Names and
state transitions are the contract; line numbers identify the implementation
that enforces them.

The module is a scoped, explicit adapter over the reviewed large-model ROCr
ABI. It does not discover topology, schedule a graph, compile a kernel, or
infer dependencies. Callers provide the measured agent description, resolved
memory pools, code object bytes, geometry, and immutable task assignments.
The module owns the native resources and makes invalid ownership, lifetime,
queue, memory, and dispatch states observable as typed errors.

## 1. Execution boundary

The public execution objects form one ownership pipeline:

```text
Runtime::open / Runtime::discover
        |
        v
DiscoveredAgent::into_session
        |
        +--> Session::allocate* / grant_access* --> Allocation
        |
        +--> Session::load_hsaco --> Executable::kernel --> Kernel
        |
        +--> Session::create_queue --> Queue
        |
        +--> Session::copy_async or prepare_pending --> Pending / PreparedPending
        |                                                   |
        +----------------------------------------------- Queue::dispatch*
                                                            |
                                                            v
                                                   Poll / wait / retirement
```

`Runtime` is the balanced ROCr initialization owner (`runtime.rs:10-76`). A
`Session` borrows the runtime and owns the per-agent signal pool, fault state,
and the exact discovered pool handles (`session.rs:135-143`). `execution.rs`
adds the resource and submission operations to that session. Rust lifetimes
prevent a child from outliving the runtime it calls, while `Rc` and `Arc`
keep each native object alive across asynchronous device work.

The module exports these execution-facing values through `hsa/src/lib.rs`:

| value | role |
| --- | --- |
| `Allocation` | one ROCr pool allocation, with checked host copies and access metadata |
| `Executable`, `Kernel`, `KernelMetadata` | a frozen agent executable and one resolved kernel entry |
| `DispatchGeometry` | dimensions, grid, workgroup, dynamic segment bytes, and barrier bit |
| `Pending`, `Dependency`, `PreparedPending` | completion ownership and optional device-side dependency inputs |
| `PollStatus`, `WaitOutcome` | nonblocking and bounded-wait observations |
| `QueueProgress`, `dependent_dispatch_packet_count` | bounded AQL capacity and dependency-tree sizing |
| `RetirementReport` | explicit progress over deferred signals and keepalives |

`Queue` and `Session` are declared in `session.rs`, but their execution
methods are implemented here. The division keeps queue ownership and callback
registration next to the raw queue object while keeping packet construction,
signals, and memory operations in one execution implementation.

## 2. Native ownership and lifetime model

### 2.1 Runtime and session

`Runtime::open` dynamically resolves every ABI symbol and calls `hsa_init`.
`Runtime::discover` requires `active` and returns records borrowing that
runtime. `Runtime::close` consumes the owner and calls `hsa_shut_down`; `Drop`
is the status-discarding fallback (`runtime.rs:18-76`). A runtime cannot be
closed while a discovery record or session is borrowed, so execution never has
to guess whether its function pointers are still valid.

`DiscoveredAgent::into_session` is the first execution gate
(`session.rs:145-200`):

1. the runtime must still be active;
2. the selected agent must be a GPU;
3. the GPU must advertise kernel dispatch;
4. every discovered ISA must have an exact AMD target; and
5. the runtime timestamp frequency is queried for bounded signal waits.

The method then creates a fresh `SharedFault` and `SignalPool` for this exact
agent. A session is not a process-global device context. Two sessions have
independent fault and signal state even when they refer to the same runtime.
`Session::available_memory_bytes` is a live ROCr counter query, not a cached
discovery value; it checks runtime activity and session health before reading
`AMD_AGENT_INFO_MEMORY_AVAIL` (`session.rs:202-228`). The native preparation
layer uses this query when it accounts current HSA capacity.

### 2.2 Resource owners

| Rust object | Native object(s) | Ownership rule |
| --- | --- | --- |
| `SignalPool` | reusable `hsa_signal_t` handles and deferred records | pool state is `Rc` and is scoped to one session |
| `Allocation` | one pointer from one discovered `hsa_memory_pool_t` | `AllocationInner` frees exactly once; clones keep it live |
| `Queue` | one `hsa_queue_t`, callback context, ring and doorbell | `QueueCore` is shared by pending keepalives; close requires uniqueness |
| `Executable` | HSACO backing bytes, code-object reader, frozen executable | destroy executable, then reader, then backing bytes |
| `Kernel` | copied symbol name, object handle, metadata, executable `Rc` | a kernel keeps its executable alive |
| `Pending` | one completion signal plus all operation references | incomplete drop moves both to deferred retirement |
| `PreparedPending` | one signal plus pre-reserved keepalive vectors | `Ready -> Active -> terminal -> Ready` for loop reuse |

`Allocation`, `Executable`, `Kernel`, `Pending`, and `PreparedPending` carry a
`PhantomData` session/runtime borrow even though the native fields are stored in
reference-counted internals. This prevents a pointer, code object, or signal
from being used after its runtime lifetime in safe Rust.

`PendingKeepalive` retains the queue, executable, allocations, and dependency
signals referenced by an asynchronous operation (`execution.rs:64-81`). The
completion signal is not enough to keep those objects alive by itself. The
keepalive is released only after system-scope terminal completion, and it is
released before the completion signal can be recycled (`execution.rs:168-173`).

## 3. Lifecycle from setup to teardown

The execution lifecycle has a strict pre-loop boundary:

```text
open runtime
  -> discover exact agents and pools
  -> create GPU Session and signal pool
  -> allocate arenas, staging, kernargs, metrics, scratch
  -> grant exact cross-agent access
  -> load and freeze HSACO images, resolve entries
  -> create single-producer queues
  -> prepare one completion token per finalized task
  -> init admission transfers
  -> repeated loop calculation/transfer/metric submissions
  -> exit transfers and host output collection
  -> drain retirements
  -> close queues, kernels, executables, allocations
  -> drop Session and close Runtime
```

Discovery, compilation, allocation, loading, and queue creation are not model
work. They are preparation. In the native executor, the HSA backend performs
them in `HsaResources::realize` and `realize_device`
(`native-executor/src/hsa.rs:384-580`, `1808-2050`), before the executor enters
`init`. Submission and poll paths use only those resources.

### 3.1 Native executor phase mapping

The backend maps the closed executor work classes to this module:

| Recipe work | HSA operation | completion action |
| --- | --- | --- |
| `InitAdmission` | `Session::copy_async_prepared` from fine host staging into the device arena | no extra host result |
| `InternalTransfer` | `copy_async_prepared` between two resolved ranges in HSA arenas | no extra host result |
| `Calculation` | `Queue::dispatch_prepared` with a prefilled kernarg slot | no extra host result |
| `Metric` | `copy_async_prepared` of exactly four bytes into a preallocated fine host buffer | decode F32 or I32 after completion |
| `ExitTransfer` to device | `copy_async_prepared` to the resolved destination range | no extra host result |
| `ExitTransfer` to external | `copy_async_prepared` to fine staging, then copy staging into an egress `Vec<u8>` | `collect_exit` copies the egress bytes to caller storage |

These calls are in `native-executor/src/hsa.rs:713-1038`. `HsaBackend::submit`
validates the immutable task contract before selecting a method. The HSA
backend opts into loop repetition (`supports_loop_repetition: true`) and
calls `PreparedPending::reset` only after terminal completion
(`native-executor/src/hsa.rs:1040-1084`, `1381-1475`).

`HsaResources::destroy` first drops the unused prepared-token pool, then
`destroy_devices` requires every completion slot to be available, drains each
session's deferred retirements, closes queues, drops kernel references, closes
executables, closes kernarg and metric allocations, closes staging, and frees
optional scratch (`native-executor/src/hsa.rs:699-711`, `2571-2605`). This order
is required because queue and executable close operations reject live dependent
references.

### 3.2 Direct smoke and probe callers

`hsa/examples/execute_smoke.rs` is the smallest direct user path. It discovers
one CPU and one GPU, creates a GPU session, allocates CPU fine and GPU coarse
buffers, grants access in both directions, writes host bytes, submits and waits
for host-to-device and device-to-host copies, reads the completed host result,
and verifies a byte-for-byte round trip. Its optional HSACO path loads an
executable, resolves a symbol, allocates and initializes kernargs, creates a
single-producer queue, submits a prerequisite copy plus `dispatch_after`, waits
for both signals, verifies a kernel-produced output, and closes queue, kernel,
executable, and allocations (`hsa/examples/execute_smoke.rs:1-128`).

`native-probe/src/hsa.rs` uses the same public operations for measured evidence:
it bounds allocations from the discovered capacity, uses `copy_async` for H2D,
D2H, and D2D measurements, verifies every transfer, builds a Recipe-owned
HSACO, loads and resolves its kernel, initializes a CPU kernarg allocation,
dispatches through a minimum-size single-producer queue, downloads and verifies
the result, and explicitly closes the queue, executable, kernel, and kernarg
(`native-probe/src/hsa.rs:330-568`). Its `complete_hsa` helper polls without
releasing a live token; a timed-out operation remains polled at a capped rate
until terminal completion so a driver operation never loses its live resources
(`native-probe/src/hsa.rs:790-851`).

The staged cross-backend bridge uses `copy_async_prepared` for its HSA source
and destination legs. It keeps a prepared HSA token in an `ActiveLeg::Hsa`,
polls it as a normal `BackendPoll`, and resets it only after terminal completion
(`native-executor/src/bridge.rs:1508-1635`).

## 4. Signal pool and deferred retirement

### 4.1 State and reservation accounting

`SignalPoolState` contains four pieces of session-scoped state
(`execution.rs:34-47`):

| field | meaning |
| --- | --- |
| `available` | terminal signals reset to value `1`, ready for reuse |
| `retired` | submitted signal plus `PendingKeepalive` awaiting terminal value |
| `retirement_reservations` | signals already acquired or being rearmed, reserving future retired-vector capacity |
| `fatal_signal` | first negative completion observed by this pool |

`SignalPool::acquire` first performs a nonblocking retirement collection, then
rejects a pool with `fatal_signal`. It reserves capacity for one more deferred
record before creating or reusing a signal. A new signal starts at value `1`
and must have a nonzero handle (`execution.rs:187-239`). Capacity reservation
is the reason a later `retire` cannot allocate after an operation has been
published.

`SignalPool::rearm` repeats the same capacity reservation, marks a terminal
record reserved, clears its cached terminal value, and stores `1` with release
ordering (`execution.rs:247-279`). `PreparedPending::reset` is the only public
path that calls rearm.

### 4.2 Submission and completion sequence

```text
acquire signal(value = 1, reservation = held)
  -> build keepalive and packet/copy arguments
  -> publish operation
  -> reservation moves to retired set
  -> device changes signal to 0 or a negative value
  -> collect_retired observes terminal value with acquire load
  -> release keepalive first
  -> mark signal terminal
  -> negative: destroy and poison pool
     zero: reset to 1 and place in available pool
```

For a `Pending` token, the signal remains in the token while active. `Drop`
moves an incomplete token into the explicit retired set, preserving every
`Rc` referenced by its keepalive (`execution.rs:1267-1279`). A terminal token
releases the keepalive and lets `SignalRecord::Drop` recycle or destroy the
signal.

`SignalRecord::Drop` is deliberately fail-closed (`execution.rs:122-166`):

* a positive value outside `retired` is an ownership invariant violation. The
  module logs a terminal-resource leak and does not destroy a signal the device
  may still reference;
* a negative value is terminal and is destroyed, never recycled;
* zero is recycled into `available` when its vector can reserve one slot, or
  destroyed if the pool is gone or reservation fails.

`SignalPool::collect_retired` scans for any signal at or below zero, removes it
under the state borrow, records the first negative value, and then drops the
keepalive before the signal record. It returns `RetirementReport` with counts,
the first failed signal, and the shared queue-fault state
(`execution.rs:281-318`). The scan is nonblocking and may reclaim records in a
different order from submission; each record still retains its own resources.

`Session::poll_retirements` exposes one such pass. `drain_retirements` repeats
polling until empty, a still-pending set is poisoned, or the caller's duration
expires. It waits on only the first retired signal in chunks of at most one
millisecond and never drops unresolved records on timeout
(`execution.rs:741-769`). A drained negative signal returns
`Error::AsyncSignal`. When records remain pending, a poison or timeout returns
`Error::DeferredRetirement` with the unresolved count and poison bit. A fully
drained, nonfailed set returns its report, including a poison bit if the queue
callback arrived after the last signal was reclaimed.

When the signal pool itself drops, available signals are destroyed. Retired
signals that are already terminal are released normally. Positive unresolved
records are intentionally forgotten and logged because runtime shutdown may
still be pending; guessing that the device stopped would permit use-after-free
(`execution.rs:362-391`). The documented safe teardown is to call
`Session::drain_retirements` before dropping the session.

### 4.3 Asynchronous queue faults

Each queue receives a stable boxed callback context from `session.rs`. The
callback records status, optional source queue ID, and an incrementing epoch in
`SharedFault` without taking a lock or allocating (`session.rs:29-112`). The
fault is permanent for that session:

* `Session::ensure_healthy` returns `Error::SessionPoisoned` after the callback;
* `Pending::poll` checks the fault while its signal is positive, retaining the
  token nonterminal so `Drop` defers its resources;
* after a signal reaches zero, `Pending::poll` releases operation references,
  records terminal success, then checks the fault and may return
  `SessionPoisoned`;
* `SignalPool::collect_retired` reports the callback poison independently of a
  signal's value.

An asynchronous negative signal is a separate terminal failure. It sets the
pool's `fatal_signal`, makes later signal acquisition fail, and returns
`Error::AsyncSignal { value }`. The native executor translates either a
session-poisoned result, a poisoned deferred-retirement result, or a negative
async signal into its permanently poisoned HSA backend state
(`native-executor/src/hsa.rs:1146-1156`, `2486-2493`).

## 5. Memory allocation, access, and copies

### 5.1 Pool selection and allocation

`allocate_from` is the one implementation used by both `DiscoveredAgent` and
`Session` (`execution.rs:525-578`, `623-708`). It requires an active runtime,
valid parallel raw-pool and description indices, a pool that advertises runtime
allocation, and a nonzero size not above the discovered aggregate maximum. It
calls `hsa_amd_memory_pool_allocate` with standard zero flags, rejects a null
pointer, and records the owner agent, pool index, global flags, and an initial
access set containing only the owner.

Convenience methods choose the first discovered global, allocatable pool with
the requested flag:

| method | required pool flag |
| --- | --- |
| `allocate_coarse` | `COARSE_GRAINED` |
| `allocate_fine` | `FINE_GRAINED` or `EXTENDED_SCOPE_FINE_GRAINED` |
| `allocate_kernarg` | `KERNARG_INITIALIZATION` |

`matching_pool` does not synthesize a fallback pool. Missing capabilities return
`Error::NoMatchingMemoryPool`. `AllocationInner::destroy` consumes its pointer
before calling the matching free, so a repeated drop cannot double-free
(`execution.rs:394-434`). `Allocation::close` requires the inner `Rc` to be
unique; otherwise it drops its clone and returns `Error::ResourceBusy`.

### 5.2 Exact access grants

`grant_access` calls `hsa_amd_agents_allow_access` with a nonempty discovered
agent list. ROCr treats this API as replacement, not accumulation. On success,
the allocation records exactly the owner plus the supplied agents
(`execution.rs:595-621`). A later grant therefore replaces the remembered set;
callers must supply the complete desired set.

`DiscoveredAgent::grant_access` grants one agent. `Session::grant_access` is the
same operation for the session's GPU agent. `Session::grant_access_exact_set`
accepts any number of same-runtime sessions and discovered agents, validates
that all objects belong to the same runtime and API, sorts and deduplicates the
handles, and replaces the set (`execution.rs:701-739`). The list must fit the
ABI's `u32` count field and cannot be empty. The pool owner is always retained
even when it is not repeated in the argument list.

`copy_async` requires mutual direct access: the source allocation must list the
destination owner and the destination must list the source owner. This is why a
GPU arena allocated from a GPU pool is granted to the CPU host allocator and
all participating sessions before it is used by the native executor
(`native-executor/src/hsa.rs:382-487`).

### 5.3 Host copies

`Allocation::copy_from_host` and `copy_to_host` are unsafe because the module
cannot prove pool accessibility, coherence, or absence of concurrent device
use. Both check `offset + size` with checked arithmetic before using
`copy_nonoverlapping` (`execution.rs:458-497`). The caller must guarantee:

* the pool permits host access in the requested direction;
* no device operation concurrently reads or writes the range for a host write;
* a producer has reached system-scope terminal completion before a host read.

The only range failure is `Error::CopyOutOfBounds`, which reports buffer name,
offset, size, and capacity. Zero-length synchronous host copies are accepted if
the range is valid; zero-byte asynchronous copies are rejected because their
completion semantics are ambiguous.

### 5.4 Asynchronous copies

`Session::copy_async` allocates a completion signal, validates both ranges and
access sets, stores source and destination allocation `Rc`s in a keepalive,
and calls `hsa_amd_memory_async_copy` with no dependency array
(`execution.rs:1577-1650`). A successful call returns an active `Pending`.
An API failure releases the unsubmitted signal. If a queue callback poisoned
the session immediately after publication, the operation is retired rather
than releasing live allocations.

`Session::copy_async_prepared` is the allocation-free submission counterpart
(`execution.rs:1501-1575`). It verifies that the token belongs to this session,
rejects zero bytes, checks ranges and mutual access, obtains its signal and
pre-reserved keepalive through `PreparedPending::begin(2, 0)`, and uses
`Api::check_status_only` so a live-loop failure stores only a numeric HSA status
instead of allocating a diagnostic string. Failed publication restores the
token to `Ready`; a post-publication poison retires it.

Neither copy method uses an AQL queue. ROCr's AMD asynchronous-copy API owns
the transfer and writes the supplied completion signal. Queue capacity and
single-producer discipline apply to kernel and barrier packets only.

## 6. HSACO loading and kernel metadata

`Session::load_hsaco` is preparation-time code-object realization
(`execution.rs:974-1060`):

1. require an active, healthy session and reject an empty byte slice;
2. copy bytes into an `Arc<[u8]>` retained by `ExecutableInner`;
3. create a code-object reader from that memory;
4. create an executable with the discovered profile and nearest-even rounding;
5. load the code object for the session's exact GPU agent;
6. freeze the executable; and
7. recheck session health before returning `Executable`.

Every failure after a reader or executable is created invokes the matching
destruction order. `ExecutableInner::destroy` destroys the executable before
the reader, then drops the HSACO backing bytes. If executable destruction fails,
the reader and backing bytes are leaked rather than risking a reader use-after-
free. If reader destruction fails, the backing bytes are leaked
(`execution.rs:772-815`). `Executable::close` requires no live `Kernel` or
pending keepalive references.

`Executable::kernel` converts the requested name to a NUL-terminated
`CString`, looks it up for the session agent, verifies symbol kind
`SYMBOL_KIND_KERNEL`, reads the kernel object and all resource attributes, and
rejects a zero object or a zero/non-power-of-two kernarg alignment
(`execution.rs:845-944`). The returned `Kernel` owns an `Rc` of the executable,
so an entry cannot outlive its frozen image.

`KernelMetadata` is the dispatch contract:

```text
kernarg_segment_size
kernarg_segment_alignment
group_segment_size
private_segment_size
dynamic_callstack
```

The native executor compares this metadata with inspected immutable artifact
ABI data before binding a task (`native-executor/src/hsa.rs:1937-1987`). It
derives dynamic private bytes from the finalized resource envelope when the
kernel has a dynamic callstack (`native-executor/src/hsa.rs:2052-2115`).

## 7. Completion tokens

### 7.1 Signal classification and dependencies

ROCr completion values have one interpretation in this module
(`execution.rs:1097-1111`):

| raw value | state | public result |
| --- | --- | --- |
| `> 0` | pending | `PollStatus::Pending`, unless the session fault is already poisoned |
| `0` | complete | `PollStatus::Complete`, unless the session fault is poisoned |
| `< 0` | terminal failure | `Error::AsyncSignal { value }` |

`Dependency` is a clonable borrow of a signal record, not a new signal. Clones
keep the record and its pool alive and cannot recycle it. `Dependency::poll`
performs an acquire load and applies the table above without consuming the
dependency (`execution.rs:1113-1155`). A dependency from another session is
rejected by `dispatch_after` before its handle reaches a barrier packet.

### 7.2 Dynamic `Pending`

`Pending::poll` is nonblocking (`execution.rs:1188-1230`). A terminal cached
result is returned repeatedly. While positive, it checks the shared fault but
leaves the token nonterminal on error, ensuring `Drop` defers the signal and all
resources. On zero it marks the signal terminal, releases the retirement
reservation, clears the keepalive, then checks the fault. On a negative value it
marks the pool fatal, clears the keepalive, caches an async error, and returns
that error.

`Pending::wait` is a bounded host wait, not an unbounded sleep
(`execution.rs:1232-1264`). It polls first, computes the caller deadline, and
requests ROCr active waits in chunks no larger than one millisecond. HSA wait
timeouts are documented as hints, so the elapsed host deadline is checked on
every iteration. `duration_to_ticks` converts nanoseconds using the discovered
frequency, saturates arithmetic, and clamps to at least one tick
(`execution.rs:1455-1458`).

Dropping a nonterminal `Pending` is safe and asynchronous: the signal and
keepalive enter the pool's retired set. Dropping a terminal one releases both
immediately. There is no implicit blocking wait in `Drop`.

### 7.3 Prepared `PreparedPending`

`Session::prepare_pending(allocation_capacity, dependency_capacity)` performs
all signal creation and vector reservation before `init`
(`execution.rs:1460-1499`). The token state is:

```text
Ready { signal, reserved keepalive vectors }
  -- begin --> Active(Pending)
  -- terminal poll --> reset --> Ready
  -- drop while Ready --> release_unsubmitted
  -- drop while Active --> deferred retirement
  -- failed or consumed --> Consumed
```

`begin` temporarily marks the token `Consumed`, verifies that the requested
keepalive counts fit existing capacities, clears old references, and restores
`Ready` on a capacity error (`execution.rs:1318-1348`). It cannot acquire a
signal, grow a vector, or allocate while submitting. `dispatch_prepared`
requires one allocation slot when kernargs are supplied. `copy_async_prepared`
requires two allocation slots and no dependency slots.

`PreparedPending::poll` rejects `Ready` as not submitted and `Consumed` as
busy. `dependency` is available only in `Active`. `reset` is idempotent for a
never-submitted `Ready` token, rejects an active positive signal as busy, and
rearms only after a terminal successful poll. A failed poll leaves `Active` so
the caller can preserve the unresolved operation. Dropping `Ready` marks its
signal terminal before releasing it; no positive signal is left outside the
retirement set (`execution.rs:1370-1453`).

This prepared state is the bridge to Recipe's immutable `init -> loop -> exit`
lifecycle. The native executor creates one token per finalized task in
`prepare_pending_pool` (`native-executor/src/hsa.rs:1757-1775`) and reuses loop
tokens without device-object creation or host collection growth.

## 8. Queue ownership and AQL publication

### 8.1 Queue creation contract

`Session::create_queue` is implemented in `session.rs:230-335` and is a
preparation operation. It requires discovered queue capabilities, a power-of-
two size in the exact `[minimum_packets, maximum_packets]` range, and a kind
compatible with the advertised protocol. The callback context is boxed before
the ROCr call and retained in `QueueCore` until `hsa_queue_destroy` returns.
`QueueConfig::new` leaves private and group segment limits at `u32::MAX`, the
ROCr sentinel for its normal limit; callers can replace those fields when an
explicit queue limit is part of the measured contract (`session.rs:114-133`).

The returned queue fields are validated again: nonnull ring base, nonzero
power-of-two actual size, kernel-dispatch feature bit, and a recognized actual
kind. A malformed queue is destroyed immediately. If destruction fails, the
callback context is leaked because ROCr may still call it. The safe packet
publication methods additionally require the requested kind to be
`QueueKind::SingleProducer`, even if ROCr reports a multi-producer protocol in
the read-only queue field.

`Queue::close` requires the queue core's `Rc` to be unique. Pending keepalives
retain a queue core until their completion signal reaches terminal state, so a
premature close returns `Error::ResourceBusy` rather than destroying a live
queue.

### 8.2 Ring occupancy and bounded capacity

`HsaQueueIo` is the sole `QueueIo` implementation
(`execution.rs:1653-1838`). Queue creation has already established a nonnull
power-of-two ring whose slots are exactly 64 bytes. Occupancy loads the write
index relaxed and the read index with sequentially consistent acquire, then
uses wrapping subtraction (`execution.rs:1683-1691`).

`enqueue_packets` checks that occupied slots plus the requested packet count do
not exceed the queue size before writing any body. Each packet is then published
in order:

1. write the 64-byte body while its header is `PACKET_TYPE_INVALID`;
2. release-store the final valid header, making the body visible;
3. release-store the next write index; and
4. release-store the doorbell index.

No fallible construction or allocation is allowed after this sequence starts.
If capacity is insufficient, no packet is written and `Error::QueueFull`
reports the observed indices and size (`execution.rs:1693-1715`).

`Queue::progress_capacity` performs at most a caller-provided nonzero number of
nonblocking occupancy probes. It never sleeps, waits, or publishes. A request
of zero or greater than the queue size is `Error::InvalidProgressRequest`.
Otherwise it returns `QueueProgress::Ready` with the available count and probe
number, or `Backpressured` with the final available count and required count
(`execution.rs:1717-1761`). The HSA native executor probes for one kernel
packet immediately before dispatch and maps backpressure to resource
contention (`native-executor/src/hsa.rs:923-937`).

### 8.3 Packet headers and layouts

`AqlPacket` accepts exactly the reviewed 64-byte kernel-dispatch and barrier-AND
layouts from `abi.rs:179-212`. Kernel headers contain packet type, the optional
barrier bit from `DispatchGeometry::barrier`, and system acquire/release fence
scopes. Barrier headers contain the barrier type and system fences
(`execution.rs:1840-1851`). The `AtomicU16` header store is the first aligned
field of every slot; packet body writes happen before it with release
publication.

The queue is host-thread-confined. There is no internal mutex or producer
serialization. Calling a packet method concurrently from multiple producers
violates the `QueueCore::kind` contract and is outside the safe API.

## 9. Device-side dependency trees

`Queue::dispatch_after` implements device-side dependencies without a host
wait (`execution.rs:2113-2305`). It first validates every dependency's session
identity and polls it, catching an already-negative signal or permanent queue
poison before publishing a barrier that could never be satisfied.

The HSA barrier packet has five dependency slots, so
`BARRIER_AND_FAN_IN` is five (`execution.rs:1853`). `barrier_packet_count`
repeatedly rounds the current level up by five until one completion remains.
`lower_barrier_packets` emits one barrier packet per chunk, stores intermediate
completion signals as the next level's dependencies, and leaves unused slots
as zero handles. The final barrier completion is the sole dependency of the
following kernel packet. `dependent_dispatch_packet_count` returns the tree
count plus one kernel packet with checked `u32` conversion
(`execution.rs:1867-1907`). For zero dependencies, the tree has zero packets
and the count is one.

All barrier signals are acquired before queue publication. The keepalive stores
the original dependencies and every intermediate barrier signal. After a
successful enqueue their retirement reservations are released and the local
records are dropped, but the keepalive clones remain until the kernel's terminal
signal. On enqueue failure, every acquired signal and keepalive is released as
unsubmitted. A dependency tree that needs more packets than the queue can hold
is rejected before any signal is acquired.

`dispatch_prepared` intentionally has no dependency argument. It is the
allocation-free path for a host-side DAG whose readiness has already been
established by the executor. If device-side dependencies are required during
realization, callers use `dispatch_after`; the prepared loop path does not
silently allocate a barrier tree.

## 10. Geometry, kernargs, and kernel dispatch

### 10.1 Geometry validation

`DispatchGeometry` has one to three active dimensions, three grid dimensions,
three workgroup dimensions, dynamic group/private byte counts, and a kernel
barrier bit. `one_dimensional` sets unused dimensions to one and clears dynamic
bytes and the barrier (`execution.rs:1062-1083`).

`validate_geometry` applies every discovered ISA limit
(`execution.rs:1909-1960`):

* dimensions must be 1, 2, or 3;
* every grid and workgroup component is nonzero;
* inactive components must equal one;
* a workgroup component cannot exceed its grid component;
* checked products of all workgroup components and all grid components must
  fit their packet field types; and
* each ISA must accept total and per-dimension workgroup and grid sizes.

The validation is against exact discovered limits, not a guessed vendor
default. A failure is `Error::InvalidDispatch`.

### 10.2 Kernarg and segment checks

`dispatch_prepared` and `dispatch_after` share the same kernarg validation:

* a kernel with zero kernarg size may receive `None`, which publishes a null
  kernarg pointer, or a supplied allocation that passes the same pool and
  access checks;
* a kernel with nonzero size must receive an allocation from a discovered pool
  carrying `KERNARG_INITIALIZATION`;
* the allocation must be directly accessible by the queue's agent;
* its length must cover `kernarg_segment_size`; and
* its address must satisfy the power-of-two metadata alignment.

Missing kernargs, a wrong pool, insufficient length, inaccessible agent, or
misalignment are `Error::InvalidDispatch`. A dynamic-callstack kernel must be
given a nonzero `dynamic_private_bytes` allowance. Static plus dynamic private
and group segment sizes are checked with overflow-safe additions. The packet
uses the resulting sizes, the frozen kernel object, kernarg address, grid,
workgroup, dimensions, and completion signal (`execution.rs:1962-2088`,
`2117-2305`).

### 10.3 Submission variants

`Queue::dispatch` is exactly `dispatch_after(..., &[])`; it allocates a normal
dynamic `Pending` and publishes one kernel packet. `Queue::dispatch_after`
allocates one completion signal plus any barrier signals and returns a dynamic
`Pending`. `Queue::dispatch_prepared` obtains the signal and keepalive from a
`PreparedPending`, publishes one dependency-free kernel packet, and activates
the token in place. If queue capacity fails, the prepared token returns to
`Ready`; if a callback poisons the session after publication, it enters
deferred retirement and the call returns the poison error.

The queue agent and executable agent must match. The token's signal pool must
be the queue session's pool. These identity checks prevent a signal or kernel
from crossing session/device boundaries even when Rust references happen to
have compatible lifetimes.

## 11. Public methods and their failure boundary

| method | performs | does not perform |
| --- | --- | --- |
| `Session::allocate*` | validate discovered pool and allocate | infer a pool if required flags are absent |
| `Session::grant_access*` | replace ROCr direct-access set and record exact handles | accumulate grants implicitly |
| `Allocation::copy_*_host` | checked host pointer copy under caller safety contract | synchronize a device operation |
| `Session::copy_async` | submit AMD async copy with a fresh signal | wait, use an AQL queue, or retain a dependency list |
| `Session::copy_async_prepared` | submit the same copy through an already realized token | allocate a signal, grow vectors, or format status text |
| `Session::load_hsaco` | create reader, executable, load, and freeze code object | compile or inspect Recipe artifacts |
| `Executable::kernel` | resolve and validate one frozen kernel symbol | launch it |
| `Session::prepare_pending` | create one signal and reserve keepalive capacities | submit work |
| `Queue::progress_capacity` | bounded occupancy probes | sleep, block, or publish |
| `Queue::dispatch*` | validate geometry/resources and publish AQL packets | schedule a graph or wait for dependencies on the host |
| `Pending::poll/wait` | observe one completion signal | release resources while a signal is positive |
| `Session::poll/drain_retirements` | reclaim explicit terminal records | guess that unresolved device work stopped |

## 12. Error taxonomy and invariants

### 12.1 Execution-owned `Error` variants

The variants below are constructed directly in `execution.rs`; lower-level
loader status failures remain `Error::Hsa` with the named ROCr operation.

| group | variants | invariant represented |
| --- | --- | --- |
| runtime and asynchronous health | `RuntimeClosed`, `SessionPoisoned`, `AsyncSignal`, `DeferredRetirement` | calls require an active, unpoisoned session; unresolved work is retained |
| signal creation and retirement | `NullSignal`, `AllocationFailed` | every acquired signal has a nonzero handle and a reserved retirement slot |
| memory identity and size | `InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, `NoMatchingMemoryPool`, `InvalidAllocationSize`, `NullAllocation`, `CopyOutOfBounds` | allocation is from an exact discovered pool and every range is bounded |
| ownership and close | `ResourceBusy` | queue, executable, or allocation still has a live `Rc` dependent |
| code object and symbols | `EmptyCodeObject`, `NameContainsNul`, `SymbolNotKernel`, `InvalidKernel` | code bytes and symbol metadata are valid before a `Kernel` exists |
| dispatch arguments | `InvalidDispatch` | session, agent, queue kind, geometry, kernarg, segment sizes, capacities, and token state agree |
| queue capacity | `QueueFull`, `InvalidProgressRequest` | publication is all-or-nothing and bounded by a valid queue size |

`Error::Hsa` is produced by `Api::check` for ordinary ROCr status failures.
Preparation paths include the runtime status string when it can be queried.
`copy_async_prepared` uses `Api::check_status_only` so a live submission path
returns the numeric status without allocating (`loader.rs:272-297`). Most
submission and code-object methods call `Session::ensure_healthy` before their
ABI operation. The low-level allocation and access-grant entry points only
require an active runtime, because they are the resource-building primitives;
they do not silently turn a grant or allocation into a submission. The caller
still must not proceed to a submission after a session fault.

### 12.2 Core invariants

1. **No positive signal outside retirement.** A submitted positive signal is
   owned by an active token or the explicit retired set. A destructor that sees
   a positive signal outside both records a terminal leak and never destroys it.
2. **Keepalive before signal reuse.** Queue, executable, allocation, and
   dependency references are dropped before a terminal signal is reset to one.
3. **One producer per safe queue.** Packet publication is exposed only for a
   queue requested as `SingleProducer`; the body/header/index/doorbell sequence
   relies on that confinement.
4. **Release publication.** Every packet body is written while invalid and is
   made visible by release header, write-index, and doorbell stores.
5. **Exact access knowledge.** The remembered access set is owner plus the most
   recent grant list. Copy submission requires both endpoint owners in both
   sets.
6. **Frozen kernel metadata.** Kernarg size, alignment, static segment sizes,
   dynamic-callstack behavior, and kernel object are read from the frozen
   executable and checked before packet publication.
7. **No implicit synchronization.** `copy_async`, `dispatch`, and
   `dispatch_after` return tokens. Only `poll`, `wait`, or explicit retirement
   APIs observe completion.
8. **No silent fallback.** Missing pools, invalid queues, queue pressure,
   malformed symbols, poisoned sessions, and failed teardown are surfaced as
   typed errors or explicit leak diagnostics. The module does not retry with a
   different pool, queue kind, packet layout, or runtime.

### 12.3 Caller safety obligations

The unsafe host-copy methods require callers to establish host accessibility,
coherence, bounds, and synchronization. The native executor satisfies those
obligations by allocating CPU fine-grained staging and kernarg pools, granting
GPU access before publication, and reading only after a terminal system-scope
signal. It also keeps all arena and executable references in `PendingKeepalive`
until terminal completion.

The caller must drain unresolved operations before closing a session. A
`ResourceBusy` close indicates a real dependent reference, not a condition to
work around by dropping or replacing the native object.

## 13. End-to-end traces

### 13.1 Host to GPU copy

```text
CPU agent allocate_fine
  -> GPU Session allocate_coarse
  -> Session::grant_access(host allocation)
  -> CPU agent grant_access(device allocation)
  -> unsafe host copy into CPU allocation
  -> Session::copy_async(device, host, bytes)
       acquire signal = 1
       keep source and destination Rc
       hsa_amd_memory_async_copy
  -> Pending::poll or wait
       signal > 0: Pending
       signal = 0: release keepalive, recycle signal, Complete
  -> unsafe host read only after Complete
```

The reverse copy uses the same path with endpoints exchanged. A device-to-
device copy needs both GPU allocations to list both endpoint owners. The
executor obtains this set with `grant_access_exact_set` during arena
realization.

### 13.2 Dependency-aware kernel dispatch

```text
copy Pending A
  -> A.dependency() gives a shared signal record
  -> queue.dispatch_after(kernel, kernarg, geometry, [A])
       validate same session and poll A
       acquire completion signal and barrier signal
       lower one barrier-and packet
       build keepalive(A signal, barrier signal, queue, executable, kernarg)
       publish barrier packet, then kernel packet
  -> queue reaches kernel only after barrier completion
  -> Pending B poll/wait reaches terminal
  -> drop B keepalive, then barrier and completion signals recycle
```

The host does not wait for A before submitting B. A negative A signal or
session poison is rejected before publication. A positive A signal is retained
by the barrier keepalive until the device consumes it.

### 13.3 Prepared calculation in the native loop

```text
preparation: load HSACO, resolve Kernel, allocate kernarg, create queue,
             Session::prepare_pending(2, 0)
loop iteration:
  fill preallocated kernarg bytes and host-copy them to kernarg memory
  queue.progress_capacity(1, probe_budget = 1)
  queue.dispatch_prepared(kernel, Some(kernarg), geometry, token)
  poll token until Complete
next iteration:
  token.reset()  // reuses signal and vector capacity
```

The HSA backend fills ABI arguments from finalized arena addresses and loop
metadata before calling `dispatch_prepared`
(`native-executor/src/hsa.rs:885-988`, `2306-2389`). It never creates a queue,
executable, allocation, or dynamic dependency tree in the loop.

### 13.4 Exit output

For a device-to-external `ExitTransfer`, the backend copies the resolved device
range into preallocated fine staging with `copy_async_prepared`. Terminal
completion runs `finish_action`, which copies staging into the task's egress
`Vec<u8>` (`native-executor/src/hsa.rs:809-883`, `2517-2561`). The executor then
invokes `collect_exit`, which validates task, phase, slots, source, and byte
count and copies the egress bytes into caller-owned output storage
(`native-executor/src/hsa.rs:628-656`). Metric completion follows the same
copy path but reads exactly four bytes and decodes the declared F32 or I32
type. `TaskKind::Metric` therefore remains a specialized transfer, not a
separate execution primitive.

## 14. Non-goals and deliberate limits

* The module does not provide a scheduler or host-side DAG. The executor owns
  readiness; `dispatch_after` is the explicit device-side barrier option.
* The module does not provide multi-producer packet publication. A queue whose
  requested kind is not `SingleProducer` is rejected by the safe dispatch API.
* The module does not turn queue pressure into a blocking retry. Callers may
  perform a bounded `progress_capacity` probe and choose their scheduler
  action from the observed result.
* The module does not silently recover a poisoned session. Faults and negative
  signals remain visible and prevent new submissions.
* The module does not infer memory locations or grant sets. Access must be
  explicitly established from discovered agents and pools.
* The module does not compile or rewrite HSACO. It loads caller-supplied bytes,
  freezes them for one exact agent, and checks symbol metadata.
* The module does not force a wait during destruction. Incomplete work is
  retained in deferred retirement, and unresolved work at pool drop is logged
  as a safety leak.

The resulting contract is intentionally small: preparation realizes every
object reachable from a task, submission publishes one already validated copy
or packet, completion observes one signal, and teardown releases only objects
whose native users have reached terminal state.
