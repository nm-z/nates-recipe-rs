# `primitives/src/validate.rs`: canonical lowered-program validation

## Purpose and boundary

`recipe-primitives` lowers one already-resolved
`recipe_language::PrimitiveKernel` into an immutable, backend-neutral
`LoweredProgram`. [`validate.rs`](../../src/validate.rs) is the final integrity
boundary inside that crate. It does not discover hardware, inspect runtime
state, allocate memory, select a target, emit LLVM, or execute a stage. It
checks that the record emitted by the primitive lowerer is a complete and
canonical representation of the source kernel's stages, views, synchronization
and resource envelope.

The public entry point is `LoweredProgram::validate` in `model.rs:519-526`:

```rust
pub fn validate(&self) -> Result<(), Vec<ProgramValidationError>> {
	crate::validate::validate(self)
}
```

The implementation function is `pub(crate) fn validate` at
`validate.rs:8`. The model method exposes the check to the planner, kernel
realization, and callers of the crate. The implementation remains private so
that all construction and revalidation use one rule set.

Validation is structural and deterministic. It consumes only the immutable
`LoweredProgram` and the constants and pure helper functions in this module.
It never normalizes an invalid field, fills in a missing stage, or chooses an
alternate representation. Every discovered failure is retained as a
`ProgramValidationError { path, detail }`; success is returned only when the
error vector is empty.

The validator is deliberately downstream of language validation. The source
`PrimitiveKernel::validate` call in `lower.rs` owns source tensor lookup,
primitive arity, source shapes and dtypes, alias matrix meaning, and primitive
parameter legality. This module owns the emitted program contract: dense IDs,
typed static views, stage ABI and fault records, fixed algorithm plans, exact
resource accounting, and the digest that authenticates all of those fields.

## Record being validated

`LoweredProgram` (`model.rs:507-517`) contains the following authoritative
state:

| Field | Validation role |
| --- | --- |
| `schema_version` | Must equal `LOWERED_PROGRAM_SCHEMA_VERSION`, currently `2`. |
| `source_kernel` | Source identity retained for planner and artifact provenance. It is included in the digest but has no independent range check here. |
| `source_input_count`, `source_output_count` | Arity used to validate the complete source alias matrix. |
| `source_aliases` | One `SourceAliasContract` for every input/output pair, with a source `AliasPermission`. |
| `buffers` | Dense typed storage records for external tensors, one-dimensional scratch, and the optional four-byte fault flag. |
| `stages` | Ordered `ProgramStage` records. Each stage owns dispatch geometry, buffer bindings, synchronization points, atomic contracts, an optional fault contract, exact resources, and a `StageKind`. |
| `resources` | Exact aggregate `ProgramResourceBounds` derived from all stages and non-external buffers. |
| `digest` | Domain-separated canonical digest of every program field except the stored digest itself. |

The model types used by the validator are all public and immutable by
convention, although their fields are public Rust fields. The relevant shape
and state relationships are:

* `BufferId` and `StageId` wrap `u32`. Their numeric values are table indices,
  not arbitrary names. `validate_buffers` and `validate_stages` therefore
  require dense, zero-based order.
* `StaticAccess` is an affine view: `logical_extents`, `offset_elements`,
  `strides`, and `storage_bytes`. A view can be read-only and overlapping, but
  a non-atomic writable view must be injective over its non-unit axes.
* `ProgramBuffer.shape` is the declared logical tensor shape. Its canonical
  `access.logical_extents` must be exactly the same vector. Tensor buffers are
  external values, scratch buffers are one-dimensional program-owned storage,
  and the fault buffer is an int32 shape `[1]` with four bytes of storage.
* `BufferBinding.view` is a complete static view into a referenced program
  buffer. `ReadWriteAtomic` is allowed to overlap because atomic operations,
  rather than ordinary writes, resolve conflicts.
* `DispatchGeometry` is one-dimensional: logical lanes, workgroup lanes, and
  the number of workgroups. Higher-rank addressing remains in each static
  binding view.
* `ProgramStage.dependencies` is the immutable dispatch chain. Lowering emits
  no arbitrary graph here: stage `n` depends only on stage `n - 1`.
* `StageKind` is the owned algorithm contract consumed by `recipe-kernel`.
  It covers scalar maps, internal fill and copy stages, fixed reduction and
  scan trees, uniform scan combines, tiled contractions, gather and scatter,
  histogram clear and accumulation, stable sorting, index maps, and
  Philox4x32-10 random maps.

## Top-level order and result semantics

`validate` executes the following sequence (`validate.rs:8-35`):

```text
schema version
source alias matrix
all buffers and static accesses
all stages, bindings, atomics, faults, kinds, resources, and sync counts
program resource aggregate
canonical digest
```

Each phase appends to one `Vec<ProgramValidationError>`. The function does not
return after the first failed `require`; local lookup guards in binding
validation and overflow guards in aggregate validation are the only early
continues or returns. If the vector is empty, `errors.is_empty().then_some(())`
returns `Ok(())`; otherwise the complete vector is returned as `Err(errors)`.
The order of the vector is therefore stable: top-level checks first, then
buffer order, stage order, and finally aggregate and digest checks.

`source_kernel` is not passed to the validator as context. Consequently paths
identify fields such as `stages[2].bindings[1].view.strides`, not a kernel
name. Callers that need source identity add it while converting the vector to
their own error type.

## Schema and source alias rules

### Schema version

The first `require` compares `program.schema_version` with
`LOWERED_PROGRAM_SCHEMA_VERSION` (`2`). A mismatch reports path
`schema_version` and the detail `schema N is unsupported; expected 2`. No
version migration is attempted.

### Complete alias matrix

`validate_source_aliases` (`validate.rs:37-64`) treats the source alias list as
a matrix, not as an optional list:

1. For each entry `source_aliases[index]`, `input < source_input_count` and
   `output < source_output_count` must both hold. An out-of-range pair reports
   `source alias pair exceeds the declared input/output arity`.
2. A `BTreeSet<(usize, usize)>` rejects a second entry for the same input and
   output pair, regardless of its permission. The path is the entry path and
   the detail is `source input/output alias pair appears more than once`.
3. The number of entries must equal
   `source_input_count.saturating_mul(source_output_count)`. A shorter list
   reports `source_aliases: source alias matrix is incomplete`. The comparison
   intentionally uses saturating multiplication, so there is no separate
   arithmetic-overflow diagnostic for an unrepresentable theoretical matrix.

The `permission` value is retained and hashed, but this layer does not decide
whether a particular `Forbidden`, `MayAliasExact`, or `MustAliasExact` value is
semantically appropriate. The language kernel validator owns the source alias
contract, and the planner later enforces `MustAliasExact` against the actual
typed static views.

## Buffer and static-access rules

### Buffer identity, shape, origin, and lifetime

`validate_buffers` (`validate.rs:66-136`) runs in table order. For every
`buffers[index]` it checks:

* `usize::try_from(buffer.id.get()) == Ok(index)`. IDs must be dense and
  ordered from zero. A value that cannot be represented as `usize` also fails
  this check.
* `buffer.shape` is nonempty. A zero extent is allowed, but rank zero is not.
* `buffer.shape == buffer.access.logical_extents`. The canonical buffer view
  must describe the complete declared shape, not a subview.
* The canonical access is read-only for purposes of overlap validation, then
  passes the general static-access checks described below.
* The origin key is unique. Tensor origins key by `(0, value_id, 0)`, scratch
  origins by `(1, ordinal, scratch_purpose_tag(purpose))`, and the fault flag by
  `(2, 0, 0)`. Thus the same tensor cannot become two buffers, a scratch
  ordinal cannot be reused for the same purpose, and only one fault origin is
  possible.

The origin/lifetime/type/shape combinations are intentionally narrow:

| Origin | Required lifetime and shape/type |
| --- | --- |
| `BufferOrigin::Tensor(_)` | `BufferLifetime::ExternalValue`; any nonempty rank and the tensor dtype. |
| `BufferOrigin::Scratch { .. }` | `BufferLifetime::ProgramScratch`; exactly one shape axis (`shape.as_slice()` matches `[_]`). |
| `BufferOrigin::FaultFlag` | `BufferLifetime::ProgramFaultFlag`, `DType::I32`, shape exactly `[1]`. |

At least one of the tensor or scratch standard contracts, or the exact fault
contract, must match. A fault-contract buffer must additionally report
`ByteCount::new(4)` storage. The validator does not impose a dtype on scratch
buffers or a nonzero storage requirement for an empty extent; it checks the
actual access arithmetic and the origin contract that is present.

`ScratchPurpose` is mapped to stable tags by `scratch_purpose_tag`: reduction
values `0`, reduction indices `1`, scan values `2`, scan block totals `3`, sort
values `4`, and sort indices `5`. The tags are used only for origin uniqueness
in this module, but the same enum is included in the canonical digest.

### Static access arithmetic

`validate_access` (`validate.rs:149-205`) is used for a buffer's canonical
access and for every binding view. Its `dtype` argument determines bytes per
element through `DType::byte_width()`, which is four for both `F32` and `I32`.

The checks are:

1. The extent and stride vectors have equal rank. A mismatch reports
   `*.strides: static access extent and stride ranks differ`.
2. If any logical extent is zero, the required storage is defined as zero. For
   a nonempty view, `access_required_bytes` computes the highest addressed
   element with checked arithmetic:

   ```text
   maximum = offset_elements + sum((extent - 1) * stride)
   elements = maximum + 1
   required_bytes = elements * dtype.byte_width()
   ```

   Any subtraction, multiplication, addition, or final byte multiplication
   overflow reports `*.access: static access address calculation overflows
   u64`.
3. The required byte count must be at most `storage_bytes`. Failure reports
   `*.storage_bytes: static access exceeds its N byte storage`.
4. For injectivity, only axes with `extent > 1` are considered. They are
   sorted by stride. Starting with `occupied = 1`, each axis must have
   `stride >= occupied`; its contribution `(extent - 1) * stride` must not
   overflow and is added to `occupied` with checked arithmetic.
5. A writable view is required to be injective unless it is empty or the rank
   vectors already differ. The rank-mismatch disjunct prevents a second
   overlap diagnostic after the primary rank diagnostic. Read-only views may
   overlap or broadcast. Atomic writable views are passed with `writable =
   false` by `validate_bindings`, so their overlap is intentionally not
   rejected here.

The rank check does not short-circuit the byte calculation: on a mismatch,
`access_required_bytes` still folds the pairs available from `zip`, then the
rank diagnostic is retained. The writable injectivity condition explicitly
allows a rank mismatch so that the same malformed view does not add a second
overlap diagnostic. The cross-buffer binding check compares declared storage
bytes only; it does not compare a binding offset or shape against the
canonical buffer offset beyond the binding view's own arithmetic checks.

The injectivity test rejects a zero stride on an axis with extent greater than
one, rejects overlapping strides, and accepts ordinary contiguous or padded
layouts when the ordered occupied span permits them. Unit axes do not affect
injectivity, although their strides still participate in required-byte
calculation when the rank is valid.

## Common stage rules

`validate_stages` (`validate.rs:221-270`) validates every `stages[index]` and
then delegates to five stage-specific helpers:

```text
validate_bindings
validate_atomics
validate_fault
validate_kind
validate_stage_resources
```

Before those helpers, every stage must satisfy:

* `stage.id` is exactly `StageId::new(index)`.
* Dependencies equal the one-element vector containing the immediately
  preceding stage ID, or an empty vector for stage zero. There is no allowance
  for a second dependency, a forward dependency, or a dependency on an
  earlier non-immediate stage.
* `geometry.logical_lanes` is nonzero.
* `geometry.workgroup_lanes` is nonzero.
* `geometry.workgroups` equals
  `geometry.logical_lanes.div_ceil(u64::from(geometry.workgroup_lanes))`.

`DispatchGeometry` is therefore a complete one-dimensional launch contract.
The validator does not infer a different grid or round a caller-provided
workgroup count. There is no requirement that `stages` be nonempty. A
zero-dispatch lowered program can validate when its empty stage aggregate,
buffer storage aggregate, and digest are canonical.

### Buffer bindings

`validate_bindings` (`validate.rs:273-321`) processes each binding in order.
It first converts `binding.buffer.get()` to `usize`. A conversion failure
reports `*.buffer: buffer identifier cannot index the buffer table`; an absent
slot reports `*.buffer: binding references an absent buffer`. Both cases skip
the remaining checks for that binding.

For a present buffer it requires:

* `buffer.id == binding.buffer`, preserving the dense slot identity;
* `buffer.dtype == binding.dtype`;
* `binding.view.storage_bytes <= buffer.access.storage_bytes`; and
* `validate_access` succeeds for the binding view.

`Read`, `Write`, `ReadWrite`, and `ReadWriteAtomic` are the four modes. Only
`Write` and `ReadWrite` set the `writable` argument for injectivity. A
`ReadWriteAtomic` view may overlap because the corresponding atomic contracts
define conflict resolution. Binding count and ordering are checked by
`validate_kind` only for stage kinds with a fixed ABI; this helper itself does
not require a particular number of bindings or uniqueness of buffer IDs.

### Atomic contracts

`validate_atomics` (`validate.rs:323-385`) checks each `AtomicContract`:

* The tuple `(buffer, domain, operation, ordering)` must be unique. The tuple
  deliberately excludes dtype, so changing only dtype does not make two
  otherwise identical contracts distinct.
* A binding for the same buffer with mode `ReadWriteAtomic` must exist.
* That binding's dtype must equal the atomic contract dtype.

The enum-to-tag helpers use stable tags for the uniqueness set:

* address domains: `SingleFaultFlag = 0`, `TensorElements = 1`,
  `HistogramBins = 2`;
* operations: `Exchange = 0`, `Add = 1`, `Minimum = 2`, `Maximum = 3`; and
* orderings: `Relaxed = 0`, `Acquire = 1`, `Release = 2`,
  `AcquireRelease = 3`, `SequentiallyConsistent = 4`.

There is no reverse requirement that every `ReadWriteAtomic` binding have an
atomic entry. Stage-kind and kernel realization checks supply any stronger ABI
relationship needed by a particular operation.

### Fault contracts

`validate_fault` (`validate.rs:387-417`) returns immediately when a stage has
no fault contract. For `Some(fault)` it requires:

* `guard_before_address` is `true`, so a rejected lane publishes before it
  forms or issues an invalid payload address;
* `fault.flag == fault.publish.buffer`;
* publication uses `AtomicAddressDomain::SingleFaultFlag`,
  `OwnedAtomicOperation::Exchange`, and `DType::I32`; and
* the exact `fault.publish` contract appears in `stage.atomics`.

The ordering of the fault publication is not constrained by this helper, and
the reason and numeric code are retained but not range-checked here. The
builder's `checked_fault` constructor emits a release exchange, and the later
kernel stage contract checks the ordered fault binding and ABI.

## Stage-kind rules

`validate_kind` (`validate.rs:419-619`) dispatches on the owned `StageKind`.
The following are the complete checks at this layer. Fields not listed remain
part of the digest and are consumed by later source or target-specific
validators, but are not independently constrained here.

### `ScalarMap { template }`

* `template.validate()` must succeed. Any core validation errors are converted
  to one `ProgramValidationError` at `*.kind.template` with the core error's
  display text.
* `template.index_space.elements().get()` must equal
  `stage.geometry.logical_lanes`.
* Binding count must equal
  `template.inputs.len() + template.outputs.len() + usize::from(stage.fault.is_some())`.
* `template.program.requires_fault_flag()` must equal
  `stage.fault.is_some()`.

This preserves the scalar program's complete input/output ABI and its optional
arithmetic fault channel. The core `KernelTemplate` validator owns scalar
instruction typing, IDs, static input/output accesses, arity, and alias rules.

### `FixedTreeReduce`

`validate_reduction_tree` requires a power-of-two tree width in `1..=1024` and
requires the exact descending-stride sequence generated by
`expected_reduction_steps`:

```text
for level = 0 .. tree.lanes.trailing_zeros():
    TreeStep { phase: Reduction, stride: tree.lanes >> (level + 1) }
```

For a one-lane tree the canonical step list is empty. The reduction stage then
requires:

* `output_width == input_width.div_ceil(u64::from(tree.lanes))`; and
* `tie_break == ReductionTieBreak::LowestLogicalIndex`.

The latter fixes deterministic value/index behavior. `pass`, operator, dtype,
sequence count, reduced axes, padding, and nonzero input width are retained in
the contract and digest, but are not separately checked by this function.

### `FixedTreeScanLocal`

`validate_scan_tree` uses the same power-of-two `1..=1024` lane range. Its
canonical list is a Blelloch tree generated by `expected_scan_steps`:

```text
upsweep:   TreeStep { phase: ScanUpsweep,   stride: 1 << level }
           for level = 0 .. lanes.trailing_zeros()
downsweep: TreeStep { phase: ScanDownsweep, stride: 1 << level }
           for level = (lanes.trailing_zeros() - 1) ..= 0
```

The actual code appends the reverse iterator, so the down-sweep is the exact
reverse level order. The stage requires
`output_width == input_width.div_ceil(u64::from(tree.lanes))`. Operator, dtype,
axis, direction, mode, identity, level, and sequence fields are not otherwise
checked here.

### `TiledContraction`

The stage must set `canonical_contracted_order` to `true`. All tile dimensions
`output_x`, `output_y`, and `reduction` must be nonzero, and the output tile
must fill the realized workgroup:

```text
output_x.checked_mul(output_y) == Some(stage.geometry.workgroup_lanes)
```

The checked multiplication makes an overflowing tile fail rather than wrap.
The contraction axes, dtype, element counts, and direct versus staged strategy
remain explicit contract data. Their resource and synchronization formulas
below still consume those values.

### `Gather` and `Scatter`

For both indexed kinds, `IndexBounds::Reject` must be equivalent to the
presence of a fault contract:

```text
(bounds == IndexBounds::Reject) == stage.fault.is_some()
```

Clamp and wrap therefore cannot carry a preallocated rejection flag, while
rejecting an out-of-range index must have one. Source arity, index dtype, and
derived update shapes are owned by language validation. Scatter conflict
operations and orderings are retained in the kind and atomic entries and are
consumed by target realization.

### `HistogramAccumulate`

The stage must have a fault contract whose `reason` is exactly
`FaultReason::HistogramBinOutOfBounds`. This prevents invalid bins from being
silently discarded. Bin count, weighted input shape, mapping, and ordering
remain explicit fields; source legality comes from language validation and the
atomic ABI is checked by the binding and atomic helpers.

### Stable sort kinds

`StableSortInitialize` and `StableSortFinalize` call
`validate_sort_network`. `StableSortCompareExchange` does so as well and adds
the fixed-network phase checks:

* `merge_width` is a power of two and no greater than
  `network.padded_axis_length`;
* `compare_distance` is a power of two; and
* `compare_distance < merge_width`.

`validate_sort_network` requires a nonzero `axis_length`, a power-of-two
`padded_axis_length` at least as large as the axis, and exact least padding:
`padded_axis_length == axis_length.next_power_of_two()`. It also requires
`SortTieBreak::OriginalAxisIndexAscending`,
`FloatSortOrder::Ieee754TotalOrder`, and
`SortPadding::AfterAllValidElements`. Direction, slice count, axis number, and
`requested_stable` are not independently checked at this layer.

### `Philox4x32_10`

The random contract must be Recipe's exact Philox4x32-10 form:

* `rounds == 10`;
* multipliers are `0xd251_1f53` and `0xcd9e_8d57`;
* Weyl constants are `0x9e37_79b9` and `0xbb67_ae85`;
* the counter words are exactly
  `[ElementLow, ElementHigh, IterationXorStreamLow, IterationXorStreamHigh]`;
* both `fold_kernel_id_into_key` and `fold_run_id_into_key` are `true`.

The key, distribution, and mapping enums remain part of the authenticated
stage contract. The fixed-variant checks for `uniform_i32` and `normal_f32`
are implemented by the target emitter, not by this match arm.

### `IndexMap`

An index-map stage has exactly two bindings, one output and one fault binding
in the emitted ABI. Its optional `modulus` must be strictly positive, and the
stage must carry `Some(FaultContract { reason: ArithmeticDomain, .. })`.
The affine start and step values are not range-checked here.

### Internal fixed-ABI kinds

The following kinds have only binding-count checks in `validate_kind`:

| Kind | Required binding count |
| --- | ---: |
| `Fill` | `1` output |
| `Copy` | `2`, one input and one output |
| `ScanUniformCombine` | `2`, target and offset |
| `HistogramClear` | `1` output |

`CopyReason` and the other payload fields remain in the canonical digest and
are interpreted by the lowerer and kernel emitter. There is no wildcard arm;
the match covers every current `StageKind` variant.

## Exact stage resources and synchronization

### Stage resource derivation

`validate_stage_resources` (`validate.rs:708-782`) recomputes one
`StageResourceBounds` from the stage kind and geometry. It requires the stored
value to equal the recomputed value. Any checked arithmetic overflow produces
one `*.resources: stage resource arithmetic overflowed` error instead of a
possibly wrapped bound.

The formulas below use:

* `L = stage.geometry.logical_lanes`;
* `W = stage.geometry.workgroup_lanes`;
* `S = sequences`, `O = output_width`, `I = input_width`;
* `T = tree.lanes`; and
* `F(opcode)` for the scalar opcode's core FLOP count.

The tuple order is `(flops, integer_operations, atomic_operations,
shared_bytes_per_workgroup, private_bytes_per_lane)`.

| Stage kind | Exact resource tuple before `maximum_workgroup_lanes = W` |
| --- | --- |
| `ScalarMap` | `(sum(F(instruction)) * L, L, fault_present * L, 0, (inputs + constants + instructions) * 4)` |
| `Fill` | `(0, L, 0, 0, 4)` |
| `Copy` | `(0, L, 0, 0, 8)` |
| `FixedTreeReduce` | `groups = S * O`, `combines = groups * (T - 1)`, `multiplier = 2` for `ValueAndIndex` else `1`; `(combines * multiplier, L, 0, T * words * 4, words * 8)`, where `words = 1 + (result != Value)` |
| `FixedTreeScanLocal` | `groups = S * O`, `tree = groups * (2 * (T - 1))`, `inclusive = S * I` only for `UserInclusive`; `(tree + inclusive, L, 0, T * 4, 12)` |
| `ScanUniformCombine` | `(L, L, 0, 0, 12)` |
| `TiledContraction` | `shared = 0` for `Direct`, otherwise `W * dtype.byte_width()`; `(output_elements * contracted_elements * 2, L, 0, shared, 24)` |
| `Gather` | `(0, L, fault_present * L, 0, 16)` |
| `Scatter` | `arithmetic = 1` for atomic add/min/max, otherwise `0`; `payload_atomic = 1` for any atomic conflict, `fault_atomic = fault_present`; `(L * arithmetic, L, L * (payload_atomic + fault_atomic), 0, 16)` |
| `HistogramClear` | `(0, L, 0, 0, 4)` |
| `HistogramAccumulate` | `(L, L, 2 * L, 0, 16)` |
| `StableSortInitialize` | `(0, L, 0, 0, 16)` |
| `StableSortCompareExchange` | `(L, L, 0, 0, 20)` |
| `StableSortFinalize` | `(0, L, 0, 0, 12)` |
| `IndexMap` | `(0, L * INDEX_MAP_INTEGER_OPERATIONS_PER_LANE, L, 0, 32)`, with the constant currently `9` |
| `Philox4x32_10` | `(L * rounds * 4, L * 20, 0, 0, 32)` |

The source uses checked arithmetic for dynamic `u64` bounds and reports an
overflow instead of accepting a wrapped result. Fixed factors, bounded tree
expressions, and the scalar-program slot-count addition are ordinary
operations exactly as written in `expected_stage_resources`. The result is
wrapped in `FlopCount` and `ByteCount` newtypes, retaining the unit-bearing
representation used by the rest of the workspace.

### Synchronization count

The validator checks the length of `stage.synchronization`, not the contents
of each `SynchronizationPoint` (`after_step`, scope, or memory semantics).
Expected lengths are:

| Stage kind | Required synchronization entries |
| --- | ---: |
| `FixedTreeReduce` | `tree.steps.len()` |
| `FixedTreeScanLocal` | `tree.steps.len()` |
| `TiledContraction` with `Direct` strategy | `0` |
| `TiledContraction` with `Staged` strategy | `contracted_elements.div_ceil(tile.reduction).saturating_mul(2)` |
| Every other current kind | `0` |

For a staged contraction the expected count is converted to `usize`. A
conversion failure appends a `synchronization count conversion failed: ...`
error and substitutes the current synchronization length, so the subsequent
length check does not add a duplicate diagnostic. The `None` branch in the
match reports `synchronization count cannot be represented`; with the current
variants, all ordinary branches resolve to `Some`.

### Program resource aggregate

`validate_resources` (`validate.rs:933-997`) recomputes the program envelope
independently of the stored `program.resources`:

* Stage FLOPs, integer operations, and atomic operations are checked sums.
* Shared bytes per workgroup, private bytes per lane, and maximum workgroup
  lanes are the respective maxima across stages.
* `ProgramScratch` storage bytes are a checked sum into
  `persistent_scratch_bytes`.
* `ProgramFaultFlag` storage bytes are a checked sum into `fault_bytes`.
* `ExternalValue` storage is deliberately excluded from program-owned storage
  accounting.

An overflow in stage totals reports `resources: program resource aggregation
overflowed` and returns without attempting the storage aggregate. An overflow
in storage reports `resources: program storage aggregation overflowed` and
returns. Otherwise the constructed `ProgramResourceBounds` must equal the
stored value exactly, or the validator reports
`program resource bounds differ from exact aggregate {expected:?}`.

## Digest integrity

The final top-level check compares `program.digest` to
`program.canonical_digest()` (`model.rs:524-526`). `hash.rs::program_digest`
uses the domain string `recipe-lowered-primitive-program-v2\0`, then encodes
the schema, source identity and arity, every alias and permission, every
buffer, every stage, and the complete aggregate resource record. Sequences are
length-prefixed and scalar integers are encoded in the canonical little-endian
format used by `CanonicalWriter`; the result is SHA-256 in `ProgramDigest`.

The stored digest is not included as an input to its own recomputation. A
caller cannot alter a stage, static view, fault contract, resource bound, or
source alias without changing the expected digest. A mismatch reports
`digest: program digest does not match its canonical contents`. No digest is
repaired in place.

The planner uses this digest when it creates stage-scoped artifact identities,
and deferred preparation finds a lowered program by source kernel plus digest.
The kernel realization boundary repeats the canonical validation and compares
the artifact build provenance digest with the program digest before emitting
LLVM.

## Construction and error propagation

### Primitive lowering constructor

`primitives/src/lower.rs` creates the values that this validator expects:

1. `lower` validates measured `LoweringHardware`, then calls
   `PrimitiveKernel::validate(tensors)` and maps a language failure to
   `LoweringErrorKind::InvalidLanguage`.
2. `ProgramBuilder::add_tensor` emits one `ExternalValue` buffer per distinct
   tensor. `scratch` emits one-dimensional `ProgramScratch` buffers and
   `fault` emits at most one `[1]` int32 `ProgramFaultFlag` with four bytes.
3. `push_stage` assigns dense stage IDs and the immediately previous stage as
   the dependency. Each primitive-specific lowerer supplies geometry,
   bindings, atomics, fault, kind, and initial resource values.
4. `finish` aggregates resources, constructs a schema-versioned program,
   computes its canonical digest, and calls `program.validate()` before
   returning. A validation vector is joined with `; `, wrapped in
   `LoweringErrorKind::InvalidLoweredProgram`, and annotated with the source
   kernel ID by `for_kernel`.

The validator is therefore both a constructor postcondition and a public
revalidation operation. A malformed value manufactured outside the builder is
not normalized to match the builder's output.

### Operation facade and registry

`ops/src/primitive.rs::lower_primitive` selects a direct primitive recipe and
calls `recipe_primitives::lower`. A recipe mismatch is rejected before the
primitive crate. Any `LoweringError`, including the joined
`InvalidLoweredProgram` produced by `finish`, is converted to
`OperationErrorKind::PrimitiveLoweringFailed`; the descriptor ID is attached
by `for_operation`. `ops/src/lib.rs` and the root facade re-export
`LoweredProgram`, so an advanced caller can also invoke its public
`validate()` directly.

### Planner

`planner/src/planner.rs::lower_programs` selects one common measured
`LoweringHardware`, lowers each `CalculationGraph` node, and immediately calls
`program.validate()` again (`planner.rs:359-393`). Failure becomes
`PlannerErrorKind::InvalidGraph` with the kernel ID and all formatted
`path: detail` entries joined by `; `. The planner then consumes only a
validated program to:

* materialize external, scratch, and fault buffers while preserving lifetimes
  and static views;
* emit one calculation task for each immutable stage and translate the
  previous-stage dependency;
* create stage-scoped artifact build recipes whose digest, dispatch, bindings,
  work, fault, and resources must match the stage; and
* enforce source `MustAliasExact` rules against the typed static views.

The validator does not perform placement, arena allocation, route selection,
or scheduling. Those are planner and scheduler state, not primitive semantics.

### Kernel and deferred native preparation

`prepare/src/production.rs::lower_deferred_stage` finds the exact lowered
program in a `PlannedCandidate` by `source_kernel` and
`build.provenance.program_digest`, then calls `recipe_kernel::lower_stage`.
`kernel/src/stage.rs::validate_contract` begins with
`program.validate().is_ok()`. A failure is intentionally collapsed to the
single `LoweringErrorKind::InvalidStageContract` message
`lowered program failed canonical validation`; the kernel then checks the
artifact build recipe, target, digest, stage identity, dispatch, work bounds,
bindings, resource envelope, and fault ABI. No LLVM is emitted until all of
those checks pass.

This creates the complete end-to-end boundary:

```text
PrimitiveKernel + tensor index + measured limits
    -> recipe_primitives::lower
    -> LoweredProgram::validate (constructor postcondition)
    -> planner revalidation and stage/task/artifact contracts
    -> deferred preparation finds source-kernel + digest
    -> recipe_kernel::lower_stage revalidates the complete program
    -> target LLVM and native artifact realization
    -> runtime execution
```

The primitive validator remains pre-loop AOT work. It authenticates the
calculation and transfer contracts that later scheduling, realization, and
execution are allowed to consume; it does not itself publish runtime state.

## Deliberate limits and malformed-input behavior

The validator is exhaustive for the fields and relationships it owns, but it
does not duplicate upstream or downstream policy:

* Source tensor existence, primitive arity, dtypes, axes, derived shapes,
  source alias permissions, random distribution bounds, and scalar SSA
  validity belong to `recipe-language` and `recipe-core` validation.
* Binding counts are required only where a `StageKind` has a fixed ABI. The
  validator does not infer missing bindings from the kind.
* Synchronization point contents are not interpreted, only the required count
  is checked. The kernel stage contract and emitter consume the ordered points.
* The stage validator does not independently check every payload field such as
  reduction operator, scan axis, contraction axes, histogram mapping, sort
  direction, or random mapping. They remain authenticated by the digest and
  are interpreted by the corresponding lowerer and target emitter.

Checks are error-collecting rather than short-circuiting. Several arithmetic
expressions intentionally rely on the nonzero conditions checked earlier in
the same stage: workgroup count divides by `workgroup_lanes`, tree output and
resource formulas use `tree.lanes`, and staged contraction synchronization
uses `tile.reduction`. Because later checks still run after an earlier failure,
an externally fabricated record with a zero workgroup width, zero tree lanes,
or zero staged-contraction reduction tile can panic in those arithmetic
expressions before returning its accumulated errors. Builder-produced records
cannot enter those states because its geometry and tree/tile selectors choose
positive widths; this is an implementation characteristic to preserve or
repair deliberately, not an implicit fallback contract.

The helper `require` (`validate.rs:999-1005`) is the sole generic error
constructor used for boolean rules. It appends exactly one
`ProgramValidationError::new(path, detail)` when `condition` is false. Direct
`errors.push` sites are reserved for absent bindings, resource arithmetic
overflow, and synchronization conversion overflow. This keeps all diagnostics
path-addressable while leaving the underlying invalid transition visible to
the caller.

## Source map

| Source | Responsibility |
| --- | --- |
| `primitives/src/validate.rs:8-35` | Top-level order and all-error result. |
| `primitives/src/validate.rs:37-64` | Complete source alias matrix. |
| `primitives/src/validate.rs:66-147` | Buffer identity, origin/lifetime contracts, and scratch tags. |
| `primitives/src/validate.rs:149-219` | Static-access rank, byte-span, storage, and injectivity rules. |
| `primitives/src/validate.rs:221-321` | Stage identity, dependency, geometry, and buffer binding rules. |
| `primitives/src/validate.rs:323-417` | Atomic uniqueness and fault publication rules. |
| `primitives/src/validate.rs:419-619` | Stage-kind contracts and fixed binding/fault relationships. |
| `primitives/src/validate.rs:621-706` | Reduction, scan, and sort canonical helper plans. |
| `primitives/src/validate.rs:708-931` | Exact stage resource and synchronization formulas. |
| `primitives/src/validate.rs:933-997` | Program aggregate resources. |
| `primitives/src/validate.rs:999-1005` | Shared `require` diagnostic constructor. |
| `primitives/src/model.rs:507-526` | Public `LoweredProgram` record, `validate`, and digest methods. |
| `primitives/src/hash.rs:9-27` | Domain-separated canonical digest input. |
| `primitives/src/lower.rs:51-95` | Primitive lowering entry and source-language prevalidation. |
| `primitives/src/lower.rs:310-395` | Dense stage construction, resource aggregation, digest, and constructor postcondition. |
| `planner/src/planner.rs:359-393` | Per-kernel planner revalidation and error mapping. |
| `kernel/src/stage.rs:122-207` | Realization revalidation and artifact contract checks. |
| `prepare/src/production.rs:523-547` | Exact source-kernel plus digest lookup before target lowering. |
