# Remote worker driver boundary

This document describes `remote/src/driver.rs` and the callers that give its
`WorkerDriver` contract meaning. The module is the narrow native execution
boundary of `recipe-remote`: a worker session has already completed
provisioning and handshake validation, and the driver receives only finalized
run, device, task, digest, and byte-slice values. It does not establish a
connection, discover hardware, compile an artifact, read a file, calculate on
the host, or accept an arbitrary callback.

The public items are re-exported by `remote/src/lib.rs`:

```rust
DriverFault, DriverFaultCode, DriverPoll, DriverTransferPoll, WorkerDriver
```

`remote/src/executor_driver.rs` supplies the in-tree implementation,
`ExecutorWorkerDriver`, over a `recipe-executor::WorkerExecutionSession`.
Other native backends can implement the same trait without changing the wire
state machine.

## Boundary types

### `DriverFault`

`DriverFault` is the allocation-free fault payload that can be returned from a
driver method and encoded in a `TaskFailed` or terminal `DriverFault` message.
It is `Copy`, `Debug`, `Eq`, and contains only:

```text
code: u32
detail: u64
```

`DriverFault::new(code, detail)` stores those values unchanged. The wire codec
encodes the code as little-endian `u32` and detail as little-endian `u64`.

`DriverFault::cleanup_failed(primary, cleanup)` is used when the required
fault cleanup itself fails. It replaces the code with
`DriverFaultCode::FATAL_CLEANUP_FAILED` and packs the original and cleanup
codes into one detail value:

```text
detail = (primary.code as u64 << 32) | cleanup.code as u64
```

The detail does not include either original detail value. A receiver can still
distinguish an ordinary native fault from a cleanup failure without allocating
or parsing text.

### `DriverFaultCode`

This is the Recipe-reserved namespace for faults produced by the remote or
executor integration boundary. `get()` returns the raw `u32`. A backend-native
implementation may use another code. The currently reserved values are:

| Constant | Value | Meaning |
| --- | --- | --- |
| `PROGRAM_IDENTITY_MISMATCH` | `0x5245_0002` | `prepare` received a program digest different from the adapter's expected digest |
| `INVALID_LIFECYCLE` | `0x5245_0003` | a method was called without the required active session, run, phase, or pending transfer |
| `FATAL_CLEANUP_FAILED` | `0x5245_0004` | the primary fault was followed by unsuccessful fatal cleanup |
| `PROGRAM_PROJECTION_MISMATCH` | `0x5245_0005` | reserved integration mismatch between a wire program and a worker projection |
| `EXECUTOR_PREPARE_FAILED` | `0x5245_0006` | `WorkerExecutionSession::prepare` failed |
| `EXECUTOR_OPERATION_FAILED` | `0x5245_0007` | an active executor operation failed |
| `EXECUTOR_CLEANUP_FAILED` | `0x5245_0008` | executor fatal cleanup failed |

`PROGRAM_PROJECTION_MISMATCH` is used as a reserved boundary code, while the
current `ExecutorWorkerDriver::new` reports projection mismatches as the
construction-only `ExecutorDriverBuildError::ProgramMismatch` before a
`WorkerDriver` exists.

### Completion polls

`DriverPoll` describes one already-submitted native task:

- `Pending` means the task remains active and the caller must poll again.
- `Complete { metric: None }` means the task completed without a scalar
  readback.
- `Complete { metric: Some(RemoteMetricValue) }` means the task completed and
  produced one finalized `f32` or `i32` metric. The remote session sends that
  scalar on its metrics lane before it sends the task completion control frame.

`DriverTransferPoll` is the corresponding nonblocking state for one byte
transfer. `Complete { bytes }` reports the actual completed length. The remote
session compares that length with the finalized transfer or image length before
advancing its protocol state. A driver must not use a completion value to
change the protocol's expected size.

## `WorkerDriver` contract

`WorkerDriver: core::fmt::Debug` is intentionally a method-only boundary. The
session owns command ordering, fixed storage, transport progress, task roles,
phase checks, and wire acknowledgments. The implementation owns native queues,
arenas, pending backend work, and native resource lifetime.

The valid high-level call sequence is:

```text
prepare
  -> begin_init_image / begin_init_chunk / poll_init_chunk / finish_init_image / poll_init_image
  -> submit_task / poll_task and external transfer begin/poll/ack calls
  -> begin_exit
  -> exit submit/poll and external transfer calls
  -> release_arena for every worker device
  -> finish
```

Cancellation branches from the loop or exit path to `cancel`, then uses the
same per-arena `release_arena` and `finish` sequence. A method error that the
session treats as terminal branches to `cleanup_after_fault` instead of
continuing the ordinary release protocol.

### Preparation

`prepare(run, program)` binds the finalized local executor and pre-realizes
all work needed by the later calls. It runs after the worker has accepted the
master's manifest and received `Message::Prepare`, and before any init image is
admitted. `run` is nonzero and `program` is the exact `ProvisionedProgram`
validated during `WorkerHandshake` construction. A successful call is the
worker's proof that local execution is ready for the init phase.

The trait does not provide a compiler, artifact loader, discovery hook, or host
calculation callback. Native realization belongs to this preparation step and
is not a runtime task.

### Init image admission

The following methods admit exactly one finalized logical image for each worker
device:

```rust
fn begin_init_image(&mut self, device: DeviceId, bytes: ByteCount, digest: Digest)
fn begin_init_chunk(&mut self, device: DeviceId, offset: u64, bytes: &[u8])
fn poll_init_chunk(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault>
fn finish_init_image(&mut self, device: DeviceId)
fn poll_init_image(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault>
```

`begin_init_image` starts the arena admission contract. `bytes` is the exact
image size from the program and `digest` is the SHA-256 digest supplied by the
master. The worker session rejects an unknown device, a repeated image, a size
mismatch, or an image begun outside `Prepared` or `Init` before invoking the
driver.

`begin_init_chunk` receives a contiguous chunk at exactly the next offset. The
slice must remain valid until the matching `poll_init_chunk` reports
completion. The session sends chunks on the user-data lane with a static
schedule position, permits only one outstanding chunk, and zeroes its staging
buffer after completion. The driver must not retain the slice after the poll
returns.

`finish_init_image` is called only after all chunks have arrived in order and
the session's SHA-256 matches the declared digest. It submits the final native
admission. `poll_init_image` polls that submission. The worker emits `InitAck`
only after this poll returns `Complete` with the exact image byte count. After
every device has been acknowledged, `InitCompleteAck` transitions the worker to
the run phase.

The protocol permits no overlapping images or chunks. The logical image is
admitted once, and an implementation must not silently accept a partial or
out-of-order image.

### Local tasks and metrics

```rust
fn submit_task(&mut self, task: TaskId) -> Result<(), DriverFault>
fn poll_task(&mut self, task: TaskId) -> Result<DriverPoll, DriverFault>
```

`submit_task` is called for a finalized `ProgramTaskKind::Driver` after the
master sends `Message::Execute`. The worker checks task identity, active phase,
role, idle status, projected dependencies, schedule windows, and fixed task
slot capacity before dispatching it. The driver receives no calculation graph
or host-side operation, only the task ID.

The worker polls active tasks round-robin. `Pending` leaves the task active.
When a poll completes with a metric, the worker sends `Message::Metric` on the
metrics lane and waits for that frame to flush before sending
`Message::TaskComplete` on control. Without a metric it sends the completion
frame directly. A driver error returned by `poll_task` is recorded as a
task-local `TaskFailed { task, fault }` result by `WorkerRuntime`; this path is
different from errors returned by submit or transfer methods, which enter
terminal driver-fault cleanup.

### External ingress and egress

The two directions are deliberately explicit:

```rust
fn begin_receive_user_data(&mut self, task: TaskId, bytes: &[u8])
fn poll_receive_user_data(&mut self, task: TaskId)
fn begin_produce_user_data(&mut self, task: TaskId, destination: &mut [u8])
fn poll_produce_user_data(&mut self, task: TaskId)
fn user_data_acked(&mut self, task: TaskId)
```

For ingress, the master sends a statically scheduled `Message::UserData`.
The worker verifies direction, task phase, schedule, and exact byte count,
acquires any finalized half-duplex token, copies the payload into worker-owned
scratch, and calls `begin_receive_user_data`. The input slice is that scratch
storage and remains stable until `poll_receive_user_data` completes. The
session zeroes it and reports `DataAccepted` only after the exact length is
returned. The worker then sends the cross-transfer `TaskComplete` and releases
the token.

For egress, the master sends `Message::DataRequest`. The worker allocates no
per-request storage: it passes a bounded slice of its fixed `data_scratch` to
`begin_produce_user_data`, then polls it. On exact completion it sends the
resulting bytes as a user-data-lane `Message::UserData` with the transfer's
static schedule and retains the scratch until the master sends `Message::DataAck`.
Only then does `user_data_acked` run, the scratch get zeroed, the task become
complete, and the half-duplex token get released. An egress implementation must
not overwrite or release caller-owned destination storage before its matching
poll and acknowledgment contract permits it.

The finalized `ProvisionedProgram` limits each cross transfer to its planned
one-hop route, byte count, direction, schedule, and capacity claims. The
driver never chooses a route or a transfer resource.

### Cancellation and normal exit

`cancel(reason)` receives the wire cancel reason code. The session calls it
while in loop or exit, then enters `WorkerCancelled`. The driver must quiesce
active native work, after which the worker releases every arena exactly once,
waits for each `ReleaseAck`, calls `finish`, flushes `CancelAck`, and returns
`WorkerComplete { cancelled: true }`.

`begin_exit()` is called for `Message::BeginExit` only after every loop task is
complete. It starts the driver's exit phase. The worker sends `ExitReady`, runs
the finalized exit tasks through the same submit/poll and external transfer
methods, and accepts `Message::Release` only after all exit work is complete.

`release_arena(device)` releases exactly one still-owned worker arena. The
worker sends `ReleaseAck` only after the call succeeds. `finish()` is called
only after all release acknowledgments have been flushed and the master has
sent `Message::ExitComplete`; it destroys the remaining local execution
resource. The worker flushes `ExitAck` and returns `WorkerComplete { cancelled:
false }`.

### Terminal fault cleanup

`cleanup_after_fault(primary)` is the exceptional lifecycle boundary. It must
quiesce every active operation, release every locally realized arena, clear or
invalidate native image storage as appropriate, destroy execution resources,
and return only when native work can no longer reference an arena. The method
must be deterministic and safe when some resources were already released. It
has no protocol-level per-arena acknowledgment.

`session.rs` calls `begin_driver_fault` exactly once for a terminal driver
failure. That helper first invokes `cleanup_after_fault`. It reports the
original fault if cleanup succeeds, or `DriverFault::cleanup_failed` if cleanup
fails. The worker then queues one terminal `Message::DriverFault`, waits for
its transport completion token to flush, and only then returns
`RemoteError::Driver`. A second terminal fault attempt is a protocol error.
Thus receipt of an ordinary terminal driver-fault frame means the worker's
local cleanup has already completed. A task-local error from `poll_task` is the
intentional exception described above and is sent as `TaskFailed`.

## Connection and command flow

`recipe-remote` does not open a socket. The caller constructs a connected
`recipe_transport::RuntimeChannel` and wraps it in `RemoteChannel`. A new
`SessionCore` validates the channel's endpoint identities, the nonzero run ID,
the manifest and init schedule, and strict run monotonicity when a completed
`RemoteChannel` is reused. It preallocates one codec scratch buffer and tracks
independent control, metrics, and user-data sequence numbers.

Every session progress call first advances the nonblocking channel. Encoded
messages carry the current run ID and per-lane sequence. Control messages use
the control lane, metrics use the metrics lane, and `InitChunk` plus user data
use the user-data lane. User-data frames add the run's connection-global
schedule base to their finalized logical schedule. A full fixed lane reports
backpressure, not an alternate transport path.

Received frames are accepted only when all of these checks pass:

1. The frame is a runtime frame, not a probe frame.
2. The codec version, size, digest, message tag, and payload are valid.
3. The encoded run equals the active nonzero `RunId`.
4. The message's lane matches the transport frame kind.
5. The per-lane sequence is the exact next sequence, not a replay or skip.
6. The schedule is not before the connection-global inbound base.

Any violation poisons the session. The session releases each received transport
completion token after the corresponding state transition. Transmitted tokens
are released by `progress_transport`; a caller cannot reuse a fixed slot before
that release.

### Fixed bounds that reach the driver

The driver sees capacities that were checked while `ProvisionedProgram` was
constructed, not values selected during execution:

- `RunId`, `TaskId`, `DeviceId`, artifact IDs, transfer schedules, and capacity
  resources must be nonzero. The schedule value `u64::MAX` is reserved by the
  transport as the unscheduled sentinel.
- `RemoteLimits::new` requires `max_message_bytes >= 256` and a value that fits
  the transport `u32` wire length. Manifest artifact, task, device, and
  transfer capacities, plus task, data, and metric slot counts, are all
  nonzero.
- An init chunk carries a 28-byte remote codec header and 20 bytes of payload
  fields, so its maximum logical chunk is
  `max_message_bytes - INIT_CHUNK_OVERHEAD`, where
  `INIT_CHUNK_OVERHEAD` is exactly `48`. The init schedule reserves one
  position for every such chunk across all worker devices.
- A user-data transfer must fit the remote codec's
  `USER_DATA_OVERHEAD` of `28 + 12` bytes, and therefore cannot exceed
  `max_message_bytes - USER_DATA_OVERHEAD`. Cross-transfer construction also
  rejects zero byte counts, duplicate capacity resources, and invalid one-hop
  route claims.
- Worker init and data scratch buffers are allocated from the configured
  message bound. Worker task slots, metric slots, half-duplex tokens, and
  release records are fixed arrays derived from the same finalized limits.

These bounds explain why a driver method receives a bounded slice and a
finalized ID instead of a length or resource policy to decide for itself.

The command-to-driver mapping is:

| Wire message received by the worker | Driver operation | Worker-side validation and response |
| --- | --- | --- |
| `Prepare` | `prepare(run, program)` | Manifest proof and handshake flags must be complete; success sends `PrepareAck`. |
| `InitBegin` | `begin_init_image(device, bytes, digest)` | One known, not-yet-admitted device; exact finalized image size. |
| `InitChunk` | `begin_init_chunk`, then repeated `poll_init_chunk` | One contiguous scheduled chunk at the next offset; completion length must match. |
| `InitEnd` | `finish_init_image`, then repeated `poll_init_image` | The complete SHA-256 image must match; completion length must match. |
| `Execute` | `submit_task`, then repeated `poll_task` | Only an idle projected local task in the active phase; sends metric if present, then `TaskComplete`, or task-local `TaskFailed` on poll fault. |
| `UserData` | `begin_receive_user_data`, then `poll_receive_user_data` | Static ingress direction, schedule, and byte count; completion sends cross-transfer `TaskComplete`. |
| `DataRequest` | `begin_produce_user_data`, then `poll_produce_user_data` | Static egress direction, schedule, and byte count; completion sends `UserData` and waits for `DataAck`. |
| `DataAck` | `user_data_acked` | Must match the one egress transfer in `WaitAck`; then clears storage and completes it. |
| `Cancel` | `cancel(reason)` | Quiesces native work and branches to exact release and cancel acknowledgment. |
| `BeginExit` | `begin_exit` | Only after loop completion; success sends `ExitReady`. |
| `Release` | `release_arena(device)` | Only after exit completion or cancellation; success later sends `ReleaseAck`. |
| `ExitComplete` | `finish` | Only after all exit tasks and arena acknowledgments; success flushes `ExitAck`. |

The worker's responses that are not direct driver calls are
`InitAck`, `InitCompleteAck`, `TaskComplete`, `TaskFailed`, `Metric`,
`UserData`, `ExitReady`, `ReleaseAck`, `CancelAck`, `ExitAck`, and terminal
`DriverFault`; `DataAccepted`, `DataAcknowledged`, `TaskAccepted`, and
`TaskReported` are local `WorkerRunEvent` values returned to the caller while
those wire responses are being progressed.
The master never calls a `WorkerDriver` method. It records these responses in
`MasterRunEvent` or `MasterExit` state and applies the same static task and
transfer contracts.

## Concrete `ExecutorWorkerDriver`

`remote/src/executor_driver.rs` is the in-tree adapter from this trait to
`recipe_executor::WorkerExecutionSession<B>`, where `B: WorkerBackend +
Debug`. Its fields are:

| Field | Purpose |
| --- | --- |
| `bundle` | The finalized bundle used to prepare the executor session. |
| `projection` | Immutable `WorkerProjection` for one machine and worker node. |
| `expected_program` | Digest of the exact wire `ProvisionedProgram`. |
| `watchdog` | Executor polling bound passed into preparation. |
| `backend` | The unprepared backend, held in `Some` until `prepare`. |
| `session` | The active prepared `WorkerExecutionSession`, or `None`. |
| `run` | The active nonzero `RunId`, or `None` after recovery. |
| `pending_init_chunk` | At most one copied init chunk awaiting its driver poll. |

### Construction proof

`ExecutorWorkerDriver::new` first derives `WorkerProjection::derive` for the
supplied `FinalizedBundle`, measured `Topology`, and `WorkerAssignment`. That
projection rejects invalid topology, topology identity mismatch, unknown or
misassigned nodes, empty workers, missing arenas, reservations or images,
invalid task resources, foreign value references, and missing or duplicate
init admissions. It then calls `validate_program` before retaining the backend.

`validate_program` proves the wire program is the exact non-init projection:

- The manifest bundle digest equals the projection bundle digest.
- Device count, device IDs, and finalized init image byte counts match.
- The program contains exactly the projected tasks after filtering out
  `RunPhase::Init`.
- A projected local task maps to `ProgramTaskKind::Driver`; ingress and egress
  projections map to `ProgramTaskKind::CrossTransfer`; an init admission in
  the runtime manifest is rejected.
- Every task ID, phase, and role-derived kind matches in the finalized order.
- Cross-transfer count, task identity, direction, byte count, and exactly one
  duplex claim match the first topology link's capacity resource and duplex
  mode. An absent route link is a construction error.

Construction failures are `ExecutorDriverBuildError::Projection` or
`ProgramMismatch` and occur before a session or wire fault exists. The adapter
stores the program digest as `expected_program`; its `projection()` and
`expected_program()` accessors expose the proof inputs, and `active_run()`
reports the current run.

### Adapter method behavior

The adapter's private `session_mut` and `current_run` reject calls before
preparation or after recovery with `INVALID_LIFECYCLE`. `active_phase` maps
executor lifecycle `Loop` and `Exit` to the remote phases and rejects every
other lifecycle for dispatch.

The trait methods then map directly to executor operations:

| `WorkerDriver` method | Executor call and adapter detail |
| --- | --- |
| `prepare` | Checks `program.digest() == expected_program`, rejects an existing session or run, takes the backend, and calls `WorkerExecutionSession::prepare(run, bundle, projection.clone(), backend, watchdog)`. On success stores session and run. On preparation failure restores the backend and maps primary plus cleanup errors. |
| `begin_init_image` | Calls `session.begin_init_image(current_run, device, bytes, digest)`. |
| `begin_init_chunk` | Rejects an existing pending chunk, calls `write_init_chunk`, and records the returned copied length and device in `pending_init_chunk`. |
| `poll_init_chunk` | Requires the matching pending device, clears the pending marker, and returns `Complete { bytes: copied }`. The executor image copy itself is synchronous; this poll supplies the remote protocol's completion boundary. |
| `finish_init_image` | Rejects a pending chunk, then calls `submit_init_image`. |
| `poll_init_image` | Calls `poll_init_image` and maps `ExternalTransferPoll` to `DriverTransferPoll`. |
| `submit_task` | Requires an active loop or exit phase and calls `submit_task(current_run, phase, task)`. |
| `poll_task` | Calls `poll_task`; `Pending` maps directly, and an executor `F32` or `I32` metric maps to the corresponding `RemoteMetricValue`. |
| `begin_receive_user_data` / `poll_receive_user_data` | Calls `begin_external_ingress` and `poll_external_ingress`. |
| `begin_produce_user_data` / `poll_produce_user_data` | Calls `begin_external_egress` and `poll_external_egress`; the destination remains executor-owned scratch until the poll and remote acknowledgment complete. |
| `user_data_acked` | Calls `acknowledge_external_egress`. |
| `cancel` | Ignores the numeric reason at this adapter boundary and calls `session.cancel(current_run)`. The remote protocol still preserves and transports the reason code. |
| `begin_exit` | Calls `session.begin_exit(current_run)`. |
| `release_arena` | Calls `session.release_arena(current_run, device)`. |
| `finish` | Calls `session.finish(current_run)`, then `recover_finished_session`. |
| `cleanup_after_fault` | Calls `fatal_cleanup` when a session exists, then recovers the backend. A missing session is already clean. Cleanup and recovery errors are returned, while the primary argument is intentionally not re-encoded here because `session.rs` combines the primary and cleanup faults. |

`recover_finished_session` takes the session and calls its `into_parts`. It
restores the session if the executor still owns resources or is in a
nonterminal lifecycle. On success it returns the backend to `backend`, clears
`run` and `pending_init_chunk`, and makes the adapter reusable for a later
strictly increasing remote run. `Drop` performs best-effort
`fatal_cleanup` when a session is still present, so dropping an active adapter
does not silently leave native work running.

### Executor fault encoding

All `WorkerExecutionError` values from active adapter operations map to a
stable `DriverFault` code and a compact `detail` value. The operation code is
the reserved executor code (`EXECUTOR_PREPARE_FAILED`,
`EXECUTOR_OPERATION_FAILED`, or `EXECUTOR_CLEANUP_FAILED`); the detail is:

- `Projection` and `BundleMismatch`: `1`.
- `InvalidRun`: the run ID; `RunMismatch`: the actual run ID.
- Unknown, duplicate, inactive, metric-contract, and watchdog task errors: the
  task ID.
- Unknown device, init digest mismatch, and already-released arena: the device
  ID.
- Wrong role, incomplete dependency, schedule conflict, and device readback
  fault: the task ID.
- Wrong phase and byte-count mismatch: the optional task ID, or `0` when no
  task is attached.
- Incomplete phase: the task ID; init offset mismatch: the device ID.
- Invalid lifecycle: a closed lifecycle code, `Prepared=1`, `Init=2`,
  `Loop=3`, `Exit=4`, `Cancelling=5`, `Finished=6`, `Failed=7`.
- Backend errors: a physical operation code, `BindProjection=1`,
  `PrepareLocal=2`, `PrepareExternal=3`, `AllocateArena=4`, `SubmitLocal=5`,
  `PollLocal=6`, `SubmitIngress=7`, `SubmitEgress=8`, `PollExternal=9`,
  `AcknowledgeEgress=10`, `Quiesce=11`, `ReleaseArena=12`,
  `DestroyResources=13`.
- Journal failure: `u64::MAX - 1`; capacity overflow: `u64::MAX`;
  unclassified errors: `u64::MAX - 2`.

For `PROGRAM_IDENTITY_MISMATCH`, `prepare` reports the first eight digest
bytes of the received program as a little-endian `u64`. This is a compact
diagnostic and not a replacement for the full digest proof performed during
construction and handshake.

## End-to-end role

For one real run, the caller provisions identical `ProvisionedProgram` values
and measured endpoint identities on both peers, constructs the connected
`RuntimeChannel`, creates a `WorkerHandshake` with a native driver, and drives
both typestate machines by repeatedly calling `progress`:

```text
MasterHandshake <-> WorkerHandshake
  Hello, Manifest, ManifestAck, Prepare, PrepareAck
MasterInit      <-> WorkerInit
  InitBegin, scheduled InitChunk*, InitEnd, InitAck, InitComplete, InitCompleteAck
MasterRun       <-> WorkerRun
  Execute / TaskComplete or TaskFailed / Metric
  UserData / DataAccepted / TaskComplete
  DataRequest / UserData / DataAck
MasterExit      <-> WorkerExit
  BeginExit, ExitReady, Execute and data commands, Release / ReleaseAck,
  ExitComplete, ExitAck
```

At every point the master commands only IDs and bounded payloads already
present in the finalized plan. The worker driver executes those IDs against
the local prepared projection, returns nonblocking progress, and never
invents a task, route, schedule, arena size, or artifact. The session owns all
wire sequencing and acknowledgment state; the driver owns only the native
execution effects needed to make the resulting authoritative state true.
