# `cluster/src/hash.rs`

## Role and boundary

This private module computes the content identity for an assembled cluster. It
does not encode a `MeasuredProfile`, assign topology IDs, or authenticate a
peer session. It turns the already validated cluster inputs into one
SHA-256 digest, and the caller uses the same byte stream with three domain
labels to create the topology identity, discovery identity, and profile cache
identity.

The only caller is `assemble_cluster` in `cluster/src/assemble.rs`. The caller
has already performed the structural and measurement checks, sorted the
configuration, matched each submitted profile to its configured member, and
resolved each network measurement to a canonical `PairKey`. Keeping the
hashing boundary after those steps is important: this module is a canonical
identity calculation, not another validation or reconciliation path.

The three calls in `assemble_cluster` are:

| Domain | Destination | Meaning |
| --- | --- | --- |
| `recipe-cluster-topology-v3` | `TopologyIdentity` | Identity of the assembled topology and its measured directional links. |
| `recipe-cluster-discovery-v3` | `DiscoveryIdentity` | Identity of the assembled discovery capability snapshot. |
| `recipe-cluster-profile-v3` | `CacheIdentity.digest` with `PROFILE_SCHEMA` as its separate schema field | Identity used for the complete assembled measured profile and its cache entry. |

All three calls receive the same configuration, prepared members, resolved
network pairs, and common benchmark metadata. Domain separation means that the
same field bytes are hashed in three independent digest domains. `cluster_digest`
is `pub(crate)`, and no other crate can call it directly.

## `CanonicalDigest`

`CanonicalDigest` is a small stateful wrapper around `sha2::Sha256`. Every
method appends bytes directly to one hasher; there is no intermediate buffer,
serialization object, random salt, timestamp, process state, or platform
dependent value.

The primitive encodings are fixed as follows:

| Method | Bytes appended |
| --- | --- |
| `bytes(value)` | `value.len()` as a little-endian `u64`, followed by the bytes. |
| `string(value)` | `value.as_bytes()` passed to `bytes`; text is UTF-8 and is length-prefixed in bytes, not characters. |
| `digest(value)` | The 32 raw bytes from `recipe_core::Digest::bytes()`, with no length prefix. |
| `u8(value)` | One byte. |
| `u32(value)` | `value.to_le_bytes()`, four bytes. |
| `u64(value)` | `value.to_le_bytes()`, eight bytes. |
| `bool(value)` | `u8(0)` for `false`, `u8(1)` for `true`. |
| `finish()` | Finalizes SHA-256 and wraps its 32-byte output in `recipe_core::Digest`. |

`new(domain)` starts a stream with `string(domain)` and then
`u32(CLUSTER_SCHEMA)`. `CLUSTER_SCHEMA` is currently `4` in
`cluster/src/model.rs`. The domain is therefore length-delimited before the
schema number, so a domain cannot run into the schema bytes. The domain string
is supplied by the private caller rather than selected by a runtime enum. The
constructor does not reject an empty domain; the only in-tree caller supplies
the three fixed nonempty labels listed above.

`bytes` is the only fallible primitive. It converts the Rust `usize` length to
`u64`; conversion failure becomes
`ClusterError::InvalidClusterProfile("identity input is too large: ...")`.
Every `string` call, including the domain and all evidence labels, propagates
that error. On the ordinary 64-bit targets used by Recipe, a `usize` cannot be
larger than `u64`, but the checked conversion is part of the function's
cross-target behavior. Fixed-width integer, digest, boolean, and tag writes
cannot fail.

The wrapper does no text normalization. `Label::as_str()` supplies the exact
stored string, and `Label::new` only rejects an all-whitespace value. Case,
leading or trailing whitespace, Unicode normalization form, and every other
UTF-8 byte distinction remain identity-relevant.

There are no record counts in this stream. Member and network records are
delimited by their length-prefixed marker strings and by the known field order;
the number of records comes from the validated slices supplied by the caller.
Changing that framing, field order, or width changes the identity format and
requires an intentional schema or domain-version change.

## Top-level byte order

For one call to `cluster_digest`, bytes are appended in this exact order:

1. The length-prefixed `domain`.
2. `CLUSTER_SCHEMA` as little-endian `u32`.
3. The configuration's master label as a length-prefixed string.
4. The four common benchmark plans, through `hash_benchmarks`.
5. One member record for each `(spec, member)` pair, in the order of the two
   slices supplied to `zip`.
6. One network-pair record for every element of `network`, in slice order.
7. The SHA-256 final digest.

`cluster_digest` itself does not assert that `members.len()` equals
`configuration.members().len()`. `prepare_members` constructs the member slice
by iterating every configured member, and `assemble_cluster` passes that exact
slice, so the private call-site invariant is equal lengths and matching order.
Likewise, the network slice is produced by `resolve_network` for every
configured pair. A hypothetical in-crate caller that bypassed those stages
would get the literal `zip` truncation and whatever network records it passed,
not an additional cardinality error.

For byte-level audits, the fixed-width accounting is also deterministic. Let
`d` be the domain byte length and `m` the master-label byte length. The
top-level prefix before records is `168 + d + m` bytes. A member record is
`226 + key_bytes + address_bytes` bytes. A network record is
`492 + first_key_bytes + second_key_bytes + evidence_string_bytes`, where
`evidence_string_bytes` is the sum of the 11 evidence-label byte lengths. These
constants include every length prefix, tag, digest, integer, benchmark plan,
and peer-evidence field; the only variable portions are the explicitly named
UTF-8 string payloads.

## Benchmark metadata encoding

`hash_benchmarks` first appends `BenchmarkMetadata.seed_schema` as `u32`, then
hashes plans in this fixed order:

1. `ram`
2. `storage`
3. `gpu`
4. `network`

Each `BoundedBenchmarkPlan` contributes:

1. `buffer_bytes.get()` as little-endian `u64`.
2. `iterations` as little-endian `u32`.
3. `maximum_duration.as_nanos().to_le_bytes()` passed through `bytes`.

The duration conversion produces a fixed 16-byte little-endian `u128`, so its
encoded form is an eight-byte little-endian length of `16` followed by the 16
duration bytes. This is intentionally the hash module's encoding. The profile
codec separately stores validated durations as `u64` nanoseconds; the hash
does not reuse the codec's byte stream.

`MeasuredProfileCodec` and `MeasuredNetworkPair::from_probe` validate that
plans are bounded and that durations fit their canonical profile/evidence
representation before `cluster_digest` runs. `resolve_pair` also requires each
pair plan to equal the common `metadata.network` plan. Consequently the global
network plan and the per-pair `benchmark` field (hashed later) are normally the
same value, but both occurrences are part of the stream.

## Member records

For each configured member and corresponding prepared member, the stream is:

1. The marker string `member`.
2. `MemberSpec.key()` as a length-prefixed string.
3. `MemberSpec.address()` as a length-prefixed string.
4. The expected endpoint machine digest, raw 32 bytes.
5. The expected endpoint profile digest, raw 32 bytes.
6. The expected cache schema as little-endian `u32`.
7. The expected cache digest, raw 32 bytes.
8. The expected topology digest, raw 32 bytes.
9. The expected discovery digest, raw 32 bytes.
10. The submitted member's actual endpoint profile digest, raw 32 bytes.

The `expected` values are the `MemberProfileIdentity` carried by
`MemberSpec`. Before this function is reached, `prepare_members` derives a
fresh identity from the submitted profile and requires it to equal that
expected value. That check makes the final actual profile digest redundant in
value with the expected endpoint profile digest, but it is still appended by
the implementation and therefore is part of the exact format. The actual
machine digest and actual cache/topology/discovery fields are not appended as
separate values; the equality check makes the expected fields authoritative.

`MemberProfileIdentity::derive` obtains the endpoint profile digest by hashing
the complete canonical `MeasuredProfileCodec` byte stream. Its endpoint
machine digest is a separate SHA-256 domain calculation over the retained
stable machine ID. Therefore this module binds all canonical per-machine
profile content indirectly through the profile digest, while retaining the
configured endpoint and identity values explicitly in the cluster manifest.

That profile byte stream is itself versioned and ordered as codec magic, codec
schema, profile schema, cache schema and digest, origins, benchmark metadata,
peer benchmarks, topology, and discovery, followed by its SHA-256 checksum.
`hash.rs` does not re-encode any of those sections, so a profile-codec change
can alter the member endpoint profile digest without changing this module's
field list.

The member's local topology IDs, device IDs, link IDs, origin-vector order, and
raw profile fields do not appear directly in this stream. They are represented
by the canonical profile digest and are later remapped to cluster IDs by
`remap_profile`. Changing a profile field that changes its canonical encoded
bytes changes the endpoint profile digest and is rejected as a stale member if
the configured expectation was not updated.

## Network-pair records

For each `ResolvedNetworkPair`, the fields are appended in this exact order:

1. The marker string `network-pair`.
2. `pair.key.first` as a length-prefixed string.
3. `pair.key.second` as a length-prefixed string.
4. `pair.kind` through `transport_kind_tag`.
5. `pair.duplex` through `duplex_tag`.
6. `first_to_second.value.get()` as `u64`.
7. `first_to_second.provenance` through `provenance_tag`.
8. `second_to_first.value.get()` as `u64`.
9. `second_to_first.provenance` through `provenance_tag`.
10. `first_to_second_lanes.get()` as `u32`.
11. `second_to_first_lanes.get()` as `u32`.
12. `asynchronous_submission` as one boolean byte.
13. `first_memory_capacity` as `u64`.
14. `first_memory_rate` as `u64`.
15. `second_memory_capacity` as `u64`.
16. `second_memory_rate` as `u64`.
17. `evidence_session` as a length-prefixed string.
18. `evidence_link` as a length-prefixed string.
19. `evidence_remote_driver` as a length-prefixed string.
20. `evidence_remote_firmware` as a length-prefixed string.
21. `evidence_remote_stable_id` as a length-prefixed string.
22. `evidence_remote_runtime_abi` as a length-prefixed string.
23. `evidence_remote_machine_firmware` as a length-prefixed string.
24. `evidence_remote_memory` as a length-prefixed string.
25. `evidence_local_memory` as a length-prefixed string.
26. `evidence_local_interface` as a length-prefixed string.
27. `evidence_remote_interface` as a length-prefixed string.
28. The pair's `BoundedBenchmarkPlan` through `hash_benchmark`.
29. `evidence_outbound_rate.value.get()` as `u64`.
30. `evidence_outbound_rate.provenance` through `provenance_tag`.
31. `evidence_inbound_rate.value.get()` as `u64`.
32. `evidence_inbound_rate.provenance` through `provenance_tag`.
33. The raw peer benchmark evidence through `hash_peer_evidence`.

`first_device` and `second_device` are fields of `ResolvedNetworkPair`, but
they are deliberately not read by `hash.rs`. They are per-member topology IDs
used to construct remapped links. The identity stream instead binds the
canonical member labels, retained RAM-domain keys, measured capacities/rates,
and all peer evidence that selected those devices. The remapped cluster IDs
are allocated after hashing and must not affect this identity.

### Enum tags

The conversion helpers use explicit stable tags, not Rust enum discriminants:

| Value | Tag |
| --- | ---: |
| `TransportKind::Memory` | 0 |
| `TransportKind::Pcie` | 1 |
| `TransportKind::Sata` | 2 |
| `TransportKind::Sas` | 3 |
| `TransportKind::Nvme` | 4 |
| `TransportKind::Ethernet` | 5 |
| `TransportKind::Wlan` | 6 |

`DuplexMode::Half` is `0` and `DuplexMode::Full` is `1`.
`PropertyProvenance::Estimated` is `0`, `Measured` is `1`, and `Override` is
`2`. Each helper is exhaustive, so adding a new enum variant requires a source
change before the crate compiles. Reordering enum declarations alone does not
change these existing tags, but changing a helper's mapping changes every
affected digest.

The assembly path normally supplies `Measured` network rates and evidence, and
the profile codec requires measured persisted properties. The provenance bytes
are still written rather than assumed; a value that reached this private
function with a different valid provenance would produce a different identity.

`ResolvedNetworkPair` is built from probe evidence after `PairKey::new` orders
the two member labels lexicographically. `resolve_pair` places directional
rates, lane limits, and gateway memory values into `first` and `second` order
even when the authenticated measurement arrived with the other member local.
`ClusterConfiguration::new` sorts network-pair specifications by that key, and
`resolve_network` emits the result in that sorted configuration order. This is
why reversing the submission vector or measurement vector does not change the
cluster identity.

The same key also makes duplicate orientation evidence fail closed. Measurements
for `alpha` to `beta` and `beta` to `alpha` both index as one `PairKey`; supplying
both produces `DuplicateNetworkMeasurement` before hashing rather than allowing
one orientation to overwrite the other.

The normalization does not rewrite orientation-bearing evidence strings or
peer endpoint records. `evidence_session`, remote descriptor strings, and the
`local_endpoint`/`remote_endpoint` order inside peer evidence are hashed exactly
as they arrive in the resolved pair. Reversing an authenticated session is
therefore not an additional identity-preserving transformation; only the
first/second numeric and gateway fields are normalized.

## Peer benchmark evidence

`hash_peer_evidence` appends the complete retained `PeerBenchmarkEvidence`:

1. `protocol_schema` is widened from `u16` to `u32` and written little-endian.
2. `local_endpoint.machine`, `local_endpoint.profile`,
   `remote_endpoint.machine`, and `remote_endpoint.profile`, each as raw
   32-byte digests in that order.
3. The execution tag, `Simultaneous = 0` or `Serialized = 1`.
4. The outbound directional record, then the inbound record. Each record is:
   `total_bytes` as `u64`, `elapsed_nanoseconds` as `u64`, `sample_count` as
   `u32`, `minimum_sample_nanoseconds` as `u64`,
   `maximum_sample_nanoseconds` as `u64`, `mean_sample_nanoseconds` as `u64`,
   and `variance_nanoseconds_squared` as a raw little-endian `u128`.

The variance is written directly to the hasher rather than through `bytes`, so
it has no length prefix. The other fixed-width evidence values have no length
prefix either. There are no outbound/inbound marker bytes; their positions in
the fixed sequence distinguish them.

The evidence is not trusted merely because it is hashable. The public
`MeasuredNetworkPair::from_probe` constructor checks the protocol schema,
authenticated local and remote endpoint digests, duplex execution mode,
bounded plan, nonzero and bounded samples, sample ordering, byte totals, and
the duration-derived rates. `resolve_pair` then checks that the session matches
the configured members, the remote fingerprint matches its retained machine
origin, the benchmark is the common network plan, and both RAM gateways match
the exact measured origin capacity/rate. The hash therefore records validated
raw measurement evidence as well as the derived rates, making a change in
sample statistics identity-relevant even when a caller tried to keep the
reported throughput unchanged.

## Identity consumers

The digest values leave this module through `assemble_cluster`:

* The topology digest is placed in `Topology.identity`.
* The discovery digest is placed in `DiscoveryProfile.identity`, and the same
  topology identity is stored in `DiscoveryProfile.topology`.
* The profile digest is placed in `MeasuredProfile.cache_identity.digest` with
  `PROFILE_SCHEMA` (`7`) as `cache_identity.schema`.

`CLUSTER_SCHEMA` (`4`) versions the byte stream described here. `PROFILE_SCHEMA`
(`7`) versions the returned profile and its cache identity, while
`PROFILE_CODEC_SCHEMA` and `PEER_BENCHMARK_PROTOCOL_SCHEMA` belong to the probe
codec and peer evidence contracts. They are separate fields and are not
implicitly substituted for `CLUSTER_SCHEMA`; the member records carry each
configured member's expected cache schema explicitly, and `assemble_cluster`
sets the output cache schema explicitly.

`MeasuredProfileCodec::encode` is called before `assemble_cluster` returns. It
revalidates the profile, including nonzero cache/topology/discovery identities,
the topology/discovery identity relationship, canonical vector order, measured
provenance, and peer/topology consistency. `ClusterProfileCodec` applies the
same canonical profile codec plus the cluster shape checks when callers use the
public cluster-specific facade.

The profile cache uses the profile digest as its requested identity key; the
cache implementation's filesystem path is still explicitly supplied by its
caller and is not generated by this module. In `probe/src/model.rs`,
`MeasuredProfile::is_cache_valid_for` requires both
`PROFILE_SCHEMA` and exact `CacheIdentity` equality. In
`probe/src/cache.rs`, `ExplicitPathProfileCache::load_existing` decodes the
profile and rejects a different cache identity as a stale cache; storing a
different profile at an existing identity also fails. A cluster profile's
`recipe-cluster-profile-v3` digest therefore controls cache reuse, not just
diagnostic display.

The topology and discovery identities are carried into the generic Recipe
planning and execution contracts. Core validation rejects a zero topology or
discovery identity and requires `DiscoveryProfile.topology == Topology.identity`.
`DraftPlan::validate`, `RealizationProfile::validate`, and
`FinalizedBundle::finalize_with_loop_schedule` retain and compare these values.
Planner candidates and drafts bind both values, and preparation, native
candidate sessions, worker projections, remote provisioning, and training
resume checks reject a profile or capability snapshot whose identities differ
from the one captured during preparation. Those consumers compare the values
produced here; they do not recompute this private hash.

The member endpoint profile digest has a separate transport role. It is used by
`EndpointIdentity` and `SessionIdentity` to authenticate per-machine peer
frames. The cluster topology/discovery/profile digests are not substituted for
those endpoint identities, although the member records above bind the expected
endpoint values into each cluster digest.

## Stability guarantees and limits

The implementation provides these concrete stability properties:

* Explicit UTF-8 byte lengths, little-endian integer encodings, fixed raw
  digest widths, and explicit enum tags make the result independent of host
  endianness, Rust enum layout, and process order.
* `ClusterConfiguration::new` sorts members by key and pairs by canonical
  `PairKey`. `prepare_members` indexes submissions by key, and
  `resolve_network` indexes measurements by pair key before iterating the
  sorted configuration. Reordering those input vectors does not change the
  stream or resulting identities. This is the input-order guarantee documented
  on `assemble_cluster`.
* The common benchmark check makes every member contribute the same benchmark
  metadata, while the pair benchmark check prevents one pair from using a
  stale network plan.
* Domain separation and `CLUSTER_SCHEMA` version the topology, discovery, and
  profile domains over the same field set. Any change to the canonical field
  set or its meaning must deliberately update the schema or domain version
  rather than silently reusing old bytes.
* All cluster-hash values are content-derived. No cluster allocator IDs,
  allocator counters, filesystem paths, wall-clock times, or pointer addresses
  are hashed directly. A per-machine profile digest can still include the
  source profile's own IDs because that digest is computed by the profile codec.

The guarantee is not that every semantically similar input hashes alike. The
master label, member labels, endpoint addresses, expected identity fields,
network evidence labels, provenance tags, measured values, plan values, and
raw benchmark statistics are all identity inputs. Any change to one of those
fields changes the stream. Conversely, fields omitted by the implementation,
such as remapped device IDs, cannot change a digest unless some hashed
representation of the same change also changes.

There is also no collision detection beyond SHA-256's cryptographic digest.
`Digest` can theoretically contain all zero bytes, although the profile codec
rejects zero identities before a generated profile is accepted. The hash
module itself does not test for zero output, compare against an existing
identity, or retry with another encoding.

## Failure boundary

`cluster_digest` can return `Err` only through the fallible domain/string and
benchmark-duration writes described above. It does not emit logs, mutate
inputs, or provide a fallback hash. Every other invalid condition is rejected
before the call by the assembly and profile-validation stages, with their
specific `ClusterError` variants: invalid or stale members, duplicate or
missing submissions, inconsistent benchmark metadata, duplicate or missing
network measurements, invalid peer evidence, unknown RAM origins, mismatched
gateway measurements, or an unconfigured pair. If those stages fail, no
topology, discovery, or profile digest is installed in the output.

If a future platform can construct a string or byte slice whose length does not
fit `u64`, the direct error is `InvalidClusterProfile` with the conversion
detail from `u64::try_from`. There is no truncation. Since the hash is private,
there is no public API for a caller to recover from a partial stream or supply
an alternate representation. A successful return always means the hasher was
finalized exactly once into one 32-byte `recipe_core::Digest`.
