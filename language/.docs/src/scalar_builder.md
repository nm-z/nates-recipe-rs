# Scalar program builder

`recipe-language::ScalarProgramBuilder` is the public, typed construction API
for a `recipe_core::ScalarProgram`. It emits a small scalar SSA graph that is
later attached to an elementwise primitive. The builder is backend-neutral: it
does not load buffers, choose a device, schedule work, or emit LLVM. Its only
state is the ordered set of scalar inputs, constants, and instructions being
assembled.

The implementation is [`language/src/scalar_builder.rs`](../../src/scalar_builder.rs).
The value and opcode contracts used by the builder are defined in
[`core/src/scalar.rs`](../../../core/src/scalar.rs).

## Public types

`ScalarProgramBuilder` and `ScalarExpression` are re-exported from
`recipe_language::lib`. `ScalarExpression` is an opaque, typed handle:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarExpression {
    // private to the language crate
    owner: u64,
    id: ScalarValueId,
    dtype: DType,
}
```

Only `id()` and `dtype()` are exposed. There is no public constructor, so a
caller cannot forge a handle with a different type or with another builder's
owner. Handles are copyable and can be reused in several later instructions,
but they remain tied to the builder that produced them.

`DType` has exactly two payload types, `F32` and `I32`. Both occupy four bytes.
`ScalarLiteral` is either `F32Bits(u32)` or `I32(i32)`. The f32 form stores the
raw binary32 bits, so signed zero, infinities, and NaN payloads are preserved.
Use `ScalarLiteral::dtype()` to obtain the corresponding type.

The finished value is the core `ScalarProgram`:

```text
inputs:       Vec<ScalarInput>       (id, dtype)
constants:    Vec<ScalarConstant>    (id, literal)
instructions: Vec<ScalarInstruction> (result, dtype, opcode, operands)
outputs:      Vec<ScalarValueId>
```

The program is typed, acyclic, and intra-kernel. The owner marker exists only
while building and is intentionally absent from the finished artifact.

## Lifecycle and identity

1. Call `ScalarProgramBuilder::new()`.
2. Add inputs and constants.
3. Apply opcodes through `apply`, `unary`, `binary`, or `ternary`.
4. Pass one or more expressions to `finish`; this consumes the builder and
   returns a validated `ScalarProgram`.

`new()` allocates an owner from the process-wide `NEXT_BUILDER` `AtomicU64`.
The counter starts at one and uses checked increment with relaxed atomic
ordering. If the counter is already `u64::MAX`, allocation fails instead of
wrapping and the method returns `LanguageErrorKind::InvalidScalarProgram` with
`scalar builder identity space exhausted`.

Each builder starts its local scalar-value counter at one. Every successful
`input`, `constant`, or `apply` receives the next `ScalarValueId` and then
increments the counter with checked arithmetic. A value-counter overflow is
reported as `InvalidScalarProgram` with `scalar value identity space
exhausted`. Numeric IDs can therefore repeat in two builders, but the private
owner prevents handles from being mixed.

Allocation order is deterministic. Inputs are appended to `inputs`, literals
to `constants`, and successful applications to `instructions`; instruction
order is exactly the order of successful builder calls. Inputs and constants
are separate vectors in the final program, so all declarations appear before
instructions even when the caller interleaves input and constant calls. The
output vector preserves the order of the slice passed to `finish`. These
orders and IDs are part of the serialized and canonical program identity.

An unsuccessful `apply` does not allocate an ID or append an instruction: the
owner check and opcode signature check happen before `next_expression`. A
failed `next_expression` likewise leaves the corresponding declaration or
instruction unappended.

## Construction methods

| Method | Effect | Immediate checks |
| --- | --- | --- |
| `new()` | Create an empty builder with a unique owner and `next_value = 1`. | Checked owner allocation. |
| `input(dtype)` | Allocate an ID, append `ScalarInput { id, dtype }`, and return a handle. | Checked value-ID allocation. |
| `constant(literal)` | Allocate an ID, append `ScalarConstant { id, value: literal }`, and return a handle. | Checked value-ID allocation; the literal supplies the type. |
| `f32(value)` | Call `constant(ScalarLiteral::F32Bits(value.to_bits()))`. | No numeric-domain filtering. |
| `i32(value)` | Call `constant(ScalarLiteral::I32(value))`. | Checked value-ID allocation. |
| `apply(opcode, operands)` | Check ownership and the opcode signature, allocate a result ID, append one instruction, and return its typed handle. | Every operand must belong to this builder; `ScalarOpcode::result_dtype` must accept the arity and types. |
| `unary(opcode, operand)` | Convenience call to `apply(opcode, &[operand])`. | The unary signature is enforced by `apply`. |
| `binary(opcode, left, right)` | Convenience call to `apply(opcode, &[left, right])`. | The binary signature is enforced by `apply`. |
| `ternary(opcode, first, second, third)` | Convenience call to `apply(opcode, &[first, second, third])`. | The ternary signature is enforced by `apply`. |
| `finish(outputs)` | Check ownership of every output, assemble the core program, call `ScalarProgram::validate`, and consume `self`. | Outputs must be local handles and the completed program must pass core validation. |

`apply` reports a foreign handle as
`InvalidScalarProgram: scalar value <id> belongs to another program builder`.
An unsupported arity or type reports
`InvalidScalarProgram: <opcode> does not accept operands <types>`.
`finish` reports a foreign output as
`InvalidScalarProgram: scalar output <id> belongs to another program builder`.

## Opcode signatures

`ScalarOpcode::arity()` is one for the unary set below, three for `Fma` and
`Select`, and two for every other opcode. `result_dtype()` first requires the
exact arity and then applies these type rules. No implicit numeric conversion
or promotion occurs. `ScalarOpcode` is `non_exhaustive`, so downstream code
must retain a future-opcode rejection path even though this table covers the
current schema.

| Opcode(s) | Operand types | Result |
| --- | --- | --- |
| `Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`, `Minimum`, `Maximum` | Two operands of the same type, either both `F32` or both `I32`. | That same type. |
| `Negate`, `Absolute` | One `F32` or one `I32`. | The operand type. |
| `Fma` | Three `F32` operands. | `F32`. |
| `Equal`, `NotEqual`, `LessThan`, `LessThanOrEqual`, `GreaterThan`, `GreaterThanOrEqual` | Two operands of the same type, `F32` or `I32`. | `I32`, normalized to a truth value by the lowering. |
| `Select` | First operand `I32`; second and third operands have the same type, `F32` or `I32`. | The arm type. A zero condition chooses the third operand; a nonzero condition chooses the second. |
| `BitAnd`, `BitOr`, `BitXor`, `ShiftLeft`, `ShiftRightLogical`, `ShiftRightArithmetic` | Two `I32` operands. | `I32`. |
| `BitNot` | One `I32` operand. | `I32`. |
| `BitcastF32ToI32` | One `F32` operand. | `I32`, preserving the 32-bit representation. |
| `BitcastI32ToF32` | One `I32` operand. | `F32`, interpreting the 32-bit representation. |
| `Require` | One `I32` operand. | `I32`; zero is a rejected calculation and nonzero is normalized to one. |
| `IsFinite`, `IsNan` | One `F32` operand. | `I32` classification flag. |
| `SquareRoot`, `Floor`, `Ceiling`, `RoundNearestEven` | One `F32` operand. | `F32`. |
| `ConvertF32ToI32` | One `F32` operand. | `I32`. |
| `ConvertI32ToF32` | One `I32` operand. | `F32`. |

The opcode comments in `core/src/scalar.rs` define the scalar semantics that
the builder accepts. In particular, f32 `Divide` and `Remainder` use IEEE
operations, while their int32 forms are checked truncating operations;
`Fma` is one-rounding fused multiply-add; and f32 comparisons are ordered,
with NaN equal to false for `Equal` and true for `NotEqual`. These runtime
checks are not evaluated by the builder. A caller expresses a domain
precondition with comparison and `Require` instructions.

`f32(value)` does not reject a non-finite value. A program that needs finite
inputs must emit `IsFinite` and `Require`, as the math and materialization
callers do. The builder also does not evaluate int32 overflow or division
domains. The eventual lowering supplies checked fault behavior for int32
division, remainder, negate, and absolute; ordinary int32 add, subtract, and
multiply are emitted as ordinary int32 operations.

## `finish` validation and invariants

`finish` first rejects output handles owned by a different builder. It then
constructs a `ScalarProgram` and invokes `ScalarProgram::validate()` from
`recipe-core`. That validation runs over the finished vectors in this order:

1. Input IDs are inserted into the definition table.
2. Constant IDs are inserted into the definition table.
3. Instructions are checked in vector order. Every operand must already be in
   the table, the opcode arity and type must match, and the result ID must be
   new before it is inserted.
4. `outputs` must be nonempty and every output ID must be defined.

The core validator reports machine-readable `ValidationCode` values. Relevant
scalar failures are:

| Code | Meaning |
| --- | --- |
| `DuplicateScalarValue` | An input, constant, or instruction result reuses an ID. |
| `ScalarUseBeforeDefinition` | An instruction references an unknown or later instruction result. |
| `ScalarArity` | The instruction operand count differs from `ScalarOpcode::arity()`. |
| `ScalarTypeMismatch` | The instruction result type does not match the opcode signature. |
| `MissingScalarOutput` | The output slice is empty. |
| `UnknownScalarValue` | An output ID is not defined by an input, constant, or instruction. |

The public builder prevents the normal causes of duplicate IDs, use-before-
definition, bad instruction arity, and unknown outputs. The final validation is
still intentional: it is the last boundary before a program becomes an
owned artifact, and it protects the core representation if construction code
changes. All validation failures are converted to one
`LanguageErrorKind::InvalidScalarProgram`; the detail is the core
`ValidationErrors` display string, which retains each code and path.

The builder does not validate tensor arity, tensor dtype, broadcast shape,
alias rules, or index-space geometry. Those belong to the enclosing
`PrimitiveKernel` and `KernelTemplate` contracts.

## Caller inventory

The language crate re-exports the builder from `language/src/lib.rs`. Direct
construction sites currently use it in the math, operations, and training
crates. They all return a `ScalarProgram` and then embed it in an
elementwise primitive or use it as a program input to a larger graph.

### Math

`math/src/program.rs::build` creates the input handles, checks finite inputs,
emits the selected `MathFunction`, and finishes one output. Its helpers use
the same builder for reciprocal, square root, trigonometric range checks,
polynomial/FMA expansions, logarithm and exponential range reduction, and
classification preconditions. `Require` is used for all domain checks instead
of host-side branching in the generated scalar program.

### Operations

The following operation modules call the builder directly or through small
error-mapping helpers:

| Module | Scalar program families |
| --- | --- |
| `ops/src/bayes.rs` | Configuration contribution, joint/query bin indexing, posterior probability, and int32 range checks. |
| `ops/src/binary_metrics.rs` | Input validation, constant division, Brier and ranking terms, recall/calibration contributions, positive-count normalization, and fixed sums. The local `scalar_builder`, `scalar_input`, `scalar_f32`, `scalar_i32`, `scalar_unary`, `scalar_binary`, `scalar_ternary`, and `scalar_finish` helpers map `LanguageError` to `GraphMaterializationFailed`. |
| `ops/src/kmeans.rs` | Square, pairwise L2 distance, membership conversion, constant-one, and centroid update. Its helper family maps every builder call through `graph_error`. |
| `ops/src/knn_outputs.rs` | Square, pairwise distance, known-count, masked distance, finite divide/identity, and categorical code/bin programs, with the same helper pattern. |
| `ops/src/tree.rs` | Split, feature, next-node, leaf-base/index, and scaling programs plus finite and int32 interval requirements. |
| `ops/src/materialize.rs` | FFT and triangular-solve maps, identity and normalization maps, centered statistics, optimizer state/weight updates, gradient clipping, tree branches, and all shared scalar helper functions. The `scalar_*` wrappers attach the materializing operation ID and map failures to `GraphMaterializationFailed`. |
| `ops/src/materialize/training.rs` | Bias-add map. |
| `ops/src/materialize/convolution_pooling.rs` | Divisor, typed pair identity, channelwise pool, two-dimensional pool, and pool-backward maps. |
| `ops/src/materialize/attention_sequence_embedding.rs` | Embedding blend, positional selection, causal masks/shifts, mask-to-zero, checked divide, and rotation maps. |
| `ops/src/materialize/indexing_sort_encoding.rs` | Segment boundaries/destinations, zero and add maps, scaled add, uniform single-row, typed select, clears, checked one, and triangular masks. |
| `ops/src/materialize/loss_metrics.rs` | Accuracy, count normalization, checked/class matches, finite flags, hinge, positive-or-one, logarithm/KL, squared, contrastive, cosine, and triplet maps. |
| `ops/src/materialize/graph_cluster_rl.rs` | Temporal difference, GCN normalization, advantage, Gaussian, subtraction, checked action, categorical, masked, neighborhood/centroid normalization, squared/root, pairwise distance, and representative-hook maps. |
| `ops/src/materialize/tree_boosting.rs` | Split indexing, gradient/Hessian, histogram, regularized leaf, histogram subtraction, gain, one, ordered statistic, threshold, prediction, and split-write maps. |

`ops/src/scalar.rs` is a related but separate path. It has an internal
`Composer` for canonical symbol/composite recipes and inlines
`recipe_math::MathFunction` programs. It does not instantiate
`ScalarProgramBuilder`, although it produces the same core `ScalarProgram`
shape and is consumed by the same primitive and kernel lowering contracts.

### Training and inference

`training/src/forward.rs` builds causal masks and source indices, arithmetic
maps, recurrent GRU/LSTM cells, normalization maps, masked L2 maps, and Huber
activation programs. These functions return `LanguageResult`, so the original
`LanguageError` is retained.

`training/src/compile.rs` uses the builder for the compiled training graph's
tree and recurrence indexing, one-hot and masking maps, dense losses and
backward terms, metric reductions, normalization and clipping, schedule and
warmup maps, learning-rate decay, optimizer gates and updates, Adam powers,
and temperature scaling. Its `TrainingCompileResult` conversion wraps a
builder failure as `TrainingCompileErrorKind::Language`.

`training/src/inference.rs` builds zero, checked conversion, stable sigmoid,
softmax exponent, one-hot update, and feature-destination programs.
`training/src/gguf_llama.rs::symmetric_clamp_program` builds the f32 clamp map.
Both use the inference compile error conversion for `LanguageError`.

All of these callers follow the same pattern: allocate inputs and literals in
the order expected by the enclosing primitive, emit instructions through the
typed API, and call `finish` with the exact output list required by that
primitive. Numeric extent conversions, operation-specific range checks, and
tensor shape checks happen in the caller before or around the builder; they are
not hidden in `ScalarProgramBuilder`.

For a mechanical source inventory, the scalar-returning constructor families
are:

- `math/src/program.rs`: `build` is the public-facing constructor behind
  `ScalarProgram::try_from(MathFunction)`. Its builder-emitting helpers are
  `emit_function`, `require_finite_inputs`, `require_nonzero`,
  `require_positive`, `require_closed_interval`, `require_condition`,
  `require_atan2_nonzero_pair`, `preserve_zero`, `horner`,
  `emit_sine_cosine_core`, `emit_atan_polynomial`, `emit_atan2_core`,
  `emit_exp_core`, `emit_exp_with_underflow`, `emit_exp_reconstruction`,
  `emit_expm1_core`, `emit_log_core`, `emit_log1p_core`, `emit_trunc`,
  `emit_sign`, `emit_sigmoid_core`, `emit_tanh_core`, `emit_softplus_core`,
  `emit_erf_core`, and `copy_sign_from`.
- `ops/src/bayes.rs`: `configuration_contribution_program`,
  `joint_bin_program`, `query_bin_program`, and
  `posterior_probability_program`.
- `ops/src/binary_metrics.rs`: `validated_inputs_program`,
  `divide_by_constant_program`, `brier_program`, `group_start_program`,
  `subtract_one_program`, `ranking_contributions_program`,
  `normalize_ranking_program`, `recall_hits_program`,
  `normalize_positive_count_program`, `calibration_bin_program`,
  `calibration_contribution_program`, and `fixed_sum_program`.
- `ops/src/kmeans.rs`: `square_program`, `pairwise_l2_program`,
  `membership_program`, `one_program`, and `centroid_update_program`.
- `ops/src/knn_outputs.rs`: `square_program`, `pairwise_l2_program`,
  `known_count_program`, `masked_distance_program`, `finite_divide_program`,
  `finite_identity_program`, `categorical_code_program`, and
  `categorical_bin_program`.
- `ops/src/tree.rs`: `split_index_program`, `feature_index_program`,
  `next_node_program`, `leaf_base_program`, `leaf_index_program`, and
  `scale_program`.
- `ops/src/materialize.rs`: `fft_linear_stage_program`, `multiply_program`,
  `triangular_solve_program`, `identity_program`,
  `batch_normalization_program`, `centered_square_program`,
  `normalization_statistics_program`, `affine_normalization_program`,
  `batch_normalization_backward_terms_program`,
  `batch_normalization_gradient_program`, `running_statistics_update_program`,
  `layer_normalization_program`, `layer_normalization_backward_terms_program`,
  `layer_normalization_input_gradient_program`, `rms_normalization_program`,
  `rms_dot_term_program`, `rms_normalization_backward_program`,
  `momentum_program`, `momentum_weight_program`, `adagrad_state_program`,
  `rmsprop_state_program`, `adaptive_weight_program`, `adam_state_program`,
  `adam_weight_program`, `nadam_weight_program`, `lion_state_program`,
  `lion_weight_program`, `lamb_phase_one_program`, `lamb_phase_two_program`,
  `two_identity_program`, `square_program`, `gradient_clip_program`, and
  `tree_branch_program`.
- `ops/src/materialize/training.rs`: `add_program`.
- `ops/src/materialize/convolution_pooling.rs`: `divide_by_program`,
  `typed_pair_identity_program`, `channelwise_pool_result_program`,
  `max_pool_2d_result_program`, and `max_pool_backward_program`.
- `ops/src/materialize/attention_sequence_embedding.rs`:
  `embedding_blend_program`, `positional_select_program`,
  `causal_max_mask_program`, `causal_safe_shift_program`,
  `mask_to_zero_program`, `checked_divide_program`, and `rotation_program`.
- `ops/src/materialize/indexing_sort_encoding.rs`: `segment_boundary_program`,
  `segment_destination_program`, `zero_from_i32_program`, `add_program`,
  `scaled_add_program`, `uniform_single_row_program`, `select_program`,
  `clear_program`, `checked_one_program`, and `triangular_mask_program`.
- `ops/src/materialize/loss_metrics.rs`: `binary_accuracy_program`,
  `normalize_i32_count_program`, `normalize_f32_count_program`,
  `checked_class_match_program`, `class_match_program`,
  `finite_flag_program`, `hinge_loss_program`, `positive_or_one_program`,
  `kl_divergence_program`, `squared_difference_program`,
  `squared_deviation_program`, `contrastive_loss_program`,
  `cosine_terms_program`, `cosine_embedding_loss_program`,
  `triplet_terms_program`, and `triplet_loss_program`.
- `ops/src/materialize/graph_cluster_rl.rs`: `temporal_difference_program`,
  `gcn_normalization_program`, `advantage_delta_program`,
  `gaussian_term_program`, `subtract_program`, `checked_flat_action_program`,
  `categorical_finish_program`, `masked_product_program`,
  `neighbor_normalize_program`, `masked_value_program`,
  `centroid_normalize_program`, `squared_difference_program`,
  `square_program`, `nonnegative_square_root_program`,
  `pairwise_l2_result_program`, and `minimum_representative_hook_program`.
- `ops/src/materialize/tree_boosting.rs`: `split_index_program`,
  `gradient_hessian_program`, `histogram_contribution_program`,
  `regularized_leaf_program`, `histogram_subtraction_program`,
  `two_bin_gain_program`, `constant_one_program`,
  `ordered_statistic_program`, `threshold_program`,
  `leaf_prediction_program`, and `write_split_program`.
- `training/src/forward.rs`: `causal_mask_program`,
  `head_major_source_index_program`, `sum_program`, `binary_program`,
  `add_program`, `subtract_program`, `multiply_program`, `divide_program`,
  `square_program`, `multiply_constant_program`,
  `divide_constant_program`, `constant_binary_program`,
  `gru_hidden_program`, `lstm_cell_program`, `z_score_program`,
  `min_max_program`, `l2_square_program`, `l2_norm_program`, and
  `huber_activation_program`.
- `training/src/inference.rs`: `zero_f32_program`,
  `checked_i32_to_f32_program`, `stable_sigmoid_exponent_program`,
  `stable_sigmoid_result_program`, `softmax_exponent_input_program`,
  `checked_one_hot_update_program`, and `feature_destination_program`.
- `training/src/gguf_llama.rs`: `symmetric_clamp_program`.
- `training/src/compile.rs`: `resume_tensor_program`,
  `tree_one_hot_target_program`, `tree_relative_node_program`,
  `tree_lightgbm_active_row_program`, `tree_lightgbm_candidate_choice_program`,
  `tree_lightgbm_positive_gain_program`, `tree_lightgbm_destination_program`,
  `tree_lightgbm_select_i32_program`, `tree_lightgbm_select_f32_program`,
  `tree_lightgbm_next_node_program`, `tree_candidate_index_program`,
  `repeat_f32_with_i32_program`, `tree_safe_mean_program`,
  `tree_left_weight_program`, `tree_parent_output_index_program`,
  `tree_variance_gain_program`, `tree_row_feature_index_program`,
  `tree_split_index_program`, `tree_feature_index_program`,
  `tree_next_node_program`, `tree_leaf_base_program`, `tree_leaf_index_program`,
  `sequence_major_source_index_program`, `convert_i32_program`,
  `constant_f32_from_i32_program`, `validity_program`,
  `zero_outputs_program`, `constant_parameter_program`,
  `group_routing_mask_program`, `gru_hidden_backward_program`,
  `gru_reset_product_backward_program`, `lstm_hidden_backward_program`,
  `lstm_cell_backward_program`, `masked_zero_f32_program`,
  `masked_zero_i32_program`, `negative_multiply_program`,
  `pointwise_loss_program`, `cross_entropy_with_logits_program`,
  `cross_entropy_with_dense_targets_program`, `signed_logarithm_backward_program`,
  `natural_logarithm_backward_program`, `signed_log_one_plus_backward_program`,
  `huber_activation_backward_program`, `tangent_activation_backward_program`,
  `kmeans_distance_gradient_scale_program`, `r2_program`, `safe_count_program`,
  `mean_scalar_values_program`, `inverse_standard_deviation_program`,
  `normalization_backward_program`, `masked_mean_program`,
  `clip_scale_program`, `schedule_inputs_program`,
  `unbounded_schedule_inputs_program`, `cosine_angle_program`,
  `cosine_decay_program`, `exponential_decay_argument_program`,
  `exponential_decay_program`, `learning_rate_program`,
  `constant_one_program`, `optimizer_update_gate_program`,
  `accepted_update_program`, `saturating_step_program`,
  `adam_beta_power_initial_program`, `adam_beta_power_update_program`,
  `adamw_program`, `temperature_gradient_program`, and
  `temperature_update_program`.

The non-constructor helper call sites are part of the same path. Math uses its
`require_*`, `preserve_zero`, `horner`, and `emit_*` helpers to keep one builder
alive while expanding a function. Bayes, tree, KMeans, KNN, binary metrics,
and materialization use `scalar_builder`, `scalar_input`, literal helpers,
`scalar_unary`, `scalar_binary`, `scalar_ternary`, and `scalar_finish`; their
domain helpers (`require_finite`, range predicates, `bool_mask`,
`require_nonnegative`, `require_positive`, `bias_correct`, and
`weighted_sum_four`) only compose those calls and never create a second scalar
representation. This is why a failure from one low-level builder call retains
the enclosing operation's error context.

## From a builder to a device kernel

The complete path is:

```text
ScalarProgramBuilder
    -> ScalarProgram (validated typed SSA)
    -> PrimitiveKind::Elementwise { program }
    -> PrimitiveKernel::validate
    -> recipe-primitives::lower::lower_elementwise
    -> KernelTemplate / StageKind::ScalarMap
    -> recipe-kernel::stage::lower_stage
    -> recipe-kernel::llvm::lower_elementwise
    -> target-specific LLVM IR and native artifact
```

### Primitive and graph validation

An elementwise program is stored in `language::Elementwise { program }` and is
normally attached to a `PrimitiveKernel`. `PrimitiveKernel::validate` calls
`ScalarProgram::validate`, then checks:

- the number of tensor inputs equals `program.inputs.len()`;
- each tensor input dtype equals its scalar input dtype;
- the output tensors match `program.outputs.len()` and each output dtype;
- the input shapes broadcast to the output shape; and
- the kernel's complete alias matrix is present.

The output checks use `ScalarProgram::dtype_of` to resolve each output ID
through the input, constant, and instruction definitions. That lookup is a
consumer-side type check; it does not change the builder's local handles.

`CalculationGraph::to_ogdl` validates this complete graph before encoding. The
strict OGDL codec writes every scalar input, constant, instruction, operand,
and output, using the same numeric IDs and bit-preserving literal forms. On
decode it requires the exact schema fields and opcode spellings, reconstructs
the `ScalarProgram`, and validates the graph before returning it.

### Primitive lowering

`primitives/src/lower.rs::lower_elementwise` clones the scalar program into a
`recipe_core::KernelTemplate`. It creates an index space from the output
shape, derives static buffer accesses for broadcast inputs and tensor outputs,
and preserves the program's input/output order. It also derives resource
bounds from the program:

- per-lane FLOPs are the sum of `ScalarOpcode::flops()` values;
- total FLOPs multiply that sum by output elements;
- integer-operation work is bounded by one operation per output element;
- atomic-operation work is bounded by one fault publication per output element
  when a fault flag is present, otherwise zero;
- private scalar storage is four bytes times input, constant, and instruction
  slots; and
- a scalar program with `requires_fault_flag()` receives a preallocated int32
  fault buffer and an arithmetic-domain fault contract (the current primitive
  lowering reserves fault code `2` for this contract before stage realization
  applies its immutable publication code).

The result is a `StageKind::ScalarMap` in the immutable lowered program. No
builder or expression survives this boundary.

`ScalarProgram::requires_fault_flag()` is true when an instruction is
`Require(I32)`, int32 `Divide`, int32 `Remainder`, int32 `Negate`, or int32
`Absolute`. Those are the operations whose device lowering must publish a
rejected calculation instead of allowing a device trap or undefined integer
behavior. `ScalarOpcode::flops()` counts conventional arithmetic and
comparisons, counts `Fma` as two, and deliberately counts selection,
bit-manipulation, classification, conversion, and `Require` as zero scheduling
FLOPs.

The planner treats the resulting program as identity-bearing data. Its scalar
stage template identity is derived from the lowered-program digest, source
kernel ID, and stage ordinal. The planner's canonical template hash includes
the index space, accesses, scalar inputs, literal bits, instruction opcodes
and operands, outputs, and alias rules. Primitive validation also requires
that the stage binding count and fault contract agree with
`program.requires_fault_flag()`, so a scalar program cannot be paired with a
different launch or fault ABI after lowering.

### LLVM lowering

`recipe-kernel::stage::lower_stage` validates the complete lowered program and
artifact build contract. For a scalar-map stage it calls
`recipe-kernel::llvm::lower_elementwise`, then rewrites the generic scalar
fault publication to the stage's immutable fault code.

The LLVM emitter validates the `KernelTemplate` again, emits target-specific
lane indexing and bounds checks, loads each scalar input through its static
buffer access, materializes constants from `ScalarLiteral` bits, and lowers
instructions in vector order. It then stores outputs in the program's output
order. The generated ABI is input buffers, output buffers, an optional fault
flag when checked instructions are present, and the element count.

The lowering preserves the core signatures. Floating arithmetic uses strict
IEEE-aware LLVM operations and canonicalizes NaN results. Int32 division and
remainder substitute a safe divisor when zero or `i32::MIN / -1` would be
invalid, then publish the fault. Int32 negate and absolute similarly guard
`i32::MIN`. `Require` converts a nonzero predicate to one and records a fault
for zero. Invalid instruction/opcode combinations are rejected by lowering,
but a program made through the public builder cannot reach those combinations
because `result_dtype` and `finish` enforce the same contract first.

## Minimal construction example

The following builds an f32 positive-value map with an explicit domain fault
and a selected fallback. The `?` operators stand for the caller's normal
`LanguageResult` or error conversion.

```rust
use recipe_core::{DType, ScalarOpcode};
use recipe_language::ScalarProgramBuilder;

let mut builder = ScalarProgramBuilder::new()?;
let value = builder.input(DType::F32)?;
let zero = builder.f32(0.0)?;
let positive = builder.binary(ScalarOpcode::GreaterThan, value, zero)?; // I32
builder.unary(ScalarOpcode::Require, positive)?;
let output = builder.ternary(ScalarOpcode::Select, positive, value, zero)?; // F32
let program = builder.finish(&[output])?;
```

The resulting program has one f32 input, one f32 constant, three
instructions, and one f32 output. The `Require` instruction means primitive
lowering allocates the arithmetic fault channel. The builder itself performs
no per-element execution; it only records the typed calculation that later
stages apply to every output element.
