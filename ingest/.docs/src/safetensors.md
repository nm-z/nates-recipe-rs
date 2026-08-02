---
module: ingest/src/safetensors.rs
crate: recipe-ingest
role: bounded_safetensors_container_parser
public_api:
  - SafeTensorDType
  - SafeTensorLimits
  - SafeTensorEntry
  - SafeTensorArchive
  - SafeTensorErrorKind
  - SafeTensorError
  - SafeTensorResult
  - parse_safetensors
direct_consumer: ingest/src/dataset.rs::parse_safetensor_tables
dispatch: ingest/src/dataset.rs::parse_leaf extension == safetensors
payload_policy: retain_encoded_tensor_bytes; never_decode_or_calculate_on_cpu
ordering: metadata_and_entries_are_deterministically_sorted
failure_policy: fail_closed_without_partial_archive
authority:
  - ingest/src/safetensors.rs
  - ingest/src/dataset.rs
  - ingest/src/semantic.rs
  - ingest/src/prepare.rs
  - ingest/src/lib.rs
---

# Safetensors ingestion

`ingest/src/safetensors.rs` is the bounded structural parser for one complete
safetensors image. It validates the length-prefixed JSON header, metadata,
tensor descriptors, arithmetic, and ownership of the data section. It returns
an immutable view of the encoded bytes and never interprets a tensor value as
an f32, int32, image, or other calculation payload. The parser is a format
boundary, not a model loader, tensor decoder, numerical converter, or GPU
operation.

The public entry point is:

```rust
pub fn parse_safetensors(
    bytes: &[u8],
    limits: SafeTensorLimits,
) -> SafeTensorResult<SafeTensorArchive<'_>>
```

The result borrows `bytes`, so the caller must keep the complete source image
alive for as long as it uses `SafeTensorArchive::data` or
`SafeTensorArchive::encoded_tensor`. Header-owned names, shapes, metadata, and
validated entries are copied into the archive. A failed parse returns only a
`SafeTensorError`; no partially validated archive is exposed.

## Machine-readable contract

The implementation has the following shape. The names in this block are
source-level names, not a second runtime representation.

```yaml
image:
  prefix: one little-endian u64 at bytes 0..8
  header: JSON object at bytes 8..(8 + header_bytes)
  data: all bytes after the header
header:
  reserved_field: __metadata__
  metadata: string_to_string_object
  tensor_fields: [dtype, shape, data_offsets]
tensor_validation:
  dtype: exact supported uppercase spelling
  shape: product of u64 dimensions, checked in u64
  encoded_bytes: end - begin, exactly product * dtype.element_bytes()
  offsets: relative to data, in [0, data_bytes]
  ownership: sorted tensor spans cover data exactly once, with no gap or overlap
archive:
  data_start: absolute byte offset of the data section
  metadata: BTreeMap<String, String>
  entries: tensor entries sorted by name
consumer:
  valid_extension: safetensors, case-folded by dataset path handling
  metadata_table: metadata_key, metadata_value
  tensor_table: tensor_name, tensor_type, tensor_shape, tensor_rank, tensor_bytes, binary
  empty_archive: one Binary/Bytes row containing the complete original image
conversion:
  parser: encoded bytes only
  later_preparation: VectorEncoding::Bytes remains variable-width and has no scalar dtype
```

## Ownership and boundaries

The module belongs to the lexical ingest layer. `ingest/src/lib.rs` keeps the
module private and re-exports its public types and `parse_safetensors`, so a
library caller can parse an already admitted byte slice with explicit bounds.
There is one executable repository caller of the parser,
`dataset::parse_safetensor_tables`; source and operation registries may name
`parse_safetensors`, but they do not invoke it. The operation registry classifies
that symbol, and any source containing `safetensors`, as the non-calculation
`ModelContainerParsing` operation in the `Parsing` family. This classification
does not authorize a CPU calculation fallback.

The parser does not:

* discover files or decide whether a path is safe to open;
* inspect a file extension or safetensors magic signature;
* decompress archives;
* validate a model schema, layer topology, or parameter names beyond the
  container's tensor-name rules;
* decode F16, BF16, F32, or any other encoded value;
* perform endian conversion, dequantization, normalization, or shape
  reshaping; or
* allocate a runtime device buffer or schedule a calculation.

Those concerns remain with source traversal, model preparation, and the
calculation graph. The parser's dtype size calculation is only structural
arithmetic used to prove that a span has the declared length.

## Source map

The implementation is intentionally compact, but each region owns one
distinct part of the boundary:

| Source region | Responsibility |
| --- | --- |
| `ingest/src/safetensors.rs:9-65` | Supported dtype vocabulary and structural element widths. |
| `ingest/src/safetensors.rs:67-113` | Nonzero caller limits and accessors. |
| `ingest/src/safetensors.rs:115-183` | Borrowed archive model, tensor entries, lookup, and encoded slices. |
| `ingest/src/safetensors.rs:185-227` | Public error kinds, error payload, display, and result alias. |
| `ingest/src/safetensors.rs:229-391` | Length-prefix parsing, bounds, tensor validation, and archive construction. |
| `ingest/src/safetensors.rs:393-442` | Name and contiguous physical-span validation. |
| `ingest/src/safetensors.rs:444-586` | Custom serde visitors for the header, metadata, and tensor fields. |
| `ingest/src/safetensors.rs:588-616` | JSON end-of-input checking and nonzero-limit helpers. |
| `ingest/src/lib.rs:22,59-61` | Private module declaration and public re-exports. |
| `ingest/src/dataset.rs:825-849` | Extension-based leaf dispatch to the safetensors table reader. |
| `ingest/src/dataset.rs:1503-1593` | Consumer limits, error wrapping, metadata/tensor tables, and empty fallback. |
| `ingest/src/semantic.rs:24-43` | `VectorEncoding::Bytes` has no scalar calculation dtype. |
| `ingest/src/prepare.rs:1540-1668` | Bytes remain variable-width during later semantic preparation. |

The line ranges identify the current source layout and are useful when
checking this document against the implementation. They do not imply that the
documentation is a generated copy of comments or private helper names.

## Safetensors image layout

The parser implements the length-prefixed safetensors layout directly:

```text
offset 0                 8-byte little-endian unsigned header length H
offset 8                 H bytes of one JSON object
offset 8 + H             data section, borrowed without copying
```

`H` is not a host `usize`; it is read as `u64`. `data_start` in the returned
archive is the absolute `u64` value `8 + H`. Every `data_offsets` pair in a
tensor descriptor is relative to the beginning of the borrowed data section,
not relative to the beginning of the image or the JSON header. There is no
additional magic, version, alignment, padding, or checksum check in this
module. Whitespace permitted by `serde_json` inside the header remains part of
the counted `H` bytes.

A schematic valid image is:

```text
[H as eight little-endian bytes]
{"__metadata__":{"format":"demo"},"weights":{"dtype":"U8","shape":[3],"data_offsets":[0,3]}}
[three data bytes]
```

The numeric prefix must equal the UTF-8 byte length of the shown JSON object,
and the three data bytes are the complete data section. The parser does not
require tensor descriptors to appear in offset order in JSON. It sorts spans
for the ownership check and sorts the final entries by tensor name.

An empty JSON object is structurally valid only when the data section is also
empty. A scalar tensor has an empty `shape` array, whose product is one. A
dimension of zero gives a zero-element tensor and therefore a zero-byte span;
zero-length spans are allowed when the complete data section is correspondingly
covered.

## Public data model

### `SafeTensorDType`

`SafeTensorDType` is a copyable, ordered enum with the exact accepted header
spellings below. `SafeTensorDType::parse` is private and is called only while
validating a tensor descriptor. Callers can inspect a parsed value and call
`element_bytes`, but cannot construct it from an arbitrary string through a
public parser.

| Header spelling | Variant | Bytes per element |
| --- | --- | ---: |
| `BOOL` | `Bool` | 1 |
| `U8` | `U8` | 1 |
| `I8` | `I8` | 1 |
| `U16` | `U16` | 2 |
| `I16` | `I16` | 2 |
| `F16` | `F16` | 2 |
| `BF16` | `Bf16` | 2 |
| `U32` | `U32` | 4 |
| `I32` | `I32` | 4 |
| `F32` | `F32` | 4 |
| `U64` | `U64` | 8 |
| `I64` | `I64` | 8 |
| `F64` | `F64` | 8 |

The match is exact and case-sensitive. Any other string, including a spelling
that a newer safetensors producer may know, returns
`SafeTensorErrorKind::UnsupportedDType`. `element_bytes` returns only the
structural width shown above. It does not establish a calculation dtype or
decode a payload.

### `SafeTensorLimits`

`SafeTensorLimits::new(header_bytes, data_bytes, tensors, rank, name_bytes)`
creates the caller-owned bounds for one parse. All five values must be
nonzero. The values are stored as `NonZeroU64` or `NonZeroU32`, and the
accessors return those nonzero wrappers:

| Bound | Type | Checked against |
| --- | --- | --- |
| `header_bytes` | `NonZeroU64` | The little-endian JSON header length before the header is sliced. |
| `data_bytes` | `NonZeroU64` | The number of bytes after the header. |
| `tensors` | `NonZeroU32` | The number of non-metadata top-level fields. |
| `rank` | `NonZeroU32` | The length of each tensor's `shape` vector. |
| `name_bytes` | `NonZeroU64` | The UTF-8 byte length of each tensor name. |

Passing zero for any bound returns `InvalidLimit`, with no usable limits
value. There are deliberately no parser bounds for metadata entry count,
metadata key length, metadata value length, or aggregate shape dimensions
beyond the header-byte bound and checked `u64` arithmetic. A caller that wants
those limits must bound the whole header before invoking this API.

### `SafeTensorEntry`

Each entry is a validated tensor descriptor:

| Accessor | Meaning |
| --- | --- |
| `name()` | Nonempty tensor name as an owned UTF-8 `String`. |
| `dtype()` | Parsed `SafeTensorDType`. |
| `shape()` | Owned `Vec<u64>` exposed as a slice, in header order. |
| `begin()` | Relative data-section start offset. |
| `end()` | Relative data-section end offset, exclusive. |
| `encoded_bytes()` | `end - begin`, already proven to equal the shape and dtype byte count. |

The offsets are not absolute file positions. To address the original image,
add `archive.data_start()` to the relative offset; to obtain the bytes, use
`archive.encoded_tensor(name)`, which performs the relative slice directly.
`encoded_bytes()` is an infallible subtraction because construction only
retains entries after `end >= begin` has been checked.

### `SafeTensorArchive<'a>`

The archive owns metadata and entry descriptors while borrowing the complete
data section:

| Accessor | Result and ordering |
| --- | --- |
| `data_start()` | Absolute `u64` image offset at which `data()` begins. |
| `data()` | Borrowed `&'a [u8]` containing every data byte, including bytes not selected by a named call. |
| `metadata()` | `&BTreeMap<String, String>`, ordered by metadata key. |
| `entries()` | `&[SafeTensorEntry]`, ordered by tensor name. |
| `entry(name)` | Binary-search lookup in the name-sorted entries. |
| `encoded_tensor(name)` | Borrowed relative span, or `None` for an unknown name or an unrepresentable host slice. |

The validated range makes `encoded_tensor` present for every entry on a normal
host. It still returns an `Option` because converting a `u64` offset to
`usize`, or obtaining the slice from `data`, can fail on a host whose address
space cannot represent that range. The archive derives `Clone`, `Debug`, and
`Eq`; cloning duplicates descriptors and maps but does not copy the borrowed
source bytes.

## Header grammar and duplicate handling

`parse_header` feeds the exact header slice to a custom `serde_json`
deserializer. The grammar accepted by the visitors is narrower than a generic
JSON object:

```text
header       := JSON object
top-level    := "__metadata__" -> metadata | tensor_name -> tensor
metadata     := JSON object whose keys and values are strings
tensor        := JSON object with exactly dtype, shape, data_offsets
dtype        := JSON string
shape        := JSON array deserializable as Vec<u64>
data_offsets := JSON array deserializable as [u64; 2]
```

The details are important:

* The reserved top-level name `__metadata__` is always interpreted as
  metadata. It cannot be used for a tensor name. If it is present more than
  once, the top-level duplicate check rejects it.
* Every other top-level name is a tensor name. Top-level names are inserted
  into a `BTreeSet` before their values are read, so duplicate JSON keys are
  rejected rather than silently overwritten.
* Metadata values must be JSON strings. Numbers, booleans, arrays, objects,
  and null values are malformed headers. Duplicate metadata keys are rejected
  by `MetadataVisitor`, even though a generic JSON map would normally retain
  only the last value.
* A tensor object accepts only `dtype`, `shape`, and `data_offsets`. Unknown
  fields are rejected. `set_once` rejects a repeated known field, and all
  three fields are required. Missing fields, wrong JSON types, a shape element
  that is not a `u64`, or an offsets array that is not exactly two `u64`
  values are malformed headers.
* Header JSON must contain one object and then only whitespace. The explicit
  `Deserializer::end()` check rejects a second JSON value or any other
  trailing content inside the declared header slice.

Serde errors are mapped to `DuplicateField` when their display text contains
`duplicate`; all other deserialization and trailing-content errors are mapped
to `MalformedHeader`. This is the actual classification rule, so callers
should match the public kind rather than parse the human-readable detail.

The visitors collect tensors and metadata into `BTreeMap`s. Input object order
therefore does not affect the final archive or the dataset rows generated from
it. The name-sorted map also means the tensor-count limit is checked before
descriptor validation, while descriptor validation itself runs in lexical
name order.

## Parse and validation sequence

`parse_safetensors` is intentionally a single fail-closed pipeline. The first
failure in this sequence is returned:

1. Read exactly the first eight bytes. Fewer than eight bytes is
   `Truncated`. Decode the prefix as little-endian `u64` `header_bytes`.
2. Compare `header_bytes` with `limits.header_bytes()`. An oversized declared
   header returns `HeaderLimitExceeded` before any header slice is attempted.
3. Compute `data_start = 8 + header_bytes` with checked `u64` addition, then
   convert it to `usize` for host slicing. Either operation can return
   `ArithmeticOverflow`.
4. Slice the header from byte 8 through `data_start`. If the complete image
   does not contain that range, return `Truncated`. The remaining bytes become
   the borrowed data section.
5. Convert `data.len()` to `u64` and compare it with
   `limits.data_bytes()`. Conversion failure is `ArithmeticOverflow`; an
   oversized data section is `DataLimitExceeded`.
6. Deserialize and fully consume the JSON header as described above. Header
   syntax and duplicate errors return `MalformedHeader` or `DuplicateField`.
7. Convert the parsed tensor-map length to `u32`, then compare it with
   `limits.tensors()`. Conversion failure is `ArithmeticOverflow`; an
   oversized map is `TensorLimitExceeded`.
8. For every tensor in name order, validate its name, rank, dtype, shape
   product, expected byte count, and offset pair. A failure stops the loop and
   no archive is returned.
9. Sort all validated spans by `(begin, end, name)` and require that the first
   begin is zero, each subsequent begin equals the previous end, and the final
   end equals `data_bytes`. This rejects overlap, gaps, and trailing unowned
   bytes.
10. Sort entries by name for binary-search lookup and return the archive.

The parser never trusts header order for physical ownership, and it never uses
an unchecked multiplication or subtraction to decide a slice.

## Tensor arithmetic and physical ownership

For a descriptor with shape dimensions `d_0 ... d_n` and dtype width `w`, the
parser computes:

```text
elements       = checked_product([d_0, ..., d_n]), starting at 1
expected_bytes = checked_mul(elements, w)
encoded_bytes  = checked_sub(end, begin)
```

`encoded_bytes` must equal `expected_bytes`. This check means a tensor with
shape `[2, 3]` and `F32` must occupy 24 bytes, while a scalar `F64` with an
empty shape must occupy 8 bytes. A zero dimension produces a zero-byte tensor.
The parser does not require a nonzero rank, nonzero dimensions, or a particular
stride because safetensors stores dense contiguous spans, not a stride model.

Offset validation is separate from shape validation:

* `end < begin` is `InvalidOffset`.
* `end > data_bytes` is `InvalidOffset`.
* A span whose length is not the expected dtype and shape size is
  `InvalidShape`.
* After all individual checks, any gap, overlap, or trailing data is
  `NonContiguousData`.

Because `begin` is checked through `end >= begin` and `end <= data_bytes`, a
valid span cannot address bytes outside `data()`. The final ownership pass also
means an otherwise valid descriptor cannot leave arbitrary padding or an
unclaimed suffix in the image. Empty spans may share the same cursor when
their expected size is zero; nonempty spans cannot overlap.

## Failure taxonomy

`SafeTensorErrorKind` is `#[non_exhaustive]`; callers must include a wildcard
when matching it. `SafeTensorError` exposes the kind and a human-readable
detail, formats as `Kind: detail`, and implements `std::error::Error`. It does
not carry a filesystem path. The dataset consumer adds the logical path when
wrapping it in `DatasetSourceError`.

| Kind | Returned when |
| --- | --- |
| `InvalidLimit` | `SafeTensorLimits::new` receives zero for any bound. |
| `Truncated` | The image is shorter than the eight-byte prefix or shorter than the declared header range. |
| `HeaderLimitExceeded` | The declared JSON header length is greater than `limits.header_bytes()`. |
| `DataLimitExceeded` | The remaining data section is greater than `limits.data_bytes()`. |
| `TensorLimitExceeded` | The parsed tensor map contains more entries than `limits.tensors()`. |
| `RankLimitExceeded` | A shape vector has more dimensions than `limits.rank()`. |
| `NameLimitExceeded` | A nonempty tensor name exceeds `limits.name_bytes()` UTF-8 bytes. |
| `MalformedHeader` | JSON is not the required object shape, a tensor name is empty, a tensor field is missing or unknown, a JSON value has the wrong type, or the header has trailing content. |
| `DuplicateField` | A duplicate top-level field, metadata key, or tensor field is reported by serde as text containing `duplicate`. |
| `UnsupportedDType` | `dtype` is not one of the thirteen exact supported spellings. |
| `InvalidShape` | The offset span length differs from shape product times dtype width. |
| `InvalidOffset` | Offsets are reversed or `end` exceeds the data section. |
| `NonContiguousData` | Sorted spans do not start at the current cursor, or the final cursor is not the data length. |
| `ArithmeticOverflow` | A `u64` addition, `usize` conversion, tensor/rank `u32` conversion, shape product, byte-count multiplication, or name/data length conversion cannot be represented. |

The parser does not normalize any of these errors into success. In particular,
an unsupported dtype is not retained as an opaque tensor, an invalid span is
not truncated to the available bytes, and unowned data is not silently
discarded.

## Dataset consumer

### Dispatch and parser bounds

The recursive source distiller in `ingest/src/dataset.rs` selects
`parse_safetensor_tables` from `parse_leaf` when the logical path extension,
after ASCII lowercasing, is exactly `safetensors`. `visit_bytes` performs ZIP
and specialized-container dispatch before `parse_leaf`; therefore a nested
archive member named `weights.safetensors` follows the same path after it has
been safely extracted, while a safetensors-looking byte stream with no such
extension follows another leaf rule.

Before calling the parser, the distiller has already read the leaf under the
aggregate `IngestLimits::source_bytes` bound and called `Accumulator::admit_leaf`.
`parse_safetensor_tables` then derives parser bounds from this in-memory image:

```text
byte_count   = max(u64::try_from(bytes.len()).unwrap_or(u64::MAX), 1)
tensor_limit = max(u32::try_from(byte_count).unwrap_or(u32::MAX), 1)
limits       = SafeTensorLimits::new(
    byte_count,       // header bytes
    byte_count,       // data bytes
    tensor_limit,
    64,               // rank
    byte_count,       // tensor-name bytes
)
```

The `max(1)` calls ensure the parser's nonzero constructor contract even for
an empty source. The parser's header and data bounds are therefore no larger
than the complete source image, and the consumer imposes a rank bound of 64.
The source-level record, field, and aggregate-byte bounds remain enforced by
the surrounding distiller and accumulator; they are not hidden inside the
safetensors parser.

Parser or parser-limit errors are wrapped as
`DatasetSourceErrorKind::MalformedFormat` with detail beginning
`parse safetensors source:` or `construct safetensors parser limits:` and with
the current logical path attached. The wrapper is the only place in this path
that adds a source path. A malformed safetensors image cannot produce a partial
logical table.

### Metadata table

When `archive.metadata()` is nonempty, the consumer emits one logical table
whose member is `metadata` and whose `SourceFormat` is `SafeTensors`:

| Column | Semantic rule | Row value |
| --- | --- | --- |
| `metadata_key` | `Classify` | UTF-8 bytes of the sorted metadata key. |
| `metadata_value` | `Infer` | UTF-8 bytes of the sorted metadata value. |

Rows follow `BTreeMap` key order. Metadata values were already required to be
JSON strings, but they are still raw bytes in the table and are not parsed as
numbers or converted to a calculation type here. An empty metadata map emits
no metadata table.

### Tensor table

When `archive.entries()` is nonempty, the consumer emits one logical table
whose member is `tensors` and whose `SourceFormat` is `SafeTensors`:

| Column | Semantic rule | Row value |
| --- | --- | --- |
| `tensor_name` | `Classify` | Entry name bytes, in name order. |
| `tensor_type` | `Classify` | `format!("{:?}", entry.dtype())`, such as `F32`. |
| `tensor_shape` | `Classify` | Shape dimensions joined by `x`, such as `2x3`; an empty shape becomes an empty value. |
| `tensor_rank` | Exact `Numeric` / `I32` | Decimal shape length. |
| `tensor_bytes` | Exact `Numeric` / `I32` | Decimal `entry.encoded_bytes()`. |
| `binary` | Exact `Binary` / `Bytes` | `archive.encoded_tensor(entry.name())` copied to an owned vector. |

The parser has already proven that every `encoded_tensor` lookup is the entry's
validated span. The consumer nevertheless uses `unwrap_or_default()` on the
optional API and copies an absent result as an empty value. Under a successful
parse, absence can only arise from a host offset conversion or slice
representation failure, so this is not a second validation path. A parser
error has already stopped the operation before rows are built.

`LogicalTable::new` checks rectangular row width. The outer `Accumulator`
then merges tables from all sources into one sparse, ordered `RawTable`; cells
that belong to another source table remain empty. When source context is
enabled for a directory, archive member, or multi-source declaration, each row
also receives the standard fourteen source metadata columns. A data column
whose name collides with one of those context names is prefixed with `data:`.
The context records `format = safetensors` and `member = metadata`, `tensors`,
or the empty fallback member.

### Empty archive fallback

If both the metadata map and tensor entries are empty, the consumer emits no
metadata or tensor table. It instead emits one `single_payload` table with:

```text
format:       SafeTensors
member:       empty
column:       binary
semantic:     Binary
encoding:     Bytes
row:          the complete original safetensors image bytes
```

This fallback is reachable for a valid `{}` header with an empty data section.
It is not a recovery path for a malformed archive, unsupported dtype, invalid
offset, or failed limit check. Those cases return `MalformedFormat` from the
dataset boundary.

## Conversion and preparation boundary

`SafeTensorDType` and `SafeTensorEntry::encoded_bytes` preserve format facts;
they do not convert payloads. In the dataset tensor table, `tensor_rank` and
`tensor_bytes` are textual decimal fields with exact `I32` semantic rules, but
the `binary` field remains the original variable-width bytes. The parser never
turns an F32 byte sequence into host `f32` values, and it never interprets
BF16, F16, or integer bytes as numbers.

Later semantic inference and preparation consume the resulting `RawTable`.
`VectorEncoding::Bytes` has no calculation dtype (`dtype()` returns `None`),
and `prepare` retains it as `PreparedValues::VariableWidth`. Binary tensor
payloads therefore remain opaque unless a separately declared operation gives
them a typed lowering. The same boundary applies to the empty-archive
fallback. The exact `Numeric/I32` columns for rank and byte count can be
encoded as int32 by normal preparation, but that is downstream semantic
preparation, not safetensors parsing.

This separation preserves the crate-wide contract: external representation is
validated and copied before runtime, while f32/int32 calculation payloads are
admitted only through the later preparation and GPU calculation path. Parsing
is pre-run model-container work and does not add a calculation task.

## Invariants to preserve

The following properties are observable at the public boundary and should be
kept together if the parser changes:

1. Limits are explicit and nonzero. No parser-side default limits or retries
   may replace a caller's `SafeTensorLimits`.
2. The eight-byte prefix, header slice, and data slice use checked arithmetic
   and host-representable offsets before indexing.
3. Header objects, metadata keys, tensor names, and tensor fields are unique;
   unknown tensor fields and non-string metadata are rejected.
4. Every accepted dtype is one of the exact supported spellings, and every
   accepted span has the exact shape-product byte count.
5. Tensor offsets are relative to `data`, and sorted spans cover the complete
   data section with no gap, overlap, or trailing bytes.
6. Returned entries are name-sorted for deterministic lookup, while their
   physical ownership is checked independently by offset order.
7. The archive borrows source bytes and never decodes them. Downstream users
   must not treat a `SafeTensorDType` as proof that a calculation payload has
   been produced.
8. Dataset dispatch remains extension-based and maps parser failures to
   `MalformedFormat` with the logical path. A malformed source must not become
   a binary fallback row.
9. A valid archive with no metadata and no entries uses the explicit empty
   archive fallback; that fallback must not be generalized to other failures.

These invariants define the parser's trust boundary. They are stronger than
JSON deserialization alone and are the reason callers can safely use the
returned spans without rechecking overlap or tensor byte counts.
