# Inference

This document describes the target-free inference instrument as it exists in
the workspace. Inference is a complete `data -> saved model -> immutable graph
-> measured native bundle -> init -> loop -> exit -> report` path. The public
builder records intent only. Filesystem reads, schema application, graph
construction, native compilation, allocation, and execution happen after the
declaration is resolved.

Inference is not training with a different number of epochs. It never fits a
schema, reads target values, emits user metrics, or admits external data during
the loop. A saved semantic model supplies the feature schema, parameter bytes,
normalization state, topology, target interpretation, and output decoding
metadata. The native loop executes that fixed program once for the complete
query table.

## Source map

The end-to-end implementation is split across these boundaries:

| Boundary | Implementation | Role |
| --- | --- | --- |
| Public declaration | `src/facade.rs`, `src/api.rs` | Stores the immediately preceding `Data` and `Model` declarations and builds `Infer`. |
| Public orchestration | `src/inference.rs` | Validates the target-free policy, loads the model, distills the source, dispatches model-family preparation and compilation, enters native preparation, and writes the report. |
| Source distillation | `src/data_prepare.rs`, `ingest/src/dataset.rs`, `ingest/src/table.rs` | Reads bounded sources into one `RawTable`; target-free selection applies only declared column and row exclusions. |
| Saved feature binding | `ingest/src/inference.rs` | Matches exact saved feature names, preserves source-vector identities, and encodes new values as I32 or F32 bits. |
| Semantic artifacts | `training/src/checkpoint.rs`, `training/src/knn_checkpoint.rs`, `training/src/bayes_checkpoint.rs`, `training/src/gguf_llama.rs` | Decodes and validates dense, KNN, Bayesian, or supported GGUF llama state. |
| Graph compiler | `training/src/inference.rs`, `training/src/forward.rs` | Emits `CalculationGraph` tensors and primitive/composition nodes, then wraps them in `StaticCalculationProgram`. |
| Candidate planning and preparation | `prepare/src/lib.rs`, `prepare/src/production.rs`, `planner/src/planner.rs` | Resolves artifacts, enumerates placements, realizes and warms a candidate, measures capacity, and finalizes one immutable bundle. |
| Native lifecycle | `training/src/execute.rs`, `executor/src/executor.rs` | Packs init images, runs the one-iteration lifecycle, validates exit egress, and retains evidence after teardown. |
| Public result | `src/inference.rs` | Exposes `InferenceReport`, typed prediction bytes, model-family decoders, journal, native evidence, and device/time output. |

`training/src/lib.rs` re-exports the preparation, compilation, execution, and
artifact types. The root crate re-exports the public `compile_inference`,
`InferenceReport`, `InferenceModelKind`, and `InferenceError` surfaces.

## Public declaration and resolution

`Recipe` is a zero-sized public facade. `recipe.data(...)` creates a `Data`,
records it in thread-local `RECIPE_SEQUENCE`, and returns the builder. Each
`Data` mutator records the latest clone. `recipe.model()` creates and records a
`Model`; `Model::load(path)` records a checkpoint source and rejects an empty
path, an inline topology mixed with a checkpoint, or a second source. The
builder does not read the path.

The user-facing call is:

```rust
recipe.data("query.csv").exclude("target");
recipe.model().load("model.ogdl");
let report = recipe.infer().log([Time, Device]).evaluate()?;
```

`Recipe::infer()` returns a fresh `Infer`. `Infer::log` accepts one or more
`LogItem`s. `Time` and `Device` are the only legal inference log items. Loss,
accuracy, regression and calibration metrics need targets; epoch and learning
rate need training state. The first invalid item is retained as a deferred
declaration error and is reported by `Infer::resolve_declaration`.

`Infer::evaluate` resolves the declaration and then calls
`evaluate_inference_declaration`:

1. `take_recipe_inference_sequence` consumes the preceding data/model pair. A
   missing data or model declaration is an `InvalidInferenceConfiguration`.
2. The `Infer`, `Data`, and `Model` declarations are validated.
3. `compile_inference_declaration` builds a `CompiledInferencePackage` without
   native probing, device allocation, or execution.
4. `execute_current_inference_native` performs native preparation and one
   complete native run.
5. The model artifact and completed execution are joined into a family-specific
   `InferenceReport`.
6. `write_inference_report` prints requested time/device records and the exact
   prediction rows only after native teardown.

The lower-level `compile_inference(policy, data, model)` entry point performs
steps 2 and 3 and returns only the immutable compiled program. It is useful to
callers that own preparation and execution; it does not perform either one.

## Target-free data policy

`compile_inference_package` enforces three boundaries before reading model or
data bytes:

- `Data::targets()` must be empty. Target interpretation comes from the saved
  model, not from a second declaration.
- `Data::split(...)` is rejected. Every selected query row is evaluated.
- `Data::norm(...)` is rejected. Dense and KNN normalization is fixed by saved
  model state or by the KNN graph's reference calculation.

`distill_data` is the shared bounded source boundary. It validates that at least
one source exists, applies the finite default ingest limits, and calls
`distill_datasets`. It preserves source order and rectangular table semantics,
but does not perform training target selection, semantic fitting, splitting,
or normalization. `select_target_free_data` then applies only public column
patterns and row conditions through `select_table`. The result is a `RawTable`.
No target column is needed or implicitly removed; a caller normally excludes
the saved training targets explicitly, as in the example above.

## Model sources and semantic artifacts

The model source must have one of the exact `.ogdl` or `.gguf` extensions.
Other extensions, a missing extension, and a model with no `load` declaration
fail as unsupported inference declarations. A `.cubin` or `.hsaco` is a native
kernel artifact, not an inference semantic model, and is not accepted by this
entry point.

### Semantic `.ogdl`

`load_semantic_model_file` reads one bounded regular-file snapshot. It examines
only the first line to select a strict decoder, and the decoder then validates
the complete OGDL document, version, canonical fields, payload sizes, topology,
and semantic invariants. The recognized roots are:

- `recipe`: a dense `CheckpointArtifact`.
- `recipe-knn-model`: a `KnnModelArtifact` containing saved references and
  output dictionaries.
- `recipe-bayes-model`: a `BayesModelArtifact` containing observed categorical
  conditionals and exact code/dictionary state.

The result is `SemanticModelArtifact::Dense`, `::Knn`, or `::Bayes`. Dispatch is
strict: a decoder never falls back to another family when a root or field is
wrong.

### Dense checkpoint artifact

`CheckpointArtifact` is a validated, native-handle-free semantic image. It
retains the source vector schemas and roles, contiguous `CompiledFeatureSpan`s,
the dense feature width, target source identities and calculation dtypes,
`DenseTask`, optional output adapter, model configuration, fitted data
normalization tensors and mask, effective topology in `blocks`, optional
temperature, and optional native realization identities. Optimizer moments are
not an inference input. The effective block list is authoritative for every
checkpoint version: flat legacy layers and structured blocks are traversed
without flattening pool or residual structure.

When present, `DenseOutputAdapter` describes the synthetic final operation-free
linear block that joins a preceding source width to the task output width. The
decoder validates that relationship, and inference traverses that synthetic
block in the same ordered `blocks` list as every other effective block.

The supported block image variants are `Embedding`, `Attention`, `Rnn`, `Gru`,
`Lstm`, `Layer`, `Convolution`, `Pool`, `KMeans`, `Tree`, and `Residual`. Each
parameter image is typed as F32 with an exact shape and byte count before it can
enter the graph.

### KNN artifact

`KnnModelArtifact` stores a `KnnReferenceSet`, optional saved data normalization,
and post-reduction operation declarations. The reference set contains feature
schemas and spans, the prepared reference matrix as F32 bit patterns, source
row order, optional normalization mask, neighbor count, one independent output
per saved target, known-reference masks, and either F32 numeric reference values
or I32 class codes plus exact `KnnLabelValue` dictionaries. The current
inference compiler deliberately rejects nonempty post-KNN operations because a
single transform is not defined for heterogeneous numeric and discrete output
types.

### Bayesian artifact

`BayesModelArtifact` stores one observed categorical conditional in format
version 1 or two or more repeated conditionals in version 2. It retains the
ordered reference rows, parent and child dictionaries, parent codes, child
codes, parent cardinalities and mixed-radix multipliers, and the fixed
Laplace-one smoothing contract. It does not store a host-computed histogram or
opaque native state. The histogram and posterior are reconstructed as device
calculations for each query.

## Schema-bound inference preparation

`ingest::prepare_inference_table` applies the saved model schema to the selected
`RawTable` without refitting anything. It requires a nonempty schema, unique
saved names, unique source-vector identities, nonempty names, and canonical
ascending nonempty categorical dictionaries.

Feature lookup is exact byte equality on column names. Source columns may be
reordered and unrelated columns are ignored. Each required column must occur
exactly once. Every source row must contain the resolved column.

The resulting `PreparedInferenceDataset` contains a row count and one
`PreparedInferenceFeature` per saved feature. Values are retained as:

- `PreparedInferenceValues::I32` for saved numeric I32 and categorical
  dictionary encodings;
- `PreparedInferenceValues::F32Bits` for saved numeric F32 encodings.

Numeric values must be present, UTF-8, finite where required by the contract,
and exactly parseable as the saved type. A known categorical label uses its
saved dictionary code. Missing or unseen categorical labels use the saved
reserved code, while a parallel `CategoricalObservation` route records whether
the value was missing, known, or an unseen nonempty label. This distinction is
preserved for callers that need it, but the graph consumes the exact I32 codes.

The preparation errors are path-addressed as
`inference.feature[...].source-vector[...].column[...]` and optionally
`.source-row[...]`. Missing and duplicate columns, invalid values, malformed
dictionaries, and row-length or arithmetic mismatches fail closed.

Dense and KNN preparation use the saved feature spans and call this same
schema boundary. Bayesian preparation builds a union of all observed parent
schemas in first occurrence order, so shared parents are parsed once. GGUF
llama preparation uses a separate one-column token-stream contract described
below.

## Compiled program representation

The compiler creates an `InferenceGraphCompiler` with deterministic
`ValueId` and `KernelTemplateId` counters starting at one. It owns:

- a `BTreeMap<ValueId, Tensor>` of exact dtype, shape, contiguous row-major
  storage contracts;
- ordered `CalculationNode`s containing `PrimitiveKernel`s;
- one `KernelIterationDomain` per emitted kernel;
- external input descriptors and a set of their value identities.

`external` checks that the supplied little-endian bytes exactly equal
`shape.bytes(dtype)`, marks the tensor as an external input, and records its
semantic `InferenceInputRole`. `external_checkpoint_tensor` applies the same
contract to a saved parameter image. Intermediate tensors are not host inputs.
All calculation inputs and outputs use forbidden input/output alias rules unless
an explicitly owned primitive requires another contract.

The compiler emits direct `PrimitiveKind` nodes for elementwise programs,
reductions, gathers, scatters, index maps, and contractions. Structured
operations call the Recipe operation registry and
`materialize_composition`; the compiler reserves a bounded value/kernel identity
range, inserts the returned tensor contracts and nodes, and assigns the first
iteration domain. No training-only wrapper or alternate implementation is
introduced.

Every completed family compiler validates the graph, serializes it to canonical
OGDL, decodes it again, creates a one-iteration `StaticCalculationProgram`,
serializes the program, and decodes it again. The round trip is an executable
canonicality check, not a model artifact export. A static program is one acyclic
calculation graph plus an explicit domain for every kernel. Inference always
uses `LoopIterations::ONE` and `IterationDomain::first()`.

`CompiledInference` contains the static program, all typed external inputs, one
`InferenceOutputContract`, row count, `InferenceTask`, and an optional dense
output adapter. `CompiledKnnInference` contains the same program and inputs,
but one `KnnInferenceOutputContract` per declared target. The contracts preserve
logical `ValueId`, dtype, shape, source target dtype or source-vector identity,
and semantic output kind.

### External input roles

`InferenceInputRole` is the compiler's semantic inventory. The roles are
grouped as follows:

- query features and normalization mask/statistics;
- dense layer weight, bias, PReLU, temperature, embedding table, attention
  projections, convolution and pooling tables, KMeans centroids, tree split
  features/thresholds/leaf values, and residual projection/PReLU values;
- RNN, GRU, and LSTM gate parameters;
- KNN reference features, values, and known masks;
- Bayesian query/reference codes, parent cardinalities/multipliers, and
  concatenation index/select tables;
- GGUF token IDs, model tensors, and RoPE partner/cosine/signed-sine tables.

The execution boundary accepts exactly this complete role set and rejects a
duplicate role or a role that does not match its family. Every graph external
input must have one descriptor, and every descriptor must name one graph input.

The `recipe-training` crate also exposes composable lower-level calls for
callers that already own a distilled dataset: `load_checkpoint_file`,
`load_knn_model_file`, `load_bayes_model_file`, and
`load_semantic_model_file` decode bounded artifacts; the checkpoint, KNN, and
Bayesian `load_and_prepare_*` functions decode and schema-bind in one call;
`prepare_*_inference_table` binds an already admitted `RawTable`; and the four
`compile_prepared_*_inference` functions emit the dense, KNN, Bayesian, or GGUF
program. None of these calls probes hardware or executes a kernel. The two
`prepare_and_execute_local_*` functions are the only training-crate boundaries
that cross from a compiled program into the native executor.

## Dense graph compilation

`compile_prepared_inference` first requires at least one effective block, a
nonzero row count, and a nonzero saved feature width. It emits the feature
matrix, applies saved input normalization, walks `CheckpointArtifact::blocks()`
in order, checks each resulting width against the next block, and finishes with
the saved task interpretation.

### Feature lowering

For an ordinary dense input, `compile_features` verifies one prepared feature
per saved span, contiguous nonempty spans, source-vector identity, and row
length. Numeric I32 features are converted to F32 by an elementwise checked
round-trip conversion. Numeric F32 values retain their bits. Categorical I32
features are expanded to dictionary width plus the reserved route by a device
one-hot operation. Blocks are scattered into a zero-filled flat matrix, then a
device index map and gather produce the canonical `[rows, feature_width]` F32
matrix.

If the first block is an embedding, `compile_token_features` instead requires
one I32 scalar span per token position. It checks every token against the saved
vocabulary, packs positions into the fixed sequence matrix, and leaves the
matrix I32 for the embedding gather.

### Saved data normalization

`apply_data_normalization` never recomputes dense training statistics. Identity
requires no saved normalization tensors. Z-score requires exactly mean and
variance tensors; min-max requires exactly minimum and maximum tensors; L2
requires no fitted tensors. The saved feature normalization mask, tensors, and
epsilon enter the graph as external inputs. Z-score, min-max, and L2 arithmetic
is emitted as F32 elementwise and reduction work, with masked values retaining
their unnormalized input route.

### Effective blocks

The compiler's block methods and their contracts are:

- `compile_embedding`: validates an I32 token matrix and F32 table, gathers one
  row per token, and repacks `[rows, sequence, dimensions]` into the saved flat
  width.
- `compile_attention`: requires the saved sequence length, channel count, head
  geometry, and F32 query/key/value/output matrices. It projects the sequence,
  contracts query and key, scales scores, applies causal softmax, contracts
  probabilities with value, restores sequence-major order, and applies the
  output projection.
- `compile_rnn`, `compile_gru`, and `compile_lstm`: validate exact sequence and
  gate parameter shapes, admit F32 parameters, and delegate recurrent equations
  to `forward::lower_rnn_sequence`, `lower_gru_sequence`, and
  `lower_lstm_sequence`. The shared forward boundary starts zero hidden state
  (and zero cell state for LSTM), gathers each sequence column, emits gate
  projections and activations, and returns the final hidden state.
- `compile_layer`: admits an F32 weight and bias, materializes
  `gpu_linear_into`, then applies the saved operation order. Activation lowering
  uses owned operation symbols or canonical scalar programs; PReLU requires one
  saved scalar per ordered occurrence. Layer and batch normalization are device
  reductions followed by F32 normalization programs.
- `compile_convolution`: validates logical sequence/channel geometry, prepares
  channelwise 1D window indices, gathers windows, contracts them with the
  saved weight, adds the saved bias, admits PReLU state when required, and
  applies saved operations.
- `compile_pool`: validates grouped pool geometry and winner contracts, admits
  window and winner-base tables, materializes the pool operation, and unpacks
  grouped output back to the canonical row matrix.
- `compile_kmeans`: admits centroids and materializes the saved pairwise L2
  distance calculation, yielding one F32 distance per cluster.
- `compile_tree`: admits split feature/threshold and leaf-value images and
  materializes the complete tree ensemble inference operation.
- `compile_residual`: compiles each saved branch layer and operation, checks
  branch PReLU scalars, uses an identity skip only for equal widths, otherwise
  admits the exact projection matrix, adds branch and skip with equal dtype and
  shape, and applies output operations.

After the block walk, the final width must equal `DenseTask::output_width`.
Optional saved temperature is an external F32 scalar and divides the logits in
the graph. No host-side model transform is performed.

### Dense prediction interpretation

`compile_prediction` turns the final F32 matrix into the one external output:

| Saved task | Device calculation | `InferencePredictionKind` | Output shape |
| --- | --- | --- | --- |
| Binary classification | Stable sigmoid | `BinaryProbability` | `[rows, 1]` |
| Multiclass classification | Row maximum, shift, exponent, sum, divide | `MulticlassProbabilities` | `[rows, class_count]` |
| Scalar regression | Identity | `Regression` | `[rows, 1]` |
| Multi-target binary classification | Stable sigmoid per target | `MultiTargetBinaryProbabilities` | `[rows, target_count]` |
| Joint multiclass classification | One row softmax over joint targets | `JointTargetProbabilities` | `[rows, target_count]` |
| Multi-target regression | Identity | `MultiTargetRegression` | `[rows, target_count]` |

Stable sigmoid uses `exp(-abs(logit))` and selects the positive or negative
formula by logit sign. Stable softmax subtracts each row maximum before
exponentiation and divides by the row sum. `InferenceOutputContract` always
uses F32 prediction bytes, while `target_dtypes` retains the saved source target
dtypes for report interpretation.

## Bayesian graph compilation

`prepare_bayes_inference_table` builds the physical feature schema from the
union of every conditional's parents. `compile_prepared_bayes_inference`
requires at least one query row and one conditional. For each conditional,
`compile_bayes_conditional`:

1. Verifies reference rows, parent count, parent configurations, and child
   class count fit the checked integer domains.
2. Resolves every prepared parent by source identity and exact name, requires
   dictionary-coded I32 values, and validates each query code against its saved
   cardinality.
3. Admits reference parent codes, reference child codes, query parent codes,
   parent multipliers, and parent cardinalities as external I32 tensors.
4. Allocates an F32 `[rows, child_classes]` output and asks
   `recipe-ops` for the checked categorical Bayesian requirements.
5. Appends the native categorical Bayesian composition, which performs
   mixed-radix lookup, histogramming, and Laplace-one posterior calculation on
   the device.

The resulting probability matrices are concatenated in repeated declaration
order by device index/select tables and `gpu_concat_into`. The final output is
one F32 `[rows, total_class_width]` matrix with adjacent class ranges per
conditional, `BayesProbabilities` as its kind, and one I32 target dtype per
conditional. `InferenceReport` uses the saved child dictionaries to expose
output names, class ranges, labels, and per-conditional argmax rows.

The executable Bayesian slice is observed categorical inference. Declared
children are saved targets and every parent is a saved categorical feature. A
target child cannot silently become evidence for another output, and latent
nodes are not inferred by this instrument.

## KNN graph compilation

`prepare_knn_inference_table` applies the saved KNN feature schema and validates
the saved spans. `compile_prepared_knn_inference` rejects any post-KNN operation,
requires nonzero query/reference rows and feature width, and then:

1. Compiles query features with the same numeric and categorical lowering used
   by dense inference.
2. Admits the saved F32 reference feature matrix.
3. Applies optional normalization to query and reference features in the graph.
   Z-score computes reference sums, means, centered squares, variances, and
   masked transforms on the device. Min-max computes reference extrema and
   masked transforms. L2 computes masked row norms and normalized values. The
   current KNN path uses a fixed positive epsilon for these runtime formulas.
4. For every target, admits a known-reference mask and either F32 reference
   values or I32 reference codes. Numeric output tensors are F32; discrete
   output tensors are I32.
5. Builds one `KnnOutputRequest` per target and appends
   `recipe-ops::append_knn_all_outputs`, which performs distance, deterministic
   neighbor selection, numeric mean, or discrete mode on the device.

The program exposes one exit tensor per saved target. Each
`KnnInferenceOutputContract` records its source-vector identity, `[rows, 1]`
shape, dtype, and `NumericMean` or `DiscreteMode` kind. The report preserves
target declaration order and decodes discrete codes through the saved
`KnnLabelValue` dictionaries. KNN has no singular `prediction()` payload;
callers use `knn_predictions()`.

## GGUF llama graph compilation

`.gguf` is a separate model source and is dispatched to
`load_gguf_llama_model_file`. The first concrete instrument is deliberately
small and fail-closed. `decode_gguf_llama` requires a version 3 little-endian
container, `general.architecture = llama`, dense F32 tensor images, equal query
and KV head counts, full even-head RoPE, causal attention, ordinary residual
blocks, and no mixture-of-experts or unsupported RoPE/parallel-residual
variants. It validates all required metadata and tensor names, shapes, and
finite parameter bytes. Unsupported architectures, quantized variants, missing
tensors, invalid metadata, or malformed containers return `GgufLlamaError`.

`prepare_gguf_llama_inference_table` requires a one-column selected table. Each
row may contain whitespace-separated decimal int32 token IDs. IDs must be
nonnegative, inside the saved vocabulary, and the complete stream must be
nonempty and no longer than the saved context. No tokenizer, chat template,
sampling, KV cache, or session state is inferred by this path.

`compile_prepared_gguf_llama_inference` admits token IDs and every execution
tensor, gathers token embeddings, prepares RoPE partner/cosine/signed-sine
tables, and emits each block as RMSNorm, Q/K/V projections, causal attention,
output projection, residual addition, RMSNorm, parallel SwiGLU gate/up/down
projections, and residual addition. A final RMSNorm and output contraction emit
raw F32 logits with shape `[positions, vocabulary]`, task
`TokenLogits`, and one I32 source dtype. The report prints the maximum-logit
token and its logit for each position, but the retained prediction bytes remain
the complete logits matrix.

## Preparation against the measured system

`execute_current_inference_native` creates a fresh run ID and enters
`with_current_native_preparation`:

1. The CLI's current native receipt identifies one exact measured profile. The
   profile is loaded through its identity-named cache path; a missing, stale,
   or mismatched profile is an error.
2. The native scope reopens the measured local GPU bindings and host inventory,
   verifies that the local device set exactly covers every measured calculation
   device, and constructs owned CUDA/HSA target build specifications.
3. Runtime tuning is derived from the compiled graph, measured profile, and
   host machine. A host backend configuration, staged cross-backend bridge,
   `LocalCandidateFactory`, `NativeExecutorDriver`, and deferred target
   compiler are created inside the scope.
4. A `NativeArtifactProvider` and `Preparer` receive the exact graph and
   profile. Handles and compiler inputs are scoped; they are not stored in the
   public declaration or semantic artifact.

`Preparer::prepare_program` is a fixed-point pipeline. It validates the measured
profile, obtains the mandatory reservation plan, resolves an artifact catalog,
enumerates finite planner candidates, realizes and warms each candidate at
maximum concurrency, validates stabilization capacity snapshots, packs arenas,
and calls `FinalizedBundle::finalize_with_loop_schedule`. A rejected candidate
is destroyed before the next candidate. A successful `PreparedSystem` keeps the
realized native session and the finalized bundle together.

The planner lowers every graph node to a calculation task, adds any required
fault readbacks, inserts phase/dependency barriers, schedules transfers and
queues, and adds external output egresses. For each external tensor it chooses
one non-overwritten resident copy, creates one `RunPhase::Exit` transfer to
`TransferEndpoint::External`, and records the exact logical-to-physical
`PlannedExternalOutput`. `FinalizedBundle` resolves every value to immutable
arena locations and embeds one `InitDataImage` manifest per device.

## Inference execution lifecycle

`prepare_and_execute_local_inference` and
`prepare_and_execute_local_knn_inference` share the same lifecycle; KNN differs
only in its multiple output contracts and collection routine.

### Preflight boundary validation

Before preparation and again before image packing, the execution layer checks:

- the program and finalized bundle execute exactly `LoopIterations::ONE`;
- the graph is valid and every kernel has a valid iteration domain;
- no program or finalized bundle emits user metrics;
- every external input role is legal and unique;
- every descriptor names a graph external input with the same dtype, shape,
  canonical contiguous row-major layout, storage byte count, and host byte
  count;
- the descriptor set equals the graph external-input set;
- dense/Bayesian/GGUF inference has exactly one F32 external output, while KNN
  has one valid output per unique target source identity;
- each output tensor is canonical, produced by a calculation node, not also an
  input, and has the expected shape and semantic kind.

Any mismatch is an `InferenceExecutionError` before native work starts.

### Init image admission

`build_inference_device_images` consumes the finalized bundle's init manifests.
`pack_inference_input_images` creates one zero-filled host image per manifest,
sorts members by image offset and logical value, and copies each exact external
input byte slice into its assigned range. It rejects duplicate devices,
duplicate members, undeclared members, unbound inputs, dtype or byte-size
mismatches, out-of-bounds ranges, overlapping members, and image sizes that do
not fit the host. A logical input may appear in more than one device manifest,
but each copy is admitted only in that device's singular init phase. Gaps and
fault-buffer bytes remain zero.

### PreparedRun state machine

After `Preparer` returns, execution consumes its bundle and warmed native
session, hands the session to the local backend, and calls
`PreparedRun::prepare_recoverable`. The executor allocates bounded journal
capacity from the finalized task graph, realizes the init, loop, and exit
phase state, binds resources, and records `Prepared`.

The only legal transitions are:

```text
PreparedRun
    --initialize(images)--> InitializedRun
    --start_loop()--------> RunningRun
    --poll until complete-> ExitedLoop
    --exit()--------------> ExitedRun
```

Initialization allocates the finalized arenas, performs the init admission,
and records `Initialized`. `start_loop_recoverable` starts iteration zero and
records `LoopStarted` and `LoopIterationStarted`. Inference then calls
`poll_with_progress` until `LoopStatus::Complete`. Polls are nonblocking; a
short bounded backoff is used only when no progress is reported. A premature
or failed loop is converted to a recoverable run failure.

For inference there is no next iteration. On completion, the executor records
`LoopIterationCompleted` and `LoopCompleted`, then `into_exited_loop` makes the
exit phase legal. `exit_recoverable` runs all planned exit transfers, releases
arenas, destroys native resources, and records `Exited`. The backend, exit
images, metrics mailbox, and bounded `RunJournal` are returned only after this
teardown succeeds.

`elapsed` starts immediately before loop polling and stops when the one loop
iteration reaches completion. It excludes data admission, model decoding,
graph compilation, candidate realization, arena allocation, output egress,
teardown, and the required warm pass. `CompletedInferenceExecution` retains
the run ID, bundle identity, prediction bytes, realized native kernel set,
native execution evidence, elapsed interval, and journal. Native resources have
already been destroyed when it is returned.

## Exit mapping and output bytes

The planner's `PreparedSystem::external_outputs` is the authoritative mapping
between an exit task, logical output value, selected device, and physical arena
copy. `map_inference_output` requires every planned task to be an exit transfer
to the external endpoint, checks the source device and physical value against
the finalized bundle, rejects duplicate or unexpected tasks and overlapping
source ranges, and requires exactly one dense/Bayesian/GGUF descriptor. It then
checks dtype and both planned and transfer byte counts against the output
contract.

KNN uses `map_knn_inference_outputs` to require exact set equality between
declared contracts and exit transfers. It validates each target dtype, shape,
source identity, physical location, byte count, and non-overlap, then orders the
descriptors by saved target order. `collect_knn_inference_predictions` copies
each exit image into one `KnnInferencePrediction`.

`collect_inference_prediction` performs the analogous single-output checks.
`InferencePrediction` stores the contract and a copy of the finalized exit
bytes. It does not reinterpret or calculate them on the host. The native graph
has already produced all prediction values.

## Report and printed output

`InferenceReport` joins the loaded artifact with the completed execution and
records the selected device origins. It exposes:

- `kind`, `run`, `bundle`, `elapsed`, `devices`, and `journal`;
- `prediction()` for dense, Bayesian, and GGUF reports;
- `knn_predictions()` for KNN reports;
- exact little-endian F32 values for singular F32 outputs;
- dense multiclass label decoding from the saved dictionary;
- Bayesian output count, name, class count, class range, and label decoding;
- KNN discrete-label decoding from saved `KnnLabelValue` dictionaries;
- realized native kernels for dense, Bayesian, and GGUF reports; KNN exposes
  native driver evidence but not a retained realization image through the
  report accessor.

`write_inference_report` prints only requested `time` and `device` records, then
family-specific prediction rows:

- binary rows contain `prediction`, row number, and `probability`;
- scalar regression rows contain `value`;
- multiclass rows contain row argmax `class`, saved `label`, and all
  probabilities;
- multi-target rows contain saved target names paired with values or
  probabilities;
- Bayesian rows contain one argmax and probability block for each conditional,
  with saved child labels;
- KNN rows contain each target's numeric value or discrete code and decoded
  label;
- GGUF rows contain each position's maximum-logit token and logit.

Formatting and argmax are report presentation. They do not replace graph
calculation, and they run only after native teardown and output validation.

## Failure boundaries

Failures remain attached to the stage that observed them. The root
`InferenceError` distinguishes declaration, data preparation, model loading,
graph compilation, native preparation, native execution, runtime report I/O,
and explicitly unsupported declarations.

### Declaration and data failures

Missing sequence entries, invalid log items, target/split/normalization policy,
an empty source declaration, and malformed builder values fail before model
bytes are read. Source framing, regular-file or container errors, row
selection errors, missing required features, ambiguous columns, missing numeric
values, invalid UTF-8 or scalar values, and categorical schema errors are
reported as data or path-addressed inference preparation failures.

### Model and compilation failures

Model source bounds, I/O, invalid roots, unsupported versions, malformed OGDL,
inconsistent checkpoint vectors, invalid parameter shapes or bytes, empty rows,
zero widths, noncontiguous spans, width transitions, unsupported topologies,
post-KNN operations, integer-domain overflow, checked index limits, graph
language errors, operation materialization errors, and static-program errors
map to `InferencePreparationError` or `InferenceCompileError`. The compile error
kinds are `EmptyDataset`, `InconsistentCheckpoint`, `UnsupportedTopology`,
`UnsupportedExtent`, `ArithmeticOverflow`, `IdentityExhausted`, `Language`,
`Operation`, `Program`, and `Ogdl`.

GGUF has independent `Container`, `UnsupportedArchitecture`,
`UnsupportedVariant`, `MissingMetadata`, `InvalidMetadata`, `MissingTensor`,
`InvalidTensor`, `InvalidTokenStream`, and `ArithmeticOverflow` classes. The
decoder never treats a quantized or different GGUF architecture as dense F32
llama.

### Native preparation and execution failures

Profile/cache identity, local GPU binding, toolchain, target specification,
host configuration, artifact catalog, reservation, candidate realization,
stabilization, capacity, finalization, or candidate exhaustion errors are
`NativePreparationError` or `InferenceExecutionError::Preparation`.

Execution boundary errors include invalid iteration count, metrics or loop
external transfers, duplicate or unbound external inputs, init image device,
member, dtype, size, bounds, and overlap failures, absent or unexpected exit
outputs, duplicate outputs, source identity mismatch, output overlap, output
dtype or byte-count mismatch, and a loop that never reaches terminal state.

After backend handoff, `Executor` failures carry an `InferenceRunFailure` with
run ID, bundle identity, optional journal, and the first cleanup error if
teardown also failed. If the loop and native teardown complete but exit bytes
fail validation, `PostExitValidation` preserves the completed run evidence and
journal while reporting the validation cause. No retry, alternate decoder,
substitute output, or host-side fallback masks these failures.

## Invariants and end-to-end role

The inference path preserves these invariants:

- The saved semantic model is authoritative for feature schema, target
  interpretation, normalization, topology, parameters, and labels.
- The query table is target-free and all selected rows are evaluated.
- Numeric and model calculations are GPU F32/I32 payload operations. Host code
  prepares exact byte images and formats already validated output.
- One immutable graph is compiled once. Discovery, code generation, allocation,
  artifact loading, and native warming are preparation, not model work in the
  loop.
- The graph has one external input descriptor for every graph input and one
  dense/Bayesian/GGUF external prediction or one KNN output per target.
- Init admission is the only external ingress. Exit egress is the only external
  prediction transfer. The loop contains calculations, internal transfers, and
  mandatory fault readbacks only.
- The finalized bundle, not the declaration or compiler, owns physical device
  placement, routes, queues, dependencies, arena locations, and egress source
  identities.
- The executor owns the ordered `init -> loop -> exit` lifecycle. Output bytes
  are collected only after loop completion and native teardown.
- The semantic artifact contains no native kernel bytes. Native images are
  realized for the current measured system and retained only as execution
  evidence.

The complete user-level role is therefore deterministic target-free evaluation:
bind new rows to a saved schema, lower every model calculation and transfer to
Recipe's graph, select a measured native implementation, execute one fixed
query pass on real hardware, validate the exact egress, and expose typed
predictions plus lifecycle evidence. A successful type check or declaration
parse is not runtime evidence. Acceptance requires the public declaration,
real dataset, current measured profile, and real CUDA or HSA execution to cross
the entire path above.
