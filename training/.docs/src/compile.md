# `training/src/compile.rs`

`compile.rs` is the Recipe-owned dense-training graph compiler. It translates a
prepared, semantically typed dataset plus a `DenseTrainingConfig` and ordered
`DenseBlock` topology into one immutable `CompiledTraining`. The result is a
`recipe_program::StaticCalculationProgram`: an acyclic `CalculationGraph`, an
explicit `IterationDomain` for every kernel, and statically bound metric
emissions. It is a graph description, not a native executable and not a
runtime loop implementation.

The compiler deliberately stops before hardware discovery, profile selection,
allocation, native-image loading, and execution. `training/src/execute.rs`
consumes the returned program and inputs later. The public facade path in
`src/training.rs` prepares data, maps declarations, calls one of the entry
points below, and may then admit a checkpoint. No host transfer is added to the
live loop by this compiler.

## End-to-end position

The real dense-training path is:

```text
Train + Data + Model declarations
    -> src/training.rs::compile_training_graph
       validate facade declarations and policy
       prepare_data(data)
       map LayerSpec values to DenseBlock values
       build DenseTrainingConfig and validation config
    -> one compile_dense_training* wrapper in this module
       resolve task, lower feature/target matrices, validate topology/config
       build static forward, loss, backward, optimizer, validation graphs
       finish GraphCompiler into CalculationGraph/StaticCalculationProgram
    -> CompiledTraining
       (optional) checkpoint::apply_checkpoint_resume changes only external
       resume input bytes and sets ResumeEnabled
    -> execute.rs::prepare_and_execute_local_training_controlled
       profile-dependent preparation, native realization, init admission,
       bounded loop, exit outputs and metric mailbox
```

`src/training.rs` rejects unsupported model sources, Bayesian declarations,
non-AdamW optimizers, and missing learning-rate schedules before this module is
called. It maps public layer spellings as follows:

| Public declaration | Dense compiler block |
| --- | --- |
| `.embed(...).vocab(...)` | `DenseBlock::Embedding` |
| `.attn(heads)` | `DenseBlock::Attention` |
| `.rnn(width)` | `DenseBlock::Rnn` |
| `.gru(width)` | `DenseBlock::Gru` |
| `.lstm(width)` | `DenseBlock::Lstm` |
| dense or perc layer | `DenseBlock::Layer(DenseLayer)` |
| convolution | `DenseBlock::Convolution` |
| pool | `DenseBlock::Pool` |
| K-means | `DenseBlock::KMeans` |
| lgbm, cbst, xgbst, or forest | terminal `DenseBlock::Tree` |
| residual | `DenseBlock::Residual` |

The facade supplies the legacy `config.layers` list for flat calls. The flat
wrappers turn that list into `DenseBlock::Layer` values. Topology-preserving
callers pass an explicit block slice. There are no parallel flat and
topology compilers: all wrappers call `compile_dense_training_impl`.

## Public entry points

All entry points return `TrainingCompileResult<CompiledTraining>` and differ
only in whether they derive flat blocks and which one validation request is
passed:

* `compile_dense_training` uses `flat_dense_blocks(config)` and no validation.
* `compile_dense_training_with_blocks` uses the supplied ordered topology and
  no validation.
* `compile_dense_training_with_binary_validation` and
  `compile_dense_training_with_blocks_and_binary_validation` request BCE/focal
  validation, binary metrics, and optional temperature scaling.
* `compile_dense_training_with_multiclass_validation` and its block variant
  request categorical cross-entropy mean loss and top-one accuracy.
* `compile_dense_training_with_regression_validation` and its block variant
  request full-partition R2.

The wrappers are intentionally thin. `flat_dense_blocks` clones every legacy
layer, preserving order and operation lists, and maps it to a block. The
implementation performs every semantic check before emitting the dependent
graph.

## Inputs and intermediate contracts

### Task resolution and dataset lowering

`resolve_dense_task(dataset, loss)` reads target vectors in the exact order
reported by `PreparedDataset::target_source_indices` and returns one
`DenseTask`:

* one numeric or at-most-two-category target with BCE/focal becomes binary
  classification. Categorical labels are bound to an explicit positive code;
  an unseen reserved category is marked unsupervised.
* one categorical dictionary target with cross entropy becomes multiclass
  classification. The dictionary size plus one reserved unseen-label route is
  the class count. The reserved code is checked for `i32` representation.
* one numeric target with MSE, MAE, or Huber becomes scalar regression.
* two or more homogeneous numeric targets become a target matrix. BCE/focal
  produces independent multi-target binary classification, cross entropy
  produces a one-hot joint multiclass task, and the regression losses produce
  ordered multi-target regression.

Missing values become zero payload values plus `TargetObservation::Missing`.
Dictionary codes equal to the reserved unseen code become zero payload values
plus `TargetObservation::Unseen`. Values outside the declared dictionary,
non-finite floating targets, non-binary BCE values, and non-one-hot joint
cross-entropy rows are rejected. Integer targets must be exactly representable
when a dense f32 calculation will consume them.

`DenseFeaturePlan::from_prepared` plans the feature matrix before values are
copied:

* numeric `I32` or `F32` features have width one and participate in numeric
  normalization;
* categorical dictionary `I32` features become one-hot spans of
  `dictionary.len() + 1`, where the final column is the reserved/missing route,
  and do not participate in numeric normalization;
* duplicate or empty dictionary labels and unsupported semantic/encoding/value
  combinations are errors.

`LoweredDenseDataset::from_prepared` applies that plan to train and validation
partitions. A validation partition is compacted by `DensePartition::known_only`
to rows with known targets; `validation_split_rows` retains the original
prepared split size for availability reporting. The training partition is not
compacted. `DensePartition::target_supervision` produces an `[rows, 1]` f32
mask, one for known rows and zero for missing or unseen rows. A full-known
partition has no extra supervision input.

`validate_lowered_dataset` then checks target-column counts and stable dtypes
across partitions. Binary and scalar regression matrices must survive exact
I32-to-f32 conversion; multiclass targets must remain dictionary I32 codes;
multi-target matrices must be f32 and binary matrices must contain only zero or
one. `validate_embedding_dataset` additionally requires every feature to be an
exact numeric I32 token, checks vocabulary against `i32::MAX`, and checks every
train/validation token is in `0..vocabulary`. `validate_recurrent_dataset`
requires numeric I32 or F32 feature scalars with no metadata.

### Logical shape and topology

`LogicalFeatureShape` carries a nonzero sequence `length` and nonzero channel
count. A scalar feature matrix starts as `length = input_width,
channels = 1`. Its checked transitions are:

* `embedded`: only an unchannelled fixed token sequence can enter an embedding;
  channel count becomes embedding dimensions.
* `attended`: channel count must divide evenly by the requested head count;
  the returned head dimension is checked nonzero.
* `recurrent`: only one scalar per time step is admitted; the returned logical
  length is the recurrent width.
* `pooled`: length is ceiling-divided by pool size and cannot become zero.
* `convolved`: kernel must not exceed length; output length is
  `input_length - kernel + 1`; filters become channels.
* `width`: checked multiplication of length and channels.

`effective_blocks` propagates this shape through the declared sequence and
checks the terminal contract. A tree is valid only as the sole block. A pool
cannot be the final block. Every other final width must equal
`DenseTask::output_width`, and classification tasks require the final block to
have only linear output operations, so the loss receives raw logits. The
`DenseOutputAdapter` return slot is retained in `CompiledTraining` for the
public model contract, but this implementation returns `None` for the accepted
paths and never silently inserts a width adapter.

`validate_config` checks the remaining topology and policy invariants:

* at least one block;
* one leading embedding only, because token IDs have no input gradient;
* attention only immediately after that embedding and at most once;
* the first RNN, GRU, or LSTM case must be the leading block and may not be
  duplicated;
* a tree or forest is one terminal block, with depth at most 30 for checked
  int32 complete-tree indices;
* pool and K-means grouped routing must be followed immediately by a layer of
  the declared destination width;
* a residual must contain at least one branch layer;
* embedding input requires `Identity` data handling, while non-embedding input
  requires an explicit numeric normalization;
* finite warmup cannot exceed the finite horizon; unbounded training accepts
  only constant post-warmup learning rate and warmup no larger than
  `i32::MAX`;
* reduction lanes are a power of two in `1..=1024`;
* clip norm, normalization epsilon, learning rate, AdamW epsilon, and weight
  decay must be finite with the documented positivity/nonnegativity rules;
  beta one and beta two are finite and in `[0, 1)`.

`training_bounds` converts the nonempty training row count to `u64` and limits
it to the int32 `IndexMap` domain. Finite training iterations must fit
`i32::MAX`; calibration iterations may be added only to a finite horizon.
Unbounded training cannot request post-training calibration. The returned
`TrainingBounds` records train rows, declared horizon, training iterations,
calibration iterations, total iterations, and warmup iterations.

When a training partition can contain unknown targets,
`accepted_update_plan` enforces at most one accepted full-partition update per
epoch. It computes finite maximum and warmup accepted-update counts, checks
them against physical loop bounds and the int32 schedule domain, and keeps a
counter limit of at least one. A partition with no known target still executes
its calculations but gates optimizer state and learning rate to no update.

## `GraphCompiler` state and graph boundary

`GraphCompiler` is the only graph builder in this module. Its state is:

* `tensors: BTreeMap<ValueId, Tensor>` with exact dtype, shape, contiguous
  row-major layout, storage bytes, and eventual boundary flags;
* ordered `nodes: Vec<CalculationNode>` and matching `domains` entries;
* checked `next_value`, `next_kernel`, and random `next_weight_stream`
  identities, all beginning at one or zero as appropriate;
* `next_parameter_input`, which assigns ordered resume parameter ordinals;
* the total `LoopIterations` and `training_domain` (`every(training_iterations)`);
* one `resume_enabled` I32 scalar and sets of external input/output IDs;
* owned `OwnedExternalInput` records containing role, value ID, dtype, shape,
  and initial bytes.

`GraphCompiler::new` emits `ExternalInputRole::ResumeEnabled` as an I32 `[1]`
zero. This is the sole graph-wide resume selector. Fresh values and resume
values are both constructed statically; `select_resume` emits a scalar
ternary selecting resumed bytes only when that selector is nonzero.

External helpers validate all dimensions and byte counts before adding inputs:

* `external_matrix` converts `DenseMatrix::I32` or `F32Bits` to little-endian
  bytes and checks shape byte size;
* `external_f32_vector` and `external_f32_tensor` reject empty or non-finite
  bit patterns and check element counts;
* `external_f32_zeros` and `external_i32_zeros` allocate correctly sized zero
  images for resume or prepared index data;
* `convert_matrix_if_i32` emits the small `ConvertI32ToF32` scalar program only
  when the source tensor is I32.

The compiler uses `IterationDomain::first()` for init-time constants, random
initialization, tree/K-means structure construction, and static index maps. It
uses `training_domain` for one complete logical partition per training loop
iteration. Validation kernels use `IterationDomain::every(bounds.training_iterations)`.
Temperature scaling starts in a domain whose first iteration is the end of the
training phase and whose finite extent is the total training plus calibration
iterations. No graph is unrolled for epochs.

`ExternalInputRole` values used by this compiler include train features,
targets, and target supervision; validation features and targets; the resume
selector and three resume images per learned parameter; K-means centroids;
tree split features and thresholds; pool window, winner, and backward index
tables; convolution window and backward index/validity tables; and a feature
normalization mask. The role is the stable bridge used by checkpoint and
execution code, not a second source of domain state.

The internal tape types make the forward/reverse ownership explicit:

* `InitialParameter` is one value plus first and second AdamW moments.
* `LayerValues`, `ConvolutionValues`, and `OperationValues` retain the exact
  input, output, optional variance, routing mask, and parameter values needed
  for their reverse operations.
* `EmbeddingValues`, `AttentionValues`, `RnnValues`, `GruValues`, and
  `LstmValues` retain block geometry, parameter handles, and per-step or
  per-head intermediates.
* `PoolValues`, `KMeansValues`, and `TreeValues` retain preparation tables,
  distances, split arrays, leaf IDs, and operation-owned state.
* `ResidualValues` owns the branch tape, optional projection, and post-merge
  operation tape. `BlockValues` is the one ordered enum that covers every
  forward block variant.
* `GradientPair`, `ResidualBranchGradients`, and `BlockGradients` mirror those
  variants for reverse mode. `NormalizedValues`, `SoftmaxValues`, and
  `ValidationValues` carry the statistics and boundary values consumed by
  later stages. `AcceptedUpdatePlan` is the checked host-side description of
  the device update gate and counter.

These are compile-time graph handles, not host arrays of model payloads. The
payload lives in tensors identified by `ValueId`; the tape only guarantees
that every consumer and gradient has a typed producer.

## Main implementation pipeline

`compile_dense_training_impl` is ordered so every later graph stage consumes
validated earlier state:

1. Resolve `DenseTask`, validate config and declared block order, build the
   feature plan and lowered dataset, and run task, embedding, and recurrent
   dataset checks.
2. Resolve effective blocks and output width, construct `CompiledDatasetSchema`,
   validate at most one requested validation family, and decide whether a
   validation partition is available. A requested metric family with no known
   validation rows is represented as `ValidationMetricStatus::Unavailable`,
   rather than compiled with fabricated rows.
3. Compute `TrainingBounds` and, if target supervision is masked, the accepted
   update plan. Construct the compiler and admit train/validation tensors.
4. Convert numeric I32 train/validation features and non-multiclass targets to
   f32. Multiclass target codes remain I32 until their stable categorical loss
   consumes them.
5. Apply the selected input normalization, preserving state needed by
   execution/checkpoint output. Create validity indices and counts, supervision
   masks, and a target-matrix objective denominator.
6. Lower forward blocks, mask the output and targets, emit the selected loss and
   loss gradient, normalize the gradient over supervised elements, and reduce
   masked losses to the training-loss scalar.
7. Lower reverse-mode block gradients, flatten them in parameter-role order,
   optionally emit global clipping, derive dynamic AdamW scalars, and lower one
   update state per learned parameter.
8. Lower requested validation and calibration graphs using updated parameters,
   bind all metrics, mark normalization and state outputs, then call
   `GraphCompiler::finish`.

### Normalization and supervision

The compiler creates a full training shape `[train_rows, feature_columns]` and
target shapes `[train_rows, target_code_columns]` and `[train_rows, output_columns]`.
For `Identity` the converted matrix is used directly. `z_score` computes
per-column sums, means, squared deviations, variances, and applies a guarded
epsilon denominator; `min_max` computes per-column extrema and a guarded range;
`l2_norm` computes per-row squared norms and a guarded square-root denominator.
Categorical one-hot columns are excluded from numeric normalization by the
optional f32 feature mask.

The `[rows, 1]` `validity` index map is one for every retained training row and
zero for any physical lane outside that row count. `valid_count` is its f32
sum. When target supervision exists, `supervised_count` is the safe known-row
count and `update_signal` is the unsafeguarded known count used only for the
optimizer gate. Otherwise validity itself supplies both. Every f32 value and
I32 category code entering the loss is masked to zero for unsupervised rows.
For a target matrix, the objective denominator is supervised count times output
width except for cross entropy, whose row-wise softmax already couples columns.

### Forward block lowering and tapes

`compile_training_blocks` walks blocks in declaration order, carries both a
flat width and `LogicalFeatureShape`, and records a `BlockValues` tape for the
reverse pass. It also carries one optional pool/K-means grouped routing contract
to the immediately following dense layer.

#### Embedding

`compile_training_embedding` initializes a learned table with shape
`[vocabulary, dimensions]`, checks the input as I32 `[rows, sequence]`, packs
token IDs to flat storage, gathers rows from the table, and reinterprets the
result as `[rows, sequence * dimensions]`. `EmbeddingValues` retains indices,
table parameter/moments, and all resolved geometry. Token IDs are never
normalized or differentiated.

#### Causal attention

`compile_training_attention` initializes independent query, key, value, and
output `[dimensions, dimensions]` parameters. `compile_attention_forward`
reinterprets `[rows, sequence * dimensions]` as `[rows, sequence, dimensions]`,
projects Q/K/V, reinterprets them as
`[rows, sequence, heads, head_dimension]`, and contracts Q with K into
`[rows, heads, sequence, sequence]`. Scores are scaled by
`1 / sqrt(head_dimension)`. `causal_softmax` reshapes rows and heads, creates a
triangular I32 visibility mask, and materializes `gpu_causal_softmax_rows` with
the mask explicitly verified. Probabilities contract with V, are reordered
from head-major to sequence-major, and pass through the output projection.
`AttentionValues` retains every intermediate needed by the backward pass.

#### RNN, GRU, and LSTM

The compiler delegates exact recurrent equations to `forward.rs` through
`TrainingForwardGraph`, which exposes only zero tensors, matrix-column gathers,
bias-free contractions, f32 scalar programs, and activation lowering.

* Vanilla RNN starts with a zero `[rows, width]` hidden tensor. Each feature
  column is gathered as one scalar step, projected by input and recurrent
  weights, summed with bias, and passed through tanh. The final hidden state is
  the block output; every step is retained in `RnnValues`.
* GRU has reset and update sigmoid gates, a reset-hidden candidate projection
  with tanh, and `candidate + update * (previous - candidate)` hidden update.
  Every gate projection and intermediate is retained in `GruValues`.
* LSTM starts with zero hidden and cell state. Four gates use sigmoid for input,
  forget, and output and tanh for the candidate; the cell is
  `forget * previous_cell + input * candidate`, and hidden is
  `output * tanh(cell)`. Every gate, cell, and activation is retained in
  `LstmValues`.

These first recurrent cases consume one numeric scalar per feature-column time
step. Validation calls the same forward lowering with updated parameters and a
different iteration domain.

#### Dense layer and ordered operations

`compile_training_layer` creates a `[input_width, output_width]` random weight
and zero bias. If pool/K-means routing is present, `group_routing_mask` emits a
static f32 mask and zeroes forbidden weight entries. The linear calculation is
the Recipe-owned `gpu_linear_into` materialization. Operations then run in
declaration order:

* activations use `apply_activation`; linear returns its input, owned operation
  symbols resolve through the operation registry, and canonical scalar
  programs cover composite activations;
* PReLU allocates a learned scalar initialized to `0.25` with zero moments;
* layer or batch normalization records variance and masks the output.

`LayerValues` retains input, original/forward weight, optional routing mask,
bias, and operation tapes. `OperationValues` retains safe input, output,
normalization variance, and an optional PReLU parameter.

#### Convolution and pooling

`compile_training_convolution` resolves `DenseConvolutionGeometry`, masks the
input, initializes a `[kernel, input_channels, filters]` weight and filter bias,
and calls `prepare_channelwise_convolution_1d`. The preparation provides static
window indices and output shape. The compiler packs the matrix, gathers
windows, contracts kernel and channel axes, adds bias, and scatters grouped
output to `[rows, output_width]`. It retains columns and index tables for
backward input and weight gradients. The same ordered operation path as a
dense layer follows the convolution.

`compile_training_pool` calls `prepare_channelwise_max_pool_1d` and the
Recipe-owned `recipe_max_pool_1d` materialization. It admits window indices,
winner bases, and gradient-batch indices as external init data, retains winning
indices, and unpacks grouped `[rows, groups, channels]` output to a matrix.
The preparation's lowest-logical-index winner contract is used by both forward
and backward paths. Pooling has no learned parameter.

#### K-means

`compile_training_kmeans` materializes K-means initialization in the first
domain, reserves the exact identities reported by
`kmeans_initialization_requirements`, and selects either fresh centroids or
checkpoint-provided centroids through `resume_enabled`. It then materializes
one Lloyd step over the training domain using `kmeans_lloyd_requirements`,
emitting distance and updated-centroid values. Updated centroids are external
outputs and are retained in `DenseKMeansState`; distances feed the loss and
backward input gradient. The centroids are not an optimizer parameter.

#### Supervised trees and forests

`compile_training_tree` resolves exact tree requirements from
`tree_ensemble_inference_requirements`, including internal nodes, leaves, split
array size, and leaf-value size. Categorical targets become one-hot matrices
for tree construction. A single tree uses the full partition; a forest emits a
deterministic Philox bootstrap sample per tree.

`compile_supervised_tree_structure` builds split features and thresholds in the
first domain. The XGBoost-like path evaluates each complete binary level;
CatBoost computes global per-feature thresholds and chooses one shared feature
per level; the LightGBM path repeatedly finds the best active leaf candidate,
writes a split only when gain is positive, and routes rows to children. All
candidate indices, counts, means, gains, finite checks, and checked complete
tree indices are f32/I32 scalar programs plus deterministic segment sums and
value/index reductions. The split arrays select checkpoint values when resume
is enabled and are external outputs.

`materialize_tree_predictions` reserves the operation's exact identity
namespace and inserts `materialize_tree_ensemble_inference` into the graph.
`route_tree_leaf_indices` walks every tree depth with checked split and feature
index programs and emits `[rows, trees, outputs]` leaf IDs. Leaf values are the
only tree parameters and are updated by AdamW; split arrays are structural
state. A tree is terminal, so the reverse pass cannot request an input
gradient.

#### Residual blocks

The residual branch is lowered as an ordinary ordered sequence of dense layers
and operations. Its final branch width must match the declaration. Equal input
and output widths use an identity skip. A width mismatch initializes a learned,
bias-free `[input_width, output_width]` projection and contracts the input.
The branch and skip are combined by `exact_add`, which requires equal dtype and
shape. Post-merge operations then run in declaration order. `ResidualValues`
retains branch tapes, optional projection, and post-merge operation tapes.

### Loss and training scalar outputs

After forward lowering, the current output and target matrix are masked. Loss
selection is exact:

* BCE uses the Recipe operation `gpu_bce_with_logits`, returning per-element
  loss and gradient.
* Focal uses `canonical_focal_with_logits_program`.
* MSE, MAE, and Huber use `pointwise_loss_program`, which checks finite input
  and target values and emits both loss and derivative.
* Categorical cross entropy first computes a stable row-wise softmax: maximum,
  shifted exponentials, exponential sum, and logarithmic sum. Integer targets
  use `cross_entropy_with_logits_program`, which checks class range and emits a
  one-hot indicator; dense one-hot targets use
  `cross_entropy_with_dense_targets_program`, which checks each target in
  `[0, 1]`.

`masked_mean_program` divides each supervised gradient element by the safe
known count and returns zero for unsupervised rows. Masked losses are reduced
over rows and outputs to `loss_sum`, then divided by the objective denominator
to produce `training_loss`. `training_loss_domain` is the training domain, so
it is a loop metric rather than an external output.

## Reverse-mode lowering

`backward_blocks` traverses `BlockValues` in reverse order and produces exactly
one `BlockGradients` value per block. It enforces the topology rules again at
the point where they matter: embedding and recurrent input gradients are not
allowed in the first-block cases, attention must have a differentiable
embedding predecessor, and a supervised tree cannot expose a differentiable
input.

`backward_operation` masks upstream gradients and dispatches activation,
PReLU, or normalization derivatives. `backward_activation` uses owned
backward symbols for ReLU, sigmoid, tanh, GELU, and SiLU; canonical operation
programs for LeakyReLU, SELU, ELU, and PReLU; and dedicated scalar programs
for cosine, logarithm variants, Huber, and tangent. PReLU additionally reduces
the element derivative to one learned-slope gradient. `backward_normalization`
computes masked gradient and gradient-times-normalized means, applies guarded
inverse standard deviation, and re-masks batch-normalized results.

Block-specific reverse lowering is:

* `backward_embedding` reshapes output gradients to token rows and materializes
  `gpu_embedding_backward`, which accumulates table rows by token index.
* `backward_attention` contracts output projection gradients, reverses the
  causal softmax and score scaling, contracts Q/K/V gradients, converts head
  order back to sequence order, and emits query/key/value input and weight
  gradients.
* `backward_rnn` walks steps backward, differentiates tanh, contracts input and
  recurrent weight gradients, reduces bias gradients, and propagates hidden
  gradients.
* `backward_gru` differentiates candidate, reset product, reset/update gates,
  and all four previous-hidden contributions. `backward_lstm` carries both
  hidden and cell gradients through all four gates and the cell recurrence.
* `backward_layer` and `backward_convolution` reverse operation tapes, then
  materialize full or weights-only linear gradients. Routing masks are applied
  to weight gradients; convolution uses prepared contribution indices and
  validity tables for input gradients.
* `backward_pool` materializes `recipe_max_pool_1d_backward` using saved winner
  IDs and gathers the matrix-shaped input gradient.
* `backward_kmeans` scales distance gradients by guarded distances, contracts
  centroid terms, and subtracts them from the input term. K-means centroid
  updates remain operation-owned state, not AdamW gradients.
* `tree_leaf_value_gradient` gathers each row's leaf gradient and uses a
  deterministic segment sum over leaf IDs.
* residual backward first reverses post-merge operations, then the branch,
  computes a projection weight and input gradient when present, and exact-adds
  branch and skip input gradients.

`flatten_parameter_gradients` converts the block-shaped result to a single
ordered tape of `(ParameterRole, ValueId)`. The order is part of the checkpoint
and optimizer contract. It covers embedding tables; attention Q/K/V/output;
RNN parameters; all GRU and LSTM gate parameters; dense and convolution
weight/bias plus PReLU slopes; tree leaf values; and residual branch slopes,
projection, and post-merge slopes. Pool and K-means blocks contribute no
AdamW gradient. `ParameterUpdates::take` checks every role against the expected
declaration order and rejects early or unused gradients.

For a concrete declaration order, `flatten_parameter_gradients` emits these
roles in block order: embedding table; attention query, key, value, output;
RNN input weight, recurrent weight, bias; GRU reset input/recurrent/bias,
update input/recurrent/bias, candidate input/recurrent/bias; LSTM input-gate
input/recurrent/bias, forget-gate input/recurrent/bias, output-gate
input/recurrent/bias, candidate input/recurrent/bias; layer weight, bias, then
each PReLU slope; convolution weight, bias, then each PReLU slope; no pool or
K-means entries; tree leaf values; and residual branch layer parameters and
free-operation PReLU slopes, optional projection weight, then post-merge
PReLU slopes. `update_blocks` consumes exactly this sequence even though its
variant match is arranged for readability rather than declaration order.

## Optimizer and schedule graph

`global_clip` optionally computes one squared norm per gradient, reduces all
norms, forms `min(1, maximum_norm / max(sqrt(norm), f32::EPSILON))`, and scales
every gradient by that one f32 factor.

`dynamic_adam_scalars` emits all schedule state on the device:

* with masked targets, `optimizer_update_gate_program` turns known-count into
  an I32 `apply_update`; `accepted_update_program` increments a bounded accepted
  update counter only when that gate is set;
* finite schedules use an iteration `IndexMap` starting at one; unbounded
  schedules use a saturating I32 recurrence with the warmup limit;
* `schedule_inputs_program` emits exact warmup, progress, and remaining
  fractions. Large integer ratios use a checked 12-bit-limb quotient and FMA
  correction so I32 schedule coordinates do not lose endpoint behavior;
* constant decay is one, linear uses remaining fraction, cosine uses a guarded
  squared sine curve, and exponential uses a normalized `exp(-5 * remaining)`
  curve with exact start/end selections;
* `learning_rate_program` multiplies base rate, warmup, and decay, and gates it
  to zero for an unsupervised partition;
* beta powers initialize to one and update with exact pair recurrence aliases,
  optionally gated by `apply_update`.

`adamw_update` emits one `Elementwise` primitive per parameter with gradient,
weight, first and second moments, learning rate, beta powers, and optional gate.
The scalar program computes Adam bias correction, epsilon-protected adaptive
step, decoupled weight decay, and gated state retention. `exact_adam_aliases`
requires the previous weight and moments to alias their corresponding outputs
and forbids every other alias. Updated parameter, moments, and update kernel
are external outputs and are recorded in `ParameterState`.

`update_blocks` walks the same block and role order as the flattening pass and
builds persistable `DenseBlockState` values. It preserves geometry and static
index IDs for embedding, attention, recurrent, convolution, pool, K-means,
tree, and residual blocks. `OptimizerProgressState` records accepted-update
counter values, beta powers, gate and recurrence kernels, and finite bounds,
but is execution state, not a user artifact.

## Validation, metrics, and calibration

`validate_validation_config` permits at most one binary, multiclass, or
regression family. It checks that the loss/task pair matches the requested
family and that the prepared validation split is nonempty. Binary thresholds
must be finite, in `[0, 1]`, and have distinct f32 bit patterns; operation
requirements are checked for row count, threshold count, and calibration-bin
limits. Temperature scaling requires finite positive learning rate and strict,
positive minimum and maximum temperatures.

`validation_metric_status` distinguishes no known validation rows from an
available known-only partition. Binary availability adds a stricter check:
every output column must contain both a known zero and a known one. A
single-known-class split is represented as `Unavailable` with its original
split row count and is not compiled into metric calculations.

When retained, validation inputs are admitted once and validation kernels run
over `IterationDomain::every(training_iterations)`. `compile_validation_blocks`
replays the exact topology using updated parameter values and checks every
declaration/state pair, geometry, routing contract, and ordered PReLU state.
Validation normalization computes fresh validation statistics for layer or
batch normalization; it does not reuse mutable training-loop statistics.

The retention switch is intentionally narrow: `compile_dense_training_impl`
retains a prepared validation partition for `NotRequested` as well as
`Available`, so a caller can inspect the same admitted boundary later, but it
drops the partition only for an explicitly `Unavailable` metric status. No
validation calculation is emitted when no family was requested.

Binary validation applies sigmoid to logits, computes BCE or focal per-element
loss, and then:

* for one target column, materializes
  `materialize_binary_classification_metrics` with mean BCE, AUROC, AUPRC,
  Brier score, expected calibration error, and configured recall-at values,
  plus Recipe-owned accuracy;
* for multiple target columns, computes those metrics independently per
  column, averages scalar results, and computes matrix accuracy. Temperature
  scaling is rejected for this multi-target case.

Multiclass validation uses stable integer or dense-target cross entropy,
reduces mean loss, and computes accuracy through `gpu_accuracy` or
`gpu_argmax_accuracy_into`. Regression validation computes residual sum of
squares, target total sum of squares, and guarded R2 (`1 - RSS/TSS`).

`compile_temperature_scaling` is allowed only after a finite training phase. It
creates an initial temperature of one, divides validation logits by it, emits
BCE gradients, reduces the temperature gradient, and updates a clamped scalar
through a must-alias recurrence for the configured calibration iterations.
Updated temperature is an external output. Its domain begins at the first
post-training iteration, so calibration is a distinct static phase rather than
part of the training epoch update.

`training_metric_bindings` assigns monotonically increasing nonzero
`MetricId`s to training loss, learning rate, and any validation metrics. Each
binding carries the scalar `ValueId` and its domain. These become
`MetricEmission`s in the final `StaticCalculationProgram`; metric tensors stay
loop-internal and are never marked external outputs.

## Recipe language, operations, and primitive boundaries

The compiler owns the semantic lowering but emits only the existing Recipe
calculation/transfer reduction:

* `PrimitiveKind::Elementwise` carries a validated
  `recipe_core::ScalarProgram`.
* `PrimitiveKind::Reduce` carries operator, axes, keep-dimensions, and a
  power-of-two deterministic `tree_lanes` count.
* `PrimitiveKind::Contraction` describes ordered batch and contract axes for
  matrix, recurrent, attention, convolution, and gradient products.
* `PrimitiveKind::Gather` and `PrimitiveKind::Scatter` use `IndexBounds::Reject`
  and unique-index conflicts for reindexing and packing. Their index values are
  checked I32 tensors.
* `PrimitiveKind::IndexMap` produces affine iteration-aware I32 coordinates.
* `PrimitiveKind::Random` uses Recipe's explicit ten-round Philox mapping and
  deterministic seed/stream values.

`emit_elementwise`, `elementwise_f32`, and `owned_scalar_f32` centralize tensor
allocation and forbidden alias contracts. `emit_owned_scalar` resolves a
unique symbol through `operation_registry`, then calls `lower_scalar`; this is
how owned operations such as `gpu_sigmoid_into`, `gpu_tanh_into`, and
`gpu_exp` enter the graph. Composite activations and all domain-specific
formulas use `ScalarProgramBuilder` programs constructed in this file or
`forward.rs` and `recipe_ops`.

`materialize` is the boundary for a Recipe operation composition. It resolves
the operation descriptor, wraps named input/output tensor contracts, reserves
`MATERIALIZATION_RESERVATION` (64) value and kernel identities, passes an
`IdentityNamespace` and `WORKSPACE_LIMIT`, and inserts the returned graph
fragment. Current symbols include `gpu_linear_into`, its full and weights-only
backward forms, `gpu_bce_with_logits`, `gpu_causal_softmax_rows`, embedding
backward, pairwise L2, segment sum, accuracy, max-pool forward/backward, and
the tree/K-means/binary-metric materializers. The operation crate validates
shapes, dtypes, aliases, exact resource requirements, and workspace before a
fragment is accepted.

`insert_materialized_graph` imports fragment tensors and nodes. Repeated tensor
IDs are allowed only when dtype, shape, layout, and storage bytes match exactly;
conflicts are `Language` errors. Fragment-local boundary flags are discarded
so the compiler can mark the complete graph boundary once, at finish.

### Planner and scheduler boundary

The graph emitted here is placement-free. `CalculationNode` contains a
`PrimitiveKernel` and its alias contract, while `KernelIterationDomain` says
when that kernel is eligible. There are no device IDs, queue IDs, routes,
allocations, transfers, measured costs, or native artifact identities in
`GraphCompiler`.

Later, `training/src/execute.rs` calls
`recipe_prepare::Preparer::prepare_program(training.program(), profile)`. The
preparer and its planner/scheduler dependencies consume the graph plus a
measured profile, select legal primitive lowerings, assign devices and queues,
derive dependencies and synchronization, allocate immutable images, and
produce a `FinalizedBundle`. The executor then realizes native kernels and
admits external inputs in init. Compile-time materialization only reserves
logical value/kernel identity namespaces so those later passes can preserve
the graph contract; it does not probe hardware or choose a backend.

## Finalization and returned state

`GraphCompiler::finish` performs the final boundary and program construction:

1. Mark every tensor external input iff its ID is in `external_input_ids`, and
   external output iff its ID is in `external_outputs`.
2. Translate metric bindings to `MetricEmission` values.
3. Build `CalculationGraph { tensors, nodes }` and call `graph.validate()`.
   This checks tensor contracts, unique producers and kernels, explicit
   boundaries, aliases, and topological dependencies.
4. Serialize to canonical OGDL and parse back. This proves the graph's encoded
   form is the same graph accepted by the language codec.
5. Construct `StaticCalculationProgram::new_with_metrics` with total loop
   iterations, every kernel's domain, and metric emissions. Serialize and parse
   the program OGDL once more.
6. Return `CompiledTraining` containing the program, owned external inputs,
   `TrainingBounds`, `TrainingOutputs`, dataset schema, cloned config, effective
   blocks, legacy layer view, and optional output adapter.

The resulting output state includes:

* normalization values (`ZScoreState` mean/variance or `MinMaxState`
  min/max), marked external when they are semantically reusable;
* `DenseBlockState` parameter images and geometry, including every
  `ParameterState` initial/updated parameter and AdamW moment pair;
* updated K-means centroids, tree split arrays, and optional temperature;
* training loss and optimizer progress; validation outputs and status; and
  metric bindings.

`TrainingOutputs::visit_parameter_states` walks the resulting block state in
the same semantic order used by checkpoint manifests and native output
mapping. It visits all value/moment pairs, including ordered PReLU scalars,
but skips pool and K-means operation state. This is why a compiled graph can be
resumed without rebuilding or reordering its native parameter images.

The compiler itself writes no `.ogdl`, `.cubin`, or `.hsaco` file. Checkpoint
and artifact code later serializes the semantic graph/state or realizes a
native kernel. `checkpoint::apply_checkpoint_resume` validates compatibility,
replaces bytes for every `ResumeParameter`, moment, K-means, and tree-split
external input, and changes the one `ResumeEnabled` byte from zero to one. It
does not alter graph identities, domains, or native artifact identity. If no
checkpoint is admitted, all resume images remain zero and the statically
selected fresh initialization path is used.

## Runtime lifecycle contract

`execute.rs` receives `CompiledTraining`, calls the profile-aware preparer on
its `StaticCalculationProgram`, rejects any loop external transfer, packs the
owned inputs into one init image per finalized device, and starts the native
`init -> loop -> exit` lifecycle. The loop consumes the complete prepared
training partition once per iteration and never accepts data/file ingress or
egress. External outputs are read only after exit. Metric values are four-byte
loop-internal readback slots emitted on their declared domains; they are not a
third model-work kind and are not external outputs.

For finite training, the program's exact nonzero iteration count is the
terminal boundary. For unbounded training, the program has no invented terminal
epoch: `execute.rs` requires a host stop source and accepts a graceful stop
only after a completed loop iteration. Dynamic loop shortening is intentionally
not implemented in this compiler; see `REMAINING_UNSUPPORTED` in `model.rs`.

## Scalar-program helper groups

The helpers at the end of `compile.rs` are the single source of the scalar
formulas used by the graph. They are not alternate execution paths:

* resume selection, I32-to-f32 conversion, constants, zero images, validity,
  and group-routing masks;
* tree target one-hot conversion, complete-tree relative/candidate/split/leaf
  indices, finite checks, LightGBM candidate selection and next-node routing,
  safe means, left weights, and variance gain;
* attention sequence/head source indices and recurrent GRU/LSTM forward and
  backward equations;
* masked zero, pointwise MSE/MAE/Huber, stable categorical cross entropy,
  signed/natural logarithm derivatives, tangent and normalization derivatives;
* R2, safe counts, scalar means, inverse standard deviation, masked means, and
  global clip scale;
* finite and unbounded schedule inputs, exact nonnegative I32 ratios, cosine
  and exponential curves, base learning-rate gating, accepted-update and
  saturating counters;
* Adam beta initialization/update, the full AdamW update equation, and bounded
  temperature gradient/update formulas.

Every helper uses `ScalarProgramBuilder` and returns a typed
`TrainingCompileResult<ScalarProgram>`. Checked extents and invalid domains
return a specific `TrainingCompileError`; scalar `Require` nodes leave invalid
payloads visible to the device fault channel rather than adding a host-side
fallback.

## Error and invariant map

All failures are `TrainingCompileError` values. The public kinds and their
typical sources are:

| Kind | Compiler sources |
| --- | --- |
| `EmptyDataset` | zero training rows in `training_bounds` |
| `InconsistentRows` | prepared matrix/observation count or row storage mismatch |
| `InvalidFeatureMatrix` | feature semantics, encoding, normalization, shape, token, or f32 exactness checks |
| `InvalidTargetMatrix` | target order, dtype, category, missing/unseen, binary/one-hot, validation, or class checks |
| `InvalidNetwork` | block order, logical shape, output width/logits, state pairing, alias traversal, or required tape absence |
| `InvalidOptimizer` | horizon, warmup, decay, reduction lanes, hyperparameter, accepted-update, or calibration checks |
| `UnsupportedExtent` | u64/usize/I32 conversion limits and resource dimensions |
| `ArithmeticOverflow` | checked products, sums, byte counts, identity reservations, and schedule bounds |
| `IdentityExhausted` | value, kernel, random stream, or metric ID counters cannot advance |
| `Ingest` | conversion of prepared dataset/materialized input bytes |
| `Language` | tensor, primitive, graph, scalar-program, or graph-fragment validation |
| `Operation` | operation registry/materialization requirements and workspace/resource contracts |
| `Program` | static program domain, metric, or lifecycle validation |
| `Ogdl` | canonical graph/program encode or decode round-trip |

The compiler relies on the active graph state rather than defensive event
handlers. It registers exactly the domains and metrics that the requested
training state can produce. Every index map, gather, scatter, contraction,
reduction, random map, materialized composition, and scalar program is checked
at construction. No mock, proxy, synthetic test path, duplicate backend, or
runtime fallback is introduced here.

## Source map

The implementation is intentionally large but organized into these concrete
regions (line numbers refer to the current source):

* 54-632: constants, forward/backward tape structs, logical shape, parameter
  roles, validation state, and public wrapper declarations.
* 715-1388: `compile_dense_training_impl`, dataset admission, normalization,
  supervision, forward/loss/backward/optimizer orchestration, validation
  dispatch, metric binding, and finish call.
* 1390-1715: recurrent/target/embedding/lowered-matrix validation.
* 1741-1964: topology/config/horizon/optimizer validation.
* 1966-2256: effective logical shape and ordered parameter-gradient flattening.
* 2606-3737: `GraphCompiler` state, external inputs, normalization, activation
  and loss primitives, and normalization backward.
* 3760-5300: ordered training-block lowering, supervised tree construction,
  prediction materialization, and leaf routing.
* 5300-6938: embedding, attention, recurrent, dense, convolution, K-means,
  pool, residual operations, packing, index transforms, routing, and parameter
  initialization/resume selectors.
* 6944-8379: validation block replay, binary/multiclass/regression metric
  graphs, metric materialization, and temperature scaling.
* 8381-10102: reverse-mode block, recurrent, attention, tree, convolution,
  K-means, pool, layer, and operation gradients.
* 10113-10753: state updates, global clipping, dynamic schedule scalars, and
  AdamW.
* 10754-11263: primitive emission, materialized graph insertion, tensor checks,
  identity allocation, final program construction, and metric bindings.
* 11272-12528: all scalar programs and exact alias-contract helpers.

The paired module `training/src/forward.rs` owns reusable recurrent equations
and activation classification. `training/src/model.rs` owns public dense
declarations, dataset lowering, persistable state, and `CompiledTraining`.
`recipe_language` owns tensor/primitive/graph contracts, `recipe_ops` owns
operation descriptors and bounded materializers, and `recipe_program` owns
iteration domains and metric validation. `compile.rs` is the single place that
connects those contracts into a complete dense training graph.
