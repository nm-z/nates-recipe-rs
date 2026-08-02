<!--
module: training/src/gguf_llama.rs
public_model_boundary: recipe.model().load("model.gguf")
public_inference_boundary: recipe.infer().evaluate()
advanced_decode_entrypoint: recipe_training::decode_gguf_llama
advanced_compile_entrypoint: recipe_training::compile_prepared_gguf_llama_inference
model_family: dense-F32 GGUF-v3 llama
calculation_dtypes: [f32, i32]
prediction: raw [sequence, vocabulary] f32 token logits
runtime_lifecycle: [preparation, init, one loop iteration, exit, reporting]
authoritative_state: GgufLlamaArtifact, finalized bundle, native exit image
-->

# GGUF Llama

`training/src/gguf_llama.rs` is the one executable GGUF model instrument in
this workspace. It turns a bounded GGUF-v3 file whose
`general.architecture` metadata is exactly `llama` into a validated,
Recipe-owned model artifact, then lowers one exact integer-token sequence to a
static calculation graph. The instrument is deliberately narrow. It accepts
the ordinary dense F32 Llama graph with full-head adjacent-pair RoPE, equal
query and key/value head counts, causal multi-head attention, RMSNorm, and
parallel SwiGLU. Unsupported architectures, quantization, GQA, partial rotary
dimensions, mixture-of-experts state, and incompatible Llama metadata fail
before native preparation. No other GGUF architecture is silently interpreted
as this graph.

The complete user path is:

```text
recipe.data("tokens.txt")
recipe.model().load("model.gguf")
recipe.infer().evaluate()
    -> declaration resolution and target-free policy checks
    -> bounded token-source distillation and selection
    -> bounded GGUF snapshot and dense-F32 Llama decoding
    -> one token table bound to the saved vocabulary/context contract
    -> one immutable Recipe calculation graph
    -> measured native preparation, init admission, one loop iteration, exit
    -> validated raw logits and post-exit report rows
```

The implementation owns model-family-specific decoding and graph construction.
The generic inference module owns typed external-input records, canonical graph
validation, static-program construction, native handoff, lifecycle checks, and
prediction-image collection. The root library owns public declaration
resolution and report formatting. GGUF structural conversion in
`recipe-ingest` is a separate feature; parsing or converting an arbitrary GGUF
does not make that architecture executable.

## Public declaration and dispatch

### `Model::load` records intent

`src/api.rs:1008-1059` defines `Model` as a backend-neutral declaration. Its
`load(&str)` method records one nonempty path and remembers the cloned model in
the declaration sequence. It does not open the path, parse metadata, allocate
weights, or compile a graph. A second source, an empty path, or a loaded source
combined with inline layers, Bayesian dependencies, an objective, or gradient
policy is deferred as a declaration error. `Model::validate`
(`src/api.rs:1462-1476`) requires at least one model source or an inline model,
and therefore accepts a loaded `.gguf` only as a source declaration.

`Infer::evaluate` (`src/api.rs:2218-2240`) consumes the immediately preceding
data/model sequence, validates the static logging policy, and calls
`evaluate_inference_declaration`. Inference logging is limited to host `Time`
and `Device` reporting. Training metrics are rejected because a GGUF run has no
targets, optimizer state, or training epoch.

### Extension dispatch is exact

`src/inference.rs:482-543` is the root-library preparation boundary:

1. `Infer`, `Data`, and `Model` declarations are validated.
2. `require_target_free_data_policy` rejects `.target(...)`, `.split(...)`,
   and `.norm(...)`. A loaded model supplies the complete feature and
   normalization contract, and inference evaluates every retained row.
3. `require_inference_model_source` requires `Model::load` and the exact
   lowercase extension `.ogdl` or `.gguf`; unknown or absent extensions are
   unsupported declarations.
4. `.gguf` selects `LoadedInferenceModel::GgufLlama` and calls
   `load_gguf_llama_model_file` with `gguf_limits_for_file`.
5. The selected data is distilled and target-free row/column selection is
   applied. The selected `RawTable` is passed to
   `prepare_gguf_llama_inference_table`.
6. The prepared table is lowered with
   `compile_prepared_gguf_llama_inference` and wrapped as
   `CompiledModelInference::GgufLlama`.

`gguf_limits_for_file` (`src/inference.rs:584-600`) obtains the regular-file
length with `std::fs::metadata`, refuses a metadata failure as a runtime
error, uses `max(file_bytes, 1)` as every file, metadata, tensor, string, and
array bound, and fixes GGUF rank and array-depth-related parser limits to the
current bounded values. The limit is then enforced again by the immutable
source snapshot and by `parse_gguf`.

`training/src/inference.rs:45-52` includes `gguf_llama.rs` as a private module
and re-exports the public artifact, error, decode, load, table-preparation, and
compile entrypoints. `training/src/lib.rs:62-74` exposes those APIs from the
`recipe-training` crate. No alternate GGUF decoder or execution path is
registered.

### Data is prepared before model binding

`src/data_prepare.rs:97-129` is the target-free source boundary used by the
root dispatch:

* `distill_data` reads the declared files and containers once under finite
  ingest limits, retaining source and row order without fitting semantic
  types, targets, normalization, or a split.
* `select_target_free_data` applies only declared row predicates and column
  exclusions, then returns a rectangular `RawTable`. Predicates see the
  original row before excluded columns are removed. The loaded model remains
  authoritative after selection.

The GGUF path intentionally does not call the semantic feature-schema
preparation used by `.ogdl` models. It consumes the selected table as one
ordered token stream and performs its own exact integer-token checks.

## Bounded source and GGUF container boundary

### Immutable source snapshot

`load_gguf_llama_model_file` (`training/src/gguf_llama.rs:478-483`) converts the
file-byte limit to a nonzero `SourceLimit`, calls
`recipe_ingest::read_source_snapshot`, and decodes only the returned bytes.
`read_source_snapshot` (`ingest/src/source.rs:34-177`) opens a regular file,
checks metadata against the bound, reads at most `limit + 1` bytes so a
concurrently growing file cannot bypass the bound, computes a SHA-256 digest,
and closes the file before returning. The runtime graph retains no file handle,
callback, or path-reading operation. A source open, non-regular file, metadata,
read, arithmetic, or bound failure is retained as an
`InferencePreparationError::CheckpointSource` through the generic conversion
boundary, while GGUF structural failures become `InferencePreparationError::GgufLlama`.

### `parse_gguf` validates structure and retains encoded bytes

`recipe-ingest/src/gguf.rs` parses a complete GGUF v2 or v3 image without
decoding tensor payloads. `GgufLimits` (`ingest/src/gguf.rs:20-77`) contains
nonzero bounds for file bytes, metadata pairs, tensor count, rank, aggregate
string bytes, aggregate array elements, and array depth. `GgufArchive`
(`ingest/src/gguf.rs:447-503`) borrows the source bytes and owns metadata and
tensor descriptor vectors. `raw_tensor(name)` returns the exact encoded byte
span from `data_offset` through `data_offset + encoded_bytes`; it never converts
quantized or F32 bytes.

Before returning an archive, `parse_gguf` (`ingest/src/gguf.rs:874-1109`):

* checks the file bound and `GGUF` magic;
* detects little-endian v2/v3 or big-endian v3, reads counts, and enforces all
  aggregate limits;
* decodes typed metadata, including nested arrays, while enforcing UTF-8,
  duplicate-key, scalar-type, boolean, string, element-count, and depth rules;
* accepts `general.alignment` only as a positive multiple of eight, defaulting
  to 32 (`ingest/src/gguf.rs:1269-1305`);
* validates unique tensor names, rank, dimensions, tensor type codes, aligned
  offsets, and encoded block sizes;
* requires zero header-to-data, inter-tensor, and terminal padding;
* checks that all tensor spans lie within the image and do not overlap; and
* rejects trailing bytes other than the permitted final alignment padding.

The parser's broad format support is intentionally narrower than the executable
instrument. Its `GgufTensorType` enum includes F32, F16, integer, and many
quantized block layouts (`ingest/src/gguf.rs:219-412`), but the Llama decoder
admits only F32 tensors with its required dimensions. Likewise, the parser can
return v2, big-endian v3, or other valid layouts, while `decode_gguf_llama`
accepts only v3 little-endian.

## Decode and model-state construction

### Failure types

`GgufLlamaErrorKind` (`training/src/gguf_llama.rs:25-38`) is the stable
model-specific failure class:

| Kind | Actual boundary |
| --- | --- |
| `Container` | `parse_gguf` rejected bounds, magic, version encoding, metadata, tensor layout, offsets, padding, or arithmetic. |
| `UnsupportedArchitecture` | `general.architecture` is present and typed as a string but is not exactly `llama`. |
| `UnsupportedVariant` | A valid Llama container selects a graph variant not implemented by this instrument, or an unconsumed tensor remains. |
| `MissingMetadata` | A required metadata key is absent. |
| `InvalidMetadata` | A required key has the wrong encoded type, a zero geometry, a nonfinite or nonpositive scalar, an invalid array, or another malformed value. |
| `MissingTensor` | A required named tensor is absent. |
| `InvalidTensor` | A named tensor has a wrong type/shape, has no raw byte span, or a saved RoPE factor is nonfinite or zero. |
| `InvalidTokenStream` | The selected table is not one token vector, has malformed rows/UTF-8/integers, contains an out-of-range token, is empty, or exceeds context. |
| `ArithmeticOverflow` | A GGUF geometry cannot fit the host address space while building the artifact. |

`GgufLlamaError` stores the kind and a concrete detail string and implements
`Display` as `Kind: detail`. `InferencePreparationError::GgufLlama` wraps it
without changing the classification.

### Artifact representation

`GgufLlamaArtifact` (`training/src/gguf_llama.rs:99-168`) is owned, cloneable,
validated model state. Public accessors expose only `architecture()` (`"llama"`),
vocabulary, context length, embedding length, and block count. Internal state
retains:

| Field | Meaning |
| --- | --- |
| `vocabulary`, `context_length`, `embedding_length`, `feed_forward_length` | Saved nonzero geometry. |
| `heads`, `head_dimension` | Equal query/KV head count and `embedding_length / heads`. |
| `rms_epsilon_bits` | Exact saved RMS epsilon bits. |
| `rope_base_bits` | Exact saved RoPE frequency-base bits. |
| `rope_frequency_scale_bits` | Effective scale, `1` for no scaling or reciprocal positive linear factor. |
| `rope_attention_factor_bits` | Product of optional YaRN attention and ordinary attention factors. |
| `attention_scale_bits` | Saved attention score scale or `1 / sqrt(head_dimension)`. |
| `clamp_kqv_bits` | Nonnegative saved Q/K/V symmetric clamp, or zero. |
| `tensors` | Owned names, graph-order shapes, and exact raw F32 bytes. |
| `token_embedding`, `output_norm`, `output`, `rope_factors` | Tensor indices. `output` falls back to `token_embedding`; RoPE factors remain optional. |
| `blocks` | Ordered `GgufLlamaBlock` values, one for each saved block. |

Floating values are stored as integer bit patterns so artifact equality and
graph preparation preserve the exact GGUF bits. `execution_tensor_indices`
(`training/src/gguf_llama.rs:145-168`) collects every embedding, norm,
projection weight, optional bias, and optional scale used by the graph. It
explicitly removes `rope_factors`: those bytes are decoded during graph
preparation into immutable rotation tables rather than admitted as a device
tensor input.

`PreparedGgufLlamaInference` (`training/src/gguf_llama.rs:171-184`) owns one
artifact plus one checked `Vec<i32>` token sequence. It is the only value passed
from token-table preparation into graph compilation.

### Metadata contract and variant gates

`decode_gguf_llama` begins at `training/src/gguf_llama.rs:186`. It maps any
`parse_gguf` error to `Container`, then applies the following gates in order.

#### Container and architecture

* `archive.version()` must be `3`.
* `archive.endian()` must be `GgufEndian::Little`. The graph always treats
  tensor bytes as little-endian F32 images.
* `general.architecture` must be a required string and equal `"llama"`.

Missing `general.architecture` is `MissingMetadata`; a non-string is
`InvalidMetadata`; another string is `UnsupportedArchitecture`.

#### Required integer geometry

The following keys must be encoded GGUF `U32` values and are converted to
`u64` without host-side narrowing:

| Metadata key | Artifact use and gate |
| --- | --- |
| `llama.vocab_size` | Vocabulary width. It must be nonzero and no greater than `i32::MAX`, because token IDs and gather indices use exact int32. |
| `llama.context_length` | Maximum accepted token-stream length. It must be nonzero. |
| `llama.embedding_length` | Model row width. It must be nonzero and divisible by `heads`. |
| `llama.block_count` | Number of repeated transformer blocks. It must be nonzero and fit `usize` when allocating the block vector. |
| `llama.feed_forward_length` | SwiGLU gate/up width. It must be nonzero. |
| `llama.attention.head_count` | Query and key/value head count. It must be nonzero. |
| `llama.attention.head_count_kv` | Must equal `head_count`; grouped-query attention is not implemented. |
| `llama.rope.dimension_count` | Must equal the complete `head_dimension` and be even. |

`head_dimension` is derived as `embedding_length / heads`. A non-divisible
embedding is `InvalidMetadata`; a partial or odd rotary dimension is
`UnsupportedVariant`.

The optional `llama.attention.key_length` and
`llama.attention.value_length` are typed `U32` when present and default to
`head_dimension`. Both must equal `head_dimension`. Optional
`llama.expert_count` defaults to zero and must remain zero. Optional
`llama.use_parallel_residual` defaults to false and must be false. Optional
`llama.attention.causal` defaults to true and must be true. These checks keep
the graph's single dense causal interpretation unambiguous.

#### Floating and boolean metadata

`llama.attention.layer_norm_rms_epsilon` is required and must be an encoded
F32 that is finite and positive. `llama.rope.freq_base` defaults to `10000.0`
and must be finite and positive. Optional values are read only when their GGUF
encoded type matches the expected type. A present value with another type is
`InvalidMetadata`, not a coercion.

RoPE scaling is reduced to the exact factors saved in the artifact:

* `llama.rope.scaling.type` defaults to `"linear"`. `"none"` gives a
  frequency scale of `1.0`.
* For `"linear"`, an absent or zero factor gives `1.0`; a finite positive
  `llama.rope.scaling.factor` gives its reciprocal. A nonfinite or negative
  nonzero factor is `InvalidMetadata`.
* Any other scaling type is `UnsupportedVariant`.
* `llama.rope.scaling.yarn_ext_factor` defaults to `-1.0`; negative values are
  treated as an effective zero. Any nonzero effective value selects an
  unsupported YaRN extrapolation variant.
* `llama.rope.scaling.yarn_attn_factor` and
  `llama.rope.scaling.attn_factor` default to `1.0`. Their product must be
  finite and positive and is stored as `rope_attention_factor_bits`.

`llama.attention.scale` defaults to `1 / sqrt(head_dimension)` and otherwise
must be finite and positive. `llama.attention.clamp_kqv` defaults to zero and
must be finite and nonnegative. The clamp is applied symmetrically to Q, K,
and V projections when nonzero.

If `llama.swiglu_clamp_shexp` is present, it must be a metadata array with at
least `block_count` F32 entries. The first entry for each block must have
absolute value at most `1e-6`. A wrong array or element type, or a short array,
is `InvalidMetadata`; a nonzero clamp selects `UnsupportedVariant`.

### Tensor name and shape binding

`ArtifactTensorBuilder` (`training/src/gguf_llama.rs:1111-1217`) is the single
tensor admission mechanism. `required` calls `capture` and maps absence to
`MissingTensor`; `optional` admits a present tensor or returns `None`;
`ignore_optional` validates and marks a known-but-unused tensor as consumed.
`capture` checks exact `GgufTensorType::F32` and exact GGUF dimensions, obtains
the parser's raw byte span, reverses the dimension vector for Recipe's graph
shape convention, and copies the bytes into `GgufLlamaTensorImage`. The
consumed-name set makes the final admission closed: `finish` rejects the first
archive tensor not explicitly admitted as a required, optional, or ignored
name.

The top-level names and contracts below list GGUF declaration dimensions. The
builder reverses each dimension vector for Recipe's row-major graph shape.

| GGUF tensor | GGUF dimensions and artifact role |
| --- | --- |
| `token_embd.weight` | F32 `[embedding_length, vocabulary]`; token lookup source. |
| `output_norm.weight` | F32 `[embedding_length]`; final RMSNorm scale. |
| `output.weight` | Optional F32 `[embedding_length, vocabulary]`; final projection. If absent, `output` aliases the token embedding index, implementing tied output weights. |
| `rope_freqs.weight` | Optional F32 `[rotary_dimension / 2]`; per-pair frequency factors. Values must be finite and nonzero. If absent, every factor is `1.0`. |

For each `blk.{block}` in `0..block_count`, the decoder requires the following
GGUF dimensions:

* `attn_norm.weight`: F32 `[embedding_length]`;
* `attn_q.weight`, `attn_k.weight`, `attn_v.weight`, and
  `attn_output.weight`: F32 `[embedding_length, embedding_length]`;
* `ffn_norm.weight`: F32 `[embedding_length]`;
* `ffn_gate.weight` and `ffn_up.weight`: F32
  `[embedding_length, feed_forward_length]`;
* `ffn_down.weight`: F32 `[feed_forward_length, embedding_length]`.

Each linear stem may also have a F32 bias `[output_width]` and F32 scale `[1]`.
The seven `*.input_scale` names may be present as F32 `[1]`; they are validated
and marked consumed but never captured or applied. Any other tensor, including
quantized variants, unexpected adapter tensors, or an extra Llama-specific
state image, is rejected by `finish` as an unsupported variant.

The corresponding graph shapes are the reversed vectors, for example
`token_embd.weight` becomes `[vocabulary, embedding_length]`, Q/K/V weights
become `[embedding_length, embedding_length]`, and `ffn_gate.weight` becomes
`[feed_forward_length, embedding_length]`.

`GgufLlamaLinear` stores only tensor indices for weight, optional bias, and
optional scale. `GgufLlamaBlock` stores the two RMSNorm indices plus the seven
linear descriptors. All block descriptors point into the one ordered image
vector, so graph compilation cannot accidentally load a name twice or use a
different tensor family for one projection.

## Token-stream preparation

`prepare_gguf_llama_inference_table` (`training/src/gguf_llama.rs:485-566`)
binds a selected `RawTable` to a decoded artifact. This is a distinct contract
from `.ogdl` feature-schema inference:

1. `table.width()` must be exactly one. A table with zero or more than one
   vector is `InvalidTokenStream`.
2. Every row must have a first field. The first field is treated as bytes and
   split on ASCII whitespace. Additional fields cannot occur after the width
   check.
3. Each nonempty token must be valid UTF-8, then pass
   `recipe_ingest::parse_contract_i32`. That parser accepts the exact bounded
   decimal int32 representation, not a host float conversion or a lossy
   integer cast.
4. Each value must be nonnegative and strictly less than
   `artifact.vocabulary`.
5. The aggregate stream must contain at least one token and its length must be
   no greater than `artifact.context_length`.

The resulting `Vec<i32>` preserves row order and token order within each
whitespace-separated field. It is not retokenized, padded, split into batches,
sampled, or associated with a tokenizer vocabulary. A malformed UTF-8 field,
empty row, decimal parse failure, negative or out-of-vocabulary ID, empty
stream, or context overflow is reported before graph compilation. There is no
KV cache, cross-call state, host tokenizer, or user-facing context override.

The checked acceptance corpus uses one `tokens.txt` vector containing exactly
128 IDs, but the implementation accepts any nonempty stream within the saved
context and int32/vocabulary bounds.

## Graph compilation

### External boundary and identity

`compile_prepared_gguf_llama_inference` starts at
`training/src/gguf_llama.rs:568`. It creates one `InferenceGraphCompiler` and
converts the token count to `u64`. `InferenceGraphCompiler::external`
(`training/src/inference.rs:1851-1898`) validates that every byte image exactly
matches `shape.bytes(dtype)`, allocates a canonical contiguous row-major tensor,
records the semantic `InferenceInputRole`, and marks the value as an immutable
external input.

The GGUF compiler admits:

| Role | Dtype and shape | Bytes |
| --- | --- | --- |
| `GgufTokenIds` | `I32 [sequence]` | Prepared token IDs in little-endian int32 order. |
| `GgufTensor { tensor }` | `F32` with the captured tensor's reversed graph shape | Exact copied raw F32 tensor image. One input is created for every index in `execution_tensor_indices`. |
| `GgufRopePartnerIndices` | `I32 [sequence * heads * head_dimension]` | Prepared adjacent-pair gather indices. |
| `GgufRopeCosines` | `F32` with the same flat shape | Prepared cosine bits, duplicated for the two coordinates in each pair. |
| `GgufRopeSignedSines` | `F32` with the same flat shape | Prepared `[-sin, +sin]` bits for each adjacent pair. |

The generic executor treats all of these roles as allowed inference inputs
(`training/src/execute.rs:1835-1906`). It rejects duplicate roles or values,
any declared role not present in the graph, byte-size mismatch, dtype/shape
mismatch, noncanonical layout, or an input that is also the prediction output.

### Entry lookup and embedding

The compiler first emits a `Gather` with `axis = 0` and
`IndexBounds::Reject` (`training/src/gguf_llama.rs:580-611`). It gathers each
token ID from `token_embd.weight` into `current` with shape
`[sequence, embedding_length]`. Bounds are checked by the primitive at runtime,
and preparation has already proven the IDs are in vocabulary range.

### RoPE table construction

`prepare_rope_inputs` (`training/src/gguf_llama.rs:762-856`) prepares all
position/head/pair tables during graph compilation, not inside the loop:

* `rotary_half = head_dimension / 2`.
* Captured `rope_freqs.weight` bytes are decoded as little-endian F32 factors;
  absent factors use a generated vector of `1.0` values. Decoder validation
  proves every captured factor is finite and nonzero.
* `rows = sequence * heads` and
  `elements = rows * head_dimension` use checked products, must fit int32
  indexing, and must fit the host `usize` used for vector capacities.
* `theta_scale = rope_base ^ (-2 / head_dimension)`.
  `frequency_scale` and `attention_factor` come from the artifact bits.
* For each `row`, `token = row / heads`, and each pair, the compiler computes
  `angle = frequency_scale * theta / factor`, then stores
  `cos(angle) * attention_factor` and `sin(angle) * attention_factor`.
  `theta` is multiplied by `theta_scale` after each pair.
* For a pair whose flat base is `row * head_dimension + pair * 2`, partner
  indices are `[base + 1, base]`, cosine bits are duplicated, and signed-sine
  bits are `[-sine, sine]`.

The three vectors are admitted as external inputs with roles listed above. The
materialized `gpu_rope_partial` composition consumes them with parameters
`rows`, `head_dim`, `rotary_dim`, `heads_per_token`, `theta`, and the verified
fact `rotation_tables_verified = true` (`training/src/gguf_llama.rs:918-978`).
The operation registry dispatches this symbol to the Recipe-owned attention
materializer (`ops/src/materialize/attention_sequence_embedding.rs:38-66`).
That materializer requires F32 values/cosines/signed sines, I32 partner indices,
identical flat shapes, an even rotary dimension no greater than head dimension,
finite positive theta, and the true verification fact
(`ops/src/materialize/attention_sequence_embedding.rs:467-556`). It gathers
partner values with reject bounds and applies the owned rotation scalar program
`rotated = value * cosine + partner * signed_sine`
(`ops/src/materialize/attention_sequence_embedding.rs:788-816`).

The current instrument always supplies `rotary_dim = head_dimension`, so the
decoder's full-head gate and the materializer's partial-RoPE shape checks agree.

### One transformer block

The block loop (`training/src/gguf_llama.rs:613-723`) is ordered exactly as the
saved block vector. Each iteration performs the following calculations.

#### Attention normalization and Q/K/V

`rms_norm` (`training/src/gguf_llama.rs:1054-1090`) requires input
`F32 [sequence, embedding_length]` and scale `F32 [embedding_length]`, creates
an equal-shape output, and materializes `gpu_rmsnorm`. It passes the saved
epsilon bits and `MAXIMUM_REDUCTION_TREE_LANES` as prepared parameters.
The Recipe-owned materializer squares each value, reduces a fixed-tree mean
square over the row, adds the positive epsilon, takes a checked reciprocal
square root, and multiplies by the saved scale. In formula form its scaled
forward value is `x * rsqrt(sum(x^2) / embedding_length + epsilon) * scale`
(`ops/src/materialize.rs:1688-1745`, `2936-3000`).

`linear` (`training/src/gguf_llama.rs:980-1035`) checks its input and weight
contracts, emits a `Contraction` with contract axis `(1, 1)` for a weight laid
out `[output_width, input_width]`, and then applies optional scale, optional
bias, and optional symmetric clamp. For Q, K, and V, `scale_before_bias` is
true, so the sequence is contraction, scale, bias, then clamp. Their output
widths are `embedding_length`, and `clamp_kqv_bits` is applied. The attention
output projection uses the same scale-before-bias order but clamp zero.

The optional scale is always required to be F32 `[1]`; it is broadcast by the
elementwise multiply. Biases are F32 `[output_width]` and are broadcast by the
owned add program. A nonzero clamp constructs one scalar program with
`maximum(value, -limit)` followed by `minimum(value, limit)`
(`training/src/gguf_llama.rs:1092-1100`).

#### Rotary causal attention

`causal_attention` (`training/src/gguf_llama.rs:858-916`) first applies the
prepared RoPE tables to Q and K. Each is reinterpreted as
`[1, sequence, heads, head_dimension]`; the values tensor is reinterpreted to
the same shape without changing its element count. Q/K contraction emits
scores with shape `[1, heads, sequence, sequence]`, sharing batch and head axes
and contracting the final head-dimension axis. The scores are multiplied by
`attention_scale_bits`.

`InferenceGraphCompiler::causal_softmax`
(`training/src/inference.rs:4175-4212`) reinterprets scores to a matrix with
`attention_rows = 1 * heads * sequence` rows and `sequence` columns, creates
an identity index map, and lowers `causal_mask_program(sequence)` to an I32
mask. It materializes `gpu_causal_softmax_rows` with the values, mask, fixed
reduction-tree lane count, and `causal_mask_verified = true`. The operation
materializer (`ops/src/materialize/attention_sequence_embedding.rs:321-417`)
applies `-1e30` to masked score positions, performs max-subtracted
exponentiation, zeros masked exponentials, sums each row with the fixed
reduction tree, and divides by the positive row sum. The result is
reinterpreted back to
`[1, heads, sequence, sequence]`.

Probabilities contract with the head-major values on the sequence axis,
producing context `[1, heads, sequence, head_dimension]`. `head_major_to_sequence`
(`training/src/inference.rs:4139-4173`) lowers a checked index map and gather
to reorder it to `[1, sequence, heads, head_dimension]`, then the compiler
reinterprets it as `[sequence, embedding_length]`.

#### Attention residual and parallel SwiGLU

The attention context passes through `attn_output`, then `exact_add` adds the
pre-attention residual. `exact_add` requires equal dtype and shape and emits an
elementwise add. The resulting matrix is saved as the feed-forward residual,
then normalized again with `ffn_norm.weight`.

`ffn_gate` and `ffn_up` both consume that same normalized matrix. Their linear
calls use `scale_before_bias = false`, so an optional bias is added before an
optional scale. The gate output receives Recipe's `DenseActivation::Silu`, the
activated gate is multiplied elementwise by the up output, and the product is
passed through `ffn_down`. A second exact residual add completes the block.

This is parallel SwiGLU in the sense that gate and up projections share one
normalized input. The decoder rejects `llama.use_parallel_residual = true`, so
the graph does not implement a separate parallel-residual block variant.

### Final RMSNorm and logits

After all blocks, the compiler applies `output_norm.weight` through
`rms_norm`, then emits a final `Contraction` between
`current [sequence, embedding_length]` and the selected output weight
`[vocabulary, embedding_length]` (`training/src/gguf_llama.rs:724-742`). The
output is a canonical F32 tensor `[sequence, vocabulary]`.

`compiler.finish` is called with:

```text
prediction kind: InferencePredictionKind::TokenLogits
task: InferenceTask::TokenLogits { vocabulary }
rows: sequence
target dtypes: [DType::I32]
output adapter: None
```

There is no softmax, argmax, sampling, loss, target, temperature, metric, or
host conversion in this model-specific graph. The output is the raw logit
matrix for every input position.

## Generic graph and operation invariants

The GGUF compiler uses the existing inference graph machinery rather than a
parallel execution ontology.

### Canonical tensors and static program

`InferenceGraphCompiler::tensor` creates contiguous row-major tensors with
fresh deterministic `ValueId`s. `emit` creates `PrimitiveKernel` nodes with
fresh `KernelTemplateId`s and `IterationDomain::first()`; all aliases are
forbidden for GGUF intermediate operations. `materialize` reserves a fixed
identity range, invokes `recipe_ops::materialize_composition`, merges its
tensor contracts and nodes, and rejects conflicting contracts
(`training/src/inference.rs:1934-2077`).

`finish` (`training/src/inference.rs:4619-4658`) marks only the admitted
external images as graph inputs and only the logits tensor as an external
output. It validates the calculation graph, serializes and reparses canonical
OGDL, creates a `StaticCalculationProgram` with exactly one nonzero iteration,
serializes and reparses that program, and returns `CompiledInference` with the
typed external-input list and output contract. A graph serialization failure,
operation materialization failure, tensor-contract conflict, shape failure,
identity exhaustion, or static-program error is an
`InferenceCompileError` with the corresponding kind.

### Shape, byte, and index limits

The generic helpers in `training/src/inference.rs:4717-4747` check shape
construction, checked `u64` products, and conversion to int32 index values.
GGUF-specific graph preparation additionally checks:

* token count conversion to `u64`;
* `sequence * heads`, `rows * head_dimension`, and all matrix extents;
* RoPE table capacity conversion to `usize`;
* RoPE partner indices and element count in the int32 domain;
* output and projection extents through `Shape::new` and `shape.bytes`; and
* tensor byte images before any external input is registered.

No graph value is admitted with a silently truncated extent or byte count.

### Native boundary validation

`training/src/execute.rs:1444-1645` validates a compiled GGUF inference before
device images are built:

* program iterations must be exactly one and program metrics must be empty;
* the calculation graph must validate;
* every declared role must be an allowed inference role, including all four
  GGUF roles;
* roles and `ValueId`s must be unique, each input byte count must match its
  typed shape, and each input must be a canonical external graph tensor;
* declared input values must equal the complete graph-input set;
* exactly one canonical F32 graph tensor may be an external output, it must be
  produced by a calculation node, and it must equal the output contract; and
* `InferenceTask::TokenLogits { vocabulary }` must agree with
  `InferencePredictionKind::TokenLogits` and shape
  `[inference.rows(), vocabulary]`.

The boundary also rejects an input/output alias, loop metric, loop external
transfer, or any output with a wrong dtype, source, byte count, or shape. These
checks are independent of the decoder and protect the planner/executor handoff
from a malformed compiled object.

`build_inference_device_images` (`training/src/execute.rs:1184-1196`) copies
every declared external input into the finalized `init` image manifests. It
rejects duplicate devices or members, unexpected or unbound values, dtype and
size differences, image overlap, and out-of-bounds offsets. GGUF bytes enter
the device only through this init admission; there is no loop ingress.

## Native execution and report

### Measured preparation and lifecycle

`src/inference.rs:602-659` sends every `CompiledModelInference` family through
the same measured native path. For GGUF it:

1. obtains a fresh run identity;
2. enters `with_current_native_preparation`, which supplies the measured
   profile, machine-derived tuning, backend bindings, and deferred compiler;
3. derives host worker and staging tuning from the GGUF calculation graph;
4. constructs the production cross-backend bridge, candidate factory, native
   driver, realizer, and `Preparer`;
5. calls `prepare_and_execute_local_inference` with the one-iteration limits.

`prepare_and_execute_local_inference` (`training/src/execute.rs:1198-1310`)
revalidates the graph, prepares and finalizes the native bundle, and requires
`LoopIterations::ONE`. It rejects any finalized loop external transfer or user
metric. It maps exactly one finalized exit transfer to the logits contract,
retains the realized native images, creates a recoverable `PreparedRun`, and
uploads the packed init images. The executor starts the loop and polls until
`LoopStatus::Complete`, resetting or waiting on the blocking poll backoff as
progress is observed. A one-iteration wait that cannot enter the exited-loop
state is a lifecycle invariant failure.

After loop completion, the executor runs `exit_recoverable`, obtains the
finalized external egress images, and collects the one prediction image. The
timed interval is the completed loop only. Native preparation, graph
compilation, image realization, init admission, output publication, and
teardown are outside the throughput interval but remain represented in native
evidence and the ordered `RunJournal`. Native resources are destroyed before
`CompletedInferenceExecution` is returned.

### Exit mapping and prediction bytes

`map_inference_output` (`training/src/execute.rs:2514-2634`) requires exactly one
planned output. It verifies that the plan names the compiled logits `ValueId`,
is a `TransferEndpoint::External` in `RunPhase::Exit`, comes from the planned
device/value pair, has the expected F32 dtype and byte count, and does not
overlap another exit image. Missing, extra, duplicate, or mismatched output
tasks are execution failures.

`collect_inference_prediction` and
`validate_completed_prediction_images`
(`training/src/execute.rs:2831-2921`) repeat those checks on actual exit images
after teardown. They require one image, one task, the expected source location,
F32 dtype, and exactly `shape.bytes(dtype)` bytes. The bytes are copied as an
opaque little-endian image into `InferencePrediction`; Recipe does not compute
or reinterpret logits on the host in the execution layer.

### Public report shape

`InferenceReportPayload::GgufLlama` stores the `GgufLlamaArtifact` and the
completed execution (`src/inference.rs:148-189`). `InferenceReport::kind()` is
`InferenceModelKind::GgufLlama`; `prediction()` returns the singular raw-logit
prediction; `native_kernels()`, `native_evidence()`, `journal()`, `devices()`,
`run()`, `bundle()`, and `elapsed()` expose the common completed-lifecycle
evidence. Dense label and Bayesian decoding accessors return `None` or zero for
GGUF because a token-logit matrix has no saved class dictionary or target
schema.

`InferenceReport::values()` (`src/inference.rs:297-307`) is an exact-size
iterator over validated four-byte little-endian F32 values. The report is
created only after the native loop, exit transfer, image validation, and
ordered teardown complete.

`write_gguf_llama_prediction_rows` (`src/inference.rs:780-843`) is the public
text-report adapter. It verifies:

* prediction kind is `TokenLogits`;
* the contract is rank two `[positions, vocabulary]`;
* contract width equals `artifact.vocabulary()`;
* vocabulary width and `positions * vocabulary * 4` fit host `usize`; and
* the validated byte image has exactly that size.

For each position it chooses the lowest-index maximum logit with the shared
`multiclass_argmax` helper and prints one line:

```text
prediction    <position>    token    <argmax-token-id>    logit    <raw-logit>
```

This line is reporting, not model calculation. The graph still returns every
raw logit for every position, and no token is sampled or fed back into the
model. `Time` and `Device` lines, when declared, are printed after prediction
collection and teardown. A report write failure is an `InferenceError::Runtime`
failure at the `write inference report` stage.

## Error boundaries

The same unsupported or malformed condition is not converted into a fallback
model. The observed error chain is:

```text
declaration failure
    -> InferenceError::Declaration or InferenceError::Unsupported
source/data framing failure
    -> InferenceError::Data
source snapshot or GGUF decoder failure
    -> InferenceError::Model
graph shape, operation, or static-program failure
    -> InferenceError::Compile
native profile, planner, realization, or backend handoff failure
    -> InferenceError::Native
executor, lifecycle, image, or post-exit contract failure
    -> InferenceError::Execute
report stream failure
    -> InferenceError::Runtime
```

The model-specific classes are intentionally explicit:

* `Container` preserves the parser's GGUF error text, including file-limit,
  truncation, invalid magic/version/endian, metadata/tensor limits, invalid
  UTF-8 or keys, duplicate entries, unsupported encoded types, bad dimensions,
  offsets, overlap, nonzero padding, trailing data, and arithmetic overflow.
* `UnsupportedArchitecture` identifies a valid non-Llama GGUF architecture.
* `UnsupportedVariant` identifies a valid Llama file whose geometry, scaling,
  residual, expert, causal, clamp, tensor, or unconsumed-state contract is
  outside this instrument.
* `MissingMetadata` and `MissingTensor` identify omitted required state.
* `InvalidMetadata` identifies wrong encoded types and malformed required or
  optional values.
* `InvalidTensor` identifies wrong F32/shape contracts, absent raw spans, and
  invalid saved RoPE factors.
* `InvalidTokenStream` identifies the separate data-side sequence contract.
* `ArithmeticOverflow` identifies a model count that cannot become host-owned
  artifact state.

Compile and execute errors remain visible after model decoding. Examples
include a token count that cannot become a graph extent, a graph tensor whose
typed byte image disagrees with its shape, an operation ABI mismatch in
`gpu_rope_partial`, `gpu_rmsnorm`, or `gpu_causal_softmax_rows`, a materialized
tensor-contract conflict, exhausted deterministic identity space, a malformed
static program, an input not admitted to every finalized init image, a loop
external transfer, missing or duplicate logits egress, wrong exit source, and
prediction byte-size or dtype mismatch.

## Acceptance and end-to-end proof

The hardware acceptance runner exercises the same public boundary rather than
calling `decode_gguf_llama` or a private compiler function. Its Llama gate is
`acceptance/src/main.rs:1122-1226`:

* inputs are `llamacpp-archs-seed42/tokens.txt`,
  `llama-dense.gguf`, and the checked-in `llama-dense.logits` reference;
* the reference is an `LGT0` little-endian file declaring 128 positions and
  128 vocabulary values and containing exactly `128 * 128` F32 values;
* every Recipe invocation performs `recipe.data(tokens)`,
  `recipe.model().load(model)`, and
  `recipe.infer().log([Time, Device]).evaluate()`;
* the report must dispatch as `InferenceModelKind::GgufLlama`, use one GPU
  device, complete one lifecycle iteration, expose one nonempty CUDA Cubin,
  and provide complete native evidence and an exit logits image;
* a warm-up and seven measured Recipe runs are checked against the reference;
* `verify_logits` computes
  `sum((actual - reference)^2) / sum(reference^2)` and requires finite NMSE
  strictly below `1e-3`; and
* positive finite elapsed times become tokens/second. Seven samples are sorted
  and compared by median against seven interleaved pinned llama.cpp oracle
  samples. Recipe fails if its median is lower.

The pinned oracle (`acceptance/oracle/main.cpp:116-198`) loads all GGML
backends, requires exactly one GPU, loads all model layers to the GPU, creates
a 128-token context and batch, performs one warm-up decode, clears memory, and
times one decode plus collection of all positions. It is a comparison process,
not a Recipe execution path. The gate therefore proves both the public Recipe
dispatch/lifecycle and numerical parity against the pinned oracle without
making the oracle an implementation dependency.

## Deliberate non-support and boundaries

The current source does not infer, convert, or emulate any of the following:

* GGUF v2, big-endian v3, or a non-Little F32 image at the executable boundary;
* architectures other than the exact `general.architecture = "llama"` string;
* F16, BF16, integer, or quantized weight tensors;
* grouped-query attention, unequal key/value widths, partial or odd rotary
  dimensions, noncausal attention, or parallel-residual blocks;
* mixture-of-experts tensors, nonzero per-block SwiGLU clamp state, unsupported
  RoPE scaling types, or nonzero YaRN extrapolation;
* a tokenizer, chat template, sampling policy, padding, batching, KV cache,
  recurrent state, or cross-call model state; or
* GGUF model training, optimizer updates, semantic `.ogdl` export, or generic
  GGUF execution through the structural converter.

Those are separate contracts. The first executable GGUF case remains a
bounded, explicit instrument whose graph and tensor admission are fully
observable in the source listed below.

## Source map

| Concern | Implementation | Key lines |
| --- | --- | --- |
| Model declaration | `src/api.rs` | `1008-1059`, `1462-1476` |
| Infer terminal | `src/api.rs` | `2181-2240` |
| Public extension dispatch and native handoff | `src/inference.rs` | `482-659` |
| Target-free source distillation/selection | `src/data_prepare.rs` | `97-129` |
| Bounded source snapshot | `ingest/src/source.rs` | `34-177` |
| Generic GGUF parser/archive | `ingest/src/gguf.rs` | `20-77`, `414-503`, `874-1267` |
| Llama errors and artifact | `training/src/gguf_llama.rs` | `25-184` |
| Llama metadata/tensor decode | `training/src/gguf_llama.rs` | `186-476`, `1111-1343` |
| Token binding | `training/src/gguf_llama.rs` | `485-566` |
| Graph lowering | `training/src/gguf_llama.rs` | `568-753` |
| RoPE preparation and attention | `training/src/gguf_llama.rs` | `762-978` |
| Linear and RMSNorm helpers | `training/src/gguf_llama.rs` | `980-1100` |
| Generic graph compiler | `training/src/inference.rs` | `1851-2077`, `4088-4658` |
| Native input/output boundary | `training/src/execute.rs` | `1184-1310`, `1444-1645`, `2514-2634`, `2831-2921` |
| Recipe-owned RoPE/RMSNorm/causal softmax materializers | `ops/src/materialize/attention_sequence_embedding.rs`, `ops/src/materialize.rs` | `321-417`, `419-617`, `1688-1745` |
| Public logit report | `src/inference.rs` | `661-843` |
| Hardware parity gate | `acceptance/src/main.rs`, `acceptance/oracle/main.cpp` | `1122-1407`, `116-198` |
