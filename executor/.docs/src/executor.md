# `recipe_executor::executor`

```yaml
document: recipe_executor.executor
source: executor/src/executor.rs
kind: typestate-native-executor-contract
authority:
  - executor/src/executor.rs
  - executor/src/backend.rs
  - executor/src/error.rs
  - executor/src/metrics.rs
  - executor/src/lib.rs
  - core/src/plan.rs
  - core/src/schedule.rs
  - system-contract.md
```

This page describes the backend-neutral executor for one immutable
`recipe_core::FinalizedBundle`. The executor owns the run lifecycle, fixed
pending tokens, device arenas, dependency and schedule admission, bounded
polling, metric publication, external exit images, and teardown. It does not
discover hardware, compile kernels, load native modules, or mutate a finalized
plan. Those operations are complete before `PreparedRun` is created.

The public lifecycle is an owned typestate chain:

```text
PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun
       \             \              \             \
        RunFailure    RunFailure      RunFailure     RunFailure
```

Each arrow consumes the previous handle. A failed transition returns the
backend and bounded journal through `RunFailure` only when the recoverable
method is used. The non-recoverable convenience methods return only the
`ExecutorError` and therefore drop the recovered failure parts.

## 1. Contract vocabulary

### 1.1 Lifecycle handles and capabilities

| handle | state owned by the handle | legal transition | exposed live operations |
| --- | --- | --- | --- |
| `PreparedRun<B>` | bound backend resource, all phase pending tokens, no arenas, no admitted image | `initialize(images)` | journal inspection |
| `InitializedRun<B>` | arenas allocated and every `Init` task complete | `start_loop()` | journal inspection |
| `RunningRun<B>` | loop phase slots, exit slots, active iteration, completion ledger | `poll`, `wait`, `into_exited_loop`, or `fail` | bounded progress, metric mailbox, journal, capacities, current iteration |
| `ExitedLoop<B>` | completed loop and exit-phase slots, arenas still live | `exit()` | exit-phase progress through the blocking terminal, metrics, journal |
| `ExitedRun<B>` | backend value, metrics, copied exit images, final journal; no native resource | none | identities, journal, images, metric consumption, `into_parts` |
| `RunFailure<B>` | backend value and optional journal after best-effort cleanup | `into_parts()` | primary error, cleanup error, identities, optional journal |

`RunningRun` intentionally has no public submission, allocation, compilation,
or external-ingress method. All task slots and backend pending values are
realized before `init`, and the loop API can only progress the fixed schedule.
The state types make illegal phase transitions unrepresentable to callers.
The internal checks remain because finalized data and backend protocol results
are external boundaries.

### 1.2 Phases and work classes

The finalized bundle supplies three ordered phases: `Init`, `Loop`, and
`Exit`. `PreparedTask::new` is the executor's phase legality boundary
(`executor/src/executor.rs:1664-1810`). The accepted matrix is:

| phase | accepted finalized task | prepared backend work | execution meaning |
| --- | --- | --- | --- |
| `Init` | external to device transfer | `InitAdmission` | copy one packed host image into a device value |
| `Init` | device to device transfer | `Transfer { class: InternalTransfer }` | perform pre-loop device movement |
| `Loop` | calculation | `Calculation` | invoke one realized kernel in an exact loop iteration |
| `Loop` | device to device transfer | `Transfer { class: InternalTransfer }` | move a value during the loop |
| `Loop` | metric | `Metric` | read one finalized device value into a metric or fault result |
| `Exit` | device to device transfer | `Transfer { class: ExitTransfer }` | move a value toward an external result or another device |
| `Exit` | device to external transfer | `Transfer { class: ExitTransfer }` | copy a completed result into an `ExitImage` |

Calculations and metrics outside `Loop`, loop transfers that are not internal,
init transfers that are neither admission nor internal movement, and exit
transfers whose source is external are rejected with `InvalidPhaseTask`.
`TaskKind::Metric` remains a specialized readback transfer. It is not a third
model operation.

### 1.3 Fixed-resource rule

Preparation computes capacities from the finalized graph, binds the backend,
and calls `prepare_pending` once for every finalized task. Initialization then
allocates exactly one arena for each finalized arena layout. Loop submission and
polling borrow these objects and pass read-only `ArenaSet` views to the backend.
No `submit`, `submit_loop_iteration`, or `poll` operation may load code,
allocate, grow a collection, or lazily initialize driver state. The backend
contract puts that work in `bind_resources` and `prepare_pending`
(`executor/src/backend.rs:105-118`, `:302-322`).

The executor itself follows the same boundary:

```text
prepare: BTreeMap and Vec capacities, backend resource, pending tokens
init:    validated image map, arenas, init submissions and polls
loop:    fixed phase slots, fixed completion ledger, fixed journal storage
exit:    per-result host image allocation, collection, arena release, destroy
```

The per-result host buffer is allocated only in `Exit` by
`collect_exit_image`; it is never allocated by a live-loop operation.

## 2. Public records

### 2.1 Watchdog and blocking backoff

`Watchdog` stores one nonzero `max_nonprogress_polls`
(`executor/src/executor.rs:62-115`). `Watchdog::new(0)` returns
`InvalidWatchdog`. `for_expected_duration(expected, safety_multiplier)` models
the blocking backoff: it multiplies the measured duration by the nonzero
safety factor, adds one maximum-delay interval, counts 50 microsecond,
exponentially increasing sleeps capped at 2 milliseconds, adds one poll, and
saturates at `u32::MAX` with a minimum of one poll. The watchdog counts passes
that make no submission, inactive completion, or terminal completion. It is a
bound on executor scheduler passes, not a backend per-task timeout.

`BlockingPollBackoff` is private. `new` starts at 50 microseconds, `reset`
returns to that delay, and `wait` sleeps once then doubles the delay up to 2
milliseconds (`executor/src/executor.rs:29-52`). `RunningRun::wait` and
`run_phase_blocking` reset after progress and sleep only after a nonprogress
pass.

### 2.2 External images

`DeviceImage { device, image, bytes }` is the caller-owned packed init image
for one required device (`executor/src/executor.rs:119-136`). Its constructor
does no validation. `initialize_recoverable` validates the complete iterator
against the bundle before allocating or submitting any arena admission.

`ExitImage { task, source, bytes }` is a host-owned copy produced by a
finalized `Exit` transfer whose destination is `External`
(`executor/src/executor.rs:145-150`). `ExitedRun::exit_images` exposes the
ordered copies retained by the executor, and `into_parts` transfers ownership
of the vector to the caller.

### 2.3 Logical events

`LogicalEvent` is the ordered, contract-level stream. It is deliberately
separate from backend-reported `PhysicalCall` records, because one logical
admission or submission can produce multiple driver-level actions
(`executor/src/executor.rs:153-232`). The direct executor emits the following
events:

| event | emitted by | meaning |
| --- | --- | --- |
| `Prepared` | successful preparation | backend resource and every phase pending token exist |
| `ArenaAllocated` | each successful arena allocation | one finalized device arena is live |
| `InitAdmission` | init task submission | one packed image admission was submitted |
| `TaskSubmitted` | non-admission task submission | one calculation, internal transfer, metric, or exit transfer was submitted |
| `TaskCompleted` | inactive completion or backend terminal completion | the task is complete in the completion ledger |
| `Initialized` | successful init phase | all init tasks and arenas are ready |
| `LoopStarted` | loop entry | iteration processing is now legal |
| `LoopIterationStarted` | loop entry and each next iteration | active iteration identity for backend work and metrics |
| `MetricPublished` | user metric completion | latest-value-wins mailbox publication occurred |
| `FaultChecked` | zero fault readback completion | checked device calculation reported no fault |
| `LoopIterationCompleted` | terminal completion of an iteration | all active and inactive loop slots reached completion |
| `LoopStopAccepted` | stop callback accepted at a safe boundary | no next loop iteration will begin |
| `LoopCompleted` | final or accepted-stop loop completion | exit is now legal |
| `LoopFailed` | the first failed loop poll pass | a loop error was recorded before it is returned |
| `ArenaReleased` | each successful release during teardown | one device arena is no longer owned by the run |
| `Exited` | successful final teardown | backend resources are destroyed and the run is fully exited |

`ExternalTransferSubmitted`, `ExternalTransferCompleted`, and
`WorkerQuiesced` are shared journal vocabulary used by the worker executor in
`worker.rs`; no code path in `executor.rs` emits them directly. They remain in
the enum so a `RunJournal` can represent both local and worker lifecycles.

### 2.4 Journal capacity and summaries

`JournalCapacity { logical_events, physical_calls }` is the declared fixed
capacity for the two ordered vectors (`executor/src/executor.rs:234-351`).
`JournalCapacity::new` accepts explicit values. `for_bundle` retains one loop
iteration's ordered detail. The crate-private `for_bundle_retaining` helper
accepts a larger retained iteration count for callers inside the executor
crate that need a larger bounded detail window.

The capacity calculation first checks that
`B::MAX_NON_POLL_PHYSICAL_CALLS` is nonzero and no larger than the ABI-wide
`MAX_PHYSICAL_CALLS_PER_OPERATION`. It then derives:

```text
loop_task_count       = count(tasks with phase Loop)
non_loop_task_count   = total tasks - loop_task_count
active_loop_tasks     = sum of domain activations before retained_end
task_executions       = non_loop_task_count + active_loop_tasks
metric_emissions      = sum of metric-domain activations before retained_end
exit_image_count      = count(Exit transfers whose destination is External)
arena_count           = count(finalized arena layouts)
```

All sums and products use checked arithmetic. The logical bound is:

```text
RUN_LIFECYCLE_LOGICAL_EVENTS        // Prepared, Initialized, LoopStarted,
                                    // optional LoopStopAccepted, LoopCompleted,
                                    // Exited
+ 2 * arena_count                    // ArenaAllocated and ArenaReleased
+ 2 * non_loop_task_count            // submit and complete
+ loop_task_count * retained_count   // completion slots for retained iterations
+ active_loop_tasks                  // loop submissions
+ metric_emissions                   // metric publication or fault check
+ 2 * retained_count                 // iteration started and completed
```

The physical bound is:

```text
fixed_backend_operations = 2                         // bind and destroy
                          + 2 * arena_count           // allocate and release
                          + total_task_count          // prepare_pending
                          + task_executions           // submissions
                          + exit_image_count          // collect_exit
physical_calls = fixed_backend_operations * B::MAX_NON_POLL_PHYSICAL_CALLS
               + task_executions                     // terminal poll records
               + total_task_count                     // first pending marker per task
```

Pending polls are compacted into one retained first marker per task plus an
exact `u128` counter. The bound therefore does not grow with a slow backend's
number of pending observations. Capacity arithmetic overflow returns
`PreparationCapacityOverflow`.

`PendingPollCount { task, count }` is the exact compact count for one
finalized task. `JournalSummary` records saturating `u128` totals for observed
and compacted logical and physical records. It is accounting, not a second
event stream (`executor/src/executor.rs:353-370`).

### 2.5 RunJournal storage and API

`RunJournal` owns:

```text
logical_events: Vec<LogicalEvent>       // retained ordered logical records
physical_calls: Vec<PhysicalCall>       // retained nonpending and retained-detail terminal calls
pending_polls: Vec<PendingPollCount>    // one sorted entry per finalized task
loop_tasks: Vec<TaskId>                 // sorted loop-task membership index
current_loop_iteration: Option<LoopIteration>
retain_all_loop_iterations: bool
summary: JournalSummary
declared: JournalCapacity
```

`with_capacity` is the production compact mode. `with_loop_detail` can retain
all loop iterations for a bounded diagnostic or worker use case. Both allocate
the ordered vectors at the declared capacities and prebuild sorted task
indexes (`executor/src/executor.rs:372-442`).

`record_logical` increments `logical_events_observed`, updates the current
iteration on `LoopIterationStarted`, and classifies repeated loop events with
`repeated_loop_event`. In compact mode, repeated events after iteration zero
increment `logical_events_compacted` and are not appended. Non-repeated events
are always retained until `logical_events` reaches its declared capacity,
then `JournalCapacityExceeded { stream: Logical, .. }` is returned.

The compacted logical set is exactly:

```text
TaskSubmitted { phase: Loop, .. }
TaskCompleted { phase: Loop, .. }
LoopIterationStarted { .. }
MetricPublished { .. }
FaultChecked { .. }
LoopIterationCompleted { .. }
```

`record_physical` validates every pending poll task against the sorted fixed
table. It retains all non-loop calls and all first-iteration loop calls. In
compact mode, repeated loop calls, including their terminal polls, are
compacted; only the first pending poll marker for each task is retained while
every pending observation is counted exactly. It counts several pending
records in one fixed batch without losing any of them. An unknown task
produces `BackendProtocol`; a counter overflow produces
`PendingPollCountOverflow`; a vector overrun produces
`JournalCapacityExceeded { stream: Physical, .. }`
(`executor/src/executor.rs:452-575`).

Public inspection methods return the retained logical events, retained physical
calls, fixed pending-count table, one task's pending count, summary, declared
capacity, and the actual allocated vector capacities
(`executor/src/executor.rs:577-614`). `physical_calls()` explicitly excludes
most pending records. Consumers must pair it with `pending_poll_counts()` for
exact polling totals.

`RuntimeCapacities` reports the live loop allocation shape: phase slot capacity,
completion-ledger capacity, logical and physical journal capacities, and
pending-poll entry capacity (`executor/src/executor.rs:616-622`).

## 3. Owned run core and failure ownership

### 3.1 RunCore and ResourceState

The private `RunCore<B>` is shared by every typestate handle
(`executor/src/executor.rs:624-637`):

```text
run_id, bundle, backend
resource: ResourceState<B::Resource>
arenas: BTreeMap<DeviceId, B::Arena>
completed: CompletionLedger
metrics: MetricMailbox
exit_images: Vec<ExitImage>
exit_image_capacity: usize
journal: RunJournal
watchdog: Watchdog
```

`ResourceState` is either `Active(resource)` or `Taken`
(`executor/src/executor.rs:638-675`). `active` and `active_mut` return a
`LifecycleInvariant` error after consumption. `consume` atomically replaces
`Active` with `Taken` and returns the resource, or reports the same invariant
error on a second attempt. Only teardown consumes the backend resource.

### 3.2 RunFailure and RunFailureParts

`RunFailure` stores run and bundle identity, the primary `ExecutorError`, an
optional first cleanup error, the backend, and an optional journal
(`executor/src/executor.rs:699-749`). The cleanup error is strictly secondary:
teardown still attempts every remaining arena release and resource destruction.
`journal()` is `None` only for an unstarted failure that happened before a
journal could be created, such as capacity overflow or unsupported loop
repetition. `into_parts` exposes all owned components as
`RunFailureParts`, preserving the backend for caller-controlled recovery.

Failure construction has three paths:

| helper | when used | cleanup behavior |
| --- | --- | --- |
| `unstarted_failure` | capacity or loop-support rejection before journal/resource binding | returns backend directly, no journal, no cleanup |
| `prepared_resource_failure` | phase realization or `Prepared` journal failure after `bind_resources` | calls `destroy_resources` once and records its error as cleanup error |
| `failed_core` | any failure after a `RunCore` exists | calls `teardown_resources`, then returns `run_failure` |

`backend_value` always records the physical batch before examining the backend
result. If the backend result is an error, it is formatted into the fixed
96-byte `BackendMessage` and returned as `ExecutorError::Backend`. If recording
the physical batch fails, that journal error wins and the backend error is not
returned separately (`executor/src/executor.rs:2693-2716`).

## 4. Preparation

### 4.1 Entry points and loop support

`PreparedRun::prepare` and `prepare_with_journal_capacity` are convenience
wrappers that map a recoverable failure to its primary error. The recoverable
forms preserve `RunFailure<B>` (`executor/src/executor.rs:795-839`).

`prepare_recoverable` derives `JournalCapacity::for_bundle` and returns an
unstarted failure if capacity arithmetic fails. The capacity-explicit form
then rejects a finalized unbounded loop or a finite loop with more than one
iteration unless `backend.supports_loop_repetition()` is true. This rejection
occurs before journal construction or backend binding and returns
`LoopRepetitionUnsupported` with the original backend.

### 4.2 Phase conversion and backend binding

`PreparedPhases::new` filters the bundle into `Init`, `Loop`, and `Exit`
`PreparedPhase`s. Each task becomes a `PreparedTask`; tasks are sorted by
`(window.start, task.id)`. `PreparedPhases::fault_resets` collects loop
calculations with fault flags, sorts by resolved value, and deduplicates by
value (`executor/src/executor.rs:1966-2018`).

Preparation then performs this exact sequence:

| order | operation | state/evidence |
| --- | --- | --- |
| 1 | create `RunJournal` and `CompletionLedger` | all task indexes fixed |
| 2 | compute external exit slot count and fault reset list | exit image vector capacity fixed |
| 3 | `backend.bind_resources(bundle, calls)` | one backend resource and `BindResources` physical record |
| 4 | `realize_phase(init)` | one `PendingRequest` and pending token per init task |
| 5 | `realize_phase(loop)` | one pending token per loop task |
| 6 | `realize_phase(exit)` | one pending token per exit task |
| 7 | create `MetricMailbox` from user metric slots | one capacity-one slot per planned user metric |
| 8 | record `LogicalEvent::Prepared` | `PreparedRun` becomes constructible |

`realize_phase` calls `backend.prepare_pending` for each task, records its
physical batch with `BackendOperation::PreparePending`, and creates a
`TaskSlot` in `Remaining` state. Failure while realizing a phase drops already
realized phase states and destroys the bound resource through
`prepared_resource_failure` (`executor/src/executor.rs:2085-2122`).

### 4.3 PreparedTask and backend work

`PreparedTask` captures immutable task metadata: id, schedule window,
dependencies, optional loop activation domain, and one `PreparedWork`
(`executor/src/executor.rs:1621-1663`). `PreparedWork` carries all resolved
values and backend ABI data:

| variant | fields that cross to the backend |
| --- | --- |
| `InitAdmission` | device, destination value location, exact bytes, submission slots |
| `Calculation` | device, kernel template, artifact, submission slots, resolved inputs and outputs, optional fault flag |
| `Transfer` | `WorkClass`, resolved endpoints, bytes, route, lane claims, submission slots |
| `Metric` | purpose, metric id, metric slot, resolved value, submission slots |

`backend_work(run, iteration, images)` turns the prepared data into one
borrowed `BackendWork`:

```text
InitAdmission -> requires images map and exact (device, value, bytes) key
Calculation   -> requires active LoopIteration and carries run and iteration
Transfer      -> maps InternalTransfer or ExitTransfer class directly
Metric        -> requires active LoopIteration and carries purpose and slot
```

Missing images, a loop work item without an active iteration, or inconsistent
resolved data return `MissingAdmission`, `LifecycleInvariant`, or
`BackendProtocol` respectively. `active_on(iteration)` is true only when the
task's finalized `IterationDomain` contains the zero-based iteration index.
`external_exit()` identifies only a device-to-external `ExitTransfer` and
`fault_reset()` identifies a calculation with a fault flag
(`executor/src/executor.rs:1811-1964`).

### 4.4 Resolved locations

`resolve_value` requires every referenced `ValueId` to have a finalized
`ResolvedValueLocation`. `resolve_values` applies it to a calculation's input
or output list. `resolve_transfer_endpoints` requires a finalized pair of
resolved transfer endpoints. A missing location is a `BackendProtocol` error
with the task id, not a runtime lookup or fallback
(`executor/src/executor.rs:2667-2691`).

## 5. Initialization

### 5.1 Admission validation

`validate_images(bundle, images, fault_resets)` builds a device-keyed map and
requires an exact one-to-one match with `bundle.init_images()` and arena
layouts (`executor/src/executor.rs:2124-2220`):

1. Duplicate supplied devices return `DuplicateAdmission`.
2. Duplicate finalized manifests for one device return `DuplicateAdmission`.
3. Every arena layout device must have a finalized init manifest.
4. Every expected device must have one supplied image.
5. The supplied `ValueId` must equal the manifest image.
6. The supplied byte length must equal the manifest `ByteCount` exactly.
7. Any supplied device left after expected entries are consumed returns
   `UnexpectedAdmission`.

The validated map is keyed by `InitImageKey { device, image, bytes }`. Before
submission, every unique loop calculation fault flag is reset in place inside
its device image. `fault_reset_range` requires the flag and image to share
device, arena object, and exact image byte count, then checks nonunderflowing
relative offsets, host `usize` conversion, and the four-byte `i32` range. Any
failure is `BackendProtocol`. This makes a reused admission image start with a
zero fault word without changing the finalized value layout.

### 5.2 Arena allocation and init execution

`PreparedRun::initialize_recoverable` first validates all images. It then walks
the finalized arena layouts in bundle order and calls
`backend.allocate_arena(resource, layout, calls)`. Each successful call is
recorded as `PhysicalCall::AllocateArena` by the backend and as
`LogicalEvent::ArenaAllocated { device, bytes }` by the executor. A duplicate
device would violate finalized layout uniqueness.

The executor calls `run_phase_blocking` for the realized init phase with the
validated image map. Init `submit_slot` uses `backend.submit`, and an admission
submission records `LogicalEvent::InitAdmission`; all other init work records
`TaskSubmitted`. `poll_phase_once` continues until every init slot is complete,
recording terminal `TaskCompleted` events. Successful completion records
`Initialized` and moves ownership to `InitializedRun`. Any allocation, submit,
poll, completion, journal, or image error tears down all arenas and the bound
resource through `failed_core`.

## 6. Loop scheduler

### 6.1 Loop entry

`InitializedRun::start_loop_recoverable` obtains `loop_iterations().iteration(0)`.
If no iteration exists, it returns a `LifecycleInvariant` failure. It resets
the loop phase to `Remaining` slots, sets its active `LoopIteration`, records
`LoopStarted` and `LoopIterationStarted`, and moves the core and both phase
states into `RunningRun` (`executor/src/executor.rs:1072-1124`).

`PhaseState::begin_loop_iteration` asserts that the previous iteration has no
noncomplete slots, resets every slot status to `Remaining`, clears the
nonprogress counter, marks an empty phase complete, and stores the new
iteration (`executor/src/executor.rs:1579-1620`).

### 6.2 One nonblocking pass

`RunningRun::poll_with_progress_or_stop` first re-returns a stored failure if a
previous pass failed. It then delegates to `poll_phase_once(core, phase, None)`
(`executor/src/executor.rs:1169-1250`). A pass reports `(LoopStatus,
made_progress)` through the public wrappers.

`poll_phase_once` follows this order:

```text
1. If state.complete, return complete with no new progress.
2. Mark inactive loop tasks complete when their dependencies are complete.
3. Scan Remaining slots and submit every runnable task.
4. Poll every Pending slot once.
5. Classify complete, pending, stalled, watchdog, or phase-complete state.
```

An inactive task is ready when its domain does not contain the active iteration
and every dependency is already complete. It is marked complete without a
backend submission and receives `TaskCompleted`.

For each remaining slot, the runnable predicate requires:

```text
slot.status == Remaining
and (no loop iteration or task.active_on(iteration))
and every dependency is complete
    or is Pending on a backend-supported same-queue pipeline
and every currently Pending slot has an overlapping schedule window
    or is on the same supported pipeline queue
```

The pipeline queue is populated only for loop work when
`backend.supports_same_queue_pipelining(resource, task)` is true and the task
has submission slots. A backend that does not opt in requires dependency
completion and schedule compatibility before another submission can use the
queue. The executor does not guess at a queue or reorder the finalized task
graph; the sorted task order and predicate above determine admission.

### 6.3 Submission and polling

`submit_slot` builds borrowed backend work, obtains the active resource and
read-only arena set, then calls `submit_loop_iteration` for loop work with the
exact iteration. For `Init` and `Exit` it calls `submit`. The default backend
implementation forwards `submit_loop_iteration` to `submit`; repeatable
backends may rearm a terminal pending token using its own activation state
(`executor/src/executor.rs:2430-2494`, `executor/src/backend.rs:345-365`).

The physical batch is recorded before the result is interpreted. Successful
submission records `InitAdmission` for admission work or `TaskSubmitted` with
the phase and `WorkClass` for all other work, then changes the slot to
`Pending`.

For every pending slot, `poll_phase_once` calls `backend.poll` once and records
the matching `PhysicalCall::Poll`. `BackendPoll::Pending` leaves the slot
pending. `BackendPoll::Complete { metric }` delegates to `complete_slot`, then
marks the slot `Complete` and reports progress. The backend contract requires
exactly one physical poll record whose status matches the returned result
(`executor/src/backend.rs:164-176`, `:369-384`).

### 6.4 Completion, metrics, faults, and exit copies

`complete_slot` first checks whether the task is an external exit. If so, it
calls `collect_exit_image` before processing a possible metric result. It then
enforces the metric contract (`executor/src/executor.rs:2496-2594`):

| prepared work and result | action |
| --- | --- |
| user metric with a `MetricValue` | publish the sample for the active iteration and record `MetricPublished` |
| fault readback with `I32(0)` | record `FaultChecked` and continue |
| fault readback with nonzero `I32(code)` | return `DeviceFault` |
| fault readback with `F32` | return `BackendProtocol` |
| metric work without a value | return `BackendProtocol` |
| nonmetric work with any value | return `BackendProtocol` |
| nonmetric work without a value | continue |

After a valid result, the completion ledger is marked and `TaskCompleted` is
recorded. `MetricMailbox::publish` validates the slot and metric identity,
increments a checked sequence, and replaces any older unconsumed sample in the
same slot. A replacement is reported in `MetricPublished`; different slots
remain independent. `try_take_metric` is nonblocking and returns the latest
sample or `None`.

`collect_exit_image` converts the finalized byte count to host `usize`,
allocates exactly that many bytes, rebuilds the exit transfer work, and calls
`backend.collect_exit` with the destination buffer
(`executor/src/executor.rs:2596-2665`). It reports `ExitImageTooLarge` when the
count cannot fit the host address space, `ExitImageAllocationFailed` when the
buffer cannot be reserved, and `BackendProtocol` if the prepared work is not
an exit transfer or if the precomputed result-slot capacity is exceeded.

### 6.5 Phase completion, stop, and repetition

After submissions and polls, `poll_phase_once` examines slot states:

| state | result |
| --- | --- |
| no `Remaining`, no `Pending` | mark phase complete and return `complete: true` |
| `Remaining`, no `Pending`, and no progress | return `SchedulerStalled` |
| otherwise with progress | reset `nonprogress_polls` and return pending |
| otherwise without progress | increment `nonprogress_polls`; at the watchdog bound return `WatchdogExpired` |

The scheduler-stalled check precedes the watchdog because no pending task can
ever make progress in that state. A pass that only marks inactive work or
submits/completes one task counts as progress.

When a loop phase becomes complete for the active iteration,
`RunningRun::poll_with_progress_or_stop` records `LoopIterationCompleted` and
evaluates the supplied stop callback. A false callback requests the next
finalized iteration, if one exists, resets the loop completion ledger and
phase slots, records `LoopIterationStarted`, and returns `Pending`. A true
callback accepts the stop only at this completed boundary, records
`LoopStopAccepted`, then records `LoopCompleted`. If there is no next iteration,
the loop completes without needing the callback. The final completion sets
`completion_recorded`, so later polls keep returning `Complete` without
duplicating terminal journal events.

The backend must advertise `supports_loop_repetition` before preparation for a
finite count greater than one or an unbounded loop. Each repeated activation
uses the same fixed task slot and pending token; no task graph or allocation is
created per iteration.

### 6.6 RunningRun API

| method | behavior |
| --- | --- |
| `poll()` | one bounded nonblocking pass, returns `Pending` or `Complete` |
| `poll_with_progress()` | same pass plus whether submission or completion progress occurred |
| `poll_with_progress_or_stop(f)` | same pass, evaluates `f` only after an iteration reaches terminal completion |
| `wait()` | repeats passes with bounded exponential backoff until loop completion |
| `try_take_metric(slot)` | removes the latest sample in one metric slot without waiting |
| `metric_mailbox()` | read-only mailbox view |
| `journal()` | read-only bounded journal view |
| `capacities()` | actual capacities of loop slots, completion ledger, journals, and pending table |
| `current_iteration()` | active finalized iteration; panics only if the internal running invariant is broken |
| `into_exited_loop()` | succeeds only when the loop phase is complete and no stored failure exists |
| `fail(error)` | consumes the running handle, drops phase tokens, and performs ordered teardown |

`LoopStatus` has exactly `Pending` and `Complete`. `Complete` means the loop
has reached a safe terminal boundary, not that arenas have already been
released. The caller must still consume `RunningRun` into `ExitedLoop` and run
the exit terminal before observing `ExitedRun`.

## 7. Public method matrix

The following matrix lists every public operation implemented in this module.
Methods returning `Result<T>` have a recoverable twin where shown; the twin
preserves backend ownership and the optional journal in `RunFailure<B>`.

| type | public operation | purpose and result |
| --- | --- | --- |
| `Watchdog` | `new(max_nonprogress_polls)` | construct a nonzero watchdog or return `InvalidWatchdog` |
| `Watchdog` | `max_nonprogress_polls()` | inspect the fixed scheduler bound |
| `Watchdog` | `for_expected_duration(expected, safety_multiplier)` | derive a finite bound from measured duration and backoff constants |
| `DeviceImage` | `new(device, image, bytes)` | package one caller-owned packed admission image without validating it |
| `JournalCapacity` | `new(logical_events, physical_calls)` | provide explicit journal bounds |
| `JournalCapacity` | `for_bundle::<B>(bundle)` | derive checked compact-mode bounds from finalized tasks and backend ABI |
| `RunJournal` | `logical_events()`, `physical_calls()`, `pending_poll_counts()` | inspect retained ordered evidence and exact compact poll counts |
| `RunJournal` | `pending_poll_count(task)` | inspect one task's exact pending count |
| `RunJournal` | `summary()`, `declared_capacity()`, `allocated_capacity()` | inspect accounting and fixed allocation evidence |
| `RunFailure<B>` | `run_id()`, `bundle_identity()`, `error()`, `cleanup_error()`, `journal()` | inspect failure identity, primary/secondary errors, and optional evidence |
| `RunFailure<B>` | `into_parts()` | recover backend, journal, and all failure fields |
| `PreparedRun<B>` | `prepare`, `prepare_recoverable` | bind resources, realize every pending token, and construct the prepared state |
| `PreparedRun<B>` | `prepare_with_journal_capacity`, `prepare_with_journal_capacity_recoverable` | same preparation with caller-supplied checked bounds |
| `PreparedRun<B>` | `initialize(images)`, `initialize_recoverable(images)` | validate images, allocate arenas, and complete `Init` |
| `PreparedRun<B>` | `journal()` | inspect preparation evidence before initialization |
| `InitializedRun<B>` | `start_loop()`, `start_loop_recoverable()` | enter iteration zero and construct `RunningRun` |
| `InitializedRun<B>` | `journal()` | inspect init-complete evidence |
| `RunningRun<B>` | `poll`, `poll_with_progress`, `poll_with_progress_or_stop` | perform bounded scheduler passes and advance or stop the loop |
| `RunningRun<B>` | `wait()` | block with bounded backoff until loop completion |
| `RunningRun<B>` | `try_take_metric`, `metric_mailbox()` | consume or inspect latest-value metric slots |
| `RunningRun<B>` | `journal()`, `capacities()`, `current_iteration()` | inspect bounded evidence, allocation shape, and active iteration |
| `RunningRun<B>` | `into_exited_loop()`, `fail(error)` | enter legal exit state or force ordered failure teardown |
| `ExitedLoop<B>` | `exit()`, `exit_recoverable()` | complete exit transfers, copy external results, release arenas, and destroy resources |
| `ExitedLoop<B>` | `try_take_metric`, `metric_mailbox()`, `journal()` | inspect final pre-exit metrics and evidence |
| `ExitedRun<B>` | `run_id()`, `bundle_identity()`, `journal()`, `exit_images()` | inspect fully exited identities, evidence, and outputs |
| `ExitedRun<B>` | `try_take_metric(slot)` | consume a final latest-value metric |
| `ExitedRun<B>` | `into_parts()` | recover backend, mailbox, exit images, and journal |

## 8. Exit and result ownership

### 8.1 ExitedLoop

`ExitedLoop` is constructible only from a complete, nonfailed
`RunningRun::into_exited_loop`. It still owns arenas and all exit pending
tokens. Its `exit_recoverable` method runs the exit phase to completion with
`run_phase_blocking(core, exit_phase, None)`. Exit submissions use
`backend.submit`, and terminal external transfers call `collect_exit_image`.

If the exit phase fails, its phase tokens are dropped and `failed_core` releases
all arenas and destroys resources. If the phase completes, the phase tokens
are dropped and `teardown_resources` runs. A teardown error becomes the primary
failure, with a later teardown error retained as `cleanup_error`. Only after
successful teardown does the executor record `Exited` and construct
`ExitedRun` (`executor/src/executor.rs:1323-1382`).

The non-recoverable `exit` maps a `RunFailure` back to its primary error. The
recoverable form preserves backend, journal, and cleanup diagnostics.

### 8.2 ExitedRun

`ExitedRun` contains only caller-recoverable output state:

```text
run_id: RunId
bundle: BundleIdentity
backend: B
metrics: MetricMailbox
exit_images: Vec<ExitImage>
journal: RunJournal
```

`run_id`, `bundle_identity`, `journal`, and `exit_images` are read-only. A
caller may consume final metrics without waiting and call `into_parts()` to
recover the backend, mailbox, exit images, and journal in one move
(`executor/src/executor.rs:1384-1424`). No arena or backend resource remains
live after this state.

## 9. Teardown and failure paths

`teardown_resources` takes the arena map, iterates devices in `BTreeMap` order,
and attempts every `release_arena` call. Each successful release records a
physical batch and `ArenaReleased`; a backend, journal, or resource-state
failure is passed to `record_teardown_error`. After all arenas are attempted,
`resource.consume()` is called exactly once and `destroy_resources` is invoked
with the owned resource (`executor/src/executor.rs:1489-1554`).

The first teardown error becomes the returned primary teardown error. The next
teardown error becomes `cleanup_error`; later errors are not retained,
but no remaining release or destruction attempt is skipped. A `Taken` resource
is itself a lifecycle error. The direct executor does not insert a quiesce or
retry step: the completed exit path is already terminal, and failure exposes
the real backend or lifecycle error.

The failure ownership matrix is:

| failure point | backend state | journal state | cleanup |
| --- | --- | --- | --- |
| capacity or unsupported repetition before journal | original backend | `None` | none needed |
| phase conversion before bind | original backend | `Some` journal | none, no resource bound |
| bind failure | original backend | `Some` journal with physical bind calls | none, no resource returned |
| phase realization or `Prepared` event after bind | bound resource | `Some` journal | destroy resource once |
| initialization or loop failure | arenas and resource in `RunCore` | `Some` journal | release all arenas, destroy resource |
| exit-phase failure | partial exit state, arenas, resource | `Some` journal | same ordered teardown |
| teardown failure | some resources may have failed to release | `Some` journal | all remaining operations still attempted |
| `Exited` journal failure after teardown | backend already destroyed | `Some` journal | no native cleanup remains |

## 10. Internal state machines

### 10.1 TaskSlot and PhaseState

`SlotStatus` is the private three-state task machine:

```text
Remaining -> Pending -> Complete
     \----------------> Complete   (inactive loop task)
```

`TaskSlot<P>` couples one immutable `PreparedTask` with one backend pending
token and status. `PhaseState<P>` stores the phase, fixed slot vector,
nonprogress count, complete bit, and optional active loop iteration
(`executor/src/executor.rs:1556-1620`). No slot can be created while a run is
running. `begin_loop_iteration` is the only reset operation and requires all
previous slots to be complete in debug builds.

### 10.2 CompletionLedger

`CompletionLedger` preallocates one sorted `CompletionEntry` for every
finalized task (`executor/src/executor.rs:2026-2083`). `contains` is the
dependency query, `mark` validates the task id and marks it complete,
`completed_count` supports debug output and capacity reporting, and
`reset_phase(RunPhase::Loop)` clears only loop entries between repeated
iterations. Init and exit completions are never reset.

### 10.3 Fault reset and activation arithmetic

`iteration_domain_activations_before(domain, retained_end)` counts a domain's
half-open arithmetic progression up to the retained bound with checked span,
stride, multiplication, and host conversion. It is used only for journal
capacity derivation, not scheduling. `checked_capacity_sum` and
`checked_capacity_mul` centralize overflow errors
(`executor/src/executor.rs:2718-2762`).

## 11. Backend call trace

The executor invokes the sealed `Backend` surface in this order. Every call
receives a fixed `PhysicalCallBatch`, and `backend_value` records that batch
before converting the result:

| lifecycle point | backend method | physical call family | logical event or result |
| --- | --- | --- | --- |
| preparation | `bind_resources` | `BindResources` and backend-specific setup | `Prepared` after all later preparation succeeds |
| preparation | `prepare_pending` per task | `PreparePending` | no logical event yet |
| initialization | `allocate_arena` per layout | `AllocateArena` | `ArenaAllocated` |
| init, loop, exit submit | `submit` | class-specific submit records | `InitAdmission` or `TaskSubmitted` |
| loop submit | `submit_loop_iteration` | class-specific submit records | `TaskSubmitted` |
| every pending pass | `poll` | exactly one `Poll` record | `TaskCompleted`, metric/fault event, or pending |
| exit completion | `collect_exit` | `CollectExit` | `ExitImage` appended |
| teardown | `release_arena` per live device | `ReleaseArena` | `ArenaReleased` |
| teardown | `destroy_resources` | `DestroyResources` | `Exited` only after success |

`BackendWork` is closed over `InitAdmission`, `Calculation`,
`InternalTransfer`, `Metric`, and `ExitTransfer`. The executor never passes a
compiler, loader, discovery, allocator, topology, or transport operation as
model work. A backend error becomes `ExecutorError::Backend` with its
operation tag and bounded message. A protocol mismatch, missing finalized
location, impossible phase, metric violation, or lifecycle violation becomes a
specific `ExecutorError` before the backend can conceal it.

## 12. Error taxonomy at the executor boundary

| error family | variants | source condition |
| --- | --- | --- |
| backend result | `Backend` | backend method returned its error after physical calls were journaled |
| admission | `DuplicateAdmission`, `MissingAdmission`, `UnexpectedAdmission`, `AdmissionImageMismatch`, `AdmissionSizeMismatch` | supplied images do not exactly match finalized manifests |
| phase/protocol | `InvalidPhaseTask`, `BackendProtocol` | finalized task or backend result violates the closed executor contract |
| device execution | `DeviceFault` | fault readback returned a nonzero integer code |
| lifecycle | `LifecycleInvariant` | consumed resource, missing iteration, invalid phase state, or impossible transition |
| scheduler | `SchedulerStalled`, `WatchdogExpired` | no runnable work, or too many complete-without-progress polls |
| repetition | `InvalidWatchdog`, `LoopRepetitionUnsupported` | invalid watchdog or backend lacking repeated-token support |
| accounting | `MetricSequenceOverflow`, `PendingPollCountOverflow`, `PreparationCapacityOverflow`, `JournalCapacityExceeded` | checked sequence, count, capacity, or journal bound failed |
| output storage | `ExitImageTooLarge`, `ExitImageAllocationFailed` | external result cannot be represented or allocated on host |

The error type is non-exhaustive. Callers should preserve the primary error and
inspect `RunFailure::cleanup_error` separately when using recoverable methods.

## 13. End-to-end execution recipe

The complete real call path is:

```text
bundle + backend + watchdog
  -> PreparedRun::prepare_recoverable
       -> JournalCapacity::for_bundle
       -> PreparedPhases::new / PreparedTask::new
       -> Backend::bind_resources
       -> Backend::prepare_pending for every task
       -> MetricMailbox and Prepared event
  -> PreparedRun::initialize_recoverable(device_images)
       -> validate_images and fault-flag zeroing
       -> Backend::allocate_arena for every layout
       -> run_phase_blocking(Init)
            -> submit_slot / Backend::submit
            -> poll_phase_once / Backend::poll
       -> Initialized event
  -> InitializedRun::start_loop_recoverable
       -> LoopStarted and iteration-zero events
  -> RunningRun::poll or wait
       -> dependency/window/activation admission
       -> submit_loop_iteration and poll
       -> metrics, fault checks, completion ledger
       -> repeat or safe stop at iteration boundary
  -> RunningRun::into_exited_loop
  -> ExitedLoop::exit_recoverable
       -> run_phase_blocking(Exit)
       -> collect_exit_image for each external egress
       -> release every arena
       -> destroy_resources
       -> Exited event
  -> ExitedRun::into_parts
```

At any recoverable error edge, phase tokens are dropped, the resource is
destroyed if it was bound, all live arenas are attempted in order, and the
backend plus bounded journal are returned in `RunFailureParts`. This is the
only ownership-preserving failure path; there is no hidden retry, fallback
backend, alternate schedule, or substitute state.
