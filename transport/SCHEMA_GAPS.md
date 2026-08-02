# Transport benchmark evidence and control closure

`recipe-probe::PeerSession` exposes `descriptor()`, the compatibility
`benchmark(BoundedBenchmarkPlan) -> PeerMeasurement` entry point, and
`benchmark_controlled(BoundedBenchmarkPlan, PeerBenchmarkControl) ->
PeerBenchmarkAttempt`.

Probe profile schema 5 retains each discovered `MachineFingerprint` and the
stable discovery key for every RAM, storage, and GPU device. Cluster assembly
therefore uses the fingerprint's stable machine ID for transport endpoint
identity and exact RAM-origin keys for gateway selection; it no longer derives
either from profile content or measured capacity/rate.

Probe profile schema 5 preserves the exact machine/profile digests
authenticated by the transport frame, simultaneous-versus-serialized execution,
per-direction bytes and elapsed time, sample count, minimum/maximum/mean sample
duration, integer variance, the protocol schema, and the exact
duration-derived rates.

The control plane now retains both successful and unsuccessful attempt
semantics without allowing a failed attempt into a measured profile:

- unsuccessful attempts report the exact validation, begin, ready,
  directional-transfer, or completion phase and classify cancellation,
  deadline, identity, integrity, protocol, and transport failures;
- callers provide cloneable cancellation state and an absolute attempt
  deadline in addition to the bounded plan and per-operation transport
  deadline; and
- `ProbeEngine` always invokes the controlled attempt path, converts only
  `Measured` attempts into profile inputs, and rejects every structured failure
  before cache construction or storage.

`TcpPeerSession` fails closed on identity, protocol, deadline, bounds, duplex,
plan, or completion disagreement. It applies the earliest of the bounded-plan
deadline, caller absolute deadline, and per-operation deadline, checks
cancellation between framed operations, reports the active failure phase, and
returns only genuinely measured directional rates. It does not substitute a
nominal link rate.
