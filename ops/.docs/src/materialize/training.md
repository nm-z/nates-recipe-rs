# Training materialization

`ops/src/materialize/training.rs` is the concrete family owner for six
source-qualified calculation rows. It is the preparation-time bridge from the
descriptive compositions in `ops/src/composition.rs` to a checked
`recipe_language::CalculationGraph`. It does not run a device kernel, own an
optimizer loop, update parameters, or retain mutable runtime state. It only
checks the declared tensor and prepared-parameter ABI and emits the finite
primitive stages that the shared materialization boundary later inserts into a
training or inference program.

The source pair is part of operation identity. A symbol that happens to be
similar, or the same symbol from another legacy source, is not owned by this
module. The exact pair table at `training.rs:11-24` is therefore authoritative
for ownership.

## The six source-qualified rows

The immutable legacy inventory is `operation-surface.txt`. The relevant rows
are line 49, lines 210-213, and line 231. `ops/build.rs` parses that file into
the generated `RAW_OPERATION_SURFACE`; `registry.rs::describe` preserves the
symbol, source, and `OperationId` while selecting each row's lowering and
contracts.

| Symbol and exact source | Registry recipe | Steps and payload | Concrete owner |
| --- | --- | --- | --- |
| `gpu_bce_with_logits`, `gpu-core/src/losses.rs:145` (surface line 49) | `binary_cross_entropy_with_logits` | `MAP_ONLY`, `F32`, `OperationFamily::Loss`; stable per-element loss and gradient | `emit_binary_cross_entropy_with_logits` |
| `gpu_linear_backward_full_into`, `gpu-core/src/kernels.rs:3434` (surface line 210) | `linear_projection_backward` | `LINEAR_BACKWARD_FULL_STEPS`, `F32`, `OperationFamily::Contraction`; input gradient, weight gradient, and bias gradient | `emit_linear_backward_full` |
| `gpu_linear_backward_weights_only_into`, `gpu-core/src/kernels.rs:3400` (surface line 211) | `linear_projection_backward_weights_only` | `LINEAR_BACKWARD_WEIGHTS_ONLY_STEPS`, `F32`, `OperationFamily::Contraction`; weight and bias gradients | `emit_linear_backward_weights_only` |
| `gpu_linear_f32`, `gpu-core/src/nn_f32.rs:200` (surface line 212) | `linear_projection` | `CONTRACT_MAP`, `F32`, `OperationFamily::Contraction`; canonical matrix projection | `emit_linear` |
| `gpu_linear_into`, `gpu-core/src/kernels.rs:2796` (surface line 213) | `linear_projection` | `CONTRACT_MAP`, `F32`, `OperationFamily::Contraction`; canonical matrix projection | `emit_linear` |
| `gpu_matvec_bias_into`, `gpu-core/src/kernels.rs:3084` (surface line 231) | `linear_projection` | `CONTRACT_MAP`, `F32`, `OperationFamily::Contraction`; matrix-vector projection | `emit_matvec_bias` |

`composition_for_entry` assigns the three forward symbols to the same
descriptive recipe and assigns each backward row its own step list
(`composition.rs:1867-1892`). The shared recipe is only the algorithm-shape
contract. The exact tensor names, shapes, scalar SSA, and primitive parameters
come from this module.

The registry currently gives all six rows a canonical f32 payload contract and
fixed primitive-order determinism. The `_into` rows receive the registry's
`AliasContract::NoAlias` metadata, while symbols that do not end in `_into`
retain `OperationSpecific` metadata. The concrete `GraphBuilder::emit` path
still writes an explicit forbidden alias rule for every input/output pair, so a
materialized fragment never relies on implicit input/output aliasing.

## Ownership, registry, and dispatch

The complete production path is:

```text
operation-surface.txt
    -> ops/build.rs -> RAW_OPERATION_SURFACE
    -> OperationRegistry::iter/resolve_unique
    -> OperationDescriptor { lowering: Composition(...) }
    -> recipe::operations::materialize (src/facade.rs)
    -> recipe_ops::materialize_composition (ops/src/materialize.rs)
    -> validate_request
    -> has_concrete_materializer
    -> expand_composition
    -> Emitter::new
    -> dispatch_concrete
    -> training::supports + training::dispatch
    -> one emit_* function below
    -> Emitter::finish -> GraphBuilder::finish -> CalculationGraph::validate
```

`OperationRegistry::resolve_unique` is the symbol-only lookup used by the
production compiler helpers. It succeeds for each of these six currently
unique symbols and returns `UnknownOperation` or `AmbiguousSymbol` when the
canonical inventory cannot provide exactly one descriptor. A direct public
caller can use `resolve_exact(symbol, source)` when it needs the source pair
explicitly.

`materialize_composition` first requires `LoweringAvailability::Composition`,
validates the request, and checks `has_concrete_materializer` before resolving
the iteration input. Each training row is present in this predicate because
`training::supports` compares `(descriptor.symbol, descriptor.source)` against
the six-entry `OPERATIONS` table. A source mismatch returns
`FamilyDispatch::NotOwned`; it cannot fall through to a semantically adjacent
row. The shared dispatcher probes the other family modules first and reaches
the training arm last (`materialize.rs:452-480`). Once this module returns
`FamilyDispatch::Owned(result)`, no other family is consulted.

`training::dispatch` then matches the owned symbol to exactly one emitter. The
`gpu_linear_f32` and `gpu_linear_into` branches intentionally share
`emit_linear`; their source-qualified ownership remains distinct in
`supports`. An owned symbol that is missing from the match is an
`InvalidMaterializationRequest` with the detail that training dispatch is
incomplete. That branch is unreachable for the six declared rows unless the
ownership table and match are edited inconsistently.

## Shared request contract

`MaterializationRequest` carries the descriptor, immutable named input and
output tensor declarations, the name of the iteration-shape input, typed
`PreparedParameters`, a caller-reserved `IdentityNamespace`, and a workspace
limit. Shared validation (`materialize.rs:4268-4331`) requires:

* structured composition lowering, at least one input and output, nonempty
  unique names, and unique `ValueId`s;
* every declared tensor to pass `Tensor::validate`;
* every input to be marked `external_input` and no output to be an input;
  outputs must be marked `external_output`;
* the iteration-shape name to identify an actual input before composition
  expansion.

`require_exact_abi` compares the set of input names, output names, and
prepared-parameter names, rejecting both omissions and additions. It is not a
best-effort lookup. `require_dtype` rejects noncanonical payload dtypes;
`require_shape` reports `UnsupportedConcreteShape`; and the family helpers
report malformed tensor names or contracts as
`InvalidMaterializationRequest`.

`expand_composition` validates the static recipe and records one
`ResolvedStep` per primitive in source order. The six recipes have no repeated
bound, so the forward and BCE rows resolve to two or one steps and the
backward rows resolve to three or two steps. `Emitter::emit_stage` checks that
the emitted primitive family is the family recorded by the corresponding
resolved step and that the concrete emitter emits exactly the resolved step
count. An extra, missing, empty, or wrong-family stage is
`GraphMaterializationFailed`.

The caller reserves half-open value and kernel ranges before entering this
boundary. `GraphBuilder` rejects a declared tensor inside the intermediate
range, rejects exhausted value or kernel ranges, checks workspace-byte
addition, and validates the completed graph. These errors are respectively
`IdentityNamespaceOverlap`, `IdentityNamespaceExhausted`,
`WorkspaceArithmeticOverflow`, `WorkspaceLimitExceeded`, or a
`GraphMaterializationFailed` wrapping a language graph error. No partial graph
is returned on failure.

## Forward projection materializers

### `gpu_linear_f32` and `gpu_linear_into`

Both exact source pairs use `emit_linear` (`training.rs:52-86`). Their ABI is:

| Declaration | Contract |
| --- | --- |
| `input` input | nonempty or empty rank-two f32 `[rows, inner]` matrix; the implementation requires rank two and f32, while ordinary tensor validation supplies layout and byte checks |
| `weight` input | f32 `[inner, columns]`; its leading extent must equal `input`'s `inner` extent |
| `bias` input | f32 `[columns]` |
| `output` output | f32 `[rows, columns]` |
| prepared parameters | exact empty set |
| iteration-shape input | exactly `"input"` |

The graph is the canonical projection

```text
contracted = input @ weight       // Contraction, contract axes (1, 0)
output     = contracted + bias    // Elementwise f32 Add with broadcast bias
```

The contraction has two inputs and one f32 output. The explicit axis pair
`(1, 0)` contracts the input feature axis with the weight row axis; the
language-level contraction validator also checks equal contracted extents and
derives `[rows, columns]` as the output shape. The second stage uses a scalar
program built by `add_program`: two f32 inputs, `ScalarOpcode::Add`, one f32
output. `Emitter::intermediate` allocates exactly one f32 `[rows, columns]`
image for `contracted`, so the fragment's workspace is exactly
`4 * rows * columns` bytes. There are no prepared values, mutable state,
device-side loops, gathers, reductions, or host callbacks.

The explicit checks reject a missing or extra declaration, a non-f32 or
non-matrix input or weight, a weight inner extent that differs from `input`, a
wrong bias or output shape, and an iteration-shape name other than `input`.
An invalid scalar add program or an invalid contraction shape is wrapped as a
graph materialization error. The intermediate and both kernel identities come
from the caller's namespace, not from an implicit global counter.

### `gpu_matvec_bias_into`

`emit_matvec_bias` (`training.rs:88-116`) uses the same two-stage recipe and
the same empty prepared-parameter and `"input"` iteration contracts, but its
ABI is the vector specialization:

```text
input       : f32 [rows, inner]
weight      : f32 [inner]
bias        : f32 [1]
output      : f32 [rows]
```

The contraction still uses axes `(1, 0)`, producing `[rows]`; the scalar Add
then broadcasts the one-element bias. One f32 `[rows]` intermediate is
reserved, for exactly `4 * rows` workspace bytes. No production compiler call
currently names this descriptor. Its supported path is the public operations
materialization boundary, and it remains independently source-qualified from
the matrix projection rows.

## Linear backward materializers

The backward rows use the same dense matrix dimensions. Their common symbols
are `output_gradient` `dY` with f32 shape `[rows, columns]`, `input` `X` with
f32 shape `[rows, inner]`, and, for the full row, `weight` `W` with f32 shape
`[inner, columns]`. Every output is f32, and the row extent of `input` must
equal the row extent of `output_gradient`.

### `gpu_linear_backward_full_into`

`emit_linear_backward_full` requires the exact ABI

```text
inputs:  output_gradient [rows, columns], input [rows, inner], weight [inner, columns]
outputs: input_gradient [rows, inner], weight_gradient [inner, columns], bias_gradient [columns]
params:  tree_lanes = PreparedParameter::U64(...)
shape input: "output_gradient"
```

It emits the three `LINEAR_BACKWARD_FULL_STEPS` in order:

1. `input_gradient = dY * Wᵀ`, a `Contraction` with axis pair `(1, 1)`.
2. `weight_gradient = Xᵀ * dY`, a `Contraction` with axis pair `(0, 0)`.
3. `bias_gradient = sum(dY, axis=0)`, a `Reduce` with
   `ReduceOperator::Sum`, `keep_dimensions=false`, `ReduceResult::Value`, and
   the prepared fixed tree lane count.

No intermediate is allocated by this emitter. Each stage writes one declared
external output, so workspace is zero. The language graph validator verifies
the contraction output shapes and axis extents, while the family checks make
the intended matrix contracts explicit before emission.

`prepared_tree_lanes` accepts only a typed `U64` value that converts to a
power-of-two `u32` in `1..=1024`. A missing value, wrong parameter type, zero,
non-power-of-two, or out-of-range count produces
`MissingPreparedParameter`, `PreparedParameterTypeMismatch`, or
`InvalidMaterializationRequest`. The reduction's fixed tree is part of the
numeric determinism contract: it is not a request to choose a backend-specific
sum order at execution time.

### `gpu_linear_backward_weights_only_into`

`emit_linear_backward_weights_only` requires:

```text
inputs:  output_gradient [rows, columns], input [rows, inner]
outputs: weight_gradient [inner, columns], bias_gradient [columns]
params:  tree_lanes = PreparedParameter::U64(...)
shape input: "output_gradient"
```

It emits only the second and third stages above, the `(0, 0)` contraction for
`weight_gradient` followed by the axis-zero fixed-tree sum for
`bias_gradient`. The missing `weight` input is deliberate because the input
gradient is not requested. Workspace remains zero.

The source row is not an alternate implementation of the full backward case.
It has a distinct composition recipe, exact input-name set, and exact
resolved-stage count. Extra `weight` input declarations are rejected rather
than ignored.

## Binary cross entropy with logits

`emit_binary_cross_entropy_with_logits` (`training.rs:237-263`) requires the
exact ABI:

```text
inputs:  logits, targets
outputs: losses, gradients
params:  empty
shape input: "logits"
```

`logits` must be f32. `targets`, `losses`, and `gradients` must each be f32
with exactly the logits shape. The operation has no rank-specific matrix
requirement and no prepared dimension parameter; ordinary tensor validation
still enforces a valid non-rank-zero shape and storage contract.

The one `MAP_ONLY` stage is an `Elementwise` primitive with two outputs. The
owned scalar program in `ops/src/scalar.rs:477-502` validates every target in
the calculation itself:

```text
finite(target) && target >= 0 && target <= 1
loss     = max(logit, 0) - logit * target + log1p(exp(-abs(logit)))
gradient = sigmoid(logit) - target
```

The `Require` instructions make a nonfinite or out-of-range target a device
calculation fault through the normal fault channel. `softplus` and `sigmoid`
are Recipe-owned inlined math programs, so no host callback or vendor loss
routine is selected. The stage emits directly to both declared outputs and
reserves no workspace.

The materializer therefore rejects dtype, shape, name, and iteration-input
violations before scalar construction. Scalar-builder or graph-validation
failures become `GraphMaterializationFailed`. There is no silent clipping,
target conversion, alternate BCE formula, or fallback to the focal or other
loss programs.

## Production compiler callers and state flow

The private `GraphCompiler::materialize` helper in
`training/src/compile.rs:10937-10997` is the training caller shared by all
materialized symbols. It clones the existing compiler tensors, marks cloned
inputs as external inputs and cloned outputs as external outputs, creates
`NamedTensor` declarations, and resolves the symbol through
`operation_registry().resolve_unique`. It reserves 64 value identities and 64
kernel identities (`MATERIALIZATION_RESERVATION`) before constructing the
request. On success it inserts every materialized tensor contract and kernel
node into the compiler graph and associates every inserted kernel with the
caller-provided `IterationDomain`. The training compiler's workspace limit is
`u64::MAX`; the per-fragment intermediate identity reservation is still finite
and checked.

The exact training call sites are:

* `compile_dense_training_impl` (`compile.rs:1122`) uses BCE for the binary
  objective after masking invalid rows. It passes the full training domain and
  the empty parameter map.
* `GraphCompiler::compile_training_layer` (`compile.rs:5842`) uses
  `gpu_linear_into` for each dense forward layer, after optional routing-mask
  application and before activation or normalization operations.
* `GraphCompiler::compile_validation_layer` (`compile.rs:7601`) uses the same
  forward row with updated checkpoint parameter state and the validation
  iteration domain.
* `GraphCompiler::compile_validation` (`compile.rs:7703`) uses BCE to produce
  validation losses and an intentionally unused gradient buffer for binary
  validation metrics.
* `GraphCompiler::compile_temperature_scaling` (`compile.rs:8330`) uses BCE on
  temperature-scaled logits. It consumes the emitted gradient in the later
  temperature-gradient reduction; the BCE loss output is intentionally unused
  in that optimization subgraph.
* `GraphCompiler::backward_layer` (`compile.rs:9991` and `10009`) selects the
  full row when the surrounding model needs an input gradient, and the
  weights-only row otherwise. In the reverse block walk,
  `need_input_gradient` is `block_index != 0` (`compile.rs:8393`), so the
  leading model block can stop at parameter gradients while preceding blocks
  receive a propagated input gradient. Residual branches explicitly request
  the full row.

No production compiler currently calls `gpu_linear_f32` or
`gpu_matvec_bias_into`; their public materialization support is real, but no
standard dense training or inference path selects those descriptors today.
The absence of a caller is not permission for either symbol to fall through to
`gpu_linear_into` or to acquire a host wrapper.

`InferenceGraphCompiler::materialize` in
`training/src/inference.rs:2008-2077` follows the same request and insertion
path with a 64-value and 64-kernel reservation and the same unbounded
workspace limit. Its `compile_layer` caller (`inference.rs:3438`) uses
`gpu_linear_into` for each checkpoint-backed dense layer, marks checkpoint
weights and bias as immutable external inputs, and assigns the emitted nodes
to `IterationDomain::first()`. Inference materialization errors are converted
from `OperationError` to `InferenceCompileErrorKind::Operation`.

Training materialization errors are converted by
`training/src/error.rs:58-60` to `TrainingCompileErrorKind::Operation`. The
caller may therefore report an operation error without changing its kind or
pretending that a graph was compiled. Tensor-contract conflicts when a
materialized intermediate is inserted are separate language errors from the
compiler's `insert_tensor_contract` checks.

After all forward, loss, backward, and optimizer nodes have been assembled,
`GraphCompiler::finish` (`compile.rs:11078-11123`) marks authoritative external
boundaries, validates the complete `CalculationGraph`, serializes and parses
the canonical OGDL form, and creates the `StaticCalculationProgram` with loop
iterations, kernel domains, and metrics. Materialized stages are therefore
static graph nodes, not runtime callbacks or per-step host dispatch.

The public training path in `src/training.rs` calls this compiler from
`compile_training_graph`, then `execute_current_training` prepares the static
program against the measured profile. `prepare_and_execute_local_training_controlled`
(`training/src/execute.rs:2178`) validates and plans the graph, resolves native
artifacts, creates device images, and starts the production loop. Contraction,
elementwise, and reduction nodes emitted here are lowered and scheduled by the
normal planner and native executor. The materializer's role ends when its
validated graph fragment is inserted; it does not choose hardware, allocate a
device buffer, or perform execution.

## State, identity, workspace, and error boundaries

The operation family has no mutable state object. Dense weights, bias values,
optimizer moments, resume state, loop counters, and validation state belong to
the surrounding training compiler and its later optimizer or lifecycle
stages. The materializer sees only immutable tensor declarations and typed
preparation facts. In particular:

* forward matrix projection reserves one f32 matrix image;
* forward matrix-vector projection reserves one f32 vector image;
* BCE, full backward, and weights-only backward reserve no intermediate
  image;
* all emitted kernels have explicit forbidden input/output aliases;
* caller-reserved value and kernel namespaces are preserved in
  `MaterializedComposition::identity_namespace` and cannot overlap declared
  input/output IDs;
* graph validation must succeed before `MaterializedComposition` is returned.

The observable error boundaries are deliberate:

| Condition | Error boundary |
| --- | --- |
| Descriptor is not a composition, missing declarations, duplicate names or IDs, wrong external flags, wrong iteration input, dtype mismatch, or unexpected prepared names | `InvalidMaterializationRequest` or `WrongLoweringKind` |
| Exact shape does not match a concrete ABI, including matrix rank, inner extent, bias extent, or gradient output shape | `UnsupportedConcreteShape` (or the request error where the check is a tensor contract) |
| `tree_lanes` is absent, mistyped, zero, non-power-of-two, or outside `1..=1024` | `MissingPreparedParameter`, `PreparedParameterTypeMismatch`, or `InvalidMaterializationRequest` |
| Tensor construction, scalar construction, contraction/reduction validation, or final graph validation fails | `GraphMaterializationFailed` wrapping the language error |
| A reserved intermediate or kernel range is too small or overlaps a declared tensor | `IdentityNamespaceExhausted` or `IdentityNamespaceOverlap` |
| Workspace byte arithmetic exceeds the caller limit | `WorkspaceArithmeticOverflow` or `WorkspaceLimitExceeded` |
| A source pair has a descriptive recipe but no exact owner | `MissingConcreteFormula`; it remains in `remaining_composition_manifest` |

There is no retry, source substitution, host loop, CPU implementation,
unchecked index behavior, or placeholder kernel. A malformed or unsupported
request remains a visible preparation failure.

## End-to-end completion boundary

For the six rows, the end-to-end contract is complete only when all of these
boundaries hold in sequence:

1. The caller resolves the canonical source-qualified descriptor and supplies
   real tensors with the exact ABI above.
2. `materialize_composition` validates the request, resolves the descriptive
   composition, emits the exact primitive stages, accounts for workspace and
   identities, and validates the fragment graph.
3. The training or inference compiler inserts that graph into its one static
   program, preserves the caller's iteration domain, validates and round-trips
   OGDL, and exposes the complete graph to preparation.
4. The production preparer and planner lower the graph for the measured CUDA
   or HSA system, realize native kernels, and execute the complete loop through
   the native executor.

Compilation of a descriptor, a success status, a `StageEmission`, or an
emitted kernel ID is not runtime correctness evidence. A runtime claim must
measure the resulting user-level loss, prediction, gradient, or parameter state
through the public training or inference entry point on the required real
dataset and hardware. Conversely, no hardware run can make an invalid ABI or
missing source pair valid, because those failures occur at the preparation
boundary first.

## Evidence and validation

The authoritative implementation evidence is:

* `ops/src/materialize/training.rs`, the exact pair table, dispatch branches,
  ABI checks, primitive emissions, and scalar builder;
* `ops/src/materialize.rs`, shared request validation, concrete-owner gate,
  family dispatch, finite expansion, identity and workspace accounting, and
  graph validation;
* `ops/src/composition.rs`, recipe names, definitions, payload contracts, and
  resolved stage lists;
* `ops/src/registry.rs`, canonical descriptor construction and symbol/source
  resolution;
* `training/src/compile.rs` and `training/src/inference.rs`, production
  callers, graph insertion, domains, and final static programs;
* `src/training.rs`, `training/src/execute.rs`, and `prepare/src/lib.rs`, the
  public compile, measured preparation, native realization, and execution
  boundary.

Focused structural checks for this documentation and its source boundary are:

```text
cargo check -p recipe-ops
cargo check -p recipe-training
git diff --check
```

These checks establish that the documented registry, materializer, and
production callers compile together. They do not substitute for a real
hardware acceptance run, and no such run is implied by this documentation
change.
