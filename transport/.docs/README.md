# recipe-transport

`recipe-transport` is Recipe's bounded TCP framing and channel-capacity crate.
It carries two different kinds of traffic over an already connected
`std::net::TcpStream`:

1. blocking probe frames used by `TcpPeerSession` to measure one established
   peer link; and
2. nonblocking runtime frames used by `recipe-remote::RemoteChannel` for the
   master and worker execution protocol.

The crate owns framing, endpoint identity validation, payload integrity,
directional sequence numbers, deadlines, fixed slot storage, and progress
reporting. It does not open a listener, resolve a name, discover an interface,
choose a peer, establish a socket, negotiate security, compile a program, or
interpret a remote execution message. The caller supplies the connected stream,
the exact local and remote identities, and the limits that were finalized by
higher layers.

The implementation is `#![forbid(unsafe_code)]` and denies missing `Debug`
implementations. The only dependencies are `recipe-core` for identity and
measured-value types, `recipe-probe` for the peer benchmark contract, and
`sha2` for the per-frame payload digest.

## Position in the stack

The transport call graph has two independent entry paths that share the same
wire header and identity checks.

```text
connected TcpStream + SessionIdentity + ProtocolLimits
                         |
             +-----------+-----------+
             |                       |
       split_stream             RuntimeChannel::new
             |                       |
   WireSender/WireReceiver       fixed runtime lanes
             |                       |
     TcpPeerSession                 |
             |                       |
  recipe-probe::PeerSession   recipe-remote::RemoteChannel
             |                       |
  ProbeEngine::probe          Master*/Worker* typestates
```

`recipe-cluster` is the identity and measured-profile caller. Its
`MemberProfileIdentity::derive` canonicalizes a per-machine measured profile,
derives a machine digest from the stable machine ID, derives a profile digest
from the canonical profile bytes, and constructs the transport
`EndpointIdentity`. Cluster validation then binds benchmark evidence to the
`SessionIdentity` endpoints before measured network links enter a profile.
Transport accepts those digests as opaque values and checks them byte for byte;
it does not derive or look up identities itself.

`recipe-probe::ProbeEngine` accepts `&dyn PeerSession`. A `TcpPeerSession`
supplies the descriptor and controlled benchmark implementation, so the engine
can retain only measured peer rates and structured evidence. A failed attempt
never becomes a measured property.

`recipe-remote` is the runtime caller. It receives a `RuntimeChannel` through
`RemoteChannel`, encodes a master/worker message into its own bounded scratch
buffer, submits the encoded bytes to one transport lane, calls
`RuntimeChannel::progress`, and releases completed transport storage. The
remote crate owns role, run, task, transfer, metric, cancellation, and arena
semantics. Transport sees only `Control`, `Metrics`, and `UserData` frames.

## Module map and public surface

`transport/src/lib.rs` keeps the module implementations private and re-exports
the public boundary below. Internal helpers remain `pub(crate)` so callers
cannot bypass the bounded state machines.

| Module | Public boundary | Actual responsibility |
| --- | --- | --- |
| `error.rs` | `TransportError`, `TransportResult` | Closed error vocabulary and `io::Error` conversion. |
| `protocol.rs` | `EndpointIdentity`, `SessionIdentity`, `CompletionToken`, `FrameKind`, `FrameMetadata`, `ProtocolLimits`, `ReceivedFrame`, `WireSender`, `WireReceiver`, `split_stream` | Canonical 200-byte frame headers, blocking deadline I/O, identity and digest checks, and per-direction wire sequence numbers. |
| `runtime.rs` | `RuntimeLane`, `ScheduleStamp`, `ChannelCapacities`, `Submission`, `CompletionState`, `Progress`, `RuntimeChannel` | Preallocated fixed lanes, nonblocking transmit/receive progress, completion ownership, and connection-global user-data schedule monotonicity. |
| `probe.rs` | `MeasuredLocalMemory`, `TcpPeerSession` | `recipe-probe::PeerSession` adapter and the bounded begin/ready/data/complete benchmark protocol. |

### Identity and frame value types

`EndpointIdentity` contains a nonzero machine `Digest` and a nonzero profile
`Digest`. `new` rejects either zero value. Accessors return the two digests by
value.

`SessionIdentity` stores the local and remote endpoint identities. `new`
rejects equal machine identities, because a transport session must connect two
distinct machines. `reversed` swaps the orientation for the other peer. The
orientation is not cosmetic: every frame records both endpoints and a receiver
expects the sender fields to equal its configured remote endpoint and the
receiver fields to equal its configured local endpoint.

`CompletionToken` wraps a nonzero `u64`. A zero token is rejected and can never
appear in a valid frame. Runtime submissions and received frames use this token
to identify storage ownership. Probe tokens additionally encode a benchmark
round, phase, and iteration.

`FrameKind` has exactly seven wire values:

| Value | Kind | Valid owner |
| ---: | --- | --- |
| 1 | `ProbeBegin` | probe exchange |
| 2 | `ProbeReady` | probe exchange |
| 3 | `ProbeData` | directional probe transfer |
| 4 | `ProbeComplete` | probe exchange |
| 16 | `Control` | runtime control messages |
| 17 | `Metrics` | runtime metric messages |
| 18 | `UserData` | runtime scheduled data and init chunks |

Unknown values fail decoding. `is_runtime` recognizes only the last three
values, and `RuntimeChannel` rejects a probe frame that reaches its runtime
receive path.

`FrameMetadata` combines a kind, nonzero completion token, and an optional
schedule position. Only `UserData` may carry a schedule, and a user-data frame
must carry a finite value. Control, metrics, and all probe frames must omit it.
The sentinel `NO_SCHEDULE` is `u64::MAX` and is never a legal schedule value.

`ReceivedFrame<'a>` borrows its payload from the receiver's caller-provided or
preallocated storage. The borrow remains valid until the caller releases the
matching completion token.

### Protocol limits and stream split

`ProtocolLimits::new(max_payload, operation_timeout)` requires a payload limit
from one byte through 64 MiB and a nonzero operation timeout. Its `deadline`
method creates an `Instant` deadline from the configured operation timeout.

`split_stream(stream, identity, limits)` is the only transport protocol
constructor. It puts the supplied `TcpStream` in blocking mode, enables
`TCP_NODELAY`, clones the stream for the writer, and returns one
`WireSender` plus one `WireReceiver`. Both halves start their own sequence at
zero and share the same identity and limits. The function does not connect,
listen, authenticate, or perform a handshake.

### Runtime value types

`RuntimeLane` is the three-lane runtime class. Its private `frame_kind` mapping
is `Control -> FrameKind::Control`, `Metrics -> FrameKind::Metrics`, and
`UserData -> FrameKind::UserData`.

`ScheduleStamp::new` rejects `u64::MAX`; `get` exposes the finite static
position. `ChannelCapacities::new` requires at least one slot in every lane and
a nonzero maximum payload. The runtime constructor additionally requires that
the lane payload capacity does not exceed `ProtocolLimits::max_payload`.

`Submission` returns the allocated completion token and its lane.
`CompletionState` reports `Queued`, `Writing`, or `Complete`. A released slot is
not a completion state and querying its token returns `UnknownCompletion`.

`Progress` contains at most one newly transmitted token, at most one newly
received token, and an `advanced` bit. `advanced` is true when a write or read
made progress, including a frame becoming complete. A pending received frame
is reported repeatedly with `advanced == false` until the caller releases it.

## Canonical frame protocol

Every transport frame is one 200-byte big-endian header followed by exactly the
declared payload bytes. The fixed header is encoded and decoded by
`encode_header` and `decode_header` in `protocol.rs`.

| Byte range | Width | Field | Meaning |
| --- | ---: | --- | --- |
| `0..8` | 8 | magic | ASCII `RCPTRN01`. |
| `8..10` | 2 | protocol version | Current transport version is `1`. |
| `10` | 1 | frame kind | One of the seven `FrameKind` values. |
| `11` | 1 | reserved flags | Must be zero. |
| `12..16` | 4 | payload length | Big-endian `u32`, also bounded by `ProtocolLimits`. |
| `16..24` | 8 | sequence | Direction-local, starts at zero, strictly increments by one. |
| `24..32` | 8 | completion token | Nonzero token for the frame. |
| `32..40` | 8 | schedule | Finite user-data position, or `u64::MAX` for unscheduled frames. |
| `40..72` | 32 | sender machine | Configured sender endpoint machine digest. |
| `72..104` | 32 | sender profile | Configured sender endpoint profile digest. |
| `104..136` | 32 | receiver machine | Configured receiver endpoint machine digest. |
| `136..168` | 32 | receiver profile | Configured receiver endpoint profile digest. |
| `168..200` | 32 | payload digest | SHA-256 over the payload bytes only. |

The sender validates the payload limit and metadata before writing the header.
The receiver validates, in order, magic, version, reserved flags, frame kind,
payload bound, expected sequence, token and schedule metadata, and both endpoint
identities. It then reads the payload and compares its SHA-256 digest with the
header. A mismatch is `IntegrityFailure`.

The header's sender and receiver identities are directional. To use one TCP
connection, the other process must configure the reversed `SessionIdentity`.
The transport does not infer the reversal from the socket or from a role flag.

### Blocking wire halves

`WireSender::send` writes the complete header and payload with one absolute
deadline. `WireReceiver::receive_into` reads one complete frame into the
provided buffer with one absolute deadline. Each helper repeatedly sets the
stream read or write timeout to the remaining duration, handles interrupted
system calls, and fails on a zero-byte read or write. A timeout or `WouldBlock`
maps to `DeadlineExceeded`; other I/O errors retain their `ErrorKind` in
`TransportError::Io`.

The sender increments its sequence only after both header and payload are
written. The receiver increments its sequence only after the payload digest
passes. Any send or receive error poisons that half and shuts down both stream
directions. A poisoned half rejects all later operations with `Poisoned`.
Sequence exhaustion is a protocol-state failure. The stream remains ordinary
TCP; the framing layer supplies bounds, ordering, identity, and integrity but
not encryption or peer discovery.

## RuntimeChannel: fixed, nonblocking transport

`RuntimeChannel::new` is the runtime constructor. It makes the supplied stream
nonblocking, enables `TCP_NODELAY`, allocates every transmit slot and the one
receive buffer, and initializes all connection-local counters:

```text
next_token                 1
next_order                 0
tx_sequence                0
rx_sequence                0
last_submitted_schedule   none
last_received_schedule    none
active transmit slot       none
received frame             none
poisoned                   false
```

The three `FixedLane`s are named `control lane`, `metrics lane`, and
`user-data lane`. Each contains the configured number of `TxSlot`s. A slot owns
one payload allocation of the lane maximum size, a 200-byte header, digest,
metadata, write cursors, and a monotonically increasing submission order. The
`ReceiveBuffer` owns one header and one payload allocation of the same maximum
size. Submission, progress, completion queries, receive access, and release do
not allocate after construction.

### Submission and backpressure

`submit_control` and `submit_metrics` enqueue an unscheduled frame. Every
`submit_user_data` call requires a `ScheduleStamp` and checks that it is
strictly greater than the previous outbound user-data stamp. All submission
methods:

1. reject a poisoned channel;
2. reject payloads larger than the receive buffer capacity;
3. allocate a nonzero `CompletionToken` from `next_token`;
4. validate lane kind and schedule metadata;
5. copy the payload into the first free lane slot and compute its digest;
6. assign the current `next_order`; and
7. advance token and order counters with checked arithmetic.

If no lane slot is free, submission returns `CapacityExhausted` for that lane.
This is bounded backpressure, not an implicit allocation or an automatic wait.
The caller must call `progress`, release completed slots, or choose a later
submission point. Counter exhaustion is `ProtocolState`.

`next_outbound_schedule_position` returns zero for a fresh channel, otherwise
the previous outbound stamp plus one. `next_inbound_schedule_position` does
the same for received user data. Both reject counter exhaustion. The values are
connection-global, so a higher-level run protocol can add a new run's logical
schedule to the base without replaying positions from an earlier run.

### Transmit progress

`progress` first runs `progress_write`, then `progress_read`. If either side
returns an error, the channel is poisoned and the error is returned. Successful
progress combines the two `Activity` results into one `Progress` value.

The write path has one active slot at a time. When no slot is active,
`next_queued` chooses the oldest queued control slot, otherwise the oldest
queued user-data slot, otherwise the oldest queued metrics slot. This is lane
class priority followed by per-lane submission order, not a global reorder of
already active bytes. The selected slot is encoded with the channel's current
transport sequence and changes to `Writing`.

Each call writes either the remaining header or the remaining payload. A
successful partial write advances the corresponding cursor and sets
`advanced`. `WouldBlock` and `Interrupted` return no activity without failing
the channel. A zero-byte write is `ConnectionClosed`; other errors use the
transport I/O conversion. Once all bytes are written, the slot becomes
`Complete`, the active key is cleared, the transmit sequence increments, and
the token is returned in `Progress::transmitted`.

Completed slots remain occupied. `release_completion(token)` requires the token
to identify a `Complete` slot, then zeroes its metadata, header, payload bytes,
digest, cursors, and order and returns it to `Free`. Releasing a queued or
writing slot is `ProtocolState`; an absent token is `UnknownCompletion`.

### Receive progress

The receive path never consumes a second frame while the prior frame is
pending. If `receive.ready` is set, `progress_read` returns the pending token
again and makes no stream progress. Otherwise it reads the fixed header in
nonblocking pieces. After all 200 bytes arrive it calls `decode_header` with
the channel's expected receive sequence and rejects probe kinds. If the frame
has a schedule, it constructs a finite `ScheduleStamp` and requires strict
increase over `last_received_schedule`.

The payload is then read into the preallocated receive buffer. Partial reads
set `advanced`; `WouldBlock` and `Interrupted` leave the channel healthy. After
the declared length is present, `finish_receive` checks the SHA-256 digest,
records the schedule, marks the buffer ready, increments the receive sequence,
and returns the frame token in `Progress::received`.

`received()` exposes the ready frame as a borrowed `ReceivedFrame`. It does not
copy or release the payload. `release_received(token)` requires a ready frame
whose metadata token exactly matches the supplied token, then clears and
zeroes the receive buffer. A missing frame or wrong token is `ProtocolState`.

### Runtime state invariants

The channel state is intentionally small and closed:

| State | Legal transition | Required caller action |
| --- | --- | --- |
| `Free` transmit slot | `submit` -> `Queued` | Hold the returned token. |
| `Queued` | `progress` selects it -> `Writing` | Keep the token outstanding. |
| `Writing` | partial writes remain `Writing`; final write -> `Complete` | Poll until transmitted. |
| `Complete` | `release_completion` -> `Free` | Release only after completion. |
| receive buffer empty | `progress` reads header/payload -> ready | Poll until `received()` is available. |
| receive buffer ready | `release_received` -> empty | Consume and release the matching token. |
| any live state | protocol, identity, digest, sequence, connection, or I/O error -> poisoned | Stop using the channel. |

There is no retry, alternate queue, spill buffer, or implicit slot reclamation.
The only way to recover capacity is the explicit release operation after the
corresponding completion condition. A poisoned channel cannot be made healthy
by another progress call.

## TcpPeerSession and the probe path

`TcpPeerSession` implements `recipe_probe::PeerSession` over the blocking wire
halves. Its constructor receives a connected stream, a `SessionIdentity`,
`ProtocolLimits`, a `PeerDescriptor`, and measured local memory. It rejects a
descriptor whose `asynchronous_submission` flag is false, splits the stream,
and stores both wire halves behind a mutex. The mutex serializes benchmark
attempts on one connection; a concurrent caller waits with `thread::yield_now`
while continuing to check cancellation and deadline state. A poisoned mutex is
reported as `TransportError::Poisoned`.

`MeasuredLocalMemory::new` requires a nonzero `ByteCount` capacity and retains
the supplied `BytesPerSecond` transfer rate. The value is sent to the peer in
the begin payload and is returned as the peer's measured local-memory evidence.

### PeerSession callers and evidence boundary

`ProbeEngine::probe` obtains each session descriptor, creates a bounded network
plan from the seed estimates, creates a fresh `PeerBenchmarkControl`, and
always calls `benchmark_controlled`. It accepts only
`PeerBenchmarkAttempt::Measured`, validates the returned evidence, and then
builds the measured profile. `recipe-cluster::MeasuredNetworkPair::from_probe`
performs the next boundary checks: endpoint evidence must match the
`SessionIdentity`, duplex execution must match the declared link, all rates and
memory properties must have measured provenance, and the recorded bytes,
sample count, elapsed time, and derived rates must agree with the plan.

The topology seed only bounds the initial work selected by `ProbeEngine`; it is
not a measured network rate and does not enter `TcpPeerSession` as a nominal
fallback. The transport result is the sole source of peer throughput evidence.

### Benchmark limits and control

The probe adapter applies all of these bounds before sending a probe frame:

| Constraint | Value or rule |
| --- | --- |
| peer plan | `buffer_bytes != 0`, `iterations != 0`, and nonzero `maximum_duration` |
| iteration ceiling | `1_000_000` iterations |
| total transfer ceiling | `8 GiB`, computed with checked multiplication |
| per-frame payload | must fit the configured protocol maximum, at most 64 MiB |
| attempt deadline | earliest of plan duration from now and caller absolute deadline |
| operation deadline | earliest of the phase deadline, caller deadline, and one protocol operation timeout |
| cancellation | checked before phases and between directional frame operations |

`benchmark(plan)` creates a `PeerBenchmarkControl` whose absolute deadline is
the plan duration. `benchmark_controlled(plan, control)` retains the caller's
absolute deadline and cloneable cancellation state, then converts every
failure into a structured `PeerBenchmarkFailure` with a phase and kind. The
phases are `Validation`, `BeginExchange`, `ReadyExchange`,
`DirectionalTransfer`, and `CompletionExchange`.

### Wire phases

One successful benchmark is a lock-protected sequence of four framed
exchanges. Both peers execute the same phases with the same round token
components.

```text
Validation
  -> ProbeBegin send and receive
  -> ProbeReady send and receive
  -> ProbeData iterations in the selected duplex mode
  -> ProbeComplete send and receive
  -> measured PeerMeasurement
```

#### Begin exchange

The session increments a connection-local `next_round`, starting at one, and
encodes a 40-byte `ProbeBegin` payload:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 2 | peer benchmark schema, currently `1` |
| 2 | 1 | duplex, `1` for full or `2` for half |
| 3 | 1 | reserved byte, must be zero |
| 4 | 8 | buffer byte count |
| 12 | 4 | iteration count |
| 16 | 8 | maximum duration in nanoseconds |
| 24 | 8 | local memory capacity |
| 32 | 8 | local memory transfer rate |

Each peer sends and receives a `ProbeBegin` with the same token. The receiver
requires schema compatibility, a zero reserved byte, the expected duplex, and
an exactly equal `BoundedBenchmarkPlan`. It validates the peer memory values
through the core value constructors.

#### Ready exchange

Each peer sends and receives a zero-length `ProbeReady` frame with the phase
token. This separates plan and memory agreement from data measurement. A frame
of another kind or with another token is an invalid out-of-phase frame.

#### Directional transfer

The adapter allocates one payload of the requested size and fills it with a
round-specific byte pattern, `round.to_le_bytes()[0] ^ 0xa5`. Every iteration
uses a `ProbeData` token with phase `3` and a one-based iteration component.

For a full-duplex link, `measure_full_duplex` starts a scoped receiver thread,
uses a two-party barrier, and measures send and receive concurrently. The
receiver verifies every byte against the round pattern. The sender and
receiver maintain independent directional accumulators containing sample
count, minimum, maximum, sum, sum of squares, and elapsed duration.

For a half-duplex link, both peers compare their machine digest bytes. The
lexicographically lower machine sends all iterations first and receives
second; the other peer receives first and sends second. This deterministic
ordering prevents both ends from waiting to send at once and records
`PeerDuplexExecution::Serialized` evidence.

After all iterations, the accumulators require at least one sample. The
transport computes each rate as
`total_bytes * 1_000_000_000 / elapsed_nanoseconds`, checked in `u128`, clamped
to the valid `BytesPerSecond` range, and tagged with measured provenance.

#### Completion exchange

The peer sends a 16-byte `ProbeComplete` payload containing its outbound and
inbound measured rates as two big-endian `u64` values. It receives and validates
the peer's corresponding payload. The final `PeerMeasurement` contains remote
memory capacity and rate, both directional rates, endpoint identity evidence,
protocol schema, duplex execution mode, and the full directional timing
evidence. No nominal rate is substituted if measurement fails.

### Probe failure behavior

`benchmark_failure` maps transport failures to the probe control vocabulary:

| Transport condition | `PeerBenchmarkFailureKind` |
| --- | --- |
| `Cancelled` | `Cancelled` |
| `DeadlineExceeded` | `Deadline` |
| zero identity or wrong frame identity | `Identity` |
| payload digest or data-pattern mismatch | `Integrity` |
| unsupported version, invalid frame, sequence, protocol state, unknown completion | `Protocol` |
| invalid configuration, bounds, closed connection, I/O, capacity, probe arithmetic, or poisoned wire | `Transport` |

The failure retains the active phase and string detail. `ProbeEngine` converts a
failed attempt into a `ProbeError` before profile construction, so cancellation,
deadline, identity disagreement, malformed frames, and incomplete samples cannot
be mistaken for a measured link.

## Runtime integration with master and worker execution

The transport layer is deliberately below the master/worker protocol. The
following is the actual handoff into `recipe-remote`:

1. An external caller connects the TCP stream and constructs a
   `SessionIdentity` in each direction. The caller also chooses a
   `ProtocolLimits` and `ChannelCapacities` that match the finalized remote
   limits.
2. Both processes construct `RuntimeChannel` over their own stream endpoint.
3. `RemoteChannel::new` wraps the channel. It preserves the connection-global
   last `RunId` and returns the channel only after `MasterComplete` or
   `WorkerComplete`.
4. The master creates `MasterHandshake`; the worker creates
   `WorkerHandshake` with a local `WorkerDriver`. Each `progress` call first
   advances transport, then consumes at most the frame currently held by the
   receive buffer, and explicitly releases that frame token.
5. The remote codec encodes protocol messages into a bounded scratch buffer and
   maps them to transport lanes. Transport never parses the remote message
   bytes.

The remote codec's lane mapping is fixed:

| Runtime lane | Remote messages |
| --- | --- |
| `Control` | `Hello`, `Manifest`, acknowledgments, `Prepare`, `Execute`, task completion/failure, `DriverFault`, `DataRequest`, `DataAck`, `Cancel`, `BeginExit`, `ExitReady`, `Release`, `ExitComplete`, and terminal acknowledgments. |
| `UserData` | `InitChunk` and `UserData`. Both carry the static schedule required by transport. |
| `Metrics` | `Metric`. |

`SessionCore::try_send_tracked` increments a remote per-lane codec sequence
only after transport accepts the frame. It maps a full transport slot to a
nonfatal remote `Backpressured` result. Any other transport error poisons the
remote session. `SessionCore::progress_transport` calls
`RuntimeChannel::progress` and immediately releases each returned transmitted
token. A remote caller must still release each received token after decoding
and validating the frame.

The transport sequence is one connection-global sequence per direction across
all three runtime lanes. The remote codec sequence is separate per lane. This
two-level ordering is intentional: TCP framing remains globally ordered while
remote control, metric, and user-data message streams retain independent
logical sequence numbers.

### Handshake: identity and plan proof

The public remote typestates are `MasterHandshake` and `WorkerHandshake`.
Both sides send `Hello` on the control lane. The receiver checks the role,
required capabilities, both endpoint/profile digests, and the exact fixed
remote limits. The master then sends the provisioned manifest; the worker
validates the bundle, draft, realization, manifest, program, and artifact
proof, then sends `ManifestAck`. The master validates the acknowledgment and
sends `Prepare`. The worker calls its `WorkerDriver::prepare`, then sends
`PrepareAck`. Only that acknowledgment changes the typestate to `MasterInit`
and `WorkerInit`.

Transport contributes the endpoint fields, sequence, digest, and lane
capacity. `recipe-remote` contributes the role, capabilities, run ID, manifest,
and order checks. A wrong lane, wrong identity, wrong run, duplicate, or
out-of-order remote message poisons the remote session after the transport frame
has already passed its own checks.

### Init: one bounded image at a time

The master enters `MasterInit` with one finalized arena image per worker device.
`queue_image` accepts exactly one image while no other logical image is active,
requires the known device and exact finalized byte count, hashes the image, and
starts this control sequence:

```text
InitBegin (Control)
  -> InitChunk* (UserData, strictly increasing schedule)
  -> InitEnd (Control)
  -> InitAck (Control)
```

The maximum chunk payload is the remote message capacity minus the remote codec
overhead. The transport user-data schedule is connection-global; `MasterInit`
starts its logical schedule at zero and `SessionCore` adds the channel's
outbound schedule base. `validate_init_schedule` ensures a provisioned
master-to-worker transfer never collides with the reserved init chunk positions.

The worker receives one image and one chunk at a time. It validates the device,
size, digest, offset, exact next schedule, and bounded chunk length, copies the
chunk into its preallocated scratch, and calls the driver. A chunk is not
accepted as complete until `poll_init_chunk` reports the exact byte count. On
`InitEnd`, the worker verifies the accumulated digest and calls
`finish_init_image`; only then does it send `InitAck`. After every device image
is acknowledged, the master sends `InitComplete` and the worker sends
`InitCompleteAck`, producing `MasterRun` and `WorkerRun`.

The transport consequences are bounded user-data slots, one pending received
frame, monotonic schedules, and explicit release of each frame before the next
one is decoded. A second image, overlapping chunk, wrong offset, or unbounded
payload is rejected by the remote state machine rather than hidden by another
transport buffer.

### Run: calculations, metrics, and cross-machine data

`MasterRun` and `WorkerRun` use only finalized task and transfer IDs. The master
submits local worker-driver tasks as `Execute` control frames. A worker admits
each command into one fixed task slot, polls its `WorkerDriver`, and reports
`TaskComplete`, `TaskFailed`, or a separate `Metric` frame. Metrics use the
metrics lane so they do not consume control or user-data slots.

For a master-to-worker cross transfer, the master validates the provisioned
direction, byte count, phase, and idle task state, acquires any finalized
half-duplex capacity token, then submits scheduled `UserData`. The worker
validates the same static transfer and schedule, stages the bytes into its
bounded ingress buffer, and acknowledges driver completion with `TaskComplete`.

For a worker-to-master transfer, the master submits `DataRequest` on control.
The worker produces exactly the provisioned byte count into its bounded egress
buffer, submits scheduled `UserData`, and waits for `DataAck`. The master holds
the received bytes in one fixed data slot, exposes `DataReady`, and sends
`DataAck` only after the caller's data handling point. The corresponding slot
and half-duplex token are then released. Only one worker-to-master request and
one pending data acknowledgment are allowed by the remote state machine.

Transport enforces the frame size, schedule, sequence, identity, digest, and
slot lifetime at every step. Remote enforces the task phase, static transfer
contract, half-duplex resource ownership, and native driver progress.

### Exit and cancellation

The master can call `MasterRun::begin_exit` only after all loop-phase tasks are
complete. `MasterExit::progress` sends `BeginExit`, waits for `ExitReady`, runs
exit-phase tasks through the same three transport lanes, and then requests each
worker arena release exactly once. The worker calls its driver's release hook,
sends one `ReleaseAck` for each device, and only then accepts `ExitComplete`.
After the worker driver finishes, it sends `ExitAck`; the flushed terminal frame
produces `MasterComplete` and `WorkerComplete`.

Cancellation changes the terminal path, not the transport rules. The master
enters `MasterCancelling`, sends `Cancel`, tolerates already in-flight task,
metric, data, and exit-ready frames, waits for exact release acknowledgments,
and accepts `CancelAck` only after every worker arena release is complete. The
worker invokes its driver cancellation hook, performs the same per-arena release
and acknowledgment sequence, finishes native cleanup, and flushes `CancelAck`.
Any driver fault follows a separate cleanup-before-fault-report path in
`recipe-remote`; transport carries the resulting `DriverFault` control frame
and does not attempt recovery or retry.

The complete remote lifecycle, including the transport channel underneath, is:

```text
MasterHandshake  <-> WorkerHandshake
        |                    |
MasterInit       <-> WorkerInit
        |                    |
MasterRun        <-> WorkerRun
        |                    |
MasterExit       <-> WorkerExit
        |                    |
MasterComplete   <-> WorkerComplete

MasterRun or MasterExit --Cancel--> MasterCancelling <-> WorkerCancelled
                                      -> Complete
```

The channel itself does not know these typestates. It supplies the bounded
progress and ownership operations that make each remote transition explicit.

## Errors and fail-closed behavior

`TransportError` is `#[non_exhaustive]`, so downstream matches must retain a
future-proof wildcard. The variants and their direct meanings are:

| Variant | Direct cause |
| --- | --- |
| `InvalidConfiguration(&'static str)` | Constructor, sentinel, identity, or bound configuration is invalid. |
| `InvalidIdentity(&'static str)` | Machine or profile digest is zero. |
| `UnsupportedVersion(u16)` | Wire or probe schema is not the supported version. |
| `InvalidFrame(&'static str)` | Magic, flags, kind, metadata, payload shape, or probe phase is malformed. |
| `FrameTooLarge { declared, limit }` | Declared or submitted payload exceeds the configured bound. |
| `BufferTooSmall { required, available }` | Blocking receiver's caller buffer cannot hold the declared payload. |
| `UnexpectedSequence { expected, received }` | Direction-local wire sequence is replayed, skipped, or out of order. |
| `UnexpectedIdentity` | Header endpoints do not match the configured session orientation. |
| `IntegrityFailure` | SHA-256 payload digest or probe data pattern does not match. |
| `Cancelled` | Caller cancellation was observed at a bounded probe control check. |
| `DeadlineExceeded` | The remaining operation or attempt deadline elapsed. |
| `ConnectionClosed` | The peer returned zero bytes while a frame was incomplete. |
| `Io(io::ErrorKind)` | An I/O error other than timeout or would-block. |
| `ProtocolState(&'static str)` | A local state transition, sequence counter, schedule, or release rule is invalid. |
| `CapacityExhausted(&'static str)` | A fixed lane, buffer, or bounded probe resource cannot accept the request. |
| `UnknownCompletion(u64)` | A completion token is not held by any transmit slot. |
| `Probe(String)` | Probe arithmetic, sample, duration, or peer evidence construction failed. |
| `Poisoned` | A prior protocol or I/O failure closed the channel. |

`TransportError::io` maps `TimedOut` and `WouldBlock` to
`DeadlineExceeded`; runtime progress handles expected nonblocking `WouldBlock`
and `Interrupted` inline as no progress before this conversion is needed.

The fail-closed rules are consistent across both paths:

- protocol, identity, sequence, digest, connection, and unexpected I/O errors
  poison the affected wire or runtime channel and close or stop it;
- capacity exhaustion during runtime submission is reported directly so the
  caller can observe bounded backpressure, but it never allocates or silently
  drops a frame;
- a completed transmit or received frame remains owned until its explicit
  release operation succeeds;
- a probe failure is represented as structured phase evidence and cannot enter
  the measured profile; and
- transport does not retry, reorder, substitute a nominal rate, accept a
  second receive while one is pending, or recover a poisoned channel.

## Practical use boundary

The minimal runtime setup is intentionally explicit:

```rust,ignore
let identity = SessionIdentity::new(local_endpoint, remote_endpoint)?;
let limits = ProtocolLimits::new(max_payload, operation_timeout)?;
let capacities = ChannelCapacities::new(control_slots, metric_slots, user_data_slots, max_payload)?;
let channel = RuntimeChannel::new(stream, identity, limits, capacities)?;
let mut remote = recipe_remote::RemoteChannel::new(channel);
```

The caller must already have a connected stream and finalized values for the
identity, timeout, payload bound, and lane capacities. It then drives the
remote typestate by repeatedly calling its `progress` method, handling the
returned event, and allowing the remote layer to release transport completions.
For a direct wire or probe use, `split_stream` and `TcpPeerSession` expose the
same framing without the remote execution codec.

Compilation checks structural validity only. The relevant structural commands
are:

```text
cargo check -p recipe-transport
cargo fmt --all -- --check
```

Runtime claims require an established peer stream, the real probe or remote
entry point, and the corresponding measured or native system. A successful
compile does not prove identity agreement, payload integrity, backpressure
behavior, measured throughput, or master/worker lifecycle completion.
