# Inference, quantization, and diffusion materialization

This page documents the source file
[`ops/src/materialize/inference_quantization_diffusion.rs`](../../../src/materialize/inference_quantization_diffusion.rs)
and the descriptors that it is reserved to materialize. The file is currently
an explicit fail-closed boundary. It contains no tensor ABI, scalar program,
primitive parameter policy, workspace formula, or graph emitter. The registry
therefore exposes the operation intent, while a valid request still stops with
`MissingConcreteFormula` before a calculation graph is built.

That distinction is deliberate. A `CompositionRecipe` is a checked description
of primitive families and static repeat structure. It is not permission to
invent tensor names, shapes, prepared facts, aliases, or scratch storage. This
module is the owner slot for the inference, quantization, and diffusion rows;
until it claims an exact source-qualified row, no neighboring family may
claim it by symbol resemblance.

## The two Rust cases in this module

The implementation has exactly two private cases:

| Rust case | Inputs | Current result | State or graph effect |
| --- | --- | --- | --- |
| `supports(_descriptor)` at lines 4-4 | An `OperationDescriptor` copied from the registry | Always `false` | None. It does not inspect the symbol, source, family, lowering, or dtype contract. |
| `dispatch(_request, _emitter)` at lines 6-8 | A `MaterializationRequest` and the shared `Emitter` | Always `FamilyDispatch::NotOwned` | None. It does not read the request, allocate an intermediate, emit a primitive, consume a stage, or mutate the emitter. |

The only imports are `Emitter`, `FamilyDispatch`, and
`MaterializationRequest` from the parent module plus `OperationDescriptor`.
There is no `OPERATIONS` table and no symbol match. In particular, the
dispatcher is not an unfinished implementation that happens to emit an empty
graph. It explicitly declines ownership of every descriptor.

`FamilyDispatch` is private to `materialize.rs` and has two cases:

```text
NotOwned
Owned(OperationResult<()>)
```

Other family modules use an exact `(descriptor.symbol, descriptor.source)`
table in `supports`, then return `Owned(result)` from `dispatch`. This module
does neither. The pair of no-op functions is therefore the complete current
semantic boundary for the family.

## Descriptors reserved for this family

The operation inventory is parsed from
[`operation-surface.txt`](../../../../operation-surface.txt). The current rows
below are the only rows whose registry recipes select
`OperationFamily::Inference`, `OperationFamily::Quantization`, or
`OperationFamily::Diffusion` and have no concrete family materializer. Ordinals
are zero-based `OperationId::ordinal()` values. The source line is the physical
line in the inventory, not a Rust line in the current checkout.

| Ordinal | Symbol and source | Registry recipe | Primitive sequence | Payload contract | Legacy marker |
| ---: | --- | --- | --- | --- | --- |
| 4 | `convert`, `recipe-infer/src/dequant.rs:170` | `checked_dequantization` | `Elementwise` map | `F32OrI32` | none |
| 5 | `dequant_f32`, `recipe-infer/src/dequant.rs:215` | `checked_dequantization` | `Elementwise` map | `F32OrI32` | none |
| 7 | `generate`, `recipe-infer/src/llm.rs:2876` | `bounded_autoregressive_generation` | repeat `Gather -> Contraction -> Elementwise -> Reduce -> Scatter` | `F32AndI32` | none |
| 79 | `gpu_convert`, `gpu-core/src/infer_ops.rs:300` | `checked_dequantization` | `Elementwise` map | `F32OrI32` | `DynamicQuantized` |
| 100 | `gpu_diffusion_commit`, `gpu-core/src/diffusion.rs:65` | `diffusion_commit` | `Elementwise` map | `F32AndI32` | none |
| 101 | `gpu_diffusion_sample`, `gpu-core/src/diffusion.rs:92` | `diffusion_sample` | `Random -> Elementwise` | `F32` | none |
| 117 | `gpu_entropy_gated_step`, `gpu-core/src/diffusion.rs:39` | `entropy_gated_diffusion_step` | `Elementwise -> Reduce` | `F32AndI32` | none |
| 276 | `gpu_quantize_features`, `gpu-core/src/encoding.rs:144` | `feature_bin_quantization` | `Gather -> Elementwise` | `F32AndI32` | none |
| 400 | `gpu_vae_backward_latent`, `gpu-core/src/kernels.rs:2357` | `vae_latent_backward` | `Elementwise` map | `F32` | none |
| 405 | `greedy`, `recipe-infer/src/llm.rs:143` | `greedy_token_selection` | `Elementwise -> Reduce` | `F32AndI32` | none |
| 406 | `greedy_windowed`, `recipe-infer/src/llm.rs:147` | `greedy_token_selection` | `Elementwise -> Reduce` | `F32AndI32` | none |
| 407 | `last_logits`, `recipe-infer/src/llm.rs:175` | `last_token_logits` | `Gather -> Elementwise` | `F32AndI32` | none |

These twelve rows are source-qualified even where the public symbol is unique.
The eventual materializer must therefore use the same pair, not a broad
`symbol` test. `gpu_convert` is intentionally marked
`LegacyDType::DynamicQuantized`; the marker records a removed legacy payload
contract and never authorizes dynamic quantized values in a canonical graph.

### How each recipe is described

The recipe table above is generated by
`CompositionRecipe::for_entry` in `ops/src/composition.rs`.
`CompositionStep` names the primitive family and a human-readable role. The
shared constants have these meanings:

* `MAP` is one typed `Elementwise` scalar SSA map. The actual scalar formula
  remains unspecified for every row in this module.
* `REDUCE` is one fixed, recorded `Reduce` tree. It is not a host reduction
  loop and does not select a vendor reduction implementation.
* `GATHER` is a checked int32-indexed read. Concrete gather bounds must be
  `IndexBounds::Reject`.
* `CONTRACT` visits contracted coordinates in canonical order.
* `SCATTER` writes with a concrete conflict policy. A future emitter must state
  whether the destinations are unique or use an allowed atomic policy.
* `RANDOM` is a counter-keyed Philox4x32-10 primitive. A descriptive random
  step does not by itself define its key layout, distribution parameters, or
  output tensor.

`generate` uses `GENERATION_STEPS`, one repeat whose bound is the prepared
parameter `maximum_generated_tokens`. Its body is exactly:

1. gather checked model, token, and KV-state slices;
2. contract model values in canonical order;
3. apply owned activation and normalization formulas;
4. select or score the next token with fixed tie breaking;
5. scatter the next token and KV-state slots into prepared disjoint ranges.

`gpu_diffusion_sample` uses `RANDOM_MAP`, so its descriptive path first creates
counter-keyed normal noise and then applies a prepared reverse-diffusion mean
and variance update. `gpu_entropy_gated_step` uses `MAP_REDUCE`: its map forms
the operation-specific entropy terms and its reduction combines them before a
concrete gate selects state. `gpu_diffusion_commit` and
`gpu_vae_backward_latent` are one-map descriptions, but the map formulas and
their exact input and output names are still absent.

`gpu_quantize_features` describes a fixed-iteration binary search over prepared
f32 edges and an int32 bin output. The old source implementation used a
data-dependent `while` loop and a legacy byte output; neither behavior can be
copied into the current graph without a checked static iteration bound and a
canonical int32 tensor ABI.

`greedy` and `greedy_windowed` describe the same two-stage selection: map the
logit and index values into the typed comparison form, then reduce with
lowest-index ties. `last_logits` describes a checked gather of the statically
verified final sequence position followed by an identity map. The source
specific helper currently matches these symbols without a source predicate,
but the inventory has one source row for each, and concrete dispatch remains
source-qualified.

## Registry classification before materialization

`registry::describe` constructs each `OperationDescriptor` in a fixed order.
For these rows the relevant path is:

```text
raw symbol and source
    -> scalar recipe lookup (no match)
    -> primitive recipe lookup (no match)
    -> workspace formula lookup (no match)
    -> non-calculation lookup (no match)
    -> CompositionRecipe::for_entry (match)
    -> OperationDescriptor
```

Composition lookup happens before legacy-dtype exclusion. Consequently,
`gpu_convert` is a composition with a `DynamicQuantized` marker, rather than an
`Unsupported(LegacyDTypeExcluded)` descriptor. The marker is still visible to
callers through `legacy_dtype`, and the composition payload is still the
canonical `F32OrI32` contract.

For a composition, `dtype_contract` is taken directly from
`CompositionPayload`. The resulting contracts are therefore exactly the table
above, not the payload types used by the historical GPU wrappers. A non-`gpu_`
target such as `convert`, `generate`, or `greedy` receives the registry's
default `NoAlias` contract. A `gpu_` target receives
`AliasContract::OperationSpecific` unless its name ends in `_into`; none of
these twelve rows ends in `_into`. No alias promise is turned into a graph
because this module emits no kernel.

`determinism` classifies every composition as
`DeterminismContract::FixedPrimitiveOrder`. That is the descriptor-level
classification even for `gpu_diffusion_sample`, whose recipe contains the
counter-keyed `RANDOM` primitive. The random primitive still requires a
counter and distribution ABI when a concrete emitter is eventually added;
the current module does not supply one.

`OperationRegistry::resolve_unique` is sufficient for all twelve symbols in
the current inventory. `resolve_exact` remains the authoritative API when a
caller has a source-qualified pair. Unknown symbols return
`UnknownOperation`; an actually duplicated public symbol returns
`AmbiguousSymbol` from `resolve_unique` instead of selecting a neighboring
source.

## Materialization control flow

The public root facade re-exports this boundary as
`recipe::operations::materialize`. It accepts one immutable
`MaterializationRequest` containing:

* the source-qualified `OperationDescriptor`;
* named input and output `Tensor` declarations;
* the name of one input whose shape resolves repeat bounds;
* `PreparedParameters`, whose values are only `U64`, `I32`, `F32Bits`, or
  `Bool`;
* a caller-reserved `IdentityNamespace` for intermediate values and kernels;
* a `ByteCount` workspace limit.

`materialize_composition` in `ops/src/materialize.rs` has a strict order:

1. `validate_request` checks that the descriptor is a composition, that both
   boundary lists are nonempty, that names and tensor IDs are unique, that all
   tensors validate, and that input/output external flags are correct.
2. `has_concrete_materializer` asks each family module's `supports` function,
   including this module's unconditional `false`.
3. If no module claims the descriptor, return `MissingConcreteFormula` with the
   symbol and source. The detail says that tensor ABI, scalar SSA, primitive
   parameters, and workspace policy are not concrete.
4. Only a claimed descriptor reaches iteration-shape lookup,
   `expand_composition`, `Emitter::new`, `dispatch_concrete`, and
   `Emitter::finish`.

For every well-formed request built from one of the twelve descriptors, step 2
is false and step 3 is the observed result. Therefore:

* the iteration-shape input name is not looked up;
* repeat bounds are not resolved;
* missing or mistyped prepared parameters are not examined by expansion;
* no `ResolvedComposition` is returned;
* no `GraphBuilder` is created;
* no value or kernel identity is consumed;
* no workspace object or byte count is allocated;
* no `StageEmission` is recorded;
* no `CalculationGraph::validate` call occurs for the requested operation.

Invalid request structure is still checked first. For example, a request with
no inputs returns `InvalidMaterializationRequest` before the missing-formula
error, and a request whose descriptor is not a composition returns
`WrongLoweringKind`. This ordering keeps boundary failures distinct from the
honest statement that a valid composition lacks a concrete ABI.

If a future change makes `supports` return true for a row but leaves
`dispatch` returning `NotOwned`, the request would pass step 2, be expanded,
and reach `dispatch_concrete`. Every family would decline it and the final
error would be `GraphMaterializationFailed` with
`concrete dispatch is missing for source-qualified operation ...`. The current
unconditional `false` prevents that unreachable-in-normal-operation state for
this module. A concrete implementation must make the support table and the
dispatch match, rather than relying on this fall-through error.

## Dispatcher ownership and no fallthrough

`dispatch_concrete` probes family modules in this exact order:

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

Each `NotOwned` result allows the next module to inspect the same request. An
`Owned(result)` returns immediately, preserving the concrete module's error or
success. A request that reaches the end without ownership gets
`GraphMaterializationFailed`; there is no symbol-based fallback to an
adjacent family. The inference, quantization, and diffusion module appears in
the middle of this sequence, but its current `NotOwned` result has no effect on
the valid current path because `has_concrete_materializer` has already rejected
all of its rows.

`has_concrete_materializer` uses the same family order as the dispatcher and
ORs each `supports` result. This keeps the manifest and materialization gate
consistent: a row enters the concrete path only if at least one family claims
it. Since this module claims none, its rows are not accidentally removed from
the manifest by a dispatch-only change.

## Remaining-composition manifest

`remaining_composition_manifest` iterates the complete registry, keeps only
`LoweringAvailability::Composition` descriptors, and filters with the same
`has_concrete_materializer` predicate. Every row in the table above is emitted
with its recipe name and all four missing components:

```text
TensorAbi
ScalarFormula
PrimitiveParameters
WorkspacePolicy
```

The manifest is evidence of the preparation boundary, not an execution plan.
Calling `recipe::operations::remaining_compositions()` can list these rows,
and calling `recipe::operations::validate_composition(descriptor)` can validate
their nonempty names, roles, repeat bodies, and bounds. Neither operation
claims that tensor wiring, scalar formulas, prepared verification facts, or
workspace accounting already exist. `validate_composition` is therefore able
to succeed for a row whose `materialize` call must still fail closed.

## Historical source provenance and canonical replacement

The source strings in the inventory point to the pre-reorganization legacy
tree. Those paths are not current Rust modules in this checkout. The historical
implementation explains the intent recorded by the recipes, but it is not a
callee of this module and it is not an acceptable fallback:

* The old `recipe-infer/src/dequant.rs` `convert` path decoded complete source
  blocks through a runtime codec, accumulated host values, and encoded a
  destination format. `dequant_f32` converted the result to little-endian
  f32 bytes. The old `gpu_convert` wrapper accepted runtime dtype codes and
  block sizes, with a special Q8_0 encoder. These dynamic format and packed
  quantized payloads are why the current registry uses a canonical
  `F32OrI32` description and marks `gpu_convert` as `DynamicQuantized`.
* The old `recipe-infer/src/llm.rs` `greedy` and
  `greedy_windowed` functions ran model forward work, maintained a KV cache,
  and selected tokens on the host. `last_logits` exposed the final logits
  slice. The current recipes retain the mathematical selection and final
  position gather while requiring a bounded static graph, checked int32 token
  indexes, and no host payload loop.
* The old `gpu-core/src/diffusion.rs` wrappers launched f64 HIP kernels. The
  entropy-gated step computed a row maximum, normalized exponentials, Shannon
  entropy, and an accept-versus-reuse gate. Commit updated an in-progress
  canvas only for uncommitted positions. The sample helper owned a host loop,
  downloaded a committed marker, and stopped when every position was done.
  The current recipe descriptions replace that dynamic f64 path with each
  operation's canonical f32 or f32/int32 payload contract, explicit map/reduce
  or random/map stages, and prepared static bounds. No host loop is represented
  in a `CalculationGraph`.
* The old `gpu-core/src/encoding.rs` quantization kernel binary-searched
  f64 edge tables and wrote a byte bin index. The current
  `feature_bin_quantization` description requires prepared f32 edges, fixed
  search iterations, checked int32 indexes, and an operation-specific tensor
  ABI that this module has not yet supplied.
* The old `gpu-core/src/kernels.rs` VAE latent backward wrapper accepted f64
  buffers and launched one device kernel. The current
  `vae_latent_backward` recipe records an analytic f32 map with explicit scalar
  order, but its input names, output names, KL parameter facts, and workspace
  policy remain absent.

The historical code is useful provenance only. Registry presence, a recipe
definition, or a source path does not make that code reachable from the
current `recipe::operations::materialize` API.

## Callers, callees, and state boundaries

### Root facade and direct callers

`src/facade.rs` exposes the dependency-clean `recipe::operations` facade:

* `registry`, `all`, `resolve`, and `resolve_exact` forward to
  `OperationRegistry`;
* `validate_composition` forwards the descriptive recipe check;
* `materialize` forwards the immutable request to
  `recipe_ops::materialize_composition`;
* `remaining_compositions` forwards the fail-closed manifest.

The module has no public constructor, no public state, and no direct backend
dependency. The only state it can observe is the descriptor, request, and
emitter passed by the parent materializer.

### Training compiler caller

`training/src/compile.rs` has a `Compiler::materialize` method. It:

1. clones compiler tensor contracts into named input and output declarations;
2. marks inputs as `external_input` and outputs as `external_output`;
3. reserves 64 value identities and 64 kernel identities with checked adds;
4. resolves the public symbol with `operation_registry().resolve_unique`;
5. constructs `MaterializationRequest` with an unlimited `ByteCount` limit;
6. calls `materialize_composition`;
7. inserts only the returned tensors and nodes into the compiler graph.

A target-family operation fails at step 6 with `OperationErrorKind::MissingConcreteFormula`.
The `From<OperationError>` conversion in `training/src/error.rs` wraps the
display text as `TrainingCompileErrorKind::Operation`. Since insertion happens
after the call, no partial target graph enters the training program.

### Inference compiler caller

`training/src/inference.rs` has the same materialization boundary. Its
`InferenceGraphCompiler::materialize` marks boundary flags, reserves the same
64-value and 64-kernel namespace, resolves the symbol, and calls the shared
function. A missing concrete formula becomes
`InferenceCompileErrorKind::Operation`; tensor contracts and kernel domains are
inserted only after success.

Current dense, KNN, Bayes, and GGUF inference compilation uses concrete
materializers and specialized graph builders for the operation families it
actually needs. The twelve descriptors documented here are not a hidden route
inside those model compilers. A caller that explicitly asks the generic
materialization helper for one of them observes the fail-closed error instead
of silently selecting a legacy kernel.

### Public inference and native execution

The root `src/inference.rs` boundary validates declarations, loads an `.ogdl`
or `.gguf` model, prepares target-free rows, and calls one of the compiled
inference builders. Native preparation, planner construction, allocation, and
execution happen only after a complete graph has been returned. If a target
family row were selected by a compiler and its materialization failed, the
error propagates as `InferenceError::Compile`; no native profile, device image,
queue, or lifecycle state is created for that row.

The native executor therefore never receives a `MaterializedComposition` from
this module in the current checkout. It cannot load a kernel, schedule a task,
or publish output state for an operation that stopped at `MissingConcreteFormula`.

## Invariants

The current implementation and its surrounding boundary preserve these
invariants:

1. Ownership is exact and source-qualified. A matching symbol from another
   legacy source cannot fall through to this family.
2. `supports` and `dispatch` are both total over descriptors and both decline
   every descriptor. There is no accidental partial table.
3. A descriptive composition never emits a placeholder graph. The concrete
   ABI, scalar SSA, primitive parameters, and workspace policy must all exist
   before `materialize_composition` can return `Ok`.
4. Canonical calculation payloads are f32 and int32. Historical f64, f16, u8,
   and dynamic quantized buffers are not reintroduced through this module.
5. Any eventual repeat is resolved before execution. The shared expander
   records every bound and dependency and rejects more than one million
   expanded primitive steps.
6. Any eventual gather rejects out-of-range int32 indexes. Any eventual
   scatter or random stage must state its conflict or key policy in the
   concrete ABI.
7. Intermediate values and kernel IDs come only from the caller's reserved
   `IdentityNamespace`; no hard-coded identity or implicit allocation exists.
8. Workspace bytes are accounted before execution and compared with the
   request limit. The current module accounts zero bytes because it emits no
   graph.
9. There is no callback, host payload loop, CPU substitute, retry, or adjacent
   family fallback in the current path.
10. A status, manifest row, descriptor definition, or successful recipe
    validation is not runtime evidence. Runtime evidence would require a
    concrete graph, native preparation, and an end-to-end execution, none of
    which this module currently supplies.

## Failure vocabulary and ordering

The single `OperationError` type carries an `OperationErrorKind`, detail, and
optional `OperationId`. For this module the observable cases are:

| Condition | Result | Where it occurs |
| --- | --- | --- |
| Symbol is absent | `UnknownOperation` | `OperationRegistry::resolve_unique` or `resolve_exact` |
| Symbol has several source rows and no source was supplied | `AmbiguousSymbol` | `resolve_unique` |
| Descriptor is scalar, primitive, workspace, non-calculation, or unsupported | `WrongLoweringKind` | `validate_request` or `expand_composition` |
| Boundary lists are empty, names or IDs repeat, tensors are invalid, or external flags are wrong | `InvalidMaterializationRequest` or a language error | `validate_request`, before the support probe |
| Descriptor is one of the twelve rows and the boundary request is valid | `MissingConcreteFormula` | `materialize_composition`, after `validate_request` and before shape lookup |
| A required repeat bound is absent or mistyped when expansion is called directly | `MissingPreparedParameter` or `PreparedParameterTypeMismatch` | `expand_composition`; not reached by current materialization for these rows |
| A shape axis or prepared repeat value cannot be resolved when expansion is called | `IterationBoundUnresolved` | `expand_composition`; not reached by current materialization for these rows |
| More than one million primitive stages would be expanded | `CompositionExpansionOverflow` | shared expansion; not reached after the current concrete gate |
| A future support table claims a row while this `dispatch` still declines | `GraphMaterializationFailed` | final `dispatch_concrete` fall-through, after expansion |
| A future emitter exhausts reserved values or kernels, exceeds workspace, or fails graph validation | `IdentityNamespaceExhausted`, `WorkspaceLimitExceeded`, or `GraphMaterializationFailed` | shared `GraphBuilder` and `Emitter`; no current target path reaches them |

`OperationError::Display` renders the kind and detail, then appends
`[operation N]` when an operation identity is present. Training and inference
callers retain this text under their `Operation` error kinds; they do not
replace it with a success status or a legacy execution attempt.

## End-to-end path observed today

For a direct caller that requests `gpu_quantize_features`, the complete path is:

```text
operation-surface.txt
  -> build.rs generated RAW_OPERATION_SURFACE
  -> OperationRegistry::resolve_unique("gpu_quantize_features")
  -> CompositionRecipe::for_entry
       feature_bin_quantization / Gather -> Elementwise
  -> MaterializationRequest
  -> validate_request
  -> has_concrete_materializer
       inference_quantization_diffusion::supports(...) == false
  -> MissingConcreteFormula [operation 276]
```

The same shape applies to the other eleven rows, with their own descriptor
ordinal. `expand_composition`, `Emitter`, `GraphBuilder`, planner, native
image compiler, executor, and output publication are not called. A training or
inference caller receives its typed operation compilation error at the shared
boundary. This is the complete current behavior, not a skipped test or a
hardware-dependent branch.

For comparison, a concrete row such as `gpu_linear_into` passes the support
probe, resolves its input shape, expands its recipe, emits checked stages,
validates its graph, and is then inserted by the compiler. The contrast is the
reason this document does not describe hypothetical tensor names or claim
runtime semantics for the twelve unowned rows.

## Concrete completion requirements

When evidence supplies the missing implementation, the change belongs in this
module and must remain narrow:

1. Add an exact `(symbol, source)` ownership table for only the rows whose ABI
   is known.
2. Make `supports` test that table and make `dispatch` match each owned symbol
   to one emitter, returning `FamilyDispatch::Owned(result)`.
3. Define exact input and output names, dtypes, shapes, aliases, prepared
   parameter names and variants, verification facts, index domains, random
   keys, and workspace bytes for each owned row.
4. Emit only Recipe language primitive kinds through the shared `Emitter`,
   preserving the recipe's resolved stage order and one-million-step bound.
5. Keep unsupported or still ambiguous rows in
   `remaining_composition_manifest`; do not make a partial table appear
   concrete.

Until those facts are present, the two no-op Rust cases documented at the top
of this page are the correct implementation. They make the missing boundary
visible and prevent a descriptive inference, quantization, or diffusion row
from becoming an unverified native operation.
