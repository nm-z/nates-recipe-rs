<!--
This document describes the implementation in ingest/src/prepare.rs.  The
source is the authority for details that are not part of the public comments.
Line references below are intentionally approximate anchors, not a second API.
-->

# Table preparation

`ingest/src/prepare.rs` is the boundary between an owned, rectangular
`RawTable` of source bytes and an immutable, typed `PreparedDataset`.  It owns
the training-time policy that is not part of lexical ingestion:

- resolving declared target names and case-insensitive exclusion globs;
- evaluating typed row predicates before columns are dropped;
- constructing a deterministic train/validation partition;
- discovering or accepting one semantic/encoding identity per source column;
- fitting encoding metadata on the train rows only on the fit-only entrypoint;
- applying that metadata losslessly to every retained row; and
- exposing fixed-width dense projections without imputation or lossy casts.

It does not read files, infer source framing, normalize values, derive
features, run a model, or own a runtime loop.  `RawTable` has already copied
the source fields as byte vectors, and all output vectors remain aligned to
the retained source-row order.  The calculation graph and native executors
consume the prepared values later.

The module is exported from `ingest/src/lib.rs` as `TrainFraction`,
`ColumnPattern`, `RowPredicate`, `PreparationRequest`, the prepared value and
schema types, `PrepareError`, and the three public boundaries
`prepare_table`, `prepare_inferred_table`, and `select_table`.

## Input state and request

`RawTable` is an ordered header vector plus a rectangular vector of rows.  Each
field is still source bytes: an empty field is the module's missing-value
representation.  A source row index always refers to the row in this original
table, even after filtering and partitioning.  `RawTable::from_parts` checks
rectangularity when a table is created; preparation performs additional
identity and alignment checks because an inference result can be supplied by
a caller.

`PreparationRequest` is immutable policy assembled by value:

```text
targets:           Vec<Vec<u8>>       declaration order is significant
excluded_columns:  Vec<ColumnPattern>
excluded_rows:     Vec<RowPredicate>
train_fraction:    TrainFraction
```

`PreparationRequest::new` does not validate the target list.  Validation occurs
when a table is resolved.  `exclude_columns` and `exclude_rows` append to the
existing lists and preserve request order.  Exclusion results are sets, while
target order is retained separately.

### Exact train fractions

`TrainFraction` stores a reduced rational `(numerator, denominator)` with a
nonzero denominator and `0 < numerator < denominator`.  `new` rejects zero,
one, reversed values, and a zero denominator with
`InvalidTrainFraction`.  Reduction uses an integer greatest-common-divisor,
so no floating-point operation participates in a split.

`from_f32` first rejects non-finite values and values outside `(0, 1)`.  It
decomposes the exact IEEE-754 finite binary value, removes powers of two from
the numerator, and constructs the corresponding power-of-two denominator.  A
reduced denominator that cannot fit in `u64` is rejected.  `TryFrom<f32>` is
the same operation.

For `retained_rows`, `train_rows` computes

```text
floor(retained_rows * numerator / denominator)
```

using checked `u128` multiplication and a checked conversion back to `usize`.
There is no minimum-train-row rule here.  A valid fraction and a small retained
set can therefore produce a zero-row fit partition.  Later semantic fitting
and encoding decide whether that state is usable.

### Column patterns

`ColumnPattern::new` requires a nonempty byte pattern.  `*` matches zero or
more bytes, `?` matches one byte, and all other bytes are literal.  Matching is
ASCII case-insensitive per byte, so `*ID*` matches `PatientId`.  The matcher
is a single-pass glob routine with backtracking to the most recent `*`; it
does not interpret a pattern as a regular expression.

### Row predicates

`RowPredicate` identifies a source column by exact header bytes, a
`ComparisonOperator` (`Equal`, `NotEqual`, `Less`, `LessOrEqual`, `Greater`,
or `GreaterOrEqual`), and one `PredicateLiteral`:

- `Signed(i64)` and `Unsigned(u64)` compare parsed decimal integers;
- `F32Bits(u32)` compares an exact finite IEEE-754 binary32 value; and
- `Text(String)` compares UTF-8 source text lexicographically.

The constructor is intentionally declarative and does not parse source rows.
Multiple predicates are combined with logical OR: a row is excluded as soon
as one predicate evaluates true.  An empty source field is not comparable and
fails with `MissingPredicateValue`, rather than being treated as false.

## Output state

`PreparedDataset` contains the complete result of preparation:

```text
source_row_count:      original table row count
retained_source_rows:  original indices that survived row predicates
excluded_source_rows:  original indices excluded by a predicate
vectors:               retained columns, in original source-column order
target_source_indices: target source indices, in user declaration order
train:                 retained positions [0, train_rows)
validation:            retained positions [train_rows, retained_len)
```

The retained and excluded row vectors are complementary because filtering
visits every source row in order.  `vectors` contains no column selected by an
exclusion pattern, but source indices and names are not renumbered.  A target
that was declared second or third in the request can consequently appear at a
later source position in `vectors`; `target_source_indices` is the separate,
authoritative output order for consumers that produce one result per declared
target.

### Vector identity and metadata

`InferredVector` (from `semantic.rs`) records source index, name, semantic type,
encoding, and fit evidence.  `VectorSchema` is the row-free identity copied
into each `PreparedVector` and later into checkpoints.  It contains:

```text
source_index, name, role (Feature or Target), semantic_type,
encoding, metadata
```

`PreparedVector` adds packed values and, for dictionary categorical vectors,
one `CategoricalObservation` per retained row.  Its `schema()` method drops
the row data while preserving all semantic identity and fitted metadata.

`VectorMetadata` is the inverse information needed by a consumer:

- `None` for ordinary `I32`, `F32`, UTF-8, or opaque byte vectors;
- `Temporal { origin }`, where the origin is the fit minimum instant used by
  relative-second encoding;
- `Categorical { dictionary }`, where code `n` names `dictionary[n]` and
  `dictionary.len()` is the reserved calculation route for a nonempty value
  absent from the fit dictionary;
- `Ordinal { ordered_labels }`, where code `n` is the fitted rank; and
- `Image { encoded_variants }`, the deterministic set of image headers seen
  while fitting.

The image metadata comment describes retained nonmissing values, but the
implementation intentionally fits the set from train values only.  Applying
the schema validates every retained image header and does not extend the
fitted set with validation-only formats or shapes.

### Packed values

`PreparedValues` is one of:

- `I32(Vec<Option<i32>>)` for exact integer, temporal, dictionary, and ordinal
  encodings;
- `F32Bits(Vec<Option<u32>>)` for exact binary32 bit patterns; or
- `VariableWidth(VariableWidthVector)` for UTF-8 and opaque bytes.

Missing fixed-width values are `None`.  `F32Bits` stores bits rather than host
floating-point values so preparation cannot accidentally perform arithmetic or
canonicalize a source NaN, although the contract parser rejects non-finite
decimal input for ordinary numeric fields.

`VariableWidthVector` is an Arrow-like offset/payload/validity representation:
`offsets` starts with zero and has one final offset, `payload` is the
concatenation of every source byte sequence including empty fields, and
`valid` is false for an empty field.  `value(retained_row)` returns
`None` for an invalid row index or malformed offset conversion, `Some(None)`
for a missing field, and `Some(Some(bytes))` for a valid slice.

`CategoricalObservation` is a lossless parallel route, not a replacement for
the calculation codes:

```text
Known { code }       code is in the fitted dictionary
Missing              source field was empty
Unseen { label }     source field was nonempty but absent from fit dictionary
```

The reserved code is used for `Unseen`, while `Missing` remains `None` in the
training `PreparedValues::I32`.  The observation vector preserves the
distinction and exact unseen label bytes for target and non-training consumers.

### Partitions and dense matrices

`PreparedPartition` stores both retained positions and their original source
rows.  Positions index every `PreparedVector` value array.  `Train` is the
contiguous prefix chosen by the exact rational split; `Validation` is the
remaining suffix.  No randomization, row copying, or independent per-vector
partitioning occurs.

`DenseMatrix` is an explicit row-major projection with either `I32` values or
`F32Bits` values and recorded row and column counts.  It is requested with
`PreparedDataset::fixed_dense_matrix(role, partition)`, not built implicitly
by preparation.  The method filters vectors by role in source order, rejects
an empty role selection, and rejects any variable-width vector.  If every
selected vector is `I32`, it emits an `I32` matrix.  Otherwise it emits an
`F32Bits` matrix, preserving f32 values and converting each i32 only after an
exact `f64` round-trip check.  Missing values, lossy integer conversions,
missing retained positions, and matrix-capacity overflow fail closed.  There
is no normalization, imputation, feature derivation, or implicit categorical
one-hot expansion in this method.

## Main preparation pipeline

The fit-only path is `prepare_table(table, request, model)`, which delegates to
the crate-private `prepare_table_with_semantics(table, request, model, &[])`.
`DistilledDataset::prepare` calls the same function with source-owned semantic
rules.  The implementation order is the anti-leakage contract:

```text
RawTable + request + semantic rules
  -> resolve names, targets, column exclusions, and row predicates
  -> filter source rows, preserving original order
  -> exact train-row count from retained rows
  -> clone only the train prefix as the fit table
  -> apply source rules and predicate constraints to fit semantics
  -> infer one vector identity per fit-table column
  -> validate that inference still describes the full source table
  -> validate predicate types against fitted semantics
  -> fit row-free metadata from train values
  -> apply each fitted schema to all retained rows
  -> make contiguous train and validation partitions
  -> PreparedDataset
```

Each stage returns the first typed error.  There is no partial output and no
fallback path.

### 1. Resolve names and targets

`select_rows_and_columns_before_fit` first builds a `BTreeMap` from original
header bytes to source index.  Duplicate names fail with
`DuplicateColumnName`, because all named target and predicate selection must be
unambiguous.  `resolve_targets` then requires at least one target, resolves
each exact byte name, rejects a missing target, and rejects a repeated source
index.  It returns both the declaration-ordered vector and a set used for
membership checks.

### 2. Resolve column exclusions

Every `ColumnPattern` is matched against every source header.  A pattern that
matches nothing is `UnmatchedColumnPattern`.  If a match includes a target,
preparation fails with `TargetExcluded`; target columns cannot silently vanish.
Matched non-target indices accumulate in a `BTreeSet`, so overlapping patterns
are harmless and output order remains source order.

The no-feature check occurs before rows are filtered.  If every source column
is a target or excluded column, the request fails with `NoFeatureVectors`.

### 3. Resolve and evaluate row predicates before fitting

The fit-only path resolves predicate names against the original headers, then
assigns a provisional type based on the literal:

```text
Signed or Unsigned -> Numeric / I32
F32Bits            -> Numeric / F32
Text               -> Text / Utf8
```

An f32 literal must be finite or `InvalidPredicateLiteral` is returned.  At
this stage `fitted_type` is false because semantic inference has not happened.
The actual source table is then scanned row by row.  For each predicate:

1. a missing column value is `InconsistentInference`;
2. an empty value is `MissingPredicateValue`;
3. integer literals parse the UTF-8 value as `i64` or `u64`;
4. f32 literals parse through `parse_contract_f32` and compare finite values;
5. text literals parse UTF-8 and compare strings; and
6. the operator is applied to the resulting `Ordering`.

An invalid UTF-8 or numeric source value in this pre-fit pass is
`PredicateTypeMismatch`, because the provisional literal type cannot be
applied.  A row is appended to `excluded_source_rows` when any predicate is
true, otherwise to `retained_source_rows`.  Source order is unchanged.  An
empty retained set is `NoRetainedRows`.

Predicates run before excluded columns are removed.  A helper column can
therefore control row selection and then be excluded from model vectors.

### 4. Split retained rows exactly

The exact rational fraction is applied to the count of retained rows, not the
original count.  The resulting `train_rows` is a prefix length.  The first
`train_rows` retained source indices are copied into a fit `RawTable`, with the
original headers and cloned source fields.  A source-row index outside the
original table is `InconsistentInference`; failure to rebuild the rectangular
fit view is reported under the same kind.

The fit table is an owned semantic view only.  It is not the final dataset and
does not alter source indices.

### 5. Establish fit semantic rules

`fit_semantics_with_predicate_constraints` creates one rule per fit-table
column.  A caller-supplied rule at that index is preserved; missing entries
default to `VectorSemanticRule::Infer`.  For each predicate, if its fit column
contains no nonempty value and its rule is still `Infer`, the provisional
literal rule is promoted to `Exact(Numeric/I32)`, `Exact(Numeric/F32)`, or
`Exact(Text/Utf8)`.  This lets an all-missing fit column retain a declared
comparison contract without inventing a model classification.  An explicitly
`Classify` or `Exact` source rule is never overwritten.

`infer_table_vectors_with_semantics` then validates every fit row width and
emits one `InferredVector` per column.  For `Infer`, the parser/classifier order
is image signature, temporal syntax, ordinal vocabulary, exact int32, exact
f32, then the ambiguous model.  `Classify` calls only the model and chooses
the encoding implied by its returned semantic type.  `Exact` uses the caller's
semantic type and encoding unchanged.  Evidence counts are exact integers and
thousandths ratios, so classification is deterministic.

The resulting list is checked against the original table, not just the fit
view.  Its length must equal the table width, and vector `index` and `name`
must match each corresponding original header.  Any mismatch is
`InconsistentInference`.  Semantic failures are converted to
`PrepareErrorKind::SemanticInference` while retaining the semantic error text.

Finally, every predicate is checked against the fitted semantic type and
encoding.  Signed/unsigned literals require `Numeric/I32`, f32 literals require
`Numeric/F32`, and text literals require `Categorical`, `Ordinal`, or `Text`.
Fitted non-finite f32 literals remain invalid.  A mismatch is
`PredicateTypeMismatch`.  This second check is what prevents a predicate from
silently forcing a semantic interpretation merely because its literal had a
particular Rust type.

### 6. Fit schemas and apply them

`prepare_preselected_table` takes only inferred vectors not in the exclusion
set.  For each vector it assigns `Feature` or `Target` by target membership,
then `fit_vector_schema` reads the exact fit rows and builds row-free metadata.
The schema is applied to all retained rows by `apply_vector_schema`.  The
metadata is immutable after fitting; validation values can be rejected by it
but cannot change it.

Every resulting value array, and every categorical observation array, must
have exactly `retained_source_rows.len()` elements.  A mismatch is
`InconsistentPreparedVector`.  Partitions are then materialized from ranges
over the retained positions, and the final `PreparedDataset` records the
original row count, retained/excluded source indices, source-ordered vectors,
declaration-ordered target indices, and both partitions.

## Authoritative-inference and selection paths

### `prepare_inferred_table`

This public path accepts an `InferredVectorList` as an authoritative semantic
contract.  It first runs `validate_inference` against the full table, then
resolves targets, exclusions, and predicates against that supplied list.  It
filters rows before the exact split and still fits temporal origins,
dictionaries, ordinal vocabularies, and image variant metadata on the train
prefix.  It never re-infers semantic types or encodings.  This path is the
right boundary when a caller has already made a deliberate semantic choice;
the automatic fit-only callers are `prepare_table` and
`DistilledDataset::prepare`.

`select_rows_and_columns` differs from the pre-fit resolver in one important
way: predicate types are validated against the supplied inference before rows
are scanned.  Consequently parse/UTF-8 failures during filtering are
`InvalidPredicateValue` (`fitted_type = true`) rather than the provisional
`PredicateTypeMismatch` used by automatic fit selection.

### `select_table`

`select_table` is target-free row/column selection.  It constructs an internal
request with an irrelevant valid `1/2` fraction, resolves names from headers,
resolves exclusions and pre-fit predicates, filters rows, and rebuilds a new
rectangular `RawTable` from retained columns and rows.  It does not infer
semantics, fit metadata, or construct train/validation partitions.  Both
headers and rows preserve their original relative order.  The internal
fraction exists only because the shared request type carries a fraction; no
partition is returned.

This is the selection boundary used by target-free inference.  It is also why
predicate columns can be helpers: filtering happens against the original table
before the rebuilt table drops excluded columns.

## Fitting each encoding

The following rules are implemented by `fit_vector_schema` and
`apply_vector_schema`.

### Ordinary numeric values

`I32` and `F32` have no metadata.  Every nonempty retained field must be UTF-8
and pass the corresponding contract parser.  `encode_i32` stores exact signed
values, and `encode_f32` stores exact bits.  Empty fields become `None`.
Invalid UTF-8, a malformed decimal, a value outside the contract, or a
non-finite f32 is `EncodingFailure` with the vector name and source row.

### Temporal values

`RelativeSecondsI32` is fit from the nonempty fit values.  Every fit value must
parse using `parse_temporal_instant`.  The origin is the minimum parsed instant;
if the fit has no nonempty values, the origin is Unix second zero and
nanosecond zero.  Applying the schema parses every nonempty retained value and
computes a signed nanosecond delta in checked `i128`.  The delta must be a
whole number of seconds and fit `i32`.  A malformed instant is
`EncodingFailure`, nanosecond misalignment is `EncodingFailure`, arithmetic
overflow is `ArithmeticOverflow`, and an out-of-range relative second is
`TemporalRangeExceeded`.

The temporal parser accepts a strict `YYYY-MM-DD` date, optionally a `T` or
space time with seconds, optional up to nine decimal fractional digits (extra
digits must be zero), and no timezone, `Z`, or an explicit `+/-HH:MM` offset.
Date-only values are midnight UTC.  These parser rules live in `semantic.rs`,
but their fit-origin and relative encoding are owned here.

### Dictionary categorical values

`fit_dictionary` collects every nonempty fit byte value into a `BTreeSet`, so
the dictionary is unique and ascending by raw bytes.  Empty fit evidence is
allowed and yields an empty dictionary; a dictionary length that cannot be
represented by the reserved i32 route is `EncodingFailure`.

`encode_dictionary` builds known codes `0..dictionary.len()`.  Empty source
fields produce `(None, Missing)`.  A known label produces its code and
`Known { code }`.  A nonempty label absent from the fit dictionary produces
the reserved code `dictionary.len()` and `Unseen { label }`.  The helper then
checks every code and observation pair with `validate_categorical_alignment`.
Misaligned lengths, negative/out-of-range known codes, an empty unseen label,
or a code not matching its observation route are
`InconsistentPreparedVector`.

The reserved route is deliberately calculation-facing.  Consumers that need
to distinguish missing from unseen must use `categorical_observations`, not
the i32 code alone.

### Ordinal values

The fit partition's nonempty labels are passed to `fit_ordinal_vocabulary`.
The vocabulary is selected only if exactly one declared order accepts every
fit label.  One fit label can identify one vocabulary; empty fit evidence gives
an empty ordered-label list.  Ambiguous or unrecognized fit evidence is
`EncodingFailure`.  Applying the schema maps labels case-insensitively to
their fitted rank, preserves empty values as `None`, and rejects a retained
nonempty label not in the vocabulary with `EncodingFailure`.

Automatic semantic classification uses the complete vector presented to it,
but fit-only preparation presents only the fit rows.  Thus validation-only
ordinal labels cannot disambiguate an otherwise ambiguous fit vocabulary.

### UTF-8 text and opaque bytes

`Utf8` and `Bytes` use `VariableWidthVector`.  UTF-8 vectors validate every
nonempty retained value as UTF-8; opaque bytes are copied unchanged.  The
image semantic uses `Bytes` plus fit-time `inspect_image_variants`; all
nonempty retained values, including validation rows, must have recognized image
headers when the schema is applied.  Payloads are never decoded into
calculation tensors here.

## Error model

`PrepareError` is a stable, non-exhaustive category plus optional source-column
bytes, optional original source row, and a human-readable detail.  `Display`
prints the kind and detail, followed by column and row context when present.
The categories and their observable causes are:

| Kind | Cause in the preparation boundary |
| --- | --- |
| `SemanticInference` | `semantic.rs` rejected fit-table width/evidence or another semantic failure. |
| `InvalidTrainFraction` | zero/out-of-range fraction, non-finite f32, or an exact f32 denominator that does not fit `u64`. |
| `InvalidColumnPattern` | empty `ColumnPattern`. |
| `EmptyTargetSet` | no declared targets. |
| `DuplicateTarget` | the same resolved target source index appears twice. |
| `DuplicateColumnName` | named selection would be ambiguous. |
| `TargetNotFound` | a declared target header is absent. |
| `UnmatchedColumnPattern` | an exclusion glob matches no source column. |
| `TargetExcluded` | a glob also matches a declared target. |
| `NoFeatureVectors` | every source column is a target or excluded. |
| `NoRetainedRows` | row predicates exclude every source row. |
| `PredicateColumnNotFound` | a predicate names no source column in the table or fit semantics. |
| `InvalidPredicateLiteral` | an f32 predicate contains NaN or infinity. |
| `PredicateTypeMismatch` | a literal cannot be used with the provisional or fitted semantic/encoding, or a pre-fit value cannot be parsed under the literal type. |
| `MissingPredicateValue` | a predicate attempted to compare an empty source field. |
| `InvalidPredicateValue` | a supplied semantic contract was fitted, but a filtered value is malformed or not valid UTF-8 for its predicate. |
| `InconsistentInference` | width/index/name identity differs, a fit row is out of range, a source field is absent, or a rebuilt selection is not rectangular. |
| `EncodingFailure` | a lossless numeric, temporal, categorical, ordinal, UTF-8, or image encoding cannot be completed. |
| `TemporalRangeExceeded` | a relative temporal second does not fit `i32`. |
| `VariableWidthDenseMatrix` | a requested dense role contains text, image, or binary storage. |
| `MixedDenseEncoding` | an i32 value cannot be represented exactly when mixed with f32 columns. |
| `MissingDenseValue` | a dense projection would have to impute a `None`, which it never does. |
| `EmptyDenseSelection` | no vector has the requested role. |
| `InconsistentPreparedVector` | schema metadata, value lengths, categorical observations, or reserved routes disagree. |
| `ArithmeticOverflow` | checked row counts, matrix sizes, offsets, temporal deltas, or conversions overflow. |

Errors produced while loading or framing a source (`IngestError` and
`DatasetSourceError`) are outside this module.  Public `src/data_prepare.rs`
wraps those errors with its own `DataPreparationError` without exposing a
partial `PreparedDataset`.

## Distilled dataset semantic state

`DistilledDataset` in `ingest/src/dataset.rs` owns a `RawTable`, its source
file count, and one `VectorSemanticRule` per merged header.  Logical source
columns can be `Infer`, `Classify`, or `Exact(semantic_type, encoding)`.  When
multiple files contribute the same header, an agreement preserves the rule;
a disagreement resets that header to `Infer`.  This state is source-owned
semantic intent, not fitted metadata.

`DistilledDataset::infer_vectors(model)` runs the semantic inference function
on its whole table with those source rules.  `DistilledDataset::prepare` instead
passes the rules into `prepare_table_with_semantics`, so row filtering and the
exact split happen before semantic discovery.  This latter operation is the
leak-free automatic boundary described above.

There is an important distinction in the current public facade.  In
`src/data_prepare.rs`, `prepare_data_with_limits` validates the public
declaration, converts its f32 split and conditions, distills the source, calls
`distilled.infer_vectors(&CategoricalEncodingModel)` on the full distilled
table, and then calls `prepare_inferred_table`.  Therefore:

```text
public prepare_data_with_limits:
  semantic identity is supplied from full-table inference
  encoding metadata is still fitted on the retained train prefix

DistilledDataset::prepare / prepare_table:
  semantic identity and encoding metadata are both discovered from fit rows
```

`prepare_inferred_table` is intentionally authoritative once given the
inferred list, so it does not re-infer validation rows.  The distinction is an
observable call-graph fact and should not be erased when reasoning about
leakage or reproducibility.

## Public data declaration path

`src/data_prepare.rs` is the adapter from the public `Data` declaration:

1. `prepare_data` obtains finite default ingest bounds and delegates.
2. `prepare_data_with_limits` validates the declaration, requires at least one
   target and an explicit split, converts the split to `TrainFraction`, and
   maps exclusions and conditions to this module's request types.
3. `distill_data_with_limits` reads the declared files, directories, or nested
   archives into one bounded `DistilledDataset`.
4. The adapter performs full-table `infer_vectors` as described above.
5. `prepare_inferred_table` performs row/column selection, train fitting, and
   schema application.

The resulting `PreparedDataset` is the only data object handed to the training
compilers.  No filesystem handle, parser callback, or source snapshot remains
part of the runtime graph.

For target-free inference, `select_target_free_data` maps public exclusions
and conditions and calls `select_table`.  It returns a selected `RawTable`,
not a `PreparedDataset`; the saved model schema is applied later by
`ingest::prepare_inference_table`, which has different rules: it performs no
semantic inference, dictionary fitting, target selection, or split, and it
requires every saved numeric feature to be present and nonmissing.  This is a
separate inference boundary, not a hidden mode of `prepare_table`.

## Consumers of `PreparedDataset`

### Dense training

`training/src/compile.rs` builds a dense task and calls
`DenseFeaturePlan::from_prepared`.  That plan consumes feature vectors in
source order:

- Numeric `I32` or `F32` vectors become one normalized scalar each.
- Dictionary categorical vectors become one-hot spans of
  `dictionary.len() + 1`, with the final slot reserved for missing/unseen.
- Other semantic/encoding combinations are rejected until a dedicated
  lowering exists.

`lower_dense_features` uses the plan.  If there are no categorical spans it
calls `PreparedDataset::fixed_dense_matrix`; otherwise it walks each
partition position, converts numeric features exactly, and emits categorical
one-hot bits.  `lower_dense_targets` uses `target_source_indices` for target
order and the partition's retained positions for rows.  It maps missing and
unseen categorical targets to distinct `TargetObservation` states so
validation can be filtered and supervision can be represented without
pretending an unknown label is a fitted class.

`resolve_dense_task` also treats the prepared target contract as authoritative:
one-target binary loss accepts explicit numeric 0/1 or at most two fitted
categorical labels, scalar regression accepts only numeric fixed-width targets,
and multiclass cross-entropy requires a dictionary categorical target and adds
exactly one reserved unseen class.  A multi-target loss requires at least two
declared targets, preserves their request order, and requires homogeneous
numeric fixed-width target meanings.  These checks occur in training, after
`prepare.rs` has already guaranteed target identity and lossless storage.

Embedding and recurrent first-block consumers impose narrower contracts on the
same vectors.  Embeddings require every feature to remain numeric `I32` with
nonnegative in-range token IDs.  RNN, GRU, and LSTM first blocks require one
numeric scalar (`I32` or `F32`, no metadata) per feature-column time step.  A
semantic type that preparation can represent but the selected model cannot
lower is rejected by that consumer, not silently coerced by preparation.

`CompiledDatasetSchema::from_prepared` copies every vector schema and the
declaration-ordered target identities into the compiled model.  Checkpoint
serialization stores this row-free semantic contract, including temporal
origins, dictionaries, ordinal labels, and image variants, while source rows
and transient `PreparedValues` remain training inputs rather than model schema.

### KNN

`training/src/knn.rs::prepare_knn_reference_set` requires a nonempty training
partition and at least one declared target.  It builds the same dense feature
plan, lowers train features, then walks `target_source_indices` in declaration
order.  Numeric target references use exact numeric state; categorical and
ordinal targets use fitted dictionaries or ordered labels, while temporal and
variable-width targets derive deterministic labels from their fitted train
values.  Missing references are kept in row alignment but excluded
independently per output.  The saved KNN artifact retains schemas, feature
spans, normalization mask, train source rows, and reference values.

### Observed categorical Bayes

`training/src/bayes.rs` requires every declared child to be a prepared target,
every parent to be a prepared feature, and every node to be
`Categorical/DictionaryI32` with nonempty fitted dictionary metadata.  It uses
train partition positions and exact category codes to build observed
conditional reference sets.  Target source identities must exactly equal the
declared child order.  Missing or reserved unseen codes are rejected by the
observed complete-row contract rather than silently counted as a fitted class.

### Target-free model inference

The inference compiler does not reuse `PreparedDataset` because targets and
train/validation state are not part of a query.  It distills and selects with
`select_table`, then applies a saved feature schema through
`prepare_inference_table`.  Checkpoint, KNN, and Bayes inference each reuse
their saved dictionaries and source identities, and source columns may be
reordered or contain unrelated columns.  This preserves the same semantic
metadata contract without allowing query rows to refit it.

## Invariants to preserve

The implementation relies on the following invariants rather than defensive
fallbacks:

1. Headers are unique whenever named selection is performed.
2. Every inferred vector list has exactly table width and matching source index
   and name for each column.
3. Targets are present, unique, never excluded, and retained in declaration
   order separately from source vector order.
4. Predicates are evaluated against original columns before column exclusion,
   and any true predicate excludes the row.
5. Retained rows preserve original source order; the split is one contiguous
   exact-rational prefix and suffix.
6. Every fitted schema is built from fit rows and is applied unchanged to all
   retained rows.
7. Every prepared vector and categorical observation route has exactly one
   entry per retained source row.
8. Fixed-width values remain lossless; variable-width values remain bytes with
   explicit validity and offsets.
9. Dense projections are explicit and fail on missing, variable-width, or
   lossy values instead of imputing or silently converting.
10. Calculation-facing reserved categorical codes are never used as the sole
    source of observation identity; the typed observation route remains
    authoritative when missing and unseen differ.

The code checks these invariants at the boundary where they become relevant,
returns a typed error with source context, and leaves the underlying failure
visible to its caller.
