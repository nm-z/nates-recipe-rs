# `recipe-ingest` crate facade

Source: [`ingest/src/lib.rs`](../../src/lib.rs)

```toml
[module]
path = "ingest/src/lib.rs"
kind = "dependency-clean-external-representation-facade"
intent = "Admit bounded external bytes, frame them into tables, preserve representation contracts, and produce immutable preparation inputs."
purpose = "Expose one root API for source snapshots, table framing, dataset distillation, semantic inference, schema-driven preparation, model-container inspection, structural GGUF conversion, and init-image packing."
structure = "Twelve private implementation modules behind one explicit root re-export surface."
dependencies = ["calamine", "csv", "quick-xml", "recipe-core", "recipe-ogdl", "serde", "serde_json", "sha2", "zip"]
private_modules = ["dataset", "gguf", "gguf_ogdl", "image", "image_header", "inference", "numeric", "prepare", "safetensors", "semantic", "source", "table"]
crate_attributes = ["forbid(unsafe_code)", "deny(missing_debug_implementations)"]
state = "per-call owned or borrowed values; no global mutable state"
public_module_tree = false

[boundary]
inputs = ["filesystem paths", "bounded source bytes", "RawTable", "recipe_core::InitDataImage manifests", "ExternalValue payloads", "GgufLimits", "SafeTensorLimits"]
outputs = ["SourceSnapshot", "RawTable", "DistilledDataset", "InferredVectorList", "PreparedDataset", "PreparedInferenceDataset", "borrowed GgufArchive", "borrowed SafeTensorArchive", "structural OGDL", "PackedInitImage"]
owned_domain = "lexical framing, structural validation, lossless representation encoding, and pre-run host packing"
not_owned = ["model arithmetic", "normalization policy", "scheduling", "GPU allocation", "driver calls", "native execution", "runtime file handles"]
runtime_rule = "all filesystem and representation work completes before a runtime loop receives its inputs"

[flow]
steps = ["bounded source snapshot or admitted bytes", "format dispatch and lexical framing", "RawTable or validated container archive", "semantic inference or saved-schema application", "lossless typed preparation and train/validation partition", "downstream training or inference compilation", "native preparation and execution outside this crate"]
dataset_path = "Path(s) -> read_source_snapshot -> distill_datasets -> RawTable + source-owned semantic rules -> infer_vectors or prepare -> PreparedDataset"
inference_path = "RawTable + InferenceFeatureSchema -> prepare_inference_table -> PreparedInferenceDataset -> recipe-training inference compiler"
container_path = "bytes -> parse_gguf or parse_safetensors -> validated borrowed spans; explicit GGUF structural conversion is a separate decode/re-encode path"
image_path = "InitDataImage[] + ExternalValue[] -> pack_init_images -> one complete PackedInitImage per device"
failure_rule = "invalid bounds, malformed representations, inconsistent contracts, and arithmetic overflow return typed errors without partial public output"

[ownership]
core = "recipe_core owns Digest, DType, DeviceId, ValueId, ByteCount, InitDataImage, and their graph contracts; ingest only consumes them."
ogdl = "recipe_ogdl owns Graph parsing used by structural_ogdl_to_gguf; ingest owns the structural GGUF schema and binary conversion."
training = "recipe-training owns model-family decoding, graph compilation, and native execution; ingest supplies bytes, tables, schemas, and prepared values."
facade = "the root recipe crate re-exports this crate as recipe::engine::ingest and translates public Data declarations into its request types."
text = "recipe-text uses SourceLimit and read_source_snapshot for bounded tokenizer files."
cli = "the recipe convert command calls the streaming GGUF to structural OGDL and structural OGDL to GGUF entry points."

[limits]
source = "SourceLimit and IngestLimits are caller-fixed and nonzero; reads use limit plus one to detect growth."
dataset = "aggregate byte, record, field, archive-depth, and vector limits apply across a declared source collection."
gguf = "file, metadata, tensor, rank, string, array-element, and array-depth limits are validated before archive exposure."
safetensors = "header, data, tensor, rank, and tensor-name limits are validated before archive exposure."

[exports]
dataset = ["DatasetSourceError", "DatasetSourceErrorKind", "DatasetSourceResult", "DistilledDataset", "SourceFormat", "distill_dataset", "distill_datasets"]
gguf = ["GgufArchive", "GgufEndian", "GgufError", "GgufErrorKind", "GgufLimits", "GgufMetadataArray", "GgufMetadataEntry", "GgufMetadataType", "GgufMetadataValue", "GgufResult", "GgufTensor", "GgufTensorType", "parse_gguf"]
gguf_ogdl = ["GgufModelDescriptor", "GgufOgdlError", "GgufOgdlErrorKind", "GgufOgdlResult", "gguf_to_structural_ogdl", "gguf_to_structural_ogdl_stream", "inspect_gguf_model_stream", "structural_ogdl_declared_gguf_bytes", "structural_ogdl_to_gguf", "structural_ogdl_to_gguf_stream"]
image = ["ExternalValue", "ImagePackError", "ImagePackErrorKind", "ImagePackResult", "PackedInitImage", "pack_init_images"]
image_header = ["EncodedImageFormat", "EncodedImageMetadata", "ImageColorModel", "ImageValueLayout", "ImageValueRange"]
inference = ["InferenceDataPath", "InferenceFeatureEncoding", "InferenceFeatureSchema", "InferencePrepareError", "InferencePrepareErrorKind", "InferencePrepareResult", "PreparedInferenceDataset", "PreparedInferenceFeature", "PreparedInferenceValues", "prepare_inference_table"]
numeric = ["DecimalError", "DecimalErrorKind", "F32_GUARANTEED_SIGNIFICANT_DIGITS", "F32Decimal", "I32_GUARANTEED_SIGNIFICANT_DIGITS", "I32Decimal", "parse_contract_f32", "parse_contract_i32"]
prepare = ["CategoricalObservation", "ColumnPattern", "ComparisonOperator", "DenseMatrix", "PartitionKind", "PredicateLiteral", "PreparationRequest", "PrepareError", "PrepareErrorKind", "PrepareResult", "PreparedDataset", "PreparedPartition", "PreparedValues", "PreparedVector", "RowPredicate", "TemporalOrigin", "TrainFraction", "VariableWidthVector", "VectorMetadata", "VectorRole", "VectorSchema", "prepare_inferred_table", "prepare_table", "select_table"]
safetensors = ["SafeTensorArchive", "SafeTensorDType", "SafeTensorEntry", "SafeTensorError", "SafeTensorErrorKind", "SafeTensorLimits", "SafeTensorResult", "parse_safetensors"]
semantic = ["AmbiguousVectorModel", "CategoricalEncodingModel", "InferredVector", "InferredVectorList", "SemanticError", "SemanticErrorKind", "SemanticResult", "SemanticType", "VectorEncoding", "VectorEvidence", "infer_table_vectors"]
source = ["SourceError", "SourceErrorKind", "SourceLimit", "SourceResult", "SourceSnapshot", "read_source_snapshot"]
table = ["Delimiter", "HeaderMode", "IngestError", "IngestErrorKind", "IngestLimits", "IngestResult", "RawTable", "TableRequest", "parse_table", "read_table"]
```

## Intent and boundary

`recipe-ingest` is the preparation-side representation boundary. Its crate root
forbids unsafe code and denies missing `Debug` implementations. Every module is
private, so callers import the names above from the crate root rather than
through `recipe_ingest::table::...` or another public module path. The root
contains no runtime loop, async executor, GPU handle, device allocation, or
background state. Calls either borrow caller bytes while a validated archive is
alive or return owned values whose construction has already completed.

The crate performs two kinds of work. It performs lexical and structural work
on host bytes, such as bounded reads, delimiter framing, JSON or archive
inspection, header validation, exact decimal parsing, and tensor-span checks.
It also performs lossless representation work required to cross the input
contract, such as dictionary codes, relative whole-second temporal values,
variable-width offsets, and f32 or int32 little-endian bytes. It does not fit
model arithmetic, normalize a feature for a model, choose a schedule, compile a
kernel, allocate a device buffer, or submit a native task.

The ordinary GGUF and safetensors parsers preserve tensor payloads as borrowed
encoded spans. The explicit `gguf_ogdl` conversion API is the intentional
representation-conversion exception: it decodes supported tensor blocks into a
canonical structural OGDL description and can reconstruct the binary image.
That conversion still runs before model execution, is bounded, and reparses the
reconstructed image through the GGUF validator.

## Module structure

`lib.rs` declares these modules with `mod`, not `pub mod`, and then selects the
public names with explicit `pub use` lists:

| module | public root surface | ownership and role |
| --- | --- | --- |
| `source` | `SourceLimit`, `SourceSnapshot`, `SourceError*`, `read_source_snapshot` | One bounded regular-file read, SHA-256 content address, and closed handle. |
| `table` | `Delimiter`, `HeaderMode`, `IngestLimits`, `TableRequest`, `RawTable`, `IngestError*`, `read_table`, `parse_table` | CSV, semicolon, tab, ASCII-whitespace, and auto-delimited framing with rectangular rows. |
| `dataset` | `SourceFormat`, `DistilledDataset`, `DatasetSourceError*`, `distill_dataset`, `distill_datasets` | Deterministic file, directory, and nested ZIP traversal, specialized format dispatch, source metadata, and rectangular accumulation. |
| `semantic` | `SemanticType`, `VectorEncoding`, `VectorEvidence`, `AmbiguousVectorModel`, `CategoricalEncodingModel`, `InferredVector*`, `SemanticError*`, `infer_table_vectors` | One semantic classification and lossless encoding choice for every table column. |
| `prepare` | request, schema, value, partition, matrix, and error types plus `prepare_table`, `prepare_inferred_table`, `select_table` | Fit semantic metadata on the exact train partition, encode all retained rows, and expose fixed-width matrices only when lossless. |
| `inference` | saved feature schema, path, value, dataset, and error types plus `prepare_inference_table` | Apply an existing model schema by exact column bytes without re-inference, fitting, splitting, or normalization. |
| `numeric` | f32 and int32 decimal contracts, result and error types, constants, and parsers | Prove finite f32 round trips and exact int32 range before payload bytes are admitted. |
| `image_header` | encoded format, color model, layout, range, and metadata | Inspect private image signatures and headers; expose metadata without claiming decoded pixels. |
| `image` | `ExternalValue`, `PackedInitImage`, result and error types, `pack_init_images` | Validate core init manifests and materialize complete zero-filled per-device upload images. |
| `gguf` | limits, endian, metadata, tensor, archive, result and error types, `parse_gguf` | Validate GGUF v2/v3 structure and borrow metadata strings and tensor byte ranges. |
| `safetensors` | limits, dtype, entry, archive, result and error types, `parse_safetensors` | Validate the JSON header, shape-derived spans, and contiguous data section while borrowing bytes. |
| `gguf_ogdl` | descriptor, result and error types, in-memory and streaming conversion functions | Inspect executable GGUF v3 headers and convert the canonical structural OGDL representation. |

The implementation helpers are intentionally not exported. Examples include
the image header inspectors, semantic rules used by dataset distillation,
recursive archive walkers, GGUF binary readers and writers, safetensors JSON
visitors, and all preparation fit and encoding helpers. This prevents a caller
from constructing a `RawTable`, archive, or prepared dataset without the
checks performed by its producing boundary.

## Public API inventory and contracts

### Source snapshots and table framing

`SourceLimit::new(bytes)` accepts only a nonzero byte bound. `read_source_snapshot`
opens one regular file, rejects non-regular sources and metadata larger than the
bound, reads through `limit + 1`, rejects a file that grew past the bound, then
returns an owned `SourceSnapshot { path, bytes, digest }`. The digest is the
SHA-256 of the retained bytes as `recipe_core::Digest`. `SourceSnapshot::path`,
`bytes`, `digest`, and `into_bytes` are read-only accessors; no file handle is
retained.

`IngestLimits::new(source_bytes, records, fields_per_record, field_bytes)`
requires every bound to be nonzero. `TableRequest::new` combines a
`Delimiter`, `HeaderMode::Present` or `HeaderMode::Absent`, and those limits.
`read_table` performs one bounded read and then calls `parse_table`; the latter
frames already-admitted bytes without filesystem access. `Delimiter` supports
`Auto`, comma, semicolon, tab, and ASCII whitespace. Auto selection inspects
quoted separators and falls back to comma or whitespace rules.

`RawTable` owns headers and row field bytes. `delimiter`, `headers`, `rows`, and
`width` are the public observations. Construction is private and verifies every
row has the header width. `IngestErrorKind` distinguishes invalid limits, I/O,
source, record, field, malformed-table, width, and arithmetic failures. A
framing failure never returns a partial table.

### Recursive dataset distillation

`distill_dataset` is the one-source convenience call; `distill_datasets` accepts
an ordered iterator of paths and applies one aggregate `IngestLimits` budget to
the entire collection. Directory entries and ZIP members are sorted
deterministically. Symbolic links, archive paths that escape their root, empty
archives, and nesting beyond the implementation depth bound are rejected.

The dispatcher recognizes delimited files, JSON, text, images, GGUF,
safetensors, XLSX, PPTX, and binary payloads. Extensions select the specialized
reader where available, and recognized image signatures, tabular structure, or
probable UTF-8 text provide the content-based fallbacks. GGUF and safetensors
metadata and tensor descriptors become ordinary logical tables; tensor bytes
remain binary vector values. Every leaf contributes source context when more
than one source is declared, including declaration index, path, format, member,
content hash, file size, sample position, sample count, and source depth.

`DistilledDataset` owns the resulting rectangular `RawTable`, the semantic rules
declared by specialized readers, and a file count. `table`, `sample_count`,
`vector_count`, `file_count`, `infer_vectors`, `prepare`, and `into_table` are
its public operations. `infer_vectors` and `prepare` preserve source-owned
exact semantic rules, so a validation partition cannot influence automatic
inference when `prepare` is used directly.

`DatasetSourceErrorKind` reports invalid paths, I/O, symlinks, empty sources,
aggregate limits, malformed archives or formats, wrapped ingest failures, and
arithmetic overflow. `DatasetSourceResult<T>` is the module result alias.

### Semantic inference

`SemanticType` is the closed classification set `Numeric`, `Temporal`,
`Categorical`, `Ordinal`, `Text`, `Image`, and `Binary`. `VectorEncoding` maps
those meanings to `F32`, `I32`, `RelativeSecondsI32`, `DictionaryI32`,
`OrdinalI32`, `Utf8`, or `Bytes`; only scalar encodings report a
`recipe_core::DType`.

`infer_table_vectors(table, model)` validates table width, gathers exact byte
evidence, and emits one `InferredVector` for each source column in source order.
The classifier tries recognized image headers, temporal syntax, declared ordinal
vocabularies, exact int32, and exact f32 before delegating remaining ambiguity
to `AmbiguousVectorModel`. `CategoricalEncodingModel` is the fixed nearest
example model used by automatic dataset loading. `VectorEvidence` contains
integer counts and thousandths ratios, not host floating-point decisions.

`InferredVector` exposes source index, name, semantic type, encoding, and
evidence. `InferredVectorList::vectors` exposes the ordered list. The only public
semantic failures are inconsistent width and arithmetic overflow, represented
by `SemanticError` and `SemanticResult<T>`.

### Training preparation

`TrainFraction::new(numerator, denominator)` requires a strict fraction in
`(0, 1)` and reduces it exactly. `TrainFraction::from_f32` preserves the exact
finite binary32 fraction or rejects an unrepresentable denominator. A
`PreparationRequest` carries target column bytes, optional case-insensitive
`ColumnPattern` exclusions, OR-combined `RowPredicate` exclusions, and the
fraction. `PredicateLiteral` supports signed and unsigned integers, finite
f32 bit patterns, and text. `VectorRole` marks a retained vector as `Feature` or
`Target`.

`prepare_table` performs selection before fitting, builds a fit view from the
exact train rows, infers semantics, fits encoding metadata only on those rows,
then applies the immutable schemas to every retained row. `prepare_inferred_table`
performs the same selection, fitting, partitioning, and encoding using an
authoritative `InferredVectorList` without re-inference. `select_table` is the
target-free boundary: it applies row and column exclusions while preserving
source order, with no target choice, semantic fit, split, or encoding.

`PreparedDataset` retains source row count, retained and excluded source-row
indices, vectors, target indices in declaration order, and train and validation
partitions. `PreparedVector` exposes a row-free `VectorSchema` plus encoded
values and optional typed `CategoricalObservation` routes. `VectorMetadata`
retains temporal origins, categorical dictionaries, ordinal labels, or the
deterministic set of encoded image variants. `PreparedValues` is `I32`,
`F32Bits`, or a `VariableWidthVector` of offsets, payload, and validity bits.
Missing values remain explicit. No imputation or normalization occurs.

`PreparedDataset::fixed_dense_matrix(role, partition)` is the one fixed-width
projection. It returns `DenseMatrix::I32` when all selected vectors are int32,
or `DenseMatrix::F32Bits` when f32 values and exactly representable int32 values
can be combined. Variable-width vectors, missing values, lossy integer casts,
empty role selections, and inconsistent lengths fail with `PrepareError`.
`PrepareErrorKind` carries the complete selection, predicate, inference,
encoding, temporal-range, dense-projection, and arithmetic failure vocabulary.

### Schema-driven inference preparation

`InferenceFeatureSchema::new(source_vector, name, encoding)` records one saved
feature identity. Its encoding is `NumericI32`, `NumericF32`, or a canonical
`CategoricalDictionary`. `prepare_inference_table(table, schema)` validates
nonempty unique names and source-vector identities, matches exact header bytes,
allows source-column reordering, ignores unrelated columns, and preserves every
source row.

Numeric values use `parse_contract_i32` or `parse_contract_f32`; missing numeric
values fail. Known categorical labels use dictionary codes. Missing and unseen
nonempty labels use the reserved code `dictionary.len()`, while
`CategoricalObservation` preserves whether a row was `Known`, `Missing`, or
`Unseen { label }` and retains unseen bytes. `PreparedInferenceDataset` exposes
row count and ordered `PreparedInferenceFeature` values. Every failure is an
`InferenceDataPath`-addressed `InferencePrepareError` with one of the typed
schema, missing, ambiguous, invalid-value, or arithmetic kinds.

### Numeric, image, and init-image contracts

`F32_GUARANTEED_SIGNIFICANT_DIGITS` is six and
`I32_GUARANTEED_SIGNIFICANT_DIGITS` is nine. `parse_contract_f32` accepts a
finite decimal syntax, proves the declared significant digits round-trip through
f32, preserves signed zero, and returns `F32Decimal` with bits, precision,
canonical decimal text, and little-endian bytes. `parse_contract_i32` proves an
exact int32 value and returns `I32Decimal` with value, precision, and bytes.
Whitespace, separators, NaN, infinity, excess significant digits, range
violations, and precision loss return `DecimalError`.

`EncodedImageFormat` covers PNG, JPEG, GIF87a, GIF89a, BMP, and WebP. Public
`EncodedImageMetadata` reports dimensions and optional channels, color model,
and sample bits. `ImageValueLayout::EncodedFile` and
`ImageValueRange::EncodedBytes` explicitly state that the retained value is the
original encoded file, not decoded pixels. Header inspection itself is private;
the metadata enters public preparation schemas through image vectors.

`ExternalValue::new(logical, bytes)` pairs a core `ValueId` with admitted bytes.
`pack_init_images(manifests, sources)` checks unique devices and members,
replicated dtype and byte contracts, scalar alignment, nonoverlap, bounds, and
complete source coverage. It zero-initializes each declared image, copies every
logical source into each selected resident offset, sorts results by `DeviceId`,
and returns `PackedInitImage` values. `ImagePackErrorKind` rejects duplicate,
missing, unexpected, conflicting, incorrectly sized, or malformed sources
without exposing a partial image set. The helper consumes core planning
manifests but does not allocate or upload device memory.

### GGUF and safetensors structural archives

`GgufLimits::new` creates nonzero file, metadata-pair, tensor, rank, string,
array-element, and array-depth bounds. `parse_gguf(bytes, limits)` accepts
little-endian GGUF v2 and v3, plus big-endian v3, and validates magic, counts, UTF-8 strings,
metadata uniqueness and types, tensor names and ranks, block-derived byte spans,
alignment, zero padding, overlap, and exact terminal layout. `GgufArchive` borrows
the original bytes and exposes version, endian, alignment, data start, metadata,
tensors, keyed lookup, and `raw_tensor`. `GgufMetadataValue` preserves scalar
types and floating bit patterns; `GgufTensor` exposes dimensions, type, relative
and absolute offsets, and encoded byte length. No tensor value is converted to a
model calculation by this parser.

`SafeTensorLimits::new` creates nonzero header, data, tensor, rank, and name
bounds. `parse_safetensors` validates the eight-byte header length, JSON field
uniqueness, supported `SafeTensorDType`, shape products, exact dtype byte spans,
sorted names, and a contiguous data section with no overlap or trailing bytes.
`SafeTensorArchive` borrows the data section and exposes metadata, sorted
`SafeTensorEntry` values, keyed lookup, and `encoded_tensor`. Both parser
families use their result aliases and non-exhaustive typed error enums, and both
return no partial archive on failure.

### Structural GGUF and OGDL conversion

`gguf_to_structural_ogdl` parses a complete GGUF image and emits one canonical
`recipe-gguf-structural-v1` document rooted at `gguf`. The stream variant,
`gguf_to_structural_ogdl_stream`, performs bounded seekable passes and writes
metadata and tensor payload fields without retaining the expanded document or
binary payload. Structural output records version 3, endian, alignment,
declared file bytes, typed metadata, tensor names, dimensions, types, offsets,
and typed scalar or quantized payload blocks. It emits no base64, hexadecimal
blob, or opaque source image.

`inspect_gguf_model_stream` validates a seekable GGUF v3 layout without retaining
tensor payloads and requires a nonempty string `general.architecture`, returning
`GgufModelDescriptor` with architecture, endian, alignment, metadata count, and
tensor count. The descriptor is container identity only and does not claim that
Recipe can execute that architecture.

`structural_ogdl_declared_gguf_bytes` reads the declared output length and
rewinds its seekable input. `structural_ogdl_to_gguf_stream` performs bounded
multi-pass canonical-line validation, writes a GGUF v3 image into an initially
empty seekable output, reparses the result, and leaves the output positioned at
its end. `structural_ogdl_to_gguf` is the in-memory equivalent using
`recipe_ogdl::Graph`; it requires exactly one `gguf` root and seven ordered root
fields, rebuilds tensor bytes, and reparses the result. All conversion failures
are path-addressed `GgufOgdlError` values, including wrapped GGUF, I/O, UTF-8,
syntax, structure, value, and arithmetic failures.

## Downstream data flow and crate ownership

The normal training path is a single preparation pipeline:

```text
recipe::api::Data
    -> recipe::data_prepare maps declarations to IngestLimits and PreparationRequest
    -> recipe_ingest::distill_datasets
    -> DistilledDataset { RawTable, source semantic rules }
    -> infer_vectors or prepare_inferred_table
    -> PreparedDataset { schemas, lossless values, train/validation positions }
    -> recipe-training lowers DenseMatrix, PreparedVector, and VectorMetadata
    -> recipe-core/language/ops build the static graph
    -> recipe-prepare and native executors own realization and execution
```

Target-free inference uses the same bounded distillation and `select_table`,
then `recipe-training` applies a saved `InferenceFeatureSchema` through
`prepare_inference_table`. Checkpoint, KNN, Bayesian, and semantic model files
are read by `recipe-training` through `SourceLimit` and `SourceSnapshot`; the
training crate owns their decoders. Its supported GGUF llama path calls
`parse_gguf` and the exact numeric contracts, then binds the resulting tensor
spans into an executable graph. The ingest crate never chooses that model
family or emits a native kernel.

The root `recipe` crate exposes the complete library again as
`recipe::engine::ingest`. Its `data_prepare` module is the public declaration
adapter that chooses default bounds, distills sources, infers vectors with
`CategoricalEncodingModel`, and invokes `prepare_inferred_table`. Its CLI owns
the process-level `.gguf` and `.ogdl` conversion command and calls the streaming
conversion functions here. `recipe-text` uses the same snapshot boundary for
tokenizer files.

`recipe-core` owns the `InitDataImage` manifest and typed graph identities.
`pack_init_images` is the host-side bridge from those manifests and external
values to complete byte images; planner and executor crates own the manifest's
creation, transfer scheduling, and device upload. `recipe-ingest` has no direct
dependency on a CUDA, HSA, transport, scheduler, planner, or executor crate.

The resulting ownership rule is strict: ingest admits and describes external
representations, training and language crates assign model meaning and graph
operations, preparation realizes the static plan, and native executors perform
the runtime lifecycle. No root re-export introduces a second implementation or
permits a caller to bypass the validating boundary.
