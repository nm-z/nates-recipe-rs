# Remote session state machine

`recipe-remote` is the cooperative, bounded protocol that runs one already
provisioned Recipe master against one already provisioned worker. The session
module owns the protocol state and the wire messages. It does not create the
socket, discover hardware, compile a kernel, read an artifact, or perform file
I/O. The caller supplies an already connected
[`RuntimeChannel`](../../../transport/src/runtime.rs), the same
[`ProvisionedProgram`](../../src/model.rs#L387), matching endpoint identities
and limits, and a worker-side [`WorkerDriver`](../../src/driver.rs#L67).

The public typestates make the only successful lifecycle explicit:

```text
MasterHandshake / WorkerHandshake
            |
            v
MasterInit / WorkerInit
            |
            v
MasterRun / WorkerRun
            |                \
            v                 +--> MasterCancelling / WorkerCancelled
MasterExit / WorkerExit                    |
            |                              |
            +------------------------------+
                           v
                 MasterComplete / WorkerComplete
```

Every `progress` method is nonblocking and consumes the current typestate,
returning `Advance::Pending` with the same state or `Advance::Ready` with the
next state. The caller repeatedly drives one master and one worker until both
ends reach a terminal type. There is no background session thread.

## Boundary and shared state

`RemoteChannel` wraps a `RuntimeChannel` and stores the last completed
`RunId`. Calling `RemoteChannel::new` starts with no prior run. Converting a
completed session back with `MasterComplete::into_channel` or
`WorkerComplete::into_parts` preserves the transport and the last run, so a
reused connection must use a strictly larger nonzero run ID
([`RemoteChannel`](../../src/session.rs#L15), [`SessionCore::new`](../../src/session.rs#L423)).

`SessionCore` is embedded in every typestate. Construction performs these
checks before any frame is sent:

1. The run ID is nonzero.
2. The program manifest is canonical and its digest matches its contents.
3. Init-chunk schedule positions do not overlap the static master-to-worker
   transfer schedule.
4. The connected channel identity exactly equals the supplied local and remote
   `RemoteIdentity`.
5. A reused channel's run ID is strictly increasing.

It then captures the channel's next outbound and inbound global user-data
schedule positions, allocates one message scratch buffer of
`limits.max_message_bytes()`, and resets the session payload sequence counters
to zero for three lanes. `SessionCore` keeps separate payload sequence numbers
for control, metrics, and user data, while the underlying transport keeps its
own connection-global frame sequence.

`SessionCore::progress_transport` calls `RuntimeChannel::progress`. A
completed outbound token is immediately released back to the transport. A
transport error or failure to release a completion poisons the session. A
successful submit can still return `false` from `try_send` when the fixed lane
has no free slot; that is ordinary `Backpressured` behavior, not a protocol
failure. Other transport errors poison the session. Once poisoned,
`ensure_healthy` rejects all further work with `RemoteError::Poisoned`.

The channel's own contract is part of this boundary. It is a nonblocking
`TcpStream` wrapper with preallocated control, metrics, and user-data lanes and
one receive buffer. It validates the outer frame identity, global sequence,
payload digest, lane kind, payload bound, and strictly increasing connection
schedule before exposing a `ReceivedFrame`. The session must release that one
received token before the transport can receive another frame. The transport
writer services queued control first, then user data, then metrics.

## Wire codec and lane mapping

The session codec is manual and allocation-free after construction. Every
payload is:

```text
8-byte magic (RCPREM01)
u16 protocol version (2)
u8 message tag
u8 reserved byte (must be zero)
u64 per-lane payload sequence
u64 nonzero RunId
message fields
```

`Message::lane` maps `InitChunk` and `UserData` to the user-data lane,
`Metric` to the metrics lane, and every other message to control
([`Message`](../../src/codec.rs#L111), [`Message::lane`](../../src/codec.rs#L183)).
The codec carries 27 tags: hello, manifest and its acknowledgment, prepare and
its acknowledgment, init begin/chunk/end/ack/complete/ack, execute, task
complete/failure, data request/user data/data acknowledgment, metric,
cancellation and its acknowledgment, exit begin/ready/release/release
ack/complete/ack, and terminal driver fault.

Encoding writes little-endian fixed-width fields into `SessionCore::scratch`.
Variable payload lengths are checked against `u32`; writer range checks turn a
short scratch buffer into `CapacityExhausted` or `Codec`. Decoding rejects a
payload over `max_message_bytes`, bad magic/version/reserved byte, unknown role
or tag, a count or length that does not fit the host, an artifact count over the
configured manifest bound, a truncated field, and trailing bytes
([`encode`](../../src/codec.rs#L223), [`decode`](../../src/codec.rs#L387)).

After decoding, `SessionCore::received` adds session-level checks. The payload
run must equal the active run, the message lane must equal the outer frame
kind, and the payload sequence must be exactly the next expected value for
that lane. Wrong-run, wrong-lane, replayed, or out-of-order traffic poisons the
session. An outer user-data schedule is translated by subtracting the run's
inbound base; an underflow is a protocol error. The received frame token is
retained until the state handler has validated and consumed the message, then
`release_received` returns it to the channel.

Only a user-data message may carry an outer schedule. Session logical schedules
are added to the outbound base before submission. Thus init chunks and static
cross transfers can start at logical position zero for each run while the
transport still sees one strictly increasing connection-global schedule.

The protocol-level request and response pairs are fixed. Transport completion
tokens acknowledge bytes moving through the channel; the messages below
acknowledge the domain transition after the receiving state machine has checked
the static contract.

| Phase | Master request or report | Worker response or report | Completion condition |
| --- | --- | --- | --- |
| Handshake | `Hello`, `Manifest`, `Prepare` | `Hello`, `ManifestAck`, `PrepareAck` | Both endpoint proofs and the worker's `prepare` succeed. |
| Init | `InitBegin`, ordered `InitChunk`, `InitEnd` | `InitAck` | The worker's image digest and driver admission complete. |
| Init barrier | `InitComplete` | `InitCompleteAck` | Every worker image is complete. |
| Driver task | `Execute` | Optional `Metric`, then `TaskComplete` or `TaskFailed` | The worker driver polls the task to a terminal result. |
| Master-to-worker transfer | Scheduled `UserData` | `TaskComplete` | The worker driver receives the exact bytes. |
| Worker-to-master transfer | `DataRequest` | Scheduled `UserData`, then `DataAck` | The master stores and acknowledges the payload; the worker then marks it complete. |
| Normal exit | `BeginExit`, ordered `Release`, `ExitComplete` | `ExitReady`, matching `ReleaseAck`, `ExitAck` | Exit tasks, every arena release, and `finish` complete. |
| Cancellation | `Cancel` | Matching `ReleaseAck` values, then `CancelAck` | The worker quiesces, releases all arenas, and finishes. |
| Any phase | none | `DriverFault` | Worker cleanup has completed, or the cleanup failure is encoded in the fault. |

## Provisioned program and proof data

`ProvisionedProgram::from_bundle` is the model-side caller of the session. It
commits the exact subset of the finalized bundle visible to one worker:

- the manifest's bundle, draft, realization, artifact identities, and digest;
- canonical, nonzero worker device IDs and each finalized init image size;
- every non-init task assigned to the worker, marked `Driver` or
  `CrossTransfer` and sorted by task ID;
- every one-hop cross-machine transfer, its direction, static schedule, byte
  count, and measured link capacity claim; and
- the set of half-duplex resources used by those transfers.

Calculation and metric tasks on worker devices become driver tasks. A transfer
whose endpoints are both on the worker is a driver task; a transfer whose
endpoints are both on the master remains master-local. A master-to-worker or
worker-to-master device transfer becomes a cross transfer. The external
master-to-worker init admission is represented by the one image per worker
device, not by a runtime task. Init-phase cross transfers or worker work that
cannot be represented by that image are rejected.

Schedules are derived from finalized task windows and sorted by direction,
window start, then task ID. Master-to-worker logical positions begin after all
init chunks. Worker-to-master positions begin at zero in their own direction.
Transfer byte counts must fit the user-data codec payload
(`max_message_bytes - 40`); init chunks use
`max_message_bytes - 48`. IDs, claims, schedules, counts, and all configured
manifest and slot capacities are bounded and checked before construction
([`ProvisionedProgram::from_bundle`](../../src/model.rs#L397),
[`schedule_cross_transfers`](../../src/model.rs#L640),
[`init_chunk_count`](../../src/model.rs#L697)).

The program digest hashes the manifest digest, devices and image sizes, sorted
task IDs/phases/kinds, and complete transfer contracts. The worker therefore
receives a value that is both a runtime lookup table and a proof of the static
plan it is allowed to execute. The worker handshake independently recomputes
the manifest proof and compares the program digest before preparation.

## Handshake: symmetric proof, master-controlled prepare

### Master side

`MasterHandshake::new` constructs the shared core and fixed master storage.
`MasterHandshake::progress` first progresses the transport and then consumes at
most one received frame. Its flags enforce this exact order:

```text
send Hello(master)
receive Hello(worker), validate role/capabilities/identities/limits
send Manifest(bundle/draft/realization/artifacts/program digest)
receive ManifestAck with the exact two digests
send Prepare
receive PrepareAck
```

The master sends hello as soon as the control lane accepts it. It sends the
manifest only after the worker hello has been accepted, and sends `Prepare` only
after the exact manifest acknowledgment. A valid `PrepareAck` is released and
transitions to `MasterInit`. A `DriverFault` becomes `RemoteError::Driver`.
Every duplicate, unexpected, or out-of-order message is a protocol error and
poisons the core ([`MasterHandshake`](../../src/session.rs#L734)).

### Worker side

`WorkerHandshake::new` stores the supplied driver but does not call it. Its
`progress` method sends `Hello(worker)` first, then accepts only the master's
hello and the manifest proof in that order. `validate_hello` requires the
opposite peer role, all six required capabilities, exact endpoint/profile
digests, and an identical eight-word `RemoteLimits` tuple. The manifest proof
checks all plan digests, artifact count, canonical strictly increasing artifact
IDs, and every nonzero artifact digest without allocating a manifest.

After a valid manifest, the worker sends `ManifestAck`. On `Prepare`, it calls
`WorkerDriver::prepare(run, program)`. A successful preparation sets
`prepared`; a driver failure first runs the driver's fatal cleanup path and
queues exactly one terminal `DriverFault`. Once prepared, the worker queues
`PrepareAck` and transitions to `WorkerInit` only after that frame is accepted.
While a terminal fault is being reported, no new incoming message is processed:
the report is sent, its transmit token is observed as flushed, and only then is
`RemoteError::Driver` returned. This gives the peer a deterministic terminal
fault boundary ([`WorkerHandshake`](../../src/session.rs#L816),
[`begin_driver_fault`](../../src/session.rs#L83)).

## Init: one serialized, checked image per worker device

`MasterInit` and `WorkerInit` each track every worker device as `Needed`,
`Active`, or `Complete`. At most one logical image is active on either side.
The caller queues an image with `MasterInit::queue_image`; the device must be
provisioned and still `Needed`, and the byte length must equal the finalized
arena image size. The master computes a SHA-256 digest before sending.

For one active image, master progress follows `Begin -> Chunks -> End ->
WaitAck`:

1. Send `InitBegin(device, bytes, digest)` on control.
2. Send one `InitChunk` at a time on user data. Each chunk is at most the
   scratch capacity minus `INIT_CHUNK_OVERHEAD`; its logical schedule is the
   next init position. The master waits for that transmit token to flush before
   advancing the offset and schedule.
3. Send `InitEnd(device, total bytes)` on control and wait for `InitAck`.
4. Mark that device complete and drop the active image.

When all device states are complete, the master sends `InitComplete` and waits
for `InitCompleteAck` before entering `MasterRun`. `InitAck` for a different
device or phase, an early completion, a driver fault, or any other message is a
protocol error ([`MasterInit`](../../src/session.rs#L955)).

The worker accepts one `InitBegin` only when no image, chunk, or acknowledgment
is pending and the device is still `Needed` with the exact expected size. It
calls `begin_init_image`, starts a SHA-256 accumulator, and marks the image
active. Each chunk must have the exact next logical schedule, active device, and
active offset; its range must not overflow or exceed the image and it must fit
the worker's chunk buffer. The driver receives a stable copied slice through
`begin_init_chunk`, and `progress_chunk` polls until the exact byte count is
reported, updates the hash and offset, zeroes the chunk buffer, and advances the
expected schedule.

`InitEnd` is accepted only after the complete image has been received. The
worker verifies total bytes and the SHA-256 digest, calls `finish_init_image`,
and polls the final admission. On exact completion it marks the device
complete, queues one `InitAck`, and only then accepts the next image. `InitComplete`
is accepted only when no image or poll is pending and every device is complete;
the worker replies with `InitCompleteAck` and transitions to `WorkerRun`. Any
digest, size, order, overlap, or phase mismatch is a protocol/manifest error;
driver errors enter the terminal cleanup report path
([`WorkerInit`](../../src/session.rs#L1176)).

## Run: task, transfer, metric, and capacity state

The master and worker each copy the provisioned tasks and transfers into fixed
storage. A master task is `Idle`, `Active`, `ReceivedAwaitAck`, `Complete`, or
`Failed`. The worker additionally has a fixed array of task slots containing
`Free`, `Running`, `Complete` (with an optional metric send state), or `Failed`.
Half-duplex claims are represented by one token per measured resource on each
peer. A transfer acquires every half-duplex token before submission and releases
all of them exactly when the transfer reaches its protocol completion.

### Master operations

`MasterRun::submit_task` and the corresponding exit method accept only a
provisioned `Driver` task in the requested phase while `Idle`; they queue
`Execute` on control and then mark it `Active`.

`send_user_data` accepts only an exact-size master-to-worker cross transfer. It
acquires half-duplex tokens, queues scheduled `UserData`, marks the task
active, and rolls the token acquisition back if the fixed user-data lane is
backpressured. `request_user_data` accepts only an exact worker-to-master
transfer. Before queueing, the master rejects another request while a
worker-to-master task is `Active`, reflecting the worker's one outgoing buffer;
it acquires the transfer tokens and queues `DataRequest` on control. A payload
that has already arrived stays `ReceivedAwaitAck` until a later progress call
queues `DataAck`; the worker still rejects a new request while that outgoing
payload is awaiting its acknowledgment.

`MasterRuntime::progress` first progresses transport. If a worker-to-master
payload is waiting for acknowledgment, it tries to queue `DataAck`; once queued,
it marks the task complete, releases its capacity tokens, and emits
`TaskComplete`. Otherwise it consumes at most one received message:

- `TaskComplete` marks an active task complete and releases cross-transfer
  tokens.
- `TaskFailed` marks an active task failed and releases cross-transfer tokens;
  this is a nonterminal task event, not a session fault.
- `UserData` validates direction, static schedule, exact size, phase, and
  active status, copies it into the first free master data slot, marks the task
  `ReceivedAwaitAck`, and emits `DataReady`. Only one data acknowledgment may
  be pending.
- `Metric` validates an active driver task, stores its scalar in the first free
  metric slot, and emits `MetricReady`.
- `DriverFault` becomes a terminal `RemoteError::Driver`.

The caller reads a data slot with `data` and must call `release_data`; a metric
slot is consumed with `take_metric`. A full data or metric inbox returns
`Backpressured`, preserving fixed memory. `phase_complete` requires every task
in that phase to be `Complete`, not merely reported.

`MasterRun::begin_exit` is allowed only after all loop tasks are complete. The
caller can instead consume the run into `MasterCancelling` at any time.

### Worker operations

`WorkerRun::progress` progresses transport, reports any pending terminal driver
fault, then advances local work before consuming a new command. Incoming
master-to-worker data is polled first. Local running task slots are polled in a
round-robin order. Completed local slots send a metric first, if present, and
wait for its transmit token to flush before sending `TaskComplete`; failed slots
send `TaskFailed`. A completed inbound cross transfer sends `TaskComplete` only
after its driver receive poll has reported the exact byte count. One outgoing
worker-to-master transfer is produced and sent only after its driver poll
completes.

The worker then consumes at most one control or user-data message. The handler
enforces the current phase and static transfer contract:

- `Execute` requires an idle driver task and a free fixed task slot, then calls
  `WorkerDriver::submit_task`.
- `UserData` requires an idle master-to-worker transfer, an empty incoming
  slot, an exact schedule and byte count, and available half-duplex tokens. It
  copies the payload into stable incoming storage and calls
  `begin_receive_user_data`.
- `DataRequest` requires an idle worker-to-master transfer and an empty outgoing
  slot. It reserves the exact static length in `data_scratch` and calls
  `begin_produce_user_data`.
- `DataAck` requires the matching outgoing payload in `WaitAck`, calls
  `user_data_acked`, zeroes the outgoing buffer, marks the task complete, and
  releases its tokens.

Driver errors from submit, poll, data begin/poll/ack, or message handling are
converted into the driver's one-shot cleanup report. Other violations poison
the session. Worker events are `TaskAccepted`, `TaskReported`, `DataAccepted`,
and `DataAcknowledged`; a progress call may return no event while transport or
native work remains pending.

## Normal exit and exact resource release

`MasterExit` has six pre-terminal states plus `Done`:
`NeedBegin`, `WaitReady`, `Active`, `Releasing`, `NeedComplete`,
`WaitComplete`, and `Done`.

1. `NeedBegin` queues `BeginExit`; `WaitReady` accepts only `ExitReady`.
2. `Active` runs the same task and transfer machinery with `RunPhase::Exit`.
   Once every exit task is complete and no data acknowledgment is pending, it
   enters `Releasing`.
3. `Releasing` queues one `Release(device)` at a time. Each device must move
   `Needed -> Requested -> Complete` through the matching `ReleaseAck`.
4. `NeedComplete` queues `ExitComplete`; `WaitComplete` accepts only `ExitAck`.
   `Done` permits `into_complete`, yielding `MasterComplete` and a reusable
   `RemoteChannel`.

The worker enters `WorkerExit` only after `BeginExit` and a complete loop. Its
first progress call queues `ExitReady`. It continues exit-phase task work and
sends each requested release acknowledgment. A `Release` is legal only after
all exit tasks complete and only once per known device; the worker calls
`release_arena`, then marks that device `Requested` and sends `ReleaseAck`.
`ExitComplete` is accepted only after all exit tasks and all exact releases are
complete. The worker then calls `finish`, queues tracked `ExitAck`, waits for
that token to flush, and returns `WorkerComplete`.

`finish` is the point at which the driver destroys worker resources. The
executor-backed driver recovers its backend only after the underlying
`WorkerExecutionSession` reaches `Finished`; a drop of an unfinished concrete
driver invokes fatal cleanup as a last resource-safety boundary.

## Cancellation and terminal faults

`MasterCancelling::progress` queues one `Cancel(reason)`. Until that command is
accepted, no response is valid. Afterwards, late task completions, failures,
metrics, user data, data acknowledgments, and `ExitReady` are accepted and
released while the worker quiesces. The master records each worker
`ReleaseAck`; once every device is complete, it accepts `CancelAck` and returns
`MasterComplete { cancelled: true }`. A duplicate release acknowledgment, an
unexpected message, or a driver fault is terminal.

On the worker, `Cancel` calls `WorkerDriver::cancel` and enters
`WorkerCancelled`. The worker releases every extant arena exactly once, sends a
tracked `CancelAck` only after all release calls have succeeded and their
acknowledgments have flushed, calls `finish`, then returns
`WorkerComplete { cancelled: true }`. Cancellation uses the same exact
per-arena acknowledgment discipline as normal exit, but the driver lifecycle
is `Cancelling` rather than `Exit`.

The terminal driver-fault path is separate from cancellation. Any worker driver
failure first calls `cleanup_after_fault(primary)`. If cleanup fails, the
reported fault is replaced by `FATAL_CLEANUP_FAILED` carrying both codes. Only
after cleanup succeeds does the worker queue one `DriverFault`, wait for its
transmit completion, and return `RemoteError::Driver`. This protocol has no
per-arena release acknowledgment after a fault because the driver contract
already requires native quiescence and complete local cleanup. The master sees
that frame in handshake, init, run, exit, or release states and fails the
session with the same `RemoteError::Driver`.

## Driver and executor boundaries

`WorkerDriver` is the only session-to-native boundary. It receives finalized
run, device, task, digest, and bounded byte-slice contracts, and exposes only
nonblocking prepare, image admission, task submit/poll, ingress/egress
begin/poll/ack, cancellation, exit, release, finish, and fatal cleanup. There
is no host calculation callback, arbitrary closure, compiler hook, discovery
hook, file path, or vendor-math API in this trait
([`WorkerDriver`](../../src/driver.rs#L62)).

`ExecutorWorkerDriver` is the concrete adapter in this workspace. Construction
derives a `WorkerProjection` and proves the wire program has the same worker
devices, image sizes, non-init task roles, and one-hop cross-transfer claims.
`prepare` validates the program digest and creates a prepared
`WorkerExecutionSession`; init, task, data, exit, release, and finish methods
forward to the matching executor method and map rich executor errors into the
allocation-free `DriverFault { code, detail }`. Its `cancel` method receives
the protocol reason but deliberately calls the executor's run-scoped cancel
without passing that numeric reason. A successful `finish` extracts the
backend, journal-free for the remote caller, and clears the active run so the
driver can be reused. The adapter's `Drop` invokes executor fatal cleanup if a
session remains active
([`ExecutorWorkerDriver`](../../src/executor_driver.rs#L56)).

## Invariants and failure surface

The session deliberately fails closed. The key invariants are:

- one nonzero run ID per session, strictly increasing on a reused channel;
- exact endpoint/profile identities, protocol version, required capabilities,
  fixed limits, manifest proof, and program digest on both peers;
- exact per-lane payload sequence and outer transport sequence;
- user-data schedules strictly increasing globally at the transport and
  strictly increasing per direction in the provisioned program;
- no probe frame, wrong lane, wrong phase, unknown task/device, duplicate task,
  duplicate image, duplicate release, overlap, or trailing wire bytes;
- every transfer's exact static direction, schedule, byte count, and measured
  half-duplex claims;
- fixed task, data, metric, and transport slots with explicit backpressure;
- one active init image, one pending init chunk/image admission, one pending
  master data acknowledgment, one worker incoming transfer, and one worker
  outgoing transfer;
- every received transport token is released exactly once, and every submitted
  tracked terminal or metric frame is allowed to flush before the state advances;
- every worker arena is released exactly once before `finish` or terminal
  acknowledgment.

`RemoteError` distinguishes invalid construction, manifest mismatch, protocol
violation, run mismatch, unknown or duplicate IDs, capacity exhaustion,
backpressure, codec rejection, driver fault, transport failure, and poisoned
state ([`RemoteError`](../../src/error.rs#L7)). Capacity and backpressure are
recoverable only by the caller changing the outstanding work. A protocol,
codec, run, transport, or driver-cleanup failure leaves the core unusable.

The complete end-to-end role is therefore narrow and concrete: callers build a
profile-derived static `ProvisionedProgram`, establish and identify a
`RuntimeChannel`, drive the paired typestates, supply image bytes and task/data
commands at the master boundary, and receive only validated events and fixed
slots. The session proves and enforces the plan while the worker driver owns
all native execution and resource lifetime.
