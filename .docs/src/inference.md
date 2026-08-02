<!--
module: src/inference.rs
public_terminal: recipe.infer().log(...).evaluate()
advanced_compile_entrypoint: recipe::inference::compile_inference
purpose: compile and run one target-free model against new rows through the measured native lifecycle
model_kinds: [Dense, Knn, Bayes, GgufLlama]
runtime_phases: [declaration, bounded_preparation, graph_compilation, native_preparation, init, loop, exit, reporting]
calculation_dtypes: [f32, i32]
authoritative_state: finalized_bundle_then_native_exit_image
-->

# Inference

`src/inference.rs` is the root-library boundary for target-free inference. It
joins the public `Data`, `Model`, and `Infer` declarations to bounded source
and model preparation, one immutable Recipe calculation graph, the measured
native planner and executor, and the post-exit `InferenceReport`. It owns
orchestration, model-family dispatch, report formatting, and the public error
boundary. It does not implement source parsers, checkpoint decoders, tensor
operations, scheduling, native compilation, or backend execution itself.

The real terminal is:

```rust
recipe.data("input.csv")
    .exclude("target");
recipe.model().load("model.ogdl");
let report = recipe.infer().log([Time, Device]).evaluate()?;
```

`model.gguf` is also accepted for the named dense-F32 `llama` instrument. The
returned report exists only after native `init -> loop -> exit`, ordered
teardown, and prediction validation. `compile_inference` is the preparation
only entrypoint for callers that already hold `Infer`, `Data`, and `Model`;
it performs bounded filesystem reads and graph compilation but never probes,
allocates, realizes, or executes native resources.

## Public declaration boundary

### Declaration objects

The builders in `src/api.rs` are immutable intent records. They do not read a
file, infer a schema, probe a device, compile a kernel, allocate an arena, or
start a run.

| Declaration | Inference meaning | Runtime facts it does not contain |
| --- | --- | --- |
| `Data` | Ordered source paths plus optional column and row exclusions. | Parsed rows, inferred semantic types, fitted dictionaries, targets, splits, and normalization tensors. |
| `Model` | Exactly one loaded model source for inference. `load` rejects an empty path, a second source, or a mixed inline model. | Decoded OGDL/GGUF state, feature width, topology, parameter images, and native handles. |
| `Infer` | Static logging policy and the `evaluate` terminal. | A data/model pair until declaration resolution. |
| `InferenceDeclaration` | Private immutable snapshot of the consumed `Data`, `Model`, and `Infer`. | Native resources and mutable execution state. |

`recipe.data(...)` and `recipe.model()` write to the thread-local
`RECIPE_SEQUENCE` in `src/facade.rs`. Beginning a new data declaration replaces
the prior data and clears the model. Beginning a model declaration replaces
the prior model. `Infer::resolve_declaration` takes both values, so one
successful or failed evaluation consumes the immediately preceding pair. A
missing value is a declaration error with the exact diagnostics
`recipe.infer().evaluate() requires a preceding recipe.data(...) declaration`
or `...recipe.model()...`.

`Infer::log` accepts only `Time` and `Device`. Training metrics (`Loss`,
`Accuracy`, `R2`, `AuRoc`, `AuPrc`, `Brier`, `CalibrationError`, `Epoch`, and
`Lr`) are rejected as `InvalidInferenceConfiguration` because target-free
inference has no target values, optimizer state, or training epoch. Accepted
logging is host reporting after teardown; it is not a calculation-graph
metric and never creates a loop task.

### Declaration validation order

`Infer::evaluate` calls `resolve_declaration`, then
`evaluate_inference_declaration`. The preparation order is intentionally
linear and fail-closed:

1. `Infer::validate` reports deferred policy errors.
2. `Data::validate` requires at least one nonempty source and reports deferred
   data errors.
3. `Model::validate` requires an inline model, Bayesian network, or weight
   source, then rejects mixed inline and loaded declarations.
4. `require_target_free_data_policy` rejects `.target(...)`, `.split(...)`, and
   `.norm(...)`. Inference evaluates every retained row and reuses model-owned
   feature and normalization state.
5. `require_inference_model_source` requires `Model::load` and an exact lowercase
   `.ogdl` or `.gguf` extension.
6. The model file is decoded under bounded limits.
7. The declared source set is distilled under bounded ingest limits and then
   selected by exclusions.
8. The selected table is bound to the saved model schema and lowered to a
   static graph.
9. Only after graph compilation does current native preparation and execution
   begin.

No step substitutes a missing model, refits a schema, guesses a model family,
or falls back to another execution implementation.

## Bounded data preparation

`src/data_prepare.rs` provides the target-free path:

1. `distill_data` calls `recipe_ingest::distill_datasets` with finite defaults
   (`1 GiB` per aggregate source-byte bound, `10,000,000` records,
   `16,384` fields per record, and `16 MiB` per field).
2. Distillation recursively reads regular files, deterministic directory
   entries, and nested ZIP members. Specialized readers cover delimited text,
   JSON, images, GGUF, safetensors, XLSX, and PPTX. Other UTF-8 and opaque
   files remain lossless text or binary samples. Declared source order is
   retained as row order, and aggregate limits apply across all sources.
3. `select_target_free_data` maps `Data` column patterns and typed predicates
   to `recipe_ingest::select_table`. Predicates run against the original table
   before excluded columns are removed. Retained rows and columns preserve
   source order. An excluded required feature is not restored later.

This path does not call `prepare_data`, does not infer semantic types, does not
fit dictionaries, does not choose targets, does not split rows, and does not
normalize values. The saved model is the sole source of feature interpretation.
Source reads use one bounded immutable snapshot and retain no file handle or
callback in the runtime graph.

### Saved feature schema application

`recipe_ingest::prepare_inference_table` matches each saved feature by exact
column-name bytes. Source columns may be reordered, and unrelated columns are
ignored. A required name that is absent or appears more than once produces a
path-addressed preparation error. The saved `source_vector` identity and
feature span are checked again by `training/src/inference.rs`.

For semantic OGDL model families, the supported schema encodings are:

| Saved encoding | Input parsing | Calculation image |
| --- | --- | --- |
| `NumericI32` | Nonempty UTF-8 parsed as contract `i32`. Missing or invalid values fail. | One external `I32` vector. Dense inference emits a checked exact `i32 -> f32` conversion in the graph. |
| `NumericF32` | Nonempty UTF-8 parsed as contract finite `f32`, retained as exact bits. | One external `F32` vector. |
| `CategoricalDictionary` | Exact known byte labels map to saved codes. Empty and unseen nonempty labels use the reserved code after the dictionary. | External `I32` codes plus dense one-hot or family-specific categorical calculations. Typed observations preserve known, missing, and unseen routes. |

Numeric missing values, invalid UTF-8, parse failures, invalid saved
dictionaries, duplicate saved identities, row-length changes, and arithmetic
overflow are reported as `InferencePrepareError` values with feature, source
vector, column, and optional source-row paths. No host imputation is performed.
The GGUF llama path has a separate one-vector token-stream contract described
below and does not use this schema table.

## Model loading and family dispatch

`training/src/inference.rs::load_semantic_model_file` reads one bounded OGDL
snapshot, probes only its first root token, and delegates complete syntax,
version, canonicality, schema, tensor, and topology validation to the strict
decoder selected by that root. There is no root fallback.

| Path extension and root | Decoder and preparation | Compiled representation | Prediction boundary |
| --- | --- | --- | --- |
| `.ogdl`, root `recipe` | `decode_checkpoint` -> `CheckpointArtifact` -> `prepare_checkpoint_inference_table` | `CompiledModelInference::Dense(CompiledInference)` | One f32 matrix `[rows, task_width]`. |
| `.ogdl`, root `recipe-knn-model` | `decode_knn_model` -> `KnnModelArtifact` -> `prepare_knn_inference_table` | `CompiledModelInference::Knn(CompiledKnnInference)` | One typed `[rows, 1]` output per saved target. |
| `.ogdl`, root `recipe-bayes-model` | `decode_bayes_model` -> `BayesModelArtifact` -> `prepare_bayes_inference_table` | `CompiledModelInference::Bayes(CompiledInference)` | One concatenated f32 probability matrix `[rows, sum(child_classes)]`. |
| `.gguf` | `decode_gguf_llama` through `load_gguf_llama_model_file` -> `GgufLlamaArtifact` -> `prepare_gguf_llama_inference_table` | `CompiledModelInference::GgufLlama(CompiledInference)` | Raw f32 token logits `[sequence, vocabulary]`. |

An unknown OGDL root, unsupported extension, absent extension, missing file,
or mixed inline model is a model or unsupported-declaration failure, not a
request to reinterpret the bytes as another family.

## Common graph compiler boundary

`training/src/inference.rs::InferenceGraphCompiler` owns deterministic
`ValueId` and `KernelTemplateId` allocation, typed external input records, and
primitive graph emission. External images are immutable little-endian `f32`
or `i32` bytes. Every tensor is canonical contiguous row-major storage, and
every parameter image is checked for exact dtype, shape, and byte count before
it becomes an external input.

The compiler emits the existing calculation ontology only:

- `Elementwise` scalar programs for conversions, arithmetic, activations,
  normalization, gates, and output transforms.
- `Reduce` for sums, extrema, norms, and stable softmax statistics.
- `Contraction` for linear projections, attention, and pairwise products.
- `Gather`, `Scatter`, and `IndexMap` for feature placement, token lookup,
  sequence reshaping, windows, and tree traversal.
- `Histogram` and `Sort` for Bayesian and KNN reductions.
- Owned finite compositions materialized through `recipe_ops` when a single
  primitive is not sufficient, such as KNN, categorical Bayes, tree traversal,
  pooling, causal softmax, and GGUF RoPE.

The graph is validated, serialized to canonical OGDL, decoded again, wrapped
in a `StaticCalculationProgram`, serialized and decoded again, and assigned
`IterationDomain::first()` for every kernel. Inference programs have exactly
one loop iteration and no user metric emissions. The compiler never emits a
filesystem, host callback, optimizer, target-value, or loop-time model-load
operation.

## Dense semantic checkpoint inference

The `recipe` OGDL artifact retains feature spans, source vector identities and
names, dictionaries, target order and dtypes, the `DenseTask`, saved input
normalization tensors, effective canonical blocks, learned parameter images,
optional temperature, and any synthetic output adapter. Optimizer moments and
training-only state are not admitted to the inference graph.

### Feature matrix and input normalization

`compile_features` validates that prepared features and saved spans form one
nonempty contiguous width. Numeric `I32` values are converted to `F32` with a
device-side round-trip `i32 -> f32 -> i32` check. Categorical codes are expanded
to a dictionary-plus-reserved one-hot block. Blocks are scattered into a
row-major `[rows, feature_width]` matrix through graph calculations.

The checkpoint's data normalization is applied in the graph and cannot be
declared again on `Data`:

| Saved mode | Graph state |
| --- | --- |
| Identity | Requires no saved normalization tensors. |
| Z-score | Admits saved mean and variance tensors plus the feature mask. |
| Min-max | Admits saved minimum and maximum tensors plus the feature mask. |
| L2 norm | Squares, reduces, and normalizes each row using the saved feature mask; no fitted tensors are expected. |

Layer and batch normalization declared inside saved blocks are distinct from
input normalization. They calculate their statistics from the current query
rows in the graph, using the saved layer operation and epsilon.

### Effective block traversal

`compile_prepared_inference` traverses `CheckpointArtifact::blocks()` exactly
once. It never flattens a structured topology or substitutes a legacy layer
view. Each block checks its preceding logical shape, parameter shapes, and
output width before emitting calculations.

| Block image | Device calculation and invariant |
| --- | --- |
| `Embedding` | Requires the leading exact-`i32` token matrix and saved `[vocabulary, dimensions]` table. Gathers one vector per token and returns a flattened row sequence. |
| `Attention` | Requires the preceding sequence geometry and `heads * head_dimension == channels`. Emits bias-free Q/K/V/output projections, scaled causal softmax, head-major context, and sequence restoration. |
| `Rnn` | Unrolls the fixed scalar sequence with zero hidden state and saved input, recurrent, and bias tensors; only the final hidden state continues. |
| `Gru` | Unrolls reset, update, candidate, and hidden-blend equations with zero initial state and saved nine parameter tensors. |
| `Lstm` | Unrolls input, forget, output, candidate, cell, and hidden equations with zero hidden and cell states and saved twelve parameter tensors. |
| `Layer` | Materializes `gpu_linear_into`, then applies the saved ordered activation and normalization operations. Every learned PReLU occurrence must have one saved scalar. |
| `Convolution` | Uses checked channelwise one-dimensional windows and saved weight/bias images, then saved operations. Window indices are external immutable `I32` data. |
| `Pool` | Uses checked channelwise maximum-pool windows, winner bases, and the saved winner contract. |
| `KMeans` | Computes rooted L2 distances to saved centroids. Centroids are read-only during inference. |
| `Tree` | Materializes deterministic saved tree traversal from split features, thresholds, and leaf values. Exact feature-threshold ties route left; tree outputs sum in saved tree order. |
| `Residual` | Compiles the saved branch, validates equal-width identity or required projection skip, adds branch and skip, and applies saved output operations. |

The saved activation set is lowered by `forward::lower_activation` to Recipe
owned scalar programs or registered primitives: linear, cosine, exponential,
signed and natural logarithm, Huber, tangent, ReLU, leaky ReLU, sigmoid, tanh,
SELU, GELU, SiLU, ELU, and learned PReLU. An absent or extra saved PReLU
image, width mismatch, invalid parameter shape, unsupported structured version,
or checked extent overflow is an inconsistent checkpoint or compile failure.
Checkpoint validation also enforces version-specific topology rules, including
the leading embedding, RNN, GRU, or LSTM requirements and the optional attention
placement; inference does not infer a different sequence interpretation.

After the final block, saved temperature divides logits when present. The
`DenseTask` selects the final interpretation:

| Task | Graph output kind |
| --- | --- |
| Binary classification | Stable sigmoid, `BinaryProbability`, width `1`. |
| Multiclass classification | Row-wise stable softmax, `MulticlassProbabilities`, saved class count. |
| Scalar regression | Raw f32 value, `Regression`, width `1`. |
| Multi-target binary classification | Independent stable sigmoids, `MultiTargetBinaryProbabilities`, saved target order. |
| Joint multiclass classification | One row-wise stable softmax, `JointTargetProbabilities`, saved target order. |
| Multi-target regression | Raw ordered f32 values, `MultiTargetRegression`, saved target order. |

The compiler checks that effective block width equals the saved task width and
that saved target dtypes and target identities are complete. A task's output
adapter is represented by the effective saved block list, not by a host-side
postprocessing branch.

## All-output KNN inference

The `recipe-knn-model` artifact stores the exact reference feature image,
feature spans and schema, reference row order, per-target values and known
masks, decoders, neighbor count, and optional data normalization. It may retain
post-reduction operations in the artifact for compatibility, but
`compile_prepared_knn_inference` rejects any nonempty operation list because a
single transform cannot reinterpret heterogeneous numeric and discrete output
dtypes.

The compiler builds query features from the saved schema and admits the saved
reference feature matrix as another immutable input. If normalization is
declared, statistics are calculated from the reference matrix on the device
and applied to both query and reference features. Categorical dimensions use
the saved normalization mask.

`recipe_ops::append_knn_all_outputs` then materializes one calculation graph:

1. Compute rooted-L2 pairwise distances for every query/reference pair.
2. Validate each output's known-reference mask and replace unknown distances
   with the masked route.
3. Stable-sort each query's distances ascending, preserving saved reference
   order for exact ties.
4. Select `min(neighbors, known_references)` rows independently for every
   output.
5. Numeric targets gather f32 values and reduce a uniform mean into `[rows, 1]`
   f32 output.
6. Discrete targets gather i32 codes, histogram class counts, stable-sort counts
   descending, and gather the first code. Stable count ties select the lowest
   saved class code.

`CompiledKnnInference` retains one `KnnInferenceOutputContract` per declared
target, preserving source-vector identity, dtype, shape, and aggregation kind.
It has one loop iteration and one exit transfer per output.

## Observed categorical Bayesian inference

The `recipe-bayes-model` artifact stores one or more observed conditionals,
canonical parent and child dictionaries, ordered source identities, exact
reference row order, and raw parent and child observation codes. It stores no
host-fitted probability or count table.

`prepare_bayes_inference_table` builds the union of all parent schemas in the
first occurrence order. Shared parents are parsed once. Child vectors remain
saved model observations, not current inference columns. Missing or unseen
parent labels use each parent's reserved code after its known dictionary.

For each conditional, `compile_bayes_conditional` admits the saved reference
parents, saved reference child codes, query parent codes, parent multipliers,
parent cardinalities, and one f32 probability output. The materialized graph:

1. Packs each ordered parent row into a checked mixed-radix configuration.
2. Packs each reference configuration with its child code and histograms
   `(configuration, child_class)` counts.
3. Computes each query configuration and gathers its child counts.
4. Reduces the selected counts to a total and emits
   `(count + smoothing) / (total + child_classes)` for every class. The saved
   artifact requires Recipe's positive Laplace-one smoothing.

Repeated conditionals are compiled independently and concatenated row-wise on
the device through `gpu_concat_into`. Adjacent class ranges follow repeated
`.bayes(child, parents)` declaration order. The final
`BayesProbabilities` output is one f32 `[query_rows, sum(child_classes)]`
matrix. Empty conditionals, empty query data, invalid parent codes, state-space
overflow, identity exhaustion, and operation materialization failures are
compile errors.

## Named GGUF llama inference

`gguf_limits_for_file` first reads the file length and uses it as the aggregate
file, metadata, tensor, string, array, and depth bound, with rank capped at
four. `decode_gguf_llama` then requires GGUF v3, little-endian storage,
`general.architecture == "llama"`, dense F32 tensors, and a complete recognized
tensor set.

The first execution instrument accepts only ordinary dense llama geometry:

- nonzero vocabulary, context, embedding, feed-forward, block, and head sizes;
- equal query and key/value head counts, even full-head rotary dimension, and
  key/value widths equal to head width;
- causal attention, no mixture-of-experts tensors, no parallel residual, and
  supported linear or absent RoPE scaling;
- finite positive RMS epsilon, RoPE base, attention scale, and factors, with a
  nonnegative Q/K/V clamp; and
- F32 required tensors with exact metadata shapes. Optional biases, linear
  scales, `rope_freqs.weight`, and output projection are admitted only at their
  declared shapes. Missing output projection reuses token embeddings.

Other architectures, quantized tensors, grouped-query attention, partial RoPE,
unsupported scaling, extra tensors without an instrument, malformed metadata,
or missing tensors fail before native preparation.

`prepare_gguf_llama_inference_table` requires exactly one selected vector. It
splits every row's first field on ASCII whitespace, parses exact `i32` token
IDs, requires a nonempty stream, checks `0 <= token < vocabulary`, and checks
the stream length against context. It does not tokenize, pad, split sequences,
sample a token, or retain KV/session state.

`compile_prepared_gguf_llama_inference` admits token IDs and every execution
tensor, builds immutable RoPE partner, cosine, and signed-sine tables from
validated metadata, then emits token embedding gather, saved RMSNorm, Q/K/V
projections, adjacent-pair RoPE, scaled causal multi-head attention, residual
addition, parallel SwiGLU, second residual addition, final RMSNorm, and the
output projection. The sole output is `TokenLogits`, raw f32 `[sequence,
vocabulary]`.

## Measured native preparation and execution

`execute_current_inference_native` allocates a run ID and enters
`with_current_native_preparation` in `src/native_prepare.rs`. That boundary
reopens the identity-named measured profile, discovers the current host and
GPU origins, verifies topology and discovery identity, and lends scoped CUDA
and HSA bindings. There is no newest-profile or alternate-backend fallback.

The callback:

1. Collects measured device origin labels for the eventual report.
2. Selects the compiled graph from any `CompiledModelInference` variant.
3. Derives host worker count, staging bytes, and a watchdog from graph tensor
   sizes, graph work, measured transfer/calculation rates, host capacity, and
   measured concurrency.
4. Builds run-scoped host backend storage and a `StagedCrossBackend` bridge.
5. Constructs `LocalCandidateFactory::production`, `NativeExecutorDriver`, the
   exact deferred compiler for measured targets, `NativeCandidateRealizer`, and
   `Preparer` with the default native artifact catalog.
6. Calls `prepare_and_execute_local_inference` for Dense, Bayes, and GGUF, or
   `prepare_and_execute_local_knn_inference` for KNN.

`Preparer::prepare_program` is the fixed-point boundary between the measured
profile and a runtime bundle. It validates the graph and profile, resolves
artifacts, enumerates finite planner candidates, lowers deferred stages in the
Realize phase, creates native resources, warms maximum concurrency, takes the
bounded stabilization snapshots, rejects unstable or over-capacity candidates,
and finalizes one immutable `FinalizedBundle` plus the same warmed native
session. Finalization records the selected artifact identities, placements,
routes, arenas, loop domains, init images, and exact logical-to-physical exit
mapping.

### Execution phases and transfers

`training/src/execute.rs` enforces the closed inference boundary before any
native handoff:

- `CompiledInference` must have one loop iteration, an acyclic valid graph,
  exactly the declared external inputs, no duplicate semantic roles, and one
  canonical contiguous F32 external output with the expected `[rows, width]`
  kind and shape.
- `CompiledKnnInference` must have one loop iteration, no user metrics, the
  complete declared input set, at least one output, and one unique canonical
  output per saved target. KNN output tensors may be F32 or I32 according to
  their aggregation kind.
- Every external input role must be an allowed inference role: feature,
  normalization, saved dense parameter, KNN reference, Bayesian observation or
  query table, or GGUF tensor and RoPE table. Training-only roles are rejected.
- The finalized bundle may not contain external ingress or egress in `Loop`,
  and may not contain user metric tasks. Any violation names the task and fails
  closed.

The lifecycle is:

| Phase | Authoritative work |
| --- | --- |
| `init` | `build_inference_device_images` packs every declared input into each finalized device image. The planner creates one `External -> Device` admission transfer per image. Dtype, exact byte count, duplicate members, bounds, overlap, and coverage are checked. |
| `loop` | The one static calculation iteration executes on GPU storage. Internal device-to-device copies and synchronization are scheduler tasks, not model semantics. No filesystem, host callback, target value, optimizer update, user metric, or external transfer may enter this phase. |
| `exit` | The planner's exact `Device -> External` transfer for the single prediction, or each KNN output, runs only after the loop. The executor collects images, validates their planned source identities, then tears down arenas, queues, modules, and backend resources in order. |

The native backend is handed the warmed pre-final session exactly once. Kernel
compilation, module loading, allocation, and native-image realization are
pre-loop preparation. `NativeExecutionEvidence` records image loads, entry
lookups, queues, completion objects, persistent allocations, zero loop
realization calls, completed teardown, and zero live resources after teardown.

### Output mapping and byte validation

`map_inference_output` and `map_knn_inference_outputs` compare planned exit
tasks with the finalized bundle, including phase, destination, physical device,
physical value identity, dtype, shape-derived byte count, and non-overlapping
source locations. Missing, duplicate, unexpected, aliased, or stale output
tasks are execution errors.

After `ExitedLoop::exit_recoverable` returns, the collector accepts only the
planned exit images. Dense, Bayes, and GGUF images are copied into one
`InferencePrediction` whose contract remains the graph contract. KNN images are
copied into one `KnnInferencePrediction` per target. Host code does not
recalculate a prediction; it only retains validated bytes.

## `InferenceReport` and stdout

`evaluate_inference_declaration` joins the loaded artifact with its matching
completed execution, records elapsed loop time and measured device origin
labels, writes stdout, and then returns the fully exited `InferenceReport`.

| Report accessor | Meaning |
| --- | --- |
| `kind`, `run`, `bundle` | Model family and immutable run/bundle identities. |
| `prediction` | Singular dense, Bayes, or GGUF `InferencePrediction`; `None` for KNN. |
| `knn_predictions` | Ordered typed KNN predictions; `None` for other families. |
| `values` | Exact little-endian f32 decoding of a singular prediction after validation. KNN returns an empty iterator because outputs may mix F32 and I32. |
| `decode_multiclass_class` | Saved multiclass label or explicit reserved-unseen identity. |
| `bayes_output_count`, `bayes_output_name`, `bayes_output_classes`, `bayes_output_range`, `decode_bayes_output_class` | Saved child dictionaries and adjacent probability ranges for repeated Bayesian outputs. `decode_bayes_class` is the first-output compatibility accessor. |
| `decode_knn_class` | Saved KNN discrete code to `i32` or byte-label value. |
| `journal` | Completed ordered lifecycle and physical-call evidence. |
| `native_kernels` | Exact realized images for Dense, Bayes, and GGUF. KNN exposes typed predictions and native evidence but no retained kernel set. |
| `native_evidence`, `elapsed`, `devices` | Teardown evidence, checked loop-only duration, and measured device origins. |

The report writer always emits one prediction record per source row or token
position. Optional `Time` and `Device` lines precede those records:

| Family or task | Record content |
| --- | --- |
| Dense binary | `prediction`, row, `probability`. |
| Dense scalar regression | `prediction`, row, `value`. |
| Dense multiclass | Row, lowest-index maximum `class`, saved `label`, and every probability. Strictly greater comparison preserves the lowest index on ties. |
| Dense multi-target binary or regression | Row and `[saved-target-name=value, ...]` in saved target order. |
| Dense joint multiclass | Row, lowest-index maximum class, saved target name, and named probabilities. |
| Bayesian | One probability block per conditional and row, with child target name when there are repeated conditionals. |
| KNN | One record per row and saved target, with numeric value or discrete code plus decoded label. |
| GGUF llama | One record per sequence position with the argmax token ID and its logit. The returned report still retains every raw vocabulary logit. |

Labels and arbitrary saved bytes use reversible quoted escapes for backslash,
quotes, control bytes, and non-ASCII bytes. A stdout write or report-shape
guard failure is a `Runtime` error after native teardown, not a second execution
attempt.

## Failure taxonomy

The public `InferenceError` preserves the stage and nested source error:

| Variant | Reachable causes |
| --- | --- |
| `Declaration` | Deferred builder errors, missing immediately preceding declarations, invalid logging policy, invalid model/data declaration. |
| `Unsupported` | Targets, split, or data normalization supplied to inference; inline model or missing `load`; unsupported or absent model extension; missing data declaration at the compile-package boundary. |
| `Data` | Bounded source I/O, symlink or path rejection, malformed file/archive, source or record limits, duplicate or unmatched exclusion columns, invalid predicates, no retained rows or vectors, and selected-table shape failures. |
| `Model` | Bounded OGDL source snapshot, unknown semantic root, strict checkpoint/KNN/Bayes decode, saved schema inconsistency, feature application failure, GGUF container, metadata, tensor, architecture, variant, or token-stream failure. |
| `Compile` | Empty query data, empty model state, unsupported topology or extent, inconsistent checkpoint spans or parameter bytes, invalid operation materialization, graph/language/program/OGDL validation, workspace overflow, or exhausted deterministic IDs. |
| `Native` | Missing or stale measured profile, current host/GPU identity mismatch, unsupported container or no GPU, target/toolchain mismatch, invalid host staging configuration, native binding, artifact, candidate realization, stabilization, or final-capacity failure. |
| `Execute` | One-iteration or graph-boundary violations, undeclared or unbound init inputs, image dtype/size/bounds/overlap errors, loop external transfers, native handoff, executor watchdog/lifecycle failure, missing or aliased exit output, physical source mismatch, output dtype/size mismatch, or incomplete teardown. Post-exit validation wraps the original execution error with `InferenceRunFailure`. |
| `Runtime` | GGUF file-length inspection or bound construction, report formatting, stdout I/O, or post-exit report invariant failure. |

`InferenceRunFailure` retains the run and bundle identities, an optional
bounded `RunJournal`, and the first ordered cleanup error after native handoff.
Executor failures never trigger a retry or alternate backend. Native resources
are still destroyed in their defined order, and the primary error remains
visible.

## Invariants and implementation boundaries

1. Inference is target-free at the public data boundary. Current targets,
   train splits, and newly fitted normalization state never enter the graph.
2. The saved model owns feature names, source identities, dictionaries,
   normalization, topology, parameters, output interpretation, and target
   labels. Current input is only a bounded table selected by declared
   exclusions and matched to that schema.
3. Every executable path lowers completely to Recipe calculations and
   transfers. `TaskKind::Metric` is not inference model work. Init admission and
   output egress are transfers, while dependencies, routes, queues, and
   synchronization only order or realize calculations and transfers.
4. All calculation payloads are device-side `f32` or `i32`; host code performs
   bounded parsing, immutable byte packing, and post-exit byte interpretation.
5. The graph has one canonical output boundary for Dense, Bayes, and GGUF, and
   one canonical typed boundary per target for KNN. All boundary tensors are
   contiguous row-major images with exact storage bytes.
6. Inference has exactly one loop iteration, no loop ingress or egress, no
   optimizer state, no training metric, and no loop-time kernel compilation,
   loading, or allocation.
7. Native execution uses the current measured profile and one fixed-point
   realized bundle. Candidate rejection, capacity instability, or identity
   drift is a visible failure, never a fallback.
8. `InferenceReport` is returned only after ordered exit and teardown. Its
   prediction bytes, journal, native evidence, run identity, bundle identity,
   and measured-device list describe that completed run.

The module intentionally does not provide chat, tokenization, sampling, KV
cache state, model conversion, training, resume, artifact saving, remote
execution, or a second inference implementation. Structural GGUF conversion
and legacy chat code do not imply executable GGUF or chat support here.

## Source map

| Concern | Source boundary |
| --- | --- |
| Public builders, logging policy, declaration consumption | `src/api.rs`, `src/facade.rs` |
| Orchestration, dispatch, report, public errors | `src/inference.rs` |
| Bounded source distillation and target-free selection | `src/data_prepare.rs`, `ingest/src/dataset.rs`, `ingest/src/source.rs`, `ingest/src/prepare.rs` |
| Schema-bound query encoding | `ingest/src/inference.rs` |
| Semantic model decoding, dense/KNN/Bayes preparation and graph compilation | `training/src/inference.rs`, `training/src/checkpoint.rs`, `training/src/knn_checkpoint.rs`, `training/src/bayes_checkpoint.rs` |
| Named GGUF llama decoder and lowering | `training/src/gguf_llama.rs` |
| Recurrent forward equations and activation lowering | `training/src/forward.rs` |
| KNN, categorical Bayes, tree, and other finite operation materialization | `ops/src/knn_outputs.rs`, `ops/src/bayes.rs`, `ops/src/tree.rs`, `ops/src/composition.rs` |
| Static one-iteration program and graph contract | `program/src/lib.rs`, `language/src/graph.rs`, `language/src/primitive.rs`, `language/src/tensor.rs` |
| Measured profile, native bindings, host tuning, and scoped identity | `src/native_prepare.rs`, `src/training.rs`, `src/cli.rs` |
| Candidate artifact compilation, warmup, stabilization, planning, and finalize | `prepare/src/lib.rs`, `prepare/src/production.rs`, `planner/src/planner.rs`, `scheduler/src/static_schedule.rs`, `core/src/plan.rs` |
| Native CUDA/HSA realization, handoff, and evidence | `native-executor/src/local.rs`, `native-executor/src/cuda.rs`, `native-executor/src/hsa.rs`, `native-executor/src/evidence.rs` |
| Typestate lifecycle, phase transfers, output images, and journal | `training/src/execute.rs`, `executor/src/executor.rs` |
| Normative target-free and family contracts | `system-contract.md` sections C27 and C33-C42, `API.ogdl` |
