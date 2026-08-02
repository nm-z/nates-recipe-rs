# Creation, shape, and miscellaneous materialization

`ops/src/materialize/creation_shape_misc.rs` is a declared family seam, not a
concrete graph builder. The file currently owns no source-qualified operation.
Its two functions are intentional fail-closed stubs:

```rust
pub(super) fn supports(_descriptor: OperationDescriptor) -> bool { false }

pub(super) fn dispatch(
	_request: &MaterializationRequest<'_>,
	_emitter: &mut Emitter<'_>,
) -> FamilyDispatch {
	FamilyDispatch::NotOwned
}
```

The underscore-prefixed arguments are not inspected. There is no operation
table, source pair, tensor ABI, prepared-parameter ABI, shape policy, scalar
program, primitive parameter set, workspace calculation, intermediate
allocation, kernel emission, or graph validation in this module. `supports`
therefore cannot claim ownership accidentally, and `dispatch` cannot emit a
placeholder graph or select a semantically adjacent operation.

## The two functions and their callers

`materialize.rs` declares this private module beside the other family modules.
The shared dispatcher probes families in this order:

1. `optimizer_normalization`
2. `solver_fft`
3. `attention_sequence_embedding`
4. `convolution_pooling`
5. `loss_metrics`
6. `indexing_sort_encoding`
7. `graph_cluster_rl`
8. `tree_boosting`
9. `inference_quantization_diffusion`
10. `creation_shape_misc`
11. `training`

For each family, the `dispatch_family!` macro calls `dispatch(request,
emitter)`. `FamilyDispatch::Owned(result)` returns the operation result
immediately; `FamilyDispatch::NotOwned` lets the probe continue. This module
always returns `NotOwned`, so the dispatcher reaches `training` and then its
final `GraphMaterializationFailed` branch when the request has not been
rejected earlier as incomplete.

The same `supports` function is part of `has_concrete_materializer` in
`materialize.rs`. That predicate is an OR over all ten family modules. Because
this arm is always false, this family contributes no concrete operation to the
predicate and no request can pass the concrete-materializer gate through this
module.

The public boundary is `recipe::operations::materialize` in `src/facade.rs`,
which delegates to `recipe_ops::materialize_composition`. The production
training and inference compilers use their private `Compiler::materialize`
helpers, in `training/src/compile.rs` and `training/src/inference.rs`, which
also construct a `MaterializationRequest` and call the same
`materialize_composition` function. Their current calls name concrete symbols
owned by other family modules. A repository search finds no production caller
requesting any of the four creation descriptors below.

## Registry rows that describe creation operations

`composition.rs` still provides descriptive recipes for four source rows with
`OperationFamily::Creation`. The registry reaches those recipes after the
scalar, primitive, workspace, and non-calculation classification checks. The
recipe describes an algorithm shape, but it is not executable tensor wiring.

| Symbol and exact legacy source | Registry recipe | Composition steps and payload | Current materializer result |
| --- | --- | --- | --- |
| `gpu_eye`, `gpu-core/src/kernels.rs:5584` (operation-surface line 132) | `identity_matrix` | `MAP_ONLY`, `F32AndI32`; map static row and column indexes to f32 one on the diagonal and positive zero elsewhere | No concrete owner. `creation_shape_misc::supports` is false, so the row is in `remaining_composition_manifest`. |
| `gpu_fill_sentinel`, `gpu-core/src/reductions.rs:336` (operation-surface line 137) | `sort_padding_fill` | `MAP_ONLY`, `F32AndI32`; write the prepared IEEE-total-order sentinel only into the statically padded tail | No concrete owner. The descriptive recipe is visible, but no sentinel ABI or emitter is present here. |
| `gpu_init_idx`, `gpu-core/src/reductions.rs:357` (operation-surface line 184) | `int32_iota` | `MAP_ONLY`, `I32`; convert each statically generated linear lane index to checked canonical int32 | No concrete owner. No index-map or elementwise graph is emitted by this family. |
| `gpu_iota`, `gpu-core/src/catboost.rs:41` (operation-surface line 185) | `int32_iota` | `MAP_ONLY`, `I32`; the same descriptive recipe as `gpu_init_idx`, but a distinct source-qualified descriptor | No concrete owner. Source identity is retained; the symbol is not allowed to fall through to `gpu_init_idx`. |

The source pair is part of the identity. `OperationRegistry::resolve_exact`
requires both symbol and source, and `OperationDescriptor` retains the
`OperationId`, symbol, source, family, payload contract, alias contract, and
determinism contract. The composition match for these symbols is concise, but
it does not create a source-agnostic materializer. A future `OPERATIONS` table
in this module must list each exact `(symbol, source)` pair, including both
distinct `int32_iota` rows.

`gpu_fill` and `gpu_fill_f32` are a useful boundary distinction. Registry
classification sends those symbols to the scalar identity recipe in
`scalar.rs`, despite their `OperationFamily::Creation` classification. They
are not composition descriptors and must not be added to this module. Likewise,
shape or encoding symbols such as `gpu_one_hot`, `gpu_concat_into`,
`gpu_transpose`, and `gpu_slice_rows` are composition entries, but their exact
pairs are owned by `indexing_sort_encoding.rs`; `gpu_repeat_rows` is owned by
`attention_sequence_embedding.rs`. A family name or operation noun is not
enough to transfer ownership.

## Materialization path for an unowned creation row

For a public request resolved to one of the four descriptors, the actual path
is:

```text
operation_registry().resolve_exact(symbol, source)
    -> OperationDescriptor { lowering: Composition(...) }
    -> recipe::operations::materialize(request)
    -> materialize_composition
    -> validate_request
    -> has_concrete_materializer == false
    -> MissingConcreteFormula
```

`validate_request` still performs the shared boundary checks before the family
gate. It requires a composition lowering, at least one input and output,
nonempty unique tensor names, unique tensor IDs, valid tensor contracts,
external-input flags on inputs, and non-input external-output flags on outputs.
Thus malformed declarations can fail as `WrongLoweringKind`,
`InvalidMaterializationRequest`, or `GraphMaterializationFailed` from tensor
validation before the absence of a concrete family is observed. These checks do
not imply that a creation ABI exists.

After shared validation, `materialize_composition` tests
`has_concrete_materializer`. For each creation descriptor this is false, so the
function returns `OperationErrorKind::MissingConcreteFormula` with the detail
that the source-qualified row remains in `remaining_composition_manifest` and
that tensor ABI, scalar SSA, primitive parameters, and workspace policy are not
concrete. It returns before it looks up `iteration_shape_input`, calls
`expand_composition`, constructs `Emitter`, allocates an `IdentityNamespace`,
or calls `dispatch_concrete`. No shape-dependent repeat is resolved and no
`CalculationGraph`, `StageEmission`, or `WorkspaceAllocation` is produced.

If the private dispatcher were reached directly, this module would still return
`FamilyDispatch::NotOwned`; after every family returns `NotOwned`, the shared
fallback is `GraphMaterializationFailed` with a dispatch message naming the
operation symbol and operation ID. The public path normally sees
`MissingConcreteFormula` first because of the explicit gate. There is no retry,
alternate implementation, CPU path, placeholder kernel, or recovery branch.

## Remaining-manifest state

`remaining_composition_manifest()` iterates the canonical registry and keeps
only descriptors whose lowering is `LoweringAvailability::Composition` and
whose `has_concrete_materializer` result is false. For each retained row it
records the operation ID, symbol, source, descriptive recipe name, and the
complete missing-component set:

| Missing component | Meaning at this boundary |
| --- | --- |
| `TensorAbi` | No exact named input/output tensor contract is implemented. |
| `ScalarFormula` | No owned scalar SSA program is wired to the declared operation. |
| `PrimitiveParameters` | No checked primitive kinds, axes, bounds, or conflict policy are emitted. |
| `WorkspacePolicy` | No exact intermediate reservation and workspace-byte policy is implemented. |

The four creation rows therefore remain observable, source-qualified entries,
not silently dropped operations. The public facade exposes this state through
`recipe::operations::remaining_compositions()`. A caller can inspect the
manifest without attempting graph emission.

## Shapes, state, and invariants that are not yet accepted

The descriptive recipes name the intended shape/state facts, but the current
module checks none of them:

* `gpu_eye` would need a statically declared row/column coordinate ABI, a
  checked output shape, and an exact int32 index domain. The diagonal one and
  positive-zero off-diagonal rule is only prose in `composition.rs` today.
* `gpu_fill_sentinel` would need a verified padded-tail index table, the exact
  sentinel representation, and a no-write-outside-tail invariant. No prepared
  fact or output alias policy is read here.
* `gpu_init_idx` and `gpu_iota` would need a nonempty statically bounded output
  shape, a checked conversion from every lane coordinate to int32, and an
  explicit distinction between each source's input/output ABI. No index map,
  shape extent, or int32 overflow check is performed by this module.

These are requirements for a future concrete implementation, not accepted
behavior. Until the exact facts are represented as a request ABI and emitted
through the shared `Emitter`, a shape or parameter supplied by a caller cannot
make one of these rows materializable.

## Identity, graph, and workspace consequences

Because no creation dispatch is owned:

* `Emitter::intermediate` is never called, so no `WorkspaceObject` or
  `WorkspaceAllocation` exists for these rows.
* `Emitter::emit` and `Emitter::emit_stage` are never called, so no
  `PrimitiveKernel`, kernel identity, alias rule, or `StageEmission` exists.
* `GraphBuilder::finish` and `CalculationGraph::validate` are never reached
  for the normal unowned path.
* The caller's `IdentityNamespace` is not consumed by a partial fragment. No
  value or kernel reservation is returned, and no fragment can be assembled
  accidentally.

This preserves the materialization contract: descriptive composition metadata
cannot be mistaken for a complete graph, and missing semantics remain visible
at the preparation boundary.

## Evidence and validation

The source evidence for this module is the exact eight-line implementation in
`ops/src/materialize/creation_shape_misc.rs`, the module probe and concrete
materializer predicate in `ops/src/materialize.rs`, the four creation recipes in
`ops/src/composition.rs`, and operation-surface lines 132, 137, 184, and 185.
The public caller chain is `src/facade.rs` to `recipe_ops`, with compiler
callers in `training/src/inference.rs` and `training/src/compile.rs` using the
same production entry point.

Structural validation uses the real package boundary:

```text
cargo check -p recipe-ops
cargo check -p recipe-training
```

These commands verify that the registry, materializer module, shared dispatcher,
and production consumers compile together. They do not claim runtime graph
success for the four unowned rows. No production training or inference
declaration currently requests these creation symbols, so there is no valid
hardware end-to-end graph to report for them. The observable end state is the
fail-closed `MissingConcreteFormula` result or the source-qualified entry in
`remaining_compositions()`. A future acceptance run must first supply a real
ABI and invoke the public materialization path before any CUDA or HSA execution
claim is possible.

## Concrete-family completion boundary

Adding ownership here requires one coherent change across the existing
boundary, not a wrapper around another family:

1. Add exact source pairs to a local `OPERATIONS` table and make `supports`
   source-qualified.
2. Add one symbol branch in `dispatch` that returns `FamilyDispatch::Owned` and
   emits exactly the primitive stages described by the selected
   `CompositionRecipe`.
3. Define the named tensor and prepared-parameter ABI, shape and int32 bounds,
   verification facts, scalar SSA program, primitive parameters, conflict and
   alias policy, intermediate identities, and workspace accounting.
4. Exercise the public `recipe::operations::materialize` path, then run the
   resulting graph through the production compiler and measured backend. Until
   all of those facts are present and independently observed, leaving both
   stubs unchanged is the correct behavior.
