# Host runtime

The runtime in [`host/src/runtime.rs`](../../src/runtime.rs) is Recipe's
preallocated asynchronous copy engine for host-owned storage. It moves bytes
between the [`Arena`](../../src/arena.rs) backings that represent RAM and
disk. It does not calculate payloads, discover devices, choose paths, allocate
arenas, or account for executor calls. Those responsibilities belong to the
host backend, planner, and executor around it.

The runtime's operation lifecycle is:

1. `Runtime::new(RuntimeConfig)` allocates every job slot, worker staging
   buffer, worker thread, and synchronization object needed by the engine.
2. `Runtime::prepare_copy()` claims one fixed slot and returns a
   `PendingCopy` token.
3. `PendingCopy::submit(...)` validates two arena ranges and publishes one
   copy job to the worker pool.
4. `PendingCopy::poll()` reports `Pending`, `Complete`, or the worker error.
5. The owner resets a completed token for loop reuse, or closes the runtime
   after all work has reached a terminal state.

The runtime is intentionally bounded. A slot is claimed once from the
runtime's monotonic slot index. Resetting a `PendingCopy` returns that same
slot to `PREPARED`; it does not make another call to `Runtime::prepare_copy`
possible. The host backend therefore sizes the slot table from the finalized
host task set and pre-realizes the pending tokens required by the executor.

The source-level function map is deliberately small and closed:

| Source region | Intent |
| --- | --- |
| `SlotCapacity::{new,get}` | Validate and expose the fixed slot count. |
| `RuntimeConfig::{new,worker_threads,slots,staging_bytes_per_worker}` | Validate and expose worker and staging limits. |
| `Runtime::{new,config,state,prepare_copy,close,shutdown}` | Allocate, observe, claim, and close the worker pool. |
| `PendingCopy::{submit,poll,reset}` | Publish, observe, and reuse one slot. |
| `validate_range`, `host_range` | Keep arena and host-index arithmetic checked. |
| `notify_worker`, `allocate_staging`, `worker_loop` | Wake and run the bounded workers. |
| `execute_slot`, `execute_copy`, `copy_ram_to_ram`, `copy_ram_slices`, `copy_disk_to_disk` | Dispatch and execute the four backing pairs. |
| `read_exact_at`, `write_all_at` | Complete positional disk I/O or return a worker error. |
| `fmt::Debug` and `Drop` implementations | Expose bounded state for diagnostics and guarantee worker joins. |

The current implementation is organized in these source ranges:

| `runtime.rs` lines | Structure |
| ---: | --- |
| 17-90 | Slot constants, `SlotCapacity`, `RuntimeConfig`, `RuntimeState`, and `PollStatus`. |
| 92-126 | `Job`, `JobSlot`, and `Shared` storage. |
| 128-257 | `Runtime` construction, admission, state, shutdown, debug, and drop. |
| 259-344 | `PendingCopy` submission, polling, reset, and debug. |
| 346-427 | Range validation, wake notification, staging allocation, and worker loop. |
| 429-549 | Job extraction and RAM/disk copy dispatch. |
| 551-600 | Host-index conversion and exact positional I/O helpers. |

## Public contract

| Item | Meaning | Exact validity rule |
| --- | --- | --- |
| `SlotCapacity` | Number of one-time runtime job slots | `SlotCapacity::new(value)` rejects `0` with `InvalidConfiguration("job slot capacity must be nonzero")`. |
| `RuntimeConfig::worker_threads` | Requested worker count | `RuntimeConfig::new` rejects `0` with `InvalidConfiguration("worker thread count must be nonzero")`. |
| `RuntimeConfig::slots` | Fixed slot table capacity | Supplied as a valid `SlotCapacity`; `Runtime::new` creates exactly this many `JobSlot` values. |
| `RuntimeConfig::staging_bytes_per_worker` | Private disk-to-disk scratch capacity for each worker | `RuntimeConfig::new` rejects `0` with `InvalidConfiguration("per-worker disk staging capacity must be nonzero")`. |
| `RuntimeConfig::worker_threads()` | Requested count, not necessarily spawned count | The spawned count is `min(worker_threads, slots.get())`. |
| `RuntimeConfig::slots()` | The configured `SlotCapacity` | Returns the copied configuration value. |
| `RuntimeConfig::staging_bytes_per_worker()` | Per-worker scratch size | Returns the copied configuration value. |
| `RuntimeState` | Runtime lifecycle observation | `Ready`, dynamically observed `Poisoned`, or `Closed`. |
| `PollStatus` | One copy-token observation | `Pending` while queued or running, `Complete` after a successful copy. |

`RuntimeConfig` and `SlotCapacity` are `Copy`, `Clone`, `Debug`,
`PartialEq`, and `Eq`. `Runtime` is not cloneable or copyable because it owns
the worker join handles. `PendingCopy` is the caller's capability for one
slot and is also not cloneable or copyable.

## Runtime-owned data

The source has four private layers. Their fields are the synchronization and
ownership contract, not an alternate public API.

| Type | Fields | Purpose |
| --- | --- | --- |
| `Job` | `source: Arc<Backing>`, `source_offset: u64`, `destination: Arc<Backing>`, `destination_offset: u64`, `bytes: u64` | Immutable copy description. Cloning the backing `Arc`s keeps RAM storage or the temporary disk file alive until the worker has finished using it. |
| `JobSlot` | `state: AtomicU8`, `job: Mutex<Option<Job>>`, `failure: Mutex<Option<Error>>` | One state machine cell, one published job, and one terminal worker error. |
| `Shared` | `slots`, `wake_generation`, `wake_lock`, `wake`, `stopping`, `poisoned` | State shared by all workers and by every `PendingCopy` token. |
| `Runtime` | `config`, `shared`, `workers`, `next_slot`, `state` | Owns the fixed resources and joins all worker threads on close or drop. |
| `PendingCopy` | `slot: Arc<JobSlot>`, `shared: Arc<Shared>` | A lightweight handle to one slot. It carries no arena, byte buffer, or thread ownership. |

The state atomics use acquire/release orderings to publish a job before a
worker claims it and to publish completion before a caller observes it.
`Job` and `failure` are protected by independent mutexes. A worker owns its
own mutable staging slice, so workers never share a disk scratch buffer.

The atomic roles are fixed: `next_slot` uses relaxed fetch-add because it only
allocates unique indexes; slot state uses acquire/release CAS, loads, and
stores to publish jobs and terminal results; `wake_generation` uses release
increments and acquire observations; and `stopping`/`poisoned` use release
stores with acquire loads for worker shutdown and poison visibility.

The ownership graph is:

```text
Runtime
  -> Arc<Shared>
       -> Vec<Arc<JobSlot>>
  -> Vec<JoinHandle> (each worker also holds Arc<Shared>)
  -> next_slot and stored lifecycle state
PendingCopy
  -> Arc<JobSlot> + Arc<Shared>
JobSlot.job
  -> Job -> Arc<Backing> source/destination
HostPending
  -> PendingCopy + optional staging Arena + executor flags
```

Dropping a `PendingCopy` removes only its handle. Dropping `Runtime` joins
workers and releases its own `Shared` reference; queued jobs and slots remain
alive until their remaining worker or pending-token references are gone.

## Slot state machine

The constants are private `u8` values in `runtime.rs` and have this exact
meaning:

| Constant | Value | Meaning |
| --- | ---: | --- |
| `SLOT_UNCLAIMED` | `0` | The slot has never been returned as a pending token. |
| `SLOT_PREPARED` | `1` | A `PendingCopy` owns the slot and may submit one job. |
| `SLOT_QUEUED` | `2` | A validated `Job` is published for a worker. |
| `SLOT_RUNNING` | `3` | One worker claimed the job and is executing it. |
| `SLOT_COMPLETE` | `4` | The copy succeeded. The token can be polled as complete or reset. |
| `SLOT_FAILED` | `5` | The worker recorded an error. This slot has no reset path. |

The only valid transitions are:

```text
UNCLAIMED --Runtime::prepare_copy--> PREPARED
PREPARED  --PendingCopy::submit-->  QUEUED
QUEUED    --worker CAS-->           RUNNING
RUNNING   --copy Ok(())-->          COMPLETE
RUNNING   --copy Err(error)-->      FAILED
COMPLETE  --PendingCopy::reset-->   PREPARED
```

The transition operations have these failure-preservation rules:

| Operation | Required state | Success state | Failure state effect |
| --- | --- | --- | --- |
| `Runtime::prepare_copy` | Runtime `Ready`, next fixed slot available, slot `UNCLAIMED` | Slot `PREPARED` | Runtime state and existing slots are unchanged; the monotonic index has still advanced if lookup or CAS failed. |
| `PendingCopy::submit` | Shared poison clear, slot `PREPARED`, both ranges valid, job field empty | Slot `QUEUED` | Slot remains `PREPARED` until a job is successfully published. |
| Worker execution | Slot `QUEUED`, CAS succeeds | `COMPLETE` or `FAILED` | `FAILED` stores the worker error when its failure mutex is usable. |
| `PendingCopy::reset` | Slot `COMPLETE` | Slot `PREPARED` with empty job/failure fields | State and fields remain unchanged when a precondition or mutex check fails. |

`SLOT_FAILED` is terminal. The job has already been removed from the slot by
the time the failure is stored, so a failed token cannot be retried or
rearmed. `SLOT_UNCLAIMED` and `SLOT_PREPARED` are invalid inputs to
`poll()`. Any other byte value makes `poll()` return `Poisoned`; the debug
formatter renders it as `invalid`.

## Runtime lifecycle

### Construction

`Runtime::new` performs the following steps in order:

1. Reserve exactly `config.slots.get()` entries in a local slot vector. A
   failed reservation returns
   `InvalidConfiguration("host slot table allocation failed")`. Each entry
   is an `Arc<JobSlot>` initialized to `SLOT_UNCLAIMED` with empty job and
   failure mutexes.
2. Build `Shared` with wake generation `0`, `stopping = false`, and
   `poisoned = false`.
3. Compute `worker_count = config.worker_threads.min(config.slots.get())`.
   Because slot capacity is nonzero, at least one worker is created when
   construction succeeds.
4. Reserve the staging table. A failed table reservation returns
   `InvalidConfiguration("host staging table allocation failed")`.
5. Allocate one boxed byte slice of
   `config.staging_bytes_per_worker` bytes per worker. The allocation uses
   `try_reserve_exact`, then zero-fills with `resize`; allocation failure
   returns `InvalidConfiguration("host staging allocation failed")`.
6. Reserve the worker-handle table. A failed reservation returns
   `InvalidConfiguration("host worker table allocation failed")`.
7. Spawn each worker with the name `recipe-host-{index}`. The worker receives
   `Arc<Shared>` and its own mutable staging slice. If a spawn fails, the
   runtime sets `stopping`, wakes all already-spawned workers, joins them, and
   returns `ThreadSpawn(error.kind())` without returning a partial runtime.
8. Return a `Runtime` in `RuntimeState::Ready` with `next_slot = 0`.

There is no lazy worker, slot, or staging allocation after this point. The
runtime operation path only publishes jobs, performs byte copies, and joins
already-created threads.

### Observing state

`Runtime::state()` reads the stored lifecycle state and the shared poison bit:

```text
stored Ready  + shared.poisoned = false -> Ready
stored Ready  + shared.poisoned = true  -> Poisoned
stored Closed, regardless of poison    -> Closed
```

The runtime never writes `state = Poisoned`; poisoning is a derived
observation while the stored state is still `Ready`. `Runtime::prepare_copy`
returns `Error::Poisoned` whenever `state()` is not `Ready`. In particular,
the public error does not distinguish `Closed` from `Poisoned` at that call
site.

The shared poison bit is set only by runtime-wide synchronization failures:

- a worker cannot lock a slot's failure mutex after a copy error;
- a worker cannot lock `wake_lock` while going to sleep.

A normal per-job copy failure is stored in that slot and does not by itself
set `Shared::poisoned`. `HostResources` deliberately adds its own
`poisoned` flag and poisons the host backend when its copy submission or
runtime poll reports an error, so executor use is stricter than direct runtime
use. A poison bit does not cancel work already queued or running; workers
continue until shutdown drains those states.

Only new admission and submission consult the shared poison bit. A direct
`PendingCopy::poll` or crate-visible `reset` still observes or resets its own
terminal slot when shared poison is set, subject to that slot's state and
mutexes. HostResources blocks those operations first through its own poison
flag.

### Close and drop

`Runtime::close(self)` consumes the runtime and calls `shutdown`. The shutdown
sequence is:

1. If the stored state is already `Closed`, return `Ok(())`.
2. Store `stopping = true` with release ordering and notify all workers.
3. Drain the worker handle vector and join every handle.
4. If any join reports a panic, return `ThreadPanicked`; otherwise return
   `Ok(())`.
5. Store `state = Closed` even when a join failed.

Workers do not cancel queued work. After observing `stopping`, each worker
continues scanning and executing queued or running slots, then exits only
when no slot is `QUEUED` or `RUNNING`. Prepared, unclaimed, complete, and
failed slots do not delay shutdown. `Drop` invokes the same shutdown and
discards its result, ensuring that a normal drop still joins all workers.
If a worker panics before publishing a terminal slot state, shutdown reports
`ThreadPanicked`; it does not synthesize `COMPLETE` or `FAILED` for that slot.

The shutdown flag is not checked by `PendingCopy::submit`. Callers must not
submit a token after its owning `Runtime` has been closed: a token can publish
`SLOT_QUEUED` after workers have exited, and `poll()` would then remain
`Pending`. The host executor orders all submissions before resource
destruction, and `Runtime::close` itself drains any queued or running work
during that destruction. `Runtime::prepare_copy` also has no stopping check,
so `Runtime` admission and token submission must not race a concurrent close.

## Claiming a slot

`Runtime::prepare_copy()` is the only constructor for `PendingCopy`:

1. Require `state() == Ready`; otherwise return `Poisoned`.
2. Atomically fetch and increment `next_slot` with relaxed ordering.
3. Index the fixed `shared.slots` vector. An index outside the vector returns
   `SlotCapacityExhausted`.
4. CAS that slot from `UNCLAIMED` to `PREPARED`. A failed CAS returns
   `SlotBusy`.
5. Return a token containing `Arc` references to the slot and shared state.

The index is never decremented. An exhausted or failed claim still consumes
the fetched index, and a reset token is reused through `PendingCopy::reset`
rather than by claiming a new slot.

## Submitting a copy

`PendingCopy::submit` takes mutable access to the token, two `Arena` references,
two byte offsets, and one nonzero `ByteCount`. Validation and publication are
ordered as follows:

1. Read `Shared::poisoned`; return `Poisoned` when set.
2. Require the slot state to be `PREPARED`; otherwise return
   `InvalidPendingState`.
3. Validate the source range and then the destination range. A zero byte count
   returns `InvalidConfiguration("host copy size must be nonzero")`. The
   checked condition is `offset + bytes <= arena.bytes()`. A failed check,
   including offset addition overflow, returns `OutOfBounds { device }` for
   that arena.
4. Try to lock the slot's job mutex. Contention returns `SlotBusy`; a poisoned
   mutex returns `Poisoned`.
5. Require the job field to be `None`; an occupied field returns `SlotBusy`.
6. Store a `Job` containing cloned source and destination backing `Arc`s,
   offsets, and `bytes.get()`.
7. Store `QUEUED` with release ordering, drop the job mutex guard, increment
   the shared wake generation, and call `notify_one` on the condition
   variable.

The job is not copied into a separate queue. The slot itself is the queue
entry. The release store publishes the fully initialized job before a worker's
acquire CAS can claim it. The token remains valid for polling while the worker
owns the slot.

## Worker loop and wake protocol

Every worker executes `worker_loop(shared, staging)`:

1. Set `progressed = false`.
2. Scan the fixed slot vector in order. For every slot whose state is
   `QUEUED`, atomically CAS it to `RUNNING`. A successful CAS marks progress,
   calls `execute_slot`, and stores `COMPLETE` or `FAILED`.
3. On a copy error, lock the failure mutex and store the error, or set the
   shared poison bit if that mutex is poisoned. Then store `FAILED`.
4. If `stopping` is set and no slot is `QUEUED` or `RUNNING`, exit.
5. If no slot was claimed, read `wake_generation`, lock `wake_lock`, and wait
   for up to 10 milliseconds while both the generation remains unchanged and
   `stopping` remains false.
6. Repeat the scan.

`notify_worker` increments `wake_generation` before `notify_one`. The
generation predicate closes the race in which a submit occurs between a
worker's scan and its condition-variable wait. The timed wait is a bounded
idle wait, not the completion mechanism; callers still use `poll()` to
observe terminal state. Multiple workers can claim different queued slots in
one scan, and the first successful CAS determines which worker owns each
slot. There is no separate FIFO queue or ordering guarantee beyond the slot
scan and atomic claims. The 10 millisecond wait is an intrinsic runtime
constant, not a `RuntimeConfig` field.

`execute_slot` locks the job mutex, takes the `Job` out of the slot, and calls
`execute_copy`. Taking the job before I/O means the slot's job field is empty
while the copy runs, but its state remains `RUNNING`; a completed or failed
slot therefore cannot retain arena backings through the slot.

## Copy implementations

`execute_copy` dispatches only on the two `Backing` variants. All four pairs
are valid:

| Source | Destination | Implementation | Scratch |
| --- | --- | --- | --- |
| `Ram` | `Ram` | `copy_ram_to_ram` | None |
| `Ram` | `Disk` | Lock source storage, then `write_all_at` the destination file | None |
| `Disk` | `Ram` | Lock destination storage, then `read_exact_at` the source file | None |
| `Disk` | `Disk` | `copy_disk_to_disk` in bounded chunks | One worker-owned staging slice |

### RAM to RAM

When source and destination `Arc<Backing>` pointers are equal, the operation
is an in-place move. The worker locks the storage once, validates the source
range, converts the destination offset to `usize`, and uses
`slice::copy_within`, which preserves overlapping ranges.

For distinct backings, the worker locks both storage mutexes in allocation
pointer order. If the source backing pointer is lower, it locks source then
destination; otherwise it locks destination then source. This is the runtime's
deadlock-avoidance order for two concurrent cross-copy jobs. It then validates
both `usize` ranges and calls `copy_from_slice`.

### RAM and disk

RAM-to-disk holds the source mutex while it converts the source range and
repeatedly writes the bytes with positional `FileExt::write_at`. Disk-to-RAM
holds the destination mutex while it converts the destination range and
repeatedly reads with positional `FileExt::read_at`. Positional I/O does not
share a mutable file cursor between workers.

### Disk to disk

The worker's staging length is nonzero by configuration. Each iteration uses
`min(remaining, staging.len())`, converts the chunk length to `usize`, reads a
chunk, writes a chunk, and advances with checked arithmetic.

When source and destination are the same backing and the destination starts
inside the source range at a higher offset, the operation copies chunks from
the end toward the beginning. This is the disk equivalent of an overlapping
`memmove`. All other cases copy forward from offset zero through the requested
length. Separate source and destination backings, including ordinary
non-overlapping same-file cases, use the forward path.

### Positional I/O and range helpers

`host_range` converts a `u64` offset and length to a `usize..usize` range with
checked conversion and addition. Conversion or addition failure returns
`RangeOverflow`. `read_exact_at` loops until the destination slice is full;
an OS read error returns `WorkerFailed { operation: "disk read", kind }`, a
zero-byte read returns `WorkerFailed` with `UnexpectedEof`, and offset advance
overflow returns `RangeOverflow`. `write_all_at` is symmetric, using
`WorkerFailed { operation: "disk write", kind }` and `WriteZero` for a
zero-byte write.

The worker copy functions do not call `sync_data`. Disk durability and
temporary-file removal belong to `Arena`'s close/drop behavior, not to each
asynchronous copy.

### Arena backing assumptions

The runtime receives `Arena` handles, not raw slices or paths. `Arena::ram`
owns a nonzero `Mutex<Box<[u8]>>`; `Arena::disk` creates a caller-selected
file with `create_new`, allocates its extent, and stores the open `File` plus
path in a `Backing`. Cloning an arena clones the backing `Arc`, so two handles
can intentionally address one RAM allocation or one disk file for overlapping
copy semantics. The runtime never creates a disk path or chooses an arena
kind.

Runtime validation is byte-oriented: it checks each offset and length against
`Arena::bytes()`, but it does not compare `Arena::device()`, enforce a route,
or reject a RAM-to-RAM versus disk pair. The host backend performs those
device, phase, endpoint, and contract checks before invoking the runtime.

`Backing::Drop` syncs and removes a disk file when its final `Arc` is dropped.
Explicit `Arena::close` first requires `Arc::try_unwrap`; it syncs and removes
the disk file, or returns `SlotBusy` while any runtime job or other arena
handle still retains the backing. Runtime job `Arc`s are therefore part of
the arena lifetime contract even though `PendingCopy` itself exposes no
arena field.

## Polling and token reuse

`PendingCopy::poll()` performs one acquire load of the slot state:

| Slot state | `poll()` result |
| --- | --- |
| `QUEUED` or `RUNNING` | `Ok(PollStatus::Pending)` |
| `COMPLETE` | `Ok(PollStatus::Complete)` |
| `FAILED` | Lock `failure`; return the stored `Error`, or `Poisoned` if no error was present. A poisoned failure mutex returns `Poisoned`. |
| `UNCLAIMED` or `PREPARED` | `Err(InvalidPendingState)` |
| Any other byte | `Err(Poisoned)` |

`PendingCopy::reset()` is crate-visible because the host backend controls
repetition. It requires `COMPLETE`, locks and clears both the job and failure
fields, then stores `PREPARED`. It returns `InvalidPendingState` for queued,
running, failed, unclaimed, or already prepared tokens, and `Poisoned` if
either mutex is poisoned.

The direct runtime API has no blocking wait or callback. The caller polls
until complete, then either resets the token for another loop iteration or
drops it. A failed token cannot be reset.

The minimal valid direct sequence is:

```text
slots = SlotCapacity::new(slot_count)?      # slot_count != 0
config = RuntimeConfig::new(worker_threads, slots, staging_bytes_per_worker)?
# worker_threads != 0, staging_bytes_per_worker != 0, bytes != ByteCount::ZERO
runtime = Runtime::new(config)?
pending = runtime.prepare_copy()?
pending.submit(source, source_offset, destination, destination_offset, bytes)?
repeat pending.poll()? until Complete
pending.reset()                  # crate-visible host-backend operation
runtime.close()?                 # drains queued/running jobs before joining
```

The `reset` line is not part of the public external API because it is
`pub(crate)`; it is shown to make the host backend's loop-repetition contract
explicit. A caller that does not own the crate boundary can instead drop the
completed token and close the runtime.

## Host resource and executor boundary

The runtime is embedded by `HostResources` in
[`host/src/backend.rs`](../../src/backend.rs). The backend is the policy and
contract layer; the runtime is only its copy mechanism.

Capacity discovery is also outside the runtime. `HostBackendConfig::available_bytes`
and `HostResources::available_bytes` query RAM or the configured disk
directory before realization; worker submission never rechecks host capacity
or changes the binding.

### Realization and slot ownership

- `HostResources::realize_scoped` validates bindings, finalized bundle
  devices, reservations, and task contracts, then creates
  `SlotCapacity::new(contracts.len().max(1))` and a `RuntimeConfig` from the
  host backend's worker and staging settings. The contract count covers all
  selected host tasks across init, loop, and exit, so loop iterations do not
  grow the runtime slot table.
- `HostPreparedResources::realize` follows the same rule for a selected host
  partition, using `tasks.len().max(1)`. Its
  `prepare_candidate_pending` pass calls `Runtime::prepare_copy` for every
  selected task before finalization and stores the resulting `HostPending`
  values in a map. If that pass fails, it attempts `Runtime::close` before
  returning the original preparation error.
- A deferred `HostResources` value calls `Runtime::prepare_copy` from
  `prepare_pending`. A warm/prepared value removes an already-realized token
  from its pending map instead of allocating a new runtime slot.
- Every `HostPending` contains one `PendingCopy`, optional preallocated RAM
  staging `Arena`, an optional init-image contract, a pending action, and
  `submitted`/`terminal` flags. The flags prevent host-level double submit and
  define the executor phase lifecycle around the runtime slot.

The executor's `realize_phase` calls `Backend::prepare_pending` once per
finalized task before submitting work. This is why all runtime slots and any
task-specific staging arenas exist before the loop starts.

The host backend accepts the following phase and endpoint combinations before
the runtime is called:

| Phase | Endpoint pair | Host work class | Runtime consequence |
| --- | --- | --- | --- |
| `Init` | `External -> Device` | `InitAdmission` | Stage the init image, then copy it into the device arena. |
| `Init` or `Loop` | `Device -> Device` | `InternalTransfer` | Copy between host-owned arenas. |
| `Exit` | `Device -> Device` | `ExitTransfer` | Copy between host-owned arenas without external collection. |
| `Exit` | `Device -> External` | `ExitTransfer` | Copy to staging, then `collect_exit` reads the external result. |

Other phase or endpoint combinations are rejected as host protocol errors;
they never become runtime jobs. Planner-expanded routes must also be at most
one hop for finalized host transfers.

### Work mapping

`HostResources::submit` validates the immutable finalized contract and then
maps executor work to one runtime copy:

| Executor work | Runtime copy | Token staging |
| --- | --- | --- |
| `InitAdmission` | Write the image bytes into a preallocated staging RAM arena, then copy staging to the destination device arena | Image-sized RAM arena |
| `InternalTransfer` | Copy one host arena to another host arena | None |
| `ExitTransfer` | Copy the device source arena to a staging RAM arena | Source-sized RAM arena |
| `Metric` | Copy exactly four bytes from the device value arena to staging | Four-byte RAM arena, decoded as `F32` or `I32` on completion |
| `Calculation` | Rejected as `UnsupportedWork`; payload calculations belong to CUDA or HSA | None |

The backend also verifies task class, endpoints, bytes, route, lane claims,
submission slots, and arena bounds before invoking `PendingCopy::submit`. A
successful call sets `HostPending::submitted = true`. An error from the
dispatch path, including staging or runtime-copy failure, marks
`HostResources::poisoned = true` and is returned to the executor. Initial
host-health and contract guards return their protocol error directly.

`HostResources::poll_pending` requires both `submitted = true` and
`terminal = false`, then delegates to `PendingCopy::poll`:

- runtime `Pending` becomes executor `BackendPoll::Pending`;
- runtime `Complete` optionally reads the metric staging arena, sets
  `terminal = true`, and becomes `BackendPoll::Complete { metric }`;
- a runtime poll error poisons `HostResources` and is returned. Host-level
  token or contract guards fail before that poison update.

The executor repeatedly calls backend `poll` through its nonblocking phase
poll loop. The host backend records the corresponding physical poll status in
its executor-facing accounting layer, but the runtime itself owns no
`PhysicalCall` records.

For loop repetition, `HostBackend::submit_loop_iteration` first calls
`prepare_loop_pending`. A never-used token
(`submitted = false`, `terminal = false`) is used as-is. A token from a
completed prior iteration
(`true, true`) calls `PendingCopy::reset` and clears both flags. Any active or
inconsistent combination is a protocol error. Warm candidate teardown calls
`recycle_pending`, which requires a terminal token, resets its runtime slot,
and returns it to the pre-final pending map.

`LocalBackend` routes only tasks owned by the host partition to this path.
GPU calculations go to CUDA or HSA resources, and cross-backend transfers go
to the bridge resource. Thus the host runtime's complete domain is
host-backed byte movement and the four-byte metric readback specialization.

### Exit collection and resource destruction

After an exit transfer reaches runtime `COMPLETE`, the executor calls
`HostResources::collect_exit`. The host backend validates that the completed
token is an exit action and reads the staging arena into the caller's external
destination. It does not submit another runtime job.

Executor teardown releases realized arenas first, then calls
`HostResources::destroy`. Destruction drops the pending-token pool and then
calls `Runtime::close`, so workers are always joined. A `Job` retains cloned
backing `Arc`s while queued or running; this keeps its source and destination
alive until the worker finishes even if an owning `Arena` handle is dropped.
`Arena::close` separately requires the backing `Arc` to be unique and can
return `SlotBusy` while an outstanding job still holds a reference.

## Error map

The following errors are the observable failure classes emitted by runtime
operations. The host backend may wrap the same error in a backend or local
executor error; copy-submit and runtime-poll failures mark its resources
poisoned, while initial contract guards return without that update.

| Error | Runtime cause |
| --- | --- |
| `InvalidConfiguration` | Zero slot capacity, zero worker count, zero worker staging, failed fixed-table allocation, failed staging allocation, or a submitted zero-byte copy. |
| `SlotCapacityExhausted` | The monotonic `next_slot` index is outside the preallocated slot vector. |
| `SlotBusy` | A slot CAS did not see `UNCLAIMED`, a job mutex was contended, or a slot already contained a job. |
| `InvalidPendingState` | Submit, poll, or reset was called for a slot state that does not permit that operation. |
| `OutOfBounds { device }` | A submit-time source or destination range, including checked offset addition, exceeds its arena. |
| `RangeOverflow` | A worker could not convert a `u64` range to host `usize` or could not advance a file offset with checked arithmetic. |
| `WorkerFailed { operation, kind }` | A worker disk read/write returned an I/O error, EOF, or zero-byte write. |
| `ThreadSpawn(kind)` | A worker thread could not be created during construction. Previously spawned workers are stopped and joined. |
| `ThreadPanicked` | A worker join reported panic during `close` or shutdown. State still becomes `Closed`. |
| `Poisoned` | Shared runtime state, a job/failure/wake mutex, or a direct operation observing a malformed slot state. At the host boundary, `HostResources` uses the same error after poisoning itself. |

A normal worker copy failure is stored in `JobSlot::failure` and surfaced by
`poll()` as the original `Error`; it is not silently retried. There is no
alternate copy path, retry loop, or cancellation path in the runtime.

## Debug and invariants

`Runtime` debug output includes its copied `RuntimeConfig`, dynamic
`RuntimeState`, and the monotonic `next_slot` value under the field name
`prepared_slots`; it intentionally does not expose workers or shared internals.
`PendingCopy` debug output exposes only the decoded slot state under `state`.

The implementation relies on these invariants:

- `SlotCapacity` is nonzero before `Runtime::new` computes worker count.
- Each slot has at most one published job, and a worker claims a queued slot
  with one compare-and-exchange.
- A slot's job is removed exactly once by `execute_slot`; terminal slots retain
  only their state and, on failure, one cloned error.
- Every worker has a distinct staging slice and never accesses another
  worker's scratch storage.
- Submit-time arena checks precede publication, while worker-side range and
  conversion checks protect the concrete slice and file operations.
- Shutdown drains queued and running slots before joining workers.
- Runtime construction, not submission or polling, owns the vector, staging,
  and thread allocations.
- Host backend resource realization, not the runtime, validates finalized task
  contracts and decides whether staging is needed.
