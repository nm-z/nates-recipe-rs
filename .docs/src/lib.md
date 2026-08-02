---
file: src/lib.rs
crate: recipe
package_version: 0.1.1
edition: "2024"
role: root-library-facade
intent: >
  Present one dependency-clean public entry point for immutable Recipe
  declarations, bounded preparation, static compilation, measured native
  execution, reports, and advanced lower-level modules.
side_effect_rule: declaration_builders_do_not_read_data_probe_hardware_compile_allocate_or_execute
terminal_rule: train_run_and_infer_evaluate_are_the_only_declaration_facade_model_execution_paths;_cli_main_is_a_process_runner
authority:
  - src/lib.rs
  - src/facade.rs
  - src/api.rs
  - src/data_prepare.rs
  - src/training.rs
  - src/inference.rs
  - src/native_prepare.rs
  - src/cli.rs
---

# `src/lib.rs`

## Intent

`src/lib.rs` is the root crate boundary for the `recipe` package. It does not
implement a second runtime. It declares the four public root implementation
modules, keeps three lifecycle and source-runner helpers private, lifts the
small set of result and evidence types that users need, and lexically includes
`src/facade.rs` into the crate root. The included facade supplies the ergonomic
`recipe.data(...).target(...); recipe.model()...; recipe.train().run()` and
`recipe.infer().evaluate()` surface, while the public implementation modules
own preparation and execution orchestration.

The root therefore has two deliberately different surfaces:

1. The declaration surface is immutable intent. `Data`, `Model`, `Train`, and
   `Infer` contain strings, enums, vectors, and validated declaration facts.
   Their builder methods perform local shape and policy checks, but do not read
   a file, inspect a GPU, compile an artifact, allocate a buffer, or submit a
   task.
2. The execution surface is explicit. `Train::run` and `Infer::evaluate` consume
   the immediately preceding declarations, perform bounded preparation, use
   the current measured native profile, prepare one fixed-point execution
  bundle, execute the real lifecycle, tear resources down, and return a typed
   report. The standalone `compile_*` and data/native preparation functions
   stop before native execution and are useful to advanced callers.

The root keeps these surfaces connected without allowing runtime handles or
mutable domain state to enter declarations. Native CUDA contexts and HSA
sessions are lent only through higher-ranked callbacks in
`native_prepare`; they cannot escape into `Data`, `Model`, or policy values.

The same structure in a compact machine-readable record is:

```yaml
structure:
  public_modules: [data_prepare, inference, native_prepare, training]
  private_modules: [signal, source_frontend, validation_reporting]
  included_files: [facade.rs, cli.rs, api.rs]
  root_namespaces: [api, engine, operations, cli]
  root_values: [recipe]
  root_types: [Recipe]
  terminals: [Train::run, Infer::evaluate]
  pre_execution_boundaries:
    - data_prepare::prepare_data
    - data_prepare::distill_data
    - training::compile_training
    - training::compile_knn_model
    - training::compile_bayes_model
    - inference::compile_inference
    - native_prepare::load_native_preparation
    - native_prepare::build_native_target_plan
```

## Source structure

The declarations in `src/lib.rs` are the complete lexical module graph at the
root:

| Source line(s) | Root item | Visibility | Ownership and purpose |
| --- | --- | --- | --- |
| `src/lib.rs:1` | `data_prepare` | `pub mod` | Bounded source distillation, target-free selection, semantic fitting, row predicates, and training preparation. |
| `src/lib.rs:2` | `inference` | `pub mod` | Inference compilation, native execution dispatch, prediction decoding, report construction, and report output. |
| `src/lib.rs:3` | `native_prepare` | `pub mod` | Measured-profile loading, exact local GPU reopening, build-target planning, and scoped native bindings. |
| `src/lib.rs:4` | `signal` | private `mod` | Process SIGINT installation, request flag, restoration, and child signalling. Only training and the CLI use it. |
| `src/lib.rs:5` | `source_frontend` | private `mod` | CLI-only syntax classification and source rewriting for the literal multi-argument and named-field forms in `API.ogdl`; compiler diagnostics are remapped to the original source. |
| `src/lib.rs:6` | `training` | `pub mod` | Training compilation, KNN and observed-categorical-Bayesian preparation, native dense execution, metrics, artifact saves, and reports. |
| `src/lib.rs:7` | `validation_reporting` | private `mod` | Converts unavailable validation status into the one human-readable training line; output failures belong to `TrainingError::Runtime`. |
| `src/lib.rs:33` | `include!("facade.rs")` | lexical inclusion | Adds `cli`, `api`, `engine`, `operations`, `Recipe`, the `recipe` value, and the thread-local declaration sequence directly to the root. There is no `recipe::facade` module. |

`src/facade.rs` itself includes `src/cli.rs` as the public `recipe::cli`
module and `src/api.rs` as the public `recipe::api` module. The subsequent
`pub use api::*` means the same API items are also available at `recipe::*`.
The root binary in `src/main.rs` is intentionally tiny: it calls
`recipe::cli::main()` and returns that `ExitCode`.

## Root re-export surface

The explicit re-exports in `src/lib.rs:9-31` are curated. They are not a
wildcard re-export of every subordinate crate.

### Data preparation exports

The root exports the four default aggregate bounds:

```text
DEFAULT_DATA_SOURCE_BYTES       = 1 GiB aggregate source-byte bound
DEFAULT_DATA_RECORDS            = 10,000,000 framed records, including header handling
DEFAULT_DATA_FIELDS_PER_RECORD  = 16,384 vectors in one filesystem table
DEFAULT_DATA_FIELD_BYTES        = 16 MiB per source field
```

It also exports `DataPreparationError`, `DataPreparationResult<T>`,
`prepare_data`, `prepare_data_with_limits`, `distill_data`,
`distill_data_with_limits`, and `select_target_free_data`. The dataset values
returned by these functions remain owned by `recipe_ingest`; the root exposes
that crate as `recipe::engine::ingest`, but does not duplicate its table or
prepared-dataset types.

### Inference exports

The root exports `InferenceError`, `InferenceResult<T>`,
`InferenceModelKind`, `CompiledModelInference`, `InferenceReport`, and
`compile_inference`.

`CompiledModelInference` is an immutable family-tagged program with variants
`Dense`, `Knn`, `Bayes`, and `GgufLlama`. Its `kind()` method is the only root
family inspection needed before execution. `InferenceReport` is returned only
after native teardown and exposes the run and bundle identities, one dense,
Bayesian, or GGUF prediction, typed KNN predictions when applicable, the
bounded `RunJournal`, native evidence, elapsed loop time, participating device
labels, and saved-label decoding helpers.

### Native preparation exports

The root exports the error/result pair and all plan handles from
`native_prepare`:

```text
NativePreparationError
NativePreparationResult<T>
NativeDeviceBuildTarget
NativeTargetPlan
NativeHostPlan
LoadedNativePreparation
NativePreparationScope<'cuda, 'hsa>
```

It exports `load_cached_measured_profile`, `load_native_preparation`,
`build_native_target_plan`, `with_native_preparation`, and
`with_current_native_preparation`. `NativeDeviceBuildTarget` records a measured
device, its source origin label, target identity, and toolchain identity.
`NativeTargetPlan` owns unique build specifications and can construct the
deferred compiler. `NativeHostPlan` owns measured RAM and storage origins and
creates deterministic run-scoped host bindings without performing I/O.
`NativePreparationScope` lends exact CUDA/HSA bindings, host origins, and
target plans for one callback; its lifetimes make escaping driver handles
impossible.

### Native and training evidence exports

The root lifts three native-executor evidence types:
`NativeBackendKind` (`Cuda` or `Hsa`), `NativeDeviceExecutionEvidence`, and
`NativeExecutionEvidence`. The latter records per-device image loads, entry
lookups, queues, completion objects, persistent allocations, whether teardown
completed, live resources after teardown, and loop-time realization calls. A
successful production report has zero loop-time realization calls and zero
live resources after teardown.

The root also lifts the report-facing training types from `recipe_training`:

```text
BayesModelArtifact
KnnModelArtifact
KnnInferencePrediction
KnnInferencePredictionKind
KnnLabelValue
NativeKernelFormat
RealizedNativeKernel
RealizedNativeKernelSet
TrainingExecutionEvidence
ValidationMetricFamily
ValidationMetricStatus
ValidationUnavailableReason
```

`NativeKernelFormat` is the extension-bearing `Cubin` or `Hsaco` identity;
`RealizedNativeKernel` contains the exact bytes, target, toolchain, digest, and
entry identities; `RealizedNativeKernelSet` joins those images to realization,
topology, and discovery identities. Validation status distinguishes
`NotRequested`, `Available`, and `Unavailable` with a family and reason.

The root training module exports `TrainingError`, `TrainingResult<T>`,
`TrainingModelKind` (`Dense`, `Knn`, or `Bayes`), `TrainingReport`,
`compile_training`, `compile_knn_model`, and `compile_bayes_model`.

The root does not alias `recipe_training` under `engine`; only the selected
artifact, prediction, execution-evidence, and validation types above are
lifted. Lower-level compiler graph types are implementation-owned even when a
public compile function returns one.

## Included facade and public namespaces

### `recipe::api` and root wildcard imports

`facade.rs:3-15` defines `pub mod api { include!("api.rs"); }` and then
`pub use api::*`. The following groups are the declaration vocabulary exposed
at both `recipe::api::*` and `recipe::*`.

| Group | Public types and values | Boundary represented |
| --- | --- | --- |
| Declaration errors | `DeclarationErrorKind`, `DeclarationError`, `DeclarationResult` | Builder validation is deferred into the first error and surfaced at terminal validation. |
| Data sources and predicates | `IntoTargets`, `ConditionValue`, `IntoConditionValue`, `ComparisonOperator`, `Condition`, `Exclusion`, `IntoExclusions`, `cond!` | Source names, target names, column patterns, and typed row exclusions. `cond!` stringifies the column identifier and does not evaluate it as Rust data. |
| Data policy | `Data`, `DataNormalization`, `z_score`, `min_max`, `l2_norm` | Immutable source list, targets, exclusions, optional f32 train split, and optional numeric normalization. |
| Model vocabulary | `Activation`, `LayerNormalization`, `layer_norm`, `batch_norm`, `LayerOperation`, `ResidualOperation`, `ResidualSkip`, `ForestBooster`, `GroupCount`, `GroupToNeuronRouting`, `GroupToNeuronConnection`, `LayerSpec`, `IntoLayer` | Backend-neutral model structure and local shape/routing constraints. |
| Model objective | `Loss`, `mse`, `mae`, `huber`, `bce`, `ce`, `focal`, `Grad`, `clip`, `Objective`, `IntoObjective`, `Optimizer`, `adamw`, `BayesDependency`, `Model` | Dense losses, explicit gradient policy, model references, AdamW selection, checkpoint sources, and acyclic observed categorical dependencies. |
| Training policy | `LogItem`, `IntoLogItems`, metric constants and aliases (`LossMetric`, `Loss`, `Accuracy`, `Acc`, `R2`, `AuRoc`, `AuPrc`, `Brier`, `CalibrationError`, `Epoch`, `Lr`, `Time`, `Device`, plus lowercase aliases), `LearningRateSchedule`, `Train` | Epoch and optimizer policy, logging cadence, plotting, resume paths, save paths, and no runtime state. |
| Inference policy | `Infer`, `InferenceDeclaration` | Target-free logging policy and the validated pair of loaded model plus data declaration. |

Some value/type names intentionally share a Rust name in separate namespaces,
for example the `Loss` enum and the `Loss` metric constant. `Metric` itself is
`pub(crate)` and is not part of the public API; `LogItem` is the public typed
wrapper. `__condition` is a `#[doc(hidden)]` public helper needed by the
exported `cond!` macro, and the `__recipe_resume_pair`, `__recipe_save_pair`,
and `__recipe_run_with` methods are `#[doc(hidden)]` source-frontend targets.
They are compatibility boundaries, not alternate user-facing builders.

### `recipe::engine`

`facade.rs:17-42` exposes dependency-clean implementation crates under stable
names for advanced callers. The aliases are:

| Alias | Crate responsibility at the root boundary |
| --- | --- |
| `cluster` | Identity-checked assembly of complete machine and peer measurements. |
| `core` | Dependency-free IDs, units, identities, typed graphs, plans, schedules, and topology invariants. |
| `cuda` | Reviewed CUDA Driver API discovery, contexts, runtime objects, artifact identities, and FFI. |
| `executor` | Backend-neutral typestate lifecycle and bounded run journal. |
| `host` | Preallocated RAM and disk transfer resources. |
| `hsa` | Reviewed ROCr/HSA discovery, sessions, queues, allocations, and asynchronous execution. |
| `ingest` | Bounded files, directories, archives, tables, semantic vectors, model-format readers, and encoders. |
| `kernel` | Recipe-owned scalar-to-LLVM lowering, artifact building, audit, and target identities. |
| `language` | Typed backend-neutral calculation graphs, tensors, shapes, scalar programs, and primitive kinds. |
| `math` | Recipe-owned scalar math contracts and deterministic scalar programs. |
| `native_executor` | CUDA/HSA resource realization beneath the executor boundary and native evidence. |
| `native_probe` | Bare-metal native backend probing and preparation-scoped bindings. |
| `ogdl` | Ordered graph syntax parser and serializer used by semantic artifacts. |
| `ops` | Canonical operation inventory, descriptors, lowering availability, compositions, and workspace formulas. |
| `planner` | Finite measured-topology candidate enumeration and ranking. |
| `prepare` | Fixed-point artifact selection, candidate realization, stabilization, capacity checks, and finalization. |
| `primitives` | Backend-neutral primitive lowering, dispatch geometry, resource bounds, and validation. |
| `probe` | Host/GPU discovery, bounded benchmarks, identity-keyed measured profiles, and profile caches. |
| `program` | Static calculation graph lifecycle envelope and loop iteration domains. |
| `remote` | Bounded master/worker execution over an already connected transport channel. |
| `scheduler` | Measured static schedule, routes, dependencies, and arena packing. |
| `text` | Bounded tokenizer and chat-template transformations before init admission. |
| `transport` | Bounded framed transport over a caller-provided connected stream. |

`recipe_training` is intentionally absent from this alias list. The root lifts
only its selected public report and artifact types, keeping the declaration
facade independent from the full compiler crate surface.

### `recipe::operations`

`facade.rs:44-125` is the exact operation inventory and lowering boundary. It
re-exports `ScalarProgram` from `recipe_core` and the operation descriptors,
recipes, primitive requests, materialization structures, workspace structures,
error/result types, and `LoweringAvailability` from `recipe_ops`.

The re-exported type set is:

```text
ScalarProgram
CompositionPayload, CompositionRecipe, CompositionStep, IdentityNamespace,
IterationBound, LoweredProgram, LoweringAvailability, LoweringHardware,
MaterializationRequest, MaterializedComposition, MissingConcreteComponent,
NamedTensor, NonCalculationRecipe, OperationDescriptor, OperationError,
OperationErrorKind, OperationRegistry, OperationResult, PreparedParameter,
PreparedParameters, PrimitiveRequest, RemainingComposition, ResolvedBound,
ResolvedComposition, ResolvedIteration, ResolvedStep, StageEmission,
UnsupportedReason, WorkspaceAllocation, WorkspaceFormula, WorkspaceObject,
WorkspaceUnit, WorkspaceValue
```

The wrapper functions are direct calls with no alternate implementation:

| Function | Direct operation |
| --- | --- |
| `registry()` | Returns the finite normative `OperationRegistry`. |
| `all()` | Iterates every source-qualified descriptor in canonical order. |
| `resolve(symbol)` | Succeeds only for one matching symbol; unknown or duplicate source-qualified symbols return `OperationError`. |
| `resolve_exact(symbol, source)` | Resolves one exact symbol/source pair. |
| `lower_scalar(descriptor)` | Lowers a scalar-owned descriptor to `ScalarProgram`. |
| `lower_primitive(descriptor, request, hardware)` | Validates and lowers a non-elementwise primitive using measured lowering hardware. |
| `validate_composition(descriptor)` | Checks that a structured operation is a finite owned primitive composition. |
| `materialize(request)` | Materializes concrete tensor wiring after shapes and prepared parameters are fixed. |
| `remaining_compositions()` | Reports structured entries still outside the concrete materialization boundary. |
| `evaluate_workspace(descriptor, dimensions)` | Evaluates one checked static workspace formula. |

Every descriptor is classified as an owned scalar, primitive, finite
composition, checked workspace formula, deterministic host/lifecycle
declaration, or `LoweringAvailability::Unsupported`. Unsupported entries fail
closed. The operation layer never substitutes a legacy implementation merely
because a symbol exists in `operation-surface.txt`.

## Declaration state and ordering

`facade.rs:127-280` owns the one ergonomic value and the declaration handoff:

```text
pub struct Recipe;
pub static recipe: Recipe = Recipe;

thread-local RECIPE_SEQUENCE: RefCell<RecipeSequence>
RecipeSequence { data: Option<Data>, model: Option<Model> }
```

The value is zero-sized and has no runtime state. The state is per thread, not
process-global. `Recipe::data` accepts the private `IntoDataSources` bound for
`()`, `&str`, `String`, `&String`, arrays, vectors, and slices of `AsRef<str>`.
It starts with `Data::empty`, appends each source through `Data::set`, stores a
clone in the thread-local sequence, and returns the declaration. `Recipe::model`
constructs `Model::new`, remembers it, and returns it. `train()` and `infer()`
are `const` constructors for empty static policies.

The private sequence transitions are exact:

```text
recipe.data(...)        -> data = Some(new_data), model = None
Data::set/target/...    -> data = Some(updated_data), model = None
recipe.model()          -> model = Some(new_model), data preserved
Model::* builder        -> model = Some(updated_model), data preserved
Train::run              -> take_recipe_training_sequence()
Infer::evaluate         -> Infer::resolve_declaration()
```

Every data builder calls `remember_recipe_data`, which invokes
`begin_recipe_data` and therefore clears any model remembered before the data
mutation. Every model builder calls `remember_recipe_model`. This makes the
required order observable: finish the data chain, then finish the model chain,
then call the terminal. A later `recipe.data(...)` starts a new declaration
sequence rather than mutating an earlier model.

`take_recipe_sequence_with_diagnostics` takes both `Option`s out of the
thread-local cell before checking either one. Missing data and missing model
therefore produce the family-specific static message and consume the pending
sequence. Training maps these messages to `TrainingError::Unsupported`; inference
maps them to `DeclarationErrorKind::InvalidInferenceConfiguration`.

The normal direct Rust API has zero-argument `data` only through the private
conversion implementation `IntoDataSources for ()`; the CLI source frontend
inserts the explicit `()` token for a literal `recipe.data()` call. The private
conversion traits are intentionally not named public types, although the
`impl Trait` methods remain callable by external users.

## Declaration semantics that the root relies on

### `Data`

`Data` is `Clone + Debug + Eq + PartialEq` with private fields for sources,
targets, column exclusions, typed condition exclusions, an exact f32 split
stored as bits, optional normalization, and one deferred declaration error.

* `set` appends one nonempty source path.
* `target` replaces the target vector with a nonempty string or string list.
* `exclude` appends column patterns or validated `Condition` values.
* `split` accepts only a finite f64 that remains strictly in `(0, 1)` after f32
  narrowing; the narrowed bits are retained.
* `norm` records `ZScore`, `MinMax`, or `L2Norm` without touching data.
* `validate` reports the first deferred error or requires at least one source.
  Missing targets and missing split are preparation errors, because target-free
  inference intentionally has neither.

The public readers are `source`, `sources`, `targets`, `exclusions`,
`condition_exclusions`, `split_fraction`, and `normalization`. They expose
declaration facts only; none opens a source or fits a semantic model.

`ConditionValue` preserves floating declaration bits as f64 bits. The data
preparation bridge maps signed, unsigned, text, and boolean values to ingest
predicates and narrows float predicates to finite, non-underflowing f32. A
nonzero f64 that narrows to zero is rejected as
`DataPreparationError::FloatPredicateOutsideF32`.

`prepare_data` and `prepare_data_with_limits` do not apply the declared
normalization on the host. They fit and preserve the lossless vector schema and
partition; dense training maps `DataNormalization` to a Recipe GPU calculation,
while target-free inference takes normalization only from the saved model.

The ingest boundary accepts a regular file, recursively discovered directory,
or recursively nested ZIP source under aggregate byte, record, vector, and
field limits. Declared source order is retained; directory and archive members
are deterministic, and symbolic links, archive-root escapes, malformed
containers, empty sources, and excessive nesting fail closed in the wrapped
`DatasetSourceError` or `IngestError`.

### `Model`

`Model` is a backend-neutral declaration with ordered `LayerSpec` values,
ordered Bayesian dependencies, an optional builtin loss or model reference,
an optional f32 gradient clip, an optional semantic/checkpoint source, and one
deferred declaration error. It contains no loaded bytes, device handles,
allocations, or mutable registry entries.

The layer variants are `Dense`, `Perc`, `Rnn`, `Gru`, `Lstm`, `Convolution`,
`Pool`, `Lgbm`, `Cbst`, `Xgbst`, `Forest`, `KMeans`, `Knn`, `Residual`,
`Embedding`, and `Attention`. Local validation rejects zero dimensions, invalid
residual output widths, invalid grouped routing, and malformed forest/KNN
declarations. Global `Model::validate` additionally enforces:

* a model has at least one layer, Bayesian dependency, or loaded source;
* a loaded checkpoint cannot be combined with inline layers, Bayesian edges,
  loss, or gradient policy;
* KNN is one standalone terminal all-output block with no activation or
  normalization;
* Pool and KMeans grouped-to-dense connections point to the immediately
  following dense layer with the declared neuron count;
* Bayesian children are unique, parents are nonempty and unique, self-edges are
  rejected, and the dependency graph is acyclic;
* referenced objective models validate recursively.

`Model::layer` automatically fills grouped routing for an immediately following
dense layer after `pool` or `kmeans`. Activation and normalization methods only
append to dense, perceptron, recurrent, or residual blocks, except convolution
activation which replaces its pending activation. Calling one on an invalid
receiver records the first deferred `InvalidActivation` or `InvalidLayer`.

`GroupToNeuronConnection::routing` resolves the runtime group count to
`Identity`, contiguous `Expand`, contiguous `Contract`, or ordinary
`FullyConnected` routing. An exact KMeans group count mismatch, zero group, or
zero neuron count returns `None`; non-divisible widths intentionally use full
connectivity rather than a guessed partial mask.

The builder vocabulary is `load`, `layer`, `embed`, `vocab`, `attn`, `perc`,
`rnn`, `gru`, `lstm`, `bayes`, `conv`, `pool`, `lgbm`, `cbst`, `xgbst`,
`forest`, `kmeans`, `knn`, `residual`, the activation methods `relu`, `leak`,
`sigmoid`, `tanh`, `selu`, `gelu`, `silu`, `elu`, `prelu`, `cos`, `exp`, `log`,
`ln`, `huber`, and the policy methods `loss`, `grad`, and `norm`. All return a
new `Model` value and remember that value in the thread-local sequence.

`load` requires one nonempty source and one checkpoint declaration. The
extension is interpreted later: inference accepts `.ogdl` or `.gguf`; dense
training rejects a loaded model as a dense input. The model source is a path,
not an already opened file.

### `Train`

`Train` is a static policy. It stores optional epochs, learning rate bits,
warmup, decay schedule, optimizer, ordered log declarations and cadences,
plot metrics, independent resume and save artifact declarations, and one
deferred error. Builder calls only mutate a returned clone.

* `epochs` and `warmup` require nonzero values; validation later requires
  warmup less than the finite epoch bound.
* `lr` requires a finite positive f32-representable rate and selects linear
  decay by default; `cos` and `exp` replace that schedule.
* `optimizer` records the enum, currently `AdamW`.
* `log` appends typed metrics with cadence one; `every` changes only the most
  recent log declaration and requires a preceding `log` call; `plot` appends
  typed metrics.
* `resume(path)` accepts exactly one semantic `.ogdl` path. The hidden
  `__recipe_resume_pair` is the source-frontend target for a literal model plus
  `.cubin` or `.hsaco` path.
* `save(path)` accepts one `.ogdl`, `.cubin`, or `.hsaco` path. The hidden
  `__recipe_save_pair` is the source-frontend target for two literal paths.
  Save and resume declarations are independent and are not implied by each
  other.

The remaining public readers are `epoch_bound`, `learning_rate`,
`warmup_epoch_bound`, `learning_rate_schedule`, `optimizer_spec`, `log_items`,
`plot_items`, and `resume_source`; destination and native-resume getters stay
crate-private because only the execution owner writes or authenticates those
paths.

`Train::validate` checks deferred errors, warmup ordering, and metric validity,
but dense compilation owns requirements that depend on the selected model:
AdamW must be selected, a decay must be explicit, an objective must be a
builtin dense loss, and a native resume kernel requires a semantic `.ogdl`
resume model. KNN and observed categorical Bayesian preparation deliberately
reject optimizer, learning-rate, epoch, iterative-metric, native-kernel, and
other dense-only policy fields instead of inventing semantics.

At execution, a declared semantic resume path is existence-conditional: an
existing `.ogdl` checkpoint is loaded and a missing one starts a fresh run. A
native resume path is optional and also existence-conditional; an absent
`.cubin` or `.hsaco` is the normal recompile-from-model path, while an existing
file must match the semantic program digest, measured topology and discovery,
target, toolchain, and authenticated bytes. A kernel-only resume is rejected at
declaration. Save destinations are independent of resume and are written only
for the declared extension, with the literal two-path source form writing both
the semantic `.ogdl` and native `.cubin` or `.hsaco` artifacts.

The semantic writer records the model declaration, prepared vector schemas,
parameter and optimizer-moment images, output contracts, and authenticated
native realization metadata in the Recipe checkpoint format. The checkpoint
reader is bounded and path-addressed; output dtype, shape, byte count, target
identity, and native-image multiplicity are checked before a resume or save is
accepted.

### `Infer` and `InferenceDeclaration`

`Infer` stores only a log vector and one deferred error. `log` rejects metrics
that need training targets (`Loss`, `Accuracy`, `R2`, `AuRoc`, `AuPrc`, `Brier`,
`CalibrationError`) or optimizer state (`Epoch`, `Lr`); `Time` and `Device` are
valid. `resolve_declaration` consumes the pending sequence, validates the
policy, data, and model, and constructs an `InferenceDeclaration` containing a
model, optional data, and policy. `evaluate` requires that data be present and
then delegates to the private inference execution function.

The public `InferenceDeclaration` getters expose the model, optional data, and
policy for the root inference compiler. It is not a mutable runtime object.

## Real execution flows

The following flows are the source-level ownership chain behind the root
facade. Preparation, compilation, and execution are separate phases.

### Training terminal

```text
recipe.data(...).target(...).exclude(...).norm(...).split(...)
  -> immutable Data builders and thread-local data slot
recipe.model().layer(...).loss(...).grad(...)
  -> immutable Model builders and thread-local model slot
recipe.train().optimizer(...).epochs(...).lr(...).log(...).save(...)
  -> immutable Train policy
Train::run
  -> take_recipe_training_sequence (consumes Data + Model)
  -> dispatch by model family
```

Dispatch is explicit in `training.rs:869-917`:

1. Bayesian dependencies select `compile_bayes_model`. The bounded prepared
   dataset becomes exact observed categorical reference sets. This branch has
   no optimizer loop or native training kernel. An existing `.resume` model is
   loaded only if it exists, and a declared semantic save is written after the
   model artifact is prepared.
2. A standalone `Knn` layer selects `compile_knn_model`. The complete training
   partition becomes the immutable reference set. KNN has no optimizer, loss,
   epoch loop, iterative metrics, or native training kernel. Existing compatible
   references are continued by appending current references; semantic saves are
   independent.
3. Every other model selects the dense path:

   ```text
   compile_training_graph
     -> validate policy, data, model
     -> require a supported builtin loss and dense policy
     -> prepare_data (bounded ingest, semantic inference, split, exclusions)
     -> map LayerSpec to DenseBlock values
     -> construct DenseTrainingConfig and validation family
     -> recipe_training compile_* graph builder
     -> optional existing .ogdl checkpoint resume
   compile_training_package
     -> load and authenticate optional native resume image
     -> build CheckpointManifest from the compiled graph
   ```

   The graph compiler emits Recipe-owned f32/int32 calculation and transfer
   tasks. Mapping can still return an explicit unsupported error for a missing
   embedding vocabulary, chained recurrent-block operations, an incomplete
   forest booster, or another model combination without a concrete lowering.
   It does not probe hardware, build native images, allocate arenas, or
   execute.

4. Dense execution installs `signal::SigintGuard`, reports unavailable
   validation when needed, and selects either static metric output or a bounded
   nonblocking live metric observer.
5. `execute_current_training_native` calls
   `with_current_native_preparation`:

   ```text
   CLI active-native inputs
     -> exact measured profile and NativeProbeConfig
     -> current host discovery and exact GPU origin reopening
     -> scoped CUDA/HSA bindings + NativeHostPlan + NativeTargetPlan
     -> measured graph tuning (worker threads, staging bytes, watchdog)
     -> HostBackendConfig + StagedCrossBackend
     -> LocalCandidateFactory + NativeExecutorDriver
     -> DeferredArtifactCompiler, optionally with authenticated prebuilt bytes
     -> NativeCandidateRealizer + NativeArtifactProvider + Preparer
     -> prepare_and_execute_local_training_controlled
   ```

   The preparer performs finite candidate planning, native artifact
   compilation, driver realization, maximum-concurrency warmup, bounded
   capacity stabilization, finalization, and executor handoff. The executor
   then runs its typestate lifecycle `PreparedRun -> InitializedRun ->
   RunningRun -> ExitedLoop -> ExitedRun`; init admission and exit egress are
   transfers, and the loop contains the finalized calculation, internal
   transfers, and metric readbacks.
6. Native resources are destroyed before `TrainingReport::dense` returns. The
   report retains the exact checkpoint state, exit images, final metrics,
   journal, realized kernel set, native evidence, training evidence, validation
   status, and graceful-stop flag.
7. A declared model destination is saved through the report's semantic writer.
   A declared `.cubin` or `.hsaco` destination is saved through the report's
   native-kernel writer. No save declaration means no user-owned artifact.

`TrainingReport` keeps family semantics honest: dense reports have a native
run and bundle identity, exit images, final metrics, journal, realized kernels,
native evidence, training evidence, validation status, and a graceful-stop
flag. KNN reports expose only the prepared `KnnModelArtifact`, and Bayesian
reports expose only the prepared `BayesModelArtifact`; those reference-only
branches report no optimizer run, bundle, journal, native kernel, or training
execution evidence.

### Inference terminal

```text
recipe.data(...).exclude(...)
recipe.model().load("model.ogdl" or "model.gguf")
recipe.infer().log([Time, Device])
  -> Infer::resolve_declaration
  -> take_recipe_inference_sequence (consumes Data + Model)
  -> validate target-free policy and loaded model source
  -> Infer::evaluate
```

`compile_inference_package` performs bounded work only:

1. Validate policy, data, and model.
2. Reject targets, split, and data normalization. Inference uses the saved
   model schema, dictionaries, normalization state, and output interpretation.
3. Require `.ogdl` or `.gguf`; load the semantic checkpoint or bounded GGUF
   artifact. A path with another extension is an explicit unsupported error.
4. `distill_data` the ordered source set under the default ingest limits, then
   apply only row and column exclusions with `select_target_free_data`.
5. Prepare and compile the family-specific table: dense checkpoint, standalone
   KNN, observed Bayesian model, or GGUF Llama.

`execute_current_inference_native` repeats the measured native preparation and
fixed-point handoff used by dense training, but calls the family-specific
`prepare_and_execute_local_*_inference` entry point. The completed execution is
joined with the loaded artifact only when the compiled and completed families
match. `write_inference_report` runs after teardown: it prints requested time
and device lines, then exact prediction rows. Any write failure is
`InferenceError::Runtime`.

Inference report payload behavior is intentionally typed:

| Family | `prediction()` | Family-specific accessors | Native image accessor |
| --- | --- | --- | --- |
| Dense | One checked f32 prediction matrix | `values`, multiclass dictionary decoding | `Some(RealizedNativeKernelSet)` |
| KNN | `None` | `knn_predictions`, `decode_knn_class` | `None` in the current report API, although native execution evidence remains available |
| Bayes | One packed probability matrix | output count, child name/class/range, `decode_bayes_*` | `Some(RealizedNativeKernelSet)` |
| GGUF Llama | One raw-logit matrix | `values` | `Some(RealizedNativeKernelSet)` |

`InferenceReport::values` is safe to construct only after the native exit
boundary has checked dtype, shape, and byte count. KNN values may mix f32 and
int32, so it exposes typed predictions instead of a misleading f32 iterator.

### Standalone compile and preparation functions

`compile_training`, `compile_knn_model`, `compile_bayes_model`, and
`compile_inference` stop before native probing, artifact realization, resource
allocation, and execution. `prepare_data*` and `distill_data*` stop at bounded
source preparation. `load_cached_measured_profile`,
`load_native_preparation`, and `build_native_target_plan` stop at measured
profile and build-plan ownership. `with_native_preparation` and
`with_current_native_preparation` create a scoped handoff but do not themselves
execute a graph; the callback decides which fixed-point preparation and native
executor path follows.

## Native profile and identity boundary

`native_prepare::with_current_native_preparation` is the root's bridge from
the CLI probe state to real execution. `cli::current_native_inputs` requires
bare metal, discovers the current host, reads the private active-native
receipt when present, reopens its pinned libraries and tool paths, and derives
the exact identity-named profile path. If no receipt exists it computes the
canonical path from the seed and current host, but it still requires the exact
profile to exist. It never chooses a merely newest profile.

The preparation callback then:

* reopens every current GPU by the origin keys retained in the profile;
* verifies current machine, RAM, storage, and GPU sets against the profile;
* rejects missing, duplicate, unsupported, or ambiguous CUDA/HSA bindings;
* verifies target ABI, architecture, driver capability, pinned toolchain, and
  code-object identities;
* builds one unique target specification for equivalent GPUs while retaining
  one device entry per measured GPU;
* lends bindings with a higher-ranked callback so contexts and sessions cannot
  escape; and
* returns a typed `NativePreparationError` rather than a partial scope.

The current helper caches the exact `(NativeProbeConfig, NativeGpuProbe)` pair
per thread after the first successful opening. Repeated runs reuse that opened
GPU runtime while rebuilding run-scoped host resources and preparation state;
changing the current native configuration on that thread is an
`IdentityMismatch`, not an implicit runtime replacement.

`load_cached_measured_profile` accepts only a path named
`measured-v<schema>-<64 lowercase hex>.recipe-profile`, validates the filename
identity and the content identity, and rejects malformed, insecure, stale,
missing, or invalid profiles. `NativeTargetPlan::deferred_compiler` is the
only next-stage compiler constructor for those validated target specifications.

## CLI and source-runner relationship

The public `recipe::cli` module is included by the facade, not declared in
`src/lib.rs`. `cli::main` returns success or failure after `cli::run` dispatches
the three current commands:

```text
recipe run FILE.rs [ARGS...]
recipe probe [OPTIONS]
recipe convert INPUT OUTPUT
```

* `run` canonicalizes and reads the source, lowers Recipe-only syntax through
  private `source_frontend::lower_recipe_source`, invokes rustc against the
  built `librecipe.rlib`, remaps diagnostics, forwards the compiled program's
  live output, and removes the temporary binary.
* `probe` requires bare metal, discovers host and native devices, benchmarks
  bounded resources, stores an identity-keyed measured profile, and writes the
  active-native receipt consumed by later training or inference.
* `convert` performs bounded `.gguf <-> .ogdl` structural conversion and uses
  create-new output semantics; it is not the inference terminal.

The source frontend is intentionally private. It classifies receivers from
`recipe`, `Data`, `Model`, `Train`, and `Infer` bindings and applies only these
known edits before rustc: `recipe.data()` gets `()`, multi-argument residual
calls become an array, literal two-path `save` and `resume` calls target their
hidden pair methods, literal two-argument `run(model, data)` targets
`__recipe_run_with`, and the named `.grad(clip: EXPR)` form becomes
`.grad(::recipe::clip(EXPR))`. Parse failures return `None` and compile the
original source; overlapping or malformed edits return a CLI error. Rustc
diagnostics never choose a second parse or implementation path.

`signal::SigintGuard` serializes process-level handler installation with a
global mutex, clears an atomic request flag before each operation, and restores
the previous handler on drop. The handler itself performs one atomic store only;
the training loop observes the flag at safe completed-iteration boundaries, and
the CLI forwards one SIGINT to its compiled child.

## Failure ownership matrix

The root keeps failures at the boundary that can explain and repair them:

| Observed failure | Owning type/path | What the owner guarantees |
| --- | --- | --- |
| Empty source, invalid target/exclusion/split/layer/activation/loss/metric/policy | `api::DeclarationError` and `DeclarationErrorKind` | Records the first local declaration error and reports it during terminal validation. |
| Missing declaration order | `facade::take_recipe_*_sequence` then `TrainingError::Unsupported` or `InferenceError::Declaration` | Reports the exact required preceding `data` and `model` declarations; no fallback state is invented. |
| Source I/O, archive framing, aggregate bounds, malformed table, semantic inference, predicate narrowing, train split | `DataPreparationError` | Distinguishes declaration, missing targets/split, float predicate range, ingest, source, semantic, and prepare failures. |
| Dense graph or operation lowering | `TrainingCompileError` inside `TrainingError::Compile` | Preserves ingest, language, operation, program, OGDL, shape, identity, and arithmetic categories. |
| Existing semantic resume decode, checkpoint compatibility, save, or authenticated native metadata | `CheckpointError` inside `TrainingError::Checkpoint` (with a separate `TrainingError::Resume` conversion slot for recipe-training inference-preparation errors) | Validates semantic state, program digest, target/toolchain identity, and native bytes instead of silently continuing incompatible state. |
| Native resume bytes cannot be read or digest does not match | `TrainingError::NativeKernelSource` or `TrainingError::Checkpoint` | Keeps file-source and authenticated semantic-state failures distinct. |
| Current profile/cache/path/binding/toolchain/target mismatch | `NativePreparationError` wrapped as `TrainingError::Native` or `InferenceError::Native` | Fails closed before a partial native scope is handed to the executor. |
| Native driver, queue, completion, allocation, realization, execution, or teardown | `InferenceExecutionError`, `TrainingError::Runtime`, or `InferenceError::Execute` | Keeps backend failure visible and returns reports only after teardown. |
| Unsupported family/policy combination | `TrainingError::Unsupported` or `InferenceError::Unsupported` | Rejects unsupported semantics instead of selecting a legacy or duplicate path. |
| SIGINT install or report/metric stdout write | `TrainingError::Runtime` or `InferenceError::Runtime` | Attributes process and output failures to the root orchestration stage. |
| Operation lookup/lowering/materialization/workspace | `recipe::operations::OperationError` | Reports unknown, ambiguous, unsupported, wrong-kind, invalid composition, missing parameter, identity, shape, and checked arithmetic failures. |
| CLI command, source rewrite, rustc, child process, probe, or conversion | `String` from `cli::run`, rendered by `cli::main` as `ExitCode::FAILURE` | Keeps process orchestration errors outside typed training/inference results. |

There are no root-level runtime or native retry, newest-file, ordinal-device,
proxy, mock, or legacy fallback paths. The source frontend's deliberate
parse-preservation behavior is the one syntax case described above: a source
that cannot be parsed for a rewrite is compiled unchanged, not sent through a
second runtime implementation. An unavailable operation, model family, profile,
or native target remains an explicit failure at its owning boundary.

## Boundaries and non-responsibilities

`src/lib.rs` owns public names and orchestration, not the implementation of
every lower-level concern. In particular:

* `recipe_ingest` owns external file and model-format parsing, bounded source
  snapshots, table framing, semantic vectors, and lossless preparation.
* `recipe_language`, `recipe_ops`, and `recipe_primitives` own graph semantics,
  operation inventory, scalar programs, primitive lowering, and validation.
* `recipe_training` owns dense/KNN/Bayes/GGUF graph compilation, checkpoint
  codecs, prepared inference, and family execution adapters. The root calls it
  through typed functions and exposes only selected report-facing types.
* `recipe_probe` and `recipe_native_probe` own discovery, measurement, profile
  identity, and native binding reopening. Seed values bound probing only and do
  not become production schedule values.
* `recipe_planner`, `recipe_scheduler`, and `recipe_prepare` own finite
  measured candidate search, route/schedule/arena construction, native
  realization, stabilization, and immutable finalization.
* `recipe_kernel`, `recipe_cuda`, `recipe_hsa`, `recipe_native_executor`,
  `recipe_host`, and `recipe_executor` own target lowering, reviewed driver
  boundaries, host transport, native resources, typestate lifecycle, and
  bounded evidence.
* `recipe_remote` and `recipe_transport` provide explicit connected
  master/worker communication; root facade training and inference currently use
  the current local native path.

The root does not expose an API that lets a running loop compile, load, allocate,
change placement, change authoritative state, or admit external data beyond
the finalized lifecycle. Init admission and exit egress are transfers, not
separate model kinds. A `Metric` is a four-byte device readback transfer, not a
third calculation ontology.

## Minimal real-user path and its proof boundary

The shortest complete public path is the same path used by
`examples/train.rs`:

```rust
use recipe::*;

recipe.data("examples/datasets/no-show-appointments/KaggleV2-May-2016.csv")
    .target("No-show")
    .norm(z_score)
    .split(0.8);
recipe.model().layer(128).silu().layer(1).loss(bce);
recipe.train()
    .optimizer(adamw)
    .epochs(100)
    .lr(0.0001)
    .cos()
    .save("model.ogdl")
    .run()?;
```

The declaration calls establish only immutable intent and the thread-local
handoff. `.run()` is the first point that can read the dataset, require the
active measured profile, compile and realize native images, allocate resources,
execute `init -> loop -> exit`, and write an artifact. The resulting
`TrainingReport` is evidence of the complete real path, not merely of a parsed
declaration or a successful compile.
