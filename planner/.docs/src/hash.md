# Planner hashing

## Scope

`planner/src/hash.rs` contains the crate-private `StableDigest` writer. It is a
small, deterministic SHA-256 stream builder used by `planner/src/planner.rs`.
It does not define a public hash algorithm, serialize a Rust value, or perform
validation. The planner explicitly visits each field that belongs to an
identity and feeds the field's representation to this writer in a fixed order.

The result is a `recipe_core::Digest`, which is exactly 32 bytes. The core
identity wrappers `KernelTemplateId` (for stage-scoped kernel identities),
`CandidateIdentity`, and `DraftIdentity` each wrap a `Digest` or a value derived
from one. The wrappers are typed identities, not different hash algorithms.

There are two related implementations elsewhere in the workspace:

* `kernel/src/stage.rs` has `ContractDigest`. Its `bytes`, `digest`, `length`,
  `u8`, `u64`, and `finish` operations intentionally use the same byte stream as
  `StableDigest`. The kernel crate independently recomputes the stage-template
  identity and the deferred artifact-build contract digest during realization.
* `prepare/src/lib.rs` has `CanonicalHash` for realization and finalized-bundle
  identities. It uses little-endian binary lengths and sorts collections before
  hashing. It is a separate identity layer and is not byte-compatible with
  `StableDigest`.

The paired implementation that must remain wire-compatible with this file is
`ContractDigest`; `CanonicalHash` is only a downstream consumer of the
identities produced by the planner.

## `StableDigest` stream format

`StableDigest` owns one `sha2::Sha256` state. The methods append bytes to that
state; they never reset it and never return an intermediate digest.

| Method | Bytes appended | Actual purpose |
| --- | --- | --- |
| `new(domain)` | `bytes(domain.as_bytes())` into a new SHA-256 state | Domain-separates every identity family. |
| `bytes(value)` | ASCII decimal `value.len()`, ASCII `:`, then `value` | Length-frames arbitrary bytes and textual values. |
| `digest(value)` | The 32 bytes returned by `Digest::bytes()` | Appends an already-computed digest without another frame. |
| `label(value)` | `bytes(value.as_str().as_bytes())` | Frames the exact UTF-8 bytes of a validated `Label`. |
| `length(value)` | `bytes(value.to_string().as_bytes())` | Frames the ASCII decimal spelling of a collection length. |
| `bool(value)` | One byte, `0` for false or `1` for true | Encodes boolean state without text. |
| `u8(value)` | The one byte `value` | Encodes enum and option tags. |
| `u64(value)` | `value.to_le_bytes()` | Encodes identifiers, counts, offsets, sizes, and numeric work values. |
| `finish()` | SHA-256 finalization, copied into `[u8; 32]`, then `Digest::new` | Produces the typed 256-bit digest. |

The framing is exact. For example, the bytes for a payload of length three are
the ASCII sequence `3:`, followed by the three payload bytes. `length(12)` is
therefore the framed payload `2:12`. Numeric `u64` values are fixed-width
little-endian bytes. `bool` and `u8` are one byte each. A `digest` is always the
raw 32-byte value and has no length prefix. There are no field names, implicit
separators, host-endian integers, random salt, or process state in the stream.
Callers supply all sequence lengths, variant tags, and ordering. Reordering two
fields or changing a tag changes the digest.

`Label` validation happens in `recipe_core::Label::new`; `StableDigest::label`
does not trim, normalize, or otherwise transform it. A label's exact stored
UTF-8 string is hashed. The SHA-256 finalizer is infallible in this API, so the
hash writer itself has no `Result` boundary. The planner validates the values
and identity wrappers around the writer.

## Shared tags and helper encodings

The planner keeps semantic enum encodings next to the hash callers. They are
not supplied by `StableDigest` itself.

* `dtype_tag`: `DType::F32 = 0`, `DType::I32 = 1`.
* `alias_permission_tag`: `Forbidden = 0`, `MayAliasExact = 1`,
  `MustAliasExact = 2`.
* Run phases in draft tasks: `Init = 0`, `Loop = 1`, `Exit = 2`.
* Task kinds: `Calculation = 0`, `Transfer = 1`, `Metric = 2`.
* Metric purposes: `User = 0`, `FaultReadback = 1`.
* Artifact build access: `Read = 0`, `Write = 1`, `ReadWrite = 2`,
  `ReadWriteAtomic = 3`.
* Transfer endpoints: `External = 0`; `Device` is `1`, followed by device and
  value IDs.
* Transfer lane claims: `Link = 0`, followed by link and lane; `External = 1`,
  followed by device and lane.
* Optional values are encoded with `0` for `None` and `1` followed by the value
  for `Some`.

`hash_loop_iterations` appends the unbounded flag and either the finite count or
zero. `hash_iteration_domain` appends, in order, first iteration, unbounded
flag, exclusive end or zero, and nonzero stride. This representation keeps
finite and unbounded domains distinct even when their numeric zero fields are
the same.

`hash_static_access` appends a static buffer view's element offset, stride
count and each stride, then storage bytes. `hash_kernel_template` appends the
index-space dimension count and dimensions; ordered input IDs, types, and
access views; ordered output IDs, types, and access views; scalar-program input
IDs and types; constants; instructions; scalar-program outputs; and alias
rules. A constant uses tag `0` plus the `u32` f32 bit pattern widened to a
little-endian `u64`, or tag `1` plus the signed i32 little-endian bytes framed by
`bytes`. An instruction includes result ID, type, the UTF-8 bytes of
`format!("{:?}", instruction.opcode)` framed by `bytes`, operand count and
operand IDs. The template's own ID is written by its caller before this helper.

## Direct planner callers

All direct uses are in `planner/src/planner.rs` and are crate-private. The
planner validates the program, graph, topology, discovery profile, reservation
ledger, capacity ledger, and artifact catalog before these identity-producing
paths are allowed to return a candidate.

### Stage-template identity and the kernel pair

`stage_template_identity` uses domain `recipe-planner-stage-template-v1` and
appends, in order:

1. `program.digest.bytes()` through `bytes`, so the 32-byte program digest is
   length-framed;
2. `program.source_kernel.get()` as a little-endian `u64`;
3. `stage.id.get()` as a little-endian `u64`.

The final SHA-256 is reduced to a `KernelTemplateId` by taking the first eight
digest bytes and interpreting them as a little-endian `u64`. The planner maps a
digest-width conversion failure to `PlannerErrorKind::InvalidDraft` and rejects
the reserved zero ID as `PlannerErrorKind::IdentityCollision`.

`lower_programs` calls this once for every lowered primitive stage. It stores a
map from the resulting stage identity to `(source_kernel, stage_ordinal)` and
rejects one identity produced by two different pairs. Scalar-map stages copy
the identity into the `KernelTemplate.id`; the same identity is also used for
the deferred artifact ID. Thus the stage identity names a specific stage of a
specific lowered program, not merely the source kernel.

`kernel/src/stage.rs::stage_template_identity` repeats the same domain and the
same three fields with `ContractDigest`, copies the first eight bytes into a
fixed array, and rejects zero through `InvalidStageContract`. During
`lower_stage`, the kernel checks that the build's `kernel_template` equals this
independently recomputed identity and that `build.artifact` has the same ID.
Changing either implementation's framing, field order, or little-endian
conversion causes realization to reject an otherwise matching planner output.

### Graph identity

`graph_digest` starts domain `recipe-planner-graph-v7` and returns a plain
`Digest`. Its stream is:

1. Loop iteration encoding from `hash_loop_iterations`.
2. Tensor count. The tensors are visited through a `BTreeMap<ValueId, &Tensor>`,
   therefore in ascending `ValueId` order. Each tensor contributes ID, dtype
   tag, storage bytes, external-input flag, external-output flag, layout element
   offset, shape extent count and extents, then stride count and strides.
3. Topological kernel-order count. For each kernel in the supplied order:
   * kernel ID;
   * the source iteration domain, or `InvalidGraph` if it is absent;
   * input count and ordered input value IDs;
   * output count and ordered output value IDs;
   * `CalculationNode::kernel.work(tensors)`, mapped to `InvalidGraph` on an
     error;
   * the lowered program digest through `bytes`;
   * stage-template count and ordered stage-template IDs. A missing node or
     lowered program is `InvalidGraph`.
4. Metric-emission count. Each emission contributes metric ID, value ID, and its
   iteration domain.

`plan_program_candidates` obtains the topological order, tensor index, lowered
program map, source domains, and metric list before calling this function. The
graph digest does not include topology or discovery identities, placement
choices, artifact catalog entries, or scheduled task details. Those belong to
the candidate or draft layers below.

### Candidate identity

`candidate_identity` starts domain `recipe-planner-candidate-v3` and appends:

1. the graph digest as raw digest bytes;
2. `topology.identity.digest()` as raw digest bytes;
3. `discovery.identity.digest()` as raw digest bytes;
4. assignment length;
5. for each pair in topological `order` and the corresponding placement option,
   kernel ID followed by selected device ID.

The function wraps the final digest in `CandidateIdentity`. Legal choices are
formed once per ordered kernel from measured GPU calculation devices and are
sorted by device ID. Assignment enumeration visits that product recursively.
`plan_program_candidates` inserts each produced identity into a `BTreeSet`; two
distinct assignment vectors producing one identity return
`PlannerErrorKind::IdentityCollision`. Candidates are ranked by measured
makespan and then the candidate identity. `PlannerSearch` and
`ProgramPlannerSearch` use the same typed identity in their issued and rejected
sets: only an issued candidate can be rejected, and a candidate cannot be
rejected twice.

Candidate identity deliberately stops at graph, measured-system identities,
and placement. It does not include artifact contents or the eventual task
schedule. The full selected Draft is hashed separately.

### Deferred artifact-build contract

`hash_build_contract` starts domain `recipe-planner-artifact-build-v1`. It
appends the exact target-independent build recipe in this order:

1. artifact ID, stage-scoped kernel-template ID, and source-kernel ID;
2. provenance program digest, then stage ordinal;
3. binding count; for each ordered binding, value ID, dtype tag, access tag,
   logical-extent count and extents, element offset, stride count and strides,
   and storage bytes;
4. dispatch logical lanes, workgroup lanes, and workgroup count;
5. work bounds: flops, integer operations, and atomic operations;
6. optional fault flag tag and value ID;
7. resource bounds: private bytes per lane, shared bytes per workgroup,
   scratch bytes per dispatch, and maximum workgroup lanes.

The planner constructs each `ArtifactBuildRecipe` with a zero contract digest,
computes this digest, writes it into `provenance.contract_digest`, and then
validates the recipe. The contract digest is intentionally not included in its
own input. `hash_artifact_build` later includes the stored contract digest when
the complete Draft identity is computed.

`kernel/src/stage.rs::artifact_build_contract_digest` is an independent copy of
the same stream. `validate_contract` compares the recomputed value to
`build.provenance.contract_digest` before lowering. It also checks program
digest, source kernel, stage ordinal, canonical stage identity, dispatch
geometry, work bounds, resource bounds, ordered bindings, and fault binding.
The result is a fail-closed realization boundary for mutated or stale build
recipes.

### Complete Draft identity

`hash_draft` starts domain `recipe-planner-draft-v10` and receives one
`DraftHashInput` containing candidate identity, topology and discovery profiles,
values, kernels, scheduled tasks, selected artifacts, deferred build recipes,
resource manifest, arena objects, value bindings, alias contracts, init images,
arena releases, loop iteration count, and per-task loop domains.

The topology and discovery structures are represented by their existing typed
identity digests, not by rehashing their full vectors. The remaining stream is:

1. candidate digest, topology identity digest, discovery identity digest;
2. loop-iteration encoding, loop-domain count, then each task ID and domain;
3. value count. Each value contributes ID, dtype, bytes, device, and optional
   producer tag plus producer task ID;
4. kernel count. Each kernel contributes its ID followed by
   `hash_kernel_template`;
5. task count. Each task contributes ID, phase tag, window start and end,
   dependency count and dependencies in the stored order, then one task-kind
   branch:
   * calculation tag, device, kernel-template ID, artifact ID, work, ordered
     input IDs, ordered output IDs, optional fault flag, queue ID, and
     completion ID;
   * transfer tag, byte count, source endpoint, destination endpoint, route
     count and ordered link IDs, lane-claim count and ordered tagged claims,
     queue ID, and completion ID;
   * metric tag, purpose tag, metric ID, value ID, metric-slot ID, queue ID, and
     completion ID;
6. artifact count and each artifact through `hash_artifact`;
7. artifact-build count and each recipe through `hash_artifact_build`;
8. queue count and each queue ID/device, completion count and each completion
   ID/device, metric count and each metric-slot ID/metric ID;
9. pinned-staging and scratch vectors through `hash_device_bytes`, each with a
   count followed by device ID and bytes;
10. arena-object count and each object ID, device, bytes, alignment, lifetime
    start, and lifetime end;
11. value-binding count and each value ID, arena-object ID, and object offset;
12. value-alias count and each input ID, output ID, and permission tag;
13. init-image count. Each image contributes device, image value ID, total
    bytes, member count, then every member's logical ID, physical ID, dtype,
    bytes, and image offset;
14. arena-release count and each release device ID.

`hash_artifact` includes artifact ID, image digest, format label, target backend,
architecture and ABI labels, toolchain name/version labels and digest, entry
symbol label, kernel-template ID, all kernel resource bounds, and an optional
build-provenance tag with program digest, stage ordinal, and contract digest.

`hash_artifact_build` uses the same build fields as `hash_build_contract`,
including the stored contract digest in the provenance section. `hash_endpoint`
is the endpoint tag helper described above; `hash_device_bytes` supplies the
length and device/byte pairs for a resource vector.

There is no sorting inside `hash_draft`. The caller establishes the relevant
canonical order immediately before the call: chosen artifacts and build recipes
are sorted by ID, values by value ID, releases by device ID, and kernels come
from a `BTreeMap`'s value order. Scheduled tasks and resource vectors are hashed
in the order supplied by the scheduler and lowering state. Alias contracts are
sorted by input/output ID by `value_alias_contracts`; loop domains are extracted
from loop tasks in scheduled-task order. This distinction is material: changing
the order of a vector that the caller does not sort changes the Draft identity.

The resulting digest is wrapped in `DraftIdentity` and placed in
`DraftPlan.identity`, while the same candidate identity is placed in
`DraftPlan.candidate`. `DraftPlan` itself is offset-free and contains values,
kernels, artifacts, build recipes, tasks, resources, arena objects, bindings,
aliases, init images, and releases. The hash includes the loop schedule sidecar
even though loop iterations and loop domains are carried by
`PlannedProgramCandidate` and later `FinalizedBundle`, not by the `DraftPlan`
struct. It does not include makespan, placement sidecars, lowered-program
copies, logical-copy/external-output sidecars, or finalized arena offsets.
Those are ranking, presentation, or later realization data rather than the
offset-free Draft contract.

## State identity and consumers

The complete path is deterministic and one-way:

1. `Preparer::prepare_program` validates the measured profile and asks
   `plan_program_candidates` for a finite ranked search.
2. Planning lowers each graph stage, derives paired stage identities and build
   contracts, computes one graph digest, computes one candidate identity per
   placement assignment, lowers and schedules the assignment, and computes its
   Draft identity. `DraftPlan::validate` then checks nonzero identities,
   topology/discovery ownership, resource references, tasks, and the complete
   offset-free contract.
3. The prepare loop takes the next candidate, passes its Draft and candidate
   identity to the realizer, and records candidate-specific rejection stages.
   Realization must preserve the Draft, candidate, topology, and discovery
   identities. A successful observation receives a downstream `RealizationIdentity`
   from `CanonicalHash`, not from `StableDigest`.
4. Prepare combines Draft, realization, measured resources, reservations,
   capacity, loop domains, and finalized arena layouts into a
   `BundleIdentity`, again with `CanonicalHash`, then calls
   `FinalizedBundle::finalize_with_loop_schedule`. Finalization stores and
   exposes the Draft, candidate, realization, topology, and discovery identities
   as immutable associations.
5. Native local, CUDA, and HSA paths validate that the finalized bundle's Draft
   identity and candidate identity equal the prepared candidate and that the
   task, kernel, artifact-build, resource, reservation, and init-image payloads
   are unchanged. The stage backend independently verifies the paired stage and
   build hashes before producing native code.
6. Remote manifests carry the finalized bundle digest, Draft digest, realization
   digest, and artifact digests. Remote codec validation rejects zero or
   mismatched plan identities. Worker projections and executor sessions key
   their state to the finalized bundle identity; the planner hash is therefore
   part of the end-to-end state identity even when a later protocol adds its own
   digest.

The hashes are identifiers and integrity checks, not authorization or proof of
semantic correctness by themselves. Validation remains at the graph, topology,
Draft, realization, finalization, and backend boundaries. A digest collision is
treated as an explicit planner error where the planner can compare source
identity pairs or candidate assignments. A zero identity is rejected by stage
identity construction or by the relevant core validation. A stale deferred
artifact contract is rejected by the independent kernel recomputation. No
fallback identity, retry hash, alternate encoding, or substitute state exists.

## Failure boundaries and stability rules

* `StableDigest` methods have no fallible operations. `finish` always returns a
  32-byte `Digest` after SHA-256 finalization.
* `graph_digest` can return `InvalidGraph` for missing nodes, missing iteration
  domains, failed work calculation, or missing lowered programs. These failures
  happen before a candidate is emitted.
* Stage-template reduction can return `InvalidDraft` for an unexpected digest
  width or `IdentityCollision` for a reserved zero ID. `lower_programs` also
  rejects two distinct source-stage pairs or duplicate scalar templates sharing
  one stage identity.
* Candidate enumeration reports `IdentityCollision` when two distinct placement
  assignments produce one `CandidateIdentity`; it reports `NoViableCandidate`
  only after all lowered assignments fail their allowed route, dependency,
  capacity, or feasibility boundaries.
* Build hashing itself cannot fail. Recipe validation and the kernel's
  independent contract check are the failure boundaries for malformed or
  modified build data.
* Draft hashing itself cannot fail. Errors while constructing the ordered input,
  scheduling, packing, resource manifest, loop-domain sidecar, or artifact
  catalog prevent `hash_draft` from being called or prevent its result from
  being accepted by `DraftPlan::validate`.

The version suffix in each domain string is the visible version boundary for
that identity family: graph `v7`, candidate `v3`, stage-template `v1`,
artifact-build contract `v1`, and Draft `v10`. Changing a field, tag, framing,
order, or domain without coordinating every paired reader or verifier changes
the identity and therefore changes the accepted planner state.
