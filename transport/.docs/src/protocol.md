# Recipe transport and remote wire protocol

This document describes the protocol that is implemented by the current
`recipe-transport` and the `recipe-remote` callers. It is an implementation
record, not a proposed protocol. The transport crate is deliberately narrow:
it receives an already connected `std::net::TcpStream` and caller-supplied
endpoint identities. It does not listen, resolve names, discover interfaces,
select a peer, authenticate a user, or establish a connection.

There are three wire layers, each with its own version and byte order:

1. The transport frame (`transport/src/protocol.rs`) provides framing,
   endpoint binding, a direction-local sequence, a completion token, a static
   schedule stamp, and a SHA-256 payload digest.
2. The probe payload protocol (`transport/src/probe.rs`) uses transport
   frames with `Probe*` kinds. Its payload schema is
   `recipe_probe::PEER_BENCHMARK_PROTOCOL_SCHEMA`, currently `1`, and its
   integer fields are big endian.
3. The remote execution codec (`remote/src/codec.rs`) puts a second, manual
   message header inside runtime transport frames. Its message protocol is
   `recipe_remote::PROTOCOL_VERSION`, currently `2`, and its integer fields
   are little endian. `recipe-remote` is a caller of `RuntimeChannel`, not a
   second socket implementation.

The transport frame sequence is global per stream direction. RuntimeChannel
adds one nonblocking scheduler and fixed storage around that stream. Remote
messages add independent per-lane sequences and a nonzero connection-global
`RunId`. Thus an outer sequence proves byte-stream order, while a remote
per-lane sequence proves the order of messages after the lane scheduler has
interleaved lanes.

## Ownership and end-to-end call path

The call path is:

```text
caller owns connected TcpStream and identities
    -> recipe_transport::split_stream or RuntimeChannel::new
    -> WireSender/WireReceiver for a probe
       or RuntimeChannel for runtime frames
    -> recipe_remote::SessionCore encodes a remote message into a scratch
       buffer and submits it to the matching runtime lane
    -> RuntimeChannel writes one complete outer frame and reads one complete
       outer frame
    -> SessionCore decodes and validates the inner message
    -> MasterHandshake/WorkerHandshake, Init, Run, Exit, or Cancel state
       invokes the caller's WorkerDriver or returns an event
```

`recipe-probe::ProbeEngine` is the probe caller. It obtains each
`PeerDescriptor`, creates a `PeerBenchmarkControl`, invokes
`PeerSession::benchmark_controlled`, and accepts only a `Measured` attempt for
profile construction. `TcpPeerSession` is the concrete `PeerSession`
implementation in this crate. `cluster` consumes the measured endpoint and
peer evidence, but does not put rates into the transport header or configure a
wire fallback. `recipe-remote` is the only current runtime message caller in
the workspace. Its caller supplies the same `ProvisionedProgram`, identities,
limits, and an already constructed `RuntimeChannel` on both peers.

The public transport surface is exported from `transport/src/lib.rs`:
`EndpointIdentity`, `SessionIdentity`, `CompletionToken`, `FrameKind`,
`FrameMetadata`, `ProtocolLimits`, `WireSender`, `WireReceiver`,
`split_stream`, `ChannelCapacities`, `RuntimeChannel`, `RuntimeLane`,
`ScheduleStamp`, `Submission`, `CompletionState`, `Progress`, and
`ReceivedFrame`. The remote surface is exported from `remote/src/lib.rs` and
contains the typed lifecycle states and the `WorkerDriver` boundary.

## Version namespaces and fixed limits

| Layer | Source constant | Current value | Byte order | Scope |
| --- | --- | ---: | --- | --- |
| outer transport header | `PROTOCOL_VERSION` | `1` | big endian fields | every transport frame |
| probe payload | `PEER_BENCHMARK_PROTOCOL_SCHEMA` | `1` | big endian fields | `ProbeBegin` payload |
| remote message | `remote::PROTOCOL_VERSION` | `2` | little endian fields | runtime payloads |
| probe profile | `PROFILE_SCHEMA` and `PROFILE_CODEC_SCHEMA` | `7` | profile codec-defined | measured profile/cache, not wire framing |

The outer protocol accepts a payload limit from `ProtocolLimits::new`. The
limit is nonzero, no greater than 64 MiB, and must fit a `u32` length field.
The operation timeout is nonzero. `WireSender` checks this limit before
encoding; `decode_header` checks the declared length before reading a payload.
`RuntimeChannel` additionally allocates a fixed payload buffer and requires
its configured capacity to be no greater than the `ProtocolLimits` maximum.

Probe validation imposes two more limits: at most `1_000_000` iterations and
at most `8 * 1024 * 1024 * 1024` total benchmark bytes. The bounded plan must
have nonzero buffer bytes, iterations, and duration. The ordinary
`ProbeEngine` currently derives plans with a 4 KiB to 64 MiB buffer, eight
iterations, and a two-second maximum duration, but `TcpPeerSession` validates
the supplied plan independently.

Remote limits are fixed in `RemoteLimits` and are sent in every Hello as eight
little-endian `u64` values, in this exact order:

```text
max_message_bytes,
max_artifacts,
max_tasks,
max_devices,
max_transfers,
task_slots,
data_slots,
metric_slots
```

`max_message_bytes` is at least 256 and at most `u32::MAX`; every manifest and
runtime capacity is nonzero. The peer must advertise exactly the same tuple,
not merely compatible values. The remote codec rejects a decoded message
larger than `max_message_bytes`, an artifact count greater than the advertised
maximum, and lengths that do not fit the host or the message buffer. The
transport caller must choose a runtime payload capacity that can carry this
message limit; a mismatch otherwise becomes a transport frame-size failure
when a message is submitted.

The remote codec reserves 28 bytes for its inner header. A user-data message
also carries an eight-byte task ID and four-byte byte length, so its maximum
data bytes are `max_message_bytes - 40`. An init chunk carries an eight-byte
device ID, eight-byte offset, and four-byte byte length, so its maximum chunk
bytes are `max_message_bytes - 48`. `RemoteLimits::new` permits a 256-byte
minimum precisely so these fixed fields fit.

## Identities, tokens, metadata, and schedules

`EndpointIdentity` contains a machine `Digest` and a profile `Digest`. Both
must be nonzero. `SessionIdentity` contains local and remote endpoint
identities and rejects equal machine digests. `SessionIdentity::reversed`
produces the peer's view. The outer header carries all four digests, so a frame
is bound to both the direction and the exact measured profile. `decode_header`
requires the sender fields to equal `identity.remote`, and the receiver fields
to equal `identity.local`; it does not infer or negotiate identities.

`CompletionToken` is a nonzero `u64`. Blocking probes derive tokens from the
round, phase, and iteration. RuntimeChannel allocates monotonically from `1`
and uses each token to identify one fixed transmit slot or one received frame.
Remote message sequencing is separate from the transport completion token.

`FrameMetadata` contains `FrameKind`, a completion token, and an optional
schedule. A `UserData` frame must have a finite schedule, and every other kind
must have no schedule. `u64::MAX` is the on-wire unscheduled sentinel and is
also rejected as a `ScheduleStamp`. RuntimeChannel requires outbound and
inbound user-data schedules to increase strictly. Its first legal schedule is
zero, or one greater than the last schedule on a reused connected channel.

The remote layer translates each run's logical schedule onto those
connection-global positions. `SessionCore` captures the channel's next
outbound and inbound positions at construction. A remote user-data schedule is
added to the outbound base before submission and has the inbound base
subtracted after decoding. Init chunks use logical schedules beginning at
zero. The master-to-worker cross-transfer range starts after the total number
of init chunks; worker-to-master transfers start at logical zero. This prevents
an init chunk and a later master-to-worker transfer from sharing a position.

## Outer transport frame ABI

Every frame is exactly a 200-byte header followed by the declared payload.
All integer fields in this header are big endian. The following offsets are
byte offsets from the beginning of the header, with the end offset exclusive:

| Offset | Size | Field | Encoding and meaning |
| ---: | ---: | --- | --- |
| 0 | 8 | magic | ASCII `RCPTRN01` |
| 8 | 2 | version | `u16`, must equal `1` |
| 10 | 1 | kind | `FrameKind` discriminant |
| 11 | 1 | flags | reserved, must be `0` |
| 12 | 4 | payload length | `u32`, in bytes |
| 16 | 8 | transport sequence | starts at `0`, increments by one per frame in this direction |
| 24 | 8 | completion token | nonzero `u64` |
| 32 | 8 | schedule | finite user-data position, or `u64::MAX` for all other kinds |
| 40 | 32 | sender machine | local machine digest at the sender |
| 72 | 32 | sender profile | local profile digest at the sender |
| 104 | 32 | receiver machine | remote machine digest expected by the sender |
| 136 | 32 | receiver profile | remote profile digest expected by the sender |
| 168 | 32 | payload digest | SHA-256 of the payload bytes |

There is no header checksum, compression marker, encryption field, or
extension length. The reserved flags byte is the only reserved header byte.
`encode_header` validates metadata and the `u32` length before filling all
fields. `decode_header` validates magic, version, flags, kind, payload limit,
the expected direction-local sequence, token, schedule/metadata combination,
and both endpoint identities before returning a decoded header.

The current `FrameKind` discriminants are:

| Value | Kind | Lane or use |
| ---: | --- | --- |
| 1 | `ProbeBegin` | blocking peer benchmark begin exchange |
| 2 | `ProbeReady` | blocking peer benchmark readiness exchange |
| 3 | `ProbeData` | blocking directional benchmark samples |
| 4 | `ProbeComplete` | blocking benchmark completion/rate exchange |
| 16 | `Control` | remote handshake, lifecycle, task, release, and fault messages |
| 17 | `Metrics` | remote metric samples |
| 18 | `UserData` | remote init chunks and cross-boundary data |

Only values in this table decode. `FrameKind::is_runtime` accepts exactly
`Control`, `Metrics`, and `UserData`; RuntimeChannel rejects all four probe
kinds and never accepts a probe frame on a runtime channel.

## Blocking sender and receiver

`split_stream` makes a TCP stream blocking, enables `TCP_NODELAY`, clones the
stream for the sender, and initializes both direction-local transport
sequences to zero. It returns `WireSender` over the clone and `WireReceiver`
over the original. The two halves carry the same `SessionIdentity` and
`ProtocolLimits`; they do not perform a handshake of their own.

`WireSender::send` first rejects a payload above the configured limit. It
encodes the header with the current sequence and digest, writes the complete
header and then the complete payload with `write_all_deadline`, and increments
the sequence after both writes finish. Each write recomputes the remaining
deadline, retries interrupted system calls, treats a zero-byte write as a
closed connection, and maps `TimedOut` or `WouldBlock` to
`DeadlineExceeded`. Any send error poisons the sender and shuts down both
directions of its stream. A poisoned sender rejects every later send.
The sequence increment is checked after the bytes are written; if the `u64`
sequence space is exhausted, the send returns `ProtocolState` after the frame
has already left the socket. The normal bounded runs never reach this state.

`WireReceiver::receive_into` reads exactly 200 header bytes, decodes them
against the expected sequence and identity, checks that the caller's buffer
can hold the declared payload, reads exactly that many bytes, and compares the
SHA-256 digest. Only after the digest matches does it increment the receive
sequence and return `ReceivedFrame { metadata, payload }`. Any error poisons
the receiver and shuts down both directions. An empty payload is valid and
still gets a digest and a completion token.

The deadline helpers set the TCP read or write timeout for each partial
operation. A deadline that is already elapsed, or has only a zero duration
remaining, produces `DeadlineExceeded` before the system call. No retry or
alternate transport path exists.

## Nonblocking RuntimeChannel

`RuntimeChannel::new` requires nonzero fixed slots in all three lanes and a
nonzero payload capacity. It rejects a capacity greater than
`ProtocolLimits::max_payload`, makes the stream nonblocking, enables
`TCP_NODELAY`, allocates every transmit slot and one receive buffer, and
starts `next_token`, `tx_sequence`, and `rx_sequence` at `1`, `0`, and `0`
respectively. The channel has no heap allocation on submit, progress,
completion, receive, or release paths after construction.

Each lane owns fixed `TxSlot` values. A slot stores metadata, a 200-byte
header buffer, a preallocated payload buffer, partial header/payload offsets,
the payload digest, and a global submission order. States are `Free`, `Queued`,
`Writing`, and `Complete`. Submitting a control, metrics, or user-data payload
copies it into the first free slot in that lane, computes its digest, allocates
a nonzero completion token, and increments global token and order counters.
Lane exhaustion returns `CapacityExhausted` and does not silently drop data.

`submit_user_data` checks strict schedule monotonicity before submitting. The
other two submit methods create unscheduled metadata. Payloads above the
receive buffer capacity return `FrameTooLarge` before a slot is taken.

`progress` performs one write pass and one read pass, even if the write pass
already returned an error. A successful `Progress` reports the token whose
frame completed transmission this pass, the token whose frame became ready
for receipt this pass, and whether either side advanced. Any write or read
error marks the channel poisoned. A poisoned channel rejects submit, progress,
and all other operations that call `ensure_healthy`.

### Write order and completion

If there is no active write, `next_queued` selects the oldest queued control
slot first, then the oldest user-data slot, then the oldest metrics slot. This
is a lane priority, not a global-order sort. Only one slot is active at a time.
The header is encoded with the channel's global `tx_sequence`; partial writes
remain in `Writing` until all 200 header bytes are sent, followed by all
payload bytes. `WouldBlock` and `Interrupted` leave the slot active and return
no activity. A zero-byte operation is `ConnectionClosed`. When payload bytes
are complete, the slot becomes `Complete`, the active key is cleared, and the
outer transmit sequence increments. A zero-length payload completes as soon
as its header finishes. The increment is checked after the frame is complete;
an exhausted `u64` sequence reports `ProtocolState` at that point.

`completion_state` reports `Queued`, `Writing`, or `Complete` for a live token.
`release_completion` requires `Complete`, then resets the entire slot to
`Free`; completion storage is not recycled automatically. Releasing a token
before completion is a protocol-state error, and a token not found in any lane
is `UnknownCompletion`.

### Read order and receive ownership

The single receive buffer holds one header, one decoded header, and one payload
at a time. If a frame is already `ready`, `progress` reports its token again
with `advanced = false` without reading another frame. Otherwise it reads the
header incrementally.
Once complete, it performs all outer-header validation, rejects probe kinds,
and checks strict inbound schedule order. For a zero-length payload it checks
the digest immediately; otherwise it reads payload bytes incrementally. A
digest mismatch poisons the channel. On success, `ready` becomes true and the
outer receive sequence increments.

`received` borrows the ready frame. The caller must process it and call
`release_received` with exactly its completion token before the channel can
accept another frame. Releasing without a pending frame, with a wrong token,
or before `ready` is a protocol-state error. Thus the runtime boundary has
explicit ownership on both sides: a transmitted slot is held until completion
release, and received bytes are held until receive release.

The configured capacities must therefore be chosen consistently: a remote
peer may legally declare any payload up to `ProtocolLimits::max_payload`, but
the runtime receive buffer is only `ChannelCapacities::max_payload`. Outbound
submission checks the smaller runtime buffer. Inbound `progress_read` checks
only the protocol limit before taking a slice of the fixed receive buffer; it
does not separately reject a declared payload between the runtime capacity
and the protocol limit. With mismatched values, such a peer frame reaches an
out-of-bounds slice rather than a structured `BufferTooSmall` error. The
current callers must keep the runtime capacity and protocol maximum aligned;
the implementation does not negotiate or repair this mismatch.

## Probe protocol and measured-profile boundary

`TcpPeerSession` is a serialized owner of one blocking probe connection. Its
`ProbeConnection` contains a `WireSender`, a `WireReceiver`, the authenticated
`SessionIdentity`, the same `ProtocolLimits`, and `next_round`, initially `1`.
The connection is guarded by a mutex. A concurrent benchmark spins with
`thread::yield_now`, checking cancellation and the absolute deadline on every
attempt; a poisoned mutex is reported as a transport poison failure.

The constructor rejects a `PeerDescriptor` whose
`asynchronous_submission` flag is false. It then calls `split_stream`, so
probe I/O is blocking and still uses the 200-byte outer frame. The session
does not create a listener or negotiate the descriptor. The caller has already
selected the stream, descriptor, local measured memory, and endpoint
identities.

### Attempt control and phases

`PeerSession::benchmark` creates a fresh `PeerBenchmarkControl` whose absolute
deadline is now plus the plan duration, then delegates to
`benchmark_controlled`. The concrete implementation runs these phases in
order:

1. `Validation`
2. `BeginExchange`
3. `ReadyExchange`
4. `DirectionalTransfer`
5. `CompletionExchange`

Cancellation is an atomic caller-owned flag. It is checked between framed
operations, not by interrupting a system call. Every transport operation uses
the earliest of the plan/phase deadline, the caller's absolute deadline, and
the per-operation `ProtocolLimits` timeout. An unsuccessful attempt is
returned as `PeerBenchmarkAttempt::Failed` with the active phase and one of
`Cancelled`, `Deadline`, `Identity`, `Integrity`, `Protocol`, or `Transport`.
`ProbeEngine` calls `into_measurement`, which rejects every failed attempt and
only allows a genuinely measured result into a profile.

Validation requires a bounded, nonzero plan, at most one million iterations,
and total bytes no greater than 8 GiB. The buffer length must fit the host
address space and the configured outer payload limit. `next_round` is consumed
before the first frame; exhausting its `u32` space is a protocol-state failure.

### Probe payload ABI

`ProbeBegin` has exactly 40 payload bytes. Its fields are big endian at these
offsets:

| Offset | Size | Field | Values |
| ---: | ---: | --- | --- |
| 0 | 2 | payload schema | `1` (`PEER_BENCHMARK_PROTOCOL_SCHEMA`) |
| 2 | 1 | duplex | `1` full duplex, `2` half duplex |
| 3 | 1 | reserved | must be `0` |
| 4 | 8 | buffer bytes | plan `buffer_bytes` |
| 12 | 4 | iterations | plan `iterations` |
| 16 | 8 | maximum duration | plan duration in nanoseconds |
| 24 | 8 | local memory capacity | measured local `ByteCount` |
| 32 | 8 | local memory rate | measured local `BytesPerSecond` |

The begin token is `round << 32 | 1 << 24 | (iteration + 1)`, so it is
`round << 32 | 0x01000000` for the begin exchange. The peer's begin frame must
use the same token, exact plan, expected duplex, schema, reserved byte, and a
nonzero valid memory capacity/rate. A disagreement is an invalid frame or
unsupported version, not a nominal fallback.

`ProbeReady` has an empty payload and token phase `2`. Both sides send and
receive it before data. `ProbeData` uses token phase `3` and iteration values
starting at one. The payload is exactly the configured buffer length and is
filled with one byte, `round.to_le_bytes()[0] ^ 0xa5`. The receiver checks every
byte against this pattern in addition to the outer SHA-256 digest.

For full duplex, a two-party barrier starts a scoped receiver thread and the
local sender together. For half duplex, both peers compare their machine
digest bytes; the lexicographically smaller endpoint sends all iterations
first and then receives, while the other receives first and then sends. This
is why the resulting evidence records `Simultaneous` for full duplex and
`Serialized` for half duplex.

Each direction has a `DirectionAccumulator`: it records a monotonic sample
count, minimum, maximum, integer mean, integer variance, and elapsed time from
the accumulator start. Nanoseconds are canonicalized to at least one and must
fit `u64`. A zero-sample result, duration overflow, sum overflow, or variance
overflow is a probe error. The measured rate is
`total_bytes * 1_000_000_000 / elapsed_nanoseconds`, calculated with checked
`u128` arithmetic and clamped to the valid `BytesPerSecond` range of one to
`u64::MAX`.

`ProbeComplete` has exactly 16 payload bytes, both big endian `u64` rates:
outbound at offset 0 and inbound at offset 8. Its token uses phase `4`. The
receiver validates that both values are valid `BytesPerSecond` values, but it
does not replace its own measured evidence with the peer's rates.

After completion, `TcpPeerSession` returns measured remote memory capacity and
rate, measured outbound and inbound rates, endpoint evidence copied from the
authenticated `SessionIdentity`, the protocol schema, and the directional
statistics. The caller-side `cluster` checks that endpoint evidence, execution
mode, total bytes, sample count, elapsed bounds, min/mean/max ordering, and
rate derivation are all consistent with the measured plan before assembly.

## Remote message framing inside runtime frames

The remote codec (`remote/src/codec.rs`) encodes one message in one runtime
frame payload. The inner header is 28 bytes and uses little endian integers:

| Offset | Size | Field | Encoding |
| ---: | ---: | --- | --- |
| 0 | 8 | magic | ASCII `RCPREM01` |
| 8 | 2 | protocol version | `u16`, must equal `2` |
| 10 | 1 | message tag | `1` through `27` below |
| 11 | 1 | reserved | must be `0` |
| 12 | 8 | remote lane sequence | starts at `0` independently for each lane and run |
| 20 | 8 | run ID | nonzero `RunId`, exact for this session |

`Writer` and `Reader` write/read all integer fields little endian and reject
buffer overrun, range overflow, truncation, and trailing bytes. The outer
transport payload length is the only length framing for a remote message.
There is no inner checksum because the outer frame digest covers the complete
remote payload.

The tag and body ABI is:

| Tag | Message | Lane | Body fields, in order |
| ---: | --- | --- | --- |
| 1 | `Hello` | control | role `u8`; capability bits `u64`; local machine digest; local profile digest; remote machine digest; remote profile digest; eight `u64` remote limits |
| 2 | `Manifest` | control | bundle digest; draft digest; realization digest; manifest digest; program digest; artifact count `u32`; then `artifact_count` records of artifact ID `u64` and artifact digest |
| 3 | `ManifestAck` | control | manifest digest; program digest |
| 4 | `Prepare` | control | no body |
| 5 | `PrepareAck` | control | no body |
| 6 | `InitBegin` | control | device ID `u64`; logical image bytes `u64`; image SHA-256 digest |
| 7 | `InitChunk` | user data | device ID `u64`; image offset `u64`; chunk length `u32`; chunk bytes |
| 8 | `InitEnd` | control | device ID `u64`; logical image bytes `u64` |
| 9 | `InitAck` | control | device ID `u64` |
| 10 | `InitComplete` | control | no body |
| 11 | `InitCompleteAck` | control | no body |
| 12 | `Execute` | control | task ID `u64` |
| 13 | `TaskComplete` | control | task ID `u64` |
| 14 | `TaskFailed` | control | task ID `u64`; driver fault code `u32`; driver fault detail `u64` |
| 15 | `DataRequest` | control | task ID `u64` |
| 16 | `UserData` | user data | task ID `u64`; byte length `u32`; data bytes |
| 17 | `DataAck` | control | task ID `u64` |
| 18 | `Metric` | metrics | task ID `u64`; scalar kind `u8` (`1` f32, `2` i32); scalar bits `u32` |
| 19 | `Cancel` | control | cancellation reason code `u32` |
| 20 | `CancelAck` | control | no body |
| 21 | `BeginExit` | control | no body |
| 22 | `ExitReady` | control | no body |
| 23 | `Release` | control | device ID `u64` |
| 24 | `ReleaseAck` | control | device ID `u64` |
| 25 | `ExitComplete` | control | no body |
| 26 | `ExitAck` | control | no body |
| 27 | `DriverFault` | control | driver fault code `u32`; driver fault detail `u64` |

Digest fields are exactly 32 bytes. Fixed remote message lengths, excluding
variable artifact/chunk/data bytes, are the 28-byte header plus the body:
Hello is 229 bytes, Manifest is `192 + 40 * artifact_count`, ManifestAck is
92, no-body messages are 28, InitBegin is 76, InitChunk is `48 + chunk_len`,
InitEnd is 44, InitAck/Release/ReleaseAck are 36, Execute/
TaskComplete/DataRequest/DataAck are 36, TaskFailed is 48, UserData is
`40 + data_len`, Metric is 41, Cancel is 32, and DriverFault is 40.

The codec rejects an unknown tag or role, a nonzero reserved byte, a version
mismatch, an unknown metric scalar kind, an over-limit artifact count, a
truncated variable field, a field that does not fit the host, or any trailing
bytes. Capability bits are decoded as an opaque `u64`; Hello validation
requires all six `Capabilities::REQUIRED` bits but permits additional bits.

`Message::lane` assigns `InitChunk` and `UserData` to `RuntimeLane::UserData`,
`Metric` to `RuntimeLane::Metrics`, and every other message to
`RuntimeLane::Control`. A frame whose outer kind and decoded message lane do
not agree is a remote protocol error. Probe frame kinds cannot reach this
decoder: `frame_lane` rejects them first.

`SessionCore::try_send` selects the message lane, encodes with that lane's
current inner sequence and the active run ID, and submits the complete encoded
bytes to the corresponding fixed RuntimeChannel lane. A lane-capacity result
returns `false`/`None` so the typed state can retry later; every other transport
failure poisons the remote session. The inner per-lane transmit sequence is
incremented when the slot is accepted, before the outer frame is necessarily
flushed. `received` increments the matching receive sequence after checking
the decoded run, lane, and expected sequence, then translates the outer
schedule relative to the active run base. A schedule below that base is a
protocol error, not a schedule reset.

## Remote identity, capabilities, manifest, and program proof

`RemoteIdentity` repeats the outer transport's local and remote endpoint
identities and rejects equal machine digests. `SessionCore::new` requires the
remote identity to equal `RuntimeChannel::identity()` exactly. It also
requires a nonzero `RunId`, a valid manifest, a valid init schedule, and a
strictly greater run ID than the previous run when a `RemoteChannel` is reused.
The wrapper retains the last run while `RuntimeChannel` retains the outer
sequences and schedule positions.

Hello capability bits are:

| Bit | Capability |
| ---: | --- |
| 0 | `CHUNKED_INIT` |
| 1 | `BIDIRECTIONAL_DATA` |
| 2 | `PREPARED_EXECUTION` |
| 3 | `METRICS` |
| 4 | `CANCELLATION` |
| 5 | `EXACT_RELEASE_ACK` |

Both peers send `Capabilities::REQUIRED`, the OR of these six bits. On
receipt, `validate_hello` requires the expected peer role, all required bits,
the exact opposite local/remote machine and profile digests, and an exact
match for all eight fixed limits. It does not negotiate a subset or choose a
larger limit.

`Manifest::from_bundle` checks the artifact count and encoded size against the
limits, sorts artifacts by ID, rejects duplicate IDs, sets protocol `2`, and
computes a digest over this little-endian sequence:

```text
"recipe-remote-manifest-v1"
protocol u16
bundle digest
draft digest
realization digest
artifact count u64
for each sorted artifact: artifact ID u64, artifact digest
```

Manifest validation rejects a protocol mismatch, zero plan identities, a
noncanonical/non-strict artifact order, a zero artifact digest, or a digest
that does not recompute. The wire `Manifest` repeats the five plan/program
proof digests and the canonical artifact records. The worker's
`WireManifest::matches_proof` independently checks every field, each artifact
ID/digest, and the same manifest hash. The master accepts a `ManifestAck` only
if both the manifest and program digests equal its own.

`ProvisionedProgram` is the static contract that makes later messages
meaningful. Construction verifies the bundle topology identity and topology
scheduling properties, canonicalizes nonzero unique worker device IDs, and
records each device's exact init image byte count. It classifies every task as
master-owned, worker driver work, a cross transfer, or an init image. Init
phase worker work and cross transfers must be represented by the single device
image and cannot appear as runtime tasks. Non-init cross transfers must be
planner-expanded one-hop tasks whose route, link direction, endpoint devices,
lane claim, byte count, and duplex mode match the measured topology.

The program digest is calculated over the manifest digest, sorted worker
device IDs and image sizes, sorted task IDs with phase (`Init=1`, `Loop=2`,
`Exit=3`) and kind (`Driver=1`, `CrossTransfer=2`), then sorted cross-transfer
records containing task ID, direction (`MasterToWorker=1`,
`WorkerToMaster=2`), schedule, bytes, and each capacity claim's resource and
mode (`Half=1`, `Full=2`). This digest is not sent as a separate wire
message, but is carried in `Manifest` and `ManifestAck` and is checked before
`Prepare`.

Cross-transfer schedules are assigned after sorting by direction, planner
window start, and task ID. Master-to-worker schedules begin at the total init
chunk count; worker-to-master schedules begin at zero. Each direction is
strictly increasing. Every transfer must fit the user-data codec bound. A
duplicate task, over-limit task/device/transfer count, unknown device, invalid
route, duplicate capacity resource, zero task/resource/byte count, or schedule
sentinel collision is rejected during provisioning rather than interpreted at
runtime.

## Lifecycle message order

The typed remote API permits the following graph and no other transition:

```text
MasterHandshake <-> WorkerHandshake
       | PrepareAck                | PrepareAck
       v                           v
    MasterInit                 WorkerInit
       | InitCompleteAck          | InitCompleteAck
       v                           v
     MasterRun  <--------------> WorkerRun
       | begin_exit                  | BeginExit
       v                             v
    MasterExit  <--------------> WorkerExit
       | ExitAck                     | ExitAck
       v                             v
   MasterComplete              WorkerComplete

MasterRun --cancel--> MasterCancelling <--> WorkerCancelled --> Complete
```

The arrows are a message sequence, not a second transport. Each `progress`
method first advances `RuntimeChannel`, consumes at most one ready frame, and
then attempts the next legal outbound message. A frame is released only after
its state machine has validated and consumed it. Duplicate or out-of-order
messages return a remote protocol error; most transport, codec, run, lane,
sequence, and identity violations poison the session so no later message can
be accepted.

### Symmetric handshake

The master sends `Hello(role=Master)`. The worker sends
`Hello(role=Worker)`. Each side validates the other's role, capabilities,
identities, and limits. After receiving the worker Hello, the master sends
`Manifest`; after validating it, the worker sends `ManifestAck` with its
manifest and program digests. The master checks both digests and sends
`Prepare`. The worker may call `WorkerDriver::prepare(run, program)` only in
response to that message. If preparation succeeds it sends `PrepareAck` and
both sides enter their init state. A Hello, Manifest, or Prepare received
before its predecessor, a duplicate, or a DriverFault is terminal for the
handshake.

The worker can report a driver fault at any point through tag 27. Before the
fault is submitted, `begin_driver_fault` calls `cleanup_after_fault`; a
cleanup failure is encoded as `FATAL_CLEANUP_FAILED` with the primary and
cleanup codes packed into the detail. The report is held until its transport
completion is transmitted, then the worker returns a terminal driver error.

### Init image admission

The master queues one exact image at a time. `queue_image` requires a
provisioned device in `Needed` state and a byte length exactly equal to the
finalized arena image. It computes SHA-256, sends `InitBegin`, then sends
one `InitChunk` at a time. The chunk size is the remote scratch capacity minus
48 bytes. Each chunk uses a user-data schedule and is not considered consumed
until the outer transport reports its completion token. After the final chunk,
the master sends `InitEnd` and waits for the matching `InitAck`. Once every
device is complete, it sends `InitComplete` and waits for
`InitCompleteAck` before entering `MasterRun`.

The worker accepts only one active image, one pending chunk, and one pending
final image admission. `InitBegin` must name a needed provisioned device and
the exact image size. Each `InitChunk` must carry the next logical schedule,
the active device, the exact next offset, and a range within the image. The
driver receives a stable copied chunk and must report the exact same byte
count. The worker hashes completed chunks in order. `InitEnd` must name the
active device, exact size, and complete received range; the final SHA-256 must
equal the begin digest. The driver then completes final image admission
before the worker marks the device complete and sends `InitAck`.

`InitComplete` is accepted only when every device is complete and no image,
chunk, or acknowledgment is pending. It causes `InitCompleteAck`, and only
after that acknowledgment is submitted does the worker enter `WorkerRun`.
Overlapping images/chunks, unknown or duplicate devices, wrong offsets,
schedule replay, digest mismatch, incomplete image bytes, wrong driver byte
counts, and init messages in any other state are protocol failures.

### Run task and transfer messages

The master can submit a provisioned worker-driver task with `Execute`, send a
master-to-worker cross transfer with scheduled `UserData`, or request a
worker-to-master transfer with `DataRequest`. Every operation checks the
static task phase, kind, direction, byte count, current `Idle` status, and any
half-duplex capacity claims. A half-duplex resource is represented by exactly
one token and cannot be acquired by two active transfers. A failed lane
submission releases the claim and returns backpressure.

The worker handles `Execute` by submitting the exact task to its
`WorkerDriver`, then polls fixed task slots round-robin. A completed task with
a metric sends `Metric` first and waits until that frame is transmitted before
sending `TaskComplete`. A task without a metric sends `TaskComplete` directly.
A native task failure sends `TaskFailed` with its `DriverFault`. The master
accepts these only for an active task in the current phase and releases a
cross-transfer claim when the task reaches complete or failed status.

For a master-to-worker `UserData`, the worker checks direction, logical
schedule, byte count, phase, and idle status, copies the bytes into bounded
storage, calls `begin_receive_user_data`, polls until the exact byte count is
reported, then sends `TaskComplete`. For a worker-to-master `DataRequest`, it
calls `begin_produce_user_data` into bounded storage, polls the exact byte
count, sends scheduled `UserData`, and waits for `DataAck`. Only after
`DataAck` does it call `user_data_acked`, clear the buffer, mark the transfer
complete, and release its half-duplex claim. The master stores one incoming
worker data frame in a fixed data slot and sends its `DataAck` before reporting
the task complete. The caller must call `release_data(slot)` to return that
data slot; until then a later worker-to-master frame can be backpressured. It
stores metrics in fixed metric slots, and the caller must call `take_metric`
to clear a metric slot. A full data inbox or metric inbox is therefore a
bounded backpressure result, not an overwrite.

Full-duplex transfer claims may progress independently. Half-duplex claims
for the same finalized resource are serialized by the master and worker token
stores. The runtime lane itself still remains a single outer stream with the
control, user-data, and metrics queue priority described above. The queue has
no fairness rotation: a continuously occupied higher-priority lane can delay
lower-priority frames until its slots are released.

### Normal exit

The master cannot call `begin_exit` until every loop task is complete. It sends
`BeginExit` and waits for `ExitReady`. The worker accepts it only after all
loop work is complete, calls `WorkerDriver::begin_exit`, sends `ExitReady`, and
continues accepting exit-phase `Execute`, `UserData`, and `DataRequest`
messages.

When exit-phase tasks and data acknowledgments are complete, the master sends
one `Release { device }` at a time. The worker requires exit work to be
complete and the device to be in `Needed` release state, calls
`release_arena`, and marks it `Requested`. It sends exactly one matching
`ReleaseAck`; the master marks that device `Complete` and advances to the
next device. Only after every exact release acknowledgment does the master
send `ExitComplete`. The worker accepts it only with complete exit work and
all release states complete, calls `finish`, sends a tracked `ExitAck`, and
returns `WorkerComplete` after that frame is flushed. The master enters
`MasterComplete` only after receiving the matching `ExitAck`.

An unknown, duplicate, or out-of-order release acknowledgment, an early
`ExitComplete`, or an exit task/data message with the wrong phase is a remote
protocol failure. `MasterExit::is_complete` is true only in its `Done` state;
`into_complete` rejects every earlier state.

### Cancellation

`MasterRun::cancel` and `MasterExit` cancellation move the master into
`MasterCancelling`, where it sends one `Cancel { reason }`. The worker accepts
Cancel in run or exit processing, calls `WorkerDriver::cancel(reason)`, and
enters `WorkerCancelled`. The worker then releases every provisioned arena in
order, sending one tracked `ReleaseAck` after each local `release_arena` call,
calls `finish`, and sends a tracked `CancelAck`. The master accepts ordinary
task/data/metric progress while cancellation is draining, records each exact
release acknowledgment, and enters cancelled `MasterComplete` only after
`CancelAck` and all releases are complete. No alternate cancellation or
forced socket path exists.

## Error and poison behavior

The transport error enum is `TransportError` in `transport/src/error.rs`.
These are the actual variants and their boundaries:

| Error | Produced by | Meaning |
| --- | --- | --- |
| `InvalidConfiguration(&'static str)` | constructors, schedule/token checks, probe plan checks, runtime state checks | Caller supplied a value outside the static contract, such as zero capacity, zero timeout, equal machines, an exhausted schedule, or an unbounded plan. |
| `InvalidIdentity(&'static str)` | `EndpointIdentity::new` | Machine or profile digest is zero. |
| `UnsupportedVersion(u16)` | outer header or probe begin decode | The peer's version/schema is not the one compiled into this crate. |
| `InvalidFrame(&'static str)` | metadata/header/probe validation | Magic, flags, kind, schedule, payload shape, duplex, plan, or phase bytes are invalid. |
| `FrameTooLarge { declared, limit }` | sender/runtime submit/header decode | Declared or supplied payload exceeds the configured bound. |
| `BufferTooSmall { required, available }` | blocking receive | The caller's supplied receive buffer cannot hold the declared payload. |
| `UnexpectedSequence { expected, received }` | outer header decode | Direction-local outer frame sequence is missing, replayed, or out of order. |
| `UnexpectedIdentity` | outer header decode | Sender or receiver digest fields do not match the prevalidated session. |
| `IntegrityFailure` | blocking/runtime payload digest or probe pattern check | SHA-256 differs, or a probe data byte differs from the round pattern. |
| `Cancelled` | controlled probe checks | Caller cancellation flag is set between operations. |
| `DeadlineExceeded` | deadline helpers or I/O timeout mapping | The absolute/operation deadline elapsed, including a `TimedOut` or `WouldBlock` system result in blocking I/O. |
| `ConnectionClosed` | zero-byte blocking/nonblocking read or write | The peer or local socket returned EOF/zero progress. |
| `Io(io::ErrorKind)` | other I/O | The system call failed with an unhandled I/O kind. |
| `ProtocolState(&'static str)` | sequence/token/schedule/slot/probe state | Local state cannot legally perform the requested transition or a monotonic counter overflowed. |
| `CapacityExhausted(&'static str)` | fixed lanes, probe data, or runtime storage | No fixed slot/buffer/capacity is available. |
| `UnknownCompletion(u64)` | runtime completion lookup | The token has no live transmit slot. |
| `Probe(String)` | probe statistics/rate conversion | Duration, arithmetic, or measured-result construction failed. |
| `Poisoned` | any operation after a fatal channel error | The sender, receiver, runtime channel, or mutex cannot be reused. |

`WireSender` and `WireReceiver` poison themselves on any failed I/O, header,
buffer, sequence, identity, or digest operation and call `shutdown(Both)`. A
`RuntimeChannel` marks itself poisoned when either pass of `progress` fails.
Remote `SessionCore` converts transport errors to `RemoteError::Transport` and
poisons the remote session for transport, decode, run, lane, and sequence
violations. There is no reconnect, retry, alternate codec, or nominal-rate
fallback in this code.

The remote error enum (`remote/src/error.rs`) adds these exact categories:

| Error | Meaning in the remote layer |
| --- | --- |
| `InvalidConfiguration` | Invalid identity, run, limits, manifest/program, or lifecycle construction input. |
| `ManifestMismatch` | Manifest protocol, digest, topology, artifact, image, or content proof differs. |
| `Protocol` | A message is wrong for the active typestate, lane, phase, task status, direction, schedule, or release state. |
| `RunMismatch { expected, actual }` | Inner message RunId differs from the active run. |
| `UnknownTask` / `DuplicateTask` | A task is absent or appears more than once in the provisioned program. |
| `UnknownDevice` / `DuplicateDevice` | An image/release names an absent or already-used device. |
| `CapacityExhausted` | A codec buffer, fixed slot, manifest, task/device/transfer bound, or scratch area cannot hold the operation. |
| `Backpressured` | A fixed runtime lane, half-duplex token, data inbox, or other bounded resource is currently occupied. |
| `Codec` | Inner magic/version/reserved/tag/role/scalar/length/trailing-byte validation failed. |
| `Driver` | The worker's `WorkerDriver` returned a code/detail fault. |
| `Transport` | An underlying `TransportError`. |
| `Poisoned` | The remote session was explicitly made unusable after a terminal violation. |

Backpressure is exposed to the caller where the state machine can retry the
same logical submission after a later `progress`; it never silently changes
the message or bypasses the lane. A driver fault is distinct from protocol
failure. The worker first calls `cleanup_after_fault`, then flushes one
`DriverFault` frame; ordinary terminal receipt therefore means native work was
already quiesced. If cleanup itself fails, the combined fault uses the reserved
`FATAL_CLEANUP_FAILED` code.

## WorkerDriver and native executor boundary

The remote protocol supplies only final IDs, digests, phase, and bounded byte
slices to `WorkerDriver`. The trait has no host-calculation callback, compiler,
discovery, file, or vendor-math operation. Its methods are called by the
state transitions above:

| Lifecycle point | Driver call |
| --- | --- |
| handshake `Prepare` | `prepare(run, program)` |
| image begin/chunks/final admission | `begin_init_image`, `begin_init_chunk`, `poll_init_chunk`, `finish_init_image`, `poll_init_image` |
| driver task | `submit_task`, repeated `poll_task` |
| master-to-worker data | `begin_receive_user_data`, repeated `poll_receive_user_data` |
| worker-to-master data | `begin_produce_user_data`, repeated `poll_produce_user_data`, then `user_data_acked` |
| cancellation | `cancel(reason)` |
| exit | `begin_exit`, `release_arena(device)` for each exact device, `finish` |
| terminal fault | `cleanup_after_fault(primary)` |

`ExecutorWorkerDriver` is the concrete adapter. Construction derives a
`WorkerProjection` and proves its devices, image sizes, non-init task roles,
cross-transfer directions/bytes/routes, and duplex resources equal the
`ProvisionedProgram`. It rejects a projection mismatch before any handshake.
`prepare` pre-realizes the native worker execution session. Its live calls
translate directly to the executor's nonblocking task and external-transfer
operations, and its cleanup path calls native fatal cleanup before returning a
terminal fault.

## Invariants that connect the layers

The following properties are enforced by the current callers and callees:

* Every outer frame has one authenticated sender/receiver pair, one global
  direction-local sequence, one nonzero completion token, and exactly one
  payload digest.
* Probe frames are blocking-only and cannot be accepted by RuntimeChannel;
  runtime frames are nonblocking-only and cannot be accepted by the probe
  `receive_exact` phase machine.
* Every runtime user-data frame has a finite, strictly increasing schedule;
  every control or metric frame is unscheduled. Remote logical schedules are
  translated onto connection-global positions rather than reset on reuse.
* The outer transport sequence is global per direction, while the remote
  codec sequence is independently monotonic for control, metrics, and user
  data. A run reset never resets the outer sequence.
* Completion storage is explicit. A transmitted slot remains occupied until
  `release_completion`; one received payload remains borrowed until
  `release_received`.
* Runtime lane capacity is fixed at construction. Control has priority over
  user data, and user data has priority over metrics when choosing the next
  queued transmit slot.
* Probe measurements are accepted only when both peers agree on schema, plan,
  duplex, tokens, pattern, completion shape, and endpoint identities; profile
  code then independently checks measured evidence and derived rates.
* Remote handshake proves exact roles, capabilities, endpoint/profile
  identities, eight fixed limits, manifest digest, artifact list, and program
  digest before `prepare`.
* Init image bytes are checked by exact size, sequential offsets, per-chunk
  schedules, and SHA-256. A final `InitComplete` is legal only after every
  device's image admission and acknowledgment has completed.
* Tasks and cross transfers are accepted only from the immutable provisioned
  manifest, in their declared `RunPhase`, direction, byte count, schedule,
  and capacity claims. No task ID or transfer contract is inferred from the
  wire.
* Full-duplex resources can overlap; each half-duplex capacity resource has
  one ownership token and is released only when its exact task reaches the
  matching acknowledgment/completion state.
* Normal exit and cancellation release every worker arena exactly once and
  require one exact `ReleaseAck` per device before their terminal message.
* A terminal native fault is sent only after local cleanup. Protocol,
  identity, integrity, sequence, run, and transport failures are visible as
  errors and poison the relevant session; no fallback implementation masks
  them.

## End-to-end communication walkthrough

For a probe, the caller obtains a connected stream and two nonzero endpoint
digests, constructs `TcpPeerSession`, and invokes `ProbeEngine`. Both peers
exchange 40-byte begin payloads, then empty ready frames, then the exact number
of patterned data frames, then 16-byte completion payloads. The session returns
only measured directional evidence. `ProbeEngine` validates and stores that
evidence in a profile; cluster assembly later derives transport endpoint
identity from the stable machine ID and canonical profile digest.

For runtime execution, the caller builds the same `RuntimeChannel` contract at
both ends and wraps it in `RemoteChannel`. A nonzero monotonically increasing
run enters symmetric Hello/Manifest/Prepare. The master then streams each
worker device image, waits for exact acknowledgments, and both sides enter the
run. During loop and exit phases, each public submission becomes one inner
message, one runtime lane submission, one 200-byte authenticated outer header,
and one payload digest. `progress` drives the nonblocking socket and returns
typed task, data, metric, cancellation, or exit events only after the complete
frame has arrived and the inner sequence/run/lane/static contract has passed.

When the phase finishes, the same channel executes the explicit release and
terminal acknowledgment sequence. The completed wrapper returns a
`RemoteChannel` carrying the prior run ID; a later session on the same
connection must use a strictly larger run and starts its logical schedules at
the channel's next connection-global positions. The transport never owns the
socket lifecycle beyond configuring, reading, writing, and shutting down a
failed stream.

## Source map for the observed implementation

The complete wire behavior is split across these concrete symbols:

* `transport/src/protocol.rs`: constants, identities, tokens, frame kinds,
  metadata validation, header encode/decode, blocking stream split, digest,
  and deadline I/O.
* `transport/src/runtime.rs`: lane capacities, preallocated transmit/receive
  storage, nonblocking progress, completion ownership, sequence and schedule
  state.
* `transport/src/probe.rs`: `TcpPeerSession`, phase deadlines, begin/ready/data/
  complete payloads, full/half duplex measurement, statistics, and conversion
  to `PeerMeasurement`.
* `transport/src/error.rs`: all transport error variants and display text.
* `remote/src/codec.rs`: the 28-byte inner header, tags 1 through 27, manual
  little-endian reader/writer, manifest proof, and message/lane mapping.
* `remote/src/model.rs`: capabilities, remote limits, manifests, provisioned
  program digest, task ownership, cross-transfer schedules, and bounds.
* `remote/src/session.rs`: `SessionCore`, handshake/init/run/exit/cancel
  typestates, fixed storage, message order checks, and release/fault handling.
* `remote/src/driver.rs` and `remote/src/executor_driver.rs`: the bounded
  native worker boundary and fault cleanup contract.
* `probe/src/model.rs` and `probe/src/engine.rs`: controlled attempt phases,
  cancellation/deadline ownership, and the rule that only measured peer
  evidence enters a profile.
* `cluster/src/model.rs`: endpoint/profile evidence binding and independent
  validation of measured peer rates before cluster assembly.

No other listener, codec, retry path, rate fallback, or transport protocol is
implemented by these crates.
