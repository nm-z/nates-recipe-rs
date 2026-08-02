# `ops/src/tree.rs`: complete-tree ensemble inference

## Module identity

```text
crate: recipe-ops
source: ops/src/tree.rs
graph_kind: recipe_language::CalculationGraph
calculation_dtype: F32 payloads with I32 indices and node identities
operation_identity: Recipe-owned direct graph builder, not an OperationId
model_shape: complete binary trees with one fixed depth and one output vector per leaf
```

This module turns a saved, statically shaped ensemble of complete binary trees
into one placement-free calculation graph. It owns validation of the concrete
tree traversal request, checked resource accounting, scalar index and branch
program construction, graph-local identities, and final graph validation. It
does not load a checkpoint, prepare feature schemas, choose a backend, allocate
device memory, schedule a graph, or execute a kernel. Those boundaries belong to
the training and inference crates and to the later Recipe planning/runtime
stages.

The module-level contract is stated in the opening documentation of
`ops/src/tree.rs:18-24`:

* `features` is a flattened row-major feature matrix with logical shape
  `[rows, feature_width]`;
* `split_features` and `split_thresholds` are tree-major arrays in complete-tree
  heap order, with one entry for each internal node;
* `leaf_values` is tree-major `[trees, 2^depth, outputs]` storage;
* an exact feature/threshold tie takes the left child;
* all tree outputs are reduced in tree axis order and then multiplied by
  `scale`.

The request carries the boundary tensors and all concrete dimensions rather than
inferring dimensions from a byte buffer. The graph builder therefore remains
usable by both training graph compilation and target-free checkpoint inference.

## Public surface and ownership

`ops/src/lib.rs:78-81` re-exports the three public types and three functions:

| Export | Meaning |
|---|---|
| `TreeEnsembleInferenceRequest` | Immutable boundary tensor contracts, dimensions, scale, reduction lanes, identity ranges, and workspace limit. |
| `TreeEnsembleInferenceRequirements` | Checked counts for internal nodes, leaves, intermediate values, kernels, and bytes. |
| `TreeEnsembleInferenceMaterialization` | Standalone graph, generated intermediate and kernel IDs, complete-tree extents, workspace bytes, and the unchanged identity namespace. |
| `tree_ensemble_inference_requirements` | Pure checked shape and resource calculation. It emits no graph. |
| `materialize_tree_ensemble_inference` | Validate one request and emit one complete graph fragment. |
| `append_tree_ensemble_inference` | Validate caller-owned boundary storage, materialize the fragment, and append only its intermediates and nodes. |

The direct functions in this module are deliberately separate from the
operation-surface registry. `ops/src/composition.rs:2506-2513` maps the legacy
source-qualified symbols `gpu_tb_apply_tree` and `gpu_tree_ensemble_predict` to a
descriptive `tree_ensemble_inference` composition. A normal
`materialize_composition` call (`ops/src/materialize.rs:413-480`) dispatches
those descriptors to `ops/src/materialize/tree_boosting.rs`, not to this file.
That materializer handles a named operation ABI and the generic composition
emitter. It is not a caller of `TreeEnsembleInferenceRequest`.

Conversely, `ops/src/tree.rs` has no `OperationDescriptor`, `OperationId`,
registry row, or operation-surface symbol. Its `tree_error` helper
(`ops/src/tree.rs:887-900`) leaves `OperationError.operation` as `None`. The
training and inference compilers call this direct graph boundary explicitly.
Keeping those two paths distinct prevents a generic legacy operation ABI from
being mistaken for the saved complete-tree model path.

## Request contract

Let

```text
L = 2^depth                         leaves in one tree
I = L - 1                           internal nodes in one tree
P = rows * trees                    row/tree traversal pairs
E = P * outputs                     expanded leaf contributions
R = rows * outputs                  reduced prediction elements
```

All arithmetic above is checked `u64` arithmetic in
`tree_ensemble_inference_requirements` (`ops/src/tree.rs:63-145`). The request
fields have these exact logical contracts:

| Field | Required dtype and logical shape | Use |
|---|---|---|
| `features` | `F32`, one-dimensional `[rows * feature_width]` | Flattened row-major feature matrix. `feature_index_program` maps a row and selected feature to this logical index. |
| `split_features` | `I32`, one-dimensional `[trees * I]` | Feature selector for every internal node, tree-major heap order. |
| `split_thresholds` | `F32`, one-dimensional `[trees * I]` | Threshold paired with each split feature. |
| `leaf_values` | `F32`, one-dimensional `[trees * L * outputs]` | Leaf vectors, tree-major then leaf-major then output-major. |
| `predictions` | `F32`, rank-two `[rows, outputs]` | Caller-owned output matrix. It is never an input and is marked external output in the emitted fragment. |

The code checks every tensor with `Tensor::validate` before checking the tree
ABI (`ops/src/tree.rs:303-312`). `Tensor::validate` enforces a valid layout,
storage span, and byte size (`language/src/tensor.rs:180-206`). The tree request
then enforces the listed dtype and extents (`ops/src/tree.rs:320-358`). It does
not require the layout metadata to be row-major or contiguous beyond the module
contract and the generic tensor validity rules. A non-contiguous but valid
logical layout is therefore accepted by this layer and retained in the boundary
contract.

Scalar request invariants are:

| Field | Invariant and failure |
|---|---|
| `rows`, `feature_width`, `trees`, `outputs` | Each is nonzero. A zero value returns `UnsupportedConcreteShape`. |
| `depth` | `1..=30`; zero and values `>=31` return `UnsupportedConcreteShape`. The upper bound keeps complete-tree indexes in the checked int32 domain. |
| Flattened image element counts | Feature, split, leaf-value, and row/tree pair counts must each be at most `i32::MAX`; otherwise `UnsupportedConcreteShape`. |
| `scale` | Must be finite. NaN and infinities return `InvalidMaterializationRequest`. |
| `tree_lanes` | Must be a power of two in `1..=1024` (`MAXIMUM_TREE_LANES` at `ops/src/tree.rs:12`). Invalid values return `InvalidMaterializationRequest`. |
| `identity_namespace` | Half-open value and kernel ranges supplied by the caller. The first ID plus capacity must not overflow `u64`, and required counts must fit both capacities. |
| `workspace_limit` | The exact byte reservation computed by the requirements must not exceed this limit. |

The request's `scale` and `tree_lanes` checks are operation-level checks. The
generic primitive validator repeats the fixed reduction-tree lane rule for the
emitted `Reduce` node (`language/src/primitive.rs:430-493` and
`language/src/primitive.rs:751-759`).

### Resource formula

`tree_ensemble_inference_requirements` computes:

```text
intermediate_values = 7 + 6 * depth
kernels             = 8 + 6 * depth
pair_tensors        = 3 + 6 * depth
workspace_elements  = pair_tensors * P + outputs + 2 * E + R
workspace_bytes     = 4 * workspace_elements
```

The `3 + 6 * depth` pair tensors are the two initial pair maps, six traversal
intermediates per depth, and the final `leaf_bases`. The remaining workspace is
the output-offset map (`outputs`), `leaf_indices` plus `contributions` (`2 * E`),
and `sums` (`R`). Every intermediate is either `I32` or `F32`, both four bytes in
the canonical core dtype contract (`core/src/scalar.rs:11-22`).

All products and sums use `checked_product` and `checked_sum`
(`ops/src/tree.rs:865-885`). Overflow returns
`WorkspaceArithmeticOverflow`, never a wrapped reservation. The requirements
function returns only counts and `ByteCount`; it does not reserve IDs or mutate
a graph.

## Mathematical traversal semantics

One logical pair index `p` represents row

```text
row  = floor(p / trees)
tree = p mod trees
```

The pair axis is `[rows, trees, 1]`, so all rows and all trees are traversed in
one static graph. For each depth level, the current `node` is a complete-tree
heap index. The split entry is

```text
split_index = tree * I + node
```

`split_features[split_index]` selects a feature, and the feature payload index
is

```text
feature_index = row * feature_width + split_features[split_index]
```

The selected feature value and threshold determine the child:

```text
right = 1 if selected_value > selected_threshold else 0
left  = 2 * node + 1
next  = left + right
```

The strict `GreaterThan` in `next_node_program` (`ops/src/tree.rs:693-709`)
means exact equality routes left. The scalar program requires the incoming
node to be an internal-node index and requires both feature and threshold to be
finite before evaluating the comparison. A malformed value triggers the device
fault channel through scalar `Require`; it is not clamped or replaced.

After the final depth, `node` is a leaf heap index. The leaf base is

```text
leaf         = node - I
tree_leaf    = tree * L + leaf
leaf_base    = tree_leaf * outputs
leaf_index   = leaf_base + output_offset
```

`leaf_base_program` checks `I <= node < I + L`
(`ops/src/tree.rs:711-732`), and `leaf_index_program` checks the output offset
(`ops/src/tree.rs:734-742`). Thus every gathered leaf value has a checked index
within the flattened tree-major leaf image. The graph gathers one vector entry
for every `(row, tree, output)` tuple, reduces axis `1` of
`[rows, trees, outputs]` to `[rows, outputs]`, and applies `scale` elementwise.
The reduction is `ReduceOperator::Sum`, `keep_dimensions = false`,
`ReduceResult::Value`, with the caller's fixed `tree_lanes`
(`ops/src/tree.rs:211-223`).

For `scale = 1.0`, the result is the fixed-order sum of the saved tree leaf
vectors. Other finite scales support a forest mean or a caller-owned ensemble
normalization without changing traversal or tree order. No fitting, averaging,
boosting round, or learning-rate behavior is hidden in this graph.

## Emitted graph

`materialize_tree_ensemble_inference` first calls `validate_request` and
`validate_resources` (`ops/src/tree.rs:147-152`). It then constructs one
`TreeEmitter` and emits the following relative sequence. Actual kernel IDs
start at `identity_namespace.first_kernel()` and increase by one. Actual
intermediate value IDs start at `identity_namespace.first_value()` and increase
by one.

### Boundary declarations

`TreeEmitter::new` inserts four input boundaries and one prediction boundary in
a `BTreeMap<ValueId, Tensor>` (`ops/src/tree.rs:477-546`):

| Boundary | Flags inside the standalone fragment |
|---|---|
| `features`, `split_features`, `split_thresholds`, `leaf_values` | `external_input = true`, `external_output = false` |
| `predictions` | `external_input = false`, `external_output = true` |

`insert_boundary` accepts a repeated input ID only when dtype, shape, layout,
and storage bytes are exactly equal (`ops/src/tree.rs:825-849`). A conflicting
repeated contract returns `InvalidMaterializationRequest`. The prediction ID
must not match any input ID, even if the contracts would otherwise match.
Boundary tensors are not counted as generated intermediates or workspace.

### Relative kernel and intermediate order

| Relative order | Intermediate output | Shape and dtype | Primitive | Inputs -> output | Meaning |
|---:|---|---|---|---|---|
| `K0` | `pair_positions` | `I32 [rows, trees, 1]` | `IndexMap(start=0, element_step=1, iteration_step=0)` | none -> `pair_positions` | Logical pair indices `0..P-1`. |
| `K1` | `nodes` | `I32 [rows, trees, 1]` | `IndexMap(start=0, element_step=0, iteration_step=0)` | none -> `nodes` | All pairs start at heap root node `0`. |
| `K2 + 6d` | `split_indices` | `I32 [rows, trees, 1]` | `Elementwise(split_index_program)` | `pair_positions`, `nodes` -> `split_indices` | Maps the current tree and internal node into tree-major split storage. |
| `K3 + 6d` | `selected_features` | `I32 [rows, trees, 1]` | `Gather(axis=0, bounds=Reject)` | `split_features`, `split_indices` -> `selected_features` | Reads the selected feature ID for every row/tree pair. |
| `K4 + 6d` | `selected_thresholds` | `F32 [rows, trees, 1]` | `Gather(axis=0, bounds=Reject)` | `split_thresholds`, `split_indices` -> `selected_thresholds` | Reads the paired threshold. |
| `K5 + 6d` | `feature_indices` | `I32 [rows, trees, 1]` | `Elementwise(feature_index_program)` | `pair_positions`, `selected_features` -> `feature_indices` | Maps row and selected feature into flattened feature storage. |
| `K6 + 6d` | `selected_values` | `F32 [rows, trees, 1]` | `Gather(axis=0, bounds=Reject)` | `features`, `feature_indices` -> `selected_values` | Reads each row's selected feature. |
| `K7 + 6d` | next `nodes` | `I32 [rows, trees, 1]` | `Elementwise(next_node_program)` | `selected_values`, `selected_thresholds`, current `nodes` -> next `nodes` | Applies the strict-greater-than branch rule and advances one depth. |
| after depth | `leaf_bases` | `I32 [rows, trees, 1]` | `Elementwise(leaf_base_program)` | `pair_positions`, final `nodes` -> `leaf_bases` | Converts final heap leaf indexes to flattened leaf-vector bases. |
| next | `output_offsets` | `I32 [1, 1, outputs]` | `IndexMap(start=0, element_step=1, iteration_step=0)` | none -> `output_offsets` | Generates output offsets `0..outputs-1` for broadcasting. |
| next | `leaf_indices` | `I32 [rows, trees, outputs]` | `Elementwise(leaf_index_program)` | `leaf_bases`, `output_offsets` -> `leaf_indices` | Expands each leaf base across output columns. |
| next | `contributions` | `F32 [rows, trees, outputs]` | `Gather(axis=0, bounds=Reject)` | `leaf_values`, `leaf_indices` -> `contributions` | Selects every saved leaf output. |
| next | `sums` | `F32 [rows, outputs]` | `Reduce(Sum, axis=1, keep_dimensions=false, tree_lanes)` | `contributions` -> `sums` | Sums tree contributions for each row and output. |
| final | caller `predictions` | `F32 [rows, outputs]` | `Elementwise(scale_program)` | `sums` -> `predictions` | Multiplies each sum by the finite request scale. |

The six rows marked `d` are emitted once per declared depth. The initial two
index maps plus the final six kernels give `8 + 6d` kernels. The two initial
pair tensors, six per depth, and final `leaf_bases` account for the `3 + 6d`
pair tensors in workspace; five additional intermediates are emitted after the
traversal, producing `7 + 6d` total intermediate values.

All emitted primitive nodes use `forbidden_aliases` (`ops/src/tree.rs:851-863`),
which creates an explicit `AliasPermission::Forbidden` rule for every input and
output pair. The traversal cannot overwrite a feature, split, threshold, leaf,
or intermediate input in place.

### Scalar program inventory

| Program | Inputs | Checked constants and guards | Result |
|---|---|---|---|
| `split_index_program` (`:667-678`) | `pair: I32`, `node: I32` | `trees`, `internal_nodes` converted to I32; `0 <= node < internal_nodes` | `(pair % trees) * internal_nodes + node` |
| `feature_index_program` (`:680-691`) | `pair: I32`, `feature: I32` | `trees`, `feature_width` converted to I32; `0 <= feature < feature_width` | `(pair / trees) * feature_width + feature` |
| `next_node_program` (`:693-709`) | `value: F32`, `threshold: F32`, `node: I32` | Internal-node range; `IsFinite(value)` and `IsFinite(threshold)` through `Require` | `2 * node + 1 + (value > threshold)` |
| `leaf_base_program` (`:711-732`) | `pair: I32`, `node: I32` | `trees`, `internal_nodes`, `leaves`, `outputs` converted to I32; `internal_nodes <= node < internal_nodes + leaves` | `((pair % trees) * leaves + (node - internal_nodes)) * outputs` |
| `leaf_index_program` (`:734-742`) | `base: I32`, `output: I32` | `0 <= output < outputs` | `base + output` |
| `scale_program` (`:744-750`) | `value: F32` | Finite request scale is embedded as an F32 literal | `value * scale` |

`require_i32_range` and `require_i32_interval`
(`ops/src/tree.rs:762-789`) construct two-sided comparisons and an int32
`BitAnd`, then issue scalar `Require`. `scalar_i32`
(`ops/src/tree.rs:797-805`) reports `UnsupportedConcreteShape` when a semantic
extent cannot be represented by an i32 literal. `scalar_builder`,
`scalar_input`, `scalar_binary`, and `scalar_finish` convert language-level
builder errors to `GraphMaterializationFailed`.

Gather nodes use `IndexBounds::Reject`. The generic gather validator requires
two inputs, an I32 index tensor, matching value/output dtype, and the exact
shape derived by `Shape::gather_result` (`language/src/primitive.rs:594-604`).
Thus an arithmetic or payload corruption that produces an out-of-range logical
index becomes a device fault, not unchecked memory access. The request-level
shape checks and scalar guards make valid saved trees stay in range; `Reject`
keeps invalid state observable at runtime.

## Emitter state, identity, and final graph validation

`TreeEmitter` (`ops/src/tree.rs:463-475`) owns:

| State | Purpose |
|---|---|
| `tensors` | Boundary declarations followed by generated contiguous intermediate tensors. |
| `nodes` | Ordered `CalculationNode` values, one per emitted primitive kernel. |
| `intermediate_values`, `kernels` | Exact generated ID lists returned to the caller. |
| `next_value`, `value_end` | Cursor and exclusive limit for the caller's reserved value range. |
| `next_kernel`, `kernel_end` | Cursor and exclusive limit for the caller's reserved kernel range. |
| `workspace_bytes` | Checked sum of every intermediate tensor's storage bytes. |
| `requirements`, `identity_namespace` | Expected count/byte formula and provenance returned in the materialization. |

`TreeEmitter::new` checks both range additions for `u64` overflow and rejects a
boundary tensor whose `ValueId` lies inside the reserved intermediate range
(`ops/src/tree.rs:482-533`). It does not treat zero as an invalid graph ID; ID
validity for calculation graphs is owned by `CalculationGraph` and the caller's
identity policy. Each `intermediate` call checks the value cursor, creates a
row-major contiguous tensor, adds its storage bytes with checked arithmetic, and
records the new value (`:549-570`). Each `emit` call checks the kernel cursor,
creates a `PrimitiveKernel` with forbidden aliases, and records the new kernel
(`:573-593`).

`finish` is an internal consistency gate (`:632-664`):

1. the emitted intermediate count must equal `requirements.intermediate_values`;
2. the emitted kernel count must equal `requirements.kernels`;
3. the measured intermediate storage total must equal
   `requirements.workspace_bytes`;
4. the assembled `CalculationGraph` must pass `graph.validate()`.

The language graph validator checks unique tensor and kernel IDs, valid tensor
contracts, known kernel inputs and outputs, complete alias matrices, one
producer per non-input tensor, no producer for external inputs, and an acyclic
topological order (`language/src/graph.rs:78-137`). The tree builder therefore
returns a graph only after both its own formula and the generic graph semantics
agree.

The returned `TreeEnsembleInferenceMaterialization` contains the full graph,
but `intermediate_values` identifies only generated tensors. Boundary tensors
and the prediction output are intentionally excluded from that list and from
workspace accounting.

## Appending to a caller graph

`append_tree_ensemble_inference` (`ops/src/tree.rs:237-300`) is a separate
assembly boundary. It performs the following checks before mutating the graph:

1. `validate_boundary_storage` rejects repeated tensor IDs already present in
   the caller graph and requires each declared boundary ID to exist with the
   exact dtype, shape, layout, and storage-byte contract (`:375-424`). External
   flags are deliberately not compared.
2. The standalone request is fully validated and materialized.
3. Every generated intermediate ID must be absent from caller tensors.
4. Every generated kernel ID must be absent from caller nodes.
5. No existing caller kernel may already produce `request.predictions.id`.

Only after all five checks does the function append the generated intermediate
tensors and all materialized nodes (`:286-300`). It does not append boundary
tensor declarations, does not rewrite caller flags, and does not call
`graph.validate()` after the extension. The caller owns pre-existing graph
validity and should validate the assembled graph at its normal graph boundary.
There is no partial append on a reported precondition failure because mutation
occurs only after the checks above.

## Public declaration to training graph path

The direct graph is reached from the public fluent API through a typed model
declaration, not through the operation registry.

### Facade declarations

`API.ogdl:23-28` exposes standalone `.lgbm(depth)`, `.cbst(depth)`, and
`.xgbst(depth)` blocks plus `.forest(trees)` followed by exactly one nested
booster. The corresponding `Model` methods are in `src/api.rs:1255-1280` and
`src/api.rs:1560-1580`:

* a standalone booster creates `LayerSpec::Lgbm`, `Cbst`, or `Xgbst`;
* `.forest(trees)` records a pending `LayerSpec::Forest { trees, booster: None }`;
* the next `.lgbm`, `.cbst`, or `.xgbst` fills that pending booster;
* calling a booster without a pending forest creates the matching standalone
  layer;
* zero tree counts and zero depths are retained as deferred declaration errors.

`LayerSpec::validate` requires nonzero standalone depth, nonzero forest count,
and a present nested booster with nonzero depth (`src/api.rs:780-826`). The
public activation and normalization methods reject tree and forest blocks as
valid predecessors (`src/api.rs:1426-1457` and `:1584-1621`), so the fluent API
does not attach those operations to a terminal tree.

### Dense model typing

`src/training.rs:1985-2034` maps the facade to `DenseTree`:

| Facade | `DenseTreeFamily` | Tree count |
|---|---|---:|
| `.lgbm(depth)` | `LightGbm` | `1` |
| `.cbst(depth)` | `CatBoost` | `1` |
| `.xgbst(depth)` | `XGBoost` | `1` |
| `.forest(trees).lgbm(depth)` | `LightGbm` | `trees` |
| `.forest(trees).cbst(depth)` | `CatBoost` | `trees` |
| `.forest(trees).xgbst(depth)` | `XGBoost` | `trees` |

`DenseTree` stores family, nonzero tree count, and nonzero depth
(`training/src/model.rs:546-588`). A `DenseBlock::Tree` has no ordinary output
operation (`training/src/model.rs:760-806`). `validate_config` requires a tree
block to be the sole leading model block and repeats the depth `1..=30` bound
(`training/src/compile.rs:1827-1841`). `effective_blocks` deliberately returns a
single tree block with no dense output adapter (`training/src/compile.rs:1966-1992`).

### Training compilation sequence

`compile_training_blocks` dispatches a `DenseBlock::Tree` to
`GraphCompiler::compile_training_tree` (`training/src/compile.rs:3760-4024` and
`:3910-3927`). The tree compiler:

1. requires nonzero input and output widths;
2. calls `tree_ensemble_inference_requirements` for the declared rows, width,
   tree count, depth, and task output width (`:5017-5035`);
3. builds a supervised split structure with the family-specific builder;
4. creates resume-selectable split feature and split threshold tensors;
5. initializes leaf values to zero as the only learned tree parameter;
6. packs the f32 feature matrix to one-dimensional storage;
7. calls `materialize_tree_ensemble_inference` for forward predictions; and
8. emits a second, training-owned route traversal for leaf indices used by the
   leaf-value gradient.

The saved complete-tree layout is produced once in `init`, not rebuilt every
epoch. For a one-tree declaration, the structure builder uses the prepared rows
directly. For a forest, `bootstrap_tree_rows` draws a full-size sample with
replacement for each tree using a distinct Recipe Philox stream
(`training/src/compile.rs:4068-4104`). The tree structure builders retain the
family distinction:

* LightGBM (`:4372-4555`) selects one positive-gain reachable node-feature pair
  at a time, up to `min(2^depth - 1, rows - 1)` transitions, and retains a
  finite dummy-left route for unfilled positions.
* XGBoost (`:4557-5001` with a non-CatBoost declaration) selects a separate
  feature and threshold for each node at each level, writing complete-tree heap
  positions by unique scatters.
* CatBoost uses the same level loop but computes one supervised global mean
  threshold per feature and chooses one shared feature per level. Its branch is
  selected by the `DenseTreeFamily::CatBoost` condition around
  `training/src/compile.rs:4637-4648` and `:4828-4833`.

The gain builder uses fixed reduction trees and the documented supervised
variance-gain formula. Empty children and nonpositive candidate gains are
handled by the family-specific scalar programs before a split is written. The
resulting split feature and threshold tensors have lengths `trees * I` and are
the exact tensors later consumed by this module.

`materialize_tree_predictions` allocates a caller prediction tensor, reserves
exactly the requirements count from the compiler's `next_value` and
`next_kernel` cursors, constructs a request with `scale = 1.0` and
`workspace_limit = requirements.workspace_bytes`, and inserts the returned graph
(`training/src/compile.rs:5134-5189`). `insert_materialized_graph` copies tensor
contracts and nodes and assigns every emitted kernel the training iteration
domain (`training/src/compile.rs:11000-11015`). The final compiler finish
re-applies authoritative external-input/output sets and validates the complete
graph before canonical OGDL round-tripping (`training/src/compile.rs:11078-11112`).

### Training targets, gradients, and state

`tree_target_matrix` accepts f32 `[rows, outputs]` targets for regression,
binary, and multi-target tasks. For multiclass classification it accepts I32
`[rows, 1]` class codes and emits a supervised one-hot f32 matrix
(`training/src/compile.rs:4026-4065`). This target preparation is outside
`ops/src/tree.rs`; the traversal itself only sees f32 feature, threshold, leaf,
and prediction payloads.

The tree is terminal and does not expose a differentiable input gradient. During
backward compilation, `BlockValues::Tree` rejects a request for an input
gradient and calls `tree_leaf_value_gradient`
(`training/src/compile.rs:8498-8507` and `:9628-9665`). That routine gathers
the output gradient for every row, repeats it across tree/output pairs using the
saved `leaf_indices`, and performs a deterministic segment sum into the flat
leaf parameter image. Only leaf values receive gradients.

`BlockGradients::Tree` maps that image to `ParameterRole::TreeLeafValue`
(`training/src/compile.rs:2204-2209`). At state update, AdamW applies the
gradient and writes a `DenseTreeState` retaining the declaration, widths,
complete-tree extents, split tensors, and updated leaf parameter state
(`training/src/compile.rs:10311-10323`). Split features and thresholds are fixed
structure tensors, not optimizer parameters.

## Target-free checkpoint inference path

The checkpoint inference compiler has its own caller of the same direct module.
`training/src/inference.rs:3782-3874` performs this sequence:

1. require that the checkpoint tree input width equals the current matrix width;
2. require f32 input shape `[rows, input_width]`;
3. recompute requirements from the saved declaration and output width;
4. reject cached `internal_nodes_per_tree` or `leaves_per_tree` values that
   differ from the declaration;
5. expose checkpoint split features, thresholds, and leaf parameters as external
   tensors;
6. pack the query matrix to flat f32 storage;
7. allocate `[rows, output_width]` predictions and reserve the exact requirement
   ranges;
8. build the request with `scale = 1.0`, saved tree counts/depth, and the
   inference reduction lane count; and
9. materialize and insert the graph into the target-free static program with
   `IterationDomain::first()`.

The broader inference compiler dispatches a saved `CheckpointBlockImage::Tree`
to `compile_tree` (`training/src/inference.rs:1093-1102`). The public inference
preparation path therefore decodes and validates the semantic artifact first,
then uses this graph builder for query rows. No tree structure is rebuilt or
resampled during inference.

## Checkpoint and resume state

Tree state is persisted by `DenseTreeState` (`training/src/model.rs:2404-2415`).
Checkpoint format version 12 is the tree semantic-model boundary
(`training/src/checkpoint.rs:40-71`). A serialized tree image contains:

```text
index
family                 lightgbm | catboost | xgboost
trees
depth
input-width
output-width
internal-nodes-per-tree
leaves-per-tree
split-features
split-thresholds
leaf-values             parameter, first moment, second moment
```

The decoder reconstructs the exact `DenseTree` family/count/depth and all saved
tensors (`training/src/checkpoint.rs:3901-3977`). Structured checkpoint
validation requires exactly one terminal tree block, no dense output adapter,
depth at most 30, cached complete-tree counts matching `2^depth`, and output
width matching the saved task (`training/src/checkpoint.rs:6802-6848`). Payload
validation requires split features to be valid I32 values within input width,
finite f32 thresholds, and leaf parameter tensors with the exact flattened
shape (`training/src/checkpoint.rs:6568-6641` and `:7185-7251`).

On resume, `ExternalInputRole::ResumeTreeSplitFeatures` and
`ResumeTreeSplitThresholds` replace the fresh structure tensors with exact
checkpoint payloads after shape and dtype checks
(`training/src/checkpoint.rs:7824-7859`). Leaf values and both AdamW moment
images use the generic three-component parameter resume path
(`training/src/checkpoint.rs:7862-7888`). The resume selector in the compiled
graph chooses fresh versus saved structure using the authoritative resume-enable
input (`training/src/compile.rs:6883-6919`). Missing or invalid checkpoint
structure is therefore rejected at checkpoint/inference preparation, while a
valid resumed run preserves the same complete-tree traversal graph.

## Failure taxonomy and ownership

The direct module reports `OperationErrorKind` values from
`ops/src/error.rs:5-28`. The observed tree-specific paths are:

| Kind | Direct cause in this module |
|---|---|
| `UnsupportedConcreteShape` | Zero dimensions, depth outside `1..=30`, image count above the checked I32 domain, or a semantic extent that cannot become an I32 scalar literal. |
| `WorkspaceArithmeticOverflow` | Checked product/sum overflow, workspace-byte overflow, or another checked `u64` arithmetic failure. |
| `InvalidMaterializationRequest` | Wrong dtype/shape, nonfinite scale, invalid tree lane count, conflicting duplicate boundary contract, or prediction/input identity alias. |
| `IdentityNamespaceOverlap` | Boundary ID inside the reserved intermediate range, generated value/kernel ID already present in an append target, or a generated namespace cursor collision. |
| `IdentityNamespaceExhausted` | Required value or kernel count exceeds the caller reservation, the first ID plus capacity overflows, or an emitter cursor reaches its exclusive end. |
| `WorkspaceLimitExceeded` | Exact generated intermediate bytes exceed `workspace_limit`. |
| `WorkspaceFormulaMismatch` | The emitted counts or bytes differ from the checked requirements formula. This indicates a materializer implementation inconsistency, not a caller shape error. |
| `GraphMaterializationFailed` | Tensor or scalar builder failure, primitive validation failure, duplicate existing prediction producer, or final `CalculationGraph::validate` failure. |

The generic language layer can additionally report duplicate tensors/kernels,
unknown values, duplicate producers, invalid alias matrices, shape or dtype
mismatches, invalid axes, gather contract failures, cycles, and invalid fixed
reduction lanes. `graph_error` wraps these language errors as
`GraphMaterializationFailed` with the original detail (`ops/src/tree.rs:887-892`).

This separation is intentional. Shape and identity facts known before emission
are rejected synchronously. Payload facts that can only be observed while a
kernel runs, such as a nonfinite selected value or an out-of-range gather index,
remain scalar `Require` or `IndexBounds::Reject` device-fault behavior. No host
fallback, retry, clamping, substitute tree, or duplicate traversal is added.

## End-to-end lifecycle

The complete production path for a saved tree model is:

```text
public fluent declaration or semantic checkpoint
    -> DenseTree family/count/depth and validated task/schema
    -> training structure builder or checkpoint payload admission
    -> TreeEnsembleInferenceRequest
    -> requirements and resource/identity validation
    -> static CalculationGraph with fixed primitive IDs and domains
    -> graph validation and OGDL/static-program canonicalization
    -> preparation of backend artifacts and immutable init/loop/exit schedule
    -> device execution of the graph
    -> authoritative prediction state and public reporting
```

For training, structure construction and leaf-gradient updates happen in the
compiler's static graph. For validation and target-free inference, the saved
split tensors and updated leaf values are external inputs to the same traversal
graph. The direct module never performs a host prediction, never mutates a
checkpoint, and never decides lifecycle phase or device placement.

The graph's authoritative state is its tensor contracts, primitive nodes,
external boundary flags supplied by the owning compiler, and the fixed scalar
programs. The caller owns feature preparation, checkpoint compatibility,
identity-range allocation, graph assembly, iteration domains, scheduling, and
runtime fault publication. This module owns only the validated complete-tree
traversal fragment and its exact resource contract.

## Source map

| Concern | Rust source |
|---|---|
| Request, requirements, materialization, append API | `ops/src/tree.rs:18-301` |
| Request and boundary validation | `ops/src/tree.rs:303-424` |
| Resource and identity checks | `ops/src/tree.rs:426-461`, `:477-570` |
| Primitive emission and final graph check | `ops/src/tree.rs:573-665` |
| Scalar programs and guards | `ops/src/tree.rs:667-820` |
| Boundary insertion, aliases, checked arithmetic | `ops/src/tree.rs:823-900` |
| Public re-exports | `ops/src/lib.rs:78-81` |
| Registry/composition distinction | `ops/src/composition.rs:472-485`, `:2506-2513`; `ops/src/materialize.rs:413-480` |
| Fluent tree declarations | `src/api.rs:606-611`, `:744-796`, `:1255-1280`, `:1560-1580` |
| Facade to dense tree mapping | `src/training.rs:1985-2034` |
| Dense tree declaration/state types | `training/src/model.rs:546-588`, `:760-806`, `:2404-2415` |
| Training structure and forward materialization | `training/src/compile.rs:3760-4024`, `:4372-5001`, `:5004-5190` |
| Leaf routing and gradients | `training/src/compile.rs:5193-5297`, `:8498-8507`, `:9628-9665` |
| Validation/inference caller | `training/src/compile.rs:6944-7131`; `training/src/inference.rs:3782-3874` |
| Checkpoint decode/validate/resume | `training/src/checkpoint.rs:3901-3977`, `:6568-6641`, `:6802-6848`, `:7824-7888` |
| Generic graph and primitive invariants | `language/src/graph.rs:78-187`; `language/src/primitive.rs:206-249`, `:430-604`, `:687-759` |
