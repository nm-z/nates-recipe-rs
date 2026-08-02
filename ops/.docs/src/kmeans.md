# K-means

K-means is a structured dense-model block. It reduces each input row to one
rooted L2 distance per centroid, so a block with `clusters = K` changes a
`[rows, dimensions]` feature matrix into a `[rows, K]` matrix. The centroids
are not optimizer parameters. They are recomputed from the complete training
partition on every training-loop iteration and are carried out of the run as
model state.

The implementation has two layers of ownership:

- `ops/src/kmeans.rs` owns the concrete initialization and Lloyd-transition
  graphs, their resource formulas, identity allocation, alias contracts, and
  validation.
- `training/src/compile.rs` owns the model-facing lifecycle. It places the two
  graphs into the static `init -> loop -> exit` program, admits optional resume
  centroids, computes the gradient that can flow through distances, and marks
  the updated centroids as an exit output.

Inference reuses the saved centroid image but does not run Lloyd updates. The
inference compiler emits the source-qualified `gpu_pairwise_l2` composition to
calculate distances from query rows to the saved centroids.

## Public declaration

The public facade accepts K-means through `Model::kmeans`:

```rust
recipe.model()
    .kmeans(4)
    .layer(8)
    .relu()
    .layer(1)
    .loss(bce);
```

`API.ogdl` lists `.kmeans(clusters)`. `src/api.rs` stores it as
`LayerSpec::KMeans { clusters, group_to_neuron }`. `Model::kmeans` validates
the nonzero cluster count immediately. A zero count is retained as a deferred
declaration error, so the eventual `Model::validate` call reports the invalid
layer instead of constructing a partial block.

If an ordinary `.layer(neurons)` immediately follows K-means, `Model::layer`
records `group_to_neuron = Some(...)` on the preceding K-means specification.
The recorded group count is exactly the K-means cluster count. `Model::validate`
requires that routed dense layer to remain the immediately following block and
to have exactly the declared neuron width. K-means itself can be used without
that routing declaration; the following dense layer then receives an ordinary
fully connected weight unless another block supplies a route.

`src/training.rs::map_dense_block` converts the facade specification to
`DenseBlock::KMeans(DenseKMeans)`. `DenseKMeans` stores two nonzero widths:

- `clusters`, the number of centroids and output columns;
- optional `group_to_neuron`, the width of an immediately following dense
  block.

`DenseBlock::output_width` returns the cluster count and
`DenseBlock::output_operations` is empty. K-means therefore emits a feature
matrix, not logits or an activation. `DenseKMeans::routing` resolves an exact
group-to-neuron relationship to one of `Identity`, `Expand`, `Contract`, or
`FullyConnected`:

- equal cluster and neuron widths use identity routing;
- a divisible larger destination uses expansion;
- a divisible smaller destination uses contraction;
- nondivisible widths use full connectivity.

## Direct operation boundary

`ops/src/lib.rs` publicly re-exports the concrete K-means operation API. The
functions are useful to the training compiler and to callers that already own
typed `recipe_language::Tensor` boundaries:

```text
kmeans_initialization_requirements
kmeans_lloyd_requirements
materialize_kmeans_initialization
materialize_kmeans_lloyd_step
```

The request and result types are:

| Type | Meaning |
| --- | --- |
| `KMeansInitializationRequest` | Point matrix, destination centroid matrix, caller-reserved `IdentityNamespace`, and workspace limit. |
| `KMeansInitializationMaterialization` | Validated `CalculationGraph`, emitted intermediate values and kernels, exact workspace bytes, and the preserved namespace. |
| `KMeansLloydRequest` | Point matrix, prior centroid matrix, updated-centroid output, distance output, reduction-tree lane count, namespace, and workspace limit. |
| `KMeansLloydMaterialization` | Validated graph and resource lists plus the `centroid_update_kernel` identity. |
| `KMeansRequirements` | Exact intermediate count, kernel count, and workspace byte count. |

The operation module is a direct Recipe-owned composition. It is not a public
operation-surface symbol and does not dispatch through the generic registry.
The separate legacy operation `gpu_pairwise_l2` is used by checkpoint-backed
inference, as described below.

## Common tensor and resource contracts

Both materializers first call `validate_f32_matrix` for every boundary. Each
boundary must be a valid, contiguous or otherwise valid rank-two `F32` tensor.
The tensor's layout and declared storage bytes are validated by
`Tensor::validate`; malformed layout or storage is reported as
`InvalidMaterializationRequest`.

`validate_dimensions(rows, dimensions, clusters)` then enforces:

- `rows` is in `1..=16_777_216` (`MAXIMUM_EXACT_F32_COUNT`);
- `dimensions` is nonzero;
- `clusters` is in `1..=i32::MAX`.

The row ceiling keeps index-derived values exactly representable in the
binary32 calculation domain and also keeps the `IndexMap` modulus in int32.
The cluster ceiling is required for int32 assignment and gather indices.

All products and sums used for extents and workspace are checked `u64`
arithmetic. An overflow is `OperationErrorKind::WorkspaceArithmeticOverflow`.
`Shape::new` and scalar-program construction failures are wrapped as
`GraphMaterializationFailed`.

Every request supplies a caller-owned `IdentityNamespace`. Its half-open value
and kernel ranges are reserved before emission. The emitter rejects a boundary
value that falls inside the intermediate-value range, rejects repeated boundary
identities, and fails when either range ends overflow `u64`. Values and kernels
are allocated monotonically inside those ranges. The resulting graph is
validated before it leaves the operation module, so every intermediate has a
producer, every kernel ID is unique, and the graph is acyclic.

`validate_resources` checks the requirement counts against the namespace
capacities and the byte formula against the supplied limit. The corresponding
errors are `IdentityNamespaceExhausted` and `WorkspaceLimitExceeded`.

The emitter's default alias matrix marks every input/output pair as
`AliasPermission::Forbidden`. K-means output IDs must therefore be distinct
from all input IDs and from each other. The centroid-update kernel is the one
intentional exception: its prior-centroid input has
`AliasPermission::MustAliasExact` with the updated-centroid output, allowing a
backend to reuse that storage while preserving distinct logical tensor IDs.

## Deterministic initialization

`kmeans_initialization_requirements(clusters)` returns:

| Resource | Requirement |
| --- | ---: |
| Intermediates | `1` |
| Kernels | `2` |
| Workspace | `4 * clusters` bytes |

`materialize_kmeans_initialization` requires a points tensor shaped
`[rows, dimensions]` and a centroid tensor shaped `[clusters, dimensions]`.
The second extent must match exactly. It emits this graph:

1. An `I32` `IndexMap` creates `[clusters]` row indices with
   `start = 0`, `element_step = 1`, `iteration_step = 0`, and
   `modulus = rows`.
2. An axis-zero `Gather` reads `points` at those indices and writes the
   external `centroids` tensor. Bounds are `IndexBounds::Reject`.

Consequently centroid `c` starts from training row `c mod rows`. The first
`min(clusters, rows)` centroids preserve source-row order. When `clusters >
rows`, rows are cycled deterministically. There is no random seed, host-side
sampling, or convergence test in initialization.

The initialization emitter allocates one `I32` index tensor, accounts for its
four bytes per element, emits exactly two kernels, checks those counts against
`KMeansRequirements`, and calls `CalculationGraph::validate` before returning.

## One Lloyd transition

`kmeans_lloyd_requirements(rows, dimensions, clusters)` returns a fixed graph
shape and a closed workspace formula:

| Resource | Requirement |
| --- | ---: |
| Intermediates | `16` |
| Kernels | `18` |
| Workspace elements | `3 * rows * dimensions` + `4 * clusters * dimensions` + `2 * rows` + `3 * clusters` + `4 * rows * clusters` |
| Workspace bytes | `4 * workspace elements` |

The request boundaries are:

- `points`: `F32 [rows, dimensions]`, input;
- `prior_centroids`: `F32 [clusters, dimensions]`, input;
- `updated_centroids`: `F32 [clusters, dimensions]`, output;
- `distances`: `F32 [rows, clusters]`, output.

The prior and updated centroid widths must match `dimensions`, and the output
shapes must match the request exactly. The two output identities must not equal
one another or any input identity. `tree_lanes` must be a power of two in
`1..=1024`. This request-specific check is in addition to the fixed-tree
validation performed by the language and primitive lowerers.

The graph is emitted in this order. The ordering is part of the dependency
graph, not a host loop:

1. Square `points` into `point_squared [rows, dimensions]`.
2. Square `prior_centroids` into `centroid_squared [clusters, dimensions]`.
3. Reduce `point_squared` over axis 1 with `Sum`, `keep_dimensions = true`,
   producing `point_norms [rows, 1]`.
4. Reduce `centroid_squared` over axis 1 with `Sum`,
   `keep_dimensions = false`, producing `centroid_norms [clusters]`.
5. Contract axes `(1, 1)` of points and prior centroids, producing
   `products [rows, clusters]`.
6. Apply the owned pairwise-L2 scalar program to
   `point_norms`, `centroid_norms`, and `products`, producing prior distances
   `sqrt(max(0, ||x||² + ||c||² - 2 * dot(x,c)))`.
7. Reduce those prior distances over axis 1 with `Minimum`,
   `keep_dimensions = true`, and `ReduceResult::Index`, producing
   `assignments I32 [rows, 1]`.
8. Create `cluster_indices I32 [clusters]` with an identity `IndexMap`.
9. Compare every assignment with every cluster index and convert the result
   from `I32` to `F32`, producing `memberships [rows, clusters]`.
10. Create an all-zero `I32` `one_seed [rows, dimensions]` with an `IndexMap`.
11. Map that seed to constant `F32` one values, producing `ones`.
12. Contract `memberships` with `points` over axis pair `(0, 0)`, producing
    `sums [clusters, dimensions]`.
13. Contract `memberships` with `ones` over axis pair `(0, 0)`, producing
    `counts [clusters, dimensions]`. The same row count is therefore available
    for each feature of a cluster.
14. Update each centroid with the dedicated scalar program and the explicit
    alias matrix. For each feature, `count > 0` selects
    `sum / max(count, 1)`. If `count == 0`, the prior centroid value is
    selected instead.
15. Square `updated_centroids`, producing `updated_squared`.
16. Reduce `updated_squared` over axis 1 with `Sum`, producing
    `updated_norms [clusters]`.
17. Contract `points` with `updated_centroids` over axes `(1, 1)`, producing
    `updated_products [rows, clusters]`.
18. Apply the same pairwise-L2 scalar program to produce the external
    `distances [rows, clusters]` against the updated centroids.

The fixed reduction lowerer uses a power-of-two tree and the canonical
`LowestLogicalIndex` tie rule. Equal prior distances therefore select the
lowest cluster index. The output distances are recomputed after the centroid
update, so they are not the distances used to choose assignments in that same
transition.

The centroid update is an ordinary elementwise primitive with three inputs:
`sums`, `counts`, and `prior_centroids`. Its program is equivalent to:

```text
populated = count > 0
safe_count = max(count, 1)
mean = sum / safe_count
updated = select(populated, mean, prior)
```

This is the empty-cluster contract. No divide-by-zero is attempted and no
fallback centroid is synthesized.

The 16 intermediate tensors account for the formula exactly:

- three `rows * dimensions` tensors: point squares, the all-zero seed, and
  ones;
- four `clusters * dimensions` tensors: centroid squares, sums, counts, and
  updated squares;
- two `rows` tensors: point norms and assignment indices;
- three `clusters` tensors: centroid norms, cluster indices, and updated norms;
- four `rows * clusters` tensors: products, prior distances, memberships, and
  updated products.

## Training compiler integration

`training/src/compile.rs::compile_dense_training_impl` compiles the complete
prepared training partition as one logical matrix. The compiler's documented
epoch contract is one full partition and one optimizer update per epoch; a
backend may tile the matrix physically without changing that semantic shape.

When `compile_training_blocks` reaches `DenseBlock::KMeans`, it calls
`compile_training_kmeans` with:

- the current feature value;
- `partition_rows`, the full prepared training row count;
- the current logical feature width;
- the block declaration and index;
- `config.reduction_tree_lanes`.

`compile_training_kmeans` performs the following allocation sequence:

1. It requires the current value to be `F32 [partition_rows, input_width]` and
   creates `fresh_centroids F32 [clusters, input_width]`.
2. It reserves exactly the initialization requirement counts from the
   compiler's monotonically increasing value and kernel IDs, then inserts the
   initialization graph with `IterationDomain::first()`. Initialization is
   therefore admitted once before the loop.
3. It creates an external-zero `ResumeKMeansCentroids` input and selects
   between fresh and resumed centroids with the compiler's `resume_enabled`
   scalar. A new run leaves that scalar at zero. `apply_checkpoint_resume`
   replaces the input bytes with saved centroid bytes and sets the scalar to
   one.
4. It creates `updated_centroids` and `distances`, reserves the Lloyd
   requirement counts, and inserts the Lloyd graph with
   `self.training_domain`, which is every training-loop iteration.
5. It marks `updated_centroids` as a compiler external output and stores a
   `DenseKMeansState` containing `input_width`, `clusters`,
   `initial_centroids`, `updated_centroids`, and the centroid update kernel
   identity.
6. It returns `distances` as the current block value, changes the logical
   width to `clusters`, and carries the optional K-means routing to the next
   ordinary dense layer.

The initialization and Lloyd fragments use distinct reserved identity ranges.
The compiler then inserts both graphs into one `CalculationGraph`, assigns
their domains, canonicalizes the graph through OGDL, and reconstructs the
`StaticCalculationProgram`. A resulting program has no host callback or
per-epoch graph rebuild.

There is no K-means optimizer parameter. `BlockGradients::KMeans` is excluded
from `flatten_parameter_gradients`, and `update_blocks` returns the
`DenseKMeansState` unchanged. Centroids move only through the Lloyd transition
described above.

## Backward behavior

K-means participates in the dense backward walk when a later block needs an
input gradient. `backward_kmeans` does not differentiate the assignment
indices, memberships, or centroid update. It differentiates the emitted
distance image with centroids treated as the current fixed values:

```text
scaled = upstream_distance_gradient / max(distance, epsilon)
row_scales = sum_over_clusters(scaled)
input_term = points * row_scales
centroid_term = contraction(scaled, updated_centroids)
input_gradient = input_term - centroid_term
```

The result is masked by the training validity image before it is passed to an
earlier block. `epsilon` is the configured dense normalization epsilon. If
K-means is the first block, the backward walk does not request an input
gradient, because the external feature image has no trainable predecessor.
In every case K-means contributes no parameter gradient and no AdamW update.

## Group-to-neuron routing

After a K-means block, `preceding_routing` carries the resolved group route and
one scalar channel. When the immediately following dense layer is compiled,
`group_routing_mask` builds an `F32 [clusters, neurons]` mask at
`IterationDomain::first()` and multiplies it into the initialized weight.
The mask is:

- diagonal for `Identity`;
- contiguous repeated destination groups for `Expand`;
- contiguous grouped source rows for `Contract`;
- all ones for `FullyConnected`.

The dense forward and backward graphs then use the masked weight. The route is
not a separate model operation and has no host-side state. Checkpoint
validation additionally verifies that disallowed entries in the routed dense
parameter, first-moment, and second-moment tensors are exact `+0.0` values.

## Checkpoint and resume state

`DenseKMeansState` is deliberately smaller than an optimizer parameter state.
It retains the logical input width and cluster count, the selected initial
centroid value, the updated centroid value, and the update-kernel identity.
`TrainingOutputs::visit_parameter_states` skips K-means, so centroids do not
acquire AdamW first- or second-moment images.

`CheckpointManifest::from_compiled` converts a K-means block with
`checkpoint_kmeans`:

- the declaration cluster count must equal the state cluster count;
- the saved `input-width` is the state input width;
- the saved `centroids` tensor is `state.updated_centroids`;
- the optional `group-to-neuron` width is retained.

After native exit, `CompletedTrainingCheckpoint` maps the external output
bytes into this centroid image and writes the semantic checkpoint. A semantic
`.ogdl` model contains the centroid tensor and declaration metadata. A native
`.cubin` or `.hsaco` companion remains a separate optional artifact.

Resume is existence-conditional at the public `Train::run` boundary. If the
declared model path is missing, the compiler leaves `resume_enabled = 0` and
the deterministic fresh initialization is used. If the path exists,
`apply_checkpoint_resume` validates the saved and current topology, input
width, cluster count, centroid dtype, shape, and byte count. It requires exactly
one `ResumeKMeansCentroids { block }` input for each saved K-means block, copies
the saved centroid bytes into that input, and sets the one `ResumeEnabled`
scalar to `1`. Missing, duplicated, unmatched, or shape-incompatible centroid
inputs fail with `CheckpointError::IncompatibleResume`; they are not silently
reinitialized.

## Checkpoint-backed inference

The public target-free path is `recipe.model().load("model.ogdl")` followed by
`recipe.infer()`. `training/src/inference.rs::compile_prepared_inference`
walks the decoded `CheckpointBlockImage` sequence and keeps a current logical
feature width.

For `CheckpointBlockImage::KMeans`, `compile_kmeans`:

1. checks that the saved `input-width` equals the preceding logical width;
2. requires the current rows to be `F32 [rows, input_width]`;
3. validates the saved centroid image as `F32 [clusters, input_width]`;
4. admits those bytes once as `InferenceInputRole::KMeansCentroids`;
5. allocates `distances F32 [rows, clusters]`;
6. resolves the unique `gpu_pairwise_l2` descriptor and materializes it with
   `queries = rows`, `training_rows = clusters`, `dimensions = input_width`,
   and the saved reduction-tree lane count;
7. returns the distance image and sets the current width to `clusters`.

The generic operation is the source-qualified row
`gpu_pairwise_l2    gpu-core/src/kernels.rs:5239` in
`operation-surface.txt`. `ops/src/materialize/graph_cluster_rl.rs` enforces
the exact ABI: f32 `query`, `training`, and `distances` tensors with shapes
`[queries, dimensions]`, `[training_rows, dimensions]`, and
`[queries, training_rows]`, plus exactly the four typed preparation
parameters. Its graph squares both inputs, reduces row norms in fixed trees,
contracts query and centroid rows, and applies

```text
sqrt(max(0, query_norm + training_norm - 2 * dot(query, training)))
```

Inference therefore exposes distances from the saved centroids but never
reassigns rows or changes centroid state. Following blocks consume the
`[rows, clusters]` image; a final K-means block must still produce the task's
declared output width.

Checkpoint decode and inference preparation reject an input-width mismatch,
invalid centroid dtype or shape, malformed external bytes, unknown operation,
ambiguous operation symbol, or any graph/materialization contract failure.
Native discovery, artifact realization, allocation, and the actual inference
`init -> loop -> exit` run happen only after this static compilation succeeds.

## Failure and invariant summary

The operation layer reports typed `OperationErrorKind` values rather than
silently changing algorithms:

| Condition | Result |
| --- | --- |
| Zero or too-large rows, zero dimensions, or zero/too-large clusters | `UnsupportedConcreteShape` |
| Non-f32, non-rank-two, malformed, duplicate, or output/input-conflicting boundary tensors | `InvalidMaterializationRequest` |
| Invalid power-of-two Lloyd tree lanes | `InvalidMaterializationRequest` |
| Namespace range overflow or insufficient value/kernel capacity | `IdentityNamespaceExhausted` |
| Boundary value inside a reserved intermediate range | `IdentityNamespaceOverlap` |
| Workspace product/sum or tensor storage arithmetic overflow | `WorkspaceArithmeticOverflow` |
| Required workspace exceeds the caller's limit | `WorkspaceLimitExceeded` |
| Emitter counts do not match the closed formulas | `GraphMaterializationFailed` or `WorkspaceFormulaMismatch` |
| Language graph, shape, alias, or scalar-program validation failure | `GraphMaterializationFailed` |

Training adds its own typed boundaries: an invalid facade declaration is a
`DeclarationError`; invalid routing or network width is a
`TrainingCompileError`; incompatible saved centroid state is a
`CheckpointError`; and unavailable native hardware or a failed finalized run
is a native preparation or execution error. None of these paths substitutes a
different centroid algorithm, adds a convergence fallback, or hides a broken
graph transition.

## End-to-end role

For a public training declaration, the complete path is:

```text
recipe.model().kmeans(K)
  -> LayerSpec::KMeans
  -> DenseKMeans
  -> compile_training_kmeans
  -> deterministic init graph (first iteration only)
  -> Lloyd graph (every full-partition training iteration)
  -> distance loss and optional input backward graph
  -> static CalculationGraph / StaticCalculationProgram
  -> native preparation and finalized init -> loop -> exit execution
  -> updated centroid exit image
  -> semantic checkpoint centroids and optional native artifact
```

For a loaded model, the path is:

```text
model.ogdl centroid image
  -> CheckpointKMeansImage
  -> compile_kmeans
  -> gpu_pairwise_l2 distance graph
  -> subsequent dense blocks or task output
  -> native inference lifecycle
```

The observable model contract is thus deterministic initialization, fixed-tree
Lloyd assignment and mean update, explicit empty-cluster retention, rooted
distance output, optional sparse group routing, and persistence of only the
latest centroid image. There is no host-side K-means loop and no separate
assignment output in the public artifact.
