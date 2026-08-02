# Operation registry

The registry is the immutable compatibility inventory and lowering boundary for
Recipe's finite public operation surface. It does not discover operations at
runtime, mutate a global table, or invoke a legacy implementation. It turns one
source-qualified inventory row into an `OperationDescriptor` that records the
canonical payload contract, operation family, alias and determinism policy,
and the exact owned lowering (or an explicit fail-closed reason).

The registry is implemented in `ops/src/registry.rs`. `recipe-ops` re-exports
its public types from `ops/src/lib.rs`; the root crate exposes the same boundary
through `recipe::operations` in `src/facade.rs`. Training and inference use the
registry while compiling their calculation graphs. Once a scalar program,
primitive program, workspace value, or materialized graph has been emitted, the
planner and executors consume that result and do not perform another operation
registry lookup.

## Normative input and generated source

`operation-surface.txt` is the preserved C13 compatibility inventory. Its
header says that a row remains until its public behavior is mapped to an owned
f32/int32 scalar program or kernel template and routed through the clean facade.
The file is the source of legacy order and source-qualified identity, not an
executable implementation.

`ops/build.rs` is the only generator. It emits
`$OUT_DIR/operation_surface.rs`, which `registry.rs` includes at compile time:

1. Cargo is told to rerun the generator when `../operation-surface.txt` changes.
2. The file is read as UTF-8 text. `source.lines()` supplies one-based physical
   line numbers; trailing whitespace is removed, but leading whitespace is not.
3. Empty lines and lines whose first remaining character is `#` are skipped.
4. Every remaining row must contain exactly two tab-separated, non-empty fields:
   `symbol` and `source`. Missing fields, extra fields, or empty fields panic
   the build script with the row number.
5. A `BTreeMap<String, u16>` counts every occurrence of each symbol. A second
   map records the occurrence number as rows are emitted. The checked `u16`
   increments make an overlarge symbol population a build failure.
6. Each parsed row becomes a `RawSurfaceEntry` with its zero-based canonical
   ordinal, physical source line, string fields, one-based occurrence, and
   total occurrence count. `RAW_OPERATION_COUNT` is the number of parsed rows.
7. The generated file contains the immutable `RAW_OPERATION_SURFACE` slice and
   `RAW_OPERATION_COUNT`. It is not checked into the repository.

At this checkout the input contains 421 operation rows on 431 physical lines
and 415 distinct symbols. Four symbols intentionally have multiple source
entries:

| Symbol | Entries | Sources |
| --- | ---: | --- |
| `predict` | 3 | `catboost-rs/src/lib.rs:811`, `lightgbm-rs/src/lib.rs:776`, `xgboost-rs-broken/src/lib.rs:367` |
| `predict_proba` | 2 | `lightgbm-rs/src/lib.rs:928`, `xgboost-rs-broken/src/lib.rs:504` |
| `train` | 3 | `catboost-rs/src/lib.rs:594`, `lightgbm-rs/src/lib.rs:696`, `xgboost-rs-broken/src/lib.rs:288` |
| `train_multiclass` | 2 | `lightgbm-rs/src/lib.rs:833`, `xgboost-rs-broken/src/lib.rs:388` |

The duplicate rows remain distinct descriptors. They are not deduplicated by
the generator or by the registry.

## Recipe-owned extension segment

The generated legacy prefix is followed by the manually maintained
`RECIPE_OWNED_OPERATIONS` slice in `registry.rs`. It currently contains exactly
two rows:

| Ordinal | Symbol | Source | Surface line |
| ---: | --- | --- | ---: |
| 421 | `recipe_max_pool_1d` | `ops/src/pooling.rs:channelwise_max_pool_1d` | 0 |
| 422 | `recipe_max_pool_1d_backward` | `ops/src/pooling.rs:channelwise_max_pool_1d_backward` | 0 |

Their ordinals are `RAW_OPERATION_COUNT` and `RAW_OPERATION_COUNT + 1`.
`surface_line == 0` is the identity marker for this extension segment, so
`OperationId::is_recipe_owned()` is true only for these entries. The two public
descriptor helpers index this fixed slice, and `owned_iter()` exposes the same
descriptors. The current extension symbols do not collide with the legacy
prefix. The code does not calculate cross-segment occurrence counts, so keeping
that uniqueness is an invariant of the maintained extension list.

The resulting current registry census is:

| Measure | Value |
| --- | ---: |
| `surface_len()` | 421 |
| `owned_len()` | 2 |
| `len()` | 423 |
| Distinct symbol names including extensions | 417 |
| Descriptors with duplicate-symbol identity | 10 |

## Identity and descriptor data

`OperationRegistry` is a zero-sized, `Copy`, `Clone`, `Default` value. It
contains no state. Every method derives its answer from the two static slices
and the pure classification functions below. `OperationDescriptor` is also
`Copy`; all strings and dtype slices are `'static` references.

### `OperationId`

An ID contains:

- `ordinal`: zero-based position in the complete canonical registry;
- `surface_line`: one-based physical line in `operation-surface.txt`, or zero
  for a Recipe-owned extension;
- `occurrence`: this symbol's one-based position among source-surface rows; and
- `occurrences`: the total count for that symbol in the generated surface.

The type derives ordering and hashing, so it can be used as a stable key in
maps, graph metadata, and operation-attached errors. `is_duplicate_symbol()` is
`occurrences > 1`; `is_recipe_owned()` is `surface_line == 0`. For the current
extension rows, `occurrence == occurrences == 1` by construction.

### `OperationDescriptor`

Each descriptor has these public fields:

| Field | Meaning |
| --- | --- |
| `id` | Stable canonical identity and error provenance. |
| `symbol` | Public operation name, preserved verbatim. |
| `source` | Legacy source-qualified origin, or the Recipe-owned source label. |
| `family` | Registry category used for inventory and composition dispatch metadata. |
| `dtypes` | Canonical f32/int32 payload or non-calculation contract. |
| `lowering` | Scalar, primitive, composition, workspace, non-calculation, or unsupported ownership. |
| `definition` | Human-readable definition delegated from the selected lowering. |
| `alias` | Output/input alias contract classification. |
| `determinism` | Ordering, random-key, atomic, or host determinism contract. |
| `legacy_dtype` | Optional compatibility marker. It never authorizes the legacy payload type. |

`LoweringAvailability::definition()` is the single definition source for an
owned recipe. Unsupported reasons have explicit text instead of an empty or
optimistic definition:

- `DedicatedPrimitiveCompositionPending`: a GPU surface entry still needs a
  dedicated owned primitive composition;
- `HostBehaviorPending`: host behavior still needs a dependency-clean facade;
- `LegacyDTypeExcluded`: a legacy non-f32/int32 calculation path is excluded;
- `LifecycleOperation`: lifecycle behavior is tracked but is not a calculation;
- `WorkspaceQuery`: a static scratch query still needs its owned formula; and
- `DynamicFormatConversionPending`: dynamic format conversion still needs an
  explicit f32/int32 ingestion recipe.

The enum is deliberately broader than the current census. No current row is
`Unsupported`; the variants are the fail-closed representation for a future
unclassified row or an explicitly excluded path.

## Descriptor construction and classification

`describe(raw)` computes the lowering once, then constructs every other field
from that lowering and the raw symbol/source. There is no registration side
effect and no cache. The classification order is significant:

| Order | Probe | Result |
| ---: | --- | --- |
| 1 | `ScalarRecipe::for_symbol(symbol)` | `LoweringAvailability::Scalar` |
| 2 | `PrimitiveRecipe::for_symbol(symbol)` | `LoweringAvailability::Primitive` |
| 3 | `WorkspaceFormula::for_symbol(symbol)` | `LoweringAvailability::Workspace` |
| 4 | `NonCalculationRecipe::for_entry(symbol, source)` | `LoweringAvailability::NonCalculation` |
| 5 | `CompositionRecipe::for_entry(symbol, source)` | `LoweringAvailability::Composition` |
| 6 | `explicit_legacy_dtype(symbol, source)` | `Unsupported(LegacyDTypeExcluded)` |
| 7 | `symbol == "gpu_convert" || symbol == "convert"` | `Unsupported(DynamicFormatConversionPending)` |
| 8 | symbol does not start with `gpu_` | `Unsupported(HostBehaviorPending)` |
| 9 | remaining GPU symbol | `Unsupported(DedicatedPrimitiveCompositionPending)` |

The first match wins. This matters for names that carry compatibility markers:
`gpu_add_f16`, `gpu_gelu_f16`, `gpu_mul_f16`, `gpu_relu_f16`, the f64-named
entries, and the u8-named entries use owned canonical recipes when those recipes
exist. `convert`, `dequant_f32`, and `gpu_convert` are currently matched by the
composition table before the legacy or dynamic-format fallbacks.

### Canonical dtype contract

`dtype_contract(symbol, source, lowering)` follows this order:

1. Scalar and primitive recipes provide their exact input and output `DType`
   slices. Scalar comparisons return an `I32` output; scalar arithmetic returns
   `F32`.
2. A composition delegates to its `CompositionPayload` (`F32`, `I32`,
   `F32AndI32`, or `F32OrI32`).
3. Workspace and non-calculation entries are `NonNumericHostData`.
4. An unsupported legacy marker is `LegacyExcluded` with `DType::F32` as the
   canonical replacement.
5. `dequant_f32` is `F32Payload`; any remaining symbol containing `convert` is
   `F32OrI32Payload`.
6. A remaining non-GPU symbol is `NonNumericHostData`.
7. GPU names containing one of `argmax`, `argmin`, `argsort`, `categorical`,
   `count`, `degree`, `histogram`, `index`, `indices`, `iota`, `mask`,
   `permutation`, `route`, `sort`, `split`, or `topk` are marked
   `F32AndI32Payloads`; `bernoulli` and `mask` receive the same treatment.
8. Other remaining GPU names default to `F32Payload`.

The final steps are metadata for entries that have not acquired an owned recipe;
they do not add an execution path. The current 423-descriptor census is:

| Contract | Count |
| --- | ---: |
| `Exact` | 123 |
| `F32Payload` | 89 |
| `I32Payload` | 7 |
| `F32AndI32Payloads` | 167 |
| `F32OrI32Payload` | 3 |
| `NonNumericHostData` | 34 |
| `LegacyExcluded` | 0 |

### Legacy dtype marker

`explicit_legacy_dtype` is independent metadata. Its matching order is:

- a symbol containing `_f16` gives `F16`;
- a symbol containing `_f64`, beginning `gpu_dgem`, or exactly equal to
  `gpu_dger_into`, `gpu_dsyrk`, `gpu_dasum`, `gpu_idamax`, or
  `gpu_scale_f64_inplace` gives `F64`;
- a symbol containing `_u8` gives `U8`; and
- `gpu_convert` from a source containing `infer_ops.rs` gives
  `DynamicQuantized`.

The current registry carries 15 markers: four `F16`, eight `F64`, two `U8`, and
one `DynamicQuantized`. All 15 also have an owned scalar or composition
lowering, so none currently produces `LegacyExcluded`. The unused enum variants
(`UntypedDeviceBuffer`, `F32`, `HostBytes`, `HostText`, and `HostObject`) reserve
names for future compatibility metadata but are not emitted by this function.

### Operation family

Family selection is also first-match. Owned scalar, primitive, composition, and
non-calculation recipes delegate to their own family methods. Workspace formulas
are `Workspace`. The code recognizes unsupported `WorkspaceQuery` and
`LifecycleOperation` as `Workspace` and `Lifecycle`, respectively, although the
current lowering function does not emit either unsupported reason. Remaining
entries use source and symbol heuristics in this order:

1. `Data`, `Model`, `Train`, and `Infer` are `Facade`.
2. `safetensors` sources or `parse_` symbols are `Parsing`.
3. `bpe` or `chat` sources are `Encoding`.
4. `llm` or `infer` sources are `Inference`.
5. `optimizers` sources are `Optimizer`; `losses` sources are `Loss`.
6. `attention` in the source or symbol is `Attention`.
7. `linalg` sources use `Fft` for `fft`, `Contraction` for `gem`, `ger`,
   `syrk`, or `bmm`, and `Solver` otherwise.
8. `reductions` sources use `Scan` for `cum`, `prefix`, or `scan`,
   `ShapeAndIndexing` for `sort`, and `Reduction` otherwise.
9. `encoding`, `graph`, `cluster`, `bayes`, `svm`, and `rl` sources map to
   `Encoding`, `Graph`, `Clustering`, `Bayesian`, `SupportVectorMachine`, and
   `ReinforcementLearning`.
10. `forest`, `catboost`, `lightgbm`, and `xgboost` sources map to `Tree`.
11. `diffusion`, `sequence`, and `moe` sources map to `Diffusion`, `Sequence`,
    and `MixtureOfExperts`.
12. The remaining GPU name goes through `kernel_family`: convolution and
    im2col/col2im, pooling/upsample, normalization, embedding, GRU/LSTM,
    SSM/gated-delta, random/dropout/bernoulli, reduction/sum-all,
    gather/scatter/slice/concat/transpose, GEMM/linear/matvec, loss/grad, and
    finally `Other`, in that order.

Scalar families have their own precedence: loss sources or BCE/MSE/KL names,
optimizer sources or `_update`, dropout, activation names (`elu`, `gelu`,
`relu`, `selu`, `sigmoid`, `silu`, `softplus`, `tanh`), fill operations, then
`Elementwise`. The current family counts are:

| Family | Count | Family | Count |
| --- | ---: | --- | ---: |
| `Activation` | 26 | `Attention` | 9 |
| `Bayesian` | 6 | `Clustering` | 5 |
| `Contraction` | 16 | `Convolution` | 10 |
| `Creation` | 6 | `Diffusion` | 4 |
| `Distance` | 5 | `Elementwise` | 57 |
| `Embedding` | 10 | `Encoding` | 8 |
| `Fft` | 2 | `Facade` | 4 |
| `Graph` | 5 | `Histogram` | 1 |
| `Inference` | 4 | `Lifecycle` | 1 |
| `Loss` | 18 | `Metric` | 6 |
| `MixtureOfExperts` | 4 | `Normalization` | 18 |
| `Optimizer` | 13 | `Other` | 2 |
| `Parsing` | 1 | `Pooling` | 15 |
| `Quantization` | 4 | `Random` | 7 |
| `Recurrent` | 4 | `Reduction` | 18 |
| `ReinforcementLearning` | 4 | `Scan` | 7 |
| `Sequence` | 2 | `ShapeAndIndexing` | 24 |
| `Solver` | 12 | `StateSpace` | 6 |
| `Statistics` | 3 | `SupportVectorMachine` | 6 |
| `Tree` | 45 | `Workspace` | 25 |

### Alias and determinism metadata

`alias_contract` is a name-based compatibility classification. The known
in-place families require output aliasing input 1 for
`gpu_add_inplace`, `gpu_add_scalar_inplace`, `gpu_mul_inplace`,
`gpu_scale_f64_inplace`, `gpu_scale_inplace`, and `gpu_sub_inplace`; input 2 for
`gpu_add_col_scaled_inplace`, `gpu_clip_value`, `gpu_sgd_update`, and
`gpu_sgd_update_f32`; and input 0 for `gpu_scatter_add` and
`gpu_softmax_inplace`. Any other `_into` operation is `NoAlias`; an unlisted
GPU operation is `OperationSpecific`; non-GPU operations are `NoAlias`.

`determinism` gives random and conflict operations precedence over lowering
kind. Names containing `rand`, `bernoulli`, or `bootstrap` are
`CounterBasedRandom`. Names containing `scatter` or `histogram` are
`ExplicitAtomicPolicy`. Otherwise scalar recipes are
`PerElementExactOrder`, primitive, composition, and workspace recipes are
`FixedPrimitiveOrder`, non-calculation entries and host-behavior-pending
entries are `HostDeterministic`, and other unsupported entries are
`PendingDefinition`.

Current metadata counts are:

| Alias contract | Count | Determinism contract | Count |
| --- | ---: | --- | ---: |
| `NoAlias` | 87 | `PerElementExactOrder` | 94 |
| `OperationSpecific` | 324 | `FixedPrimitiveOrder` | 302 |
| `OutputMustAliasInput { input: 0 }` | 2 | `CounterBasedRandom` | 9 |
| `OutputMustAliasInput { input: 1 }` | 6 | `ExplicitAtomicPolicy` | 8 |
| `OutputMustAliasInput { input: 2 }` | 4 | `HostDeterministic` | 10 |
|  |  | `PendingDefinition` | 0 |

These fields describe the operation contract. The registry does not apply alias
rules to a graph and does not execute an atomic policy; the language and
materializer layers enforce concrete kernel alias and conflict rules.

## Iteration and lookup API

`operation_registry()` returns a fresh zero-sized `OperationRegistry` value.
The methods are deterministic scans over static arrays:

- `len()` is `RAW_OPERATION_COUNT + RECIPE_OWNED_OPERATIONS.len()`;
- `is_empty()` is the corresponding length check;
- `iter()` maps every ordinal to `describe`, selecting the generated prefix for
  ordinals below `RAW_OPERATION_COUNT` and the extension slice afterward;
- `surface_len()` and `surface_iter()` expose only the generated prefix;
- `owned_len()` and `owned_iter()` expose only the extension segment; and
- `named(symbol)` filters the complete iterator by symbol, preserving canonical
  order and retaining every source-qualified duplicate.

`resolve_unique(symbol)` is the symbol-only boundary used by compilers. It
returns `UnknownOperation` with detail `operation symbol "..." is absent` when
there is no match. It returns `AmbiguousSymbol` with the number of
source-qualified entries when a second match exists. The error has no attached
`OperationId`, because no unique descriptor was selected. A unique match is
returned unchanged.

`resolve_exact(symbol, source)` is the source-qualified boundary. It filters
both fields, returns `UnknownOperation` with detail
`operation ("...", "...") is absent from the canonical registry` when no row
matches, and returns `AmbiguousSymbol` if the same pair occurs more than once.
The current duplicate symbols have distinct sources, so exact lookup selects
one of them. The implementation deliberately retains the duplicate-pair check
even though the current inventory has no duplicate pair.

Observed current examples:

| Call | Result |
| --- | --- |
| `resolve_unique("gpu_add_into")` | Ordinal 19, scalar `Add`, `Elementwise`, exact f32/f32 to f32 contract. |
| `resolve_unique("predict")` | `AmbiguousSymbol`, three source-qualified entries. |
| `resolve_exact("predict", "lightgbm-rs/src/lib.rs:776")` | Ordinal 410, the LightGBM tree composition. |
| `resolve_unique("does_not_exist")` | `UnknownOperation`. |

## Lowering consumers and validation boundary

The registry selects a lowering kind; the corresponding module owns validation
and execution preparation.

### Scalar recipes

`ScalarRecipe::for_symbol` is a static symbol match in `ops/src/scalar.rs`.
Recipes are typed opcodes, `recipe_math` functions, or multi-instruction
`CompositeScalar` programs. `lower_scalar(descriptor)` accepts only
`LoweringAvailability::Scalar`, builds a validated `recipe_core::ScalarProgram`,
and attaches `descriptor.id` to invalid-program errors. A non-scalar descriptor
returns `WrongLoweringKind`; an unsupported descriptor returns
`UnsupportedLowering` with the descriptor definition and reason.

The current registry has 94 scalar descriptors. They cover simple arithmetic,
comparisons, math functions, activation and loss derivatives, dropout, and
optimizer scalar updates. Scalar dtype slices are the source of the exact
contracts for these rows.

### Primitive recipes

`PrimitiveRecipe::for_symbol` is a static symbol table in
`ops/src/primitive.rs`. A recipe describes reduction, scan, contraction,
gather, scatter-add, sort, index-map, or counter-based random behavior. It
also carries axis, result, contraction, direction, stability, and distribution
requirements. `lower_primitive(descriptor, request, hardware)` first requires a
primitive lowering, then checks the supplied `PrimitiveKernel` and tensor map
against the recipe. A mismatch is `PrimitiveRecipeMismatch`; a backend lowering
failure is `PrimitiveLoweringFailed`; wrong kind and unsupported entries fail
before lowering, with the descriptor ID attached.

The current registry has 29 primitive descriptors. The recipe check is where
the descriptor's alias and dtype intent meets concrete language tensors and
hardware lowering. The registry itself does not inspect a kernel request.

### Structured compositions

`CompositionRecipe::for_entry(symbol, source)` is an exact source-qualified
match in `ops/src/composition.rs`. A composition has a name, definition,
primitive-family steps, bounded repeat rules, payload domain, and family. The
registry stores its recipe and definition but not concrete tensor names or
prepared parameter values.

`validate_composition(descriptor)` accepts only a composition and checks that
its name, definition, and top-level steps are nonempty; every role is nonempty;
repeat bodies are nonempty; fixed repeat counts are nonzero; prepared parameter
names are nonempty; and nesting is at most eight levels. A wrong lowering kind
is `WrongLoweringKind`, and validation failures carry the descriptor ID.

`materialize_composition` in `ops/src/materialize.rs` is the preparation-time
semantic boundary. It validates the request's exact named tensor and parameter
ABI, requires a concrete source-qualified materializer, resolves shape and
prepared-parameter bounds, expands a finite dependency chain, emits concrete
language kernels and scalar SSA, accounts for reserved workspace and identity
ranges, and validates the resulting `CalculationGraph`. It refuses to infer
tensor ABI or a scalar formula from a symbol alone.

Concrete ownership is split into these eleven source-pair dispatch modules, in
this exact order:

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

Each concrete module owns a static
`(&str, &str)` operation list and returns `FamilyDispatch::NotOwned` unless the
descriptor's exact symbol and source pair is present. If a pair is listed but
its symbol match is incomplete, it returns `GraphMaterializationFailed` rather
than falling through to another implementation.

`remaining_composition_manifest()` scans `operation_registry().iter()`, keeps
only composition descriptors, and reports every descriptor with no concrete
materializer. Each result retains the `OperationId`, symbol, source, recipe
name, and the fixed missing-component list (`TensorAbi`, `ScalarFormula`,
`PrimitiveParameters`, `WorkspacePolicy`). In the current build, 266
descriptors are compositions, 136 have concrete source-qualified materializers,
and 130 are reported as remaining. A remaining entry is not silently mapped to
a nearby operation.

### Workspace formulas

`WorkspaceFormula::for_symbol` recognizes the static scratch queries in
`ops/src/workspace.rs`, including fixed-tree reductions and scans, sort/run
encoding, random-key sort, solver panels, and split-K partials. The current
registry has 24 workspace descriptors. `evaluate_workspace` accepts only that
lowering kind and validates dimension arity and checked arithmetic. It returns
`WorkspaceValue` in bytes or f32 elements. Wrong kind is `WrongLoweringKind`,
dimension mismatch is `WorkspaceFormulaMismatch`, and checked arithmetic
overflow is `WorkspaceArithmeticOverflow`, all with the descriptor ID attached.

### Non-calculation entries

`NonCalculationRecipe::for_entry` marks orchestration and metadata, never a CPU
calculation fallback. The current ten entries are:

| Recipe | Symbols |
| --- | --- |
| `FacadeDeclaration` | `Data`, `Model`, `Train`, `Infer` |
| `TextTokenization` | `encode` |
| `ModelContainerParsing` | `parse_safetensors` and any matching safetensors source |
| `ChatTemplateRendering` | `render_chat`, `render_template` |
| `RunShutdown` | `gpu_shutdown` |
| `EliminatedVendorWorkspaceBinding` | `gpu_blas_workspace` |

Their definitions explicitly describe host preparation, lifecycle transition,
or eliminated vendor binding. They are classified as `Facade`, `Encoding`,
`Parsing`, `Lifecycle`, or `Workspace` and do not authorize payload arithmetic.

## Compiler call paths

The public root facade exposes registry and lowering operations through
`recipe::operations`:

- `registry()` returns `recipe_ops::operation_registry()`;
- `all()` returns `registry().iter()`;
- `resolve()` delegates to `resolve_unique`;
- `resolve_exact()` delegates to `resolve_exact`; and
- `lower_scalar`, `lower_primitive`, `validate_composition`, `materialize`,
  `remaining_compositions`, and `evaluate_workspace` are thin calls into
  `recipe-ops`.

The dense training compiler uses two registry boundaries in
`training/src/compile.rs`:

1. `emit_owned_scalar` resolves a symbol with `resolve_unique`, lowers the
   descriptor with `lower_scalar`, and emits the resulting scalar program as a
   `PrimitiveKind::Elementwise` node.
2. Its `materialize` method resolves the symbol, builds named input/output
   tensors, reserves a value and kernel identity namespace, calls
   `materialize_composition`, and inserts the validated graph nodes into the
   training graph. Reservation overflow remains a compiler error; the registry
   does not allocate IDs.

The inference compiler mirrors those two paths in
`training/src/inference.rs`: `emit_owned_scalar` resolves and lowers scalar
symbols, while `materialize` resolves a source operation, passes named tensors,
prepared parameters, and reserved identities to `materialize_composition`, and
inserts the returned graph contracts and nodes.

At the root workflow, `src/training.rs` and `src/inference.rs` prepare public
declarations and call the corresponding training or inference compiler. The
compiled `StaticCalculationProgram` then goes through graph validation,
planning, scheduling, preparation, and native execution. Registry metadata is
therefore consumed before the immutable `init -> loop -> exit` run; the native
executor never guesses an operation from a string and never falls back to the
retired implementation.

Operation errors convert at the compiler boundary to
`TrainingCompileErrorKind::Operation` or `InferenceCompileErrorKind::Operation`.
The original operation kind/detail and, where a descriptor was selected, its
ordinal are retained in the formatted detail.

## Current lowering census

The current build reports these lowering counts across all 423 descriptors:

| Lowering | Count | Meaning |
| --- | ---: | --- |
| `Scalar` | 94 | Owned typed scalar opcode, math function, or composite SSA program. |
| `Primitive` | 29 | Owned non-elementwise primitive recipe checked against a kernel request. |
| `Composition` | 266 | Finite source-qualified multi-stage recipe. |
| `Workspace` | 24 | Checked static scratch/resource formula. |
| `NonCalculation` | 10 | Facade, parsing, encoding, lifecycle, or metadata behavior. |
| `Unsupported` | 0 | Fail-closed representation available for future or excluded rows. |

This census is derived from `operation_registry().iter()` in the compiled
checkout. It is intentionally separate from source text counts: a row can carry
a legacy dtype marker while still resolving to an owned canonical lowering,
and a composition can be registered while still remaining in the concrete
materialization manifest.

## Invariants and failure behavior

The following invariants are enforced by the current implementation or are
required by its static layout:

- The generated surface prefix preserves source order and zero-based ordinals.
- Source-qualified rows remain distinct even when symbols duplicate.
- Recipe-owned rows are appended after the prefix and have `surface_line == 0`.
- All descriptor classification is pure and deterministic from static strings
  and compile-time recipe tables.
- The lowering precedence above is authoritative. A later fallback never
  replaces an earlier owned recipe.
- Legacy dtype markers describe compatibility history only; canonical payload
  contracts remain f32/int32 and owned lowerings do not authorize legacy
  payloads.
- Symbol-only compilation uses `resolve_unique`; duplicate symbols must use
  `resolve_exact` or fail with `AmbiguousSymbol`.
- Composition materialization is exact by `(symbol, source)` and does not infer
  an ABI from a symbol or source family.
- Unsupported, wrong-kind, incomplete, and arithmetic failures remain visible
  as typed `OperationError` values. There is no retry, CPU substitute, or
  legacy implementation path in the registry.
- `OperationError::for_operation` is applied by the lowering, composition, and
  workspace layers after a descriptor has been selected. Lookup errors do not
  invent an ID.

Build-time failures are intentionally direct: unreadable `operation-surface`,
malformed tab rows, empty fields, u16 occurrence overflow, missing `OUT_DIR`, or
an unwritable generated file stop the build. Runtime lookup failures are
`UnknownOperation` and `AmbiguousSymbol`. Lowering and preparation failures
include wrong-kind, unsupported-lowering, primitive mismatch, invalid scalar or
composition program, missing concrete formula, invalid materialization request,
graph materialization, workspace shape, and checked-arithmetic error kinds from
`ops/src/error.rs`.

The registry is thus a finite, inspectable source-to-semantics index. It keeps
legacy identity visible, makes Recipe-owned coverage measurable, and forces
every public calculation to cross an explicit owned lowering boundary before it
can become a graph node or a native execution artifact.
