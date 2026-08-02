# `examples/cookbook.rs`

## Purpose and source contract

`examples/cookbook.rs` is the single public-API example binary. It is a
sequential executable cookbook, not a library of callable example functions.
The current source has one `main` function (lines 5-444) and one Cargo example
test (lines 446-449). `main` returns
`Result<(), Box<dyn std::error::Error>>`; every training or inference call uses
`?`, so the first error ends the process and later cookbook declarations are not
attempted. The test calls `main()` directly and therefore exercises the same
21-workflow sequence rather than a separate test-only path.

The example is registered as `[[example]] name = "cookbook" test = true` in
`Cargo.toml`. Run it from the repository root, because every dataset and model
path is relative to the current working directory:

```text
cargo run --bin recipe -- probe
cargo run --example cookbook
```

The first command creates or refreshes the exact measured native profile used by
the second command. The example itself does not probe hardware. Training and
inference require a current profile, a matching local GPU discovery, the
corresponding driver/toolchain, and all checked-in dataset or model bytes.

`cargo run --example cookbook` invokes the example binary directly. It does not
go through the CLI source-rewrite path used by `recipe run FILE.rs`; the only
CLI interaction in the documented invocation is the preceding `recipe probe`.

The source imports `recipe::*`. That brings the public facade value `recipe`,
the immutable `Data`, `Model`, `Train`, and `Infer` builders, normalization and
loss constants, model operation constants (`layer`, `relu`), and logging items
such as `Loss`, `Accuracy`, `R2`, `Brier`, `Epoch`, `Lr`, `Time`, and `Device`.

## Declaration and execution state

The public facade is intentionally declarative. Builder calls record paths,
targets, model blocks, and policy; they do not read files, inspect the GPU,
compile kernels, allocate memory, or execute a loop.

The state transition is:

```text
recipe.data(...) or recipe.model()
    -> thread-local pending declaration
    -> .run() or .evaluate() takes and clears data/model
    -> declaration validation
    -> bounded data/model preparation and graph compilation
    -> measured native preparation, realization, load, and warm-up
    -> one data-image admission per device
    -> training loop or target-free inference loop
    -> output egress and report publication
    -> native teardown
    -> optional semantic-OGDL artifact write
```

The KNN and observed Bayesian branches stop at semantic reference preparation
before native training. They can save a semantic model and later execute native
inference, but they do not claim an optimizer loop or a native training kernel.

`recipe.data(sources)` starts a new thread-local sequence and clears any pending
model. `recipe.model()` starts a new pending model. Every mutating builder method
remembers its returned clone in that sequence, which is why the example can
write a declaration as separate statements and then call `recipe.train()` or
`recipe.infer()` in a later statement. A `.run()` or `.evaluate()` takes both
pending values, so a failed or incomplete sequence must be redeclared before a
retry. There is no fallback sequence or hidden global model.

`Train::run` resolves the immediately preceding data and model, dispatches
Bayesian declarations to `compile_bayes_model`, standalone `.knn(...)` to
`compile_knn_model`, and all other models to the dense training compiler. Dense
training compiles the graph, applies an existing `.resume(...)` checkpoint when
the path exists, prepares the current measured native system, executes, builds a
completed report, and only then writes requested artifacts. KNN and Bayesian
training prepare their semantic artifacts and write a requested `.ogdl` without
native training execution.

`Infer::evaluate` resolves and clears the same pending data/model pair. It
requires target-free data, loads the model source (`.ogdl` semantic model or
`.gguf` dense-F32 Llama model), distills and selects rows, compiles the matching
inference family, executes the measured native loop, and writes the report after
native teardown. The report is discarded by this example after `?`; the
`Time` and `Device` log declarations select the printed observability fields.

## Public declaration vocabulary used here

### Data

- `recipe.data("path")` records one source. Arrays are accepted, preserving
  order. The implementation converts each source to a `String` and calls
  `Data::set`, which appends to the source list. Consequently
  `recipe.data([WINE_PATH, WHITE_WINE_PATH]).set(WINE_PATH)` deliberately records
  the ordered list `[red, white, red]`; sources are not deduplicated by the
  facade.
- `.target("name")` records one target. An array records an explicitly ordered
  target vector, which is significant for multiclass and multi-output models.
- `.exclude("name")` or `.exclude(["a", "b"])` records columns removed from a
  target-free inference table. It does not select training targets.
- `.norm(z_score)`, `.norm(min_max)`, and `.norm(l2_norm)` record numeric data
  normalization. Embedding input intentionally omits normalization because the
  embedding compiler consumes exact integer token IDs.
- `.split(fraction)` records the training fraction. The declaration accepts only
  finite values strictly between zero and one. Training preparation requires an
  explicit target and split; target-free inference rejects both target and
  split declarations and uses the saved model schema instead.

### Model

`recipe.model()` creates a backend-neutral `Model`. The facade stores model
blocks as `LayerSpec` values and attaches operations to the most recent block.
The cookbook uses these concrete forms:

| Public calls | Facade representation and compiler family |
|---|---|
| `.layer(width)` | `LayerSpec::Dense`, mapped to a dense `Layer` block. |
| `.perc(count)` | `LayerSpec::Perc`, mapped to parallel perceptrons. |
| `.rnn(width)`, `.gru(width)`, `.lstm(width)` | Recurrent `LayerSpec` blocks with the declared hidden width. |
| `.forest(trees).lgbm(depth)` | A `Forest` block with a LightGBM-family booster and depth. The following `.loss(...)` remains the objective. |
| `.residual([layer(width), relu()])` | A residual branch whose output width is the last branch layer. The outer activation is attached to that residual block. |
| `.kmeans(clusters).layer(width)` | A K-means grouped block followed immediately by a dense destination. The facade retains an exact group-to-neuron connection. |
| `.knn(neighbors)` | One standalone terminal all-output KNN block. It cannot compose with another block. |
| `.embed(dimensions).vocab(vocabulary).attn(heads)` | An embedding block with an immediate vocabulary declaration followed by attention. The later dense blocks consume the embedding/attention output. |
| `.conv(filters, kernel)` and `.pool(size)` | Convolution blocks with attached activation followed by pooling. A following dense layer receives the derived pool groups. |
| `.bayes(child, parents)` | Ordered Bayesian conditional dependencies. Repeated calls form one observed categorical Bayesian artifact. |
| `.load(path)` | A checkpoint-backed model. `.ogdl` is a semantic Recipe model; `.gguf` selects the checked-in dense-F32 Llama inference instrument. A loaded model cannot also contain inline blocks or a loss. |

The model objective is supplied by `.loss(...)`. The cookbook uses `mse`
(mean squared error), `bce` (binary cross entropy from logits), `ce`
(categorical cross entropy from logits), `focal` (binary focal loss), and
`huber`. `.exp()`, `.log()`, and `.ln()` in the logarithm workflow are model
activations, not learning-rate schedules: `.log()` is the signed
`sign(x) * ln(abs(x))` operation on nonzero values, while `.ln()` requires
strictly positive values.

### Training policy

`recipe.train()` creates a static `Train` policy. The dense workflows select the
Recipe-owned `adamw` optimizer, a positive `.lr(...)`, and a finite `.epochs(...)`.
Calling `.lr(...)` initially selects linear decay; `.cos()` or `.exp()` replaces
that schedule. `.warmup(n)` is valid only when `n` is nonzero and less than the
total epoch bound. `.log(...)` records training metrics. `.every(n)` changes the
cadence of the immediately preceding `.log(...)` declaration, and therefore
follows `.log(...)` in the observability workflow. `.plot(...)` requests bounded
terminal plotting fields.

`.save("file.ogdl")` selects a semantic model artifact. The extension is
authoritative; every save in this source uses `.ogdl`, and no workflow requests a
native `.cubin` or `.hsaco` output. `.resume("file.ogdl")` selects an existing
semantic checkpoint if it exists and otherwise starts from fresh state. No
workflow uses the optional native-kernel second argument.

### Inference policy

`recipe.infer()` creates a target-free `Infer` policy. Only `Time` and `Device`
are requested in cookbook inference logs. Training metrics such as `Loss`,
`Accuracy`, `R2`, `Brier`, `Epoch`, and `Lr` are rejected for target-free
inference because no target values or optimizer state are available. The
inference data declaration therefore uses `.exclude(...)` for saved semantic
models, or a bare token source for GGUF, and never calls `.target(...)`,
`.split(...)`, or `.norm(...)`.

## Source call graph and lifecycle boundary

The example reaches the implementation through these real production callers:

1. `recipe.data` and `recipe.model` are `Recipe` methods in `src/facade.rs`.
   They construct immutable `Data` and `Model` values and update the
   thread-local `RECIPE_SEQUENCE`.
2. `Data` and `Model` methods in `src/api.rs` retain declaration state and defer
   the first declaration error. `Train` and `Infer` methods in the same facade
   validate their static policies.
3. `Train::run` in `src/training.rs` takes the pending pair. Its
   `try_run_with` dispatches `.bayes(...)`, `.knn(...)`, or dense blocks, then
   saves any declared artifact after the selected branch succeeds.
4. Dense preparation calls `prepare_data`, maps `LayerSpec` values to
   `recipe_training::DenseBlock` values, builds a `DenseTrainingConfig`, selects
   the validation family implied by the requested metrics, and calls the
   corresponding `compile_dense_training...` entry point in `training/src/compile.rs`.
5. `execute_current_training_native` reopens the exact profile through
   `with_current_native_preparation` in `src/native_prepare.rs`, derives runtime
   tuning, creates the production candidate realizer and executor driver, and
   calls `prepare_and_execute_local_training_controlled` in
   `training/src/execute.rs`.
6. `Infer::evaluate` resolves the pending pair in `src/api.rs`, while
   `evaluate_inference_declaration` and `compile_inference_package` in
   `src/inference.rs` select a semantic Dense, KNN, Bayes, or GGUF model and
   compile its target-free graph.
7. Native inference calls the corresponding `prepare_and_execute_local_inference`
   or `prepare_and_execute_local_knn_inference` path. The executor accepts one
   finalized `init` image per device, runs exactly one inference loop iteration,
   executes `exit`, validates the output image, and only then returns a typed
   report for publication.

`with_current_native_preparation` obtains the active native receipt and its
identity-named measured profile from the private Recipe state root. It reopens
the pinned host memory origin, backend libraries, LLVM tools, linker/assembler,
and probe configuration. If the receipt, profile, host origin, tool digest, or
profile identity no longer matches the current machine, preparation fails and
the caller must run `recipe probe` again. There is no newest-profile or ordinal
cache fallback.

Training preparation maps the complete prepared training partition into one
logical matrix per epoch. The compiler's contract is one logical optimizer update
per epoch; a backend may tile physical work but cannot change that end state.
The runtime rejects external transfers in the loop. Training inputs are packed
into one finalized init image per device, while model outputs are collected only
from finalized exit transfers. User metrics are read back through their metric
slots, and live notifications cannot change calculation progress or final metric
retention.

Inference is bounded to one loop iteration. Inputs are admitted only during
`init`, external transfers are forbidden in the loop, and one typed prediction
image is collected after `exit`. The final report exposes raw F32 values for
Dense, Bayes, and GGUF predictions; KNN uses independently typed predictions
because its declared outputs can mix discrete class codes and numeric means.

## Workflow manifest

The following entries follow the current source order. A source span is a
navigation aid into `examples/cookbook.rs`; the declaration text is the
authoritative behavior. `report: discarded` means the call returns a typed
report and the example propagates errors with `?` but does not inspect the
report.

### 1. Wine-quality regression, lines 6-27

- **Intent:** Train a batch-normalized dense regression model for UCI red-wine
  quality.
- **Input:** `examples/datasets/uci-winequality-semicolon/winequality-red.csv`;
  target `quality`; `z_score`; train split `0.8`.
- **Model:** `layer(64) -> gelu -> batch_norm -> layer(16) -> silu ->
  layer(1)` with `mse`.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.0002)`, exponential schedule via
  `.exp()`, and `log(Loss)`.
- **Output:** Saves `cookbook-wine-quality-regression.ogdl`. No inference is
  declared; `TrainingReport` is discarded after successful save.

### 2. Tree forest, lines 29-47

- **Intent:** Train a two-tree LightGBM-family supervised forest and exercise
  semantic-model inference.
- **Input:** `examples/datasets/cookbook/binary.csv`; target `target`;
  `z_score`; train split `0.75`.
- **Model:** `forest(2).lgbm(2)` with `bce`.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log([Loss, Accuracy])`.
- **Output:** Saves `cookbook-tree-forest.ogdl`. Then a new target-free data
  declaration excludes `target`, loads that model, and evaluates inference with
  `log([Time, Device])`; the report is discarded.

### 3. Save and resume dense model, lines 49-74

- **Intent:** Demonstrate two independent dense training declarations connected
  by a semantic checkpoint.
- **Input:** Both runs use `binary.csv`, target `target`, `z_score`, and split
  `0.75`.
- **Model:** Both runs declare `layer(8) -> silu -> layer(1)` with `bce`.
- **Policy:** Both use `adamw`, `epochs(1)`, and `lr(0.0002)`. The first saves
  `cookbook-save-resume-first.ogdl`; the second resumes that path and saves
  `cookbook-save-resume-second.ogdl`.
- **Output:** Two semantic model files, with no inference call in this section.
  If the first resume path is absent at the second run, the existence-conditional
  resume contract starts the second run from fresh state.

### 4. Vanilla RNN train, resume, and inference, lines 76-107

- **Intent:** Train and resume a fixed-width vanilla recurrent model, then run
  target-free inference.
- **Input:** `binary.csv`, target `target`, `z_score`, split `0.75` for both
  training declarations; the inference declaration excludes `target`.
- **Model:** `rnn(8).layer(1)` with `bce` for both training and resume.
- **Sequence semantics:** Each numeric row is one fixed scalar sequence whose
  feature columns are consumed in order; each row starts with an all-zero
  hidden state.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log(Loss)` for both runs. Saves
  `cookbook-rnn-first.ogdl`, then resumes it and saves
  `cookbook-rnn-resumed.ogdl`.
- **Output:** The final load/evaluate selects the Dense semantic inference
  family and logs no training metrics; the returned report is discarded.

### 5. Residual branch, lines 109-125

- **Intent:** Exercise a residual branch whose declared width differs from the
  final scalar output and therefore uses the facade's identity-or-linear
  projection skip rule.
- **Input:** `binary.csv`, target `target`, `min_max`, split `0.75`.
- **Model:** `residual([layer(8), relu()]) -> silu -> layer(1)` with `bce`.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log([Loss, Brier])`.
- **Output:** Saves `cookbook-residual.ogdl`; no inference declaration follows.

### 6. Airfoil observability, lines 127-143

- **Intent:** Train scalar regression while exercising warm-up, metric cadence,
  host/device logging, and bounded plotting.
- **Input:** `examples/datasets/uci-airfoil/airfoil_self_noise.dat`; target
  `col6`; `z_score`; split `0.8`.
- **Model:** `layer(16) -> tanh -> layer(1)` with `mse`.
- **Policy:** `adamw`, `epochs(2)`, `lr(0.0002)`, `warmup(1)`, cosine schedule,
  `log([Loss, R2, Epoch, Lr, Time, Device])`, `.every(1)`, and
  `plot([Loss, R2])`.
- **Output:** No `.save(...)` call, so successful execution exports no
  user-owned model or kernel artifact. The report is discarded.

### 7. Sonar multiclass, lines 145-159

- **Intent:** Train a three-class categorical classifier from the UCI Sonar
  categorical target.
- **Input:** `examples/datasets/uci-sonar/sonar.all-data`; target `col61`;
  `z_score`; split `0.8`.
- **Model:** `layer(24) -> gelu -> layer(3)` with categorical `ce`.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.0003)`, cosine schedule, and
  `log([Loss, Accuracy])`.
- **Output:** Saves `cookbook-multiclass.ogdl`; no inference call follows.

### 8. Ordered multi-target classification, lines 161-175

- **Intent:** Train one joint categorical objective over an explicitly ordered
  three-column target vector.
- **Input:** `examples/datasets/cookbook/multi_target.csv`; targets, in order,
  `winner_model_b`, `winner_model_a`, `winner_tie`; `z_score`; split `0.8`.
- **Model:** `layer(12) -> gelu -> layer(3)` with `ce`.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.0003)`, cosine schedule, and
  `log([Loss, Accuracy])`.
- **Output:** Saves `cookbook-multi-target.ogdl`; target order remains part of
  the semantic model contract.

### 9. LSTM train, resume, and inference, lines 177-208

- **Intent:** Train and resume a fixed-width long short-term-memory model, then
  evaluate its saved semantic model without targets.
- **Input:** `binary.csv`, target `target`, `z_score`, split `0.75` for both
  training declarations; inference excludes `target`.
- **Model:** `lstm(8).layer(1)` with `bce` in both runs.
- **Sequence semantics:** Feature columns are one scalar sequence per row, and
  each row starts with all-zero hidden and cell state.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log(Loss)`; saves `cookbook-lstm-first.ogdl`, resumes it, then saves
  `cookbook-lstm-resumed.ogdl`.
- **Output:** The final `load` plus `infer().evaluate()` returns a Dense
  inference report, which this example discards.

### 10. Signed and natural logarithms, lines 210-228

- **Intent:** Exercise distinct signed logarithm and ordinary natural-log
  activation declarations in one dense model.
- **Input:** `binary.csv`, target `target`, `z_score`, split `0.75`.
- **Model:** `layer(8) -> exp -> ln -> layer(4) -> exp -> log -> layer(1)`
  with `bce`. The two `.exp()` calls are activations; `.ln()` and `.log()` have
  different input domains.
- **Policy:** `adamw`, `epochs(1)`, and `lr(0.0002)`. No explicit `.cos()` or
  `.exp()` schedule call follows `.lr`, so the policy retains the linear decay
  selected by `.lr`; it logs `Loss`.
- **Output:** No `.save(...)` call, so no user-owned artifact is exported. A
  device fault for invalid logarithm inputs is returned through the training
  runtime error boundary.

### 11. All-output KNN, lines 230-242

- **Intent:** Prepare a standalone all-output K-nearest-neighbor reference model
  with one discrete and one numeric output, then infer both output types.
- **Input:** `examples/datasets/cookbook/knn.csv`; ordered targets
  `class_target`, `numeric_target`; `z_score`; split `0.75`.
- **Model:** `knn(3)`. KNN is terminal and has no loss, optimizer, epoch, or
  learning-rate policy.
- **Policy:** `recipe.train().save("cookbook-knn.ogdl")` only. This selects the
  KNN semantic preparation branch, not native optimizer training.
- **Output:** Saves `cookbook-knn.ogdl`. Target-free inference excludes both
  target columns, loads the model, logs `Time` and `Device`, and returns typed
  KNN predictions in declared-target order; the report is discarded.

### 12. K-means followed by dense layers, lines 244-256

- **Intent:** Use a four-cluster K-means distance reduction as input to a dense
  binary classifier.
- **Input:** `binary.csv`, target `target`, `z_score`, split `0.75`.
- **Model:** `kmeans(4) -> layer(8) -> relu -> layer(1)` with `bce`. The
  immediate dense layer records the exact four-group-to-eight-neuron routing.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log([Loss, Accuracy])`.
- **Output:** Saves `cookbook-kmeans.ogdl`; no inference declaration follows.

### 13. Semantic OGDL inference, lines 258-273

- **Intent:** Train, save, and immediately execute target-free inference from a
  semantic OGDL dense model.
- **Input:** Training uses `binary.csv`, target `target`, `z_score`, split
  `0.75`. Inference reuses the source and excludes `target`.
- **Model:** `layer(8) -> silu -> layer(1)` with `bce` for training; inference
  declares only `.load("cookbook-inference.ogdl")`.
- **Policy:** `adamw`, `epochs(1)`, and `lr(0.0002)`, with
  `save("cookbook-inference.ogdl")`. Inference logs `Time` and `Device`.
- **Output:** One semantic OGDL artifact and one target-free Dense inference
  report, discarded after successful evaluation.

### 14. GRU train, resume, and inference, lines 275-306

- **Intent:** Train and resume a gated recurrent-unit model, then run saved-model
  inference.
- **Input:** `binary.csv`, target `target`, `z_score`, split `0.75` for both
  training declarations; inference excludes `target`.
- **Model:** `gru(8).layer(1)` with `bce` in both runs.
- **Sequence semantics:** Feature columns are one scalar sequence per row, each
  row starts with an all-zero hidden state, and the implementation uses the
  reset-before GRU equations.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log(Loss)`; saves `cookbook-gru-first.ogdl`, resumes it, then saves
  `cookbook-gru-resumed.ogdl`.
- **Output:** Final target-free `load` and `evaluate` produce a Dense inference
  report. The report is discarded.

### 15. Dense-F32 Llama GGUF inference, lines 308-313

- **Intent:** Execute the checked-in dense-F32 Llama instrument against exact
  integer token IDs.
- **Input:** `examples/datasets/llamacpp-archs-seed42/tokens.txt`, with no
  target, split, exclusion, or normalization declaration.
- **Model:** `recipe.model().load("examples/datasets/llamacpp-archs-seed42/llama-dense.gguf")`.
  The `.gguf` extension selects the `GgufLlama` inference family; model
  geometry and vocabulary come from the file.
- **Policy:** `infer().log([Time, Device])` only.
- **Output:** Native inference returns raw little-endian F32 logits for every
  input position and vocabulary entry, and prints the selected timing/device
  fields after teardown. The report is discarded. There is no local llama.cpp
  parity assertion in the current consolidated source.

### 16. Embedding and attention train, resume, and inference, lines 315-359

- **Intent:** Train fixed integer-token embeddings with attention, resume the
  semantic model, and infer target-free rows.
- **Input:** `examples/datasets/cookbook/tokens.csv`; target `target`; split
  `0.75`; no numeric normalization.
- **Model:** `embed(4).vocab(8).attn(2) -> layer(4) -> relu -> layer(1)` with
  `mse`. `.vocab(8)` immediately completes the embedding declaration.
- **Sequence semantics:** Each row's integer feature columns are fixed token
  positions. Attention is causal multi-head self-attention over that embedding
  sequence, and the embedding input remains exact integer IDs rather than a
  numerically normalized feature matrix.
- **Policy:** Both runs use `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule,
  and `log(Loss)`. The first saves `cookbook-embedding-first.ogdl`; the second
  repeats the declaration, resumes it, and saves
  `cookbook-embedding-resumed.ogdl`.
- **Output:** Target-free inference excludes `target`, loads the resumed model,
  and evaluates a Dense report. The report is discarded.

### 17. Ordered multi-source wine data, lines 361-375

- **Intent:** Train regression over an ordered multi-source declaration and an
  explicit appended source.
- **Input:** `recipe.data([WINE_PATH, WHITE_WINE_PATH]).set(WINE_PATH)` records
  red wine, white wine, then red wine again in source order; target `quality`;
  `z_score`; split `0.8`.
- **Model:** `layer(16) -> silu -> layer(1)` with `mse`.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.0002)`, the linear schedule selected by
  `.lr`, and `log([Loss, R2])`.
- **Output:** Saves `cookbook-data-sources.ogdl`; no inference declaration
  follows.

### 18. Convolution and pooling, lines 377-396

- **Intent:** Exercise two channelwise convolution blocks, activation attachment,
  pooling, and a grouped-to-dense output path for binary focal training.
- **Input:** `binary.csv`, target `target`, `z_score`, split `0.75`.
- **Model:** `conv(2, 2) -> relu -> conv(3, 2) -> prelu -> pool(2) ->
  layer(1)` with `focal`. The final dense layer receives the pool's derived
  groups.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.001)`, cosine schedule, and
  `log([Loss, Accuracy])`.
- **Output:** Saves `cookbook-convolution-pooling.ogdl`; no inference call
  follows.

### 19. Repeated Bayesian conditionals, lines 398-411

- **Intent:** Prepare two observed categorical target conditionals in declaration
  order and evaluate both outputs from one semantic model.
- **Input:** `examples/datasets/cookbook/bayes_multi.csv`; targets, in order,
  `play`, `travel`; split `0.8`; no numeric normalization.
- **Model:** `.bayes("play", ["weather", "wind"])` followed by
  `.bayes("travel", ["weather"])`. Every declared parent remains an inference
  feature; each child is an observed categorical output.
- **Policy:** `recipe.train().save("cookbook-bayes-multi.ogdl")` only. Bayesian
  preparation owns its likelihood and has no generic optimizer loop.
- **Output:** Saves `cookbook-bayes-multi.ogdl`. Target-free inference excludes
  both children, loads the artifact, logs `Time` and `Device`, and returns the
  concatenated Bayesian probability report; the report is discarded.

### 20. Singular Bayesian conditional, lines 413-421

- **Intent:** Prepare and execute one observed categorical conditional.
- **Input:** `examples/datasets/cookbook/bayes.csv`; target `play`; split `0.8`;
  no numeric normalization.
- **Model:** `.bayes("play", ["weather", "wind"])`.
- **Policy:** `recipe.train().save("cookbook-bayes.ogdl")` only.
- **Output:** Saves `cookbook-bayes.ogdl`. Inference excludes `play`, loads the
  semantic artifact, logs `Time` and `Device`, and discards the typed Bayesian
  report after evaluation.

### 21. Airfoil perceptron, lines 423-441

- **Intent:** Train a perceptron block with a Huber activation and Huber loss on
  the UCI Airfoil regression target.
- **Input:** `airfoil_self_noise.dat`; target `col6`; `min_max`; split `0.8`.
- **Model:** `perc(24) -> silu -> layer(12) -> huber -> layer(1)` with the
  built-in `huber` objective.
- **Policy:** `adamw`, `epochs(1)`, `lr(0.0001)`, cosine schedule, and
  `log(Loss)`.
- **Output:** Saves `cookbook-airfoil-perceptron.ogdl`; no inference follows.

## Outputs and artifact rules

The source requests only semantic `.ogdl` saves. The user-owned paths are:

```text
cookbook-wine-quality-regression.ogdl
cookbook-tree-forest.ogdl
cookbook-save-resume-first.ogdl
cookbook-save-resume-second.ogdl
cookbook-rnn-first.ogdl
cookbook-rnn-resumed.ogdl
cookbook-residual.ogdl
cookbook-multiclass.ogdl
cookbook-multi-target.ogdl
cookbook-lstm-first.ogdl
cookbook-lstm-resumed.ogdl
cookbook-knn.ogdl
cookbook-kmeans.ogdl
cookbook-inference.ogdl
cookbook-gru-first.ogdl
cookbook-gru-resumed.ogdl
cookbook-embedding-first.ogdl
cookbook-embedding-resumed.ogdl
cookbook-data-sources.ogdl
cookbook-convolution-pooling.ogdl
cookbook-bayes-multi.ogdl
cookbook-bayes.ogdl
cookbook-airfoil-perceptron.ogdl
```

The observability and logarithm workflows intentionally omit `.save(...)` and
therefore export no model. Native images, journals, plans, profiles, caches,
and runtime logs are not cookbook model artifacts. The public artifact contract
allows a native kernel only when a caller explicitly supplies a `.cubin` or
`.hsaco` save path; this example supplies none.

Training reports distinguish native dense execution from reference preparation:
dense reports contain a run, bundle, journal, metrics, and native evidence;
KNN and Bayesian reports contain semantic artifacts and no optimizer run. In
all three cases, the example ignores the report value and uses `?` only for
success or failure. Inference reports distinguish Dense, KNN, Bayes, and
GgufLlama families and are likewise ignored after evaluation.

The saved KNN image is canonical textual OGDL containing the prepared feature
schema, row-ordered reference features, each independently masked target, and
the decoder for discrete labels. It does not contain a native kernel. A saved
Bayesian image is canonical textual OGDL containing exact observed parent and
child codes, dictionaries, source-row order, and Recipe's Laplace-one contract;
version one represents one conditional and version two represents repeated
conditionals. Counts and posterior probabilities are reconstructed by the
native inference graph rather than stored as host-fitted state.

## Failure boundary

Failures remain visible at the real public boundary. There are no retries,
alternate implementations, proxy reports, or test-only fallbacks.

### Declaration failures

`Data`, `Model`, `Train`, and `Infer` builders defer their first invalid
declaration error until `.run()` or `.evaluate()` resolves the sequence.
Relevant `DeclarationErrorKind` values include:

- `EmptyValue`: empty source, target, exclusion, checkpoint, save, or resume
  paths, or an empty target list.
- `InvalidSplit`: non-finite or out-of-range train fraction.
- `InvalidLayer` and `InvalidActivation`: zero or incomplete blocks, invalid
  composition, an activation or normalization without a compatible preceding
  block, a KNN block used non-terminally, or a checkpoint mixed with inline
  model declarations.
- `InvalidBayes`: empty, duplicate, self-referential, repeated-child, or cyclic
  Bayesian dependency declarations.
- `InvalidLearningRate`: a non-positive, non-finite, or non-f32-representable
  learning rate.
- `InvalidTrainingConfiguration`: zero epochs or warm-up, `.every(0)`,
  `.every(...)` without a preceding `.log(...)`, warm-up not less than the
  epoch bound, or an invalid artifact declaration.
- `InvalidInferenceConfiguration`: target-free metric requests such as `Loss`,
  `Accuracy`, `R2`, `Brier`, `Epoch`, or `Lr`.

Calling `.run()` or `.evaluate()` without both preceding `recipe.data(...)` and
`recipe.model()` declarations returns an unsupported sequence error. Taking a
sequence clears it before validation, so the caller must declare it again.

### Data and model preparation failures

Training maps declaration failures into `TrainingError::Declaration` and data
failures into `TrainingError::Data`. Data preparation requires at least one
source, at least one target, and an explicit split. It can fail while reading or
distilling a file/container, inferring categorical or numeric vectors, applying
column/row selection, encoding, or splitting. Missing files, malformed CSV or
DAT rows, incompatible target widths, unsupported values, and the finite default
ingest limits are all preparation failures.

Inference maps the same source and selection failures into
`InferenceError::Data`. Its target-free policy rejects any target, split, or
normalization declaration because the loaded semantic model owns that schema.
`.load(...)` must name an existing `.ogdl` or `.gguf`; unknown extensions,
missing files, invalid GGUF bounds, unsupported architectures, quantized or
incompatible Llama geometry, empty or out-of-vocabulary token sequences, and
semantic checkpoint decoding failures are returned as model or compile errors.

The dense training compiler accepts only an explicit built-in objective and the
Recipe-owned AdamW policy. It rejects loaded checkpoints as inline dense
training models, model-referenced objectives, missing normalization for ordinary
numeric input, numeric normalization on an embedding input, cosine or
exponential schedules without a finite epoch endpoint, incompatible metric and
loss families, and more than one validation metric family in one run. Embedding
blocks require an immediate nonzero `.vocab(...)`; recurrent blocks in this
source have no chained block operations.

KNN preparation requires exactly one standalone `.knn(neighbors)` block and
rejects optimizer, learning-rate, warm-up, epoch, iterative log/plot, kernel,
or generic-loss declarations. Bayesian preparation similarly rejects inline
layers, normalization, generic objectives, optimizer policy, loop bounds,
iterative metrics, and kernel artifacts. A missing semantic resume model is
normal for both reference families and starts a fresh reference set.

### Resume, artifact, native, and runtime failures

- `TrainingError::Resume` covers an unreadable or incompatible semantic resume
  model. A supplied native resume kernel is accepted only with a semantic
  `.ogdl`, an existing authenticated realization for the current static
  program, matching target/toolchain/topology/discovery identity, and matching
  bytes. A missing kernel file follows the normal recompilation path only when
  the semantic model remains valid.
- `TrainingError::Checkpoint` covers semantic model or requested native-kernel
  write failures. KNN and Bayesian reports reject a native-kernel save because
  those branches have no training kernel artifact.
- `TrainingError::Native` and `InferenceError::Native` cover exact measured
  profile loading, current device discovery, binding, target specification,
  pinned toolchain, local host configuration, and identity mismatches. No
  discovered local GPU, a missing or stale measured profile, or a changed
  native configuration fails closed.
- `TrainingError::Compile` and `InferenceError::Compile` cover graph lowering,
  shape, operation, and artifact-program errors before execution.
- `TrainingError::Runtime` and `InferenceError::Runtime` cover native execution,
  report publication, signal-handler setup, and other runtime stages. A device
  fault from invalid `.log()` or `.ln()` input is visible through this boundary.
- `InferenceError::Execute` covers a failed native inference lifecycle or an
  invalid output image. Report publication occurs only after teardown, and a
  publication failure is returned rather than hidden.

All errors are propagated by `main` as boxed errors. A successful run reaches
the final `Ok(())`; it does not imply that a later workflow was attempted when
an earlier workflow failed.
