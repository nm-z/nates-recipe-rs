# `recipe-training`: crate root and end-to-end runtime boundary

`recipe-training` is the Recipe-owned compiler and native execution bridge for
static training and target-free inference programs.  It accepts a prepared,
typed dataset or a decoded semantic model, emits a
`recipe_program::StaticCalculationProgram`, and hands that program to
`recipe-prepare`, `recipe-native-executor`, and `recipe-executor`.  It does not
own a GPU context, a queue, a native handle, or a scheduler.  Those are created
by the preparation and execution crates after this crate has fixed the graph,
external-image contracts, lifecycle domains, and semantic output boundary.

The public root is deliberately a facade.  Most implementation modules are
private and are exposed only through typed re-exports.  The only public module
is `bayes`, which is also the only declaration-resolution module that callers
may address by module path.  The root therefore remains the stable surface for
the frontend (`src/training.rs` and `src/inference.rs`) while the graph builder
can share private lowering code between training, validation, checkpoint
inference, KNN inference, Bayesian inference, and the GGUF instrument.

## Crate root

`training/src/lib.rs` has two crate-wide compiler guarantees:

* `#![forbid(unsafe_code)]` keeps all unsafe work at the native backend and FFI
  boundaries owned by other crates.
* `#![deny(missing_debug_implementations)]` requires public state containers and
  error values to retain useful structural diagnostics.

The root declares these modules:

| declaration | visibility | role |
| --- | --- | --- |
| `bayes` | `pub` | Typed Bayesian schema resolution and observed categorical reference preparation. It emits no graph. |
| `bayes_checkpoint` | private | Versioned OGDL encode/decode and continuation of observed categorical Bayesian artifacts. |
| `checkpoint` | private | Dense semantic checkpoint schema, manifest construction, strict decoding, output mapping, atomic saving, native identity, and resume admission. |
| `compile` | private | Dense training graph construction, validation, loss, backward graph, optimizer, validation metrics, and static-program finalization. |
| `error` | private | Dense compile error taxonomy and result alias. |
| `execute` | private | Native preparation handoff, init image packing, lifecycle polling, metric observation, output collection, and execution evidence. |
| `forward` | private | Shared recurrent equations, activation lowering, masks, scalar programs, and alias rules used by training and inference compilers. |
| `inference` | private | Semantic model loading and schema application, dense/KNN/Bayes/GGUF target-free graph compilation, contracts, and preparation errors. |
| `knn` | private | Loss-independent KNN reference-state preparation and deterministic target reducers. |
| `knn_checkpoint` | private | Versioned OGDL KNN model artifacts and bounded decoding. |
| `model` | private | Public dense declarations, task resolution types, compiled schema, graph state, optimizer state, and metric contracts. |

`inference.rs` includes `gguf_llama.rs` as a private nested module with
`#[path = "gguf_llama.rs"]`.  Its selected artifact and compiler are re-exported
from the crate root, but the nested module is not a separate public namespace.
`forward` is entirely `pub(crate)`: its trait and helpers are implementation
boundaries, never a user graph API.

The crate depends on Recipe's typed graph and execution layers rather than
reimplementing them: `recipe-core` supplies identifiers, dtypes, domains,
plans, and topology identities; `recipe-language` supplies graph and scalar
program construction; `recipe-ops` supplies checked operation materialization;
`recipe-program` owns immutable static programs; `recipe-ingest` supplies
prepared datasets and schema-driven inference rows; `recipe-prepare`,
`recipe-probe`, `recipe-native-executor`, and `recipe-executor` own measured
placement, native realization, backend handoff, and lifecycle execution.
`recipe-ogdl` is the canonical textual model/checkpoint codec. `rustix` is used
for bounded filesystem-capacity checks and `sha2` authenticates the canonical
compiled program digest.

## Re-exported public surface

The root re-exports all supported user-facing types and boundary functions.  A
caller does not need to know which private file owns an item.

### Bayesian declarations and artifacts

From `bayes` the root exports
`BayesianCategoricalReferenceSet`, `BayesianCategoricalSchema`,
`BayesianDependency`, `BayesianNodeId`, `BayesianNodeSchema`,
`BayesianNodeSource`, `BayesianSchemaError`, `BayesianSchemaErrorKind`,
`BayesianSchemaPathSegment`, `CATEGORICAL_BAYES_SMOOTHING`,
`ResolvedBayesianDependency`, `ResolvedBayesianSchema`,
`prepare_categorical_bayesian_reference_set`,
`prepare_categorical_bayesian_reference_sets`, and `resolve_bayesian_schema`.

`bayes_checkpoint` adds `BayesModelArtifact`, `BayesModelDecodeLimits`, and
`decode_bayes_model`.  Bayesian artifacts preserve observed rows and label
dictionaries, not host-computed count tables.  The native inference operation
rebuilds histograms and the Laplace-one posterior from that state.

### Dense compilation

`compile` exports the flat and topology-preserving entrypoints:

* `compile_dense_training` lowers `DenseTrainingConfig.layers` as a flat list.
* `compile_dense_training_with_blocks` accepts the explicit `DenseBlock` graph.
* `compile_dense_training_with_blocks_and_binary_validation` adds binary
  validation, calibration metrics, and optional temperature scaling.
* `compile_dense_training_with_blocks_and_multiclass_validation` adds
  cross-entropy and top-one accuracy over the validation partition.
* `compile_dense_training_with_blocks_and_regression_validation` adds full
  partition R2 validation.
* `compile_dense_training_with_binary_validation`,
  `compile_dense_training_with_multiclass_validation`, and
  `compile_dense_training_with_regression_validation` are the corresponding
  flat-list wrappers.

All eight functions return `TrainingCompileResult<CompiledTraining>`, which is
`Result<T, TrainingCompileError>`.

### Dense compile errors

`TrainingCompileError`, `TrainingCompileErrorKind`, and
`TrainingCompileResult` are re-exported from `error`.  The stable compile
classes are `EmptyDataset`, `InconsistentRows`, `InvalidFeatureMatrix`,
`InvalidTargetMatrix`, `InvalidNetwork`, `InvalidOptimizer`,
`UnsupportedExtent`, `ArithmeticOverflow`, `IdentityExhausted`, `Ingest`,
`Language`, `Operation`, `Program`, and `Ogdl`.  The error keeps a class and a
human-readable detail.  Ingest, language, operation, program, and OGDL errors
are converted at the crate boundary; lower layers are not hidden behind a
generic string-only result.

### Native execution

`execute` exports:

`CompletedInferenceExecution`, `CompletedKnnInferenceExecution`,
`CompletedTrainingExecution`, `FinalTrainingMetric`,
`InferenceExecutionError`, `InferenceExecutionLimits`,
`InferenceExecutionResult`, `InferencePrediction`, `InferenceRunFailure`,
`KnnInferencePrediction`, `NativeKernelFormat`, `RealizedNativeKernel`,
`RealizedNativeKernelSet`, `TrainingExecutionControl`,
`TrainingExecutionError`, `TrainingExecutionEvidence`,
`TrainingExecutionLimits`, `TrainingExecutionResult`,
`TrainingMetricObserver`, `TrainingMetricObserverStats`,
`TrainingMetricSample`, `bounded_training_metric_channel`,
`build_inference_device_images`, `build_knn_inference_device_images`,
`build_training_device_images`, `prepare_and_execute_local_inference`,
`prepare_and_execute_local_knn_inference`, `prepare_and_execute_local_training`,
`prepare_and_execute_local_training_controlled`, and
`prepare_and_execute_local_training_with_observer`.

These are production boundaries.  They accept a measured profile and a
production-configured `Preparer`, and use the same local native session from
preparation through backend handoff.  They do not provide a test backend or a
host-side calculation path.

### Checkpoint surface

`checkpoint` exports the semantic model and checkpoint vocabulary:

`CheckpointArtifact`, `CheckpointArtifactMetadata`,
`CheckpointArtifactVector`, `CheckpointAttentionImage`,
`CheckpointBlockImage`, `CheckpointConvolutionImage`, `CheckpointDecodeError`,
`CheckpointDecodeErrorKind`, `CheckpointDecodeLimits`,
`CheckpointEmbeddingImage`, `CheckpointError`, `CheckpointGruImage`,
`CheckpointImageMetadata`, `CheckpointKMeansImage`,
`CheckpointLayerImage`, `CheckpointLstmImage`, `CheckpointManifest`,
`CheckpointNativeKernel`, `CheckpointNativeRealization`,
`CheckpointParameterImage`, `CheckpointPath`, `CheckpointPathSegment`,
`CheckpointPoolImage`, `CheckpointResidualBranchImage`,
`CheckpointResidualImage`, `CheckpointResidualSkipImage`,
`CheckpointResult`, `CheckpointRnnImage`, `CheckpointTensorImage`,
`CheckpointTreeImage`, `CheckpointVectorSchema`,
`CompletedTrainingCheckpoint`, `apply_checkpoint_resume`,
`compiled_training_program_digest`, and `decode_checkpoint`.

`CheckpointError` covers strict decode, manifest and resume incompatibility,
output mapping, native-image availability or ambiguity, invalid targets,
capacity reservation, and filesystem I/O.  `CheckpointDecodeError` adds a
typed class and a stable structural `CheckpointPath`; it distinguishes limit,
UTF-8, syntax, missing/duplicate/unknown field, value, and consistency
failures.

### Target-free inference

`inference` exports the dense, KNN, Bayesian, and GGUF contracts and boundary
functions:

`CompiledInference`, `CompiledKnnInference`, `GgufLlamaArtifact`,
`GgufLlamaError`, `GgufLlamaErrorKind`, `GgufLlamaResult`,
`InferenceCompileError`, `InferenceCompileErrorKind`,
`InferenceCompileResult`, `InferenceExternalInput`, `InferenceInputRole`,
`InferenceOutputContract`, `InferencePredictionKind`,
`InferencePreparationError`, `InferencePreparationResult`, `InferenceTask`,
`KnnInferenceOutputContract`, `KnnInferencePredictionKind`,
`PreparedBayesInference`, `PreparedGgufLlamaInference`,
`PreparedInference`, `PreparedKnnInference`, `SemanticModelArtifact`,
`compile_prepared_bayes_inference`,
`compile_prepared_gguf_llama_inference`, `compile_prepared_inference`,
`compile_prepared_knn_inference`, `decode_gguf_llama`,
`load_and_prepare_bayes_inference`, `load_and_prepare_checkpoint_inference`,
`load_and_prepare_knn_inference`, `load_bayes_model_file`,
`load_checkpoint_file`, `load_gguf_llama_model_file`, `load_knn_model_file`,
`load_semantic_model_file`, `prepare_bayes_inference_table`,
`prepare_checkpoint_inference`, `prepare_checkpoint_inference_table`,
`prepare_gguf_llama_inference_table`, and `prepare_knn_inference_table`.

`SemanticModelArtifact` dispatches a strict root to `Dense(CheckpointArtifact)`,
`Knn(KnnModelArtifact)`, or `Bayes(BayesModelArtifact)`.  It never tries a
second decoder after a root match fails.

### KNN

`knn` exports `KnnLabelValue`, `KnnReferenceOutput`, `KnnReferenceSet`,
`KnnReferenceValues`, and `prepare_knn_reference_set`.  `knn_checkpoint`
exports `KnnModelArtifact`, `KnnModelDecodeLimits`, and `decode_knn_model`.
KNN is a semantic reference model, not a dense optimizer loop.

### Dense model and state vocabulary

`model` exports the complete typed declaration and state set:

`AdamWConfig`, `BinaryMetricOutputs`, `BinaryValidationConfig`,
`BinaryValidationOutputs`, `CompiledDatasetSchema`, `CompiledFeatureSpan`,
`CompiledTraining`, `DataNormalizationState`, `DecodedMulticlassClass`,
`DenseActivation`, `DenseAttention`, `DenseAttentionState`, `DenseBlock`,
`DenseBlockKind`, `DenseBlockState`, `DenseConvolution`,
`DenseConvolutionGeometry`, `DenseConvolutionState`,
`DenseDataNormalization`, `DenseEmbedding`, `DenseEmbeddingState`,
`DenseFeatureLowering`, `DenseGroupToNeuronRouting`, `DenseGru`,
`DenseGruState`, `DenseKMeans`, `DenseKMeansState`, `DenseLayer`,
`DenseLayerState`, `DenseLoss`, `DenseLstm`, `DenseLstmState`,
`DenseNormalization`, `DenseOperation`, `DenseOutputAdapter`, `DensePool`,
`DensePoolGroupOrder`, `DensePoolState`, `DensePoolWinnerContract`,
`DenseResidual`, `DenseResidualOperation`, `DenseResidualState`, `DenseRnn`,
`DenseRnnState`, `DenseTask`, `DenseTrainingConfig`, `DenseTree`,
`DenseTreeFamily`, `DenseTreeState`, `ExternalInputRole`, `LearningRateDecay`,
`MAXIMUM_REDUCTION_TREE_LANES`, `MinMaxState`,
`MulticlassMetricOutputs`, `MulticlassValidationConfig`,
`MulticlassValidationOutputs`, `OptimizerProgressState`,
`OwnedExternalInput`, `ParameterState`, `REMAINING_UNSUPPORTED`,
`RecallMetricOutput`, `RegressionMetricOutputs`,
`RegressionValidationConfig`, `RegressionValidationOutputs`,
`TemperatureScalingConfig`, `TemperatureScalingState`, `TrainingBounds`,
`TrainingHorizon`, `TrainingMetricBinding`, `TrainingMetricKind`,
`TrainingOutputs`, `UnsupportedTrainingFeature`, `ValidationMetricFamily`,
`ValidationMetricStatus`, `ValidationUnavailableReason`, `ZScoreState`, and
the listed feature, convolution, recurrent, tree, pool, residual, and
normalization state types.

## Dense model declarations and compiled state

### Declarations

`DenseActivation` covers `Linear`, `Cosine`, `Exponential`, signed
`Logarithm`, positive-domain `NaturalLogarithm`, decoder-only
`LegacySignedLogOnePlus`, `Huber`, `Tangent`, `Relu`, `LeakyRelu`, `Sigmoid`,
`Tanh`, `Selu`, `Gelu`, `Silu`, `Elu`, and learned `PRelu`.  `DenseOperation`
contains an activation or `DenseNormalization::Layer` or
`DenseNormalization::Batch` operation.  `DenseLayer::new` omits a linear
operation, while `with_operations` and `with_kind` preserve the declared
operation order and `DenseBlockKind` (`Layer` or `Perc`).

Structured blocks retain shape rather than flattening it prematurely:

* `DenseEmbedding::new(dimensions, vocabulary)` consumes exact int32 token
  positions and emits `sequence_length * dimensions` f32 channels.
* `DenseAttention::new(heads)` is one causal multi-head self-attention block
  immediately after a leading embedding.  Head divisibility and full causal
  masking are checked during compilation.
* `DenseRnn`, `DenseGru`, and `DenseLstm` consume one scalar feature per fixed
  sequence position, start each row with zero state, and emit only the final
  hidden state.
* `DenseConvolution` is a valid stride-one one-dimensional convolution.  Its
  `DenseConvolutionGeometry` records input length/channels, output length,
  filters, kernel, and checked flattened widths.
* `DensePool` is channelwise, non-overlapping maximum pooling.  Its optional
  `group_to_neuron` retains the following dense width so routing cannot be
  lost.  `DensePoolGroupOrder::GroupMajorChannelMinor` and
  `DensePoolWinnerContract::LowestLogicalIndex` are explicit persisted
  contracts.
* `DenseKMeans` emits one distance per centroid and has the same optional
  divisible-group routing rule.
* `DenseResidual` stores an ordered branch of `DenseLayer` and free
  `DenseOperation` entries plus post-merge operations.  The final branch layer
  defines the output width.  The skip is identity at that width or a learned,
  weight-only projection.
* `DenseTree` records a `DenseTreeFamily` (`LightGbm`, `CatBoost`, or
  `XGBoost`), tree count, and depth.  A tree or forest is terminal and cannot
  be chained with another block.

`DenseBlock` is the topology-preserving sum of these declarations.  Its
`output_width` and `output_operations` accessors expose only shape facts that
are already resolved by the declaration.  `DenseGroupToNeuronRouting::resolve`
selects `Identity`, divisible `Expand`, divisible `Contract`, or
`FullyConnected` from two nonzero widths.  No host-side fallback chooses a
different route later.

### Tasks, config, and schema

`DenseTask` records the resolved target semantics:

* `BinaryClassification` and `ScalarRegression` have one output column.
* `MulticlassClassification` stores the dictionary class count and its
  reserved unseen-label code.
* `MultiTargetBinaryClassification` evaluates BCE or focal loss per ordered
  numeric target column.
* `JointMulticlassClassification` treats ordered numeric one-hot columns as
  one row-wise softmax target.
* `MultiTargetRegression` retains ordered numeric outputs.

`target_vector`, `target_count`, `uses_target_matrix`, and `output_width` are
  pure semantic accessors.  `DenseOutputAdapter` is a compatibility field
  carried through compiled, checkpoint, and inference contracts.  The current
  dense compiler does not synthesize one for an accepted declaration: ordinary
  non-tree output width is required to match the task, while a terminal tree
  derives its leaf-output width.  A decoded artifact may still retain and
  validate an adapter.

`DenseFeatureLowering` is either `NumericScalar` or
`CategoricalOneHot { dictionary_width, reserved_index }`.  Each
`CompiledFeatureSpan` records source-vector identity, output offset, width,
and lowering.  `CompiledDatasetSchema` retains the complete row-free vector
  schemas, feature spans, target source identities in declaration order, source
  target dtypes, input width, task, and output width.  Its multiclass decoder
  maps a valid class index to a saved label or the explicit `ReservedUnseen`
  route; an out-of-range or non-multiclass request returns `None`.

`DenseTrainingConfig` carries the layers, loss, data normalization, horizon,
  warmup, decay, optional global gradient clip, normalization epsilon,
  reduction-tree lane ceiling, random seed, and `AdamWConfig`.  AdamW's code
  default is learning rate `1e-4`, beta one `0.9`, beta two `0.999`, epsilon
  `1e-8`, and weight decay `0.01`; callers may provide a different typed
  configuration.  `BinaryValidationConfig` stores calibration-bin count,
  bit-preserved recall thresholds, and optional `TemperatureScalingConfig`.
  Multiclass and regression validation requests are marker structs.
  Temperature scaling defaults to 64 iterations, learning rate `0.01`, and
  bounds `0.05..20.0`.

`TrainingHorizon::Finite(NonZeroU64)` has a fixed loop count;
`TrainingHorizon::Unbounded` has no synthetic terminal epoch and can end only
after a safe-boundary stop request.  `TrainingBounds` records train rows,
training iterations, calibration iterations, total loop iterations, and
warmup iterations.  It is part of compiled evidence and the checkpoint
manifest, but optimizer schedule position is intentionally not a semantic
resume field.

### State and output contracts

Every learned parameter is represented by a `ParameterState` containing initial
and updated parameter, first-moment, and second-moment value identities plus the
AdamW update kernel.  `DenseLayerState` adds ordered PReLU scalars;
convolution, embedding, attention, RNN, GRU, LSTM, tree, and residual states
retain exactly the tensors and resolved geometry needed for checkpoint and
validation traversal.  Pool has no learned state.  K-means owns centroids;
trees own split-feature and split-threshold images plus learned leaf values.

`DataNormalizationState` is `Identity`, `ZScore(mean, variance)`,
`MinMax(minimum, maximum)`, or `L2Norm`.  `OptimizerProgressState` is GPU
resident transition state: the update gate, accepted-update counters, beta
powers, update kernels, and bounded accepted-update policy.  It advances once
for a complete partition only when supervision permits an update.  It is not
an exported artifact; resume restores parameter and moment tensors under the
newly declared phase.

`TrainingOutputs` joins training loss and iteration domain, normalization,
optimizer progress, block/layer state, optional validation outputs, validation
status, and user metric bindings.  Binary metrics are mean BCE, accuracy,
AUROC, AUPRC, Brier score, expected calibration error, and requested recalls;
multiclass metrics are mean cross entropy and accuracy; regression exposes R2.
Metric bindings map each semantic kind to a graph value, metric identity, and
iteration domain.

`CompiledTraining` is the immutable handoff from compilation to preparation.
Its accessors expose the canonical graph and static program, typed external
inputs, bounds, outputs, compiled schema, config, effective blocks/layers, and
optional output adapter.  The graph, not the accessor vectors, is authoritative
for execution and checkpoint output identity.

## Compilation lifecycle

All public dense compile functions are wrappers over one
`compile_dense_training_impl` call.  The flat functions first map each
`DenseLayer` to `DenseBlock::Layer`; the block functions pass the explicit
topology.  The implementation performs these ordered steps:

1. Resolve the task from prepared vector roles, semantic types, encodings,
   dictionaries, and the selected loss.  Binary loss requires explicit numeric
   0/1 or at most two-category dictionary targets.  Regression requires numeric
   int32/f32 targets.  Cross entropy requires dictionary int32 targets and adds
   the reserved unseen-label class.  Multiple ordered numeric targets resolve
   to the corresponding matrix task.
2. Validate the network and configuration.  A network is nonempty; embedding
   is the only leading token block; attention may occur exactly once directly
   after embedding; each recurrent case is one leading recurrent block; trees
   are terminal; pool/K-means routing must match the immediate layer; residuals
   need a branch layer; a final pool is invalid; classification output blocks
   must emit raw logits without a final activation or normalization.  Tree
   depth is at most 30.  Reduction lanes are a power of two in `1..=1024`.
   Epsilons, learning rates, clip norms, weight decay, and beta values are
   checked for finite, domain-valid values.  Warmup cannot exceed a finite
   horizon.  Unbounded phases require constant post-warmup decay.
3. Build `DenseFeaturePlan` and lower the prepared train and validation
   partitions.  Non-embedding int32 values must be exactly representable in f32.
   Embedding features remain exact int32 token IDs, with a nonzero vocabulary
   and every value in `0..vocabulary`.  Recurrent features must be numeric fixed
   width values.  Empty partitions, zero columns, inconsistent rows, target
   width or dtype changes, and unsupported variable-width target encodings are
   rejected before graph emission.
4. Resolve logical shape through every declared block and retain the effective
   block list, layer compatibility view, dataset schema, target width, and
   output adapter.  Ordinary non-tree topologies must finish at the loss output
   width; the terminal tree path derives its leaf-output width from the task.
5. Create `GraphCompiler`.  It allocates checked `ValueId` and
   `KernelTemplateId` identities, establishes the training iteration domain,
   and creates one `ResumeEnabled` int32 scalar initialized to zero.  Every
   external input is typed, shaped, and byte-count checked at creation.
6. Admit train features, train targets, target-supervision masks, validation
   features/targets, and a categorical normalization mask as external images.
   Non-embedding values are converted to f32 in the first iteration domain.
   Z-score, min-max, and L2 normalization are graph calculations.  Their fitted
   tensors are retained as external outputs for semantic checkpointing.
7. Create a full-partition row validity mask and, when targets are masked, a
   known-target supervision count.  The safe count prevents division by zero.
   Objective denominators account for target matrix width where appropriate.
8. Compile every block's forward values and then emit the selected loss and
   its gradient.  BCE-with-logits uses the registered GPU materialization;
   focal, pointwise regression losses, and all scalar programs are emitted as
   Recipe elementwise calculations.  Cross entropy uses either dictionary class
   codes or dense one-hot rows.  Masked values are zeroed before loss and
   gradient reduction.
9. Backpropagate through blocks, flatten parameter gradients in the same stable
   parameter order used by checkpoint traversal, optionally apply one global
   norm clip, and build dynamic AdamW scalars.  Each epoch represents one
   logical complete prepared training partition and one optimizer update.  A
   backend may tile physical work, but it cannot change this contract.  The
   accepted-update gate suppresses updates for a partition with no known target.
10. Compile requested validation in the same static lifecycle.  Binary,
    multiclass, and regression validation requests are mutually exclusive and
    must match the loss/task family.  A missing known validation population is
    reported as `ValidationUnavailableReason::NoKnownTargets`; a binary split
    with only one known class is reported as `SingleKnownClass`.  Unavailable
    validation does not emit invalid metric tensors.  Binary temperature scaling
    is a bounded post-training domain and is rejected for unbounded training.
11. Bind all user metrics, mark only semantic state needed for resume/checkpoint
    as external outputs, and call `GraphCompiler::finish`.  Finish validates
    the graph, serializes and reloads its canonical OGDL form, and constructs a
    `StaticCalculationProgram` with immutable iteration domains.  No native
    compilation or device allocation occurs in this crate's compile phase.

`REMAINING_UNSUPPORTED` explicitly lists `DynamicLoopShortening` and
`ExactOptimizerResume`.  A stop can end an unbounded run at a completed loop
boundary, but compilation does not rewrite its static loop horizon.  Resume
loads semantic parameter and moment bytes, while accepted-update counters and
schedule position start from the newly declared phase.

## Shared forward lowering

`forward.rs` defines the private `RecurrentForwardGraph` trait.  It asks a
caller to supply zero tensors, matrix-column gathers, bias-free linear
projections, f32 elementwise emission, and activation application.  Training's
`TrainingForwardGraph` and inference's `InferenceGraphCompiler` implement the
same interface, so equations cannot diverge between validation and loaded-model
inference.

The sequence helpers initialize zero hidden state per row and consume columns
in order.  Vanilla RNN computes `tanh(input * input_weight + hidden *
recurrent_weight + bias)`.  The reset-before GRU computes sigmoid reset and
update gates, multiplies reset by the previous hidden state for the candidate,
then uses `candidate + update * (previous - candidate)`.  LSTM computes input,
forget, output, and tanh candidate gates, updates
`forget * previous_cell + input * candidate`, and emits `output * tanh(cell)`.
All intermediate values required by backward lowering are retained in typed
step records.

`lower_activation` maps declaration activations to registered GPU operations or
canonical scalar programs.  Signed-magnitude logarithms preserve sign around
`ln(abs(x))`; natural logarithm is a separate positive-domain operation; the
legacy signed `ln(1 + abs(x))` spelling remains decode-only compatibility.
PReLU uses the canonical learned-slope program.  Causal attention masks and
head-major index programs use checked int32 extents.  Elementwise helpers and
materializations install forbidden alias rules where an output may not overlap
an input.  No forward helper is callable from outside the crate.

## Native execution lifecycle

### Inputs, limits, and evidence

`OwnedExternalInput` exposes role, `ValueId`, dtype, shape, and exact bytes.  Its
`ExternalInputRole` distinguishes train/validation data, resume components,
normalization masks, pool/convolution index tables, K-means centroids, and tree
split/leaf state.  `TrainingExecutionLimits` and
`InferenceExecutionLimits` contain the caller's executor `Watchdog`.

`TrainingExecutionControl::uninterrupted` has no stop source.  A graceful-stop
control contains a read-only `AtomicBool`; it is observed only after a complete
loop iteration.  An unbounded program without a stop source fails before native
preparation with `UnboundedTrainingRequiresStopControl`.

`TrainingMetricObserver` uses a bounded synchronous channel.  Each selected
metric has an independent cadence counter, and emission uses `try_send`; a full
or disconnected consumer drops only the live notification.  The observer
cannot backpressure or mutate the graph.  `FinalTrainingMetric` always retains
the newest sample available at loop, exit, or mailbox drain.  Epochs presented
to users are one-based; the raw zero-based loop index remains a diagnostic
field.

`CompletedTrainingExecution` contains run and bundle identities, sorted exit
images and their logical `ValueId` mapping, final metrics, exact realized native
kernel images, native evidence, `TrainingExecutionEvidence`, and the complete
executor journal.  Native resources have been destroyed before it is returned.
`TrainingExecutionEvidence` joins compiled bounds, logical updates per epoch,
optimizer kernel/task/submission counts, started/completed loop counts, loop
calculation counts, non-GPU calculation counts, and compacted journal totals.

`CompletedInferenceExecution` contains one typed `InferencePrediction`, native
images/evidence, elapsed loop time, and journal.  The elapsed interval starts
after initialization and ends when the one-iteration loop completes; preparation,
warmup, egress, and teardown are outside it.  KNN uses
`CompletedKnnInferenceExecution` with one independently typed prediction per
declared target.  `InferenceRunFailure` retains run, bundle, optional journal,
and the first cleanup error after executor handoff.

### Training entrypoints

`build_training_device_images` packs every declared external input into exactly
one init image per finalized device.  It rejects duplicate inputs/devices,
unknown or missing members, dtype or byte mismatches, out-of-bounds members,
overlap, and images that cannot fit the host.  A logical input may be copied to
multiple device images, but it is uploaded only by init.  No external transfer
is permitted in the loop.

`prepare_and_execute_local_training` is the uninterrupted convenience wrapper.
`prepare_and_execute_local_training_with_observer` adds a bounded live metric
observer.  `prepare_and_execute_local_training_controlled` is the single
implementation: it validates stop control, calls
`Preparer::prepare_program` with the static program and measured profile,
rejects finalized loop external transfers, builds init images and output maps,
retains realized native images, hands the prepared local session to the backend,
then performs `PreparedRun::prepare -> initialize -> start_loop`.

The loop polls with progress and a bounded 50 microsecond to 2 millisecond
backoff.  It drains metrics without blocking, accepts a requested stop only at
the executor's completed-iteration boundary, enters the exited-loop state,
executes exit egress, drains the metric mailbox, computes execution evidence,
and returns after teardown.  Loop output is never admitted or collected through
the live API.  Training execution errors are typed as preparation, native
handoff, executor, init-image contract, loop-transfer, external-output mapping,
invalid-bounds, missing-stop-control, or terminal-state failures.

### Inference entrypoints

`build_inference_device_images` and its KNN sibling apply a closed boundary:
every declared input must occur in an init manifest and every manifest member
must name a declared input.  `prepare_and_execute_local_inference` and
`prepare_and_execute_local_knn_inference` require both compiled program and
finalized bundle to have exactly one loop iteration, reject loop external
transfers and user metrics, and use the same preparation, handoff, polling,
exit, and teardown sequence as training.

Before execution, output mapping checks that every planned prediction is an
exit transfer from the expected physical device/value, has the declared dtype
and byte count, is non-overlapping, and appears exactly once.  After exit,
collection checks the returned image task, physical source, dtype, and exact
host byte length again.  Inference errors retain executor failure evidence when
handoff occurred and distinguish invalid boundary, input-image, loop-transfer,
missing/duplicate/unexpected output, overlap, dtype/size/source mismatch, and
terminal-state failures.

## Checkpoint and resume lifecycle

### Manifest construction

`CheckpointManifest::from_compiled` captures the row-free vector schema, feature
spans and width, target source identities and dtypes, resolved task and output
adapter, config with legacy `layers` cleared, bounds, fitted normalization
tensors, effective topology, optional temperature, and the canonical
`compiled_training_program_digest`.  It validates that every manifest tensor is
unique and exactly matches the graph's external-output set.  The manifest has
no rows, queues, devices, contexts, native handles, or executable bytes.

The selected format version is the newest semantic topology version required by
the effective blocks: v9 native dense, v10 K-means, v11 multi-target, v12 tree,
v13 embedding/attention, v14 RNN, v15 GRU, and v16 LSTM.  Older versions v5 to
v8 remain decoder compatibility paths for flat, structured, pool, and legacy
native images.  Native realization metadata is optional in a decoded semantic
artifact and is attached by `CompletedTrainingCheckpoint::new` after a real
native run.

`compiled_training_program_digest` serializes the complete static program to
canonical OGDL and hashes it with SHA-256.  External input bytes, including
resumed parameter bytes, are intentionally outside this identity.  The digest
authenticates a supplied native kernel against the current graph without
making model state part of the program identity.

### Completed checkpoint and save

`CompletedTrainingCheckpoint::new` accepts only a fully exited
`CompletedTrainingExecution`.  It attaches measured realization, topology, and
discovery identities plus exact native image target/toolchain/digest metadata,
validates semantic invariants, and maps every exit image to its logical
checkpoint tensor.  Its `save(path)` encodes the semantic OGDL model and tensor
images.  Native bytes are not embedded in that model.

`save_native_kernel(path, format)` writes the exact bytes that were built,
validated, loaded, and warmed.  The path extension must match `.cubin` for
CUDA or `.hsaco` for HSA.  Saving fails if no matching image exists or if more
than one distinct image of that format was realized.  Semantic and native saves
are independent, and no journal, plan, cache, profile, or intermediate file is
part of the public artifact pair.

All saves use `atomic_save`: validate a regular non-symlink target and parent,
check filesystem capacity while preserving the exact user reservation, write a
private mode-0600 temporary, flush and sync it, verify measured size, atomically
rename, and sync the parent.  Failed temporary writes are removed by a guard.

### Decode

`decode_checkpoint` first enforces the source-byte limit, requires UTF-8, parses
the OGDL graph, enforces node and allocation limits, and then runs the strict
versioned decoder.  It rejects unknown or duplicate fields, missing fields,
noncanonical values, inconsistent topology, invalid tensor dtype/shape/size,
and semantic model violations.  `CheckpointDecodeLimits` bounds source bytes,
nodes, vectors, feature spans, layers, metadata entries, tensor count/rank,
per-tensor bytes, and total payload bytes.  Decoding creates a native-handle-free
`CheckpointArtifact` with accessors for schema, task, normalization, layers or
structured blocks, temperature, native realization metadata, and multiclass
label decoding.

### Resume

`apply_checkpoint_resume` validates the decoded artifact, captures a manifest
from the current compiled program, and compares schema, feature spans and
normalization mask, target order and task, output adapter, objective and
normalization semantics, AdamW beta/epsilon/decay bits, effective topology,
parameter tensor contracts, K-means centroid contracts, and tree split tensor
contracts.  It then replaces the compiled external bytes for exactly one
`ResumeEnabled` input, every parameter/first-moment/second-moment triple, every
K-means centroid, and every tree split image.  It verifies all expected roles
occur once and all inputs are consumed.

The current declaration still supplies the new horizon, warmup, learning-rate
schedule, and static program.  Accepted-update counters, beta powers, and
automatic stopping are not silently continued as a host-side substitute.  A
missing resume file is handled by the public frontend as “start fresh”; it is
not an error and does not disable an independent save declaration.

## Target-free inference lifecycle

### Loading and schema preparation

`load_checkpoint_file`, `load_knn_model_file`, and `load_bayes_model_file` read
bounded regular-file snapshots and pass their admitted bytes to strict
decoders.  `load_semantic_model_file` probes only the first root line, dispatches
`recipe` to the dense decoder, `recipe-knn-model` to KNN, and
`recipe-bayes-model` to Bayes, then lets the selected decoder perform complete
syntax, version, canonicality, and model-family validation.

`prepare_checkpoint_inference_table` applies only saved feature names and
schemas.  Input columns may be reordered, targets may be absent, and unrelated
columns are ignored.  Categorical dictionaries and the reserved unseen route
are reused exactly.  `PreparedInference` retains the decoded checkpoint and
unnormalized schema-bound rows; no host numeric conversion, one-hot expansion,
normalization, or model calculation is performed.  KNN preparation uses its
saved feature spans.  Bayesian preparation constructs the union of all parent
schemas in first-occurrence order and reads shared parents once.

### Dense compilation

`compile_prepared_inference` requires nonempty effective blocks and at least one
row.  It creates external inputs for raw schema-bound feature tensors, fitted
normalization tensors and masks, and every learned parameter or structural
index image needed by the checkpoint.  Optimizer moments are never admitted.
Numeric conversion, categorical one-hot expansion, concatenation, saved
normalization, every block forward operation, optional temperature scaling, and
prediction interpretation are graph calculations.  The compiler traverses the
canonical `blocks` list exactly once, preserving pooling, residual, convolution,
tree, embedding, attention, RNN, GRU, and LSTM topology.  It finishes a single
iteration static program and emits one f32 prediction matrix with an
`InferenceOutputContract` describing value, shape, source target dtypes, and
`InferencePredictionKind`.

The dense prediction kinds are binary probability, multiclass probabilities,
regression, multi-target binary probabilities, joint target probabilities, and
multi-target regression.  The task is represented by `InferenceTask::Dense`.

### KNN compilation

`compile_prepared_knn_inference` rejects post-KNN scalar or normalization
operations because one transform cannot be applied coherently to heterogeneous
numeric and discrete outputs.  It requires nonempty query and reference rows
and nonzero feature width.  Query/reference feature conversion and optional
normalization, distance, neighbor selection, numeric means, and discrete modes
are all device operations.  Each declared target gets its own output contract:
f32 `[rows, 1]` for `NumericMean` or int32 `[rows, 1]` for `DiscreteMode`.

`KnnReferenceSet` retains reference row order as deterministic distance-tie
order, exact f32 feature bits, per-output known masks, numeric values or
deterministic discrete label dictionaries, and schema/lowering metadata.
`KnnModelArtifact::continue_with` appends a compatible prepared partition,
preserving repeated rows and their order.  KNN has no optimizer loop and no
native training kernel artifact.

### Bayesian compilation

`resolve_bayesian_schema` assigns canonical node IDs by name bytes while keeping
declaration and parent order.  It rejects empty or duplicate names, duplicate
children/edges, self-dependencies, and cycles.  Missing parent-only names are
latent roots and missing children with parents are latent conditionals; the
observed executable slice rejects latent nodes because no state space was
declared.

`prepare_categorical_bayesian_reference_sets` requires nonempty prepared rows,
one or more declarations, observed dictionary-categorical parent features,
observed dictionary-categorical target children, and target source identities
equal to declaration order.  It retains parent codes, child codes,
mixed-radix cardinalities/multipliers, reference source rows, and deterministic
Laplace-one smoothing metadata.  `BayesModelArtifact::continue_with` appends
compatible observations while retaining saved rows before current rows.

`compile_prepared_bayes_inference` lowers each repeated conditional in
declaration order.  Native histogramming and posterior calculation produce
f32 probability blocks; adjacent class ranges follow repeated declaration
order, and a concatenation graph emits one aggregate prediction matrix.  The
task is `InferenceTask::BayesProbabilities`.

### GGUF llama instrument

The private `gguf_llama` module is exposed through
`GgufLlamaArtifact`, `PreparedGgufLlamaInference`, `decode_gguf_llama`,
`load_gguf_llama_model_file`, `prepare_gguf_llama_inference_table`, and
`compile_prepared_gguf_llama_inference`.  The supported case is a bounded
little-endian GGUF v3 dense-f32 `llama` graph with equal query/KV head counts,
full-head adjacent-pair RoPE, RMSNorm, causal attention, and parallel SwiGLU
disabled.  Unsupported architectures, quantized variants, grouped-query
attention, mixture-of-experts, noncausal or parallel-residual variants,
unsupported RoPE scaling, invalid metadata, missing tensors, and bad tensor
shapes fail closed with `GgufLlamaError`.

The input is one ordered whitespace-separated int32 token stream in one table
column.  Tokens must be nonnegative, below vocabulary, nonempty, and no longer
than model context.  Compilation admits token IDs and execution tensor images,
emits all-position raw vocabulary logits, and marks the task
`InferenceTask::TokenLogits`.  Tokenization, chat templates, sampling, and KV
session state are not implied by this instrument.

## Callers and complete system path

The crate is called by the public root frontend, not directly by a user-facing
CLI process.

### Training caller (`src/training.rs`)

`Train::run` obtains the preceding `Data` and `Model` declarations and calls
`try_run_with`:

1. Bayesian declarations call `compile_bayes_model`, which validates that no
   dense layers, generic loss, normalization, optimizer, loop, or kernel save
   was requested; prepares observed references; optionally continues an
   existing `.ogdl` Bayes model; and saves the semantic artifact if requested.
2. A single KNN block calls `compile_knn_model`, which applies the analogous
   no-optimizer/no-loop policy, prepares references, optionally appends a saved
   KNN model, and saves only its semantic `.ogdl` artifact.
3. Dense declarations call `compile_training_package`.  It prepares the data,
   maps facade blocks and policy into `DenseTrainingConfig`, selects the one
   validation family, invokes the matching dense compile entrypoint, loads an
   existing checkpoint only when the declared resume path exists, applies
   `apply_checkpoint_resume`, and captures `CheckpointManifest::from_compiled`.
4. The frontend enters `with_current_native_preparation`, derives runtime
   tuning and a watchdog from the measured graph/profile, builds a production
   `Preparer`, optionally authenticates a supplied `.cubin` or `.hsaco` against
   checkpoint digest and measured identities, and calls
   `prepare_and_execute_local_training_controlled` with a SIGINT stop flag.
5. A real completed execution becomes `CompletedTrainingCheckpoint`, then
   `TrainingReport::dense`.  Independent `.save(MODEL_PATH)` and native-kernel
   destinations are written after execution.  The public report exposes no
   native resources that have not completed teardown.

The facade supplies some policy defaults before calling this crate, including
normalization epsilon `1e-6`, reduction lane ceiling `1024`, and a fixed
Recipe seed.  The crate still validates the resulting typed configuration and
does not infer missing policy from runtime observations.

### Inference caller (`src/inference.rs`)

`compile_inference` validates target-free data policy, requires a `.ogdl` or
`.gguf` model source, loads it through the bounded functions above, distills and
selects target-free rows, prepares the matching schema, and chooses one of
`compile_prepared_inference`, `compile_prepared_knn_inference`,
`compile_prepared_bayes_inference`, or
`compile_prepared_gguf_llama_inference`.  Inference data cannot declare targets,
split fractions, or replacement normalization.

The frontend then creates a production measured-profile `Preparer` and calls
`prepare_and_execute_local_inference` for dense/Bayes/GGUF or
`prepare_and_execute_local_knn_inference` for KNN.  The returned typed bytes
are decoded by the public report layer using the contract and saved schema;
there is no host-side prediction substitute.

## Invariants that span modules

* The graph is static and typed.  Compilation, preparation, and execution all
  preserve `ValueId`, dtype, shape, phase, and physical-source contracts.
* Training has one logical full-partition update per epoch.  There is no public
  partial-batch control, and backend tiling cannot change update semantics.
* Data ingress belongs to `init`; model calculations belong to `loop`; semantic
  outputs and checkpoint state leave through `exit`.  A loop external transfer
  is rejected, not silently serviced by a fallback.
* External images are singular per finalized device and are validated for
  duplicate members, exact dtype/bytes, bounds, and overlap.  Inference uses a
  closed input boundary; training permits only the planned external-output
  checkpoint set.
* Native preparation is measured and immutable.  The realized kernel bytes and
  target/toolchain/topology/discovery identities retained in execution evidence
  are exactly the images handed to the executor.
* Checkpoint compatibility is semantic and structural, not a filename check.
  Program digest authenticates native realization; schema, objective,
  normalization, topology, and tensor contracts authenticate resume.
* Host code observes metrics and stop requests but never recreates calculation
  state, mutates model tensors during the loop, or computes predictions from
  returned bytes.
* All bounded readers and decoders enforce finite allocation limits before
  allocating from untrusted model input.  All artifact writes are atomic and
  capacity checked.
