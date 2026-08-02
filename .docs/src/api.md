# Recipe declaration API

`src/api.rs` is the public, backend-neutral declaration layer. It is included
as `recipe::api` by [`src/facade.rs`](../../src/facade.rs), and the facade
re-exports it at the crate root, so the examples can write `use recipe::*`.
The file records data sources, model topology, training policy, inference
policy, and the typed declaration errors that cross into the preparation and
execution modules. It does not own a device handle, read a file, probe a
machine, allocate a tensor, compile a kernel, or start execution.

The normative list of spellings is [`API.ogdl`](../../API.ogdl). It is the full
specification even when a particular declaration is rejected by a downstream
compiler. This page describes both the declaration values in `src/api.rs` and
the real consumers that determine whether a valid-looking declaration can be
prepared and executed.

## One public declaration flow

The normal Rust surface is a short sequence followed by one terminal:

```rust
recipe.data("examples/data.csv")
    .target("label")
    .exclude(cond!(bad_value < 0))
    .norm(z_score)
    .split(0.8);
recipe.model().layer(32).silu().layer(1).loss(bce);
recipe.train()
    .optimizer(adamw)
    .epochs(10)
    .lr(0.001)
    .cos()
    .save("model.ogdl")
    .run()?;
```

`recipe.data(...)` and `recipe.model()` are the facade methods in
`src/facade.rs`. They create an empty `Data` or `Model` and place a clone in a
thread-local `RECIPE_SEQUENCE`. Every `Data` and `Model` builder method also
updates that sequence, which means the last complete value is the one consumed
by the terminal. `Data` updates replace the remembered model with `None`, while
`Model` updates leave the remembered data in place. This makes a new data
declaration start a new pair rather than accidentally reusing a previous model.

`Train::run` is implemented in `src/training.rs`. It consumes the pair with
`take_recipe_training_sequence`, validates it, and dispatches to one of three
real paths:

1. Bayesian declarations use `compile_bayes_model` and produce an observed
   categorical semantic model without an optimizer loop.
2. A standalone `.knn(neighbors)` declaration uses `compile_knn_model` and
   produces an immutable reference model without an optimizer loop.
3. All other models use `compile_training_package`, measured native
   preparation, the immutable `init -> loop -> exit` lifecycle, teardown, and
   independent model and native-kernel saves.

`Infer::evaluate` follows the same sequence consumption through
`take_recipe_inference_sequence`, then `src/inference.rs` performs target-free
data distillation, semantic model loading, compilation, measured native
execution, teardown, and post-exit prediction reporting. The sequence is
thread-local and is consumed even when a terminal subsequently reports a
declaration error. A caller that wants to retry must declare the data and model
again.

The source frontend in [`src/source_frontend.rs`](../../src/source_frontend.rs)
performs one deterministic pre-rustc lowering pass for syntax that ordinary
Rust cannot express directly. It converts a zero-argument `recipe.data()` to
the unit source form, wraps a multi-argument `.residual(...)` in an array,
maps the literal two-path forms to `__recipe_save_pair` and
`__recipe_resume_pair`, maps `.run(model, data)` to `__recipe_run_with`, and
rewrites the named `.grad(clip: EXPR)` form to `.grad(::recipe::clip(EXPR))`.
It identifies Recipe receivers from the facade and local `Data`, `Model`,
`Train`, and `Infer` bindings. Compiler diagnostics are not parser control
flow, and no retry compilation is performed.

### Public signature matrix

The declaration methods below all return a new value of the same builder type
unless noted. Their inputs are copied into declaration-owned storage; none of
these calls performs runtime work.

| Receiver | Signature family | Output or observable result |
| --- | --- | --- |
| `recipe` | `data(sources)`, `model()`, `train()`, `infer()` | New `Data`, `Model`, `Train`, or `Infer`; the first two also update the thread-local sequence. |
| `Data` | `set(&str)`, `target(IntoTargets)`, `exclude(IntoExclusions)`, `split(f64)`, `norm(DataNormalization)` | Updated declaration; invalid input is deferred in `DeclarationError`. |
| `Data` | `source()`, `sources()`, `targets()`, `exclusions()`, `condition_exclusions()`, `split_fraction()`, `normalization()` | Immutable source/target/policy views. |
| `Model` | `load(&str)`, `layer(IntoLayer)`, `embed(usize)`, `vocab(usize)`, `attn(usize)`, `perc(usize)`, `rnn(usize)`, `gru(usize)`, `lstm(usize)` | Updated topology or checkpoint declaration. |
| `Model` | `bayes(&str, IntoTargets)`, `conv(usize, usize)`, `pool(usize)`, `lgbm(usize)`, `cbst(usize)`, `xgbst(usize)`, `forest(usize)`, `kmeans(usize)`, `knn(usize)`, `residual(branch)` | Updated structured model declaration. |
| `Model` | Activation methods, `norm(LayerNormalization)`, `loss(IntoObjective)`, `grad(Grad)` | Updated ordered operation, objective, or gradient policy. |
| `Model` | `layers()`, `bayes_dependencies()`, `objective()`, `gradient_clip_value()`, `weights_source()` | Immutable topology and policy views. |
| `Train` | `optimizer(Optimizer)`, `epochs(usize)`, `lr(f64)`, `warmup(usize)`, `cos()`, `exp()`, `every(usize)`, `log(IntoLogItems)`, `plot(IntoIterator<Item = LogItem>)` | Updated static training policy. |
| `Train` | `resume(path)`, `save(path)`, plus source-lowered pair forms | Independent artifact declarations; path spelling errors are deferred. |
| `Train` | `run()` and source-lowered `__recipe_run_with(&Model, &Data)` | `TrainingResult<TrainingReport>` after preparation, execution, teardown, and requested saves. |
| `Infer` | `log(IntoLogItems)`, `log_items()` | Updated or observed inference logging policy. |
| `Infer` | `evaluate()` | `InferenceResult<InferenceReport>` after target-free preparation, native execution, teardown, and prediction output. |
| `InferenceDeclaration` | `model()`, `data()`, `policy()` | Immutable views of a resolved, owned declaration. |

The public enum and struct fields that represent user policy are intentionally
read-only after construction. `DeclarationError` fields are public so callers
can branch on `kind` or display `detail`; topology and artifact internals are
exposed through accessor methods instead of mutable state.

## Declaration errors and coercion traits

`DeclarationErrorKind` is `#[non_exhaustive]` and currently contains:

| Kind | Declarations that can produce it |
| --- | --- |
| `EmptyValue` | Empty data source, target, exclusion pattern, or model path. |
| `InvalidExclusion` | Empty condition column, or a non-finite floating condition value. |
| `InvalidSplit` | A non-finite fraction, or a fraction outside `(0, 1)` after both `f64` and `f32` checks. |
| `InvalidLayer` | Zero-width or incomplete blocks, illegal block order, checkpoint/inline conflicts, KNN composition, or grouped routing errors. |
| `InvalidActivation` | An activation after a terminal or non-activation block, or with no preceding block. |
| `InvalidBayes` | Empty or duplicate names, self-parenting, duplicate parents, duplicate children, or a dependency cycle. |
| `InvalidLearningRate` | A non-finite, non-positive, or non-`f32`-representable rate. |
| `InvalidMetric` | Reserved for metric validation. The current `Metric::validate` implementation accepts the private metric enum unconditionally, so this variant is not emitted by the shipped log constants. |
| `InvalidTrainingConfiguration` | Invalid warmup/epoch relationship, log cadence, repeated artifact declaration, invalid gradient clip, or another deferred training builder error. |
| `InvalidInferenceConfiguration` | Missing sequence declarations or a metric that needs targets or training state during inference. |

`DeclarationError` exposes the `kind` and a human-readable `detail`, formats
as `"{kind:?}: {detail}"`, and implements `std::error::Error`.
`DeclarationResult<T>` is its `Result<T, DeclarationError>` alias. Builder
methods are intentionally infallible in their signatures: the first observed
error is stored in a private `deferred` field, subsequent errors do not replace
it, and the relevant `validate` method returns that first error.

The generic conversion traits keep the public methods small without adding
alternate declaration paths:

- `IntoTargets` accepts `&str`, `String`, `[&str; N]`, `&[&str]`, and
  `Vec<String>`. A scalar becomes a one-element ordered vector.
- `IntoConditionValue` accepts signed and unsigned integer widths, `isize`,
  `usize`, `f32`, `f64`, `bool`, `&str`, and `String`. Integers retain signed
  or unsigned identity. Floating values retain exact bits in `FloatBits`; an
  `f32` is first widened to `f64` and then stored as `f64` bits so that the
  declaration remains structurally comparable.
- `IntoExclusions` accepts column names, `Condition`, `Exclusion`, and vectors
  or arrays of those forms. Names become `Exclusion::Column`; predicates remain
  typed `Exclusion::Condition` values.
- `IntoLayer` accepts a `usize` shorthand for a dense block and a complete
  `LayerSpec`.
- `IntoObjective` accepts a built-in `Loss` or `&Model`. A referenced model is
  cloned into `Objective::Reference` and is validated recursively.
- `IntoLogItems` accepts one `LogItem`, a fixed array, `Vec<LogItem>`, or a
  slice.

### Conditions and `cond!`

`ComparisonOperator` has `Equal`, `NotEqual`, `Less`, `LessOrEqual`,
`Greater`, and `GreaterOrEqual`. `Condition` keeps a private column string,
operator, and `ConditionValue`, with `column()`, `operator()`, and `value()`
accessors. `Condition::validate` requires a nonempty column and finite
floating bits.

The exported `cond!` macro accepts an identifier and one of the six comparison
operators:

```rust
cond!(Age < 0)
cond!(status == "cancelled")
cond!(score >= 0.0_f32)
```

The identifier is passed through `stringify!`; it is never evaluated as a Rust
expression. The hidden `__condition` function is the macro's cross-crate
constructor. At preparation time `src/data_prepare.rs` maps operators directly
to `recipe_ingest` predicates. Signed, unsigned, and text values retain their
kind. Boolean literals become their textual `true` or `false` form. Floating
values must narrow to a finite, non-underflowing `f32`; otherwise preparation
returns `DataPreparationError::FloatPredicateOutsideF32`.

## `Data`: immutable ingestion policy

`Data` contains ordered `sources`, ordered `targets`, column-pattern
`exclusions`, typed `condition_exclusions`, an optional `f32` train-fraction
bit image, an optional `DataNormalization`, and a deferred declaration error.
Construction is pure declaration building. All mutators consume and return
`Self`, then call `remember_recipe_data` with a clone.

### Entry points and mutators

The facade's `Recipe::data` accepts `()`, one string, a string reference,
arrays, vectors, and slices whose elements implement `AsRef<str>`. It creates
`Data::empty()` and calls `.set` once for each source. The direct `.set(path)`
method appends exactly one source. An empty path records `EmptyValue` and does
not append it. Calling `.set` repeatedly preserves declaration order.

`.target(targets)` replaces the complete target vector with the converted
ordered list. An empty list or any empty name records `EmptyValue`. It does not
append to an earlier target declaration.

`.exclude(exclusions)` appends each column pattern or condition. An empty input
records `EmptyValue`. Empty column patterns record `EmptyValue`; valid patterns
are retained in order. Conditions are validated before being appended, and an
invalid condition records `InvalidExclusion` without appending it.

`.split(train_fraction)` requires finite `f64` and narrowed `f32` values in
the open interval `(0, 1)`. It stores the narrowed value's bits, so
`split_fraction()` returns an `Option<f32>` and downstream preparation uses the
same representation.

`.norm(z_score)`, `.norm(min_max)`, or `.norm(l2_norm)` overwrites the optional
normalization policy. The three lower-case constants are aliases for
`DataNormalization::ZScore`, `MinMax`, and `L2Norm`.

### Accessors and validation boundary

`source()` returns the first source or `""` when no source exists.
`sources()`, `targets()`, `exclusions()`, and `condition_exclusions()` return
ordered slices. `split_fraction()` decodes the stored `f32` bits, and
`normalization()` returns the optional enum.

`Data::validate()` returns a deferred error first, then requires at least one
source. It deliberately does not require targets or a split, because inference
uses the same declaration type without either. The preparation boundary adds
the mode-specific requirements:

- `prepare_data` validates the declaration, then requires at least one target
  and an explicit split, maps exclusions, distills all sources under finite
  ingest limits, infers semantic vectors, and prepares the train/validation
  partitions. Data normalization is compiled as part of the model calculation,
  not applied by a host-side mutation.
- `distill_data` validates only the source declaration and preserves the source
  order and container semantics for target-free inference.
- `select_target_free_data` applies column and row exclusions to the distilled
  table without targets, fitting, splitting, or normalization. The saved model
  schema is authoritative after selection.

`prepare_data` and `distill_data` surface declaration, ingest, source,
semantic, selection, and preparation failures as typed
`DataPreparationError` values. They never return a partial dataset.

## Model vocabulary and topology

### Operations and shorthand values

`Activation` contains `Linear` (the default), `Cosine`, `Exponential`,
`Logarithm`, `NaturalLogarithm`, `Huber`, `Tangent`, `Relu`, `LeakyRelu`,
`Sigmoid`, `Tanh`, `Selu`, `Gelu`, `Silu`, `Elu`, and `PRelu`.

`Model::log()` is the signed logarithm `sign(x) * ln(abs(x))` on finite,
nonzero values. `Model::ln()` is ordinary `ln(x)` on finite, strictly positive
values. Zero, including negative zero, is rejected at the device fault
boundary. `LayerNormalization` has `LayerNorm` and `BatchNorm`, with the
lower-case `layer_norm` and `batch_norm` aliases. `LayerOperation` stores an
activation or a normalization in exact source order.

Residual branches use `ResidualOperation::Layer { width }` and
`ResidualOperation::Activation(Activation)`. The public `layer(width)` and
`relu()` constructors create the two common branch operations. The private
`IntoResidualBranch` bound accepts one operation or a fixed operation array;
the source frontend turns the literal multi-argument API form into that array.
`ResidualSkip::IdentityOrLinearProjection` records the single declared skip
rule. The training compiler uses identity when widths match and a learned,
weight-only projection otherwise.

`ForestBooster` distinguishes LightGBM, CatBoost, and XGBoost depth. A
`GroupCount` is either `Derived` (pooling) or `Exact(clusters)` (K-means).
`GroupToNeuronConnection` retains the declared group count and following dense
width. `routing(groups)` returns `None` for zero dimensions or an exact-count
mismatch, otherwise it returns:

- `Identity` for equal widths;
- `Expand` when neurons are an integer multiple of groups, using contiguous
  equal ranges;
- `Contract` when groups are an integer multiple of neurons, also contiguous;
- `FullyConnected` for a non-divisible shape.

The connection's `groups()` and `neurons()` accessors expose the unresolved
declaration facts. `GroupCount::Derived` is resolved from the realized pool
shape; `GroupCount::Exact` is checked against the declared K-means cluster
count.

`LayerSpec` is the complete typed topology enum:

| Variant | Stored declaration |
| --- | --- |
| `Dense` | Unit count and ordered operations. `Model::layer(usize)` creates this form. |
| `Perc` | Count of parallel perceptrons and ordered operations. |
| `Rnn`, `Gru`, `Lstm` | Recurrent width and ordered operations. |
| `Convolution` | Filter count, kernel width, and one activation (linear initially). |
| `Pool` | Pool size and optional grouped-to-dense connection. |
| `Lgbm`, `Cbst`, `Xgbst` | One standalone tree family and depth. |
| `Forest` | Exact tree count and an optional nested family/depth. |
| `KMeans` | Cluster count and optional grouped-to-dense connection. |
| `Knn` | Neighbor count and an operations vector that must remain empty for the terminal all-output form. |
| `Residual` | Branch operations, resolved output width, skip rule, and post-merge operations. |
| `Embedding` | Embedding dimensions and optional vocabulary. |
| `Attention` | Head count. |

Each block must be nonzero and structurally complete. Pool and K-means
connections must retain nonzero neurons and the correct `Derived` or `Exact`
group identity. Residual output width is the width of the last branch layer;
an empty branch or a zero layer is invalid. Forest blocks require a nonzero
tree count and a nonzero nested booster depth when validated for execution.

### `Model` construction

`Model` stores the ordered `layers`, ordered `bayes_dependencies`, optional
`Objective`, optional gradient clip bits, optional checkpoint `weights_source`,
and the first deferred error. Every mutator remembers a clone in the facade
sequence.

`load(path)` declares a checkpoint source. The path must be nonempty, a model
accepts one source only, and a checkpoint-backed model cannot also contain a
layer, Bayesian dependency, objective, or gradient policy. The API layer does
not inspect the extension or filesystem. Inference later requires `.ogdl` or
`.gguf`; dense training rejects a loaded source because loading an existing
weight image is not a dense training declaration.

`layer(spec)` accepts a `usize` dense shorthand or a complete `LayerSpec`.
When a dense block immediately follows a `Pool` or `KMeans`, this method fills
that preceding block's grouped-to-neuron connection with the dense width. The
later model validator requires the connection to refer to the immediately
following dense block with exactly that width.

`embed(dimensions)` appends an embedding with no vocabulary. `vocab(vocabulary)`
only succeeds after an embedding and requires a nonzero vocabulary.
`attn(heads)` appends an attention block and requires nonzero heads.
`perc(count)`, `rnn(width)`, `gru(width)`, and `lstm(width)` append their typed
blocks and require nonzero dimensions.

`bayes(child, parents)` appends one dependency in source order. Every child
must be unique; child and parent names must be nonempty; a child cannot parent
itself; each parent's name may occur only once for that child; and the complete
network must remain acyclic. Parent nodes may be declared later or remain
implicit roots. The executable repeated-call instrument treats each child as a
declared data target and each parent as an observed categorical feature. It
does not infer sampling, marginalization, latent nodes, or a custom prior.

`conv(filters, kernel)` and `pool(size)` append structured spatial blocks with
nonzero dimensions. `lgbm(depth)`, `cbst(depth)`, and `xgbst(depth)` set a
pending family on a preceding `Forest` with no booster; otherwise they append a
standalone one-tree block. `forest(trees)` appends an unselected forest and
requires a nonzero tree count. A downstream compile therefore requires one
nested family for a forest.

`kmeans(clusters)` appends a deterministic distance-reduction block.
`knn(neighbors)` appends the one public KNN form. KNN is validated as one
standalone terminal block, not as a composable layer and not as a prediction
plus output-count pair.

`residual(branch)` resolves the branch's final layer width and stores the
identity-or-projection skip rule. A branch without a nonzero layer defers
`InvalidLayer`.

The activation methods `relu`, `leak`, `sigmoid`, `tanh`, `selu`, `gelu`,
`silu`, `elu`, `prelu`, `cos`, `exp`, `log`, `ln`, `huber`, and `tan` append an
ordered operation to a dense, perceptron, recurrent, or residual block. On a
convolution they replace that block's single activation. They defer
`InvalidActivation` when there is no preceding compatible block, after a
terminal KNN block, or after pool, tree, forest, K-means, embedding, or
attention blocks. PReLU occurrences are separate learned scalar parameters;
the training compiler initializes each at its own ordered occurrence.

`loss(objective)` stores a built-in `Loss` or a cloned model reference.
`grad(clip(maximum_norm))` stores an optional global clip norm. `clip` accepts
only a finite, positive value representable as `f32`; an invalid value is
remembered as raw bits so `Model::grad` can return a typed
`InvalidTrainingConfiguration`. Omitting `.grad(...)` means no clipping policy.
`norm(layer_norm)` or `norm(batch_norm)` appends normalization only to a dense,
perceptron, recurrent, or residual block. KNN, convolution, pool, trees,
forest, K-means, embedding, attention, and an empty model reject that call as
`InvalidLayer`.

### Model validation and accessors

`Model::validate()` returns a deferred error first. It then rejects checkpoint
and inline-definition conflicts, empty models, KNN blocks that are not the one
standalone first block or that contain operations, grouped connections without
their exact following dense layer, and invalid Bayesian networks. A referenced
objective model is validated recursively.

The public accessors are `layers()`, `bayes_dependencies()`, `objective()`,
`gradient_clip_value()`, and `weights_source()`. They expose immutable slices or
decoded values, never mutable runtime state.

## Losses, objectives, and optimizers

`Loss` has `MeanSquaredError`, `MeanAbsoluteError`, `Huber`,
`BinaryCrossEntropy`, `CrossEntropy`, and `Focal`. The constants `mse`, `mae`,
`huber`, `bce`, `ce`, and `focal` are the public spellings. The loss comments
define MSE, MAE, unit-delta Huber, logits-based BCE, numerically stable
log-softmax cross entropy, and focal loss with Recipe's fixed historical
`alpha = 0.25` and `gamma = 2.0`.

`Objective::Builtin` holds one of those losses. `Objective::Reference` holds a
cloned model declaration, but `src/training.rs::require_supported_model`
currently rejects referenced objectives for dense lowering rather than silently
choosing a fallback loss.

`Optimizer` currently has only `AdamW`, exposed as `adamw`. The optimizer
choice is declaration state. Dense training requires exactly `Some(AdamW)` at
the execution boundary, while KNN and observed Bayesian preparation reject any
optimizer, learning rate, schedule, warmup, epoch bound, or iterative metric
because those paths have no optimizer loop.

## Logging values and metrics

`Metric` is an internal enum with `Loss`, `Accuracy`, `R2`, `AuRoc`, `AuPrc`,
`Brier`, `CalibrationError`, `Epoch`, `LearningRate`, `Time`, and `Device`.
`LogItem` wraps one metric. The public constants are the canonical names
`LossMetric`, `Accuracy`, `R2`, `AuRoc`, `AuPrc`, `Brier`, `CalibrationError`,
`Epoch`, `Lr`, `Time`, and `Device`, plus aliases `Loss`, `Acc`, `loss`,
`accuracy`, `epoch`, `lr`, `time`, `r2`, and `device`.

`Train::log` accepts one item or an ordered collection. It appends valid items
to the flattened `log` list and creates one cadence declaration per call, with
default interval one. `Train::every(interval)` changes only the most recent
`.log(...)` declaration and requires a nonzero interval and a preceding log
call. `Train::plot` appends valid items to a separate plot list; plotting does
not create a log cadence.

During dense training, `src/training.rs` derives validation metric families
from the selected objective. Binary metrics and calibration require BCE or
focal loss, `Accuracy` for categorical cross entropy uses multiclass validation,
and `R2` requires MSE, MAE, or Huber regression. One run cannot request more
than one validation family. `Epoch`, `Time`, and `Device` are host/lifecycle
bindings; `Time` and realized `Device` are reported after native teardown and
do not add a loop transfer. KNN and Bayesian reference preparation have no
iterative metrics.

## `Train`: static training policy and artifacts

`Train` stores optional epoch and warmup bounds, an `f32` learning rate bit
image, an optional `LearningRateSchedule`, an optional `Optimizer`, flattened
log and plot items, cadence declarations, independent resume and save artifact
slots, one-declaration guards, and a deferred error. `Train::new()` is crate
private and is returned by `Recipe::train()`.

### Policy builders

- `.epochs(count)` requires a nonzero count. Omitting it means an unbounded
  horizon in the dense compiler; execution waits for the graceful SIGINT stop
  boundary rather than substituting a numeric sentinel.
- `.lr(rate)` requires finite, positive `f64` and `f32` values. It stores the
  narrowed bits and selects `LinearDecay`. A later `.cos()` or `.exp()` changes
  only the schedule context. If no rate is declared, the dense compiler uses
  the Recipe AdamW default rate, but still requires an explicit schedule and
  optimizer declaration.
- `.warmup(count)` requires nonzero epochs. `Train::validate` additionally
  requires warmup to be strictly less than a declared total epoch bound.
- `.cos()` and `.exp()` select cosine or exponential decay after an `lr` call.
  They are also valid policy methods without an earlier `lr`, but dense
  compilation rejects an unbounded cosine or exponential schedule because no
  endpoint exists. Linear decay on an unbounded horizon is treated as a
  constant rate after optional warmup.
- `.optimizer(adamw)` stores the optimizer enum. The API currently has no other
  optimizer.
- `.log(...)`, `.every(...)`, and `.plot(...)` behave as described in the
  metrics section.

### Resume and save declarations

The artifact contract is extension-driven and independent for model and
native-kernel paths:

| Form | Declaration result |
| --- | --- |
| `.resume("model.ogdl")` | Semantic model source; existence is checked at run time. |
| `.resume("model.ogdl", "kernel.cubin")` | Semantic source plus CUDA native source. |
| `.resume("model.ogdl", "kernel.hsaco")` | Semantic source plus HSA native source. |
| `.save("model.ogdl")` | Model export only. |
| `.save("kernel.cubin")` or `.save("kernel.hsaco")` | Native-kernel export only. |
| `.save("model.ogdl", "kernel.cubin")` or `.save("model.ogdl", "kernel.hsaco")` | Both exports. |

The one-path methods accept exactly one declaration. A second call defers an
`InvalidTrainingConfiguration` error and does not add a second artifact. An
empty path or unknown extension is rejected. Resume's first path must be
`.ogdl`; its optional second path must be `.cubin` or `.hsaco`. A kernel-only
resume is not representable by either the public one-path method or the pair
lowering.

The API layer validates spelling and extensions but does not touch the
filesystem. During dense training, an existing resume `.ogdl` is loaded and
applied. A missing model path is the normal fresh-run path, not an error. If a
kernel path is supplied and exists, `src/training.rs` authenticates its digest,
program identity, topology, discovery, target, and toolchain against the
semantic checkpoint. A missing native path causes current-system recompilation;
an existing path whose bytes, digest, or identity do not match is an explicit
incompatible-resume error. Model and kernel saves are independent at exit. With no `.save`
declaration, a successful run writes no user-owned artifact.

The semantic `.ogdl` is a versioned, validated document rather than a dump of
the runtime graph. It retains the ordered vector schema and target identities,
feature lowering spans, fitted normalization tensors, task and loss semantics,
declared topology, parameter tensors, AdamW first and second moments, and the
resolved geometry or state needed by structured blocks. Semantic versions also
retain measured native realization identities and kernel digests, but not native
bytes. Versioned decoders accept the shipped flat and structured historical
forms and reject unknown versions, unknown fields, duplicate fields, malformed
values, inconsistent shapes, and decode-limit violations with path-addressed
checkpoint errors. Resume therefore compares semantic declarations and saved
state instead of treating an arbitrary file with an `.ogdl` suffix as valid.

KNN and Bayesian paths save only their semantic `.ogdl` model. They return a
typed unsupported error if a native training-kernel destination or native
resume source is requested, because those paths are reference preparation, not
optimizer training.

### Validation and accessors

`Train::validate()` returns the first deferred error, checks warmup versus a
finite epoch bound, and rechecks every log and plot item. Public policy
accessors are `epoch_bound`, `learning_rate`, `warmup_epoch_bound`,
`learning_rate_schedule`, `optimizer_spec`, `log_items`, `plot_items`, and
`resume_source`. The crate-facing accessors
`resume_kernel_source`, `save_model_destination`, `save_kernel_destination`,
and `metric_log_interval` are consumed by the training and artifact layers.

The terminal `Train::run` is not defined in `src/api.rs`, but it is an inherent
method on the same public type in `src/training.rs`. Its typed failures include
declaration, data preparation, graph compilation, checkpoint serialization,
resume decoding, native-kernel source, measured native preparation, unsupported
declarations, and runtime lifecycle errors. Dense reports are created only
after native teardown. They retain the completed journal, native images,
validation status, metrics, and graceful-stop evidence. KNN and Bayesian
reports intentionally have no optimizer run, execution bundle, or native
kernel evidence.

`TrainingReport::kind` identifies `Dense`, `Knn`, or `Bayes`. Dense reports
provide optional `run` and `bundle` identities, `external_outputs`, final
metrics, the completed `journal`, realized native kernels, native execution
evidence, full-partition training evidence, validation status, and the
`gracefully_stopped` flag. KNN reports expose their immutable reference model
through `knn_model`; Bayesian reports expose their observed conditional model
through `bayes_model`. The specialized reports intentionally return no dense
execution journal or native training-kernel set.

## `Infer` and `InferenceDeclaration`

`Infer` stores an ordered log list and a deferred error. `Infer::new()` is crate
private and is returned by `Recipe::infer()`. `Infer::log` accepts the same
`IntoLogItems` forms as training, retains valid items, and records the first
invalid inference configuration. Target-dependent metrics (`Loss`, `Accuracy`,
`R2`, `AuRoc`, `AuPrc`, `Brier`, and `CalibrationError`) are rejected because
target-free inference has no target values. `Epoch` and `Lr` are rejected
because inference has no training state. Only `Time` and `Device` are valid
inference log items.

`resolve_declaration()` first takes the thread-local data/model pair, validates
the inference policy, then validates `Data` and `Model`, and returns an
`InferenceDeclaration` containing owned model and data plus a cloned policy.
`InferenceDeclaration::model()`, `data()`, and `policy()` expose immutable
references. `data()` is currently always `Some` for a resolved public
declaration; the optional field lets the execution boundary fail closed if an
internal caller constructs an incomplete declaration.

`Infer::evaluate()` resolves that declaration and calls
`evaluate_inference_declaration` in `src/inference.rs`. The real compiler then:

1. Requires no data targets, no split, and no data normalization. The semantic
   model owns target interpretation and normalization.
2. Requires `Model::load(MODEL_PATH)`. `.ogdl` loads a semantic Recipe model;
   `.gguf` loads the bounded supported GGUF llama artifact. Other extensions,
   missing extensions, or inline model topology are unsupported at this
   boundary.
3. Distills and selects target-free rows, applying declared column and row
   exclusions before the saved model schema.
4. Compiles the loaded dense, KNN, Bayesian, or GGUF llama inference graph.
5. Uses the current measured profile, hardware-derived host tuning, native
   artifact preparation, and the one-iteration `init -> loop -> exit`
   lifecycle. Native resources are torn down before reporting.

There is one deliberate grammar-to-facade distinction to keep visible:
`API.ogdl` lists `.infer().load("model.ogdl" OR "model.gguf")`, but the current
Rust `Infer` type has no `load` method and the source frontend does not lower
that call. The executable spelling is
`recipe.model().load(MODEL_PATH); recipe.infer().evaluate()`. The model load is
separate because it supplies the semantic checkpoint or GGUF source that the
inference resolver consumes. The normative grammar entry remains in
`API.ogdl`; it is specified but not currently implemented as a direct
`Infer::load` builder call.

The returned `InferenceReport` keeps the completed execution, elapsed time,
realized devices, prediction bytes, and the loaded semantic artifact. Reporting
prints `Time` and `Device` only when requested, then emits one prediction record
per selected source row. Dense predictions distinguish binary probabilities,
regression values, multiclass probabilities and decoded class labels,
multi-target binary probabilities, joint-target probabilities, and ordered
multi-target regression values. KNN emits each declared output in target order,
with numeric means or decoded discrete modes. Bayesian output ranges follow
conditional declaration order. GGUF llama emits the highest-logit token and
logit per sequence position. Reporting occurs after teardown and is not a
substitute for the returned typed result.

The report accessors are family-aware: `kind`, `run`, `bundle`, `journal`,
`native_kernels`, `native_evidence`, `elapsed`, `devices`, and `values` expose
the common lifecycle result; `prediction` is `None` for all-output KNN while
`knn_predictions` is `Some` in that case. Dense reports expose saved multiclass
decoding, Bayesian reports expose output count, names, class counts, packed
ranges, and dictionary decoding, and KNN reports expose discrete-label
decoding. These accessors do not refit schemas or infer labels from current
query rows.

Inference failures are `InferenceError::Declaration`, `Data`, `Model`,
`Compile`, `Native`, `Execute`, `Runtime`, or `Unsupported`. The lower-level
`InferenceResult<T>` alias is `Result<T, InferenceError>`.

## Specified surface versus executable boundary

The declaration facade intentionally remains broader than any one execution
case. The following distinction is the current source-backed status:

| Surface | What `src/api.rs` records | What the current consumers execute or reject |
| --- | --- | --- |
| Data sources, targets, exclusions, predicates, normalization, split | All API.ogdl forms, ordered and immutable | `data_prepare` implements bounded file, directory, archive, semantic, selection, and train/validation preparation. Inference intentionally rejects targets, split, and redeclared normalization. |
| Dense and perceptron blocks | Ordered widths and operations | Dense training maps `Layer` and `Perc` into Recipe-owned full-partition graphs. A built-in objective, AdamW, an explicit decay, and valid data normalization are required, except a leading embedding deliberately uses identity normalization for exact token IDs. |
| Structured blocks | Convolution, pool, K-means, residual, trees/forests, embedding, attention, RNN, GRU, and LSTM variants | `map_dense_block` routes each variant to the typed training compiler. It rejects incomplete embedding vocabularies, unselected forests, chained operations on the first RNN/GRU/LSTM cases, invalid dimensions, and any downstream shape or task mismatch. |
| KNN | One standalone `.knn(neighbors)` block | Dedicated reference preparation and target-free native inference. Objectives, optimizer policy, iterative metrics, operations, and native training artifacts are rejected. Existing resume appends compatible references; a missing model starts fresh. |
| Bayesian declarations | Repeated acyclic child/parent dependencies | Dedicated observed categorical preparation and native Laplace posterior inference. Generic layers, objectives, normalization, optimizer policy, iterative metrics, and native training artifacts are rejected. |
| Checkpoint model loading | One nonempty source with no inline declarations | Inference accepts `.ogdl` and supported `.gguf`; dense training rejects loaded weights as a training model. Resume always starts from semantic `.ogdl`, optionally authenticating a supplied native image. |
| Training metrics | All API.ogdl log and plot names, ordered and cadence-aware | Dense training derives objective-compatible validation bindings and host lifecycle metrics. KNN/Bayes have no iterative metrics. Inference accepts only `Time` and `Device`. |
| Save and resume syntax | Literal one-path and two-path declarations | Source lowering preserves the literal forms. Runtime writes at most the independently requested `.ogdl` and `.cubin` or `.hsaco` artifacts, with no implicit default path or sidecar. |
| `.epochs` | Optional finite nonzero bound | Dense compilation preserves a finite or unbounded horizon. Unbounded runs require the public SIGINT stop source; cosine/exponential schedules and post-training calibration require a finite endpoint. |

This table is an implementation boundary, not a reduction of the
specification. A declaration that is represented by `src/api.rs` but rejected
by `training.rs` or `inference.rs` remains part of the public contract and
should be documented as unsupported for that concrete execution path, not
removed from `API.ogdl`.

## Source map

- [`src/api.rs`](../../src/api.rs): declaration types, builders, coercion
  traits, validation, and public constants.
- [`src/facade.rs`](../../src/facade.rs): root `recipe` value, thread-local
  data/model sequence, and facade constructors.
- [`src/data_prepare.rs`](../../src/data_prepare.rs): conversion of `Data`
  into bounded prepared or target-free tables.
- [`src/training.rs`](../../src/training.rs): training compilation, KNN and
  Bayesian specialization, native lifecycle, resume, and artifact saves.
- [`src/inference.rs`](../../src/inference.rs): target-free compilation,
  native execution, report construction, and prediction output.
- [`src/source_frontend.rs`](../../src/source_frontend.rs): deterministic
  lowering for named gradient, pair artifact, multi-argument residual, and
  explicit data/model run syntax.
- [`API.ogdl`](../../API.ogdl): complete normative public declaration grammar.
- [`system-contract.md`](../../system-contract.md): executable semantics and
  acceptance contracts for the declaration families.
