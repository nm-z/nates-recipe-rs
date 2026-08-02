# `training/src/knn_checkpoint.rs`: KNN semantic model codec

## Module identity

```text
crate: recipe-training
root: recipe-knn-model
format version: 1
artifact: immutable all-output KNN reference set plus row-free schemas and reduction declarations
encoding: canonical textual OGDL for writes, bounded parsed OGDL for reads
native bytes: none
optimizer state: none
training lifecycle: preparation only, no native loop
inference lifecycle: one native loop after this artifact is decoded and bound to query rows
```

This module owns the semantic `.ogdl` image for `.knn(neighbors)`. It does not
own data ingestion, feature preparation, distance calculation, native kernel
realization, device allocation, or prediction reporting. Those boundaries are
separate and are called out below so that a saved model is not mistaken for a
dense optimizer checkpoint or for a realized `.cubin` or `.hsaco` image.

The normative behavior is [system-contract C33](../../../system-contract.md#c33-deterministic-all-output-knn-reduction):
the exact prepared training partition is the reference set, each declared
target has an independent reduction and missing-value mask, stable reference
order decides distance ties, and a resume appends observations without
deduplicating them. The codec is the persistence boundary for that state.

## End-to-end role

The public workflow has two different paths. KNN `train()` prepares and may
save a semantic model without touching native hardware. Later `infer()` loads
that model, compiles a target-free query program, and executes the program on
the measured native system.

```text
recipe.data(...).target(...).norm(...).split(...)
        |
recipe.model().knn(neighbors)
        |
recipe.train().resume(optional_model).save(optional_model).run()
        |
src/training.rs::compile_knn_model
        |-- optional existing file -> load_knn_model_file
        |       -> read_source_snapshot -> decode_knn_model
        |       -> Decoder::new -> Decoder::decode -> validate_artifact
        |-- prepare_data -> prepare_knn_reference_set
        |       -> typed feature spans, reference f32 image, output masks/values
        |-- KnnModelArtifact::new -> validate_artifact
        |-- saved.continue_with(current) when a model existed
        |-- TrainingReport::knn -> optional KnnModelArtifact::save
        |       -> encode -> encode_graph -> OGDL canonical string
        |       -> atomic_save

recipe.data(...).exclude(...)
        |
recipe.model().load("model.ogdl")
        |
recipe.infer().evaluate()
        |
src/inference.rs::compile_inference_package
        -> load_semantic_model_file (root probe selects recipe-knn-model)
        -> prepare_knn_inference_table (saved feature schema is authoritative)
        -> compile_prepared_knn_inference
        |       -> query/reference f32 tensors and one input per output mask/value
        |       -> optional device normalization from saved declaration and mask
        |       -> recipe-ops::append_knn_all_outputs
        |       -> static one-iteration program
        -> prepare_and_execute_local_knn_inference
        |       -> native preparation, init admission, one loop, exit egress
        |       -> typed F32/I32 prediction images
        -> InferenceReport::Knn and public row/label reporting
```

The training branch is selected before dense compilation in
`src/training.rs::Train::try_run_with`. A KNN `TrainingReport` has no run ID,
bundle ID, journal, native kernels, external outputs, or training metrics. It
contains the validated `KnnModelArtifact` and reports
`ValidationMetricStatus::NotRequested`. Inference is the place where KNN has a
native run, one loop iteration, a journal, and typed prediction images.

## Artifact state

`KnnModelArtifact` is the complete semantic state. Its fields are private, but
the accessors below are public through `recipe_training` and the root facade.

| Artifact field | Stored meaning | Downstream use |
|---|---|---|
| `format_version` | Codec version, currently `1`. | Decoder rejects another version; resume requires equality. |
| `references.neighbors` | Nonzero requested neighbor count. | Passed to the all-output KNN operation; each output uses `min(neighbors, known_references)`. |
| `references.vectors` | Every prepared vector schema, in increasing source-index order. | Rebuilds target-free feature schema and proves output schemas refer to saved targets. |
| `references.feature_spans` | Ordered dense feature lowering: source vector, start, width, and numeric or categorical one-hot lowering. | Recreates the exact feature coordinate system for query rows. |
| `references.normalization_mask` | Optional one f32 bit per lowered feature column, with `1.0` for numeric columns and `0.0` for categorical one-hot columns. | Keeps categorical one-hot columns unchanged when numerical normalization is applied. |
| `references.reference_source_rows` | Prepared training source-row indexes in retained row order. | Preserves provenance and the stable reference-row order. The KNN operation uses the stored order, not the indexes as global identities. |
| `references.reference_rows` | Number of retained training reference rows. | Shapes every reference image and every output mask/value vector. |
| `references.feature_width` | Width after typed scalar and one-hot lowering. | Shapes the `[reference_rows, feature_width]` f32 matrix and validates spans. |
| `references.reference_feature_bits` | Exact finite f32 bit image in row-major retained-row order. | Admitted as the device reference feature input during inference. |
| `references.outputs` | One output per declared target, in target declaration order. | Supplies independent known masks, reference values or codes, class dictionaries, and public output identities. |
| `data_normalization` | Optional `ZScore`, `MinMax`, or `L2Norm` declaration. | Device inference normalizes both query and reference features in the saved coordinate system. No separate fitted host tensor is stored. |
| `operations` | Ordered `DenseOperation` tokens retained with the artifact. | Resume requires exact equality; current public KNN construction leaves it empty and inference rejects a nonempty list because heterogeneous post-KNN outputs cannot share one transform. |

`KnnReferenceSet` is immutable after construction except for the explicit
`continue_with` append. It exposes `operation_specs()` so inference can turn
each output into a numeric or categorical `recipe_ops::KnnOutputSpec`.

### Output representation

`KnnReferenceOutput` retains the exact target schema, a row-aligned `known`
mask, its `known_references` count, and one of two value forms:

| Value form | Stored image | Missing-row sentinel | Inference result |
|---|---|---|---|
| `NumericF32Bits(Vec<u32>)` | One finite f32 bit pattern per reference row. | Positive zero, ignored where the mask is `0`. | One F32 mean per query row. |
| `DiscreteI32 { codes, labels }` | One int32 code per reference row plus an exact `KnnLabelValue` dictionary. | Code zero, ignored where the mask is `0`. | One I32 mode code per query row, decoded through `labels`. |

`KnnLabelValue::I32` represents temporal labels after deterministic remapping;
`KnnLabelValue::Bytes` represents categorical, ordinal, text, image, or binary
semantic values. `KnnReferenceOutput::decode_class` returns `None` for numeric
outputs, negative codes, or codes outside the saved dictionary. It never fits a
dictionary during inference.

The artifact contains no opaque host object, source file path, optimizer
parameter, loss, epoch, or native image. Every value needed to reproduce the
reference calculation and interpret a mixed-dtype result is explicit.

## Upstream preparation that supplies the artifact

`src/training.rs::compile_knn_model` first validates the public declarations
and then calls `prepare_data` and `prepare_knn_reference_set`. The preparation
boundary requires targets and a split, so `dataset.train()` is the exact
retained training partition. With no split, the public data preparation
boundary rejects the declaration; KNN does not silently choose a subset.

The public model must contain exactly one `.knn(neighbors)` block. Model
validation rejects composition with any other block and rejects activations or
normalization after the terminal KNN block. The KNN compiler additionally
rejects Bayesian dependencies, a loaded model declaration, an objective,
gradient clipping, optimizer, learning-rate, schedule, warmup, epoch, log,
plot, native resume-kernel, or native save-kernel declarations. These are
unsupported semantic concepts for reference preparation, not decoder
fallbacks.

`prepare_knn_reference_set` performs the following work:

1. Reject an empty training partition or an empty declared-target list.
2. Build `DenseFeaturePlan::from_prepared`. Numeric I32/F32 features lower to
   one scalar column. Categorical dictionary features lower to dictionary width
   plus one reserved missing/unseen column. The plan records contiguous spans
   and an optional exact `0.0/1.0` normalization mask.
3. Lower the train partition with `lower_dense_features`. Numeric I32 values
   must convert to f32 exactly. F32 values must be finite. Categorical codes
   select a dictionary or reserved one-hot position. The resulting matrix is
   retained as exact f32 bits; it is not normalized on the host.
4. Resolve every target source index in the prepared vector list and preserve
   the target declaration order. Duplicate declarations or an absent target
   become `TrainingCompileErrorKind::InvalidTargetMatrix`.
5. Prepare each target according to its semantic tuple:
   numeric I32/F32 targets become finite f32 bits; categorical and ordinal
   targets use their fitted byte dictionaries; temporal targets build a sorted
   deterministic I32 label dictionary; text, binary, and image targets build a
   sorted byte-value dictionary. Missing rows retain alignment but receive a
   `0` mask and sentinel value.
6. `finish_output` counts known rows, rejects a target with no known training
   reference, and returns the target schema plus its value image. Discrete
   dictionaries must be nonempty, unique, and fit the int32 class-code domain.

The resulting reference row order is the order of the prepared training
partition. It is not sorted by distance or target value. Later stable distance
sorts therefore retain this order for exact distance ties.

Relevant preparation sources are `training/src/knn.rs:16-216` for the public
reference types and plan assembly, `training/src/knn.rs:218-568` for feature and
target lowering, and `training/src/knn.rs:570-660` for dictionary, count, and
error checks. The dense span and one-hot definitions are in
`training/src/model.rs:960-1003` and `training/src/model.rs:1145-1240`.

## Canonical OGDL structure

`encode_graph` creates one `recipe-ogdl::Graph` root named
`recipe-knn-model`. The root fields are fixed and complete:

```text
recipe-knn-model
    format-version <u32>
    neighbors <u64>
    data-normalization <none | z-score | min-max | l2-norm>
    operations
        operation <DenseOperation token>       # zero or more
    vectors
        vector ...                              # one per saved vector
    feature-spans
        span ...                                # one per saved feature
    normalization-mask
        none | f32-bits <concatenated u32 bits>
    reference-shape
        rows <usize>
        feature-width <usize>
    reference-source-rows <concatenated u64 values>
    reference-feature-f32-bits <concatenated u32 values>
    outputs
        output ...                              # one per declared target
```

The encoder always emits the fields above, including an empty `operations`
node and a `normalization-mask none` node when there is no categorical feature.
`recipe_ogdl::Graph::to_canonical_string()` supplies the textual indentation and
ordering used by the saved image.

### Vector and metadata entries

Each `vector` contains `source-index`, `name-bytes`, `role`, `semantic-type`,
`encoding`, and one `metadata` tag. `encode_metadata` supports all
`CheckpointArtifactMetadata` variants used by the ingest schema:

```text
metadata none
metadata temporal unix-seconds <i64> nanoseconds <u32>
metadata categorical value-bytes <hex> ...
metadata ordinal value-bytes <hex> ...
metadata image
    variant
        format <png | jpeg | gif87a | gif89a | bmp | webp>
        width <u32>
        height <u32>
        channels <u8 | none>
        color-model <known model | none>
        sample-bits <u8 | none>
        value-layout encoded-file
        value-range encoded-bytes
```

`CheckpointArtifactVector` is row-free schema. It retains source identity,
name bytes, role, semantic type, encoding, and metadata, but not source-row
values. `validate_saved_vector` delegates semantic/encoding/metadata
compatibility to the common checkpoint validator. Temporal nanoseconds must be
below one billion; dictionaries and image variants follow their common
checkpoint rules.

### Feature spans

Each `span` stores `source-index`, contiguous `start`, `width`, and a `lowering`
tag:

```text
lowering numeric-scalar
lowering categorical-one-hot dictionary-width <usize> reserved-index <usize>
```

The reserved index is equal to the dictionary width and the span width is
`dictionary-width + 1`. The span list is the authoritative mapping from saved
vector schema to the dense feature coordinate system. It is not safe to rebuild
feature width from names alone.

### Output entries

Each `output` stores the target source identity, a compact binary known mask,
and one typed value tag:

```text
output
    source-index <usize>
    known-mask 0b<one 0 or 1 digit per reference row>
    values numeric-f32-bits <8 lowercase hex digits per row>

output
    source-index <usize>
    known-mask 0b<one 0 or 1 digit per reference row>
    values discrete-int32
        codes <8 lowercase hex digits per row>
        labels
            int32 <canonical i32>
            bytes <canonical hex>
```

Output order is declaration order. It is not sorted by source index, so resume
compatibility compares both the output count and the pairwise schema order.

### Scalar and payload codecs

The small helpers near `knn_checkpoint.rs:370-428` define the wire forms:

| Helper | Wire form | Decode rule |
|---|---|---|
| `encode_bytes` | `0x` followed by two lowercase hex digits per byte. | Even digit count, lowercase `0x`, bounded payload allocation. |
| `encode_u32_hex` | `0x` followed by eight lowercase digits per `u32`, including exact f32 bits. | Caller supplies the expected element count; digit length must be exactly `count * 8`. |
| `encode_i32_hex` | The same u32 image after two's-complement `i32 as u32`. | Parsed as u32 bits and cast back to i32, preserving negative codes. |
| `encode_usize_hex` | `0x` followed by sixteen lowercase digits per source-row index. | Parsed as u64 and converted to the current `usize`; a value that does not fit is rejected. |
| `encode_known` | `0b` followed by one `0` or `1` per row. | Exact row count and binary digits are required. |
| integer fields | Rust decimal `ToString` output. | Parsed and reserialized; a spelling that changes on reserialization is noncanonical. |

The encoder validates first, so its `encode_known` fallback of “anything other
than `1` becomes `0`” is unreachable for a valid artifact. The decoder does not
accept uppercase hex, missing prefixes, leading-zero decimal spellings that are
not the type's canonical `ToString`, extra digits, or malformed binary masks.

## Construction and encode path

`KnnModelArtifact::new` stamps `KNN_MODEL_FORMAT_VERSION`, collects the supplied
operation sequence, and immediately calls `validate_artifact`. This is the
constructor used by `compile_knn_model` after data preparation.

`KnnModelArtifact::encode` validates again, builds the graph, serializes it to
the canonical OGDL string, and returns UTF-8 bytes. Revalidation makes an
artifact mutated by a caller inside the crate fail at the write boundary
rather than emitting a partially checked image. `KnnModelArtifact::decode` is
only a convenience wrapper around `decode_knn_model`.

`KnnModelArtifact::save` then:

1. Requires a path whose extension is exactly `.ogdl`. A `.cubin` or `.hsaco`
   target is an `InvalidTarget` error, not a native artifact route.
2. Encodes before opening the target and converts the byte length to `u64`.
3. Calls the common `atomic_save` helper. That helper checks the parent and
   target, checks filesystem capacity while preserving the exact user
   reservation, writes a private temporary file, flushes and syncs it, checks
   the measured byte count, renames it into place, and syncs the parent.

The common helper can therefore return `InvalidTarget`, `InsufficientCapacity`,
or `Io` in addition to manifest/codec errors. The KNN save call never writes a
native kernel and never creates a journal, plan, cache, profile, or other
execution artifact.

## Artifact validation

`validate_artifact` is used by construction, encoding, and after resume append.
Its checks are ordered as follows.

### Version and matrix image

* `format_version` must equal `1`.
* `reference_rows` and `feature_width` must be nonzero.
* `reference_rows * feature_width` uses checked `usize` arithmetic.
* `reference_feature_bits` must have exactly that many elements, and every bit
  pattern must decode to a finite f32.
* `reference_source_rows.len()` must equal `reference_rows`.

The source-row values themselves are not required to be unique or sequential.
They are retained provenance and order, not global identities. Resume therefore
preserves repeated observations and does not attempt deduplication.

### Vector schemas

`validate_vectors` requires at least one vector. It rejects duplicate source
indexes, non-increasing source-index order, empty names, and duplicate names.
Each vector then passes `validate_saved_vector`, which enforces the semantic
type and encoding tuple plus its metadata. A target dictionary or image
metadata error is a common checkpoint validation error, not a KNN-specific
reinterpretation.

### Feature spans

`validate_spans` requires at least one span. It builds the set of saved feature
source indexes and walks spans with a cursor that starts at zero:

* every span is nonempty and starts exactly at the cursor;
* each source is a saved feature and occurs only once;
* numeric lowering has width one;
* categorical one-hot lowering has `reserved_index == dictionary_width` and
  width `dictionary_width + 1`;
* checked cursor advancement ends exactly at `feature_width`;
* the seen source set exactly equals the saved feature set.

This catches gaps, overlaps, duplicate features, a target accidentally lowered
as a feature, and a stale shape declaration.

### Normalization mask

When present, the mask length must equal `feature_width` and every element must
be exactly `0x0000_0000` or `0x3f80_0000`, the positive-zero and positive-one
f32 bit patterns. `None` is allowed and is the normal current representation
when all features are numeric. The validator checks the wire invariant, while
the preparation path is responsible for making the mask agree with its feature
spans.

### Outputs

`validate_outputs` requires at least one output and checks each output against
the saved vector map:

* source identity is unique;
* the output schema is exactly equal to one saved vector schema;
* that schema has target role;
* `known.len()` equals `reference_rows` and each entry is `0` or `1`;
* the counted `1` entries are nonzero and equal `known_references`;
* numeric values have one finite f32 bit pattern per row;
* discrete values have one code per row, a nonempty dictionary with at most
  `i32::MAX` labels, unique labels, and codes in dictionary range.

The checkpoint layer validates the structural output kind and code range. The
upstream target-preparation match is what establishes that, for example,
temporal values use `KnnLabelValue::I32` and image values use bytes. It does not
infer a new semantic dictionary during validation or inference.

All validation failures before decoding are `CheckpointError::InvalidManifest`
with a human-readable detail. A decoded malformed value is represented as a
path-addressed `CheckpointDecodeError` instead, as described next.

## Resume and append semantics

### Public entry and existence condition

`Train::resume` accepts one semantic `.ogdl` model path or the literal pair
`.resume("model.ogdl", "kernel.cubin" | "kernel.hsaco")`. The public API checks
the extension and argument shape. KNN compilation rejects any native resume
kernel because KNN has no native training kernel; only the semantic model path
is meaningful.

`compile_knn_model` checks `Path::try_exists` before loading. A missing model is
normal: it means start with the current prepared reference set. An existing
path is read with `load_knn_model_file` and decoded with the default
`KnnModelDecodeLimits`. A directory, unreadable file, over-limit file, invalid
OGDL image, or invalid semantic state is an error; the missing-file rule does
not mask those cases.

### Compatibility gate

`KnnModelArtifact::continue_with(saved, current)` validates both artifacts before
comparing them. `validate_resume_compatibility` requires exact equality for:

* format version, requested neighbors, data-normalization declaration, and
  operation topology;
* the complete row-free vector schemas and order;
* feature spans, normalization mask, and feature width;
* output count and every output schema in declaration order.

Any mismatch returns `CheckpointError::IncompatibleResume` with one of the
specific details for neighbor/normalization/topology, row-free schema/lowering,
or output schema/order. The saved reference rows, source-row indexes, target
masks, and target values are intentionally not compared. They are the state to
be extended.

### Append operation

`append_reference_set` performs checked row and feature-element additions,
reserves capacity for source rows and feature bits, appends current rows after
saved rows, and appends each output in the same order. The combined feature
image must still have exactly `combined_rows * saved.feature_width` elements.
Allocation failures and arithmetic overflow are mapped to
`IncompatibleResume`, because the requested continuation cannot be represented
as one compatible artifact.

For each output, `append_output` adds known-reference counts with checked `u64`
arithmetic, appends the current binary mask, and then appends the typed values.
Numeric bit images concatenate directly. Discrete outputs use
`append_discrete_output`:

1. Build a map from every saved label to its existing int32 code. Saved codes
   never move.
2. Walk current labels in their saved dictionary order. Existing labels reuse
   their saved codes; unseen labels append to the saved dictionary and receive
   the next int32 code.
3. For every current row code, look up the current label, then translate that
   label through the combined map. Negative or out-of-range current codes are
   incompatible-resume errors.
4. Append the translated code image. Unknown rows retain their sentinel code,
   but their appended mask entry remains zero and excludes them from reduction.

Thus dictionary order and class identity are stable across resume, while a
current partition can introduce new labels. Repeated source observations stay
in the reference image because multiplicity is statistical weight and source
row indexes are not a deduplication key. Saved rows remain first, so they retain
stable distance-tie precedence.

The final artifact is validated again after append. A failed continuation does
not return a partially appended artifact because `self` is consumed by the
method.

## Bounded decoder

`decode_knn_model` constructs a `Decoder` and then calls `Decoder::decode`.
Decoder state includes the parsed graph, caller-supplied limits, cumulative
payload bytes, cumulative metadata entries, and cumulative labels. The default
limits are finite:

| Limit | Default |
|---|---:|
| source bytes | `1 << 30` |
| OGDL nodes | `4_000_000` |
| vectors | `65_536` |
| feature spans | `65_536` |
| reference rows | `100_000_000` |
| feature width | `1_000_000` |
| outputs | `65_536` |
| labels | `1_000_000` |
| metadata entries | `1_000_000` |
| total decoded payload bytes | `1 << 30` |

### Source and graph admission

`Decoder::new` rejects a source larger than `source_bytes`. Before parsing it
computes a node pre-bound from one plus newline/tab count, rejects that bound
when it exceeds `nodes`, converts the source to UTF-8, and parses OGDL. It then
checks the actual graph node count against the same limit. The errors are
`LimitExceeded`, `InvalidUtf8`, or `InvalidSyntax`, all rooted at
`<checkpoint>`.

The source snapshot boundary in `training/src/inference.rs::load_knn_model_file`
adds regular-file and I/O checks before this decoder. `load_semantic_model_file`
uses the maximum source bound needed by the semantic model families, probes only
the first root line, and still delegates complete syntax and validation to this
decoder.

### Root and required fields

`Decoder::decode` requires exactly one graph root named `recipe-knn-model`. It
then requires exactly these root fields:

```text
format-version, neighbors, data-normalization, operations, vectors,
feature-spans, normalization-mask, reference-shape,
reference-source-rows, reference-feature-f32-bits, outputs
```

`fields_from` rejects an unknown child, a duplicate field, or a missing required
field. It does not silently select a fallback family or ignore extra state.
Scalar fields must have exactly one leaf child; tagged fields must have exactly
one tag child; a `none` tag must have no descendants. Decode paths are built
from `CheckpointPath` fields and indexes, so failures identify locations such as
`vectors[2].metadata.dictionary[1]` or
`outputs[0].values.codes`.

### Parse order and object assembly

The decoder parses and checks values in this order:

1. Canonical `format-version`, nonzero `neighbors`, and data-normalization
   token.
2. Ordered operation entries, rejecting an unknown `DenseOperation` token.
3. Bounded vectors, including bytes, role, semantic type, encoding, and
   metadata.
4. Bounded feature spans and their numeric or categorical lowering tags.
5. Optional normalization mask. Its f32 count is derived from the hex image,
   then exact hex parsing and cumulative payload accounting are applied.
6. `reference-shape.rows` and `reference-shape.feature-width`, both nonzero and
   within their individual limits.
7. Exactly `reference_rows` source-row indexes, each encoded as one 16-digit
   u64 chunk, converted to the host `usize`.
8. Exactly `reference_rows * feature_width` f32 bit patterns.
9. Bounded outputs. Each source index must identify a saved target vector;
   each known mask has exactly one bit per row; numeric or discrete values are
   parsed according to their row count; labels contribute to the cumulative
   label limit.
10. A `KnnModelArtifact` is assembled and passed through `validate_artifact`.

When final validation reports an `InvalidManifest`, the decoder wraps it as an
`InconsistentValue` at the root. An existing `CheckpointError::Decode` is
preserved unchanged. This keeps syntax/canonicality failures at their precise
path and structural cross-field failures in the same typed decode family.

### Metadata parsing

`parse_metadata` accepts the same `none`, `temporal`, `categorical`, `ordinal`,
and `image` tags emitted by `encode_metadata`. Byte and image entry lists call
`reserve_metadata` before allocating. Image variants require exactly the
encoded-file/encoded-bytes layout and range emitted by this codec; a different
layout is an `InvalidValue`, not a silently normalized alternative. The common
vector validator then checks the semantic tuple and image header facts.

### Payload and canonical-value accounting

`reserve_payload` accumulates checked byte counts and rejects a total above
`total_payload_bytes`. It is called for bytes, f32/i32 images, source-row
images, known-mask digits, and labels. `reserve_metadata` performs the same
checked accounting for dictionary and image metadata entries. Label counts are
tracked separately across all outputs.

`canonical_hex` requires a lowercase `0x` prefix and lowercase ASCII hex. The
integer parser accepts only a decimal spelling that parses and round-trips to
the same `ToString` value. `parse_bytes`, `parse_u32_hex`, `parse_i32_hex`, and
`parse_usize_hex` all check exact digit lengths before allocating. `parse_known`
requires a lowercase `0b` prefix, exactly `reference_rows` digits, and only
`0`/`1`.

The decoder therefore has no unbounded `Vec` reservation driven solely by a
declared count or hex string length. Every allocation is tied to a finite shape,
an explicit per-kind limit, and cumulative payload/metadata/label budgets.

## Public save, resume, and report behavior

The root `Train` builder owns path declaration and extension validation. A
one-path `.save("model.ogdl")` selects semantic-model output. A native path or
literal model-plus-native pair is rejected during KNN compilation because the
reference preparation has no native training kernel. Omitting `.save(...)`
exports nothing. Omitting `.resume(...)` starts a new reference set and does not
disable an independent semantic save.

`Train::try_run_with` handles KNN before dense native package construction:

```text
report = TrainingReport::knn(compile_knn_model(...)?);
if save_model_destination exists:
    report.save_model(destination)?;
return report;
```

`TrainingReport::save_model` dispatches the KNN payload to
`KnnModelArtifact::save`. `TrainingReport::save_native_kernel` contains an
explicit KNN unsupported error, although the KNN compiler rejects native save
declarations before that method can be selected. The report accessors make the
absence of a native training lifecycle explicit:

| `TrainingReport` accessor | KNN result |
|---|---|
| `kind()` | `TrainingModelKind::Knn` |
| `knn_model()` | `Some(&KnnModelArtifact)` |
| `run()`, `bundle()` | `None` |
| `journal()`, `native_kernels()`, `native_evidence()`, `training_evidence()` | `None` |
| `external_outputs()`, `metrics()` | Empty slices |
| `validation_status()` | `NotRequested` |
| `gracefully_stopped()` | `false` |

This is intentional. KNN's “training” operation is semantic reference
preparation and optional file export, not an optimizer loop or a native image
realization.

## Decode and inference handoff

### Model-family selection

Target-free `recipe.model().load("model.ogdl")` is accepted by the root
inference boundary. `load_semantic_model_file` reads a bounded snapshot, probes
the first root line, and dispatches `recipe-knn-model` to
`decode_knn_model`. It never falls back to the dense checkpoint decoder if the
KNN decoder fails. `SemanticModelArtifact::Knn` retains the decoded artifact in
the inference package.

`load_knn_model_file` and `load_and_prepare_knn_inference` expose the same
bounded decoder directly to advanced callers. All source reads happen before
native preparation; decoded bytes are not read from the file again during
execution.

### Schema-bound query preparation

`prepare_knn_inference_table` derives a target-free feature schema from the
saved vectors and feature spans, calls `recipe_ingest::prepare_inference_table`,
and checks the prepared spans against the saved spans. Source columns may be
reordered and unrelated columns ignored, but every saved feature must be
present with a matching semantic encoding and lowering. Targets are not
required in the query table. The model's saved output schemas and dictionaries
remain authoritative.

### Static graph compilation

`compile_prepared_knn_inference` rejects nonempty post-KNN operations, zero query
rows, an empty reference matrix, or extents that do not fit the graph's u64
domain. It compiles:

* query features from the newly prepared rows under the saved spans;
* saved reference features as an external F32 `[reference_rows, feature_width]`
  input, using little-endian bytes of `reference_feature_bits`;
* one external I32 known mask per output;
* one external F32 reference-value vector for numeric output or I32 code vector
  for discrete output;
* one F32 or I32 prediction tensor with shape `[query_rows, 1]` per output.

When a normalization declaration exists, the compiler emits device calculations
for both matrices. Z-score reduces saved references to means and variances;
min-max reduces minimum and maximum; L2 computes a per-row norm. The optional
normalization mask leaves categorical one-hot columns at their raw values and
normalizes numeric columns. The epsilon used by the scalar programs is the
intrinsic `1.0e-6` calculation guard in `training/src/inference.rs`; it is not a
serialized fitted tensor.

The compiler calls `knn_all_output_requirements` for checked workspace and ID
counts, builds a namespace for the operation fragment, appends
`recipe_ops::append_knn_all_outputs`, validates and round-trips the graph, and
constructs `StaticCalculationProgram` with exactly one loop iteration. The
operation emits one shared rooted-L2 distance matrix, stable distance selection
per independent mask, numeric means, and discrete modes with lowest-code tie
breaking. Those calculations belong to `ops/src/knn_outputs.rs`; the checkpoint
only supplies their inputs and semantic counts.

### Native execution and public interpretation

`prepare_and_execute_local_knn_inference` validates the compiled boundary,
prepares the measured native program, requires one loop iteration, rejects loop
external transfers and user metrics, packs all external inputs into init images,
maps every output to an exit transfer, and executes the ordinary
`init -> loop -> exit` lifecycle. `collect_knn_inference_predictions` checks
that every declared output has exactly one non-overlapping exit image, matching
logical value, device source, dtype, shape, and byte count. It returns
`CompletedKnnInferenceExecution` in declaration order with mixed F32/I32 images.

`src/inference.rs::InferenceReport` stores both the decoded artifact and the
completed execution. KNN reports expose `knn_predictions()` and native evidence,
but `prediction()` and the f32-only `values()` iterator return `None` or an
empty iterator because outputs can be mixed dtype. `decode_knn_class(output,
code)` delegates to the saved output dictionary; it never guesses a label from
the query data. Public row reporting checks target identity, prints numeric means
as values, and prints discrete codes together with their exact I32 or byte
labels. Unknown output codes are a post-exit invalid-data error.

## Error boundaries

| Stage | Error family | Examples and meaning |
|---|---|---|
| Public declaration | `DeclarationError` or `TrainingError::Unsupported` | Wrong KNN composition, zero neighbors, post-KNN operation, optimizer/epoch/log/native artifact declaration. |
| Data preparation | `DataPreparationError` / `TrainingCompileError` | Missing target or split, empty training partition, unsupported feature tuple, nonfinite or inexact numeric conversion, no known target value. |
| Existing resume file read | `InferencePreparationError` wrapped as `TrainingError::Resume` | Source limit, nonregular or unreadable file, arithmetic bound failure. |
| OGDL source admission | `CheckpointDecodeError` | Source byte or node limit, invalid UTF-8, invalid OGDL syntax. |
| OGDL structure/value parse | `CheckpointDecodeError` | Missing or duplicate field, unknown field/tag, noncanonical integer/hex/binary value, wrong image shape or count. |
| Cross-field artifact validation | `CheckpointDecodeErrorKind::InconsistentValue` on decode, `InvalidManifest` on construction/encode | Shape mismatch, bad span coverage, invalid target schema, mask count mismatch, nonfinite image, invalid class code. |
| Resume compatibility/append | `CheckpointError::IncompatibleResume` | Declaration/schema/lowering/order mismatch, dictionary remap failure, arithmetic overflow, reserve failure. |
| Semantic save | `CheckpointError` wrapped as `TrainingError::Checkpoint` | Non-`.ogdl` target, capacity reservation, temporary write/sync/rename failure, measured-size mismatch. |
| Query schema/graph compile | `InferencePreparationError` or `InferenceCompileError` | Missing saved feature, span disagreement, zero query rows, unsupported post-KNN operations, extent/identity/workspace failure. |
| Native handoff/lifecycle | `InferenceExecutionError` | Invalid boundary, wrong iteration count, missing init or exit image, backend failure, incomplete loop, prediction source/dtype/size mismatch. |
| Public report | `io::Error` wrapped as `InferenceError::Runtime` | Output count/target identity mismatch or unknown discrete code while writing rows. |

No stage substitutes another model family, invents a kernel, drops an unknown
target, or silently falls back to a host-side prediction. The first failing
boundary remains visible through its typed error and checkpoint path where
available.

## Invariants across the complete path

1. The artifact root is `recipe-knn-model` and the only supported format
   version is `1`.
2. The saved reference matrix is finite f32, row-major, and exactly shaped by
   `reference_rows * feature_width`.
3. The saved vector schemas are row-free, strictly source-ordered, named, and
   semantically compatible with their metadata.
4. Feature spans are contiguous from zero and cover every feature exactly once;
   categorical spans include their reserved missing/unseen column.
5. Every output is a distinct saved target schema in declaration order, with one
   independent binary mask and one value image per reference row.
6. Numeric output values are finite f32 bits. Discrete output dictionaries are
   unique and code images stay inside dictionary range.
7. A resume compares every declaration, row-free schema, lowering, and output
   schema before appending. Saved rows remain first, repeated rows remain, and
   saved dictionary codes never change.
8. A missing resume model starts fresh. An existing malformed or incompatible
   model fails; it is not ignored.
9. KNN semantic save writes only the requested `.ogdl` model through the common
   atomic-save boundary. KNN has no native training artifact.
10. Inference binds query data to the saved schema, admits all reference and
    output inputs during `init`, executes one native loop, and egresses only
    typed predictions during `exit`.
11. Public interpretation preserves output declaration order and uses only the
    saved dictionary for discrete labels.

## Concrete artifact example

The workspace's `cookbook-knn.ogdl` is a 51-line model image produced by the
public cookbook. Its header demonstrates the complete shape:

```text
recipe-knn-model  format-version  1
    neighbors  3
    data-normalization  z-score
    operations
    vectors  vector  source-index  0 ...
    feature-spans  span  source-index  0 ...
    normalization-mask  none
    reference-shape  rows  9  feature-width  2
    reference-source-rows  0x<9 sixteen-digit u64 chunks>
    reference-feature-f32-bits  0x<18 eight-digit f32 chunks>
    outputs  output  source-index  2 ...
             output  source-index  3 ...
```

The actual file contains one categorical target with byte labels and one
numeric target, both with nine known rows. The categorical codes and numeric
bits are retained separately, which is why a single dense f32 prediction image
would lose information.

## Source map

| Evidence | Location |
|---|---|
| KNN artifact type, limits, constructor, accessors, resume, encode, and save | `training/src/knn_checkpoint.rs:20-148` |
| OGDL root, vector, metadata, span, output, and primitive encoders | `training/src/knn_checkpoint.rs:150-428` |
| Artifact validation, resume compatibility, row append, dictionary remap | `training/src/knn_checkpoint.rs:430-634` |
| Vector, feature-span, mask, and output validators | `training/src/knn_checkpoint.rs:636-805` |
| Decoder source admission, root fields, object assembly, and operation/vector parsing | `training/src/knn_checkpoint.rs:807-1156` |
| Metadata, byte/image entries, spans, mask, outputs, and label parsing | `training/src/knn_checkpoint.rs:1158-1512` |
| Required-field, scalar/tag, allocation-limit, and payload parsing helpers | `training/src/knn_checkpoint.rs:1514-1812` |
| Canonical hex/integer and enum token codecs | `training/src/knn_checkpoint.rs:1814-1999` |
| Public KNN declaration restrictions and save/resume path forms | `src/api.rs:1462-1496`, `src/api.rs:1992-2079` |
| KNN training compiler, existence-conditional resume, and preparation branch | `src/training.rs:417-498` |
| KNN report payload, model save dispatch, and absence of native training state | `src/training.rs:241-389` |
| KNN branch of public `Train::try_run_with` | `src/training.rs:869-887` |
| Typed reference preparation, target codecs, and masks | `training/src/knn.rs:16-216`, `training/src/knn.rs:218-660` |
| Dense feature span and one-hot lowering used by KNN | `training/src/model.rs:960-1003`, `training/src/model.rs:1145-1240`, `training/src/model.rs:1908-2045` |
| Common vector metadata validation and atomic save | `training/src/checkpoint.rs:5352-5413`, `training/src/checkpoint.rs:10403-10470` |
| Bounded KNN file loading and root-based semantic dispatch | `training/src/inference.rs:696-780` |
| Saved-schema query preparation | `training/src/inference.rs:783-842` |
| KNN feature compilation and saved lowering checks | `training/src/inference.rs:1566-1788`, `training/src/inference.rs:2107-2228` |
| Device normalization for query and reference matrices | `training/src/inference.rs:2588-2799`; scalar programs `training/src/forward.rs:529-596` |
| Compiled KNN execution contract and one-iteration native lifecycle | `training/src/execute.rs:607-655`, `training/src/execute.rs:1312-1427` |
| Boundary, planned exit-output, and typed prediction validation | `training/src/execute.rs:1648-1833`, `training/src/execute.rs:2636-2829` |
| Public KNN report accessors and dictionary decoding | `src/inference.rs:230-423`, `src/inference.rs:432-543` |
| Public KNN row and label reporting | `src/inference.rs:679-695`, `src/inference.rs:990-1058` |
| All-output operation that consumes this artifact's inputs | `ops/src/knn_outputs.rs:21-118`, `ops/src/knn_outputs.rs:157-559` |
| Public workflow and concrete output image | `examples/cookbook.rs:230-243`, `cookbook-knn.ogdl` |
| Normative contract and validator inventory | `system-contract.md:562-587`, `system-contract.md:883-892` |
