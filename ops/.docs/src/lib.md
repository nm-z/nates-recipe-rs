# recipe-ops::lib

recipe-ops is Recipe's canonical operation inventory and lowering boundary. The
crate turns a source-qualified public operation into one of five explicit
representations: an owned scalar SSA program, a checked direct primitive recipe,
a finite structured composition, a checked static workspace formula, or an
explicit non-calculation lifecycle or host declaration. An operation that has no
one of those representations remains visible as LoweringAvailability::Unsupported
and fails closed. This is an ownership boundary, not a compatibility shim: the
crate never silently selects a legacy implementation or a CPU payload fallback.

The facade itself is deliberately small. ops/src/lib.rs forbids unsafe code and
denies missing Debug implementations, declares every implementation module
privately, and re-exports the public types and entry points from those modules.
The implementation modules own validation, graph construction, formulas,
resource accounting, and error classification. The crate does not execute a
graph, allocate device memory, or choose a backend. It produces typed
recipe-language graphs and recipe-primitives programs for the training and
inference compilers to insert into their static programs.

## Module and export map

All modules are private to keep one stable public surface. The complete re-export
list is in ops/src/lib.rs:31-82.

| Module | Boundary role | Public items re-exported by the facade |
| --- | --- | --- |
| registry | Source-qualified identity, contracts, classification, and lookup | OperationId, OperationDescriptor, OperationRegistry, operation_registry, the Recipe-owned pool descriptor functions, OperationFamily, LoweringAvailability, UnsupportedReason, CanonicalDTypeContract, LegacyDType, AliasContract, DeterminismContract |
| error | One typed error channel for every operation boundary | OperationErrorKind, OperationError, OperationResult |
| scalar | Owned elementwise scalar programs and canonical model formulas | ScalarRecipe, CompositeScalar, lower_scalar, focal constants, and the canonical activation/objective builders |
| primitive | Direct non-elementwise primitive contracts and lowering | PrimitiveFamily, PrimitiveRecipe, PrimitiveRequest, AxisRequirement, ContractionClass, RandomRecipe, lower_primitive, lower_index_map |
| composition | Static algorithm-shape recipes | CompositionPayload, CompositionRecipe, CompositionStep, IterationBound, validate_composition |
| materialize | Shape- and parameter-bound composition to a concrete graph | NamedTensor, PreparedParameter, PreparedParameters, IdentityNamespace, request and resolved-step types, MaterializedComposition, workspace allocation types, identity validation, expansion, materialization, and the remaining-composition manifest |
| workspace | Checked scratch/resource formulas | WorkspaceFormula, WorkspaceUnit, WorkspaceValue, evaluate_workspace |
| non_calculation | Explicit host, metadata, and lifecycle entries | NonCalculationRecipe |
| convolution | Immutable channelwise 1-D convolution index preparation | ChannelwiseConvolution1dPreparation, prepare_channelwise_convolution_1d |
| pooling | Immutable channelwise non-overlapping max-pool preparation | ChannelwiseMaxPool1dPreparation, prepare_channelwise_max_pool_1d |
| kmeans | K-means initialization and one Lloyd transition | request, requirement, materialization types, requirement functions, and materializers |
| knn_outputs | One graph for all numeric and categorical KNN outputs | output specs and requests, requirement and materialization types, requirement/materialization/append functions |
| bayes | Observed categorical Bayesian inference fragment | request, requirement, materialization types, requirement/materialization/append functions |
| binary_metrics | GPU-only binary metrics fragment | request, threshold type, requirement/materialization types, limits, requirement/materialization/append functions |
| tree | Saved complete-binary-tree inference fragment | request, requirement/materialization types, requirement/materialization/append functions |

The facade also re-exports recipe_primitives::LoweredProgram and
recipe_primitives::LoweringHardware, so callers can pass a direct primitive
request and receive the backend-neutral lowered program without depending on an
implementation module.

## Inventory generation and registry identity

ops/build.rs is the only generated-inventory step. It watches
../operation-surface.txt, ignores blank and comment lines, and requires exactly
two non-empty tab-separated fields per row: the public symbol and its legacy
source. For each parsed row it records the source line, stable zero-based ordinal,
occurrence number, and total occurrence count for that symbol. The generated
operation_surface.rs contains RAW_OPERATION_SURFACE and RAW_OPERATION_COUNT; the
registry includes it at registry.rs:208.

The current source file contains 421 parsed rows, 415 unique symbols, and four
symbols with duplicate source-qualified entries. The registry then appends two
Recipe-owned extensions that are intentionally outside the legacy file:

- recipe_max_pool_1d, sourced from ops/src/pooling.rs:channelwise_max_pool_1d;
- recipe_max_pool_1d_backward, sourced from
  ops/src/pooling.rs:channelwise_max_pool_1d_backward.

Consequently, operation_registry().surface_len() is 421, owned_len() is 2, and
len() is 423. Legacy ordinals remain unchanged because owned extensions are
appended after the generated prefix. An owned descriptor has
OperationId::surface_line() == 0; a legacy descriptor retains its source line.
OperationId::occurrence() and occurrences() preserve duplicate-symbol identity,
while is_duplicate_symbol() and is_recipe_owned() expose the two important
distinctions without exposing mutable registry state.

OperationRegistry is a zero-sized, Copy registry handle. Its behavior is purely
over immutable static data:

- iter() returns all 423 descriptors in canonical order.
- surface_iter() returns only the generated legacy prefix.
- owned_iter() returns only the two Recipe-owned extensions.
- named(symbol) yields every source-qualified descriptor with that symbol.
- resolve_unique(symbol) returns the sole match, or UnknownOperation when absent
  and AmbiguousSymbol when more than one source row exists.
- resolve_exact(symbol, source) requires exactly one matching pair and uses
  UnknownOperation or AmbiguousSymbol for the corresponding failures.

describe derives a complete OperationDescriptor from one raw entry. The
descriptor carries the identity, symbol, source, operation family, canonical
dtype contract, lowering availability, human-readable definition, alias contract,
determinism contract, and optional explicitly excluded legacy dtype. The
classification order is significant and fail-closed:

1. ScalarRecipe::for_symbol claims owned scalar elementwise formulas.
2. PrimitiveRecipe::for_symbol claims direct non-elementwise recipes.
3. WorkspaceFormula::for_symbol claims static scratch queries.
4. NonCalculationRecipe::for_entry claims declarations, parsing, encoding, chat
   rendering, shutdown, and the eliminated vendor workspace setter.
5. CompositionRecipe::for_entry(symbol, source) claims a source-qualified finite
   composition.
6. Explicit legacy dtype markers become Unsupported(LegacyDTypeExcluded).
7. convert and gpu_convert become Unsupported(DynamicFormatConversionPending).
8. Non-gpu entries without an earlier owner become
   Unsupported(HostBehaviorPending).
9. Remaining gpu entries become
   Unsupported(DedicatedPrimitiveCompositionPending).

The dtype classifier follows the chosen owner. Scalar and direct primitive
recipes expose exact input and output dtypes. Compositions expose a payload
domain (F32, I32, both, or either), workspace and non-calculation entries are
non-numeric, and unowned GPU symbols default to the canonical f32 payload unless
their name identifies an f32 and int32 payload. Explicit _f16, _f64, _u8, and
the legacy quantized gpu_convert entry are retained as metadata and never
authorize those payload paths. Family classification similarly follows an owned
recipe first, then source and symbol categories such as parsing, encoding,
inference, optimizer, loss, attention, linear algebra, reductions, graph,
clustering, Bayesian, tree, diffusion, sequence, and state-space families.

Alias and determinism are descriptive contracts carried by the descriptor. Known
in-place symbols require the output to alias the specified input; _into entries
default to no alias, other GPU entries remain operation-specific, and non-GPU
entries default to no alias. Random, bootstrap, scatter, and histogram symbols
identify counter-based random or explicit atomic policies. Scalar recipes are
per-element exact order, direct primitives and compositions are fixed primitive
order, non-calculation entries are host deterministic, and unsupported entries
retain a pending definition unless the pending behavior is explicitly host-side.

## Errors and closed failure boundaries

OperationErrorKind is the single vocabulary used by the crate. Its variants
cover lookup (UnknownOperation, AmbiguousSymbol), ownership and lowering
(UnsupportedLowering, WrongLoweringKind, PrimitiveRecipeMismatch,
InvalidScalarProgram, PrimitiveLoweringFailed), composition and preparation
(InvalidCompositionRecipe, InvalidMaterializationRequest,
MissingPreparedParameter, PreparedParameterTypeMismatch, IterationBoundUnresolved,
CompositionExpansionOverflow, MissingConcreteFormula, UnsupportedConcreteShape),
identity and resources (IdentityNamespaceOverlap, IdentityNamespaceExhausted,
WorkspaceLimitExceeded), graph consistency (GraphMaterializationFailed), and
workspace accounting (WorkspaceFormulaMismatch, WorkspaceArithmeticOverflow).

OperationError contains the kind, a detailed string, and an optional
OperationId. new starts an unbound error, while for_operation attaches the
descriptor identity as the error crosses a public lowering boundary. Display
prints the kind and detail, followed by the operation ordinal when present. All
public operations return OperationResult<T>, so a missing implementation,
invalid shape, identity collision, or arithmetic overflow is observable at the
boundary instead of being replaced by a fallback.

## Scalar lowering

ScalarRecipe is the registry-owned description of one elementwise calculation:

- Opcode names one ScalarOpcode and its exact input dtypes.
- Math embeds one recipe_math::MathFunction; its arity determines the f32 input
  contract and its owned algorithm is converted into a scalar program.
- Composite selects a multi-instruction formula from CompositeScalar.

The symbol table in scalar.rs:79-164 maps the public GPU symbols to these
recipes. It covers typed arithmetic and comparison, stable math functions,
reverse and in-place argument order, copy and fill identity, dropout, ReLU and
backward, leaky ReLU and PReLU helpers, ELU and SELU, sigmoid/tanh/SiLU/GELU
families, GLU variants, scaled exponential, reparameterization, KL terms,
binary cross-entropy gradients, MAE and Huber gradients, and SGD updates. The
table is intentionally many-to-one where legacy symbols have identical owned
semantics, while the descriptor still preserves each source-qualified identity.

lower_scalar(descriptor) first requires LoweringAvailability::Scalar. A
primitive, composition, workspace, or non-calculation descriptor returns
WrongLoweringKind; an unsupported descriptor returns UnsupportedLowering with its
definition and reason. Opcode recipes are assembled by the local Composer, math
recipes come from ScalarProgram::try_from, and composite recipes are expanded
into the same SSA identity space. The finished ScalarProgram is validated before
it is returned. The composer checks every opcode's input dtypes, allocates
checked scalar value identities, inlines math programs without leaving a foreign
value namespace, and rejects unknown values, arity mismatches, and identity-space
exhaustion.

The public canonical builders provide formulas used by model compilation even
when a legacy operation symbol is not the right lookup surface:

- canonical_leaky_relu_program and its backward form use fixed alpha 0.01.
- canonical_prelu_program accepts a learned scalar; its backward form emits the
  input gradient and per-element learned-alpha contribution for a later
  reduction.
- canonical_elu_program and its backward form use alpha 1.0.
- canonical_selu_program and its backward form embed the canonical SELU alpha and
  lambda constants.
- canonical_focal_with_logits_program validates finite binary targets and emits
  loss and gradient directly from logits, with the public fixed constants
  FOCAL_ALPHA = 0.25 and FOCAL_GAMMA = 2.0.

The internal binary-cross-entropy-with-logits builder validates targets in [0, 1],
uses the owned stable softplus and sigmoid math, and returns both loss and
gradient. It is consumed by the concrete training materializer rather than
being exposed as a separate facade method.

## Direct primitive lowering

PrimitiveRecipe describes a non-elementwise operation while leaving
shape-dependent data in the caller's immutable PrimitiveKernel. The public
PrimitiveRequest borrows that kernel and a BTreeMap<ValueId, &Tensor>, so the
lowerer does not copy or own graph state. Recipe categories are reductions,
scans, contractions, gathers, scatter-add, sorts, index maps, and counter-based
random maps. PrimitiveFamily maps those categories to reduction, scan,
contraction, shape/indexing, random, and related operation families.

The current direct symbol table contains argmax/argmin, batched and transposed
matrix contractions, cumulative and prefix scans, vector and matrix dot/gemm,
row gathers, all-value reductions, scatter-add, sort, and uniform/normal random
maps. lower_index_map is the Recipe-owned extension path for an
iteration-aware affine int32 index source and intentionally has no legacy
registry symbol.

lower_primitive requires LoweringAvailability::Primitive, then checks the recipe
against the concrete kernel before calling recipe_primitives::lower with the
requested LoweringHardware. The checks preserve the semantic invariants that
are otherwise easy to lose in a generic lowering:

- axis requirements are Any, first, last, or all and are checked against the rank
  of the first input tensor;
- reductions require the declared operator and result shape (f32, i32, or value
  plus index);
- scans require the declared inclusive or exclusive mode, a valid identity,
  forward direction, and the expected axis;
- contractions distinguish vector, matrix, left-transposed, right-transposed,
  and batched layouts by input rank and contracted axes;
- gathers and scatters require IndexBounds::Reject;
- scatter-add additionally requires sequentially consistent atomic add,
  exact output-to-input aliasing, and the declared axis;
- sorts require the declared direction and stability and must not emit indexes;
- random recipes require the matching uniform or normal distribution;
- every mismatched primitive kind returns PrimitiveRecipeMismatch before the
  backend-neutral lowerer is called.

The lowerer therefore accepts only a concrete kernel whose kind, axis, bounds,
alias policy, and dtype contract match the descriptor. A backend failure is
wrapped as PrimitiveLoweringFailed and retains the descriptor operation ID.

## Structured composition recipes

CompositionRecipe is an algorithm-shape contract, not executable tensor
semantics. Its CompositionPayload records the canonical payload domain;
CompositionStep is either one primitive family with a role string or a repeat
with an IterationBound; and CompositionRecipe stores a stable name, definition,
immutable steps, payload, and operation family.

The supported bounds are fixed values, a selected input shape extent, the minimum
shape extent, a ceiling-log2 shape extent, and a named prepared parameter. The
recipe validator requires nonempty names, definitions, roles, and step lists;
rejects zero fixed repeats and empty repeat bodies; rejects empty prepared names;
and limits nesting to eight levels. It does not inspect tensors because wiring is
the materializer's responsibility.

composition_for_entry(symbol, source) owns a large source-qualified mapping. The
shared recipes cover checked dequantization, bounded generation and greedy
selection, metrics, losses, optimizers, normalization, attention, embedding,
convolution and pooling, indexing and sorting, reductions and scans, FFT and
linear algebra solvers, graph and clustering algorithms, reinforcement learning,
tree and boosting operations, quantization, diffusion, sequence decoding,
state-space scans, and support-vector-machine training. Source-qualified dispatch
is strict: a matching symbol from another legacy source does not inherit a nearby
recipe. The source-specific tail handles the tree-library predict and train
families, plus the inference greedy and last_logits entries.

The step constants make the intended dataflow explicit. Examples include
map-reduce-map for accuracy and statistics, gather-map for shape and embedding
operations, map-sort-gather for bounded top-k, gather-contract-scatter for
convolution and pooling backward, fixed-tree softmax stages, bounded radix-2 FFT
repeats, shape-bounded triangular/LU/QR repeats, prepared Jacobi and SVD sweeps,
and prepared iteration counts for generation, dynamic programming, boosting, and
SMO. These descriptions cannot by themselves create a graph. They become
executable only when a concrete materializer supplies exact tensor names,
prepared facts, scalar programs, primitive parameters, and workspace policy.

## Composition expansion and graph materialization

materialize.rs is the common preparation boundary for structured operations. The
public request is immutable and contains:

- the source-qualified OperationDescriptor;
- named input and output Tensor declarations;
- the name of the input whose shape resolves shape-dependent bounds;
- a typed PreparedParameters map of U64, I32, F32Bits, or Bool values;
- a caller-reserved IdentityNamespace for intermediate ValueId and
  KernelTemplateId ranges; and
- a ByteCount workspace limit.

NamedTensor keeps the ABI name and borrowed tensor together. Inputs must be
external inputs, outputs must be non-input external outputs, all names and tensor
IDs must be unique, every tensor must validate, and at least one input and output
must be present. require_exact_abi then lets each concrete materializer reject
missing or additional tensor and parameter names rather than guessing a shape.

validate_identity_namespaces checks every pair of caller reservations before
fragments are assembled. Value and kernel ranges are half-open and independently
checked for overlap. identity_ranges checks range-end arithmetic, and
GraphBuilder::new rejects a declared boundary tensor that falls inside the
reserved intermediate range. Every emitted intermediate is contiguous, counted
against the workspace limit, and allocated from that range. Every emitted kernel
uses the reserved kernel range, receives forbidden input/output aliases by
default, and is checked for range exhaustion.

expand_composition performs the first half of preparation. It requires a
composition descriptor, validates its static recipe, resolves every bound from
the selected input shape or prepared parameter, records ResolvedBound values,
and recursively unrolls repeats into ResolvedStep values. Each primitive step
records its ordinal, family, role, surrounding repeat indexes, and the preceding
step dependency. The expansion has a hard one-million primitive-step limit and
returns IterationBoundUnresolved for an invalid shape axis, empty shape, or
missing/wrongly typed prepared value.

materialize_composition then:

1. validates the request and requires a composition descriptor;
2. rejects recipes that lack a concrete source-qualified materializer with
   MissingConcreteFormula;
3. resolves the selected iteration-shape input;
4. expands the finite recipe;
5. creates an Emitter and GraphBuilder with the caller namespace;
6. dispatches to concrete family modules in a fixed order;
7. checks that each emitted kernel's primitive family matches the resolved step;
8. checks that the emitter consumed exactly every resolved step; and
9. validates the resulting CalculationGraph and returns its graph, resolved steps,
   stage-to-kernel map, workspace allocation, and identity namespace.

The concrete dispatch order is optimizer and normalization, solver and FFT,
attention/sequence/embedding, convolution/pooling, loss/metrics,
indexing/sort/encoding, graph/clustering/reinforcement learning, tree/boosting,
inference/quantization/diffusion, creation/shape/miscellaneous, and training.
Each family module owns an exact (symbol, source) table and returns
FamilyDispatch::NotOwned for every other descriptor. The current
inference_quantization_diffusion and creation_shape_misc modules are explicit
closed stubs with supports == false, so recipes in those categories remain in
remaining_composition_manifest() until their concrete tensor ABI and formulas
exist. A family claiming a descriptor but failing to emit the expected primitive
family returns GraphMaterializationFailed, not a substitute implementation.

The materialized result is intentionally inspectable without exposing builder
mutability. ResolvedComposition exposes bounds and steps, StageEmission maps a
resolved step to concrete kernel IDs, WorkspaceAllocation exposes every
intermediate object and total bytes, and MaterializedComposition::graph() returns
the validated immutable graph. The manifest reports every source-qualified
composition that has a descriptive recipe but no concrete family materializer,
together with the recipe name and the missing component set (TensorAbi,
ScalarFormula, PrimitiveParameters, and WorkspacePolicy).

## Workspace formulas

WorkspaceFormula describes static scratch resources without pretending that a
workspace query is a calculation kernel. It includes no-persistent-scratch,
fixed-tree reduction and scan, stable sort, sort-run encoding, map-then-reduce,
random-key sort, Cholesky, LU, QR, symmetric eigensolver, SVD, and split-K partial
formulas. WorkspaceValue carries an amount and a unit, either bytes or f32
elements; bytes() is available only for byte-valued formulas.

WorkspaceFormula::evaluate requires the exact dimension count for the selected
formula and performs all additions, multiplications, padding, tree-level counts,
panel sizing, fault-word accounting, and split-K clamping with checked u64
arithmetic. The fixed reduction tree uses 64 lanes, solver panels use 32
columns, and split-K uses at most eight 256-row partitions. A dimension mismatch
returns WorkspaceFormulaMismatch, arithmetic overflow returns
WorkspaceArithmeticOverflow, and evaluate_workspace adds the descriptor identity.
Calling it for a scalar, primitive, composition, or non-calculation descriptor
returns WrongLoweringKind.

## Explicit non-calculation entries

NonCalculationRecipe prevents host or lifecycle behavior from being mistaken for
a payload operation. Its variants are facade declaration, text tokenization,
model-container parsing, chat-template rendering, run shutdown, and the
eliminated vendor workspace binding. They carry a human-readable definition and
operation family, remain deterministic host behavior, and never authorize a CPU
arithmetic fallback. The registry therefore keeps these public symbols visible
while making their non-calculation boundary explicit.

## Specialized graph builders

The six larger public builders in this crate are dependency-clean graph
materializers. They share the same invariants as materialize.rs: validate every
boundary tensor, reserve independent identity ranges, use checked f32 and int32
payloads, emit explicit PrimitiveKind nodes with alias rules, count every
intermediate and kernel, enforce the caller workspace limit, and validate the
finished CalculationGraph. Their append_* functions additionally verify that the
caller graph already contains every boundary tensor with the same ID, dtype,
shape, layout, and storage bytes, then reject intermediate or kernel ID overlap
and duplicate output production before extending the graph.

### Channelwise convolution preparation

prepare_channelwise_convolution_1d(batch, input_length, input_channels, filters,
kernel_size) accepts only positive dimensions with kernel_size <= input_length
and products that fit the checked int32 index domain and host usize. It records
output_length = input_length - kernel_size + 1, flat input and output sizes,
forward receptive-field indices in [batch, output_length, kernel_size,
input_channels] order, and backward input-gradient indices plus a matching f32
zero/one validity image. It reports the flat input/output/window shapes and exact
forward and backward workspace byte counts. Overflow or an invalid geometry is
UnsupportedConcreteShape.

The training and inference compilers use this preparation to admit immutable
window-index payloads, gather a flat input, contract it with f32 filters, add the
f32 bias, and build the backward scatter path. Preparation is host metadata; the
payload calculation remains in the resulting graph.

### Channelwise non-overlapping max pool

prepare_channelwise_max_pool_1d(batch, input_length, channels, pool_size) records
groups = ceil(input_length / pool_size) and a rectangular
window_width = min(input_length, pool_size). A final short window is represented
by repeating its last valid coordinate. The preparation exposes flat input and
output shapes, window indices, output winner bases, identity batch indices, and
forward/backward workspace bytes. forward_parameters(tree_lanes) and
backward_parameters() produce the exact typed facts required by the two
Recipe-owned descriptors, including non-overlap, tail repetition, unique winner,
and zero-base guarantees. Positive dimensions, checked products, int32
coordinates, and host allocation sizes are enforced before any graph is emitted.

### K-means

KMeansInitializationRequest copies deterministic source rows into the initial
centroid tensor with an affine IndexMap using row % rows, followed by a checked
gather. kmeans_initialization_requirements fixes one intermediate and two
kernels, and charges four bytes per centroid element.

KMeansLloydRequest describes one complete Lloyd transition. It validates rank-two
f32 point and centroid matrices, nonzero dimensions, a row count within the
exact-f32 integer domain, distinct output identities, and a power-of-two
tree_lanes value in 1..=1024. Its graph squares points and prior centroids,
reduces norms, contracts point/centroid products, forms rooted L2 distances,
selects lowest-index minimum assignments, builds membership indicators, computes
cluster sums and counts, preserves prior centroids for empty clusters, and
recomputes distances against the updated centroids. Requirement functions fix 16
intermediate values and 18 kernels for the transition and derive workspace from
the row, feature, cluster, and assignment images. The emitter checks exact
counts, exact workspace bytes, reserved identities, alias policy, and final graph
validation.

### All-output KNN

KnnOutputSpec distinguishes numeric mean outputs from categorical mode outputs;
KnnOutputRequest binds each semantic output to reference values or canonical int32
codes, a known-reference mask, and its prediction tensor. Each output has an
independent mask and count. knn_all_output_requirements validates nonzero
query/reference/dimension/neighbor domains, reference rows in the int32 sort
domain, known counts in 1..=reference_rows, exact f32 numeric neighbor counts,
and categorical class counts and histogram sizes. It computes per-output
effective neighbors as min(neighbors, known_references) and returns exact value,
kernel, and workspace reservations.

materialize_knn_all_outputs squares normalized f32 query and reference matrices,
reduces norms, forms rooted L2 distances, validates each known mask on-device,
masks unknown rows to positive infinity, performs a stable ascending sort with
reference-row ties, gathers the prepared neighbor prefix, and independently
aggregates each output. Numeric outputs reduce a f32 mean. Categorical outputs
build row/class histogram bins, stable-sort counts descending, and select the
lowest class code on ties. Finiteness, categorical-code ranges, reject-bounds
gathers, exact count checks, namespace capacity, and final resource totals all
remain graph-visible validation steps. append_knn_all_outputs inserts only the
new fragment into the caller graph.

### Observed categorical Bayesian inference

CategoricalBayesInferenceRequest binds reference parent and child codes, query
parent codes, parent multipliers and cardinalities, the f32 posterior output,
reference/query dimensions, parent configuration and child-class counts,
positive finite Laplace smoothing, tree lanes, namespace, and workspace limit.
The parent cardinalities include a reserved unseen route, so an unseen query
configuration receives the uniform smoothed posterior; child codes do not have a
reserved class.

The materializer validates exact int32 and f32 tensor shapes, checked histogram
dimensions, positive finite smoothing, and power-of-two reduction lanes. It
packs each reference parent row, combines the packed configuration with a child
code into a joint histogram bin, packs query configurations, gathers selected
counts, reduces class totals, and emits
(count + smoothing) / (total + smoothing * child_classes) into the probability
matrix. The scalar programs require all codes to be within their declared ranges.
Resource requirements are exactly ten intermediates and eleven kernels plus the
checked reference/query/histogram workspace. The append path protects boundary
contracts, IDs, and duplicate probability producers.

### Binary classification metrics

BinaryClassificationMetricRequest consumes probabilities, binary targets, and
the already-emitted per-element BCE vector, and writes mean BCE, AUROC, AUPRC,
Brier score, expected calibration error, and a caller-selected set of
RecallAtOutput scalars. Population, recall threshold, and calibration-bin limits
are explicit: at most 9,999,999 examples, 256 thresholds, and 256 bins.

The fragment first performs one guarded device copy that validates finite
probabilities, binary targets, and finite nonnegative losses. It reduces mean BCE
and Brier score, then stable-sorts probabilities descending and gathers targets.
Explicit group starts, scans, histograms, and fixed-order reductions produce
tie-aware non-interpolated AUROC and AUPRC. Each recall threshold counts hits and
requires a positive population. Equal-width calibration bins over [0, 1] emit
confidence and target sums, then a fixed scalar tree computes expected
calibration error. Exact requirements are computed before identity reservation,
and the emitter checks all value, kernel, and byte totals before graph validation.

### Saved tree ensemble inference

TreeEnsembleInferenceRequest binds flattened f32 features, tree-major int32
split features, f32 split thresholds, tree-major f32 leaf values, f32 predictions,
the row/feature/tree/depth/output dimensions, finite scale, tree lanes, namespace,
and workspace limit. Requirements accept complete binary trees with depth 1..=30,
nonzero dimensions, and all flattened images in the int32 domain.

The graph starts with row/tree pair indexes and follows each tree depth using
checked split-index and feature-index scalar maps, gathers selected features and
thresholds, and sends exact threshold ties left. It computes tree-major leaf
bases, gathers every output contribution, reduces trees in fixed order, and
applies the finite scale. Requirement counts are fixed plus six intermediates and
six kernels per depth, with checked workspace for pair images and expanded
contributions. The append path rejects boundary contract changes, ID overlap, and
duplicate prediction producers.

## Real caller path

The root crate exposes this crate in two ways. src/facade.rs:17-41 re-exports
recipe_ops as recipe::engine::ops for advanced callers. Its public
recipe::operations module (src/facade.rs:51-124) re-exports the dependency-clean
operation types and forwards registry, unique/exact resolution, scalar and
primitive lowering, composition validation/materialization,
remaining-manifest, and workspace evaluation calls one-for-one. The root facade
adds no alternate operation implementation and does not infer domain state.

The production training and inference compilers depend directly on recipe-ops:

- Their emit_owned_scalar helpers resolve a symbol with
  operation_registry().resolve_unique, call lower_scalar, and emit the returned
  ScalarProgram as a PrimitiveKind::Elementwise node. This is the complete scalar
  path used by both compilers.
- Their materialize helpers turn named ValueId pairs into NamedTensor declarations,
  mark input/output boundary flags, reserve a fixed identity namespace, resolve
  the exact descriptor, and call materialize_composition. They insert the
  returned graph tensors and nodes and attach each kernel to the caller's
  iteration domain.
- Training calls the specialized binary-metric, K-means, tree, convolution, and
  pooling requirement and materialization functions while compiling the static
  training graph. K-means reserves separate namespaces for deterministic
  initialization and each Lloyd transition. Binary metrics reserve exactly the
  resources returned by binary_metric_requirements before the fragment is
  appended.
- Inference calls append_categorical_bayes_inference and append_knn_all_outputs
  after constructing the caller graph's boundary tensors. It calls the tree,
  convolution, and pooling preparation APIs while lowering saved model state,
  then converts the completed graph through the normal recipe-language and
  recipe-program static-program path.

In every case the end-to-end dataflow is declaration or prepared model state,
then registry identity and owned lowering, then a validated graph fragment, then
the training or inference compiler's static program and native backend lowerer.
recipe-ops stops at the graph/program boundary. Hardware discovery, allocation,
queueing, execution, output readback, and lifecycle transitions belong to the
downstream crates described by the root facade.

## Invariants that callers can rely on

The implementation consistently preserves the following observable contracts:

- Source-qualified identity is never collapsed when a symbol is duplicated.
- Canonical calculation payloads are f32 and int32; excluded legacy payload
  types are metadata, not executable alternatives.
- Every index operation uses checked int32 coordinates and reject bounds unless a
  concrete operation explicitly requires a different preparation-time policy.
- Every reduction and scan carries explicit axes, tree width, result shape, and
  deterministic order.
- Every scatter, histogram, and atomic update declares its conflict and ordering
  policy; no implicit race behavior is inferred.
- Every graph fragment owns a caller-reserved value and kernel namespace, forbids
  accidental boundary aliases, accounts for every intermediate byte, and is
  validated before return.
- Every composition repeat is resolved before execution, so the final graph has
  no host payload loop or callback.
- Unsupported symbols remain enumerable and fail at the earliest boundary with a
  typed error. A descriptive recipe without a concrete ABI cannot cross
  materialize_composition.

These invariants are why the registry, lowerers, preparers, and materializers are
kept behind one facade. The public surface describes exactly what is owned today,
the resulting graph makes all state and resource decisions explicit, and the
downstream compiler can execute only the validated calculation and transfer nodes
it received.

## Validation status

The current crate builds successfully with:

    cargo check -p recipe-ops

That command verifies the live Rust facade, generated operation inventory, all
private modules, and their cross-crate type contracts. It is structural evidence
only. Runtime correctness remains the responsibility of the training and
inference end-to-end paths, which must execute the resulting static graph on the
required real data and hardware.
