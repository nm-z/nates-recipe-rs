# `ingest/src/gguf_ogdl.rs`: reversible structural GGUF conversion

`gguf_ogdl.rs` is the GGUF v3 structural conversion boundary in
`recipe-ingest`. It maps a validated GGUF byte image to a canonical OGDL text
description and reconstructs a GGUF byte image from that description. The
conversion preserves container metadata, tensor descriptors, exact floating
point bit patterns, packed quantization fields, ordering, offsets, and required
zero padding.

The OGDL produced here is structural, not an executable Recipe semantic model.
Its root is `gguf`, its schema is `recipe-gguf-structural-v1`, and its tensor
payload is a named description of encoded blocks. It does not tokenize a model,
infer an architecture graph, lower operations, allocate a device image, or
execute inference. `GgufModelDescriptor` deliberately reports container-level
identity only. The separate `training/src/gguf_llama.rs` loader uses
`parse_gguf` directly for the first executable dense-F32 `llama` instrument; it
does not call this converter.

The `.ogdl` suffix is shared by this structural interchange and by semantic
Recipe checkpoints, but the roots are intentionally different. The structural
reader requires `gguf`; the dense semantic checkpoint decoder in
`training/src/checkpoint.rs` requires `recipe` and its dataset, training, model,
and optional native fields. A structural conversion output must not be passed
to semantic checkpoint loading merely because both files end in `.ogdl`.

The design is the implementation of the structural conversion contract in
`system-contract.md` C24: current GGUF v3 in either byte order, all supported
metadata scalar and array forms, zero-rank and zero-extent cases admitted by
the shared GGUF rules, and every current `GgufTensorType` block layout. A
newer version or an unknown type fails closed. No raw image, opaque payload,
base64, or hexadecimal tensor text is emitted.

## Public surface and callers

`ingest/src/lib.rs` re-exports the following items. The `limits` argument is
always a `GgufLimits` created by the caller, so admission bounds are explicit
and shared with the base GGUF parser.

| API | Boundary | Result |
| --- | --- | --- |
| `gguf_to_structural_ogdl(bytes, limits)` | Complete in-memory GGUF image | A complete structural OGDL `String`. The input is first parsed by `parse_gguf`. |
| `gguf_to_structural_ogdl_stream(input, output, limits)` | Seekable GGUF input and writable OGDL output | Streams canonical OGDL after a complete validation pass. It retains descriptors and one block or scalar chunk, not the image or expanded text. |
| `inspect_gguf_model_stream(input, limits)` | Seekable GGUF input | A `GgufModelDescriptor` containing nonempty `general.architecture`, byte order, alignment, metadata count, and tensor count. It validates the complete layout without retaining tensor payloads. |
| `structural_ogdl_declared_gguf_bytes(input)` | Seekable canonical structural OGDL | Reads and validates only the preamble, rewinds the input, and returns the declared binary length. |
| `structural_ogdl_to_gguf(text, limits)` | Complete in-memory structural OGDL | Reconstructs a `Vec<u8>` and reparses it through `parse_gguf` before returning. |
| `structural_ogdl_to_gguf_stream(input, output, limits)` | Seekable, buffered structural OGDL and empty readable/writable/seekable output | Performs bounded validation and writing passes, reparses the emitted bytes through the stream validator, and leaves the destination positioned at end. |

The only direct production call path in this workspace is the CLI model
conversion command in `src/cli.rs`:

```text
recipe convert SOURCE.gguf MODEL.ogdl
    -> run_model_conversion
    -> conversion_limits
    -> gguf_to_structural_ogdl_stream

recipe convert SOURCE.ogdl MODEL.gguf
    -> structural_ogdl_declared_gguf_bytes
    -> conversion_limits
    -> structural_ogdl_to_gguf_stream
```

The CLI opens the destination with `create_new`, mode `0600`, and read/write
access, synchronizes it after a successful stream, and removes it after a
stream or synchronization failure. Existing paths are never overwritten.
`conversion_limits` uses the source byte length as the metadata, tensor,
aggregate string, aggregate array element, and aggregate depth bound, uses the
declared output length as the file-byte bound, and fixes rank at four. The
forward stream uses that file-byte bound to admit the GGUF input; it does
not impose a second byte count on the expanded OGDL output. The reverse stream
uses it to bound the declared reconstructed GGUF length. The in-memory
functions and `inspect_gguf_model_stream` are public library APIs but have no
other in-repository caller. Inference accepts `.gguf`, but its caller loads the
model through the separate training GGUF instrument.

That execution instrument is intentionally narrower than this converter. Its
`decode_gguf_llama` path requires little-endian v3, `general.architecture` equal
to `llama`, dense F32 tensors with a specific tensor-name and shape contract,
and a restricted attention/RoPE variant. It copies admitted tensor images into
an executable artifact and later lowers those images to Recipe calculations.
`gguf_ogdl.rs` instead accepts every current type handled by `GgufTensorType`,
both v3 byte orders, arbitrary valid metadata, and arbitrary valid tensor
names. A successful structural conversion therefore proves container
round-trip validity only, never that the model can execute.

## Binary and textual representations

The shared `gguf.rs` parser validates this binary shape before conversion:

```text
GGUF magic (4 bytes)
GGUF v3 (u32, little or big endian)
tensor_count (u64)
metadata_count (u64)
metadata key/type/value records, in source order
tensor name/rank/dimensions/type/offset records, in source order
zero padding to the declared alignment
tensor data, addressed by offsets relative to data_start
optional zero padding through the exact or aligned file end
```

`parse_gguf` itself can admit the little-endian GGUF v2 container for other
ingest users. Every public conversion entry point in this file deliberately
narrows that result to v3: the in-memory function checks `archive.version()`
after parsing, while both stream validators reject any version bytes that do
not decode to v3 in little or big endian. Thus a v2 parse success is not a
conversion success.

`general.alignment` is a `u32` metadata value when present, otherwise the
default is 32. It must be nonzero and divisible by eight. Tensor offsets are
relative to the aligned tensor-data start and must have the same alignment.
Header padding, inter-tensor gaps, and terminal padding are required to contain
only zero bytes. Tensor spans are sorted by offset for overlap and padding
validation, while descriptor and metadata emission retains the original source
order.

The canonical structural OGDL shape is:

```text
gguf
	schema recipe-gguf-structural-v1
	version 3
	endian little
	alignment 32
	file_bytes 1234
	metadata
		entry
			key "general.architecture"
			value string "llama"
	tensors
		tensor
			name "weight"
			dimensions 32 16
			type q4_0
			offset 0
			payload
				block 0
					delta f16 0 15 1
					quant_codes u4 ...
```

The actual `line` and `io_line` writers use one leading tab per OGDL depth and
LF line endings. A scalar-element tensor uses `chunk START` nodes followed by
one field line that can contain up to `SCALAR_CHUNK_VALUES` (128) values. A
blocked tensor uses one `block INDEX` node and one line per decoded field. Each
metadata array declaration names its element type, for example
`value array string`; every child is `element TYPE VALUE` and nested arrays use
the same form. Metadata strings and tensor names are JSON strings so spaces,
quotes, and punctuation remain unambiguous.

The stream reader enforces this canonical line profile directly. It rejects
blank lines, CRLF or trailing carriage-return line endings, and a line whose
leading indentation skips an ancestor. LF separates ordinary lines; the final
line may be unterminated. The
in-memory reverse path uses `recipe_ogdl::Graph`, whose
general parser accepts the normal OGDL forest representation; the converter
then requires the exact root, field order, and child shape described below.

The stream APIs do not own destination transactions. A library caller that
needs create-new, synchronization, or partial-output cleanup must provide that
boundary itself; the production CLI supplies it in `write_new_conversion_output`.

## Error boundary

`GgufOgdlError` carries a stable, non-exhaustive `GgufOgdlErrorKind`, a path,
and a detail string. `Display` prints `path: detail`; the kind remains
available through `kind()`. A `GgufError` from `parse_gguf` is wrapped as kind
`Gguf` at path `<gguf>`. I/O failures use kind `Io` and preserve the operation
path.

| Kind | Typical causes in this module |
| --- | --- |
| `Gguf` | The bounded base parser rejected magic, version, counts, metadata, tensor descriptors, spans, or padding. |
| `Io` | Seek, read, write, flush, or output patch failure in a stream path. |
| `InvalidUtf8` | A stream GGUF string contains bytes that are not UTF-8. |
| `InvalidSyntax` | `structural_ogdl_to_gguf` could not parse the input through `Graph::parse`. |
| `InvalidStructure` | Missing or reordered OGDL fields, wrong child count, scalar children, malformed array stack, payload field mismatch, or an impossible traversal result. |
| `InvalidValue` | Unsupported v3/version or type spelling, bad JSON, duplicate names or keys, limits, alignment, bool byte, range, dimensions, nonzero padding, or noncanonical scalar text. |
| `ArithmeticOverflow` | Checked additions, products, alignment, host-size conversions, block indexes, line numbers, or output ranges overflow. |

Paths are intentionally specific, for example
`gguf.metadata[3].value`, `gguf.tensors[1].payload`,
`<ogdl>:17`, `<gguf-output>.padding`, and
`gguf.tensors.weight.dimensions[0]`. This lets a caller distinguish a source
format error from a malformed structural edit without inspecting implementation
state.

## Forward conversion: GGUF to structural OGDL

### Complete in-memory path

`gguf_to_structural_ogdl` calls `parse_gguf(bytes, limits)`. The base parser
retains metadata and validated tensor descriptors as borrowed values over the
source image. The converter then rejects any version other than v3, creates one
`String`, and emits these seven ordered root fields:

1. `schema recipe-gguf-structural-v1`;
2. `version 3`;
3. `endian little` or `endian big`;
4. `alignment N`;
5. `file_bytes LEN` from the input slice length;
6. `metadata`, containing one `entry` per source metadata pair;
7. `tensors`, containing one descriptor and payload tree per source tensor.

Metadata is written in source order by `write_metadata_value`. Scalar values
retain their GGUF type. `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, and `i64`
are decimal; booleans are `true` or `false`; strings are JSON; and floating
values are written as exact sign, exponent, and fraction components. Arrays are
walked with an explicit pending stack, so deeply nested metadata does not rely
on recursive Rust calls. An unavailable raw tensor span is an
`InvalidStructure` failure even though the archive had validated its
descriptor.

`write_tensor_payload` divides the borrowed raw span into exact
`block_bytes()` pieces. For scalar layouts (`block_elements() == 1`) it decodes
up to 128 blocks at a time, checks that every decoded field has the same name
and type, merges the values, and emits one chunk. For every other layout it
emits each block separately and writes all decoded fields in layout order.
There is no raw byte fallback.

### Streaming path

`gguf_to_structural_ogdl_stream` first seeks to the input end, enforces the
file-byte limit, rewinds, and calls `inspect_stream_gguf`. That first pass
validates the complete image and records only `StreamTensor` descriptors,
metadata count, architecture observations, alignment, and `data_start`. The
emission pass seeks to byte 24, the position immediately after the magic,
version, tensor count, and metadata count, and rereads each metadata value into
the output. It then seeks independently to `data_start + tensor.offset` for
each descriptor and decodes only the current block or scalar chunk. The input
must be seekable because the validation pass and source-order payload seeks are
separate operations.

`StreamBinaryReader` is endian-aware and tracks its absolute input position.
Every read checks the next position against the measured file length before
calling `read_exact`. Its string method enforces both the per-value limit and
the remaining aggregate string budget, validates UTF-8, and subtracts the
consumed bytes. Its metadata walker uses `StreamArrayFrame` values instead of
recursive calls, decrements the aggregate array-element budget when an array is
opened, and rejects a depth beyond the configured limit. The callback receives
already canonical text, so the output writer never needs a second metadata
representation.

The forward stream does not snapshot or hash the input between its validation
and emission passes. The caller must keep the seekable source stable. A
truncation or incompatible layout change normally becomes an I/O or decode
failure; a mutation that remains structurally valid can be emitted as the
changed source under the first-pass descriptors. The reverse stream has the
explicit preamble and descriptor comparisons described below, but it likewise
does not compare every metadata scalar across passes.

`inspect_stream_gguf` performs the stream-only equivalent of the base parser:

* It requires `GGUF` magic and a version byte sequence that decodes to v3 in
  either little or big endian.
* It checks tensor and metadata counts against `GgufLimits`, validates nonempty
  metadata keys of at most 65,535 bytes, and rejects duplicate keys.
* It requires `general.alignment`, when present, to be `u32`; missing alignment
  uses 32. It records `general.architecture`'s type and root string when that
  key is present, but does not require an architecture for generic conversion.
* It checks unique tensor names of at most 64 bytes, rank at most four and at
  most the configured rank, dimensions, supported type codes, aligned offsets,
  and the checked encoded byte count for every tensor.
* It derives the aligned data start, accepts a tensor-free file only when its
  length is the unpadded or aligned header end, and checks all header padding,
  inter-tensor gaps, terminal padding, span order, and overlap.

`inspect_gguf_model_stream` calls this complete validator and adds the
executable-model identity condition: `general.architecture` must exist, have
metadata type `string`, and be nonempty. It returns no tensor bytes and makes
no claim that the named architecture has a Recipe lowering.

## Reverse conversion: structural OGDL to GGUF

### Streaming path and its passes

`structural_ogdl_to_gguf_stream` requires the destination to be empty at the
initial seek-to-end check. It uses `CanonicalLines` over a seekable
`BufRead`, so the same source can be rewound for bounded passes. The sequence
is:

1. `inspect_structural_stream` parses the preamble and all metadata, records
   tensor descriptors, checks names, dimensions, types, offsets, alignment,
   non-overlap, and limits, and skips payload lines only after each descriptor.
   It computes the maximum tensor-data end.
2. The input is rewound and the preamble is parsed again. Endian, alignment,
   and declared `file_bytes` must be byte-for-byte identical to pass one. The
   writer emits `GGUF`, version 3, counts, metadata, descriptors, zero header
   padding, and a zero-filled tensor-data region sized to the declared file.
   `SeekBinaryWriter` tracks its position and can patch array lengths after
   child values have been counted.
3. The input is rewound a third time. Metadata and every descriptor are parsed
   again without writing; each descriptor must equal the first-pass descriptor.
   Each payload is encoded directly at `data_start + offset`. Scalar chunks are
   expanded one value per encoded block; blocked layouts are encoded one block
   at a time. Extra payload lines and trailing input lines fail.
4. The output is flushed, rewound, and passed through `inspect_stream_gguf`.
   Alignment and tensor descriptors must match the structural header. The
   destination is finally rewound to its end before returning.

The zero-filled allocation is intentional. Structural OGDL contains offsets
and named fields, not padding bytes. The declared layout determines every
header gap, inter-tensor gap, and terminal gap, and the final validator proves
that those reconstructed bytes are zero. A mutation of the source between
passes is visible when it changes the preamble or a tensor descriptor. Metadata
is parsed and type-checked on every pass, but the implementation retains only
its count and alignment summary for cross-pass comparison; a metadata value
change with the same key, type, count, and alignment is not reported as a
source-mutation mismatch.

`parse_structural_preamble` requires exactly `gguf`, the structural schema,
`version 3`, an exact endian spelling, a nonzero multiple-of-eight alignment,
a nonzero `file_bytes`, and the `metadata` section marker. The stream parser
requires LF separators, rejects CRLF, bare carriage returns, and blank lines,
and permits an unterminated final line. `structural_ogdl_declared_gguf_bytes`
uses only this preamble parser, rewinds the source, and deliberately does not
claim that the rest of the document is valid.

The in-memory `Graph::parse` boundary is slightly broader because
`recipe-ogdl` accepts CRLF as well as LF and permits the ordinary OGDL inline
child form. The structural checks still require the resulting tree to have the
exact seven root fields and exact scalar/container children, so accepting that
lexical form does not weaken the binary schema or permit an extra field.

### In-memory path

`structural_ogdl_to_gguf` parses the complete text into a `recipe_ogdl::Graph`.
It requires one root named `gguf`, exactly seven ordered root children, and the
same schema, version, endian, alignment, file length, metadata, and tensor
markers as the stream path. `parse_metadata` and `parse_tensors` then enforce
ordered field counts and duplicate-name rules while constructing owned values.
`BinaryWriter` writes the binary header and metadata into one `Vec<u8>`, pads
the header, allocates the declared file length with zeroes, checks aligned
non-overlapping tensor spans, and fills each destination slice through
`read_tensor_payload`.

The in-memory parser uses iterative actions for metadata arrays and a custom
iterative `Drop` for `OwnedMetadataArray`, avoiding recursive traversal and
destruction for hostile nesting. It applies top-level metadata/tensor/file
limits and all block/range checks. Unlike the streaming reverse path, it does
not need a destination-empty check or multiple source passes, and the
in-memory metadata traversal does not use the stream aggregate string and array
budgets. The final `parse_gguf` call remains the authoritative binary layout
check; the converter additionally compares the reparsed alignment with the
declared alignment.

## Metadata mapping

The type names in the structural text are the exact inverse of
`GgufMetadataType::code`:

| GGUF code | `GgufMetadataType` | Structural spelling | Binary payload |
| ---: | --- | --- | --- |
| 0 | `U8` | `u8` | one byte |
| 1 | `I8` | `i8` | one signed byte |
| 2 | `U16` | `u16` | endian-aware 16-bit integer |
| 3 | `I16` | `i16` | endian-aware 16-bit integer |
| 4 | `U32` | `u32` | endian-aware 32-bit integer |
| 5 | `I32` | `i32` | endian-aware 32-bit integer |
| 6 | `F32` | `f32` | exact 32-bit IEEE bits |
| 7 | `Bool` | `bool` | byte `0` or `1` |
| 8 | `String` | `string` | u64 byte length and UTF-8 bytes |
| 9 | `Array` | `array TYPE` | element type, u64 length, typed values |
| 10 | `U64` | `u64` | endian-aware 64-bit integer |
| 11 | `I64` | `i64` | endian-aware 64-bit integer |
| 12 | `F64` | `f64` | exact 64-bit IEEE bits |

`F32Bits` and `F64Bits` are not converted through a host floating-point value.
`split_float` stores sign, exponent, and fraction separately, and
`join_float` rejects a component outside the exact width before rebuilding the
bits. This preserves signed zero, infinities, and NaN payloads. The same
representation is used for tensor `f16`, `bf16`, `f32`, and `f64` fields.

Arrays are homogeneous. On input, `parse_metadata_value` and
`parse_structural_metadata_value` compare each child value's type with the
declared element type. On output, the binary writer emits the element type and
the count before recursively nested values, but the implementation uses an
explicit stack. Empty arrays are valid and are emitted with a zero length.
Keys are exact nonempty UTF-8 strings within the GGUF 65,535-byte limit. The
validator checks length and uniqueness, not a restricted punctuation alphabet;
namespaces such as `command-r.*` remain valid.
`general.alignment` is the one metadata value with container-level semantic
meaning: it must be `u32`, and the structural preamble must agree with it or
with the default 32. `general.architecture` has meaning only to
`inspect_gguf_model_stream`, where it must be a nonempty string.

## Tensor descriptor and block mapping

`tensor_type_name` and `parse_tensor_type` are exact inverses for every current
`GgufTensorType`. The code, element count, encoded block size, and named field
sequence are:

| Type (text spelling) | Code | Elements/block | Bytes/block | Structural fields, in binary order |
| --- | ---: | ---: | ---: | --- |
| `f32` | 0 | 1 | 4 | `value f32[1]` |
| `f16` | 1 | 1 | 2 | `value f16[1]` |
| `q4_0` | 2 | 32 | 18 | `delta f16[1]`, `quant_codes u4[32]` |
| `q4_1` | 3 | 32 | 20 | `delta f16[1]`, `minimum f16[1]`, `quant_codes u4[32]` |
| `q5_0` | 6 | 32 | 22 | `delta f16[1]`, `quant_high_bits u1[32]`, `quant_low_codes u4[32]` |
| `q5_1` | 7 | 32 | 24 | `delta f16[1]`, `minimum f16[1]`, `quant_high_bits u1[32]`, `quant_low_codes u4[32]` |
| `q8_0` | 8 | 32 | 34 | `delta f16[1]`, `quant_codes i8[32]` |
| `q8_1` | 9 | 32 | 36 | `delta f16[1]`, `scaled_sum f16[1]`, `quant_codes i8[32]` |
| `q2_k` | 10 | 256 | 84 | `scale_codes u4[16]`, `minimum_codes u4[16]`, `quant_bitplanes u2[256]`, `scale f16[1]`, `minimum_scale f16[1]` |
| `q3_k` | 11 | 256 | 110 | `quant_high_masks u1[256]`, `quant_low_codes u2[256]`, `scale_codes u6[16]`, `scale f16[1]` |
| `q4_k` | 12 | 256 | 144 | `scale f16[1]`, `minimum_scale f16[1]`, `scale_codes u6[8]`, `minimum_codes u6[8]`, `quant_codes u4[256]` |
| `q5_k` | 13 | 256 | 176 | `scale f16[1]`, `minimum_scale f16[1]`, `scale_codes u6[8]`, `minimum_codes u6[8]`, `quant_high_bitplanes u1[256]`, `quant_low_codes u4[256]` |
| `q6_k` | 14 | 256 | 210 | `quant_low_codes u4[256]`, `quant_high_codes u2[256]`, `scale_codes i8[16]`, `scale f16[1]` |
| `q8_k` | 15 | 256 | 292 | `delta f32[1]`, `quant_codes i8[256]`, `block_sums i16[16]` |
| `iq2_xxs` | 16 | 256 | 66 | `delta f16[1]`, `grid_indices u8[32]`, `sign_codes u7[32]`, `scale_codes u4[8]` |
| `iq2_xs` | 17 | 256 | 74 | `delta f16[1]`, `grid_indices u9[32]`, `sign_codes u7[32]`, `scale_codes u4[16]` |
| `iq3_xxs` | 18 | 256 | 98 | `delta f16[1]`, `grid_indices u8[64]`, `sign_codes u7[32]`, `scale_codes u4[8]` |
| `iq1_s` | 19 | 256 | 50 | `delta f16[1]`, `grid_index_low u8[32]`, `grid_index_high u3[32]`, `scale_codes u3[8]`, `shift_bits u1[8]` |
| `iq4_nl` | 20 | 32 | 18 | `delta f16[1]`, `nonlinear_quant_codes u4[32]` |
| `iq3_s` | 21 | 256 | 110 | `delta f16[1]`, `grid_index_low u8[64]`, `grid_index_high_bits u1[64]`, `sign_bits u1[256]`, `scale_codes u4[8]` |
| `iq2_s` | 22 | 256 | 82 | `delta f16[1]`, `grid_index_low u8[32]`, `sign_bits u1[256]`, `grid_index_high_codes u2[32]`, `scale_codes u4[16]` |
| `iq4_xs` | 23 | 256 | 136 | `delta f16[1]`, `scale_codes u6[8]`, `nonlinear_quant_codes u4[256]` |
| `i8` | 24 | 1 | 1 | `value i8[1]` |
| `i16` | 25 | 1 | 2 | `value i16[1]` |
| `i32` | 26 | 1 | 4 | `value i32[1]` |
| `i64` | 27 | 1 | 8 | `value i64[1]` |
| `f64` | 28 | 1 | 8 | `value f64[1]` |
| `iq1_m` | 29 | 256 | 56 | `grid_index_low u8[32]`, `grid_index_high u3[32]`, `shift_bits u1[32]`, `scale f16[1]`, `subblock_scale_codes u3[16]` |
| `bf16` | 30 | 1 | 2 | `value bf16[1]` |
| `tq1_0` | 34 | 256 | 54 | `base3_pentad_codes u8[48]`, `base3_quad_codes u8[4]`, `delta f16[1]` |
| `tq2_0` | 35 | 256 | 66 | `ternary_quant_codes u2[256]`, `delta f16[1]` |
| `mxfp4` | 39 | 32 | 17 | `scale e8m0[1]`, `quant_codes e2m1[32]` |
| `nvfp4` | 40 | 64 | 36 | `subblock_scales ue4m3[4]`, `quant_codes e2m1[64]` |
| `q1_0` | 41 | 128 | 18 | `delta f16[1]`, `sign_bits u1[128]` |
| `q2_0` | 42 | 64 | 18 | `delta f16[1]`, `quant_codes u2[64]` |

The field names are part of the structural contract. They are not descriptive
comments that may be reordered or substituted. `decode_block` consumes one
exact `block_bytes` slice with `BlockReader`, emits fields in this table's
order, and rejects any unread bytes. `encode_block` looks up each field by
index, name, scalar kind, and exact count, packs it back into the same order,
and rejects an output whose byte length differs from `block_bytes`.

The compact field forms preserve the bit layout without pretending to decode
quantized weights into host `f32` values. For example, Q4-K emits separate
six-bit scale and minimum codes plus 256 four-bit quant codes; IQ2-XS emits
nine-bit grid indices, seven-bit sign codes, and four-bit scale codes; and
NVFP4 emits E2M1 codes plus UE4M3 subblock scales. These are encoded
components, not Recipe calculation tensors.

## Block decoding and re-encoding mechanics

`BlockReader` owns a byte slice, endian, and cursor. Its `take` method checks
the range before every read. Typed reads use the GGUF byte order; one-byte
values are copied directly. The helper decoders then expose the packed layouts:

* `unpack_lsb` extracts fixed-width low-bit groups from each byte;
* `unpack_halves` returns all low nibbles followed by all high nibbles, the
  ordering used by the GGML quant blocks;
* `unpack_nibble_pairs` separates paired low/high nibble arrays;
* `unpack_q3k_scales`, `unpack_k_scale_min`, and `unpack_iq4xs_scales` rebuild
  split scale codes from their low and high portions;
* `unpack_nvfp4` preserves each eight-byte subblock's low and high E2M1 lanes;
* `endian_u32_bytes` and `endian_u32` preserve the IQ2-XXS grid-word byte
  order when the word is represented as four structural `u8` values.

`parse_field_text` is the inverse text boundary. It requires the exact field
name and kind, parses unsigned or signed values, checks the declared maximum or
minimum, and parses floating fields as sign/exponent/fraction triples. It can
also require an exact vector length for non-scalar block fields. Scalar chunks
are checked to be nonempty, at most 128 values, contiguous, and no longer than
the remaining encoded block count.

`BlockWriter` provides endian-aware integer and exact-bit floating writes.
`pack_lsb`, `pack_halves`, `pack_nibble_pairs`, `pack_q3k_scales`,
`pack_k_scale_min`, `pack_iq4xs_scales`, and `pack_nvfp4` are the inverse
operations to the unpack helpers. They reject invalid widths, odd or unequal
lengths, out-of-range codes, and incomplete storage bytes. Generic
`unsigned_array` and `signed_array` conversions are checked before writing.

## Descriptor, dimension, and layout invariants

`tensor_encoded_bytes` is the one encoded-size calculation used by both stream
validation and the structural parsers. It applies these rules:

* The first dimension defaults to one for a zero-rank descriptor and must be
  divisible by the type's `block_elements`.
* If any dimension is zero, the element count and encoded byte count are zero.
* Otherwise, all dimensions are multiplied with checked `u64` arithmetic,
  divided by `block_elements`, and multiplied by `block_bytes` with another
  checked operation.
* A rank above four or above `GgufLimits::rank` is rejected by the relevant
  descriptor parser.

Tensor descriptors must have unique names. Offsets must be aligned. Sorting by
offset must produce non-overlapping spans; every gap before, between, or after
the spans must be zero. The file length must equal either the exact final span
end or the next aligned end. For a tensor-free image, the length must equal
the header end or its aligned end. Every absolute offset calculation uses a
checked addition, and every conversion between `u64`, `usize`, and `u32` goes
through `host_usize`, `host_u64`, or `host_u32`.

Names are JSON strings in OGDL and are bounded to 64 bytes by the GGUF
validator. Empty tensor names are allowed by the container parser, provided
they remain unique. The stream structural parser applies the byte bound while
reading; the in-memory Graph path reaches the same constraint through its final
`parse_gguf` validation after reconstruction.

The payload writer and reader derive block counts from the checked encoded byte
count, so a block cannot be silently truncated or extended. A scalar tensor's
chunks must cover every block exactly once. A blocked tensor's payload must have
exactly one node and the exact template field count per block.

## Limits, bounded passes, and ownership

`GgufLimits::new` requires nonzero bounds for file bytes, metadata pairs,
tensors, rank, aggregate string bytes, aggregate array elements, and array
depth. The stream implementation consumes those limits as follows:

* `StreamBinaryReader` checks input position against measured file length and
  tracks aggregate string bytes, array elements, and depth.
* `inspect_stream_gguf` bounds counts and host descriptor capacity before
  allocating vectors.
* Structural OGDL uses `StructuralBudgets` for keys, string values, array
  elements, and array depth during every stream validation pass.
* `STREAM_ZERO_BUFFER_BYTES` limits each padding read or write to 16 KiB.
* Scalar payload text is merged only in 128-value chunks. Non-scalar conversion
  retains one encoded block at a time.

The streaming GGUF-to-OGDL path retains descriptors and one block/chunk while
writing. The streaming reverse path retains descriptors, the source line
lookahead, one block's fields, and the zero-filled output destination. It never
constructs the complete expanded OGDL text or a second complete binary image.
The in-memory APIs intentionally retain the complete `String` or `Vec<u8>` and
are therefore appropriate only when the caller has made that allocation
choice. `OwnedMetadataArray` and all metadata traversals use explicit stacks to
avoid recursion-driven destruction or traversal depth failures; configured
array depth remains the admission bound.

## Code map

The source is intentionally organized as one forward and reverse path over a
shared field model rather than separate format-specific wrappers:

| Source range | Responsibility |
| --- | --- |
| `1-224` | Error vocabulary, descriptor, float-bit model, owned metadata, and parsed tensor state. |
| `225-293` | Complete in-memory GGUF-to-OGDL conversion. |
| `294-690` | Streaming binary reader, typed metadata walker, and scalar text generation. |
| `691-750` | Public architecture inspection and its executable-model identity checks. |
| `751-1168` | Streaming GGUF-to-OGDL emission, complete GGUF stream validation, zero-padding checks, and block reads. |
| `1170-1440` | Canonical OGDL line reader, aggregate budgets, endian-aware seek writer, and output patching. |
| `1441-1735` | Streaming reverse entry point, preamble parser, and declared-byte API. |
| `1736-2359` | Structural stream inspection, metadata arrays/scalars, tensor headers, and payload encoding. |
| `2361-3219` | Complete in-memory reverse path, Graph traversal, owned metadata, and binary header/payload setup. |
| `3252-4084` | Block decoding, exact float components, packed-code unpacking, and block readers. |
| `4085-4955` | In-memory payload parsing, field validation, and the complete inverse `encode_block` match. |
| `4956-5668` | Block writing and packing helpers, type-name maps, encoded-size calculation, Graph guards, numeric parsing, alignment, and error constructors. |

The important architectural invariant is that both directions use the same
`GgufTensorType` block geometry and the same named `StructuralField` vocabulary.
The structural text is therefore a lossless, inspectable intermediate for a
validated GGUF container, while remaining honest about the absence of an
execution or semantic-model lowering boundary.

## Failure matrix by boundary

The implementation has one deliberate failure boundary for each layer. A
later layer is not entered to reinterpret an earlier failure.

| Stage | Admission and checks | Representative paths and kinds |
| --- | --- | --- |
| Source length | Seek/read the measured input length and compare with `limits.file_bytes` | `gguf`, `InvalidValue` or `Io` |
| GGUF prefix | Exact `GGUF` magic and v3 endian decoding | `gguf.magic`, `gguf.version`, `InvalidValue` |
| Counts and strings | Nonzero configured limits, count bounds, UTF-8, per-value and aggregate string budgets | `gguf.tensor_count`, `gguf.metadata[i].key`, `InvalidValue` or `InvalidUtf8` |
| Metadata binary | Known type code, bool byte 0/1, typed nested arrays, aggregate element count and depth | `metadata`, wrapped `Gguf`, `InvalidValue`, or `InvalidStructure` |
| Descriptor records | Unique names and keys, rank at most four and configured rank, dimensions and known tensor code | `gguf.tensors[i]`, wrapped `Gguf` or `InvalidValue` |
| Binary layout | Alignment, checked encoded size, ordered non-overlapping spans, exact zero gaps and final length | `gguf.data`, `gguf.tensors`, `InvalidValue` or `ArithmeticOverflow` |
| OGDL syntax | Graph parse for the in-memory API, or LF canonical lines for the stream API | `<ogdl>`, `<ogdl>:LINE`, `InvalidSyntax`, `InvalidValue` |
| OGDL preamble | One `gguf` root, schema/version/endian/alignment/file length, metadata and tensor section markers | `gguf.schema`, `gguf.version`, `gguf.file_bytes`, `InvalidStructure` or `InvalidValue` |
| OGDL metadata | JSON keys and strings, exact type spellings, scalar values, homogeneous arrays, budgets | `gguf.metadata[i].value`, `InvalidStructure` or `InvalidValue` |
| OGDL descriptors | Exact field order, JSON names, dimensions, type, offset, payload marker, uniqueness | `gguf.tensors[i]`, `InvalidStructure` or `InvalidValue` |
| OGDL payload | Exact block/chunk sequence, field names/kinds/counts, code ranges, float component widths | `gguf.tensors[i].payload`, `InvalidStructure` or `InvalidValue` |
| Binary output | Declared length and layout arithmetic, empty stream destination, checked output ranges | `gguf`, `<gguf-output>`, `ArithmeticOverflow` or `InvalidValue` |
| Final validation | Reparse the reconstructed bytes through the complete bounded GGUF validator | `<gguf>`, wrapped `Gguf` or `InvalidStructure` |

The stream reverse path can report an output I/O failure after it has already
written a prefix. That is why output cleanup belongs to the CLI transaction,
not to the ingest conversion function. The in-memory reverse path has no
partial external destination, but it can allocate its `Vec<u8>` before a later
payload or final-validation error; ownership of that vector remains with the
failed `Result` and is released normally.

## Why the unedited round trip is byte preserving

For converter-produced text, the reverse mapping is intended to reproduce the
source image byte-for-byte. The proof is distributed across the layers rather
than implemented as a byte-copy shortcut:

1. The forward preamble records the source version, endian, alignment, exact
   file length, descriptor order, dimensions, types, and offsets.
2. Metadata scalar writers retain integer values and floating bit components;
   array writers retain element type, nesting, order, and length. The reverse
   metadata writers use the same GGUF codes and endian order.
3. `decode_block` exposes every payload bit as a named field. `parse_field_text`
   requires the same name, scalar kind, range, and count, and `encode_block`
   invokes the inverse packer for that exact type. No quantized value is
   rounded through an unrelated host floating representation.
4. The descriptor offsets determine each destination slice. Header padding,
   gaps, and terminal padding are zero-filled from the declared layout, so no
   padding bytes need an OGDL node.
5. Source order is retained for metadata and descriptors, while offset order is
   used only for overlap and padding checks. The two orderings therefore cannot
   accidentally be conflated.
6. The reconstructed bytes are reparsed by `parse_gguf` or
   `inspect_stream_gguf`, which checks the same counts, types, dimensions,
   offsets, spans, and zero padding before success is returned.

An edited structural document can intentionally change metadata, dimensions,
types, offsets, or field values, but it must still satisfy every invariant in
the failure matrix. A valid edited output is a new GGUF image, not a promise of
identity with the original source.

## In-memory versus streaming behavior

The two implementations share type names, field layouts, packing helpers, and
the final GGUF rules, but their allocation and syntax boundaries differ:

| Concern | In-memory API | Streaming API |
| --- | --- | --- |
| GGUF input | Borrows one complete `&[u8]` archive and its tensor spans | Seeks a `Read` source, validates first, then rereads metadata and blocks |
| OGDL input | Parses one complete `&str` into an ordered `Graph` | Uses rewindable `BufRead` lines and explicit lookahead |
| OGDL output | Builds one complete `String` | Writes each line to an `IoWrite` as it is decoded |
| GGUF output | Builds one complete `Vec<u8>` | Writes an initially empty `Read + Write + Seek` destination and patches array lengths |
| GGUF header preflight | Shared `parse_gguf` checks conservative minimum section bytes before records, including 33 bytes per tensor descriptor | Reads records directly, so the small rank-zero/empty-name edge follows the exact field reads |
| Metadata recursion | Explicit pending values and iterative array actions | `StreamArrayFrame` state machine and explicit line callbacks |
| Aggregate text/array budgets | Enforced by the final in-memory `parse_gguf`; the structural Graph walk itself has no separate `StructuralBudgets` object | Enforced on every structural validation pass by `StructuralBudgets` |
| Output transaction | Returns an owned vector or error | Leaves transaction, synchronization, and partial-file cleanup to the caller |
| Revalidation | Final `parse_gguf` checks the output and alignment is compared with the declaration | Final `inspect_stream_gguf` checks output and alignment/tensor descriptors are compared with the first pass |

This is a resource-boundary distinction, not two semantic formats. Both paths
emit and consume the same `recipe-gguf-structural-v1` tree and the same named
block fields.

## Function inventory

The following index follows the source order and names the complete operation
surface. Private helpers are listed because they carry format invariants rather
than being interchangeable convenience wrappers.

### Types and public boundaries

| Item | Responsibility |
| --- | --- |
| `GgufOgdlErrorKind`, `GgufOgdlError`, `GgufOgdlResult` | Typed conversion failures, source path, detail, and result alias. |
| `GgufModelDescriptor` and accessors | Container identity returned by the architecture inspection boundary. |
| `FloatFormat::{name,widths}`, `FloatParts` | Exact textual names, bit widths, and sign/exponent/fraction components for metadata and tensor floats. |
| `FieldValues`, `StructuralField` | Named unsigned, signed, or floating encoded block fields with range metadata. |
| `OwnedMetadataValue`, `OwnedMetadataArray`, `OwnedMetadataArray::drop` | Owned, typed reverse-path metadata values, including nested arrays, with iterative destruction. |
| `OwnedMetadataValue::value_type` | Maps an owned metadata variant back to its GGUF type code family. |
| `ParsedTensor` | Graph node plus name, dimensions, type, and offset used by the in-memory writer. |
| `gguf_to_structural_ogdl` | Complete in-memory forward conversion. |
| `inspect_gguf_model_stream` | Full stream validation plus nonempty architecture admission. |
| `gguf_to_structural_ogdl_stream` | Bounded stream forward conversion. |
| `structural_ogdl_to_gguf_stream` | Bounded, multi-pass stream reverse conversion. |
| `structural_ogdl_declared_gguf_bytes` | Preamble-only declared-length reader. |
| `structural_ogdl_to_gguf` | Complete in-memory reverse conversion. |

### Stream GGUF input and forward emission

| Item | Responsibility |
| --- | --- |
| `StreamTensor`, `StreamGgufHeader` | Retain validated descriptors, data start, alignment, counts, and architecture observations without tensor bytes. |
| `StreamArrayFrame`, `StreamMetadataScalar`, `StreamMetadataRoot` | Iterative nested-array state and the root scalar observations needed for alignment and architecture. |
| `StreamBinaryReader::new`, `position`, `read_exact`, `bytes` | Track endian, absolute position, file bound, and bounded byte reads. |
| `StreamBinaryReader::{u8,i8,u16,i16,u32,i32,u64,i64}` | Endian-aware primitive reads with checked position advancement. |
| `StreamBinaryReader::string` | Read length-prefixed UTF-8, enforce per-value and aggregate string bounds, and report invalid UTF-8. |
| `StreamBinaryReader::metadata_value` | Iteratively decode scalar and nested array values while emitting canonical OGDL lines. |
| `StreamBinaryReader::metadata_scalar` | Convert one binary scalar to canonical text and retain root `u32` or string observations. |
| `inspect_stream_gguf` | Validate v3 prefix, counts, metadata, descriptors, spans, alignment, and zero padding. |
| `validate_stream_metadata_key` | Enforce nonempty metadata keys and the 65,535-byte format bound. |
| `require_zero_stream_range` | Seek through a range in 16 KiB chunks and reject any nonzero padding byte. |
| `write_tensor_payload_stream` | Read descriptor-sized blocks, decode scalar chunks or blocked fields, and write canonical payload lines. |
| `io_line`, `stream_io_error` | Emit tabs plus LF and normalize stream read/write failures to `GgufOgdlError`. |

### Canonical structural input and stream GGUF output

| Item | Responsibility |
| --- | --- |
| `CanonicalLine`, `CanonicalLines::{new,reset,next,peek,read_next}` | Rewindable LF-only line source with depth, line number, and one-line lookahead. |
| `StructuralBudgets::{new,string,array_element}` | Apply aggregate key/string/array/depth budgets to structural text. |
| `SeekBinaryWriter::{new,position,bytes,u8,i8,u16,i16,u32,i32,u64,i64,string,zeros,patch_u64}` | Write endian-aware GGUF bytes, zero-fill gaps, and patch array lengths without losing the append position. |
| `StructuralStreamHeader`, `StructuralMetadataSummary`, `StructuralArrayFrame` | First-pass descriptor/layout summary and open-array stack for the reverse stream. |
| `expect_canonical_line`, `take_canonical_line` | Enforce exact text/depth or depth-only line contracts with line-numbered errors. |
| `parse_structural_preamble` | Parse and validate root, schema, version, endian, alignment, file length, and metadata marker. |
| `inspect_structural_stream` | First structural pass over metadata and descriptors, including name, rank, offset, overlap, and encoded-size checks. |
| `write_structural_metadata` | Walk metadata entries, enforce keys and counts, and optionally write their GGUF representation. |
| `parse_structural_metadata_declaration` | Split `label TYPE PAYLOAD`, resolve a metadata type, and validate array declarations. |
| `parse_structural_metadata_value` | Iteratively close/open nested array frames and dispatch scalar writing. |
| `open_structural_array`, `close_structural_array` | Enforce depth, write element type and placeholder length, count children, and patch the final length. |
| `write_structural_metadata_scalar` | Parse every scalar spelling, preserve exact float bits, enforce string budgets, and optionally write bytes. |
| `parse_structural_tensor_header` | Read one exact descriptor and calculate its encoded span. |
| `read_structural_tensor_payload` | Parse canonical chunk/block field lines and encode them at the declared output offset. |

### In-memory Graph and binary paths

| Item | Responsibility |
| --- | --- |
| `line`, `json_string`, `write_metadata_value` | Build canonical in-memory lines, JSON strings, and iterative metadata trees. |
| `parse_metadata` | Check metadata entry shape, keys, duplicates, and delegate value traversal. |
| `parse_metadata_value` | Use `Visit` and `FinishArray` actions to construct owned homogeneous arrays without recursion. |
| `parse_metadata_scalar` | Validate scalar node text/children and parse each supported metadata type. |
| `parse_tensors` | Check tensor field order, names, dimensions, types, offsets, and payload markers. |
| `BinaryWriter::{new,u8,i8,u16,i16,u32,i32,u64,i64,string,metadata_value}` | Build the in-memory GGUF header and typed metadata in the selected byte order. |
| `write_tensor_payload` | Decode a borrowed tensor span into canonical scalar chunks or block fields. |
| `read_tensor_payload` | Parse an in-memory payload tree and fill exact output block slices. |
| `parse_field`, `parse_field_text` | Check node shape, field name/kind, numeric ranges, float triples, and expected lengths. |
| `field_text`, `field_len`, `field_value_at`, `extend_field` | Serialize, measure, split, and merge structural fields while preserving type metadata. |

### Block decoding, packing, and guards

| Item | Responsibility |
| --- | --- |
| `decode_block`, `BlockReader`, `exact_array` | Decode one exact block according to its `GgufTensorType` and endian. |
| `unsigned_values`, `signed_values`, `signed_field`, `float_field` | Construct typed structural fields with allowed ranges or exact float formats. |
| `f16_from`, `f32_from`, `u8_array_from`, `i8_array_from`, `i16_array_from` | Read common scalar and array field forms from a block cursor. |
| `split_float`, `join_float` | Separate and validate exact IEEE component widths. |
| `unpack_lsb`, `unpack_halves`, `unpack_nibble_pairs` | Decode low-bit, nibble-half, and paired-nibble storage. |
| `unpack_q3k_scales`, `unpack_k_scale_min`, `unpack_iq4xs_scales`, `unpack_nvfp4` | Decode the split scale and subblock layouts of the advanced quantizers. |
| `endian_u32_bytes`, `endian_u32` | Preserve IQ2-XXS four-byte grid words in the selected byte order. |
| `encode_block`, `BlockWriter` | Inverse field dispatch and exact block byte emission. |
| `WriteUnsigned`, `WriteSigned` | Generic checked primitive array output for the block writer. |
| `unsigned_values_ref`, `signed_values_ref`, `float_values` | Require a field at an exact index, name, kind/format, and count. |
| `exact_signed` | Checked signed conversion before primitive output. |
| `pack_lsb`, `pack_halves`, `pack_nibble_pairs` | Validate and repack low-bit and nibble arrays. |
| `pack_q3k_scales`, `pack_k_scale_min`, `pack_iq4xs_scales`, `pack_nvfp4` | Repack split scales and subblock codes for the advanced layouts. |
| `metadata_type_name`, `parse_metadata_type` | Exact textual metadata type map and inverse parser. |
| `tensor_type_name`, `parse_tensor_type` | Exact textual tensor type map and inverse parser. |
| `tensor_encoded_bytes` | Checked dimension-to-block-to-byte calculation shared by all paths. |
| `node_text`, `node_children`, `expect_exact`, `require_no_children` | Safe Graph access and ordered scalar/container shape checks. |
| `strip_prefix`, `parse_single_u32`, `parse_number`, `parse_float_parts` | Canonical text extraction and typed numeric/float parsing. |
| `align_up`, `host_u64`, `host_u32`, `host_usize` | Checked alignment and host representation conversions. |
| `invalid_structure`, `invalid_value`, `invalid_overflow` | Constructors for the three locally generated failure classes. |

## Canonical text examples and edge shapes

The following small forms show what the parser accepts and what each form
means. They are structural examples, not executable model declarations.

### Metadata scalars and arrays

```text
	metadata
		entry
			key "general.alignment"
			value u32 64
		entry
			key "tokenizer.chat_template"
			value string "<s> {{ prompt }}"
		entry
			key "flags"
			value array bool
				element bool true
				element bool false
		entry
			key "nested"
			value array array
				element array u16
					element u16 7
					element u16 8
```

The array node records only its element type; its length is reconstructed from
the number of child lines. The binary writer emits that count after the type
code and patches it when the children close. The stream parser permits an
empty array, represented by an array declaration with no child lines. A child
whose type differs from its parent declaration is rejected before any binary
value is written.

Floating metadata does not use a decimal literal:

```text
			value f32 1 127 0
			value f64 1 2047 1
```

The first value is an exact positive infinity bit pattern and the second is an
exact negative-sign NaN payload under the selected format widths. The parser
does not classify or normalize these values; `join_float` only checks that each
component fits its sign, exponent, and fraction width.

### Scalar and blocked tensors

A scalar tensor has one `chunk` node per up-to-128-value group:

```text
		tensor
			name "bias"
			dimensions 4
			type f32
			offset 0
			payload
				chunk 0
					value f32 0 0 0 0 127 0 0 0 128 0 0 0
```

The chunk start is a logical block index, not a byte offset. The stream reverse
path requires the first chunk to start at zero, every following chunk to start
at the previous end, and the total number of values to equal the descriptor's
encoded block count. A blocked tensor identifies each encoded block:

```text
		tensor
			name "qweight"
			dimensions 32
			type q4_0
			offset 32
			payload
				block 0
					delta f16 0 15 512
					quant_codes u4 0 1 2 3 ...
```

The example omits values only for readability. A real `q4_0` block must have
one `delta` value and exactly 32 low-to-high `quant_codes` values. The decoder
and encoder agree on field order, so a text edit that moves `quant_codes` before
`delta` is a structural error even if the two field names are both present.

### Dimensions and empty payloads

The text `dimensions` with no suffix means rank zero. For a one-element scalar
type, such as `f32`, the synthetic first dimension is one and the descriptor
contains one block. For a blocked type, the synthetic first dimension one is
not divisible by its block width, so a zero-rank `q4_0` descriptor is rejected.

There is one path-specific preflight difference worth preserving in the
failure model. `parse_gguf`, which backs the in-memory forward path and the
final in-memory reverse check, performs a conservative minimum-header check of
13 bytes per metadata pair and 33 bytes per tensor-info record before reading
the actual records. The seekable stream validator reads each record directly
and has no equivalent 33-byte preflight. Because the format-level checks allow
an empty tensor name and a zero-rank scalar descriptor, a very small image with
that combination can be rejected as `Truncated` by the in-memory path while
the stream path admits it. This is an implementation difference, not a reason
to claim that a failed in-memory conversion proves the GGUF layout is invalid.

A zero extent makes the encoded element product zero only after the first-axis
divisibility check. Thus `dimensions 0 32` is valid for a 32-element block
type and has an empty `payload`, while `dimensions 1 0` is rejected for that
same type because the first axis does not fill a block. A zero-byte tensor may
still carry a nonzero aligned offset, which reserves a zero gap in the
reconstructed file and participates in span ordering.

Offsets are always relative to the reconstructed `data_start`; they are not
absolute OGDL line numbers or absolute file positions. The converter writes
the descriptor's `offset`, then adds the computed data start with checked
arithmetic when it seeks or fills the output destination.
