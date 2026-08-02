# Optimizer and normalization materialization

This document describes the concrete materialization boundary implemented by
[`ops/src/materialize/optimizer_normalization.rs`](../../../src/materialize/optimizer_normalization.rs)
and the shared emitters in
[`ops/src/materialize.rs`](../../../src/materialize.rs). The module owns 23
source-qualified legacy operation descriptors. It does not infer an operation
from a name alone, reuse a nearby legacy source, or execute a host-side loop.
It accepts one immutable `MaterializationRequest`, resolves the finite
`CompositionRecipe` selected by the registry, emits Recipe-owned primitive
kernels and scalar SSA programs, and returns a validated
`MaterializedComposition`.

The source strings in the registration table are the preserved identities from
`operation-surface.txt`. The old `gpu-core` tree is not a workspace crate here;
those strings are identity and provenance, while the Rust emitters below are
the current owned semantics. A caller must therefore preserve both the public
symbol and its source when resolving an ambiguous operation.

## Owned descriptors and paired Rust dispatch

`OPERATIONS` is the only ownership registration for this family. Its
`supports` function compares the exact pair `(descriptor.symbol,
descriptor.source)`. Once that comparison succeeds, `dispatch` switches on the
symbol and calls exactly one shared emitter. The table and match are kept as a
pair deliberately: adding a row without a branch is an explicit
`GraphMaterializationFailed`, and a branch cannot claim an operation from a
different source.

| Surface line | Public symbol | Preserved source | Composition recipe | Rust dispatch callee |
| ---: | --- | --- | --- | --- |
| 22 | `gpu_adagrad_update` | `gpu-core/src/optimizers.rs:149` | `adagrad_update`, `OPTIMIZER_STEPS` | `emit_adagrad_update` |
| 23 | `gpu_adam_update` | `gpu-core/src/kernels.rs:6421` | `adam_update`, `OPTIMIZER_STEPS` | `emit_adam_update(request, emitter, false)` |
| 24 | `gpu_adamw_update` | `gpu-core/src/kernels.rs:6454` | `adamw_update`, `OPTIMIZER_STEPS` | `emit_adam_update(request, emitter, true)` |
| 45 | `gpu_batchnorm_backward` | `gpu-core/src/kernels.rs:6388` | `batch_normalization_backward`, `NORMALIZE_BACKWARD_STEPS` | `emit_batch_normalization_backward` |
| 46 | `gpu_batchnorm_forward` | `gpu-core/src/kernels.rs:6322` | `batch_normalization_training`, `NORMALIZE_STEPS` | `emit_batch_normalization_forward` |
| 47 | `gpu_batchnorm_inference` | `gpu-core/src/kernels.rs:6355` | `batch_normalization_inference`, `MAP_ONLY` | `emit_batch_normalization_inference` |
| 62 | `gpu_bn_update_running` | `gpu-core/src/attention.rs:454` | `batch_normalization_running_update`, `MAP_ONLY` | `emit_batch_normalization_running_update` |
| 170 | `gpu_grad_clip_norm` | `gpu-core/src/kernels.rs:6670` | `global_gradient_norm_clip`, `MAP_REDUCE_MAP` | `emit_gradient_clip_norm` |
| 194 | `gpu_lamb_phase1` | `gpu-core/src/optimizers.rs:174` | `lamb_moment_update`, `OPTIMIZER_STEPS` | `emit_lamb_phase_one` |
| 195 | `gpu_lamb_phase2` | `gpu-core/src/optimizers.rs:217` | `lamb_trust_ratio_update`, `LAMB_TRUST_STEPS` | `emit_lamb_phase_two` |
| 196 | `gpu_layernorm_backward_f32` | `gpu-core/src/nn_f32.rs:339` | `layer_normalization_backward`, `NORMALIZE_BACKWARD_STEPS` | `emit_layer_normalization_backward` |
| 197 | `gpu_layernorm_backward_full_into` | `gpu-core/src/kernels.rs:3491` | `layer_normalization_backward`, `NORMALIZE_BACKWARD_STEPS` | `emit_layer_normalization_backward` |
| 198 | `gpu_layernorm_f32` | `gpu-core/src/nn_f32.rs:313` | `layer_normalization`, `NORMALIZE_STEPS` | `emit_layer_normalization` |
| 199 | `gpu_layernorm_into` | `gpu-core/src/kernels.rs:3205` | `layer_normalization`, `NORMALIZE_STEPS` | `emit_layer_normalization` |
| 200 | `gpu_layernorm_opt_into` | `gpu-core/src/kernels.rs:3233` | `layer_normalization`, `NORMALIZE_STEPS` | `emit_layer_normalization` |
| 214 | `gpu_lion_update` | `gpu-core/src/optimizers.rs:243` | `lion_update`, `OPTIMIZER_STEPS` | `emit_lion_update` |
| 252 | `gpu_momentum_update` | `gpu-core/src/optimizers.rs:95` | `momentum_update`, `OPTIMIZER_STEPS` | `emit_momentum_update` |
| 259 | `gpu_nadam_update` | `gpu-core/src/optimizers.rs:273` | `nadam_update`, `OPTIMIZER_STEPS` | `emit_nadam_update` |
| 313 | `gpu_rmsnorm` | `gpu-core/src/attention.rs:287` | `canonical_f32_rms_normalization`, `NORMALIZE_STEPS` | `emit_rms_normalization` |
| 314 | `gpu_rmsnorm_backward` | `gpu-core/src/attention.rs:313` | `rms_normalization_backward`, `NORMALIZE_BACKWARD_STEPS` | `emit_rms_normalization_backward` |
| 315 | `gpu_rmsnorm_f64` | `gpu-core/src/infer_ops.rs:335` | `canonical_f32_rms_normalization`, `NORMALIZE_STEPS` | `emit_rms_normalization` |
| 316 | `gpu_rmsnorm_f64_nogamma` | `gpu-core/src/infer_ops.rs:360` | `canonical_f32_rms_normalization`, `NORMALIZE_STEPS` | `emit_rms_normalization` |
| 317 | `gpu_rmsprop_update` | `gpu-core/src/optimizers.rs:121` | `rmsprop_update`, `OPTIMIZER_STEPS` | `emit_rmsprop_update` |

The registry reaches these entries through
`registry::lowering`: `CompositionRecipe::for_entry` returns a structured
recipe for each symbol, and `describe` stores that recipe together with the
source-qualified `OperationId`, canonical dtype contract, family, alias
contract, determinism contract, and optional legacy dtype marker. The
composition recipes use canonical f32 payloads for every row in this table.
The two `*_f64` RMS rows retain `legacy_dtype = F64` as provenance, but the
composition explicitly replaces that payload with f32; the old f64 buffer is
not admitted.

For symbols ending in `_into`, registry alias metadata is `NoAlias`. Other
GPU rows are `OperationSpecific` at the public descriptor level. The concrete
materializer is stricter for every emitted primitive: `GraphBuilder::emit`
installs a forbidden rule for every input/output pair. A caller cannot make an
in-place alias appear by choosing a tensor ID that happens to match another
declaration.

## Preparation and end-to-end call chain

The public facade exposes this path as
`recipe::operations::materialize` in [`src/facade.rs`](../../../src/facade.rs):

```text
resolve_exact(symbol, source) or resolve(symbol)
        |
        v
MaterializationRequest::new
        |
        v
materialize_composition
  validate_request
  has_concrete_materializer
  locate iteration_shape_input
  expand_composition
  Emitter::new / GraphBuilder::new
  dispatch_concrete
    optimizer_normalization::dispatch
  Emitter::finish / CalculationGraph::validate
        |
        v
MaterializedComposition
```

`MaterializationRequest` is immutable and carries the descriptor, named input
and output tensor declarations, the name of the input whose shape supplies
iteration bounds, a `BTreeMap<String, PreparedParameter>`, a caller-reserved
`IdentityNamespace`, and a `ByteCount` workspace limit. Inputs must be marked
`external_input`; outputs must be non-input and marked `external_output`.
Names and tensor IDs are unique across both lists, and each tensor must pass
the language tensor validator before a family emitter is reached.

`expand_composition` first requires `LoweringAvailability::Composition` and
validates the recipe. It resolves fixed, shape-derived, and prepared-parameter
bounds before any graph node is created. These 23 recipes have no dynamic host
loop. `OPTIMIZER_STEPS` resolves to two elementwise stages. `MAP_ONLY` resolves
to one elementwise stage. `NORMALIZE_STEPS` resolves to elementwise, reduce,
reduce, elementwise stages. `NORMALIZE_BACKWARD_STEPS` resolves to
elementwise, reduce, reduce, elementwise, reduce. `MAP_REDUCE_MAP` resolves to
elementwise, reduce, elementwise. `LAMB_TRUST_STEPS` resolves to elementwise,
reduce, reduce, elementwise.

Each `ResolvedStep` records its primitive family, role, surrounding resolved
iterations, and a dependency on the immediately preceding ordinal. The
`Emitter` cursor must consume exactly one resolved step per `emit_stage` call.
The last kernel in a stage is the primary kernel used to check the resolved
family. A stage may contain several kernels, which is how a family emitter
keeps parallel reductions or affine outputs in one ordered stage while still
using the same finite recipe. Emitting too many, too few, or the wrong primary
family is a `GraphMaterializationFailed` error.

`GraphBuilder::intermediate` allocates a contiguous f32 tensor in the reserved
value range, records its storage bytes in `WorkspaceAllocation`, and rejects
both identity exhaustion and a workspace total above the request limit.
`GraphBuilder::emit` allocates a kernel ID in the reserved kernel range,
records `PrimitiveKind` plus forbidden alias rules, and leaves all execution
ordering to the resulting calculation graph. `Emitter::finish` requires the
cursor to equal the resolved-step count, then calls `CalculationGraph::validate`
before returning graph, stages, resolved composition, workspace, and the
identity namespace.

The two production compiler wrappers use the same boundary. The training
`GraphCompiler::materialize` and inference `InferenceGraphCompiler::materialize`
clone value contracts, mark request inputs and outputs at the boundary, reserve
their materialization value/kernel ranges, call
`materialize_composition`, and insert the returned tensors and calculation
nodes into the larger graph. The inserted kernels receive the caller's
iteration domain. Later planning and native execution consume that graph; this
module does not discover hardware, allocate device memory, or execute kernels.

The current real call site for this family is GGUF Llama inference:
`training/src/gguf_llama.rs::rms_norm` calls the generic inference wrapper with
`gpu_rmsnorm`, f32 `[rows, columns]` values and scale, positive epsilon, and
`MAXIMUM_REDUCTION_TREE_LANES`. Dense model batch/layer normalization and the
training compiler's global clip and AdamW update currently use their own direct
graph-builder routines in `training/src/compile.rs` (`normalize_*`,
`global_clip`, and `adamw_update`); those paths do not dispatch through this
module. The public operations facade and either generic compiler wrapper can
still materialize any of the 23 rows when a caller supplies the exact ABI below.

## Shared contracts and failure boundary

All concrete tensor payloads here are canonical f32. The matrix normalizers
(batch training forward/backward, layer normalization, and RMS normalization)
require rank two `[rows, columns]`, nonzero extents, each extent no larger than
`MAX_I32_INDEX`, and exact f32 representation of each divisor. Batch inference
uses the language broadcast contract instead, and running-statistics updates
use rank-one statistics. `exact_f32_extent` rejects a dimension above
`16_777_216` before embedding it in a scalar program.
All reduction paths require `tree_lanes` as a u32 power of two in `1..=1024`.
`sum_reduction` creates a `ReduceOperator::Sum` with an explicit `AxisSet`,
`keep_dimensions` choice, value result, and the supplied tree lane count.

`require_exact_abi` compares sets, not prefixes. An omitted or extra tensor name
or prepared parameter is an `InvalidMaterializationRequest`. `input` and
`output` then perform the named lookup and produce the same request error when
the selected declaration is absent. `require_dtype` rejects a non-f32 tensor;
`require_same_tensor_contract` requires both dtype and shape equality;
`require_shape` reports an `UnsupportedConcreteShape` for an operation-specific
shape mismatch. Missing or wrongly typed typed parameters are
`MissingPreparedParameter` and `PreparedParameterTypeMismatch`.

The shared scalar helpers build `ScalarProgram` values through
`ScalarProgramBuilder`. Builder, opcode, scalar-domain, and final SSA failures
are converted to `GraphMaterializationFailed` and tagged with the operation ID.
Device-side `Require` nodes remain in the emitted scalar program for positive
denominators, nonnegative squared statistics, and other domain checks. They do
not become host fallback branches.

The outer boundary can additionally return:

* `WrongLoweringKind` when a caller supplies a scalar, primitive, workspace, or
  non-calculation descriptor;
* `InvalidMaterializationRequest` for empty declarations, duplicate names or
  IDs, missing iteration-shape input, external-flag violations, ABI mismatches,
  invalid finite parameters, or an invalid reduction construction;
* `UnsupportedConcreteShape` for rank, zero extent, int32-index, exact-f32,
  singleton, or vector-shape violations;
* `MissingConcreteFormula` when a composition recipe has no family owner;
* `GraphMaterializationFailed` for an incomplete dispatch branch, stage-family
  drift, scalar-language failure, or final graph validation failure;
* `IterationBoundUnresolved` or `CompositionExpansionOverflow` while resolving
  a recipe (the global expanded-step limit is one million);
* `IdentityNamespaceOverlap` or `IdentityNamespaceExhausted` for caller
  reservations and generated values or kernels;
* `WorkspaceArithmeticOverflow` or `WorkspaceLimitExceeded` for intermediate
  storage accounting.

Every error produced below is tagged with the source-qualified `OperationId`
where the helper has a request. There is no alternate implementation or
fallback dispatch when one of these checks fails.

## Optimizer materializers

For each elementwise optimizer update, state and parameter tensors are
nonempty, equal-shaped f32 tensors. LAMB's two norm-square tensors are the
documented f32 `[1]` exceptions. The name sets and parameter sets in the
following table are exact.
All state transitions are separate elementwise graph nodes, so a parameter
node depends on the newly written state node. AdaGrad, RMSProp, Adam, AdamW,
NAdam, and LAMB use checked square-root denominators. Momentum and Lion use no
reduction workspace; LAMB and global clipping use explicit fixed-tree scalar
reductions.

| Symbol | Inputs | Outputs | Prepared parameters |
| --- | --- | --- | --- |
| `gpu_momentum_update` | `parameters`, `velocity`, `gradient` | `updated_velocity`, `updated_parameters` | `momentum`, `learning_rate` |
| `gpu_adagrad_update` | `gradient`, `weight`, `accumulator` | `updated_accumulator`, `updated_weight` | `learning_rate`, `epsilon` |
| `gpu_rmsprop_update` | `gradient`, `weight`, `cache` | `updated_cache`, `updated_weight` | `learning_rate`, `decay`, `epsilon` |
| `gpu_adam_update` | `gradient`, `weight`, `first_moment`, `second_moment` | `updated_first_moment`, `updated_second_moment`, `updated_weight` | `learning_rate`, `beta_one`, `beta_two`, `epsilon`, `step` |
| `gpu_adamw_update` | same Adam inputs | same Adam outputs | Adam parameters plus `weight_decay` |
| `gpu_nadam_update` | same Adam inputs | same Adam outputs | Adam parameters |
| `gpu_lion_update` | `gradient`, `weight`, `moment` | `updated_moment`, `updated_weight` | `learning_rate`, `beta_one`, `beta_two`, `weight_decay` |

### Momentum, AdaGrad, and RMSProp

`emit_momentum_update` validates all five tensors as equal f32 contracts. It
accepts finite `momentum` (the code does not restrict it to a unit interval)
and finite nonnegative `learning_rate`. The first scalar program emits

```text
updated_velocity = momentum * velocity - learning_rate * gradient
```

and the second emits `updated_parameters = parameters + updated_velocity`.
There are two elementwise kernels and zero intermediate workspace.

`emit_adagrad_update` uses the `weight` tensor as the shape reference and emits
the state program `updated_accumulator = accumulator + gradient * gradient`.
The adaptive program then computes
`updated_weight = weight - learning_rate * gradient /
(sqrt(updated_accumulator) + epsilon)`. `learning_rate` must be finite and
nonnegative; `epsilon` must be finite and positive. It emits two elementwise
kernels and reserves no intermediate tensor.

`emit_rmsprop_update` emits

```text
updated_cache = decay * cache + (1 - decay) * gradient * gradient
updated_weight = weight - learning_rate * gradient
                  / (sqrt(updated_cache) + epsilon)
```

The decay parameter is finite in the closed unit interval `[0, 1]`, learning
rate is finite and nonnegative, and epsilon is finite and positive. The state
and weight nodes are two elementwise stages with zero workspace.

### Adam, AdamW, and NAdam

`emit_adam_update` serves both `gpu_adam_update` and `gpu_adamw_update`; its
boolean argument is the only semantic fork. Adam's exact parameter set omits
`weight_decay`; AdamW's set includes it. In both cases, `beta_one` and
`beta_two` are finite in `[0, 1)`, epsilon is finite and positive, learning
rate is finite and nonnegative, and `step` is a positive u64 that fits i32.
`step + 1` must also fit i32. The host computes and validates

```text
first_correction      = 1 - beta_one^step
next_first_correction = 1 - beta_one^(step + 1)
second_correction     = 1 - beta_two^step
```

before embedding the constants into scalar SSA. Every correction must be
finite and positive. This preparation-time state is why these materializers
have no dynamic step counter or host loop.

The first elementwise stage emits the two moment recurrences:

```text
updated_first_moment  = beta_one * first_moment
                      + (1 - beta_one) * gradient
updated_second_moment = beta_two * second_moment
                      + (1 - beta_two) * gradient * gradient
```

The second stage bias-corrects both moments, requires the corrected second
moment to be nonnegative, takes its square root, adds epsilon, requires the
denominator to be positive, and computes the adaptive step. Adam emits
`weight - adaptive_step`. AdamW first forms the decoupled base
`weight * (1 - learning_rate * weight_decay)` and then subtracts the same
adaptive step. Adam has zero workspace.

`emit_nadam_update` shares the moment stage and all parameter checks. Its second
stage forms the Nesterov first estimate in this scalar order:

```text
beta_one * first_moment / (1 - beta_one^(step + 1))
+ (1 - beta_one) * gradient / (1 - beta_one^step)
```

It bias-corrects the second moment with `second_correction`, applies the same
positive square-root denominator and learning-rate multiply, and subtracts
the result from `weight`. It also reserves no workspace.

### Lion

`emit_lion_update` requires finite nonnegative learning rate and weight decay;
both beta values are finite in the closed interval `[0, 1]`. The first
elementwise node emits both `updated_moment` and a one-image direction
intermediate. Its interpolation is `beta_one * moment + (1 - beta_one) *
gradient`. The direction is exactly `1` for a positive interpolation, `-1`
for a negative interpolation, and `0` for equality. The stored moment uses
`beta_two * moment + (1 - beta_two) * gradient`. The second node emits

```text
updated_weight = weight
               - learning_rate * (direction + weight_decay * weight)
```

The direction image is the only workspace object, exactly four bytes per
parameter element.

### LAMB phases

`gpu_lamb_phase1` has inputs `gradient`, `weight`, `moment`, and `velocity`; its
outputs are `updated_moment`, `updated_velocity`, `update`,
`weight_norm_squared`, and `update_norm_squared`. The four state tensors and
the `update` tensor are equal-shaped nonempty f32 contracts. The two norm-square
outputs are f32 `[1]` tensors. Parameters are `beta_one`, `beta_two`,
`epsilon`, `weight_decay`, positive int32-range `step`, and valid `tree_lanes`.

The first elementwise stage performs the Adam moment and velocity recurrences,
applies the positive checked bias-corrected denominator, adds decoupled
`weight_decay * weight` to form `update`, and writes `weight * weight` and
`update * update` images. A second stage contains two fixed-tree all-axis sum
reductions and an identity elementwise publication of `update`. It reserves
three f32 images, or `12 * element_count` bytes.

`gpu_lamb_phase2` consumes equal-shaped f32 `update` and `weight`, f32 `[1]`
`weight_norm_squared` and `update_norm_squared`, and produces equal-shaped
`updated_weight`. Parameters are finite nonnegative `learning_rate` and valid
`tree_lanes`. The graph explicitly copies each scalar, reduces each singleton
image through a fixed axis-zero tree, then computes nonnegative square roots.
The scalar program forms `weight_norm / update_norm`; if either norm is zero it
selects trust ratio one, otherwise it uses the quotient, and emits
`weight - learning_rate * ratio * update`. The four scalar intermediates use
16 bytes of workspace.

### Global gradient clipping

`gpu_grad_clip_norm` consumes f32 `values`, produces equal-shaped f32
`clipped`, and requires finite nonnegative `maximum_norm` plus valid
`tree_lanes`. It emits a square image, an all-axis fixed-tree sum into f32
`[1]`, then the clipping map. The scalar map requires a nonnegative norm
square, takes its square root, forms `maximum_norm / norm`, and selects the
original value when `norm <= maximum_norm`, otherwise the scaled value. The
scale expression is built before the select; the program has no separate
zero-norm denominator guard. Workspace is one f32 image plus one f32 scalar,
`4 * element_count + 4` bytes.

## Batch normalization materializers

The three batch-normalization forms use rank-two nonempty f32
`values[rows, columns]`. `normalization_matrix` checks both extents against
the int32 and exact-f32 limits and supplies row and column divisors. Channel
statistics always reduce axis zero with a fixed tree.

### Training forward: `gpu_batchnorm_forward`

The exact inputs are `values`, column-shaped `scale`, and column-shaped
`bias`. Outputs are equal-shaped `normalized`, f32 `[columns]`
`saved_mean`, and f32 `[columns]`
`saved_inverse_standard_deviation`. Parameters are positive finite `epsilon`
and valid `tree_lanes`.

The four resolved stages are:

1. Copy `values` into an explicit f32 statistic image.
2. Reduce axis zero into `sums`.
3. In one stage, form centered squares using `sums / rows`, then reduce them
   over axis zero into `variance_sums`.
4. In one stage, publish the mean and inverse standard deviation, then apply
   the affine normalized output.

The statistic program computes `mean = sums / rows` and
`inverse = 1 / sqrt(variance_sums / rows + epsilon)`, with a device-side
positive requirement on the adjusted variance. The affine program computes
`(values - saved_mean) * scale * saved_inverse_standard_deviation + bias` in
that order. Workspace is `8 * rows * columns + 8 * columns` bytes.

### Backward: `gpu_batchnorm_backward`

Inputs are `output_gradient`, `values`, `saved_mean`,
`saved_inverse_standard_deviation`, and column-shaped `scale`. Outputs are
equal-shaped `input_gradient`, column-shaped `scale_gradient`, and
`bias_gradient`. The only parameter is valid `tree_lanes`; the saved statistics
make epsilon unnecessary at this boundary.

The first elementwise stage emits output-gradient and normalized-gradient
terms. Two fixed axis-zero reductions publish their column sums. The next
elementwise stage computes input-gradient terms plus per-element scale and
bias-gradient terms. The final stage reduces those two images over rows into
the external affine gradients. The scalar formula subtracts the mean gradient
and the normalized mean-gradient projection from each output gradient, then
multiplies by `scale * inverse`; it emits scale gradient as
`output_gradient * normalized` and bias gradient as the original output
gradient. Workspace is `16 * rows * columns + 8 * columns` bytes.

### Inference: `gpu_batchnorm_inference`

The exact inputs are `values`, `running_mean`, `running_variance`, `scale`, and
`bias`; output is `normalized`. Every tensor is f32, and the output shape must
equal the language broadcast result of all five inputs. Positive finite
`epsilon` is the only parameter. One elementwise stage computes

```text
((values - running_mean) * scale) / sqrt(running_variance + epsilon) + bias
```

in the actual scalar order `scale * centered`, divide by the square root, then
add bias. No intermediate workspace is reserved beyond the external output.

### Running-statistics update: `gpu_bn_update_running`

Inputs are `saved_mean`, `saved_variance`, `running_mean`, and
`running_variance`; outputs are `updated_running_mean` and
`updated_running_variance`. All six tensors are equal-shaped f32, and the
running tensors must be rank one. The sole parameter `momentum` is finite in
the closed interval `[0, 1]`.

The one elementwise node evaluates the same explicit order for mean and
variance:

```text
retained = 1 - momentum
updated = retained * running + momentum * saved
```

The `saved` values therefore receive the `momentum` coefficient in this owned
implementation. There is no host-side state mutation and no workspace image;
the updated tensors are graph outputs for the caller to publish as the next
running state.

## Layer normalization aliases

The source-qualified forward rows `gpu_layernorm_f32`, `gpu_layernorm_into`,
and `gpu_layernorm_opt_into` share one emitter only after their ABI has been
resolved. The two backward rows share a separate emitter. All use nonempty
rank-two f32 `values[rows, columns]`, reduce axis one with `keep_dimensions =
true`, and use exact f32 `columns` as the divisor.

### Forward aliases

The two required-affine rows take `values`, column-shaped `scale`, and
column-shaped `bias`, produce equal-shaped `normalized`, and require positive
`epsilon` and valid `tree_lanes`. `gpu_layernorm_opt_into` additionally
requires typed boolean `has_scale` and `has_bias`. Its exact input set is
constructed from those booleans: a disabled affine input is absent, not a
dummy tensor. Its exact parameter set includes both booleans.

The optional emitter preserves one subtle source behavior: when `has_scale` is
false, `layer_normalization_program` does not read or apply `bias`, even if
`has_bias` is true. The ABI may still contain the bias declaration in that
case, because the flag requested it, but the scalar program returns the
unaffined normalized value. When scale is present, bias is applied only when
`has_bias` is true.

The four stages copy values, reduce each row sum, form centered squares and
reduce their row sums, then apply the normalized value and optional affine
terms. The scalar inverse is
`1 / sqrt(variance_sum / columns + epsilon)` with a device-side positive
requirement. Workspace is `8 * rows * columns + 8 * rows` bytes.

### Backward aliases

Both `gpu_layernorm_backward_f32` and `gpu_layernorm_backward_full_into` take
exact inputs `output_gradient`, `values`, and column-shaped `scale`. Outputs
are equal-shaped `input_gradient`, column-shaped `scale_gradient`, and
`bias_gradient`; parameters are positive `epsilon` and valid `tree_lanes`.

The emitter first reconstructs row sums and centered variance, then emits four
gradient-term images and two row reductions in one stage. The next stage
reduces scale and bias terms over axis zero. Its scalar order is:

```text
normalized = (values - mean) * inverse
transformed_gradient = output_gradient * scale
first_mean = first_sum / columns
second_mean = normalized * second_sum / columns
input_gradient = inverse * (transformed_gradient - first_mean - second_mean)
scale_gradient_term = output_gradient * normalized
bias_gradient_term = output_gradient
```

The external affine gradients are fixed-tree row reductions. Workspace is
`24 * rows * columns + 16 * rows` bytes.

## RMS normalization aliases

`gpu_rmsnorm`, `gpu_rmsnorm_f64`, and `gpu_rmsnorm_f64_nogamma` all dispatch to
`emit_rms_normalization`. The first two require exact f32 inputs `values` and
column-shaped `scale`; the `_nogamma` row requires only `values`. Every row
produces equal-shaped f32 `normalized` and requires positive `epsilon` and
valid `tree_lanes`. The f64 source rows are canonicalized to this f32 ABI, as
described in the registry section above.

The four stages square values, reduce squared values over axis one into
`square_sums`, reduce that kept-dimension image once more into
`fixed_square_sums`, and apply `value * inverse` plus optional scale. The
inverse is `1 / sqrt(square_sum / columns + epsilon)`, with a positive
device-side requirement. The second reduction is intentionally present in the
owned graph even though it preserves the singleton dimension. Workspace is
`4 * rows * columns + 8 * rows` bytes.

`gpu_rmsnorm_backward` takes exact inputs `output_gradient`, `values`, and
column-shaped `scale`; outputs are equal-shaped `input_gradient` and
`scale_gradient`; parameters are positive `epsilon` and valid `tree_lanes`.
It squares and reduces values, computes a dot-term image
`output_gradient * values * inverse`, reduces that dot over rows, then emits
input and scale-gradient terms and reduces the latter over rows. The scalar
input gradient is `scale * inverse * (output_gradient - normalized *
(dot_sum / columns))`, and scale-gradient terms are
`output_gradient * normalized`. Workspace is `12 * rows * columns + 8 * rows`
bytes.

## State, determinism, and lifecycle role

These operations are model calculations and transfers only through their
resulting `PrimitiveKind` graph. The optimizer state tensors, saved
normalization statistics, running statistics, and gradient outputs are ordinary
graph values. Preparation computes dimensions, constants, and verification
facts before the immutable `init -> loop -> exit` plan; the materializer itself
does not add a lifecycle phase, device allocation, queue, synchronization, or
metric event.

Every descriptor has `DeterminismContract::FixedPrimitiveOrder`. Reductions use
the explicit `tree_lanes` tree and explicit axes. Elementwise formulas preserve
the scalar builder order shown above. There are no scatter or random kernels in
this family. All calculations remain f32, with int32 only appearing in the
shared graph/index contracts outside these tensor ABIs.

The caller owns authoritative state publication. For an optimizer, the graph
outputs `updated_*` values become the next state only when the surrounding
compiled graph assigns them to its state transition. For batch normalization,
`saved_*` outputs are consumed by the backward graph and
`updated_running_*` outputs are the next running-statistics values. RMSNorm and
LayerNorm outputs are ordinary activation values. No emitter mutates a caller
tensor in place, and no missing output is silently substituted.

## Source-qualified failure cases

The most important paired cases are source and ABI mismatches:

* `gpu_adam_update` is valid only with
  `gpu-core/src/kernels.rs:6421`; pairing that symbol with the AdamW source
  `gpu-core/src/kernels.rs:6454` is absent from the canonical registry rather
  than being silently reclassified.
* `gpu_adamw_update` is valid only with
  `gpu-core/src/kernels.rs:6454`; pairing it with the Adam source
  `gpu-core/src/kernels.rs:6421` is not accepted as AdamW. The valid AdamW
  descriptor passes `true` and requires `weight_decay`.
* `gpu_layernorm_f32` and `gpu_layernorm_into` share the same forward graph,
  but remain separate descriptor identities and exact source rows. A source
  mismatch fails before the emitter rather than falling through.
* `gpu_layernorm_opt_into` cannot be represented by adding optional tensors to
  either required-affine ABI. Its booleans determine the exact input set.
* `gpu_rmsnorm_f64` and `gpu_rmsnorm_f64_nogamma` retain f64 provenance but
  cannot receive f64 tensors. They dispatch to the canonical f32 graph, with
  scale present only for the first row.
* `gpu_layernorm_backward_f32` and `gpu_layernorm_backward_full_into` share
  backward semantics, but neither accepts the optional-affine boolean ABI.
* A recognized symbol with a nonmatching source is `NotOwned` by this module.
  Because no other family claims that exact pair, the outer boundary reports
  `MissingConcreteFormula` before graph emission.
* A recognized exact pair whose match arm is accidentally removed is owned by
  the table but reaches the explicit dispatch fallback and reports
  `GraphMaterializationFailed`; it never selects a neighboring arm.

These cases are why the table, composition recipe, dispatch match, emitter ABI,
and caller source identity must be changed together. The module's purpose is a
small, inspectable preparation boundary, not a compatibility layer around
several competing implementations.
