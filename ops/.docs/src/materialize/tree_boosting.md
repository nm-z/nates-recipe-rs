# Tree and boosting materialization

`ops/src/materialize/tree_boosting.rs` is the concrete materializer for the
tree, forest, and boosting rows that have crossed the structured-composition
boundary. It does not call the legacy GPU sources named in
`operation-surface.txt`. Those source paths identify the exact public rows and
preserve source-qualified identity. The module instead emits a finite
`recipe-language::CalculationGraph` made from Recipe-owned scalar SSA programs
and the checked `Gather`, `Scatter`, `Histogram`, `Scan`, `Reduce`, `Sort`, and
`Random` primitives.

The module owns exactly 24 `(symbol, source)` pairs. A symbol match from any
other source is not owned. The pair is accepted only after the registry has
classified it as a `Composition`, shared request validation has succeeded, and
the exact prepared shape and fact contract for the selected emitter is true.
There is no host loop, callback, legacy-kernel fallback, CPU substitute, or
retry path in this boundary.

## Where this family sits

The source-qualified rows are recorded in `operation-surface.txt`, turned into
`OperationDescriptor` values by `ops/src/registry.rs`, and assigned their
descriptive `CompositionRecipe` by `ops/src/composition.rs`. The recipes name
primitive-family stages and static repeat bounds, but they do not contain
tensor names, scalar expressions, primitive parameters, or workspace policy.
`tree_boosting.rs` supplies those missing concrete facts.

The production path is:

```text
operation_registry().resolve_exact(symbol, source)
    -> OperationDescriptor { lowering: Composition(recipe) }
    -> recipe::operations::materialize(request)
    -> recipe_ops::materialize_composition
    -> shared validate_request and concrete-materializer gate
    -> expand_composition from the iteration-shape tensor
    -> dispatch_concrete, including tree_boosting::dispatch
    -> Emitter stages and GraphBuilder tensors/kernels
    -> CalculationGraph::validate
    -> MaterializedComposition
```

The public facade exposes this as `src/facade.rs`'s
`recipe::operations::materialize`. The training and inference compilers also
construct `MaterializationRequest` values directly and call the same
`recipe_ops::materialize_composition` function. Their `Compiler::materialize`
helpers in `training/src/compile.rs` and `training/src/inference.rs` resolve
the symbol with `operation_registry().resolve_unique` (the caller has already
selected an unambiguous public row), then copy the returned graph's tensor
contracts and kernel nodes into the larger program,
attach the caller's iteration domain, and later validate and serialize the
complete `StaticCalculationProgram`. The materializer's checked
`WorkspaceAllocation` and `StageEmission` remain available on
`MaterializedComposition`; the current compiler insertion path consumes the
graph and does not silently replace it with those metadata objects.

`dispatch_concrete` probes family modules in this fixed order:

1. `optimizer_normalization`
2. `solver_fft`
3. `attention_sequence_embedding`
4. `convolution_pooling`
5. `loss_metrics`
6. `indexing_sort_encoding`
7. `graph_cluster_rl`
8. `tree_boosting`
9. `inference_quantization_diffusion`
10. `creation_shape_misc`
11. `training`

Each probe returns `NotOwned` or `Owned(result)`. The tree module is therefore
not selected by the broad Tree family label alone. Its exact `supports` table
must match both the symbol and legacy source, and an owned result returns
immediately without allowing a later family to reinterpret the request.

## Exact ownership and paired cases

The first table is the ownership manifest. Operation-surface line numbers are
the immutable rows in `operation-surface.txt`. Entries grouped in one row have
the same local handler, but each source-qualified descriptor is still distinct
in the registry.

| Source-qualified operation(s) | Surface row and legacy source | Registry recipe and stage shape | Local dispatch |
| --- | --- | --- | --- |
| `gpu_argmax_write_split` | 36, `gpu-core/src/kernels.rs:4155` | `argmax_split_write`, `REDUCE_MAP` | `emit_argmax_split` |
| `gpu_bootstrap_sample` | 63, `gpu-core/src/forest.rs:58` | `bootstrap_sample`, `RANDOM_GATHER` | `emit_bootstrap_sample` |
| `gpu_feature_subset` | 133, `gpu-core/src/forest.rs:82` | `feature_subset`, `GATHER_MAP` | `emit_feature_subset` |
| `gpu_grad_hess_into` | 171, `gpu-core/src/kernels.rs:3703` | `logistic_gradient_hessian`, `MAP_ONLY` | `emit_gradient_hessian` |
| `gpu_histogram_build` | 178, `gpu-core/src/kernels.rs:6745` | `gradient_hessian_histogram`, `MAP_HISTOGRAM` | `emit_tree_histogram(..., true)` |
| `gpu_leaf_finalize` | 201, `gpu-core/src/kernels.rs:4372` | `regularized_leaf_value`, `HISTOGRAM_REDUCE_MAP` | `emit_regularized_leaf(..., false)` |
| `gpu_leaf_reduce` | 202, `gpu-core/src/kernels.rs:4346` | `leaf_statistic_accumulation`, `MAP_SCATTER` | `emit_leaf_reduce` |
| `gpu_leaf_split_apply` | 203, `gpu-core/src/kernels.rs:7173` | `leaf_split_apply`, `TREE_ROUTE_STEPS` | shared `emit_tree_leaf_split` in `materialize.rs` |
| `gpu_lgbm_best_split` | 206, `gpu-core/src/kernels.rs:7080` | `best_histogram_split`, `TREE_HISTOGRAM_STEPS` | `emit_two_bin_split` |
| `gpu_lgbm_hist_subtract` | 207, `gpu-core/src/kernels.rs:7053` | `histogram_subtraction`, `MAP_ONLY` | `emit_histogram_subtraction` |
| `gpu_lgbm_histogram` | 208, `gpu-core/src/kernels.rs:7018` | `tree_gradient_histogram`, `MAP_HISTOGRAM` | `emit_tree_histogram(..., true)` |
| `gpu_lgbm_leaf_reduce` | 209, `gpu-core/src/kernels.rs:7119` | `regularized_leaf_value`, `HISTOGRAM_REDUCE_MAP` | `emit_regularized_leaf(..., true)` |
| `gpu_oblivious_histogram` | 264, `gpu-core/src/kernels.rs:4205` | `tree_gradient_histogram`, `MAP_HISTOGRAM` | `emit_tree_histogram(..., false)` |
| `gpu_oblivious_split_eval` | 268, `gpu-core/src/kernels.rs:4396` | `best_histogram_split`, `TREE_HISTOGRAM_STEPS` | `emit_two_bin_split` |
| `gpu_ordered_target_stats` | 271, `gpu-core/src/catboost.rs:100` | `ordered_target_statistics`, `SORT_SCAN_MAP` | `emit_ordered_target_statistics` |
| `gpu_random_threshold_split` | 292, `gpu-core/src/forest.rs:108` | `random_threshold_split`, `RANDOM_MAP` | `emit_random_threshold` |
| `gpu_scatter_add_by_leaf` | 337, `gpu-core/src/kernels.rs:4323` | `leaf_statistic_accumulation`, `MAP_SCATTER` | `emit_leaf_prediction` |
| `gpu_scatter_add_by_leaf_col` | 338, `gpu-core/src/kernels.rs:4497` | `leaf_statistic_accumulation`, `MAP_SCATTER` | `emit_leaf_prediction` |
| `gpu_split_eval` | 372, `gpu-core/src/kernels.rs:6778` | `best_histogram_split`, `TREE_HISTOGRAM_STEPS` | `emit_two_bin_split` |
| `gpu_tb_histogram` | 394, `gpu-core/src/kernels.rs:3737` | `tree_gradient_histogram`, `MAP_HISTOGRAM` | `emit_tree_histogram(..., false)` |
| `gpu_tb_leaf_sum` | 395, `gpu-core/src/kernels.rs:3830` | `tree_builder_leaf_statistic`, `HISTOGRAM_REDUCE_MAP` | `emit_regularized_leaf(..., true)` |
| `gpu_tb_leaf_val` | 396, `gpu-core/src/kernels.rs:3856` | `tree_builder_leaf_statistic`, `HISTOGRAM_REDUCE_MAP` | `emit_regularized_leaf(..., false)` |
| `gpu_tb_split_eval` | 399, `gpu-core/src/kernels.rs:3770` | `best_histogram_split`, `TREE_HISTOGRAM_STEPS` | `emit_two_bin_split` |
| `gpu_write_split` | 415, `gpu-core/src/kernels.rs:4182` | `write_split_metadata`, `MAP_ONLY` | `emit_write_split` |

The descriptive stage aliases come from the constants in
`ops/src/composition.rs`:

* `MAP_ONLY` is one elementwise stage.
* `REDUCE_MAP`, `GATHER_MAP`, `RANDOM_MAP`, and `RANDOM_GATHER` are two
  stages in the listed order.
* `MAP_HISTOGRAM` is a map followed by one histogram stage. The histogram
  stage may contain two or three kernels, while still consuming one resolved
  recipe step.
* `HISTOGRAM_REDUCE_MAP` is histogram, reduction, then elementwise map.
* `MAP_SCATTER` is map then scatter.
* `SORT_SCAN_MAP` is sort, scan, then map.
* `TREE_HISTOGRAM_STEPS` is histogram, scan, score, then reduction.
* `TREE_ROUTE_STEPS` is gather, branch map, then unique scatter.

For these rows, `OperationDescriptor::family` is `OperationFamily::Histogram`
only for `gpu_histogram_build`; the other 23 descriptors are in the Tree
family. Every row except `gpu_write_split` advertises the
`CompositionPayload::F32AndI32` canonical payload contract. `gpu_write_split`
advertises `CompositionPayload::I32`. Registry metadata also retains the
operation ID, source, definition, alias contract, and determinism contract.
The generic `gpu_*` alias rule is operation-specific, while the
`_into` spelling on `gpu_grad_hess_into` selects the registry's no-alias
contract. The materializer itself emits forbidden input/output alias rules for
each concrete kernel; a later caller can apply the descriptor's public alias
policy without allowing an intermediate to alias accidentally.

Registry determinism is derived from the source-qualified symbol before this
module runs. Symbols containing `bootstrap` or `rand` use the
`CounterBasedRandom` contract. Symbols containing `histogram` or `scatter` use
the `ExplicitAtomicPolicy` contract. The remaining composition rows use
`FixedPrimitiveOrder`, even when a concrete handler such as `gpu_leaf_reduce`
contains an explicitly sequential atomic scatter. The primitive graph is the
authoritative policy for that handler; the descriptor field records the
registry-level classification.

`BOOST_TRAIN_STEPS` is the separate source-specific recipe for `train` and
`train_multiclass` rows. It repeats seven primitive families for the prepared
`boosting_rounds` count. This family module does not own those source-specific
training symbols, because its exact `OPERATIONS` table contains only the 24
rows above. A source-specific training descriptor therefore remains a
different operation even when its recipe mentions the same tree stages.

The same source-agnostic composition match also covers
`gpu_logloss_grad_f32` and `gpu_logloss_grad_mc` beside
`gpu_grad_hess_into`. Only the latter has an exact entry in this module's
`OPERATIONS` table. The two logloss aliases therefore pass registry recipe
validation but fail the concrete-materializer gate and remain visible in the
remaining manifest. This is the intended distinction between a descriptive
recipe match and source-qualified ownership.

## Shared request, state, and graph invariants

`MaterializationRequest` is immutable and contains the descriptor, named input
and output tensors, the one input name used for shape-dependent iteration
resolution, typed `PreparedParameters`, an `IdentityNamespace`, and a
workspace byte limit. Before dispatch, `validate_request` in
`ops/src/materialize.rs` requires:

* `LoweringAvailability::Composition` rather than scalar, primitive, workspace,
  non-calculation, or unsupported lowering;
* at least one input and one output;
* nonempty, unique tensor declaration names and unique tensor IDs across both
  boundaries;
* valid tensor contracts;
* every input marked `external_input` and not `external_output`;
* every output marked `external_output` and not `external_input`.

Each emitter then calls `require_exact_abi`. It compares the request's input,
output, and prepared-parameter *sets* against the exact names listed by that
operation. `input` and `output` return a missing-name error instead of
guessing. `require_dtype` and `require_shape` reject any canonical f32 or i32
contract mismatch. Shape mismatches are `UnsupportedConcreteShape`; missing or
wrongly typed declarations and parameters are
`InvalidMaterializationRequest`, `MissingPreparedParameter`, or
`PreparedParameterTypeMismatch` as appropriate.

All index-bearing primitive constructors in this module use
`IndexBounds::Reject` on axis zero. A verified preparation fact is required
where the emitter cannot derive bounds from a tensor shape, for example
`leaf_indices_verified`, `contribution_table_verified`, or
`destination_indices_unique`. The fact is not a comment: `require_true` fails
when it is absent, false, or not a `Bool` parameter.

`prepared_dimension` reads a `PreparedParameter::U64`, requires a nonzero value
no larger than `MAX_I32_INDEX` (`2_147_483_647`), and reports
`UnsupportedConcreteShape` otherwise. `prepared_i32` performs the explicit
U64-to-i32 conversion for scalar metadata. `prepared_tree_lanes` is shared by
all fixed reductions and scans and requires a power of two in `1..=1024`.
`finite_parameter`, `finite_nonnegative`, and `finite_positive` decode
`F32Bits` and enforce the corresponding domain before embedding a value in
scalar SSA. The random-key helper reads the three U64 words
`seed_low`, `seed_high`, and `stream` without inventing a default.

`gather()` creates an axis-zero checked gather. `scatter(conflict)` creates an
axis-zero checked scatter with the caller-selected `ScatterConflict`.
`reduction()` validates the requested `AxisSet`, operator, result form, and
fixed tree lanes. `inclusive_sum_scan` and `exclusive_sum_scan` both use a
forward sum scan and the prepared lane count; the exclusive helper chooses a
dtype-matched zero identity (`F32Bits(0.0)` or `I32(0)`).

The `Emitter` owns a cursor into the resolved composition. `emit_stage` rejects
an empty stage, rejects emission past the resolved step count, and compares the
last kernel's primitive family with the current recipe step. Multiple kernels
are legal within one stage, which is how paired gradient/Hessian histograms,
parallel reductions, and sort/scan helper work are represented without adding
extra recipe steps. Every kernel receives forbidden input/output alias rules.

`GraphBuilder` allocates intermediates only inside the caller's half-open value
range, accounts each tensor's storage bytes against `workspace_limit`, and
allocates kernels only inside the caller's kernel range. Exhaustion, arithmetic
overflow, or a workspace limit breach fails before a partial result is
returned. On `Emitter::finish`, the graph's own validation checks tensor
contracts, primitive arity and shape, reduction trees, histogram domains,
random distribution, and scalar programs. The resulting
`MaterializedComposition` contains the validated graph, resolved steps,
stage-to-kernel IDs, workspace objects, byte total, and identity namespace.

## Gradient and sampling preparation

### `gpu_argmax_write_split`

`emit_argmax_split` accepts exactly input `gains`, outputs `best_gain`,
`best_feature`, and `best_bin`, and parameters `features`, `bins`, and
`tree_lanes`. `gains` is f32 `[features, bins]`; the three outputs are scalar
`[1]`, with `best_gain` f32 and the coordinates i32. The dimensions are
nonzero and int32-indexable, and the lane count fixes the reduction tree.

The graph first reduces `gains` over axes `[0, 1]` with
`ReduceOperator::Maximum` and `ReduceResult::ValueAndIndex`. The reduction
publishes the maximum gain and its flattened i32 index. The second map divides
that index by `bins` for the feature coordinate and takes its remainder for the
bin coordinate. The primitive lowering's value/index contract retains the
lowest logical index when gains tie, so the selected split is deterministic.
`split_index_program` rejects a nonrepresentable bin divisor while building the
SSA program. A missing name, wrong shape or dtype, invalid dimension, bad lane
count, or scalar builder failure stops materialization before graph return.

### `gpu_bootstrap_sample`

The exact ABI is input `row_ids`, output `sampled_rows`, and parameters `rows`,
`samples`, `seed_low`, `seed_high`, and `stream`. Both tensors are i32,
`row_ids` has shape `[rows]`, and `sampled_rows` has shape `[samples]`; `rows`
and `samples` are nonzero int32-indexable dimensions. The random key is
counter-based and uses exactly Philox4x32-10.

The first stage emits an input-free `RandomMap` with
`UniformI32 { low: 0, high_exclusive: rows }` into an intermediate `[samples]`
index image. The second stage gathers `row_ids` at those checked random
indices into `sampled_rows`. The conversion of `rows` to the i32 exclusive
bound is explicit, and the primitive validator also rejects an invalid random
range. There is no attempt to make the sample unique: this is bootstrap
sampling with replacement.

### `gpu_feature_subset`

The exact ABI is i32 `feature_ids[features]`, i32
`selection_indices[selected]`, and i32 `selected_features[selected]`. The
parameters are `features`, `selected`, and the true facts
`selection_indices_verified` and `selection_indices_unique`. The selected
count must not exceed the feature count. The first stage gathers
`feature_ids[selection_indices]` with rejected out-of-range indices. The second
stage applies an explicit i32 identity scalar program to publish the external
output. The verified and unique facts are mandatory even though the gather
primitive itself checks its index dtype and shape.

### `gpu_grad_hess_into`

The exact ABI is f32 `probabilities`, `weights`, `gradients`, and `hessians`,
plus i32 `targets` and `mask`, all shape `[rows]`. Parameters are `rows` and
the i32-convertible `class_index`. The one map stage uses
`gradient_hessian_program`:

```text
target_matches = f32(target == class_index)
gradient = (probability - target_matches) * weight * mask
variance = probability * (1 - probability) * weight
hessian = clamp(variance, 0.001, 1_000_000.0) * mask
```

`mask` is checked in scalar SSA to be exactly zero or one before conversion to
f32. The source code does not add finite or nonnegative checks for probability
or weight; those domains are therefore not implied by this materializer. The
class index conversion, exact tensor shapes, dtypes, and scalar-program
construction remain checked.

## Histogram construction and leaf statistics

### Histogram aliases

`emit_tree_histogram` is parameterized only by whether counts are exposed. The
count-bearing pair is `gpu_histogram_build` and `gpu_lgbm_histogram`; each
requires inputs `bin_indices`, `gradient_contributions`, and
`hessian_contributions`, outputs `gradient_histogram`, `hessian_histogram`,
and `count_histogram`, and parameters `contributions`, `histogram_bins`, and
true `contribution_table_verified`. The count-free pair is
`gpu_oblivious_histogram` and `gpu_tb_histogram`; they use the same inputs and
parameters but expose only the two f32 histogram outputs.

In both variants, `bin_indices` is i32 `[contributions]`, both contribution
vectors are f32 `[contributions]`, and each f32 histogram is `[histogram_bins]`.
The optional count histogram is i32 with the same bin shape. The two-stage
graph first maps the three contribution vectors through the identity
`histogram_contribution_program`. The second stage emits one weighted f32
histogram for gradients and one for Hessians, plus an unweighted i32 histogram
when counts are enabled. Histogram bin count conversion to u32 is checked.
The primitive histogram lowering rejects bins outside `1..=i32::MAX` and
publishes its checked out-of-range fault for any invalid bin index. The
preparation fact means the contribution table was generated for this exact
layout; the materializer does not infer or repair it.

The registry recipe names differ intentionally: `gpu_histogram_build` is the
legacy gradient/Hessian histogram row, while the LightGBM, oblivious, and tree
builder aliases are tree gradient histograms. Their concrete graph is shared
only after exact source ownership has been established.

### Regularized leaf aliases

`emit_regularized_leaf` serves two pairs. `gpu_leaf_finalize` and
`gpu_tb_leaf_val` pass `expose_sums = false`, so their only output is
`leaf_value`. `gpu_lgbm_leaf_reduce` and `gpu_tb_leaf_sum` pass
`expose_sums = true`, adding scalar outputs `leaf_gradient` and
`leaf_hessian`. All four require inputs `leaf_indices`,
`gradient_contributions`, and `hessian_contributions`, and parameters
`contributions`, `leaves`, `lambda`, `contribution_table_verified`, and
`tree_lanes`.

The concrete shape domain deliberately supports exactly one leaf. Inputs are
`leaf_indices: i32[contributions]` and two f32 contribution vectors of the same
shape. Every output is a scalar `[1]` with the declared dtype. `lambda` must be
finite and positive, and the contribution-table fact must be true. The first
stage emits two weighted one-bin histograms, the second emits two fixed-tree
sum reductions, and the third maps the reduced values with
`regularized_leaf_program`:

```text
require(hessian >= 0)
denominator = hessian + lambda
require(denominator > 0)
leaf_value = -gradient / denominator
```

The sum-bearing aliases expose the reduced gradient and Hessian alongside the
value. One-bin histogram bounds still reject any leaf index other than zero;
there is no hidden multi-leaf path. A non-finite or nonpositive lambda, a
`leaves` value other than one, a bad table fact, invalid lane count, shape or
dtype mismatch, or a device-side Hessian/denominator `Require` failure stops
the operation.

### `gpu_leaf_reduce`

This operation has the `leaf_statistic_accumulation` recipe, but its ABI is
different from the prediction-update aliases. Inputs are i32
`leaf_indices[rows]`, f32 `gradients[rows]`, f32 `hessians[rows]`, and f32
`gradient_base[leaves]` plus `hessian_base[leaves]`. Outputs are f32
`leaf_gradients[leaves]` and `leaf_hessians[leaves]`. Parameters are `rows`,
`leaves`, true `leaf_indices_verified`, and true `bases_zero`.

The map stage copies the three row vectors through
`histogram_contribution_program`. The scatter stage atomically adds mapped
gradients and Hessians into the two base arrays with
`ScatterConflict::Atomic { operation: Add, ordering: SequentiallyConsistent }`.
The base arrays are caller-supplied state, so `bases_zero` is a preparation
fact rather than a materializer-side fill operation. The verified leaf index
fact supplies the checked `[0, leaves)` domain; rejected indices remain a real
primitive fault. No leaf count restriction equivalent to the one-leaf leaf
value path is applied here.

### `gpu_scatter_add_by_leaf` and `gpu_scatter_add_by_leaf_col`

Both source rows dispatch to `emit_leaf_prediction`, despite sharing the
`leaf_statistic_accumulation` recipe with `gpu_leaf_reduce`. Their exact ABI is
f32 `prediction_base[prediction_elements]`, f32 `leaf_values[leaves]`, i32
`leaf_indices[updates]`, and i32 `destination_indices[updates]`, producing
f32 `predictions[prediction_elements]`. Parameters are
`prediction_elements`, `updates`, `leaves`, finite `learning_rate`, and true
`leaf_indices_verified` and `destination_indices_unique`.

The first stage has three kernels: gather leaf values by leaf index, gather
the current prediction by destination index, and map

```text
mapped_update = current_prediction + learning_rate * selected_leaf_value
```

The second stage uniquely scatters `mapped_update` over `prediction_base` at
`destination_indices` into `predictions`. `learning_rate` is only required to
be finite, so a negative rate is representable. The two verified facts carry
the leaf and destination domains. The unique-scatter contract is required even
though the primitive also rejects out-of-range indices.

## Split selection, routing, and subtraction

### `gpu_leaf_split_apply`

This row is owned by `tree_boosting::dispatch` but reuses the shared
`emit_tree_leaf_split` implementation in `ops/src/materialize.rs`, because the
same three primitive stages are also used by the inference tree materializer.
Inputs are flattened f32 `features`, and equal-shaped row vectors of i32
`feature_indices`, i32 `assignments`, i32 `destination_indices`, and f32
`thresholds`; the output is i32 `updated_assignments`. The only prepared fact
is true `destination_indices_unique`.

The graph gathers `features[feature_indices]` with rejected bounds, maps each
row through `tree_branch_program`, and uniquely scatters into the output:

```text
branch = int32(feature > threshold)
next_assignment = assignment * 2 + branch
```

`features` must be rank one. The row tensors and output must have exactly the
feature-index shape, with the listed f32/i32 dtypes. The scalar program has no
finite threshold requirement, and a destination index that is not in range
remains a checked scatter fault.

### The four two-bin split evaluators

`gpu_lgbm_best_split`, `gpu_oblivious_split_eval`, `gpu_split_eval`, and
`gpu_tb_split_eval` are four exact source rows sharing `emit_two_bin_split`.
Their registry recipe is `best_histogram_split`, whose four resolved stages
are histogram, scan, score, and value/index reduction. The materializer emits
multiple kernels inside those four stages:

1. two weighted two-bin histograms copy the f32 gradient and Hessian inputs;
2. inclusive sum scans form gradient, Hessian, and i32 count prefixes;
3. gathers read the total gradient and Hessian at `total_index`, then an
   elementwise program computes a gain for each bin;
4. an identity map publishes `feature_zero`, a gather publishes
   `best_left_count`, and a maximum value/index reduction publishes
   `best_gain` and `best_bin`.

The exact inputs are i32 `bin_indices[2]`, f32
`gradient_histogram[2]`, f32 `hessian_histogram[2]`, i32
`count_histogram[2]`, and scalar i32 `left_count_index`, `total_index`, and
`feature_zero`. Outputs are scalar f32 `best_gain` and scalar i32
`best_feature`, `best_bin`, and `best_left_count`. Parameters are
`features`, `bins`, `nodes`, positive finite `lambda`, nonnegative finite
`minimum_child_weight`, `tree_lanes`, and these required true facts:

```text
bin_indices_identity
left_count_index_zero
total_index_one
feature_zero_is_zero
count_histogram_nonnegative
positive_gain_guaranteed
```

The concrete shape gate requires exactly one feature, two bins, and one node.
`two_bin_gain_program` first requires all four f32 statistics to be finite and
the total Hessian to be nonnegative. It accepts only bin zero or one. Bin zero
is legal only when both its left Hessian and its derived right Hessian meet
`minimum_child_weight`; bin one is the terminal sentinel candidate. For a
legal left candidate it computes

```text
right_gradient = total_gradient - left_gradient
right_hessian = total_hessian - left_hessian
gain = left_gradient^2 / (left_hessian + lambda)
     + right_gradient^2 / (right_hessian + lambda)
     - total_gradient^2 / (total_hessian + lambda)
```

Bin zero receives this gain and bin one receives `-f32::MAX`. The scalar
`Require` on candidate legality and the `positive_gain_guaranteed` fact keep
the division domain explicit. The final value/index reduction retains the
lowest bin index on a tie. Counts are scanned for the selected left count and
all gather/histogram indices remain checked.

### `gpu_lgbm_hist_subtract`

This one-stage map accepts six inputs: f32
`parent_gradients`, `parent_hessians`, `child_gradients`, and
`child_hessians`, plus i32 `parent_counts` and `child_counts`. It produces
f32 `remaining_gradients` and `remaining_hessians`, and i32
`remaining_counts`, all shape `[elements]`, with the sole parameter
`elements`.

`histogram_subtraction_program` computes the three elementwise differences:

```text
remaining_gradient = parent_gradient - child_gradient
remaining_hessian = parent_hessian - child_hessian
remaining_count = parent_count - child_count
```

The materializer does not impose nonnegative count or Hessian facts. If a
caller needs those domains for a later split, that later operation's own
prepared facts and scalar `Require` checks are the boundary. Missing names,
wrong shapes or dtypes, and scalar-program construction failures are rejected
before the graph is returned.

## Ordered statistics and random split metadata

### `gpu_ordered_target_stats`

The CatBoost row accepts i32 `ordering_keys[rows]`, f32 `targets[rows]`, f32
`sum_base[rows]`, and f32 `count_base[rows]`, producing f32 `encoded[rows]`.
Parameters are `rows`, finite `prior`, positive finite `smoothing`,
`tree_lanes`, and true `one_category`, `ordering_keys_unique`, and
`bases_zero` facts.

The three recipe stages are concrete as follows:

1. A stable ascending axis-zero sort emits sorted keys and the original
   positions.
2. A gather reorders targets by those positions, an identity map creates a
   f32 one vector, and two exclusive f32 sum scans create prefix target sums
   and prefix counts. Exclusive scans provide leave-current-out values.
3. Unique scatters add the prefix images to the supplied sum and count bases
   at the original positions, then an elementwise map computes

   ```text
   encoded = (prefix_sum + prior * smoothing)
           / (prefix_count + smoothing)
   ```

The stable sort and unique-scatter facts are required preparation state. The
primitive sort emits i32 positions and uses the repository's stable total-order
and original-index tie policy; rejected gather/scatter positions remain real
faults. `prior` may be negative, while `smoothing` must be finite and strictly
positive. The source contract does not add a separate target finite check.

### `gpu_random_threshold_split`

Inputs and output are scalar f32 `minimum`, `maximum`, and `threshold`.
Parameters are the three U64 random-key words. The first stage emits a
counter-keyed Philox4x32-10 `UniformF32` scalar. The second stage runs
`threshold_program`, which requires all three values to be finite and
`minimum <= maximum`, then computes:

```text
threshold = minimum + uniform * (maximum - minimum)
```

The primitive random validator requires exactly ten Philox rounds. There is no
host-side random state and no replacement RNG path. Any missing key word,
wrong scalar contract, non-finite endpoint, reversed interval, or scalar
builder failure is an error.

### `gpu_write_split`

This one-stage map accepts i32 `feature_base[slots]`, `bin_base[slots]`, and
`write_mask[slots]`, producing i32 `split_features[slots]` and
`split_bins[slots]`. Parameters are `slots`, i32-convertible prepared
`feature` and `bin`, and true `write_mask_one_hot`.

`write_split_program` validates every mask lane as zero or one, then selects
the prepared feature and bin constants for mask one and preserves the
corresponding base values for mask zero. The two selected vectors are emitted
by one elementwise node. `slots` uses the nonzero int32-indexable dimension
helper; prepared feature and bin values use checked U64-to-i32 conversion.
The materializer does not infer which slot is active or mutate a base array in
place.

## Scalar-program and primitive details

The local scalar builders are deliberately small and reused only where their
input/output contracts match:

| Builder | Inputs and checks | Result |
| --- | --- | --- |
| `bool_mask` | i32 mask, `Require(mask == 0 or mask == 1)` | original mask expression |
| `split_index_program` | i32 flattened index and checked i32 bin divisor | quotient feature and remainder bin |
| `gradient_hessian_program` | f32 probability/weight, i32 target/mask, class index | logistic gradient and clamped Hessian |
| `histogram_contribution_program` | i32 bin, f32 gradient, f32 Hessian | unchanged typed triple |
| `regularized_leaf_program` | f32 gradient/Hessian, positive lambda | optional sums and `-gradient/(hessian+lambda)` |
| `histogram_subtraction_program` | f32 parent/child pairs and i32 counts | three typed differences |
| `two_bin_gain_program` | finite f32 stats, i32 bin, lambda and child-weight policy | legal gain or terminal sentinel |
| `constant_one_program` | one consumed f32 value | f32 one, preserving shape |
| `ordered_statistic_program` | f32 prefix sum/count, prior, smoothing | smoothed posterior statistic |
| `threshold_program` | finite ordered endpoints and uniform | interpolated threshold |
| `leaf_prediction_program` | current f32 prediction, leaf value, finite rate | updated prediction |
| `write_split_program` | f32-free i32 bases, one-hot mask, prepared constants | selected or preserved split metadata |

`constant_one_program` uses a scalar `Select` with an always-true i32
condition rather than a separate fill primitive, preserving the composition's
map stage and the caller-provided shape. Every builder reports
`GraphMaterializationFailed` through the shared `language_error` adapter when
the scalar program builder rejects an expression or output list.

At the primitive boundary, `recipe-language` validates the resulting arity,
dtypes, shapes, reduction axes, tree lanes, histogram bin count, random
distribution, sort index dtype, gather bounds policy, and scatter update shape.
`recipe-primitives` then lowers these kinds to fixed-tree device stages. Value
and index reductions must use `ReduceOperator::Minimum` or `Maximum` and retain
the lowest logical index; histogram and gather/scatter stages carry explicit
int32 checked-fault contracts. These lowerings are downstream of this module,
but their validation is why the module never substitutes unchecked indexing or
an unrecorded floating-point reduction order.

## Failure paths and rejected states

The normal public request fails closed at the first concrete boundary that
observes an invalid state:

| Observed condition | Result |
| --- | --- |
| Descriptor is not a structured composition, or tensor boundary is empty/invalid | `WrongLoweringKind`, `InvalidMaterializationRequest`, or tensor `GraphMaterializationFailed` |
| Exact tensor or parameter name set differs | `InvalidMaterializationRequest` from `require_exact_abi` |
| Missing or wrongly typed prepared value/fact | `MissingPreparedParameter` or `PreparedParameterTypeMismatch` |
| Dtype mismatch | `InvalidMaterializationRequest` |
| Shape, dimension, one-leaf, or two-bin policy is outside the implemented concrete domain | `UnsupportedConcreteShape` |
| Non-finite, nonpositive, negative, or reversed scalar policy value | `InvalidMaterializationRequest` |
| Scalar SSA construction, `AxisSet`, or graph primitive validation fails | `GraphMaterializationFailed` |
| Emitter emits too few/many stages, an empty stage, or a different primitive family | `GraphMaterializationFailed` |
| Caller value/kernel reservation overflows or is exhausted | `IdentityNamespaceExhausted` |
| Intermediate byte arithmetic overflows or exceeds `workspace_limit` | `WorkspaceArithmeticOverflow` or `WorkspaceLimitExceeded` |
| Runtime histogram, gather, or scatter index is outside its checked domain | device checked-fault path from the lowered primitive |
| Runtime scalar `Require` fails for a mask, Hessian, gain candidate, or threshold | device checked-fault path from the lowered scalar program |

The `supports` predicate is exact and source-qualified. If it returns false,
`tree_boosting::dispatch` returns `FamilyDispatch::NotOwned`; the shared
dispatcher tries the other declared families. A true `supports` result enters
the symbol match and returns `FamilyDispatch::Owned(result)`. The final
unreachable symbol arm reports `GraphMaterializationFailed` with
`tree/boosting dispatch is incomplete for ...`, making an ownership-table and
match drift visible rather than selecting a nearby implementation.

`materialize_composition` checks `has_concrete_materializer` before expansion.
For a composition descriptor with no exact family owner, it returns
`MissingConcreteFormula` and records the row in
`remaining_composition_manifest`; no emitter, workspace, graph, or identity
reservation is partially produced. For one of the 24 owned pairs, all shared
validation and the selected emitter run before `CalculationGraph::validate`.
There is no recovery branch that changes the requested symbol, source, shape,
or prepared state.

## End-to-end role in execution

The materialized graph is preparation-time state. The graph's input and output
tensors retain the caller's IDs, while intermediates and kernel templates use
the caller-reserved `IdentityNamespace`. Training and inference compilers copy
those graph contracts and nodes into their own builders, attach iteration
domains, and validate the assembled graph. Their canonical OGDL round trip then
feeds the normal planner, scheduler, kernel lowering, native image preparation,
and executor lifecycle. The tree module therefore contributes calculation and
transfer graph nodes only; it does not allocate device memory, launch kernels,
perform discovery, or execute a boosting round at materialization time.

For a repeated source-specific training composition, `expand_composition`
unrolls the prepared `boosting_rounds` bound before any graph node is emitted.
For the individual rows documented here, all dimensions and iteration shapes
are fixed in the request. The resulting graph is immutable, and every stage's
dependency is represented by value IDs and graph node order. A later backend
cannot observe a hidden host loop or infer missing tree state from operation
nouns.

## Evidence and validation

The implementation evidence is concentrated in:

* `ops/src/materialize/tree_boosting.rs`, the exact ownership table,
  dispatch arms, emitters, scalar programs, and helper contracts;
* `ops/src/materialize.rs`, shared request validation, composition expansion,
  `Emitter`, `GraphBuilder`, `emit_tree_leaf_split`, and the concrete-family
  predicate;
* `ops/src/composition.rs`, the registry recipe names, payload domains, and
  stage arrays;
* `ops/src/registry.rs` and `operation-surface.txt`, source-qualified
  descriptor identity and immutable legacy rows;
* `training/src/compile.rs` and `training/src/inference.rs`, the production
  graph insertion callers; and
* `language/src/primitive.rs` and `primitives/src/lower.rs`, the downstream
  primitive contracts that validate and lower the emitted graph.

The documentation and source boundary can be checked with:

```text
cmark ops/.docs/src/materialize/tree_boosting.md
git diff --check -- ops/.docs/src/materialize/tree_boosting.md
cargo check -p recipe-ops
cargo check -p recipe-training
```

These checks establish Markdown structure, whitespace correctness, and
compilation of the registry, materializer, and production compiler callers.
They do not claim a CUDA or HSA runtime acceptance run. Such a run requires a
real prepared request, measured hardware, and the public training or inference
entry point to exercise the resulting graph through native lowering and
execution.
