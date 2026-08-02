# Primitive kernels and owned parallel operations

This document is the contract for [`language/src/primitive.rs`](../../src/primitive.rs).
The module describes one placement-free calculation node and the ten
Recipe-owned primitive families that can be lowered from it. It does not own
device placement, memory transfers, queues, synchronization with other graph
nodes, native handles, or lifecycle state. Those concerns consume a validated
`PrimitiveKernel` through `recipe-primitives`, then through the planner and the
target-specific kernel emitters.

The authority flow is:

```text
Tensor metadata + PrimitiveKernel
        -> PrimitiveKernel::validate
        -> CalculationGraph::validate
        -> recipe_primitives::lower
        -> self-validating LoweredProgram
        -> planner stage tasks and target realization
```

The same `PrimitiveKernel::validate` call is made by `PrimitiveKernel::work`
and by `recipe-primitives::lower`. A caller cannot obtain a work estimate or a
lowered program from an invalid primitive. The graph and OGDL boundaries add
their own checks around that primitive contract; they do not replace it.

## Module boundary and call graph

`language/src/lib.rs` re-exports every public item from this module. The
important paths are:

| Boundary | Input | Primitive responsibility | Result or next owner |
| --- | --- | --- | --- |
| `ScalarProgramBuilder` | Typed scalar expressions | Builds the `ScalarProgram` embedded by `Elementwise`; ownership and opcode signatures are checked before a primitive sees it. | `Elementwise.program` |
| `CalculationGraph` | `Tensor` records and `CalculationNode { kernel }` records | Indexes tensor IDs, calls `kernel.validate`, enforces unique producers and acyclic dependencies, and orders nodes. | Validated graph |
| OGDL codec | A graph or strict `RecipeIR` OGDL text | Encodes every primitive field explicitly and decodes exact enum spellings, numbers, collections, and fields. | Validated graph or a path-aware codec error |
| `ops::PrimitiveRecipe` | Public operation descriptor plus borrowed `PrimitiveKernel` and tensor index | Matches the operation-specific subset of primitive shapes, dtypes, axes, bounds, and aliasing, then delegates lowering. | `recipe_primitives::LoweredProgram` or an operation error |
| `recipe-primitives::lower` | Validated kernel, tensor index, measured common hardware | Converts each `PrimitiveKind` to immutable stages, typed views, faults, trees, atomics, scratch, and resource bounds. | Self-validating `LoweredProgram` |
| Planner | Validated graph and lowered programs | Prices `PrimitiveKernel::work`, gives every lowered stage a scoped identity, and turns stages into calculation tasks. | Draft plan and artifact build contracts |
| Kernel emitter | One lowered `ProgramStage` | Emits the exact owned stage algorithm. It does not infer a different primitive or repair a mismatch. | Target LLVM IR and native artifact |

The direct primitive constructors in the workspace are the operation
materializers and compiler paths in `ops/src/{bayes,binary_metrics,kmeans,knn_outputs,materialize,tree}.rs`,
`ops/src/materialize/*.rs`, and `training/src/{compile,gguf_llama,inference}.rs`.
`language/src/ogdl.rs` is the persistence consumer, and
`primitives/src/lower.rs` is the backend-neutral lowering consumer. The
planner, kernel, executor, and native backends consume the lowered stages, not
the source `PrimitiveKind` directly.

## Shared vocabulary

All enums derive `Clone`, `Copy`, `Debug`, `PartialEq`, and `Eq`. Their exact
variant names are also the OGDL spellings shown later.

| Type | Variants | Use |
| --- | --- | --- |
| `AtomicOrdering` | `Relaxed`, `Acquire`, `Release`, `AcquireRelease`, `SequentiallyConsistent` | Ordering on scatter read-modify-write operations and histogram accumulation. The lowering crate maps it one-for-one to `OwnedAtomicOrdering`. |
| `AtomicOperation` | `Exchange`, `Add`, `Minimum`, `Maximum` | The payload operation of an atomic scatter conflict policy. The lowering crate maps it one-for-one to `OwnedAtomicOperation`. |
| `IndexBounds` | `Reject`, `Clamp`, `Wrap` | Index policy for gather and scatter. `Reject` publishes a device fault through a preallocated flag and never forms an unchecked payload address. `Clamp` and `Wrap` produce an in-range address without the checked fault path. |
| `ReduceOperator` | `Sum`, `Product`, `Minimum`, `Maximum`, `Any`, `All` | Associative operator for reductions and scans. `Any` and `All` use I32 truth payloads only. |
| `ReduceResult` | `Value`, `Index`, `ValueAndIndex` | Reduction output selection. Index output is I32; a value-and-index reduction has the value output first and the index output second. |
| `ScanMode` | `Inclusive`, `Exclusive { identity: ScalarLiteral }` | Whether the current input participates in its prefix result. Exclusive identity must have the input dtype. |
| `ScatterConflict` | `UniqueIndices`, `Atomic { operation, ordering }` | Declares whether updates are known disjoint or must use the exact atomic operation and ordering. |
| `SortDirection` | `Ascending`, `Descending` | Sort comparator direction. `stable` and `emit_indices` are separate `Sort` fields. |

The primitive layer has exactly two payload dtypes from `recipe-core`: `F32`
and `I32`. Both occupy four bytes. There is no implicit promotion or boolean
payload type. Predicate-like values use I32.

## Primitive specification records

The records below are plain, public data. Their fields are deliberately
explicit so an artifact cannot select a backend algorithm or a policy by
omission.

### `Elementwise`

```text
Elementwise { program: ScalarProgram }
```

The scalar program is one typed SSA calculation applied at every output
coordinate. Its input and output order is the same order as the primitive
kernel's tensor input and output vectors. Broadcasting is a tensor-shape
operation; it does not add scalar inputs or instructions.

### `Reduce`

| Field | Contract |
| --- | --- |
| `operator` | One `ReduceOperator`. `Any` and `All` are I32-only. |
| `axes` | A nonempty, sorted, duplicate-free `AxisSet`; rank membership is checked against the input tensor. |
| `keep_dimensions` | If true, reduced axes remain with extent `1`; if false, they are removed. A reduction over every axis still uses shape `[1]`, because rank-zero payload shapes are not implicit. |
| `result` | `Value`, `Index`, or `ValueAndIndex`. Index-bearing results are legal only for `Minimum` and `Maximum`. |
| `tree_lanes` | A power of two in `1..=1024`. The value fixes the reduction tree and therefore the floating-point operation order. |

### `Scan`

| Field | Contract |
| --- | --- |
| `operator` | One `ReduceOperator`. `Any` and `All` require I32 input and output. |
| `axis` | One input axis, checked against the input rank. |
| `mode` | Inclusive or exclusive with a literal identity whose dtype equals the input dtype. The language layer does not require a particular identity value for the operator; the operation registry adds canonical identities for its direct recipes. |
| `reverse` | Selects forward or reverse traversal. It is retained in the immutable stage contract. |
| `tree_lanes` | A power of two in `1..=1024`; it fixes the local scan tree. |

Input and output tensors have identical shape and dtype. A scan has one input
and one output.

### `Contraction`

```text
Contraction {
    batch_axes: Vec<(left_axis, right_axis)>,
    contract_axes: Vec<(left_axis, right_axis)>,
}
```

Axis pairs are ordered. Batch pairs become the leading output dimensions,
followed by every unpaired left axis in left tensor order, then every unpaired
right axis in right tensor order. Contracted axes are summed and do not appear
in the output. A pair must have equal extents on both operands. No left or
right axis may occur in more than one batch or contract pair. At least one
contract pair is required. If every axis is paired, the output shape is
`[1]`.

### `Gather`

```text
Gather { axis: usize, bounds: IndexBounds }
```

Input 0 is the source tensor, input 1 is an I32 index tensor, and output 0 has
the source dtype. The output shape replaces source axis `axis` with all index
tensor extents. `bounds` is carried unchanged into lowering.

### `Scatter`

```text
Scatter { axis: usize, bounds: IndexBounds, conflict: ScatterConflict }
```

Input 0 is the base tensor, input 1 is an I32 index tensor, and input 2 is the
update tensor. Output 0 has the base dtype and exact base shape. The update
shape is the gather shape obtained by replacing base axis `axis` with the
index-tensor shape. `UniqueIndices` selects a non-atomic write. `Atomic`
selects the exact operation and ordering; validation does not narrow the
operation or ordering by dtype.

### `Histogram`

| Field | Contract |
| --- | --- |
| `bins` | `1..=i32::MAX`. The output shape is exactly `[bins]`. |
| `weighted` | False means one input and an I32 output. True means two inputs, with input 1 F32 weights of the same shape as input 0, and an F32 output. The value input may be F32 or I32. |
| `ordering` | The exact atomic ordering used for accumulation. |

The lowering stage clears the output first, then accumulates. I32 values map
directly to bins; F32 values map by truncation toward zero. An out-of-range bin
publishes the histogram fault code through the checked fault channel.

### `Sort`

| Field | Contract |
| --- | --- |
| `axis` | A valid input axis. Its extent must be representable by I32 because emitted indices use I32. |
| `direction` | Ascending or descending. |
| `stable` | Requests stable ordering. Lowering retains the original axis index as a deterministic tie-break even for an unstable public request. |
| `emit_indices` | False produces one value output. True produces a same-shaped I32 index output in slot 1. |

The values output always has the input dtype and exact input shape. The
lowering implementation uses a least-power-of-two padded bitonic network,
IEEE-754 total-order keys, and original-index ascending tie-breaking.

### `IndexMap`

```text
IndexMap {
    start: i32,
    element_step: i32,
    iteration_step: i32,
    modulus: Option<i32>,
}
```

There are no inputs and exactly one I32 output. For element index `e` and loop
iteration `t`, the intended value is:

```text
affine = start + e * element_step + t * iteration_step
```

The implementation evaluates the products and additions through checked I64
intermediates. Without `modulus`, `affine` must fit I32. With a strictly
positive modulus `m`, Recipe applies Euclidean remainder, yielding a value in
`0..m`, with the upper endpoint excluded, before narrowing to I32. Arithmetic
rejection publishes fault code 4. The source validator checks only arity,
output dtype, and positivity of an optional modulus; the dynamic affine and
I32-range checks are part of the emitted stage.

### `RandomKey`, `RandomDistribution`, and `RandomMap`

`RandomKey` contains `seed_low`, `seed_high`, and `stream`, all `u64`. The
runtime additionally folds the immutable `RunId`, loop iteration, source
kernel ID, and linear element index into the Recipe-owned counter/key
contract.

| Distribution | Output dtype and parameters |
| --- | --- |
| `UniformF32` | F32 in the Recipe Philox mapping. |
| `NormalF32` | F32 in the Recipe Box-Muller V1 mapping. |
| `BernoulliI32 { probability_bits }` | I32; bits decode to a finite F32 probability in `[0, 1]`. |
| `UniformI32 { low, high_exclusive }` | I32; requires `low < high_exclusive`. |

`RandomMap` stores the distribution, key, and `philox_rounds`. Validation
requires exactly `10`, so the artifact cannot silently select another RNG
implementation. `RandomMap` has no inputs and one output whose dtype follows
the distribution.

## `PrimitiveKind`, alias rules, and kernel identity

`PrimitiveKind` is the tagged union of `Elementwise`, `Reduce`, `Scan`,
`Contraction`, `Gather`, `Scatter`, `Histogram`, `Sort`, `IndexMap`, and
`Random`. It is the complete language primitive vocabulary. There is no
separate transfer or placement variant. A calculation graph represents
transfers and placement later in planning.

`PrimitiveAliasRule` names one zero-based input/output pair and carries the
core `AliasPermission` (`Forbidden`, `MayAliasExact`, or `MustAliasExact`).
The pair is about the tensor storage contract, not scalar values. Every pair
must be present exactly once, including pairs that cannot alias in the current
materializer.

`PrimitiveKernel` is:

| Field | Meaning |
| --- | --- |
| `id: KernelTemplateId` | Stable source-kernel identity used in graph ordering, errors, and later stage identities. |
| `inputs: Vec<ValueId>` | Ordered tensor values consumed by the primitive. |
| `outputs: Vec<ValueId>` | Ordered tensor values written by the primitive. |
| `alias_rules: Vec<PrimitiveAliasRule>` | Complete input/output alias matrix. |
| `kind: PrimitiveKind` | Typed operation and all shape, policy, ordering, and algorithm parameters. |

The kernel is placement-free. `recipe-primitives` copies its alias matrix into
`SourceAliasContract` records and preserves the source kernel ID in the
`LoweredProgram`; the planner later resolves those contracts against resident
values and arena bindings.

## Validation pipeline

`PrimitiveKernel::validate` is a fail-fast `LanguageResult<()>` path. It does
not accumulate independent failures. The ordered steps are:

1. Look up every input ID in the supplied `BTreeMap<ValueId, &Tensor>`. A
   missing ID returns `UnknownTensor`, tagged with the kernel and value.
2. Look up every output ID the same way. The first missing output also returns
   `UnknownTensor` with both context fields.
3. Validate the alias matrix before inspecting `kind`.
4. Dispatch to the validator for the one `PrimitiveKind` variant.

`LanguageError` stores `kind`, `detail`, and optional `kernel` and `value`
identifiers. `Display` prints the kind and detail, followed by `[kernel N]`
and `[value N]` when present. Shape and axis helpers return their own
`LanguageErrorKind` values, then the primitive validator attaches the kernel
context with `for_kernel`.

### Alias matrix invariant

For `I` inputs and `O` outputs, the accepted alias-rule set has exactly
`I * O` entries. Each rule must satisfy:

```text
0 <= rule.input  < I
0 <= rule.output < O
```

The pair `(rule.input, rule.output)` must be unique. A bounded, duplicate-free
set with the required cardinality is therefore a complete matrix. An out of
range pair, duplicate pair, or incomplete matrix returns
`InvalidPrimitive`, tagged with the kernel. If either arity is zero, the
required matrix size is zero, so a kernel with no inputs or no outputs must not
carry alias rules for nonexistent pairs.

### Per-kind validation

The following table is a compact parseable summary. “Exact” means that the
tensor metadata must equal the computed value, not merely be broadcastable or
compatible.

| Kind | Input arity | Output arity | Dtype rules | Shape and parameter rules |
| --- | ---: | ---: | --- | --- |
| `Elementwise` | `program.inputs.len()` | `program.outputs.len()` | Each tensor input equals the corresponding scalar input dtype. Each output equals `program.dtype_of(program.outputs[i])`. | Scalar program validates. Inputs must be nonempty and broadcast-compatible. Every output has the broadcast result shape. |
| `Reduce` | `1` | `1` for `Value` or `Index`, `2` for `ValueAndIndex` | `Value` preserves input dtype. `Index` is I32. `ValueAndIndex` is input dtype then I32. `Any` and `All` require I32 input. | Axes validate against rank. `tree_lanes` is power of two in `1..=1024`. Output shape is `Shape::reduced(input, axes, keep_dimensions)`. Index-bearing results require `Minimum` or `Maximum`. Empty Min/Max domains are rejected. |
| `Scan` | `1` | `1` | Output equals input dtype. `Any` and `All` require I32. Exclusive identity dtype equals input. | Axis is in rank. `tree_lanes` is power of two in `1..=1024`. Output shape equals input shape. |
| `Contraction` | `2` | `1` | Both operands have equal dtype; output equals the left dtype. | At least one contract pair. Every pair is in range, uses each side's axis once, and has equal extents. Output order is batch extents, free left extents, free right extents, or `[1]` if none. |
| `Gather` | `2` | `1` | Input 1 is I32. Output equals input 0 dtype. | `Shape::gather_result(input0, axis, input1.shape)` must equal output shape. |
| `Scatter` | `3` | `1` | Input 1 is I32. Updates and output equal base input dtype. | Output shape equals base shape. Update shape equals `Shape::gather_result(base, axis, indices.shape)`. Conflict and bounds are retained exactly. |
| `Histogram` | `1` or `2` | `1` | Weighted input 1 is F32 and output is F32. Unweighted output is I32. | `bins` is `1..=i32::MAX`; weighted values and weights have equal shape; output shape is `[bins]`. |
| `Sort` | `1` | `1` or `2` | Values output equals input dtype. Emitted indices output is I32. | Axis is in rank and its extent is at most `i32::MAX`. Both outputs, when present, equal input shape. |
| `IndexMap` | `0` | `1` | Output is I32. | Optional modulus is strictly positive. No tensor shape restriction beyond the output tensor metadata. |
| `Random` | `0` | `1` | Distribution selects F32 or I32 as described above. | `philox_rounds == 10`; Bernoulli probability is finite and in `[0, 1]`; uniform I32 has `low < high_exclusive`. |

#### Elementwise validation details

`ScalarProgram::validate` runs before tensor arity and shape checks. Its
failures are wrapped as `InvalidScalarProgram` with the validator's complete
text. Arity mismatches use `ArityMismatch`. A zero-input elementwise map is
rejected even if its scalar program has only constants: the detail directs
callers to `Random` or an explicit filled input. `Shape::broadcast_result`
uses standard leading-axis broadcasting, where an extent of `1` may expand and
all other conflicting extents fail with `ShapeMismatch`.

Output scalar IDs are looked up with `program.dtype_of`. An unknown scalar
output therefore produces `DTypeMismatch` with an expected dtype of `None`,
after the scalar-program validator would normally have rejected that same
program.

#### Reduction validation details

`AxisSet::new` rejects an empty axis list with `InvalidAxis` and duplicate axes
with `DuplicateAxis`; it sorts accepted axes. `validate_rank` rejects an axis
outside the input rank. `Shape::reduced` replaces each selected axis with
extent `1` when `keep_dimensions` is true, removes it otherwise, and inserts
`[1]` if all axes were removed.

An empty selected axis is legal for `Sum`, `Product`, `Any`, and `All`; the
lowering crate fills value outputs with the operator identity and fills an
index output with I32 zero. It is not legal for `Minimum` or `Maximum`, because
this language layer defines no implicit empty-domain identity. This is an
`InvalidPrimitive` failure rather than a backend-dependent result.

#### Scan validation details

The language layer checks the exclusive identity's dtype but not whether its
numeric value is the algebraic identity. `ops::PrimitiveRecipe::matches` adds
that narrower check for its direct F32 recipes: exclusive sum must use F32
zero, exclusive product must use F32 one, and exclusive scans for minimum,
maximum, any, or all are not accepted by the direct recipe matcher. Inclusive
scans do not carry an explicit identity.

#### Contraction validation details

The validator walks `batch_axes` first and `contract_axes` second while filling
one shared `used_left` and `used_right` set. Thus an axis reused across the two
classes is rejected as well as an axis repeated within one class. Batch pair
order is observable in the leading output dimensions. Free-axis order is
always source tensor order, independent of pair order. Extent mismatch is
`ShapeMismatch`; rank or axis overflow is `InvalidAxis`; axis reuse is
`DuplicateAxis`.

#### Index, histogram, sort, and random validation details

The bounds enum itself is always accepted. Its safety contract is enforced in
lowering: `Reject` must have a checked fault stage, and `Clamp` or `Wrap` must
not fabricate one. Histogram bin bounds are checked before `Shape::new`, so
the `expect("nonzero bin count")` in the source is reached only after the
explicit `bins` range check. Sort's I32 extent check protects both emitted
indices and the deterministic sort network. Random Bernoulli probability is
decoded from bits, so NaN and infinity are rejected without a float parsing or
round-trip ambiguity.

## Work accounting

`PrimitiveKernel::work` first calls `validate`. It then computes an exact
`u64` arithmetic-work bound and wraps it in `FlopCount`. It counts only the
operation family described below. Addressing, loads, stores, validation
predicates, representation changes, atomics that are not arithmetic, and
fault publication are not included unless the table explicitly prices them.

Let `N_in` and `N_out` be the input and output logical element counts, `A` be
the selected sort-axis extent, `S = N_in / A` when `A != 0`, and `C` be the
product of the left contraction extents named by `contract_axes`.

| Kind | Work returned before `FlopCount` wrapping | Checked operations |
| --- | --- | --- |
| `Elementwise` | `N_out * sum(instruction.opcode.flops())` | Instruction FLOP sum and final multiplication; overflow is `WorkOverflow`. |
| `Reduce` | `(N_in.saturating_sub(N_out)) * (2 if ValueAndIndex else 1)` | Final multiplication; subtraction intentionally saturates. |
| `Scan` | `2 * N_in` | Final multiplication. |
| `Contraction` | `2 * C * N_out` | Contracted-extent product, then output product, then factor two. |
| `Gather` | `N_out` | No additional multiplication. |
| `Scatter` | `N_out` | No additional multiplication. |
| `Histogram` | `N_in` | No additional multiplication. |
| `Sort` | `0` when `A <= 1`; otherwise `S * A * (u64::BITS - leading_zeros(A - 1))` | Comparison factor, then final multiplication. Division by zero yields `S = 0`. |
| `IndexMap` | `0` | None. Integer map work is priced separately by lowered-stage resource bounds. |
| `Random` | `N_out * philox_rounds * 4` | Two checked multiplications; validation fixes rounds at ten. |

`scalar_work` delegates to `ScalarOpcode::flops`: FMA costs two; arithmetic,
comparisons, and F32 elementary rounding operations cost one; bit operations,
casts, selection, checks, conversions, and `Require` cost zero. The planner
includes `kernel.work(tensors)` in its graph digest and uses the resulting
maximum to derive training tuning. `src/training.rs` also computes the maximum
kernel work for measured native tuning, then divides it by the slowest local
calculation rate. Work is therefore part of graph identity and scheduling
evidence, not an informational log.

## Scalar builder and elementwise construction

`language/src/scalar_builder.rs` supplies the normal construction path for an
elementwise scalar program. `ScalarProgramBuilder::new` allocates a process
local owner token from a checked atomic counter. Every `ScalarExpression`
contains that owner, a `ScalarValueId`, and its dtype. IDs start at one and
advance through checked arithmetic across inputs, constants, and instruction
results.

| Builder operation | Effect and failure |
| --- | --- |
| `input(dtype)` | Adds one typed scalar input. |
| `constant(literal)`, `f32(bits)`, `i32(value)` | Adds one scalar constant; F32 values are stored as exact bits. |
| `apply(opcode, operands)` | Rejects a foreign expression, resolves `ScalarOpcode::result_dtype`, allocates a result ID, and appends one instruction in call order. Foreign ownership or an invalid signature is `InvalidScalarProgram`. |
| `unary`, `binary`, `ternary` | Arity-specific wrappers over `apply`. |
| `finish(outputs)` | Rejects foreign outputs, assembles `ScalarProgram`, and calls core scalar validation before returning. |

`PrimitiveKernel::validate` repeats scalar validation even when the builder was
used. This protects decoded OGDL and direct struct literals. The lowering
crate converts an elementwise primitive into a core `KernelTemplate`: scalar
inputs and outputs are paired with tensor dtypes, broadcast input strides are
materialized, output access remains the tensor's exact layout, and the source
alias matrix is translated to core input/output IDs. A scalar program that
requires checked I32 arithmetic or `Require` gets one preallocated I32 fault
flag in the lowered stage.

## Graph integration

`CalculationNode` is a one-field wrapper around `PrimitiveKernel`.
`CalculationGraph` stores ordered `Tensor` and `CalculationNode` vectors.

### `CalculationGraph::validate`

The graph validator in [`language/src/graph.rs`](../../src/graph.rs) executes
these checks in order:

1. Build a unique `BTreeMap<ValueId, &Tensor>`. Duplicate tensor IDs return
   `DuplicateTensor`.
2. Validate every tensor's layout, storage span, and byte bound.
3. Reject duplicate kernel IDs with `DuplicateKernel`.
4. Call `node.kernel.validate(&tensor_index)` for every node. Any primitive
   error retains its kernel and, when relevant, tensor value context.
5. Build a producer map and reject multiple producers for one output with
   `DuplicateProducer`.
6. Require every non-external tensor to have a producer (`MissingProducer`) and
   reject an external input that is also produced (`DuplicateProducer`).
7. Derive the producer-to-consumer edges from kernel inputs and reject a
   self-edge or any cycle with `Cycle`.

`topological_order` validates again, then returns the deterministic order from
a sorted ready set. `dependencies(kernel)` validates again, resolves input
producers, sorts IDs, and deduplicates them. A requested but absent kernel is
reported as `InvalidPrimitive` for that kernel ID.

### `CalculationGraph::assemble`

Assembly combines independently materialized fragments without trusting their
temporary boundary flags. It requires unique external input and output sets,
requires each boundary value to be declared by some fragment, and accepts
repeated tensor declarations only when ID, dtype, shape, layout, and
`storage_bytes` are identical. It then applies the caller's exact external
flags, sorts tensors and nodes by ID, and calls `validate`. Primitive aliases
are therefore checked only after all fragments share one storage contract.

The graph layer intentionally keeps `PrimitiveKernel` placement-free. External
flags, producer edges, and topological dependencies describe graph ownership;
they do not become fields on `PrimitiveKind`.

## Canonical OGDL representation

`language/src/ogdl.rs` uses root `RecipeIR`, schema `CalculationGraph`, version
`1`. Encoding calls `CalculationGraph::validate` before constructing output.
Each node has a `kernel` record with these required fields, in canonical
encoder order:

```text
kernel
  id <unsigned integer>
  inputs
    value <unsigned integer> ...
  outputs
    value <unsigned integer> ...
  alias_rules
    alias_rule
      input <unsigned integer>
      output <unsigned integer>
      permission <Forbidden | MayAliasExact | MustAliasExact>
  kind
    <exactly one PrimitiveKind variant>
```

The `kind` field has one exact child and the following required fields:

| Variant child | Required fields and child records |
| --- | --- |
| `Elementwise` | `program` with explicit `inputs`, `constants`, `instructions`, and `outputs` scalar records. |
| `Reduce` | `operator`, `axes.axis` list, `keep_dimensions`, `result`, `tree_lanes`. |
| `Scan` | `operator`, `axis`, `mode` (`Inclusive` leaf or `Exclusive.identity` literal), `reverse`, `tree_lanes`. |
| `Contraction` | `batch_axes.pair` and `contract_axes.pair`, each pair carrying `left` and `right`. |
| `Gather` | `axis`, `bounds`. |
| `Scatter` | `axis`, `bounds`, `conflict` (`UniqueIndices` leaf or `Atomic.operation` and `Atomic.ordering`). |
| `Histogram` | `bins`, `weighted`, `ordering`. |
| `Sort` | `axis`, `direction`, `stable`, `emit_indices`. |
| `IndexMap` | `start`, `element_step`, `iteration_step`, and `modulus` (`None` leaf or `Some` value). |
| `Random` | `distribution`, `key.seed_low`, `key.seed_high`, `key.stream`, `philox_rounds`. Distribution is `UniformF32`, `NormalF32`, `BernoulliI32.probability_bits`, or `UniformI32.low/high_exclusive`. |

All fields are required. The decoder rejects unknown or duplicate fields,
missing fields, collection items with the wrong name, enum variants with the
wrong spelling, multiple variant children, unexpected leaf children, invalid
booleans, and out-of-range integer text with path-aware
`OgdlCodecError::Document` values. `F32Bits` is a decimal `u32`, not a textual
float. After decoding, `AxisSet::new` runs while constructing reductions, then
`CalculationGraph::from_ogdl_graph` calls graph validation, including every
primitive rule above. A syntactically valid OGDL document is not accepted
until this semantic pass succeeds.

## Operation registry and `ops::PrimitiveRecipe`

The operation crate has a second, narrower abstraction that must not be
confused with `language::PrimitiveKind`:

| Language layer | `ops` layer |
| --- | --- |
| `PrimitiveKind` carries all concrete parameters and tensor arities. | `PrimitiveRecipe` identifies one public operation-specific subset and leaves axes, extents, bounds, random keys, and tree widths in the caller's `PrimitiveKernel`. |
| `PrimitiveKernel::validate` accepts the full typed vocabulary. | `PrimitiveRecipe::matches` narrows to the exact operation contract used by a registry descriptor. |
| `recipe_primitives::lower` lowers any validated `PrimitiveKind`. | `ops::lower_primitive` first requires the descriptor's `LoweringAvailability::Primitive`, then matches and delegates. |

`PrimitiveFamily` contains `Elementwise`, `Reduce`, `Scan`, `Contraction`,
`Gather`, `Scatter`, `Histogram`, `Sort`, `IndexMap`, and `Random`. The direct
`PrimitiveRecipe` enum has recipes for reductions, scans, contractions,
gather, scatter-add, sort, index maps, and F32 random maps. Elementwise and
histogram remain valid language kinds and are used by operation compositions and
materializers, but they are not direct legacy-symbol recipes in this enum.

### Recipe matching rules

`PrimitiveRequest` borrows the kernel and tensor index, so matching allocates
no graph or tensor data. The matcher checks:

* Reduction operator and result exactly, plus `AxisRequirement`:
  `Any` means a nonempty axis list, `First` is `[0]`, `Last` is the final input
  axis, and `All` is every axis in `0..rank`.
* Scan operator, inclusive/exclusive mode, and axis requirement. Reverse scans
  are rejected by direct recipes. Inclusive mode is always accepted; exclusive
  F32 sum requires bitwise zero and exclusive F32 product requires bitwise one.
  Exclusive minimum, maximum, any, and all, or an I32 exclusive identity for
  sum/product, do not match.
* Contraction class and ranks: vector is `(1,1)` with `(0,0)` contraction;
  matrix is `(2,2)` with `(1,0)`; left-transposed matrix is `(2,2)` with
  `(0,0)`; right-transposed matrix is `(2,2)` with `(1,1)`; batched requires
  both ranks at least three, at least one batch pair, and exactly one contract
  pair.
* Gather axis requirement and `IndexBounds::Reject`.
* Scatter-add axis requirement, `IndexBounds::Reject`, an atomic `Add` with
  `SequentiallyConsistent` ordering, and a `MustAliasExact` rule from input 0
  to output 0.
* Sort direction and stability, the requested axis requirement, and
  `emit_indices == false`.
* Random distribution family (`UniformF32` or `NormalF32`).
* Index maps unconditionally, after the language validator has checked the
  concrete index-map kernel.

Direct registry dtype contracts are intentionally narrower than the language
vocabulary:

| Recipe family | Inputs | Outputs |
| --- | --- | --- |
| F32 value reduction or scan | `[F32]` | `[F32]` |
| F32 index reduction | `[F32]` | `[I32]` |
| F32 value-and-index reduction | `[F32]` | `[F32, I32]` |
| F32 contraction | `[F32, F32]` | `[F32]` |
| F32 gather | `[F32, I32]` | `[F32]` |
| F32 scatter-add | `[F32, I32, F32]` | `[F32]` |
| F32 sort | `[F32]` | `[F32]` |
| Index map | `[]` | `[I32]` |
| F32 random map | `[]` | `[F32]` |

`lower_primitive` returns `WrongLoweringKind` when a descriptor is scalar,
composition, workspace, or non-calculation instead of direct primitive;
`UnsupportedLowering` for an explicitly unsupported descriptor;
`PrimitiveRecipeMismatch` when the kernel does not satisfy the selected
recipe; and `PrimitiveLoweringFailed` when `recipe_primitives::lower` rejects
hardware, language, arithmetic, or lowered-program invariants. All operation
errors retain the descriptor's operation ID.

### Registry symbols

The current direct symbol table in [`ops/src/primitive.rs`](../../../ops/src/primitive.rs)
is exact and source-qualified by the operation registry:

| Symbol | Primitive recipe |
| --- | --- |
| `gpu_argmax_f32` | Maximum, `Index`, all axes |
| `gpu_argmax_rows` | Maximum, `Index`, last axis |
| `gpu_argmin_rows` | Minimum, `Index`, last axis |
| `gpu_bmm_into` | Batched contraction |
| `gpu_cummax` | Inclusive maximum scan, any axis |
| `gpu_cumprod` | Inclusive product scan, any axis |
| `gpu_cumsum_cols` | Inclusive sum scan, first axis |
| `gpu_cumsum_rows` | Inclusive sum scan, last axis |
| `gpu_dot` | Vector contraction |
| `gpu_gather_rows_into` | Gather, first axis, reject bounds |
| `gpu_gemm` | Matrix contraction |
| `gpu_gemm_at` | Left-transposed matrix contraction |
| `gpu_gemm_bt` | Right-transposed matrix contraction |
| `gpu_gemm_bt_into` | Right-transposed matrix contraction |
| `gpu_max_all` | Maximum value reduction, all axes |
| `gpu_min_all` | Minimum value reduction, all axes |
| `gpu_prefix_sum_exclusive` | Exclusive sum scan, any axis |
| `gpu_prefix_sum_inclusive` | Inclusive sum scan, any axis |
| `gpu_rand_uniform_into` | Uniform F32 random map |
| `gpu_randn` | Normal F32 random map |
| `gpu_reduce_max_cols` | Maximum value reduction, first axis |
| `gpu_reduce_max_rows` | Maximum value reduction, last axis |
| `gpu_reduce_min_cols` | Minimum value reduction, first axis |
| `gpu_reduce_min_rows` | Minimum value reduction, last axis |
| `gpu_reduce_sum_cols_into` | Sum value reduction, first axis |
| `gpu_reduce_sum_rows` | Sum value reduction, last axis |
| `gpu_scatter_add` | Atomic-add scatter, first axis, reject bounds |
| `gpu_sort` | Ascending unstable sort over the only axis |
| `gpu_sum_all` | Sum value reduction, all axes |

The table has 29 symbols. A symbol not in this table can still be materialized
as a concrete `PrimitiveKind` by an operation composition or compiler path; it
simply does not receive a direct `PrimitiveRecipe` descriptor.

## Backend-neutral primitive lowering

[`primitives/src/lower.rs`](../../../primitives/src/lower.rs) is the complete
consumer of the language primitive union. Its public `lower` function performs
the following linear sequence:

1. Validate measured common hardware: subgroup lanes are nonzero and a power
   of two, maximum workgroup lanes are at least subgroup lanes, and shared
   memory per workgroup is nonzero. Failure is `LoweringErrorKind::InvalidLoweredProgram`.
2. Call `kernel.validate`. A language error becomes
   `LoweringErrorKind::InvalidLanguage`, preserving kernel and value context.
3. Add every distinct source input/output tensor as an external-value buffer
   with complete extents, offset, strides, dtype, and storage bytes.
4. Dispatch on `PrimitiveKind` and append zero or more immutable
   `ProgramStage` values.
5. Copy the complete alias matrix to `source_aliases`, record source input and
   output counts, aggregate exact resource bounds, compute the canonical
   digest, and run `LoweredProgram::validate`.

The builder assigns dense buffer and stage IDs. Stages depend on the
immediately preceding stage, so a multi-stage primitive is an ordered chain.
Scratch buffers have explicit purposes and program lifetime. Checked paths
share one I32 `FaultFlag` buffer per lowered program; every fault contract must
publish with an int32 release atomic exchange before forming a rejected payload
address.

### Source kind to lowered stage mapping

| Source kind | Stages and zero-element behavior | Immutable algorithm contract |
| --- | --- | --- |
| `Elementwise` | One `ScalarMap` stage unless output elements are zero, in which case there is no stage. | Core `KernelTemplate` with broadcast input views, exact output views, scalar SSA, aliases, and an optional arithmetic fault flag. |
| `Reduce` | One or more `FixedTreeReduce` passes for nonempty work. A zero reduced width emits `Fill` stages for nonempty value outputs (operator identity) and index outputs (I32 zero); an empty output emits no stage. | Power-of-two fixed tree, operator-identity padding, lowest logical index tie-break, and value/index scratch buffers as required. Empty Min/Max was already rejected by the language validator. |
| `Scan` | Hierarchical `FixedTreeScanLocal` stages plus reverse `ScanUniformCombine` stages. Empty input or zero-width scan axis emits no stage. | Canonical Blelloch upsweep/downsweep tree, user inclusive or exclusive mode at level zero, and the declared reverse direction. |
| `Contraction` | One `TiledContraction` stage unless output elements are zero. | Ordered contracted coordinates, measured-capacity output tile, and direct private accumulation in the current implementation. The lowered contract still records strategy and tile explicitly. |
| `Gather` | One `Gather` stage unless output is empty. `Reject` adds a checked fault flag with code 1. | Exact axis and bounds policy. |
| `Scatter` | A `Copy { ScatterBase }` stage for a nonempty base, then a `Scatter` stage when updates are nonempty. `Reject` adds code 1. | Unique updates use `Write`; atomic updates use `ReadWriteAtomic` plus the exact payload atomic contract. |
| `Histogram` | Always one `HistogramClear` stage for the output, then an `HistogramAccumulate` stage when input is nonempty. Accumulation always has fault code 3. | I32 direct or F32 truncate-toward-zero bin mapping, F32 weight binding when weighted, and declared atomic ordering. |
| `Sort` | For a nonempty axis, `StableSortInitialize`, every fixed bitonic compare-exchange phase, and `StableSortFinalize`; empty input or zero axis emits no stages. | Least power-of-two padding, IEEE total-order keys, valid elements before padding, original index ascending tie-break, and optional emitted indices. |
| `IndexMap` | One `IndexMap` stage unless output is empty. It always has an arithmetic fault flag with code 4. | Checked I64 affine calculation, positive Euclidean modulus, and a loop-iteration ABI argument only when `iteration_step != 0`. |
| `Random` | One `Philox4x32_10` stage unless output is empty. | Ten rounds, Recipe multipliers and Weyl constants, key folding, dynamic `RunId` ABI, and the exact distribution mapping. |

The lowered validation layer in [`primitives/src/validate.rs`](../../../primitives/src/validate.rs)
checks more than source language validation: schema version, dense IDs,
storage spans, writable injectivity, stage dependencies, launch geometry,
binding modes, atomic contracts, fault publication, canonical reduction and
scan trees, sort network phases, Philox constants and counter layout, exact
stage resource formulas, aggregate resource totals, and the canonical digest.
It reports `ProgramValidationError { path, detail }` values and never repairs a
stale or near-compatible stage.

### Lowering failure boundaries

The source language can fail before any stage exists:

| Failure | Owner and condition |
| --- | --- |
| `UnknownTensor`, arity, dtype, shape, axis, alias, or semantic primitive errors | `PrimitiveKernel::validate`. |
| `InvalidLanguage` | `recipe-primitives::lower` received one of the above errors. |
| `MissingTensor` | A tensor ID was absent while building source or scratch bindings. |
| `ArithmeticOverflow` | Checked index-space, scratch, stage, resource, or identifier arithmetic overflowed. |
| `InvalidStaticAccess` | A broadcast or binding view could not be represented safely. |
| `InvalidLoweredProgram` | Hardware limits, stage structure, exact resources, or final digest validation disagree. |
| `PrimitiveRecipeMismatch` and operation-kind errors | The `ops` registry selected a narrower direct recipe that the concrete kernel does not satisfy. |

Empty payloads are not failures. They produce an honest zero-stage lowered
program where the primitive's lowering can prove no dispatch is required. The
planner consumes that program without creating a fabricated one-lane kernel,
artifact, or calculation task.

## Planner, kernel, and artifact consumption

The planner calls `recipe_primitives::lower` for every graph node after graph,
topology, discovery, reservation, and capacity validation. It revalidates each
returned program, derives a stage identity from `(source_kernel, StageId)` and
the lowered digest, and retains a complete `KernelTemplate` only for
`StageKind::ScalarMap`. Every other stage receives the corresponding owned
Recipe emitter in `kernel/src/stage.rs`.

The kernel boundary verifies the stage-scoped contract before LLVM generation:

* `Reject` gather and scatter paths guard before payload address formation and
  publish their exact fault code.
* Scatter and histogram atomics preserve operation and ordering. Floating
  atomic add/min/max use Recipe-owned compare/exchange loops rather than a
  vendor library.
* Reductions and scans use the encoded fixed trees and identities.
* Sort uses the encoded total-order comparator and tie-break.
* Index maps expose the loop-iteration argument only when requested by the
  nonzero iteration step.
* Philox uses the dynamic run ID and the exact ten-round constants; its
  artifact never embeds a run-specific value.

The planner translates stage dependencies and buffer lifetimes into calculation
tasks, resident tensor values, scratch values, and preallocated fault readback
paths. A finalized artifact must retain the same source kernel, stage ordinal,
typed views, access modes, dispatch geometry, fault binding, operation bounds,
and resource envelope. A target backend cannot silently choose a different
primitive algorithm or a vendor implementation.

## Direct construction inventory and non-goals

The source union is constructed directly in these current modules:

| Producer group | Current files and role |
| --- | --- |
| Language persistence | `language/src/ogdl.rs` encodes and decodes all ten variants. |
| Operation materializers | `ops/src/bayes.rs`, `binary_metrics.rs`, `kmeans.rs`, `knn_outputs.rs`, `tree.rs`, `materialize.rs`, and the materializer submodules build concrete graph fragments. |
| Training compiler and inference | `training/src/compile.rs`, `training/src/gguf_llama.rs`, and `training/src/inference.rs` emit graph kernels for model and data paths. |
| Primitive registry | `ops/src/primitive.rs` matches the subset tied to public operation symbols and delegates to `recipe-primitives`. |
| Backend-neutral lowerer | `primitives/src/lower.rs` is the only complete `PrimitiveKind` dispatch into immutable stages. |

The primitive layer intentionally does not add:

* transfer nodes, placement or device IDs, queue or synchronization edges
  between graph kernels, or lifecycle phases;
* host-side fallback calculations, vendor-library calls, runtime-selected
  algorithms, or a second alias representation;
* implicit defaults for omitted OGDL fields, enum variants, tree widths,
  bounds, atomic ordering, random keys, or Philox rounds;
* a separate boolean, index, or tensor type outside the explicit F32 and I32
  payload domain.

Those exclusions preserve one direct contract: a `PrimitiveKernel` is a typed,
fully parameterized, placement-free calculation. Its validation establishes
the only language-level preconditions, `work` prices the same semantic object,
and every downstream stage and artifact must preserve the declared operation,
ordering, safety policy, and data shape.
