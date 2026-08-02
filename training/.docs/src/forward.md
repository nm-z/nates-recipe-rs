# Forward graph construction

`training/src/forward.rs` is the shared, internal forward-lowering module for
the training crate (`lib.rs` declares it as `mod forward`, so none of its
items are part of the public facade).  It contains two related kinds of code:

1. the exact recurrent equations and the temporary values needed to
   differentiate them; and
2. small, typed `ScalarProgram` constructors and alias declarations reused by
   dense training, checkpoint inference, KNN normalization, attention, and
   the supported GGUF llama graph.

The module does not load data, initialize or update parameters, calculate a
loss, schedule a kernel, prepare a native image, or execute a program.  Those
responsibilities remain in `compile.rs`, `inference.rs`, `execute.rs`, and the
lower crates.  The module emits only graph calculations through
`RecurrentForwardGraph`, which keeps training and inference on the same
forward equations without sharing parameter ownership or backward code.

## The paired graph boundary

`RecurrentForwardGraph` is deliberately minimal.  Its associated error must
convert from `recipe_language::LanguageError`, and it exposes exactly these
operations:

| Trait method | Training implementation | Inference implementation | Graph operation and contract |
| --- | --- | --- | --- |
| `zero_f32(shape)` | `TrainingForwardGraph` delegates to `GraphCompiler::zero_f32_tensor` | `InferenceGraphCompiler::zero_f32` | An `IndexMap` creates a zero seed in `IterationDomain::first`; an f32 elementwise zero program writes the requested shape. |
| `gather_matrix_column(matrix, rows, columns, column)` | `GraphCompiler::gather_matrix_column` | `InferenceGraphCompiler::gather_matrix_column` | Validates f32 `[rows, columns]`, emits a checked i32 `IndexMap` containing `column`, then a `Gather` on axis 1 with output `[rows, 1]` and `IndexBounds::Reject`. The gather runs in the caller's domain. |
| `bias_free_linear(input, weight, rows, output_width)` | `GraphCompiler::bias_free_linear` | `InferenceGraphCompiler::bias_free_linear` | Emits an f32 `[rows, output_width]` `Contraction` with contract axes `(1, 0)`, meaning each row is multiplied by a `[input_width, output_width]` matrix. |
| `elementwise_f32(shape, inputs, program)` | Allocates an f32 tensor and emits an elementwise node in the supplied domain | Allocates an f32 tensor and emits an elementwise node in the single inference domain | The supplied `ScalarProgram` is the complete per-element operation. Every input/output pair is marked `AliasPermission::Forbidden` by `forbidden_aliases`. |
| `activate(input, activation, shape)` | Calls `GraphCompiler::apply_activation` with no PReLU parameter | Calls `InferenceGraphCompiler::apply_activation` with no PReLU parameter | Uses `lower_activation`. Recurrent gates only request fixed sigmoid or tanh, so a learned PReLU scalar is never implicit in recurrent lowering. |

The training adapter stores a `GraphCompiler` borrow and a domain.  Recurrent
nodes use `training_domain`, which is the static domain for every training
iteration.  Index maps used only to construct constant indices or zero seeds
stay in `IterationDomain::first`.  The inference adapter emits every node in
`IterationDomain::first`, because a compiled inference program has exactly one
iteration.  Both implementations append `CalculationNode` values, validate
the completed `CalculationGraph`, canonicalize it through OGDL, and round-trip
the resulting `StaticCalculationProgram`.  Thus the trait is a compile-time
abstraction, not a runtime dispatch or a second execution path.

## Recurrent model boundary and shapes

`DenseRnn`, `DenseGru`, and `DenseLstm` in `model.rs` contain only a nonzero
hidden `width`.  Their first executable form is intentionally narrow:

- the recurrent block must be the leading block, and there can be only one
  block of that recurrent kind;
- the prepared feature schema must contain one ordinary numeric scalar per
  feature column, with `I32` or `F32` storage and no metadata; and
- the current logical shape must have one channel.  Categorical expansion,
  an embedding, an attention block, a preceding layer, padding, or a missing
  sequence boundary cannot be reinterpreted as a recurrent sequence.

`LogicalFeatureShape::recurrent` enforces the one-channel rule and changes the
logical output to sequence length `width` with one channel.  Before lowering,
the training compiler has normalized or explicitly converted the feature
matrix to f32.  The recurrent input is therefore exactly `[rows, sequence]` in
`DType::F32`, where `sequence` is the preceding logical feature length and
`rows` is the complete training or validation partition.  A step gathers one
column, giving `[rows, 1]`.  Hidden and cell tensors are `[rows, width]`.

The model owns no cross-row or cross-run state.  Each sequence starts with an
exact all-zero hidden tensor, and LSTM also starts with an exact all-zero cell
tensor.  Only the final hidden tensor leaves a recurrent block, so the output
is `[rows, width]`; the full sequence and the final LSTM cell are not public
outputs.  `Shape::new` rejects rank-zero shapes and detects element-count
overflow.  The recurrent callers additionally check products that must fit
the checked int32 index domain.

## Sequence lowering

All three `lower_*_sequence` functions unroll the sequence at graph-construction
time.  The `for step in 0..sequence` loop is a host-side graph builder loop,
not a runtime loop or a host transfer.  At each iteration it gathers the
column, calls the corresponding `lower_*_step`, records the returned step
values, and carries the new hidden (and for LSTM, cell) `ValueId` into the next
step.  The return value is `(final_hidden, step_values)`.

### Shared recurrent gate

`lower_recurrent_gate` is used by RNN, both GRU gates, and all four LSTM gates.
For a step input `x`, previous hidden `h`, and gate parameters
`(W_x, W_h, b)`, it emits:

```text
input_projection    = x   * W_x       // [rows, 1] x [1, width]
recurrent_projection = h  * W_h       // [rows, width] x [width, width]
preactivation       = input_projection + recurrent_projection + b
activated           = activation(preactivation)
```

The two projections are `Contraction` primitives with no bias.  The three-way
sum is `sum_program(3)`, an f32 elementwise program.  The bias is a `[width]`
value broadcast over rows by the elementwise shape contract.  The caller
chooses the activation, so the shared helper does not own a gate-specific
parameter or gradient.

### Vanilla RNN

`lower_rnn_step` calls the shared gate with `DenseActivation::Tanh` and returns
`RnnStepValues { input, previous_hidden, preactivation, hidden }`.  The exact
equation is:

```text
h_t = tanh(x_t W_x + h_(t-1) W_h + b)
```

`lower_rnn_sequence` creates `h_0 = 0`, consumes columns in increasing order,
and returns `h_last`.  The training caller owns `[1, H]` input weights,
`[H, H]` recurrent weights, and `[H]` bias.  `compile_training_rnn` initializes
the two weights from distinct deterministic Philox normal streams, scales
them by `sqrt(2 / fan_in)` with fan-in 1 and `H` respectively, and initializes
the bias to zero.  Resume selection is performed by the caller's parameter
initialization path, not by `forward.rs`.

### Reset-before GRU

`lower_gru_step` emits two shared gates and then the reset-before candidate:

```text
r_t = sigmoid(x_t W_xr + h_(t-1) W_hr + b_r)
z_t = sigmoid(x_t W_xz + h_(t-1) W_hz + b_z)
r_h  = r_t * h_(t-1)
n_t = tanh(x_t W_xn + r_h W_hn + b_n)
h_t = n_t + z_t * (h_(t-1) - n_t)
```

The candidate input and recurrent projections are separate bias-free
contractions.  The candidate preactivation uses `sum_program(3)`.  The final
blend is `gru_hidden_program`, whose inputs are candidate, update, and
previous hidden; it computes the displayed `candidate + update * (previous -
candidate)` form.  `GruStepValues` retains the input and previous hidden,
both gate preactivations and outputs, the reset product, candidate
preactivation and output, and the final hidden value.  Nine tensors are owned
by the training caller: three `[1, H]` input weights, three `[H, H]` recurrent
weights, and three `[H]` biases.  All weights use separate deterministic
normal streams and all biases start at zero.

### Zero-cell LSTM

`lower_lstm_step` emits input, forget, output, and candidate gates, then carries
both state tensors:

```text
i_t = sigmoid(x_t W_xi + h_(t-1) W_hi + b_i)
f_t = sigmoid(x_t W_xf + h_(t-1) W_hf + b_f)
o_t = sigmoid(x_t W_xo + h_(t-1) W_ho + b_o)
g_t = tanh(x_t W_xg + h_(t-1) W_hg + b_g)
c_t = f_t * c_(t-1) + i_t * g_t
h_t = o_t * tanh(c_t)
```

`lstm_cell_program` implements the cell equation with four f32 inputs, and a
second `multiply_program` implements output-gate times activated cell.  The
returned `LstmStepValues` preserves input, previous hidden and cell, every
gate preactivation and output, the new cell, and `tanh(cell)` so reverse-time
backward code can differentiate both state paths.  The training caller owns
twelve tensors, four `[1, H]` input weights, four `[H, H]` recurrent weights,
and four `[H]` biases, with independent deterministic weight streams and
zero biases.

## Activation lowering

`lower_activation` turns a semantic `DenseActivation` into one of four
internal forms.  `Owned` names are resolved by `operation_registry` and
`lower_scalar` in the compiler; `Program` values are emitted directly as
elementwise `ScalarProgram`s.

| Declaration | Lowered form |
| --- | --- |
| `Linear` | `Identity`, returns the input `ValueId` without a node |
| `Cosine`, `Exponential`, `Tangent`, `Relu`, `Sigmoid`, `Tanh`, `Gelu`, `Silu` | `Owned("gpu_cos")`, `gpu_exp`, `gpu_tan`, `gpu_relu_into`, `gpu_sigmoid_into`, `gpu_tanh_into`, `gpu_gelu_into`, or `gpu_silu_into` respectively |
| `Logarithm` | `SignedMagnitude("gpu_log_into")`, which computes `sign(input) * gpu_log_into(abs(input))` |
| `NaturalLogarithm` | `Owned("gpu_log_into")`, with no sign reconstruction |
| `LegacySignedLogOnePlus` | `SignedMagnitude("gpu_log1p")`, preserving the historical `sign(x) * log1p(abs(x))` model token |
| `Huber` | `Program(huber_activation_program())` |
| `LeakyRelu`, `Selu`, `Elu` | Canonical Recipe programs from `recipe_ops`: `canonical_leaky_relu_program`, `canonical_selu_program`, or `canonical_elu_program` |
| `PRelu` | `PRelu(canonical_prelu_program())`, requiring the caller to pass the learned scalar alpha |

Training's `apply_activation` allocates an f32 output.  Signed-magnitude
activations allocate separate absolute-value, magnitude, and sign outputs and
finish with `multiply_program`.  A missing PReLU alpha is an
`InvalidNetwork`/`InconsistentCheckpoint` error in the corresponding
compiler.  Ordinary layer, convolution, residual, and saved inference
operations supply the alpha; recurrent gates do not request PReLU.

`huber_activation_program` is a pointwise Huber transform, not the Huber loss.
It selects `0.5 * x * x` when `abs(x) <= 1`, and `abs(x) - 0.5` otherwise.
Every constructor uses `ScalarProgramBuilder`, so operand dtypes, arity,
builder ownership, instruction order, and final output validation are checked
before a node enters the graph.

## Scalar programs and index helpers

The remaining helpers in `forward.rs` are shared building blocks rather than
additional model kinds.

| Helper | Program or contract | Main callers |
| --- | --- | --- |
| `sum_program(n)` | Creates `n` f32 inputs and folds `Add` left-to-right. `n == 0` returns `LanguageErrorKind::InvalidScalarProgram`. | Recurrent gate sums, convolution/residual additions, gradient accumulation, and normalization arithmetic in training and inference. |
| `add_program`, `subtract_program`, `multiply_program`, `divide_program` | Two f32 inputs and one corresponding `ScalarOpcode` result. | Dense and checkpoint inference arithmetic, GGUF linear bias/scale and SwiGLU, temperature scaling, losses and gradients. |
| `square_program` | `value * value`. | Feature and layer normalization statistics, validation, and KNN reference statistics. |
| `multiply_constant_program(c)`, `divide_constant_program(c)` | One f32 input and one f32 bit-preserving constant. | Means and variances, attention scale `1 / sqrt(head_dimension)`, validation denominators, and GGUF scales. |
| `gru_hidden_program` | `candidate + update * (previous - candidate)`. | GRU forward only; the matching backward program is owned by `compile.rs`. |
| `lstm_cell_program` | `forget * previous_cell + input * candidate`. | LSTM forward only; the matching backward program is owned by `compile.rs`. |
| `z_score_program(epsilon, masked)` | `max(variance, epsilon)`, square root, center, divide. With `masked`, an f32 mask greater than zero selects the normalized value and zero selects the original value. | Dense training feature normalization, saved-checkpoint inference, model normalization, and KNN query/reference normalization. |
| `min_max_program(epsilon, masked)` | `max(maximum - minimum, epsilon)` denominator, then `(value - minimum) / denominator`; the same optional mask selection applies. | The same dense, inference, and KNN normalization paths. |
| `l2_square_program(masked)` and `l2_norm_program(epsilon, masked)` | Square each value, optionally multiply by the mask, reduce squared norms outside the helper, clamp by epsilon, take the square root, divide, and optionally select the original value for masked entries. | Dense training/inference and KNN normalization. |
| `causal_mask_program(sequence_length)` | Treats the element position as i32, derives key position and query row by checked remainder/division, and emits i32 `key <= query`. | Training and checkpoint causal attention, and the GGUF llama causal softmax path through the inference compiler. |
| `head_major_source_index_program(sequence, heads, head_dimension)` | Maps a sequence-major output position to the flattened source index of a `[rows, heads, sequence, head_dimension]` tensor. It decomposes channel, head, token, and row with checked i32 arithmetic. | Training and inference attention `head_major_to_sequence`. |
| `forbidden_aliases(inputs, outputs)` | Produces one `AliasPermission::Forbidden` rule for every input/output pair. | Every elementwise node emitted by both graph compilers and the GGUF compiler, plus primitive contractions and gathers where callers require non-aliasing. |

`attention_extent` is the checked boundary for the three attention index
program constants.  A sequence, head count, or head dimension that does not
fit i32 returns `LanguageErrorKind::InvalidScalarProgram`, which is converted
to the caller's typed compile error.  `causal_mask_program` returns an i32
predicate because `ScalarOpcode::LessThanOrEqual` produces an i32 truth value;
the materialized causal softmax consumes that mask without a host-side branch.

## Training callers and lifecycle

The forward module is reached from one public compilation family:

1. `compile_dense_training*` validates the prepared schema and topology,
   converts non-embedding feature matrices to f32, and fits data normalization
   as graph calculations in `IterationDomain::first`.  `z_score`, `min_max`,
   and `l2_norm` in `GraphCompiler` use the corresponding programs from this
   module.  Layer and residual operation normalization uses the same z-score
   program for per-row layer statistics or masked per-column batch statistics.
2. `compile_training_blocks` walks the effective `DenseBlock` list.  For an
   RNN, GRU, or LSTM it calls `compile_training_rnn`,
   `compile_training_gru`, or `compile_training_lstm`.  Each validates the
   current f32 matrix, creates the parameter tensors and resume selectors,
   invokes the shared sequence lowerer through `TrainingForwardGraph`, and
   stores the final output plus step records in `RnnValues`, `GruValues`, or
   `LstmValues`.  Downstream layers see only `[rows, width]`.
3. The loss consumes the final model output.  `backward_blocks` walks the
   block tape in reverse and dispatches the saved step records to
   `backward_rnn`, `backward_gru`, or `backward_lstm`.  Those functions use the
   captured preactivations, gate values, previous states, and recurrent weights
   to emit complete reverse-time gradients.  RNN gradients cover all three
   parameter tensors, GRU all nine, and LSTM all twelve; each step contribution
   is accumulated with elementwise additions over the complete supervised
   partition.
4. `update_blocks` consumes those gradients in the declaration order and
   applies the ordinary AdamW transition.  It stores the updated parameter and
   moment `ValueId`s in `DenseRnnState`, `DenseGruState`, or `DenseLstmState`,
   together with sequence length and width.  Checkpoint versions 14, 15, and
   16 retain these row-free states.  Forward lowering itself never serializes
   a host vector of values or carries hidden state between epochs.
5. Validation calls `compile_validation_rnn`, `compile_validation_gru`, and
   `compile_validation_lstm` from `compile_validation_blocks`.  They verify
   declaration/state sequence geometry, require `[rows, sequence]` f32 input,
   pass the updated parameter `ValueId`s, and replay the same lowerers in the
   validation domain.  They retain only the final hidden state for the
   requested metrics.
6. `GraphCompiler::finish` marks external tensors, validates and canonicalizes
   the graph, constructs `StaticCalculationProgram::new_with_metrics`, and
   round-trips its OGDL form.  Runtime execution then follows
   `prepare_and_execute_local_training_controlled`: preparation and native
   realization happen before `init`, exactly one admission image is uploaded
   per finalized device, the static forward/loss/backward/update graph runs in
   the loop, and outputs and metric samples are retained after `exit`.  The
   loop has no host feature or model ingress and no loop-time forward graph
   construction.

## Inference, KNN, and GGUF callers

The same source functions are reused by three target-free paths.

### Saved dense checkpoint inference

`compile_prepared_inference` lowers feature spans and saved normalization,
then traverses the checkpoint's effective `blocks` exactly once.  For a saved
RNN, GRU, or LSTM, `InferenceGraphCompiler::compile_rnn`, `compile_gru`, and
`compile_lstm` require that saved `sequence_length` equals the current input
width, require f32 input `[rows, sequence_length]`, and validate every saved
parameter image:

- RNN: input weight `[1, H]`, recurrent weight `[H, H]`, bias `[H]`;
- GRU: three of each of those shapes, for reset, update, and candidate; and
- LSTM: four of each of those shapes, for input, forget, output, and candidate.

The images become external f32 tensors and are passed to the shared sequence
lowerer.  Inference discards the returned step vectors and returns only the
final hidden tensor to later saved blocks.  Its adapter emits the same
`IndexMap`, `Gather`, `Contraction`, and elementwise forms, but all domains are
first-iteration domains.  The final graph is validated, canonicalized, and
round-tripped through `StaticCalculationProgram::new` with one iteration.

Saved layer and residual operations call `lower_activation`,
`z_score_program`, `min_max_program`, `l2_*` programs, and the same forbidden
alias contract.  `apply_data_normalization` additionally admits the saved
feature mask and fitted statistics; identity normalization rejects unexpected
statistics, and L2 normalization rejects unexpected fitted tensors.  KNN
inference uses `normalize_knn_features` and `apply_knn_*` to compute reference
statistics and apply the same scalar programs to query and reference rows,
with its fixed epsilon supplied by the KNN path.

### GGUF llama inference

`training/src/gguf_llama.rs` imports only the generic arithmetic pieces from
`forward.rs`: `add_program`, `multiply_program`,
`multiply_constant_program`, and `forbidden_aliases`.  The supported dense-F32
llama graph uses them for linear bias and optional scale, SwiGLU gate-times-up,
attention scaling, and non-aliasing declarations.  Token gathering, RoPE,
causal attention, RMSNorm, and output projection are assembled by the GGUF
compiler and the inference compiler.  No recurrent lowerer is involved in
this path.

## Failure boundaries

The forward helpers deliberately propagate the real lower-layer error instead
of inventing a fallback graph.

- `Shape::new` errors become `TrainingCompileErrorKind::Language` or
  `InferenceCompileErrorKind::Language` for rank-zero or overflowing shapes.
- A zero-input `sum_program`, an invalid scalar opcode signature, foreign
  scalar-builder values, builder identity exhaustion, or an out-of-range
  attention extent becomes a language invalid-scalar-program error.
- A recurrent column outside `0..sequence`, a missing compiler tensor, an
  unexpected dtype or extent, a nonzero-width violation, or a missing learned
  PReLU scalar is reported as `InvalidNetwork` during training and
  `InconsistentCheckpoint` during inference.
- Checked products and checked int32 conversions report
  `ArithmeticOverflow` or `UnsupportedExtent`; this covers matrix element
  counts, index maps, sequence/head extents, and parameter byte counts.
- Registry lookup and scalar lowering propagate `Operation`; static-program
  validation propagates `Program`; graph and OGDL round trips propagate
  `Language` and `Ogdl` respectively.

The inference compiler has an additional checkpoint-specific boundary: any
saved recurrent sequence/width mismatch, malformed parameter image, or
checkpoint topology mismatch is rejected before native preparation.  The
training compiler similarly rejects invalid recurrent placement, duplicate
recurrent blocks, nonnumeric features, and identity normalization on a
non-embedding sequence before the forward graph is built.

These checks preserve one authoritative path: the same equations and scalar
program identities are used for training forward, validation replay, saved
inference, and the shared arithmetic portions of other inference models.
