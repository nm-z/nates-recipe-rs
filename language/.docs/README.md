# `recipe-language`

`recipe-language` is Recipe's backend-neutral calculation language. It describes
typed, statically shaped tensor values and placement-free primitive kernels in a
form that can be validated, hashed, lowered, and serialized without choosing a
GPU, driver, queue, allocator, lifecycle phase, or vendor library. Its graph is
the semantic boundary between operation materializers and the measured AOT
pipeline.

The crate owns the meaning of a calculation: tensor shapes and layouts, scalar
SSA programs, primitive parameters, input/output alias permissions, graph
producers, and canonical Recipe OGDL. It does not own physical copies,
transfers, dispatch placement, scheduling, native code, or execution state.
Those concerns consume the validated graph in `recipe-primitives`,
`recipe-planner`, `recipe-prepare`, and `recipe-kernel`.

## Position in the workspace

The language boundary is deliberately narrow and one way:

```text
recipe-core
  DType, IDs, ScalarProgram, ScalarOpcode, units, alias permissions
       |
       +------------------+
       |                  |
recipe-ogdl          recipe-language
 ordered tree        shapes, tensors, scalar builder,
 parser/serializer    primitive kernels, graph, Recipe IR codec
                            |
             +--------------+------------------+
             |                                 |
   recipe-ops and recipe-training       recipe-program
   materialize CalculationGraph         loop domains and metrics
             |                                 |
             +-------------------+-------------+
                                 |
                         recipe-primitives
                         deterministic stage lowering
                                 |
             +-------------------+--------------------+
             |                                        |
       recipe-planner                         recipe-kernel
       placement and tasks                    LLVM, HSACO, cubin
             |
       recipe-prepare -> native executor -> init -> loop -> exit
```

`recipe-language` is a workspace member and a direct dependency of the root
`recipe` package, `recipe-math`, `recipe-ops`, `recipe-primitives`,
`recipe-program`, `recipe-planner`, `recipe-prepare`, `recipe-kernel`, and
`recipe-training`. The root facade re-exports it for advanced callers as
`recipe::engine::language` in [`src/facade.rs`](../../src/facade.rs). The
`native-probe` build script also includes the language source in its provenance
hash, but does not call language APIs at runtime.

## Manifest and module graph

[`Cargo.toml`](../Cargo.toml) declares package `recipe-language` version
`0.1.0`, Rust edition 2024, MIT licensing, and the description
"Backend-neutral tensor and primitive-kernel language for Recipe". It has two
direct path dependencies and no feature flags, build script, executable,
explicit test target, filesystem integration, or runtime dependency:

| Dependency | What the language uses it for |
| --- | --- |
| `recipe-core` | `DType`, byte units, stable `ValueId` and `KernelTemplateId`, scalar SSA records and validation, scalar op signatures and FLOP counts, and alias permissions. |
| `recipe-ogdl` | The ordered rooted-tree parser, builder, node IDs, graph errors, and canonical serializer used by the Recipe IR codec. |

Unsafe Rust is forbidden in the package manifest and at the crate root. The
root also denies missing `Debug` implementations. All implementation modules
are private; [`src/lib.rs`](../src/lib.rs) is the only facade:

```text
src/lib.rs
├── error.rs          LanguageError and LanguageResult
├── shape.rs          Shape and AxisSet
├── tensor.rs         TensorLayout, Tensor, contiguous order
├── scalar_builder.rs ScalarExpression and ScalarProgramBuilder
├── primitive.rs      primitive descriptors, validation, and work estimates
├── graph.rs          CalculationGraph, CalculationNode, assembly and DAG checks
└── ogdl.rs           RecipeIR schema 1 encoder and strict decoder
```

The facade re-exports the typed graph and node, the language and OGDL errors,
all primitive descriptors and enums, the scalar builder, shapes and axis sets,
and tensor/layout types. The underlying `recipe_core` scalar structs are not
duplicated: callers get them through primitive fields or through the core
crate when a complete scalar program is needed.

## Ownership and data flow

The language accepts declarations assembled by operation compilers, training
compilers, inference compilers, and advanced callers:

1. A caller creates `Shape` and `Tensor` values, constructs scalar SSA with
   `ScalarProgramBuilder`, and selects one `PrimitiveKind` with its explicit
   parameters.
2. A `PrimitiveKernel` names input and output `ValueId`s, gives every input and
   output pair an explicit `AliasPermission`, and carries one primitive kind.
3. A `CalculationGraph` groups tensors and calculation nodes. `validate`
   checks storage, primitive contracts, producer uniqueness, external
   boundaries, and acyclicity before the graph is consumed.
4. `CalculationGraph::to_ogdl` emits the canonical `RecipeIR` document. A
   decoder parses the ordered OGDL tree, accepts only the exact schema fields
   and enum spellings, then runs the same graph validation.
5. `StaticCalculationProgram` in `recipe-program` adds a loop iteration domain
   to every source kernel and optional scalar metric declarations. It retains
   this graph unchanged rather than unrolling it.
6. `recipe-planner` validates the static program and graph, computes one common
   measured lowering hardware envelope, and calls `recipe_primitives::lower`
   for every `PrimitiveKernel`. It then enumerates legal device assignments,
   transfers, tasks, and artifacts.
7. `recipe-primitives` turns the primitive into an immutable `LoweredProgram`
   with buffers, one-dimensional dispatch stages, bindings, synchronization,
   atomics, fault contracts, resources, and a canonical digest. Its validator
   is a second boundary after language validation.
8. `recipe-kernel::lower_stage` checks the complete lowered program and the
   deferred artifact contract. Scalar-map stages use the core
   `ScalarProgram` template and direct LLVM emission; the other stage kinds
   use Recipe-owned reduction, scan, contraction, indexing, sort, histogram,
   index-map, and Philox emitters. Only then can a pinned offline toolchain
   produce HSACO or cubin during preparation.

No language object represents a transfer, a queue, a device location, a
physical arena value, a lifecycle phase, or an executable binary. A tensor's
`external_input` and `external_output` bits identify the run boundary only;
they do not allocate or copy anything.

## Core types and contracts

### `LanguageError`

[`src/error.rs`](../src/error.rs) defines the single error type for typed
language construction and validation:

| Field | Meaning |
| --- | --- |
| `kind: LanguageErrorKind` | Machine-readable contract failure. |
| `detail: String` | Human-readable local explanation. |
| `value: Option<ValueId>` | Value context added by `for_value`. |
| `kernel: Option<KernelTemplateId>` | Kernel context added by `for_kernel`. |

`LanguageErrorKind` covers `EmptyShape`, `InvalidAxis`, `DuplicateAxis`,
`ShapeOverflow`, `ByteSizeOverflow`, `InvalidLayout`, `DuplicateTensor`,
`DuplicateKernel`, `UnknownTensor`, `DuplicateProducer`, `MissingProducer`,
`Cycle`, `ArityMismatch`, `DTypeMismatch`, `ShapeMismatch`,
`InvalidScalarProgram`, `InvalidPrimitive`, and `WorkOverflow`.
`Display` prints the kind and detail followed by kernel and value context when
present. `LanguageResult<T>` is the crate-wide result alias. The language does
not catch or replace an invalid transition with a fallback value.

### `Shape` and `AxisSet`

[`src/shape.rs`](../src/shape.rs) owns fixed-rank, fixed-extent metadata.

`Shape::new(extents)` requires a nonempty extent vector. Rank-zero shapes are
not implicit: scalar payloads use `[1]`. Extent zero is valid and produces
zero elements. The constructor multiplies extents with checked `u64`
arithmetic, retaining the element count for later work, storage, and dispatch
decisions. `bytes(dtype)` multiplies that count by the four-byte `DType`
width, also with overflow checking. `rank`, `extents`, `elements`, and
`is_empty` are read-only queries.

The shape transforms are semantic, not backend policies:

| Operation | Result |
| --- | --- |
| `broadcast_result(inputs)` | Right-aligned NumPy-style broadcast. Each result axis accepts equal extents or `1`; conflicting non-singletons return `ShapeMismatch`. An empty input list is invalid. |
| `reduced(axes, keep_dimensions)` | Removes selected axes, or writes extent `1` when `keep_dimensions` is true. Reducing every axis returns `[1]`. The axis set must fit the rank. |
| `gather_result(axis, indices)` | Replaces one source axis with all index-tensor extents. An out-of-range axis returns `InvalidAxis`. |

`AxisSet::new` rejects an empty vector and duplicate axes, sorts the surviving
axes, and exposes a stable slice. `validate_rank` rejects any axis outside the
given rank. These checks are reused by reductions and contractions.

Shape errors are `EmptyShape`, `InvalidAxis`, `DuplicateAxis`,
`ShapeOverflow`, `ByteSizeOverflow`, and `ShapeMismatch`, depending on the
operation. Shape metadata can describe an empty payload; callers must not
turn that state into a fake one-lane dispatch.

### `TensorLayout` and `Tensor`

[`src/tensor.rs`](../src/tensor.rs) keeps static addressing separate from
payload calculations. `ContiguousOrder` has `RowMajor` and `ColumnMajor`.
`TensorLayout::contiguous` computes checked strides with an element stride of
one, starting from the trailing axis for row-major or the leading axis for
column-major, and sets `offset_elements` to zero. `TensorLayout` stores an
element offset and one stride per rank axis.

`TensorLayout::validate(shape)` enforces:

* stride rank equals shape rank;
* every non-singleton axis has a nonzero stride;
* nonempty mappings are injective, checked by sorting axes by stride
  and proving that each occupied span starts at or after the previous span;
* the maximum addressed element plus one fits `u64`.

An empty shape has a zero span and skips the non-overlap walk. The separate
`span_elements` and `byte_offset(dtype)` queries use checked arithmetic and
return `InvalidLayout` or `ByteSizeOverflow` when the address domain cannot be
represented.

`Tensor` is the graph value declaration:

```text
Tensor {
    id: ValueId,
    dtype: DType,                 // F32 or I32
    shape: Shape,
    layout: TensorLayout,
    storage_bytes: ByteCount,    // logical backing object size
    external_input: bool,
    external_output: bool,
}
```

`Tensor::contiguous` creates a row-major layout and derives logical storage
bytes from shape and dtype. `Tensor::validate` validates the layout, computes
the addressed byte span, and rejects a span larger than `storage_bytes`.
Storage may be larger than the span, but never smaller. A tensor error carries
its value ID during `Tensor::validate`, so downstream diagnostics identify the
offending declaration. Constructor errors can occur before a tensor exists and
therefore carry only the shape or layout context available at that point.

### Scalar SSA and `ScalarProgramBuilder`

[`src/scalar_builder.rs`](../src/scalar_builder.rs) is the safe constructor for
the scalar IR defined by `recipe-core`. It does not define a second opcode
set. `ScalarOpcode::result_dtype` in core owns arity and type signatures, and
`ScalarProgram::validate` owns duplicate-definition, use-before-definition,
signature, and output checks.

`ScalarExpression` is an opaque `(builder owner, ScalarValueId, DType)`
reference. A process-wide checked atomic counter gives each builder a unique
owner. Values start at ID one and are allocated in call order. The owner tag
prevents accidentally passing an expression from one builder into another.

The builder accepts:

* `input(dtype)` for one typed scalar input;
* `constant(literal)`, plus `f32(value)` (stored as exact f32 bits) and
  `i32(value)` helpers;
* `apply(opcode, operands)`, and `unary`, `binary`, and `ternary` arity
  conveniences; and
* `finish(outputs)`, which checks ownership, constructs the core
  `ScalarProgram`, and runs core validation.

Instruction order is exactly builder call order, so it is part of the program
identity and later artifact digest. Applying an opcode with a foreign value,
an invalid arity or dtype signature, exhausted builder identity space, an
exhausted scalar value ID space, a foreign output, or a core validation error
returns `InvalidScalarProgram`. The builder never reorders or silently inserts
an instruction.

`recipe-math` builds all of its deterministic special-function programs through
this builder. Operation materializers and training/inference compilers use the
same API for elementwise payloads, predicates, conversions, and checked
`Require` operations.

### Primitive kinds and ownership

[`src/primitive.rs`](../src/primitive.rs) owns one placement-free calculation
kind and its semantic parameters. The shared enums are intentionally explicit:

* `AtomicOrdering` and `AtomicOperation` describe the exact atomic memory
  operation and ordering;
* `IndexBounds` selects `Reject`, `Clamp`, or `Wrap` indexing behavior;
* `ReduceOperator` and `ReduceResult` describe reduction domains and value or
  index outputs;
* `ScanMode` distinguishes inclusive from exclusive scans with a typed
  identity;
* `ScatterConflict` records unique-index assumptions or an atomic RMW pair;
* `SortDirection` records ascending or descending ordering; and
* `RandomDistribution` and `RandomKey` make deterministic distribution and
  counter material explicit.

The closed domains are intentional: atomic orderings are `Relaxed`, `Acquire`,
`Release`, `AcquireRelease`, and `SequentiallyConsistent`; atomic operations
are `Exchange`, `Add`, `Minimum`, and `Maximum`; bounds are `Reject`, `Clamp`,
and `Wrap`; reductions are `Sum`, `Product`, `Minimum`, `Maximum`, `Any`, and
`All`; reduction results are `Value`, `Index`, and `ValueAndIndex`; scan modes
are `Inclusive` or `Exclusive { identity }`; and sort directions are
`Ascending` or `Descending`. Random distributions are `UniformF32`,
`NormalF32`, `BernoulliI32 { probability_bits }`, and
`UniformI32 { low, high_exclusive }`.

`PrimitiveKind` has ten variants:

| Kind | Payload and semantic output |
| --- | --- |
| `Elementwise` | A validated core `ScalarProgram`, applied once per broadcast element. |
| `Reduce` | Operator, nonempty `AxisSet`, keep-dimensions flag, value/index result mode, and a power-of-two fixed tree width. |
| `Scan` | Operator, one axis, inclusive or typed-identity exclusive mode, direction, and fixed tree width. |
| `Contraction` | Ordered batch-axis pairs and contracted-axis pairs. Batch dimensions lead the output, followed by all unpaired left and right dimensions. |
| `Gather` | One source axis and an explicit bounds policy. Index input is `I32`; its shape replaces the selected source extent. |
| `Scatter` | One destination axis, bounds policy, and unique or atomic conflict contract. Updates have the gather-derived shape. |
| `Histogram` | Bin count, weighted/unweighted mode, and atomic ordering. Output shape is `[bins]`. |
| `Sort` | Axis, direction, stability, and optional index emission. Values preserve the input shape; emitted indices are `I32`. |
| `IndexMap` | Checked affine `I32` source using `start + element_index * element_step + loop_iteration * iteration_step`, optionally reduced by a positive modulus. |
| `Random` | Recipe-owned Philox4x32-10 distribution, key material, and output dtype. The runtime folds run ID, loop iteration, kernel ID, and element index into the counter. |

`PrimitiveAliasRule` maps a zero-based primitive input index and output index to
one `recipe_core::AliasPermission` (`Forbidden`, `MayAliasExact`, or
`MustAliasExact`). `PrimitiveKernel` combines its stable `KernelTemplateId`,
ordered input and output `ValueId` vectors, complete alias matrix, and one
`PrimitiveKind`:

```text
PrimitiveKernel
├── id: KernelTemplateId
├── inputs:  [ValueId]
├── outputs: [ValueId]
├── alias_rules: every (input, output) pair exactly once
└── kind: one PrimitiveKind
```

The language supplies no kernel or value allocator. The operation and training
compilers own ID allocation and graph assembly; language validation checks the
resulting declarations.

## Primitive validation and work

`PrimitiveKernel::validate(tensors)` first resolves every referenced tensor,
then validates the alias matrix, then dispatches to the kind-specific checker.
The complete contract is:

| Kind | Required arity and checks |
| --- | --- |
| Elementwise | Core scalar program valid; tensor input/output counts equal scalar input/output counts; at least one tensor input; input dtypes equal scalar input dtypes; outputs match broadcast shape and scalar output dtypes. |
| Reduce | One input; one output for `Value` or `Index`, two for `ValueAndIndex`; axes fit rank; tree width is a power of two in `1..=1024`; `Any` and `All` require `I32`; index results require `Minimum` or `Maximum`; output shape is `Shape::reduced`; value/index dtypes are exact; empty `Minimum` and `Maximum` domains are rejected. |
| Scan | One input and output; axis fits rank; fixed tree width; `Any` and `All` require `I32`; an exclusive identity has the input dtype; output dtype and shape equal input. |
| Contraction | Two inputs and one output; at least one contracted pair; all operand and output dtypes equal; every pair is in range, reuses no left or right axis, and has equal extents; output extent order follows batch, unpaired left, then unpaired right axes. All-used operands produce `[1]`. |
| Gather | Two inputs and one output; indices are `I32`; output dtype equals source dtype; output shape is `gather_result`. |
| Scatter | Three inputs and one output; indices are `I32`; updates and output match source dtype; output matches source shape; updates match the gather-derived shape. The selected lowering must preserve the conflict operation and ordering. |
| Histogram | One input when unweighted, two when weighted; one output; bins are `1..=i32::MAX`; weighted values are `F32` and shape-equal to the data; output dtype is `F32` when weighted and `I32` otherwise; output shape is `[bins]`. |
| Sort | One input; one output or two when indices are emitted; axis fits rank and its extent fits `I32` indices; values preserve dtype and shape; emitted indices are `I32` with the same shape. |
| IndexMap | No inputs and one output; output is `I32`; an optional modulus is strictly positive. Output shape remains caller-declared. |
| Random | No inputs and one output; exactly ten Philox rounds; `UniformF32` and `NormalF32` output `F32`; Bernoulli probability is finite in `[0, 1]` and outputs `I32`; integer ranges require `low < high_exclusive` and output `I32`. |

Every failed check returns a `LanguageError` with the kernel ID and, where a
specific value is known, its value ID. Missing tensor references are
`UnknownTensor`; bad counts are `ArityMismatch`; type and shape failures are
`DTypeMismatch` and `ShapeMismatch`; malformed axis, tree, conflict, modulus,
random, or alias declarations are `InvalidAxis`, `DuplicateAxis`, or
`InvalidPrimitive`.

`PrimitiveKernel::work(tensors)` validates first and then returns a checked
`recipe_core::FlopCount`. The deterministic estimates are:

| Kind | Work estimate |
| --- | --- |
| Elementwise | Output elements times the sum of core opcode FLOPs. `Fma` contributes two; representation, predicates, and addressing contribute zero. |
| Reduce | `(input elements - output elements)` with saturating subtraction, doubled for `ValueAndIndex`. |
| Scan | Two times input elements. |
| Contraction | Product of left contracted extents, output elements, and two. |
| Gather or Scatter | Output elements. |
| Histogram | Input elements. |
| Sort | A bounded `ceil(log2(axis)) * axis` comparison estimate times the number of slices. |
| IndexMap | Zero FLOPs. |
| Random | Output elements times Philox rounds times four. |

Any checked multiplication or addition that cannot fit `u64` returns
`WorkOverflow`. This count is a scheduling input, not a measured runtime
result, and it never includes transfers or lifecycle work.

## What the next lowering boundary does

The language deliberately stops before physical realization, but the mapping
from each language primitive to `recipe-primitives::lower` is fixed and
observable:

| Language primitive | Target-independent lowered stages |
| --- | --- |
| `Elementwise` | One `StageKind::ScalarMap` carrying a validated core `KernelTemplate`. Broadcast dimensions become zero strides in the input views. A scalar program that uses checked integer arithmetic or `Require` receives one preallocated arithmetic fault flag. Empty output shapes emit no dispatch. |
| `Reduce` | One or more `StageKind::FixedTreeReduce` passes. The measured common shared-memory and workgroup limits select a power-of-two collective width no larger than the declared tree width. Nonfinal passes write typed reduction scratch; final passes write the requested value and/or index tensors. Empty identity domains use explicit `StageKind::Fill` stages, while empty `Minimum` and `Maximum` are rejected earlier by the language. |
| `Scan` | A hierarchy of `FixedTreeScanLocal` stages, optional scan-value and block-total scratch, and `ScanUniformCombine` stages for block offsets. The first level retains the user inclusive or exclusive identity and reverse flag; hierarchy levels use an explicit exclusive identity. |
| `Contraction` | One `StageKind::TiledContraction` with ordered contracted coordinates, a measured tile, a fixed accumulation order, and a direct private-accumulator strategy. The stage has no hidden backend contraction library. |
| `Gather` | One `StageKind::Gather` with the language axis and bounds policy. `Reject` adds an index fault contract guarded before address formation. |
| `Scatter` | A `StageKind::Copy` of the base tensor followed by `StageKind::Scatter`. Unique indices use ordinary writes; an atomic conflict carries the exact language operation and ordering into the tensor-element atomic contract. `Reject` adds the same checked index fault path. |
| `Histogram` | `HistogramClear` followed by `HistogramAccumulate`. The input dtype selects direct `I32` or truncate-toward-zero `F32` bin mapping. Accumulation always carries a histogram-bin fault flag and the requested atomic ordering. |
| `Sort` | `StableSortInitialize`, a fixed bitonic sequence of `StableSortCompareExchange` stages over the least power-of-two padded axis, then `StableSortFinalize`, optionally writing `I32` original-axis indices. NaNs, signed zero, ties, and padding use explicit deterministic ordering contracts. |
| `IndexMap` | One checked `StageKind::IndexMap` with the affine coefficients and optional positive modulus. It always has an arithmetic-domain fault flag because signed-64 intermediate and final `I32` bounds are part of the language contract. |
| `Random` | One `StageKind::Philox4x32_10` with the exact multipliers, Weyl constants, counter words, run/kernel folding, and distribution mapping. The stage exposes run ID and loop iteration as dynamic ABI arguments when required. |

The primitive lowerer first validates measured `LoweringHardware`, revalidates
the language kernel, and registers each referenced tensor as an external
buffer. It then allocates only program scratch and a shared four-byte fault
flag, assigns dense buffer and stage IDs, serializes dependencies as a chain
from each stage to the immediately previous stage, computes dispatch geometry,
and aggregates exact FLOP, integer, atomic, shared, private, scratch, and
fault bounds. `LoweredProgram::validate` recomputes every aggregate and its
domain-separated digest. Thus a malformed language graph cannot be made to
look valid by constructing a stage object directly, and a malformed stage
cannot be passed to the planner or kernel compiler.

The downstream validator's structural checks are explicit:

* schema version is `2`, source aliases form a complete unique input/output
  matrix, buffer IDs and stage IDs are dense, and every tensor/scratch/fault
  origin has the matching lifetime, dtype, rank, shape, and storage contract;
* every static access has matching extent and stride ranks, fits its storage,
  and is injective whenever it is writable without atomics;
* each stage has nonzero logical and workgroup lanes, the exact ceiling
  workgroup count, the immediately previous stage dependency, valid bindings,
  unique atomic contracts, and a fault contract whose reason matches the kind;
* reduction trees are descending fixed-width trees, scan trees are canonical
  Blelloch upsweep and downsweep trees, contractions have a nonzero tile whose
  output product equals the workgroup width, and sorts use the least
  power-of-two padded domain with IEEE total ordering and original-index ties;
* `Reject` gather/scatter, histogram accumulation, index maps, and checked
  scalar maps carry the required preallocated fault path; Philox stages retain
  the fixed constants, counter words, run/kernel folding, and ten rounds; and
* per-stage resource equations and program-wide aggregates must exactly match
  the recomputed digest input. Any mismatch is a validation error rather than a
  fallback stage.

## `CalculationGraph`

[`src/graph.rs`](../src/graph.rs) is the typed in-memory graph contract:

```text
CalculationGraph {
    tensors: Vec<Tensor>,
    nodes:   Vec<CalculationNode>,
}

CalculationNode {
    kernel: PrimitiveKernel,
}
```

The graph stores only calculation nodes. It has no explicit edge list because
edges are derived from tensor producer IDs: a kernel depends on the producer of
each input value. This keeps the semantic graph in terms of values and
primitive calculations rather than queues, routes, or transfer tasks.

### Assembly

`CalculationGraph::assemble(fragments, external_inputs, external_outputs)` is
the composition boundary for independently materialized operation fragments.
It canonicalizes the supplied boundary sets into sorted sets while rejecting
repeated boundary IDs, merges tensor declarations by `ValueId`, and allows
repeated declarations only when dtype, shape, layout, and storage bytes are
identical. Fragment external flags are discarded because they describe
temporary fragment boundaries. The caller's exact boundary sets are applied to
the merged tensors.
Unknown boundary values are rejected. Nodes are merged and sorted by kernel ID,
tensors by value ID, and the assembled graph is validated before it is
returned.

`CalculationGraph` and `CalculationNode` have public fields and can also be
constructed directly. Direct construction does not bypass any contract:
callers must invoke `validate`, and all production compilers do so before
serialization or lowering. `assemble` is the one reusable merge operation when
independently materialized fragments need boundary flags repaired at the
complete-graph level.

### Validation

`CalculationGraph::validate` runs in this order:

1. Index tensors by ID and reject duplicate tensor declarations. Validate every
   tensor layout and storage span.
2. Index kernel IDs and reject duplicates. Validate every primitive kernel,
   recording one producer kernel for each output value.
3. Reject two producers for one value. Reject an external input that is also
   produced. Reject a non-external tensor that has no producer. A value may be
   both an external input and an external output when it is intentionally
   passed through unchanged.
4. Build producer-derived dependency edges, reject self-consumption, and run a
   deterministic Kahn topological walk using ascending kernel IDs. Any nodes
   left behind are a `Cycle`.

`topological_order()` revalidates and returns kernel IDs in that stable order.
`dependencies(kernel)` revalidates, resolves the named kernel, collects the
producer IDs for its inputs, sorts them, and removes duplicates. Asking for an
absent kernel returns `InvalidPrimitive` with kernel context.

The graph therefore guarantees unique storage declarations, valid static
addressing, complete primitive contracts, one producer per calculated value,
an explicit external-input boundary, and an acyclic value dependency graph.
It does not prove placement feasibility, measured capacity, native ABI, or
runtime completion. Those are downstream contracts.

## Canonical Recipe OGDL

[`src/ogdl.rs`](../src/ogdl.rs) owns the textual `RecipeIR` schema. It uses the
ordered tree from `recipe-ogdl`, whose node identities are local and whose
serializer preserves insertion order. Shared references and general OGDL
anchors are not part of this format.

### Document shape

Every encoded document has exactly one root and these exact root fields:

```text
RecipeIR
    schema CalculationGraph
    version 1
    tensors
        tensor ...
    nodes
        node
            kernel ...
```

Each `tensor` records `id`, `dtype` (`F32` or `I32`), ordered `shape` extents,
`layout.offset_elements`, ordered `layout.strides`, `storage_bytes`,
`external_input`, and `external_output`. Each kernel records `id`, ordered
`inputs.value`, ordered `outputs.value`, ordered `alias_rules` with `input`,
`output`, and exact permission spelling, and one `kind` variant.

The primitive variants encode all semantic parameters without defaults:

| Variant | Fields |
| --- | --- |
| `Elementwise` | `program` with scalar `inputs`, `constants`, `instructions`, and `outputs`. |
| `Reduce` | `operator`, `axes.axis`, `keep_dimensions`, `result`, `tree_lanes`. |
| `Scan` | `operator`, `axis`, `mode` (`Inclusive` or `Exclusive.identity`), `reverse`, `tree_lanes`. |
| `Contraction` | `batch_axes.pair(left, right)`, `contract_axes.pair(left, right)`. |
| `Gather` | `axis`, `bounds`. |
| `Scatter` | `axis`, `bounds`, `conflict` (`UniqueIndices` or `Atomic.operation` and `Atomic.ordering`). |
| `Histogram` | `bins`, `weighted`, `ordering`. |
| `Sort` | `axis`, `direction`, `stable`, `emit_indices`. |
| `IndexMap` | `start`, `element_step`, `iteration_step`, `modulus` (`None` or `Some(value)`). |
| `Random` | `distribution`, `key.seed_low`, `key.seed_high`, `key.stream`, `philox_rounds`. |

Scalar programs retain IDs, dtypes, literals, opcode spellings, operand order,
and output order. Literals are exact `F32Bits(u32)` or `I32(i32)` records, so
serialization does not round-trip through decimal floating-point text. The
encoder has an explicit schema-version gate: a core scalar opcode not known by
schema version 1 returns `UnsupportedValue` instead of emitting an ambiguous
record.

### Encoding

`CalculationGraph::to_ogdl_graph` validates the graph, appends the root and
fields in the fixed order above, and returns a `recipe_ogdl::Graph`.
`to_ogdl` serializes that graph with `to_canonical_string`. A successful output
is therefore both canonical text and a semantically valid graph. Build failures
from the underlying ordered graph are returned as `OgdlCodecError::Build`.

### Strict decoding

`from_ogdl` first invokes `recipe_ogdl::Graph::parse`; syntax failures are
`OgdlCodecError::Syntax`. `from_ogdl_graph` then applies strict document rules:

* exactly one root named `RecipeIR`;
* exact schema `CalculationGraph` and version `1`;
* every record has all required fields exactly once;
* unknown fields, duplicate fields, missing fields, and wrong collection item
  names are rejected;
* scalar fields have exactly one leaf value, and enum nodes have exactly one
  variant child;
* booleans are the lowercase tokens `true` or `false`;
* integers are parsed with the destination type and must be in range; and
* enum and opcode spellings must be exact and known.

The decoder builds `Tensor`, `PrimitiveKernel`, and core scalar records only
after these document checks. It then constructs `CalculationGraph` and calls
`validate`, so malformed shapes, layouts, aliases, primitive contracts,
producer sets, and cycles are reported as `OgdlCodecError::InvalidGraph` (or a
document error when the failure is in the record itself).

`OgdlDocumentErrorKind` distinguishes `InvalidRoot`, `MissingField`,
`DuplicateField`, `UnknownField`, `UnknownVariant`, `InvalidNumber`,
`InvalidBoolean`, `UnexpectedChildren`, and `UnsupportedValue`. The public
`OgdlCodecError` also preserves underlying `ParseError`, `GraphError`, and
`LanguageError`, with path-addressed document detail for precise diagnostics.

## Downstream lowering and validation boundaries

The language contract is intentionally checked more than once at narrower
boundaries. Each layer adds facts it owns rather than replacing language
validation:

| Boundary | Input from language | Additional checks and output |
| --- | --- | --- |
| `recipe-program::StaticCalculationProgram` | `CalculationGraph` | Revalidates the graph, assigns one explicit iteration domain per kernel, checks producer domains and metric scalar values, and serializes a `RecipeProgram` root plus the embedded `RecipeIR`. |
| `recipe-ops` and `recipe-training` finalizers | Materialized tensors, nodes, and scalar programs | Allocate IDs in their own namespaces, set final external flags, call `graph.validate`, and perform a canonical graph OGDL round trip before constructing the static program. |
| `recipe-planner::plan_program_candidates` | Static program and graph | Validates program, graph, measured topology/discovery/reservations/capacity, obtains topological order, calls `recipe_primitives::lower` for every node, validates each `LoweredProgram`, and enumerates measured device assignments. |
| `recipe-primitives::lower` | `PrimitiveKernel`, tensor index, measured hardware envelope | Revalidates the kernel, creates external tensor buffers, emits immutable stages for each primitive family, copies alias rules into a complete source alias contract, and validates the resulting buffers, bindings, resources, stages, fault contracts, and digest. |
| `recipe-kernel::lower_stage` | One validated `LoweredProgram` stage and `ArtifactBuildRecipe` | Revalidates program and build contract, checks stage identity, ABI geometry, bindings, work/resource bounds, and fault binding, then emits direct target LLVM IR. Scalar maps use the core scalar program; owned stages use Recipe-owned emitters. IR is audited before offline HSACO or cubin construction. |
| `recipe-prepare` | Graph or static program plus measured profile | Wraps a bare graph in a one-iteration program when requested, validates measured profile and reservations, resolves artifacts, asks the planner for ranked candidates, realizes and stabilizes exact candidates, then finalizes an immutable bundle. |
| `recipe-training::execute` and inference boundaries | Compiled static program and external declarations | Revalidate graph and loop constraints before native preparation or output egress. They independently check external input/output roles and shape/storage contracts. |

The most important distinction is that language validation proves semantic
well-formedness, while primitive lowering proves a target-independent stage
can represent that semantics under measured limits. Planner and preparation
then prove placement, route, capacity, artifact, and lifecycle contracts.
Compilation success, an emitted status, or a serialized text match is not
runtime correctness evidence.

`recipe-ops` is the main production graph producer outside training. Its direct
primitive entrypoint first checks that an operation descriptor really owns a
primitive recipe, matches the descriptor's family, dtype contract, axis class,
bounds policy, conflict ordering, stable-sort choice, or exact alias rule, and
only then calls `recipe_primitives::lower`. Its structured-operation boundary
does more work before the language graph exists: `MaterializationRequest`
provides named input/output tensors, prepared parameters, a caller-reserved
value and kernel identity namespace, and a workspace limit; composition
expansion resolves finite shape/parameter bounds; concrete family emitters
create intermediates with `Tensor::contiguous` and kernels with explicit
`PrimitiveKind`; and `GraphBuilder::finish` calls `CalculationGraph::validate`
before returning `MaterializedComposition`. Missing tensor ABI, scalar formula,
primitive parameters, or workspace policy fails closed instead of inventing a
language node. Specialized emitters such as K-means, Bayesian inference, tree
inference, KNN outputs, and binary metrics follow the same pattern and each
validates its completed graph.

At the final compiler boundary, `recipe-kernel::lower_stage` refuses to trust a
caller-selected fragment. It recomputes the stage-scoped template identity from
the lowered-program digest, source kernel, and stage ordinal; checks the
deferred artifact's program, source, stage, artifact, and contract digests;
matches dispatch geometry, work bounds, resource limits, and every static view;
and requires the fault value to be the exact ordered fault binding. Its ABI
contains only the stage's read/write buffer pointers, an optional four-byte
fault pointer, optional `RunId` and loop-iteration values for counter-based or
iteration-aware stages, and the explicit element count. Scalar maps are emitted
from the core `KernelTemplate` and their checked fault publication is rewritten
to the stage's exact release exchange code. All other stage kinds are emitted
by Recipe-owned LLVM builders, then audited for prohibited interfaces before
the pinned offline compiler is reached.

## Inputs, outputs, and failure behavior

### Inputs accepted by this crate

* `Shape` extents and axis vectors from operation-specific shape inference;
* `Tensor` declarations with typed payload, static layout, storage bytes, and
  explicit run-boundary flags;
* core scalar dtypes, literals, opcodes, and scalar programs;
* primitive parameters and ordered tensor IDs from operation/training graph
  materializers;
* fragments plus exact external boundary ID sets for graph assembly; and
* parsed `recipe_ogdl::Graph` values or textual OGDL for the codec.

The crate does not read files, discover hardware, consult environment state,
allocate device memory, or invoke a compiler.

### Outputs produced by this crate

* validated `Shape`, `AxisSet`, `TensorLayout`, `Tensor`, scalar expressions,
  and core `ScalarProgram` values;
* validated, ordered `PrimitiveKernel` declarations and `CalculationGraph`s;
* deterministic topological order, producer-derived dependencies, and checked
  primitive FLOP estimates;
* canonical `RecipeIR` text or ordered graphs; and
* `LanguageError` or `OgdlCodecError` with machine-readable kind, path/context,
  and no substitute object on failure.

It never produces a `LoweredProgram`, `KernelTemplate`, device task, artifact,
native image, or execution journal. Those outputs belong to downstream crates.

## End-to-end validation recipe

For a real graph path, the smallest complete boundary is:

```text
materializer -> CalculationGraph::validate
             -> CalculationGraph::to_ogdl
             -> CalculationGraph::from_ogdl
             -> StaticCalculationProgram::new(...)
             -> recipe_primitives::lower per kernel
             -> LoweredProgram::validate
             -> recipe_planner::plan_program_candidates
             -> recipe_prepare::Preparer::prepare
             -> recipe_kernel / native executor
```

Each arrow preserves the same semantic graph and adds only the facts owned by
the next layer. A failure should be repaired at the first boundary that can
state it: shape/layout and primitive errors in `recipe-language`, document
errors in the OGDL codec, stage-contract errors in `recipe-primitives`,
placement and capacity errors in planner/preparation, and target or artifact
errors in `recipe-kernel`. There is no alternate graph representation or
fallback execution path that can mask a broken transition.
