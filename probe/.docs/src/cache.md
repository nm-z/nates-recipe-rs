# Measured-profile cache

The probe cache is the persistence boundary for one fully measured
`MeasuredProfile`. It keeps a validated topology and discovery profile on disk
so a later invocation can reuse the exact measurements for the same machine,
seed, devices, toolchains, and peer descriptors. The implementation is in
`probe/src/cache.rs`; the data contract is in `probe/src/model.rs`, the identity
builder is in `probe/src/engine.rs`, and the binary format is in
`probe/src/codec.rs`.

The cache is deliberately explicit. `ExplicitPathProfileCache` has no default
location, does not search a directory, does not select the newest file, and
never replaces a different file at its configured path. A caller chooses one
absolute file path and supplies the `CacheIdentity` it expects. Keeping more
than one profile therefore means choosing more than one identity-derived path.

## Contract and ownership

`probe/src/model.rs` defines the small interface used by the engine:

```rust
pub trait ProfileCache: core::fmt::Debug {
    fn load(&self, identity: CacheIdentity)
        -> ProbeResult<Option<MeasuredProfile>>;
    fn store(&self, profile: &MeasuredProfile) -> ProbeResult<()>;
}
```

`CacheIdentity` is a schema number and a 32-byte `recipe_core::Digest`.
`MeasuredProfile` carries the same identity in `cache_identity`, in addition to
its profile schema, stable origins, benchmark plans and evidence, topology, and
discovery profile. `MeasuredProfile::is_cache_valid_for` expresses the same
predicate used by callers: the profile schema must be `PROFILE_SCHEMA` (7) and
the complete `CacheIdentity` must equal the current one. The concrete cache
performs the content check after decoding, rather than trusting the path or the
caller alone.

`probe/src/cache.rs` owns filesystem policy and delegates profile meaning to
the codec. It does not discover hardware, run benchmarks, derive identities, or
resolve a profile back to live devices. Those operations remain in the engine
and resolver, which means a cache hit can only return a profile that has passed
the same validation as a newly produced profile.

## Path and filename layout

`ExplicitPathProfileCache::new` accepts a path only when it is absolute and has
a file name. It stores the path unchanged. On every load or store, the parent
directory is required to be an existing, real, canonical directory owned by the
effective user with no group or other permission bits. A lexical path that
resolves through `..`, a parent symlink, a non-directory parent, an insecure
directory, or a directory owned by another user is an error. The cache never
creates the parent directory.

The command-line probe supplies the normal path. `src/cli.rs::private_state_root`
selects an absolute `XDG_CACHE_HOME`, or the canonical `$HOME/.cache`, then
uses `<base>/recipe-next`. `recipe probe` creates private `scratch` and
`profiles` directories with mode `0700`. Unless `--profile` is supplied, the
profile path is:

```text
<private-state-root>/profiles/measured-v<schema>-<64 lowercase hex>.recipe-profile
```

For the current schema this is `measured-v7-<digest>.recipe-profile`. The
digest is the lowercase hexadecimal form of `CacheIdentity.digest`. An
explicit `--profile` path is passed directly to `ExplicitPathProfileCache`, so
its absolute, canonical, ownership, and permission requirements still apply,
but the CLI does not create its parent on the caller's behalf.

Native preparation uses the same name contract. The private
`cache_identity_from_path` parser in `src/native_prepare.rs` requires the
`measured-v<decimal-schema>-<64 lowercase hex>.recipe-profile` shape and turns
it into a `CacheIdentity`. This syntax check does not make the filename
authoritative: the file contents must decode, validate as profile schema 7,
and carry exactly the parsed identity.

## Cache identity and key derivation

`ProbeEngine::inspect` performs discovery and normalization without running
benchmarks. It requires exhaustive, non-empty GPU discovery, sorts GPU
descriptors by their stable keys, sorts RAM, storage, and network domains by
key, sorts peer descriptors by session id, checks uniqueness and all local
memory/interface references, and then calls `build_cache_identity`. The
public `current_cache_identity` method exposes that exact discovery-only
calculation. This is why checking a cache does not silently run a measurement,
and why a change in an identity input causes a miss rather than reuse of an
approximately similar profile.

`build_cache_identity` starts a `CanonicalDigest` with domain
`recipe-probe-cache-v7` and schema `PROFILE_SCHEMA`, then hashes the following
values in their source order:

* The complete seed identity: seed schema, all numeric estimates, storage
  reservation, invalidation facets, and named transport/duplex entries.
* The machine fingerprint: hostname, stable machine id, runtime ABI, and
  firmware.
* Every RAM domain: key, capacity hint, link identity, and maximum in-flight
  transfers.
* Every storage domain: key, name, capacity hint, benchmark root, host-memory
  key, driver, firmware, link identity, transport kind, asynchronous and duplex
  flags, and read/write concurrency limits.
* Every network interface: key, name, address, driver, firmware, link identity,
  transport kind, asynchronous and duplex flags.
* Every GPU descriptor: key, name, capacity hint, host-memory key, complete
  target identity, driver, runtime ABI, firmware, link identity, transport
  kind, complete toolchain identity, asynchronous and queue/concurrency
  limits, subgroup/workgroup/shared-memory limits, transfer overlap, duplex,
  and both directional in-flight limits.
* Every peer descriptor: session id, remote machine fingerprint, all local and
  remote memory/interface identities, remote driver and firmware, link
  identity, transport kind, asynchronous and duplex flags, and both directional
  in-flight limits.

`CanonicalDigest` length-prefixes byte strings with a little-endian `u64`,
encodes numeric values as little-endian integers, encodes booleans as one byte,
appends digest bytes directly, and finishes with SHA-256. The resulting digest
is paired with schema 7 in the `CacheIdentity`. The measured topology and
discovery identities are also derived later from this cache identity plus the
actual benchmark values, so a cache hit preserves the identity chain used by
native preparation and scheduling.

## On-disk codec

The cache file is one canonical binary value produced by
`MeasuredProfileCodec::encode`. It is not TOML, JSON, a journal, or a directory
of fragments. The payload order is fixed:

| order | encoding | contents |
| --- | --- | --- |
| 1 | 16 raw bytes | `RECIPEPROFILE` magic followed by three NUL bytes |
| 2 | little-endian `u32` | codec schema, currently `PROFILE_CODEC_SCHEMA = 7` |
| 3 | little-endian `u32` | `MeasuredProfile.schema` |
| 4 | little-endian `u32` | `cache_identity.schema` |
| 5 | 32 raw bytes | `cache_identity.digest` |
| 6 | section | measured origins |
| 7 | section | benchmark metadata |
| 8 | section | peer benchmark records and evidence |
| 9 | section | topology |
| 10 | section | discovery profile |
| 11 | 32 raw bytes | SHA-256 of every preceding payload byte |

The section details are part of the same versioned format:

* Origins contain length-prefixed machine records (machine id and all four
  fingerprint labels), followed by RAM, storage, and GPU origin lists. Each
  device origin contains its numeric device id and stable discovery key.
* Benchmark metadata contains the seed schema and four bounded plans in the
  order RAM, storage, GPU, network. Each plan stores buffer bytes (`u64`),
  iterations (`u32`), and maximum duration in nanoseconds (`u64`).
* Each peer record stores its session label, measured outbound and inbound
  rates, protocol schema, local and remote endpoint machine/profile digests,
  duplex-execution tag, and complete outbound/inbound directional evidence.
* Topology stores its identity, ordered machines, nodes and device membership,
  devices with typed measured properties, and directed links with transport,
  duplex, endpoints, measured bandwidth/concurrency, and capacity resource.
* Discovery stores its identity and topology identity, then ordered device
  capabilities and ordered link capabilities, including availability,
  measured transfer properties, queue limits, asynchronous flags, and optional
  calculation targets and limits.

Primitive encoding is intentionally small and deterministic: booleans use tags
0 and 1, enum tags are fixed in `codec.rs`, labels are a `u32` byte length plus
UTF-8 bytes, and a persisted `Property<T>` is its numeric value followed by a
provenance tag. Lengths are capped at `MAXIMUM_ITEMS` (1,000,000), labels at
`MAXIMUM_STRING_BYTES` (1 MiB), and the complete profile at
`MAXIMUM_PROFILE_BYTES` (256 MiB). The decoder rejects truncation, invalid tags,
invalid UTF-8, invalid typed values, unsupported schema values, trailing
payload bytes, and checksum mismatch before returning a profile.

Encoding validates the complete profile before writing any bytes. Decoding
checks the size bound, verifies the trailing SHA-256 over the payload, checks
magic and codec schema, decodes every section, requires the decoder to be at
EOF, reconstructs `MeasuredProfile`, and calls the same `validate_profile`.
Therefore a successfully decoded file is not merely parseable; it is a
canonical, schedulable measured profile.

`validate_profile` enforces the cache invariants that matter at this boundary:

* profile and cache schema are 7 and the cache digest is nonzero;
* seed schema and every benchmark plan are bounded and canonically representable;
* peer protocol, endpoint identities, directional sample counts, durations,
  byte totals, rate derivations, and endpoint distinctness agree;
* topology, scheduling properties, and discovery validate independently;
* every topology machine and measured device has exactly one stable origin,
  origins point to the right device kinds, and keys are unique per machine;
* all origin, peer, topology, and discovery lists are in strict canonical
  numeric/session order;
* every persisted capacity, rate, bandwidth, and concurrency property has
  `Measured` provenance rather than an estimate or override;
* topology and discovery contain matching properties for every device and link;
* every peer benchmark has matching cross-machine topology transport evidence.

## Loading one file

`ExplicitPathProfileCache::load` is a thin call to `load_existing`, whose state
transitions are deliberately fail-closed:

1. Validate the parent directory as described above.
2. Inspect the target with `symlink_metadata`. A missing target returns
   `Ok(None)`, and every other metadata error is an I/O error. An existing
   target must be a regular, non-symlink file, owner-only in its permission
   bits, and owned by the effective user.
3. Open it with `O_NOFOLLOW | O_CLOEXEC`, inspect the opened descriptor, and
   compare device and inode with the metadata from step 2. A changed target is
   rejected instead of being read through a race.
4. Reject a file larger than `MAXIMUM_PROFILE_BYTES`, allocate exactly its
   checked length, and read it completely.
5. Decode with `MeasuredProfileCodec` and compare the decoded
   `profile.cache_identity` to the caller's requested identity. A mismatch is
   the explicit `stale cache identity` error, not a cache miss.
6. Return `Ok(Some(profile))` only after all codec and profile validation has
   succeeded.

Thus only absence is a miss. A symlink, directory, insecure file, ownership
change, race, oversized file, malformed bytes, invalid profile, checksum
failure, or stale identity is an error and must remain visible to the caller.

## Storing one profile

`ExplicitPathProfileCache::store` makes installation an atomic, no-replacement
state transition:

1. Encode and validate the supplied profile. No temporary file exists until
   this succeeds.
2. Validate the parent directory and call `load_existing` with the profile's
   own identity. If the target contains the same decoded profile, return
   success idempotently. If it contains a different profile with that same
   identity, return `cache target ... already contains a different profile`.
   A target carrying another identity is rejected by the stale-identity check.
3. Allocate a same-directory temporary name
   `.<target>.tmp.<process-id>.<nonce>` with `create_new`, `O_NOFOLLOW`,
   `O_CLOEXEC`, and mode `0600`. The process-local atomic nonce is tried for at
   most 64 collisions. The file is explicitly reset to owner-only mode.
4. Write all encoded bytes, call `sync_all` on the temporary file, close it,
   and install it with `renameat_with(..., RenameFlags::NOREPLACE)`. The rename
   is same-directory and therefore atomic while `NOREPLACE` prevents replacing
   a file that appeared concurrently.
5. If a race created the exact same profile before the rename, reload and
   return success. Any other rename failure is returned as an I/O error. After
   a successful install, `sync_all` on the parent directory commits the
   directory entry.
6. `PendingFile::Drop` removes an uncommitted temporary file on every error or
   early return. A committed temporary is left in place under its final name.

The result is either the old identical profile, one newly installed complete
profile, or an error. There is no truncation, replacement, retry loop, partial
file acceptance, or fallback path.

## Callers and end-to-end role

The cache boundary is reached through two production paths.

### `recipe probe`

`src/cli.rs::run_probe` requires bare metal, reads the selected seed contract,
discovers the host and native GPU inventory, and creates a `ProbeEngine`. It
computes `current_cache_identity` first, chooses the explicit or generated
identity-named profile path, constructs `ExplicitPathProfileCache`, and calls
`ProbeEngine::load_or_probe_and_store`.

`load_or_probe_and_store` computes the identity again through the same
discovery-only `inspect`, calls `cache.load(identity)`, and returns a validated
profile on a hit. On `Ok(None)` it calls `probe`, which runs bounded RAM,
storage, GPU, and peer measurements, builds measured topology, origins, and
discovery, validates the result, and then `probe_and_store` calls
`cache.store`. The CLI records the resulting profile path and identity in its
private active-native receipt and prints whether the path was treated as a
validated cache or a fresh measurement. The receipt is a handoff record for
native configuration; it is not another measured-profile cache format.

### Native preparation, training, inference, and acceptance

`src/native_prepare.rs::load_cached_measured_profile` parses an
identity-derived filename, constructs the same cache implementation, and
requires `load(identity)` to return a profile. Absence becomes
`NativePreparationError::ProfileNotFound`; malformed path syntax becomes
`InvalidCachePath`; all cache, codec, and profile errors remain wrapped as
`NativePreparationError::Probe`. `load_native_preparation` then builds the
native target plan from that exact profile.

`with_current_native_preparation` obtains either the active receipt's exact
path and identity or recomputes them from current discovery. It loads the
profile without probing or rewriting it, reopens exhaustive current GPU
inventory, and calls `MeasuredProfile::resolve_local_inventory`. Resolution
requires the same machine fingerprint and exact RAM, storage, and GPU stable
keys. Capacity, product name, ordinal, and benchmark similarity are never
fallback selectors. Only after this succeeds are CUDA/HSA bindings, toolchain
identities, host plans, and target specifications opened.

The production training and inference entry points call
`with_current_native_preparation`. They use the loaded measured properties to
derive runtime tuning, watchdog limits, host backend bindings, native target
plans, and candidate realization. They do not write a cache entry. Acceptance
captures the profile schema, cache digest, topology digest, discovery digest,
and every measured native target from the same preparation scope. A missing or
stale profile therefore stops native execution rather than silently falling
back to a newly guessed device or rate.

`cluster/src/assemble.rs` and `cluster/src/model.rs` also use
`MeasuredProfileCodec` for member and assembled cluster profiles. That is a
codec reuse boundary, not an alternate `ProfileCache` implementation: cluster
profiles are assembled and transported by the cluster protocol, while the
single-file local cache remains owned by `probe/src/cache.rs`.

## Failure meanings at the public boundaries

`ProbeError::Cache` describes cache policy and codec failures. `ProbeError::Io`
retains the operation and path for filesystem failures. The engine propagates
these errors without converting a bad file to a miss. The CLI renders them as
command errors. Native preparation keeps the distinction between a syntactically
invalid identity path, a missing exact profile, and a profile that failed cache
or validation checks. This separation is important operationally:

* `Ok(None)` means only that the selected regular file does not exist after its
  parent was validated.
* A stale filename or stale file contents means the requested identity no
  longer describes that file and must be repaired by selecting the correct
  identity-derived path or running a fresh probe.
* A different profile at an occupied path is never overwritten. Choose a new
  path or remove the old one through an explicit user action.
* A missing profile during `recipe probe` is the normal fresh-measurement path;
  a missing profile during native preparation is a hard failure because native
  execution must use an already measured profile.

These transitions preserve one authoritative measured state from discovery and
benchmarking through codec validation, atomic persistence, identity-exact
reload, local inventory resolution, and native execution.
