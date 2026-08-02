# `ClusterError`

`ClusterError` is the typed failure boundary for `recipe-cluster`.  It is
defined in `cluster/src/error.rs` and re-exported with `ClusterResult` from
`cluster/src/lib.rs:18-22`.  The root facade exposes the whole crate to
advanced callers as `recipe::engine::cluster` (`src/facade.rs:17-40`).

The enum covers four related boundaries:

1. user-controlled cluster configuration and its member/pair index;
2. probe-produced member profiles and authenticated peer measurements;
3. deterministic remapping and identity allocation while assembling one
   complete cluster profile; and
4. validation and serialization of the cluster-profile binary representation.

The error is fail-closed.  Every construction site returns `Err` through a
`ClusterResult`; no site substitutes an estimate, silently drops an input as a
successful path, retries a measurement, or returns a partial profile.  A caller that receives an error
does not receive the requested configuration, measured pair, identity, bytes,
or assembled `MeasuredProfile`.

## Structure

```text
ClusterError (#[non_exhaustive], Clone + Debug + PartialEq + Eq)
|-- InvalidConfiguration(String)
|-- MissingMember(String)
|-- DuplicateMember(String)
|-- StaleMember(String)
|-- InvalidMember { member: String, message: String }
|-- MissingNetworkMeasurement(String)
|-- DuplicateNetworkMeasurement(String)
|-- InvalidNetworkMeasurement(String)
|-- IdentityExhausted(&'static str)
|-- InvalidClusterProfile(String)
`-- Codec(String)

ClusterResult<T> = Result<T, ClusterError>
```

The `String` payloads are already formatted context from the rejecting
operation.  `IdentityExhausted` carries a static identity-kind label because
the failure is selected by the fixed allocator operation.  The enum has no
`From` implementation and no error source chain.  `std::error::Error` is
implemented directly (`cluster/src/error.rs:64`), so callers receive one
`ClusterError` value and its display text; lower-level errors are retained only
as text at the explicit `map_err` sites described below.

`#[non_exhaustive]` means external matches must include a wildcard for future
variants.  The current `Display` match is exhaustive over the variants above.

## Public failure boundaries and propagation

| Public boundary | Internal path that can fail | Return behavior |
| --- | --- | --- |
| `MemberProfileIdentity::derive` (`model.rs:28`) | Canonical profile encoding, per-machine shape checks, and endpoint identity construction | Returns `ClusterResult<Self>` directly. |
| `NetworkPairSpec::new` (`model.rs:139`) | `PairKey::new` rejects a self-pair | Returns `ClusterResult<Self>` directly. |
| `ClusterConfiguration::new` (`model.rs:162`) | Membership, endpoint, expected-identity, pair-reference, duplicate-pair, and connectivity checks | Returns the first failed check as `ClusterResult<Self>`. |
| `MeasuredNetworkPair::from_probe` (`model.rs:302`) | Peer descriptor, provenance, throughput, session binding, benchmark arithmetic, and evidence consistency checks | Returns `ClusterResult<Self>` before evidence is accepted. |
| `assemble_cluster` (`assemble.rs:130`) | Member indexing/preparation, common benchmark selection, network resolution, canonical digest creation, ID remapping, and final codec validation | Propagates each error with `?`; no profile is returned. |
| `ClusterProfileCodec::encode` (`assemble.rs:788`) | Cluster-shape validation, then canonical profile encoding | Shape errors are `InvalidClusterProfile`; codec errors are `Codec`. |
| `ClusterProfileCodec::decode` (`assemble.rs:793`) | Canonical decode, then cluster-shape validation | Decode errors are `Codec`; decoded shape errors are `InvalidClusterProfile`. |

The four public creation/validation APIs listed above and both codec methods
are the only public APIs in this crate that return `ClusterResult` other than
`assemble_cluster`.  Internal helpers return the same alias so `?` preserves
the original variant while unwinding through the boundary.  There are no
current in-workspace callers of the cluster APIs beyond these crate-local
paths; the crate is a workspace dependency and is re-exported for advanced
callers.

## Variant semantics

### `InvalidConfiguration(String)`

This variant means that the declared cluster shape or a canonical pair key is
not a valid configuration.  It is distinct from an invalid measured value:
configuration chooses members, addresses, expected identities, and required
pairs, while measured values are checked by `InvalidMember` or
`InvalidNetworkMeasurement`.

Construction sites are:

* `PairKey::new` (`model.rs:113-125`) rejects equal labels with
  `network pair {first} references the same member twice`.  The helper is
  reached by `NetworkPairSpec::new` (`model.rs:139-142`), by
  `MeasuredNetworkPair::pair_key` (`model.rs:391-393`), and by
  `resolve_network` while indexing submitted evidence (`assemble.rs:263-265`
  and `276`).  `MeasuredNetworkPair::from_probe` rejects equal local and
  remote labels earlier as `InvalidNetworkMeasurement`; this `PairKey` guard
  still protects the canonical-key boundary if such a value reaches it.
* `ClusterConfiguration::new` (`model.rs:162-222`) rejects fewer than two
  members; repeated endpoint addresses; repeated expected endpoint machine
  identities; repeated expected measured-profile identities; a master label
  absent from membership; a network pair whose two labels are not both in
  membership; and duplicate configured pair keys.  The exact payloads are,
  respectively, `a cluster requires at least two measured members`,
  `endpoint address {address} appears more than once`,
  `member {key} repeats an endpoint machine identity`,
  `member {key} repeats a measured profile identity`,
  `master member {master} is not in membership`,
  `network pair {pair} references a member outside membership`, and
  `network pair {pair} appears more than once`.
* `require_connected` (`model.rs:241-271`) has an empty-key guard with
  `membership is empty` and rejects a disconnected configured pair graph with
  `network-pair graph is disconnected; unreachable members: {missing}`.  The
  public `ClusterConfiguration::new` minimum-size check runs before this
  helper, so the empty-key branch is an internal invariant guard; the
  disconnected branch is reachable from the constructor.
* `common_benchmarks` (`assemble.rs:238-253`) has a `membership is empty`
  guard before selecting shared benchmark metadata.  A valid public
  `ClusterConfiguration` has at least two members, so `assemble_cluster`
  normally cannot supply an empty prepared-member slice; the guard still
  returns this variant if that internal precondition is ever violated.

`NetworkPairSpec::new` propagates the `PairKey` result before a pair can be
passed to `ClusterConfiguration::new`; the configuration constructor itself
checks the already-normalized pair keys.  Its own checks use `ensure`, which
returns `Err(error)` immediately.  During assembly,
the self-pair branch from `MeasuredNetworkPair::pair_key` propagates through
`resolve_network` and `assemble_cluster`; other configuration errors are
normally discovered when the configuration is constructed.  No invalid
configuration is converted to a member or network-measurement category.

`Display` is:

```text
invalid cluster configuration: {message}
```

The consequence is rejection of the configuration or pair before it can
define a cluster topology.  For an assembly call, the `Err` stops member or
network processing at the first failing check and no assembled profile or
derived identity is returned.

### `MissingMember(String)`

`prepare_members` constructs this variant at `assemble.rs:183-186` when a
configured `MemberSpec` key has no corresponding `SubmittedProfile` after
`index_submissions` builds its `BTreeMap`.  The payload is the configured
member key.  Configuration members are sorted by key before this loop, so the
first missing key in that order is the one reported.

The error propagates from `prepare_members` to `assemble_cluster` at
`assemble.rs:135`.  It is not produced by `ClusterConfiguration::new`, which
validates only the declaration, not the later submission vector.  Duplicate
submission keys are rejected earlier by `index_submissions` as
`DuplicateMember`, so this variant specifically means that no submission was
indexed for the requested key.

`Display` is:

```text
member {member} did not submit a measured profile
```

Assembly stops before identity derivation, benchmark comparison, network
resolution, hashing, or remapping.  No profile is assembled and no missing
member is silently omitted.

### `DuplicateMember(String)`

The same variant reports three distinct duplicate-member boundaries:

* `ClusterConfiguration::new` (`model.rs:172-176`) rejects two configured
  `MemberSpec` values with the same key.  The payload is that label.
* `index_submissions` (`model.rs:470-478`) rejects two submitted profiles with
  the same key.  `BTreeMap::insert` detects the second key; the error is
  returned immediately rather than allowing the replacement value to be used.
  `assemble_cluster` reaches this through `prepare_members`.
* `prepare_members` (`assemble.rs:197-200`) rejects a submitted profile whose
  derived endpoint profile digest has already been accepted for another
  configured member.  The payload is the current configured member key.  This
  catches two member labels carrying the same measured profile identity even
  when their labels differ.

The configured-member case propagates directly from `ClusterConfiguration::new`.
The two submission cases propagate through `prepare_members` and then
`assemble_cluster` with `?`.  A duplicate network pair has its own
`DuplicateNetworkMeasurement` category and is not represented here.

`Display` is:

```text
member {member} appears more than once
```

The rejecting boundary does not continue with an arbitrary duplicate.  A
configuration, submission set, or profile-identity set containing duplicates
therefore cannot produce an assembled profile.

### `StaleMember(String)`

`prepare_members` constructs this variant at `assemble.rs:193-196` after
`MemberProfileIdentity::derive` succeeds but the derived identity differs from
the `MemberSpec::expected()` identity.  The payload is the configured member
key.  The identity comparison covers the content-bound endpoint identity and
the cache, topology, and discovery identities carried by
`MemberProfileIdentity`; the variant does not carry a second identity or a
diff.

The check is after canonical/shape validation and before profile-identity
duplicate and machine-name checks.  An invalid profile therefore produces
`InvalidMember`, while a valid profile that is not the configured expected
profile produces `StaleMember`.  The error propagates through `prepare_members`
to `assemble_cluster` and prevents any network evidence from being resolved.

`Display` is:

```text
member {member} submitted a stale or unexpected measured profile
```

The submitted profile is not admitted as a substitute for the expected
profile, so assembly returns `Err` without hashing or remapping it.

### `InvalidMember { member, message }`

This variant identifies a configured member label and preserves the reason why
its submitted profile cannot participate.  Every construction is in
`prepare_members` or the shared benchmark check:

* `assemble.rs:187-192` maps any error from `MemberProfileIdentity::derive`
  into `InvalidMember`.  That includes canonical measured-profile encoding
  failure, a profile with a machine count other than one, a profile with a
  machine-origin count other than one, a missing machine origin, or an
  endpoint-identity constructor failure.  The lower-level error's display text
  is copied into `message`.
* `assemble.rs:201-210` reports `profile contains no machine` when the
  topology has no machine to name.
* `assemble.rs:213-219` reports
  `machine name {machine_name} is duplicated in cluster membership` when two
  accepted member profiles expose the same machine name.
* `assemble.rs:226-234` reports `profile was submitted by an unconfigured
  member` for the first leftover submission after all configured keys have
  been removed from the index.  The payload's `member` is the unconfigured
  submission key.  A missing configured key is reported earlier while the
  removal loop runs, so this branch applies only when every configured member
  was found.
* `common_benchmarks` (`assemble.rs:244-251`) reports
  `benchmark metadata differs from the other cluster members` when a member's
  benchmark seed or bounded plans differ from the first accepted member.

`prepare_members` returns these errors unchanged to `assemble_cluster`.  The
`MemberProfileIdentity::derive` mapping is intentionally a category boundary:
an inner `InvalidClusterProfile` or lower-level codec/transport text becomes
the member-specific `message`.  Consequently its displayed text can contain
the inner prefix, for example `invalid cluster profile: ...`, after the outer
member prefix.

`Display` is:

```text
member {member} has an invalid measured profile: {message}
```

The member is rejected, and assembly stops before it can contribute topology,
origins, benchmark metadata, or network gateways.  An extra submitted member
is also rejected rather than ignored.

### `MissingNetworkMeasurement(String)`

`resolve_network` constructs this variant at `assemble.rs:275-280` after each
configured `NetworkPairSpec` is normalized into a `PairKey`.  If the indexed
`MeasuredNetworkPair` map has no entry for that key, the payload is the
canonical pair display, `first<->second`.  Pair orientation is normalized by
`PairKey::new`, so evidence submitted in the reverse label order still
matches.  A self-pair in a configured `NetworkPairSpec` fails earlier as
`InvalidConfiguration`; a public `MeasuredNetworkPair::from_probe` call rejects
equal member labels as `InvalidNetworkMeasurement` before indexing.

The error propagates through `resolve_network` to `assemble_cluster` after
members and common benchmark metadata have been prepared.  It is not produced
by `MeasuredNetworkPair::from_probe`, which validates one evidence object in
isolation and does not know the required configuration pair set.  Evidence is
indexed before required configuration pairs are removed, so an earlier
duplicate or self-key error wins over a later missing-pair error.

`Display` is:

```text
network pair {pair} has no measured probe evidence
```

Assembly cannot construct the corresponding directed inter-machine links or
their measured rates without this evidence, so it returns `Err` rather than
creating an unmeasured link.

### `DuplicateNetworkMeasurement(String)`

`resolve_network` constructs this variant at `assemble.rs:262-268` when two
submitted `MeasuredNetworkPair` values produce the same normalized `PairKey`.
The payload is the canonical `first<->second` pair display.  Because
`MeasuredNetworkPair::pair_key` calls `PairKey::new`, opposite orientations of
the same two labels are duplicates as well.

The error propagates through `resolve_network` to `assemble_cluster`.  A
duplicate configured pair is rejected earlier as
`InvalidConfiguration`; an extra nonconfigured evidence pair is instead
`InvalidNetworkMeasurement`.  There is no selection rule for choosing one
measurement over another.

`Display` is:

```text
network pair {pair} has duplicate measured probe evidence
```

The network map is not accepted, and assembly stops before resolving gateways
or adding either direction of the pair to the output topology.

### `InvalidNetworkMeasurement(String)`

This variant means that peer evidence is present but cannot be trusted as the
measured inter-machine link described by the configuration.  It is constructed
at both the single-pair validation boundary and the assembly boundary.

#### Single-pair validation (`MeasuredNetworkPair::from_probe`)

`MeasuredNetworkPair::from_probe` (`model.rs:302-389`) rejects, in source
order:

* equal local and remote member labels, with `a measured pair must connect
  distinct members`;
* an unbounded benchmark plan, with `network benchmark plan is unbounded`;
* a descriptor that does not support asynchronous submission, with `peer
  transport does not support asynchronous submission`;
* a descriptor duplex that does not match the transport kind's required
  duplex, with `{transport:?} requires {expected_duplex:?} duplex operation`;
* a transport kind other than `Ethernet` or `Wlan`, with `{transport:?} is not
  an inter-machine network transport`;
* unmeasured remote memory capacity or remote memory transfer-rate provenance,
  with `{name} does not have measured provenance`;
* absent outbound or inbound throughput, with `outbound throughput measurement
  is missing` or `inbound throughput measurement is missing`; and
* outbound or inbound throughput whose provenance is not `Measured`, with
  `directional throughput does not have measured provenance`.

The private `validate_peer_evidence` call then checks the authenticated session
and benchmark arithmetic (`model.rs:396-465`).  It returns this variant when:

* the protocol schema or either endpoint identity is not exactly bound to the
  supplied `SessionIdentity`, with `peer benchmark evidence is not bound to the
  authenticated session endpoints`;
* the recorded execution mode contradicts the link duplex, with `peer
  benchmark execution mode contradicts link duplex`;
* buffer bytes multiplied by iteration count overflow, with `peer benchmark
  byte count overflowed`;
* the benchmark duration cannot be represented in canonical `u64` nanoseconds,
  with `peer benchmark duration exceeds canonical nanoseconds`;
* either throughput is absent at this validation stage; the same outbound or
  inbound missing messages are used; or
* the derived directional throughput calculation overflows, with
  `{direction} throughput calculation overflowed`, or any recorded sample
  invariant is false, with `{direction} benchmark evidence is inconsistent`.

The sample invariant requires exact total bytes and iteration count, nonzero
elapsed and minimum-sample durations, elapsed time within the plan maximum,
ordered minimum/mean/maximum sample durations, and an advertised rate equal to
the duration-derived rate.  These checks reject malformed or stale peer
evidence before it can be stored in a
`MeasuredNetworkPair`.

#### Assembly validation (`resolve_network` and `resolve_pair`)

`resolve_network` rejects a leftover evidence pair not named by the
configuration (`assemble.rs:282-290`) with `unconfigured pair {pair} supplied
probe evidence`.

`resolve_pair` (`assemble.rs:294-435`) rejects:

* an evidence-local label absent from the prepared-member index, with `unknown
  local member {label}`;
* an evidence-remote label absent from that index, with `unknown remote member
  {label}`;
* session endpoint identities that do not exactly equal the identities of the
  two named prepared members, with `session identities do not match members
  {local} and {remote}`;
* a pair benchmark plan that differs from the common member benchmark plan,
  with `pair {pair} used a stale or inconsistent benchmark plan`;
* a remote profile without a machine origin, with `remote profile has no
  machine origin`;
* a peer descriptor machine fingerprint that differs from the remote member's
  retained machine origin, with `peer descriptor fingerprint does not match
  member {member}`;
* a missing local or remote RAM origin key, with `{side} RAM origin key {key} is
  absent from member {member}`;
* a RAM origin that points to no topology device, with `{side} RAM origin {key}
  references a missing device in member {member}`;
* capacity or transfer rate that does not match the exact retained RAM origin,
  with `{side} RAM measurement does not match exact origin {key} in member
  {member}`; or
* absent outbound or inbound rates at resolution time, with the same missing
  throughput messages used by the single-pair validator.

The resolved evidence is then oriented by the canonical pair key and converted
to the two directed links.  A consistently reversed local/remote evidence
object is normalized by that step, but any other error propagates through
`resolve_network` and `assemble_cluster`; the implementation does not guess a
gateway or repair contradictory evidence.

`Display` is:

```text
invalid inter-machine measurement: {message}
```

The consequence is rejection of the evidence before a measured link, transfer
lane count, RAM gateway, benchmark evidence record, or cluster digest can use
it.  No unmeasured or contradictory network property is admitted.

### `IdentityExhausted(&'static str)`

`take_id` (`assemble.rs:110-116`) constructs this variant when incrementing an
ID counter with `checked_add(1)` would overflow.  `IdAllocator` starts each of
its counters at one (`assemble.rs:63-83`), and the static kind is one of
`machine`, `node`, `device`, `link`, `transport`, or `duplex resource`.
The allocator helpers (`machine`, `node`, `device`, `link`, `transport`, and
`resource`) propagate it with `?` (`assemble.rs:85-107`).

The error can therefore unwind from `allocate_member` while assigning remaps,
or from `push_network_pair` while allocating a transport, resource, or either
direction's link.  Each path returns through `remap_profile` and
`assemble_cluster`; there is no wraparound and no reuse of an exhausted ID.

`Display` is:

```text
{kind} identity space is exhausted
```

Assembly returns `Err` without a completed `MeasuredProfile`.  Any partially
filled in-memory collection is dropped during unwinding; no partial profile is
published by this crate.

### `InvalidClusterProfile(String)`

This variant means that a measured profile or an assembly-derived identity or
remap violates the cluster profile contract.  It is deliberately distinct
from `Codec`, which denotes a failure in the binary codec facade itself.

#### Identity derivation and hashing

`MemberProfileIdentity::derive` constructs it at `model.rs:28-63` when:

* canonical `MeasuredProfileCodec::encode` rejects the submitted profile,
  reported as `member profile failed canonical validation: {error}`;
* the per-machine topology has anything other than one machine, reported as
  `per-machine profile must contain exactly one machine, found {count}`;
* the origins list has anything other than one machine origin, reported as
  `per-machine profile must contain exactly one machine origin, found {count}`;
* the first machine origin is absent, reported as `per-machine profile has no
  machine origin`; or
* `EndpointIdentity::new` rejects a zero machine/profile digest, with that
  lower-level transport error's display text copied directly.

`CanonicalDigest::bytes` in `hash.rs:26-32` also constructs it if an identity
input length cannot be represented as `u64`, with `identity input is too large:
{error}`.  `cluster_digest` is called for topology, discovery, and profile
identity domains by `assemble_cluster` (`assemble.rs:138-158`), so the hashing
failure propagates through any of those three calls.

#### Assembly remapping

`remap_profile` and its helpers construct it when an expected ID mapping is
missing (`assemble.rs:506-537`, `719-724`).  The fixed messages are `member
remap is missing`, `member {member} has no ID remap`, and `{kind} remap is
missing`.  `kind` identifies the mapped machine, machine origin, node, node
machine, node device, device, device machine, RAM origin, storage origin, GPU
origin, link, transport, link source, link destination, capacity resource,
discovered device, network source device, network destination device, or
discovered link operation at the call site.  A missing mapping means the
assembled IDs cannot preserve topology references, so no substitute ID is
invented.

#### Final profile validation

`assemble_cluster` maps a final `MeasuredProfileCodec::encode` failure to this
variant (`assemble.rs:159-172`).  The lower-level codec text is copied into the
message, so canonical profile validation failures at this final boundary retain
their original reason.

`validate_cluster_shape` constructs it for a profile with fewer than two
machines (`assemble.rs:801-805`), a link whose endpoint device is unknown
(`assemble.rs:818-837`), a profile with no first machine (`assemble.rs:839-844`),
or a disconnected undirected inter-machine transport graph
(`assemble.rs:845-858`).  The shape validator is used by both public codec
methods.  It treats links whose two endpoint devices belong to the same
machine as non-connecting for cluster connectivity, and treats cross-machine
links as undirected adjacency for the reachability check.

`Display` is:

```text
invalid cluster profile: {message}
```

`MemberProfileIdentity::derive` returns this variant directly to its caller;
`prepare_members` may then wrap its display text in `InvalidMember` with the
member key.  Hashing, remapping, and final assembly propagate it unchanged.
The codec methods return it for shape failures before or after binary codec
work.  In every case the profile or identity is rejected rather than exposed
as a usable cluster artifact.

### `Codec(String)`

Only `ClusterProfileCodec::encode` and `ClusterProfileCodec::decode` construct
this variant (`assemble.rs:783-798`).  Both first/last boundaries are explicit:

* `encode` calls `validate_cluster_shape` first.  If shape is invalid it
  returns `InvalidClusterProfile`; otherwise a failure from
  `MeasuredProfileCodec::encode` is mapped to `Codec(error.to_string())`.
* `decode` maps any `MeasuredProfileCodec::decode` failure to `Codec`, then
  calls `validate_cluster_shape` on the decoded profile.  A decoded profile
  that is codec-valid but not a connected multi-machine cluster therefore
  returns `InvalidClusterProfile`, not `Codec`.

The wrapped probe codec currently reports the following lower-level failures,
all of which become `Codec` at this facade when they occur after the cluster
shape gate:

* `decode` rejects an input above `MAXIMUM_PROFILE_BYTES`, a truncated payload,
  a checksum mismatch, a magic mismatch, an unsupported codec schema, decoder
  field truncation or invalid tags, and unconsumed payload bytes
  (`probe/src/codec.rs:41-79`, `1358-1440`).
* Both decoded and to-be-encoded profiles run `validate_profile`, which checks
  profile and cache schemas, cache identity, bounded benchmark metadata,
  peer-benchmark arithmetic and endpoint evidence, topology and discovery
  validation, origin references, canonical ordering, exclusively measured
  properties, and topology/discovery/peer consistency
  (`probe/src/codec.rs:121-156`).
* `encode` can additionally reject item-count or label-size limits and an
  encoded byte vector above `MAXIMUM_PROFILE_BYTES`
  (`probe/src/codec.rs:98-118`, `1303-1321`).

The cluster wrapper preserves only the lower-level display text, with no
additional source object or conversion path.

`Display` is:

```text
cluster profile codec: {message}
```

The consequence is that no byte vector is returned from a failed encode and no
profile is returned from a failed decode.  Callers must handle the codec error
at the serialization boundary; the cluster crate does not fall back to an
older schema, unchecked decoding, or an alternate artifact.

## Display and trait contract

The exact formatting implemented in `cluster/src/error.rs:19-62` is:

| Variant | `Display` output |
| --- | --- |
| `InvalidConfiguration(message)` | `invalid cluster configuration: {message}` |
| `MissingMember(member)` | `member {member} did not submit a measured profile` |
| `DuplicateMember(member)` | `member {member} appears more than once` |
| `StaleMember(member)` | `member {member} submitted a stale or unexpected measured profile` |
| `InvalidMember { member, message }` | `member {member} has an invalid measured profile: {message}` |
| `MissingNetworkMeasurement(pair)` | `network pair {pair} has no measured probe evidence` |
| `DuplicateNetworkMeasurement(pair)` | `network pair {pair} has duplicate measured probe evidence` |
| `InvalidNetworkMeasurement(message)` | `invalid inter-machine measurement: {message}` |
| `IdentityExhausted(kind)` | `{kind} identity space is exhausted` |
| `InvalidClusterProfile(message)` | `invalid cluster profile: {message}` |
| `Codec(message)` | `cluster profile codec: {message}` |

Formatting is the only user-facing normalization performed by this module.
There are no log writes, status values, retries, or side effects in
`error.rs`.  `ClusterResult<T>` leaves the success type unchanged and carries
one of the typed categories above on failure.

## Failure flow summary

```text
declaration
  -> PairKey / ClusterConfiguration checks
  -> InvalidConfiguration

member submissions
  -> index_submissions / prepare_members / common_benchmarks
  -> MissingMember | DuplicateMember | StaleMember | InvalidMember

peer evidence
  -> MeasuredNetworkPair::from_probe
  -> InvalidNetworkMeasurement
  -> resolve_network / resolve_pair
  -> MissingNetworkMeasurement | DuplicateNetworkMeasurement |
     InvalidNetworkMeasurement

assembly
  -> cluster_digest / ID remapping / final profile codec validation
  -> InvalidClusterProfile | IdentityExhausted

serialized cluster profile
  -> ClusterProfileCodec::{encode, decode}
  -> Codec or InvalidClusterProfile
```

All arrows are ordinary `Result` propagation.  The first failed invariant at
each boundary determines the returned variant, and there is no conversion that
turns a failure into a successful or partially populated cluster profile.
