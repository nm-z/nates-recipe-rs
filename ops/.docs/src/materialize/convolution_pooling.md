# Convolution and pooling materialization

This page describes the current, source-qualified implementation boundary for
the convolution and pooling family. The important distinction is between the
legacy operation-surface materializers in
ops/src/materialize/convolution_pooling.rs, the immutable one-dimensional
preparation tables in ops/src/convolution.rs and ops/src/pooling.rs, and the
public dense-model compiler in training/src/compile.rs and
training/src/inference.rs. They are related, but they are not one shared
implementation:

* The family materializer owns the concrete im2col, col2im, pooling,
  gradient-expansion, and nearest-neighbor descriptors listed below.
* The public .conv(filters, kernel) model path uses the paired convolution
  preparation type and emits its own gather, contraction, bias, and gradient
  nodes in the training compiler. There is no gpu_conv1d_into materializer in
  this module.
* The public .pool(size) model path uses ChannelwiseMaxPool1dPreparation and
  crosses materialize_composition for both Recipe-owned pooling descriptors.

Keeping those paths explicit prevents a description of a hypothetical
convolution materializer from being mistaken for the graph that actually runs.

## Source-qualified ownership

OPERATIONS in ops/src/materialize/convolution_pooling.rs is an exact list of
(symbol, source) pairs. supports compares both fields, so a same-named
operation from another legacy source is not silently routed here. The dispatch
function first checks this list and then selects one emitter by symbol.

| Symbol | Source-qualified owner | Emitter | Composition recipe |
| --- | --- | --- | --- |
| gpu_avg_pool_1d | gpu-core/src/kernels.rs:4596 | emit_average_pool_1d | average_pool |
| gpu_avg_pool_2d | gpu-core/src/kernels.rs:5962 | emit_average_pool_2d | average_pool |
| gpu_avg_pool_2d_f32 | gpu-core/src/nn_f32.rs:391 | emit_average_pool_2d | average_pool |
| gpu_avg_pool_2d_backward | gpu-core/src/kernels.rs:5999 | emit_average_pool_2d_backward | average_pool_backward |
| gpu_avg_pool_2d_backward_f32 | gpu-core/src/nn_f32.rs:427 | emit_average_pool_2d_backward | average_pool_backward |
| gpu_col2im_1d | gpu-core/src/kernels.rs:5847 | emit_col2im_1d | column_to_image |
| gpu_col2im_2d | gpu-core/src/kernels.rs:5877 | emit_col2im_2d(..., false) | column_to_image |
| gpu_col2im_2d_ext | gpu-core/src/attention.rs:384 | emit_col2im_2d(..., true) | column_to_image |
| gpu_im2col_1d | gpu-core/src/kernels.rs:5097 | emit_checked_gather_identity | image_to_column |
| gpu_im2col_2d | gpu-core/src/kernels.rs:5417 | emit_im2col_2d(..., false) | image_to_column |
| gpu_im2col_2d_ext | gpu-core/src/attention.rs:341 | emit_im2col_2d(..., true) | image_to_column |
| gpu_max_pool_1d | gpu-core/src/kernels.rs:5911 | emit_legacy_max_pool_1d | maximum_pool |
| gpu_max_pool_1d_backward | gpu-core/src/kernels.rs:5936 | emit_legacy_max_pool_1d_backward | maximum_pool_backward |
| gpu_max_pool_2d | gpu-core/src/kernels.rs:6037 | emit_max_pool_2d | maximum_pool |
| gpu_max_pool_2d_f32 | gpu-core/src/nn_f32.rs:463 | emit_max_pool_2d | maximum_pool |
| gpu_max_pool_2d_backward | gpu-core/src/kernels.rs:6076 | emit_max_pool_2d_backward | maximum_pool_backward |
| gpu_max_pool_2d_backward_f32 | gpu-core/src/nn_f32.rs:501 | emit_max_pool_2d_backward | maximum_pool_backward |
| gpu_pool_grad_expand | gpu-core/src/kernels.rs:4622 | emit_pool_gradient_expand | pool_gradient_expand |
| gpu_upsample_nearest_2d | gpu-core/src/kernels.rs:6620 | emit_upsample_nearest_2d | nearest_neighbor_upsample |
| recipe_max_pool_1d | ops/src/pooling.rs:channelwise_max_pool_1d | emit_channelwise_max_pool_1d | channelwise_maximum_pool_1d |
| recipe_max_pool_1d_backward | ops/src/pooling.rs:channelwise_max_pool_1d_backward | emit_channelwise_max_pool_1d_backward | channelwise_maximum_pool_1d_backward |

The four gpu_conv1d* entries are different. ops/src/composition.rs gives
them convolution_1d or convolution_1d_backward recipes, but they do not
appear in this module's OPERATIONS list. Consequently has_concrete_materializer
is false for those source-qualified descriptors and
remaining_composition_manifest() reports them as still lacking a concrete
tensor ABI, scalar formula, primitive wiring, and workspace policy. The dense
model compiler does not call those descriptors; it lowers the public
one-dimensional convolution directly as described later.

The two Recipe-owned descriptors are appended after the immutable
operation-surface.txt prefix in ops/src/registry.rs. Their source strings
are the preparation function names, and the public accessors are
channelwise_max_pool_1d_descriptor() and
channelwise_max_pool_1d_backward_descriptor().

## Materialization boundary

The public facade calls recipe_ops::materialize_composition with a
MaterializationRequest. A request contains:

* one OperationDescriptor, including its source-qualified OperationId;
* immutable named input and output Tensor declarations;
* the name of the input whose shape resolves composition iteration bounds;
* a BTreeMap<String, PreparedParameter> containing only U64, I32,
  F32Bits, or Bool values;
* a caller-reserved IdentityNamespace for intermediate values and kernel
  templates; and
* a checked ByteCount workspace limit.

validate_request is fail-closed before family dispatch. It requires a
structured LoweringAvailability::Composition, at least one input and output,
unique nonempty names, unique tensor IDs, valid tensor declarations, external
input flags on every input, and non-input external-output flags on every
output. The family emitter then calls require_exact_abi, which compares the
input-name, output-name, and prepared-parameter sets as BTreeSets. Missing or
extra names are an InvalidMaterializationRequest, not an ignored argument.

materialize_composition resolves the named iteration-shape input, validates
the static CompositionRecipe, expands every repeat before graph construction,
creates an Emitter, dispatches to the exact family owner, and requires the
emitter to finish every resolved step. The family dispatcher is ordered after
the other materializer modules, but an owned convolution/pooling descriptor
returns immediately with its result. A descriptor outside the exact list is
NotOwned; if no family owns it, materialization fails with
MissingConcreteFormula rather than substituting a nearby operation.

For this family the recipes are fixed, three-stage chains:

~~~
forward pooling:  Gather -> Reduce -> Elementwise
backward pooling: Gather -> Elementwise -> Scatter
im2col:           Gather -> Elementwise
col2im:           Gather -> Elementwise -> Scatter
upsample:         Gather -> Elementwise
~~~

POOL_STEPS describes the gather, fixed-order window reduction, and final
average-divisor or maximum-tie map. POOL_BACKWARD_STEPS describes checked
source-window reconstruction, contribution mapping, and an explicit scatter
conflict policy. A materializer must emit one kernel for each resolved step.
Emitter::emit_stage checks the primitive family against the resolved recipe,
records the concrete kernel IDs in StageEmission, and rejects extra or empty
stages. The current convolution/pooling emitters use one stage per call, so
each stage has one kernel template ID.

### Graph and workspace state

GraphBuilder starts its value and kernel cursors at the caller's half-open
identity ranges. Declared tensor IDs may not fall inside the intermediate value
range. Every intermediate(dtype, shape) allocates a contiguous tensor,
accounts its storage_bytes against the request limit, and records a
WorkspaceObject; it does not reuse an earlier image. Every emitted primitive
gets a new KernelTemplateId, a PrimitiveKernel, and a forbidden alias rule
for every input/output pair. The resulting CalculationGraph is validated
before MaterializedComposition is returned.

The result contains the operation ID, graph, resolved bounds and steps,
stage-to-kernel mapping, complete workspace allocation, and the identity
namespace. validate_identity_namespaces separately rejects overlapping value
or kernel reservations when fragments are assembled. Exhausting either range
is IdentityNamespaceExhausted; exceeding the byte limit is
WorkspaceLimitExceeded; checked byte or element arithmetic can produce
WorkspaceArithmeticOverflow.

The training and inference callers reserve 64 values and 64 kernels for each
materialization call and pass an effectively unlimited workspace limit. They
copy the returned graph's tensor contracts and nodes into their larger static
graph and assign each imported node the caller's iteration domain. They do not
execute a materializer callback at runtime, and the callers do not use the
returned StageEmission or WorkspaceAllocation as a second execution path.

## Shared geometry and scalar helpers

The family module uses one PoolGeometry for legacy one-dimensional pooling,
two-dimensional pooling, and two-dimensional receptive-field maps:

~~~
input_elements   flat source payload size
output_elements  number of output positions or pooled values
window_elements  values in one logical window
input_width      source spatial width, used by 2-D winner decoding
kernel_width     kernel width, used by 2-D winner decoding
~~~

ChannelwisePool1dGeometry additionally retains batch, channels, groups,
window_width, and input_elements for the Recipe-owned
[batch, group, channel, window] layout.

prepared_extent reads a U64 parameter and requires 1..=2_147_483_647.
checked_product detects u64 multiplication overflow and then requires the
linearized product to fit the same legacy int32 index domain. These checks are
applied to payload, patch, window, contribution, and output counts. The
generic exact_f32_extent helper also rejects values above 16_777_216, the
largest integer guaranteed lossless in f32, before a count becomes a divisor.

pool_axis constructs axis one, which is the window axis for rank-two
[output, window] tables. channelwise_pool_axis constructs axis three for
rank-four [batch, group, channel, window] tables. prepared_tree_lanes
requires a power of two in 1..=1024; the lanes become the fixed reduction
tree in the emitted Reduce primitive.

The scalar programs are deliberately small and typed:

* divide_by_program builds one f32 input divided by an embedded exact f32
  divisor.
* typed_pair_identity_program passes an f32 value and an i32 index through a
  two-output elementwise node. It makes the index result explicit instead of
  treating a multi-output reduction as an implicit side effect.
* channelwise_pool_result_program computes
  global = winner_base + local_winner * channels and returns the maximum and
  global index.
* max_pool_2d_result_program decodes a local flattened kernel coordinate as
  row = local / kernel_width, column = local % kernel_width, then returns
  window_base + row * input_width + column with the maximum value.
* max_pool_backward_program computes
  destination = base + winner * index_stride and returns the incoming
  gradient with that destination index.

All scalar-program construction errors are converted to
GraphMaterializationFailed. Runtime index bounds remain IndexBounds::Reject;
a preparation fact proves an index table's formula but does not turn a malformed
device index into an unchecked read or write.

## Receptive-field maps

### gpu_im2col_1d

The shared emit_checked_gather_identity helper requires inputs image: f32 and
receptive_field_indices: i32, output columns: f32, and prepared parameters
axis: U64 and indices_encode_receptive_fields: Bool(true). The output shape
must equal the checked gather result for the prepared axis and the supplied
index shape. The graph is one axis-aware gather followed by an explicit f32
identity map. Its workspace is four bytes per gathered output element.

### gpu_im2col_2d and gpu_im2col_2d_ext

Both variants require inputs image: f32 and
receptive_field_indices: i32, output columns: f32, and a true
indices_encode_receptive_fields fact. The basic variant's prepared dimensions
are batch, channels, input_height, input_width, kernel_height, and kernel_width.
The extended variant adds positive stride_height, stride_width,
dilation_height, and dilation_width, plus U64 padding_height and padding_width.

convolution_2d_geometry computes:

~~~
effective_h = (kernel_height - 1) * dilation_height + 1
effective_w = (kernel_width  - 1) * dilation_width  + 1
padded_h    = input_height + 2 * padding_height
padded_w    = input_width  + 2 * padding_width
output_h    = (padded_h - effective_h) / stride_height + 1
output_w    = (padded_w - effective_w) / stride_width + 1
patches     = batch * output_h * output_w
window      = channels * kernel_height * kernel_width
~~~

The source image shape is [batch * channels * input_height * input_width];
both the index table and lowered columns have shape [patches * window].
gpu_im2col_2d_ext rejects nonzero padding before geometry is emitted because a
gather-only graph cannot synthesize the source kernel's zero-filled padded
lanes. Zero padding, stride, and dilation are supported. The graph is a flat
axis-zero gather followed by identity, with four workspace bytes per patch
element. Invalid extents, effective kernels that do not fit the padded input,
u64 arithmetic overflow, or an int32-incompatible product fail before graph
emission.

### gpu_col2im_1d

Inputs are patches: f32, patch_indices: i32,
destination_indices: i32, and image_base: f32; output is image: f32.
Parameters are positive batch, input_length, and kernel_size, plus true
patch_indices_verified, destination_indices_verified, and image_base_zero. The
kernel must fit the input. The output length is input_length - kernel_size + 1,
the image has batch * input_length elements, and all three contribution tables
have batch * output_length * kernel_size elements. A checked gather reads the
patches, identity preserves the f32 contribution image, and an axis-zero
relaxed atomic-add scatter adds into the supplied image base. Workspace is
eight bytes per contribution.

### gpu_col2im_2d and gpu_col2im_2d_ext

The basic and extended inverse maps use the same four tensor inputs and one
f32 image output. The basic parameter set is the six positive dimensions
batch, channels, input_height, input_width, kernel_height, and kernel_width,
plus true patch_indices_verified, destination_indices_verified, and
image_base_zero. The extended set adds stride, padding, dilation, and positive
valid_contributions.

The basic contribution count is output_elements * window_elements, where
output_elements and window_elements come from
convolution_2d_geometry(..., false). The extended graph retains the complete
patches shape but uses valid_contributions for both index-table shapes. It
rejects a valid count larger than the complete patch tensor, while allowing
nonzero padding because preparation simply omits padded coordinates from the
valid tables. The common gather, identity map, and relaxed atomic-add scatter
therefore use eight bytes per valid contribution. image_base must have the
same dtype and shape as image; the zero-base fact is the explicit source
contract rather than an implicit allocation.

## Average pooling and gradient expansion

### Forward average pooling

gpu_avg_pool_1d requires values: f32, window_indices: i32, and output
pooled: f32. Its exact parameters are batch, window_length, filters,
tree_lanes, and true window_indices_verified. Values have
batch * window_length * filters elements, pooled has batch * filters, and
the index table has shape [batch * filters, window_length].

gpu_avg_pool_2d and gpu_avg_pool_2d_f32 share the same tensor ABI. Their
parameters are batch, channels, input and kernel heights and widths, stride
height and width, tree_lanes, and the verification fact. pooling_2d_geometry
requires each positive kernel extent to fit its input and uses floor division
for the output dimensions. Values are flat
batch * channels * input_height * input_width; each output window contains
kernel_height * kernel_width values, while channels are already represented
in the output position count.

Both dimensionalities emit:

1. axis-zero gather of values through window_indices into a
   [output_elements, window_elements] f32 image;
2. fixed-tree axis-one sum into [output_elements]; and
3. an elementwise division by the exact f32 window_elements count.

Workspace is
4 * output_elements * window_elements + 4 * output_elements bytes. The
index table fact is required even though gather itself still rejects invalid
indices.

### Average-pool backward and gpu_pool_grad_expand

The two-dimensional backward aliases gpu_avg_pool_2d_backward and
gpu_avg_pool_2d_backward_f32 require inputs output_gradient: f32,
gradient_indices: i32, destination_indices: i32, and input_gradient_base: f32,
output input_gradient: f32, and parameters for the same 2-D geometry plus true
gradient_indices_verified, destination_indices_verified, and
input_gradient_base_zero. gpu_pool_grad_expand has the analogous one-
dimensional ABI with batch, window_length, filters, the two index facts,
destination_indices_unique, and input_gradient_base_zero.

emit_average_backward checks all facts and dtypes, requires output-gradient
shape [output_elements], both index tables shape [contributions], and base
and result shape [input_elements]. It gathers one output gradient per
contribution, divides by the exact window count, and scatters the mapped value
to destination_indices. Ordinary 2-D backward uses relaxed atomic add for
overlapping windows. gpu_pool_grad_expand additionally requires the unique
destination fact and selects ScatterConflict::UniqueIndices, matching its
direct-write legacy contract despite the generic composition description using
the phrase "atomically accumulate". Both paths reserve eight bytes per
contribution for the gathered and mapped images.

## Maximum pooling

### Legacy one-dimensional maximum pooling

gpu_max_pool_1d has inputs values: f32 and window_indices: i32, outputs
pooled: f32 and winning_indices: i32, and parameters batch,
window_length, filters, tree_lanes, and true window_indices_verified. Its flat
shapes match one-dimensional average pool.

The graph gathers a rank-two window image, performs a fixed-tree maximum
reduction with ReduceResult::ValueAndIndex along axis one, and applies the
typed pair identity program to publish both outputs. The reduction's index is
the local time coordinate in the legacy one-dimensional window. Workspace is
4 * output_elements * window_elements + 8 * output_elements bytes.

### Two-dimensional maximum pooling

gpu_max_pool_2d and gpu_max_pool_2d_f32 require the same values, window
indices, pooled output, and winning-index output, plus an int32
window_bases input. Parameters are the 2-D pool geometry, tree_lanes, true
window_indices_verified, and true window_bases_verified. Values and windows
use the shapes from pooling_2d_geometry; pooled, winning indices, and window
bases each have shape [output_elements].

After gather and value/index reduction, the two-dimensional scalar map decodes
the local kernel index and adds it to the prepared spatial base:

~~~
row    = local_index / kernel_width
column = local_index % kernel_width
winner = window_base + row * input_width + column
~~~

This preserves the source's spatial winner index and lowest-coordinate tie
behavior. Workspace is the same forward maximum formula above.

### Legacy backward maximum pooling

gpu_max_pool_1d_backward requires inputs output_gradient: f32,
winning_indices: i32, gradient_indices: i32, destination_bases: i32, and
input_gradient_base: f32, output input_gradient: f32, and parameters batch,
window_length, filters, true gradient_indices_verified, true
destination_bases_verified, true destination_indices_unique, and true
input_gradient_base_zero. The output and base shapes are [batch * filters];
index_stride is filters, so the scalar map computes
destination_base + winning_index * filters. The final scatter is unique because
the prepared destination table proves one writer per destination.

gpu_max_pool_2d_backward and its _f32 alias require output gradient,
winning indices, gradient indices, plane_bases, and zero input-gradient base;
the parameter set is batch, channels, input and output heights and widths,
true gradient_indices_verified, true plane_bases_verified, and true
input_gradient_base_zero. All four per-output tables have shape
[batch * channels * output_height * output_width]. The map uses
plane_base + winning_index, and the final scatter uses relaxed atomic add
because overlapping windows can select the same source coordinate. The
gather, mapped gradient, and destination images reserve twelve bytes per
pooled output.

## Recipe-owned channelwise maximum pool

recipe_max_pool_1d is the concrete bridge used by the public dense model. Its
inputs are values: f32, window_indices: i32, and winner_bases: i32; outputs
are pooled: f32 and winning_indices: i32. The exact prepared set is

~~~
batch
channelwise_nonoverlap = true
channels
input_length
pool_size
tail_window_repeats_last_coordinate = true
tree_lanes
window_indices_encode_channelwise_nonoverlap = true
winner_bases_encode_channelwise_nonoverlap = true
~~~

The geometry is logical [batch, input_length, channels] to output
[batch, groups, channels], where groups = ceil(input_length / pool_size) and
window_width = min(input_length, pool_size). Values are flat
batch * input_length * channels; windows have shape
[batch, groups, channels, window_width]; winner bases, pooled, and winning
indices have shape [batch, groups, channels].

The graph gathers windows, reduces axis three with a fixed maximum value/index
tree, then maps the local winner to
winner_base + local_winner * channels. A final short window is represented by
repeating its last real coordinate, so the rectangular table remains valid and
the maximum cannot read an uninitialized padded lane. Workspace is
4 * output_elements * window_width + 8 * output_elements bytes.

recipe_max_pool_1d_backward requires output_gradient: f32,
winning_indices: i32, gradient_batch_indices: i32, and
input_gradient_base: f32, output input_gradient: f32, and the exact parameters

~~~
batch
channelwise_nonoverlap = true
channels
gradient_batch_indices_identity = true
input_gradient_base_zero = true
input_length
pool_size
winning_indices_from_matching_forward = true
winning_indices_unique = true
~~~

It checks output-gradient and winner shapes [batch, groups, channels],
identity batch indices shape [batch], and flat base/result shape
[batch * input_length * channels]. A checked axis-zero gather expands each
batch's gradient through the identity batch table. A typed pair identity map
keeps the gradient and already-global winner index together, and a unique
axis-zero scatter publishes into the zero base. Because channelwise groups do
not overlap and each output cell has exactly one winner, this path never uses
an atomic scatter. Workspace is twelve bytes per output cell.

## Nearest-neighbor upsample

gpu_upsample_nearest_2d requires values: f32, source_indices: i32, output
upsampled: f32, and positive parameters batch, channels, input_height,
input_width, scale_height, scale_width, plus true source_indices_verified.
The output dimensions are input_height * scale_height and
input_width * scale_width; values, source indices, and output have flat shapes
derived from those dimensions. The graph is a checked axis-zero gather followed
by f32 identity, with four workspace bytes per output element. The table
encodes source[input_y / scale_height, input_x / scale_width]; a malformed
runtime coordinate still fails through IndexBounds::Reject.

## Paired one-dimensional preparation modules

### ops/src/convolution.rs

ChannelwiseConvolution1dPreparation is immutable metadata for a valid,
stride-one, no-padding channelwise convolution. Its logical layout is

~~~
input   [batch, input_length, input_channels]
weight  [kernel_size, input_channels, filters]
output  [batch, output_length, filters]
output_length = input_length - kernel_size + 1
~~~

prepare_channelwise_convolution_1d requires every extent to be nonzero and
rejects a kernel wider than the input. It checks input and output element
counts against the int32 linear-index domain, checks host usize capacity for
the two generated tables, and detects all u64 workspace arithmetic overflow.

The forward window_indices table is generated in row-major
[batch, output_length, kernel_size, input_channels] order. Each entry is the
absolute flat input coordinate

~~~
(((batch_index * input_length + output_index + kernel_index)
   * input_channels) + channel)
~~~

The backward input_gradient_indices table is row-major
[batch, input_length, kernel_size, filters]. For each input position and
kernel offset, output_index = input_index - kernel_index; a contribution is
valid only when that index is in 0..output_length. Invalid entries use index
zero and the parallel input_gradient_validity image contains exact f32 zero
bits. Valid entries contain the corresponding flat output-gradient index and
exact f32 one bits. This turns boundary handling into data consumed by the
graph, rather than a conditional device scatter.

The preparation exposes shapes, counts, and forward_workspace = 4 *
window_elements and backward_workspace = 4 * (window_elements +
input_gradient_elements). Those fields are preparation metadata. The current
training and inference compilers do not ask the materializer to allocate them;
they create the corresponding graph tensors directly.

### ops/src/pooling.rs

ChannelwiseMaxPool1dPreparation is immutable metadata for a non-overlapping
maximum pool over logical [batch, input_length, channels]. It computes

~~~
groups       = ceil(input_length / pool_size)
window_width = min(input_length, pool_size)
output       [batch, groups, channels]
~~~

All four extents must be nonzero. Input elements, output elements, and window
table entries must fit the int32 index domain; host allocation conversion and
byte arithmetic are checked separately. The row-major window table uses
absolute flat coordinates ((batch * input_length + position) * channels +
channel). The final short group repeats input_length - 1 for slots after its
last real coordinate. winner_bases stores the first flat coordinate for each
output cell. The gradient_batch_indices table is the identity sequence 0..batch,
used by backward's checked gather.

forward_parameters(tree_lanes) returns exactly the eight values required by
recipe_max_pool_1d, including the non-overlap, tail-repeat, and table
verification facts. backward_parameters() returns exactly the eight values
required by recipe_max_pool_1d_backward, including zero base, matching
forward winners, and unique destinations. The preparation exposes
forward_workspace = 4 * (window_entries + 2 * output_elements) and
backward_workspace = 12 * output_elements. As with convolution preparation,
the current compiler uses these values as metadata while graph tensor creation
and materializer workspace accounting remain separate concerns.

## Public dense-model path

The facade accepts .conv(filters, kernel) and .pool(size) as LayerSpec
variants. LayerSpec::validate rejects zero extents and the facade records a
following dense layer's width as pool group-to-neuron routing. The mapping in
src/training.rs creates DenseConvolution or DensePool with nonzero typed
extents. LogicalFeatureShape begins as [length, 1]; embedding and recurrent
blocks establish channels under their own rules, while convolved and pooled
preserve the complete channel dimension.

The model contract is length-major, channel-minor and has no implicit padding:

~~~
conv:  [rows, input_length, input_channels]
      -> [rows, input_length - kernel + 1, filters]
pool:  [rows, input_length, channels]
      -> [rows, ceil(input_length / pool_size), channels]
~~~

Repeated convolution consumes all preceding channels. Pooling keeps channels,
and a following ordinary dense layer receives group-major, channel-minor
flattening. Divisible group-to-neuron widths use contiguous ranges; otherwise
the compiler selects the explicit fully connected route. A final pool is not an
implicit output layer and is rejected by the public topology contract.

### Training forward

compile_training_graph validates policy, data, and model, maps blocks, and
selects the structured-block compiler when a convolution or pool is present.
compile_training_blocks walks blocks in declaration order and updates the
logical length, channels, and matrix width after each spatial block.

For a convolution, compile_training_convolution masks invalid partition rows
to zero, initializes f32 weight [kernel, input_channels, filters] and bias
[filters], then calls compile_convolution_forward. That routine:

1. validates f32 input [rows, input_length * input_channels] and parameter
   shapes;
2. calls prepare_channelwise_convolution_1d;
3. packs the row matrix to a flat f32 image with a unique scatter;
4. admits the immutable i32 window table as an external input and gathers the
   columns;
5. contracts columns with weights over kernel and input-channel axes;
6. adds the channel bias with a typed elementwise sum; and
7. unpacks [rows, output_length, filters] back to the matrix width
   output_length * filters.

Declared convolution activation operations are then applied in order. This is
direct primitive graph construction, not a call to materialize_composition.

For a pool, compile_training_pool calls
prepare_channelwise_max_pool_1d, packs the input matrix, admits immutable
window and winner-base tables, creates pooled and winner outputs, and calls
self.materialize("recipe_max_pool_1d", ...). The compiler passes the
preparation's exact forward_parameters, reserves 64 intermediate values and
kernels, inserts the materialized graph, and unpacks pooled
[rows, groups, channels] to the dense matrix. PoolValues retains the
preparation, winners, gradient batch indices, and packing indices for backward.

Validation uses the same convolution direct path with updated parameter images
and the same pool materializer path with validation-specific external tables.
Each imported node receives the validation iteration domain.

### Training backward

The block loop in backward_blocks visits spatial blocks in reverse order.
backward_convolution reverses declared activation operations, gathers the
grouped output gradient through saved group indices, contracts it with columns
for weight gradients, and reduces over rows and output positions for the bias
gradient. If an input gradient is requested, it packs the gradient, gathers
through the prepared input_gradient_indices, multiplies by the validity image
to zero invalid boundary contributions, contracts those contributions with the
weights over filters and kernel positions, and unpacks the deterministic input
gradient. It never uses an unordered atomic scatter for convolution input
gradients.

backward_pool gathers the dense output gradient into grouped
[rows, groups, channels] form, creates a zero flat gradient base, and calls
self.materialize("recipe_max_pool_1d_backward", ...) with saved winners and
gradient_batch_indices. The unique scatter returns flat input gradients,
which are gathered back to the matrix width. Partition validity masking remains
outside the materializer, so invalid training rows cannot contribute.

### Inference

Checkpoint-backed inference validates saved logical geometry before graph
construction. compile_convolution repeats the direct preparation and graph
sequence with saved weight, bias, and optional PReLU images. It checks that
saved input length, channels, and matrix width agree with the preceding logical
shape, and that the prepared output length agrees with the checkpoint.

compile_pool checks saved input width, logical length, channels, and output
width, regenerates ChannelwiseMaxPool1dPreparation, verifies its group count,
and calls materialize_composition for recipe_max_pool_1d with saved
parameters. It then unpacks the grouped output. Every materialized node is
assigned IterationDomain::first() in the static inference program. The final
graph is validated, serialized to canonical OGDL, and reconstructed before
native execution.

## Checkpoint state and resume invariants

Convolution checkpoints retain the declaration, resolved
DenseConvolutionGeometry, f32 weight and bias parameter images, and ordered
PReLU parameter images. Checkpoint validation requires declaration filters and
kernel to equal geometry, at most one activation operation, input geometry to
match the preceding logical shape, kernel to fit, and output length to equal
input_length - kernel + 1. Weight shape is
[kernel, input_channels, filters]; bias shape is [filters]; PReLU count and
scalar shapes must match activation occurrences.

Pool checkpoints retain size, optional group-to-neuron width, input length,
channels, output length and the explicit group-major-channel-minor and
lowest-logical-index contracts. Validation recomputes the canonical
CheckpointPoolImage, checks cached widths against the preceding logical shape,
requires ceil(input_length / size) output groups, and requires a
group-to-neuron route to target the immediately following ordinary layer of
the saved width. Pool blocks own no parameter or optimizer-moment tensors.

Saving converts compiled block declarations and state into these images. On
resume, an existing semantic model is decoded and applied to the freshly
compiled graph only after all shape and state checks pass. Missing resume files
are handled by the outer training contract as a fresh run; they do not alter
the convolution or pooling materializer contracts.

## Failure and edge-case behavior

The following failures are deliberate and observable at the boundary where
the bad state is first known:

* Unknown or ambiguous symbols fail registry resolution. Exact source mismatch
  is not a fallback.
* A non-composition descriptor, malformed composition, unresolved shape bound,
  or expansion beyond one million resolved steps fails before concrete graph
  emission.
* Missing, extra, duplicated, or incorrectly typed tensor and parameter names
  fail exact ABI validation. Missing Bool(true) facts fail as missing or false
  preparation facts, not as warnings.
* Zero or int32-incompatible dimensions, products, contribution tables, or
  scalar divisors fail with UnsupportedConcreteShape; multiplication overflow
  is WorkspaceArithmeticOverflow.
* A convolution kernel wider than its input, a 2-D effective kernel wider than
  the padded input, or a 2-D pooling kernel wider than the input is rejected.
  Zero padding is specifically rejected only for gpu_im2col_2d_ext, because
  that graph has no zero-fill primitive; extended col2im accepts padded
  tables through valid_contributions.
* A nonzero gpu_im2col_2d_ext padding request is not silently cropped, and a
  malformed extended inverse table cannot exceed its complete patch shape.
* Wrong dtype or shape, mismatched image-base/output contracts, non-external
  request tensors, aliasing IDs, graph family mismatch, and graph validation
  errors fail as typed operation errors.
* Invalid runtime gather or scatter coordinates remain device-visible
  IndexBounds::Reject failures even when host preparation facts are true.
* A unique-scatter fact selects ScatterConflict::UniqueIndices; overlapping
  average and two-dimensional maximum gradients select relaxed atomic add.
  There is no alternate scatter implementation or retry path.

## End-to-end result

For a legacy structured operation, the real path is

~~~
operation-surface.txt
  -> build.rs generated source-qualified registry
  -> CompositionRecipe validation and static expansion
  -> MaterializationRequest exact ABI and prepared-fact checks
  -> convolution_pooling dispatch
  -> typed Gather/Reduce/Elementwise/Scatter graph
  -> workspace and identity validation
  -> graph insertion into training or inference static program
  -> measured native preparation and execution
~~~

For a public dense model, .conv takes the paired direct compiler path while
.pool takes the Recipe-owned materializer path. Both paths produce immutable
f32/int32 graph payloads, preserve the init -> loop -> exit lifecycle, and
carry the resolved logical geometry into checkpoint validation and target-free
inference. No host payload loop, hidden padding, implicit output layer, or
semantically adjacent fallback is introduced by this family.
