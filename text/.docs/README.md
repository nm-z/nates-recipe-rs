# `recipe-text`

`recipe-text` is Recipe's bounded, host-side text preparation crate. It owns
the transformations that must happen before text becomes a calculation input:

```text
raw text or chat messages
        |
        +--> render_template -> rendered text
        |                         |
        +-------------------------+
                                  v
                     Tokenizer::encode / prepare_batch
                                  |
                                  v
                 checked int32 token IDs and masks
                                  |
                                  v
                   one caller-owned init admission
```

The crate never scores tokens, performs model arithmetic, changes a GPU
payload, reads a file during the execution loop, or retains a file handle. It
is preparation metadata, not a tokenizer-backed inference engine. A caller
must still move the resulting IDs into the ingest and training or inference
contracts that own the model graph.

This README describes the implementation that is present in
[`text/src/lib.rs`](../src/lib.rs). It is intentionally separate from the
normative workspace contracts in [`system-contract.md`](../../system-contract.md),
[`API.ogdl`](../../API.ogdl), and [`operation-surface.txt`](../../operation-surface.txt).

## Package boundary

[`text/Cargo.toml`](../Cargo.toml) declares package `recipe-text`, version
`0.1.0`, Rust edition 2024, MIT licensing, and `publish = false`. The manifest
forbids unsafe code and denies all and pedantic Clippy lints. It has no binary,
examples, feature flags, build script, or test module. The implementation is a
single public facade in `src/lib.rs`; the private helpers in that file are the
only construction and validation paths.

The dependency boundary is deliberately small:

| Dependency | Use in this crate |
| --- | --- |
| `tokenizers` `0.21` with default features disabled and `fancy-regex` enabled | Owns the inner tokenizer object, JSON deserialization, BPE implementation, byte-level and metaspace pre-tokenizers, decoders, and token ID conversion. |
| `hf-chat-template` `0.2` with `strftime` enabled | Compiles and renders Hugging Face Jinja chat templates. Its `minijinja` context receives the message list and generation metadata. |
| `recipe-ingest` | Provides the one bounded `SourceLimit` and `read_source_snapshot` call used by `Tokenizer::from_file`. The file is copied and closed before parsing returns. |

The crate does not depend on `recipe-training`, `recipe-executor`, a native
backend, or a model graph crate. That dependency direction is intentional:
text preparation can be reused by a preparation caller without importing GPU
execution.

## Public access and current callers

The root package includes `recipe-text` as a workspace member and dependency.
[`src/facade.rs`](../../src/facade.rs) exposes it under the advanced-caller
namespace:

```rust
use recipe::engine::text::{Message, TextLimits, Tokenizer};
```

There is no root-level `recipe::text` module and no fluent `.tokenize()` or
`.chat()` method in the public declaration builders. A repository-wide source
trace finds no call to `Tokenizer::from_json`, `from_file`, `from_vocabulary`,
`encode`, `decode`, `prepare_batch`, `decode_batch_row`, or
`render_template` outside this crate. Thus the API is available to an
advanced caller, but it is not currently wired into the root training or
inference command paths.

The operation inventory has a separate compatibility classification in
[`ops/src/non_calculation.rs`](../../ops/src/non_calculation.rs):

| Inventory symbol | Classification | Definition |
| --- | --- | --- |
| `encode` | `NonCalculationRecipe::TextTokenization` | Deterministic raw text to checked int32 token-ID metadata before payload admission. |
| `render_chat`, `render_template` | `NonCalculationRecipe::ChatTemplateRendering` | Deterministic host rendering before tokenization and payload admission. |

Those entries are descriptors in the source-qualified registry. They do not
dispatch into `recipe-text`; their inventory provenance still points at the
historical `pantry/src/bpe.rs` and `recipe-infer/src/chat.rs` paths in
[`operation-surface.txt`](../../operation-surface.txt), which are not current
workspace crates. The classification prevents these host transformations from
being mistaken for GPU calculations or a CPU calculation fallback.

## Module shape and data flow

The module graph is intentionally flat:

```text
text/Cargo.toml
└── text/src/lib.rs
    ├── limits and shared errors
    ├── tokenizer metadata and identities
    ├── fixed-width token batch contracts
    ├── tokenizer construction, encoding, decoding, and batch preparation
    ├── BPE vocabulary adapters
    └── chat message/template rendering
```

All public state that can cross this crate's boundary is owned and immutable
after construction. Borrowed `VocabularySpec` and `Message` inputs are copied
where needed. `Tokenizer` owns the parsed `tokenizers::Tokenizer`, its
immutable `TextLimits`, and a canonical `TokenizerIdentity`. `TextBatch` owns
boxed flat arrays and a cloned batch specification. No public value stores a
path or external file handle, and no method performs a later filesystem read.

The normal preparation sequence is:

1. Build one nonzero `TextLimits` value.
2. Construct a tokenizer from a bounded JSON snapshot, a bounded file
   snapshot, or already framed vocabulary metadata.
3. Optionally render a chat message list with `render_template`.
4. Encode each raw string, or call `prepare_batch` to encode and lay out a
   fixed-width batch in one operation.
5. Pass the resulting `i32` IDs and explicit validity mask to a caller that
   owns the model input and init transfer.

Rendering and tokenization are separate operations. `render_template` returns
raw `String` data; it never calls `Tokenizer::encode` implicitly. Likewise,
`Tokenizer::decode_batch_row` removes padding using the stored mask and then
delegates to the tokenizer decoder; it does not reconstruct text from token
ID values by itself.

## Public types

### `TextLimits`

`TextLimits` is a `Copy` value containing nine independent, nonzero bounds:

| Getter | Bound checked |
| --- | --- |
| `model_bytes()` | Input bytes accepted by `from_json`, and the `SourceLimit` used by `from_file`. |
| `input_bytes()` | Raw text passed to `encode`, and each `Message::content` passed to `render_template`. |
| `output_tokens()` | Encoded output length, decoder input length, and the fixed sequence length accepted by `prepare_batch`. |
| `vocabulary_entries()` | Parsed vocabulary size, including added tokens reported by the inner tokenizer. |
| `aggregate_piece_bytes()` | Sum of all token-piece byte lengths in a `VocabularySpec`. |
| `merge_entries()` | Number of explicit BPE merge strings in a `VocabularySpec`. |
| `template_bytes()` | UTF-8 byte length of the chat template source. |
| `messages()` | Number of chat messages. |
| `rendered_bytes()` | Decoded text length and rendered chat output length. |

`TextLimits::new` converts every argument to `NonZeroUsize`. A zero value
returns `TextErrorKind::InvalidLimit` naming the bound. The limits have no
implicit defaults and no relationships are inferred between them. In
particular, the caller chooses the model, text, vocabulary, template, and
batch bounds explicitly.

### Vocabulary metadata

`VocabularyKind` selects one of two adapters:

* `BytePair` expects an explicit merge table and no score metadata.
* `SentencePieceBpe` expects one finite IEEE-754 f32 score bit pattern per
  token, no explicit merges, and an in-range unknown-token ID.

`VocabularySpec<'a>` borrows the already framed token strings, merge strings,
score bit patterns, and optional unknown ID. The token slice order is the
numeric token ID order. The implementation clones pieces while constructing
the owned inner tokenizer, so the caller may release the borrowed metadata
after `from_vocabulary` returns.

### Errors

`TextErrorKind` is a non-exhaustive, `Copy` classification. `TextError` stores
the public `kind` and a human-readable `detail`; it implements `Display` as
`<Debug-kind>: <detail>` and implements `std::error::Error`. `TextResult<T>`
is the crate-wide result alias.

The variants are:

| Kind | Produced by |
| --- | --- |
| `InvalidLimit` | A zero `TextLimits` or `TextBatchSpec` bound. |
| `LimitExceeded` | Any byte, token, vocabulary, merge, message, sequence, or rendered-output bound exceeded. |
| `InvalidModel` | Malformed tokenizer JSON or failure to serialize its canonical identity. |
| `InvalidVocabulary` | Empty or duplicate pieces, malformed BPE merges, incompatible score/merge fields, nonfinite scores, or an inner BPE construction failure. |
| `InvalidTokenId` | Negative role IDs, an ID outside the tokenizer vocabulary, a token ID above `i32::MAX`, or a negative ID supplied to `decode`. |
| `InvalidBatch` | Empty input to `prepare_batch` or a row index outside a prepared batch. |
| `TokenizerMismatch` | `decode_batch_row` receives a batch whose canonical identity differs from the decoder. |
| `Tokenization` | The inner tokenizer rejects an input during encoding. |
| `Decode` | The inner tokenizer rejects checked IDs during decoding. |
| `InvalidMessage` | A chat role is empty or message content exceeds the input-byte bound. |
| `Template` | Chat-template compilation or rendering fails. The detail identifies `compile` or `render`. |
| `Source` | `SourceLimit` creation or `read_source_snapshot` fails in `from_file`. |
| `ArithmeticOverflow` | A host-size conversion, aggregate piece sum, fixed-batch product, row offset, or source-read bound cannot be represented. |

There are no fallback tokenizers, retries, unknown-ID substitutions, or
alternate file paths. The first concrete failure is returned.

### `TokenizerIdentity`

`TokenizerIdentity` wraps an `Arc<str>` containing the complete canonical
serialization returned by `tokenizers::Tokenizer::to_string(false)`. Equality
therefore covers the vocabulary, added-token metadata, normalizer,
pre-tokenizer, and decoder, rather than a lossy hash or process-local pointer.
The canonical string is private; callers compare identities through `PartialEq`
and receive only a byte count in `Debug`. Every `TextBatch` stores a clone of
this identity so a row cannot be decoded by a differently configured tokenizer
without an explicit `TokenizerMismatch` error.

### Token roles and batch contracts

`TextTokenIds` stores a pad ID, an optional unknown ID, and a boxed list of
special IDs. `TextTokenIds::new` rejects negative IDs but permits role overlap.
For example, a model may use its end-of-sequence ID as padding. During
`prepare_batch`, `Tokenizer::validate_token_roles` additionally requires every
role ID to resolve through the actual tokenizer vocabulary. Validity is never
inferred from the numeric value of a role.

`TextBatchSpec` stores:

* nonzero `max_sequences`;
* nonzero fixed `sequence_length`;
* `PaddingSide::Left` or `Right`;
* `TruncationPolicy::Reject`, `KeepStart`, or `KeepEnd`;
* whether `encode` should add model special tokens; and
* the checked `TextTokenIds` contract.

Construction checks `max_sequences * sequence_length` with
`checked_mul`, returning `ArithmeticOverflow` if the maximum flat layout does
not fit `usize`. `TextBatchLayout` currently has one variant,
`BatchMajor`: flat index `batch * sequence_length + position`.

`TextBatch` owns the resulting metadata:

| Field | Meaning |
| --- | --- |
| `tokenizer_identity()` | Exact tokenizer configuration that assigned the IDs. |
| `spec()` | The immutable padding, truncation, role, and dimension contract. |
| `sequences()` and `shape()` | Number of rows and `[rows, sequence_length]`. |
| `token_ids()` | Flat batch-major `i32` IDs, including explicit pad positions. |
| `attention_mask()` | Flat `i32` values containing only `0` and `1`; one marks a retained encoded token, zero marks padding. |
| `original_lengths()` | Encoded length before truncation for each row. |
| `retained_lengths()` | Number of encoded tokens retained in the fixed row. |

`row(index)` returns a borrowed `TextBatchRow` or `None` for an out-of-range
index or checked offset failure. The row exposes slices of IDs and mask plus
the two lengths. It does not allocate or copy the row.

## Tokenizer construction

### JSON snapshot: `Tokenizer::from_json`

`from_json(bytes, limits)` first checks `bytes.len()` against
`model_bytes`. It then calls `tokenizers::Tokenizer::from_bytes`. A malformed
JSON model is reported as `InvalidModel`. The resulting vocabulary size,
including added tokens, must fit `vocabulary_entries` and must be at most
`i32::MAX`; otherwise construction returns `LimitExceeded` or
`InvalidTokenId`. Finally, the canonical identity is serialized and stored.

The input slice is borrowed only during construction. The returned tokenizer
owns all state required for later encode, decode, and batch operations.

### File snapshot: `Tokenizer::from_file`

`from_file(path, limits)` converts the model-byte bound to `u64`, constructs an
`ingest::SourceLimit`, and calls
[`read_source_snapshot`](../../ingest/src/source.rs). The ingest boundary opens
one regular file, rejects a metadata size above the limit, reads at most
`limit + 1` bytes to detect a concurrent growth race, computes the content
digest, and closes the handle before returning. `from_file` then passes the
snapshot bytes through exactly the same `from_json` path. Source I/O,
non-regular paths, growth, or source-limit failures become `TextErrorKind::Source`.
No digest or path is retained in `Tokenizer`.

### Vocabulary metadata: `Tokenizer::from_vocabulary`

The adapter validates the borrowed `VocabularySpec` before constructing a
tokenizer:

1. The token list must be nonempty and within the vocabulary, aggregate-piece,
   and merge-entry bounds.
2. The aggregate UTF-8 byte sum is checked with `checked_add`.
3. Token pieces must be unique. Their slice positions become IDs starting at
   zero, and the number of entries must fit the checked `int32` domain.
4. A `BytePair` spec must have an empty `score_bits` slice.
5. A `SentencePieceBpe` spec must have one score per token, no explicit
   merges, finite scores, and an unknown ID inside the token slice.

The constructed inner tokenizer is validated again for final vocabulary size
and canonical identity. The `unknown_token_id` field is intentionally ignored
for `BytePair`; only the SentencePiece flavor requires and validates it.

#### Byte-pair flavor

`byte_pair_tokenizer` parses each merge string by splitting on a literal ASCII
space. Exactly two nonempty pieces are required, with no third field or empty
piece. The pairs are passed to `tokenizers::models::bpe::BPE`, then a
`ByteLevel` pre-tokenizer and decoder are installed with the crate's fixed
configuration (`false, true, true`). Inner construction errors are
`InvalidVocabulary`.

#### SentencePiece BPE flavor

`sentencepiece_bpe_tokenizer` derives candidate merges instead of accepting an
explicit merge list. For every token and every character boundary after its
first character, it considers the left and right substrings when both are
already present in the vocabulary. Candidates carry the token's score bits,
are sorted by descending `f32::total_cmp` score, then left and right lexical
order, and are supplied to the same BPE model. The validated unknown piece is
installed with unknown-token, byte-fallback, and fused-unknown behavior.

The pre-tokenizer is metaspace with marker `U+2581` (`▁`) and the
`PrependScheme::First` scheme. The decoder is a fixed sequence:

```text
Replace("▁", " ")
  -> ByteFallback
  -> Fuse
  -> Strip(' ', 1, 0)
```

Nonfinite score bits are rejected before this path. The implementation keeps
the original score bit patterns until conversion to `f32` for validation and
ordering; no model file or external metadata handle is retained.

## Encoding, decoding, and fixed batches

### `Tokenizer::encode`

`encode(text, add_special_tokens)` checks the UTF-8 byte length against
`input_bytes`, calls the inner tokenizer, and checks the returned token count
against `output_tokens`. Each `u32` inner ID is converted to `i32`; a value
that does not fit is `InvalidTokenId`. The result is a newly owned `Vec<i32>`.

The method does not truncate, pad, score, or validate role IDs. Those are
separate batch and caller contracts.

### `Tokenizer::decode`

`decode(ids, skip_special_tokens)` applies the same `output_tokens` bound to
the input ID count, rejects every negative ID during conversion to `u32`, and
delegates to the inner decoder. The decoded UTF-8 string must fit
`rendered_bytes`. The `skip_special_tokens` flag is passed unchanged to the
inner tokenizer.

### `Tokenizer::prepare_batch`

`prepare_batch(texts, spec)` is the only operation that constructs a
`TextBatch`:

1. An empty slice is `InvalidBatch`; the row count must fit
   `spec.max_sequences`.
2. `spec.sequence_length` must fit the tokenizer's `output_tokens` limit.
3. Every pad, unknown, and special ID must be nonnegative and present in the
   actual vocabulary.
4. The flat element count `texts.len() * sequence_length` is checked before
   allocating IDs and masks. IDs start filled with the pad ID and masks start
   filled with zero.
5. Each text is encoded with `spec.add_special_tokens`. If its encoded length
   exceeds the width, `Reject` returns `LimitExceeded`, `KeepStart` retains
   the first width IDs, and `KeepEnd` retains the last width IDs.
6. Retained IDs are copied to the left or right side according to
   `PaddingSide`; exactly those positions receive mask value one. The original
   and retained lengths are recorded independently.

Padding is therefore structural, not inferred from token values. A real token
whose ID equals the pad ID remains valid because the mask, not the ID, controls
decoding.

### `Tokenizer::decode_batch_row`

This method first compares the decoder's canonical identity with the batch's
stored identity. It then obtains the requested row, filters only positions
whose attention mask is exactly one, and delegates the retained IDs to
`decode`. Left and right padding are removed by the mask. A valid pad-valued
token is preserved. An identity mismatch is `TokenizerMismatch`; an invalid
row is `InvalidBatch`; ID and output bounds are enforced by `decode`.

## Chat template rendering

`Message` is a `Clone + Debug + PartialEq + Eq` pair of public `role` and
`content` strings. `Message::new` only converts its arguments and performs no
validation. Callers can also construct or edit the public fields directly.

`render_template(template, messages, add_generation_prompt, bos_token,
eos_token, limits)` applies the following checks and calls:

1. Template bytes must fit `template_bytes` and message count must fit
   `messages`.
2. Every role must be nonempty. Every content string must fit
   `input_bytes`. Empty content is allowed. Role bytes and the BOS/EOS strings
   have no separate input bound; the rendered-output bound remains authoritative.
3. Each message becomes a `minijinja::Value` with only `role` and `content`
   fields. The values are collected in input order.
4. `hf_chat_template::ChatTemplate::from_str` compiles the template on every
   call. Compilation failures are `Template` with a `compile:` detail.
5. Rendering receives `messages`, `add_generation_prompt`, `bos_token`, and
   `eos_token` in one context. Rendering failures are `Template` with a
   `render:` detail.
6. The resulting string must fit `rendered_bytes` and is returned as an owned
   `String`.

An empty message list is allowed when it fits the message bound. Rendering is
not implicitly tokenization, and no chat history or template object is stored
after the function returns.

## Limits, invariants, and ownership

The following invariants are enforced by the current implementation:

* Every configured external-size limit is nonzero.
* Every externally supplied token-role ID is nonnegative, and every role used
  in a prepared batch must exist in the selected vocabulary.
* Vocabulary pieces are unique, token index order is stable, and all IDs that
  cross the text boundary are representable as `i32`.
* SentencePiece scores are finite. Byte-pair and SentencePiece metadata cannot
  mix explicit merge and score conventions.
* Aggregate byte sums, products, row offsets, and source read bounds use
  checked arithmetic before allocation or slicing.
* A prepared row has one fixed width, one explicit mask, and one original and
  retained length. The mask is the sole padding validity signal.
* A batch can be decoded only by a tokenizer with exactly equal canonical
  configuration identity.
* File input is a bounded preparation snapshot. Runtime code sees owned bytes,
  never a path or open descriptor.
* Chat roles are nonempty, message contents and rendered output are bounded,
  and template compile/render errors remain visible as `Template`.

The limits are not a model context contract by themselves. For example,
`TextBatchSpec::sequence_length` is bounded by `TextLimits::output_tokens`,
but the model's own context length or embedding vocabulary must be checked by
the downstream model compiler. Likewise, `TextBatch` does not contain model
weights, positional encodings, KV cache, logits, sampling state, or a device
allocation.

## Training and inference role

### Intended boundary

For a fixed-token embedding workload, the intended role is:

```text
raw source text
  -> optional render_template
  -> Tokenizer::prepare_batch
  -> TextBatch.token_ids + TextBatch.attention_mask
  -> caller converts rows to the model's exact int32 feature contract
  -> training or inference preparation
  -> init transfer
  -> leading embedding gather and the GPU calculation graph
```

The text crate stops before the last four stages. Its `i32` IDs line up with
Recipe's `DenseEmbedding` declaration, whose incoming columns are fixed
sequence positions and whose table maps each checked ID in `0..vocabulary` to
f32 channels. The embedding compiler, not `recipe-text`, owns that range check,
table gather, graph construction, scheduling, and execution.

### Current training path

The current root training path is `recipe::Train::run` in
[`src/training.rs`](../../src/training.rs), which calls
[`src/data_prepare.rs`](../../src/data_prepare.rs) and then the `training`
crate compiler. `prepare_data` distills files with `recipe-ingest`, infers
vectors, applies the declared split and normalization, and produces a
`PreparedDataset`. No step invokes `recipe-text` or accepts `TextBatch`.

When the first model block is an embedding, `training/src/compile.rs`
`validate_embedding_dataset` requires every feature vector to be an exact
numeric int32 vector, requires the prepared feature matrix to remain `I32`,
and rejects values outside `0..vocabulary`. The compile path then emits a
GPU embedding gather. Thus a current end-to-end text training run would need a
caller-owned adapter that turns the batch IDs into an ingest table or directly
into the exact prepared dataset contract. That adapter does not exist in this
workspace and must not be inferred from this crate's standalone API.

### Current semantic inference path

The root inference path in [`src/inference.rs`](../../src/inference.rs) is
target-free. It loads an `.ogdl` or supported `.gguf` model, distills and
selects a `RawTable`, and delegates to `training::prepare_*_inference_table`.
For a semantic dense checkpoint with a leading embedding,
`training/src/inference.rs::compile_token_features` consumes
`PreparedInferenceDataset` features that are already exact `I32` values,
checks each value against the saved vocabulary, and emits the external int32
feature transfers followed by the embedding graph. It does not call
`Tokenizer`, render chat templates, or decode model outputs.

The supported GGUF llama path is even more explicit:
`training/src/gguf_llama.rs::prepare_gguf_llama_inference_table` expects one
selected table column containing whitespace-separated decimal int32 IDs. It
rejects an empty or multi-column stream, non-UTF-8 or non-int32 fields, IDs
outside `0..vocabulary`, an empty stream, and a stream longer than the GGUF
context. The current GGUF path therefore consumes pre-tokenized numeric text;
it does not use `recipe-text` to turn natural language into those IDs, render a
chat prompt, sample logits, or maintain a KV session.

### Practical integration consequence

`recipe-text` is a complete bounded preparation component, but it is not proof
that public `.train()` or `.infer()` supports natural-language input. A real
integration must explicitly bridge `TextBatch` into the corresponding
`recipe-ingest` or GGUF token-stream contract, preserve the mask and fixed
sequence geometry, and make the tokenizer identity and vocabulary part of the
same preparation state. Until such a caller exists, the honest end-to-end
status is:

```text
recipe-text construction and text transformations: implemented
root training/inference invocation of recipe-text: not wired
GPU execution of recipe-text operations: not applicable
```

## Non-goals and failure posture

This crate does not implement a language model, chat session, sampling policy,
logit scoring, detokenization policy beyond the configured tokenizer decoder,
KV-cache management, prompt history, dataset semantic inference, GPU
allocation, or native execution. `Tokenizer::decode` is a bounded decoder
operation, not a model response generator. `render_template` is a bounded
string transformation, not a chat API with role policy or conversation state.

The failure posture is fail-closed. Malformed model bytes, invalid vocabulary
metadata, oversized text, invalid IDs, truncation under `Reject`, a mismatched
tokenizer identity, source failures, and template failures return typed
`TextError` values. There is no silent truncation unless the caller selects
`KeepStart` or `KeepEnd`, no mask inference from pad IDs, and no substitute
tokenizer when a configured construction path fails.

## Source map

| Path | Responsibility |
| --- | --- |
| [`src/lib.rs`](../src/lib.rs) | All public types, tokenizer construction and validation, BPE adapters, fixed-batch layout, decoding, and template rendering. |
| [`Cargo.toml`](../Cargo.toml) | Package identity, dependency versions/features, and lint policy. |
| [`../../src/facade.rs`](../../src/facade.rs) | Advanced `recipe::engine::text` reexport. |
| [`../../ops/src/non_calculation.rs`](../../ops/src/non_calculation.rs) | Source-qualified compatibility classification for tokenization and chat rendering. |
| [`../../ingest/src/source.rs`](../../ingest/src/source.rs) | Bounded one-shot source snapshot used by `Tokenizer::from_file`. |
| [`../../training/src/model.rs`](../../training/src/model.rs) | Downstream `DenseEmbedding` token-ID contract. |
| [`../../training/src/compile.rs`](../../training/src/compile.rs) | Training embedding feature validation and graph entry. |
| [`../../training/src/inference.rs`](../../training/src/inference.rs) | Semantic inference token-feature validation and GGUF llama preparation bridge. |

The companion source note [`text/.docs/src/lib.md`](src/lib.md) can contain a
line-level walkthrough of `lib.rs`; this README remains the crate-level
purpose, structure, and live integration map.

## Structural verification

The package-level structural check is:

```bash
cargo check -p recipe-text
```

This verifies the manifest and the public implementation compile with their
actual dependencies. It does not constitute a training or inference
acceptance run. Such an acceptance run requires a real caller that bridges
the text outputs into a real dataset and a real CUDA or HSA execution path;
the current workspace has no such caller.
