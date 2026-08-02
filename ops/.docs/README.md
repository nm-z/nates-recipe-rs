# `recipe-ops`

`recipe-ops` is Recipe's canonical operation inventory and lowering boundary.
It turns the source-qualified legacy operation surface into one immutable
registry, then gives each entry exactly one explicit meaning: a scalar SSA
program, a direct primitive recipe, a finite structured composition, a checked
workspace formula, a non-calculation declaration, or an unsupported entry
that fails closed. The crate never calls a vendor library, runs a host payload
loop, or substitutes a CPU implementation for an unowned operation.

The graph emitted by this crate is the `recipe-language::CalculationGraph`
contract. `recipe-primitives` lowers that graph for measured hardware, while
the planner, preparation, and executors own placement, scheduling, allocation,
native-image loading, and lifecycle. `recipe-ops` owns operation semantics,
typed payload domains, static preparation facts, and the exact graph fragment
that realizes those semantics.

## Position in the pipeline

The complete operation path is:

```text
operation-surface.txt
        |
        |  ops/build.rs parses exact (symbol, source) rows
        v
RAW_OPERATION_SURFACE (generated in OUT_DIR)
        |
        v
OperationRegistry::iter / resolve_unique / resolve_exact
        |
        +--> ScalarRecipe       -> lower_scalar -> ScalarProgram
        +--> PrimitiveRecipe    -> lower_primitive -> recipe-primitives
        +--> CompositionRecipe  -> validate/expand/materialize
        +--> WorkspaceFormula   -> evaluate_workspace
        +--> NonCalculationRecipe (metadata, host, or lifecycle)
        +--> UnsupportedReason  -> explicit fail-closed error
                                      |
                                      v
                           CalculationGraph fragment
                                      |
                                      v
                         recipe-primitives and AOT planning
```

Structured materialization is preparation work. Shapes, typed parameters,
identity reservations, and workspace limits are fixed first. The materializer
then expands every repeat into a finite dependency chain, emits only
`Elementwise`, `Reduce`, `Scan`, `Contraction`, `Gather`, `Scatter`,
`Histogram`, `Sort`, `IndexMap`, or `Random` primitives, validates the graph,
and returns immutable intermediate and kernel reservations. No materialized
result contains an operation callback or a data-dependent host loop.

## Manifest and build contract

[`ops/Cargo.toml`](../Cargo.toml) declares package `recipe-ops` version
`0.1.0`, Rust edition 2024, MIT licensing, and the description
"Canonical operation registry and honest lowering boundary for Recipe". The
only direct dependencies are:

| Dependency | Used for |
| --- | --- |
| `recipe-core` | `DType`, byte units, stable value and kernel identities, scalar SSA records, opcodes, and alias permissions. |
| `recipe-language` | Shapes, tensors, primitive kinds, primitive kernels, calculation graphs, and typed scalar builders. |
| `recipe-math` | Owned binary32 math algorithms embedded in scalar programs. |
| `recipe-primitives` | Backend-neutral lowering of a validated primitive kernel for a selected lowering hardware description. |

Unsafe Rust is forbidden and missing `Debug` implementations are denied.
There are no features, binaries, examples, runtime files, or test targets in
the package manifest. [`MATERIALIZATION.md`](../MATERIALIZATION.md) records
the source-qualified composition ABI and fragment-identity contract; this
README records the crate-wide structure.

### `operation-surface.txt` and `build.rs`

[`build.rs`](../build.rs) is the only build-time code. It watches
`../operation-surface.txt`, skips blank and comment lines, and requires every
remaining line to contain exactly two nonempty tab-separated fields:

```text
symbol<TAB>legacy source
```

The parser assigns a zero-based `ordinal`, the one-based source `line`, and
per-symbol `occurrence` and `occurrences` counts. It writes
`OUT_DIR/operation_surface.rs` containing `RAW_OPERATION_SURFACE` and
`RAW_OPERATION_COUNT`. A malformed row, missing source, extra field, empty
field, or `u16` occurrence overflow aborts the build. The generated inventory
preserves source order and does not deduplicate equal symbols.

The checked snapshot contains 421 parsed legacy rows. The registry appends two
Recipe-owned rows (`recipe_max_pool_1d` and
`recipe_max_pool_1d_backward`) after that prefix, so
`operation_registry().len()` is 423. Four symbols are intentionally
source-ambiguous in the legacy prefix: `predict` has three rows,
`predict_proba` has two, `train` has three, and `train_multiclass` has two.
Callers must use `resolve_exact(symbol, source)` for those entries; a symbol
only lookup returns `AmbiguousSymbol`.

## Module graph

All implementation modules are private. [`src/lib.rs`](../src/lib.rs) is the
single public facade and re-exports the typed contracts listed below.

```text
lib.rs
├── registry.rs       OperationId, descriptors, registry, classification
│   ├── scalar.rs     scalar opcode/math/composite recipes and SSA lowering
│   ├── primitive.rs  direct primitive recipes and recipe-primitives bridge
│   ├── composition.rs finite multi-stage descriptors and validation
│   ├── workspace.rs  static scratch formulas
│   └── non_calculation.rs host, facade, metadata, and lifecycle entries
├── materialize.rs    request validation, repeat expansion, graph emission
│   ├── optimizer_normalization.rs
│   ├── solver_fft.rs
│   ├── attention_sequence_embedding.rs
│   ├── convolution_pooling.rs
│   ├── loss_metrics.rs
│   ├── indexing_sort_encoding.rs
│   ├── graph_cluster_rl.rs
│   ├── tree_boosting.rs
│   ├── inference_quantization_diffusion.rs
│   ├── creation_shape_misc.rs
│   └── training.rs
├── bayes.rs          categorical Bayes graph builder
├── binary_metrics.rs binary metrics graph builder
├── kmeans.rs         deterministic initialization and Lloyd transition
├── knn_outputs.rs    numeric and categorical KNN output graph builder
├── tree.rs           complete binary-tree ensemble inference
├── convolution.rs    prepared 1-D receptive-field metadata
└── pooling.rs        prepared channelwise max-pool metadata
```

`error.rs` is shared by every module. `recipe-core` and `recipe-language`
provide the identity, tensor, scalar, primitive, and graph types; no module
reimplements those contracts. The materializer family modules depend on the
private helpers in `materialize.rs`, so tensor ABI checks, identity allocation,
workspace accounting, alias rules, stage checks, and graph validation have one
implementation.

## Registry and descriptor semantics

`OperationId` is stable within the generated inventory. It exposes `ordinal`,
`surface_line`, `occurrence`, `occurrences`, `is_duplicate_symbol`, and
`is_recipe_owned`. A legacy ID has a nonzero source line; the two native
extensions use line zero and ordinals after `RAW_OPERATION_COUNT`.

`OperationDescriptor` carries:

| Field | Meaning |
| --- | --- |
| `id`, `symbol`, `source` | Source-qualified identity and original inventory location. |
| `family` | `OperationFamily`, selected from the owned recipe or source/name classification. |
| `dtypes` | Canonical f32/int32 payload contract, exact scalar types, or nonnumeric host data. |
| `lowering` | One `LoweringAvailability` variant. |
| `definition` | Human-readable definition supplied by the selected recipe. |
| `alias` | `NoAlias`, a required input/output alias, or an operation-specific contract. |
| `determinism` | Exact scalar order, fixed primitive order, counter-based random, explicit atomic policy, host determinism, or pending definition. |
| `legacy_dtype` | Optional excluded F16, F64, U8, or dynamic-quantized legacy payload marker. It never authorizes that payload. |

`OperationRegistry` is a zero-sized handle. `iter` yields the complete
canonical order, `surface_iter` yields only the 421 generated rows, and
`owned_iter` yields the two extension rows. `named` permits inspection of all
source-qualified matches. `resolve_unique` returns `UnknownOperation` for a
missing symbol and `AmbiguousSymbol` for more than one row. `resolve_exact`
requires one exact `(symbol, source)` pair and rejects both missing and
duplicate exact rows.

### Classification precedence

`registry::lowering` is deliberately first-match and source-qualified in this
order:

1. `ScalarRecipe::for_symbol`.
2. `PrimitiveRecipe::for_symbol`.
3. `WorkspaceFormula::for_symbol`.
4. `NonCalculationRecipe::for_entry(symbol, source)`.
5. `CompositionRecipe::for_entry(symbol, source)`.
6. An explicit legacy dtype exclusion.
7. Dynamic `convert` or `gpu_convert` format conversion.
8. A non-`gpu_` host behavior entry.
9. `DedicatedPrimitiveCompositionPending` for an otherwise unowned GPU symbol.

This order is significant. A symbol that has a scalar recipe never falls
through to a composition with a similar name, and a source mismatch never
falls through to an adjacent legacy implementation. The dtype and family
functions use the selected lowering first, then apply conservative source and
symbol heuristics only for unsupported rows. GPU entries default to canonical
f32 payloads; names containing index, mask, route, sort, histogram, or related
terms receive the f32/int32 payload contract. Explicit F16, F64, U8, and
dynamic-quantized rows are marked `legacy_dtype` and remain excluded even when
a canonical replacement recipe exists.

`AliasContract` describes the public legacy alias expectation. In-place add,
multiply, scale, subtract, SGD, scatter-add, and in-place softmax entries have
specific required aliases; `_into` entries default to no alias; other GPU
entries are operation-specific. `DeterminismContract` treats random and
bootstrap operations as counter-based, scatter and histogram operations as
explicit-atomic, scalar programs as per-element exact order, and primitive or
composition operations as fixed primitive order.

## Scalar lowering

[`scalar.rs`](../src/scalar.rs) owns 94 source symbols grouped into three
recipe forms:

* `Opcode` creates a typed one-instruction program for add, subtract,
  multiply, divide, absolute, negate, min, max, FMA, comparisons, select,
  square root, and the other `ScalarOpcode` operations.
* `Math` delegates to one owned `recipe_math::MathFunction`, including
  `atan2`, `ceil`, `cos`, `exp`, `expm1`, `floor`, `fmod`, `log`, `log1p`,
  `pow`, reciprocal, round-to-nearest-even, reciprocal square root, sigmoid,
  sign, sine, softplus, tangent, hyperbolic tangent, and truncation.
* `Composite` builds a multi-instruction scalar SSA program for reversed
  in-place operand order, clamping, BCE gradients, dropout, ReLU family,
  ELU/SELU, sigmoid/tanh/SiLU/GELU/GLU, scaled exponentials, VAE
  reparameterization, KL elements, MAE and Huber gradients, and SGD updates.

The exact symbol table is in `ScalarRecipe::for_symbol`. All scalar payloads
are f32 except comparison results and masks, which are int32. `gpu_where_mask`
requires an int32 mask and two f32 values. Math arity is taken from
`MathFunction`; composite input signatures are fixed by
`CompositeScalar::inputs`.

`lower_scalar` accepts only a descriptor whose lowering is `Scalar`. It builds
typed inputs and constants with the private `Composer`, inlines owned math
programs into the same value-ID space, checks every opcode result dtype, and
calls `ScalarProgram::validate` before returning. Passing a primitive,
composition, workspace, non-calculation, or unsupported descriptor returns
`WrongLoweringKind` or `UnsupportedLowering` with the operation ID. Composer
value-ID exhaustion, an invalid opcode signature, an unknown inlined value, or
an invalid output becomes `InvalidScalarProgram`.

The exported canonical model programs are leaky-ReLU forward/backward, PReLU
forward/backward, alpha-one ELU forward/backward, self-normalizing SELU
forward/backward, and parameterless focal loss with `FOCAL_ALPHA = 0.25` and
`FOCAL_GAMMA = 2.0`. These helpers are used by `training/src/forward.rs` and
are not alternate registry paths.

## Direct primitive lowering

[`primitive.rs`](../src/primitive.rs) owns 29 direct primitive symbols. A
`PrimitiveRecipe` records the operation family and only the invariant that
can be checked before the caller supplies shape-dependent kernel parameters.
Axes, extents, tree widths, random keys, bounds policies, and aliases remain
in the borrowed `PrimitiveKernel` in `PrimitiveRequest`.

| Primitive recipe family | Source symbols and required shape policy |
| --- | --- |
| Fixed-tree reductions | `gpu_argmax_f32` (all axes, index), `gpu_argmax_rows` (last axis, index), `gpu_argmin_rows` (last axis, index), `gpu_max_all`, `gpu_min_all`, `gpu_sum_all` (all axes, value), `gpu_reduce_max_cols`, `gpu_reduce_min_cols`, `gpu_reduce_sum_cols_into` (first axis), `gpu_reduce_max_rows`, `gpu_reduce_min_rows`, `gpu_reduce_sum_rows` (last axis). |
| Fixed-tree scans | `gpu_cummax`, `gpu_cumprod`, `gpu_prefix_sum_inclusive` (any axis, inclusive), `gpu_prefix_sum_exclusive` (any axis, exclusive sum with f32 zero identity), `gpu_cumsum_cols` (first axis), `gpu_cumsum_rows` (last axis). Reverse scans are rejected. |
| Contractions | `gpu_dot` (rank-1 vector), `gpu_gemm` (matrix contract `(1,0)`), `gpu_gemm_at` (left-transposed `(0,0)`), `gpu_gemm_bt` and `gpu_gemm_bt_into` (right-transposed `(1,1)`), `gpu_bmm_into` (rank at least three with nonempty batch axes). |
| Indexed data movement | `gpu_gather_rows_into` is a first-axis checked gather with `IndexBounds::Reject`; `gpu_scatter_add` is a first-axis checked atomic-add scatter requiring sequentially consistent atomic conflict policy and exact output/input alias. |
| Sort and random | `gpu_sort` is an ascending, non-stable, all-axis f32 sort with no index output; `gpu_rand_uniform_into` and `gpu_randn` are no-input counter-keyed Philox4x32-10 maps producing f32. |

`PrimitiveRecipe::matches` rejects any primitive kind or parameter that does
not satisfy the recipe, including wrong axes, reverse scans, noncanonical
exclusive identities, non-`Reject` bounds, sort index emission, or scatter
alias/atomic mismatches. A match is passed to
`recipe_primitives::lower`; backend lowering errors become
`PrimitiveLoweringFailed`. `lower_index_map` deliberately lowers the
iteration-aware affine index primitive without adding a legacy registry
symbol, and requires no inputs, one int32 output, and no aliases.

## Structured composition descriptors

[`composition.rs`](../src/composition.rs) is descriptive, not executable.
`CompositionPayload` declares f32, i32, both, or either payload domains.
`CompositionStep` is either a primitive family with a required role or a
repeat with an `IterationBound`. The bound can be fixed, a shape extent, the
minimum shape extent, a ceiling log2 shape extent, or a named prepared `U64`
parameter.

The primitive role vocabulary is intentionally small and maps directly to
`recipe-language`: elementwise map, fixed-tree reduce, fixed-tree scan,
canonical contraction, checked gather, explicit-policy scatter, bounded
histogram, stable total-order sort, and counter-keyed random generation.
Reusable static templates cover:

* linear forward/backward, softmax and log-softmax, normalization and
  normalization backward, pooling and convolution forward/backward;
* attention forward/backward, recurrent cells, optimizer updates, and global
  gradient clipping;
* tree histograms, split scoring, routing, segment reductions, count-distinct
  encoding, and ordered target statistics;
* radix-2 FFT, Cholesky, triangular solve, LU, QR, Jacobi eigensolver, SVD,
  Boruvka, union-find, dynamic programming, autoregressive generation,
  boosting rounds, and bounded SMO iterations;
* losses, distances, multiclass metrics, KNN-like sorting, attention head
  transforms, embeddings, positional/rotary transforms, and state-space scans.

`CompositionRecipe::validate` requires nonempty name, definition, and top
level steps, a nonempty role on every step, a nonempty repeat body, nonzero
fixed counts, nonempty prepared parameter names, and at most eight nesting
levels. `validate_composition` applies that check to a descriptor and rejects
non-composition lowerings with `WrongLoweringKind`.

`CompositionRecipe::for_entry` is source-qualified for legacy `predict`,
`predict_proba`, `train`, and `train_multiclass` entries. A CatBoost symbol is
never accepted as LightGBM or XGBoost merely because the symbol matches.
Other entries retain their exact operation symbol mapping. A recipe can be
descriptive while still unresolved at the concrete ABI boundary; such rows
remain visible through `remaining_composition_manifest`.

## Materialization boundary

[`materialize.rs`](../src/materialize.rs) turns one supported composition into
one validated graph fragment. Its public types are intentionally immutable
descriptions:

* `NamedTensor` gives every boundary tensor a unique name.
* `PreparedParameter` and `PreparedParameters` carry only `U64`, `I32`,
  `F32Bits`, and `Bool` values fixed during preparation.
* `IdentityNamespace` reserves half-open intermediate-value and kernel ranges
  for one independently compiled fragment.
* `MaterializationRequest` combines the descriptor, exact input/output ABI,
  the tensor used to resolve shape bounds, prepared parameters, the namespace,
  and a workspace byte limit.
* `ResolvedBound`, `ResolvedIteration`, `ResolvedStep`, and
  `ResolvedComposition` preserve every static repeat value, role, index,
  nesting context, and linear dependency.
* `WorkspaceObject` and `WorkspaceAllocation` account for every intermediate
  tensor before execution.
* `StageEmission` maps each resolved step to its concrete kernel IDs.
* `MaterializedComposition` returns the validated graph, resolved recipe,
  stage map, workspace allocation, and the namespace that produced it.

### Request validation and expansion

Before dispatch, `validate_request` requires a composition descriptor, at least
one input and output, unique nonempty names, unique tensor IDs, valid tensors,
external-input flags on every input, and non-input external-output flags on
every output. Every concrete family then calls `require_exact_abi`: the set of
input names, output names, and prepared parameter names must match exactly.
Missing names, additional names, wrong parameter variants, wrong dtypes,
wrong shapes, false verification facts, nonfinite values, and dimensions
outside the checked int32 or exact-f32 domains fail closed.

`expand_composition` resolves all bounds from one declared iteration-shape
input and the typed parameter map. It records each bound and recursively
unrolls each repeat. A primitive receives a copy of its surrounding iteration
stack and depends on the immediately previous resolved step. Expansion stops
with `CompositionExpansionOverflow` at the fixed one-million-step limit.
Missing shape axes produce `IterationBoundUnresolved`; missing or mistyped
prepared bounds produce `MissingPreparedParameter` or
`PreparedParameterTypeMismatch`.

### Emission and identity ownership

`GraphBuilder` starts at the caller's reserved value and kernel IDs. It rejects
boundary tensor IDs inside the intermediate range, allocates contiguous
intermediate tensors, accumulates their checked storage bytes, enforces the
workspace limit, assigns forbidden aliases to ordinary emitted kernels, and
validates the final `CalculationGraph`.

`Emitter::emit_stage` checks that every concrete materializer emits exactly
one kernel sequence for the next resolved step and that the last kernel's
`PrimitiveFamily` equals the resolved family. Emitting too many or too few
stages, an empty stage, a wrong family, a missing dispatch, an exhausted value
or kernel range, a workspace overflow, or graph validation failure returns
`GraphMaterializationFailed`, `IdentityNamespaceExhausted`,
`WorkspaceArithmeticOverflow`, or the more specific boundary error.

`validate_identity_namespaces` checks every pair of caller reservations for
overlap in both value and kernel ranges. Independently materialized fragments
can therefore be assembled without implicit "start after the largest input"
or hard-coded kernel IDs. [`MATERIALIZATION.md`](../MATERIALIZATION.md) gives
the concrete ABI and fragment examples.

### Concrete dispatch ownership

`dispatch_concrete` probes family modules in this fixed order:

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

Each nonempty `OPERATIONS` table is a set of exact `(symbol, source)` pairs;
`supports` does not match by symbol alone. The current concrete boundary owns
136 source-qualified entries:

| Family module | Count | Concrete symbols |
| --- | ---: | --- |
| `optimizer_normalization.rs` | 23 | `gpu_adagrad_update`, `gpu_adam_update`, `gpu_adamw_update`, `gpu_batchnorm_backward`, `gpu_batchnorm_forward`, `gpu_batchnorm_inference`, `gpu_bn_update_running`, `gpu_grad_clip_norm`, `gpu_lamb_phase1`, `gpu_lamb_phase2`, `gpu_layernorm_backward_f32`, `gpu_layernorm_backward_full_into`, `gpu_layernorm_f32`, `gpu_layernorm_into`, `gpu_layernorm_opt_into`, `gpu_lion_update`, `gpu_momentum_update`, `gpu_nadam_update`, `gpu_rmsnorm`, `gpu_rmsnorm_backward`, `gpu_rmsnorm_f64`, `gpu_rmsnorm_f64_nogamma`, `gpu_rmsprop_update`. |
| `solver_fft.rs` | 2 | `gpu_fft_c2c_1d`, `gpu_tri_solve`. |
| `attention_sequence_embedding.rs` | 12 | `gpu_causal_softmax_rows`, `gpu_embed_blend`, `gpu_embedding_backward`, `gpu_mha_merge`, `gpu_mha_split`, `gpu_positional_encoding`, `gpu_repeat_rows`, `gpu_rope`, `gpu_rope_partial`, `gpu_rope_partial_factors`, `gpu_rope_partial_factors_pos`, `gpu_rope_partial_pos`. |
| `convolution_pooling.rs` | 21 | `gpu_avg_pool_1d`, `gpu_avg_pool_2d`, `gpu_avg_pool_2d_backward`, `gpu_avg_pool_2d_backward_f32`, `gpu_avg_pool_2d_f32`, `gpu_col2im_1d`, `gpu_col2im_2d`, `gpu_col2im_2d_ext`, `gpu_im2col_1d`, `gpu_im2col_2d`, `gpu_im2col_2d_ext`, `gpu_max_pool_1d`, `gpu_max_pool_1d_backward`, `gpu_max_pool_2d`, `gpu_max_pool_2d_backward`, `gpu_max_pool_2d_backward_f32`, `gpu_max_pool_2d_f32`, `gpu_pool_grad_expand`, `gpu_upsample_nearest_2d`, `recipe_max_pool_1d`, `recipe_max_pool_1d_backward`. |
| `loss_metrics.rs` | 15 | `gpu_accuracy`, `gpu_accuracy_into`, `gpu_argmax_accuracy_into`, `gpu_contrastive_loss`, `gpu_cosine_embedding_loss`, `gpu_has_nan`, `gpu_hinge_loss`, `gpu_isfinite_all`, `gpu_kl_div_loss`, `gpu_mean_all`, `gpu_mse_into`, `gpu_reduce_mean_cols`, `gpu_reduce_var_cols`, `gpu_ss_res_into`, `gpu_triplet_loss`. |
| `indexing_sort_encoding.rs` | 17 | `gpu_add_col`, `gpu_add_col_scaled_inplace`, `gpu_add_diag`, `gpu_argsort`, `gpu_bin_edges_uniform`, `gpu_concat_into`, `gpu_one_hot`, `gpu_pack_upper_tri`, `gpu_partial_argsort`, `gpu_segment_sum`, `gpu_slice_cols`, `gpu_slice_lead_into`, `gpu_slice_rows`, `gpu_topk_per_row`, `gpu_transpose`, `gpu_tril_mask`, `gpu_vconcat`. |
| `graph_cluster_rl.rs` | 16 | `gpu_boruvka_mst`, `gpu_categorical_logprob`, `gpu_centroid_update`, `gpu_core_distance`, `gpu_csr_spmm`, `gpu_csr_spmv`, `gpu_degree`, `gpu_discounted_returns`, `gpu_fixed_radius_neighbors`, `gpu_gae`, `gpu_gaussian_logprob`, `gpu_gcn_norm`, `gpu_neighbor_aggregate`, `gpu_pairwise_l2`, `gpu_td_targets`, `gpu_union_find_cc`. |
| `tree_boosting.rs` | 24 | `gpu_argmax_write_split`, `gpu_bootstrap_sample`, `gpu_feature_subset`, `gpu_grad_hess_into`, `gpu_histogram_build`, `gpu_leaf_finalize`, `gpu_leaf_reduce`, `gpu_leaf_split_apply`, `gpu_lgbm_best_split`, `gpu_lgbm_hist_subtract`, `gpu_lgbm_histogram`, `gpu_lgbm_leaf_reduce`, `gpu_oblivious_histogram`, `gpu_oblivious_split_eval`, `gpu_ordered_target_stats`, `gpu_random_threshold_split`, `gpu_scatter_add_by_leaf`, `gpu_scatter_add_by_leaf_col`, `gpu_split_eval`, `gpu_tb_histogram`, `gpu_tb_leaf_sum`, `gpu_tb_leaf_val`, `gpu_tb_split_eval`, `gpu_write_split`. |
| `training.rs` | 6 | `gpu_bce_with_logits`, `gpu_linear_backward_full_into`, `gpu_linear_backward_weights_only_into`, `gpu_linear_f32`, `gpu_linear_into`, `gpu_matvec_bias_into`. |

`inference_quantization_diffusion.rs` and `creation_shape_misc.rs` currently
return `supports = false` for every descriptor. Their descriptive compositions
remain in the registry and in `remaining_composition_manifest`; no placeholder
graph is emitted. A family table entry that claims ownership but has no symbol
branch returns `GraphMaterializationFailed`, making a dispatch/table drift
visible instead of silently selecting a neighboring operation.

The family implementations use checked gathers with `IndexBounds::Reject`,
fixed-tree reductions and scans, canonical-order contractions, explicit
atomic conflict policies, stable total-order sorting, counter-keyed random
maps, and Recipe-owned scalar programs. Examples of concrete facts include:

* attention and rotary tables require exact tensor and parameter name sets and
  true verification facts; causal softmax masks `column <= row` and checks a
  positive row sum;
* convolution and pooling materializers require checked receptive-field or
  winner tables; overlapping backward paths use explicit atomic add, while
  channelwise max-pool backward uses unique scatter;
* normalization and optimizer materializers require finite, positive, or
  unit-interval parameters as appropriate and embed bias corrections in SSA;
* tree and graph materializers require checked int32 indexes, bounded rounds,
  deterministic lowest-index ties, and explicit ordering for atomic updates;
* training materializers lower linear contractions, bias maps, and BCE-with-
  logits without a host-side loop.

## Workspace formulas

[`workspace.rs`](../src/workspace.rs) owns 24 static query symbols. A
`WorkspaceFormula` is selected by descriptor, checked against an exact number
of dimensions, and evaluated with checked `u64` arithmetic:

| Formula | Symbols and resource meaning |
| --- | --- |
| `NoPersistentScratch` | `gpu_dot_workspace_bytes`, zero persistent bytes because the contraction uses fixed shared-memory tiles. |
| `FixedTreeReduction` | `gpu_sum_all_workspace_bytes`, `gpu_max_all_workspace_bytes`, `gpu_min_all_workspace_bytes`, `gpu_mean_all_workspace_bytes`, and `gpu_reduce_sum_cols_workspace_bytes`, reserving every nonfinal 64-lane tree level. |
| `MapThenReduction` | `gpu_l2_norm_workspace_bytes`, one f32 map image plus reduction levels. |
| `FixedTreeScan` | `gpu_cumprod_workspace_bytes`, `gpu_cummax_workspace_bytes`, block totals and hierarchy outputs. |
| `StableSort` and `SortRunEncoding` | `gpu_random_permutation_workspace_bytes`, `gpu_count_distinct_workspace_bytes`, `gpu_run_length_workspace_bytes`, padded sort images, transition flags, and scan levels. |
| Solver formulas | Cholesky factor/solve/inverse, LU factor/solve, QR, symmetric eigensolver, and SVD workspace queries. Each includes its documented panels, vectors, and fault words. |
| `SplitKPartials` | `gpu_splitk_dw_partials_elems`, an f32 element count for at most eight deterministic 256-row slices. |

Workspace values are either bytes or f32 elements. Wrong dimension counts,
overflow, or invoking `evaluate_workspace` on a non-workspace descriptor return
`WorkspaceFormulaMismatch`, `WorkspaceArithmeticOverflow`, or
`WrongLoweringKind`. A formula is an accounting contract, not permission to
bind a vendor workspace handle.

## Non-calculation entries

[`non_calculation.rs`](../src/non_calculation.rs) makes orchestration explicit:

| Recipe | Entries | Meaning |
| --- | --- | --- |
| `FacadeDeclaration` | `Data`, `Model`, `Train`, `Infer` | Typed public declarations with no payload arithmetic. |
| `TextTokenization` | `encode` | Deterministic raw-text to canonical int32 token IDs before payload admission. |
| `ModelContainerParsing` | `parse_safetensors` and safetensors-source entries | Validated container metadata and opaque byte ranges before admission. |
| `ChatTemplateRendering` | `render_chat`, `render_template` | Deterministic host text rendering before tokenization. |
| `RunShutdown` | `gpu_shutdown` | Lifecycle exit transition; not a calculation. |
| `EliminatedVendorWorkspaceBinding` | `gpu_blas_workspace` | A prohibited vendor workspace setter that has been eliminated; owned scratch is fixed during preparation. |

These rows are classified as host, facade, encoding, parsing, lifecycle, or
workspace family operations. They cannot trigger a CPU calculation fallback.

## Specialized graph builders

The following modules are public graph builders rather than generic registry
composition materializers. They use the same `OperationError` kinds,
`IdentityNamespace` rules, forbidden aliases, checked storage contracts, and
final `CalculationGraph::validate` boundary.

### Categorical Bayes (`bayes.rs`)

`CategoricalBayesInferenceRequest` supplies reference parent/child codes,
query parent codes, parent multipliers and cardinalities, a probability output,
row and configuration counts, child classes, positive finite Laplace smoothing,
tree lanes, namespace, and workspace limit. Requirements are exactly ten
intermediates and eleven kernels with checked four-byte f32/i32 workspace.
Parent configuration codes reserve an unseen route; query counts are gathered
from a histogram and normalized with Laplace smoothing. Dimensions, histogram
bins, and all tensor dtypes/shapes must fit the checked int32 domain. The
`append_*` variant verifies that the caller already owns every boundary tensor,
rejects value or kernel collisions, and appends only intermediates and nodes.

### Binary metrics (`binary_metrics.rs`)

`BinaryClassificationMetricRequest` consumes rank-one f32 probabilities,
targets, and a pre-emitted per-element BCE vector. It writes `[1]` f32 mean
BCE, AUROC, AUPRC, Brier score, expected calibration error, and a statically
declared set of recall-at thresholds. Populations are limited to 9,999,999
exact binary32 counts, recall thresholds to 256, calibration bins to 256, and
tree lanes to powers of two through 1024. Thresholds are finite and in `[0,1]`
with distinct bit patterns. The graph validates inputs on device, performs a
stable descending sort with explicit tie groups, fixed scans and reductions,
and equal-width calibration bins. Requirements are computed before identity
reservation; `append_*` checks boundary storage and output-producer uniqueness.

### K-means (`kmeans.rs`)

Initialization copies point rows using an int32 modulo index map. It reserves
one intermediate and two kernels. One Lloyd transition reserves sixteen
intermediates and eighteen kernels, computes rooted-L2 assignments with a
fixed-tree lowest-index tie rule, accumulates membership sums and counts, keeps
empty centroids unchanged, and recomputes distances against updated centroids.
Rows must be 1 through 16,777,216 for exact f32 counts, dimensions and clusters
must be nonzero, indexes must fit int32, and tree lanes must be a power of two
through 1024. Resource requirements, output/input identity separation, and
workspace formulas are checked before emission.

### KNN outputs (`knn_outputs.rs`)

`KnnAllOutputRequest` accepts query and reference f32 matrices and any number
of independent numeric or categorical output requests. Each output has its
own int32 known mask and known-reference count, so semantic target spaces are
never concatenated. Distances are rooted L2, stable ascending distance ties
retain prepared reference order, and effective neighbors are
`min(neighbors, known_references)`. Numeric outputs use an unweighted f32
mean; categorical outputs histogram int32 codes and choose the lowest class on
ties. Dimensions, classes, known counts, exact f32 neighbor counts, tree lanes,
identity capacities, and workspace are all checked. Prediction IDs cannot alias
any source or another prediction. The append variant rejects existing
intermediate, kernel, or prediction producers.

### Tree ensemble inference (`tree.rs`)

`TreeEnsembleInferenceRequest` describes flattened row-major features, complete
binary-tree split features and thresholds, tree-major leaf values, and a
prediction output. Depth is 1 through 30. Every threshold tie goes left;
traversal uses checked int32 heap coordinates, fixed-order tree reduction, and
the finite caller scale, supporting both boosted sums and forest means.
Requirements derive internal nodes, leaves, depth-dependent identities, and
workspace before allocation. Boundary storage, namespace capacity, tree lanes,
and output producers are checked by the append variant.

### Convolution and pooling preparation

`prepare_channelwise_convolution_1d` accepts positive batch, input length,
input channels, filters, and kernel size with `kernel_size <= input_length`.
It prepares absolute receptive-field indexes and an input-gradient validity
image for valid stride-one convolution, exposes logical shapes, and reports
forward/backward four-byte workspace. Input and output element counts and host
table sizes must fit checked int32/`usize` domains.

`prepare_channelwise_max_pool_1d` prepares non-overlapping channelwise windows
with `groups = ceil(input_length / pool_size)`. The final short window is
retained and padded by repeating its final real coordinate. Winner bases and
backward batch coordinates are immutable tables. Forward parameters assert
channelwise non-overlap, tail repetition, and table verification; backward
parameters assert zero bases, matching winners, unique winning indexes, and
identity batch coordinates. Positive dimensions, int32 coordinates, checked
workspace, and table capacities are required.

## Consumers and boundaries

The root facade exposes `recipe_ops` in two ways:

* `recipe::engine::ops` re-exports the complete crate for advanced callers.
* `recipe::operations` wraps registry iteration and resolution, scalar and
  primitive lowering, composition validation/materialization, remaining
  manifests, and workspace evaluation.

The production training compiler is the principal consumer:

| Consumer | Ops boundary used |
| --- | --- |
| `training/src/forward.rs` | Canonical ELU, leaky-ReLU, PReLU, and SELU scalar programs for dense activations. |
| `training/src/inference.rs` | `operation_registry`, `lower_scalar`, `materialize_composition`, tree inference, KNN requirements/append, convolution and pooling preparation, and prepared parameters. |
| `training/src/compile.rs` | Binary metrics, K-means initialization/Lloyd graphs, tree inference, convolution and pooling preparation, and structured materialization. |
| `training/src/knn.rs` | Numeric/categorical `KnnOutputSpec` and independently masked target semantics. |
| `training/src/gguf_llama.rs` | Typed `PreparedParameter` values at the inference boundary. |
| `recipe-primitives` and planner | The `LoweredProgram` returned by `lower_primitive` and the validated graph emitted by materializers. |

The consumers reserve value and kernel ranges before asking `recipe-ops` to
materialize. They then insert only the returned graph contracts into their
larger static program. No consumer may bypass registry resolution by calling a
legacy kernel name, infer a missing ABI, or treat a status as runtime proof.

## Error and failure vocabulary

[`error.rs`](../src/error.rs) defines one `OperationError` with a machine
readable `OperationErrorKind`, detail string, and optional `OperationId`.
`Display` includes the operation ordinal when present. The kinds and their
meaning are:

| Kind | Boundary failure |
| --- | --- |
| `UnknownOperation`, `AmbiguousSymbol` | Registry lookup is absent or not source-unique. |
| `UnsupportedLowering`, `WrongLoweringKind` | The requested lowering API does not match the descriptor's owned form. |
| `PrimitiveRecipeMismatch`, `PrimitiveLoweringFailed` | Kernel parameters violate a direct recipe or backend-neutral lowering fails. |
| `InvalidScalarProgram` | Typed SSA construction or validation failed. |
| `InvalidCompositionRecipe` | A descriptor has empty metadata, an empty repeat body, an invalid bound, or excessive nesting. |
| `InvalidMaterializationRequest` | Boundary names, IDs, flags, storage contracts, ABI sets, aliases, or operation facts are invalid. |
| `MissingPreparedParameter`, `PreparedParameterTypeMismatch` | A required typed preparation value is absent or has the wrong variant. |
| `IterationBoundUnresolved`, `CompositionExpansionOverflow` | Shape/parameter repeat resolution failed or exceeded one million expanded steps. |
| `MissingConcreteFormula`, `UnsupportedConcreteShape` | A descriptive composition has no concrete ABI/formula or the supplied shape is outside its checked domain. |
| `IdentityNamespaceOverlap`, `IdentityNamespaceExhausted` | Boundary IDs overlap a reservation, two fragments overlap, or a reserved range is too small/overflows. |
| `WorkspaceLimitExceeded`, `WorkspaceFormulaMismatch`, `WorkspaceArithmeticOverflow` | Emitted intermediates exceed the caller limit, disagree with a static requirement, or overflow checked arithmetic. |
| `GraphMaterializationFailed` | A concrete dispatch is missing, stage families/counts disagree, or final graph validation fails. |

Errors are not converted into fallback implementations. The caller receives the
first observed contract failure with its source-qualified operation context and
must repair the declaration or preparation evidence.

## Extension rules

Adding an operation requires one source-qualified row in
`operation-surface.txt`, one explicit classification in the registry's owned
recipe tables, and the corresponding dtype, family, alias, determinism, and
definition contracts. A structured operation additionally needs a finite
`CompositionRecipe`, an exact concrete tensor ABI and prepared-parameter set,
owned scalar SSA, primitive parameters, identity/workspace accounting, and a
family `OPERATIONS` row plus dispatch branch. If any of those facts are not
available, leave the operation in `remaining_composition_manifest` with all
four `MissingConcreteComponent` values. Do not add a wrapper, CPU fallback,
duplicate implementation, retry, implicit ID scheme, or source-agnostic
symbol match.

The real validation path is structural and end to end through the production
entry point: `cargo check -p recipe-ops` verifies the generated inventory,
registry, and graph code, while `cargo check -p recipe-training` verifies the
consumer boundary. Compilation is only structural evidence; runtime
acceptance remains the responsibility of the training/inference workflows on
real data and hardware.
