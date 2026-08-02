# Remote protocol codec

This document describes the implementation in `remote/src/codec.rs` and the
session code that consumes it. The codec is an internal, manual byte codec for
`recipe-remote`. It is not a socket implementation, a manifest store, or a
driver API. Its output is the payload submitted to an already constructed
`recipe_transport::RuntimeChannel`.

The important source boundaries are:

| Source | Responsibility |
| --- | --- |
| `remote/src/codec.rs:12-684` | Remote payload constants, tags, message values, encoding, decoding, bounded writer and borrowed reader. |
| `remote/src/model.rs:81-190` | `RemoteLimits`, its eight-value wire tuple, manifest and runtime capacity validation. |
| `remote/src/model.rs:267-366` | Canonical manifest construction, digest calculation, and validation. |
| `remote/src/model.rs:388-568` | Provisioned program digest and the task/device/transfer contracts used by the session. |
| `remote/src/session.rs:407-730` | `SessionCore`, codec call sites, lane checks, per-lane sequence checks, run checks, and hello validation. |
| `remote/src/session.rs:734-921` | Master and worker handshake state machines. |
| `remote/src/session.rs:923-1487` | One-device-at-a-time chunked init image exchange. |
| `remote/src/session.rs:1489-2446` | Runtime task, metric, and bidirectional user-data exchange. |
| `remote/src/session.rs:2448-3135` | Exit, cancellation, exact arena release, terminal acknowledgments, and terminal driver faults. |
| `transport/src/protocol.rs` and `transport/src/runtime.rs` | The outer frame, endpoint identity, transport sequence, payload digest, nonblocking lanes, completion tokens, and schedule metadata. |

## The two wire layers

The remote codec writes a complete payload beginning at offset zero. The
transport then puts that payload in a transport frame. The layers have separate
versions, magic values, sequences, and integrity responsibilities.

* The remote payload begins with the eight bytes `RCPREM01`.
* `recipe_transport` wraps it in a 200-byte frame headed by `RCPTRN01`.
* The transport header carries endpoint machine and profile identities,
  transport frame kind, one connection-global sequence, a completion token,
  an optional user-data schedule stamp, payload length, and a SHA-256 payload
  digest. It is responsible for rejecting a bad transport magic or version,
  wrong endpoint identities, wrong transport sequence, a frame larger than its
  `ProtocolLimits`, a bad payload digest, a probe frame on a runtime channel,
  and a non-increasing connection-global user-data schedule.
* The remote payload carries a second sequence and the nonzero `RunId` used by
  the remote session. Remote sequences are independent per runtime lane, while
  the transport sequence is one stream for the connection.

`RuntimeChannel` maps `RuntimeLane::Control`, `RuntimeLane::Metrics`, and
`RuntimeLane::UserData` to transport frame kinds 16, 17, and 18. Its transmit
selection prefers the oldest control submission, then user data, then metrics.
The remote codec does not see the outer frame header or its digest. Conversely,
the transport does not interpret the remote tag or fields.

## Limits and allocation boundary

`RemoteLimits::new` requires `max_message_bytes` to be at least 256 and no
greater than `u32::MAX`. Every manifest and runtime capacity passed to the
constructor must be nonzero. The eight limits advertised in `Hello` are, in
order, the exact values returned by `RemoteLimits::wire_tuple()`:

1. maximum remote payload bytes;
2. maximum manifest artifacts;
3. maximum provisioned tasks;
4. maximum worker devices;
5. maximum cross-machine transfers;
6. worker task slots;
7. worker/master data slots;
8. metric slots.

The tuple is eight little-endian `u64` fields. The peer's tuple must compare
equal to the local tuple during `SessionCore::validate_hello`; a different
capacity is a protocol error, not a negotiated downgrade. Capability bits are
also checked there. Unknown capability bits are retained by
`Capabilities::from_bits`, but all six required bits must be present.

`SessionCore::new` allocates one `scratch` buffer of exactly
`max_message_bytes`. Every normal send encodes into that buffer and submits
only the returned prefix. `WorkerInit` and `WorkerStorage` allocate their
fixed-size chunk and data buffers during state construction. The codec itself
does not allocate: `Writer` writes into a caller-provided mutable slice and
`Reader` returns borrowed slices from the received payload. The session's
`ReceivedMessage` borrows the transport receive buffer until the matching
completion token is released.

The size helpers are source-of-truth for the surrounding model:

* `HEADER_BYTES` is 28, the remote payload envelope size.
* `ARTIFACT_BYTES` is 40, one artifact id (`u64`) plus one digest (32 bytes).
* `manifest_encoded_bytes(n)` returns `28 + 164 + 40*n`, or `None` on checked
  arithmetic overflow. The 164 bytes are five digests (160) and the artifact
  count (`u32`). `Manifest::from_bundle` rejects an artifact list over
  `max_artifacts` or a computed manifest larger than `max_message_bytes`.
* `USER_DATA_OVERHEAD` is `28 + 8 + 4 = 40`. The model rejects a cross transfer
  whose finalized byte count exceeds `max_message_bytes - 40`.
* `INIT_CHUNK_OVERHEAD` in `model.rs` is 48, consisting of the remote header,
  device id, offset, and chunk length. Init chunk capacity is therefore
  `max_message_bytes - 48`. `init_chunk_count` uses that value and reserves
  the resulting master-to-worker schedule positions before any scheduled
  cross transfer.

The generic encoder does not take a `RemoteLimits` argument. Its bound is
enforced by the destination slice, normally `SessionCore::scratch`; an
oversized write returns `CapacityExhausted("message codec buffer")`. The
specialized manifest encoder performs an explicit size preflight because it
knows the artifact count. The decoder independently rejects any input whose
total length exceeds `RemoteLimits::max_message_bytes`.

## Remote payload envelope

All integers in this payload are little-endian. Digests are copied as their
32 raw bytes. The first 28 bytes are always:

| Offset | Width | Field | Encoding and meaning |
| ---: | ---: | --- | --- |
| 0 | 8 | magic | ASCII bytes `RCPREM01`. |
| 8 | 2 | protocol | `PROTOCOL_VERSION`, currently 2, little-endian. |
| 10 | 1 | tag | Message tag 1 through 27. |
| 11 | 1 | reserved | Must be zero. The encoder always writes zero. |
| 12 | 8 | sequence | Sender's zero-based sequence for the message's runtime lane. |
| 20 | 8 | run | `RunId::get()`. The session validates the decoded value against its active run. |

The decoder checks magic, protocol, and reserved before reading the sequence,
run, and tag body. It constructs `RunId` and the typed ids directly from their
`u64` values. Those constructors do not reject zero; session/model validation
rejects zero where the remote contract requires a nonzero id.

`encode` writes the envelope, dispatches on `Message`, and returns the number
of bytes written. `tag` is a `const fn` with the one-to-one table below.
`decode` reads one body according to the tag, then requires the `Reader` to be
exactly at the payload end. Any trailing byte is rejected.

## Message tags, fields, lengths, lanes, and direction

The lengths in the table are remote payload lengths, including the 28-byte
envelope. `n` is the encoded byte count of a variable byte slice. A direction
is the direction used by the session state machines. The codec itself does not
reject a semantically reversed direction; the receiving typestate does.

| Tag | Variant and direction | Body field order | Body bytes | Total bytes | Lane |
| ---: | --- | --- | ---: | ---: | --- |
| 1 | `Hello`, both peers | `role:u8`, `capabilities:u64`, `local_machine:digest`, `local_profile:digest`, `remote_machine:digest`, `remote_profile:digest`, eight `limits:u64` | 201 | 229 | Control |
| 2 | `Manifest`, master to worker | `bundle:digest`, `draft:digest`, `realization:digest`, `manifest_digest:digest`, `program_digest:digest`, `artifact_count:u32`, then `artifact_count` repetitions of `id:u64`, `digest:digest` | `164 + 40n` | `192 + 40n` | Control |
| 3 | `ManifestAck`, worker to master | `manifest:digest`, `program:digest` | 64 | 92 | Control |
| 4 | `Prepare`, master to worker | no fields | 0 | 28 | Control |
| 5 | `PrepareAck`, worker to master | no fields | 0 | 28 | Control |
| 6 | `InitBegin`, master to worker | `device:u64`, `bytes:u64`, `digest:digest` | 48 | 76 | Control |
| 7 | `InitChunk`, master to worker | `device:u64`, `offset:u64`, `length:u32`, `bytes[length]` | `20+n` | `48+n` | UserData |
| 8 | `InitEnd`, master to worker | `device:u64`, `bytes:u64` | 16 | 44 | Control |
| 9 | `InitAck`, worker to master | `device:u64` | 8 | 36 | Control |
| 10 | `InitComplete`, master to worker | no fields | 0 | 28 | Control |
| 11 | `InitCompleteAck`, worker to master | no fields | 0 | 28 | Control |
| 12 | `Execute`, master to worker | `task:u64` | 8 | 36 | Control |
| 13 | `TaskComplete`, worker to master | `task:u64` | 8 | 36 | Control |
| 14 | `TaskFailed`, worker to master | `task:u64`, `fault.code:u32`, `fault.detail:u64` | 20 | 48 | Control |
| 15 | `DataRequest`, master to worker | `task:u64` | 8 | 36 | Control |
| 16 | `UserData`, either direction | `task:u64`, `length:u32`, `bytes[length]` | `12+n` | `40+n` | UserData |
| 17 | `DataAck`, master to worker | `task:u64` | 8 | 36 | Control |
| 18 | `Metric`, worker to master | `task:u64`, `scalar_kind:u8`, `bits:u32` | 13 | 41 | Metrics |
| 19 | `Cancel`, master to worker | `reason.code:u32` | 4 | 32 | Control |
| 20 | `CancelAck`, worker to master | no fields | 0 | 28 | Control |
| 21 | `BeginExit`, master to worker | no fields | 0 | 28 | Control |
| 22 | `ExitReady`, worker to master | no fields | 0 | 28 | Control |
| 23 | `Release`, master to worker | `device:u64` | 8 | 36 | Control |
| 24 | `ReleaseAck`, worker to master | `device:u64` | 8 | 36 | Control |
| 25 | `ExitComplete`, master to worker | no fields | 0 | 28 | Control |
| 26 | `ExitAck`, worker to master | no fields | 0 | 28 | Control |
| 27 | `DriverFault`, worker to master | `fault.code:u32`, `fault.detail:u64` | 12 | 40 | Control |

`Message::lane` is the canonical lane mapping. Only `InitChunk` and `UserData`
use the user-data lane. Only `Metric` uses the metrics lane. Every other tag,
including handshake, lifecycle, task, fault, and acknowledgment messages,
uses control. A message received in a transport frame whose lane differs from
this mapping poisons the session.

The metric scalar kind is 1 for `RemoteMetricValue::F32` and 2 for
`RemoteMetricValue::I32`. F32 is encoded with `f32::to_bits`. I32 is encoded
by preserving its two's-complement bit pattern in a little-endian `u32`.
Decode rejects any other scalar kind. A cancel reason is an opaque `u32`; the
codec does not assign meaning to it.

## Encoding implementation

`Writer` has only a mutable byte slice and a cursor. `bytes` uses checked
addition, obtains the exact cursor range with `get_mut`, copies the source, and
advances the cursor. The typed methods (`u8`, `u16`, `u32`, `u64`, and
`digest`) only construct little-endian bytes and call `bytes`. Consequently:

* arithmetic overflow in a variable write returns
  `RemoteError::Codec("message length overflowed")`;
* a destination that cannot hold the next field returns
  `RemoteError::CapacityExhausted("message codec buffer")`;
* `Message::Manifest` rejects an artifact count that does not fit `u32`;
* `Message::InitChunk` rejects a chunk length that does not fit `u32`;
* `Message::UserData` rejects a data length that does not fit `u32`.

The fixed-width fields have no alignment or host-layout dependence. There is
no serde representation, padding, length prefix outside the fields shown in
the table, or implicit string encoding.

`encode_hello` builds a `WireHello` from the supplied `RemoteIdentity`, sets
`Capabilities::REQUIRED`, copies the local and remote endpoint/profile
digests, and copies `RemoteLimits::wire_tuple()`, then calls `encode` with the
control sequence supplied by `SessionCore`.

`encode_manifest` is a specialized path called by
`SessionCore::try_send_manifest`. It precomputes the exact required size,
checks the destination, writes the common envelope with tag 2, and writes the
canonical local manifest fields followed by each sorted artifact id and digest.
It does not serialize the `Manifest.protocol` field as a separate body field:
the common protocol field in the 28-byte envelope is the wire protocol, and
the manifest digest covers the manifest protocol value as described below.

## Decoding implementation

`decode(payload, limits)` performs these checks in order:

1. total payload length is at most `limits.max_message_bytes()`;
2. the eight-byte remote magic equals `RCPREM01`;
3. the little-endian protocol equals `PROTOCOL_VERSION`;
4. the reserved byte is zero;
5. the tag is known and its body is fully available;
6. any tag-specific enum or scalar value is valid;
7. the reader is exactly at the payload end.

`Reader::take` uses checked cursor arithmetic and a borrowed range. A range
overflow returns `Codec("message range overflowed")`; a missing range returns
`Codec("message is truncated")`. The typed reads use `array` and return
`Codec("fixed-width field is truncated")` only if the fixed-width conversion
itself fails after a successful take. The normal short-input path is therefore
the `message is truncated` error. Any unconsumed suffix returns
`Codec("message has trailing bytes")`.

Tag-specific decode behavior is:

* tag 1 decodes role 1 as `Master` and role 2 as `Worker`; any other role is
  `Codec("unknown peer role")`. Capabilities are decoded as raw bits, and all
  eight limits are read without local clamping.
* tag 2 converts the count to `usize`, requires it to be no greater than the
  configured artifact limit, checks `count * 40` with checked arithmetic, and
  borrows exactly that many artifact bytes. It does not parse individual
  artifact ids or digests in the codec; `WireManifest::matches_proof` parses
  those 40-byte records during worker handshake validation.
* tag 7 and tag 16 convert their `u32` lengths to `usize` and borrow exactly
  that many bytes. A conversion failure reports the corresponding
  `...does not fit this host` codec error.
* tags 6, 8, 9, 12, 13, 15, 23, and 24 construct typed ids directly from
  `u64`. Zero is not rejected by the codec itself.
* tag 14 and tag 27 decode a `DriverFault` as a `u32` code plus a `u64`
  detail.
* tag 18 accepts only scalar kinds 1 and 2. Unknown kinds return
  `Codec("unknown metric scalar type")`.
* an unknown tag returns `Codec("unknown message tag")`.

The decoder has no fallback tag, alternate endian mode, version downgrade, or
partial-message result. It returns one `Decoded { sequence, run, message }`
only after the whole payload is consumed.

## Manifest proof and codec callers

The manifest has two related forms. `Manifest::from_bundle` creates a local
canonical value, while `encode_manifest` writes its wire form. The local value
contains `protocol`, bundle/draft/realization identities, sorted artifacts,
and a digest. `Manifest::compute_digest` hashes, in order:

```text
"recipe-remote-manifest-v1"
manifest.protocol as little-endian u16
bundle digest
draft digest
realization digest
artifact count as little-endian u64
for each sorted artifact: id as little-endian u64, artifact digest
```

The worker receives a borrowed `WireManifest`. `WireManifest::matches_proof`
first compares the five plan/program identities and the artifact count with
the worker's already provisioned values. It then parses each fixed 40-byte
record, rejects id zero, non-increasing ids, or an all-zero artifact digest,
recomputes the same domain-separated manifest digest, and compares it with the
wire `manifest_digest`. This is the semantic check after byte decoding.

`Manifest::validate`, called by both `MasterHandshake::new` and
`WorkerHandshake::new` through `SessionCore::new`, independently checks the
local manifest protocol, nonzero plan identities, canonical artifact ordering,
nonzero artifact digests, and the local digest.

The codec's direct callers are intentionally few:

| Caller | Codec call | What the caller supplies or checks |
| --- | --- | --- |
| `model::Manifest::from_bundle` | `manifest_encoded_bytes` | Artifact count and max-message preflight. |
| `model::schedule_cross_transfers` | `USER_DATA_OVERHEAD` | Maximum finalized cross-transfer payload. |
| `model::init_chunk_count` and `session::validate_init_schedule` | `INIT_CHUNK_OVERHEAD` | Chunk count and reserved init schedule positions. |
| `SessionCore::try_send_tracked` | `encode` | Per-lane sequence, active run, lane submission, optional logical schedule. |
| `SessionCore::try_send_hello` | `encode_hello` | Role, endpoint identity, required capabilities, fixed limits. |
| `SessionCore::try_send_manifest` | `encode_manifest` | Local manifest and program digest. |
| `SessionCore::received` | `decode` | Frame payload, active limits, then run/lane/sequence validation. |

No other crate calls the private codec module. The public API exposes
typestates, not raw payloads or `Message` values.

## SessionCore: sequencing, lanes, schedules, and poison state

`SessionCore` owns the connected `RuntimeChannel`, prevalidated identity,
limits, immutable `ProvisionedProgram`, the active `RunId`, one scratch
buffer, three transmit sequence counters, three receive sequence counters,
connection-global schedule bases, and a poisoned bit.

The lane index is fixed as control 0, metrics 1, user data 2. On send,
`try_send_tracked`:

1. rejects a previously poisoned core;
2. selects the lane from `Message::lane`;
3. encodes with that lane's current transmit sequence and the active run;
4. submits the encoded prefix to the matching transport lane;
5. for user data only, requires a logical schedule, adds `tx_schedule_base`,
   and submits a finite `ScheduleStamp`;
6. returns `None` for transport lane capacity exhaustion without advancing the
   remote sequence;
7. on a successful submission, increments that lane's sequence with checked
   arithmetic and returns the transport completion token;
8. poisons on any other transport failure.

The specialized hello and manifest senders perform the same sequence advance
and capacity behavior, but call their specialized encoders directly.

On receive, `SessionCore::received` first requires a ready transport frame and
maps its `FrameKind` to a runtime lane. Probe frame kinds are rejected before
decode. It then decodes the remote payload, and on success requires:

* the decoded `RunId` equals the active run;
* the decoded message lane equals the outer transport lane;
* the decoded sequence equals the next expected sequence for that lane.

The receive sequence advances only after all three checks pass. A user-data
transport schedule is translated back by subtracting `rx_schedule_base`; an
underflow is a protocol error. The returned `ReceivedMessage` keeps the
transport completion token and translated logical schedule. The state machine
must call `release_received` after inspecting the message. Decode failures,
run mismatches, lane mismatches, sequence replays or gaps, and any later state
error call `SessionCore::poison`; all future operations then return
`RemoteError::Poisoned`.

`RemoteChannel` retains the `RuntimeChannel` and the last run when a complete
session is converted back into a channel. A new `SessionCore` requires a
strictly larger nonzero run. It also snapshots the channel's next outbound and
inbound user-data schedule positions. This prevents a completed session's
payload sequence or schedule from being replayed on a reused connection.

## Handshake call graph and invariants

The two typestates run over the same `SessionCore`, but each accepts only the
messages valid for its state.

### Master

`MasterHandshake::progress` performs transport progress, consumes at most one
ready frame, and then sends the next handshake message if its lane has space:

1. send tag 1 `Hello` with role `Master`;
2. receive tag 1 `Hello`, requiring role `Worker`, required capabilities,
   reversed local/remote identities, and byte-for-byte equal limits;
3. send tag 2 `Manifest`;
4. receive tag 3 `ManifestAck`, requiring both the manifest and program digest
   to equal the local program;
5. send tag 4 `Prepare`;
6. receive tag 5 `PrepareAck`, release the receive token, and become
   `MasterInit`.

A duplicate, wrong tag, or early message is a protocol error. A worker tag 27
`DriverFault` is converted to `RemoteError::Driver` and poisons the core.

### Worker

`WorkerHandshake::progress` follows the peer-facing sequence:

1. send tag 1 `Hello` with role `Worker`;
2. receive and validate the master's tag 1 `Hello`;
3. receive tag 2 `Manifest` and run `WireManifest::matches_proof` against the
   local provisioned program;
4. send tag 3 `ManifestAck` with the local manifest and program digests;
5. receive tag 4 `Prepare`, call `WorkerDriver::prepare` with the active run
   and exact program;
6. send tag 5 `PrepareAck` only after preparation succeeds, then become
   `WorkerInit`.

Driver preparation failures enter the terminal driver-fault path. The worker
first calls `cleanup_after_fault`; only after cleanup succeeds does it send
tag 27, and a cleanup failure is folded into `DriverFault::cleanup_failed`.

## Init image exchange

The init codec messages represent one logical arena image per worker device.
The session intentionally serializes images and chunks rather than allowing
overlap.

On the master, `MasterInit::queue_image` requires an inactive image, a known
device still in `Needed` state, and a byte slice whose length exactly equals
the finalized device image. It computes SHA-256 and stores the image. Progress
then sends:

1. tag 6 `InitBegin(device, bytes, digest)` on control;
2. one or more tag 7 `InitChunk(device, offset, bytes)` messages on user data,
   each with a logical schedule starting at zero for the init stream;
3. tag 8 `InitEnd(device, bytes)` on control;
4. waits for matching tag 9 `InitAck(device)` before marking that device
   complete;
5. after every device is acknowledged, sends tag 10 `InitComplete` and waits
   for tag 11 `InitCompleteAck`, then enters `MasterRun`.

Each chunk remains in the master image until the transport completion token for
that submission is reported. A chunk's next schedule is incremented only after
that transmission completes. The master rejects an acknowledgment naming the
wrong device or arriving outside `WaitAck`.

On the worker, `WorkerInit::process_received` accepts only the next legal init
message. `InitBegin` must name a `Needed` device with the exact finalized byte
count and cannot overlap an active image, pending final admission, or pending
acknowledgment. The driver receives the image metadata before bytes.

`InitChunk` must carry the next worker-side logical schedule, the active device,
and the exact next offset. The checked offset plus decoded length must stay
within the finalized image and within the fixed chunk buffer. The bytes are
copied into stable storage before `WorkerDriver::begin_init_chunk`; the worker
polls that operation, hashes the bytes only on completion, clears the chunk
buffer, advances its offset and schedule, and accepts the next chunk.

`InitEnd` requires the active image, exact device and byte count, and received
offset equal to the complete image. The worker compares the computed SHA-256
with the `InitBegin` digest, calls `finish_init_image`, polls the final
admission, then emits tag 9 `InitAck`. Tag 10 is accepted only when every
device is complete and no chunk, final admission, or acknowledgment remains.
The worker sends tag 11 and enters `WorkerRun` only after that condition.

## Runtime task and data call graph

`MasterRun` and `MasterExit` share `MasterRuntime`, but pass `RunPhase::Loop`
or `RunPhase::Exit` to its operations. Every provisioned task is looked up by
id, checked for the expected phase, kind, and `Idle` status, and then marked
active only after a successful codec submission.

### Driver tasks and metrics

* The master calls `submit_task`, which sends tag 12 `Execute(task)` on
  control. The worker's `handle_execution_message` requires a worker-owned
  driver task in the active phase, a free task slot, and then calls
  `WorkerDriver::submit_task`.
* The worker polls active slots. A normal completion with a metric first sends
  tag 18 `Metric(task, value)` on the metrics lane and waits for that
  submission to flush. It then sends tag 13 `TaskComplete(task)` on control.
  A completion without a metric sends only tag 13.
* A worker poll error is represented as tag 14 `TaskFailed(task, fault)` on
  control. The master accepts it only for an active task in the expected phase
  and then marks the task failed. This is a task result, not the terminal
  cleanup path.
* The master accepts tag 18 only for an active driver task in the expected
  phase and stores it in a fixed metric slot. A full metric inbox returns
  `Backpressured("master metric inbox")` through the session path.

### Master-to-worker user data

For a `MasterToWorker` cross transfer, the master validates direction, exact
finalized byte count, phase, kind, and idle status, acquires every finalized
half-duplex token, then sends tag 16 `UserData(task, bytes)` on the user-data
lane with the transfer's static schedule. `SessionCore` adds the connection
base before submitting the transport frame. If the lane is full, the token is
released and the caller receives `Backpressured("user-data lane")`.

The worker requires no overlapping inbound transfer, verifies direction,
translated schedule, exact byte count, phase, and idle status, and copies the
payload into fixed `incoming_scratch`. It calls
`WorkerDriver::begin_receive_user_data`, marks the task active, and polls. On
the driver's completion byte count matching the payload, it clears the scratch
buffer, marks the transfer awaiting its task acknowledgment, and emits a
`DataAccepted` event. The worker subsequently sends tag 13 `TaskComplete` and
releases its half-duplex token.

### Worker-to-master user data

For a `WorkerToMaster` cross transfer, the master validates that no other
worker-to-master transfer is active, acquires the transfer's half-duplex
tokens, and sends tag 15 `DataRequest(task)` on control. The worker validates
direction, size, phase, and idle status, calls
`WorkerDriver::begin_produce_user_data` into fixed `data_scratch`, and polls.
After an exact byte-count completion it sends tag 16 `UserData(task, bytes)`
on user data with the finalized transfer schedule and waits in `WaitAck`.

The master accepts tag 16 only with the expected direction, schedule, length,
phase, and active status. It copies the borrowed bytes into a fixed data slot,
marks the task `ReceivedAwaitAck`, and allows one pending acknowledgment. The
next progress call sends tag 17 `DataAck(task)` on control. Once that send is
accepted, the master marks the task complete, releases the half-duplex token,
and reports `DataReady` followed by `TaskComplete` through the public event
surface. The worker accepts tag 17 only for its matching outgoing transfer in
`WaitAck`, calls `user_data_acked`, clears its scratch, marks complete, and
releases its token.

The codec only transports bytes and identifiers. All direction, schedule,
phase, task status, byte count, and half-duplex ownership checks are in the
session state machine.

## Exit and cancellation messages

Normal exit is a distinct typestate:

1. after all loop tasks complete, the master enters `MasterExit` and sends tag
   21 `BeginExit`;
2. the worker requires loop completion, calls `WorkerDriver::begin_exit`, and
   sends tag 22 `ExitReady`;
3. the master enters active exit work and may submit phase-Exit tags 12, 15,
   and 16 through the same runtime routines;
4. after exit tasks and any pending data acknowledgment complete, the master
   sends tag 23 `Release(device)` one device at a time;
5. the worker requires exit tasks complete, calls `release_arena`, and later
   sends the matching tag 24 `ReleaseAck` after the driver operation has
   completed. Duplicate or unknown devices are protocol errors;
6. when every release is acknowledged, the master sends tag 25
   `ExitComplete`;
7. the worker accepts it only with complete exit work and all release
   acknowledgments complete, calls `finish`, sends tag 26 `ExitAck` only after
   the send is flushed, and becomes `WorkerComplete`;
8. the master accepts the terminal tag 26 only in `WaitComplete` and becomes
   `MasterComplete`.

Cancellation starts from the master with tag 19 `Cancel(reason)`. The worker
calls `WorkerDriver::cancel`, then `WorkerCancelled` releases each arena and
sends tag 24 acknowledgments, calls `finish`, and sends tag 20 `CancelAck`
only after the transport completion token is flushed. The master accepts
normal task results and exit-ready messages that were already in flight while
waiting, but completes cancellation only after every release acknowledgment
and tag 20. A duplicate release acknowledgment, an unexpected message, or a
driver fault poisons the session.

Terminal worker driver faults use tag 27 `DriverFault` instead of the ordinary
task result path. `begin_driver_fault` permits only one terminal report and
first invokes `WorkerDriver::cleanup_after_fault`; the report is sent after
cleanup and the peer turns it into `RemoteError::Driver`. This path has no
per-arena release acknowledgment protocol. Ordinary requested cancellation
continues to use the exact release and acknowledgment flow above.

## Error surfaces and fail-closed behavior

### Direct codec errors

The codec returns `RemoteError::Codec` for malformed or unrepresentable wire
content and `RemoteError::CapacityExhausted` for a destination buffer that is
too small. The concrete codec details are:

| Condition | Error detail |
| --- | --- |
| Writer cursor arithmetic overflows | `message length overflowed` |
| Writer destination range is absent | `CapacityExhausted("message codec buffer")` |
| Manifest encoded-size arithmetic overflows | `manifest payload length overflowed` |
| Manifest destination is too small | `CapacityExhausted("manifest codec buffer")` |
| Artifact, init-chunk, or user-data count does not fit `u32` | The corresponding `...does not fit the wire ABI` detail |
| Input exceeds `max_message_bytes` | `message exceeds the configured bound` |
| Remote magic, version, or reserved byte is wrong | `message magic differs`, `message protocol version differs`, or `message reserved byte is nonzero` |
| Short input or checked range failure | `message is truncated` or `message range overflowed` |
| Unknown role, metric scalar, or tag | `unknown peer role`, `unknown metric scalar type`, or `unknown message tag` |
| Manifest artifact count is over the advertised bound | `artifact count exceeds configured bound` |
| Artifact byte multiplication overflows | `artifact byte count overflowed` |
| A decoded `u32` length cannot fit this host | The corresponding init-chunk or user-data host-fit detail |
| Any bytes remain after the tag body | `message has trailing bytes` |

`RemoteError::Display` prefixes these as `remote codec rejected a message:`.
There is no error recovery, alternate decoder, version fallback, or partial
message acceptance.

### Session and transport errors around the codec

The codec's successful return is not sufficient to accept a message. The
session can still return or poison on:

* `RunMismatch` for a decoded run different from the active run;
* `Protocol` for a wrong lane, replayed or out-of-order per-lane sequence,
  wrong role, missing capability, endpoint/profile mismatch, fixed-limit
  mismatch, schedule mismatch, duplicate or out-of-state lifecycle message,
  wrong task/device, wrong direction, wrong byte count, or invalid
  half-duplex ownership;
* `ManifestMismatch` for an invalid local manifest, a failed wire manifest
  proof, or an init image digest mismatch;
* `Backpressured` when a fixed runtime lane, data inbox, metric inbox, or
  half-duplex token is unavailable;
* `CapacityExhausted` when a finalized manifest, init chunk, user-data payload,
  task slot, or fixed buffer exceeds its configured bound;
* `Driver` for a worker driver fault;
* `Transport` for any outer framing, identity, integrity, connection, or I/O
  failure; and
* `Poisoned` after any unrecoverable error has marked the session closed.

Transport completion tokens are part of these invariants. A sent message is
not considered flushed until `RuntimeChannel::progress` reports its token;
`SessionCore::progress_transport` then releases the transmit slot. A received
payload remains unavailable until the state machine releases the matching
receive token. This is why init chunks, metrics, terminal acknowledgments, and
driver-fault reports carry explicit waiting states around otherwise simple
codec calls.

## End-to-end communication role

The complete path for a normal message is:

```text
public typestate method or worker poll
    -> SessionCore::try_send_tracked / try_send_hello / try_send_manifest
    -> codec::encode* into the fixed scratch buffer
    -> RuntimeChannel::submit_* (lane, schedule, completion token)
    -> nonblocking transport frame with endpoint identity and SHA-256 payload digest
    -> peer RuntimeChannel::progress and received frame
    -> SessionCore::received
    -> codec::decode borrowed payload
    -> run, lane, and per-lane sequence checks
    -> current handshake/init/run/exit typestate
    -> WorkerDriver or master storage
    -> authoritative state transition and public event
```

The reverse direction follows the same path with master and worker roles
swapped. The codec therefore provides a canonical, bounded representation for
the control protocol and the two byte-carrying lanes, while the transport
provides delivery framing and identity/integrity checks, and the session owns
all lifecycle semantics. No domain calculation, device discovery, compiler,
file operation, or socket establishment occurs in this codec.
