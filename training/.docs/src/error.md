# Training error model

This page describes the error values exported by `training` and the internal
helpers that construct them. The error types are deliberately split at the
boundaries where ownership and state change:

* compilation errors stop before a `CompiledTraining` or compiled inference
  program exists;
* checkpoint errors cover decode, semantic validation, resume admission, output
  mapping, and durable artifact writes;
* inference preparation errors cover source snapshots, model decoding, and
  schema-bound rows;
* execution errors cover finalized-boundary checks, native handoff, executor
  lifecycle, and post-exit image validation.

All of the public result aliases are ordinary `Result<T, E>` values. A `?`
propagates the error to the caller unless a documented conversion wraps it.
There are no retries, alternate decoders, or fallback execution paths.

## Error propagation at a glance

```text
PreparedDataset + declaration
        |
        +-- compile_* --------------------------> TrainingCompileError
        |
        +-- prepare_* / load_* ------------------> InferencePreparationError
        |        |                                  |
        |        +-- SourceError ------------------+
        |        +-- CheckpointError ---------------+
        |        +-- GgufLlamaError ----------------+
        |        +-- InferencePrepareError ---------+
        |
        +-- decode_* -----------------------------> CheckpointError
        |                                              |
        |                                              +-- CheckpointDecodeError
        |
        +-- compile_prepared_* ------------------> InferenceCompileError
        |
        +-- prepare_and_execute_local_* ----------> execution error
                 |
                 +-- PrepareError ----------------- Training/InferenceExecutionError
                 +-- LocalError ------------------- NativeHandoff
                 +-- ExecutorError ---------------- Executor
                 +-- finalized-boundary checks ----- structural variants
```

The Bayesian resolver is the one structural API that has its own result type:
`resolve_bayesian_schema` returns `BayesianSchemaError` directly. Executable
Bayesian preparation converts that structured error into
`TrainingCompileErrorKind::InvalidNetwork`, retaining the formatted path only
inside `detail`.

## `TrainingCompileError`

`TrainingCompileErrorKind` is a `Copy`, non-exhaustive classification enum.
`TrainingCompileError` has public `kind` and `detail` fields, is `Clone`, and
implements `Display` as `"{kind:?}: {detail}"`. Its `Error::source` is always
`None`: converted dependency errors are flattened into `detail`.

| kind | Meaning and construction sites |
| --- | --- |
| `EmptyDataset` | A required training/reference partition has no rows. `model::validate_partition` rejects an empty dense partition; `compile::training_bounds` rejects zero training rows; Bayesian and KNN reference preparation reject empty training data. |
| `InconsistentRows` | Row-aligned storage disagrees. `model::DensePartition::new` checks target-observation count; `model::validate_partition` checks feature/target row counts; `model::compact_dense_rows` reports an absent row slice. |
| `InvalidFeatureMatrix` | Feature semantics, storage, or lowering are not valid. `DenseFeaturePlan::from_prepared`, `lower_dense_features`, `lower_numeric_feature`, categorical dictionary/code checks, `validate_matrix_storage`, recurrent and embedding validation, and exact int32-to-f32 validation construct it. `compile::validate_config` also uses it for incompatible embedding/normalization modes. |
| `InvalidTargetMatrix` | Target identity, dtype, shape, values, or task meaning is invalid. `CompiledDatasetSchema::from_prepared`, `resolve_dense_task`, all dense target lowerers, binary-target checks, validation configuration, and model compaction construct it. `bayes_vector_error`, Bayesian target/order checks, and `knn_target_error` use it for vector-specific failures. |
| `InvalidNetwork` | The declared topology or derived semantic state violates a supported network invariant. `compile::validate_config`, `effective_blocks`, PReLU validation, block/state matching, and scalar-program loss helpers construct it. Bayesian preparation and reference-set validation use it for unsupported latent/shape/schema cases. |
| `InvalidOptimizer` | Optimizer, schedule, validation, or horizon policy is invalid. `validate_config`, `validate_validation_config`, `training_bounds`, `accepted_update_plan`, schedule-input/program helpers, and temperature-scaling validation construct it. |
| `UnsupportedExtent` | A valid concept does not fit a required representation. Checked `usize` to `u64`/`i32` conversions in compilation, tree depth, embedding vocabulary, validation rows, schedule counters, and Bayesian cardinalities/multipliers use it. |
| `ArithmeticOverflow` | A checked product, addition, reserve, or derived shape/counter overflows. It is constructed throughout model lowering, dense compilation, schedule planning, scalar-program construction, Bayesian reference append/mixed-radix preparation, and KNN lowering. |
| `IdentityExhausted` | `compile::identity_exhausted` maps exhaustion of the graph identity namespace to this kind. It is used by identity-index increments while lowering blocks and calculations. |
| `Ingest` | `From<recipe_ingest::PrepareError>` wraps a failed prepared-data operation and stores its `to_string()` text. |
| `Language` | `From<recipe_language::LanguageError>` and the generic forward lowering path (`ForwardGraph::Error: From<LanguageError>`) use this kind. |
| `Operation` | `From<recipe_ops::OperationError>` and operation materialization/lowering propagate this kind. |
| `Program` | `From<recipe_program::ProgramError>` propagates static-program construction failures. |
| `Ogdl` | `From<recipe_language::OgdlCodecError>` propagates OGDL serialization failures encountered while compiling. |

The constructor `TrainingCompileError::new` is `pub(crate)` and is the single
constructor used by the training crate. The five `From` implementations are
the only dependency conversions. `TrainingCompileResult<T>` is
`Result<T, TrainingCompileError>`.

### Compilation call chain and state consequence

Every public dense entry point in `compile.rs` (`compile_dense_training`, the
`with_blocks` form, and the three validation families, including their flat
wrappers) calls `compile_dense_training_impl`. That implementation resolves the
task, validates configuration/topology, lowers features and targets, validates
partitions, computes bounds and accepted-update plans, emits graph operations,
and finishes a `CompiledTraining`. Any `TrainingCompileError` returns before
the final value is produced. Intermediate local vectors and graph builders are
dropped; no caller-owned dataset or configuration is mutated.

The main propagation stages are:

1. `model::DenseFeaturePlan::from_prepared` and
   `LoweredDenseDataset::from_prepared` call the feature/target lowerers with
   `?`. Their errors reach `compile_dense_training_impl` unchanged.
2. `resolve_dense_task`, `validate_lowered_dataset`,
   `validate_embedding_dataset`, `validate_recurrent_dataset`,
   `validate_config`, `effective_blocks`, and validation/bounds helpers return
   the same error type and are chained with `?`.
3. `GraphCompiler` and the forward module return dependency errors. `From`
   conversions classify them as `Language`, `Operation`, `Program`, or
   `Ogdl`; their original typed source is not retained.
4. `prepare_categorical_bayesian_reference_sets` and
   `prepare_knn_reference_set` return `TrainingCompileResult` directly. The
   Bayesian DAG resolver error is converted to `InvalidNetwork`; KNN feature and
   target failures use the shared constructors described above.

`TrainingCompileError` therefore means that no executable graph, schedule, or
native preparation boundary is available. It is a declaration/data failure,
not an execution status.

## Bayesian schema errors

`resolve_bayesian_schema` returns `BayesianSchemaResult<T>`, where
`BayesianSchemaError` stores a private `kind`, a vector of
`BayesianSchemaPathSegment`, and a private `detail`. The accessors are
`kind()`, `path()`, and `detail()`. `Display` is
`"{kind:?} at {path}: {detail}"`; it implements `Error` without a source.

### Kinds and paths

`BayesianSchemaErrorKind` is non-exhaustive and has exactly these current
classes:

* `EmptyName`: an empty declared child or parent name;
* `DuplicateDatasetName`: two prepared vectors have the same name;
* `DuplicateChild`: the same child is declared twice;
* `DuplicateParent`: one dependency repeats a parent;
* `SelfDependency`: child and parent names are equal;
* `Cycle`: deterministic topological ordering cannot consume every node.

`BayesianSchemaPathSegment` identifies `Dataset`, `Vectors`, `Vector(index)`,
`Name`, `Declarations`, `Declaration(index)`, `Child`, `Parents`,
`Parent(index)`, `Graph`, and `ExecutionOrder`. The formatter emits paths such
as `dataset.vectors[2].name` and `declarations[0].parents[1]`.

`observed_schemas` constructs `DuplicateDatasetName` at the duplicate vector's
name path. `validate_dependencies` constructs `EmptyName`, `DuplicateChild`,
`SelfDependency`, and `DuplicateParent` at declaration/child/parent paths.
`deterministic_topological_order` constructs `Cycle` at
`graph.execution-order`, listing the nodes with nonzero indegree. The private
`BayesianSchemaError::new` is the only constructor.

`resolve_bayesian_schema` first validates observed names and declarations, then
builds canonical name-ordered nodes, declaration-ordered edges, and a
deterministic execution order. On an error it returns no `ResolvedBayesianSchema`.
`prepare_categorical_bayesian_reference_sets` maps any resolver error to
`TrainingCompileErrorKind::InvalidNetwork` with the resolver's formatted text;
the structured path and kind are not available through the resulting compile
error fields.

## Checkpoint errors

`CheckpointResult<T>` is `Result<T, CheckpointError>`. Checkpoint decoding uses
`CheckpointPath`, whose segments are `Field(String)` and `Index(usize)`, so a
decoder failure can identify the exact OGDL location.

### Decode classifications

`CheckpointDecodeErrorKind` is non-exhaustive and currently contains:

* `LimitExceeded`: source bytes, OGDL nodes, metadata entries, tensors, rank,
  tensor bytes, or total payload exceed a checked limit, or a checked count
  overflows;
* `InvalidUtf8`: source or model-root bytes are not UTF-8;
* `InvalidSyntax`: OGDL roots/children, scalar shape, tagged fields, or payload
  structure are malformed;
* `MissingField`: a required field, tagged value, native kernel, or payload
  chunk is absent;
* `DuplicateField`: a field name occurs more than once in a field set;
* `UnknownField`: a field or list entry is not allowed at its path;
* `InvalidValue`: canonical numeric/hex text, enum, dtype, digest, root,
  version, format, or payload value is invalid;
* `InconsistentValue`: the decoded artifact violates cross-field semantic
  invariants.

`CheckpointDecodeError` is constructed by the crate-private `decode_error`
helper (`CheckpointError::Decode(CheckpointDecodeError::new(...))`). It exposes
`kind()`, `path()`, and `detail()`, displays as `"{path}: {detail}"`, and has no
source of its own.

The decoder's construction and propagation order is fixed:

1. `decode_checkpoint` checks `source_bytes`, UTF-8, OGDL parsing, and node
   count. These produce root-path `LimitExceeded`, `InvalidUtf8`, or
   `InvalidSyntax` errors.
2. `Decoder::fields`, `FieldSet::require`, `node`, `scalar`, `tagged`, and
   `fields_from_children` produce path-addressed missing, duplicate, unknown,
   and structural syntax errors.
3. `Decoder::decode` validates the single `recipe` root, supported version and
   format, required semantic sections, and native metadata presence. Invalid
   roots/versions/formats are `InvalidValue`; semantic versions without native
   metadata are `MissingField`; legacy versions containing it are `UnknownField`.
4. Numeric, enum, digest, shape, and payload helpers classify canonicality and
   range errors as `InvalidValue`, payload/rank accounting as `LimitExceeded`,
   and malformed OGDL nesting as `InvalidSyntax`.
5. `validate_artifact` and its `validation_error` helper classify decoded
   cross-field mismatches as `InconsistentValue` at the affected model path.

An error from any step returns no `CheckpointArtifact`; no partially decoded
artifact escapes.

### `CheckpointError` variants

`CheckpointError` is non-exhaustive. `Display` prefixes each class with its
operation, and `Error::source` is present only for `Decode` and `Io`.

| variant | Construction and invariant |
| --- | --- |
| `Decode(CheckpointDecodeError)` | `decode_error` wraps every strict decoder failure. `load_semantic_model_file` also creates root `InvalidUtf8`/`InvalidValue` decode errors before dispatch. |
| `InvalidManifest { detail }` | `CheckpointError::manifest` is used by `CheckpointManifest::from_compiled`, block/state builders, program digest encoding, semantic invariant conversion (`manifest_semantic_error`), and KNN checkpoint encoding/validation. It means the in-memory declaration or output boundary cannot describe a valid artifact. |
| `IncompatibleResume { detail }` | `apply_checkpoint_resume`, `validate_resume_tensor`, and `validate_resume_compatibility` construct it when schemas, objective/normalization/AdamW semantics, topology, parameter tensors, K-means tensors, tree tensors, resume roles, or component coverage differ. |
| `DuplicateOutput { value }` | `map_outputs` and KNN output mapping see the same logical checkpoint value more than once. |
| `MissingOutput { value }` | `map_outputs` does not receive a physical image for an expected checkpoint tensor. |
| `UnexpectedOutput { value }` | An executor exit image names a value absent from the checkpoint manifest. |
| `OutputDTypeMismatch { value, expected, actual }` | Exit image dtype differs from the manifest tensor dtype. |
| `OutputSizeMismatch { value, expected, actual }` | Exit source or host bytes differ from the manifest tensor byte count. |
| `NativeKernelUnavailable { requested }` | `CompletedTrainingCheckpoint::save_native_kernel` finds no realized image in the requested `.cubin` or `.hsaco` format. |
| `NativeKernelAmbiguous { requested, images }` | More than one distinct realized image has the requested format, so one exported file cannot represent the realization. |
| `InvalidTarget { path, detail }` | `invalid_target` rejects an empty/non-file target, a non-directory parent, symlink/non-regular target, extension mismatch, kernel-size conversion failure, encoder-size mismatch, or exhausted private temporary-name attempts. |
| `InsufficientCapacity { path, available, checkpoint_allocation, reservation }` | `require_capacity` rejects a target when available filesystem bytes are less than allocation plus `EXACT_USER_RESERVATION`. Zero fragment size and capacity arithmetic overflow are `InvalidManifest` instead. |
| `Io { operation, path, source }` | `io` wraps filesystem/stat/temporary-file/write/flush/sync/rename errors with the exact operation and path. The original `io::Error` is returned by `source()`. |

### Manifest, resume, and save propagation

`CheckpointManifest::from_compiled` snapshots schema, feature spans, target
identities, config, bounds, normalization tensors, effective blocks, and the
canonical program digest. It builds block declarations through
`checkpoint_blocks`, `checkpoint_layer`, `checkpoint_convolution`, the
recurrent/embedding/attention/K-means/tree/residual helpers, and
`checkpoint_tensor`. Any missing graph tensor, non-external tensor, declaration
versus state mismatch, or output-boundary mismatch returns `InvalidManifest`.

`compiled_training_program_digest` maps OGDL serialization failure to
`InvalidManifest`. `CompletedTrainingCheckpoint::new` then records realized
native identities, validates semantic invariants, and maps physical exit images
to logical tensor values. It cannot produce a checkpoint if native realization
is empty or output mapping fails.

`apply_checkpoint_resume` validates the artifact and compatibility before
admitting bytes into the existing `CompiledTraining`. It sets the one
`ResumeEnabled` input to `1`, replaces parameter/moment, K-means, and tree
resume input bytes, and checks that every required role was admitted exactly
once. The function has no rollback buffer: if a later role check fails after
earlier replacements, the caller's `CompiledTraining` can retain those earlier
byte replacements even though the function returns `IncompatibleResume`.

`CompletedTrainingCheckpoint::save` measures canonical OGDL size, then calls
`atomic_save`. `save_native_kernel` first checks the requested extension and
format cardinality, then uses the same atomic writer. `atomic_save` validates
the parent/target, checks capacity, writes a private temporary file, flushes and
syncs it, verifies the measured byte count, renames it into place, and syncs the
parent directory. The temporary guard removes the temporary file when an error
occurs before installation. A parent-directory sync error can therefore return
`Io` after the rename has already installed the target.

KNN and Bayesian checkpoint decoders (`decode_knn_model`, `decode_bayes_model`)
reuse `CheckpointError::Decode` and the same decode-kind semantics. KNN
checkpoint encoders and compatibility checks reuse `InvalidManifest` and
`IncompatibleResume`; they do not define a parallel error enum.

## Inference preparation errors

`InferencePreparationResult<T>` is `Result<T, InferencePreparationError>`.
`InferencePreparationError` is non-exhaustive and has six variants:

* `CheckpointSource(SourceError)`: bounded source-limit construction or
  `read_source_snapshot` failed;
* `Checkpoint(CheckpointError)`: strict dense, KNN, or Bayesian model decoding
  failed, including a root-dispatch decode error;
* `GgufLlama(GgufLlamaError)`: GGUF parsing/binding failed;
* `Data(InferencePrepareError)`: `recipe_ingest::prepare_inference_table`
  rejected the query table;
* `InconsistentCheckpoint { feature, source_vector, detail }`: prepared feature
  identities, encodings, or span widths do not agree with the saved model;
* `ArithmeticOverflow { detail }`: a caller-supplied source-byte bound cannot
  be represented as `u64`.

`Display` prefixes the first four dependency variants with the operation. The
`source()` method exposes their wrapped error and returns `None` for the two
owned variants. The four `From` implementations are the only conversions.

`load_checkpoint_file`, `load_knn_model_file`, and `load_bayes_model_file`
convert the source-byte bound, read a regular-file snapshot, and map the
family decoder to `Checkpoint`. `load_semantic_model_file` reads one bounded
snapshot, probes only its first line, and dispatches strictly to the dense,
KNN, or Bayesian decoder. Unknown roots become a root-path
`CheckpointDecodeErrorKind::InvalidValue`; there is no decoder fallback.

`prepare_checkpoint_inference_table`, `prepare_knn_inference_table`, and
`prepare_bayes_inference_table` call ingestion preparation and then validate
saved feature spans where applicable. `validate_prepared_feature_spans`
constructs `InconsistentCheckpoint` for a count, source identity, encoding,
dictionary width, reserved route, or value-storage mismatch. The combined
`load_and_prepare_*` functions use `?`, so the first source, decode, schema, or
data error reaches the caller unchanged. No prepared inference object is
returned on failure and no native graph is created.

## Inference compilation errors

`InferenceCompileErrorKind` is non-exhaustive with these classes:

* `EmptyDataset`: no query rows for dense, Bayesian, or KNN inference;
* `InconsistentCheckpoint`: empty/contradictory saved topology, task widths,
  target dtypes, KNN reference state, or an unbound GGUF tensor;
* `UnsupportedTopology`: a supported preparation contains a topology the
  selected inference instrument does not define, currently post-KNN operations;
* `UnsupportedExtent`: a row, width, class, label, or token count does not fit
  the required integer domain;
* `ArithmeticOverflow`: derived widths, RoPE indices, aggregate output widths,
  or other checked products overflow;
* `IdentityExhausted`: graph identity allocation is exhausted;
* `Language`, `Operation`, `Program`, and `Ogdl`: dependency conversions from
  the corresponding Recipe crates.

`InferenceCompileError` keeps private `kind` and `detail` fields, exposes
`kind()` and `detail()`, displays as `"{kind:?}: {detail}"`, and has no source.
Its private `new` constructor is used by `inference.rs` and GGUF lowering. The
four dependency `From` implementations flatten only the dependency text.

`compile_prepared_inference` rejects empty blocks, zero rows/width, block-output
width mismatches, missing fixed target dtypes, and identity exhaustion while
traversing the saved effective topology exactly once. Dense compiler helpers
propagate graph, operation, language, and program failures with `?`.
`compile_prepared_bayes_inference` rejects zero rows or no conditionals and
checks each child-class width and aggregate-width addition. The KNN compiler
rejects post-KNN operations, empty query/reference matrices, and widths or
label counts outside checked domains. GGUF compilation uses the same error type
for token-count conversion, RoPE shape/index overflow, missing admitted tensors,
and all graph/materialization failures.

All compilation happens in local graph-builder state. An error drops that local
state and returns no `CompiledInference` or `CompiledKnnInference`; prepared
model artifacts and query rows remain owned by the caller and are not mutated.

## GGUF llama errors

`GgufLlamaResult<T>` is `Result<T, GgufLlamaError>`. The non-exhaustive
`GgufLlamaErrorKind` classes are:

* `Container`: `parse_gguf` rejected the bounded byte image;
* `UnsupportedArchitecture`: `general.architecture` is not `llama`;
* `UnsupportedVariant`: unsupported GGUF version/endianness, grouped-query
  attention, non-full-head RoPE, key/value widths, mixture-of-experts,
  parallel residual, non-causal attention, YaRN/other RoPE scaling, extra
  tensors, or nonzero SwiGLU clamps;
* `MissingMetadata`: a required metadata key is absent;
* `InvalidMetadata`: a metadata value has the wrong type or violates nonzero,
  finite, positive, range, or array-length constraints;
* `MissingTensor`: a required model tensor is absent;
* `InvalidTensor`: a tensor is not F32 with the exact declared dimensions, has
  no encoded span, or contains an invalid RoPE factor;
* `InvalidTokenStream`: token table width, row contents, UTF-8, int32 syntax,
  vocabulary domain, nonempty stream, or context length is invalid;
* `ArithmeticOverflow`: a checked GGUF extent cannot fit host `usize`.

`GgufLlamaError::new` is private. `decode_gguf_llama` constructs container,
variant, architecture, metadata, and tensor errors through its metadata helpers
and `ArtifactTensorBuilder`. `load_gguf_llama_model_file` maps source and
snapshot failures to `InferencePreparationError::CheckpointSource`, then maps
the decoder to `InferencePreparationError::GgufLlama`. Token binding in
`prepare_gguf_llama_inference_table` constructs `InvalidTokenStream` and
converts it to the same preparation wrapper. A failed decode or token bind
returns no artifact/prepared stream and never proceeds to graph compilation.

The error displays as `"{kind:?}: {detail}"` and has no nested source. The
decoder is fail-closed: an unsupported architecture or variant is an error, not
an interpretation as ordinary llama.

## Training execution errors

`TrainingExecutionResult<T>` is `Result<T, TrainingExecutionError>`. The enum is
non-exhaustive, displays operation-specific messages, and exposes dependency
sources only for `Preparation`, `NativeHandoff`, and `Executor`. The
`From<PrepareError>` and `From<ExecutorError>` implementations construct those
three propagation classes.

| variant | Construction site and invariant |
| --- | --- |
| `Preparation(PrepareError)` | `Preparer::prepare_program` fails before native handoff. |
| `NativeHandoff(Box<dyn Error + Send + Sync>)` | `native_handoff_error` wraps `LocalError` from `ValidatedCandidateSession::into_backend`. |
| `Executor(ExecutorError)` | `PreparedRun::prepare`, `initialize`, `start_loop`, polling, and `exit` use the `From<ExecutorError>` conversion. |
| `DuplicateExternalInput { value }` | `pack_device_images` sees the same logical training input twice. |
| `DuplicateInitDevice { device }` | More than one finalized init image names one device. |
| `DuplicateImageMember { device, value }` | One device image repeats a logical member. |
| `MissingExternalInput { device, value }` | A finalized image member has no corresponding compiled input. |
| `ImageMemberDTypeMismatch { device, value }` | Input and finalized member dtypes differ. |
| `ImageMemberSizeMismatch { device, value, expected, actual }` | Input byte length is not the finalized member size. |
| `ImageSizeUnsupported { device, bytes }` | A finalized image byte count cannot fit host `usize`. |
| `ImageMemberOutOfBounds { device, value }` | Offset/length arithmetic or host slicing leaves the finalized image. |
| `ImageMembersOverlap { device, first, second }` | Sorted finalized members overlap. |
| `LoopExternalTransfer { task }` | `reject_loop_external_transfers` finds external ingress or egress in `RunPhase::Loop`. |
| `ExternalOutputMapping { detail }` | `map_external_output_tasks` finds an invalid/missing/duplicate checkpoint exit mapping, endpoint, source identity, or task set. |
| `InvalidTrainingBounds { detail }` | The variant exists for a bounds failure but has no construction site in the current workspace. |
| `UnboundedTrainingRequiresStopControl` | `validate_training_execution_control` rejects an unbounded horizon without an `AtomicBool` stop source. |
| `LoopDidNotReachTerminalState` | The current construction is the `into_exited_loop` map when a bounded wait returns a still-running training loop. |

`build_training_device_images` only performs packing. The three public training
execution entry points converge on `prepare_and_execute_local_training_controlled`:

1. Validate stop-control policy, prepare the exact program, reject loop
   external transfers, pack one init image per finalized device, and map every
   external output. These errors occur before backend handoff, so no native run
   exists.
2. Convert the validated local session into a backend. A handoff error is
   wrapped as `NativeHandoff`.
3. Prepare and initialize `PreparedRun`, start the loop, poll until complete,
   drain metrics, exit, and map logical output identities. Executor failures are
   returned as `Executor`; the executor owns cleanup for its recoverable path.
4. On success, return `CompletedTrainingExecution` only after native teardown,
   with exit images, logical identities, final metrics, native evidence, and a
   complete journal. No training execution error is a partial-success signal.

The packing invariants are strict: one external-input declaration per logical
value, one init image per device, unique members, exact dtype/size, in-bounds
non-overlapping ranges, and zero-filled gaps. Output mapping requires the
planned and finalized exit-transfer task sets and logical tensor set to match
exactly. A failure therefore returns before the completed evidence value exists.

## Inference execution errors

`InferenceExecutionResult<T>` is `Result<T, InferenceExecutionError>`. Unlike
training executor errors, native failures retain `InferenceRunFailure` evidence
when the backend has been handed off. `run_failure()` returns that evidence for
`Executor` and `PostExitValidation` variants.

| variant | Construction site and invariant |
| --- | --- |
| `Preparation(PrepareError)` | `Preparer::prepare_program` fails before handoff. |
| `NativeHandoff(Box<dyn Error + Send + Sync>)` | `native_inference_handoff_error` wraps local backend handoff failure. |
| `Executor { source, failure }` | `inference_executor_failure` decomposes `RunFailure`, retains run/bundle/journal/first cleanup error, drops the backend, and stores the executor source. |
| `PostExitValidation { source, failure }` | `post_exit_inference_failure` wraps prediction-image validation after successful `exit`; the retained journal is present and cleanup has completed. |
| `InvalidLoopIterations { actual }` | Compiled program or finalized bundle is not exactly `LoopIterations::ONE`. |
| `InvalidInferenceBoundary { detail }` | Boundary validators reject metrics, graph/input/output identity, role, canonical layout/storage, task kind/shape, or typed-shape arithmetic. |
| `DuplicateExternalInput { value }` | A declared inference/KNN input value is repeated. |
| `UnboundExternalInput { value }` | An input is declared but appears in no finalized init image. |
| `ExternalInputByteSizeMismatch { value, expected, actual }` | Owned input bytes differ from its typed shape. |
| `DuplicateInitDevice { device }` | More than one init image names a device. |
| `DuplicateImageMember { device, value }` | A device init image repeats a logical input. |
| `UnexpectedImageMember { device, value }` | A finalized image names an undeclared inference input. |
| `ImageMemberDTypeMismatch { device, value }` | Input dtype differs from finalized member dtype. |
| `ImageMemberSizeMismatch { device, value, expected, actual }` | Input bytes differ from finalized member size. |
| `ImageSizeUnsupported { device, bytes }` | Finalized image size cannot fit host `usize`. |
| `ImageMemberOutOfBounds { device, value }` | Offset/length arithmetic or host slice is outside the image. |
| `ImageMembersOverlap { device, first, second }` | Sorted image members overlap. |
| `LoopExternalTransfer { task }` | A loop task performs external ingress/egress. |
| `MissingPredictionOutput` | A planned/finalized/exited prediction image or output set is empty or incomplete. |
| `DuplicatePredictionOutput { value }` | One logical output or task appears more than once. |
| `UnexpectedPredictionOutput { task, value }` | An output task/value is not the declared prediction, or an extra exit task exists. `value: None` identifies an unexpected task with no logical mapping. |
| `PredictionOutputImagesOverlap { first, second }` | Distinct output images overlap at the same device location. |
| `PredictionOutputDTypeMismatch { expected, actual }` | Physical prediction dtype differs from its contract. |
| `PredictionOutputSizeMismatch { expected, actual }` | Physical or host prediction bytes differ from its contract. |
| `PredictionOutputSourceMismatch { task }` | Completed image source differs from the finalized plan source. |
| `LoopDidNotReachTerminalState` | The variant is defined and displayed, but the current inference lifecycle maps this condition to an `ExecutorError::LifecycleInvariant` and then `Executor`; there is no direct constructor in the current workspace. |

### Boundary and lifecycle propagation

`validate_compiled_inference_boundary` and its KNN counterpart are called by
both the image builders and the full local execution functions. They enforce
one loop iteration, no user metrics, canonical contiguous boundary tensors,
closed input sets, and exact prediction contracts. `pack_inference_input_images`
then enforces unique devices/members, exact dtype/size, non-overlap, and that
every declared input is bound. The builders return these structural errors
without preparing or handing off a backend.

`prepare_and_execute_local_inference` and
`prepare_and_execute_local_knn_inference` then prepare the program, recheck the
finalized one-iteration bundle, reject loop transfers and user metrics, map
planned output tasks, hand off the backend, and run `init -> loop -> exit`.
Before a journal can be allocated, executor bounds failures produce an
`Executor` error whose `InferenceRunFailure.journal` is `None`. After handoff,
`inference_executor_failure` retains the run and bundle identities, optional
journal, first ordered cleanup error, and executor source while dropping the
backend. The executor still attempts remaining teardown operations.

After a successful exit, `collect_inference_prediction`,
`collect_knn_inference_predictions`, and
`validate_completed_prediction_images` enforce the planned task set, logical
values, source locations, dtype, byte counts, uniqueness, and non-overlap. Any
failure is wrapped in `PostExitValidation`, preserving the completed journal
and proving that native cleanup already ran. Only a fully validated prediction
produces `CompletedInferenceExecution` or `CompletedKnnInferenceExecution`.

## End-to-end failure behavior

The real public boundaries therefore have these observable consequences:

* A compile or preparation error returns before a graph, native bundle, or
  prepared execution exists.
* A checkpoint decode error returns no artifact and retains its exact path and
  decode class. A checkpoint save error leaves the temporary file guarded; the
  target is untouched until the atomic rename, although a post-rename parent
  sync can still report `Io`.
* Resume compatibility is checked before admission, but admission itself is
  sequential and non-transactional. Later incompatibility can follow earlier
  input-byte replacements.
* Training structural errors occur before native handoff. Native/executor
  errors occur during the lifecycle and prevent `CompletedTrainingExecution`.
* Inference executor failures retain run/cleanup evidence after handoff, while
  post-exit prediction failures retain the completed journal through
  `PostExitValidation`. No error variant is converted into a successful result.
