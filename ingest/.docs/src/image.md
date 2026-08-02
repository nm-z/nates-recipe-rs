# Image ingestion, header inspection, and init-image packing

This page documents the two image-shaped boundaries owned by `ingest`:

1. encoded image files become one lossless `RawTable` value, are classified as
   `SemanticType::Image`, have their container headers inspected, and remain
   variable-width encoded bytes during preparation; and
2. graph-level external values become one complete zero-initialized init image
   per device through `pack_init_images`.

These are different operations. Image columns are source data. An init image
is a planner-produced memory admission image. Neither operation decodes pixels,
resizes images, normalizes channels, or performs CPU-side payload calculation.
`ImageValueLayout::EncodedFile` and `ImageValueRange::EncodedBytes` are the
explicit contract for retained image values.

Source references in this page use the current implementation files:

- `ingest/src/image.rs`: `ExternalValue`, `PackedInitImage`, and
  `pack_init_images`.
- `ingest/src/image_header.rs`: signature recognition and full encoded-header
  inspection.
- `ingest/src/dataset.rs`: source traversal and image-to-table admission.
- `ingest/src/semantic.rs`: image classification and numeric-parser ordering.
- `ingest/src/prepare.rs`: fitted image metadata and variable-width storage.
- `ingest/src/table.rs` and `ingest/src/source.rs`: source framing and byte
  bounds.

## End-to-end image-column path

The production path is preparation-only and ends before the runtime loop:

```text
regular file or archive member
  -> bounded SourceSnapshot
  -> dataset leaf dispatch
  -> one image LogicalTable / RawTable value
  -> semantic rule or signature classification
  -> train-partition header inspection
  -> retained-row header validation
  -> VariableWidthVector of original encoded bytes
```

The file is read once, closed, and retained in memory before this path runs.
The resulting vector is not a decoded pixel tensor. A later model compiler may
choose an operation that understands bytes, but `ingest` does not invent that
operation.

### Source dispatch and `RawTable` consumers

`distill_dataset` and `distill_datasets` in `dataset.rs` read regular files via
`read_source_snapshot` (`dataset.rs:658-663`). Directory traversal is sorted,
ZIP members are sorted by enclosed path, symbolic links are refused, and
archive nesting is bounded at `ARCHIVE_DEPTH_LIMIT = 32`.

`visit_bytes` admits the complete leaf first, then calls `parse_leaf`
(`dataset.rs:679-709`). The image cases are deliberately small:

| Dispatch case | Condition | Result |
| --- | --- | --- |
| `.png` | extension is `png` and bytes start with the PNG signature | one `LogicalTable` with column `image`, exact `Image` / `Bytes` semantics, and one value containing all source bytes |
| any other extension | `has_recognized_image_signature(bytes)` is true | the same one-column, one-row image table |
| otherwise | no recognized image signature | normal JSON, delimited, text, GGUF, safetensors, or binary dispatch |

The `.png` branch uses only `is_png`, which checks the eight-byte signature.
It does not inspect the IHDR there. A truncated or malformed PNG that still has
the signature can therefore enter a `RawTable`; full header validation happens
when image preparation calls `inspect_encoded_image`.

`single_payload` (`dataset.rs:952-963`) creates:

- one header, `image`;
- one row containing an owned copy of the complete encoded file; and
- an exact semantic rule `(SemanticType::Image, VectorEncoding::Bytes)`.

The `Accumulator` merges this logical table with other files. With multiple
declared sources, it may add fourteen source-context columns (`source_index`,
`source_path`, `format`, `content_sha256`, `file_bytes`, `sample_index`, and
others). Image bytes themselves remain one opaque field. Every appended value
is checked by the dataset-level `check_field_bound` (`dataset.rs:1730-1758`):
an image field must fit the source-byte bound; the smaller textual field bound
is applied only when the value is valid UTF-8. Thus an opaque binary image is
not rejected merely because it is larger than `IngestLimits::field_bytes`.

`RawTable` is still only framing (`table.rs:85-131`). Its rows and headers are
byte vectors. `parse_table` never recognizes or decodes images; it is the
delimited-table route used when a source is selected as CSV/TSV/whitespace
data. Image dispatch reaches the same `RawTable` shape through
`LogicalTable::new`, so rectangularity remains the table invariant.

### Semantic classification and numeric precedence

`infer_table_vectors_with_semantics` first verifies that every row has the
table width (`semantic.rs:267-283`). For each column, the default classifier
uses this exact order (`semantic.rs:320-353`):

1. every nonmissing value has a recognized image signature ->
   `SemanticType::Image` and `VectorEncoding::Bytes`;
2. every nonmissing value is a valid temporal instant -> relative seconds
   `i32`;
3. an ordinal vocabulary is recognized -> ordinal `i32`;
4. every nonmissing UTF-8 value passes `parse_contract_i32` -> numeric `i32`;
5. every nonmissing UTF-8 value passes `parse_contract_f32` -> numeric `f32`;
6. the configured ambiguous-vector model chooses categorical, text, numeric,
   or another remaining semantic type.

An exact semantic rule supplied by dataset distillation wins over this
classifier (`semantic.rs:292-303`). Consequently, the one value emitted by an
image file remains `Image` / `Bytes` even if a future signature-only change
would otherwise classify it differently. Generic tables without an exact rule
are image columns only when all present values have recognized signatures.
Missing values are ignored for the `all` checks. A column with no present
values is sent to the ambiguous model unless its source supplied an exact
rule.

The numeric parsers are therefore consumers of the same raw table, not image
decoders. `parse_contract_i32` and `parse_contract_f32` are reached only after
the image, temporal, and ordinal checks. If a column contains mixed image and
non-image bytes, it does not satisfy the image `all` check and may be treated
as numeric, text, categorical, or binary according to the remaining rules.
There is no implicit conversion from an image byte field to f32 or i32.

### Header metadata types

`image_header.rs` exposes the metadata value that preparation and checkpoint
consumers retain:

| Type | Values and contract |
| --- | --- |
| `EncodedImageFormat` | `Png`, `Jpeg`, `Gif87a`, `Gif89a`, `Bmp`, `WebP` |
| `ImageColorModel` | `Grayscale`, `GrayscaleAlpha`, `Rgb`, `Rgba`, `Bgr`, `IndexedRgb`, `YCbCr`, `Cmyk`, `Ycck` |
| `ImageValueLayout` | only `EncodedFile`, meaning original container bytes are retained |
| `ImageValueRange` | only `EncodedBytes`, meaning no pixel numeric range is claimed |
| `EncodedImageMetadata` | format, nonzero dimensions, optional channel count, optional color model, optional sample-bit width, and the two fixed value contracts |

`EncodedImageMetadata` fields are private and read through the value-returning
accessors `format`, `width`, `height`, `channels`, `color_model`,
`sample_bits`, `value_layout`, and `value_range` (`image_header.rs:49-84`). It
derives `Ord`; preparation uses that to canonicalize a set of mixed image
variants.

### Signature recognition versus full inspection

`has_recognized_image_signature` (`image_header.rs:121-128`) is a cheap boolean
used by dataset and semantic routing. It recognizes only these prefixes:

| Format | Signature test |
| --- | --- |
| PNG | `89 50 4e 47 0d 0a 1a 0a` |
| JPEG | `ff d8 ff` |
| GIF | `GIF87a` or `GIF89a` |
| BMP | `BM` |
| WebP | `RIFF` at bytes `0..4` and `WEBP` at bytes `8..12` |

The boolean does not prove that a complete header or payload exists. The
crate-private `inspect_encoded_image` performs the full format-specific checks
below and returns `ImageHeaderError { detail }` on the first failure. The error
type is intentionally not re-exported. Preparation attaches its detail to a
public `PrepareError` with column and source-row context.

### Format-specific header operations and bounds

All numeric reads use checked slices. All dimensions must be nonzero through
`require_dimensions` unless the format's bit-packed dimensions are inherently
positive after adding one. Header inspection reads no compressed scan data and
does not verify that a complete image can be rendered.

#### PNG (`inspect_png`, `image_header.rs:150-205`)

- Requires at least 33 bytes, a length field of exactly 13, and chunk type
  `IHDR` immediately after the eight-byte signature.
- Reads big-endian width and height. Both are nonzero and each is at most
  `i32::MAX`, the format's accepted 31-bit bound.
- Maps color type to channels and model: `0` = one grayscale channel,
  `2` = three RGB, `3` = one indexed-RGB, `4` = two grayscale-alpha, and
  `6` = four RGBA. Other values are rejected as reserved.
- Accepts sample bits `{1,2,4,8,16}` for grayscale, `{8,16}` for RGB,
  grayscale-alpha, and RGBA, and `{1,2,4,8}` for indexed RGB.
- Requires compression and filter methods to be zero and interlace method to
  be zero or one. The IHDR CRC is recomputed with `png_crc32` and must match.
- Returns `EncodedImageFormat::Png` and the corresponding metadata. It does
  not inspect palette, IDAT, IEND, or the declared file length beyond the IHDR
  slice.

#### GIF (`inspect_gif`, `image_header.rs:207-241`)

- Requires the 13-byte logical screen descriptor and preserves the exact
  version as `Gif87a` or `Gif89a`.
- Width and height are little-endian `u16`, promoted to `u32`, and must be
  nonzero.
- GIF is represented as one indexed-RGB channel. If the global-color-table
  flag is set, table bits are `(packed & 0x07) + 1`, so the table has
  `2^table_bits` colors and `3 * colors` bytes. The complete table must be
  present. Those table bits become `sample_bits`; with no global table,
  `sample_bits` is `None`.
- Local color tables, image descriptors, extension blocks, and the trailer are
  not decoded or required by this header operation.

#### JPEG (`inspect_jpeg` and `jpeg_frame_metadata`, `image_header.rs:243-364`)

- Requires at least four bytes and scans marker segments after the SOI prefix.
  Every segment starts with `0xff`; repeated fill bytes are accepted.
- Standalone SOI, TEM, and restart markers have no length. A stuffed `0x00`,
  EOI before a frame, or SOS before a frame is rejected. Other segments require
  a big-endian length of at least two and a checked in-bounds payload.
- APP0 `JFIF\0` and APP14 `Adobe` transform bytes are remembered. The first
  start-of-frame marker in the accepted set (`C0`, `C1`, `C2`, `C3`, `C5`,
  `C6`, `C7`, `C9`, `CA`, `CB`, `CD`, `CE`, `CF`) supplies the frame metadata.
- The frame requires six bytes, nonzero sample precision, nonzero `u16`
  width and height, a nonzero component count, and a complete three-byte
  component table for every component. Sample precision is preserved as the
  nonzero `u8` header value; this parser does not impose an additional JPEG
  precision ceiling.
- One component is grayscale. Three components are RGB when Adobe transform
  is zero or IDs spell `RGB`; otherwise they are YCbCr when Adobe transform is
  one, JFIF is present, or IDs are `[1,2,3]`. Four components are YCCK for
  transform two and CMYK for transform zero or IDs `CMYK`. Other component
  counts or ambiguous IDs retain `color_model = None` while preserving the
  channel count.
- A file with no frame marker is rejected. Scan entropy bytes and the final
  EOI are intentionally outside this header-only contract.

#### BMP (`inspect_bmp`, `image_header.rs:366-448`)

- Requires the 14-byte file header plus at least the first four-byte DIB-size
  field. DIB size must be at least 12 and its complete checked range must be
  present.
- A 12-byte DIB reads unsigned little-endian `u16` width, height, planes, and
  bits per pixel. DIB sizes of at least 16 read four-byte dimensions and the
  planes/bits fields. Windows DIB sizes `40`, `52`, `56`, `108`, and `124`
  interpret width as a positive `i32` and height as a signed `i32`; the absolute
  height is used so top-down images are accepted. `i32::MIN` cannot be
  absolute-valued and is rejected.
- Dimensions must be nonzero, planes must equal one, and bits per pixel must
  be nonzero. Bits-per-pixel maps to indexed RGB for `1`, `2`, `4`, or `8`
  bits; BGR with three channels and eight sample bits for `24`; and BGR with
  three channels and sixteen sample bits for `48`. Other widths preserve
  `channels = None`, `color_model = None`, and `sample_bits = None` rather
  than claiming a false layout.
- Pixel offset, palette, compression, row stride, and pixel payload are not
  inspected.

#### WebP (`inspect_webp` and its VP8 variants, `image_header.rs:450-596`)

- Requires a 12-byte RIFF header. The little-endian RIFF size is converted to
  `usize`, checked for overflow, must be at least four, and must end within the
  supplied bytes. Chunks are walked only inside that declared RIFF range.
- Every chunk needs an eight-byte header and an in-range payload. Chunk size
  addition, odd-byte padding, and the next cursor are checked. `ALPH` sets a
  separate-alpha flag for a later lossy `VP8 ` chunk; unknown chunks are
  skipped. The first dimension-bearing `VP8X`, `VP8L`, or `VP8 ` chunk returns
  metadata. No such chunk is an error.
- `VP8X` requires ten payload bytes and zero reserved bytes. Three little-endian
  24-bit dimensions are incremented by one, giving a positive range up to
  `2^24`. Alpha flag `0x10` selects four-channel RGBA, otherwise three-channel
  RGB. Sample bits are eight.
- `VP8L` requires five bytes, signature byte `0x2f`, and zero version bits in
  `header[4] & 0xe0`. Its bit-packed width and height are each incremented by
  one. Alpha flag `0x10` selects RGBA or RGB, with eight sample bits.
- Lossy `VP8 ` requires a ten-byte key-frame header, clear frame bit, and the
  start code `9d 01 2a`. Width and height are 14-bit little-endian values and
  must be nonzero. Separate alpha selects RGBA; otherwise the model is YCbCr,
  with eight sample bits.

The read helpers (`image_header.rs:619-652`) return a truncation error instead
of indexing outside the supplied slice. `checked_add`, `saturating_add` in
slice-end calculation, and explicit `usize` conversions keep header offsets
bounded on both 32-bit and 64-bit hosts.

## Preparation representation

### Fit metadata on the train partition

`prepare_table_with_semantics` selects columns and rows, computes an exact
rational train prefix, builds a fit-only table, and infers semantics before
applying any schema (`prepare.rs:813-838`). Rows excluded by predicates never
enter either fit or retained partitions. The fit partition is the only source
of learned image metadata.

`fit_vector_schema` handles an image tuple only when the inferred encoding is
`VectorEncoding::Bytes` and the semantic type is `SemanticType::Image`
(`prepare.rs:1536-1596`):

```text
fit nonmissing rows
  -> inspect_encoded_image for each value
  -> collect BTreeSet<EncodedImageMetadata>
  -> sort into VectorMetadata::Image { encoded_variants }
```

The set is exact and deterministic. Different formats, dimensions, channel
models, or sample widths remain separate entries. Empty fit values are skipped.
No validation-only variant from the validation partition is added to fitted
metadata. A malformed nonempty fit value returns `PrepareErrorKind::EncodingFailure`
with its source row and column.

### Apply the fitted schema to all retained rows

`apply_vector_schema` retrieves values by source row and column, then selects
the encoding operation (`prepare.rs:1598-1702`). For
`(SemanticType::Image, VectorEncoding::Bytes, VectorMetadata::Image { .. })` it:

1. calls `inspect_image_variants_for_schema` for every nonempty retained value,
   including validation rows;
2. stores the original bytes with `encode_variable`; and
3. leaves `categorical_observations` absent.

Applying a schema validates headers but does not mutate its fitted variant set.
Thus a valid validation image may have a format or shape not observed during
training, while an invalid header fails the same `EncodingFailure` route.

`encode_variable` (`prepare.rs:2005-2034`) appends each raw value to one payload,
pushes a `u64` end offset, and records `valid = !value.is_empty()`. UTF-8 is
checked only for `VectorEncoding::Utf8`; image bytes are intentionally not
required to be UTF-8. A checked conversion of every payload length to `u64`
returns `PrepareErrorKind::ArithmeticOverflow` if it cannot be represented.

### Public prepared types and invariants

| Type | Image-path meaning and invariant |
| --- | --- |
| `VectorSchema` | row-free identity: source index/name, `Feature` or `Target` role, `SemanticType::Image`, `VectorEncoding::Bytes`, and `VectorMetadata::Image` |
| `PreparedValues::VariableWidth` | the only storage variant used for images; no scalar `DType` exists because `VectorEncoding::Bytes::dtype()` is `None` |
| `VariableWidthVector` | `offsets` has one more entry than rows, starts at zero, is monotonic as constructed, and indexes the concatenated `payload`; `valid` has one entry per retained row |
| `VariableWidthVector::value(row)` | returns outer `None` for an out-of-range row or unrepresentable offset, inner `None` for a missing/empty row, and `Some(Some(bytes))` for a nonempty image; construction is responsible for keeping valid offsets inside `payload` |
| `PreparedVector` | preserves source index and role while carrying the immutable schema and row-aligned values |
| `PreparedPartition` | stores retained positions plus original source-row identities; image bytes are aligned to these retained positions |
| `VectorMetadata::Image` | fitted, sorted, distinct header facts only, not decoded pixels or a pixel tensor |

`PreparedDataset::fixed_dense_matrix` rejects any variable-width vector before
matrix construction (`prepare.rs:611-649`). Images therefore produce
`PrepareErrorKind::VariableWidthDenseMatrix`; they cannot be silently padded,
flattened, cast to f32, or mixed with scalar columns. The dense path also
rejects missing values and lossy integer-to-f32 conversions, but those numeric
rules do not transform image bytes.

### Image preparation errors

The header error is private, but its detail is wrapped as follows:

```text
ImageHeaderError(detail)
  -> PrepareErrorKind::EncodingFailure
  -> "lossless Bytes encoding failed: ..."
  -> column = source header, source_row = offending source row
```

Other failures reachable around an image column are:

- `SemanticInference` or `InconsistentInference` for a nonrectangular table or
  a supplied semantic list that does not describe the source columns;
- `NoRetainedRows`, `NoFeatureVectors`, target-selection errors, and predicate
  errors before image fitting;
- `InconsistentPreparedVector` if an image schema is paired with incompatible
  metadata or loses row alignment; and
- `ArithmeticOverflow` if row, payload-offset, or dense-capacity arithmetic
  cannot be represented.

`ImageHeaderError` itself reports concrete causes such as unrecognized
signature, truncated format-specific headers, zero dimensions, invalid PNG
color/sample combinations, bad CRC, malformed JPEG marker/frame tables,
invalid BMP DIB dimensions, and invalid WebP chunk or codec headers. A
signature-only routing success is not an inspection success.

## Source and table bounds

The image path is bounded before header inspection:

| Bound | Owner | Effect on images |
| --- | --- | --- |
| `IngestLimits::source_bytes` | `table.rs` and `dataset.rs` | one regular file, ZIP member, aggregate leaf bytes, and each opaque image field must fit this nonzero `u64` bound |
| `IngestLimits::records` | table parser and `Accumulator` | aggregate image samples cannot exceed this nonzero `u64` bound; an empty logical table is represented by one empty sample when appended |
| `IngestLimits::fields_per_record` | table parser and accumulated headers | image payload contributes one vector; source-context columns and merged files count toward the same nonzero `u32` bound |
| `IngestLimits::field_bytes` | delimited framing and textual distilled values | image bytes are checked against this only when they are valid UTF-8; opaque encoded bytes use the source bound |
| `ARCHIVE_DEPTH_LIMIT` | recursive dataset traversal | nested ZIP image members fail at 32 levels before expansion |

`read_table`/`read_bounded` and `read_source_snapshot` read at most
`limit + 1` bytes, detect a file that grows beyond the limit, and return no
partial source. ZIP members use the same `source_bytes + 1` read cap. Header
inspection adds no independent image-payload allocation and never reads from a
filesystem handle.

## Graph-level init image packing (`ingest/src/image.rs`)

This path is independent of encoded image files. It packs arbitrary graph
external inputs, which may happen to be bytes from an image column, into the
planner's `InitDataImage` manifests.

### Types

`ExternalValue<'a>` is a borrowed pair:

```rust
pub struct ExternalValue<'a> {
    pub logical: ValueId,
    pub bytes: &'a [u8],
}
```

`ExternalValue::new` is a `const` constructor. The packer borrows source slices
only while constructing output; every `PackedInitImage` owns a complete
`Vec<u8>`.

`PackedInitImage` has private fields and read/move accessors:

- `device() -> DeviceId` identifies the target device;
- `image() -> ValueId` identifies the resident value spanning the packed image;
- `bytes() -> &[u8]` borrows the complete upload; and
- `into_bytes() -> Vec<u8>` transfers ownership of the upload.

`InitDataImage` and `InitDataImageMember` are planner/core types
(`core/src/schedule.rs:306-328`):

- one manifest has `device`, resident `image`, total `bytes`, and members;
- a member has logical graph identity, candidate-specific `physical` identity,
  `dtype`, exact `bytes`, and `image_offset`; and
- member offsets are relative to the beginning of the device image.

Both calculation dtypes currently have a four-byte scalar width
(`core/src/scalar.rs:9-23`), but the packer uses `DType::byte_width()` rather
than embedding that policy.

### `pack_init_images` algorithm

`pack_init_images(manifests, sources)` (`image.rs:90-179`) is fail-closed and
returns no partial vector:

1. `index_sources` inserts each `ExternalValue` into a `BTreeMap<ValueId,
   &[u8]>`. Repeated logical IDs immediately return `DuplicateSource`.
2. `validate_manifests` checks every manifest and builds one global logical
   contract map of `(DType, ByteCount)`.
3. Every supplied source must occur in that map, otherwise `UnexpectedSource`.
4. Every required logical member must have a supplied source, otherwise
   `MissingSource`.
5. Each supplied slice length is converted to `u64` and must equal the
   manifest contract, otherwise `SourceSizeMismatch`.
6. For every manifest, allocate exactly `manifest.bytes` host bytes initialized
   to zero. Copy the source for each member into
   `[image_offset, image_offset + source.len())`.
7. Return all complete images sorted by `DeviceId`, independent of manifest
   input order.

The zero fill is intentional. Unoccupied gaps, including planner-reserved
device fault flags, remain zero. A logical source appearing in several device
manifests is copied independently into every resident image. The source slice
itself is never mutated or retained in the result.

### Manifest invariants checked by `ingest`

`validate_manifests` (`image.rs:197-278`) enforces:

| Invariant | Failure |
| --- | --- |
| at most one manifest per `DeviceId` | `DuplicateDevice` |
| total image byte count is nonzero | `InvalidManifest` |
| no repeated logical member in one manifest | `DuplicateMember` |
| no repeated physical member in one manifest | `DuplicateMember` |
| every member offset is a multiple of `member.dtype.byte_width()` | `InvalidManifest` |
| `image_offset + member.bytes` does not overflow `ByteCount` | `ArithmeticOverflow` |
| member end is at or before manifest total bytes | `InvalidManifest` |
| member ranges in one manifest do not overlap | `InvalidManifest` |
| replicated logical IDs have identical dtype and byte contracts | `ConflictingContract` |

The function permits a nonempty image with zero members, which produces an
all-zero image. It also permits zero-byte members because this module checks
range and alignment rather than inventing a nonzero-value policy. The finalized
plan validator is stricter about the complete graph contract: it checks device
and value references, one required image for every topology device, physical
value contracts, arena bindings, producer identity, and physical placement
(`core/src/plan.rs:1601-1825`).

The per-manifest physical-member set in `ingest/src/image.rs` is local to each
manifest. Cross-device physical identity uniqueness and all planner-level
producer/binding constraints belong to core plan validation, not this byte
packer.

### `ImagePackError` index

`ImagePackError` is public and contains a non-exhaustive `ImagePackErrorKind`
plus a human-readable `detail`. `Display` renders `"{kind:?}: {detail}"`.
The exact construction sites are:

| Kind | Trigger |
| --- | --- |
| `DuplicateSource` | two `ExternalValue`s use one logical ID |
| `MissingSource` | a manifest logical member has no admitted payload |
| `UnexpectedSource` | a supplied logical ID is absent from all manifests |
| `SourceSizeMismatch` | a payload slice length differs from the replicated member byte contract |
| `DuplicateDevice` | more than one manifest names a device |
| `DuplicateMember` | a manifest repeats a logical or physical member |
| `ConflictingContract` | one logical ID is replicated with different dtype or size |
| `InvalidManifest` | zero image, misalignment, out-of-range member, overlap, or defensive destination-slice failure |
| `ArithmeticOverflow` | `u64`/`usize` conversion, checked member-end, host-range addition, or image allocation size cannot be represented |

Validation is performed before the first `PackedInitImage` is returned. An
error after a later manifest has been checked still discards the local vector;
callers never receive a prefix of valid images.

## Runtime consumers and related packers

The low-level `ingest::pack_init_images` function is exported by `ingest/src/lib.rs`
but the current training and inference execution boundary has typed packers in
`training/src/execute.rs`:

- `pack_device_images` (`training/src/execute.rs:3077-3192`) maps
  `OwnedExternalInput` values, checks dtype and exact bytes, rejects missing
  training members, zero-fills gaps, checks sorted range overlap, and returns
  one `DeviceImage` per device.
- `pack_inference_input_images` (`training/src/execute.rs:1963-2074`) performs
  the same typed placement for inference and additionally rejects an input not
  bound by any manifest (`UnboundExternalInput`), because inference admission
  is closed.
- Both execution packers sort their results by device. They are not wrappers
  around `pack_init_images`; they enforce their own typed execution errors.

`executor::validate_images` then requires exactly one supplied admission image
for every finalized manifest, matches image identity and exact byte count, and
rejects duplicate, missing, unexpected, or mismatched device admissions
(`executor/src/executor.rs:2124-2192`). Fault-reset ranges are checked against
the resolved init-image location before upload. CUDA and HSA realization each
require the manifest byte count to fit their pre-realized pinned/fine staging
allocation (`native-executor/src/cuda.rs:1714-1729` and
`native-executor/src/hsa.rs:1849-1872`).

These runtime checks preserve the lifecycle boundary: init admission is the
single external upload, the loop has no file or image ingress, and the packed
bytes become authoritative device state only after the backend accepts the
validated image.

## Persistence and model consumers

Checkpoint decoders copy the public header facts into a checkpoint-local
`CheckpointImageMetadata` rather than reconstructing an encoded payload. They
require `value-layout = encoded-file` and `value-range = encoded-bytes`, parse
the six format names and optional color models, and validate image metadata as
an `Image` / `Bytes` vector (`training/src/checkpoint.rs:414-494,
2550-2587, 5392-5551`). Saved variants must be nonempty, distinct, in canonical
ascending order, have nonzero dimensions, and be producible by the declared
format's channel/model/sample-bit contract. This is persistence validation of
the metadata produced by preparation, not pixel decoding.

The KNN compiler accepts an image target only as a variable-width byte route
(`training/src/knn.rs:314-326`). It builds deterministic byte labels from
`VariableWidthVector`; it does not reinterpret encoded files as numeric pixels.

## Non-goals and review checklist

The implementation intentionally does not:

- decode PNG filters, JPEG entropy, GIF frames, BMP rows, or WebP bitstreams;
- verify complete compressed payloads after the minimum header facts;
- infer a pixel numeric range or convert encoded bytes to f32/i32;
- normalize dimensions, channels, color models, or sample precision;
- impute an empty image value;
- use `field_bytes` as an opaque-image limit when the bytes are not UTF-8; or
- admit an init image without a planner manifest and exact byte contract.

When changing this path, preserve these independently testable contracts:

1. signature routing remains cheaper and weaker than full header inspection;
2. image classification remains ahead of numeric parsing;
3. fitted image metadata comes only from the train partition and remains sorted
   and distinct;
4. all retained image rows receive header validation before raw-byte packing;
5. variable-width offsets, payload, and validity remain row-aligned;
6. fixed dense projection rejects images rather than applying a lossy guess; and
7. init-image validation either returns every complete device image or returns
   one typed error with no partial result.
