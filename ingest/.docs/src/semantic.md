# Semantic distillation

This document describes the current implementation in `ingest/src/semantic.rs`
and the contracts that consume its result. Semantic distillation is a
preparation-time boundary. It turns the bytes in a rectangular [`RawTable`]
into one typed, ordered description for each source column. It does not create
rows, derive columns, normalize values, impute missing values, run a model, or
perform device work.

The implementation is the authority for the present behavior. Some older
design prose calls out six inferred types. The current `SemanticType` enum has
seven variants, including `Binary`, because opaque binary payloads are now a
first-class exact source declaration and can also be returned by a custom
ambiguous-vector model. The source-column order and exact byte names remain
authoritative throughout preparation, checkpointing, and inference.

## Boundary and data flow

The end-to-end path is:

1. A source reader frames bytes into a rectangular `RawTable`. A raw cell is
   still an owned byte vector. An empty byte vector is the repository's
   missing-value marker.
2. `DistilledDataset` keeps that table together with one crate-private
   `VectorSemanticRule` per column. Parsers for known container formats may
   supply exact rules; ordinary delimited, JSON, spreadsheet, and ambiguous
   container fields usually supply `Infer` or `Classify`.
3. `infer_table_vectors_with_semantics` validates row width, collects complete
   column evidence, applies the requested rule, and returns an
   `InferredVectorList` in source-column order. The public
   `infer_table_vectors` entry point is the same operation with all rules set
   to `Infer`.
4. Preparation fits encoding metadata only on the exact retained training
   partition. It applies that immutable schema to every retained row and
   records partitions and source-row identities in `PreparedDataset`.
5. Training consumers use the resulting `(SemanticType, VectorEncoding,
   VectorMetadata, PreparedValues)` tuple to choose dense scalar or categorical
   one-hot lowerings, KNN label reduction, Bayesian categorical operation
   boundaries, and task validation. Checkpoint manifests persist the row-free
   tuple and fitted metadata.
6. Target-free inference does not re-run semantic inference. It reconstructs a
   narrow feature schema from the checkpoint and parses only the saved numeric
   or categorical features. Missing numeric input is an error. A missing or
   unseen categorical value receives the reserved calculation code while a
   parallel observation route preserves the distinction and unseen bytes.

Semantic code therefore owns meaning selection and the lossless source
representation. It does not own task semantics, feature engineering, model
normalization, output interpretation, or graph execution.

## Semantic vocabulary

`SemanticType` (the exhaustive classification in `semantic.rs:8-19`) is:

| Type | Meaning in the source contract | Normal encoding chosen by this module |
| --- | --- | --- |
| `Numeric` | A scalar decimal that passes the repository's exact int32 or finite f32 lexical contract | `I32` when every present value is an exact int32, otherwise `F32`; an ambiguity-model return of `Numeric` maps to `F32` |
| `Temporal` | A date or timestamp accepted by `parse_temporal_instant` | `RelativeSecondsI32`, relative to an origin fitted later |
| `Categorical` | Discrete labels with no ordered numeric meaning | `DictionaryI32`; dictionary labels are fitted later and code `dictionary.len()` is the reserved missing/unseen route |
| `Ordinal` | Values in one of the six declared ordered vocabularies | `OrdinalI32`; rank zero is the first canonical label |
| `Text` | Variable-width valid UTF-8 prose or other textual data that the ambiguity model identifies as text | `Utf8` |
| `Image` | Encoded image bytes recognized by a supported signature | `Bytes`; image headers are validated and metadata is fitted later |
| `Binary` | Opaque bytes with no scalar interpretation | `Bytes` |

`VectorEncoding` is the smallest calculation-facing representation that still
preserves the source vector under its semantic meaning (`semantic.rs:21-32`).
`dtype()` maps `F32` to `DType::F32`; `I32`, relative seconds, dictionary, and
ordinal encodings to `DType::I32`; and `Utf8` and `Bytes` to `None` because
variable-width payloads have no scalar dtype until a declared operation gives
them a typed lowering. This is source metadata, not an invitation to cast the
source on the host. Training may explicitly convert an int32 matrix to f32 for
a mathematical operation, but the saved vector encoding remains int32.

`InferredVector` carries five facts for one source column:

* `index`, the original zero-based source-column index;
* `name`, copied byte-for-byte from the header, or `col{index + 1}` only when
  the table has no header at that index;
* the selected `semantic_type`;
* the selected `encoding`; and
* the complete `VectorEvidence` used for any ambiguous classification.

`InferredVectorList` owns only the ordered vector descriptions. Rows stay in
`RawTable`; no feature-generation side path exists. The accessors expose the
facts read-only. `VectorSemantic` and `VectorSemanticRule` are deliberately
crate-private so source parsers can declare an authoritative exact meaning
without expanding the public inference API.

## Evidence collection

`collect_evidence` (`semantic.rs:367-443`) receives all cell byte slices for one
column, including validation rows when its caller passes the full table. It
uses a byte-ordered `BTreeSet` for distinct nonmissing values. The fields mean:

| Field | Definition |
| --- | --- |
| `values` | Number of cells in the column, including empty/missing cells |
| `missing` | Number of empty byte slices |
| `unique` | Distinct nonempty raw byte values |
| `utf8_values` | Nonempty values accepted by `str::from_utf8` |
| `whitespace_values` | Valid UTF-8 values containing at least one ASCII whitespace byte |
| `source_bytes` | Checked sum of every source byte length, including missing cells (which contribute zero) |
| `dictionary_bytes` | Checked sum of distinct label bytes plus `values * size_of::<i32>()`; the code storage includes one code slot per source row |
| `mean_value_bytes` | Integer `source_bytes / present`, or zero when no value is present |
| `unique_per_thousand` | `unique / present * 1000`, saturating and clamped to 1000 |
| `whitespace_per_thousand` | `whitespace_values / present * 1000`, saturating and clamped to 1000 |

`present` is `values.saturating_sub(missing)`. The current construction cannot
make `missing > values`, but saturation keeps evidence total if a future
caller changes that assumption. Ratio arithmetic intentionally uses integer
thousandths rather than host floating point, making the classifier deterministic
across platforms.

Every aggregate addition that can overflow `usize` is checked. The function
returns `SemanticErrorKind::ArithmeticOverflow` with a distinct detail for
source bytes, missing count, UTF-8 count, dictionary payload, dictionary code
bytes, or total dictionary bytes. Set insertion itself is not converted into a
semantic error: allocation failure remains an ordinary process failure rather
than an invented semantic state.

## Ambiguous-vector classification

`AmbiguousVectorModel` (`semantic.rs:67-71`) is the explicit model boundary for
values that no lossless parser can identify. It receives only `VectorEvidence`
and returns one `SemanticType`; it never receives or invents derived feature
vectors. `CategoricalEncodingModel` is the built-in fixed nearest-example
model (`semantic.rs:73-171`). Its seven auditable examples cover four
categorical shapes and three prose/text shapes. Each example contains:

* unique-value cardinality in thousandths;
* whitespace-bearing-value ratio in thousandths;
* mean nonmissing value width in bytes;
* whether the dictionary representation is no larger than the source; and
* the categorical or text label.

Before nearest-example scoring, the model fails closed to `Categorical` when
any present value is not UTF-8. This is the check
`utf8_values != values.saturating_sub(missing)`, so opaque or mixed-invalid
bytes cannot be called text by the built-in model. For all-UTF-8 evidence it
computes `dictionary_smaller` as `dictionary_bytes <= source_bytes`, then scores
each example with saturating `u64` arithmetic:

```text
  squared unique-ratio difference
+ squared whitespace-ratio difference
+ 16 * squared mean-width difference
+ 250000 when dictionary-smaller differs
```

`min_by_key` returns the first example on an exact tie. The model's fallback is
`Categorical`, although the fixed table is nonempty. A caller may supply a
different model and may return any current `SemanticType`; the subsequent
`encoding_for` mapping is deterministic. A custom model returning a tuple that
does not match actual cells is not repaired here. Preparation's lossless
encoding pass is the later failure boundary.

## Rule precedence and inference

`infer_table_vectors_with_semantics` (`semantic.rs:267-318`) first obtains the
table width from `RawTable::width`, then checks every row has exactly that many
cells. A width mismatch returns `SemanticErrorKind::InconsistentWidth` with the
row index and expected width. This check is retained even though normal
`RawTable::from_parts` construction already enforces rectangular rows, because
semantic inference is an internal boundary and must not index a malformed row.

For each source index it builds a complete value-slice list, collects evidence,
and selects one rule. A `semantics` slice shorter than the table is legal; an
absent entry means `Infer`. Extra entries are ignored because no source column
corresponds to them. The rule meanings are:

* `Infer` calls `classify_vector`, which tries parsers in a fixed order and only
  then invokes the ambiguity model.
* `Classify` bypasses all semantic parsers and invokes the model directly on
  evidence. The returned type is converted with `encoding_for`.
* `Exact(VectorSemantic)` bypasses both parsers and the model, retaining the
  source producer's exact semantic type and encoding. It still records fresh
  evidence, because evidence is part of the returned diagnostic description.

`classify_vector` removes empty values for parser checks, but never removes them
from evidence or from the eventual row alignment. If there are no present
values, all parser checks are skipped and the ambiguity model decides.

For a nonempty present set, parser precedence is exact and observable:

1. If every present value has a recognized image signature, return
   `Image/Bytes`. Signature recognition is intentionally cheap and does not
   prove a complete header. `prepare` later calls the full image-header
   inspector.
2. If every present value parses as a temporal instant, return
   `Temporal/RelativeSecondsI32`.
3. If `ordinal_vocabulary` finds one recognized order, return
   `Ordinal/OrdinalI32`.
4. If every present value is UTF-8 and passes `parse_contract_i32`, return
   `Numeric/I32`.
5. If every present value is UTF-8 and passes `parse_contract_f32`, return
   `Numeric/F32`.
6. Otherwise call the ambiguity model and map its type through `encoding_for`.

The integer check precedes f32, so an integer-looking column retains the
smallest exact int32 representation. A decimal or exponent form that passes
the f32 contract but not the integer contract is f32. Numeric parsing is the
same contract used during preparation: whitespace, separators, nonfinite
values, out-of-range values, excess significant digits, and f32 precision loss
are not silently accepted.

`encoding_for` is exhaustive over the enum. It maps categorical to dictionary
int32, text to UTF-8, numeric to f32, temporal to relative seconds, ordinal to
ordinal int32, and image or binary to bytes. Thus a custom model returning
`Numeric` cannot force an int32 encoding; only the parser's exact integer path
does that, while a custom `Exact` rule can declare an int32 numeric contract.

## Temporal parser and representation

`TemporalInstant` is crate-private and contains `(unix_seconds: i64,
nanoseconds: u32)`. `parse_temporal_instant` (`semantic.rs:461-553`) accepts
only UTF-8 bytes in these forms:

* `YYYY-MM-DD`, interpreted as midnight UTC;
* the same date followed by `T` or a space and `HH:MM:SS`;
* an optional fractional second after `.`; up to nine digits become
  nanoseconds, and any digits beyond nine must all be zero; and
* an optional terminal `Z` or signed `+HH:MM`/`-HH:MM` offset.

Year is 1 through 9999, month is 1 through 12, and day is checked against leap
years using the Gregorian 400/100/4 rule. Hours are 0 through 23, minutes 0
through 59, and seconds 0 through 60 (the implementation admits a leap-second
spelling). Offset hours are 0 through 23 and offset minutes 0 through 59. No
other suffix, fractional spelling, separator, or trailing byte is accepted.

The parser converts a civil date with `days_from_civil`, then checked arithmetic
builds Unix seconds and subtracts the offset. Any failed conversion returns
`None`; the parser is intentionally an option-returning recognizer rather than
a public error-producing API. A temporal column therefore falls back to the
ambiguous model if even one present value is not in this grammar.

`prepare` fits the origin as the minimum nonmissing instant in the training
partition. An all-missing fit partition uses Unix zero as the origin. Every
retained nonmissing value is reparsed and converted to a checked whole-second
delta. A fractional delta that cannot be represented exactly in whole seconds
returns `PrepareErrorKind::EncodingFailure`; a delta outside i32 returns
`TemporalRangeExceeded`; checked nanosecond arithmetic overflow returns
`ArithmeticOverflow`. The fitted origin is serialized as temporal metadata and
is the only state needed to interpret relative values.

## Ordinal vocabularies

The six static, ordered, case-insensitive vocabularies are:

* `low`, `medium`, `high`;
* `small`, `medium`, `large`;
* `beginner`, `intermediate`, `advanced`;
* `poor`, `fair`, `good`, `very good`, `excellent`;
* `first`, `second`, `third`, `fourth`, `fifth`; and
* `bronze`, `silver`, `gold`, `platinum`.

`ordinal_vocabulary` requires at least two present values. Every value must be
case-insensitively present in one candidate order, and at least two distinct
ranks must occur. It returns the first matching static order. The set-based
check permits repeated labels, which is required for ordinary ordinal columns.
Because `medium` belongs to the first two orders, a value set such as only
`medium` cannot pass the two-value inference rule, and a set containing labels
that fit both orders resolves by static order.

`fit_ordinal_vocabulary` has intentionally different rules. It examines only
the nonmissing fit-partition values, permits a single label when that label
identifies exactly one vocabulary, and returns `None` if there are zero values
or more than one candidate. Validation labels never disambiguate the order.
`fit_vector_schema` turns a resolved order into canonical lowercase
`ordered_labels`; if the inferred encoding is ordinal but nonempty fit values do
not identify exactly one vocabulary, preparation fails with
`PrepareErrorKind::EncodingFailure`. An all-missing fit partition receives an
empty ordinal label list and later nonempty retained values fail because no
rank can be assigned.

## Source producers of semantic rules

`ingest/src/dataset.rs` is the main producer of crate-private rules. The
`LogicalColumn` constructors are the only three source-side declarations:

* `infer(name)` delegates to parser precedence followed by the ambiguity model;
* `classify(name)` delegates directly to the ambiguity model; and
* `exact(name, semantic_type, encoding)` preserves a format parser's known
  contract without trying to rediscover it.

`Accumulator::resolve_header` merges columns from multiple files, directory
members, or archive members. When a repeated header has a different rule from
the rule already stored at that global index, it deliberately downgrades the
global rule to `Infer`. This prevents one file's exact declaration from
silently claiming a heterogeneous aggregate. The accumulator still keeps one
global source-column index, fills absent member values with empty bytes, and
preserves row order.

Current format-specific producers are:

| Producer | Rule behavior |
| --- | --- |
| Delimited CSV, TSV, whitespace, and tabular text | Every parsed header is `Infer` |
| JSON object, array, and scalar tables | Generated or object-key headers are `Infer`; a JSON object key column is `Classify` |
| XLSX worksheets | Headers are `Infer`; numeric/date cells remain source bytes for later semantic parsing |
| Plain UTF-8 text files | Exact `Text/Utf8` one-row payload |
| Recognized image files or image signatures | Exact `Image/Bytes` one-row payload |
| Opaque binary fallback, `.bin`, `.logits`, `.model`, and empty GGUF/safetensors archives | Exact `Binary/Bytes` one-row payload |
| PPTX | Exact numeric `slide/I32` plus exact `text/Utf8` |
| GGUF metadata and tensors | Keys, types, and shapes are `Classify` or `Infer`; tensor rank and byte count are exact `Numeric/I32`; encoded tensor payload is exact `Binary/Bytes` |
| Safetensors metadata and tensors | Keys, types, and shapes are `Classify` or `Infer`; rank and byte count are exact `Numeric/I32`; encoded tensor payload is exact `Binary/Bytes` |
| Multi-source context columns | `source_index` is exact categorical dictionary int32; path, folder, file, member, digest, and related labels are `Classify`; byte counts, sample indices/counts, and depth are exact numeric int32 |

The source context is ordinary table data, not hidden control state. It is
prefixed with `data:` when it would collide with a payload header. It therefore
passes through the same semantic and preparation contracts as user columns.

`DistilledDataset` stores the final `RawTable`, rules, and file count. Its
`infer_vectors` method uses the stored rules on the full table. Its `prepare`
method instead calls `prepare_table_with_semantics`, so infer/classify rules
are resolved only on the retained training partition while exact source rules
remain exact. This distinction is critical for avoiding validation leakage.

The public facade in `src/data_prepare.rs` currently does two separate calls:
`distilled.infer_vectors(&CategoricalEncodingModel)` followed by
`prepare_inferred_table`. That path infers the whole distilled table before
fitting metadata. The lower-level `DistilledDataset::prepare` and
`prepare_table` paths implement the documented train-only semantic discovery;
callers that need that guarantee must use them rather than reusing a full-table
`InferredVectorList`.

## Fitting and applying a semantic schema

`ingest/src/prepare.rs` owns the stateful half of semantic encoding. The main
entry points are:

* `prepare_table` starts with no rules and delegates to
  `prepare_table_with_semantics`;
* `prepare_table_with_semantics` selects columns and rows, computes an exact
  rational train count, constructs a fit-only table, applies predicate-derived
  rules for all-missing fit columns, infers the fit table, validates the result,
  fits metadata, and applies schemas to all retained rows; and
* `prepare_inferred_table` accepts an already authoritative full-width
  `InferredVectorList`, validates its source indexes and names, and skips
  semantic re-inference. It still fits encoding metadata on train rows.

Selection and predicates occur before fitting. Predicates are resolved against
original source headers, evaluated before excluded columns are removed, and can
use a helper column that is not a final feature. The fit table contains only
the first exact train partition of retained rows. `fit_semantics_with_predicate_constraints`
can set an `Infer` column to an exact numeric or UTF-8 rule when a predicate
requires that type but every fit value is missing; this lets filtering proceed
without asking the ambiguity model to guess from no evidence. After fit, every
predicate is checked against the actual inferred semantic tuple.

`fit_vector_schema` creates row-free `VectorSchema` metadata:

* `I32`, `F32`, and `Utf8` need no fitted metadata;
* relative seconds receives the minimum fit `TemporalOrigin`;
* dictionary int32 receives the lexicographically ascending set of nonempty
  fit labels, with dictionary length checked against i32;
* ordinal int32 receives the one fit-resolved static vocabulary; and
* bytes receives `VectorMetadata::Image` with the deterministic set of image
  header variants only when the semantic type is `Image`, otherwise metadata is
  `None`.

`apply_vector_schema` then reads source values by original source row and
executes the lossless encoder selected by the tuple. All output vectors must
have exactly the retained-row count. The resulting `PreparedVector` keeps the
source index, byte name, feature/target role, semantic type, encoding,
metadata, and packed values. A `VectorSchema` is the same identity without row
values and is the form persisted by training.

### Encoding-specific behavior

* Numeric int32 and f32 values retain missing entries as `None`; present cells
  must be UTF-8 and pass the exact numeric contract. F32 values are stored as
  raw IEEE-754 bits.
* Temporal values retain missing entries as `None` and store checked relative
  whole-second int32 deltas from the fit origin.
* A categorical dictionary is a sorted set of nonempty fit labels. A known
  label receives its zero-based dictionary code. A missing cell receives
  `None` in training calculation storage. A nonempty label absent from the fit
  dictionary receives the reserved code `dictionary.len()`. The parallel
  `CategoricalObservation` records `Known`, `Missing`, or `Unseen { label }` and
  is checked against every code for alignment.
* An ordinal value receives the case-insensitive rank of its canonical label;
  missing is `None`, and an unrecognized nonempty value is an encoding failure.
* UTF-8 and byte vectors use offsets, payload bytes, and validity bits. UTF-8
  validates every retained nonmissing value. Image byte vectors additionally
  validate every retained nonmissing header, but validation-only image variants
  outside the fit set are not added to fitted metadata.

Dense matrix materialization is a later boundary. It rejects variable-width
vectors, missing values, an absent role, inconsistent lengths, and lossy i32
to f32 conversion. All-i32 vectors remain an i32 matrix; mixed/f32 vectors are
an f32 matrix only when every integer converts exactly. No semantic operation
silently turns text, binary, or images into scalar features.

## Predicate compatibility

Preparation uses semantic tuples to validate row-exclusion predicates. Signed
and unsigned literals require `Numeric/I32`; finite f32 literals require
`Numeric/F32`; text literals require a `Categorical`, `Ordinal`, or `Text`
vector. A mismatch is `PredicateTypeMismatch`. Missing predicate values are
`MissingPredicateValue`, and a value that fails the already fitted numeric or
UTF-8 contract is `InvalidPredicateValue`. Predicate selection never changes
the semantic type of a nonempty fit vector.

## Public training consumers

Training consumes the prepared tuple, not raw `SemanticType` in isolation.
The relevant downstream contracts are:

### Dense feature plan and model lowering

`training/src/model.rs::DenseFeaturePlan::from_prepared` accepts exactly:

* `Numeric/I32/None/PreparedValues::I32` or
  `Numeric/F32/None/PreparedValues::F32Bits` as one normalized scalar; and
* `Categorical/DictionaryI32/Categorical/PreparedValues::I32` as a one-hot
  span of `dictionary.len() + 1`, where the final index is the reserved
  missing/unseen route and categorical features are not numerically normalized.

Every other feature tuple, including temporal, ordinal, text, image, binary,
or a metadata mismatch, fails with an invalid feature matrix error until a
declared model family supplies a dedicated lowering. This is a deliberate
boundary, not an implicit cast.

Recurrent and embedding model validation is stricter: every feature must be a
numeric scalar with `None` metadata, and embedding token positions must retain
exact int32 values inside the checked vocabulary range.

### Dense targets and objectives

`resolve_dense_task` binds the objective to saved target semantics. Numeric
int32/f32 targets support scalar regression. Binary cross entropy or focal loss
accepts numeric targets with positive code 1, or a categorical dictionary with
at most two fitted labels and a positive code derived from dictionary length
(`-1`, `0`, or `1` for lengths zero, one, or two). Cross entropy requires a
categorical dictionary and adds exactly one output class for the reserved
unseen route. Multi-target objectives require homogeneous numeric int32/f32
targets. The lowering validates missing observations, exact conversions, code
ranges, and target matrix dtype before graph construction.

### KNN

`training/src/knn.rs` consumes every supported target tuple. Numeric targets
become finite f32 means. Categorical and ordinal labels use their fitted byte
labels and int32 codes. Temporal relative seconds become discrete int32 labels
decoded as exact relative values. Text, binary, and image variable-width bytes
become deterministic byte-label dictionaries. Missing references are excluded
per target; an output with no known training reference fails. KNN persists a
`CheckpointArtifactVector` for each source schema and keeps output dictionaries
so mode codes can be decoded back to exact semantic values.

### Observed categorical Bayesian models

`training/src/bayes.rs` requires child and parent vectors to be
`Categorical/DictionaryI32` with nonempty fitted dictionaries and i32 values.
Parents must be feature-role vectors; a target-as-parent edge is rejected. The
saved dictionary, reserved inference route, and raw training codes define the
native histogram and Laplace posterior. Numeric, ordinal, temporal, text,
image, and binary vectors are not coerced into Bayesian categorical nodes.

## Checkpoint and model persistence

`training/src/checkpoint.rs` converts each prepared `VectorSchema` into a
`CheckpointArtifactVector` or row-free `CheckpointVectorSchema` with source
index, byte name, role, semantic type, encoding, and cloned `VectorMetadata`.
The semantic and encoding strings are canonical:

```text
numeric | temporal | categorical | ordinal | text | image | binary
f32 | int32 | relative-seconds-int32 | dictionary-int32 |
ordinal-int32 | utf8 | bytes
```

Checkpoint decoding parses those enums strictly and parses vector metadata as
`none`, `temporal`, `categorical`, `ordinal`, or `image`. Manifest validation
accepts only the following tuples:

* numeric with f32 or int32 and `None` metadata;
* temporal with relative-seconds-int32 and temporal origin metadata whose
  nanoseconds are below one billion;
* categorical with dictionary-int32 and a canonical sorted byte dictionary
  (an empty dictionary is permitted for an all-missing fit partition);
* ordinal with ordinal-int32 and a nonempty distinct ordered-label list;
* text with UTF-8 and `None`;
* image with bytes and validated encoded-image variants; or
* binary with bytes and `None`.

The preparation layer can represent an all-missing ordinal fit with an empty
label list, but the checkpoint artifact validator does not accept that empty
ordinal metadata. Such a schema therefore fails at model persistence rather
than being silently assigned an invented order. Image checkpoint metadata is
similarly required to contain at least one validated fit variant, even though
the in-memory application path can validate an all-missing fit and later
nonmissing rows.

Any other semantic/encoding/metadata combination is rejected before graph or
native execution. Checkpoint task validation also derives class counts,
reserved codes, target dtypes, recurrent input requirements, embedding token
requirements, and feature-span lowerings from this persisted semantic tuple,
not from host casts. `artifact_metadata` copies temporal origin, dictionaries,
ordered labels, and image-header facts into the artifact; it does not retain
source rows.

The KNN checkpoint codec has the same canonical semantic and encoding strings
and validates its schemas before native inference. Thus semantic metadata is a
stable artifact contract, not an implementation detail of one training path.

## Target-free inference consumers

`ingest/src/inference.rs` is deliberately narrower than semantic distillation.
`InferenceFeatureSchema` stores only source-vector identity, exact byte name,
and one of `NumericI32`, `NumericF32`, or
`CategoricalDictionary { dictionary }`. It is produced from a saved checkpoint
feature span by `training/src/inference.rs::saved_feature_schema_from_parts`:

* a numeric scalar span must match `Numeric/I32/None` or
  `Numeric/F32/None`;
* a categorical one-hot span must match `Categorical/DictionaryI32`, have
  `dictionary_width == dictionary.len()`, `reserved_index == dictionary.len()`,
  and width `dictionary.len() + 1`; and
* all temporal, ordinal, text, image, binary, or inconsistent tuples fail as
  an inconsistent checkpoint rather than receiving a guessed inference path.

`prepare_inference_table` validates a nonempty schema, nonempty names, unique
source-vector identities, unique names, and canonical ascending nonempty
categorical labels (the empty dictionary is accepted as the all-missing fit
case). It resolves source columns by exact name bytes, allowing reordering and
unrelated columns but rejecting a missing or duplicate required column. It
parses every source row under the saved numeric contract. Numeric missing or
invalid values are path-addressed errors. Categorical known labels retain
their saved codes; both missing and unseen nonempty labels use the reserved
code `dictionary.len()` in the existing graph-facing values, while
`categorical_observations` records `Missing` versus `Unseen { label }`.

The returned values retain row count exactly and are serialized by
`inference_feature_bytes` as little-endian i32 or f32-bit payloads. The training
inference compiler marks these tensors as external inputs, performs numeric
conversion, categorical one-hot expansion, normalization, and model math as
Recipe calculations, and never re-infers semantics or admits checkpoint
optimizer state. KNN inference reuses the same feature-schema conversion and
Bayesian inference builds a union of saved categorical parent schemas in first
occurrence order.

## Failure surfaces and invariants

The semantic module itself exposes only two typed error kinds:

* `InconsistentWidth` stops before any column indexing when a row does not
  match the table width; and
* `ArithmeticOverflow` stops checked evidence aggregation.

Most cell-level failures intentionally belong to the later preparation
boundary, where a source column and source row are known. Important preparation
failures include semantic inference wrapping, invalid or inconsistent inferred
schemas, invalid temporal values or ranges, ambiguous ordinal fit vocabularies,
dictionary code overflow, image-header failures, categorical code/observation
misalignment, variable-width offset overflow, missing dense values, and lossy
mixed dense conversion. Predicate failures are path-addressed by column and
source row. Inference-time failures separately identify empty/invalid saved
schemas, missing or ambiguous required features, missing numeric values,
invalid values, and row-length arithmetic violations.

The following invariants are relied upon by every consumer:

* Every table column receives exactly one semantic type and one encoding.
* Source-column indexes and header bytes are preserved; inferred vectors never
  reorder or create columns.
* Empty bytes mean missing at the raw boundary. Categorical missing and unseen
  nonempty labels are distinct typed observations even when graph codes share
  one reserved route.
* Numeric values are admitted only through the exact int32/f32 contract. F32
  values are carried as bits, not host arithmetic results.
* Temporal origin, categorical dictionary, ordinal vocabulary, and image
  variants are fit from training rows only on the safe preparation path and
  then applied immutably to retained rows.
* Dictionaries and ordered labels are deterministic byte-order collections;
  reserved code is exactly the dictionary length.
* Variable-width text, image, and binary payloads remain offsets plus bytes and
  never become scalar dense features without an explicit model-family lowering.
* Checkpoint semantic tuples and metadata must be mutually compatible before
  inference or native execution.
* Runtime code receives prepared values and immutable schemas. Filesystem
  reading, semantic inference, fitting, and source-row filtering do not occur
  in the model loop.

## Source map

The implementation regions that define this contract are:

* `ingest/src/semantic.rs:8-47`: semantic and encoding enums and dtype map;
* `ingest/src/semantic.rs:49-171`: evidence, ambiguity-model boundary, and
  built-in nearest-example classifier;
* `ingest/src/semantic.rs:173-318`: inferred-vector values, errors, and table
  inference entry points;
* `ingest/src/semantic.rs:320-451`: parser precedence, encoding mapping,
  evidence arithmetic, and ratios;
* `ingest/src/semantic.rs:453-588`: temporal grammar and civil-date conversion;
* `ingest/src/semantic.rs:590-632`: ordinal vocabularies and fit-only
  disambiguation;
* `ingest/src/dataset.rs:112-203,268-485`: stored source rules, logical
  producers, rule merging, and `DistilledDataset` boundaries;
* `ingest/src/prepare.rs:798-872,1028-1123`: fit-only semantic discovery,
  authoritative inferred-table application, and validation;
* `ingest/src/prepare.rs:1536-1702,1763-2034`: metadata fitting and lossless
  per-encoding application;
* `ingest/src/inference.rs:6-499`: saved feature schemas and target-free
  schema-bound parsing;
* `training/src/model.rs:1151-1210`: dense scalar and categorical feature
  lowerings;
* `training/src/compile.rs:1390-1563`: task compatibility with semantic
  target tuples;
* `training/src/knn.rs:149-340,450-634`: semantic target reduction and exact
  label decoding;
* `training/src/bayes.rs:292-395,602-640`: categorical Bayesian role and
  dictionary boundary;
* `training/src/checkpoint.rs:380-556,2315-2440,5352-5413,9389-9428,
  10264-10322`: persisted schema, strict decoder, tuple validation, metadata
  conversion, and canonical string forms; and
* `training/src/inference.rs:783-910,4843-4955`: saved-schema reconstruction,
  target-free preparation, and external-input compilation.
