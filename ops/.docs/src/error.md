# Operation errors

`ops/src/error.rs` is the typed failure boundary for the `recipe-ops` crate.
It does not validate a graph, lower a kernel, allocate workspace, execute a
device operation, or recover from a rejected request.  It supplies the one
machine-readable category enum, the diagnostic carrying that category, and the
`Result` alias used by the registry, scalar and primitive lowerers, composition
expander, concrete materializers, workspace formulas, and Recipe-owned
operation preparations.

The module is deliberately small because the operation implementations own
the checks.  Each implementation constructs an `OperationError` at the point
where it can state the violated contract, propagates it with `?`, and may add
the source-qualified operation identity once a descriptor is available.  A
returned error means that the requested operation did not produce the value,
program, workspace reservation, or graph requested by the caller.  There is no
retry, fallback operation, placeholder graph, or silently substituted shape.

## Public surface

The crate facade re-exports the three names from `ops/src/lib.rs`:

| Name | Role |
| --- | --- |
| `OperationErrorKind` | The fixed current set of 21 operation failure categories. |
| `OperationError` | One category, one concrete diagnostic string, and an optional `OperationId`. |
| `OperationResult<T>` | `Result<T, OperationError>`, used for every fallible operation-owned boundary. |

The root facade re-exports the same types from `src/facade.rs` under
`recipe::operations`.  The root methods `resolve`, `resolve_exact`,
`lower_scalar`, `lower_primitive`, `validate_composition`, `materialize`, and
`evaluate_workspace` return this alias directly.  Training and inference
compilers use the lower-level crate facade as well, so these errors are part of
the compile-time operation contract even when a higher-level error later
renders them as text.

## Type structure

```text
OperationErrorKind (Copy, Clone, Debug, PartialEq, Eq)
|-- UnknownOperation
|-- AmbiguousSymbol
|-- UnsupportedLowering
|-- WrongLoweringKind
|-- PrimitiveRecipeMismatch
|-- InvalidScalarProgram
|-- PrimitiveLoweringFailed
|-- InvalidCompositionRecipe
|-- InvalidMaterializationRequest
|-- MissingPreparedParameter
|-- PreparedParameterTypeMismatch
|-- IterationBoundUnresolved
|-- CompositionExpansionOverflow
|-- MissingConcreteFormula
|-- UnsupportedConcreteShape
|-- IdentityNamespaceOverlap
|-- IdentityNamespaceExhausted
|-- WorkspaceLimitExceeded
|-- GraphMaterializationFailed
|-- WorkspaceFormulaMismatch
`-- WorkspaceArithmeticOverflow

OperationError
|-- kind: OperationErrorKind
|-- detail: String
`-- operation: Option<OperationId>

OperationResult<T> = Result<T, OperationError>
```

`OperationErrorKind` is an ordinary exhaustive enum in this release.  It has
no payload because the detail string carries the observed values and contract
wording.  `OperationError` is `Clone`, `Debug`, `PartialEq`, and `Eq`; cloning
copies the owned diagnostic.  The fields are public so an advanced caller can
classify the category, inspect the exact detail, and inspect the optional
operation identity without parsing `Display` output.

### Construction

`OperationError::new(kind, detail)` accepts any `Into<String>` and always
starts with `operation: None`.  The constructor does not normalize, prefix, or
deduplicate the supplied detail.  The caller therefore controls whether the
message contains a shape, tensor identity, source symbol, or lower-level error
text.

`OperationError::for_operation(operation)` is a `const` consuming builder.  It
sets the optional `OperationId` and returns the same error, preserving both
the category and detail.  It is used after descriptor dispatch has selected a
source-qualified operation.  It is not a lookup or validation operation and it
does not alter the operation registry.

### Display and error traits

`Display` writes the debug spelling of the category, a colon, and the detail:

```text
{kind:?}: {detail}
```

When `operation` is present it appends one context suffix, using only the
descriptor ordinal:

```text
{kind:?}: {detail} [operation {operation.ordinal()}]
```

The suffix intentionally does not print the symbol or source.  Those fields
remain available through `OperationId` and `OperationDescriptor`, while the
short ordinal is stable in the canonical registry order.  `Display` forwards
the formatter result and performs no truncation or allocation beyond the
already-owned `detail` string.  `OperationError` implements
`std::error::Error` with the empty default implementation.  It has no nested
source error, so callers that need a lower-level diagnostic receive its
rendered text in `detail`.

## Operation identity context

There are two intentional classes of error context.

### Descriptor-qualified errors

The descriptor-facing APIs add `OperationId` before returning:

* `OperationRegistry::resolve_unique` and `resolve_exact` cannot add one,
  because a missing or ambiguous entry has no unique identity.
* `lower_scalar` adds it for an unsupported lowering, an invalid generated
  scalar program, and failures returned by a composite recipe.
* `lower_primitive` adds it for wrong lowering kind, unsupported lowering, a
  primitive recipe mismatch, or a backend-neutral primitive lowering failure.
* `validate_composition` adds it to `InvalidCompositionRecipe` and
  `WrongLoweringKind`.
* `expand_composition` adds it to recipe validation and all concrete expansion
  failures.
* `materialize_composition` and its `Emitter`/`GraphBuilder` helpers construct
  errors with the descriptor operation already attached.
* `evaluate_workspace` adds it to formula dimension, arithmetic, and formula
  evaluation errors.

The common helper is `operation_error` in `ops/src/materialize.rs`; it calls
`OperationError::new(...).for_operation(operation)`.  The lowerers use the
same pattern directly or through `invalid_program` and `wrong_kind`.

### Unqualified low-level errors

An error remains `operation: None` when it is produced before a descriptor is
available or by a reusable operation fragment that has no source-qualified
identity of its own.  This is intentional rather than missing context:

* Registry resolution errors describe the absent or multiply present lookup
  key itself.
* `CompositionRecipe::validate` returns an unqualified recipe error;
  `validate_composition` or `expand_composition` adds the descriptor later.
* `lower_recipe` and `lower_index_map` return unqualified recipe or
  primitive errors; `lower_primitive` adds the descriptor for registry-backed
  calls.
* `Composer` errors and the public `canonical_*_program` helpers in
  `ops/src/scalar.rs` have no descriptor and therefore remain unqualified.
  `lower_scalar` wraps them with its descriptor.
* `WorkspaceFormula::evaluate` and its checked arithmetic helpers are
  unqualified; `evaluate_workspace` adds the descriptor.
* The standalone convolution and pooling preparations have no registry
  descriptor and report their shape or arithmetic errors unqualified.
* The dedicated Bayes, binary-metrics, K-means, KNN, and tree graph builders
  use local `*_error` helpers without an `OperationId`, because their request
  types are already the concrete public boundary rather than a registry
  descriptor.
* `validate_identity_namespaces` validates a caller's group of reservations,
  not one operation, so its overlap and range errors are unqualified.

The distinction lets a caller tell whether an error is a named registry
operation failure or a reusable preparation failure.  No code guesses an
operation from a detail string.

## End-to-end role and propagation

The normal public path is:

```text
recipe::operations::resolve / resolve_exact
        |
        v
OperationRegistry -> OperationDescriptor
        |
        +--> lower_scalar ------------> ScalarProgram
        +--> lower_primitive ---------> LoweredProgram
        +--> validate_composition ----> checked finite recipe
        +--> materialize -------------> MaterializedComposition
        `--> evaluate_workspace ------> WorkspaceValue
```

The facade methods in `src/facade.rs` are thin forwarding functions.  They do
not catch, remap, retry, or replace an `OperationError`.  Training compilation
uses `operation_registry().resolve_unique(symbol)?` followed by
`lower_scalar(...)` or `materialize_composition(...)` in
`training/src/compile.rs`.  Inference compilation follows the same sequence
in `training/src/inference.rs`, including the concrete Bayes, KNN, and other
operation materializers.  A failure stops graph construction at that boundary;
no partially emitted stage is returned as a successful compile.

At the higher-level compile boundary, `TrainingCompileError::from(OperationError)`
and `InferenceCompileError::from(OperationError)` store
`error.to_string()` under their `Operation` category.  The conversion drops
the structured `OperationErrorKind` and `operation` field from the outer type,
but the rendered text retains both the category spelling and, when present,
the ordinal suffix.  The original `OperationError` is therefore the source of
truth until that explicit compile-error conversion, and its exact diagnostic
is not replaced by a generic status.

`training/src/forward.rs::lower_activation` also accepts `OperationError` via a
generic `From` bound because canonical activation programs are reusable
low-level builders.  Those canonical helpers can return an unqualified error;
the training compile wrapper gives it the same `Operation` classification and
rendered detail.

All operation errors occur before native execution.  The operation layer owns
semantic lowering, graph materialization, checked dimensions, identity
reservations, and workspace accounting.  Native executors receive only a
validated graph or plan, so an `OperationError` is a rejected compile or
preparation transition, not a device fault or runtime lifecycle status.

## Failure construction index

The following sections enumerate the actual construction and propagation
sites.  The category is the stable classification; the detail is the concrete
contract and observed value supplied by the rejecting branch.

### Registry resolution

#### `UnknownOperation`

`OperationRegistry::resolve_unique` constructs this category when
`named(symbol)` yields no descriptor.  Its detail is
`operation symbol {symbol:?} is absent`.  `resolve_exact` uses the same
category when the `(symbol, source)` pair is absent and reports
`operation ({symbol:?}, {source:?}) is absent from the canonical registry`.
These failures are necessarily unqualified because no descriptor can supply
an `OperationId`.  The root `recipe::operations::resolve` and
`resolve_exact` methods return them unchanged.

Source: [`registry.rs:resolve_unique`](../../src/registry.rs#L288),
[`registry.rs:resolve_exact`](../../src/registry.rs#L306).

#### `AmbiguousSymbol`

`resolve_unique` returns this category when a symbol has more than one
source-qualified descriptor.  It recounts the matching descriptors and
reports `operation symbol {symbol:?} has {count} source-qualified entries`.
`resolve_exact` also checks for an impossible duplicate exact pair and reports
`operation ({symbol:?}, {source:?}) occurs more than once`.  The registry
does not select the first match, collapse legacy occurrences, or infer a
source.  Both errors are unqualified and return through the facade unchanged.

Source: [`registry.rs:resolve_unique`](../../src/registry.rs#L288),
[`registry.rs:resolve_exact`](../../src/registry.rs#L306).

### Lowering ownership and kind checks

#### `UnsupportedLowering`

`lower_scalar` and `lower_primitive` emit this category only when the selected
descriptor has `LoweringAvailability::Unsupported(reason)`.  The detail
combines the descriptor definition and the explicit `UnsupportedReason` debug
spelling.  The result is then qualified with the descriptor ID.  Unsupported
means the registry entry is known but has no owned canonical implementation;
it is not an invitation to use a legacy backend or a different operation.

Source: [`scalar.rs:lower_scalar`](../../src/scalar.rs#L347),
[`primitive.rs:lower_primitive`](../../src/primitive.rs#L390).

#### `WrongLoweringKind`

The descriptor is known, but the caller selected an API whose expected
lowering representation does not match the descriptor:

| API | Accepted representation | Failure detail |
| --- | --- | --- |
| `lower_scalar` | `LoweringAvailability::Scalar` | `operation does not have a scalar elementwise recipe` |
| `lower_primitive` | `LoweringAvailability::Primitive` | `operation does not have a direct primitive recipe` |
| `validate_composition` and `expand_composition` | `LoweringAvailability::Composition` | `operation does not own a multi-stage composition` or the debug lowering value is reported by expansion |
| `evaluate_workspace` | `LoweringAvailability::Workspace` | `operation does not own a static workspace formula` |
| `materialize_composition` admission | `LoweringAvailability::Composition` | `materialization requires a structured operation descriptor` |

Every descriptor-facing branch qualifies the error with `OperationId`.  The
check is a direct enum match, so an unsupported or non-calculation entry is
rejected rather than coerced into another lowering path.

Sources: [`composition.rs:validate_composition`](../../src/composition.rs#L127),
[`primitive.rs:lower_primitive`](../../src/primitive.rs#L390),
[`scalar.rs:lower_scalar`](../../src/scalar.rs#L347),
[`workspace.rs:evaluate_workspace`](../../src/workspace.rs#L270),
[`materialize.rs:validate_request`](../../src/materialize.rs#L4268).

### Primitive and scalar recipe validity

#### `PrimitiveRecipeMismatch`

`lower_recipe` calls `PrimitiveRecipe::matches` against the supplied
`PrimitiveKernel` and tensor map.  A false result means the operation's
required family, operator, axis relationship, dtype contract, bounds policy,
alias rule, or random distribution is not the kernel actually supplied.  The
detail is `kernel kind does not satisfy {recipe.definition()}`.  This check
happens before calling `recipe_primitives::lower`, and it never rewrites the
kernel to fit.  `lower_primitive` adds the descriptor ID; direct
`lower_index_map` leaves the error unqualified.

Source: [`primitive.rs:lower_recipe`](../../src/primitive.rs#L429).

#### `PrimitiveLoweringFailed`

When the recipe matches, `lower_recipe` delegates to
`recipe_primitives::lower`.  Any backend-neutral primitive compiler failure is
converted to this category with the lowerer's `to_string()` as detail.  The
underlying error is not retained as a `source()` chain.  Registry-backed
`lower_primitive` qualifies the result, while direct `lower_index_map` keeps
it unqualified.

Source: [`primitive.rs:lower_recipe`](../../src/primitive.rs#L429).

#### `InvalidScalarProgram`

This category is the scalar SSA construction and validation boundary.  The
`Composer` uses it for:

* an opcode whose operand dtypes have no result dtype;
* an inline math function that cannot be converted to a scalar program;
* an inline math arity mismatch or argument dtype mismatch;
* an instruction referring to an unknown scalar value;
* an instruction whose reconstructed result dtype differs from its declared
  dtype;
* a scalar program with no output or an unknown output;
* `ScalarProgram::validate()` failures in `Composer::finish`; and
* exhaustion of the local scalar-value identity space.

`lower_scalar` also wraps conversion and final validation failures in this
category through `invalid_program`, then attaches the descriptor identity.
The canonical parameterless activation helpers call `Composer` directly and
therefore return the same category without an operation ID.  A scalar
program is never returned after a failed validation pass.

Sources: [`scalar.rs:lower_scalar`](../../src/scalar.rs#L347),
[`scalar.rs:Composer`](../../src/scalar.rs#L803).

### Composition recipe validity and expansion

#### `InvalidCompositionRecipe`

`CompositionRecipe::validate` emits this unqualified category for structural
recipe defects:

* an empty name, definition, or top-level step list;
* nesting deeper than eight levels;
* an empty step role;
* an empty repeated body;
* a fixed repeat bound of zero; or
* a prepared-parameter repeat bound with an empty parameter name.

`validate_composition` and `expand_composition` attach the descriptor ID after
this validation.  The validator does not attempt to infer a repeat count or
drop a malformed step.

Source: [`composition.rs:CompositionRecipe::validate`](../../src/composition.rs#L116),
[`composition.rs:validate_steps`](../../src/composition.rs#L143).

#### `IterationBoundUnresolved`

`Expansion::resolve_bound` emits this category when a
`MinimumShapeExtent` is requested for an empty shape or when a
`ShapeExtent`/`CeilingLog2ShapeExtent` axis is outside the supplied shape rank.
The concrete shape is the only authority for shape bounds; no default extent
is invented.  A `PreparedParameter` bound delegates to `prepared_u64`, so a
missing or wrongly typed parameter reports the more specific prepared-
parameter category instead.  The expansion error is qualified with the
operation ID.

Source: [`materialize.rs:Expansion::resolve_bound`](../../src/materialize.rs#L577).

#### `CompositionExpansionOverflow`

`Expansion::push_primitive` rejects the transition that would exceed the fixed
`MAX_EXPANDED_STEPS` preparation limit of 1,000,000.  The detail names that
limit.  This is a bounded static expansion failure, not a request to emit a
loop or to truncate the recipe.  The operation ID is attached.

Source: [`materialize.rs:Expansion::push_primitive`](../../src/materialize.rs#L557).

#### `MissingConcreteFormula`

`materialize_composition` first validates the descriptor and request, then
checks `has_concrete_materializer`.  A composition recipe that is valid as a
description but lacks an operation-specific tensor ABI, scalar SSA formula,
primitive parameters, and workspace policy receives this category.  The
detail names the symbol and source and points to the remaining-composition
manifest.  This explicit fail-closed result is distinct from
`GraphMaterializationFailed`: no concrete materializer was selected at all.

Source: [`materialize.rs:materialize_composition`](../../src/materialize.rs#L415).

### Materialization request admission

#### `InvalidMaterializationRequest`

This category means the caller supplied a malformed boundary contract or a
prepared fact that contradicts the operation's declared ABI.  The central
materializer uses `request_error` for these cases.  `validate_request` rejects
empty or duplicated named tensor declarations, repeated tensor identities,
inputs that are not marked `external_input`, and outputs that remain inputs or
are not marked `external_output`.  Missing named inputs and outputs, wrong
dtypes, shape-independent storage-contract mismatches, and false required
boolean facts use the same category.  Tensor metadata validation failures are
wrapped as `GraphMaterializationFailed` by the central `language_error` path;
the specialized modules choose their own wrapper at their concrete boundary.

The concrete request modules apply the same distinction at their own public
boundaries:

* Bayes rejects non-finite or non-positive smoothing, invalid reduction lanes,
  missing or mismatched caller boundary tensors, and a probability output that
  aliases an input.
* Binary metrics rejects malformed tensor validation, missing or mismatched
  caller boundaries, wrong metric input/output dtypes, and invalid boundary
  declarations.
* K-means rejects malformed or non-f32 rank-two matrix boundaries, invalid
  reduction lanes, output identities that alias an input or one another, and
  repeated boundary identities in its emitter.
* KNN rejects prediction identities that alias any source or another
  prediction, and missing or mismatched caller boundaries.
* Tree inference rejects non-finite scale, invalid reduction lanes, missing or
  mismatched caller boundaries, and an output identity that aliases an input.
* Graph/clustering/RL and tree/boosting helpers use `request_error` for
  request facts and for conversion failures that make a declared scalar
  action or parameter impossible to represent.

An invalid request is rejected before a successful graph fragment is returned.
The implementation does not repair flags, rename identities, or select a
different ABI.

Sources: [`materialize.rs:validate_request`](../../src/materialize.rs#L4268),
[`bayes.rs:validate_request`](../../src/bayes.rs#L296),
[`binary_metrics.rs:validate_request`](../../src/binary_metrics.rs#L698),
[`kmeans.rs:materialize_kmeans_lloyd_step`](../../src/kmeans.rs#L176),
[`knn_outputs.rs:validate_request`](../../src/knn_outputs.rs#L561),
[`tree.rs:validate_request`](../../src/tree.rs#L303).

#### `MissingPreparedParameter`

`prepared_u64`, `prepared_f32`, and `require_true` in the central materializer
emit this category when the requested key is absent from
`PreparedParameters`.  The same typed lookup pattern is used by the graph/
clustering/RL concrete materializer.  The detail names the missing key and
states whether it is a scalar parameter or a required fact.  There is no
code-side default: the preparation boundary must provide the value before
the graph can be made concrete.  The central helper qualifies the error with
the operation ID.

Source: [`materialize.rs:prepared_u64`](../../src/materialize.rs#L4418),
[`materialize.rs:prepared_f32`](../../src/materialize.rs#L4438),
[`materialize.rs:require_true`](../../src/materialize.rs#L4458),
[`graph_cluster_rl.rs:prepared_bool`](../../src/materialize/graph_cluster_rl.rs#L1561).

#### `PreparedParameterTypeMismatch`

The same lookup helpers emit this category when a key exists but carries the
wrong `PreparedParameter` variant.  The supported variants are `U64`, `I32`,
`F32Bits`, and `Bool`; each helper names the received value and expected
variant.  `require_true` treats `Bool(false)` as an invalid request fact, not
as a type mismatch, because the value has the correct type but violates the
contract.  Typed values are never coerced between variants.

Source: [`materialize.rs:prepared_u64`](../../src/materialize.rs#L4418),
[`materialize.rs:prepared_f32`](../../src/materialize.rs#L4438),
[`materialize.rs:require_true`](../../src/materialize.rs#L4458),
[`graph_cluster_rl.rs:prepared_bool`](../../src/materialize/graph_cluster_rl.rs#L1561).

### Concrete shape domain

#### `UnsupportedConcreteShape`

This category is used when a request is well-formed but its fixed dimensions,
dtype/shape contract, or canonical index domain cannot be represented by the
owned calculation.  It is not a generic malformed-request fallback.  The
actual checks include:

* nonzero dimensions and positive extents in Bayes, binary metrics, K-means,
  KNN, tree, convolution, and pooling preparation;
* int32 linear-index or histogram bounds, exact f32 count domains, and host
  `usize` capacity bounds;
* required even dimensions, legacy int32 extent ranges, kernel-fit geometry,
  and supported stride/padding/rotation shapes in concrete materializers;
* tensor dtypes and extents that do not match a concrete operation's declared
  ABI; and
* source-specific constraints such as the currently supported `k = 1`
  embedding path or a complete rather than partial legacy argsort prefix.

Checked products that overflow `u64` use `WorkspaceArithmeticOverflow`
instead; a product that is representable but exceeds the canonical int32
domain uses `UnsupportedConcreteShape`.  The operation-specific graph
materializers attach their descriptor ID through `operation_error`; standalone
convolution and pooling preparation has no descriptor and leaves it absent.

Sources: [`convolution.rs:prepare_channelwise_convolution_1d`](../../src/convolution.rs#L107),
[`pooling.rs:prepare_channelwise_max_pool_1d`](../../src/pooling.rs#L166),
[`materialize.rs:require_shape`](../../src/materialize.rs#L4380),
[`attention_sequence_embedding.rs:prepared_dimension`](../../src/materialize/attention_sequence_embedding.rs#L559),
[`convolution_pooling.rs:prepared_extent`](../../src/materialize/convolution_pooling.rs#L1423),
[`indexing_sort_encoding.rs:require_i32_indexable`](../../src/materialize/indexing_sort_encoding.rs#L1205),
[`tree.rs:tree_ensemble_inference_requirements`](../../src/tree.rs#L63).

### Identity reservations

#### `IdentityNamespaceOverlap`

Identity ranges are caller-reserved half-open intervals for generated
intermediate `ValueId`s and `KernelTemplateId`s.  This category means an
authoritative identity already belongs to another declaration or reservation:

* `validate_identity_namespaces` detects overlap between two caller ranges;
* central `GraphBuilder::new` rejects a boundary tensor inside its reserved
  intermediate value range;
* Bayes, binary metrics, K-means, KNN, and tree emitters reject the same
  boundary overlap;
* append APIs reject a generated intermediate or kernel already present in the
  caller graph; and
* the append APIs never rename a conflicting identity.

The range overlap checks are direct interval checks.  A zero-capacity range is
empty, while a nonempty overlap is rejected before graph assembly.  Group-level
namespace validation is unqualified; descriptor materialization and
operation-specific append failures carry the operation only when the caller
entered through a descriptor-backed path.

Sources: [`materialize.rs:validate_identity_namespaces`](../../src/materialize.rs#L319),
[`materialize.rs:GraphBuilder::new`](../../src/materialize.rs#L727),
[`bayes.rs:append_categorical_bayes_inference`](../../src/bayes.rs#L233),
[`binary_metrics.rs:append_binary_classification_metrics`](../../src/binary_metrics.rs#L448),
[`knn_outputs.rs:append_knn_all_outputs`](../../src/knn_outputs.rs#L493),
[`tree.rs:append_tree_ensemble_inference`](../../src/tree.rs#L237).

#### `IdentityNamespaceExhausted`

This category means a reserved identity range cannot supply the next required
value or kernel, or that computing the half-open range itself would overflow
`u64`.  The central materializer reports range-end overflow from
`identity_ranges`, value allocation exhaustion from `GraphBuilder::intermediate`,
and kernel allocation exhaustion from `GraphBuilder::emit`.  Each dedicated
Bayes, binary-metrics, K-means, KNN, and tree emitter performs the same checks
both at resource admission and on each allocation.  Their requirements phase
also rejects capacities smaller than the statically required number of values
or kernels.

No identity wraps around, is reused, or is silently expanded beyond the
caller reservation.  The error is qualified by a descriptor in central
materialization and unqualified in standalone request modules.

Sources: [`materialize.rs:identity_ranges`](../../src/materialize.rs#L4603),
[`materialize.rs:GraphBuilder::intermediate`](../../src/materialize.rs#L763),
[`materialize.rs:GraphBuilder::emit`](../../src/materialize.rs#L812),
[`kmeans.rs:KMeansEmitter`](../../src/kmeans.rs#L410).

### Workspace accounting

#### `WorkspaceLimitExceeded`

The predicted or accumulated operation-owned scratch exceeds the caller's
`workspace_limit`.  Central `GraphBuilder::intermediate` checks the running
byte total after each contiguous intermediate tensor is created.  Bayes,
binary metrics, K-means, KNN, and tree check their exact static requirement
before starting their emitters.  The detail reports required and permitted
bytes.  Boundary tensors and final outputs are excluded according to each
operation's resource formula, so this category is not a generic allocation
failure.

The limit is an admission contract.  The implementation returns the error
before a successful materialization and does not trim intermediates or choose
a lower-memory algorithm.

Sources: [`materialize.rs:GraphBuilder::intermediate`](../../src/materialize.rs#L763),
[`bayes.rs:validate_resources`](../../src/bayes.rs#L423),
[`binary_metrics.rs:materialize_binary_classification_metrics`](../../src/binary_metrics.rs#L156),
[`kmeans.rs:validate_resources`](../../src/kmeans.rs#L615),
[`knn_outputs.rs:validate_resources`](../../src/knn_outputs.rs#L649),
[`tree.rs:validate_resources`](../../src/tree.rs#L426).

#### `WorkspaceFormulaMismatch`

This category means a static workspace/resource formula and the independently
emitted result disagree, or a workspace formula receives the wrong number of
dimensions.  `WorkspaceFormula::evaluate` rejects a dimensions slice whose
length is not the formula's declared arity.  Bayes, KNN, and tree emitter
`finish` methods compare emitted intermediate count, kernel count, and bytes
with their requirements.  Binary metrics compares the same quantities in
separate checks: intermediate/kernel count mismatches are
`GraphMaterializationFailed`, while a byte mismatch is
`WorkspaceFormulaMismatch`.  K-means compares its emitted byte total with its
requirements under this category.

The mismatch is a failed internal accounting invariant, not a request to
accept the larger value.  A graph is returned only after the formula and
emission agree exactly.

Sources: [`workspace.rs:WorkspaceFormula::evaluate`](../../src/workspace.rs#L155),
[`bayes.rs:BayesEmitter::finish`](../../src/bayes.rs#L643),
[`binary_metrics.rs:MetricEmitter::finish`](../../src/binary_metrics.rs#L647),
[`kmeans.rs:KMeansEmitter::finish`](../../src/kmeans.rs#L533),
[`knn_outputs.rs:KnnAllOutputEmitter::finish`](../../src/knn_outputs.rs#L839),
[`tree.rs:TreeEmitter::finish`](../../src/tree.rs#L621).

#### `WorkspaceArithmeticOverflow`

Every operation-owned byte and element formula uses checked `u64`
arithmetic.  This category is constructed when a checked multiply, add, next
power-of-two, shift, or range total cannot be represented.  It is used by:

* `WorkspaceFormula` reductions, scans, sorts, solver images, and split-K
  formulas;
* central composition workspace accumulation;
* Bayes, binary metrics, K-means, KNN, and tree requirement and emitter
  accounting;
* concrete attention, convolution/pooling, graph/RL, and indexing/sort
  materializers; and
* pooling flat-coordinate and workspace arithmetic.

The standalone convolution preparation uses its local `overflow` helper for
both shape and byte multiplication and classifies that overflow as
`UnsupportedConcreteShape`, because that API exposes a concrete preparation
shape rather than the generic workspace formula boundary.  The standalone
pooling preparation uses `WorkspaceArithmeticOverflow`.  No checked operation
falls back to wrapping arithmetic or a sentinel size.

Sources: [`workspace.rs:workspace_overflow`](../../src/workspace.rs#L337),
[`materialize.rs:GraphBuilder::intermediate`](../../src/materialize.rs#L763),
[`attention_sequence_embedding.rs:checked_product`](../../src/materialize/attention_sequence_embedding.rs#L599),
[`convolution_pooling.rs:checked_product`](../../src/materialize/convolution_pooling.rs#L1435),
[`indexing_sort_encoding.rs:checked_product`](../../src/materialize/indexing_sort_encoding.rs#L1047),
[`pooling.rs:arithmetic_overflow`](../../src/pooling.rs#L307).

### Graph assembly and concrete emission

#### `GraphMaterializationFailed`

This category covers failures after a concrete operation has been selected,
when the requested calculation graph cannot be constructed or validated.  The
central `materialize.rs` helpers use it for:

* missing concrete family dispatch after the registry says a composition is
  supported;
* a concrete materializer emitting too many or too few kernels for the
  resolved recipe;
* an empty stage, a primitive family different from the resolved step, or a
  caller graph that already has a conflicting producer;
* `Shape`, `Tensor`, `AxisSet`, scalar-builder, or `CalculationGraph`
  validation errors converted by `language_error`; and
* graph validation at `GraphBuilder::finish`.

Each concrete family module has a dispatch sentinel that emits this category
when its source symbol is not handled.  The sentinels are present in
attention/sequence/embedding, convolution/pooling, graph/clustering/RL,
indexing/sort/encoding, optimizer/normalization, solver/FFT, and
tree/boosting materializers.  The dedicated Bayes, binary-metrics, K-means,
KNN, and tree APIs use `graph_error` for language graph failures and use the
same category when an append operation would duplicate an output producer or
a caller graph repeats a tensor.  Binary metrics additionally uses this
category for intermediate/kernel reservation count mismatches, as noted above.

`GraphMaterializationFailed` is therefore distinct from
`InvalidMaterializationRequest`: the latter rejects the caller's declared ABI
or prepared facts, while this category says the operation's concrete graph
construction or validation failed after admission.  Lower-level language
errors are rendered into `detail`; the `std::error::Error::source` chain is not
retained.

Sources: [`materialize.rs:dispatch_concrete`](../../src/materialize.rs#L452),
[`materialize.rs:Emitter::emit_stage`](../../src/materialize.rs#L646),
[`materialize.rs:GraphBuilder::finish`](../../src/materialize.rs#L835),
[`attention_sequence_embedding.rs:dispatch`](../../src/materialize/attention_sequence_embedding.rs#L45),
[`convolution_pooling.rs:dispatch`](../../src/materialize/convolution_pooling.rs#L90),
[`graph_cluster_rl.rs:dispatch`](../../src/materialize/graph_cluster_rl.rs#L45),
[`indexing_sort_encoding.rs:dispatch`](../../src/materialize/indexing_sort_encoding.rs#L45),
[`tree_boosting.rs:dispatch`](../../src/materialize/tree_boosting.rs#L60).

## Shared invariants

The concrete implementations rely on a small set of invariants that the
error categories keep visible:

1. A descriptor must be resolved uniquely before descriptor-qualified lowering
   or materialization.  Missing and duplicate symbols are never guessed.
2. The selected `LoweringAvailability` must match the API being called.  A
   scalar, primitive, composition, workspace, non-calculation, and unsupported
   entry are separate branches with no compatibility shim.
3. Scalar programs and primitive kernels are validated before a graph or
   lowered program is returned.  A failed validation never yields a partial
   SSA program or altered kernel.
4. Concrete materialization consumes caller-declared tensor identities, typed
   prepared parameters, and explicit shape facts.  Missing facts, wrong types,
   unsupported extents, and malformed boundary flags remain distinct.
5. Generated values and kernels come only from caller-reserved half-open
   ranges.  Overlap and exhaustion fail before identity reuse or graph append.
6. Workspace totals are checked against both a caller limit and the operation's
   exact formula.  Arithmetic overflow is observable and never wraps.
7. A graph is returned only after the concrete stage sequence, tensor contracts,
   alias rules, node identities, and `CalculationGraph::validate` all succeed.

These invariants explain why the enum has separate categories for request
admission, unsupported shape, identity reservation, workspace accounting, and
graph assembly.  They are not interchangeable status labels.

## Relevant source map

| Boundary | Implementation | What the error module contributes |
| --- | --- | --- |
| Registry | [`ops/src/registry.rs`](../../src/registry.rs) | Lookup categories and operation identity selection. |
| Scalar lowering | [`ops/src/scalar.rs`](../../src/scalar.rs) | Scalar recipe ownership and SSA validation categories. |
| Primitive lowering | [`ops/src/primitive.rs`](../../src/primitive.rs) | Recipe matching and backend-neutral lowering failure categories. |
| Composition | [`ops/src/composition.rs`](../../src/composition.rs) and [`ops/src/materialize.rs`](../../src/materialize.rs) | Recipe validity, bound resolution, identity, workspace, and graph emission categories. |
| Static workspace | [`ops/src/workspace.rs`](../../src/workspace.rs) | Formula arity and checked arithmetic categories. |
| Concrete operation graphs | [`ops/src/bayes.rs`](../../src/bayes.rs), [`ops/src/binary_metrics.rs`](../../src/binary_metrics.rs), [`ops/src/kmeans.rs`](../../src/kmeans.rs), [`ops/src/knn_outputs.rs`](../../src/knn_outputs.rs), [`ops/src/tree.rs`](../../src/tree.rs) | Request, shape, identity, workspace, and graph categories at specialized public boundaries. |
| Recipe-owned preparation | [`ops/src/convolution.rs`](../../src/convolution.rs) and [`ops/src/pooling.rs`](../../src/pooling.rs) | Concrete shape and checked preparation arithmetic categories. |
| Public facade | [`src/facade.rs`](../../../src/facade.rs) | Re-exports the types and forwards operation calls without remapping. |
| Compile propagation | [`training/src/error.rs`](../../../training/src/error.rs) and [`training/src/inference.rs`](../../../training/src/inference.rs) | Converts an operation error to the outer `Operation` compile category using its rendered text. |

The error module itself remains intentionally free of operation-specific
branches.  New checks belong beside the state they can observe, and they must
select an existing category whose detail states the actual violated contract.
