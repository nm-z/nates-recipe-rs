# Attention, sequence, and embedding materialization

## Purpose and ownership

`ops/src/materialize/attention_sequence_embedding.rs` is the concrete graph
materializer for the source-qualified attention, sequence, embedding, and
rotation entries listed in its `OPERATIONS` table. It does not execute GPU
work. It converts one checked `MaterializationRequest` into a
`recipe-language::CalculationGraph` made only from elementwise maps, fixed
reductions, gathers, and a scatter. The graph is then inserted into the
training or inference graph by the caller.

The module owns these exact `(symbol, source)` pairs:

| Symbol | Surface source | Composition recipe | Concrete entry point | Current in-tree caller |
| --- | --- | --- | --- | --- |
| `gpu_causal_softmax_rows` | `gpu-core/src/attention.rs:191` | `causal_row_softmax`, `SOFTMAX_STEPS` | `emit_causal_softmax` | dense training and dense inference attention |
| `gpu_embed_blend` | `gpu-core/src/infer_ops.rs:266` | `embedding_blend`, `GATHER_MAP` | `emit_embedding_blend` | no current repository caller |
| `gpu_embedding_backward` | `gpu-core/src/attention.rs:427` | `embedding_backward`, `MAP_SCATTER` | `emit_embedding_backward` | dense training backward pass |
| `gpu_mha_merge` | `gpu-core/src/attention.rs:227` | `multi_head_attention_merge`, `GATHER_MAP` | `emit_mha_merge` | no current repository caller |
| `gpu_mha_split` | `gpu-core/src/attention.rs:202` | `multi_head_attention_split`, `GATHER_MAP` | `emit_checked_gather_identity` | no current repository caller |
| `gpu_positional_encoding` | `gpu-core/src/attention.rs:276` | `sinusoidal_positional_encoding`, `MAP_ONLY` | `emit_positional_encoding` | no current repository caller |
| `gpu_repeat_rows` | `gpu-core/src/kernels.rs:6598` | `repeat_rows`, `GATHER_MAP` | `emit_repeat_rows` | no current repository caller |
| `gpu_rope` | `gpu-core/src/attention.rs:252` | `rotary_position_embedding`, `GATHER_MAP` | `emit_single_tensor_rope` | no current repository caller |
| `gpu_rope_partial` | `gpu-core/src/infer_ops.rs:154` | `rotary_position_embedding`, `GATHER_MAP` | `emit_single_tensor_rope` | GGUF LLaMA inference |
| `gpu_rope_partial_factors` | `gpu-core/src/infer_ops.rs:209` | `rotary_position_embedding`, `GATHER_MAP` | `emit_single_tensor_rope` | no current repository caller |
| `gpu_rope_partial_factors_pos` | `gpu-core/src/infer_ops.rs:237` | `rotary_position_embedding`, `GATHER_MAP` | `emit_single_tensor_rope` | no current repository caller |
| `gpu_rope_partial_pos` | `gpu-core/src/infer_ops.rs:181` | `rotary_position_embedding`, `GATHER_MAP` | `emit_single_tensor_rope` | no current repository caller |

The authoritative legacy inventory is
[`operation-surface.txt`](../../../../operation-surface.txt). Its rows are parsed
by [`ops/build.rs`](../../../build.rs), which generates the raw registry
entries included by [`registry.rs`](../../../src/registry.rs). Registry
`describe` first resolves a scalar, primitive, workspace, non-calculation, or
composition lowering, in that order; these entries resolve to the composition
recipes in
[`composition.rs`](../../../src/composition.rs#L946). The composition
payload is `F32AndI32`, so the registry exposes canonical f32 and int32 payloads
for every entry in this module. `CompositionRecipe::validate` rejects empty
names, definitions, steps, roles, repeat bodies, and invalid static bounds
before graph construction.

The concrete list in
[`attention_sequence_embedding.rs`](../../../src/materialize/attention_sequence_embedding.rs#L16)
is deliberately source-qualified. `supports` compares both symbol and source.
`dispatch` returns `NotOwned` for every other descriptor, and
`materialize.rs` tries each family once in a fixed order. An owned but
unrecognized branch returns `GraphMaterializationFailed`; there is no alternate
materializer. The composition entries `gpu_rope_qk` and
`gpu_rope_qk_heads_inplace` share the registry recipe but are not in this
concrete list, so they remain in `remaining_composition_manifest` and fail
closed rather than being silently treated as the single-tensor operation.

## Request boundary and graph lifecycle

The public types and the materialization pipeline are defined in
[`materialize.rs`](../../../src/materialize.rs#L36):

* `NamedTensor` pairs a stable ABI name with an immutable `Tensor` declaration.
  `Tensor` carries the `ValueId`, canonical `DType` (`F32` or `I32`), shape,
  layout, storage byte count, and temporary external-boundary flags.
* `PreparedParameter` is a typed `U64`, `I32`, `F32Bits`, or `Bool` value. The
  `PreparedParameters` map is the only source for concrete dimensions, scalar
  constants, and verified host facts. The module never reads payload values or
  invents defaults.
* `IdentityNamespace` reserves half-open intermediate-value and kernel-ID
  ranges. The caller supplies the input and output IDs separately. The graph
  builder rejects a declared tensor that falls inside the intermediate range,
  and exhaustion is an error rather than an ID wrap or fallback.
* `MaterializationRequest` also names the input whose shape resolves any
  composition bound and carries a `ByteCount` workspace limit.

`materialize_composition` performs this ordered sequence:

1. `validate_request` requires a composition lowering, at least one input and
   output, unique nonempty names and tensor IDs, valid tensor layouts, all
   inputs marked `external_input`, and all outputs marked non-input
   `external_output`.
2. `has_concrete_materializer` checks the source-qualified family inventory.
   A structured operation without a concrete family returns
   `MissingConcreteFormula` and names the missing tensor ABI, scalar formula,
   primitive parameters, and workspace policy.
3. The named iteration-shape input is found. A missing name is
   `InvalidMaterializationRequest`.
4. `expand_composition` validates the recipe and resolves every static bound
   from the selected input shape or a prepared parameter. It records
   `ResolvedBound`, `ResolvedIteration`, and `ResolvedStep` values in a finite
   dependency chain. The expansion limit is one million steps; there is no
   data-dependent host loop.
5. `Emitter::new` creates a `GraphBuilder`. Each call to `intermediate` creates
   a contiguous f32 or int32 tensor, accounts its exact storage bytes against
   the request limit, and consumes one reserved value ID. Each `emit` consumes
   one reserved kernel ID and installs forbidden alias rules for every
   input/output pair.
6. `dispatch_concrete` calls this family. `Emitter::emit_stage` checks that the
   last kernel in a stage has the `PrimitiveFamily` required by the next
   resolved step. A stage may contain parallel or preparatory kernels, but it
   still consumes exactly one recipe step. Emitting too few or too many stages,
   or emitting the wrong family, is `GraphMaterializationFailed`.
7. `Emitter::finish` requires every resolved step to have been consumed,
   validates the complete `CalculationGraph`, and returns
   `MaterializedComposition`. Callers currently use its graph; the resolved
   steps, stage mapping, workspace allocation, and identity namespace remain
   available for preparation and auditing.

The shared helpers enforce the ABI. `require_exact_abi` compares sets of input
names, output names, and prepared-parameter keys, so an omitted, extra, or
misspelled field is an `InvalidMaterializationRequest`. `input` and `output`
then retrieve the named tensor. `require_dtype` maps a dtype mismatch to the
same request error; `require_shape` reports an exact concrete-shape mismatch
as `UnsupportedConcreteShape`; `require_same_tensor_contract` compares dtype
and shape. `prepared_u64`, `prepared_f32`, and `require_true` distinguish a
missing parameter, a wrong parameter variant, and a false verified fact.

`GraphBuilder::finish` invokes `CalculationGraph::validate`. Language-level
validation checks scalar-program signatures, primitive arity and dtypes,
gather/scatter result shapes, reduction axes and tree lanes, alias matrices,
unique producers, external-boundary rules, and acyclic topology. All such
failures are converted to the operation's `GraphMaterializationFailed` error.
`Gather` and `Scatter` use `IndexBounds::Reject`, which reports an invalid
device index through the preallocated fault channel instead of permitting an
unchecked access. The caller's verified-index facts are therefore required
admission evidence, not a reason to change the primitive's bounds policy.

## Concrete operation contracts

### Multi-head attention split

`gpu_mha_split` is dispatched to the shared
`emit_checked_gather_identity` helper with names `packed`, `indices`, `heads`,
and `axis`. It requires f32 `packed`, int32 `indices`, and f32 `heads`. The
`axis` prepared u64 must fit `usize`; `Shape::gather_result(axis,
indices.shape)` independently derives the expected result shape, and `heads`
must match it exactly. The graph is:

1. `Gather { axis, bounds: Reject }` from `packed` and `indices` into a
   same-shaped f32 intermediate.
2. A f32 identity elementwise map into `heads`.

The helper does not require a verified-fact parameter and does not add a
dimension-specific policy beyond the language gather shape and index checks.

### Multi-head attention merge

`emit_mha_merge` requires inputs `heads` (f32) and `merge_indices` (i32), output
`packed` (f32), and parameters `seq`, `n_heads`, `head_dim`, and the true fact
`merge_indices_verified`. Each dimension is nonzero and at most
`MAX_I32_INDEX` (`i32::MAX`). Their checked product is also required to fit the
legacy int32 linear-index domain. All three tensors have flat shape
`[seq * n_heads * head_dim]`. It emits a reject-bounds axis-zero gather into a
f32 intermediate followed by an identity map into `packed`. Duplicate merge
indices are allowed because this operation only reads; bounds are still
checked by the primitive.

### Repeat rows

`emit_repeat_rows` requires `values` (f32), `repeat_indices` (i32), and
`repeated` (f32), plus `source_elements`, `repeats`, and the true fact
`repeat_indices_verified`. Both dimensions and their product are checked in
the nonempty int32 range. `values` is `[source_elements]`; indices and output
are `[source_elements * repeats]`. A reject-bounds axis-zero gather reads each
source row, then an identity map writes `repeated`. The operation does not
derive the modulo mapping itself; that mapping is part of the caller's
prepared index table and verified fact.

### Embedding blend

`emit_embedding_blend` accepts `source_rows` (f32), `row_indices` (i32), and
`weights` (f32), and writes `blended` (f32). Its exact parameter set is
`rows`, `k`, `source_row_count`, `embedding_width`, `scale`, and the true fact
`row_indices_verified`. All dimensions are nonempty int32 extents. The only
concrete `k` supported by this materializer is `1`; another value returns
`UnsupportedConcreteShape` with the explicit legacy-path message. `scale` is
decoded from `F32Bits` and must be finite, but it may be negative or zero.

The required shapes are:

* `source_rows`: `[source_row_count, embedding_width]`
* `row_indices`: `[rows]`
* `weights`: `[rows, 1]`
* `blended`: `[rows, embedding_width]`

The graph first gathers rows along axis zero with reject bounds. Its scalar
program then computes `(gathered_value * weight) * scale`; the `[rows, 1]`
weight broadcasts across the embedding width. The `k` parameter is admission
metadata for the one-row legacy path and does not add a loop.

### Embedding backward

`emit_embedding_backward` requires f32 `gradient`, int32 `indices`, f32
`gradient_table_base`, and f32 `gradient_table`; parameters are `rows`,
`columns`, and `vocabulary`. Both `rows * columns` and `vocabulary * columns`
must be checked products in the int32 linear-index domain. Shapes are:

* `gradient`: `[rows, columns]`
* `indices`: `[rows]`
* `gradient_table_base` and `gradient_table`: `[vocabulary, columns]`

The base and output must have identical dtype and shape. An identity map copies
the row gradient to an intermediate, then a reject-bounds axis-zero scatter
writes `gradient_table` from `gradient_table_base`, `indices`, and the mapped
updates. Its conflict policy is `Atomic { operation: Add, ordering: Relaxed }`.
That explicit atomic add is what makes repeated token IDs accumulate rather
than overwrite one another. The caller normally supplies a zero base, but the
materializer treats it as an arbitrary f32 tensor with the declared contract.

### Sinusoidal positional encoding

`emit_positional_encoding` requires f32 `angles`, int32 `channel_parity`, and
f32 `encoding`, with parameters `seq`, `dim`, and the true fact
`angles_verified`. Every tensor has shape `[seq, dim]`; both extents are
nonempty and int32-indexable.

The one `MAP_ONLY` recipe step is emitted as one stage containing three kernels:

1. A f32 `MathFunction::Sin` scalar program maps `angles` to a sine
   intermediate.
2. A f32 `MathFunction::Cos` scalar program maps `angles` to a cosine
   intermediate.
3. `positional_select_program` selects cosine when parity is zero and sine
   when parity is one, then writes `encoding`.

The selection program compares parity with int32 zero and one, ORs those
predicates, and executes `Require` before `Select`. Any parity other than zero
or one faults the calculation. Recipe math programs require finite inputs;
sin and cos additionally require each angle to be in `[-8192, 8192]` through
their scalar `Require` predicates. The host fact says that the angle table was
prepared and verified; the scalar program still enforces its runtime domain.

### Causal row softmax

`emit_causal_softmax` requires f32 `values`, int32 `causal_mask`, and f32
`softmax`, with `rows`, `columns`, `tree_lanes`, and the true fact
`causal_mask_verified`. `values` and `causal_mask` must both be
`[rows, columns]`; `softmax` must have the same dtype and shape as `values`.
`tree_lanes` is a power of two in `1..=1024`, matching the language reduction
contract. `rows` and `columns` are nonempty int32 extents. The surrounding
callers check their flattened row products before entering this boundary.

The composition is `SOFTMAX_STEPS`, four logical steps. The concrete stages
are grouped so that the parallel work still consumes those four steps:

1. The first stage applies `causal_max_mask_program`, selecting `value` for a
   nonzero mask and the finite floor `-1.0e30` otherwise, then reduces the
   masked matrix with `ReduceOperator::Maximum` over axis one, keeping shape
   `[rows, 1]`.
2. The second stage subtracts the row maximum for unmasked elements and selects
   zero for masked elements, applies the owned binary32 `Exp`, and masks the
   raw exponential back to zero. The result remains `[rows, columns]`.
3. A sum reduction over axis one, keeping dimensions, produces positive row
   sums `[rows, 1]`.
4. `checked_divide_program` requires each denominator to be strictly greater
   than f32 zero and divides each exponential by its row sum.

The mask must admit at least one position in every row. A row containing no
true mask has a zero denominator and faults through `Require`. Unmasked values
must remain in the `MathFunction::Exp` finite `[-80, 80]` domain after the
maximum subtraction; non-finite values or an out-of-domain shift fault rather
than selecting a fallback. The causal predicate itself is not synthesized by
this module. Callers prepare the int32 mask and set `causal_mask_verified` only
after checking it.

### Rotary position embedding variants

All five concrete symbols use `emit_single_tensor_rope` and the two-step
`GATHER_MAP` recipe. The common inputs are f32 `values`, f32 `cosines`, f32
`signed_sines`, int32 `partner_indices`, and f32 `rotated`. Every common tensor
has flat shape `[elements]`, where `elements` is checked against the int32
linear-index domain. `rotation_tables_verified` is an exact true fact.

`gpu_rope` uses parameters `seq`, `dim`, `base`, and the verification fact.
`dim` must be even, `base` must be finite and positive, and
`elements = seq * dim` must fit the checked product domain. The cosine and
sine tables are supplied by the caller; `base` is an admission parameter and
is not used to generate tables inside this materializer.

The four partial forms use `rows`, `head_dim`, `rotary_dim`,
`heads_per_token`, `theta`, and the verification fact. `rotary_dim` is even
and cannot exceed `head_dim`; all four dimensions are nonempty int32 extents;
`theta` is finite and positive; `elements = rows * head_dim` is checked.
The two `*_pos` forms add a `position_base` u64. They check
`position_base + (rows - 1) / heads_per_token` for u64 overflow and require the
final position to fit int32. This check prevents the caller's position table
from leaving the legacy coordinate domain.

The two `*_factors` forms add an f32 `factors` input with shape
`[rotary_dim / 2]`. It is type and shape checked, but the emitted gather and
rotation kernels do not include its value, and `rotation_program` has only the
four scalar inputs `value`, `partner`, `cosine`, and `signed_sine`. Similarly,
`theta`, `position_base`, and full-form `base` are validated admission data,
not runtime scalar inputs. This is the current concrete behavior and is
important when interpreting the operation's declared ABI.

The graph gathers `partners` from `values` and `partner_indices` along axis
zero with `IndexBounds::Reject`, then applies
`(value * cosine) + (partner * signed_sine)` elementwise to `rotated`. The
caller supplies signed sine values for the desired direction, so the scalar
formula is deliberately a single add of two products.

## Scalar program details and error mapping

The family reuses the scalar-builder helpers in
[`materialize.rs`](../../../src/materialize.rs#L4149). They create a fresh
typed `ScalarProgramBuilder`, add typed inputs and bit-preserving constants,
apply `ScalarOpcode` signatures, and finish with a validated SSA program. A
builder error, math-program conversion error, or invalid scalar signature is
wrapped as `OperationErrorKind::GraphMaterializationFailed` with the operation
ID.

The operation-specific programs are:

| Program | Inputs | Formula and runtime checks |
| --- | --- | --- |
| `embedding_blend_program` | f32 value, f32 weight | `(value * weight) * scale`; `scale` is embedded as an f32 constant after host finiteness validation |
| `positional_select_program` | f32 sine, f32 cosine, i32 parity | `Require((parity == 0) OR (parity == 1))`, then `Select(parity, cosine, sine)` |
| `causal_max_mask_program` | f32 value, i32 mask | `Select(mask, value, -1.0e30)` |
| `causal_safe_shift_program` | f32 value, f32 row maximum, i32 mask | `Select(mask, value - maximum, 0.0)` |
| `mask_to_zero_program` | f32 value, i32 mask | `Select(mask, value, 0.0)` |
| `checked_divide_program` | f32 numerator, f32 denominator | `Require(denominator > 0.0)`, then `numerator / denominator` |
| `rotation_program` | f32 value, partner, cosine, signed sine | `(value * cosine) + (partner * signed_sine)` |

`MathFunction::Sin`, `Cos`, and `Exp` come from
[`math/src/program.rs`](../../../../math/src/program.rs#L17). Their contracts
reject non-finite f32 inputs through `Require`; sine and cosine use the
closed interval `[-8192, 8192]`, while exp uses `[-80, 80]`. These are scalar
program faults reported by the runtime fault channel, not host-side
preflight branches in this module.

The local helper error classes are intentionally narrow:

* Missing names, extra names, duplicate names, bad external flags, dtype
  mismatches, missing facts, false facts, wrong prepared variants, non-finite
  scalar parameters, and invalid `tree_lanes` are
  `InvalidMaterializationRequest`.
* Zero or oversized dimensions, odd dimensions where an even extent is
  required, `k != 1`, rotary dimensions larger than head dimensions, checked
  product overflow into the int32 domain, and partial-RoPE final positions
  outside int32 are `UnsupportedConcreteShape`.
* A u64 multiplication overflow in `checked_product` is
  `WorkspaceArithmeticOverflow`; this is distinct from a valid u64 product
  that exceeds the legacy int32 linear-index limit.
* Shape construction, scalar construction, primitive construction, graph
  validation, stage-family mismatches, and incomplete emission are
  `GraphMaterializationFailed`.
* Exhausted reserved value or kernel ranges are
  `IdentityNamespaceExhausted`; exceeding the caller's scratch-byte limit is
  `WorkspaceLimitExceeded`.

Every `OperationError` retains the source-qualified `OperationId`. The outer
training and inference compilers convert it without replacing its text:
[`training/src/error.rs`](../../../../training/src/error.rs#L50) maps it to
`TrainingCompileErrorKind::Operation`, while
[`training/src/inference.rs`](../../../../training/src/inference.rs#L183) maps it
to `InferenceCompileErrorKind::Operation`.

## Callers and end-to-end roles

### Dense training

`GraphCompiler::materialize` in
[`training/src/compile.rs`](../../../../training/src/compile.rs#L10937) is the
training adapter. It clones the existing tensor contracts, marks the local
inputs and outputs as fragment boundaries, converts names to `NamedTensor`,
reserves 64 value IDs and 64 kernel IDs, resolves the unique registry
descriptor, and calls `materialize_composition` with an unlimited workspace
limit. It inserts every returned tensor contract and node into the training
compiler and assigns the caller's `IterationDomain` to each materialized
kernel.

The causal attention forward path validates the fixed embedding geometry, does
the query/key/value contractions and scaling, then calls
`gpu_causal_softmax_rows` from
[`compile.rs`](../../../../training/src/compile.rs#L6431). Its prepared request
uses flattened rows `rows * heads * sequence`, columns `sequence`, the
configured reduction tree lanes, and `causal_mask_verified = true`. The
materialized result is reinterpreted back to
`[rows, heads, sequence, sequence]`, contracted with values, reordered from
head-major to sequence-major, and projected by the output matrix.

The backward block walker requires the embedding to be the first block because
token IDs have no input gradient. `backward_embedding` masks invalid training
rows, packs `[rows, sequence, dimensions]` gradients into `[token_rows,
dimensions]`, creates a zero `[vocabulary, dimensions]` base, and calls
`gpu_embedding_backward` with `rows = token_rows`, `columns = dimensions`, and
the embedding vocabulary. Duplicate token IDs are accumulated by the
materialized atomic scatter. The resulting table gradient is consumed by the
optimizer update path and becomes the next `DenseEmbeddingState` table.

The training compiler's ordinary embedding forward path does not call
`gpu_embed_blend`: it validates an exact int32 token matrix, packs it, performs
a direct reject-bounds gather from the learned table, and reshapes the gathered
rows. This direct path is in
[`compile.rs`](../../../../training/src/compile.rs#L5330). Likewise, attention
projection, head layout conversion, and context contractions use direct
primitive emission. The materializer therefore supplies the registered legacy
operation contract without duplicating the specialized forward implementation.

The same causal and embedding geometry rules are enforced before materializing:
`DenseEmbedding` documents exact int32 IDs in `0..vocabulary`, and
`DenseAttention` requires an even division of the embedding dimension across
heads in [`training/src/model.rs`](../../../../training/src/model.rs#L422). Dataset
validation rejects vocabularies above int32, non-int32 or non-numeric token
features, missing fixed positions, and token values outside the vocabulary in
[`compile.rs`](../../../../training/src/compile.rs#L1635). Network validation
requires one leading embedding and permits the first attention block only
immediately after it. These checks are why the materializer can require true
verification facts instead of guessing or clamping index tables.

### Dense inference

`InferenceGraphCompiler::materialize` in
[`training/src/inference.rs`](../../../../training/src/inference.rs#L2008) follows
the same adapter sequence: clone and mark fragment boundary tensors, reserve
64 IDs in each namespace, resolve the unique registry entry, materialize, then
insert the graph nodes and assign `IterationDomain::first()` to each one. Its
workspace limit is also `u64::MAX`.

`compile_prepared_inference` validates checkpoint block order and geometry,
loads a leading embedding table as an external f32 tensor, and uses direct
gather for embedding forward. For an attention block it validates sequence
length, channel width, `heads * head_dimension`, and all four f32 matrices,
then projects query, key, and value, forms scaled scores, and calls the same
causal helper. The request is assembled at
[`inference.rs`](../../../../training/src/inference.rs#L4175) with a mask generated
from identity positions, flattened rows `rows * heads * sequence`, columns
`sequence`, configured tree lanes, and `causal_mask_verified = true`. The
softmax result is reinterpreted to the four-dimensional score shape before the
context contraction and output projection.

Inference compilation reports operation failures as
`InferenceCompileErrorKind::Operation`. Its final graph is validated,
canonicalized through OGDL, round-tripped, and wrapped in a one-iteration
`StaticCalculationProgram`, so the materialized stages participate in the same
production graph boundary as every other inference primitive.

### GGUF LLaMA inference

The GGUF path uses the partial RoPE entry. `prepare_rope_inputs` in
[`training/src/gguf_llama.rs`](../../../../training/src/gguf_llama.rs#L755) creates
one flat partner-index, cosine, and signed-sine table for every
`sequence * heads * head_dimension` element. It checks all partner indices and
element counts against int32, and publishes the three tables as immutable
external inputs. Optional model frequency factors are resolved while preparing
the table.

`apply_rope` reinterprets the query or key tensor to the flat element shape,
allocates a flat rotated output, and calls `gpu_rope_partial` with
`rows = sequence * heads`, `head_dim = rotary_dim = head_dimension`,
`heads_per_token = heads`, the model RoPE base as `theta`, and
`rotation_tables_verified = true` ([`gguf_llama.rs`](../../../../training/src/gguf_llama.rs#L918)).
The caller then reinterprets the result back to
`[1, sequence, heads, head_dimension]`. The surrounding GGUF attention path
uses the materialized rotation for query and key, forms scaled causal scores,
calls the shared causal softmax materializer, and continues with the context
contraction.

### Static program and native execution boundary

Training finish validates the assembled graph, serializes and deserializes it
through OGDL, then builds a `StaticCalculationProgram` with the recorded kernel
domains in [`compile.rs`](../../../../training/src/compile.rs#L11078). Inference
finish performs the same graph validation and OGDL round trip and fixes the
program to one inference iteration in
[`inference.rs`](../../../../training/src/inference.rs#L4619). The program crate
requires one explicit iteration domain for every graph kernel and checks that
producer domains cover consumer first-use iterations in
[`program/src/lib.rs`](../../../../program/src/lib.rs#L39). Native preparation and
the executor consume this static graph; the attention/embedding materializer
has no runtime loop, queue, allocation, or backend branch of its own.

## Invariants to preserve when changing this module

1. Keep the operation list source-qualified and synchronized with
   `operation-surface.txt`. A symbol with a different source is a different
   registry descriptor, even if its spelling is identical.
2. Preserve the exact prepared-parameter and tensor-name sets. They are the
   materializer's typed ABI and are checked before any graph node is emitted.
3. Keep all extents and products inside the canonical nonempty int32 index
   domain. Use the existing checked helpers, not casts, clamping, or a second
   limit.
4. Keep `IndexBounds::Reject` and the explicit scatter conflict policy. Host
   verification facts admit a prepared table; they do not authorize unchecked
   device memory access.
5. Keep reduction axes, `keep_dimensions`, and power-of-two tree lanes aligned
   with the declared `SOFTMAX_STEPS` recipe. The Emitter's stage grouping is
   part of the resolved-step contract.
6. Keep scalar formulas in typed `ScalarProgram` SSA. `Require` is the runtime
   fault mechanism for invalid parity, non-finite math domains, and zero row
   sums; it must not be replaced by a host fallback or an alternate result.
7. Reserve intermediates before the run loop and account their exact bytes.
   Input/output IDs remain caller-owned, and every emitted kernel must have
   forbidden aliases unless a future concrete contract explicitly proves a
   different rule.
8. Preserve the current boundary between this generic legacy materializer and
   specialized training or inference helpers. Adding a second embedding or
   attention implementation here would create a parallel path instead of
   generalizing the existing one.
