# `recipe_core::artifact`

`core/src/artifact.rs` defines the artifact contract shared by Recipe's
discovery, Draft, Realize, Finalize, and native-execution layers. It is a
dependency-free part of `recipe-core`. The module contains typed metadata and
validation only. It does not compile kernels, load a driver image, allocate
memory, open a file, or encode/decode a document.

The public module is exported by `core/src/lib.rs` as both `recipe_core::artifact`
and a crate-root re-export. The only methods implemented in this module are
`ArtifactBuildAccess::reads`, `ArtifactBuildAccess::writes`,
`ArtifactBuildRecipe::validate`, and `ArtifactIdentity::validate`. All other
values are constructed with struct literals in the surrounding pipeline.

At the module boundary the callable inputs and outputs are deliberately small:

| Operation | Input | Output | Failure behavior |
| --- | --- | --- | --- |
| `ArtifactBuildAccess::reads` | One access enum value | `bool` | Cannot fail; it is a total `const` projection. |
| `ArtifactBuildAccess::writes` | One access enum value | `bool` | Cannot fail; it is a total `const` projection. |
| `ArtifactBuildRecipe::validate` | One complete target-independent recipe | `Ok(())` or all direct recipe validation errors | Returns `ValidationErrors`; it does not mutate or repair the recipe. |
| `ArtifactIdentity::validate` | One complete image identity | `Ok(())` or all identity-local validation errors | Returns `ValidationErrors`; it does not inspect runtime bytes or mutate the identity. |

The surrounding layers construct the structs, validate them at their boundary,
and carry the same values forward. There is no hidden default constructor,
normalization pass, or fallback artifact in `recipe-core`.

## Where an artifact exists in the pipeline

| Phase | Input | Artifact value | Output and ownership |
| --- | --- | --- | --- |
| Discovery | Measured GPU capability | `TargetIdentity` inside `CalculationCapability` | The target is the exact backend, architecture, and ABI that a device reports. |
| Draft | Lowered primitive stages, placement, and discovery | `ArtifactBuildRecipe` or an already matching `ArtifactIdentity` | `DraftPlan` stores `artifact_builds` and `artifacts` as disjoint alternatives. |
| Realize | One immutable `PlannedCandidate`, a target build specification, and a catalog | `ArtifactIdentity` plus a native executor `RuntimeArtifact` | Deferred recipes are lowered and compiled, or a validated prebuilt image is selected. |
| Finalize | Validated `DraftPlan`, `RealizationProfile`, and arena layouts | `FinalizedBundle::artifacts` and `FinalizedBundle::artifact_builds` | The bundle is immutable and exposes only validated metadata to the run lifecycle. |
| Run | A finalized bundle and runtime images | `RuntimeArtifact` and per-device loaded entries | CUDA and HSA execution plans bind each calculation task to exactly one image and ABI. |
| Public save/load | A completed dense training execution | Training checkpoint native metadata and exact native bytes | The training layer writes semantic `.ogdl` and native `.cubin` or `.hsaco` files. It does not serialize this module's Rust values directly. |

An artifact ID is not a file name. `ArtifactId` and `KernelTemplateId` are
stable numeric identities used inside a plan. A native file is an image that
must match the identity and ABI selected by the plan. Conversely, a semantic
`.ogdl` model is a training checkpoint and is not a serialized
`FinalizedBundle`.

## Supporting types

The fields below use types defined by the other `recipe-core` modules.

### `TargetIdentity`

```rust
pub struct TargetIdentity {
    pub backend: Label,
    pub architecture: Label,
    pub abi: Label,
}
```

This is the opaque calculation target selected from measured capability. It is
not a product-name switch. The native probes currently construct these values
as follows:

| Backend | `backend` | `architecture` | `abi` |
| --- | --- | --- | --- |
| NVIDIA | `nvidia-cuda-driver` | The validated CUDA compute capability string | `elf64-cubin` |
| AMD | `amd-rocr-hsa` | The validated AMD target ID | `elf64-amdgpu-code-object-v<version>` |

`probe::CalculationCapability.target`, the planner's target check, the native
compiler's `TargetBuildSpec`, and the CUDA/HSA runtime checks all compare this
complete value. `TargetIdentity` has no own `validate` method. Each field is a
`Label`, so the only public label constructor, `Label::new`, rejects empty and
whitespace-only strings. The probe and checkpoint decoders use that constructor
and report a field-specific error on failure.

The derives (`Ord` and `Hash` in addition to equality and debugging) make a
target safe to use as a deterministic map key and as part of candidate and
realization identity calculations.

### `ToolchainIdentity`

```rust
pub struct ToolchainIdentity {
    pub name: Label,
    pub version: Label,
    pub digest: Digest,
}
```

This identifies the exact toolchain that produced a native image. The digest is
caller-computed and is not interpreted by `recipe-core`. Native probing hashes
the backend, release, target configuration, probe source digest, and every
pinned tool into a `ToolchainIdentity`. Native preparation rejects a zero
toolchain digest before a target build specification can be used. Artifact
identity validation also rejects a zero toolchain digest.

### `KernelResourceBounds`

```rust
pub struct KernelResourceBounds {
    pub private_bytes_per_lane: ByteCount,
    pub shared_bytes_per_workgroup: ByteCount,
    pub scratch_bytes_per_dispatch: ByteCount,
    pub maximum_workgroup_lanes: u32,
}
```

This is the conservative resource envelope retained from Draft. The first
three fields are exact byte counts. `maximum_workgroup_lanes` is the workgroup
size that the stage was drafted with, not an unbounded capability hint. A
deferred recipe must retain the exact drafted width. A realized runtime image
may use no more than this width, and HSA derives its finalized resource
envelope from these fields.

### `ArtifactBuildProvenance`

```rust
pub struct ArtifactBuildProvenance {
    pub program_digest: Digest,
    pub stage_ordinal: u32,
    pub contract_digest: Digest,
}
```

The program digest identifies the complete backend-neutral lowered program.
`stage_ordinal` identifies one stage in that program. `contract_digest` hashes
the complete deferred build contract. Together these fields explain which
immutable stage recipe a realized image implements without claiming that a
target or toolchain existed during Draft. All three fields are carried into
the realized identity when a deferred recipe is compiled.

### `ArtifactBuildAccess`

```rust
pub enum ArtifactBuildAccess {
    Read,
    Write,
    ReadWrite,
    ReadWriteAtomic,
}
```

`reads()` is true for `Read`, `ReadWrite`, and `ReadWriteAtomic`. `writes()` is
true for `Write`, `ReadWrite`, and `ReadWriteAtomic`. The methods are `const`
and marked `must_use`.

The planner maps `recipe_primitives::AccessMode` one-to-one to this enum. The
same mapping is independently repeated by the kernel lowerer. Every consumer
uses the methods instead of reinterpreting the enum:

* planner calculation inputs are the ordered bindings for which `reads()` is
  true, excluding the optional fault binding;
* planner calculation outputs are the ordered bindings for which `writes()` is
  true, again excluding the fault binding;
* kernel lowering creates read and write pointer slots from those predicates;
* native ABI validation reconstructs the expected operand list from those
  predicates.

### `ArtifactBuildView`

```rust
pub struct ArtifactBuildView {
    pub logical_extents: Vec<u64>,
    pub offset_elements: u64,
    pub strides: Vec<u64>,
    pub storage_bytes: ByteCount,
}
```

This is the complete affine view of one stage argument. Extents and strides are
kept in rank order, `offset_elements` is the base element offset, and
`storage_bytes` is the exact backing allocation size. The view is copied from
the primitive stage, hashed into the build contract, compared again by kernel
lowering, and used to derive the runtime ABI operand storage sizes.

`ArtifactBuildRecipe::validate` requires an explicit nonempty extent vector and
requires the extent and stride vectors to have equal length. It intentionally
does not infer rank, reject zero extents, check address arithmetic, or check
that the value exists in a graph. Those checks belong to primitive lowering,
Draft value indexes, and the finalized arena validator.

### `ArtifactBuildBinding`

```rust
pub struct ArtifactBuildBinding {
    pub value: ValueId,
    pub dtype: DType,
    pub access: ArtifactBuildAccess,
    pub view: ArtifactBuildView,
}
```

Bindings are ordered stage arguments. `value` names the candidate-local
resident value, `dtype` is the calculation payload type (`F32` or `I32`), and
`access` plus `view` define the complete pointer contract. `DType` currently
has a four-byte width for both variants. The recipe validator requires all
binding values to be distinct. Draft validation additionally requires every
binding value to be present, resident on the calculation device, and equal in
dtype and storage size to the corresponding `ValueSpec`.

### `ArtifactDispatchGeometry`

```rust
pub struct ArtifactDispatchGeometry {
    pub logical_lanes: u64,
    pub workgroup_lanes: u32,
    pub workgroups: u64,
}
```

This is the fixed launch geometry selected before target realization. A valid
recipe has nonzero logical and workgroup lane counts. The validator computes
the exact ceiling division

```text
ceil(logical_lanes / workgroup_lanes)
```

with checked arithmetic and requires that result to equal `workgroups`. An
addition overflow produces a `ResourceMismatch` rather than a wrapped launch
count. The resource envelope's `maximum_workgroup_lanes` must equal the
geometry's `workgroup_lanes`.

### `ArtifactWorkBounds`

```rust
pub struct ArtifactWorkBounds {
    pub flops: FlopCount,
    pub integer_operations: u64,
    pub atomic_operations: u64,
}
```

These are exact operation bounds retained even though the current measured
scheduler prices floating-point work. The planner copies all three values from
the lowered stage. The kernel lowerer verifies all three against the primitive
stage, while the calculation task currently carries the `flops` component as
its scheduler work value.

## `ArtifactBuildRecipe`

`ArtifactBuildRecipe` is the target-independent request consumed by Realize.
It deliberately contains no `TargetIdentity`, format, entry symbol, or
`ToolchainIdentity`.

The recipe derives cloning, debugging, and equality, but not an implicit
serializer or content hash. Planner and preparation hash implementations walk
its fields explicitly so field order and domain separation remain visible.

```rust
pub struct ArtifactBuildRecipe {
    pub artifact: ArtifactId,
    pub kernel_template: KernelTemplateId,
    pub source_kernel: KernelTemplateId,
    pub provenance: ArtifactBuildProvenance,
    pub bindings: Vec<ArtifactBuildBinding>,
    pub dispatch: ArtifactDispatchGeometry,
    pub work: ArtifactWorkBounds,
    pub fault_flag: Option<ValueId>,
    pub resources: KernelResourceBounds,
}
```

`artifact` reserves the ID that the eventual `ArtifactIdentity` must carry.
`kernel_template` is a collision-checked stage-scoped identity. It is not the
source primitive kernel ID, which is retained separately as `source_kernel`.
`fault_flag`, when present, names the candidate-local value used to publish a
checked-stage failure. The resource envelope and all ordered bindings are part
of the immutable build contract.

### Recipe validation

`ArtifactBuildRecipe::validate` uses `Validator`, which accumulates every
failure in one pass and returns `ValidationResult<()>`, that is,
`Result<(), ValidationErrors>`. Each error includes a machine-readable
`ValidationCode`, a field path, and a message. The direct method checks exactly
the following conditions:

| Path | Required condition | Failure code |
| --- | --- | --- |
| `artifact` | `ArtifactId::get()` is not zero | `InvalidIdentity` |
| `kernel_template` | `KernelTemplateId::get()` is not zero | `InvalidIdentity` |
| `source_kernel` | `KernelTemplateId::get()` is not zero | `InvalidIdentity` |
| `provenance.program_digest` | Digest is not zero | `InvalidIdentity` |
| `provenance.contract_digest` | Digest is not zero | `InvalidIdentity` |
| `dispatch.logical_lanes` | Nonzero | `MissingRequiredObject` |
| `dispatch.workgroup_lanes` | Nonzero | `MissingRequiredObject` |
| `dispatch.workgroups` | Exact checked ceiling division | `ResourceMismatch` |
| `resources.maximum_workgroup_lanes` | Equal to `dispatch.workgroup_lanes` | `ResourceMismatch` |
| `bindings[i].value` | No value appears twice | `DuplicateId` |
| `bindings[i].view.logical_extents` | Explicit nonempty rank | `MissingRequiredObject` |
| `bindings[i].view.strides` | Same rank as logical extents | `ResourceMismatch` |
| `fault_flag` | If present, names a binding with the same value, `DType::I32`, `ReadWriteAtomic`, and exactly `ByteCount::new(4)` storage | `ResourceMismatch` |

No target, toolchain, entry symbol, primitive-stage, graph-reference, or
runtime-image check is performed here. A `ValueId::new(0)` is not rejected by
this method if it is otherwise unique. The surrounding Draft, kernel, and
native checks close those larger contracts.

### How the planner constructs a recipe

`planner::plan_program_candidates` validates the program graph, topology,
measured discovery, reservations, capacity, and artifact catalog before any
candidate is lowered. `lower_programs` lowers every primitive kernel and derives
one stage-scoped `KernelTemplateId` per nonempty stage. The identity is the
first eight bytes of a SHA-256 digest in the
`recipe-planner-stage-template-v1` domain over the lowered program digest, the
source kernel ID, and the stage ordinal. A collision between distinct source
stages is a planner `IdentityCollision` error. The corresponding scalar
template is copied with that stage identity.

For each selected placement, `lower_program_invocation` performs this sequence:

1. It allocates one task and submission slot per lowered stage.
2. It materializes each program buffer into a candidate-local `ValueId` and
   records producer and barrier dependencies.
3. It maps each primitive `BufferBinding` to an ordered
   `ArtifactBuildBinding`. A missing program buffer is an `InvalidDraft`
   planner error.
4. It derives calculation `inputs` and `outputs` from `reads()` and
   `writes()`, excluding the fault value from both lists.
5. It reserves `ArtifactId::new(stage_template.get())`, copies the lowered
   program digest and stage ordinal, copies exact geometry and operation bounds,
   and copies private/shared resource bounds. Draft currently sets
   `scratch_bytes_per_dispatch` to `ByteCount::ZERO` at this boundary.
6. It computes `provenance.contract_digest` with the canonical
   `recipe-planner-artifact-build-v1` digest, validates the recipe, and either
   selects an exact catalog identity or appends the recipe to
   `DraftPlan::artifact_builds`.
7. It emits a loop `TaskKind::Calculation` carrying the stage template, the
   reserved artifact ID, ordered inputs and outputs, optional fault value, and
   the stage FLOP count.

The planner's `select_or_defer_artifact` searches the catalog by reserved
artifact ID. If an identity is found, all of the following must match:

* its `kernel_template` equals the recipe stage identity;
* its `build` equals the recipe provenance;
* its `resources` equals the recipe resources; and
* its target equals the measured calculation target of the selected device.

Any mismatch is `PlannerErrorKind::InvalidArtifact`. If no catalog entry is
found, the validated recipe is retained for Realize. Thus a generated stage
never silently accepts an image for a neighboring stage or target.

The planner computes scratch and staging peaks after scheduling. A calculation
task must resolve its artifact ID to exactly one catalog identity or deferred
recipe. A transfer that enters or leaves a device contributes staging usage;
the selected artifact or recipe contributes scratch usage. Missing, duplicate,
or overlapping resolution is an `InvalidArtifact` error. The resulting peaks
are included in capacity feasibility and the final Draft resources.

### Contract hashing

The planner's `hash_build_contract` uses SHA-256 domain
`recipe-planner-artifact-build-v1`. It hashes, in order, the artifact ID,
stage and source kernel IDs, program digest, stage ordinal, binding count and
each binding's value, dtype, access tag, extents, offset, strides, and storage
bytes, then dispatch geometry, all three work bounds, optional fault flag, and
all four resource bounds. `ArtifactBuildProvenance::contract_digest` is the
result.

`hash_artifact_build` includes the same recipe data plus the already computed
contract digest when hashing a full Draft. The Draft identity therefore changes
when any recipe field changes. Lengths are domain-separated and encoded by
`StableDigest`; integer values are little-endian, and labels use length-prefixed
bytes.

## `ArtifactIdentity`

`ArtifactIdentity` is the complete identity and resource contract for one
realized or preexisting native image.

It likewise derives cloning, debugging, and equality only. Stable artifact
hashes are explicit planner and preparation operations, not a Rust trait
implementation that could silently omit a field.

```rust
pub struct ArtifactIdentity {
    pub id: ArtifactId,
    pub digest: Digest,
    pub format: Label,
    pub target: TargetIdentity,
    pub toolchain: ToolchainIdentity,
    pub entry_symbol: Label,
    pub kernel_template: KernelTemplateId,
    pub resources: KernelResourceBounds,
    pub build: Option<ArtifactBuildProvenance>,
}
```

Field meaning:

* `id` is the stable artifact key used by calculation tasks and runtime maps.
* `digest` is the SHA-256 digest of the native image bytes. The core type does
  not calculate it.
* `format` is the image format label. Native preparation requires it to equal
  `target.abi`.
* `target` and `toolchain` are the measured target and exact producer identity.
* `entry_symbol` is the function name resolved by the runtime loader.
* `kernel_template` binds the image to one stage-scoped kernel identity.
* `resources` is the finalized envelope that native allocation and HSA resource
  setup must honor.
* `build` is `Some` when the image realizes a deferred Draft recipe. It is
  `None` only for an identity that did not come from a deferred recipe. A
  catalog identity offered for a generated deferred stage must still carry the
  exact matching provenance, otherwise planner selection rejects it.

### Identity validation

`ArtifactIdentity::validate` checks only the identity-local contract:

| Path | Required condition | Failure code |
| --- | --- | --- |
| `digest` | Image digest is not zero | `InvalidIdentity` |
| `toolchain.digest` | Toolchain digest is not zero | `InvalidIdentity` |
| `resources.maximum_workgroup_lanes` | Nonzero | `MissingRequiredObject` |
| `build.program_digest` | If `build` is present, nonzero | `InvalidIdentity` |
| `build.contract_digest` | If `build` is present, nonzero | `InvalidIdentity` |

It intentionally does not require a nonzero `id` or `kernel_template`, compare
`format` with `target.abi`, compare the entry symbol with a runtime ABI, or
compare the resource envelope with a primitive stage. Those are enforced at
catalog, Draft, Realize, kernel, and native-executor boundaries. The planner
requires nonzero unique artifact IDs, and the Draft validator requires exact
artifact/build resolution.

## Draft and Finalize ownership

`DraftPlan` stores two disjoint vectors:

* `artifacts: Vec<ArtifactIdentity>` for identities already available to Draft;
* `artifact_builds: Vec<ArtifactBuildRecipe>` for target-independent stages that
  Realize must build.

`DraftPlan::validate` indexes both vectors and accumulates these artifact
invariants:

* artifact IDs are unique, and each identity passes
  `ArtifactIdentity::validate`;
* deferred artifact IDs are unique and cannot also occur in `artifacts`;
* deferred stage-scoped kernel identities are unique;
* every deferred build passes `ArtifactBuildRecipe::validate`, and every binding
  value is known to the Draft value index;
* each calculation task resolves its artifact ID to exactly one realized
  identity or deferred build;
* a realized identity's kernel template equals the calculation task's stage and
  its target equals the measured target of the task's device;
* a deferred build's stage identity and FLOP work equal the calculation task;
* calculation inputs and outputs exactly equal the ordered binding projections
  described above;
* every binding value is resident on the calculation device with matching dtype
  and storage bytes;
* a fault value, when present, is a resident four-byte `I32` and its presence
  agrees with the stage contract.

`RealizationProfile::validate` then requires an exact identity set: every
prebuilt Draft identity is unchanged, every deferred recipe has exactly one
realized identity, no extra identity is present, each deferred identity passes
its own validation, and its kernel template, provenance, resource envelope,
and measured target all match the unchanged Draft. Realization resources must
also equal Draft resources.

`FinalizedBundle::finalize_with_loop_schedule` validates the Draft and
Realization together, validates loop domains and arena layouts, resolves value
locations, and stores `realization.artifacts` plus `draft.artifact_builds` in
private immutable fields. Its public accessors are `artifacts()`,
`artifact_builds()`, and `artifact_build(ArtifactId)`. The bundle has no
serializer and no artifact mutation API.

## Realize and native image construction

The production implementation is in `prepare/src/production.rs`.

### Catalog input

`NativeArtifact` pairs one `ArtifactIdentity` with one native-executor
`RuntimeArtifact`. Its constructor validates both domains before the pair can
enter `NativeArtifactCatalog`. The catalog sorts by artifact ID, rejects
duplicate IDs, validates every pair, and exposes only identities to the planner
and complete pairs to the candidate realizer. `NativeArtifactProvider::resolve`
also requires every catalog target to exist in the measured discovery profile.

### Deferred compilation

`DeferredArtifactCompiler::materialize` first copies each prebuilt identity only
when the catalog contains an exact matching runtime image. It then groups every
deferred recipe by the one measured target selected by its candidate's
calculation tasks. A recipe unused by a candidate, or assigned to more than one
target, is an `InvalidCandidate` failure. A target without a build specification
is `MissingBuildTarget`.

For each target group, the compiler:

1. locates the candidate's exact lowered program by `source_kernel` and
   `program_digest`;
2. calls `recipe_kernel::lower_stage` with an entry symbol
   `recipe_stage_<artifact id>` and the recipe workgroup width;
3. either inspects a supplied prebuilt bundle or invokes the pinned builder in
   `BuildPhase::Realize` for one multi-entry cubin or HSACO;
4. requires one inspected entry for each lowered stage and checks every entry
   against the lowered ABI; and
5. calls `native_artifact_from_image` to construct the core identity and the
   runtime image.

The constructed identity uses the image-byte digest, target ABI as `format`,
the exact target and toolchain identities from `TargetBuildSpec`, the lowered
entry symbol, the recipe's stage identity and resources, and
`build: Some(build.provenance)`. `RuntimeArtifact::from_image` retains the
same bytes, digest, `KernelAbi`, and backend kind. All artifacts are sorted and
duplicate IDs are rejected.

The core `Digest` and kernel `ArtifactDigest` are distinct wrappers around the
same 32-byte SHA-256 value. Realize converts the inspected image digest by
copying its bytes into `recipe_core::Digest`; native preparation and native
execution then compare the two byte arrays explicitly. A type conversion never
weakens the equality check.

`validate_native_artifact` is the first cross-domain gate. It requires the core
identity to validate, runtime and identity IDs to match, runtime and identity
digests to match, runtime and identity entry symbols to match, and
`format == target.abi`. CUDA runtime metadata must identify
`nvidia-cuda-driver`, `elf64-cubin`, the same compute capability, and the same
image digest. HSA metadata must identify `amd-rocr-hsa`, the expected code
object ABI, and the same target ID.

The candidate preparation loop keeps this image set alongside the warmed native
session. A candidate rejection destroys that session and tries the next ranked
candidate. On success, `RealizationObservation.artifacts` is hashed into the
realization identity and passed unchanged to Finalize.

## Kernel lowering contract

`kernel::lower_stage` is the independent verifier and target lowerer for a
`ArtifactBuildRecipe`.

Before emitting LLVM IR it requires:

* the complete lowered program and recipe to pass their canonical validators;
* the target and lowering options to be valid;
* the recipe program digest and source kernel to equal the lowered program;
* the stage ordinal to exist in the lowered program;
* the recipe stage identity to equal the canonical stage identity;
* the reserved artifact ID to equal the stage identity;
* `contract_digest` to equal an independently recomputed
  `recipe-planner-artifact-build-v1` digest;
* dispatch geometry and lowering workgroup width to equal the primitive stage;
* all FLOP, integer, and atomic work bounds to equal the primitive stage;
* all resource bounds to equal the primitive stage's expected envelope; and
* binding count, dtype, access, extents, offset, strides, and storage bytes to
  equal the ordered primitive bindings.

The lowerer validates the optional fault binding against the stage's ordered
fault buffer. Its `StageSignature` uses `reads()` and `writes()` to emit the
ordered input and output pointer ABI, emits one fault pointer when required,
and adds dynamic run and loop arguments only for stage kinds that need them.
The realized ABI is checked for logical element count, workgroup width, FLOP
work, and fault argument count. The builder then verifies every LLVM module,
ensures unique entry symbols in the target bundle, invokes only the pinned
offline tools, and inspects every resulting cubin or HSACO entry.

The planner and kernel verifier intentionally duplicate the contract digest
implementation. A stale or modified recipe therefore fails at Realize instead
of allowing a mutated binding, geometry, fault storage, or resource envelope to
reach a compiler.

The kernel artifact inspectors are the byte-level gate below the core identity.
`inspect_cubin` parses a little-endian ELF64 CUDA image, accepts only the
explicitly supported CUDA OS-ABI tags and CUDA machine, decodes the SM value,
requires the requested global function and executable `.text.<entry>` section,
rejects unresolved global symbols, and returns a SHA-256 `ArtifactDigest`.
`inspect_hsaco_bundle` performs the corresponding AMDGPU-HSA ELF checks,
validates code-object version and target ID, decodes AMDGPU MessagePack kernel
metadata, requires the requested function and descriptor symbols, and checks
the complete HSA argument ABI. Malformed ranges, missing metadata, duplicate
kernels, target mismatch, and ABI mismatch are lowering failures before
`ArtifactIdentity` construction.

## Native executor consumption

`native-executor` keeps the core identity and the runtime bytes in separate
types:

```rust
pub struct RuntimeArtifact {
    id: ArtifactId,
    bytes: Arc<[u8]>,
    digest: ArtifactDigest,
    abi: KernelAbi,
    kind: RuntimeArtifactKind,
}
```

`ExecutionPlan::validate_scoped` constructs a map of runtime images and walks
the finalized bundle's artifact identities. The runtime set must contain each
required image exactly once, with no missing or unexpected IDs. For every
identity/runtime pair it checks:

* runtime ID and image SHA-256 equal the core identity;
* runtime ABI entry equals `entry_symbol`;
* `format` equals `target.abi`;
* runtime workgroup width is nonzero and no larger than the identity's maximum;
* CUDA or HSA backend, architecture, ABI, and backend digest metadata match the
  target identity; and
* every calculation task that names the artifact also names the identity's
  `kernel_template`.

Before that finalized-plan check, the candidate request validator builds the
expected set from Draft `artifacts` plus Draft `artifact_builds` and rejects any
overlap. Every supplied pair must have a unique ID and a valid identity. A
prebuilt identity must equal the immutable Draft identity byte-for-byte. A
deferred identity must match the build's stage identity, resources, and
provenance, have one immutable stage placement, and match the measured GPU
target at that placement. The observed runtime IDs must equal the expected set
exactly, so an extra image cannot be smuggled into a candidate session.

For a deferred build, native ABI validation derives the expected logical lane
count, workgroup width, and ordered typed operands from `ArtifactBuildRecipe`.
For an identity without a build, it derives those values from the finalized
`KernelTemplate`. Either path must agree with the runtime ABI. It rejects
argument count overflow, element-count mismatch, workgroup mismatch, and
operand dtype or storage mismatch as `Error::ArtifactMismatch`.

CUDA resource realization groups runtime entries by image digest, inspects the
cubin entry for the selected compute capability, loads each distinct module
once, and resolves every stage entry into a per-artifact `LoadedArtifact`. A
calculation submission looks up its artifact, fills the invocation from the
validated ABI, computes the grid from `abi.elements` and
`abi.workgroup_lanes`, and launches that exact function. HSA performs the same
identity and ABI checks, then binds a resource envelope copied from the core
identity. Local handoff validation compares the prepared artifact map and
deferred-build vector with the finalized bundle before the immutable
`init -> loop -> exit` run begins.

Target-free dense inference follows the same `Preparer` and native artifact
path, then retains a `RealizedNativeKernelSet` for execution evidence while it
admits inputs once and collects prediction egress after `exit`. KNN inference
also prepares and validates the same artifact contracts, but its completed
execution does not expose the native image set as a public save product. In
both cases artifact selection and ABI validation happen before the one-iteration
run, not inside the inference report layer.

## Hashes, metadata, and serialization boundaries

### Core and planning hashes

`planner::hash_artifact` hashes every `ArtifactIdentity` field: ID, image
digest, format, all target labels, toolchain name/version/digest, entry symbol,
kernel template, all resource bounds, and the optional build provenance.
`hash_artifact_build` hashes every recipe field. These values contribute to the
Draft identity, so changing an artifact contract changes the candidate's
immutable identity.

`prepare::hash_realization` includes the complete deferred-build list and the
realized artifact list, in addition to tasks, aliases, resources, reservations,
and capacity snapshots. `hash_bundle_with_loop_domains` includes the same
artifact/build contracts plus loop domains and final arena allocation offsets.
The resulting `RealizationIdentity` and `BundleIdentity` authenticate the
artifact set without embedding native bytes.

The preparation hash helper sorts artifact identities and deferred builds by
artifact ID before writing them, while preserving the ordered binding list
inside each recipe. It length-prefixes labels and sequences, writes digest bytes
directly, and uses the same access tags and resource fields as the planner
contract hash. A reordered artifact vector therefore has the same canonical
meaning, but a reordered stage binding vector changes the contract as it must.

`recipe-core` has no serialization dependency and `artifact.rs` contains no
`serde`, binary codec, OGDL writer, or path operation. `ArtifactIdentity` and
`ArtifactBuildRecipe` are in-memory contracts. The surrounding preparation and
execution layers hash and validate them; they do not serialize the full core
bundle to a user artifact.

The measured-profile codec is a separate boundary. `probe/src/codec.rs` writes
each discovered calculation capability's target as three length-prefixed labels
in backend, architecture, ABI order, and reads those labels back into a
`TargetIdentity`. It also persists the discovery and topology digests and all
measured capability fields. This makes a target in a cached discovery profile
the same typed value used by planning, but it does not persist an
`ArtifactIdentity` or a deferred build recipe.

### Public training save/load

The public `Train::save` and `Train::resume` declarations are handled in
`src/api.rs` and `src/training.rs`, not in this module:

* one save path is routed by extension to a semantic `.ogdl` model or a native
  `.cubin` or `.hsaco` image;
* the literal two-path form saves both, with the first path required to be
  `.ogdl` and the second required to be `.cubin` or `.hsaco`;
* a resume declaration always requires the semantic model path first, and the
  optional second path is a native kernel;
* an absent semantic resume model is existence-conditional and starts a fresh
  training run; and
* an absent native resume file is also a normal recompilation path once the
  semantic model is available, rather than a reason to export or accept a
  substitute image; and
* KNN and categorical Bayesian preparation can save semantic models but do not
  realize a native training kernel.

For dense training, `compile_training_graph` loads the semantic checkpoint when
the `.ogdl` path exists. If a native resume path is also present,
`load_resume_native_bundle` requires authenticated native metadata, checks the
current program digest, measured topology and discovery identities, target and
toolchain, and the exact source byte digest. It then supplies the bytes as a
prebuilt target bundle. Realize still lowers the current deferred stages and
inspects every entry, so a kernel file never bypasses the current core contract.

After a successful `init -> loop -> exit`, `RealizedNativeKernelSet` is built
from the `NativeArtifact` pairs. It groups entries only when format, target,
toolchain, image digest, and bytes are all equal, while retaining every
`ArtifactIdentity` entry point in the group. This is the bridge from core
artifact identities to the public checkpoint layer.

`CheckpointArtifact::save` writes semantic model fields and tensor images. It
does not embed native bytes. The optional `native` metadata records the program,
realization, topology, discovery, and each native kernel's format, target,
toolchain, and image digest. The decoder reconstructs `TargetIdentity` and
`ToolchainIdentity` with `Label::new` and digest parsing, rejects empty kernel
lists, zero image or toolchain digests, format/backend/ABI disagreement, and
duplicate native identities. `CheckpointArtifact::save_native_kernel` writes
the exact retained bytes, without regenerating or wrapping them, and rejects an
unavailable or ambiguous format.

This metadata is an authentication record, not a serialized
`ArtifactBuildRecipe`, `ArtifactIdentity`, `DraftPlan`, or `FinalizedBundle`.
The native executor always reconstructs and validates the current in-memory
contract before loading bytes.

## Failure map

| Boundary | Observable failure | Meaning |
| --- | --- | --- |
| Direct recipe validation | `ValidationErrors` with `InvalidIdentity`, `MissingRequiredObject`, `DuplicateId`, or `ResourceMismatch` | The target-independent recipe is internally malformed. |
| Direct identity validation | `ValidationErrors` with `InvalidIdentity` or `MissingRequiredObject` | The image or toolchain digest, build provenance, or maximum workgroup bound is unusable. |
| Planner catalog | `InvalidArtifact` | Catalog IDs are zero or duplicated, or a catalog identity does not exactly realize the reserved stage and measured target. |
| Planner lowering | `InvalidDraft`, `IdentityCollision`, `NoViableCandidate`, or `CandidateInfeasible` | A binding, stage identity, resource peak, or placement cannot satisfy the immutable Draft. |
| Draft validation | `UnknownReference`, `DuplicateId`, `ArtifactMismatch`, or `ResourceMismatch` | A task does not resolve to exactly one artifact/build or its task, value, target, fault, or ABI contract differs. |
| Deferred target resolution | `MissingBuildTarget` or `InvalidCandidate` | A deferred artifact has no current target specification, is unused, or spans more than one native target. |
| Kernel lowering | `LoweringErrorKind::InvalidStageContract`, `ArtifactMismatch`, `InvalidEntrySymbol`, or `InvalidWorkgroupSize` | The recipe differs from the canonical lowered stage or target options. |
| Native artifact pairing | `InvalidArtifact` | Runtime ID, digest, entry, format, target, backend kind, or toolchain identity disagrees. |
| Candidate native request | `MissingRuntimeArtifact`, `UnexpectedArtifact`, `DeferredArtifactMismatch`, `MissingStagePlacement`, or `TargetMismatch` | The loaded image set does not exactly cover the candidate's static and deferred requirements. |
| Native execution plan | `Error::ArtifactMismatch`, `MissingArtifact`, or `DuplicateArtifact` | The finalized bundle and runtime image set cannot produce a safe ABI-bound launch. |
| Public save declaration | Declaration error | A path is empty, has the wrong extension, or repeats a save/resume declaration. |
| Native resume load | Incompatible-resume or native-kernel source error | Metadata, current measured identities, program digest, format, or source bytes do not authenticate the supplied image. |
| Native save | Native-kernel unavailable or ambiguous | The completed run has zero or multiple images for the requested format. |

Failures are intentionally not hidden by retries or substitute artifacts. A
candidate that cannot satisfy the observed target and resource contract is
destroyed and the planner's next finite candidate is attempted. A malformed
contract, missing target specification, or inconsistent finalized handoff is a
fatal boundary error.

## Invariant checklist

For a valid realized calculation, all of the following must hold:

1. The artifact ID is the stage-scoped identity reserved by Draft.
2. The image digest and runtime bytes are identical.
3. The target and toolchain identities describe the measured backend and exact
   producer.
4. The format equals the target ABI and the entry symbol equals the runtime ABI.
5. The build provenance, when present, proves the exact lowered program and
   stage contract.
6. Ordered bindings, access modes, affine views, geometry, operation bounds,
   fault storage, and resources match the primitive stage.
7. The calculation task names the same kernel template, artifact, ordered
   inputs, outputs, and fault value.
8. The finalized bundle contains every required artifact exactly once and no
   artifact absent from Draft.
9. The native executor validates the complete image set and ABI before loading
   or launching anything.
10. Public model metadata authenticates a native image, while public native
    save writes the exact bytes retained from the successful realization.
