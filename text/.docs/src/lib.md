# `recipe-text` crate root

Source of truth: [`text/src/lib.rs`](/home/nate/Desktop/nates-recipe-rs/text/src/lib.rs).
The descriptions below follow the current checkout, including the fact that
this crate is exposed for advanced callers but is not yet called by the
training or inference pipelines.

## Role and crate boundary

`recipe-text` is a host-side metadata crate. Its root module is the complete
implementation crate: there are no implementation submodules, private helper
files, tests, or additional public modules under `text/src/`. The crate has
`#![forbid(unsafe_code)]` and
`#![deny(missing_debug_implementations)]` (source lines 1-2). Its module-level
contract says that tokenization converts raw text into checked `int32` token
identifiers and that chat rendering produces raw text before Recipe's single
init admission (lines 4-9). The implementation itself ends at those returned
host values. It does not score tokens, perform calculation-payload arithmetic,
allocate a device buffer, execute a kernel, or own a runtime file handle.

The package is `recipe-text` version `0.1.0`, edition 2024, unpublished. Its
only declared dependencies are:

* `tokenizers` 0.21 with only the `fancy-regex` feature. This supplies the
  inner Hugging Face tokenizer, BPE models, pre-tokenizers, decoders, and JSON
  serialization used by the wrapper.
* `hf-chat-template` 0.2 with `strftime`, used to compile and render a raw
  Hugging Face Jinja chat template.
* `recipe-ingest` from this workspace, used only by `Tokenizer::from_file` to
  obtain one bounded source snapshot.

The package manifest also denies the Rust `unsafe_code` lint and the Clippy
`all` and `pedantic` groups for this crate (`text/Cargo.toml` lines 9-16).
These are build-time lint policy; they do not add runtime behavior or extra
API entry points.

The root workspace lists `text` as a member (`Cargo.toml` lines 18-56) and the
top-level `recipe` package depends on it (`Cargo.toml` lines 58-84). The
top-level facade re-exports the package only in the advanced implementation
namespace as `recipe::engine::text` (`src/facade.rs` lines 17-42). There is no
top-level `recipe::text` alias and no `pub use` of these types at the root of
the facade.

The current source call graph is therefore:

```text
recipe::engine::text
        |
        v
recipe-text::Tokenizer / render_template
        |                         |
        |                         +--> hf_chat_template::ChatTemplate
        |                              +--> minijinja::Value/context
        |
        +--> tokenizers::Tokenizer
        |      +--> JSON parser/serializer
        |      +--> BPE model, ByteLevel or Metaspace pipeline
        |
        +--> recipe_ingest::read_source_snapshot (from_file only)
               +--> one closed, bounded file read
```

`ops/src/non_calculation.rs` classifies the legacy operation-surface symbol
`encode` as `NonCalculationRecipe::TextTokenization` and `render_chat` and
`render_template` as `NonCalculationRecipe::ChatTemplateRendering` (lines
17-64). The classification is inventory metadata and does not call this crate.
The operation-surface rows still point to the historical
`recipe-infer/src/chat.rs` sources (rows 425-426); those paths are not present
in this checkout. A repository search finds no call to `Tokenizer`,
`render_template`, `TextBatch`, or another `recipe_text` item outside the
facade re-export. In particular, `recipe.data(...)`, training, inference,
`recipe-ingest` dataset preparation, `recipe-prepare`, and the executors do not
currently traverse this crate. An advanced caller must explicitly connect a
rendered string to `Tokenizer::encode` or `Tokenizer::prepare_batch`; no such
connection is supplied by the facade.

## Shared limit and error representations

### `TextLimits`

`TextLimits` (lines 26-97) is a `Copy` value containing nine private
`NonZeroUsize` bounds. `TextLimits::new` converts each supplied `usize` with the
private `nonzero` helper. The first zero is rejected as
`TextErrorKind::InvalidLimit`; the error detail names the corresponding limit,
for example `input bytes limit must be nonzero`. There are no cross-field
relationships and no defaults. The caller must provide every bound explicitly.

| accessor | measured surface | checked by |
| --- | --- | --- |
| `model_bytes()` | serialized `tokenizer.json` bytes | `Tokenizer::from_json`; also the source bound in `from_file` |
| `input_bytes()` | UTF-8 bytes in one text input, and each message content | `encode`, `prepare_batch` through `encode`, and `render_template` |
| `output_tokens()` | encoded token count, decoder input count, and requested fixed sequence width | `encode`, `decode`, and `prepare_batch` |
| `vocabulary_entries()` | tokenizer vocabulary size, including added tokens | JSON and metadata constructors |
| `aggregate_piece_bytes()` | sum of `String::len()` for all vocabulary pieces | `from_vocabulary` |
| `merge_entries()` | supplied merge-table entry count | `from_vocabulary` |
| `template_bytes()` | raw Jinja template bytes | `render_template` |
| `messages()` | number of messages | `render_template` |
| `rendered_bytes()` | decoded output bytes and rendered chat bytes | `decode` and `render_template` |

Each getter returns the stored `NonZeroUsize`, not a plain `usize`. The
`Tokenizer` stores the complete `TextLimits` privately and offers no accessor
for it, so callers retain their own configuration if they need to report it.
The limits are invocation bounds, not reservations: successful calls may use
zero tokens or zero messages where the operation permits those values.

The private `require_limit(name, actual, limit)` helper (lines 930-939) uses a
single inclusive rule: `actual > limit` returns `LimitExceeded`, while
`actual == limit` succeeds. It is used for byte counts, sequence counts,
vocabulary and merge counts, token counts, and rendered output. Arithmetic
that cannot be represented in `usize` is reported separately as
`ArithmeticOverflow`.

### `TextErrorKind`, `TextError`, and `TextResult`

`TextErrorKind` is a `Copy`, `Debug`, `Eq` and `PartialEq` non-exhaustive enum
(lines 126-142). Its variants are the crate's stable failure categories:

| kind | current producers and meaning |
| --- | --- |
| `InvalidLimit` | any zero `TextLimits` or `TextBatchSpec` bound |
| `LimitExceeded` | an inclusive bound was exceeded by input, output, vocabulary, merges, message count, batch count, sequence width, or rendered bytes; also `Reject` truncation |
| `InvalidModel` | malformed tokenizer JSON or failure serializing the tokenizer identity |
| `InvalidVocabulary` | empty or duplicate pieces, invalid BPE merges, wrong score/merge shape, non-finite scores, or a BPE builder/pre-tokenizer/decoder construction error |
| `InvalidTokenId` | negative role or decode ID, a role outside the tokenizer vocabulary, a token ID above `i32::MAX`, or a vocabulary/index domain violation |
| `InvalidBatch` | no input sequences or an out-of-range prepared row |
| `TokenizerMismatch` | a `TextBatch` identity does not equal the decoder's identity |
| `Tokenization` | failure returned by the inner tokenizer while encoding |
| `Decode` | failure returned by the inner tokenizer while decoding |
| `InvalidMessage` | a message role is empty or message content exceeds `input_bytes` |
| `Template` | chat-template compilation or rendering failure |
| `Source` | source-limit construction or the bounded file snapshot failed |
| `ArithmeticOverflow` | checked conversion/product/sum/addition overflow |

`TextError` (lines 144-165) has public `kind: TextErrorKind` and
`detail: String` fields, derives `Clone`, `Debug`, `Eq`, and `PartialEq`, and
implements `Display` as `"{kind:?}: {detail}"` plus the standard error trait.
Its constructor is private, so all details are generated by this crate or by
mapped dependency errors. `TextResult<T>` is simply `Result<T, TextError>`.
Consumers matching `TextErrorKind` must include a wildcard because the enum is
non-exhaustive.

## Vocabulary and tokenizer identity metadata

### `VocabularyKind` and `VocabularySpec`

`VocabularyKind` (lines 108-113) selects one of two metadata adapters:
`BytePair` or `SentencePieceBpe`. `VocabularySpec<'a>` (lines 115-124) is a
borrowed framing of already-read model metadata:

* `kind` chooses the adapter.
* `tokens: &'a [String]` supplies the token piece at each integer ID. The
  wrapper assigns IDs from the slice index, so input ordering is semantic.
* `merges: &'a [String]` supplies literal `left right` entries for `BytePair`.
  SentencePiece requires this slice to be empty.
* `score_bits: &'a [u32]` supplies the IEEE-754 `f32` bit pattern for each
  SentencePiece token. The wrapper reconstructs scores with `f32::from_bits`
  and does not perform host arithmetic on the metadata. BytePair requires an
  empty score slice.
* `unknown_token_id` is required and range-checked for `SentencePieceBpe`. It
  is ignored for `BytePair`.

The spec borrows caller-owned strings and bits only during construction. The
resulting tokenizer owns cloned vocabulary strings and does not retain the
spec slices.

### `TokenizerIdentity`

`TokenizerIdentity` (lines 169-186) owns an `Arc<str>` containing the inner
tokenizer's compact `to_string(false)` serialization. The serialization is
created after every successful constructor by `tokenizer_identity`; failure is
`InvalidModel`. Equality is exact string equality. Consequently the identity
carries the full serialized tokenizer configuration, including model
vocabulary, added-token metadata, normalizer, pre-tokenizer, post-processor,
and decoder, rather than a lossy hash or an address. Cloning a batch or a
tokenizer shares the string allocation through the `Arc`.

The canonical string itself is private. `Debug` intentionally reports only its
byte length under `canonical_bytes` and uses `finish_non_exhaustive`, so callers
cannot inspect or reconstruct the serialization through this API. The identity
is attached to every `TextBatch`; `decode_batch_row` compares it before using
any IDs.

## Fixed-width batch representations

### Token roles, padding, and truncation

`TextTokenIds` (lines 206-251) stores a required `pad: i32`, an optional
`unknown: Option<i32>`, and an owned boxed slice of `special` IDs. The
constructor checks every supplied role for negativity and returns
`InvalidTokenId` naming the role and value. It does not check vocabulary
membership, uniqueness, or overlap. Overlap is explicitly valid, including a
pad ID that is also an end-of-sequence or special ID. `pad`, `unknown`, and
`special` are read through immutable getters; the special slice cannot be
mutated through the API.

`PaddingSide` (lines 188-193) is `Left` or `Right`. `TruncationPolicy` (lines
195-204) is:

* `Reject`, which refuses any encoded sequence wider than the fixed width;
* `KeepStart`, which keeps the first `sequence_length` IDs; or
* `KeepEnd`, which keeps the last `sequence_length` IDs.

The policy does not alter the tokenizer or raw text. It is applied by
`prepare_batch` after encoding.

### `TextBatchSpec`

`TextBatchSpec` (lines 253-318) is an immutable preparation contract with:

* nonzero `max_sequences`;
* nonzero `sequence_length`;
* the selected `PaddingSide`;
* the selected `TruncationPolicy`;
* `add_special_tokens: bool`, passed unchanged to each `Tokenizer::encode`;
* a cloned `TextTokenIds` role set.

`TextBatchSpec::new` rejects either zero bound as `InvalidLimit`. It also
performs `max_sequences.checked_mul(sequence_length)` and returns
`ArithmeticOverflow` if the maximum flat layout cannot fit `usize`. It does not
compare the width with a particular tokenizer's output limit or validate token
IDs against a vocabulary. Those checks happen when a spec is supplied to
`Tokenizer::prepare_batch`.

### `TextBatchLayout`, `TextBatch`, and `TextBatchRow`

`TextBatchLayout` currently has one variant, `BatchMajor` (lines 320-326). A
batch is contiguous `[batch, sequence]` storage, with flat index
`batch_index * sequence_length + sequence_index`.

`TextBatch` (lines 328-386) owns:

* the exact `TokenizerIdentity` that produced its IDs;
* a clone of the `TextBatchSpec` used to lay it out;
* `sequences`, always the nonempty input count from a successful preparation;
* boxed flat `token_ids: [i32]`, including explicit pad values;
* boxed flat `attention_mask: [i32]`, containing only zero or one;
* boxed per-row `original_lengths`, measured immediately after encoding; and
* boxed per-row `retained_lengths`, measured after truncation.

The accessors expose the layout, identity, spec, sequence count, `[batch,
sequence]` shape, and immutable slices. In a successful value, the token and
mask slices each have `sequences * sequence_length` elements, and both length
slices have `sequences` elements. `shape()` recomputes the two dimensions from
the stored sequence count and spec width.

The attention mask is positional metadata, not a comparison with the pad ID:
one marks an encoded token retained in the row, and zero marks an explicit
padding position. Thus a real token whose ID equals `pad` remains valid with a
mask of one. A row may have retained length zero if the underlying tokenizer
encodes an empty string to no IDs.

`TextBatch::row(index)` uses checked multiplication and addition and then
`slice.get`. A valid index returns a `TextBatchRow<'_>` borrowing the complete
row and its two lengths. Any out-of-range index returns `None` without a panic.
The row view is `Copy`, `Clone`, `Debug`, `Eq`, and `PartialEq`; its four
accessors return borrowed token and mask slices and copied lengths.

## `Tokenizer` state and constructors

`Tokenizer` (lines 411-426) owns exactly three values: the private
`tokenizers::Tokenizer` (`inner`), a copied `TextLimits` value, and the
`TokenizerIdentity` derived at construction. It has no path, source snapshot,
external handle, mutable configuration API, or runtime/device state. Its
custom `Debug` reports inner vocabulary size, limits, and identity without
exposing private implementation details.

All constructors validate before returning a value. A failed constructor does
not expose a partially built tokenizer.

### `Tokenizer::from_json`

`from_json(bytes, limits)` (lines 429-450) is the in-memory Hugging Face path:

1. It applies `limits.model_bytes` to `bytes.len()`.
2. It calls `tokenizers::Tokenizer::from_bytes`. Any JSON/schema/dependency
   failure becomes `InvalidModel` with the dependency's display text.
3. It calls `validate_vocab_size`, which obtains
   `inner.get_vocab_size(true)`, including added tokens, and checks both
   `vocabulary_entries` and the `i32` identifier ceiling.
4. It serializes the loaded tokenizer compactly to create its identity.

The loaded tokenizer's normalizer, pre-tokenizer, model, post-processor,
added-token table, and decoder are retained exactly as supplied by the JSON.
The wrapper does not rewrite those components.

### `Tokenizer::from_file`

`from_file(path, limits)` (lines 452-469) is preparation-only file access. It
converts the `usize` model bound to `u64`; a conversion failure is
`ArithmeticOverflow`. It then constructs `recipe_ingest::SourceLimit`, maps a
zero or other source-limit failure to `Source`, and calls
`recipe_ingest::read_source_snapshot`. That helper opens one regular file,
checks metadata, reads at most `limit + 1` bytes, rejects growth beyond the
limit, computes a digest, and closes the handle before returning its owned
bytes (see `ingest/src/source.rs` lines 13-18 and 116-176). Any such failure is
mapped to `TextErrorKind::Source`. Finally, only `snapshot.bytes()` is passed to
`from_json`.

The resulting `Tokenizer` does not retain the source path or digest. A caller
that needs provenance must retain it independently. There is no file read in
`encode`, `decode`, batch preparation, or chat rendering.

### `Tokenizer::from_vocabulary`

`from_vocabulary(spec, limits)` (lines 471-494) adapts already validated GGUF
metadata without reading a file. The sequence is:

1. `validate_vocabulary` checks shape, bounds, pieces, scores, and unknown ID.
2. `vocabulary_map` clones each token and assigns its slice index as a `u32`.
3. The selected BPE constructor builds an inner tokenizer.
4. `validate_vocab_size` applies the final vocabulary bound and `i32` ceiling.
5. `tokenizer_identity` serializes the resulting tokenizer.

`from_vocabulary` owns no references to `spec` after it returns.

#### Common vocabulary validation

`validate_vocabulary` (lines 742-828) applies the following exact checks:

* `tokens` cannot be empty.
* Token count and merge count use their corresponding limits. The merge count
  is checked even for SentencePiece, where it must subsequently be zero.
* The sum of each token's UTF-8 byte length uses checked addition and must fit
  `aggregate_piece_bytes`.
* Token count must fit the `i32` identifier domain.
* A `HashSet` rejects duplicate token strings. The first duplicate is reported
  as `InvalidVocabulary` with the quoted piece.

The wrapper assigns IDs by enumeration, independent of any IDs that might have
been present in an upstream container. The source metadata adapter is expected
to have already ordered the slice according to the model's ID contract.

#### BytePair construction

For `VocabularyKind::BytePair`:

* `score_bits` must be empty. `unknown_token_id` is not examined.
* Each merge string is split on the literal ASCII space character. It must
  produce exactly two nonempty pieces. Extra pieces, missing pieces, or
  repeated spaces return `InvalidVocabulary` before the dependency is called.
* The resulting `(left, right)` list is passed to
  `BPE::builder().vocab_and_merges(...).build()`. The dependency requires both
  pieces and their concatenated result to exist in the vocabulary; any failure
  is mapped to `InvalidVocabulary`.
* The wrapper installs `ByteLevel::new(false, true, true)` as both the
  pre-tokenizer and decoder. The first flag disables a synthetic leading space,
  the second trims offsets, and the third enables the byte-level regex. The
  custom tokenizer has no post-processor, so its `add_special_tokens` argument
  cannot inject special tokens by itself.

#### SentencePieceBpe construction

For `VocabularyKind::SentencePieceBpe`:

* `score_bits.len()` must equal `tokens.len()` and `merges` must be empty.
* Every score bit pattern must decode to a finite `f32`; NaN and infinities are
  rejected as `InvalidVocabulary`.
* `unknown_token_id` is required and must index `tokens`. An absent ID is
  `InvalidVocabulary`; an out-of-range ID is `InvalidTokenId`.
* For every token and every UTF-8 character boundary after the first, the
  wrapper proposes a merge candidate when both substrings are separate
  vocabulary pieces. Candidates carry the token's reconstructed score.
* Candidates are sorted by descending `f32::total_cmp` score, then ascending
  left piece and ascending right piece for equal scores. Their score is used
  only for ordering. The resulting pairs are passed to the BPE model.
* The unknown piece is the token at `unknown_token_id`; the BPE model is built
  with that unknown piece, `byte_fallback(true)`, and `fuse_unk(true)`.
* The pre-tokenizer is `Metaspace::new('▁', PrependScheme::First, false)`.
  The decoder is a `Sequence` containing, in order, replacement of `▁` with a
  space, byte fallback, fuse, and `Strip::new(' ', 1, 0)`.
* Construction errors from the BPE model, replacement normalizer, or decoder
  are `InvalidVocabulary`. The two `expect` calls in this helper are reached
  only after the range and presence checks above have succeeded.

## Token operations

### `Tokenizer::encode`

`encode(text, add_special_tokens)` (lines 496-530) performs one bounded call:

1. `text.len()` is checked against `input_bytes`. This is a byte count, not a
   Unicode scalar or grapheme count.
2. The exact bool is forwarded to `inner.encode(text, add_special_tokens)`.
   Normalization, pre-tokenization, model merges, added-token handling, and any
   JSON post-processor therefore come from the inner tokenizer.
3. The resulting `Encoding` length is checked against `output_tokens`.
4. Every returned `u32` ID is converted to `i32`; a value above `i32::MAX`
   produces `InvalidTokenId` naming the value. The successful result is an
   owned `Vec<i32>`.

Empty input is not rejected by this wrapper. Whether it yields no IDs or IDs
from a configured post-processor is determined by the inner tokenizer. The
wrapper does not truncate here, and it does not use the token-role metadata.

### `Tokenizer::decode`

`decode(ids, skip_special_tokens)` (lines 532-566) performs the inverse
boundary checks:

1. The input ID count is checked against `output_tokens` (the same bound used
   for encode output).
2. Every `i32` is converted to `u32`; any negative value is
   `InvalidTokenId`.
3. The positive IDs and the exact skip flag are passed to the inner decoder.
4. Decoder failures become `Decode`, and the resulting string's byte length is
   checked against `rendered_bytes`.

The wrapper does not range-check positive IDs itself. In tokenizers 0.21, the
inner decoder filters IDs that have no added-vocabulary or model token and
passes the remaining token strings to the decoder. Thus an unknown positive
ID can be silently omitted by the dependency rather than producing
`InvalidTokenId`; only negative IDs and wrapper conversion failures are
rejected here. `skip_special_tokens` has the dependency's exact semantics.

`vocabulary_size()` returns `inner.get_vocab_size(true)`, including added
tokens. It does not return the configured vocabulary limit. `identity()` returns
the private identity reference used for batch provenance.

## Batch preparation and row decoding

### `Tokenizer::prepare_batch`

`prepare_batch(texts, spec)` (lines 575-661) is the only operation that turns a
set of strings into the fixed-width representation. It is deterministic and
does not mutate the tokenizer or the caller's spec.

The admission checks run in this order:

1. An empty `texts` slice returns `InvalidBatch` with
   `text batch contains no sequences`.
2. `texts.len()` must fit `spec.max_sequences`.
3. `spec.sequence_length` must fit this tokenizer's `output_tokens` limit.
4. Every pad, unknown, and special role is converted to `u32` and looked up
   with `inner.id_to_token`. A negative role or an ID absent from the model and
   added vocabulary returns `InvalidTokenId`. Role overlap remains valid.
5. `texts.len() * sequence_length` is checked before allocating the flat
   arrays. Overflow is `ArithmeticOverflow`.

The arrays start as all pad IDs and all-zero masks. The method then processes
each input in order:

1. It calls `encode` using `spec.add_special_tokens`, so each input is subject
   to the tokenizer input and output limits.
2. `original_length` is the returned encoded length before fixed-width
   truncation.
3. If the length is within `width`, all IDs are retained. If it is wider,
   `Reject` returns `LimitExceeded` with the row index and both lengths,
   `KeepStart` takes `encoded[..width]`, and `KeepEnd` takes
   `encoded[original_length - width..]`.
4. For right padding, retained IDs start at the row start. For left padding,
   they start at `row_start + width - retained_length`. The selected region is
   copied into `token_ids`; exactly that region is filled with mask value one.
5. The original and retained lengths are appended for the row.

The result clones the spec, clones the tokenizer identity, and boxes all
vectors. A later encode error or truncation rejection returns an error rather
than a partially initialized batch. The `padding` value affects placement only;
it never decides validity by comparing IDs. `unknown` and `special` IDs are
used only for vocabulary admission and are not rewritten or masked.

### `Tokenizer::decode_batch_row`

`decode_batch_row(batch, row, skip_special_tokens)` (lines 663-693) enforces
provenance before decoding:

1. It compares this tokenizer's exact `TokenizerIdentity` with the batch's
   identity. A mismatch returns `TokenizerMismatch` immediately, even if the
   requested row is also out of range.
2. It obtains the complete row with `TextBatch::row`. `None` becomes
   `InvalidBatch` naming the requested index.
3. It zips row token IDs and mask values and retains only entries whose mask is
   exactly `1`. It does not compare IDs with the stored pad role.
4. It calls `decode` on the retained IDs, so the decoder input and rendered byte
   limits and all negative-ID checks apply again.

Because batch fields are private and preparation writes only zero and one,
every successful batch has a canonical mask. The explicit `== 1` filter is the
actual rule if a future constructor creates another mask value.

## Chat API

### `Message`

`Message` (lines 941-955) is a public, cloneable pair of `pub role: String` and
`pub content: String`. `Message::new(role, content)` performs only `Into<String>`
conversion and no validation. Direct field construction and mutation are also
possible. Validation belongs to `render_template`.

### `render_template`

`render_template(template, messages, add_generation_prompt, bos_token,
eos_token, limits)` (lines 957-1011) compiles and renders one bounded raw Jinja
chat template:

1. The template byte length is checked against `template_bytes`.
2. The message count is checked against `messages`.
3. Every message must have a nonempty role and content no larger than
   `input_bytes`. Empty content is accepted. An empty message slice is also
   accepted if it fits the count bound and the template itself can render it.
   Role length, `bos_token` length, and `eos_token` length have no separate
   wrapper bound.
4. Each message becomes a minijinja context value with exactly `role` and
   `content` keys. The method builds a `Vec<Value>` in input order.
5. `ChatTemplate::from_str(template)` compiles a fresh template for this call.
   Compilation errors become `Template` with a `compile:` detail prefix.
6. `render_value` receives a context containing `messages`, the supplied
   `add_generation_prompt` bool, `bos_token`, and `eos_token`. Render errors
   become `Template` with a `render:` detail prefix.
7. The rendered UTF-8 string byte length is checked against `rendered_bytes`.

The returned `String` is not tokenized automatically. A caller normally passes
it to `Tokenizer::encode` or `prepare_batch` in a separate step, which applies
the input and token-count limits. The template is not retained or shared
between calls, and the function does not use a tokenizer identity.

`ChatTemplate` 0.2's raw-string builder installs its default
transformers-compatible Jinja environment. This wrapper deliberately uses the
low-level `render_value` route, so special tokens are supplied explicitly in
the context rather than loaded from a tokenizer configuration. The template
may ignore or reinterpret any of the supplied context values according to its
own Jinja source. The wrapper does not enforce a particular role vocabulary or
require a generation marker.

## End-to-end lifecycle and state ownership

The intended host-side sequence is:

```text
bounded tokenizer.json or GGUF metadata
        |
        +--> Tokenizer constructor
        |      validates model/vocabulary and freezes limits + identity
        |
raw messages --render_template--> raw prompt text
                                      |
                                      +--> encode or prepare_batch
                                             checked int32 IDs
                                             fixed-width IDs + mask
        |
        +--> caller's preparation/admission boundary
```

The code currently implements only the boxed region. It does not invoke the
next `recipe-prepare` admission step, publish authoritative runtime state, or
send token data over host, CUDA, HSA, or remote transports. It also does not
provide generation, sampling, logits, KV-cache, conversation state, or a
`.chat()` builder. The public `Message` and `render_template` surface is
rendering metadata, not a language-model execution API.

State is deliberately one-directional and owned:

* Constructors consume bounded inputs and return an owned `Tokenizer`.
* `Tokenizer` calls borrow text and specs, then return owned vectors or strings.
* `TextBatch` owns all IDs, masks, lengths, its cloned spec, and its identity.
* Row views borrow a batch and cannot outlive it.
* `decode_batch_row` is the only batch consumer and requires identity equality.
* `from_file` closes the source handle before returning, and no later method
  can read the path again.

No method exposes mutable access to the inner tokenizer, its limits, or batch
arrays. Internal dependency caches, if present in `tokenizers`, remain an
implementation detail of the inner tokenizer and do not change the wrapper's
owned outputs or limits.

## Failure and boundary matrix

| call | checks before dependency work | dependency work | wrapper result |
| --- | --- | --- | --- |
| `TextLimits::new` | all nine values nonzero | none | `TextLimits` or `InvalidLimit` |
| `TextTokenIds::new` | role IDs nonnegative | none | role metadata or `InvalidTokenId` |
| `TextBatchSpec::new` | both bounds nonzero, product fits `usize` | none | spec or `InvalidLimit`/`ArithmeticOverflow` |
| `Tokenizer::from_json` | model bytes within limit | parse tokenizer JSON, inspect vocabulary, serialize identity | tokenizer or `LimitExceeded`/`InvalidModel`/`InvalidTokenId` |
| `Tokenizer::from_file` | model limit converts to `u64` | bounded `recipe-ingest` snapshot, then JSON path | tokenizer or `ArithmeticOverflow`/`Source` plus JSON errors |
| `Tokenizer::from_vocabulary` | shape, pieces, scores, IDs, aggregate bounds | construct BPE pipeline, inspect vocabulary, serialize identity | tokenizer or `InvalidVocabulary`/`InvalidTokenId`/limit errors |
| `Tokenizer::encode` | input bytes | inner encode | `Vec<i32>` or `LimitExceeded`/`Tokenization`/`InvalidTokenId` |
| `Tokenizer::decode` | ID count and nonnegative IDs | inner decode | `String` or `LimitExceeded`/`Decode`/`InvalidTokenId` |
| `Tokenizer::prepare_batch` | nonempty/count/width/roles/layout | one encode per input | `TextBatch` or batch, tokenization, truncation, limit, or arithmetic error |
| `Tokenizer::decode_batch_row` | exact identity and valid row | decode retained IDs | `String` or mismatch/batch/decode errors |
| `render_template` | template/message/content bounds | compile and render Jinja | `String` or message/template/limit errors |

The crate never converts a failure to a success through a fallback tokenizer,
unbounded read, CPU calculation path, or alternate state representation.

## Validation evidence for this documentation

The current package was checked with:

```text
cargo check -p recipe-text
```

It completed successfully for `recipe-text` and its dependencies. The source
inventory confirms that `text/src/lib.rs` is the sole implementation file and
that the only direct Rust-source reference to the package is the
`recipe::engine::text` re-export. The package is also named explicitly in the
workspace member list and the root `recipe` manifest dependency, as described
above. No behavior described here depends on a mock, a test-only entry point,
or an unobserved caller.
