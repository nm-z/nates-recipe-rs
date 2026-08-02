# `ingest/src/image_header.rs`

## Source map

The implementation is one module with no submodules. The stable regions are:

| source region | responsibility |
| --- | --- |
| lines `3..12` | `EncodedImageFormat` |
| lines `14..29` | `ImageColorModel` |
| lines `31..58` | value layout/range tags and `EncodedImageMetadata` |
| lines `60..101` | metadata getters and `ImageHeaderError` display |
| lines `103..148` | signature dispatch, shallow signature predicate, metadata constructor |
| lines `150..241` | PNG and GIF parsing |
| lines `243..364` | JPEG marker scan and SOF metadata |
| lines `366..448` | BMP/DIB parsing |
| lines `450..596` | RIFF/WebP chunk scan and VP8 variants |
| lines `598..617` | shared dimension guard and PNG CRC |
| lines `619..652` | checked endian field readers |

The line ranges are descriptive anchors for the current source layout, not a
second contract. Function names and field rules in the sections below are the
authoritative map when implementation lines move.

## Intent

`image_header` is the header-only boundary for encoded image values in the
ingestion crate. It recognizes the image containers that Recipe admits,
checks enough of each container header to establish safe metadata, and returns
the original bytes as an opaque encoded-file value. It never decodes pixels,
inflates compressed payloads, walks a complete image, or claims a pixel numeric
range. A successful result therefore describes the retained file bytes, not a
calculation-ready pixel tensor.

The module has two deliberately different probes:

1. `has_recognized_image_signature` is a cheap signature predicate. It is used
   while choosing a semantic source route and does not prove that the bytes
   contain a valid image header.
2. `inspect_encoded_image` dispatches by the same signatures and performs the
   format-specific header checks. A value that passes the first probe can still
   fail this second probe.

The full parser is fail-closed. It returns one `ImageHeaderError` instead of
constructing partial metadata whenever a required header span is absent, a
declared range overflows, a dimension is invalid, or a format contract is not
met.

## Module surface and data model

The module itself is private in `ingest/src/lib.rs`. The five metadata types
are re-exported from the `recipe-ingest` crate root; the parser functions and
`ImageHeaderError` remain `pub(crate)`.

### Encoded format tags

`EncodedImageFormat` has exactly these tags. GIF version is retained as a
distinct tag because the six-byte signatures are distinct.

| tag | recognized leading bytes |
| --- | --- |
| `Png` | `89 50 4e 47 0d 0a 1a 0a` |
| `Jpeg` | `ff d8 ff` |
| `Gif87a` | ASCII `GIF87a` |
| `Gif89a` | ASCII `GIF89a` |
| `Bmp` | ASCII `BM` |
| `WebP` | `RIFF` at bytes `0..4` and `WEBP` at bytes `8..12` |

All enum and metadata values derive `Clone`, `Copy`, `Debug`, `PartialEq`,
`Eq`, `PartialOrd`, and `Ord`. The ordering is used by the preparation layer
when it canonicalizes a set of observed image variants.

### Color model tags

`ImageColorModel` records a color interpretation only when the header provides
one. `None` in `EncodedImageMetadata::color_model()` means that the header
provided a component count but did not provide an unambiguous interpretation.
The available interpretations are `Grayscale`, `GrayscaleAlpha`, `Rgb`,
`Rgba`, `Bgr`, `IndexedRgb`, `YCbCr`, `Cmyk`, and `Ycck`.

### Retained value contract

`ImageValueLayout::EncodedFile` means the original compressed or
container-encoded file bytes are retained. `ImageValueRange::EncodedBytes`
means no numeric pixel range is asserted. The private `metadata` constructor
sets both tags on every successful result, so there is no successful parser
path for decoded pixels or a decoded sample range.

### `EncodedImageMetadata`

The fields are private and are exposed through `const` getters:

| getter | type | meaning |
| --- | --- | --- |
| `format()` | `EncodedImageFormat` | selected container and, for GIF, version |
| `width()` | `u32` | header-declared nonzero width |
| `height()` | `u32` | header-declared nonzero height |
| `channels()` | `Option<u8>` | component count when the format/header establishes one |
| `color_model()` | `Option<ImageColorModel>` | unambiguous color interpretation, if available |
| `sample_bits()` | `Option<u8>` | per-sample precision when the header establishes one |
| `value_layout()` | `ImageValueLayout` | always `EncodedFile` for parser output |
| `value_range()` | `ImageValueRange` | always `EncodedBytes` for parser output |

Every parser calls `metadata`, so dimensions are nonzero for all successful
results. Format-specific exceptions to a fully populated interpretation are
intentional: JPEG may have an unknown model, GIF may have no global color
table and therefore no sample-bit value, and BMP may accept an unsupported
bits-per-pixel value while leaving channels, color model, and sample bits all
`None`.

`ImageHeaderError` stores one detail `String` and implements `Display` by
writing that detail. It is not a public error type and does not implement
`std::error::Error`. The preparation layer adds the format-independent
`PrepareErrorKind::EncodingFailure`, column, and source-row context.

## Callers and data flow

The relevant call graph is:

```text
distill_dataset / distill_datasets
  -> dataset::parse_member
     -> has_recognized_image_signature (via dataset::is_image)
        -> SourceFormat::Image + exact Image semantic rule

infer_table_vectors
  -> semantic::classify_vector
     -> semantic::is_image_value
        -> has_recognized_image_signature
           -> SemanticType::Image + VectorEncoding::Bytes

prepare_table / prepare_inferred_table
  -> fit_vector_schema
     -> inspect_image_variants
        -> inspect_encoded_image
           -> parse each nonmissing fit value, then deduplicate metadata
  -> apply_vector_schema
     -> inspect_image_variants_for_schema
        -> inspect_encoded_image for every nonmissing retained value
```

* In `dataset.rs`, the generic image route uses only
  `has_recognized_image_signature`. A value with a recognized prefix is
  admitted as one lossless `SourceFormat::Image` payload named `image`; no
  header parser runs during dataset distillation. The `.png` extension route
  has a separate `is_png` prefix check and likewise marks the payload as an
  image. Other extensions reach the generic signature route only after their
  specialized extension cases do not match.
* In `semantic.rs`, `classify_vector` filters empty values and classifies a
  column as `SemanticType::Image` with `VectorEncoding::Bytes` when every
  nonempty value passes the signature predicate. This check precedes temporal,
  ordinal, and numeric parsers. Empty values are ignored for this decision.
  Mixed image formats, dimensions, and models are allowed by this stage. A
  malformed value with a valid signature can therefore be classified as an
  image and is rejected later by preparation.
* In `prepare.rs`, `fit_vector_schema` calls `inspect_image_variants` only for
  an inferred image vector with `VectorEncoding::Bytes`. It skips empty fit
  values, parses every other value, inserts the metadata into a `BTreeSet`,
  and stores the canonical sorted, distinct values in
  `VectorMetadata::Image::encoded_variants`. An invalid header becomes an
  `EncodingFailure` whose detail ends in `invalid encoded image header: {detail}`
  and is prefixed by `lossless Bytes encoding failed:`; it carries
  the inferred column and source row.
* Applying a fitted schema calls `inspect_image_variants_for_schema` on all
  retained rows, including rows outside the fit partition. It validates every
  nonempty header but intentionally does not add validation-only variants to
  fitted metadata. The bytes then go through `encode_variable` and remain
  lossless variable-width payload bytes with offsets; no image decoder is
  involved. Its invalid-header errors use the same `EncodingFailure` detail
  prefix and source-row context as the fit pass.
  `inspect_image_variants` can therefore return an empty variant set when the
  fit partition contains only missing image values. The parser itself has no
  nonempty-set requirement; the checkpoint validators reject an empty saved
  image variant set later.
* `training/src/knn.rs` accepts the prepared tuple
  `SemanticType::Image`/`VectorEncoding::Bytes`/`VectorMetadata::Image` as a
  variable-width byte label. It consumes the retained encoded bytes and does
  not infer a second image representation. Missing image values are excluded
  from the byte-label dictionary, and KNN later rejects a training partition
  with no known references, independently of header metadata.
* Dense feature lowering in `training/src/model.rs` intentionally accepts only
  numeric scalar and categorical one-hot vectors. An image vector used as a
  dense feature remains variable-width and fails with the training compiler's
  dedicated-semantic-lowering error; `ingest/src/inference.rs` likewise has no
  image feature encoding. Image metadata can still be retained in a semantic
  checkpoint, and image targets can use the KNN byte-label path above.
* `training/src/checkpoint.rs` copies each public metadata getter into its
  checkpoint image metadata and writes the format, dimensions, optional
  channels/model/sample bits, `encoded-file`, and `encoded-bytes`. On decode it
  accepts only those two value-contract tags and validates that each saved
  variant could have been produced by this parser. That downstream validator
  also requires a nonempty, distinct, canonical ascending variant list.
* `training/src/knn_checkpoint.rs` uses the same `CheckpointArtifactMetadata`
  and `CheckpointImageMetadata` route for KNN artifacts. Its OGDL encoder
  writes the same fields and its decoder requires the same two value-contract
  tags before the shared saved-vector validation runs. Neither checkpoint
  format reparses the retained image bytes; both persist the metadata facts
  produced during preparation.

The persisted vocabulary is a direct, lossless spelling of these enums:
`png`, `jpeg`, `gif87a`, `gif89a`, `bmp`, and `webp` for formats;
`grayscale`, `grayscale-alpha`, `rgb`, `rgba`, `bgr`, `indexed-rgb`, `y-cb-cr`,
`cmyk`, and `ycck` for color models; `encoded-file` for layout; and
`encoded-bytes` for range. Optional channels, color model, and sample bits use
the scalar `none` token. The parser itself does not parse these textual tokens;
they are the checkpoint boundary's representation of its typed output.

## Signature dispatch

`inspect_encoded_image` tests prefixes in this exact order: PNG, JPEG, GIF
(87a or 89a), BMP, then WebP. `has_recognized_image_signature` repeats the
same predicates. The WebP condition is equivalent to
`bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")` because the
`&&` binds more tightly than `||`.

The signature predicate is intentionally shallow. It does not check minimum
header length, dimensions, CRC, RIFF length, marker structure, chunk structure,
or any reserved field. For example, exactly two bytes `BM`, exactly three
bytes `ff d8 ff`, or a PNG signature with no IHDR all return `true` from the
predicate where the full parser returns a truncation error. A short RIFF input
cannot pass the WebP predicate because `get(8..12)` returns `None`.

An unrecognized prefix causes `inspect_encoded_image` to return the exact detail
`unrecognized encoded image signature`.

## Format parsers

The parsers inspect only the header spans described below. Bytes after the
necessary header or dimension-bearing chunk are not decoded or authenticated.

The minimum accepted input lengths and dimensional field limits provide a
quick cross-format summary:

| format | signature minimum | full-header minimum | dimension representation |
| --- | ---: | ---: | --- |
| PNG | `8` | `33` | nonzero `u32`, each at most `i32::MAX` |
| JPEG | `3` | `4`, then marker/SOF dependent | nonzero big-endian `u16` |
| GIF87a/GIF89a | `6` | `13`, plus a complete global table when flagged | nonzero little-endian `u16` |
| BMP | `2` | `18`, plus the declared DIB | core `u16`, known Windows signed `i32`, or other-DIB `u32` |
| WebP | `12` for the RIFF/form signature | `12`, plus complete declared chunks | VP8 `u14`, VP8L packed `14`-bit, or VP8X `24`-bit minus-one |

The signature minimum is the shortest prefix that makes the shallow predicate
true. It is not a validity guarantee and is intentionally smaller than the
full-header minimum for every format except WebP.

### PNG: `inspect_png`

The parser requires `bytes.get(..33)`, so the input must contain the signature
and a complete 25-byte prefix through the first IHDR CRC. Relative offsets in
that 33-byte view are:

| range | field | encoding |
| --- | --- | --- |
| `0..8` | PNG signature | already selected by dispatch |
| `8..12` | IHDR data length | big-endian `u32`, must equal `13` |
| `12..16` | chunk type | must be `IHDR` |
| `16..20` | width | big-endian `u32` |
| `20..24` | height | big-endian `u32` |
| `24` | sample bit depth | one byte |
| `25` | color type | one byte |
| `26` | compression method | must be `0` |
| `27` | filter method | must be `0` |
| `28` | interlace method | must be `0` or `1` |
| `29..33` | IHDR CRC | big-endian `u32` |

The parser requires nonzero dimensions and then rejects either dimension above
`i32::MAX`, the format's retained 31-bit limit. Color types and allowed sample
bits are:

| IHDR color type | channels | color model | allowed sample bits |
| --- | ---: | --- | --- |
| `0` | `1` | `Grayscale` | `1`, `2`, `4`, `8`, `16` |
| `2` | `3` | `Rgb` | `8`, `16` |
| `3` | `1` | `IndexedRgb` | `1`, `2`, `4`, `8` |
| `4` | `2` | `GrayscaleAlpha` | `8`, `16` |
| `6` | `4` | `Rgba` | `8`, `16` |

Any other color type is reported as `PNG IHDR has reserved color type {n}`.
An allowed color type with a disallowed sample bit is reported as
`PNG color type {n} does not permit {bits}-bit samples`. Nonzero compression or
filter methods, or an interlace value above one, produce
`PNG IHDR has an unsupported reserved method value`.

The CRC is computed by `png_crc32` over `header[12..29]`, namely the `IHDR`
type and its 13 data bytes, and compared with the stored CRC. A mismatch
produces `PNG IHDR CRC does not match its header bytes`. The parser does not
require an `IEND` chunk, inspect IDAT data, or verify that the rest of the file
is a coherent PNG.

Other PNG failures are `truncated PNG IHDR chunk`, or the structural message
`PNG signature is not followed by a 13-byte IHDR chunk` when the length/type
fields do not match.

### GIF: `inspect_gif`

The parser requires the first 13 bytes, consisting of a six-byte version and a
seven-byte logical screen descriptor:

| range | field | encoding |
| --- | --- | --- |
| `0..6` | version | `GIF87a` or `GIF89a` |
| `6..8` | logical width | little-endian `u16`, widened to `u32` |
| `8..10` | logical height | little-endian `u16`, widened to `u32` |
| `10` | packed fields | global color table flag and table-size code |
| `11` | background color index | not inspected |
| `12` | pixel aspect ratio | not inspected |
| `13..` | global color table, when present | RGB triplets |

Dimensions must be nonzero. If bit `0x80` of the packed byte is set, the table
size code is `(packed & 0x07) + 1`, giving `2^table_bits` colors and
`3 * 2^table_bits` table bytes immediately after the 13-byte descriptor. The
size and addition are checked, and `bytes.get(..13 + table_bytes)` must succeed.
The only possible table-size failure on a supported `u8` packed field is still
reported explicitly as `GIF global color table size overflowed usize`; a short
input reports `truncated GIF global color table`. When the flag is clear,
`sample_bits` is `None`; when set it is `Some(table_bits)`, from `1` through
`8`. For GIF this field records the global color-table size code exposed by
the header, not a decoded pixel sample precision.

Successful GIF metadata always reports one channel and `IndexedRgb`, and keeps
`Gif87a` versus `Gif89a`. The parser does not inspect image descriptors, local
tables, extension blocks, trailer bytes, or the actual pixel stream. Failures
are `truncated GIF logical screen descriptor` and, for an impossible internal
version value, `invalid GIF version signature`.

### JPEG: `inspect_jpeg` and `jpeg_frame_metadata`

JPEG dispatch recognizes only `ff d8 ff`; full inspection first requires at
least four bytes. The cursor starts at offset `2`, immediately after the SOI
bytes, and scans marker segments until the first start-of-frame marker. Before
scan data, every marker must be introduced by `0xff`; repeated `0xff` fill
bytes are consumed. Marker `0x00` is rejected as stuffed data in the header.

Standalone markers `0xd8` (SOI), `0x01` (TEM), and `0xd0..=0xd7` (restart)
advance without a length. `0xd9` (EOI) before a frame and `0xda` (SOS) before
a frame are rejected. All other markers read a big-endian `u16` length at the
current cursor. The length includes its own two bytes and must be at least two.
If the length is `L`, the payload range is `cursor + 2 .. cursor + L`; the
cursor advances to `cursor + L`. Checked additions guard both starts and ends.

APP0 payloads beginning with `JFIF\0` set a `jfif` flag. An APP14 payload
beginning with `Adobe` and at least 12 payload bytes records payload byte `11`
as `adobe_transform`; a later matching Adobe segment overwrites the earlier
value. The first marker recognized by `is_jpeg_start_of_frame` is one of
`c0`, `c1`, `c2`, `c3`, `c5`, `c6`, `c7`, `c9`, `ca`, `cb`, `cd`, `ce`, or
`cf`. The frame payload excludes the marker length bytes.

`jpeg_frame_metadata` requires six frame bytes and interprets them as:

| frame payload range | field | encoding |
| --- | --- | --- |
| `0` | sample precision | nonzero `u8` |
| `1..3` | height | big-endian `u16`, widened to `u32` |
| `3..5` | width | big-endian `u16`, widened to `u32` |
| `5` | component count | nonzero `u8` |
| `6..6 + 3 * channels` | component records | only each record's first byte is inspected |

The component table span is checked with `channels * 3` and `6 + ...`. The
first byte of each three-byte record is collected as a component identifier.
Extra bytes in the SOF payload after the component records are ignored; the
parser does not require the segment length to equal the minimum table length.
Color model inference is ordered as follows:

* one channel means `Grayscale`;
* three channels mean `Rgb` when Adobe transform is `0` or IDs are `RGB`;
* otherwise three channels mean `YCbCr` when Adobe transform is `1`, a JFIF
  APP0 marker was seen, or IDs are `[1, 2, 3]`;
* four channels mean `Ycck` when Adobe transform is `2`;
* otherwise four channels mean `Cmyk` when Adobe transform is `0` or IDs are
  `CMYK`;
* every other component count or clue combination leaves the model `None`.

Sample precision is accepted as any nonzero `u8`; it is not restricted to a
JPEG standard precision set. Width and height are bounded by their `u16`
fields and must be nonzero. The parser does not validate sampling factors,
quantization or Huffman tables, entropy-coded data, an EOI marker after the
frame, or the complete JPEG stream.

JPEG failure details are:

* `truncated JPEG marker header` for fewer than four bytes;
* `JPEG header contains data outside a marker segment` when the cursor is not
  on `0xff`;
* `truncated JPEG marker code` when fill bytes reach the end;
* `JPEG header contains a stuffed marker before scan data` for marker `0x00`;
* `JPEG ended before a start-of-frame marker` for EOI before SOF;
* `JPEG scan begins before a start-of-frame marker` for SOS before SOF;
* `JPEG marker segment length is smaller than two bytes` for a length below two;
* `JPEG marker offset overflowed usize` or `JPEG marker length overflowed usize`
  for checked range arithmetic;
* `truncated JPEG marker segment` for a segment beyond the input;
* `JPEG has no start-of-frame marker` if the scan ends without any SOF;
* `truncated JPEG start-of-frame payload`, `JPEG sample precision is zero`,
  `JPEG frame declares zero components`, `JPEG component table size overflowed
  usize`, or `truncated JPEG frame component table` for frame-level failures;
* the generic big-endian field error when a marker length or frame dimension is
  truncated.

### BMP: `inspect_bmp`

BMP dispatch checks `BM`. Full inspection first requires 18 bytes, enough for
the 14-byte file header and the four-byte DIB size at file offset `14..18`.
The DIB size is converted to `usize`, must be at least `12`, and must satisfy a
checked `14 + dib_size` range present in the input.

For a 12-byte OS/2 core DIB, fields are little-endian `u16` at offsets `18..20`
(width), `20..22` (height), `22..24` (planes), and `24..26` (bits per pixel),
widened where needed. For a DIB of at least 16 bytes, width is at `18..22`,
height at `22..26`, planes at `26..28`, and bits per pixel at `28..30`, all
initially read as little-endian `u32` or `u16`.

DIB sizes `40`, `52`, `56`, `108`, and `124` are treated as Windows headers:
width is interpreted as a signed `i32` and must convert to a positive `u32`;
height is interpreted as signed `i32`, and its absolute value is used so a
negative top-down height is accepted. `i32::MIN` is rejected because its
absolute value cannot be represented. Other DIB sizes of at least 16 use the
raw unsigned width and height. Sizes `13..15` fail because they do not provide
the dimensions and pixel contract expected by this parser.

After the common nonzero-dimension check, planes must equal one and bits per
pixel must be nonzero. Metadata mapping is:

| bits per pixel | channels | color model | sample bits |
| ---: | ---: | --- | ---: |
| `1`, `2`, `4`, `8` | `1` | `IndexedRgb` | same value |
| `24` | `3` | `Bgr` | `8` |
| `48` | `3` | `Bgr` | `16` |
| any other nonzero value | `None` | `None` | `None` |

The parser does not validate the BMP file-size field, pixel-data offset,
compression, color masks, palette contents, row stride, or pixel payload. Its
failure details are `truncated BMP file and DIB header`,
`BMP DIB size cannot address host memory: {error}`,
`BMP DIB header is only {size} bytes`, `BMP DIB header size overflowed usize`,
`truncated BMP DIB header`, `BMP width is not positive`,
`BMP height cannot be represented as a positive dimension`,
`BMP DIB header size {size} does not contain dimensions and a pixel contract`,
`BMP declares {planes} color planes instead of one`, and
`BMP declares zero bits per pixel`, together with generic little-endian field
errors and the shared zero-dimension error.

### WebP: `inspect_webp`

WebP dispatch requires `RIFF` at `0..4` and `WEBP` at `8..12`. Full inspection
requires the 12-byte RIFF header. The little-endian `u32` at `4..8` is the RIFF
size, which includes the four-byte `WEBP` form type. It must be at least four;
`declared_end = riff_size + 8` is checked and must not exceed the input length.
Trailing bytes after `declared_end` are ignored.

Chunks are walked from cursor `12` while the cursor is below `declared_end`:

| range relative to chunk | field |
| --- | --- |
| `0..4` | four-character chunk type |
| `4..8` | little-endian `u32` payload size |
| `8..8 + size` | payload |
| next byte when size is odd | one required padding byte |

The payload end and padded next-cursor calculation use checked additions and
must remain within the declared RIFF range. A dimension-bearing chunk returns
immediately after its payload parser succeeds, before its own padding byte is
checked. Therefore a dimension-bearing chunk can have an unverified missing
odd-size padding byte, while every earlier non-dimension chunk must pass the
padding check. `ALPH` sets `separate_alpha = true` for a later `VP8 ` chunk
without inspecting its payload contents; unknown chunks are skipped. The scan
otherwise fails with
`WebP has no dimension-bearing VP8, VP8L, or VP8X chunk`.

#### Extended `VP8X`

`inspect_webp_extended` requires 10 payload bytes. Payload bytes `1..4` are
reserved and must all be zero. The little-endian 24-bit values at `4..7` and
`7..10` are width-minus-one and height-minus-one; each is checked before adding
one, so successful dimensions are always positive and at most `2^24`. Flag
bit `0x10` at payload byte `0` selects four-channel `Rgba`; otherwise metadata
is three-channel `Rgb`, with eight sample bits. Other flags are not inspected.

#### Lossless `VP8L`

`inspect_webp_lossless` requires five payload bytes. Byte `0` must be `0x2f`.
The high three bits of byte `4` are a version field and must be zero. Width is
`1 + byte1 + ((byte2 & 0x3f) << 8)`. Height is
`1 + (byte2 >> 6) + (byte3 << 2) + ((byte4 & 0x0f) << 10)`. The alpha bit is
`0x10` in byte `4`. Successful metadata is three-channel `Rgb` or four-channel
`Rgba`, with eight sample bits. The bit formulas guarantee positive dimensions.

#### Lossy `VP8 `

`inspect_webp_lossy` requires ten payload bytes. Byte `0` bit `0` must be zero,
and bytes `3..6` must equal `9d 01 2a`, the key-frame start code. Width is the
little-endian `u16` at `6..8` masked with `0x3fff`; height is the corresponding
value at `8..10`. Both must be nonzero. `ALPH` state selects four-channel
`Rgba`; without it the parser reports three-channel `YCbCr`. Sample bits are
always eight. A malformed key frame reports
`WebP VP8 chunk does not begin with a key-frame header`; short payloads report
`truncated WebP VP8 frame header`.

WebP errors also include `truncated WebP RIFF header`,
`WebP RIFF size cannot address host memory: {error}`,
`WebP RIFF size overflowed usize`, `truncated WebP RIFF payload`,
`WebP chunk header offset overflowed usize`, `truncated WebP chunk header`,
`WebP chunk size cannot address host memory: {error}`,
`WebP chunk size overflowed usize`, `WebP chunk exceeds its declared RIFF
payload`, `truncated WebP chunk payload`, `WebP padded chunk size overflowed
usize`, `WebP next chunk offset overflowed usize`, and
`truncated WebP chunk padding`. VP8X-specific failures are
`truncated WebP VP8X header`, `WebP VP8X reserved bytes are nonzero`,
`WebP VP8X width overflowed u32`, and `WebP VP8X height overflowed u32`.
VP8L-specific failures are `truncated WebP VP8L header`,
`invalid WebP VP8L signature byte`, and `WebP VP8L header has a nonzero
version`.

## Shared invariants and arithmetic

`require_dimensions` is the common zero-size guard for PNG, GIF, JPEG, BMP,
and lossy VP8. It returns
`{format} declares zero-sized dimensions {width}x{height}`. VP8X and VP8L
derive dimensions by adding one to encoded minus-one or bit-packed values, so
they do not call this helper.

All multi-byte reads use one of five private helpers: `read_be_u16`,
`read_be_u32`, `read_le_u16`, `read_le_u24`, or `read_le_u32`. Each builds a
range with `offset.saturating_add(width)` and uses `slice.get`; a missing range
returns a descriptive truncation error instead of indexing or panicking:

* `truncated big-endian u16 image header field`;
* `truncated big-endian u32 image header field`;
* `truncated little-endian u16 image header field`;
* `truncated little-endian u24 image header field`;
* `truncated little-endian u32 image header field`.

Format-specific offset and size calculations use `checked_add`,
`checked_mul`, and checked integer conversions before constructing a slice. In
particular, JPEG segment ranges, GIF table spans, BMP DIB ends, WebP RIFF and
chunk ends, and the WebP padding step are never allowed to wrap. Normal source
loading applies its own configured byte bounds before this module runs, but the
module itself has no configurable byte, pixel, or decompression limit. Its
fixed limits are the widths of the fields it reads and the explicit PNG 31-bit
dimension guard.

For the public `Data` preparation boundary, the default source limit is
`1 << 30` bytes, the aggregate record limit is `10_000_000`, the vector limit is
`1 << 14`, and the textual field limit is `16 << 20` bytes. Dataset source
loading and archive expansion enforce the source limit for every image value.
The field-byte limit is applied only when a distilled value is valid UTF-8, so
opaque encoded image bytes are governed by the source limit rather than the
textual field limit. A caller that constructs a `RawTable` directly and invokes
preparation can bypass those dataset framing limits; `image_header` still
performs its own checked header ranges.

Checkpoint and KNN checkpoint decoders independently bound the number of
metadata entries (the default is `1_000_000` in each decode-limit type), but
they do not re-read or reparse the encoded image payload. Those bounds constrain
the persisted variant list, not the header parser's byte slice.

## What this boundary proves, and what it does not

On success, the boundary proves that the selected signature and the inspected
header fields are internally consistent under the rules above, and that the
metadata can be represented by the public enums and integer fields. It does
not prove a complete file, a valid compressed stream, a decodable image, a
present pixel payload, or a uniform image variant set. Uniformity is not
required because preparation records a sorted set of all headers observed in a
column and retains each original encoded value.

On failure, no `EncodedImageMetadata` is returned. Dataset and semantic
classification may have already selected the image route from a shallow
signature, but preparation is the authoritative full-header gate and reports
the exact detail with column and row context. This separation keeps detection
cheap while ensuring that bytes entering a prepared image vector have passed
the real parser.
