<!--
Intent: document the public identity primitives in core/src/identity.rs and
the complete producer/consumer boundary that gives each value its meaning.
The constructors in identity.rs are deliberately small. Meaning, hashing,
nonzero requirements, and cross-stage validation live in their callers.
-->

# Identity

`core/src/identity.rs` defines the shared identity values used by discovery,
planning, preparation, artifact handling, transport, execution, and persisted
model metadata. It contains no hashing algorithm, filesystem access, hardware
lookup, serialization, or domain validation beyond the `Label` constructor.
Callers compute a digest, wrap it in the type for the stage it represents, and
then validate that value at the boundary where it is consumed.

## Intent and parseable contract

The following is the contract implemented by `core/src/identity.rs`:

```text
identity_model:
  Label:
    storage: original String, including surrounding non-whitespace characters
    constructor: Label::new(value)
    accept_when: value.trim().is_empty() == false
    reject_error: LabelError("label must not be empty or whitespace")
    projection: as_str() -> &str
    display: original string
  Digest:
    storage: exactly 32 bytes
    constructor: Digest::new([u8; 32])
    zero: Digest::ZERO = [0; 32]
    projection: bytes() -> [u8; 32]
    predicate: is_zero() -> (self == Digest::ZERO)
    hashing: caller-defined; core does not hash or validate content
  typed_digest_identity:
    types: [TopologyIdentity, DiscoveryIdentity, DraftIdentity,
            CandidateIdentity, RealizationIdentity, BundleIdentity]
    storage: one Digest
    constructor: Type::new(Digest), infallible
    projection: digest() -> Digest
    predicate: is_zero() -> wrapped_digest.is_zero()
    zero_policy: rejected by the consuming validator, not by Type::new
```

The six typed identities are intentionally distinct Rust types even though
they have the same representation. A `TopologyIdentity` cannot be passed to a
function requiring a `DraftIdentity` without an explicit extraction and
re-wrapping operation. That type distinction prevents accidental stage mixing;
it is not a cryptographic namespace. `Type::new` will wrap any digest,
including `Digest::ZERO`, when a caller explicitly asks it to.

## `Label`

`Label(String)` is the validated textual identity used for names and keys that
must survive configuration, discovery, codec, and artifact boundaries.

### Construction and representation

* `Label::new(value: impl Into<String>)` consumes the value into a `String`.
* It checks `value.trim().is_empty()`. An empty string and a string made only
  of Unicode whitespace are rejected with `Err(LabelError)`.
* A non-empty value is stored exactly as supplied. The constructor does not
  trim it, normalize Unicode, reject control characters, or reject duplicate
  labels. Those constraints, when needed by a wire format or a domain, are
  imposed by that caller.
* `as_str` borrows the original string. `Display` writes the original string.
* The derived `Clone`, `Eq`, `Ord`, and `Hash` implementations compare and hash
  the stored string, not a normalized form. This makes ordering deterministic
  only for the exact bytes selected by the caller.
* `Debug` is the derived string debug representation. `LabelError` implements
  `Display` and `std::error::Error` with the message `label must not be empty or
  whitespace`.

### Where labels originate and are used

Probe discovery validates host, storage, network, GPU, and peer keys with
`Label::new` (`probe/src/engine.rs`, `probe/src/local.rs`, and
`probe/src/seed.rs`). The resulting labels identify retained origins and are
not inferred from capacity or performance. `Topology` uses labels for machine
names. `ReservationEntry` uses a label for the reservation name.

Artifact and native preparation use labels for `TargetIdentity` fields
(`backend`, `architecture`, and `abi`), `ToolchainIdentity` fields (`name` and
`version`), and the native entry symbol. Native probing creates toolchain labels
and rejects an invalid label before a target can enter an artifact catalog.
Checkpoint decoding, measured-profile decoding, and CLI receipt decoding all
call `Label::new`; malformed or whitespace-only serialized values fail at the
decode boundary. Checkpoint native metadata additionally rejects labels that
contain its tab, carriage-return, or newline delimiters after construction.

The label is an identity component, but it is not a digest and it is not a
numeric `*Id`. Numeric IDs in `core/src/ids.rs` are local collection keys. A
label can be included in a caller's canonical digest, while the label itself
remains available for origin lookup and human-readable diagnostics.

## `Digest`

`Digest([u8; 32])` is the common value for content, manifest, profile, and
stage identities. The type is deliberately algorithm-neutral: callers usually
use SHA-256, but `Digest::new` accepts any caller-computed 32-byte value.

### Construction and representation

* `Digest::new` is `const` and infallible. It does not reject all-zero bytes.
* `Digest::bytes` returns a copy of all 32 bytes. There is no borrowed byte
  accessor, hex parser, or `Display` implementation in core.
* `Digest::ZERO` is the all-zero sentinel. `is_zero` is an exact equality test.
* `Clone`, `Copy`, `Eq`, `Ord`, and `Hash` operate on all 32 bytes.
* `Debug` intentionally exposes only the first six bytes as
  `Digest(<12 lowercase hex digits>...)`. It is a diagnostic prefix, not a
  serialization format and not a collision-resistant display identity.

The zero value means absent, uninitialized, or invalid in the surrounding
contracts. It is not a valid content digest for a persisted profile, artifact,
plan, or finalized bundle. The core type itself permits it so that parsers and
builders can construct an intermediate value and report the precise failure at
their own boundary.

### Digest production rule

Every producer must define a domain, schema/version where applicable, a
canonical field order, and an encoding before calling `Digest::new`. The
identity type does not make two equal inputs equal across domains, and it does
not make different inputs unequal. Stability therefore means:

```text
same domain + same schema + same canonical bytes -> same Digest bytes
changed domain/schema/input/order -> a new Digest is expected
```

The callers below use SHA-256 and length-delimited encodings. The domain tags
are part of the digest input. Version suffixes and schema numbers are separate
inputs, so changing either is an intentional identity break.

## Typed digest identities

The macro `digest_identities!` generates the same API for six stage domains:

| Type | Stage meaning | Produced by | Main consumers |
| --- | --- | --- | --- |
| `TopologyIdentity` | One topology snapshot, including the structural and measured facts selected by its producer | `probe::engine::build_profile_digest` or `cluster::assemble_cluster` | `Topology`, `DiscoveryProfile`, Draft, Realize, Finalize, transport, worker projection, checkpoint metadata |
| `DiscoveryIdentity` | One capability snapshot associated with one topology | `probe::engine::build_profile_digest` or `cluster::assemble_cluster` | `DiscoveryProfile`, Draft, Realize, Finalize, native resume checks, checkpoint metadata |
| `CandidateIdentity` | One graph and placement assignment considered by the planner | `planner::candidate_identity` | ranked candidate stream, rejection bookkeeping, preparation observations and errors |
| `DraftIdentity` | One exact scheduled candidate, before backend realization | `planner::hash_draft` | `DraftPlan`, `RealizationProfile`, `FinalizedBundle`, remote manifest, checkpoint and executor state |
| `RealizationIdentity` | One successful backend realization and its observed resource evidence | `prepare::hash_realization` | `RealizationProfile`, `FinalizedBundle`, remote manifest, native kernel metadata |
| `BundleIdentity` | One immutable finalized execution bundle, including concrete arena layouts | `prepare::hash_bundle_with_loop_domains` | executor lifecycle, training/inference reports, remote provisioning, worker projection |

Each wrapper offers only:

```rust
Type::new(digest) -> Type
Type::digest(self) -> Digest
Type::is_zero(self) -> bool
```

They are `Copy`, `Debug`, `Eq`, `Ord`, and `Hash`. There is no implicit
conversion between wrappers, no recomputation method, and no embedded domain
tag. The producer and the validation boundary are therefore part of the
meaning of every wrapped value.

## Identity creation and hashing pipeline

The pipeline below names the current code paths and the exact identity inputs.
It is the source-of-truth map for how a `Digest` becomes one of the typed
identities.

### 1. Probe cache and measured profile

`probe/src/engine.rs` first derives a non-core `CacheIdentity`:

```text
CanonicalDigest("recipe-probe-cache-v7", PROFILE_SCHEMA=7)
  -> cache digest
```

The cache digest includes the seed contract, machine fingerprint, sorted RAM,
storage, network, and GPU descriptors, pinned driver/runtime/firmware/link
identity fields, target and toolchain labels/digests, peer descriptors, and
their concurrency limits. Discovery input is normalized and sorted before this
hash. `MeasuredProfile::is_cache_valid_for` requires both schema 7 and exact
`CacheIdentity` equality. An identity-keyed cache file with a different digest
is stale, not a near match.

After actual bounded measurements, the same engine computes two independent
profile digests:

```text
CanonicalDigest("recipe-topology-v6", PROFILE_SCHEMA=7,
               cache_digest, measured RAM/storage/GPU/peer values)
  -> TopologyIdentity

CanonicalDigest("recipe-discovery-v6", PROFILE_SCHEMA=7,
               cache_digest, measured RAM/storage/GPU/peer values)
  -> DiscoveryIdentity
```

The domain strings currently end in `v6` while the profile and codec schema is
7. They are independent version controls and must not be silently changed to
match. The two identities are assigned to the constructed `Topology` and
`DiscoveryProfile`; the latter also stores the former in its `topology` field.
The profile is then validated before it can be cached or returned.

`probe/src/codec.rs` encodes and decodes each 32-byte value explicitly. Decode
checks the byte width, schema, checksum, canonical ordering, origin metadata,
topology/discovery property equality, and nonzero cache/topology/discovery
identities through the core validators. It does not recompute the topology or
discovery digest from decoded fields; those digests remain opaque producer
claims. A cache identity mismatch is still rejected against the freshly
inspected host before a cached profile is accepted.

### 2. Cluster profile identities

`cluster/src/model.rs` derives each member's endpoint identity from the
retained stable machine label and canonical encoded profile. `EndpointIdentity`
rejects a zero machine or profile digest. `cluster/src/hash.rs` then uses
`CanonicalDigest` with `CLUSTER_SCHEMA=4` and these domains:

```text
recipe-cluster-topology-v3  -> TopologyIdentity
recipe-cluster-discovery-v3 -> DiscoveryIdentity
recipe-cluster-profile-v3   -> CacheIdentity.digest
```

The hash covers ordered member keys and addresses, expected endpoint machine
and profile digests, expected cache/topology/discovery identities, submitted
profile identities, benchmark metadata, and the measured network evidence.
`assemble_cluster` remaps local numeric IDs deterministically after identity
checking, then stores the two typed identities on the assembled topology and
discovery profile. Duplicate member machine/profile identities and stale
expected identities fail closed before assembly.

### 3. Planner candidate and draft identities

`planner/src/hash.rs::StableDigest` is the planner's stable encoder. It writes
the domain and byte fields as decimal-length-plus-colon prefixes, writes `u64`
values little-endian, writes booleans and tags as bytes, and feeds the result to
SHA-256. `Digest::new` wraps the 32-byte result.

The planner computes the following sequence in `planner/src/planner.rs`:

1. `recipe-planner-graph-v7` produces a generic graph digest from loop bounds,
   tensors, dtypes, layouts, topological kernel order, scalar programs,
   lowered program digests, stage-template IDs, and metrics.
2. `recipe-planner-candidate-v3` hashes that graph digest, the exact topology
   and discovery digests, kernel order, and each placement option's device.
   The result is a `CandidateIdentity`. Every finite assignment is checked in a
   `BTreeSet`; a collision is `PlannerErrorKind::IdentityCollision`, not a
   silently merged candidate. `PlannerSearch` and `ProgramPlannerSearch` use
   this identity as their issue/reject key. Rejecting an unknown or already
   rejected candidate is an error.
3. Each lowered stage gets a stage-scoped numeric `KernelTemplateId` from the
   first eight bytes of `recipe-planner-stage-template-v1` (program digest,
   source kernel, and stage ordinal). This is a numeric ID, not one of the six
   typed identities. Zero and collisions are rejected.
4. A deferred stage receives a reserved `ArtifactId` and an
   `ArtifactBuildRecipe`. Its provenance contains the lowered program digest
   and stage ordinal. `recipe-planner-artifact-build-v1` independently hashes
   the complete build contract into `ArtifactBuildProvenance.contract_digest`.
5. After all values, kernels, tasks, artifacts/builds, resources, arena
   objects, bindings, aliases, init images, releases, loop iterations, and
   loop domains are canonicalized, `recipe-planner-draft-v10` hashes them with
   the candidate, topology, and discovery digests. The result is a
   `DraftIdentity` stored in `DraftPlan`.

`DraftPlan::validate` requires nonzero draft and candidate identities and exact
equality with the supplied topology and discovery identities. It also requires
that every calculation reference exactly one realized artifact or deferred
build and that the corresponding artifact/build identities and contracts are
consistent.

### 4. Native artifact identities

`core/src/artifact.rs` defines the structured artifact identity carried by a
Draft or Realization. It is not a typed wrapper from `identity.rs`, but it uses
the same `Digest` and `Label` primitives:

```text
ArtifactIdentity:
  id: ArtifactId                         # reserved numeric stage ID
  digest: Digest                         # exact native image bytes
  format: Label                          # equals target.abi
  target: TargetIdentity { Label, Label, Label }
  toolchain: ToolchainIdentity { Label, Label, Digest }
  entry_symbol: Label
  kernel_template: KernelTemplateId
  resources: KernelResourceBounds
  build: Option<ArtifactBuildProvenance>
```

Prebuilt catalog entries must match the deferred stage's reserved artifact
ID, stage template, build provenance, resources, and discovered target. During
Realize, `prepare/src/production.rs` hashes the final native image bytes and
stores that SHA-256 value as `ArtifactIdentity.digest`; the runtime image is
then checked against the same digest, entry symbol, target ABI, backend, and
toolchain identity. `ArtifactIdentity::validate` rejects a zero image digest,
zero toolchain digest, zero deferred program digest, or zero deferred contract
digest. It does not recompute the image digest itself; the native preparation
boundary does that comparison against the runtime bytes.

The kernel crate independently recomputes the build-contract digest and stage
template identity before emitting IR. A stale, mutated, or near-match build
recipe therefore fails in `Realize` rather than producing a different artifact
under the same reserved ID.

`TargetIdentity` and `ToolchainIdentity` have no constructors or validators of
their own. They are ordinary structs containing `Label` values (and, for a
toolchain, one `Digest`). Discovery and native preparation supply the target
labels from measured capability, check backend/architecture/ABI compatibility,
and require the toolchain digest to be nonzero before an artifact is accepted.

Other generic `Digest` producers feed this same artifact and resume contract:

| Producer | Digest input | Consumer |
| --- | --- | --- |
| `native-probe::backend_toolchain_identity` (`recipe-native-probe-toolchain-and-benchmark-v2`) | pinned compiler/verifier/linker or assembler paths and digests, release, target configuration, and probe source digest | `ToolchainIdentity.digest` |
| `kernel::ArtifactDigest` and `src/native_prepare.rs` | exact cubin/HSACO bytes | `ArtifactIdentity.digest` and native runtime identity |
| `primitives::program_digest` | canonical lowered primitive program | copied into `ArtifactBuildProvenance.program_digest` |
| `ingest::SourceSnapshot` | exact source file bytes | native resume kernel-byte comparison |
| `executor::worker::projection_digest` | bundle/topology identities, worker assignment, and projected tasks | worker projection identity |
| `remote::ProvisionedProgram::compute_digest` | manifest, worker devices, task roles, and cross-machine transfer contracts | remote program handshake |

`ProgramDigest` in `recipe-primitives` and `ArtifactDigest` in `recipe-kernel`
are separate typed wrappers. They are converted to the core `Digest` only at
the boundary where the core artifact contract needs a generic digest. This is
another type-level distinction, not a new core identity domain.

### 5. Realization and finalized bundle identities

`prepare/src/lib.rs` creates the next two typed identities only after the exact
Draft has been realized:

```text
recipe-realization-v3:
  draft, candidate, discovery, topology digests
  deferred build contracts, aliases, tasks
  stabilization policy
  realized artifact identities
  resource manifest, reservations, every capacity snapshot
  -> RealizationIdentity

recipe-finalized-bundle-v5:
  topology, discovery, draft, realization, candidate digests
  realized artifacts and deferred builds
  aliases and tasks
  loop iteration/domain assignments
  resources, reservations, final capacity
  sorted arena layouts and allocations
  -> BundleIdentity
```

Both encoders length-prefix byte fields and use little-endian numeric values.
They sort artifacts, builds, tasks, resources, reservations, capacities, and
arena allocations where order is not already part of the contract. This makes
the result depend on semantic content, not incidental vector insertion order.

`RealizationProfile::validate` requires a nonzero realization identity and
exact predecessor equality for draft, candidate, discovery, and topology. It
also requires resources to remain byte-for-byte equal to the Draft and checks
all realized artifact, reservation, and capacity contracts.

`FinalizedBundle::finalize*` requires a nonzero `BundleIdentity`, validates the
unchanged Draft and Realization, resolves immutable value locations and arena
offsets, and stores all predecessor identities. `FinalizedBundle` exposes
accessors for bundle, topology, discovery, draft, realization, and candidate
identities. Finalize does not recompute a bundle identity; the preparation
caller supplies the one produced by `hash_bundle_with_loop_domains`.

## Propagation and consumers

The identity values form a one-way chain. A later stage may retain an earlier
identity and compare it, but it must not replace or reinterpret it:

```text
measured profile
  -> TopologyIdentity + DiscoveryIdentity
  -> CandidateIdentity
  -> DraftIdentity
  -> RealizationIdentity
  -> BundleIdentity
  -> run, remote manifest, worker projection, checkpoint metadata
```

The remaining direct consumers are intentionally narrow:

* `recipe-host` stores `BundleIdentity` in the finalized host-resource handoff
  and will not submit a task under a different finalized bundle.
* `recipe-native-executor` passes `BundleIdentity` into CUDA/HSA launch state,
  keeps topology/discovery identities in the candidate validation wrapper, and
  groups loaded modules by the kernel crate's exact `ArtifactDigest`.
* `src/native_prepare.rs` parses identity-keyed measured-profile filenames,
  converts their 64 lowercase hexadecimal characters into `Digest`, and
  rejects malformed paths before native preparation. `src/cli.rs` performs the
  corresponding profile-digest decode/display and validates native receipt
  labels.
* `src/training.rs` checks saved native topology/discovery identities and
  target/toolchain identities against the current measured preparation scope.
  `src/inference.rs` and the public training reports return the finalized
  `BundleIdentity` from their execution result.
* `recipe-transport` carries endpoint machine/profile digests in its handshake
  and hashes frame payloads. `recipe-remote` carries plan and artifact digests
  in its manifest, program, and init-image messages. These are generic content
  checks around the typed identities, not new `core::identity` wrapper types.

### Core validators

The direct core consumers are the authoritative structural checks:

| Boundary | Required identity facts | Failure code/message |
| --- | --- | --- |
| `Topology::validate` | topology identity is nonzero | `InvalidIdentity`, `topology identity must not be zero` |
| `DiscoveryProfile::validate` | discovery identity is nonzero; `discovery.topology == topology.identity` | `InvalidIdentity` or `IdentityMismatch` |
| `DraftPlan::validate` | draft and candidate are nonzero; discovery and topology exactly match supplied profiles | `InvalidIdentity` or `IdentityMismatch` |
| `ArtifactBuildRecipe::validate` | reserved IDs and program/contract provenance digests are nonzero | `InvalidIdentity` |
| `ArtifactIdentity::validate` | image and toolchain digests are nonzero; optional build provenance is nonzero | `InvalidIdentity` |
| `RealizationProfile::validate` | realization is nonzero; draft, candidate, discovery, topology exactly match; resources and artifacts agree | `InvalidIdentity`, `IdentityMismatch`, or `ResourceMismatch` |
| `FinalizedBundle::finalize_with_loop_schedule` | supplied bundle identity is nonzero; Draft and Realization validate | `InvalidIdentity` plus nested validation errors |

These validators compare typed values for exact equality. They do not compare
only a debug prefix, labels, numeric IDs, or a reconstructed near match.

### Preparation and native execution

`recipe-prepare` carries `CandidateIdentity` through every rejection. A
candidate that cannot realize, stabilize, or fit final capacity is rejected by
that exact identity; candidate exhaustion reports the identities and stages of
the failed attempts. The validated native candidate wrapper retains the exact
topology and discovery identities captured at realization and rejects a
different candidate or profile before evidence crosses the preparation
boundary.

The local native factory records an initial capacity snapshot keyed by
`TopologyIdentity` and `DiscoveryIdentity`. Warm execution must use the same
planned candidate, and finalization must produce a bundle whose draft identity
equals the realized candidate's Draft identity. The provisional warm bundle
used internally before final capacity has a placeholder nonzero identity only
to exercise address resolution; the final preparation path replaces it with
the canonical realization and bundle hashes.

The executor stores `BundleIdentity` in prepared, failed, and exited run
states, and publishes it in the `Prepared` lifecycle event. Every run handle
therefore remains tied to one immutable bundle. Worker projection derives a
generic projection digest from the bundle and topology identities plus worker
assignment and projected tasks; projection construction rejects a topology
different from `bundle.topology()`.

### Remote transport

`recipe-transport::EndpointIdentity` stores two generic `Digest` values: a
nonzero machine digest and a nonzero profile digest. A `SessionIdentity` or
`recipe-remote::RemoteIdentity` additionally requires distinct endpoint
machines. The remote hello exchanges these four endpoint digests and fixed
limits; a mismatch poisons the session.

`recipe-remote::Manifest::from_bundle` copies the finalized bundle, Draft, and
Realization digests and every artifact ID/image digest. It then computes
`recipe-remote-manifest-v1` over the protocol version and canonical artifact
list. Manifest validation rejects zero plan or artifact digests, noncanonical
artifact ordering, and a recomputed manifest mismatch. Provisioning also
requires the supplied topology identity to equal the bundle's topology.

Remote init images are independently SHA-256 hashed while chunked. The worker
compares the final image digest with the sender's digest before admitting the
image to the native driver. These image digests use `Digest`, but they are
content checks, not replacements for the typed plan identities.

### Checkpoints, resume, and public reports

`training/src/checkpoint.rs` persists native realization metadata as:

```text
program: Digest                    # canonical static training program
realization: RealizationIdentity
topology: TopologyIdentity
discovery: DiscoveryIdentity
kernels: [format, TargetIdentity, ToolchainIdentity, image Digest]
```

`compiled_training_program_digest` is SHA-256 of the canonical OGDL static
program. External input and resumed parameter bytes are deliberately excluded.
Checkpoint decode requires exactly 32 digest bytes and rejects zero values;
labels are reconstructed through `Label::new`. Manifest validation requires all
program, realization, topology, and discovery identities to be nonzero and
requires unique native target/toolchain/image identities.

When a native kernel is resumed, the current compiled program digest must
match the saved program digest, the supplied bytes must hash to the saved
kernel digest, and the current measured topology and discovery identities must
equal the saved values. The selected target and toolchain labels/digest must
also match the current target plan. A mismatch is an incompatible resume, not
an instruction to select a similar artifact or silently recompile under the
old identity.

Training and inference reports expose `BundleIdentity` for native execution
and expose `RealizationIdentity`, `TopologyIdentity`, and `DiscoveryIdentity`
through the retained native-kernel set. Semantic KNN and Bayesian preparation
has no native execution bundle and therefore exposes no bundle identity.

## Stability and invariant boundaries

1. **Typed identity is immutable value data.** All six wrappers are `Copy` and
   contain only a `Digest`. They cannot observe hardware or update themselves.
2. **Nonzero is a protocol invariant.** `Digest::ZERO` and a zero wrapped
   identity are representable intermediate values. Core validators and the
   persisted/executable boundaries listed in the failure matrix reject zero
   wherever their contract names it. Generic content checks may retain an
   opaque digest until their owning boundary compares it.
3. **Equality is exact.** Identity comparisons use all 32 bytes. A matching
   prefix, label, numeric ID, or target product name is insufficient.
4. **A digest is domain-scoped.** Equal bytes from different domains are still
   different semantic claims. Producers must include their domain and schema;
   consumers must use the typed wrapper and the expected stage field.
5. **Core does not prove provenance.** `Digest::new` does not know whether the
   bytes came from SHA-256, a canonical encoder, or untrusted input. The caller
   that owns the boundary must recompute or compare the digest when that is
   required, as native artifact, remote manifest, init-image, and checkpoint
   paths do.
6. **Numeric IDs are not content identities.** `MachineId`, `DeviceId`,
   `KernelTemplateId`, `ArtifactId`, `TaskId`, and related values are scoped
   collection keys. They may be included in a digest, but they do not replace
   topology, candidate, draft, realization, or bundle identity.
7. **Canonical ordering is producer-owned.** Probe normalizes inventory;
   planner and preparation sort unordered collections before hashing; cluster
   assembly canonicalizes members and pairs. Bypassing those normalizers can
   produce a different identity for semantically equivalent input and is a
   producer bug, not something `Digest` repairs.
8. **Identity changes fail closed.** A changed measured machine, device,
   driver/runtime, firmware, link, toolchain, graph, placement, artifact,
   capacity observation, or arena layout must produce a changed upstream
   identity or an explicit equality failure. No consumer may silently retain a
   stale later-stage value.

## Failure matrix

| Condition | First relevant boundary | Observable result |
| --- | --- | --- |
| empty or whitespace-only label | `Label::new` | `Err(LabelError)` |
| zero digest supplied to a core object | topology, discovery, Draft, artifact, Realization, or Finalize validator | `ValidationCode::InvalidIdentity` |
| discovery belongs to another topology | `DiscoveryProfile::validate` | `ValidationCode::IdentityMismatch` |
| Draft belongs to another topology/discovery or has a changed candidate | `DraftPlan::validate` or preparation | identity mismatch, candidate rejection, or planning failure |
| realization does not describe the unchanged Draft | `RealizationProfile::validate` | `IdentityMismatch` or `ResourceMismatch` |
| artifact bytes, target, ABI, entry, or toolchain differ from identity | native artifact validation | native preparation failure; no artifact enters the catalog/session |
| cache schema/digest differs from current discovery | profile cache load | stale cache or a fresh probe; no stale profile is reused |
| serialized digest is truncated, wrong width, or zero where forbidden | probe/checkpoint/remote codec | codec or incompatible-resume failure |
| remote endpoint, manifest, bundle, Draft, Realization, or program differs | transport/remote handshake or manifest verification | protocol/manifest mismatch and poisoned session |
| current measured profile differs from saved native metadata | training native resume | incompatible resume; no near-match kernel is selected |
| two distinct planner assignments produce one candidate digest | planner assignment enumeration | `PlannerErrorKind::IdentityCollision` |

The failure policy is deliberately located at the first boundary that has the
evidence needed to diagnose the mismatch. `core/src/identity.rs` remains a
small, dependency-free value layer and does not add fallback identities,
automatic hashing, alternate algorithms, or recovery branches.
