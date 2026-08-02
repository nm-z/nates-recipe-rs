# `recipe-executor`

`recipe-executor` is Recipe's backend-neutral runtime for one immutable
`recipe_core::FinalizedBundle`. It owns the execution state machine, dependency
dispatch, completion polling, metric publication, fixed-capacity accounting,
exit-image collection, and ordered teardown. It does not discover hardware,
compile or load a kernel, allocate a driver queue, choose a transfer route, or
mutate a finalized plan.

The crate has two related boundaries:

* `Backend` executes a complete finalized bundle in one process through the
  owned typestate sequence `PreparedRun -> InitializedRun -> RunningRun ->
  ExitedLoop -> ExitedRun`.
* `WorkerBackend` executes the exact local projection of a bundle on one remote
  worker. `WorkerExecutionSession` adds chunked init images and explicit
  cross-machine ingress and egress to the same backend primitives.

Both boundaries are deliberately closed. The executor can only hand a backend
one of the finalized calculation, transfer, admission, or metric values that
it resolved before `init`. All native resources reachable from a running
operation are required to be realized before the first init admission.

## Manifest and module graph

`executor/Cargo.toml` declares package `recipe-executor` version `0.1.0`, Rust
edition 2024, MIT licensing, and the description “Backend-neutral, typestate
execution lifecycle for finalized Recipe bundles”. Its only dependencies are
`recipe-core` (the immutable graph, schedule, resource, and identity contract)
and `sha2` (the worker-projection digest). There are no driver, runtime,
compiler, allocator, filesystem, network, or async-runtime dependencies.

The crate root, [`src/lib.rs`](../src/lib.rs), forbids unsafe code and denies
missing `Debug` implementations. It declares five private implementation
modules and re-exports their public contract types:

```text
recipe_executor
├── backend.rs   closed Backend ABI, work values, arenas, polls, physical calls
├── error.rs     fixed backend diagnostics and ExecutorError
├── executor.rs  whole-bundle typestate state machine, scheduler, journal
├── metrics.rs   bounded newest-value-wins metric mailbox
└── worker.rs    remote projection, worker ABI, and worker session lifecycle
```

The internal dependency direction is intentionally one way:

```text
recipe_core::FinalizedBundle
          |
          +--> backend.rs -------> metrics.rs
          |          |
          |          +------------> executor.rs
          |
          +----------------------> worker.rs
```

`executor.rs` uses the work structs and backend trait from `backend.rs`, the
error vocabulary from `error.rs`, and the metric mailbox from `metrics.rs`.
`worker.rs` reuses all of those pieces and adds a projection-specific native
transfer ABI. `sha2` is used only for the deterministic worker projection
identity and for remote init-image digest verification. The crate root exposes
the sealed marker, so only Recipe-owned adapter crates can implement the
backend traits.

The most important downstream edges are:

| Caller or adapter | Role at the executor boundary |
| --- | --- |
| `recipe-core` | Supplies the validated `FinalizedBundle`, task kinds, iteration domains, resolved value locations, resource slots, init-image manifests, and arena layouts. |
| `recipe-host` | Implements `Backend` for preallocated RAM and disk transfers, staging, and metric readbacks. It rejects calculation work. |
| `recipe-native-executor` | Implements `Backend` for CUDA, HSA, and the composed local backend. It also realizes artifacts, queues, completion objects, staging, and arenas before handoff. |
| `recipe-remote` | Adapts `WorkerExecutionSession` to the bounded master/worker protocol through `ExecutorWorkerDriver`. |
| `recipe-training` | Builds the final bundle, obtains a warmed `LocalBackend`, supplies init images, drives the loop, drains metrics, collects outputs, and maps failures. |
| the root `recipe` facade and acceptance runner | Expose completed reports and inspect `RunJournal` and native evidence after the real lifecycle has exited. |

Preparation, compilation, artifact realization, and capacity stabilization are
therefore upstream of this crate. The executor receives the product of those
stages, not their mutable working state.

## Finalized input contract

The executor treats a `FinalizedBundle` as authoritative. Finalize has already
resolved every logical value to a `ResolvedValueLocation`, assigned every task
to `RunPhase::Init`, `RunPhase::Loop`, or `RunPhase::Exit`, assigned schedule
windows and dependencies, selected queue and completion slots, packed one init
image per required device, and produced physical arena layouts. The executor
never recomputes any of those choices.

The three core task kinds remain the complete model ontology:

* `TaskKind::Calculation` is loop-only GPU calculation. Its work names the
  finalized device, kernel template, artifact, resolved input and output
  locations, and optional int32 fault flag.
* `TaskKind::Transfer` is byte movement. Init external-to-device tasks are
  admissions, init or loop device-to-device tasks are internal transfers, and
  exit device-to-device or device-to-external tasks are exit transfers. A
  multi-link path must already have been expanded by planning into dependency
  chained one-hop tasks.
* `TaskKind::Metric` is a four-byte device readback. A user metric publishes a
  nonblocking sample, while a `FaultReadback` metric is the mandatory checked
  calculation status readback. It is not a third kind of model work.

The executor relies on the finalized invariants rather than adding fallback
interpretations. If a task kind is in the wrong phase, if a value has no
resolved location, or if an endpoint has no finalized resolution,
`PreparedTask::new` reports `InvalidPhaseTask` or `BackendProtocol` and the run
does not enter `init`.

## Closed backend ABI

[`src/backend.rs`](../src/backend.rs) is the only native execution contract.
The `sealed::Sealed` marker prevents arbitrary external implementations while
allowing the host, CUDA, HSA, composed-local, and worker adapters to share one
runtime.

### Work values

`BackendWork<'a>` has exactly five variants, with `task()` and `class()` as the
single variant-dispatch helpers:

| Variant | Payload and owner |
| --- | --- |
| `InitAdmission` | `InitAdmissionWork`: one packed device image, destination value location, byte count, and finalized submission slots. |
| `Calculation` | `CalculationWork`: run identity, zero-based `LoopIteration`, device, kernel template, artifact, resolved inputs and outputs, optional fault flag, and submission slots. |
| `InternalTransfer` | `TransferWork` for one finalized device-to-device task in init or loop. |
| `Metric` | `MetricWork`: iteration, metric purpose and identity, metric slot, resolved four-byte value, and submission slots. |
| `ExitTransfer` | `TransferWork` for one finalized exit transfer. External exits are copied to an `ExitImage` only after their backend token completes. |

`PendingRequest` is the pre-init description used to realize a reusable
completion token. It carries task, phase, work class, and optional submission
slots. `ArenaSet` is a read-only view of the exact per-device arenas allocated
from the finalized layouts. No backend can extend or mutate the arena map
through this view.

### Completion and physical accounting

`BackendPoll` is nonblocking: `Pending` or `Complete { metric }`. A metric value
is allowed only for a finalized metric task. `PhysicalPollStatus` records the
corresponding physical result as pending, complete, or failed.

Every adapter reports ordered `PhysicalCall` values into a fixed
`PhysicalCallBatch`. The ABI-wide bound is
`MAX_PHYSICAL_CALLS_PER_OPERATION = 16`; an adapter may declare a smaller
`MAX_NON_POLL_PHYSICAL_CALLS`, but never a larger one. The batch stores an
inline array of optional calls and returns `PhysicalCallBatchOverflow` instead
of growing. The physical vocabulary includes resource binding, pending-token
preparation, arena allocation, admission chunks, local and external
submissions, metric submissions, polls, exit collection, worker quiescence,
arena release, and resource destruction.

`Backend::poll` has a particularly strict protocol: it appends exactly one
`PhysicalCall::Poll` for the pending task, with a status matching the returned
result, and appends no other physical record. `submit`, `poll`, and their error
formatting must not allocate, load code, create queues, or lazily initialize a
driver resource. Such work belongs in `bind_resources` and
`prepare_pending`, which complete before the first init admission.

The lifecycle hooks are:

```text
bind_resources(bundle)
  -> prepare_pending(task) for every Init/Loop/Exit task
  -> allocate_arena(layout) for every finalized device
  -> submit / submit_loop_iteration(work)
  -> poll(pending)
  -> collect_exit(work, destination) for external exit images
  -> release_arena(device, arena)
  -> destroy_resources(resource)
```

`supports_loop_repetition` defaults to false. A backend that cannot rearm its
pending tokens is rejected before any resources are bound when the finalized
loop is unbounded or contains more than one iteration. The default
`submit_loop_iteration` simply calls `submit`; repeatable adapters override it
to rearm a terminal token without allocating. `supports_same_queue_pipelining`
defaults to false. An adapter may opt in only when queue ordering and resource
liveness make a pending predecessor safe to overlap.

## Whole-bundle typestate lifecycle

The implementation in [`src/executor.rs`](../src/executor.rs) keeps the backend
and all live resources inside an owned `RunCore`. Public handles consume one
state and return the next, so an operation that is invalid in the current
phase is not exposed by the type being held.

```text
PreparedRun
   | initialize(DeviceImage...)
   v
InitializedRun
   | start_loop()
   v
RunningRun -- poll / wait / metric mailbox --> RunningRun
   | into_exited_loop() after terminal loop completion
   v
ExitedLoop
   | exit()
   v
ExitedRun -- backend, metrics, exit images, journal --> caller
```

Recoverable variants (`prepare_recoverable`, `initialize_recoverable`,
`start_loop_recoverable`, and `exit_recoverable`) return `RunFailure<B>` when
the backend must be preserved. `RunFailure` retains the run and bundle
identities, the primary `ExecutorError`, an optional first cleanup error, the
backend value, and an optional bounded journal. `into_parts` exposes those
owned components for a caller that must decide how to persist or retry outside
the executor.

### Prepare

`PreparedRun::prepare` first derives `JournalCapacity` from the exact bundle.
It rejects unsupported loop repetition, constructs the three `PreparedPhase`
values, creates a fixed `CompletionLedger`, and calls `Backend::bind_resources`.
Each phase is then realized with `Backend::prepare_pending` for every task,
sorted by schedule-window start and task identity. This is the point at which
all reusable completion tokens and backend-owned resources must exist. The
executor records `LogicalEvent::Prepared` only after all three pending pools
have been realized.

No arena has been allocated yet, and no user data has been admitted. The
prepared core owns the finalized bundle, active backend resource, empty arena
map, completion ledger, user-metric mailbox, bounded exit-image vector, journal,
and watchdog.

### Init and admission

`PreparedRun::initialize` validates the caller's `DeviceImage` collection
against `FinalizedBundle::init_images()` and the arena layouts before touching
the backend:

* one image per expected device is required;
* duplicate, missing, and unexpected devices are rejected;
* the supplied value identity and byte length must exactly match the manifest;
* every finalized arena device must have an init manifest;
* each checked calculation's fault flag is located inside its exact packed init
  image and is zeroed in the private validated copy before submission.

The executor then allocates every finalized arena and runs the Init phase with
the blocking poll loop. Init admission is the only whole-bundle ingress path:
the packed image is borrowed by `InitAdmissionWork` and the backend owns the
asynchronous copy until its token completes. Once all init tasks complete,
`LogicalEvent::Initialized` is recorded and the handle becomes
`InitializedRun`.

### Loop scheduler

`InitializedRun::start_loop` obtains finalized iteration zero, resets all loop
slot states to `Remaining`, and records `LoopStarted` and
`LoopIterationStarted`. A `RunningRun` exposes only bounded scheduler/poll
operations, latest-value metric consumption, capacities, current iteration, and
journal inspection.

One `poll_with_progress_or_stop` call performs one complete nonblocking pass:

1. Inactive tasks whose iteration domain does not contain the current index are
   marked complete once their dependencies are complete. This permits sparse
   periodic schedules without submitting phantom work.
2. Remaining active tasks are considered in fixed phase order. A task is
   runnable only when all dependencies are complete, or when the dependency is
   a pending task on the same queue and the backend explicitly supports same
   queue pipelining. Pending tasks with non-overlapping schedule windows block a
   new submission unless that same queue-pipelining contract applies.
3. Runnable tasks are lowered to one borrowed `BackendWork` value and submitted
   through `submit_loop_iteration` for loop tasks. The executor records either
   `InitAdmission` or `TaskSubmitted` as the logical event.
4. Every pending slot is polled once. A pending result leaves the slot active;
   a complete result runs `complete_slot`, validates metric payloads, collects
   external exit images when appropriate, marks the completion ledger, and
   records `TaskCompleted`.
5. If no slots remain or are pending, the phase is terminal. If slots remain
   with no progress and no pending work, the scheduler reports
   `SchedulerStalled`. Repeated passes with no submission or completion advance
   the phase watchdog and eventually report `WatchdogExpired`.

For a completed iteration the executor records `LoopIterationCompleted`, then
checks the caller's stop closure. A requested stop is accepted only at this
boundary, after all tasks in the current iteration are terminal. It records
`LoopStopAccepted` and `LoopCompleted` without starting another iteration. If
there is a next finalized iteration, the loop completion ledger and slot states
are reset and the next `LoopIterationStarted` is recorded. An unbounded loop
has no invented terminal iteration, so it can leave the loop only through this
safe stop boundary or failure.

`wait` uses the same pass and applies a small blocking backoff only when a pass
made no progress. The backoff starts at 50 microseconds and saturates at 2
milliseconds. It does not alter the asynchronous scheduler or allocate a
second execution path.

### Exit and teardown

`RunningRun::into_exited_loop` succeeds only when the current loop phase is
complete and no failure is recorded. `ExitedLoop::exit` then runs the immutable
Exit phase to completion. A device-to-device exit transfer is ordinary
`ExitTransfer` work. A device-to-external exit transfer additionally allocates
one exact host `Vec<u8>`, calls `Backend::collect_exit` after the pending token
is terminal, and stores an `ExitImage { task, source, bytes }`. Its capacity was
derived from the finalized exit task set before the run started.

After Exit completes, `teardown_resources` consumes arenas in ordered map order,
calls `release_arena` for each one, records `ArenaReleased`, consumes the active
resource exactly once, and calls `destroy_resources`. Only after all of that
does the executor record `Exited` and return `ExitedRun`. The returned handle
contains the backend, metric mailbox, external images, and complete journal;
`into_parts` transfers those values to the caller.

Failure teardown is ordered and best effort. `failed_core` attempts every
remaining arena release and then resource destruction. The first teardown
failure becomes the primary cleanup error and a later teardown failure becomes
the secondary `cleanup_error`; a failure never silently skips the remaining
arenas. Preparation failures before a `RunCore` exists return a `RunFailure`
with no journal, while failures after binding preserve the bounded journal.

## Lowering finalized tasks to backend work

`PreparedTask::new` is the single whole-bundle lowering point. It resolves
logical values and transfer endpoints once, copies dependency and schedule
metadata, and stores a compact `PreparedWork` variant:

| Finalized shape | Prepared work | Legal phase |
| --- | --- | --- |
| `Calculation` | Device, kernel template, artifact, resolved input/output locations, optional fault location, submission slots | Loop only |
| `Metric` | Purpose, metric identity, slot, resolved value, submission slots | Loop only |
| `External -> Device` transfer | Device destination, resolved destination, bytes, submission slots | Init only, `InitAdmission` |
| `Device -> Device` transfer | Resolved endpoints, bytes, route, lane claims, submission slots | Init or Loop, `InternalTransfer` |
| `Device -> Device` or `Device -> External` transfer | Resolved endpoints, bytes, route, lane claims, submission slots | Exit only, `ExitTransfer` |

The lowering code rejects calculation or metric tasks in Init or Exit, external
loop ingress or egress in the whole-bundle API, init transfers that are neither
admission nor device movement, and exit transfers that attempt to admit data.
These are `InvalidPhaseTask` failures, not fallback routing decisions.

Every loop task carries one finalized `IterationDomain`. `active_on` uses the
domain's zero-based arithmetic progression, so the graph remains singular and
the executor does not unroll or allocate per-iteration task records. The
`CompletionLedger` is indexed by task identity and is reset only for the Loop
phase between completed iterations. Init and Exit completion state is never
reused as loop state.

Checked calculations carry an int32 fault location. `validate_images` proves
that each fault flag is in the same device arena object and packed image as the
manifest, then clears its four-byte range before init. `complete_slot` accepts
only an int32 zero fault readback, records `FaultChecked`, and converts a
nonzero code into `DeviceFault`. A float fault readback or a missing metric is a
backend protocol failure.

## Metrics and journals

[`src/metrics.rs`](../src/metrics.rs) separates user telemetry from execution
control. `MetricMailbox::new` creates one slot only for a finalized metric slot
that has at least one `MetricPurpose::User` task. Fault-readback slots are not
published to callers.

Publishing verifies both slot identity and metric identity, assigns a monotonic
`u64` sequence, and replaces an older unconsumed sample in that slot. It never
waits. `try_take` removes the newest sample, so a slow consumer can lose
intermediate values but cannot delay scheduler polling or another metric slot.
`MetricValue` is exactly `F32(f32)` or `I32(i32)`.

[`RunJournal`](../src/executor.rs) has independent logical and physical
streams:

* Logical events describe the contract lifecycle: preparation, arena
  allocation, admission, task submission and completion, loop boundaries,
  metric publication, fault checks, worker quiescence, arena release, and
  exit.
* Physical calls describe what an adapter actually did: resource binding,
  token preparation, native submission, chunks, polls, exit copies, releases,
  and destruction. One logical admission may therefore contain several
  physical chunks.

`JournalCapacity::for_bundle` derives capacities from task, arena, loop-domain,
metric, and external-exit counts using checked arithmetic. The production
journal retains detailed loop events for the first iteration and compacts
repeated loop submissions, completions, iteration markers, user metric samples,
fault checks, and iteration completions after that point. It retains the first
pending poll marker for each task and keeps an exact task-indexed
`PendingPollCount`, so a long asynchronous delay does not multiply host
allocation by the watchdog limit. `JournalSummary` reports observed and
compacted counts for both streams. A debug or inspection caller may request
all loop detail with the internal `with_loop_detail` constructor, but that is
not the normal production allocation policy.

The journal rejects a physical call from an unknown task, an impossible pending
poll status, capacity arithmetic overflow, and any logical or retained
physical stream that exceeds its declared bound. These failures are surfaced
as `BackendProtocol`, `PreparationCapacityOverflow`, or
`JournalCapacityExceeded`; the executor does not grow the journal as a hidden
fallback.

## Error and progress semantics

[`src/error.rs`](../src/error.rs) keeps backend diagnostics allocation-free in
the live path. `BackendMessage` stores at most 96 UTF-8 bytes and marks
truncation; `ExecutorError::Backend` retains the operation and the bounded
message. The error families are:

| Family | Examples | Meaning |
| --- | --- | --- |
| Admission contract | `DuplicateAdmission`, `MissingAdmission`, `UnexpectedAdmission`, `AdmissionImageMismatch`, `AdmissionSizeMismatch` | Caller images do not exactly match the finalized init manifests. |
| Phase and protocol | `InvalidPhaseTask`, `BackendProtocol`, `LifecycleInvariant` | A finalized reference, backend result, or typestate transition violates the closed contract. |
| Device result | `DeviceFault` | A checked calculation reported a nonzero int32 fault code. |
| Scheduling progress | `SchedulerStalled`, `WatchdogExpired`, `InvalidWatchdog` | No legal task can run, or pending work has exceeded the caller's nonprogress bound. |
| Loop capability | `LoopRepetitionUnsupported` | The backend cannot rearm the finalized finite or unbounded loop. |
| Accounting | `MetricSequenceOverflow`, `PendingPollCountOverflow`, `PreparationCapacityOverflow`, `JournalCapacityExceeded` | A fixed counter or capacity cannot represent the observed run. |
| Exit storage | `ExitImageTooLarge`, `ExitImageAllocationFailed` | A finalized external output cannot be represented or allocated in host memory. |
| Backend operation | `Backend { operation, message }` | The adapter failed while binding, preparing, allocating, submitting, polling, collecting, releasing, or destroying. |

`Watchdog::new` rejects zero. `Watchdog::for_expected_duration` converts a
measured upper-bound operation duration and a nonzero safety multiplier into a
finite number of nonprogress polls using the executor's 50-microsecond to
2-millisecond backoff, saturating at `u32::MAX`.

## Worker projection and remote execution

The worker side in [`src/worker.rs`](../src/worker.rs) is not a second planner.
`WorkerProjection::derive` consumes the same finalized bundle plus a validated
topology and one `(MachineId, NodeId)` assignment. It checks topology identity,
worker-node ownership, nonempty local devices, arena layouts, reservations, and
init manifests. It then classifies every finalized task against the worker's
device set:

| Worker role | Projection rule |
| --- | --- |
| `InitAdmission` | External-to-local-device admission in Init. Exactly one must exist per local device and it must match the finalized image value and bytes. |
| `Local` | Calculation whose device is local, metric whose resolved value is local, or device-to-device transfer with both endpoints local. Init and Loop transfers are internal; Exit transfers use `ExitTransfer`. |
| `ExternalIngress` | Device-to-device task whose source is foreign and destination is local. The worker receives bytes from the master. |
| `ExternalEgress` | Device-to-device task whose source is local and destination is foreign, or local device-to-external Exit output. The worker sends bytes to the master or protocol owner. |

Tasks with neither endpoint local are omitted. Projected dependencies retain
only dependencies whose tasks are also projected; the remote protocol remains
responsible for ordering tasks owned by another worker. A cross-machine task
must already be a planner-expanded one-hop route. Its route must name an
existing measured link, point in the worker's direction, carry exactly one
matching `TransferLaneClaim::Link`, and carry no external lane claim. Queue and
completion slots must exist, belong to the same device, and belong to the
correct local endpoint.

The projection stores sorted devices, exact arena layouts, projected tasks,
required artifacts, and a SHA-256 identity over the bundle and topology
identities, assignment, devices, task phases, roles, and dependencies. The
digest makes an accidental mixture of a program, topology, and worker
assignment visible before a session starts. Projection errors are explicit:
invalid topology or identity, unknown machine/node, non-worker node, empty
worker, missing arena/reservation/image/admission, duplicate admission,
invalid task or resource, and capacity overflow.

### Worker backend hooks

`WorkerBackend: Backend` adds only the operations the whole-bundle ABI cannot
represent:

* `bind_worker_resources` binds the immutable projection;
* `prepare_external` realizes every external transfer token before Init;
* `begin_external_ingress` and `begin_external_egress` submit bounded byte
  buffers against pre-realized tokens;
* `poll_external` returns `Pending` or `Complete { bytes }`;
* `acknowledge_external_egress` releases a master's receive buffer contract;
* `quiesce_worker` drains all native queues before arena release.

The worker backend still uses `Backend::allocate_arena`, `submit`, `poll`,
`release_arena`, and `destroy_resources` for local tasks and resources. Every
operation reports a `PhysicalCall` through the same fixed batch.

### Worker session states

`WorkerExecutionSession` preallocates task slots, one image buffer per local
device, external pending tokens, arenas, and a bounded `RunJournal` before it
accepts data. Its state machine is:

```text
Prepared -> Init -> Loop -> Exit -> Finished
                       \-> Cancelling -> Failed
```

`prepare` rejects zero run IDs and bundle/projection identity mismatches,
binds worker resources, prepares every local and external pending token, builds
the image buffers, and records `Prepared`.

Init is explicit and sequential per device. `begin_init_image` checks the exact
byte count, allocates that device's arena, records `ArenaAllocated`, and enters
`Receiving`. `write_init_chunk` requires the next exact offset and never accepts
an overrun. `submit_init_image` requires all bytes, verifies the caller's SHA-256
digest, checks projected dependencies, submits the local admission token, and
records `InitAdmission`. `poll_init_image` completes the image only when the
backend reports a non-metric terminal result. When every image is complete, the
session records `Initialized` and `LoopStarted` and enters `Loop`.

`submit_task` and `poll_task` are valid only for a projected `Local` task in the
active phase. Dispatch rejects unknown or duplicate tasks, wrong phase or role,
incomplete projected dependencies, and overlapping active schedule windows.
Metric completions are checked against the finalized value dtype. User metrics
return a value, fault readbacks accept only int32 zero or raise `DeviceFault`,
and non-metric tasks must not return a metric.

Ingress and egress use separate calls and byte-count checks. An ingress becomes
`Complete` when its external token reports the exact byte count. An egress
becomes `AwaitingAck` first, and only `acknowledge_external_egress` changes it
to `Complete`; this keeps the caller's output buffer stable until the protocol
has acknowledged it. Each pending external or local poll increments a per-task
watchdog counter, not a global allocation.

`begin_exit` is legal only after every projected Loop task is complete. It
records `LoopCompleted` and enters Exit. Exit tasks must complete before an
arena can be released. `cancel` is legal from Loop or Exit, calls
`quiesce_worker`, marks active and awaiting-ack tasks `Quiesced`, and enters
Cancelling. `finish` requires Exit or Cancelling plus exact release of every
arena, destroys the resource, records `Exited`, and enters Finished.

`fatal_cleanup` is the terminal failure path. It quiesces first and deliberately
keeps all arenas if quiescence fails because native work may still reference
them. After successful quiescence it releases each remaining arena, zeroes
image buffers, destroys resources, marks the session Failed, and returns the
first observed cleanup error. `into_parts` recovers the backend and journal only
from Finished or Failed with no live resource.

`recipe-remote/src/executor_driver.rs` implements the wire-level
`WorkerDriver` by deriving and validating one `WorkerProjection`, then
forwarding every protocol operation to this session. It maps all executor and
backend errors into fixed-size `DriverFault` codes, restores backend ownership
after successful `finish`, and invokes `fatal_cleanup` on drop or after a
terminal protocol fault. It does not add a second execution engine.

## Downstream backend ownership

The executor crate owns scheduling and lifecycle. Concrete crates own only the
resources and native translation below that boundary.

### Host backend

[`host/src/backend.rs`](../../host/src/backend.rs) implements `Backend` for
RAM and disk arenas and asynchronous byte copies. `HostBackend` has a prepared
state machine (`Ready`, `Prepared`, `Warmed`, `Bound`), validates exact task
contracts, preallocates host job slots and staging, and declares
`MAX_NON_POLL_PHYSICAL_CALLS = 1`. It supports loop repetition by recycling a
terminal `HostPending` token before each iteration. Host calculation work is
rejected because calculation belongs to native GPU adapters.

[`host/src/runtime.rs`](../../host/src/runtime.rs) owns fixed job slots, worker
threads, staging buffers, and a condition variable. `PendingCopy` transitions
from prepared to queued, running, complete, or failed without allocating in
the executor's submit or poll calls. [`host/src/arena.rs`](../../host/src/arena.rs)
owns RAM or `create_new` disk backing and removes disk files when the last arena
reference closes. The host adapter records physical calls and maps runtime,
I/O, range, slot, poisoning, and worker-thread errors into its own `Error`.

### CUDA and HSA adapters

[`native-executor/src/cuda.rs`](../../native-executor/src/cuda.rs) and
[`native-executor/src/hsa.rs`](../../native-executor/src/hsa.rs) implement the
same `Backend` trait over already realized CUDA Driver and ROCr/HSA resources.
Both declare one non-poll physical record per operation and support loop
repetition. CUDA explicitly supports same-queue pipelining; HSA keeps the
default conservative completion-token ownership rule.

Their resource structs pre-realize the resources named by the immutable plan:
device arenas, submission queues, completion events or signals, loaded runtime
artifacts and entry points, invocation parameter blocks, metric readback
buffers, staging, egress storage, and scratch. `ExecutionPlan` validates
artifacts, ABI, devices, queue/completion slots, values, routes, and runtime
images before these resources are handed to `Backend`. `submit` validates the
closed `BackendWork` against those contracts and launches copies, kernels, or
four-byte readbacks. `poll` observes an event, stream, or signal and returns a
matching `BackendPoll`; no loader or allocator call is admitted in that path.
Native evidence is sampled immediately before destruction and proves image
loads, entry lookups, queues, completion objects, and persistent allocations.

### Composed local backend and staged bridge

[`native-executor/src/local.rs`](../../native-executor/src/local.rs) is the
production `LocalBackend` used by training and inference. It partitions every
finalized device and task to `Host`, `Cuda`, `Hsa`, or `Bridge`, owns each
partition's resources in one `LocalResources`, and wraps pending tokens in
`LocalPending`. `bind_resources`, `prepare_pending`, `allocate_arena`, submit,
poll, exit collection, release, and destroy dispatch directly to the one owner
selected by the finalized endpoint and device classes. A host task cannot be
submitted through CUDA, a calculation cannot be assigned to host or bridge,
and a bridge task cannot be used for an external exit.

`LocalBackend` reports one enclosing physical call and delegates the native
operation. It supports same-queue pipelining only for CUDA-owned tasks and
inherits loop-repetition support from its cross-backend bridge. Native evidence
from CUDA and HSA is joined when resources are destroyed.

[`native-executor/src/bridge.rs`](../../native-executor/src/bridge.rs) supplies
`StagedCrossBackend`, a one-hop bridge for device-to-device transfers whose
endpoints have different local owners. Candidate realization allocates each
CUDA/HSA staging buffer, stream/event or HSA signal, and a host staging worker
before finalization. A bridge pending token advances through source copy, host
middle job, destination copy, and completion. The bridge validates the exact
route, lane claims, values, byte count, and submission slots on every handoff,
rearms loop tokens without allocation, and destroys all legs and worker
threads during teardown.

The `LocalCandidateFactory` path performs a maximum-concurrency warm pass,
allocates and releases representative arenas, observes measured capacity, and
hands only a validated warmed resource set to `LocalBackend`. This is
pre-final preparation. The executor's loop sees only `BackendWork` and fixed
pending tokens.

### Remote adapter

`recipe-remote` keeps transport and wire ownership outside this crate. It
requires both peers to agree on bundle, topology, program, artifact, and
capacity identities, then drives the worker session through chunked init,
task dispatch, metric return, cross-machine data, cancellation, exact arena
release, and finish. The remote protocol's data slots and half-duplex tokens
are bounded state; they do not change the executor's local task semantics.

## Real production call path

The normal local training and inference path in
[`training/src/execute.rs`](../../training/src/execute.rs) is:

```text
public declaration
  -> compile graph and planner Draft
  -> Preparer::prepare_program(profile)
       -> measured realization, artifact validation, maximum-concurrency warm pass
       -> FinalizedBundle + warmed LocalPreparedSession
  -> LocalPreparedSession::into_backend(&bundle)
  -> PreparedRun::prepare(..., LocalBackend, Watchdog)
  -> initialize(DeviceImage...)
  -> start_loop()
  -> poll_with_progress[_or_stop]() until LoopStatus::Complete
  -> into_exited_loop()
  -> exit()
       -> collect external ExitImage values
       -> release arenas and destroy host/CUDA/HSA/bridge resources
  -> CompletedTrainingExecution or CompletedInferenceExecution
```

Inference enforces exactly one loop iteration, rejects loop external transfers
and user metrics, and measures only the checked loop interval. Training can use
finite or unbounded loop counts, requires a stop source for an unbounded run,
drains user metrics after each pass without backpressure, and accepts a stop
only after a complete iteration. Both paths map `RunFailure` into a report that
retains the bounded journal and cleanup error. Output decoding happens only
after `ExitedRun` has copied the finalized external images; the host never
recreates a model value from partial backend state.

## Invariants and forbidden transitions

The following invariants are enforced at the boundary where the relevant
state becomes observable:

| Invariant | Enforcement |
| --- | --- |
| All native work reachable from submit or poll was realized before Init | `Backend` documentation contract, `PreparedRun::prepare`, and each adapter's prepared resource pool. |
| Init has exactly one image per required device | `validate_images`, worker image manifests, and adapter admission contracts. |
| Every task is dispatched at most once per activation | `TaskSlot`/`WorkerTaskSlot` states and duplicate-dispatch errors. |
| Dependencies are complete before dispatch | Whole-bundle `CompletionLedger`; worker projected dependency checks. |
| Schedule windows do not conflict | Whole-bundle runnable predicate; worker active-window check. |
| Sparse loop tasks submit only in their domain | `PreparedTask::active_on`; inactive tasks become logically complete when dependencies permit. |
| A backend metric appears only on a metric task with the right dtype and slot | `complete_slot`, `MetricMailbox::publish`, and worker metric validation. |
| Checked calculation flags are cleared before Init and read back as int32 | `fault_reset_range` and `DeviceFault` handling. |
| External egress buffers remain stable until the backend and caller acknowledge completion | Whole-bundle `collect_exit` after terminal poll; worker `AwaitingAck` state. |
| Loop repetition never creates a new token or native resource | `supports_loop_repetition` gate and adapter-specific rearm hooks. |
| Arenas are released only after the phase that can reference them is terminal | ExitedLoop exit ordering; worker Exit/Cancelling checks and exact release tracking. |
| Native work is quiesced before worker arena release on cancellation or failure | `quiesce_worker` in `cancel` and `fatal_cleanup`. |
| Resource ownership is consumed exactly once | `ResourceState::Taken`, `Option<Resource>` on worker sessions, and backend state transitions to `Bound`. |

There is no public operation to submit new work after `RunningRun` has become
`ExitedLoop`, to admit external data into the whole-bundle loop, to release an
arena while a task can still reference it, or to recover a live resource from a
nonterminal worker session. Invalid calls fail at the nearest boundary rather
than being converted into no-ops.

## Failure map and recovery ownership

The executor keeps the primary failure visible and preserves enough ownership
for a caller to inspect or clean up:

1. Before backend binding, capacity, repetition, phase preparation, or watchdog
   validation failures return `RunFailure` with the original backend and no
   journal.
2. During binding or pending-token preparation, the backend remains owned by
   `RunFailure`; if a resource was acquired, `prepared_resource_failure` calls
   `destroy_resources` and records the first cleanup failure.
3. During Init, Loop, or Exit, `failed_core` drops phase token vectors, releases
   every arena it can, consumes and destroys the resource, and returns
   `RunFailure` with the journal. A backend error is wrapped with its operation
   and a bounded diagnostic message.
4. A journal-capacity, metric-sequence, pending-counter, or exit-image failure
   follows the same teardown path. The executor does not allocate a larger
   replacement journal or silently discard a required external image.
5. Worker preparation returns `WorkerPrepareFailure` with backend ownership and
   a possible cleanup error. Worker runtime failures remain in the session
   until `fatal_cleanup` has quiesced, released, and destroyed what it safely
   can; `into_parts` then recovers the backend and journal from the terminal
   state.

This design makes failure evidence a consequence of the real execution path:
logical events show which lifecycle transition was reached, physical calls show
which adapter operations actually occurred, pending-poll counters show waiting
without unbounded storage, and cleanup errors show whether teardown completed.

## Source-of-truth and validation

When changing this boundary, inspect the implementation files linked above and
the corresponding `recipe-core` schedule and plan contracts first. The
normative repository lifecycle is the immutable `init -> loop -> exit` model;
the executor should be generalized only when a reachable finalized task shape
requires it.

Structural validation for this crate and its direct adapters is:

```text
cargo check -p recipe-executor --all-targets
cargo check -p recipe-host --all-targets
cargo check -p recipe-native-executor --all-targets
cargo check -p recipe-remote --all-targets
```

These commands prove the module and trait graph compiles. Runtime correctness
requires the public training or inference entrypoint, a real finalized bundle,
the measured native preparation path, and the real CUDA, HSA, or host backend.
The acceptance runner independently checks completed logical lifecycle events,
one bind and destroy, no loop-time native realization, native evidence, metric
and output contracts, and complete teardown.
