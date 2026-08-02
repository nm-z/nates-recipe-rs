# `ops/src/knn_outputs.rs`: all-output KNN graph construction

## Module identity

```text
crate: recipe-ops
source: ops/src/knn_outputs.rs
scope: one terminal, all-declared-output KNN inference graph
calculation_dtype: F32 for feature geometry and numeric means; I32 for masks, indices, codes, and categorical counts
distance: rooted L2 over one shared query/reference feature matrix
selection: stable ascending distance sort, with prepared reference-row order as the tie order
aggregation: unweighted numeric mean or unweighted categorical mode
execution: static CalculationGraph only; no host counting, allocation, backend choice, or native execution
```

This module is the Recipe operation boundary for the calculation part of
`.knn(neighbors)`. It receives already typed and shaped tensors, checks their
concrete contracts, computes a checked resource reservation, and emits a
standalone `recipe_language::CalculationGraph`. The graph has one shared
distance calculation and one independent reduction for each declared target.
The module does not prepare a dataset, fit dictionaries, normalize host data,
load an OGDL model, choose CUDA or HSA, allocate device storage, submit native
work, or decode public labels. Those boundaries remain in the training,
inference, and execution crates described below.

The normative semantic contract is system-contract C33. In particular,
`.knn(neighbors)` is a standalone terminal model, the saved training partition
is the immutable reference set, missing target values are excluded per output,
and heterogeneous output dtypes remain separate through native execution and
reporting. The operation source implements the calculation portion of that
contract; its callers own the semantic preparation and lifecycle portions.

`ops/src/lib.rs` re-exports the operation types and entry points:

| Export | Role |
|---|---|
| `KnnOutputSpec` | Static per-output kind, known-reference count, and optional class count used by resource planning. |
| `KnnOutputRequest` | Per-output reference values or codes, known mask, prediction boundary, and semantic counts. |
| `KnnAllOutputRequest` | Shared query/reference feature matrices, all output requests, neighbor count, reduction lanes, identity namespace, and workspace limit. |
| `KnnOutputRequirement` | Effective neighbor count and class count for one output. |
| `KnnAllOutputRequirements` | Checked output requirements, intermediate count, kernel count, and workspace bytes. |
| `KnnAllOutputMaterialization` | Validated graph fragment, generated value and kernel IDs, output requirements, workspace bytes, and unchanged namespace. |
| `knn_all_output_requirements` | Pure checked reservation calculation. |
| `materialize_knn_all_outputs` | Validate a request and emit one standalone graph fragment. |
| `append_knn_all_outputs` | Validate caller-owned boundary storage, materialize, and append the fragment to an existing graph. |

The module is private in `recipe-ops` and is reached by the public re-exports
only through typed callers. The operation-surface registry is not a second KNN
implementation: registry descriptors and legacy inventory rows do not invoke
this module.

## Semantic boundary and symbols

Use these symbols for one materialization:

```text
Q       = query row count
R       = reference row count
D       = feature width
K       = requested neighbor count
K_i     = min(K, known references for output i)
C_i     = class count for categorical output i
P       = Q * R, the pairwise query/reference element count
```

The caller provides contiguous row-major matrices. `query_features` and
`reference_features` are already normalized f32 matrices at this boundary.
The operation trusts no host-side distance or label result: every value it
uses is represented as a graph input and every reduction is a graph primitive.

### Request types

`KnnOutputSpec` has two variants:

| Variant | Fields | Meaning |
|---|---|---|
| `Numeric` | `known_references: u64` | One f32 value per reference row; prediction is the unweighted mean of selected values. |
| `Categorical` | `known_references: u64`, `classes: u64` | One int32 class code per reference row; prediction is the unweighted mode over `0..classes`. |

`KnnOutputSpec::known_references()` returns the mask cardinality for either
variant. `KnnOutputRequest::spec()` projects a full request to this planning
type without inspecting payload bytes.

`KnnOutputRequest` carries one independent semantic target:

| Variant | Reference boundary | Known boundary | Prediction boundary | Required dtype and shape |
|---|---|---|---|---|
| `Numeric` | `reference_values` | `known` | `predictions` | `F32 [R]`, `I32 [R]`, `F32 [Q, 1]` |
| `Categorical` | `reference_codes` | `known` | `predictions` | `I32 [R]`, `I32 [R]`, `I32 [Q, 1]` |

The known mask is one binary int32 value per reference row. A `1` means that
row participates in this output's neighbor set; a `0` means that row remains
in alignment but is excluded. The same reference feature row can therefore be
valid for one output and unknown for another. The request never concatenates
numeric and discrete target spaces.

`KnnAllOutputRequest` contains:

| Field | Boundary contract |
|---|---|
| `query_features` | Rank-two `F32 [Q, D]` query matrix. |
| `reference_features` | Rank-two `F32 [R, D]` saved reference matrix. Width must equal the query width. |
| `outputs` | Nonempty vector in target declaration order. Each output has its own values, mask, and prediction. |
| `neighbors` | Nonzero requested count. The effective count is calculated independently as `K_i`. |
| `tree_lanes` | Power of two in `1..=1024` for every sum reduction. The inference caller uses `1024`. |
| `identity_namespace` | Half-open value and kernel ID ranges reserved by the caller for this fragment. |
| `workspace_limit` | Maximum operation-owned intermediate bytes. Boundary tensors and prediction outputs are not charged to it. |

`KnnAllOutputRequirements` is the single planning result consumed by both the
caller and emitter. `KnnAllOutputMaterialization` returns the graph plus the
IDs and accounting that the caller must register in its static program. The
returned `identity_namespace` is the request's namespace, not a newly allocated
or renamed namespace.

## Mathematical behavior

For query row `q` and reference row `r`, the graph first computes the squared
rooted-L2 quantity from shared row norms and a contraction:

```text
dot[q,r]       = sum(d = 0 .. D-1, query[q,d] * reference[r,d])
norm_q[q]      = sum(d = 0 .. D-1, query[q,d] * query[q,d])
norm_r[r]      = sum(d = 0 .. D-1, reference[r,d] * reference[r,d])
squared[q,r]  = max(norm_q[q] + norm_r[r] - 2 * dot[q,r], 0)
distance[q,r] = sqrt(squared[q,r])
```

All operands and the result are required to be finite f32 values. The maximum
with zero is a deliberate roundoff boundary: a tiny negative squared distance
does not reach square root as a negative value, while a non-finite or otherwise
invalid value remains visible through the scalar `Require` checks.

For output `i`, the graph applies its known mask before sorting:

```text
masked_distance[q,r] = distance[q,r] when known_i[r] == 1
                      = +infinity otherwise
```

The mask scalar program requires every mask element to be exactly zero or one.
The reduced mask count must equal `known_references` from the request. Because
the count is positive and no more than `R`, the first `K_i` sorted rows are
known rows. The stable ascending sort on reference axis `1` emits both sorted
distances and int32 reference indices. Equal distances retain prepared
reference-row order, including after a resume append.

Numeric output `i` gathers the selected reference values, sums them across the
neighbor axis, and divides by the f32 representation of `K_i`:

```text
prediction_i[q,0] = sum(j = 0 .. K_i-1, value_i[q, j]) / K_i
```

The divisor is checked to remain in the exact f32 integer domain. The input and
result of the divide must be finite; there is no NaN substitution or fallback.

Categorical output `i` gathers selected class codes, maps each pair `(query,
class)` to a flat row/class bin, and builds an unweighted int32 histogram. It
then stably sorts each query's counts in descending order and gathers the first
class code. The lookup stream is generated in ascending class-code order, so an
exact vote tie retains the lowest canonical class code. The histogram uses
`AtomicOrdering::Relaxed` for its accumulation primitive; semantic ordering is
defined by the stable post-histogram sort, not by atomic completion order.

## Checked resource reservation

`knn_all_output_requirements` runs before graph emission and uses checked `u64`
products and sums. It rejects a zero `Q`, `R`, `D`, or `K`, an empty output
list, and any `R` above the int32 sort-index domain. For each output it rejects
zero or excessive known references, class counts outside `1..=i32::MAX`, and a
categorical `Q * C_i` histogram larger than `i32::MAX` bins. A numeric `K_i`
larger than `16_777_216` is rejected because the final divisor must be exactly
representable as f32.

The formula is expressed in logical elements. Every operation-owned tensor is
four bytes because its dtype is f32 or int32:

```text
base_workspace =
    (Q * D) + (R * D) + Q + R + (2 * P)

common_i =
    2 + (3 * P) + K_i + (Q * K_i)

numeric_extra_i =
    R + (Q * K_i) + Q

categorical_extra_i =
    R + (2 * Q * K_i) + Q + (5 * Q * C_i) + 1

workspace_elements = base_workspace + sum_i(common_i + extra_i)
workspace_bytes = workspace_elements * 4
```

The `2` in `common_i` is the one-element known-count and validated-count
storage. The three `P` terms are masked distances, sorted distances, and sorted
indices. `K_i` is the prefix index vector and `Q * K_i` is the gathered
neighbor-index matrix. Numeric extra storage is the validated reference vector,
gathered values, and row sums. Categorical extra storage is gathered codes,
row bases, bin indices, five query/class matrices, and the one-element first
class index.

The static counts are:

```text
base intermediates = 6
base kernels       = 6
numeric output     = 10 intermediates, 10 kernels
categorical output = 17 intermediates, 16 kernels
```

`requirements.outputs` preserves request order and stores `K_i` plus
`Some(C_i)` for categorical output or `None` for numeric output. The emitter
must produce exactly these counts and byte totals. A mismatch is
`WorkspaceFormulaMismatch`, not an adjusted reservation.

## Emitted calculation graph

`materialize_knn_all_outputs` validates and reserves before constructing a
`KnnAllOutputEmitter`. Base nodes are emitted once, before the per-output loop:

| Relative node | Intermediate | Shape and dtype | Primitive and inputs | Meaning |
|---:|---|---|---|---|
| `B0` | `query_squared` | `F32 [Q, D]` | `Elementwise(square_program)`, query features -> output | Finite square of every query feature. |
| `B1` | `reference_squared` | `F32 [R, D]` | `Elementwise(square_program)`, reference features -> output | Finite square of every saved feature. |
| `B2` | `query_norms` | `F32 [Q, 1]` | `Reduce(Sum, axis=1, keep_dimensions=true)` | One squared norm per query row. |
| `B3` | `reference_norms` | `F32 [R]` | `Reduce(Sum, axis=1, keep_dimensions=false)` | One squared norm per reference row. |
| `B4` | `products` | `F32 [Q, R]` | `Contraction(contract_axes=[(1,1)])`, query and reference features -> output | All query/reference dot products. |
| `B5` | `distances` | `F32 [Q, R]` | `Elementwise(pairwise_l2_program)`, query norms, reference norms, products -> output | Finite rooted-L2 matrix shared by every target. |

Each output then receives a fixed branch. The following eight tensors are
common to both branches:

| Relative node | Intermediate | Shape and dtype | Primitive and inputs | Meaning |
|---:|---|---|---|---|
| `O0` | `validated_reference` | `F32 [R]` or `I32 [R]` | `Elementwise(finite_identity_program)` or `categorical_code_program(C_i)` | Copies values only after finite or class-range validation. |
| `O1` | `known_count` | `I32 [1]` | `Reduce(Sum, axis=0, keep_dimensions=false)`, known mask -> output | Counts exactly the binary mask entries. |
| `O2` | `validated_count` | `I32 [1]` | `Elementwise(known_count_program(known_references))` | Requires the runtime count to equal the declared count. |
| `O3` | `masked_distances` | `F32 [Q, R]` | `Elementwise(masked_distance_program)`, distances and known mask -> output | Replaces unknown rows with positive infinity. |
| `O4` | `sorted_distances` | `F32 [Q, R]` | Stable ascending `Sort(axis=1, emit_indices=true)` | Distance order for diagnostics and index emission. |
| `O5` | `sorted_indices` | `I32 [Q, R]` | Second output of `O4` | Stable reference-row indices. |
| `O6` | `prefix` | `I32 [K_i]` | `IndexMap(start=0, element_step=1)` | The first `K_i` sorted positions. |
| `O7` | `neighbor_indices` | `I32 [Q, K_i]` | `Gather(axis=1, bounds=Reject)`, sorted indices and prefix -> output | Selected reference row for every query and neighbor slot. |

The numeric branch adds three intermediates and three nodes:

| Relative node | Intermediate or output | Shape and dtype | Primitive and inputs | Meaning |
|---:|---|---|---|---|
| `N0` | `values` | `F32 [Q, K_i]` | `Gather(axis=0, bounds=Reject)`, validated reference and neighbor indices -> output | Selected numeric target values. |
| `N1` | `sums` | `F32 [Q, 1]` | `Reduce(Sum, axis=1, keep_dimensions=true)` | Neighbor sum per query. |
| `N2` | `predictions` boundary | `F32 [Q, 1]` | `Elementwise(finite_divide_program(K_i))` | Unweighted numeric mean. |

The categorical branch adds nine intermediates and one output node after the
common branch. Its fixed sequence is:

| Relative node | Intermediate or output | Shape and dtype | Primitive and inputs | Meaning |
|---:|---|---|---|---|
| `C0` | `codes` | `I32 [Q, K_i]` | `Gather(axis=0)`, validated reference and neighbor indices -> output | Selected class codes. |
| `C1` | `row_bases` | `I32 [Q, 1]` | `IndexMap(start=0, element_step=C_i)` | Base offset for each query's class block. |
| `C2` | `bin_indices` | `I32 [Q, K_i]` | `Elementwise(categorical_bin_program(C_i))` | Flat row/class histogram indices. |
| `C3` | `flat_counts` | `I32 [Q * C_i]` | `Histogram(bins=Q*C_i, weighted=false, ordering=Relaxed)` | Counts selected class codes per query. |
| `C4` | `lookups` | `I32 [Q, C_i]` | `IndexMap(start=0, element_step=1)` | Flat class offsets to gather. |
| `C5` | `counts` | `I32 [Q, C_i]` | `Gather(axis=0)`, flat counts and lookups -> output | Per-query count matrix in class-code order. |
| `C6` | `sorted_counts` | `I32 [Q, C_i]` | Stable descending `Sort(axis=1, emit_indices=true)` | Vote order. |
| `C7` | `sorted_classes` | `I32 [Q, C_i]` | Second output of `C6` | Class codes paired with sorted votes. |
| `C8` | `first` | `I32 [1]` | `IndexMap(start=0, element_step=1)` | Position zero for the winning class. |
| `C9` | `predictions` boundary | `I32 [Q, 1]` | `Gather(axis=1)`, sorted classes and first -> output | Lowest-code stable mode for each query. |

The names above are documentation labels, not graph identities. Actual value
and kernel IDs start at the caller's `IdentityNamespace` and increase in
emission order. The graph's boundary tensors are the caller's values, while
the listed intermediates are operation-owned and counted in the returned
materialization.

Every emitted primitive uses a complete input/output alias matrix with
`AliasPermission::Forbidden`. The operation never overwrites a source or
prediction buffer. `Gather` uses `IndexBounds::Reject`; an invalid index is a
calculation fault rather than a clamped or substituted value.

## Scalar programs and device guards

The helper programs are built through `ScalarProgramBuilder`. Builder or
language failures are wrapped as `GraphMaterializationFailed` by
`graph_error`.

| Program | Inputs and result | Guard and behavior |
|---|---|---|
| `square_program` | one f32 -> one f32 | Requires finite input, squares it, then requires finite output. |
| `pairwise_l2_program` | query norm, reference norm, product -> distance | Requires all inputs finite, computes `norm_q + norm_r - 2*product`, clamps with `Maximum(..., 0)`, takes square root, and requires finite output. |
| `known_count_program(expected)` | one I32 count -> same count | Requires exact equality with `expected`; a wrong runtime mask count fails. |
| `masked_distance_program` | f32 distance and I32 known flag -> f32 | Requires finite distance and flag exactly `0` or `1`; selects distance for one and `f32::INFINITY` for zero. |
| `finite_divide_program(K_i)` | f32 sum -> f32 mean | Requires finite input and finite quotient. The divisor is an f32 constant created only after the exact-domain check. |
| `finite_identity_program` | one f32 -> one f32 | Requires finite input and returns it unchanged. |
| `categorical_code_program(C_i)` | one I32 code -> same code | Requires `0 <= code < C_i`. |
| `categorical_bin_program(C_i)` | row base and code -> flat I32 bin | Reuses the class-range guard, then adds row base and code. |

The graph does not clamp an invalid category, reinterpret a bad mask, replace a
non-finite distance, or recover from a count mismatch. Checked host arithmetic
and device `Require` instructions preserve the first observed failure.

## Request validation and resource admission

`materialize_knn_all_outputs` executes these stages in order:

1. `validate_request` validates every declared tensor and computes the checked
   requirements.
2. `validate_resources` checks value capacity, kernel capacity, and workspace
   limit against that exact result.
3. `KnnAllOutputEmitter::new` validates namespace end arithmetic, normalizes
   boundary flags, and inserts all boundary tensors into a deterministic map.
4. The fixed base graph and one branch per output are emitted.
5. `finish` checks exact counts and bytes, validates the language graph, and
   returns the materialization.

`validate_request` first calls `Tensor::validate` for the two feature matrices
and every per-output tensor. A feature tensor must be rank two, f32, and have a
nonzero checked shape; query and reference widths must match. Reference values,
known masks, and predictions must match the exact table above. The function
collects source IDs in a `BTreeSet`, rejects any prediction that aliases a source
boundary, and rejects duplicate prediction IDs. Reusing an input value for
multiple outputs is allowed only when its storage contracts agree, which is
what `insert_boundary` later checks.

Reduction lanes are rejected unless they are a power of two no greater than
`MAXIMUM_TREE_LANES` (`1024`). This is a graph policy check, not a backend
fallback. Every dimension product and sum uses `checked_product` or
`checked_sum`; overflow is `WorkspaceArithmeticOverflow`.

`validate_resources` reports `IdentityNamespaceExhausted` when the reserved
value or kernel capacity is short, and `WorkspaceLimitExceeded` when the
formula exceeds the caller's byte limit. `KnnAllOutputEmitter::new` separately
rejects a first-ID-plus-capacity overflow and any boundary ID inside the
reserved intermediate value range. Allocation checks the range again for every
intermediate and primitive kernel, so a short range cannot wrap an identity.

## Standalone materialization versus append

### `materialize_knn_all_outputs`

The standalone graph includes all declared boundary tensors, all operation-owned
intermediates, and all emitted nodes. Boundary flags are normalized in the
emitter: query features, reference features, per-output values, and per-output
known masks are external inputs; prediction tensors are external outputs. The
operation-owned byte counter sums each intermediate tensor's `storage_bytes`,
then `finish` requires it to equal the checked formula exactly. The final
`CalculationGraph::validate` checks tensor contracts, producers, primitive
shapes, IDs, aliases, and acyclicity.

### `append_knn_all_outputs`

The append boundary is used by `compile_prepared_knn_inference` after feature
lowering and optional normalization have already populated a caller graph. It
does not silently repair the caller graph:

1. Every caller tensor ID must be unique. A repeated ID is
   `GraphMaterializationFailed`.
2. Every declared source and prediction boundary must already exist in the
   caller graph with the same ID, dtype, shape, layout, and storage byte count.
   A missing or mismatched boundary is `InvalidMaterializationRequest`.
3. Generated intermediate values and kernel IDs must not already occur in the
   caller graph. Either collision is `IdentityNamespaceOverlap`.
4. No caller node may already produce one of the requested predictions. That
   duplicate producer is `GraphMaterializationFailed`.
5. Only operation-owned intermediate tensors and all materialized nodes are
   appended. Boundary tensors are retained from the caller graph.

The caller performs final graph and static-program validation after appending.
There is no alternate operation path for a failed append.

## Operation failure contract

Every failure is an `OperationError` with a specific `OperationErrorKind` and
no operation registry ID. The important KNN checks are:

| Check site | Error kind | Failing condition |
|---|---|---|
| Requirements dimensions | `UnsupportedConcreteShape` | `Q`, `R`, `D`, or `K` is zero, or `R` exceeds the int32 sort-index domain. |
| Requirements outputs | `UnsupportedConcreteShape` | No outputs, known count outside `1..=R`, numeric `K_i` outside the exact f32 integer domain, class count outside `1..=i32::MAX`, or categorical `Q*C_i` above `i32::MAX`. |
| Checked products and sums | `WorkspaceArithmeticOverflow` | A shape, pairwise, histogram, intermediate, or byte calculation overflows u64. |
| Tensor validation | `GraphMaterializationFailed` | A tensor has invalid layout, storage span, shape, or byte count. |
| Feature request shape | `UnsupportedConcreteShape` | A feature tensor is not rank-two f32 or query/reference widths differ. |
| Output request shape | `UnsupportedConcreteShape` | Reference values, masks, or predictions differ from their exact dtype or extent. |
| Prediction/source IDs | `InvalidMaterializationRequest` | A prediction aliases a source boundary or another prediction. |
| Reduction lanes | `InvalidMaterializationRequest` | Lanes are zero, non-power-of-two, or above 1024. |
| Namespace reservation | `IdentityNamespaceExhausted` | Capacity is below the checked count, a range end overflows, or an allocation reaches the end. |
| Boundary IDs | `IdentityNamespaceOverlap` | A boundary ID lies inside the operation's intermediate range. |
| Workspace admission | `WorkspaceLimitExceeded` | Required operation-owned bytes exceed the request limit. |
| Emitter accounting | `WorkspaceFormulaMismatch` | Emitted value count, kernel count, or bytes differ from requirements. |
| Caller graph storage | `GraphMaterializationFailed` | Caller graph repeats a tensor ID. |
| Caller boundary storage | `InvalidMaterializationRequest` | A declared boundary is absent or its ID, dtype, shape, layout, or byte count differs. |
| Caller identity or producer | `IdentityNamespaceOverlap` or `GraphMaterializationFailed` | Generated IDs collide, or a prediction already has a producer. |
| Scalar/language construction | `GraphMaterializationFailed` | A scalar program, shape, axis set, primitive, or final graph is invalid. |

The operation has no retry, alternate sort, host-side reduction, output
substitution, or fallback dtype. Invalid runtime payloads reach scalar
`Require` or checked primitive bounds in the graph.

## Training caller: semantic reference preparation

The public declaration and training path establish the operation's input
meaning before `recipe-ops` is called.

### Declaration restrictions

`Model::knn(neighbors)` stores a `LayerSpec::Knn` with a nonzero `usize`
neighbor count. `Model::validate` requires exactly one KNN layer at index zero,
rejects any activation or normalization operation on that layer, and rejects
composition with other model blocks. The facade also rejects a normalization or
activation method invoked after the terminal KNN layer. These checks ensure the
operation sees one standalone all-output model rather than a dense output head.

`compile_knn_model` in `src/training.rs` then validates policy, data, and model
and rejects Bayesian dependencies, loaded weights, objective or gradient
settings, optimizer or learning-rate controls, warmup or epoch bounds,
iterative logs or plots, and native training-kernel save/resume declarations.
KNN has no optimizer state, loss loop, validation metric loop, or training
kernel. A declared semantic resume path is existence-conditional: an existing
file is loaded, while an absent file starts a fresh reference set.

### `prepare_knn_reference_set`

`training/src/knn.rs` prepares the exact training partition consumed by the
operation:

1. The prepared training partition must be nonempty and have at least one
   declared target.
2. `DenseFeaturePlan::from_prepared` enumerates feature vectors in source order.
   Numeric int32 or f32 features lower to width-one scalar columns. Categorical
   dictionary features lower to one-hot width `dictionary_width + 1`, with the
   final reserved index representing a missing or unseen feature value.
3. `lower_dense_features(..., PartitionKind::Train)` produces finite row-major
   f32 bits in retained training-row order. Int32 conversion must be exact as
   f32, and non-finite f32 bits are rejected.
4. Target source identities are looked up in prepared target vectors in the
   caller's declaration order. Duplicate declarations or a declared target
   missing from prepared vectors are `InvalidTargetMatrix`.
5. One `KnnReferenceOutput` is produced per target. Its known mask retains row
   alignment, and `known_references` is the exact count of ones. A target with
   no known training value is rejected.

Numeric int32 and f32 targets retain finite f32 bit patterns. Missing numeric
rows contain positive zero but have mask zero. Categorical and ordinal targets
use their validated dictionary order as canonical int32 codes. Temporal targets
build a deterministic sorted dictionary from known training values and remap
their int32 values. Text, binary, and image targets build a deterministic sorted
byte dictionary and remap variable-width values. Missing discrete rows contain
code zero and mask zero. Every dictionary is nonempty, unique, and within the
int32 calculation-code domain.

`KnnReferenceSet` retains the nonzero `neighbors`, vector schemas, feature
spans, optional normalization mask, source-row order, reference shape, f32
feature image, and output list. `KnnReferenceOutput::operation_spec` converts
the exact target representation to the operation's numeric or categorical
planning variant. `decode_class` is deliberately a semantic decoder for the
public reporting boundary, not an operation input transformation.

### Normalization state and feature coordinates

`DenseFeaturePlan` marks numeric scalar columns with one and categorical one-hot
columns with zero. The optional `normalization_mask` is present when any
categorical feature exists, and its bits are stored in the semantic artifact.
The KNN feature image is still unnormalized in the reference artifact. The
inference compiler applies the declared normalization to both query and saved
reference matrices in this same fixed coordinate system.

This separation preserves one shared distance matrix while ensuring that a
categorical one-hot coordinate is not treated as a continuous numeric axis.
Feature spans must remain contiguous from zero, cover every saved feature once,
and retain source-vector identity. These invariants are checked again when the
artifact is encoded, decoded, resumed, and bound to query rows.

## Semantic artifact and resume path

`training/src/knn_checkpoint.rs` owns the KNN semantic model around the
operation. `KnnModelArtifact::new` stores format version 1, the immutable
reference set, an optional `DenseDataNormalization`, and the declared dense
operation list, then validates all invariants. `save` accepts only an `.ogdl`
path and atomically writes the canonical textual root `recipe-knn-model`; the
artifact contains no native kernel bytes.

The OGDL image records the neighbor count, normalization declaration, dense
operation declarations, vector schemas, contiguous feature spans, optional
normalization mask, reference shape and source-row order, exact f32 feature
bits, and every output's schema, binary known mask, values, and decoder labels.
`decode_knn_model` applies bounded source, node, payload, metadata, output, and
label limits before constructing the artifact and running `validate_artifact`.
The decoder requires exactly one `recipe-knn-model` root, strict known fields,
nonzero shapes, finite feature and numeric values, binary masks whose count
matches `known_references`, valid class codes, and target schemas that exactly
match the saved vector list.

`KnnModelArtifact::continue_with` validates both artifacts and requires exact
agreement on format, neighbor count, normalization, post-reduction topology,
row-free vector schemas, feature spans, normalization mask, feature width,
output count, output schemas, and declaration order. It then appends current
reference rows after saved rows. Duplicate observations remain because KNN has
no global row identity and multiplicity is part of the observed distribution.
Discrete resume output keeps every saved label code, appends previously unseen
current labels, and remaps current codes to the resulting dictionary. The
appended row order is also the stable distance-tie order.

## Inference caller: building the operation request

`src/inference.rs::compile_inference_package` enforces target-free inference,
loads the bounded `.ogdl` semantic root, distills and selects query rows, and
dispatches `SemanticModelArtifact::Knn` to
`prepare_knn_inference_table` followed by `compile_prepared_knn_inference`.
It never treats a KNN artifact as a dense or Bayesian model.

`prepare_knn_inference_table` derives the saved feature schema from the KNN
vector and span metadata, asks `recipe_ingest` to prepare only those query
features, and rechecks the prepared spans. Query target columns are not
required. Schema, missing-source, and data errors remain preparation errors.

`compile_prepared_knn_inference` is the direct caller of this module. Its
stages are:

1. Reject any nonempty post-KNN operation list. Applying one scalar transform
   to independent numeric and discrete outputs is not defined.
2. Convert query row count, saved reference row count, and feature width to
   checked `u64`; reject zero query rows or an empty saved matrix.
3. Compile query feature values under the saved spans. Numeric int32 values are
   converted on device through the checked f32 program; categorical values are
   one-hot-scattered with the saved reserved route. Saved reference f32 bits are
   admitted as an external `KnnReferenceFeatures` image.
4. Apply optional `DenseDataNormalization` to query and reference features.
   With no normalization or `Identity`, values are unchanged. Z-score computes
   reference column means and variances, MinMax computes reference minima and
   maxima, and L2Norm computes per-row norms. The source uses epsilon `1e-6`
   and the saved feature mask so categorical one-hot coordinates are not
   normalized as continuous values. All these are graph calculations.
5. For each saved output, admit an external `KnnReferenceKnown` mask and either
   an f32 `KnnReferenceValues` image or an int32 `KnnReferenceValues` code image.
   Allocate one `[Q, 1]` prediction tensor and retain a
   `KnnInferenceOutputContract` with saved source-vector identity and
   `NumericMean` or `DiscreteMode` kind.
6. Convert the saved outputs to `KnnOutputSpec`, call
   `knn_all_output_requirements`, and reserve exactly the returned value and
   kernel capacities starting at the compiler's next IDs. The request's
   workspace limit is exactly `requirements.workspace_bytes`, and tree lanes
   are `MAXIMUM_REDUCTION_TREE_LANES` (`1024`).
7. Mark every external input and output contract, build the caller graph from
   feature and normalization nodes, and call `append_knn_all_outputs`.
8. Assign every returned kernel `IterationDomain::first()`, validate the graph,
   canonicalize and reparse it through OGDL, and wrap it in a static program
   with exactly one nonzero iteration.

The compiler exposes three KNN input role families for each output: reference
features, reference values or codes, and reference known masks. The operation
receives the tensors after all host schema work but before native admission.

## Native execution and output collection

`training/src/execute.rs::prepare_and_execute_local_knn_inference` is the real
runtime boundary after graph compilation. It validates the compiled KNN
boundary, prepares the measured native system, requires one loop iteration,
rejects loop-time external transfers and user metrics, packs the finalized init
images, and maps every declared prediction to an exit transfer. It then runs
the normal recoverable `init -> loop -> exit` lifecycle on the selected native
backend. The operation module itself does not know which backend executes its
primitives.

The execution validator requires:

- exactly one static program iteration and no metric emissions;
- a graph that passes `CalculationGraph::validate`;
- one unique semantic role and one exact byte image for every external input;
- a declared input set equal to the graph's external-input tensor set;
- canonical contiguous row-major boundary layouts;
- at least one prediction, with unique value IDs and unique saved source-vector
  identities;
- prediction dtype `F32` for `NumericMean` or `I32` for `DiscreteMode`, shape
  `[Q, 1]`, external-output status, no input alias, and an actual producer;
- a graph external-output set equal to the declared prediction set.

`map_knn_inference_outputs` requires every output plan to be an exit-phase
transfer to the external endpoint. It rejects duplicate or unexpected tasks,
source-device/value mismatches, dtype or byte-size mismatches, missing plans,
extra exit transfers, and overlapping output images. After the backend exits,
`collect_knn_inference_predictions` repeats the contract and image checks and
returns one `KnnInferencePrediction` per saved target in declaration order.
Each prediction retains its `KnnInferenceOutputContract` and exact little-endian
bytes in a `CompletedKnnInferenceExecution`.

## Public report and end-to-end role

`src/inference.rs::evaluate_inference_declaration` keeps the semantic KNN
artifact beside the completed typed execution in `InferenceReportPayload::Knn`.
The report's `prediction()` and `values()` accessors intentionally return no
singular f32 matrix for KNN. Consumers use `knn_predictions()` for the mixed
output vector and `decode_knn_class(output, code)` for a saved discrete label.
Numeric output has no class decoder.

`write_knn_prediction_rows` checks that execution output count equals saved
target count, that each prediction has shape `[rows, 1]`, and that each
prediction's saved source identity matches its artifact output. It writes one
row per query and output. Numeric rows decode the four bytes as f32 and print a
`value`. Discrete rows decode int32, resolve the code through the saved
dictionary, and print both `class` and `label`. An unknown code or source
identity is an `InvalidData` output error, not a replacement label.

The complete current path is therefore:

```text
public .data(...).target([...]).norm(...).split(...).model().knn(K)
    -> Model validation and compile_knn_model
    -> prepare_data and prepare_knn_reference_set
    -> KnnModelArtifact validation and optional .ogdl save/resume
    -> target-free inference model load and prepare_knn_inference_table
    -> compile_prepared_knn_inference
    -> feature lowering and optional normalization graph
    -> append_knn_all_outputs
    -> graph/static-program validation and one iteration
    -> measured native init, loop, exit
    -> typed prediction collection
    -> KNN report accessors and row logging
```

The operation contributes only the shared distance and independent output
calculation segment. Dataset admission, semantic artifact persistence, native
realization, transfers, lifecycle state, and public decoding remain explicit
neighboring boundaries.

## Caller and callee failure classes

Failures before and after `recipe-ops` retain their typed owner. They are not
converted into an operation error unless the operation call itself fails.

| Boundary | Relevant failure conditions |
|---|---|
| Public declaration | `DeclarationError` for zero neighbors, a KNN layer composed with another layer, or an activation/normalization operation after terminal KNN. |
| Training policy | `TrainingError::Unsupported` for objectives, gradients, optimizer controls, iterative metrics, Bayesian dependencies, loaded weights, or native training-kernel declarations. |
| Training semantic preparation | `TrainingCompileErrorKind::EmptyDataset`, `InvalidTargetMatrix`, `InvalidFeatureMatrix`, `UnsupportedExtent`, or `ArithmeticOverflow` for empty partitions, target order drift, incompatible semantic tuples, missing values, invalid dictionaries/codes, non-finite values, inexact int32-to-f32 conversions, or shape overflow. |
| Model save and resume | `CheckpointError` for a non-`.ogdl` target, invalid artifact, bounded source or decode failure, incompatible topology/schema, or append/reservation overflow. |
| Target-free query preparation | `InferencePreparationError` for target-bearing data, split or redeclared normalization, absent/wrong saved features, malformed values, or inconsistent spans. |
| Inference graph compilation | `InferenceCompileErrorKind::EmptyDataset`, `InconsistentCheckpoint`, `UnsupportedExtent`, `ArithmeticOverflow`, `Operation`, `Language`, `Program`, or `Ogdl`. The `Operation` variant carries this module's `OperationErrorKind`. |
| Native handoff and lifecycle | `InferenceExecutionError` for invalid graph or boundary, wrong loop count, metric or transfer policy, preparation/handoff failure, executor failure, or failure to reach exit. |
| Exit output collection | `InferenceExecutionError` for missing, duplicate, unexpected, overlapping, wrong-source, wrong-dtype, or wrong-size output images. |
| Public reporting | `io::ErrorKind::InvalidData` for output count, shape, source identity, or unknown discrete code mismatch. |

No boundary catches a failure to select a different KNN implementation. The
observed invalid transition remains visible at the boundary that owns it.

## Non-goals and maintained invariants

This source does not imply any behavior beyond the observed C33 contract:

- no optimizer, gradient, loss, epoch loop, validation metric, or native
  training kernel;
- no host-side distance, mask count, histogram, mode, mean, or label inference;
- no numeric transformation after independent KNN outputs;
- no concatenation of heterogeneous targets into one semantic tensor;
- no implicit treatment of missing values as known values;
- no unstable tie rule, arbitrary dictionary reordering, or duplicate-row
  deduplication;
- no backend-specific primitive, vendor math library, retry, or substitute
  output path.

The maintained invariants are concrete and cross-boundary:

1. Query and reference features share one finite f32 coordinate system and width.
2. Prepared reference row order is retained through artifact, graph sort, and
   resume append, defining stable distance ties.
3. Each output has an independent binary mask whose count equals its declared
   known-reference count.
4. Numeric values remain finite f32 and categorical values remain validated
   int32 codes with an exact decoder.
5. Every output's effective neighbor count is positive and no larger than its
   known references.
6. All graph IDs, storage contracts, aliases, producers, and workspace totals
   are checked before native admission.
7. Native execution performs one static iteration with init admission and exit
   egress, and public reporting preserves output declaration order and dtype.

## Source map

| Evidence | Location |
|---|---|
| KNN request/spec/materialization types | `ops/src/knn_outputs.rs:21-155` |
| Checked dimensions, counts, and workspace formula | `ops/src/knn_outputs.rs:157-301` |
| Shared distance and numeric/categorical graph emission | `ops/src/knn_outputs.rs:303-491` |
| Append boundary and caller graph collision checks | `ops/src/knn_outputs.rs:493-559` |
| Request, dtype, shape, alias, lane, and resource validation | `ops/src/knn_outputs.rs:561-681` |
| Emitter identity, intermediate accounting, graph finish, and boundary storage | `ops/src/knn_outputs.rs:683-974` |
| Scalar programs, forbidden aliases, checked arithmetic, and error wrapping | `ops/src/knn_outputs.rs:976-1180` |
| Operation error kinds and typed result | `ops/src/error.rs:5-66` |
| Public KNN layer declaration and terminal restrictions | `src/api.rs:757-817`, `src/api.rs:1297-1307`, `src/api.rs:1426-1457`, `src/api.rs:1462-1496`, `src/api.rs:1584-1599` |
| KNN training policy and artifact compilation | `src/training.rs:417-498` |
| Typed target/reference preparation and masks | `training/src/knn.rs:16-216`, `training/src/knn.rs:218-372`, `training/src/knn.rs:375-568`, `training/src/knn.rs:570-660` |
| Dense feature spans, one-hot lowering, and normalization mask | `training/src/model.rs:960-1003`, `training/src/model.rs:1145-1240`, `training/src/model.rs:1908-2045` |
| KNN OGDL artifact, validation, resume, and save | `training/src/knn_checkpoint.rs:55-148`, `training/src/knn_checkpoint.rs:345-630`, `training/src/knn_checkpoint.rs:636-805` |
| KNN model decoding and output codecs | `training/src/knn_checkpoint.rs:816-1080`, `training/src/knn_checkpoint.rs:1389-1505` |
| Prepared KNN inference table and model-root dispatch | `training/src/inference.rs:696-780`, `training/src/inference.rs:783-842` |
| Feature lowering and optional KNN normalization | `training/src/inference.rs:2107-2228`, `training/src/inference.rs:2588-2799` |
| Direct operation caller and static one-iteration program | `training/src/inference.rs:1566-1790` |
| Compiled KNN output contracts | `training/src/inference.rs:520-631` |
| Native KNN execution lifecycle and typed predictions | `training/src/execute.rs:587-655`, `training/src/execute.rs:1312-1427` |
| Native boundary validation and exit-output mapping | `training/src/execute.rs:1648-1833`, `training/src/execute.rs:2636-2829` |
| Public model dispatch and KNN report accessors | `src/inference.rs:228-423`, `src/inference.rs:432-480`, `src/inference.rs:500-543`, `src/inference.rs:602-695` |
| Public KNN row reporting and dictionary decoding | `src/inference.rs:990-1058` |
| Public KNN workflow | `examples/cookbook.rs:230-243` |
| Normative C33 contract | `system-contract.md:562-587` |
