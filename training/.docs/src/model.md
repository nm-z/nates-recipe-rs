# Training model and state

This page documents the model boundary in `training/src/model.rs` and the
representations that are paired with it by the training compiler.  The public
model declaration is not itself executable state.  Compilation resolves that
declaration against the prepared row-free schema and creates a static graph,
typed external images, loop bounds, and a parallel state tape.  Checkpoint code
then turns the state tape into a row-free semantic model, while inference turns
the same saved declarations and final parameter images back into a target-free
graph.

The authoritative implementation is [training/src/model.rs](../../src/model.rs).
The lowering and lifecycle code is in [compile.rs](../../src/compile.rs),
[checkpoint.rs](../../src/checkpoint.rs), [execute.rs](../../src/execute.rs),
and [inference.rs](../../src/inference.rs).  The facade maps the public
`Model`, `Data`, and `Train` declarations in [src/training.rs](../../../src/training.rs).

## The paired representations

One dense run has five related but deliberately different representations:

| Representation | Owner | What it contains | What it does not contain |
| --- | --- | --- | --- |
| Declaration | `DenseBlock`, `DenseLayer`, `DenseTask`, `DenseTrainingConfig` | User topology, operation order, objective, normalization policy, and schedule policy | Tensor identities, fitted rows, device placement, or executable kernels |
| Lowered dataset | `DenseFeaturePlan`, `CompiledDatasetSchema`, `DensePartition`, `LoweredDenseDataset` | Source vector schema, feature spans, fixed-width matrices, target observations, and the original validation split size | Native buffers or learned parameters |
| Compiled graph state | `CompiledTraining`, `TrainingOutputs`, `DenseBlockState`, `ParameterState` | Recipe value IDs, kernel IDs, resolved geometry, loop domains, external input images, moments, and metric bindings | Host rows after execution and native handles |
| Semantic checkpoint | `CheckpointManifest` during execution, `CheckpointArtifact` after decoding | Row-free schema, visible topology, final parameter and moment images, normalization tensors, deterministic K-means/tree state, and native identity metadata | Dataset rows, executable artifacts, device allocations, queues, or native handles |
| Target-free inference | `PreparedInference`, `CompiledInference` | Saved feature schema, query rows, saved normalization, final parameters, and prediction contract | Training targets, optimizer moments, training schedule, or a training update |

The end-to-end data flow is:

```text
public Model/Data/Train declarations
        -> prepared dataset and Dense* declarations
        -> DenseFeaturePlan + LoweredDenseDataset + DenseTask
        -> static CalculationGraph + external input images
        -> DenseBlockState/TrainingOutputs + TrainingBounds
        -> native init -> complete full-partition loop -> exit
        -> CompletedTrainingExecution + CheckpointManifest
        -> CompletedTrainingCheckpoint (.ogdl, optional native file)
        -> decode + schema-bound query preparation
        -> target-free static inference graph
```

Every branch in this flow uses one representation as the source of truth for
the next boundary.  Compilation does not infer missing model blocks, merge
target columns, or silently replace a declaration with a compatible shape.

## Scalar semantic vocabulary

`MAXIMUM_REDUCTION_TREE_LANES` is the grammar ceiling, `1024`.  Physical
lowering selects a realizable power-of-two width no greater than this ceiling
for the exact tensor and measured device.  It is not a runtime performance
tuning switch.

`DenseActivation` is the complete activation identity serialized in a model:

* `Linear` is the identity operation.
* `Cosine`, `Exponential`, `Tangent`, `Relu`, `Sigmoid`, `Tanh`, `Gelu`,
  `Silu`, and `Elu` lower to Recipe-owned scalar operations.
* `Logarithm` is the signed logarithm `sign(x) * ln(abs(x))` on finite,
  nonzero input.  Its token is `signed-logarithm`.
* `NaturalLogarithm` is ordinary `ln(x)` on finite, strictly positive input.
  Its token is `natural-logarithm`.
* `LegacySignedLogOnePlus` decodes the historical `logarithm` token and keeps
  the old `sign(x) * ln(1 + abs(x))` behavior.  New facade declarations never
  create this variant.
* `Huber` is the Recipe activation, distinct from the Huber loss.
* `LeakyRelu`, `Selu`, and `Elu` use the canonical Recipe programs.
* `PRelu` owns a learned scalar parameter for each ordered occurrence.

`DenseActivation::token` and `DenseActivation::from_token` are crate-private
codec helpers.  The token mapping is one-to-one, including the legacy spelling.
`DenseNormalization` has `Layer` and `Batch`; `DenseOperation` wraps either an
activation or one of these normalizations.  `DenseOperation::token` and
`from_token` use activation tokens plus `layer-normalization` and
`batch-normalization`.

`DenseBlockKind` distinguishes a public `.layer(...)` (`Layer`) from a public
`.perc(...)` (`Perc`).  The kind is declaration metadata used by checkpoint and
facade translation; both use the same dense calculation path.

`DenseDataNormalization` selects the input calculation: `Identity`, `ZScore`,
`MinMax`, or `L2Norm`.  Identity preserves the prepared dtype and values and is
the only legal policy for exact token IDs entering an embedding.  The other
three are graph calculations, not host-side rewrites.  `DenseLoss` is one of
`BinaryCrossEntropy`, `Focal`, `MeanSquaredError`, `MeanAbsoluteError`,
`CrossEntropy`, or `Huber`.  `LearningRateDecay` is `Constant`, `Linear`,
`Cosine`, or `Exponential`.

`TrainingHorizon` is either `Finite(NonZeroU64)` or `Unbounded`.  Its
constructors are `finite` and `unbounded`; `bound` exposes the optional finite
bound, `is_unbounded` tests the variant, and `loop_iterations` maps it to
`LoopIterations`.  `From<NonZeroU64>` creates a finite horizon and `Display`
prints the count or `unbounded`.  An unbounded graph has no numeric sentinel;
execution requires a graceful-stop source and observes it only after a
completed epoch.

## Model declarations

All declaration widths, counts, dimensions, and depths are nonzero by type.
The constructors do not perform full network validation.  `validate_config`,
logical shape resolution, and dataset validation in `compile.rs` perform the
cross-field checks before graph construction.

### Dense layers and operations

`DenseLayer` stores private `kind`, `width`, and ordered `operations`.  The
constructors are:

* `new(width, activation)` creates a `Layer` and omits a `Linear` activation,
  otherwise retaining one activation operation.
* `with_operations(width, operations)` creates a `Layer` with exactly the
  supplied operation sequence, including an explicit `Linear` if supplied.
* `with_kind(kind, width, operations)` is the general constructor used by the
  facade and checkpoint decoder.

`kind`, `width`, and `operations` are the read accessors.  Operation order is
semantic: forward, backward, validation, checkpoint, resume, and inference
all consume the same order.  A final classification block must have only
linear output operations, so it emits raw logits.  Regression retains every
declared final operation.

`DenseConvolution` stores `filters`, `kernel`, and ordered `operations`.
`new` has the same linear-operation elision as `DenseLayer::new`, while
`with_operations` preserves the supplied sequence.  `filters`, `kernel`, and
`operations` are accessors.  The compiler resolves a stride-one geometry from
the preceding logical shape.  For input `[rows, input_length, input_channels]`
the weight is `[kernel, input_channels, filters]`, the bias is `[filters]`, and
the output is `[rows, input_length - kernel + 1, filters]`.  A kernel wider than
the input is rejected.

`DenseConvolutionGeometry` is the resolved pair of declaration and shape.  Its
fields are `input_length`, `input_channels`, `output_length`, `filters`, and
`kernel`; `new` is the constructor and each field has an accessor.  `input_width`
computes `input_length * input_channels` and `output_width` computes
`output_length * filters`, both with checked multiplication and `Option` for
overflow.

### Structured blocks

The `DenseBlock` enum retains topology instead of flattening every block into
ordinary layers.  Its variants and paired declaration fields are:

| Variant | Declaration | Resolved behavior and state |
| --- | --- | --- |
| `Embedding(DenseEmbedding)` | `dimensions`, `vocabulary` | Leading exact-int32 token sequence.  Output is `[rows, sequence, dimensions]` and the learned table is `[vocabulary, dimensions]`. |
| `Attention(DenseAttention)` | `heads` | Exactly one optional block immediately after the embedding.  `heads` divides embedding dimensions; Q, K, V, and output are bias-free `[dimensions, dimensions]` matrices and the mask is causal. |
| `Rnn(DenseRnn)` | `width` | Leading numeric scalar sequence.  Zero hidden state, `tanh` recurrence, final hidden state only. |
| `Gru(DenseGru)` | `width` | Leading numeric scalar sequence.  Reset-before GRU, zero hidden state, final hidden state only. |
| `Lstm(DenseLstm)` | `width` | Leading numeric scalar sequence.  Zero hidden and cell states, four gates, final hidden state only. |
| `Layer(DenseLayer)` | `kind`, `width`, `operations` | Ordinary affine layer followed by its ordered operations. |
| `Convolution(DenseConvolution)` | `filters`, `kernel`, `operations` | Channelwise one-dimensional convolution with resolved geometry. |
| `Pool(DensePool)` | `size`, optional `group_to_neuron` | Channelwise non-overlapping max pool.  It owns no learned parameter. |
| `KMeans(DenseKMeans)` | `clusters`, optional `group_to_neuron` | One L2 distance per centroid, one deterministic Lloyd transition per training epoch.  Centroids are loop state, not AdamW parameters. |
| `Tree(DenseTree)` | `family`, `trees`, `depth` | Terminal supervised tree ensemble.  Split structure is fixed after `init`; leaf values are learned. |
| `Residual(DenseResidual)` | ordered branch and post-merge operations | Branch output is merged with an identity skip or a learned weight-only projection, then post-merge operations run. |

`DenseEmbedding::new(dimensions, vocabulary)` and its two accessors define one
fixed token table.  `DenseAttention::new(heads)` and `heads` define the head
count.  `DenseRnn`, `DenseGru`, and `DenseLstm` each have `new(width)` and a
`width` accessor.  Their sequence length is resolved from the number of
prepared feature columns, not stored in the declaration.

`DensePool::new(size, group_to_neuron)`, `size`, `group_to_neuron`, and
`routing(groups)` preserve a requested following dense width.  `DenseKMeans`
has the analogous `new`, `clusters`, `group_to_neuron`, and `routing()` API.
`DenseTreeFamily` is `LightGbm`, `CatBoost`, or `XGBoost`; the family is retained
so checkpoint and inference cannot collapse the three construction rules.
`DenseTree::new(family, trees, depth)` and its three accessors describe exactly
the declared tree count and depth.  A standalone booster has one tree; a
`.forest(trees)` declaration supplies exactly `trees` trees, with no hidden
boosting rounds or averaging.

`DenseGroupToNeuronRouting::resolve(groups, neurons)` returns:

* `Identity { width }` when group and neuron counts are equal;
* `Expand { groups, neurons, neurons_per_group }` when neurons are divisible
  by groups;
* `Contract { groups, neurons, groups_per_neuron }` when groups are divisible
  by neurons; or
* `FullyConnected { groups, neurons }` otherwise.

The compiler turns the selected route into a mask over a dense weight.  The
same route is used in training, validation, checkpoint compatibility, resume,
and inference.  `DensePoolGroupOrder::GroupMajorChannelMinor` is the stable
flattening order for `[rows, groups, channels]`; the only winner contract,
`DensePoolWinnerContract::LowestLogicalIndex`, is shared by max-pool forward
and backward.

`DensePoolState::new(input_length, channels, output_length)` records the
resolved logical shape and sets those two contracts.  Its accessors expose all
five fields.  `input_width`, `output_width`, `logical_input_shape(rows)`, and
`logical_output_shape(rows)` use checked products and explicit three-dimensional
shapes.

`DenseResidualOperation` is either `Layer(DenseLayer)` or
`Operation(DenseOperation)`.  `From<DenseLayer>` and `From<DenseOperation>`
support mixed iterators.  `DenseResidual::new(branch, operations)` collects the
ordered branch and post-merge operation lists.  `branch` and `operations`
return slices.  `output_width` scans backward for the last branch layer and
returns `None` when the branch has no width-producing layer.  A residual block
with no such layer is rejected.  If input and branch output widths differ, the
compiler allocates a learned weight-only projection; otherwise the skip is
identity.

`DenseBlock::output_width` returns a direct width only for RNN, GRU, LSTM,
ordinary layer, K-means, and residual blocks.  Embedding, attention,
convolution, pool, and tree output widths require logical shape or task
resolution and therefore return `None`.  `output_operations` returns the
declared operations only for layer, convolution, and residual blocks; all
shape/reduction blocks return an empty slice.  `From` implementations wrap the
embedding, attention, RNN, GRU, LSTM, layer, pool, K-means, convolution, and
tree declarations in their corresponding variants; the facade constructs a
residual variant explicitly because residual construction also validates its
retained output width.

The structured forward contracts are fixed by these declarations.  Embedding
features are one exact int32 token per sequence position, each row is gathered
from the learned table, and the flattened embedding width is
`sequence_length * dimensions`.  Attention computes Q, K, and V with the four
saved square matrices, scores `Q * K^T / sqrt(head_dimension)`, masks keys
after the current query position, applies stable row/head softmax, concatenates
heads, and applies the output matrix.  A vanilla RNN starts each row at zero
and applies `h_t = tanh(x_t W_x + h_(t-1) W_h + b)`.  The reset-before GRU
applies `r_t = sigmoid(x_t W_xr + h_(t-1) W_hr + b_r)`,
`z_t = sigmoid(x_t W_xz + h_(t-1) W_hz + b_z)`,
`n_t = tanh(x_t W_xn + (r_t * h_(t-1)) W_hn + b_n)`, and
`h_t = (1 - z_t) * n_t + z_t * h_(t-1)`.  The zero-cell LSTM applies four
input/recurrent gates, `c_t = f_t * c_(t-1) + i_t * g_t`, and
`h_t = o_t * tanh(c_t)`, with both initial states zero.  All recurrent blocks
return only the final hidden state and share parameters across every row and
time step.

Pooling partitions only the logical length axis into consecutive groups,
including a bounded final short group, preserves channels, and uses the
lowest logical index on equal maxima.  K-means initializes fresh centroids from
training rows in deterministic row order, chooses the lowest centroid index on
distance ties, retains an empty centroid, and emits updated-centroid distances
after one Lloyd transition per epoch.  A tree or forest is terminal; its family,
tree count, depth, complete split tensors, and leaf value state remain explicit
in the paired checkpoint representation.

### Task and output contracts

`DenseTask` is resolved from target schema and loss, never from a host cast:

| Task | Fields | Target matrix and output |
| --- | --- | --- |
| `BinaryClassification` | `target_vector`, `positive_code` | One target column, one raw-logit output.  `positive_code` is `1` for numeric targets, or the fitted categorical binding (`-1`, `0`, or `1`). |
| `MulticlassClassification` | `target_vector`, `class_count`, `reserved_code` | Int32 dictionary codes, with one reserved unseen-label class.  Output width is dictionary width plus one. |
| `ScalarRegression` | `target_vector` | One numeric target and one output. |
| `MultiTargetBinaryClassification` | `first_target_vector`, `target_count` | Ordered numeric `[rows, k]` matrix, exact binary cells, independent BCE or focal coordinates. |
| `JointMulticlassClassification` | `first_target_vector`, `target_count` | Ordered one-hot `[rows, k]` matrix, row-wise coupled cross entropy. |
| `MultiTargetRegression` | `first_target_vector`, `target_count` | Ordered numeric `[rows, k]` matrix and output. |

`target_vector` returns the singular target or the first declared target,
`target_count` returns one or the matrix width, `uses_target_matrix` identifies
the three multi-target variants, and `output_width` returns the loss output
width.  `CompiledDatasetSchema::from_prepared` requires the prepared target
source list to have exactly this count and first identity.

`DenseOutputAdapter` stores private `source_width` and `target_width`.  Its
crate-private `new` constructor and two accessors exist for decoding explicit
legacy checkpoint adapters.  New compilation requires the final declared
effective width to equal the task width and does not invent an adapter.

## Dataset representations and lowering

`DenseFeatureLowering` is either `NumericScalar` or
`CategoricalOneHot { dictionary_width, reserved_index }`.  A
`CompiledFeatureSpan` records the source vector identity, flattened `start`,
lowered `width`, and lowering kind.  Its crate-private `new` constructor and
four accessors are used by both checkpoint and inference schema code.

`DenseFeaturePlan::from_prepared` walks feature vectors in prepared order:

* numeric `I32` or `F32` vectors with no metadata lower to one scalar and are
  marked for normalization;
* categorical dictionary `I32` vectors validate nonempty, unique labels and
  lower to `dictionary.len() + 1` one-hot columns, where the last column is the
  reserved route for missing or unseen values and is not normalized;
* every span start and width is checked for `usize` overflow; no feature vector
  is accepted without a dedicated semantic lowering; and
* at least one feature vector is required.  A normalization mask is retained
  only when at least one categorical feature exists.  Its bits are `1.0` for
  numeric columns and `0.0` for categorical one-hot columns so model
  normalization can leave categorical coordinates unchanged.

`lower_dense_features` uses the plan for each train or validation partition.
With no categorical feature it delegates to the prepared fixed dense matrix.
With categorical features it emits a binary32 row-major matrix, preserving
feature and row order.  Numeric missing values, variable-width values,
non-exact int32-to-f32 conversions, stale source identities, negative category
codes, and codes past the dictionary plus reserved route are typed
`InvalidFeatureMatrix` failures.  Categorical missing values select the
reserved index.

`CompiledDatasetSchema` is the row-free contract captured in a compiled model:
`vectors` are the complete prepared `VectorSchema` list, `features` are the
spans, `targets` are source identities in declaration order,
`target_dtypes` are the fixed source dtypes in that same order,
`input_width` is the sum of span widths, and `task` is the resolved
`DenseTask`.  `vectors`, `features`, `target`, `targets`, `target_dtype`,
`target_dtypes`, `input_width`, `task`, and `output_width` are accessors.
`target` and `target_dtype` are compatibility accessors for singular targets;
the slices are authoritative for multi-target models.  `decode_multiclass_class`
returns `Label(&[u8])`, `ReservedUnseen`, or `None` for non-multiclass,
out-of-range, or missing dictionary state.

`DensePartition` pairs a `DenseMatrix` of features with a target matrix and one
`TargetObservation` per row.  `new` validates matrix storage, dimensions, and
observation count.  `features`, `targets`, `rows`, and `feature_columns` are
accessors.  `all_targets_known` tests whether every row is supervised;
`accepted_updates_per_epoch` is `1` when at least one row is known and `0`
otherwise; and `target_supervision` emits a binary32 `[rows, 1]` mask with one
for `Known` and zero for `Missing` or `Unseen`.

`TargetObservation` is private and has `Known`, `Missing`, and `Unseen`:

* Missing means the source target value was absent.
* Unseen means a categorical code equals the explicit reserved route.
* Both are represented by zero target data plus a zero supervision mask.

`known_only` is used for validation.  It preserves the whole partition when
all rows are known, otherwise compacts feature and target rows that are
`Known`, returns `None` when no known row remains, and rebuilds an all-known
observation vector.  It never compacts the training partition.

`LoweredDenseDataset` stores the nonempty training partition, optional known-
only validation partition, and `validation_split_rows`, the original prepared
validation row count before unknown rows were removed.  `new` requires equal
feature-column counts.  `from_prepared` lowers train and validation features
and targets, validates each partition, applies `known_only` to validation, and
retains the original split count.  `train`, `validation`, and
`validation_split_rows` are accessors.

Target lowering is task-specific.  Singular int32 targets preserve int32
storage until the loss requires f32 conversion; singular f32 targets must be
finite, and multiclass targets must retain dictionary int32 codes.  Numeric
binary values are exactly `0` or `1`.  Categorical binary values bind the
positive code to dictionary size (`-1` for an empty dictionary, `0` for one
label, and `1` for two labels), map known categories to zero/one, and mark the
reserved code unseen.  Multiclass class count must contain exactly one
reserved route and rejects negative or greater-than-reserved codes.

Multi-target lowering requires at least two declared targets, exact prepared
target order, homogeneous numeric fixed-width sources, finite values, and
exact int32-to-f32 conversion.  A row with any missing coordinate is entirely
unsupervised and its lowered row is zeroed.  Binary matrices accept only exact
zero and one; joint cross-entropy rows require exactly one `1.0` and all other
coordinates `0.0`; multi-target regression has no extra range restriction.

`validate_partition` rejects empty rows, zero feature columns, mismatched row
counts, and zero target columns.  `validate_matrix_storage` checks checked
`rows * columns`, exact backing-vector length, and finite f32 bits.  The
crate-private `validate_binary_targets` applies the exact zero/one check to
either int32 or f32 storage.

## Configuration, validation requests, and bounds

`AdamWConfig` exposes `learning_rate`, `beta_one`, `beta_two`, `epsilon`, and
`weight_decay`.  `Default` is learning rate `1e-4`, beta one `0.9`, beta two
`0.999`, epsilon `1e-8`, and weight decay `0.01`.  `validate_config` requires
finite positive learning rate and epsilon, finite beta values in `[0, 1)`, a
finite nonnegative weight decay, and a positive finite optional gradient clip
norm.  These are semantic configuration values, not graph-local constants.

`DenseTrainingConfig` contains:

* legacy flat `layers` (used only by flat entrypoints; structured compilation
  uses the explicit `DenseBlock` slice);
* `loss` and `data_normalization`;
* `epochs` and `warmup_epochs`;
* `learning_rate_decay`;
* optional `gradient_clip_norm`;
* `normalization_epsilon`;
* `reduction_tree_lanes`, bounded to a power of two in `1..=1024`;
* `random_seed`; and
* `adamw`.

The facade constructs this config after requiring a supported objective and
policy.  A non-embedding model must declare a numeric normalization policy; a
leading embedding must use identity.  Warmup cannot exceed finite epochs, an
unbounded phase permits only constant decay, and unbounded warmup is bounded by
the int32 schedule domain.  The config, except its legacy `layers` view, is
copied into `CompiledTraining` and the semantic checkpoint.

`BinaryValidationConfig` stores nonzero `calibration_bins`, recall thresholds
as exact f32 bit patterns, and optional `TemperatureScalingConfig`.  `new`
collects the threshold bits, `with_temperature_scaling` sets the optional
configuration, and the three accessors expose bins, decoded thresholds, and
temperature configuration.  Thresholds must be finite, distinct bit patterns,
and in `[0, 1]`.  `MulticlassValidationConfig` and
`RegressionValidationConfig` are zero-field request markers.  The former
requests epoch-bound cross entropy and top-one accuracy; the latter requests
R2 over the known validation partition.  Validation families are mutually
exclusive for one run.

`TemperatureScalingConfig` has nonzero `iterations`, positive finite
`learning_rate`, and finite strictly increasing positive
`minimum_temperature`/`maximum_temperature`; its default is 64 iterations,
learning rate `0.01`, and bounds `0.05..20.0`.

`TrainingBounds` records `train_rows`, the chosen `epochs`, physical
`training_iterations`, post-training `calibration_iterations`, total
`iterations`, and `warmup_iterations`.  `training_bounds` rejects empty
training data, finite epoch counts above the int32 schedule domain, calibration
on an unbounded horizon, row counts above the int32 `IndexMap` domain, and
checked arithmetic overflow.

## Compiled graph state

### External input boundary

`OwnedExternalInput` owns one immutable graph input image: `role`, `value`,
`dtype`, `shape`, and little-endian `bytes`.  `new` is crate-private and
`replace_bytes` is used only by resume admission.  Public accessors expose all
five fields.  `build_training_device_images` in `execute.rs` packs these
inputs into one init image per finalized device.  No training input is
admitted through the loop.

`ExternalInputRole` identifies every input image without relying on vector
position alone:

* `TrainFeatures`, `TrainTargets`, and optional `TrainTargetSupervision` are
  the complete prepared training matrices;
* `ValidationFeatures`, `ValidationTargets`, and `FeatureNormalizationMask`
  serve validation and categorical normalization;
* `ResumeEnabled` is one int32 scalar initially zero;
* each `ResumeParameter`, `ResumeFirstMoment`, and `ResumeSecondMoment` carries
  one parameter ordinal;
* `ResumeKMeansCentroids`, `ResumeTreeSplitFeatures`, and
  `ResumeTreeSplitThresholds` carry non-Adam loop/static state by block;
* `TrainingPool*` and `ValidationPool*` roles carry checked pool windows,
  winner bases, and gradient indices;
* `TrainingConvolution*` roles carry checked convolution windows and input
  gradient validity; and
* `ResumeTreeLeafValues` is reserved as the role name for tree leaf admission
  in the role vocabulary, while ordinary leaf parameters use the canonical
  parameter ordinal tape in the current compiler.

The graph compiler creates exactly one `ResumeEnabled` input.  Each learned
parameter receives three zero-filled resume inputs before its fresh initializer
is selected.  With the default zero flag, fresh deterministic initialization is
used.  `apply_checkpoint_resume` replaces these bytes and sets the flag to one,
so the same graph selects saved parameter and moment images without changing
value or kernel identities.

### Parameter and block state

`ParameterState` is the common learned transition record.  It contains
`initial_parameter`, `updated_parameter`, `initial_first_moment`,
`updated_first_moment`, `initial_second_moment`, `updated_second_moment`, and
the `update_kernel` identity.  The initial values are the selected fresh or
resumed images consumed by the loop; updated values are the external outputs
used by validation, checkpointing, and subsequent lifecycle phases.  Every
parameter is binary32 and every update is one AdamW transition over the full
training partition.

The declaration/state pairs are:

| Declaration | State fields and parameter order |
| --- | --- |
| `DenseLayer` | `DenseLayerState { weight, bias, prelu }`.  `prelu` contains one `ParameterState` per `.prelu()` occurrence in operation order. |
| `DenseConvolution` | `DenseConvolutionState { geometry, weight, bias, prelu }`; geometry must equal the resolved declaration shape. |
| `DenseEmbedding` | `DenseEmbeddingState { sequence_length, dimensions, vocabulary, table }`; table is the shared learned token table. |
| `DenseAttention` | `DenseAttentionState { sequence_length, dimensions, heads, head_dimension, query, key, value, output }`.  Head dimension must satisfy `heads * head_dimension == dimensions`. |
| `DenseRnn` | `DenseRnnState { sequence_length, width, input_weight, recurrent_weight, bias }`. |
| `DenseGru` | `DenseGruState { sequence_length, width, reset_input_weight, reset_recurrent_weight, reset_bias, update_input_weight, update_recurrent_weight, update_bias, candidate_input_weight, candidate_recurrent_weight, candidate_bias }`. |
| `DenseLstm` | `DenseLstmState { sequence_length, width, input_gate_input_weight, input_gate_recurrent_weight, input_gate_bias, forget_gate_input_weight, forget_gate_recurrent_weight, forget_gate_bias, output_gate_input_weight, output_gate_recurrent_weight, output_gate_bias, candidate_input_weight, candidate_recurrent_weight, candidate_bias }`. |
| `DensePool` | `DensePoolState`; geometry and winner/flattening contracts only, no parameter or moments. |
| `DenseKMeans` | `DenseKMeansState { input_width, clusters, initial_centroids, updated_centroids, update_kernel }`; centroid transition is loop-carried but not an AdamW parameter. |
| `DenseTree` | `DenseTreeState { declaration, input_width, output_width, internal_nodes_per_tree, leaves_per_tree, split_features, split_thresholds, leaf_values }`; split tensors are fixed after init, leaf values use `ParameterState`. |
| `DenseResidual` | `DenseResidualState { branch, branch_prelu, projection, prelu }`; branch has one `DenseLayerState` per branch layer, `branch_prelu` covers free branch PReLUs, `projection` is optional weight-only skip projection, and `prelu` covers post-merge operations. |

`DenseBlockState` is the topology-preserving enum containing each pair above.
State structs are built by compiler literals, not public constructors.  The
compiler checks declaration/state variant alignment and geometry again while
building validation and while producing a checkpoint.

`ZScoreState { mean, variance }` and `MinMaxState { minimum, maximum }` hold
normalization tensor IDs.  `DataNormalizationState` is `Identity`,
`ZScore(ZScoreState)`, `MinMax(MinMaxState)`, or `L2Norm`.  Mean and variance or
minimum and maximum are external outputs when present; L2 normalization has no
fitted tensor.

`OptimizerProgressState` is bounded graph execution state, not a public model
artifact.  It records the update gate and accepted-update recurrence IDs,
their update kernels, initial and updated Adam beta powers, accepted updates
per epoch, optional maximum accepted updates, a counter limit, and warmup
accepted updates.  A row with no known target keeps the gate closed and does not
advance moments or beta powers.  A valid full partition performs at most one
accepted update per epoch.  Resume restores parameter and moment tensors but
starts the newly declared phase's schedule and accepted-update counters from
its new `TrainingBounds`.

### Outputs and metrics

`ValidationMetricFamily` is `Binary`, `Multiclass`, or `Regression`.
`ValidationUnavailableReason` is `NoKnownTargets` or
`SingleKnownClass { known_rows }`.  `ValidationMetricStatus` is
`NotRequested`, `Available { family, known_rows }`, or
`Unavailable { family, reason, split_rows }`.  `split_rows` remains the
original prepared split count, while `known_rows` is the retained supervised
population.

`TrainingOutputs` is the graph's public output record:

* `training_loss` and `training_loss_domain` identify the loss scalar;
* `normalization` identifies fitted normalization tensors;
* optional `optimizer_progress` identifies the gated optimizer recurrence;
* `blocks` is the canonical structured state tape;
* `layers` is the legacy flat state view, populated only when all effective
  blocks are ordinary layers;
* `validation`, `multiclass_validation`, and `regression_validation` hold
  validation logits/predictions, metric values, domains, and optional
  temperature scaling;
* `validation_status` explains whether a requested metric has a supervised
  population; and
* `metric_bindings` maps public metric IDs to graph values and iteration domains.

`visit_parameter_states` is crate-private and is used to collect the set of
optimizer update kernels for execution evidence.  It traverses blocks in
declaration order and parameters in this order: embedding table; four attention
matrices; three RNN parameters; nine GRU parameters; twelve LSTM parameters;
layer weight, bias, then PReLU scalars; convolution weight, bias, then PReLU
scalars; no pool or K-means parameter; tree leaf values; and residual branch
layer states, branch PReLU states, optional projection, then post-merge PReLU
states.  For a residual block, the actual optimizer and resume tape follows the
mixed branch declaration order, interleaving a branch layer's parameters with
each free branch PReLU as encountered.  The checkpoint macro reproduces that
mixed order.  When `blocks` is empty this helper traverses the legacy `layers`
view instead.

`TrainingMetricKind` covers training loss, learning rate, validation BCE or
cross entropy, accuracy, R2, AUROC, AUPRC, Brier score, expected calibration
error, and thresholded recall.  `TrainingMetricBinding` associates a kind,
public `MetricId`, graph `ValueId`, and `IterationDomain`.  `RecallMetricOutput`
stores threshold bits plus its value and decodes the threshold with
`threshold()`.  `BinaryMetricOutputs` stores mean BCE, accuracy, AUROC, AUPRC,
Brier score, calibration error, and ordered recall outputs.
`MulticlassMetricOutputs` stores mean cross entropy and accuracy.
`RegressionMetricOutputs` stores R2.  `RegressionValidationOutputs` adds
predictions and metric domain.  `BinaryValidationOutputs` adds logits, binary
metrics, metric domain, and optional `TemperatureScalingState`.
`MulticlassValidationOutputs` adds logits, metrics, and metric domain.
`TemperatureScalingState` stores initial and updated temperature IDs, its
update kernel, and the nonzero calibration iteration count.

`CompiledTraining` joins the executable `StaticCalculationProgram`, owned
external inputs, `TrainingBounds`, `TrainingOutputs`, `CompiledDatasetSchema`,
copied config, effective `DenseBlock` and legacy layer declarations, and the
optional legacy `DenseOutputAdapter`.  `graph`, `program`, `external_inputs`,
`bounds`, `outputs`, `dataset_schema`, `config`, `layers`, `blocks`, and
`output_adapter` are its accessors.  Native preparation consumes this value;
the model and state records remain independent of devices and handles.

## Compile lifecycle and callers

The facade's `compile_training` first validates public policy, data, and model,
prepares the dataset, maps every `LayerSpec` to the declaration types above,
and builds `DenseTrainingConfig`.  Mapping failures include zero or
unrepresentable widths, missing embedding vocabulary, unsupported recurrent
operations, invalid forest booster declarations, and residual branch/output
width disagreement.  It chooses one validation family and dispatches to one of
the public compiler entrypoints:

* `compile_dense_training` and the three validation variants wrap the legacy
  `config.layers` as `DenseBlock::Layer`;
* `compile_dense_training_with_blocks` and its validation variants use the
  explicit topology; and
* every entrypoint reaches `compile_dense_training_impl`.

`compile_dense_training_impl` performs these ordered operations:

1. Resolve `DenseTask` from prepared target semantics and loss.
2. Validate block ordering, topology, output constraints, normalization policy,
   optimizer values, reduction lanes, and horizon.
3. Identify a leading embedding or recurrent block and validate its exact input
   schema.  Embedding requires exact int32 numeric features and token IDs in
   `0..vocabulary`; recurrent blocks require one numeric scalar per feature
   column and no categorical expansion.
4. Build `DenseFeaturePlan`, lower the complete train partition and optional
   validation partition, retain original split rows, and validate target dtype,
   target width, finite values, and exact conversions.
5. Resolve `effective_blocks` with `LogicalFeatureShape`.  Embedding changes
   channels, attention checks head divisibility, recurrent blocks replace the
   sequence with hidden width, convolution and pool resolve length/channels,
   K-means reduces to clusters, and residual output uses its final branch
   layer.  A tree must be the only block.  A final pool is rejected, the final
   width must equal task output width, and classification output must be raw
   logits.  No new output projection is inserted.
6. Build `CompiledDatasetSchema`, validation status, `TrainingBounds`, and an
   accepted-update plan when target rows can be missing or unseen.
7. Create graph external inputs for complete training features and targets,
   optional supervision, validation data, and a categorical normalization mask.
   Int32 matrices are converted to f32 only at the calculation boundary that
   requires it.  Input normalization emits `DataNormalizationState` and the
   fitted tensors.
8. Emit a validity mask, supervision count, normalized forward graph, loss and
   loss gradient, masked objective reduction, reverse-mode gradients, and
   optional global gradient clipping.
9. Emit dynamic learning-rate and Adam beta scalars.  `update_blocks` consumes
   the flattened gradient tape in canonical `ParameterRole` order, emits one
   AdamW transition for every learned parameter, emits the K-means Lloyd update
   and tree structure where applicable, and builds `DenseBlockState` values.
10. Compile requested validation graphs against updated parameters only,
    construct metric bindings, mark semantic state tensors as external outputs,
    and call `GraphCompiler::finish` to create `CompiledTraining`.

The graph never performs a host update in the loop.  Each epoch consumes the
complete prepared training partition as one logical matrix; physical tiling
cannot change the one-update-per-epoch semantic contract.

### Parameter initialization and update order

`initialize_weight` creates a deterministic Recipe Philox normal tensor, scales
it by `sqrt(2 / fan_in)`, creates zero first and second moments, and selects
fresh or resumed images through `ResumeEnabled`.  `initialize_zero_parameter`
creates three zero outputs, and `initialize_constant_parameter` does the same
for a scalar constant.  PReLU uses exactly `0.25`.  Every parameter receives a
unique resume ordinal and three external input roles even when no resume file
exists.

The principal initialized shapes are:

| Block | Parameter shapes |
| --- | --- |
| Embedding | table `[vocabulary, dimensions]`, `fan_in = dimensions` |
| Attention | query, key, value, output `[dimensions, dimensions]`, `fan_in = dimensions` |
| RNN | input `[1, width]`, recurrent `[width, width]`, bias `[width]` |
| GRU | three copies of the RNN input/recurrent/bias shapes, in reset, update, candidate order |
| LSTM | four copies of the RNN input/recurrent/bias shapes, in input, forget, output, candidate order |
| Dense layer | weight `[input_width, output_width]`, bias `[output_width]`, optional scalar PReLU parameters |
| Convolution | weight `[kernel, input_channels, filters]`, bias `[filters]`, optional scalar PReLU parameters |
| K-means | centroid matrix `[clusters, input_width]`, updated by Lloyd, no moments |
| Tree | leaf values `[trees, leaves_per_tree, output_width]`; split features and thresholds are fixed tensors |
| Residual | branch layers as above; optional projection `[input_width, output_width]` with no bias; post-merge PReLU scalars |

`ParameterUpdates::take` rejects a gradient tape whose role or length differs
from the expected declaration.  `adamw_update` emits updated parameter,
first-moment, and second-moment tensors plus one update kernel and marks all
three as checkpoint external outputs.  `update_blocks` then builds the state
pair for each block and rejects unused or missing gradients.  This single tape
is the link between graph calculation, execution evidence, checkpoint output
ordering, and resume ordinals.

## Checkpoint and resume lifecycle

`CheckpointManifest::from_compiled` captures the row-free dataset schemas and
feature spans, output adapter, task, config, bounds, normalization tensors,
effective declarations paired with `TrainingOutputs` state, a canonical static
program digest, and no native bytes.  It clears the legacy config-layer view
for structured blocks.  The selected semantic format version is the newest
format required by the effective topology: v9 native structured, v10 K-means,
v11 multi-target, v12 trees, v13 embedding/attention, v14 RNN, v15 GRU, and v16
LSTM.  Older v5 through v8 forms remain decode-compatible for their historical
topology contracts.

`checkpoint_blocks` requires declaration and state slices to have equal length
and matching enum variants.  It serializes declaration fields, resolved
geometry, final parameter tensors, first and second moments, K-means updated
centroids, tree split tensors, and tree leaf parameter state.  Pools serialize
shape and routing contracts but no parameter images.  Residual serialization
retains branch operation order, branch PReLU images, identity or projection
skip, and post-merge operation/PReLU order.

`CompletedTrainingCheckpoint::new` is called only after native execution has
reached exit and destroyed native resources.  It adds measured topology,
discovery, realization, target, toolchain, and digest metadata for the exact
realized native images, maps external exit images to logical checkpoint values,
and validates the manifest boundary.  `save` writes one canonical semantic
OGDL file atomically.  `save_native_kernel` writes the exact selected `.cubin`
or `.hsaco` bytes separately; native bytes are never embedded in OGDL.

`decode_checkpoint` bounds source bytes, parses one OGDL `recipe` root, checks
version and semantic format, decodes schema/config/topology/tensor images, and
performs full path-addressed validation.  Semantic versions require native
realization metadata, while legacy v5 has no native field.  The resulting
`CheckpointArtifact` exposes format version, vectors, feature spans and mask,
feature width, target source order and dtypes, task, output adapter, config,
bounds, normalization images, legacy layers, canonical blocks, optional
temperature, and optional native identities.  It also provides the same
multiclass dictionary decoder as `CompiledDatasetSchema`.

`apply_checkpoint_resume` is the only admission path for a loaded dense model:

1. Validate the decoded artifact and build a manifest from the current graph.
2. Require exact feature width/spans/mask, target source order, task,
   output-adapter, row-free vector schema, objective, normalization epsilon,
   AdamW beta/epsilon/decay, effective topology, and every parameter tensor
   dtype/shape/byte-size contract.
3. Require matching K-means centroid and tree split tensor contracts and
   matching block geometry/declarations.
4. Admit exactly one resume-enable scalar, all three images for each parameter
   ordinal, and the complete set of K-means/tree static state inputs.  Duplicate
   or missing roles, stale tensor shapes, and extra roles are incompatible
   resume errors.
5. Replace the owned input bytes and set `ResumeEnabled` to one.  The compiled
   graph, value IDs, kernel IDs, and native program digest remain unchanged.

The current declaration controls the new horizon, warmup, learning-rate decay,
and validation phase.  Saved parameter and AdamW moment images continue the
model, but schedule position, accepted-update counters, and automatic-stop
state are not semantic artifacts.  A missing `.resume` path is existence-
conditional and leaves all resume inputs at their fresh zero images.

## Inference lifecycle

`prepare_checkpoint_inference` applies only the saved feature schema to a
target-free table.  It preserves saved feature order and dictionaries, permits
source-column reordering, ignores unrelated columns, rejects missing required
features, and validates that prepared spans equal the saved spans.  It does not
read target columns, fit a new normalization, or perform host one-hot or model
calculation.  `PreparedInference` holds the decoded `CheckpointArtifact` and
the unnormalized `PreparedInferenceDataset`; its accessors expose checkpoint,
data, feature spans, saved normalization policy/tensors, mask, and epsilon.

`compile_prepared_inference` requires a nonempty canonical `blocks` list and at
least one query row.  It emits feature lowering, saved normalization, and each
saved block exactly once.  Every block/state geometry is checked again:
embedding vocabulary and sequence, attention head geometry, recurrent sequence
and width, convolution geometry, pool contracts, K-means centroid width,
tree declaration and split shape, and residual branch/skip operation order.
Only `updated_parameter` values are admitted.  Optimizer moments, training
targets, supervision masks, schedule bounds, and update kernels never enter
the inference graph.  The final width must equal the saved task width;
optional saved temperature is applied, then `compile_prediction` emits the
appropriate binary, multiclass, regression, or multi-target f32 output and its
source target dtype contract.

The native inference executor uses the same measured preparation and
`init -> loop -> exit` lifecycle as training, but the program has one loop
iteration, no training ingress/egress or metrics, and one finalized prediction
egress.  Saved tree structures and K-means centroids are traversed as fixed
state; no tree rebuild, centroid update, or optimizer transition occurs.

## Validation and error behavior

The model layer returns `TrainingCompileResult<T>`, whose typed
`TrainingCompileErrorKind` distinguishes `EmptyDataset`, `InconsistentRows`,
`InvalidFeatureMatrix`, `InvalidTargetMatrix`, `InvalidNetwork`,
`InvalidOptimizer`, `UnsupportedExtent`, `ArithmeticOverflow`,
`IdentityExhausted`, and wrapped `Ingest`, `Language`, `Operation`, `Program`,
and `Ogdl` failures.  Errors retain the direct failing boundary and do not
invent fallback topology or substitute data.

Important validation gates are:

* at least one nonempty feature vector and nonempty training partition;
* exact matrix row/column/storage lengths and finite f32 payloads;
* typed target count/order, fixed target dtypes, finite numeric values, and
  objective-specific binary, one-hot, categorical, and reserved-code rules;
* embedding first and unique, attention only immediately after embedding,
  recurrent blocks leading and unique, trees terminal and alone, pool/K-means
  routes immediately followed by the matching dense width, and residuals with
  a final branch layer;
* convolution kernel and all checked logical products;
* final output width, final classification logits, and no final pool;
* normalization/embedding compatibility, horizon/decay/warmup rules, valid
  AdamW values, reduction lanes, and validation configuration;
* declaration/state variant and geometry equality in validation, checkpoint,
  resume, and inference; and
* checkpoint source bounds, OGDL structure, tensor shapes/bytes, semantic
  version, and native identity metadata.

Requested validation with no known target rows is represented as
`Unavailable::NoKnownTargets`, not a fabricated metric.  Binary validation is
also unavailable when a target column has only one known class, represented by
`SingleKnownClass`.  Validation may be omitted only when it was not requested,
or when the status explicitly records that no supervised population exists.

`UnsupportedTrainingFeature` currently lists `DynamicLoopShortening` and
`ExactOptimizerResume` in `REMAINING_UNSUPPORTED`.  The implemented resume
path does restore model and AdamW tensor images; the unsupported item means
that the exact prior schedule/counter position is not carried as a public
training phase state.

## End-to-end role at the facade and executor

`src/training.rs` is the public caller.  It maps grammar layer operations to
`DenseOperation`, dimensions to nonzero widths, block spellings to
`DenseBlock`, target/loss combinations to `DenseTask`, and policy values to
`DenseTrainingConfig`.  `Train::run` compiles the graph, optionally admits an
existing checkpoint and authenticated native kernel, prepares the measured
native session, executes the complete loop, creates `CompletedTrainingCheckpoint`,
and performs only declared `.save(...)` exports.  The executor receives
`CompiledTraining::external_inputs`, runs the finalized native lifecycle, and
returns only post-exit external outputs and final metric samples.  The
checkpoint layer interprets those outputs by the exact `ValueId` and parameter
order documented above.

The resulting semantic `.ogdl` model is therefore a pair of row-free model
declaration plus final state images.  It is sufficient for compatible resume
and for target-free inference, while all preparation, scheduling, device
placement, native compilation, and transient execution state remain outside
the artifact.

## Source map

* Declarations, schema lowering, target observation, configuration, state
  structs, outputs, and validation helpers: `training/src/model.rs`.
* Task resolution, logical shape validation, graph construction, parameter
  initialization, AdamW updates, validation graphs, and `CompiledTraining`
  assembly: `training/src/compile.rs`.
* Public facade mapping and compile/resume/execute orchestration:
  `src/training.rs`.
* Semantic model versions, manifests, decode validation, output mapping,
  save, and resume admission: `training/src/checkpoint.rs`.
* Native init/loop/exit, external image packing, and completed execution:
  `training/src/execute.rs`.
* Saved-schema preparation and updated-parameter inference compilation:
  `training/src/inference.rs`.
* Canonical recurrent equations and activation lowering:
  `training/src/forward.rs`.
* Normative activation, objective, horizon, topology, multi-target, tree,
  embedding, attention, RNN, GRU, and LSTM contracts: `system-contract.md`
  sections C22 and C25 through C40.
