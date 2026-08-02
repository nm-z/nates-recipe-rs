# `recipe-primitives`

`recipe-primitives` is Recipe's backend-neutral primitive compiler. It turns one
validated `recipe_language::PrimitiveKernel`, a tensor index, and measured
hardware limits into an immutable `LoweredProgram`. The result describes the
complete dispatch contract for a primitive, including buffers, affine views,
stage ordering, synchronization, atomic operations, checked-fault behavior,
and exact resource bounds. It does not choose a GPU, driver API, LLVM target,
vendor library, ABI, or native artifact. Target realization belongs to
`recipe-kernel` and the native preparation crates.

The package depends only on `recipe-core` and `recipe-language`. It forbids
unsafe code and denies missing `Debug` implementations. The public API is
intentionally the model and the single lowering entry point; implementation
modules remain private so callers cannot bypass construction and validation.

## Crate root and module graph

`primitives/src/lib.rs` declares five private modules:

| Module | Responsibility | Root export |
| --- | --- | --- |
| `error` | Lowering and post-lowering validation error values | `LoweringError`, `LoweringErrorKind`, `LoweringResult`, `ProgramValidationError` |
| `hash` | Domain-separated canonical encoding and SHA-256 digest calculation | reached by `LoweredProgram::canonical_digest` |
| `lower` | Hardware checks, `ProgramBuilder`, and one lowerer for every `PrimitiveKind` | `lower` |
| `model` | Public immutable program schema and resource contracts | `pub use model::*` |
| `validate` | Structural, algorithmic, resource, synchronization, and digest checks | reached by `LoweredProgram::validate` |

The root re-exports the complete model with `pub use model::*`, the
`lower(kernel, tensors, hardware)` function, and the four error names above.
There are no public constructors that can create a partially lowered program:
the builder is private, and `lower` calls both canonical digesting and
validation before returning.

## End-to-end position

The production path is:

1. `recipe-language` constructs a placement-free `PrimitiveKernel` and validates
   its tensor references, alias matrix, shapes, dtypes, axes, and primitive
   parameters.
2. `recipe-planner` derives one common `LoweringHardware` from available
   measured calculation devices. It takes the maximum observed subgroup width,
   the minimum workgroup width, and the minimum shared-memory capacity, then
   rejects the set if the subgroup cannot fit every workgroup. It calls
   `recipe_primitives::lower` for every calculation node.
3. `lower` rechecks hardware and language validity, registers source tensors as
   external buffers, dispatches to the `PrimitiveKind` lowerer, allocates only
   declared scratch or fault buffers, and appends stages in dependency order.
4. `ProgramBuilder::finish` aggregates exact resources, computes the canonical
   digest, and rejects any invalid program. The planner validates once more,
   derives a stage-scoped `KernelTemplateId` from `(program digest, source
   kernel, stage ordinal)`, and stores the program in the planned candidate.
5. During planner materialization, every `ProgramBuffer` becomes a resident
   value or an init-owned fault flag. Every `ProgramStage` becomes one loop
   `TaskKind::Calculation`; stage dependencies, read producers, and writes are
   translated into task dependencies and value producers. Fault-bearing stages
   are grouped by device and iteration domain, then receive one metric-style
   int32 fault readback after their calculations. Alias contracts are carried
   into the planned graph and exact must-alias views are checked.
6. `recipe-kernel` consumes the complete program and a planner build recipe.
   It validates digest, stage identity, geometry, bindings, work, resources, and
   fault ABI before emitting LLVM IR. Scalar-map stages reuse the core
   `KernelTemplate` emitter; other stage kinds use Recipe-owned emitters. Native
   preparation then compiles and loads the target-specific image. No target
   details are fabricated by this crate.

Advanced callers reach the crate through `recipe::engine::primitives`. The
normal operation facade calls `recipe_ops::lower_primitive`, which first
selects and matches a registry recipe and then calls this crate. The planner is
the direct production caller of `recipe_primitives::lower`; there are no other
lowering implementations or fallback paths.

## Public constants and scalar identifiers

`LOWERED_PROGRAM_SCHEMA_VERSION` is `2`. It is serialized and checked on every
program. `INDEX_MAP_INTEGER_OPERATIONS_PER_LANE` is `9`, the fixed accounting
factor used by index-map stages and validation.

`ProgramDigest` wraps the 32-byte SHA-256 result. `ZERO` is the temporary
builder value, `new` and `bytes` are const accessors, and its `Debug` output
prints the first six bytes followed by an ellipsis. `BufferId` and `StageId`
wrap dense `u32` table indexes and expose const `new` and `get` methods. The
builder assigns each ID from the current vector length, so validation requires
IDs to be dense and ordered.

## Public model

All model types derive the traits shown in the source and are re-exported at
the crate root. Their fields are the serialized contract, not hints for a
backend implementation.

### Hardware, buffers, and views

| Type | Contract |
| --- | --- |
| `LoweringHardware` | Measured `subgroup_lanes`, `maximum_workgroup_lanes`, and `maximum_shared_memory_per_workgroup`. Subgroups must be nonzero powers of two, workgroups must contain them, and shared memory must be nonzero. |
| `BufferOrigin` | `Tensor(ValueId)` for an external graph value, `Scratch { ordinal, purpose }` for compiler-owned storage, or `FaultFlag` for the one shared int32 fault channel. |
| `ScratchPurpose` | `ReductionValues`, `ReductionIndices`, `ScanValues`, `ScanBlockTotals`, `SortValues`, or `SortIndices`. The purpose is part of the digest and prevents indistinguishable scratch allocations. |
| `BufferLifetime` | `ExternalValue`, `ProgramScratch`, or `ProgramFaultFlag`. Origin, lifetime, dtype, and shape must agree. |
| `StaticAccess` | Complete affine view: `logical_extents`, `offset_elements`, `strides`, and `storage_bytes`. It is immutable and describes the whole addressable view. |
| `ProgramBuffer` | Dense `id`, origin, lifetime, dtype, canonical `shape`, and `access`. Tensor buffers retain source `ValueId`; scratch buffers are one-dimensional. The fault buffer is exactly one `I32` element and four bytes. |
| `AccessMode` | `Read`, `Write`, `ReadWrite`, or `ReadWriteAtomic`. Non-atomic writable views must be injective, so broadcasts and overlapping writes cannot be emitted. |
| `BufferBinding` | A buffer ID, matching dtype, access mode, and stage-local static view. |
| `DispatchGeometry` | One-dimensional `logical_lanes`, power-of-two `workgroup_lanes`, and `workgroups = ceil(logical_lanes / workgroup_lanes)`. Tensor rank stays in the bindings. |

`ProgramBuilder::workgroup_lanes` selects the largest power of two no larger
than the logical width or measured workgroup limit. Collective operations use
`collective_lanes`, which additionally limits lanes by declared tree width and
the shared-memory bytes needed for their payload words.

### Synchronization, atomics, and faults

`SynchronizationScope` is `Workgroup` or `DispatchBoundary`;
`MemorySemantics` is `SharedAcquireRelease` or `GlobalAcquireRelease`; and
`SynchronizationPoint` records the step after which that scope and memory
ordering apply. Fixed reduction and scan trees produce one workgroup
acquire-release point per tree step. The current direct contraction strategy
requires no tree barriers.

`AtomicAddressDomain` distinguishes `SingleFaultFlag`, `TensorElements`, and
`HistogramBins`. `OwnedAtomicOperation` owns exchange, add, minimum, and
maximum; `OwnedAtomicOrdering` owns relaxed, acquire, release,
acquire-release, and sequentially consistent orderings. Their `From`
implementations copy the corresponding language enums without changing the
semantic choice. `AtomicContract` binds an operation and ordering to one typed
buffer and address domain. Validation requires every atomic contract to have a
matching `ReadWriteAtomic` binding with the same dtype and forbids duplicate
contracts.

`FaultReason` is `ArithmeticDomain`, `IndexOutOfBounds`, or
`HistogramBinOutOfBounds`. `FaultContract` records the flag buffer, an int32
code, the reason, a `guard_before_address` requirement, and the publishing
`AtomicContract`. The builder always publishes with an int32 release exchange
to the single fault flag. A rejected lane must publish before returning and
must not form the invalid payload address. Gather and scatter request this path
only for `IndexBounds::Reject`; histogram accumulation always has a checked bin
fault; index maps and faulting scalar programs use their corresponding reasons.

### Fixed algorithms

`TreePhase` has `Reduction`, `ScanUpsweep`, and `ScanDownsweep`; `TreeStep`
stores a phase and stride; and `FixedTree` stores the lane count and exact
steps. Reduction trees are descending-stride binary trees. Scan trees are a
Blelloch upsweep followed by downsweep. Validation permits only powers of two
from 1 through 1024 and compares the complete step sequence with the canonical
one.

`ReductionPadding::OperatorIdentity` records padding with the operator's
identity. `ReductionTieBreak::LowestLogicalIndex` fixes deterministic argmin
and argmax ties. `ReductionStage` carries pass number, operator, result kind,
dtype, sequence count, input and output widths, reduced axes, tree, padding,
and tie-break. Multi-pass reductions write intermediate value and optional
index scratch buffers, then write the requested output on the final pass. Zero
reduced width uses identity `Fill` stages where the language permits it; empty
minimum and maximum are rejected as invalid language state.

`ScanStageMode` distinguishes user inclusive, user exclusive with an explicit
identity, and hierarchy-exclusive identity. `ScanLocalStage` records level,
operator, dtype, sequence count, input/output widths, axis, reverse flag, mode,
and tree. `ScanUniformStage` records level, operator, dtype, sequence count,
width, block lanes, and reverse flag. Large scans are lowered to local tree
stages plus reverse-order uniform-combine stages over block totals.

`ContractionTile` contains output-x, output-y, and reduction widths.
`ContractionStrategy` is `Direct` or `Staged`; direct keeps one complete
ordered accumulator per lane, while staged reserves measured-capacity shared
storage and fixed barriers. `ContractionStage` retains the language axis pairs,
dtype, output and contracted element counts, strategy, tile, and a mandatory
`canonical_contracted_order`. The current planner chooses `Direct`, and the
lowerer visits contracted coordinates in row-major axis-pair order.

### Indexing, histogram, sort, and random contracts

`CopyReason::ScatterBase` identifies the preliminary copy that preserves a
scatter base tensor before updates. `HistogramBinMapping` is either direct
int32 indexing or f32 truncation toward zero. `FloatSortOrder` is the owned
IEEE-754 total order. `SortPadding::AfterAllValidElements` places padded keys
after valid values, and `SortTieBreak::OriginalAxisIndexAscending` makes equal
keys deterministic.

`SortNetwork` records axis, original and padded lengths, slice count, direction,
whether stability was requested, tie-break, float order, and padding.
`SortCompareStage` adds bitonic merge width and compare distance. Sort lowering
uses the least power-of-two padded axis, allocates value and int32-index scratch,
emits initialization, one compare-exchange stage per network phase, and final
output, optionally including indices. Validation rejects any nonminimal or
noncanonical network phase.

`PhiloxCounterWord` fixes the four counter words to element low/high and
iteration-xor-stream low/high. `UniformI32Mapping` owns unbiased multiply-high
with counter rejection; `NormalF32Mapping` owns Box-Muller version 1.
`Philox10Contract` records the language key and distribution, exactly ten
rounds, the two Philox multipliers, two Weyl constants, counter layout, kernel
and run-ID key folding, and both output mappings. Validation requires those
constants and flags, so a target cannot substitute another RNG.

### Stage and program envelopes

`StageKind` is the complete backend-neutral operation inventory:

* `ScalarMap { template }` embeds a validated core `KernelTemplate`.
* `Fill { value }` writes an identity or other scalar literal.
* `Copy { reason }` performs the scatter-base copy.
* `FixedTreeReduce`, `FixedTreeScanLocal`, and `ScanUniformCombine` carry the
  fixed collective contracts above.
* `TiledContraction` carries `ContractionStage`.
* `Gather` and `Scatter` carry an axis, bounds policy, and scatter conflict.
* `HistogramClear` and `HistogramAccumulate` carry bin count, weighting,
  mapping, and atomic ordering.
* `StableSortInitialize`, `StableSortCompareExchange`, and `StableSortFinalize`
  carry the sort network and optional index emission.
* `IndexMap` carries the language affine map.
* `Philox4x32_10` carries `Philox10Contract`.

`StageResourceBounds` gives exact per-stage FLOPs, integer operations, atomic
operations, shared bytes per workgroup, private bytes per lane, and maximum
workgroup lanes. `ProgramStage` combines a dense ID, the immediately previous
stage dependency (or none for stage zero), geometry, ordered bindings,
synchronization points, atomic contracts, optional fault contract, resource
bound, and kind. `ProgramResourceBounds` aggregates total FLOPs, integer and
atomic work, persistent scratch bytes, fault bytes, peak shared and private
bytes, and maximum workgroup lanes. `SourceAliasContract` preserves one
permission for every source input/output pair.

`LoweredProgram` is the complete immutable artifact before target realization:
schema version, source kernel ID, source input and output arity, the complete
alias matrix, dense buffers, ordered stages, exact program resources, and
digest. `validate()` returns every discovered `ProgramValidationError` rather
than stopping at the first path. `canonical_digest()` recomputes the digest
without consulting runtime state.

## Lowering implementation

`lower` first calls `validate_hardware` and `PrimitiveKernel::validate`. A missing
tensor is reported as `MissingTensor` with both kernel and value context. It
then creates a `ProgramBuilder`, registers every source input and output once,
and dispatches by `PrimitiveKind`:

| Kind | Lowered stages and state |
| --- | --- |
| `Elementwise` | Builds a core `KernelTemplate` from the scalar program and broadcast views. Singleton broadcast axes become zero strides. It binds all inputs and outputs, adds an arithmetic fault flag when the scalar program requires one, and accounts scalar slots as private bytes. Empty output shapes produce no dispatch. |
| `Reduce` | Computes reduced width, chooses shared-memory-aware power-of-two collective lanes, and emits one or more `FixedTreeReduce` passes. Intermediate values and indices use purpose-tagged scratch. Empty output uses identity `Fill` stages. |
| `Scan` | Computes sequence and axis widths, emits local fixed-tree stages and scratch block totals for each hierarchy level, then emits uniform combines in reverse level order. Inclusive, exclusive identity, and reverse settings remain explicit. Empty input produces no dispatch. |
| `Contraction` | Computes contracted products and a measured tile. The current physical plan uses direct private accumulators, canonical contracted order, and no shared storage or barriers. |
| `Gather` | Binds source, index, and output. Reject bounds add a checked index fault and release exchange; clamp and wrap do not. |
| `Scatter` | Emits a `ScatterBase` copy for a nonempty base, then update stage bindings. Unique indices use normal writes; atomic conflicts use the owned operation and ordering. Reject bounds add the checked index fault. |
| `Histogram` | Always clears the output bins first. Nonempty input emits atomic accumulation with direct int32 or truncating f32 mapping, optional weight input, and a checked out-of-bounds bin fault. |
| `Sort` | Allocates value and index scratch, pads to the next power of two, emits the complete bitonic network, and finalizes into value and optional index outputs. Empty input produces no stages. |
| `Random` | Emits one `Philox4x32_10` output stage using the language key and distribution, fixed constants, and the four-word counter contract. |
| `IndexMap` | Emits one output stage with the fixed nine integer operations per lane and an arithmetic-domain fault contract. Positive modulus is validated by the language and again in the program kind. |

`ProgramBuilder` keeps tensor-to-buffer IDs, scratch ordinal, optional shared
fault buffer, and ordered stages. `push_stage` assigns the next dense stage ID
and a dependency on the immediately preceding stage. `finish` sums stage work,
takes peak memory bounds, sums persistent scratch and fault storage, sets the
schema and source metadata, computes the digest, and calls
`LoweredProgram::validate`. Any overflow is converted to `ArithmeticOverflow`;
the builder never silently wraps a count or substitutes a smaller geometry.

## Validation and canonical digest

The private `validate` pipeline is also the behavior of the public
`LoweredProgram::validate` method:

1. Check schema version and the complete, unique source alias matrix.
2. Check dense buffer IDs, explicit shapes, shape-to-access agreement,
   rank-matched strides, checked address and storage-byte bounds, injective
   non-atomic writes, unique origins, and origin/lifetime/dtype contracts.
3. Check dense stage IDs, immediate-previous dependencies, nonzero geometry,
   ceiling workgroup count, binding table references and dtypes, view bounds,
   atomic binding contracts, and checked-fault publication.
4. Check each `StageKind`: scalar templates and fault ABI, reduction and scan
   tree steps, output widths, contraction tile and canonical order, reject
   fault requirements, histogram fault reason, sort padding and compare phases,
   Philox constants and counter folding, index-map modulus and fault, and
   fixed binding arities for fill, copy, uniform combine, and clear stages.
5. Recompute exact stage work and synchronization counts, then recompute exact
   program totals and storage peaks. A mismatch is an error even if the values
   are otherwise safe.
6. Recompute the canonical digest and require equality with `program.digest`.

The hash module uses the domain `recipe-lowered-primitive-program-v2` with a
terminating zero byte. `CanonicalWriter` length-prefixes byte strings and
sequences with fixed 128-bit little-endian lengths, encodes integer fields in
little endian, and assigns explicit tags to every enum. It serializes schema,
source identity and arity, aliases, all buffers, all stages, and aggregate
resources, but not the digest field itself. Scalar opcodes use their stable
debug spelling because the core enum is non-exhaustive. The final bytes are
hashed by the crate's self-contained SHA-256 implementation. Thus equal
canonical contracts have equal digests, and any contract change invalidates
the digest before target realization.

## Errors

`LoweringErrorKind` is non-exhaustive and currently includes
`InvalidLanguage`, `MissingTensor`, `ArithmeticOverflow`,
`InvalidStaticAccess`, and `InvalidLoweredProgram`. `LoweringError` stores the
kind, human-readable detail, and optional `KernelTemplateId` and `ValueId`
context. `new`, `for_kernel`, and `for_value` are builder-style constructors;
`From<LanguageError>` preserves language detail and context while changing the
kind to `InvalidLanguage`. `Display` prints the kind and detail followed by any
kernel and value labels, and it implements `std::error::Error`.

`LoweringResult<T>` is the crate's `Result<T, LoweringError>` alias.
`ProgramValidationError` stores a stable field path and detail, has `new`, and
formats as `path: detail`. Validation returns a vector so callers can repair a
complete contract rather than chase one discovered mismatch at a time.

## Architectural invariants

The crate owns operation semantics and physical, target-independent facts. It
must preserve GPU-only f32 and int32 payload calculation, fixed algorithm order,
explicit atomics and memory orderings, checked addresses before invalid access,
and exact resource accounting. It may allocate only external tensor views,
purpose-tagged scratch, and one int32 fault flag. Stages are immutable, dense,
serially dependent records. Empty work is represented by no stage or an
identity fill where the language requires an output, never by a fabricated
dispatch. All later layers must consume this contract directly. A target
backend may choose instruction selection and ABI details only after validating
the complete program and its digest; it may not replace a tree, comparator,
fault path, random mapping, or resource envelope with a runtime-selected
alternative.
