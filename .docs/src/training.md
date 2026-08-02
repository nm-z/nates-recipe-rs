# `src/training.rs`: public training boundary

This document describes the implementation that is present in `src/training.rs` and the concrete crates that it calls. It is an implementation trace, not a promise that every declaration in the wider Recipe grammar executes. The file is the root-library boundary between immutable `Data`, `Model`, and `Train` declarations and either a native dense training lifecycle or a semantic KNN/Bayesian preparation result.

Source links use the current checkout. The primary implementation is [`src/training.rs`](../../src/training.rs). The public declarations are in [`src/api.rs`](../../src/api.rs), data preparation is in [`src/data_prepare.rs`](../../src/data_prepare.rs), native scope construction is in [`src/native_prepare.rs`](../../src/native_prepare.rs), and the static compiler and executor are the `training` workspace crate.

## Position in the public call path

Recipe builder calls only record declarations. `Data::set`, `target`, `exclude`, `split`, and `norm` store paths, names, filters, a split fraction, and a normalization choice; they do not read files. `Model` stores layer or Bayesian declarations, an objective, gradient policy, or an optional checkpoint path. `Train` stores the epoch, optimizer, schedule, metrics, resume declaration, and save declaration. The builders retain the latest values in the thread-local facade sequence (`src/facade.rs:135-190`) so the fluent spelling can be resolved at the run boundary.

The normal user path is:

```text
recipe.data(...).target(...).split(...)
recipe.model().layer(...).loss(...)
recipe.train().optimizer(...).epochs(...).run()
        |
        +-- take_recipe_training_sequence() -> (Data, Model)
        +-- Train::try_run_with(data, model)
             +-- Bayesian model declarations -> compile_bayes_model
             +-- one KNN block             -> compile_knn_model
             `-- otherwise                 -> compile_training_package
                    -> compile_training_graph
                    -> optional semantic/native resume
                    -> current measured native preparation
                    -> init -> loop -> exit
                    -> TrainingReport and optional artifacts
```

`Train::run` consumes the remembered data and model. Missing either declaration is returned as `TrainingError::Unsupported` with the facade diagnostic (`src/training.rs:848-861`). The source frontend's hidden `.run(&model, &data)` lowering calls `__recipe_run_with`, which uses the same `try_run_with` path and does not create a second implementation (`src/training.rs:863-867`).

## Boundary types and completed reports

### `TrainingError` and `TrainingResult`

`TrainingError` is the typed error boundary for all work performed by this module (`src/training.rs:79-181`):

| Variant | Origin and meaning |
| --- | --- |
| `Declaration(DeclarationError)` | Deferred or final validation failure in `Data`, `Model`, or `Train`. |
| `Data(DataPreparationError)` | Bounded source ingestion, semantic inference, selection, target/split preparation, or lossless matrix preparation failed. |
| `Compile(TrainingCompileError)` | Dense task resolution, feature/target lowering, graph construction, primitive lowering, shape or schedule validation failed. |
| `Checkpoint(CheckpointError)` | Semantic checkpoint construction, resume compatibility, output mapping, model save, or native-kernel save failed. |
| `Resume(InferencePreparationError)` | An existing dense/KNN/Bayesian semantic resume file could not be read or decoded through the bounded model loader. |
| `NativeKernelSource(SourceError)` | A requested native resume file could not be read under the source-byte limit. |
| `Native(NativePreparationError)` | The measured profile, current device identities, toolchain/target plan, bindings, or native preparation scope failed. |
| `Unsupported { detail }` | The declaration is valid facade syntax but has no implementation in this training boundary, or violates a branch-specific policy. |
| `Runtime { stage, detail }` | A run-time operation such as signal installation, host tuning, metric presentation, or native execution failed. |

Display prefixes preserve the stage, for example `prepare training data`, `compile training graph`, `load training resume model`, `prepare current native system`, and `execute native training`. The error source chain is retained for all wrapped error types; `Unsupported` and `Runtime` intentionally have no source object (`src/training.rs:108-141`).

`TrainingResult<T>` is `Result<T, TrainingError>`. Conversion implementations at `src/training.rs:145-179` keep downstream error categories visible instead of rewriting them into one generic string.

### `TrainingModelKind`, `TrainingReport`, and payload invariants

`TrainingModelKind` has exactly three variants: `Dense`, `Knn`, and `Bayes` (`src/training.rs:183-189`). The private `TrainingReportPayload` stores the corresponding completed object (`src/training.rs:191-196`). `TrainingReport` adds `validation_status` and `gracefully_stopped` (`src/training.rs:198-206`).

The dense constructor is called only after native execution has exited. It scans the completed `RunJournal` for `LogicalEvent::LoopStopAccepted` and then constructs `CompletedTrainingCheckpoint` from the execution and a manifest (`src/training.rs:223-239`). This ordering means a report never exposes a live native handle or an incompletely finalized checkpoint.

The report accessors are intentionally branch-specific:

- `kind()` always identifies the payload.
- `run()`, `bundle()`, `journal()`, `native_kernels()`, `native_evidence()`, and `training_evidence()` return values only for dense execution. KNN and Bayesian preparation have no native run and return `None`.
- `external_outputs()` and `metrics()` expose dense post-exit values and newest metric samples. Reference-only KNN and Bayesian reports return empty slices.
- `validation_status()` reports whether requested dense validation had known target rows. KNN and Bayesian reports use `NotRequested`.
- `gracefully_stopped()` is true only when the executor accepted the host stop request at a loop boundary.
- `knn_model()` and `bayes_model()` expose only the matching semantic artifact.

`save_model` dispatches to the dense checkpoint, KNN model artifact, or Bayesian model artifact. `save_native_kernel` is implemented only for dense; KNN and Bayesian payloads return an explicit unsupported error because they have no training kernel (`src/training.rs:369-388`).

## Public declaration inputs

### Data and preparation contract

`Data` is validated before any source access. `prepare_data` in `src/data_prepare.rs:79-172` applies finite ingest limits, requires at least one target and an explicit train split, distills files/directories/ZIP containers into one table, infers typed vectors, applies column and row exclusions, fits vector metadata on the training prefix, and returns a `PreparedDataset`. The source order, retained source-row indices, vector schemas, target source indices, training partition, and validation partition remain explicit. No normalization or lossy scalar conversion occurs in this boundary.

Rows are filtered before splitting. The train fraction is represented as an exact `TrainFraction`; the first `train_rows` retained rows form the training partition and the rest form validation. Schemas and dictionaries are fitted from training rows, then applied to all retained rows. The prepared object carries source rows and retained positions, so later lowering can report source-row failures and preserve declaration order.

The preparation error surface includes invalid declarations, missing targets, missing split, f32 predicate values outside finite non-underflowing f32, bounded ingest/source failures, semantic inference failures, and selection/preparation failures. The data boundary fails closed and does not return a partial dataset.

### Model declarations that reach this file

`Model::validate` enforces nonempty inline/checkpoint/Bayesian content, KNN standalone placement, grouped routing adjacency, valid Bayesian DAG structure, and checkpoint-versus-inline exclusivity (`src/api.rs:1462-1526`). `src/training.rs` then applies branch-specific constraints.

For dense compilation, `require_supported_model` (`src/training.rs:1780-1805`) rejects a loaded weight source, any Bayesian dependencies, a model-referenced objective, and a missing objective. The accepted objectives are the built-ins `BinaryCrossEntropy`, `MeanSquaredError`, `MeanAbsoluteError`, `CrossEntropy`, `Huber`, and `Focal`, mapped to `DenseLoss`.

`map_dense_block` translates the public layer list to `DenseBlock` values (`src/training.rs:1902-2025`). The translation accepts:

- embedding with a required immediately following `.vocab(...)`;
- attention with a nonzero head count;
- vanilla RNN, GRU, and LSTM with a nonzero width, but no chained recurrent-block operations in the current first case;
- convolution with nonzero filters and kernel and a mapped activation;
- pool and K-means reductions, retaining grouped-to-neuron routing when the facade supplied it;
- LightGBM, CatBoost, XGBoost, and forest tree blocks with nonzero tree/depth values;
- residual branches whose layer widths and activations are mapped and whose resolved output width must equal the facade's retained width;
- ordinary dense and perceptron blocks with ordered activation and layer/batch-normalization operations.

`map_dense_layer` deliberately handles only `Dense` and `Perc`; all structured variants are handled by the preceding `map_dense_block` arms. Every width is converted to a nonzero `u64` by `map_dense_width`, and tree depth is checked for `u32` representability and nonzero value (`src/training.rs:2028-2081`). Activation mapping covers linear, cosine, exponential, signed logarithm, natural logarithm, Huber, tangent, ReLU, leaky ReLU, sigmoid, tanh, SELU, GELU, SiLU, ELU, and PReLU (`src/training.rs:2102-2121`).

### Train policy and artifact declarations

`Train` is static policy. Its builder methods do not probe hardware, parse data, compile a graph, or start a run (`src/api.rs:1845-1888`). The methods and their stored meaning are:

- `.epochs(n)` sets a finite nonzero horizon. Omitting it leaves an unbounded horizon.
- `.lr(rate)` requires a finite positive f32-representable rate and also selects linear decay. `.cos()` and `.exp()` select cosine or exponential decay.
- `.warmup(n)` sets a nonzero warmup epoch count; final validation requires warmup less than a finite epoch bound.
- `.optimizer(Optimizer::AdamW)` records the optimizer. Dense compilation rejects any other value or an omitted value.
- `.log(items).every(n)` records metric declarations and per-declaration cadence. `.plot(items)` records plot requests.
- `.resume(path)` accepts exactly one semantic `.ogdl` model path. The hidden `__recipe_resume_pair(model, kernel)` preserves the literal two-path form and requires a `.ogdl` first path plus a `.cubin` or `.hsaco` second path.
- `.save(path)` accepts one `.ogdl`, `.cubin`, or `.hsaco` path. The hidden `__recipe_save_pair(model, kernel)` preserves the literal two-path form and requires `.ogdl` followed by a native extension.

Repeated resume or save declarations are deferred errors. `Train::validate` rechecks deferred errors, warmup ordering, and all log/plot items before compilation (`src/api.rs:2081-2094`). A missing resume file is not itself a declaration error and is handled conditionally at preparation time.

## Dispatch in `Train::try_run_with`

`try_run_with` is the single branch point (`src/training.rs:869-917`):

1. If the model has any Bayesian dependencies, it calls `compile_bayes_model`. The resulting semantic artifact is wrapped in a Bayesian report and saved to the requested model destination, if any. No native preparation or execution is performed.
2. Otherwise, if any layer is `LayerSpec::Knn`, it calls `compile_knn_model`. The immutable reference model is wrapped in a KNN report and optionally saved as a semantic model. No optimizer loop or native kernel exists.
3. Otherwise it calls `compile_training_package`, installs the SIGINT guard, executes the dense native path, constructs the report, and writes requested model and kernel artifacts after execution.

The branch order makes Bayesian declarations authoritative if a malformed mixed model contains both Bayesian dependencies and layers. Normal `Model::validate` rejects incompatible combinations before the branch-specific compiler is entered.

## KNN semantic preparation

`compile_knn_model` validates the declarations and requires exactly one standalone `.knn(neighbors)` block (`src/training.rs:417-498`). It rejects Bayesian dependencies, loaded models, objectives, gradient policy, optimizer/lr/schedule/warmup/epochs, log/plot metrics, native resume kernels, and native save destinations. Neighbor count must fit `u64` and be nonzero.

If a semantic resume path is declared and exists, it is read through the bounded KNN loader. A missing path means no saved references. `prepare_data` then builds the complete training partition. `prepare_knn_reference_set` lowers typed features, preserves prepared row order as deterministic distance-tie order, and creates one output per declared target. Numeric outputs use a uniform mean. Nonnumeric outputs use a deterministic mode and retain the label dictionary. Missing references are retained in row alignment but masked independently per output. The optional data normalization is recorded as a semantic choice but no optimizer state is created.

When a saved artifact exists, `KnnModelArtifact::continue_with` validates the saved/current neighbors, vector schema, feature spans, normalization, and operations, then appends current references after saved references. Duplicate observations are retained because row multiplicity is part of the declaration. The returned report has no run, bundle, journal, metrics, native kernels, or execution evidence.

## Bayesian semantic preparation

`compile_bayes_model` validates the declarations and requires at least one `.bayes(child, parents)` dependency (`src/training.rs:500-579`). It rejects layer blocks, loaded models, generic objective/gradient policy, numeric normalization, optimizer/lr/schedule/warmup/epochs, log/plot metrics, native resume kernels, and native save destinations.

An existing semantic resume path is loaded with bounded Bayesian decoding if it exists. `prepare_categorical_bayesian_reference_sets` resolves the acyclic observed categorical DAG, requires each child to be a declared target and each parent to be a feature, and requires the target source order to equal dependency order. It stores exact source rows, dictionary codes, parent cardinalities, mixed-radix configuration, and child codes. Native inference owns histogramming and Laplace posterior calculation; this path does not claim a generic optimizer loop.

Bayesian `continue_with` appends current observed rows after saved observations after validating format, smoothing, conditional count, and schema contracts. As with KNN, the report is semantic-only and cannot expose a native run or kernel.

## Dense graph compilation

### `compile_training` and `compile_training_package`

`compile_training` is the public compile-only boundary. It calls `compile_training_graph` and discards the optional loaded semantic checkpoint (`src/training.rs:408-415`). It performs declaration validation, data preparation, model/policy checks, and graph construction only. It never probes hardware, compiles native artifacts, allocates device memory, or executes.

`compile_training_package` retains the optional semantic resume checkpoint, calls `load_resume_native_bundle` to authenticate an optional native resume image, and builds a `CheckpointManifest` from the compiled graph (`src/training.rs:581-590`). The package has three fields: `CompiledTraining`, its row-free `CheckpointManifest`, and an optional `ResumeNativeBundle` containing topology/discovery/target/toolchain identities plus exact kernel bytes.

### Configuration derived from the public policy

`compile_training_graph` first calls `policy.validate`, `data.validate`, `model.validate`, `require_supported_model`, and `require_supported_policy` (`src/training.rs:592-603`). `require_supported_policy` requires `Optimizer::AdamW`, an explicit learning-rate schedule, and a semantic resume path whenever a native resume kernel is supplied (`src/training.rs:1807-1824`).

The compiler then:

1. prepares the data once;
2. maps every declared model block;
3. retains a flat `layers` view only when every block is an ordinary `DenseBlock::Layer`, otherwise the structured block list is authoritative;
4. maps a finite nonzero `.epochs(n)` to `TrainingHorizon::Finite`, or uses `TrainingHorizon::Unbounded` when no bound is declared;
5. converts warmup to `u64` and uses AdamW's default learning rate only when `.lr` was omitted;
6. maps schedules. Linear decay is constant for an unbounded run, while cosine and exponential decay require a finite endpoint. Omitting a schedule is an unsupported dense declaration;
7. selects input normalization. An embedding-first model requires no numeric normalization and uses `Identity`. An embedding with declared normalization is rejected. Non-embedding dense training requires explicit Z-score, min-max, or L2 normalization;
8. builds `DenseTrainingConfig` with mapped layers, loss, horizon, warmup, decay, optional gradient clip, normalization epsilon, the semantic reduction-tree ceiling, a fixed Recipe seed, and AdamW defaults with the requested learning rate.

The reduction-tree lane value is a semantic ceiling. The training compiler and native lowering select the physical reduction shape from tensor extent and measured hardware. It is not a device-specific hard-coded execution width.

### Validation selection and compile entrypoint

Validation configuration is derived from requested log/plot metrics (`src/training.rs:1826-1900`):

- binary metrics are requested by AUROC, AUPRC, Brier, calibration error, or accuracy with BCE/Focal. They require BCE or focal loss and use a fixed 15-bin calibration configuration;
- multiclass validation is requested by accuracy with categorical cross entropy;
- regression validation is requested by R2 with MSE, MAE, or Huber;
- incompatible metric/objective combinations return `Unsupported`;
- one run may select at most one validation family. The dispatch match rejects multiple families.

The structured-block flag selects one of eight `recipe_training` compile entrypoints: flat or structured blocks, each with no validation, binary validation, multiclass validation, or regression validation (`src/training.rs:702-745`). Each entrypoint emits a `CompiledTraining` whose graph contains all declared calculations and transfers, external input images, typed dataset schema, metric bindings, output tensors, and a finite or unbounded loop horizon.

The downstream compiler guarantees one logical optimizer update over the complete prepared training partition per epoch. Physical realization may tile the matrix, but it cannot change that semantic unit. If target rows are missing or unseen, supervision masks loss and gradients; the accepted update plan rejects more than one accepted update per epoch. An entirely unsupervised training partition therefore does not advance AdamW state.

### Lowered data and task semantics

`recipe_training::compile_dense_training_impl` constructs a `DenseFeaturePlan` and a `LoweredDenseDataset`. Numeric feature vectors are one scalar wide. Categorical dictionary vectors are one-hot expanded with an explicit reserved unseen/missing route. Numeric integer values must be exactly representable in f32 when converted for dense calculation. Variable-width, missing numeric, non-finite, unsupported semantic, inconsistent-row, and out-of-range category values fail compilation.

The loss and target schema determine the dense task:

- one numeric/binary target with BCE or focal becomes binary classification;
- one categorical dictionary target with cross entropy becomes multiclass classification with one reserved unseen class;
- one numeric target with MSE, MAE, or Huber becomes scalar regression;
- multiple homogeneous numeric targets become multi-target binary, joint one-hot multiclass, or multi-target regression according to the loss.

The compiler retains target source indices in declaration order. Multi-target binary rows must contain exact zero/one values; joint multiclass rows must be exact one-hot vectors; non-finite target values and unknown categorical codes fail. Validation rows are compacted to known targets for metric calculation, while split-row count remains available for an unavailable-status message.

Normalization is emitted as graph calculations, not host preprocessing. Z-score computes training means and variances, min-max computes training minima and maxima, and L2 normalization computes row norms. Validation features reuse the training state. Categorical one-hot spans are masked out of numeric normalization.

The graph then emits validity/supervision masks, forward blocks, loss and loss gradient, masked means, reductions, backward gradients, optional global gradient clipping, dynamic AdamW scalars, parameter and moment updates, validation metrics, and external outputs for semantic model state. The `CompiledTraining` contains a `StaticCalculationProgram`, owned external inputs, `TrainingBounds`, `TrainingOutputs`, schema, config, effective blocks, and any output adapter (`training/src/model.rs:2698-2741`).

### Semantic resume in the dense graph

If `.resume("model.ogdl")` exists, `compile_training_graph` loads it with `CheckpointDecodeLimits::default`, applies it to the newly compiled graph, and retains the artifact (`src/training.rs:746-758`). If it does not exist, compilation continues from fresh initialization.

`apply_checkpoint_resume` validates the artifact and current manifest, then replaces the compiled external images for the resume-enable scalar, every parameter's value, first moment, and second moment, plus K-means centroids and tree split state where present. It requires exactly one resume-enable input and all three moment components for every parameter. Compatibility includes:

- feature width and feature spans;
- target source indices, dense task, output adapter, and row-free vector schema;
- objective, data normalization, normalization epsilon, and AdamW beta/epsilon/weight-decay semantics;
- effective block topology and tensor dtypes/shapes/byte lengths.

The static graph and value identities do not change. The new declaration supplies the new phase's epoch horizon, warmup, learning-rate choice, and schedule. Schedule position, accepted-update counters, automatic stopping state, and exact optimizer progress are not restored as semantic artifacts. `recipe_training::REMAINING_UNSUPPORTED` explicitly lists dynamic loop shortening and exact optimizer resume.

### Optional native resume image

`load_resume_native_bundle` is conditional on all three facts: a kernel path was declared, a semantic checkpoint was loaded, and the kernel file exists (`src/training.rs:761-846`). A missing kernel file returns `None`, causing current-system realization from the semantic model metadata.

The extension selects `NativeKernelFormat::Cubin` or `NativeKernelFormat::Hsaco`. The checkpoint must contain native realization metadata. The current compiled program's canonical OGDL digest must equal the checkpoint's recorded program digest. The metadata must contain exactly one kernel of the requested format. The file is read through `read_source_snapshot` under the checkpoint source-byte limit, and its digest must equal the authenticated kernel digest. The resulting bundle carries the saved topology, discovery, target, toolchain, and exact bytes.

The program digest intentionally excludes external input images, including resumed parameter bytes. It authenticates the static graph and lets a semantic resume phase reuse a kernel only when the graph is identical.

## Native preparation and runtime tuning

`execute_current_training_native` obtains the current measured profile and scoped CUDA/HSA bindings through `with_current_native_preparation` (`src/training.rs:1278-1338`). With a supplied resume image it first requires matching topology and discovery identities, then requires a current target-build specification with the same target and toolchain identity. It constructs:

1. measured host/GPU bindings, host plan, and target plan;
2. `NativeRuntimeTuning` from the graph and measured profile;
3. a host backend configuration with a run-scoped `RunId`, worker-thread count, and staging bytes per worker;
4. a staged cross-backend bridge, production local candidate factory, native executor driver, deferred compiler, and optional prebuilt bundle;
5. a `NativeCandidateRealizer`, artifact provider/catalog, and `Preparer`;
6. `prepare_and_execute_local_training_controlled` with the watchdog, optional metric observer, and graceful-stop control.

`derive_native_runtime_tuning` is measured-system derived (`src/training.rs:1011-1276`):

- it selects discovered devices belonging to the measured local machine and fails if none exist;
- host lanes are the sum of measured RAM/Disk transfer lanes, bounded by each device's submission queues;
- worker threads are the minimum of available host parallelism, measured host lanes, and graph tensor opportunities, with a minimum of one;
- staging bytes per worker are bounded by the largest graph tensor, measured storage or RAM buffer scale, and local RAM divided by worker count;
- maximum transfer bytes include the largest tensor, aggregate external input bytes, and aggregate external output bytes;
- expected operation duration is the larger of measured calculation time for the graph's maximum FLOP work and measured transfer time for the largest image, using the slowest local calculation/transfer/link rate and a minimum one-nanosecond duration;
- watchdog safety is the sum of measured calculation concurrency and transfer concurrency, clamped to the nonzero `u32` domain.

Overflow, missing local RAM, missing calculation rate, missing transfer rate, no local devices, or impossible host parallelism produces a typed `Runtime` error which is then wrapped as native local configuration failure.

`next_run_id` combines the current UNIX-time nanoseconds, process identity, and an atomic per-process sequence, then forces a nonzero `RunId` (`src/training.rs:1770-1778`). It is used only after compilation and before native preparation.

## Executor lifecycle and authoritative state

The controlled executor function in `training/src/execute.rs:2176-2294` is the complete local lifecycle:

1. It rejects an unbounded graph unless a `TrainingExecutionControl` stop source is present.
2. It asks `Preparer` to build and finalize the exact native bundle from the measured profile.
3. It rejects any loop-phase transfer whose source or destination is external. External admission and egress belong to init and exit.
4. It packs the compiled external inputs into exactly one finalized device image per device. A logical input may be copied into several device images, but admission occurs once per device in init.
5. It maps finalized exit transfer tasks back to logical `ValueId`s and allocates final metric slots for user metrics.
6. It hands the warmed native session to the executor backend, creates `PreparedRun`, and calls `initialize(images)`. This is the singular external input admission point.
7. It starts the loop and polls with progress. Host stop state is observed only at complete loop-iteration boundaries. A non-progressing poll uses bounded backoff; progress resets it.
8. It drains user metrics without allowing the observer to delay polling. After loop completion or accepted graceful stop, it transitions to an exited loop and calls `exit()`.
9. It collects post-exit external output images, native evidence, the mailbox's newest metric samples, and the `RunJournal`. Outputs are sorted by task and mapped to logical values.
10. It returns `CompletedTrainingExecution` only after native resources have been torn down.

The loop cannot read files, admit data, or egress external outputs. The static graph, finalized bundle, executor events, and post-exit images are the authoritative state. `TrainingExecutionEvidence` joins compiled full-partition bounds with actual journal submissions and records logical updates per epoch, optimizer parameter kernels/tasks/submissions, started/completed loop iterations, loop calculation task counts, non-GPU calculation submissions, and compacted event/call counts.

`TrainingReport::dense` then builds the checkpoint wrapper and derives `gracefully_stopped` from the authoritative journal. A successful dense report therefore contains the exact native images that were built, loaded, warmed, executed, and destroyed, not a reconstructed or test-only status.

## Live metrics, logs, plots, and validation availability

Before execution, `execute_current_training` emits one validation-unavailable line when the compiled status is `Unavailable` (`src/training.rs:920-967`). The line distinguishes binary, multiclass, or regression family, reason (`no-known-targets` or `single-known-class`), known rows, and split rows.

`live_metric_presentations` examines compiled metric bindings and the public log/plot declarations (`src/training.rs:1341-1432`). Training loss, validation BCE/cross entropy, accuracy, R2, learning rate, AUROC, AUPRC, Brier, calibration error, and recall-at bindings are mapped to presentation labels. Only bindings selected by the policy are presented. Epoch/time host requests force a training-loss binding to trigger row assembly even when loss itself is not logged.

If no live metric is selected, native execution runs without an observer and only static device reporting is emitted. If any live metric or host cadence is selected, the module creates a bounded channel with capacity 256 and cadence one, starts a `recipe-live-metrics` presenter thread, and passes the observer to native execution. `TrainingMetricObserver` uses `try_send`; a full or disconnected consumer increments dropped statistics and never backpressures executor polling. Execution and presenter errors are joined and preserved (`src/training.rs:920-957`).

The presenter batches samples by executor iteration, exposes one-based user epochs, writes rows when each metric's cadence is due or at the final epoch, flushes each row, and emits bounded plots after the channel closes. `BoundedPlotSeries` retains at most 64 values per label, tracks total sample count, ignores non-finite values for range calculation, and renders Unicode levels with an N/A range when no finite sample exists (`src/training.rs:1562-1748`). Metric values render f32 with fixed width and four decimals, integer values as integers, and NaN as `N/A`. Fields use a fixed twelve-color ANSI palette.

After a successful run, `report_static_training_metrics` handles the `Device` log/plot item by collecting unique `backend:architecture:abi` identities from realized native kernels. KNN and Bayesian reports never print training metrics because no iterative native training occurred.

## Saving and resuming artifacts

### Dense model and native kernel

`CheckpointManifest::from_compiled` captures row-free vector schemas, feature spans, feature width, target source indices, task, output adapter, dense config, bounds, normalization tensors, effective block topology, parameter/moment roles, and the canonical program digest. It contains no dataset rows, device handles, queues, or executable bytes. `CompletedTrainingCheckpoint::new` attaches authenticated native realization metadata after execution; native bytes remain separate from the semantic OGDL image.

When a dense model destination is present, `TrainingReport::save_model` calls `CompletedTrainingCheckpoint::save`. The checkpoint writer validates output identity, dtype, and byte length, serializes the semantic model and external output tensors as canonical textual OGDL, and writes atomically. When a native destination is present, `save_native_kernel` requires the destination extension to match the requested format, finds exactly one realized image of that format, and writes its exact bytes atomically. Missing or ambiguous format images are typed checkpoint errors.

`try_run_with` performs both writes only after native execution and report construction (`src/training.rs:897-916`). A one-path `.save` selects one artifact by extension. A literal two-path save selects both. Omitting `.save` writes nothing. The implementation does not export journals, plans, profiles, caches, or intermediate checkpoints as public artifacts.

### Semantic resume and native resume

`.resume` and `.save` are independent declarations. A missing semantic resume model starts a new phase and does not suppress a declared save. A supplied native resume path is valid only with a semantic model path, and the semantic model must be `.ogdl`; a kernel alone is rejected by `require_supported_policy` and by the facade declaration validation.

If no native resume path is supplied, a needed native kernel is realized from the compiled semantic graph for the current measured target plan. If a native path is supplied but absent, the same recompilation path is used. If it exists, its authenticated topology, discovery, target, toolchain, static-program digest, and byte digest must all match the current measured system and semantic checkpoint before it is handed to the compiler as a prebuilt bundle.

KNN and Bayesian resume files are semantic `.ogdl` artifacts with their own roots and strict bounded decoders. KNN appends immutable reference rows. Bayesian artifacts append exact observed conditional rows. Neither branch accepts a native resume or emits a native kernel.

## Failure boundaries and explicit unsupported surface

The module deliberately distinguishes declaration errors from unsupported semantics and from runtime failures. Important concrete rejection boundaries include:

- no preceding facade data/model declaration;
- empty or invalid data source, missing target, missing split, no retained rows/features, unmatched exclusions, invalid predicates, and bounded ingest failures;
- dense model without a built-in objective, with a loaded weight source, with Bayesian dependencies, or with a referenced objective;
- dense policy without AdamW, without an explicit schedule, with a kernel resume lacking a semantic model, with multiple validation families, or with cosine/exponential decay and no finite epochs;
- an embedding without immediate vocabulary, embedding plus numeric normalization, recurrent block operations in the first supported case, zero/nonrepresentable widths, zero tree depth, invalid residual output width, and unsupported layer operations;
- dense features/targets that are variable width, missing, non-finite, not exactly f32-convertible, wrong dtype/shape, invalid categorical code, invalid binary value, or invalid one-hot row;
- empty or oversized prepared partitions, i32 index/schedule limits, arithmetic/identity overflow, and accepted-update plans requiring more than one full-partition update per epoch;
- semantic resume schema/objective/normalization/topology/parameter incompatibility, malformed OGDL, missing authenticated native metadata, wrong native extension, ambiguous native realizations, or native byte digest mismatch;
- measured-profile absence, stale identity, missing local GPU/RAM/rates, changed current native configuration, invalid target/toolchain binding, candidate preparation failure, loop external transfer, watchdog/loop/executor failure, or inability to present live metrics.

Several declarations are intentionally surfaced as unsupported rather than silently ignored. In particular, generic referenced objectives, checkpoint-backed dense `.model().load(...)` training, optimizer state exact continuation, dynamic loop shortening, native execution for KNN/Bayesian preparation, and mixed validation families do not fall through to a plausible but incorrect implementation.

## Function and structure inventory

| Source range | Role |
| --- | --- |
| `1-77` | Imports, live-channel constants, metric palette, and run sequence. |
| `79-181` | `TrainingError`, conversions, and `TrainingResult`. |
| `183-390` | `TrainingModelKind`, report payload, report construction/accessors, and artifact dispatch. |
| `392-406` | Dense compile package and authenticated native-resume bundle. |
| `408-415` | Public dense compile-only boundary. |
| `417-498` | KNN semantic preparation and optional continuation. |
| `500-579` | Bayesian semantic preparation and optional continuation. |
| `581-760` | Dense package/graph compilation, policy mapping, validation selection, and semantic resume. |
| `761-846` | Native resume path loading and digest/identity authentication. |
| `848-918` | Public `Train::run`, frontend lowering target, branch dispatch, and post-run saves. |
| `920-1009` | Validation availability, native execution selection, and static device reporting. |
| `1011-1276` | Measured native runtime tuning and watchdog derivation. |
| `1278-1338` | Current native scope, compiler/realizer construction, and controlled executor handoff. |
| `1340-1501` | Metric presentation planning and public metric-to-binding selection. |
| `1503-1560` | Presenter thread, channel draining, panic/error conversion, and row writes. |
| `1562-1768` | Metric row aggregation, cadence, bounded plots, formatting, and ANSI fields. |
| `1770-1778` | Nonzero run identity generation. |
| `1780-1824` | Dense objective and policy support gates. |
| `1826-1900` | Binary, multiclass, and regression validation configuration. |
| `1902-2121` | Public layer-to-training block/operation/activation lowering and extent checks. |

The implementation's central invariant is direct data flow: immutable declarations are validated and prepared once, the prepared typed partition is lowered into one static calculation graph, measured native preparation realizes that graph, the executor owns authoritative lifecycle state, and only post-exit values become a report or an optional public artifact.
