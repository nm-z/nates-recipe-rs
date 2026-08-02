# `loss_metrics` materializers

## Ownership and boundary

`ops/src/materialize/loss_metrics.rs` is the concrete materializer for fifteen
source-qualified entries in the legacy operation inventory. It does not call a
legacy GPU implementation, execute a host payload loop, or provide a CPU
fallback. It turns each accepted `MaterializationRequest` into a finite
`recipe-language::CalculationGraph` made from typed `Elementwise` and `Reduce`
primitives. The request is prepared before the run loop, so shapes, prepared
values, identity ranges, and the workspace limit are immutable while this
module emits the graph.

The right-hand source strings below are the identity recorded in
`operation-surface.txt`. They are legacy source labels, not claims that a
`gpu-core` checkout is present in this repository. The implementation paired
with every row is the exact branch and emitter named in the table.

## Surface pairs, registration, and concrete branches

`operation-surface.txt` is parsed by `ops/build.rs` into source-qualified
`RawSurfaceEntry` values. `registry::describe` then chooses a
`CompositionRecipe`, derives the dtype, family, alias, and determinism
contracts, and preserves the source pair in `OperationDescriptor`. The
composition names and step lists come from `ops/src/composition.rs`; the
concrete branch and tensor ABI come from `loss_metrics.rs`.

| Inventory row | Exact pair | Registry composition and primitive sequence | Concrete branch | Payload, family, alias, determinism |
| ---: | --- | --- | --- | --- |
| `operation-surface.txt:20` | `gpu_accuracy` / `gpu-core/src/kernels.rs:4472` | `multiclass_correct_count` / `REDUCE_MAP_REDUCE` | `dispatch` -> `emit_multiclass_correct_count` (`loss_metrics.rs:42,110-165`) | `F32AndI32`, `Metric`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:21` | `gpu_accuracy_into` / `gpu-core/src/kernels.rs:2914` | `binary_accuracy` / `MAP_REDUCE_MAP` | `dispatch` -> `emit_binary_accuracy` (`loss_metrics.rs:43,69-108`) | `F32AndI32`, `Metric`, `NoAlias` because the symbol ends in `_into`, `FixedPrimitiveOrder` |
| `operation-surface.txt:33` | `gpu_argmax_accuracy_into` / `gpu-core/src/kernels.rs:2963` | `dense_multiclass_accuracy` / `DENSE_ARGMAX_ACCURACY_STEPS` | `dispatch` -> `emit_dense_multiclass_accuracy` (`loss_metrics.rs:44,167-234`) | `F32AndI32`, `Metric`, `NoAlias`, `FixedPrimitiveOrder` |
| `operation-surface.txt:85` | `gpu_contrastive_loss` / `gpu-core/src/losses.rs:328` | `contrastive_loss` / `DISTANCE_LOSS_STEPS` | `dispatch` -> `emit_contrastive_loss` (`loss_metrics.rs:45,547-589`) | `F32`, `Loss`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:94` | `gpu_cosine_embedding_loss` / `gpu-core/src/losses.rs:270` | `cosine_embedding_loss` / `COSINE_EMBEDDING_STEPS` | `dispatch` -> `emit_cosine_embedding_loss` (`loss_metrics.rs:46,591-651`) | `F32`, `Loss`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:176` | `gpu_has_nan` / `gpu-core/src/math_ops.rs:456` | `any_nan` / `MAP_REDUCE` | `dispatch` -> `emit_finite_metric(..., false)` (`loss_metrics.rs:47,236-274`) | `F32AndI32`, `Metric`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:177` | `gpu_hinge_loss` / `gpu-core/src/losses.rs:246` | `hinge_loss` / `MAP_ONLY` | `dispatch` -> `emit_hinge_loss` (`loss_metrics.rs:48,276-295`) | `F32`, `Loss`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:186` | `gpu_isfinite_all` / `gpu-core/src/math_ops.rs:476` | `all_finite` / `MAP_REDUCE` | `dispatch` -> `emit_finite_metric(..., true)` (`loss_metrics.rs:49,236-274`) | `F32AndI32`, `Metric`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:190` | `gpu_kl_div_loss` / `gpu-core/src/losses.rs:225` | `kl_divergence_loss` / `MAP_ONLY` | `dispatch` -> `emit_kl_divergence_loss` (`loss_metrics.rs:50,297-331`) | `F32`, `Loss`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:241` | `gpu_mean_all` / `gpu-core/src/reductions.rs:275` | `global_mean` / `REDUCE_MAP` | `dispatch` -> `emit_global_mean` (`loss_metrics.rs:51,333-361`) | `F32`, `Statistics`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:254` | `gpu_mse_into` / `gpu-core/src/kernels.rs:2888` | `mean_squared_error` / `MAP_REDUCE_MAP` | `dispatch` -> `emit_mean_squared_error` (`loss_metrics.rs:52,363-405`) | `F32`, `Loss`, `NoAlias` because the symbol ends in `_into`, `FixedPrimitiveOrder` |
| `operation-surface.txt:297` | `gpu_reduce_mean_cols` / `gpu-core/src/kernels.rs:5176` | `column_mean` / `REDUCE_MAP` | `dispatch` -> `emit_column_mean` (`loss_metrics.rs:53,442-474`) | `F32`, `Statistics`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:303` | `gpu_reduce_var_cols` / `gpu-core/src/kernels.rs:5209` | `column_variance` / `REDUCE_MAP_REDUCE` | `dispatch` -> `emit_column_variance` (`loss_metrics.rs:54,476-545`) | `F32`, `Statistics`, `OperationSpecific`, `FixedPrimitiveOrder` |
| `operation-surface.txt:376` | `gpu_ss_res_into` / `gpu-core/src/kernels.rs:2862` | `sum_squared_residuals` / `MAP_REDUCE` | `dispatch` -> `emit_sum_squared_residuals` (`loss_metrics.rs:55,407-440`) | `F32`, `Loss`, `NoAlias` because the symbol ends in `_into`, `FixedPrimitiveOrder` |
| `operation-surface.txt:407` | `gpu_triplet_loss` / `gpu-core/src/losses.rs:299` | `triplet_margin_loss` / `TRIPLET_LOSS_STEPS` | `dispatch` -> `emit_triplet_loss` (`loss_metrics.rs:56,653-710`) | `F32`, `Loss`, `OperationSpecific`, `FixedPrimitiveOrder` |

The `CompositionPayload` column is the registry's broad canonical payload
contract. The concrete emitters impose the narrower tensor names, shapes, and
individual dtypes documented below. In particular, a descriptor with an
`F32AndI32` payload may have f32 boundary inputs and an i32 class-index tensor,
while a descriptor with `F32` still uses i32 intermediates for truth flags or
argmax indexes when its concrete graph requires them.

### Registry construction

`ops/build.rs:3-53` watches `operation-surface.txt`, rejects malformed rows,
assigns each accepted row an ordinal and source line, and writes
`OUT_DIR/operation_surface.rs`. `ops/src/registry.rs:208` includes that file;
`OperationRegistry::iter` retains the generated order and appends the two
Recipe-owned extensions (`registry.rs:245-281`). `resolve_unique` rejects an
ambiguous symbol, while `resolve_exact` requires one exact `(symbol, source)`
pair (`registry.rs:283-323`). These fifteen symbols are unique in the current
surface, but the materializer still carries and checks the source field.

`registry::lowering` checks scalar recipes, primitive recipes, workspace
formulas, non-calculation entries, and then source-qualified compositions in a
fixed order (`registry.rs:347-373`). The fifteen rows above select the
composition branch in `composition.rs:976-1000,1282-1325,1669-1757,
1993-2071,2266-2281,2569-2576`. Composition validation requires a nonempty
name, definition, and finite primitive step list (`composition.rs:69-123`),
so these rows are not merely descriptive registry text.

### Concrete dispatcher

`materialize_composition` is the shared callee for every concrete branch:

```text
MaterializationRequest
  -> validate_request
  -> require a Composition descriptor with a concrete family owner
  -> resolve the iteration-shape input
  -> expand_composition into a finite ResolvedComposition
  -> Emitter::new with caller-reserved IDs
  -> dispatch_concrete
  -> loss_metrics::supports exact (symbol, source) lookup
  -> loss_metrics::dispatch symbol match
  -> family emitter and Recipe scalar programs
  -> Emitter::finish
  -> MaterializedComposition { graph, stages, workspace, identities }
```

The public advanced facade exposes this same path as
`recipe::operations::materialize` (`src/facade.rs:44-124`), which forwards to
`recipe_ops::materialize_composition`. `ops/src/materialize.rs:413-445`
performs the sequence above. `dispatch_concrete` probes family modules in a
fixed order and returns the first `FamilyDispatch::Owned` result
(`materialize.rs:447-480`); the loss/metric module is the fifth probe. A
descriptor outside this module returns `FamilyDispatch::NotOwned`, allowing
the next module to inspect it. A pair listed in `OPERATIONS` but absent from
the symbol match would instead return `GraphMaterializationFailed` with
`loss/metric dispatch is incomplete for ...`, making table and branch drift
visible.

`supports` is exact tuple membership (`loss_metrics.rs:14-35`), not a symbol
prefix test. A same-named operation from a different legacy source therefore
cannot enter one of these emitters.

## Shared request, type, state, and failure contract

### Request and boundary state

`MaterializationRequest` (`materialize.rs:109-143`) carries one immutable
`OperationDescriptor`, named input and output slices, the name of the tensor
whose shape drives composition expansion, a `PreparedParameters` map, an
`IdentityNamespace`, and a `workspace_limit`. `NamedTensor` is a borrowed
name-to-`Tensor` declaration (`materialize.rs:36-53`). A caller must mark every
input `external_input = true` and every output `external_output = true` while
keeping input and output IDs distinct. `validate_request` checks the lowering
kind, nonempty declaration sets, unique names, unique tensor IDs, each tensor's
own `Tensor::validate`, and the external flags before any family branch runs
(`materialize.rs:4268-4331`).

Every family branch then calls `require_exact_abi` before looking up a tensor or
parameter. The helper compares sets, so a missing name, an extra name, or an
extra prepared parameter is rejected (`materialize.rs:4221-4266`). The local
`input` and `output` helpers fail with `InvalidMaterializationRequest` when a
required declaration is absent (`materialize.rs:4333-4357`).

`GraphBuilder` copies boundary declarations, rejects a boundary value inside
the reserved intermediate range, allocates intermediate tensors contiguously,
adds their checked storage bytes to workspace, and assigns forbidden aliases
to every emitted kernel (`materialize.rs:712-845`). The caller owns the
half-open value and kernel ranges. Exhausted or overflowing ranges are
`IdentityNamespaceExhausted`; overlapping boundary and reserved identities
are `IdentityNamespaceOverlap`. A workspace total above the caller limit is
`WorkspaceLimitExceeded`, and checked byte arithmetic overflow is
`WorkspaceArithmeticOverflow`.

`Emitter::emit_stage` consumes one resolved composition step per call. It
rejects an empty stage, an extra stage, a missing stage, or a primary kernel
whose `PrimitiveFamily` differs from the recipe (`materialize.rs:609-710`).
`Emitter::finish` requires the cursor to equal the expanded step count and then
validates the complete `CalculationGraph` (`materialize.rs:687-708`). Scalar
builder errors, invalid `Shape` or `AxisSet` construction, primitive mismatch,
and final graph validation are reported as `GraphMaterializationFailed` with
the operation ID.

### Common shape, dtype, and prepared-state checks

The family-local helpers in `loss_metrics.rs:713-864` define the common
contract:

| Helper | Actual check | Failure and state consequence |
| --- | --- | --- |
| `require_nonempty_f32` | f32 dtype; total element count in `1..=2_147_483_647` | `InvalidMaterializationRequest` for a wrong dtype, empty tensor, or legacy int32 launch-domain overflow. |
| `require_equal_nonempty_f32` | Applies the preceding check to a reference and requires each named tensor to have exactly the same dtype and shape | `InvalidMaterializationRequest`; no intermediate is emitted before the mismatch is returned. |
| `require_f32_matrix` | Rank two, nonempty f32 matrix; both row and column extents in `1..=2_147_483_647`; returns `(rows, columns)` | `InvalidMaterializationRequest` for rank or extent violations. The total element check runs first. |
| `require_paired_loss_tensors` | Equal f32 rank-two `left` and `right`; f32 `labels[rows]`; f32 `losses[rows]` | `InvalidMaterializationRequest` for any dtype, shape, empty, or extent mismatch. |
| `require_f32_scalar` / `require_i32_scalar` | Exact dtype and shape `[1]` | `InvalidMaterializationRequest` for a scalar with another rank, extent, or dtype. |
| local `require_shape` | Exact extents, including output vectors and class-index vectors | `InvalidMaterializationRequest` with the actual and expected extents. |
| `finite_parameter` | Reads a prepared `F32Bits` value and requires `is_finite()`; no sign restriction | `MissingPreparedParameter`, `PreparedParameterTypeMismatch`, or `InvalidMaterializationRequest`. Negative margins remain valid because only finiteness is checked. |
| `prepared_tree_lanes` | Reads prepared `U64`, converts to `u32`, and requires a power of two in `1..=1024` | `MissingPreparedParameter`, `PreparedParameterTypeMismatch`, conversion failure, or `InvalidMaterializationRequest`. The value fixes every reduction tree. |
| `exact_f32_extent` (`materialize.rs:3963-3985`) | Requires a count no larger than `16_777_216` before embedding it as an f32 divisor | `UnsupportedConcreteShape`; this prevents a count or normalization divisor from losing integer exactness. |

The matrix helpers enforce the legacy int32 index domain, while
`exact_f32_extent` independently enforces the stricter binary32 exact-integer
domain whenever a count is converted to f32. `axis` and `all_axes` construct
checked `AxisSet` values. `reduction` creates a `Reduce` with the requested
operator, axes, `keep_dimensions`, `ReduceResult`, and prepared `tree_lanes`
(`loss_metrics.rs:827-864`).

### Device-side guards versus preparation failures

The host validates tensor contracts and prepared metadata only. Several scalar
programs deliberately keep value-domain checks in the graph:

* `ScalarOpcode::Require` accepts an int32 truth value and records a device
  fault in preallocated storage when it is zero (`core/src/scalar.rs:79-85`,
  `kernel/src/llvm.rs:828-840`, `system-contract.md:332-334`).
* Checked class targets in `checked_class_match_program` require
  `0 <= target < classes` before comparing (`loss_metrics.rs:929-974`).
* Distance losses require finite, nonnegative reduced squared distances before
  taking a square root (`loss_metrics.rs:1151-1217,1235-1320,1358-1395`).
* The KL path sends positive targets through Recipe's owned `Log` program,
  whose scalar program emits a positive-domain requirement
  (`math/src/program.rs:85-88,152-175`).

A failed device `Require` is an execution-time fault, not an
`OperationError` returned while materializing. The graph does not clip,
replace, or silently repair invalid values. This distinction is part of the
state contract: a successful materialization proves only that a typed graph was
constructed; runtime execution must still observe the device result or fault.

## Concrete operations

### Accuracy and finite-value metrics

All reductions in this section use the prepared `tree_lanes`. A `Reduce` with
`ReduceResult::Index` publishes an int32 index, and Recipe's backend validates
the fixed-tree lowest-logical-index tie policy (`language/src/primitive.rs:58-66,
430-493`; `kernel/src/stage.rs:1194-1210,1766-1809`).

#### `gpu_accuracy_into`: binary accuracy

`emit_binary_accuracy` (`loss_metrics.rs:69-108`) requires exactly
`predictions` and `targets` as f32 inputs, `accuracy` as an f32 `[1]` output,
and only `tree_lanes` as a prepared parameter. The two inputs must be equal,
nonempty f32 tensors. `exact_f32_extent` converts their element count `N` to a
lossless f32 divisor.

The graph is:

1. `binary_accuracy_program` maps each pair. It compares each f32 value with
   the literal `0.5` using `GreaterThanOrEqual`, then compares the two int32
   truth flags with `Equal`. The result is an int32 match flag.
2. `Reduce(Sum, all axes, Value)` sums the flags into an int32 `[1]`
   intermediate.
3. `normalize_i32_count_program` converts the count to f32 and divides by the
   exact `N`, writing the caller's `accuracy` output.

Only the per-element int32 match image and one int32 scalar are intermediate,
so workspace is `4*N + 4` bytes. A NaN compares as not greater than or equal to
`0.5`, producing the same false classification as any other value below the
threshold. If both sides classify false they count as equal; there is no
separate target-value domain guard in this emitter. The public
training caller uses this operation for one-column binary validation
(`training/src/compile.rs:7728-7761,8181-8271`), while the standalone
`operations::materialize` API can accept any matching f32 tensors.

#### `gpu_accuracy`: sparse multiclass correct count

`emit_multiclass_correct_count` (`loss_metrics.rs:110-165`) requires
`predictions: F32[rows, classes]`, `targets: I32[rows]`, and
`correct_count: F32[1]`, plus `tree_lanes`. The matrix dimensions and row count
must satisfy the int32 and exact-f32 limits. `classes` is converted to an i32
constant for the device guard.

The graph first performs `Reduce(Maximum, axis 1, Index)` to produce the
lowest-index prediction class for each row. `checked_class_match_program`
requires every target to be nonnegative and below `classes`, compares the
predicted and target indexes, and converts the match flag to f32. A final
`Reduce(Sum, axis 0, Value)` writes the f32 `correct_count`. The prediction
class image is I32 `[rows]`, and the match image is F32 `[rows]`, for
`8*rows` bytes of workspace.

This emitter intentionally returns a count, not an accuracy. The categorical
multiclass validation caller reduces its `[rows, 1]` int32 target matrix along
axis one to an int32 row vector, materializes `gpu_accuracy`, then divides the
returned count by the validation row count (`training/src/compile.rs:7861-7900`). An invalid target
class reaches the device `Require` guard and faults during execution.

#### `gpu_argmax_accuracy_into`: dense-target multiclass accuracy

`emit_dense_multiclass_accuracy` (`loss_metrics.rs:167-234`) requires
`predictions: F32[rows, classes]`, an equal-shaped dense f32 `targets` matrix,
and `accuracy: F32[1]`, plus `tree_lanes`. It does not interpret target values
as class codes; it computes a target row argmax instead.

Two independent `Reduce(Maximum, axis 1, Index)` nodes produce I32 prediction
and target class vectors. `class_match_program` compares those vectors and
converts each equality flag to f32. A fixed `Reduce(Sum, axis 0, Value)` forms
the match count, and `normalize_f32_count_program` divides by the exact row
count. The two index images, one match image, and one scalar count require
`12*rows + 4` bytes.

The joint multiclass validation path invokes this operation directly with
dense targets (`training/src/compile.rs:7867-7876`). The multi-target binary
validation path also invokes it once on the full probability and dense-target
matrices, after independently compiling each target column's other metrics
(`training/src/compile.rs:8058-8158`). Equal maximum values select the lowest
logical class index by the fixed reduction contract.

#### `gpu_has_nan` and `gpu_isfinite_all`: finite-value flags

Both symbols call `emit_finite_metric` (`loss_metrics.rs:236-274`) with the
same exact ABI: `values: F32` nonempty and within the int32 element domain,
one I32 `[1]` output, and `tree_lanes`. The `all_finite` branch maps each value
through `ScalarOpcode::IsFinite` and reduces with `Reduce(All, all axes,
Value)`. The `has_nan` branch maps through `ScalarOpcode::IsNan` and reduces
with `Reduce(Any, all axes, Value)`. The output is canonical int32 truth data:
`1` means all values are finite or at least one value is NaN, respectively.

The flag image is the only intermediate, so each operation reserves `4*N`
bytes. Infinity is not a NaN, so `has_nan` remains zero for an infinite value
while `all_finite` becomes zero. These are concrete metric operations exposed
through the registry and direct materialization boundary; no current high-level
training caller names either symbol.

### Scalar and aggregate losses

#### `gpu_hinge_loss`

`emit_hinge_loss` (`loss_metrics.rs:276-295`) requires exact f32, equal-shaped,
nonempty `scores` and `labels` inputs and equal-shaped f32 `losses` and
`gradients` outputs. There are no prepared parameters and no intermediate
images. One elementwise `hinge_loss_program` emits two results:

```text
margin   = 1 - label * score
loss     = margin > 0 ? margin : 0
gradient = margin > 0 ? -label : 0
```

The comparison is strict. A zero margin therefore emits zero gradient. Labels
are not constrained to `{-1, +1}` by this materializer, so the formula is
applied to the supplied f32 value exactly. Workspace is zero.

#### `gpu_kl_div_loss`

`emit_kl_divergence_loss` (`loss_metrics.rs:297-331`) requires equal-shaped,
nonempty f32 `log_probabilities` and `targets`, with f32 `losses` of that same
shape. There are no prepared parameters. It emits three elementwise stages:

1. `positive_or_one_program` maps a target greater than zero to itself and
   every nonpositive or unordered value to `1.0`.
2. Recipe's `MathFunction::Log` program computes the log of the safe target;
   its positive-domain guard remains live in the graph. The surrounding
   materializer adds no separate finite-value guard for the target.
3. `kl_divergence_program` computes `target * (log(target) - log_probability)`
   for a positive original target, and selects zero otherwise.

The safe-target and logarithm images are each one f32 value per input element,
for `8*N` bytes of workspace. This code does not silently take the logarithm
of a zero target. A negative, zero, or unordered target selects zero in the
final program; a positive value outside the owned log domain reaches the
device guard.

#### `gpu_mean_all`

`emit_global_mean` (`loss_metrics.rs:333-361`) requires a nonempty f32
`values` tensor, f32 `mean: [1]`, and `tree_lanes`. The exact element count is
embedded as a lossless f32 divisor. A fixed `Reduce(Sum, all axes, Value)`
first writes a f32 `[1]` sum intermediate; `normalize_f32_count_program` then
divides by the count into `mean`. Workspace is four bytes. The reduction spans
every rank axis, so the output is the mean of every element rather than a
per-row or per-column mean.

#### `gpu_mse_into` and `gpu_ss_res_into`

`emit_mean_squared_error` (`loss_metrics.rs:363-405`) and
`emit_sum_squared_residuals` (`loss_metrics.rs:407-440`) share the exact f32
equal-shape input contract for `predictions` and `targets`. Their outputs are
f32 `[1]` scalars named `mean_squared_error` and `sum_squared_residuals`; both
require `tree_lanes`.

Both first run `squared_difference_program`, which computes
`(prediction - target) * (prediction - target)` in that source order. Both then
use a fixed `Reduce(Sum, all axes, Value)`. MSE allocates a scalar residual sum
and divides by the exact element count with
`normalize_f32_count_program`; its workspace is `4*N + 4` bytes. SSR writes
the reduced sum directly to its output and uses `4*N` bytes. Neither emitter
computes gradients, and neither has an in-tree high-level training caller;
training's declared MSE objective currently uses its own pointwise loss path
(`training/src/compile.rs:1142-1149`), while these registry operations remain
available to direct composition callers.

### Fixed-order column statistics

#### `gpu_reduce_mean_cols`

`emit_column_mean` (`loss_metrics.rs:442-474`) requires
`values: F32[rows, columns]`, `means: F32[columns]`, and `tree_lanes`. It
checks both matrix extents and the exact f32 row divisor. A fixed
`Reduce(Sum, axis 0, Value)` writes `sums: F32[columns]`; an elementwise
normalization divides each sum by `rows` into `means`. Workspace is
`4*columns` bytes. The row axis is the only reduced axis, so column order and
the fixed tree are preserved.

#### `gpu_reduce_var_cols`

`emit_column_variance` (`loss_metrics.rs:476-545`) uses the same matrix input
and output shape contract, with output `variances: F32[columns]`. It creates:

```text
sums              F32[columns]
squared_deviations F32[rows, columns]
deviation_sums    F32[1, columns]
normalized        F32[1, columns]
```

The stages are deliberately ordered:

1. `Reduce(Sum, axis 0, keep_dimensions = false)` forms `sums`.
2. `squared_deviation_program` computes
   `(value - sums / rows) * (value - sums / rows)` per cell.
3. `Reduce(Sum, axis 0, keep_dimensions = true)` forms `deviation_sums`.
4. `normalize_f32_count_program` divides the kept-dimension sum by `rows`.
5. A final `Reduce(Sum, axis 0, keep_dimensions = false)` removes the extent-one
   axis and writes `variances`.

The last reduction is a shape publication step over an extent-one axis. It
does not move the division before the source-order sum. The result is the
population variance, not a sample variance. Workspace is
`4*rows*columns + 12*columns` bytes. No atomic reduction or host-side
post-processing is used.

### Per-example distance losses

The three distance emitters all require a rank-two f32 feature matrix, one f32
loss value per row, a finite prepared `margin`, and a fixed row reduction. The
pairing helper enforces equal feature shapes and labels/losses of shape
`[rows]` (`loss_metrics.rs:768-782`). A negative finite margin is accepted by
`finite_parameter`; the scalar formulas use that value without clamping.

#### `gpu_contrastive_loss`

`emit_contrastive_loss` (`loss_metrics.rs:547-589`) requires
`left: F32[rows, dimensions]`, equal-shaped `right`, `labels: F32[rows]`,
`losses: F32[rows]`, and prepared `margin` plus `tree_lanes`. It maps each
feature pair to a squared difference, reduces axis one into one squared
distance per row, and runs `contrastive_loss_program`:

```text
distance_squared = sum((left - right)^2)
distance         = sqrt(distance_squared)
positive         = label * distance_squared
negative         = (1 - label) * max(margin - distance, 0)^2
loss             = positive + negative
```

The program requires the reduced squared distance to be finite and
nonnegative before `SquareRoot`. Labels are not range-checked, so the formula
retains the supplied f32 weighting. `squared_differences` and
`squared_distances` require `4*rows*dimensions + 4*rows` bytes.

#### `gpu_cosine_embedding_loss`

`emit_cosine_embedding_loss` (`loss_metrics.rs:591-651`) has the same boundary
ABI and prepared parameters. Its first elementwise stage emits three images:
`left * right`, `left * left`, and `right * right`. Three independent fixed
`Reduce(Sum, axis 1, Value)` nodes preserve the dot product and both norm-sum
orders. `cosine_embedding_loss_program` then:

```text
left_norm  = sqrt(left_norm_squared)
right_norm = sqrt(right_norm_squared)
cosine     = dot / (left_norm * right_norm + 1e-12)
loss       = label > 0 ? (1 - cosine) : max(cosine - margin, 0)
```

The dot and both squared norms are required finite; the norm sums are also
required nonnegative before square roots. The epsilon is the literal
`1.0e-12` in the owned scalar program. A positive label takes the positive
branch; zero and negative labels take the margin branch. Three feature images
and three row images require `12*rows*dimensions + 12*rows` bytes.

#### `gpu_triplet_loss`

`emit_triplet_loss` (`loss_metrics.rs:653-710`) requires equal-shaped
`anchor`, `positive`, and `negative` f32 matrices, `losses: F32[rows]`, and
finite `margin` plus `tree_lanes`. `triplet_terms_program` emits two squared
feature-difference images:

```text
positive_distance = sum((anchor - positive)^2)
negative_distance = sum((anchor - negative)^2)
loss              = max(positive_distance - negative_distance + margin, 0)
```

The two row reductions are independent fixed trees. The final scalar program
requires both reduced distances finite and nonnegative. A zero margin branch
is inactive because the comparison is strict `> 0`. The two feature images and
two row images require `8*rows*dimensions + 8*rows` bytes.

## Scalar program inventory

The emitter functions above reuse a small set of operation-specific SSA
builders rather than embedding host arithmetic:

| Builder (`loss_metrics.rs`) | Consumers | Actual program |
| --- | --- | --- |
| `binary_accuracy_program:866-893` | `gpu_accuracy_into` | f32 threshold at `0.5`, compare int32 truth flags. |
| `normalize_i32_count_program:895-913` | `gpu_accuracy_into` | int32 count -> f32 -> divide by exact divisor. |
| `normalize_f32_count_program:915-927` | dense accuracy, MSE, global mean, column mean, variance | divide an f32 value by the prepared exact divisor. |
| `checked_class_match_program:929-974` | `gpu_accuracy` | require target class range, compare int32 indexes, convert truth to f32. |
| `class_match_program:976-994` | `gpu_argmax_accuracy_into` | compare two int32 indexes and convert truth to f32. |
| `finite_flag_program:996-1009` | `gpu_has_nan`, `gpu_isfinite_all` | `IsNan` or `IsFinite` to an int32 flag. |
| `hinge_loss_program:1011-1056` | `gpu_hinge_loss` | margin, hinge select, and subgradient select. |
| `positive_or_one_program:1058-1079` | `gpu_kl_div_loss` | replace nonpositive targets with one before log. |
| `kl_divergence_program:1081-1117` | `gpu_kl_div_loss` | positive-target `target * (log(target) - log_probability)`, else zero. |
| `squared_difference_program:1119-1132` | MSE, SSR, contrastive | source-order f32 subtraction and square. |
| `squared_deviation_program:1134-1149` | column variance | divide column sum by row count, subtract, and square. |
| `contrastive_loss_program:1151-1217` | `gpu_contrastive_loss` | finite/nonnegative check, square root, positive and margin branches. |
| `cosine_terms_program:1219-1233` | `gpu_cosine_embedding_loss` | dot and two squared-norm terms. |
| `cosine_embedding_loss_program:1235-1320` | `gpu_cosine_embedding_loss` | checked cosine denominator and label branch. |
| `triplet_terms_program:1322-1356` | `gpu_triplet_loss` | anchor-positive and anchor-negative squared terms. |
| `triplet_loss_program:1358-1396` | `gpu_triplet_loss` | finite/nonnegative distance checks and margin hinge. |
| `require_finite:1398-1406` | cosine and distance programs | append `IsFinite` plus `Require`. |
| `require_finite_nonnegative:1408-1424` | cosine and distance programs | append finite check and `>= 0` `Require`. |

All builders use the shared `scalar_builder`, `scalar_input`,
`scalar_binary`, `scalar_unary`, `scalar_ternary`, and `scalar_finish` helpers
from `materialize.rs:4511-4545`. A builder failure is attached to the
operation's `OperationId` as `GraphMaterializationFailed`. The builders do not
invoke a backend or perform host data inspection.

## In-tree callers and public end-to-end path

### Training validation callers

The production training compiler is the only current in-tree caller that names
these symbols directly:

* `compile_multiclass_validation` constructs validation logits and loss values,
  then calls `gpu_argmax_accuracy_into` for joint dense-target multiclass
  validation (`training/src/compile.rs:7801-7909`). For categorical multiclass
  validation it reduces the `[rows, 1]` int32 target matrix to an I32 class
  vector, calls `gpu_accuracy`, and divides the returned count by
  `validation.rows`.
* `materialize_binary_metrics` constructs one-column probability, target, and
  loss vectors, then calls `gpu_accuracy_into` after the custom binary metric
  fragment is inserted (`training/src/compile.rs:8181-8271`).
* `materialize_multi_target_binary_metrics` calls the preceding per-column
  path for all scalar metrics and calls `gpu_argmax_accuracy_into` once on the
  full probability and dense-target matrices for its accuracy output
  (`training/src/compile.rs:8058-8158`). Temperature scaling is rejected for
  multiple binary target columns before this path is entered.

No current workspace caller names `gpu_contrastive_loss`,
`gpu_cosine_embedding_loss`, `gpu_has_nan`, `gpu_hinge_loss`,
`gpu_isfinite_all`, `gpu_kl_div_loss`, `gpu_mean_all`, `gpu_mse_into`,
`gpu_reduce_mean_cols`, `gpu_reduce_var_cols`, `gpu_ss_res_into`, or
`gpu_triplet_loss` outside their registry, composition, and materializer
definitions. They are not silently substituted into high-level dense training:
for example, the declared training MSE branch uses
`pointwise_loss_program` (`training/src/compile.rs:1142-1149`). Their public
role in this checkout is the advanced `recipe::operations::materialize`
boundary, where a caller can provide the exact ABI and prepared state.

### Compiler callee and graph insertion

`GraphCompiler::materialize` in `training/src/compile.rs:10937-10998` resolves
the unique registry symbol, marks input and output tensor boundaries, reserves
contiguous value and kernel ranges, constructs `MaterializationRequest`, calls
`materialize_composition`, and inserts the returned graph with
`insert_materialized_graph`. The inference compiler has the same request and
graph insertion boundary (`training/src/inference.rs:2008-2076`), although no
current inference call site names one of these fifteen symbols.

When dense training is requested through the public `.run()` path,
`src/training.rs:592-758` validates the declaration, prepares the data, selects
the relevant validation compiler, and propagates an `OperationError` as a
`TrainingCompileErrorKind::Operation` (`training/src/error.rs:50-60`).
`GraphCompiler::finish` validates and round-trips the complete graph through
OGDL, then builds `StaticCalculationProgram::new_with_metrics`
(`training/src/compile.rs:11078-11123`). The metric bindings point at the
caller-owned `[1]` f32 outputs emitted by these operations.

`Train::try_run_with` then executes the compiled program through the native
preparation and executor path (`src/training.rs:869-917`). The operation graph
therefore follows the complete user path:

```text
public Recipe declaration
  -> prepare and validate typed dataset/model
  -> training or advanced operations compiler
  -> OperationRegistry descriptor
  -> MaterializationRequest
  -> loss_metrics emitter
  -> validated CalculationGraph / StaticCalculationProgram
  -> measured native preparation and backend lowering
  -> executor observes output tensors or metric bindings
```

Materialization is not runtime work. Discovery, compilation, allocation, and
native image loading occur before execution; the loop executes the finalized
primitive graph and exposes metric values through the normal metric observation
path. A status, descriptor, or graph construction success is not a measured
numeric result.

## Failure matrix

| Boundary | Failure observed | Examples in this family |
| --- | --- | --- |
| Registry resolution | `UnknownOperation` or `AmbiguousSymbol` | Missing surface row, duplicate symbol passed to `resolve_unique`, or wrong source passed to `resolve_exact`. |
| Composition classification | `WrongLoweringKind` or `MissingConcreteFormula` | A non-composition descriptor passed to `materialize_composition`, or a descriptive composition outside all concrete family tables. |
| Request declaration | `InvalidMaterializationRequest` | Empty input/output sets, duplicate names or tensor IDs, invalid tensor flags, missing named tensors, exact ABI mismatch, wrong dtype, shape mismatch, empty tensor, rank mismatch, or invalid prepared fact. |
| Prepared metadata | `MissingPreparedParameter` or `PreparedParameterTypeMismatch` | Missing `tree_lanes` or `margin`, or a parameter supplied with a variant other than the required `U64` or `F32Bits`. |
| Static shape/count | `UnsupportedConcreteShape` | A count used as an f32 divisor above `16_777_216` from `exact_f32_extent`; generic composition shape or matrix constraints can also fail before emission. |
| Scalar and graph construction | `GraphMaterializationFailed` | Invalid `AxisSet` or `Shape`, scalar SSA builder failure, wrong stage family/count, primitive graph mismatch, or final `CalculationGraph::validate` failure. |
| Identity ownership | `IdentityNamespaceOverlap` or `IdentityNamespaceExhausted` | Boundary IDs inside reserved intermediate ranges, overlapping independently reserved fragments, range-end overflow, or too few caller-reserved IDs. |
| Workspace accounting | `WorkspaceLimitExceeded` or `WorkspaceArithmeticOverflow` | Emitted intermediate bytes exceed the request limit or checked byte totals overflow `u64`. |
| Execution value domain | Device fault through the preallocated fault channel | Out-of-range sparse class target, nonfinite or negative distance square, nonpositive logarithm input, or another live `Require` condition. This occurs after materialization and is not converted to an `OperationError`. |

There is no fallback branch for any failure. The first observed boundary error
is returned with the source-qualified operation context, and a runtime value
fault remains visible to the native execution path.

## Invariants to preserve

1. Ownership is exact `(symbol, source)` membership in `OPERATIONS`; symbol-only
   near matches never dispatch here.
2. Every table row has one registry composition and one concrete emitter branch.
   A listed pair with no branch fails closed as an incomplete dispatcher.
3. Boundary tensors are canonical f32 or i32, have exact names and shapes, and
   remain caller-owned. Intermediate values are contiguous, private, and
   forbidden from aliasing emitted inputs or outputs.
4. Empty tensors and dimensions outside the checked int32 launch domain are
   rejected before graph emission. Counts embedded in f32 are additionally
   limited to the exact binary32 integer domain.
5. Every reduction records its axes, `keep_dimensions`, result kind, and
   prepared power-of-two `tree_lanes`. Fixed trees use the canonical
   lowest-logical-index tie rule and never use an atomic reduction.
6. The variance emitter keeps the source order of mean, squared-deviation sum,
   normalization, and extent-one publication. It must not move the division
   before the fixed-tree sum.
7. Runtime domain checks use Recipe scalar SSA and the preallocated device fault
   channel. Invalid device data is never clipped, replaced, or sent to host
   recovery code.
8. Workspace accounting counts only family-owned intermediate tensors. The
   exact byte totals are `4*N + 4`, `8*rows`, `12*rows + 4`, `4*N`, `8*N`, `4`,
   `4*N + 4`, `4*N`, `4*columns`, `4*rows*columns + 12*columns`,
   `4*rows*dimensions + 4*rows`,
   `12*rows*dimensions + 12*rows`, and
   `8*rows*dimensions + 8*rows` for the operations whose graphs allocate those
   intermediates; elementwise-only hinge loss allocates zero.
9. The resulting graph is validated before it is inserted into a training or
   inference program. Execution, not graph construction, is the proof of the
   resulting metric or loss value.

The crate-wide materializer contract and the shorter source-qualified ABI
summary remain in `ops/MATERIALIZATION.md:541-652`; this page records the
loss/metric module's paired source rows, implementation branches, callers,
state, invariants, and failure behavior.

## Evidence and validation

The authoritative implementation evidence for this page is:

* `operation-surface.txt`, `ops/build.rs`, and `ops/src/registry.rs`, the
  immutable source-qualified rows and descriptor construction;
* `ops/src/composition.rs`, the recipe names, payload contracts, operation
  families, and resolved primitive step sequences;
* `ops/src/materialize/loss_metrics.rs`, the exact ownership table, dispatch
  arms, tensor ABIs, scalar SSA programs, reductions, and workspace-producing
  intermediates;
* `ops/src/materialize.rs` and `ops/src/error.rs`, shared request validation,
  concrete-owner gating, finite expansion, identity and workspace accounting,
  stage checks, graph validation, and error kinds;
* `core/src/scalar.rs`, `language/src/primitive.rs`,
  `kernel/src/stage.rs`, and `kernel/src/llvm.rs`, downstream scalar fault,
  reduction, tie, and native lowering contracts; and
* `training/src/compile.rs`, `training/src/inference.rs`, and
  `src/training.rs`, production caller resolution, graph insertion, metric
  binding, public compilation, and execution boundaries.

Focused structural checks for this documentation and its source boundary are:

```text
cmark ops/.docs/src/materialize/loss_metrics.md
git diff --check -- ops/.docs/src/materialize/loss_metrics.md
cargo check -p recipe-ops
cargo check -p recipe-training
```

These checks establish Markdown structure, whitespace correctness, and
compilation of the registry, materializer, and production compiler callers.
They do not substitute for a CUDA or HSA runtime acceptance run. Such a run
requires a real prepared request, measured hardware, and the public training or
advanced operations entry point to exercise the resulting graph through native
lowering and execution.
