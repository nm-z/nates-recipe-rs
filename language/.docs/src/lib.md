# `recipe-language` facade

`language/src/lib.rs` is the public declaration facade for the
`recipe-language` crate. The crate is the backend-neutral, statically shaped
calculation language used by Recipe. It owns tensor metadata, scalar-program
construction, primitive-kernel declarations, graph validation, and the
canonical OGDL codec for a calculation graph. It does not own placement,
transfers, queues, drivers, lifecycle phases, allocation, native images, or
execution.

The root contract is visible in the source attributes:

```rust
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
```

`language/Cargo.toml` has exactly two dependencies, `recipe-core` and
`recipe-ogdl`. Core supplies shared IDs, data types, scalar SSA records,
checked units, alias permissions, and scalar opcode semantics. OGDL supplies
the ordered syntax tree, parser, and graph-building errors. The language crate
adds the domain contract around those shared records without reimplementing
their ownership.

## Intent and semantic boundary

The root documentation states one authoritative interpretation:

1. `CalculationGraph` is the typed in-memory contract.
2. `CalculationGraph::to_ogdl` emits the canonical versioned textual form.
3. `CalculationGraph::from_ogdl` accepts only that exact schema and then
   validates the resulting graph.
4. No missing value is defaulted. Fields, collection items, enum spellings,
   booleans, and numbers are exact.

The graph is a graph of Recipe-owned calculations. A primitive is a
placement-free operation over named tensor values. A tensor shape, layout, and
storage size are static metadata, not payload calculations. Scalar programs
are typed SSA payload descriptions. The language layer therefore stops at the
semantic operation boundary:

```text
shape/layout + scalar program + primitive kind + value edges
    -> validated CalculationGraph
    -> canonical RecipeIR/CalculationGraph OGDL
```

The next stages consume this contract as follows:

```text
recipe-language
    -> recipe-program       (loop domains around the same graph)
    -> recipe-primitives    (backend-neutral AOT stage lowering)
    -> recipe-planner       (placement, copies, resources, and schedule)
    -> recipe-prepare       (target realization and immutable preparation)
    -> recipe-kernel/executor/native-executor (native image and execution)
```

Those later stages must not be described as fields of this crate. In
particular, a `CalculationGraph` does not contain a device, queue, transfer,
kernel image, allocation, lifecycle phase, or measured hardware property.

## What `lib.rs` exposes

The implementation modules are private and declared in this order:

```text
error, graph, ogdl, primitive, scalar_builder, shape, tensor
```

The root flattens the intended public surface with these reexports:

```rust
pub use error::{LanguageError, LanguageErrorKind, LanguageResult};
pub use graph::{CalculationGraph, CalculationNode};
pub use ogdl::{OgdlCodecError, OgdlDocumentErrorKind};
pub use primitive::{
    AtomicOperation, AtomicOrdering, Contraction, Elementwise, Gather, Histogram, IndexBounds, IndexMap,
    PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce,
    ReduceOperator, ReduceResult, Scan, ScanMode, Scatter, ScatterConflict, Sort, SortDirection,
};
pub use scalar_builder::{ScalarExpression, ScalarProgramBuilder};
pub use shape::{AxisSet, Shape};
pub use tensor::{ContiguousOrder, Tensor, TensorLayout};
```

The reexports are one import surface, not duplicate implementations. Each
name is owned by the private module where it is declared. Types owned by
`recipe-core`, such as `DType`, `ValueId`, `KernelTemplateId`, `ScalarLiteral`,
`ScalarOpcode`, `ScalarProgram`, `ScalarValueId`, `ByteCount`, `ByteOffset`,
`FlopCount`, and `AliasPermission`, remain imported from `recipe_core`; the
language facade deliberately does not shadow them.

## Module map

| Module | Root names | Responsibility | Explicit non-responsibility |
| --- | --- | --- | --- |
| `error` | `LanguageError`, `LanguageErrorKind`, `LanguageResult` | Structured language and graph diagnostics, with optional value and kernel context | No recovery, defaults, or backend errors |
| `shape` | `Shape`, `AxisSet` | Checked fixed-rank extents, element counts, broadcasting, reduction and gather shape rules, and axis sets | No dynamic shape inference or device shape |
| `tensor` | `ContiguousOrder`, `TensorLayout`, `Tensor` | Static element layout, byte span, storage declaration, and external boundary flags | No allocation or physical address |
| `scalar_builder` | `ScalarExpression`, `ScalarProgramBuilder` | Typed, builder-owned scalar SSA construction | No scalar execution or backend source strings |
| `primitive` | Primitive enums, specs, alias rules, `PrimitiveKernel` | Placement-free operation declarations, validation, and checked work estimates | No lowering, placement, synchronization, or execution |
| `graph` | `CalculationNode`, `CalculationGraph` | Tensor/node assembly, producer uniqueness, dependency validation, and deterministic topological order | No loop activation domains or schedule |
| `ogdl` | `OgdlCodecError`, `OgdlDocumentErrorKind`, graph codec methods | Canonical `RecipeIR`/`CalculationGraph` version-1 serialization and strict decode | No compatibility defaults or semantic migration |

## Exact public item inventory

The following inventory is the complete root surface as of the source traced for
this document. Public fields are intentional construction data; private fields
are used where the crate must preserve an invariant during construction.

### Errors

`LanguageErrorKind` is `#[non_exhaustive]` and currently contains:

```text
EmptyShape, InvalidAxis, DuplicateAxis, ShapeOverflow, ByteSizeOverflow,
InvalidLayout, DuplicateTensor, DuplicateKernel, UnknownTensor,
DuplicateProducer, MissingProducer, Cycle, ArityMismatch, DTypeMismatch,
ShapeMismatch, InvalidScalarProgram, InvalidPrimitive, WorkOverflow
```

`LanguageError` has public fields:

```text
kind: LanguageErrorKind
detail: String
value: Option<ValueId>
kernel: Option<KernelTemplateId>
```

Its constructors and context methods are:

```rust
LanguageError::new(kind, detail) -> LanguageError
LanguageError::for_value(self, value) -> LanguageError
LanguageError::for_kernel(self, kernel) -> LanguageError
```

`Display` prints `Kind: detail`, then an optional `[kernel ...]` and
`[value ...]`. It implements `std::error::Error`. `LanguageResult<T>` is the
alias `Result<T, LanguageError>`.

### Shapes and axes

`Shape` is a fixed-rank, fixed-extent value with private `extents: Vec<u64>`
and cached `elements: u64`. `Shape::new` rejects rank zero, accepts zero
extents, and checked-multiplies all extents. Scalar payloads use shape `[1]`;
there is no implicit rank-zero payload. Its public methods are:

```rust
Shape::new(extents: Vec<u64>) -> LanguageResult<Shape>
Shape::extents(&self) -> &[u64]
Shape::rank(&self) -> usize
Shape::elements(&self) -> u64
Shape::is_empty(&self) -> bool
Shape::bytes(&self, dtype: DType) -> LanguageResult<ByteCount>
Shape::broadcast_result(inputs: &[&Shape]) -> LanguageResult<Shape>
Shape::reduced(&self, axes: &AxisSet, keep_dimensions: bool) -> LanguageResult<Shape>
Shape::gather_result(&self, axis: usize, indices: &Shape) -> LanguageResult<Shape>
```

Broadcasting right-aligns ranks, permits extent `1` to expand, rejects
conflicting non-one extents, and rejects an empty input list. Reduction checks
axis rank, removes selected axes or replaces them with `1`, and returns `[1]`
when every axis is removed. Gather replaces one source axis with all index
extents and checks that the source axis exists. Zero elements are valid and
represent a no-dispatch calculation for planning.

`AxisSet` owns a sorted, nonempty, duplicate-free `Vec<usize>`:

```rust
AxisSet::new(axes: Vec<usize>) -> LanguageResult<AxisSet>
AxisSet::as_slice(&self) -> &[usize]
AxisSet::contains(&self, axis: usize) -> bool
AxisSet::validate_rank(&self, rank: usize) -> LanguageResult<()>
```

The constructor sorts before checking uniqueness. Rank validation rejects any
axis greater than or equal to the supplied rank.

### Tensor layout and tensor metadata

`ContiguousOrder` has `RowMajor` and `ColumnMajor`. `TensorLayout` has public
metadata fields:

```text
offset_elements: u64
strides: Vec<u64>
```

Its methods are:

```rust
TensorLayout::contiguous(shape: &Shape, order: ContiguousOrder) -> LanguageResult<TensorLayout>
TensorLayout::validate(&self, shape: &Shape) -> LanguageResult<()>
TensorLayout::span_elements(&self, shape: &Shape) -> LanguageResult<u64>
TensorLayout::byte_offset(&self, dtype: DType) -> LanguageResult<ByteOffset>
```

Contiguous layout starts at element offset zero and computes checked strides in
the requested order. Validation requires one stride per rank, rejects a zero
stride on a non-singleton nonempty axis, checks that non-singleton axes do not
overlap when ordered by stride, and checks the full span. Empty shapes have a
zero element span. `byte_offset` only converts the starting element offset to
bytes and checks overflow.

`Tensor` has public fields:

```text
id: ValueId
dtype: DType
shape: Shape
layout: TensorLayout
storage_bytes: ByteCount
external_input: bool
external_output: bool
```

`Tensor::contiguous(id, dtype, shape, external_input, external_output)` creates
a row-major tensor and sets storage bytes from the logical shape. The
`storage_bytes` comment means the logical backing object before placement.
`Tensor::validate` validates the layout and checks that its byte span does not
exceed the declared storage. Tensor flags are graph-boundary metadata only;
they do not allocate or transfer a value.

### Scalar programs

`ScalarExpression` is an opaque, typed reference. Its owner token, scalar value
ID, and dtype are private. Only these accessors are public:

```rust
ScalarExpression::id(self) -> ScalarValueId
ScalarExpression::dtype(self) -> DType
```

`ScalarProgramBuilder` is a deterministic constructor for a core
`ScalarProgram`. It has a private builder owner, next-value counter, and ordered
input, constant, and instruction vectors. Its public methods are:

```rust
ScalarProgramBuilder::new() -> LanguageResult<ScalarProgramBuilder>
builder.input(dtype: DType) -> LanguageResult<ScalarExpression>
builder.constant(value: ScalarLiteral) -> LanguageResult<ScalarExpression>
builder.f32(value: f32) -> LanguageResult<ScalarExpression>
builder.i32(value: i32) -> LanguageResult<ScalarExpression>
builder.apply(opcode: ScalarOpcode, operands: &[ScalarExpression])
    -> LanguageResult<ScalarExpression>
builder.unary(opcode, operand) -> LanguageResult<ScalarExpression>
builder.binary(opcode, left, right) -> LanguageResult<ScalarExpression>
builder.ternary(opcode, first, second, third) -> LanguageResult<ScalarExpression>
builder.finish(outputs: &[ScalarExpression]) -> LanguageResult<ScalarProgram>
```

Every expression ID is allocated in call order. `f32` records exact
`F32Bits`; `i32` records an `I32` literal. `apply` rejects an expression owned
by another builder, asks the core opcode for its result dtype, and appends one
instruction. The arity and dtype signature therefore comes from
`ScalarOpcode::result_dtype`, not from a second language-side table. `finish`
rejects foreign outputs, builds the core program, and invokes its validator.
Builder identity exhaustion and scalar ID overflow are reported as
`InvalidScalarProgram`. This owner token is the reason expressions cannot be
spliced between programs while remaining cheap `Copy` handles.

### Primitive declarations

The primitive domain is closed in this source (the enums are not marked
`non_exhaustive`):

```text
AtomicOrdering = Relaxed | Acquire | Release | AcquireRelease | SequentiallyConsistent
AtomicOperation = Exchange | Add | Minimum | Maximum
IndexBounds = Reject | Clamp | Wrap
ReduceOperator = Sum | Product | Minimum | Maximum | Any | All
ReduceResult = Value | Index | ValueAndIndex
ScanMode = Inclusive | Exclusive { identity: ScalarLiteral }
ScatterConflict = UniqueIndices | Atomic { operation: AtomicOperation, ordering: AtomicOrdering }
SortDirection = Ascending | Descending
RandomDistribution = UniformF32 | NormalF32
                     | BernoulliI32 { probability_bits: u32 }
                     | UniformI32 { low: i32, high_exclusive: i32 }
PrimitiveKind = Elementwise(Elementwise) | Reduce(Reduce) | Scan(Scan)
              | Contraction(Contraction) | Gather(Gather) | Scatter(Scatter)
              | Histogram(Histogram) | Sort(Sort) | IndexMap(IndexMap)
              | Random(RandomMap)
```

The specification records are all public-field `Clone + Debug + Eq` values:

| Record | Fields and meaning |
| --- | --- |
| `Elementwise` | `program: ScalarProgram`, one scalar map applied over broadcast inputs |
| `Reduce` | `operator`, `axes: AxisSet`, `keep_dimensions`, `result`, `tree_lanes`; the power-of-two tree fixes floating-point operation order |
| `Scan` | `operator`, `axis`, `mode`, `reverse`, `tree_lanes` |
| `Contraction` | Ordered `batch_axes` and `contract_axes` pairs, with batch dimensions leading in the result |
| `Gather` | `axis`, `bounds` |
| `Scatter` | `axis`, `bounds`, `conflict` |
| `Histogram` | `bins`, `weighted`, `ordering` |
| `Sort` | `axis`, `direction`, `stable`, `emit_indices` |
| `IndexMap` | `start`, `element_step`, `iteration_step`, optional positive `modulus`; an int32 affine value over element and loop iteration |
| `RandomKey` | `seed_low`, `seed_high`, `stream`; key material, with runtime context folded in later |
| `RandomMap` | `distribution`, `key`, `philox_rounds`; current validation requires Recipe Philox4x32-10, exactly ten rounds |
| `PrimitiveAliasRule` | Input ordinal, output ordinal, and core `AliasPermission` |
| `PrimitiveKernel` | `id: KernelTemplateId`, ordered `inputs` and `outputs: Vec<ValueId>`, complete `alias_rules`, and one `PrimitiveKind` |
| `CalculationNode` | One public `kernel: PrimitiveKernel` wrapper used as a graph node |

`IndexMap` evaluates
`start + element_index * element_step + loop_iteration * iteration_step` in
checked signed-64 intermediates. A present modulus is strictly positive and
uses Euclidean remainder before the checked int32 result. `RandomMap` records
explicit deterministic distribution and key semantics; the runtime later adds
run ID, iteration, kernel ID, and element index.

`PrimitiveKernel::validate(&self, tensors)` first resolves every input and
output ID and requires a complete alias matrix: each input/output ordinal pair
must occur exactly once, with no out-of-range or duplicate rule. It then
dispatches to the one validator for the selected primitive kind:

| Kind | Contract checked by `validate` |
| --- | --- |
| Elementwise | Core scalar program validates; input/output arity equals scalar input/output count; at least one input; tensor dtypes match scalar inputs/outputs; input shapes broadcast to every output |
| Reduce | One input; one output except two for `ValueAndIndex`; axes in rank; tree lanes power of two in `1..=1024`; `Any`/`All` require int32; index results require `Minimum` or `Maximum`; reduced shapes and dtypes match; empty `Minimum`/`Maximum` domains are rejected |
| Scan | One input and output; axis in rank; fixed tree lanes; int32 `Any`/`All`; exclusive identity dtype equals input; output dtype and shape equal input |
| Contraction | Two inputs and one output; at least one contract pair; equal operand/output dtype; each pair in rank and uses each operand axis once; paired extents equal; output is batch extents then unused left and right extents, or `[1]` |
| Gather | Source plus int32 indices, one output; output dtype follows source and shape is `Shape::gather_result` |
| Scatter | Base, int32 indices, updates, one output; update and output dtypes follow base; output equals base shape; update shape is the base gather shape; conflict operation and ordering remain explicit |
| Histogram | One input or two when weighted; one output; bins in `1..=i32::MAX`; weighted weights are f32 and shape-equal; unweighted input/output are int32; output shape is `[bins]` |
| Sort | One input; one output or an additional index output; axis in rank and extent representable by int32; values preserve dtype/shape; emitted indices are int32 and shape-equal |
| IndexMap | Zero inputs and one int32 output; optional modulus strictly positive |
| Random | Zero inputs and one output; exactly ten Philox rounds; distribution determines f32 or i32 output; Bernoulli probability is finite and in `[0, 1]`; uniform int range has `low < high_exclusive` |

`PrimitiveKernel::work(&self, tensors)` validates first and returns a checked
core `FlopCount`. The estimate is elementwise instruction flops times output
elements, reduction combinations (doubled for value plus index), two passes
per scan element, two operations per contracted product, one per gather or
scatter output element, one per histogram input element, a bounded comparison
estimate for sort, zero for `IndexMap`, and four times Philox rounds per random
output element. Any checked arithmetic overflow is `WorkOverflow`. This is a
static planning estimate, not a device measurement.

### Calculation graph

`CalculationGraph` has public vectors:

```text
tensors: Vec<Tensor>
nodes: Vec<CalculationNode>
```

Its public methods are:

```rust
CalculationGraph::assemble(
    fragments: impl IntoIterator<Item = CalculationGraph>,
    external_inputs: impl IntoIterator<Item = ValueId>,
    external_outputs: impl IntoIterator<Item = ValueId>,
) -> LanguageResult<CalculationGraph>
graph.validate() -> LanguageResult<()>
graph.topological_order() -> LanguageResult<Vec<KernelTemplateId>>
graph.dependencies(kernel: KernelTemplateId) -> LanguageResult<Vec<KernelTemplateId>>
```

`assemble` merges independently materialized fragments. It rejects duplicate
boundary IDs, requires every boundary value to have a tensor declaration, and
allows repeated tensor declarations only when dtype, shape, layout, and
storage bytes are identical. Fragment-local boundary flags are discarded and
replaced by the supplied complete boundary sets. The result is sorted by value
ID and kernel ID, then validated.

`validate` builds a unique tensor index, validates every tensor and primitive
kernel, rejects duplicate kernel IDs, and requires exactly one producer for
each produced value. An external input cannot also be produced. Every
non-external tensor must have a producer. Finally it builds value-to-producer
edges and rejects self-dependencies and all cycles.

`topological_order` revalidates and runs a deterministic Kahn traversal with a
sorted ready set, so equal graphs produce equal kernel-ID order. `dependencies`
revalidates, rejects an absent kernel as `InvalidPrimitive`, and returns the
deduplicated, sorted producer IDs for the selected kernel's inputs.

The graph owns semantic edges only. It does not own iteration domains,
placement, physical copies, transfer routes, queues, or allocation.

### OGDL codec

`OgdlDocumentErrorKind` is `#[non_exhaustive]`:

```text
InvalidRoot, MissingField, DuplicateField, UnknownField, UnknownVariant,
InvalidNumber, InvalidBoolean, UnexpectedChildren, UnsupportedValue
```

`OgdlCodecError` is `#[non_exhaustive]` with these variants:

```text
Syntax(ParseError)
Document { kind: OgdlDocumentErrorKind, path: String, detail: String }
InvalidGraph(LanguageError)
Build(GraphError)
```

It implements `Display`, `Error`, and conversions from `ParseError`,
`GraphError`, and `LanguageError`. Syntax, invalid graph, and build errors
retain their source; document-shape errors are self-contained with a precise
path.

The codec methods added to `CalculationGraph` are:

```rust
graph.to_ogdl() -> Result<String, OgdlCodecError>
graph.to_ogdl_graph() -> Result<recipe_ogdl::Graph, OgdlCodecError>
CalculationGraph::from_ogdl(input: &str) -> Result<CalculationGraph, OgdlCodecError>
CalculationGraph::from_ogdl_graph(graph: &recipe_ogdl::Graph)
    -> Result<CalculationGraph, OgdlCodecError>
```

Encoding validates before constructing the ordered graph and returns its
canonical string. Decoding parses text first, then requires strict document
shape, decodes every value, and calls `CalculationGraph::validate`.

The exact schema is:

```text
RecipeIR
  schema CalculationGraph
  version 1
  tensors
    tensor
      id <u64>
      dtype F32|I32
      shape (extent <u64>)*
      layout
        offset_elements <u64>
        strides (stride <u64>)*
      storage_bytes <u64>
      external_input true|false
      external_output true|false
  nodes
    node
      kernel
        id <u64>
        inputs (value <u64>)*
        outputs (value <u64>)*
        alias_rules
          alias_rule
            input <usize>
            output <usize>
            permission Forbidden|MayAliasExact|MustAliasExact
        kind <one primitive variant>
```

The primitive variant fields are exactly these:

```text
Elementwise: program
Reduce: operator, axes(axis*), keep_dimensions, result, tree_lanes
Scan: operator, axis, mode(Inclusive | Exclusive(identity)), reverse, tree_lanes
Contraction: batch_axes(pair(left,right)*), contract_axes(pair(left,right)*)
Gather: axis, bounds(Reject|Clamp|Wrap)
Scatter: axis, bounds, conflict(UniqueIndices | Atomic(operation,ordering))
Histogram: bins, weighted, ordering
Sort: axis, direction(Ascending|Descending), stable, emit_indices
IndexMap: start, element_step, iteration_step, modulus(None | Some(value))
Random: distribution, key(seed_low,seed_high,stream), philox_rounds
```

An elementwise `program` contains `inputs(input(id,dtype)*)`,
`constants(constant(id,literal)*)`, `instructions(instruction(result,dtype,
opcode,operands(value)* )*)`, and `outputs(value*)`. Literals are exactly
`F32Bits(<u32>)` or `I32(<i32>)`. The codec currently spells these scalar
opcodes exactly:

```text
Add, Subtract, Multiply, Divide, Remainder, Negate, Absolute, Minimum,
Maximum, Fma, Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan,
GreaterThanOrEqual, Select, BitAnd, BitOr, BitXor, BitNot, BitcastF32ToI32,
BitcastI32ToF32, ShiftLeft, ShiftRightLogical, ShiftRightArithmetic, Require,
IsFinite, IsNan, SquareRoot, Floor, Ceiling, RoundNearestEven,
ConvertF32ToI32, ConvertI32ToF32
```

An opcode not represented by this version is a deterministic `UnsupportedValue`
error rather than a fallback spelling.

Strictness is structural as well as semantic:

* exactly one `RecipeIR` root is required;
* `schema` must be `CalculationGraph` and `version` must be `1`;
* every record has the listed fields exactly once;
* unknown fields and wrong collection item names are rejected;
* variant/value nodes require exactly one child, while leaf values require no
  children;
* numbers must parse in range and booleans must be exactly `true` or `false`;
* enum names are case-sensitive and unknown names are rejected;
* decoded axes, shapes, layouts, scalar programs, primitives, and graph edges
  still pass their normal language validators.

## Ownership and downstream consumers

The crate's ownership can be read from the dependency graph and import sites.
The direct workspace dependents are the root package plus `kernel`, `math`,
`ops`, `planner`, `prepare`, `primitives`, `program`, and `training`. Their
roles are deliberately distinct:

| Consumer | Evidence in source | What it owns around the language contract |
| --- | --- | --- |
| Root `recipe` | `src/facade.rs` reexports `recipe_language` as `recipe::engine::language`; `src/training.rs` consumes `CalculationGraph` | Public facade and orchestration; no alternate language implementation |
| `recipe-math` | `math/src/program.rs`, `math/src/lib.rs` | Reusable Recipe-owned scalar math programs, built with `ScalarProgramBuilder`; it does not own scalar opcode signatures |
| `recipe-ops` | `ops/src/primitive.rs`, `bayes.rs`, `binary_metrics.rs`, `kmeans.rs`, `knn_outputs.rs`, `tree.rs`, `materialize.rs`, and `materialize/*` | Operation registry, graph fragments, scalar maps, and primitive requests; it constructs language records and leaves lowering to primitives/planner |
| `recipe-training` | `training/src/compile.rs`, `inference.rs`, `forward.rs`, `model.rs`, `gguf_llama.rs`, `execute.rs`, and `error.rs` | Dense training and inference graph construction, scalar programs, model graph access, and OGDL error propagation; it owns model policy, not graph invariants |
| `recipe-program` | `program/src/lib.rs` | Wraps the graph in explicit loop iteration domains and metric declarations; it calls language validation and embeds the `RecipeIR` subtree in program OGDL |
| `recipe-primitives` | `primitives/src/lower.rs`, `validate.rs`, `model.rs`, `hash.rs`, `error.rs` | Checks and lowers each validated `PrimitiveKernel` into backend-neutral stages, geometry, views, synchronization, atomics, faults, and resource bounds; it does not mutate the language graph |
| `recipe-kernel` | `kernel/src/stage.rs` | Realizes a lowered primitive stage into target LLVM/native-kernel contracts after the language and primitive boundaries have been checked |
| `recipe-planner` | `planner/src/planner.rs` | Reads `CalculationGraph`, `CalculationNode`, and `Tensor` to select placements, copies, resources, and schedule; graph ordering and primitive semantics remain language-owned |
| `recipe-prepare` | `prepare/src/lib.rs`, `prepare/src/production.rs` | Uses the graph as input to fixed-point preparation and immutable target realization; it does not add graph semantics |

Other crates reach the language surface through these dependents. Executors
consume finalized core plans and native artifacts, not language declarations.
No consumer is permitted to add a backend string, vendor operation call,
placement field, transfer edge, queue, or lifecycle state to a language record.

### Construction and consumption flow

The normal real path is:

1. `recipe-ops` and `recipe-training` create `Shape`, `Tensor`, scalar
   programs, primitive specs, alias matrices, and `CalculationNode` values.
2. They assemble a `CalculationGraph` with explicit external input/output
   sets and call `validate` or a method that validates it.
3. `recipe-program` assigns loop domains without unrolling or changing the
   calculation graph.
4. `recipe-primitives` validates each kernel against its tensor map, computes
   static work, and lowers it into immutable backend-neutral stages.
5. `recipe-planner` uses graph dependencies and tensor metadata while adding
   placement, copies, resource reservations, and schedule records in core.
6. `recipe-prepare` and `recipe-kernel` realize target artifacts. Execution
   crates consume the finalized core bundle.
7. When a semantic graph crosses a file boundary, the language codec emits or
   reads only the canonical `RecipeIR`/`CalculationGraph` document described
   above. Program OGDL wraps that subtree but does not redefine its fields.

This ordering is an ownership boundary, not a suggestion to duplicate checks.
Callers rely on language validation for shape, layout, scalar, primitive, and
graph invariants, and downstream crates add only the invariants of their own
stage.

## Semantic boundaries to preserve

The following are the non-negotiable meanings encoded by this facade:

* **Backend-neutral payloads.** Only f32 and int32 dtypes enter this language
  through the core types used by tensor and scalar validation. GPU backend
  selection and native lowering happen later.
* **Static shapes and layouts.** Shape extents, strides, offsets, and backing
  bytes are checked metadata. They are not runtime pointers, allocations, or
  transfers.
* **Explicit boundaries.** External input/output flags are part of the graph
  contract. `assemble` replaces fragment-local flags with the caller's exact
  sets and rejects unknown boundary IDs.
* **Acyclic calculations.** Every non-input tensor has exactly one producer;
  producer edges define a deterministic DAG. Loop repetition belongs to
  `recipe-program`, not to `CalculationGraph`.
* **Explicit aliasing.** Every input/output ordinal pair has exactly one
  `PrimitiveAliasRule`. The permission is core-owned and is consumed by later
  lowering and planning; absence is not interpreted as permission.
* **Deterministic scalar identity.** Builder ownership prevents cross-program
  references, call order fixes instruction order, and `finish` delegates the
  complete SSA proof to the core program validator.
* **Stable primitive semantics.** Reduction trees, scan modes, scatter conflict
  operations and orderings, index bounds, sort options, affine iteration
  mapping, random distributions, key material, and Philox round count are
  explicit artifact data. A backend cannot silently substitute another choice.
* **Strict persistence.** Version 1 OGDL has one schema, one root, exact
  fields, exact variants, exact booleans, and no defaults. New schema behavior
  must be an explicit versioned change, not a decoder fallback.
* **Validation before consumption.** Encoding, graph ordering, primitive work,
  program wrapping, and downstream lowering all begin from validated language
  records. A structural success or a status string alone is not execution
  evidence; the language layer proves only the declaration contract.

When changing this crate, keep the root reexports, module ownership, and the
typed graph/OGDL boundary synchronized. A new operation must be represented by
one `PrimitiveKind` variant, one validator, one codec spelling, and the
downstream lowering path that consumes it. It must not be introduced as a
backend-only branch, an unvalidated graph node, a duplicate wrapper, or a
silent compatibility default.
