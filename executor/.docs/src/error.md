# Executor errors

`executor/src/error.rs` is the backend-neutral execution failure boundary. It
contains the fixed-size diagnostic used for native backend failures, the
operation and journal-stream labels carried by those failures, the complete
`ExecutorError` vocabulary, and the `Result<T>` alias used by the executor and
metric mailbox. The module does not perform recovery or choose a substitute
task, image, metric, resource, or capacity. A returned error means that the
requested lifecycle transition did not complete with the authoritative state
described by the finalized bundle.

The public crate facade re-exports all of these names from `executor/src/lib.rs`
(`BACKEND_MESSAGE_CAPACITY`, `BackendMessage`, `BackendOperation`,
`ExecutorError`, `JournalStream`, and `Result`). The root Recipe facade exposes
the crate as `recipe::engine::executor` (`src/facade.rs:22`), so callers may
observe the same typed failures directly or through the training and inference
execution wrappers.

## Structure

```text
BACKEND_MESSAGE_CAPACITY = 96 bytes
BackendMessage
|-- bytes: [u8; 96]
|-- len: u16
`-- truncated: bool

BackendOperation
|-- BindResources
|-- PreparePending { task }
|-- AllocateArena { device }
|-- Submit { task }
|-- Poll { task }
|-- CollectExit { task }
|-- ReleaseArena { device }
`-- DestroyResources

JournalStream
|-- Logical
`-- Physical

ExecutorError (#[non_exhaustive], Copy + Clone + Debug + Eq)
|-- Backend { operation, message }
|-- DuplicateAdmission { device }
|-- MissingAdmission { device }
|-- UnexpectedAdmission { device }
|-- AdmissionImageMismatch { device, expected, actual }
|-- AdmissionSizeMismatch { device, expected, actual }
|-- InvalidPhaseTask { phase, task, detail }
|-- BackendProtocol { task, detail }
|-- DeviceFault { readback, value, code }
|-- LifecycleInvariant { detail }
|-- SchedulerStalled { phase }
|-- WatchdogExpired { phase, nonprogress_polls }
|-- InvalidWatchdog { max_nonprogress_polls }
|-- LoopRepetitionUnsupported { iterations }
|-- MetricSequenceOverflow
|-- PendingPollCountOverflow { task }
|-- PreparationCapacityOverflow
|-- JournalCapacityExceeded { stream, capacity }
|-- ExitImageTooLarge { task, bytes }
`-- ExitImageAllocationFailed { task, bytes }

Result<T> = std::result::Result<T, ExecutorError>
```

`ExecutorError` is `#[non_exhaustive]`, so downstream matches must retain a
wildcard. It implements `std::error::Error` directly and has no source method;
the only nested diagnostic is the already-rendered `BackendMessage`. The
`detail` fields are static strings selected by the rejecting executor branch,
not caller-owned or heap-allocated text. `DeviceId`, `TaskId`, `ValueId`,
`ByteCount`, `RunPhase`, and `LoopIterations` retain the exact typed identity
needed to inspect the failed boundary.

## Fixed backend diagnostics

`BackendMessage` (`executor/src/error.rs:5-89`) keeps a backend error without
allocating while the live loop is running. Its 96-byte array is accompanied by
the current UTF-8 byte length and a truncation bit. `Default` creates an empty,
untruncated message. `new` writes a caller-supplied string through the same
`fmt::Write` implementation, and `as_str` returns the valid UTF-8 prefix.

`write_str` encodes one Unicode scalar at a time. It checks the checked byte
addition and the 96-byte bound before copying a scalar, so a truncated message
never contains a partial UTF-8 sequence. Once the next scalar does not fit it
sets `truncated` and returns `fmt::Error`; the bytes already accepted remain
the retained prefix. The checked `u16` conversion is an internal invariant
because the configured capacity fits in `u16`; a failed conversion is an
`unreachable!` assertion, not an `ExecutorError` construction.

`Display` writes the retained prefix and appends the literal `...` when the
truncation bit is set. `as_str` never appends the marker, and
`was_truncated` exposes the bit for diagnostics and debug assertions. The
invalid-UTF-8 branch in `as_str` is unreachable because all writes pass through
the scalar encoder and the fixed prefix length.

The executor does not currently call `BackendMessage::new` from another
workspace module. `backend_value` (`executor/src/executor.rs:2693-2715`) uses
`BackendMessage::default` and `write!` so the backend's `Display` output is
captured directly. A formatting failure is expected only when the fixed bound
was reached, and the debug assertion checks `was_truncated`; the operation
still returns the retained diagnostic as `ExecutorError::Backend`.

## Operation and journal labels

`BackendOperation` is the closed operation label attached to a backend error:

| Label | Backend call represented by `backend_value` |
| --- | --- |
| `BindResources` | `Backend::bind_resources` during `PreparedRun::prepare_recoverable` (`executor/src/executor.rs:866-872`) |
| `PreparePending { task }` | `Backend::prepare_pending` while realizing each phase (`executor/src/executor.rs:2102-2111`) |
| `AllocateArena { device }` | `Backend::allocate_arena` while initializing each finalized arena (`executor/src/executor.rs:997-1010`) |
| `Submit { task }` | `Backend::submit` for init or exit, and `Backend::submit_loop_iteration` for loop work (`executor/src/executor.rs:2436-2474`) |
| `Poll { task }` | `Backend::poll` for one pending task (`executor/src/executor.rs:2362-2376`) |
| `CollectExit { task }` | `Backend::collect_exit` for one external exit image (`executor/src/executor.rs:2631-2648`) |
| `ReleaseArena { device }` | `Backend::release_arena` during ordered teardown (`executor/src/executor.rs:1492-1520`) |
| `DestroyResources` | `Backend::destroy_resources` during prepared-resource cleanup or final teardown (`executor/src/executor.rs:1449-1458`, `1523-1536`) |

The worker API has its own `WorkerBackendOperation` labels. A worker backend
failure is therefore `WorkerExecutionError::Backend`, not
`ExecutorError::Backend`; only worker journal failures wrap an
`ExecutorError` (`executor/src/worker.rs:2632-2646`).

`JournalStream` distinguishes the two bounded run-journal vectors. `Logical`
is the ordered `LogicalEvent` stream, including lifecycle, task, metric, and
fault events. `Physical` is the ordered `PhysicalCall` stream plus the fixed
per-task pending-poll counters. The two labels are used only by
`JournalCapacityExceeded`, at `RunJournal::record_logical`
(`executor/src/executor.rs:420-441`) and `RunJournal::record_physical`
(`executor/src/executor.rs:479-574`).

## Propagation and failure ownership

The executor's public typestate is

```text
PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun
```

The plain methods `PreparedRun::prepare`,
`PreparedRun::prepare_with_journal_capacity`, `PreparedRun::initialize`,
`InitializedRun::start_loop`, `ExitedLoop::exit`, and the running-loop polling
methods expose `Result<T>` and therefore only the primary `ExecutorError`.
Their recoverable counterparts return `RunFailure<B>` so the backend, bounded
journal, primary error, and first cleanup error remain owned by the caller.

`RunFailure::error` is the transition that first failed. `cleanup_error` is the
first teardown failure observed after that transition. `RunFailure::journal`
may be absent only for an unstarted failure, before a run journal exists.
`into_parts` exposes the backend and these fields for higher-level execution
wrappers. Teardown still attempts every remaining arena release and resource
destruction in lifecycle order. `record_teardown_error` keeps at most two
teardown failures: the first as the cleanup error and the second as a later
cleanup observation. The primary run error is never replaced during teardown.

There are three important precedence rules:

1. `backend_value` records the physical-call batch before it inspects the
   backend result. If recording fails, the journal error is returned and the
   backend's own error is not wrapped. If recording succeeds and the backend
   returns `Err`, the result becomes `ExecutorError::Backend` with the fixed
   operation label and message.
2. When `RunningRun::poll_with_progress_or_stop` receives a phase error, it
   tries to record `LogicalEvent::LoopFailed`. If that recording itself fails,
   the journal error is the reported and stored failure; otherwise the original
   phase error is retained (`executor/src/executor.rs:1238-1246`).
3. A failure after resources exist calls `failed_core` or
   `prepared_resource_failure`; teardown is attempted once, in order, without
   retrying a failed backend operation or inventing a replacement resource.

The worker session preserves the same journal boundary by mapping
`crate::Result<T>` to `WorkerExecutionError::Journal` at
`WorkerExecutionSession::journal_result` (`executor/src/worker.rs:2644-2646`)
and by mapping `RunJournal::record_physical` in `backend_result`
(`executor/src/worker.rs:2632-2642`). `WorkerExecutionError::source` exposes
the nested `ExecutorError`, so a caller can still inspect the exact executor
variant after the worker prefix is rendered.

At the training boundary, `TrainingExecutionError::Executor` is created by
`From<ExecutorError>` (`training/src/execute.rs:865-867`) and displays
`training execution failed: {error}` while retaining the executor as its
source (`training/src/execute.rs:743-858`). The controlled training path uses
the plain executor methods (`training/src/execute.rs:2233-2259`), so it
returns that primary error and performs executor teardown before the wrapper
sees it. The top-level facade maps the resulting text to its runtime error at
`src/training.rs:1334-1338`.

Inference uses the recoverable path. `inference_executor_failure` moves the
`RunFailure` into `InferenceExecutionError::Executor`, preserving the primary
`ExecutorError`, optional journal, run and bundle identities, and cleanup
error (`training/src/execute.rs:2972-2985`). Its display is
`inference execution failed: {source}` and its `source` is the nested
`ExecutorError` (`training/src/execute.rs:973-1178`). A post-exit output
validation error uses `PostExitValidation` and retains the completed journal,
but that later wrapper is not an executor failure.

## Variant reference

The following sections enumerate every current construction site, its direct
propagation path, the runtime consequence, and the exact `Display` text.

### `Backend { operation, message }`

`backend_value` is the sole constructor (`executor/src/executor.rs:2693-2715`).
It receives every `Backend` trait result listed in the operation table above.
It first commits that operation's `PhysicalCallBatch` to the journal. Only a
successful journal commit permits a backend `Err` to be formatted into a
`BackendMessage`; the backend error type's `Display` output is copied into the
fixed 96-byte prefix. A long diagnostic is intentionally retained as a
truncated prefix with `...`, not allocated or dropped.

The `Backend` variant propagates through the operation's `Result` into the
current lifecycle method. Preparation errors become an unstarted or prepared
`RunFailure`; initialize and loop errors teardown arenas and resources; exit
errors teardown the remaining resources. A backend error observed while
releasing an arena or destroying resources is recorded as a cleanup error when
there is already a primary failure. There is no retry and no alternate backend
operation.

Display is:

```text
backend {operation:?} failed: {message}
```

The operation uses its `Debug` spelling, for example
`backend Poll { task: ... } failed: ...`. `ExecutorError` has no source chain,
so the original backend error is retained only as this bounded text.

### `DuplicateAdmission { device }`

`validate_images` constructs it in two distinct maps
(`executor/src/executor.rs:2129-2148`):

* inserting caller-supplied `DeviceImage` values finds a second image for the
  same device; and
* inserting finalized `bundle.init_images()` manifests finds a duplicate
  manifest device.

The caller-input case is rejected before arena allocation or init submission.
The manifest case reports an invalid finalized admission contract. Both errors
return through `PreparedRun::initialize_recoverable`, which drops phase tokens,
tears down any resources already allocated, and returns the backend and journal
inside `RunFailure`.

Display is `device {device} received more than one init image`.

### `MissingAdmission { device }`

The variant protects every exact device admission lookup:

* `validate_images` requires each arena layout device to have an init manifest
  (`executor/src/executor.rs:2151-2157`);
* it requires one supplied `DeviceImage` for every expected manifest
  (`executor/src/executor.rs:2160-2164`);
* each fault-flag reset requires the corresponding expected manifest and
  validated image (`executor/src/executor.rs:2194-2211`); and
* `PreparedTask::backend_work` rejects an absent image map or absent
  `(device, value, bytes)` key when an init admission is submitted
  (`executor/src/executor.rs:1841-1856`).

The first three sites are in initialize-time image validation. The last site is
the execution-time guard that prevents an init task from reaching a backend
without its exact bytes. Any of these errors stops the phase and invokes the
same ordered cleanup as other initialized failures. No image is inferred from
another device, value, or byte count.

Display is `required device {device} has no init image`.

### `UnexpectedAdmission { device }`

After all expected devices have been removed from the supplied-image map,
`validate_images` reports the lowest remaining device key
(`executor/src/executor.rs:2189-2192`). An extra image is not ignored and is
not passed to a backend. Initialization fails before any init task is run for
that input set, with normal prepared-run cleanup.

Display is `init image targets unexpected device {device}`.

### `AdmissionImageMismatch { device, expected, actual }`

During the expected-device loop, `validate_images` compares the supplied
`DeviceImage.image` value with the finalized manifest value
(`executor/src/executor.rs:2164-2169`). The device identity and byte count can
be otherwise correct, but the bytes would populate the wrong logical value, so
the image is rejected before it is copied to the backend. The error propagates
through `initialize_recoverable` and teardown.

Display is `device {device} init image identifies value {actual}, expected
{expected}`.

### `AdmissionSizeMismatch { device, expected, actual }`

`validate_images` converts the supplied host vector length to `ByteCount` and
compares it with the exact finalized manifest size
(`executor/src/executor.rs:2171-2185`). A length conversion that cannot fit in
`u64` is `PreparationCapacityOverflow`, while a representable but unequal
length is this variant. The mismatch stops initialization before any image is
admitted.

Display is `device {device} init image is {actual}, expected exactly {expected}`.

### `InvalidPhaseTask { phase, task, detail }`

`PreparedTask::new` is the sole constructor, reached while
`PreparedPhases::new` classifies every finalized task
(`executor/src/executor.rs:1665-1801`). It rejects the five invalid shapes:

| Observed task shape | Detail |
| --- | --- |
| Calculation outside `Loop` | `calculations are legal only in the loop` |
| Metric outside `Loop` | `metrics are legal only in the loop` |
| Transfer in `Loop` with a non-device endpoint | `loop transfers must be internal` |
| Transfer in `Init` that is neither external-to-device admission nor device-to-device movement | `init transfer is neither admission nor internal movement` |
| Transfer in `Exit` with an external source | `exit cannot admit external data` |

The table has five branches because the transfer rule is split by phase. Valid
init admissions, internal init or loop transfers, and device-to-device or
device-to-external exit transfers are lowered to `PreparedWork`; these errors
are returned before backend binding and before a resource exists. The
recoverable preparation method returns a `RunFailure` with a journal allocated
for evidence and no cleanup error. No invalid task is silently dropped or
reclassified.

Display is `task {task} is invalid in {phase:?}: {detail}`.

### `BackendProtocol { task, detail }`

This is the executor-owned contract violation category. It is used whenever a
finalized bundle, a backend physical report, or a completed backend result
cannot be interpreted according to the closed executor protocol. The current
construction sites are:

* `RunJournal::record_physical` rejects a pending poll whose task is absent
  from the fixed finalized-task table (`executor/src/executor.rs:491-499`,
  `522-531`, and `548-557`).
* `PreparedTask::new` rejects a finalized init admission whose resolved
  destination is not a device (`executor/src/executor.rs:1712-1717`).
* `CompletionLedger::mark` rejects a completion for a task outside the fixed
  ledger (`executor/src/executor.rs:2059-2071`).
* Fault-image validation rejects a missing resolved init-image location, a
  flag outside the validated image, a flag and image that do not share one
  exact arena object, a flag preceding the image, an offset that does not fit
  the host address space, or an overflowing four-byte flag range
  (`executor/src/executor.rs:2199-2215`, `2221-2260`).
* `resolve_transfer_endpoints` and `resolve_value` reject finalized task
  references without resolved locations (`executor/src/executor.rs:2673-2691`).
* `complete_slot` rejects a fault readback carrying `f32`, a metric completion
  without a metric value, a non-metric task tagged as metric work, or any
  non-metric task carrying a metric value (`executor/src/executor.rs:2558-2585`).
* `collect_exit_image` rejects an external exit that cannot prepare exit
  transfer work or that exceeds its precomputed result-image slots
  (`executor/src/executor.rs:2625-2630`, `2657-2662`).
* `MetricMailbox::publish` rejects a user metric completion naming an
  unplanned slot or the wrong metric for its slot (`executor/src/metrics.rs:66-85`).

All of these sites return immediately through the current `Result` chain. A
protocol error during preparation prevents a run from being created. One
during init, loop, or exit stops the current phase and invokes ordered
teardown. A protocol error from physical journaling takes precedence over a
backend operation result because `backend_value` records the physical calls
first. The category is deliberately not a generic backend message: it means
the executor could not trust the task identity, resolved location, metric
shape, image range, completion table, or fixed result capacity.

Display is `backend protocol violation for task {task}: {detail}`.

### `DeviceFault { readback, value, code }`

`complete_slot` constructs this variant only for a `MetricPurpose::FaultReadback`
whose completed metric value is `MetricValue::I32(code)` with a nonzero code
(`executor/src/executor.rs:2543-2557`). A zero `i32` is the successful checked
path and records `LogicalEvent::FaultChecked`; an `f32` value is a
`BackendProtocol` error instead. The readback task and flag value identify the
checked calculation cohort, while `code` is the device-reported fault.

The run does not publish a successful completion for the faulting work. The
error propagates from the poll pass, marks the loop failed, and tears down the
backend through the normal failure path. It is an observed device result, not
a scheduler timeout and not a backend formatting error.

Display is `checked device work reported fault code {code} through value {value}
at readback task {readback}`.

### `LifecycleInvariant { detail }`

This variant marks an executor state that should be impossible after the
typestate and finalized-bundle contracts have been honored. Its direct sites
are:

* `ResourceState::active`, `active_mut`, and `consume` find a backend resource
  already marked `Taken` (`executor/src/executor.rs:643-674`).
* `InitializedRun::start_loop_recoverable` cannot obtain finalized iteration
  zero (`executor/src/executor.rs:1078-1085`).
* Running-loop completion has no active iteration, or the next iteration index
  would overflow `u64` (`executor/src/executor.rs:1183-1204`).
* `PreparedTask::new` finds a loop task without an iteration domain
  (`executor/src/executor.rs:1665-1673`).
* `PreparedTask::backend_work` tries to create calculation or metric work
  without an active iteration (`executor/src/executor.rs:1864-1875`,
  `1913-1931`).
* `submit_slot` is asked to submit loop work without an active iteration
  (`executor/src/executor.rs:2440-2457`).
* `complete_slot` receives a user metric completion without an active loop
  iteration (`executor/src/executor.rs:2525-2529`).

These are fail-closed invariant failures. They never advance a task, reuse a
consumed resource, or synthesize an iteration. A preparation occurrence
returns a failed prepared run; an active-run occurrence records loop failure
and performs ordered teardown. The same error can therefore expose a broken
state transition in preparation, live scheduling, or cleanup while retaining
the exact static detail that identified the violated invariant.

Display is `executor lifecycle invariant failed: {detail}`.

### `SchedulerStalled { phase }`

`poll_phase_once` constructs this variant when a phase still has `Remaining`
tasks, has no `Pending` tasks, and the pass made no progress
(`executor/src/executor.rs:2390-2408`). No task is runnable under its
dependency, schedule-window, iteration-domain, and same-queue-pipelining
rules, and there is no pending backend operation that could change that fact.
This is detected before the watchdog counter is used, because waiting cannot
make a phase with no pending work runnable.

The phase returns an immediate failure. Blocking `run_phase_blocking` stops,
and live loop polling stores the failure and attempts the `LoopFailed` logical
event. No sleep, retry, or task bypass is performed.

Display is `{phase:?} scheduler has no runnable or pending task`.

### `WatchdogExpired { phase, nonprogress_polls }`

After a pass with pending work but no progress, `poll_phase_once` increments
the phase's nonprogress counter. When it reaches the configured maximum it
returns this variant (`executor/src/executor.rs:2412-2422`). A pass that made
progress resets the counter. The watchdog therefore bounds waiting on pending
native work; it does not replace the scheduler-stall check above.

The error is returned from both blocking preparation/exit phases and the live
loop. The current phase is abandoned and the run follows the same failure and
teardown path as a backend error. The counter in the payload is the exact
threshold-reaching count, so callers can distinguish it from an invalid zero
configuration.

Display is `{phase:?} watchdog expired after {nonprogress_polls} nonprogress polls`.

### `InvalidWatchdog { max_nonprogress_polls }`

`Watchdog::new` constructs this variant only when the caller supplies zero
(`executor/src/executor.rs:66-80`). A zero maximum would make the first
nonprogress observation ambiguous and is rejected before a `Watchdog` value is
created. `Watchdog::for_expected_duration` always returns a nonzero, saturated
watchdog and therefore has no error path.

This is a configuration error at the public constructor boundary. No backend,
journal, phase, or run has been created, and no cleanup is needed.

Display is `watchdog maximum must be nonzero, got {max_nonprogress_polls}`.

### `LoopRepetitionUnsupported { iterations }`

`PreparedRun::prepare_with_journal_capacity_recoverable` constructs this error
when the finalized loop count is unbounded or finite with more than one
iteration and `backend.supports_loop_repetition()` is false
(`executor/src/executor.rs:830-847`). A one-shot backend is not allowed to
reuse a pending token for another activation. The complete `LoopIterations`
value is retained, and its display renders either the finite bound or the word
`unbounded`.

The check occurs before journal creation, phase realization, and backend
binding. The result is an unstarted `RunFailure` with no journal or cleanup
error, while the backend value is returned to the caller. It does not execute a
partial first iteration.

Display is `backend does not support the finalized loop count of {count}`, where
`count` is the finite count or `unbounded`.

### `MetricSequenceOverflow`

`MetricMailbox::publish` constructs this unit variant when its monotonic `u64`
sequence counter cannot be incremented (`executor/src/metrics.rs:87-103`). It
checks the counter before replacing the capacity-one sample in the metric
slot, so an overflow cannot silently wrap and make a newer sample appear older.

The error is reached from `complete_slot` for a user metric. The loop pass
fails, the slot is not published, and normal run teardown follows. The mailbox
has no recovery sequence or alternate counter.

Display is `metric sequence counter overflowed`.

### `PendingPollCountOverflow { task }`

`RunJournal::record_physical` constructs this error while counting pending
polls for a fixed task (`executor/src/executor.rs:500-515`, `522-536`, and
`548-562`). The inline batch count is checked before compaction, and the
stored per-task `u128` count is checked before being committed. The ordered
journal keeps only the first pending marker, but this counter preserves the
exact total, so arithmetic overflow cannot be hidden by compaction.

The error is returned by `backend_value` before the corresponding backend
result is inspected. The operation fails as an accounting failure, and the run
tears down any live resources. There is no unbounded counter or dropped poll
record.

Display is `pending-poll counter overflowed for task {task}`.

### `PreparationCapacityOverflow`

This variant means that the executor cannot prove a fixed capacity or host
index for data needed by the run. It is constructed at every checked arithmetic
site below:

* `JournalCapacity::for_bundle_retaining` rejects an invalid backend physical
  bound, a retained-iteration conversion, task-count subtraction, missing loop
  iteration domain, active-task sum, metric-emission sum, task-execution sum,
  arena-operation multiplication, or final logical/physical journal sum
  (`executor/src/executor.rs:258-335`).
* `RunJournal::record_physical` rejects the physical-vector length plus the
  newly retained call count overflowing `usize` (`executor/src/executor.rs:537-542`).
* `validate_images` maps a supplied host-image length that cannot fit in
  `u64` (`executor/src/executor.rs:2171-2178`).
* `checked_capacity_sum`, `iteration_domain_activations_before`, and
  `checked_capacity_mul` return it for checked sum, span, ceiling-division,
  `usize` conversion, and product failures (`executor/src/executor.rs:2718-2741`,
  `2759-2762`).

Capacity errors during `JournalCapacity::for_bundle` produce an unstarted
failure before journal allocation. Errors discovered while recording or
validating an active run stop that transition and teardown. The executor never
grows a bounded journal after this error, wraps an overflowing host index, or
reduces the declared work to fit.

Display is `executor preallocation capacity arithmetic overflowed`.

### `JournalCapacityExceeded { stream, capacity }`

`RunJournal::record_logical` constructs the `Logical` form when a non-compacted
logical event would exceed `declared.logical_events`
(`executor/src/executor.rs:425-440`). Repeated loop detail is compacted only
according to the journal's configured retention policy; events outside that
policy must fit the declared vector.

`RunJournal::record_physical` constructs the `Physical` form when retained
physical calls would exceed `declared.physical_calls`
(`executor/src/executor.rs:537-546`). Pending polls do not grow this vector
after their first marker, but the fixed counter is still updated and can report
its own overflow variant.

Every journal error propagates through the operation that was trying to record
the event. During a running-loop failure, inability to append `LoopFailed`
replaces the original phase error as described in the propagation rules. During
teardown, an exhausted journal can become the first or second cleanup error.
This fail-closed behavior preserves the bounded allocation contract and never
silently drops an event or expands a vector.

Display is `{stream:?} journal exhausted its declared capacity of {capacity}`.

### `ExitImageTooLarge { task, bytes }`

`collect_exit_image` constructs this variant when the finalized external exit
payload's `ByteCount` cannot convert to a host `usize`
(`executor/src/executor.rs:2596-2611`). The check happens before allocation,
backend collection, or insertion into the exited image list. The task and
declared bytes identify the exact egress that cannot be represented in host
address space.

The exit phase fails and the run tears down its arenas and backend resource.
The executor does not truncate the output, stream it in an alternate format,
or report a successful exit without the image.

Display is `exit task {task} payload of {bytes} does not fit host address space`.

### `ExitImageAllocationFailed { task, bytes }`

After the `usize` conversion succeeds, `collect_exit_image` uses
`Vec::try_reserve_exact` for the exact payload length. An allocation failure is
mapped to this variant (`executor/src/executor.rs:2612-2623`). The debug
assertion documents that allocation failure is unexpected under the measured
run bounds, but the release path still returns the typed error.

No partial vector is handed to the backend. The exit phase fails before
`Backend::collect_exit` and the normal teardown path runs. The executor does
not retry with a smaller buffer or replace the external result with an
intermediate artifact.

Display is `exit task {task} could not allocate its {bytes} result image`.

## Rendering and source behavior

`ExecutorError::fmt` is the exhaustive current renderer
(`executor/src/error.rs:184-305`). It renders typed IDs and byte counts using
their `Display` implementations, phases and backend operations using `Debug`,
and static details verbatim. In particular, backend diagnostics pass through
`BackendMessage::fmt`, so truncation is visible as a trailing `...` rather than
an allocation or an omitted error. There is no JSON, log-file, retry, or
secondary formatting path in this module.

`impl std::error::Error for ExecutorError` is intentionally empty
(`executor/src/error.rs:307`): `source()` returns `None` even for a backend
failure. Higher-level wrappers may expose the `ExecutorError` itself as their
source, as training and inference do, and worker errors expose it through their
`Journal` variant. The bounded backend message remains the only backend detail
that crosses the executor error boundary.

The direct construction inventory is complete in this module: `rg
"ExecutorError::" executor/src` finds all constructors in `executor.rs` and
`metrics.rs`; no other workspace Rust file constructs an `ExecutorError`.
Other crates either receive the public type, use `From<ExecutorError>` at the
training boundary, or wrap journal results in a worker/inference failure
container. Every constructor returns through a `Result` or a recoverable
failure object, and every failure leaves the requested lifecycle transition
uncompleted.
