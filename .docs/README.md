# Root package: `recipe`

This document describes the root workspace package in `/home/nate/Desktop/nates-recipe-rs`.
It follows the public Rust entrypoints in `src/` into their actual callers and
callees. It is an implementation map, not a replacement for the normative
contracts.

The authority order used by the root code is:

| Source | What it defines |
| --- | --- |
| [`API.ogdl`](../API.ogdl) | The complete public declaration grammar, including declarations that are not yet executable in every path. |
| [`system-contract.md`](../system-contract.md) | Lifecycle, dtype, scheduling, artifact, device, and failure rules. |
| [`topology/contract.toml`](../topology/contract.toml) | Bounded probe seed estimates and probe policy. These values are not production measurements. |
| [`operation-surface.txt`](../operation-surface.txt) | The finite source-qualified compatibility operation inventory. |
| `src/` and workspace crates | The current implementation of the declared boundaries. Unsupported implementation cases fail closed. |

## Package shape and public entrypoints

The root [`Cargo.toml`](../Cargo.toml) names the package and library `recipe`,
the binary `recipe`, and the `cookbook` example. The root library is assembled
by [`src/lib.rs`](../src/lib.rs) and [`src/facade.rs`](../src/facade.rs):

| Path | Role | Public result |
| --- | --- | --- |
| [`src/main.rs`](../src/main.rs) | Binary shim | Calls `recipe::cli::main()` and returns its `ExitCode`. |
| [`src/facade.rs`](../src/facade.rs) | Facade, thread-local declaration bridge, engine reexports, operation facade | Exports `recipe`, `Data`, `Model`, `Train`, `Infer`, all public declaration types, and the lower-level `engine` and `operations` modules. |
| [`src/api.rs`](../src/api.rs) | Immutable declaration types and validation | Defines `Data`, `Model`, `Train`, `Infer`, layer/objective/metric types, and deferred `DeclarationError` values. |
| [`src/data_prepare.rs`](../src/data_prepare.rs) | Public data boundary | Converts a validated `Data` declaration to a bounded `PreparedDataset`, or to a target-free `RawTable` for inference. |
| [`src/training.rs`](../src/training.rs) | Public training boundary | Compiles dense, KNN, or observed categorical Bayesian declarations, owns native execution handoff, metrics, SIGINT control, and artifact writes. |
| [`src/inference.rs`](../src/inference.rs) | Public inference boundary | Loads a semantic `.ogdl` or supported `.gguf`, prepares target-free rows, executes one native loop, prints predictions, and returns `InferenceReport`. |
| [`src/native_prepare.rs`](../src/native_prepare.rs) | Measured native-system boundary | Reopens the exact measured profile and exact CUDA/HSA bindings, creates target plans, and lends scoped native handles to training or inference. |
| [`src/cli.rs`](../src/cli.rs) | Installed command implementation | Implements `run`, `probe`, and `convert`, private state and receipt validation, source compilation, live output, and child SIGINT forwarding. |
| [`src/source_frontend.rs`](../src/source_frontend.rs) | Rust source frontend | Performs syntax-derived Recipe-only rewrites before `rustc` and maps compiler diagnostics back to the user source. |
| [`src/signal.rs`](../src/signal.rs) | Process SIGINT guard | Records one atomic stop request and restores the previous process handler when the guard drops. |
| [`src/validation_reporting.rs`](../src/validation_reporting.rs) | Training validation status text | Renders an unavailable-validation line for the public live output. |
| [`src/bin/generate-train.rs`](../src/bin/generate-train.rs) | API-chain generator binary | Reads `API.ogdl`, chooses a checked-in dataset and supported token subset deterministically, and emits a Rust training source. |

`src/lib.rs` reexports the root-level preparation, inference, native evidence,
training, checkpoint, KNN, Bayesian, and validation result types. It also
includes `facade.rs`, so the fluent declarations are the package's primary
user-facing API. The `engine` module is a namespaced reexport of the workspace
crates for advanced callers. It does not add another implementation path.

## Declaration model and sequence bridge

`recipe` is a zero-sized static [`Recipe`](../src/facade.rs) value. Its methods
only construct immutable values:

```text
recipe.data(sources) -> Data
recipe.model()       -> Model
recipe.train()       -> Train
recipe.infer()       -> Infer
```

The builders do not read files, discover hardware, compile kernels, allocate
native resources, or start execution. Every mutating-looking method consumes
and returns a new declaration value. Invalid arguments store the first
`DeclarationError` in that value, allowing a complete chain to be built before
the terminal call reports the error.

The facade keeps the immediately preceding data and model declarations in a
thread-local `RefCell<RecipeSequence>`:

1. `Recipe::data` creates an empty `Data`, applies each source through
   `Data::set`, then stores a clone and clears any preceding model.
2. `Data` methods (`set`, `target`, `exclude`, `split`, `norm`) store their
   updated clone, so a fluent continuation remains the sequence's current
   data declaration.
3. `Recipe::model` stores a new empty `Model`; model methods store each updated
   clone.
4. `Train::run` consumes the pair with
   `take_recipe_training_sequence`. `Infer::evaluate` consumes it with
   `take_recipe_inference_sequence`. Missing data or model is an explicit
   unsupported terminal error, and a failed terminal consumes the pair.

The source frontend also has a direct two-argument training target,
`Train::__recipe_run_with(&Model, &Data)`, so `recipe.train().run(model, data)`
can be lowered without relying on the thread-local sequence. That method is
hidden from ordinary API documentation and calls the same `try_run_with` path.

### `Data`

`Data` stores an ordered list of source paths, target names, column-pattern
exclusions, typed row predicates, an optional f32 split fraction, an optional
normalization (`z_score`, `min_max`, or `l2_norm`), and one deferred declaration
error. `cond!(column operator value)` constructs a typed `Condition` without
evaluating a Rust identifier. Numeric condition values preserve their source
bits; a nonfinite condition or an f64 value that cannot be represented as a
finite, non-underflowing f32 is rejected at the preparation boundary.

`Data::validate` requires at least one source and propagates the deferred error.
Training additionally requires at least one target and an explicit split.
Target-free inference rejects targets, splits, and data normalization because
the saved semantic model owns the feature schema and normalization.

### `Model`

`Model` contains ordered `LayerSpec` blocks, ordered Bayesian dependencies, an
optional built-in or referenced objective, an optional f32 gradient clipping
norm, an optional checkpoint path, and one deferred error. It contains no file
contents, weights, handles, allocations, or mutable global registry state.

The declaration surface includes dense and perceptron blocks, convolution,
pooling, K-means, KNN, trees/forests, residual branches, embedding and
attention, RNN/GRU/LSTM, activations, normalization, built-in losses, Bayesian
dependencies, and checkpoint loading. Layer validation rejects zero dimensions,
invalid grouped-to-dense routing, an incomplete forest booster, malformed
residual output width, activation or normalization on terminal all-output KNN,
and a KNN block composed with any other block. `Model::load` is mutually
exclusive with inline blocks, dependencies, a loss, or gradient policy.

Bayesian dependencies are checked for nonempty unique child/parent names,
self-parent edges, duplicate parents, duplicate children, and cycles. A grouped
pool or K-means block records its required immediately following dense width and
the resolved route (identity, contiguous expansion/contraction, or full
connectivity) remains part of the declaration.

### `Train`

`Train` stores optional epochs, learning rate, warmup, decay schedule, optimizer,
log and plot declarations, optional model/kernel resume paths, optional
model/kernel save paths, and a deferred error. The public contract accepts one
literal one-path or two-path declaration for each of `resume` and `save`:

| Declaration | Accepted paths | Meaning |
| --- | --- | --- |
| `.resume("model.ogdl")` | Semantic model only | Load it if it exists, otherwise start fresh. |
| `.resume("model.ogdl", "kernel.cubin" or "kernel.hsaco")` | Semantic model first, native kernel second | Authenticate and reuse the kernel only when the saved model, current program, measured target, toolchain, and bytes all match. |
| `.save("model.ogdl")` | One semantic path | Export the final semantic model. |
| `.save("kernel.cubin" or "kernel.hsaco")` | One native path | Export the realized dense native image. |
| `.save("model.ogdl", "kernel.cubin" or "kernel.hsaco")` | Both paths | Export exactly the two requested artifacts. |

The first duplicate save or resume declaration is an error. An empty path or
wrong extension is an error. Omitting `.resume` does not disable `.save`.
Omitting `.save` exports nothing. KNN and Bayesian preparation can save their
semantic model, but cannot produce a native training kernel.

`Train::validate` enforces nonzero epochs and warmup, `warmup < epochs` when
both are present, nonzero log cadence following a log declaration, a finite
positive f32 learning rate, and valid artifact declarations. Dense execution
requires the Recipe-owned AdamW optimizer, an explicit learning-rate schedule,
an explicit normalization unless an embedding is the leading block, and one
compatible validation metric family at most. Cosine and exponential schedules
require a finite epoch endpoint; an omitted epoch bound is an unbounded loop
that uses a constant rate after optional warmup and requires graceful stop.

### `Infer`

`Infer` stores only log items and a deferred error. It rejects target-dependent
metrics (`Loss`, `Accuracy`, `R2`, AUROC, AUPRC, Brier, calibration) and
training-state metrics (`Epoch`, `Lr`) because target-free inference has no
targets or optimizer state. `Time` and `Device` are post-exit host reporting
and do not add loop transfers. `Infer::evaluate` resolves the sequence, checks
both declarations, and delegates to `evaluate_inference_declaration`.

## Rust source frontend and command paths

`recipe run FILE.rs` is a source runner, not a Cargo project builder. The path
is canonicalized and must be a regular file. The runner locates a built
`librecipe.rlib`, creates a private run binary path beneath the private state
root, and invokes `rustc` with edition 2024, JSON diagnostics, `-Dunused_must_use`,
the library search paths, and `--extern recipe=...`.

Before `rustc`, `lower_recipe_source` parses the source with `syn`, classifies
the receiver of each method call (`recipe`, `Data`, `Model`, `Train`, or
`Infer`), and applies only syntax-proven edits:

| Source form | Lowered form | Reason |
| --- | --- | --- |
| `recipe.data()` | `recipe.data(())` | The Rust method receives the empty `IntoDataSources` tuple. |
| `.residual(a, b, ...)` | `.residual([a, b, ...])` | The public Rust method receives one residual-branch value. |
| `.grad(clip: expr)` | `.grad(::recipe::clip(expr))` | `API.ogdl` names the declarative field form while Rust receives `Grad`. |
| `.save(model, kernel)` | `.__recipe_save_pair(model, kernel)` | Preserves the literal two-argument API without adding a tuple form. |
| `.resume(model, kernel)` | `.__recipe_resume_pair(model, kernel)` | Same literal two-path boundary for resume. |
| `.run(model, data)` | `.__recipe_run_with(model, data)` | Uses the explicit declaration pair without the thread-local bridge. |

Named-gradient candidates are first replaced with a temporary valid expression
for receiver classification. A malformed field list is reported at the
original source span. Edits are represented by `SourceRewrite` ranges, and
`DiagnosticStream` remaps JSON and raw compiler diagnostics back to the original
path and line/column. There is no compiler-diagnostic-driven retry or source
rewrite loop.

The child binary's stdout and stderr are forwarded live by dedicated threads;
each stream retains only a bounded tail for a failure message. A process SIGINT
is forwarded once to the child. The compiled binary is removed after the run,
and failure reports distinguish allocation failure, signal termination, exit
status, and truncated output tails.

The command dispatcher in [`src/cli.rs`](../src/cli.rs) exposes:

| Command | Inputs | Output and failure boundary |
| --- | --- | --- |
| `recipe run FILE.rs [ARGS...]` | A regular Rust source file and child arguments | Compiles and executes the source through the real root library. Compilation, child status, output forwarding, and cleanup errors are surfaced. |
| `recipe probe [OPTIONS]` | Optional exact paths for the seed, profile, CUDA/HSA libraries, LLVM `opt`/`llc`, `lld`, and `ptxas` | Requires bare metal, discovers and measures the host, writes an identity-named measured profile and active-native receipt. |
| `recipe convert IN OUT` | `.gguf -> .ogdl` or `.ogdl -> .gguf` | Performs bounded structural GGUF v3 conversion with create-new atomic output. It does not imply executable GGUF inference. |

The private state root is `$XDG_CACHE_HOME/recipe-next` when the variable is an
absolute path, otherwise `$HOME/.cache/recipe-next`. Directories are canonical,
real, effective-user-owned, and not group/other accessible. The active receipt
is a fixed-order, mode-0600 file containing the measured-profile path and
identity, host memory origin, PCI root, scratch path, selected native-library
pins, tool pins and digests, PTX/HSA settings, release label, and FMA-chain
length. It is written atomically and decoded canonically. Any identity,
permission, ownership, symlink, or digest change requires a fresh probe.

## Data preparation trace

The public training path calls `prepare_data`, which creates bounded
`IngestLimits`, validates `Data`, requires targets and a split, maps exclusions
to `recipe_ingest::ColumnPattern` and `RowPredicate`, and creates a
`PreparationRequest`. It then calls `distill_data_with_limits`:

```text
Data.sources
  -> recipe_ingest::distill_datasets
  -> DistilledDataset { RawTable, source semantics, file count }
  -> infer_vectors(&CategoricalEncodingModel)
  -> prepare_inferred_table
  -> PreparedDataset { schemas, train/validation partitions, dense values }
```

Ingestion is a bounded filesystem boundary. It can recursively distill regular
files, directories, and ZIP containers into one rectangular ordered table while
retaining source context. Semantic inference chooses a lossless representation
per vector: f32 for numeric values, int32 for exact integers, relative seconds,
dictionaries, or ordinals, and variable-width values for text or bytes until a
typed operation supplies a lowering. The preparation layer applies row and
column exclusion, fits semantic dictionaries and numeric normalization on the
training partition, and retains source order. It does not perform production
payload arithmetic on the CPU.

`prepare_data` fails before producing a partial dataset for invalid declarations,
missing targets or split, source framing or limit errors, inconsistent rows,
semantic inference errors, an unrepresentable f32 predicate, or preparation
errors. `distill_data` and `select_target_free_data` intentionally stop before
training target fitting, splitting, and normalization; inference uses those two
functions and then applies the saved model schema.

## Measured native preparation

`recipe probe` reads `topology/contract.toml` as a seed, discovers host RAM and
storage, creates `NativeProbeConfig`, constructs `NativeGpuProbe`, and gives all
of them to `ProbeEngine`. The engine discovers every expected native GPU and
benchmarks capacity, calculation rate, transfer rate, and directed links. It
writes a versioned, hashed `MeasuredProfile` under an identity-derived filename
and records an active-native receipt. Seed estimates bound benchmark work only.

`with_current_native_preparation` reopens the receipt or reconstructs the same
identity-derived path, loads the exact cached profile, and caches the native
probe configuration per thread. `with_native_preparation` then:

1. discovers the current exhaustive GPU inventory;
2. calls `MeasuredProfile::resolve_local_inventory`, matching only stable machine,
   RAM, storage, and GPU origin identities and exact target identities;
3. rejects missing, extra, or changed devices, nonlocal GPUs, unsupported
   backends, duplicate bindings, and changed toolchain or library digests;
4. creates exact `NativeDeviceBuildTarget` entries and one deduplicated
   `TargetBuildSpec` for each target identity; and
5. lends CUDA and HSA bindings, a `NativeHostPlan`, and the target plan to one
   higher-ranked callback. Native handles cannot escape that callback.

The native target plan maps NVIDIA deployment identities to CUDA Driver policy
and pinned PTX/LLVM tools, or AMD identities to ROCr/HSA target and code-object
policy. It constructs a `DeferredArtifactCompiler`; the compiler is retained
only inside preparation and never enters the running lifecycle.

## Operation and graph ownership

The root `engine` reexports the lower-level ownership layers, while
`operations` is the single root facade for the finite registry:

```text
operation-surface.txt + Recipe-owned extensions
  -> recipe_ops::OperationRegistry
  -> ScalarProgram, PrimitiveRequest, or finite composition
  -> recipe_primitives::LoweredProgram
  -> recipe_language::CalculationGraph / recipe_program::StaticCalculationProgram
```

Each source-qualified descriptor declares its family, canonical f32/int32
dtype contract, alias contract, determinism contract, definition, and one
`LoweringAvailability`: owned scalar, primitive, composition, workspace,
non-calculation, or explicit `Unsupported`. `operations::resolve` rejects an
ambiguous symbol; `resolve_exact` requires symbol and source. `materialize`
requires concrete shapes and prepared parameters before emitting tensor wiring.
No unsupported descriptor falls back to a legacy vendor or CPU implementation.

`recipe-language::CalculationGraph` owns tensors and primitive kernels only. It
checks tensor storage contracts, unique producers, external boundaries, kernel
validity, and acyclic topological order. `recipe-core::ScalarProgram` is the
typed intra-kernel SSA-like f32/int32 program; `KernelTemplate` applies it over
an index space and carries complete alias rules. `recipe-primitives` lowers
validated primitives into fixed stages, resource bounds, accesses,
synchronization, atomics, fault contracts, and a stable program digest.

`recipe-program::StaticCalculationProgram` adds one finite or unbounded loop
horizon, one iteration domain per kernel, and four-byte metric emissions. It
validates domain coverage and metric producer coverage, serializes the graph and
domains to strict versioned OGDL, and rejects unknown, missing, duplicate, or
noncanonical fields when decoding.

## Training source trace

`Train::run` takes the data/model pair, dispatches model families, and returns a
`TrainingReport`:

```text
Train::run
  -> take_recipe_training_sequence (or __recipe_run_with)
  -> family dispatch
     -> compile_bayes_model        -> semantic Bayes artifact, no native loop
     -> compile_knn_model           -> immutable KNN reference artifact, no native loop
     -> compile_training_package    -> CompiledTraining + CheckpointManifest
        -> prepare_data
        -> map LayerSpec -> DenseBlock / DenseLayer
        -> recipe_training compiler
        -> optional checkpoint resume
        -> optional authenticated native-kernel resume bundle
     -> install SigintGuard
     -> execute_current_training
        -> with_current_native_preparation
        -> derive_native_runtime_tuning from measured graph/profile
        -> LocalCandidateFactory + NativeExecutorDriver
        -> Preparer::prepare_program
        -> prepare_and_execute_local_training_controlled
        -> TrainingReport
```

### Dense compilation

`compile_training_graph` validates all declarations, maps the objective to one
of the owned dense losses, maps layer blocks and operations, chooses a finite or
unbounded `TrainingHorizon`, converts learning-rate policy to an owned decay,
selects explicit data normalization, and selects at most one binary,
multiclass, or regression validation family. The graph compiler receives the
complete prepared partition as one logical matrix and emits one optimizer
update per epoch; backend tiling does not change that semantic unit.

The root mapper has concrete cases for embedding, attention, RNN, GRU, LSTM,
convolution, pooling, K-means, LightGBM/CatBoost/XGBoost trees and forests,
residuals, dense layers, and perceptrons. It fails closed where a declaration
has no current semantic lowering, such as chained operations on the first
recurrent blocks or a forest without a nested booster. Dense training rejects
loaded weights, Bayesian dependencies, reference objectives, missing explicit
loss, non-AdamW optimizer, missing schedule, incompatible metric/loss pairs,
multiple validation families, and invalid width conversions.

### Resume and native kernel authentication

If `.resume(model.ogdl)` names an existing file, the root loads and applies the
checkpoint to the newly compiled graph. A missing model is the normal fresh-run
case. If a native kernel path is supplied and the semantic checkpoint contains
native realization metadata, the root verifies program digest, topology and
discovery identities, target and toolchain identity, file size limits, and exact
kernel digest before passing the bytes to `DeferredArtifactCompiler` as a
prebuilt bundle. A missing kernel is not an error and causes ordinary realization
from the current measured system. An unauthenticated, ambiguous, or mismatched
kernel is an incompatible-resume error.

### Native training execution

`execute_current_training_native` enters the scoped native callback, derives
worker count, host staging bytes, and watchdog from measured host/device/link
properties, builds deterministic per-run host resources, creates a staged
cross-backend bridge, and constructs `Preparer<NativeArtifactProvider,
NativeCandidateRealizer>`. `recipe-prepare::Preparer` validates the measured
profile, plans reservations, resolves artifacts, enumerates ranked finite
candidates, realizes and warms one candidate, checks the required stable
capacity tail, packs arenas, hashes realization, and finalizes an immutable
`FinalizedBundle`. A rejected candidate is destroyed before the next candidate;
finalization cannot mutate the drafted choices.

The training execution entrypoint validates the optional stop source, packs one
`DeviceImage` per finalized device, maps exit tasks to logical output values,
creates a bounded metric mailbox, hands the warmed native session to the
backend-neutral executor, and traverses the typestate lifecycle:

```text
PreparedRun::prepare
  -> InitializedRun::initialize (allocate fixed arenas, one init image/device)
  -> RunningRun::start_loop / poll_with_progress_or_stop
  -> ExitedLoop::exit (run exit transfers)
  -> ExitedRun (ordered resource teardown complete)
```

The running handle exposes only nonblocking polling, metric consumption, stop
checking, and journal inspection. It has no compiler, loader, allocator,
topology mutation, external ingress, or file egress operation. Stop is accepted
only after a complete iteration. User metric notifications use preallocated
four-byte device readbacks and a bounded newest-value-wins channel, so a slow
presenter cannot backpressure calculation.

`TrainingReport::dense` is built only after native teardown. It contains the
completed `RunJournal`, exact native-kernel set, native-driver evidence, full
partition execution evidence, final metrics, validation status, and a graceful
stop flag. The root then performs only the independently declared model and
kernel saves. KNN and Bayesian reports contain semantic artifacts and no native
execution evidence.

## Inference source trace

`Infer::evaluate` is target-free and requires `recipe.model().load(PATH)` where
the extension is `.ogdl` or `.gguf`. The root rejects data targets, splits, and
normalization, then executes:

```text
compile_inference_package
  -> load semantic model or bounded GGUF Llama artifact
  -> distill_data
  -> select_target_free_data
  -> saved-schema preparation
  -> family compiler
     -> Dense / KNN / Bayes / GGUF Llama static program
  -> with_current_native_preparation
  -> Preparer::prepare_program
  -> one `init -> loop -> exit` iteration
  -> collect validated exit prediction bytes
  -> print rows after teardown
```

Semantic `.ogdl` roots dispatch to dense checkpoints, KNN artifacts, or
observed categorical Bayesian artifacts. Checkpoint preparation uses saved
feature names, source indexes, encodings, dictionaries, normalization and
effective block topology. It ignores unrelated source columns, allows source
column reordering, and does not refit the model. KNN preserves mixed f32 and
int32 output contracts. Bayesian conditionals preserve declaration order and
pack adjacent class ranges. `.gguf` dispatch is structural and bounded before
the dedicated dense-F32 Llama compiler checks architecture, token IDs, context,
tensor shapes, and graph contracts.

`recipe-training`'s inference executors reject loop external transfers and user
metrics, require exactly one loop iteration, admit all declared inputs during
`init`, and collect prediction bytes only from finalized exit transfers. The
root validates prediction dtype, rank, shape, byte count, saved target order,
class dictionaries, and model-family-specific output contracts before writing
rows. `InferenceReport::values` exposes validated little-endian f32 values for
dense, Bayesian, and GGUF predictions; KNN callers use typed prediction
accessors because outputs may be f32 means or int32 class codes.

Inference errors are partitioned into declaration, data, model decoding/schema,
graph compilation, native preparation, native execution, runtime output, and
explicit unsupported categories. A report is returned only after ordered
teardown and contains run/bundle identities, journal, elapsed loop duration,
native-kernel/evidence accessors, device labels, and model-family output
decoders.

## Fixed-point preparation and lifecycle ownership

The key lower-level flow is the one implemented by `recipe-prepare` and
`recipe-executor`:

```text
MeasuredProfile
  -> validate topology/discovery/reservation
  -> optimistic measured-capacity bound
  -> finite planner candidate stream
  -> native artifact resolution or deferred Realize compilation
  -> candidate load, warm, resource creation, capacity snapshots
  -> stable-tail validation
  -> exact arena packing and value-location resolution
  -> FinalizedBundle
  -> PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun
```

`recipe-planner::plan_program_candidates` lowers graph stages, enumerates legal
device/artifact assignments, schedules each candidate from measured FLOP and
transfer rates, and ranks by makespan then stable identity. `recipe-scheduler`
uses critical-path list scheduling, measured compute and transfer duration,
queue/completion slots, compute lanes, directed-link lanes, duplex resources,
and discovered transfer/compute overlap. Longer routes are explicit chained
one-hop transfer tasks with resident intermediates.

`recipe-prepare::Preparer` first reserves every required device, resolves the
artifact catalog, and obtains a finite `ProgramPlannerSearch`. It distinguishes
candidate rejection during realization, stabilization, or final capacity from
fatal preparation and teardown errors. Stable realization records exact
artifact identities, resources, reservations, and capacity. `FinalizedBundle`
contains immutable loop domains, tasks, artifacts, resource slots, one init
image manifest per device, arena layouts, alias contracts, and resolved value
locations. The finalization validator checks draft and realization identities,
artifact provenance, capacity, layout bounds and non-overlap, task phase and
domain coverage, and external output routes.

`recipe-core` reduces executable model semantics to two task kinds plus a
specialized metric transfer: `TaskKind::Calculation`, `TaskKind::Transfer`, and
`TaskKind::Metric`. `Metric` is a four-byte device readback, not a third model
work category. Init admission and exit egress are transfers. Compiler,
discovery, allocation, native image loading, and resource realization are
pre-loop preparation, never loop model work.

## Native backend ownership

`recipe-cuda` owns the reviewed 64-bit Linux CUDA Driver API dynamic loader,
capability-gated symbols, contexts, modules, streams, events, device buffers,
pinned host buffers, and asynchronous completion tokens. It does not expose the
CUDA Runtime API. `recipe-hsa` owns ROCr/HSA discovery, sessions, queues,
allocations, executable code objects, AQL dispatch, signal pools, and
nonblocking completion tokens. It does not create a competing raw KFD path.
`recipe-host` owns preallocated RAM and disk byte transport only.

`recipe-native-executor` adapts those resources to the backend-neutral executor.
Its `LocalCandidateFactory` realizes resources before handoff, its
`LocalBackend` submits only finalized work, and its `NativeExecutionEvidence`
records image loads, entry lookups, queues, completion objects, persistent
allocations, zero loop realization calls, and completed teardown. A successful
root report therefore proves that compiler/loader/allocation work occurred
before the running loop and that retained resources were destroyed before the
report escaped.

## Artifacts and output boundaries

The root's user-owned artifact contract is deliberately small:

| Artifact | Producer | Contents |
| --- | --- | --- |
| `.ogdl` semantic model | Dense checkpoint, KNN model, or Bayesian model save | Row-free schema, semantic encodings, effective topology, parameter images and optimizer state where applicable, loop/training bounds, and compatibility identities. |
| `.cubin` | Dense training save on NVIDIA | One authenticated realized native image when the requested format has exactly one matching image. |
| `.hsaco` | Dense training save on AMD | One authenticated realized native image under the same uniqueness rule. |

Plans, journals, measured profiles, caches, temporary source files, native
receipts, and acceptance records are execution or diagnostic state, not public
model artifacts. Checkpoint writes use bounded, create-new temporary files,
sync, atomic install, and a live save-path capacity check that preserves the
required user reservation. Missing optional save declarations write no model
files.

## Implementation boundary versus specification

`API.ogdl` is intentionally broader than any one current compiler dispatch.
The root preserves those declaration methods and validates their structural
meaning, while concrete execution maps only the cases with an owned
f32/int32 graph and native path. `recipe_ops::LoweringAvailability::Unsupported`
and root `TrainingError::Unsupported`/`InferenceError::Unsupported` are the
honest boundary. They are not permission to silently reinterpret a declaration,
run it on the CPU, select a near-match kernel, skip a device, or use a legacy
vendor library.

The implementation/spec split is visible at these seams:

- API construction is broad and side-effect free; execution is narrow and
  evidence-producing.
- `operation-surface.txt` keeps every source-qualified legacy symbol visible;
  `recipe_ops` classifies each symbol instead of deleting unsupported entries.
- Semantic model formats retain enough schema and topology to support exact
  resume and target-free inference; a native kernel alone cannot replace the
  semantic model.
- Probe estimates are configuration input to bounded measurement, not runtime
  scheduling values.
- Build, check, format, and lint commands prove structural validity only.
  Runtime acceptance must enter through the public command or example, use real
  datasets and real CUDA/HSA hardware, and independently inspect resulting
  state, lifecycle, artifacts, and correctness.

## End-to-end commands

Structural workspace checks:

```text
cargo build --workspace
cargo check --workspace --all-targets
cargo build --examples --release
cargo +nightly fmt --all -- --check
cargo run --bin recipe -- probe --help
```

Measured native setup and public workflows:

```text
cargo run --release --bin recipe -- probe
cargo run --release --example train
cargo run --example cookbook
```

The hardware acceptance runner in [`acceptance/README.md`](../acceptance/README.md)
is the runtime gate. It requires a measured profile, the real datasets and
models, the required native toolchain and driver, and the pinned comparison
oracle where a correctness/performance comparison is declared. Missing
prerequisites fail the acceptance command rather than becoming a skipped pass.

## Source-trace index

The shortest path from a public call to its implementation is:

```text
recipe.data / recipe.model / recipe.train / recipe.infer
  -> src/facade.rs + src/api.rs

Train::run
  -> src/training.rs
  -> src/data_prepare.rs
  -> recipe-ingest
  -> recipe-training compile.rs / model.rs / checkpoint.rs
  -> src/native_prepare.rs
  -> recipe-probe + recipe-native-probe
  -> recipe-prepare + recipe-planner + recipe-scheduler
  -> recipe-native-executor + recipe-executor
  -> TrainingReport + .ogdl/.cubin/.hsaco writes

Infer::evaluate
  -> src/inference.rs
  -> src/data_prepare.rs target-free path
  -> recipe-training inference.rs
  -> src/native_prepare.rs
  -> recipe-prepare + recipe-native-executor + recipe-executor
  -> InferenceReport + post-exit prediction rows

recipe probe / run / convert
  -> src/cli.rs
  -> src/source_frontend.rs where a Rust source rewrite is needed
  -> native probe, rustc child, or bounded GGUF structural converter
```

The root package owns orchestration, declaration semantics, and public error and
report boundaries. It does not duplicate graph lowering, operation definitions,
planner logic, scheduler equations, native driver ownership, or executor
typestate that already live in the focused workspace crates.
