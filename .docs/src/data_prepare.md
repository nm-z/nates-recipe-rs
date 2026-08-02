# `src/data_prepare.rs`

## Purpose and boundary

`data_prepare` is the root-crate adapter between the public immutable `Data`
declaration and the dependency-clean `recipe_ingest` preparation types. It is
the only root-level boundary that turns declared filesystem paths into either
an immutable `PreparedDataset` for training or a bounded `DistilledDataset` /
`RawTable` pair for target-free inference.

The module performs preparation-time work only:

1. Validate the public declaration and translate its strings, split, exclusions,
   and typed conditions into ingest requests.
2. Read each declared source once under finite bounds, recursively distill its
   structural records into one ordered rectangular `RawTable`, and retain the
   parser-owned semantic rules.
3. Infer semantic types, fit lossless encodings and metadata, select rows and
   columns, and construct typed partitions when the caller requests training
   data.

It does not probe hardware, compile kernels, allocate device memory, execute a
model, normalize values, impute missing values, or retain an open filesystem
handle. The returned values are owned, closed-input preparation state. Numeric
normalization is a later model calculation; the `Data::normalization` field is
not read by this module.

The normative companion is `system-contract.md` C31 (bounded ordered data
distillation and preparation) and C27 (target-free inference). The descriptions
below follow the current call graph and identify where an implementation detail
differs from the intended train-only semantic fit.

## Public surface

All names in this table are re-exported by `src/lib.rs`.

| Symbol | Inputs | Output | Policy | Filesystem work |
| --- | --- | --- | --- | --- |
| `prepare_data` | `&Data` | `DataPreparationResult<PreparedDataset>` | `default_data_limits()` | yes |
| `prepare_data_with_limits` | `&Data`, caller-owned `IngestLimits` | `DataPreparationResult<PreparedDataset>` | targets, explicit split, exclusions, predicates, semantic inference, train/validation partitions | yes |
| `distill_data` | `&Data` | `DataPreparationResult<DistilledDataset>` | default limits, no target/split policy | yes |
| `distill_data_with_limits` | `&Data`, `IngestLimits` | `DataPreparationResult<DistilledDataset>` | source validation and ordered distillation only | yes |
| `select_target_free_data` | `&Data`, already distilled dataset | `DataPreparationResult<RawTable>` | row and column exclusions only; no target, semantic fit, split, or normalization | no additional source read |

`DataPreparationResult<T>` is `Result<T, DataPreparationError>`. The module
exports four default bounds:

| Constant | Value | Unit and scope |
| --- | ---: | --- |
| `DEFAULT_DATA_SOURCE_BYTES` | `1,073,741,824` | aggregate leaf and expanded source bytes, 1 GiB |
| `DEFAULT_DATA_RECORDS` | `10,000,000` | framed records/samples, including a table header at the table parser boundary |
| `DEFAULT_DATA_FIELDS_PER_RECORD` | `16,384` | aggregate vectors/columns in the distilled table |
| `DEFAULT_DATA_FIELD_BYTES` | `16,777,216` | one textual field, 16 MiB |

`default_data_limits()` constructs `recipe_ingest::IngestLimits` from these
values. `IngestLimits` rejects zero bounds, so a default-construction failure is
reported as `DataPreparationError::Ingest` before a source is opened.

## Declaration capture and validation

### `Recipe::data` to `Data`

`Recipe::data(sources)` in `src/facade.rs` accepts one string, an array, a
slice, or a vector. It creates `Data::empty()`, calls `Data::set` once per
source in the caller's order, remembers the declaration in the thread-local
recipe sequence, and returns the value. `Data::set` appends a nonempty path and
defers the first `EmptyValue` error for an empty path. No source is read by a
builder call.

The subsequent immutable builders operate as follows:

| Builder | Stored state | Validation at builder time | Ordering/replacement |
| --- | --- | --- | --- |
| `.set(path)` | `sources: Vec<String>` | path must be nonempty | appends exactly one source |
| `.target(value)` | `targets: Vec<String>` | nonempty collection and names | replaces the previous target vector; array order is retained |
| `.exclude(value)` | column glob strings or `Condition` values | nonempty pattern; condition column nonempty and float value finite | appends each exclusion |
| `.split(f64)` | exact bits of `f64 as f32` in `split_fraction_bits` | original and narrowed values finite and strictly in `(0, 1)` | replaces the previous split |
| `.norm(kind)` | `Option<DataNormalization>` | enum has no additional validation | replaces the previous normalization |

`ConditionValue` preserves signed and unsigned integers, exact floating-point
bits, booleans, and text. The `cond!` macro creates a `Condition` without
evaluating a column as a Rust value. Every builder calls
`remember_recipe_data`, so the facade's later `train().run()` or
`infer().evaluate()` sees the latest immutable declaration.

`Data::validate()` is intentionally small. It first returns the first deferred
builder error, then rejects an empty source list. It does not require targets,
split, or normalization because target-free inference is valid without them.
`prepare_data_with_limits` adds the training-only checks for targets and split;
`compile_inference_package` adds the target-free policy checks for inference.

## Public training path

`prepare_data` is a two-step wrapper:

```text
prepare_data(data)
  -> default_data_limits()
  -> prepare_data_with_limits(data, limits)
```

`prepare_data_with_limits` is ordered and fail-closed:

```text
Data::validate
  -> require at least one target
  -> require Data::split_fraction()
  -> TrainFraction::from_f32
  -> map column exclusions to ColumnPattern
  -> map condition exclusions to RowPredicate
  -> build PreparationRequest
  -> distill_data_with_limits
  -> DistilledDataset::infer_vectors(CategoricalEncodingModel)
  -> prepare_inferred_table
  -> PreparedDataset
```

No later step is entered after an error, and no partial dataset escapes.
Declaration, target, split, fraction, pattern, and condition failures therefore
occur before source reading. Source and semantic failures occur before typed
preparation. `DataPreparationError` preserves that stage through its variant.

### Translating the preparation request

`PreparationRequest::new` receives target names as owned UTF-8 bytes and the
exact `TrainFraction`. Each exclusion pattern is passed to
`ColumnPattern::new(pattern.as_bytes().to_vec())`. The pattern language is a
case-insensitive byte glob: `*` matches zero or more bytes and `?` matches one
byte. A pattern that matches no source header fails; matching a target fails;
matching a non-target excludes it.

`map_condition` maps the public and ingest operator enums one-to-one:

| Public operator | Ingest operator |
| --- | --- |
| `Equal` | `Equal` |
| `NotEqual` | `NotEqual` |
| `Less` | `Less` |
| `LessOrEqual` | `LessOrEqual` |
| `Greater` | `Greater` |
| `GreaterOrEqual` | `GreaterOrEqual` |

Literal conversion is loss-aware:

| `ConditionValue` | `PredicateLiteral` | Comparison domain |
| --- | --- | --- |
| `Signed(i64)` | `Signed(i64)` | numeric `I32` |
| `Unsigned(u64)` | `Unsigned(u64)` | numeric `I32` |
| `FloatBits(u64)` | `F32Bits(u32)` after `f64` to `f32` | numeric `F32` |
| `Boolean(bool)` | UTF-8 text `"true"` or `"false"` | categorical, ordinal, or text |
| `Text(String)` | UTF-8 text | categorical, ordinal, or text |

The float branch returns `FloatPredicateOutsideF32` when the narrowed value is
non-finite or a nonzero source value underflows to `f32` zero. It permits normal
finite rounding to an `f32`; row evaluation later uses the contract f32 parser.
Missing predicate values are errors, not nonmatches. Multiple row predicates
are combined with logical OR by `recipe_ingest::select_table` and the table
preparation path.

`TrainFraction::from_f32` reconstructs the exact binary rational represented by
the stored f32 bits, reduces numerator and denominator by their greatest common
divisor, and computes `floor(retained_rows * numerator / denominator)`. Rows are
never shuffled or randomized. Selection happens before this calculation, so
excluded rows cannot consume a train position. A valid fraction can still
produce zero training rows for a small retained dataset; training consumers then
reject the empty training partition.

## Source distillation

`distill_data` and `distill_data_with_limits` validate the declaration and call
`recipe_ingest::distill_datasets` with the declared source paths in order. The
ingest crate owns all filesystem, archive, format, and aggregate-bound logic.

### Traversal and admission

The source traversal contract is:

1. At least one path is required. A path must be a regular file or directory;
   the root and every recursive directory entry reject symbolic links.
2. Directory entries are sorted by filename before recursion. Regular files
   are read once through `read_source_snapshot`, whose handle is closed before
   the bytes are returned. Metadata is checked first and the read is capped at
   `limit + 1`, so a concurrently growing file cannot bypass the bound.
3. A ZIP file, or any file with a ZIP signature, is recursively opened. Archive
   members are sorted by enclosed path, directory entries are skipped, and
   `enclosed_name()` rejects absolute or parent-escaping paths. A member's
   declared and expanded size is bounded before it is visited. Nesting at or
   beyond `ARCHIVE_DEPTH_LIMIT = 32` fails.
4. `Accumulator::admit_leaf` adds every regular leaf's bytes to one aggregate
   counter, applies the source-byte bound, and increments the file count.
   `Accumulator::append` applies aggregate record and vector limits across all
   declared sources and all expanded members. Compression and source splitting
   therefore cannot bypass admission.
5. Every logical table is merged into one global header map. New headers extend
   prior rows with missing bytes. A duplicate header within a logical table gets
   a `#2`, `#3`, ... suffix. If a source-context name collides with a data name,
   the data name is prefixed with `data:`.
6. A logical table with no rows is represented by one all-missing row so an
   admitted empty file still has a sample. The final accumulator rejects a
   source set with no admitted files or rows and constructs a rectangular
   `RawTable`.

Source context is included for every directory member and archive member, and
for every member when more than one source is declared. A single regular file
does not receive context columns. Context is ordinary data, not hidden control
state. The 14 context vectors are:

```text
source_index, source_path, parent_path, folder, file_name, file_stem,
extension, format, member, content_sha256, file_bytes, sample_index,
sample_count, source_depth
```

`source_index`, `sample_index`, `sample_count`, and `source_depth` are exact
numeric `I32` vectors. `source_index` is the zero-based declaration index and
`sample_index` is zero-based within the logical table. `source_path`, folder
names, format, member names, and the 64-character SHA-256 digest are classified
text. `source_index` is exact dictionary-categorical. The context metadata is
included in the vector limit and can itself become a feature unless excluded.

### Format dispatch

`visit_bytes` handles `.xlsx` and `.pptx` before ZIP recursion, recursively
handles ZIP signatures, and otherwise calls `parse_leaf`. The leaf dispatch is:

| Input | Logical output |
| --- | --- |
| `.csv` | delimited table, auto delimiter, header present |
| `.tsv` | tab-delimited table, header present |
| `.all-data`, `.dat`, `.data`, `.data-numeric`, `.tra`, `.trn` | delimited table, header absent |
| `.json` | one or more JSON-derived tables |
| `.gguf` | metadata and tensor tables, or one binary payload table |
| `.safetensors` | metadata and tensor tables, or one binary payload table |
| `.png` with PNG signature | one exact image/bytes vector |
| `.inp`, `.out`, `.patch`, `.sh` | one exact UTF-8 text vector |
| `.txt` with a structural delimiter | delimited table, header absent |
| `.txt` otherwise | one exact UTF-8 text vector |
| `.bin`, `.logits`, `.model` | one exact binary vector |
| any recognized image signature | one exact image/bytes vector |
| other probable text with a structural delimiter | delimited table, header absent |
| other probable UTF-8 text | one exact text vector |
| all other bytes | one exact binary vector |

Delimited parsing uses `recipe_ingest::parse_table`. Auto detection examines
quoted commas, semicolons, tabs, and ASCII whitespace. CSV framing is strict
and rectangular; whitespace framing ignores blank lines. Source, record, field,
and field-byte limits are checked while framing. A present header consumes the
first framed record but the record limit includes it. Header-absent tables get
`col1`, `col2`, ... names.

JSON objects and arrays are losslessly flattened to bytes: object arrays become
the union of object keys, arrays become `col1` through `colN`, scalars become a
`value` vector, and object-key/value forms add a classified `key` vector.
`null` is missing, booleans and numbers use their JSON text, strings use UTF-8,
and nested arrays/objects use their JSON text. JSON object maps that contain
multiple object-array members can produce one table per member; split-shaped
objects produce one table per nested member with the outer key as a `key`
column.

XLSX is bounded by a ZIP expansion preflight. Each nonempty worksheet becomes a
table. If the first row consists only of numeric/date/bool cells it is data and
headers are generated; otherwise it is the header row. Later rows are padded or
truncated to the header width. PPTX is also expansion-preflighted; each slide
paragraph becomes a row with exact numeric `slide` and exact UTF-8 `text`
columns, in slide-number order.

GGUF and safetensors parsers validate container structure and retain encoded
tensor bytes. They do not decode model tensors into host f32 values. GGUF emits
`metadata_key`, `metadata_type`, `metadata_value` and, when present,
`tensor_name`, `tensor_type`, `tensor_shape`, `tensor_rank`, `tensor_bytes`,
`binary`. Safetensors emits the same tensor columns and a two-column metadata
table. `tensor_rank` and `tensor_bytes` are exact `I32`; tensor `binary` is
opaque `Bytes`. An archive with neither metadata nor tensors falls back to one
opaque binary payload.

Recognized image headers include PNG, JPEG, GIF87a/GIF89a, BMP, and WebP. The
payload remains the original encoded file bytes. Preparation records validated
format, dimensions, channels, color model, and sample precision as image
metadata; it never claims to have decoded pixels.

## Semantic inference

`DistilledDataset` owns the `RawTable`, one `VectorSemanticRule` per global
column, and the admitted file count. A parser can mark a column `Exact`,
`Classify`, or `Infer`; if merged sources provide conflicting rules for one
header, the accumulator resets that header to `Infer`.

`DistilledDataset::infer_vectors(&CategoricalEncodingModel)` checks every row
width, collects exact byte evidence, and produces one `InferredVector` per
source column. For an `Infer` rule, classification is parser-first:

1. Every present value with a recognized image signature becomes
   `Image/Bytes`.
2. Every present value accepted by the temporal parser becomes
   `Temporal/RelativeSecondsI32`.
3. A recognized ordinal vocabulary becomes `Ordinal/OrdinalI32`.
4. Values that all pass the nine-significant-digit exact int32 validator become
   `Numeric/I32`.
5. Values that all pass the finite six-significant-digit f32 validator become
   `Numeric/F32`.
6. Remaining columns go to `CategoricalEncodingModel`.

`Classify` rules skip steps 1 through 5 and ask the model directly.
`Exact` rules retain the parser-provided semantic type and encoding. The
encoding map is deterministic: categorical to dictionary I32, text to UTF-8,
numeric to F32, temporal to relative-seconds I32, ordinal to ordinal I32, and
image/binary to opaque bytes.

The ambiguous model uses complete-column evidence: value count, missing count,
unique count, UTF-8 count, whitespace count, total bytes, dictionary bytes,
mean width, and integer ratios in thousandths. Non-UTF-8 present values are
categorical. Otherwise a fixed nearest-example table chooses categorical or
text using cardinality, whitespace, mean width, and whether a dictionary is the
smaller lossless representation. It never invents a feature or performs host
numeric calculation.

## Selection, fitting, and typed state

`prepare_inferred_table` is the lower ingest boundary used by
`prepare_data_with_limits`. It first verifies that the supplied
`InferredVectorList` has exactly the table width and that every vector's index
and name match the corresponding source header. It then performs the following
ordered operations:

1. Build a unique header-name index.
2. Resolve every declared target in declaration order; reject an empty target
   set, an unknown target, or a duplicate target.
3. Resolve exclusion globs against the original headers. Exclusions are applied
   after target and predicate resolution; a target cannot be excluded, and all
   columns being targets or excluded is `NoFeatureVectors`.
4. Resolve predicates against the original headers and inferred semantic tuple.
   Signed/unsigned literals require numeric I32, f32 literals require numeric
   F32, and text literals require categorical, ordinal, or text semantics.
5. Evaluate each predicate against the original row bytes before removing
   helper columns. Numeric values use strict UTF-8 decimal parsing; text values
   use UTF-8 lexical ordering. A missing value, invalid UTF-8, invalid decimal,
   or type mismatch is an error. Predicates OR together. Retained and excluded
   source-row indexes preserve original order.
6. Compute the exact train row count from the retained row count and split
   fraction. The first `train_rows` retained positions form train; the suffix
   forms validation.
7. Fit encoding metadata from the train source rows, then apply that schema to
   every retained row without refitting.

The last two steps are represented by `fit_vector_schema` and
`apply_vector_schema` in `recipe_ingest::prepare`. The public adapter supplies
the already inferred list, so semantic parser/classifier discovery has happened
before this fit step. The lower-level `DistilledDataset::prepare` API instead
fits semantic rules from the exact train partition; it is not the path called by
`prepare_data_with_limits` today.

### Prepared vector encodings

| Encoding | Storage | Metadata fit | Failure/limit behavior |
| --- | --- | --- | --- |
| `I32` | `PreparedValues::I32(Vec<Option<i32>>)` | none | every nonmissing value must pass exact int32 decimal validation |
| `F32` | `PreparedValues::F32Bits(Vec<Option<u32>>)` | none | every nonmissing value must be finite, non-underflowing, and pass the f32 round-trip contract |
| `RelativeSecondsI32` | optional i32 seconds from a retained origin | minimum temporal instant from train values | every value must parse, differ by a whole second, and fit i32 |
| `DictionaryI32` | known code, reserved unseen code, or missing | sorted byte dictionary from nonmissing train values | dictionary and reserved code must fit i32; unseen labels retain exact bytes in `CategoricalObservation` |
| `OrdinalI32` | ordinal rank or missing | one unambiguous recognized vocabulary from train values | unknown labels or ambiguous fit vocabulary fail |
| `Utf8` | `VariableWidthVector` offsets, payload, validity | none | nonmissing values must be UTF-8 |
| `Bytes` | `VariableWidthVector` offsets, payload, validity | image variants when semantic type is image | image values are header-validated; opaque binary is retained unchanged |

Missing fixed-width values are `None`. A variable-width vector has one validity
bit per retained row and `len + 1` u64 offsets; empty input is missing rather
than an empty present payload. Categorical vectors also carry one typed
`CategoricalObservation` per row: `Known { code }`, `Missing`, or
`Unseen { label }`. The calculation-facing reserved code is aligned with the
unseen route, but the observation route preserves the distinction and original
bytes.

### `PreparedDataset` invariants

`PreparedDataset` contains:

```text
source_row_count             original RawTable row count
retained_source_rows         retained position -> original source row
excluded_source_rows         original rows removed by predicates
vectors                      source-order PreparedVector values and schemas
target_source_indices        target source indexes in declaration order
train                        retained positions and original source rows
validation                   remaining retained positions and source rows
```

Every prepared vector has exactly the retained-row length, and every vector
retains its original source index and name. Target vectors remain distinct and
retain declaration order even when source columns are physically interleaved.
`PreparedDataset::fixed_dense_matrix` is a later explicit projection: it refuses
variable-width values, missing values, and lossy integer-to-f32 conversion. It
returns row-major I32 when all selected vectors are I32, otherwise row-major
F32 bits with exact integer conversion only.

Preparation does not normalize, impute, drop a row because a value is missing,
or silently cast a value. Those choices remain visible to the downstream model
compiler.

## Target-free inference path

`src/inference.rs::compile_inference_package` handles inference separately from
training:

```text
Infer::validate
  -> Data::validate
  -> require no Data::targets()
  -> require no Data::split_fraction()
  -> require no Data::normalization()
  -> load .ogdl or .gguf model
  -> distill_data(data)
  -> select_target_free_data(data, distilled)
  -> model-specific recipe_training::prepare_*_inference_table
  -> compile model graph
```

`distill_data` preserves source order, parser-owned semantic rules, and all
source/container context. `select_target_free_data` translates only column
patterns and row conditions, then calls `recipe_ingest::select_table`. The
selection operation does not infer types, fit dictionaries, split rows, or
consult targets. Predicates still run before excluded columns are removed, so a
helper column can select rows and then be excluded.

The function does not call `Data::validate()` itself; the normal inference
caller validated the declaration before calling it. A direct caller that passes
an already distilled value is responsible for that precondition.

After selection, the saved model is authoritative. Dense checkpoints resolve
their saved feature names and encodings, KNN uses its saved reference schema,
Bayesian inference requires saved dictionary-coded parent features, and GGUF
Llama preparation requires its saved token feature contract. Source columns may
arrive in a different order because the saved schema resolves them by name, but
missing, ambiguous, or invalid query values fail in the model-specific
inference preparation layer, not by silently refitting the query table.

`InferenceError::Data` wraps every `DataPreparationError` from distillation or
selection. Native probing, graph compilation, allocation, and execution are
later `InferenceError` variants and are not part of this module.

## Training consumers

The three public training preparation families all call `prepare_data(data)`
before native preparation:

| Caller | Immediate consumer | Data-specific contract |
| --- | --- | --- |
| `compile_training_graph` | dense `compile_dense_training*` functions | builds feature and target matrices from typed vectors; downstream normalization is a GPU calculation over the prepared training partition |
| `compile_knn_model` | `prepare_knn_reference_set` | exact prepared training partition is the immutable reference set; requires a nonempty train partition and at least one target |
| `compile_bayes_model` | `prepare_categorical_bayesian_reference_sets` | declared children must be target categorical vectors and parents feature categorical vectors; training rows require known dictionary codes |

`TrainingError::Data` preserves the data-preparation stage. Dense lowering
rejects a semantic tuple without a dedicated model lowering, while KNN and
Bayes apply their own stricter target contracts. None of these consumers can
read the original path after `prepare_data` returns.

Dense training keeps source dtypes as data authority. Numeric I32/F32 features
are lowered to scalar features, categorical dictionary vectors become one-hot
features with a reserved unseen route, and unsupported variable-width or opaque
features fail the relevant lowering boundary. `.norm(z_score)`,
`.norm(min_max)`, and `.norm(l2_norm)` select a later GPU calculation; the
loader never mutates `PreparedValues`.

When a dense run becomes a semantic checkpoint, the checkpoint manifest copies
each prepared vector's source index, name, role, semantic type, encoding, and
encoding metadata, plus feature spans, target source-index order, and the
normalization mask. It retains row-free schema rather than source rows. Resume
therefore compares the newly prepared schema and target order exactly, while
target-free inference applies the saved feature names and dictionaries to new
rows.

KNN lowers the prepared training feature matrix to finite f32 bits only when
integer conversion is exact. Each declared target remains one independent
output in declaration order; missing target references are masked per output.
Bayesian preparation requires complete known child and parent codes and rejects
missing or reserved unseen codes in its observed training slice.

## Failure taxonomy and ordering

`DataPreparationError` is non-exhaustive and currently contains:

| Variant | Origin and trigger | `source()` |
| --- | --- | --- |
| `Declaration(DeclarationError)` | deferred builder error or no source | yes |
| `MissingTargets` | training preparation without at least one target | no |
| `MissingSplit` | training preparation without `.split(...)` | no |
| `FloatPredicateOutsideF32 { column, value }` | condition float cannot be finite, non-underflowing f32 | no |
| `Ingest(IngestError)` | default `IngestLimits` construction | yes |
| `Source(DatasetSourceError)` | path, symlink, archive, recursive bound, structural format, or source framing failure | yes |
| `Semantic(SemanticError)` | inconsistent row width or evidence arithmetic overflow during inference | yes |
| `Prepare(PrepareError)` | invalid split, target/name/pattern resolution, predicate evaluation, encoding, metadata, or prepared-state invariant failure | yes |

Source failures are fail-closed and include a stable kind, optional logical
path, and detail. Important source kinds are `InvalidPath`, `Io`, `Symlink`,
`EmptySource`, `LimitExceeded`, `MalformedArchive`, `MalformedFormat`,
`Ingest`, and `ArithmeticOverflow`. Important preparation kinds include
`EmptyTargetSet`, `DuplicateTarget`, `TargetNotFound`,
`UnmatchedColumnPattern`, `TargetExcluded`, `NoFeatureVectors`,
`NoRetainedRows`, `PredicateColumnNotFound`, `PredicateTypeMismatch`,
`MissingPredicateValue`, `InvalidPredicateValue`, `InconsistentInference`,
`EncodingFailure`, `TemporalRangeExceeded`, `VariableWidthDenseMatrix`,
`MixedDenseEncoding`, `MissingDenseValue`, `EmptyDenseSelection`,
`InconsistentPreparedVector`, and arithmetic overflow. `SemanticInference`,
`InvalidTrainFraction`, `InvalidColumnPattern`, `DuplicateColumnName`, and
`InvalidPredicateLiteral` are also represented by the enum. The explicit dense
projection variants are used by `PreparedDataset::fixed_dense_matrix` and
downstream callers; the current `prepare_data_with_limits` call graph reaches
the typed selection and encoding variants, not that projection method.

Failure ordering is observable and should remain stable:

```text
declaration validation
  < training-only target/split checks
  < split/pattern/condition translation
  < source read and aggregate admission
  < format parse and rectangular distillation
  < semantic inference
  < target/column/predicate selection
  < train fraction and fit metadata
  < retained-row encoding
  < downstream model lowering
```

No fallback source, retry, imputation, alternate semantic interpretation, or
partial result is introduced by `data_prepare`.

## Trace findings to preserve

- Source declaration order is the row order authority. Directory and archive
  traversal add deterministic path order within their source, while aggregate
  limits apply across the complete declared set.
- Predicates see original source columns and execute before helper-column
  removal. Targets are resolved before exclusions and cannot be excluded.
- Train and validation are contiguous slices of the retained source order. The
  split is exact binary-fraction arithmetic, not a random sample.
- Semantic encodings and metadata are immutable typed state. Unknown categorical
  labels are not collapsed into missing; they use a reserved calculation code
  plus an exact `Unseen` observation.
- Normalization belongs to model calculation and is never a host-side loader
  mutation.
- The current public training path calls
  `DistilledDataset::infer_vectors` on the complete distilled table, then passes
  that result to `prepare_inferred_table`. Thus parser/classifier semantic
  discovery can observe rows that later become validation or are removed by row
  predicates. The lower-level
  `DistilledDataset::prepare` path exists specifically to fit semantic rules
  from the exact retained train partition, but `prepare_data_with_limits` does
  not call it. Documentation and future changes must not claim train-only
  semantic discovery for the public path without changing this call graph.
