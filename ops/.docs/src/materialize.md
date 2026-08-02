# `ops/src/materialize.rs`: structured composition materialization

```yaml
document: recipe_ops.materialize
kind: preparation-boundary
authority:
  - ops/src/materialize.rs
  - ops/src/materialize/*.rs
  - ops/src/composition.rs
  - ops/src/registry.rs
  - ops/src/error.rs
  - ops/MATERIALIZATION.md
  - training/src/compile.rs
  - training/src/inference.rs
  - src/facade.rs
```

This module is the concrete preparation boundary for a structured operation.
`CompositionRecipe` describes a finite primitive-family algorithm, but its role
strings are not tensor semantics. `materialize_composition` supplies the exact
source-qualified tensor ABI, scalar SSA programs, primitive parameters, static
tables, identity ownership, and workspace policy, then returns one validated
`recipe_language::CalculationGraph` fragment. The fragment contains only
Recipe primitive kinds and has no callback, dynamic blueprint, vendor handle,
host payload loop, or execution side effect.

The module does not parse the operation surface, resolve a symbol, read a
dataset, probe hardware, schedule tasks, allocate device memory, lower a
primitive for CUDA or HSA, load a native image, or execute the graph. Those
boundaries remain with the registry, training and inference compilers,
preparation, planner, native executors, and runtime. Shapes, typed preparation
facts, identity reservations, and the workspace limit must already be fixed
when this module is called.

## Position in the production path

The source-qualified path is:

```text
operation-surface.txt
  -> ops/build.rs (generated RAW_OPERATION_SURFACE)
  -> OperationRegistry::resolve_unique / resolve_exact
  -> OperationDescriptor { symbol, source, lowering: Composition(recipe) }
  -> training::TrainingGraphCompiler::materialize
     or training::InferenceGraphCompiler::materialize
  -> MaterializationRequest
  -> validate_request
  -> expand_composition
  -> GraphBuilder + Emitter
  -> dispatch_concrete (exact family table and source pair)
  -> MaterializedComposition.graph (validated CalculationGraph)
  -> compiler inserts tensor contracts and kernel nodes
  -> StaticCalculationProgram -> recipe-primitives/planner -> runtime
```

The public `recipe::operations::materialize` facade in `src/facade.rs` is a
thin forwarding API. It calls `recipe_ops::materialize_composition` and does
not alter the request or provide a fallback. The companion
`recipe::operations::remaining_compositions` forwards
`remaining_composition_manifest` so unresolved descriptive rows remain
visible.

`ops/src/composition.rs` is the paired descriptive source. It defines
`CompositionRecipe`, `CompositionStep`, `IterationBound`, and the static step
lists selected by `(symbol, source)`. Its `validate` method checks nonempty
metadata, nonempty repeat bodies, nonzero fixed bounds, nonempty parameter
names, and nesting no deeper than eight levels. `materialize.rs` calls that
validation again before expansion, so a registry row cannot bypass the recipe
contract.

## Public request and result types

All public records below are immutable at the boundary. Their fields are private
and are exposed only through constructors or read-only accessors. This keeps
the caller from changing an ABI or identity range after validation.

| Type | Actual role and invariant |
| --- | --- |
| `NamedTensor<'a>` | One name-addressed borrowed `Tensor`. The name is the operation ABI key; the tensor supplies `ValueId`, dtype, shape, layout, storage bytes, and external flags. |
| `PreparedParameter` | Exactly one typed preparation fact: `U64`, `I32`, `F32Bits`, or `Bool`. Floating values use bits so the request is deterministic and does not silently coerce variants. |
| `PreparedParameters` | `BTreeMap<String, PreparedParameter>`. Concrete materializers require an exact key set, not merely a subset. |
| `IdentityNamespace` | Caller-reserved half-open ranges `first_value .. first_value + value_capacity` and `first_kernel .. first_kernel + kernel_capacity`. Input and output tensor IDs are outside the value range. |
| `MaterializationRequest<'a>` | Descriptor, named input and output slices, the input name whose shape resolves iteration bounds, typed parameters, identity namespace, and `ByteCount` workspace limit. |
| `ResolvedIteration` | One surrounding repeat invocation, recording its role, zero-based index, and resolved count. |
| `ResolvedBound` | One repeat bound, its `IterationBound` expression, preparation-time value, and the outer iteration stack at the repeat site. |
| `ResolvedStep` | One primitive after unrolling, with linear ordinal, `PrimitiveFamily`, role, surrounding iteration stack, and dependency ordinals. |
| `ResolvedComposition` | Operation identity plus all resolved bounds and steps in deterministic emission order. |
| `WorkspaceObject` | One intermediate `ValueId`, dtype, shape, and checked storage byte count. |
| `WorkspaceAllocation` | The ordered intermediate list and total bytes charged to the operation. |
| `StageEmission` | One resolved step ordinal and the kernel IDs emitted for that stage. A stage can contain more than one kernel. |
| `MaterializedComposition` | Operation ID, validated graph, resolved recipe, stage map, workspace allocation, and the unchanged identity namespace. |
| `MissingConcreteComponent` | Manifest vocabulary: `TensorAbi`, `ScalarFormula`, `PrimitiveParameters`, or `WorkspacePolicy`. |
| `RemainingComposition` | Source-qualified unresolved descriptor, recipe name, and all four missing components. |

`OperationError` from `ops/src/error.rs` carries an `OperationErrorKind`, detail,
and optional `OperationId`. Every error constructed by this module carries the
descriptor ID, except a namespace range arithmetic error detected before an
operation is attached. `Display` includes the operation ordinal when present.

## `materialize_composition` call sequence

`materialize_composition` is the only central entry point for the family
materializers. Its order is significant:

1. `validate_request` checks the boundary descriptor, tensors, names, IDs, and
   external flags.
2. `has_concrete_materializer` checks exact `(symbol, source)` ownership in the
   family tables. A structured row with no concrete table entry returns
   `MissingConcreteFormula` before any graph or workspace allocation.
3. The named `iteration_shape_input` is looked up among inputs. Its shape is the
   sole shape source for shape-dependent `IterationBound` values. A missing name
   returns `InvalidMaterializationRequest`.
4. `expand_composition` validates and recursively unrolls the selected recipe.
5. `Emitter::new` creates a `GraphBuilder` with the caller's namespace and
   resolved step count.
6. `dispatch_concrete` probes each family in its fixed order. The first family
   whose exact table owns the descriptor emits the graph and returns its result.
7. `Emitter::finish` requires the concrete emitter to have consumed every
   resolved step, finishes and validates the graph, and returns the immutable
   result.

No phase retries a failed family, selects a neighboring symbol, or invokes a
legacy implementation. A dispatch error is returned as observed.

## Request validation

`validate_request` enforces the common boundary before family code runs:

* `descriptor.lowering` must be `LoweringAvailability::Composition(_)`.
  Scalar, primitive, workspace, non-calculation, and unsupported descriptors
  return `WrongLoweringKind`.
* There must be at least one input and one output declaration.
* Every declaration name is nonempty and unique across both slices.
* Every `ValueId` is declared exactly once across inputs and outputs.
* Every boundary tensor passes `Tensor::validate`, including layout rank,
  non-overlap, span arithmetic, and storage byte coverage.
* Every input has `external_input = true`.
* Every output has `external_input = false` and `external_output = true`.

Family code then calls `require_exact_abi`. It compares `BTreeSet`s of input
names, output names, and parameter keys. Missing names, extra names, duplicate
names, or a wrong typed parameter key set all return
`InvalidMaterializationRequest`. `input` and `output` resolve a required name
only in their corresponding slice and fail instead of guessing.

Common semantic checks are shared in `materialize.rs`:

* `require_dtype` rejects a payload that is not the canonical requested `F32`
  or `I32` type.
* `require_shape` compares exact extents and reports
  `UnsupportedConcreteShape`.
* `require_same_tensor_contract` requires equal dtype and shape for a related
  tensor, such as an output image or optimizer state.
* `prepared_u64`, `prepared_f32`, and `prepared_bool` reject missing values and
  type variants. `prepared_axis` checks conversion to `usize`.
* `prepared_tree_lanes` requires a power of two in `1..=1024` and checks the
  `u32` conversion.
* `finite_nonnegative_parameter`, `finite_positive_parameter`, and
  `unit_interval_parameter` reject NaN, infinity, negative values, and values
  outside the operation's documented interval.
* `exact_f32_extent` rejects integer extents above `2^24`, where conversion to
  an f32 divisor would not be guaranteed lossless.
* `normalization_matrix` requires a nonempty rank-two f32 matrix and checks
  both extents against the signed int32 launch domain.

Verification facts are represented as `PreparedParameter::Bool(true)` and are
checked with `require_true`; a false fact or a missing fact is an error. The
materializers do not inspect host table contents or silently regenerate a table.

## Repeat expansion

`expand_composition` accepts only a descriptor whose lowering is a composition.
It calls `CompositionRecipe::validate`, then creates an `Expansion` holding the
operation ID, the selected input shape, and the typed parameter map.

`Expansion::resolve_bound` handles every declared bound:

| Bound | Resolution |
| --- | --- |
| `Fixed(value)` | The nonzero literal as `u64`. |
| `ShapeExtent { axis }` | The selected input shape extent at `axis`. |
| `MinimumShapeExtent` | The minimum extent of the selected nonempty shape. |
| `CeilingLog2ShapeExtent { axis }` | The selected extent passed through the integer `ceiling_log_two` helper. Zero and one resolve to zero. |
| `PreparedParameter { name }` | A `PreparedParameter::U64` with the exact key. |

Each repeat appends one `ResolvedBound`, preserving the outer iteration stack,
then visits its body once per resolved index. A primitive appends one
`ResolvedStep` with a copied iteration stack and a dependency containing the
immediately preceding step ordinal when one exists. The dependency chain is
linear and deterministic even when a family emits multiple kernels for one
step. The expansion is fully finite before any `PrimitiveKernel` is emitted.

`MAX_EXPANDED_STEPS` is one million. If the next primitive would exceed that
fixed preparation limit, expansion returns `CompositionExpansionOverflow`.
Missing shape axes return `IterationBoundUnresolved`; an empty shape cannot
resolve `MinimumShapeExtent`; missing or mistyped prepared bounds return
`MissingPreparedParameter` or `PreparedParameterTypeMismatch`.

## Identity, workspace, and graph ownership

`identity_ranges` computes both half-open ranges with checked `u64` addition.
Overflow returns `IdentityNamespaceExhausted`. The public
`validate_identity_namespaces` helper computes every supplied namespace range
and rejects any pair whose value ranges or kernel ranges overlap with
`IdentityNamespaceOverlap`. Empty ranges do not overlap. This is the assembly
guard for independently materialized fragments.

`GraphBuilder::new` clones the boundary tensor declarations, checks the ranges,
and rejects any declared tensor ID inside the reserved intermediate value
range. It starts value and kernel cursors at the caller's first IDs. It does not
choose an implicit ID after the largest boundary tensor and does not use a
global or hard-coded kernel ID.

`GraphBuilder::intermediate` performs all operation-owned allocation:

1. It consumes the next reserved `ValueId`, failing when the value capacity is
   exhausted.
2. It constructs a contiguous `Tensor` with the requested dtype and shape.
3. It adds the tensor's checked storage bytes to the running workspace total.
4. It rejects `u64` byte-total overflow with `WorkspaceArithmeticOverflow`.
5. It rejects a total above `workspace_limit` with `WorkspaceLimitExceeded`.
6. It records a `WorkspaceObject` and appends the tensor to the graph.

`GraphBuilder::emit` consumes one reserved `KernelTemplateId`, builds a
`PrimitiveKernel`, and creates a forbidden alias rule for every input/output
pair. The graph therefore cannot accidentally reuse a boundary buffer for a
new output. Family-specific operations that need explicit atomic or unique
scatter behavior encode that policy in the `PrimitiveKind`; the default alias
matrix remains forbidden.

`GraphBuilder::finish` builds `CalculationGraph { tensors, nodes }` and calls
`CalculationGraph::validate`. The language validator checks that all referenced
tensors exist, tensor contracts match each primitive kind, axes are in rank,
index bounds and scatter conflicts are explicit, scalar programs are valid, and
alias matrices are complete. A language failure is wrapped as
`GraphMaterializationFailed` with the operation ID. The returned workspace byte
count is the sum of every emitted intermediate image and scalar, not a formula
that can drift from the graph.

## Stage accounting

`Emitter` owns the resolved composition, a cursor, the graph builder, and the
`StageEmission` list. `emit` is shorthand for a one-kernel `emit_stage`.
`emit_stage` enforces all of the following before inserting any kernel:

* a resolved step exists at the cursor;
* the supplied kernel sequence is nonempty;
* the final kernel's `PrimitiveKind` maps through `primitive_family` to the
  resolved step's `PrimitiveFamily`;
* every supplied kernel can consume an identity and satisfy graph validation.

The last-kernel check lets one resolved step contain a fixed local sequence,
such as gather, map, and scatter, while still proving that its terminal
primitive family is the family declared by the recipe. A concrete emitter that
emits too many kernels, emits an empty stage, or emits a family mismatch returns
`GraphMaterializationFailed`. `Emitter::finish` returns the same error if the
cursor is short of the resolved step count. Thus a table branch cannot silently
under-emit a descriptive recipe.

`primitive_family` is exhaustive over the language primitive kinds:
`Elementwise`, `Reduce`, `Scan`, `Contraction`, `Gather`, `Scatter`,
`Histogram`, `Sort`, `IndexMap`, and `Random`. These are the only model work
forms produced by this boundary.

## Exact dispatch ownership

`dispatch_concrete` probes modules in this exact order:

```text
optimizer_normalization
solver_fft
attention_sequence_embedding
convolution_pooling
loss_metrics
indexing_sort_encoding
graph_cluster_rl
tree_boosting
inference_quantization_diffusion
creation_shape_misc
training
```

Every nonempty `OPERATIONS` table is a set of exact `(symbol, source)` pairs.
`supports` never matches a symbol by itself. A pair is owned only when both
fields match the operation descriptor. The current tables contain 136 concrete
source-qualified entries:

### Optimizer and normalization, `optimizer_normalization.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_adagrad_update` | `gpu-core/src/optimizers.rs:149` |
| `gpu_adam_update` | `gpu-core/src/kernels.rs:6421` |
| `gpu_adamw_update` | `gpu-core/src/kernels.rs:6454` |
| `gpu_batchnorm_backward` | `gpu-core/src/kernels.rs:6388` |
| `gpu_batchnorm_forward` | `gpu-core/src/kernels.rs:6322` |
| `gpu_batchnorm_inference` | `gpu-core/src/kernels.rs:6355` |
| `gpu_bn_update_running` | `gpu-core/src/attention.rs:454` |
| `gpu_grad_clip_norm` | `gpu-core/src/kernels.rs:6670` |
| `gpu_lamb_phase1` | `gpu-core/src/optimizers.rs:174` |
| `gpu_lamb_phase2` | `gpu-core/src/optimizers.rs:217` |
| `gpu_layernorm_backward_f32` | `gpu-core/src/nn_f32.rs:339` |
| `gpu_layernorm_backward_full_into` | `gpu-core/src/kernels.rs:3491` |
| `gpu_layernorm_f32` | `gpu-core/src/nn_f32.rs:313` |
| `gpu_layernorm_into` | `gpu-core/src/kernels.rs:3205` |
| `gpu_layernorm_opt_into` | `gpu-core/src/kernels.rs:3233` |
| `gpu_lion_update` | `gpu-core/src/optimizers.rs:243` |
| `gpu_momentum_update` | `gpu-core/src/optimizers.rs:95` |
| `gpu_nadam_update` | `gpu-core/src/optimizers.rs:273` |
| `gpu_rmsnorm` | `gpu-core/src/attention.rs:287` |
| `gpu_rmsnorm_backward` | `gpu-core/src/attention.rs:313` |
| `gpu_rmsnorm_f64` | `gpu-core/src/infer_ops.rs:335` |
| `gpu_rmsnorm_f64_nogamma` | `gpu-core/src/infer_ops.rs:360` |
| `gpu_rmsprop_update` | `gpu-core/src/optimizers.rs:121` |

The dispatcher calls the shared `materialize.rs` emitters
`emit_batch_normalization_*`, `emit_layer_normalization*`,
`emit_rms_normalization*`, `emit_momentum_update`, `emit_adagrad_update`,
`emit_rmsprop_update`, `emit_adam_update`, `emit_nadam_update`,
`emit_lion_update`, `emit_lamb_phase_one`, `emit_lamb_phase_two`, and
`emit_gradient_clip_norm`. These emit exact f32 tensor contracts and owned
scalar programs. Optimizer state transitions are separate nodes when the
source operation requires a dependency. Adam and NAdam bias corrections are
computed from a positive int32-range preparation `step`; beta, epsilon,
learning-rate, decay, and weight-decay domains are checked before SSA creation.
Normalization reductions use prepared fixed-tree lane counts and reject empty,
non-rank-two, non-f32, or non-exact-f32-divisor matrices.

### Solver and FFT, `solver_fft.rs`

| Symbol | Paired source | Emitter |
| --- | --- | --- |
| `gpu_fft_c2c_1d` | `gpu-core/src/linalg.rs:867` | `emit_radix_two_fft` |
| `gpu_tri_solve` | `gpu-core/src/kernels.rs:2263` | `emit_triangular_solve` |

The FFT emitter resolves one repeat per radix-two stage and uses checked gather,
fixed FMA scalar programs, and unique scatters. The triangular solver resolves
one row per shape element, reduces the strict prefix with prepared `tree_lanes`,
requires a nonzero diagonal in SSA, and uniquely scatters each next solution
image. Both reject unverified tables, non-power-of-two or empty shapes, and
identity or workspace exhaustion.

### Attention, sequence, and embedding, `attention_sequence_embedding.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_causal_softmax_rows` | `gpu-core/src/attention.rs:191` |
| `gpu_embed_blend` | `gpu-core/src/infer_ops.rs:266` |
| `gpu_embedding_backward` | `gpu-core/src/attention.rs:427` |
| `gpu_mha_merge` | `gpu-core/src/attention.rs:227` |
| `gpu_mha_split` | `gpu-core/src/attention.rs:202` |
| `gpu_positional_encoding` | `gpu-core/src/attention.rs:276` |
| `gpu_repeat_rows` | `gpu-core/src/kernels.rs:6598` |
| `gpu_rope` | `gpu-core/src/attention.rs:252` |
| `gpu_rope_partial` | `gpu-core/src/infer_ops.rs:154` |
| `gpu_rope_partial_factors` | `gpu-core/src/infer_ops.rs:209` |
| `gpu_rope_partial_factors_pos` | `gpu-core/src/infer_ops.rs:237` |
| `gpu_rope_partial_pos` | `gpu-core/src/infer_ops.rs:181` |

The dispatcher calls `emit_causal_softmax`, `emit_embedding_blend`,
`emit_embedding_backward`, `emit_mha_merge`, `emit_checked_gather_identity`
for MHA split, `emit_positional_encoding`, `emit_repeat_rows`, and
`emit_single_tensor_rope` for the five rotary variants. Dimensions and products
are checked, int32 tables use `IndexBounds::Reject`, and verification facts must
be true. Causal softmax emits fixed max and sum reductions with a positive-row
sum requirement. Embedding backward uses explicit relaxed atomic add for
duplicate table rows. Rotary variants preserve source-specific parameter sets,
including even rotary dimensions and finite positive bases.

### Convolution and pooling, `convolution_pooling.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_avg_pool_1d` | `gpu-core/src/kernels.rs:4596` |
| `gpu_avg_pool_2d` | `gpu-core/src/kernels.rs:5962` |
| `gpu_avg_pool_2d_backward` | `gpu-core/src/kernels.rs:5999` |
| `gpu_avg_pool_2d_backward_f32` | `gpu-core/src/nn_f32.rs:427` |
| `gpu_avg_pool_2d_f32` | `gpu-core/src/nn_f32.rs:391` |
| `gpu_col2im_1d` | `gpu-core/src/kernels.rs:5847` |
| `gpu_col2im_2d` | `gpu-core/src/kernels.rs:5877` |
| `gpu_col2im_2d_ext` | `gpu-core/src/attention.rs:384` |
| `gpu_im2col_1d` | `gpu-core/src/kernels.rs:5097` |
| `gpu_im2col_2d` | `gpu-core/src/kernels.rs:5417` |
| `gpu_im2col_2d_ext` | `gpu-core/src/attention.rs:341` |
| `gpu_max_pool_1d` | `gpu-core/src/kernels.rs:5911` |
| `gpu_max_pool_1d_backward` | `gpu-core/src/kernels.rs:5936` |
| `gpu_max_pool_2d` | `gpu-core/src/kernels.rs:6037` |
| `gpu_max_pool_2d_backward` | `gpu-core/src/kernels.rs:6076` |
| `gpu_max_pool_2d_backward_f32` | `gpu-core/src/nn_f32.rs:501` |
| `gpu_max_pool_2d_f32` | `gpu-core/src/nn_f32.rs:463` |
| `gpu_pool_grad_expand` | `gpu-core/src/kernels.rs:4622` |
| `gpu_upsample_nearest_2d` | `gpu-core/src/kernels.rs:6620` |
| `recipe_max_pool_1d` | `ops/src/pooling.rs:channelwise_max_pool_1d` |
| `recipe_max_pool_1d_backward` | `ops/src/pooling.rs:channelwise_max_pool_1d_backward` |

The dispatcher calls `emit_im2col_2d`, `emit_col2im_1d`, `emit_col2im_2d`,
`emit_average_pool_1d`, `emit_average_pool_2d`, `emit_average_pool`,
`emit_legacy_max_pool_1d`, `emit_channelwise_max_pool_1d`, `emit_max_pool_2d`,
`emit_average_pool_2d_backward`, `emit_pool_gradient_expand`,
`emit_legacy_max_pool_1d_backward`, `emit_channelwise_max_pool_1d_backward`,
`emit_max_pool_2d_backward`, and `emit_upsample_nearest_2d`. Geometry helpers
check nonzero extents, checked products, int32 indexability, and effective
kernels. Forward tables use checked gathers; backward overlap uses explicit
relaxed atomic scatter, while verified unique channelwise destinations use
`ScatterConflict::UniqueIndices`. Extended im2col rejects nonzero padding
because the prepared gather plus map cannot synthesize padding zeros; extended
col2im accepts prepared valid-contribution tables and preserves the source
conditional update.

### Loss and metrics, `loss_metrics.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_accuracy` | `gpu-core/src/kernels.rs:4472` |
| `gpu_accuracy_into` | `gpu-core/src/kernels.rs:2914` |
| `gpu_argmax_accuracy_into` | `gpu-core/src/kernels.rs:2963` |
| `gpu_contrastive_loss` | `gpu-core/src/losses.rs:328` |
| `gpu_cosine_embedding_loss` | `gpu-core/src/losses.rs:270` |
| `gpu_has_nan` | `gpu-core/src/math_ops.rs:456` |
| `gpu_hinge_loss` | `gpu-core/src/losses.rs:246` |
| `gpu_isfinite_all` | `gpu-core/src/math_ops.rs:476` |
| `gpu_kl_div_loss` | `gpu-core/src/losses.rs:225` |
| `gpu_mean_all` | `gpu-core/src/reductions.rs:275` |
| `gpu_mse_into` | `gpu-core/src/kernels.rs:2888` |
| `gpu_reduce_mean_cols` | `gpu-core/src/reductions.rs:5176` |
| `gpu_reduce_var_cols` | `gpu-core/src/reductions.rs:5209` |
| `gpu_ss_res_into` | `gpu-core/src/kernels.rs:2862` |
| `gpu_triplet_loss` | `gpu-core/src/losses.rs:299` |

The dispatcher calls `emit_binary_accuracy`, `emit_multiclass_correct_count`,
`emit_dense_multiclass_accuracy`, `emit_finite_metric`, `emit_hinge_loss`,
`emit_kl_divergence_loss`, `emit_global_mean`, `emit_mean_squared_error`,
`emit_sum_squared_residuals`, `emit_column_mean`, `emit_column_variance`,
`emit_contrastive_loss`, `emit_cosine_embedding_loss`, and `emit_triplet_loss`.
These materializers use fixed-tree reductions with the prepared lane count,
checked f32 count conversion, lowest-index argmax ties, and explicit finite or
nonnegative scalar requirements. They return canonical f32 or i32 payloads and
never add an atomic reduction.

### Indexing, sorting, and encoding, `indexing_sort_encoding.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_add_col` | `gpu-core/src/kernels.rs:6839` |
| `gpu_add_col_scaled_inplace` | `gpu-core/src/kernels.rs:4525` |
| `gpu_add_diag` | `gpu-core/src/kernels.rs:2298` |
| `gpu_argsort` | `gpu-core/src/reductions.rs:458` |
| `gpu_bin_edges_uniform` | `gpu-core/src/encoding.rs:96` |
| `gpu_concat_into` | `gpu-core/src/kernels.rs:5361` |
| `gpu_one_hot` | `gpu-core/src/encoding.rs:169` |
| `gpu_pack_upper_tri` | `gpu-core/src/kernels.rs:5563` |
| `gpu_partial_argsort` | `gpu-core/src/kernels.rs:5267` |
| `gpu_segment_sum` | `gpu-core/src/reductions.rs:627` |
| `gpu_slice_cols` | `gpu-core/src/kernels.rs:6550` |
| `gpu_slice_lead_into` | `gpu-core/src/kernels.rs:5390` |
| `gpu_slice_rows` | `gpu-core/src/kernels.rs:5619` |
| `gpu_topk_per_row` | `gpu-core/src/kernels.rs:4712` |
| `gpu_transpose` | `gpu-core/src/kernels.rs:5540` |
| `gpu_tril_mask` | `gpu-core/src/kernels.rs:6578` |
| `gpu_vconcat` | `gpu-core/src/kernels.rs:6514` |

The dispatcher calls column and diagonal updates, full and partial argsort,
uniform-bin-edge construction, segment sum, top-k, row and vector
concatenation, one-hot, packed upper triangle, column/row/leading slices,
transpose, triangular masks, and their checked flat gather helpers. Prepared
sort prefixes, segment boundaries, index maps, and destination tables are
validated exactly. Sorts are stable total-order operations; gathers reject
out-of-range indices; scatter conflict policy is explicit.

### Graph, clustering, and reinforcement learning, `graph_cluster_rl.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_boruvka_mst` | `gpu-core/src/cluster.rs:395` |
| `gpu_categorical_logprob` | `gpu-core/src/rl.rs:131` |
| `gpu_centroid_update` | `gpu-core/src/kernels.rs:4685` |
| `gpu_core_distance` | `gpu-core/src/cluster.rs:476` |
| `gpu_csr_spmm` | `gpu-core/src/graph.rs:82` |
| `gpu_csr_spmv` | `gpu-core/src/graph.rs:56` |
| `gpu_degree` | `gpu-core/src/graph.rs:158` |
| `gpu_discounted_returns` | `gpu-core/src/rl.rs:57` |
| `gpu_fixed_radius_neighbors` | `gpu-core/src/cluster.rs:179` |
| `gpu_gae` | `gpu-core/src/rl.rs:79` |
| `gpu_gaussian_logprob` | `gpu-core/src/rl.rs:156` |
| `gpu_gcn_norm` | `gpu-core/src/graph.rs:180` |
| `gpu_neighbor_aggregate` | `gpu-core/src/graph.rs:111` |
| `gpu_pairwise_l2` | `gpu-core/src/kernels.rs:5239` |
| `gpu_td_targets` | `gpu-core/src/rl.rs:105` |
| `gpu_union_find_cc` | `gpu-core/src/cluster.rs:251` |

The dispatcher calls the degree, temporal-difference, GCN normalization,
discounted-return, generalized-advantage, Gaussian and categorical log
probability, CSR padded, neighbor aggregate, centroid, core-distance,
pairwise-L2, fixed-radius, union-find, and Boruvka emitters. Their shared
helpers construct checked gathers, scans, histograms, reductions, and scatters.
Prepared node counts, edge counts, CSR row pointers, component rounds, and
int32 index facts are required. Boruvka and union-find use bounded prepared
rounds rather than host loops; duplicate updates carry explicit atomic policies.

### Tree and boosting, `tree_boosting.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_argmax_write_split` | `gpu-core/src/kernels.rs:4155` |
| `gpu_bootstrap_sample` | `gpu-core/src/forest.rs:58` |
| `gpu_feature_subset` | `gpu-core/src/forest.rs:82` |
| `gpu_grad_hess_into` | `gpu-core/src/kernels.rs:3703` |
| `gpu_histogram_build` | `gpu-core/src/kernels.rs:6745` |
| `gpu_leaf_finalize` | `gpu-core/src/kernels.rs:4372` |
| `gpu_leaf_reduce` | `gpu-core/src/kernels.rs:4346` |
| `gpu_leaf_split_apply` | `gpu-core/src/kernels.rs:7173` |
| `gpu_lgbm_best_split` | `gpu-core/src/kernels.rs:7080` |
| `gpu_lgbm_hist_subtract` | `gpu-core/src/kernels.rs:7053` |
| `gpu_lgbm_histogram` | `gpu-core/src/kernels.rs:7018` |
| `gpu_lgbm_leaf_reduce` | `gpu-core/src/kernels.rs:7119` |
| `gpu_oblivious_histogram` | `gpu-core/src/kernels.rs:4205` |
| `gpu_oblivious_split_eval` | `gpu-core/src/kernels.rs:4396` |
| `gpu_ordered_target_stats` | `gpu-core/src/catboost.rs:100` |
| `gpu_random_threshold_split` | `gpu-core/src/forest.rs:108` |
| `gpu_scatter_add_by_leaf` | `gpu-core/src/kernels.rs:4323` |
| `gpu_scatter_add_by_leaf_col` | `gpu-core/src/kernels.rs:4497` |
| `gpu_split_eval` | `gpu-core/src/kernels.rs:6778` |
| `gpu_tb_histogram` | `gpu-core/src/kernels.rs:3737` |
| `gpu_tb_leaf_sum` | `gpu-core/src/kernels.rs:3830` |
| `gpu_tb_leaf_val` | `gpu-core/src/kernels.rs:3856` |
| `gpu_tb_split_eval` | `gpu-core/src/kernels.rs:3770` |
| `gpu_write_split` | `gpu-core/src/kernels.rs:4182` |

The dispatcher maps these rows to `emit_argmax_split`,
`emit_bootstrap_sample`, `emit_feature_subset`, `emit_gradient_hessian`,
`emit_tree_histogram`, `emit_regularized_leaf`, `emit_leaf_reduce`,
`emit_tree_leaf_split`, `emit_two_bin_split`, `emit_histogram_subtraction`,
`emit_ordered_target_statistics`, `emit_random_threshold`,
`emit_leaf_prediction`, and `emit_write_split`. Histograms and scans use
checked bins and deterministic ordering. Split selection uses checked int32
features and bins with deterministic lowest-index ties. Random rows use the
counter-keyed language random primitive. Leaf scatter destinations and table
facts are explicit preparation inputs, not inferred by the materializer.

### Training, `training.rs`

| Symbol | Paired source |
| --- | --- |
| `gpu_bce_with_logits` | `gpu-core/src/losses.rs:145` |
| `gpu_linear_backward_full_into` | `gpu-core/src/kernels.rs:3434` |
| `gpu_linear_backward_weights_only_into` | `gpu-core/src/kernels.rs:3400` |
| `gpu_linear_f32` | `gpu-core/src/nn_f32.rs:200` |
| `gpu_linear_into` | `gpu-core/src/kernels.rs:2796` |
| `gpu_matvec_bias_into` | `gpu-core/src/kernels.rs:3084` |

`gpu_linear_f32` and `gpu_linear_into` call `emit_linear`, which contracts
`input[m,k]` with `weight[k,n]`, then maps a broadcast bias into `output[m,n]`.
`gpu_matvec_bias_into` uses the vector weight and scalar bias shape. Full and
weights-only backward emit the input and weight contractions plus an axis-zero
fixed-tree bias reduction. `gpu_bce_with_logits` calls the owned stable
binary-cross-entropy SSA program and writes losses and gradients in one
elementwise primitive. All inputs and outputs are f32, exact shapes are
required, and the designated iteration input must be the one named by the
source branch.

### Deliberately unowned families

`inference_quantization_diffusion.rs` and `creation_shape_misc.rs` each return
`supports = false` for every descriptor and always return
`FamilyDispatch::NotOwned`. Their descriptive composition recipes remain in
the registry and in `remaining_composition_manifest`; no placeholder graph is
emitted. If a future table row is added without a matching symbol branch, its
family returns `GraphMaterializationFailed`, exposing table and dispatch drift.

## Shared scalar and primitive callees

Family modules use private helpers in `materialize.rs` instead of duplicating
boundary policy:

* `emit_checked_gather_identity` checks f32 values, i32 indices, the prepared
  axis, optional verification fact, exact gather result shape, then emits a
  checked gather followed by an identity `Elementwise` program.
* `sum_reduction` validates axes against the selected iteration-shape rank and
  creates a `ReduceOperator::Sum` with `ReduceResult::Value`, caller-selected
  `keep_dimensions`, and prepared `tree_lanes`.
* `scalar_builder`, `scalar_input`, `scalar_f32`, `scalar_binary`,
  `scalar_unary`, `scalar_ternary`, and `scalar_finish` are the one typed SSA
  construction path. Builder failures become `GraphMaterializationFailed`.
* `require_nonnegative` and `require_positive` insert device-side
  `ScalarOpcode::Require` guards. They do not turn invalid data into a default.
* `identity_program`, `multiply_program`, normalization programs, optimizer
  programs, and tree branch programs are all Recipe-owned scalar SSA. Prepared
  coefficients and constants are embedded only after their host-side domain
  checks pass.
* `forbidden_aliases` creates a complete `AliasPermission::Forbidden` matrix
  for ordinary emitted kernels. Operation-specific atomic and unique scatter
  semantics stay in their primitive descriptors.

The source-specific materialization contract, tensor ABI tables, exact
workspace formulae, and legacy formula notes are recorded in
[`ops/MATERIALIZATION.md`](../../MATERIALIZATION.md). The family implementation
docs under `ops/.docs/src/materialize/` describe each emitter's ABI in detail;
this document is the central dispatch and lifecycle contract.

## Production callers and graph insertion

### Training compiler

`training/src/compile.rs:10937-10998` defines the production
`TrainingGraphCompiler::materialize` wrapper. For each symbolic operation it:

1. clones compiler tensors for the requested names;
2. marks clones as external inputs or external outputs;
3. creates `NamedTensor` slices preserving the caller's ABI order;
4. reserves 64 value IDs and 64 kernel IDs from the compiler cursors, using
   checked cursor increments;
5. resolves the unique operation descriptor through
   `operation_registry().resolve_unique(symbol)`;
6. passes the selected iteration input, typed parameters, the explicit
   `IdentityNamespace`, and `WORKSPACE_LIMIT = ByteCount::new(u64::MAX)`;
7. inserts the returned graph through `insert_materialized_graph` with the
   caller's `IterationDomain`.

Insertion clones every returned tensor and marks it nonexternal. An existing
tensor ID is accepted only when dtype, shape, layout, and storage bytes all
match. A conflicting contract is a training compiler language error. Every
returned node is appended with the same iteration domain. The wrapper is used
by real training paths such as `gpu_bce_with_logits`, linear layers, optimizer
and metric operations, and the prepared tree/graph workflows.

### Inference compiler

`training/src/inference.rs:2008-2077` implements the analogous
`InferenceGraphCompiler::materialize` wrapper. It performs the same clone,
external-flag, name, unique-resolution, and 64-value/64-kernel reservation
steps. Inference also passes an effectively unlimited `WORKSPACE_LIMIT`, then
inserts each returned tensor contract and node with `IterationDomain::first()`.
The insertion check is identical: an existing ID must have the same dtype,
shape, layout, and storage bytes. The inference wrapper is exercised by
operations such as `gpu_concat_into`, prepared feature assembly, and source
qualified inference branches.

Other `recipe-ops` materializers, including categorical Bayes, binary metrics,
K-means, KNN outputs, and tree ensemble inference, are dedicated graph builders
in sibling source files. They have their own requirement calculators and append
APIs. They do not call `materialize_composition` unless the caller is using one
of the structured operation rows above. Their explicit identity and workspace
requests follow the same ownership contract but are not hidden behind this
module.

### Public facade

`src/facade.rs` reexports the operation types and forwards:

* `operations::registry`, `all`, `resolve`, and `resolve_exact` to the registry;
* `operations::validate_composition` to `recipe_ops::validate_composition`;
* `operations::materialize` to `recipe_ops::materialize_composition`;
* `operations::remaining_compositions` to the fail-closed manifest.

The facade is declaration and forwarding surface only. It does not infer tensor
names, manufacture parameters, or catch a materialization error.

## Failure behavior

The module returns the first observed contract failure and never retries or
falls back. The relevant failure vocabulary is:

| Error kind | Materialization cause |
| --- | --- |
| `WrongLoweringKind` | The descriptor is not a structured composition, or a composition expansion API received another lowering form. |
| `InvalidMaterializationRequest` | Empty or duplicate names, duplicate IDs, missing names, wrong flags, ABI key mismatch, wrong dtype, false fact, wrong iteration input, invalid parameter domain, or invalid request-level shape relation. |
| `MissingPreparedParameter` / `PreparedParameterTypeMismatch` | A required typed value or repeat bound is absent or has another `PreparedParameter` variant. |
| `InvalidCompositionRecipe` | The recipe metadata, repeat body, fixed bound, parameter name, or nesting depth violates `CompositionRecipe::validate`. |
| `IterationBoundUnresolved` | A shape axis is outside the selected input rank, or a minimum extent has no value. |
| `CompositionExpansionOverflow` | More than one million primitive steps would be unrolled. |
| `MissingConcreteFormula` | No family table owns the exact source-qualified structured descriptor. |
| `UnsupportedConcreteShape` | A concrete emitter's rank, extent, product, int32 index, exact-f32 divisor, or source-specific shape restriction fails. |
| `IdentityNamespaceOverlap` | A boundary tensor occupies the intermediate range, or caller reservations overlap. |
| `IdentityNamespaceExhausted` | A value or kernel range overflows or is too small for the emitted graph. |
| `WorkspaceArithmeticOverflow` | Summing intermediate storage bytes overflows `u64`. |
| `WorkspaceLimitExceeded` | Emitted operation-owned intermediate bytes exceed the request limit. |
| `GraphMaterializationFailed` | A scalar builder, language validator, family dispatch branch, stage count, stage family, primitive contract, or final graph validation fails. |

`WorkspaceFormulaMismatch` is part of the shared operation error vocabulary for
other requirement calculators. The central `GraphBuilder` derives workspace
from emitted tensor storage and therefore reports arithmetic or limit failures
directly rather than comparing against an untrusted duplicate formula.

## Authoritative end state

Success means all of the following are true:

* the descriptor is a source-qualified owned composition;
* every boundary tensor and preparation fact passed the exact ABI and semantic
  checks;
* every repeat is statically resolved and represented in `ResolvedComposition`;
* every intermediate and kernel identity came from the caller's reserved
  namespace;
* every intermediate byte is present in `WorkspaceAllocation` and within the
  caller limit;
* every resolved step maps to one recorded `StageEmission` sequence whose
  terminal primitive family matches the recipe;
* every primitive uses a typed Recipe language kind, explicit bounds, explicit
  reduction or random order, and explicit alias/conflict policy;
* `CalculationGraph::validate` accepted all tensor and primitive contracts;
* the caller inserted the returned contracts into the larger static program and
  assigned the intended iteration domain.

The returned graph is authoritative model work for later lowering and planning.
Preparation, allocation, scheduling, native-image loading, and runtime
execution remain downstream concerns. An unresolved row is not partially
materialized: it stays in `remaining_composition_manifest` with all four
`MissingConcreteComponent` values, and no neighboring operation or host path is
substituted.
