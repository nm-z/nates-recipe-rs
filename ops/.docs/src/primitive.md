# `recipe-ops` direct primitive lowering

<!--
Intent: describe the direct primitive registry recipes in
ops/src/primitive.rs, the recipe-primitives lowering they call, and the
validated planner and target-realization path that consumes the result.
This file documents the implementation contract. It is not an alternate
operation registry or a promise that every PrimitiveKind has a legacy symbol.
-->

## Boundary and intent

[`ops/src/primitive.rs`](../../src/primitive.rs) is the direct, non-elementwise
lowering boundary of `recipe-ops`. It maps a finite set of source-qualified
legacy symbols to a small `PrimitiveRecipe` value, checks the invariant part of
the requested `recipe_language::PrimitiveKernel`, and delegates the concrete
shape and hardware work to [`recipe-primitives`](../../../primitives/src/lib.rs).
The returned value is a backend-neutral `LoweredProgram`, not a vendor kernel,
an artifact, or an execution task.

The split is deliberate:

* `recipe-ops` owns operation meaning at the public symbol boundary: family,
  canonical payload dtypes, axis policy, shape class, alias policy, and the
  exact direct-recipe definition.
* `recipe-language` owns the typed kernel declaration, tensors, layouts,
  primitive parameters, and language-level validation.
* `recipe-primitives` owns immutable dispatch geometry, buffer views, scratch,
  trees, synchronization, atomics, checked-fault behavior, resource bounds,
  and the canonical program digest.
* `planner` owns placement, physical copies, task dependencies, artifact-build
  contracts, and lifecycle admission. `kernel` owns target LLVM realization.

No layer substitutes a scalar implementation, a host loop, a vendor library,
or a fallback operation when a direct recipe is absent or invalid.

The complete direct path is:

```text
operation-surface.txt
        |
        | ops/build.rs generates RAW_OPERATION_SURFACE
        v
OperationRegistry::resolve_unique / resolve_exact
        |
        | registry::lowering calls PrimitiveRecipe::for_symbol
        v
OperationDescriptor { lowering: Primitive(recipe), dtypes, family, ... }
        |
        | facade::operations::lower_primitive
        v
PrimitiveRecipe::matches(PrimitiveKernel, tensor index)
        |
        | recipe_primitives::lower(kernel, tensor index, measured hardware)
        v
LoweredProgram { buffers, stages, resources, digest }
        |
        v
planner validation and stage-scoped artifact builds
        |
        v
kernel::lower_stage -> target LLVM -> native artifact
```

The planner's universal graph path also calls `recipe_primitives::lower`
directly for every validated graph node. That is not a second semantic
implementation: the node already contains the same `PrimitiveKernel` contract
and the planner is lowering the complete graph, not resolving a legacy symbol.

## Public types in `ops/src/primitive.rs`

### `PrimitiveFamily`

`PrimitiveFamily` is the backend-neutral family vocabulary shared by direct
recipes and structured materialization:

`Elementwise`, `Reduce`, `Scan`, `Contraction`, `Gather`, `Scatter`,
`Histogram`, `Sort`, `IndexMap`, and `Random`.

`PrimitiveRecipe::family` can produce only `Reduce`, `Scan`, `Contraction`,
`Gather`, `Scatter`, `Sort`, `IndexMap`, or `Random`. The `Elementwise` and
`Histogram` variants are needed by the shared materialization vocabulary and
by `materialize::primitive_family`, but this direct symbol table intentionally
has no elementwise or histogram recipe. Elementwise symbols use
`ScalarRecipe`; structured operations can emit both elementwise and histogram
`PrimitiveKind` values.

`PrimitiveRecipe::operation_family` projects a direct family into the public
`OperationFamily` registry taxonomy:

| Direct family | Registry family |
| --- | --- |
| `Reduce` | `Reduction` |
| `Scan` | `Scan` |
| `Contraction` | `Contraction` |
| `Gather`, `Scatter`, `Sort`, `IndexMap` | `ShapeAndIndexing` |
| `Random` | `Random` |

The match arms for `Elementwise` and `Histogram` are shared-enum arms and are
not reachable from a `PrimitiveRecipe` value.

### `AxisRequirement`

`Any` means a nonempty axis list for a reduction or any axis for a scan or
indexed operation. `First` means axis `0`, `Last` means the final axis of the
first input tensor, and `All` means all axes in ascending order for a reduction.
For a single-axis operation, `All` is intentionally restricted to rank one
and axis zero. A missing first input or tensor index makes rank-dependent
requirements fail instead of guessing a rank.

### `ContractionClass`

The five static entrypoint shape classes are `Vector`, `Matrix`,
`LeftTransposedMatrix`, `RightTransposedMatrix`, and `Batched`. They describe
which axis pairs are retained from the legacy symbol. Extents, layouts, and
the concrete output shape remain in the `Contraction` kernel and tensors.

### `RandomRecipe`

Only `UniformF32` and `NormalF32` are direct registry recipes. The key,
distribution parameters, and explicit Philox round count remain in the
borrowed `RandomMap` inside the caller's kernel. Other language distributions
are valid `PrimitiveKind::Random` values for general graphs, but do not match a
direct legacy recipe.

### `PrimitiveRecipe`

`PrimitiveRecipe` is a `Copy` descriptor containing only pre-kernel invariants:

```text
Reduce { operator, result, axes }
Scan { operator, exclusive, axis }
Contraction(ContractionClass)
Gather { axis }
ScatterAdd { axis }
Sort { direction, stable, axis }
IndexMap
Random(RandomRecipe)
```

The descriptor does not copy shape-dependent extents, tree widths, random keys,
index bounds, or alias declarations. Keeping those facts in the
`PrimitiveKernel` prevents a symbol recipe from silently overriding a caller's
typed graph declaration.

`family`, `operation_family`, `dtype_contract`, and `definition` are all
`const` projections. The registry uses them while constructing an
`OperationDescriptor`; `definition` is also used in the mismatch diagnostic.

`dtype_contract` is exact for direct recipes:

| Recipe | Input dtypes | Output dtypes |
| --- | --- | --- |
| value reduction or scan | `[F32]` | `[F32]` |
| index reduction | `[F32]` | `[I32]` |
| value and index reduction | `[F32]` | `[F32, I32]` |
| contraction | `[F32, F32]` | `[F32]` |
| gather | `[F32, I32]` | `[F32]` |
| scatter add | `[F32, I32, F32]` | `[F32]` |
| sort | `[F32]` | `[F32]` |
| index map | `[]` | `[I32]` |
| uniform or normal random | `[]` | `[F32]` |

This contract is stricter than the general language primitive vocabulary. For
example, the language permits int32 `Any` and `All` reductions, but no direct
legacy recipe selects those forms.

### `PrimitiveRequest<'a>`

`PrimitiveRequest` borrows the exact `PrimitiveKernel` and a
`BTreeMap<ValueId, &Tensor>`. It is `Copy` and allocation-free. The map is the
complete tensor index needed by language validation and backend-neutral
lowering. The request owns no graph state and cannot outlive its caller's
kernel or tensors.

## Registry materialization

[`ops/src/registry.rs`](../../src/registry.rs) classifies a raw
`operation-surface.txt` row in fixed first-match order. It tries scalar recipes,
then `PrimitiveRecipe::for_symbol`, then workspace, non-calculation, and
composition recipes, followed by explicit unsupported classifications. A
symbol is never inferred from a similarly named legacy source.

`PrimitiveRecipe::for_symbol` is a private linear lookup over the immutable
`SYMBOL_RECIPES` slice. `OperationRegistry::resolve_unique` or
`resolve_exact` first establishes the source-qualified descriptor. The
descriptor's `lowering` is then the sole authority for whether direct
primitive lowering is allowed. Its `family`, `dtypes`, `definition`, alias
contract, and determinism contract are derived from that same recipe.

The 29 direct symbols are preserved here with their source location and
recipe-level invariant:

| Symbol | Legacy source | Direct recipe invariant |
| --- | --- | --- |
| `gpu_argmax_f32` | `gpu-core/src/kernels.rs:4125` | maximum index reduction over all axes |
| `gpu_argmax_rows` | `gpu-core/src/kernels.rs:5122` | maximum index reduction over the last axis |
| `gpu_argmin_rows` | `gpu-core/src/kernels.rs:4646` | minimum index reduction over the last axis |
| `gpu_max_all` | `gpu-core/src/reductions.rs:253` | maximum value reduction over all axes |
| `gpu_min_all` | `gpu-core/src/reductions.rs:264` | minimum value reduction over all axes |
| `gpu_sum_all` | `gpu-core/src/reductions.rs:242` | sum value reduction over all axes |
| `gpu_reduce_max_cols` | `gpu-core/src/kernels.rs:6141` | maximum value reduction over the first axis |
| `gpu_reduce_min_cols` | `gpu-core/src/kernels.rs:6204` | minimum value reduction over the first axis |
| `gpu_reduce_sum_cols_into` | `gpu-core/src/kernels.rs:3040` | sum value reduction over the first axis |
| `gpu_reduce_max_rows` | `gpu-core/src/kernels.rs:6108` | maximum value reduction over the last axis |
| `gpu_reduce_min_rows` | `gpu-core/src/kernels.rs:6174` | minimum value reduction over the last axis |
| `gpu_reduce_sum_rows` | `gpu-core/src/kernels.rs:5143` | sum value reduction over the last axis |
| `gpu_cummax` | `gpu-core/src/reductions.rs:605` | inclusive maximum scan over any axis |
| `gpu_cumprod` | `gpu-core/src/reductions.rs:583` | inclusive product scan over any axis |
| `gpu_cumsum_cols` | `gpu-core/src/reductions.rs:562` | inclusive sum scan on the first axis |
| `gpu_cumsum_rows` | `gpu-core/src/reductions.rs:541` | inclusive sum scan on the last axis |
| `gpu_prefix_sum_exclusive` | `gpu-core/src/kernels.rs:6718` | exclusive sum scan with the canonical f32 zero identity |
| `gpu_prefix_sum_inclusive` | `gpu-core/src/kernels.rs:6691` | inclusive sum scan over any axis |
| `gpu_dot` | `gpu-core/src/reductions.rs:310` | rank-one vector contraction, contract `(0, 0)` |
| `gpu_gemm` | `gpu-core/src/kernels.rs:1803` | rank-two matrix contraction, contract `(1, 0)` |
| `gpu_gemm_at` | `gpu-core/src/kernels.rs:1836` | rank-two left-transposed contraction, contract `(0, 0)` |
| `gpu_gemm_bt` | `gpu-core/src/infer_ops.rs:567` | rank-two right-transposed contraction, contract `(1, 1)` |
| `gpu_gemm_bt_into` | `gpu-core/src/kernels.rs:1869` | same right-transposed contraction; the registry marks the `_into` form as no-alias |
| `gpu_bmm_into` | `gpu-core/src/linalg.rs:319` | rank-at-least-three batched contraction with one contract pair |
| `gpu_gather_rows_into` | `gpu-core/src/kernels.rs:5798` | first-axis gather with `IndexBounds::Reject` |
| `gpu_scatter_add` | `gpu-core/src/kernels.rs:5822` | first-axis checked atomic add, sequentially consistent, exact input/output alias |
| `gpu_sort` | `gpu-core/src/reductions.rs:434` | ascending, non-stable, all-axis sort without index output |
| `gpu_rand_uniform_into` | `gpu-core/src/kernels.rs:3325` | counter-keyed Philox uniform f32 map |
| `gpu_randn` | `gpu-core/src/kernels.rs:7006` | counter-keyed Philox normal f32 map |

`IndexMap` is intentionally not assigned a legacy symbol. Its public helper is
`lower_index_map`, which lowers Recipe's iteration-aware affine int32 source
without pretending that an old operation-surface row exists.

Structured materialization treats direct recipes as already executable and does
not dispatch them as compositions. In
[`materialize.rs`](../../src/materialize.rs), `expand_composition` and
`materialize_composition` accept only `LoweringAvailability::Composition`;
`remaining_composition_manifest` filters out `Primitive` entries. A concrete
composition can still emit `PrimitiveKind` nodes, including histogram and
elementwise nodes, and the planner later lowers those nodes through
`recipe-primitives`.

## Recipe matching

`PrimitiveRecipe::matches` is the narrow bridge between a registry name and a
typed language kernel. It returns `false` for every mismatched primitive kind,
then checks only the recipe's invariant. It does not replace
`PrimitiveKernel::validate`; the callee runs language validation again before
constructing a program.

### Reductions

The operator and `ReduceResult` must equal the recipe. `AxisRequirement` is
checked against the first input tensor's rank:

* `Any` requires a nonempty axis set.
* `First` requires exactly `[0]`.
* `Last` requires exactly `[rank - 1]`.
* `All` requires every axis in ascending `0..rank` order.

The language layer additionally checks one input, the expected one or two
outputs, a valid fixed tree lane count, axis rank, reduced output shape, and
the value/index dtypes. Empty minimum and maximum domains are rejected because
they have no implicit identity.

### Scans

The operator, inclusive versus exclusive mode, and axis requirement must
match. Reverse scans never match a direct recipe. Inclusive mode has no
identity check. Exclusive sum must carry an `F32Bits` zero and exclusive
product must carry an `F32Bits` one. An int32 identity, or an exclusive
minimum, maximum, `Any`, or `All` scan, fails the direct match even if the
general language type would otherwise be valid.

### Contractions

The two input ranks and ordered axis pairs must match the class:

| Class | Required ranks | Batch axes | Contract axes |
| --- | --- | --- | --- |
| `Vector` | `(1, 1)` | empty | `[(0, 0)]` |
| `Matrix` | `(2, 2)` | empty | `[(1, 0)]` |
| `LeftTransposedMatrix` | `(2, 2)` | empty | `[(0, 0)]` |
| `RightTransposedMatrix` | `(2, 2)` | empty | `[(1, 1)]` |
| `Batched` | both at least rank 3 | nonempty | exactly one pair |

`PrimitiveKernel::validate` then checks axis bounds, no axis reuse, equal
contract and batch extents, matching operand/output dtypes, and the exact
derived output shape.

### Gather and scatter

Gather requires the recipe axis relationship and `IndexBounds::Reject`. The
language kernel must have a value tensor and an int32 index tensor, with the
derived gather shape and value dtype.

Scatter add requires the axis relationship, `Reject` bounds, an atomic conflict
policy of `AtomicOperation::Add` with
`AtomicOrdering::SequentiallyConsistent`, and an explicit
`MustAliasExact` rule from input zero to output zero. The language layer checks
three inputs, int32 indices, update shape, output shape, and matching value
dtypes. The exact alias is semantic: the lowered scatter first preserves the
base in the output and then applies atomic updates to that same output.

### Sort and random

Sort compares direction and stability, checks the axis requirement, and rejects
`emit_indices`. The direct table contains only ascending, non-stable,
all-axis f32 sort. Language validation still checks one input, output shape,
the int32 representability of the sorted axis, and optional index output rules.

Random recipes match only `UniformF32` or `NormalF32` distributions. Language
validation requires no inputs, one output of the distribution dtype, and
exactly ten Philox rounds. Bernoulli and integer-uniform maps remain valid
general primitives but do not satisfy either direct recipe.

`IndexMap` matches any `PrimitiveKind::IndexMap` at this layer. Language
validation supplies the remaining contract: no inputs, one int32 output, and a
strictly positive optional modulus. Its checked affine arithmetic is
`start + element_index * element_step + loop_iteration * iteration_step`, with
positive Euclidean modulus when present.

## Lowering entry points and errors

### `lower_primitive`

The public operation boundary is:

```rust
pub fn lower_primitive(
    descriptor: OperationDescriptor,
    request: PrimitiveRequest<'_>,
    hardware: LoweringHardware,
) -> OperationResult<LoweredProgram>
```

It first inspects `descriptor.lowering`:

* `LoweringAvailability::Primitive(recipe)` continues.
* Scalar, composition, workspace, and non-calculation descriptors return
  `OperationErrorKind::WrongLoweringKind` with the descriptor's operation ID.
* An unsupported descriptor returns `UnsupportedLowering`, including the
  recipe definition and `UnsupportedReason`.

For a direct recipe, `lower_recipe` calls `matches`. A failed match returns
`PrimitiveRecipeMismatch` with the human-readable `definition`. A successful
match delegates to `recipe_primitives::lower`; any backend-neutral lowering
error becomes `PrimitiveLoweringFailed` and is annotated with the descriptor's
operation ID by `lower_primitive`.

### `lower_index_map`

`lower_index_map` calls the same private `lower_recipe` with
`PrimitiveRecipe::IndexMap`. It intentionally has no descriptor or registry
symbol. The helper's documented ABI is no inputs, one int32 output, and no
aliases; output shape determines the generated index count, while affine
parameters stay in the immutable language kernel and lowered stage. Because it
has no descriptor, its errors do not receive an `OperationId` wrapper.

### Callee error vocabulary

[`primitives/src/error.rs`](../../../primitives/src/error.rs) defines
`LoweringErrorKind`:

| Kind | Meaning at this boundary |
| --- | --- |
| `InvalidLanguage` | `PrimitiveKernel::validate` rejected tensor, arity, dtype, shape, alias, or primitive parameters. The converted `LanguageError` retains kernel and value context. |
| `MissingTensor` | A kernel input or output is absent from the borrowed tensor index. |
| `ArithmeticOverflow` | A checked static extent, byte size, stage, buffer, scratch, resource, or identifier calculation overflowed. |
| `InvalidStaticAccess` | A broadcast or buffer view cannot be represented as the required static access. |
| `InvalidLoweredProgram` | Hardware limits, canonical stage construction, program validation, or digest construction produced an invalid program. |

`LoweringError` carries a detail string and optional `KernelTemplateId` and
`ValueId`. `OperationError` carries its machine-readable kind, detail, and
optional `OperationId`; neither error path creates a fallback.

## `recipe-primitives` program construction

The public callee is
[`primitives/src/lower.rs`](../../../primitives/src/lower.rs)::`lower`. Its
input is one `PrimitiveKernel`, the complete tensor index, and measured
`LoweringHardware`:

```text
LoweringHardware {
    subgroup_lanes,
    maximum_workgroup_lanes,
    maximum_shared_memory_per_workgroup,
}
```

The hardware precondition is explicit: subgroup lanes are nonzero and a power
of two, maximum workgroup lanes are at least one subgroup, and measured shared
memory is nonzero. The lowering never chooses a device, driver API, vendor
library, target, entry symbol, or native artifact.

The callee then:

1. Calls `kernel.validate(tensors)` and converts a `LanguageError` into
   `InvalidLanguage`.
2. Adds each distinct input and output tensor to `ProgramBuffer` records. A
   tensor buffer preserves dtype, complete shape, offset, strides, and backing
   storage bytes and has `ExternalValue` lifetime.
3. Dispatches on `PrimitiveKind` to one of the `lower_*` implementations.
4. Allocates typed one-dimensional `ProgramScratch` buffers for reduction,
   scan, and sort intermediates, and at most one four-byte int32
   `ProgramFaultFlag` buffer per program.
5. Chooses power-of-two workgroup widths from logical extents, the declared
   tree width, measured workgroup limits, subgroup limits, and shared-memory
   capacity. `workgroups` is always the ceiling of logical lanes divided by
   workgroup lanes.
6. Emits dense `ProgramStage` IDs. Every stage depends on the immediately
   preceding stage, so multi-pass algorithms are an explicit immutable chain.
7. Aggregates exact stage resources, computes the domain-separated canonical
   digest, and calls `LoweredProgram::validate` before returning.

### Program model

[`primitives/src/model.rs`](../../../primitives/src/model.rs) defines the
immutable program contract:

* `LoweredProgram` records schema version, source kernel and arity, the complete
  source alias matrix, buffers, stages, aggregate resources, and digest.
* `ProgramBuffer` records dense `BufferId`, `BufferOrigin`, lifetime, dtype,
  shape, and `StaticAccess` (logical extents, offset, strides, storage bytes).
  Origins are a source tensor, an ordinal scratch purpose, or the shared fault
  flag.
* `BufferBinding` records access mode (`Read`, `Write`, `ReadWrite`, or
  `ReadWriteAtomic`) and a complete static view.
* `ProgramStage` records dense ID, dependency IDs, one-dimensional dispatch
  geometry, bindings, synchronization points, atomic contracts, optional fault
  contract, exact resource bounds, and `StageKind`.
* `StageKind` is the owned algorithm contract consumed by the kernel emitter:
  `ScalarMap`, `Fill`, `Copy`, fixed-tree reduction and scan stages,
  `ScanUniformCombine`, tiled contraction, gather, scatter, histogram clear and
  accumulate, stable-sort initialization/compare/finalize, `IndexMap`, and
  `Philox4x32_10`.
* `ProgramResourceBounds` aggregates total FLOPs, integer and atomic work,
  persistent scratch bytes, fault bytes, peak shared and private bytes, and
  maximum workgroup lanes.

### Stage lowering by primitive kind

The concrete lowerers encode the following behavior. Zero-element cases are
handled deliberately by each lowerer rather than by an invalid dispatch.

| Language primitive | Emitted program behavior |
| --- | --- |
| `Elementwise` | One `ScalarMap` stage with a complete `KernelTemplate`, broadcast input views, scalar private storage, and an arithmetic fault contract when the scalar program requires one. |
| `Reduce` | One or more `FixedTreeReduce` passes. Intermediate value and index buffers are typed scratch, padding uses the operator identity, and value/index ties retain the lowest logical source index. A zero-width reduced domain with nonempty outputs uses explicit sum/product/`Any`/`All` identities; empty min/max is rejected by language validation. A completely empty output emits no dispatch. |
| `Scan` | Local fixed-tree stages use canonical Blelloch upsweep and downsweep steps. Hierarchical scans allocate block totals and add `ScanUniformCombine` stages between levels. User inclusive/exclusive mode and reverse direction are retained in the stage contract. |
| `Contraction` | One `TiledContraction` stage with measured tile dimensions, a power-of-two reduction tile, row-major contracted-coordinate order, and a complete private accumulator per lane. The current physical plan selects `ContractionStrategy::Direct` and zero shared bytes. |
| `Gather` | One checked indexed stage when the output has elements. `Reject` bounds allocate the shared fault flag and a release atomic exchange before an invalid address can be formed. Clamp and wrap are represented without that fault contract for general graph primitives. |
| `Scatter` | A nonempty base first emits a `Copy` stage from the base input to the aliased output. Updates then use either ordinary writes for unique indices or `ReadWriteAtomic` with the exact declared operation and ordering. Reject bounds publish the checked index fault. |
| `Histogram` | A `HistogramClear` stage always initializes bins. Nonempty input then emits `HistogramAccumulate` with direct int32 or truncate-toward-zero f32 bin mapping, atomic bin updates, and a checked bin fault. |
| `Sort` | Scratch value and int32-index arrays hold a least-power-of-two padded domain. Initialization, fixed bitonic compare-exchange phases, and finalization are separate stages. Equal values tie by original axis index, f32 comparison uses IEEE total order, and padding follows all valid elements. |
| `IndexMap` | One checked arithmetic stage with output-only binding and an arithmetic-domain fault flag. The stage records the affine parameters and positive-modulus policy, and includes loop iteration in the target ABI only when `iteration_step` is nonzero. |
| `Random` | One `Philox4x32_10` stage. Constants, ten rounds, element and iteration counter words, stream/key material, run ID and kernel ID folding, unbiased integer mapping, and owned Box-Muller normal mapping are all explicit in the stage contract. |

`Fill` and `Copy` are internal materialization stages, not additional public
operation semantics. They are used for empty outputs, scatter-base preservation,
and concrete composition plumbing.

## Validation and invariants

Validation occurs at three boundaries and each boundary has a different
authority.

### Language kernel validation

[`language/src/primitive.rs`](../../../language/src/primitive.rs)::
`PrimitiveKernel::validate` first resolves every input and output tensor,
requires every input/output alias pair exactly once, and then validates the
selected `PrimitiveKind`:

* Elementwise scalar programs must validate, have matching arities and dtypes,
  broadcast-compatible inputs, and exactly shaped outputs.
* Reductions require one input, one or two outputs according to `ReduceResult`,
  valid axes and tree lanes in `1..=1024`, compatible dtypes and reduced shape,
  legal value/index operators, and no empty min/max domain.
* Scans require one input and output, an in-range axis, valid tree lanes,
  matching identity dtype, and matching output shape.
* Contractions require two operands, one output, nonempty contract pairs,
  unique in-range axes, matching paired extents and dtypes, and the exact
  derived output shape.
* Gather and scatter require int32 index tensors and derived update/output
  shapes. Scatter preserves the declared conflict operation and ordering.
* Histograms require valid bin count, weighted input shape/type when selected,
  and a one-dimensional output of the exact bin count.
* Sort checks axis range, int32 index representability, value and optional
  index outputs.
* Index maps require no inputs, one int32 output, and positive modulus when
  present. Random maps require no inputs, one distribution-typed output,
  exactly ten Philox rounds, and valid distribution parameters.

The language `work` method reuses this validation and derives checked FLOP
bounds. It is accounting only; it does not lower or execute the kernel.

### Lowered program validation

[`primitives/src/validate.rs`](../../../primitives/src/validate.rs)::`validate`
collects all `ProgramValidationError { path, detail }` values rather than
silently normalizing an invalid program. It checks:

1. Schema version equals `LOWERED_PROGRAM_SCHEMA_VERSION` (currently `2`).
2. The source alias list is a complete, unique, in-range input/output matrix.
3. Buffer IDs are dense and ordered; origins are unique; tensor, scratch, and
   fault lifetimes agree with dtype, shape, and storage contracts; access ranks,
   spans, storage bytes, and non-atomic writable injectivity are valid.
4. Stage IDs are dense, dependencies are exactly the previous stage, geometry
   is nonzero, and workgroup count is the required ceiling.
5. Bindings reference existing buffers with the same dtype and an in-storage
   static view. Atomic contracts have matching atomic bindings and unique
   buffer/domain/operation/ordering tuples.
6. Fault contracts guard before address formation, publish the declared int32
   flag through a release exchange, and list that publication among stage
   atomics.
7. Every `StageKind` has its canonical tree, sort network, Philox constants,
   binding count, fault relationship, tile relationship, and stage-specific
   invariants.
8. Stage resource bounds and synchronization counts equal exact arithmetic
   derived from the stage kind and geometry. Program aggregate resources equal
   the exact sum and peak of all stages and buffers.
9. The stored digest equals `canonical_digest`, which is domain separated from
   other Recipe digests and covers canonical program contents.

These checks make the returned program safe to hand to planner and kernel
without trusting caller-selected fragments, statuses, or implementation
details.

## Callers and complete end-to-end role

### Registry and facade callers

The only in-repository caller of `PrimitiveRecipe::for_symbol` is
`registry::lowering`. `OperationRegistry::resolve_unique` and `resolve_exact`
are exposed through [`src/facade.rs`](../../../src/facade.rs)::`operations`.
The facade re-exports `PrimitiveRequest`, `LoweredProgram`, and
`LoweringHardware`, and its `operations::lower_primitive` is a thin call to
`recipe_ops::lower_primitive`.

`lower_primitive` itself is therefore an explicit advanced boundary. It does
not discover tensors, probe devices, choose a target, allocate memory, or
schedule a run. The caller must supply the already-resolved descriptor, the
typed kernel, the complete tensor index, and measured hardware limits.

`lower_index_map` is exported from `ops/src/lib.rs` and the facade's operation
module re-exports the request and result types, but no production module calls
the helper by name. Graph-producing code emits `PrimitiveKind::IndexMap` and
the planner's ordinary graph lowering handles it uniformly.

### Graph and materialization callers

Domain builders in `ops/src/*`, `training/src/compile.rs`, and
`training/src/inference.rs` construct `PrimitiveKernel` nodes directly or via
concrete composition materializers. They preserve aliases, iteration domains,
typed shapes, and prepared parameters in the graph. Composition preparation
resolves shape and typed-parameter bounds, reserves identities, emits only
validated primitive nodes, and finishes with `CalculationGraph::validate`.

Those builders do not call a legacy symbol to infer a kernel. A direct legacy
descriptor can be lowered through the facade API; a structured descriptor is
materialized first; both routes converge at the same language kernel and
`recipe-primitives` program contract.

### Planner caller

[`planner/src/planner.rs`](../../../planner/src/planner.rs)::`lower_programs`
selects one common `LoweringHardware` from available measured calculation
devices, calls `recipe_primitives::lower` for every graph node, and immediately
calls `LoweredProgram::validate`. It rejects the graph if lowering or any
program invariant fails.

For each planned invocation, `lower_program_invocation`:

1. Materializes every tensor, scratch, and fault buffer into a physical value
   while preserving `BufferLifetime` and static views.
2. Creates one calculation task per immutable stage and translates stage
   dependencies plus read-after-write readiness into task dependencies.
3. Converts each binding and resource bound into an
   `ArtifactBuildRecipe` whose program digest, stage ordinal, dispatch geometry,
   work, fault flag, and resource envelope must match exactly.
4. Records output writers, source alias contracts, and fault-readback metrics.
   A required exact alias is rejected if the typed static views differ.
5. Leaves placement, scheduling, arena allocation, transfer routes, and loop
   domains to the planner and scheduler. None of those concerns are encoded as
   extra primitive semantics.

### Kernel and runtime caller

[`kernel/src/lib.rs`](../../../kernel/src/lib.rs)::`lower_stage` receives the
complete `LoweredProgram`, one stage-scoped artifact build contract, a target,
and lowering options. Before emitting LLVM it validates the program, build,
target, digest, source kernel, stage ordinal, stage identity, dispatch,
bindings, work bounds, resource envelope, and fault ABI. It then reuses the
scalar emitter for `ScalarMap` and dispatches every other `StageKind` to the
Recipe-owned LLVM emitter in `kernel/src/stage.rs`.

The target emitter implements the encoded fixed trees, canonical contraction
order, checked index guards, atomic memory orderings, total-order sort
comparator, and Philox constants. It does not replace them with a runtime
algorithm or vendor library. Native artifact realization and execution happen
after this contract has been validated; the primitive module itself remains
pre-loop AOT preparation.

Fault-bearing stages publish a four-byte device flag before returning from a
rejected lane. The planner schedules a loop-phase readback metric after all
calculations in the fault cohort. The runtime can therefore report the
authoritative device fault without allowing an invalid payload address to be
issued.

## Non-goals and maintenance rules

* Adding a symbol requires an `operation-surface.txt` row and one explicit
  `SYMBOL_RECIPES` entry, then the associated source-qualified dtype, family,
  alias, determinism, and end-to-end graph contract must be reviewed.
* Do not add a direct recipe for an operation whose semantics are a scalar
  program, structured composition, workspace query, host behavior, or
  lifecycle declaration. The registry's first-match classification must remain
  authoritative.
* Do not put extents, tree widths, keys, iteration values, or hardware limits
  in `PrimitiveRecipe`; those values belong to the typed kernel, measured
  lowering hardware, or preparation state that already owns them.
* Do not bypass `PrimitiveKernel::validate`, `LoweredProgram::validate`, the
  canonical digest, planner artifact checks, or the target stage contract.
* A failed match, unsupported descriptor, invalid language kernel, arithmetic
  overflow, invalid access, or invalid lowered program is an observable error.
  There is no alternate implementation or compatibility shim.
