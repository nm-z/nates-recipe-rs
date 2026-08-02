# Generic dense checkpoints

The generic checkpoint is Recipe's semantic dense-training artifact. It is the
boundary between a completed native training run and a later training or
inference declaration. The implementation is concentrated in
`training/src/checkpoint.rs`; `training/src/inference.rs` owns bounded file
loading and target-free schema preparation, `src/training.rs` owns the public
training terminal and declaration routing, and `training/src/execute.rs` owns
the native lifecycle that produces the checkpoint egress images.

A generic checkpoint is not a journal, a plan, a cache, a device allocation, or
a native handle. It contains the row-free dataset schema, the effective model
topology, fitted normalization, final parameter bytes, AdamW moments, and the
identities that authenticate realized native images. A semantic `.ogdl` model
never embeds native image bytes. A native `.cubin` or `.hsaco` companion is an
independent file containing the exact realized image bytes.

The public artifact contract is:

| Declaration | Result |
| --- | --- |
| no `.save(...)` | no user-owned model or kernel artifact |
| `.save("model.ogdl")` | one semantic model |
| `.save("kernel.cubin")` or `.save("kernel.hsaco")` | one native image |
| `.save("model.ogdl", "kernel.cubin")` or the HSA form | both artifacts |

The pair forms are literal two-argument methods. The source frontend rewrites
them to the hidden `__recipe_save_pair` and `__recipe_resume_pair` methods so
Rust can type-check the public syntax without changing the public API shape.

## Source map and ownership

The following symbols are the generic checkpoint path. They are intentionally
listed by responsibility because the path crosses the facade, compiler,
executor, and file boundary.

| Source | Responsibility |
| --- | --- |
| `src/api.rs`, `Train::resume`, `Train::__recipe_resume_pair` | Validate and retain one optional model path and one optional native path. |
| `src/api.rs`, `Train::save`, `Train::__recipe_save_pair` | Route one or two output paths by extension. |
| `src/source_frontend.rs`, the `RecipeReceiver::Train` rewrite | Lower literal two-argument `.save` and `.resume` calls. |
| `src/training.rs`, `Train::run` and `Train::try_run_with` | Consume the preceding declarations, dispatch dense versus KNN or Bayesian preparation, execute dense training, then perform declared exports. |
| `src/training.rs`, `compile_training_graph` | Build the new static training graph and, only when an existing model path is present, load and admit its state. |
| `src/training.rs`, `compile_training_package` | Join the compiled graph, optional resumed model, optional resumed native bundle, and a declaration-only manifest. |
| `src/training.rs`, `load_resume_native_bundle` | Authenticate an optional supplied kernel and return its bytes for prebuilt preparation. |
| `src/training.rs`, `execute_current_training_native` | Check measured-system identity, choose prebuilt versus newly compiled native code, and call the real executor. |
| `training/src/execute.rs`, `prepare_and_execute_local_training_controlled` | Prepare the finalized bundle, admit init images, run the loop, perform exit egress, and return only after native teardown. |
| `training/src/checkpoint.rs`, `CheckpointManifest::from_compiled` | Capture declaration and tensor contracts without rows or payload bytes. |
| `training/src/checkpoint.rs`, `apply_checkpoint_resume` | Compare a decoded artifact with a newly compiled graph and replace its external resume inputs. |
| `training/src/checkpoint.rs`, `CompletedTrainingCheckpoint::new` | Join completed exit images with the manifest and attach authenticated native identities. |
| `training/src/checkpoint.rs`, `CompletedTrainingCheckpoint::save` | Encode the semantic model and atomically install it. |
| `training/src/checkpoint.rs`, `CompletedTrainingCheckpoint::save_native_kernel` | Select exactly one realized image of the requested format and atomically install its raw bytes. |
| `training/src/inference.rs`, `load_checkpoint_file` | Read a bounded regular-file snapshot and call the strict decoder. |
| `training/src/checkpoint.rs`, `decode_checkpoint` and `Decoder` | Parse versioned OGDL, enforce decode limits and canonical forms, and validate every semantic invariant. |
| `training/src/inference.rs`, `load_semantic_model_file` | Dispatch a `.ogdl` root to the dense, KNN, or Bayesian decoder. |

`training/src/lib.rs` re-exports the generic surface: `CheckpointArtifact`,
`CheckpointManifest`, `CheckpointError`, `CheckpointDecodeLimits`, the image
types, `decode_checkpoint`, `apply_checkpoint_resume`,
`compiled_training_program_digest`, `CompletedTrainingCheckpoint`, and the
file-loading and inference preparation functions.

KNN and observed categorical Bayesian preparation use their own semantic
artifact modules (`knn_checkpoint.rs` and `bayes_checkpoint.rs`). The public
`TrainingReport::save_model` dispatches to those implementations when the
model declaration selects those families. They have no generic optimizer
checkpoint, no generic native training kernel, and no path through
`CompletedTrainingCheckpoint`.

## Public declarations

`Train` stores `resume` and `save` as independent
`TrainingArtifactDeclaration` values, each with an optional model path and an
optional kernel path. Builder calls do not read files, probe hardware, compile
code, allocate buffers, or start a run.

### `.resume(...)`

* `.resume(path)` accepts exactly one nonempty `.ogdl` path and records it as
  the semantic model source.
* `.resume(model, kernel)` is represented by the literal two-argument form and
  requires a nonempty `.ogdl` first path plus a `.cubin` or `.hsaco` second
  path.
* A second resume declaration is deferred as a declaration error. The error
  tells the caller to use the literal pair form for a model plus kernel.
* A native resume path without a semantic model is rejected by
  `require_supported_policy`; a native image does not contain semantic
  training state.

The path extension is checked exactly, not case-folded. The first path in a
pair is always the semantic model. A kernel-only resume is therefore invalid.

### `.save(...)`

* `.save(path)` accepts exactly one nonempty path ending in `.ogdl`, `.cubin`,
  or `.hsaco`. `.ogdl` selects the semantic model; the native extensions select
  a kernel image.
* `.save(model, kernel)` requires the first path to end in `.ogdl` and the
  second path to end in `.cubin` or `.hsaco`.
* A second save declaration is deferred as a declaration error. The pair form
  is the one way to request both artifacts.

Save and resume are independent declarations. Omitting resume starts a fresh
run and does not disable save. Omitting save performs no export. A declared
resume model is existence-conditional, as described below.

## End-to-end training path

The real user boundary is `recipe.train()...run()`, or the source-frontend
equivalent `.run(&model, &data)`. `Train::run` consumes the immediately
preceding data and model declarations. `Train::try_run_with` first dispatches
the specialized KNN and Bayesian paths. A dense declaration follows this
sequence:

1. `compile_training_package` calls `compile_training_graph`.
2. The data and model declarations are validated and prepared. The dense
   graph, all typed inputs, the full training partition, optimizer state, and
   external output boundary are compiled.
3. If `resume_source()` names an existing path, `load_checkpoint_file` reads
   and fully validates it, then `apply_checkpoint_resume` admits its state into
   the new graph. If the path does not exist, no read or admission occurs and
   the graph retains its fresh initialization.
4. `load_resume_native_bundle` handles a supplied second resume path. It is
   skipped when no kernel path was declared, when the model did not exist, or
   when the supplied kernel file is absent. In all of those cases the current
   measured system realizes a fresh native bundle.
5. `CheckpointManifest::from_compiled` captures the current declaration and
   tensor contracts. At this point it has no native realization metadata.
6. `execute_current_training_native` enters the measured native preparation
   boundary. It either installs an authenticated prebuilt bundle or compiles
   the current program for the measured target, then calls
   `prepare_and_execute_local_training_controlled`.
7. The executor prepares discovery, compilation, placement, allocation,
   queues, synchronization, and native images before execution. It admits the
   one external image for each device during `init`, runs the static loop, and
   performs checkpoint egress only in `exit`. Finalized loop tasks cannot be
   external transfers.
8. `CompletedTrainingExecution` is produced after `exit` and native teardown.
   `CompletedTrainingCheckpoint::new` attaches the realized native identities,
   maps every physical exit image to its logical `ValueId`, and validates the
   completed manifest boundary.
9. `try_run_with` performs the independently declared model export and kernel
   export. The report is returned only after all requested writes succeed.

The executor's output mapping is itself a hard boundary. Every graph tensor
marked `external_output` must have exactly one finalized exit transfer from a
device to the external endpoint, and every such transfer must map to one
logical `ValueId`. Missing, duplicate, unexpected, wrong-dtype, or wrong-size
images fail before a semantic file is written.

The saved tensor set is therefore the authoritative final state of the
completed run, not a host-side reconstruction. For each learned parameter the
set includes its updated parameter, updated first moment, and updated second
moment. Normalization tensors, K-means centroids, tree split tensors, and
temperature calibration are also selected from external outputs when their
declarations require them.

## Declaration-only manifest

`CheckpointManifest` is the typed bridge between a compiled graph and a
completed execution. `from_compiled` captures:

* the format version selected by the effective topology;
* every row-free vector schema, preserving source index and declaration order;
* feature spans, feature width, and the derived numeric normalization mask;
* target source identities in declaration order and the complete `DenseTask`;
* an optional legacy linear output adapter;
* loss, data-normalization, learning-rate schedule, clipping, AdamW, seed,
  reduction-tree, epoch, warmup, and checked execution bounds;
* fitted data-normalization tensors;
* the effective block declarations and state tensor contracts;
* optional temperature calibration;
* the SHA-256 digest of the canonical static program; and
* no native identities until a completed native execution is attached.

The manifest contains no source rows. `CheckpointTensor` stores a graph
`ValueId`, dtype, shape, and byte count. `checkpoint_tensor` accepts only graph
tensors marked `external_output`; a value without an external output
declaration is a manifest error. `validate_external_boundary` compares the
manifest's logical tensor set with the graph's external-output set and rejects
any missing, unexpected, or multiply assigned role.

The program digest is generated by `compiled_training_program_digest`: the
canonical static calculation program is encoded to OGDL and hashed with
SHA-256. External input bytes, including resumed parameter bytes, are outside
this identity. A supplied native image must implement the same static program,
but its runtime input state is not part of the digest.

### Format version selection

The manifest chooses the newest semantic version required by its topology. The
version constants and their actual accepted topology are:

| Version | Root format | Effective topology |
| ---: | --- | --- |
| 5 | `dense-training-checkpoint` | Legacy flat nonempty layer view. The block view must mirror the layers. |
| 6 | `dense-training-checkpoint` | Structured residual topology, without pool, convolution, K-means, or tree blocks. |
| 7 | `dense-training-checkpoint` | Structured topology containing pool, without convolution, K-means, or tree blocks. |
| 8 | `recipe-semantic-model` | Canonical structured topology retained for native metadata, still convolution-free and without K-means or tree. |
| 9 | `recipe-semantic-model` | Native structured topology; convolution is allowed, but K-means and tree blocks are not. |
| 10 | `recipe-semantic-model` | K-means-capable structured topology, without tree. |
| 11 | `recipe-semantic-model` | Multi-target-capable structured topology, without tree. |
| 12 | `recipe-semantic-model` | One terminal supervised tree block. |
| 13 | `recipe-semantic-model` | One leading fixed-token embedding, optionally followed by attention and ordinary blocks. |
| 14 | `recipe-semantic-model` | Structured topology containing a scalar-sequence vanilla RNN. |
| 15 | `recipe-semantic-model` | Structured topology containing a scalar-sequence reset-before GRU. |
| 16 | `recipe-semantic-model` | Structured topology containing a scalar-sequence zero-cell LSTM. |

Versions 5 through 7 retain the legacy root format and do not carry native
realization metadata. Versions 8 through 16 are semantic models and require
native realization metadata in a decoded completed artifact. Manifest
validation permits the metadata to be absent temporarily because the manifest
is created before native execution; `CompletedTrainingCheckpoint::new` fills
it from the realized kernel set before any public export.

## OGDL semantic model schema

`encode_artifact` emits one canonical OGDL document. The root is `recipe` with
the following fields, in this order:

```text
recipe
    format          dense-training-checkpoint | recipe-semantic-model
    version         5 .. 16 (the supported version set is explicit)
    semantics
    dataset
    training
    model
    native          (required for completed semantic versions, absent for v5-v7)
```

The decoder rejects extra, missing, and duplicate fields at each level. The
field order above describes the writer's canonical order; the OGDL parser still
identifies fields by name and semantic validation enforces all order-sensitive
collections.

### Semantics and dataset

`semantics` contains `objective`, `normalization`, and the literal `optimizer`
value `adamw`. Objective tokens are:

* `binary-cross-entropy-with-logits`;
* `binary-focal-with-logits-alpha-0.25-gamma-2`;
* `mean-squared-error`, `mean-absolute-error`, `cross-entropy`, or
  `huber-unit-delta`.

Data normalization is one of `identity`, `z-score`, `min-max`, or `l2-norm`.

`dataset` contains `feature-width`, a tagged `target`, `vectors`,
`feature-spans`, and `feature-normalization-mask`.

The target is one of:

* `binary-classification`: one `source-index`, `positive-code`;
* `multiclass-classification`: one `source-index`, `class-count`, and
  `reserved-unseen-code`;
* `scalar-regression`: one `source-index`;
* `multi-target-binary-classification`, `joint-multiclass-classification`, or
  `multi-target-regression`: an ordered `source-indices` list containing at
  least two identities.

Each `vector` stores `source-index`, `name-bytes`, `role`, `semantic-type`,
`encoding`, and tagged metadata. Source indices are strictly increasing in the
saved vector list and names are nonempty and unique. Roles are `feature` and
`target`; semantic types are `numeric`, `temporal`, `categorical`, `ordinal`,
`text`, `image`, and `binary`. Encodings are `f32`, `int32`,
`relative-seconds-int32`, `dictionary-int32`, `ordinal-int32`, `utf8`, and
`bytes`.

Metadata variants are:

* `none` for numeric, text, and binary forms;
* `temporal` with signed `unix-seconds` and `nanoseconds` below one billion;
* `categorical` with a canonical ascending byte dictionary;
* `ordinal` with a distinct, nonempty ordered byte dictionary; and
* `image` with a nonempty canonical set of encoded header variants.

`name-bytes`, dictionary labels, ordinal labels, and image-independent byte
values are written as lowercase `0x` hexadecimal. The decoder preserves bytes
exactly and validates UTF-8 only for the source document and identity labels
where those types require it.

Every `feature-spans` entry has `source-index`, `start`, `width`, and a tagged
`lowering`. Numeric features use `numeric-scalar` and width one. Categorical
features use `categorical-one-hot` with `dictionary-width`, `reserved-index`,
and a width equal to dictionary length plus the reserved route. Spans must be
contiguous, cover the declared feature width exactly, and appear in feature
vector order.

`feature-normalization-mask` is a list of `value-bits` f32 bit patterns. It is
derived from the spans: numeric scalar coordinates contain `1.0` bits and
categorical one-hot coordinates contain `0.0` bits. A decoded mask that differs
from this derivation is rejected.

### Training configuration and bounds

`training` stores:

* `epochs`, either a canonical nonzero integer or `unbounded` for semantic
  versions;
* `warmup-epochs`;
* `learning-rate-decay`, one of `constant`, `linear`, `cosine`, or
  `exponential`;
* `gradient-clip-norm`, as canonical f32 bits or `none` for semantic versions;
* `normalization-epsilon` as canonical f32 bits;
* `reduction-tree-lanes`;
* `random-seed`;
* `adamw` with binary32-bit `learning-rate`, `beta-one`, `beta-two`, `epsilon`,
  and `weight-decay`; and
* `bounds` containing `train-rows`, `epochs`, `training-iterations`,
  `calibration-iterations`, `iterations`, and `warmup-iterations`.

The bounds describe the actual static schedule. There is one full-partition
update per epoch, so finite `training-iterations` equals the epoch count and
`warmup-iterations` equals `warmup-epochs`. Finite total iterations equal
training plus calibration iterations. Unbounded training requires constant
post-warmup decay, zero calibration iterations, and unbounded total iterations.
Training rows are nonzero and fit the checked int32 index domain. A legacy
document must retain an explicit gradient clip value; `none` is a semantic
model feature.

Validation also requires reduction lanes to be a power of two in `1..=1024`,
finite positive normalization epsilon, learning rate, and AdamW epsilon,
nonnegative weight decay, and finite beta values in `[0, 1)`. Warmup cannot
exceed a finite horizon, and unbounded warmup must fit its int32 schedule
domain.

### Model and blocks

`model` stores `input-width`, `output-width`, optional tagged
`output-adapter linear-projection` (`source-width`, `target-width`), model
normalization tensors, `blocks`, and optional `calibration.temperature`.

The effective block list is canonical and order-sensitive. Parameter-bearing
blocks carry a `parameter`, `first-moment`, and `second-moment` tensor for every
learned value. A tensor has `dtype` (`f32` or `int32`), a zero-or-more extent
`shape` chain, and `payload raw-bytes-hex`. Payload chunks are at most 64 bytes;
all nonfinal chunks are exactly 64 bytes, and an empty payload is represented by
one `0x` chunk. Shape and byte counts must agree with the declared dtype.

The block forms are:

* `layer` or `perc`: width is encoded in the block tag, followed by `index`,
  operation list, optional ordered PReLU parameter list, `weight`, and `bias`.
* `convolution`: `index`, `filters`, `kernel`, resolved `input-length`,
  `input-channels`, `output-length`, operations, optional PReLU values, weight,
  and bias. The saved weight shape is
  `[kernel, input-channels, filters]`; bias shape is `[filters]`.
* `pool`: `size`, optional `group-to-neuron`, input length, channels, output
  length, `group-major-channel-minor` order, and the
  `lowest-logical-index` winner contract. Pools own no parameters.
* `kmeans`: cluster count, optional routing width, input width, and updated
  centroid tensor. Centroids are loop-carried state, not AdamW parameters.
* `tree`: family (`lightgbm`, `catboost`, or `xgboost`), tree count, depth,
  input/output widths, internal-node and leaf counts, split-feature and
  split-threshold tensors, and leaf values with moments. A supervised tree is
  terminal and has no output adapter.
* `embedding`: dimensions, vocabulary, sequence length, and one learned table.
* `attention`: sequence length, dimensions, heads, head dimension, and query,
  key, value, and output projection parameters.
* `rnn`: sequence length, width, input weight, recurrent weight, and bias.
* `gru`: sequence length, width, and nine reset, update, and candidate
  parameter triples.
* `lstm`: sequence length, width, and twelve input-gate, forget-gate,
  output-gate, and candidate parameter triples.
* `residual`: ordered branch layer or operation steps, branch PReLU values,
  output width, identity or weight-only linear-projection skip, output
  operations, and output PReLU values.

Every block has an `index` equal to its position in the enclosing list. The
decoder rejects a reordered or repeated index. Structured validation checks
the logical shape carried from one block to the next, the final task width,
classification logits, residual skip width, pool and K-means routing masks,
convolution geometry, tree extents and split-feature range, and recurrent or
embedding sequence contracts.

### Native realization metadata

Semantic versions append `native` with nonzero `program`, `realization`,
`topology`, and `discovery` digests. Each `kernel` contains:

* `format`, either `cubin` or `hsaco`;
* `target.backend`, `target.architecture`, and `target.abi`;
* `toolchain.name`, `toolchain.version`, and its nonzero digest; and
* the nonzero digest of the native image bytes.

The decoder and validator require the backend and ABI to agree with the file
format: `.cubin` uses `nvidia-cuda-driver` and `elf64-cubin`; `.hsaco` uses
`amd-rocr-hsa` and an `elf64-amdgpu-code-object-v...` ABI. Identity labels may
not contain OGDL delimiter characters. Duplicate native identities are
rejected. The metadata authenticates a companion file but never carries its
bytes.

## Strict decoding and validation

`load_checkpoint_file` turns the configured `CheckpointDecodeLimits.source_bytes`
into a checked `u64`, reads a regular-file snapshot through
`read_source_snapshot`, then calls `decode_checkpoint`. A source that is zero,
unrepresentable, non-regular, too large, unreadable, or otherwise invalid is a
typed `InferencePreparationError` and then a `TrainingError::Resume` when used
for training.

`decode_checkpoint` enforces the source-byte limit, UTF-8, OGDL syntax, and a
node-count limit before constructing `Decoder`. The decoder then enforces:

* one root named `recipe`;
* required and optional field sets at every object, with unknown and duplicate
  fields rejected;
* one scalar child for scalar fields and no descendants on scalar values;
* tagged values with a valid tag and only the permitted payload fields;
* canonical unsigned and signed decimal forms, with no leading zeroes or
  negative zero;
* lowercase hexadecimal f32 bit strings and byte strings;
* 32-byte nonzero digests;
* limits for vectors, feature spans, blocks, metadata entries, tensors, tensor
  rank, each tensor payload, total payload, and OGDL nodes; and
* exact payload chunk sizes and canonical empty payload representation.

After parsing, `validate_artifact` runs the full semantic validator with payload
checking. The same validator is used in declaration mode by
`validate_manifest_semantic_invariants`, so a manifest cannot describe a
payload shape that a completed artifact would later reject.

Validation is path-addressed. `CheckpointPath` renders locations such as
`<checkpoint>.dataset.vectors[2].metadata`, and `CheckpointDecodeError` exposes
the stable error kind, path, and detail. Error kinds are `LimitExceeded`,
`InvalidUtf8`, `InvalidSyntax`, `MissingField`, `DuplicateField`,
`UnknownField`, `InvalidValue`, and `InconsistentValue`.

Semantic validation covers all of the following:

* version and root-format compatibility, including the topology allowed by
  each historical version;
* flat versus structured storage, legacy layer views, and canonical block
  presence;
* objective and task compatibility, target count, target order, source roles,
  classification codes, categorical dictionaries, regression representation,
  and multi-target numeric restrictions;
* unique increasing source indices, unique nonempty names, metadata contracts,
  image header admissibility and canonical ordering, contiguous feature spans,
  and the derived feature-normalization mask;
* input and output widths, logits requirements, output-adapter nonredundancy,
  model normalization tensor count and shape, calibration shape, and every
  learned tensor's dtype, rank, extent, byte count, and finite f32 payload;
* pool extents and winner contract, routed dense zero entries, K-means cluster
  bounds and routed zero entries, tree depth and extents, split-feature range,
  convolution geometry and activation count, recurrent dimensions, embedding
  token schema, and residual branch and skip topology; and
* training horizon, schedule, bounds, AdamW ranges, reduction lanes, native
  identities, backend and ABI, and duplicate kernel identities.

The payload validator also inspects every f32 and int32 payload. It is not
enough for a file to parse or for a byte count to match. A semantic model must
represent a graph that the current dense compiler can execute and resume.

## Resume admission

`apply_checkpoint_resume(training, checkpoint)` is the only generic resume
admission function. It never replaces the compiled graph or its identities.
The new declaration owns the new training phase. The checkpoint supplies only
the state required to continue the same semantic program.

The function performs these steps:

1. Fully validate the artifact, including payloads.
2. Build a declaration-only manifest from the newly compiled training graph.
3. Compare the saved and current row-free semantics.
4. Traverse the saved and current topologies in one canonical parameter order.
5. Validate every compiled external resume input and replace its bytes.

Compatibility requires exact equality of:

* feature width, feature spans, derived normalization mask, target source
  order, task, and output adapter;
* vector count and every source index, name, role, semantic type, encoding,
  and metadata value;
* objective, data normalization, normalization epsilon, AdamW beta-one,
  beta-two, epsilon, and weight decay;
* effective topology, including historical flat-layer declarations or each
  structured block's declaration, dimensions, geometry, routing, branch order,
  operation order, and skip kind; and
* the number, order, dtype, shape, and byte size of every parameter, first
  moment, and second moment. K-means centroid and tree split tensor contracts
  are checked separately as loop-carried or terminal state.

The compatibility comparison intentionally does not import the old schedule
position or accepted-update counter. A new learning rate, decay endpoint,
epoch bound, warmup declaration, random seed, or stopping control belongs to
the new `Train` declaration. The saved optimizer moments and updated
parameters are the continuation state; no old journal, plan, metric, native
handle, or runtime counter is resumed.

The canonical parameter tape is shared by manifest and artifact traversal. Its
order is:

* embedding table;
* attention query, key, value, output;
* RNN input, recurrent, bias;
* GRU reset, update, candidate triples;
* LSTM input, forget, output, candidate triples;
* ordinary layer or convolution weight, bias, then ordered PReLU values;
* no parameters for pool or K-means; K-means centroids are separate;
* tree leaf values, with split features and thresholds separate; and
* residual branch layers and PReLU operations in branch order, optional
  projection, then residual output PReLU values.

For each parameter ordinal, `apply_checkpoint_resume` expects exactly three
compiled external inputs. Components are identified by
`ExternalInputRole::ResumeParameter`, `ResumeFirstMoment`, and
`ResumeSecondMoment`. It also expects one `ResumeEnabled` int32 scalar and sets
that scalar to `1`. K-means centroid and tree split inputs are matched by block
index and admitted independently. Duplicate roles, absent roles, wrong dtype,
wrong shape, wrong byte size, or a missing block input produce
`CheckpointError::IncompatibleResume`.

The final checks require one resume-enable input, a `0b111` component mask for
every parameter, and exact equality between saved K-means or tree block sets
and the compiled special-state inputs. The graph, external-output identity,
and native-artifact contract remain unchanged.

### Existence-conditional paths

`compile_training_graph` calls `Path::try_exists` before loading the model:

* existing `.ogdl`: bounded read, strict decode, compatibility check, and state
  admission;
* missing `.ogdl`: no error, no state admission, fresh initialization; and
* filesystem inspection error: a runtime error at `inspect training resume
  model`.

`load_resume_native_bundle` is called only after an existing model was decoded.
For a supplied kernel path:

* an absent `.cubin` or `.hsaco` is a normal miss and returns `None`;
* an existing file requires semantic native metadata, the current static
  program digest, exactly one authenticated kernel identity of that format,
  and bytes whose SHA-256 digest equals the saved identity;
* a malformed or unreadable source is a `NativeKernelSource` error;
* a model with no matching format, or with multiple matching identities, is an
  incompatible-resume error; and
* a filesystem inspection error is reported at `inspect training resume
  kernel`.

When the path is absent, or when no kernel path was declared, execution enters
the ordinary measured preparation path. A missing kernel is not converted into
an invented artifact and is not treated as a failed resume.

## Native recompilation and authentication

`execute_current_training_native` receives an optional `ResumeNativeBundle`.
When present it first checks that the current measured topology and discovery
identities equal the saved identities. It then checks that the current target
plan contains the authenticated target and toolchain. The deferred compiler
receives the prebuilt bytes only after these checks.

When no bundle is present, the same production preparer realizes the current
compiled program from the current measured profile. This is the normal path for
a fresh run, a model-only resume, a missing supplied kernel, or a missing
resume model. The native image is generated for the current measured system,
loaded, warmed, and retained in `RealizedNativeKernelSet` for the completed
report.

`CheckpointNativeRealization` stores only identities: program, realization,
topology, discovery, target, toolchain, and image digests. A companion file is
accepted only when its raw bytes match the authenticated digest. This prevents
an old or unrelated kernel from silently entering a new static program.

## Completed export

`CompletedTrainingCheckpoint::new` can succeed only with a
`CompletedTrainingExecution` whose native resources have already been
destroyed. It adds native realization metadata to the declaration manifest,
validates semantic invariants again, and maps the execution's sorted
`ExitImage` values to the manifest's logical tensor IDs.

### Semantic `.ogdl`

`CompletedTrainingCheckpoint::save` obtains the exact bytes for every manifest
tensor from its mapped exit image. It measures the canonical encoding with a
`CountingWriter`, then calls `atomic_save` with that exact size. The writer
revalidates the manifest and the fully populated artifact before emitting
OGDL. Native bytes are never inserted into this file.

### Native `.cubin` or `.hsaco`

`save_native_kernel` first requires that the target extension equals the
requested `NativeKernelFormat`. It filters the retained realized kernels by
format:

* no matching image returns `NativeKernelUnavailable`;
* exactly one matching image is selected; and
* more than one distinct matching image returns `NativeKernelAmbiguous`.

The selected `RealizedNativeKernel::bytes` are written unchanged. The method
does not regenerate, wrap, OGDL-encode, or concatenate images.

### Atomic file safety

`atomic_save` is shared by semantic and native exports. It:

1. Requires a nonempty file target and an existing directory parent.
2. Rejects a target that is a symbolic link or non-regular object. Existing
   regular files are replaceable.
3. Queries filesystem capacity, rounds the measured encoded size to the
   filesystem fragment size, and preserves `EXACT_USER_RESERVATION` bytes.
4. Creates a private mode `0600` temporary file with a unique name, retrying
   only its bounded sequence of names.
5. Writes, flushes, and `sync_all`s the temporary file; checks that the final
   length equals the measured length; renames it onto the target; and syncs the
   parent directory.
6. Removes the temporary file on any failure through `TemporaryGuard`.

The capacity check, temporary write, byte-count check, rename, and directory
sync all have typed `CheckpointError::Io` or target/capacity errors. A failed
export does not leave a partially written target.

## Error surface

The generic `CheckpointError` variants and their boundaries are:

| Variant | Meaning |
| --- | --- |
| `Decode(CheckpointDecodeError)` | Bounded OGDL, canonical-value, structural, or semantic decode failure with a stable path. |
| `InvalidManifest` | A newly compiled declaration or state cannot describe a valid checkpoint boundary. |
| `IncompatibleResume` | The saved model, topology, tensor contract, or authenticated kernel cannot continue the new declaration. |
| `DuplicateOutput` | One logical checkpoint value appeared in multiple physical exit images. |
| `MissingOutput` | A manifest tensor had no physical exit image. |
| `UnexpectedOutput` | Execution returned an image for a value outside the manifest tensor set. |
| `OutputDTypeMismatch` | Physical exit dtype differs from the graph tensor contract. |
| `OutputSizeMismatch` | Physical source or byte length differs from the graph tensor contract. |
| `NativeKernelUnavailable` | No retained image has the requested native format for export. |
| `NativeKernelAmbiguous` | More than one distinct retained image has the requested format, so one file would be ambiguous. |
| `InvalidTarget` | Path is empty, has an invalid parent or object type, has the wrong native extension, or cannot be safely staged. |
| `InsufficientCapacity` | The target filesystem cannot hold the rounded output while preserving the required reservation. |
| `Io` | A named filesystem operation failed, with operation, path, and source error. |

The public `TrainingError` wraps these at the user boundary. Resume source and
strict decode failures are `TrainingError::Resume`; export failures are
`TrainingError::Checkpoint`; an existing native source read failure is
`TrainingError::NativeKernelSource`; measured preparation and executor failures
retain their native or runtime categories. Missing model and missing kernel
paths are intentionally not errors.

## Target-free inference role

`load_semantic_model_file` reads one bounded `.ogdl` source and probes its first
root line. `recipe` selects `decode_checkpoint`; `recipe-knn-model` and
`recipe-bayes-model` select their specialized decoders. There is no fallback
from one decoder to another after the root is selected.

For a dense model, `prepare_checkpoint_inference` applies only the saved
feature schema and saved row-free semantics to newly selected target-free
rows. It does not fit dictionaries, use training targets, use a train split,
or fit new normalization. `compile_prepared_inference` emits the saved
normalization, topology, parameter bytes, and output interpretation as Recipe
calculations. Optimizer moments are not admitted to the inference graph.

Inference has its own native `init -> loop -> exit` lifecycle and one prediction
egress. A model-only `.ogdl` export is therefore sufficient for target-free
inference; a `.cubin` or `.hsaco` companion is not required. The saved native
metadata authenticates an optional training-resume kernel, not inference
state.

## Real artifact cases

`acceptance/src/main.rs::run_artifact_gate` drives the public `recipe run`
entrypoint through real generated Rust sources and checks the resulting
directory independently. The gate covers nine cases:

1. no save, expecting no files;
2. model-only save, expecting one `.ogdl`;
3. kernel-only save, expecting one `.cubin`;
4. model plus kernel save, expecting exactly both;
5. missing model resume plus independent save, expecting only the fresh model;
6. existing model resume plus save;
7. existing model and kernel resume plus pair save;
8. existing model plus an absent kernel path, proving current recompilation and
   pair export; and
9. graceful stop at a completed epoch followed by a model-only resume.

Every expected file must be nonempty, and no journal, plan, cache, profile,
temporary, or intermediate checkpoint is accepted as a user artifact. The
acceptance runner separately verifies that a dense run has one real full-
partition update per epoch, GPU-only calculation payload work, one pre-loop
native image load, no loop-time realization, and zero live native resources
after teardown.

The cookbook examples exercise the same path for ordinary dense, residual,
tree, convolution and pooling, K-means, embedding and attention, RNN, GRU,
LSTM, and multi-target declarations. The model saved by the first run is
redeclared with the same schema and topology before `.resume(...)`; the resumed
run writes a separate semantic artifact and can then be loaded by
`.model().load(...).infer().evaluate()`.

## Invariants to preserve

The generic checkpoint boundary is correct only when all of these remain true:

* semantic state is row-free but complete enough for exact schema-bound
  inference and optimizer continuation;
* the new training declaration controls the new phase, while all parameter and
  AdamW-moment images are admitted in canonical order;
* K-means centroids and tree split tensors are carried as their own state and
  never mislabeled as AdamW parameters;
* native bytes remain outside `.ogdl`, and a supplied or exported native file
  is authenticated by target, toolchain, measured-system identity, program
  digest, and raw-byte digest;
* every checkpoint tensor is a real external graph output collected by one
  finalized `exit` transfer;
* model and kernel exports are independently optional, and a missing resume
  path starts fresh without suppressing an independent save;
* decode limits and canonical syntax are enforced before allocation expands
  with untrusted source data;
* all semantic, topology, shape, dtype, routing, and payload invariants are
  checked both for declarations and for completed bytes; and
* the only user-owned exports are the requested semantic `.ogdl` and optional
  realized `.cubin` or `.hsaco` files.
