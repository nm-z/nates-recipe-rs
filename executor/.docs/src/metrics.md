<!--
Intent: document the complete metric and checked-device-readback contract that
is implemented by executor/src/metrics.rs and the callers that surround it.
This is an implementation contract, not a second metric API. The finalized
bundle, its task graph, and its resource manifest remain authoritative.
-->

# Executor metrics and readbacks

## Contract summary

TaskKind::Metric is a scheduled, asynchronous four-byte device readback. It
is not a third calculation kind. The task reads a value already produced or
owned by a loop calculation and copies that value to pre-realized host-visible
staging. The same task shape serves two purposes:

~~~text
MetricPurpose::User           -> publish newest-value-wins telemetry
MetricPurpose::FaultReadback  -> validate a checked calculation's int32 flag
~~~

The execution path is:

~~~text
MetricEmission
  -> planner MetricTask + MetricSlot
  -> Draft validation and Finalize value location
  -> PreparedWork::Metric
  -> BackendWork::Metric
  -> backend four-byte asynchronous copy
  -> BackendPoll::Complete { metric: Some(MetricValue) }
  -> executor completion handler
       User: publish + LogicalEvent::MetricPublished
       FaultReadback: zero -> FaultChecked, nonzero -> DeviceFault
~~~

Only MetricPurpose::User tasks populate MetricMailbox. Fault readbacks are
internal control checks and are never exposed as user samples.

The relevant authority is distributed as follows:

| Layer | Authoritative responsibility |
| --- | --- |
| program/src/lib.rs | User metric declaration, scalar shape, producer and iteration-domain contract. |
| core/src/schedule.rs | MetricPurpose, MetricTask, MetricSlot, and the closed task vocabulary. |
| core/src/plan.rs | Slot, value, device, fault-cohort, dependency, and iteration-domain validation. |
| planner/src/planner.rs | Creation of user metric tasks and one readback task per checked fault cohort. |
| executor/src/metrics.rs | Bounded user mailbox and sample sequence/replacement semantics. |
| executor/src/executor.rs | Lifecycle, scheduling, backend completion validation, publication, and fault failure. |
| executor/src/backend.rs | Backend-neutral metric work and BackendPoll result contract. |
| host, native-executor | Physical four-byte copy and little-endian scalar decoding. |
| executor/src/worker.rs | Remote-worker projection and completion validation before forwarding. |
| training/src/execute.rs | User-slot draining, live observation, and final newest-sample retention. |

## Data model

### Scalar value

MetricValue is the executor's calculation-domain scalar:

~~~text
MetricValue = F32(f32) | I32(i32)
~~~

It derives Clone, Copy, Debug, and PartialEq. It deliberately does not derive
Eq, because an f32 sample may be a NaN. A backend decodes exactly four
little-endian bytes according to the finalized value's DType.

### Sample

MetricSample is one completed user emission:

~~~text
MetricSample {
    sequence: u64,
    iteration: LoopIteration,  // zero-based finalized loop coordinate
    task: TaskId,
    slot: MetricSlotId,
    metric: MetricId,
    value: MetricValue,
}
~~~

sequence is assigned by the mailbox, starts at zero, and is strictly
increasing for every accepted publication, including a publication that
replaces an unread sample. The sequence is global to one mailbox, not per
slot. iteration preserves the immutable schedule coordinate; user-facing
training converts it to a one-based epoch later.

### Finalized task and resource entries

The core schedule types are:

~~~text
MetricTask {
    purpose: MetricPurpose,
    metric: MetricId,
    value: ValueId,
    slot: MetricSlotId,
    submission: SubmissionSlots { queue, completion },
}

MetricSlot {
    id: MetricSlotId,
    metric: MetricId,
}
~~~

Every metric task is RunPhase::Loop. value resolves during Finalize to a
ResolvedValueLocation containing the device, dtype, byte count, arena object,
and arena offset. submission.queue and submission.completion both belong to
that same device. The finalized resource manifest contains the exact set of
metric slots and no runtime code may add one.

MetricPurpose::User means nonblocking, newest-value-wins telemetry.
MetricPurpose::FaultReadback means the mandatory readback of a preallocated
int32 fault flag shared by one checked-calculation cohort.

## Declaration and planning

### User declarations

program::MetricEmission has exactly these fields:

~~~text
MetricEmission {
    metric: MetricId,
    value: ValueId,
    domain: IterationDomain,
}
~~~

StaticCalculationProgram::new_with_metrics sorts declarations by metric ID
and validates them. A declaration is rejected when any of the following is
true:

* metric ID is zero or duplicated;
* value ID is zero, unknown, or not produced by a calculation;
* the value is not one scalar element stored in exactly four bytes;
* the value is marked as an external output;
* the emission domain is outside the program loop;
* the producer kernel's iteration domain does not cover every emission.

The OGDL representation contains metric, value, first, end_exclusive, and
stride. Parsing reconstructs the same MetricEmission and reruns the same
validation. Metric declaration identity is included in the planner's stable
graph digest.

### User metric task creation

planner::add_user_metrics performs the following deterministic steps for each
declaration:

1. Find the source kernel whose outputs contain the declared value.
2. Use that kernel's selected device as the metric device.
3. Select the producer-resident value copy. A transferred copy is rejected.
4. Allocate a task ID and use it as the MetricSlotId.
5. Add MetricSlot { id: slot, metric: emission.metric } to the resource manifest.
6. Create a loop TaskKind::Metric(MetricTask { purpose: User, ... }).
7. Depend on the value-copy producer and on every already-created fault readback.
8. Assign the declaration's exact iteration domain.

The task therefore reads the producer's local value after its calculation and
after all earlier fault cohorts have been checked. The metric readback itself
does not alter the value or create an external output.

### Checked fault readback creation

planner::add_fault_readbacks groups checked calculations by (device, domain,
fault_flag). For each cohort it:

1. Deduplicates and sorts the checked calculation task IDs.
2. Allocates one readback task, metric ID, and slot ID.
3. Adds an exclusive MetricSlot for that readback.
4. Creates MetricPurpose::FaultReadback reading the shared fault_flag.
5. Depends directly on every checked calculation in the cohort.
6. Assigns the same iteration domain as the cohort.
7. Records the readback in fault_readbacks for later user-task dependencies.

The planner uses the readback task ID for its metric and slot IDs because these
are internal identities. They still participate in the canonical bundle
digest, so changing purpose, value, slot, or submission resources changes the
finalized identity.

The static scheduler gives a metric task a one-nanosecond schedule duration and
claims only its assigned submission queue and completion slot. A metric has no
transfer route or lane claims: its physical copy is local to the resolved value
device and is ordered by task dependencies and the selected queue.

## Static validation before execution

DraftPlan::validate and FinalizedBundle::finalize_with_loop_schedule reject
invalid metric graphs before a backend is handed a run. The metric-specific
rules are:

~~~text
task.phase == RunPhase::Loop
metric.value exists
metric.slot exists
resource.metric(metric.slot).metric == metric.metric
queue(metric.submission.queue).device == value.device
completion(metric.submission.completion).device == value.device
~~~

For a FaultReadback, the resolved value must be exactly DType::I32 and four
bytes. The general value binding validator also checks alignment, arena
membership, lifetime coverage, producer references, and every task that reads
the value.

validate_fault_readbacks enforces the fail-closed ordering contract:

* every fault flag with one or more checked calculations has exactly one
  readback;
* the readback's slot is used exactly once;
* the readback directly depends on every checked calculation;
* every user metric and every exit task transitively depends on that readback;
* a readback naming an unused flag is invalid;
* the readback and every checked calculation share the same iteration domain.

The result is a closed dependency graph with no possible user publication or
exit egress before the corresponding fault has been checked.

## Preparation and fixed storage

PreparedRun::prepare_recoverable validates loop repetition support, computes a
fixed journal capacity from the finalized task graph, binds backend resources,
and realizes one pending token for every task in each phase. During
PreparedPhase::new, a metric task is converted to:

~~~text
PreparedWork::Metric {
    purpose,
    metric,
    slot,
    value: ResolvedValueLocation,
    submission,
}
~~~

The conversion rejects a metric in Init or Exit with
ExecutorError::InvalidPhaseTask, and rejects a loop metric whose value has no
finalized location with BackendProtocol. A loop metric always has a finalized
iteration domain.

MetricMailbox::new runs after all three phases have been prepared. It scans
the finalized resource slots and retains only slots that have at least one
TaskKind::Metric with purpose == User. Each retained slot stores its
finalized MetricId and starts empty. Fault-readback slots are intentionally
absent from this mailbox. The mailbox allocation is therefore fixed before
init, and its capacity equals the number of finalized user metric slots.

Journal capacity accounts for loop metric tasks in the retained-iteration
bound. Repeated loop detail is compacted after the first iteration by
RunJournal, but exact pending-poll counts remain in the fixed task-indexed
table. Metric mailbox state is independent of journal compaction.

## Lifecycle and ordering

The executor's owned typestate is:

~~~text
PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun
~~~

Metric behavior at each state is:

| State | Metric action |
| --- | --- |
| PreparedRun | Slots and backend pending tokens are realized; no metric can submit. |
| InitializedRun | Arenas and one init image per device are admitted; init has no metric tasks. |
| RunningRun | Loop metrics submit, poll, publish, or fault the run. try_take_metric is nonblocking. |
| ExitedLoop | No new loop task can submit; the final mailbox can still be drained. |
| ExitedRun | Exit transfers and ordered teardown are complete; mailbox ownership is returned or consumed. |

Initialization validates and zeroes every finalized fault flag in its device init
image before the first loop iteration. start_loop sets iteration zero and
records LoopStarted and LoopIterationStarted.

Each bounded scheduler pass (RunningRun::poll_with_progress_or_stop) does the
following in order:

1. Mark inactive loop tasks complete when their dependencies are complete.
2. Find runnable active tasks by dependency completion, schedule windows, and
   backend same-queue-pipelining rules.
3. Submit runnable metric tasks through Backend::submit_loop_iteration.
4. Record TaskSubmitted { class: Metric } and mark the task pending.
5. Poll every pending task once. A backend must append exactly one physical poll
   record and return Pending or terminal Complete.
6. For terminal metric completion, run complete_slot before marking the task
   complete and recording TaskCompleted.

The scheduler never waits for a mailbox consumer. wait only backs off when a
poll made no progress; it does not alter metric semantics. A graceful stop is
accepted only immediately after a completed iteration, so a completed metric
and its fault checks cannot be abandoned halfway through an iteration.

When another iteration begins, the completion ledger resets only loop tasks.
Repeatable backends rearm the same pre-realized pending tokens, while the
mailbox and sequence counter remain in place. Sparse domains may leave a metric
task inactive for an iteration; inactive tasks are marked complete without a
backend submission and produce no sample.

## Completion contract

BackendPoll is:

~~~text
BackendPoll::Pending
BackendPoll::Complete { metric: Option<MetricValue> }
~~~

complete_slot first collects an external exit image when the task is an exit
transfer, then validates the metric payload against the prepared work:

~~~text
PreparedWork::Metric + Some(value) -> purpose-specific handling
PreparedWork::Metric + None        -> BackendProtocol
non-metric work + Some(value)      -> BackendProtocol
~~~

For a user metric, complete_slot requires an active loop iteration, calls
MetricMailbox::publish, and records:

~~~text
LogicalEvent::MetricPublished {
    task,
    slot,
    replaced_unconsumed,
}
~~~

For a fault readback:

~~~text
I32(0)       -> LogicalEvent::FaultChecked { readback: task, value }
I32(nonzero) -> ExecutorError::DeviceFault { readback: task, value, code }
F32(_)       -> ExecutorError::BackendProtocol
~~~

A nonzero device fault fails the affected run. It is never converted into a
user sample, ignored, or treated as a successful completion. After successful
purpose handling, the completion ledger marks the task and the ordinary
TaskCompleted event is recorded.

If a logical journal write fails, the journal error is returned. The running
loop records LoopFailed when possible, preserving the primary failure or the
journal failure according to the existing executor error path.

## MetricMailbox structure and semantics

executor/src/metrics.rs owns the only local user-mailbox implementation:

~~~text
MetricMailbox {
    slots: BTreeMap<MetricSlotId, SlotState>,
    next_sequence: u64,
}

SlotState {
    metric: MetricId,
    sample: Option<MetricSample>,
}
~~~

### Construction

MetricMailbox::new(slots, tasks) iterates the finalized MetricSlot slice and
keeps a slot when any task has the same slot ID and is a user metric. It stores
the slot's declared metric ID and sample: None. The finalized validators
provide unique slot IDs and matching slot/task metric IDs; new therefore does
not duplicate that validation or create a second source of truth.

### Publishing

publish(iteration, task, slot, metric, value) -> Result<bool> has this exact
behavior:

1. Look up slot in the mailbox. Missing slots return
   ExecutorError::BackendProtocol with metric completion names an unplanned
   slot.
2. Compare the completion's metric with the slot's metric. A mismatch returns
   BackendProtocol with metric completion names the wrong metric for its slot.
3. Take the current next_sequence. Increment it with checked arithmetic. An
   exhausted counter returns ExecutorError::MetricSequenceOverflow and does
   not replace the existing sample.
4. Replace the slot's Option<MetricSample> with the new sample.
5. Return true when an unread sample was replaced, otherwise false.

Replacement is per slot. Publishing metric A never removes metric B. Publishing
never waits, allocates a channel message, or consults a consumer. The boolean
is only journal evidence; it does not change the value returned to a consumer.

### Taking and introspection

try_take(slot) removes and returns the current sample without waiting. An
unknown slot and an empty known slot both return None. pending_len() counts
occupied slots, capacity() counts user slots, and is_empty() is equivalent to
pending_len() == 0. Therefore:

~~~text
0 <= pending_len() <= capacity()
capacity() == number of finalized user metric slots
~~~

Taking a sample makes that slot empty; the next publication for that slot will
return replaced_unconsumed == false unless it is published again before the
next take.

The mailbox itself is moved through the typestate handles. The public methods
are:

~~~text
RunningRun::try_take_metric
RunningRun::metric_mailbox
ExitedLoop::try_take_metric
ExitedLoop::metric_mailbox
ExitedRun::try_take_metric
ExitedRun::into_parts -> (backend, MetricMailbox, exit_images, journal)
~~~

## Backend producers and physical readback

All backend implementations consume the same BackendWork::Metric contract:

~~~text
MetricWork {
    task,
    iteration,
    purpose,
    metric,
    slot,
    value: ResolvedValueLocation,
    submission,
}
~~~

Submission and polling are required to be allocation-free after preparation.
Every backend pre-realizes its completion token and four-byte metric staging
before init.

### Host backend

host::ExpectedWork::Metric records the metric, slot, resolved value, and
submission slots. Preparation reserves a four-byte staging arena on the value's
device. Submission checks the complete contract, verifies that staging exists,
and enqueues a four-byte copy from the resolved arena offset to staging offset
zero. On terminal poll, the host backend reads exactly four staging bytes and
decodes F32 or I32 with from_le_bytes. A host backend rejects calculation
work, so it cannot silently replace a GPU calculation with a CPU calculation.

### CUDA backend

CudaBackend::submit_metric requires value.bytes == 4 and
value.device == planned.device, resolves the arena range, looks up a
pre-realized pinned metric buffer keyed by task, and enqueues a four-byte
device-to-host copy on the assigned stream. Completion is observed by the
pre-realized event or stream-idle path. finish_action reads the four-byte
pinned buffer and decodes the finalized dtype. Missing buffers, non-four-byte
buffers, unowned completion tokens, and mismatched work are backend protocol
errors.

### HSA backend

HsaBackend::submit_metric applies the same device and four-byte checks, then
uses a pre-realized fine-grained host buffer and asynchronous HSA copy. It
claims the assigned completion signal before submission and releases it only
after terminal completion. finish_action copies the four bytes to host after
system-scope completion and decodes the finalized dtype. A missing or malformed
buffer, queue, completion signal, or resolved arena is an error.

### Local heterogeneous backend

native-executor::local assigns a metric task to the owner of its resolved
value device: Host, CUDA, or HSA. The bridge owner is valid only for staged
transfers and rejects BackendWork::Metric. The local backend records one
PhysicalCall::SubmitMetric { task, slot } at submission and one poll record
for each backend poll result. It forwards the scalar unchanged to the executor
completion handler.

### Bridge and transfer distinction

Cross-backend bridge operations return BackendPoll::Complete { metric: None }
because they are transfers, not metric readbacks. A metric value must remain
resident on the producer device and be read directly by the owner backend.
This keeps metric egress independent from ordinary transfer routes.

## Remote worker path

Remote execution preserves the metric purpose and resolved location in the
worker projection even though the wire program carries only task identities.

### Projection and worker session

WorkerProjection::classify_task resolves a metric value and includes the task
only when its device is local to the assigned worker. The resulting
PreparedWorkerWork::Metric stores purpose, metric, slot, value location, and
submission slots. Projection validation checks that the resource manifest has a
slot with the same metric ID.

WorkerExecutionSession::prepare realizes one local pending token for every
projected metric task. submit_task enforces run ID, active phase, local role,
idle state, complete projected dependencies, and nonconflicting schedule
windows. poll_task polls the backend and runs validate_metric_completion:

~~~text
User + Some(F32/I32 matching finalized dtype) -> Some(value)
FaultReadback + Some(I32(0))                  -> None
FaultReadback + Some(I32(nonzero))            -> DeviceFault
Metric + None or F32 for a fault readback     -> MetricContract
Non-metric + Some(value)                      -> MetricContract
~~~

The worker completes its task only after this validation. An
executor-backed worker driver converts the executor scalar into
RemoteMetricValue and maps worker metric/device-fault errors into bounded
DriverFault payloads.

### Wire protocol and master consumption

The remote codec encodes Message::Metric { task, value } on the independent
metrics lane. The value tag is 1 for f32 or 2 for int32, followed by four
bytes. Unknown tags are codec errors. The message does not carry a slot,
metric ID, or iteration; the provisioned task identity and bundle projection
provide that static context.

The worker sends a metric message before its TaskComplete message. The master
accepts a metric only for an active loop driver task, stores it in one of the
preallocated RemoteLimits.metric_slots() inbox entries, and emits
MasterRunEvent::MetricReady. A full inbox returns
RemoteError::Backpressured("master metric inbox"); it never mutates the worker
calculation. MasterRun::take_metric and MasterComplete::take_metric remove an
inbox sample without waiting. Remote metric storage is a transport mailbox,
separate from executor::MetricMailbox; callers that bridge remote execution
must map the static task identity back to the finalized slot.

## Consumers

### Training

training::execute::user_metric_slots filters the finalized bundle to
MetricPurpose::User tasks, sorts and deduplicates (slot, metric) pairs, and
initializes one FinalTrainingMetric per pair with sample: None.

After every running poll pass, training calls try_take_metric for each user
slot. It drains once more after the loop reaches terminal completion, again
after ExitedLoop, and finally from the mailbox returned by ExitedRun. This
ensures the newest completed metric is retained even if it was published during
the last pass.

TrainingMetricSample::from_executor preserves the executor sequence, raw
zero-based LoopIteration, task, slot, metric, and scalar, and derives the
one-based epoch as iteration.index() + 1. The live observer applies each
metric's cadence and uses SyncSender::try_send; a full or disconnected live
consumer increments dropped and never backpressures executor polling.
FinalTrainingMetric.sample is replaced only when the incoming sequence is
greater than the retained sequence. A metric whose domain never activates
remains None.

### Inference

Target-free inference rejects user metric declarations before native
preparation (reject_inference_user_metrics and the compiled-inference
boundary checks). Inference still permits internal fault readbacks required by
checked calculations, but it has no user metric mailbox output. Prediction
egress remains a finalized Exit transfer.

### Presentation

The facade's live presentation reads TrainingMetricSample.value, formats f32
NaN as N/A, and formats f32/i32 without adding executor-side work. It never
reads or mutates MetricMailbox directly.

## Bounds and failure surface

The following bounds are fixed by the finalized plan or a typed scalar:

| Bound or invariant | Enforced by | Failure or result |
| --- | --- | --- |
| Metric value is one four-byte scalar | Program/Core validation, backend submit | Program/validation error or backend protocol error. |
| Value dtype is f32 or i32 | DType, backend decode, worker completion validation | Metric contract/protocol error. |
| Metric task is loop-only | Core validation and PreparedTask::new | InvalidPhaseTask. |
| Slot exists and names the same metric | Core validation and MetricMailbox::publish | Validation error or BackendProtocol. |
| Queue/completion belong to value device | Core validation and backend contracts | Validation/backend protocol error. |
| One readback per checked flag cohort | validate_fault_readbacks | InvalidFaultReadback. |
| Readback precedes user metrics and exit | Draft dependency validation | InvalidFaultReadback. |
| Mailbox occupancy is at most one per user slot | Option<MetricSample> per map entry | Old unread value is replaced. |
| Sequence counter is u64 and checked | MetricMailbox::publish | MetricSequenceOverflow. |
| Unknown try_take slot is harmless | BTreeMap::get_mut | None. |
| Pending poll journal is fixed per task | RunJournal::pending_polls | Overflow/protocol error, never unbounded vector growth. |
| Physical report has fixed ABI capacity | PhysicalCallBatch | Backend physical-report overflow. |
| Loop repetition is backend capability | PreparedRun::prepare | LoopRepetitionUnsupported. |
| No-progress scheduler is bounded | Watchdog and phase scheduler | SchedulerStalled or WatchdogExpired. |
| Fault code is fail-closed | complete_slot and worker validation | DeviceFault or worker DeviceFault. |

Metric-specific executor errors and exact diagnostic text are:

~~~text
BackendProtocol(task, "metric completion names an unplanned slot")
BackendProtocol(task, "metric completion names the wrong metric for its slot")
BackendProtocol(task, "metric task completed without a metric value")
BackendProtocol(task, "non-metric task was tagged as metric work")
BackendProtocol(task, "non-metric task completed with a metric value")
BackendProtocol(task, "fault readback completed with a non-int32 value")
DeviceFault { readback, value, code }
MetricSequenceOverflow
~~~

Backend-specific protocol errors also cover missing pre-realized metric staging,
non-four-byte buffers, wrong queue or completion ownership, wrong resolved
device, and attempts to submit or poll a terminal token. They are surfaced as
ExecutorError::Backend { operation, message }, with the backend message held
in the fixed 96-byte BackendMessage buffer.

Worker-specific metric errors are:

~~~text
WorkerExecutionError::MetricContract { task, detail }
WorkerExecutionError::DeviceFault { readback, code }
~~~

Remote transport adds bounded metric-inbox backpressure and codec errors. None
of these paths silently drops a fault readback or substitutes a CPU calculation.

## Ordering proof in terms of observable events

For one active user metric task M, a valid run has this ordered subsequence:

~~~text
TaskSubmitted { phase: Loop, task: producer, class: Calculation }
TaskCompleted { phase: Loop, task: producer }
TaskSubmitted { phase: Loop, task: fault_readback, class: Metric }
FaultChecked { readback: fault_readback, value: flag }
TaskCompleted { phase: Loop, task: fault_readback }
TaskSubmitted { phase: Loop, task: M, class: Metric }
MetricPublished { task: M, slot: M.slot, replaced_unconsumed: bool }
TaskCompleted { phase: Loop, task: M }
~~~

The exact event stream may include other independent tasks and physical poll
records, but dependency and schedule validation forbid M from completing before
its producer and every relevant fault readback. A nonzero fault replaces the
FaultChecked and publication tail with DeviceFault and loop failure.

For repeated iterations, the same task IDs and slots recur with a new
LoopIteration and a larger mailbox sequence. After the first iteration the
ordered journal may compact repeated events, but the mailbox still represents
the latest completed sample per slot and the pending-poll counters preserve
exact poll totals.

## Source map

The implementation regions that define this contract are:

~~~text
executor/src/metrics.rs
  MetricValue, MetricSample, MetricMailbox::new/publish/try_take and bounds
executor/src/backend.rs
  MetricWork, BackendWork::Metric, BackendPoll, PhysicalCall::SubmitMetric
executor/src/executor.rs
  JournalCapacity metric counting
  PreparedWork::Metric and phase legality
  RunningRun metric APIs and lifecycle transitions
  poll_phase_once, submit_slot, complete_slot
executor/src/worker.rs
  projection classification, worker completion validation, worker errors
core/src/schedule.rs
  MetricPurpose, MetricTask, MetricSlot, TaskKind
core/src/plan.rs
  metric task validation, fault-readback dependency and domain checks
program/src/lib.rs
  MetricEmission declaration and scalar/domain validation
planner/src/planner.rs
  add_user_metrics, add_fault_readbacks, metric resource hashing
host/src/backend.rs
  host metric staging, copy, poll, and dtype decoding
native-executor/src/cuda.rs
  CUDA D2H metric submission and completion decoding
native-executor/src/hsa.rs
  HSA asynchronous metric copy and completion decoding
native-executor/src/local.rs
  owner routing and bridge rejection for metric work
remote/src/session.rs, remote/src/codec.rs
  remote metric lane, bounded inbox, wire scalar encoding
training/src/execute.rs
  user-slot discovery, draining, cadence observation, final retention
~~~

The source map is intentionally descriptive. The code and finalized bundle
remain the source of truth; this document does not authorize a second mailbox,
an alternate fault path, or a new metric purpose.
