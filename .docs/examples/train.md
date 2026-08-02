# `examples/train.rs`

## Purpose and current status

`examples/train.rs` is a single dense-training declaration for the checked-in
no-show appointments CSV. It is a direct Cargo example. It does not use the
`recipe run FILE.rs` source-frontend command, so the Rust source is compiled and
executed as written by Cargo.

The source is intentionally small, but its terminal `.run()?` crosses the full
public training boundary. The declaration builders are inert. They record
immutable intent and update the thread-local declaration sequence; they do not
read the CSV, inspect a GPU, compile a graph, allocate memory, or execute a
training loop.

The current real path is blocked during dense graph compilation:

| Boundary | Current result | Evidence |
| --- | --- | --- |
| Rust declaration compilation | pass | `cargo check --example train` |
| CSV distillation, semantic inference, filtering, and split | pass | `cargo run --example train` reaches `TrainingCompileError` after `prepare_data` |
| Dense graph compilation | blocked | `ScheduledDay` is `Temporal/RelativeSecondsI32`, and the dense feature planner has no lowering for it |
| Measured native preparation | not reached | `compile_training_graph` returns before `execute_current_training` |
| Native `init -> loop -> exit` execution | not reached | no `CompiledTraining` is returned |
| `model.ogdl` save | not reached | the save branch runs only after a completed dense execution; no `model.ogdl` was created |

The reproducible command and its current output are:

```text
$ cargo run --example train
Error: compile training graph: InvalidFeatureMatrix: feature "ScheduledDay" classified Temporal/RelativeSecondsI32 does not yet have a dedicated semantic dense lowering
```

This document traces both the observed prefix and the exact downstream path
that would run if the unsupported temporal feature crossed the current compiler
boundary. Downstream stages are marked `unreachable in the current run`; they
are implementation contracts, not claims that this invocation completed native
training.

## Source manifest

The source has one constant and one `main` function:

| Lines | Source declaration | Stored meaning |
| --- | --- | --- |
| 3 | `const DATASET: &str = "examples/datasets/no-show-appointments/KaggleV2-May-2016.csv"` | One path relative to the process working directory |
| 5 | `fn main() -> TrainingResult<()>` | The public training result is propagated with `?` |
| 6-11 | `recipe.data(DATASET)...` | Data source, target, exclusions, row predicate, z-score policy, and train fraction |
| 13-20 | `recipe.model()...` | Three dense layers, two SiLU operations, BCE objective, and global gradient clipping |
| 22-30 | `recipe.train()...run()?` | AdamW, finite 100-epoch policy, warm-up, cosine rate, validation logs, and one semantic save |
| 31 | `Ok(())` | Reached only when every preceding boundary succeeds |

The `use recipe::*` import supplies the static `recipe` value, declaration
types, `cond!`, the `z_score`, `bce`, `adamw`, and logging constants, and the
`clip` constructor.

## Declaration state and sequence bridge

The root facade defines a zero-sized `Recipe` and a static value named
`recipe` (`src/facade.rs:127-133`). A thread-local `RefCell<RecipeSequence>`
stores the immediately preceding `Data` and `Model` values
(`src/facade.rs:135-190`). Every builder method consumes and returns a new
value, then remembers a clone of that value in the sequence. The sequence is
not a second model implementation or a mutable runtime registry.

The source transitions are:

```text
recipe.data(DATASET)
    -> Data::empty -> Data::set -> thread-local data, model cleared
    -> Data::target/exclude/exclude/norm/split -> remembered Data clones
recipe.model()
    -> Model::new -> thread-local model
    -> Model::layer/silu/layer/silu/layer/loss/grad -> remembered Model clones
recipe.train()
    -> Train::new -> independent immutable policy
Train::run()
    -> take_recipe_training_sequence -> consumes and clears both pending values
    -> try_run_with(data, model)
```

`Train::run` consumes the pair before validation. If a terminal call fails, the
caller must redeclare `recipe.data(...)` and `recipe.model()` before retrying.
There is no fallback sequence. In this example the sequence is consumed, data
preparation succeeds, and the compile error is then returned as
`TrainingError::Compile`.

## Data declaration lowering

### Public calls and facade values

The data statement at lines 6-11 is a sequence of `Data` transformations:

| Source call | Facade operation | Resulting state |
| --- | --- | --- |
| `recipe.data(DATASET)` | `Recipe::data` creates `Data::empty`, then `Data::set` appends the nonempty path | `sources = [DATASET]` |
| `.target("No-show")` | `IntoTargets for &str` creates one owned string | `targets = ["No-show"]` |
| `.exclude(["AppointmentID", "PatientId"])` | `IntoExclusions for [&str; 2]` creates two `Exclusion::Column` entries | Exact column patterns for the two identifier columns |
| `.exclude(cond!(Age < 0))` | `cond!` expands to `__condition("Age", Less, 0)`; integer `0` becomes `ConditionValue::Signed(0)` | One typed row predicate in `condition_exclusions` |
| `.norm(z_score)` | `z_score` is `DataNormalization::ZScore` | Numeric feature normalization requested |
| `.split(0.8)` | The f64 is narrowed to f32 after finite and open-interval checks | `split_fraction_bits` stores the exact f32 declaration |

`Data::set`, `target`, `exclude`, `norm`, and `split` defer the first invalid
declaration error instead of panicking. `Data::validate` later requires a
nonempty source. Training preparation additionally requires at least one target
and an explicit split.

The exclusion array does not select the target. Target resolution and column
exclusion are separate preparation operations. `ColumnPattern` matching is
case-insensitive glob matching, although these two patterns contain no wildcard
and match the named CSV headers exactly.

### Production data call graph

For the dense branch, `src/training.rs:592-758` performs the following ordered
calls before native work:

```text
Train::run
  -> take_recipe_training_sequence
  -> try_run_with
  -> compile_training_package
  -> compile_training_graph
  -> policy.validate, data.validate, model.validate
  -> require_supported_model
  -> require_supported_policy
  -> prepare_data
  -> prepare_data_with_limits
  -> distill_data_with_limits
  -> distill_datasets
  -> distill_one_source
  -> CSV parse with HeaderMode::Present
  -> DistilledDataset::infer_vectors(CategoricalEncodingModel)
  -> prepare_inferred_table
  -> select rows and columns, fit semantic metadata on the train prefix
  -> apply the fitted vector schemas to all retained rows
  -> PreparedDataset { train, validation, vectors, source-row mappings }
```

`prepare_data` uses finite defaults from `src/data_prepare.rs:11-18`:

```text
source bytes       <= 1 GiB
framed records     <= 10,000,000
fields per record  <= 16,384
one field          <= 16 MiB
```

The CSV reader retains source order and does not normalize or cast values during
distillation. Semantic inference tries image, temporal, ordinal, exact int32,
and exact f32 parsers before the ambiguous categorical/text classifier
(`ingest/src/semantic.rs:257-364`).

### Dataset facts and partition construction

The checked-in file has 14 headers, 110,527 data rows, and 10,739,535 bytes.
The source headers are:

```text
PatientId, AppointmentID, Gender, ScheduledDay, AppointmentDay, Age,
Neighbourhood, Scholarship, Hipertension, Diabetes, Alcoholism, Handcap,
SMS_received, No-show
```

Preparation resolves the target and exclusions against the original headers,
then evaluates `Age < 0` before removing columns and before splitting. In the
checked-in bytes exactly one row has a negative age, so the independently
measured selection result is:

| Quantity | Count |
| --- | ---: |
| Source data rows | 110,527 |
| Rows excluded by `Age < 0` | 1 |
| Retained rows | 110,526 |
| Retained feature source columns | 11 (14 source columns minus 2 identifiers and 1 target) |
| Target source column | `No-show` |

`Data::split(0.8)` stores the f32 value, not an arbitrary host double. The
current f32 value is the exact rational `13,421,773 / 16,777,216`. The
preparer computes `floor(retained_rows * numerator / denominator)`, preserving
retained row order. Therefore:

| Partition | Rows | `No` rows | `Yes` rows |
| --- | ---: | ---: | ---: |
| Train | 88,420 | 70,286 | 18,134 |
| Validation | 22,106 | 17,921 | 4,185 |

The public root path first calls `DistilledDataset::infer_vectors` on the full
distilled table, so deterministic type classification sees all retained rows.
It then calls `prepare_inferred_table`, whose vector dictionaries, temporal
metadata, and other fit state are built from only the first 88,420 retained
rows before the fitted schema is applied to both partitions. The `No-show`
values are the two observed labels `No` and `Yes`, which satisfy the binary
target contract needed by `bce`. The date columns are recognized as temporal
values with `RelativeSecondsI32` encoding.

## Model declaration lowering

`recipe.model()` calls `Model::new` (`src/api.rs:1020-1031`). The fluent model
state after each source line is:

| Source call | `LayerSpec` or policy field | Stored operation |
| --- | --- | --- |
| `.layer(128)` | `LayerSpec::Dense { units: 128, operations: [] }` | First dense block |
| `.silu()` | Mutates the last dense block | Appends `LayerOperation::Activation(Activation::Silu)` |
| `.layer(128)` | Second `LayerSpec::Dense` | Second dense block |
| `.silu()` | Mutates the second block | Appends a second SiLU operation |
| `.layer(1)` | Third `LayerSpec::Dense` | One-output final block |
| `.loss(bce)` | `Objective::Builtin(Loss::BinaryCrossEntropy)` | BCE from logits |
| `.grad(clip(1.0))` | `clip` stores the positive f32 bit pattern | `gradient_clip_bits = 1.0f32` |

`Model::layer` validates nonzero widths and retains the block in declaration
order. `Model::silu` attaches to the most recent compatible dense block.
`Model::validate` rejects deferred declaration errors, empty models, malformed
blocks, and incompatible checkpoint or objective combinations. This model has
three valid dense blocks and one built-in objective.

When dense compilation is reached, `map_dense_block` maps the three facade
blocks to `recipe_training::DenseBlock::Layer` values. `map_dense_operations`
maps each SiLU to `DenseOperation::Activation(DenseActivation::Silu)`.
`require_supported_model` maps BCE to `DenseLoss::BinaryCrossEntropy` and
rejects no declaration in this model. `effective_blocks` checks that the final
logical width is one, equal to the binary task output width, and emits raw
logits because the final block has no post-output activation.

## Training policy lowering

`recipe.train()` calls `Train::new` (`src/api.rs:1864-1888`). The chain at lines
22-30 stores this policy:

| Source call | Stored field | Value |
| --- | --- | --- |
| `.optimizer(adamw)` | `optimizer` | `Some(Optimizer::AdamW)` |
| `.epochs(100)` | `epochs` | `Some(100)` |
| `.lr(0.0001)` | `learning_rate_bits` and schedule | Positive f32 `0.0001`, initially `LinearDecay` |
| `.warmup(5)` | `warmup_epochs` | `5` |
| `.cos()` | `learning_rate_schedule` | Replaces linear with `CosineDecay` |
| `.log([Loss, AuRoc, AuPrc, Brier, CalibrationError])` | `log` and one `LogDeclaration` | Five valid metrics, default cadence one |
| `.save("model.ogdl")` | `save.model` | One semantic model destination; no kernel destination |
| `.run()?` | terminal operation | Takes the pending data/model and starts validation |

`Train::validate` checks the deferred error, `warmup < epochs` (`5 < 100`),
and every log item. `require_supported_policy` then requires AdamW and an
explicit rate schedule. The policy has no resume declaration, so no semantic
checkpoint or native-kernel resume path is considered.

If the compile boundary were reached, the resulting
`DenseTrainingConfig` would contain:

```text
loss                  = BinaryCrossEntropy
data_normalization    = ZScore
epochs                = Finite(100)
warmup_epochs         = 5
learning_rate_decay   = Cosine
gradient_clip_norm    = Some(1.0)
normalization_epsilon = 1.0e-6
reduction_tree_lanes  = 1024
random_seed           = 0x7265_6369_7065
AdamW rate            = 1.0e-4
AdamW beta1           = 0.9
AdamW beta2           = 0.999
AdamW epsilon         = 1.0e-8
AdamW weight decay    = 0.01
```

The model has one complete training partition per epoch. The finite horizon
would produce `TrainingBounds { train_rows: 88420, epochs: Finite(100),
training_iterations: Finite(100), calibration_iterations: 0, iterations:
Finite(100), warmup_iterations: 5 }`. This is a downstream expected value,
not a value produced by the blocked invocation.

## Observed compile boundary

After `prepare_data` returns, `compile_training_graph` maps the layers and
enters the selected `recipe_training::compile_dense_training...` function. The
compiler first resolves the binary task, validates the declared blocks and
configuration, and constructs `DenseFeaturePlan::from_prepared`.

`DenseFeaturePlan::from_prepared` accepts only these feature contracts for the
current dense lowering:

```text
Numeric / I32 or F32 / no metadata       -> one normalized scalar
Categorical / DictionaryI32 / dictionary -> reserved one-hot span
```

The first retained feature in source order that is outside these cases is
`ScheduledDay`, which has `SemanticType::Temporal` and
`VectorEncoding::RelativeSecondsI32`. The function returns
`TrainingCompileError::InvalidFeatureMatrix` with the exact message shown at
the start of this document. It does not silently reinterpret the timestamp as
a numeric scalar, drop the feature, or choose another compiler.

This is the terminal state of the current real invocation:

```text
PreparedDataset
  -> DenseFeaturePlan::from_prepared
  -> Err(InvalidFeatureMatrix for ScheduledDay)
  -> compile_dense_training... propagates TrainingCompileError
  -> compile_training_graph maps it to TrainingError::Compile
  -> Train::run returns Err
  -> `?` returns the error from `main`
```

No `CompiledTraining`, `CalculationGraph`, `StaticCalculationProgram`, native
profile, prepared bundle, executor run, metric row, or artifact exists for this
run. The later stages below are documented as `unreachable in the current run`.

## Downstream graph construction (unreachable in the current run)

If all retained feature vectors had a supported dense lowering, the selected
compiler entry point would call `compile_dense_training_impl` with the three
dense blocks and `BinaryValidationConfig::new(15, [])`. The graph construction
would be:

1. `DenseFeaturePlan` assigns spans and a numeric normalization mask. Categorical
   spans are one-hot encoded and excluded from z-score fitting; numeric spans
   use the mask.
2. `LoweredDenseDataset::from_prepared` preserves train and validation row order
   and target source indices. `resolve_dense_task` selects
   `DenseTask::BinaryClassification` for the one `No-show` target.
3. `training_bounds` creates a finite 100-iteration loop. There is no
   temperature-scaling request, so no calibration iterations extend the loop.
4. `GraphCompiler` creates external matrix inputs for train features, train
   targets, and validation features/targets. For z-score normalization it
   computes training mean and variance on the train partition and applies those
   same values to validation features.
5. `compile_training_blocks` emits three linear layers. Each weight is a GPU
   Philox normal draw scaled by `sqrt(2 / fan_in)`, each bias starts at zero, and
   each hidden block applies SiLU. First and second AdamW moments start at zero.
6. The final one-column logits feed `gpu_bce_with_logits`. The compiler emits
   masked mean gradients, loss reduction, reverse-mode block gradients, and the
   global norm clip at `1.0`.
7. `dynamic_adam_scalars` derives warm-up progress, the finite cosine decay,
   the current learning rate, and beta powers in the graph. `update_blocks`
   emits AdamW state transitions for every weight and bias and exposes updated
   parameters and moments as exit values.
8. Binary validation runs over all 22,106 validation rows because both labels
   are present. The graph materializes validation BCE, accuracy, AUROC, AUPRC,
   Brier score, and expected calibration error with 15 calibration bins. No
   recall threshold or temperature-scaling state is requested.
9. `training_metric_bindings` binds training loss and learning rate, then the
   binary validation outputs. The graph is validated, canonicalized to OGDL,
   decoded again, and wrapped as a `StaticCalculationProgram` with metric
   emissions.

The compiler's semantic contract is one complete prepared training partition
and one optimizer update per epoch. A backend may tile physical work, but it
cannot turn the 88,420-row logical epoch into user-visible partial batches.

## Downstream measured native preparation (unreachable in the current run)

After graph compilation, `compile_training_package` would build a
`CheckpointManifest` and optionally load a declared resume model. This source
has no `.resume(...)`, so `resume_checkpoint` and `resume_native` would both be
`None`.

`Train::try_run_with` would install the process `SigintGuard`, then call
`execute_current_training`. Since the requested metric set is nonempty,
`execute_current_training` would create a bounded 256-entry metric channel and
spawn the live presenter before native execution.

`execute_current_training_native` performs the following measured-system handoff
(`src/training.rs:1278-1337`):

```text
next_run_id
  -> with_current_native_preparation
  -> cli::current_native_inputs
  -> load the identity-named active measured profile
  -> rediscover current host/GPU inventory and validate identity
  -> lend scoped CUDA/HSA bindings, host plan, and target build specs
  -> derive_native_runtime_tuning from graph, profile, and measured links
  -> construct HostBackendConfig with run-scoped resources
  -> StagedCrossBackend::new
  -> LocalCandidateFactory::production
  -> NativeExecutorDriver::new
  -> TargetPlan::deferred_compiler
  -> NativeCandidateRealizer::new
  -> Preparer::new(NativeArtifactProvider, realizer)
  -> prepare_and_execute_local_training_controlled
```

`with_current_native_preparation` does not choose a merely newest profile. It
loads the exact profile identity named by the active receipt, verifies the
current host memory origin and complete GPU scope, reopens pinned backend and
toolchain inputs, and keeps runtime handles inside a higher-ranked callback.
Changed topology, discovery, receipt, host origin, tool digest, or target
identity is a `TrainingError::Native` failure.

`Preparer::prepare_program` is the fixed-point boundary in `prepare/src/lib.rs`:

1. Validate preparation policy and the measured profile.
2. Obtain and validate the mandatory reservation plan.
3. Resolve the native artifact catalog and enumerate finite planned candidates.
4. For each candidate, lower and build native stages in `BuildPhase::Realize`,
   load them, reserve resources, warm the maximum-concurrency schedule, and
   collect capacity snapshots.
5. Reject candidates whose realization, stabilization, or final arena packing
   does not match the measured contracts. Candidate resources are destroyed
   before the next candidate.
6. Pack arenas against post-warm capacity and call
   `FinalizedBundle::finalize_with_loop_schedule` with the finite 100-iteration
   loop domains.

`Preparer::new` uses three stabilization passes with a stable tail of two. A
successful return owns one immutable finalized bundle and the live realized
native session. Compilation, loading, allocation, and warm-up all finish before
the executor receives the session.

## Downstream executor lifecycle (unreachable in the current run)

`training::execute::prepare_and_execute_local_training_controlled` rejects any
external transfer in the loop, packs the compiled external inputs into exactly
one finalized init image per device, maps finalized exit tasks to logical
output values, and creates one empty final metric record for each user metric
slot. It then consumes the prepared native session and enters the executor's
owned typestate:

| State | Transition | Authoritative work |
| --- | --- | --- |
| `PreparedRun` | `PreparedRun::prepare` | Compute bounded journal capacity, bind resources, realize init/loop/exit phases, record `Prepared` |
| `InitializedRun` | `initialize(images)` | Validate images, allocate every finalized arena, admit one image per device through init, record `Initialized` |
| `RunningRun` | `start_loop()` | Start loop iteration zero and record `LoopStarted` and `LoopIterationStarted` |
| `RunningRun` | `poll_with_progress_or_stop` | Submit and poll fixed tasks, publish metric slots, complete each epoch, and begin the next finite iteration |
| `ExitedLoop` | `into_exited_loop()` | Legal only after the final iteration has completed; loop ingress is no longer possible |
| `ExitedRun` | `exit()` | Run exit egress transfers, collect external outputs, quiesce and destroy native resources, record `Exited` |
| `CompletedTrainingExecution` | return to `src/training.rs` | Retain run, bundle, outputs, newest metric samples, native evidence, and bounded journal after teardown |

The loop has no file or data ingress/egress API. `init` admits the complete
logical training inputs once per device. Loop calculations and internal
transfers repeat for 100 iterations. `exit` publishes updated model tensors,
normalization state, and requested metric readbacks. The executor's typestate
makes an invalid phase transition unrepresentable.

Production `RunJournal` retains ordered nonrepeating lifecycle events and the
first loop iteration while compacting repeated loop detail into counters. It is
not allocated in proportion to 100 epochs. The resulting
`TrainingExecutionEvidence` would independently report 100 loop iterations,
one logical optimizer update per epoch, and the actual finalized task and
submission counts.

## Metric presentation (unreachable in the current run)

`binary_validation_config` recognizes the requested AUROC, AUPRC, Brier, and
calibration metrics and creates a binary validation request with 15 ECE bins.
The compiler still emits all binary metric outputs, but
`live_metric_presentations` selects only what the public policy requested:

| Requested log item | Selected binding(s) | Terminal field |
| --- | --- | --- |
| `Loss` | Training loss and validation mean BCE | `loss`, `validation_loss` |
| `AuRoc` | Validation AUROC | `auroc` |
| `AuPrc` | Validation AUPRC | `auprc` |
| `Brier` | Validation Brier score | `brier` |
| `CalibrationError` | Expected calibration error | `calibration_error` |

`Accuracy`, `LearningRate`, `Epoch`, `Time`, and `Device` are not selected by
this policy. Each selected binding has default cadence one because `.every(...)`
was not called. The executor offers samples through a nonblocking observer;
channel saturation could drop a live notification but cannot change graph
progress or the final newest-sample records. The presenter groups one-based
epoch samples and writes one live row per completed epoch when this stage is
reachable.

## Semantic artifact write (unreachable in the current run)

The one-path `.save("model.ogdl")` declaration sets only the model destination.
It does not request a `.cubin` or `.hsaco` companion. For a successful dense
run, the order is:

```text
CompletedTrainingExecution
  -> TrainingReport::dense
  -> CompletedTrainingCheckpoint::new
  -> CheckpointManifest::from_compiled
  -> map finalized exit images to logical parameter and optimizer values
  -> attach authenticated native realization metadata
  -> report.save_model("model.ogdl")
  -> atomic semantic checkpoint write
```

The semantic checkpoint retains the feature schema and spans, target identity,
dense block declarations and operations, z-score state, updated parameters,
AdamW moments, objective/configuration, bounds, and program digest. Native bytes
are not embedded in the OGDL file. Because this source supplies no kernel save
path, a successful invocation would produce at most the one user-owned file
`model.ogdl`; journals, plans, caches, profiles, and runtime images are not
exported artifacts.

The current run never constructs `CompletedTrainingCheckpoint`, so it writes no
file. The compile error is returned before the native profile, artifact writer,
or atomic save boundary.

## Failure boundary and retry semantics

The public result is `TrainingResult<()>` from `main`, and `?` preserves the
first typed failure. Relevant error layers for this source are:

| Layer | Error variant | Examples for this path |
| --- | --- | --- |
| Declaration | `TrainingError::Declaration` | Empty source/target, invalid exclusion, invalid split, malformed layer, invalid clip, invalid metric, or invalid save path deferred by builders |
| Data | `TrainingError::Data` | Missing CSV, malformed CSV, source-limit violation, missing target, missing split, predicate parse/type failure, semantic inference failure, or empty partition |
| Graph compile | `TrainingError::Compile` | Unsupported semantic feature lowering, incompatible target/loss, wrong final width, invalid normalization, graph shape/primitive failure |
| Resume | `TrainingError::Resume` | Not used here because no `.resume(...)` is declared; would cover an existing incompatible semantic checkpoint |
| Native preparation | `TrainingError::Native` | Not reached here; covers stale or missing measured profile, current discovery mismatch, target/toolchain mismatch, reservation, realization, or finalization failure |
| Runtime | `TrainingError::Runtime` | Not reached here; covers signal handler, native executor, metric presenter, device fault, or publication failures |
| Checkpoint | `TrainingError::Checkpoint` | Not reached here; covers semantic `.ogdl` write failure after teardown |

The observed error is specifically `TrainingError::Compile` wrapping
`InvalidFeatureMatrix` for `ScheduledDay`. There is no retry, alternate
semantic interpretation, or fallback compiler. A caller that wants to retry
after a source or declaration change must construct a fresh data/model sequence.

## Validation evidence

The following commands were run from the repository root:

```text
cargo check --example train
    Finished `dev` profile

cargo run --example train
    Error: compile training graph: InvalidFeatureMatrix: feature "ScheduledDay" classified Temporal/RelativeSecondsI32 does not yet have a dedicated semantic dense lowering
```

Independent source measurements used above were obtained from the checked-in
CSV bytes: 110,527 data rows, one negative-age row, 110,526 retained rows,
88,420 train rows, and 22,106 validation rows. No native hardware acceptance
run is possible through this source until the observed temporal dense lowering
boundary is crossed; documenting that blocker is part of the current behavior,
not a passing or skipped acceptance result.
