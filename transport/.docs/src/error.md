# Transport error contract

`transport/src/error.rs` defines the complete failure vocabulary for the
`recipe-transport` crate.  The crate is an already-connected, identity-bound,
bounded TCP transport.  It does not open a listener, choose a peer, discover
an interface, establish security, or recover a failed connection.  A
`TransportError` therefore describes either a rejected transport input, a
wire or stream failure, an invalid runtime state, or a peer benchmark that
could not produce measured evidence.

The public surface is re-exported by `transport/src/lib.rs`:

```rust
pub use error::{TransportError, TransportResult};
```

`TransportResult<T>` is exactly `Result<T, TransportError>`.  It adds no
conversion, retry, or recovery behavior.  Every public protocol, runtime, and
probe operation that can fail uses this alias or a result whose transport
error is converted at that boundary.

## Shape, rendering, and conversion

`TransportError` is `Clone`, `Debug`, `PartialEq`, and `Eq`.  It is marked
`#[non_exhaustive]`, so code outside this crate must retain a wildcard when
matching it.  It implements `Display` and `std::error::Error` and does not
override `source()`.  The variant value and its rendered text are the complete
cause information; an underlying `io::Error` is reduced to its
`io::ErrorKind`.

The current variants and exact `Display` forms are:

| Variant | Payload | Display form | Contract meaning |
| --- | --- | --- | --- |
| `InvalidConfiguration` | `&'static str` | `invalid transport configuration: MESSAGE` | A caller-supplied transport or probe value violates a fixed bound or constructor invariant. |
| `InvalidIdentity` | `&'static str` | `NAME identity must not be zero` | A machine or profile digest is the zero digest. |
| `UnsupportedVersion` | `u16` | `unsupported transport version VERSION` | A wire header or probe payload carries a schema/version other than the one implemented here. |
| `InvalidFrame` | `&'static str` | `invalid transport frame: MESSAGE` | A frame header, metadata combination, or probe payload is structurally invalid. |
| `FrameTooLarge` | `declared`, `limit: usize` | `frame payload DECLARED exceeds limit LIMIT` | The declared or submitted payload cannot fit the configured wire or runtime capacity. |
| `BufferTooSmall` | `required`, `available: usize` | `receive buffer has AVAILABLE bytes but frame requires REQUIRED` | A blocking receiver decoded a frame larger than the caller's destination buffer. |
| `UnexpectedSequence` | `expected`, `received: u64` | `expected frame sequence EXPECTED, received RECEIVED` | The connection-global wire sequence is not the next expected value. |
| `UnexpectedIdentity` | none | `frame endpoint identity does not match this session` | A decoded frame is not addressed from the prevalidated remote endpoint to the local endpoint. |
| `IntegrityFailure` | none | `frame payload digest does not match` | SHA-256 of a received payload differs from the digest authenticated in its header. |
| `Cancelled` | none | `transport operation was cancelled` | The controlled peer benchmark cancellation flag was observed. |
| `DeadlineExceeded` | none | `transport deadline exceeded` | A bounded operation deadline elapsed, or an OS timeout/nonblocking result was normalized to the same condition. |
| `ConnectionClosed` | none | `peer closed the connection` | A read or write returned zero before the requested bytes were complete. |
| `Io` | `io::ErrorKind` | `transport I/O KIND` | An operating-system stream operation failed with a kind other than the normalized timeout/nonblocking case. |
| `ProtocolState` | `&'static str` | `invalid protocol state: MESSAGE` | An internal counter, schedule, slot, or API lifecycle invariant was violated. |
| `CapacityExhausted` | `&'static str` | `NAME capacity is exhausted` | No fixed transmit slot is free in the named runtime lane. |
| `UnknownCompletion` | `u64` | `unknown completion token TOKEN` | A completion query or release names no retained transmit slot. |
| `Probe` | `String` | `peer benchmark failed: MESSAGE` | Probe statistics, unit conversion, thread execution, or peer-reported measured-rate data cannot be represented or validated. |
| `Poisoned` | none | `transport channel is poisoned` | A stateful sender, receiver, or runtime channel has already crossed its terminal failure boundary. |

The private `TransportError::io(&io::Error)` helper is the only normalizer for
OS errors (`error.rs:27-35`).  `TimedOut` and `WouldBlock` become
`DeadlineExceeded`; every other kind becomes `Io(kind)`.  `From<io::Error>`
delegates to that helper (`error.rs:83-85`), so `?` on stream setup and socket
option calls follows the same mapping.  The protocol and runtime read/write
loops explicitly consume `Interrupted` and, for nonblocking runtime I/O,
`WouldBlock` without returning an error.  An `Interrupted` or `WouldBlock`
that reaches `TransportError::io` through another operation is therefore
rendered as `Io(Interrupted)` or normalized to `DeadlineExceeded`, exactly as
the OS kind dictates.

No variant owns a source error, and no conversion retries an operation,
substitutes a value, or clears a poisoned state.

## Complete construction ledger

The following sections enumerate every current construction site in the
transport sources.  The line references are source anchors, not inferred
possibilities.

### `InvalidConfiguration`

This variant rejects values before the corresponding object or operation can
be admitted.  Its current messages and callers are:

* `SessionIdentity::new` rejects equal local and remote machine digests with
  `transport endpoints must identify distinct machines`
  (`protocol.rs:49-55`).
* `CompletionToken::new` rejects zero with `completion token must be nonzero`
  (`protocol.rs:77-83`).
* `ProtocolLimits::new` rejects a zero payload, a payload above 64 MiB, or a
  payload above the header's `u32` length field with `maximum payload must be
  between 1 byte and 64 MiB`; it rejects a zero operation timeout with
  `operation timeout must be nonzero` (`protocol.rs:159-173`).
* `MeasuredLocalMemory::new` rejects zero capacity with `measured local memory
  capacity must be nonzero` (`probe.rs:36-47`).
* `TcpPeerSession::new` rejects a descriptor that does not declare
  asynchronous submission with `peer descriptor must declare asynchronous
  submission` (`probe.rs:66-90`).
* The probe directional setup converts a `u64` buffer size that cannot fit
  `usize` to `probe buffer does not fit address space` (`probe.rs:190-193`).
* `validate_plan` rejects an unbounded or zero plan with `peer benchmark plan
  must be bounded and nonzero`, iterations above `MAXIMUM_ITERATIONS` with
  `peer benchmark iteration limit exceeded`, and a multiplication overflow or
  total above `MAXIMUM_TOTAL_BYTES` with `peer benchmark total byte limit
  exceeded` (`probe.rs:395-418`).
* `encode_begin` rejects a duration whose nanoseconds do not fit `u64` with
  `benchmark duration is too large` (`probe.rs:420-439`).
* `ScheduleStamp::new` rejects the wire unscheduled sentinel
  (`u64::MAX`) with `schedule position collides with the unscheduled
  sentinel` (`runtime.rs:34-46`).
* `ChannelCapacities::new` rejects any zero control, metrics, or user-data
  slot count with `every runtime lane requires at least one fixed slot`, and
  rejects a zero runtime payload capacity with `runtime maximum payload must
  be nonzero` (`runtime.rs:56-79`).
* `RuntimeChannel::new` rejects a runtime payload capacity larger than the
  `ProtocolLimits` payload with `runtime payload capacity exceeds protocol
  limit` (`runtime.rs:274-317`).

These checks return directly.  They do not create a sender, receiver, probe
measurement, or runtime channel, and they do not set a poison bit.  A socket
option failure in `RuntimeChannel::new` or `split_stream` is instead an
`io::Error` converted through `From<io::Error>`.

### `InvalidIdentity`

`EndpointIdentity::new` checks both digest fields.  A zero machine digest
returns `InvalidIdentity("machine")`; a zero profile digest returns
`InvalidIdentity("profile")` (`protocol.rs:24-32`).  The identity never exists
on success with either zero field.  A nonzero but unexpected identity in a
received header is a separate `UnexpectedIdentity` failure.

### `UnsupportedVersion`

`decode_header` reads the two-byte wire version and returns
`UnsupportedVersion(version)` when it differs from `PROTOCOL_VERSION` 1
(`protocol.rs:371-378`).  Probe begin payload validation performs the same
check against `PROBE_PAYLOAD_SCHEMA` and returns `UnsupportedVersion(schema)`
(`probe.rs:442-455`).  In both cases the value is the peer's received number,
not a locally substituted version.

### `InvalidFrame`

`FrameKind::decode` rejects all values other than the seven assigned frame
kind bytes with `unknown frame kind` (`protocol.rs:103-114`).
`FrameMetadata::new` rejects a user-data frame without a finite schedule with
`user-data frame requires a finite schedule position`, and rejects a schedule
on every other kind with `only user-data frames may carry a schedule position`
(`protocol.rs:129-148`).

`decode_header` rejects a bad eight-byte magic with `bad magic` and nonzero
reserved flags with `reserved flags are nonzero` (`protocol.rs:371-382`).
`RuntimeChannel::progress_read` rejects a decoded probe kind on a runtime
channel with `probe frame is not valid on a runtime channel`
(`runtime.rs:592-603`).

Probe payload validation adds these structural failures:

* `decode_and_validate_begin` requires exactly 40 bytes, a zero reserved byte,
  duplex value 1 or 2, matching duplex, and a plan equal to the local plan.
  The messages are respectively `wrong probe-begin payload length`, `probe-begin
  reserved byte is nonzero`, `invalid probe duplex value`, `peer duplex
  declaration disagrees`, and `peer benchmark plan disagrees`
  (`probe.rs:442-479`).
* `receive_exact` rejects a valid wire frame whose kind or completion token is
  not the expected phase value with `probe frame kind or completion token is
  out of phase` (`probe.rs:747-760`).
* `validate_complete` requires exactly 16 bytes and reports `wrong
  probe-complete payload length` (`probe.rs:794-808`).
* `read_u16`, `read_u32`, and `read_u64` all report `truncated integer` when
  the requested range is absent or cannot be converted to the fixed array
  (`probe.rs:810-835`).

The kind and header errors are produced by the wire decoder.  A blocking
`WireReceiver` poisons itself for them.  A probe phase mismatch from
`receive_exact` occurs after `receive_into` has already returned a valid frame,
so that helper itself does not poison the receiver; the failed benchmark
attempt is returned to its caller instead.

### `FrameTooLarge`

There are four distinct checks:

1. `WireSender::send` compares the submitted payload with
   `ProtocolLimits::max_payload` (`protocol.rs:210-219`).
2. `encode_header` protects its four-byte payload-length field with the
   `u32::MAX` limit (`protocol.rs:340-348`).
3. `decode_header` rejects a peer-declared payload above the configured
   protocol limit (`protocol.rs:382-389`).
4. `RuntimeChannel::submit` compares an application payload with the
   preallocated receive buffer, which is the runtime capacity limit
   (`runtime.rs:361-373`).

The probe validation path constructs one more value before any directional
transfer: when `plan.buffer_bytes` is larger than the connection's protocol
limit, it returns the declared `usize` (or `usize::MAX` if conversion itself
fails) and that limit (`probe.rs:106-115`).  The probe maps it to a validation
phase transport failure.  A size rejection does not itself poison a sender,
receiver, or runtime channel because no bytes were attempted.

### `BufferTooSmall`

`WireReceiver::receive_inner` decodes the header first, then compares the
declared payload length with the caller's supplied slice
(`protocol.rs:274-283`).  It returns `BufferTooSmall { required, available }`
before reading the payload.  `WireReceiver::receive_into` treats every error
from `receive_inner` as terminal, poisons the receiver, and shuts down both
directions of its stream.  The header has already been consumed, and the
payload is not drained or retried.

### `UnexpectedSequence`

`decode_header` compares the received connection-global sequence with the
receiver's `expected_sequence` and returns both values on mismatch
(`protocol.rs:389-396`).  The blocking receiver therefore poisons and shuts
down through `receive_into`.  Runtime receive uses the same decoder and
poisons the `RuntimeChannel` through `progress` when the error escapes
`progress_read`.

### `UnexpectedIdentity`

`decode_header` compares sender machine/profile bytes with the session's
remote endpoint and receiver machine/profile bytes with its local endpoint.
Any difference returns `UnexpectedIdentity` (`protocol.rs:398-413`).  This is
checked after version, flags, kind, length, sequence, token, and metadata
validation.  As with every `WireReceiver::receive_into` decode failure, a
blocking receiver poisons and shuts down; runtime progress marks the channel
poisoned.

### `IntegrityFailure`

`WireReceiver::receive_inner` computes SHA-256 over the fully read payload and
compares it with the header digest (`protocol.rs:284-294`).  A mismatch returns
`IntegrityFailure`, and `receive_into` poisons the receiver.  Runtime
`finish_receive` performs the same digest check over its preallocated payload
(`runtime.rs:667-685`); `RuntimeChannel::progress` marks the channel poisoned
when that error is returned.

The probe also checks that every received `ProbeData` byte equals the round
pattern.  A mismatch returns `IntegrityFailure` from `measure_receive`
(`probe.rs:684-711`), after the wire receiver has already accepted the frame.
That failure is classified as an integrity benchmark failure, not as a new
wire digest error.

### `Cancelled` and `DeadlineExceeded`

`transfer_control_check` checks cancellation first, then the earlier of the
phase deadline and the caller's absolute deadline.  It returns
`Cancelled` when the shared cancellation flag is set, otherwise
`DeadlineExceeded` when that minimum deadline has elapsed
(`probe.rs:714-735`).  The check runs before every directional operation and
again after all samples.  `operation_deadline` and
`transfer_operation_deadline` use the same minimum when calculating socket
deadlines (`probe.rs:358-368`, `probe.rs:714-725`).

The protocol `remaining` helper returns `DeadlineExceeded` when an absolute
deadline is expired or has no nonzero duration left (`protocol.rs:458-463`).
`write_all_deadline` and `read_exact_deadline` set the corresponding socket
timeout on every partial-I/O iteration.  A kernel `TimedOut` or `WouldBlock`
from those operations is normalized to `DeadlineExceeded` by
`TransportError::io`.

In a controlled peer benchmark, either value is converted by
`benchmark_failure` into structured `PeerBenchmarkFailureKind::Cancelled` or
`PeerBenchmarkFailureKind::Deadline`, retaining the active
`PeerBenchmarkPhase`.  `RuntimeChannel` has no cancellation API; its deadline
errors arise only from direct protocol helpers or OS conversion and poison the
channel when returned by `progress`.

### `ConnectionClosed` and `Io`

The blocking protocol loops return `ConnectionClosed` when `read` or `write`
returns zero before the requested slice is complete
(`protocol.rs:425-456`).  The nonblocking runtime header and payload paths
return the same variant for a zero read or write
(`runtime.rs:507-555`, `runtime.rs:577-665`).  Protocol sender writes poison
the sender on the failed `send` result; receiver reads poison the receiver;
runtime `progress` poisons the channel.

All other stream setup and I/O failures enter through `From<io::Error>` or
`TransportError::io`, becoming `Io(kind)` except for the timeout/nonblocking
normalization described above.  There is no direct `TransportError::Io(...)`
construction in the transport sources.  Protocol write/read loops consume
`Interrupted` and retry.  Runtime write/read paths consume `WouldBlock` and
`Interrupted` as a no-progress result, so those cases do not poison or return
an error from `progress`.

### `ProtocolState`

This variant records a violated state or checked-arithmetic invariant.  The
current messages are:

| Site | Message | Invariant and immediate consequence |
| --- | --- | --- |
| `WireSender::send` (`protocol.rs:231-237`) | `transmit sequence exhausted` | A successfully written frame cannot advance the sender sequence beyond `u64::MAX`; the bytes were written, the sender is not explicitly poisoned, and the error is returned. |
| `WireReceiver::receive_inner` (`protocol.rs:287-294`) | `receive sequence exhausted` | A verified frame cannot advance the receiver sequence; `receive_into` poisons after this error. |
| `TcpPeerSession::run_benchmark` (`probe.rs:116-120`) | `probe round exhausted` | The per-session benchmark round cannot be incremented; the validation attempt fails. |
| `DirectionAccumulator::record` (`probe.rs:509-527`) | `probe sample count exhausted` | Measured samples cannot be counted; the directional phase fails. |
| `RuntimeChannel::submit_user_data` (`runtime.rs:347-359`) | `user-data schedule positions must be strictly increasing` | A caller reused or reordered an outbound schedule stamp; no slot is submitted and the previous stamp remains. |
| `RuntimeChannel::submit` (`runtime.rs:382-393`) | `completion token space exhausted`; `submission order space exhausted` | A queued slot may already exist when checked arithmetic fails, because the counters advance after lane admission; the channel is not automatically poisoned. |
| `RuntimeChannel::release_completion` (`runtime.rs:427-435`) | `transmit completion cannot be released before it is complete` | A queued or writing slot remains retained. |
| `RuntimeChannel::release_received` (`runtime.rs:450-459`) | `no received frame is pending`; `received completion token does not match the pending frame` | Receive storage is not reset. |
| `RuntimeChannel::progress_write` (`runtime.rs:463-475`) | `queued slot has no metadata` | A queued transmit slot cannot be encoded. `progress` poisons the channel. |
| `RuntimeChannel::finish_active` (`runtime.rs:559-574`) | `active slot has no metadata`; `transmit sequence exhausted` | A completed slot or sequence cannot advance; `progress` poisons the channel. |
| `RuntimeChannel::progress_read` (`runtime.rs:635-638`) | `complete header was not decoded` | Payload bytes cannot be associated with metadata; `progress` poisons the channel. |
| `RuntimeChannel::progress_read` (`runtime.rs:604-613`) | `received user-data schedule is not strictly increasing` | A decoded user-data frame violates the connection-global schedule order; `progress` poisons the channel. |
| `RuntimeChannel::finish_receive` (`runtime.rs:667-685`) | `receive has no decoded header`; `receive sequence exhausted` | The pending receive cannot be completed or its sequence cannot advance; `progress` poisons the channel. |
| `next_schedule_position` (`runtime.rs:767-779`) | `user-data schedule positions exhausted` | A reused connection cannot produce the next finite schedule position. |

`ProtocolState` is not a generic catch-all.  Each message corresponds to one
checked counter, slot transition, schedule rule, or phase invariant.  The
runtime's `progress` method marks the channel poisoned for any state error
that escapes its write or read pass.  Constructor and submission arithmetic
errors that occur before or after slot admission return directly unless the
caller chooses to poison its own higher-level session.

### `CapacityExhausted`

`FixedLane::submit` scans the fixed slot array for a `Free` slot.  If none is
available it returns `CapacityExhausted(self.name)`, whose names are `control
lane`, `metrics lane`, or `user-data lane` (`runtime.rs:160-182`).  A slot is
not free while queued, writing, or complete.  Completed slots remain retained
until `release_completion` succeeds, so capacity is caller-owned lifecycle
state rather than an allocation request.

The generic runtime submission path returns this value without changing the
token, order, or schedule counters.  It does not poison `RuntimeChannel`.
The remote session deliberately treats it as ordinary backpressure:
`SessionCore::try_send_tracked` returns `Ok(None)`, and the hello and manifest
send helpers return `Ok(false)` (`remote/src/session.rs:535-596`).  Direct
transport callers see the original `CapacityExhausted` value.

### `UnknownCompletion`

`RuntimeChannel::find_slot` scans all three lanes by token and returns
`UnknownCompletion(token.get())` if no metadata matches (`runtime.rs:716-732`).
`completion_state` also reports this if a found slot is `Free`, while
`release_completion` reports it whenever `find_slot` cannot locate the token
(`runtime.rs:416-435`).  This is a caller lifecycle error, not a wire failure.
It does not poison the runtime channel by itself.  The probe classification
table treats it as a protocol failure if one reaches a controlled benchmark,
although the current probe code does not use runtime completions.

### `Probe`

`Probe` owns an explanatory `String` for failures that are neither wire
syntax nor transport I/O:

* `decode_and_validate_begin` converts an invalid peer `BytesPerSecond` unit
  value to the unit-constructor text (`probe.rs:481-485`).
* `DirectionAccumulator::record` reports duration-sum or variance overflow;
  `finish` reports no samples or a mean that cannot fit `u64`; and
  `canonical_nanoseconds` reports a duration that cannot fit `u64`
  (`probe.rs:509-556`).
* Full-duplex measurement converts a panicking receive thread join into
  `receive benchmark thread panicked` (`probe.rs:558-600`).  If both worker
  directions fail, the outbound error is selected because the match checks
  that result first.
* `measured_rate` reports arithmetic overflow, a rate that cannot fit the
  profile schema, or a `BytesPerSecond` constructor failure
  (`probe.rs:765-778`).
* `validate_complete` converts invalid peer outbound or inbound completion
  rates to `peer reported invalid outbound completion: ...` and `peer reported
  invalid inbound completion: ...` (`probe.rs:794-807`).

These are returned from the directional or completion phase and are not
replaced by nominal rates.  The resulting benchmark attempt is failed, so no
`PeerMeasurement` is returned.

### `Poisoned`

`WireSender::send` and `WireReceiver::receive_into` check their own poison bit
before doing any I/O (`protocol.rs:209-218`, `protocol.rs:262-271`).  A sender
sets the bit and calls `shutdown(Shutdown::Both)` after a header or payload
write failure; it does not poison for a pre-write payload-size or
header-construction failure.  A receiver sets the bit and shuts down after any `receive_inner`
failure, including malformed headers, bounds, digest, sequence, identity, and
I/O errors.

`RuntimeChannel::ensure_healthy` returns `Poisoned` once its bool is set
(`runtime.rs:758-764`).  `progress` sets that bool whenever either its write
or read pass returns an error (`runtime.rs:397-413`).  The runtime does not
call `shutdown` at that point, and there is no reset operation.  `submit_*`
and `progress` enforce the bit; completion lookup/release and schedule-query
methods do not call `ensure_healthy`, so they can still inspect or release
retained local state after poisoning if their own lifecycle preconditions are
met.

The probe constructs `TransportError::Poisoned` only when its session mutex is
poisoned (`probe.rs:332-347`).  It maps that value to a transport benchmark
failure.  It does not add a second poison state to `TcpPeerSession`.

## Protocol propagation and state consequences

The protocol layer (`transport/src/protocol.rs`) has two distinct stateful
objects over one already-connected stream:

* `WireSender::send` validates local poison and payload size, encodes a header,
  writes the header and payload with the same absolute deadline, and advances
  `next_sequence` only after both writes succeed.  Any write-side result error
  poisons and shuts down the sender.  Header construction errors occur before
  the write result and leave the sender usable.  A sequence overflow is
  returned after the bytes have been written and leaves the sender's sequence
  at its maximum value.
* `WireReceiver::receive_into` validates local poison, reads exactly the fixed
  header, decodes all identity, version, flags, kind, length, sequence, token,
  schedule, and digest metadata, checks the caller buffer, reads exactly the
  payload, verifies SHA-256, and advances `next_sequence`.  Any error poisons
  and shuts down.  A successful frame is borrowed from the caller's buffer.

`split_stream` makes the supplied stream blocking, enables `TCP_NODELAY`,
clones the write side, and initializes both sequences to zero
(`protocol.rs:312-333`).  Socket setup and clone errors use the `io` conversion.
The two halves therefore share identity and limits but keep independent poison
bits and sequence counters.

`write_all_deadline` and `read_exact_deadline` repeatedly set the socket
timeout to the remaining duration.  Zero-byte I/O is `ConnectionClosed`,
`Interrupted` is retried, and every other failure is normalized by
`TransportError::io`.  No helper retries after an actual I/O error or drains a
partially consumed frame.

## Runtime propagation and state consequences

`RuntimeChannel` is a preallocated nonblocking transport with fixed control,
metrics, and user-data lanes.  Construction validates capacities, changes the
stream to nonblocking mode, and allocates every slot and the receive buffer.
`submit_control`, `submit_metrics`, and `submit_user_data` all route through
the one generic `submit` operation.  It checks health and payload capacity,
creates a nonzero completion token and metadata, admits the payload to a free
lane slot, and only then advances token and submission-order counters.  A full
lane returns `CapacityExhausted` without mutating the counters.  User-data
submission additionally requires a strictly increasing `ScheduleStamp`, and
updates the last submitted stamp only after successful admission.

`progress` executes one transmit pass and one receive pass on every call.  A
successful transmit reports the token when a slot becomes `Complete`; a
successful receive reports the token when the complete frame is digest-checked
and held in `received()`.  `WouldBlock` and `Interrupted` mean no progress,
not failure.  If either pass returns any `TransportError`, `progress` sets the
channel poison bit and returns the transport error.  Because both pass
results are computed before the match, a receive pass is still attempted when
the transmit pass has already returned an error; the transmit error is the
returned error when both fail.

Transmit completion storage is retained until the caller calls
`release_completion` with a token whose slot is `Complete`.  Receive storage
is retained until `release_received` names the pending frame token.  Unknown
tokens and premature releases return `UnknownCompletion` or `ProtocolState`
without changing wire state.  `received()` itself is an infallible view that
returns `None` until a verified frame is ready.

The runtime decoder accepts only runtime kinds (`Control`, `Metrics`, and
`UserData`).  It preserves connection-global sequence ordering and requires
strictly increasing user-data schedule stamps across reused runs.  A malformed
or out-of-order frame therefore poisons the channel at the `progress` boundary;
it cannot be converted into a different lane or a new schedule.

## Probe propagation and phase evidence

`TcpPeerSession::new` first validates the peer descriptor and then delegates
stream setup to `split_stream`.  A successful session owns a mutex containing
one `WireSender`, one `WireReceiver`, the fixed identity and limits, and the
next benchmark round.  Mutex acquisition is nonblocking: a contended lock
causes `thread::yield_now`, while a poisoned lock constructs `Poisoned`.

`run_benchmark` is the only implementation of the controlled peer benchmark.
It validates the bounded plan, computes the attempt deadline as the earlier of
the plan duration and caller absolute deadline, locks the connection, and
executes these phases in order:

1. **Validation.** Control cancellation/deadline is checked, plan bounds are
   checked, the protocol payload limit is checked, and the benchmark round is
   incremented.
2. **BeginExchange.** A 40-byte begin payload carries the schema, duplex,
   plan, and local memory evidence.  The sender and receiver exchange matching
   `ProbeBegin` frames and validate the remote plan and memory values.
3. **ReadyExchange.** Matching zero-length `ProbeReady` frames establish that
   both peers completed begin validation.
4. **DirectionalTransfer.** Full duplex measures send and receive concurrently
   behind a two-party barrier.  Half duplex orders the directions by the two
   machine identities.  Every sample has a token, bounded operation deadline,
   exact frame validation, payload-pattern validation, and duration statistics.
5. **CompletionExchange.** Each peer sends a 16-byte completion payload with
   measured outbound and inbound rates, receives the matching peer frame, and
   validates both reported rates.

Every `TransportError` that reaches `run_benchmark` is converted at the phase
where it occurred by `benchmark_failure` (`probe.rs:370-393`).  The mapping is
exhaustive:

| Transport error(s) | `PeerBenchmarkFailureKind` | Phase evidence |
| --- | --- | --- |
| `Cancelled` | `Cancelled` | The active validation, begin, ready, directional, or completion phase. |
| `DeadlineExceeded` | `Deadline` | The same active phase. |
| `InvalidIdentity`, `UnexpectedIdentity` | `Identity` | Header or endpoint identity disagreement. |
| `IntegrityFailure` | `Integrity` | Wire digest or directional payload-pattern mismatch. |
| `UnsupportedVersion`, `InvalidFrame`, `UnexpectedSequence`, `ProtocolState`, `UnknownCompletion` | `Protocol` | Wire or phase/state contract disagreement. |
| `InvalidConfiguration`, `FrameTooLarge`, `BufferTooSmall`, `ConnectionClosed`, `Io`, `CapacityExhausted`, `Probe`, `Poisoned` | `Transport` | Resource, stream, statistics, or session failure not assigned to the narrower categories. |

The failure retains the `TransportError::Display` text as its `detail`.  The
controlled API returns `PeerBenchmarkAttempt::Failed` rather than a raw
transport result; the successful branch is the only path that constructs a
`PeerMeasurement`.  A failed attempt does not add a nominal rate or partial
evidence to the profile.  `TcpPeerSession` itself has no poison bool: wire
sender/receiver failures retain their own terminal state, while validation
errors that occur after a successfully received frame return the structured
failure without an additional session-level poison operation.

The compatibility `PeerSession::benchmark` method creates a fresh
`PeerBenchmarkControl`, runs the same implementation, and calls
`PeerBenchmarkAttempt::into_measurement`.  Control deadline construction can
return `recipe_probe::ProbeError` before transport execution; a failed
structured attempt is converted there to `ProbeError::Benchmark` with its
phase, kind, and detail.

## Downstream end-to-end propagation

The transport crate has two workspace consumers that preserve the above
boundary rather than inventing a second transport failure vocabulary.

### Probe profile construction

`probe::ProbeEngine::probe` creates a controlled plan for every peer, calls
`PeerSession::benchmark_controlled`, and immediately calls
`PeerBenchmarkAttempt::into_measurement` (`probe/src/engine.rs:92-100`).
`Measured` results are validated and become measured peer profile properties.
`Failed` results become `ProbeError::Benchmark`; the `?` exits profile
construction before topology/discovery validation, cache construction, or
profile publication.  Thus a transport, identity, integrity, protocol,
deadline, cancellation, or statistics failure cannot masquerade as measured
throughput.

### Remote runtime sessions

`remote/src/error.rs` has a dedicated
`RemoteError::Transport(recipe_transport::TransportError)` variant and an
unchanged `From<recipe_transport::TransportError>` conversion
(`remote/src/error.rs:64-66`).  `SessionCore` uses that conversion whenever a
`RuntimeChannel` operation is returned through `SessionCore::new`'s schedule
queries, `progress_transport`, a tracked send, or `release_received`
(`remote/src/session.rs:423-462`, `remote/src/session.rs:484-547`,
`remote/src/session.rs:550-660`).  The explicit `ScheduleStamp::new` mapping
for a user-data send uses the same conversion.

The remote layer makes one deliberate exception for fixed lane capacity:
`CapacityExhausted` from control, metrics, or user-data submission becomes
`Ok(None)` or `Ok(false)`, which lets the phase machine remain pending and
retry after transport progress.  Every other transport error passed through
those helpers calls `SessionCore::poison`, stores the remote poisoned bit, and
returns `RemoteError::Transport` unchanged inside its wrapper.  Later
transport-driving operations first return `RemoteError::Poisoned`; no retry or
replacement channel is created.  Remote handshake, init, loop, and exit
typestate `progress` methods all use these core helpers, so the first concrete
transport error is the end-to-end failure observed by their caller.

## Failure boundary summary

The complete normal flow for a malformed or failed peer frame is therefore:

```text
TCP read/write or decoded frame
        |
        v
TransportError (wire/runtime/probe construction site)
        |
        +-- WireReceiver/WireSender: poison and shutdown on I/O/decode receive failure
        |
        +-- RuntimeChannel::progress: set poison bit, return first pass error
        |
        +-- TcpPeerSession: phase-tagged PeerBenchmarkFailure
        |       |
        |       +-- ProbeEngine: ProbeError::Benchmark, no measured profile
        |
        +-- Remote SessionCore: RemoteError::Transport and poison, except lane capacity
```

The caller remains responsible for releasing successful completion tokens and
for stopping a failed or poisoned session.  Transport errors are evidence of
the actual failed transition, not statuses that prove a successful operation,
and this crate does not add a fallback state after one is returned.
