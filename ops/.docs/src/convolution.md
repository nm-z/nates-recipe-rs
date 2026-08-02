# `ops/src/convolution.rs`: prepared channelwise 1-D convolution metadata

## Module identity

```text
crate: recipe-ops
source: ops/src/convolution.rs
public exports: ChannelwiseConvolution1dPreparation,
                prepare_channelwise_convolution_1d
logical operation: valid, stride-one, channelwise 1-D convolution
calculation dtype at consumers: F32 payloads, I32 index tables
module output: immutable host-owned coordinates, shapes, and byte counts
```

This module prepares the deterministic tables needed by Recipe's structured
convolution path. It does not multiply f32 values, build a calculation graph,
select a device, allocate device memory, launch a kernel, or serialize a
model. The training and inference compilers own those boundaries. The module
is deliberately smaller than a convolution implementation: it resolves one
concrete geometry, proves that its flat coordinates are representable, and
hands those coordinates to graph builders.

`ops/src/lib.rs` re-exports both public items. `recipe::engine::ops` exposes
the complete crate for advanced callers, but the normal end-to-end consumers
are `training/src/compile.rs` and `training/src/inference.rs`. The operation
registry and composition materializer are a separate compatibility boundary,
described below, and do not call this preparation function.

The paired implementation is [`ops/src/convolution.rs`](../../src/convolution.rs)
and its facade is [`ops/src/lib.rs`](../../src/lib.rs). The consumer links in
this document point at the same checkout, not at generated API text.

The normative model contract is [system-contract C30](../../../system-contract.md#c30-channelwise-one-dimensional-convolution-and-pooling):
`.conv(filters, kernel)` is valid, stride-one, has no implicit padding, uses
length-major and channel-minor logical order, and emits the complete channel
dimension of the preceding block. This file documents the implementation of
the checked coordinate part of that contract.

## Concrete geometry

Use these names for one call:

```text
B = batch rows prepared together
L = input_length
C = input_channels
F = filters, the output channel count
K = kernel_size
O = output_length = L - K + 1

I = B * L * C                       input elements
Y = B * O * F                       output elements
W = B * O * K * C                   receptive-field table elements
G = B * L * K * F                   input-gradient contribution elements
```

The logical tensors are:

| Value | Logical shape | Flat representation used by the callers |
| --- | --- | --- |
| Input | `[B, L, C]` | `[I]` while the checked gather runs |
| Weight | `[K, C, F]` | The training and inference graph keeps this rank-three tensor |
| Bias | `[F]` | One f32 bias per output filter |
| Output | `[B, O, F]` | Returned to the model as `[B, O * F]` |

This is channelwise in the model's sense, not depthwise: every output filter
contracts every input channel at every kernel position. The forward equation
for output `(b, o, f)` is therefore

```text
y[b, o, f] = bias[f]
              + sum(k = 0 .. K-1, c = 0 .. C-1)
                    x[b, o + k, c] * weight[k, c, f]
```

There is no padding, dilation, or stride parameter in this preparation. The
only accepted output length is `O = L - K + 1`, and `K == L` is valid with one
output position.

## `ChannelwiseConvolution1dPreparation`

The struct is `Clone`, `Debug`, `PartialEq`, and `Eq`. Its fields are private;
callers observe the complete contract through the following getters.

| Getter | Meaning |
| --- | --- |
| `batch()` | `B`, the row count represented by every prepared table |
| `input_length()` | `L`, the logical length before the block |
| `input_channels()` | `C`, the complete preceding channel count |
| `filters()` | `F`, the logical output channel count |
| `kernel_size()` | `K` |
| `output_length()` | `O = L - K + 1` |
| `input_elements()` | Checked `I = B * L * C` |
| `output_elements()` | Checked `Y = B * O * F` |
| `window_indices()` | Absolute flat input coordinates in `[B, O, K, C]` order |
| `input_gradient_indices()` | Absolute flat output-gradient coordinates in `[B, L, K, F]` order; invalid boundary entries are zero |
| `input_gradient_validity()` | f32 zero and one bit patterns matching `input_gradient_indices()` |
| `input_shape()` | `[I]`, the flat input shape expected by the checked gather boundary |
| `window_indices_shape()` | `[B, O, K, C]` |
| `output_shape()` | `[B, O, F]` |
| `input_gradient_indices_shape()` | `[B, L, K, F]` |
| `forward_workspace()` | `ByteCount` for the receptive-field image, `4 * W` bytes |
| `backward_workspace()` | `ByteCount` for the receptive-field and gradient-contribution images, `4 * (W + G)` bytes |

The shape getters return logical tensor extents, not byte lengths. The two
index getters return borrowed slices, so the preparation remains the owner of
the immutable tables. The f32 validity table is stored as `u32` bits so the
same bytes can be registered as an external `DType::F32` tensor without a
host-side conversion loop.

## Preparation algorithm

`prepare_channelwise_convolution_1d(B, L, C, F, K)` executes these checks and
construction steps in order.

1. It checks each of `B`, `L`, `C`, `F`, and `K` for zero. A zero value returns
   `OperationErrorKind::UnsupportedConcreteShape` with the detail
   `"<name> must be positive"`, where `<name>` is the argument label.
2. It rejects `K > L` with detail
   `"kernel_size <K> exceeds input_length <L>"`.
3. It computes `O = L - K + 1`. The preceding check makes this subtraction
   safe and guarantees a positive output length.
4. It computes `I` and `Y` using `checked_product`. A u64 product overflow
   returns `UnsupportedConcreteShape` with the role-specific detail
   `"convolution input elements overflowed u64"` or
   `"convolution output elements overflowed u64"`.
5. It requires both `I` and `Y` to be at most `i32::MAX`. This is the checked
   linear-index domain used by Recipe's gather and contraction primitives. A
   violation returns the single detail
   `"convolution input and output must fit the checked int32 index domain"`.
6. It computes `W` and `G` with the same checked-product helper. These products
   are the exact lengths of the two host tables and participate in workspace
   accounting.
7. It converts `W` and `G` to `usize` for `Vec::with_capacity`. A conversion
   failure returns `UnsupportedConcreteShape` with either
   `"convolution receptive-field table does not fit host usize: ..."` or
   `"convolution input-gradient table does not fit host usize: ..."`.
8. It fills the forward window table and the backward index and validity
   tables in deterministic nested-loop order.
9. It computes the forward and backward `ByteCount` values with checked
   multiplication and addition, then returns the immutable struct.

The function returns `OperationResult<ChannelwiseConvolution1dPreparation>`.
Every error created in this module has `operation: None`; the preparation does
not know an operation-registry ordinal. A caller that has an operation context
wraps the error in its own compile error type.

### Forward receptive-field coordinates

The loops are ordered `batch_index`, `output_index`, `kernel_index`, then
`channel`. For a tuple `(b, o, k, c)`, the stored coordinate is

```text
window_indices[b, o, k, c]
    = ((b * L + o + k) * C) + c
```

This is the absolute coordinate in the flat row-major input `[B, L, C]`.
`o + k` is always less than `L` because `o < O` and `O + K - 1 = L`. The
consumer therefore uses a checked `Gather(axis = 0, bounds = Reject)` and
does not need a padding branch or a runtime window calculation.

The table has exactly `W` entries and the corresponding shape is
`[B, O, K, C]`. In the forward graph, gathered f32 columns retain this shape;
the contraction then sums axes `(2, 0)` against the weight's kernel axis and
`(3, 1)` against its input-channel axis.

### Backward output-gradient coordinates and validity

The backward loops are ordered `batch_index`, `input_index`, `kernel_index`,
then `filter`. For each `(b, i, k, f)`, the candidate output position is

```text
q = i - k
```

computed with `checked_sub`. The contribution is valid exactly when `q` exists
and `q < O`. For a valid contribution the stored output-gradient coordinate is

```text
input_gradient_indices[b, i, k, f]
    = ((b * O + q) * F) + f
input_gradient_validity[b, i, k, f] = 1.0_f32.to_bits()
```

For an invalid left or right boundary contribution, the index is `0` and the
validity bits are `0.0_f32.to_bits()`. Index zero is intentional: it keeps the
subsequent gather in range. The matching validity image then maps that gathered
value to exact f32 zero before the input-gradient contraction. No invalid
contribution is omitted from the rectangular table, and no atomic scatter is
needed to sum overlapping windows.

The backward table has exactly `G` entries and shape `[B, L, K, F]`. For each
input coordinate, the later contraction sums kernel/filter contributions into
the input-channel axis. The deterministic table plus validity mask makes the
accumulation order a fixed contraction order rather than an unordered atomic
add.

### Workspace accounting

`recipe_core::ByteCount` is an exact u64 byte wrapper. The helper
`workspace_bytes(elements, role)` computes `elements * 4` with checked
arithmetic and returns `UnsupportedConcreteShape` with `"<role> overflowed
u64"` on overflow.

```text
forward_workspace  = 4 * W
backward_workspace = 4 * (W + G)
```

The addition `W + G` is checked before the byte multiplication. These values
describe the f32 images represented by the prepared tables. In the current
training and inference compilers the getters are metadata only: those callers
allocate graph tensors through their own compiler and do not pass these counts
to `recipe_ops::materialize_composition` as a workspace limit. The counts are
still part of the preparation contract and make the host-side table cost
observable without re-deriving the formulas.

## Bounds and representation invariants

The implementation relies on several facts that are worth keeping explicit.

* All five input dimensions are positive. `O` is therefore positive after the
  `K <= L` check.
* `I` and `Y` are checked products and fit `i32::MAX`. Every value written to
  `window_indices` is an input coordinate below `I`, and every valid value in
  `input_gradient_indices` is an output coordinate below `Y`. The casts to
  `i32` are consequently safe.
* `W` and `G` are checked for u64 overflow and for host `usize` capacity. The
  preparation source does not impose a separate `i32::MAX` limit on table
  lengths, only on the flat input and output domains. Downstream `Shape` and
  tensor constructors may impose their own storage limits.
* `input_gradient_indices` and `input_gradient_validity` are built together
  and always have equal length. The public shape getter describes both images.
* The validity bits are only `0.0` or `1.0`, both finite f32 values. This is
  required by the training compiler's external f32-input boundary.
* All arithmetic that can affect a count, coordinate, or workspace amount is
  checked. The nested-loop arithmetic is safe because its maxima are bounded
  by the checked products above.
* `Vec::with_capacity` can still fail through the allocator, for example on an
  out-of-memory condition. That is a process-level allocation failure, not an
  `OperationError` path implemented by this module.

## Public declaration to preparation

The user-facing route begins in `src/api.rs`:

1. `Model::conv(filters, kernel)` appends a `LayerSpec::Convolution` with a
   linear activation. The declaration validator requires both values to be
   nonzero and records a deferred declaration error instead of silently
   creating an invalid layer.
2. A later activation method can replace the convolution's activation. Unlike
   dense and residual operation lists, convolution declarations hold one
   activation field. `.norm(...)` is rejected after a convolution, so the
   preparation path does not receive a normalization operation from that API.
3. `src/training.rs::map_dense_block` converts the declaration to
   `DenseConvolution`, preserving nonzero `filters`, `kernel`, and the selected
   activation. `DenseConvolution::new` stores linear as an empty operation list
   and any other activation as one ordered `DenseOperation::Activation`.
4. `training/src/compile.rs::LogicalFeatureShape::convolved` resolves the
   preceding logical length and channel count. It rejects a kernel wider than
   the current logical length, computes `O`, stores
   `DenseConvolutionGeometry`, and changes the next logical shape to
   `[O, F]`. Repeated convolution blocks consume the complete preceding
   channel count.

The same geometry rule is enforced when a checkpoint is decoded. The
checkpoint validator verifies declaration filters and kernel, input length and
channels, `K <= L`, `output_length = L - K + 1`, weight shape `[K, C, F]`, bias
shape `[F]`, and the ordered optional PReLU parameter list before the compiler
uses this module.

## Training compiler integration

`training/src/compile.rs` is the main producer of a training graph. The
following path is the exact use of this preparation.

### Forward graph

`compile_training_convolution` first masks invalid partition rows to zero,
initializes a f32 weight `[K, C, F]` and zero bias `[F]`, and calls
`compile_convolution_forward`. That helper:

1. Requires the input to be f32 `[B, L * C]`, the weight to be f32 `[K, C, F]`,
   and the bias to be f32 `[F]`.
2. Calls `prepare_channelwise_convolution_1d(B, L, C, F, K)`.
3. Packs the matrix input to flat f32 `[I]` using a unique-index scatter.
4. Registers `window_indices()` as an immutable external I32 tensor with shape
   `window_indices_shape()`. Training uses
   `ExternalInputRole::TrainingConvolutionWindowIndices { block }`.
5. Gathers the flat input with those coordinates to f32 columns
   `[B, O, K, C]`.
6. Emits a `PrimitiveKind::Contraction` with contract axes `(2, 0)` and
   `(3, 1)`, producing `[B, O, F]`.
7. Emits an f32 elementwise add of the `[F]` bias, preserving `[B, O, F]`.
8. Unpacks the grouped output to matrix `[B, O * F]` using a unique-index
   scatter. The helper returns `output_group_indices`, an identity map used to
   regroup output gradients during backward compilation.

The resulting `ConvolutionValues` retains the geometry, this preparation,
columns, output grouping map, initialized parameters, and any declared
operation values. `compile_training_convolution` then applies each activation
or other operation in declaration order. The forward graph therefore owns
the model's actual calculation, while this module owns only static indexing and
shape facts.

### Backward graph

`backward_convolution` is called when the reverse block walk reaches a
convolution. It first walks post-convolution operations in reverse order and
then computes the convolution parameter gradients:

* The output gradient is checked as f32 `[B, O * F]` and masked to zero for
  invalid partition rows.
* `output_group_indices` gathers it back to `[B, O, F]`.
* A contraction of saved `columns` and grouped output gradient, with contract
  axes `(0, 0)` and `(1, 1)`, produces the weight gradient `[K, C, F]`.
* A fixed reduction over axes `[0, 1]` produces the bias gradient `[F]`.
* Learned PReLU gradients from the reversed operation walk are returned in
  ordered occurrence order with the weight and bias gradients.

When the surrounding block needs an input gradient, the helper additionally:

1. Packs the output gradient to flat f32 `[Y]`.
2. Registers `input_gradient_indices()` as an external I32 tensor with shape
   `input_gradient_indices_shape()` under
   `ExternalInputRole::TrainingConvolutionInputGradientIndices { block }`.
3. Registers `input_gradient_validity()` as an external F32 tensor with the
   same shape under
   `ExternalInputRole::TrainingConvolutionInputGradientValidity { block }`.
4. Gathers flat output gradients by the prepared coordinates. Invalid boundary
   entries read index zero, which is in range.
5. Applies a masked-zero f32 scalar program using the validity image.
6. Contracts those contributions with the learned weight using axes `(2, 0)`
   and `(3, 2)`, producing grouped input gradients `[B, L, C]`.
7. Unpacks to matrix `[B, L * C]` and masks invalid partition rows again.

This is the C30 deterministic backward route. It deliberately does not use an
atomic scatter for overlapping receptive fields. The prepared rectangular
contribution image and the fixed contraction order account for every valid
`(b, i, k, f)` contribution.

After the optimizer update, `update_convolution_state` stores the geometry,
updated weight and bias `ParameterState`s, and ordered PReLU states in
`DenseConvolutionState`. The preparation tables are compile-time graph inputs,
not learned or checkpointed model state.

### Validation

`compile_validation_convolution` reuses `compile_convolution_forward` with the
saved geometry and updated weight and bias parameters, but registers a
`ValidationConvolutionWindowIndices { block }` table. It applies the saved
ordered operations with saved validation PReLU state. Validation has no
backward table because it does not differentiate the model. Geometry mismatch
between the training state and the declaration fails before this helper is
called.

## Target-free inference integration

`training/src/inference.rs::InferenceGraphCompiler::compile_convolution` is a
second direct consumer. It receives a validated
`CheckpointConvolutionImage` and the preceding logical shape.

It first requires the checkpoint geometry's input length, input channels, and
flattened input width to equal the current logical shape. It checks the saved
weight bytes as f32 `[K, C, F]` and bias bytes as f32 `[F]`, calls this
preparation with the query row count as `B`, and verifies that the prepared
`output_length()` equals the checkpoint's saved output length. A mismatch is an
`InferenceCompileErrorKind::InconsistentCheckpoint` rather than an implicit
geometry correction.

The graph then follows the same real calculation as training forward: pack
matrix features to flat, register an immutable
`InferenceInputRole::ConvolutionWindowIndices { block }` I32 table, gather
`[B, O, K, C]`, contract axes `(2, 0)` and `(3, 1)` with the saved weight, add
the saved bias, and unpack `[B, O, F]` to `[B, O * F]`. It applies each saved
operation in order. A PReLU operation consumes one saved scalar in occurrence
order and rejects an omitted or extra scalar; other operations use their saved
configuration and the inference reduction settings.

The inference compiler recomputes the window table for the actual query row
count. The semantic checkpoint stores declaration, resolved geometry,
parameters, and optimizer state, not a row-count-specific preparation table.
Semantic model version 9 is the first contract that records convolution blocks;
the checkpoint validator still decodes older convolution-free versions under
their prior topology rules.

## Operation-surface and materializer boundary

The repository also contains legacy source-qualified convolution descriptors:

| Descriptor | Composition recipe in `ops/src/composition.rs` |
| --- | --- |
| `gpu_conv1d_into` | `convolution_1d`: Gather, Contraction, optional bias or activation |
| `gpu_conv1d_backward_data_into` | `convolution_1d_backward`: Gather, Contraction, Scatter |
| `gpu_conv1d_backward_filter_into` | Same backward recipe |
| `gpu_conv1d_backward_bias_into` | Same backward recipe |

Their inventory rows are in [`operation-surface.txt`](../../../operation-surface.txt)
at the `gpu_conv1d_*` entries and are classified as
`OperationFamily::Convolution`. The recipes are descriptive operation-surface
metadata. They are not calls to `prepare_channelwise_convolution_1d`.

In particular, [`ops/src/materialize/convolution_pooling.rs`](../../src/materialize/convolution_pooling.rs)
owns exact `(symbol, source)` pairs for image-to-column, column-to-image,
pooling, upsampling, and the Recipe channelwise max-pool pair. Its `OPERATIONS`
table does not contain any `gpu_conv1d_*` descriptor. Consequently,
`materialize_composition` rejects a direct request for those descriptors with
`OperationErrorKind::MissingConcreteFormula` and the message that tensor ABI,
scalar SSA, primitive parameters, and workspace policy are not concrete. The
training and inference convolution paths bypass that unresolved compatibility
request and emit their own graph primitives directly, using this preparation
module for the checked tables.

This distinction matters when tracing a failure: a missing concrete formula is
not a coordinate-preparation failure, and adding a branch to this module would
not make the legacy composition request concrete. The current concrete
materializer does implement `gpu_im2col_1d` and the `gpu_col2im_*` family, but
those flattened image operations have different ABIs and are not used by the
model compiler's `DenseConvolution` path.

## Failure propagation

### Errors raised here

All preparation failures use `OperationErrorKind::UnsupportedConcreteShape`:

| Condition | Detail shape |
| --- | --- |
| Any dimension is zero | `<name> must be positive` |
| `K > L` | `kernel_size <K> exceeds input_length <L>` |
| A checked product overflows u64 | `<role> overflowed u64` |
| `I` or `Y` exceeds `i32::MAX` | `convolution input and output must fit the checked int32 index domain` |
| `W` cannot become host `usize` | `convolution receptive-field table does not fit host usize: ...` |
| `G` cannot become host `usize` | `convolution input-gradient table does not fit host usize: ...` |
| `W + G` overflows | `convolution backward workspace elements overflowed u64` |
| `4 * W` overflows | `convolution forward workspace overflowed u64` |
| `4 * (W + G)` overflows | `convolution backward workspace overflowed u64` |

The first matching check determines the returned detail. There is no fallback
shape, clamping, padding, partial table, or alternate index type. Allocation
failure from `Vec::with_capacity` remains a normal process allocation failure.

### Training and inference wrappers

`TrainingCompileError` converts an `OperationError` to
`TrainingCompileErrorKind::Operation` and preserves the operation error's
display text. Other graph construction failures remain `InvalidNetwork`,
`Language`, or `Program` errors according to the layer that observes them.

`InferenceCompileError` converts an `OperationError` to
`InferenceCompileErrorKind::Operation`. Before that conversion,
`compile_convolution` can report checkpoint-specific failures such as
`InconsistentCheckpoint` for logical geometry, parameter shape, output-length,
or PReLU-state disagreement. It can also report `ArithmeticOverflow` when a
saved flattened width cannot be computed. These checks are adjacent caller
contracts, not alternate behavior of the preparation function.

The graph primitives use `IndexBounds::Reject`. Thus a corrupted external
index table is not silently clamped at execution time. In the normal path the
preparation formulas make every forward and valid backward coordinate safe;
the invalid backward coordinates are deliberately zero plus a zero validity
bit.

## End-to-end role and non-responsibilities

For a public `.conv(filters, kernel)` model, the complete relevant flow is:

```text
Model::conv / activation
  -> LayerSpec::Convolution
  -> DenseConvolution and LogicalFeatureShape::convolved
  -> DenseConvolutionGeometry [L, C, O, F, K]
  -> prepare_channelwise_convolution_1d(B, L, C, F, K)
  -> immutable external index tensors
  -> pack -> checked gather -> canonical contraction -> bias
  -> unpack to the model's matrix width O * F
  -> declared activation
  -> training backward and optimizer state, or checkpoint inference
```

The module owns exactly the boxed preparation step in that flow. It does not
own public declaration validation, model geometry resolution, weight
initialization, activation semantics, checkpoint encoding, optimizer updates,
graph identity allocation, execution scheduling, or device runtime behavior.
Those layers consume its checked facts and must preserve the same C30 geometry.
