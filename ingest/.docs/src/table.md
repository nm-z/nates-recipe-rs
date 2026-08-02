# `recipe-ingest::table`

`ingest/src/table.rs` is Recipe's bounded, byte-preserving delimited-table
boundary. It frames an already admitted byte image into one rectangular
`RawTable`. It does not infer a semantic type, decode a numeric payload,
normalize a value, impute a missing value, or perform a model calculation.
Those operations belong to `semantic.rs`, `prepare.rs`, and their downstream
consumers.

The boundary has two entry points:

```text
Path + TableRequest  -> read_table  -> bounded file bytes -> parse_table
bytes + TableRequest -> parse_table -> RawTable
```

Both paths either return a complete table or an `IngestError`. There is no
partial-table result. `read_table` owns the one filesystem read; `parse_table`
is the same framing operation for bytes that a caller has already admitted.

## Public surface

The module is re-exported by `ingest/src/lib.rs`. The current root inventory is:

```text
Delimiter::{Auto, Comma, Semicolon, Tab, AsciiWhitespace}
HeaderMode::{Present, Absent}
IngestLimits::{new, source_bytes, records, fields_per_record, field_bytes}
TableRequest::{new, delimiter, header, limits}
RawTable::{delimiter, headers, rows, width}
IngestErrorKind::{InvalidLimit, Io, SourceLimitExceeded, RecordLimitExceeded,
                  FieldLimitExceeded, FieldByteLimitExceeded, MalformedTable,
                  InconsistentWidth, ArithmeticOverflow}
IngestError { kind, path, detail }
IngestResult<T>
read_table(path, request)
parse_table(bytes, request)
```

`IngestErrorKind` is `#[non_exhaustive]`; consumers must retain a wildcard
branch when matching it. `Delimiter`, `HeaderMode`, `IngestLimits`,
`TableRequest`, and `RawTable` all implement `Clone`, `Copy` where their fields
permit it, `Debug`, and equality as shown by the source derives.

## Framing data model

### `RawTable`

`RawTable` owns three private fields:

```text
delimiter: Delimiter
headers:   Vec<Vec<u8>>
rows:      Vec<Vec<Vec<u8>>>
```

The outer row vector is in source order. Every row is an ordered vector of
owned field byte vectors. Headers are also owned byte vectors and occupy the
same column positions as row fields. A field is not decoded as UTF-8 and is not
converted to a number. An empty byte vector is retained as-is. Later semantic
and preparation layers use empty fields as their missing-value representation,
so an empty source field and a parser-produced empty field have the same
downstream meaning.

The rectangular invariant is enforced by the crate-private
`RawTable::from_parts` constructor:

```text
for every row at index i:
    row.len() == headers.len()
```

An offending row returns `IngestErrorKind::InconsistentWidth` with a
one-based record number and no table is returned. `from_parts` sets
`delimiter` to `Delimiter::Auto`; this is intentional for tables assembled
from more than one logical source or produced by selection. Only
`assemble_table` preserves the delimiter selected for one parsed byte image.

`headers()` and `rows()` return shared slices. There is no public mutation API,
so callers cannot change one row without re-entering a checked constructor.
`width()` normally equals `headers.len()`, because valid tables are
rectangular. Its implementation is the maximum of the header length and the
first row length, which also gives a useful answer for the empty-header,
empty-row internal case. The delimiter accessor reports framing metadata only;
it does not alter the stored bytes or control later semantic inference.

Example layout:

```text
headers = [[b"name"], [b"age"]]
rows    = [
             [[b"Ada"], [b"37"]],
             [[b"",    ], [b"42"]],
          ]
```

The second row has a missing `name` value, but `RawTable` does not decide that
it is missing. It only owns the empty byte vector.

### Request and bounds types

`TableRequest` is a small, copyable declaration containing a delimiter choice,
a header mode, and one `IngestLimits` value. `TableRequest::new` is `const` and
does not validate anything beyond the already-constructed limits.

`IngestLimits::new(source_bytes, records, fields_per_record, field_bytes)`
requires all four bounds to be nonzero. It stores them as `NonZeroU64` or
`NonZeroU32`, so a valid request cannot silently carry an unbounded zero. The
constructor reports `InvalidLimit` and names the first zero bound it checks.
The accessors return the nonzero wrappers, not plain integers:

| accessor | bound applies to |
| --- | --- |
| `source_bytes()` | the complete source byte image passed to `parse_table`, and each file/member read by `read_table` or the dataset layer |
| `records()` | framed records, including a header record when `HeaderMode::Present` is used |
| `fields_per_record()` | fields in each framed record, before the header is removed or generated |
| `field_bytes()` | each decoded field byte vector, not the aggregate payload |

The limits are preparation input. The runtime loop never owns a file handle or
calls this parser.

## Delimiter and header semantics

### Explicit delimiters

`Delimiter::Comma`, `Semicolon`, and `Tab` select the corresponding CSV
separator byte. They all use the `csv` crate's byte-record reader with
`has_headers(false)` and `flexible(false)`. The parser therefore handles CSV
quoting and escaped quotes, preserves non-UTF-8 field bytes, and rejects a
record-width change as a CSV framing error. It does not treat the first record
as a header; `HeaderMode` is applied only after all records have been framed.

`Delimiter::AsciiWhitespace` uses a separate line parser. The source is split
on LF bytes, a trailing CR is removed from each line, and each remaining line
is split on every ASCII-whitespace byte. Empty tokens are discarded. Blank
lines are skipped and do not consume a record bound. Quotes have no special
meaning in this mode, so quote bytes remain part of a token. Width is checked
after all nonblank lines have been collected.

`Delimiter::Auto` is resolved once by `sniff_delimiter` and the selected value
is stored in the resulting `RawTable`. Sniffing examines only the first line
that contains a non-whitespace byte:

```text
1. Count comma, semicolon, and tab bytes outside a simple CSV quote state.
2. Sort by descending count, with Comma before Semicolon before Tab on ties.
3. If an unquoted ASCII space is present, no comma or semicolon was seen, and
   exactly two nonempty whitespace fields are visible, choose AsciiWhitespace.
4. Otherwise, return the highest nonzero punctuation count.
5. If all punctuation counts are zero but two whitespace fields are visible,
   choose AsciiWhitespace.
6. Otherwise, fall back to Comma.
```

The quote state toggles on `"`; a doubled `""` while quoted is skipped as one
escaped quote. This is a delimiter sniff only, not the full CSV parser. It
examines one line, so a delimiter that appears only on later lines does not
affect the choice. Punctuation inside quoted text is not counted.

### Header modes

After records have passed bound and width checks, `assemble_table` applies the
header mode:

| mode | result |
| --- | --- |
| `Present` | removes record zero and uses it verbatim as `headers`; remaining records become `rows` |
| `Absent` | retains every framed record as a row and creates `col1` through `colN` from the first record width |

The record bound is checked before this removal, so a present header counts as
one record. A source with one record and `Present` yields a header-only table
with zero rows. An empty source yields empty headers and rows regardless of
header mode. An absent header is never inferred from content.

## `read_table`: bounded filesystem path

`read_table(path, request)` performs exactly this sequence:

```text
metadata = path.metadata()
metadata.len() <= request.limits.source_bytes()
file = File::open(path)
reader = file.take(source_bytes + 1)
read_to_end(reader)
actual_len <= source_bytes
parse_table(bytes, request)
attach path to any parse error
```

The metadata check rejects an already-too-large file before opening it. The
`+1` read bound detects a file that grows after metadata was inspected. The
addition is checked and reports `ArithmeticOverflow` if it cannot be
represented. The initial allocation is the metadata length converted to
`usize`, capped at 1 MiB; that capacity is only an allocation hint and is not a
semantic limit. Read, metadata, open, and size errors are `Io`,
`SourceLimitExceeded`, or `ArithmeticOverflow` as appropriate, and include the
path.

The second `parse_table` call repeats the source-byte bound check. This is not
redundant protection against the caller supplying a byte slice larger than the
request. If parsing fails, `read_table` adds the original path to the
`IngestError`; a failure already carrying a path remains path-addressed.

## `parse_table`: byte-image framing

`parse_table(bytes, request)` has no filesystem side effects. Its ordered
operations are:

1. Convert `bytes.len()` to `u64`. A conversion failure is
   `ArithmeticOverflow`.
2. Reject a byte count above `limits.source_bytes()` with
   `SourceLimitExceeded`.
3. Resolve `Delimiter::Auto` with `sniff_delimiter`.
4. Dispatch to `parse_whitespace_records` for `AsciiWhitespace`, otherwise to
   `parse_csv_records`.
5. Assemble headers and rows with `assemble_table`.

The byte parser returns no table if any bound, framing, or rectangularity check
fails. It never performs type detection or payload calculation.

### CSV records

`parse_csv_records` maps the three CSV delimiters to `b','`, `b';'`, and
`b'\t'`. Passing `Auto` or `AsciiWhitespace` to this helper is an internal
contract violation and returns `MalformedTable`; public dispatch never sends
those values here. For every `ByteRecord`, it performs the record and field
checks before cloning fields into owned vectors. CSV reader failures, including
invalid quoting and non-flexible width failures, become
`MalformedTable` with the CSV error text. The records are then handed to
`assemble_table` for the header operation.

### ASCII-whitespace records

`parse_whitespace_records` skips empty lines, checks each nonempty line's field
count, checks every token's byte length, and clones the tokens. It calls
`validate_widths` after scanning all lines. A later line with a different token
count therefore returns `InconsistentWidth`, not a partial table.

### Width and assembly checks

`validate_widths` compares every record after record zero with the first
record's field count and reports the one-based record number. `assemble_table`
also invokes it before removing or generating headers. This keeps the
rectangular invariant independent of the parser selected above.

## Error contract

The public `IngestError` carries three fields:

```text
kind:   IngestErrorKind
path:   Option<PathBuf>
detail: String
```

`Display` emits `Kind: detail` and appends ` [path]` when available. The type
implements `std::error::Error`. `IngestError::new` and `for_path` are private,
so external producers inspect the public fields but cannot construct a parser
error through those helpers.

The failure categories in the current implementation are:

| kind | observed trigger |
| --- | --- |
| `InvalidLimit` | any `IngestLimits` bound is zero |
| `Io` | metadata, open, or read failure in `read_table` |
| `SourceLimitExceeded` | metadata, actual read length, or admitted byte slice exceeds `source_bytes` |
| `RecordLimitExceeded` | the next framed record would exceed `records`; this includes a present header |
| `FieldLimitExceeded` | a record has more than `fields_per_record`, or its field count cannot fit in `u32` |
| `FieldByteLimitExceeded` | one decoded field exceeds `field_bytes` |
| `MalformedTable` | CSV framing/quoting failure, or the CSV helper receives a non-CSV delimiter |
| `InconsistentWidth` | whitespace records, assembled records, or `RawTable::from_parts` are not rectangular |
| `ArithmeticOverflow` | checked conversion/addition for source length, record count, field length, or the `source_bytes + 1` read bound fails |

Errors are fail-closed. There is no fallback delimiter after a chosen parser
reports malformed input, and there is no truncation after a bound is exceeded.

## Producers of `RawTable`

The direct producers are the two public functions in `table.rs`:

| producer | delimiter field | source context |
| --- | --- | --- |
| `parse_table` | selected explicit delimiter or sniff result | none, because input is already a byte slice |
| `read_table` | same as `parse_table` | filesystem path attached to errors only |

The rest of the workspace uses the same framing contract through these paths:

### Dataset distillation

`dataset.rs::parse_delimited` chooses a request from the logical file type:

```text
csv                         -> Delimiter::Auto, HeaderMode::Present
tsv                         -> Delimiter::Tab,  HeaderMode::Present
all-data/dat/data/
data-numeric/tra/trn        -> structural delimiter or Auto, HeaderMode::Absent
txt or text-like tabular    -> structural delimiter or Auto, HeaderMode::Absent
```

It calls `parse_table`, converts the headers to `LogicalColumn::infer`, and
copies the rows into a `LogicalTable`. `Accumulator::append` then combines
logical tables from files, directories, archives, and container readers. It
retains source context when required, forms a union of column names, pads a
row with empty fields for a column absent from that source, and enforces
aggregate record/vector/byte limits. `Accumulator::finish` calls
`RawTable::from_parts`; consequently the final distilled table has
`Delimiter::Auto` even when one member was parsed as CSV or TSV. The
accumulator's final rectangularity check is the same `RawTable` invariant.

`DistilledDataset::table()` borrows this final table, `sample_count()` reads
`rows().len()`, `vector_count()` reads `width()`, `infer_vectors()` sends the
columns to semantic inference, and `prepare()` sends the table plus preserved
source semantic rules to preparation. `into_table()` transfers ownership.

### Preparation-derived tables

`prepare.rs` uses `RawTable::from_parts` for two internal views:

* `select_table` clones retained headers and retained source-row fields after
  target-free selection. It keeps original row and column order and therefore
  returns delimiter metadata `Auto`.
* `fit_partition_table` clones the original headers and only the first train
  rows used to fit semantic metadata. It is never exposed as a user dataset;
  it is a semantic-fitting view and also has delimiter metadata `Auto`.

No producer mutates a `RawTable` in place. New row or column selections are
new owned tables with the invariant rechecked.

## Consumers and column handoff

### Semantic inference

`semantic.rs::infer_table_vectors` validates each row length against
`table.width()`, then reads one positional byte slice from every row for each
column. It produces one `InferredVector` per source column, retaining the
source index and header bytes. The inference layer may recognize image,
temporal, ordinal, exact int32, or exact f32 values and otherwise delegates
ambiguous values to its classifier. It does not rewrite `RawTable` or create
derived columns. A width mismatch is `SemanticErrorKind::InconsistentWidth`;
checked evidence arithmetic can report `ArithmeticOverflow`.

### Training preparation

`prepare_table`, `prepare_inferred_table`, and `select_table` are the next
public boundaries:

```text
RawTable
  -> name/target/exclusion/predicate selection
  -> retained source-row indices
  -> exact rational train split
  -> fit-only semantic metadata
  -> all-retained-row encoding
  -> PreparedDataset
```

`PreparedDataset` is still preparation state, not runtime model state. It
retains original source-column identities, source-row identities, roles,
encodings, metadata, and lossless values for consumers in `training` and
`training::inference`.

### Schema-driven inference

`ingest/src/inference.rs::prepare_inference_table` consumes a `RawTable` and a
saved feature schema. It matches required columns by exact header bytes, so
source columns may be reordered and unrelated columns may be present. It does
not infer semantics, fit a dictionary, select targets, filter rows, or split
the input. Numeric missing values fail closed. For a saved categorical
dictionary, known labels use their dictionary code, while missing and unseen
labels both use the reserved calculation code; the parallel
`CategoricalObservation` route distinguishes `Missing` from `Unseen` and
retains unseen label bytes.

The checkpoint, KNN, and Bayesian inference paths in `training/src/inference.rs`
call this schema boundary. `training/src/gguf_llama.rs` is a specialized
consumer that instead requires `table.width() == 1`, treats each row's first
field as a whitespace-separated int32 token sequence, and checks token range
against the model vocabulary and context length.

### Model compilers

The training compilers consume `PreparedDataset`, not raw table bytes. Their
table-facing assumptions are explicit:

| consumer | table-derived contract |
| --- | --- |
| dense compiler | feature vectors must have a supported fixed-width numeric or categorical one-hot lowering; train and validation matrices use the prepared partition positions |
| KNN reference preparation | the train partition must be nonempty; features lower to a dense matrix and target values retain source-row alignment and missing/known state |
| observed categorical Bayesian preparation | the train partition must be nonempty; declared children are target-role dictionary vectors and parents are feature-role dictionary vectors |
| schema-driven checkpoint/KNN/Bayesian inference | only saved feature identities are read from a fresh `RawTable`; target columns are not required |

Variable-width text and image/binary values remain lossless bytes in ingest.
A downstream operation must provide a typed lowering before a dense calculation
can consume them. Ingest never silently casts or imputes them.

## Column selection and row predicates

The selection APIs resolve names against the original table before any column
is removed. This ordering is part of the contract.

### `select_table`

`select_table(table, excluded_columns, excluded_rows)` performs no semantic
inference, target selection, fitting, or partitioning. Its deterministic
sequence is:

1. Build a unique byte-name index from `table.headers()`.
2. Resolve every `ColumnPattern` against the original headers. Patterns are
   nonempty, ASCII-case-folded byte globs where `*` matches zero or more bytes
   and `?` matches one byte. An unmatched pattern fails.
3. Reject a request that excludes every source column.
4. Resolve row predicates against the original header index.
5. Evaluate every predicate on every original row. A row is excluded when any
   predicate is true, so predicates combine with logical OR. A missing value
   is an error, not a nonmatch.
6. Reject a result with no retained rows.
7. Clone retained columns and retained rows in original order and construct a
   fresh rectangular `RawTable`.

A helper/predicate column may therefore select rows without becoming a model
feature. Duplicate source names are rejected because named selection would be
ambiguous. A predicate literal is typed as signed integer, unsigned integer,
finite f32 bits, or text; malformed values and incompatible comparisons return
typed `PrepareError`s.

The selection failures that can be reached from this boundary are deliberately
typed:

| error kind | selection trigger |
| --- | --- |
| `DuplicateColumnName` | two original headers have the same bytes |
| `UnmatchedColumnPattern` | an exclusion glob matches no source header |
| `NoFeatureVectors` | every source column is excluded (or is a target) |
| `TargetNotFound` / `DuplicateTarget` | a declared target is absent or repeated |
| `TargetExcluded` | an exclusion glob also matches a declared target |
| `PredicateColumnNotFound` | a row predicate names no source header |
| `InvalidPredicateLiteral` | an f32 literal is non-finite |
| `PredicateTypeMismatch` | a source value or fitted semantic encoding cannot satisfy the literal type |
| `MissingPredicateValue` | a predicate reaches an empty source field |
| `InvalidPredicateValue` | a fitted numeric/text value cannot be parsed for its predicate |
| `NoRetainedRows` | predicates exclude every source row |

### Training preparation selection

`prepare_table` performs the same name, target, exclusion, and predicate
resolution before semantic fitting. It additionally requires at least one
target, rejects duplicate or missing targets, rejects an exclusion that also
matches a target, and requires at least one feature after target/exclusion
resolution. `prepare_inferred_table` validates that a caller-supplied
`InferredVectorList` has exactly one vector for each table column, with matching
index and name, then applies the same selection rules without re-inference.

Rows are filtered before the split. The retained and excluded source-row lists
are stored as original table indices, so a later error or downstream metric can
identify the source row that produced it.

## Exact split and fit semantics

`TrainFraction` is an exact rational value. `TrainFraction::new` requires
`0 < numerator < denominator`, reduces the fraction by the greatest common
divisor, and stores a nonzero denominator. `TrainFraction::from_f32` decodes
the finite f32 bit pattern into an exact binary rational before reducing it;
values outside `(0, 1)`, non-finite values, and exact denominators that cannot
fit in `u64` fail with `InvalidTrainFraction`.

For `retained_rows`, the train count is the checked floor:

```text
train_rows = floor(retained_rows * numerator / denominator)
validation_rows = retained_rows - train_rows
```

The multiplication is checked in `u128`. There is no randomization,
rounding-through-host-f32, or row shuffling. The first `train_rows` retained
source rows are the fit and training partition; the remainder is validation.
The API does not itself reject a zero train count when a small retained set and
a fraction produce a floor of zero. Dense and Bayesian/KNN training consumers
require a nonempty train partition and report their own typed training error.

Automatic preparation fits semantic metadata only from a temporary table made
from those train rows. Validation rows therefore cannot introduce a temporal
origin, ordinal vocabulary, categorical dictionary, or image-variant metadata.
The fitted schema is then applied to every retained row. A caller-supplied
`prepare_inferred_table` keeps the supplied semantic type and encoding but
still fits encoding metadata on the train rows. `select_table` has no split at
all.

Predicate typing is also fit-aware. Before inference, a signed or unsigned
predicate requests numeric/int32 semantics, an f32 predicate requests
numeric/f32 semantics, and a text predicate requests text/UTF-8 semantics. If
the predicate column has no nonempty value in the fit rows and its source rule
is still `Infer`, preparation installs that exact rule so the predicate can be
checked against the fitted table. After fitting, every predicate is validated
again against the inferred semantic type and encoding. A real mismatch remains
`PrepareErrorKind::PredicateTypeMismatch`; it is not silently coerced.

## Prepared column storage

Preparation converts each retained source column into one `PreparedVector`.
The vector retains:

```text
source_index: original RawTable column index
name:         original header bytes
role:         Feature or Target
semantic_type and encoding
metadata:     fitted reversible state
values:       one entry per retained source row
```

`PreparedVector::schema()` removes row values while preserving the identity and
metadata needed by a checkpoint. `PreparedDataset::vectors()` remains in
source-column order after exclusions. `target_source_indices()` separately
preserves the user's target declaration order; model outputs must use this
list rather than assuming target vectors were physically moved.

The value representation is:

| `PreparedValues` variant | storage | missing representation |
| --- | --- | --- |
| `I32(Vec<Option<i32>>)` | exact int32 or relative/ordinal/dictionary codes | `None` (dictionary unseen values use the dictionary-length reserved code) |
| `F32Bits(Vec<Option<u32>>)` | exact IEEE-754 f32 bits | `None` |
| `VariableWidth(VariableWidthVector)` | offsets plus one concatenated byte payload | `valid[index] == false`; an empty source byte vector contributes no payload bytes |

`VariableWidthVector` has `offsets` with one more entry than rows,
`payload`, and one `valid` flag per retained row. `value(position)` returns
`None` for an out-of-range position, `Some(None)` for a missing value, and
`Some(Some(bytes))` for a valid payload slice. UTF-8 vectors validate UTF-8
before storage; byte/image vectors retain arbitrary bytes.

Dictionary vectors also carry one `CategoricalObservation` per retained row:

```text
Known { code }
Missing
Unseen { label }
```

The calculation-facing code and observation route are checked for equal length
and alignment. Missing stays `None`; an unseen nonempty label receives the
reserved code equal to `dictionary.len()` and retains its original bytes in the
observation.

`VectorMetadata` records only state needed to invert the encoding. Temporal
vectors store the minimum fit instant as an origin and encode whole-second
relative offsets. Categorical vectors store a sorted fit dictionary. Ordinal
vectors store their recognized ordered vocabulary. Image vectors store the
exact encoded variants observed in fit values. Text and binary vectors have no
scalar dtype and remain variable-width.

### Partitions and matrices

`PreparedPartition` stores retained positions and their corresponding original
source-row indices. The train partition covers positions `0..train_rows`; the
validation partition covers the remaining positions. Every prepared vector
and every categorical observation route has exactly `retained_source_rows.len()`
entries, and partition positions index those arrays directly.

`PreparedDataset::fixed_dense_matrix(role, partition)` selects vectors by role
in prepared source order and emits a row-major `DenseMatrix`. All-i32 vectors
produce `DenseMatrix::I32`. A mixture of i32 and f32 vectors produces f32 bits
only when every integer is exactly representable. Missing values, variable
width vectors, absent roles, lossy integer conversion, and inconsistent vector
lengths fail closed with a `PrepareError`.

The dense compiler, KNN reference builder, and recurrent validation consume
these matrices and partition maps. They do not revisit raw bytes or perform a
second source-row selection.

## Cross-layer invariants

The following invariants are the safe handoff from table framing to the rest of
Recipe:

```text
source bytes are bounded before framing
each framed record is bounded by field count
each field is bounded by decoded field byte length
all records are rectangular before RawTable is returned
headers and rows use the same positional column order
RawTable owns bytes but performs no payload calculation
selection resolves names and predicates before columns are removed
row exclusions happen before the exact rational train split
fit metadata sees train rows only
prepared vectors preserve source indices and retained-row alignment
runtime consumers receive prepared values, not filesystem handles
```

No fallback parser, retry, truncation, imputation, or parallel public table
representation exists in this boundary. The fit and selection views described
above are checked, short-lived `RawTable` values, not alternate public storage
models. A malformed transition remains visible as its typed error and must be
repaired by the caller or the producer that supplied the invalid source.

## Edge cases worth preserving

* A zero limit is rejected at `IngestLimits::new`; the parser never treats zero
  as unlimited.
* A present header consumes one record bound. A header-only source is valid and
  has zero data rows; an empty source has zero width.
* Whitespace parsing skips blank lines, while CSV record accounting follows the
  records returned by the CSV reader.
* Auto detection examines only the first nonblank line, ignores punctuation in
  quoted spans, uses comma as the punctuation tie-breaker, and falls back to
  comma when no structural evidence exists.
* CSV width failures are reported by the non-flexible CSV reader as
  `MalformedTable`; whitespace and crate-internal assembled width failures are
  `InconsistentWidth`.
* A table assembled from a directory, archive, multiple source declarations,
  fit rows, or selected rows has delimiter metadata `Auto`, because one
  delimiter cannot describe all of those origins.
* Empty fields remain empty bytes. Numeric preparation can preserve them as
  missing `Option` values, while schema-driven inference deliberately fails on
  missing numeric features.
* Named selection requires unique header bytes. Exact byte names, not decoded
  Unicode or case-folded names, identify a column; case folding applies only to
  exclusion glob matching.
* A tiny retained set can produce zero train rows under floor rational split.
  The table and preparation layers preserve that result; training entry points
  that require observations reject it explicitly.

This is the complete table boundary. New source formats should produce logical
tables through the same bounded `parse_table` or an equivalent producer that
ends at `RawTable::from_parts`; they should not add a second row/column
representation or bypass the rectangular and limit checks.
