# Primitive lowering

This document describes `primitives/src/lower.rs`, the only constructor for a
backend-neutral `LoweredProgram`. The module turns one validated,
placement-free `recipe_language::PrimitiveKernel` and the graph's typed tensor
index into an immutable sequence of GPU calculation stages. It fixes the
algorithm, tensor views, launch geometry, scratch and fault storage,
synchronization, atomics, and exact resource bounds. It deliberately stops
before target code generation, artifact selection, queue scheduling, memory
placement, or execution.

The complete transition is:

```text
PrimitiveKernel + BTreeMap<ValueId, &Tensor> + measured LoweringHardware
        |
        | validate_hardware
        | PrimitiveKernel::validate
        | register source tensor buffers
        v
one PrimitiveKind-specific lowering routine
        |
        | ProgramBuilder stages, scratch, fault contracts, resource bounds
        v
ProgramBuilder::finish
        |
        | aggregate resources
        | canonical program digest
        | LoweredProgram::validate
        v
immutable LoweredProgram
        |
        +--> recipe-planner stage tasks and deferred artifact build recipes
        +--> recipe-kernel exact stage LLVM lowering
        +--> recipe-prepare and native executor realization
```

`lower` does not infer an operation from a public symbol. `recipe-ops` first
matches a descriptor to an explicit `PrimitiveRecipe`, then calls this module
with the already materialized kernel. The planner also calls `lower` directly
for every node in a validated calculation graph. Both paths receive the same
source contract and produce the same canonical representation.

## Boundary and call graph

### `lower` (`lower.rs:51-95`)

```rust
pub fn lower(
    kernel: &PrimitiveKernel,
    tensors: &BTreeMap<ValueId, &Tensor>,
    hardware: LoweringHardware,
) -> LoweringResult<LoweredProgram>
```

The arguments have intentionally different ownership:

| Argument | Required facts | What lowering does with it |
| --- | --- | --- |
| `kernel` | A source kernel ID, ordered input/output value IDs, a complete alias matrix, and one `PrimitiveKind`. | Retains the ID and arities, copies alias permissions, and dispatches exactly one variant-specific routine. It is never mutated. |
| `tensors` | A `BTreeMap` containing every referenced value, each with validated dtype, shape, layout, and storage bytes. | Looks up each source value, creates one external `ProgramBuffer` per distinct `Tensor`, and supplies tensor metadata to the selected routine. |
| `hardware` | Measured subgroup width, maximum workgroup width, and nonzero shared-memory capacity. | Selects power-of-two workgroup and collective widths. It is copied into the private builder and does not become a target or driver object. |

The function is fail-fast and ordered:

1. `validate_hardware` rejects a zero or non-power-of-two subgroup, a
   workgroup maximum smaller than the subgroup, or zero shared memory. The
   error is `InvalidLoweredProgram`, tagged with `kernel.id`.
2. `PrimitiveKernel::validate(tensors)` resolves input and output tensors,
   checks the alias matrix, and validates the selected primitive's arity,
   dtype, shape, axis, and policy contract. A `LanguageError` is converted to
   `LoweringErrorKind::InvalidLanguage`, preserving its kernel and value
   context.
3. `ProgramBuilder::new` creates empty dense buffer and stage tables. Every
   input followed by every output is looked up and passed to `add_tensor`.
   Repeated value IDs are de-duplicated by the `tensor_buffers` map, so an
   aliased input/output has one program buffer rather than two.
4. The `PrimitiveKind` tag selects exactly one of
   `lower_elementwise`, `lower_reduce`, `lower_scan`, `lower_contraction`,
   `lower_gather`, `lower_scatter`, `lower_histogram`, `lower_sort`,
   `lower_index_map`, or `lower_random`.
5. `finish` copies the source alias matrix and input/output counts, aggregates
   program resources, computes the canonical digest, and validates the whole
   result. No partially built program escapes.

There is no default or fallback branch. An unsupported future language variant
would be a compiler error until the lowering dispatch and model validator are
updated together.

### External callers

`ops/src/primitive.rs::lower_recipe` checks that the descriptor's
`LoweringAvailability` is a direct primitive recipe and that the borrowed
kernel satisfies its operation-specific shape, dtype, axis, bounds, and alias
requirements. It then calls `recipe_primitives::lower`; a mismatch is an
operation error and never reaches this function. `ops::lower_index_map` uses
the same path with the explicit index-map recipe.

`planner/src/planner.rs::lower_programs` is the graph path. It derives one
common `LoweringHardware` from all available measured calculation devices,
calls `lower` once per graph node, calls `program.validate()` again, and
assigns a stage-scoped identity to every resulting `ProgramStage`. The planner
does not reinterpret `PrimitiveKind` or recalculate stage resources.

## Builder representation

### `StageDraft` and `push_stage!`

The private `StageDraft` is the construction-time tuple of geometry, ordered
bindings, fixed synchronization points, atomic contracts, optional fault
contract, exact `StageResourceBounds`, and one `StageKind`. The
`push_stage!` macro only constructs that tuple and delegates to
`ProgramBuilder::push_stage`; it contains no behavior of its own.

### `ProgramBuilder`

The builder owns:

| Field | Role |
| --- | --- |
| `kernel` | Error context for every builder failure. |
| `hardware` | Measured limits used by width selection. |
| `buffers` | Dense `Vec<ProgramBuffer>` indexed by `BufferId`. |
| `tensor_buffers` | Stable `ValueId -> BufferId` map for source tensors. |
| `stages` | Dense ordered `Vec<ProgramStage>`. |
| `scratch_ordinal` | Monotonic ordinal used with each `ScratchPurpose`. |
| `fault_buffer` | At most one shared I32 fault flag per lowered program. |

`workgroup_lanes(logical_lanes)` converts the logical count to `u32`, clamps
it to at least one and to `maximum_workgroup_lanes`, then rounds down to a
power of two. It is used by independent elementwise, gather, scatter,
histogram, sort, random, and index-map stages. Empty calculations are filtered
before this helper is used to create a stage; a registered zero-element tensor
can therefore produce a valid zero-stage program rather than a fake one-lane
dispatch.

`collective_lanes(logical_width, payload_words, declared_maximum)` additionally
accounts for shared memory. A lane's payload consumes
`max(payload_words * 4, 1)` bytes. The selected power of two is bounded by the
logical width, the primitive's declared tree maximum, the measured workgroup
maximum, and the number of lanes that fit in measured shared memory. Reduction
payloads use one word for a value and a second word when an index is tracked;
scans use one value word.

### Source and scratch buffers

`add_tensor` captures the tensor's complete affine view:

```text
ProgramBuffer {
    origin: Tensor(value_id),
    lifetime: ExternalValue,
    dtype: tensor.dtype,
    shape: tensor.shape.extents(),
    access: {
        logical_extents: tensor.shape.extents(),
        offset_elements: tensor.layout.offset_elements,
        strides: tensor.layout.strides,
        storage_bytes: tensor.storage_bytes,
    },
}
```

`scratch(dtype, elements, purpose)` checks `elements * dtype.byte_width()`
before allocating a dense buffer ID, gives it a one-dimensional shape and
unit stride, and records `ProgramScratch` lifetime. Scratch purposes are
`ReductionValues`, `ReductionIndices`, `ScanValues`, `ScanBlockTotals`,
`SortValues`, and `SortIndices`. The ordinal is checked for `u32` overflow.

`fault()` lazily allocates one I32 `[1]` buffer with four-byte storage and
`ProgramFaultFlag` lifetime. `checked_fault(reason, code)` reuses that buffer
and returns three coupled objects: a `FaultContract`, a `ReadWriteAtomic`
binding, and a release `Exchange` in the `SingleFaultFlag` domain. The fault
contract always says `guard_before_address: true`, meaning a rejected lane
publishes before forming an invalid payload pointer.

`buffer` converts a dense `BufferId` to a vector index and distinguishes an
arithmetic conversion failure from an absent buffer. `binding` copies the
buffer's dtype and complete static view into an ordered `BufferBinding` with
the requested `AccessMode`.

### Stage order and finish

`push_stage` assigns `StageId` equal to the current stage-vector length. Every
stage depends on the immediately preceding stage, and the first stage has no
dependencies. This is the canonical serialization used by validation and is
also the dependency chain consumed by the planner. Stage IDs and buffer IDs
are checked for `u32` conversion overflow.

`finish` computes `ProgramResourceBounds` before moving builder state into:

```text
LoweredProgram {
    schema_version: LOWERED_PROGRAM_SCHEMA_VERSION (= 2),
    source_kernel: kernel.id,
    source_input_count,
    source_output_count,
    source_aliases,
    buffers,
    stages,
    resources,
    digest,
}
```

The digest starts as `ProgramDigest::ZERO`, is recomputed over the complete
canonical contents, and is then checked by `LoweredProgram::validate`. A
validation failure is flattened into one `InvalidLoweredProgram` detail string
with the source kernel attached.

`aggregate_resources` sums stage FLOPs, integer operations, and atomic
operations with checked `u64` arithmetic. It takes maxima for shared bytes per
workgroup, private bytes per lane, and workgroup width. It separately sums
storage bytes of all `ProgramScratch` and `ProgramFaultFlag` buffers; external
tensor storage is not charged as program-owned storage. Any overflow is a
kernel-tagged `ArithmeticOverflow`.

`geometry(logical, lanes)` stores a one-dimensional dispatch with
`workgroups = ceil(logical / lanes)`. `basic_resources` only wraps the exact
counts supplied by a primitive routine in typed `FlopCount` and `ByteCount`
fields. The primitive routine, not the builder, is responsible for choosing
the counts.

## Elementwise scalar maps

`lower_elementwise` creates one `StageKind::ScalarMap` stage from the source
`Elementwise.program`.

1. A zero-element output returns with no stage. Otherwise the output extents
   become a `recipe_core::IndexSpace`; conversion of extents and index-space
   construction are mapped to `ArithmeticOverflow`.
2. Each source input becomes a `KernelInput` with one-based
   `KernelInputId`, its tensor dtype, and a `broadcast_access` view. Each
   output becomes a one-based `KernelOutput` carrying dtype, offset, strides,
   and storage bytes. The complete source alias matrix is translated to core
   `AliasRule` values and embedded in a `KernelTemplate` with the source
   scalar program.
3. The stage binds all input buffers as `Read` followed by all output buffers
   as `Write`. If `ScalarProgram::requires_fault_flag()` is true, it appends
   the shared checked fault binding and an arithmetic-domain fault code `2`.
   The scalar opcode set requests this flag for `Require`, I32 divide or
   remainder, and I32 negate or absolute operations. A scalar program that
   only uses ordinary arithmetic does not receive a fault argument.
4. Per-lane FLOPs are the checked sum of `ScalarOpcode::flops()`. Total FLOPs
   multiply that count by output elements. Private bytes are four bytes per
   scalar input, constant, and instruction slot. Integer operations are one
   per output element, and fault atomic operations are one per output element
   when a fault flag is present.
5. The stage geometry is output elements by the measured power-of-two
   workgroup width, and the `KernelTemplate` is retained verbatim in the
   `StageKind` payload.

`broadcast_access` right-aligns a lower-rank input to the output rank. A
singleton input extent maps to stride zero only when the corresponding output
extent is not one; otherwise its original tensor stride is retained. The
offset and storage bound are never changed. If an input rank exceeds the
output rank, the checked subtraction returns `InvalidStaticAccess` tagged with
the value ID. Shape and dtype compatibility were already established by
`PrimitiveKernel::validate`; this helper only builds the affine view used by
the scalar template and later backend emitter.

The scalar template is validated twice: first by `PrimitiveKernel::validate`
through `KernelTemplate::validate` semantics, and then as part of the complete
`LoweredProgram`. `recipe-kernel` reuses its existing scalar emitter for this
stage and rewrites the generic fault publication to the encoded code `2`.

## Reductions

`lower_reduce` lowers one fixed-tree reduction, potentially as several serial
`FixedTreeReduce` stages.

The input is `kernel.inputs[0]`; the reduced width is the checked product of
the extents named by `spec.axes`. The output element count comes from output
zero. A zero-element output returns no stage. When a reduced axis has zero
extent, `lower_empty_reduction` fills every nonempty output with the operator's
identity and emits no reduction tree. The identities are:

| Operator and dtype | Identity |
| --- | --- |
| `Sum` or `Any`, F32 | F32 bit pattern `0` |
| `Product` or `All`, F32 | F32 bit pattern for `1.0` |
| `Sum` or `Any`, I32 | I32 `0` |
| `Product` or `All`, I32 | I32 `1` |
| index output | I32 `0` |

Language validation rejects empty `Minimum` and `Maximum`; reaching that
combination in `lower_empty_reduction` is therefore an `InvalidLanguage`
error, not a guessed identity. A fill stage has one output binding, one
integer operation per logical output element, four private bytes per lane,
and no fault or synchronization contract.

For a nonempty reduction, `payload_words` is one for value-only results and
two for index-bearing results. `collective_lanes` fixes the shared-memory
payload width and `reduction_tree` creates one `TreeStep::Reduction` per
descending power-of-two stride. Every tree step receives a workgroup
`SharedAcquireRelease` synchronization point through `tree_synchronization`.

The pass-width sequence starts at `reduced_width` and repeatedly replaces it
with `ceil(current_width / lanes)` while the result is greater than one. Each
pass creates:

* a value scratch buffer for non-final value or value-and-index passes;
* an I32 index scratch buffer for non-final index-bearing passes;
* the final source output buffer for a value result;
* the final source output buffer for an index result; or
* both final output buffers for a value-and-index result.

Each stage binds the current value read, an optional current index read, the
selected value output write, and the selected index output write. Its
`ReductionStage` records pass number, operator, result, input and output
widths, reduced axes, fixed tree, operator-identity padding, and lowest-logical
index tie-breaking. Groups are `output_elements * output_width`, logical lanes
are `groups * lanes`, and each group performs `lanes - 1` combines. Value-and-
index doubles the FLOP multiplier. Shared bytes are
`lanes * payload_words * 4`; private bytes are `payload_words * 8`.

The source currently allocates non-final scratch outputs but keeps
`current_value` and `current_index` unchanged until the final pass assignment.
Consequently a multi-pass program's later stage bindings still name the
original input buffer rather than the preceding scratch output. This is the
literal behavior of `lower.rs`; the immutable stage metadata exposes the pass
widths and scratch buffers, while `LoweredProgram::validate` checks structure
and arithmetic but does not infer or repair those bindings. A caller must not
describe this representation as a verified hierarchical dataflow beyond what
the stage records actually encode.

## Scans

`lower_scan` implements a hierarchical fixed-tree scan along one input axis.
It returns no stage for a zero-element tensor or a zero-width scan axis. The
number of independent sequences is `input_elements / axis_width`. Collective
lanes are selected from the axis width, one value payload word, and the
declared tree maximum. `scan_tree` is the canonical Blelloch tree: all
upsweep strides in increasing order followed by all downsweep strides in
decreasing order. Every tree step receives a workgroup shared acquire/release
barrier.

For each width in the hierarchy, the routine emits a
`FixedTreeScanLocal` stage:

* level zero writes the user output; higher levels write `ScanValues` scratch;
* when more than one block remains, `ScanBlockTotals` scratch is written;
* `input_width`, `output_width = ceil(input_width / lanes)`, axis, sequence
  count, reverse flag, operator, dtype, and tree are retained;
* level zero uses `UserInclusive` or `UserExclusive { identity }`, while
  higher levels use `HierarchyExclusiveIdentity`;
* logical lanes are `sequences * blocks * lanes` and workgroups are
  `sequences * blocks`;
* resource bounds count `2 * (lanes - 1)` tree combines plus one combine per
  active element for user-inclusive mode, `lanes * 4` shared bytes, and twelve
  private bytes per lane.

`current_input` advances to block totals when they exist, otherwise to the
current output. After local levels, reverse `levels.windows(2)` pairs are
combined with `ScanUniformCombine` stages. A combine is omitted when
`sequences * (lower.width - lanes)` is zero. Otherwise it reads the upper
level and read-writes the lower level, uses ordinary workgroup geometry, and
records the lower level, operator, sequence count, block width, and reverse
flag. The kernel emitter recovers the scan axis for a uniform stage by finding
the unique same-level local stage in the complete program; this is why the
full program, not an isolated stage fragment, remains available downstream.

## Contractions

`lower_contraction` creates one `StageKind::TiledContraction` stage for the two
operand inputs and one output. A zero-element output produces no stage. The
contracted element count is the checked product of the left extents named by
`contract_axes`; total products are `output_elements * contracted_elements`,
and each product contributes two FLOPs. The stage binds left read, right read,
and output write views.

`contraction_physical_plan` chooses a deterministic output tile from measured
capacity:

1. `free_extent` is the checked product of left axes that are neither batch nor
   contract axes.
2. `right_extent` is `output_elements / max(free_extent, 1)`, with a minimum
   of one.
3. `maximum` is the measured power-of-two workgroup width for the output.
   `balanced` grows as a square-friendly power of two while it fits.
4. `left_capacity` and `right_capacity` are power-of-two capacities bounded by
   their logical extents and `maximum`. `output_x` and `output_y` grow within
   `maximum`, preserving the product as the final workgroup width.
5. `reduction` is the power-of-two portion of contracted elements bounded by
   the measured subgroup width.

The selected strategy is always `ContractionStrategy::Direct`: each lane
retains a complete ordered accumulator privately, so no cross-lane reduction,
shared storage, or barrier is emitted. The stage records
`canonical_contracted_order: true`, zero shared bytes, 24 private bytes per
lane, integer work equal to output elements, and workgroup lanes
`output_x * output_y`. The `Staged` model variant remains represented and
validated for schema completeness, but this lowering routine does not select
it. For a staged plan, validation would expect two synchronization points per
reduction tile; the direct plan has zero.

## Gather

`lower_gather` emits one `StageKind::Gather` stage for nonempty output. It
binds source tensor input zero as `Read`, I32 indices input one as `Read`, and
output zero as `Write`. `axis` and `bounds` are copied unchanged. `Reject`
allocates the shared fault flag with `IndexOutOfBounds` code `1`, appends its
atomic binding, and counts one fault atomic per output element. `Clamp` and
`Wrap` omit the fault binding and atomic. The stage uses ordinary output
geometry, one integer operation per output element, and sixteen private bytes
per lane.

## Scatter

Scatter has an explicit two-stage shape when the base is nonempty:

1. `CopyReason::ScatterBase` copies input zero's base tensor into output zero.
   It binds base read and output write, uses base-element geometry, and costs
   one integer operation and eight private bytes per base element.
2. The update stage binds indices input one read, updates input two read, and
   output zero with `Write` for `UniqueIndices` or `ReadWriteAtomic` for an
   atomic conflict policy.

An empty base skips the copy stage. An empty update tensor returns after the
copy, leaving a copy-only program when the base has elements. For
`ScatterConflict::Atomic`, the exact operation and ordering are converted to
owned atomic enums and the address domain is `TensorElements`. For
`UniqueIndices`, no payload atomic is declared. `Reject` bounds add fault code
`1` and a fault atomic; `Clamp` and `Wrap` do not. Atomic Add, Minimum, and
Maximum count one FLOP per update; Exchange and unique writes count none.
Atomic-operation count is one per update for the payload atomic plus one per
update for a checked fault. The update stage otherwise uses one integer
operation and sixteen private bytes per update.

The source base and update dtype and shape relationships are not recomputed
here. They were established by the language validator. This routine only
turns the already valid conflict and bounds policy into the stage ABI.

## Histograms

`lower_histogram` always emits a `HistogramClear` stage first. It writes the
entire output bin buffer with output-element geometry, one integer operation
per bin, and four private bytes per lane. Since language validation requires a
positive bin count, this stage has a nonzero logical domain even when the
input sample tensor is empty.

For a nonempty input, a second `HistogramAccumulate` stage is appended. I32
samples use `I32Direct` bin mapping; F32 samples use
`F32TruncateTowardZero`. The output is read-write atomic with an `Add` in the
`HistogramBins` domain and the exact requested ordering. Weighted histograms
add input one as a read binding; unweighted histograms have no weight binding.
Every accumulation stage has a checked fault contract with
`HistogramBinOutOfBounds` code `3`, regardless of whether the input is
weighted. Resources are one FLOP and one integer operation per input element,
two atomic operations per element (bin add plus fault publication), and
sixteen private bytes per lane. An empty input therefore produces only the
clear stage.

## Stable sort

`lower_sort` emits a deterministic least-power-of-two bitonic network for the
selected axis. Empty input or a zero-length sort axis produces no stage. For a
nonempty axis it computes:

```text
padded_axis_length = axis_length.next_power_of_two()
slices = input_elements / axis_length
scratch_elements = slices * padded_axis_length
```

It allocates `SortValues` scratch with the input dtype and `SortIndices` I32
scratch. One `StableSortInitialize` stage reads the input and writes both
scratch arrays. Its `SortNetwork` records axis, original and padded lengths,
slices, requested direction and stability, original-axis-index ascending tie
break, IEEE-754 total order, and padding after all valid elements.

For each merge level from one through `trailing_zeros(padded_axis_length)`,
and each compare distance in descending power-of-two order, it emits a
`StableSortCompareExchange` stage that read-writes both scratch arrays. The
network phase records `merge_width` and `compare_distance`; there are no extra
barriers because each stage is serially dependent on its predecessor. A final
`StableSortFinalize` stage reads scratch values and indices, writes output zero,
and writes output one when `emit_indices` is true. Initialize, compare, and
finalize resource private bounds are 16, 20, and 12 bytes per lane; compare
stages count one FLOP and one integer operation per comparison lane. The
padding and comparator metadata make equal values, NaNs, signed zero, and
invalid padded positions deterministic even when `requested_stable` is false.

## Random maps

`lower_random` emits one `StageKind::Philox4x32_10` write-only stage for a
nonempty output and no stage for an empty output. The source validator already
requires no inputs, one output, a distribution-compatible dtype, and exactly
ten rounds. The lowered contract nevertheless stores the requested key and
distribution plus the complete Recipe-owned constants:

```text
rounds       = 10
multiplier_0 = 0xd251_1f53
multiplier_1 = 0xcd9e_8d57
weyl_0       = 0x9e37_79b9
weyl_1       = 0xbb67_ae85
counter      = ElementLow, ElementHigh,
               IterationXorStreamLow, IterationXorStreamHigh
fold kernel ID into key = true
fold run ID into key    = true
uniform I32 mapping     = unbiased multiply-high with counter rejection
normal F32 mapping      = owned Box-Muller V1
```

The stage geometry is output elements by a measured power-of-two workgroup.
Scheduled FLOPs are `output_elements * philox_rounds * 4`; integer operations
are `output_elements * 20`; private bytes are 32 per lane. The loop iteration,
run ID, kernel ID, and linear element index become explicit launch inputs in
`recipe-kernel`; this module only fixes their counter/key role.

## Iteration-aware index maps

`lower_index_map` emits one write-only `StageKind::IndexMap` stage for a
nonempty I32 output. It has no source inputs, always allocates the arithmetic
fault flag with code `4`, and binds output zero followed by the fault binding.
The complete `IndexMap { start, element_step, iteration_step, modulus }` is
copied into the stage. Resources are zero FLOPs, nine integer operations per
output element (`INDEX_MAP_INTEGER_OPERATIONS_PER_LANE`), one atomic operation
per element for the fault channel, and 32 private bytes per lane.

The emitted kernel evaluates
`start + element_index * element_step + loop_iteration * iteration_step`
through checked signed-I64 intermediates. A present modulus is known positive
from language validation and selects Euclidean remainder before the checked
I32 store. The lowerer does not evaluate any element or iteration itself; it
only preserves the immutable formula and declares the fault ABI.

## Errors and checked arithmetic

`LoweringError` carries a machine-readable `LoweringErrorKind`, detail text,
and optional `KernelTemplateId` and `ValueId` context. The lowerer can return:

| Kind | Produced by |
| --- | --- |
| `InvalidLanguage` | `PrimitiveKernel::validate` failure, including source arity, dtype, shape, alias, axis, tree, bounds, and scalar-program errors; also the unreachable empty min/max identity case. |
| `MissingTensor` | Missing tensor lookup at the input/output registration boundary or a buffer lookup that has no registered tensor. |
| `ArithmeticOverflow` | Checked products, byte sizes, pass/group counts, resource sums, scalar-slot counts, ID conversions, or static index-space construction. |
| `InvalidStaticAccess` | Broadcast input rank greater than output rank. Other static view safety is checked by the final program validator. |
| `InvalidLoweredProgram` | Invalid measured hardware, an absent buffer, a missing final reduction output, or any complete-program validation failure. |

All dimension products and resource formulas use checked arithmetic unless the
operation is explicitly a saturating *count bound* (fault and atomic counts
use `saturating_mul` after the logical domain is known). `checked_mul` is the
small helper used by collective, scan, contraction, sort, random, and
index-map routines. It reports the subject and source kernel rather than
wrapping a static representation.

## Postconditions and validation boundary

Successful `lower` returns a program satisfying the independent validator in
`primitives/src/validate.rs`:

* schema version is `2`, source alias entries cover every input/output pair
  exactly once, and buffer and stage IDs are dense and ordered;
* tensor buffers are external values, scratch buffers are one-dimensional
  program scratch, and a fault buffer is exactly I32 `[1]` with four bytes;
* every static view has matching extent and stride rank, an address span that
  fits storage, and an injective non-atomic writable mapping;
* every stage has nonzero logical and workgroup widths, ceiling-consistent
  workgroup count, the immediate prior stage as its only dependency, valid
  buffer bindings, and no duplicate atomic contract;
* fault paths use a release I32 exchange on the one fault flag and require the
  guard-before-address invariant;
* scalar templates, reduction and scan trees, contraction tiles, gather and
  scatter fault policy, histogram fault policy, sort network phases, Philox
  constants, and index-map modulus all match their exact kind-specific
  contracts;
* stage synchronization counts and per-kind FLOP, integer, atomic, shared,
  private, and workgroup bounds equal the formulas derived from the stage
  metadata; and
* the program-level sums and maxima equal the stage/resource and owned-storage
  aggregates, and the digest equals `canonical_digest()`.

This validator is structural. It does not execute a stage, inspect a device,
or independently prove numerical output. The final target emitter repeats the
validation at realization and checks that its artifact build recipe has the
same stage geometry, ordered bindings, fault value, work bounds, and resource
envelope.

## Downstream role and end-to-end ownership

The planner consumes the lowered program without changing its semantics. For
each selected device it:

1. materializes each `ProgramBuffer` as an external value, scratch allocation,
   or init-resident fault flag;
2. allocates one loop `CalculationTask` and one deferred artifact build recipe
   per `ProgramStage`;
3. translates the stage's immediate dependency and read-after-write bindings
   into task dependencies, preserving the declared order;
4. copies dispatch geometry, work counters, fault flag, and resource bounds
   into the build contract, and computes a stage-scoped identity from the
   program digest, source kernel ID, and stage ordinal; and
5. attaches the source alias matrix to the invocation and adds fault readbacks
   after all checked stages in the device/iteration cohort.

`recipe-kernel::lower_stage` requires the complete program and exact build
recipe. `StageKind::ScalarMap` reuses the core scalar emitter; every other
kind dispatches to a Recipe-owned emitter for the encoded tree, tile, index
guard, atomic order, total-order comparator, or Philox contract. No target
backend may replace a stage with a vendor library or choose a different
algorithm. Native preparation, artifact compilation, allocation, and loading
are pre-loop concerns; the `LoweredProgram` itself contains only calculation
stages and their declared resources.

An empty tensor path is represented honestly: a primitive routine can register
external zero-element buffers and emit zero stages, while histogram clear or
scatter base-copy stages remain present whenever their nonempty output/base
domains require them. The planner and acceptance paths must therefore accept
zero-stage programs without inventing a fake dispatch.

The program digest authenticates every field that affects this contract,
including source alias permissions, tensor views, scratch purposes, stage
order, geometry, bindings, synchronization, atomics, faults, stage kinds, and
resource bounds. It is the immutable hand-off identity from primitive lowering
to planning and realization. There is no target artifact identity here because
target, ABI, toolchain, entry symbol, and native format do not exist until the
later kernel and preparation boundaries.
