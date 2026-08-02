# Transport runtime

[`transport/src/runtime.rs`](../../src/runtime.rs) is Recipe's fixed-capacity,
nonblocking runtime channel. It owns one already connected
[`std::net::TcpStream`], turns application payloads into authenticated transport
frames, advances at most one transmit step and one receive step per poll, and
keeps all transmit and receive storage alive until the caller releases it. It
does not open a listener, resolve an address, establish a connection, discover
a peer, negotiate security, or choose a route. Those facts are supplied by the
caller as a connected stream, a [`SessionIdentity`](../../src/protocol.rs#L42),
protocol limits, and fixed lane capacities.

The crate has two intentionally separate framed paths:

| Path | Implementation | Stream mode | Frame kinds | Consumer |
| --- | --- | --- | --- | --- |
| Runtime execution | [`RuntimeChannel`](../../src/runtime.rs#L255-L765) | Nonblocking | `Control`, `Metrics`, `UserData` | `recipe-remote` master and worker sessions |
| Peer measurement | [`TcpPeerSession`](../../src/probe.rs#L50-L330) plus [`WireSender`/`WireReceiver`](../../src/protocol.rs#L200-L306) | Blocking with per-operation deadlines | `ProbeBegin`, `ProbeReady`, `ProbeData`, `ProbeComplete` | `recipe-probe` and cluster assembly |

The runtime rejects probe frames as soon as their outer header is decoded. A
probe connection therefore cannot accidentally become an execution channel,
and a runtime channel cannot be used to collect measured link evidence.

## End-to-end role and call graph

The complete production boundary is:

```text
host/GPU discovery and bounded peer probe
    -> measured profile and cluster endpoint identities
    -> caller establishes TcpStream
    -> RuntimeChannel::new
    -> RemoteChannel
    -> MasterHandshake / WorkerHandshake
    -> MasterInit / WorkerInit
    -> MasterRun / WorkerRun
    -> MasterExit or cancellation
    -> MasterComplete / WorkerComplete
    -> RemoteChannel reuse or drop
```

`recipe-remote` is the only workspace crate that calls the runtime API. Its
`RemoteChannel` wraps a `RuntimeChannel` and adds the connection-global
`RunId` epoch. `SessionCore::progress_transport` calls
`RuntimeChannel::progress`, releases every reported transmitted token, and
borrows one received frame until the active typestate consumes it. The session
codec encodes one remote message into its preallocated scratch buffer, submits
it to the matching runtime lane, then tracks the returned
`CompletionToken`. No listener or socket constructor exists in either
`recipe-transport` or `recipe-remote`.

The identity path starts before transport construction. A per-machine measured
profile is converted by
[`MemberProfileIdentity::derive`](../../../cluster/src/model.rs#L27-L64) into a
nonzero machine digest (from the retained stable machine fingerprint) and a
profile digest (from canonical measured-profile bytes). Cluster assembly checks
those identities and the directional probe evidence before a
`SessionIdentity` is supplied to a connected transport. The transport then
binds the two endpoint identities into every frame, so a stream cannot be
reused for a different machine or profile pair without failing header
validation.

## Public runtime surface

The public types are deliberately small and correspond directly to the state
held in `runtime.rs`:

| Type | Role | Validity rule |
| --- | --- | --- |
| `RuntimeLane` | Selects the control, metrics, or user-data lane | Maps one-to-one to `FrameKind::Control`, `Metrics`, or `UserData`. |
| `ScheduleStamp` | Finite static position for a user-data frame | `ScheduleStamp::new(u64::MAX)` is rejected because `u64::MAX` is the unscheduled wire sentinel. Zero and every other value are valid. |
| `ChannelCapacities` | Fixed slot count for each lane and one payload bound | All three counts and `max_payload` must be nonzero. |
| `Submission` | Caller capability for one queued transmit | Contains the nonzero completion token and its lane. |
| `CompletionState` | Observation of a retained transmit slot | `Queued`, `Writing`, or `Complete`; a released token is unknown. |
| `Progress` | One poll result | Reports at most one transmitted token, at most one received token, and whether either side made byte or state progress. |
| `RuntimeChannel` | Connected, preallocated, bidirectional transport | Owns the stream, slot tables, receive storage, sequence counters, and connection-global schedules. |

`ChannelCapacities::new` rejects zero slots with
`InvalidConfiguration("every runtime lane requires at least one fixed slot")`
and rejects zero payload capacity with
`InvalidConfiguration("runtime maximum payload must be nonzero")`. It exposes
only `max_payload`; lane counts are fixed construction facts rather than a
second runtime policy surface.

## Construction and owned state

`RuntimeChannel::new(stream, identity, limits, capacities)` first checks that
the runtime payload capacity does not exceed `ProtocolLimits::max_payload()`.
It then sets the supplied stream to nonblocking mode and enables TCP_NODELAY.
Construction allocates, once:

- one `FixedLane` for each of control, metrics, and user data;
- exactly the configured number of `TxSlot` values in each lane, each with a
  `HEADER_BYTES` header array and a payload backing of `max_payload` bytes; and
- one `ReceiveBuffer` with a 200-byte header array and a
  `max_payload`-byte payload backing.

`next_token` starts at 1, `next_order`, `tx_sequence`, and `rx_sequence` start
at zero, and both connection-global schedule histories are empty. The channel
starts healthy. All operations after construction, including submission,
polling, completion lookup, and release, reuse these backings and do not heap
allocate.

The private state is the runtime contract:

| State | Contents | Meaning |
| --- | --- | --- |
| `TxSlot::Free` | No metadata, zeroed counters | The lane may accept one submission. |
| `TxSlot::Queued` | Copied payload, SHA-256 digest, metadata, monotonic order | The frame is admitted but not yet selected for writing. |
| `TxSlot::Writing` | Encoded header plus partial header/payload offsets | This is the only active transmit slot. |
| `TxSlot::Complete` | Fully written frame and its token | The local stream accepted every frame byte; no peer-level acknowledgment is implied, and the slot remains occupied until explicit release. |
| `ReceiveBuffer` not ready | Partial header, decoded header, or partial payload | `progress` may continue reading. |
| `ReceiveBuffer` ready | Verified frame and borrowed payload | The caller must process and release the matching token before another frame can be read. |

`SlotKey` identifies a lane and slot index. Token lookup searches all three
lanes, so completion tokens are connection-wide even though capacity is
lane-local. The monotonic token and submission-order counters prevent token or
order reuse until their respective `u64` spaces are exhausted.

## Outer frame contract

The paired protocol implementation in
[`protocol.rs`](../../src/protocol.rs#L12-L463) owns the 200-byte outer header.
`RuntimeChannel` calls `encode_header` when a queued slot becomes active and
`decode_header` after reading a complete header. The fields are:

```text
0..8     magic: RCPTRN01
8..10    protocol version: 1, big endian
10       frame kind
11       reserved flags, must be zero
12..16   payload length: u32, big endian
16..24   direction-local frame sequence: u64, big endian
24..32   nonzero completion token: u64, big endian
32..40   schedule position or u64::MAX for no schedule
40..72   sender machine digest
72..104  sender profile digest
104..136 receiver machine digest
136..168 receiver profile digest
168..200 SHA-256 payload digest
```

Header encoding rechecks the payload length and `FrameMetadata` schedule rule.
Header decoding checks magic, version, reserved flags, known frame kind, the
configured payload bound, the expected direction-local sequence, nonzero
token, schedule legality, and the exact reverse identity pair. A mismatch is
reported before payload bytes are accepted. Once the complete payload arrives,
`finish_receive` computes the same SHA-256 digest and rejects a mismatch as
`IntegrityFailure`.

`EndpointIdentity::new` rejects zero machine or profile digests. `SessionIdentity::new`
rejects equal machine digests, because the two endpoints must be distinct, and
`reversed()` supplies the peer's exact view. These checks happen before a
channel is normally constructed; the header checks enforce them on every frame.

`ProtocolLimits::new` accepts payloads from 1 byte through 64 MiB and requires a
nonzero operation timeout. The timeout is used by the blocking wire and probe
APIs. `RuntimeChannel` itself has no deadline or sleep path: it reports
`WouldBlock` as no progress and relies on its caller to poll again.

## Submission and transmit ordering

The three public submission methods are direct lane selectors:

| Method | Lane | Schedule |
| --- | --- | --- |
| `submit_control(payload)` | `Control` | None |
| `submit_metrics(payload)` | `Metrics` | None |
| `submit_user_data(stamp, payload)` | `UserData` | Required finite `ScheduleStamp` |

`submit_user_data` first requires a stamp strictly greater than
`last_submitted_schedule`. The private `submit` path then:

1. rejects a poisoned channel;
2. checks `payload.len()` against the receive backing, which is the runtime
   payload capacity;
3. allocates the next nonzero `CompletionToken` and validates
   `FrameMetadata`;
4. finds the first `Free` slot in the selected fixed lane, copies the payload,
   computes its digest, records the metadata and `next_order`, and marks the
   slot `Queued`;
5. advances the token and order counters with checked arithmetic; and
6. returns `Submission { token, lane }`.

No submission is silently dropped or replaced. A full selected lane returns
`CapacityExhausted("control lane")`, `CapacityExhausted("metrics lane")`, or
`CapacityExhausted("user-data lane")`. An over-bound payload returns
`FrameTooLarge { declared, limit }`. A failed user-data admission does not
advance `last_submitted_schedule`, so the caller can retry the same logical
position after freeing capacity. Counter exhaustion is a
`ProtocolState` error, not a wraparound.

Only one slot is actively written at a time. `next_queued` applies strict lane
priority, selecting the oldest queued control slot first, then the oldest
queued user-data slot, then the oldest queued metrics slot. `order` provides
FIFO ordering within each lane, but a newer control submission can precede an
older user-data or metrics submission by design. A completed slot does not
block another queued slot from becoming active; only its unreleased storage
remains occupied.

`progress_write` encodes the header at activation, marks the slot `Writing`,
then `write_active` performs one nonblocking write operation per call. Header
bytes are written before payload bytes. Partial writes advance the corresponding
offset. A zero-byte write is `ConnectionClosed`. `WouldBlock` and `Interrupted`
return an unadvanced activity result. Once all bytes are written,
`finish_active` marks the slot `Complete`, clears `active`, increments the
direction's frame sequence with checked arithmetic, and reports its token in
`Progress::transmitted`.

The caller must call `completion_state(token)` and eventually
`release_completion(token)`. Release is valid only for `Complete`; releasing a
queued or writing slot returns
`ProtocolState("transmit completion cannot be released before it is complete")`.
Release resets and zeroes the slot, including only the payload prefix that was
used, and returns its fixed capacity to that lane. A released token is no
longer discoverable and produces `UnknownCompletion`.

## Receive progression and backpressure

`progress_read` is the receive half of the same poll. It never reads past the
one receive buffer:

1. If a frame is already ready, it returns that frame's token again with
   `advanced = false`; the caller owns the pending storage until release.
2. Otherwise it reads the remaining header bytes. A partial read returns
   `advanced = true`; `WouldBlock` or `Interrupted` returns no activity.
3. On a complete header it calls `decode_header` with `rx_sequence`, rejects
   any probe kind, and checks that a user-data schedule is strictly greater
   than `last_received_schedule`.
4. It reads exactly the declared payload length into the fixed backing. An
   empty payload skips directly to completion.
5. `finish_receive` verifies the digest, records the received schedule,
   marks the buffer ready, increments `rx_sequence`, and reports the received
   completion token.

`received()` returns `None` until the buffer is ready. Once ready it returns a
`ReceivedFrame` borrowing the receive backing, with copied `FrameMetadata` and
the exact payload slice. `release_received(token)` requires a ready frame and
the exact token, then resets and zeroes the buffer. Calling it with no pending
frame or a different token returns a `ProtocolState` error. Until release,
another frame cannot be admitted, so an application that does not drain its
frame is intentionally backpressured at one receive buffer.

The public `progress` method first invokes `progress_write`, then
`progress_read`, in that order. It combines their activities into one
`Progress` value. `advanced` is true for any successful byte or state advance,
including activating a slot, completing a frame, reading a header, or reading
payload bytes. It is false when both directions are idle, and it remains false
when a ready received frame is merely reported again. At most one transmit and
one receive token can be reported by one call.

## Poisoning and transport errors

`progress` marks the channel `poisoned` if either half returns an error and
returns that original error. Future submissions and future `progress` calls
fail immediately with `Poisoned`; there is no retry, alternate stream, or
silent reset. Completion lookup, `received()`, and release methods do not call
`ensure_healthy`, so already retained storage can still be inspected and
released during caller teardown after a transport failure.

The runtime can expose these error classes:

| Source | Errors and meaning |
| --- | --- |
| Construction and value checks | `InvalidConfiguration`, `InvalidIdentity`, `Io` |
| Frame admission | `FrameTooLarge`, `CapacityExhausted`, `InvalidFrame`, `UnknownCompletion`, `ProtocolState`, `Poisoned` |
| Header validation | `InvalidFrame`, `UnsupportedVersion`, `UnexpectedSequence`, `UnexpectedIdentity`, `FrameTooLarge`, `ProtocolState` |
| Payload validation | `IntegrityFailure`, `ConnectionClosed`, `Io`, `ProtocolState` |
| Completion and receive release | `UnknownCompletion` or `ProtocolState` for an early, duplicate, or mismatched release |

`TransportError::io` converts a timed-out or would-blocking blocking I/O
operation to `DeadlineExceeded`; the runtime handles nonblocking
`WouldBlock` and `Interrupted` directly as ordinary pending progress. Other
I/O kinds become `Io(kind)`. A zero-byte read or write is always
`ConnectionClosed`. Sequence, token, and schedule counters use checked
arithmetic and report `ProtocolState` exhaustion rather than wrapping.

## Connection-global schedules and reuse

Transport schedules are connection-global, not run-local. Before the first
user-data submission or reception, both `next_outbound_schedule_position()`
and `next_inbound_schedule_position()` return zero. After a stamp is submitted
or received, the next legal value is the previous value plus one. `u64::MAX`
is never legal. Submission schedules advance on admission, while receive
schedules advance only after the full payload and digest have been verified.

`SessionCore::new` captures these two bases when a `RemoteChannel` starts a new
run. The remote protocol can then use logical run-relative schedules while
adding the captured outbound base before submission and subtracting the
captured inbound base after reception. `RemoteChannel` also retains the last
completed `RunId`; a reused connection requires a strictly larger nonzero run
ID. `MasterComplete::into_channel` and `WorkerComplete::into_parts` return the
same channel and epoch, preserving both schedule monotonicity and run
monotonicity across lifecycles.

## Remote master and worker lifecycle

The runtime carries only framed bytes. The paired `recipe-remote` state machine
assigns those frames to a master or worker role. Each progress call is
cooperative and nonblocking. Handshake and worker typestates consume the
current state and return `Advance::Pending(state)` or `Advance::Ready(next)`;
master run and exit retain mutable state so callers can submit work between
polls.

### Handshake

`MasterHandshake::new` and `WorkerHandshake::new` both call
`SessionCore::new`, which validates the run ID, manifest proof, reserved init
schedule range, exact transport identity, and strict increase over a reused
channel. Each side allocates fixed task, data, metric, half-duplex, release,
and codec storage before sending anything.

The symmetric wire exchange is:

```text
master Hello <-> worker Hello
master Manifest -> worker ManifestAck
master Prepare -> worker PrepareAck
```

Each `Hello` is a control-lane remote message containing the role, all required
capability bits, both endpoint machine/profile digests, and the complete fixed
`RemoteLimits` tuple. The peer role, capabilities, identities, and limits must
match. The master sends the manifest only after accepting the worker hello and
sends `Prepare` only after the exact manifest and program digests are
acknowledged. The worker recomputes the manifest proof from its own
`ProvisionedProgram`; it calls `WorkerDriver::prepare` only after accepting the
manifest and `Prepare`. A duplicate or out-of-order message is a remote
protocol error.

Worker driver faults take a terminal path. The worker calls
`cleanup_after_fault`, queues one `DriverFault` control message, waits until
the runtime reports that message transmitted, and only then returns the driver
fault to its caller. A cleanup failure is combined with the primary fault.
There is no second fault report or replacement execution path.

### Init image admission

After `PrepareAck`, the master enters `MasterInit` and the worker enters
`WorkerInit`. Exactly one logical image is active at a time for the worker's
provisioned devices. The master caller must queue an image whose byte length
equals the finalized arena image size. It computes the image SHA-256 digest and
sends:

```text
InitBegin(device, bytes, digest)
InitChunk(device, offset, bytes) ...
InitEnd(device, bytes)
InitAck(device)
```

`InitChunk` uses the user-data lane and a connection-global schedule, with a
maximum chunk payload of `max_message_bytes - INIT_CHUNK_OVERHEAD`. The master
waits for each chunk's transmit completion before admitting the next chunk.
The worker requires the expected device, exact next offset, exact next logical
schedule, and a range within the finalized image. It passes each chunk to
`WorkerDriver`, hashes the bytes after driver completion, verifies the final
digest, and calls `finish_init_image`. Only then does it send `InitAck`.

After every device is acknowledged, the master sends `InitComplete`. The worker
accepts that message only when every device is complete and no image, chunk, or
ack is pending, then sends `InitCompleteAck`. The two sides transition to
`MasterRun` and `WorkerRun` only after that final control frame is accepted.

### Run work and lane use

`MasterRun` exposes `submit_task`, `send_user_data`, `request_user_data`, and
`progress`. `MasterExit` exposes the same work methods for `RunPhase::Exit`
after the worker has acknowledged exit readiness. The master may submit only a
provisioned driver task that is idle in the active phase. It may send only the
exact byte count and static schedule of an idle master-to-worker cross
transfer, or request an idle worker-to-master transfer. Half-duplex transfer
claims acquire one fixed token per finalized resource; a busy token returns
backpressure rather than overlapping work. Full-duplex claims do not share a
token.

The remote message-to-lane map is fixed:

| Runtime lane | Remote messages |
| --- | --- |
| Control | `Hello`, `Manifest`, `ManifestAck`, `Prepare`, `PrepareAck`, `InitBegin`, `InitEnd`, `InitAck`, `InitComplete`, `InitCompleteAck`, `Execute`, `TaskComplete`, `TaskFailed`, `DataRequest`, `DataAck`, `Cancel`, `CancelAck`, `BeginExit`, `ExitReady`, `Release`, `ReleaseAck`, `ExitComplete`, `ExitAck`, `DriverFault` |
| User data | `InitChunk`, `UserData` |
| Metrics | `Metric` |

The remote codec has its own `RCPREM01` payload header, protocol version 2,
per-lane sequence, nonzero `RunId`, and 27 message tags. `SessionCore` encodes
into a fixed scratch buffer, passes the payload to the corresponding runtime
submission method, and increments the per-lane payload sequence only after
admission. On receive it requires the decoded run, message lane, and next
per-lane sequence, translates the outer connection-global schedule back to the
run-relative value, and releases the transport receive token after the active
state handler consumes the message. A malformed, wrong-run, wrong-lane, or
out-of-order remote payload is surfaced as a remote protocol error and normally
poisons `SessionCore`.

Worker progress polls native work through `WorkerDriver`. A completed driver
task may first send a metric on the metrics lane, wait for that metric's
transmit completion, then send `TaskComplete` on control. A worker-to-master
cross transfer produces bytes into bounded scratch storage, sends `UserData` on
user data, waits for `DataAck`, and then releases its transfer token. The
master stores received data in a fixed data slot and emits `DataReady`; the
caller should release that slot to return inbox capacity. The next master
progress call can send `DataAck` and mark the task complete even if the caller
has not released the slot, so retaining the slot still backpressures later
worker-to-master data. Data and metric inboxes therefore provide explicit
application-level backpressure in addition to transport lane capacity.

### Exit and cancellation

`MasterRun::begin_exit` is valid only after every loop task is complete. The
master sends `BeginExit`, waits for `ExitReady`, then enters active exit work.
After exit tasks and pending data acknowledgments complete, it sends one
`Release(device)` at a time. Each worker calls `release_arena` and returns an
exact `ReleaseAck`; duplicate, unknown, or early releases are protocol errors.
Only after every release is acknowledged does the master send `ExitComplete`.
The worker requires all exit tasks and all arena releases to be complete, calls
`WorkerDriver::finish`, sends `ExitAck`, and transitions to `WorkerComplete`
after that token is flushed. The master reaches `MasterComplete` only after
receiving and releasing `ExitAck`.

`MasterRun::cancel` and `MasterExit::cancel` send `Cancel(reason)`. The worker
calls `WorkerDriver::cancel`, releases every arena through the same exact
release acknowledgment states, calls `finish`, sends `CancelAck`, and returns
`WorkerComplete { cancelled: true }`. A cancellation is a terminal protocol
choice, not a retry or a reset of the active run.

## Probe path and measured-profile handoff

`TcpPeerSession::new` is the caller-facing peer probe constructor. It requires
an asynchronous peer descriptor, accepts an already connected stream, calls
`split_stream`, and stores blocking `WireSender` and `WireReceiver` halves in a
mutex. A `next_round` counter starts at one and serializes benchmark attempts
on that connection. The probe path has no fixed runtime lane slots and does
not construct a `RuntimeChannel`.

One controlled benchmark (`TcpPeerSession::benchmark_controlled`) is:

1. Validate a nonzero bounded plan, no more than 1,000,000 iterations, no more
   than 8 GiB total bytes, and a payload that fits `ProtocolLimits`.
2. Exchange 40-byte `ProbeBegin` payloads containing schema 1, duplex mode,
   exact plan, and measured local memory capacity/rate.
3. Exchange empty `ProbeReady` frames.
4. Transfer the patterned payload for the requested iterations. Full-duplex
   links run sender and receiver concurrently behind a two-party barrier.
   Half-duplex links serialize direction in machine-digest order, so both
   endpoints choose the same order.
5. Exchange 16-byte `ProbeComplete` rates and validate both peer rates.

Every probe frame uses a deterministic round/phase/iteration token. The
blocking wire halves validate outer identity, sequence, frame kind, token,
payload bound, and SHA-256 digest. Each operation applies the earliest of the
phase deadline, caller absolute deadline, and protocol operation timeout.
Cancellation is checked between framed operations. Failures retain the active
phase and classify cancellation, deadline, identity, integrity, protocol, or
transport failure; a failed attempt cannot become measured profile data.

`ProbeEngine` invokes the controlled `PeerSession` attempt and accepts only a
`Measured` result. It validates directional evidence, duplex execution mode,
sample counts, elapsed bounds, and derived rates before writing the measured
profile. Cluster assembly then verifies that evidence is bound to the exact
`SessionIdentity` endpoints and uses the resulting machine/profile digests for
future transport construction. Runtime execution starts only after this
measurement and identity pipeline has completed.

## Teardown and ownership

There is no `RuntimeChannel::close` method. A successful remote lifecycle
flushes the terminal acknowledgment protocol before returning a
`MasterComplete` or `WorkerComplete`; those values can return the channel for a
new, strictly higher run. Dropping the returned `RuntimeChannel` or
`RemoteChannel` drops its `TcpStream`, which closes the connection through
normal Rust ownership. Dropping a channel does not invent a control frame,
retry a partial write, or acknowledge outstanding completions.

If runtime polling fails, the channel remains poisoned and the caller must
release any already complete transmit tokens or pending receive token while
tearing down. The runtime has no background thread, event loop, timeout worker,
or cancellation callback. The owner drives every progress call and owns the
decision to stop polling or to complete the remote cancellation/exit protocol.

`WireSender` and `WireReceiver` have a separate poison path for probe failures:
they mark themselves poisoned and call `shutdown(Shutdown::Both)` on their
owned stream half. The cloned stream halves are dropped with the
`TcpPeerSession`; this teardown is independent from `RuntimeChannel` teardown.

## Invariants to preserve at callers

- Supply a connected stream and exact identities; transport never establishes
  or authenticates a peer by address.
- Use `ChannelCapacities` that fit `ProtocolLimits`, and release every complete
  transmit token and every ready receive token.
- Poll the channel until a queued operation is reported transmitted. A queued
  slot is not wire-complete merely because admission succeeded.
- Do not submit user data with `u64::MAX`, and keep outbound and inbound user
  data schedules strictly increasing for the lifetime of the connection.
- Keep probe frames on `TcpPeerSession` and runtime messages on
  `RuntimeChannel`; the two frame families are intentionally disjoint.
- Keep the `RemoteChannel` returned from a completed session when reusing a
  connection, and choose a strictly larger nonzero `RunId` for the next
  session.
- Let `recipe-remote` validate static task, transfer, metric, release, and
  half-duplex state. The transport only provides byte framing and fixed
  capacity; it must not infer domain state or fabricate a fallback message.
- Treat `Poisoned`, identity, sequence, schedule, digest, capacity, and
  connection-closed errors as terminal for the affected path. There is no
  alternate implementation or hidden retry.
