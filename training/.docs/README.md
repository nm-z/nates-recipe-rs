# `recipe-training`

`recipe-training` is Recipe's owned static dense-training compiler and the
training-side model/inference artifact boundary. It turns a prepared,
typed dataset plus a declared model and policy into one canonical
`StaticCalculationProgram`. The program contains only Recipe calculations and
transfers, with an immutable `init -> loop -> exit` lifecycle. Native discovery,
compilation, allocation, image admission, execution, and teardown are kept in
the caller and in `execute.rs`; they are not graph nodes invented by this
crate.

The crate also owns three target-free model instruments which share the same
preparation and native execution boundary:

- dense checkpoints, which retain semantic schema, effective topology,
  parameters, AdamW moments, normalization state, and optional native identity;
- KNN reference models, which retain an ordered immutable reference image and
  independently typed target outputs; and
- observed categorical Bayesian models, which retain exact categorical
  observations and dictionaries while native inference performs histogramming
  and Laplace posterior calculation.

The first concrete GGUF instrument, a dense-F32 `llama` graph, is also exposed
through this crate's inference module. It is deliberately fail-closed for
architectures and variants outside the implemented graph.

This document describes the active implementation in `training/src`, the root
crate's callers in `src/training.rs` and `src/inference.rs`, and the boundaries
that are visible to a real Recipe declaration. It does not treat a declaration
or a parsed configuration as proof that a native run has succeeded.

## Package and module ownership

`training/Cargo.toml` names the package `recipe-training`, version `0.1.0`,
edition 2024, with the description "Recipe-owned static dense-training graph
compiler". It depends on the typed graph and runtime crates rather than
implementing a second execution system:

| Dependency | Role at this boundary |
| --- | --- |
| `recipe-core` | IDs, dtypes, lifecycle identities, loop bounds, artifacts, and typed units. |
| `recipe-executor` | `PreparedRun`, native-independent lifecycle polling, journals, metrics, and watchdog errors. |
| `recipe-ingest` | lossless source preparation, typed vectors, dense matrices, inference tables, and GGUF archives. |
| `recipe-language` | shapes, tensors, calculation nodes, primitive kernels, scalar programs, and OGDL graph codecs. |
| `recipe-math` | stable mathematical scalar programs used by inference. |
| `recipe-native-executor` | local backend handoff and native execution evidence. |
| `recipe-ogdl` | semantic KNN and Bayesian graph documents. |
| `recipe-ops` | owned materialized operations, metric requests, convolution/pool preparation, tree/K-means lowering, and operation registry. |
| `recipe-prepare` | measured placement, candidate realization, native artifact loading, warming, and finalization. |
| `recipe-probe` | measured topology/discovery profiles used for runtime tuning and preparation. |
| `rustix` | bounded filesystem operations used by checkpoint atomic saves. |
| `sha2` | canonical static-program digest stored with native realization metadata. |

`training/src/lib.rs` forbids unsafe code and denies missing `Debug`
implementations. It declares these modules:

| Module | Visibility | Responsibility |
| --- | --- | --- |
| `model.rs` | private module, almost all model/state types re-exported | Dense model IR, task/config/state/output contracts, dataset lowering helpers, and bounds. |
| `compile.rs` | private module, compile entry points re-exported | Graph construction, shape/routing checks, forward tape, losses, reverse tape, validation, schedules, and AdamW updates. |
| `forward.rs` | private module | Shared activation lowering and exact RNN/GRU/LSTM forward equations used by training and inference. |
| `execute.rs` | private module, lifecycle functions/types re-exported | Init image packing, native preparation handoff, polling, stop control, metrics, output mapping, and evidence. |
| `checkpoint.rs` | private module, artifact types and resume APIs re-exported | Dense manifest capture, strict versioned OGDL decode/encode, output validation, native identity, resume admission, and atomic save. |
| `inference.rs` | private module, inference APIs re-exported | Dense/KNN/Bayesian/GGUF preparation and one-iteration graph compilation. |
| `bayes.rs` | public module and re-exported types/functions | Bayesian declaration resolution, DAG validation, categorical schema, mixed-radix metadata, and exact observed references. |
| `bayes_checkpoint.rs` | private module, artifact/decode types re-exported | Version 1/2 Bayesian semantic OGDL artifacts and bounded canonical decode. |
| `knn.rs` | private module, reference types/functions re-exported | Typed KNN reference image and deterministic target aggregation preparation. |
| `knn_checkpoint.rs` | private module, artifact/decode types re-exported | Version 1 KNN semantic OGDL artifacts, append/resume, and bounded canonical decode. |
| `gguf_llama.rs` | nested private module under `inference.rs`, public API re-exported | Dense-F32 `llama` GGUF validation, token binding, RoPE tables, and logits graph. |
| `error.rs` | private module, error types re-exported | `TrainingCompileErrorKind`, `TrainingCompileError`, and the compile result alias. |

The crate does not provide a public constructor for a native backend. Native
execution is entered through the generic `Preparer` boundary in
`execute.rs`, normally created by `src/training.rs` or `src/inference.rs`
inside the current native binding scope.

## Public surface

The facade in `lib.rs` re-exports the following groups.

### Dense model and compiler contracts

`DenseActivation`, `DenseNormalization`, `DenseOperation`, `DenseLayer`,
`DenseBlock`, `DenseResidual`, `DenseConvolution`, `DensePool`, `DenseKMeans`,
`DenseTree`, `DenseEmbedding`, `DenseAttention`, `DenseRnn`, `DenseGru`, and
`DenseLstm` describe the effective topology. `DenseBlockKind` distinguishes
ordinary layers from `Perc` layers. `DenseResidualOperation` preserves the
ordered branch operations and `DenseResidualState` preserves the learned skip
projection and PReLU occurrences.

`DenseTask` describes one scalar binary target, categorical target, scalar
regression target, or one ordered multi-target matrix for binary, joint
categorical, or regression objectives. `DenseTrainingConfig` carries the
loss, data normalization, finite or unbounded horizon, warmup, decay, gradient
clip, reduction-tree bound, seed, and AdamW values. `BinaryValidationConfig`,
`MulticlassValidationConfig`, `RegressionValidationConfig`, and
`TemperatureScalingConfig` select optional validation/calibration graphs.

The eight dense entry points are:

```text
compile_dense_training
compile_dense_training_with_blocks
compile_dense_training_with_binary_validation
compile_dense_training_with_multiclass_validation
compile_dense_training_with_regression_validation
compile_dense_training_with_blocks_and_binary_validation
compile_dense_training_with_blocks_and_multiclass_validation
compile_dense_training_with_blocks_and_regression_validation
```

The flat functions convert `config.layers` to `DenseBlock::Layer`. The
`with_blocks` functions take the topology explicitly; `config.layers` remains
the legacy declaration only for the flat wrappers.

`CompiledTraining` exposes the canonical graph/program, owned external input
images, `TrainingBounds`, `TrainingOutputs`, compiled dataset schema, config,
flat layers, effective blocks, and an optional output adapter. The graph is
canonicalized through OGDL and reparsed before the `StaticCalculationProgram`
is returned.

### Runtime and evidence

`execute.rs` re-exports:

- `build_training_device_images` and
  `prepare_and_execute_local_training` for the ordinary path;
- `prepare_and_execute_local_training_with_observer` for bounded live metric
  delivery; and
- `prepare_and_execute_local_training_controlled` for a host stop flag and an
  optional observer.

`TrainingExecutionLimits` wraps the caller-selected watchdog. A
`TrainingExecutionControl` is either uninterrupted or a read-only reference
to an `AtomicBool` sampled after completed loop iterations. `CompletedTrainingExecution`
contains run and bundle identities, exit images, logical output IDs, final
metric samples, realized native images, native evidence, training evidence,
and the completed `RunJournal`.

`TrainingExecutionEvidence` reports the compiled bounds, logical updates per
epoch, optimizer update-kernel/task/submission counts, loop iterations started
and completed, loop calculation counts, non-GPU calculation counts, and
compacted journal event/call totals.

`bounded_training_metric_channel` returns a `TrainingMetricObserver` and a
bounded receiver. The observer uses nonblocking `try_send`; saturation or a
dropped consumer drops only a live notification. The final report still keeps
the newest sample for every planned user metric.

### Dense semantic artifacts and resume

`CheckpointManifest::from_compiled` captures the row-free dataset schema,
feature spans, target order/task, effective blocks, config, bounds,
normalization tensors, optional calibration temperature, and canonical program
digest. `CompletedTrainingCheckpoint::new` joins that manifest to a completed
native execution, authenticates the native realization identities, and maps
every finalized exit image to the exact logical tensor it represents.

The public checkpoint surface includes `CheckpointArtifact`, all block and
parameter image types, `CheckpointDecodeLimits`, path-addressed decode errors,
`decode_checkpoint`, `apply_checkpoint_resume`, and
`compiled_training_program_digest`. A completed dense checkpoint can `save`
the semantic model or `save_native_kernel` one unambiguous `.cubin` or
`.hsaco` image.

### Target-free inference

`inference.rs` exposes:

```text
load_checkpoint_file
prepare_checkpoint_inference / prepare_checkpoint_inference_table
load_and_prepare_checkpoint_inference
compile_prepared_inference
load_knn_model_file / prepare_knn_inference_table / load_and_prepare_knn_inference
compile_prepared_knn_inference
load_bayes_model_file / prepare_bayes_inference_table / load_and_prepare_bayes_inference
compile_prepared_bayes_inference
load_semantic_model_file
load_gguf_llama_model_file / prepare_gguf_llama_inference_table
compile_prepared_gguf_llama_inference
```

`PreparedInference` retains a decoded dense checkpoint and schema-bound,
unnormalized rows. `CompiledInference` contains one static one-iteration
program, typed external inputs, an output contract, row count, task kind, and
the saved output adapter. `CompiledKnnInference` contains one egress contract
per declared target. Dense, Bayesian, and GGUF outputs use
`InferencePrediction`; KNN returns independently typed
`KnnInferencePrediction` values.

## Root-crate call graph

The public declaration facade lives outside this crate. The concrete dense
path is:

```text
Train::run (src/training.rs)
  -> take_recipe_training_sequence
  -> Train::try_run_with
  -> compile_training_package
     -> compile_training_graph
        -> policy/data/model validate
        -> prepare_data
        -> map_dense_block / map_dense_operations
        -> compile_dense_training[_{with_blocks}][_and_*_validation]
        -> optional load_checkpoint_file + apply_checkpoint_resume
     -> load_resume_native_bundle (only when both model and kernel paths exist)
     -> CheckpointManifest::from_compiled
  -> install SigintGuard
  -> execute_current_training
     -> with_current_native_preparation
     -> derive_native_runtime_tuning from measured profile
     -> Preparer::prepare_program
     -> prepare_and_execute_local_training_controlled
  -> TrainingReport::dense
     -> optional `.save` model and `.save` native kernel
```

`Train::try_run_with` dispatches before dense compilation:

- any Bayesian dependency declarations call `compile_bayes_model`; the result
  is a semantic report with no native optimizer run;
- a model containing a KNN layer calls `compile_knn_model`; the result is a
  reference-only report with no optimizer loop; and
- all other declarations use the dense path above.

`src/inference.rs` follows the same preparation boundary for target-free
inference. It loads a strict semantic root (`recipe`, `recipe-knn-model`, or
`recipe-bayes-model`) or a bounded GGUF image, binds a distilled target-free
table, compiles the selected instrument, and calls the matching
`prepare_and_execute_local_*` function. It owns native runtime tuning and
report presentation, while `recipe-training` owns graph contracts and output
validation.

## Dense compilation pipeline

### 1. Public declaration and dataset preparation

`src/training.rs::compile_training_graph` first validates `Train`, `Data`, and
`Model`, rejects model-loaded weights for a new dense graph, maps the built-in
objective to `DenseLoss`, and requires Recipe's AdamW optimizer plus an explicit
learning-rate decay. A public dense declaration must select an explicit train
split. `prepare_data` loads and distills the source under finite ingest bounds,
infers typed vectors, applies target/row/column selection and the train/validation
split, and performs no imputation, normalization, or lossy scalar conversion.

The facade mapping is direct:

- `Dense` and `Perc` become `DenseLayer` with ordered `DenseOperation`s;
- `Embedding` requires an immediate vocabulary declaration and becomes a
  leading `DenseEmbedding`;
- `Attention`, `Rnn`, `Gru`, `Lstm`, `Convolution`, `Pool`, `KMeans`, tree
  boosters/forests, and residual declarations become their corresponding
  `DenseBlock` variants;
- `LayerNormalization` and `BatchNormalization` become the two
  `DenseNormalization` variants; and
- all public activations map to `DenseActivation`, including the decoder-only
  legacy signed-log token retained for old artifacts.

The facade selects `DenseDataNormalization::Identity` only for a leading
embedding. Non-embedding dense training requires explicit Z-score, min-max, or
L2 normalization. An unbounded phase can use only a constant post-warmup
learning rate. Finite decay endpoints are represented by a GPU-derived schedule
inside the loop.

### 2. Dataset and task lowering (`model.rs`)

`DenseFeaturePlan::from_prepared` walks feature vectors in prepared order:

- numeric I32/F32 vectors use one scalar column and are marked for numeric
  normalization;
- dictionary categorical vectors become `dictionary length + 1` one-hot columns,
  where the final reserved column represents a missing/unseen code and is not
  normalized; and
- all other semantic tuples fail with `InvalidFeatureMatrix` because this
  compiler has no dedicated dense lowering for them.

`lower_dense_features` emits a fixed F32 matrix when categorical expansion is
needed. Numeric I32 values must be exactly representable as F32. Missing
numeric values fail; missing categorical values take the reserved one-hot
route. Dictionary labels must be nonempty and unique, and category codes may
not exceed the dictionary or reserved index.

`resolve_dense_task` derives the task from target schema and loss:

| Target/loss | `DenseTask` | Required output |
| --- | --- | --- |
| Numeric 0/1 plus BCE/focal | `BinaryClassification` | one raw logit |
| Dictionary categorical plus cross entropy | `MulticlassClassification` | dictionary classes plus one reserved unseen route |
| Numeric plus MSE/MAE/Huber | `ScalarRegression` | one value |
| Two or more homogeneous numeric targets plus BCE/focal | `MultiTargetBinaryClassification` | one logit per target |
| Two or more homogeneous numeric targets plus cross entropy | `JointMulticlassClassification` | one row-wise softmax vector |
| Two or more homogeneous numeric targets plus MSE/MAE/Huber | `MultiTargetRegression` | one value per target |

Target lowering preserves an `I32` categorical matrix for ordinary
multiclass, or an F32 matrix for numeric/multi-target tasks. Missing target
rows are marked `Missing`, dictionary labels absent from the fit partition are
marked `Unseen`, and their values are zeroed under a GPU supervision mask.
Validation retains only known rows. The training partition itself remains the
complete prepared training set, not a host-side mini-batch stream.

`DensePartition` verifies nonempty rows, positive feature/target widths,
matching row counts, exact matrix storage length, finite F32 payloads, and
target-observation alignment. `CompiledDatasetSchema` retains source vector
schemas, ordered feature spans, ordered target identities and dtypes, and the
task/output width. It can decode multiclass indices through the fitted
dictionary, with the reserved unseen route explicit.

### 3. Network shape and policy checks

`LogicalFeatureShape` carries fixed logical sequence length and channel count.
It resolves embedding dimensions, attention head dimensions, recurrent hidden
widths, convolution output length (`input length - kernel + 1`), pooled group
count (`ceil(input length / pool size)`), and group-to-neuron routing. The
compiler rejects:

- empty networks, zero or overflowing widths, a final pool, a final width that
  differs from the task output width, or a missing explicit output layer;
- embeddings that are not the first block or token features that are not exact
  numeric I32;
- attention that is not immediately after one embedding, has non-divisible
  heads, or appears more than once;
- recurrent blocks that are not the leading block, contain chained operations,
  or use nonnumeric/categorical feature columns;
- repeated or misrouted pool/K-means group-to-neuron declarations;
- trees that are not the sole terminal block, exceed depth 30, or overflow
  complete-tree index domains;
- residual branches without a final layer, or output widths inconsistent with
  the declared facade; and
- normalization/optimizer values that are nonfinite, nonpositive where
  required, outside AdamW beta bounds, or use a reduction-tree lane count that
  is not a power of two in `1..=1024`.

`effective_blocks` requires classification outputs to be raw logits: the final
declared block cannot add activation or normalization after the logits. A
residual skip is identity at equal widths and a learned weight-only projection
otherwise. Divisible pooled/K-means group connections use contiguous expansion
or contraction; nondivisible shapes use full connectivity.

### 4. Graph compiler and external state

`GraphCompiler` allocates monotonically checked `ValueId` and
`KernelTemplateId` identities, records tensor contracts and primitive nodes,
and associates every kernel with either `IterationDomain::first()` or the
training loop domain. It creates one `ResumeEnabled` I32 scalar input first,
then typed `OwnedExternalInput` images for features, targets, masks,
normalization inputs, resume parameters/moments, structured block metadata,
and validation inputs.

All external input bytes are little-endian and checked against the declared
dtype/shape byte count. Materialized operations reserve a bounded identity
namespace, import their tensor contracts, and reject conflicts with existing
contracts. `finish` marks only explicitly declared input/output values as
external, validates the calculation graph, canonicalizes it to OGDL and back,
then constructs and round-trips the `StaticCalculationProgram` with metric
emissions.

The external roles are semantic rather than an alternate model ontology. They
cover train features/targets/supervision, resume-enable and parameter/moment
images, K-means centroids, tree split arrays, convolution/pool index tables,
validation images, and the categorical normalization mask. Structured values
such as pool winners and convolution windows are immutable preparation data,
not user model outputs.

### 5. Forward graph

The forward pass is emitted once for the complete training partition. A
backend may tile the matrix physically, but the logical domain remains one
full-partition update per epoch.

- A leading embedding gathers exact I32 token rows from its learned
  `[vocabulary, dimensions]` table.
- Causal attention projects Q/K/V, reshapes to head-major order, contracts
  query/key scores, applies `1/sqrt(head dimension)`, applies a causal masked
  softmax, contracts values, restores sequence order, and applies the output
  projection. Head permutations are explicit index programs.
- `forward.rs` lowers RNN, reset-before GRU, and zero-cell LSTM sequences over
  one scalar feature per time step. Hidden and cell states begin at zero. The
  training tape retains every gate/preactivation needed by reverse mode.
- Ordinary layers materialize `gpu_linear_into`, then apply ordered activation
  and normalization operations. PReLU owns one learned scalar per occurrence.
- Channelwise stride-one convolution prepares deterministic windows, contracts
  columns with `[kernel, input channels, filters]` weights, adds bias, restores
  grouped matrix order, and applies operations.
- Maximum pool materializes `recipe_max_pool_1d` with prepared window and
  winner-base indices. Its winner contract is lowest logical index and its
  flattened group order is group-major/channel-minor.
- K-means materializes initialization on `IterationDomain::first()` and a
  Lloyd update in the training domain. Centroids are external outputs for
  checkpointing, while distances feed the following block.
- Trees build split features and thresholds in the first domain, optionally
  bootstrap rows for forests, materialize tree predictions, and retain leaf
  indices for leaf-value gradients. LightGBM, CatBoost, and XGBoost keep
  distinct growth rules.
- Residual branches lower their ordered layers/operations, merge with identity
  or a learned projection, then apply post-merge operations.

Activations use owned GPU operation symbols where available and scalar programs
for composed forms. `forward.rs` supplies exact scalar programs for signed and
natural logarithms, Huber, leaky/PReLU/SELU/ELU, GRU/LSTM equations, causal
masks, normalization, L2, and arithmetic helpers. Alias rules explicitly
forbid accidental in-place reuse except for the recurrence/update contracts
that require exact aliasing.

### 6. Loss, masking, and reverse graph

The compiler emits BCE-with-logits, focal, MSE, MAE, Huber, categorical
cross-entropy, or dense-target cross-entropy. Cross entropy first computes a
stable row-wise softmax using a maximum reduction. Missing/unseen target rows
are zeroed before the loss; normalized gradients use a safe known-count
denominator. Multi-target non-cross-entropy objectives divide by target count
as well as the known-row count.

`backward_blocks` walks the forward tape in reverse. It computes layer and
operation gradients, normalization backward statistics, convolution window
gradients, max-pool scatter gradients, K-means input routing, tree leaf-value
gradients, embedding table accumulation, attention Q/K/V/input gradients, and
the full reverse recurrence for RNN/GRU/LSTM. The first embedding, recurrent,
and terminal tree constraints are enforced again while traversing the tape.

An optional global clip computes one squared norm across every flattened
parameter gradient and scales every gradient by the same bounded factor.

### 7. GPU-resident optimizer and horizon

`dynamic_adam_scalars` derives learning-rate progress from loop iteration or
from an accepted-update counter when target rows are masked. It emits constant,
linear, cosine, or exponential finite schedules, plus warmup. Unbounded runs
use a saturating integer step and require constant post-warmup rate.

`adamw_update` emits one elementwise update per parameter with exact AdamW beta
powers, epsilon, weight decay, optional update gating, and explicit recurrence
alias rules. Updated parameter, first moment, and second moment tensors are
external outputs. `OptimizerProgressState` records the update gate, accepted
counter, beta powers, update kernels, and bounds. For a complete known training
partition, `accepted_updates_per_epoch` is one; a masked partition may gate an
epoch update to zero when no target row is known. There is no loop-time host
transfer for progress, parameters, or metrics.

`TrainingHorizon::Finite(n)` creates exactly `n` training loop iterations,
optionally followed by finite calibration iterations. `Unbounded` has no
synthetic terminal iteration and can return only after the execution boundary
accepts an explicit stop request. The compiler rejects unbounded calibration,
nonconstant unbounded decay, warmup outside the declared horizon, and int32
schedule overflow.

### 8. Validation and calibration

Validation is mutually exclusive by family. Binary BCE/focal validation emits
mean BCE, accuracy, AUROC, AUPRC, Brier score, expected calibration error, and
requested recall thresholds. Multi-target binary metrics are computed per
column and averaged; temperature scaling is rejected for that topology.
Categorical validation emits mean cross entropy and top-one accuracy.
Regression validation emits R2 over the complete known validation partition.

Validation with no known rows is represented as
`ValidationMetricStatus::Unavailable::NoKnownTargets`; a binary validation
partition containing only one known class is represented as
`SingleKnownClass`. The root caller reports that status rather than pretending
that a metric was measured. Optional binary temperature scaling adds a bounded
finite post-training loop, validates positive strictly ordered bounds, and
exports the final temperature tensor.

Each metric is a `TrainingMetricBinding` with a stable `MetricId`, value, and
iteration domain. `TrainingOutputs` retains the binding list and all state
needed by the checkpoint manifest.

## Native execution lifecycle

### Admission image construction

`build_training_device_images` receives the finalized bundle's init manifests
and the compiler's owned external inputs. `pack_device_images` creates one
zero-filled image per finalized device, sorts members by offset and logical ID,
and copies only matching dtype/size inputs into their declared ranges. Gaps
remain zero. A logical input may occur in multiple device images, but it is
admitted only through each device's init image. Duplicate devices, duplicate
members, missing members, dtype/size mismatches, out-of-bounds offsets,
overlaps, and host-size overflow are typed failures.

### `init -> loop -> exit`

`prepare_and_execute_local_training_controlled`:

1. requires a stop source for an unbounded program;
2. calls `Preparer::prepare_program`, which performs measured placement,
   native compilation/realization, loading, warming, allocation, and bundle
   finalization before any user image is admitted;
3. rejects any finalized loop transfer whose source or destination is
   `External`;
4. verifies every planned external checkpoint output maps to one finalized
   exit transfer from the expected physical tensor;
5. hands the warmed validated native session to `PreparedRun`, initializes the
   device images, and starts the loop;
6. polls with bounded backoff, drains user metrics, and samples the stop flag
   only at completed loop-iteration boundaries; and
7. enters `exit`, drains remaining metrics/mailbox samples, joins physical exit
   images to logical output IDs, and returns only after native resources have
   been torn down.

The ordinary and observer entry points are thin calls into this controlled
path. The execution loop has no API for external data ingress or egress. Init
admission and exit egress are transfers, while loop work is calculation and
device-local transfer only.

`TrainingMetricSample::epoch` is one-based for users; the raw
`zero_based_iteration` remains available only for correlation with immutable
schedules. `FinalTrainingMetric` retains the newest sample by sequence for
every statically planned user metric. The live observer uses independent
per-metric cadence counters and never backpressures executor polling.

`TrainingExecutionEvidence::complete` derives counts from the actual completed
journal. It joins compiled optimizer update kernels and non-GPU calculation
tasks with physical submission records, and counts started/completed loop
iterations. A report is therefore post-exit evidence, not a success flag
constructed before teardown.

Inference uses the same lifecycle with stricter boundaries: the program and
bundle must have exactly one loop iteration, loop transfers to/from `External`
are rejected, user metrics are rejected, and prediction images are validated
after exit for task identity, source location, dtype, shape-derived byte count,
and non-overlap. KNN maps all declared output tasks in declaration order.

## Dense checkpoint and resume flow

### Manifest and versions

`CheckpointManifest` contains no row payload. It stores vector schemas and
metadata, feature spans and normalization mask, target identities/task,
effective topology, config/bounds, fitted normalization tensors, optional
temperature, canonical program digest, and optional native realization
identities. `CheckpointManifest::from_compiled` chooses the semantic format
version from topology:

| Version | Canonical addition |
| --- | --- |
| 5 | Flat layer-only dense checkpoints. |
| 6 | Structured residual topology. |
| 7 | Maximum-pool topology and winner contract. |
| 8 | Canonical structured topology and native realization metadata. |
| 9 | Convolution geometry and parameter images. |
| 10 | K-means declarations and centroid tensors. |
| 11 | Ordered multi-target task semantics. |
| 12 | Terminal supervised tree ensembles. |
| 13 | Leading embedding and causal attention. |
| 14 | Leading vanilla RNN. |
| 15 | Leading reset-before GRU. |
| 16 | Leading zero-cell LSTM. |

Every learned parameter is represented by a parameter image containing the
updated value and both AdamW moments. Non-parameter state includes K-means
centroids, tree split features/thresholds, pool geometry, normalization
tensors, and the final calibration temperature. Native bytes are not embedded
in semantic OGDL; only target, toolchain, digest, program, realization,
topology, and discovery identities are retained.

### Decode and validation

`decode_checkpoint` enforces source, node, vector, span, layer, metadata,
tensor-rank, tensor-byte, and aggregate-payload limits before strict graph
decoding. It validates the single `recipe` root, canonical fields, versioned
flat/structured topology, feature spans, target/task compatibility, model
geometry, normalization state, finite/nonnegative tensor constraints where
required, PReLU counts, tree/pool/K-means routing, and native identity/backend
format. A valid but noncanonical OGDL document is rejected by the decoder.

`CheckpointArtifact` is native-handle-free and exposes schemas, effective
blocks, parameters, moments, normalization, bounds/config, native metadata, and
multiclass dictionary decoding. The artifact can be encoded again only after
the same semantic invariants pass.

### Resume admission

`apply_checkpoint_resume` compiles the new declaration first, builds a fresh
manifest, and requires exact compatibility of feature width/spans and
normalization mask, ordered vector schemas, targets/task/output adapter,
objective/normalization/AdamW semantics, effective topology, and every
parameter/moment tensor contract. It then fills the already compiled
`OwnedExternalInput` images:

- `ResumeEnabled` becomes I32 one;
- every parameter, first moment, and second moment receives its saved bytes;
- K-means centroids and tree split arrays receive their saved tensors; and
- every expected resume input must be admitted exactly once.

The graph, IDs, schedule declaration, and native contract are not rewritten.
The new declaration owns the new phase and its horizon. Saved model and moment
tensors are restored, but exact optimizer progress across a prior phase is
still listed as `UnsupportedTrainingFeature::ExactOptimizerResume`; loop
shortening is likewise listed as `DynamicLoopShortening`.

### Completed save

`CompletedTrainingCheckpoint::new` can succeed only after native exit and
teardown. It authenticates the realized image set, maps every manifest tensor
to exactly one post-exit image, and rejects missing, duplicate, unexpected,
wrong-dtype, or wrong-size outputs. `save` writes canonical semantic OGDL via
the atomic-save helper. `save_native_kernel` selects exactly one matching
realized image by format, rejects unavailable or ambiguous images, checks the
extension, and writes the raw native bytes without wrapping them in OGDL.

At the public `Train` boundary, `.save(...)` is optional. A literal one-path
save selects `.ogdl`, `.cubin`, or `.hsaco` by extension; the literal two-path
form saves both. `.resume(...)` always requires a first `.ogdl` model path and
may take a second native path. A missing resume model or kernel is an
existence-conditional fresh/recompile path; an existing native image must
match the saved program, measured topology/discovery, current target, and
toolchain before it is used.

## Target-free inference pipeline

### Strict model dispatch and schema binding

`load_semantic_model_file` snapshots one bounded regular file, probes only its
first root line, and dispatches exactly one decoder: `recipe` to dense
checkpoint, `recipe-knn-model` to KNN, or `recipe-bayes-model` to Bayesian.
Unknown roots, non-UTF8 input, syntax errors, limits, duplicate/unknown
fields, and noncanonical values fail closed. No decoder fallback is attempted.

`prepare_checkpoint_inference_table` derives a schema from saved feature spans
and vector metadata, allows source columns to be reordered or unrelated
columns to be present, and does not require target columns. The saved feature
lowering and dictionaries remain authoritative. `PreparedInference` retains
raw schema-bound values; no host one-hot expansion, numeric conversion,
normalization, or model calculation happens before graph compilation.

### Dense inference graph

`compile_prepared_inference` rejects empty rows/blocks, checks the saved feature
spans, emits raw feature inputs, performs I32-to-F32 conversion and categorical
one-hot expansion in the graph, applies saved normalization, traverses the
canonical `blocks` list exactly once, validates every saved parameter shape,
and emits the same effective block equations used for training. It applies a
saved temperature when present and translates final logits/values into the
typed prediction kind:

- stable sigmoid for binary and multi-target binary probabilities;
- stable row-wise softmax for multiclass and joint target probabilities;
- raw values for scalar and multi-target regression; and
- saved output-adapter and task width checks before finalization.

The graph has one loop iteration, marks only raw inputs and the prediction as
external, canonicalizes through OGDL, and becomes `CompiledInference`. AdamW
moments are never admitted to this graph.

### KNN inference

`compile_prepared_knn_inference` rejects post-KNN operations because one
numeric transform cannot be applied coherently to independently typed outputs.
It emits query and reference feature lowering, derives normalization statistics
from the retained reference image for Z-score/min-max/L2 policies, admits one
known mask and value image per target, and appends the owned KNN all-output
materialization. Numeric targets return F32 means; categorical, ordinal,
temporal, text, binary, and image targets return I32 mode codes with a saved
label decoder. The result has one output contract per declared target and one
loop iteration.

### Bayesian inference

`compile_prepared_bayes_inference` emits one native conditional per saved
observed target, using query parent codes, reference parent/child codes,
mixed-radix multipliers/cardinalities, and the fixed Laplace-one smoothing
contract. Repeated conditionals are concatenated into one row-major F32 output;
adjacent class ranges follow declaration order. The artifact stores observations,
not host-computed counts, so histogramming and posterior calculation remain
native payload work.

### GGUF llama inference

`decode_gguf_llama` accepts only bounded little-endian GGUF v3 archives with
ordinary dense-F32 `llama` metadata: equal query/KV head counts, full even-head
RoPE, causal attention, RMSNorm, and parallel SwiGLU disabled. Mixture of
experts, grouped-query attention, noncausal attention, unsupported RoPE
scaling/YaRN, quantized tensors, invalid dimensions, missing tensors, and
nonfinite factors fail with a typed `GgufLlamaErrorKind`.

`prepare_gguf_llama_inference_table` consumes one ordered token stream, checks
UTF-8/int32 tokens, vocabulary range, nonempty input, and context length.
`compile_prepared_gguf_llama_inference` admits token IDs and every execution
tensor, builds RoPE partner/cosine/signed-sine tables, then emits embedding,
RMSNorm, Q/K/V projections, causal attention, residuals, SwiGLU, final norm,
and all-position vocabulary logits. The output kind is `TokenLogits` and the
program is one immutable inference iteration.

## Bayesian reference preparation and artifact

`bayes.rs::resolve_bayesian_schema` resolves declaration names against prepared
vectors. Nodes receive stable IDs in ascending name-byte order; declarations
remain repeated-call order; parent lists preserve declaration order; and a
separately derived deterministic topological order is exposed. It rejects
missing nodes, duplicate names/source identities, invalid role edges, latent
nodes for the observed executable slice, and dependency cycles.

`prepare_categorical_bayesian_reference_sets` requires at least one declaration,
a nonempty training partition, every child to be a declared target, every
parent to be a feature, all nodes to be dictionary-encoded categorical vectors,
and target source indices to equal declaration order. It derives one reserved
unseen route for every parent cardinality, checked mixed-radix multipliers and
configuration count, and exact row-ordered I32 parent/child observations.
Missing or unknown observations are rejected for this observed instrument;
there is no implied latent or ancestral posterior.

`BayesModelArtifact::new` writes one conditional as format v1. `from_conditionals`
writes repeated declarations as format v2. `continue_with` requires identical
schemas/order/smoothing and appends source rows and observations without
deduplication. `encode`/`decode_bayes_model` use canonical `recipe-bayes-model`
OGDL with finite node/conditional/parent/label/row/payload limits. Every
conditional must share the same ordered reference partition, child identities
must be unique, repeated schemas must agree, and a child cannot be another
conditional's parent.

## KNN reference preparation and artifact

`prepare_knn_reference_set` lowers the complete prepared training partition to
F32 feature bits using the dense feature plan. It retains source-row order,
feature spans, optional categorical normalization mask, and exact reference
features. It emits one `KnnReferenceOutput` per declared target:

- finite numeric I32/F32 targets become F32 values plus a known mask and a
  known-reference count;
- categorical and ordinal dictionary codes retain their labels;
- temporal and variable-width text/binary/image values are deterministically
  remapped to a sorted I32 dictionary; and
- missing targets remain row-aligned but are excluded by the per-output known
  mask.

Every output needs at least one known reference. Labels are unique, nonempty in
their source dictionary, and bounded by the I32 calculation code domain. The
prepared set has no optimizer, objective, or native artifact.

`KnnModelArtifact::new` writes format v1. `continue_with` requires equal
neighbor count, normalization, post-reduction operations, row-free schemas,
feature spans/mask/width, and output declarations, then appends rows and
features. Discrete dictionaries are merged deterministically and current codes
are remapped. `encode`/`decode_knn_model` use canonical `recipe-knn-model` OGDL
with finite source/node/vector/span/row/feature/label/output/payload limits.

## Failure taxonomy and invariants

### Compile and preparation failures

`TrainingCompileErrorKind` distinguishes empty data, inconsistent rows,
invalid feature/target matrices, invalid networks/optimizers, unsupported
extents, arithmetic or ID exhaustion, and wrapped ingest/language/operation/
program/OGDL failures. Details identify the vector, source row, block, task,
shape, or scalar contract that failed. The Bayesian and KNN preparers reuse
these typed kinds rather than returning a partially prepared artifact.

`InferencePreparationError` distinguishes bounded checkpoint source/decode,
GGUF, inference-table preparation, inconsistent checkpoint schema, and checked
arithmetic failures. `InferenceCompileErrorKind` adds empty datasets,
unsupported topology, checkpoint inconsistency, extents, arithmetic/identity,
language, operation, program, and OGDL failures.

### Checkpoint failures

`CheckpointError` covers strict decode/path errors, invalid manifests,
incompatible resume contracts, duplicate/missing/unexpected outputs,
output dtype/size mismatch, unavailable or ambiguous native images, invalid
targets, insufficient capacity, and atomic-save I/O. Decode errors carry a
stable `CheckpointPath` and `CheckpointDecodeErrorKind` such as limit,
syntax, UTF-8, missing/duplicate/unknown field, invalid value, or inconsistent
value.

### Execution failures

`TrainingExecutionError` covers preparation/native handoff/executor failures,
all init-image identity/size/overlap errors, loop external transfers, output
mapping errors, invalid bounds, missing stop control for an unbounded run, and
failure to reach terminal loop state. `InferenceExecutionError` additionally
carries recoverable `InferenceRunFailure` evidence for post-handoff executor
or post-exit prediction-validation failures, exactly-one-iteration checks,
unbound inputs, user metrics in an inference bundle, missing/duplicate/
unexpected prediction tasks, output source/dtype/size mismatches, and image
overlap.

The active-state invariants are intentional:

- only calculations and transfers are model work; dependencies, routes,
  queues, synchronization, and lifecycle phases do not become new model kinds;
- all train data is admitted in init, all public model outputs leave in exit,
  and the loop has no external data transfer;
- one epoch is one logical update over the complete prepared training partition,
  with no user-facing partial-batch control;
- GPU payload calculations use F32/I32 storage and finite checked values;
- the active backend state defines valid events, so impossible event branches
  are not registered as application behavior;
- native realization identities authenticate a reused kernel against the
  canonical program, measured topology/discovery, target, and toolchain; and
- public artifacts are semantic `.ogdl` and optional realized `.cubin` or
  `.hsaco`, never journals, plans, caches, profiles, or intermediate files.

`model.rs::REMAINING_UNSUPPORTED` explicitly lists only
`DynamicLoopShortening` and `ExactOptimizerResume`. The implementation does
not claim those behaviors. Other unsupported cases fail at their real
boundary, rather than selecting a fallback model, retrying a broken transition,
or silently dropping a declaration.

## What a complete user run proves

For dense training, a successful `Train::run` has traversed declaration
validation, bounded source preparation, typed lowering, graph validation and
canonicalization, optional semantic resume admission, measured native
preparation and realization, init image upload, the complete loop, exit
egress, metric drain, native teardown, and optional artifact writes. The
returned `TrainingReport` is the joined semantic/native/evidence snapshot.

For KNN or Bayesian declarations, success proves semantic reference preparation
and optional `.ogdl` persistence; there is no optimizer loop or native
training-kernel report. Target-free inference success similarly proves strict
model/schema binding, one-iteration graph compilation, native preparation,
loop completion, exit output validation, and teardown through the corresponding
`Completed*InferenceExecution` value.

Cargo compilation and graph canonicalization are structural checks. The
repository's hardware acceptance runner remains the authority for measured
correctness, performance, native lifecycle, and complete training behavior on
real CUDA or HSA systems.
