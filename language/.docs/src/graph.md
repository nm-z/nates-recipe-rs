# `language/src/graph.rs`

## Intent

`graph.rs` defines Recipe's typed, backend-neutral calculation graph. A graph
is the static model boundary between operation and training compilers and
the later primitive lowering, placement, transfer, and execution layers. It
describes values, their static tensor contracts, and placement-free primitive
calculations. It does not describe a device, a queue, a transfer, a native
artifact, an iteration domain, or a run lifecycle.

The public language facade re-exports [`CalculationGraph`] and
[`CalculationNode`] from [`language/src/lib.rs`](../../src/lib.rs). The graph's
canonical text form is the versioned `RecipeIR` OGDL document implemented in
[`ogdl.rs`](../../src/ogdl.rs). Every producer of a graph is expected to make
`validate()` the semantic gate before handing the graph to a program, planner,
or materializer.

The graph is a value-flow DAG:

```text
Tensor(ValueId) --input--> PrimitiveKernel --output--> Tensor(ValueId)
                                  |
                                  +--> CalculationNode
```

An edge exists when a kernel consumes a value produced by another kernel. An
external input has no calculation producer and enters through the run
boundary. An external output is a value that the caller may read after the
calculation graph runs. A value may be both an external input and an external
output, which represents a legal pass-through boundary value.

## Scope and structure

The module contains two public structs and one private helper layer:

| Item | Fields | Purpose |
| --- | --- | --- |
| `CalculationNode` (`graph.rs:7-10`) | `kernel: PrimitiveKernel` | The graph's node wrapper. The wrapper keeps the graph representation uniform and leaves primitive semantics in `primitive.rs`. |
| `CalculationGraph` (`graph.rs:12-16`) | `tensors: Vec<Tensor>`, `nodes: Vec<CalculationNode>` | The complete value and calculation declaration. Tensors are indexed by `ValueId`; nodes are identified by `PrimitiveKernel.id`. |
| `same_storage_contract` (`graph.rs:282-288`) | `id`, dtype, shape, layout, storage bytes | The equality relation used when independently materialized fragments repeat a tensor declaration. Boundary flags are deliberately excluded. |

`CalculationGraph` derives `Clone`, `Debug`, `PartialEq`, and `Eq`. The fields
are public so compilers and materializers can construct and append declarations,
but construction itself performs no validation. Current graph producers call
`validate()` before exposing their result to a program, planner, materializer,
or execution boundary.

### Tensor records

Each [`Tensor`](../../src/tensor.rs) in `tensors` contains:

- `id: ValueId`, the stable value identity referenced by every kernel input and
  output;
- `dtype: DType`, currently the language's `F32` or `I32` payload type;
- `shape: Shape`, a nonempty fixed-rank extent vector whose element count is
  checked for `u64` overflow;
- `layout: TensorLayout`, an element offset and one stride per shape axis;
- `storage_bytes: ByteCount`, the logical backing allocation size before
  placement;
- `external_input`, which says that the value is supplied by the run boundary
  and therefore must not have a calculation producer;
- `external_output`, which says that the value is part of the caller-visible
  result boundary.

`Tensor::validate()` checks the layout against the shape, computes the highest
addressed element span, multiplies that span by the dtype width, and rejects a
span larger than `storage_bytes`. `Shape`, layout, and byte-size failures are
returned as `LanguageError` values carrying the tensor's `ValueId` where the
tensor layer has enough context. Empty extents are valid and produce zero
elements. They are later treated by planning as a no-dispatch calculation, not
as an invented one-lane payload.

### Kernel records

Each node contains a [`PrimitiveKernel`](../../src/primitive.rs):

| Field | Meaning |
| --- | --- |
| `id: KernelTemplateId` | Stable identity of this calculation node. It is the key used by topological order, dependencies, iteration-domain assignments, planner identities, and lowered stage provenance. |
| `inputs: Vec<ValueId>` | Ordered logical input positions. Each ID must name a declared tensor. An input whose tensor is externally supplied contributes no graph edge; an input produced by a kernel contributes an edge from that producer. |
| `outputs: Vec<ValueId>` | Ordered logical output positions. Every output must name a declared tensor, and each value may have exactly one producer across the complete graph. |
| `alias_rules: Vec<PrimitiveAliasRule>` | One explicit permission for every input/output index pair. The primitive validator requires a complete, nonduplicated matrix, even when every pair is `Forbidden`. |
| `kind: PrimitiveKind` | One Recipe-owned calculation family and its typed parameters. The graph does not reinterpret the family or lower it itself. |

The primitive layer validates input and output tensor existence, alias-matrix
shape, scalar SSA programs, arity, dtype, shape, axes, bounds, deterministic
reduction choices, and work arithmetic. The graph validator invokes that
layer for every node, so a graph's semantic validity includes the full
`PrimitiveKernel` contract.

## `CalculationGraph::assemble`

`assemble` (`graph.rs:18-76`) combines independently materialized graph
fragments into one canonical graph. It is a general graph operation, not an
operation-specific lowering path. Current in-tree operation appenders perform
their own boundary and identity checks and do not call this method directly;
the method remains the reusable contract for callers that have complete graph
fragments.

The exact data flow is:

```text
fragment graphs + external input IDs + external output IDs
    -> unique_boundary("external input")
    -> unique_boundary("external output")
    -> merge tensor contracts by ValueId
    -> concatenate all CalculationNode values
    -> check every requested boundary ID exists
    -> overwrite every tensor's boundary flags from the two requested sets
    -> sort tensors by ValueId and nodes by KernelTemplateId
    -> validate the assembled graph
    -> canonical CalculationGraph
```

### Boundary sets

`unique_boundary` (`graph.rs:268-280`) inserts each `ValueId` into a
`BTreeSet`. Repeating a value within one boundary set returns
`LanguageErrorKind::DuplicateTensor` and attaches the value ID. The input and
output sets are checked independently, so one value may intentionally appear
in both sets. That is how an external pass-through value is represented.

### Tensor merge

Fragments are consumed one at a time. Every tensor is indexed in a
`BTreeMap<ValueId, Tensor>`:

1. The first declaration for an ID is retained.
2. A later declaration with the same ID is accepted only when
   `same_storage_contract` agrees on ID, dtype, shape, layout, and
   `storage_bytes`.
3. A mismatch returns `DuplicateTensor` with the conflicting value ID and the
   detail `fragments declare different storage contracts for the same tensor`.

The fragment-local `external_input` and `external_output` flags are ignored
while merging. They describe temporary fragment boundaries, not the boundary
of the completed run. After all fragments are consumed, the caller-supplied
sets overwrite both flags for every retained tensor. A repeated tensor can
therefore differ in boundary flags and still merge, but it cannot differ in
any storage-defining field.

Nodes are appended without deduplication. A duplicate kernel identity, a
duplicate output producer, an absent tensor, or a cycle is intentionally left
for the final graph validator to report. This keeps assembly's merge rule
small and makes the complete graph validator the single semantic gate.

### Canonical ordering and postconditions

Boundary IDs must all be present in the merged tensor map. A missing boundary
returns `UnknownTensor`. On success, tensors are sorted by `tensor.id` and
nodes by `node.kernel.id`, independent of fragment order. The method then
constructs a graph, calls `validate()`, and returns it only if every tensor,
kernel, producer relation, and dependency relation is valid.

An empty fragment iterator with no boundaries produces an empty graph, which
passes the current validator. Any nonempty boundary list still fails because
the referenced tensor is absent. Assembly does not invent tensors or kernels.

## `CalculationGraph::validate`

`validate` (`graph.rs:78-138`) is fail-fast and ordered. The first error in this
sequence is the returned error; later checks are not attempted.

```text
tensor_index
    -> validate every Tensor
    -> index unique kernel IDs and validate every PrimitiveKernel
    -> build one producer map for all kernel outputs
    -> check external-input and producer consistency
    -> run deterministic DAG validation
```

### 1. Build the tensor index

`tensor_index` (`graph.rs:189-201`) inserts every tensor into a
`BTreeMap<ValueId, &Tensor>`. A repeated ID returns
`DuplicateTensor` with that value. The map gives the primitive validator a
single lookup source and makes every subsequent tensor lookup deterministic.

### 2. Validate tensor contracts

Each indexed tensor calls `Tensor::validate()`. This includes:

- one stride for each shape axis;
- no zero stride on a non-singleton axis;
- non-overlapping logical elements under the declared strides;
- checked layout span arithmetic;
- checked conversion of the span to bytes;
- `storage_bytes` large enough for the addressed span.

The graph does not recalculate or normalize layouts. A layout that is valid for
its tensor is retained exactly and is later copied into lowering buffer views.

### 3. Validate kernel identity and primitive semantics

The validator inserts each `node.kernel.id` into a `BTreeSet`. A repeated
identity returns `DuplicateKernel`, reports the kernel ID, and stops.

Then `PrimitiveKernel::validate(&tensors)` is called. It first resolves every
input and output ID. An absent one returns `UnknownTensor` with both the value
and kernel context. It checks the complete alias matrix, then dispatches to the
kind-specific validator. The graph therefore rejects a node whose tensor IDs
exist but whose calculation contract is impossible.

The kind-specific checks are summarized here because they are part of the
graph's validation boundary:

| Kind | Required semantic checks |
| --- | --- |
| `Elementwise` | The scalar program is valid; tensor input and output arities equal the scalar program; there is at least one tensor input; input dtypes equal scalar input dtypes; input shapes broadcast; scalar output dtypes and broadcast result shapes match output tensors. |
| `Reduce` | One input; one or two outputs according to `ReduceResult`; `tree_lanes` is a power of two in `1..=1024`; axes are in rank; `Any` and `All` use `I32`; index results require `Minimum` or `Maximum`; reduced shape, value dtype, index dtype, and empty-domain rules agree. |
| `Scan` | One input and one output; axis is in rank; the tree lane count has the same fixed range; `Any` and `All` use `I32`; an exclusive identity has the input dtype; output dtype and shape equal input. |
| `Contraction` | Exactly two inputs and one output; at least one contracted pair; operand and output dtypes agree; every batch and contracted axis is in rank and used once per side; paired extents match; output order is batch extents, left free extents, then right free extents, with `[1]` for a scalar result. |
| `Gather` | Two inputs and one output; index input is `I32`; output dtype equals values input; `Shape::gather_result(axis, indices_shape)` equals output shape. |
| `Scatter` | Three inputs and one output; index input is `I32`; update and base dtypes agree; output matches base dtype and shape; update shape equals the base shape with the selected axis gathered by the index shape. The declared bounds and conflict operation/order remain exact lowering requirements. |
| `Histogram` | One input when unweighted or two when weighted; one output; bins are in `1..=i32::MAX`; weighted values are `F32` and shape-match the index input; output is `I32` when unweighted or `F32` when weighted and has shape `[bins]`. |
| `Sort` | One input; one output unless `emit_indices`, which requires two; axis is in rank and fits `I32` result indices; values preserve dtype and shape; optional indices are `I32` with the input shape. |
| `IndexMap` | No inputs and one output; output is `I32`; a present modulus is strictly positive. Its affine expression uses checked signed-64 intermediates and a positive Euclidean modulus when supplied. |
| `Random` | No inputs and one output; exactly ten Philox rounds; `UniformF32` and `NormalF32` output `F32`; Bernoulli probability bits decode to finite `F32` in `[0,1]` and output `I32`; uniform integer ranges require `low < high_exclusive` and output `I32`. |

These checks return `ArityMismatch`, `DTypeMismatch`, `ShapeMismatch`,
`InvalidAxis`, `DuplicateAxis`, `InvalidScalarProgram`, or
`InvalidPrimitive` as appropriate. Checked work arithmetic is exposed by
`PrimitiveKernel::work`; it first repeats primitive validation and returns
`WorkOverflow` when its static FLOP estimate cannot fit. The planner uses that
work after graph validation when it builds the graph identity and candidate
plans.

### 4. Construct the producer map

For every kernel output, `validate` inserts `output ValueId -> kernel ID` into
one `BTreeMap`. A value already in the map returns `DuplicateProducer`, with
the new kernel and value context and a detail naming both producer IDs. This
also catches a kernel that repeats the same value in its own `outputs` vector:
the first insertion succeeds and the second insertion fails.

There is no implicit last-writer rule. A value must have exactly zero or one
calculation producer, and zero is legal only for an external input.

### 5. Enforce boundary producer rules

The validator examines every tensor against `external_input` and the producer
map:

| `external_input` | Producer | Result |
| --- | --- | --- |
| `true` | present | `DuplicateProducer`: an external input cannot also be calculated. |
| `true` | absent | Valid run-boundary input. |
| `false` | present | Valid calculated value, whether or not it is an external output. |
| `false` | absent | `MissingProducer`: an internal or output-only tensor has no calculation source. |

`external_output` is not an independent producer rule. An output-only tensor
must be produced because it is not an external input. A tensor marked both
external input and external output may have no producer and is valid as a
pass-through boundary value.

The validator does not require every producer to feed an external output, does
not remove unreachable nodes, and does not infer or repair missing flags.

### 6. Enforce acyclicity

`topological_order_from` builds the kernel dependency relation from the
producer map. For each kernel input that is produced by another kernel, it
adds one directed edge `producer -> consumer`. `BTreeSet<(KernelTemplateId,
KernelTemplateId)>` removes duplicate edges when a consumer reads multiple
outputs from the same producer or repeats an input ID.

A self-edge is reported immediately as `Cycle` with the consuming kernel
context and detail `kernel consumes its own output value`. Other cycles leave
one or more nonzero indegrees after Kahn's algorithm and return `Cycle` with
detail `calculation graph contains a cycle`. External inputs do not appear in
the dependency relation.

The graph validator calls this routine only for its success/failure result and
does not retain the order. Callers that need the order use
`topological_order()`.

## Dependency queries

### `topological_order`

`topological_order` (`graph.rs:140-153`) first calls `validate()`, rebuilds the
producer map, and returns the stable kernel order from
`topological_order_from`. The implementation is deterministic:

1. indegrees are indexed in a `BTreeMap` by kernel ID;
2. successor vectors are sorted by kernel ID;
3. all currently ready kernels are stored in a `BTreeSet`;
4. `pop_first()` selects the smallest ready ID;
5. newly zero-indegree successors are inserted into the ready set.

Thus independent kernels are ordered by their `KernelTemplateId`, not by
fragment insertion order, hash order, or an arbitrary queue. The returned
vector contains every node exactly once. A graph error is returned before any
partial order is exposed.

### `dependencies`

`dependencies(kernel)` (`graph.rs:155-187`) also validates first. It then finds
the node with the requested ID. If no node exists, it returns
`LanguageErrorKind::InvalidPrimitive` with `for_kernel(kernel)` and detail
`kernel {kernel} is absent`.

For an existing node, it maps each input value through the producer map,
discards external inputs, sorts producer IDs numerically, and deduplicates
them. The result is the set of direct calculation predecessors, not a
transitive closure and not a schedule. It is empty for a source kernel. The
method does not include alias, transfer, phase, or iteration-domain edges;
those are added by later planner layers.

Both query methods deliberately repeat complete graph validation. A caller
cannot obtain a topological order or a dependency list from a graph that has
changed into an invalid state.

## Construction and builder paths

The graph module does not expose a graph-specific builder. Producers construct
`Tensor` and `PrimitiveKernel` values directly, then validate the completed
graph. The following builders are the current in-tree paths.

### Scalar programs used by elementwise nodes

[`ScalarProgramBuilder`](../../src/scalar_builder.rs) is the builder beneath
`PrimitiveKind::Elementwise`:

- `new()` allocates an owner identity and starts scalar IDs at one;
- `input()` and `constant()` append typed declarations;
- `apply()` checks that every `ScalarExpression` belongs to this builder and
  that the opcode accepts the operand dtypes;
- `finish()` checks output ownership, constructs `ScalarProgram`, and calls
  scalar-program validation.

Instruction order is call order and is therefore part of artifact identity.
Training, inference, and operation materializers use this builder to produce
the scalar SSA program stored in an elementwise `PrimitiveKernel`. Graph
validation invokes `ScalarProgram::validate()` again through
`PrimitiveKernel::validate`; a builder-created program is not an exemption
from the graph contract.

### Dense training compiler

`training/src/compile.rs` defines `GraphCompiler` (`compile.rs:2605-2620`). It
owns a `BTreeMap<ValueId, Tensor>`, a `Vec<CalculationNode>`, kernel iteration
domains, identity counters, and the eventual external-boundary sets.

Its graph-relevant path is:

```text
GraphCompiler::tensor / next_value
    -> operation-specific PrimitiveKind
    -> GraphCompiler::emit
       (allocate KernelTemplateId, push PrimitiveKernel and domain)
    -> insert_materialized_graph
       (copy tensor contracts with boundary flags cleared, append nodes)
    -> finish
       (apply external flags, construct CalculationGraph)
    -> graph.validate()
    -> graph.to_ogdl() -> CalculationGraph::from_ogdl()
    -> StaticCalculationProgram::new_with_metrics
```

`insert_materialized_graph` rejects a materialized tensor that disagrees with
an existing dtype, shape, layout, or storage contract. It clears the fragment
boundary flags before insertion because the dense compiler owns the complete
run boundary. `finish` applies the final input and output sets, validates, and
performs a canonical OGDL round trip before exposing the graph in
`CompiledTraining`. This round trip checks both typed semantic validity and
the stable textual representation used by later digests and artifacts.

### Inference compilers

`training/src/inference.rs` has analogous `InferenceGraphCompiler` paths for
dense inference, categorical Bayesian inference, and KNN inference. The
compiler emits nodes with fresh kernel IDs, marks declared external inputs and
outputs after materialization, validates the resulting graph, and in the
normal dense and KNN paths performs a graph OGDL round trip before constructing
the `StaticCalculationProgram`.

The categorical Bayes and KNN output operations append independently
materialized nodes to a temporary graph. They reserve disjoint value and
kernel identity ranges, set final boundary flags before appending, and then
validate the complete graph. `CompiledInference` and
`CompiledKnnInference` expose the resulting graph only through their static
program (`CompiledInference::graph()` and `CompiledKnnInference::graph()`).

### Operation materializers

`ops/src/materialize.rs` uses a private `GraphBuilder`. It copies caller
boundary tensors, reserves a disjoint identity namespace, creates each
intermediate with `Tensor::contiguous(..., false, false)`, and emits a
`PrimitiveKernel` with a fresh ID and a complete forbidden-alias matrix. Its
`finish` constructs `CalculationGraph { tensors, nodes }` and immediately
validates it before returning `MaterializedComposition`.

The concrete operation modules (`bayes.rs`, `binary_metrics.rs`, `kmeans.rs`,
`knn_outputs.rs`, and `tree.rs`) follow the same shape. Their emitters track
intermediate values, kernels, workspace bytes, and reserved identity ranges;
they fail if the emitted counts or workspace do not match the precomputed
requirements, then call `graph.validate()`.

The public `append_*` operations validate that the caller graph already
contains every requested boundary tensor with the same storage contract. They
reject repeated tensor or kernel identities, boundary outputs that already
have a producer, and identity-namespace overlap. On success they append only
intermediate tensor declarations and materialized nodes. They leave caller
boundary flags alone, and the caller validates the whole graph after all
fragments are appended. These appenders are intentionally explicit rather
than silently relying on a last-writer merge rule.

There are no current in-tree calls to `CalculationGraph::assemble`; the
operation appenders implement their narrower boundary-aware append contracts
because they also have to return operation-specific workspace and identity
evidence.

## Program, planner, and lowering consumers

The graph is the model input to the rest of the runtime. Its consumers retain
the distinction between calculation semantics and operational scheduling.

### `StaticCalculationProgram`

`program/src/lib.rs` wraps one graph with loop iterations, one iteration domain
per kernel, and optional metric emissions. `StaticCalculationProgram::validate`
calls `graph.validate()` first, then checks that every graph kernel has exactly
one domain, every domain lies within the loop, producer domains do not start
after their consumer, and metric values are valid produced four-byte internal
scalars. Graph acyclicity remains the graph module's responsibility; program
validation adds time-domain constraints.

`StaticCalculationProgram::to_ogdl_graph` serializes a `RecipeProgram` root and
copies the graph's `RecipeIR` subtree as a second root. `from_ogdl_graph`
requires exactly those two roots, parses program fields and domains, copies the
`RecipeIR` subtree into a temporary OGDL graph, and delegates its semantic
decode to `CalculationGraph::from_ogdl_graph`.

### Primitive lowering

`primitives/src/lower.rs::lower` receives one `PrimitiveKernel`, the graph's
tensor index, and measured lowering hardware. It validates the hardware,
calls `kernel.validate(tensors)` again, registers tensor buffers preserving
shape, offset, strides, dtype, and storage bytes, lowers the selected
`PrimitiveKind` into one or more stages, carries the source alias matrix into
the lowered program, computes resources, and validates the lowered program.

Lowering may add scratch and fault buffers and stage dependencies, but those
are implementation artifacts of the primitive lowering. They do not become
new graph nodes or new model semantics.

### Planner

`planner/src/planner.rs::plan_program_candidates` validates the static program
and graph, requests `graph.topological_order()`, indexes graph tensors and
nodes by their stable IDs, and calls `recipe_primitives::lower` for every
node. It uses the returned lowered programs to enumerate measured placement
and artifact choices. The planner's graph digest incorporates tensor contracts,
boundary flags, the stable kernel order, kernel input and output IDs, static
work, iteration domains, lowered-program digests, and stage identities.

The planner then expands each primitive into calculation stages, value
materialization, external input admission, output egress, fault readbacks,
metrics, and transfer tasks. It adds task dependencies for stage order, graph
producer order, exact aliases, and transfer routes. These task edges are
downstream operational relations; `CalculationGraph::dependencies` supplies
only the direct producer relation among model kernels.

### Preparation and execution boundaries

`prepare/src/lib.rs::Preparer::prepare` wraps a cloned graph in a one-iteration
`StaticCalculationProgram`; `prepare/src/production.rs::NativeArtifactProvider`
validates the graph before accepting a native artifact catalog. Preparation
does not mutate graph semantics.

`training/src/execute.rs` validates the graph at compiled dense and KNN
inference boundaries before checking declared external input roles, byte
sizes, output contracts, and the one-iteration execution boundary. The
executor consumes the validated graph through the compiled program and does
not infer graph state from host buffers.

`training/src/checkpoint.rs::compiled_training_program_digest` hashes the
canonical static-program OGDL text. Because the program embeds canonical
`RecipeIR`, graph tensor contracts, primitive parameters, IDs, and ordering
are part of the native training artifact identity. Checkpoint admission keeps
the compiled graph unchanged.

Two narrower access paths do not reinterpret the graph. `CompiledTraining::graph`
and the compiled inference accessors return the graph held by their static
program. `src/training.rs::derive_native_runtime_tuning` uses only the number
of declared graph tensors as one input to host staging and worker-lane tuning;
it receives an already compiled graph and does not add nodes, infer producers,
or change tensor contracts.

## Canonical OGDL representation

The graph codec in [`ogdl.rs`](../../src/ogdl.rs) uses these exact constants:

```text
root:    RecipeIR
schema:  CalculationGraph
version: 1
```

The top-level record has exactly four fields: `schema`, `version`, `tensors`,
and `nodes`. The following shape is the complete record structure. Collection
items are repeated in source order; scalar fields contain one leaf child.

```text
RecipeIR
    schema
        CalculationGraph
    version
        1
    tensors
        tensor*
            id
                <u64>
            dtype
                F32 | I32
            shape
                extent*
                    <u64>
            layout
                offset_elements
                    <u64>
                strides
                    stride*
                        <u64>
            storage_bytes
                <u64>
            external_input
                true | false
            external_output
                true | false
    nodes
        node*
            kernel
                id
                    <u64>
                inputs
                    value*
                        <u64>
                outputs
                    value*
                        <u64>
                alias_rules
                    alias_rule*
                        input
                            <usize>
                        output
                            <usize>
                        permission
                            Forbidden | MayAliasExact | MustAliasExact
                kind
                    <exactly one PrimitiveKind variant record>
```

`to_ogdl_graph()` calls `validate()` before encoding. It emits tensors and
nodes in the order held by the graph; `assemble` and the main compiler finish
paths sort those collections before encoding when canonical order is needed.
`to_ogdl()` then calls `to_canonical_string()` on the ordered OGDL graph.

### Primitive variant records

`kind` contains exactly one of these case-sensitive variants:

| Variant | Fields and nested variants |
| --- | --- |
| `Elementwise` | `program`, containing `inputs` (`input: id, dtype`), `constants` (`constant: id, literal`), `instructions` (`instruction: result, dtype, opcode, operands/value*`), and `outputs/value*`. Literals are `F32Bits` or `I32`. |
| `Reduce` | `operator` (`Sum`, `Product`, `Minimum`, `Maximum`, `Any`, `All`), `axes/axis*`, `keep_dimensions`, `result` (`Value`, `Index`, `ValueAndIndex`), `tree_lanes`. |
| `Scan` | `operator`, `axis`, `mode` (`Inclusive` or `Exclusive/identity`), `reverse`, `tree_lanes`. |
| `Contraction` | `batch_axes/pair*` and `contract_axes/pair*`; every `pair` has `left` and `right`. |
| `Gather` | `axis`, `bounds` (`Reject`, `Clamp`, `Wrap`). |
| `Scatter` | `axis`, `bounds`, and `conflict` (`UniqueIndices` or `Atomic/operation/ordering`). Atomic operations are `Exchange`, `Add`, `Minimum`, `Maximum`; orderings are `Relaxed`, `Acquire`, `Release`, `AcquireRelease`, `SequentiallyConsistent`. |
| `Histogram` | `bins`, `weighted`, `ordering`. |
| `Sort` | `axis`, `direction` (`Ascending` or `Descending`), `stable`, `emit_indices`. |
| `IndexMap` | `start`, `element_step`, `iteration_step`, `modulus` (`None` or `Some/<i32>`). |
| `Random` | `distribution` (`UniformF32`, `NormalF32`, `BernoulliI32/probability_bits`, or `UniformI32/low/high_exclusive`), `key/seed_low/seed_high/stream`, and `philox_rounds`. |

Scalar opcodes are encoded by their exact names. Schema v1 includes
`Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`, `Negate`, `Absolute`,
`Minimum`, `Maximum`, `Fma`, `Equal`, `NotEqual`, `LessThan`,
`LessThanOrEqual`, `GreaterThan`, `GreaterThanOrEqual`, `Select`, `BitAnd`,
`BitOr`, `BitXor`, `BitNot`, `BitcastF32ToI32`, `BitcastI32ToF32`,
`ShiftLeft`, `ShiftRightLogical`, `ShiftRightArithmetic`, `Require`,
`IsFinite`, `IsNan`, `SquareRoot`, `Floor`, `Ceiling`, `RoundNearestEven`,
`ConvertF32ToI32`, and `ConvertI32ToF32`. The encoder returns
`UnsupportedValue` if a `ScalarOpcode` has no schema-v1 spelling. The decoder
rejects every unknown spelling rather than silently choosing a default.

### Strict decode order

`from_ogdl(input)` parses OGDL syntax with `recipe_ogdl::Graph::parse` and then
calls `from_ogdl_graph`. The strict decoder performs these checks before graph
validation:

1. exactly one root exists and its text is `RecipeIR`;
2. the root has exactly the required fields, with no missing, duplicate, or
   unknown field;
3. schema is exactly `CalculationGraph` and version is exactly `1`;
4. every collection child has its exact item name (`tensor`, `node`, `value`,
   `pair`, `input`, `constant`, or `instruction`);
5. every scalar field has exactly one leaf child;
6. every enum variant has exactly one variant child and the exact spelling;
7. booleans are exactly `true` or `false`;
8. integers parse as the destination type and fit its range.

Only after these document checks does the decoder construct
`CalculationGraph { tensors, nodes }` and call `graph.validate()`. Thus a
syntactically well-formed document may still fail as `OgdlCodecError::InvalidGraph`
when its tensor contracts, primitive parameters, producer map, or dependency
DAG is invalid.

`OgdlCodecError` preserves four failure boundaries:

| Variant | Origin |
| --- | --- |
| `Syntax(ParseError)` | The OGDL parser could not read the input text. |
| `Document { kind, path, detail }` | The ordered graph has the wrong root, field, item, enum, number, boolean, or child shape. `path` identifies the exact record location. |
| `InvalidGraph(LanguageError)` | Document decoding succeeded, but the typed graph validator rejected the result. |
| `Build(GraphError)` | Encoding could not construct the ordered OGDL graph. |

No field or enum receives a default during decoding. This is what makes the
OGDL form a stable semantic artifact rather than a permissive interchange
format.

## Invariants and failure matrix

The following table is the compact contract for callers constructing or
consuming a graph:

| Invariant | Enforced by | Failure |
| --- | --- | --- |
| Tensor IDs are unique | `tensor_index` and assembly merge | `DuplicateTensor` |
| Repeated fragment tensors have identical dtype, shape, layout, and storage | `same_storage_contract` | `DuplicateTensor` |
| Boundary IDs are unique within each supplied set and declared in the graph | `unique_boundary`, assembly presence check | `DuplicateTensor` or `UnknownTensor` |
| Kernel IDs are unique | `validate` kernel set | `DuplicateKernel` |
| Every kernel input and output names a tensor | `PrimitiveKernel::validate` | `UnknownTensor` |
| Every input/output alias pair appears once and only once | primitive alias-matrix check | `InvalidPrimitive` |
| Every output has one producer | producer map | `DuplicateProducer` |
| A nonexternal tensor has a producer | boundary check | `MissingProducer` |
| An external input has no producer | boundary check | `DuplicateProducer` |
| Tensor layout fits declared storage | `Tensor::validate` | `InvalidLayout` or `ByteSizeOverflow` |
| Each primitive's arity, dtype, shape, axis, and parameters are legal | kind-specific primitive validators | `ArityMismatch`, `DTypeMismatch`, `ShapeMismatch`, `InvalidAxis`, `DuplicateAxis`, or `InvalidPrimitive` |
| Scalar SSA is legal | `ScalarProgram::validate` | `InvalidScalarProgram` |
| Kernel dependency relation is acyclic | `topological_order_from` | `Cycle` |
| Canonical OGDL has exact schema-v1 records | `ogdl.rs` strict codec | `Syntax`, `Document`, `InvalidGraph`, or `Build` |

The graph validator does not perform recovery. It does not drop duplicate
nodes, pick one of two producers, synthesize a missing tensor, break a cycle,
guess an alias permission, or infer a boundary flag. The returned
`LanguageError` contains a `kind`, human-readable `detail`, and optional
`ValueId` and `KernelTemplateId`; its display form appends those contexts as
`[kernel ...]` and `[value ...]`.

## Boundary example

For a graph with external input `v1`, kernels `k10` and `k20`, and external
output `v3`:

```text
v1 (external_input)
  -> k10: v1 -> v2
  -> k20: v2 -> v3 (external_output)
```

`validate()` accepts the graph when `v2` and `v3` have valid tensor contracts,
`k10` and `k20` have unique IDs, each primitive validates, and no other node
produces `v2` or `v3`. `topological_order()` returns `[10, 20]` when those are
the kernel IDs, and `dependencies(k20)` returns `[10]`.

If `k10` also lists `v3` as an output, the producer map returns
`DuplicateProducer`. If `v3` is marked neither external input nor produced,
the boundary pass returns `MissingProducer`. If `k20` consumes `v3` while
producing `v3`, the self-edge check returns `Cycle`; a longer closed chain is
reported after Kahn's algorithm exhausts all acyclic-ready nodes.

## Out of scope

The graph intentionally stops at typed calculations and static value flow.
The following belong to downstream crates:

- hardware discovery and measured capabilities;
- placement, queues, links, and transfer routes;
- scratch, fault, and native-image buffers introduced by primitive lowering;
- iteration domains, metric emissions, and lifecycle phases;
- candidate enumeration, scheduling, realization, and executor state;
- CUDA, HSA, LLVM, PTX, or other backend syntax.

Keeping these concerns out of `CalculationGraph` is what lets the same
validated graph feed backend-neutral lowering, measured planning, OGDL
artifacts, training, inference, and remote preparation without changing its
calculation semantics.
