# Recipe implementation TODO

## How to read this file

`API.ogdl` is the intended public Recipe API specification. It must not be reduced, hidden, or rewritten merely to
match an incomplete implementation. A specified API that does not work remains unfinished.

An unchecked item means its end-to-end behavior has not been accepted against the current checkout. It does not by
itself prove that no implementation exists; inspect current code and tests before changing it.

Every implementation item carries at least one provenance tag:

- `[human]`: requested directly by the repository owner.
- `[contract]`: required by `AGENTS.md`, `CONTRIBUTING.md`, `system-contract.md`, or `topology/contract.toml`.
- `[api]`: required to implement an entry in `API.ogdl`.
- `[audit]`: tied to an observed implementation defect or incomplete execution path.
- `[decision]`: requires an explicit semantic or API decision before implementation; do not guess.
- `[specified]`: the repository owner already clarified the intended good-faith approach and implementation boundary;
  do not treat the requirement as ambiguous without identifying a concrete contradiction.

Policy, not checklist work:

- Preserve every intended `API.ogdl` declaration even while it is incomplete.
- Report declarations as working, semantically wrong, partial, or unimplemented based on execution evidence.
- Remove only obsolete non-spec mechanisms; do not delete specified behavior to make the implementation look complete.
- Accept a feature only after its public execution, failure boundaries, serialization, and resume behavior where
  applicable are tested. A compile test that discards a `Result` is not acceptance evidence.

Work proceeds in priority order: repair execution foundations, resolve semantic decisions, remove obsolete mechanisms,
then complete the remaining API surface.

## P1 — Live, bounded execution output

Basis: `[human]` live loss visibility priority; `[audit]` the source runner buffers child output with
`Command::output()`.

- [x] `[human]` `[audit]` Replace `Command::output()` with genuinely live child stdout/stderr forwarding so loss is
      observable while training runs and output memory remains bounded.
- [x] `[audit]` Propagate live-metric writer and forwarding-thread failures instead of silently ignoring write errors
      or presenter panics.
- [x] `[audit]` Add a source-runner test proving epoch output is observable before the training child exits.

## P2 — Whole-training-set semantics and bounded instrumentation

Basis: `[contract]` a training epoch is one optimizer update over the complete training partition; `[audit]` one-row
updates and a production journal preallocation once expanded a small run into 1.272 TB of trace storage.

- [x] `[contract]` `[audit]` Bound production lifecycle accounting independently of future iteration count. Retain
      only bounded state required for failure handling, contract validation, and read-only hardware acceptance
      inspection. Never add execution controls or lifecycle evidence to semantic models or exported artifacts.
- [x] `[contract]` `[audit]` Remove one-row optimizer updates and process the entire training partition as one logical
      matrix and one optimizer update per epoch.
- [x] `[contract]` Separate logical full-set execution from physical realization: remove public minibatch controls and
      logical batches-per-epoch, repeated-row, padding, and per-batch gather semantics. Retain hardware-derived
      tiling/chunking only as a backend detail when required, without changing epoch or optimizer-update semantics.
- [x] `[contract]` `[audit]` Report loss over the complete training partition rather than the final physical batch or
      sample.
- [x] `[contract]` `[audit]` Make batch normalization operate over the complete training partition and replace tests
      that normalize one-row batches.
- [x] `[contract]` Derive full-set admission from prepared shapes, memory requirements, and the measured hardware;
      reject only when the complete logical operation cannot be realized.

## P3 — Graceful stopping and current-state saving

Basis: `[human]` Ctrl+C saving priority; `[contract]` optional epochs run until Ctrl+C and save declarations are known
before execution.

- [x] `[contract]` `[audit]` Put optional `.save(...)` declarations on `Train` before `.run()` so save paths are known
      throughout execution and remain independent of `.resume(...)`.
- [x] `[human]` `[contract]` Implement SIGINT/Ctrl+C as a graceful stop request that quiesces at a safe boundary,
      preserves current GPU weights, runs exit transfers, and releases resources afterward.
- [x] `[human]` `[audit]` Make `recipe run` survive the first Ctrl+C long enough for its training child to stop, save,
      report cleanly, and remove the temporary binary.
- [x] `[contract]` On Ctrl+C, write current weights to every declared model save path; without `.save(...)`, stop
      gracefully and export nothing.
- [x] `[contract]` Add process-level tests proving Ctrl+C produces a loadable model containing the latest completed
      training state.

## P4 — Semantic-model and native-kernel artifacts

Basis: `[contract]` the exported artifact contract permits only an optional `.ogdl` model and optional realized
`.cubin` or `.hsaco` kernel, selected by extension.

- [x] `[contract]` `[audit]` Retain the exact realized CUDA or HSA artifact from preparation so kernel save paths write
      native bytes, never OGDL checkpoint text.
- [x] `[contract]` `[api]` Implement literal one-path and two-path save forms for model-only, kernel-only, and
      model-plus-kernel export, with no tuple, array, macro, repeated save chain, or extra user-owned files.
- [x] `[contract]` `[api]` Implement model-only and model-plus-kernel resume; reject kernel-only resume, load an
      existing OGDL model, and start fresh when the model path does not exist.
- [x] `[contract]` Recompile a compatible native kernel from OGDL metadata when resume or save requires one and no
      usable native path was supplied.
- [x] `[contract]` Validate extensions, backend, measured-machine identity, and semantic compatibility without
      exporting caches, plans, profiles, journals, or checkpoints.

## P5 — Deterministic source handling

Basis: `[human]` remove compiler-diagnostic control flow; `[audit]` the source frontend currently parses rustc E0061
diagnostics and recompiles rewritten source.

- [x] `[human]` `[audit]` Delete the rustc-diagnostic-driven retry path, including E0061 arity probing and
      named-gradient probe rewriting.
- [x] `[api]` `[decision]` Implement one deliberate pre-rustc lowering pass for `API.ogdl` forms that Rust cannot
      express directly, or record a human-approved API decision for those forms. Preserve literal one- and two-argument
      contracts where required; never infer syntax from compiler diagnostics or silently rewrite `API.ogdl` to match
      implementation limitations.
- [x] `[audit]` Add tests proving `recipe run FILE.rs` invokes rustc once, does not use diagnostics as parser control
      flow, and reports invalid syntax without retrying a modified source file.

## P6 — Semantic corrections and required decisions

Basis: `[api]` context defines declaration meaning; `[audit]` current runnable paths sometimes reinterpret data,
insert topology, select labels, or accept metrics without implementing their declared behavior.

- [x] `[contract]` `[audit]` Make distilled vector dtypes authoritative through target preparation, graph compilation,
      output-shape selection, loss selection, metrics, execution, serialization, resume, and inference.
- [x] `[api]` `[audit]` Derive loss choice and target validation from the declared or inferred dtype rather than a
      case-specific BCE fallback.
- [x] `[api]` `[audit]` Implement applicable full-partition observability for every declared metric: Loss, Accuracy,
      R2, AuRoc, AuPrc, Brier, CalibrationError, Epoch, Lr, Time, and Device. Plotting and logging must execute rather
      than silently no-op; BCE is reported through Loss, not invented as a separate API metric.
- [x] `[audit]` `[decision]` Define and record the normative output-shape rule before changing code. The executed
      topology must be visible and serialized; do not silently append a learned output adapter that was absent from
      the model declaration.
- [x] `[api]` `[decision]` `[human]` `[specified]` **Human decision (recovered and implemented 2026-07-30):** Multiple
      declared numeric targets form one ordered target matrix and the final model width equals the declared target
      count. BCE/focal use independent binary coordinates, CE uses one row-wise one-hot joint class structure, and
      regression losses use one ordered output vector. Preserve declaration order through reduction, metrics,
      semantic-model v11, exact-order resume, inference, and named public output; the complete rule is C34 in
      `system-contract.md`.
- [x] `[api]` `[decision]` `[human]` `[specified]` **Human decision (2026-07-29):** `.log()` is the signed logarithm
      `sign(x) * ln(abs(x))` on finite nonzero inputs with derivative `1 / abs(x)`; `.ln()` is ordinary `ln(x)` on
      finite positive inputs with derivative `1 / x`. Both reject positive and negative zero. Training, inference,
      public lowering, and serialization use distinct identities; the old `logarithm` checkpoint token remains a
      decoder-compatible identity for the former provisional `sign(x) * ln(1 + abs(x))` behavior.
- [x] `[api]` Make `.cos()` and `.exp()` context-sensitive: after `.lr(...)` they select learning-rate schedules; after
      a model layer they are activations.
- [x] `[api]` Enforce ordered normalization semantics: data normalization belongs to data; model layer and batch
      normalization execute at the exact declared position relative to activations.
- [x] `[api]` `[audit]` Make gradient clipping optional; when `.grad(...)` is absent, do not silently clip at norm 1.0
      or compile a clipping graph.
- [x] `[audit]` Replace expected training and save panics with typed Recipe errors and graceful nonzero CLI exits;
      reserve OGDL diagnostics for OGDL parsing or serialization boundaries.
- [x] `[api]` Add regression tests for dtype preservation, loss selection, output shape, normalization order,
      activation/schedule context, metrics, and serialized architecture.

## P7 — Inventory and boundary cleanup

Basis: `[api]` `API.ogdl` defines the root declaration facade; `[contract]` internal/inter-crate APIs may still be
required for measured physical realization and lifecycle validation.

- [x] `[api]` `[audit]` Inventory every `API.ogdl` entry as routed, partial, or unimplemented, with its production
      boundary and an explicit separation between implementation status and dated real-hardware evidence.
- [x] `[audit]` Make runnable cookbook programs propagate failures so a broken public execution cannot appear
      supported.
- [x] `[contract]` `[api]` Make `.epochs(...)` optional end to end; an omitted bound trains indefinitely until graceful
      Ctrl+C without inventing a finite epoch count.
- [x] `[contract]` `[audit]` Delete automatic early-stopping configuration, state, kernels, optimizer gates,
      checkpoint fields, public methods, and tests.
- [x] `[contract]` `[audit]` Derive workgroup widths, contraction strategy and tile, reduction lanes, worker count,
      staging memory, and watchdog policy from measured capabilities and realized shapes instead of fixed tuning
      constants.
- [x] `[contract]` `[audit]` Remove hidden `SavePath` defaults such as `.save(())`, repeated chained save behavior, and
      other user-facing artifact forms absent from `API.ogdl`.
- [x] `[contract]` `[audit]` Classify lower-level exported controls by audience. Remove banned user-facing batching,
      early-stopping, checkpoint, and instrumentation controls, but retain or narrow inter-crate types required for
      hardware-derived physical realization and contract validation.
- [x] `[contract]` Add regression tests for fresh resume fallback, existing-weight continuation, save without resume,
      resume without save, no-save training, real native-kernel export, and absence of extra artifacts.

## P8 — Complete every specified API family

Basis: `[api]` each item below maps to declarations in `API.ogdl`. These are feature epics, not claims that every
subfeature is wholly absent. Split an epic only after its current implementation status is inventoried.

- [x] `[api]` Fully implement the data surface for single and multiple sources, `.set`, targets, exclusions,
      normalization, splitting, supported files, directories, and archives with semantic vector typing.
- [x] `[api]` Fully implement `.layer(...)` and distinct `.perc(...)` execution with ordered activations and
      normalization.
- [x] `[api]` Fully implement convolution and pooling blocks, including declared group-to-neuron division and fallback
      routing semantics.
- [x] `[api]` `[human]` `[specified]` Implement `.kmeans(clusters)` as `N features -> clusters` L2-distance
      reduction. Assignment uses pairwise L2; each centroid update is the mean of every row assigned to that cluster;
      initialization, empty clusters, and exact ties use deterministic Recipe-owned rules. Emit one distance per
      centroid and preserve the specified divisible cluster-to-neuron routing and fully connected fallback end to end.
- [x] `[api]` `[human]` `[specified]` Implement `.knn(neighbors)` as feature reduction for every declared output.
      This is the only KNN form: output count comes from the declared outputs, never a second public argument.
      Recipe owns its deterministic distance, weighting, aggregation, and tie rules; serialize, resume, and infer it
      end to end. Existing-model resume appends the current training references after the saved references, retains
      duplicate observations as statistical weight, preserves saved-then-current distance-tie order, extends
      row-derived label dictionaries without recoding saved labels, and rejects topology or row-free schema drift.
- [x] `[api]` Fully implement residual blocks with automatic identity or linear projection determined by branch output
      width.
- [x] `[api]` `[human]` `[specified]` Implement `.lgbm(depth)`, `.cbst(depth)`, and `.xgbst(depth)` as terminal
      supervised one-tree learners and `.forest(trees)` as an exact-count ensemble whose nested family selects the
      structure builder. Recipe owns deterministic mean-threshold variance-gain splitting, fixed structure,
      full-size Philox bootstrap diversity for multi-tree forests, exact-tie-left traversal, unscaled tree summation,
      loss/AdamW-trained leaf values, task-derived output width, checkpoint-v12 persistence/resume, and saved-model
      inference. Preserve the distinct LightGBM leaf-wise, CatBoost symmetric, and XGBoost level-wise construction
      rules recorded in C35 of `system-contract.md`; do not import hidden boosting rounds or shrinkage.
- [x] `[api]` `[human]` `[specified]` Implement the first executable Bayesian case as one observed dictionary-categorical
      target child conditioned on one or more observed dictionary-categorical feature parents. Preserve parent
      declaration order, complete prepared training rows, exact label dictionaries and raw observations in canonical
      `.ogdl`; calculate mixed-radix parent configurations, histogram counts, and Laplace-one posterior probabilities
      natively. Missing or unseen inference labels use each parent's one reserved route and therefore produce the
      uniform posterior for an unobserved configuration. Existing-model resume appends saved then current observations.
- [x] `[api]` Implement the next concrete Bayesian instrument: repeated observed dictionary-categorical target
      conditionals whose children exactly match `.target(...)` order and whose parents are observed categorical
      inference features. Shared parents are prepared once; each declaration retains its own raw observations and
      native Laplace-one histogram/posterior graph; semantic-model v2 preserves ordered conditionals and exact resume;
      inference packs adjacent probability ranges in declaration order. A target child used as another conditional's
      parent still rejects because ancestral prediction/marginalization is not implied. Numeric distributions, custom
      priors, latent state, missing-training-observation marginalization, and generic objectives require separate
      concrete declaration contracts rather than one speculative adapter.
- [x] `[api]` Implement the first concrete fixed-token sequence case end to end. Feature-column order is sequence
      order; every value is an exact int32 token ID in `0..vocabulary`. One leading learned embedding table uses
      deterministic Recipe initialization. One immediately following `.attn(heads)` block uses evenly divided heads,
      learned bias-free Q/K/V/output matrices, `1 / sqrt(head_dimension)` score scaling, and an always-causal mask.
      This case invents no tokenizer, padding, position vector, or configurable mask. Training includes full backward
      and AdamW; semantic-model v13 preserves all parameter and moment state for exact resume and native inference.
- [x] `[api]` Implement the first concrete vanilla RNN case end to end. Each independently prepared numeric row is
      one fixed scalar sequence in feature-column order; hidden state starts at exact zero per row, uses shared
      `tanh(x_t W_x + h_(t-1) W_h + b)` transitions, returns only the final hidden state, and never persists state
      across rows or runs. Training performs complete BPTT and AdamW; semantic-model v14 preserves all parameters and
      moments for exact resume and native inference. Chained RNN operations and non-leading/stacked RNNs remain
      rejected until a concrete case defines them.
- [x] `[api]` Implement the first concrete GRU case end to end. It uses the same independent scalar-sequence row
      boundary as the vanilla RNN, exact zero hidden state, reset-before gates
      `r_t = sigmoid(x_t W_xr + h_(t-1) W_hr + b_r)`,
      `z_t = sigmoid(x_t W_xz + h_(t-1) W_hz + b_z)`,
      `n_t = tanh(x_t W_xn + (r_t * h_(t-1)) W_hn + b_n)`, and
      `h_t = (1 - z_t) * n_t + z_t * h_(t-1)`. Training performs complete BPTT and AdamW; semantic-model v15
      preserves all nine parameters and moments for exact resume and native inference.
- [x] `[api]` Implement the first concrete LSTM case end to end. It uses the same independent scalar-sequence row
      boundary as the vanilla RNN and GRU, exact zero hidden and cell state, input/forget/output sigmoid gates,
      candidate tanh, `c_t = f_t * c_(t-1) + i_t * g_t`, and `h_t = o_t * tanh(c_t)`. Only the final hidden state
      leaves the block. Training performs complete reverse-time hidden/cell differentiation and AdamW;
      semantic-model v16 preserves all twelve parameters and moments for exact resume and native inference.
- [x] `[api]` Implement, serialize, resume, and infer the approved forward and backward semantics for relu, leak,
      sigmoid, tanh, tan, selu, gelu, silu, elu, prelu, cos, exp, signed log, natural log, and huber.
- [x] `[api]` Fully implement every loss with dtype-consistent targets: BCE, MSE, MAE, CE, Huber, and focal.
- [x] `[api]` Fully implement data, layer, and batch normalization at their declared positions.
- [x] `[api]` Fully implement AdamW, optional epoch bounds, learning-rate schedules, warmup, logging cadence, plotting,
      save, resume, and run.
- [x] `[api]` Fully implement semantic-OGDL inference loading for saved layer, perceptron, convolution, pooling, and
      residual topology, including target-free feature selection, `Time`/`Device` logging, native evaluation, and
      typed prediction output.
- [x] `[api]` Implement the first named GGUF architecture instrument end to end. Public `.load("model.gguf")`
      executes bounded GGUF-v3 little-endian `llama` models with dense F32 tensors, equal query/KV head counts,
      full-head adjacent-pair RoPE, RMSNorm, causal attention, parallel SwiGLU, saved biases/scales, exact int32 token
      input, and all-position raw logits. Other architectures, quantized tensors, MoE, GQA, and incompatible llama
      variants fail closed instead of being guessed. The checked-in corpus compiles through the public boundary and
      the runnable HSA cookbook matches its llama.cpp CPU oracle at `9.580467046e-14` NMSE and
      `4.190951586e-9` maximum absolute error.
- [x] `[human]` `[specified]` **Human-requested requirement (2026-07-28; not AI-inferred):** Implement reversible,
      structural `.gguf` -> `.ogdl` -> `.gguf` model-file conversion. The `.ogdl` must contain no embedded, copied,
      base64/hex-encoded, or otherwise opaque GGUF byte payload. Express all metadata as typed, human-readable OGDL and
      express tensor encodings and layout structurally rather than retaining source bytes. Implement every metadata
      value type, tensor encoding, endianness, and layout permitted by the current GGUF specification—GGUF v3 as of
      2026-07-29—and reconstruct every valid current-version source byte-for-byte, including its field and tensor
      ordering, exact scalar representations, offsets, alignment, and specified padding. A current-version construct
      may not be labeled unsupported to narrow completion; reject only genuinely newer or unknown extensions, without
      an opaque fallback, and enforce successful round trips with binary-equality acceptance tests.
- [x] `[api]` Keep one `examples/cookbook.rs` containing runnable recipes for every currently executable family: data
      composition, dense layers, perceptrons, ordered multi-target objectives, convolution/pooling, residuals,
      activations, normalization, losses, optimizer/schedules/observability, fixed-token embedding/causal attention,
      scalar-sequence RNN, GRU, and LSTM, save/resume, and semantic-OGDL inference. Do not multiply one cookbook into
      one binary per recipe, and do not present declaration-only stubs as working examples.
- [x] `[api]` Add a runnable observed-categorical Bayesian preparation/save/inference cookbook.
- [x] `[api]` Add runnable cookbook recipes for the repeated observed categorical Bayesian and first dense-F32 llama
      GGUF execution instruments. The `bayes_multi` recipe completed native HSA inference with two ordered target
      posteriors; the `gguf_llama` recipe uses a checked-in llama.cpp logits oracle.
- [x] `[contract]` `[api]` Historical 2026-07-30 HSA record: static workspace checks and three literal
      `recipe run FILE.rs` programs executed multi-target signed-log training, tree/forest,
      embedding/attention, GRU, all-output KNN, repeated observed-categorical Bayesian inference, and dense-F32 llama
      GGUF inference on `hsa:GPU-bb788ef9613fd5b3@0000:03:00.0`; the llama.cpp oracle comparison retained
      `9.580467046e-14` NMSE. This is not a current AMD acceptance result; only NVIDIA hardware is available for the
      present acceptance pass.
