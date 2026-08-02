<!-- Source of truth: ingest/src/inference.rs. Keep the paths and line references
     below tied to the implementation when this document is regenerated. -->

# Schema-bound inference ingestion

<code>ingest::prepare_inference_table</code> is the boundary between an already framed
<code>RawTable</code> and target-free model input. It applies a saved feature schema to new
rows and returns typed, unnormalized values in model-feature order. It does not
read a file, infer semantic types, fit a dictionary, select a target, filter
rows, split rows, normalize values, or execute a model calculation.

The implementation is <code>ingest/src/inference.rs:1-499</code>. The public re-exports
are in <code>ingest/src/lib.rs:44-47</code>. Model-specific callers live in
<code>training/src/inference.rs</code>; the public declaration and execution boundary is
<code>src/inference.rs</code>.

## Boundary and data flow

The preparation path is deliberately one-way:

    source bytes
      -> table framing (read_table or parse_table)
      -> RawTable (headers plus rectangular byte rows)
      -> saved InferenceFeatureSchema list
      -> prepare_inference_table
      -> PreparedInferenceDataset
      -> model-family compiler in training
      -> static graph and native execution

<code>RawTable</code> is defined by <code>ingest/src/table.rs</code>. File reads and framing are
bounded by <code>IngestLimits</code>; <code>prepare_inference_table</code> receives the completed
table and has no filesystem side effects. A <code>DistilledDataset</code> is another
caller-owned source of the same table (<code>ingest/src/dataset.rs:112-157</code>), and
training callers pass <code>dataset.table()</code>.

The result contains only rows and required features. A successful call has no
output file, no retained file handle, and no hidden model state. The result is
owned by the caller, so later graph compilation can consume the same exact byte
interpretations without reopening the source.

## Saved feature contract

### InferenceFeatureEncoding

<code>InferenceFeatureEncoding</code> (<code>inference.rs:8-12</code>) is the complete set of
encodings that this boundary accepts:

| Variant | Source bytes accepted | Result variant | Missing-value behavior |
| --- | --- | --- | --- |
| <code>NumericI32</code> | UTF-8 decimal integer accepted by <code>parse_contract_i32</code> | <code>PreparedInferenceValues::I32</code> | Empty bytes are an error |
| <code>NumericF32</code> | UTF-8 finite decimal accepted by <code>parse_contract_f32</code> | <code>PreparedInferenceValues::F32Bits</code> | Empty bytes are an error |
| <code>CategoricalDictionary { dictionary }</code> | Exact byte labels | <code>PreparedInferenceValues::I32</code> plus observations | Empty bytes use the reserved code and <code>Missing</code> |

The dictionary is a saved artifact, not a vocabulary fitted from inference
rows. Dictionary position <code>n</code> is the calculation-facing code <code>n</code>. The reserved
code is <code>dictionary.len()</code> and is used for both missing bytes and a nonempty
label that is not in the saved dictionary. The parallel
<code>CategoricalObservation</code> route keeps those two cases distinct.

<code>InferenceFeatureSchema</code> (<code>inference.rs:16-40</code>) stores:

* <code>source_vector: usize</code>: the saved model identity for the vector;
* <code>name: Vec<u8></code>: the exact source-header bytes used for lookup; and
* <code>encoding</code>: one of the three encodings above.

<code>InferenceFeatureSchema::new</code> accepts any bytes for <code>name</code>, including bytes that
are not UTF-8. The name must only be nonempty when the schema is applied.
<code>source_vector</code> is not used to locate a column in the raw table. Lookup is by
exact <code>name</code> bytes. The model compiler later checks this identity against saved
feature spans.

### InferenceDataPath

<code>InferenceDataPath</code> (<code>inference.rs:44-93</code>) is the stable diagnostic location for
one feature. It contains the schema feature ordinal, saved <code>source_vector</code>, a
copy of the saved column name, and an optional source-row ordinal. Its display
form is:

    inference.feature[FEATURE].source-vector[SOURCE_VECTOR].column["NAME"]
    inference.feature[FEATURE].source-vector[SOURCE_VECTOR].column["NAME"].source-row[ROW]

The public getters expose each component. The constructor is private because
paths are created only from a validated schema and, for value errors, the row
being encoded.

## Prepared result types

<code>PreparedInferenceValues</code> (<code>inference.rs:156-172</code>) is a non-optional,
row-aligned payload:

    I32(Vec<i32>)       exact integer codes or integer feature values
    F32Bits(Vec<u32>)   exact IEEE-754 bit patterns for f32 values

<code>len()</code> and <code>is_empty()</code> dispatch to the selected vector. There is no numeric
<code>Option</code> representation because missing numeric values fail during schema
application. Categorical missing and unseen values remain representable through
the reserved integer code and the observation side channel.

<code>PreparedInferenceFeature</code> (<code>inference.rs:176-202</code>) stores, in addition to the
payload:

* a clone of the applied <code>InferenceFeatureSchema</code>;
* <code>source_column</code>, the physical header index selected from the raw table;
* <code>values</code>, the calculation-facing payload; and
* <code>categorical_observations</code>, <code>Some</code> only for a dictionary feature.

<code>categorical_observations()</code> returns rows in the same order as <code>values</code>. The
observation type is defined in <code>ingest/src/prepare.rs:324-353</code>:

    Known { code: i32 }       dictionary label, with its saved code
    Missing                   empty source bytes
    Unseen { label: Vec<u8> } nonempty bytes absent from the dictionary

<code>PreparedInferenceDataset</code> (<code>inference.rs:206-217</code>) stores the raw-table row
count and one prepared feature for each schema entry. <code>features()</code> preserves
schema order, not source-header order. A table with zero rows can produce a
successful dataset with zero-length feature vectors; target-free graph
compilers reject that state later when a runtime query requires at least one
row.

## prepare_inference_table algorithm

The public function is <code>ingest/src/inference.rs:234-318</code>.

### 1. Validate the saved schema before reading rows

An empty schema returns <code>EmptyFeatureSchema</code>. For every schema entry, the
function then:

1. rejects an empty feature name;
2. for a categorical dictionary, rejects any empty label;
3. rejects a dictionary that is not strictly ascending by raw byte order;
4. checks that the dictionary length fits in <code>i32</code>, because its length is the
   reserved code;
5. rejects a duplicate saved name; and
6. rejects a duplicate saved <code>source_vector</code> identity.

The duplicate checks use <code>BTreeMap</code> values holding the first feature ordinal.
The second occurrence is reported with an <code>InvalidFeatureSchema</code> error at that
feature's path. A zero-length categorical dictionary itself is allowed: it has
no labels to violate the nonempty or ordering rules, and its reserved code is
zero. Every nonempty source label is therefore <code>Unseen</code> for that schema.

Schema validation is complete before the table header map is built. No partial
<code>PreparedInferenceDataset</code> escapes on any error.

### 2. Resolve exact source columns

The headers are folded into <code>BTreeMap<&[u8], Vec<usize>></code>
(<code>inference.rs:264-270</code>). This map preserves exact bytes and records every
physical occurrence.

For each schema feature, in schema order:

* no matching header returns <code>MissingRequiredFeature</code>;
* more than one matching header returns <code>AmbiguousRequiredFeature</code>; and
* exactly one match supplies <code>source_column</code> to the encoder.

Unrelated source columns are ignored. Source columns may be reordered relative
to the saved schema. Header names are not trimmed, decoded, case-folded, or
otherwise normalized. A duplicate header that is not required by the model is
irrelevant; a duplicate required header is an error.

### 3. Encode every source row

<code>encode_feature</code> is private (<code>inference.rs:352-429</code>). It always iterates all
<code>table.rows()</code> and calls <code>source_value</code> (<code>inference.rs:464-478</code>) before decoding
the selected field. A row that does not contain the resolved physical column
returns <code>InvalidValue</code> at the feature and source-row path. Normally
<code>RawTable</code> already guarantees rectangular rows through <code>RawTable::from_parts</code>
and the table parser, but this check keeps the inference boundary closed if an
inconsistent table is supplied internally.

#### Numeric i32

For <code>NumericI32</code> (<code>inference.rs:359-376</code>):

1. empty bytes return <code>MissingValue</code> with detail
   <code>required numeric feature value is missing</code>;
2. non-UTF-8 bytes return <code>InvalidValue</code>;
3. <code>parse_contract_i32</code> validates the decimal syntax, significant-digit
   contract, and <code>i32</code> range; and
4. the exact <code>i32</code> value is appended to <code>PreparedInferenceValues::I32</code>.

No whitespace trimming, defaulting, imputation, host arithmetic, or type
inference occurs.

#### Numeric f32

<code>NumericF32</code> (<code>inference.rs:377-394</code>) follows the same sequence, then stores
the parsed value's exact <code>u32</code> bits in <code>PreparedInferenceValues::F32Bits</code>.
<code>parse_contract_f32</code> is the numeric contract in <code>ingest/src/numeric.rs</code>; it
rejects invalid syntax, nonfinite values, out-of-range values, and precision
loss. The current contract admits at most six significant decimal digits for
f32 values. <code>parse_contract_i32</code> admits at most nine significant digits for
integer values.

#### Categorical dictionary

For <code>CategoricalDictionary</code> (<code>inference.rs:395-427</code>), the function first maps
each canonical dictionary label to its enumerated <code>i32</code> code and computes
the reserved code as the dictionary length. Each row then follows this exact
mapping:

| Source field | Calculation code | Observation |
| --- | ---: | --- |
| empty bytes | <code>reserved</code> | <code>Missing</code> |
| exact dictionary label | saved position | <code>Known { code }</code> |
| nonempty label absent from dictionary | <code>reserved</code> | <code>Unseen { label: exact bytes }</code> |

The unknown label is copied into the observation, so it is not silently
collapsed into missing. The code vector and observation vector are checked by
<code>validate_categorical_alignment</code> (<code>inference.rs:431-462</code>): both lengths must
match; known codes must be in <code>0..reserved</code>; missing and unseen labels must use
<code>reserved</code>; and unseen labels must be nonempty. A mismatch returns
<code>InvalidValue</code> at the offending source row.

### 4. Preserve row cardinality and return

After encoding, the function checks that the payload length, and the optional
categorical observation length, equal <code>table.rows().len()</code>
(<code>inference.rs:294-305</code>). Failure is reported as <code>ArithmeticOverflow</code> with
the feature path. This variant is a checked invariant failure, not a claim that
a source byte was arithmetically large. On success, <code>rows</code> is the raw table row
count and the feature list is returned unchanged in schema order.

## Error surface

<code>InferencePrepareErrorKind</code> (<code>inference.rs:98-106</code>) is <code>#[non_exhaustive]</code> and
currently contains:

| Kind | Produced when |
| --- | --- |
| <code>EmptyFeatureSchema</code> | The saved feature list is empty |
| <code>InvalidFeatureSchema</code> | Empty feature name, duplicate name or source identity, noncanonical dictionary, or dictionary length outside <code>i32</code> |
| <code>MissingRequiredFeature</code> | A required schema name has no exact header match |
| <code>AmbiguousRequiredFeature</code> | A required schema name appears more than once in the source headers |
| <code>MissingValue</code> | A numeric field is empty |
| <code>InvalidValue</code> | Missing physical field, invalid UTF-8, decimal parse/contract failure, dictionary code conversion, or categorical alignment failure |
| <code>ArithmeticOverflow</code> | Encoded payload or observation length does not equal the source row count |

<code>InferencePrepareError</code> keeps <code>kind</code>, an optional
<code>InferenceDataPath</code>, and a human-readable <code>detail</code> string. <code>kind()</code>,
<code>path()</code>, and <code>detail()</code> are the only accessors. <code>Display</code> prints
<code>KIND: DETAIL</code> and appends the formatted path when one exists. Schema-empty
errors have no path. Row decoding and categorical alignment failures include the
source-row ordinal. The result alias is
<code>InferencePrepareResult&lt;T&gt; = Result&lt;T, InferencePrepareError&gt;</code>.

The function fails closed on the first error encountered while walking the
schema or rows. It does not return partial features, substitute a source
column, or retry a failed parse.

## Source bounds and what this module does not limit

There is no byte, record, field-count, field-width, or allocation limit in
<code>inference.rs</code>. Those bounds belong to the preceding source boundaries:

* <code>SourceLimit</code> and <code>read_source_snapshot</code> bound a regular-file snapshot;
* <code>IngestLimits</code> and <code>read_table</code>/<code>parse_table</code> bound source bytes, record count,
  fields per record, and field bytes; and
* <code>DistilledDataset</code> applies its aggregate source and row bounds while combining
  files or archive members.

The inference boundary can still allocate one result vector per required
feature, one dictionary map per categorical feature, and a byte copy for each
unseen categorical label. The only explicit checked conversions inside this
module are dictionary length to <code>i32</code> and the row-alignment invariant
described above. No source file is reopened, and no source bytes are retained
except the owned prepared values and categorical unseen-label copies.

## Model-derived schemas and downstream callers

The ingestion function is intentionally generic. <code>training/src/inference.rs</code>
constructs the schema from each decoded model and then delegates all row
application to this module.

### Dense checkpoints

<code>saved_feature_schema_from_parts</code> (<code>training/src/inference.rs:4843-4907</code>)
walks saved <code>CompiledFeatureSpan</code> entries and finds the corresponding
<code>CheckpointArtifactVector</code> by <code>source_vector</code>. It creates:

* <code>NumericI32</code> for a numeric scalar with <code>VectorEncoding::I32</code> and no metadata;
* <code>NumericF32</code> for a numeric scalar with <code>VectorEncoding::F32</code> and no metadata;
* <code>CategoricalDictionary</code> for a categorical dictionary vector whose one-hot
  span width is dictionary length plus one and whose reserved index equals the
  dictionary length.

Any other span, semantic type, encoding, metadata, or width combination returns
<code>InferencePreparationError::InconsistentCheckpoint</code> before the table is
applied. <code>prepare_checkpoint_inference_table</code>
(<code>training/src/inference.rs:868-875</code>) then calls
<code>recipe_ingest::prepare_inference_table</code> and validates the prepared
feature count, source identities, encodings, payload variants, and span widths
with <code>validate_prepared_feature_spans</code> (<code>4910-4965</code>). The returned
<code>PreparedInference</code> retains both the decoded checkpoint and the prepared rows.

<code>compile_prepared_inference</code> later keeps each <code>PreparedInferenceValues</code>
payload as an external input. Numeric <code>i32</code> to <code>f32</code> conversion and
categorical one-hot expansion are emitted as Recipe graph calculations. Saved
normalization, layers, prediction interpretation, and output adapters are also
graph work; none of them run inside <code>ingest</code>.

### KNN artifacts

<code>prepare_knn_inference_table</code> (<code>training/src/inference.rs:784-795</code>) builds the
same schema from KNN reference vectors and feature spans, applies this module,
then validates the resulting spans. <code>PreparedKnnInference</code> retains the KNN
artifact and the prepared table. Its compiler rejects empty query data and
empty reference matrices, then performs query/reference normalization, distance,
neighbor selection, and typed numeric or discrete outputs in Recipe operations.
Those are downstream calculations, not ingestion transformations.

### Categorical Bayesian artifacts

<code>prepare_bayes_inference_table</code> (<code>training/src/inference.rs:800-820</code>) walks every
conditional in artifact order and every parent in parent order. It inserts the
first occurrence of each <code>source_index</code> into one union schema, reusing shared
parents only once, and gives every parent its saved dictionary. The raw table is
then applied once. <code>PreparedBayesInference</code> retains the artifact and prepared
rows.

The Bayesian compiler consumes the resulting <code>I32</code> parent codes and checks
each query code against the saved parent cardinality. Consequently, the
reserved code emitted by this module for a missing or unseen query label is
visible to the compiler and may be rejected there as an out-of-range model
code. That is a downstream model-consistency error, not an ingestion error.

### Bounded model loading

The three direct model loaders in <code>training/src/inference.rs</code> are preparation
wrappers around this boundary:

| Loader | Source operation | Decode result |
| --- | --- | --- |
| <code>load_checkpoint_file</code> | <code>SourceLimit</code> plus <code>read_source_snapshot</code> | strict dense checkpoint |
| <code>load_knn_model_file</code> | <code>SourceLimit</code> plus <code>read_source_snapshot</code> | strict KNN artifact |
| <code>load_bayes_model_file</code> | <code>SourceLimit</code> plus <code>read_source_snapshot</code> | strict Bayesian artifact |

<code>load_semantic_model_file</code> reads one bounded snapshot, probes only the
first line, and dispatches the strict decoder for roots <code>recipe</code>,
<code>recipe-knn-model</code>, or <code>recipe-bayes-model</code>. It never falls back from
one model family to another. The source snapshot and model decoder errors are
reported before <code>prepare_inference_table</code> and are not part of
<code>InferencePrepareErrorKind</code>.

## Public end-to-end caller

The public API path is implemented in <code>src/inference.rs:487-543</code>:

1. <code>compile_inference</code> validates the <code>Infer</code>, <code>Data</code>, and <code>Model</code> declarations.
2. Target-free policy rejects <code>.target(...)</code>, <code>.split(...)</code>, and a data-side
   <code>.normalization(...)</code>; the saved model owns target interpretation and
   normalization.
3. The model extension selects <code>.ogdl</code> semantic loading or the separate <code>.gguf</code>
   llama path.
4. Data is distilled and target-free rows are selected.
5. Dense, KNN, or Bayes semantic models call the corresponding training
   preparation function, which delegates to this module.
6. The prepared artifact is compiled into a static Recipe graph. Native probing,
   allocation, realization, and execution occur only after this boundary.

<code>Infer::evaluate</code> (<code>src/api.rs:2234-2240</code>) resolves the declaration and invokes
that public path. On a successful native run, the root inference module writes
prediction rows after native teardown. The result is model-family specific:
dense predictions, KNN typed outputs, or Bayesian probability blocks. The
ingest module contributes only the prepared feature payload that reaches those
graphs.

## Error propagation to the public API

<code>InferencePrepareError</code> converts to
<code>training::InferencePreparationError::Data</code>
(<code>training/src/inference.rs:60-125</code>), then to
<code>src::InferenceError::Model</code> (<code>src/inference.rs:34-116</code>). The displayed
context is therefore:

    prepare inference data: <InferencePrepareError Display>
    load inference model: <InferencePreparationError Display>

Source snapshot, strict model-decoder, checkpoint/schema-consistency, graph
compilation, native preparation, and execution failures remain distinct outer
error classes. A failure in this module never becomes a fabricated prediction,
an empty feature, or a fallback encoding.

## Invariants for maintainers

When changing this boundary, preserve these observable invariants:

* feature lookup is exact byte-name lookup, while output order is saved schema
  order;
* every required source feature occurs exactly once;
* numeric values are contract-parsed and never silently missing;
* categorical dictionaries are reused exactly, with reserved-code and typed
  observation alignment preserved;
* every prepared payload and observation vector has exactly the source row
  count;
* inference preparation performs no semantic fitting, normalization, one-hot
  expansion, target selection, row filtering, filesystem access, or native
  execution; and
* all failures are typed and path-addressed where a feature or source row is
  known.
