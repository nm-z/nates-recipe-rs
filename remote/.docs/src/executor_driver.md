# Executor-backed worker driver

`remote/src/executor_driver.rs` is the integration adapter between the
bounded wire-facing `recipe_remote::WorkerDriver` trait and the local,
backend-neutral `recipe_executor::WorkerExecutionSession`. It is not a second
executor and it does not schedule a bundle. The remote session owns protocol
ordering, lane sequencing, message storage, and the master/worker typestate.
The executor session owns the worker's finalized projection, native pending
tokens, arenas, dependency and schedule checks, asynchronous polling, physical
accounting, watchdog, and teardown. `ExecutorWorkerDriver<B>` translates the
small remote method calls into the corresponding executor operation and
compresses executor failures into the allocation-free `DriverFault` wire
payload.

The relevant source pair is:

* `remote/src/executor_driver.rs:15-528`, `ExecutorDriverBuildError`,
  `ExecutorWorkerDriver`, the `WorkerDriver` implementation, projection
  validation, and fault encoding.
* `remote/src/driver.rs:7-124`, the wire-facing worker-driver contract and
  reserved fault namespace.
* `executor/src/worker.rs:26-2805`, `WorkerAssignment`,
  `WorkerProjection`, `WorkerBackend`, and `WorkerExecutionSession`.
* `remote/src/session.rs:83-3136`, the protocol caller of every driver method.

At this revision there is no `impl WorkerBackend` in the workspace outside the
trait and this adapter's generic bound. The existing host, CUDA, HSA, and
`LocalBackend` implementations implement the ordinary sealed
`recipe_executor::Backend` ABI. They are not directly usable as `B` here until
an adapter also implements the worker-specific extension. Thus this file is a
complete boundary and lifecycle implementation, but it is presently an
integration surface rather than a constructible production remote GPU
backend.

## Two layers and their ownership

The wire layer has only finalized identities and bounded data. `WorkerDriver`
receives a nonzero `RunId`, finalized `DeviceId` and `TaskId` values, image
digests and byte counts, and caller-owned slices whose lifetime extends through
the matching poll. It has no CPU calculation callback, closure, compiler,
loader, discovery, topology mutation, file operation, or vendor-math surface.
Its operations are `prepare`, init-image begin/chunk/end/poll, local task
submit/poll, external ingress/egress begin/poll/acknowledge, cancellation,
exit, per-device arena release, terminal finish, and fatal cleanup
(`remote/src/driver.rs:67-124`).

The executor layer is closed around finalized work. `WorkerBackend` extends the
sealed `recipe_executor::Backend` trait (`executor/src/worker.rs:1023-1081`):

* `bind_worker_resources` binds one immutable `WorkerProjection` and returns a
  backend resource.
* `prepare_external` creates one reusable external-transfer token for each
  projected ingress or egress task.
* `begin_external_ingress` and `begin_external_egress` submit bounded byte
  operations against the already allocated arenas.
* `poll_external` is nonblocking and returns `Pending` or an exact byte count.
* `acknowledge_external_egress` closes the master acknowledgment half of an
  egress operation.
* `quiesce_worker` is the native barrier used before arena release and fatal
  teardown.

All ordinary local work still uses `Backend::prepare_pending`,
`allocate_arena`, `submit`, `poll`, `release_arena`, and `destroy_resources`.
The ordinary ABI explicitly requires realization before `init` and forbids
allocation, loading, compilation, or lazy driver state changes from `submit`
and `poll` (`executor/src/backend.rs:329-446`). The worker extension adds only
the external-transfer and quiescence hooks that a remote worker needs.

The remote protocol never calls `Backend` directly. Its call chain is:

```text
Master/Worker typestate in remote/src/session.rs
    -> WorkerDriver method
    -> ExecutorWorkerDriver<B>
    -> WorkerExecutionSession<B>
    -> WorkerBackend or ordinary Backend method
    -> native operation and physical-call journal
```

The reverse path maps `WorkerTaskPoll`, `ExternalTransferPoll`, and
`MetricValue` back to `DriverPoll`, `DriverTransferPoll`, and
`RemoteMetricValue`; the protocol then emits `TaskComplete`, `Metric`,
`UserData`, `ReleaseAck`, or a terminal `DriverFault`.

## Construction proves the local projection matches the wire program

`ExecutorWorkerDriver::new` takes a `FinalizedBundle`, measured `Topology`,
exact `WorkerAssignment { machine, node }`, `ProvisionedProgram`, backend, and
`Watchdog` (`remote/src/executor_driver.rs:75-96`). Construction performs two
checks before any run or native resource is bound:

1. `WorkerProjection::derive` validates and freezes the worker's exact portion
   of the finalized bundle.
2. `validate_program` proves that the wire program is the same non-init view
   of that projection.

The returned object owns the bundle and projection, stores the program digest
as `expected_program`, and puts the backend in `Some`. `session`, `run`, and
the one in-flight init-chunk marker start as `None`. The accessor methods expose
the immutable projection, expected program digest, and optional active run;
they do not expose native resources (`executor_driver.rs:98-105`).

### Projection derivation

`WorkerProjection::derive` (`executor/src/worker.rs:374-548`) is a static,
fail-closed projection operation:

* It requires `Topology::validate` and
  `Topology::validate_scheduling_properties`, then requires the topology
  identity in the bundle to equal the supplied topology identity.
* The assigned machine must exist. The assigned node must exist, be a
  `NodeRole::Worker`, belong to that machine, and own at least one device.
* Every worker device must have a finalized arena layout, reservation, and init
  image in the bundle. Device IDs are sorted and become the projection's
  canonical device set.
* Every bundle task is classified against the local device set. A local
  calculation becomes local calculation work with finalized operand locations,
  kernel template, artifact, submission slots, and optional fault flag. A
  local metric becomes local metric work with its value location and metric
  slot. A local-to-local transfer becomes internal work, with `ExitTransfer`
  class in exit and `InternalTransfer` class in init or loop.
* An external-to-local device transfer is an init admission and is legal only
  in `RunPhase::Init`. A local-to-external transfer is a worker egress and is
  legal only in `RunPhase::Exit`. A device-to-device transfer with exactly one
  endpoint local to this worker becomes external ingress or egress; transfers
  wholly foreign to this worker are omitted.
* A projected external transfer must be planner-expanded to one measured link
  (`route.len() == 1`), the link direction must match the local endpoint, and
  it must carry exactly one link lane claim for that link. External claims,
  multiple link claims, or a lane number at or above the measured
  `maximum_inflight_transfers` are rejected.
* Local value operands must resolve to worker devices. Submission queues and
  completion slots must exist and belong to the same device. A local work
  submission must belong to a worker device; ingress uses the transfer source
  device, while egress uses its local source device. A metric slot must exist
  and name the projected metric.
* Projected dependencies are the original dependencies filtered to projected
  task IDs. The complete original dependency list is retained in each private
  task contract. Tasks are sorted by `TaskId` for binary lookup.
* Each worker device must have exactly one init admission. Its destination
  value and byte count must equal the finalized init image. Init may contain
  no other projected task.
* The projection records distinct calculation artifact IDs and computes an
  identity digest over a version tag, bundle and topology identities, machine
  and node assignment, sorted device IDs, and each task's ID, phase, role, and
  original dependencies (`executor/src/worker.rs:967-1000`). The digest is a
  description of the projection, not a replacement for the program digest.

`WorkerProjection` exposes only immutable summaries needed by the adapter:
identity, bundle/topology identity, assignment, devices, arena layouts, task
triples `(TaskId, RunPhase, WorkerTaskRole)`, artifact IDs, init-image byte
counts, external transfer contracts, and task role/phase lookup
(`executor/src/worker.rs:551-616`). No foreign task or device can be submitted
through the resulting session.

### Wire-program validation

`validate_program` (`executor_driver.rs:358-449`) adds the remote-specific
proof. It requires:

* `program.manifest().bundle == projection.bundle().digest()`;
* identical worker-device count, and every program device ID and image size to
  be present in the projection;
* identical count and sorted sequence of all projected tasks except init
  admissions, with matching task ID, phase, and kind. `WorkerTaskRole::Local`
  maps to `ProgramTaskKind::Driver`; ingress and egress map to
  `ProgramTaskKind::CrossTransfer`; an init admission in the runtime manifest
  is rejected;
* identical external-transfer count, and for every projection transfer a
  program transfer with matching task, direction, byte count, exactly one
  claim, and claim resource/mode matching the first measured topology link in
  the projection route.

Failures are setup errors, not wire driver faults:
`ExecutorDriverBuildError::Projection(WorkerProjectionError)` preserves a
projection error, while `ProgramMismatch(&'static str)` reports one of the
specific contract differences. `DriverFaultCode::PROGRAM_PROJECTION_MISMATCH`
is reserved in `remote/src/driver.rs` but is not emitted by this implementation.
The handshake separately proves protocol version, endpoint/profile identities,
limits, manifest bundle/draft/realization/artifact identities, and the complete
program digest before it invokes `prepare` (`remote/src/session.rs:828-923`).
`prepare` then checks the passed program digest again against the digest saved
at construction (`executor_driver.rs:146-185`).

## Driver-owned lifecycle and state

The adapter is an ownership state machine around an optional
`WorkerExecutionSession`:

```text
backend available, no session
          | prepare
          v
      Prepared  -- first image --> Init -- all image polls --> Loop
                                             | begin_exit
                                             v
                                           Exit
                 Loop or Exit -- cancel/quiesce --> Cancelling
                         Exit or Cancelling -- finish --> Finished
                 any live session -- fatal_cleanup --> Failed
```

The executor session's public lifecycle is the closed enum
`Prepared | Init | Loop | Exit | Cancelling | Finished | Failed`
(`executor/src/worker.rs:1102-1114`). The adapter only dispatches in `Loop` or
`Exit`; missing session, missing run, and every other phase return the reserved
`INVALID_LIFECYCLE` fault.

### `prepare`

`ExecutorWorkerDriver::prepare` first rejects a program whose digest differs
from `expected_program` with `PROGRAM_IDENTITY_MISMATCH`. It then rejects an
existing session or run, takes the backend from `backend`, and calls
`WorkerExecutionSession::prepare` with the owned bundle, cloned projection,
run, and watchdog.

Session preparation (`executor/src/worker.rs:1415-1583`) is deliberately
pre-init work:

1. The run ID must be nonzero, and projection bundle/topology identities must
   still equal the finalized bundle.
2. Fixed journal capacity is derived from the bundle plus two logical records
   per external transfer and a bounded physical-call allowance for every
   projected task. Arithmetic overflow is a preparation failure.
3. `bind_worker_resources` is called once and its physical calls are recorded.
4. Every projected external transfer calls `prepare_external`. Every local
   task calls ordinary `Backend::prepare_pending` with its finalized task,
   phase, work class, and submission slots. These calls create reusable tokens
   before any init image is admitted.
5. Exact image buffers are allocated for each worker device, and a logical
   `Prepared` event is recorded. The session starts with lifecycle `Prepared`,
   loop iteration zero, no arenas, all task slots `Idle`, and image states
   `Needed`.

On any preparation failure, `WorkerPrepareFailure` retains the original
backend. Once a resource has been bound, later preparation failures attempt
quiesce and resource destruction; early validation and bind failures have no
resource to clean. The path keeps the primary error and first cleanup or
journal error separately, and returns both through `into_parts`
(`executor/src/worker.rs:1326-1383`, `2708-2764`). The adapter returns
`EXECUTOR_PREPARE_FAILED` for the primary error. If cleanup also failed, it maps the cleanup to
`EXECUTOR_CLEANUP_FAILED` and combines both codes with
`DriverFault::cleanup_failed`, whose code is `FATAL_CLEANUP_FAILED` and whose
detail packs the primary code in the high 32 bits and cleanup code in the low
32 bits.

### Init image

The remote protocol delivers one logical image per worker device as a begin,
ordered chunks, and an end. `WorkerInit` serializes images and chunks, hashes
the wire bytes, and keeps the caller-owned chunk buffer stable until the
matching poll (`remote/src/session.rs:1176-1487`). The adapter adds a second
copy and executor-side digest check:

* `begin_init_image` calls `session.begin_init_image` with the current run,
  device, finalized byte count, and digest. The session checks `Prepared` or
  `Init`, rejects duplicate images or size mismatch, allocates the exact arena,
  stores the expected digest, clears its image buffer, and enters `Init`.
* `begin_init_chunk` is synchronous at this adapter boundary. It rejects a
  second pending chunk, calls `write_init_chunk`, and records the copied byte
  count and device in `pending_init_chunk`. `write_init_chunk` requires the
  exact next offset and bounds the range against the finalized image.
* `poll_init_chunk` requires the same device, clears the one pending marker,
  and returns `Complete { bytes }`. There is no native asynchronous operation
  for this copy.
* `finish_init_image` requires that no chunk is pending and calls
  `submit_init_image`. The executor requires a complete image, hashes its full
  internal buffer, checks the digest, checks projected dependencies, and
  submits the init-admission `BackendWork` token.
* `poll_init_image` forwards the executor poll. A pending backend completion
  remains pending. A complete admission must have no metric, marks the image
  and admission task complete, and when every image is complete changes the
  executor lifecycle to `Loop`, recording `Initialized` and `LoopStarted`.

The protocol does not admit runtime work while an image, chunk, or final image
poll is active. It also rejects overlapping images, out-of-order chunk
schedule or offsets, wrong lengths, a changed digest, and `InitComplete` before
all per-device acknowledgments.

### Local task submit and poll

`submit_task` obtains the current run and active phase, then calls
`WorkerExecutionSession::submit_task`. The session requires a local role, exact
phase, idle slot, complete projected dependencies, and a permitted schedule
window before calling ordinary `Backend::submit` with the finalized
calculation, internal transfer, exit transfer, or metric work. It records a
logical `TaskSubmitted` event and marks the slot `Active`.

`poll_task` requires an active local task and forwards `Backend::poll`.
`Pending` increments that task's nonprogress watchdog count. Completion resets
the count, validates the metric contract, marks the task `Complete`, records
`TaskCompleted`, and returns an optional metric. User metrics must have the
finalized F32 or I32 dtype. Fault-readback metrics require I32 zero for normal
completion; a nonzero I32 becomes `DeviceFault`. Init, calculation, and
internal/exit transfer work must complete without a metric. Any metric on an
external task is a role error.

The worker adapter maps executor `MetricValue::F32` and `I32` into the wire's
`RemoteMetricValue` without changing the scalar. `WorkerRuntime` sends a
metric frame before the corresponding `TaskComplete` and waits for the metric
frame's transport completion token (`remote/src/session.rs:2027-2127`).

The executor session stores only `loop_iteration` zero and calls ordinary
`Backend::submit` with that iteration (`executor/src/worker.rs:1840-1900`). It
does not call `Backend::submit_loop_iteration` and the remote
`ProvisionedProgram` has no loop-count field. Consequently this remote worker
surface executes the provisioned loop task graph once, at iteration zero. It
does not implement the local executor's repeated finite or unbounded loop
iteration machinery.

### External ingress and egress

The remote worker runtime owns one fixed incoming scratch buffer and one fixed
outgoing scratch buffer. On `Message::UserData`, it validates direction,
static schedule, byte count, phase, and idle state, copies bytes into the
incoming buffer, and calls `begin_receive_user_data`. The adapter calls
`begin_external_ingress`; the worker backend owns the asynchronous transfer.
`poll_receive_user_data` forwards `poll_external_ingress`. On exact-byte
completion, the runtime clears the scratch buffer and reports the task
complete (`remote/src/session.rs:2129-2151`, `2191-2255`).

For worker-to-master data, the master sends `DataRequest`. The worker validates
the static transfer, calls `begin_produce_user_data` with the stable outgoing
slice, and polls through `poll_produce_user_data`. Completion must report the
exact provisioned length. The worker sends the bytes with the static
direction-specific schedule and changes the transfer to `WaitAck`. A master
`DataAck` calls `user_data_acked`; the adapter calls
`acknowledge_external_egress`, which requires the executor task state
`AwaitingAck`, invokes `WorkerBackend::acknowledge_external_egress`, marks the
task `Complete`, and records `ExternalTransferCompleted`.

The executor-side external state is therefore:

```text
Idle -> Active -> Complete       ingress
Idle -> Active -> AwaitingAck -> Complete   egress
```

Half-duplex capacity resources are represented by one fixed token per resource
in both master and worker storage. A transfer acquires all of its half-duplex
claims before submission and releases them only on terminal completion or
failure. Full-duplex transfers do not share that token.

### Exit, cancellation, and backend recovery

`begin_exit` calls the session only after the remote runtime has marked every
loop task complete. The executor requires lifecycle `Loop` and all projected
loop slots `Complete`, changes to `Exit`, and records `LoopCompleted`.
`WorkerExit` first sends `ExitReady`, then admits and polls exit tasks using the
same local and external calls. The master requests each device release only
after exit phase completion. `release_arena` requires `Exit` with every exit
task complete, or `Cancelling`, and releases each exact device once.

`finish` requires `Exit` or `Cancelling` and an empty arena map. It takes the
backend resource, calls `destroy_resources`, marks the session `Finished`,
records `Exited`, and calls `recover_finished_session`. Recovery consumes the
session through `into_parts`, returns the backend to `backend`, clears `run`
and `pending_init_chunk`, and leaves the driver reusable for a later run. A
session whose resource is still present or whose lifecycle is not `Finished` or
`Failed` cannot be recovered.

Cancellation deliberately follows a different protocol path from a driver
fault. The master sends `Cancel` with a reason, but the adapter currently binds
`reason` to `_reason` and the executor session does not retain it. The session
quiesces native work, marks active and awaiting tasks `Quiesced`, and enters
`Cancelling`. `WorkerCancelled` then releases every arena one at a time, waits
for each `ReleaseAck`, calls `finish`, flushes `CancelAck`, and returns
`WorkerComplete { cancelled: true }`.

Fatal cleanup is used only after a driver operation has already returned a
fault. `cleanup_after_fault` ignores the primary value, invokes
`WorkerExecutionSession::fatal_cleanup`, and then attempts the same finished or
failed-session recovery. `fatal_cleanup` quiesces all native queues before
releasing arenas, zeroes image buffers, destroys the resource, marks the
session `Failed`, and returns the first cleanup or journal error. If quiescence
fails it marks `Failed` and returns immediately, deliberately preserving arenas
and the resource because native work may still reference them. Release and
destroy failures are accumulated as a first error while cleanup continues.
The adapter maps a cleanup error to `EXECUTOR_CLEANUP_FAILED`; the protocol's
`begin_driver_fault` combines it with the original fault before sending one
terminal `DriverFault`.

`Drop` is a last-resort local safety path: if a live session remains, it calls
`fatal_cleanup` and ignores the result (`executor_driver.rs:138-143`). Normal
protocol completion always calls `finish` and recovers the backend before the
driver is dropped.

## Remote protocol callers

The worker driver is called only from the following remote typestates. Every
state owns the driver and returns it in the next typestate, so there is no
shared mutable protocol object that can bypass the order.

### Handshake

`WorkerHandshake::progress` first exchanges `Hello`, validates the peer role,
required capabilities, endpoint/profile identities, and fixed limits, then
checks the master's manifest proof. Only on a valid `Prepare` message does it
call `driver.prepare` (`remote/src/session.rs:817-923`). A successful prepare
is followed by `PrepareAck` and transition to `WorkerInit`. A driver fault
starts cleanup before the fault frame is sent. A malformed or out-of-order
handshake message poisons the core instead.

### Init

`WorkerInit::progress` serializes chunk polling, final-image polling, and frame
processing. It keeps `chunk_buffer` stable through the matching
`poll_init_chunk`, and the driver's own session keeps a separate image buffer.
`InitAck` is sent only after the final admission poll completes. `InitComplete`
is accepted only when all image states are `Complete`, and the worker sends
`InitCompleteAck` before transitioning to `WorkerRun`.

### Loop run

`WorkerRun::progress` progresses transport, then any existing incoming transfer,
one active native task in round-robin order, completed metric or task reports,
received-data acknowledgments, and outgoing data. It then consumes one control
or user-data frame:

* `Execute` validates a provisioned driver task and calls `submit_task`.
* `UserData` validates the static cross-transfer contract and calls
  `begin_receive_user_data`.
* `DataRequest` validates the opposite direction and calls
  `begin_produce_user_data`.
* `Cancel` calls `cancel` and enters `WorkerCancelled`.
* `BeginExit` requires complete loop storage, calls `begin_exit`, and enters
  `WorkerExit`.

Any driver error is converted to one cleanup-first terminal fault. Any remote
protocol or transport error poisons the session. There is no implicit retry or
alternate backend path.

### Exit

`WorkerExit::progress` sends `ExitReady`, continues active exit work, and
accepts only `Release` after the exit task set is complete. Each successful
`release_arena` changes one release state to `Requested`; the corresponding
`ReleaseAck` is sent only after the transport can accept it. `ExitComplete` is
accepted only after every release is `Complete`. The worker then calls
`finish`, sends `ExitAck`, waits for its transmission token to flush, and
returns `WorkerComplete { cancelled: false }`.

`Cancel` remains legal in exit and transfers control to the same
`WorkerCancelled` release and finish sequence. A driver fault at any point
enters the cleanup-only path and never attempts protocol-level per-arena
acknowledgments.

## Transport and storage guarantees at the boundary

`SessionCore` performs the connection-global transport checks before any
driver call (`remote/src/session.rs:407-688`):

* `RemoteChannel` preserves the last completed run ID and transport schedule
  positions across a reused channel. New runs must be nonzero and strictly
  increasing.
* Three per-lane transmit and receive sequence counters cover control,
  metrics, and user data. `try_send_tracked` encodes into one reusable scratch
  buffer, applies a connection-global user-data schedule base, and returns
  `None` for bounded transport capacity exhaustion instead of allocating.
* `received` decodes one frame, verifies its lane, run ID, expected sequence,
  and schedule relative to the current run, then advances the receive sequence.
  The caller must release the exact received completion token.
* A poisoned core rejects every subsequent operation. Probe frames cannot reach
  the runtime lane decoder.

The fixed program limits create all task, data, metric, transfer-token,
release, scratch, and active-task storage during typestate construction. The
driver therefore receives stable slices and IDs, never a socket or an
unbounded collection. Backpressure is explicit at the protocol boundary;
executor polling remains nonblocking and bounded by the per-task watchdog.

## Fault namespace and conversion

`DriverFault` is two integers, `code: u32` and `detail: u64`, designed to be
copied through the protocol without allocation. Reserved integration codes
are:

| Code | Meaning |
| --- | --- |
| `0x5245_0002` | `PROGRAM_IDENTITY_MISMATCH` from the prepare digest check |
| `0x5245_0003` | `INVALID_LIFECYCLE` for a missing or illegal adapter state |
| `0x5245_0004` | `FATAL_CLEANUP_FAILED`, combined primary and cleanup codes |
| `0x5245_0005` | `PROGRAM_PROJECTION_MISMATCH`, reserved but unused here |
| `0x5245_0006` | `EXECUTOR_PREPARE_FAILED` |
| `0x5245_0007` | `EXECUTOR_OPERATION_FAILED` |
| `0x5245_0008` | `EXECUTOR_CLEANUP_FAILED` |

`executor_fault` (`executor_driver.rs:460-492`) intentionally preserves only a
bounded discriminator in `detail`:

* projection or bundle mismatch is `1`;
* invalid or actual run IDs retain the run number;
* task, role, dependency, schedule, metric, duplicate-dispatch,
  non-active-task, phase-incomplete, device-fault, and watchdog failures retain
  the task or readback task ID;
* unknown device, init digest, already-released arena, and init-offset failures
  retain the device ID;
* wrong-phase and byte-count failures retain their optional task ID, or zero;
* invalid lifecycle maps `Prepared=1`, `Init=2`, `Loop=3`, `Exit=4`,
  `Cancelling=5`, `Finished=6`, `Failed=7`;
* backend failures map the physical operation to
  `BindProjection=1`, `PrepareLocal=2`, `PrepareExternal=3`,
  `AllocateArena=4`, `SubmitLocal=5`, `PollLocal=6`, `SubmitIngress=7`,
  `SubmitEgress=8`, `PollExternal=9`, `AcknowledgeEgress=10`, `Quiesce=11`,
  `ReleaseArena=12`, `DestroyResources=13`;
* journal failure is `u64::MAX - 1`, capacity overflow is `u64::MAX`, and
  unlisted executor errors use `u64::MAX - 2`.

The original executor error and backend source are intentionally not sent over
the wire. `RemoteError::Driver` reports only this stable code/detail pair;
`RemoteError::Protocol`, `ManifestMismatch`, `RunMismatch`, `CapacityExhausted`,
`Backpressured`, `Transport`, and `Poisoned` remain distinct protocol-side
errors (`remote/src/error.rs:7-24`).

## End-to-end role and invariants

For a real worker run, the complete path is:

1. Provisioning derives one `ProvisionedProgram` from the same finalized bundle,
   measured topology, worker-device set, and fixed limits on both peers.
2. The worker constructs `ExecutorWorkerDriver` with the exact assignment and
   a backend that implements `WorkerBackend`; projection and program checks
   fail before native binding if any identity, device, task, transfer, route,
   or lane contract differs.
3. Handshake proves endpoint/profile identities, capabilities, limits, plan
   digests, artifacts, and program digest. `Prepare` pre-binds resources and
   all local and external pending tokens before init.
4. Init transfers exactly one digest-checked image per worker device in ordered
   chunks. The adapter allocates each arena once, submits the admission token,
   and waits for its native completion. Only then is loop dispatch legal.
5. Master commands identify only provisioned task IDs. Local calculations,
   internal transfers, and metrics use ordinary finalized executor work;
   cross-machine ingress and egress use the worker extension and exact static
   byte/schedule/lane contracts. Dependencies, phase, role, queue ownership,
   schedule admission, watchdog, metrics, and device fault flags are checked by
   the executor session before completion is reported.
6. Exit is entered only after loop completion. Exit tasks finish, each arena is
   released exactly once with an acknowledgment, and `finish` destroys native
   resources before the terminal acknowledgment. Cancellation uses the same
   ordered release and finish, but reports a cancelled terminal state.
7. Any driver fault first quiesces and tears down locally, then sends one
   terminal fault. A cleanup failure is combined with the primary fault; an
   ordinary terminal fault therefore means the worker's native resources are
   already quiesced and released. Dropping an abandoned adapter still invokes
   fatal cleanup when a session remains.

The local executor's analogous lifecycle is
`PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun`
(`executor/src/executor.rs:795-1424`). It realizes all phase tokens before
init, allocates all arenas during initialization, schedules repeated loop
iterations, runs exit work, and tears down. The remote worker session retains
the same preparation, arena, nonblocking poll, journal, and ordered teardown
principles, but the wire master supplies dispatch order and the current remote
implementation represents one loop iteration. The adapter is consequently the
single place where remote protocol data is admitted to that local executor
boundary; it does not duplicate backend work or reconstruct domain state.

### Current implementation edge behavior

The following details are observable in the current code and are part of this
documentation rather than assumptions about a future design:

* `ExecutorWorkerDriver::cancel` receives a protocol reason but deliberately
  discards it before invoking the executor session.
* Init chunk polling completes immediately after the adapter copies bytes into
  the executor image buffer; only final init admission polling can remain
  native-asynchronous.
* The executor's `ensure_dispatch` schedule check currently searches for an
  active or awaiting task whose window **does not** overlap the candidate
  (`!slot.contract.window.overlaps(contract.window)`) before returning
  `ScheduleConflict` (`executor/src/worker.rs:2483-2491`). Since
  `ScheduleWindow::overlaps` is the ordinary interval-intersection predicate,
  the implemented rejection condition is the inverse of the usual
  overlapping-window interpretation. This is the actual behavior callers see.
* `fatal_cleanup` returns immediately on quiesce failure, preserving arenas and
  the resource. Release and destroy errors are instead accumulated while
  cleanup continues, with the first error returned.
* `PROGRAM_PROJECTION_MISMATCH` exists in the reserved code namespace but all
  projection or bundle failures reaching the live adapter are encoded as
  `EXECUTOR_PREPARE_FAILED` or `EXECUTOR_OPERATION_FAILED` with a bounded
  detail.
* The repository currently has no concrete worker backend implementation or
  caller constructing `ExecutorWorkerDriver`; remote protocol tests or future
  native adapters must supply that missing `WorkerBackend` integration. The
  generic adapter itself compiles against the closed executor ABI and keeps the
  required ownership, validation, polling, and cleanup semantics.
