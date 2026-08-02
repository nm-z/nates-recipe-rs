# Bayesian semantic checkpoint

This document describes the observed categorical Bayesian semantic model owned by
`training/src/bayes_checkpoint.rs`. The module is a model-artifact codec and
validator. It is not a training-loop checkpoint, a native-kernel container, a
fitted histogram cache, or an inference executor.

## Intent and boundary

A public `.bayes(child, parents)` declaration describes a categorical conditional
distribution. Preparation retains the exact observations needed to reproduce
that distribution on the native target. The semantic model therefore stores:

- the exact source identity and byte name of every parent and child;
- each categorical dictionary in its original code order;
- the prepared reference-row order and source-row identities;
- row-major `int32` parent and child observation codes; and
- checked mixed-radix metadata for parent configurations.

It deliberately does not store host-computed counts, probabilities, optimizer
state, a random seed, an execution journal, a plan, a device allocation, or
native bytes. Native inference rebuilds the histogram and Laplace posterior from
the saved observations. The artifact is thus portable across supported native
targets, subject to the normal inference preparation and operation contracts.

The module uses the shared `CheckpointError`, `CheckpointResult`, and
`checkpoint::atomic_save` definitions from `training/src/checkpoint.rs`, but its
document format and validation errors are Bayesian-specific
`CheckpointError::InvalidManifest` values.

The public graph gate is separate from artifact validation. `Model::bayes` keeps
each dependency in source order and rejects an empty child or parent name, a
self-parent edge, duplicate parents in one declaration, a duplicate child, or a
cycle. `resolve_bayesian_schema` in `training/src/bayes.rs` additionally rejects
duplicate prepared vector names and gives every absent graph node an explicit
latent source classification. The executable observed slice then rejects those
latent classifications, so a structurally resolvable graph is not by itself a
checkpointable Bayesian model.

## End-to-end ownership map

| Boundary | Concrete implementation | Bayesian checkpoint role |
| --- | --- | --- |
| Public declaration | `src/api.rs`, `Model::bayes` and `Train::{resume,save}` | Retains declarations in call order and validates path shapes before execution. |
| Dataset preparation | `training/src/bayes.rs`, `prepare_categorical_bayesian_reference_sets` | Resolves observed categorical vectors, checks target and feature roles, captures codes and source rows. |
| Artifact construction | `BayesModelArtifact::new` or `from_conditionals` | Selects semantic format version and validates every reference set. |
| Resume | `src/training.rs`, `compile_bayes_model` and `BayesModelArtifact::continue_with` | Loads an existing `.ogdl` if it exists, checks exact contracts, and appends current raw observations. |
| Export | `BayesModelArtifact::save`, reached by `TrainingReport::save_model` | Encodes canonical OGDL and installs it atomically as a `.ogdl` file. |
| Model loading | `training/src/inference.rs`, `load_bayes_model_file` or `load_semantic_model_file` | Bounded source snapshot, strict decode, and model-root dispatch. |
| Query preparation | `prepare_bayes_inference_table` | Builds the union of saved parent schemas and prepares target-free rows. |
| Native graph | `compile_prepared_bayes_inference` and `ops/src/bayes.rs` | Admits saved observations as external `int32` inputs and materializes histogram plus posterior calculations. |
| Public result | `src/inference.rs`, `InferenceReport` | Executes the ordinary native lifecycle and reports packed per-conditional probabilities and saved labels. |

A Bayes run is a reference-model preparation, not an optimizer run. The Bayes
branch in `Train::try_run_with` returns before native preparation, native
allocation, metric collection, journal creation, or kernel realization.

## In-memory semantic state

`BayesModelArtifact` (`training/src/bayes_checkpoint.rs:49-60`) contains three
private fields:

```text
format_version: u32
smoothing_bits: u32
conditionals: Vec<BayesianCategoricalReferenceSet>
```

`smoothing_bits` is stored as the exact `f32::to_bits` image of
`CATEGORICAL_BAYES_SMOOTHING`, which is currently `1.0` in
`training/src/bayes.rs`. The public `smoothing()` accessor reconstructs the
`f32` from those bits. There is no user-selected prior in this instrument.

`BayesianCategoricalReferenceSet` (`training/src/bayes.rs:187-205`) contains:

```text
parents: Vec<BayesianCategoricalSchema>
child: BayesianCategoricalSchema
reference_source_rows: Vec<usize>
reference_rows: usize
parent_codes: Vec<i32>
child_codes: Vec<i32>
parent_cardinalities: Vec<i32>
parent_multipliers: Vec<i32>
parent_configurations: u64
```

A `BayesianCategoricalSchema` (`training/src/bayes.rs:165-185`) is the source
index, byte name, and ordered dictionary for one observed categorical vector.
A dictionary entry at index `n` is the saved known code `n`. Parent
`parent_cardinalities` are one larger than the known dictionary length, reserving
one query-only route for an unseen or missing label. Child dictionaries do not
reserve an unseen output class.

`parent_codes` is row-major: for each reference row, parent values appear in
literal `.bayes(child, [parents])` order. Its length is
`reference_rows * parents.len()`. `child_codes` and
`reference_source_rows` each have one element per reference row. The source rows
preserve the ingestion partition's order, including retained source-row gaps
caused by upstream filtering.

The mixed-radix multiplier for a parent is the product of the cardinalities to
its right. The resulting packed configuration is in
`0..parent_configurations`; the decoder recomputes this metadata from the
dictionaries instead of trusting serialized multipliers.

### Format versions

- Version 1 is the canonical singular image. It contains exactly one reference
  set directly under the root. `BayesModelArtifact::new` always constructs this
  image.
- Version 2 is the repeated-call image. It contains at least two conditionals
  under `conditionals/conditional`. `from_conditionals` chooses version 1 for a
  one-element vector and version 2 otherwise; validation rejects an empty vector
  and rejects version 2 with fewer than two entries.

`references()` is a compatibility accessor for the first conditional and assumes
  validation has established that at least one conditional exists. New callers
  should use `conditionals()` so repeated declarations remain visible.

## Constructing the reference observations

`training/src/bayes.rs` intentionally emits no graph or native work during
preparation. `prepare_categorical_bayesian_reference_sets` first resolves the
declaration graph, then enforces the executable observed slice
(`bayes.rs:292-357`):

1. At least one declaration is required.
2. Every resolved node must be observed. Latent roots and latent conditionals
   are rejected because this instrument has no explicit latent state space.
3. The training partition must contain at least one row.
4. Every conditional has at least one parent.
5. Every child resolves to a declared `VectorRole::Target`.
6. The target source-index list must exactly equal the declared child list in
   repeated declaration order.
7. Every parent resolves to a `VectorRole::Feature`.
8. Parent and child vectors must all be semantic categorical,
   dictionary-encoded, `int32` prepared values with a nonempty dictionary.
9. Every retained training row must have a known parent and child code. Missing
   values, absent prepared positions, and out-of-dictionary codes are errors.

For each dependency, preparation copies the dictionaries and schemas, computes
parent cardinalities and mixed-radix metadata, then copies source rows and raw
codes. It validates the completed reference set before returning it. The
single-dependency `prepare_categorical_bayesian_reference_set` wrapper enforces
exactly one declaration for the first executable slice and returns the sole
reference set.

`validate_categorical_reference_set` (`bayes.rs:490-584`) checks all shape
relationships, nonempty and unique node names/source identities, unique labels,
the one-reserved-route cardinality rule, recomputed mixed-radix metadata, and
known-code ranges. This validator is reused by artifact construction, decode,
and resume append.

## Artifact validation

`validate_artifact` (`training/src/bayes_checkpoint.rs:159-237`) is the
semantic-model invariant boundary. It rejects:

- any format version other than 1 or 2;
- an empty conditional list;
- a version-1 list whose length is not exactly one;
- a version-2 list with fewer than two conditionals;
- any smoothing bit pattern other than Recipe's Laplace-one bits;
- any invalid reference set reported by `validate_categorical_reference_set`;
- conditionals whose `reference_rows` differ from the first conditional;
- conditionals whose ordered `reference_source_rows` differ from the first;
- duplicate child names or duplicate child source identities; and
- repeated schemas with the same name or source identity but different content.

Finally, no parent may have a name or source identity used by another
conditional's child. This makes the repeated observed instrument a set of
independent target conditionals. It does not infer ancestral prediction,
evidence propagation, or marginalization from a target-as-parent edge.

`BayesModelArtifact::new` and `from_conditionals` build the artifact with the
canonical smoothing bits and immediately call this validator. `continue_with`
validates both operands before comparing their contracts and validates the
merged result afterward.

## Canonical OGDL encoding

`encode()` validates first, builds an OGDL `Graph` with `encode_graph`, and
returns `Graph::to_canonical_string().into_bytes()` (`bayes_checkpoint.rs:130-133
and :239-253`). The root text is exactly `recipe-bayes-model`.

### Version 1 shape

```text
recipe-bayes-model
	format-version	1
		smoothing	laplace-one
	reference-rows	<canonical decimal>
	reference-source-rows	0x<16 hex digits per source row>
	parents
		parent
			source-index	<canonical decimal>
			name-bytes	0x<two hex digits per byte>
			labels
				value-bytes	0x<...>
	child
		source-index	...
		name-bytes	0x<...>
		labels
			value-bytes	0x<...>
	reference-parent-codes	0x<8 hex digits per int32 code>
	reference-child-codes	0x<8 hex digits per int32 code>
```

Version 2 retains `format-version` and `smoothing` at the root and wraps each
reference set in declaration order:

```text
recipe-bayes-model
	format-version	2
	smoothing	laplace-one
	conditionals
		conditional
			<the version-1 reference fields, without a second root>
		conditional
			<next reference fields>
```

`encode_reference` writes row count, source rows, parents, child, parent codes,
and child codes in that fixed order. `encode_schema` writes source index, byte
name, and every dictionary label in dictionary order. Byte values are lowercase
hexadecimal with a `0x` prefix. `encode_i32_hex` writes each signed code as its
eight-digit two's-complement `u32` image. `encode_usize_hex` writes each source
row as a 16-digit `u64` image and fails if a host `usize` cannot fit in `u64`.
OGDL graph construction failures are converted to `InvalidManifest` with an
encoding-specific detail.

The graph serializer is the canonicality authority. Decoder acceptance requires
the exact bytes produced by this encoder, so field order, indentation, decimal
spelling, hexadecimal case, and trailing formatting are not alternate input
spellings.

## Strict decoding

`decode_bayes_model` creates a stateful `Decoder` and then decodes the same source
bytes (`bayes_checkpoint.rs:155-157`). The decoder performs these stages:

1. Reject source bytes above `limits.source_bytes`.
2. Require UTF-8.
3. Parse OGDL and reject a graph above `limits.nodes`.
4. Require exactly one root whose text is `recipe-bayes-model`.
5. Parse exactly one canonical decimal `format-version` field.
6. For version 1, require exactly the eight root fields for one reference set,
   require `smoothing == laplace-one`, then decode one reference.
7. For version 2, require exactly `format-version`, `smoothing`, and
   `conditionals`; require two through `limits.conditionals` child nodes, each
   named `conditional`, then decode each reference in order.
8. Rebuild the artifact with the canonical smoothing bits and run
   `validate_artifact`.
9. Re-encode the artifact and compare bytes with the original source. A valid
   but noncanonical document fails with `Bayesian model is valid but not in
   canonical textual OGDL form`.

`fields` rejects unknown fields, duplicate fields, and missing required fields.
`scalar` requires exactly one leaf child and rejects descendants. Numeric fields
are parsed as `u32` or `usize` and must round-trip through their canonical
decimal spelling. Every hex payload requires `0x`, an even byte length where
appropriate, exact element-derived length for code/source-row arrays, and valid
hexadecimal digits.

### Reference and schema reconstruction

`decode_reference_fields` parses `reference-rows`, accumulates the count against
the aggregate row limit, requires at least one parent, decodes every `parent`
schema, decodes the child schema, and derives expected array lengths:

```text
source rows:       reference_rows
parent codes:      reference_rows * parent_count
child codes:       reference_rows
```

It derives each parent cardinality as `dictionary.len() + 1`, converts it to
`i32`, and recomputes multipliers and `parent_configurations` with `mixed_radix`.
Serialized cardinality and multiplier fields are intentionally absent, so they
cannot disagree with the saved dictionaries.

`decode_schema` requires source index, byte name, and a nonempty `labels` node.
It counts labels globally, requires each label node to be `value-bytes`, and
decodes each byte string. The later shared reference-set validator catches empty
names, duplicate identities, duplicate labels, and invalid code ranges.

### Finite decode limits

`BayesModelDecodeLimits::default()` (`bayes_checkpoint.rs:22-47`) is:

| Counter | Default |
| --- | ---: |
| `source_bytes` | `1 << 30` |
| `nodes` | `4_000_000` |
| `conditionals` | `65_536` |
| `parents` | `65_536` |
| `labels` | `1_000_000` |
| `reference_rows` | `100_000_000` |
| `total_payload_bytes` | `1 << 30` |

The decoder tracks aggregate parents, labels, and reference rows across all
conditionals. `decode_bytes`, `decode_i32_hex`, and `decode_usize_hex` charge
decoded bytes to `payload_bytes` before allocation. Checked additions and
multiplications turn integer overflow into `InvalidManifest`; a payload above
`total_payload_bytes` is rejected. A source row encoded in 64 bits must also fit
the host `usize` on decode.

The Bayesian decoder reports malformed input through `CheckpointError::InvalidManifest`
rather than the path-addressed `CheckpointDecodeError` hierarchy used by the
dense checkpoint decoder. `load_bayes_model_file` adds bounded regular-file
snapshot errors as `InferencePreparationError::CheckpointSource`, then maps
Bayesian manifest errors to `InferencePreparationError::Checkpoint`.

## Resume and append semantics

The public `Train::resume` API accepts one `.ogdl` model path. The literal
two-path form is lowered by `src/source_frontend.rs` to
`__recipe_resume_pair`; `src/api.rs` requires the first path to end in `.ogdl`
and the second to end in `.cubin` or `.hsaco`. A kernel-only resume is rejected
at declaration time because a native image has no semantic observations.

`compile_bayes_model` (`src/training.rs:505-579`) is the Bayes resume caller:

1. It validates policy, data, and model declarations.
2. It requires at least one Bayesian dependency and rejects layer blocks, a
   loaded model, a generic objective, gradient clipping, normalization,
   optimizer, learning rate, epoch or warmup bounds, and iterative log/plot
   metrics. These declarations have no meaning for observation preparation.
3. It rejects a native resume kernel or native save destination because the
   observed Bayesian model has no native training kernel.
4. If a semantic resume path was declared, it checks `try_exists`. A missing
   path is normal and means no resume model is loaded. An existing path is read
   through `load_bayes_model_file` with default finite limits.
5. It prepares the current dataset and converts public dependencies to
   `BayesianDependency` values in repeated-call order.
6. It builds the current `BayesModelArtifact` with
   `from_conditionals(prepare_categorical_bayesian_reference_sets(...))`.
7. If a saved artifact exists, it calls `saved.continue_with(current)`;
   otherwise it returns the current artifact unchanged.

`continue_with` first validates both artifacts. It requires identical format
version and smoothing bits and an identical conditional count. Each saved
reference set then calls `BayesianCategoricalReferenceSet::append`, which
requires exact equality of parent schemas, child schema, cardinalities,
multipliers, and configuration count. It reserves capacity, appends source rows,
parent codes, and child codes in that order, adds reference row counts with
checked arithmetic, and validates again. Consequently saved observations always
precede current observations, repeated rows remain repeated statistical
evidence, and schema, dictionary, parent order, child order, and declaration
order drift is rejected.

The missing-file branch is existence-conditional, not an error. A present but
malformed, noncanonical, incompatible, or otherwise invalid model fails the
run. The API's save and resume declarations are independent, so a missing resume
model does not disable a declared model save.

## Export and artifact lifecycle

`TrainingReport` has a `Bayes(BayesModelArtifact)` payload. Its `bayes_model()`
accessor exposes the semantic artifact; `run`, `bundle`, journal, metrics,
native-kernel, and native-evidence accessors return `None` or empty values for
Bayes because no native training lifecycle occurred.

The report's private `save_native_kernel` dispatch also returns the explicit
unsupported error `categorical Bayesian observation preparation has no native
training kernel artifact` for a Bayes payload. The normal public path rejects a
Bayes kernel destination earlier in `compile_bayes_model`, so this dispatch is a
second typed boundary rather than a fallback export mechanism.

In `Train::try_run_with` (`src/training.rs:869-879`), the presence of any Bayes
dependency selects the Bayes branch. It builds the report, calls
`report.save_model(destination)` only when a model destination was declared,
and returns. It does not call the dense native execution path. A native kernel
destination reaches the earlier `compile_bayes_model` rejection, so the current
implementation never exports `.cubin` or `.hsaco` for Bayes.

`BayesModelArtifact::save` accepts only a path whose extension is exactly
`.ogdl`, calls `encode`, converts the encoded length to `u64`, and delegates to
`atomic_save`. The shared atomic writer validates the parent directory and
target type, checks filesystem capacity while preserving the configured user
reservation, writes a private `0600` temporary file, flushes and syncs it,
verifies the measured byte length, renames it into place, and syncs the parent
directory. Errors include invalid target, insufficient capacity, filesystem I/O,
encoding/manifest failure, and a size conversion failure. No journal, plan,
profile, native image, or intermediate checkpoint is written by this path.

## Inference handoff

`training/src/inference.rs` exposes a direct bounded loader:
`load_bayes_model_file(path, limits)` reads a regular-file source snapshot under
the requested byte bound and calls `decode_bayes_model` (`inference.rs:711-723`).
`load_and_prepare_bayes_inference` composes that loader with query preparation.

`load_semantic_model_file` reads one bounded `.ogdl` snapshot, probes only the
first line, and dispatches `recipe-bayes-model` to
`decode_bayes_model` (`inference.rs:725-780`). The probe uses the default Bayes
limits after selecting the root. A caller needing custom Bayes limits should use
`load_bayes_model_file` directly.

`prepare_bayes_inference_table` (`inference.rs:797-821`) walks conditionals and
parents in saved order, creates the union of parent schemas, and reads each
shared parent once by source identity. The child vectors are intentionally not
required in a target-free query table. Ingestion applies the saved categorical
dictionaries, including the reserved unseen route.

`compile_prepared_bayes_inference` rejects zero query rows and an empty
conditional list. For each conditional it:

- checks reference rows, parent count, child class count, and operation
  requirements;
- locates every prepared query parent by saved source identity and name;
- requires dictionary-coded `int32` values and checks each query code against
  the saved parent cardinality;
- admits reference parent codes, reference child codes, query parent codes,
  parent multipliers, and parent cardinalities as external `int32` tensors;
- asks `ops/src/bayes.rs` to materialize the histogram and posterior graph; and
- retains one f32 `[query_rows, child_classes]` probability tensor per
  conditional.

`ops/src/bayes.rs` computes each parent configuration by summing
`code * multiplier`, maps each reference row to
`configuration * child_classes + child_code`, histograms those bins, gathers
the query configuration's class counts, reduces class totals, and emits

```text
(count + smoothing) / (total + smoothing * child_classes)
```

with Recipe's fixed smoothing of one. It checks all tensor dtypes/shapes,
positive finite smoothing, nonzero dimensions, int32 histogram bounds, identity
namespace capacity, workspace bytes, and forbidden boundary aliasing. New query
labels use the reserved parent route; an unobserved configuration therefore has
zero counts and a uniform posterior.

For repeated conditionals, `concatenate_bayes_probabilities` joins each matrix
on the device in declaration order. The final task is
`InferenceTask::BayesProbabilities { width: sum(child_classes) }`, with adjacent
column ranges corresponding to conditional order. The normal native preparation,
realization, execution, output collection, and teardown then run through
`prepare_and_execute_local_inference`; the semantic artifact remains immutable.

`src/inference.rs` retains the artifact in `InferenceReportPayload::Bayes`. The
report validates the final matrix shape and byte count before writing rows. For
each row and conditional it reports the argmax class, decodes that class through
the saved dictionary, and prints all probabilities. The argmax scan replaces its
best class only on a strictly greater `total_cmp` result, so an exact tie keeps
the lowest saved class code. Public accessors expose the conditional count,
saved target name, class count, packed output range, and dictionary label without
refitting or host-side probability calculation.

## Error surface by stage

- Declaration errors come from `Model::bayes`, `BayesDependency::validate`,
  network duplicate-child/cycle checks, and `Train::{resume,save}` path checks.
- Preparation errors use `TrainingCompileError` kinds such as
  `InvalidNetwork`, `InvalidTargetMatrix`, `EmptyDataset`,
  `UnsupportedExtent`, and `ArithmeticOverflow` for absent vectors, wrong
  roles/types, missing observations, shape overflow, and state-space limits.
- Artifact construction, decode, append, encode, and `.ogdl` save use
  `CheckpointError::InvalidManifest` for semantic violations and
  `CheckpointError::InvalidTarget`, `InsufficientCapacity`, or `Io` for export
  boundary failures.
- Inference source and decode failures are wrapped in
  `InferencePreparationError`; graph shape, extent, and consistency failures
  are `InferenceCompileError`; operation materialization failures come from the
  operation error contract. Final report shape or byte mismatches become
  `std::io::Error` while writing the public report.

These layers intentionally do not add fallback decoders, inferred dictionaries,
host-side count tables, alternate smoothing, implicit latent state, or native
kernel substitutes. The saved semantic bytes and their validated schemas are
the sole model state carried from preparation and resume into inference.
