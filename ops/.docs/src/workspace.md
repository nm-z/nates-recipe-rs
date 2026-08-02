# Workspace calculations and materialization

This document covers the two workspace mechanisms in the Rust workspace. They
share the word `workspace`, checked `u64` accounting, and the same canonical
four-byte `f32`/`i32` payload domain, but they are different contracts:

1. `ops/src/workspace.rs` evaluates a static workspace query from the
   source-qualified operation registry. It returns a scalar `WorkspaceValue`.
   This path is metadata evaluation. It does not create a tensor, reserve a
   device buffer, emit a kernel, or run a GPU operation.
2. `ops/src/materialize.rs` accounts for every intermediate tensor emitted while
   turning a structured `CompositionRecipe` into a concrete
   `CalculationGraph`. It returns a `WorkspaceAllocation` attached to the
   materialized graph. This path is preparation state that must be available
   before the finalized execution loop.

The static formulas are paired with the legacy operation-surface rows whose
names end in `_workspace_bytes` or `_partials_elems`. The materializer does not
call `evaluate_workspace`; it counts the concrete tensor images that its family
materializers actually emit. Keeping these paths separate prevents a query for
one legacy operation's scratch policy from being mistaken for an allocation of a
different structured graph.

## Ownership and call graph

The static query path is:

```text
operation-surface.txt
    -> ops/build.rs parses rows and writes operation_surface.rs in OUT_DIR
    -> OperationRegistry::iter/resolve_unique/resolve_exact
    -> registry::describe
    -> registry::lowering
    -> WorkspaceFormula::for_symbol
    -> LoweringAvailability::Workspace(formula)
    -> operations::evaluate_workspace (src/facade.rs)
    -> recipe_ops::evaluate_workspace
    -> WorkspaceFormula::evaluate
    -> reduction_scratch / scan_scratch / sort_scratch / checked helpers
    -> WorkspaceValue
```

`ops/build.rs` reads `operation-surface.txt`, rejects malformed rows, preserves
their ordinal order, counts duplicate symbols, and generates the raw
`RawSurfaceEntry` array included by `ops/src/registry.rs`. The two
Recipe-owned max-pool entries are appended after that immutable prefix. The
workspace query rows are all in the raw prefix; they are not separate
Recipe-owned extensions.

The concrete materialization path is:

```text
training compile or inference compiler
    -> NamedTensor declarations and PreparedParameters
    -> reserve an IdentityNamespace
    -> MaterializationRequest::new(..., workspace_limit)
    -> materialize_composition
       -> validate_request
       -> has_concrete_materializer
       -> choose the iteration-shape input
       -> expand_composition
       -> Emitter::new / GraphBuilder::new
       -> one family dispatch module emits concrete PrimitiveKind kernels
          through Emitter::intermediate and Emitter::emit
       -> GraphBuilder::finish
          -> CalculationGraph::validate
          -> WorkspaceAllocation
       -> MaterializedComposition { graph, resolved, stages, workspace }
    -> compiler inserts graph tensors and nodes into the model graph
```

The only production callers of `materialize_composition` are the training
compiler at `training/src/compile.rs:10937-10998` and the inference compiler at
`training/src/inference.rs:2008-2077`. The public facade in `src/facade.rs`
exposes this path as `operations::materialize`, while its
`operations::evaluate_workspace` wrapper exposes only the static query path.

## Static formula types

`WorkspaceUnit` has exactly two values (`ops/src/workspace.rs:11-15`):

- `Bytes` means `WorkspaceValue.amount` is an exact byte count.
- `F32Elements` means the amount is a count of `f32` partial elements, not a
  byte count.

`WorkspaceValue` stores the amount and unit (`ops/src/workspace.rs:52-56`). Its
`bytes()` method returns `Some(ByteCount)` only for `Bytes`; split-K partials
intentionally return `None` because the paired query is an element-count
contract (`ops/src/workspace.rs:58-66`). `ByteCount` is the core newtype around
one `u64` byte value, with checked arithmetic and a `get()` accessor
(`core/src/units.rs:3-23`).

`WorkspaceFormula` is the closed formula vocabulary
(`ops/src/workspace.rs:17-50`):

| Variant | Arity | Unit | Meaning |
| --- | ---: | --- | --- |
| `NoPersistentScratch` | `dimensions` | bytes | No persistent scratch; the operation may use fixed shared-memory tiles. |
| `FixedTreeReduction` | `dimensions` | bytes | Non-final levels of the fixed 64-lane reduction tree. `sequences_dimension` repeats the reduction for each independent sequence. |
| `FixedTreeScan` | `dimensions` | bytes | Block totals and hierarchy outputs for a fixed 64-lane scan. |
| `StableSort` | `dimensions` | bytes | Power-of-two-padded `f32` values and `i32` original indexes. This enum branch exists, but no current registry symbol selects it directly. |
| `SortRunEncoding` | `dimensions` | bytes | Stable-sort scratch, one `i32` transition flag per element, and a fixed-tree prefix scan. |
| `MapThenReduction` | `dimensions` | bytes | One mapped `f32` image followed by a fixed-tree reduction. |
| `RandomKeySort` | `dimensions` | bytes | One `f32` random-key image followed by stable-sort scratch. |
| `CholeskyFactor` | 1 | bytes | A clipped 32-column `f32` panel and one `i32` fault word. |
| `CholeskySolve` | 2 | bytes | One dense `f32` right-hand-side solve image and one fault word. |
| `CholeskyInverse` | 1 | bytes | One dense `f32` identity/solve image and one fault word. |
| `LuFactor` | 1 | bytes | A clipped 32-column `f32` panel, `i32` pivots, and one fault word. |
| `LuSolve` | 2 | bytes | One dense `f32` permuted right-hand-side image and one fault word. |
| `QrFactor` | 2 | bytes | One `f32` Householder panel image and one `f32` tau value per reflector. |
| `SymmetricEigensolver` | 1 | bytes | One dense `f32` Jacobi work matrix, one `f32` diagonal vector, and one fault word. |
| `SingularValueDecomposition` | 2 | bytes | One `f32` image, two reflector panels, singular values, and one fault word. |
| `SplitKPartials` | 3 | `f32` elements | Deterministic partial output for at most eight 256-row split-K slices. |

`definition()` returns the human-readable statement in this table and is also
used in dimension-mismatch diagnostics (`ops/src/workspace.rs:107-145`).
`unit()` selects `F32Elements` only for `SplitKPartials`; every other formula
returns bytes (`ops/src/workspace.rs:147-153`).

The descriptor identity that carries a formula is `OperationId`. It stores the
registry ordinal, source-surface line, one-based occurrence, and total symbol
occurrence count, and exposes whether the symbol is duplicated or belongs to
the Recipe-owned extension segment (`ops/src/registry.rs:8-36`). The surrounding
`OperationDescriptor` keeps that identity together with the exact symbol and
source, family, canonical dtype contract, lowering, human definition, alias
contract, determinism contract, and any explicitly excluded legacy dtype
(`ops/src/registry.rs:181-196`). These fields are why a workspace query error
can remain tied to the exact public row instead of only a string symbol.

## Paired operation-surface entries

`WorkspaceFormula::for_symbol` is the only constructor for registry formulas
(`ops/src/workspace.rs:68-105`). It matches the query symbol, not the source
string. Registry resolution still retains the exact source-qualified identity,
so duplicate symbols remain distinguishable and `resolve_unique` can reject an
ambiguous request. The current operation-surface pairings are:

| Query descriptor | Paired operation descriptor | Operation-surface locations | Formula |
| --- | --- | --- | --- |
| `gpu_sum_all_workspace_bytes` | `gpu_sum_all` | `gpu-core/src/reductions.rs:173` and `:242` | `FixedTreeReduction { dimensions: 1, sequences_dimension: None }` |
| `gpu_max_all_workspace_bytes` | `gpu_max_all` | `gpu-core/src/reductions.rs:179` and `:253` | Same one-dimensional fixed-tree reduction |
| `gpu_min_all_workspace_bytes` | `gpu_min_all` | `gpu-core/src/reductions.rs:185` and `:264` | Same one-dimensional fixed-tree reduction |
| `gpu_mean_all_workspace_bytes` | `gpu_mean_all` | `gpu-core/src/reductions.rs:191` and `:275` | Same one-dimensional fixed-tree reduction |
| `gpu_l2_norm_workspace_bytes` | `gpu_l2_norm` | `gpu-core/src/reductions.rs:197` and `:286` | `MapThenReduction { dimensions: 1 }` |
| `gpu_dot_workspace_bytes` | `gpu_dot` | `gpu-core/src/reductions.rs:203` and `:310` | `NoPersistentScratch { dimensions: 1 }` |
| `gpu_reduce_sum_cols_workspace_bytes` | `gpu_reduce_sum_cols_into` | `gpu-core/src/kernels.rs:3070` and `:3040` | `FixedTreeReduction { dimensions: 2, sequences_dimension: Some(1) }` |
| `gpu_cumprod_workspace_bytes` | `gpu_cumprod` | `gpu-core/src/reductions.rs:209` and `:583` | `FixedTreeScan { dimensions: 1 }` |
| `gpu_cummax_workspace_bytes` | `gpu_cummax` | `gpu-core/src/reductions.rs:215` and `:605` | Same one-dimensional fixed-tree scan |
| `gpu_random_permutation_workspace_bytes` | `gpu_random_permutation` | `gpu-core/src/catboost.rs:53` and `:60` | `RandomKeySort { dimensions: 1 }` |
| `gpu_count_distinct_workspace_bytes` | `gpu_count_distinct` | `gpu-core/src/encoding.rs:191` and `:203` | `SortRunEncoding { dimensions: 1 }` |
| `gpu_run_length_workspace_bytes` | `gpu_run_length` | `gpu-core/src/encoding.rs:230` and `:242` | Same one-dimensional sort/run encoding |
| `gpu_cholesky_workspace_bytes` | `gpu_cholesky` | `gpu-core/src/kernels.rs:2209` and `:2226` | `CholeskyFactor` |
| `gpu_cholesky_solve_workspace_bytes` | `gpu_cholesky_solve` | `gpu-core/src/kernels.rs:1903` and `:1924` | `CholeskySolve` |
| `gpu_potrs_workspace_bytes` | `gpu_potrs` | `gpu-core/src/linalg.rs:512` and `:532` | Same `CholeskySolve` formula |
| `gpu_cholesky_inv_workspace_bytes` | `gpu_cholesky_inv` | `gpu-core/src/kernels.rs:2005` and `:2010` | `CholeskyInverse` |
| `gpu_lu_factor_workspace_bytes` | `gpu_lu_factor` | `gpu-core/src/linalg.rs:381` and `:398` | `LuFactor` |
| `gpu_solve_getrf_workspace_bytes` | `gpu_solve` getrf phase | `gpu-core/src/kernels.rs:2089` and `:2128` | Same `LuFactor` formula |
| `gpu_lu_solve_workspace_bytes` | `gpu_lu_solve` | `gpu-core/src/linalg.rs:439` and `:460` | `LuSolve` |
| `gpu_solve_getrs_workspace_bytes` | `gpu_solve` getrs phase | `gpu-core/src/kernels.rs:2107` and `:2128` | Same `LuSolve` formula |
| `gpu_qr_workspace_bytes` | `gpu_qr` | `gpu-core/src/linalg.rs:579` and `:611` | `QrFactor` |
| `gpu_eigh_sym_workspace_bytes` | `gpu_eigh_sym` | `gpu-core/src/linalg.rs:689` and `:708` | `SymmetricEigensolver` |
| `gpu_svd_workspace_bytes` | `gpu_svd` | `gpu-core/src/linalg.rs:752` and `:769` | `SingularValueDecomposition` |
| `gpu_splitk_dw_partials_elems` | `gpu_splitk_dw_into` | `gpu-core/src/kernels.rs:3363` and `:3369` | `SplitKPartials` |

The `gpu_solve_getrf_workspace_bytes` and
`gpu_solve_getrs_workspace_bytes` rows describe phases of the single
`gpu_solve` operation. There are no standalone `gpu_solve_getrf` or
`gpu_solve_getrs` operation rows. `gpu_reduce_sum_cols_into` and
`gpu_splitk_dw_into` retain their `_into` operation names while their paired
queries omit `_into` or use the legacy query suffix.

## Exact static formulas

All dimensions are supplied as raw `u64` values. The evaluator checks only the
required arity. It does not prove that a matrix is square, that a right-hand
side has a compatible column count, or that a dimension is nonzero. Those
semantic constraints belong to the paired operation's own lowering or
materialization contract. Zero dimensions are therefore accepted wherever the
arithmetic can represent them.

The implementation constants are fixed in `ops/src/workspace.rs:5-9`:

```text
T = 64       fixed tree lanes
P = 32       solver panel width
S = 256      rows per split-K part
K = 8        maximum split-K parts
B = 4        bytes per f32 or i32 word
```

### Reduction and scan helpers

The fixed-tree reduction helper is `reduction_scratch(width, sequences)`
(`ops/src/workspace.rs:287-300`). Define:

```text
R(width, sequences):
    bytes = 0
    repeat:
        next = ceil(width / T)
        if next <= 1: return bytes
        bytes += sequences * next * B
        width = next
```

It reserves the output of every non-final level. A width of zero or one, and a
width up to 64, needs zero persistent reduction bytes. Width 65 needs two
`f32` values per sequence for the first non-final level. For
`gpu_reduce_sum_cols_workspace_bytes`, dimensions are `[rows, columns]` and
`sequences = columns`, so the reduction is independently scratch-accounted
for each column.

The fixed-tree scan helper is `scan_scratch(width)`
(`ops/src/workspace.rs:302-317`):

```text
S_scan(width):
    bytes = 0
    level = 0
    repeat:
        blocks = ceil(width / T)
        if level > 0: bytes += width * B
        if blocks <= 1: return bytes
        bytes += blocks * B
        width = blocks
        level += 1
```

Unlike a reduction, a scan keeps block totals for the initial level and then
keeps each hierarchy image needed by later levels. Consequently a nonzero scan
of at most 64 elements still reports one four-byte block-total word. Zero
elements report zero. The checked level increment prevents a theoretical
`u32` level counter wrap.

`sort_scratch(elements)` pads a nonzero element count to the next power of two
and reserves two words per padded position, one `f32` value and one `i32`
original index (`ops/src/workspace.rs:319-328`):

```text
Q(elements) = 0                         when elements = 0
              next_power_of_two(elements) * 2 * B otherwise
```

`checked_next_power_of_two` converts an unrepresentable padding request to
`WorkspaceArithmeticOverflow`; it never wraps or silently rounds down.

### Reduction, sort, and random-query formulas

For `FixedTreeReduction`, the evaluator calls `R(dimensions[0], sequences)`;
`sequences` is one unless `sequences_dimension` is present
(`ops/src/workspace.rs:167-175`). Therefore:

- `gpu_sum_all`, `gpu_max_all`, `gpu_min_all`, and `gpu_mean_all` return
  `R(N, 1)`.
- `gpu_reduce_sum_cols_into` returns `R(rows, columns)`.

`NoPersistentScratch` returns exactly zero (`ops/src/workspace.rs:167-169`), so
`gpu_dot_workspace_bytes` reports zero for every one-element dimension list.

`MapThenReduction` returns `N*B + R(N, 1)`
(`ops/src/workspace.rs:185-189`). The first term is the mapped f32 image and
the second term is every non-final reduction level. This is the
`gpu_l2_norm_workspace_bytes` policy.

`FixedTreeScan` returns `S_scan(N)` (`ops/src/workspace.rs:176`). This is used
by cumulative product and cumulative maximum.

`StableSort` returns `Q(N)` (`ops/src/workspace.rs:177`). It is a reusable enum
formula but no current `for_symbol` arm selects it alone.

`SortRunEncoding` returns:

```text
Q(N) + N*B + S_scan(N)
```

(`ops/src/workspace.rs:178-184`). The middle term is one transition flag per
element. Count-distinct and run-length encoding both use this policy.

`RandomKeySort` returns:

```text
N*B + Q(N)
```

(`ops/src/workspace.rs:191-195`). The first term is one f32 Philox key image;
the second is the stable sorting network's value/index storage. This is the
random-permutation policy.

### Solver formulas

`dense_image_with_fault(rows, columns)` is the shared helper
(`ops/src/workspace.rs:330-335`):

```text
D(rows, columns) = rows * columns * B + B
```

The final `B` is one int32 fault/status word. The solver variants evaluate as
follows (`ops/src/workspace.rs:197-237`):

- `CholeskyFactor(n)`:
  `n * min(n, P) * B + B`. The first term is the clipped 32-column panel.
- `CholeskySolve(n, rhs_columns)`:
  `D(n, rhs_columns)`, used by both `gpu_cholesky_solve_workspace_bytes` and
  `gpu_potrs_workspace_bytes`.
- `CholeskyInverse(n)`:
  `D(n, n)`, the dense identity/solve image plus the fault word.
- `LuFactor(n)`:
  `n * min(n, P) * B + n * B + B`. The terms are the LU panel, one int32
  pivot per row, and the fault word. Both the direct LU factor query and the
  getrf phase of `gpu_solve` use this formula.
- `LuSolve(n, rhs_columns)`:
  `D(n, rhs_columns)`, used by both the direct LU solve query and the getrs
  phase of `gpu_solve`.
- `QrFactor(m, n)`:
  `(m * min(m, n) + min(m, n)) * B`. The first term is the Householder image
  and the second is one f32 tau value per reflector. QR has no extra fault word
  in this contract.
- `SymmetricEigensolver(n)`:
  `(n*n + n) * B + B`. The terms are the dense Jacobi work matrix, diagonal
  vector, and fault word.
- `SingularValueDecomposition(m, n)`:
  `((m*n) + ((m+n)*k) + k) * B + B`, where `k = min(m,n)`. The terms are the
  bidiagonalization image, two reflector panels represented by the combined
  `(m+n)*k` storage, singular values, and the fault word.

The arithmetic is grouped as checked products and sums, so every term in the
displayed equations is subject to the same overflow behavior as the source.

### Split-K partial elements

`SplitKPartials` consumes `[m, k, n]` and returns an element count, not bytes
(`ops/src/workspace.rs:238-246`):

```text
parts = 0                         when m = 0
        clamp(ceil(m / S), 1, K)  otherwise
amount = parts * k * n
unit = F32Elements
```

Thus a nonempty `m` always reserves at least one part, a matrix with more than
`K*S` rows remains capped at eight parts, and `m = 0` reports zero partial
elements. The caller that owns `gpu_splitk_dw_into` must multiply by the
appropriate element width only when it needs a byte reservation; the public
workspace result intentionally preserves the operation's `_elems` unit.

## Static evaluator and registry behavior

`evaluate_workspace(descriptor, dimensions)` accepts only a descriptor whose
`lowering` is `LoweringAvailability::Workspace(formula)`
(`ops/src/workspace.rs:270-285`). It delegates to `formula.evaluate` and adds
the descriptor's `OperationId` to every error. Any other lowering kind returns
`WrongLoweringKind` with the detail `operation does not own a static workspace
formula`.

The registry chooses that lowering in a strict order
(`ops/src/registry.rs:347-373`): scalar recipes first, primitive recipes next,
workspace formulas next, then non-calculation recipes and compositions. Legacy
dtype exclusions and other unsupported categories are considered only after
those owned implementations. For a workspace query, `describe` records
`LoweringAvailability::Workspace`, derives `OperationFamily::Workspace`, uses
`CanonicalDTypeContract::NonNumericHostData`, stores the formula definition,
and assigns fixed-primitive determinism (`ops/src/registry.rs:326-344`,
`375-413`, `459-480`, and `643-661`). The static query is a host-side sizing
operation, not a numeric payload calculation.

`OperationRegistry::resolve_unique` reports `UnknownOperation` when the symbol
is absent and `AmbiguousSymbol` when more than one source-qualified row has the
symbol (`ops/src/registry.rs:283-304`). `resolve_exact` selects one exact
`(symbol, source)` pair and reports the same error kinds for absence or
duplicates (`ops/src/registry.rs:306-323`). The public facade exposes these as
`operations::resolve`, `operations::resolve_exact`, `operations::all`, and
`operations::evaluate_workspace` (`src/facade.rs:63-123`).

`UnsupportedReason::WorkspaceQuery` remains a representable registry reason,
but current `lowering()` does not construct it. Current query symbols map to a
real `WorkspaceFormula`; an unrecognized `gpu_` symbol falls through to
`DedicatedPrimitiveCompositionPending`, and a non-`gpu_` symbol falls through
to host behavior. This means a descriptor that reaches the evaluator as
`Workspace` always has one of the formulas above.

## Error and overflow contract

The operation error type stores an `OperationErrorKind`, human-readable detail,
and optional `OperationId` (`ops/src/error.rs:30-52`). `Display` prints the kind
and detail and appends the operation ordinal when context is present
(`ops/src/error.rs:54-61`).

Static query failures are closed and typed:

| Failure | Source and condition |
| --- | --- |
| `UnknownOperation` | Registry lookup found no symbol or exact source pair. |
| `AmbiguousSymbol` | A symbol or exact pair has multiple source-qualified rows. |
| `WrongLoweringKind` | The descriptor is scalar, primitive, composition, non-calculation, or unsupported rather than `Workspace`. |
| `WorkspaceFormulaMismatch` | The dimension slice length differs from `dimension_count()`. |
| `WorkspaceArithmeticOverflow` | Any checked sum/product, next-power-of-two padding, or checked scan-level increment exceeds `u64` or the helper's counter range. |

`checked_add` and `checked_mul` are the only arithmetic primitives used by the
static formulas (`ops/src/workspace.rs:337-345`). There is no saturating result,
fallback dimension, retry, or alternate formula. The wrapper applies the
operation ID after `evaluate`, so both arity errors and arithmetic errors retain
the query's source-qualified identity.

## Concrete composition workspace allocation

The materialization workspace is a graph property, not a
`WorkspaceFormula`. Its request and result types are in
`ops/src/materialize.rs:109-143` and `:233-315`:

- `MaterializationRequest` carries one structured `OperationDescriptor`, named
  immutable input and output tensors, the name of the input whose shape drives
  iteration expansion, typed `PreparedParameters`, a caller-owned
  `IdentityNamespace`, and a `ByteCount workspace_limit`.
- `WorkspaceObject` records an intermediate `ValueId`, `DType`, `Shape`, and
  exact `ByteCount` storage.
- `WorkspaceAllocation` owns the ordered `Vec<WorkspaceObject>` and the exact
  total bytes. `objects()` and `bytes()` expose immutable inspection.
- `MaterializedComposition` returns the operation ID, validated graph, resolved
  composition, concrete stage-to-kernel mappings, the workspace allocation,
  and the identity namespace.

`PreparedParameters` is a `BTreeMap<String, PreparedParameter>`. Each value is
one of `U64`, `I32`, `F32Bits`, or `Bool` (`ops/src/materialize.rs:55-64`).
Dimension bounds use `U64`; scalar constants retain exact f32 bit patterns in
`F32Bits`; verified preparation facts use `Bool`. The materializer never parses
untyped strings or invents a default for a missing parameter.

The allocation equation is the sum over every intermediate emitted by the
materializer:

```text
workspace_bytes = Σ intermediate.shape.elements
                    * intermediate.dtype.byte_width()
```

`Shape::bytes` and `Tensor::contiguous` implement this exact product with
checked arithmetic (`language/src/shape.rs:50-60` and
`language/src/tensor.rs:159-177`). Canonical `DType::F32` and `DType::I32`
both have a four-byte width (`core/src/scalar.rs:9-23`). Input and output
declarations are retained in the graph but are not added to this total; only
calls to `GraphBuilder::intermediate` append `WorkspaceObject` entries.

### Request validation and expansion

`materialize_composition` first calls `validate_request`
(`ops/src/materialize.rs:413-445` and `:4268-4331`). The request must have a
structured composition lowering, at least one input and output, unique
nonempty tensor names, unique tensor IDs, valid tensors, external-input flags
on all inputs, and non-input external-output flags on all outputs. Missing or
wrong declarations fail as `InvalidMaterializationRequest`; tensor validation
errors are mapped to `GraphMaterializationFailed` with the operation ID.

The descriptor must have a concrete family materializer. The support check
covers optimizer/normalization, solver/FFT, attention/sequence/embedding,
convolution/pooling, loss/metrics, indexing/sort/encoding, graph/cluster/RL,
tree/boosting, inference/quantization/diffusion, creation/shape/miscellaneous,
and training modules (`ops/src/materialize.rs:4629-4641`). A structured recipe
without an exact source-qualified implementation fails with
`MissingConcreteFormula`; `remaining_composition_manifest` reports such rows
with all four missing components: tensor ABI, scalar formula, primitive
parameters, and workspace policy (`ops/src/materialize.rs:482-505`).

Concrete dispatch is ordered and fail-closed. `dispatch_concrete` asks the
optimizer/normalization, solver/FFT, attention/sequence/embedding,
convolution/pooling, loss/metrics, indexing/sort/encoding, graph/cluster/RL,
tree/boosting, inference/quantization/diffusion, creation/shape/miscellaneous,
and training modules in that order (`ops/src/materialize.rs:452-480`). Each
module can return `NotOwned` or one `Owned` result. The first exact source-owned
implementation returns; if all modules decline, materialization reports
`GraphMaterializationFailed` instead of selecting a symbolically similar
operation.

The named `iteration_shape_input` must exist in the input declarations. Its
shape drives `expand_composition`, which validates the recipe and resolves
fixed, shape-extent, minimum-extent, ceiling-log2, and prepared-parameter
bounds (`ops/src/materialize.rs:385-411` and `:508-607`). Each repeat becomes a
`ResolvedBound`; each primitive becomes a `ResolvedStep` with its enclosing
iteration indices and a dependency on the immediately preceding step. Expansion
is finite and deterministic. A recipe that would exceed the fixed one-million
step bound returns `CompositionExpansionOverflow`; missing axes or prepared
`U64` values return `IterationBoundUnresolved`, `MissingPreparedParameter`, or
`PreparedParameterTypeMismatch`.

### Identity state and graph emission

`IdentityNamespace` supplies half-open ranges for intermediate value IDs and
kernel-template IDs. `GraphBuilder::new` checks that each range end fits `u64`
and rejects any declared tensor ID inside the reserved intermediate range
(`ops/src/materialize.rs:727-760`, with range arithmetic at `:4603-4627`).
Independent namespaces are checked pairwise by
`validate_identity_namespaces`; overlapping value or kernel ranges return
`IdentityNamespaceOverlap` (`ops/src/materialize.rs:317-346`).

`Emitter` is the one common route used by every family materializer. It carries
the resolved step cursor, concrete stage list, and `GraphBuilder`
(`ops/src/materialize.rs:609-710`). `emit_stage` requires a nonempty kernel
sequence, verifies that its final kernel's `PrimitiveFamily` matches the
resolved recipe step, allocates kernel IDs through the graph builder, and records
the `StageEmission`. Too many or too few emitted stages produce
`GraphMaterializationFailed`.

`GraphBuilder::intermediate` is the allocation point
(`ops/src/materialize.rs:763-805`):

1. Reject when the reserved value range is exhausted.
2. Allocate the next `ValueId` and a contiguous row-major tensor.
3. Convert tensor shape and dtype to exact `storage_bytes`.
4. Checked-add the bytes to the running total.
5. Reject with `WorkspaceLimitExceeded` when the new total exceeds the request
   limit.
6. Append the `WorkspaceObject` and tensor declaration.

There is no implicit reuse, liveness subtraction, or alternate storage path in
this accounting. The total is the sum of every intermediate image emitted by
the concrete graph, preserving enough information for a later planner to make
the authoritative placement decision.

`GraphBuilder::emit` rejects exhausted kernel ranges, creates a
`PrimitiveKernel`, and applies a forbidden alias rule to every input/output
pair (`ops/src/materialize.rs:807-832`). `GraphBuilder::finish` constructs the
`CalculationGraph`, validates tensor contracts, producer uniqueness, primitive
contracts, and acyclic topological order through `CalculationGraph::validate`,
then returns the graph and `WorkspaceAllocation`
(`ops/src/materialize.rs:835-846`; graph validation is
`language/src/graph.rs:78-138`). A language validation failure is reported as
`GraphMaterializationFailed`.

### Materialization errors

The materialization path uses the following workspace-relevant failures from
`OperationErrorKind` (`ops/src/error.rs:5-28`):

| Failure | Meaning |
| --- | --- |
| `WrongLoweringKind` | A static workspace, scalar, primitive, lifecycle, or unsupported descriptor was sent to composition materialization. |
| `InvalidMaterializationRequest` | Bad boundary flags, duplicate names or IDs, missing iteration input, wrong tensor ABI, or a false required preparation fact. |
| `MissingConcreteFormula` | The registry has a descriptive composition but no exact concrete family materializer. |
| `MissingPreparedParameter` / `PreparedParameterTypeMismatch` | A bound or concrete emitter lacks the typed preparation value it requires. |
| `IterationBoundUnresolved` / `CompositionExpansionOverflow` | Shape or bound resolution failed, or expansion exceeded one million primitive steps. |
| `IdentityNamespaceOverlap` / `IdentityNamespaceExhausted` | Reserved ranges overlap, overflow, or run out while intermediates or kernels are emitted. |
| `WorkspaceArithmeticOverflow` | Tensor byte calculation or running workspace total overflowed `u64`. |
| `WorkspaceLimitExceeded` | The exact running total exceeded `MaterializationRequest.workspace_limit`. |
| `UnsupportedConcreteShape` | A family emitter rejected a shape outside its documented concrete domain. |
| `GraphMaterializationFailed` | Family dispatch, kernel-family matching, scalar construction, tensor contracts, or final graph validation failed. |

These failures are not masked by a fallback formula. The operation ID is
attached at the common error constructors, so a caller can identify the exact
source-qualified descriptor that could not cross the preparation boundary.

## Training and inference integration

The training compiler's `materialize` method clones the current tensor contracts,
marks cloned inputs and outputs with the materializer's temporary boundary
flags, creates `NamedTensor` values, and reserves 64 intermediate values and 64
kernel IDs before calling the operations registry and materializer
(`training/src/compile.rs:10937-10995`). It passes
`WORKSPACE_LIMIT = ByteCount::new(u64::MAX)` (`training/src/compile.rs:54-55`),
so current training preparation imposes no practical byte ceiling beyond checked
arithmetic and the reserved identity range. After success, it inserts the
materialized graph's tensor contracts and nodes and associates the caller's
iteration domain (`training/src/compile.rs:10996-11015`). The local
`MaterializedComposition` is then dropped, so the current compiler consumes the
validated graph and its checked preparation result rather than retaining the
allocation object as a long-lived runtime handle.

Inference follows the same boundary at `training/src/inference.rs:2008-2065`:
it reserves the same 64-value and 64-kernel namespace, resolves the unique
symbol, passes `u64::MAX` as the workspace limit
(`training/src/inference.rs:54-55`), and inserts the graph tensors and nodes with
the first iteration domain (`training/src/inference.rs:2066-2077`). The concrete
materializers therefore run once during preparation; they are not host loops or
workspace allocation operations repeated in the finalized execution loop.

Other training and inference operation families have their own requirement
structures with `workspace_bytes` and pass those byte limits into specialized
requests. Those requirements are independent of `WorkspaceFormula` queries.
The repository search shows no call from those requirement builders to
`evaluate_workspace`; the static evaluator is currently reached through the
public `operations::evaluate_workspace` facade only.

## End-to-end role and invariants

For a static query, the real user boundary is:

```text
resolve("..._workspace_bytes" or "..._partials_elems")
    -> source-qualified OperationDescriptor
    -> evaluate_workspace(descriptor, dimensions)
    -> checked WorkspaceValue
```

The result is a deterministic sizing fact. It can be inspected through the
public `WorkspaceValue` API, but no side effect follows from evaluating it.
Dimension arity, formula family, unit, and arithmetic behavior are all derived
from the descriptor and formula; callers cannot select a second implementation
or silently obtain a byte conversion for an element-count query.

For a structured operation, the real preparation boundary is:

```text
validated user tensors and typed preparation facts
    -> finite resolved recipe
    -> concrete primitive kernels and intermediate tensors
    -> exact WorkspaceAllocation plus validated CalculationGraph
    -> planner/compiler state
    -> native preparation and finalized init/loop/exit execution
```

The important invariants are:

- only `LoweringAvailability::Workspace` descriptors enter the static formula
  evaluator;
- static dimension arity is exact, and every arithmetic operation is checked;
- the fixed tree, solver panel, split-K row size, split-K cap, and four-byte word
  width are part of the formula contract;
- `F32Elements` is never guessed to be bytes;
- composition workspace is the exact sum of concrete intermediate tensor storage,
  not a symbolic estimate and not a separate host payload loop;
- every intermediate and kernel identity comes from the caller's reserved
  namespace, with overlap and exhaustion rejected;
- every emitted stage matches a resolved primitive family and every final graph
  passes tensor, producer, alias, and cycle validation;
- workspace limits and arithmetic failures remain visible as typed errors;
- materialization is preparation-time state. The finalized execution loop
  receives the resulting graph and precomputed resource contracts rather than
  reconstructing workspace or graph semantics on the host.
