# `recipe-primitives`

`recipe-primitives` is the backend-neutral AOT boundary for Recipe-owned tensor
primitives. It accepts one validated `recipe_language::PrimitiveKernel`, the
graph's tensor index, and measured lowering limits. It returns an immutable,
self-validating `LoweredProgram` that fixes the algorithm, typed memory views,
dispatch geometry, dependencies, synchronization, atomics, checked-fault ABI,
and exact static resource envelope for every emitted stage.

The crate stops before target realization. It does not choose an AMDGPU or
NVPTX target, call a driver, allocate a physical arena, compile LLVM, load a
native image, execute a lane, or select a vendor library. Those actions belong
to the planner, kernel, preparation, and executor crates. This boundary is
where Recipe owns primitive semantics and turns them into a complete contract
that later layers must realize exactly.

## Position in the pipeline

The complete data flow is:

```text
recipe-language::CalculationGraph
    tensors + PrimitiveKernel
             |
             |  recipe-ops validates a source-qualified recipe
             v
recipe-primitives::lower(kernel, tensor_index, measured_hardware)
             |
             v
LoweredProgram
    buffers, serial stage contracts, resources, source aliases, digest
             |
             +--> recipe-planner
             |       resident values, scratch and fault lifetimes,
             |       stage-scoped identities, calculation tasks, build recipes
             |
             +--> recipe-kernel
             |       exact contract verification and target LLVM emission
             |
             v
recipe-prepare -> realized HSACO or cubin -> finalized bundle
             |
             v
recipe-executor / recipe-native-executor
    immutable init -> loop -> exit lifecycle
```

The language graph remains the semantic source. A lowered program is not a
second graph or a runtime plan: it is the complete per-kernel physical
contract consumed by planning and realization. The planner turns its stages
into `TaskKind::Calculation` work and its fault flag into a four-byte metric
readback. Transfers, queue ownership, arena placement, and lifecycle phase
ordering remain planner and executor concerns.

## Manifest and module graph

[`Cargo.toml`](../Cargo.toml) declares package `recipe-primitives` version
`0.1.0`, Rust edition 2024, MIT licensing, and the description
`Deterministic AOT lowering of Recipe tensor primitives into GPU stage
programs`. Its only dependencies are `recipe-core` and `recipe-language`. The
crate forbids unsafe Rust and denies missing `Debug` implementations. It has no
feature flags, build script, native dependency, runtime, or filesystem path.

The public facade in [`src/lib.rs`](../src/lib.rs) keeps implementation modules
private and re-exports the error types, `lower`, and all model contracts:

```text
src/lib.rs
├── error.rs    LoweringError and ProgramValidationError
├── hash.rs     canonical domain-separated digest
├── lower.rs    PrimitiveKernel -> LoweredProgram construction
├── model.rs    public IDs, buffers, stages, contracts, and resources
└── validate.rs complete program invariant checker
```

[`INTEGRATION.md`](../INTEGRATION.md) records the planner and kernel boundary
in compact form. The source files are the authority for the actual behavior:

| Source | Responsibility |
| --- | --- |
| [`src/lib.rs`](../src/lib.rs) | Crate policy, re-exports, schema version, hardware limits, IDs, and model types. |
| [`src/model.rs`](../src/model.rs) | Buffer origins and lifetimes, static access, dispatch and synchronization, fault and atomic contracts, all `StageKind` payloads, and program/resource records. |
| [`src/lower.rs`](../src/lower.rs) | Entry point, measured-limit selection, buffer builder, every primitive-family lowerer, exact resource formulas, and stage sequencing. |
| [`src/validate.rs`](../src/validate.rs) | Complete structural, ABI, algorithm, synchronization, resource, alias, and digest validation. |
| [`src/hash.rs`](../src/hash.rs) | Canonical encoding and dependency-free SHA-256 implementation for `ProgramDigest`. |
| [`src/error.rs`](../src/error.rs) | Fail-closed lowering and validation error records. |

## The public lowering entry point

```rust
pub fn lower(
    kernel: &recipe_language::PrimitiveKernel,
    tensors: &BTreeMap<recipe_core::ValueId, &recipe_language::Tensor>,
    hardware: LoweringHardware,
) -> Result<LoweredProgram, LoweringError>
```

`lower` performs one fixed sequence:

1. `validate_hardware` rejects zero or non-power-of-two subgroup width, a
   maximum workgroup smaller than the subgroup, or zero shared memory.
2. `PrimitiveKernel::validate` resolves every input and output tensor and
   checks its arity, dtypes, shapes, axes, alias matrix, and primitive-specific
   language contract. A `LanguageError` becomes `LoweringErrorKind::InvalidLanguage`.
3. A `ProgramBuilder` adds every input and output tensor once. Each becomes an
   external `ProgramBuffer` with its exact dtype, shape, offset, strides, and
   backing byte size. A missing tensor index entry is a `MissingTensor` error.
4. The `PrimitiveKind` selects exactly one lowerer. Lowerers append zero or
   more immutable `ProgramStage` values and allocate only declared scratch or
   fault buffers.
5. Source alias rules are copied into `SourceAliasContract` records without
   changing their permission. The builder aggregates resources, computes the
   canonical digest, and calls `LoweredProgram::validate` before returning.

There is no alternate lowerer, retry, runtime fallback, or partially valid
return. Any checked arithmetic failure, missing reference, invalid access, or
post-build invariant failure returns an error with the source kernel and, when
available, the value ID attached.

## Hardware input and physical choices

`LoweringHardware` is a small measured capability record:

| Field | Use in lowering |
| --- | --- |
| `subgroup_lanes` | Upper bound for contraction reduction tiles. It must be a nonzero power of two. |
| `maximum_workgroup_lanes` | Upper bound for all launches and collective trees. It must be at least the subgroup width. |
| `maximum_shared_memory_per_workgroup` | Bounds collective payload words per lane. It must be nonzero. |

The planner derives one common record from available measured calculation
devices. It takes the maximum subgroup width, the minimum workgroup width, and
the minimum shared-memory limit across those devices. The primitive crate does
not probe or estimate these values.

For a pointwise launch, `workgroup_lanes` is the largest power of two no larger
than the logical element count and the measured maximum, with a minimum of one.
For a collective, `collective_lanes` additionally divides shared memory by
`payload_words * 4`, caps the declared tree width, and rounds down to a power
of two. This makes the selected tree and its floating-point operation order
independent of backend grid-rank or scheduling choices.

`DispatchGeometry` stores `logical_lanes`, `workgroup_lanes`, and
`workgroups = ceil(logical_lanes / workgroup_lanes)`. The launch is always
one-dimensional. Tensor rank remains in each static binding view, so a target
does not need to support a particular grid rank.

## Program representation

### Buffers and views

`BufferId` and `StageId` are opaque dense `u32` wrappers. `ProgramBuffer` names
one backing value:

* `BufferOrigin::Tensor(ValueId)` refers to an external graph tensor.
* `BufferOrigin::Scratch { ordinal, purpose }` is a program-owned persistent
  array for reduction values or indices, scan values or block totals, or sort
  values or indices.
* `BufferOrigin::FaultFlag` is the one shared four-byte `I32` rejection flag.

The matching lifetimes are `ExternalValue`, `ProgramScratch`, and
`ProgramFaultFlag`. Scratch allocations are one-dimensional, dense, and have
their exact checked byte count. The fault allocation is `[1]`, four bytes, and
is reused by every checked stage in the program.

`StaticAccess` is the complete affine view: logical extents, element offset,
element strides, and backing `storage_bytes`. `BufferBinding` repeats the
typed view at a stage boundary with one of four access modes:

```text
Read | Write | ReadWrite | ReadWriteAtomic
```

Read-only broadcasts may contain zero strides. Non-atomic writable views must
be injective. `ReadWriteAtomic` is the explicit exception for overlapping
updates. No binding may widen its storage or silently change dtype relative to
its buffer.

### Synchronization, atomics, and faults

`SynchronizationPoint` records the fixed step boundary, scope
(`Workgroup` or `DispatchBoundary`), and memory semantics
(`SharedAcquireRelease` or `GlobalAcquireRelease`). Current tree lowerers use
workgroup shared acquire-release barriers; ordinary pointwise, indexing,
histogram, sort, random, and copy stages have no synchronization points.

`AtomicContract` records the bound buffer, dtype, Recipe-owned operation
(`Exchange`, `Add`, `Minimum`, or `Maximum`), ordering, and address domain
(`SingleFaultFlag`, `TensorElements`, or `HistogramBins`). Every atomic contract
requires a matching `ReadWriteAtomic` binding.

`FaultContract` records the flag, semantic reason, exact integer code, and the
publication atomic. A rejecting lane must guard before forming a payload
address, publish a release `I32` exchange, and return without issuing the
invalid access. The current codes are arithmetic domain `2` for scalar maps,
index out of bounds `1` for gather and scatter, histogram bin out of bounds
`3`, and index-map arithmetic domain `4`.

### Stage and program records

`ProgramStage` combines the fixed geometry, ordered bindings, dependency IDs,
synchronization points, atomic contracts, optional fault contract, exact
`StageResourceBounds`, and one `StageKind`. `ProgramBuilder::push_stage`
assigns the next dense ID and gives every stage except the first a dependency on
the immediately preceding stage. This serial chain is the canonical stage
order, even when a later planner adds value-producer dependencies.

`LoweredProgram` retains:

* schema version `2`;
* source kernel ID and source input/output arity;
* the complete source alias matrix;
* ordered buffers and stages;
* aggregate `ProgramResourceBounds`; and
* the canonical `ProgramDigest`.

`ArtifactIdentity` is intentionally absent. Target, ABI, toolchain, entry
symbol, and realized resource identity do not exist at this layer. The planner
derives a stage-scoped identity from the lowered digest, source kernel, and
stage ordinal only after this contract is complete.

## Language boundary and preconditions

`recipe-language` owns the semantic primitive declaration. A
`PrimitiveKernel` has a stable kernel ID, ordered input and output `ValueId`s,
one `PrimitiveAliasRule` for every input/output pair, and a closed
`PrimitiveKind`:

```text
Elementwise | Reduce | Scan | Contraction | Gather | Scatter |
Histogram | Sort | IndexMap | Random
```

The graph validator checks tensor layout spans and backing bytes, unique tensor
and kernel IDs, one producer per non-input tensor, missing producers, and
cycles. The kernel validator then checks the following family contracts before
the primitive lowerer runs:

| Kind | Language checks that must already hold |
| --- | --- |
| `Elementwise` | Valid `ScalarProgram`, matching scalar/tensor input and output arity and dtypes, at least one input, and a broadcast-compatible output shape. |
| `Reduce` | One input, one or two outputs according to `ReduceResult`, a power-of-two tree width in `1..=1024`, valid axes, `Any` and `All` only on `I32`, index results only for `Minimum` or `Maximum`, reduced output shape, and no empty-domain min/max. |
| `Scan` | One input and output, an in-range axis, a valid tree width, `Any` and `All` only on `I32`, a correctly typed exclusive identity, and output shape and dtype equal to input. |
| `Contraction` | Two same-dtype inputs and one output, at least one contracted axis pair, no reused or out-of-range axes, matching pair extents, and the batch/free/contract output shape. |
| `Gather` | Data plus `I32` indices, one same-dtype output, and the shape produced by replacing the selected axis with the index shape. |
| `Scatter` | Base, `I32` indices, and same-dtype updates, one output matching the base, and update shape matching the base gathered along the selected axis. |
| `Histogram` | One input or an additional same-shaped `F32` weight input, one output, bins in `1..=i32::MAX`, `I32` unweighted or `F32` weighted payload, and output shape `[bins]`. |
| `Sort` | One input, one output or an additional `I32` index output, in-range axis, axis length representable by `I32`, and output shapes and value dtype matching input. |
| `IndexMap` | No inputs, one `I32` output, and an optional strictly positive modulus. |
| `Random` | No inputs, one output, exactly ten Philox rounds, valid distribution parameters, and output dtype implied by the distribution. |

`recipe-ops` is the source-qualified registry boundary. Its
`PrimitiveRecipe::matches` rejects a kernel with a wrong family, axis, bounds
policy, exclusive identity, sort mode, conflict operation, or alias rule before
calling this crate. Direct callers can invoke `lower` themselves, but the
language validation above still runs and no operation-registry policy is
silently inferred here.

## Lowering each primitive kind

The lowerers below are the only implementations selected by `lower`. Every
nonempty stage uses the tensor's exact static view. Empty logical work returns
an honest zero-stage program where the family permits it; it never fabricates a
one-lane dispatch.

### Scalar maps and fills

`Elementwise` lowers to `StageKind::ScalarMap { template }`. The output shape
becomes a `recipe_core::IndexSpace`. Each input gets a one-based
`KernelInputId`, a broadcast view that left-aligns ranks and changes singleton
expanded axes to zero stride, and a read binding. Each output gets a one-based
`KernelOutputId`, its original affine view, and a write binding. The complete
scalar program and translated alias rules are retained in the `KernelTemplate`
so the existing scalar emitter can be reused later.

The optional fault binding is present exactly when
`ScalarProgram::requires_fault_flag()` sees `Require`, checked integer divide
or remainder, or checked integer negate or absolute value. The stage uses
arithmetic fault code `2`. Its exact bounds are:

```text
flops             = scalar instruction FLOPs per lane * output elements
integer_operations = output elements
atomic_operations  = output elements when a fault flag is present, otherwise 0
private_bytes      = (scalar inputs + constants + instructions) * 4
shared_bytes       = 0
```

An output with zero elements emits no stage. Empty reduction domains use
`StageKind::Fill` instead. A fill writes the operator identity to one output
binding, uses integer-operation bound equal to its logical elements, and has a
four-byte private bound. Sum and Any use zero, while Product and All use one
for both supported dtypes. Language validation rejects empty Minimum and
Maximum because no implicit identity is defined.

### Fixed-tree reductions

`Reduce` computes the product of reduced input axes and emits one or more
`StageKind::FixedTreeReduce` passes. The output shape supplies the number of
independent sequences. A pass has `input_width`, `output_width =
ceil(input_width / lanes)`, pass number, reduced axes, operator, result mode,
identity padding, and lowest-logical-index tie breaking. Intermediate values and
indices are scratch buffers; the final pass writes the declared output tensor
or tensors.

The collective lane count is the power-of-two capacity allowed by the declared
tree width, measured workgroup and shared memory, and payload width. A value
reduction uses one four-byte payload word per lane; index and value/index
reductions use two. Each pass has one workgroup barrier for every fixed tree
step. If `groups = output_elements * output_width`, its bounds are:

```text
logical_lanes      = groups * lanes
workgroups         = groups
flops              = groups * (lanes - 1) * (1 or 2 for ValueAndIndex)
integer_operations = logical_lanes
shared_bytes       = lanes * payload_words * 4
private_bytes      = payload_words * 8
```

The pass chain stops when `output_width == 1`; all scratch dimensions and
checked multiplications are recorded, not recomputed by the planner.

If the reduced width is zero, `lower_empty_reduction` emits only the required
identity fills. If the output has zero elements, no dispatch is emitted.

### Hierarchical scans

`Scan` emits local `StageKind::FixedTreeScanLocal` stages followed, when needed,
by `StageKind::ScanUniformCombine` stages. For an axis width `W`, a local level
uses `blocks = ceil(current_width / lanes)`, `sequences = input_elements / W`,
and the canonical Blelloch upsweep then downsweep tree. Level zero preserves
the user mode, either inclusive or exclusive with its explicit identity.
Higher levels use the hierarchy-exclusive identity and write block totals to
scratch. The reverse flag and axis remain in each stage contract.

Local-stage bounds are:

```text
groups             = sequences * blocks
logical_lanes      = groups * lanes
flops              = groups * 2 * (lanes - 1) + active elements for user-inclusive mode
integer_operations = logical_lanes
shared_bytes       = lanes * 4
private_bytes      = 12
```

The reverse `levels.windows(2)` pass emits a uniform combine when the lower
level has elements beyond one block. It reads the upper-level totals and
updates the lower-level values in place, with geometry and integer/flop bounds
equal to its active elements and a twelve-byte private bound. Empty tensors and
zero-width axes produce no stage.

### Tiled contractions

`Contraction` computes the contracted extent as the product of the left
contracted axes, then emits one `StageKind::TiledContraction`. The physical
plan balances `output_x` and `output_y` powers of two against measured
workgroup capacity and chooses a power-of-two reduction tile no larger than the
measured subgroup. Contracted coordinates are declared in the ordered
row-major axis-pair order and `canonical_contracted_order` is set true.

The current lowerer always selects `ContractionStrategy::Direct`: each lane
keeps its complete ordered accumulator privately. It therefore emits no
shared-memory checkpoint or barrier. The staged strategy is represented in the
model and validation surface, but this implementation does not emit it. Bounds
are:

```text
logical_lanes      = output elements
flops              = output elements * contracted elements * 2
integer_operations = output elements
shared_bytes       = 0 for Direct
private_bytes      = 24
```

An empty output has no dispatch. Any checked extent or tile multiplication
overflow returns `ArithmeticOverflow`.

### Gather and scatter

`Gather` binds the data tensor read-only, index tensor read-only, and output
writable. `Reject` bounds allocate the shared fault flag, publish code `1`,
and add a release exchange atomic. Clamp and Wrap have no fault binding. For
`N` output elements, geometry is `N` with the measured pointwise workgroup,
integer work is `N`, atomic work is `N` only for Reject, and private storage is
16 bytes.

`Scatter` first emits a `CopyReason::ScatterBase` stage when the base tensor is
nonempty. That stage copies the base input to the output before any updates.
If updates are empty, this copy is the complete program. Otherwise the update
stage reads indices and updates and writes the output as `Write` for
`UniqueIndices` or `ReadWriteAtomic` for an explicit atomic conflict policy.
The atomic operation and all five language orderings are preserved in the
`AtomicContract`. Reject bounds add fault code `1` and a second exchange atomic.

For `N` update elements, integer work is `N`; atomic work is `N` for the
payload policy plus `N` for the fault path; Add, Minimum, and Maximum each add
one FLOP per update, while Exchange and UniqueIndices add none. Private storage
is 16 bytes. The copy stage has integer work equal to base elements and an
eight-byte private bound.

### Histogram

`Histogram` always clears its output with a `HistogramClear` stage. A nonempty
input then adds `HistogramAccumulate`, which maps `I32` values directly to bins
or truncates `F32` values toward zero. Weighted accumulation adds the optional
weight binding. The output uses an Add atomic with the requested ordering and
the `HistogramBins` address domain. Invalid bins guard before address formation
and publish fault code `3` through the shared `I32` flag.

For `N` input elements, accumulation geometry is `N`, FLOP and integer bounds
are both `N`, atomic bound is `2N` (bin update plus fault publication), and the
private bound is 16 bytes. Clearing uses one writable output binding, integer
work equal to the bin count, and a four-byte private bound. The language layer
ensures the bin count is nonzero and representable by `I32`.

### Stable sort

`Sort` allocates value and `I32` index scratch arrays of
`slices * padded_axis_length`, where the padded axis is exactly
`axis_length.next_power_of_two()`. `StableSortInitialize` copies input values
and original axis indices into those arrays. A fixed bitonic network then emits
one `StableSortCompareExchange` stage per merge width and compare distance.
`StableSortFinalize` writes values and, when requested, indices to the declared
outputs.

The `SortNetwork` records direction, requested stability, original-index
ascending tie break, IEEE-754 total order for f32 keys, and padding after all
valid elements. Compare phases require power-of-two merge widths and distances
with `distance < merge_width`. Initialization, compare, and finalization have
integer bounds equal to their active lanes, with private bounds of 16, 20, and
12 bytes respectively. No sort stage uses a fault flag or atomics. An empty
input emits no network.

### Index maps and counter-based random maps

`IndexMap` is a no-input, one-output stage. It always allocates the arithmetic
fault flag with code `4`. Its output is `I32`, geometry is the output element
count, integer work is `9` operations per lane, atomics are one exchange per
lane, and private storage is 32 bytes. The stage preserves `start`, element and
iteration steps, and the optional positive modulus. The later kernel emitter
evaluates the affine expression in checked signed-64 intermediates, applies
Euclidean remainder when a modulus is present, rejects an out-of-range int32
result otherwise, and stores only after the rejection branch. A nonzero
iteration step adds the dynamic loop-iteration ABI argument during realization.

`Random` emits `Philox4x32_10` with one output write binding and no input or
fault binding. Language validation requires `philox_rounds == 10`, and the
lowered contract fixes the Philox multipliers `0xd2511f53` and `0xcd9e8d57`,
Weyl advances `0x9e3779b9` and `0xbb67ae85`, element low/high counter words,
run-id XOR stream words, and folding of both source kernel ID and dynamic run
ID into the key. Distribution parameters are retained for UniformF32,
NormalF32, BernoulliI32, or UniformI32. Bounds are `4 * 10 * N` FLOPs,
`20N` integer operations, zero atomics and shared bytes, and 32 private bytes.
The run ID and loop iteration are bound only by target realization, not baked
into the artifact.

## Canonical validation

`LoweredProgram::validate` is pure: it consults no runtime state and returns a
vector of all `ProgramValidationError { path, detail }` failures. The pass is
organized as follows:

1. The schema is exactly `LOWERED_PROGRAM_SCHEMA_VERSION` (`2`).
2. The source alias list contains every input/output pair exactly once, with
   in-range indexes.
3. Buffers have dense ordered IDs, unique origins, explicit nonempty shapes,
   matching canonical extents and strides, checked address spans, sufficient
   storage, and consistent origin/lifetime/dtype/shape contracts. Writable
   non-atomic views must be injective. A fault buffer is exactly `I32 [1]` and
   four bytes.
4. Stages have dense IDs, the immediately preceding stage as their only
   canonical dependency, nonzero geometry, and the exact ceiling workgroup
   count. Every binding references an existing buffer with matching dtype and
   storage. Every atomic has a unique contract and a matching atomic binding.
5. Fault contracts must guard before addresses, publish their own flag through
   an `I32` release exchange in the stage atomic list, and use the single-fault
   address domain.
6. Kind-specific checks enforce scalar template validity, fixed reduction and
   scan trees, output widths, deterministic reduction ties, contraction tile
   products and order, Reject/fault equivalence, histogram fault reason, least
   power-of-two sort padding and total-order metadata, valid sort phases,
   exact Philox constants and counter layout, index-map arity and modulus, and
   fixed binding arities for fill, copy, uniform combine, and histogram clear.
7. `expected_stage_resources` recomputes every FLOP, integer, atomic, shared,
   private, and maximum-workgroup bound from the stage kind and geometry.
   Synchronization counts must equal the fixed tree or contraction algorithm.
8. Program totals recompute stage sums, peak shared/private values, maximum
   workgroup width, scratch bytes, and fault bytes exactly.
9. The stored digest must equal a fresh canonical digest.

This validation is repeated by the planner and independently by
`recipe-kernel::lower_stage`; a stale or mutated build contract cannot be
blessed by a nearby compatible-looking stage.

## Digest and identity

`ProgramDigest` is the 32-byte SHA-256 of a canonical encoding prefixed by the
domain `recipe-lowered-primitive-program-v2`. The encoding includes the schema,
source kernel and arity, every alias permission, every buffer field, every
stage dependency, geometry, binding, synchronization point, atomic and fault
contract, exact resources, every `StageKind` parameter, and aggregate program
resources. Sequences carry explicit lengths; integers use fixed little-endian
forms; enum variants use explicit tags; scalar opcodes use their stable debug
spelling because the core opcode enum is non-exhaustive.

The implementation in `hash.rs` is dependency-free and includes its own
padding, schedule, and 64-round SHA-256 compression. `ProgramDigest::Debug`
prints only the first six bytes. `canonical_digest()` recomputes without
mutating the program, and `finish` sets the stored digest before validation.
Any semantic or structural change therefore changes the digest and all
downstream stage-scoped identities.

## Downstream callers and lifecycle role

### `recipe-ops`

`recipe_ops::lower_primitive` accepts an `OperationDescriptor`, checks that its
`LoweringAvailability` is the matching `PrimitiveRecipe`, checks the borrowed
kernel against that source-qualified recipe, and calls this crate. A mismatch
is `PrimitiveRecipeMismatch`; a lowering error is wrapped as
`PrimitiveLoweringFailed`. `lower_index_map` uses the same bridge for the
Recipe-owned index-map declaration without inventing a legacy operation symbol.

### `recipe-planner`

`planner::lower_programs` derives common measured hardware, calls `lower` for
every graph node, and validates every result again. For each stage it derives a
collision-checked stage-scoped `KernelTemplateId` from the program digest,
source kernel, and stage ordinal. Scalar-map templates receive that identity;
all stages receive one reserved artifact/build identity.

During `lower_program_invocation`, the planner:

* materializes external tensor values, scratch values, and the pre-loop fault
  flag into the selected device's value table;
* creates one loop-domain calculation submission per nonempty stage;
* translates the stage's serial dependency and read-after-write bindings into
  task dependencies;
* copies binding views and access modes into an `ArtifactBuildRecipe`;
* records exact stage work and resource bounds, source/program digest, fault
  binding, and stage ordinal;
* emits a `TaskKind::Calculation` with the stage-scoped artifact and template;
* tracks all checked stages in a device/domain fault cohort; and
* appends one four-byte fault readback metric after that cohort.

Source aliases become planner alias invocations. A `MustAliasExact` rule is
checked against the same dtype and static view before scheduling; the planner
then applies the alias permission to the physical value bindings. No physical
arena offset is selected by this crate or by the primitive task itself.

An honest zero-stage program produces no calculation task or artifact. The
planner can still carry zero-byte output metadata through initialization, but
it never fabricates a dispatch to make an empty tensor appear executed.

### `recipe-kernel` and `recipe-prepare`

Preparation groups deferred builds by measured target and finds the exact
lowered program by source kernel and digest. It calls
`recipe_kernel::lower_stage` with the stage build recipe and a target-specific
entry symbol. Kernel realization independently verifies program schema and
digest, build provenance and contract digest, stage identity, geometry, work,
resources, binding order and views, and the exact fault binding.

`recipe-kernel` reuses the existing scalar `KernelTemplate` emitter for
`ScalarMap`. Every other `StageKind` has a Recipe-owned LLVM emitter for fill,
copy, reduction, scan, contraction, gather, scatter, histogram, sort, index
map, and Philox. The generated module contains only LLVM intrinsics and direct
Recipe arithmetic. HIP, the CUDA Runtime API, and vendor math libraries are
outside this contract. AMDGPU and NVPTX realization must preserve the stage's
algorithm, access views, operation order, atomics, and dynamic run/iteration
ABI.

### Executors

After preparation, the finalized bundle contains realized native images,
resolved values, queues, transfers, and the immutable lifecycle. Executors do
not call `recipe-primitives::lower` or reconstruct a stage. They bind finalized
arena pointers, the optional fault flag, dynamic run ID and loop iteration, and
element count to the realized kernel during `init -> loop -> exit`. Discovery,
lowering, compilation, allocation, and native-image loading are all pre-loop
preparation, not model work in the loop.

The root facade re-exports `recipe_primitives` as `recipe::engine::primitives`
and re-exports `LoweredProgram` and `LoweringHardware` through
`recipe::operations`. Those are inspection and integration surfaces, not a
second execution path.

## Failure paths

The public error vocabulary is intentionally small and fail closed:

| Error | Produced when |
| --- | --- |
| `InvalidLanguage` | The tensor index, graph primitive, alias matrix, shape, dtype, axis, scalar program, or distribution violates `recipe-language`. The original kernel/value context is retained when supplied by the language error. |
| `MissingTensor` | A kernel input or output is absent from the supplied `BTreeMap`, or a builder lookup has no program buffer. |
| `ArithmeticOverflow` | A checked extent, byte count, pass count, scratch ordinal, dense ID, resource aggregate, or synchronization conversion cannot fit its static integer representation. |
| `InvalidStaticAccess` | Broadcast rank or affine view metadata cannot form a valid static access. |
| `InvalidLoweredProgram` | Measured limits are invalid, a buffer/stage lookup is inconsistent, or the finished program fails canonical validation. |

`LoweringError` carries a detail string plus optional `KernelTemplateId` and
`ValueId`; `Display` prints the kind, detail, and available context. It does
not hide the original failure behind a retry or substitute implementation.
`ProgramValidationError` carries a precise dotted or indexed path, allowing
the planner and kernel boundaries to report every mismatch in one pass.

## Invariants that define the boundary

* Payload calculation is only `F32` and `I32`, both four bytes. Shapes, views,
  offsets, and strides are immutable metadata and are never inferred later.
* Every emitted stage has fixed one-dimensional geometry, explicit bindings,
  exact dependencies, and exact operation/resource bounds.
* Collective trees are power-of-two and canonical. Reductions pad with the
  operator identity and use the lowest logical index for equal extrema. Scans
  use the fixed Blelloch upsweep/downsweep order. Contractions preserve ordered
  contracted coordinates and currently keep accumulators private.
* Checked rejection branches publish one shared `I32` fault flag before any
  invalid payload address. Fault readback is an ordinary four-byte device
  transfer scheduled by the planner, not a third model-work kind.
* Atomic operation and ordering are explicit in the stage contract. Overlap is
  permitted only through `ReadWriteAtomic`.
* Sort padding, total-order floating comparison, and original-index tie breaks
  are part of the digest. Philox constants, counter words, key folding, and
  distribution mappings are part of the digest.
* Scratch and fault storage are counted exactly. External tensor storage is not
  charged as program scratch. Stage resources sum with checked arithmetic and
  peaks are computed, not guessed.
* Alias rules are complete and source-scoped. Target artifact identity is not
  fabricated before target realization.
* A returned program has already passed its own validation and digest check.
  Downstream callers still revalidate because they are independent contract
  boundaries.

The result is a narrow, deterministic handoff: language owns what a primitive
means, `recipe-primitives` owns the complete target-independent stage contract,
the planner owns where and when it runs, and kernel/preparation owns how that
contract becomes an inspected native image.
