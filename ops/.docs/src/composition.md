# `ops/src/composition.rs`: finite structured operation recipes

## Module identity

```text
crate: recipe-ops
source: ops/src/composition.rs
role: static inventory and validation for multi-stage operations
runtime work: none
payload contract: f32 and/or int32 only
execution boundary: ops/src/materialize.rs
authoritative inventory: operation-surface.txt plus Recipe-owned extensions
```

This module describes operations that need more than one primitive stage. A
`CompositionRecipe` records the operation name, human definition, ordered
primitive-family roles, canonical payload domain, and broad operation family.
It is an algorithm-shape contract, not a tensor program and not a backend
kernel. The strings and primitive families do not carry tensor names, shapes,
axes, scalar SSA programs, prepared constants, workspace formulas, or kernel
IDs. Those details are supplied by the concrete preparation code in
[`ops/src/materialize.rs`](../../src/materialize.rs).

The distinction is deliberate. The public operation surface is finite and
source-qualified, but preparation must reject a descriptor whose concrete ABI
or scalar formula has not been implemented. A descriptive recipe therefore
does not authorize a fallback to a legacy vendor routine, a host calculation,
or an inferred tensor layout.

The module is re-exported through `ops/src/lib.rs` and the root facade's
`operations` module. The public `operations::validate_composition` entrypoint
checks only the static recipe. `operations::materialize` is the later boundary
that receives immutable tensor declarations and typed preparation facts and
returns a validated `CalculationGraph` fragment.

## Public surface

`ops/src/lib.rs` re-exports the following declarations from this module:

| Declaration | Meaning |
|---|---|
| `CompositionPayload` | Canonical f32/int32 payload class used to populate the registry dtype contract. |
| `IterationBound` | A finite repeat bound resolved during preparation. |
| `CompositionStep` | One primitive-family stage or a statically bounded nested repeat. |
| `CompositionRecipe` | Immutable name, definition, steps, payload, and operation-family metadata. |
| `validate_composition` | Public descriptor-level validation entrypoint. |

`CompositionRecipe::for_entry` and `recipe` are crate-private. The registry
calls the former while constructing an `OperationDescriptor`; callers cannot
construct a recipe with arbitrary runtime strings because every shipped recipe
is a `const` definition in this file.

## Payload contract

`CompositionPayload` is intentionally coarser than a concrete tensor ABI. Its
`dtype_contract` method maps directly to the canonical registry contract:

| Variant | Registry contract | Typical use |
|---|---|---|
| `F32` | `CanonicalDTypeContract::F32Payload` | Numeric loss, normalization, optimizer, solver, and distance payloads. |
| `I32` | `CanonicalDTypeContract::I32Payload` | Index, route, count, mask, or representative payloads. |
| `F32AndI32` | `CanonicalDTypeContract::F32AndI32Payloads` | f32 values with int32 indices, labels, masks, routes, or metadata. |
| `F32OrI32` | `CanonicalDTypeContract::F32OrI32Payload` | Conversion or dequantization whose selected canonical representation is known only at preparation. |

The payload value does not permit arbitrary mixed arithmetic. Under the system
contract, only f32 and int32 values enter production calculation payloads.
Raw bytes, opaque quantized encodings, addresses, and metadata must be checked
and explicitly converted before a concrete emitter uses them.

## Static data model

### `IterationBound`

`IterationBound` says how a repeat count is obtained. It never describes a
data-dependent host loop:

| Variant | Resolution in `materialize.rs` | Failure condition |
|---|---|---|
| `Fixed(u32)` | Uses the literal count. | `Fixed(0)` is rejected by recipe validation. |
| `ShapeExtent { axis }` | Reads the selected `iteration_shape_input` tensor's extent at `axis`. | An absent axis produces `IterationBoundUnresolved`. |
| `MinimumShapeExtent` | Takes the minimum extent of the selected shape. | An empty shape has no minimum and produces `IterationBoundUnresolved`. |
| `CeilingLog2ShapeExtent { axis }` | Resolves the extent, then applies the integer ceiling-log2 helper. Extents zero and one resolve to zero. | An absent axis produces `IterationBoundUnresolved`. |
| `PreparedParameter { name }` | Looks up `PreparedParameters[name]` and requires `PreparedParameter::U64`. | A missing key produces `MissingPreparedParameter`; another variant produces `PreparedParameterTypeMismatch`. |

The selected shape is one immutable input named by the
`MaterializationRequest`. Shape bounds are therefore fixed before graph
emission. Prepared counts are also fixed typed facts, not values read from a
device buffer. `materialize.rs` records every resolved bound in
`ResolvedComposition::bounds` and records the surrounding iteration index and
count on each `ResolvedStep`.

### `CompositionStep`

The enum has exactly two forms:

```text
Primitive { family: PrimitiveFamily, role: &'static str }
Repeat    { bound: IterationBound, role: &'static str,
            body: &'static [CompositionStep] }
```

`CompositionStep::primitive` and `CompositionStep::repeat` are `const`
constructors. `role()` returns the role string for either form. Roles are
diagnostic and semantic documentation for the preparation graph; they are
also retained in `ResolvedStep` so an emitted stage can be traced back to the
recipe.

`PrimitiveFamily` is the backend-neutral vocabulary from `ops/src/primitive.rs`:

| Family | Role recorded by this module |
|---|---|
| `Elementwise` | Apply the operation-specific typed scalar SSA formula. |
| `Reduce` | Combine values with a fixed, recorded reduction tree. |
| `Scan` | Apply a fixed-tree prefix recurrence. |
| `Contraction` | Visit contracted coordinates in canonical order. |
| `Gather` | Perform checked int32-indexed reads. |
| `Scatter` | Perform checked writes with an explicit conflict policy. |
| `Histogram` | Accumulate into statically bounded bins. |
| `Sort` | Apply the stable total-order sorting network. |
| `Random` | Generate counter-keyed Philox4x32-10 values. |

`IndexMap` exists in `PrimitiveFamily` for direct primitive recipes, but this
composition inventory does not use an `IndexMap` step constant. Concrete
materializers may still emit an index-map kernel inside a stage when their
operation ABI requires it; the emitter checks the primary emitted primitive
against the declared family and keeps any additional same-stage kernels
internal to that emission.

### `CompositionRecipe`

The fields are private and immutable:

| Field | Meaning |
|---|---|
| `name` | Stable owned recipe identifier used in descriptors and the remaining-composition manifest. |
| `definition` | Human-readable operation meaning returned by `OperationDescriptor::definition`. |
| `steps` | Static ordered slice of `CompositionStep` values. |
| `payload` | One `CompositionPayload` value mapped to the canonical dtype contract. |
| `family` | Broad `OperationFamily` classification for registry reporting. |

The `name`, `definition`, `steps`, `payload`, and `operation_family` accessors
are `const` and copy the recipe metadata. `operation_family()` is registry
metadata, not a concrete dispatch key. Concrete dispatch uses exact
`(symbol, source)` pairs in materializer family modules.

## Validation and error behavior

`CompositionRecipe::validate` checks the recipe before any shape or tensor
work. It rejects an empty name, empty definition, or empty top-level step
slice with `OperationErrorKind::InvalidCompositionRecipe`, then recursively
checks every step:

1. Every primitive and repeat role must be nonempty.
2. Every repeat body must be nonempty.
3. A `Fixed(0)` bound is invalid.
4. A `PreparedParameter` bound must name a nonempty parameter.
5. Recursive nesting may not exceed eight levels. The implementation rejects
   a call when the current depth is greater than eight, so a valid tree may
   reach depth eight and may not descend to depth nine.

`validate_steps` does not validate tensor shapes, primitive parameters,
workspace, or scalar formulas. Those checks belong to concrete materializers.
The function also does not evaluate repeat counts, so a shape extent of zero
is not rejected here unless it is a literal `Fixed(0)`; concrete operation
ABIs decide whether an empty shape is legal.

`validate_composition(descriptor)` accepts only
`LoweringAvailability::Composition(recipe)`. It calls `recipe.validate()` and
attaches the descriptor's `OperationId` to any error. A scalar, direct
primitive, workspace, non-calculation, or unsupported descriptor returns
`WrongLoweringKind` with the detail `operation does not own a multi-stage
composition`, also carrying the operation ID.

No validation path silently repairs an invalid role, empty repeat body,
missing prepared count, or unsupported descriptor. `OperationError` is the
single error type and its `for_operation` method preserves the source-stable
operation identity for callers and diagnostics.

## Reusable stage templates

The first constants define short sequences reused by many entries. The names
below are source identifiers, not additional public operations:

| Sequence | Ordered families |
|---|---|
| `MAP_ONLY` | `Elementwise` |
| `MAP_REDUCE` | `Elementwise -> Reduce` |
| `MAP_REDUCE_MAP` | `Elementwise -> Reduce -> Elementwise` |
| `PAIRWISE_L2_STEPS` | `Elementwise -> Reduce -> Contraction -> Elementwise` |
| `MAP_SCAN` | `Elementwise -> Scan` |
| `MAP_SORT` | `Elementwise -> Sort` |
| `MAP_SORT_GATHER` | `Elementwise -> Sort -> Gather` |
| `MAP_HISTOGRAM` | `Elementwise -> Histogram` |
| `MAP_SCATTER` | `Elementwise -> Scatter` |
| `GATHER_MAP` | `Gather -> Elementwise` |
| `GATHER_MAP_REDUCE` | `Gather -> Elementwise -> Reduce` |
| `GATHER_MAP_SCATTER` | `Gather -> Elementwise -> Scatter` |
| `SORT_GATHER` | `Sort -> Gather` |
| `RANDOM_MAP` | `Random -> Elementwise` |
| `RANDOM_GATHER` | `Random -> Gather` |
| `RANDOM_SORT_GATHER` | `Random -> Sort -> Gather` |
| `SORT_RANDOM_GATHER` | `Sort -> Random -> Gather` |
| `CONTRACT_MAP` | `Contraction -> Elementwise` |
| `CONTRACT_MAP_REDUCE` | `Contraction -> Elementwise -> Reduce` |
| `REDUCE_MAP` | `Reduce -> Elementwise` |
| `REDUCE_MAP_REDUCE` | `Reduce -> Elementwise -> Reduce` |
| `SCAN_MAP_SCATTER` | `Scan -> Elementwise -> Scatter` |
| `HISTOGRAM_REDUCE_MAP` | `Histogram -> Reduce -> Elementwise` |
| `SORT_SCAN_MAP` | `Sort -> Scan -> Elementwise` |
| `SORT_SCAN_SCATTER` | `Sort -> Scan -> Scatter` |

The specialized sequences preserve a more informative role description than a
short alias. They are still only static stage inventories:

| Constant | Shape of the algorithm |
|---|---|
| `LINEAR_BACKWARD_FULL_STEPS` | Contract output gradient with transposed weights, contract transposed input with output gradient, then fixed-tree row reduction for bias. |
| `LINEAR_BACKWARD_WEIGHTS_ONLY_STEPS` | Weight contraction followed by fixed-tree bias reduction. |
| `SOFTMAX_STEPS` | Row maximum reduction, max-subtracted owned exponential, exponential-sum reduction, checked division. |
| `NORMALIZE_STEPS` | Statistic map, two fixed-order reductions, epsilon and affine normalization map. |
| `NORMALIZE_BACKWARD_STEPS` | Gradient-statistic map, two reductions, analytic gradient map, affine-gradient reduction. |
| `POOL_STEPS` | Checked window gather, fixed-order window reduction, average divisor or maximum tie policy. |
| `POOL_BACKWARD_STEPS` | Source-window gather, contribution map, explicit atomic scatter for overlap. |
| `CONV_STEPS` | Checked receptive-field gather, canonical contraction, optional bias or activation map. |
| `CONV_BACKWARD_STEPS` | Gradient receptive-field gather, canonical contraction, atomic overlap scatter. |
| `ATTENTION_STEPS` | Scaled query-key contraction, mask map, max reduction, exponential map, sum reduction, checked division, value contraction. |
| `ATTENTION_BACKWARD_STEPS` | Value/probability contractions, softmax-gradient reduction and map, then query, key, and value contractions. |
| `RNN_CELL_STEPS` | Input/recurrent affine contraction followed by gate map. |
| `OPTIMIZER_STEPS` | Optimizer-state recurrence followed by normalized parameter update. |
| `TREE_HISTOGRAM_STEPS` | Histogram, prefix scan, split scoring, lowest-index maximum-gain reduction. |
| `TREE_ROUTE_STEPS` | Checked feature/threshold gather, deterministic int32 branch map, nonconflicting assignment scatter. |
| `SEGMENT_REDUCE_STEPS` | Stable segment sort, boundary scan, fixed-tree segment reduction. |
| `COUNT_DISTINCT_STEPS` | Stable total-order sort, transition map, prefix scan, bounded unique-output scatter. |
| `FFT_BUTTERFLY_BODY` | Bit-reversed partner gather, owned twiddle and complex map, disjoint scatter. |
| `CHOLESKY_BODY` | Fixed-order pivot reduction, positivity check and pivot map, lower-panel scatter. |
| `TRIANGULAR_SOLVE_BODY` | Known-prefix reduction, diagonal check and solve map, solved-component scatter. |
| `LU_BODY` | Lowest-index absolute-pivot reduction, row-permutation gather, pivot map, trailing contraction. |
| `QR_BODY` | Householder norm reduction, signed reflector map, canonical contraction. |
| `JACOBI_BODY` | Stable rotation map, two-sided contraction, off-diagonal residual reduction. |
| `BORUVKA_BODY` | Endpoint gather, deterministic minimum-edge reduction, ordered atomic union scatter. |
| `UNION_FIND_BODY` | Parent-index gather, minimum-representative map, ordered compression scatter. |
| `DYNAMIC_PROGRAM_BODY` | Legal predecessor gather, transition map, fixed-tie reduction, next-state scatter. |
| `GENERATION_BODY` | Model/token/KV gather, model contraction, owned activation/normalization, token reduction, bounded state scatter. |
| `BOOST_TRAIN_BODY` | Gradient/Hessian map, feature-bin histogram, prefix scan, split score, lowest-index gain reduction, route gather, leaf update scatter. |
| `SMO_TRAIN_BODY` | KKT score map, deterministic working-set reduction, selected-row gather, coefficient and gradient map. |
| `BITONIC_COMPARE_STEPS` | One prepared sort compare-exchange distance and merge-width level. |
| `CROSS_ENTROPY_STEPS` | Max reduction, exponential map, sum reduction, target-logit gather, log-sum-exp loss map. |
| `DISTANCE_LOSS_STEPS` | Squared-difference map, fixed-order feature reduction, distance or margin map. |
| `TRIPLET_LOSS_STEPS` | Anchor-positive and anchor-negative maps and reductions, then prepared margin map. |
| `COSINE_EMBEDDING_STEPS` | Dot and norm maps, three fixed-order reductions, checked cosine/margin map. |
| `DENSE_ARGMAX_ACCURACY_STEPS` | Prediction and dense-target argmax reductions, index comparison, match reduction, reciprocal row-count map. |
| `LOG_SOFTMAX_STEPS` | Max reduction, exponential map, sum reduction, checked logarithmic output map. |
| `REPORT_STEPS` | Argmax reduction, class histogram, class-recall map, recall reduction, class-count division. |
| `LAMB_TRUST_STEPS` | Squared parameter/direction map, two norm reductions, checked trust-ratio update map. |

The repeat templates are also explicit in this file. `FFT_STEPS` repeats
`FFT_BUTTERFLY_BODY` for `CeilingLog2ShapeExtent { axis: 0 }`;
`CHOLESKY_STEPS` and `TRIANGULAR_SOLVE_STEPS` repeat their body for
`ShapeExtent { axis: 0 }`; `LU_STEPS`, `QR_STEPS`, and the first `SVD_STEPS`
repeat for `MinimumShapeExtent`; `EIGH_STEPS` uses `jacobi_sweeps`; the
second SVD repeat uses `bidiagonal_qr_sweeps`; Boruvka and union-find use
ceiling-log2 axis zero; dynamic programming uses `dynamic_program_steps`;
generation uses `maximum_generated_tokens`; boosting uses `boosting_rounds`;
and SMO uses `smo_iterations`. These names are exact prepared-parameter keys.

## Registry and source-qualified lookup

`ops/src/registry.rs` constructs every descriptor in this order:

```text
operation-surface.txt row or Recipe-owned extension
  -> describe(raw entry)
  -> ScalarRecipe, PrimitiveRecipe, WorkspaceFormula,
     NonCalculationRecipe, CompositionRecipe, or Unsupported
```

`lowering(symbol, source)` checks scalar, direct primitive, workspace,
non-calculation, and then `CompositionRecipe::for_entry(symbol, source)`.
For a composition descriptor, the registry obtains:

```text
definition = recipe.definition()
family     = recipe.operation_family()
dtypes     = recipe.payload().dtype_contract()
```

The registry retains duplicate symbols as separate `OperationDescriptor`
values. `resolve_unique(symbol)` therefore rejects an ambiguous symbol, while
`resolve_exact(symbol, source)` is the safe entrypoint for a source-qualified
legacy row. The recipe lookup itself is intentionally compact and static:
the main `composition_for_entry` match is symbol-first, and its `source`
argument is used only by the fallback `source_specific_composition` match.
Concrete materializers do not rely on this shortcut: their `supports` tables
require the exact `(descriptor.symbol, descriptor.source)` pair.

## Complete symbol-to-recipe map

The following table is the complete match inventory in source order. Aliases
on one row share one `CompositionRecipe`, one stage slice, one payload class,
and one broad operation family. `Definition` prose is kept in the source's
`recipe(...)` call; the table exposes the executable shape metadata needed to
trace the dispatch.

```text
symbols -> recipe ; steps ; payload ; family
convert, dequant_f32, gpu_convert -> checked_dequantization ; MAP_ONLY ; F32OrI32 ; Quantization
generate -> bounded_autoregressive_generation ; GENERATION_STEPS ; F32AndI32 ; Inference
gpu_accuracy_into -> binary_accuracy ; MAP_REDUCE_MAP ; F32AndI32 ; Metric
gpu_accuracy -> multiclass_correct_count ; REDUCE_MAP_REDUCE ; F32AndI32 ; Metric
gpu_argmax_accuracy_into -> dense_multiclass_accuracy ; DENSE_ARGMAX_ACCURACY_STEPS ; F32AndI32 ; Metric
gpu_adagrad_update -> adagrad_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_adam_update -> adam_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_adamw_update -> adamw_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_add_col -> column_add ; GATHER_MAP_SCATTER ; F32AndI32 ; ShapeAndIndexing
gpu_add_col_scaled_inplace -> scaled_column_accumulate ; GATHER_MAP_SCATTER ; F32AndI32 ; ShapeAndIndexing
gpu_add_diag -> diagonal_add ; GATHER_MAP_SCATTER ; F32AndI32 ; ShapeAndIndexing
gpu_argmax_write_split -> argmax_split_write ; REDUCE_MAP ; F32AndI32 ; Tree
gpu_argsort -> argsort ; SORT_GATHER ; F32AndI32 ; ShapeAndIndexing
gpu_avg_pool_1d, gpu_avg_pool_2d, gpu_avg_pool_2d_f32 -> average_pool ; POOL_STEPS ; F32 ; Pooling
gpu_avg_pool_2d_backward, gpu_avg_pool_2d_backward_f32 -> average_pool_backward ; POOL_BACKWARD_STEPS ; F32 ; Pooling
gpu_batchnorm_forward -> batch_normalization_training ; NORMALIZE_STEPS ; F32 ; Normalization
gpu_batchnorm_inference -> batch_normalization_inference ; MAP_ONLY ; F32 ; Normalization
gpu_batchnorm_backward -> batch_normalization_backward ; NORMALIZE_BACKWARD_STEPS ; F32 ; Normalization
gpu_bce_with_logits -> binary_cross_entropy_with_logits ; MAP_ONLY ; F32 ; Loss
gpu_bernoulli_into, gpu_bernoulli_u8 -> bernoulli_i32 ; RANDOM_MAP ; F32AndI32 ; Random
gpu_bernoulli_nb_logprob -> bernoulli_naive_bayes_log_probability ; MAP_REDUCE ; F32AndI32 ; Bayesian
gpu_bin_edges_quantile -> quantile_bin_edges ; SORT_GATHER ; F32AndI32 ; Encoding
gpu_bin_edges_uniform -> uniform_bin_edges ; MAP_ONLY ; F32AndI32 ; Encoding
gpu_bitonic_step, gpu_bitonic_step_dd, gpu_bitonic_step_idx -> sorting_network_compare_exchange ; BITONIC_COMPARE_STEPS ; F32AndI32 ; ShapeAndIndexing
gpu_bn_update_running -> batch_normalization_running_update ; MAP_ONLY ; F32 ; Normalization
gpu_bootstrap_sample -> bootstrap_sample ; RANDOM_GATHER ; F32AndI32 ; Tree
gpu_boruvka_mst -> boruvka_minimum_spanning_tree ; BORUVKA_STEPS ; F32AndI32 ; Clustering
gpu_candidate_generate -> frequent_item_candidate_generation ; SORT_SCAN_SCATTER ; F32AndI32 ; Other
gpu_categorical_logprob -> categorical_log_probability ; GATHER_MAP ; F32AndI32 ; ReinforcementLearning
gpu_causal_softmax_rows -> causal_row_softmax ; SOFTMAX_STEPS ; F32AndI32 ; Attention
gpu_centroid_update -> centroid_update ; GATHER_MAP_SCATTER ; F32AndI32 ; Clustering
gpu_cholesky -> cholesky_factorization ; CHOLESKY_STEPS ; F32 ; Solver
gpu_cholesky_solve, gpu_potrs -> cholesky_solve ; TRIANGULAR_SOLVE_STEPS ; F32 ; Solver
gpu_cholesky_inv -> cholesky_inverse ; TRIANGULAR_SOLVE_STEPS ; F32 ; Solver
gpu_col2im_1d, gpu_col2im_2d, gpu_col2im_2d_ext -> column_to_image ; GATHER_MAP_SCATTER ; F32AndI32 ; Convolution
gpu_concat_into, gpu_vconcat -> tensor_concatenation ; GATHER_MAP ; F32AndI32 ; ShapeAndIndexing
gpu_contrastive_loss -> contrastive_loss ; DISTANCE_LOSS_STEPS ; F32 ; Loss
gpu_conv1d_into -> convolution_1d ; CONV_STEPS ; F32 ; Convolution
gpu_conv1d_backward_data_into, gpu_conv1d_backward_filter_into, gpu_conv1d_backward_bias_into -> convolution_1d_backward ; CONV_BACKWARD_STEPS ; F32 ; Convolution
gpu_core_distance -> core_distance ; MAP_SORT_GATHER ; F32AndI32 ; Clustering
gpu_cosine_embedding_loss -> cosine_embedding_loss ; COSINE_EMBEDDING_STEPS ; F32AndI32 ; Loss
gpu_count_distinct, gpu_run_length -> stable_run_encoding ; COUNT_DISTINCT_STEPS ; F32AndI32 ; Encoding
gpu_cross_entropy -> cross_entropy ; CROSS_ENTROPY_STEPS ; F32AndI32 ; Loss
gpu_csr_spmv, gpu_csr_spmm -> csr_sparse_contraction ; GATHER_MAP_SCATTER ; F32AndI32 ; Graph
gpu_data_partition -> stable_data_partition ; SCAN_MAP_SCATTER ; F32AndI32 ; ShapeAndIndexing
gpu_degree -> graph_degree ; MAP_HISTOGRAM ; I32 ; Graph
gpu_diffusion_commit -> diffusion_commit ; MAP_ONLY ; F32AndI32 ; Diffusion
gpu_diffusion_sample -> diffusion_sample ; RANDOM_MAP ; F32 ; Diffusion
gpu_discounted_returns -> discounted_returns ; MAP_SCAN ; F32 ; ReinforcementLearning
gpu_dropout_u8_into -> canonical_i32_mask_dropout ; MAP_ONLY ; F32AndI32 ; Random
gpu_dasum -> canonical_f32_absolute_sum ; MAP_REDUCE ; F32 ; Reduction
gpu_dgemv_into -> canonical_f32_matrix_vector_product ; CONTRACT_MAP ; F32 ; Contraction
gpu_dger_into -> canonical_f32_rank_one_update ; CONTRACT_MAP ; F32 ; Contraction
gpu_dsyrk -> canonical_f32_symmetric_rank_k_update ; CONTRACT_MAP ; F32 ; Contraction
gpu_dtw -> dynamic_time_warping ; DYNAMIC_PROGRAM_STEPS ; F32AndI32 ; Distance
gpu_eigh_sym -> symmetric_eigendecomposition ; EIGH_STEPS ; F32 ; Solver
gpu_embed_blend -> embedding_blend ; GATHER_MAP ; F32AndI32 ; Embedding
gpu_embedding_backward -> embedding_backward ; MAP_SCATTER ; F32AndI32 ; Embedding
gpu_entropy_gated_step -> entropy_gated_diffusion_step ; MAP_REDUCE ; F32AndI32 ; Diffusion
gpu_eye -> identity_matrix ; MAP_ONLY ; F32AndI32 ; Creation
gpu_feature_subset -> feature_subset ; GATHER_MAP ; F32AndI32 ; Tree
gpu_fft_c2c_1d, gpu_rfft_1d -> stockham_fft_1d ; FFT_STEPS ; F32AndI32 ; Fft
gpu_fill_sentinel -> sort_padding_fill ; MAP_ONLY ; F32AndI32 ; Creation
gpu_fixed_radius_neighbors -> fixed_radius_neighbors ; SCAN_MAP_SCATTER ; F32AndI32 ; Clustering
gpu_flash_attention_into, gpu_flash_gqa, gpu_flash_mla -> tiled_online_attention ; ATTENTION_STEPS ; F32AndI32 ; Attention
gpu_flash_attention_train_into -> tiled_online_attention_training ; ATTENTION_STEPS ; F32AndI32 ; Attention
gpu_flash_attention_backward_into -> tiled_online_attention_backward ; ATTENTION_BACKWARD_STEPS ; F32AndI32 ; Attention
gpu_focal_into -> focal_loss_and_gradient ; MAP_ONLY ; F32 ; Loss
gpu_focal_grad_into -> focal_loss_gradient ; MAP_ONLY ; F32 ; Loss
gpu_forward_backward -> forward_backward_sequence ; DYNAMIC_PROGRAM_STEPS ; F32AndI32 ; Sequence
gpu_gae -> generalized_advantage_estimation ; MAP_SCAN ; F32 ; ReinforcementLearning
gpu_gated_delta_scan -> gated_delta_scan ; MAP_SCAN ; F32 ; StateSpace
gpu_gaussian_ll, gpu_gaussian_logprob -> gaussian_log_probability ; MAP_REDUCE ; F32 ; Bayesian
gpu_gcn_norm -> gcn_edge_normalization ; GATHER_MAP ; F32AndI32 ; Graph
gpu_gemm_bt_tiles -> tiled_transposed_gemm ; CONTRACT_MAP ; F32 ; Contraction
gpu_goss_sample -> gradient_one_side_sampling ; SORT_RANDOM_GATHER ; F32AndI32 ; Tree
gpu_grad_clip_norm -> global_gradient_norm_clip ; MAP_REDUCE_MAP ; F32 ; Optimizer
gpu_grad_hess_into, gpu_logloss_grad_f32, gpu_logloss_grad_mc -> logistic_gradient_hessian ; MAP_ONLY ; F32AndI32 ; Tree
gpu_gru_cell, gpu_gru_cell_f32 -> gru_cell ; RNN_CELL_STEPS ; F32 ; Recurrent
gpu_has_nan -> any_nan ; MAP_REDUCE ; F32AndI32 ; Metric
gpu_isfinite_all -> all_finite ; MAP_REDUCE ; F32AndI32 ; Metric
gpu_hinge_loss -> hinge_loss ; MAP_ONLY ; F32 ; Loss
gpu_histogram_build -> gradient_hessian_histogram ; MAP_HISTOGRAM ; F32AndI32 ; Histogram
gpu_im2col_1d, gpu_im2col_2d, gpu_im2col_2d_ext -> image_to_column ; GATHER_MAP ; F32AndI32 ; Convolution
gpu_idamax -> canonical_f32_absolute_argmax ; MAP_REDUCE ; F32AndI32 ; Reduction
gpu_init_idx, gpu_iota -> int32_iota ; MAP_ONLY ; I32 ; Creation
gpu_itemset_support -> itemset_support ; MAP_REDUCE ; F32AndI32 ; Other
gpu_kernel_matrix, gpu_smo_kernel_row -> svm_kernel_matrix ; CONTRACT_MAP ; F32AndI32 ; SupportVectorMachine
gpu_kl_div_loss -> kl_divergence_loss ; MAP_ONLY ; F32 ; Loss
gpu_l2_norm -> l2_norm ; MAP_REDUCE ; F32 ; Reduction
gpu_l2norm_rows -> row_l2_normalization ; MAP_REDUCE_MAP ; F32 ; Normalization
gpu_lamb_phase1 -> lamb_moment_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_lamb_phase2 -> lamb_trust_ratio_update ; LAMB_TRUST_STEPS ; F32 ; Optimizer
gpu_layernorm_f32, gpu_layernorm_into, gpu_layernorm_opt_into -> layer_normalization ; NORMALIZE_STEPS ; F32 ; Normalization
gpu_layernorm_backward_f32, gpu_layernorm_backward_full_into -> layer_normalization_backward ; NORMALIZE_BACKWARD_STEPS ; F32 ; Normalization
gpu_leaf_finalize, gpu_lgbm_leaf_reduce -> regularized_leaf_value ; HISTOGRAM_REDUCE_MAP ; F32AndI32 ; Tree
gpu_leaf_reduce, gpu_scatter_add_by_leaf, gpu_scatter_add_by_leaf_col -> leaf_statistic_accumulation ; MAP_SCATTER ; F32AndI32 ; Tree
gpu_leaf_split_apply -> leaf_split_apply ; TREE_ROUTE_STEPS ; F32AndI32 ; Tree
gpu_lgbm_best_split, gpu_oblivious_split_eval, gpu_split_eval, gpu_tb_split_eval -> best_histogram_split ; TREE_HISTOGRAM_STEPS ; F32AndI32 ; Tree
gpu_lgbm_hist_subtract -> histogram_subtraction ; MAP_ONLY ; F32AndI32 ; Tree
gpu_lgbm_histogram, gpu_oblivious_histogram, gpu_tb_histogram -> tree_gradient_histogram ; MAP_HISTOGRAM ; F32AndI32 ; Tree
gpu_linear_into, gpu_linear_f32, gpu_matvec_bias_into -> linear_projection ; CONTRACT_MAP ; F32 ; Contraction
gpu_linear_backward_full_into -> linear_projection_backward ; LINEAR_BACKWARD_FULL_STEPS ; F32 ; Contraction
gpu_linear_backward_weights_only_into -> linear_projection_backward_weights_only ; LINEAR_BACKWARD_WEIGHTS_ONLY_STEPS ; F32 ; Contraction
gpu_lion_update -> lion_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_log_det_cholesky -> cholesky_log_determinant ; GATHER_MAP_REDUCE ; F32AndI32 ; Solver
gpu_log_softmax_rows -> row_log_softmax ; LOG_SOFTMAX_STEPS ; F32 ; Normalization
gpu_log_sum_exp_rows -> row_log_sum_exp ; REDUCE_MAP_REDUCE ; F32 ; Reduction
gpu_lstm_cell, gpu_lstm_cell_f32 -> lstm_cell ; RNN_CELL_STEPS ; F32 ; Recurrent
gpu_lu_factor -> lu_factorization ; LU_STEPS ; F32AndI32 ; Solver
gpu_lu_solve, gpu_solve -> pivoted_lu_solve ; TRIANGULAR_SOLVE_STEPS ; F32AndI32 ; Solver
gpu_max_pool_1d, gpu_max_pool_2d, gpu_max_pool_2d_f32 -> maximum_pool ; POOL_STEPS ; F32AndI32 ; Pooling
recipe_max_pool_1d -> channelwise_maximum_pool_1d ; POOL_STEPS ; F32AndI32 ; Pooling
recipe_max_pool_1d_backward -> channelwise_maximum_pool_1d_backward ; POOL_BACKWARD_STEPS ; F32AndI32 ; Pooling
gpu_max_pool_1d_backward, gpu_max_pool_2d_backward, gpu_max_pool_2d_backward_f32 -> maximum_pool_backward ; POOL_BACKWARD_STEPS ; F32AndI32 ; Pooling
gpu_mean_all -> global_mean ; REDUCE_MAP ; F32 ; Statistics
gpu_mha_split -> multi_head_attention_split ; GATHER_MAP ; F32AndI32 ; Attention
gpu_mha_merge -> multi_head_attention_merge ; GATHER_MAP ; F32AndI32 ; Attention
gpu_moe_route -> mixture_of_experts_route ; SORT_GATHER ; F32AndI32 ; MixtureOfExperts
gpu_moe_weighted_accumulate -> mixture_of_experts_accumulate ; GATHER_MAP_SCATTER ; F32AndI32 ; MixtureOfExperts
gpu_moe_weighted_accumulate_backward, gpu_moe_backward -> mixture_of_experts_backward ; GATHER_MAP_SCATTER ; F32AndI32 ; MixtureOfExperts
gpu_momentum_update -> momentum_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_mse_into -> mean_squared_error ; MAP_REDUCE_MAP ; F32 ; Loss
gpu_ss_res_into -> sum_squared_residuals ; MAP_REDUCE ; F32 ; Loss
gpu_multinomial_nb_logprob -> multinomial_naive_bayes_log_probability ; MAP_REDUCE ; F32AndI32 ; Bayesian
gpu_nadam_update -> nadam_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_nb_count_table -> naive_bayes_count_table ; MAP_HISTOGRAM ; F32AndI32 ; Bayesian
gpu_nb_feature_log_prob -> naive_bayes_feature_log_probability ; REDUCE_MAP ; F32 ; Bayesian
gpu_neighbor_aggregate -> graph_neighbor_aggregation ; GATHER_MAP_SCATTER ; F32AndI32 ; Graph
gpu_one_hot -> one_hot_encoding ; MAP_SCATTER ; F32AndI32 ; Encoding
gpu_oob_mask -> out_of_bag_mask ; MAP_HISTOGRAM ; I32 ; Tree
gpu_ordered_target_stats -> ordered_target_statistics ; SORT_SCAN_MAP ; F32AndI32 ; Tree
gpu_pack_upper_tri -> pack_upper_triangle ; GATHER_MAP ; F32AndI32 ; ShapeAndIndexing
gpu_pairwise_cosine -> pairwise_cosine_distance ; CONTRACT_MAP_REDUCE ; F32 ; Distance
gpu_pairwise_hamming -> pairwise_hamming_distance ; MAP_REDUCE ; F32AndI32 ; Distance
gpu_pairwise_l1 -> pairwise_l1_distance ; MAP_REDUCE ; F32 ; Distance
gpu_pairwise_l2 -> pairwise_l2_distance ; PAIRWISE_L2_STEPS ; F32 ; Distance
gpu_partial_argsort, gpu_topk_per_row -> bounded_topk_indexes ; SORT_GATHER ; F32AndI32 ; ShapeAndIndexing
gpu_pool_grad_expand -> pool_gradient_expand ; POOL_BACKWARD_STEPS ; F32AndI32 ; Pooling
gpu_positional_encoding -> sinusoidal_positional_encoding ; MAP_ONLY ; F32AndI32 ; Embedding
gpu_qr -> householder_qr ; QR_STEPS ; F32 ; Solver
gpu_quantize_features -> feature_bin_quantization ; GATHER_MAP ; F32AndI32 ; Quantization
gpu_random_permutation -> counter_keyed_permutation ; RANDOM_SORT_GATHER ; I32 ; Random
gpu_random_threshold_split -> random_threshold_split ; RANDOM_MAP ; F32AndI32 ; Tree
gpu_reduce_mean_cols -> column_mean ; REDUCE_MAP ; F32 ; Statistics
gpu_reduce_var_cols -> column_variance ; REDUCE_MAP_REDUCE ; F32 ; Statistics
gpu_repeat_rows -> repeat_rows ; GATHER_MAP ; F32AndI32 ; ShapeAndIndexing
gpu_report -> balanced_recall_report_metric ; REPORT_STEPS ; F32AndI32 ; Metric
gpu_rmsnorm, gpu_rmsnorm_f64, gpu_rmsnorm_f64_nogamma -> canonical_f32_rms_normalization ; NORMALIZE_STEPS ; F32 ; Normalization
gpu_rmsnorm_backward -> rms_normalization_backward ; NORMALIZE_BACKWARD_STEPS ; F32 ; Normalization
gpu_rmsprop_update -> rmsprop_update ; OPTIMIZER_STEPS ; F32 ; Optimizer
gpu_rope, gpu_rope_partial, gpu_rope_partial_factors, gpu_rope_partial_factors_pos, gpu_rope_partial_pos, gpu_rope_qk, gpu_rope_qk_heads_inplace -> rotary_position_embedding ; GATHER_MAP ; F32AndI32 ; Embedding
gpu_scaled_dot_product_attn -> scaled_dot_product_attention ; ATTENTION_STEPS ; F32AndI32 ; Attention
gpu_scan_linear_recurrence -> affine_linear_recurrence_scan ; MAP_SCAN ; F32 ; Scan
gpu_segment_max, gpu_segment_sum -> segmented_reduction ; SEGMENT_REDUCE_STEPS ; F32AndI32 ; Reduction
gpu_segment_sort -> segmented_sort ; MAP_SORT ; F32AndI32 ; ShapeAndIndexing
gpu_slice_cols, gpu_slice_rows, gpu_slice_lead_into -> checked_tensor_slice ; GATHER_MAP ; F32AndI32 ; ShapeAndIndexing
gpu_smo_argmax -> smo_working_set_argmax ; MAP_REDUCE ; F32AndI32 ; SupportVectorMachine
gpu_smo_kkt_score -> smo_kkt_score ; MAP_ONLY ; F32AndI32 ; SupportVectorMachine
gpu_smo_update_gradient_rows -> smo_gradient_update ; GATHER_MAP ; F32AndI32 ; SupportVectorMachine
gpu_smo_train -> bounded_smo_training ; SMO_TRAIN_STEPS ; F32AndI32 ; SupportVectorMachine
gpu_softmax_rows_into, gpu_softmax_inplace -> row_softmax ; SOFTMAX_STEPS ; F32 ; Normalization
gpu_softmax_backward_into -> softmax_backward ; REDUCE_MAP ; F32 ; Normalization
gpu_softmax_ce_class_grad_f32, gpu_softmax_ce_grad_into -> softmax_cross_entropy_gradient ; SOFTMAX_STEPS ; F32AndI32 ; Loss
gpu_sort_by_key -> stable_key_value_sort ; MAP_SORT ; F32AndI32 ; ShapeAndIndexing
gpu_splitk_dw_into -> split_k_weight_gradient ; CONTRACT_MAP_REDUCE ; F32 ; Contraction
gpu_ssm_conv_causal, gpu_ssm_conv_causal_silu -> state_space_causal_convolution ; CONV_STEPS ; F32AndI32 ; StateSpace
gpu_ssm_group_rmsnorm -> state_space_group_rms_normalization ; NORMALIZE_STEPS ; F32 ; StateSpace
gpu_ssm_scan_mamba1, gpu_ssm_scan_mamba2 -> mamba_state_space_scan ; MAP_SCAN ; F32 ; StateSpace
gpu_svd -> singular_value_decomposition ; SVD_STEPS ; F32 ; Solver
gpu_tb_apply_tree, gpu_tree_ensemble_predict -> tree_ensemble_inference ; TREE_ROUTE_STEPS ; F32AndI32 ; Tree
gpu_tb_leaf_sum, gpu_tb_leaf_val -> tree_builder_leaf_statistic ; HISTOGRAM_REDUCE_MAP ; F32AndI32 ; Tree
gpu_tb_repartition, gpu_tb_scatter -> tree_builder_stable_partition ; SCAN_MAP_SCATTER ; F32AndI32 ; Tree
gpu_td_targets -> temporal_difference_targets ; GATHER_MAP ; F32AndI32 ; ReinforcementLearning
gpu_transpose -> tensor_transpose ; GATHER_MAP ; F32AndI32 ; ShapeAndIndexing
gpu_tri_solve -> triangular_solve ; TRIANGULAR_SOLVE_STEPS ; F32 ; Solver
gpu_tril_mask -> lower_triangular_mask ; MAP_ONLY ; F32AndI32 ; ShapeAndIndexing
gpu_triplet_loss -> triplet_margin_loss ; TRIPLET_LOSS_STEPS ; F32 ; Loss
gpu_union_find_cc -> union_find_connected_components ; UNION_FIND_STEPS ; I32 ; Clustering
gpu_upsample_nearest_2d -> nearest_neighbor_upsample ; GATHER_MAP ; F32AndI32 ; Pooling
gpu_vae_backward_latent -> vae_latent_backward ; MAP_ONLY ; F32 ; Diffusion
gpu_viterbi -> viterbi_decode ; DYNAMIC_PROGRAM_STEPS ; F32AndI32 ; Sequence
gpu_write_split -> write_split_metadata ; MAP_ONLY ; I32 ; Tree
```

The four source-sensitive arms are separate from the symbol-only table:

| Symbol arm | Source condition | Recipe and shape |
|---|---|---|
| `greedy`, `greedy_windowed` | Any source reaching this fallback | `greedy_token_selection`, `MAP_REDUCE`, `F32AndI32`, `Inference`. |
| `last_logits` | Any source reaching this fallback | `last_token_logits`, `GATHER_MAP`, `F32AndI32`, `Inference`. |
| `predict`, `predict_proba` | Source contains `catboost`, `lightgbm`, or `xgboost` | `catboost_inference`, `lightgbm_inference`, or `xgboost_inference`; `TREE_ROUTE_STEPS`, `F32AndI32`, `Tree`. Other sources return `None`. |
| `train`, `train_multiclass` | Source contains `catboost`, `lightgbm`, or `xgboost` | `catboost_training`, `lightgbm_training`, or `xgboost_training`; `BOOST_TRAIN_STEPS`, `F32AndI32`, `Tree`. Other sources return `None`. |

The source-sensitive recipes retain one shared algorithm shape while changing
the stable recipe name to preserve model-source identity. The source-specific
match is the final fallback, so a symbol that already matches the main table
never reaches it.

## Preparation and concrete execution boundary

The descriptor lifecycle is:

```text
operation_registry().resolve_exact(symbol, source)
  -> OperationDescriptor { lowering: Composition(recipe), ... }
  -> validate_composition(descriptor)
  -> materialize_composition(MaterializationRequest)
  -> expand_composition(descriptor, iteration_shape, parameters)
  -> exact source-qualified family dispatch
  -> CalculationGraph validation and workspace accounting
  -> primitive lowering and scheduled init/loop/exit execution
```

### Expansion

`expand_composition` first requires `LoweringAvailability::Composition`, runs
the recipe validator, and recursively walks its static steps. For a primitive
it appends a `ResolvedStep` with an ordinal, family, role, surrounding repeat
iterations, and a dependency on the preceding ordinal. For a repeat it:

1. resolves the bound from the selected shape or prepared parameter;
2. records a `ResolvedBound` with the bound kind, resolved count, and outer
   iteration context;
3. clones the current iteration stack and appends one
   `ResolvedIteration { role, index, count }` for each index; and
4. recursively expands the body.

The expansion is a finite dependency chain. It has no callback, no dynamic
host payload loop, and no data-dependent control edge. `MAX_EXPANDED_STEPS`
in `materialize.rs` is one million; reaching that limit returns
`CompositionExpansionOverflow` before graph emission.

### Concrete materializers

`materialize_composition` performs request validation, rejects a recipe absent
from the concrete support manifest with `MissingConcreteFormula`, selects the
named iteration-shape input, expands the recipe, and then dispatches to the
concrete emitter. The source file calls eleven family modules in order:

| Module | Responsibility |
|---|---|
| `optimizer_normalization` | Optimizer recurrences and batch, layer, and RMS normalization. |
| `solver_fft` | Radix-two FFT and triangular solve. |
| `attention_sequence_embedding` | Causal softmax, MHA split/merge, embeddings, positional tables, row repetition, and rotary embeddings. |
| `convolution_pooling` | Receptive-field transforms and average/max/channelwise pooling. |
| `loss_metrics` | Losses, distance functions, finite metrics, and reductions used by them. |
| `indexing_sort_encoding` | Checked indexing, sorting, slicing, concatenation, and one-hot/edge operations. |
| `graph_cluster_rl` | Graph, clustering, and reinforcement-learning emitters. |
| `tree_boosting` | Histograms, split selection, routing, leaves, and tree updates. |
| `inference_quantization_diffusion` | Reserved family boundary; it currently reports `NotOwned`. |
| `creation_shape_misc` | Reserved family boundary; it currently reports `NotOwned`. |
| `training` | Linear projection and binary-cross-entropy materializers. |

Each concrete module owns an exact `OPERATIONS: &[(&str, &str)]` table. A
symbol-only recipe match is therefore not proof of materializability. If no
module claims the exact pair, the operation appears in
`remaining_composition_manifest()` with all four missing components:
`TensorAbi`, `ScalarFormula`, `PrimitiveParameters`, and `WorkspacePolicy`.

`Emitter::emit` and `Emitter::emit_stage` consume the resolved steps in order.
For each stage, the primary emitted kernel's `PrimitiveKind` is converted back
to a `PrimitiveFamily` and compared with the resolved recipe family. A mismatch
or an emitted kernel count different from the resolved step count is
`GraphMaterializationFailed`. A stage may emit multiple concrete kernels when
the materializer needs one shared recipe stage to produce several outputs, but
the final primary family must still match.

The emitter allocates intermediate `ValueId`s and `KernelTemplateId`s only
inside the caller-provided `IdentityNamespace`, charges every intermediate to
the checked workspace allocation, and validates the resulting
`CalculationGraph`. The recipe itself does not allocate IDs or define alias
rules. Concrete emitters apply the complete alias matrix and the primitive
families' checked bounds, atomic policy, reduction tree, and scalar SSA.

## Callers and end-to-end role

The root `src/facade.rs` exposes this module through `operations`:

```rust
operations::validate_composition(descriptor)
operations::materialize(request)
operations::remaining_compositions()
```

The facade performs no semantic work of its own. It delegates to `recipe_ops`
so advanced callers and the normal compilers use one implementation.

The training and inference graph compilers call their local `materialize`
helpers. Each helper:

1. looks up the existing tensors and marks cloned inputs as external inputs
   and cloned outputs as external outputs;
2. assigns a reserved value and kernel identity range;
3. resolves the descriptor through `operation_registry().resolve_unique`;
4. builds `NamedTensor` declarations and `PreparedParameters`;
5. invokes `materialize_composition(MaterializationRequest::new(...))`; and
6. inserts the returned tensor contracts and calculation nodes into the
   compiler graph, preserving the caller's iteration domain.

`training/src/compile.rs` uses the result in training-domain graphs and
`training/src/inference.rs` uses one-shot inference-domain graphs. The
materialized graph then enters the ordinary Recipe language validation,
primitive lowering, static program scheduling, and native executor lifecycle.
This keeps all payload calculations on GPU f32/int32 paths and leaves the
host responsible only for declaration, preparation, graph construction, and
orchestration. The composition repeats are preparation-time unrolling, not
extra host loops in the finalized `init -> loop -> exit` program.

## Error and fail-closed contract

The relevant errors come from both this module and its preparation caller:

| Error | Boundary and cause |
|---|---|
| `WrongLoweringKind` | Descriptor is not a `Composition` when validation, expansion, or materialization requires one. |
| `InvalidCompositionRecipe` | Empty metadata, empty role/body, zero fixed bound, empty prepared key, or excessive nesting. |
| `IterationBoundUnresolved` | Shape axis is absent or a minimum is requested for an empty shape. |
| `MissingPreparedParameter` | A repeat or concrete ABI requires a named parameter that was not supplied. |
| `PreparedParameterTypeMismatch` | A repeat count is not `U64`, or a concrete parameter has the wrong typed variant. |
| `CompositionExpansionOverflow` | Unrolling would reach the one-million resolved-step limit. |
| `MissingConcreteFormula` | A valid descriptive recipe has no exact concrete materializer. |
| `InvalidMaterializationRequest` | Boundary declarations, external flags, names, shapes, parameters, or iteration-shape selection are invalid. |
| `UnsupportedConcreteShape` | The exact materializer rejects the requested shape even though the descriptor is known. |
| `GraphMaterializationFailed` | Scalar/primitive construction, family matching, graph validation, producer uniqueness, or workspace emission fails. |
| `IdentityNamespaceOverlap` / `IdentityNamespaceExhausted` | Caller reservations or emitted IDs overlap or overflow. |
| `WorkspaceLimitExceeded` / `WorkspaceFormulaMismatch` | Concrete intermediates exceed the caller limit or disagree with the module's checked accounting. |

The failure is visible at the real boundary. No recipe falls through to a
legacy kernel, CPU implementation, implicit conversion, guessed ABI, retry,
or substitute state.

## Authoritative references

- [`ops/src/composition.rs`](../../src/composition.rs), lines 6-185: payload,
  bound, step, recipe, and validation types.
- [`ops/src/composition.rs`](../../src/composition.rs), lines 187-944:
  primitive vocabulary, reusable stage sequences, and specialized repeated
  bodies.
- [`ops/src/composition.rs`](../../src/composition.rs), lines 946-2675:
  recipe constructor, symbol inventory, and source-sensitive fallback.
- [`ops/src/registry.rs`](../../src/registry.rs): descriptor construction,
  canonical dtype/family metadata, duplicate source-qualified identities, and
  lowering precedence.
- [`ops/src/materialize.rs`](../../src/materialize.rs): request validation,
  repeat expansion, concrete dispatch, identity/workspace accounting, graph
  emission, and remaining-composition manifest.
- [`ops/MATERIALIZATION.md`](../../MATERIALIZATION.md): concrete ABI and graph
  contracts for materialized operations.
- [`operation-surface.txt`](../../../operation-surface.txt): preserved finite
  symbol and legacy-source inventory.
- [`src/facade.rs`](../../../src/facade.rs): root `operations` delegation.
- [`training/src/compile.rs`](../../../training/src/compile.rs) and
  [`training/src/inference.rs`](../../../training/src/inference.rs): production
  compiler callers that reserve identities, invoke materialization, and merge
  the resulting graph.
- [`system-contract.md`](../../../system-contract.md), C13, C17, C18, and C21:
  finite operation ownership, GPU-only calculation placement, f32/int32
  payloads, fixed reduction order, explicit atomic policy, and counter-based
  randomness.
