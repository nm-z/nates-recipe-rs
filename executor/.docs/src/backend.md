# `recipe_executor::backend`

```yaml
document: recipe_executor.backend
source: executor/src/backend.rs
kind: sealed-backend-abi
authority:
  - executor/src/backend.rs
  - executor/src/executor.rs
  - executor/src/worker.rs
  - executor/src/error.rs
  - host/src/backend.rs
  - native-executor/src/cuda.rs
  - native-executor/src/hsa.rs
  - native-executor/src/local.rs
  - native-executor/src/bridge.rs
  - remote/src/executor_driver.rs
```

This document specifies the closed backend boundary used by the typestate
executor. A backend receives one `FinalizedBundle` and immutable, fully
resolved work records. It owns native resources, completion tokens, and arena
handles, while the executor owns task ordering, lifecycle state, and the
authoritative journal. The boundary intentionally has no compiler, loader,
allocator, discovery, topology mutation, or replanning work variant.

Line references identify the current implementation. The enum and method
names below are the parseable contract; line numbers are source anchors, not a
second versioned API.

## 1. Boundary invariants

`Backend` is declared as `trait Backend: sealed::Sealed` (`executor/src/backend.rs:11-14,318-329`).
Only Recipe-owned adapters can implement it. The executor never exposes a
backend reference after moving it into a run. Every associated resource and
pending type is `Debug`; the backend error is an owned `Error + Send + Sync +
'static` value (`executor/src/backend.rs:329-334`).

The runtime contract is:

```text
bind_resources(bundle)
  -> one Resource
prepare_pending(Resource, request) for every task
  -> one Pending token per task, with all reachable native state realized
initialize(images)
  -> allocate every finalized arena
  -> submit and poll Init work
start_loop()
  -> submit and poll Loop work, reusing tokens only when repetition is declared
exit()
  -> submit and poll Exit work
  -> collect external result images
  -> release every arena
  -> destroy Resource
```

`bind_resources` and `prepare_pending` are the only normal preparation points
for allocation, loading, warming, registration, queue creation, completion
creation, staging allocation, or other native realization. `submit`,
`submit_loop_iteration`, `poll`, and `collect_exit` must operate on those
pre-realized objects. In particular, submission, polling, and formatting a
backend error must not allocate, grow a collection, load code, or lazily
initialize driver state (`executor/src/backend.rs:318-328`).

The executor enforces the ownership side of this contract with
`ResourceState::Active` and `ResourceState::Taken`. A resource can be consumed
for destruction once. Reusing a consumed resource is a
`LifecycleInvariant` error (`executor/src/executor.rs:634-674`).

## 2. Resolved work records

`RunPhase` is the total order `Init`, `Loop`, `Exit` (`core/src/schedule.rs:14-19`).
`FinalizedBundle` supplies resolved values, transfer endpoints, arena offsets,
routes, lane claims, artifacts, and submission slots. The executor converts
each finalized `Task` into one `BackendWork` value in
`PreparedTask::backend_work` (`executor/src/executor.rs:1834-1931`).

### 2.1 Work classes

| `WorkClass` | source task and legal phase | backend variant | completion value |
| --- | --- | --- | --- |
| `InitAdmission` | `Transfer(External -> Device)` in `Init` | `BackendWork::InitAdmission` | `None` |
| `Calculation` | `Calculation` in `Loop` | `BackendWork::Calculation` | `None` |
| `InternalTransfer` | `Transfer(Device -> Device)` in `Init` or `Loop` | `BackendWork::InternalTransfer` | `None` |
| `Metric` | `Metric` in `Loop` | `BackendWork::Metric` | `Some(MetricValue)` |
| `ExitTransfer` | `Transfer(Device -> Device)` or `Transfer(Device -> External)` in `Exit` | `BackendWork::ExitTransfer` | `None` |

The phase and endpoint checks are made while constructing `PreparedTask`
(`executor/src/executor.rs:1647-1819`). A calculation or metric outside the
loop, a loop transfer involving an external endpoint, an init transfer that is
not admission or internal movement, and an exit transfer that admits external
data are `ExecutorError::InvalidPhaseTask`. No backend is asked to interpret an
invalid task shape.

### 2.2 `InitAdmissionWork`

```text
InitAdmissionWork {
    task: TaskId,
    destination: ResolvedValueLocation,
    bytes: ByteCount,
    submission: SubmissionSlots,
    image: &'a [u8],
}
```

`destination` is the exact finalized device value location. `bytes` must equal
the finalized init-image manifest and `image.len()` when converted to
`u64`. `submission` names the preplanned queue and completion slot. `image` is
a borrow of caller-owned or executor-validated init storage and is valid for
the submission call only. The executor validates one image per required device,
its value identity, and its exact byte count before creating this record
(`executor/src/executor.rs:2073-2258`).

An admission is one logical task. A backend may report multiple
`PhysicalCall::AdmissionChunk` records when its physical path is chunked. The
current host, CUDA, HSA, and local adapters each report one index-zero chunk.

### 2.3 `CalculationWork`

```text
CalculationWork {
    task: TaskId,
    run: RunId,
    iteration: LoopIteration,
    device: DeviceId,
    kernel_template: KernelTemplateId,
    artifact: ArtifactId,
    submission: SubmissionSlots,
    inputs: &'a [ResolvedValueLocation],
    outputs: &'a [ResolvedValueLocation],
    fault_flag: Option<ResolvedValueLocation>,
}
```

`iteration` is the exact zero-based finalized loop iteration, not a count of
submissions. The executor may first activate a sparse task at a nonzero index;
the backend must use the supplied iteration for recurrent-state and schedule
addressing. `inputs`, `outputs`, and the optional int32 `fault_flag` are borrowed
slices of finalized locations. `kernel_template` identifies Recipe operation
semantics; `artifact` identifies the realized native kernel. The host adapter
rejects this variant with `UnsupportedWork`, while CUDA and HSA validate the
artifact ABI and dispatch the native kernel.

### 2.4 `TransferWork`

```text
TransferWork {
    task: TaskId,
    source: ResolvedTransferEndpoint,
    destination: ResolvedTransferEndpoint,
    bytes: ByteCount,
    route: &'a [LinkId],
    lane_claims: &'a [TransferLaneClaim],
    submission: SubmissionSlots,
}
```

`ResolvedTransferEndpoint` is either `External` or a finalized device value
location (`core/src/schedule.rs:400-405`). A transfer route and lane-claim slice
are copied from the immutable schedule. Multi-link paths are represented by
dependency-chained transfer tasks, so one backend submission sees one resolved
hop. The class is carried by the surrounding `BackendWork` variant, not by
`TransferWork` itself.

### 2.5 `MetricWork`

```text
MetricWork {
    task: TaskId,
    iteration: LoopIteration,
    purpose: MetricPurpose,
    metric: MetricId,
    slot: MetricSlotId,
    value: ResolvedValueLocation,
    submission: SubmissionSlots,
}
```

Metrics are specialized four-byte device readback transfers. `MetricPurpose::User`
publishes newest-value-wins telemetry through `MetricMailbox`; the executor
requires a matching `MetricValue` on completion. `MetricPurpose::FaultReadback`
requires `MetricValue::I32(0)` for success, raises `DeviceFault` for a nonzero
int32, and rejects an f32 result. A metric completion without a value, or a
non-metric completion with a value, is `BackendProtocol`
(`executor/src/executor.rs:2508-2584`).

### 2.6 `BackendWork` identity helpers

`BackendWork<'a>` is `Copy`, contains exactly the five variants above, and
borrows only finalized slices or the init image (`executor/src/backend.rs:73-84`).
`task()` returns the embedded `TaskId`; `class()` maps variants one-to-one to
`WorkClass` (`executor/src/backend.rs:86-107`). Backends should dispatch on the
variant and use these helpers for accounting rather than reconstructing task
kind or phase from other state.

## 3. Preparation records and arena views

### 3.1 `PendingRequest`

```text
PendingRequest {
    task: TaskId,
    phase: RunPhase,
    class: WorkClass,
    submission: Option<SubmissionSlots>,
}
```

The executor creates one request for each task in each prepared phase before
`init` (`executor/src/executor.rs:2085-2120`). All current local task classes
carry `Some(submission)`. `Option` leaves room for a backend-owned operation
whose completion token has no queue/completion pair, and is also the shape used
by the worker projection. The request must match the immutable finalized task.
The returned `Pending` token must own every resource reachable by submission
and polling, including a queue slot, completion object, staging buffer, metric
buffer, event, or native state. A token cannot be prepared twice.

Worker external transfers are not represented by `PendingRequest`; they use the
separate `WorkerBackend::prepare_external` hook before init.

### 3.2 `ArenaSet<'a, A>`

`ArenaSet` is a read-only borrow of the executor's `BTreeMap<DeviceId, A>`
(`executor/src/backend.rs:123-149`). `new` is hidden because only composite
backends and pre-final warm executors should construct it. `get`, `iter`,
`len`, and `is_empty` never mutate or extend the map. The executor passes a
fresh view to every submission and exit collection. Arena allocation happens
outside the view through `allocate_arena`; a backend cannot add a missing arena
through `ArenaSet`.

### 3.3 `BackendPoll`

`BackendPoll::Pending` is nonblocking and leaves the token active. `Complete {
metric: None }` marks a non-metric task terminal. `Complete { metric: Some(_) }`
is legal only for a metric task. The executor calls `poll` once per pending
slot per bounded scheduler pass and converts a terminal result into a
`CompletionLedger` entry (`executor/src/executor.rs:2281-2427`).

## 4. Physical accounting ABI

Logical events describe the run contract. `PhysicalCall` describes native
actions reported by a backend. The two streams are deliberately separate: one
logical admission can be chunked, and repeated loop activity is compacted in
the production journal (`executor/src/executor.rs:151-226,449-597`).

### 4.1 Physical call vocabulary

| record | meaning and producer |
| --- | --- |
| `BindResources` | `bind_resources` or `bind_worker_resources` accepted one resource set |
| `PreparePending { task }` | base `prepare_pending` realized one local token |
| `PrepareExternal { task }` | `WorkerBackend::prepare_external` realized one cross-machine token |
| `AllocateArena { device, bytes }` | one finalized arena was allocated |
| `AdmissionChunk { task, device, bytes, chunk_index }` | one physical init-image chunk was submitted |
| `SubmitCalculation { task }` | one calculation kernel submission |
| `SubmitInternalTransfer { task }` | one device-to-device internal copy |
| `SubmitMetric { task, slot }` | one metric readback submission |
| `SubmitExitTransfer { task }` | one exit transfer submission |
| `SubmitExternalIngress { task, bytes }` | worker-side master-to-worker transfer submission |
| `SubmitExternalEgress { task, bytes }` | worker-side worker-to-master transfer submission |
| `AcknowledgeExternalEgress { task }` | worker-side acknowledgement that caller consumed egress bytes |
| `QuiesceWorker` | worker backend drained or quiesced native work |
| `CollectExit { task, bytes }` | completed external exit payload copied into caller storage |
| `Poll { task, status }` | one completion query, with `Pending`, `Complete`, or `Failed` status |
| `ReleaseArena { device }` | one arena handle was released |
| `DestroyResources` | the backend resource set was consumed for teardown |

`PhysicalPollStatus` is only the status attached to `Poll`; a backend result
error is still returned through its `Error` type after recording `Failed`.

### 4.2 Fixed-capacity batches

`MAX_PHYSICAL_CALLS_PER_OPERATION` is exactly `16`
(`executor/src/backend.rs:229-233`). `PhysicalCallBatch` stores an inline
`[Option<PhysicalCall>; 16]` and a `u8` prefix length. `new` and `Default`
produce an empty batch; `single` creates a one-record batch;
`try_push` appends or returns `PhysicalCallBatchOverflow` at capacity;
`try_from_array` rejects an array larger than 16; `iter` yields the initialized
prefix; `len` and `is_empty` inspect the prefix
(`executor/src/backend.rs:235-301`). There is no owned collection return from a
backend operation, so the loop boundary cannot hide allocation or unbounded
growth.

`PhysicalCallBatchOverflow` implements `Display` and `Error`. Host maps it to
`Error::PhysicalReportOverflow`; native adapters map it to
`PhysicalAccountingOverflow`; the executor maps backend errors to
`ExecutorError::Backend` after retaining the batch. The per-backend
`MAX_NON_POLL_PHYSICAL_CALLS` must be nonzero and no larger than 16. The
executor rejects any other declaration while deriving `JournalCapacity`
(`executor/src/executor.rs:253-350`). Host, CUDA, HSA, and `LocalBackend` all
declare `1`.

`Backend::poll` has a stricter cardinality rule than other methods: it must
append exactly one `PhysicalCall::Poll` for the queried task and no other
physical record, with status matching the returned result
(`executor/src/backend.rs:411-422`). The executor records one terminal poll
and retains only the first pending marker per task in ordered storage; all
pending counts remain exact in a fixed task-indexed counter table. Repeated
loop calls are compacted after iteration zero, but observed and compacted
totals remain in `JournalSummary`.

## 5. `Backend` trait method contract

The following ordering is required for one whole-bundle run. A method may
return its associated backend error at any step. The executor records physical
calls before converting that error to `ExecutorError::Backend`.

### 5.1 `bind_resources`

```text
fn bind_resources(
    &mut self,
    bundle: &FinalizedBundle,
    physical_calls: &mut PhysicalCallBatch,
) -> Result<Self::Resource, Self::Error>
```

This is the one whole-bundle binding operation. It validates the bundle against
configured device bindings, artifacts, queues, reservations, and any warm
handoff, then returns one resource owner. The executor calls it first in
`PreparedRun::prepare_with_journal_capacity_recoverable`
(`executor/src/executor.rs:827-980`). A failed bind returns the backend to
`RunFailure` without a resource to destroy. A successful resource is destroyed
on any later preparation, initialization, loop, exit, or teardown failure.

### 5.2 `prepare_pending`

```text
fn prepare_pending(
    &mut self,
    resource: &mut Self::Resource,
    request: PendingRequest,
    physical_calls: &mut PhysicalCallBatch,
) -> Result<Self::Pending, Self::Error>
```

`realize_phase` calls this once for every prepared init, loop, and exit task
before `PreparedRun` is returned (`executor/src/executor.rs:2085-2120`). The
token must be reusable for every activation permitted by
`supports_loop_repetition`. A failure after earlier tokens were prepared drops
those tokens and calls `destroy_resources` for the resource. No later method
may allocate a missing token.

### 5.3 `allocate_arena`

```text
fn allocate_arena(
    &mut self,
    resource: &mut Self::Resource,
    layout: &ArenaLayout,
    physical_calls: &mut PhysicalCallBatch,
) -> Result<Self::Arena, Self::Error>
```

`initialize_recoverable` calls it once per finalized `ArenaLayout`, stores the
returned arena by `DeviceId`, and records `LogicalEvent::ArenaAllocated`
(`executor/src/executor.rs:985-1037`). All arenas exist before any init
submission. If one allocation fails, already allocated arenas are released in
teardown order and the resource is destroyed. The `Arena` value remains owned
by the executor until `release_arena` consumes it.

### 5.4 Repetition and same-queue capabilities

`supports_loop_repetition` defaults to `false`. `PreparedRun` rejects an
unbounded loop or a finite loop count greater than one with
`LoopRepetitionUnsupported` unless the backend returns `true`
(`executor/src/executor.rs:827-855`). A true result is a promise that each
loop token can reach terminal completion and be rearmed without allocation or
replacement. The first active iteration for a sparse task may be nonzero, so
`submit_loop_iteration` must inspect the token's activation state, not infer
freshness from the global iteration index (`executor/src/backend.rs:364-370,391-409`).

`supports_same_queue_pipelining` defaults to `false`. When true for a task,
the scheduler may submit that task behind an incomplete predecessor on the
same native queue and may treat the queue's ordering as sufficient for the
dependency. The backend must retain every submitted resource until queue idle
and ensure queue ordering makes that dependency safe
(`executor/src/backend.rs:372-380`; scheduler use at
`executor/src/executor.rs:2314-2360`). This capability is independent of loop
repetition.

### 5.5 `submit`

```text
fn submit(
    &mut self,
    resource: &mut Self::Resource,
    arenas: ArenaSet<'_, Self::Arena>,
    pending: &mut Self::Pending,
    work: BackendWork<'_>,
    physical_calls: &mut PhysicalCallBatch,
) -> Result<(), Self::Error>
```

The executor uses `submit` for `Init` and `Exit`, and the default
`submit_loop_iteration` implementation also forwards to it. The supplied
pending token must belong to `work.task()`, have the matching class and
submission slots, and be in its not-yet-submitted state. The backend validates
all finalized addresses, artifact identity, route and lane claims, byte count,
and native queue/completion ownership before enqueueing work. A successful
submission leaves the token active; the executor marks its slot `Pending` and
records the matching physical and logical submission event.

### 5.6 `submit_loop_iteration`

```text
fn submit_loop_iteration(
    &mut self,
    resource: &mut Self::Resource,
    arenas: ArenaSet<'_, Self::Arena>,
    pending: &mut Self::Pending,
    iteration: LoopIteration,
    work: BackendWork<'_>,
    physical_calls: &mut PhysicalCallBatch,
) -> Result<(), Self::Error>
```

The executor calls this only for active loop tasks
(`executor/src/executor.rs:2430-2476`). The default ignores `iteration` and
calls `submit`, preserving one-shot behavior. Repeatable adapters override it
to either use an untouched prepared token or rearm a terminal token before
submission, without allocating. Host calls `prepare_loop_pending`; CUDA and
HSA reset terminal native completion state; the staged bridge resets its worker,
source leg, destination leg, and middle job.

### 5.7 `poll`

```text
fn poll(
    &mut self,
    resource: &mut Self::Resource,
    pending: &mut Self::Pending,
    physical_calls: &mut PhysicalCallBatch,
) -> Result<BackendPoll, Self::Error>
```

Polling is nonblocking. `Pending` leaves the token active and increments the
executor's exact per-task pending-poll counter. `Complete` must leave all
resources needed by a subsequent `collect_exit` or loop rearm in a terminal
state. A native failure is reported as `PhysicalPollStatus::Failed`, then
propagated as `BackendOperation::Poll { task }`. The executor does not retry a
failed poll or substitute a different token.

### 5.8 `collect_exit`

```text
fn collect_exit(
    &mut self,
    resource: &mut Self::Resource,
    arenas: ArenaSet<'_, Self::Arena>,
    pending: &mut Self::Pending,
    work: TransferWork<'_>,
    destination: &mut [u8],
    physical_calls: &mut PhysicalCallBatch,
) -> Result<(), Self::Error>
```

The executor calls this only after an `ExitTransfer` whose destination is
`External` has polled complete. It allocates the caller-owned result image,
checks its size, passes a mutable slice to the backend, and stores the image as
an `ExitImage` (`executor/src/executor.rs:2617-2668`). A backend must not perform
exit collection during the live loop. Device-to-device exit transfers complete
normally and do not invoke this hook.

### 5.9 `release_arena` and `destroy_resources`

`release_arena` consumes one arena and receives its expected `DeviceId`.
`teardown_resources` first takes the entire arena map, releases every arena,
then consumes the resource and calls `destroy_resources`
(`executor/src/executor.rs:1489-1547`). It records the first teardown error as
the primary failure and the next as `cleanup_error`, while continuing through
all remaining arenas and destruction. A backend must therefore tolerate
teardown after a prior operation failure and must not release a handle twice.

`destroy_resources` consumes `Self::Resource`. It is also used when preparation
fails after binding, before a `PreparedRun` exists. Successful destruction is
the only point at which backend-wide queues, modules, runtime sessions,
staging pools, bridge workers, and pending pools may be finally dropped.

## 6. Whole-run call graph and ownership

### 6.1 Preparation

`PreparedRun::prepare_recoverable` first derives the exact journal capacity and
rejects unsupported loop repetition. It builds `PreparedPhases`, calls
`bind_resources`, then calls `prepare_pending` for every init, loop, and exit
task. The resulting `RunCore` contains the backend, active resource, empty
arena map, completion ledger, metric mailbox, exit-image capacity, journal,
and watchdog (`executor/src/executor.rs:775-978`).

The backend value stays owned by `RunCore`. Every `Pending` value stays inside
one `TaskSlot` in one `PhaseState`; no token is created during `poll`.

### 6.2 Init and loop

`PreparedRun::initialize_recoverable` validates all provided `DeviceImage`
records, allocates arenas, and drives the init phase with `run_phase_blocking`.
`InitializedRun::start_loop_recoverable` begins iteration zero and transitions
to `RunningRun`. `poll_phase_once` performs one bounded pass: mark inactive
tasks complete when dependencies are ready, submit runnable tasks, poll every
pending slot, and complete the phase when no remaining or pending slot exists
(`executor/src/executor.rs:2263-2427`).

For a repeated loop, the executor resets only loop completion entries, advances
the finalized `LoopIteration`, and invokes `submit_loop_iteration` on the next
active task set. Inactive sparse tasks are marked complete without backend
calls. A missing runnable task with no pending task is `SchedulerStalled`; too
many no-progress passes are `WatchdogExpired`.

### 6.3 Exit and teardown

After the loop reaches terminal completion, `RunningRun::into_exited_loop` is
legal. `ExitedLoop::exit_recoverable` drives exit tasks, calls `collect_exit`
for each external result, releases all arenas, consumes the resource through
`destroy_resources`, and records `LogicalEvent::Exited`
(`executor/src/executor.rs:1323-1424,1489-1547`). The returned `ExitedRun`
contains the backend value, metric mailbox, external images, and journal for
caller-controlled persistence. The backend is no longer attached to an active
resource or pending phase.

### 6.4 Failure ownership

`RunFailure<B>` always preserves the backend value. If failure occurs before a
resource is bound, its journal is `None`; after binding, it contains the
bounded journal and the backend has undergone best-effort teardown. `into_parts`
recovers the backend, primary `ExecutorError`, optional cleanup error, bundle
identity, run ID, and journal (`executor/src/executor.rs:693-773`). A backend
error is formatted into a fixed `BackendMessage` of 96 bytes and wrapped as
`ExecutorError::Backend { operation, message }` by `backend_value`
(`executor/src/executor.rs:2695-2714`; `executor/src/error.rs:1-120`).

The executor never retries a failed backend operation, silently drops a
physical-report overflow, or replaces a consumed resource. Protocol errors
remain visible as `BackendProtocol` or the backend's wrapped error.

## 7. Worker extension

`WorkerBackend` extends `Backend` for a remote worker projection
(`executor/src/worker.rs:1017-1082`). The base ABI handles local projected
tasks, local arenas, local polling, and resource destruction. The extension
adds only the external cross-machine hooks:

| hook | contract |
| --- | --- |
| `bind_worker_resources` | bind the exact immutable `WorkerProjection`; emits `BindResources` |
| `prepare_external` | realize one `WorkerExternalTransfer` token before init; emits `PrepareExternal` |
| `begin_external_ingress` | submit master-to-worker bytes into a local finalized value; emits `SubmitExternalIngress` |
| `begin_external_egress` | submit worker-to-master bytes into caller storage; emits `SubmitExternalEgress` |
| `poll_external` | return `ExternalTransferPoll::Pending` or exact completed byte count |
| `acknowledge_external_egress` | release or recycle an egress token after the caller consumed its buffer; emits `AcknowledgeExternalEgress` |
| `quiesce_worker` | make every native queue safe for arena release; emits `QuiesceWorker` |

External pending tokens are stored separately from `Backend::Pending` in
`WorkerPending::External`. Local tasks use `WorkerPending::Local` and the base
`submit` and `poll` methods. `WorkerExecutionSession::prepare` binds the
projection, prepares every local or external token, and records `Prepared`
before accepting an image (`executor/src/worker.rs:1401-1590`).

The worker lifecycle is closed:

```text
Prepared -> Init -> Loop -> Exit -> Finished
                    \-> Cancelling -> Finished
any active state --fatal_cleanup--> Failed
```

Init images arrive in ordered chunks. The worker allocates the matching arena,
checks offset, exact byte count, and SHA-256 digest, submits the projected
admission task, and polls it to completion. Loop local tasks and external
ingress/egress tasks are checked for run ID, active phase, role, dependencies,
schedule conflicts, and duplicate dispatch. Egress reaches `AwaitingAck` after
native completion and is not logically complete until acknowledgement.
`cancel` first quiesces; `release_arena` is legal only after completed exit or
cancellation; `finish` requires every arena released and consumes the resource
(`executor/src/worker.rs:1607-2189`).

Worker errors distinguish projection validation, wrong phase or role, unknown
task/device, duplicate dispatch, dependency or schedule conflict, byte/digest
mismatch, metric contract, device fault, watchdog expiration, backend failure,
journal failure, and capacity overflow (`executor/src/worker.rs:1119-1318`).
`fatal_cleanup` deliberately preserves arenas when quiescing fails because
native work may still reference them. Once quiescence succeeds it releases
each arena once, clears image buffers, destroys resources, and preserves the
first error (`executor/src/worker.rs:2197-2272`).

`remote/src/executor_driver.rs` is a generic `ExecutorWorkerDriver<B>` caller.
It maps wire operations to `WorkerExecutionSession`, recovers the backend after
`finish`, and calls `fatal_cleanup` on drop or a remote fault. No concrete
`impl WorkerBackend` exists in this checkout; an embedding crate must supply
that sealed implementation before it can instantiate this generic worker
driver.

## 8. Current backend implementations

All four concrete whole-bundle implementations (host, CUDA, HSA, and local)
declare
`MAX_NON_POLL_PHYSICAL_CALLS = 1` and record one physical action per non-poll
operation. The native accounting helper maps a `BackendWork` variant to
`AdmissionChunk`, `SubmitCalculation`, `SubmitInternalTransfer`,
`SubmitMetric`, or `SubmitExitTransfer` (`native-executor/src/accounting.rs:1-36`).

### 8.1 HostBackend

`recipe_host::HostBackend` implements the trait with:

```text
Arena    = recipe_host::Arena
Resource = HostResources
Pending  = HostPending
Error    = recipe_host::Error
```

(`host/src/backend.rs:1121-1127`). Its state is `Ready(config)`,
`Prepared`, `Warmed`, or `Bound` (`host/src/backend.rs:103-123`). Binding
validates host RAM or disk bindings, reservations, task contracts, and runtime
slot/staging capacity. A second bind is `BackendState`.

`HostResources::prepare_pending` validates task, phase, class, and submission,
then either consumes a pre-final pending pool entry or prepares a runtime copy
slot and any admission, metric, or egress staging arena
(`host/src/backend.rs:684-742`). `allocate_arena` creates a RAM or disk arena
for the exact layout. `supports_loop_repetition` is `true`; loop submission
resets a terminal `PendingCopy` and reuses its staging allocation.

Host submission validates the finalized contract and pending state. Admission
writes the image to preallocated staging and submits a copy into the destination
arena. Internal and exit transfers validate route, lane claims, endpoints, and
bytes. A metric copies exactly four bytes into staging. A calculation returns
`UnsupportedWork` because payload calculation belongs to CUDA or HSA. Any
submission or polling error poisons the resource, so later operations fail
closed with `Poisoned` (`host/src/backend.rs:758-902`).

Host polling maps runtime `PendingCopy` status to `BackendPoll`, decodes f32 or
i32 metric bytes, and marks the token terminal. Exit collection reads the
completed egress staging arena into the caller slice. Release checks arena
device identity and closes it; destruction drops pending slots and closes the
runtime (`host/src/backend.rs:904-939,974-1014,1262-1283`).

### 8.2 CudaBackend

`recipe_native_executor::CudaBackend` implements:

```text
Arena    = CudaArena<'context>
Resource = CudaResources<'context>
Pending  = CudaPending<'context>
Error    = recipe_native_executor::Error
```

(`native-executor/src/cuda.rs:1193-1199`). Binding validates the immutable
execution plan and runtime artifacts, exact device bindings, queues,
completions, modules, kernel entries, persistent metric/scratch/staging
allocations, and warm handoff state. `CudaPending` records task, phase, class,
device, queue, completion, native pending state, action, and terminal status
(`native-executor/src/cuda.rs:221-227,348-421,1382-1449`).

Preparation verifies that the planned queue and completion slots were realized
and rejects duplicate token preparation. Arena allocation creates one
`DeviceBuffer` per finalized layout. Loop repetition is supported and
same-queue pipelining is explicitly supported. `submit_loop_iteration` uses a
fresh `Ready` token or rearms a terminal token; an active token cannot be
submitted again.

Submission validates task, class, submission slots, route/lane claims, device
and arena bounds, then dispatches one of: H2D admission, Recipe kernel
calculation, same-context D2D transfer, four-byte metric D2H, or external exit
egress. A native submission error poisons resources. Poll queries an event or
queue idle state, records exactly one `Poll`, and on completion releases the
completion slot and decodes any metric or egress action. Exit collection copies
from the preallocated host result buffer. Arena release checks device identity
before freeing the CUDA buffer; destruction closes all devices and native
state (`native-executor/src/cuda.rs:480-647,1193-1367`).

### 8.3 HsaBackend

`recipe_native_executor::HsaBackend` implements:

```text
Arena    = HsaArena<'scope>
Resource = HsaResources<'scope>
Pending  = HsaPending<'scope>
Error    = recipe_native_executor::Error
```

(`native-executor/src/hsa.rs:1381-1387`). HSA binding validates exact artifact
targets, CPU kernarg and fine-grained host pools, GPU queues and completion
signals, reservation policy, admission manifests, and warm handoff. Its
resource owns per-device queues, artifact kernels, metric buffers, pending
pool, task contracts, and a poisoned bit. `HsaPendingState` is `Ready`,
`Active`, or `Terminal`; terminal loop tokens are reset in place
(`native-executor/src/hsa.rs:253-260,1053-1084,1583-1675`).

Preparation and arena allocation are allocation points only. Loop repetition is
supported, but same-queue pipelining is not overridden, so the executor keeps
the default terminal-token scheduling discipline. Submission fills prepared
kernargs, dispatches HSA kernels, performs HSA copies, and stages four-byte
metrics or external egress. Poll releases completion signals, marks tokens
terminal, and returns decoded metric values. HSA errors poison the resource and
are recorded as failed polls. Exit collection validates the terminal egress
action and copies from the host-visible allocation. Release checks device
identity and closes the allocation; destruction drops pending tokens and closes
all device resources (`native-executor/src/hsa.rs:1389-1573`).

### 8.4 LocalBackend and bridge composition

`LocalBackend<'cuda, 'hsa, Bridge>` composes host, CUDA, HSA, and one
cross-backend bridge:

```text
Arena    = LocalArena<'cuda, 'hsa>       // Host | Cuda | Hsa
Resource = LocalResources<..., Bridge::Resource>
Pending  = LocalPending<..., Bridge::Pending>
Error    = LocalError<Bridge::Error>
```

(`native-executor/src/local.rs:615-633,1647-1655`). `bind_resources`
classifies immutable devices and tasks, binds each partition, and binds the
bridge. `prepare_pending` and `allocate_arena` dispatch by immutable
`TaskOwner` or device class. `submit`, `submit_loop_iteration`, `poll`, and
`collect_exit` dispatch to the matching host, CUDA, HSA, or bridge resource and
reject a pending token or arena owned by another partition
(`native-executor/src/local.rs:1657-2023`).

Local same-queue pipelining is true only for tasks owned by CUDA. Loop
repetition delegates to `Bridge::supports_loop_repetition`, because bridge
tokens also participate in the loop. Local destruction records one destroy
call, then destroys bridge, HSA, CUDA, and optional host resources in that
order, retaining the first error and publishing native execution evidence
only after all destruction succeeds (`native-executor/src/local.rs:2064-2110`).

`CrossBackendTransfer` is a smaller bridge ABI, not a second whole-bundle
`Backend`. It binds immutable one-hop device-to-device transfers, prepares a
bridge token, submits and polls staged source/middle/destination legs, supports
optional loop rearming, and destroys its resource
(`native-executor/src/local.rs:125-182`; staged implementation at
`native-executor/src/bridge.rs:1066-1291`). The staged bridge owns host worker
threads and native legs. Its poll state moves `Source -> Middle -> Destination
-> Complete`; host destinations complete after the middle job, native
destinations add a destination leg. Repetition resets the worker, both native
legs, and middle job only from `Complete`.

`RejectCrossBackend` deliberately fails bind when any bridge task is present.
`PreparedBridge` transfers a candidate-created bridge resource exactly once
after handoff validation and delegates all bridge methods and repetition to the
underlying implementation (`native-executor/src/local.rs:238-338,513-612`).

The `Backend` implementations in `native-probe/src/cuda.rs` and
`native-probe/src/hsa.rs` are for that crate's private discovery/benchmark
trait (`crate::native::Backend`), not `recipe_executor::Backend`; they do not
add executor adapters.

## 9. Caller and failure map

| caller | backend boundary used | evidence |
| --- | --- | --- |
| `training/src/execute.rs` | obtains a prepared local backend, then calls `PreparedRun::prepare`, `initialize`, `start_loop`, `poll`, and `exit` for training and inference | `training/src/execute.rs:1247-1305,1362-1420,2225-2260` |
| `acceptance/src/main.rs` | consumes `RunJournal::physical_calls` to require one bind and one destroy and count calculation, transfer, and metric submissions | `acceptance/src/main.rs:1103-1118,1243-1288` |
| `remote/src/executor_driver.rs` | maps wire worker operations to `WorkerExecutionSession<B>` and recovers the backend after finish or fatal cleanup | `remote/src/executor_driver.rs:48-188,239-355` |
| `executor/src/worker.rs` | drives the `WorkerBackend` extension and records base and external physical calls in the same journal | `executor/src/worker.rs:1401-1590,1720-2189` |

The executor's error wrapper is deterministic and bounded:

```text
backend Error --format into <= 96 bytes--> BackendMessage
             --operation label-----------> ExecutorError::Backend
```

`BackendOperation` names `BindResources`, `PreparePending`, `AllocateArena`,
`Submit`, `Poll`, `CollectExit`, `ReleaseArena`, and `DestroyResources`
(`executor/src/error.rs:92-105`). Backend protocol violations detected by the
executor itself use `ExecutorError::BackendProtocol`; scheduler deadlock and
no-progress failures use `SchedulerStalled` and `WatchdogExpired`. Cleanup
errors never replace the primary run failure, but the first one is preserved in
`RunFailure::cleanup_error` or `WorkerPrepareFailure::cleanup_error`.

The required recovery rule is therefore explicit: inspect the primary
operation and its journal first, inspect the optional cleanup error second, and
use the recovered backend only after the caller has chosen whether to destroy,
rebind, or retain it. No fallback backend, retry path, or alternate resource
source is part of this ABI.
