# `LanguageError`

`recipe-language` uses one typed, fail-closed error boundary for malformed
shapes, layouts, tensors, scalar programs, primitive specifications, and
calculation graphs.  The boundary is defined in
[`language/src/error.rs`](../../src/error.rs#L1-L75).  It is a validation and
construction boundary, not an execution status: an `Err(LanguageError)` means
that the requested language object was not constructed, validated, encoded,
ordered, or costed.

The error vocabulary is intentionally smaller than the language surface.  A
primitive family does not get its own error type.  Its structural, dtype,
shape, and policy failures are classified by `LanguageErrorKind`, while the
`detail` string records the operation-specific observation.  Callers either
propagate the value unchanged through `LanguageResult` or convert it at a
crate boundary.  No language error causes a retry, a substituted tensor, a
default shape, a fake dispatch, or a partially returned graph.

## Public structure

### `LanguageErrorKind`

`LanguageErrorKind` is a public `Copy`, `Debug`, `PartialEq`, `Eq`,
`#[non_exhaustive]` enum.  Its current variants are:

```text
EmptyShape
InvalidAxis
DuplicateAxis
ShapeOverflow
ByteSizeOverflow
InvalidLayout
DuplicateTensor
DuplicateKernel
UnknownTensor
DuplicateProducer
MissingProducer
Cycle
ArityMismatch
DTypeMismatch
ShapeMismatch
InvalidScalarProgram
InvalidPrimitive
WorkOverflow
```

The non-exhaustive marker requires downstream matches to retain a wildcard.
The enum is re-exported by `language/src/lib.rs:18-36`, so users normally
import it as `recipe_language::LanguageErrorKind`.

### `LanguageError`

`LanguageError` is `Clone`, `Debug`, `PartialEq`, and `Eq`.  All four fields are
public:

| Field | Meaning |
| --- | --- |
| `kind: LanguageErrorKind` | Stable machine-readable category. |
| `detail: String` | Human-readable condition and observed values.  It is owned, so a caller can retain the error after the source object is dropped. |
| `value: Option<ValueId>` | Optional value or tensor context. |
| `kernel: Option<KernelTemplateId>` | Optional primitive or kernel context. |

`LanguageError::new(kind, detail)` accepts any `Into<String>` and initializes
both context fields to `None` ([`error.rs:36-45`](../../src/error.rs#L36-L45)).
`for_value` and `for_kernel` consume the error, set one context field, and
return it for fluent use ([`error.rs:47-57`](../../src/error.rs#L47-L57)).
They do not alter `kind` or `detail`, and calling both is valid.

### Rendering and source-chain behavior

`Display` is deterministic and has this exact order
([`error.rs:60-70`](../../src/error.rs#L60-L70)):

```text
{kind:?}: {detail} [kernel {kernel}] [value {value}]
```

The bracketed kernel and value portions are omitted when their options are
`None`.  If both are present, the kernel appears first, regardless of the
order in which the builder methods were called.  `kind` uses its Rust `Debug`
spelling, for example `ShapeMismatch`, rather than a separate display label.

`LanguageError` implements `std::error::Error` directly and does not override
`source` ([`error.rs:73`](../../src/error.rs#L73)); its source is therefore
always `None`.  The detail text is the only retained lower-level explanation.
Conversions that preserve a `LanguageError` as a typed child, such as
`OgdlCodecError::InvalidGraph`, provide the outer source chain.  Conversions
that stringify it retain the rendered text but not the child object.

### `LanguageResult<T>`

`LanguageResult<T>` is exactly `Result<T, LanguageError>`
([`error.rs:75`](../../src/error.rs#L75)).  The alias is re-exported from the
crate root.  Language constructors and validators use `?` to stop at the first
observed error.  They do not accumulate multiple `LanguageError` values.

## Construction and propagation boundaries

The following public language entry points return `LanguageResult` directly.
Each returns no object or derived value on error.

| Entry point | What is rejected or derived | Direct propagation |
| --- | --- | --- |
| `Shape::new`, `bytes`, `broadcast_result`, `reduced`, `gather_result` | Rank, element-count, byte-count, broadcasting, reduction, and gather metadata | Callers receive the original `LanguageError`; primitive validators add a kernel context when the shape is part of a kernel contract. |
| `AxisSet::new`, `validate_rank` | Empty, duplicate, or out-of-rank axes | Callers receive the original error; reduction, contraction, and other primitive validators add the current kernel ID with `for_kernel`. |
| `TensorLayout::contiguous`, `validate`, `span_elements`, `byte_offset` | Stride construction, non-overlap, span, and byte-offset contracts | `Tensor::contiguous` propagates layout and shape errors; `Tensor::validate` adds its value ID to layout and byte-span failures only at the explicit `for_value` sites. |
| `Tensor::contiguous`, `validate` | Typed storage construction and layout/storage agreement | Graph validation propagates tensor failures unchanged, while primitive lowering converts them to its own error boundary. |
| `CalculationGraph::assemble`, `validate`, `topological_order`, `dependencies` | Tensor/kernel indexes, producer uniqueness, dependency acyclicity, and query membership | Public graph methods return the first `LanguageError`; ordering and dependency queries validate before returning derived vectors. |
| `ScalarProgramBuilder` methods and `finish` | Builder ownership, scalar identity allocation, opcode signatures, and final core scalar validation | Math and training scalar builders use `?`; operation materialization converts the rendered text to `OperationError`. |
| `PrimitiveKernel::validate`, `work` | Tensor references, alias matrix, primitive contracts, and checked work arithmetic | Primitive lowering converts validation errors to `LoweringError`; planner and training tuning convert work errors to their own invalid-graph or runtime errors. |

The OGDL codec is the first typed wrapper around this boundary:

* `CalculationGraph::to_ogdl_graph` validates before encoding and converts a
  failure with `From<LanguageError>` to
  `OgdlCodecError::InvalidGraph` ([`ogdl.rs:89-103`](../../src/ogdl.rs#L89-L103)).
  `to_ogdl` delegates to that method, so it cannot emit text for an invalid
  graph.
* `decode_graph` constructs typed tensors and kernels, then calls
  `graph.validate()` ([`ogdl.rs:471-525`](../../src/ogdl.rs#L471-L525)).
  `decode_tensor` constructs each `Shape` with `Shape::new` at
  [`ogdl.rs:535-556`](../../src/ogdl.rs#L535-L556), and reduction decoding
  constructs each `AxisSet` with `AxisSet::new` at
  [`ogdl.rs:660-693`](../../src/ogdl.rs#L660-L693).  Those shape and axis
  errors also convert to `OgdlCodecError::InvalidGraph` through `?`, before
  the final graph validation.  A later semantic failure from
  `graph.validate()` is converted the same way.
  Syntax and document-shape errors are separate `OgdlCodecError` variants and
  never become `LanguageError`.
* `OgdlCodecError` displays an invalid graph as `invalid calculation graph:
  {language_error}` and exposes the `LanguageError` from `source()`
  ([`ogdl.rs:57-78`](../../src/ogdl.rs#L57-L78)).

The next workspace wrappers preserve the same fail-closed behavior but differ
in how much structure they retain:

| Boundary | Conversion site | Retained information and consequence |
| --- | --- | --- |
| `recipe-program` | `StaticCalculationProgram::validate` maps graph validation to `ProgramError::Graph(OgdlCodecError::InvalidGraph)` ([`program/src/lib.rs:99-103`](../../../program/src/lib.rs#L99-L103)); `ProgramError` also wraps `OgdlCodecError` as a source ([`program/src/lib.rs:554-610`](../../../program/src/lib.rs#L554-L610)). | The kind, detail, kernel, and value remain reachable through the OGDL child error. Program construction stops before lifecycle domains are accepted. |
| `recipe-primitives` | `lower` maps `kernel.validate` through `LoweringError::from` ([`primitives/src/lower.rs:52-60`](../../../primitives/src/lower.rs#L52-L60)); the conversion is [`primitives/src/error.rs:48-56`](../../../primitives/src/error.rs#L48-L56). | The lowerer classifies every language failure as `LoweringErrorKind::InvalidLanguage`, keeps only `detail`, `kernel`, and `value`, and drops the original `LanguageErrorKind`. No lowered program is returned. |
| `recipe-ops` | Shape, tensor, graph, and scalar-builder calls in the five graph-materialization families use `graph_error` or `language_error`. The common helper maps to `GraphMaterializationFailed` at [`ops/src/materialize.rs:4584-4593`](../../../ops/src/materialize.rs#L4584-L4593); family helpers are [`ops/src/bayes.rs:850-855`](../../../ops/src/bayes.rs#L850-L855), [`ops/src/knn_outputs.rs:1171-1175`](../../../ops/src/knn_outputs.rs#L1171-L1175), [`ops/src/kmeans.rs:839-843`](../../../ops/src/kmeans.rs#L839-L843), and [`ops/src/tree.rs:887-891`](../../../ops/src/tree.rs#L887-L891). | The rendered language text is retained as `OperationError.detail`, with an operation ID in the materializer helper. There is no typed source chain and no emitted primitive graph for the failed request. The materialization submodules that use these helpers are `materialize/{attention_sequence_embedding,convolution_pooling,graph_cluster_rl,indexing_sort_encoding,loss_metrics,training,tree_boosting}.rs`. |
| `recipe-math` | `MathFunction` construction returns `LanguageResult` from `program::build` ([`math/src/program.rs:17-27`](../../../math/src/program.rs#L17-L27)) and exposes it through `TryFrom<MathFunction>` and `exp_with_gradual_underflow_program` ([`math/src/lib.rs:23-42`](../../../math/src/lib.rs#L23-L42)). | A scalar math program is absent on failure. The public API exposes the original `LanguageError`, including its kind and context. |
| `recipe-training` scalar forward | Recurrent graph code requires `From<LanguageError>` in its graph-emission trait ([`training/src/forward.rs:80-113`](../../../training/src/forward.rs#L80-L113)); `lower_activation` has the same bound ([`training/src/forward.rs:367-388`](../../../training/src/forward.rs#L367-L388)). `attention_extent` and `sum_program` also construct language errors directly ([`training/src/forward.rs:426-450`](../../../training/src/forward.rs#L426-L450)). | Generic graph emitters propagate language failures into the caller's compile error. The direct training conversion stringifies them into `TrainingCompileErrorKind::Language` ([`training/src/error.rs:50-60`](../../../training/src/error.rs#L50-L60)); inference does the same for `InferenceCompileErrorKind::Language` ([`training/src/inference.rs:175-193`](../../../training/src/inference.rs#L175-L193)). The outer errors retain rendered text, not a source object. |
| `recipe-planner` and root training tuning | `PrimitiveKernel::work` is mapped to `PlannerErrorKind::InvalidGraph` while hashing a graph ([`planner/src/planner.rs:626-630`](../../../planner/src/planner.rs#L626-L630)). Root native tuning collects the same result and maps it to a runtime error ([`src/training.rs:1183-1195`](../../../src/training.rs#L1183-L1195)). | A work-count language failure prevents a graph digest or tuning decision. No estimate is substituted. |

The remaining direct graph, tensor, shape, axis, and scalar-builder callers are
indexed here.  These are propagation sites, not new `LanguageError`
constructors: each uses `?` or `map_err` after a language API returns the
error.

| Caller | Language operations and conversion | Semantic consequence |
| --- | --- | --- |
| `recipe-planner::plan_program_candidates` | `StaticCalculationProgram::validate`, `CalculationGraph::validate`, and `topological_order` are mapped to `PlannerErrorKind::InvalidGraph` ([`planner/src/planner.rs:220-254`](../../../planner/src/planner.rs#L220-L254)). | Planning stops before lowering, legal placement choices, or graph identity are produced. |
| `recipe-prepare::NativeArtifactProvider` | `CalculationGraph::validate` is mapped to `NativePrepareError::InvalidCandidate` ([`prepare/src/production.rs:146-156`](../../../prepare/src/production.rs#L146-L156)). | Native artifact resolution does not admit a graph with invalid language semantics. |
| `recipe-ops` graph finishers | The composition materializer uses `CalculationGraph::validate` and `language_error` ([`ops/src/materialize.rs:835-846`](../../../ops/src/materialize.rs#L835-L846)); binary metrics use `metric_error` ([`ops/src/binary_metrics.rs:678-687`](../../../ops/src/binary_metrics.rs#L678-L687)); Bayes, KNN, K-means, and tree finishers use `graph_error` ([`ops/src/bayes.rs:661-672`](../../../ops/src/bayes.rs#L661-L672), [`ops/src/knn_outputs.rs:885-898`](../../../ops/src/knn_outputs.rs#L885-L898), [`ops/src/kmeans.rs:563-574`](../../../ops/src/kmeans.rs#L563-L574), [`ops/src/tree.rs:650-664`](../../../ops/src/tree.rs#L650-L664)). | Each materializer returns its operation error and no calculation graph. The rendered language detail is retained, while the operation-specific kind and optional operation ID identify the failed materialization boundary. |
| `recipe-ops` tensor and scalar helpers | `Shape::new`, `AxisSet::new`, `Tensor::contiguous`, `Tensor::validate`, and `ScalarProgramBuilder` calls are mapped to graph-materialization errors in `materialize.rs`, `materialize/{attention_sequence_embedding,convolution_pooling,graph_cluster_rl,indexing_sort_encoding,loss_metrics,training,tree_boosting}.rs`, `binary_metrics.rs`, `bayes.rs`, `knn_outputs.rs`, `kmeans.rs`, and `tree.rs`. The shared mapping is [`ops/src/materialize.rs:4584-4593`](../../../ops/src/materialize.rs#L4584-L4593); metric-specific shape and axis mappings are [`ops/src/binary_metrics.rs:899-914`](../../../ops/src/binary_metrics.rs#L899-L914). | An invalid shape, axis set, tensor layout, or scalar program aborts operation materialization before a primitive node is emitted. No default or alternate shape is selected. |
| `recipe-training` compilation | Training graph construction uses `Shape::new`, `AxisSet::new`, `Tensor::contiguous`, and `ScalarProgramBuilder` with `?` throughout `compile.rs`; representative reducer and shape boundaries are [`training/src/compile.rs:10822-10846`](../../../training/src/compile.rs#L10822-L10846) and [`training/src/compile.rs:11148-11158`](../../../training/src/compile.rs#L11148-L11158). Final graph validation and OGDL round trips are [`training/src/compile.rs:11103-11112`](../../../training/src/compile.rs#L11103-L11112). | `TrainingCompileErrorKind::Language` retains the rendered language error at the outer boundary. No `CompiledTraining` or loop program is returned. |
| `recipe-training` inference compilation and GGUF path | Inference uses the same constructors with `?` in `inference.rs`, including `Tensor::contiguous` and `AxisSet::new` ([`training/src/inference.rs:1864-1879`](../../../training/src/inference.rs#L1864-L1879), [`training/src/inference.rs:1984-2005`](../../../training/src/inference.rs#L1984-L2005), [`training/src/inference.rs:4717-4760`](../../../training/src/inference.rs#L4717-L4760)). The GGUF llama scalar clamp uses the builder directly ([`training/src/gguf_llama.rs:1092-1100`](../../../training/src/gguf_llama.rs#L1092-L1100)). Dense and KNN graph validation, canonical OGDL encoding, and decoding are [`training/src/inference.rs:1752-1769`](../../../training/src/inference.rs#L1752-L1769) and [`training/src/inference.rs:4633-4643`](../../../training/src/inference.rs#L4633-L4643). | `InferenceCompileErrorKind::Language` retains rendered text. A failed shape, axis, scalar, or graph check prevents `CompiledInference` or `CompiledKnnInference`. |
| `recipe-training` inference execution | Canonical boundary checks map `TensorLayout::contiguous` and `Shape::bytes` to `InferenceExecutionError::InvalidInferenceBoundary` with the boundary name and value ID ([`training/src/execute.rs:1908-1944`](../../../training/src/execute.rs#L1908-L1944)). Graph validation is mapped to the same execution boundary in the ordinary and KNN paths ([`training/src/execute.rs:1444-1460`](../../../training/src/execute.rs#L1444-L1460), [`training/src/execute.rs:1648-1663`](../../../training/src/execute.rs#L1648-L1663)). | Runtime admission fails before device input packing or execution. The language error is rendered into the boundary detail and is not recoverable with a different layout or storage size. |

## Variant reference

The sections below index every `LanguageErrorKind` construction in the
workspace.  A helper is listed once with all of its call sites, rather than
repeating the same constructor for each caller.  “Context” describes the
optional `value` and `kernel` fields visible in `Display`.

### `EmptyShape`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `Shape::new` ([`shape.rs:19-25`](../../src/shape.rs#L19-L25)) | The extent vector is empty: `scalar payloads use shape [1]; rank-zero payload shapes are not implicit`. | No context. Shape construction returns no shape, so tensors and primitives cannot use an implicit rank-zero payload. |
| `Shape::broadcast_result` ([`shape.rs:62-69`](../../src/shape.rs#L62-L69)) | The input slice has no shaped input: `broadcast requires at least one shaped input`. | No context at the shape layer. `validate_elementwise` adds the current kernel with `for_kernel` when this path is reached from a primitive ([`primitive.rs:402-408`](../../src/primitive.rs#L402-L408)). |
| OGDL tensor decoding ([`ogdl.rs:535-556`](../../src/ogdl.rs#L535-L556)) | The serialized tensor's extent list is passed to `Shape::new`, so an empty list emits the same `scalar payloads use shape [1]; rank-zero payload shapes are not implicit`. | No context. `?` converts it to `OgdlCodecError::InvalidGraph`; no tensor is decoded. |

The error is distinct from `ShapeOverflow`: the rank or input collection is
missing, not too large.  A no-element shape is valid when it has at least one
extent equal to zero; this variant does not report that case.

### `InvalidAxis`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `Shape::gather_result` ([`shape.rs:110-121`](../../src/shape.rs#L110-L121)) | `axis >= self.rank()`: `gather axis {axis} is outside rank {rank}`. | No context directly. Gather and scatter validators attach the kernel ID while mapping this result ([`primitive.rs:594-603`](../../src/primitive.rs#L594-L603), [`primitive.rs:606-622`](../../src/primitive.rs#L606-L622)). |
| `AxisSet::new` ([`shape.rs:131-138`](../../src/shape.rs#L131-L138)) | The set is empty: `axis set must not be empty`. | No context. Reduction and other callers cannot represent an operation with no declared axis set. |
| OGDL reduction decoding ([`ogdl.rs:660-693`](../../src/ogdl.rs#L660-L693)) | The serialized `axes` list is passed to `AxisSet::new`, so an empty list returns `axis set must not be empty`. | No context. `?` converts it to `OgdlCodecError::InvalidGraph`; the reduction primitive is not decoded. |
| `AxisSet::validate_rank` ([`shape.rs:156-164`](../../src/shape.rs#L156-L164)) | The first sorted axis is outside the supplied rank: `axis {axis} is outside rank {rank}`. | Reduction maps it with the kernel ID ([`primitive.rs:430-460`](../../src/primitive.rs#L430-L460)); direct shape callers receive it unchanged. |
| `validate_scan` ([`primitive.rs:496-506`](../../src/primitive.rs#L496-L506)) | `spec.axis >= input.rank()`: `scan axis {axis} is outside input rank`. | Always carries the scan kernel ID. No scan is validated or lowered. |
| `validate_contraction` ([`primitive.rs:544-555`](../../src/primitive.rs#L544-L555)) | A batch or contract pair names an axis outside either operand rank: `{class} axis pair ({left}, {right}) exceeds ranks ({left_rank}, {right_rank})`. | Always carries the contraction kernel ID. |
| `validate_sort` ([`primitive.rs:656-670`](../../src/primitive.rs#L656-L670)) | `spec.axis >= input.rank()`: `sort axis {axis} is outside input rank`. | Always carries the sort kernel ID. |

An invalid axis prevents shape derivation or primitive validation.  No axis is
clamped or discarded.

### `DuplicateAxis`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `AxisSet::new` ([`shape.rs:139-147`](../../src/shape.rs#L139-L147)) | After sorting, the set contains a duplicate: `axis set contains a duplicate axis`. | No context. The returned set is never constructed. |
| OGDL reduction decoding ([`ogdl.rs:668-693`](../../src/ogdl.rs#L668-L693)) | Duplicate serialized axes reach `AxisSet::new` and return `axis set contains a duplicate axis`. | No context. `?` converts it to `OgdlCodecError::InvalidGraph`. |
| `validate_contraction` ([`primitive.rs:548-562`](../../src/primitive.rs#L548-L562)) | A batch or contract pair reuses a left or right operand axis: `{class} axis pair reuses an operand axis`. | Carries the contraction kernel ID. The axis partition is rejected instead of producing an ambiguous output shape. |

### `ShapeOverflow`

`Shape::new` checks each extent multiplication with `u64::checked_mul`
([`shape.rs:26-35`](../../src/shape.rs#L26-L35)).  On overflow it returns
`ShapeOverflow` with `shape element count overflowed u64`, without context.  The
same constructor is used while decoding serialized tensor extents
([`ogdl.rs:550-556`](../../src/ogdl.rs#L550-L556)); there the error is wrapped as
`OgdlCodecError::InvalidGraph`.  The element count is therefore never wrapped
and no byte count or layout is derived from it.

### `ByteSizeOverflow`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `Shape::bytes` ([`shape.rs:50-60`](../../src/shape.rs#L50-L60)) | `elements * dtype.byte_width()` does not fit `u64`: `typed tensor byte size overflowed u64`. | No context. `Tensor::contiguous` propagates this before constructing storage metadata. |
| `TensorLayout::byte_offset` ([`tensor.rs:134-144`](../../src/tensor.rs#L134-L144)) | `offset_elements * dtype.byte_width()` does not fit: `layout byte offset overflowed`. | No context. There is no representable byte address. |
| `Tensor::validate` ([`tensor.rs:180-194`](../../src/tensor.rs#L180-L194)) | The validated layout span multiplied by the dtype width overflows: `tensor layout span overflowed bytes`. | Carries `value = self.id`. Validation cannot compare the span with declared storage. |

### `InvalidLayout`

`TensorLayout` uses this one category for every layout contract failure.  The
complete constructor index is:

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `TensorLayout::contiguous` row-major ([`tensor.rs:20-33`](../../src/tensor.rs#L20-L33)) | A row-major stride multiplication overflows: `row-major stride overflowed`. | No context. A contiguous layout is not returned. |
| `TensorLayout::contiguous` column-major ([`tensor.rs:35-45`](../../src/tensor.rs#L35-L45)) | A column-major stride multiplication overflows: `column-major stride overflowed`. | No context. |
| `TensorLayout::validate` ([`tensor.rs:53-63`](../../src/tensor.rs#L53-L63)) | Stride count differs from rank: `layout has {strides} strides for rank {rank}`. | No context at this layer. `Tensor::validate` adds the tensor value to this error through `map_err`. |
| `TensorLayout::validate` ([`tensor.rs:64-75`](../../src/tensor.rs#L64-L75)) | A non-singleton axis has zero stride: `a non-singleton payload axis cannot have zero stride`. | `Tensor::validate` adds `value = self.id`. |
| `TensorLayout::validate` ([`tensor.rs:76-101`](../../src/tensor.rs#L76-L101)) | Sorted non-singleton axes overlap, or the occupied-span arithmetic overflows: either `tensor layout maps multiple logical elements to the same storage element` or `layout non-overlap validation overflowed`. | `Tensor::validate` adds the value only through its explicit `layout.validate(...).map_err(...)` path. |
| `TensorLayout::span_elements` ([`tensor.rs:108-132`](../../src/tensor.rs#L108-L132)) | A span multiplication, addition, or final `+1` overflows: `layout span multiplication overflowed`, `layout span addition overflowed`, or `layout span overflowed`. | No context is attached by `span_elements`; the direct `?` in `Tensor::validate` and `TensorLayout::validate` preserves that absence. |
| `Tensor::validate` ([`tensor.rs:184-204`](../../src/tensor.rs#L184-L204)) | The layout span is larger than declared storage: `layout requires {span} bytes but storage declares {storage_bytes}`. | Carries `value = self.id`. The tensor is rejected because its metadata would address bytes outside the backing object. |

`TensorLayout::validate` skips zero-element payloads for zero-stride and
non-overlap checks, and `span_elements` returns zero for them.  That is valid
empty metadata, not an error.  Any overflow or out-of-bounds nonempty layout
still fails closed.

### `DuplicateTensor`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `CalculationGraph::assemble` ([`graph.rs:27-51`](../../src/graph.rs#L27-L51)) | Two fragments use one `ValueId` with different dtype, shape, layout, or storage bytes: `fragments declare different storage contracts for the same tensor`. | Carries the conflicting tensor value. External input/output flags may differ and are intentionally not part of the storage contract. Assembly stops rather than choosing one fragment's metadata. |
| `tensor_index` ([`graph.rs:189-200`](../../src/graph.rs#L189-L200)) | The graph's tensor vector contains the same ID twice: `tensor {id} appears more than once`. | Carries the duplicate value. Graph validation cannot build an authoritative tensor index. |
| `unique_boundary` for either boundary set ([`graph.rs:268-279`](../../src/graph.rs#L268-L279)) | An external input or external output iterator repeats one value: `{name} boundary repeats tensor {value}`. | Carries the repeated value. The two boundary sets are checked independently; listing a value once in each is allowed. |

`CalculationGraph::validate`, `topological_order`, and `dependencies` all
propagate the index failure.  OGDL decoding and encoding also stop before
accepting or emitting a graph.

### `DuplicateKernel`

`CalculationGraph::validate` inserts every node kernel ID into a `BTreeSet`
([`graph.rs:78-94`](../../src/graph.rs#L78-L94)).  A repeated ID returns
`DuplicateKernel` with `kernel {id} appears more than once` and
`kernel = node.kernel.id`.  Kernel IDs are the graph's identity for
dependency and scheduling queries, so validation does not select one duplicate
or merge their outputs.

### `UnknownTensor`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `CalculationGraph::assemble` ([`graph.rs:54-61`](../../src/graph.rs#L54-L61)) | A requested external input or output is absent from all fragment tensor declarations: `assembled graph boundary references an absent tensor`. | Carries the missing value. Assembly returns no graph. |
| `PrimitiveKernel::validate` input lookup ([`primitive.rs:207-221`](../../src/primitive.rs#L207-L221)) | An input ID is absent from the supplied tensor map: `input tensor {id} does not exist`. | Carries the kernel and value. Input lookup is collected in input order and returns the first failure. |
| `PrimitiveKernel::validate` output lookup ([`primitive.rs:222-236`](../../src/primitive.rs#L222-L236)) | An output ID is absent: `output tensor {id} does not exist`. | Carries the kernel and value. Outputs are checked only after every input lookup succeeds. |

No missing tensor is synthesized.  Primitive validation does not reach alias or
family-specific checks after an unresolved input or output.

### `DuplicateProducer`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `CalculationGraph::validate` producer index ([`graph.rs:84-107`](../../src/graph.rs#L84-L107)) | Two kernels list the same output: `tensor {output} is produced by kernels {previous} and {current}`. | Carries the current kernel and output value. A value cannot have an ambiguous calculation producer. |
| `CalculationGraph::validate` external-input check ([`graph.rs:110-121`](../../src/graph.rs#L110-L121)) | An external input also appears in the producer map: `external input tensor {id} is also produced by kernel {producer}`. | Carries the input value. External admission and calculation production are mutually exclusive. |

### `MissingProducer`

After indexing all kernel outputs, `CalculationGraph::validate` checks every
tensor ([`graph.rs:110-134`](../../src/graph.rs#L110-L134)).  A non-external
tensor with no producer returns `MissingProducer` with
`non-external tensor {id} has no calculation producer` and `value = id`.
The graph cannot be topologically ordered or executed because there is no
calculation that defines the value.

### `Cycle`

`topological_order_from` creates dependency edges from producer IDs to consumer
IDs ([`graph.rs:203-238`](../../src/graph.rs#L203-L238)).  It reports two forms:

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| Self-edge check ([`graph.rs:214-227`](../../src/graph.rs#L214-L227)) | A kernel consumes a value it produces itself: `kernel consumes its own output value`. | Carries that kernel ID. |
| Kahn-order completion check ([`graph.rs:243-264`](../../src/graph.rs#L243-L264)) | The ready set is exhausted before every node is ordered: `calculation graph contains a cycle`. | No kernel context is attached because the remaining cycle can contain several kernels. |

`CalculationGraph::validate` calls this helper, and public ordering and
dependency methods validate before deriving results.  There is no partial
topological order on failure.

### `ArityMismatch`

`require_arity` is the single constructor for ordinary primitive arity checks
([`primitive.rs:762-772`](../../src/primitive.rs#L762-L772)).  It renders
`{name} arity is {actual}, expected {expected}` and attaches the kernel ID.
Every invocation is listed here:

| Validator | Required arities |
| --- | --- |
| `validate_elementwise` ([`primitive.rs:363-390`](../../src/primitive.rs#L363-L390)) | Input count equals scalar-program input count; output count equals scalar-program output count. A separate `ArityMismatch` rejects zero inputs with `constant elementwise maps use Random or an explicit filled input`. |
| `validate_reduce` ([`primitive.rs:430-437`](../../src/primitive.rs#L430-L437)) | One input; one output for `Value` or `Index`, two for `ValueAndIndex`. |
| `validate_scan` ([`primitive.rs:496-499`](../../src/primitive.rs#L496-L499)) | One input and one output. |
| `validate_contraction` ([`primitive.rs:522-530`](../../src/primitive.rs#L522-L530)) | Two inputs and one output. |
| `validate_gather` ([`primitive.rs:594-596`](../../src/primitive.rs#L594-L596)) | Two inputs and one output. |
| `validate_scatter` ([`primitive.rs:606-613`](../../src/primitive.rs#L606-L613)) | Three inputs and one output. |
| `validate_histogram` ([`primitive.rs:628-636`](../../src/primitive.rs#L628-L636)) | One input for unweighted, two for weighted, and one output. |
| `validate_sort` ([`primitive.rs:656-663`](../../src/primitive.rs#L656-L663)) | One input and one output, or two outputs when indices are emitted. |
| `validate_index_map` ([`primitive.rs:687-695`](../../src/primitive.rs#L687-L695)) | Zero inputs and one output. |
| `validate_random` ([`primitive.rs:706-713`](../../src/primitive.rs#L706-L713)) | Zero inputs and one output. |

The elementwise special case is constructed at
[`primitive.rs:384-390`](../../src/primitive.rs#L384-L390), outside the helper,
but has the same kernel context.  An arity failure prevents the corresponding
primitive lowering and all later dtype or shape checks for that call.

### `DTypeMismatch`

`dtype_error` constructs every dtype failure and always attaches the kernel ID
([`primitive.rs:774-803`](../../src/primitive.rs#L774-L803)).  `require_dtype`
uses it for exact equality checks.  The complete direct validation contexts are:

| Validator | Dtype checks |
| --- | --- |
| `validate_elementwise` ([`primitive.rs:391-419`](../../src/primitive.rs#L391-L419)) | Each tensor input must match its scalar input; each tensor output must match the scalar output's `dtype_of` result. |
| `validate_reduce` ([`primitive.rs:430-475`](../../src/primitive.rs#L430-L475)) | `Any` and `All` require `I32`; value results match the input dtype; index results are `I32`; `ValueAndIndex` applies both output checks. |
| `validate_scan` ([`primitive.rs:496-520`](../../src/primitive.rs#L496-L520)) | `Any` and `All` require `I32`; an exclusive identity must match the input; the output matches the input. |
| `validate_contraction` ([`primitive.rs:522-539`](../../src/primitive.rs#L522-L539)) | Both operands match each other and the output matches the operands. |
| `validate_gather` ([`primitive.rs:594-603`](../../src/primitive.rs#L594-L603)) | Indices are `I32`; output matches the source tensor. |
| `validate_scatter` ([`primitive.rs:606-622`](../../src/primitive.rs#L606-L622)) | Indices are `I32`; updates and output match the destination/source dtype. |
| `validate_histogram` ([`primitive.rs:628-653`](../../src/primitive.rs#L628-L653)) | Weighted inputs and output are `F32`; unweighted output is `I32`. |
| `validate_sort` ([`primitive.rs:656-684`](../../src/primitive.rs#L656-L684)) | Values match the input; optional emitted indices are `I32`. |
| `validate_index_map` ([`primitive.rs:687-703`](../../src/primitive.rs#L687-L703)) | Output is `I32`. |
| `validate_random` ([`primitive.rs:706-749`](../../src/primitive.rs#L706-L749)) | Output matches the selected distribution: `F32` for uniform/normal and `I32` for Bernoulli/uniform integer. |

The detail text identifies the named role and both actual and expected dtypes.
No implicit cast is inserted.

### `ShapeMismatch`

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `Shape::broadcast_result` ([`shape.rs:70-90`](../../src/shape.rs#L70-L90)) | Two non-singleton broadcast extents differ: `broadcast extent {extent} conflicts with {existing} at result axis {axis}`. | No context directly. Elementwise validation adds the kernel ID before returning. |
| `validate_contraction` ([`primitive.rs:563-571`](../../src/primitive.rs#L563-L571)) | A paired batch or contract extent differs: `{class} axes ({left}, {right}) have extents {left_extent} and {right_extent}`. | Carries the contraction kernel ID. |
| `require_shape` ([`primitive.rs:785-798`](../../src/primitive.rs#L785-L798)) | An actual result shape differs from the expected shape: `{name} has shape {actual:?}, expected {expected:?}`. | Carries the kernel ID. |

`require_shape` is called for elementwise outputs, reduction values and
indices, scan output, contraction output, gather output, scatter output and
updates, histogram output, and sort values and optional indices.  The call
sites are [`primitive.rs:409-425`](../../src/primitive.rs#L409-L425),
[`primitive.rs:461-475`](../../src/primitive.rs#L461-L475),
[`primitive.rs:518-520`](../../src/primitive.rs#L518-L520),
[`primitive.rs:587-592`](../../src/primitive.rs#L587-L592),
[`primitive.rs:599-604`](../../src/primitive.rs#L599-L604),
[`primitive.rs:617-625`](../../src/primitive.rs#L617-L625),
[`primitive.rs:644-654`](../../src/primitive.rs#L644-L654), and
[`primitive.rs:678-684`](../../src/primitive.rs#L678-L684).  Shape equality is
exact after the primitive's documented derivation; no reshape or broadcast is
silently applied to a declared output.

### `InvalidScalarProgram`

This category covers builder invariants and the core scalar validator.  Its
construction sites are:

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `ScalarProgramBuilder::new` ([`scalar_builder.rs:41-59`](../../src/scalar_builder.rs#L41-L59)) | The global builder owner counter cannot be incremented: `scalar builder identity space exhausted`. | No context. No builder exists. |
| `ScalarProgramBuilder::apply` ([`scalar_builder.rs:86-105`](../../src/scalar_builder.rs#L86-L105)) | An operand belongs to another builder, or the opcode has no result dtype for the supplied operand types. Details are `scalar value {id} belongs to another program builder` or `{opcode:?} does not accept operands {operand_types:?}`. | No context. The instruction is not appended. |
| `ScalarProgramBuilder::finish` ([`scalar_builder.rs:139-158`](../../src/scalar_builder.rs#L139-L158)) | An output belongs to another builder, or `ScalarProgram::validate` returns core validation errors. Details are `scalar output {id} belongs to another program builder` or the core validator's rendered collection. | No context. The returned program is not accepted. Core validation may report duplicate scalar IDs, use-before-definition, scalar arity/type errors, missing outputs, or unknown outputs. |
| `ScalarProgramBuilder::next_expression` ([`scalar_builder.rs:161-174`](../../src/scalar_builder.rs#L161-L174)) | The per-builder scalar value counter overflows: `scalar value identity space exhausted`. | No context. The input, constant, or instruction result is not allocated. |
| `validate_elementwise` ([`primitive.rs:363-371`](../../src/primitive.rs#L363-L371)) | The embedded scalar program fails core validation. The core `ValidationErrors` text is copied into `detail`. | Carries the primitive kernel ID. A scalar program that passed its builder boundary is still revalidated at the primitive boundary. |
| `attention_extent` ([`training/src/forward.rs:426-433`](../../../training/src/forward.rs#L426-L433)) | An attention sequence, head count, or head dimension does not fit `i32`: `attention {name} {value} cannot be represented by int32: {error}`. | No context. The scalar index program is not built. |
| `sum_program` ([`training/src/forward.rs:435-450`](../../../training/src/forward.rs#L435-L450)) | A requested scalar sum has zero inputs: `scalar sum requires at least one input`. | No context. The training forward formula is absent. |

The math crate and most training scalar formulas reach the builder constructors
through `?`, so owner, opcode, identity, and core-validation failures retain
their original kind until their outer compile boundary.  `LanguageError` does
not retain a typed `ValidationErrors` child; `finish` stores its display text in
`detail`.

### `InvalidPrimitive`

This category covers primitive policy and representability rules that are not
ordinary arity, dtype, axis, or shape mismatches.  Every constructor in
`primitive.rs` attaches the current kernel ID, except the graph query below.

| Construction | Condition and exact detail | Context and consequence |
| --- | --- | --- |
| `validate_alias_matrix` ([`primitive.rs:328-359`](../../src/primitive.rs#L328-L359)) | Alias input/output index is outside the declared counts: `alias pair ({input}, {output}) is outside {inputs} inputs and {outputs} outputs`; a pair repeats: `alias pair ({input}, {output}) appears more than once`; or the matrix is incomplete: `every input/output pair requires an explicit alias rule`. | Carries the kernel ID. Alias permissions are explicit and total, so no unspecified alias behavior is inferred. |
| `CalculationGraph::dependencies` ([`graph.rs:155-177`](../../src/graph.rs#L155-L177)) | The requested kernel is absent: `kernel {kernel} is absent`. | Carries the requested kernel ID. This is a query failure, not a malformed existing primitive. |
| `validate_reduce` ([`primitive.rs:441-456`](../../src/primitive.rs#L441-L456)) | An index result is requested for an operator other than `Minimum` or `Maximum`: `index reductions require Minimum or Maximum`. | Carries the reduction kernel ID. |
| `validate_reduce` ([`primitive.rs:477-492`](../../src/primitive.rs#L477-L492)) | A `Minimum` or `Maximum` reduction includes an empty input axis: `Minimum and Maximum have no implicit empty-domain identity`. | Carries the reduction kernel ID. Empty-domain reductions with operators that have an identity remain valid. |
| `validate_contraction` ([`primitive.rs:522-536`](../../src/primitive.rs#L522-L536)) | No contracted axis pair is supplied: `contraction requires at least one contracted axis pair`. | Carries the contraction kernel ID. |
| `validate_histogram` ([`primitive.rs:628-643`](../../src/primitive.rs#L628-L643)) | Bin count is zero or exceeds `i32::MAX`: `histogram bin count must be in 1..=i32::MAX`. | Carries the histogram kernel ID. |
| `validate_sort` ([`primitive.rs:664-677`](../../src/primitive.rs#L664-L677)) | The selected axis extent exceeds the representable `i32` index range: `sort axis cannot be represented by int32 result indices`. | Carries the sort kernel ID. |
| `validate_index_map` ([`primitive.rs:692-703`](../../src/primitive.rs#L692-L703)) | A present modulus is nonpositive: `index-map modulus must be strictly positive when present`. | Carries the index-map kernel ID. |
| `validate_random` ([`primitive.rs:706-749`](../../src/primitive.rs#L706-L749)) | Philox rounds differ from ten: `Recipe random maps require exactly Philox4x32-10`; Bernoulli probability is nonfinite or outside `[0, 1]`: `Bernoulli probability must be a finite f32 in [0, 1]`; or integer range has `low >= high_exclusive`: `uniform int32 range must have low < high_exclusive`. | Carries the random kernel ID. The requested distribution is not silently changed. |
| `validate_tree` ([`primitive.rs:751-759`](../../src/primitive.rs#L751-L759)) | Reduction/scan tree lanes are zero, above 1024, or not a power of two: `fixed tree lane count must be a power of two in 1..=1024`. | Carries the primitive kernel ID. The operation-order contract remains explicit. |

### `WorkOverflow`

`PrimitiveKernel::work` validates the kernel first and then computes a checked
`u64` work count ([`primitive.rs:252-326`](../../src/primitive.rs#L252-L326)).
The helper at [`primitive.rs:805-811`](../../src/primitive.rs#L805-L811)
constructs `WorkOverflow` with `primitive work count overflowed u64` and
attaches the kernel ID.  It is used at every checked work boundary:

* elementwise instruction-count accumulation and element-count scaling;
* reduction combine count multiplied by the value/index multiplier;
* scan input element count multiplied by two;
* contraction products of contracted extents, output elements, and the final
  factor of two;
* sort comparison count and slice count, including the inner logarithm-derived
  multiplication; and
* random output elements multiplied by Philox rounds and the four-lane factor.

Gather, scatter, histogram, and index-map work counts are bounded directly by
an already validated tensor element count and do not introduce another checked
overflow expression.  `PrimitiveKernel::work` returns no `FlopCount` on any
overflow.  The planner therefore refuses to hash the graph with an invented
cost, and root training tuning refuses to select a tuning value from an
unrepresentable maximum work count.

## Semantic consequences by layer

The same error value has a consistent meaning at each layer:

1. Shape and tensor constructors reject metadata before an object crosses into
   a graph.  Existing objects remain unchanged because methods borrow or
   consume their inputs only after checks that can fail.
2. Graph validation establishes unique tensor and kernel indexes, exactly one
   producer per calculated value, external boundary disjointness, and an
   acyclic dependency relation.  Ordering and dependency queries never return
   a partial result.
3. Scalar builders enforce owner isolation and typed SSA construction.  A
   failed builder operation does not append an instruction, and `finish` does
   not return a partially valid `ScalarProgram`.
4. Primitive validation checks every declared family contract before lowering.
   Alias matrices, dtypes, shapes, axis pairs, bounds, reduction identities,
   random distribution parameters, and tree lane counts are not defaulted or
   coerced.
5. Work calculation is a checked semantic cost operation.  Overflow is an
   error, not saturation, truncation, or a fallback estimate.
6. OGDL and higher-level compile paths preserve the rejection.  A typed wrapper
   may add its own category or rendered prefix, but it never returns the
   invalid language object as successful output.

This is the complete role of `LanguageError`: classify the first invalid
language boundary, retain enough detail and optional identity context to locate
it, render it deterministically, and stop the dependent operation.
