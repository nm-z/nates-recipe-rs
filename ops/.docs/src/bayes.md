# `ops/src/bayes.rs`: observed categorical Bayesian inference

## Module identity

```text
crate: recipe-ops
source: ops/src/bayes.rs
scope: one observed categorical conditional inference graph
calculation_dtype: I32 for codes, indices, and counts; F32 for probabilities
smoothing_contract: finite positive request value; public semantic artifacts use Laplace-one (1.0)
graph_kind: recipe_language::CalculationGraph
```

This module materializes the native payload calculation for one conditional
distribution

```text
P(child_class | ordered_parent_configuration)
```

from saved reference observations and target-free query parent codes. It does
not fit a model, resolve names, decode an OGDL artifact, perform host-side
counting, choose a native backend, allocate device memory, or execute a graph.
The training and inference crates own those boundaries. The module is the
typed operation boundary that turns already validated tensors into a static
calculation graph.

The public exports are re-exported by `ops/src/lib.rs`:

| Export | Role |
|---|---|
| `CategoricalBayesInferenceRequest` | Boundary tensors, concrete extents, smoothing, reduction lanes, identities, and workspace limit. |
| `CategoricalBayesInferenceRequirements` | Checked reservation: exactly 10 intermediate values, 11 kernels, and a byte count. |
| `CategoricalBayesInferenceMaterialization` | Standalone graph, generated intermediate value IDs, generated kernel IDs, workspace bytes, and the unchanged identity namespace. |
| `categorical_bayes_inference_requirements` | Pure checked resource calculation. |
| `materialize_categorical_bayes_inference` | Validate a request and build one standalone graph fragment. |
| `append_categorical_bayes_inference` | Validate caller-owned boundaries, materialize, and append the fragment to an existing graph. |

The older operation-surface entries such as
`gpu_nb_count_table` and `gpu_nb_feature_log_prob` are registry/composition
descriptors. They are not calls into this module. This module implements the
observed categorical Bayes instrument described by system-contract C36 and C42.

## Concrete request contract

Use these symbols for one materialization:

```text
R = reference_rows       (saved training/reference rows)
Q = query_rows           (target-free inference rows)
P = parent_count         (parents in literal declaration order)
G = parent_configurations (product of parent cardinalities)
C = child_classes        (known child dictionary entries)
B = G * C                (joint histogram bins)
```

Every dimension is a `u64` in the operation request. The tensor shape and dtype
are part of the contract, not inferred from a buffer length.

| Request field | Dtype and exact shape | Boundary meaning and invariant |
|---|---|---|
| `reference_parent_codes` | `I32 [R, P]` | Row-major parent codes for each saved reference row. A row stores parents in declaration order. Each code is a known dictionary code, so it is in `0..dictionary_len`; the reserved unseen route is not used by reference observations. |
| `reference_child_codes` | `I32 [R]` | Known child dictionary code for each saved reference row, in `0..C`. Child codes have no reserved unseen class. |
| `query_parent_codes` | `I32 [Q, P]` | Row-major parent codes for query rows. A code may equal the reserved unseen route, `dictionary_len`, and must be in `0..cardinality` where `cardinality = dictionary_len + 1`. |
| `parent_multipliers` | `I32 [P]` | Mixed-radix multiplier for each ordered parent. For parent `i`, `multiplier[i] = product(cardinality[j] for j > i)`. |
| `parent_cardinalities` | `I32 [P]` | The inference cardinality of each parent, including exactly one reserved unseen route. |
| `probabilities` | `F32 [Q, C]` | Caller-declared output tensor. It must not be an input, must not alias any input, and is marked external output by the caller. |

The scalar request fields are:

| Field | Contract |
|---|---|
| `reference_rows`, `query_rows`, `parent_count`, `parent_configurations`, `child_classes` | All nonzero. `G`, `P`, and `C` must fit the checked signed `i32` domain; `B` must fit both `i32` and `u32` histogram domains. |
| `smoothing` | Finite and strictly positive. The public observed categorical artifact supplies `1.0`; the operation itself accepts any finite positive value. |
| `tree_lanes` | Power of two in `1..=1024`. Inference callers use `MAXIMUM_REDUCTION_TREE_LANES`, which is `1024`. |
| `identity_namespace` | Caller-reserved half-open ranges for generated intermediate `ValueId`s and `KernelTemplateId`s. Boundary IDs are supplied separately and must not overlap the value range. |
| `workspace_limit` | Maximum bytes permitted for operation-owned intermediate tensors. Boundary tensors and the final probability output are not charged to this number. |

`Tensor::validate` runs before the semantic checks. Thus each tensor must also
have a valid layout, storage span, and byte count.

Input boundary fields may reuse one `ValueId` only when the repeated tensor
contracts are identical; `BayesEmitter::insert_boundary` deduplicates that
case. The probability output may never reuse an input ID, even when its storage
contract would otherwise match.

The operation does not recompute the mixed-radix product from
`parent_cardinalities`, compare `parent_multipliers` with that product, inspect
code values in host memory, or verify the row counts against a separate dataset
object. Those semantic facts are established by the training reference-set
validator and inference query preparation. The scalar `Require` guards and
checked graph/index domains are the operation-level enforcement boundary.

## Mathematical semantics

For one row, let `x[i]` be the code for ordered parent `i`, `m[i]` its
multiplier, and `cardinality[i]` its reserved-route-inclusive cardinality.
The graph computes the mixed-radix configuration

```text
configuration(x) = sum(i = 0 .. P-1, x[i] * m[i])
```

with `0 <= configuration(x) < G`. A reference row with child code `y` maps to
one joint histogram bin:

```text
joint_bin(x, y) = configuration(x) * C + y
```

The histogram count for bin `g * C + c` is

```text
N[g, c] = number of reference rows whose configuration is g and child code is c
```

For a query configuration `g`, the graph gathers the `C` counts
`N[g, 0..C-1]`, sums them to `T[g]`, and emits the fixed Laplace posterior for
each child class:

```text
probability[g, c] = (N[g, c] + smoothing) / (T[g] + smoothing * C)
```

The denominator is strictly positive because `smoothing > 0` and `C > 0`.
When a query label is missing or unseen, ingestion maps it to the parent's
reserved route. If that complete configuration has no reference rows, all
`N[g, c]` values are zero and the result is uniform.

Ingestion retains a separate typed `Missing` versus `Unseen { label }` route
for diagnostics, but both routes deliberately carry the same calculation code
`dictionary.len()` into this graph. The Bayes operation therefore models one
reserved unknown configuration, not two extra probability classes.

The child dictionary is the complete output class set. There is no reserved
child output class. Public reporting uses a strict greater-than comparison, so
an exact probability tie retains the lowest saved child code.

### Worked contract example

For the singular cookbook artifact, `weather` and `wind` each have two known
labels, so `P = 2`, `cardinality = [3, 3]`, `m = [3, 1]`, `G = 9`, and the
`play` child has `C = 2`. The operation therefore emits `B = 18` histogram
bins. A query `[weather=sun, wind=breeze]` has code `[1, 0]`, configuration
`1 * 3 + 0 = 3`, and joint bins `6` and `7`. If the reference partition has one
`otter` row and no `falcon` row at configuration 3, the posterior is

```text
[(0 + 1) / (1 + 2), (1 + 1) / (1 + 2)] = [1/3, 2/3]
```

An unseen `weather` label uses code `2`. With `wind=breeze` the resulting
configuration is `2 * 3 + 0 = 6`; if no reference row uses configuration 6,
the output is `[1/2, 1/2]`. In the repeated cookbook artifact, `play` and
`travel` remain independent conditionals with widths 2 and 3, and the final
packed matrix width is 5. The shared `weather` feature is prepared once but is
packed independently for each conditional's own parent order.

## Emitted graph

`materialize_categorical_bayes_inference` creates boundary tensor declarations
and then emits the following fixed sequence. The `K0` through `K10` labels are
relative order labels; actual IDs start at
`identity_namespace.first_kernel()` and increase by one. Intermediate values
start at `identity_namespace.first_value()` and increase by one.

All operation kernels use a complete input/output alias matrix with
`AliasPermission::Forbidden`. No operation may overwrite an input tensor.

| Order | Intermediate tensor | Shape and dtype | Primitive kernel | Inputs -> outputs | Meaning |
|---:|---|---|---|---|---|
| `K0` | `reference_contributions` | `I32 [R, P]` | `Elementwise(configuration_contribution_program)` | `reference_parent_codes`, `parent_multipliers`, `parent_cardinalities` -> `reference_contributions` | Checks each code is in `0..cardinality`, then multiplies code by its multiplier. |
| `K1` | `reference_configurations` | `I32 [R]` | `Reduce(Sum, axis=1, keep_dimensions=false, tree_lanes)` | `reference_contributions` -> `reference_configurations` | Sums one row's parent contributions. |
| `K2` | `reference_joint_bins` | `I32 [R]` | `Elementwise(joint_bin_program(C))` | `reference_configurations`, `reference_child_codes` -> `reference_joint_bins` | Computes `configuration * C + child_code`; checks child code range. |
| `K3` | `counts` | `I32 [B]` | `Histogram(bins=B, weighted=false, ordering=Relaxed)` | `reference_joint_bins` -> `counts` | Accumulates all reference rows into the global `(configuration, class)` table. |
| `K4` | `query_contributions` | `I32 [Q, P]` | `Elementwise(configuration_contribution_program)` | `query_parent_codes`, `parent_multipliers`, `parent_cardinalities` -> `query_contributions` | Packs every query parent row with the same mixed-radix contract. |
| `K5` | `query_configurations` | `I32 [Q, 1]` | `Reduce(Sum, axis=1, keep_dimensions=true, tree_lanes)` | `query_contributions` -> `query_configurations` | Produces one configuration per query row while retaining a broadcastable column axis. |
| `K6` | `class_offsets` | `I32 [1, C]` | `IndexMap(start=0, element_step=1, iteration_step=0, modulus=None)` | no inputs -> `class_offsets` | Generates class codes `0..C-1`. |
| `K7` | `query_bins` | `I32 [Q, C]` | `Elementwise(query_bin_program(C))` | `query_configurations`, `class_offsets` -> `query_bins` | Broadcasts each query configuration across classes and adds the class offset. |
| `K8` | `selected_counts` | `I32 [Q, C]` | `Gather(axis=0, bounds=Reject)` | `counts`, `query_bins` -> `selected_counts` | Selects each query row's `C` global bins. Out-of-range indices report a device fault. |
| `K9` | `totals` | `I32 [Q, 1]` | `Reduce(Sum, axis=1, keep_dimensions=true, tree_lanes)` | `selected_counts` -> `totals` | Sums selected class counts for each query configuration. |
| `K10` | none | output `F32 [Q, C]` | `Elementwise(posterior_probability_program(C, smoothing))` | `selected_counts`, `totals` -> `probabilities` | Requires nonnegative counts and totals, converts to F32, and emits the posterior formula. |

The graph has exactly these 11 nodes and exactly these 10 operation-owned
intermediate tensors. `probabilities` is a caller-owned boundary tensor and is
not included in `intermediate_values` or workspace accounting. The emitter
finishes by constructing `CalculationGraph`, calling `graph.validate()`, and
returning the graph plus the generated IDs and exact byte count.

### Scalar program guards

The three elementwise scalar programs are intentionally small and checked:

1. `configuration_contribution_program` has three `I32` inputs. It uses
   `Require(0 <= code < cardinality)` before multiplying by the multiplier.
2. `joint_bin_program` and `query_bin_program` use an `I32` child/class input,
   require `0 <= class < C`, then compute `configuration * C + class`.
3. `posterior_probability_program` requires both `count` and `total` to be
   nonnegative before converting to F32 and applying smoothing. It does not
   clamp, replace, or silently recover from an invalid count.

The request's checked domains and the reference-set mixed-radix invariant make
the configuration and bin arithmetic fit `i32`; device `Require` instructions
keep malformed payload codes visible as calculation faults.

### Helper inventory

| Helper | Responsibility |
|---|---|
| `require_i32_range` | Emits the `0 <= value < upper_exclusive` scalar guard used by code/class programs. |
| `scalar_i32` | Converts a checked `u64` semantic extent to an `I32` scalar literal and reports `UnsupportedConcreteShape` on failure. |
| `same_storage_contract` | Compares value ID, dtype, shape, layout, and storage bytes, deliberately excluding external-boundary flags. |
| `insert_boundary` | Inserts a boundary tensor into the ID-ordered map, accepting an exact duplicate contract and rejecting a conflicting contract. |
| `forbidden_aliases` | Generates one `AliasPermission::Forbidden` rule for every input/output pair. |
| `shape` | Builds a nonempty `Shape` and wraps language failures as `GraphMaterializationFailed`. |
| `checked_product` / `checked_sum` | Performs checked `u64` arithmetic for bins, elements, and workspace formulas. |
| `graph_error` | Converts language, shape, layout, and scalar-program errors to an operation graph-materialization failure. |
| `bayes_error` | Constructs an `OperationError` with the selected kind, detail, and no operation descriptor. |

## Resource and identity accounting

`categorical_bayes_inference_requirements` is the single source of the static
reservation. It checks products and sums with checked `u64` arithmetic.

```text
B = G * C
reference_parent_elements = R * P
query_parent_elements     = Q * P
query_output_elements     = Q * C

workspace_elements =
    (R * P) + R + R + B + (Q * P) + Q + C + (Q * C) + (Q * C) + Q

workspace_bytes = workspace_elements * 4
intermediate_values = 10
kernels = 11
```

Every intermediate is contiguous `I32`, so four bytes per logical element is
the complete operation-owned workspace formula. The final `F32 [Q, C]` output
and all six boundary inputs are excluded. Backend lowering may derive its own
fixed reduction or fault-channel scratch from these primitives; that lowering
scratch is not represented as a Bayes tensor or charged by this graph formula.
The emitter independently adds each
intermediate tensor's `storage_bytes`; `finish` rejects any difference between
the emitted count and the formula with `WorkspaceFormulaMismatch`.

The namespace ranges are half-open:

```text
value range  = first_value  .. first_value  + value_capacity
kernel range = first_kernel .. first_kernel + kernel_capacity
```

The caller must reserve at least 10 values and 11 kernels. `BayesEmitter::new`
checks range-end overflow, rejects a boundary tensor whose value ID lies inside
the intermediate range, and retains the namespace in the returned
materialization. Each allocation checks exhaustion again, so a short or
overflowed range fails closed rather than wrapping an identity.

## Standalone materialization versus append

### `materialize_categorical_bayes_inference`

The standalone entry point runs, in order:

1. `validate_request`: tensor validity, dimensions, dtypes, shapes, smoothing,
   and reduction lane checks.
2. `validate_resources`: identity capacities and workspace limit.
3. `BayesEmitter::new`: boundary normalization, identity range checks, and
   boundary insertion into a deterministic `BTreeMap`.
4. The fixed K0-K10 emission sequence above.
5. `BayesEmitter::finish`: exact accounting and language graph validation.

The result's graph includes all six boundary declarations and all generated
intermediate declarations. Boundary flags are normalized in the emitter:
reference/query code tensors are external inputs, while `probabilities` is an
external output.

### `append_categorical_bayes_inference`

The append entry point is the composition boundary used by inference. It first
validates caller storage, then materializes a standalone fragment, checks ID and
producer collisions, and finally appends only intermediate tensors and all
materialized nodes:

1. Every caller tensor ID must be unique. A repeated ID is
   `GraphMaterializationFailed`.
2. Every declared boundary tensor must already exist in the caller graph and
   match ID, dtype, shape, layout, and storage bytes. Missing or mismatched
   boundaries are `InvalidMaterializationRequest`.
3. The generated intermediate IDs must not already occur in caller tensors, and
   generated kernel IDs must not already occur in caller nodes. Either overlap
   is `IdentityNamespaceOverlap`.
4. The caller graph must not already have a node producing `probabilities`.
   Such a duplicate producer is `GraphMaterializationFailed`.
5. The operation-owned intermediate tensor declarations are extended into the
   caller graph, followed by all 11 nodes. The caller remains responsible for
   final graph validation after any other fragments are assembled.

The append function does not silently rename IDs, replace a conflicting tensor,
reuse an output producer, or create a second output path.

## Operation failure contract

All errors are `OperationError` with `operation: None`; helpers preserve the
specific `OperationErrorKind` below. Language, shape, scalar-program, and graph
errors are wrapped as `GraphMaterializationFailed` by `graph_error`.

| Check site | Error kind | Failing condition |
|---|---|---|
| `categorical_bayes_inference_requirements` | `UnsupportedConcreteShape` | Any of `R`, `Q`, `P`, `G`, or `C` is zero; `B` is above the checked `i32` or `u32` histogram domain; `G`, `P`, or `C` is above `i32::MAX`. |
| Checked dimension arithmetic | `WorkspaceArithmeticOverflow` | A product or sum for bins, elements, or workspace bytes overflows `u64`. |
| `validate_request` tensor validation | `GraphMaterializationFailed` | Invalid tensor layout, storage span, shape, or byte-size contract. |
| `validate_request` dtype/shape table | `UnsupportedConcreteShape` | Any six boundary tensors differs from its required dtype or exact extent. |
| `validate_request` scalar policy | `InvalidMaterializationRequest` | Smoothing is NaN, infinite, or nonpositive; `tree_lanes` is zero, non-power-of-two, or above 1024. |
| `validate_boundary_storage` | `GraphMaterializationFailed` | Caller graph repeats a tensor ID. |
| `validate_boundary_storage` | `InvalidMaterializationRequest` | A declared boundary tensor is absent, or its storage contract differs. |
| `BayesEmitter::new` | `InvalidMaterializationRequest` | The probability output ID aliases an input boundary. |
| `BayesEmitter::new` | `IdentityNamespaceOverlap` | A boundary value ID lies in the reserved intermediate range. |
| Namespace end/allocation | `IdentityNamespaceExhausted` | First ID plus capacity overflows, or emission would consume a value or kernel outside its reserved range. |
| `validate_resources` | `IdentityNamespaceExhausted` | Capacity is below 10 intermediate values or 11 kernels. |
| `validate_resources` | `WorkspaceLimitExceeded` | Formula workspace bytes exceed `workspace_limit`. |
| `BayesEmitter::intermediate` | `WorkspaceArithmeticOverflow` | Summing an intermediate tensor's storage bytes overflows `u64`. |
| `BayesEmitter::finish` | `WorkspaceFormulaMismatch` | Emitted value count, kernel count, or byte sum differs from the checked requirements. |
| `append_categorical_bayes_inference` | `IdentityNamespaceOverlap` | Generated value or kernel ID already exists in the caller graph. |
| `append_categorical_bayes_inference` | `GraphMaterializationFailed` | Caller graph already produces the requested probability output. |
| Final `CalculationGraph::validate` | `GraphMaterializationFailed` | Unknown tensors, duplicate producers/kernels, missing producers, alias violations, invalid primitive shapes, or a cycle are found. |

There are no fallback paths, retries, alternate kernels, implicit host counts,
or defensive substitutions. An invalid code reaches the scalar `Require` or a
checked gather fault; it is not clamped by this operation.

## Downstream lowering obligations

The graph is consumed by the ordinary `PrimitiveKind` lowering path. The
operation does not attach backend-specific code. `primitives::lower` first
validates every kernel and its alias matrix, then lowers the primitive kind into
the same stage model used by all other calculations.

| Bayes primitive | Lowering contract that remains observable |
|---|---|
| `Elementwise` | Scalar input dtypes, broadcast shapes, output dtype, and `Require` instructions are validated before a lane executes. |
| `Reduce(Sum)` | `tree_lanes` selects a fixed collective reduction tree. The realized stage retains the ordered tree and lowest-logical-index tie policy used by the generic reduction contract. |
| `Histogram` | Lowering emits a clear stage for all `B` I32 bins, then an I32-direct accumulation stage using atomic `Add` with the requested `Relaxed` ordering. Invalid signed bins publish the preallocated `HistogramBinOutOfBounds` fault before addressing the output. |
| `Gather(axis=0, Reject)` | Lowering emits the exact preallocated `IndexOutOfBounds` fault guard before calculating the payload address. It never clamps or wraps the index. |
| `IndexMap` | The zero-input class-offset map is a deterministic affine index stage with no payload input. |

The reference and query scalar range guards make valid bins and gather indices
mathematically in range, but the downstream fault paths remain part of the
graph contract. A malformed external image therefore fails through the real
fault channel rather than receiving an implicit substitute value.

## Training and artifact caller path

The operation receives data only after the training crate has enforced the
observed categorical contract.

### Declaration and preparation

1. `Model::bayes(child, parents)` in `src/api.rs` retains each call in source
   order. Declaration validation rejects empty names, duplicate parents,
   self-edges, duplicate children, and cycles.
2. `compile_bayes_model` in `src/training.rs` validates the policy, data, and
   model. It rejects layers or loaded weights, generic objectives, gradient
   policy, normalization, optimizer/learning-rate/epoch controls, iterative
   metrics, and native training-kernel save/resume declarations. Bayesian
   preparation has no optimizer loop and no training kernel.
3. `prepare_categorical_bayesian_reference_sets` in
   `training/src/bayes.rs` resolves the schema, requires nonempty declarations,
   observed nodes, nonempty training data, at least one parent per conditional,
   target children in exactly declaration order, feature parents, and
   dictionary-encoded categorical `I32` vectors.
   Schema node IDs are canonical ascending-name identities and schema
   `execution_order` is deterministic, but this observed instrument consumes
   the original declaration order instead. It does not perform ancestral graph
   evaluation.
4. Every retained training row must have a known child and known parent code.
   The prepared reference set stores source-row order, row-major parent codes,
   child codes, dictionaries, reserved-route cardinalities, mixed-radix
   multipliers, and `G`. It stores no counts or probabilities.
5. `BayesModelArtifact::from_conditionals` chooses semantic format version 1
   for one conditional and version 2 for two or more. The artifact requires
   Recipe's canonical Laplace-one smoothing. Resume appends current rows after
   saved rows only when the ordered schemas, dictionaries, declarations, and
   shared reference partition match exactly. If a declared resume path is
   absent, `compile_bayes_model` starts a fresh artifact; it does not reject an
   independent model save.

The default decoder limits are finite and aggregate across repeated
conditionals:

| Limit | Default |
|---|---:|
| `source_bytes` | `1 << 30` |
| `nodes` | `4_000_000` |
| `conditionals` | `65_536` |
| `parents` | `65_536` |
| `labels` | `1_000_000` |
| `reference_rows` | `100_000_000` |
| `total_payload_bytes` | `1 << 30` |

The canonical OGDL shape is:

```text
recipe-bayes-model
  format-version <1 or 2>
  smoothing laplace-one
  # version 1: reference fields are direct children of the root
  # version 2: conditionals / conditional / reference fields, repeated in order
  reference-rows <decimal usize>
  reference-source-rows <0x + 16 hex digits per source row>
  parents
    parent
      source-index <decimal usize>
      name-bytes <0x byte encoding>
      labels
        value-bytes <0x byte encoding>
  child
    source-index <decimal usize>
    name-bytes <0x byte encoding>
    labels
      value-bytes <0x byte encoding>
  reference-parent-codes <0x + 8 hex digits per I32 code>
  reference-child-codes <0x + 8 hex digits per I32 code>
```

Version 2 requires at least two `conditional` entries and preserves their
source order. Decoding accepts only the required fields, canonical decimal
numbers, exact hex lengths, `0x` prefixes, one root named
`recipe-bayes-model`, and a byte-for-byte canonical re-encoding. The decoder
recomputes parent cardinalities and mixed-radix metadata from the dictionaries,
then runs the same reference-set and repeated-schema validation used by the
training path.

`Train::run` wraps the result in `TrainingReport::bayes` and writes only the
declared semantic `.ogdl` model destination. It never reports an optimizer
loop, native training kernel, or training metric for this family. The runnable
examples use the same public path: `examples/cookbook.rs:398-421` trains and
saves `cookbook-bayes.ogdl` and `cookbook-bayes-multi.ogdl`, then loads each
through target-free inference.

### Inference preparation and operation invocation

1. `load_semantic_model_file` dispatches the strict root
   `recipe-bayes-model` to `decode_bayes_model`; it never falls back to another
   model decoder. The generic semantic loader uses
   `BayesModelDecodeLimits::default()`, while the direct
   `load_bayes_model_file` boundary accepts an explicit limit set. Both apply
   bounded source and payload decoding.
2. `prepare_bayes_inference_table` builds the union of all saved parent
   schemas. First occurrence in conditional/declaration order fixes physical
   feature order, and a parent shared by conditionals is read once. Target
   columns are not required in target-free inference.
3. `compile_prepared_bayes_inference` rejects zero query rows, an empty
   conditional list, unrepresentable extents, and aggregate output-width
   overflow. It calls `compile_bayes_conditional` once per conditional in
   artifact order.
4. `compile_bayes_conditional` finds each prepared parent by source identity and
   name, requires dictionary-coded `I32` query values, checks every query code
   against its reserved-route-inclusive cardinality, and creates the five
   external `I32` inputs plus an `F32 [Q, C]` output. Each external image is
   serialized as little-endian bytes and is admitted only when its byte length
   equals `shape.bytes(dtype)`; the role is retained in
   `InferenceExternalInput`. It calls
   `categorical_bayes_inference_requirements`, reserves exactly that many IDs,
   builds a request with `tree_lanes = 1024`, and calls
   `append_categorical_bayes_inference`.
   The request's `workspace_limit` is set to the same checked byte count
   returned by `categorical_bayes_inference_requirements`, so the caller does
   not invent a second Bayes workspace policy.
5. Each returned operation kernel is assigned `IterationDomain::first()` and
   merged back into the compiler. The compiler then canonicalizes and reparses
   the calculation graph and static program through OGDL, with exactly one
   inference iteration. The static-program validator requires every generated
   kernel to have exactly one explicit domain and rejects a consumer whose first
   iteration precedes its producer. Since every Bayes kernel uses the same first
   domain, the graph's value dependencies, not host ordering guesses, determine
   execution order.

The five per-conditional external input roles are
`BayesReferenceParents`, `BayesReferenceChild`, `BayesQueryParents`,
`BayesParentMultipliers`, and `BayesParentCardinalities`. Repeated-conditionals
concatenation adds three external `I32` index/select tables per join and uses
the existing `gpu_concat_into` composition. It checks all table lengths and
indices against `i32` and `usize` before materializing that composition. Each
join reserves the inference compiler's separate 64-value and 64-kernel
composition namespace; those reservations are not part of the Bayes operation's
10-value and 11-kernel requirements.

## Public inference boundary and output

`src/inference.rs` requires target-free data: no `.target(...)`, `.split(...)`,
or redeclared normalization is accepted. It loads the `.ogdl` artifact,
distills/selects query rows, prepares the shared parent table, compiles the
Bayesian graph, and dispatches `CompiledModelInference::Bayes` through the
ordinary measured native preparation and execution lifecycle. Native execution
loads the finalized graph, admits external images, runs one static iteration,
collects the output, and performs the normal teardown. This module itself does
not perform any of those lifecycle steps.

The compiled result is:

```text
InferencePredictionKind: BayesProbabilities
InferenceTask: BayesProbabilities { width = sum(child_classes per declaration) }
output dtype: F32
output shape: [Q, sum(child_classes per declaration)]
target_dtypes: one I32 entry per saved conditional (source representation only)
```

For one conditional, the output is its `[Q, C]` matrix. For repeated
conditionals, each independent `[Q, C_i]` matrix is concatenated on device into
one row-major matrix. The column range for declaration `i` is the sum of class
widths of declarations before `i`, followed by that conditional's `C_i`
columns. The public report exposes this contract through:

| Report accessor | Meaning |
|---|---|
| `prediction()` / `values()` | The validated little-endian F32 probability matrix. |
| `bayes_output_count()` | Number of conditionals in declaration order. |
| `bayes_output_name(i)` | Saved child name for conditional `i`. |
| `bayes_output_classes(i)` | Saved known class count `C_i`. |
| `bayes_output_range(i)` | Packed output column range for conditional `i`. |
| `decode_bayes_output_class(i, c)` | Saved dictionary label for class `c`; no reserved child class exists. |
| `decode_bayes_class(c)` | Compatibility accessor for conditional zero. |

Terminal report logging validates kind, rank, width, row byte count, total byte
count, and saved label widths before printing one record per row and conditional.
Each record includes the target name for repeated outputs, the lowest-code
argmax class under exact ties, its saved byte label, and all class probabilities.

## Caller failure classes

Failures before or after `recipe-ops` retain their own typed boundary. They are
not converted into an operation error unless the operation call itself fails.

| Boundary | Typed failures that can stop a Bayes run |
|---|---|
| Public declaration and training data | `DeclarationError` for invalid `.bayes(...)`, `DataPreparationError` for source, schema, target, split, or preparation failures, and `TrainingError::Unsupported` for dense/lifecycle controls that have no observed-categorical meaning. |
| Semantic preparation | `TrainingCompileErrorKind::InvalidNetwork`, `InvalidTargetMatrix`, `EmptyDataset`, `UnsupportedExtent`, or `ArithmeticOverflow` for latent nodes, missing/empty parent lists, target order drift, noncategorical vectors, missing retained observations, invalid codes, mixed-radix overflow, or a histogram domain that is too large. |
| Semantic model load/resume | `CheckpointError::Decode` with path-addressed `LimitExceeded`, `InvalidUtf8`, `InvalidSyntax`, `MissingField`, `DuplicateField`, `UnknownField`, `InvalidValue`, or `InconsistentValue`; `InvalidManifest` for version, smoothing, schema, partition, or child-as-parent drift; and `CheckpointSource` for bounded-file failures. |
| Target-free query preparation | `InferencePreparationError::Data` for query table preparation, `InconsistentCheckpoint` for absent or wrong-schema parent features, and `ArithmeticOverflow` for unrepresentable row or byte dimensions. Unknown query labels are encoded through the saved reserved parent route rather than treated as a preparation failure. |
| Inference graph compilation | `InferenceCompileErrorKind::EmptyDataset`, `InconsistentCheckpoint`, `UnsupportedExtent`, `ArithmeticOverflow`, `IdentityExhausted`, `Language`, `Operation`, `Program`, or `Ogdl`. The `Operation` variant contains the `OperationErrorKind` from this module. |
| Native lifecycle and exit | `InferenceExecutionError` can report preparation or native handoff failure, executor/run failure, invalid external input images, missing or duplicate prediction output, output dtype/size/source mismatch, or failure to reach a terminal state. A graph that reaches this boundary has already passed operation and static-program validation. |

No caller catches one of these failures to select another Bayes algorithm. A
failure remains visible at the boundary that observed it.

## Normative limits and non-goals

The operation implements only observed dictionary-categorical conditionals:

- Every child is an observed declared target and every parent is an observed
  inference feature.
- Every conditional has at least one parent.
- A feature parent may be shared by conditionals, but a target child cannot be
  another conditional's parent in this instrument.
- Missing or unseen query parent labels use the one reserved parent route.
- Numeric distributions, latent state spaces, custom priors, evidence
  propagation, ancestral prediction, marginalization, generic losses,
  optimizers, metrics, and partial training semantics are not implied.

These restrictions are enforced before this module or at its typed request
boundary. The graph contains only calculation primitives and the one output
transfer boundary; discovery, compilation, allocation, native-image loading,
and execution remain pre-loop or runtime concerns outside this source file.

## Source map

| Evidence | Location |
|---|---|
| Request and result types | `ops/src/bayes.rs:24-56` |
| Checked requirements formula | `ops/src/bayes.rs:58-130` |
| Fixed graph emission | `ops/src/bayes.rs:132-231` |
| Append and caller-graph checks | `ops/src/bayes.rs:233-293` |
| Request and boundary validation | `ops/src/bayes.rs:296-421` |
| Resource checks and emitter identity/workspace accounting | `ops/src/bayes.rs:423-675` |
| Scalar programs, aliases, checked arithmetic, error wrapping | `ops/src/bayes.rs:676-863` |
| Public Bayes declaration and model restrictions | `src/api.rs:946-1003`, `src/api.rs:1212-1222`, `src/api.rs:1472-1536`, `src/api.rs:1633-1674` |
| Semantic preparation and observed-row invariants | `src/training.rs:500-580`, `training/src/bayes.rs:260-430`, `training/src/bayes.rs:500-790` |
| Semantic OGDL versions, resume, and decode limits | `training/src/bayes_checkpoint.rs:20-237`, `training/src/bayes_checkpoint.rs:362-790` |
| Target-free table preparation and operation caller | `training/src/inference.rs:711-831`, `training/src/inference.rs:1162-1425` |
| Repeated-output device concatenation | `training/src/inference.rs:1427-1550` |
| Public model dispatch and native lifecycle | `src/inference.rs:490-655` |
| Public Bayes report decoding and row output | `src/inference.rs:306-405`, `src/inference.rs:892-978` |
| Runnable singular and repeated Bayes workflows | `examples/cookbook.rs:398-421` |
| Saved dictionary encoding and reserved query route | `ingest/src/inference.rs:219-227`, `ingest/src/inference.rs:320-461` |
| Primitive validation and backend-neutral lowering obligations | `language/src/primitive.rs:430-729`, `primitives/src/lower.rs:663-712`, `primitives/src/lower.rs:1258-1308`, `primitives/src/lower.rs:1422-1476`, `kernel/src/stage.rs:3010-3065`, `kernel/src/stage.rs:3441-3558` |
| Static one-iteration domain validation | `program/src/lib.rs:39-205`, `training/src/inference.rs:1409-1424`, `training/src/inference.rs:4639-4644` |
| Normative C36 and C42 contracts | `system-contract.md:642-660`, `system-contract.md:781-800` |
