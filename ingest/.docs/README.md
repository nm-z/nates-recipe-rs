Recipe ingest
=============

`recipe-ingest` is the pre-run boundary between external representations and
Recipe's typed, immutable execution inputs. It owns source reads, structural
framing, format inspection, decimal contract validation, semantic vector
classification, train/validation fitting, schema-bound inference input, and
init-image packing. It does not own payload calculations, normalization
calculations, kernel generation, scheduling, device allocation, or runtime
filesystem access.

That division is intentional. A successful ingest operation returns owned
bytes, borrowed validated ranges, or typed host metadata that can be closed
over before `init -> loop -> exit` preparation. A malformed source, an
unbounded source, an ambiguous contract, or a representation that cannot be
losslessly encoded fails before a partial result is exposed.

The module comments describe the same boundary in source terms:

* [`src/lib.rs`](../src/lib.rs) calls this dependency-clean external
  representation handling and says that numerical preprocessing remains a GPU
  calculation.
* [`src/source.rs`](../src/source.rs) closes the file before returning a
  content-addressed snapshot.
* [`src/table.rs`](../src/table.rs) returns raw framed bytes without type
  detection or payload calculation.
* [`src/prepare.rs`](../src/prepare.rs) fits state on the exact train
  partition, then applies that state to retained rows.
* [`src/inference.rs`](../src/inference.rs) reuses a saved feature schema and
  never re-fits it.

## Crate manifest and module graph

`ingest/Cargo.toml` defines package `recipe-ingest`, version `0.1.0`, Rust
edition 2024, MIT license, and the description "Dependency-clean external
representation and framing for Recipe". Unsafe code is forbidden and all
Clippy lints are denied for this crate.

The dependency graph is deliberately small and format-oriented:

| Dependency | Boundary it supplies |
| --- | --- |
| `recipe-core` | `DType`, byte/value/device identifiers, digest, and finalized init-image manifests. |
| `recipe-ogdl` | The graph parser used by structural GGUF OGDL reconstruction. |
| `calamine` | XLSX worksheet access. |
| `csv` | RFC-style delimited record framing. |
| `quick-xml` | PPTX slide XML extraction. |
| `serde`, `serde_json` | Strict JSON and safetensors header decoding. |
| `sha2` | SHA-256 source identity and dataset metadata. |
| `zip` with only `deflate` | Bounded ZIP, XLSX, PPTX, and nested archive traversal. |

The root workspace depends on this crate as `recipe-ingest`; `recipe-training`
uses it for prepared vectors, checkpoint schema application, GGUF Llama input,
and model source snapshots; `recipe-text` uses snapshots for tokenizer files;
the root facade re-exports it as `recipe::engine::ingest`. The CLI uses the
streaming GGUF/OGDL conversion functions. The ingest crate itself has no
dependency on a runtime executor or GPU backend.

`src/lib.rs` declares and re-exports these modules:

| Module | Primary responsibility | Public surface |
| --- | --- | --- |
| `source` | One bounded regular-file read and SHA-256 identity. | `SourceLimit`, `SourceSnapshot`, `read_source_snapshot`, source errors. |
| `table` | Delimiter framing into rectangular byte cells. | `Delimiter`, `HeaderMode`, `IngestLimits`, `TableRequest`, `RawTable`, `read_table`, `parse_table`. |
| `dataset` | Recursive files/directories/ZIPs and format dispatch into one rectangular dataset. | `SourceFormat`, `DistilledDataset`, `distill_dataset`, `distill_datasets`, dataset errors. |
| `numeric` | Six-significant-digit f32 and nine-significant-digit int32 admission. | Decimal result types, constants, and contract parsers. |
| `semantic` | Lossless semantic type and encoding selection. | `SemanticType`, `VectorEncoding`, evidence, classifier, inferred vectors. |
| `prepare` | Selection, predicate filtering, fit-only metadata, encoding, partitioning, and dense projection. | Request, schema, prepared dataset/vector/partition types and preparation functions. |
| `inference` | Exact saved-schema application to target-free rows. | Inference schemas, typed values, prepared inference dataset, errors. |
| `image_header` | Header-only inspection of encoded image files. | Image format, color, layout, range, and metadata enums. |
| `image` | Placement of external values into finalized per-device init images. | `ExternalValue`, `PackedInitImage`, `pack_init_images`. |
| `gguf` | Bounded GGUF v2/v3 container parser with borrowed tensor spans. | GGUF limits, metadata/tensor types, archive, parser, errors. |
| `safetensors` | Bounded safetensors header and contiguous data-span validator. | Dtypes, limits, entries/archive, parser, errors. |
| `gguf_ogdl` | Typed, reversible structural GGUF v3 to/from OGDL, including seekable streaming. | Model descriptor, conversion functions, path-addressed errors. |

The important internal direction is one way: `dataset` composes `source`,
`table`, `gguf`, `safetensors`, and image inspection; `prepare` composes
`semantic`, `numeric`, and image inspection; `inference` composes `table`,
`numeric`, and categorical observations; `gguf_ogdl` composes only the GGUF
parser and OGDL graph API. The public facade exposes the result types without
exposing private parser helpers.

## Core representation and ownership rules

### Source snapshots

`SourceLimit` is a nonzero byte bound. `read_source_snapshot(path, limit)`:

1. opens the path and obtains metadata from the open handle;
2. rejects anything that is not a regular file;
3. refuses metadata larger than the limit;
4. reads through `limit + 1`, so a concurrently growing file cannot bypass
   the limit;
5. refuses a post-read size above the limit;
6. computes SHA-256 over the admitted bytes; and
7. returns `SourceSnapshot { path, bytes, digest }` with the handle closed.

The metadata length is only an early refusal and allocation hint. No callback,
file descriptor, or re-read capability survives the call. `SourceErrorKind`
covers `InvalidLimit`, `Io`, `NotRegularFile`, `LimitExceeded`, and
`ArithmeticOverflow`. A failure never returns a partial snapshot.

### Raw tables

`RawTable` is ordered headers plus ordered rows of owned `Vec<u8>` fields. It
deliberately has no inferred type, encoding, imputation, scaling, or derived
feature. `RawTable::from_parts` verifies that every row has the header width.
Its `width` is the header width, or the first-row width when a caller creates a
headerless shape.

`IngestLimits` fixes four nonzero bounds for one framing boundary:

* `source_bytes`: admitted input bytes;
* `records`: records seen by the parser, including a header record when the
  selected header mode removes it later;
* `fields_per_record`: maximum fields in one record; and
* `field_bytes`: maximum bytes in one field.

`TableRequest` combines those limits with `Delimiter` (`Auto`, comma,
semicolon, tab, or ASCII whitespace) and `HeaderMode` (`Present` or
`Absent`).

`read_table` performs one bounded file read and then calls `parse_table`.
`parse_table` first checks the supplied byte slice against `source_bytes`,
selects an explicit delimiter or deterministic sniffing, frames records, and
assembles a rectangular table. CSV framing uses the `csv` crate with
`flexible(false)`, so malformed quoting and width differences fail. Whitespace
framing splits ASCII whitespace, drops empty fields and blank lines, and then
checks equal widths. Header-present mode removes the first record; absent mode
creates `col1`, `col2`, and so on. Empty input returns an empty table.

The automatic delimiter sniff examines the first nonblank line, ignores quoted
delimiters, ranks comma before semicolon before tab for ties, and selects ASCII
whitespace only when it sees two or more whitespace-separated fields and no
comma/semicolon evidence. `IngestErrorKind` reports invalid limits, I/O,
source/record/field bounds, malformed framing, inconsistent width, or
arithmetic overflow.

## Recursive dataset distillation

### Public entry points and result

`distill_dataset(path, limits)` is the one-source convenience wrapper around
`distill_datasets`. The collection function materializes declared paths in the
caller's order, rejects an empty declaration, and applies byte, record, and
vector limits to the aggregate, not independently per declaration. Source
order becomes row order.

`DistilledDataset` contains:

* one rectangular `RawTable`;
* one `VectorSemanticRule` per resulting column, preserving exact declarations
  made by specialized readers and `Infer`/`Classify` for ordinary data;
* `file_count`, the number of admitted leaf files; and
* helpers to inspect the table, count samples/vectors/files, infer vectors with
  an `AmbiguousVectorModel`, prepare using fit-only semantics, or consume the
  table.

The collection accumulator merges columns by exact byte header. A repeated
header with conflicting semantic rules is deliberately downgraded to `Infer`.
Rows are extended with empty cells when a later file introduces a new global
column. Within one logical member, empty headers become `colN`; duplicate local
headers receive `#2`, `#3`, and so on. Source-context names that would collide
with ordinary data are prefixed with `data:`. Every admitted field is checked
against both the aggregate source bound and, for textual values, the field
bound. An empty logical table contributes one all-missing row so a valid empty
member cannot silently disappear.

### Path and traversal checks

Each declared source is inspected with `symlink_metadata`; symbolic links are
rejected at the root and at every directory or archive level. A regular file is
read through `read_source_snapshot` using the source-byte component of
`IngestLimits`. Directories are read in sorted filename order and traversed
recursively. Directory members receive a logical path rooted at the directory
name and deterministic source depth.

ZIP containers are recognized by a `.zip` extension or the PK signatures. The
archive member list is checked with `enclosed_name`, which rejects paths that
escape the archive root, then sorted by enclosed path. Directory members are
ignored. Each declared uncompressed member size must fit the per-source bound;
the actual read is capped at `source_bytes + 1` and checked again. Nested ZIPs
are visited recursively, with `ARCHIVE_DEPTH_LIMIT = 32`; a depth at or beyond
that limit fails before descending. An archive with no files is an
`EmptySource` error.

XLSX and PPTX are ZIP-backed formats but are handled as specialized leaves,
not as general recursive members. Their preflight sums all declared expanded
member sizes and refuses an expansion above `source_bytes` before the format
reader runs. Other ZIP members go through the normal nested dispatch.

Aggregate accumulator bounds are checked before appending a logical table:

* `leaf_bytes` is the sum of all admitted leaf bytes;
* `file_count` counts leaves; and
* prospective row count and global vector width are checked before mutation.

`DatasetSourceErrorKind` distinguishes `InvalidPath`, `Io`, `Symlink`,
`EmptySource`, `LimitExceeded`, `MalformedArchive`, `MalformedFormat`,
`Ingest`, and `ArithmeticOverflow`. The error carries the logical path when
there is one.

### Source-context vectors

Context columns are included for every member of a directory or archive and
for every file when more than one source was declared. A single regular file
does not acquire these columns. The fixed context width is 14:

| Column | Rule and value |
| --- | --- |
| `source_index` | Exact categorical code, declaration index. |
| `source_path` | Classified logical path. |
| `parent_path` | Classified parent path. |
| `folder` | Classified parent folder name. |
| `file_name` | Classified filename. |
| `file_stem` | Classified stem. |
| `extension` | Classified lowercase extension. |
| `format` | Classified `SourceFormat::as_str()` value. |
| `member` | Classified logical member name, such as a JSON table or sheet. |
| `content_sha256` | Classified lowercase SHA-256 of the leaf bytes. |
| `file_bytes` | Exact int32 decimal byte count. |
| `sample_index` | Exact int32 ordinal within the logical member. |
| `sample_count` | Exact int32 logical-member row count. |
| `source_depth` | Exact int32 path depth. |

The context is ordinary data, not hidden control state. It is therefore
available to selection, semantic fitting, and model targets just like any
other vector.

### Format dispatch

The dispatch in `parse_leaf` is extension-first, then signature/content based:

| Input | Reader and resulting logical columns |
| --- | --- |
| `.csv` | Auto-sniffed delimited table with a present header. |
| `.tsv` | Tab-delimited table with a present header. |
| `.all-data`, `.dat`, `.data`, `.data-numeric`, `.tra`, `.trn` | Delimited table with no header. |
| `.json` | Shape-sensitive JSON table conversion. |
| `.gguf` | Validated GGUF metadata and tensor tables, or one binary payload when empty. |
| `.safetensors` | Validated metadata and tensor tables, or one binary payload when empty. |
| `.png` with a PNG signature | One exact `Image`/`Bytes` payload. |
| `.inp`, `.out`, `.patch`, `.sh` | One exact UTF-8 `Text`/`Utf8` payload. |
| `.txt` with a structural delimiter | Headerless delimited table. |
| `.txt` otherwise | One exact UTF-8 text payload. |
| `.bin`, `.logits`, `.model` | One exact `Binary`/`Bytes` payload. |
| Other recognized image signature | One exact encoded-image payload. |
| Other probably-text content with a structural delimiter | Headerless delimited table. |
| Other valid text | One exact UTF-8 text payload. |
| Other bytes | One exact binary payload. |

The structural delimiter probe examines up to eight nonblank lines and requires
the same width on every examined line. Comma, semicolon, and tab candidates
ignore quoted separators. ASCII whitespace is accepted only when every field
parses as an ordinary `f64`; this is only a shape probe, not the numeric
admission contract used later.

#### Delimited text

`parse_delimited` selects tab for `.tsv`, auto for `.csv`, and the structural
probe (falling back to auto) for other tabular extensions. It calls
`parse_table`, then transfers headers and rows into a `LogicalTable` with
`Infer` rules.

#### Plain text and opaque bytes

Plain text must be UTF-8 and may contain only horizontal tab, line feed, and
carriage return as ASCII control bytes. It becomes one row with one exact
`Text`/`Utf8` vector. Opaque data remains one exact `Binary`/`Bytes` vector;
no CPU decoding is attempted.

#### JSON

`serde_json` parses one complete value. Arrays of objects become a union of
object keys in first-seen key order, with missing members empty. Arrays of
arrays become `col1..colN` rows padded with missing cells. Other arrays become
one `value` vector. A top-level object is split into separate tables when all
values are arrays of objects. When all values are objects, a second split shape
is recognized when each object maps members to arrays; otherwise keys are
prepended as a `key` vector. Null becomes a missing byte cell, booleans and
numbers use their JSON text, strings retain UTF-8 bytes, and nested arrays or
objects retain compact JSON text.

#### XLSX

`calamine` opens the bounded workbook after ZIP expansion preflight. Each
nonempty worksheet is one logical table. If the first row consists entirely of
numeric-like cells (`Int`, `Float`, `Bool`, date/time, or duration), it is data
and generated `colN` headers are used. Otherwise the first row supplies headers,
with empty header cells replaced by `colN`. Rows are resized to the header
width. Empty cells become missing bytes; all other cells use their textual
representation. A workbook with no worksheets or no nonempty sheets fails.

#### PPTX

`quick_xml` reads `ppt/slides/slideN.xml` members in numeric slide order.
Paragraph text is joined from text runs, line breaks become spaces, and empty
paragraphs are dropped. The table has exact numeric `slide` (`I32`) and exact
UTF-8 `text` columns. Missing slides or malformed XML fail.

#### Images

Image files remain encoded file bytes. Header inspection is separate from
decoding and is used by semantic inference and preparation. Recognized
signatures are PNG, JPEG, GIF87a/GIF89a, BMP, and RIFF WebP.

`EncodedImageMetadata` always states `EncodedFile` layout and `EncodedBytes`
range, so it never claims that the payload contains decoded pixels. The
header-only validators establish:

* PNG IHDR length, dimensions, 31-bit dimensions, legal color-type/sample-bit
  combinations, reserved fields, and IHDR CRC;
* GIF logical-screen dimensions and, when present, a complete global color
  table;
* JPEG marker structure, a start-of-frame, sample precision, dimensions,
  component table, and color-model evidence from JFIF/Adobe/component IDs;
* BMP DIB size, dimensions, positive/nonzero geometry, one color plane, and
  supported bits-per-pixel interpretations;
* WebP RIFF length, chunk bounds and padding, and VP8X/VP8L/VP8 dimension and
  alpha headers or a VP8 key-frame header.

Zero dimensions, truncated headers, invalid marker/chunk structure, reserved
bits, bad PNG CRC, or impossible lengths fail image inspection. A recognized
signature alone is enough to classify a leaf during distillation; preparation
performs the full header validation for every nonmissing retained value.

#### GGUF and safetensors as dataset tables

The dataset reader constructs parser limits from the leaf byte length. GGUF
metadata becomes a `metadata` table with `metadata_key`, `metadata_type`, and
`metadata_value`; tensors become a `tensors` table with classified name/type/
shape, exact numeric rank/byte count, and an exact binary encoded span. A
container with neither metadata nor tensors is retained as one binary payload.
Safetensors uses the analogous `metadata` and `tensors` tables, with dtype
text and the validated encoded tensor span. This is inspection and framing,
not dequantization or numerical calculation.

## Numeric admission contract

`numeric.rs` is used wherever text becomes a calculation-facing scalar. It
rejects empty strings, whitespace, separators, NaN, infinity, and syntax outside
the selected contract.

`parse_contract_f32` accepts a signed finite decimal with an optional point and
base-ten exponent, at most six significant digits, and a nonzero value that
does not underflow to f32 zero. It compares the f64 reference and f32 value at
the declared precision, preserving signed zero and rejecting any changed
round-trip. `F32Decimal` retains raw f32 bits, validated precision, canonical
scientific notation, and little-endian bytes.

`parse_contract_i32` accepts a signed integer decimal with at most nine
significant digits and an exact i32 parse. `I32Decimal` retains the exact value,
digit count, and little-endian bytes. Decimal failures are classified as
`Empty`, `InvalidSyntax`, `TooManySignificantDigits`, `OutsidePayloadRange`,
or `PrecisionLoss`.

## Semantic vector inference

`SemanticType` is the complete source classification set:
`Numeric`, `Temporal`, `Categorical`, `Ordinal`, `Text`, `Image`, and
`Binary`. `VectorEncoding` is the calculation-facing or lossless storage
choice: `F32`, `I32`, `RelativeSecondsI32`, `DictionaryI32`, `OrdinalI32`,
`Utf8`, or `Bytes`. Only the fixed-width encodings expose a `recipe_core::DType`;
UTF-8 and opaque bytes remain variable-width until a declared operation gives
them a typed lowering.

`infer_table_vectors` validates rectangular rows and collects exact evidence for
each column: total/missing/present values, unique count, UTF-8 count, whitespace
count, source byte total, dictionary byte estimate, mean present width, and
ratios in thousandths. Classification tries lossless parsers in this order:

1. all present values have a recognized image signature: `Image`/`Bytes`;
2. all present values parse as the supported temporal syntax:
   `Temporal`/`RelativeSecondsI32`;
3. the values match one of six case-insensitive ordinal vocabularies with at
   least two distinct ranks: `Ordinal`/`OrdinalI32`;
4. all present values pass the int32 contract: `Numeric`/`I32`;
5. all present values pass the f32 contract: `Numeric`/`F32`; and
6. the remaining vector is delegated to the supplied `AmbiguousVectorModel`.

The temporal parser accepts `YYYY-MM-DD`, optional `T` or space time,
`HH:MM:SS`, up to nine fractional digits (extra nonzero digits are rejected),
`Z` or signed hour/minute offsets, leap-year dates, and years 1 through 9999.
Date-only values mean midnight UTC. The ordinal vocabularies are low/medium/
high, small/medium/large, beginner/intermediate/advanced, poor/fair/good/very
good/excellent, first through fifth, and bronze/silver/gold/platinum.

The default `CategoricalEncodingModel` is an auditable nearest-example model,
not a feature generator. Any non-UTF-8 present value is categorical. Otherwise
it compares unique ratio, whitespace ratio, mean width, and whether the
dictionary representation is no larger than source bytes against seven fixed
categorical/text examples. It returns only `Categorical` or `Text` for the
ambiguous case. Specialized readers can supply `Classify` or `Exact` rules;
conflicting rules merged across files are reset to `Infer`.

`VectorEvidence` and `InferredVector` preserve evidence alongside the selected
type, encoding, index, and byte header. No new column is invented.

## Training preparation

### Request and selection

`PreparationRequest` contains declared target header bytes, optional case-
insensitive byte globs for excluded columns, optional row predicates, and an
exact `TrainFraction`. Targets are resolved by exact header bytes and retain
declaration order. Duplicate targets, missing targets, duplicate source names,
unmatched exclusion globs, exclusion of a target, or a selection with no
features fail before encoding.

`ColumnPattern` uses `*` for zero or more bytes and `?` for one byte. Matching
folds ASCII case only. `RowPredicate` supports equal, not-equal, less,
less-or-equal, greater, and greater-or-equal against signed/unsigned integer,
finite f32-bit, or text literals. Multiple predicates are ORed: a row is
excluded when any predicate is true. Predicates are resolved against the full
source table before excluded columns are removed, so a helper column can filter
rows without becoming a feature.

`TrainFraction::new` requires `0 < numerator < denominator` and stores a reduced
rational. `from_f32` reconstructs the exact finite binary32 fraction and rejects
values outside `(0,1)` or an exact denominator that cannot fit u64. The train
row count is floor(retained_rows * numerator / denominator), with checked
arithmetic and no randomization.

### Fit, then apply

`prepare_table` is the automatic path:

1. resolve targets, excluded columns, and predicates against original headers;
2. filter rows before fitting and retain source-row indices;
3. compute the exact rational train count;
4. build a fit-only `RawTable` from the first train source rows;
5. force a predicate column's semantic rule from its literal only when its fit
   values are all missing;
6. infer vectors on the fit table, never using validation rows;
7. validate the inferred list against the original table and revalidate
   predicate types after fitting;
8. fit one immutable `VectorSchema` per nonexcluded column; and
9. apply every schema to every retained row, then construct train and
   validation partitions.

`DistilledDataset::prepare` is the corresponding source-aware path. It carries
the specialized `VectorSemanticRule` declarations from format distillation into
the same fit-only pipeline. `prepare_inferred_table` accepts an already saved
or caller-authoritative `InferredVectorList`; it validates identity and still
fits metadata only on the train rows. `select_table` performs only row/column
selection and rectangular reconstruction for target-free inference; it uses a
valid internal half split solely to satisfy the request type and does not fit
semantics or partition output.

### Fitted metadata and encodings

`VectorSchema` is row-free identity: source index, exact name bytes, feature or
target role, semantic type, encoding, and `VectorMetadata`.

* Numeric `I32` and `F32` use the decimal contract for every retained nonmissing
  value. Missing values stay `None`.
* Temporal metadata stores the minimum nonmissing fit instant as
  `TemporalOrigin`. Every retained instant must be representable as a whole-
  second i32 offset from that origin. Nanosecond arithmetic is checked; a
  fractional relative offset or i32 overflow fails.
* Categorical metadata stores a canonical byte-sorted dictionary from nonmissing
  fit values. Known values receive their dictionary code, missing values remain
  `None`, and a nonempty retained value absent from the fit dictionary receives
  the reserved code `dictionary.len()`. `CategoricalObservation` separately
  records `Known`, `Missing`, or `Unseen { label }`, preserving unseen bytes.
* Ordinal metadata stores one uniquely identified fit vocabulary. Every retained
  nonmissing value must belong to it and receives its rank; an ambiguous or
  empty fit vocabulary fails when a value requires ordinal encoding.
* Image metadata stores the deterministic set of encoded headers observed in
  nonmissing fit values. Every retained image header is revalidated, but
  validation-only variants are not added to fitted metadata. Image payloads are
  retained as variable-width encoded bytes.
* UTF-8 text and other bytes use an offset/payload/validity
  `VariableWidthVector`. UTF-8 values are validated; bytes remain opaque.

`PreparedVector` holds the fitted schema, `PreparedValues`, and optional typed
categorical observations. `PreparedValues` is `I32(Vec<Option<i32>>)` for
fixed integer routes, `F32Bits(Vec<Option<u32>>)` for f32 routes, or a
variable-width offset/payload vector. All vector lengths remain aligned to the
retained source-row list.

`PreparedDataset` records total source rows, retained and excluded source-row
indices, vectors in source-column order, target source indices in declaration
order, and `Train`/`Validation` partitions as retained positions plus original
source rows. The separate target index list prevents source-column layout from
reordering model outputs.

### Dense projection

`fixed_dense_matrix(role, partition)` is an explicit final projection, not part
of fitting. It rejects an absent role, variable-width vectors, missing cells,
inconsistent vector lengths, or arithmetic overflow. All-I32 vectors produce an
I32 row-major matrix. A mixed fixed-width selection produces f32 bits only when
every i32 value converts exactly and round-trips through f64; otherwise it
returns `MixedDenseEncoding`. No normalization, imputation, one-hot expansion,
or feature derivation occurs in this crate.

`PrepareErrorKind` gives callers a stable failure boundary for semantic
inference, invalid fractions/patterns, target/column selection, predicate
typing and values, no retained rows/features, inconsistent inference, temporal
range, encoding, dense conversion, and arithmetic errors. Column and source-row
coordinates are attached when a value-level failure identifies them.

## Target-free inference preparation

`InferenceFeatureSchema` is the saved model contract for one feature: source
vector identity, exact source header bytes, and one of `NumericI32`,
`NumericF32`, or `CategoricalDictionary { dictionary }`. It is not a request to
infer semantics again.

`prepare_inference_table(table, schema)` performs these checks and operations:

1. reject an empty schema;
2. require every saved name to be nonempty, every source-vector identity to be
   unique, and every categorical dictionary to be nonempty, strictly ascending
   byte labels with a reserved code representable in i32;
3. index actual table headers by exact bytes;
4. resolve each saved feature to exactly one source column, allowing source
   columns to be reordered and ignoring unrelated columns;
5. parse every row under the saved numeric contract, rejecting missing numeric
   values; or
6. map categorical known labels to saved codes and map both missing and unseen
   labels to the reserved code while recording distinct typed
   `CategoricalObservation` routes.

The result is `PreparedInferenceDataset { rows, features }`, where each feature
retains the resolved source-column index and calculation values in schema order.
`InferenceDataPath` reports feature number, saved source-vector identity, column
bytes, and source row for every failure. `InferencePrepareErrorKind` covers
empty/invalid schemas, missing or ambiguous required columns, missing/invalid
values, and arithmetic overflow. There is no target selection, split, semantic
inference, dictionary fitting, filtering, or host normalization in this path.

## Init image packing

`pack_init_images` is the bridge from graph-level external values to finalized
per-device `InitDataImage` manifests. It indexes source values by logical
`ValueId`, rejects duplicate or unexpected values, validates that every
manifest has one nonempty image per device, rejects duplicate logical/physical
members, requires scalar-aligned offsets, checks member ranges and overlap, and
rejects conflicting replicated dtype/byte contracts. Every required source must
be present with exactly the declared byte count.

For each manifest it allocates the full image zero-initialized, copies each
source at its checked offset, leaves gaps (including preallocated fault flags)
zero, returns the image and logical image value, and sorts complete images by
device. `ImagePackErrorKind` reports duplicate/missing/unexpected sources,
size mismatch, duplicate devices/members, conflicting contracts, invalid
manifests, and arithmetic overflow. No image is returned when validation fails.

## Encoded image header contract

The image-header module is intentionally header-only. `EncodedImageFormat` is
PNG, JPEG, GIF87a, GIF89a, BMP, or WebP. `ImageColorModel` can describe
grayscale, grayscale-alpha, RGB, RGBA, BGR, indexed RGB, YCbCr, CMYK, or YCCK;
color interpretation is optional when the header cannot establish it. Width,
height, optional channel count, optional sample bits, and the encoded-file /
encoded-bytes layout are all exposed through `EncodedImageMetadata`.

The parser never allocates decoded pixels and never claims a pixel value range.
This lets semantic inference recognize images and lets preparation preserve the
exact compressed/container bytes while still rejecting malformed headers.

## GGUF container parser

### Validated archive

`GgufLimits` contains nonzero limits for total file bytes, metadata pairs,
tensors, rank, aggregate string bytes, aggregate metadata-array elements, and
metadata-array depth. `parse_gguf` returns a `GgufArchive` borrowing all text
and tensor bytes from the caller's complete image.

The parser accepts GGUF v2 little-endian and GGUF v3 little- or big-endian.
It requires the `GGUF` magic, checks version/endian encoding, counts, minimum
header bytes, UTF-8 strings, metadata-key length (maximum 65,535 bytes), tensor
name length (maximum 64 bytes), duplicate keys/names, and metadata array depth,
element, and string budgets. Metadata retains all scalar types and raw f32/f64
bits. Boolean bytes must be zero or one.

`general.alignment` defaults to 32 and otherwise must be a `U32` greater than
zero and divisible by eight. Tensor rank is limited both by the caller and by
the current format maximum of four. Every dimension product is checked for
overflow, the first dimension must be divisible by the tensor type's block
width, and the encoded byte count is calculated from the block width and block
size. Offsets must be aligned, tensor spans must fit the data section and not
overlap, all inter-span/header/terminal padding must be zero, and no trailing
unowned data is accepted except a zero aligned tail. Tensor-free images may end
at the unpadded or aligned header end.

`GgufTensor` exposes name, dimensions, tensor type, relative data offset,
absolute file offset, encoded byte count, and end. `GgufArchive::raw_tensor`
returns the exact borrowed encoded span. Parsing does not dequantize or convert
payload values.

The metadata type codes are U8, I8, U16, I16, U32, I32, F32, Bool, String,
Array, U64, I64, and F64. Supported tensor codes cover scalar F32/F16/BF16/F64,
I8/I16/I32/I64, classic Q1/Q2/Q4/Q5/Q8 blocks, K-quant Q2K/Q3K/Q4K/Q5K/Q6K/
Q8K, IQ1/IQ2/IQ3/IQ4 families, TQ1/TQ2, MXFP4, NVFP4, Q1_0, and Q2_0. Each
type has an explicit block element width and encoded block byte width in
`GgufTensorType`; unknown or removed codes fail as unsupported types.

`GgufErrorKind` makes the fail-closed boundary observable: invalid limits,
file/count/rank/string/array bounds, truncation, bad magic/version/endian,
invalid UTF-8/keys/names/booleans/alignment/dimensions/offsets, duplicate
metadata/tensors, unsupported types, overlapping spans, nonzero padding,
trailing data, and arithmetic overflow are all distinct categories.

### Structural OGDL conversion

`gguf_ogdl.rs` adds a typed representation for conversion, not an execution
claim. The canonical root is:

```text
gguf
	 schema recipe-gguf-structural-v1
	 version 3
	 endian little|big
	 alignment N
	 file_bytes N
	 metadata
	 tensors
```

The in-memory `gguf_to_structural_ogdl` path first runs `parse_gguf`, requires
v3, writes metadata scalars and recursively typed arrays, then writes each
tensor descriptor and decodes every encoded block into named typed fields. It
does not emit the source image, base64, hexadecimal payload, or an opaque byte
node. Scalar tensors are emitted in chunks of at most 128 values; block tensors
emit one `block N` with fields in the exact decoder order.

The seekable `gguf_to_structural_ogdl_stream` path performs a complete first
pass over the binary header and spans, then emits metadata and tensor payloads
without retaining the expanded OGDL string or tensor bytes. `inspect_gguf_model_stream`
uses the same bounded validation but additionally requires nonempty string
`general.architecture` and returns only container-level architecture, endian,
alignment, metadata count, and tensor count. It does not claim that the named
architecture has a Recipe lowering.

The reverse functions accept only canonical structural OGDL. They require one
`gguf` root, exactly seven ordered root fields, LF line endings, tab indentation,
no blank lines, exact labels and scalar child rules, valid JSON strings for keys
and tensor names, canonical scalar text, homogeneous arrays, rank at most four,
supported tensor names/types, aligned nonoverlapping offsets, and checked file
length. Float values are represented as sign/exponent/fraction fields so NaN,
infinity, subnormal, and signed-zero bits round-trip without decimal loss.

`structural_ogdl_to_gguf_stream` uses bounded passes over a seekable OGDL input
because GGUF descriptors precede tensor data. The output must initially be
empty. It writes and patches array lengths, reserves zero padding, writes tensor
payloads at declared offsets, refuses changes between passes, flushes, reparses
the produced GGUF through the stream validator, and leaves the output at EOF.
The in-memory `structural_ogdl_to_gguf` performs the same checks in one owned
buffer and reparses through `parse_gguf` before returning.

The block decoder/encoder is an explicit inverse for every supported tensor
type. It names scale/minimum fields, packed quant codes, sign/high bits,
integer arrays, grid indices, ternary codes, and exact float fields. Packing
helpers validate widths, value maxima, required array lengths, and nibble/bit
alignment. A decoder that leaves bytes or an encoder that writes the wrong
block size is an `InvalidStructure` error. `GgufOgdlError` reports the source
category (`Gguf`, `Io`, `InvalidUtf8`, `InvalidSyntax`, `InvalidStructure`,
`InvalidValue`, or `ArithmeticOverflow`) plus a path such as
`gguf.tensors[3].payload.blocks[4].fields[2]`.

The root CLI connects these functions directly: `recipe convert INPUT OUTPUT`
accepts `.gguf -> .ogdl` and `.ogdl -> .gguf`, derives bounded limits from the
declared source/output lengths, writes a new private output, syncs it, and
removes a partial output on conversion failure.

## Safetensors container parser

`SafeTensorLimits` fixes nonzero header bytes, data bytes, tensor count, rank,
and tensor-name bytes. `parse_safetensors` reads the little-endian eight-byte
header length, bounds the JSON header and remaining data section, parses a
strict JSON object, and validates every tensor without decoding its payload.

Supported dtypes are BOOL, U8/I8, U16/I16, U32/I32, U64/I64, F16, BF16, F32,
and F64. For each tensor, shape multiplication and dtype byte multiplication
are checked; rank and name limits are enforced; offsets must be ordered,
nonreversed, inside the data section, and exactly equal the expected encoded
byte count. Every data byte must be covered exactly once, with no overlap or
trailing gap. Metadata is a string-to-string map. Top-level fields, metadata
fields, and tensor object fields must be unique; unknown or duplicate fields,
malformed JSON, unsupported dtypes, bad shapes/offsets, noncontiguous spans,
truncation, configured limits, and arithmetic overflow are distinct
`SafeTensorErrorKind` failures.

Entries are sorted by name for deterministic lookup. `SafeTensorArchive::entry`
uses binary search, and `encoded_tensor` returns the exact borrowed span. Like
GGUF, this module validates an encoded model container but does not perform a
CPU dequantization or calculation conversion.

## Real public training and inference flows

### Public training data

`src/data_prepare.rs` is the root data boundary. `prepare_data` constructs
finite defaults (1 GiB source bytes, 10,000,000 records, 16,384 fields per
record, and 16 MiB field bytes), validates the public `Data` declaration,
requires at least one target and an explicit split, maps exclusions and
conditions to ingest request types, distills all declared sources, infers with
`CategoricalEncodingModel`, and applies the saved fit-only preparation path.
`distill_data` is the same bounded source boundary without targets, split, or
training policy. `select_target_free_data` maps only exclusions and predicates
through `select_table`.

Condition float literals are narrowed to finite f32 before becoming an ingest
`PredicateLiteral`; a nonfinite or non-underflowing value that cannot survive
that boundary is rejected as `FloatPredicateOutsideF32`. The public boundary
translates ingest, source, semantic, and prepare errors without exposing a
partial dataset.

`training/src/model.rs`, `compile.rs`, and model-family preparation consume the
`PreparedDataset` vectors, schemas, partitions, dictionaries, temporal
metadata, and categorical observation routes. They are responsible for model
lowering and GPU calculations; ingest stops at exact bytes and typed metadata.

### Semantic-model inference

`src/inference.rs::compile_inference_package` validates policy, data, and model;
requires target-free data with no `.split(...)` or redeclared normalization;
requires a `.ogdl` or `.gguf` model source; and loads the model through a
bounded snapshot. `.ogdl` dispatches by semantic-model root to dense, KNN, or
Bayesian checkpoint decoders. `.gguf` constructs file-sized GGUF limits and
loads the supported GGUF Llama artifact.

The data declaration is then distilled once, selected once with
`select_target_free_data`, and bound to the loaded model:

* dense checkpoints use the saved feature spans and
  `InferenceFeatureSchema`, then `prepare_inference_table`;
* KNN models use saved reference feature schemas and the same ingest inference
  boundary;
* Bayesian models union parent schemas in first-occurrence order and reuse
  every saved categorical dictionary; and
* GGUF Llama uses the specialized one-vector token-stream binder described
  below.

After this boundary, training code compiles numeric conversion, one-hot
expansion, saved normalization, and model operations as Recipe calculations.
Ingest never normalizes a target-free row on the host.

### GGUF Llama inference

`training/src/gguf_llama.rs` calls `parse_gguf` and then narrows the validated
container to the first concrete execution instrument. It accepts only GGUF v3,
little endian, `general.architecture = "llama"`, dense F32 tensors, equal query
and KV head counts, full even-head RoPE, causal attention, ordinary dense
parallel-free residual blocks, and finite positive metadata. It requires the
exact tensor names and shapes for token embeddings, output normalization,
attention/FFN weights, and optional biases/scales. Grouped-query attention,
MoE, noncausal or parallel residual variants, unsupported RoPE scaling/YaRN,
quantized tensors, missing tensors, wrong shapes, bad metadata, and invalid
optional rope factors fail as `GgufLlamaError` categories rather than being
interpreted as another architecture.

`prepare_gguf_llama_inference_table` expects one table vector. It treats each
row's first field as whitespace-separated exact int32 token IDs, rejects missing
or invalid UTF-8/decimal tokens, rejects negative or vocabulary-out-of-range
IDs, requires at least one token, and caps the stream at model context length.
The compiled inference graph receives the token bytes and validated tensor
images as external inputs. It produces all-position token logits; prediction
formatting is outside ingest.

### Checkpoint and tokenizer snapshots

`training/src/inference.rs` loads dense, KNN, and Bayesian semantic model files
through `read_source_snapshot` with each decoder's source limit, then invokes a
strict versioned decoder. `recipe-text` uses the same snapshot boundary for
tokenizer JSON files, validating the model after the bounded read. These paths
share the ownership rule: all filesystem access is preparation-time and no
runtime loop can reopen the source.

## Failure and bound matrix

| Boundary | Output on success | Main fail-closed categories |
| --- | --- | --- |
| `read_source_snapshot` | Closed, hashed `SourceSnapshot`. | Invalid bound, I/O, nonregular path, growth/size limit, arithmetic overflow. |
| `parse_table`/`read_table` | Rectangular `RawTable` of bytes. | Invalid source/record/field bounds, malformed CSV/whitespace, inconsistent width, I/O, overflow. |
| `distill_datasets` | Ordered `DistilledDataset` with source context and semantic rules. | Invalid path, symlink, I/O, empty source, ZIP escape/depth/expansion, malformed format/archive, aggregate limits, ingest, overflow. |
| Semantic inference | One `InferredVector` per column. | Inconsistent width or aggregate evidence arithmetic overflow. |
| Preparation | Fitted schemas, lossless values, partitions, optional dense matrix. | Selection/predicate errors, no rows/features, fit inconsistency, encoding/temporal range, variable/mixed/missing dense values, overflow. |
| `prepare_inference_table` | Schema-ordered typed feature rows. | Empty/invalid schema, missing or duplicate required column, missing numeric, invalid value, overflow. |
| Image headers | Encoded metadata, no decoded pixels. | Truncation, bad signature/CRC/marker/chunk, invalid dimensions or reserved fields. |
| `pack_init_images` | Complete sorted per-device init images. | Duplicate/missing/unexpected source, size/contract mismatch, duplicate/overlapping members/devices, invalid manifest, overflow. |
| `parse_gguf` | Borrowed validated metadata and encoded tensor spans. | Invalid limits, magic/version/endian, count/rank/string/array, UTF-8/key/name/bool/alignment/dimension/offset, duplicate, unsupported type, overlap/padding/trailing, truncation/overflow. |
| `parse_safetensors` | Borrowed validated metadata and contiguous encoded spans. | Invalid limits, truncation/header/data/tensor/rank/name bounds, duplicate/malformed fields, unsupported dtype, shape/offset/noncontiguous data, overflow. |
| GGUF structural conversion | Canonical typed OGDL or a reparsed GGUF image. | GGUF/OGDL syntax, structure, value, UTF-8, I/O, bounds, block field/packing, alignment/span, and arithmetic errors. |

The consistent behavior is important: parsing or preparation errors are not
statuses that can enter the runtime graph, and no fallback parser, alternate
representation, retry, or CPU payload calculation is substituted.

## Boundary summary

The complete normal data path is:

```text
public Data paths
    -> bounded source snapshots
    -> deterministic file/directory/ZIP traversal
    -> format-specific logical tables
    -> rectangular RawTable plus semantic rules
    -> fit-only semantic inference on the train partition
    -> immutable VectorSchema and lossless PreparedValues
    -> explicit train/validation partitions or saved-schema inference values
    -> downstream Recipe graph compilation
    -> one packed init image per device
    -> native init -> loop -> exit execution
```

The model path is parallel but remains preparation-only until graph compilation:

```text
bounded model snapshot
    -> GGUF or safetensors structural validation
    -> borrowed encoded tensor spans
    -> exact architecture/checkpoint decoder
    -> saved feature schema or specialized token contract
    -> typed external input images
    -> downstream GPU calculations
```

`recipe-ingest` is therefore a representation and admission layer. A parser
success proves that bytes and structure satisfy a declared contract. It does
not, by itself, prove that a model architecture has a lowering, that a tensor
payload has been dequantized, or that a native execution completed.
