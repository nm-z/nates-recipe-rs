# All-output KNN

KNN is Recipe's loss-independent, all-output nearest-neighbor model. The public
declaration is exactly `.knn(neighbors)`. It is a single standalone terminal
model, not a dense block that can be composed with another layer. Training does
not run an optimizer or a native training loop. It prepares an immutable
reference set from the retained training partition and, optionally, writes that
semantic state as a `.ogdl` model. Target-free inference loads that state,
prepares query rows under its saved feature schema, lowers distance and
aggregation into one static Recipe calculation graph, executes exactly one
native loop iteration, and returns one independently typed prediction tensor for
every declared target.

The normative behavior is the KNN contract in `system-contract.md` (C33). The
implementation is split across the public facade (`src/api.rs`,
`src/training.rs`, and `src/inference.rs`), the training crate (`training/src`),
and the reusable operation materializer (`ops/src/knn_outputs.rs`). The data
flow is deliberately one-way:

```text
public data/model declarations
        |
        v
prepare_data -> prepare_knn_reference_set -> KnnModelArtifact
        |                                      |
        | optional .resume merge               +--> save .ogdl
        v                                      |
target-free table -> prepare_knn_inference_table
                   -> compile_prepared_knn_inference
                   -> append_knn_all_outputs
                   -> prepare, realize, execute, exit
                   -> typed per-target predictions and report
```

## Public declaration

`Model::knn(neighbors)` appends `LayerSpec::Knn { neighbors, operations: [] }`
when the nonzero neighbor count validates. A zero count defers an
`InvalidLayer` declaration error. `Model::validate` then enforces all of the
structural KNN rules:

- there is exactly one layer, and it is the KNN layer at index zero;
- the layer has no activation or normalization operations;
- KNN cannot be combined with a dense, tree, recurrent, residual, or any other
  model block;
- KNN cannot be combined with Bayesian dependencies or an inline/loaded weight
  source when the training compiler is selected.

The builder methods make the terminal rule observable immediately. Calling
`.relu()`, `.log()`, `.ln()`, or `.norm(...)` after `.knn(...)` records a
deferred declaration error. The public model therefore has no post-reduction
operation list in a valid run. The artifact still carries an `operations` field
so the codec can preserve and validate the complete semantic image; a nonempty
list from a hand-built or malformed image is rejected by inference compilation
as unsupported topology.

Training data must declare at least one target and an explicit train split.
`Data::split` accepts a finite f64 that remains strictly inside `(0, 1)` after
the f32 narrowing used by ingestion. `Data::norm(z_score)`,
`Data::norm(min_max)`, or `Data::norm(l2_norm)` is optional. Data preparation
does not impute, normalize, or lossy-convert values on the host. The exact
retained training row order is passed to KNN and later defines stable distance
ties.

The training policy is intentionally smaller than a dense training policy. The
KNN compiler rejects an optimizer, learning rate, learning-rate schedule,
warmup, epoch bound, iterative `.log(...)` or `.plot(...)` metric, resume
kernel, or save kernel. A semantic `.resume("model.ogdl")` is allowed, and a
semantic `.save("model.ogdl")` is allowed. The literal two-path forms are
parsed by the public API, but KNN rejects a native second path because no KNN
training kernel exists. Saving only a native `.cubin` or `.hsaco` therefore
returns `TrainingError::Unsupported`.

The cookbook's public example is representative:

```rust
recipe.data("examples/datasets/cookbook/knn.csv")
    .target(["class_target", "numeric_target"])
    .norm(z_score)
    .split(0.75);
recipe.model().knn(3);
recipe.train().save("cookbook-knn.ogdl").run()?;

recipe.data("examples/datasets/cookbook/knn.csv")
    .exclude(["class_target", "numeric_target"]);
recipe.model().load("cookbook-knn.ogdl");
recipe.infer().evaluate()?;
```

The inference declaration is target-free. `src/inference.rs` rejects
`.target(...)`, `.split(...)`, and a redeclared `.norm(...)` for inference;
the saved semantic model supplies target interpretation, reference rows, and
normalization policy. The data source is still distilled and can be narrowed by
the normal target-free column/row exclusions before the saved feature schema is
applied.

## Training and reference preparation

### Public dispatch

`Train::run` takes the remembered `Data` and `Model` declarations and calls
`Train::try_run_with`. After declaration validation, it checks model family in
this order: Bayesian dependencies, then any KNN layer, then dense training. The
KNN branch is:

1. `compile_knn_model` validates the policy, data, and model declarations.
2. If `.resume("path.ogdl")` was declared, it calls `Path::try_exists`. A
   missing path is the normal fresh-start case. An existing path is loaded
   through the bounded KNN decoder.
3. The data declaration is prepared by `prepare_data`, which requires targets
   and the explicit split, infers semantic vectors, applies exclusions, and
   returns the retained train partition without host normalization.
4. The public neighbor count is checked for conversion to `u64` and wrapped in
   `NonZeroU64`.
5. `prepare_knn_reference_set` lowers the exact train partition and creates one
   output for every target in `data.target_source_indices()` order.
6. The data normalization declaration is mapped to
   `DenseDataNormalization::{ZScore, MinMax, L2Norm}` and the (currently empty)
   public KNN operation list is mapped to dense operation tokens.
7. `KnnModelArtifact::new` validates the complete semantic state. If a saved
   artifact was loaded, `continue_with` checks compatibility and appends the
   current references. The saved rows stay first.
8. `TrainingReport::knn` returns the semantic artifact. A declared model save
   calls `KnnModelArtifact::save`; no native preparation, allocation, probing,
   execution, or teardown is entered.

`TrainingReport` exposes `kind() == TrainingModelKind::Knn` and
`knn_model()`. KNN reports deliberately return `None` for dense run and bundle
identities, journal, external output images, native kernels, native execution
evidence, and training evidence. They report no metrics, validation is
`NotRequested`, and `gracefully_stopped()` is false. This is a preparation
report, not a pretend optimizer execution.

### Feature schema and lowering

`DenseFeaturePlan::from_prepared` is shared by dense and KNN feature lowering.
It walks prepared vectors whose role is `Feature` in their prepared source
order, creates contiguous `CompiledFeatureSpan` entries, and rejects a feature
set with no feature vector. KNN supports these feature tuples:

| Prepared feature | Calculation lowering | Span width | Normalized by data normalization |
| --- | --- | ---: | --- |
| Numeric, `I32`, no metadata | one f32 scalar after exact int32-to-f32 conversion | 1 | yes |
| Numeric, `F32`, no metadata | one f32 scalar, finite bits retained | 1 | yes |
| Categorical, `DictionaryI32`, categorical dictionary | one-hot f32 block | dictionary length + 1 | no |

Categorical dictionaries must have nonempty, unique labels. The final one-hot
column is the reserved route for a missing or previously unseen category. The
plan records `dictionary_width` and `reserved_index == dictionary_width`; the
normalization mask has one exact f32 bit per lowered column, `1.0` for numeric
columns and `0.0` for categorical one-hot columns. The mask is `None` when no
categorical feature exists, otherwise it is retained in the artifact and sent
to the inference graph as an external f32 vector.

`lower_dense_features(..., PartitionKind::Train)` iterates each retained train
position paired with its original source row. Numeric int32 values must be
exactly representable as f32, and missing numeric features are an
`InvalidFeatureMatrix` error. Numeric f32 bits must be finite. Categorical
missing values select the reserved route; present codes must be nonnegative and
within the dictionary plus reserved index. Other semantic feature tuples are
not silently coerced. They fail with `InvalidFeatureMatrix` because KNN has no
parallel host-side encoding.

`f32_reference_features` then accepts either the all-numeric `DenseMatrix::I32`
image or the f32 image produced for one-hot data. Int32 values are checked for
exact f32 representation; every f32 bit pattern is checked for finiteness. The
result is a row-major `[reference_rows, feature_width]` vector of exact f32 bit
patterns. No data normalization is applied at this stage. The normalization
declaration and feature mask are saved so inference can apply the same policy
to both query and reference coordinates.

### Target outputs and missingness

`prepare_knn_reference_set` first rejects an empty train partition and an empty
declared target list. It maps prepared target vectors by source index, checks
that each declared source index appears exactly once and exists as a target
vector, and emits outputs in the declaration order. Each
`KnnReferenceOutput` retains the target schema, a per-reference binary known
mask, the count of known references, and one of the following calculation
representations:

| Target tuple | Stored values | Prediction reduction | Decoder |
| --- | --- | --- | --- |
| Numeric, `I32`, no metadata | finite f32 bits, exact conversion required | uniform f32 mean | none |
| Numeric, `F32`, no metadata | finite f32 bits | uniform f32 mean | none |
| Categorical, `DictionaryI32` | canonical int32 dictionary codes | int32 mode | saved byte dictionary |
| Ordinal, `OrdinalI32` | canonical int32 ordered-label codes | int32 mode | saved ordered-label bytes |
| Temporal, `RelativeSecondsI32` | deterministic codes for sorted distinct seconds | int32 mode | saved `KnnLabelValue::I32` values |
| Text, `Utf8` | deterministic codes for sorted distinct byte strings | int32 mode | saved byte dictionary |
| Binary, `Bytes` | deterministic codes for sorted distinct byte strings | int32 mode | saved byte dictionary |
| Image, `Bytes`, image metadata | deterministic codes for sorted distinct encoded bytes | int32 mode | saved byte dictionary |

For numeric targets, a missing row stores positive-zero bits and mask `0`.
Known int32 values are converted to f32 only when the conversion is exact. A
non-finite f32 target, an inexact int32 target, an absent prepared slot, or a
source-row-specific storage failure is reported as `InvalidTargetMatrix` with
the target name and source row in the detail.

For categorical and ordinal targets, the prepared dictionary is validated as
nonempty, unique, and no larger than the int32 code domain. Every known code
must be nonnegative and within that dictionary. Missing rows store code zero and
mask `0`; code zero is not treated as known unless its mask is `1`.

Temporal values are collected from known train rows into a `BTreeSet`, so the
semantic labels are sorted by their exact i32 value. Text, binary, and image
values are collected as byte vectors in a `BTreeSet`, so their labels are
sorted lexicographically. The resulting dictionary receives codes from zero in
that deterministic order. Every known row is remapped through the dictionary;
missing rows again use code zero and mask `0`.

`finish_output` counts mask entries equal to one and rejects an output with no
known reference. Thus missingness is independent per target: a row missing the
class target can still contribute its numeric target, and vice versa. Every
output has exactly `reference_rows` mask entries and value entries, and every
output must retain at least one known value.

### In-memory state

`KnnReferenceSet` is immutable after preparation except for the explicit resume
append. It contains:

- nonzero `neighbors`;
- every prepared vector schema as a `CheckpointArtifactVector`;
- contiguous `CompiledFeatureSpan` lowering metadata;
- the optional f32 normalization mask;
- retained source-row indexes in stable prepared order;
- `reference_rows`, `feature_width`, and the row-major f32 feature bit image;
- one `KnnReferenceOutput` per declared target.

`KnnReferenceOutput::operation_spec` converts numeric values to
`KnnOutputSpec::Numeric { known_references }` and discrete values to
`KnnOutputSpec::Categorical { known_references, classes }`. Its `decode_class`
method is the only semantic decoder for a discrete prediction; numeric outputs
have no class dictionary.

## Semantic `.ogdl` model and resume

### Artifact shape

`KnnModelArtifact` uses format version `1` and the strict root
`recipe-knn-model`. It contains the reference set plus an optional
`DenseDataNormalization` and the retained dense operation list. The canonical
OGDL fields are:

```text
recipe-knn-model
    format-version <integer>
    neighbors <nonzero integer>
    data-normalization <none | z-score | min-max | l2-norm>
    operations
        operation <dense-operation-token> ...
    vectors
        vector
            source-index <integer>
            name-bytes <0x...>
            role <feature | target>
            semantic-type <numeric | temporal | categorical | ordinal | text | image | binary>
            encoding <f32 | int32 | relative-seconds-int32 | dictionary-int32 | ordinal-int32 | utf8 | bytes>
            metadata <none | temporal | categorical | ordinal | image>
    feature-spans
        span
            source-index <integer>
            start <integer>
            width <integer>
            lowering <numeric-scalar | categorical-one-hot ...>
    normalization-mask
        none | f32-bits <0x...>
    reference-shape
        rows <integer>
        feature-width <integer>
    reference-source-rows <fixed-width 0x u64 image>
    reference-feature-f32-bits <fixed-width 0x u32 image>
    outputs
        output
            source-index <integer>
            known-mask <0b...>
            values
                numeric-f32-bits <0x...>
                or discrete-int32
                    codes <0x...>
                    labels
                        int32 <integer>
                        or bytes <0x...>
```

Vector metadata is preserved exactly. Temporal metadata stores Unix seconds
and nanoseconds. Categorical and ordinal metadata stores byte dictionary
entries. Image metadata stores encoded format, dimensions, optional channels
and color model, optional sample bits, and the fixed encoded-file/
encoded-bytes value contract. Feature spans must cover every feature vector in
strict contiguous order from column zero. The normalization mask is encoded as
one f32 bit per lowered column. Source-row indexes are fixed-width hexadecimal
u64 values, while f32 and i32 arrays use fixed-width lowercase hexadecimal
words. Known masks use one binary digit per row. Bytes use lowercase `0x` hex.

`KnnModelArtifact::save` accepts only a `.ogdl` target and uses the shared
atomic save boundary. The artifact contains no native image bytes, journal,
plan, cache, or other exported file.

### Validation on construction and decode

`KnnModelArtifact::new`, `encode`, and `decode` all reach `validate_artifact`.
The validator requires:

- format version one, nonzero reference rows and feature width;
- exactly `reference_rows * feature_width` finite reference feature bits;
- exactly one source-row index per reference row;
- a nonempty vector schema with unique, strictly increasing source indexes,
  unique nonempty names, and valid saved-vector metadata;
- nonempty spans that start at zero, are contiguous and nonzero, use each
  feature source exactly once, and end exactly at `feature_width`;
- numeric spans of width one or categorical one-hot spans whose width is
  `dictionary_width + 1` and whose reserved index equals dictionary width;
- a normalization mask of the declared width containing only exact positive
  zero or one bits;
- at least one distinct target output, each an exact target schema from the
  vector list;
- a binary known mask with one entry per reference row, a nonzero count that
  agrees with `known_references`, finite numeric bits, or nonempty unique
  discrete labels and in-range codes.

The decoder is strict and bounded. `KnnModelDecodeLimits::default()` sets
these finite limits: 1 GiB source bytes, 4,000,000 OGDL nodes, 65,536 vectors,
65,536 feature spans, 100,000,000 reference rows, 1,000,000 feature columns,
65,536 outputs, 1,000,000 labels, 1,000,000 metadata entries, and 1 GiB total
decoded payload. Before graph parsing it checks source length and a newline/
tab node pre-count. It then requires UTF-8, one exact root, known nonduplicate
fields, canonical decimal integers, lowercase `0x` hex, exact array widths,
and canonical `0b` masks. Payload and metadata reservations are counted while
decoding. The final artifact validator converts any inconsistent value into a
path-preserving decode error. There is no fallback to another model decoder.

### Resume semantics

`KnnModelArtifact::continue_with(saved, current)` validates both artifacts,
then requires exact equality of:

- format version, neighbor count, data normalization, and operation topology;
- row-free vector schemas, feature spans, optional normalization mask, and
  feature width;
- output count and target schema identity in declaration order.

Topology or schema drift is an `IncompatibleResume` checkpoint error. The
compatibility check intentionally does not require current source-row indexes to
match saved indexes. Source indexes are local observations, not global row
identities.

For a compatible append, saved rows and their feature bits remain first, then
the current rows are appended in current retained order. Duplicate observations
are retained because multiplicity is statistical weight. Each output appends
its known mask and known count. Numeric values append directly. Discrete output
dictionaries preserve every saved label and code, append previously unseen
current labels at the end, and remap each current code through the merged
dictionary before appending current codes. This keeps saved rows ahead of
current rows for stable distance ties and never recodes an existing saved
label. Checked additions and `try_reserve_exact` calls turn row, feature, mask,
label, and allocation overflow into an incompatible-resume error. The merged
artifact is validated again before it is returned.

An absent `.resume("model.ogdl")` path is not an error and starts a new model.
An existing path must decode and validate as this exact KNN model; a malformed,
wrong-root, over-limit, incompatible, or unreadable model is reported instead
of being silently ignored.

## Device graph materialization

`ops/src/knn_outputs.rs` owns the reusable calculation operation. Its public
request is `KnnAllOutputRequest`:

- `query_features`: rank-two f32 `[queries, dimensions]`;
- `reference_features`: rank-two f32 `[reference_rows, dimensions]`;
- one `KnnOutputRequest` per target;
- the nonzero requested neighbor count;
- a power-of-two reduction lane count in `1..=1024`;
- a caller-owned `IdentityNamespace` for intermediate values and kernels;
- a workspace byte limit.

Numeric output requests carry f32 reference values, an i32 known mask, an f32
`[queries, 1]` prediction tensor, and the known-reference count. Categorical
requests carry i32 reference codes, an i32 known mask, an i32 `[queries, 1]`
prediction tensor, the known-reference count, and a class count. Every boundary
tensor must already exist in the caller graph with the same id, dtype, shape,
layout, and storage byte count. Predictions cannot alias any input boundary.

### Distance and aggregation program

The materializer emits only Recipe calculation primitives. The shared distance
prefix is:

1. Square query features into `[Q, D]` and reference features into `[R, D]`.
2. Reduce query squares over axis one to `[Q, 1]` and reference squares over
   axis one to `[R]`.
3. Contract feature axis one of query and reference matrices into products
   `[Q, R]`.
4. Emit `sqrt(max(query_norm + reference_norm - 2 * product, 0))` into finite
   rooted-L2 distances `[Q, R]`.

The scalar programs require finite input and result values. The maximum with
zero removes a small negative roundoff before the square root. For each output,
the graph then:

1. validates numeric reference values as finite, or validates each categorical
   code in `0..classes`;
2. reduces the i32 known mask and requires that its runtime count equals the
   saved `known_references` count;
3. validates that every mask entry is exactly zero or one and replaces unknown
   distances with positive infinity;
4. performs a stable ascending sort by distance, emitting row indexes;
5. gathers the first `effective_neighbors = min(neighbors, known_references)`
   indexes in stable order;
6. aggregates independently by output type.

Numeric aggregation gathers f32 values into `[Q, effective_neighbors]`, sums
along the neighbor axis to `[Q, 1]`, divides by the exact effective-neighbor
count, and requires a finite result. It is an unweighted mean. Categorical
aggregation gathers i32 codes, forms `row_base + code` bin indexes, builds an
unweighted relaxed-ordering histogram with `Q * classes` bins, gathers a
`[Q, classes]` count matrix, stable-sorts counts descending with class indexes,
and gathers the first class code into `[Q, 1]`. The class index map starts at
zero, so stable descending count ties retain ascending class code and choose
the lowest canonical code. No host vote, mean, distance, or tie decision is
performed.

### Resource and identity contract

`knn_all_output_requirements` computes all extents with checked u64 arithmetic
before materialization. Let `Q = queries`, `R = reference_rows`, `D = dimensions`,
`K = effective_neighbors`, and `C = classes` for one categorical output. The
base workspace element count is:

```text
Q*D + R*D + Q + R + 2*Q*R
```

Each output adds common workspace:

```text
2 + 3*Q*R + K + Q*K
```

and then either:

```text
numeric:     R + Q*K + Q
categorical: R + 2*Q*K + Q + 5*Q*C + 1
```

Workspace bytes are the checked element total multiplied by four. The materializer
reserves six base intermediate values and six base kernels, then ten values and
ten kernels per numeric output or seventeen values and sixteen kernels per
categorical output. It verifies the emitted counts and byte total against this
formula before returning the graph.

The request rejects zero query/reference/dimension/neighbor extents, no
outputs, reference rows above the checked i32 sort-index domain, known counts
outside `1..=R`, numeric effective-neighbor counts above the exact f32 integer
domain (`16,777,216`), class counts outside `1..=i32::MAX`, and categorical
histograms above the checked i32 bin domain. Every product and sum reports
`WorkspaceArithmeticOverflow` rather than wrapping.

`validate_resources` checks the caller's value and kernel identity capacities
and workspace limit. `KnnAllOutputEmitter` rejects namespace endpoint overflow,
boundary overlap, exhausted identity ranges, and workspace arithmetic overflow.
It gives every generated primitive a forbidden input/output alias rule. The
finished graph is validated before `append_knn_all_outputs` checks caller graph
identity and output-producer overlap and appends only the generated intermediate
tensors and nodes.

The operation-level failure kinds used by KNN are
`UnsupportedConcreteShape`, `InvalidMaterializationRequest`,
`IdentityNamespaceOverlap`, `IdentityNamespaceExhausted`,
`WorkspaceLimitExceeded`, `WorkspaceArithmeticOverflow`,
`WorkspaceFormulaMismatch`, and `GraphMaterializationFailed`. They are wrapped
by the training or inference compile error at the public boundary.

## Target-free inference preparation

### Model loading and schema binding

`load_knn_model_file` reads a regular file through a bounded source snapshot,
then calls the strict KNN decoder. `load_semantic_model_file` admits one bounded
`.ogdl` snapshot, probes only its first line, and dispatches the exact
`recipe-knn-model` root to `decode_knn_model`; unknown roots are errors and no
decoder fallback occurs.

`prepare_knn_inference_table` derives `InferenceFeatureSchema` entries from the
saved vector list and feature spans. A saved numeric scalar must pair with a
numeric I32 or F32 vector with no metadata. A saved categorical one-hot span
must pair with a categorical dictionary-I32 vector whose dictionary width,
reserved route, and span width agree exactly. Any other saved tuple is an
`InconsistentCheckpoint` preparation error with the feature and source-vector
identity. The resulting schema is passed to
`recipe_ingest::prepare_inference_table`, which reads query rows under saved
feature names and encodings. `validate_prepared_feature_spans` then checks
feature count, source identity, row count, encoding, value storage, and span
width before a `PreparedKnnInference { artifact, data }` is returned.

The preparation object retains unnormalized target-free rows and the decoded
artifact. It does not do host one-hot expansion, normalization, distance work,
or target decoding. `load_and_prepare_knn_inference` combines bounded model
loading with this table preparation.

### Compiling the static KNN program

`compile_prepared_knn_inference` is the sole KNN inference compiler. It first
rejects a nonempty artifact operation list, requires at least one query row,
and checks that the saved reference rows and feature width convert to u64 and
are nonzero. It then:

1. lowers query feature columns through the saved spans into an f32 row-major
   matrix `[Q, D]`. Numeric I32 columns use an explicit checked conversion;
   numeric F32 columns retain their admitted bits; categorical columns use a
   device one-hot scatter with the saved reserved route. The compiler rejects
   noncontiguous spans, mismatched source identities, wrong row counts, widths
   that overflow, and element totals outside the checked i32 index domain;
2. creates an external f32 `[R, D]` reference tensor from the artifact's exact
   unnormalized feature bits;
3. applies the optional KNN data normalization to both query and reference
   matrices, using the saved feature mask when present;
4. creates one external i32 known mask `[R]` and one external reference-value
   vector `[R]` per output. Numeric values are f32; discrete codes are i32;
5. creates one fresh f32 or i32 prediction tensor `[Q, 1]` per output and an
   exact `KnnInferenceOutputContract` containing value id, dtype, shape, saved
   target source index, and aggregation kind;
6. computes operation requirements and asks `append_knn_all_outputs` to emit
   the distance, selection, and aggregation graph in one identity namespace;
7. marks exactly the declared feature/reference/mask/value tensors as external
   inputs and exactly the prediction tensors as external outputs, validates the
   graph, serializes and reparses it, and wraps it in a
   `StaticCalculationProgram` with exactly one iteration.

The resulting `CompiledKnnInference` exposes the graph, program, external
inputs, output contracts, and query row count. It has no metrics and no
optimizer or checkpoint parameter state.

### Normalization in the graph

KNN stores the normalization declaration and feature mask, not a precomputed
host-normalized image. `InferenceGraphCompiler::normalize_knn_features` keeps
the identity path when no normalization is declared. For a categorical mask,
normalization calculations run only on columns whose mask is positive; the
categorical one-hot values pass through unchanged.

The scalar programs use `EPSILON = 1.0e-6`:

- **Z-score:** reduce the saved reference matrix by columns, divide sums by
  `R` for means, center reference values, square and reduce again for variance,
  divide by `R`, then apply `(value - mean) / sqrt(max(variance, epsilon))` to
  query and reference matrices. Masked columns retain their original values.
- **Min-max:** reduce reference minimum and maximum by columns, apply
  `(value - minimum) / max(maximum - minimum, epsilon)` to query and reference
  matrices, and leave masked columns unchanged.
- **L2 norm:** square each row, reduce across columns, apply
  `value / sqrt(max(row_norm_squared, epsilon))` to query and reference rows,
  and leave masked columns unchanged.

All reductions use the requested fixed tree lanes. The normalization tensors
are calculation intermediates, not additional external boundaries or exported
artifacts.

## Native execution lifecycle

The public inference facade (`src/inference.rs`) validates the target-free
policy, loads the semantic model, distills and selects the query table, binds
the KNN schema, and compiles the static program. Native execution then uses the
same measured preparation boundary as other Recipe inference families:

1. `execute_current_inference_native` allocates a run id, obtains the current
   measured profile and target plan, derives host runtime tuning from the KNN
   graph, and constructs the production native candidate factory and preparer.
2. The KNN branch calls `prepare_and_execute_local_knn_inference`.
3. The executor validates the compiled boundary, prepares placement and native
   artifacts, and requires `LoopIterations::ONE`. Loop-time external transfers
   and user metric emissions are rejected.
4. `build_knn_inference_device_images` packs every declared external input into
   finalized init images. The image boundary checks dtype, shape, byte count,
   uniqueness, and device membership.
5. The output mapper requires one exit-phase device-to-external transfer per
   prediction. It checks task identity, source device/value, dtype, shape byte
   count, and nonoverlapping output locations.
6. The native session is handed to `PreparedRun`. Initialization admits the
   external images once, the one loop iteration is started, and polling waits
   until `LoopStatus::Complete` with bounded backoff.
7. Exit is entered and teardown is performed through the recoverable executor
   lifecycle. External exit images are collected and checked against the output
   contracts. Native evidence, elapsed loop time, run and bundle identities, and
   the complete journal are retained in `CompletedKnnInferenceExecution`.

The executor never treats KNN as a dense training run. It cannot stop after a
partial iteration, admit an extra loop transfer, or return before terminal loop
completion. Native preparation, artifact realization, allocation, loading,
waking, and teardown are lifecycle work around the one immutable calculation
program, not KNN model semantics.

`validate_compiled_knn_inference_boundary` is deliberately strict. It requires
one iteration, no metrics, a valid calculation graph, unique allowed external
input roles, exact typed input shapes and byte counts, equality between the
declared and graph input sets, at least one output, unique output value and
target identities, output dtype and shape `[Q, 1]`, external-output-only tensor
contracts, a calculation producer for every output, and equality between the
declared and graph output sets. Any mismatch is an
`InferenceExecutionError::InvalidInferenceBoundary` or its more specific
duplicate, byte-size, image, or output-source error.

## Public inference report and output interpretation

The facade represents a completed KNN run as
`InferenceReportPayload::Knn { artifact, execution }` and
`InferenceModelKind::Knn`. `InferenceReport::prediction()` is `None` because
there is no single homogeneous matrix. `knn_predictions()` returns the
independently typed prediction images in saved target declaration order.
`values()` intentionally yields an empty iterator because outputs may mix f32
and i32. Use each `KnnInferencePrediction`'s contract and little-endian bytes.

Each `KnnInferenceOutputContract` states:

- the graph value id;
- f32 or i32 dtype;
- shape exactly `[query_rows, 1]`;
- saved target source-vector identity;
- `NumericMean` or `DiscreteMode` aggregation kind.

`InferenceReport::decode_knn_class(output, code)` resolves a discrete code with
the corresponding saved output dictionary. Numeric outputs and non-KNN reports
return `None`. The report exposes run, bundle, journal, elapsed time, devices,
and native evidence, but deliberately exposes no retained native-kernel set for
KNN.

`write_knn_prediction_rows` checks that the number of returned images equals
the number of saved target outputs and that each contract has the expected
saved source identity. For every row and output it writes the saved target name
and either a finite numeric value or `class <code> label <decoded-value>`. A
discrete prediction with an unknown code is an invalid-data error rather than a
made-up label. Byte labels are emitted with the shared quoted-byte writer;
temporal labels are emitted as their exact i32 value.

## Invariants and error boundaries

The important failure classes are intentionally stage-specific:

| Stage | Main guards | Public error family |
| --- | --- | --- |
| Declaration | zero neighbors, KNN composition, post-KNN operation, invalid save/resume extension or duplicate declaration | `DeclarationError` wrapped by `TrainingError::Declaration` or `InferenceError::Declaration` |
| Data preparation | missing target, missing split, source/framing/semantic/selection/encoding errors | `DataPreparationError` |
| Reference feature preparation | unsupported feature tuple, missing numeric value, inexact int32-to-f32, nonfinite f32, categorical dictionary/code failure, shape overflow | `TrainingCompileErrorKind::InvalidFeatureMatrix`, `ArithmeticOverflow`, `EmptyDataset` |
| Reference target preparation | duplicate/missing target identity, unsupported semantic tuple, missing storage, invalid dictionary/code, no known reference | `TrainingCompileErrorKind::InvalidTargetMatrix` |
| KNN policy | objective, gradient, optimizer, learning policy, metrics, kernel resume/save, or loaded model declaration | `TrainingError::Unsupported` |
| Model codec | wrong root/version, malformed OGDL, noncanonical scalar/hex, decode limit, schema/span/mask/output inconsistency, nonfinite payload | `CheckpointError` or `InferencePreparationError` |
| Resume | changed neighbors, normalization, operation topology, row-free schema, feature lowering, target order/schema, or checked append overflow | `CheckpointError::IncompatibleResume` |
| Inference table | saved schema tuple mismatch, feature identity/order/width/value mismatch, query rows absent | `InferencePreparationError::InconsistentCheckpoint` or data preparation error |
| Inference compilation | empty query/reference shape, nonempty post-KNN operations, u64/i32/shape overflow, graph/language/program failure | `InferenceCompileErrorKind` |
| Operation materialization | wrong tensor rank/dtype/shape, mask/count/code violation, unsupported extents, alias/identity/workspace conflict, graph or formula failure | `OperationErrorKind` wrapped by `InferenceCompileError` |
| Native boundary | external image mismatch, missing or duplicate input/output, wrong output transfer, overlap, loop/metric violation, handoff or executor failure | `InferenceExecutionError` wrapped by `InferenceError::Execute` |
| Reporting | wrong output count/source identity, invalid byte width, unknown discrete class | I/O error from `write_knn_prediction_rows` or checked report access |

There are no fallback decoders, alternate host algorithms, imputation paths,
retry loops, substitute output types, or compatibility shims. A valid active
KNN state determines the only legal inputs, calculations, transfers, and
transitions; an invalid state fails at the boundary that first observes it.

## End-to-end role

KNN supplies a semantic bridge between heterogeneous supervised targets and a
single target-free native query program:

1. Data preparation establishes exact feature and target semantics and an
   explicit retained train partition.
2. Reference preparation freezes row order, feature lowering, normalization
   mask, target-specific missingness, and exact decoders.
3. The versioned `.ogdl` image makes that semantic state portable and resumable
   without exporting any execution artifact.
4. Inference binds new rows to the saved feature schema, computes optional
   normalization from saved references, and emits one device calculation graph
   containing shared distances plus independent per-target reductions.
5. The native lifecycle admits all immutable inputs once, executes one complete
   iteration, transfers each typed output at exit, and tears down cleanly.
6. Public reporting preserves target order, dtype, source identity, and exact
   dictionary decoding instead of flattening mixed outputs into a misleading
   f32 matrix.

Thus the model's learned state is precisely the retained observations and their
semantic schema. The runtime owns distance, stable selection, uniform mean,
mode, tie, finite-value, resource, and lifecycle rules; it does not invent an
optimizer, a hidden target, a second public KNN form, or a native training
kernel.

## Source map

- `src/api.rs`: `Data`, `Model::knn`, terminal-layer validation, normalization,
  split, save, and resume declarations.
- `src/data_prepare.rs`: bounded public data loading and train/target-free table
  preparation.
- `src/training.rs`: KNN family dispatch, policy rejection, optional resume,
  semantic model construction, `TrainingReport`, and model saving.
- `training/src/knn.rs`: feature-plan use, row-ordered reference features,
  target representations, known masks, label dictionaries, and semantic state.
- `training/src/knn_checkpoint.rs`: format-v1 OGDL codec, finite decode limits,
  artifact validation, atomic save, and compatible append/resume.
- `training/src/inference.rs`: KNN model loading, saved-schema table binding,
  normalization lowering, static graph compilation, contracts, and one-iteration
  program construction.
- `ops/src/knn_outputs.rs`: shared pairwise distance, stable neighbor
  selection, numeric mean, categorical mode, checked resource formulas, and
  graph materialization.
- `training/src/execute.rs`: external image packing, strict graph/output
  boundary checks, native one-iteration lifecycle, exit collection, and typed
  execution evidence.
- `src/inference.rs`: public semantic-root dispatch, target-free policy,
  measured native preparation, KNN report payload, dictionary decoding, and
  prediction-row writing.
- `training/src/lib.rs` and `src/lib.rs`: public exports for KNN artifacts,
  preparation, compilation, execution, contracts, and reports.
