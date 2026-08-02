# `ingest/src/dataset.rs`

## Purpose and boundary

`dataset` is Recipe's bounded filesystem-to-table distiller. It admits one or
more regular files, recursive directories, or recursively nested ZIP
containers, recognizes the source representation, and merges every logical
table into one owned rectangular [`RawTable`]. It also carries parser-owned
semantic rules beside that table so later semantic inference and preparation
can preserve declarations that are exact in the source format.

The module is a preparation boundary, not a runtime or model boundary. It does
not train, infer, normalize, impute, lower payload calculations, probe
hardware, or retain a file handle. Source bytes are read through
`read_source_snapshot`, copied into owned buffers, and closed before the
returned `DistilledDataset` can be consumed. GGUF, safetensors, image, and
binary payloads remain encoded bytes. A downstream semantic or model-specific
operation decides whether and how those bytes can become calculation inputs.

The implementation is fail closed. A source is either completely distilled
into a `DistilledDataset` or one typed `DatasetSourceError` is returned. The
declared source order, deterministic directory and archive order, global
limits, source context, and parser-selected semantic rules are all observable
parts of the result.

## Parseable module manifest

~~~toml
[module]
crate = "recipe-ingest"
path = "ingest/src/dataset.rs"
role = "bounded recursive source distillation"
public_entrypoints = ["distill_dataset", "distill_datasets"]
public_result = "DistilledDataset"
runtime_filesystem_access = false
payload_calculation = false
source_order = "declared order, then sorted directory or archive member path"
partial_result_on_error = false

[limits]
archive_depth_limit = 32
source_metadata_vectors = 14
aggregate_source_bytes = "IngestLimits.source_bytes"
aggregate_samples = "IngestLimits.records"
aggregate_vectors = "IngestLimits.fields_per_record"
textual_value_bytes = "IngestLimits.field_bytes"

[source_context]
single_regular_file = false
multiple_declared_sources = true
directory_member = true
archive_member = true
metadata_is_ordinary_data = true

[format_dispatch]
delimited = ["csv", "tsv", "all-data", "dat", "data", "data-numeric", "tra", "trn"]
json = ["json"]
model_metadata = ["gguf", "safetensors"]
spreadsheet = ["xlsx"]
presentation = ["pptx"]
text = ["inp", "out", "patch", "sh", "txt"]
binary = ["bin", "logits", "model"]
magic_fallbacks = ["zip", "png", "recognized image signatures", "probable UTF-8 text"]
~~~

The constants and public surface are defined at `dataset.rs:25-158`; the
entrypoint and traversal are at `dataset.rs:500-824`; parser dispatch and
format readers occupy `dataset.rs:825-1594`; source context and helper bounds
are at `dataset.rs:1596-1888`.

## Public types and ownership

### `SourceFormat`

`SourceFormat` (`dataset.rs:31-41`) records the structural parser that produced
one logical table. Its complete set is:

| Variant | `as_str()` | Produced by |
| --- | --- | --- |
| `Delimited` | `delimited` | CSV, TSV, and detected tabular text |
| `Json` | `json` | JSON arrays, objects, and scalars |
| `Text` | `text` | Recognized UTF-8 text without a table structure |
| `Image` | `image` | PNG or another recognized image signature |
| `Gguf` | `gguf` | GGUF metadata and tensor tables or opaque fallback |
| `SafeTensors` | `safetensors` | safetensors metadata and tensor tables or opaque fallback |
| `Spreadsheet` | `spreadsheet` | XLSX worksheets |
| `Presentation` | `presentation` | PPTX slide paragraphs |
| `Binary` | `binary` | Explicit binary extensions or opaque bytes |

`as_str` is the only string representation used in source metadata. It is
stable lowercase ASCII and is not inferred from an arbitrary file extension.

### `DatasetSourceError` and result alias

`DatasetSourceErrorKind` (`dataset.rs:63-75`) is a non-exhaustive stable
category set:

| Kind | Meaning at this boundary |
| --- | --- |
| `InvalidPath` | Non-UTF-8 archive member, path containment failure, or a root that is neither a regular file nor directory |
| `Io` | Source metadata, directory, entry, or bounded snapshot I/O failure |
| `Symlink` | A root or recursive directory entry is symbolic, so it is not followed |
| `EmptySource` | No source, no ZIP members, or no admitted file/sample was produced |
| `LimitExceeded` | A configured byte, sample, vector, member, expansion, or archive-depth bound was reached |
| `MalformedArchive` | ZIP structure, member path, member open, member read, or container expansion metadata is invalid |
| `MalformedFormat` | A recognized format cannot be decoded, has invalid UTF-8/XML/JSON, or violates logical table shape |
| `Ingest` | `parse_table` or final `RawTable::from_parts` rejected an otherwise recognized table |
| `ArithmeticOverflow` | A checked count, byte total, path depth, or bound could not be represented |

`DatasetSourceError` (`dataset.rs:77-108`) owns a public `kind`, an optional
`PathBuf`, and a human-readable `detail`. `for_path` attaches a filesystem or
logical archive path. `Display` writes `Kind: detail` and appends
`[path]` when present. `DatasetSourceResult<T>` is the module's
`Result<T, DatasetSourceError>` alias.

### `DistilledDataset`

`DistilledDataset` (`dataset.rs:119-159`) owns exactly three private values:

| Field | Ownership and meaning |
| --- | --- |
| `table: RawTable` | One rectangular table. Rows are source samples and columns are source vectors. All field bytes are owned. |
| `semantics: Vec<VectorSemanticRule>` | One parser rule per global column. `Infer` invokes semantic parsers/classifier, `Classify` invokes only the ambiguous model, and `Exact` preserves a source-declared type and encoding. |
| `file_count: usize` | Number of admitted regular leaf files. A directory file and each expanded ZIP member count; a ZIP container itself does not. One XLSX or PPTX file counts once even when it emits several logical tables. |

The methods are intentionally narrow:

| Method | Result and effect |
| --- | --- |
| `table()` | Borrow the owned `RawTable`; no copy or source read. |
| `sample_count()` | `table.rows().len()`. Empty logical tables are normalized to one all-missing row during append, so an admitted leaf normally contributes at least one sample. |
| `vector_count()` | `table.width()`. Global headers retain first-seen order. |
| `file_count()` | Borrow the admitted leaf count. |
| `infer_vectors(model)` | Run `infer_table_vectors_with_semantics` over the complete table while honoring the parser rules. |
| `prepare(request, model)` | Call `prepare_table_with_semantics`; selection occurs before an exact rational split and semantic/encoding fit is performed from the retained train partition. |
| `into_table()` | Move the `RawTable` out, consuming the dataset. The semantic rules and file count are dropped. |

`DistilledDataset` derives `Clone`, `Debug`, `PartialEq`, and `Eq`. Cloning
duplicates the owned bytes and metadata; it does not reopen any source.

### Internal records

The following records are private to this module and delimit the ownership
transitions:

| Record | Fields | Invariant or purpose |
| --- | --- | --- |
| `LogicalTable` (`160-224`) | `member`, `format`, `columns`, `rows` | One parser result. `new` checks every row width against the column count before it can be appended. |
| `LogicalColumn` (`168-202`) | `name: Vec<u8>`, `semantic: VectorSemanticRule` | A source column name and parser semantic rule. Constructors select `Infer`, `Classify`, or `Exact`. |
| `LogicalField` (`174`, `1687-1712`) | A `LogicalColumn` plus one value | Temporary source-context field used while appending one row. |
| `FileContext` (`235-265`) | logical path, depth, declaration index, context flag | Carries stable path identity through directory and nested archive traversal. `child` validates UTF-8 and checked depth, and always turns context on for a member. |
| `Accumulator` (`268-497`) | limits, global headers/index, rows, semantic rules, file count, leaf bytes | Owns the only mutable merge state. All source declarations and expanded leaves share one set of bounds. |
| `SourceMetadata` (`1596-1685`) | Fourteen encoded metadata values | Creates ordinary columns for source identity, format, digest, sizes, sample positions, and depth. |

`RawTable` itself is defined in `ingest/src/table.rs:88-129`. It stores a
delimiter marker, headers, and nested byte vectors. `RawTable::from_parts`
rejects any row whose width differs from the header width. Dataset distillation
uses `from_parts` only after the accumulator has already checked logical widths,
global header insertion, and every field bound.

## Entrypoints and complete call graph

`distill_dataset(path, limits)` (`dataset.rs:500-501`) is the one-path
convenience entrypoint. It delegates directly to `distill_datasets([path],
limits)`, so one-path and multi-path calls share all traversal and error rules.

`distill_datasets(paths, limits)` (`dataset.rs:517-540`) performs this fixed
sequence:

~~~text
collect each AsRef<Path> into owned PathBufs, preserving declaration order
  -> reject an empty collection with EmptySource
  -> include_source_context = paths.len() > 1
  -> construct Accumulator(limits)
  -> for each (source_index, path): distill_one_source(...)
  -> Accumulator::finish()
~~~

The function never reorders the declared roots. A source failure immediately
stops later roots and no partial accumulator escapes. Aggregate limits therefore
cover all preceding and current roots, not one root at a time.

### Root admission

`distill_one_source` (`dataset.rs:542-585`) calls `fs::symlink_metadata` so a
symbolic root is identified without following it. A root is handled as follows:

| Root metadata | Operation |
| --- | --- |
| symbolic link | Return `Symlink` with the root path |
| directory | `visit_directory(root, root, source_index, accumulator)` |
| regular file | Read once through `read_regular_file`, create a depth-zero `FileContext`, then `visit_bytes` |
| any other type | Return `InvalidPath` |

The direct regular-file context receives the collection-wide
`include_source_context` flag. Directory files always receive context, even
when the directory is the only declared root, because a directory collection
needs path identity for its members. An archive root receives the collection
flag, but every `FileContext::child` call turns context on for the member.

### Deterministic directory traversal

`visit_directory` (`dataset.rs:587-656`) reads all entries, converts the
directory iterator to a vector, and sorts by `DirEntry::file_name` before any
recursive call. It rejects symbolic entries. Directories recurse in sorted
order. Regular files use a logical path made from the root's file-name label
and the path relative to that root. The context depth is the relative path's
component count. Non-file, non-directory entries are ignored after the root
check, because no `else` branch admits them.

Each regular file is bounded by `read_regular_file`, then visited with the
same accumulator. A path that cannot be stripped relative to its root is an
`InvalidPath` failure rather than an invented logical path.

### Bounded immutable file reads

`read_regular_file` (`dataset.rs:658-663`) creates a `SourceLimit` from
`limits.source_bytes()` and calls `read_source_snapshot`. The source module
checks metadata, opens the file, caps the actual read at `limit + 1`, and
returns owned bytes only after the handle is closed. `dataset_error_from_source`
(`665-678`) maps source-layer categories as follows:

| `SourceErrorKind` | `DatasetSourceErrorKind` |
| --- | --- |
| `InvalidLimit`, `LimitExceeded` | `LimitExceeded` |
| `Io` | `Io` |
| `NotRegularFile` | `InvalidPath` |
| `ArithmeticOverflow` | `ArithmeticOverflow` |

The snapshot's digest is not reused. `SourceMetadata::new` computes a lowercase
hex SHA-256 over the bytes that the leaf parser actually receives. For an
archive member those are decompressed member bytes, not the outer archive.

## Byte admission, archive recursion, and merging

### `visit_bytes` dispatch boundary

`visit_bytes` (`dataset.rs:679-710`) receives one owned byte slice and a
logical context. It checks the lowercased extension first:

1. `.xlsx` is admitted as one leaf, parsed into one table per nonempty sheet,
   and each table is appended.
2. `.pptx` is admitted as one leaf, parsed into one slide-paragraph table,
   and appended.
3. `.zip`, or any bytes with a ZIP signature, recurse through `visit_archive`
   without admitting the container itself.
4. Every other input is admitted as one leaf, passed through `parse_leaf`, and
   each resulting logical table is appended.

`Accumulator::admit_leaf` (`dataset.rs:291-324`) converts the byte length to
`u64`, adds it with checked arithmetic to the aggregate `leaf_bytes`, compares
the total to `limits.source_bytes()`, and increments `file_count`. Thus
compression and source splitting cannot bypass the aggregate byte bound. Outer
ZIP bytes are not counted as leaves; each decoded regular member is.

### ZIP traversal

`visit_archive` (`dataset.rs:712-823`) is recursive and deterministic:

1. Reject `archive_depth >= ARCHIVE_DEPTH_LIMIT` (`32`) with `LimitExceeded`.
2. Open a `zip::ZipArchive` over the in-memory bytes. Open and member-index
   failures are `MalformedArchive`.
3. Inspect every raw member. Directory entries are skipped. `enclosed_name`
   must produce an archive-root-contained path, otherwise the member is a
   `MalformedArchive` path-escape error.
4. Sort remaining members by enclosed path, independent of ZIP directory
   order. An archive with no file members is `EmptySource`.
5. Reject a declared member size above `source_bytes` before opening it.
6. Reopen the member by original index and read it through `take(limit + 1)`.
   A declared or decoded expansion above the limit is `LimitExceeded`; member
   open/read failures are `MalformedArchive`.
7. Call `context.child(member)` and recurse through `visit_bytes` with
   `archive_depth + 1`.

`FileContext::child` joins logical paths with `/`, checks member UTF-8, adds
the member path component count to `depth` using `checked_add`, retains the
root `source_index`, and sets `include_source_context = true`. A nested ZIP
therefore gets source metadata for all of its eventual leaves and reports its
logical nested path and depth.

### Global merge algorithm

`Accumulator::append` (`dataset.rs:326-436`) merges one `LogicalTable`:

1. If the parser emitted no rows, insert one row containing one empty value per
   logical column. This preserves an admitted file as a sample with missing
   fields rather than silently dropping it.
2. Check the prospective global row count against `limits.records()` with
   checked `usize` and `u64` conversions.
3. If the context flag is true, construct `SourceMetadata` and resolve its 14
   fields before source columns. Otherwise no hidden metadata is added.
4. For each source column, prefix a name that collides with a context name with
   `data:`. An empty name becomes `col{index}` (one-based). Duplicate names in
   this logical table receive `#2`, `#3`, and so on, with checked ordinal
   arithmetic.
5. Resolve every resulting header in the global `header_indices` map. A new
   header extends all existing rows with an empty value and records its semantic
   rule. A reused header with a different rule is downgraded to `Infer`, because
   one global column cannot preserve incompatible exact declarations.
6. For every source row, allocate a global-width empty row, prepend the matching
   source metadata values, append the logical values, check each value with
   `check_field_bound`, and place values at their resolved global indexes.
7. Push the completed row. Source row order within a logical table and logical
   table order within a file are retained.

The merge is sparse by construction: a column absent from one source remains an
empty byte value for that source's rows. Empty bytes are the missing-value
representation used by the semantic and preparation layers.

`Accumulator::finish` (`dataset.rs:471-497`) rejects zero admitted files or
zero rows with `EmptySource`, constructs `RawTable::from_parts`, maps its
`IngestError` to `DatasetSourceErrorKind::Ingest`, and returns the three owned
`DistilledDataset` fields. No source path or file handle remains in the result.

## Format selection

`parse_leaf` (`dataset.rs:825-904`) first obtains a lowercase extension with
`extension` (`1761-1767`). The exact dispatch order is:

| Condition | Parser and logical semantics |
| --- | --- |
| `csv` or `tsv` | `parse_delimited`, header present. CSV uses auto delimiter; TSV forces tab. |
| `all-data`, `dat`, `data`, `data-numeric`, `tra`, `trn` | `parse_delimited`, header absent. |
| `json` | `parse_json`, possibly several logical tables. |
| `gguf` | `parse_gguf_tables`, metadata/tensor tables or opaque fallback. |
| `safetensors` | `parse_safetensor_tables`, metadata/tensor tables or opaque fallback. |
| `png` and valid PNG signature | One exact `Image` table with an `image`/`Bytes` value. |
| `inp`, `out`, `patch`, `sh` | One exact UTF-8 `Text` table. Invalid UTF-8 is `MalformedFormat`. |
| `txt` and `looks_tabular` | `parse_delimited`, header absent. |
| `txt` otherwise | One exact UTF-8 `Text` table. |
| `bin`, `logits`, `model` | One exact `Binary` table with a `binary`/`Bytes` value. |
| Any recognized image signature | One exact `Image`/`Bytes` table. |
| Probably text and structurally tabular | `parse_delimited`, header absent. |
| Probably UTF-8 text | One exact `Text`/`Utf8` table. |
| Anything else | One exact `Binary`/`Bytes` table. |

An extension does not override a recognized container branch in `visit_bytes`:
`.xlsx` and `.pptx` are handled before generic ZIP detection, while an unknown
extension with ZIP magic is still recursively opened. Conversely, a file named
`.zip` with non-ZIP bytes fails as `MalformedArchive` rather than falling back
to binary.

### Delimited and text inputs

`parse_delimited` (`dataset.rs:905-930`) selects tab for `tsv`, auto for
`csv`, and for any other extension uses `structural_delimiter` when available,
otherwise `Delimiter::Auto`. It calls `parse_table` with the selected
`HeaderMode` and all `IngestLimits`, maps every `IngestError` to
`DatasetSourceErrorKind::Ingest`, and turns headers into `LogicalColumn::infer`.

The lower `table` parser frames CSV-style records strictly and rectangularly.
`Delimiter::Auto` sniffs quoted comma, semicolon, tab, or ASCII whitespace
fields. A present header consumes the first record, but the parser's record
bound is checked before that removal. Header-absent tables use `col1`, `col2`,
and so on. Empty lines are ignored only by the ASCII-whitespace framing path.

The dataset-level structural detector (`dataset.rs:1827-1888`) examines at
most the first eight nonblank lines. It prefers tab, then semicolon, then comma
when every line has the same width greater than one. It accepts ASCII
whitespace only when widths agree and every token parses as `f64`. Quoted
delimiters are ignored by `delimiter_width`. `is_probably_text` accepts only
UTF-8 bytes whose control characters are tab, line feed, or carriage return.

`parse_text` (`932-950`) validates UTF-8 and emits one exact `text` column with
`SemanticType::Text` and `VectorEncoding::Utf8`. `single_payload` (`952-963`)
emits one row, one exact column, and caller-selected semantic type with
`VectorEncoding::Bytes`.

### JSON

`parse_json` (`dataset.rs:965-978`) decodes one `serde_json::Value` and maps
the top-level shape:

| JSON shape | Result |
| --- | --- |
| Array | One `json_values_to_table` table. |
| Object | `json_object_to_tables`, which may emit one table or several named members. |
| Scalar | One value table containing that scalar. |

`json_object_to_tables` (`980-1033`) uses these shape rules:

* An empty object becomes one value table containing the empty object.
* If every map value is an array of objects, each map key becomes a separate
  member table.
* If every map value is a nonempty object whose values are arrays, the nested
  member keys are unioned. Each member table concatenates the outer map key as
  a classified `key` column and the nested array values as the remaining table.
* If every map value is an object but the split shape does not apply, one table
  contains the map keys as classified `key` and the map values as data.
* Any other object shape becomes one value table containing the object JSON.

`json_values_to_table` (`1060-1132`) unions object keys in first-seen map
iteration order and fills a missing key with empty bytes. Arrays become a
rectangular width equal to the largest row, with `col1`, `col2`, and so on.
Mixed or scalar values become one inferred `value` column. Nested arrays and
objects are retained as compact JSON text, so flattening never invents a new
feature. `json_keyed_values_to_table` (`1035-1058`) verifies key and value row
counts before inserting its classified `key` column.

`json_cell` (`1134-1147`) encodes `null` as missing empty bytes, booleans and
numbers using their JSON text, strings as UTF-8 bytes, and nested arrays or
objects using `Value::to_string()`.

### XLSX spreadsheets

`parse_spreadsheet` (`dataset.rs:1149-1223`) first calls
`preflight_zip_expansion` for an aggregate uncompressed-byte bound. It opens
the workbook with calamine, rejects a workbook with no sheet names, and walks
each sheet in workbook order. Empty sheets are skipped. For a nonempty sheet:

* If every cell in the first row is an integer, float, bool, date, or duration,
  the first row is data and headers are generated as `col1`, `col2`, and so on.
* Otherwise the first row is consumed as headers. Empty header cells become
  generated `colN` names.
* Later rows are converted to bytes, padded with missing values to the header
  width, or truncated to it.

Every emitted table has `SourceFormat::Spreadsheet` and inferred columns named
for the sheet's headers. A workbook with no nonempty worksheets is
`EmptySource`. Workbook open and worksheet errors are `MalformedFormat`.
`excel_cell_is_data` (`1225-1235`) defines the first-row test. `excel_cell`
(`1237-1248`) retains strings, numeric text, booleans, errors, and ISO/date
values without host-side numeric conversion.

### PPTX presentations

`parse_presentation` (`dataset.rs:1250-1308`) applies the same ZIP expansion
preflight, opens the container, selects entries named
`ppt/slides/slideN.xml`, and sorts them by numeric `N`. A container with no
slide XML is `MalformedFormat`. Each slide is decoded as UTF-8 XML and passed to
`pptx_paragraphs` (`1317-1380`). Paragraph text is collected from `p:t`
elements, `br` elements append one space, character references are resolved,
and only nonempty trimmed paragraphs become rows. XML decode failures are
`MalformedFormat`.

The resulting table has exact columns:

| Header | Semantic type | Encoding | Value |
| --- | --- | --- | --- |
| `slide` | `Numeric` | `I32` | Numeric `N` parsed from the member name `ppt/slides/slideN.xml`. |
| `text` | `Text` | `Utf8` | One trimmed paragraph. |

The parser does not reject a presentation whose slides contain no nonempty
paragraphs. `LogicalTable` then has no rows and `Accumulator::append` inserts
one all-missing row.

### GGUF

`parse_gguf_tables` (`dataset.rs:1382-1483`) constructs `GgufLimits` from the
received file length, using the file length for byte, metadata-pair, tensor,
string, and array-element bounds, rank `64`, and array depth `64`. A zero-length
input is represented by one so parser limit construction remains nonzero.
Parser failures and limit-construction failures are `MalformedFormat`.

When metadata exists, one `metadata` table is emitted with:

| Header | Rule |
| --- | --- |
| `metadata_key` | `Classify` |
| `metadata_type` | `Classify`, GGUF type code as decimal text |
| `metadata_value` | `Infer`, lossless textual representation |

When tensors exist, one `tensors` table is emitted with classified
`tensor_name`, `tensor_type`, and `tensor_shape`, exact numeric `tensor_rank`
and `tensor_bytes`, and exact binary `binary`. The binary field is the raw
encoded tensor bytes from the parser. If neither metadata nor tensors exist,
the complete GGUF bytes are one exact binary payload table.

`gguf_metadata_value` (`1485-1501`) preserves integer and boolean text,
strings as bytes, arrays through their debug value list, and floating values
as bit-preserving `f32:0x...` or `f64:0x...` strings. No tensor is decoded into
host f32 or int32 payloads here.

### Safetensors

`parse_safetensor_tables` (`dataset.rs:1503-1594`) builds parser limits from
the input length: header bytes, data bytes, and name bytes use the byte length,
tensor count is the byte length narrowed to `u32` (with a nonzero minimum),
and rank is `64`. Parser and limit-construction failures are
`MalformedFormat`.

Metadata, when present, becomes a `metadata` table with classified
`metadata_key` and inferred `metadata_value`. Tensor entries, when present,
become a `tensors` table with classified `tensor_name`, `tensor_type`, and
`tensor_shape`, exact numeric `tensor_rank` and `tensor_bytes`, and exact
binary `binary` containing the encoded tensor span. An archive with neither
metadata nor entries falls back to one exact binary payload containing the
original file bytes. The format layer validates offsets and dtype names but
does not perform payload calculations.

## Source context as ordinary vectors

`SourceMetadata` (`dataset.rs:1596-1685`) is materialized whenever a context
flag is true. Its SHA-256 is computed over the exact bytes in the current leaf,
formatted as 64 lowercase hexadecimal bytes. The fields are emitted in this
fixed order:

~~~text
source_index, source_path, parent_path, folder, file_name, file_stem,
extension, format, member, content_sha256, file_bytes, sample_index,
sample_count, source_depth
~~~

The semantic rules are also fixed:

| Field(s) | Rule and encoding |
| --- | --- |
| `source_index` | Exact `Categorical` with `DictionaryI32`; zero-based declared-source index. |
| `source_path`, `parent_path`, `folder`, `file_name`, `file_stem`, `extension`, `format`, `member`, `content_sha256` | `Classify`; the model chooses categorical or text from evidence. |
| `file_bytes`, `sample_index`, `sample_count`, `source_depth` | Exact `Numeric` with `I32`. |

`sample_index` is zero-based within one logical table. `sample_count` is that
table's row count after the empty-table fallback. `source_depth` is the checked
path-component depth carried by `FileContext`. `member` is the logical table
member name, such as JSON map key, XLSX sheet name, `metadata`, or `tensors`.
For a direct regular file with no context, none of these vectors are added.

If a source data header collides with one of these names, the data header is
prefixed with `data:` before global resolution. `is_source_context_header`
(`1714-1728`) is the authoritative collision set. Duplicate data headers in a
single logical table then use the `#N` suffix rule. Context remains visible to
selection and can be used as a feature unless a caller explicitly excludes it.

## Semantic and preparation bridge

### Parser-owned semantic rules

Each parser constructs `LogicalColumn` with one of three private
`VectorSemanticRule` values from `semantic.rs`:

| Rule | Meaning |
| --- | --- |
| `Infer` | Run the parser-first semantic recognizers over the complete value list, then ask the ambiguous model only if no exact recognizer applies. |
| `Classify` | Skip exact recognizers and ask the ambiguous model directly. This is used for labels, names, and metadata where a parser must not claim numeric or temporal meaning. |
| `Exact(VectorSemantic)` | Preserve the parser's declared semantic type and encoding, such as PPTX `slide`/`text`, image bytes, binary bytes, or source-index categorical. |

When two logical tables reuse one global header with different rules,
`Accumulator::resolve_header` (`dataset.rs:437-469`) sets that global rule to
`Infer`. This is the only merge-time semantic fallback, and it is grounded in
an observed conflict rather than a speculative alternate parser.

`DistilledDataset::infer_vectors` passes these rules to
`infer_table_vectors_with_semantics`. The semantic layer checks rectangular
rows, collects exact byte evidence, and returns one `InferredVector` per
global column. Exact parser rules are not reclassified. A plain `RawTable`
caller can use `infer_table_vectors`, which supplies an all-`Infer` rule list.

### Preparation routes

`DistilledDataset::prepare` (`dataset.rs:140-154`) is the source-aware
preparation route. `prepare_table_with_semantics` in `prepare.rs` selects
targets, exclusions, and rows before computing the exact train fraction, fits
semantic metadata and encodings from the retained train rows, and applies the
fitted schemas to every retained row. It returns `PreparedDataset`, where
source row indexes, vector order, target declaration order, and contiguous
train/validation partitions remain explicit.

The public root training adapter currently takes a different, explicit route:

~~~text
src/data_prepare.rs::prepare_data_with_limits
  -> distill_data_with_limits
  -> DistilledDataset::infer_vectors(CategoricalEncodingModel)
  -> prepare_inferred_table(distilled.table(), inferred, request)
~~~

That route is visible at `src/data_prepare.rs:140-172`. The supplied
`InferredVectorList` is inferred before target/row selection, so the public
adapter's semantic discovery can observe validation rows or rows later removed
by a predicate. `DistilledDataset::prepare` exists specifically to fit rules
from the exact retained train partition; documentation must not attribute that
train-only discovery property to `prepare_data_with_limits` while this call
graph remains unchanged.

### Target-free selection

`src/data_prepare.rs::select_target_free_data` (`src/data_prepare.rs:117-129`)
translates public column exclusions and conditions, then calls
`recipe_ingest::select_table` on `distilled.table()`. Selection resolves names
and predicates against the original headers and rows, evaluates predicates
before excluded columns are removed, preserves retained row and column order,
and performs no semantic inference, metadata fit, target resolution, or split.
The saved model schema is authoritative in the next layer.

`src/inference.rs::compile_inference_package`
(`src/inference.rs:500-543`) uses the target-free sequence:

~~~text
validate policy, Data, and model
  -> reject targets, split, or data normalization for target-free inference
  -> load .ogdl or .gguf model artifact
  -> distill_data(data)
  -> select_target_free_data(data, distilled)
  -> bind the selected RawTable to the saved model schema
  -> compile the model-specific inference graph
~~~

The model-specific training crate receives the selected `RawTable`, not a
filesystem path. Its `prepare_checkpoint_inference_table`,
`prepare_knn_inference_table`, `prepare_bayes_inference_table`, and
`prepare_gguf_llama_inference_table` functions resolve required names and
encodings without refitting query semantics.

## Repository consumers

The module is private inside `recipe-ingest` but its public types and functions
are re-exported by `ingest/src/lib.rs:27-30`. Current call sites are:

| Consumer | Source location | Dataset operation | Boundary after it |
| --- | --- | --- | --- |
| Root data adapter | `src/data_prepare.rs:102-109` | `distill_data` and `distill_data_with_limits` call `distill_datasets` with `Data::sources()` in order. | `DataPreparationError::Source` on failure. |
| Root training | `src/data_prepare.rs:140-172` | Distill, infer with `CategoricalEncodingModel`, then `prepare_inferred_table`. | `PreparedDataset` only; no source handle. |
| Target-free root inference | `src/inference.rs:522-540` | Distill, `select_target_free_data`, then model-specific schema application. | `InferenceError::Data` wraps source or selection failures. |
| Dense training | `src/training.rs:592-604` | `prepare_data(data)` before graph compilation. | Features and targets are later projected to dense matrices. |
| KNN model preparation | `src/training.rs:467-497` | `prepare_data(data)` before `prepare_knn_reference_set`. | The prepared train partition is the immutable reference set. |
| Bayesian preparation | `src/training.rs:549-578` | `prepare_data(data)` before categorical reference-set construction. | Known categorical codes and source order feed observed conditionals. |
| Saved-model inference | `training/src/inference.rs:823-890` | Load an artifact and pass `dataset.table()` to model-specific schema preparation. | Saved feature names and dictionaries are applied to the new rows. |

No consumer calls a private parser or enters the accumulator directly. This
keeps filesystem admission, format selection, and aggregate bounds at one
production boundary.

## Limits and invariants

`IngestLimits` is constructed by `ingest/src/table.rs:32-55` and rejects zero
values before dataset traversal. Its four nonzero bounds are used at distinct
levels:

| Bound | Enforcement in this module |
| --- | --- |
| `source_bytes` | Each regular snapshot, each declared ZIP member, each decoded member, XLSX/PPTX uncompressed preflight, aggregate admitted leaf bytes, and every distilled value. |
| `records` | `Accumulator::append` prospective global logical rows. The lower table parser also checks framed records, including a present header. |
| `fields_per_record` | The lower table parser checks each framed record; `Accumulator::resolve_header` checks the global merged vector count. |
| `field_bytes` | `parse_table` checks every framed field. Dataset append applies the smaller textual field bound only to UTF-8 values; opaque bytes may use the larger source bound. |

Other hard invariants are:

* Roots and recursive directory entries do not follow symbolic links.
* ZIP members must be enclosed within their archive root and are visited in
  sorted enclosed-path order. Nesting at depth 32 is rejected.
* Directory members and archive members receive source context. A single direct
  regular file does not. Multiple declared roots receive context even when each
  root is a direct file.
* Every admitted leaf contributes to one aggregate byte and file count. A ZIP
  container is not itself a leaf; its decoded regular members are.
* Every global row has exactly the global header width. New columns backfill
  prior rows with missing bytes; absent source columns backfill current rows.
* Header order is first appearance after deterministic traversal, not sorted
  globally. Source root order remains authoritative.
* Empty logical row lists become one all-missing row at append time. A source
  set with no admitted files or rows, or a ZIP with no file members, fails.
* Parser-declared exact semantics survive merge unless an observed header
  conflict forces that one global rule to `Infer`.
* No format reader decodes a model tensor or image into host calculation
  payloads. Encoded bytes remain visible to later schema and operation logic.
* Every checked conversion and aggregate total either succeeds or returns
  `ArithmeticOverflow`; no wrapping count is used to bypass a limit.

## Error ordering and taxonomy

Errors are returned at the first failed stage. The complete source-stage order
is:

~~~text
source collection
  < symlink and root metadata checks
  < bounded regular-file snapshot
  < ZIP/container structure and expansion preflight
  < aggregate leaf, record, vector, and field admission
  < format decoding and logical-table shape
  < global merge and final RawTable construction
~~~

The typed categories are emitted as follows:

| Kind | Representative triggers and path behavior |
| --- | --- |
| `InvalidPath` | Non-UTF-8 `FileContext::child`, path outside a directory root, or root not file/dir. The offending path is attached when available. |
| `Io` | `symlink_metadata`, `read_dir`, entry inspection, or `read_source_snapshot` errors. |
| `Symlink` | A root or recursive entry is symbolic. The symlink path is attached. |
| `EmptySource` | `distill_datasets([])`, an archive with no file members, an XLSX workbook with no nonempty worksheet, or final zero files/rows. |
| `LimitExceeded` | Aggregate bytes, decoded/declared member bytes, XLSX/PPTX expansion, records, vectors, textual values, source values, or archive depth. |
| `MalformedArchive` | ZIP open, raw member inspection, escaping member, member open/read, or XLSX/PPTX ZIP expansion inspection. |
| `MalformedFormat` | Invalid UTF-8 text, JSON syntax/shape alignment, worksheet decode, PPTX XML, GGUF, safetensors, or logical row width. |
| `Ingest` | `parse_table` rejects CSV framing, source limits, widths, or field bounds, or `RawTable::from_parts` rejects the final shape. |
| `ArithmeticOverflow` | Byte-to-`u64`, aggregate bytes/rows/files/vectors, duplicate suffix, path depth, ZIP read bound, or expansion total overflows. |

Parser and lower-layer details are retained in `detail`, while the stable
category and optional logical path let callers distinguish a bad declaration,
an unsafe source, a bound violation, and a malformed file without parsing an
error string. No fallback parser, retry, alternate source, or partial table is
returned after failure.

## Source map

The following map is an implementation index, not a second behavioral
specification. The source remains authoritative for exact control flow.

| Source range | Responsibility |
| --- | --- |
| `31-109` | Public format and source-error types, display, and result alias. |
| `119-159` | Owned `DistilledDataset` and public access, inference, and preparation methods. |
| `160-265` | Logical table/column records and path context. |
| `268-497` | Bounded accumulator, global header merge, source context insertion, and final dataset construction. |
| `500-585` | Public entrypoints and root source admission. |
| `587-678` | Deterministic directory traversal, immutable reads, and source-error mapping. |
| `679-824` | Extension/magic dispatch and recursive ZIP traversal. |
| `825-1147` | Leaf dispatch, delimited/text, payload, and JSON parsing. |
| `1149-1380` | XLSX and PPTX extraction. |
| `1382-1594` | GGUF and safetensors table projection. |
| `1596-1728` | Fourteen-vector source metadata and collision handling. |
| `1730-1888` | Field bounds, extension and archive preflight, magic tests, text/table heuristics, and delimiter width. |

The observable contract is therefore one bounded operation:

~~~text
declared paths
  -> closed, bounded source snapshots or bounded archive members
  -> deterministic format-specific LogicalTable values
  -> one global source-context-aware Accumulator
  -> owned rectangular RawTable plus parser semantic rules
  -> semantic inference, target-free selection, or typed preparation
~~~
