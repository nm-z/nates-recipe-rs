# Channelwise one-dimensional maximum pooling

This document describes the Recipe-owned `recipe_max_pool_1d` and
`recipe_max_pool_1d_backward` operations. They are the concrete implementation
of the public `.pool(size)` model block. The operations are deliberately
separate from the source-qualified legacy `gpu_max_pool_*` entries: the public
pool block has a logical `[batch, length, channels]` contract, keeps a bounded
tail window, and carries its own checked preparation tables.

The implementation is split across these boundaries:

| Boundary | Implementation |
| --- | --- |
| Declaration | `src/api.rs`, `Model::pool`, `LayerSpec::Pool` |
| Facade to training IR | `src/training.rs`, `map_dense_block` |
| Logical state and routing | `training/src/model.rs`, `DensePool`, `DensePoolState`, `DenseGroupToNeuronRouting` |
| Immutable tables and scratch accounting | `ops/src/pooling.rs`, `prepare_channelwise_max_pool_1d` |
| Registry and operation identity | `ops/src/registry.rs`, the two `RECIPE_OWNED_OPERATIONS` rows |
| Composition recipe | `ops/src/composition.rs`, `POOL_STEPS` and `POOL_BACKWARD_STEPS` |
| Concrete graph | `ops/src/materialize/convolution_pooling.rs`, the two channelwise emitters |
| Training caller | `training/src/compile.rs`, `compile_pool_forward` and `backward_pool` |
| Target-free inference caller | `training/src/inference.rs`, `compile_pool` |
| Semantic checkpoint image | `training/src/checkpoint.rs`, `CheckpointPoolImage` |

The operation emits only Recipe calculation primitives. Preparation computes
all index tables on the host before the graph is finalized; execution performs
checked gathers, a fixed-tree maximum reduction, typed scalar maps, and
checked unique scatters. No pooling callback, data-dependent host loop, or
pool-specific native library call exists after preparation.

## Public declaration and topology

`Model::pool(size)` constructs `LayerSpec::Pool { size,
group_to_neuron: None }`. `LayerSpec::validate` requires a nonzero size. A
declaration error is deferred onto the model and is returned by the model
validation boundary, rather than causing declaration-time I/O or execution.

When `Model::layer` receives an ordinary dense layer immediately after a pool,
it fills the preceding pool's `group_to_neuron` field with a derived group
connection. The connection records the dense layer width. Final model
validation requires that this connection still refers to the immediately
following ordinary layer with exactly that width. A pool may therefore feed a
dense layer, but a stale or moved grouped connection is rejected.

The training facade maps the declaration to
`DensePool::new(nonzero_size, optional_dense_width)`. The pool owns no learned
weight, bias, or optimizer-moment image. Its only state is shape metadata and
the static routing contract.

The compiler starts each structured spatial stream with logical shape
`[length, channels]`, where a plain dense feature matrix is represented as
`[feature_width, 1]`. Pooling resolves

```text
groups       = ceil(input_length / pool_size)
output shape = [rows, groups, channels]
output width = groups * channels
```

The length axis is partitioned into consecutive non-overlapping groups. The
channel axis is never pooled or reordered. A final group shorter than
`pool_size` is retained. A pool is not an implicit output projection: the
training compiler rejects a final declared pool and requires an explicit
output layer.

### Dense routing after a pool

If a dense layer immediately follows the pool, `DensePool::routing` resolves
the saved dense width against the actual `groups` count. The possible routing
contracts are:

* `Identity` when groups equal neurons;
* `Expand` when neurons is divisible by groups, with contiguous equal neuron
  ranges per group;
* `Contract` when groups is divisible by neurons, with contiguous equal group
  ranges per neuron;
* `FullyConnected` for non-divisible counts.

The compiler checks that `groups * channels` equals the dense input width and
that the neuron count equals the dense output width. It emits a f32 weight mask
over `[input_width, output_width]`; the mask program derives the group from
`position / (channels * neurons)` and the neuron from `position % neurons`.
Disallowed entries are multiplied to exact `+0.0`. The mask is used by both
training and validation, so routing cannot silently change between phases.

## Logical flattening contract

The stable logical order is length-major, channel-minor:

```text
[row, position, channel]
flat = (row * input_length + position) * channels + channel
```

The pooled output uses group-major, channel-minor order:

```text
[row, group, channel]
flat = (row * groups + group) * channels + channel
```

The compiler's matrix boundary has the same order because a matrix row has
`width = length * channels`. `pack_matrix_to_flat` uses identity indices and a
checked unique scatter to turn `[rows, width]` into `[rows * width]` without
changing the order. `unpack_pool_to_matrix` uses identity `[groups, channels]`
indices and another checked unique scatter on axis one to restore
`[rows, groups * channels]`.

## Immutable preparation

`prepare_channelwise_max_pool_1d(batch, input_length, channels, pool_size)` in
`ops/src/pooling.rs` is the only producer of the Recipe-owned pool tables.
`ChannelwiseMaxPool1dPreparation` is immutable, cloneable, and exposes the
resolved dimensions, tables, prepared parameter maps, and scratch accounting.

### Admission checks

The preparation function checks each of `batch`, `input_length`, `channels`,
and `pool_size` for zero. A zero produces
`OperationErrorKind::UnsupportedConcreteShape` with `<name> must be positive`.
All products and coordinate arithmetic use checked `u64` operations. A
checked-arithmetic failure produces `WorkspaceArithmeticOverflow` and names the
operation such as `maximum-pool input elements`, `maximum-pool group start`, or
`maximum-pool flat coordinate`.

The linear input element count must fit the signed 32-bit index domain. The
window-table entry count must fit it as well. Either violation is
`UnsupportedConcreteShape`. Vector capacities are converted to `usize`; a
host-address conversion failure is also `UnsupportedConcreteShape`. Every
individual coordinate is converted to `i32` through the same checked boundary.

The concrete limit is `i32::MAX` (`2_147_483_647`) for linear index counts and
coordinates. This is a table and primitive contract, not a promise that a
device can allocate that much memory.

### Resolved fields

For positive inputs, preparation computes:

```text
groups          = input_length.div_ceil(pool_size)
window_width    = min(input_length, pool_size)
input_elements  = batch * input_length * channels
output_elements = batch * groups * channels
window_entries  = output_elements * window_width
```

`window_indices` has row-major shape
`[batch, groups, channels, window_width]`. Each entry is an absolute flat
input coordinate. For batch `b`, group `g`, channel `c`, and table offset `o`:

```text
group_start = g * pool_size
position    = min(group_start + o, input_length - 1)
coordinate  = (b * input_length + position) * channels + c
```

The `min` is intentional. In the final short group, slots beyond the real
window repeat the final real coordinate. The repeat keeps the table rectangular
for the reduction primitive. It does not create a second logical winner: the
fixed lowest-index tie rule selects the first occurrence of an equal repeated
coordinate.

`winner_bases` has shape `[batch, groups, channels]` and stores the absolute
flat coordinate at `group_start` for each output. The forward scalar map adds
`local_winner * channels` to this base, turning the reduction's local axis
index into a global input coordinate.

`gradient_batch_indices` is the identity vector `[0, 1, ..., batch - 1]` in
int32 form. The backward graph consumes it through a checked axis-zero gather.

The public accessors have these exact shapes:

| Accessor | Shape or value |
| --- | --- |
| `input_shape()` | `[input_elements]` |
| `window_indices_shape()` | `[batch, groups, channels, window_width]` |
| `output_shape()` | `[batch, groups, channels]` |
| `window_indices()` | int32 absolute input coordinates |
| `winner_bases()` | int32 first-coordinate bases |
| `gradient_batch_indices()` | int32 batch identity |

### Prepared parameter ABIs

`forward_parameters(tree_lanes)` returns exactly these nine named entries:

| Name | Type | Required value |
| --- | --- | --- |
| `batch` | `U64` | prepared batch |
| `channelwise_nonoverlap` | `Bool` | `true` |
| `channels` | `U64` | prepared channel count |
| `input_length` | `U64` | prepared logical length |
| `pool_size` | `U64` | declared size |
| `tree_lanes` | `U64` | caller's reduction tree lane count |
| `tail_window_repeats_last_coordinate` | `Bool` | `true` |
| `window_indices_encode_channelwise_nonoverlap` | `Bool` | `true` |
| `winner_bases_encode_channelwise_nonoverlap` | `Bool` | `true` |

`backward_parameters()` returns exactly these nine named entries:

| Name | Type | Required value |
| --- | --- | --- |
| `batch` | `U64` | prepared batch |
| `channelwise_nonoverlap` | `Bool` | `true` |
| `channels` | `U64` | prepared channel count |
| `gradient_batch_indices_identity` | `Bool` | `true` |
| `input_gradient_base_zero` | `Bool` | `true` |
| `input_length` | `U64` | prepared logical length |
| `pool_size` | `U64` | declared size |
| `winning_indices_from_matching_forward` | `Bool` | `true` |
| `winning_indices_unique` | `Bool` | `true` |

The concrete materializer requires the exact key sets. Missing keys, extra
keys, or wrong typed values fail before graph emission. `tree_lanes` is checked
as a power of two in `1..=1024` by the shared materialization helper.

### Scratch accounting

All f32 and i32 payloads are four bytes per element. The preparation exposes:

```text
forward_workspace  = 4 * (window_entries + 2 * output_elements)
backward_workspace = 12 * output_elements
```

Forward storage is one f32 gathered-window table, one f32 maximum table, and
one i32 local-index table. Backward storage is one f32 gathered-gradient table,
one f32 mapped-gradient table, and one i32 destination table. The concrete
`GraphBuilder` independently accounts for those intermediate tensors and
rejects `WorkspaceLimitExceeded` when the request's limit is too small. The
training and inference callers use an effectively unbounded materialization
limit, but the graph still carries the checked intermediate tensor contracts.

## Registry identity and composition recipe

`ops/src/registry.rs` appends two native rows after the generated
`operation-surface.txt` prefix:

```text
recipe_max_pool_1d          ops/src/pooling.rs:channelwise_max_pool_1d
recipe_max_pool_1d_backward ops/src/pooling.rs:channelwise_max_pool_1d_backward
```

They have `surface_line == 0`, ordinals immediately after the legacy prefix,
and one occurrence each. `operation_registry().resolve_unique` is therefore
unambiguous for both symbols. The public descriptor helpers return the same
source-qualified descriptors.

Descriptor classification is source-qualified and first-match:

* lowering is `LoweringAvailability::Composition`;
* recipe names are `channelwise_maximum_pool_1d` and
  `channelwise_maximum_pool_1d_backward`;
* family is `OperationFamily::Pooling`;
* payload is `CompositionPayload::F32AndI32`;
* alias contract is `NoAlias` for these non-`gpu_` extension symbols;
* determinism is `FixedPrimitiveOrder`;
* `legacy_dtype` is absent.

The forward composition is the fixed three-step `POOL_STEPS` sequence:

1. `Gather`, role `enumerate the checked pooling window`;
2. `Reduce`, role `reduce each window in a fixed order`;
3. `Elementwise`, role `apply the average divisor or maximum tie policy`.

The backward composition is the fixed `POOL_BACKWARD_STEPS` sequence:

1. `Gather`, role `reconstruct checked source-window coordinates`;
2. `Elementwise`, role `compute each source contribution`;
3. `Scatter`, role `accumulate overlapping contributions with explicit atomic addition`.

The generic role text is shared with legacy overlapping pool operations. The
Recipe-owned channelwise backward materializer specializes the final scatter
to `ScatterConflict::UniqueIndices`, because non-overlapping groups and
channels make every recorded winner destination unique. No atomic addition is
emitted for this operation.

`expand_composition` validates the recipe, resolves any shape or prepared
parameter bounds, and records a linear dependency from each step to its
predecessor. These two recipes contain no repeat, so each resolves to exactly
three stages. The materializer's `Emitter` checks that each concrete kernel's
primitive family matches the corresponding resolved stage and rejects a short
or overlong emission.

## Forward materialization

`materialize_composition` first validates the generic request: the descriptor
must be a composition, names and tensor IDs must be unique, inputs must be
external inputs, outputs must be non-input external outputs, and all tensors
must pass `recipe-language` validation. The convolution/pooling family claims
the request only for the exact `(symbol, source)` pair. Unknown source pairs
remain unowned instead of falling through to a semantically similar legacy
operation.

`emit_channelwise_max_pool_1d` requires this exact tensor ABI:

```text
inputs:  values[F32, [input_elements]]
         window_indices[I32, [batch, groups, channels, window_width]]
         winner_bases[I32, [batch, groups, channels]]
outputs: pooled[F32, [batch, groups, channels]]
         winning_indices[I32, [batch, groups, channels]]
```

It then performs these checks and graph stages:

1. Read `batch`, `input_length`, `channels`, and `pool_size` through the typed
   parameter helpers. The batch, logical length, and channel extents are
   nonzero and fit the int32 extent range; `pool_size` is only required to be
   positive because a size wider than the input still resolves to one bounded
   group. All derived products must fit the int32 linear-index domain.
2. Require all four forward verification facts to be true and require the
   reduction tree lane count to be a valid power of two.
3. Require the exact f32/i32 dtypes and exact shapes above. A dtype mismatch is
   `InvalidMaterializationRequest`; a shape mismatch is
   `UnsupportedConcreteShape`.
4. Allocate `windows` as an f32 intermediate with the window-index shape and
   emit `PrimitiveKind::Gather(Gather { axis: 0, bounds: Reject })` from
   `values` through `window_indices`. `Reject` gives every out-of-range index a
   checked device-fault path rather than clamping or wrapping.
5. Allocate f32 `maxima` and i32 `local_indices`, both output-shaped, and emit
   `PrimitiveKind::Reduce(Reduce { operator: Maximum, axes: [3],
   keep_dimensions: false, result: ValueAndIndex, tree_lanes })`.
6. Emit a typed pair elementwise program over `maxima`, `local_indices`, and
   `winner_bases`. Its f32 output is the pooled value. Its i32 output is
   `winner_base + local_index * channels`, the global flat coordinate saved for
   backward routing.

The reduction axis is table axis three, the rectangular window width. The
lowering contract records operator-identity padding and
`LowestLogicalIndex` tie breaking. For f32 values, the backend compares its
canonical total-order keys; when values are equal, the lower logical index is
selected. Thus duplicate tail coordinates, equal finite values, and all
backend fixed-tree partitions select the same source coordinate.

The graph builder gives every emitted kernel a forbidden input/output alias
rule, reserves intermediate value and kernel identity ranges supplied by the
caller, validates each primitive graph edge, and finally validates the whole
`CalculationGraph` before returning `MaterializedComposition`.

## Backward materialization

`emit_channelwise_max_pool_1d_backward` requires this exact ABI:

```text
inputs:  output_gradient[F32, [batch, groups, channels]]
         winning_indices[I32, [batch, groups, channels]]
         gradient_batch_indices[I32, [batch]]
         input_gradient_base[F32, [input_elements]]
outputs: input_gradient[F32, [input_elements]]
```

The backward verification facts assert channelwise non-overlap, identity batch
indices, a zero input-gradient base, matching forward winners, and unique
winners. The materializer also requires `input_gradient` to have exactly the
same dtype and shape as `input_gradient_base`.

The three concrete stages are:

1. Gather `output_gradient` through `gradient_batch_indices` on axis zero with
   `IndexBounds::Reject`. The identity table makes the batch mapping explicit
   and checked, even though it preserves the current values.
2. Run `typed_pair_identity_program` over the gathered f32 gradient and the
   saved i32 `winning_indices`. This produces f32 `mapped` contributions and
   i32 `destinations` without changing either payload.
3. Scatter `mapped` into `input_gradient` using `input_gradient_base` as the
   copy base, axis zero, `IndexBounds::Reject`, and
   `ScatterConflict::UniqueIndices`.

The scatter lowerer copies the base image before publishing updates. Because
the base is verified zero, every non-winning input coordinate remains zero.
Because groups are disjoint along length and channels are independent, no two
output entries can target the same logical input coordinate. The unique policy
is therefore an invariant of the preparation tables, not a best-effort runtime
assumption.

## Training compilation path

`GraphCompiler::compile_training_blocks` resolves each `DenseBlock::Pool` by
calling `LogicalFeatureShape::pooled`. The resulting `DensePoolState` preserves
input length, channels, `groups = ceil(length / size)`,
`GroupMajorChannelMinor`, and `LowestLogicalIndex`. It then calls
`compile_training_pool` with the partition row count, block index, and the
configured reduction-tree lane count.

`compile_pool_forward` performs the following work for every training
partition:

1. Require an f32 matrix `[partition_rows, input_width]`, where
   `input_width = input_length * channels`.
2. Call `prepare_channelwise_max_pool_1d`.
3. Pack the matrix to a flat f32 tensor with a checked unique scatter and retain
   the identity indices for the backward unpack.
4. Install `window_indices` and `winner_bases` as immutable i32 external inputs
   with roles `TrainingPoolWindowIndices` and `TrainingPoolWinnerBases`.
   `external_i32_tensor` checks the element count and stores little-endian i32
   bytes for the init image.
5. Allocate pooled f32 and winning-index i32 tensors and call the compiler's
   exact-symbol `materialize` helper with the forward ABI and parameters.
6. Insert the returned graph fragment into the training graph and assign every
   emitted kernel the current training iteration domain.
7. Unpack `[partition_rows, groups, channels]` to a matrix
   `[partition_rows, groups * channels]` with checked unique scatter, retaining
   group indices for the backward route.

`PoolValues` retains the preparation, forward winners, gradient batch identity,
input matrix indices, and output group indices. It also retains the resolved
`DensePoolState`; no pool parameter state is added to the optimizer tape.

During reverse block traversal, `backward_pool` masks invalid rows to zero,
gathers the dense gradient back to `[rows, groups, channels]`, allocates a zero
flat gradient base and output, and materializes the backward ABI. It then
gathers the flat result through the saved input matrix indices to recover the
matrix-shaped input gradient. The block emits no parameter gradient and the
optimizer update traversal skips it.

Validation compilation checks that the validation logical shape resolves to
the same `DensePoolState` as training and calls the same forward compiler with
validation-specific immutable table roles. Validation has no backward path.

## Inference and checkpoint path

The semantic checkpoint stores a `CheckpointPoolImage` with:

* nonzero `size`;
* optional `group_to_neuron` width;
* `input_length`, `channels`, and derived `output_length`;
* derived `input_width` and `output_width`;
* `group_order = group-major-channel-minor`;
* `winner_contract = lowest-logical-index`.

Pool images own no parameter or optimizer-moment tensors. The serializer writes
these fields as the `pool` block. The parser accepts only the two canonical
contract strings and recomputes the image through `checkpoint_pool_image`.
That check rejects a cached output length or width that disagrees with
`ceil(input_length / size)` or with checked length/channel products. Structured
checkpoint validation also requires any group-to-neuron route to have an
immediately following ordinary layer of the saved width.

`InferenceGraphCompiler::compile_pool` checks the saved logical shape and
matrix widths against the preceding block. It calls the same preparation
function, checks that the derived group count equals the saved output length,
installs `PoolWindowIndices` and `PoolWinnerBases` external i32 inputs, and
materializes the forward operation. Inference allocates an i32 winners output
even though it is unused by target-free prediction, because the operation ABI
always returns the forward winner image. It then unpacks the pooled tensor to
the saved output matrix width. Any mismatch is an
`InconsistentCheckpoint` or arithmetic-overflow inference error before graph
construction.

The inference path therefore reconstructs tables from semantic metadata; it
does not serialize or trust a realized native kernel as semantic pool state.

## Materialization request and failure behavior

The shared `MaterializationRequest` boundary supplies named tensors, typed
parameters, the iteration-shape input, an identity namespace, and a workspace
limit. The pooling implementation relies on these fail-closed checks:

| Failure | Error kind and cause |
| --- | --- |
| Missing or duplicate symbol lookup | `UnknownOperation` or `AmbiguousSymbol` from the registry |
| Non-composition descriptor | `WrongLoweringKind` |
| Unowned source-qualified composition | `MissingConcreteFormula` |
| Missing or extra tensor/parameter names | `InvalidMaterializationRequest` from the exact ABI check |
| Missing typed parameter | `MissingPreparedParameter` |
| Wrong prepared parameter type | `PreparedParameterTypeMismatch` |
| False verification fact or wrong dtype | `InvalidMaterializationRequest` |
| Zero, oversized, or mismatched concrete shape | `UnsupportedConcreteShape` |
| Checked u64 product, coordinate, or byte total overflow | `WorkspaceArithmeticOverflow` |
| Host vector capacity or i32 conversion failure | `UnsupportedConcreteShape` |
| Bad iteration shape or unresolved recipe bound | `IterationBoundUnresolved` |
| Exceeded reserved value/kernel identity range | `IdentityNamespaceExhausted` |
| Declared tensors overlap reserved intermediate IDs | `IdentityNamespaceOverlap` |
| Intermediate storage exceeds the request limit | `WorkspaceLimitExceeded` |
| Primitive family/order or final graph validation failure | `GraphMaterializationFailed` |

Runtime index faults remain visible. Every pooling gather and scatter uses
`IndexBounds::Reject`; no clamp, wrap, retry, fallback table, or alternate
implementation masks a bad table or a corrupted external input.

## End-to-end lifecycle

The complete production path is:

```text
Model::pool(size)
  -> LayerSpec::Pool and optional immediate dense routing
  -> map_dense_block -> DensePool
  -> LogicalFeatureShape::pooled -> DensePoolState
  -> prepare_channelwise_max_pool_1d
  -> pack matrix and admit immutable i32 tables
  -> operation_registry().resolve_unique("recipe_max_pool_1d")
  -> expand_composition (three finite stages)
  -> concrete Gather -> Reduce -> Elementwise graph
  -> unpack grouped output and continue declared model blocks
  -> training backward uses matching winner table and UniqueIndices scatter
  -> graph.validate and StaticCalculationProgram canonicalization
  -> measured AOT preparation and native realization
  -> one init admission image containing feature/state/table inputs
  -> immutable loop iterations
  -> exit outputs and teardown
```

Training's `build_training_device_images` packs all external inputs into one
finalized init image per device. Pool tables are therefore uploaded once in
`init`, not regenerated or transferred in the loop. The execution boundary
rejects loop-time external transfers before starting the run. Target-free
inference follows the same graph and table admission contract with one loop
iteration and no pool-specific host work.

After `CalculationGraph` validation, pooling is not a special runtime event.
`recipe-primitives` lowers the ordinary gather, fixed-tree reduce, elementwise,
and scatter kernels for the measured CUDA or HSA target. Planning, allocation,
queueing, synchronization, native-image loading, and `init -> loop -> exit`
lifecycle ownership remain in the downstream preparation and executor crates.

## Edge semantics

For `input_length = 5`, `channels = 3`, and `pool_size = 2`, preparation gives
`groups = 3`, `window_width = 2`, and windows over positions
`[0,1]`, `[2,3]`, and `[4,4]`. The final table's repeated coordinate is a
rectangular padding representation only. For `pool_size = 1`, every group is a
singleton and the global winner is that input coordinate. For
`pool_size >= input_length`, there is one group and the window width equals the
complete input length, so no repetition is needed.

Equal maximum values always select the lowest local window index, then the
global coordinate represented by the winner base and channel stride. The same
saved global index is consumed by backward, so forward and backward cannot
disagree about the winning source.

## Source map

The paired implementation and its callers are intentionally narrow:

* [`ops/src/pooling.rs`](../../src/pooling.rs): preparation, table formulas,
  parameter ABIs, and workspace formulas.
* [`ops/src/registry.rs`](../../src/registry.rs): source-qualified extension
  rows, descriptor classification, and registry lookup.
* [`ops/src/composition.rs`](../../src/composition.rs): three-stage forward and
  backward family recipes.
* [`ops/src/materialize/convolution_pooling.rs`](../../src/materialize/convolution_pooling.rs): exact tensor ABI, shape checks, primitive graph, scalar maps, and unique backward scatter.
* [`ops/src/materialize.rs`](../../src/materialize.rs): request validation,
  recipe expansion, identity reservations, workspace accounting, dispatch, and
  graph validation.
* [`src/api.rs`](../../../src/api.rs): public `.pool(size)` declaration and
  immediate dense-routing capture.
* [`src/training.rs`](../../../src/training.rs): facade-to-`DensePool` mapping.
* [`training/src/model.rs`](../../../training/src/model.rs): logical state,
  flattening order, winner contract, and routing variants.
* [`training/src/compile.rs`](../../../training/src/compile.rs): training,
  validation, packing/unpacking, forward/backward callers, and static program
  finalization.
* [`training/src/inference.rs`](../../../training/src/inference.rs): checkpoint
  shape checks and forward-only target-free reconstruction.
* [`training/src/checkpoint.rs`](../../../training/src/checkpoint.rs): semantic
  pool image parse, validation, and serialization.
* [`training/src/execute.rs`](../../../training/src/execute.rs): one init image
  admission and the native `init -> loop -> exit` lifecycle.
