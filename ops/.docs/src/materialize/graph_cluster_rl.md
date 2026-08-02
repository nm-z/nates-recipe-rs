<!--
Intent: document the complete graph, clustering, and reinforcement-learning
materialization family. This is a source trace of the exact registry entries,
composition recipes, dispatcher ownership, tensor ABIs, scalar programs, and
the bounded calculation graphs emitted by recipe-ops. The legacy gpu-core
paths named by the registry are inventory provenance, not an additional runtime
implementation in this repository.
-->

# Graph, clustering, and reinforcement-learning materialization

This module is [`ops/src/materialize/graph_cluster_rl.rs`](../../../src/materialize/graph_cluster_rl.rs).
It is the concrete materializer for sixteen source-qualified legacy operation
rows covering graph utilities, sparse graph contractions, clustering, and
reinforcement-learning (RL) calculations. It turns a validated
`MaterializationRequest` into a finite `recipe_language::CalculationGraph`
fragment. The fragment contains only typed tensors and calculation primitives.
It does not discover hardware, allocate device storage, load a native image,
execute a kernel, or run a host payload loop. Those responsibilities belong to
preparation, planning, and the native executors.

The source strings in the table below preserve the operation-surface inventory
(`gpu-core/...`). No `gpu-core` crate is present in this checkout. The strings
are stable source-qualified identities used by `OperationRegistry`; they are
not filesystem imports performed by the materializer.

## Family boundary and complete owned set

`OPERATIONS` is a closed list of `(symbol, source)` pairs
([`graph_cluster_rl.rs:17-34`](../../../src/materialize/graph_cluster_rl.rs:17)).
`supports` compares both fields, so a same-named row from a different legacy
source is not accepted by this family. The registry's composition lookup is
symbol based, but concrete ownership is source qualified. This distinction is
what keeps a descriptor visible in the registry while preventing it from
silently using a semantically adjacent implementation.

| Symbol | Legacy source | Registry recipe and payload | Concrete emitter |
| --- | --- | --- | --- |
| `gpu_boruvka_mst` | `gpu-core/src/cluster.rs:395` | `boruvka_minimum_spanning_tree`, `BORUVKA_STEPS`, F32 plus I32, `Clustering` | `emit_boruvka` |
| `gpu_categorical_logprob` | `gpu-core/src/rl.rs:131` | `categorical_log_probability`, `GATHER_MAP`, F32 plus I32, `ReinforcementLearning` | `emit_categorical_log_probability` |
| `gpu_centroid_update` | `gpu-core/src/kernels.rs:4685` | `centroid_update`, `GATHER_MAP_SCATTER`, F32 plus I32, `Clustering` | `emit_centroid_update` |
| `gpu_core_distance` | `gpu-core/src/cluster.rs:476` | `core_distance`, `MAP_SORT_GATHER`, F32 plus I32, `Clustering` | `emit_core_distance` |
| `gpu_csr_spmm` | `gpu-core/src/graph.rs:82` | `csr_sparse_contraction`, `GATHER_MAP_SCATTER`, F32 plus I32, `Graph` | `emit_csr_sparse_matrix` |
| `gpu_csr_spmv` | `gpu-core/src/graph.rs:56` | `csr_sparse_contraction`, `GATHER_MAP_SCATTER`, F32 plus I32, `Graph` | `emit_csr_sparse_vector` |
| `gpu_degree` | `gpu-core/src/graph.rs:158` | `graph_degree`, `MAP_HISTOGRAM`, I32, `Graph` | `emit_degree` |
| `gpu_discounted_returns` | `gpu-core/src/rl.rs:57` | `discounted_returns`, `MAP_SCAN`, F32, `ReinforcementLearning` | `emit_discounted_returns` |
| `gpu_fixed_radius_neighbors` | `gpu-core/src/cluster.rs:179` | `fixed_radius_neighbors`, `SCAN_MAP_SCATTER`, F32 plus I32, `Clustering` | `emit_fixed_radius_singleton` |
| `gpu_gae` | `gpu-core/src/rl.rs:79` | `generalized_advantage_estimation`, `MAP_SCAN`, F32, `ReinforcementLearning` | `emit_generalized_advantage` |
| `gpu_gaussian_logprob` | `gpu-core/src/rl.rs:156` | `gaussian_log_probability`, `MAP_REDUCE`, F32, `Bayesian` recipe family and RL dispatch | `emit_gaussian_log_probability` |
| `gpu_gcn_norm` | `gpu-core/src/graph.rs:180` | `gcn_edge_normalization`, `GATHER_MAP`, F32 plus I32, `Graph` | `emit_gcn_normalization` |
| `gpu_neighbor_aggregate` | `gpu-core/src/graph.rs:111` | `graph_neighbor_aggregation`, `GATHER_MAP_SCATTER`, F32 plus I32, `Graph` | `emit_neighbor_aggregate` |
| `gpu_pairwise_l2` | `gpu-core/src/kernels.rs:5239` | `pairwise_l2_distance`, `PAIRWISE_L2_STEPS`, F32, `Distance` | `emit_pairwise_l2` |
| `gpu_td_targets` | `gpu-core/src/rl.rs:105` | `temporal_difference_targets`, `GATHER_MAP`, F32 plus I32, `ReinforcementLearning` | `emit_temporal_difference_targets` |
| `gpu_union_find_cc` | `gpu-core/src/cluster.rs:251` | `union_find_connected_components`, `UNION_FIND_STEPS`, I32, `Clustering` | `emit_union_find_two_nodes` |

The recipe entries are in [`ops/src/composition.rs:1192-2585`](../../../src/composition.rs:1192).
The source rows are in [`operation-surface.txt:64-409`](../../../../operation-surface.txt:64).
The two operation tables deliberately have different matching rules: the
composition registry maps `gpu_gaussian_ll` and `gpu_gaussian_logprob` to one
descriptive recipe, while this concrete family owns only the exact
`gpu_gaussian_logprob` RL source row. `gpu_gaussian_ll` therefore remains a
composition descriptor without a concrete family materializer and appears in
`remaining_composition_manifest`.

## Public and internal call path

The public facade in [`src/facade.rs:51-124`](../../../src/facade.rs:51) exposes
the registry, exact resolution, composition validation, materialization, and
the remaining-composition manifest through `recipe::operations`. The
crate-level exports in [`ops/src/lib.rs:41-70`](../../../src/lib.rs:41) expose
the same typed boundary for Rust callers.

The common path is:

```text
operation-surface.txt
        |
        v
OperationRegistry::iter / resolve_exact / resolve_unique
        |
        v
CompositionRecipe::for_entry -> validate -> expand_composition
        |
        v
materialize_composition(MaterializationRequest)
        |
        +-- exact request and tensor validation
        +-- graph-cluster-RL dispatch in family order
        +-- emitter.intermediate / emitter.emit
        +-- CalculationGraph::validate
        |
        v
MaterializedComposition { graph, resolved steps, stages, workspace }
        |
        v
CalculationGraph::assemble -> program/planner -> native executor
```

`materialize_composition` first checks that the descriptor is a
`LoweringAvailability::Composition`, verifies the request, rejects a
descriptor with no concrete family owner, resolves the named iteration-shape
input, expands its finite recipe, creates one `Emitter`, invokes
`dispatch_concrete`, and finishes the graph
([`ops/src/materialize.rs:413-480`](../../../src/materialize.rs:413)).
Family dispatch order is optimizer/normalization, solver/FFT, attention/
sequence/embedding, convolution/pooling, loss/metrics, indexing/sort/encoding,
this module, tree/boosting, inference/quantization/diffusion, creation/shape,
and training. Each family returns `NotOwned` or `Owned(result)`; the first
owned family returns immediately. A family that claims a descriptor but cannot
match the symbol produces its own graph-materialization error rather than
falling through.

This module's `dispatch` first calls `supports`. An unsupported exact pair
returns `FamilyDispatch::NotOwned`. A supported pair then selects exactly one
emitter by symbol. The default arm is an explicit
`GraphMaterializationFailed` error saying that graph/clustering/RL dispatch is
incomplete ([`graph_cluster_rl.rs:36-72`](../../../src/materialize/graph_cluster_rl.rs:36)).
There is no alternate implementation or legacy callback path.

### Current higher-level callers

The public boundary is usable by any preparation caller that can provide the
declared tensor ABI and prepared facts. In the current workspace, the concrete
end-to-end callers are the K-means validation paths:

* `training/src/compile.rs:7448-7500` (`compile_validation_kmeans`) validates
  the input and updated-centroid contracts, prepares `queries`,
  `training_rows`, `dimensions`, and `tree_lanes`, then calls
  `self.materialize("gpu_pairwise_l2", ...)`.
* `training/src/inference.rs:3729-3780` (`compile_kmeans`) performs the same
  pairwise distance materialization against checkpoint centroids during
  inference. The checkpoint block is reached from the inference block
  compiler at `training/src/inference.rs:1080-1089`.

Both compilers create named tensors, mark inputs and outputs with the correct
external flags, reserve 64 value IDs and 64 kernel IDs, and pass an unlimited
workspace limit (`ByteCount::new(u64::MAX)`). Their `materialize` helper then
resolves the symbol, calls `recipe_ops::materialize_composition`, merges every
returned tensor contract, appends every returned node, and records a first
iteration domain ([`training/src/inference.rs:2008-2077`](../../../../training/src/inference.rs:2008)).
The training compiler has the same helper and reservation policy at
`training/src/compile.rs:10960-11000`.

No other graph, clustering, or RL symbol in this family is called by a
higher-level compiler in this checkout. That is an observation about current
callers, not a reduction of the public operation contract. Direct users still
enter through `recipe::operations::materialize` and must satisfy the exact ABI
below.

## Shared preparation and graph invariants

### Exact ABI matching

Every emitter begins with `require_exact_abi` from
[`ops/src/materialize.rs:4221-4265`](../../../src/materialize.rs:4221). It compares
sets, not order, for input names, output names, and prepared-parameter names.
An extra name, a missing name, or a duplicate declaration is an
`InvalidMaterializationRequest`. The request-level validator
([`materialize.rs:4268-4331`](../../../src/materialize.rs:4268)) additionally
requires a structured descriptor, at least one input and output, unique
nonempty declaration names, unique value IDs, valid tensor layouts and storage
spans, external-input flags on inputs, and non-input external-output flags on
outputs. `input` and `output` then perform name-addressed lookup and report a
missing declaration instead of guessing a positional argument.

All prepared numeric dimensions pass through `prepared_dimension`: they must
be `PreparedParameter::U64`, nonzero, and no larger than
`MAX_I32_INDEX = 2_147_483_647`. Counts such as `edges` may be zero but still
must fit that int32 domain. Products use checked `u64` multiplication and then
the same canonical int32 bound. Overflow is
`WorkspaceArithmeticOverflow`; a product that is representable in `u64` but
outside the index domain is `UnsupportedConcreteShape`. `PreparedParameter`
type mismatches and absent keys are `PreparedParameterTypeMismatch` and
`MissingPreparedParameter` respectively. F32 parameters are decoded from
`F32Bits` and must be finite when read by `finite_parameter`; booleans are
decoded only from `Bool`.

`tree_lanes` is a prepared U64 converted to `u32` and must be a power of two in
`1..=1024` ([`materialize.rs:4498-4508`](../../../src/materialize.rs:4498)). The
lane count fixes reduction and scan trees and therefore floating-point order.
All indexed primitives in this family use `axis: 0` and
`IndexBounds::Reject`. Reject means an out-of-range int32 index reports a device
fault through the preallocated fault channel; it is never clamped or wrapped
([`language/src/primitive.rs:26-33`](../../../../language/src/primitive.rs:26)).

### Identity, workspace, and alias state

`GraphBuilder::new` copies the caller boundary tensors, checks checked range
ends for the `IdentityNamespace`, and rejects any boundary value ID inside the
reserved intermediate range. Each `intermediate` allocation consumes one
value ID, creates a contiguous internal tensor, adds its storage bytes to the
workspace total, and enforces the caller's workspace limit. Each `emit`
consumes one kernel ID. Range exhaustion is `IdentityNamespaceExhausted`;
end arithmetic overflow is the same operation error kind with an explanatory
detail; byte-total overflow is `WorkspaceArithmeticOverflow`; a limit breach is
`WorkspaceLimitExceeded` ([`materialize.rs:712-846`](../../../src/materialize.rs:712)).

Every emitted kernel receives a complete input/output alias matrix with
`AliasPermission::Forbidden`. Inputs therefore remain immutable boundary or
intermediate values. `CalculationGraph::validate` checks tensor contracts,
kernel uniqueness, producer uniqueness, external-input and producer
consistency, primitive shape and dtype rules, and topological acyclicity
([`language/src/graph.rs:78-137`](../../../../language/src/graph.rs:78)). A failed
language check is wrapped as `GraphMaterializationFailed`.

The `Emitter` cursor is the bridge between the descriptive recipe and the
concrete graph. `emit_stage` requires one resolved step, rejects an empty
kernel sequence, compares the primary (last) kernel's `PrimitiveFamily` with
the resolved step family, records every kernel ID in a `StageEmission`, and
advances the cursor. It is valid for one descriptive step to contain several
same-stage kernels. `finish` rejects both over-emission and under-emission
before graph validation ([`materialize.rs:623-710`](../../../src/materialize.rs:623)).

### Finite recipe expansion

`CompositionRecipe` is a static description, not executable tensor semantics.
The shared recipes are defined in [`ops/src/composition.rs:224-240`](../../../src/composition.rs:224).
The graph family uses these shapes:

* `GATHER_MAP_SCATTER`: three resolved steps, with a gather stage, one map
  stage, and a final scatter stage. The CSR, neighbor aggregation, and
  centroid emitters use several same-stage kernels where the recipe has one
  family step.
* `GATHER_MAP`: one gather step followed by one map step. GCN normalization,
  temporal-difference targets, and categorical log probability use this shape.
* `MAP_HISTOGRAM`: an identity map followed by a histogram for graph degree.
* `MAP_SCAN`: a map followed by a reverse or forward scan for returns and GAE.
* `MAP_REDUCE`: map followed by fixed-tree reduction for Gaussian log
  probability.
* `MAP_SORT_GATHER`: map, stable sort, and checked gather for core distance.
* `PAIRWISE_L2_STEPS`: map, reduce, contraction, and map for pairwise L2.
* `BORUVKA_STEPS` and `UNION_FIND_STEPS`: a
  `CeilingLog2ShapeExtent { axis: 0 }` repeat whose body is gather, then
  reduce or elementwise, then scatter. Both concrete emitters currently admit
  exactly two nodes, so the resolved repeat count is one and the graph has
  three stages.

`expand_composition` validates the recipe, resolves shape and prepared bounds,
records every bound and iteration, and appends a `ResolvedStep` for each
primitive. It uses one linear dependency to the immediately preceding step;
actual tensor producers are checked later by `CalculationGraph`. The fixed
one-million-step cap produces `CompositionExpansionOverflow`, an unresolved
axis or missing prepared bound produces `IterationBoundUnresolved` or the
underlying prepared-parameter error ([`materialize.rs:385-411`](../../../src/materialize.rs:385)).

## Concrete operation contracts

The following sections describe each emitter's exact names, shapes, typed
parameters, graph nodes, scalar guards, and concrete domain. Shapes shown are
logical tensor extents. All tensors are validated `recipe_language::Tensor`
objects, and all internal tensors are contiguous row-major F32 or I32 values.

### Graph degree: `gpu_degree`

**Boundary.** Input `edge_destinations` is I32 `[edges]`; output `degrees` is
I32 `[nodes]`. Parameters are `nodes: U64`, `edges: U64`, and the true fact
`endpoint_indices_verified`. `nodes` is nonzero and int32-bounded; `edges` may
be zero. The verification fact binds every endpoint to the graph's node range.

**Graph.** The emitter allocates one I32 `[edges]` intermediate, copies the
endpoints through an identity `Elementwise` node, converts `nodes` to `u32`
bins, then emits `Histogram { bins, weighted: false, ordering: Relaxed }` into
`degrees`. The two recipe steps are `MAP_HISTOGRAM`; the map is not an
alternate degree algorithm, it is the explicit stage that preserves the
recipe's typed elementwise family before histogram accumulation.

**Failures.** Exact-name, dtype, shape, parameter type, false verification
fact, missing parameter, int32 extent, `u32` conversion, identity, workspace,
primitive, and graph validation failures use the shared errors. There is no
host-side endpoint scan or recovery branch.

### CSR sparse vector and matrix: `gpu_csr_spmv` and `gpu_csr_spmm`

These are a deliberate paired case. Their public symbols and source rows stay
distinct, but both dispatch to `emit_csr_padded` after resolving their operand
geometry. `gpu_csr_spmv` takes `vector` and uses `operand_elements = columns`
and `output_elements = rows`. `gpu_csr_spmm` takes `matrix`, adds
`features`, and uses `operand_elements = columns * features` and
`output_elements = rows * features`. The matrix products are checked before
the common emitter is entered.

**Boundary.** Both forms require inputs
`values`, the form-specific `vector` or `matrix`, `value_indices`,
`operand_indices`, `active_mask`, `output_base`, and `output_indices`, plus
output `product`. `values` and the dense operand are F32. The three table
indices and `active_mask` are I32 `[output_elements, row_width]`.
`output_base` is F32 `[output_elements]`, `output_indices` is I32
`[output_elements]`, and `product` is F32 `[output_elements]`. Parameters are
`rows`, `columns`, optional `features`, `nonzeros`, `row_width`,
`tables_verified`, `output_base_zero`, `output_indices_unique`, and
`tree_lanes`. Every table fact must be true. `values` has shape `[nonzeros]`;
the operand and all output tables use the derived extents.

**Graph.** Step 0 emits two checked axis-zero gathers, one from `values` by
`value_indices`, one from the dense operand by `operand_indices`. Step 1 maps
the gathered values with `masked_product_program`: it validates each mask as
exactly I32 zero or one, multiplies the pair, and selects zero for inactive
lanes. Step 2 reduces the row width with a fixed sum tree and scatters each row
sum through `output_base` and `output_indices` using `UniqueIndices`. The base
must be zero and destinations must be unique because the output is a direct
write, not an atomic accumulation. The common graph therefore implements
bounded padded CSR rows without a data-dependent loop.

**Failures.** A false table fact, shape mismatch, non-F32 payload, non-I32
index, zero or oversized dimension, product overflow, non-power-of-two lane
count, duplicate destination claim, out-of-range runtime table index, identity
exhaustion, workspace breach, or graph validation failure is visible. The
materializer does not infer CSR row pointers, sort rows, or silently pad a
missing table.

### Degree normalization: `gpu_gcn_norm`

**Boundary.** Inputs are F32 `features` `[nodes * features_per_node]`, F32
`degrees` `[nodes]`, and I32 `degree_indices` `[nodes * features_per_node]`.
Output `normalized` is F32 of the flattened feature shape. Parameters are
`nodes`, `features_per_node`, and true `degree_indices_verified`.

**Graph and formula.** A checked gather reads each degree using
`degree_indices`. The map then validates `degree >= 0`; for positive degree it
computes `feature / sqrt(degree)`, and for zero degree selects the original
feature. The zero-degree branch prevents division by zero while preserving an
isolated node's feature. The recipe is `GATHER_MAP` and the graph has exactly
one gathered F32 intermediate and one final elementwise output.

### Neighbor aggregation: `gpu_neighbor_aggregate`

**Boundary.** Inputs are flattened F32 `features` `[nodes * features_per_node]`,
F32 `degrees` `[nodes]`, I32 `feature_indices` and `active_mask`
`[nodes * features_per_node, row_width]`, I32 `degree_indices` and
`output_indices` `[nodes * features_per_node]`, and F32 `output_base` of that
same one-dimensional shape. Output `aggregate` is F32 of the same shape.
Parameters are `nodes`, `features_per_node`, `row_width`, `mean: Bool`, the true
facts `tables_verified`, `output_base_zero`, `output_indices_unique`, and
`tree_lanes`.

**Graph.** Step 0 gathers one degree per destination and the padded neighbor
feature table. Step 1 masks inactive feature entries to zero. Step 2 reduces
the row width, normalizes if `mean` is true, and scatters through unique output
indices. Mean mode requires a nonnegative degree and divides only when it is
positive; zero-degree destinations retain the sum (zero when all lanes are
inactive). Sum mode bypasses the divide. `IndexBounds::Reject` remains active
for every table read and write.

### Centroid update: `gpu_centroid_update`

**Boundary.** Inputs are F32 `points` `[points_count * dimensions]`, I32
`point_indices` and `active_mask` `[centroid_count * dimensions,
points_per_centroid]`, I32 `assignments` `[points_count]`, I32
`centroid_count_indices` and `centroid_indices` `[centroid_count * dimensions]`,
and F32 `centroid_base` of that same flattened centroid shape. Outputs are F32
`centroids` `[centroid_count * dimensions]` and I32 `counts` `[centroid_count]`.
Parameters are `points_count`, `dimensions`, `centroid_count`,
`points_per_centroid`, `contribution_table_verified`,
`centroid_count_indices_verified`, `centroid_base_zero`,
`centroid_indices_unique`, `assignments_verified`, and `tree_lanes`. Every
verification fact must be true.

**Graph.** A checked gather reconstructs the padded point contribution table.
The mask map selects each active point value or zero. The final stage contains
five same-stage kernels: fixed-tree row sums, an unweighted relaxed histogram
of `assignments` into `counts`, a gather of each centroid count by
`centroid_count_indices`, a scalar normalization, and a unique scatter through
`centroid_base` and `centroid_indices`. `centroid_normalize_program` requires
nonnegative counts, converts the count to F32, divides only for positive counts,
and emits zero for an empty centroid. Empty clusters therefore have a defined
zero contribution instead of a hidden host branch.

### Pairwise L2: `gpu_pairwise_l2`

**Boundary.** Inputs are F32 `query` `[queries, dimensions]` and F32
`training` `[training_rows, dimensions]`; output `distances` is F32
`[queries, training_rows]`. Parameters are positive int32-bounded `queries`,
`training_rows`, and `dimensions`, plus `tree_lanes`.

**Graph.** The four recipe steps are materialized as follows:

1. Two parallel elementwise maps square `query` and `training` into same-shaped
   intermediates.
2. Two fixed-tree reductions produce `query_norms [queries, 1]` and
   `training_norms [training_rows]`.
3. A contraction with `batch_axes = []` and `contract_axes = [(1, 1)]`
   computes the query/training dot-product matrix.
4. `pairwise_l2_result_program` computes

   ```text
   squared = max(0, query_norm + training_norm - 2 * dot)
   distance = sqrt(squared)
   ```

   and writes `distances`.

The maximum clamp makes small negative roundoff visible as a defined zero
distance rather than passing a negative value to square root. All intermediate
workspace is accounted by `GraphBuilder`; no matrix is copied to the host.

**Callers.** The training validation path compares validation rows against
updated K-means centroids. The inference path compares rows against checkpoint
centroids. Both callers independently derive the same dimensions and reduction
lane count, so the concrete materializer remains the single graph owner.

### Core distance: `gpu_core_distance`

**Boundary.** F32 `points` is flattened `[points_count * dimensions]`; I32
`left_indices` and `right_indices` are `[2, dimensions]`; I32 `rank_indices`
is `[1]`; output F32 `core_distances` is `[2, 1]`. Parameters are
`points_count`, `dimensions`, `minimum_points`, `pair_indices_verified`,
`rank_indices_verified`, and `tree_lanes`.

**Concrete domain.** The current exact graph admits only `points_count == 2`
and `minimum_points == 1`. This is an intentional finite shape boundary, not a
claim that general core-distance computation is complete. Both verification
facts must be true.

**Graph.** Two gathers read left and right points. A squared-difference map,
axis-one sum reduction with `keep_dimensions = true`, and nonnegative square
root produce two distances. A stable ascending axis-one `Sort` then orders each
row. A checked axis-one gather using `rank_indices` selects the requested core
distance. Sort does not emit indices, so ties retain the stable source order.

### Fixed-radius neighbors: `gpu_fixed_radius_neighbors`

**Boundary.** F32 `points` is `[dimensions]`; I32 `counts [1]`,
`row_pointer_base [2]`, `row_pointer_indices [2]`, `row_pointer_updates [2]`,
`neighbor_base [1]`, `neighbor_indices [1]`; outputs are I32
`row_pointers [2]` and `neighbors [1]`. Parameters are `points_count`,
`dimensions`, `epsilon: F32Bits`, true `singleton_tables_verified`,
`bases_zero`, `destination_indices_unique`, and `tree_lanes`.

**Concrete domain and graph.** `points_count` must equal one, and epsilon must
be finite and nonnegative. The one-row graph computes an exclusive forward sum
scan of `counts` with I32 zero identity, maps the resulting offsets through an
explicit identity, then emits two unique scatters: row-pointer updates and
neighbor output updates. The singleton and zero-base facts are required because
the concrete graph does not materialize the general pairwise radius search,
predicate, or variable-size allocation path. The prepared tables carry those
semantics into the bounded graph.

### Connected components: `gpu_union_find_cc`

**Boundary.** I32 `parent_base [2]`, `edge_sources [edges]`,
`edge_destinations [edges]`, `node_indices [2]`, and `labels_base [2]`; output
I32 `labels [2]`. Parameters are `nodes`, `edges`,
`two_node_topology_verified`, `parent_base_verified`, `node_indices_verified`,
and `labels_base_zero`. `iteration_shape_input` must be exactly `parent_base`.
The concrete shape guard requires `nodes == 2`; `edges` is positive and
int32-bounded.

**Graph.** The one resolved pointer-jumping round has three recipe families.
Two gathers read source and destination roots from `parent_base`. The
two-output map emits `hook_destinations = max(source, destination)` and
`hook_updates = min(source, destination)`. A sequentially consistent atomic
minimum scatter applies those hooks to a copied parent image. A checked gather
compresses the two requested node indices, and a unique scatter writes labels
through `labels_base`. The topology and zero-base facts are preparation
proofs, not runtime substitutes. The atomic conflict policy is explicit and
cannot be inferred from a generic scatter.

### Boruvka MST: `gpu_boruvka_mst`

**Boundary.** I32 `parent [2]`, `edge_sources [edges]`, `edge_destinations
[edges]`, `edge_indices [edges]`, `mask_base [edges]`, and `unit_update [1]`;
F32 `edge_weights [edges]`; outputs are I32 `in_mst [edges]` and F32
`total_weight [1]`. Parameters are `nodes`, `edges`,
`two_node_topology_verified`, `parent_verified`, `edge_indices_verified`,
`edge_weights_finite`, `mask_base_zero`, and `tree_lanes`. `iteration_shape_input`
must be `parent`; `nodes == 2` is required.

**Graph.** The bounded round gathers source and destination components through
`parent` and gathers edge weights through `edge_indices`. A reduction over the
edge axis returns `ValueAndIndex` for the minimum weight and representative
edge. An identity map writes the minimum weight to `total_weight`; a unique
scatter writes `unit_update` at the selected edge through `mask_base` and
`minimum_edge` to form `in_mst`. `ReduceResult::ValueAndIndex` reserves two
output tensors and fixes the tie-visible representation. The finite two-node
domain and all verification facts are required because general component
rounds, edge filtering, and union scheduling are not emitted here.

## RL and probability materializers

### Temporal-difference targets: `gpu_td_targets`

**Boundary.** F32 `rewards`, `values`, and `targets` are `[elements]`; I32
`next_value_indices` and `done_mask` are `[elements]`. Parameters are
`elements` and finite `gamma`, plus true `next_value_indices_verified`.

**Graph and formula.** A checked gather selects `next_values =
values[next_value_indices]`. The elementwise program validates `done_mask` as
exactly zero or one, converts it to F32, computes `live = 1 - done`, and emits

```text
targets = rewards + (gamma * next_values) * live
```

The terminal mask therefore removes the bootstrap value without a host branch.
`gamma` is finite but otherwise not restricted by this materializer.

### Discounted returns: `gpu_discounted_returns`

**Boundary.** F32 `rewards` and `returns` are `[length]`; parameters are
`length`, finite `gamma`, and `tree_lanes`.

**Concrete domain.** The current graph requires `gamma == 1.0`. This is because
the available `Scan` primitive is a reverse inclusive sum, while a general
discounted recurrence needs a distinct affine scan carrying the discount.
Values other than one fail with `UnsupportedConcreteShape`; no silently
incorrect sum is emitted.

**Graph.** An identity map creates `mapped_rewards`, then a reverse inclusive
axis-zero sum scan writes `returns`. The recipe is `MAP_SCAN`, and tree lanes
fix the scan's operation order.

### Generalized advantage estimation: `gpu_gae`

**Boundary.** F32 `rewards`, `values`, and `advantages` are `[length]`; I32
`next_value_indices` and `has_next` are `[length]`. Parameters are `length`,
finite `gamma`, finite `lambda`, `tree_lanes`, and true
`next_value_indices_verified`.

**Concrete domain.** The graph requires `gamma * lambda == 1.0`. The available
reverse sum scan can then represent the requested recurrence exactly. Other
finite pairs fail closed with `UnsupportedConcreteShape`.

**Graph and formula.** One stage gathers `next_values` and maps

```text
next = select(has_next, next_value, 0)
delta = rewards + gamma * next - values
```

The bool mask helper requires `has_next` to be zero or one. The final stage is
a reverse inclusive sum scan over `delta`, writing `advantages`. There is no
implicit terminal-value convention beyond the prepared mask.

### Gaussian log probability: `gpu_gaussian_logprob`

**Boundary.** F32 `means`, `log_standard_deviations`, and `actions` are
`[rows, dimensions]`; F32 `log_probabilities` is `[rows]`. Parameters are
positive `rows`, positive `dimensions`, and `tree_lanes`.

**Graph and formula.** The first stage computes `exp(log_standard_deviations)`
and one per-dimension Gaussian term. For `z = (action - mean) /
exp(log_standard_deviation)`, the scalar program emits

```text
-0.5 * z * z - log_standard_deviation - 0.9189385
```

The second stage reduces axis one with a fixed sum tree into
`log_probabilities`. The exponential is the owned `recipe_math::Exp` scalar
program and its language errors are wrapped as graph-materialization errors.

### Categorical log probability: `gpu_categorical_logprob`

**Boundary.** F32 `logits [rows, action_count]` and flattened `flat_logits
[rows * action_count]`; I32 `actions [rows]` and `row_bases [rows]`; output F32
`log_probabilities [rows]`. Parameters are `rows`, `action_count`,
`tree_lanes`, and true `logit_views_identical` and `row_bases_verified`.

**Graph.** The first resolved gather-family stage emits seven same-stage
kernels: row maxima with and without kept dimensions, max subtraction, owned
exponential, row sums, checked flat action index formation, and a gather from
`flat_logits`. The action map requires `0 <= action < action_count` and adds
the action to its row base. The second map-family stage takes `log(row_sum)`
and emits `selected_logit - (row_max + log_sum)`. This is the stable
log-softmax probability for the selected action. The exact view and row-base
facts are required because the graph accepts two caller-provided layouts and
does not reconstruct their relationship.

## Scalar program inventory and guards

The family keeps all operation arithmetic in typed scalar SSA. The shared
builders `scalar_builder`, `scalar_input`, `scalar_binary`, `scalar_ternary`,
`scalar_unary`, and `scalar_finish` wrap language errors with the operation ID
([`graph_cluster_rl.rs:1641-2095`](../../../src/materialize/graph_cluster_rl.rs:1641)).
The notable programs are:

| Program | Inputs and result | Guard or exact behavior |
| --- | --- | --- |
| `bool_mask` | I32 mask to I32 mask | Requires the value to equal exactly zero or one. Used by terminal, active, and availability masks. |
| `temporal_difference_program` | F32 reward, F32 next value, I32 done to F32 target | Converts the checked mask and emits `reward + gamma * next * (1 - done)`. |
| `advantage_delta_program` | F32 reward/value/next, I32 has-next to F32 delta | Selects zero for no next state, then emits `reward + gamma * next - value`. |
| `gcn_normalization_program` | F32 feature and degree to F32 feature | Requires nonnegative degree; divides by square root only for positive degree. |
| `gaussian_term_program` | Four F32 values to one F32 term | Uses the fixed `-0.5*z*z - log_sd - 0.9189385` order. |
| `checked_flat_action_program` | I32 row base/action to I32 flat index | Requires `0 <= action < action_count` before addition. |
| `categorical_finish_program` | F32 selected/max/log-sum to F32 log probability | Emits selected logit minus the stable log normalizer. |
| `masked_product_program` | F32/F32/I32 to F32 | Requires a Boolean active mask, multiplies, and selects zero when inactive. |
| `neighbor_normalize_program` | F32 sum and degree to F32 | In mean mode requires nonnegative degree and divides only for positive degree; sum mode returns the sum. |
| `masked_value_program` | F32 value and I32 mask to F32 | Requires zero-or-one mask and selects value or zero. |
| `centroid_normalize_program` | F32 sum and I32 count to F32 | Requires nonnegative count, converts to F32, divides only when positive, and returns zero for empty clusters. |
| `squared_difference_program` | F32/F32 to F32 | Subtracts then multiplies the difference by itself. |
| `square_program` | F32 to F32 | Multiplies a value by itself. |
| `nonnegative_square_root_program` | F32 to F32 | Requires nonnegative input before square root. |
| `pairwise_l2_result_program` | F32 query norm, training norm, dot product to F32 | Forms `max(0, q + t - 2*dot)` then square root. |
| `minimum_representative_hook_program` | I32 source/destination to two I32 outputs | Emits maximum endpoint for the hook destination and minimum endpoint for the hook update. |

`identity_program` is shared with the other materializer families for typed
copy stages. It is not a host copy and does not create a second execution
boundary.

## Workspace formulas

`GraphBuilder` is the authoritative workspace accountant. It charges every
internal tensor's `storage_bytes` as it is allocated and charges no caller
boundary tensor. The following formulas are therefore the exact minimum bytes
for the concrete graphs, assuming the named dimensions are the checked values
and every F32 or I32 element occupies four bytes. They are useful when a
caller supplies a finite `workspace_limit`; the emitter still computes the
total from actual `Tensor` objects rather than trusting this table.

| Operation | Symbols | Internal workspace bytes |
| --- | --- | ---: |
| `gpu_degree` | `E = edges` | `4E` |
| `gpu_td_targets` | `N = elements` | `4N` |
| `gpu_gcn_norm` | `N = nodes * features_per_node` | `4N` |
| `gpu_discounted_returns` | `L = length` | `4L` |
| `gpu_gae` | `L = length` | `8L` |
| `gpu_gaussian_logprob` | `R = rows`, `D = dimensions` | `8RD` |
| `gpu_categorical_logprob` | `R = rows`, `A = action_count` | `8RA + 24R` |
| `gpu_csr_spmv` | `O = rows`, `W = row_width` | `12OW + 4O` |
| `gpu_csr_spmm` | `O = rows * features`, `W = row_width` | `12OW + 4O` |
| `gpu_neighbor_aggregate` | `O = nodes * features_per_node`, `W = row_width` | `8OW + 12O` |
| `gpu_centroid_update` | `C = centroid_count * dimensions`, `P = points_per_centroid` | `8CP + 12C` |
| `gpu_pairwise_l2` | `Q = queries`, `T = training_rows`, `D = dimensions` | `4(QD + TD + Q + T + QT)` |
| `gpu_core_distance` | `D = dimensions` | `24D + 24` |
| `gpu_fixed_radius_neighbors` | singleton scan | `8` |
| `gpu_union_find_cc` | `E = edges` | `16E + 16` |
| `gpu_boruvka_mst` | `E = edges` | `12E + 8` |

For CSR matrix mode, `O` already includes `rows * features`; the operand
element count is `columns * features`. Boundary outputs such as `degrees`,
`targets`, `distances`, `labels`, `in_mst`, and `total_weight` remain outside
these totals. If a shape or byte multiplication overflows, the operation
returns an error before constructing a misleading allocation.

## Failure behavior and state transitions

The state transition is intentionally one-way:

```text
raw descriptor
  -> validated composition descriptor
  -> resolved finite steps
  -> emitter cursor and reserved IDs
  -> typed graph and workspace allocation
  -> validated immutable fragment
```

Every failure leaves the real invalid transition visible. The relevant failure
classes are:

| Stage | Failure | Meaning |
| --- | --- | --- |
| Registry | `UnknownOperation`, `AmbiguousSymbol` | A public symbol is absent or has multiple source-qualified rows. Use `resolve_exact` for source-qualified legacy entries. |
| Descriptor | `WrongLoweringKind`, `InvalidCompositionRecipe` | The request is not a structured composition or the static recipe is malformed. |
| Request | `InvalidMaterializationRequest` | Empty/duplicate names, duplicate IDs, wrong external flags, missing tensors, false prepared facts, nonfinite parameter, or wrong iteration input. |
| Prepared values | `MissingPreparedParameter`, `PreparedParameterTypeMismatch` | The exact typed ABI was not supplied. |
| Bounds | `IterationBoundUnresolved`, `CompositionExpansionOverflow`, `UnsupportedConcreteShape` | Shape axis, repeat count, int32 domain, operation-specific finite shape, or gamma restriction is invalid. |
| Identity | `IdentityNamespaceOverlap`, `IdentityNamespaceExhausted` | Caller IDs overlap declared boundaries, ranges overflow, or capacities are too small. |
| Workspace | `WorkspaceArithmeticOverflow`, `WorkspaceLimitExceeded` | Internal byte accounting overflowed or exceeded the request limit. |
| Graph | `GraphMaterializationFailed` | Scalar construction, primitive shape/dtype validation, kernel family mismatch, over/under-emission, duplicate producer, missing producer, alias, or cycle check failed. |

Runtime index violations remain `IndexBounds::Reject` device faults. A malformed
mask is rejected by a scalar `Require` instruction. Neither case is converted
into a default value, clamp, retry, alternate primitive, or host-side fallback.

### Paired-case matrix

The paired cases below are the places where one concrete Rust implementation
serves more than one source-qualified operation while preserving distinct
public descriptors:

| Pair or group | Shared implementation | Deliberate difference |
| --- | --- | --- |
| `gpu_csr_spmv` / `gpu_csr_spmm` | `emit_csr_padded` | Vector has one operand feature per row; matrix derives `columns * features` and `rows * features`. |
| `gpu_discounted_returns` / `gpu_gae` | Reverse map-plus-scan skeleton | Returns uses rewards directly and admits only `gamma == 1`; GAE gathers next values, forms deltas, and admits only `gamma * lambda == 1`. |
| `gpu_union_find_cc` / `gpu_boruvka_mst` | Two-node bounded component topology, gather and ordered scatter state updates | Union-find publishes minimum representatives and labels; Boruvka selects a minimum weighted edge and writes MST membership and total weight. |
| `gpu_gcn_norm` / `gpu_neighbor_aggregate` | Checked degree gather followed by typed F32 normalization | GCN emits one value per feature edge; neighbor aggregation gathers padded feature rows, masks, reduces, optionally divides, and scatters. |
| `gpu_core_distance` / `gpu_pairwise_l2` | F32 squared-distance arithmetic and fixed reduction trees | Core distance is a two-point rank-selection graph with sort/gather; pairwise L2 is a general `[queries, training_rows]` contraction. |
| `gpu_td_targets` / `gpu_gae` | I32 zero-or-one continuation masks and checked next-value gathers | TD writes one-step targets; GAE writes deltas and then a reverse scan. |

These are shared stages, not aliases in the registry. The `OperationId`, source,
definition, exact ABI, and error details remain attached to the original
descriptor throughout materialization.

## End-to-end completion contract

An accepted call has all of the following observable preparation outcomes:

1. The descriptor is the exact operation-surface row and its composition
   recipe validates.
2. The exact tensor and parameter name sets pass `require_exact_abi`.
3. All boundary tensors pass language validation and do not occupy the
   intermediate identity range.
4. Every prepared dimension, product, F32 parameter, Boolean fact, and tree
   lane has been checked by the operation-specific emitter.
5. The emitter cursor consumed exactly the resolved recipe steps. Same-stage
   kernels are recorded in `StageEmission` without inventing additional recipe
   steps.
6. Every internal tensor is charged to `WorkspaceAllocation`, every kernel
   and value ID remains inside the caller reservation, and every kernel has a
   complete forbidden alias matrix.
7. `CalculationGraph::validate` proves known tensors, unique kernels and
   producers, external boundary consistency, primitive contracts, and an
   acyclic producer graph.

The returned `MaterializedComposition` is then safe to pass to graph assembly.
Assembly may discard fragment-local external flags and choose the complete
model boundary, but it must preserve every dtype, shape, layout, storage span,
value identity, kernel identity, primitive kind, and dependency. Hardware
lowering and execution happen only after this graph/state contract is complete.
