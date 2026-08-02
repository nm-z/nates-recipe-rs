# Tensors, shapes, and affine storage

This document describes the tensor metadata contract in
[`language/src/tensor.rs`](../../src/tensor.rs), together with the shape
operations that feed it and the graph and lowering boundaries that consume it.
The language crate is backend-neutral. A `Tensor` does not contain payload
values, a device allocation, a queue, or a native handle. It names one typed,
statically shaped value and the affine view into its logical backing storage.

The public reexports are in [`language/src/lib.rs`](../../src/lib.rs):
`ContiguousOrder`, `Tensor`, `TensorLayout`, `AxisSet`, and `Shape`. The
primitive kernel validators in
[`language/src/primitive.rs`](../../src/primitive.rs) are the first consumers
that derive operation output shapes. Graph validation then proves that every
tensor and kernel form one complete acyclic calculation graph before the
planner lowers it.

## Contract at a glance

| Record | Meaning | Mutable payload owned here? |
| --- | --- | --- |
| `Shape` | Fixed rank and fixed extents, plus a checked element count | No |
| `TensorLayout` | Element offset and one element stride per shape axis | No |
| `Tensor` | Value identity, dtype, shape, layout, backing-byte bound, and graph boundary flags | No |
| `CalculationGraph` | Tensor records and primitive-kernel nodes with producer and dependency rules | No |

The only calculation types are `recipe_core::DType::F32` and `I32`. Both have
`byte_width() == 4`. Tensor offsets, strides, and storage sizes are measured
in unsigned `u64` element or byte units and all derived arithmetic is checked.
Placement later turns `storage_bytes` into an arena allocation. It does not
change the semantic tensor shape or layout.

## Shape model

`Shape` is a fixed-rank, fixed-extent value. Its constructor is
`Shape::new(Vec<u64>)`.

### Invariants

* The extent vector must not be empty. A rank-zero payload is not implicit. A
  scalar payload is represented by the explicit shape `[1]`.
* Every extent is an unsigned `u64`; zero is allowed. An extent of zero makes
  the shape empty, but does not make the metadata invalid.
* `elements` is the checked product of all extents. The product is zero if any
  extent is zero. A product overflow returns `LanguageErrorKind::ShapeOverflow`.
* `extents()` returns the original axis order. `rank()` is its length,
  `elements()` returns the cached product, and `is_empty()` is equivalent to
  `elements() == 0`.

`Shape::bytes(dtype)` multiplies the cached element count by the four-byte
`DType` width. The multiplication is checked and returns
`LanguageErrorKind::ByteSizeOverflow` if it cannot be represented by `u64`.
For an empty shape it returns `ByteCount::new(0)`.

The distinction between an empty shape and a missing shape is important. The
language accepts empty metadata so a planner can resolve a no-dispatch
calculation. The core native `IndexSpace` type requires every launch
dimension to be nonzero, so an empty tensor is never converted into a fake
one-lane native launch.

### Shape operations

The operation helpers are all in [`language/src/shape.rs`](../../src/shape.rs).

#### Broadcasting

`Shape::broadcast_result(inputs)` computes the right-aligned broadcast shape.
It rejects an empty input list with `EmptyShape`. It starts every result axis
at `1`, aligns each input at the trailing axes, and accepts an input extent if
it is `1` or equals the extent already selected for that result axis. Any two
other extents conflict and return `ShapeMismatch`.

Zero is an ordinary extent in this rule. Therefore zero can broadcast with
zero or one and the result remains zero; zero conflicts with a nonzero extent
other than one. The resulting extent vector is passed back through
`Shape::new`, so the rank and product invariants still apply.

For example, shapes `[4, 1, 8]` and `[1, 3, 8]` produce `[4, 3, 8]`. Shapes
`[4, 2]` and `[4, 3]` fail at the trailing axis. A shape with fewer axes is
aligned at the right, not padded at the front in the stored value.

#### Reduction

`Shape::reduced(axes, keep_dimensions)` first calls `AxisSet::validate_rank`.
For every selected axis it either removes the axis or emits extent `1` when
`keep_dimensions` is true. Unselected axes retain their original extents. If
all axes are removed, the helper emits `[1]` rather than a rank-zero result.
The resulting vector is constructed with `Shape::new`, so a surviving zero
extent remains empty.

#### Gather and scatter shape relation

`Shape::gather_result(axis, indices)` requires `axis < self.rank()`. Its result
is the source prefix before `axis`, followed by every extent of the indices
shape, followed by the source suffix after `axis`. The indices shape is itself
nonempty in rank, so the result always has at least one axis. An invalid axis
returns `InvalidAxis`.

Scatter uses the same relation for its updates tensor: the base and output
shapes are equal, and the updates shape must equal
`base_shape.gather_result(axis, indices_shape)`.

### Axis sets

`AxisSet::new(Vec<usize>)` requires at least one axis, sorts the values in
ascending order, and rejects duplicates with `DuplicateAxis`. `as_slice()`
returns the canonical sorted slice and `contains(axis)` uses binary search.
Construction does not know a tensor rank. `validate_rank(rank)` is the later
check that rejects the first axis greater than or equal to `rank` with
`InvalidAxis`.

## Tensor layout

`TensorLayout` is the affine element-address metadata paired with a `Shape`:

```text
logical element at coordinates (i0, i1, ...) maps to
    offset_elements + i0 * strides[0] + i1 * strides[1] + ...
```

`offset_elements` and every stride are counts of typed elements, not bytes.
`TensorLayout` is metadata only. It is not a scalar-program input and it does
not describe payload arithmetic.

### Contiguous layouts

`TensorLayout::contiguous(shape, order)` starts with `offset_elements == 0`
and computes one stride per axis with checked `u64` multiplication.

| `ContiguousOrder` | Stride for axis `i` |
| --- | --- |
| `RowMajor` | Product of extents after `i` |
| `ColumnMajor` | Product of extents before `i` |

For shape `[2, 3, 4]`, row-major strides are `[12, 4, 1]` and column-major
strides are `[1, 2, 6]`. A stride-product overflow returns
`InvalidLayout`. Zero extents can make subsequent computed strides zero; this
is accepted because an empty payload has no addresses to collide. The
`Tensor::contiguous` convenience constructor always selects row-major order.

### Layout validation

`TensorLayout::validate(shape)` performs four checks:

1. The stride vector length must equal the shape rank.
2. For a nonempty shape, a non-singleton axis (`extent > 1`) may not have a
   zero stride. A singleton axis may have zero stride because it names only one
   logical element. All zero-stride and overlap checks are skipped for an empty
   shape.
3. For a nonempty shape, axes with `extent > 1` are sorted by stride. Starting
   with an occupied span of one element, each next stride must be at least the
   occupied span. The span is then extended by `(extent - 1) * stride`. A
   smaller stride would map two logical coordinates to one storage element and
   returns `InvalidLayout`. Checked arithmetic protects this validation itself.
4. `span_elements(shape)` must succeed. Its result is the exclusive element
   bound of the affine view, including the offset.

The non-overlap check makes the language tensor view injective. It does not
require a dense layout: padding and larger strides are valid if the occupied
ranges do not overlap and fit the declared backing storage. Read-only
elementwise broadcast views are the deliberate exception introduced later by
primitive lowering, where a singleton input axis can receive a zero stride in
the lowered access without changing the source tensor's layout contract.

`span_elements(shape)` returns zero immediately for an empty shape. Otherwise
it computes:

```text
offset_elements + sum((extent - 1) * stride) + 1
```

Each multiplication and addition is checked. Overflow returns `InvalidLayout`.
The method uses `saturating_sub(1)` in its empty-shape-independent loop, but
the early empty-shape return means a zero extent cannot silently create a
nonzero span.

`byte_offset(dtype)` converts only `offset_elements` to bytes. It multiplies
by `dtype.byte_width()` and returns a `recipe_core::ByteOffset`; overflow is a
`ByteSizeOverflow`. It does not validate the shape, strides, or backing
storage, so callers that need a complete contract must call `validate` as well.

## Tensor construction and validation

`Tensor` has public fields because graph materializers and the OGDL codec carry
the complete static record:

| Field | Contract |
| --- | --- |
| `id: ValueId` | Stable logical value identity. It is unique within one graph. |
| `dtype: DType` | F32 or I32, four bytes per element. |
| `shape: Shape` | Fixed rank, extents, and checked element count. |
| `layout: TensorLayout` | Element offset and rank-matched affine strides. |
| `storage_bytes: ByteCount` | Declared size of the logical backing object before placement. |
| `external_input` | Final graph boundary flag for host or other external admission. |
| `external_output` | Final graph boundary flag for egress. |

The flags are graph-boundary metadata, not producers or consumers. A fragment
may set temporary flags, but `CalculationGraph::assemble` discards those
fragment flags and applies the caller's exact boundary sets.

### `Tensor::contiguous`

`Tensor::contiguous(id, dtype, shape, external_input, external_output)` is the
normal constructor used by operation materializers. It:

1. Computes a row-major `TensorLayout` with zero offset.
2. Computes `storage_bytes` with `Shape::bytes(dtype)`.
3. Stores the supplied identity and boundary flags.

It does not read or allocate payload data. It can fail with `InvalidLayout`
from stride construction or `ByteSizeOverflow` from typed byte sizing.

For example, a `[2, 3]` F32 tensor has strides `[3, 1]`, six logical elements,
and 24 declared backing bytes. A manually constructed padded or offset tensor
may declare more storage than `Shape::bytes`, but its affine span still must
fit that storage.

### `Tensor::validate`

`Tensor::validate()` is the value-level boundary used by graph validation. It
first calls `layout.validate(shape)` and attaches the tensor's `ValueId` to a
layout error. It then recomputes the layout span, multiplies it by the dtype
width with checked arithmetic, and rejects a span larger than
`storage_bytes`:

```text
required_bytes = layout.span_elements(shape) * dtype.byte_width()
required_bytes <= storage_bytes
```

The byte multiplication returns `ByteSizeOverflow` with the value identity.
An insufficient backing bound returns `InvalidLayout` with the required and
declared byte counts. This check is independent of the logical dense size:
`storage_bytes` is a capacity bound for the affine view, not a promise that the
view is packed.

## Graph ownership and tensor contracts

[`language/src/graph.rs`](../../src/graph.rs) owns the graph-level rules. A
`CalculationGraph` contains `Vec<Tensor>` and `Vec<CalculationNode>`, where a
node wraps one `PrimitiveKernel`.

### Assembly

`CalculationGraph::assemble(fragments, external_inputs, external_outputs)`
canonicalizes independently materialized fragments:

1. Each boundary set is made unique. Repeated values produce
   `DuplicateTensor`.
2. Every boundary value must exist in at least one fragment, otherwise
   `UnknownTensor` is returned.
3. Repeated declarations of one `ValueId` are accepted only when `id`, dtype,
   shape, layout, and `storage_bytes` match exactly. Boundary flags are the
   only fields allowed to differ. A mismatch is `DuplicateTensor` with the
   value identity.
4. The supplied boundary sets replace every fragment's flags. Tensor records
   are sorted by `ValueId`, kernel nodes by kernel identity, and the resulting
   graph is validated before it is returned.

### Validation order

`CalculationGraph::validate()` builds an ID index, validates every tensor, and
then validates every kernel. It also enforces:

* Tensor IDs are unique (`DuplicateTensor`).
* Kernel IDs are unique (`DuplicateKernel`).
* Primitive inputs and outputs refer to existing tensors (`UnknownTensor`).
* A tensor output has at most one producer (`DuplicateProducer`).
* An external input has no producer. A non-external tensor has a producer
  (`DuplicateProducer` or `MissingProducer`).
* Kernel input dependencies form an acyclic graph. A self-consumption or
  unresolved cycle returns `Cycle`.

`topological_order()` repeats validation and returns a deterministic order by
kernel identity among ready nodes. `dependencies(kernel)` also repeats
validation, returns the sorted unique producer IDs for that kernel's inputs,
and returns `InvalidPrimitive` if the requested kernel is absent.

The graph does not infer tensor shapes from kernel names. Every primitive
validator consumes the already materialized `Tensor` records, so a shape or
layout mismatch is reported before planning or device placement.

## Primitive shape consumers

`PrimitiveKernel::validate` first resolves all input and output `ValueId`s,
requires a complete input/output alias matrix, and dispatches to one
family-specific validator. The shape rules below are the source of truth for
operation output tensors.

| Primitive | Shape rule and important failure boundary |
| --- | --- |
| `Elementwise` | Inputs must be nonempty. `Shape::broadcast_result` determines one output shape. Every output must have that exact shape and the scalar program's output dtype. Input dtypes and scalar input dtypes must match. Broadcast conflicts are `ShapeMismatch`; empty input arity is `ArityMismatch`. |
| `Reduce` | One input. `AxisSet` must fit the input rank. `Shape::reduced` determines the value and optional index output shape. Index results are I32 and are allowed only for minimum or maximum. Any and All require I32. Minimum and Maximum reject a selected axis with extent zero because no implicit identity exists. |
| `Scan` | One input and one output. The output shape and dtype equal the input. The scan axis must be in range, and an exclusive identity must have the input dtype. |
| `Contraction` | Two same-dtype inputs and one same-dtype output. Each batch or contract axis pair must be in range, use each operand axis at most once, and have equal extents. Output extents are batch extents, all unpaired left extents, then all unpaired right extents. If no axes remain, the output is `[1]`. |
| `Gather` | Values and I32 indices are the two inputs. The output dtype equals values and its shape is `values.shape.gather_result(axis, indices.shape)`. |
| `Scatter` | Base, I32 indices, and updates are the three inputs. Output shape and dtype equal base. Updates must have the gather-result shape. Conflict mode controls whether output writes are ordinary or atomic. |
| `Histogram` | Input count is one or two when weighted. Weighted values are F32 and shape-equal to the input. Output dtype is I32 for unweighted or F32 for weighted, and output shape is `[bins]`. Bin count is in `1..=i32::MAX`. |
| `Sort` | One input. Values output preserves the input shape and dtype. Optional index output preserves shape and is I32. The selected axis must be in range and its extent must fit the I32 index contract. |
| `IndexMap` | No inputs and one I32 output. The caller supplies the output tensor shape; an optional modulus must be strictly positive. |
| `Random` | No inputs and one output. The caller supplies shape. Distribution selects F32 or I32 output dtype, and Recipe requires exactly ten Philox rounds. |

`PrimitiveKernel::work` calls the same validator before pricing work. It uses
tensor element counts and selected extents for checked FLOP or operation bounds;
overflow is `WorkOverflow`. Consequently, changing a tensor shape changes both
semantic validation and the static scheduling identity.

## Operation and program consumers

The operation materializers in `ops/src/materialize.rs` receive named input and
output tensors that already carry their complete contracts. They require every
input to be marked `external_input`, every output to be non-input and marked
`external_output`, and use shape and dtype checks before emitting a primitive.
`require_same_tensor_contract` compares dtype and shape for operation peers;
the final `CalculationGraph::validate` still checks each complete layout and
storage bound.

Materialized intermediates are created only through
`Tensor::contiguous(value, dtype, shape, false, false)`. The materializer records
the resulting `storage_bytes` in its `WorkspaceObject` list and adds the bytes
with checked arithmetic against the operation workspace limit. A tensor
construction or shape error is reported as an operation graph-materialization
failure, while a concrete operation shape restriction is reported as an
unsupported concrete shape.

The training and inference graph compilers follow the same pattern: they create
intermediates with `Tensor::contiguous`, validate input byte vectors against
`Shape::bytes(dtype)`, set external flags only after the graph has been built,
then call graph validation and perform an OGDL encode/decode round trip. This
means a prepared matrix whose byte length is not exactly its typed shape size
fails before native preparation, and a graph that changes shape or layout while
being serialized cannot pass the canonical round trip.

The specialized Bayes, K-means, tree, KNN, and metric materializers add the
same boundary in their own error domains. They call `Tensor::validate` on
caller-provided tensors, require operation-specific dtype and extent formulas,
clone boundaries with explicit external flags, create only contiguous
intermediates, accumulate workspace bytes with checked arithmetic, and finish
by calling `CalculationGraph::validate`. Their operation-specific shape errors
therefore occur before primitive lowering, while malformed affine storage still
surfaces as a tensor validation error.

`program/src/lib.rs` adds one shape-sensitive consumer for loop metrics. A
metric value must have exactly one logical element (`shape.elements() == 1`)
and exactly four backing bytes. The validator does not require rank one, so
all-one shapes such as `[1, 1]` are still one logical element. The value must
also be produced by a calculation, remain non-external, and be covered by the
metric's iteration domain.

## OGDL persistence

The canonical graph codec is in
[`language/src/ogdl.rs`](../../src/ogdl.rs). Encoding calls complete graph
validation first, so an emitted document cannot intentionally contain an
invalid tensor contract. Each tensor is encoded with these exact fields:

```text
tensor
  id <u64>
  dtype F32 | I32
  shape
    extent <u64> ...
  layout
    offset_elements <u64>
    strides
      stride <u64> ...
  storage_bytes <u64>
  external_input true | false
  external_output true | false
```

Decoding requires every field, rejects duplicate or unknown fields, parses each
extent and stride as an unsigned number, constructs the `Shape`, constructs the
`TensorLayout`, and then validates the complete `CalculationGraph`. Thus a
document can be syntactically valid while still failing with an invalid graph
because its stride rank, affine span, storage bound, producer relation, or
primitive shape relation is wrong. No missing shape, stride, storage, or
boundary value receives a code-side default.

## Lowering path and metadata propagation

The tensor contract remains visible all the way to native address generation:

```text
CalculationGraph::validate
  -> planner::lower_programs
  -> recipe_primitives::lower
  -> PrimitiveKernel::validate
  -> ProgramBuilder tensor buffers and stages
  -> LoweredProgram::validate
  -> kernel stage or LLVM lowering
```

The primitive lowering implementation is in
[`primitives/src/lower.rs`](../../../primitives/src/lower.rs). For each logical
tensor it creates one `ProgramBuffer` whose dtype, shape, logical extents,
offset, strides, and storage byte bound copy the language tensor. Scratch
buffers are separate one-dimensional buffers. The tensor itself is not
duplicated as a new semantic value.

Elementwise lowering right-aligns an input view to the output extents. An input
axis with extent one that is expanded in the output receives a zero physical
stride, which is legal for a read-only broadcast binding. Outputs retain their
language layout. Other primitive stages retain the language view and add only
the scratch or fault buffers required by their algorithm.

The core `IndexSpace` and `StaticBufferAccess` types in
[`core/src/scalar.rs`](../../../core/src/scalar.rs) then receive the nonempty
output or input extents. Native kernels launch a flattened one-dimensional
index space. Kernel address generation reconstructs coordinates from that
linear lane and applies the stored offset and strides. It does not use a
backend grid rank to define tensor semantics.

### Empty-shape lowering

An empty language shape has valid metadata but cannot become a core
`IndexSpace`, because `ElementCount::new(0)` is rejected. The primitive
lowerers therefore resolve empty cases without a fake dispatch:

| Lowerer | Empty behavior |
| --- | --- |
| Elementwise | Empty output returns a program with no map stage. |
| Reduce | Empty output returns no stage. A nonempty output whose selected reduction axis has extent zero uses an identity fill for Sum, Product, Any, or All. Minimum and Maximum were rejected by language validation. |
| Scan | Empty input or a zero-width scan axis returns no stage. |
| Contraction | Empty output returns no stage. |
| Gather | Empty output returns no stage. |
| Scatter | Copies the nonempty base first. If updates are empty, no update stage is added. |
| Histogram | Always emits the nonempty bin-clear stage. Empty input omits the accumulation stage. |
| Sort | Empty input or a zero-length sort axis returns no stage. |
| Random and IndexMap | Empty output returns no stage. |

These cases are a lowering policy, not a new tensor shape. The graph still
contains the declared tensor and its producer relationship, and planner state
must account for the no-dispatch result.

### Static-access validation and typed lowering failures

[`primitives/src/validate.rs`](../../../primitives/src/validate.rs) checks
every lowered buffer and binding. It requires:

* access logical-extents rank equals stride rank;
* nonempty affine addresses fit in `u64` and the required typed bytes fit in
  `storage_bytes`;
* non-atomic writable mappings are injective, so broadcast and overlap cannot
  become ordinary writes;
* a binding dtype equals its buffer dtype and its view does not exceed the
  buffer's backing storage;
* fault flags are exactly one I32 element and four bytes.

Atomic read-write mappings are exempt from the ordinary injectivity check
because their conflict semantics are explicit in the stage's atomic contract.
`ProgramBuilder::finish` computes the canonical digest and validates the full
`LoweredProgram`; any violation becomes `LoweringErrorKind::InvalidLoweredProgram`.

The primitive lowering error boundary is deliberately narrow:

| Error kind | Meaning at the tensor boundary |
| --- | --- |
| `InvalidLanguage` | The kernel or its tensor shape/layout contract failed language validation. |
| `MissingTensor` | A kernel value was absent from the lowering tensor index. |
| `ArithmeticOverflow` | A stride, address, element count, scratch size, stage ID, or resource bound exceeded its static integer representation. |
| `InvalidStaticAccess` | Broadcast rank or an affine view could not form the required lowered access. |
| `InvalidLoweredProgram` | The produced buffers, bindings, stages, resources, alias matrix, or digest failed final validation. |

The planner maps primitive lowering errors and invalid lowered programs to
`PlannerErrorKind::InvalidGraph`, so a shape or layout failure is reported as
an invalid candidate graph rather than silently falling back to another tensor
interpretation.

### Native stage assumptions and downstream failures

The native stage emitter in
[`kernel/src/stage.rs`](../../../kernel/src/stage.rs) validates the lowered
view again before forming addresses. In particular, indexed gather and scatter
normalization requires the indexed payload extent to be nonzero and within the
signed index domain. The language shape validator intentionally permits zero
extents, and `Shape::gather_result` does not reject a zero source extent. A
gather or scatter that replaces a zero-length source axis with nonempty indices
can therefore pass language shape validation and primitive program assembly,
then fail at native stage lowering with an invalid stage contract. This is a
real downstream boundary, not a reason to make `Shape::new` reject empty
metadata.

Contractions with a zero contracted extent likewise remain representable in the
language shape algebra. The primitive lowerer can construct a nonempty-output
contraction stage, while native coordinate generation assumes every contracted
extent is nonzero. Its coordinate helper divides by each extent, so this path
can reach a divide-by-zero panic instead of a typed lowering error. Such a graph
must be treated as a lowering failure at that boundary rather than as a valid
native launch. Other empty paths are explicitly short-circuited as shown above.

## Identity and planning consequences

Tensor metadata participates in identity, not just validation. The primitive
program digest includes every lowered logical extent, offset, stride, and
storage byte bound. The planner graph digest includes each tensor's ID, dtype,
storage bound, external flags, shape extents, layout offset, and layout
strides. A shape or affine-layout change therefore selects a different lowered
program and candidate identity even when the primitive kind and scalar formula
are unchanged.

The planner still owns placement, transfers, arena offsets, and measured
hardware choices. Those later values do not rewrite `Tensor.shape` or
`Tensor.layout`; they bind the already validated semantic view to a physical
allocation. This separation is what lets graph validation, OGDL round trips,
primitive lowering, and native artifact identity agree on one tensor contract.
