# `recipe-executor` facade

`executor/src/lib.rs` is the public boundary for backend-neutral execution of
one immutable `recipe_core::FinalizedBundle`. The crate does not build a
bundle, discover hardware, choose a placement, compile or load a kernel,
allocate a resource, or mutate topology. Those operations belong to the
preparation, native-adapter, and transport crates. This crate consumes the
already-resolved task graph and turns it into a bounded asynchronous lifecycle
over a caller-supplied backend.

The package is `recipe-executor` version `0.1.0`, Rust edition 2024, and MIT
licensed. Its only dependencies are `recipe-core` and `sha2`. The crate-level
attributes are:

```rust
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
```

Every public value therefore has a `Debug` implementation and the executor
contains no unsafe driver boundary. `sha2` is used only by worker projection
identity derivation. Native CUDA, HSA, and host implementations live below the
`Backend` trait in `recipe-native-executor`, `recipe-cuda`, `recipe-hsa`, and
`recipe-host`.

## The root module surface

The module declarations in `lib.rs` are private implementation modules:

| Module | Owns | Public boundary it supplies |
| --- | --- | --- |
| `backend.rs` | The sealed backend trait, closed work records, arena views, completion results, and fixed physical-call ABI | Backend adapters submit only finalized calculations, transfers, admissions, metrics, and exit transfers, with no preparation operation in the work enum. |
| `error.rs` | Fixed-capacity backend diagnostics, operation labels, journal stream labels, `ExecutorError`, and the crate result alias | All ordinary executor failures are represented as `ExecutorError`; backend text is retained without loop-time allocation. |
| `executor.rs` | The local typestate run, scheduler, phase preparation, image validation, journal accounting, metrics integration, exit-image collection, and teardown | `PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun`, plus watchdog, images, journal, capacity, and failure values. |
| `metrics.rs` | Calculation value representation, metric samples, and the capacity-one metric mailbox | User metrics are nonblocking latest-value slots keyed by finalized metric slot. |
| `worker.rs` | Worker-node projection, external cross-machine transfer contracts, the worker backend extension, and the imperative worker session | A remote worker receives exactly its immutable projection and can dispatch only projected work in the current lifecycle state. |

The root reexports `backend::sealed` with `#[doc(hidden)]`. This exposes the
trait path needed by Recipe-owned backend implementations while signalling that
the sealing module is not application API. The five implementation modules are
not reachable as `recipe_executor::backend`, `recipe_executor::executor`, or
similar paths. Every other root item is a deliberate reexport, not a second
implementation or an alias with independent state.

## Exact root reexports

The following names are the complete public root inventory in `lib.rs`.

### Backend ABI

```text
sealed
ArenaSet, Backend, BackendPoll, BackendWork
CalculationWork, InitAdmissionWork, MetricWork, TransferWork
PendingRequest, WorkClass
PhysicalCall, PhysicalCallBatch, PhysicalCallBatchOverflow, PhysicalPollStatus
MAX_PHYSICAL_CALLS_PER_OPERATION
```

`WorkClass` is the closed set `InitAdmission`, `Calculation`,
`InternalTransfer`, `Metric`, and `ExitTransfer`. The four work records carry
the finalized task ID and resolved values:

* `InitAdmissionWork` contains a device destination, exact byte count,
  submission slots, and a borrowed packed image.
* `CalculationWork` contains run and zero-based loop iteration, device, kernel
  template, artifact, submission slots, borrowed input and output location
  slices, and an optional resolved fault flag.
* `TransferWork` contains resolved source and destination endpoints, byte count,
  route links, lane claims, and submission slots.
* `MetricWork` contains the loop iteration, metric purpose, metric and slot IDs,
  resolved value location, and submission slots.

`BackendWork` is the single immutable operation value passed to `Backend::submit`
or `Backend::submit_loop_iteration`. Its `task()` and `class()` accessors are
the canonical task and work-class projections. There is intentionally no
compiler, loader, allocator, discovery, topology-mutation, or external-data
variant. External admission is represented by `InitAdmission`; an exit copy is
represented by `ExitTransfer` and is collected only during exit.

`PendingRequest` identifies the task, phase, work class, and optional submission
slots for one backend-owned completion token. A backend must realize every
resource reachable from the returned token before `init`; `submit` and `poll`
must then operate only on that token and other preallocated state.

`ArenaSet<'a, A>` is a read-only view over the executor's exact
`BTreeMap<DeviceId, A>`. Its hidden `new` constructor is used by the executor
and composite backends. Public readers are `get`, `iter`, `len`, and
`is_empty`; no caller can insert or replace an arena through the view.

`BackendPoll` is `Pending` or `Complete { metric: Option<MetricValue> }`.
`PhysicalPollStatus` records `Pending`, `Complete`, or `Failed` for the
corresponding physical poll record. `PhysicalCall` records concrete backend
actions such as resource binding, pending-token preparation, arena allocation,
admission chunks, local or external submissions, polls, worker quiescence,
exit collection, arena release, and resource destruction. Logical lifecycle
events are kept separately in `RunJournal`.

`MAX_PHYSICAL_CALLS_PER_OPERATION` is `16`. `PhysicalCallBatch` stores a
fixed-capacity inline prefix and exposes `new`, `single`, `try_push`,
`try_from_array`, `iter`, `len`, and `is_empty`. `PhysicalCallBatchOverflow` is
returned when an adapter attempts to exceed the ABI bound. A backend may lower
its `MAX_NON_POLL_PHYSICAL_CALLS` associated constant, but it may not exceed
the ABI bound.

`Backend` is sealed by `sealed::Sealed`, so only Recipe-owned adapters can
implement it. Its associated types are `Arena`, `Resource`, `Pending`, and an
`Error: Error + Send + Sync + 'static`. The required methods are:

```text
bind_resources(bundle, physical_calls) -> Resource
prepare_pending(resource, request, physical_calls) -> Pending
allocate_arena(resource, layout, physical_calls) -> Arena
supports_loop_repetition() -> bool
supports_same_queue_pipelining(resource, task) -> bool
submit(resource, arenas, pending, work, physical_calls)
submit_loop_iteration(resource, arenas, pending, iteration, work, physical_calls)
poll(resource, pending, physical_calls) -> BackendPoll
collect_exit(resource, arenas, pending, work, destination, physical_calls)
release_arena(resource, device, arena, physical_calls)
destroy_resources(resource, physical_calls)
```

`bind_resources` and `prepare_pending` are the preparation boundary. They may
perform native realization, loading, warming, and allocation needed by later
operations. `submit`, `poll`, and backend error formatting must not allocate,
grow collections, load code, or lazily initialize driver state. The default
`supports_loop_repetition` is false, so a finalized loop with more than one
iteration or an unbounded loop is rejected unless the adapter opts in. The
default `supports_same_queue_pipelining` is also false; an opt-in adapter must
prove queue ordering and retain all resources until the queue is idle. The
default `submit_loop_iteration` forwards to `submit`, while a repeatable
adapter may rearm a terminal token using its own activation state.

Every `poll` call must append exactly one `PhysicalCall::Poll` for its pending
task, with status matching the returned result, and no other physical call.
`collect_exit` is called only during `ExitedLoop::exit`, never by the live loop.

### Errors and result alias

```text
BACKEND_MESSAGE_CAPACITY
BackendMessage
BackendOperation
JournalStream
ExecutorError
Result<T> = std::result::Result<T, ExecutorError>
```

`BACKEND_MESSAGE_CAPACITY` is `96` bytes. `BackendMessage` is a fixed UTF-8
buffer with `new`, `as_str`, and `was_truncated`; `Display` appends `...` when
the message was clipped. `BackendOperation` identifies binding, pending
preparation, arena allocation, submit, poll, exit collection, arena release,
and resource destruction. `JournalStream` distinguishes logical and physical
journal capacity failures.

`ExecutorError` is `#[non_exhaustive]`. Its variants are the fail-closed
boundaries of the local executor:

* `Backend` wraps an operation and bounded backend message.
* `DuplicateAdmission`, `MissingAdmission`, `UnexpectedAdmission`,
  `AdmissionImageMismatch`, and `AdmissionSizeMismatch` reject any init image
  set that is not exactly the finalized per-device manifest.
* `InvalidPhaseTask` rejects a task kind in the wrong lifecycle phase, and
  `BackendProtocol` rejects a backend result or finalized location that
  violates the closed ABI.
* `DeviceFault` reports a nonzero checked int32 fault flag through its metric
  readback task and resolved value.
* `LifecycleInvariant` reports impossible typestate or resource transitions.
* `SchedulerStalled` means remaining work has no runnable or pending task;
  `WatchdogExpired` means a phase exceeded its allowed nonprogress polls;
  `InvalidWatchdog` rejects a zero limit.
* `LoopRepetitionUnsupported` rejects an unbounded or multi-iteration bundle
  for a one-shot backend.
* `MetricSequenceOverflow`, `PendingPollCountOverflow`,
  `PreparationCapacityOverflow`, and `JournalCapacityExceeded` protect
  bounded counters and preallocated journal storage.
* `ExitImageTooLarge`, `ExitImageAllocationFailed`, and the exit-side
  `BackendProtocol` checks reject an external result that cannot be represented
  or collected into its precomputed result slot.

### Local lifecycle and evidence

```text
Watchdog, DeviceImage, ExitImage, LogicalEvent
JournalCapacity, PendingPollCount, JournalSummary, RunJournal
RuntimeCapacities
RunFailure, RunFailureParts
PreparedRun, InitializedRun, RunningRun, ExitedLoop, ExitedRun
LoopStatus
```

`Watchdog::new` rejects zero. `Watchdog::max_nonprogress_polls` returns the
configured limit, and `for_expected_duration` converts a measured duration and
nonzero safety multiplier into a finite limit for the executor's 50
microsecond-to-2 millisecond blocking backoff. `DeviceImage` is the caller-owned
packed image admitted to one device, with public `device`, `image`, and `bytes`
fields plus `new`. `ExitImage` is the caller-owned result of one finalized
external exit transfer, with public `task`, `source`, and `bytes` fields.

`LogicalEvent` is the semantic lifecycle stream. It contains preparation,
arena allocation, admission, task submission and completion, external transfer
submission and completion, initialization, loop entry, per-iteration entry and
completion, metric publication, fault checks, accepted stop, loop completion or
failure, worker quiescence, arena release, and final exit. `PhysicalCall` is not
a replacement for this stream: one logical admission may produce many
`AdmissionChunk` records, and repeated pending polls are compacted separately.

`JournalCapacity::new` accepts explicit logical and physical bounds;
`JournalCapacity::for_bundle::<B>` derives fixed bounds from the exact finalized
task graph and `B::MAX_NON_POLL_PHYSICAL_CALLS`. The calculation reserves
storage for non-loop tasks, the first retained loop iteration, metric
emissions, arenas, exit images, and fixed lifecycle records. Pending polls do
not scale the ordered capacity because each task has an exact counter.

`PendingPollCount` exposes a task ID and its exact `u128` pending-poll count.
`JournalSummary` counts logical and physical records observed and compacted.
`RunJournal` exposes:

```text
logical_events() -> &[LogicalEvent]
physical_calls() -> &[PhysicalCall]
pending_poll_counts() -> &[PendingPollCount]
pending_poll_count(task) -> Option<u128>
summary() -> JournalSummary
declared_capacity() -> JournalCapacity
allocated_capacity() -> JournalCapacity
```

Production retains ordered detail for preparation, initialization, exit, and
the first loop iteration. Repeated loop submissions, completions, metric
publication, fault checks, iteration markers, and pending polls are compacted
after that first iteration; the summary and per-task counters preserve exact
observed totals without allocation proportional to the loop bound. The journal
returns `JournalCapacityExceeded` rather than growing. `RuntimeCapacities`
reports the actually allocated phase slots, completion entries, logical and
physical journal capacities, and pending-poll entry count.

`RunFailure<B>` preserves the run and bundle identities, primary
`ExecutorError`, optional first teardown `cleanup_error`, the backend value, and
an optional journal. Its readers are `run_id`, `bundle_identity`, `error`,
`cleanup_error`, and `journal`; `into_parts` returns those fields in
`RunFailureParts<B>`, whose fields are public. A failure never discards backend
ownership merely because execution stopped.

### Metrics

```text
MetricValue
MetricSample
MetricMailbox
```

`MetricValue` is the closed calculation result `F32(f32)` or `I32(i32)`.
`MetricSample` carries a monotonically increasing sequence, loop iteration,
task, slot, metric, and value. `MetricMailbox` creates one capacity-one slot
for each finalized user metric. `try_take` is nonblocking and removes the
latest sample; publishing a new sample replaces an older unconsumed sample in
the same slot while leaving other slots independent. `pending_len`,
`capacity`, and `is_empty` expose mailbox state. Slot and metric mismatches are
`BackendProtocol` errors, and sequence exhaustion is `MetricSequenceOverflow`.
Fault-readback metrics are consumed by the executor as checked int32 values and
are not published as user samples.

### Worker projection and session

```text
WorkerAssignment
WorkerTaskRole, ExternalTransferDirection, WorkerExternalTransfer
WorkerProjection, WorkerProjectionError
ExternalTransferPoll
WorkerBackend, WorkerBackendOperation, WorkerLifecycle, WorkerTaskPoll
WorkerExecutionError
WorkerPrepareFailure, WorkerPrepareFailureParts
WorkerExecutionSession
```

`WorkerAssignment` binds one `MachineId` and `NodeId`. `WorkerProjection::derive`
validates the measured topology, bundle topology identity, machine and worker
node ownership, nonempty device set, finalized arena layouts, reservations,
init images, task identities, dependencies, resource ownership, metric slots,
and exact init-admission packing. It classifies each projected task as
`InitAdmission`, `Local`, `ExternalIngress`, or `ExternalEgress`, retains only
the selected worker's immutable records, collects the referenced artifacts,
and derives a content digest over the bundle, topology, assignment, devices,
and projected task/dependency shape. Accessors expose that digest, bundle and
topology identities, assignment, device and layout slices, `(task, phase,
role)` iteration, artifact IDs, init image byte counts, external transfers,
task roles, and task phases.

`WorkerExternalTransfer` is an immutable preprovisioned cross-machine contract:
task, phase, direction, local resolved value, exact bytes, route, lane claims,
and submission slots. `ExternalTransferPoll` is `Pending` or
`Complete { bytes }`.

`WorkerBackend: Backend` adds one `ExternalPending` token type and the hooks
`bind_worker_resources`, `prepare_external`, `begin_external_ingress`,
`begin_external_egress`, `poll_external`, `acknowledge_external_egress`, and
`quiesce_worker`. Worker resource binding and every external token are
pre-realized before init. Ordinary local tasks, arenas, local polling, and
destruction continue to use the base `Backend` methods. Quiescence is required
before any arena release.

`WorkerExecutionSession::prepare` validates a nonzero run ID and exact bundle
and topology identities, derives a bounded worker journal, binds worker
resources, prepares every local or external pending token, allocates host image
buffers, and enters `WorkerLifecycle::Prepared`. The session owns the backend,
resource, projected task slots, image buffers, arenas, watchdog, lifecycle, and
journal.

The session methods form an explicit imperative lifecycle:

```text
Prepared
  -> begin_init_image / write_init_chunk / submit_init_image / poll_init_image
  -> Init (while images arrive)
  -> Loop (after every init admission completes)
  -> submit_task / poll_task
  -> begin_external_ingress / poll_external_ingress
  -> begin_external_egress / poll_external_egress / acknowledge_external_egress
  -> begin_exit
  -> Exit
  -> release_arena (every projected device exactly once)
  -> finish
  -> Finished
```

`cancel` is legal from `Loop` or `Exit`; it quiesces native work and enters
`Cancelling`, where arenas can be released before `finish`. `fatal_cleanup`
quiesces, marks active transfers quiesced, releases every remaining arena,
zeroes image buffers, destroys resources, and enters `Failed`. If quiescence
fails, arenas are deliberately preserved because native work may still refer to
them. `into_parts` succeeds only in `Finished` or `Failed` after the resource
has been destroyed, and returns the backend plus journal.

`WorkerExecutionError<E>` is `#[non_exhaustive]` and separates projection,
run/bundle identity, unknown task/device, wrong role or phase, lifecycle,
duplicate dispatch, inactive task, dependency and phase completion, schedule
conflicts, byte and init digest/offset checks, metric contract, device fault,
watchdog, repeated arena release, backend operation, journal, and capacity
failures. `WorkerPrepareFailure` and its public `into_parts` result preserve the
backend and distinguish the primary preparation error from the first cleanup
error.

## Ownership and data flow

The executor owns one `RunCore<B>` while a local run is alive:

```text
FinalizedBundle (moved into RunCore)
        |
        +-- B backend value, then B::Resource from bind_resources
        +-- BTreeMap<DeviceId, B::Arena> (created by initialize)
        +-- one B::Pending slot per prepared task and phase
        +-- CompletionLedger and MetricMailbox
        +-- bounded RunJournal and preallocated exit-image vector
        `-- Watchdog and immutable run/bundle identities
```

`PreparedRun` owns all three phase slot vectors and the backend resource, but
not arenas or admitted images. `initialize` consumes it, validates all
`DeviceImage` values, allocates every finalized arena, and blocks the init
phase. `InitializedRun` owns the same core plus loop and exit slots. It can
only be consumed by `start_loop`. `RunningRun` owns the active loop slots and
the dormant exit slots; it exposes no allocation, compilation, module loading,
or data-ingress method. `ExitedLoop` owns only exit slots and permits exit
transfers and teardown. `ExitedRun` owns the backend value after its resource
has been destroyed, the metric mailbox, collected exit images, and journal.

`BackendWork` borrows finalized location slices and `ArenaSet` borrows the
executor's arena map for the duration of one backend call. A backend never
receives a mutable reference to executor state and is not exposed after being
moved into a run. The executor alone transitions `ResourceState::Active` to
`Taken`, releases each arena, and calls `destroy_resources` exactly once in
normal teardown. On any failure, the same ordered teardown is attempted and the
backend is returned through `RunFailure`.

The worker session follows the same ownership rule with one additional
projection-owned set of external pending tokens. Remote transport code may
stream image chunks or external transfers through the session, but it cannot
dispatch a task, device, phase, route, or byte count absent from the immutable
projection.

## Local typestate lifecycle

The public local path is intentionally linear:

```text
PreparedRun
    -- initialize(images) --> InitializedRun
    -- (failure) -----------> RunFailure<B>

InitializedRun
    -- start_loop() --------> RunningRun
    -- (failure) -----------> RunFailure<B>

RunningRun
    -- poll()/wait() --------> RunningRun until LoopStatus::Complete
    -- into_exited_loop() --> ExitedLoop
    -- fail(error) ---------> RunFailure<B>

ExitedLoop
    -- exit() --------------> ExitedRun
    -- (failure) -----------> RunFailure<B>
```

`PreparedRun::prepare` is the concise error-only constructor. The
`prepare_recoverable` and `prepare_with_journal_capacity_recoverable` forms
return `RunFailure<B>` so callers can recover the backend and journal. The
explicit-capacity pair is useful when a caller has already derived or audited
the exact journal bound. Preparation checks loop-repetition support, converts
every finalized task into one private prepared work record, calls
`bind_resources`, and calls `prepare_pending` for init, loop, and exit slots.
No arena or external image is admitted during this stage.

`PreparedRun::initialize` and `initialize_recoverable` require one image for
every finalized arena device and no others. The image value ID and byte count
must equal the finalized init manifest. Before admission, the executor resets
each finalized fault flag to zero inside its exact init image range. It then
allocates all arenas and runs the init scheduler to completion. Init allows
external-to-device admissions and device-to-device internal transfers only.

`InitializedRun::start_loop` begins finalized iteration zero and records
`LoopStarted` and `LoopIterationStarted`. A missing iteration zero is a
`LifecycleInvariant` failure. The returned `RunningRun` owns the only live-loop
capabilities: `poll`, `poll_with_progress`,
`poll_with_progress_or_stop`, `wait`, latest metric consumption, journal and
capacity inspection, and `current_iteration`.

Each poll pass first marks inactive sparse tasks complete when their
dependencies are complete, then submits runnable tasks subject to dependencies,
schedule windows, and optional same-queue pipelining, then polls all pending
tasks. A metric completion is validated against its purpose and value type. A
user metric publishes into its mailbox; a fault-readback int32 zero records a
successful check, while a nonzero code becomes `DeviceFault`. A non-metric task
returning a metric, or a metric returning no value or the wrong type, is a
`BackendProtocol` failure.

`poll` returns only `LoopStatus::Pending` or `Complete`. The progress variants
also return whether that pass submitted or completed work. `wait` repeats those
nonblocking passes and sleeps with an exponential bounded backoff only when no
progress was made. A scheduler with remaining work but no runnable or pending
task returns `SchedulerStalled`; too many nonprogress passes returns
`WatchdogExpired`.

When an iteration completes, `poll_with_progress_or_stop` evaluates its stop
callback exactly at the boundary before another iteration could begin. If the
callback is true, it records `LoopStopAccepted` and `LoopCompleted`; otherwise
it resets loop completion state and starts the next finalized iteration. A
successful `into_exited_loop` requires the active phase to be complete and no
stored failure. Calling it early returns the original `RunningRun` in a `Box`.

`ExitedLoop::exit` blocks the exit phase. Exit permits device-to-device
transfers and device-to-external transfers only. A completed external exit
allocates exactly its finalized byte count, invokes `collect_exit`, and stores
one `ExitImage`. Once all exit work is complete, teardown releases every arena
in device-map order and then destroys the backend resource. Only after both
steps succeed is `LogicalEvent::Exited` recorded and `ExitedRun` returned.
`ExitedRun::into_parts` yields `(B, MetricMailbox, Vec<ExitImage>, RunJournal)`
for caller-controlled persistence or a subsequent run.

## Scheduler and preparation boundaries

The private prepared-task representation is derived directly from the
finalized bundle. Loop calculations require a finalized iteration domain and
resolve every input, output, and optional fault flag to a
`ResolvedValueLocation`. Loop metrics resolve one value and retain purpose,
metric, and slot IDs. Init external-to-device transfers become
`InitAdmission`; init and loop device-to-device transfers become
`InternalTransfer`; exit device-to-device and device-to-external transfers
become `ExitTransfer`. Calculations and metrics outside the loop, external
admission outside init, loop external transfers, and external ingress during
exit are rejected as `InvalidPhaseTask`.

The scheduler tracks each task as `Remaining`, `Pending`, or `Complete` in a
fixed slot. A task is runnable only when its dependencies are complete (or the
same-queue pipelining contract explicitly permits a pending predecessor), its
iteration domain contains the active zero-based iteration, and its schedule
window does not conflict with another pending window. Sparse tasks outside the
active domain are completed without a backend submission once their
dependencies are satisfied. Completion is entered in the fixed ledger before
dependent tasks are considered.

The executor records every backend physical batch before interpreting the
backend result. A backend error is formatted once into `BackendMessage`; a
journal or protocol error takes precedence when recording the batch fails.
This ordering keeps failure evidence observable without allowing backend error
formatting or journal growth in the live loop.

## Failure and recovery boundaries

There are two result styles for the local lifecycle. The ordinary methods map a
recoverable failure to its primary `ExecutorError`. The `_recoverable` methods
return `RunFailure<B>`, preserving backend ownership and bounded evidence.
Preparation failures before a resource exists have `journal: None`; failures
after binding retain the journal and attempt `destroy_resources`. Failures
after arenas exist release all remaining arenas, then consume and destroy the
resource. Teardown continues after each failure. The first reported failure is
the primary error, and the first later teardown failure is `cleanup_error`.

The failure ordering is therefore:

```text
primary operation or protocol error
    -> record the associated physical/logical evidence
    -> release every extant arena
    -> destroy the backend resource
    -> return backend + journal + primary error + first cleanup error
```

If an arena release fails, the arena has already been removed from the local
map and teardown continues with the remaining entries. If the resource was
already consumed, the lifecycle invariant is surfaced rather than hidden by a
fallback. A journal-capacity failure is itself a real run failure, not a reason
to grow or replace the journal.

The worker path applies the same principle with `WorkerPrepareFailure` during
pre-init and `fatal_cleanup` during an active session. External egress remains
`AwaitingAck` after its native copy completes; the transfer is not marked
complete until `acknowledge_external_egress`. A failed quiesce deliberately
keeps arenas live. `into_parts` refuses to return a worker backend while a
resource or arena remains owned by the session.

## Workspace ownership relationships

`recipe-core` owns the `FinalizedBundle`, typed IDs, resolved locations, task
phases, iteration domains, routes, metrics, and arena layouts consumed here.
`recipe-planner`, `recipe-scheduler`, and `recipe-prepare` produce the immutable
bundle and its measured schedule. `recipe-native-executor`, `recipe-host`, and
native probe adapters implement the sealed `Backend` contract over already
realized resources. `recipe-training` and `recipe` call the local typestate
path. `recipe-remote` derives a `WorkerProjection`, creates a
`WorkerExecutionSession`, and maps its errors and polls to the transport
protocol. None of those consumers may bypass this root boundary by constructing
private phase state or mutating a finalized bundle.
