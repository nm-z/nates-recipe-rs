# `training/src/bayes.rs`: observed categorical Bayesian preparation

## Scope and the real call path

This file documents the current implementation of the observed categorical
Bayesian instrument. It is an implementation trace, not a promise that every
Bayesian form named by `API.ogdl` is executable. The central boundary is
[`training/src/bayes.rs`](../../src/bayes.rs): it resolves names against an
already prepared dataset and retains exact categorical observations. It does
not build a calculation graph, fit a count table, choose a native device, or
run a training loop. The paired callers complete the path:

```text
public fluent declarations
    -> Data, Model(BayesDependency), Train policy
    -> src/training.rs::Train::run / __recipe_run_with
    -> compile_bayes_model
    -> prepare_data
    -> training/src/bayes.rs
    -> BayesModelArtifact (semantic observations, optional .ogdl save)

target-free inference declarations
    -> load_semantic_model_file (recipe-bayes-model root)
    -> prepare_bayes_inference_table
    -> compile_prepared_bayes_inference
    -> ops/src/bayes.rs native histogram and posterior graph
    -> StaticCalculationProgram, one iteration
    -> measured native preparation
    -> init input admission -> loop -> exit output egress
    -> InferenceReport and categorical probability rows
```

The source modules have deliberately different responsibilities:

| Boundary | Current responsibility | What it does not do |
| --- | --- | --- |
| [`src/api.rs`](../../../src/api.rs) | Records `Data`, `BayesDependency`, `Model`, `Train`, `Infer`, and artifact declarations. | No filesystem read, semantic inference, graph lowering, native preparation, or execution. |
| [`src/data_prepare.rs`](../../../src/data_prepare.rs) and `recipe-ingest` | Distills sources, infers vector schemas, fits dictionaries on the training prefix, applies exclusions, and creates train/validation partitions. | No Bayesian interpretation or host posterior calculation. |
| [`src/training.rs`](../../../src/training.rs) | Validates the specialized training policy, calls this module, wraps the artifact in `TrainingReport`, and writes an optional semantic model. | No native training lifecycle for Bayes and no native kernel artifact. |
| [`training/src/bayes.rs`](../../src/bayes.rs) | Resolves the declared DAG and turns each observed categorical target conditional into an immutable reference set. | No graph, parameters, optimizer state, counts, or probability values. |
| [`training/src/bayes_checkpoint.rs`](../../src/bayes_checkpoint.rs) | Encodes, strictly decodes, validates, appends, and atomically saves the semantic `.ogdl` artifact. | No target-free query preparation or native posterior calculation. |
| [`training/src/inference.rs`](../../src/inference.rs) | Decodes the artifact, prepares the union of saved parent schemas, builds the inference graph, and defines its external input/output contracts. | No device discovery or driver handoff. |
| [`ops/src/bayes.rs`](../../../ops/src/bayes.rs) | Materializes one categorical conditional as Recipe primitives with checked shapes, identities, workspace, and alias rules. | No file or public declaration handling. |
| [`training/src/execute.rs`](../../src/execute.rs) | Validates the compiled inference boundary and runs the generic one-iteration native lifecycle. | No Bayes-specific mathematics beyond task/output checks. |
| [`src/inference.rs`](../../../src/inference.rs) | Selects the semantic model family, performs measured native preparation, executes Bayes, and formats/report-accesses its result. | No alternate host-side posterior path. |

The export surface follows those boundaries. `recipe-training/src/lib.rs`
re-exports the byte-oriented dependencies, schema and reference-set types,
`resolve_bayesian_schema`, both preparation entrypoints, the smoothing constant,
`BayesModelArtifact`, its decode limits, the bounded model loader, prepared
Bayes inference, and `compile_prepared_bayes_inference`. `recipe-ops/src/lib.rs`
re-exports only the operation request, requirement/materialization records, and
the three categorical-Bayes materialization functions. The root `src/lib.rs`
re-exports `BayesModelArtifact`, `compile_bayes_model`, `TrainingReport`, and
the public inference boundary. A caller can therefore use the specialized
high-level API without reaching into private graph or artifact fields, while a
lower-level crate caller can still invoke the typed preparation and graph
compiler directly.

The normative contract calls the current repeated instrument **C42**. The
singular form remains the version-one compatibility image described by **C36**.
Both use the same observed categorical semantics: known parent dictionaries,
one reserved route per parent for unseen query labels, no reserved child
class, fixed Laplace-one smoothing, and no optimizer state. The active
implementation is the repeated form, including the one-conditional case.

## Public declarations and branch selection

### Data is an immutable preparation request

`Data::set`, `target`, `exclude`, `split`, and `norm` only store paths, names,
filters, a train fraction, and an optional numeric normalization choice. The
builders call `remember_recipe_data`, but do not read the source. `Data::validate`
requires at least one source; Bayes-specific preparation later requires at
least one target and an explicit split. The ordinary training data boundary is
`prepare_data`, not a builder call.

The Bayes branch rejects `Data::norm(...)`. Categorical node identity is the
raw dictionary identity and must not be changed by Z-score, min-max, or L2
normalization. This is an unsupported training declaration at the specialized
compiler, with the exact current detail `categorical Bayesian node identities
cannot be numerically normalized`.

### `Model::bayes` records ordered parent-to-child edges

`BayesDependency` in `src/api.rs:946-1003` owns a `String` child and a
`Vec<String>` of parents. The edge direction is `parent -> child`; both the
edge order and each parent list order are retained. `Model::bayes` at
`src/api.rs:1201-1224` appends one edge, validates the complete network, and
removes the newly appended edge when that validation fails. The first deferred
declaration error remains on the model, so a later run reports it through the
normal `TrainingError::Declaration` boundary.

The public declaration validator rejects:

* an empty child name or an empty parent name;
* a parent equal to its child;
* duplicate parents in one edge;
* a child declared by more than one edge; and
* any dependency cycle, found by following child-to-parent reachability over
  the complete ordered declaration list.

`Model::validate` also requires at least one layer, one Bayesian dependency,
or a loaded model source. A checkpoint-backed model cannot contain inline
layers, Bayesian edges, a loss, or gradient policy. Thus a normal Bayes model
has Bayesian edges and no layers, objective, gradient clip, or weight source.

The public `Model::bayes` validator is intentionally stricter than a generic
name container but does not inspect the dataset. Missing nodes, roles, vector
encodings, and row completeness are checked only after `prepare_data` has
produced the typed dataset.

### `Train` policy is specialized, not an optimizer configuration

`Train` still stores the general policy fields, but `compile_bayes_model`
(`src/training.rs:500-579`) rejects every field that would imply an iterative
training run:

| Declaration | Bayes result |
| --- | --- |
| `.optimizer(...)`, `.lr(...)`, `.cos()`, `.exp()`, `.warmup(...)`, `.epochs(...)` | Unsupported: observed preparation has no optimizer, learning rate, warmup, or epoch loop. |
| `.log(...)` or `.plot(...)` | Unsupported: there are no iterative training metrics. |
| `.resume("model.ogdl", "kernel.cubin/hsaco")` | Unsupported: Bayes does not realize a training kernel. |
| `.save("kernel.cubin/hsaco")` or the two-path save form | Unsupported for the same reason. |
| `.save("model.ogdl")` | Accepted and writes the semantic Bayes artifact after preparation. |
| `.resume("model.ogdl")` | Accepted conditionally. An existing file is decoded and appended to; a missing file starts fresh. |

The public API preserves literal one-path and two-path method forms. The
specialized branch only reaches `save_model_destination`; it never calls the
dense native package, `SigintGuard`, or `save_native_kernel`.

### Training dispatch has one Bayes branch

`Train::run` consumes the preceding `Data` and `Model` from the facade
sequence (`src/training.rs:858-861`). Source-frontend lowering of the explicit
`.run(&model, &data)` form calls the hidden `__recipe_run_with`, which reaches
the same `try_run_with` implementation (`src/training.rs:863-867`). The branch
order in `try_run_with` is:

1. Any Bayesian dependencies call `compile_bayes_model`. A successful artifact
   becomes `TrainingReportPayload::Bayes`; an optional `.ogdl` model is saved;
   the branch returns without native preparation or execution.
2. Otherwise a standalone KNN layer selects the KNN reference branch.
3. Only otherwise does dense compilation create a native package and execute
   `init -> loop -> exit`.

Normal model validation rejects a mixed Bayes-plus-layer declaration before
dispatch. The branch order is still significant: Bayesian dependencies cannot
be silently ignored by dense compilation, whose `require_supported_model`
(`src/training.rs:1780-1804`) explicitly rejects them.

The ordinary fluent facade (`src/facade.rs`) remembers the latest data and
model declarations in the training sequence. `Train::run` consumes that pair,
so a Bayes training call must be preceded by the matching `recipe.data(...)`
and `recipe.model().bayes(...)` declarations. The source frontend's direct
method form is only syntax lowering, not a second Bayes implementation. For
inference, `Infer::resolve_declaration` consumes the immediately preceding
data/model pair, validates target-free policy, and enters the same public
`src/inference.rs` package used by `Infer::evaluate` and `compile_inference`.

## Data preparation before `training/src/bayes.rs`

`compile_bayes_model` first calls `policy.validate`, `data.validate`, and
`model.validate`. It then applies the specialized policy checks above and
conditionally loads an existing semantic resume source. `Path::try_exists`
distinguishes a normal missing file from an inspection error. A missing path is
not a failure; an I/O or metadata error becomes
`TrainingError::Runtime("inspect Bayesian resume model", ...)`.

The current data is loaded exactly once through `prepare_data`:

1. `prepare_data_with_limits` validates the public `Data` declaration,
   requires at least one target, and requires `.split(...)`.
2. The source files, directories, or ZIP containers are distilled under the
   bounded ingest limits. Semantic vectors are inferred with
   `CategoricalEncodingModel`.
3. Excluded columns and rows are applied before splitting. The requested target
   names become ordered target source identities.
4. The training prefix fits each vector schema and categorical dictionary. The
   complete retained table is then encoded against those schemas and split into
   the training and validation partitions.

The resulting `PreparedDataset` retains vector role, source index, schema,
dictionary, optional encoded values, retained positions, original source row
indices, and ordered target source indices. Bayes consumes only the training
partition. The validation partition is not evidence and never enters the
artifact. A prepared source row can be addressed both by its retained position
and by its original source row, which is why row failures can name the source
row precisely.

`DataPreparationError` is fail-closed. Missing targets, missing split,
declaration errors, bounded source or ingest failures, semantic inference
failures, filtering failures, and typed preparation failures return without a
partial dataset. Bayes-specific semantic checks begin only after this boundary
has succeeded.

## Row-free schema resolution

### `BayesianDependency`, node identity, and source classification

The training crate re-owns public edge names as byte vectors through
`BayesianDependency::new`. The module-level constant
`CATEGORICAL_BAYES_SMOOTHING` is exactly `1.0f32`; it is the only smoothing
contract accepted by the semantic artifact.

`resolve_bayesian_schema` is intentionally row-free. Its output is a
`ResolvedBayesianSchema` containing three views of one validated graph:

* `nodes` are assigned `BayesianNodeId` values in ascending byte-name order,
  independent of declaration order;
* `declarations` retain repeated-call order and each literal parent order; and
* `execution_order` is a separately derived deterministic topological order.

Every prepared vector is inserted as `BayesianNodeSource::Observed`, including
vectors omitted from the declarations. A name absent from the prepared data is
represented as a latent root when it is parent-only or zero-indegree, and as a
latent conditional when it is an absent child with parents. This structural
resolver can therefore describe a broader DAG than the current executable
observed slice.

`prepare_categorical_bayesian_reference_sets` rejects any latent source before
it creates a reference set. No implicit state space, sampling, ancestral
prediction, or marginalization is invented for an absent node.

### Typed structural errors

`BayesianSchemaError` carries a `BayesianSchemaErrorKind`, a vector of typed
path segments, and a display detail. The kinds are:

| Kind | Producer condition | Display path shape |
| --- | --- | --- |
| `EmptyName` | Empty child or parent byte name. | `declarations[i].child` or `declarations[i].parents[j]` |
| `DuplicateDatasetName` | Two prepared vectors have the same name. | `dataset.vectors[i].name` |
| `DuplicateChild` | Two declarations use one child. | `declarations[i].child` |
| `DuplicateParent` | One declaration repeats a parent. | `declarations[i].parents[j]` |
| `SelfDependency` | A child names itself as a parent. | `declarations[i].parents[j]` |
| `Cycle` | Kahn-style topological ordering cannot consume every node. | `graph.execution-order` |

`DisplayPath` renders dataset and declaration paths without losing the machine
readable segments. `resolve_bayesian_schema` first checks observed names and
declarations, adds latent entries, builds canonical node IDs, resolves edges,
then uses `deterministic_topological_order`. Ready nodes are held in a
`BTreeSet`, so equal-degree choices are name-order deterministic. A cycle
detail includes the remaining node names.

## Exact observed categorical reference sets

### Public preparation functions

`prepare_categorical_bayesian_reference_sets(dataset, dependencies)` is the
current repeated-call boundary. It first resolves the row-free schema, then
requires all of the following:

* at least one declaration;
* no latent node in the resolved schema;
* at least one training row;
* every declaration has at least one parent;
* every child exists and has `VectorRole::Target`;
* every child source index equals the target source indices in the exact
  repeated-call order; and
* every parent exists and later proves to be `VectorRole::Feature`.

The target-order check is the binding between repeated `.bayes(...)` calls and
the public `.target(...)` matrix. It prevents an artifact from silently
relabeling outputs when declaration and target order differ.

`prepare_categorical_bayesian_reference_set` is a compatibility boundary for
exactly one dependency. It rejects zero or multiple dependencies and delegates
to the repeated function, returning its sole reference set. It is retained for
callers of the original singular instrument; `compile_bayes_model` uses the
repeated function so version two can represent two or more conditionals.

### Per-conditional lowering

`prepare_categorical_bayesian_reference_set_for_dependency` performs no host
posterior calculation. It creates one `BayesianCategoricalReferenceSet` with
the following state:

| Field | Meaning |
| --- | --- |
| `parents` | Parent source index, raw name bytes, and ordered dictionary labels, in literal declaration order. |
| `child` | Target source index, raw name bytes, and ordered dictionary labels. |
| `reference_source_rows` | Original source row for every training observation, in prepared training order. |
| `reference_rows` | Number of training observations. |
| `parent_codes` | Row-major int32 parent codes, with the parent dimension ordered as declared. |
| `child_codes` | One int32 child class code per training row. |
| `parent_cardinalities` | `dictionary.len() + 1` for every parent. The final code is reserved for an unseen query label. |
| `parent_multipliers` | Mixed-radix multipliers in parent declaration order. |
| `parent_configurations` | Product of parent cardinalities, including reserved unseen routes. |

Both child and parent vectors must be `SemanticType::Categorical`,
`VectorEncoding::DictionaryI32`, `VectorMetadata::Categorical`, and
`PreparedValues::I32`. Empty dictionaries are rejected. The child dictionary
contains only known training classes. Parents reserve one additional inference
route but do not add that route to their saved dictionary.

For each parent, the dictionary length plus one must fit `usize`, then `i32`.
The mixed-radix contract walks the cardinalities from right to left. The last
parent has multiplier `1`; each earlier multiplier is the product of all later
cardinalities. The total product must fit `u64` and `i32`. The child class count
and `parent_configurations * child_classes` must fit the checked int32/u32
histogram domain.

The function then reads the training partition's retained positions alongside
its source rows. For every row and every parent it requires a present,
non-missing, in-dictionary code. It performs the same check for the child.
Missing values, absent prepared storage, and out-of-dictionary codes are
`TrainingCompileErrorKind::InvalidTargetMatrix` through `bayes_vector_error`,
with the vector name, source index, and original source row in the detail.
The reference set stores these codes verbatim. It never stores a count table,
probability, sampled state, or host-computed posterior.

### Reference-set invariants and continuation

`validate_categorical_reference_set` is called after preparation, after a
resume append, and by the artifact validator. It enforces:

* at least one parent, one reference row, and one child label;
* parent and child code lengths equal their row shapes;
* `parent_codes.len() == reference_rows * parents.len()` without overflow;
* source-row count equals reference-row count;
* all names, source identities, and dictionaries are nonempty and unique within
  the conditional;
* every dictionary label is unique;
* each parent cardinality is exactly dictionary length plus one;
* mixed-radix metadata recomputes to the stored multipliers and product;
* every saved parent code is in its known dictionary range, not the reserved
  route; and
* every saved child code is in the child dictionary range.

`BayesianCategoricalReferenceSet::append` compares the complete parent and
child schemas, cardinalities, multipliers, and configuration product before it
reserves and appends source rows and codes. Row counts and allocations use
checked arithmetic and `try_reserve_exact`. Saved evidence remains before
current evidence. Repeated rows are intentionally retained as repeated
evidence, not deduplicated.

The append operation returns `InvalidNetwork` when schemas or declaration order
differ, and `ArithmeticOverflow` for a row count or reservation overflow.
Final validation runs again after the append, so no partially compatible model
escapes the boundary.

## Specialized training and semantic artifact state

### `compile_bayes_model`

After the policy checks and optional resume load, `compile_bayes_model` converts
the public `BayesDependency` values to byte-oriented training dependencies,
prepares the current dataset, creates one reference set per declaration, and
constructs `BayesModelArtifact::from_conditionals`. A compatible saved model
then calls `BayesModelArtifact::continue_with(current)`; without a saved model,
the current artifact is returned unchanged.

This is a semantic preparation operation, even though the public method is
named `compile_bayes_model`. It has no `CompiledTraining`, no
`StaticCalculationProgram`, no parameter or moment images, no run identity, no
bundle identity, and no native preparation. Native inference later compiles a
different graph from this artifact.

The public result is wrapped by `TrainingReport::bayes`:

* `kind()` is `TrainingModelKind::Bayes`;
* `bayes_model()` returns the immutable semantic artifact;
* `run()`, `bundle()`, `journal()`, `native_kernels()`, `native_evidence()`,
  and `training_evidence()` return `None`;
* `external_outputs()` and `metrics()` are empty; and
* `validation_status()` is `NotRequested`, while `gracefully_stopped()` is
  always `false` because no loop exists.

`TrainingReport::save_model` dispatches a Bayes report to
`BayesModelArtifact::save`. `save_native_kernel` instead returns the explicit
unsupported detail `categorical Bayesian observation preparation does not
realize a native training kernel artifact`. The branch writes the semantic
model only after all preparation and validation succeeds. A save write failure
is returned as `TrainingError::Checkpoint` and no success report is returned.

### Artifact versions and fields

The paired [`training/src/bayes_checkpoint.rs`](../../src/bayes_checkpoint.rs)
module owns the public semantic image. `BayesModelArtifact` contains only a
format version, the bit representation of the canonical smoothing constant,
and an ordered vector of reference sets. It never contains fitted histogram
counts or opaque native state.

| Version | Conditionals | Canonical root layout |
| --- | ---: | --- |
| `1` | Exactly one | `recipe-bayes-model`, `format-version`, `smoothing`, then one reference directly under the root. |
| `2` | Two or more | `recipe-bayes-model`, `format-version`, `smoothing`, then `conditionals/conditional` entries in repeated declaration order. |

Each reference contains `reference-rows`, fixed-width hexadecimal
`reference-source-rows`, ordered `parents/parent` schemas, one `child` schema,
fixed-width hexadecimal row-major `reference-parent-codes`, and hexadecimal
`reference-child-codes`. Each schema contains its numeric source index, raw
name bytes, and ordered `labels/value-bytes` dictionary entries. Source and
label bytes are encoded as lowercase `0x` hexadecimal. Int32 values are
eight-hex-digit words, source rows are sixteen-hex-digit words.

`encode` validates the artifact before producing canonical OGDL. `save` accepts
only a path ending in `.ogdl` and writes it through the shared atomic-save
boundary. A `.cubin` or `.hsaco` path is not a second spelling for this
artifact and is rejected by the public declaration before this method.

### Artifact validation and resume compatibility

`validate_artifact` enforces the version-to-conditional-count relationship,
exact Laplace-one smoothing bits, every reference-set invariant, and the
multi-output identity contract:

* all conditionals retain the same ordered reference source rows and row count;
* child names and child source identities are unique across conditionals;
* a repeated node name must have the same complete schema everywhere it occurs;
* a repeated source identity must have the same complete schema everywhere it
  occurs; and
* no conditional parent may be another conditional's child, by name or source
  identity.

The last rule is the role boundary that prevents a target child from becoming
an implicit evidence input to another output. The current instrument does not
perform ancestral prediction, target propagation, or marginalization.

`continue_with` validates both operands, requires equal format versions,
smoothing bits, and conditional counts, appends each saved/current pair in
order, and validates the aggregate again. Consequently:

* a singular version-one model cannot resume a repeated version-two model, or
  vice versa;
* changed child order, parent order, names, source identities, dictionaries,
  parent cardinalities, or mixed-radix metadata is incompatible; and
* repeated source rows are evidence multiplicity and remain in saved-before-
  current order.

The public `.resume("model.ogdl")` declaration is existence-conditional. A
missing file starts a new observed model and does not suppress an independent
`.save("new.ogdl")`. An existing file is bounded-read and strictly decoded
before current data preparation. A present but malformed, noncanonical,
incompatible, or over-limit file fails the run rather than silently starting a
new model.

### Strict bounded decoding

`BayesModelDecodeLimits::default` bounds source bytes, OGDL nodes, conditional
count, aggregate parent count, aggregate label count, aggregate reference rows,
and total decoded payload bytes. `load_bayes_model_file` converts the source
bound to the ingest source limit, reads one regular-file snapshot, and calls
`decode_bayes_model`.

The decoder fails closed in this order:

1. source length, UTF-8, OGDL parsing, and node count;
2. exactly one `recipe-bayes-model` root and one canonical `format-version`
   field;
3. version-specific allowed fields, required fields, and conditional count;
4. exact `smoothing/laplace-one` spelling;
5. reference row counts, aggregate parent and label limits, and shape-derived
   code lengths;
6. canonical decimal scalar values and fixed-width `0x` hexadecimal payloads;
7. reconstructed parent cardinalities and mixed-radix metadata;
8. full artifact validation; and
9. byte-for-byte equality between the decoded artifact's canonical re-encoding
   and the source text.

The decoder reports all failures as `CheckpointError::InvalidManifest` through
the training checkpoint boundary. It distinguishes source-bound arithmetic and
file-read failures through `InferencePreparationError::ArithmeticOverflow` or
`CheckpointSource`, and the public training boundary exposes those as
`TrainingError::Resume` or `TrainingError::Runtime` as appropriate. Unknown,
duplicate, or missing fields, noncanonical numbers, invalid hex, inconsistent
lengths, negative or zero cardinalities, int32 or u64 overflow, and payload
limit violations are not repaired or defaulted.

## Native inference preparation

### Model-family selection at the public inference boundary

Target-free inference is a separate public declaration. The caller supplies
`recipe.data(...)` without `.target(...)`, `.split(...)`, or `.norm(...)`, then
`recipe.model().load("model.ogdl")` and `recipe.infer().evaluate()`.
`Infer::log` accepts only `Time` and `Device`; loss, accuracy, target metrics,
epoch, and learning-rate metrics are rejected before compilation.

`src/inference.rs::compile_inference_package` validates policy, data, and model,
then requires a loaded `.ogdl` or `.gguf` source. For `.ogdl`,
`load_semantic_model_file` reads a bounded snapshot, probes only the first root
line, and dispatches strictly:

```text
recipe                 -> dense CheckpointArtifact
recipe-knn-model       -> KnnModelArtifact
recipe-bayes-model    -> BayesModelArtifact
anything else          -> unknown semantic-model root error
```

There is no decoder fallback. A Bayes root is decoded by the strict Bayesian
decoder and becomes `SemanticModelArtifact::Bayes`. `distill_data` then reads
the target-free source, `select_target_free_data` applies only the caller's
column and row exclusions, and `prepare_bayes_inference_table` binds those
rows to the saved parent schemas.

### Union of shared parent schemas

`prepare_bayes_inference_table` walks conditionals in declaration order and
parents in parent order. It inserts the first schema for each parent source
identity into one physical inference feature list; a shared parent is read
once. Each feature is a categorical dictionary schema carrying the saved
source index, name, and dictionary. The resulting `PreparedBayesInference`
retains both the artifact and the prepared target-free rows.

The saved dictionary is authoritative. Inference preparation does not refit a
dictionary, normalize labels, infer a target, or read a child column. Known
labels keep their saved codes. Unseen or missing parent labels use the ingest
reserved route, which is exactly the final code permitted by the saved parent
cardinality. A source/schema mismatch, missing parent, unsupported encoding,
or malformed row is returned as a typed `InferencePreparationError::Data`.

`load_and_prepare_bayes_inference` combines bounded model loading and table
preparation for callers that already own a `DistilledDataset`. The public root
library normally reaches the same two steps through
`compile_inference_package`.

## Inference graph and posterior calculation

### Compiled model contract

`compile_prepared_bayes_inference` is the target-free graph boundary. It
requires a nonzero query-row count and at least one conditional. For each
conditional it checks that reference rows, parent count, parent configuration
count, and child class count fit the operation domains, then calls
`compile_bayes_conditional`.

The final `CompiledInference` has:

* `InferenceTask::BayesProbabilities { width }`, where `width` is the sum of
  all child dictionary lengths;
* `InferencePredictionKind::BayesProbabilities`;
* one F32 output tensor of shape
  `[query_rows, sum(child_classes)]`;
* one output block per conditional, concatenated in repeated `.bayes(...)`
  order; and
* a one-iteration `StaticCalculationProgram` with every input admitted in
  `init` and the sole probability matrix egressing in `exit`.

The output width sum, table lengths, byte counts, and identity reservations use
checked arithmetic. A width or shape overflow is an
`InferenceCompileErrorKind::ArithmeticOverflow` or
`UnsupportedExtent`, never a truncation.

### Per-conditional external inputs

`compile_bayes_conditional` obtains the saved dimensions and builds query code
rows by looking up every saved parent feature by both source identity and name.
Each query code must be nonnegative and less than the corresponding saved
parent cardinality. It then admits the following immutable inputs:

| `InferenceInputRole` | Dtype and shape | Contents |
| --- | --- | --- |
| `BayesReferenceParents { conditional }` | I32 `[reference_rows, parent_count]` | Saved row-major parent codes. |
| `BayesReferenceChild { conditional }` | I32 `[reference_rows]` | Saved child class codes. |
| `BayesQueryParents { conditional }` | I32 `[query_rows, parent_count]` | Prepared target-free parent codes, including reserved unseen routes. |
| `BayesParentMultipliers { conditional }` | I32 `[parent_count]` | Saved mixed-radix multipliers. |
| `BayesParentCardinalities { conditional }` | I32 `[parent_count]` | Known dictionary counts plus one unseen route. |

The output tensor is an F32 `[query_rows, child_classes]` tensor. The compiler
reserves exactly the operation's ten intermediate value IDs and eleven kernel
IDs in an `IdentityNamespace` before appending the materialized fragment. The
workspace limit passed to the operation is its checked requirement, so the
operation cannot allocate an unaccounted transient image.

### Native payload graph

`ops/src/bayes.rs::materialize_categorical_bayes_inference` emits this fixed
primitive sequence for one conditional:

| Step | Inputs and output | Primitive and meaning |
| ---: | --- | --- |
| 1 | Reference parent codes, multipliers, cardinalities -> `[reference_rows, parent_count]` | Elementwise `code * multiplier`, with device `Require(0 <= code < cardinality)`. |
| 2 | Contributions -> `[reference_rows]` | Sum reduction over parent axis 1. |
| 3 | Reference configurations and child codes -> `[reference_rows]` | Elementwise `configuration * child_classes + child_code`. |
| 4 | Joint bin IDs -> `[parent_configurations * child_classes]` | Relaxed atomic unweighted histogram. |
| 5 | Query parent codes, multipliers, cardinalities -> `[query_rows, parent_count]` | Same checked mixed-radix contribution. |
| 6 | Query contributions -> `[query_rows, 1]` | Sum reduction over parent axis 1, retaining the dimension. |
| 7 | Empty input -> `[1, child_classes]` | `IndexMap` produces class offsets `0..child_classes`. |
| 8 | Query configuration and class offsets -> `[query_rows, child_classes]` | Elementwise query joint bin IDs. |
| 9 | Histogram counts and query bin IDs -> `[query_rows, child_classes]` | Bounds-rejecting gather of selected counts. |
| 10 | Selected counts -> `[query_rows, 1]` | Sum reduction over child classes, retaining the dimension. |
| 11 | Selected counts and totals -> output F32 matrix | Posterior elementwise program. |

The device posterior program validates nonnegative counts and totals, converts
them to F32, and emits for every query row `r`, class `c`:

```text
probability[r,c] = (count[r,c] + smoothing)
                   / (total[r] + smoothing * child_classes)
```

The public artifact passes `smoothing = 1.0`, so this is fixed Laplace-one
smoothing. The operation request itself checks only that smoothing is finite
and positive; the artifact and decoder enforce the stronger canonical
Laplace-one contract. A query configuration containing a reserved unseen
parent route has no reference histogram counts, making every child probability
`1 / child_classes`. Child classes never receive an unseen route.

Every scalar code and bin program uses device `Require` checks. Histogram bin
counts, dimensions, and scalar constants remain in checked int32/u32 domains.
The reductions use the caller's power-of-two tree lane setting, bounded to
`1..=1024`; the current compiler passes
`MAXIMUM_REDUCTION_TREE_LANES`.

### Workspace and identity accounting

`categorical_bayes_inference_requirements` rejects zero reference rows, query
rows, parent count, parent configurations, or child classes. It checks the
joint bin product against both int32 and u32, checks dimensions against int32,
and computes ten I32 intermediate allocations. If

```text
R  = reference_rows
Q  = query_rows
P  = parent_count
G  = parent_configurations
K  = child_classes
J  = G * K
RP = R * P
QP = Q * P
QK = Q * K
```

then transient workspace elements are exactly
`RP + R + R + J + QP + Q + K + QK + QK + Q`. The reported byte requirement is
that sum multiplied by four. The external output tensor is not included in
this transient workspace formula because its storage belongs to the caller's
boundary tensor.

`BayesEmitter` allocates ten intermediate tensors and eleven kernels in the
reserved half-open identity ranges. It forbids every input-to-output alias,
checks workspace arithmetic, checks range overflow, and verifies that the
emitted counts and bytes equal the formula before validating the graph. A
namespace too small, a boundary tensor inside the reserved range, a conflicting
boundary contract, or a workspace limit violation returns a typed
`OperationError` rather than moving an identity or silently allocating more.

`append_categorical_bayes_inference` first validates that the caller graph has
one unique, contract-matching tensor for every boundary input and output. It
rejects an existing producer for the probability output, identity collisions
with existing tensors or kernels, and any graph materialization mismatch. Only
the fragment's intermediate tensors are appended; the caller-owned boundary
tensors remain the authoritative external inputs and output.

### Repeated-output concatenation

For two or more conditionals, `compile_prepared_bayes_inference` keeps each
conditional's probability tensor independent until all posterior fragments are
complete. `concatenate_bayes_probabilities` joins them in declaration order.
For each join it computes a row-major destination table with:

* a left index for every destination element;
* a right index for every destination element; and
* a `select_left` I32 mask that selects the left block before the cumulative
  width and the right block afterward.

The index tables are immutable external I32 inputs with roles
`BayesConcatenationLeftIndices`, `BayesConcatenationRightIndices`, and
`BayesConcatenationSelectLeft`. The existing F32 probability blocks are packed
to flat F32 views, `gpu_concat_into` is materialized through the ordinary
composition boundary, and the result is reinterpreted back to
`[query_rows, cumulative_width]`. All left, right, and total element counts
must fit the checked I32 domain and `usize`; table and byte arithmetic is
checked.

The final output is one F32 matrix, not one output tensor per child. Its
adjacent column ranges are cumulative child dictionary widths in repeated
declaration order. The public report computes the same ranges from the saved
artifact, so a caller can map an output block back to its child without
re-fitting or inspecting graph internals.

`InferenceGraphCompiler::finish` marks exactly the declared external inputs and
the final concatenated probability tensor as boundaries, validates the graph,
round-trips it through canonical OGDL, constructs a one-iteration
`StaticCalculationProgram`, and round-trips the program text as well. The
round-trip is structural canonicalization, not a second implementation of the
Bayes calculation.

## Native lifecycle and authoritative state

The Bayes graph uses the same native execution boundary as dense and GGUF
inference. It does not use the training loop. `src/inference.rs` obtains a
measured profile and a scoped CUDA/HSA preparation, derives runtime tuning from
the actual graph, builds a production `Preparer`, and dispatches the
`CompiledModelInference::Bayes` variant to
`prepare_and_execute_local_inference`.

The inference executor enforces the following before any device work:

* the compiled program has exactly one loop iteration;
* no user metric tasks are present;
* no loop-phase transfer has an external source or destination;
* each declared external input is unique, canonical contiguous row-major,
  present in the graph, and has exact dtype, shape, and byte count;
* the declared input roles are inference-allowed, including every Bayes role;
* exactly one external output exists, it is F32, it is produced by a graph node,
  and its shape and kind match `InferenceTask::BayesProbabilities`; and
* the finalized bundle maps that output to an exit transfer from a real device
  value.

Preparation realizes native artifacts and warms the selected candidate before
the input image is admitted. The resulting lifecycle is:

```text
prepare graph and measured native candidate
    -> initialize one external input image per finalized device
    -> start the one-iteration loop
    -> poll until LoopStatus::Complete
    -> enter exited loop and run exit
    -> collect and validate the sole external probability image
    -> destroy native resources and return CompletedInferenceExecution
```

All reference codes, query codes, dictionaries, multipliers, and concatenation
tables enter through `init`. There is no file, host data, or model transfer in
the loop. The completed result owns copied post-exit bytes, a run and bundle
identity, realized native kernels, native evidence, elapsed loop time, and the
run journal. Any executor, handoff, output mapping, dtype, size, overlap, or
cleanup failure retains the typed `InferenceRunFailure` evidence where the
execution boundary supports it.

The authoritative state transition is therefore the finalized native graph and
its external exit image. The host does not calculate posterior values, argmax
classes, or dictionary codes during the device loop. It only validates the
post-exit image and later interprets the already returned bytes for reporting.

## Public inference report and output interpretation

`InferenceReportPayload::Bayes` retains the decoded `BayesModelArtifact` beside
the `CompletedInferenceExecution`. The report exposes:

| Accessor | Bayes behavior |
| --- | --- |
| `kind`, `run`, `bundle`, `elapsed`, `journal`, `native_evidence` | Identifies the completed native Bayes inference lifecycle. |
| `prediction`, `values` | Returns the one validated F32 probability matrix or its little-endian F32 iterator. |
| `native_kernels` | Returns the realized native image set for this inference run. |
| `bayes_output_count` | Number of saved conditionals, zero for another model family. |
| `bayes_output_name(output)` | Saved child name for one output block. |
| `bayes_output_classes(output)` | Saved child dictionary length. |
| `bayes_output_range(output)` | Cumulative column range in the packed probability row. |
| `decode_bayes_output_class(output, class)` | Saved dictionary bytes for a class code. |
| `decode_bayes_class(class)` | Compatibility accessor for output zero. |
| `knn_predictions` and dense class decoders | `None` for a Bayes report. |

Before formatting, `write_bayes_prediction_rows` checks that the prediction
kind is `BayesProbabilities`, the tensor is rank two, its width equals the sum
of all saved child dictionary lengths, and its bytes equal the exact matrix
shape. It then uses deterministic total-order argmax for the displayed class,
so an exact probability tie selects the lowest saved class code. It prints the
saved label bytes, never a host-inferred label.

For one conditional, a row is emitted as:

```text
prediction  <row>  class  <code>  label  "<saved label>"  probabilities  [p0,p1,...]
```

For repeated conditionals, one record is emitted for each output block and
also includes the output index and saved target name:

```text
prediction  <row>  output  <output>  target  "<child name>"
            class  <code>  label  "<saved label>"  probabilities  [p0,...]
```

`Time` and `Device` logs are printed before these rows when requested by
`Infer`. Bayesian training reports print no metric rows because they do not
have an execution. The inference report is returned only after native teardown
and output validation.

## Error and invariant index

The same real boundary can fail at several typed layers. The implementation
does not add a fallback path between them.

| Stage | Error boundary | Representative failures |
| --- | --- | --- |
| Public declarations | `DeclarationError` and `TrainingError::Declaration` | Empty names, duplicate parent or child, self edge, cycle, mixed checkpoint/inline model, repeated resume/save declaration, invalid artifact extension. |
| Data loading | `DataPreparationError` and `TrainingError::Data` | Missing target or split, bounded source or ingest failure, semantic vector inference failure, exclusion or split failure. |
| DAG resolution | `BayesianSchemaError`, wrapped as `TrainingCompileError::InvalidNetwork` | Duplicate prepared vector name, latent structural node, duplicate edge, self dependency, cycle. |
| Reference preparation | `TrainingCompileError` | No parents, no training rows, child not target, parent not feature, noncategorical or empty dictionary, missing/out-of-dictionary row code, target order mismatch. |
| Shape and metadata | `TrainingCompileError::UnsupportedExtent` or `ArithmeticOverflow` | Parent cardinality, mixed-radix multiplier/product, child classes, joint histogram domain, row-by-parent shape, or allocation overflow. |
| Artifact construction/resume | `CheckpointError` and `TrainingError::Checkpoint` or `Resume` | Unsupported version, version/count mismatch, non-Laplace smoothing, schema drift, source-row partition drift, child-as-parent, malformed OGDL, noncanonical text, decode limit, or atomic write failure. |
| Inference table preparation | `InferencePreparationError` | Bounded model source failure, invalid root, strict decode failure, missing saved parent feature, wrong categorical encoding, missing query row, or unknown semantic root. |
| Inference graph compilation | `InferenceCompileError` | Empty query set, no conditional, query code outside saved cardinality, dimension or output-width overflow, operation shape mismatch, identity exhaustion, or invalid graph/OGDL round-trip. |
| Native Bayes materialization | `OperationError` | Zero dimensions, int32/u32 histogram domain overflow, nonpositive smoothing, invalid tree lanes, boundary contract mismatch, namespace overlap/exhaustion, workspace limit/formula mismatch, graph validation failure. |
| Native execution | `InferenceExecutionError` and `InferenceError::Execute` | Wrong loop count, metrics or loop external transfer, duplicate/unbound input, image mismatch/overlap, missing/duplicate/unexpected output, output source mismatch, loop failure, handoff failure, or post-exit validation/cleanup failure. |
| Public reporting | `io::Error` through `InferenceError::Runtime` | Wrong Bayes output kind/rank/width, byte count mismatch, or impossible saved dictionary/range mapping. |

Important invariants are checked at the nearest owner:

* declaration shape belongs to `src/api.rs`;
* vector role, categorical encoding, row completeness, source order, and
  mixed-radix metadata belong to `training/src/bayes.rs`;
* artifact version, canonical text, aggregate identities, and resume contracts
  belong to `bayes_checkpoint.rs`;
* tensor contracts, primitive IDs, workspace, and aliasing belong to
  `ops/src/bayes.rs`;
* graph boundary, lifecycle phase, external image ownership, and exit output
  mapping belong to `training/src/execute.rs`; and
* packed output width and saved-label interpretation belong to
  `src/inference.rs`.

No later stage assumes that an earlier stage's text, status, or success message
proves a posterior result. Each stage consumes typed state and independently
checks the concrete contract required for its next action.

## End-to-end cookbook paths

### Singular conditional

The checked-in `examples/cookbook.rs:413-421` path is:

```rust
recipe.data("examples/datasets/cookbook/bayes.csv")
    .target("play")
    .split(0.8);
recipe.model().bayes("play", ["weather", "wind"]);
recipe.train().save("cookbook-bayes.ogdl").run()?;

recipe.data("examples/datasets/cookbook/bayes.csv").exclude("play");
recipe.model().load("cookbook-bayes.ogdl");
recipe.infer().log([Time, Device]).evaluate()?;
```

The five-row source is split at 80 percent. The four training rows become the
saved reference partition; the validation row is not saved. The artifact's
parent order is `weather`, then `wind`; the child dictionary contains `falcon`
and `otter`. The saved reference codes are the row-major codes for the first
four source rows, and every parent has a reserved third route for an unseen
query label even though the saved dictionary has two labels. Inference excludes
the target column, prepares both parent features from the saved dictionaries,
and returns one `[query_rows, 2]` probability matrix. The report prints each
row's argmax label and both probabilities, then exposes the realized native
inference evidence.

The checked-in `cookbook-bayes.ogdl` is the canonical version-one image with
`format-version 1`, `smoothing laplace-one`, four reference rows, source rows
`0,1,2,3`, and the exact dictionary/code payload. It contains no count table.

### Repeated conditionals with one shared parent

The `examples/cookbook.rs:398-411` path declares:

```rust
recipe.data("examples/datasets/cookbook/bayes_multi.csv")
    .target(["play", "travel"])
    .split(0.8);
recipe.model()
    .bayes("play", ["weather", "wind"])
    .bayes("travel", ["weather"]);
recipe.train().save("cookbook-bayes-multi.ogdl").run()?;

recipe.data("examples/datasets/cookbook/bayes_multi.csv")
    .exclude(["play", "travel"]);
recipe.model().load("cookbook-bayes-multi.ogdl");
recipe.infer().log([Time, Device]).evaluate()?;
```

The two target identities and declarations have the same order. The first
conditional retains parents `weather, wind` and child `play`; the second
retains parent `weather` and child `travel`. The shared `weather` feature is
prepared once for query rows, but each conditional independently builds its
own reference histogram and posterior. The version-two artifact stores two
`conditional` entries in declaration order. The final output width is
`classes(play) + classes(travel)`, and each row's adjacent ranges map to
`bayes_output_range(0)` and `bayes_output_range(1)`.

A target child cannot serve as the other conditional's parent. In this example
`play` and `travel` are targets, so only the observed feature `weather` and
`wind` are valid parents. The branch never implies a joint ancestral query.

### Resume and missing resume behavior

To continue a saved observation model, repeat the same complete declaration,
data schema, target order, and split, then use `.resume("old.ogdl")` and an
independent `.save("new.ogdl")`. Existing observations are decoded and
appended before current observations. If `old.ogdl` does not exist, current
observations form a fresh model and `new.ogdl` is still written. If the old
file exists but its parent order, dictionary, child order, source-row partition,
conditional count, or canonical smoothing differs, the run fails with an
incompatible checkpoint error.

Resume is semantic-only. Supplying a native kernel as the first or only resume
path is invalid at the public API; supplying a second native path reaches the
Bayes policy rejection because no training kernel exists.

## Deliberate non-goals and current boundaries

The current implementation intentionally does not reinterpret a declaration
outside the observed categorical contract:

* no latent nodes or implicit state spaces;
* no continuous, Gaussian, Bernoulli-feature, or custom-prior distributions;
* no target child used as another conditional's parent;
* no missing training observations, imputation, or host-side count filling;
* no target-free child column required at inference;
* no generic objective, optimizer, learning-rate schedule, epoch loop, or
  training metric;
* no native training kernel artifact, native training resume image, journal,
  plan, cache, or profile in the user-owned model export; and
* no fallback decoder, alternate graph, or host probability implementation.

The graph compiler and native operation code are nevertheless fully native for
inference. Preparation stores exact observations so the posterior calculation,
histogram reduction, and unseen-label behavior remain explicit Recipe payload
calculations on the selected measured target. A successful Bayes `.train()`
call therefore means semantic observation preparation and optional `.ogdl`
export; a successful Bayes `.infer()` call means the complete native
histogram/posterior graph ran through the ordinary measured
`init -> loop -> exit` lifecycle and its typed output passed the exit boundary.
