# Paired transport probe

`transport/src/probe.rs` is Recipe's concrete implementation of the
`recipe_probe::PeerSession` boundary. It measures one already-established TCP
connection in both directions and returns a `PeerMeasurement` that can be
admitted to a measured discovery profile. It is a discovery-time protocol, not
the runtime data channel.

The implementation is deliberately symmetric. There is no listener, connect,
accept, name lookup, peer selection, authentication service, or responder
object in this crate. A caller establishes the connection, gives both ends
the exact endpoint identities, corresponding descriptors, and the same plan,
constructs a `TcpPeerSession` at each end, and invokes the benchmark at each
endpoint sends every control phase before receiving the peer's matching phase.
Consequently a one-sided invocation waits for the other side until its
deadline, rather than selecting a fallback mode.

## Ownership and role in discovery

The paired probe crosses these boundaries:

| Boundary | Responsibility |
| --- | --- |
| Session owner | Establishes the `TcpStream`, obtains the exact local and remote profile identities, chooses `ProtocolLimits`, builds the `PeerDescriptor`, and measures the local memory domain supplied in the begin payload. |
| `recipe-transport` | Owns the blocking framed TCP exchange, endpoint binding, wire sequence, payload digest, phase ordering, duplex execution, timing samples, and structured failure conversion. |
| `recipe-probe` | Discovers host/GPU inventory, derives the bounded network plan from the seed, orders sessions, requests controlled attempts, validates measured properties and evidence, and builds or caches the measured profile. |
| `recipe-cluster` | Accepts a `MeasuredNetworkPair` from an established session, binds it to member identities, and turns its rates and remote-memory values into the cluster's directed links and worker RAM device. |
| `recipe-remote` | Owns a separate preallocated runtime channel. Its receive state rejects probe frame kinds, so a discovery frame cannot enter the execution protocol. |

The root `recipe probe` command currently passes an empty peer-session slice.
The transport implementation is therefore an integration boundary for a
distributed caller, not a hidden network-discovery path in the local CLI.

The relevant call graph is:

```text
connected TcpStream + SessionIdentity + ProtocolLimits
    + PeerDescriptor + MeasuredLocalMemory
    -> TcpPeerSession::new
    -> ProbeEngine::inspect (descriptor and cache identity)
    -> ProbeEngine::probe
       -> BenchmarkPlans::from_seed (network plan)
       -> PeerBenchmarkControl::for_plan
       -> PeerSession::benchmark_controlled
          -> TcpPeerSession::run_benchmark
             -> begin, ready, data, complete exchanges
             -> PeerMeasurement + PeerBenchmarkEvidence
       -> ProbeEngine::validate_peer_measurement
       -> MeasuredProfile::peer_benchmarks and topology links
    -> optional MeasuredNetworkPair::from_probe / cluster assembly
```

`ProbeEngine::current_cache_identity` and the inspection portion of
`load_or_probe_and_store` enumerate and validate descriptors but do not perform
the transfer benchmark. A cache hit therefore avoids the paired data exchange.
On a cache miss, every supplied session is benchmarked and any failed attempt
prevents profile construction and storage.

## Public session surface

### `MeasuredLocalMemory`

`MeasuredLocalMemory` is the local endpoint's memory fact that is sent to the
peer in `ProbeBegin` and returned as the peer's remote-memory measurement. Its
fields are a `ByteCount` capacity and a `BytesPerSecond` transfer rate. The
constructor rejects a zero capacity. `BytesPerSecond` is already a nonzero
unit type, so a zero rate cannot be constructed through the public core unit
API. The transport does not infer or substitute a nominal rate.

### `TcpPeerSession::new`

Construction takes:

* an already-connected `std::net::TcpStream`;
* a `SessionIdentity` containing nonzero local and remote machine/profile
  digests;
* `ProtocolLimits`, whose payload limit is between one byte and 64 MiB and
  whose operation timeout is nonzero;
* the `PeerDescriptor` describing the remote machine, memory/interface keys,
  link kind, duplex mode, lane bounds, and session ID; and
* `MeasuredLocalMemory` for the local endpoint.

The constructor requires `descriptor.asynchronous_submission`. It calls
`split_stream`, which makes the stream blocking, enables TCP_NODELAY, clones a
write handle, and initializes one `WireSender` and one `WireReceiver`. Both
wire halves start at sequence zero. A `ProbeConnection` stores those halves,
the identity and limits, and `next_round = 1` behind a `Mutex`. The mutex
serializes attempts on one connection, while allowing a second caller to wait
with cancellation and deadline checks.

`SessionIdentity::new` rejects equal machine identities. Endpoint identity
validation rejects a zero machine or profile digest. The transport does not
verify that a digest corresponds to a profile on disk; the caller and the
cluster/probe validators bind the digest to the canonical measured profile.

### `PeerSession` implementation

`descriptor()` returns the stored descriptor unchanged.

`benchmark(plan)` creates a fresh `PeerBenchmarkControl` whose absolute
deadline is `now + plan.maximum_duration`, then invokes the controlled path and
converts the resulting `PeerBenchmarkAttempt` with
`into_measurement()`. A deadline that cannot be represented is returned by the
`recipe_probe::ProbeError` control constructor before any wire operation.

`benchmark_controlled(plan, control)` runs the transport-specific state
machine. Success is `PeerBenchmarkAttempt::Measured(PeerMeasurement)`; every
failure is `PeerBenchmarkAttempt::Failed(PeerBenchmarkFailure)` with a phase,
failure kind, protocol schema, and human-readable detail.

### Descriptor fields and pass-through state

`PeerDescriptor` is the caller's declared identity and topology snapshot. It
contains a session ID; the remote machine fingerprint; local and remote memory
keys; the local interface key and remote interface identity; remote driver and
firmware labels; link identity and transport kind; duplex mode; outbound and
inbound maximum-inflight counts; and the asynchronous-submission flag. The
transport reads only the duplex for begin validation and directional scheduling,
and the asynchronous flag at construction. It returns the rest unchanged so
`ProbeEngine` and cluster assembly can validate exact discovery identities,
interfaces, drivers, firmware, link kind, and lane bounds. None of those labels
are placed in a probe payload; the endpoint digests in the common frame header
authenticate the connection, while the descriptor is validated by its caller.

`PeerBenchmarkControl` carries only an absolute `Instant` deadline and a
cloneable cancellation flag. It is caller state, not a wire request. The
transport derives a shorter child deadline for one attempt and never mutates
the caller's control object.

## Attempt state and deadlines

`run_benchmark` applies the following order:

1. Check caller cancellation or absolute deadline in `Validation`.
2. Validate the plan's local bounds.
3. Compute an attempt deadline as the earliest of `now + plan.maximum_duration`
   (or the caller deadline if the `Instant` addition overflows) and the caller
   absolute deadline. A child `PeerBenchmarkControl` carries that deadline and
   a clone of the caller's cancellation state.
4. Acquire the connection mutex with `try_lock` and `thread::yield_now`. The
   loop checks the child control between lock attempts. A poisoned mutex is a
   validation-phase failure.
5. Reject a buffer larger than the connection's protocol payload limit.
6. Reserve and increment the connection round with checked `u32` arithmetic.
7. Execute the begin, ready, directional-transfer, and completion phases.

For each framed control exchange, `operation_deadline` checks control first,
then takes the earliest of `now + ProtocolLimits::operation_timeout` and the
child absolute deadline. Data samples additionally use a directional phase
deadline, itself the earliest of a fresh `now + plan.maximum_duration` and the
child deadline. `transfer_operation_deadline` takes the earliest of that phase
deadline, the child deadline, and the per-operation timeout. Cancellation and
deadline are checked before each sample and after each directional loop.

Cancellation is advisory between framed operations. A blocking read or write
is still bounded by the deadline installed on the `TcpStream`; there is no
unbounded wait. The cancellation check has precedence over the deadline in the
`PeerBenchmarkControl` failure helper. The same connection cannot run two
attempts at once.

## Wire framing shared with runtime transport

Probe frames use the common 200-byte transport header. All integer fields are
big-endian.

| Offset | Size | Field |
| ---: | ---: | --- |
| 0 | 8 | Magic `RCPTRN01` |
| 8 | 2 | Wire protocol version, currently `1` |
| 10 | 1 | `FrameKind` (`ProbeBegin = 1`, `ProbeReady = 2`, `ProbeData = 3`, `ProbeComplete = 4`) |
| 11 | 1 | Reserved flags, must be zero |
| 12 | 4 | Payload length as `u32` |
| 16 | 8 | Directional wire sequence |
| 24 | 8 | Nonzero `CompletionToken` |
| 32 | 8 | Schedule, `u64::MAX` for probe frames |
| 40 | 32 | Sender machine digest |
| 72 | 32 | Sender profile digest |
| 104 | 32 | Receiver machine digest |
| 136 | 32 | Receiver profile digest |
| 168 | 32 | SHA-256 payload digest |

The sender writes its `SessionIdentity::local` values into the sender fields
and its expected remote values into the receiver fields. The peer's receiver
checks the opposite orientation exactly. The receiver also checks magic,
version, reserved flags, frame kind, payload length against `ProtocolLimits`,
and the next expected sequence before reading the payload. It checks the
payload SHA-256 before advancing its sequence. A send advances its sequence
only after the complete header and payload write succeeds.

`FrameMetadata::new` permits no schedule on probe frames. Schedules are a
runtime-only property of `UserData` frames. `WireSender::send` refuses a
payload above the configured limit. A write failure poisons its stream half
and shuts down both directions. Any receive failure, including malformed header,
identity mismatch, bad sequence, digest mismatch, or a short read, likewise
poisons and shuts down the receiver half. There is no retry or alternate
transport path.

`ProtocolLimits` itself caps payloads at 64 MiB, also at `u32::MAX`, and
requires a nonzero operation timeout. `RuntimeChannel` uses the same framing
but sets the stream nonblocking and accepts only `Control`, `Metrics`, and
`UserData`; seeing any of the four probe kinds is an invalid runtime frame.
Probe and runtime phases therefore cannot be silently mixed on a channel.

## Paired phase protocol

Both endpoints run the same sequence. The token formula is:

```text
token(round, phase, iteration) =
    (u64(round) << 32) | (u64(phase) << 24) | u64(iteration + 1)
```

`round` starts at one and advances once after local validation and payload-limit
checks, before the begin exchange. Begin, ready, and complete use
`iteration = 0`; data uses the zero-based data iteration. The
maximum plan iteration bound leaves `iteration + 1` safely representable, and
`CompletionToken::new` rejects zero.

### Begin exchange

Each endpoint sends a `ProbeBegin` with phase-one token, then receives the
peer's phase-one frame. Its payload is exactly 40 bytes:

| Offset | Size | Value |
| ---: | ---: | --- |
| 0 | 2 | Peer benchmark payload schema, `PEER_BENCHMARK_PROTOCOL_SCHEMA = 1` |
| 2 | 1 | Duplex: `1` for full duplex, `2` for half duplex |
| 3 | 1 | Reserved byte, must be zero |
| 4 | 8 | `BoundedBenchmarkPlan::buffer_bytes` |
| 12 | 4 | `BoundedBenchmarkPlan::iterations` |
| 16 | 8 | `maximum_duration` in nanoseconds |
| 24 | 8 | Sender's local memory capacity |
| 32 | 8 | Sender's local memory transfer rate |

The receiver requires the exact payload length and schema, checks the duplex
against its descriptor, reconstructs the remote plan, and requires exact plan
equality. The remote capacity must be nonzero and the remote rate must decode
as a nonzero `BytesPerSecond`. Endpoint identities are not duplicated in this
payload because they are authenticated by the frame header.

Both sides send before either side receives. A peer that has not entered the
same attempt will therefore leave its matching receive blocked until the
bounded deadline.

### Ready exchange

Each endpoint sends a zero-length `ProbeReady` with phase-two token and then
receives an exact zero-length frame with that same kind and token. The common
`receive_exact` helper rejects any frame arriving with a different kind or
token, so a valid header alone cannot advance the phase.

### Directional transfer

The local payload is allocated once at `buffer_bytes` bytes and filled with the
round pattern:

```text
probe_pattern(round) = round.to_le_bytes()[0] XOR 0xa5
```

For every iteration, the sender transmits one `ProbeData` frame containing the
whole payload and the phase-three token. The receiver reads the exact frame,
checks the common frame digest and identity, and checks every byte against the
round pattern. A wrong byte is an `IntegrityFailure`.

Full-duplex links run `measure_send` and `measure_receive` concurrently. A
two-party barrier releases the receiver thread and the sending thread at the
same point. The receiver thread owns the `WireReceiver`; the caller thread
owns the `WireSender`. The result is marked `PeerDuplexExecution::Simultaneous`.

Half-duplex links must serialize the directions to honor one shared capacity
resource. Both peers independently compare the raw machine-digest bytes. The
endpoint with the lower digest sends all outbound samples first while its peer
receives them; then the first endpoint receives all inbound samples while the
peer sends. The other endpoint chooses the inverse order from the same
comparison. This deterministic tie-break avoids a cross-wait without an extra
control frame. The result is marked `PeerDuplexExecution::Serialized`.

`measure_send` starts a sample timer immediately before `send_exact` and
records after the complete framed write. This measures local write completion,
not an acknowledgement from the peer. `measure_receive` starts immediately
before `receive_exact` and records after payload digest and round-pattern
validation. It includes the receive-side validation work. Every operation is
bounded by the transfer deadline and protocol operation timeout.

### Completion exchange

After both directions finish and evidence is computed, each endpoint sends a
`ProbeComplete` with phase-four token and a 16-byte payload:

| Offset | Size | Value |
| ---: | ---: | --- |
| 0 | 8 | Locally measured outbound bytes per second |
| 8 | 8 | Locally measured inbound bytes per second |

Each endpoint then receives its peer's matching frame and validates the exact
length and that both values decode as nonzero `BytesPerSecond`. The received
rates are not copied into the local measurement and are not compared against
the locally derived values. Their purpose here is phase completion and wire
validity; each endpoint retains its own independently timed evidence.

## Timing, evidence, and returned measurement

Each direction owns a `DirectionAccumulator`. Its `started` instant is taken
before that direction's loop. Each sample duration is canonicalized to at
least one nanosecond and must fit `u64`. The accumulator records:

* sample count;
* minimum and maximum sample duration;
* checked sum of sample durations; and
* checked sum of squared sample durations.

`finish(total_bytes)` rejects an empty sample set, takes elapsed wall-clock time
from the accumulator's start, and emits `DirectionalBenchmarkEvidence`:

```text
total_bytes                 = buffer_bytes * iterations
elapsed_nanoseconds         = wall time from direction start to finish
sample_count                = iterations
mean_sample_nanoseconds     = floor(sum / sample_count)
variance_nanoseconds_squared = max(floor(sum_squared / sample_count)
                                   - mean * mean, 0)
```

The variance expression is integer and saturating. Duration sums, sample counts,
and rate arithmetic use checked operations and produce a probe failure on
overflow. Squared sample durations are widened from `u64` into `u128`, where a
single squared value fits by construction. The derived directional rate is:

```text
rate = clamp((total_bytes * 1_000_000_000) / elapsed_nanoseconds,
             1, u64::MAX)
```

The multiplication and division occur in `u128`; the result must fit the
`BytesPerSecond` schema. Rates are marked with
`PropertyProvenance::Measured`, never `Estimated` or `Override`.

On success the returned `PeerMeasurement` contains:

* the remote capacity and transfer rate sent in the peer's begin payload,
  both marked measured;
* the locally measured outbound and inbound rates, both present and marked
  measured; and
* `PeerBenchmarkEvidence` with protocol schema, the exact local and remote
  machine/profile digests from the session, the simultaneous/serialized mode,
  and the two complete directional evidence records.

The transport result does not include the descriptor or benchmark plan. The
caller retains those alongside the measurement, which is why cluster assembly
accepts all of `SessionIdentity`, `PeerDescriptor`, `MeasuredLocalMemory`,
`PeerMeasurement`, and `BoundedBenchmarkPlan` in `MeasuredNetworkPair`.

## Profile codec handoff

The paired wire payload is not the persisted profile payload. After
`ProbeEngine` accepts a measurement, `recipe-probe::MeasuredProfileCodec`
serializes the profile for a cache or cluster handoff. The codec is a separate
canonical binary format: numeric values are little-endian, it begins with the
16-byte `RECIPEPROFILE` magic, carries codec schema 7, profile/cache schemas
and the cache digest, then origins, benchmark metadata, peer benchmarks,
topology, and discovery, and ends with a 32-byte SHA-256 checksum over all
preceding bytes. Encode validates before serialization; decode checks the
256 MiB profile limit, minimum size, checksum, magic, codec schema, complete
consumption, and the reconstructed profile.

The peer-benchmark portion of that payload is a length-prefixed list. Each
entry contains the session label, outbound and inbound `Property<BytesPerSecond>`
(value plus provenance tag), then `PeerBenchmarkEvidence`:

* protocol schema (`u16`), local machine/profile digests, and remote
  machine/profile digests;
* execution tag `0` for simultaneous or `1` for serialized; and
* outbound and inbound directional records, each containing total bytes,
  elapsed nanoseconds, sample count, minimum, maximum, mean, and `u128`
  variance.

The codec preserves the complete timing and identity evidence rather than
only the two rates. Its profile validator requires the current peer schema,
nonzero distinct endpoint machines and profiles, plan-matching total bytes and
sample count, nonzero bounded elapsed times, ordered sample statistics, and a
rate exactly equal to the duration-derived rate. Canonical measured-profile
validation also rejects an unmeasured property. Thus a valid cache entry cannot
replace the paired measurement with a nominal or partially reconstructed
network value.

## Bounds and invariants

The transport and probe layers enforce different parts of the bound:

* `BoundedBenchmarkPlan::is_bounded` requires nonzero buffer, iterations, and
  duration. Transport rejects more than 1,000,000 iterations and requires
  `buffer_bytes * iterations` to fit and remain at or below 8 GiB.
* A plan duration must encode as `u64` nanoseconds for the begin payload. The
  payload buffer must fit `usize` before allocation. The connection's protocol
  limit is checked separately before sending the begin frame.
* Begin payloads are exactly 40 bytes, ready payloads exactly zero bytes,
  data payloads exactly `buffer_bytes`, and complete payloads exactly 16 bytes.
  The plan, duplex, schema, reserved byte, frame kind, token, sequence, and
  endpoint orientation all have exact checks.
* The common protocol limits payloads to one byte through 64 MiB and requires
  a nonzero operation timeout. A runtime channel additionally requires each
  fixed lane and receive buffer capacity to be nonzero, but those runtime
  capacities are not used by `TcpPeerSession`.
* A round cannot wrap, a wire sequence cannot wrap, and a completion token
  cannot be zero. Any checked exhaustion is a protocol-state failure.
* The connection mutex prevents interleaved attempts. A poisoned wire or
  mutex is never recovered by reopening, replaying, or selecting a different
  implementation.

The `recipe-probe` engine adds the profile-level invariants. It derives the
network plan from the seed as `ethernet_rate / 8`, clamps the buffer to 4 KiB
through 64 MiB, uses eight iterations, and limits the plan to two seconds.
Before calling a session it requires unique session IDs and remote machine
fingerprints, a nonlocal peer, known local memory and network-interface keys,
matching local transport kind and duplex, and asynchronous submission. After
the attempt it requires measured remote memory values, both measured
directions, the current peer schema, nonzero distinct endpoint digests, the
duplex-consistent execution mode, and evidence whose total bytes, elapsed time,
sample count, sample ordering, and derived rate agree with the plan.

## Failure phases and conversion

Every terminal error from `run_benchmark` is attributed to one of these
phases:

* `Validation`: control, plan, connection-lock, round, or payload-limit
  checks;
* `BeginExchange`: begin encode/send/receive/decode or remote plan disagreement;
* `ReadyExchange`: ready frame send/receive or phase mismatch;
* `DirectionalTransfer`: allocation, send/receive, pattern or digest failure,
  timing arithmetic, cancellation, or deadline during data movement; and
* `CompletionExchange`: completion encode/send/receive, rate decoding, or
  final control check.

`benchmark_failure` maps transport errors to the structured
`PeerBenchmarkFailureKind`:

| `TransportError` family | Failure kind |
| --- | --- |
| `Cancelled` | `Cancelled` |
| `DeadlineExceeded` | `Deadline` |
| `InvalidIdentity`, `UnexpectedIdentity` | `Identity` |
| `IntegrityFailure` | `Integrity` |
| Unsupported version, invalid frame, unexpected sequence, protocol state, or unknown completion | `Protocol` |
| Invalid configuration, frame/buffer limits, closed connection, I/O, capacity, probe arithmetic, or poisoned state | `Transport` |

`PeerBenchmarkAttempt::Failed` preserves the phase and detail. The normal
`benchmark` entry point converts it to a `ProbeError::Benchmark` only through
`into_measurement`, so a failure cannot masquerade as a measured property.
`ProbeEngine` stops before profile construction or cache storage. There is no
nominal link-rate fallback, retry, alternate codec, or partial directional
measurement path. The wire helper retries interrupted system calls, and the
common I/O conversion maps a read/write timeout or `WouldBlock` to
`DeadlineExceeded`; other I/O kinds remain transport failures.

## Admission to a distributed measured profile

For a fresh `ProbeEngine::probe`, each successful paired measurement is kept
with its descriptor and inserted into `MeasuredProfile::peer_benchmarks`. The
profile digest includes the session ID, remote memory values, both directional
rates, protocol schema, endpoint digests, execution mode, and every directional
evidence field. The topology builder creates a remote machine and remote RAM
device for each peer, then creates two directed links between the local memory
domain and that remote RAM device. Outbound/inbound measured rates and the
descriptor's lane limits become the corresponding link properties; full and
half duplex map to the core topology duplex mode. The remote machine and
memory discovery keys are retained in measured origins.

Cluster assembly consumes the same facts through
`MeasuredNetworkPair::from_probe`. It requires a bounded plan, asynchronous
submission, an Ethernet or WLAN transport, the descriptor's required duplex,
measured local and remote memory/rates, and evidence bound exactly to the
`SessionIdentity` endpoints and current peer schema. It rejects stale plans,
missing or duplicate configured pairs, descriptors that do not match the
remote member fingerprint, and endpoint/profile mismatches. Once admitted, the
pair contributes the two directed network links, the remote worker RAM
gateway, and a `MeasuredPeerBenchmark` record to the assembled profile.

This is the complete distributed role of the transport probe: an explicitly
paired, bounded measurement supplies authenticated, measured link evidence for
discovery and cluster assembly. It does not establish the cluster, transport
runtime channels, schedule tasks, or perform any model work. Runtime execution
starts only after discovery, planning, realization, and finalization, and the
runtime channel will reject a probe frame if one is sent in that later phase.
