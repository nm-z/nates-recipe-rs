# Structured operation materialization

`CompositionRecipe` is an inventory and algorithm-shape contract. Its
`PrimitiveFamily` and role strings are not executable tensor semantics.
`materialize_composition` is the fail-closed preparation boundary that supplies
those semantics:

- input and output tensors are immutable, named `Tensor` declarations;
- the caller selects one declared input whose shape resolves shape-dependent
  `IterationBound` values;
- `PreparedParameters` contains only typed values and verified preparation
  facts;
- every repeat is expanded before execution, with a hard one-million-step
  preparation limit;
- every emitted intermediate is included in a checked `WorkspaceAllocation`;
- each resolved step records the concrete kernel IDs that implement it;
- the resulting `CalculationGraph` is validated before it is returned.

There is no callback, dynamic blueprint, or host payload loop in the result.
`remaining_composition_manifest()` returns every source-qualified composition
that still lacks enough information to cross this boundary.

Concrete dispatch is partitioned by operation family. Shared request
validation, identity allocation, graph emission, and workspace accounting stay
in `materialize.rs`; source-qualified ownership lives in the ten declared
family modules:

- optimizer and normalization;
- solver and FFT;
- attention, sequence, and embedding;
- convolution and pooling;
- loss and metrics;
- indexing, sort, and encoding;
- graph, clustering, and reinforcement learning;
- tree and boosting;
- inference, quantization, and diffusion;
- creation, shape, and miscellaneous operations.

An operation can be owned by only its exact `(symbol, source)` pair. A symbol
match from a different legacy source remains in the manifest instead of
falling through to a semantically adjacent implementation.

## Fragment identity

Every production `MaterializationRequest` carries an explicit
`IdentityNamespace`. Its half-open intermediate-value and kernel ranges are
reserved by the caller before preparation. Declared inputs and outputs may not
occupy the intermediate range, allocation beyond either capacity fails with
`IdentityNamespaceExhausted`, and
`validate_identity_namespaces` rejects overlapping fragment reservations.

`MaterializedComposition::identity_namespace` preserves that reservation for
the caller. Independently prepared fragments can therefore be passed to
`CalculationGraph::assemble` without local value or kernel IDs colliding.
There is no implicit “start after the largest input” or “kernel one” production
path.

## Concrete ABIs

All index primitives use `IndexBounds::Reject`. All scatter tables described as
unique are preparation facts and lower to `ScatterConflict::UniqueIndices`.
The optimizer and normalization ABIs below are source-qualified. Their tensor
name sets and prepared-parameter name sets are exact: missing or additional
names are rejected.

### `gpu_fft_c2c_1d`

`real` and `imaginary` are rank-one, equal-length f32 inputs. The length is a
power of two and fits int32 indexing. Outputs are `transformed_real` and
`transformed_imaginary` with the same shape.

For each resolved radix-2 stage `s`, preparation supplies:

- `stage_s_left_indices` and `stage_s_right_indices`;
- eight f32 coefficient vectors named
  `stage_s_{real|imaginary}_from_{left|right}_{real|imaginary}`;
- `stage_s_real_base`, `stage_s_imaginary_base`, and
  `stage_s_destination_indices`.

The facts `fft_stage_tables_verified` and
`fft_destination_indices_unique` must be true. Four gathers read the two
complex operands, one two-output scalar SSA program applies the prepared
butterfly coefficients in fixed FMA order, and two unique scatters publish the
stage. Thus every logarithmic recipe iteration is a finite data dependency,
not a runtime host loop.

### `gpu_tri_solve`

`solution_base` and output `solution` are equal, nonempty rank-one f32 tensors.
For each statically resolved row `r`, preparation supplies
`row_r_strict_prefix`, `row_r_rhs`, `row_r_diagonal`, and
`row_r_destination`. The facts `solution_base_zero`,
`triangular_rows_verified`, and `destination_indices_unique` must be true;
`tree_lanes` fixes the reduction tree.

Each row maps the verified strict-prefix coefficients against the previous
solution image, reduces them in fixed order, performs a checked nonzero-diagonal
division, and scatters the new component. The final row produces the external
solution.

### `gpu_mha_split`

`packed` is f32, `indices` is int32, and `heads` has the checked gather-result
shape at prepared parameter `axis`. A gather plus explicit identity scalar
program emits the prepared head view.

## Attention, sequence, and embedding materializers

Every new ABI in this section has an exact tensor-name set and an exact typed
prepared-parameter set. Dimensions are nonzero and fit the legacy int32 launch
domain. Prepared index and coefficient tensors are immutable init-time data;
their verification facts mean the compiler generated them from the stated
legacy formulas. Runtime gathers still use `IndexBounds::Reject`.

### Attention head merge and sequence repetition

`gpu_mha_merge` consumes flattened f32 `heads` and int32 `merge_indices`, and
produces flattened f32 `packed`. Parameters are `seq`, `n_heads`, `head_dim`,
and true `merge_indices_verified`. All three tensors contain exactly
`seq*n_heads*head_dim` elements. The verified indices encode

```text
[head, sequence, channel] -> [sequence, head, channel]
```

from `attention.hip`. The checked gather is followed by an identity map and
uses exactly `4*seq*n_heads*head_dim` workspace bytes.

`gpu_repeat_rows` consumes f32 `values` and int32 `repeat_indices`, and produces
f32 `repeated`. Parameters are `source_elements`, `repeats`, and true
`repeat_indices_verified`. The table must encode the legacy
`output[i] = values[i % source_elements]` map. Workspace is exactly four bytes
per output element.

### Causal row softmax

`gpu_causal_softmax_rows` consumes rank-two f32 `values` and equal-shaped int32
`causal_mask`, and produces equal-shaped f32 `softmax`. Parameters are `rows`,
`columns`, `tree_lanes`, and true `causal_mask_verified`. The verified mask is
one exactly when `column <= row`.

The four resolved stages:

1. select the legacy `-1e30` masked maximum operand and reduce each row with a
   fixed maximum tree;
2. subtract that maximum, substitute zero before evaluating masked lanes,
   apply Recipe's owned binary32 exponential, and write zero to masked lanes;
3. sum each row with a fixed tree;
4. require a positive row sum and divide.

The owned exponential includes its device-side finite-domain check. For an
`R × C` matrix, workspace is exactly `16*R*C + 8*R` bytes.

### Positional encoding

`gpu_positional_encoding` consumes equal-shaped rank-two f32 `angles` and
int32 `channel_parity`, and produces f32 `encoding`. Parameters are `seq`,
`dim`, and true `angles_verified`. Preparation computes

```text
angles[s,d] = s / 10000^((d / 2 * 2) / dim)
channel_parity[s,d] = d % 2
```

using the exact integer grouping from `attention.hip`. One resolved map stage
contains Recipe-owned sine and cosine programs followed by a checked
zero-or-one parity selection: even channels select sine and odd channels
select cosine. The owned trigonometric programs enforce their documented
finite domain. Workspace is exactly `8*seq*dim` bytes.

### Embedding forward and backward

`gpu_embed_blend` materializes the legacy `k = 1` embedding-read path. It
consumes a rank-two f32 `source_rows`, int32 `row_indices`, and f32
`weights[rows,1]`, then produces f32 `blended[rows,embedding_width]`.
Parameters are `rows`, `k`, `source_row_count`, `embedding_width`, finite f32
`scale`, and true `row_indices_verified`. Other `k` values fail with
`UnsupportedConcreteShape` and remain a deliberate future extension of this
descriptor's concrete shape domain. The graph gathers rows, evaluates
`(row * weight) * scale` in that order, and uses four workspace bytes per
output element.

`gpu_embedding_backward` consumes f32 `gradient[rows,columns]`, int32
`indices[rows]`, and f32
`gradient_table_base[vocabulary,columns]`. It produces an equal-shaped
`gradient_table`; parameters are `rows`, `columns`, and `vocabulary`. An
identity map creates the explicit update image, then a checked axis-zero
scatter uses `AtomicOperation::Add` with explicit relaxed ordering, matching
the legacy atomic-add update of the supplied table rather than assuming an
implicit zero fill. Workspace is exactly `4*rows*columns` bytes.

### Single-tensor rotary position embedding

The following five source descriptors share a graph only because their
preparation contracts preserve each source formula:

- `gpu_rope`;
- `gpu_rope_partial`;
- `gpu_rope_partial_pos`;
- `gpu_rope_partial_factors`;
- `gpu_rope_partial_factors_pos`.

All consume flattened f32 `values`, int32 `partner_indices`, f32 `cosines`,
and f32 `signed_sines`, and produce equal-shaped f32 `rotated`. Factor variants
also declare the source f32 `factors[rotary_dim/2]` tensor. The true
`rotation_tables_verified` fact binds all four tables, and the factor tensor
when present, to the exact source formula. A checked gather reads each paired
coordinate; the owned scalar map evaluates

```text
rotated = values * cosines + partner * signed_sines
```

with negative sine in each first half and positive sine in each second half.
Coordinates outside `rotary_dim` in partial variants use self partners,
cosine one, and signed sine zero.

`gpu_rope` parameters are `seq`, even `dim`, finite positive f32 `base`, and
the verification fact. Its angle is
`sequence / base^(2*pair/dim)`.

Partial variants use `rows`, `head_dim`, even `rotary_dim`,
`heads_per_token`, finite positive f32 `theta`, and the verification fact.
Their angle is

```text
(position_base + row / heads_per_token)
    * theta^(-2*pair/rotary_dim)
    / factor[pair]
```

where the non-`_pos` variants use position base zero and non-factor variants
use factor one. `position_base`, when present, must fit int32. Each graph uses
exactly four workspace bytes per input element.

### `gpu_batchnorm_inference`

Inputs are `values`, `running_mean`, `running_variance`, `scale`, and `bias`;
output is `normalized`. All are f32 and broadcast to the output shape.
Positive finite f32 parameter `epsilon` is embedded in SSA implementing
`((values - mean) / sqrt(variance + epsilon)) * scale + bias`, including a
device-side positivity requirement.

### `gpu_im2col_1d`

`image` is f32, `receptive_field_indices` is int32, and `columns` has the
checked gather-result shape at prepared `axis`.
`indices_encode_receptive_fields` must be true. The gathered image is copied by
an explicit identity scalar program.

### `gpu_momentum_update`

Equal-shaped f32 inputs are `parameters`, `velocity`, and `gradient`; outputs
are `updated_velocity` and `updated_parameters`. Finite f32 `momentum` and
nonnegative finite f32 `learning_rate` are embedded into two scalar programs:

```text
updated_velocity = momentum * velocity - learning_rate * gradient
updated_parameters = parameters + updated_velocity
```

This is the exact legacy `optim.hip` recurrence. The second node has a real
graph dependency on the first output.

### `gpu_leaf_split_apply`

`features` is a flattened f32 image. Equal-shaped row tensors are int32
`feature_indices`, f32 `thresholds`, int32 `assignments`, int32
`destination_indices`, and int32 output `updated_assignments`.
`destination_indices_unique` must be true.

The graph gathers checked features, computes
`next = assignment * 2 + int32(feature > threshold)`, and uniquely scatters the
next assignments.

## Optimizer materializers

All optimizer payload and state tensors are equal-shaped, nonempty f32
tensors. Legacy scalar device buffers and the launch count become typed
preparation parameters and static shape metadata. State and parameter outputs
are distinct SSA values; the assembled graph may later apply an explicit alias
policy.

| Source-qualified operation | Inputs | Outputs | Prepared parameters |
| --- | --- | --- | --- |
| `gpu_adagrad_update` (`optimizers.rs`) | `gradient`, `weight`, `accumulator` | `updated_accumulator`, `updated_weight` | `learning_rate`, `epsilon` |
| `gpu_rmsprop_update` (`optimizers.rs`) | `gradient`, `weight`, `cache` | `updated_cache`, `updated_weight` | `learning_rate`, `decay`, `epsilon` |
| `gpu_adam_update` (`kernels.rs`) | `gradient`, `weight`, `first_moment`, `second_moment` | both updated moments, `updated_weight` | `learning_rate`, `beta_one`, `beta_two`, `epsilon`, `step` |
| `gpu_adamw_update` (`kernels.rs`) | Adam inputs | Adam outputs | Adam parameters plus `weight_decay` |
| `gpu_nadam_update` (`optimizers.rs`) | Adam inputs | Adam outputs | Adam parameters |
| `gpu_lion_update` (`optimizers.rs`) | `gradient`, `weight`, `moment` | `updated_moment`, `updated_weight` | `learning_rate`, `beta_one`, `beta_two`, `weight_decay` |

AdaGrad and RMSProp emit the state recurrence followed by the checked
square-root weight update. Adam, AdamW, and NAdam embed preparation-time f32
bias corrections derived from positive int32-range `step`; beta values must be
in `[0, 1)`. AdamW preserves the legacy decoupled formula
`weight * (1 - learning_rate * weight_decay) - adaptive_step`. Lion preserves
the legacy three-way sign rule, including exactly zero for a zero
interpolation. Its direction is an owned intermediate, making the weight node
depend on the state node.

These six operations use no workspace except Lion, which reserves exactly one
f32 tensor image.

### LAMB

`gpu_lamb_phase1` inputs are `gradient`, `weight`, `moment`, and `velocity`.
Outputs are `updated_moment`, `updated_velocity`, `update`,
`weight_norm_squared`, and `update_norm_squared`. Parameters are `beta_one`,
`beta_two`, `epsilon`, `weight_decay`, positive `step`, and `tree_lanes`.
The first elementwise node writes both recurrences, the bias-corrected update,
and both square images. Fixed-tree reductions publish the norm squares before
an identity node publishes the update. Workspace is exactly three f32 tensor
images.

`gpu_lamb_phase2` consumes `update`, both scalar norm-square tensors, and
`weight`; it produces `updated_weight`. Parameters are `learning_rate` and
`tree_lanes`. The supplied norm squares pass through explicit singleton
fixed-tree reductions, then the final scalar program applies the legacy
zero-norm trust ratio of one. Workspace is exactly four f32 scalars.

### Gradient clipping and scalar clipping

`gpu_grad_clip_norm` consumes f32 `values`, produces equal-shaped `clipped`,
and requires `maximum_norm` plus `tree_lanes`. It squares, fixed-tree reduces
over every static axis, and conditionally scales. Workspace is one f32 tensor
image plus one f32 scalar.

`gpu_clip_value` is intentionally absent from this structured list. It is
already an owned scalar lowering with the legacy low-then-high clamp order and
its source alias contract.

## Normalization materializers

All matrix normalization inputs are nonempty rank-two f32 `[rows, columns]`
tensors. Both extents must fit int32 launch metadata and be exactly
representable as f32 divisors. `tree_lanes` fixes every reduction tree.

### Batch normalization

`gpu_batchnorm_forward` consumes `values`, column `scale`, and column `bias`.
It produces `normalized`, `saved_mean`, and
`saved_inverse_standard_deviation`; parameters are positive `epsilon` and
`tree_lanes`. The graph reduces each channel mean, forms centered squares only
after that mean exists, reduces variance, then emits saved statistics and the
affine result. For `N × C`, workspace is exactly
`8*N*C + 8*C` bytes.

`gpu_batchnorm_backward` consumes `output_gradient`, `values`, `saved_mean`,
`saved_inverse_standard_deviation`, and `scale`. It produces
`input_gradient`, `scale_gradient`, and `bias_gradient`; its only parameter is
`tree_lanes`. It preserves the legacy analytic formula and reduces both
affine-gradient images over rows. Workspace is exactly
`16*N*C + 8*C` bytes.

`gpu_bn_update_running` consumes saved and running mean/variance tensors,
produces both updated running tensors, and embeds `momentum` in `[0, 1]`. It is
a two-output scalar program with zero workspace.

### Layer normalization aliases

The following source descriptors remain distinct while sharing the same
verified graph only where their legacy semantics agree:

- `gpu_layernorm_f32` from `nn_f32.rs`;
- `gpu_layernorm_into` from `kernels.rs`;
- `gpu_layernorm_opt_into` from `kernels.rs`;
- `gpu_layernorm_backward_f32` from `nn_f32.rs`;
- `gpu_layernorm_backward_full_into` from `kernels.rs`.

Forward inputs are `values`, column `scale`, and column `bias`, with
`normalized` output and parameters `epsilon` and `tree_lanes`.
`gpu_layernorm_opt_into` additionally requires `has_scale` and `has_bias`;
disabled tensors are absent from its ABI. It preserves the legacy behavior
that bias is ignored when scale is absent. Forward workspace for `R × C` is
`8*R*C + 8*R` bytes.

Backward inputs are `output_gradient`, `values`, and column `scale`; outputs
are `input_gradient`, `scale_gradient`, and `bias_gradient`. Parameters are
`epsilon` and `tree_lanes`. Mean, centered variance, both row-gradient
statistics, and both affine-gradient reductions are explicit nodes. Workspace
is `24*R*C + 16*R` bytes.

### RMS normalization aliases

`gpu_rmsnorm` and the canonical-f32 replacement for `gpu_rmsnorm_f64` consume
`values` and column `scale`. The canonical-f32 replacement for
`gpu_rmsnorm_f64_nogamma` consumes only `values`. All produce `normalized` and
require `epsilon` plus `tree_lanes`. Workspace is
`4*R*C + 8*R` bytes.

`gpu_rmsnorm_backward` consumes `output_gradient`, `values`, and `scale`, and
produces `input_gradient` and `scale_gradient`. Its dot statistic intentionally
matches the legacy attention kernel’s exact formula. Workspace is
`12*R*C + 8*R` bytes.

## Convolution and pooling family materializers (prepared-window batch)

This family uses flattened canonical f32 payloads and immutable int32
preparation tables. Every dimension is nonzero, every legacy launch extent and
linearized element count must fit int32, and every tensor and parameter name
set is exact. Verification facts bind the tables to the source formulas;
gathers and scatters still reject an out-of-range runtime index.

### Image-to-column and column-to-image

The existing source-qualified `gpu_im2col_1d` ABI remains `image`,
`receptive_field_indices`, and output `columns`, with prepared `axis` and true
`indices_encode_receptive_fields`.

`gpu_im2col_2d` and `gpu_im2col_2d_ext` consume flattened f32 `image` plus
int32 `receptive_field_indices`, and produce flattened f32 `columns`. The basic
variant takes `batch`, `channels`, input height and width, and kernel height and
width. The extended variant additionally takes stride, padding, and dilation
for both axes. In both cases the verified table encodes the exact source
coordinate order

```text
[batch, output_y, output_x, channel, kernel_y, kernel_x].
```

The two-stage graph gathers then applies an explicit identity map, using four
workspace bytes per patch element. `gpu_im2col_2d_ext` explicitly rejects
nonzero padding: its two-stage `Gather -> Elementwise` recipe cannot synthesize
the source kernel’s padding zeros without an additional fill/scatter primitive.
Stride and dilation remain fully supported when padding is zero.

`gpu_col2im_1d`, `gpu_col2im_2d`, and `gpu_col2im_2d_ext` consume f32
`patches`, int32 `patch_indices`, int32 `destination_indices`, and f32
`image_base`; output is f32 `image`. True `image_base_zero` preserves the
legacy zero-before-accumulate behavior. The two index-verification facts bind
the gather and destination maps to the exact inverse receptive fields. A
checked gather and identity map feed a relaxed atomic-add scatter, so
overlapping windows have an explicit race policy. Workspace is eight bytes per
scattered contribution.

For the extended inverse, `patches` retains the complete padded patch shape
while positive `valid_contributions` fixes the sizes of both index tables.
Preparation omits padded coordinates from those tables, exactly matching the
legacy kernel’s conditional atomic add. Thus padded `gpu_col2im_2d_ext` does
not share the forward restriction.

### Average pooling

The following source descriptors share the same canonical f32 graph only
where their formulas agree:

- `gpu_avg_pool_1d`;
- `gpu_avg_pool_2d` and `gpu_avg_pool_2d_f32`;
- `gpu_avg_pool_2d_backward` and `gpu_avg_pool_2d_backward_f32`;
- `gpu_pool_grad_expand`.

Forward inputs are f32 `values` and int32 `window_indices`; output is f32
`pooled`. One-dimensional parameters are `batch`, `window_length`, and
`filters`. Two-dimensional parameters are `batch`, `channels`, input and
kernel height and width, and stride height and width. Both add `tree_lanes` and
true `window_indices_verified`. Each graph gathers
`[output_elements, window_elements]`, reduces the window with a fixed sum tree,
and divides by the exact f32 window count. Workspace is

```text
4 * output_elements * window_elements + 4 * output_elements.
```

Average-pool backward consumes f32 `output_gradient`, int32
`gradient_indices`, int32 `destination_indices`, and zero f32
`input_gradient_base`; output is `input_gradient`. The two verified index
tables expand each output gradient over its source window. The map divides by
the exact window count, and a relaxed atomic-add scatter handles overlap.
Workspace is eight bytes per contribution.

`gpu_pool_grad_expand` uses the analogous one-dimensional ABI and adds true
`destination_indices_unique`. Its final scatter is therefore explicitly
unique rather than atomic, matching the direct-write legacy kernel.

### Maximum pooling

`gpu_max_pool_1d`, `gpu_max_pool_2d`, and `gpu_max_pool_2d_f32` consume f32
`values` and int32 `window_indices`; outputs are f32 `pooled` and canonical
int32 `winning_indices`. The two-dimensional variants additionally consume
int32 `window_bases`. Parameters mirror the corresponding average-pool
geometry, add `tree_lanes`, and require the relevant table facts.

A checked window gather feeds a fixed maximum value/index reduction with the
lowest logical coordinate winning ties. The final map preserves the legacy
one-dimensional local time index. In two dimensions it converts the local
kernel index to the legacy spatial position:

```text
winning = window_base
        + (local / kernel_width) * input_width
        + (local % kernel_width).
```

Workspace is four bytes per gathered window value plus eight bytes per output
for the maximum and local int32 index.

The backward descriptors are `gpu_max_pool_1d_backward`,
`gpu_max_pool_2d_backward`, and `gpu_max_pool_2d_backward_f32`. They consume
f32 `output_gradient`, canonical int32 `winning_indices`, a verified identity
`gradient_indices` table, verified int32 destination or plane bases, and zero
f32 `input_gradient_base`; output is `input_gradient`. A gather preserves the
resolved backward recipe, the map computes the checked global destination,
and the scatter publishes the contribution. One-dimensional destinations are
verified unique. Two-dimensional windows may select the same source, so their
scatter uses relaxed atomic addition. Workspace is exactly twelve bytes per
pooled output.

### Nearest-neighbor upsampling

`gpu_upsample_nearest_2d` consumes flattened f32 `values` and int32
`source_indices`, and produces f32 `upsampled`. Parameters are batch, channels,
input height and width, scale height and width, and true
`source_indices_verified`. The table encodes
`input[y / scale_height, x / scale_width]`; a checked gather plus identity map
uses four workspace bytes per output.

## Loss, statistics, and metric materializers (fixed-tree batch)

This batch owns fifteen exact source descriptors. Every payload is canonical
f32 except the declared int32 class indexes and truth results. Tensor and
prepared-parameter name sets are exact. Nonempty element counts and matrix
dimensions must fit the legacy int32 launch domain. Counts converted to f32
for normalization or a f32 count result must be at most `2^24`.

Every reduction takes prepared `tree_lanes`, which must be a power of two in
`1..=1024`. The lane count fixes the operation order. Global reductions cover
every tensor axis, row reductions cover axis one, and column reductions cover
axis zero. No materializer in this batch uses an atomic reduction. All fifteen
descriptors retain `FixedPrimitiveOrder`.

### Accuracy and finite-value metrics

`gpu_accuracy_into` consumes equal-shaped f32 `predictions` and `targets`, and
produces f32 scalar `accuracy`. Both inputs use the source threshold
`value >= 0.5`. Their int32 match flags are summed, converted to f32, and
divided by the exact element count. Workspace is `4*N + 4` bytes.

`gpu_accuracy` consumes f32 `predictions[rows,classes]` and canonical int32
`targets[rows]`, and produces f32 scalar `correct_count`. A fixed maximum-index
reduction selects the lowest class index on ties. The match map checks every
target in `0..classes` through the device fault channel before comparison.
Workspace is `8*rows` bytes.

`gpu_argmax_accuracy_into` consumes f32
`predictions[rows,classes]` and equal-shaped dense f32 `targets`, and produces
f32 scalar `accuracy`. Two lowest-index maximum reductions produce the class
indexes, a fixed sum counts equal indexes, and the final map divides by the
exact row count. Workspace is `12*rows + 4` bytes.

`gpu_has_nan` and `gpu_isfinite_all` consume f32 `values`. Their outputs are
int32 scalars `has_nan` and `all_finite`. The map emits canonical int32 flags;
the final fixed tree uses `Any` or `All` over every axis. Both require
`4*N` workspace bytes.

### Scalar and aggregate losses

`gpu_hinge_loss` consumes equal-shaped f32 `scores` and `labels`; outputs are
equal-shaped f32 `losses` and `gradients`. One scalar program preserves

```text
margin = 1 - label * score
loss = margin > 0 ? margin : 0
gradient = margin > 0 ? -label : 0
```

and needs no workspace.

`gpu_kl_div_loss` consumes equal-shaped f32 `log_probabilities` and `targets`;
output is equal-shaped f32 `losses`. Nonpositive targets select zero without
entering the logarithm. Positive targets pass through Recipe's finite,
positive-domain checked logarithm and evaluate
`target * (log(target) - log_probability)`. Two f32 images use `8*N`
workspace bytes.

`gpu_mse_into` and `gpu_ss_res_into` consume equal-shaped f32 `predictions`
and `targets`, then produce f32 scalar `mean_squared_error` and
`sum_squared_residuals`. Both form `(prediction - target)^2` in source order
and sum every axis with a fixed tree. MSE divides the completed sum by the
exact element count. Workspace is `4*N + 4` bytes for MSE and `4*N` bytes for
SSR.

### Fixed-order statistics

`gpu_mean_all` consumes f32 `values` and produces f32 scalar `mean`. It sums
every axis before dividing by the exact element count, using four workspace
bytes for the completed sum.

`gpu_reduce_mean_cols` consumes f32 `values[rows,columns]` and produces f32
`means[columns]`. It reduces axis zero, then divides by the exact row count.
Workspace is `4*columns` bytes.

`gpu_reduce_var_cols` consumes f32 `values[rows,columns]` and produces f32
population `variances[columns]`. It computes fixed-tree column sums, maps
squared deviations from those means, performs a second fixed-tree column sum,
and only then divides by the row count. A final extent-one reduction publishes
the divided kept-dimension image without changing its value or moving the
division before the source sum. Exact workspace is
`4*rows*columns + 12*columns` bytes.

### Per-example distance losses

The three distance losses require f32 rank-two feature inputs, one f32 result
per row, finite prepared `margin`, and fixed row reductions.

`gpu_contrastive_loss` consumes `left`, `right`, and `labels[rows]`. It forms
and sums squared feature differences, checks that the reduced squared distance
is finite and nonnegative, and applies

```text
label * distance_squared
    + (1 - label) * max(margin - sqrt(distance_squared), 0)^2.
```

Workspace is `4*rows*dimensions + 4*rows` bytes.

`gpu_cosine_embedding_loss` uses the same ABI. Its map emits dot-product and
two squared-norm terms; three independent fixed row reductions preserve their
orders. Finite nonnegative norm sums feed the source denominator
`sqrt(left_norm_squared) * sqrt(right_norm_squared) + 1e-12`. Positive labels
select `1 - cosine`; other labels select `max(cosine - margin, 0)`. Workspace
is `12*rows*dimensions + 12*rows` bytes.

`gpu_triplet_loss` consumes equal-shaped f32 `anchor`, `positive`, and
`negative`, then produces f32 `losses[rows]`. Separate fixed reductions form
the two squared distances, both must be finite and nonnegative, and the final
map evaluates
`max(anchor_positive - anchor_negative + margin, 0)`. Workspace is
`8*rows*dimensions + 8*rows` bytes.

There are 264 structured source descriptors in the normative operation
surface. One hundred three now have concrete, source-qualified materializers;
the deterministic remaining manifest contains 161.
