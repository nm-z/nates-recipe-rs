# `src/facade.rs`

`facade.rs` is the root crate's public composition layer. It does not contain
the data parser, graph compiler, native compiler, or executor. It assembles
the public declaration API, re-exports the implementation crates for advanced
callers, exposes the normative operation lowering boundary, and keeps the
short `recipe.data(...); recipe.model(...); recipe.train(...)` declaration
syntax connected to the actual runtime entry points.

The source is included at the end of [`src/lib.rs`](../../src/lib.rs), after
the root crate has re-exported the preparation, training, inference, and native
types. The two files included by this module are [`src/api.rs`](../../src/api.rs)
and [`src/cli.rs`](../../src/cli.rs). The public API therefore has two useful
levels:

* the facade declarations (`recipe`, `Data`, `Model`, `Train`, and `Infer`),
  which are immutable descriptions until `run()` or `evaluate()` is called;
* the root re-exports and `engine`/`operations` modules, which expose concrete
  preparation, compilation, native, and operation types to callers that need a
  lower-level boundary.

This document separates current implementation from the normative sources:
[`API.ogdl`](../../API.ogdl) defines the complete declaration grammar,
[`system-contract.md`](../../system-contract.md) defines lifecycle, artifact,
dtype, scheduling, and failure rules, [`topology/contract.toml`](../../topology/contract.toml)
seeds bounded probing rather than production scheduling, and
[`operation-surface.txt`](../../operation-surface.txt) preserves the finite
source-qualified compatibility inventory. Where a valid grammar declaration
has no complete implementation, this document records the fail-closed boundary
instead of presenting construction as execution.

## Module composition

The module layout is deliberately explicit in `facade.rs`.

| Facade item | Definition or re-export | Role |
| --- | --- | --- |
| `cli` | `include!("cli.rs")` | Private-state, probe, conversion, and `recipe run` support used by the root runtime. |
| `api` | `include!("api.rs")` | Declaration types, builders, validation, source-frontend hidden methods, and report accessors. |
| root `pub use api::*` | all public `api` items | Makes `recipe::*`, `Data`, `Model`, `Train`, `Infer`, losses, metrics, and layer helpers available from the crate root. |
| `engine` | aliases for the workspace crates | Gives advanced callers one dependency-clean namespace without changing declaration state. |
| `operations` | selected `recipe_core` and `recipe_ops` types plus wrappers | Exposes the finite normative operation registry and its fail-closed lowering/materialization boundary. |
| `Recipe` and static `recipe` | lines 127-133 | The value used to begin a declaration chain. |

`Recipe` is a zero-sized value. The exported lower-case `recipe` is a static
instance, preserved for the declaration syntax in `API.ogdl` and the examples.
`Recipe::train()` and `Recipe::infer()` are `const` constructors for policy
values. They do not consult or mutate the declaration sequence. `data()` and
`model()` are the calls that establish sequence state.

The `engine` aliases are `cluster`, `core`, `cuda`, `executor`, `host`, `hsa`,
`ingest`, `kernel`, `language`, `math`, `native_executor`, `native_probe`,
`ogdl`, `ops`, `planner`, `prepare`, `primitives`, `probe`, `program`,
`remote`, `scheduler`, `text`, and `transport`. They are ordinary module
aliases. Importing one does not opt a declaration into a different runtime
path and does not bypass the measured preparation boundary.

## Declaration state and transitions

The facade stores only the latest `Data` and `Model` declaration for the
current thread:

```text
thread-local RECIPE_SEQUENCE = { data: Option<Data>, model: Option<Model> }
```

The sequence is a `RefCell` inside a `thread_local!` slot. It is not a process
global registry, it is not shared across threads, and it does not contain
runtime handles or allocations.

| Call | State transition | Return value |
| --- | --- | --- |
| `recipe.data(sources)` | Build an empty `Data`, call `Data::set` once per source, store the resulting clone, and clear the stored model. | `Data` |
| `Data::set`, `target`, `exclude`, `split`, `norm` | Store the changed immutable declaration clone as the current data. | Changed `Data` |
| `recipe.model()` | Build an empty `Model` and store it. Existing data is retained. | `Model` |
| Any `Model` builder | Store the changed immutable declaration clone as the current model. | Changed `Model` |
| `recipe.train()` | No sequence change. | New `Train` policy |
| `recipe.infer()` | No sequence change. | New `Infer` policy |
| `Train::run()` | Take both sequence entries, then compile and execute. | `TrainingResult<TrainingReport>` |
| `Infer::evaluate()` | Take both sequence entries, then compile and execute. | `InferenceResult<InferenceReport>` |

`Data` and `Model` builders use a value receiver. A chain remains immutable to
callers, while each method calls `remember_recipe_data` or
`remember_recipe_model` with its clone so that a later no-argument terminal
call sees the completed chain. Calling `recipe.data(...)` after a model starts
a new data declaration and intentionally clears the old model. Calling
`recipe.model()` after data preserves the data. Calling either terminal method
consumes both entries, even when validation or execution later fails.

The two `take_recipe_*_sequence` helpers use the same underlying take
operation but provide different diagnostics. Because both options are taken
before either missing-value check, a failed terminal call also clears any
other entry that was present. A missing data declaration reports that the
terminal needs `recipe.data(...)`; a missing model declaration reports the
corresponding `recipe.model()` requirement.

This sequence is the reason a normal Rust call is written as:

```rust
recipe.data(path).target("target").norm(z_score).split(0.8);
recipe.model().layer(8).silu().layer(1).loss(bce);
recipe.train().optimizer(adamw).epochs(1).lr(0.001).cos().run()?;
```

The declarations themselves do not read `path`, inspect a checkpoint, probe a
GPU, compile a kernel, allocate memory, or start a loop. Those operations begin
only after the terminal has resolved the sequence.

Because the sequence is thread-local, constructing a `Data` or `Model` on one
thread and calling the no-argument terminal on another does not transfer the
sequence entry. Advanced callers that intentionally cross a thread boundary
must carry the declarations to an explicit compile/run boundary rather than
expecting the static `recipe` value to provide shared state.

## Accepted source and predicate inputs

`Recipe::data` accepts a private `IntoDataSources` bound. The implementations
accept `&str`, `String`, `&String`, arrays and vectors of values implementing
`AsRef<str>`, slices of such values, and `()`. The `()` implementation exists
for the source frontend's zero-argument spelling and creates an empty source
list. An empty source list remains a declaration until `Data::validate` or
preparation rejects it.

`Data::set` appends one nonempty path. `Data::target` accepts one target or a
sequence through `IntoTargets` (`&str`, `String`, `[&str; N]`, `&[&str]`, or
`Vec<String>`). A target sequence replaces the previous target sequence; it is
not appended. Each `exclude(...)` call appends its valid column patterns and
conditions in declaration order.

`Data::norm(...)` replaces a prior normalization choice, just as a later
`Model::loss(...)` replaces a prior objective. Neither operation performs the
normalization or loss calculation in the declaration phase.

`Data::exclude` accepts column patterns or typed [`Condition`](../../src/api.rs)
values through `IntoExclusions`. The `cond!(column OP value)` macro constructs a
condition without evaluating the column as a Rust expression. A condition
retains its exact comparison operator and value category. Floating-point
values are stored as declaration bits, then preparation narrows them to finite
non-underflowing `f32` predicates. Column names and patterns must be nonempty,
and condition columns and finite floating values are checked when the
exclusion is declared. Narrowing to a non-underflowing `f32` is checked again
at data preparation.

`Exclusion::Column` carries a pattern interpreted by ingest, while
`Exclusion::Condition` carries the typed row predicate. `IntoExclusions` also
accepts one or many strings, one `Condition`, one `Exclusion`, or a vector of
already constructed exclusions.

`Condition::column()`, `operator()`, and `value()` are read-only accessors.
`ComparisonOperator` covers equality, inequality, and all four ordered
comparisons. `ConditionValue` preserves signed and unsigned integers, exact
floating declaration bits, booleans, and text until the ingest boundary.
`IntoConditionValue` implements the signed and unsigned primitive integer
families, `isize`/`usize`, `f32`/`f64`, `bool`, `&str`, and `String`.
The `cond!` macro is exported at the crate root, so `use recipe::*` makes its
declarative spelling available without importing an internal helper.

The three data normalizations are `z_score`, `min_max`, and `l2_norm`. They are
policy values only. Training preparation later applies the selected semantic
normalization in the owned calculation graph over the prepared partition;
`Data` construction does not mutate values. Inference rejects a redeclared
normalization because the saved model owns that schema.

The read accessors are intentionally structural. `source()` returns the first
source or an empty string when none exists; `sources()`, `targets()`,
`exclusions()`, and `condition_exclusions()` return the retained slices;
`split_fraction()` decodes the stored narrowed `f32`; and `normalization()`
returns the selected enum. They do not trigger validation or I/O.

## Deferred declaration errors

Most facade methods return `Self`, not `Result`. They preserve the first
invalid declaration in a private `deferred: Option<DeclarationError>` and keep
returning the value so the full chain remains inspectable. `Data::validate`,
`Model::validate`, `Train::validate`, and `Infer::validate` return that first
error before deeper work.

The public error kinds are `EmptyValue`, `InvalidExclusion`, `InvalidSplit`,
`InvalidLayer`, `InvalidActivation`, `InvalidBayes`, `InvalidLearningRate`,
`InvalidMetric`, `InvalidTrainingConfiguration`, and
`InvalidInferenceConfiguration`. The error includes a kind and human-readable
detail. Runtime wrappers convert declaration errors to `TrainingError::Declaration`
or `InferenceError::Declaration`.

Some values are checked immediately and retained only if valid. Examples:

* `Data::split` requires a finite fraction strictly inside `(0, 1)` in both
  `f64` and narrowed `f32` form, and stores the narrowed bits.
* `clip(maximum_norm)` accepts only finite positive values representable as
  `f32`; an invalid value is retained as invalid bits so `Model::grad` can
  defer a useful error.
* layer widths, tree depths, cluster counts, vocabulary sizes, attention heads,
  and neighbor counts must be nonzero.
* activations and normalization can only follow a block that accepts those
  operations. A terminal KNN block rejects both.
* Bayesian child names, parent names, duplicate parents, duplicate children,
  self-dependencies, and cycles are checked as the dependency list grows.
  An invalid newly appended Bayesian edge is removed again before its first
  error is retained.
* a checkpoint-backed model cannot also contain inline layers, Bayesian edges,
  a loss, or gradient policy.

Immediate checks do not prove runtime support. A declaration can pass facade
validation and still fail in data preparation, graph compilation, model-file
decoding, native preparation, or execution.

## Model declaration

`Model` is a backend-neutral value containing ordered `LayerSpec` blocks,
ordered Bayesian dependencies, an optional `Objective`, an optional global
gradient clip norm, an optional checkpoint path, and the deferred error. It
contains no loaded weights, device handles, allocation, or mutable global
registry.

### Model roots and block forms

`Model::load(path)` declares an external model source. The path must be
nonempty, and the model must not already have an inline definition. Inference
later accepts `.ogdl` semantic artifacts and `.gguf` Llama artifacts. Dense
training rejects a loaded model because training compilation starts from an
inline architecture and target schema.

`Model::layer(usize)` is shorthand for a dense `LayerSpec`. The same method
accepts a `LayerSpec` through `IntoLayer`. The remaining block builders retain
the following declaration data:

| Builder | Retained block | Important validation or lowering rule |
| --- | --- | --- |
| `perc(count)` | parallel perceptron block | Nonzero count; lowered as a dense block with `Perc` kind. |
| `conv(filters, kernel)` | convolution with a mutable activation slot, initially linear | Both dimensions nonzero; `.relu()`, `.prelu()`, and other activations replace the slot. Compilation additionally requires the kernel not exceed the resolved logical input length. |
| `pool(size)` | pooling block, optionally followed by an inferred grouped-to-dense connection | A following `.layer(neurons)` records the destination width and runtime validates the immediate adjacency. A final pool is rejected by dense compilation because an explicit output layer is required. |
| `kmeans(clusters)` | K-means block, optionally followed by an exact grouped-to-dense connection | The connection records the declared cluster count and following neuron width. |
| `lgbm`, `cbst`, `xgbst` | one-tree boosted block, or a booster attached to a preceding `forest` | A nonzero depth is required, and dense lowering bounds depth to `1..=30` for checked complete-tree traversal. `forest(trees)` must be followed by one of these to become a complete forest. |
| `forest(trees)` | forest with an initially empty booster slot | Model validation rejects a forest whose nested booster was never supplied. |
| `knn(neighbors)` | standalone all-output KNN block | It must be the only block, has no objective, optimizer, epochs, logs, plots, or native training kernel. |
| `residual(branch)` | residual branch plus output width and identity-or-linear-projection skip | The branch must contain a nonzero-width layer. Source syntax with two or more branch arguments is rewritten to an array before Rust compilation. |
| `embed(dimensions)` then `vocab(vocabulary)` | fixed-token embedding | `vocab` must immediately follow an embedding and both values are nonzero. Dense compilation requires identity data handling and a vocabulary in the checked int32 token-index domain. |
| `attn(heads)` | attention block | Dense compilation currently requires it to be exactly one block immediately after a leading embedding, with a nonzero head count dividing the embedding dimension. |
| `rnn`, `gru`, `lstm` | one recurrent block | The public chain may append operations, but the current dense mapper rejects chained recurrent operations. Compilation requires one leading recurrent block, with no later block of any kind. |

`layer(width)` after `pool` or `kmeans` is not merely another dense layer:
the previous block receives a `GroupToNeuronConnection`. Its routing can later
be resolved as identity, contiguous expansion, contiguous contraction, or
full connectivity when the actual group count is known. Nondivisible widths
use full connectivity, and exact K-means counts must match.

The residual helpers `layer(width)` and `relu()` construct
`ResidualOperation` values. A residual's output width is the width of its last
branch layer, and `Model::residual` retains that width explicitly so later
validation and checkpoint metadata cannot silently infer a different one.
These free helpers are distinct from the `Model::layer(width)` and
`Model::relu()` chain methods.

### Operations on the last block

The activation methods are `relu`, `leak`, `sigmoid`, `tanh`, `selu`, `gelu`,
`silu`, `elu`, `prelu`, `cos`, `exp`, `log`, `ln`, `huber`, and `tan`.
For dense, perceptron, recurrent, and residual blocks they append an ordered
`LayerOperation`. For convolution they set the block's activation field. For
pooling, trees, K-means, embedding, attention, no block, or terminal KNN they
defer `InvalidActivation`. `log` means the signed logarithm
`sign(x) * ln(abs(x))`; `ln` means ordinary natural log on strictly positive
inputs. The device fault boundary handles invalid runtime values.

`norm(layer_norm)` and `norm(batch_norm)` append a normalization operation only
to dense, perceptron, recurrent, or residual blocks. They reject all other
last-block forms. `loss(...)` stores either a built-in loss (`mse`, `mae`,
`huber`, `bce`, `ce`, or `focal`) or a cloned model reference. A referenced
objective is retained by the declaration but has no dense training lowering.
`grad(clip(...))` stores one global gradient clipping norm.

`Model::layers()`, `bayes_dependencies()`, `objective()`,
`gradient_clip_value()`, and `weights_source()` expose the retained declaration
without changing it. `BayesDependency::child()` and `parents()` expose the
edge text. `GroupToNeuronConnection::groups()`, `neurons()`, and `routing()`
expose the grouped routing contract after an actual group count is known.

Final model validation also enforces KNN's one-block terminal rule, checks every
grouped connection against its immediate following dense layer, revalidates
the complete Bayesian graph, and recursively validates a referenced objective
model. It does not inspect the filesystem for a loaded source.

`Loss` is an enum, not a host callback. MSE, MAE, unit-delta Huber, BCE from
logits, categorical cross entropy from logits, and fixed-alpha/fixed-gamma
binary focal loss are mapped by the dense compiler. `Optimizer` currently has
one value, Recipe-owned AdamW. `Activation` includes the linear, trigonometric,
exponential, logarithmic, smooth, rectifier, sigmoid, tanh, SELU, GELU, SiLU,
ELU, and PReLU forms retained by `LayerOperation`.

### Bayesian declarations

`bayes(child, parents)` appends an edge in source order. The edge direction is
parent to child, although the builder's arguments are written child first. A
child is declared at most once, parents may be implicit roots or declared
later, and the complete graph must be acyclic. The current executable branch
is observed categorical conditioning: each declared child is bound to the
corresponding data target, while parents remain observed categorical inference
features. Every child and parent must be dictionary-categorical and every
retained training row must contain the observations. A target child cannot be
another conditional's inference parent. The branch does not sample or
marginalize a target child.

Repeated valid `.bayes(...)` calls retain one conditional per declaration in
source order. Shared parent schemas are admitted once during target-free
inference, while each child keeps its own dictionary, reference rows, mixed-
radix configuration, and probability range.

The lower training IR gives the declarations these concrete meanings:

* K-means emits rooted L2 distances to deterministic centroids. A fresh run
  initializes centroids from prepared training rows in row order, one Lloyd
  transition is performed per epoch, and centroids are loop-carried semantic
  state rather than AdamW parameters. Validation and inference use the final
  saved centroids without updating them.
* A standalone boosted-tree block is one tree. `forest(trees)` plus a nested
  booster is exactly the declared tree count. Trees are terminal, structures
  are built in `init`, leaf values are the learned parameters, and inference
  traverses the saved split tensors without rebuilding or resampling.
* Embedding consumes exact int32 token IDs in the prepared feature-column
  sequence. It does not fit a tokenizer, infer padding, or apply numeric
  normalization. Optional attention is one causal block immediately after the
  embedding, with heads dividing the embedding dimension.
* RNN, GRU, and LSTM consume one normalized numeric scalar per feature column in
  column order. Each row starts from zero hidden state (and zero cell state for
  LSTM), returns only its final hidden state, and carries no state between rows,
  epochs, runs, or inference calls.

## Training policy

`Train` is an immutable policy value. Its fields are an optional epoch bound,
learning-rate bits, optional warmup, optional decay schedule, optional
optimizer, log declarations with cadence, plot items, independent resume and
save artifact declarations, and a deferred error.

| Method | Behavior |
| --- | --- |
| `epochs(n)` | Requires nonzero `n`; no bound means an unbounded loop for dense training. |
| `lr(rate)` | Requires finite positive `f32`-representable input, stores `f32` bits, and selects linear decay by default. |
| `warmup(n)` | Requires nonzero warmup epochs. Validation later requires warmup less than finite total epochs. |
| `cos()` / `exp()` | Select cosine or exponential decay. They are policy methods on the train value, not model activations. |
| `optimizer(adamw)` | Records the Recipe-owned AdamW optimizer. Dense training requires this exact value. |
| `log(items)` | Appends items and starts a log declaration with cadence one. `every(n)` changes the most recent declaration and must follow `log`. |
| `plot(items)` | Appends selected metrics for bounded plot retention. It does not create a live output stream by itself. |
| `resume(path)` | One path must have a `.ogdl` extension. Missing files are handled as fresh training at compile time. |
| `save(path)` | One path routes by extension to `.ogdl`, `.cubin`, or `.hsaco`. |
| `run()` | Consumes the preceding data/model sequence and dispatches to Bayesian, KNN, or dense execution. |

The source frontend supplies hidden `__recipe_resume_pair` and
`__recipe_save_pair` methods for the literal two-path forms. The first path
must be `.ogdl`; the second must be `.cubin` or `.hsaco`. A second one-path
declaration is rejected, so the two-path spelling is the only way to request
both artifacts in one declaration.

Resume and save declarations are independent. Omitting resume starts a fresh
run and does not disable save. A missing resume model or missing resume kernel
is a normal fresh/recompile path. When both exist, the semantic model's
authenticated native metadata, current program digest, measured topology and
discovery identities, target identity, toolchain identity, and kernel digest
are checked before the supplied kernel can be reused.

Semantic resume admits saved model parameters and AdamW moments into a newly
compiled graph. The new declaration supplies the new horizon, warmup, decay,
and stop policy; schedule position and automatic-stop state are not silently
continued from the old phase.

Omitting `save` exports nothing. The only user-owned exports are the semantic
`.ogdl` model and, for a dense native run, the exact realized `.cubin` or
`.hsaco` image. Journals, plans, caches, profiles, and intermediate execution
files are not public model artifacts. The semantic OGDL never embeds native
bytes; it stores schema, topology, parameters, optimizer moments, and native
identity metadata that authenticates a separately saved kernel.

Dense training requires an explicit decay schedule and the AdamW optimizer. A
finite epoch bound gives linear, cosine, or exponential decay an endpoint. An
unbounded run may only use a constant post-warmup rate; `.lr(...)` supplies the
linear policy that is reduced to that constant case. KNN and Bayesian reference
preparation intentionally reject optimizer, learning-rate, schedule, warmup,
epoch, log, and plot declarations because they have no optimizer loop.
Dense training also requires one built-in loss objective. A model-reference
objective is declaration-valid but fails closed at the dense lowering boundary.

The training metric constants are `Loss`, `Accuracy`, `R2`, `AuRoc`, `AuPrc`,
`Brier`, `CalibrationError`, `Epoch`, `Lr`, `Time`, and `Device`, with lower-case
and short aliases such as `loss`, `accuracy`, `epoch`, `lr`, `time`, `r2`, and
`device`. A metric is retained in the policy before its objective-dependent
meaning is checked. Dense training maps metric families as follows:

* binary accuracy and calibration metrics require BCE or focal loss;
* multiclass accuracy requires categorical cross entropy;
* R2 requires scalar regression loss;
* loss, epoch, learning rate, time, and device are available where their state
  exists;
* one run cannot request multiple validation metric families at once.

When validation was requested but the prepared validation partition has no
known targets, or binary validation has only one known class, compilation
retains a `ValidationMetricStatus::Unavailable` result. Execution reports the
availability line and continues the training lifecycle; it does not fabricate
a metric or silently use training rows as validation rows.

`every(interval)` is per preceding `.log(...)` declaration. The runtime uses a
bounded nonblocking channel for selected live metrics, so a full consumer queue
drops a live notification without backpressuring the executor. Final metrics
remain in the completed execution evidence.

The production `RunJournal` is capacity-planned before `init`. It retains the
ordered lifecycle detail needed for the first loop iteration and compacts
repeated loop events and physical calls into bounded summary counters instead
of allocating in proportion to an unbounded epoch count. The returned journal
is evidence of the completed run, not an ordinary mutable declaration field.

The presenter groups samples by epoch, emits only requested cadence fields,
keeps each plot as a bounded 64-sample tail with a total count, and flushes the
last completed epoch even when it is off cadence. `Epoch` and `Time` are host
presentation fields, while `Device` is printed after native teardown from the
realized target identities.

`TrainingReport` exposes the selected `TrainingModelKind`, optional dense
`RunId` and `BundleIdentity`, dense external outputs, final metrics, realized
native kernels, native execution evidence, full-partition training evidence,
the completed journal, validation status, and graceful-stop status. KNN and
Bayesian reports instead expose their immutable semantic model and return
`None` for dense execution identities, native evidence, and optimizer metrics.

`Train::epoch_bound()`, `learning_rate()`, `warmup_epoch_bound()`,
`learning_rate_schedule()`, `optimizer_spec()`, `log_items()`, `plot_items()`,
and `resume_source()` are public read accessors. The native resume source and
save destinations are crate-visible runtime accessors because exposing those
paths as ordinary policy state would couple callers to artifact realization.

## Inference policy

`Infer` contains only a log list and a deferred error. The public sequence still
gets the model path from `recipe.model().load(path)`, not from a method on
`Infer`. `Infer::log` accepts the same metric constants but rejects every
target-derived metric (`Loss`, accuracy and validation families), as well as
`Epoch` and `Lr`, because target-free inference has neither targets nor
optimizer state. `Time` and `Device` are the supported inference policy
metrics.

`Infer::evaluate` resolves and consumes the current data/model sequence,
validates the policy, requires a loaded model source, and invokes
`evaluate_inference_declaration`. `InferenceDeclaration` then provides read-only
access to the selected `Model`, optional `Data`, and `Infer` policy for the
runtime compiler.

Inference data is target-free. It must not declare targets, a train split, or a
normalization. Column and row exclusions remain valid. The saved semantic model
owns the feature schema, target interpretation, normalization, dictionaries,
and output shape.

`Infer::log_items()` returns the retained policy slice. After resolution,
`InferenceDeclaration::model()`, `data()`, and `policy()` expose the immutable
pair and policy that the compiler actually received.

## Source frontend lowering

The command-line `recipe run SOURCE.rs` path calls
`source_frontend::lower_recipe_source` before invoking `rustc`. The frontend
parses the source with `syn`, classifies explicit `recipe`, `Data`, `Model`,
`Train`, and `Infer` receivers, and follows local bindings until their receiver
kind is known. It never asks a compiler diagnostic to decide whether a call is
Recipe syntax and it does not retry based on `E0061` or another error.

Classification recognizes path, method-call, grouping, parentheses, and
reference expressions. Explicit local type annotations win; otherwise local
initializers are resolved to a fixed point. Methods on a known `Data`, `Model`,
`Train`, or `Infer` value preserve that receiver kind, while only the four
facade methods `data`, `model`, `train`, and `infer` create a kind from the
`recipe` receiver. Unclassified expressions are left untouched and reach
ordinary Rust compilation.

The lowering is a source edit, not a second public API:

| Source spelling | Generated spelling | Reason |
| --- | --- | --- |
| `recipe.data()` | `recipe.data(())` | Rust has no default argument for the generic source parameter. |
| `recipe.model().residual(a, b, ...)` | `recipe.model().residual([a, b, ...])` | The public method accepts one `IntoResidualBranch` value. |
| `.grad(clip: EXPR)` | `.grad(::recipe::clip(EXPR))` | Named-field syntax is part of the normative grammar, while Rust receives a `Grad` value. |
| `.save("model.ogdl", "kernel.cubin")` | `.__recipe_save_pair("model.ogdl", "kernel.cubin")` | Preserve the literal two-argument grammar while keeping the one-path method unambiguous. |
| `.resume("model.ogdl", "kernel.cubin")` | `.__recipe_resume_pair(...)` | Same two-artifact rule as save. |
| `.run(&model, &data)` | `.__recipe_run_with(&model, &data)` | The grammar permits an explicit terminal pair; the public no-argument method remains sequence-based. |

Named `grad` fields are parsed strictly. Duplicate or unknown fields, missing
`clip`, or malformed expressions produce a source-local diagnostic with a
highlighted original span. Rewrites are written to a temporary hidden source
file next to the input, compiled with path remapping, and removed on drop.
Compiler JSON diagnostics are remapped back to the original source. If no
Recipe edit is needed, the original source is compiled directly. If the source
cannot be tokenized or parsed for classification, lowering returns no rewrite
and ordinary `rustc` diagnostics remain authoritative.

The CLI canonicalizes the requested path and requires a regular file before
reading it. It compiles the resulting source with the current recipe library
and `-Dunused_must_use`, creates the child binary beneath the private run state,
forwards compiler output, then executes the binary in the source directory.
Runtime stdout and stderr are streamed live with bounded captured tails for
failure reporting. The frontend therefore supplies syntax adaptation only;
declaration state, validation, preparation, and execution remain in the facade
and root runtime.

## `operations` namespace

`operations` is independent of `RECIPE_SEQUENCE`. It re-exports
`ScalarProgram`, operation descriptors and registries, primitive and
composition requests, workspace formulas, materialization records, and the
typed operation errors from `recipe_ops`.

The registry preserves the numbered `operation-surface.txt` prefix and then
appends the Recipe-owned channelwise max-pool and backward descriptors. Each
descriptor retains a source-qualified `OperationId`, canonical f32/int32
payload contract, alias contract, determinism contract, and one
`LoweringAvailability` value. Duplicate legacy symbols are intentionally
ambiguous until resolved with their exact source.

The wrapper functions are the complete root operation boundary:

| Function | Actual operation |
| --- | --- |
| `registry()` | Returns the finite normative `OperationRegistry` in const context. |
| `all()` | Iterates every source-qualified descriptor in normative order. |
| `resolve(symbol)` | Succeeds only when the symbol has one normative entry; duplicate symbols require an exact source. |
| `resolve_exact(symbol, source)` | Selects one descriptor by public symbol and legacy source. |
| `lower_scalar(descriptor)` | Returns an owned elementwise `ScalarProgram` when the descriptor owns one. |
| `lower_primitive(descriptor, request, hardware)` | Validates request shape and hardware requirements, then returns a `LoweredProgram`. |
| `validate_composition(descriptor)` | Checks that a structured operation is a finite composition of owned primitive families. |
| `materialize(request)` | Builds a finite calculation graph after shapes and typed preparation facts are known. |
| `remaining_compositions()` | Reports structured operations whose concrete ABI or formula has not crossed the fail-closed boundary. |
| `evaluate_workspace(descriptor, dimensions)` | Evaluates the descriptor's checked static workspace formula. |

An operation with no owned scalar, primitive, finite composition, checked
workspace formula, or deterministic host/lifecycle declaration remains
`LoweringAvailability::Unsupported`. The facade does not substitute a legacy
implementation for such an entry. Operation failures are typed as unknown or
ambiguous symbols, wrong lowering kinds, primitive or scalar mismatches,
invalid or unresolved compositions, missing prepared parameters, exhausted
identity namespaces, unsupported concrete shapes, graph materialization
failures, or checked workspace arithmetic/limit failures.

## Training execution path

The real training path starts at `Train::run` in `src/api.rs` and continues
through `src/training.rs`.

The root crate also re-exports `compile_training`, `compile_knn_model`, and
`compile_bayes_model`. These are declaration-to-compiled/semantic boundaries
for advanced callers; they perform bounded data preparation and graph or
reference-model compilation but do not perform native probing or execution.
The normal facade terminal uses the same functions and then continues through
native preparation.

1. `Train::run` takes the current data/model pair. It returns a typed
   `TrainingError::Unsupported` if either declaration is missing.
2. `Train::try_run_with` chooses the model family from the declaration:
   Bayesian dependencies select observed categorical Bayesian preparation;
   a KNN block selects immutable reference-set preparation; all other models
   enter dense training compilation.
3. Bayesian preparation validates that no layer, loaded model, generic loss,
   gradient policy, numeric normalization, optimizer policy, or native kernel
   is mixed into the categorical declaration. It prepares dictionaries and
   conditionals from the training partition, retaining raw observation order
   and dictionary identities rather than a host-fitted probability table. A
   target-free native inference graph later performs one mixed-radix histogram
   and Laplace-one posterior per conditional. It optionally continues an
   existing `.ogdl` Bayesian artifact, returns a report with no native run, and
   writes a requested model artifact.
4. KNN preparation requires exactly one standalone `.knn(neighbors)` block. It
   prepares the exact training partition as an immutable reference set. Native
   inference computes one rooted-L2 neighbor order and one independently typed
   mean or mode per declared target, preserving target order and saved
   dictionaries; a declared data normalization is applied in that saved
   coordinate system to both references and queries. It optionally continues
   an existing KNN semantic artifact,
   returns a report with no optimizer loop or native kernel, and writes a
   requested model artifact.
5. Dense compilation validates policy, data, and model; loads bounded data;
   maps the facade blocks to `DenseBlock` or `DenseLayer`; derives the finite or
   unbounded horizon; maps normalization, loss, validation family, warmup,
   AdamW, and gradient policy; and calls the appropriate owned dense compiler.
   The compiler emits a `recipe-language::CalculationGraph`, whose primitive
   and composition stages use the same `recipe_ops` lowering contracts exposed
   by `operations`; a declaration never selects a legacy CPU or vendor-library
   implementation as a fallback.
6. If a semantic resume model exists, the compiler loads it and applies
   checkpoint state to the newly compiled graph. If a requested native resume
   path exists, its bytes are checked against authenticated semantic metadata
   and held as a prebuilt bundle for current-target realization.
7. The dense path installs the SIGINT guard, then enters
   `with_current_native_preparation`. This opens the exact active measured
   profile, rediscovers the local host and GPU inventory, validates topology
   and toolchain identities, builds one target specification per required GPU
   target, and lends scoped CUDA/HSA bindings and host plans to the callback.
   The native probe is cached per thread only when its configuration remains
   identical; a changed configuration is an identity mismatch, not a silent
   reinitialization.

The active receipt and profile are private, identity-named files. The CLI
checks regular-file and directory shape, canonical paths, effective-user
ownership, private permissions, file size, schema, and SHA-256-pinned native
libraries/tools before the root runtime can borrow them. A stale or changed
receipt therefore fails before target realization.
8. The callback derives runtime tuning from measured rates and capacities,
   constructs host staging, cross-backend, compiler, realizer, provider, and
   preparer objects, and calls the owned
   `prepare_and_execute_local_training_controlled` entry point.
9. Preparation performs discovery validation, placement, artifact generation
   or reuse, native image loading and warming, allocation, and finalization
   before the external data image is admitted. The executor then runs the
   immutable `init -> loop -> exit` lifecycle. Each epoch is one logical update
   over the complete prepared training partition; physical tiling cannot turn
   it into per-sample updates. The facade has no user-facing batch-size or
   partial-batch control.
   The `Preparer` reaches this point through a finite fixed-point search: it
   validates reservations and the measured profile, resolves the exact graph
   artifact catalog, enumerates planned candidates, realizes one candidate at a
   time, performs bounded stabilization and capacity checks, packs arenas, and
   finalizes one immutable bundle. Candidate-local rejection destroys that
   session before the next candidate; fatal realization, teardown, planning,
   finalization, or candidate exhaustion returns a typed preparation failure.
10. A finite run ends at its epoch bound. An unbounded run requires the SIGINT
    stop source and accepts a stop only at a completed loop boundary. Native
    resources are torn down before the completed report is constructed.
11. The dense report retains the completed native journal, final metrics,
    external outputs, realized kernels, native evidence, and full-partition
    training evidence. A requested `.ogdl` model and/or `.cubin`/`.hsaco`
    kernel is written only after the run exits.

Each native training or inference execution receives a `RunId` generated from
the process-local atomic sequence, process identity, and current time. The
`RunId` participates in run-scoped host paths, journal events, bundle evidence,
and returned reports; repeated declarations therefore repeat the complete
lifecycle under a new identity.

The SIGINT guard is process-scoped and installation is serialized. Its signal
handler performs only one atomic stop-request store; it does not allocate,
lock, format, or perform I/O. Dropping the guard restores the previous process
handler. The CLI runner has a separate guard and forwards one SIGINT to its
compiled child, so the child can accept the stop at the same safe epoch
boundary.

At the executor boundary, `PreparedRun::prepare` allocates the fixed journal,
watchdog, phase state, and exit-image capacity. `initialize` admits the exact
per-device external images and completes `init`; `start_loop` enters the first
zero-based iteration; `poll_with_progress_or_stop` is the only live-loop
capability and can advance or accept a stop only after the active iteration is
terminal; `into_exited_loop` proves no loop work remains; and `exit` performs
the planned egress and arena teardown. Finalized loop tasks cannot perform
external transfers, lazily load artifacts, or grow loop-time state. Executor
and backend failures retain journal and cleanup evidence when the failure type
allows it.

The finalized program has only calculation and transfer model work. A
`TaskKind::Metric` is a specialized four-byte device readback transfer used for
user metrics, not a third model operation category. Init admission and exit
prediction/model egress are transfers; discovery, compilation, allocation, and
native-image loading stay in preparation.

The principal training errors are `Declaration`, `Data`, `Compile`,
`Checkpoint`, `Resume`, `NativeKernelSource`, `Native`, `Unsupported`, and
`Runtime`. Their display text identifies the phase, while their source chain
preserves the lower-level typed error where one exists.

The terminal's model-family dispatch is deliberately structural rather than a
string or extension guess:

| Declaration shape | Compiler entry | Native execution | Report/artifact behavior |
| --- | --- | --- | --- |
| `Model` with one or more Bayesian dependencies and no layers/source | `compile_bayes_model` | None during training; native graph is used later by inference | `TrainingModelKind::Bayes`, semantic `.ogdl` only. |
| Exactly one standalone `LayerSpec::Knn` | `compile_knn_model` | None during training; native graph is used later by inference | `TrainingModelKind::Knn`, semantic `.ogdl` only. |
| Inline dense/structured blocks with a built-in objective | `compile_training_graph` and one of the dense block/validation compilers | Measured native `init -> loop -> exit` | `TrainingModelKind::Dense`, semantic `.ogdl` and optional exact native kernel. |
| Loaded model source | No dense training lowering | Not selected by `Train::run`; loaded sources are for inference | `require_supported_model` fails closed rather than ignoring loaded weights. |

The branch is selected before native preparation. Thus KNN and Bayesian
reference preparation do not accidentally enter a dense optimizer loop, and a
loaded checkpoint does not get mistaken for a fresh inline model.

| Boundary | Failure examples | Public result |
| --- | --- | --- |
| Sequence take and declaration validation | Missing preceding data/model, deferred empty value, invalid layer, split, metric, policy, or artifact extension | `TrainingError::Unsupported` for missing sequence, otherwise `TrainingError::Declaration`; inference uses the corresponding `InferenceError` variants. |
| Bounded source/data preparation | Missing targets or split, source framing/limit error, semantic vector error, invalid row predicate, empty prepared partition | `TrainingError::Data` or `InferenceError::Data`. |
| Semantic model or resume decoding | Missing/unreadable file, malformed OGDL, schema/objective/topology drift, unauthenticated or mismatched native bytes | `TrainingError::Checkpoint`/`Resume` or `InferenceError::Model`. |
| Graph lowering | Unsupported model family combination, invalid dtype/shape, output-width mismatch, unavailable metric family, arithmetic or identity exhaustion | `TrainingError::Compile` or `InferenceError::Compile`. |
| Measured native scope | No active profile, bare-metal or ownership violation, changed host/GPU inventory, driver/toolchain mismatch, missing target binding | `NativePreparationError` wrapped as `TrainingError::Native` or `InferenceError::Native`. |
| AOT preparation and realization | Invalid profile/reservation, candidate rejection/exhaustion, artifact or planning failure, stabilization/capacity/finalization/teardown failure | Native runtime wrapper returns a `TrainingError::Runtime` or `InferenceError::Execute` after the typed preparation detail. |
| Executor lifecycle | Invalid init image, loop external transfer, watchdog expiry, device fault, rejected stop state, incomplete exit, prediction/output mapping mismatch | `TrainingError::Runtime` or `InferenceError::Execute`; failures retain bounded journal/cleanup evidence where available. |
| Post-exit report and artifacts | Invalid egress bytes, ambiguous native format, disk capacity/atomic-save failure, output rendering failure | `TrainingError::Checkpoint` or `Runtime`, or `InferenceError::Runtime`. |

## Inference execution path

`Infer::evaluate` resolves an `InferenceDeclaration`, then
`evaluate_inference_declaration` performs the following sequence:

The root crate also re-exports `compile_inference` for callers that need the
immutable compiled program without native execution. The facade terminal uses
the same package compiler before it enters the native lifecycle.

1. Validate inference policy, data, and model declarations and require a
   target-free data policy.
2. Require `Model::load` with a `.ogdl` or `.gguf` path. `.ogdl` loads a dense,
   KNN, or Bayesian semantic artifact. `.gguf` selects the bounded current
   Llama instrument only: unsupported architectures, quantized tensors,
   incompatible head geometry, or invalid exact-int32 token input fail before
   native preparation. Structural GGUF conversion is not execution support.
3. Distill and select the declared input table using exclusions only. The
   semantic model remains authoritative for schema, normalization, targets,
   labels, and output width.
4. Compile the selected model family into a `CompiledModelInference`.
5. Reopen the current measured native preparation, derive measured runtime
   tuning, prepare native artifacts and allocations, and execute the complete
   native inference lifecycle.
6. Build an `InferenceReport`, print requested time/device lines and exact
   prediction rows, and return the fully exited report. Dense/Bayesian/GGUF
   outputs expose validated little-endian `f32` values; KNN exposes typed
   predictions and label decoding instead.

The text report is family-specific but always row-oriented: dense binary rows
print a probability, scalar regression rows a value, multiclass rows the
lowest-index maximum class, label, and probability vector, and multi-target
rows retain saved target names. Repeated Bayesian outputs print one target and
class/probability block per conditional. KNN prints one independently typed
output per saved target, using a decoded label for discrete values. GGUF Llama
prints one selected token and logit per input position; the report still
retains the complete raw logit image for programmatic access.

Inference dispatch is likewise rooted in the decoded semantic document rather
than a caller-provided model-kind flag:

| Model source/root | Prepared table | Compiled program | Report payload |
| --- | --- | --- | --- |
| `.ogdl` root `recipe` | Saved dense feature schema and normalization | `CompiledModelInference::Dense` | Dense prediction bytes plus saved class/target decoding. |
| `.ogdl` root `recipe-knn-model` | Saved KNN feature schema | `CompiledModelInference::Knn` | One typed prediction per saved target. |
| `.ogdl` root `recipe-bayes-model` | Union of saved categorical parent schemas | `CompiledModelInference::Bayes` | Concatenated probability blocks per conditional. |
| `.gguf` with supported Llama metadata | Exact token-ID sequence | `CompiledModelInference::GgufLlama` | Raw token logits and top-token tabular rows. |

The semantic decoder examines the document root, then the selected decoder owns
complete syntax, version, canonicality, and family validation. There is no
fallback from one decoder to another when a root or architecture is invalid.

`InferenceReport` exposes the selected `InferenceModelKind`, run and bundle
identities, singular dense/Bayesian/GGUF prediction bytes or typed KNN
predictions, the completed journal, native kernels where retained, native
execution evidence, elapsed loop time, and realized device origins. Its value
iterator is available only after the native exit boundary has checked dtype,
shape, and byte count. Categorical class accessors decode saved dictionaries;
they do not refit labels or infer host-side classes.

Inference errors are `Declaration`, `Data`, `Model`, `Compile`, `Native`,
`Execute`, `Runtime`, or `Unsupported`. A missing active measured profile,
model file, target-free input row, schema match, native target, or executable
operation is a failure, not a skipped run.

## Data preparation boundary

Training calls `prepare_data`, which imposes the finite default source, record,
field, and field-byte limits, requires at least one target and an explicit
split, distills all declared sources in order, infers typed vectors without
imputation, applies column and condition exclusions, and produces the prepared
training and validation partitions. Source order and deterministic directory
or archive member order are retained; the aggregate bounds cover all declared
sources and expanded members. Row predicates run against the original row
before excluded helper columns are removed, and the train fraction partitions
only after selection. Numeric normalization and target semantics are applied
by the owned preparation/compiler stack, not by `Data` builder calls.

Inference calls `distill_data` and `select_target_free_data` instead. It does
not infer training targets, split rows, or redeclared normalization. Conditions
and column patterns still apply before the saved model's schema is matched;
retained rows and columns preserve source order, and an excluded required model
feature fails as missing rather than being silently restored.

## Actual support and specification gaps

`API.ogdl` is the normative declaration inventory. The facade intentionally
keeps entries visible even when a valid declaration reaches a fail-closed
unsupported boundary. The current source and runtime establish these concrete
gaps:

* `API.ogdl` lists `.infer().load("model.ogdl" OR "model.gguf")`, but the
  current `Infer` type has no `load` method. The working spelling is
  `recipe.model().load(path); recipe.infer().evaluate()`. This is a surface
  mismatch, not evidence that inference loading is absent.
* `API.ogdl` lists `.run(model, data)`. The public `Train::run` takes no
  arguments; `source_frontend` rewrites the literal two-argument source call to
  the hidden `__recipe_run_with` method. A direct ordinary Rust caller should
  use the sequence form.
* `API.ogdl` lists a broad model grammar. Facade validation records all listed
  blocks, but execution still applies family-specific constraints: embedding
  needs immediate vocabulary and identity input; attention needs the leading
  embedding arrangement; recurrent chained operations are rejected; supervised
  trees are terminal; grouped routing requires an immediate matching layer;
  classification output blocks must emit logits; and final pooling is invalid.
* `Model::load` is an inference source, not a dense-training initializer.
  Dense training rejects a loaded model rather than silently rebuilding or
  ignoring its weights.
* KNN and observed categorical Bayesian training produce semantic `.ogdl`
  artifacts but no native training kernel. Requesting a kernel artifact for
  either family is an explicit unsupported error.
* Target-free inference rejects targets, splits, and normalization declarations
  even though those fields are legal on `Data`. The saved model supplies those
  meanings.
* Dense training requires `.optimizer(adamw)` and an explicit decay policy.
  The facade permits an otherwise well-shaped `Train` value to be built
  without them, then reports the missing policy at compile time.
* Dense training without a leading embedding also requires an explicit
  `Data::norm(...)` choice. The declaration can omit it, but graph compilation
  fails closed instead of choosing an implicit numeric normalization.
* `.epochs` is optional because dense training can be unbounded, but unbounded
  training requires a stop control and a constant post-warmup rate. A cosine
  or exponential schedule without a finite endpoint is rejected.
* `Infer::log` stores items before checking target-free availability. An
  invalid item therefore remains inspectable on the policy but makes
  `evaluate()` fail with `InvalidInferenceConfiguration`.
* `Metric::validate` currently accepts every enum value, so
  `DeclarationErrorKind::InvalidMetric` is part of the error vocabulary but is
  not produced by the current metric validator. Objective-dependent metric
  errors are raised later by training compilation.
* The native path depends on the exact measured profile and current host/GPU
  identities. `recipe probe` establishes the private profile and active receipt;
  running without a matching profile, driver, toolchain, device, or bare-metal
  environment fails in native preparation rather than selecting a fallback.

These gaps are intentionally documented here instead of hidden by removing
the corresponding declaration methods. A facade declaration is an intent
record, not proof that the complete preparation and execution path exists for
every grammar combination.

## End-to-end reference

```text
Rust source or direct Rust calls
        |
        v
source_frontend lowering (only for grammar conveniences)
        |
        v
recipe.data(...) -> Data      recipe.model() -> Model
        |                            |
        +-------- thread-local RECIPE_SEQUENCE --------+
                                                     |
                          recipe.train().run() or recipe.infer().evaluate()
                                                     |
                          declaration validation and sequence take
                                                     |
               bounded data/model preparation and graph compilation
                                                     |
                 measured native preparation and target realization
                                                     |
                       init -> loop -> exit native execution
                                                     |
                   teardown, report, prediction output, artifacts
```

The authoritative implementation details are the included
[`src/api.rs`](../../src/api.rs), the root runtime boundaries in
[`src/training.rs`](../../src/training.rs) and
[`src/inference.rs`](../../src/inference.rs), the data boundary in
[`src/data_prepare.rs`](../../src/data_prepare.rs), the measured native scope
in [`src/native_prepare.rs`](../../src/native_prepare.rs), and the syntax
adapter in [`src/source_frontend.rs`](../../src/source_frontend.rs).
