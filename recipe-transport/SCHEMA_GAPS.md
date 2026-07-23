# Remaining transport evidence schema gaps

`recipe-probe::PeerSession` exposes only `descriptor()` and
`benchmark(BoundedBenchmarkPlan) -> PeerMeasurement`.

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

Two control-plane extensions remain:

- a structured failure phase for unsuccessful benchmark attempts (failed
  attempts are currently rejected and therefore never cached); and
- a caller-provided cancellation token and absolute benchmark deadline in
  addition to the bounded plan and transport operation deadlines.

`TcpPeerSession` fails closed on identity, protocol, deadline, bounds, duplex,
plan, or completion disagreement and returns only genuinely measured
directional rates. It does not substitute a nominal link rate.
