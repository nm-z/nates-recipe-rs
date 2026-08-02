# `recipe-transport`

`recipe-transport` is Recipe's explicit, bounded TCP transport. It supplies
framing, endpoint identity checks, integrity checks, a bounded peer benchmark,
and a preallocated nonblocking runtime channel. It does not discover peers or
create connections. A caller must provide an already connected
[`std::net::TcpStream`] and the exact identities of both endpoints.

The crate root is intentionally small (`transport/src/lib.rs`): it forbids
`unsafe` code, denies missing `Debug` implementations, declares the four
private implementation modules, and re-exports their public types. The public
surface is therefore the following:

| Area | Re-exports | Role |
| --- | --- | --- |
| Errors | `TransportError`, `TransportResult` | One error taxonomy and its `Result` alias |
| Probe adapter | `MeasuredLocalMemory`, `TcpPeerSession` | Implements `recipe_probe::PeerSession` over a connected TCP stream |
| Framing | `CompletionToken`, `EndpointIdentity`, `FrameKind`, `FrameMetadata`, `ProtocolLimits`, `ReceivedFrame`, `SessionIdentity`, `WireReceiver`, `WireSender`, `split_stream` | Blocking framed I/O for probe/control exchanges |
| Runtime | `ChannelCapacities`, `CompletionState`, `Progress`, `RuntimeChannel`, `RuntimeLane`, `ScheduleStamp`, `Submission` | Fixed-capacity nonblocking I/O for remote execution |

## Module and ownership map

* `src/error.rs` owns all public failure values. Every fallible transport
  operation returns `TransportResult<T>`.
* `src/protocol.rs` owns the wire ABI. It validates identities, metadata,
  payload bounds, frame headers, direction-local sequence numbers, SHA-256
  payload digests, and blocking deadlines. `split_stream` creates the
  sender/receiver pair used by the probe adapter.
* `src/probe.rs` owns `TcpPeerSession`. It turns a bounded
  `recipe_probe::BoundedBenchmarkPlan` into a five-phase, mutually validated
  peer benchmark and returns measured `PeerMeasurement` evidence.
* `src/runtime.rs` owns `RuntimeChannel`. It preallocates per-lane transmit
  slots and one receive buffer, then advances partial writes and reads without
  allocating in the submission, progress, completion, receive, or release
  paths.

No module opens a listener, resolves an address, selects a peer, discovers an
interface, performs authentication, negotiates encryption, loads a compiler
or artifact, or owns a file-data path. Those decisions belong to callers.

## End-to-end position in Recipe

The normal data flow is:

1. Discovery and cluster assembly derive a nonzero machine digest and a
   nonzero profile digest for each member. `cluster::MemberProfileIdentity`
   creates an `EndpointIdentity` from those digests, then callers combine two
   endpoint identities with `SessionIdentity::new`.
2. An external owner establishes a TCP connection. The owner passes that
   stream, the session identity, and `ProtocolLimits` to either
   `TcpPeerSession::new` (probe time) or `RuntimeChannel::new` (execution
   time). The transport never performs the establishment step itself.
3. For probing, `recipe_probe::ProbeEngine::probe` receives a
   `&dyn PeerSession`. Its network path creates a bounded
   `PeerBenchmarkControl`, invokes `benchmark_controlled`, and accepts only a
   `Measured` attempt. `TcpPeerSession` supplies the framed implementation.
4. The resulting `PeerMeasurement` carries measured remote-memory and
   directional-throughput properties plus endpoint-bound evidence. Probe
   validation and `cluster::MeasuredNetworkPair::from_probe` reject missing,
   unmeasured, mismatched, or contradictory evidence before profile/topology
   construction.
5. For execution, `recipe_remote::RemoteChannel` wraps the
   `RuntimeChannel`. Its `SessionCore` encodes remote messages into a scratch
   buffer, maps each message to one transport lane, submits it, calls
   `progress`, releases transmitted completions, decodes the one pending
   received frame, and releases that frame. The remote state machines own
   hello/manifest/prepare, init, loop, cancellation, and exit phases; the
   transport only carries and validates their frames.

The transport therefore sits below the remote lifecycle:

```text
connected TcpStream
    -> SessionIdentity + ProtocolLimits
    -> RuntimeChannel (fixed lanes and frame ABI)
    -> RemoteChannel/SessionCore (remote message codec and run state)
    -> MasterHandshake or WorkerHandshake
    -> MasterInit/WorkerInit
    -> MasterRun/WorkerRun
    -> MasterExit/WorkerExit or cancellation
```

Probe sessions use the same frame ABI but never share a runtime channel. A
runtime channel rejects the four probe frame kinds, so a probe cannot leak into
an execution lifecycle.

## Identity and scalar handles

### `EndpointIdentity`

`EndpointIdentity::new(machine, profile)` rejects a zero machine digest or a
zero profile digest with `TransportError::InvalidIdentity`. The values are
opaque `recipe_core::Digest`s and are returned unchanged by the `machine()` and
`profile()` accessors. They are not network addresses and do not establish
trust; they bind a frame to the identities selected by the caller.

### `SessionIdentity`

`SessionIdentity::new(local, remote)` rejects equal machine digests. Profile
digests may differ or match, but the two endpoint machines must be distinct.
`local()` and `remote()` return the endpoint values, while `reversed()` builds
the peer's view of the same connection. Both sides must use opposite local and
remote values, because the receiver compares the sender fields in every header
against its `remote` identity and the receiver fields against its `local`
identity.

### `CompletionToken`

`CompletionToken` is a nonzero `u64` handle. `new(0)` returns
`InvalidConfiguration`; `get()` returns the original value. Runtime tokens are
allocated from one channel-global sequence beginning at `1`. Probe tokens are
derived from a benchmark round, phase, and one-based iteration. Tokens identify
ownership and completion, not wire sequence order.

### `ScheduleStamp`

`ScheduleStamp` wraps a static user-data schedule position. It rejects
`u64::MAX`, which is reserved as the unscheduled header sentinel. `get()`
returns the position. A new connection starts at position `0`; a reused runtime
channel must continue strictly above its previous outbound and inbound
positions.

## Framed wire protocol

`FrameKind` has four probe values and three runtime values:

* `ProbeBegin`, `ProbeReady`, `ProbeData`, and `ProbeComplete` are used only by
  `TcpPeerSession`.
* `Control`, `Metrics`, and `UserData` are the only kinds accepted by
  `RuntimeChannel`.

`FrameKind` is encoded as one byte. Unknown values fail with
`InvalidFrame("unknown frame kind")`.

`FrameMetadata::new(kind, token, schedule)` enforces the lane rule before any
header is written:

* `UserData` requires `Some(finite_position)`.
* Every other kind requires `None`.
* A `Some(u64::MAX)` schedule is invalid because it collides with the wire
  sentinel.

`ProtocolLimits::new(max_payload, operation_timeout)` requires a payload limit
from one byte through 64 MiB (and no larger than the `u32` header field) and a
nonzero operation timeout. `max_payload()` and `operation_timeout()` expose the
validated values. `deadline()` returns an `Instant` based on the configured
timeout. The limit is local configuration, not a handshake or negotiation.

Every frame has exactly 200 bytes of header followed by the payload. All
integer fields are big-endian:

| Bytes | Field |
| --- | --- |
| `0..8` | Magic `RCPTRN01` |
| `8..10` | Transport protocol version `1` |
| `10` | `FrameKind` discriminant |
| `11` | Reserved flags, must be zero |
| `12..16` | Payload length as `u32` |
| `16..24` | Direction-local frame sequence |
| `24..32` | Nonzero `CompletionToken` |
| `32..40` | Schedule position, or `u64::MAX` when unscheduled |
| `40..72` | Sender machine digest |
| `72..104` | Sender profile digest |
| `104..136` | Receiver machine digest |
| `136..168` | Receiver profile digest |
| `168..200` | SHA-256 digest of the payload |

`encode_header` revalidates metadata and the `u32` payload bound. `decode_header`
checks magic, version, reserved flags, frame kind, payload limit, exact
sequence, token, schedule/metadata consistency, and both endpoint identities.
Only then does a receiver read the payload and compare its SHA-256 digest.

### `WireSender` and `WireReceiver`

`split_stream(stream, identity, limits)` sets the supplied stream to blocking
mode and enables `TCP_NODELAY`, clones it for the sender, and returns a
`WireSender` plus `WireReceiver`. Both direction-local sequences start at zero.
The clone is an implementation detail; callers use the two returned handles,
not the original stream.

`WireSender::send(metadata, payload, deadline)` checks the poisoned flag and
payload length, encodes a header with the sender's current sequence, writes the
complete header and payload before incrementing the sequence, and returns
`Ok(())`. Any write error, timeout, or closed peer poisons the sender and shuts
down both directions of its stream. `next_sequence()` reports the next sequence
that would be encoded. A checked increment protects the sequence space.

`WireReceiver::receive_into(buffer, deadline)` reads one complete header,
decodes it against the expected session identity and sequence, checks that the
declared payload fits the caller's buffer, reads the payload, verifies its
digest, increments the receive sequence, and returns `ReceivedFrame { metadata,
payload }` borrowing the caller's buffer. A framing, identity, integrity,
buffer, timeout, I/O, or sequence error poisons the receiver and shuts down
the connection. `next_sequence()` reports the next expected sequence.

Blocking writes and reads repeatedly set the socket timeout to the remaining
deadline. `Interrupted` is retried. `WouldBlock` and timed-out I/O map to
`DeadlineExceeded`; a zero-byte read or write maps to `ConnectionClosed`.
There is no retry after a transport operation fails: the handle is poisoned and
the underlying connection must be discarded by the caller.

`ReceivedFrame<'a>` contains only public `metadata` and a payload slice tied to
the buffer passed to `receive_into`. The transport does not copy or retain a
blocking receive payload after the call returns.

## Probe adapter: `TcpPeerSession`

### Construction and caller contract

`MeasuredLocalMemory { capacity, transfer_rate }` records the local memory
values that the peer should learn during the begin exchange. Its constructor
rejects zero capacity. The `BytesPerSecond` value is already validated by
`recipe_core`.

`TcpPeerSession::new(stream, identity, limits, descriptor, local_memory)`
requires `descriptor.asynchronous_submission == true`, creates a blocking
`WireSender`/`WireReceiver` pair with `split_stream`, and stores them behind a
`Mutex`. The mutex serializes benchmark attempts on one connection. It starts
the benchmark round counter at `1`, so a session can be reused for multiple
sequential measurements without token collisions.

The session implements `recipe_probe::PeerSession`:

* `descriptor()` clones and returns the original `PeerDescriptor`.
* `benchmark(plan)` creates the default `PeerBenchmarkControl`, runs one
  controlled attempt, and converts `Measured`/`Failed` into the compatibility
  `ProbeResult` form.
* `benchmark_controlled(plan, control)` runs the bounded implementation and
  preserves structured `PeerBenchmarkFailure` phase and kind evidence.

`ProbeEngine` is the direct caller. It passes the network plan and a control
object, then calls `PeerBenchmarkAttempt::into_measurement`. A failed attempt
cannot be turned into measured profile properties. There are no current in-tree
callers that construct a `TcpPeerSession`; the type is the concrete transport
implementation supplied by an application or a future connection owner.

### Bounds and control

Before touching the socket, `run_benchmark`:

1. checks cancellation and the caller's absolute deadline,
2. requires `BoundedBenchmarkPlan::is_bounded()`,
3. limits iterations to `1_000_000`,
4. limits `buffer_bytes * iterations` to `8 GiB`,
5. limits one buffer to the configured protocol payload, and
6. obtains the connection mutex without blocking indefinitely. A contended
   mutex is polled with `try_lock` and `thread::yield_now`, with cancellation
   and deadline checked on each pass.

The effective attempt deadline is the earlier of the plan duration and the
caller absolute deadline. Each framed operation uses the earlier of that
attempt deadline and the protocol operation timeout. During the directional
loop, cancellation and deadline are checked before each sample and after each
direction. Cancellation is therefore advisory between framed operations, while
the absolute deadline remains the hard bound for a socket operation.

### Five-phase exchange

Both peers execute the same sequence, using matching frame kind and completion
tokens at every receive:

1. **Validation.** The local plan is checked, the round number is allocated,
   and a 40-byte begin payload is prepared.
2. **Begin exchange.** Each side sends and receives `ProbeBegin` with token
   `(round, phase 1, iteration 0)`. The payload contains, in order, schema 1,
   duplex (`1` full or `2` half), a reserved zero byte, buffer bytes, iteration
   count, maximum duration in nanoseconds, local memory capacity, and local
   memory transfer rate. The peer's plan and duplex must exactly match the
   local values. The peer's memory values are decoded as the remote-memory
   measurement.
3. **Ready exchange.** Each side sends and receives an empty `ProbeReady`
   frame with token `(round, phase 2, iteration 0)`. No data sample is sent
   before both ready frames are accepted.
4. **Directional transfer.** Each sample uses `ProbeData` and token
   `(round, phase 3, iteration + 1)`. The payload is a fixed byte pattern derived
   from the round (`round.to_le_bytes()[0] ^ 0xa5`), and receivers verify every
   byte. Full duplex starts a scoped receiver thread behind a two-party barrier
   and measures send and receive simultaneously. Half duplex orders the two
   directions deterministically by comparing the local and remote machine
   digest bytes, then measures one direction followed by the other.
5. **Completion exchange.** Each side computes its directional rates, sends a
   16-byte `ProbeComplete` payload containing outbound and inbound rates, and
   receives and validates the peer's completion with token `(round, phase 4,
   iteration 0)`. A completion rate must be a valid `BytesPerSecond` value.

Each directional accumulator records sample count, minimum, maximum, integer
mean, integer variance, and total elapsed time. The final rate is
`total_bytes * 1_000_000_000 / elapsed_nanoseconds`, clamped to the representable
nonzero `u64` range before constructing `BytesPerSecond`. A direction with no
samples, an elapsed-time overflow, a sum/variance overflow, or a rate that
cannot be represented is a failed benchmark, not a nominal fallback.

On success the returned `PeerMeasurement` marks remote memory capacity/rate and
both directional rates as `PropertyProvenance::Measured`. Its evidence records
schema 1, both authenticated endpoint digests, `Simultaneous` for full duplex
or `Serialized` for half duplex, and the complete directional statistics.

### Probe failures

Transport failures are mapped to the structured probe failure taxonomy:

| `TransportError` family | `PeerBenchmarkFailureKind` |
| --- | --- |
| Cancellation | `Cancelled` |
| Deadline or timeout | `Deadline` |
| Invalid/other endpoint identity | `Identity` |
| Payload digest mismatch | `Integrity` |
| Unsupported version, invalid frame, sequence, protocol state, or unknown completion | `Protocol` |
| Invalid configuration, size/buffer bound, closed/I/O socket, capacity, internal probe arithmetic, or poisoned session | `Transport` |

Every failure also records the active phase (`Validation`, `BeginExchange`,
`ReadyExchange`, `DirectionalTransfer`, or `CompletionExchange`) and a human
readable detail. The connection remains unusable after a wire failure because
the underlying sender/receiver is poisoned.

## Runtime channel

`RuntimeChannel` is the execution-time transport. It is deliberately
nonblocking and fixed-capacity. Construction validates that the lane payload
capacity does not exceed `ProtocolLimits::max_payload()`, sets the supplied
stream nonblocking, enables `TCP_NODELAY`, allocates every transmit slot and a
receive buffer, and starts transport sequences and completion tokens at zero
and one respectively.

### Configuration and public state

`ChannelCapacities::new(control_slots, metrics_slots, user_data_slots,
max_payload)` requires at least one slot in each lane and a nonzero payload
capacity. `max_payload()` returns the capacity. `RuntimeLane` identifies the
three fixed lanes and maps directly to `FrameKind::Control`, `Metrics`, or
`UserData`.

`Submission { token, lane }` identifies one copied transmit payload.
`CompletionState` reports `Queued`, `Writing`, or `Complete` for a live token.
`Progress { transmitted, received, advanced }` reports at most one newly
completed transmit token and at most one newly ready receive token from one
call. `advanced` is true when any header/payload bytes were moved or a frame
transition completed; it is false when the channel had no I/O to perform or a
received frame is waiting for release.

### Transmit lifecycle

Each lane owns a fixed array of `TxSlot`s. A slot moves through this state
machine:

```text
Free -> Queued -> Writing -> Complete -> Free
```

`submit_control(payload)` and `submit_metrics(payload)` copy a payload into a
free slot, hash it, assign a nonzero token, and return `Submission`. They carry
no schedule. `submit_user_data(schedule, payload)` requires a finite
`ScheduleStamp` strictly greater than the previous outbound user-data stamp,
then performs the same copy and returns a submission carrying that schedule.
All three methods reject a poisoned channel, a payload larger than the
preallocated capacity, exhausted lane slots, exhausted token/order space, or
invalid metadata.

The channel assigns a monotonically increasing submission order. When no frame
is currently writing, `progress` chooses the oldest queued frame in this
priority order: control, then user data, then metrics. It does not globally
compare order values between those three priority groups. Only one frame is
active at a time; its 200-byte header and payload can be written over many
`progress` calls. `WouldBlock` and `Interrupted` leave the slot in place and
return no new completion. A zero-byte write is `ConnectionClosed`; other I/O
errors poison the channel.

When header and payload bytes are complete, the slot becomes `Complete`, the
active key is cleared, the global transmit sequence is incremented, and
`Progress::transmitted` reports the token. The slot remains unavailable until
the caller calls `release_completion(token)`. Releasing before `Complete`, or
using an unknown/already released token, returns a protocol/unknown-completion
error. Releasing is explicit so the caller can keep ownership of a completed
payload until its application event has been observed.

`completion_state(token)` reports the current live slot state. A free or
unknown token is `UnknownCompletion`.

### Receive lifecycle

The single preallocated `ReceiveBuffer` moves through partial header and
payload reads until one frame is ready. `received()` returns `None` until the
complete frame has passed header, runtime-kind, identity, sequence, schedule,
length, and digest checks. It then returns a borrowed `ReceivedFrame` whose
payload remains owned by the channel.

`progress` reads at most one partial header or payload segment after attempting
one transmit step. A complete header is decoded against the expected global
receive sequence. Probe kinds are rejected with `InvalidFrame`, because only
`Control`, `Metrics`, and `UserData` are valid on this channel. Any received
user-data schedule must be strictly greater than the previous inbound stamp.
After the payload digest matches, the frame is marked ready, the inbound global
sequence increments, and `Progress::received` reports its token.

While a frame is ready, subsequent `progress` calls report the same received
token with `advanced == false`; no second frame can overwrite the buffer.
`release_received(token)` requires the pending frame to be ready and the token
to match exactly, then zeroes the used buffer and permits the next receive.
Calling it with no frame, a wrong token, or a frame that is not ready is a
protocol-state error.

### Schedule continuity and reuse

`next_outbound_schedule_position()` and
`next_inbound_schedule_position()` return `0` on a fresh channel or one more
than the latest submitted/received user-data stamp on a reused channel. They
fail when the next position would collide with the `u64::MAX` sentinel. The
transport does not know a logical run's local schedule. `recipe_remote` adds
these connection-global bases to each run's schedule and subtracts the inbound
base after decoding, allowing a connected channel to carry multiple prepared
runs without resetting ordering.

### Poisoning and allocation contract

`progress` marks the channel poisoned for any framing, identity, integrity,
sequence, schedule, closed-socket, or other I/O error. Future submissions and
progress calls return `Poisoned`; the caller must discard the channel. The
transport has no retry or alternate path. Completion and receive release
methods operate on already materialized state so callers can finish ownership
cleanup after a reported completion, but a poisoned channel cannot make further
wire progress.

All lane slots, header arrays, payload storage, and receive storage are
allocated in `RuntimeChannel::new`. The normal submit/progress/completion/
receive/release paths copy into those allocations and perform no heap
allocation. This is a capacity guarantee, not an unbounded queue: if a lane is
full, `CapacityExhausted` is the expected back-pressure signal.

## `recipe-remote` integration

`recipe_remote::RemoteChannel` is a thin owner of a `RuntimeChannel` plus the
last `RunId`. `SessionCore::new` verifies that its `RemoteIdentity` exactly
matches `RuntimeChannel::identity()`, requires a strictly increasing run on a
reused channel, and snapshots the transport's next inbound and outbound
schedule positions. It allocates the remote codec scratch buffer, not the
transport.

For every encoded remote message, `SessionCore::try_send_tracked` chooses the
transport lane from `Message::lane`, encodes a per-lane remote sequence and run
id, and calls the matching `submit_*` method. User-data messages receive the
connection-global schedule stamp. A full transport lane returns `Ok(None)` so a
remote state machine remains pending and retries from its next `progress` call;
other transport errors poison the remote session.

`SessionCore::progress_transport` calls `RuntimeChannel::progress`. Whenever a
transmit token is reported, it immediately calls `release_completion`, making
that fixed slot available for later remote messages. `SessionCore::received`
borrows the transport frame, maps its runtime lane, decodes the remote message,
checks the run id and per-lane remote sequence, translates the schedule back to
the current run, and then `release_received` is called after the state machine
has consumed the message.

The handshake and lifecycle callers are therefore layered, not duplicated in
the transport:

* `MasterHandshake` and `WorkerHandshake` exchange hello, capabilities,
  endpoint/profile proofs, manifest identity, and prepare acknowledgments.
* `MasterInit` and `WorkerInit` use the user-data lane for bounded init image
  chunks and control for acknowledgments.
* `MasterRun` and `WorkerRun` submit tasks, user data, and metrics during the
  prepared loop, with remote task state enforcing the calculation/transfer
  lifecycle.
* `MasterCancelling`, `MasterExit`, and `WorkerExit` use control frames and
  release acknowledgments to close the run while preserving channel reuse.

The transport itself has no knowledge of `RunId`, task phases, manifests,
devices, or driver faults. Those are encoded as payloads by `recipe_remote` and
remain outside this crate's protocol state.

## Errors and caller-visible invariants

`TransportError` is `#[non_exhaustive]`, so callers must not assume the variant
set is closed. Its current variants are:

* `InvalidConfiguration` for zero capacities, unsupported descriptor choices,
  invalid limits, exhausted schedule/token setup, or other caller inputs;
* `InvalidIdentity` and `UnexpectedIdentity` for zero or mismatched endpoint
  digests;
* `UnsupportedVersion` and `InvalidFrame` for ABI, metadata, reserved-field,
  lane, or payload-shape disagreement;
* `FrameTooLarge` and `BufferTooSmall` for declared or destination capacity
  violations;
* `UnexpectedSequence` for a missing, replayed, or out-of-order wire frame;
* `IntegrityFailure` for a SHA-256 or probe-pattern mismatch;
* `Cancelled` and `DeadlineExceeded` for controlled probe interruption or
  expired socket operations;
* `ConnectionClosed` and `Io` for peer closure and other socket failures;
* `ProtocolState` for an invalid local lifecycle transition, including
  releasing a non-complete slot or breaking schedule/sequence monotonicity;
* `CapacityExhausted` for a full fixed lane;
* `UnknownCompletion` for a token that is not live;
* `Probe` for benchmark arithmetic or evidence failures; and
* `Poisoned` after a channel detects an unrecoverable wire failure.

`Display` is stable and human-readable, and `From<io::Error>` maps timed-out or
would-block I/O to `DeadlineExceeded` while preserving other `ErrorKind`s as
`Io`. Every transport operation either completes the requested state transition
or returns one of these explicit failures. It never silently substitutes a
nominal rate, drops a frame, resets a sequence, reuses a live slot, or retries a
poisoned connection.

The caller contract is consequently narrow and explicit:

* establish and own the TCP connection;
* construct nonzero, peer-consistent endpoint identities;
* choose bounded protocol and lane capacities;
* drive blocking probe operations with compatible plans and deadlines, or drive
  runtime operations by repeatedly calling `progress`;
* treat `CapacityExhausted` as back-pressure and retry after progress;
* release every transmitted completion and every received frame exactly once;
* preserve schedule monotonicity when reusing a channel; and
* discard a poisoned channel instead of attempting recovery through another
  transport path.
