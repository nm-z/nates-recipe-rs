# `recipe_language::shape`

This document is the contract for [`language/src/shape.rs`](../../src/shape.rs)
and for the tensor and graph boundaries that consume its values. The module is
the backend-neutral shape vocabulary of `recipe-language`. It describes fixed
rank, fixed extent payload metadata. It does not allocate storage, calculate a
stride view, select a device, lower a primitive, or infer a missing dimension.

The public surface is re-exported by [`language/src/lib.rs`](../../src/lib.rs):
`Shape` and `AxisSet` are crate-root types, alongside `Tensor`,
`TensorLayout`, and the primitive specifications. There is no separate public
`Dimension` or `Dim` type in `recipe-language`. A dimension is one `u64`
element of a `Shape` extent vector. An axis is one `usize` index, represented in
an `AxisSet` when a set of axes is required.

## Parseable intent

The following block states the invariants without relying on the surrounding
prose:

```text
shape_contract:
  shape:
    extent_type: u64
    extent_order: axis_zero_to_axis_rank_minus_one
    rank: number_of_extents
    minimum_rank: 1
    zero_extent: allowed
    empty_payload: elements == 0
    scalar_payload: explicit_shape == [1]
    elements: checked_product_of_extents_in_input_order
    storage: private_extents_and_cached_elements
  axis_set:
    axis_type: usize
    constructor_input: nonempty_vector
    stored_order: ascending
    duplicates: rejected
    rank_bounds: checked_only_when_validate_rank_or_a_consumer_checks
  transformations:
    broadcast: trailing_axis_alignment_with_extent_one_expansion
    reduction: remove_or_replace_selected_axes_with_one
    gather: replace_one_source_axis_with_the_complete_index_shape
  bytes:
    typed_size: checked(elements * dtype.byte_width())
  rank_zero:
    representation: forbidden
    error: EmptyShape
  zero_elements:
    representation: any_shape_containing_a_zero_extent
    planning: no_dispatch_for_calculations_that_have_no_logical_lanes
```

The source of truth for the `Shape` and `AxisSet` records and methods is
`language/src/shape.rs:7-165`. The source of truth for layout is
`language/src/tensor.rs:5-206`; primitive shape equations are checked by
`language/src/primitive.rs:363-799`; and graph assembly and validation call
those checks from `language/src/graph.rs:18-287`.

## Shape representation

`Shape` is a fixed-rank, fixed-extent value:

```rust
pub struct Shape {
    extents: Vec<u64>,
    elements: u64,
}
```

The fields are private. The value derives `Clone`, `Debug`, `PartialEq`, `Eq`,
`PartialOrd`, `Ord`, and `Hash`. Because every public construction path runs
the same checked product, `elements` is a deterministic cache of `extents`.
Equality therefore represents equal ordered extents, not merely equal element
counts. For example, `[2, 3]` and `[1, 2, 3]` both have six elements but are
different shapes and different ranks.

### Rank and scalar convention

`Shape::new` rejects an empty extent vector. Rank zero is not an implicit
scalar representation. Its exact error is:

```text
EmptyShape: scalar payloads use shape [1]; rank-zero payload shapes are not implicit
```

The repository's scalar payload convention is therefore the explicit rank-one
shape `[1]`. `Shape` itself does not reject other one-element shapes such as
`[1, 1]`; callers that require a scalar use an exact `[1]` check. This explains
why reduction over every axis emits `[1]`, why operation helpers construct
`Shape::new(vec![1])`, and why a one-element metric buffer can still have its
own domain-level validation.

An extent may be zero. A zero extent does not make the rank zero. It makes the
cached product zero, so `Shape::is_empty()` reports a zero-element payload.
The name `EmptyShape` is reserved for the forbidden empty vector and must not
be confused with `is_empty() == true`.

### Construction

`Shape::new(extents: Vec<u64>) -> LanguageResult<Shape>` performs these steps
in order (`language/src/shape.rs:18-36`):

1. Reject `extents.is_empty()` with `LanguageErrorKind::EmptyShape`.
2. Start `elements` at one.
3. Fold through the supplied extents with `checked_mul`.
4. Return `LanguageErrorKind::ShapeOverflow` if any intermediate product does
   not fit in `u64`.
5. Store the original ordered vector and the resulting product.

The fold is performed in extent order and does not pre-scan for zero. Once a
zero has been multiplied into the accumulator, later products remain zero and
cannot overflow. An overflow before a later zero is still an error. Thus
`[0, u64::MAX, 2]` is accepted as a zero-element shape, while
`[u64::MAX, 2, 0]` fails at the second extent. The implementation does not
impose positivity, an int32 limit, a maximum rank, or a domain-specific upper
bound on an individual extent.

The direct construction errors are:

| Condition | Kind | Exact detail |
| --- | --- | --- |
| The extent vector is empty | `EmptyShape` | `scalar payloads use shape [1]; rank-zero payload shapes are not implicit` |
| A checked product overflows | `ShapeOverflow` | `shape element count overflowed u64` |

### Accessors and typed bytes

The read-only accessors (`language/src/shape.rs:38-60`) are:

| Method | Result | Contract |
| --- | --- | --- |
| `extents(&self)` | `&[u64]` | Borrows the ordered extent vector. No copy, sorting, or normalization occurs. |
| `rank(&self)` | `usize` | Returns `extents.len()`. For every valid `Shape`, it is at least one. |
| `elements(&self)` | `u64` | Returns the cached checked product. |
| `is_empty(&self)` | `bool` | Returns `elements == 0`, which is true exactly when at least one extent is zero. |
| `bytes(&self, dtype)` | `LanguageResult<ByteCount>` | Computes `elements * u64::from(dtype.byte_width())` with checked multiplication. |

`bytes` returns `LanguageErrorKind::ByteSizeOverflow` with detail
`typed tensor byte size overflowed u64` if the typed byte count does not fit.
The current `recipe_core::DType` domain contains `F32` and `I32`, and both have
`byte_width() == 4` (`core/src/scalar.rs:11-22`). A shape can therefore be
valid while its typed byte size overflows, for example `[u64::MAX]` has a valid
element count but cannot be represented as four-byte storage. A zero-element
shape returns `ByteCount::new(0)` for either current dtype.

`Shape` does not validate a layout, a backing allocation, an index value, an
int32 conversion, or an operation's semantic dimension policy. Those checks
belong to the consumers described below.

### Shape is not a physical index space

`recipe_core::IndexSpace` is a different type. It stores nonzero
`ElementCount` dimensions for an emitted lowered stage and rejects an empty
dimension vector or a zero dimension with `UnitError`. `Shape` is the earlier
semantic tensor metadata and deliberately permits zero extents and zero total
elements. Primitive lowering constructs an `IndexSpace` only after it has
proved that the relevant `Shape` is nonempty. Do not use `IndexSpace` as a
replacement for shape metadata, and do not make `Shape` adopt the physical
index-space nonzero rule.

## Shape transformations

The transformation methods construct a new `Shape`; they never mutate their
inputs. Every method retains the explicit-rank invariant by passing its
resulting vector through `Shape::new`.

### Broadcasting

`Shape::broadcast_result(inputs: &[&Shape]) -> LanguageResult<Shape>` is the
elementwise shape rule (`language/src/shape.rs:62-90`). It is equivalent to the
following deterministic procedure:

```text
rank = max(input.rank() for input in inputs), or 0 for no inputs
if rank == 0:
    return EmptyShape("broadcast requires at least one shaped input")
result = [1, 1, ..., 1] with length rank
for input in inputs in slice order:
    leading = rank - input.rank()
    for (index, extent) in input.extents in order:
        result_axis = leading + index
        current = result[result_axis]
        if current == 1:
            result[result_axis] = extent
        else if extent != 1 and extent != current:
            return ShapeMismatch
return Shape::new(result)
```

Axes are aligned from the trailing side. A lower-rank input never aligns to
axis zero merely because it is visited first. For example:

| Inputs | Result |
| --- | --- |
| `[2, 1, 4]`, `[3, 4]` | `[2, 3, 4]` |
| `[5, 1]`, `[1, 7]` | `[5, 7]` |
| `[0, 3]`, `[1, 3]` | `[0, 3]` |
| `[0, 3]`, `[2, 3]` | `ShapeMismatch` at the conflicting result axis |
| no inputs | `EmptyShape` |

The implementation treats extent one as the only expandable extent. A zero
extent can combine with one or another zero, but it conflicts with a distinct
non-one extent. The resulting extents are passed to `Shape::new`, so a
broadcast choice that exposes a product overflow returns `ShapeOverflow`.
The direct conflict error has kind `ShapeMismatch` and detail of the form:

```text
broadcast extent {extent} conflicts with {current} at result axis {axis}
```

`PrimitiveKind::Elementwise` calls this method after scalar-program, arity,
and dtype checks. Each elementwise output must equal the returned shape exactly
(`language/src/primitive.rs:363-428`). Concrete materialization uses the same
method for operations such as normalized batch statistics
(`ops/src/materialize.rs:1170-1198`).

### Reduction

`Shape::reduced(&self, axes: &AxisSet, keep_dimensions: bool)` computes the
shape of a reduction (`language/src/shape.rs:92-108`):

```text
validate axes against self.rank()
for each (axis, extent) in self.extents in original order:
    if axis is selected:
        append 1 when keep_dimensions is true
        append nothing when keep_dimensions is false
    otherwise:
        append extent
if the result vector is empty:
    append 1
return Shape::new(result)
```

Examples:

| Input | Axes | `keep_dimensions` | Result |
| --- | --- | --- | --- |
| `[2, 3, 4]` | `[1]` | `false` | `[2, 4]` |
| `[2, 3, 4]` | `[1]` | `true` | `[2, 1, 4]` |
| `[2, 3, 4]` | `[0, 1, 2]` | `false` | `[1]` |
| `[2, 3, 4]` | `[0, 1, 2]` | `true` | `[1, 1, 1]` |

`AxisSet::validate_rank` runs before the loop. An out-of-range axis returns
`InvalidAxis`. The call to `Shape::new` is retained even though most ordinary
reductions shrink the product: a zero extent can have masked an otherwise
overflowing product, and removing that zero can expose `ShapeOverflow` in the
result. A selected zero axis also has ordinary reduction semantics: removing
it can produce a nonempty output, while keeping it replaces it with one.

`PrimitiveKind::Reduce` uses the result as the exact shape of its value and/or
index outputs. It separately rejects Minimum and Maximum when a selected axis
has extent zero because those operators have no implicit empty-domain identity
(`language/src/primitive.rs:430-493`). Sum, Product, Any, and All can lower an
empty reduction through an explicit identity in the primitive lowerer.

### Gather and scatter shape substitution

`Shape::gather_result(&self, axis: usize, indices: &Shape)` replaces one
source axis with every axis of the index shape
(`language/src/shape.rs:110-122`):

```text
if axis >= self.rank():
    return InvalidAxis
result = self.extents[..axis]
       ++ indices.extents
       ++ self.extents[axis + 1..]
return Shape::new(result)
```

For source `[2, 3, 4]`, axis `1`, and index shape `[5, 6]`, the result is
`[2, 5, 6, 4]`. The index shape is itself always rank one or greater, so a
valid source and valid index shape produce a nonempty extent vector. Index
values are not inspected here, and index dtype or bounds policy is not part of
this method. The direct invalid-axis detail is:

```text
gather axis {axis} is outside rank {self.rank}
```

The replacement can expose a product overflow, which is returned as
`ShapeOverflow`. `PrimitiveKind::Gather` requires an `I32` index tensor and an
output equal to this result. `PrimitiveKind::Scatter` requires the output to
equal the base shape and the update tensor to equal this result
(`language/src/primitive.rs:594-625`). Concrete materialization repeats the
same checked shape calculation before emitting a gather
(`ops/src/materialize.rs:1110-1152`).

## `AxisSet`

`AxisSet` is the sorted, nonempty axis collection used by reductions and other
axis-parameterized builders:

```rust
pub struct AxisSet {
    axes: Vec<usize>,
}
```

Its fields are private and it derives `Clone`, `Debug`, `PartialEq`, and `Eq`.
Unlike `Shape`, it does not derive ordering or hashing. The constructor and
projections are (`language/src/shape.rs:125-164`):

| Method | Result | Contract |
| --- | --- | --- |
| `new(axes)` | `LanguageResult<AxisSet>` | Rejects an empty input, sorts ascending, and rejects duplicates. Rank is not known here. |
| `as_slice()` | `&[usize]` | Borrows the canonical ascending, unique axis list. |
| `contains(axis)` | `bool` | Uses binary search over the canonical list. |
| `validate_rank(rank)` | `LanguageResult<()>` | Rejects the first stored axis with `axis >= rank`; otherwise succeeds. |

The direct constructor errors are:

| Condition | Kind | Exact detail |
| --- | --- | --- |
| Input vector is empty | `InvalidAxis` | `axis set must not be empty` |
| Any axis occurs more than once | `DuplicateAxis` | `axis set contains a duplicate axis` |

`AxisSet::new(vec![3, 1, 3])` fails rather than silently deduplicating;
`AxisSet::new(vec![3, 1])` stores `[1, 3]`. A value such as `usize::MAX` is
accepted by the constructor and rejected only when a consumer supplies a rank
that it exceeds. `validate_rank(0)` rejects every validly constructed set
because the set cannot be empty. Its rank error is:

```text
axis {axis} is outside rank {rank}
```

Sorting is part of the value contract. It makes `contains` logarithmic, makes
OGDL output canonical, and makes reduction axes deterministic. It does not
change the semantic order of contraction axis pairs, which are separate
`Vec<(usize, usize)>` fields on `Contraction`.

## Tensor and layout consumers

`Shape` is metadata. `TensorLayout` and `Tensor` turn it into an affine view
and a typed backing-size contract (`language/src/tensor.rs:5-206`).

### Contiguous layout

`ContiguousOrder` has `RowMajor` and `ColumnMajor`. For a shape with extents
`[a, b, c]`, `TensorLayout::contiguous` produces offset zero and these strides:

| Order | Strides |
| --- | --- |
| Row major | `[b*c, c, 1]` |
| Column major | `[1, a, a*b]` |

The implementation computes the stride products with checked multiplication.
An overflow is `InvalidLayout` with detail `row-major stride overflowed` or
`column-major stride overflowed`. `Tensor::contiguous` always requests
row-major order, computes `shape.bytes(dtype)`, and stores the exact logical
typed size as `storage_bytes`. Direct callers may request column-major layout,
but no `Tensor` constructor selects it.

Shape validity and contiguous-layout validity are separate. A zero extent can
make the forward product in `Shape::new` zero while a reverse row-major stride
fold still encounters a huge trailing product before it reaches the zero. Such
a shape remains valid metadata but `Tensor::contiguous` may return
`InvalidLayout`. The shape module intentionally does not try to precompute or
normalize layout strides.

### Layout validation and span

`TensorLayout::validate(shape)` performs these checks:

1. `strides.len()` must equal `shape.rank()`, otherwise `InvalidLayout` reports
   `layout has {count} strides for rank {rank}`.
2. For a nonempty payload only, an extent greater than one may not have a zero
   stride. The error is `a non-singleton payload axis cannot have zero stride`.
3. For a nonempty payload only, axes with extent greater than one are sorted by
   stride. Each stride must begin at or after the span occupied by earlier axes;
   otherwise the layout is overlapping or broadcast and returns
   `tensor layout maps multiple logical elements to the same storage element`.
   Checked span arithmetic can return `layout non-overlap validation overflowed`.
4. `span_elements(shape)` must succeed.

Empty payloads intentionally skip the zero-stride and non-overlap checks. Their
span is zero even if the offset or strides contain large values. A direct call
to `span_elements` does not itself check the stride-vector rank; `validate`
does that first. For a nonempty shape, the span is the one-past-the-last
element address:

```text
last = offset_elements
for (extent, stride):
    last += (extent.saturating_sub(1)) * stride
return last + 1
```

Multiplication, addition, and the final increment are checked and report
`InvalidLayout` with details `layout span multiplication overflowed`,
`layout span addition overflowed`, or `layout span overflowed`. The
`byte_offset(dtype)` method is independent of a shape and checks only
`offset_elements * dtype.byte_width()`, returning `ByteSizeOverflow` with
detail `layout byte offset overflowed` when necessary.

`Tensor::validate()` maps layout failures to its `ValueId`, recomputes the span
in bytes, and rejects a backing object smaller than that span. A span-byte
overflow returns `ByteSizeOverflow` with detail `tensor layout span overflowed
bytes`, carrying the value ID. A too-small allocation returns `InvalidLayout`
with detail `layout requires {span} bytes but storage declares {storage}` and
also carries the value ID. It permits storage larger than the required span,
and it does not require `storage_bytes` to equal `Shape::bytes(dtype)` for an
arbitrary non-contiguous or offset view.

The shape, layout, and storage fields form one tensor storage contract. The
two external-boundary flags are lifecycle metadata and are not part of that
contract.

## Graph ownership and OGDL

`CalculationGraph` stores `Vec<Tensor>` and `Vec<CalculationNode>`. Shape
participates in graph ownership in three places:

* `CalculationGraph::assemble` accepts repeated declarations of one tensor ID
  only when dtype, shape, layout, and storage bytes match exactly. Boundary
  flags are discarded from fragments and rebuilt from the caller's explicit
  external input and output sets (`language/src/graph.rs:18-76`). A shape
  conflict is `DuplicateTensor` with the tensor value context.
* `CalculationGraph::validate` validates every tensor before validating any
  primitive kernel (`language/src/graph.rs:78-138`). It therefore rejects
  malformed layouts and undersized storage before operation-specific shape
  equations run. Producer, duplicate-ID, and cycle errors are graph errors,
  not shape errors.
* `same_storage_contract` compares `left.shape == right.shape` in addition to
  dtype, layout, and storage bytes (`language/src/graph.rs:282-287`). Equal
  element counts with different rank or extents do not satisfy this contract.

The canonical OGDL codec writes tensor shape as an ordered list of repeated
`extent` fields under `tensor.shape` (`language/src/ogdl.rs:147-166`). It does
not serialize rank or the cached `elements` field. Decoding parses every
extent as `u64`, calls `Shape::new`, and then builds the layout and storage
records (`language/src/ogdl.rs:535-595`). Thus an empty extent list produces
`EmptyShape`, a checked-product overflow produces `ShapeOverflow`, and a zero
extent is retained. A `LanguageError` from decoding is wrapped as
`OgdlCodecError::InvalidGraph`; strict document errors such as missing fields,
duplicate fields, invalid numbers, or unknown variants are reported before the
shape constructor. The completed graph is validated after all tensors and
nodes are decoded.

Reduction axes are encoded as repeated `axis` fields. Decoding calls
`AxisSet::new`, so an empty or duplicate list fails immediately; rank bounds
are checked later by primitive validation. OGDL encoding validates the graph
first, so emitted text cannot contain an unvalidated shape or layout.

## Primitive validation that consumes shapes

`PrimitiveKernel::validate` first resolves all input and output `ValueId`s to
`Tensor`s, validates the alias matrix, and then selects a kind-specific shape
rule (`language/src/primitive.rs:207-249`). Shape failures from this stage are
annotated with the kernel ID by `LanguageError::for_kernel`. The complete
shape-facing matrix is:

| Primitive kind | Shape rule | Shape or axis failures |
| --- | --- | --- |
| `Elementwise` | Broadcast every input with `Shape::broadcast_result`; every output must equal the result. | Broadcast `EmptyShape`, `ShapeMismatch`, or `ShapeOverflow`; output mismatch is `ShapeMismatch`. |
| `Reduce` | Validate `AxisSet` against input rank; output shape is `input.shape.reduced(axes, keep_dimensions)`. Every value/index output must equal it. | `InvalidAxis`, `ShapeOverflow`, or output `ShapeMismatch`; Minimum/Maximum over a selected zero extent is `InvalidPrimitive` because there is no identity. |
| `Scan` | Axis must be less than input rank; output shape must equal input shape. | `InvalidAxis` or output `ShapeMismatch`. Zero-element input is valid language metadata and is handled by no-dispatch lowering. |
| `Contraction` | Validate every ordered batch and contract pair, require equal paired extents, then output batch extents followed by free left extents and free right extents. If none remain, output is `[1]`. | An axis pair outside either operand rank is `InvalidAxis`; reused operand axes are `DuplicateAxis`; unequal paired extents are `ShapeMismatch`; expected construction can return `ShapeOverflow`; output mismatch is `ShapeMismatch`. |
| `Gather` | Output equals `values.shape.gather_result(axis, indices.shape)`. | `InvalidAxis`, `ShapeOverflow`, or output `ShapeMismatch`. Indices must separately be `I32`. |
| `Scatter` | Output equals the base shape; updates equal `base.shape.gather_result(axis, indices.shape)`. | Same gather errors plus output/update `ShapeMismatch`. |
| `Histogram` | Output is exactly `[bins]`; weighted values must have equal input and weight shapes. | Bin range is an `InvalidPrimitive`; shape disagreement is `ShapeMismatch`. |
| `Sort` | Values output and optional index output equal input shape. | Axis out of rank is `InvalidAxis`; an axis above `i32::MAX` is `InvalidPrimitive`; shape disagreement is `ShapeMismatch`. |
| `IndexMap` | No input shape equation. The output is any valid tensor shape and must be `I32`. | Shape-specific rejection is delegated to tensor validation and lowering. |
| `Random` | No input shape equation. The output is any valid tensor shape, with dtype determined by its distribution. | Shape-specific rejection is delegated to tensor validation and lowering. |

Contraction output order is intentionally not a generic reshape. Batch pairs are
appended in the order stored in `batch_axes`; free axes retain each operand's
original order. Contracted extents contribute to the reduction domain but do
not appear in the output. This ordered equation is also consumed by primitive
lowering and operation-family matching.

`PrimitiveKernel::work` calls `validate` before using `Shape::elements()` and
`Shape::extents()` to price work. Elementwise work is scalar-program work per
output element, reductions use input and output element counts, contractions
multiply contracted extents by output elements, and gather/scatter,
histogram, sort, random, and index-map work all use shape-derived counts. A
checked work formula overflow is `WorkOverflow` with kernel context, not a
shape-construction error.

## Builders and materialization boundaries

Shape creation is deliberately separate from scalar SSA creation. The public
`ScalarProgramBuilder` (`language/src/scalar_builder.rs:11-175`) owns scalar
value IDs, dtypes, op signatures, and program validation. It imports neither
`Shape` nor `AxisSet`. An elementwise builder can therefore produce a valid
scalar program without choosing a tensor shape; `PrimitiveKernel::validate`
combines that program with tensor inputs and applies the broadcast rule. A
foreign scalar expression, invalid opcode signature, or scalar output error is
`InvalidScalarProgram`, not a shape error.

The operation and training graph builders use the same shape-to-tensor
sequence for shaped intermediates:

```text
extent values
  -> Shape::new
  -> Tensor::contiguous(value, dtype, shape, flags)
  -> workspace or external-byte accounting
  -> CalculationGraph::validate
  -> primitive validation and lowering
```

The principal consumers are:

* `ops/src/materialize.rs:385-606` keeps an iteration `Shape`, resolves
  shape-extent and minimum-extent composition bounds, and reports an invalid
  bound as `IterationBoundUnresolved` at the operation boundary. Its
  `GraphBuilder::intermediate` (`ops/src/materialize.rs:712-846`) calls
  `Tensor::contiguous`, adds the resulting typed storage bytes to workspace,
  enforces the configured workspace limit, and validates the finished graph.
  Concrete materializers use `Shape::broadcast_result` for normalized
  tensors, `Shape::gather_result` for checked indexing, exact extent checks
  for operation ABIs, and `AxisSet::new` plus `validate_rank` for reductions.
* The specialized operation emitters in `ops/src/bayes.rs`,
  `ops/src/binary_metrics.rs`, `ops/src/kmeans.rs`, `ops/src/knn_outputs.rs`,
  and `ops/src/tree.rs` centralize their local `shape(&[u64])` helper on
  `Shape::new`, then call `Tensor::contiguous` for each intermediate. Their
  reduction helpers construct `AxisSet`; exact request shapes are checked
  before graph emission. A shape or layout language error is converted to the
  operation's graph-materialization error, while domain policies such as
  positive row counts, int32 index limits, and workspace limits remain
  operation errors.
* `training/src/forward.rs:80-364` passes explicit `Shape` values through the
  `RecurrentForwardGraph` interface. RNN, GRU, and LSTM state tensors use
  `[rows, width]`; each constructor's `LanguageError` is converted by the
  training or inference compiler. This interface does not infer a shape from a
  scalar program.
* `training/src/compile.rs:2680-2859` and `training/src/inference.rs:1792-1909`
  build tensors with `Tensor::contiguous`, compute external byte contracts with
  `Shape::bytes`, and compare supplied value counts with `Shape::elements`.
  Their exact tensor checks compare dtype and ordered extents. Attention and
  other reinterpretation paths require equal element counts explicitly, then
  use a checked gather to materialize the new ordered shape; there is no
  general `reshape` method on `Shape`.
* `training/src/compile.rs` and `training/src/inference.rs` construct
  `AxisSet` for reductions and round-trip the completed graph through OGDL.
  The round trip re-runs `Shape::new`, layout validation, graph validation, and
  primitive shape equations before a compiled program is accepted.

Most helpers intentionally propagate `Shape::new` or `AxisSet::new` with `?`.
Where an operation must report its own identity, it maps the language error's
display text into `OperationError`, `TrainingCompileError`, or
`InferenceCompileError`. This preserves the one shape implementation rather
than adding operation-specific shape wrappers.

### Direct call-site inventory

The direct constructor and transformation call sites in the current workspace
are intentionally thin wrappers around this module:

| Consumer group | `Shape` calls | `AxisSet` calls | Boundary behavior |
| --- | --- | --- | --- |
| `recipe-language` | OGDL tensor decoding; contraction and histogram expected-output construction; elementwise, reduce, gather, and scatter shape equations. | OGDL reduce decoding; reduce validation. | Errors remain `LanguageError`, with kernel context added by primitive validation or `InvalidGraph` added by OGDL. |
| Operation graph emitters | `ops/src/bayes.rs`, `binary_metrics.rs`, `kmeans.rs`, `knn_outputs.rs`, `tree.rs`; `ops/src/materialize.rs`; materializer modules `attention_sequence_embedding.rs`, `loss_metrics.rs`, `graph_cluster_rl.rs`, `indexing_sort_encoding.rs`, and `tree_boosting.rs`. | The same emitters plus `convolution_pooling.rs` and operation-level reduction helpers. | Shape and axis errors become graph-materialization or operation errors after request and tensor validation. |
| Training forward and compile | `training/src/forward.rs` recurrent state construction; `training/src/compile.rs` shape helper and all tensor, normalization, attention, tree, and parameter paths. | `training/src/compile.rs` reduction and materialized primitive construction. | `LanguageError` is converted to `TrainingCompileError`; exact byte and element counts are checked against prepared data. |
| Inference compile | `training/src/inference.rs` shape helper, checkpoint tensors, recurrent states, attention reinterprets, and outputs. | `training/src/inference.rs` reduction construction. | `LanguageError` is converted to `InferenceCompileError`; checkpoint bytes and ordered extents must match. |

No call site creates a second shape type or stores a guessed rank. Callers either
construct a `Shape`, pass it to `Tensor::contiguous`, or compare an existing
tensor's ordered extents against an independently derived expected vector.

## Empty payloads and lowering

The shape source comment states that zero-element shapes are valid metadata and
are resolved as no-dispatch calculations rather than fake one-lane kernels.
The primitive lowerer is the downstream proof of that rule
(`primitives/src/lower.rs`):

* Elementwise, scan, contraction, gather, sort, random, and index-map lowering
  return without an emitted dispatch when the relevant shape is empty.
* Reduction lowering returns without a stage when the output is empty. When a
  selected reduction axis is empty but the output still has elements, it emits
  an identity fill for Sum, Product, Any, and All. Minimum and Maximum were
  rejected by language validation before this path.
* Scatter emits a base copy only when the base shape is nonempty and emits no
  update stage when updates are empty.
* Histogram always clears its positive-size `[bins]` output, then skips input
  accumulation when the input shape is empty.

Every emitted lowered stage must have nonzero logical lanes
(`primitives/src/validate.rs:221-270`). Shape emptiness is therefore a control
point for stage creation, not a request to synthesize a dispatch with one lane.
The lowerer also converts nonempty extents into `recipe_core::IndexSpace`
dimensions, which require nonzero `ElementCount` values. The early empty-shape
checks are what keep that downstream type from seeing a zero dimension.

The same distinction appears in static access validation: a logical extent
vector containing zero requires zero address bytes, and empty writable views
do not fail the non-overlap check (`primitives/src/validate.rs:149-219`). This
does not change the `Shape` invariant or make a zero-element shape rank zero.

## Failure and context propagation

`LanguageErrorKind` and its context fields are defined in
[`language/src/error.rs`](../../src/error.rs):

```rust
pub struct LanguageError {
    pub kind: LanguageErrorKind,
    pub detail: String,
    pub value: Option<ValueId>,
    pub kernel: Option<KernelTemplateId>,
}
```

Shape and axis methods create context-free errors. Consumers add context with
`for_value` or `for_kernel`; `Display` prints the kind and detail followed by
`[kernel ...]` and `[value ...]` when present. The shape-facing direct kinds are
`EmptyShape`, `InvalidAxis`, `DuplicateAxis`, `ShapeOverflow`,
`ByteSizeOverflow`, and `ShapeMismatch`. `InvalidLayout` is the tensor-level
kind for stride and span failures. `ArityMismatch`, `DTypeMismatch`,
`InvalidPrimitive`, and `WorkOverflow` belong to primitive validation or work
accounting even when a shape helped expose the condition.

The normal propagation chain is:

```text
Shape or AxisSet method
  -> LanguageResult<...>
  -> Tensor::contiguous or Tensor::validate (optional ValueId context)
  -> PrimitiveKernel::validate (KernelTemplateId context)
  -> CalculationGraph::validate
  -> OGDL InvalidGraph, operation error, training compile error, or inference error
```

No layer silently substitutes a rank, drops an axis, clamps an extent, or
creates a fallback shape. Domain-specific callers may reject an otherwise
valid shape, for example a concrete matrix materializer requiring rank two and
positive int32 dimensions, but that policy is reported at that caller's
boundary rather than added to `Shape`.

## Non-goals and reading rules

When reading or extending this contract:

* Do not introduce a second dimension abstraction. Use `u64` extents and
  `usize` axes already carried by `Shape` and `AxisSet`.
* Do not treat `is_empty()` as rank-zero. Rank zero is impossible; empty means
  zero logical elements.
* Do not infer a scalar from `elements() == 1`. The repository's scalar payload
  spelling is `[1]`, and exact scalar consumers check that extent vector.
* Do not use `Shape` to validate strides, offsets, backing bytes, index values,
  dtype compatibility, or hardware limits. Use `TensorLayout`, `Tensor`, the
  primitive validator, or the domain materializer that owns that rule.
* Do not replace `reduced` or `gather_result` with an unchecked vector splice.
  Both call the checked constructor so zero extents and product overflow keep
  the same failure behavior as direct construction.
* Do not bypass `AxisSet::new` by relying on a caller's ordering. Its sorted,
  unique invariant is required by `contains`, canonical OGDL, and reduction
  lowering.

The shape module is intentionally small. Its purpose is to make every tensor
shape explicit, rankable, comparable, serializable, and safe to use in checked
element-count and byte-count equations before any backend-specific work is
allowed to exist.
