<!--
Intent: describe the complete source-qualified indexing, sorting, and encoding
materialization family. The module turns prepared tensor tables and typed facts
into one validated, placement-free calculation graph. It does not execute a
legacy kernel, choose hardware, allocate device memory, or infer an ABI from a
symbol alone.
-->

# `ops/src/materialize/indexing_sort_encoding.rs`

## Module identity

```text
crate: recipe-ops
source: ops/src/materialize/indexing_sort_encoding.rs
family: indexing, sort, and encoding
owned_entries: 17 exact (symbol, source) pairs
payload: canonical f32 and int32 tensors
graph: recipe_language::CalculationGraph
runtime_index_policy: IndexBounds::Reject, except one intentional Clamp sentinel gather
scatter_policy: ScatterConflict::UniqueIndices
sort_policy: ascending, stable, emitted original indexes
```

[`indexing_sort_encoding.rs`](../../../src/materialize/indexing_sort_encoding.rs)
is the concrete materializer for the source-qualified operations listed below.
The old `gpu-core` paths in the `source` column are preserved operation
identities. Their executable semantics are not called at runtime. This module
checks the ABI and preparation facts, builds Recipe-owned scalar programs and
primitive nodes, and returns a static graph fragment through
`materialize_composition`.

The module owns no host loop over payload elements. A loop in
`emit_segment_sum` is represented by the fixed three-stage recipe and the
primitive graph. The caller has already prepared immutable index tables,
coordinate tables, and verification facts before this boundary is crossed.

## Registry, recipe, and dispatch ownership

`operation_registry()` turns the generated operation-surface entries into
`OperationDescriptor` values. For these rows, `registry::lowering` selects the
matching `CompositionRecipe`; `CompositionPayload::F32AndI32` supplies the
canonical dtype contract and the operation family is either
`ShapeAndIndexing`, `Encoding`, or `Reduction`. The registry retains the
legacy source path in every descriptor. `resolve_unique(symbol)` is safe for
the current rows because each of the 17 symbols occurs once in
`operation-surface.txt`; `resolve_exact(symbol, source)` remains the
source-qualified API for callers that have both fields.

`materialize_composition` in [`materialize.rs`](../../../src/materialize.rs)
performs the common boundary in this order:

1. Validate the descriptor, named tensor declarations, external flags, and
   tensor layouts.
2. Reject descriptors without a concrete family materializer.
3. Resolve the declared iteration-shape input and expand the finite
   `CompositionRecipe` into `ResolvedStep` values.
4. Allocate an `Emitter` from the caller's `IdentityNamespace`.
5. Probe family modules in fixed order. This module is probed after
   `loss_metrics` and before `graph_cluster_rl`.
6. Require each emitted stage's final primitive family to equal the next
   resolved step, then finish with graph validation and exact workspace
   accounting.

The family dispatcher first calls `supports(request.descriptor)`. `supports`
uses `OPERATIONS.contains`, so both symbol and source must match. Only after
that exact test does `dispatch` match the symbol and call a concrete emitter.
An operation with the right symbol and another source is `NotOwned`; a pair in
`OPERATIONS` without a symbol branch returns `GraphMaterializationFailed`.
This prevents a semantically adjacent legacy operation from silently using a
different ABI.

## The 17 paired source cases

The table is the complete `OPERATIONS` registry in source order. Recipe step
names are the descriptive inventory in `composition.rs`; the helper column is
the actual branch in this file.

| Symbol | Preserved source pair | Registry recipe and family | Composition steps | Concrete branch |
| --- | --- | --- | --- | --- |
| `gpu_add_col` | `gpu-core/src/kernels.rs:6839` | `column_add`, `ShapeAndIndexing` | `GATHER -> MAP -> SCATTER` | `emit_column_update(..., Add)` |
| `gpu_add_col_scaled_inplace` | `gpu-core/src/kernels.rs:4525` | `scaled_column_accumulate`, `ShapeAndIndexing` | `GATHER -> MAP -> SCATTER` | `emit_column_update(..., ScaledAdd)` |
| `gpu_add_diag` | `gpu-core/src/kernels.rs:2298` | `diagonal_add`, `ShapeAndIndexing` | `GATHER -> MAP -> SCATTER` | `emit_diagonal_add` |
| `gpu_argsort` | `gpu-core/src/reductions.rs:458` | `argsort`, `ShapeAndIndexing` | `SORT -> GATHER` | `emit_argsort` |
| `gpu_bin_edges_uniform` | `gpu-core/src/encoding.rs:96` | `uniform_bin_edges`, `Encoding` | `MAP` | `emit_uniform_edges_single_row` |
| `gpu_concat_into` | `gpu-core/src/kernels.rs:5361` | `tensor_concatenation`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_row_concatenation` |
| `gpu_one_hot` | `gpu-core/src/encoding.rs:169` | `one_hot_encoding`, `Encoding` | `MAP -> SCATTER` | `emit_one_hot` |
| `gpu_pack_upper_tri` | `gpu-core/src/kernels.rs:5563` | `pack_upper_triangle`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_dense_upper_triangle` |
| `gpu_partial_argsort` | `gpu-core/src/kernels.rs:5267` | `bounded_topk_indexes`, `ShapeAndIndexing` | `SORT -> GATHER` | `emit_partial_argsort` |
| `gpu_segment_sum` | `gpu-core/src/reductions.rs:627` | `segmented_reduction`, `Reduction` | `SORT -> SCAN -> REDUCE` | `emit_segment_sum` |
| `gpu_slice_cols` | `gpu-core/src/kernels.rs:6550` | `checked_tensor_slice`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_column_slice` |
| `gpu_slice_lead_into` | `gpu-core/src/kernels.rs:5390` | `checked_tensor_slice`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_leading_slice` |
| `gpu_slice_rows` | `gpu-core/src/kernels.rs:5619` | `checked_tensor_slice`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_row_slice` |
| `gpu_topk_per_row` | `gpu-core/src/kernels.rs:4712` | `bounded_topk_indexes`, `ShapeAndIndexing` | `SORT -> GATHER` | `emit_topk_per_row` |
| `gpu_transpose` | `gpu-core/src/kernels.rs:5540` | `tensor_transpose`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_transpose` |
| `gpu_tril_mask` | `gpu-core/src/kernels.rs:6578` | `lower_triangular_mask`, `ShapeAndIndexing` | `MAP` | `emit_triangular_mask` |
| `gpu_vconcat` | `gpu-core/src/kernels.rs:6514` | `tensor_concatenation`, `ShapeAndIndexing` | `GATHER -> MAP` | `emit_vector_concatenation` |

The recipe only describes primitive-family order. It does not supply tensor
names, shapes, scalar formulas, table meanings, or workspace policy. Those
contracts are the responsibility of the concrete branches below.

## Shared request and graph state

Every branch receives `MaterializationRequest` and a mutable `Emitter`.
`MaterializationRequest` carries one `OperationDescriptor`, exact named input
and output declarations, the name of the input whose shape resolves recipe
bounds, a `PreparedParameters` map, an `IdentityNamespace`, and a
`ByteCount` workspace limit. `PreparedParameter` can only be `U64`, `I32`,
`F32Bits`, or `Bool`.

`require_exact_abi` compares input names, output names, and parameter keys as
sets. Missing names, extra names, or a wrong name spelling fail before any
graph node is emitted. `positive_parameter` accepts only a `U64` greater than
zero. `checked_product` uses checked `u64` multiplication and reports
`WorkspaceArithmeticOverflow`; `require_i32_indexable` additionally requires
the computed extent to be in `1..=2_147_483_647`.

The emitter allocates every intermediate as a contiguous `Tensor` with its
caller-reserved value identity. `GraphBuilder::intermediate` adds its exact
storage bytes to the workspace total and enforces the request limit. Every
ordinary emitted kernel receives a complete forbidden alias matrix, including
the branch for the legacy `gpu_add_col_scaled_inplace` name. The descriptor's
registry alias contract is metadata; the concrete graph still has distinct
boundary and intermediate tensors and cannot silently overwrite a caller
input.

`Emitter::emit_stage` permits several concrete kernels in one resolved stage.
It records all of their IDs but checks the family of the last kernel against
the recipe step. This is why the segment scan stage can contain index-map,
gather, and elementwise setup kernels while ending in `Scan`, and why the
concatenation gather stage contains two gathers.

## Column updates and diagonal updates

### `gpu_add_col` and `gpu_add_col_scaled_inplace`

Both branches require the following exact ABI:

```text
inputs:  matrix[F32, [rows*columns]], column[F32, [rows]],
         column_indices[I32, [rows]]
output:  updated_matrix[F32, [rows*columns]]
parameters: rows[U64], columns[U64], column_index[U64],
            column_indices_verified_unique[Bool]
```

`rows` and `columns` are positive. `column_index < columns` is checked during
preparation, the matrix element product must fit checked int32 indexing, and
the uniqueness fact must be true. The scaled variant adds one exact input:
`scale[F32, [1]]`. The source table `column_indices` is expected to contain
the selected column's flattened matrix coordinates, one per row; the table is
not regenerated by this materializer.

The graph for both variants is:

```text
selected = Gather(axis=0, Reject, matrix, column_indices)
updates  = Elementwise(selected + column)                    # gpu_add_col
updates  = Elementwise(selected + column * scale)            # scaled variant
updated_matrix = Scatter(axis=0, Reject, UniqueIndices,
                         matrix, column_indices, updates)
```

The scatter copies `matrix` as its base image, then writes the disjoint update
coordinates. The result is therefore a full matrix, not only a column. An
out-of-range table entry faults through the checked gather or scatter channel;
the true uniqueness fact is what makes `UniqueIndices` valid.

Workspace is `8 * rows` bytes: one f32 image for `selected` and one for
`updates`. The output and all boundary tensors are not workspace objects.

### `gpu_add_diag`

The exact ABI is:

```text
inputs:  matrix[F32, [dimension*dimension]],
         diagonal_value[F32, [1]],
         diagonal_indices[I32, [dimension]]
output:  updated_matrix[F32, [dimension*dimension]]
parameters: dimension[U64], diagonal_indices_verified_unique[Bool]
```

`dimension` is positive, its square must fit checked int32 indexing, and the
uniqueness fact must be true. The concrete graph gathers the diagonal entries,
maps `entry + diagonal_value`, and scatters the updates over the original
matrix with `Reject` bounds and `UniqueIndices` conflict policy. Preparation
normally supplies `row*dimension + row` coordinates, but the graph only trusts
the supplied table and verification fact.

The two f32 intermediates have shape `[dimension]`, so workspace is
`8 * dimension` bytes.

## Stable sorting and index selection

### `gpu_argsort`

```text
inputs:  values[F32, [elements]],
         exposure_indices[I32, [elements]]
output:  indices[I32, [elements]]
parameters: elements[U64], exposure_indices_verified_identity[Bool]
```

`elements` is positive and int32-indexable. The exposure table must be the
verified identity table. `emit_sorted_prefix` emits a stable ascending
`Sort(axis=0, emit_indices=true)` producing `sorted_values[F32]` and
`sorted_indices[I32]`, then gathers `sorted_indices` with
`exposure_indices` into `indices`. The concrete order is therefore the
original index of each value in ascending stable order. Stable sorting retains
original order for equal values, and the primitive's total-order contract
handles the complete binary32 ordering.

Workspace is `8 * elements` bytes.

### `gpu_partial_argsort`

The ABI is `values[F32, [elements]]` and
`prefix_indices[I32, [prefix]]` to `indices[I32, [prefix]]`, with exact
parameters `elements[U64]`, `prefix[U64]`, and
`prefix_indices_verified[Bool]`. The implementation intentionally accepts
only `prefix == elements`. In that one case it emits exactly the same complete
stable sort and gather as `gpu_argsort`. Any other prefix, including a shorter
or larger one, returns `UnsupportedConcreteShape` with the detail that the
legacy operation writes the complete sorted index vector and a shorter prefix
needs a distinct source operation. Workspace is `8 * elements` bytes.

### `gpu_topk_per_row`

```text
inputs:  values[F32, [rows, columns]], prefix_indices[I32, [k]]
output:  indices[I32, [rows, k]]
parameters: rows[U64], columns[U64], k[U64],
            prefix_indices_verified[Bool]
```

All three extents are positive, `k <= columns`, `columns` fits checked int32
indexing, and the prefix fact must be true. The graph performs a stable
ascending sort on axis 1, producing full matrix `sorted_values` and
`sorted_indices`, then gathers axis 1 at the same prepared `prefix_indices`
for every row. Despite the legacy name, the actual materializer selects the
prepared ascending prefix, not a descending top-k. Runtime prefix entries use
`IndexBounds::Reject` and must be valid column positions.

Both sorted matrices are f32 or int32 respectively and consume
`8 * rows * columns` workspace bytes.

## Uniform bin edges

### `gpu_bin_edges_uniform`

The exact ABI is:

```text
inputs:  feature_values[F32, [columns, 1]],
         bin_indices[F32, [1, bins+1]]
output:  edges[F32, [columns, bins+1]]
parameters: rows[U64], columns[U64], bins[U64],
            bin_indices_verified[Bool]
```

`rows`, `columns`, and `bins` are positive, and `rows` must equal one. A
request with another row count returns `UnsupportedConcreteShape` because the
normative concrete graph has no reduction stage for multiple rows. `bins + 1`
uses checked arithmetic and `bins` must be exactly representable as f32, which
limits it to
`16_777_216`. The bin-table fact must be true; the branch does not require
the input values to be finite.

The single elementwise program is the actual source of the result. For each
feature value `v` and bin coordinate `b`, it computes

```text
difference = v - v
width      = difference / f32(bins)
edge       = v + bin_indices[b] * width
```

Thus finite `v` produces `edge == v` for every bin. This is the current
materialized semantics, not the descriptive recipe's intended min-to-max
interpolation: there is no max input, and the scalar program subtracts the
same value from itself. NaN input remains NaN through the arithmetic. The
elementwise node allocates no intermediate workspace.

## Table-driven concatenation

### `gpu_concat_into` and `gpu_vconcat`

Both symbols share `emit_concatenation` but have different parameter ABIs.
`gpu_concat_into` requires:

```text
inputs:  left, right, left_indices[I32], right_indices[I32],
         select_left[I32]
output:  concatenated
parameters: rows[U64], left_columns[U64], right_columns[U64],
            concatenation_tables_verified[Bool]
```

The flattened shapes are `[rows*left_columns]`, `[rows*right_columns]`, and
`[rows*(left_columns+right_columns)]` for the two inputs and every table/output
respectively. `gpu_vconcat` uses the same tensor names and uses
`left_elements[U64]`, `right_elements[U64]`, and
`concatenation_tables_verified[Bool]`; the three source and output shapes are
`[left_elements]`, `[right_elements]`, and
`[left_elements+right_elements]`. All products and the vector sum are checked.

`left` may be f32 or int32. `right` and `concatenated` must have the same dtype
as `left`; both index tables and `select_left` are int32. The graph first
performs two checked axis-zero gathers:

```text
gathered_left  = Gather(left,  left_indices)
gathered_right = Gather(right, right_indices)
concatenated   = Select(select_left, gathered_left, gathered_right)
```

The scalar `Select` treats zero as false and any nonzero int32 value as true.
Both gathers execute and both tables must be in range even when one side is
not selected. The verification fact is a preparation assertion about the
tables, not a runtime substitute for `IndexBounds::Reject`.

Two result-sized intermediates consume `8 * total_elements` bytes. The
registry marks `_into` operations as no-alias by the general suffix rule, and
the emitter also forbids every input/output alias in the concrete graph.

## One-hot encoding

### `gpu_one_hot`

```text
inputs:  labels[I32, [rows]], destination_indices[I32, [rows]],
         output_base[F32, [rows*classes]]
output:  encoded[F32, [rows*classes]]
parameters: rows[U64], classes[U64],
            destination_indices_verified_unique[Bool]
```

`rows` and `classes` are positive, the product is int32-indexable, and the
destination uniqueness fact must be true. Each label is checked in the scalar
program with `Require(0 <= label < classes)`. The graph first maps the base to
zero and maps each valid label to f32 one, then performs

```text
encoded = Scatter(axis=0, bounds=Reject, conflict=UniqueIndices,
                  cleared_base, destination_indices, one_updates)
```

The scatter copies the cleared base, so all unwritten elements remain zero.
An invalid label raises the scalar fault; an invalid destination raises the
checked scatter fault. The two intermediates consume
`4 * (rows*classes + rows)` bytes.

## Dense upper-triangle packing

### `gpu_pack_upper_tri`

```text
inputs:  factor[F32, [factor_rows*dimension]],
         factor_indices[I32, [dimension*dimension]],
         upper_mask[I32, [dimension*dimension]],
         zero[F32, [1]]
output:  upper_triangle[F32, [dimension*dimension]]
parameters: factor_rows[U64], dimension[U64],
            factor_indices_verified[Bool], upper_mask_verified[Bool],
            zero_verified[Bool]
```

Both dimensions are positive and `factor_rows >= dimension`. The factor
product must fit checked int32 indexing. All three verification facts are
required. The graph gathers the factor at each prepared output coordinate,
then selects gathered data when `upper_mask` is nonzero and `zero` otherwise:

```text
gathered = Gather(axis=0, Reject, factor, factor_indices)
upper_triangle = Select(upper_mask, gathered, zero)
```

`factor_indices` normally describes the canonical row-major packed upper
triangle, but only the supplied table is consumed. The gathered f32 image is
the only intermediate, so workspace is `4 * dimension * dimension` bytes.

## Deterministic segmented sum

### `gpu_segment_sum`

```text
inputs:  values[F32, [elements]], segment_ids[I32, [elements]]
output:  sums[F32, [segments]]
parameters: elements[U64], segments[U64],
            maximum_segment_length[U64], tree_lanes[U64]
```

All parameters are positive. `maximum_segment_length <= elements`, and
`elements`, `segments`, and
`segments * maximum_segment_length` must each fit checked int32 indexing.
`tree_lanes` must be a power of two in `1..=1024`. The implementation does
not clamp an invalid segment ID or silently truncate an overlong segment;
device-side `Require` and checked scatter/gather faults expose those failures.

The resolved `segmented_reduction` recipe has three stages. The concrete
stages contain the following nodes, with all intermediate tensors in int32 or
f32 as shown:

| Stage | Concrete nodes | Purpose |
| --- | --- | --- |
| `SORT` | stable ascending `Sort(axis=0)` from `segment_ids` to `sorted_ids` and `sorted_original_indices` | Group equal segment IDs while retaining original value order. |
| `SCAN` | `positions = IndexMap(0,1)`, `previous_positions = IndexMap(-1,1)`, `previous_ids = Gather(Clamp)`, boundary elementwise map, inclusive `Scan(Maximum)` to `segment_starts` | Mark each first position or ID transition and prefix the start position of its segment. The clamp is only for the position-zero sentinel. |
| `REDUCE` | gather `sorted_values`; elementwise checked destination map; packed position `IndexMap`; zero-base map; unique scatter into packed workspace; matrix-position `IndexMap`; gather to `[segments, maximum_segment_length]`; `Reduce(Sum, axis=1, keep_dimensions=false, tree_lanes)` | Place every sorted value in its bounded segment slot and reduce each segment. |

For sorted position `p`, the boundary map emits `p` at the first position of a
segment and zero elsewhere. The inclusive maximum scan therefore gives
`segment_starts[p]`. The destination map computes

```text
local       = p - segment_starts[p]
destination = segment_id * maximum_segment_length + local
```

and requires `0 <= segment_id < segments` and
`0 <= local < maximum_segment_length`. Consequently, for valid prepared
tables the result is the deterministic segment sum

```text
sums[s] = sum(values[i] for i where segment_ids[i] == s)
```

in stable sorted order and with the fixed reduction tree. Segment IDs with no
rows reduce the zero-filled packed row. A group longer than
`maximum_segment_length` faults in the destination scalar program instead of
being truncated.

Let `e = elements` and `g = segments * maximum_segment_length`. The graph
allocates nine `e`-sized intermediates and five `g`-sized intermediates:

```text
workspace_bytes = 9 * 4 * e + 5 * 4 * g
                 = 36 * e + 20 * g
```

The final `sums` tensor is a caller-owned output and is not charged to
workspace.

## Checked slices and transpose

The three slice operations and transpose share `emit_flat_gather_map`. The
helper chooses table and output names from the exact symbol, so it cannot be
used by an unlisted operation.

### `gpu_slice_cols`

```text
inputs:  values[F32, [rows*columns]], slice_indices[I32, [rows*count]]
output:  sliced[F32, [rows*count]]
parameters: rows[U64], columns[U64], start[U64], count[U64],
            slice_indices_verified[Bool]
```

The range check requires `start + count <= columns`; source and result
products must fit checked u64 arithmetic, and the source extent must fit
int32 indexing.

### `gpu_slice_lead_into`

```text
inputs:  values[F32, [rows*source_columns]],
         slice_indices[I32, [rows*take]]
output:  sliced[F32, [rows*take]]
parameters: rows[U64], source_columns[U64], take[U64],
            slice_indices_verified[Bool]
```

`take` is positive and is checked against `source_columns` as the range
`0 + take`. The `_into` suffix does not authorize aliasing in this graph.

### `gpu_slice_rows`

```text
inputs:  values[F32, [rows*columns]], slice_indices[I32, [count*columns]]
output:  sliced[F32, [count*columns]]
parameters: rows[U64], columns[U64], start[U64], count[U64],
            slice_indices_verified[Bool]
```

The range check requires `start + count <= rows`.

### `gpu_transpose`

```text
inputs:  values[F32, [rows*columns]],
         transpose_indices[I32, [rows*columns]]
output:  transposed[F32, [rows*columns]]
parameters: rows[U64], columns[U64], transpose_indices_verified[Bool]
```

The transpose branch validates positive `rows` and `columns`, then consumes a
prepared flat permutation table. It does not derive a two-dimensional
coordinate formula from the parameters. The table-driven graph for all four
operations is:

```text
gathered = Gather(axis=0, bounds=Reject, values, table)
result   = Elementwise(identity_f32(gathered))
```

The identity map is deliberate. It preserves the `GATHER -> MAP` recipe stage
and gives the output its own forbidden-alias tensor. Workspace is four bytes
per result element: `4*rows*count`, `4*rows*take`, `4*count*columns`, or
`4*rows*columns`, respectively.

## Lower triangular mask

### `gpu_tril_mask`

```text
inputs:  row_indices[I32, [dimension, dimension]],
         column_indices[I32, [dimension, dimension]],
         fill_value[F32, [1]]
output:  mask[F32, [dimension, dimension]]
parameters: dimension[U64], coordinate_tables_verified[Bool]
```

`dimension` is positive and fits checked int32 indexing. The coordinate-table
fact must be true. The one elementwise program computes

```text
mask[row, column] = if column <= row { 0.0 } else { fill_value }
```

using the supplied coordinate values. This is a lower-triangle retention mask
with the upper triangle filled, not a mask that writes one on retained
elements. It allocates no intermediate workspace. The branch includes a
redundant positive-element and output-element consistency check after shape
validation; a failure is `InvalidMaterializationRequest`.

## Scalar programs and primitive safety

The family uses only Recipe-owned scalar SSA builders. The relevant formulas
are:

| Program | Inputs and checks | Result |
| --- | --- | --- |
| `add_program` | two f32 values | `left + right` |
| `scaled_add_program` | f32 old value, column, and scale | `old + column * scale` in that operation order |
| `segment_boundary_program` | int32 current ID, previous ID, position | `position` at the first lane or an ID change, otherwise zero |
| `segment_destination_program` | int32 segment, position, segment start | `Require` bounded segment and local position, then `segment*maximum_segment_length + local` |
| `zero_from_i32_program` | int32 position | convert to f32, then select constant zero |
| `uniform_single_row_program` | f32 value and f32 bin index | `value + bin_index * ((value-value)/bins)` |
| `select_program` | int32 condition and two equal-typed payloads | nonzero condition selects the second payload, zero selects the third |
| `clear_program` | f32 input | constant zero through an unconditional select |
| `checked_one_program` | int32 label | `Require(0 <= label < classes)`, then f32 one |
| `triangular_mask_program` | int32 row/column and f32 fill | zero when `column <= row`, otherwise fill |

`ScalarOpcode::Select` is not a host branch: int32 zero selects the third
operand and any nonzero value selects the second. `Require` reports a device
calculation fault through the preallocated fault channel. Primitive gathers and
unique scatters use `IndexBounds::Reject`, so malformed prepared tables never
become unchecked memory accesses. The segment predecessor gather is the sole
`Clamp` use and clamps only the synthetic `-1` position for lane zero.

## Workspace, identities, and result state

The following formulas count only operation-owned intermediate tensors. Every
intermediate is contiguous and four bytes per element for f32 or int32. The
caller-owned inputs and outputs, including base images used by scatter, are not
workspace objects.

| Operation(s) | Intermediate element count | Workspace bytes |
| --- | ---: | ---: |
| `gpu_add_col`, `gpu_add_col_scaled_inplace` | `2*rows` | `8*rows` |
| `gpu_add_diag` | `2*dimension` | `8*dimension` |
| `gpu_argsort`, `gpu_partial_argsort` | `2*elements` | `8*elements` |
| `gpu_bin_edges_uniform`, `gpu_tril_mask` | `0` | `0` |
| `gpu_concat_into`, `gpu_vconcat` | `2*total_elements` | `8*total_elements` |
| `gpu_one_hot` | `rows*classes + rows` | `4*(rows*classes + rows)` |
| `gpu_pack_upper_tri` | `dimension*dimension` | `4*dimension*dimension` |
| `gpu_segment_sum` | `9*elements + 5*segments*maximum_segment_length` | `36*elements + 20*segments*maximum_segment_length` |
| `gpu_slice_cols` | `rows*count` | `4*rows*count` |
| `gpu_slice_lead_into` | `rows*take` | `4*rows*take` |
| `gpu_slice_rows` | `count*columns` | `4*count*columns` |
| `gpu_topk_per_row` | `2*rows*columns` | `8*rows*columns` |
| `gpu_transpose` | `rows*columns` | `4*rows*columns` |

`GraphBuilder` checks each addition against `workspace_limit`, and allocation
of a value or kernel beyond the caller's half-open `IdentityNamespace` returns
`IdentityNamespaceExhausted`. Declared boundary IDs inside the reserved value
range return `IdentityNamespaceOverlap`. The returned
`MaterializedComposition` preserves the graph, resolved recipe, stage-to-kernel
mapping, exact `WorkspaceAllocation`, and the unchanged namespace so the caller
can assemble independent fragments without renumbering them.

## Error and failure contract

Concrete materialization failures are `OperationError` values carrying the
descriptor's `OperationId`; registry lookup errors occur before a descriptor
exists and therefore have no operation identity. The concrete family uses the
following categories:

| Condition | Error kind and behavior |
| --- | --- |
| Unknown or non-unique symbol at the registry boundary | `UnknownOperation` or `AmbiguousSymbol`; no materializer is selected. |
| Descriptor is not a structured composition, or has no concrete family owner | `WrongLoweringKind` or `MissingConcreteFormula`; no graph is emitted. |
| Empty, duplicated, or wrongly flagged declarations; exact ABI mismatch; missing or false verification fact; invalid range or parameter relation | `InvalidMaterializationRequest`. |
| Missing or wrongly typed `U64`, `F32Bits`, or `Bool` parameter | `MissingPreparedParameter` or `PreparedParameterTypeMismatch`. |
| Zero computed extent reaching `require_i32_indexable`, non-int32 extent, shape mismatch, multiple rows for uniform edges, or unsupported partial prefix | `UnsupportedConcreteShape`; a zero prepared dimension is rejected earlier by `positive_parameter` as `InvalidMaterializationRequest`. |
| Checked products, edge counts, or identity range ends overflow | `WorkspaceArithmeticOverflow` or `IdentityNamespaceExhausted`, depending on the exhausted domain. |
| Intermediate total exceeds the request limit | `WorkspaceLimitExceeded`. |
| Scalar-builder, language graph, primitive shape, stage-count, wrong-family, or final graph validation failure | `GraphMaterializationFailed`. |
| Runtime index table is outside its source extent | Device fault from `Gather` or `Scatter` with `IndexBounds::Reject`. |
| Runtime label or segment destination violates a scalar predicate | Device fault from `ScalarOpcode::Require`; there is no clamp or fallback. |

The error detail preserves the operation-specific name, expected dtype or
shape, failed fact, or checked arithmetic label. This is a preparation failure
before native lowering, not a status that proves an execution result.

## End-to-end callers and downstream role

The public `recipe::operations::materialize` facade delegates directly to
`recipe_ops::materialize_composition`. Advanced callers can construct the
request from `recipe_ops` re-exports, but production compilers use the same
boundary.

`TrainingGraphCompiler::materialize` in
[`training/src/compile.rs`](../../../../training/src/compile.rs) clones each
named tensor contract, marks inputs and outputs for the materialization
request, reserves `MATERIALIZATION_RESERVATION` value and kernel ranges, and
resolves the symbol through `operation_registry().resolve_unique`. It inserts
the returned tensor contracts and nodes into the training graph and records
the caller's iteration domain on every emitted kernel. The current in-tree
consumer of this family is `gpu_segment_sum`, used by
`deterministic_segment_sum` after flattening f32 updates and int32 IDs; it uses
the `values` declaration as the iteration-shape input.

`InferenceGraphCompiler::materialize` in
[`training/src/inference.rs`](../../../../training/src/inference.rs) follows
the same request and reservation path, then tags nodes with the first
iteration domain. The current in-tree consumer of this family is
`gpu_concat_into`, used when Bayesian inference joins a left and right
probability matrix with prepared index and selection tables; it uses `left` as
the iteration-shape input. The remaining
family operations are available through the public materialization boundary
for callers that provide their exact source-qualified ABI and preparation
facts; no in-tree caller substitutes a legacy kernel call or an internal
helper.

After insertion, the training or inference compiler validates the larger
`CalculationGraph`; primitive lowering and the planner then turn each node
into backend-neutral stages and native CUDA or HSA work. Discovery,
compilation, allocation, native-image loading, and execution remain outside
this module. The module's end-to-end role ends at one immutable, validated
graph fragment with explicit workspace and identity ownership.
