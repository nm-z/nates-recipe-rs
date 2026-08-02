# `recipe-primitives` lowered-program model

This page documents `primitives/src/model.rs`, together with the code that
constructs, validates, hashes, and consumes its values. The word *model* here
means a backend-neutral primitive execution model. It is not the public
`recipe.model()` declaration or a serialized neural-network model.

## Boundary and purpose

`recipe-language::PrimitiveKernel` is a typed, placement-free calculation
declaration. `recipe-primitives::lower` turns one such declaration plus the
measured limits in [`LoweringHardware`] into one immutable [`LoweredProgram`].
The result fixes everything that must be identical on every target realization:

- the source kernel and its complete input/output alias matrix;
- typed tensor views and the lifetimes of external values, scratch, and fault
  storage;
- a one-dimensional dispatch geometry;
- ordered stages and their dependencies;
- stage bindings, barriers, atomic operations, and checked-fault behavior;
- the exact Recipe-owned algorithm contract for each primitive kind; and
- stage and program resource bounds, plus a canonical digest of all of it.

The crate stops before target realization. It does not select a device, driver,
LLVM target, ABI entry symbol, toolchain, or vendor library. The measured
hardware is used only to choose immutable geometry and collective widths. The
hardware values themselves are not fields in `LoweredProgram`; their effect is
represented by geometry and resource fields and is therefore covered by the
digest.

The model is intentionally data-only. Every public type derives `Clone`,
`Debug`, and equality traits appropriate to its shape. No type owns a runtime
handle or mutable execution state. The planner, kernel realization, native
preparation, and executor retain or borrow these values through the
`init -> loop -> exit` lifecycle.

## Source-to-runtime path

The real path is:

```text
CalculationGraph
  -> PrimitiveKernel::validate
  -> recipe_primitives::lower
  -> LoweredProgram::validate + canonical_digest
  -> planner stage/value/artifact contracts
  -> recipe_kernel::lower_stage (target realization)
  -> prepared artifact and scheduled loop task
  -> native executor
```

The operation-facing wrapper `recipe_ops::lower_primitive` first verifies that
the selected operation descriptor owns a direct primitive recipe, checks that
recipe against the supplied `PrimitiveKernel` and tensor index, and then calls
this crate's `lower`. `src/facade.rs` re-exports that wrapper as
`Recipe::engine::operations::lower_primitive`. The planner uses the lowerer
directly for every node in a validated `CalculationGraph`, because planning
needs every `StageKind`, including stages that are not represented by a legacy
operation symbol.

Empty shapes are valid language metadata and contain zero elements. A
zero-element primitive is an honest no-dispatch program where the lowerer can
prove that no primitive dispatch stage is needed. Graph-boundary initialization
is still a planner concern. A scatter may still need its base-copy stage when
the base is nonempty but its updates are empty, and a histogram always emits
its clear stage because its bin output is nonempty.

## Constants, identities, and small value types

### Schema and operation constants

| Name | Type | Value | Meaning |
| --- | --- | ---: | --- |
| `LOWERED_PROGRAM_SCHEMA_VERSION` | `u32` | `2` | Version required by validation and included in the digest. |
| `INDEX_MAP_INTEGER_OPERATIONS_PER_LANE` | `u64` | `9` | Exact integer-operation bound used by index-map stages and checked by validation. |

`LoweringHardware` is the measured input to geometry selection:

| Field | Type | Meaning |
| --- | --- | --- |
| `subgroup_lanes` | `u32` | Measured subgroup width. It must be nonzero and a power of two. |
| `maximum_workgroup_lanes` | `u32` | Measured upper bound for one workgroup. It must be at least `subgroup_lanes`. |
| `maximum_shared_memory_per_workgroup` | `ByteCount` | Measured shared-memory capacity. It must be nonzero. |

`lower` rejects any other hardware tuple as
`LoweringErrorKind::InvalidLoweredProgram`, annotated with the source kernel.
`ProgramBuilder::workgroup_lanes` clamps a logical width to the hardware limit
and rounds down to a power of two, with a minimum of one. Collective stages
use `collective_lanes`, which additionally limits the width by the declared
tree maximum and by the shared-memory capacity for their payload words.

### Stable IDs and digest

`ProgramDigest` is a private-field, 32-byte value with total ordering and
hashing. `ProgramDigest::ZERO` is the construction placeholder. `new` and
`bytes` are the only raw-byte conversions. Its `Debug` implementation prints
`ProgramDigest(` followed by six hexadecimal bytes and `...)`, so logs do not
dump the complete digest.

`BufferId` and `StageId` wrap `u32` and expose `new` and `get`. The lowerer
allocates both densely from zero. A buffer ID is its table index, and a stage ID
is its stage-table index. These identities are structural, not allocator
handles; validation rejects gaps, reordering, and references to absent slots.

## Buffer representation

### Origins and lifetimes

`BufferOrigin` explains where a program buffer comes from:

| Variant | Payload | Construction and role |
| --- | --- | --- |
| `Tensor(ValueId)` | Logical language value ID | Added once for every source input/output tensor encountered by the builder. |
| `Scratch { ordinal, purpose }` | Dense scratch ordinal and [`ScratchPurpose`] | Allocated for intermediate reduction, scan, or sort storage. The ordinal is unique within a program. |
| `FaultFlag` | None | One shared checked-fault flag per program. |

`ScratchPurpose` records why scratch exists: `ReductionValues`,
`ReductionIndices`, `ScanValues`, `ScanBlockTotals`, `SortValues`, or
`SortIndices`. It is descriptive but identity-bearing because it is included
in the digest and origin uniqueness key.

`BufferLifetime` is the allocation contract consumed by the planner:

- `ExternalValue` means a tensor value that comes from the graph boundary or
  is produced by a stage and may be copied between devices;
- `ProgramScratch` means persistent per-program storage, not a user artifact;
- `ProgramFaultFlag` means four-byte, preallocated device state initialized in
  the init image and read back after the loop.

The lowerer creates tensor buffers with `ExternalValue`, scratch buffers with
`ProgramScratch`, and the fault buffer with `ProgramFaultFlag`. Validation
rejects any other origin/lifetime pairing.

### `StaticAccess`

`StaticAccess` is a complete affine view into one typed backing allocation:

| Field | Type | Meaning |
| --- | --- | --- |
| `logical_extents` | `Vec<u64>` | Rank and logical extent of the view. For a canonical program buffer this is exactly `ProgramBuffer::shape`. |
| `offset_elements` | `u64` | First logical element's offset in the backing object. |
| `strides` | `Vec<u64>` | Element strides, one per logical axis. Zero strides are allowed for read-only broadcast views. |
| `storage_bytes` | `ByteCount` | Size of the complete backing object, not merely the logical payload. |

`validate_access` requires equal extent and stride ranks, checks the maximum
address calculation for `u64` overflow, and requires that the calculated byte
span fit in `storage_bytes`. For a writable non-atomic view, axes with extent
greater than one are sorted by stride and must be injective. A broadcast or
overlapping `Write` or `ReadWrite` view is therefore rejected. Atomic views are
allowed to overlap because their semantics are supplied by an explicit atomic
contract.

### `ProgramBuffer`

`ProgramBuffer` combines `id`, `origin`, `lifetime`, `dtype` (`F32` or `I32`),
`shape`, and `access`. The builder's tensor path copies language tensor shape,
layout offset, layout strides, and storage byte count. Scratch is always a
one-dimensional `[elements]` allocation with offset zero, stride one, and bytes
computed from the element count and dtype. The fault buffer is exactly an
`I32` `[1]` allocation with four bytes.

Validation additionally requires:

- a nonempty explicit rank (`shape` may contain a zero extent, but the vector
  itself cannot be empty);
- `shape == access.logical_extents`;
- dense, ordered IDs; and
- unique origin keys. Tensor origins are keyed by value ID, scratch origins by
  ordinal and purpose, and the fault origin has one reserved key.

### Bindings and access modes

`AccessMode` is `Read`, `Write`, `ReadWrite`, or `ReadWriteAtomic`.
`BufferBinding` identifies a `BufferId`, repeats the dtype for ABI checking,
stores the mode, and carries the exact `StaticAccess` view used by that stage.
The planner maps these modes one-for-one to `ArtifactBuildAccess`; the kernel
realizer checks the order, dtype, extents, offset, strides, and storage bytes
against the stage.

`ReadWriteAtomic` is distinct from ordinary `ReadWrite`: it permits overlapping
updates and must have a matching `AtomicContract`. A fault binding is normally
`ReadWriteAtomic`, while the stage's payload binding retains its own mode.

## Dispatch, synchronization, atomics, and faults

### Dispatch geometry

`DispatchGeometry` fixes a one-dimensional launch:

| Field | Meaning |
| --- | --- |
| `logical_lanes` | Number of logical elements or logical work items. |
| `workgroup_lanes` | Power-of-two physical lane width selected from measured limits. |
| `workgroups` | Exactly `ceil(logical_lanes / workgroup_lanes)`. |

Higher-rank addressing remains in each binding and stage contract. The launch
does not depend on a backend grid-rank limit. A stage must have nonzero logical
and workgroup widths.

### Synchronization

`SynchronizationPoint` records a barrier after a logical algorithm step using
`SynchronizationScope::Workgroup` or `DispatchBoundary` and
`MemorySemantics::SharedAcquireRelease` or `GlobalAcquireRelease`.
Current construction emits workgroup/shared-acquire-release points for fixed
reduction and scan trees. The model also represents the points required by a
staged contraction, although the current physical planner selects the direct
contraction strategy. Other current stages have no points. The model retains
both scope and memory variants so the exact contract remains explicit and
digest-covered; validation checks the required count for the selected
algorithm.

`ProgramBuilder::push_stage` gives every stage a dependency on the immediately
previous stage, or no dependency for stage zero. This serial chain is part of
the canonical model. The planner adds the additional value-producer
dependencies needed by bindings, but it does not replace the model's stage
chain.

### Atomic contracts

`AtomicAddressDomain` distinguishes `SingleFaultFlag`, `TensorElements`, and
`HistogramBins`. `OwnedAtomicOperation` is `Exchange`, `Add`, `Minimum`, or
`Maximum`; `OwnedAtomicOrdering` preserves all five language orderings:
`Relaxed`, `Acquire`, `Release`, `AcquireRelease`, and
`SequentiallyConsistent`. The `From` implementations are total, field-for-
field translations from `recipe-language::AtomicOperation` and
`AtomicOrdering`.

`AtomicContract` names the target buffer, dtype, operation, ordering, and
address domain. A stage cannot contain duplicate contracts. Every contract
must have a `ReadWriteAtomic` binding for the same buffer and dtype.

### Checked faults

`FaultReason` is `ArithmeticDomain`, `IndexOutOfBounds`, or
`HistogramBinOutOfBounds`. A `FaultContract` contains:

| Field | Meaning |
| --- | --- |
| `flag` | `BufferId` of the `I32` fault buffer. |
| `reason` | Domain that can reject a lane. |
| `code` | Exact `I32` code published by the stage. Current lowerers use `1` for index bounds, `2` for scalar arithmetic, `3` for histogram bins, and `4` for index-map arithmetic. |
| `guard_before_address` | Must be true. The rejecting branch publishes before forming or issuing the invalid payload address. |
| `publish` | Explicit release `Exchange` on the flag in the `SingleFaultFlag` domain. |

`ProgramBuilder::checked_fault` allocates or reuses the one fault buffer,
creates the release exchange, and returns the fault contract, its atomic
binding, and the atomic entry to add to the stage. Validation requires the
publication to target the declared flag, be an `I32` exchange in the fault
domain, and appear in `ProgramStage::atomics`. Kernel realization emits the
fault branch exactly and planner initialization places the flag in the device
init image.

## Algorithm contracts

### Trees and reductions

`TreePhase` has `Reduction`, `ScanUpsweep`, and `ScanDownsweep`. A `TreeStep`
stores the phase and stride. `FixedTree` stores a power-of-two lane count and
the complete ordered step list.

`reduction_tree(lanes)` emits descending strides
`lanes / 2, lanes / 4, ..., 1`. `scan_tree(lanes)` emits the Blelloch
upsweep strides `1, 2, ..., lanes / 2`, then downsweep in reverse order.
`tree_synchronization` emits one workgroup/shared-acquire-release point per
step.

`ReductionPadding::OperatorIdentity` says inactive lanes use the operator's
identity. `ReductionTieBreak::LowestLogicalIndex` says value/index reductions
retain the lowest source index for equal values. These are currently the only
variants and are checked as canonical.

`ReductionStage` records:

| Field | Meaning |
| --- | --- |
| `pass` | Hierarchical reduction pass, starting at zero. |
| `operator` / `result` | Language operator and `Value`, `Index`, or `ValueAndIndex` result contract. |
| `dtype` | Input/value dtype. Index payloads are `I32`. |
| `sequences` | Number of independent output sequences. |
| `input_width` / `output_width` | Width entering and leaving this fixed-tree pass. `output_width = ceil(input_width / tree.lanes)`. |
| `reduced_axes` | Original source axes, retained on every pass. |
| `tree` | Exact fixed reduction tree. |
| `padding` / `tie_break` | Identity padding and deterministic index tie policy. |

`lower_reduce` emits one stage per hierarchy pass. Intermediate values and,
when needed, indices use purpose-tagged scratch. The final pass writes the
declared output tensor(s). If a reduction has a zero output domain, it emits no
stage. If the reduced width is zero, language validation has already rejected
minimum and maximum, and the lowerer emits `Fill` stages using the sum/any
identity (`0`) or product/all identity (`1`), with an `I32` zero for a pure
index output. The current branch selects that `I32` zero only for
`ReduceResult::Index`; for `ValueAndIndex` it reuses the value identity for
both output tensors. `LoweredProgram::validate` does not compare a `Fill`
literal's dtype with its binding, so an empty `ValueAndIndex` reduction whose
value dtype is `F32` reaches kernel realization and is rejected there by the
fill emitter's exact literal-type check.

### Scans

`ScanStageMode` distinguishes user inclusive scans, user exclusive scans with
an explicit `ScalarLiteral` identity, and hierarchy levels that use the
operator identity. `ScanLocalStage` records hierarchy level, operator, dtype,
sequence count, input/output widths, source axis, reverse flag, mode, and the
fixed Blelloch tree. `ScanUniformStage` records the level, operator, dtype,
sequence count, lower-level width, block lane count, and reverse flag.

`lower_scan` creates local stages for each width in the hierarchy. Level zero
writes the declared output; higher levels write `ScanValues` scratch and may
write `ScanBlockTotals` scratch. It then emits `ScanUniformCombine` stages in
reverse hierarchy order for the active elements not covered by one local
block. Empty input or zero axis width produces no stages. The language layer
has already checked axis, dtype, tree width, and exclusive identity.

### Contractions

`ContractionTile` contains output-x, output-y, and reduction tile widths.
`ContractionStrategy::Direct` keeps each ordered accumulator private and needs
no workgroup barriers. `Staged` describes a strategy that checkpoints lane
accumulators in measured-capacity shared memory and synchronizes at fixed
barriers. The current `lower.rs` physical planner always chooses `Direct`, but
the model and validator fully represent and validate either strategy.

`ContractionStage` retains the language `Contraction` axis pairs, dtype,
output element count, contracted element count, strategy, tile, and the
`canonical_contracted_order` proof flag. Lowering sets that flag true and
visits contracted coordinates in row-major axis-pair order. The physical tile
is chosen from free extents and measured workgroup limits; the reduction tile
is a power of two bounded by the measured subgroup width. A zero output domain
is a no-dispatch program.

### Gather, scatter, and copy

`CopyReason::ScatterBase` identifies the preliminary copy that preserves the
scatter base tensor in the output before updates. Gather and scatter retain
the source axis and `IndexBounds` policy (`Reject`, `Clamp`, or `Wrap`).

`StageKind::Gather` binds source, index, and output buffers. `Reject` adds a
checked index fault; clamp and wrap do not. Scatter additionally retains the
language `ScatterConflict`: `UniqueIndices` uses an ordinary write, while
`Atomic` preserves the requested operation and ordering in both the stage kind
and its `AtomicContract`. A nonempty base gets a `Copy` stage before the update
stage; an empty update tensor stops after that copy. The lowerer counts one FLOP
per update for atomic add/minimum/maximum and none for exchange or unique
writes.

### Histograms

`HistogramBinMapping` is `I32Direct` or `F32TruncateTowardZero`.
`HistogramClear` has one output binding and always precedes accumulation.
`HistogramAccumulate` retains the bin count, weighted flag, input mapping, and
owned atomic ordering. It binds the input, output as `ReadWriteAtomic`, an
optional `F32` weight tensor, and the checked fault flag. Invalid bins always
use `HistogramBinOutOfBounds` and code `3`. An empty input still clears the
nonempty bin output but emits no accumulation stage.

### Stable sort

`FloatSortOrder::Ieee754TotalOrder`, `SortPadding::AfterAllValidElements`, and
`SortTieBreak::OriginalAxisIndexAscending` encode deterministic comparison
semantics. `SortNetwork` records axis, original axis length, least-power-of-two
padded length, number of slices, direction, the user's `requested_stable`
bit, tie-break, float ordering, and padding policy. `SortCompareStage` adds a
bitonic merge width and compare distance.

`lower_sort` allocates `SortValues` and `SortIndices` scratch for
`slices * padded_axis_length`, emits one initialize stage, every fixed
bitonic compare-exchange phase, and one finalize stage. Finalize optionally
writes original-axis indices. The validator requires nonzero axis length,
least-power-of-two padding, and the canonical tie/float/padding policies.
Zero-element input emits no sort stages.

### Index maps and Philox random maps

`PhiloxCounterWord` fixes the four counter positions:
`ElementLow`, `ElementHigh`, `IterationXorStreamLow`, and
`IterationXorStreamHigh`. `UniformI32Mapping` is
`UnbiasedMultiplyHighWithCounterRejection`; `NormalF32Mapping` is
`OwnedBoxMullerV1`.

`Philox10Contract` records the `RandomKey`, distribution, ten rounds, the two
Philox multipliers, two Weyl constants, counter layout, kernel/run-ID key
folding flags, and the two distribution mappings. The lowerer fixes the
constants `0xd2511f53`, `0xcd9e8d57`, `0x9e3779b9`, and `0xbb67ae85`, and
requires ten rounds through language validation and model validation. A random
stage has one output binding, no atomics, and uses the dynamic run ID only at
kernel realization.

`StageKind::IndexMap` stores the language `IndexMap` (`start`, element step,
iteration step, and optional positive modulus). It always has an arithmetic
fault flag, one output binding plus one fault binding, nine integer operations
per logical lane, and one fault atomic per lane. Both index-map and random
stages are omitted for zero-element outputs.

## `StageKind`, stage envelope, and resource accounting

`StageKind` is the complete finite algorithm vocabulary. Its variants and
payloads are:

| Variant | Payload |
| --- | --- |
| `ScalarMap` | A complete validated `recipe_core::KernelTemplate`, including scalar program, static views, and alias rules. |
| `Fill` | One `ScalarLiteral` identity or fill value. |
| `Copy` | A `CopyReason`, currently `ScatterBase`. |
| `FixedTreeReduce` | `ReductionStage`. |
| `FixedTreeScanLocal` | `ScanLocalStage`. |
| `ScanUniformCombine` | `ScanUniformStage`. |
| `TiledContraction` | `ContractionStage`. |
| `Gather` | Axis and bounds policy. |
| `Scatter` | Axis, bounds policy, and conflict policy. |
| `HistogramClear` | No payload. |
| `HistogramAccumulate` | Bin count, weighted bit, bin mapping, and owned atomic ordering. |
| `StableSortInitialize` | `SortNetwork`. |
| `StableSortCompareExchange` | `SortCompareStage`. |
| `StableSortFinalize` | `SortNetwork` and `emit_indices`. |
| `IndexMap` | Language `IndexMap`. |
| `Philox4x32_10` | `Philox10Contract`. |

`StageResourceBounds` is exact, not a hint:

| Field | Meaning |
| --- | --- |
| `flops` | Payload arithmetic count as `FlopCount`. |
| `integer_operations` | Addressing, control, and integer payload work bound. |
| `atomic_operations` | Exact bound for payload and fault atomics. |
| `shared_bytes_per_workgroup` | Shared storage required by this stage. |
| `private_bytes_per_lane` | Private storage required by one lane. |
| `maximum_workgroup_lanes` | Stage's physical width envelope. |

`ProgramStage` joins an ID and canonical previous-stage dependency with
geometry, ordered bindings, synchronization points, atomic contracts, an
optional fault, exact resources, and a `StageKind`. The ordered binding list
is the ABI source of truth. A stage with a fault includes the fault binding in
that list and in its atomic contracts.

The validator recomputes the following resource formulas from the kind and
geometry, then requires byte-for-byte equality with the stored bounds:

| Kind | FLOPs | Integer operations | Atomics | Shared | Private |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar map | scalar-instruction FLOPs per lane times logical lanes | logical lanes | fault-present times logical lanes | 0 | scalar input/constant/instruction slots times 4 |
| Fill | 0 | logical lanes | 0 | 0 | 4 |
| Copy | 0 | logical lanes | 0 | 0 | 8 |
| Tree reduction | tree combines times 1, or 2 for value plus index | logical lanes | 0 | tree lanes times payload words times 4 | payload words times 8 |
| Local scan | tree combines plus user-inclusive active elements | logical lanes | 0 | tree lanes times 4 | 12 |
| Uniform scan combine | logical lanes | logical lanes | 0 | 0 | 12 |
| Contraction | output elements times contracted elements times 2 | logical lanes | 0 | 0 for direct, lanes times dtype width for staged | 24 |
| Gather | 0 | logical lanes | fault-present times logical lanes | 0 | 16 |
| Scatter | one per add/min/max update | logical lanes | logical lanes times payload-atomic plus fault-atomic bits | 0 | 16 |
| Histogram clear | 0 | logical lanes | 0 | 0 | 4 |
| Histogram accumulate | logical lanes | logical lanes | two per lane (payload and fault) | 0 | 16 |
| Sort initialize | 0 | logical lanes | 0 | 0 | 16 |
| Sort compare-exchange | logical lanes | logical lanes | 0 | 0 | 20 |
| Sort finalize | 0 | logical lanes | 0 | 0 | 12 |
| Index map | 0 | logical lanes times 9 | logical lanes | 0 | 32 |
| Philox | logical lanes times rounds times 4 | logical lanes times 20 | 0 | 0 | 32 |

`ProgramResourceBounds` is the checked aggregate over all stages and buffers:

| Field | Aggregate |
| --- | --- |
| `total_flops`, `total_integer_operations`, `total_atomic_operations` | Checked sums over stage bounds. |
| `persistent_scratch_bytes` | Checked sum of `ProgramScratch` buffer storage. |
| `fault_bytes` | Checked sum of `ProgramFaultFlag` storage. |
| `peak_shared_bytes_per_workgroup`, `peak_private_bytes_per_lane` | Maximum over stages. |
| `maximum_workgroup_lanes` | Maximum over stages. |

External tensor storage is deliberately excluded from persistent scratch and
fault totals. Any arithmetic overflow while constructing or validating these
values is an error, not a saturating or fallback estimate.

## Construction in `lower.rs`

`lower(kernel, tensors, hardware)` performs this exact sequence:

1. Validate the measured hardware tuple.
2. Call `PrimitiveKernel::validate`, translating `LanguageError` into
   `LoweringErrorKind::InvalidLanguage` while preserving kernel and value
   context.
3. Create `ProgramBuilder` with the source kernel ID, hardware, empty buffer
   and stage tables, scratch ordinal zero, and no fault buffer.
4. Add every source input and output tensor. `add_tensor` deduplicates repeated
   `ValueId`s and preserves the language dtype, shape, layout, and storage.
5. Dispatch to the one lowerer matching `PrimitiveKind`: elementwise, reduce,
   scan, contraction, gather, scatter, histogram, sort, index map, or random.
6. Convert every language alias rule to `SourceAliasContract`, preserving input
   and output positions and permission, and pass the complete source arity to
   `ProgramBuilder::finish`.

The builder is the sole constructor for program tables in production code:

- `tensor_buffer` resolves an already-added tensor or returns
  `MissingTensor`;
- `scratch` checks element-to-byte multiplication, allocates a dense buffer
  ID, increments a checked scratch ordinal, and records its purpose;
- `fault` allocates one four-byte `I32` flag and reuses its ID thereafter;
- `binding` copies the buffer's dtype and canonical view into a stage binding;
- `push_stage` allocates a dense stage ID and links it to the immediately
  preceding stage; and
- `checked_mul`, ID conversions, and `overflow` turn every failed static
  arithmetic operation into `ArithmeticOverflow` with the source kernel.

`finish` first aggregates resources, creates a `LoweredProgram` with schema
version 2 and a zero digest, computes `canonical_digest`, writes that digest,
and calls `validate`. Any validation error is joined into one
`InvalidLoweredProgram` detail annotated with the source kernel. A successful
return is therefore a self-validating program with a non-placeholder digest.

## Per-kind lowering behavior

The following details describe the actual constructors, including when a
program intentionally has no stages.

### Scalar maps

`lower_elementwise` uses the output shape as a `recipe_core::IndexSpace`,
creates one-based core input/output IDs, broadcasts input strides into the
output rank, and clones the language scalar program into a
`KernelTemplate`. It translates the source alias matrix to core IDs. Input
bindings are `Read`, output bindings are `Write`. If the scalar program
contains a checked arithmetic instruction, `checked_fault` adds an arithmetic
fault with code `2`. The single `ScalarMap` stage's geometry is the output
element count, and its resource bounds are derived from scalar opcode FLOPs,
scalar slots, and the fault flag. A zero-element output emits no stage.

### Reductions

`lower_reduce` computes the product of reduced-axis extents and the output
element count. For a nonempty output and nonzero reduced width it chooses a
collective width from payload size, emits one `FixedTreeReduce` per hierarchy
pass, and uses scratch for every nonfinal value/index result. A final value,
index, or value-plus-index pass binds the declared output tensors. Stage
dependencies serialize passes. For an empty reduced domain it emits identity
`Fill` stages, subject to language rejection of empty minimum/maximum.

### Scans

`lower_scan` computes independent sequence count and axis width, chooses a
collective width for one value word, emits local fixed-tree levels, and then
uniform combine stages for active tails. Level zero writes the public output;
higher levels use `ScanValues` and `ScanBlockTotals`. User mode is retained only
at level zero, while hierarchy levels use the operator identity. An empty input
or zero-width axis has no dispatch.

### Contractions

`lower_contraction` computes contracted and output products, chooses a
physical output tile from measured workgroup limits, and emits one direct
`TiledContraction` stage with three bindings. It records the exact language
axis pairs and sets canonical contracted order true. No staged strategy is
selected by the current physical planner.

### Gather and scatter

Gather binds source, indices, and output, with a fault only for `Reject`.
Scatter first copies a nonempty base to the output, then binds indices,
updates, and output. Atomic conflicts add an owned payload atomic; rejected
indices add the checked fault atomic. The stage kind retains both the bounds
and original conflict ordering.

### Histograms

Histogram lowering emits a clear stage before inspecting input emptiness. For a
nonempty input it chooses direct `I32` or truncating `F32` bin mapping, adds the
weighted input when requested, adds the output add atomic, and always adds the
checked bin fault. Empty input therefore still clears bins but has no
accumulation dispatch.

### Sorts

Sort lowering computes least-power-of-two padding, allocates value/index
scratch, emits initialization, every compare-exchange pair in the fixed
bitonic network, and finalization. The network retains the user's direction
and stable bit, but the tie, IEEE total order, and valid-before-padding rules
are always Recipe-owned and digest-covered.

### Index maps and random maps

Index maps always bind output plus an arithmetic fault and use the fixed nine
integer operations per lane. Random maps bind one output and use the fixed
Philox10 constants and counter layout. Both return without stages for a
zero-element output.

## Validation and error behavior

`LoweredProgram::validate` is a pure, runtime-independent check. It collects
all discoverable failures into `Vec<ProgramValidationError>` rather than
stopping at the first one. Each error has a dotted/indexed `path` and a human
`detail`; its `Display` form is `path: detail`.

Validation proceeds in this order:

1. **Schema and digest.** The schema must be version 2, and the stored digest
   must equal a fresh canonical digest.
2. **Source aliases.** Every pair is in range, no pair is duplicated, and the
   vector has exactly `source_input_count * source_output_count` entries.
3. **Buffers.** IDs are dense, rank is explicit, shape and logical extents
   agree, static addresses fit, storage is sufficient, origins are unique, and
   origin/lifetime/dtype/shape combinations are legal. Fault storage is
   exactly four bytes.
4. **Stages.** IDs are dense, each dependency is exactly the prior stage, launch
   widths are nonzero, and workgroups equal the ceiling division.
5. **Bindings.** Every buffer exists at its dense ID, dtype matches, the view
   fits its storage, and writable non-atomic views are injective.
6. **Atomics and faults.** Contracts are unique, have matching atomic
   bindings and dtypes, and checked-fault publications satisfy the guard,
   target, domain, operation, dtype, and atomic-list requirements.
7. **Kind contracts.** Scalar templates, trees, output widths, contraction
   tiles, index bounds, sort phases, Philox constants, index-map modulus, and
   fixed binding arities are checked against their canonical values. This layer
   does not repeat every language-level shape/type relationship for every
   payload field; kernel realization performs the remaining exact ABI and
   emitter checks.
8. **Resources and synchronization.** Exact per-kind formulas and fixed tree
   or staged-contraction synchronization counts are recomputed. Program totals
   are independently summed or maximized with checked arithmetic.

The lowerer reports failures before a model exists as `LoweringError`:

| `LoweringErrorKind` | Actual sources |
| --- | --- |
| `InvalidLanguage` | `PrimitiveKernel::validate` failed; the original language detail, kernel, and value are retained. |
| `MissingTensor` | A source tensor is absent from the supplied index or a builder lookup has no program buffer. |
| `ArithmeticOverflow` | Static byte, extent, ID, stage, work, resource, or conversion arithmetic overflowed. |
| `InvalidStaticAccess` | Broadcast rank or another lowerer-created view cannot be represented. |
| `InvalidLoweredProgram` | Hardware limits, absent internal buffer, impossible stage construction, or final canonical validation failed. |

`LoweringError` stores the kind and detail plus optional `KernelTemplateId`
and `ValueId`. `Display` prints the kind and detail followed by any kernel and
value annotations. It implements `std::error::Error`; there is no retry,
substitute lowerer, or compatibility fallback.

## Canonical digest and identity

`canonical_digest` calls the crate-private `hash::program_digest`. The hash
starts with the domain separator
`recipe-lowered-primitive-program-v2\0`, then canonical little-endian fields
with length prefixes. `usize` values use a fixed 16-byte little-endian
representation, making collection lengths explicit. The digest covers:

- schema version, source kernel ID, source input/output arities, and every
  ordered source alias contract;
- every buffer ID, origin, scratch purpose, lifetime, dtype, shape, and static
  access field;
- every stage ID, dependency, geometry, ordered binding, synchronization point,
  atomic contract, optional fault, resource bound, and `StageKind` payload; and
- all program resource bounds.

Enum variants use explicit tags. Scalar opcode names use their stable `Debug`
spelling because `ScalarOpcode` is non-exhaustive; adding an opcode therefore
cannot silently collide with an existing tag. Floating literals use exact
`F32Bits`, not a host float conversion. The complete stage contract is hashed,
not just its operation name.

The digest is target-independent in representation, but it is not hardware
agnostic: changing measured limits can change geometry, scratch sizes,
resource bounds, and therefore the digest. It does not include target ABI,
toolchain, entry symbol, or realized artifact bytes. Those belong to later
planner and native-artifact identities.

The planner derives a stage-scoped `KernelTemplateId` from the program digest,
source kernel ID, and `StageId` under the namespace
`recipe-planner-stage-template-v1`. It uses that identity for scalar template
IDs, reserved artifact IDs, and stage placements. A collision is a planner
error. The native kernel verifier repeats the derivation and rejects any
artifact build whose source digest, stage ordinal, geometry, work, resources,
bindings, or fault binding differs from the complete lowered program.

## Planner, kernel, and lifecycle consumers

### Planner

`planner::lower_programs` computes one common measured `LoweringHardware`,
lowers every graph node, validates every returned program, derives each stage
identity, and retains the complete `LoweredProgram` in the planned candidate.
For each `ProgramBuffer`, `materialize_program_buffer`:

- resolves input tensor origins to resident/copy values;
- allocates output tensor and scratch values with their declared dtype and
  storage bytes; and
- resolves fault origins to the preallocated `I32` flag in the device init
  image.

For each `ProgramStage`, `lower_program_invocation` creates a loop
`CalculationTask`, translates buffer bindings to an `ArtifactBuildRecipe`,
retains the program digest and stage ordinal, and combines model dependencies
with ready-value dependencies. It also records fault cohorts for four-byte
metric readback after the relevant calculations. Source aliases become planner
alias invocations; `MustAliasExact` is checked against the typed static input
and output views before scheduling. External output copies are added after
stage lowering.

Zero-stage programs produce no calculation task or artifact. They remain in
the candidate as honest empty lowered programs, and zero-byte output readiness
uses the device init task rather than a fabricated writer.

### Kernel realization

`recipe_kernel::lower_stage` receives the complete `LoweredProgram`, not a
caller-selected fragment. Before emitting target LLVM it independently checks
program validation and digest, build validation, source kernel, stage ordinal,
stage-scoped identity, artifact identity, build-contract digest, geometry, work
bounds, resource envelope, binding order and views, and the exact fault
binding. It then dispatches on `StageKind`: scalar maps reuse the validated
`KernelTemplate`; every other variant has a Recipe-owned emitter. No emitter
chooses a different algorithm or repairs a stale contract.

The realized ABI follows the ordered stage bindings: readable pointers,
writable pointers, the optional fault pointer, a dynamic `RunId` for Philox,
and `element_count`. The run ID is runtime input and is never embedded in an
artifact. The model's fixed geometry and operation contract therefore survive
AMD and NVIDIA lowering unchanged.

### Preparation, execution, and remote transport

Native preparation finds the exact candidate program by source kernel and
program digest, calls `lower_stage`, and packages the resulting target image.
The planner's finalized bundle carries the artifact/build identities and the
scheduled init, loop, and exit tasks. Native executors consume the stage
artifacts and values through those tasks; they do not mutate `LoweredProgram`.

Remote provisioning and handshake transmit finalized plan and program
identities. A remote worker checks the provisioned program digest against its
worker projection before preparation and rejects mismatches. This is a later
wire-program identity, not a second primitive model or a replacement for the
canonical `ProgramDigest`.

## Architectural invariants

The model makes the following properties explicit and testable:

- only GPU payload calculations and transfers surround it; placement, queues,
  synchronization realization, and lifecycle tasks do not become new primitive
  kinds;
- source alias permissions are complete, ordered data, never inferred from
  pointer coincidence;
- all views, resource bounds, and algorithm choices are immutable before the
  loop;
- stage IDs, buffer IDs, dependency chains, binding order, and digests are
  stable identities, not incidental vector positions hidden from consumers;
- checked invalid-index and arithmetic paths guard before address formation,
  publish an explicit fault code, and are read back through a specialized
  transfer metric;
- reductions, scans, contractions, sorts, and random maps carry operation
  order or mapping details needed for backend-independent behavior;
- all static arithmetic is checked and failures remain visible; and
- validation is repeated at every trust boundary, so a digest or status from a
  caller cannot stand in for an independently derived end-state contract.

This is the complete role of `primitives/src/model.rs`: it is the immutable,
typed contract between language primitive intent and target-specific execution,
with enough identity, validation, and resource information for every later
stage to consume the same program without inference or fallback.
