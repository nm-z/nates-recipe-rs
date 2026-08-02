# ingest/src/gguf.rs

## Role and boundary

This module is Recipe's bounded, zero-copy structural reader for a complete
GGUF image. It is a binary container boundary, not a model interpreter. The
public parse_gguf(bytes, limits) function validates the header, metadata,
tensor descriptors, encoded block geometry, alignment, padding, and every
tensor span before returning a GgufArchive. The returned archive borrows
strings and tensor bytes from the caller's bytes slice. It never decodes,
dequantizes, or copies a tensor payload.

The module is private to the recipe-ingest crate, but ingest/src/lib.rs
re-exports all of its public types and parse_gguf. Callers therefore use this
module as a stable ingestion API while the implementation keeps Reader,
RawTensor, and all parser helpers private. The source has forbid(unsafe_code)
through the crate facade.

The boundary is intentionally complete and fail-closed:

* GGUF versions 2 and 3 are recognized in little-endian form. Version 3 is
  also recognized in big-endian form; big-endian version 2 is rejected.
* All metadata scalar types and typed, nested metadata arrays represented by
  this module are retained.
* Every tensor type represented by GgufTensorType has a checked block shape
  and encoded byte width. Payload bytes remain opaque to this module.
* Unknown version, metadata, or tensor codes, malformed text, bad counts,
  impossible dimensions, bad offsets, overlap, nonzero padding, trailing bytes,
  and arithmetic or host-address overflow return an error. No partially parsed
  archive is returned.

The structural OGDL converter and the first executable llama instrument impose
additional policies after this boundary. For example, conversion currently
requires version 3, while the llama execution path requires little-endian,
dense F32 tensors and the llama architecture. Those are consumer policies,
not claims made by parse_gguf.

## Source map

The following map records the implementation regions traced for this document.
Line numbers refer to the current checkout.

| Source region | Current lines | Boundary |
| --- | ---: | --- |
| ingest/src/gguf.rs constants, endian, and limits | 8-77 | Format constants and nonzero caller budgets. |
| ingest/src/gguf.rs metadata types and values | 79-217 | Type-code mapping, borrowed values, and iterative array destruction. |
| ingest/src/gguf.rs tensor types and block geometry | 219-412 | Accepted GGML codes and encoded block widths. |
| ingest/src/gguf.rs tensor/archive/error values | 414-566 | Public descriptors, borrowing archive, and fail-closed error surface. |
| ingest/src/gguf.rs Reader | 568-872 | Checked slicing, endian-aware scalars, strings, and array frames. |
| ingest/src/gguf.rs parse_gguf | 874-1109 | Complete in-memory image parse and archive construction. |
| ingest/src/gguf.rs materialization and encoded lengths | 1111-1198 | Checked tensor byte spans. |
| ingest/src/gguf.rs span and alignment validation | 1200-1305 | Overlap, padding, terminal length, and general.alignment. |
| ingest/src/gguf.rs format helpers | 1307-1484 | Name/key checks, version detection, zero padding, and arithmetic helpers. |

The main consumers are visible at
ingest/src/dataset.rs:1382-1501,
ingest/src/gguf_ogdl.rs:225-291 and 2578-2589,
training/src/gguf_llama.rs:187-483 and 1111-1217, and
src/inference.rs:500-600. The seekable converter's independent bounded
reader is in ingest/src/gguf_ogdl.rs:691-1065.

## Wire layout and coordinate systems

GGUF fields are laid out in this order:

| Region | Encoding | Meaning |
| --- | --- | --- |
| 0..4 | Four literal bytes | ASCII magic GGUF. |
| 4..8 | u32 in the detected byte order | Version, currently 2 or 3. |
| 8..16 | u64 in the detected byte order | Tensor count. |
| 16..24 | u64 in the detected byte order | Metadata pair count. |
| Metadata section | Repeated records | Each record is a length-prefixed UTF-8 key, a u32 metadata type code, and the value encoded by that type. |
| Tensor-info section | Repeated records | Each record is a length-prefixed UTF-8 name, a u32 rank, rank u64 dimensions, a u32 tensor type code, and a u64 data offset. |
| Header padding | Zero bytes | Bytes from the end of tensor-info records to the aligned tensor-data start. |
| Tensor-data section | Raw encoded blocks | Tensor offsets are relative to this section. |

Reader::new starts at byte 8, so the two counts are the first values read by
the reader. All integer fields, string lengths, dimensions, type codes, and
offsets use the endian selected from the version bytes. Tensor payload bytes
are not byte-swapped or interpreted.

There are two offset spaces:

* GgufTensor::data_offset is relative to GgufArchive::data_start, the first
  byte of the tensor-data section.
* GgufTensor::file_offset is the absolute offset in the original bytes slice,
  equal to data_start + data_offset.

GgufArchive::data() returns only the tensor-data section, not the header.
GgufArchive::raw_tensor(name) converts a tensor's relative half-open span to
an &[u8] slice of that section. A valid archive has already checked the
range, but the method still returns None if a u64 to host usize conversion or
the final slice lookup fails.

## Caller-supplied bounds

GgufLimits is an aggregate, nonzero limit set. Its fields are private and can
only be populated through GgufLimits::new; a zero in any argument returns
GgufErrorKind::InvalidLimit. Accessors return NonZeroU64 or NonZeroU32, so the
parser never has a zero limit:

| Limit | Applied to |
| --- | --- |
| file_bytes | bytes.len() before any field is read. |
| metadata_pairs | Declared metadata count. |
| tensors | Declared tensor count. |
| rank | Each rank, in addition to the current format maximum of four. |
| string_bytes | Aggregate byte budget for every metadata key, tensor name, and metadata string value, including strings nested in arrays. |
| array_elements | Aggregate count of elements declared by every metadata array, including nested arrays. Each array length is charged before its values are read. |
| array_depth | Maximum nested array depth. A root array has depth one. |

The caller chooses these bounds for its trust context. The format limits remain
authoritative even when a caller supplies larger limits: metadata keys are at
most 65,535 bytes, tensor names at most 64 bytes, and ranks at most four.
GgufLimits::new does not make the file limit equal to the input length; the
parser compares the two and rejects an oversized image.

Current call-site policies are:

| Caller | Limits passed |
| --- | --- |
| ingest::dataset::parse_gguf_tables | Every byte-sized bound is the source file length (clamped to at least one), rank 64, and array depth 64. The parser's rank maximum of four still wins. Parse failures become DatasetSourceErrorKind::MalformedFormat. |
| training::gguf_llama::load_gguf_llama_model_file through src/inference.rs | The file's metadata length, clamped to at least one, for file, counts, strings, arrays, and array depth, with rank four. A bounded SourceSnapshot read uses the same file limit before parse_gguf. |
| recipe convert setup in src/cli.rs | Source length for count and string budgets, rank four, and the requested output length as the file limit. The conversion itself uses the seekable streaming reader; the in-memory reverse conversion reparses generated bytes through this module. |
| gguf_to_structural_ogdl and structural_ogdl_to_gguf | Caller-provided limits. The reverse in-memory conversion reparses its generated image before returning it. |

## Metadata types and owned shape

GgufMetadataType maps the wire code exactly:

| Code | Variant | Value encoding |
| ---: | --- | --- |
| 0 | U8 | One unsigned byte. |
| 1 | I8 | One signed byte, represented by its two's-complement byte. |
| 2 | U16 | Endian-aware two-byte unsigned integer. |
| 3 | I16 | Endian-aware two-byte signed integer. |
| 4 | U32 | Endian-aware four-byte unsigned integer. |
| 5 | I32 | Endian-aware four-byte signed integer. |
| 6 | F32 | Endian-aware four-byte IEEE bit pattern, retained as u32. |
| 7 | Bool | Exactly one byte, 0 or 1. |
| 8 | String | u64 byte length followed by UTF-8 bytes. |
| 9 | Array | u32 element type, u64 element count, then that many values of the declared type. The element type may itself be Array. |
| 10 | U64 | Endian-aware eight-byte unsigned integer. |
| 11 | I64 | Endian-aware eight-byte signed integer. |
| 12 | F64 | Endian-aware eight-byte IEEE bit pattern, retained as u64. |

GgufMetadataType::parse and code are inverse for codes 0 through 12. Any
other code returns UnsupportedMetadataType. The enum is Copy, ordered,
hashable, and public. Its parse method is crate-private because only binary
readers should admit numeric codes.

GgufMetadataValue<'a> carries the corresponding typed value. Floating point
values intentionally use F32Bits(u32) and F64Bits(u64), preserving NaN
payloads, signed zero, and all other source bits. value_type() maps each
variant back to its declared GgufMetadataType; it does not convert numeric
types.

GgufMetadataArray<'a> stores the declared element_type and a Vec of
GgufMetadataValue<'a>. Array elements are homogeneous in the binary image,
and nested arrays are represented as Array(GgufMetadataArray { ... }).
Strings inside values and arrays borrow the source image. The custom Drop
implementation drains nested arrays iteratively instead of recursively, so a
deep valid array uses the configured depth budget without making destruction
depend on call-stack recursion.

GgufMetadataEntry<'a> stores a borrowed key and one value. key() and value()
expose them without copying. The parser requires a key to be valid UTF-8,
nonempty, at most 65,535 bytes, and unique across the archive. The key-length
bound is measured in UTF-8 bytes, not Unicode scalar values.

### Metadata decoding and budgets

Reader::read_metadata_value uses an explicit ArrayFrame stack. It does not
recurse through nested arrays:

1. For an array marker, it computes frames.len() + 1 and checks array_depth.
2. It reads and validates the element type and length.
3. It checks and subtracts the length from array_elements.
4. It multiplies the element's minimum wire width by the length with checked
   arithmetic and rejects a section that cannot contain that many bytes.
5. It allocates a bounded Vec and iteratively reads each scalar or child array,
   closing frames from the inside out.

An empty array is returned immediately after its type and length. A nested
array consumes one depth level and its own declared element count. Strings
consume the shared string budget when their bytes are actually taken. A scalar
Array reaching read_metadata_scalar is an internal impossible state and
returns UnsupportedMetadataType.

Scalar decoding is endian-aware for widths larger than one byte. Boolean
values other than zero or one return InvalidBoolean; no truthiness conversion
is performed. A string length is first checked against its per-value bound and
the aggregate remaining string budget, then converted to host usize, checked
against the input slice, validated as UTF-8, and finally charged to the
aggregate budget.

## Tensor types and encoded geometry

GgufTensorType accepts the current codes below. block_elements() is the number
of logical tensor elements represented by one encoded block, and block_bytes()
is the exact number of bytes occupied by that block:

| Code | Variant | Block elements | Block bytes |
| ---: | --- | ---: | ---: |
| 0 | F32 | 1 | 4 |
| 1 | F16 | 1 | 2 |
| 2 | Q4_0 | 32 | 18 |
| 3 | Q4_1 | 32 | 20 |
| 6 | Q5_0 | 32 | 22 |
| 7 | Q5_1 | 32 | 24 |
| 8 | Q8_0 | 32 | 34 |
| 9 | Q8_1 | 32 | 36 |
| 10 | Q2K | 256 | 84 |
| 11 | Q3K | 256 | 110 |
| 12 | Q4K | 256 | 144 |
| 13 | Q5K | 256 | 176 |
| 14 | Q6K | 256 | 210 |
| 15 | Q8K | 256 | 292 |
| 16 | Iq2Xxs | 256 | 66 |
| 17 | Iq2Xs | 256 | 74 |
| 18 | Iq3Xxs | 256 | 98 |
| 19 | Iq1S | 256 | 50 |
| 20 | Iq4Nl | 32 | 18 |
| 21 | Iq3S | 256 | 110 |
| 22 | Iq2S | 256 | 82 |
| 23 | Iq4Xs | 256 | 136 |
| 24 | I8 | 1 | 1 |
| 25 | I16 | 1 | 2 |
| 26 | I32 | 1 | 4 |
| 27 | I64 | 1 | 8 |
| 28 | F64 | 1 | 8 |
| 29 | Iq1M | 256 | 56 |
| 30 | Bf16 | 1 | 2 |
| 34 | Tq1_0 | 256 | 54 |
| 35 | Tq2_0 | 256 | 66 |
| 39 | Mxfp4 | 32 | 17 |
| 40 | Nvfp4 | 64 | 36 |
| 41 | Q1_0 | 128 | 18 |
| 42 | Q2_0 | 64 | 18 |

Codes 4, 5, 31, 32, 33, 36, 37, and 38, plus every other code not in the
table, are rejected as UnsupportedTensorType. The error detail calls out
unsupported or removed GGML types. GgufTensorType::code() returns the
canonical code for every accepted variant.

For a tensor named name with dimensions d and type t, tensor_encoded_bytes
applies this exact rule:

    block_elements = t.block_elements()
    first_dimension = d[0] if rank > 0, otherwise 1
    require first_dimension % block_elements == 0
    elements = 0 if any dimension is zero, otherwise product(d)
    blocks = elements / block_elements
    encoded_bytes = blocks * t.block_bytes()

The first-dimension divisibility check is performed even when another
dimension is zero. A zero extent therefore gives an encoded length of zero
only when its first dimension is divisible by the block width. A zero-rank
tensor uses a synthetic first dimension of one: zero-rank scalar types with
one-element blocks are representable, while a zero-rank blocked quantized type
fails the divisibility check. Every multiplication is checked and overflow is
ArithmeticOverflow.

The parser does not inspect the meaning of block bytes. It only uses the table
to prove that each descriptor's span fits in the tensor-data section. Consumers
that need numeric values must implement the tensor-specific decoding policy.

## Public archive values

GgufTensor<'a> is the validated descriptor:

| Method | Meaning |
| --- | --- |
| name() | Borrowed UTF-8 tensor name. Empty names are permitted by this module, but names over 64 bytes are rejected and names must be unique. |
| dimensions() | Source-order u64 extents. The parser permits rank zero through four, subject to the caller rank limit. |
| tensor_type() | Accepted GgufTensorType. |
| data_offset() | Relative offset from GgufArchive::data_start. |
| file_offset() | Checked absolute offset in the original image. |
| encoded_bytes() | Checked size derived from dimensions and block geometry. |
| data_end() | data_offset + encoded_bytes; safe for descriptors returned by parse_gguf because materialization checked the addition. |

GgufArchive<'a> stores version, detected endian, validated alignment,
data_start, the borrowed data section, metadata in source order, and tensors
in source order. metadata_entry and tensor perform exact linear name/key
lookups. Duplicate keys and names were rejected during parsing, so a
successful lookup has at most one result. No map changes the order exposed by
metadata() or tensors().

## Parse pipeline

The implementation is deliberately staged so cheap bounds and encoding checks
happen before allocations or span arithmetic:

1. Convert bytes.len() to u64 and enforce limits.file_bytes.
2. Require at least four bytes for the magic and exactly GGUF; require bytes
   4 through 7 for the version and run detect_version.
3. Start Reader at byte 8, read endian-aware tensor and metadata counts, and
   enforce their caller limits.
4. Checked-multiply the count lower bounds (13 bytes per metadata record and
   33 bytes per tensor-info record), add them, and require the remaining input
   to be at least that large. Individual reads still perform their own bounds
   checks.
5. Read metadata records in source order. Validate each key, reject duplicate
   keys, parse its type code, and decode its value with string, array, UTF-8,
   boolean, and arithmetic bounds.
6. Derive alignment from general.alignment, defaulting to 32. The value must
   be metadata type U32, nonzero, and a multiple of eight.
7. Read tensor descriptors in source order. Validate the name length and
   uniqueness, require rank at most four and within limits.rank, read all
   dimensions, parse the tensor type, and require an offset aligned to the
   derived alignment.
8. Record the byte immediately after tensor-info records as header_end.
   Compute aligned_header_end = align_up(header_end, alignment). A
   tensor-bearing image starts data at that aligned position. A tensor-free
   image is accepted only when its total length is exactly header_end or
   exactly aligned_header_end; its data_start is the total length.
9. Convert data_start to host usize, require the header-to-data bytes to be
   present and all zero, and borrow the remaining bytes as data.
10. Materialize each raw descriptor. Compute its encoded length, check the
    relative end against data.len(), and check data_start + data_offset for
    absolute overflow.
11. Sort temporary tensor references by relative offset and validate nonoverlap,
    alignment, zero inter-tensor padding, and an exact or aligned terminal end.
    Return the archive only after terminal padding is also all zero.

All vectors are allocated only after count and host-address checks. Metadata and
tensor descriptor order remains the order in the input, while span validation
uses a temporary sorted reference list and does not reorder the returned
archive.

## Span, alignment, and padding invariants

metadata_alignment searches the already unique metadata list for the exact key
general.alignment. It does not infer alignment from tensor offsets or from the
file length. Missing metadata uses 32. A present value with any type other
than U32, zero, or not a multiple of eight returns InvalidAlignment.

For a tensor-bearing image, data_start is the aligned header end even if the
first tensor has a later offset. The gap from header_end to data_start must be
present and zero. Each tensor offset is relative to data_start, must be
aligned, and must satisfy offset + encoded_bytes <= data.len(). The span
validator sorts by offset and maintains a cursor at the previous end:

* offset < cursor is OverlappingTensor.
* offset >= cursor is allowed, including adjacent spans and zero-byte spans.
* Bytes from cursor to offset must be zero.
* After the last span, data.len() must equal the last end or
  align_up(last_end, alignment).
* Any terminal bytes between the last end and data.len() must be zero.

For an image with no tensors, arbitrary bytes after the header are not allowed.
Only an unpadded header or the exact zero-padded aligned header is valid. This
keeps the empty archive's data section empty and makes trailing data an
observable format error.

The same padding rules are checked after sorting even though each descriptor's
offset was checked once during tensor-info parsing. The duplicate check keeps
the materialized-tensor invariant local and ensures helper changes cannot
silently bypass it.

## Error boundary

GgufErrorKind is a public non_exhaustive enum. GgufError contains public kind
and detail fields, plus kind() and detail() accessors. Display renders
Debug(kind): detail, and the type implements std::error::Error.
GgufResult<T> is an alias for Result<T, GgufError>. The parser never substitutes
an empty value, skips a bad descriptor, or returns a partial archive.

| Kind | Failing boundary |
| --- | --- |
| InvalidLimit | Any GgufLimits::new argument is zero. |
| FileLimitExceeded | The input length exceeds limits.file_bytes. |
| Truncated | A required fixed field, string, metadata value, header pad, data start, or declared section is outside the input. |
| InvalidMagic | The first four bytes are not GGUF. Fewer than four bytes is Truncated instead. |
| UnsupportedVersion | Version bytes decode to neither little-endian 2 or 3 nor big-endian 3. Big-endian 2 has the separate UnsupportedEndian kind. |
| UnsupportedEndian | The bytes identify big-endian version 2, which this reader does not support. |
| MetadataLimitExceeded | Declared metadata pair count is greater than the caller bound. |
| TensorLimitExceeded | Declared tensor count is greater than the caller bound. |
| RankLimitExceeded | A rank is at most four but greater than limits.rank. |
| StringLimitExceeded | A key, name, or metadata string exceeds its per-value bound or the aggregate string budget. |
| ArrayLimitExceeded | An array length exceeds the aggregate remaining array-element budget. |
| ArrayDepthExceeded | A nested array depth exceeds limits.array_depth. |
| InvalidUtf8 | A length-prefixed key, name, or metadata string is not UTF-8. |
| InvalidMetadataKey | A metadata key is empty or exceeds 65,535 bytes. |
| DuplicateMetadata | A metadata key repeats. |
| UnsupportedMetadataType | A metadata type code is unknown, or an internal scalar decoder receives Array. |
| InvalidBoolean | A boolean value byte is neither zero nor one. |
| InvalidAlignment | general.alignment is not U32, zero, or not a multiple of eight. |
| InvalidTensorName | A tensor name exceeds 64 bytes. Empty names are not rejected by this module. |
| DuplicateTensor | A tensor name repeats. |
| UnsupportedTensorType | A tensor type code is unknown or removed. |
| InvalidDimension | Rank exceeds four, or the first dimension is not divisible by its block width. |
| InvalidOffset | A tensor offset is unaligned, a tensor end is outside data, or a padding range is outside data. |
| OverlappingTensor | A sorted tensor starts before the previous tensor's end. |
| NonZeroPadding | Header, inter-tensor, or terminal padding contains a nonzero byte. |
| TrailingData | A tensor-free image has bytes beyond its permitted header, or data extends beyond the exact or aligned tensor end. |
| ArithmeticOverflow | A count, array minimum, dimension product, encoded length, alignment, absolute offset, host conversion, or byte range cannot be represented. |

The distinction between InvalidDimension and RankLimitExceeded is intentional:
a rank above the current format maximum is invalid independent of caller
policy, while a current rank above the caller's bound is a resource policy
failure. Likewise, malformed padding is not folded into InvalidOffset;
NonZeroPadding identifies bytes that violate the zero-fill rule.

## Direct consumers

### Dataset ingestion

ingest/src/dataset.rs::parse_gguf_tables chooses GGUF from the source
extension, constructs limits from the input length, and calls parse_gguf.
After success it exposes up to two logical tables:

* metadata has one row per metadata entry with the key bytes, the numeric
  metadata type code, and a human-readable value. F32 and F64 values are
  rendered as hexadecimal bit patterns, strings retain their UTF-8 bytes, and
  arrays use their debug representation.
* tensors has one row per descriptor with name, type code, dimensions joined by
  x, rank, encoded byte count, and the validated raw tensor bytes in a binary
  column.

If both sections are empty, ingestion emits one binary table carrying
the complete source bytes. A GgufError becomes a
DatasetSourceErrorKind::MalformedFormat tied to the logical source path. The
raw_tensor(name).unwrap_or_default() fallback is unreachable for a successful
archive because span validation proved every returned range, but the dataset
mapper still keeps a binary column type.

### Structural OGDL conversion

ingest/src/gguf_ogdl.rs::gguf_to_structural_ogdl calls parse_gguf, requires
version 3, and walks metadata and tensors in archive order. It obtains each
payload through raw_tensor and structurally emits its fields, never an opaque
byte or base64 fallback. Its seekable streaming counterpart has a separate
bounded first-pass reader because it must avoid retaining the complete image;
it mirrors this module's layout and padding checks but does not call
parse_gguf.

The in-memory structural_ogdl_to_gguf path writes a complete image and calls
parse_gguf on the generated bytes before returning. That final parse is the
round-trip boundary for alignment, spans, encoded lengths, and padding. The
streaming reverse path uses its own inspect_stream_gguf validation for the same
reason and has no in-memory archive.

### Dense-F32 llama execution

training/src/gguf_llama.rs::decode_gguf_llama calls parse_gguf and maps every
parser failure to GgufLlamaErrorKind::Container. It then applies model
semantics:

* version must be 3 and endian must be little;
* general.architecture must be the exact string llama;
* required numeric metadata must be present as U32, optional metadata must have
  its declared type, and the supported geometry must be nonzero;
* query and key/value head counts must match, RoPE must cover an even full
  head, and unsupported MoE, noncausal, grouped-query, and other llama
  variants fail closed; and
* ArtifactTensorBuilder captures required and optional named tensors only when
  they are F32 with exact expected dimensions. Any unconsumed tensor is an
  unsupported variant, and any missing required tensor is a missing-tensor
  error.

The builder copies the validated raw bytes into owned
GgufLlamaTensorImage values and reverses the descriptor dimensions for the
execution representation. This is the first consumer that interprets tensor
payloads; gguf.rs itself remains format-only.

load_gguf_llama_model_file first reads a bounded immutable SourceSnapshot, then
calls decode_gguf_llama. Root src/inference.rs selects this loader for a .gguf
model and derives GgufLimits from the file length. The public inference path
therefore crosses filesystem bounds, this parser, architecture validation,
target-free token preparation, and native execution in that order.

## Invariants and non-goals

The following facts hold for every successful GgufArchive:

1. All borrowed strings are valid UTF-8 and point into the caller's input.
2. Metadata keys and tensor names are unique, and metadata keys are nonempty.
3. Version, endian, counts, ranks, type codes, and alignment are known and
   bounded.
4. Every tensor's dimensions have a checked encoded length, every relative span
   lies in archive.data(), and every absolute file offset was checked.
5. Tensor spans do not overlap. All alignment gaps and terminal bytes are zero,
   and no unvalidated trailing bytes remain.
6. Metadata and tensor vectors preserve source order. The archive does not
   reorder or normalize names, dimensions, numeric values, or payload bytes.
7. The archive's lifetime cannot outlive the input slice, and no parser result
   owns a second copy of the source image.

The module does not:

* read a path, seek a file, or perform I/O;
* choose a model architecture or a supported execution graph;
* tokenize text, sample output, create KV state, or run native code;
* convert quantized, half, integer, or floating tensor bytes to F32;
* infer missing metadata, repair alignment, ignore unknown type codes, or
  tolerate nonzero padding; or
* promise that a valid container is executable by any downstream consumer.

Use parse_gguf when the complete byte slice can be bounded and borrowed. Use
the seekable stream functions in gguf_ogdl.rs when conversion must retain only
descriptors and bounded payload chunks. In both cases, architecture and
payload semantics belong to the consumer that has evidence for them.

## Function and type index

The implementation's public and boundary-relevant symbols are:

| Symbol | Responsibility |
| --- | --- |
| GgufEndian | Records the byte order selected from the version field. |
| GgufLimits::new and accessors | Constructs and exposes nonzero caller bounds. |
| GgufMetadataType::parse and code | Maps metadata codes 0 through 12 in both directions. |
| GgufMetadataArray::element_type and values | Exposes homogeneous array type and borrowed values. |
| GgufMetadataValue::value_type | Reports the encoded variant without numeric conversion. |
| GgufMetadataEntry::key and value | Exposes one borrowed metadata pair. |
| GgufTensorType::parse, code, block_elements, block_bytes | Maps tensor codes and proves encoded block geometry. |
| GgufTensor accessors | Exposes validated names, dimensions, type, offsets, and span size. |
| GgufArchive accessors | Exposes validated header state, data section, ordered metadata, and ordered tensors. |
| GgufArchive::metadata_entry and tensor | Performs exact lookup by unique key or name. |
| GgufArchive::raw_tensor | Returns the validated encoded byte span from the data section. |
| GgufError and GgufErrorKind | Carries the fail-closed error category and detail. |
| parse_gguf | Runs the complete bounded parse and returns a borrowing archive. |
| Reader::take and scalar reads | Performs checked host slicing and endian-aware field decoding. |
| Reader::read_string and read_metadata_value | Enforces string and array budgets while retaining typed values. |
| materialize_tensors and tensor_encoded_bytes | Computes checked spans from tensor descriptors. |
| validate_tensor_spans | Proves nonoverlap, padding, alignment, and terminal length. |
| metadata_alignment and detect_version | Derives the two format-wide header decisions. |
| align_up, bounded_usize, require_available_bytes | Centralizes checked arithmetic and host-address conversion. |

This index describes the implementation in ingest/src/gguf.rs. It does not
replace the normative GGUF specification or the additional consumer contracts
documented by the callers above.
