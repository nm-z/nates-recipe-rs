# Non-calculation registry entries

`NonCalculationRecipe` is the explicit compatibility classification for public
surface entries whose meaning is host metadata, orchestration, or lifecycle. It
is not an execution kind and it is not a license to evaluate payload values on
the CPU. The enum is defined in
[`ops/src/non_calculation.rs`](../../src/non_calculation.rs:1-65), re-exported
from [`ops/src/lib.rs`](../../src/lib.rs:60), and exposed through the dependency
clean operations facade in [`src/facade.rs`](../../../src/facade.rs:51-61).

The distinction matters because Recipe has one executable model: an immutable
graph of GPU calculations and explicit byte transfers. Host work may prepare
that graph and its inputs, but it may not become a hidden calculation task.
`NonCalculationRecipe` makes that boundary visible in the operation registry
while leaving the actual host implementation in the crate that owns it.

## The boundary in one view

The normative contract defines a calculation as payload arithmetic on GPU
`f32` or `int32` values and a transfer as explicit byte movement
(`system-contract.md:42-49`). CPU orchestration, parsing, framing, discovery,
and metadata construction are allowed, but production transformations of
calculation payload values remain GPU calculations (`system-contract.md:262-279`).
The run therefore has this shape:

```text
public declaration
    -> bounded host preparation and metadata
    -> typed CalculationGraph
    -> immutable TaskKind schedule
    -> init external-to-device transfers
    -> loop calculations, internal transfers, and four-byte metric readbacks
    -> exit device-to-external transfers
    -> one arena release per device and resource destruction
```

The enum entries are present in the first two stages only. They never add a
fourth `TaskKind`, never create a `CalculationNode`, and never add a transfer
without a prepared value. `TaskKind` is intentionally exhaustive in
[`core/src/schedule.rs`](../../../core/src/schedule.rs:436-450):

| Graph kind | Legal phase | Meaning |
| --- | --- | --- |
| `Calculation(CalculationTask)` | `RunPhase::Loop` | GPU payload arithmetic. The task names a GPU device, kernel template, artifact, f32/int32 values, optional int32 fault flag, FLOP work, and submission slots. |
| `Transfer(TransferTask)` | `Init`, `Loop`, or `Exit` according to its endpoints | Explicit movement between `External` and device values, or between device values. The route and lane claims are fixed by planning. |
| `Metric(MetricTask)` | `RunPhase::Loop` | A specialized asynchronous four-byte readback to a preallocated metric slot. `MetricPurpose::FaultReadback` is the checked device fault path, not a third model-work category. |

`RunPhase` itself is only `Init`, `Loop`, or `Exit` (`core/src/schedule.rs:14-19`).
The plan validator enforces that calculations and metric emissions are loop
tasks, external admission is init-only, and external egress is exit-only
(`core/src/plan.rs:472-480`, `core/src/plan.rs:634-772`). This is the concrete
calculation/transfer boundary that the compatibility classification must not
cross.

## Which entries are classified

The preserved compatibility inventory records these symbols and their original
source-qualified identity (`operation-surface.txt:11-14,17,60,346,419,425-426`):

| `NonCalculationRecipe` | Inventory symbol and source | Registry family | Definition returned by `definition()` |
| --- | --- | --- | --- |
| `FacadeDeclaration` | `Data`, `Model`, `Train`, `Infer`, all from `root public facade` | `OperationFamily::Facade` | Typed declaration that records data, model, training, or inference configuration without payload arithmetic. |
| `TextTokenization` | `encode` from `pantry/src/bpe.rs:59` | `OperationFamily::Encoding` | Deterministic raw text to int32 token-ID metadata transformation before payload admission. |
| `ModelContainerParsing` | `parse_safetensors` from `recipe-infer/src/safetensors.rs:146`, plus any row whose source contains `safetensors` | `OperationFamily::Parsing` | Validated container parsing that constructs tensor metadata and opaque byte ranges before admission. |
| `ChatTemplateRendering` | `render_chat` from `recipe-infer/src/chat.rs:43`, and `render_template` from `recipe-infer/src/chat.rs:21` | `OperationFamily::Encoding` | Deterministic raw-text template rendering before tokenization and payload admission. |
| `RunShutdown` | `gpu_shutdown` from `gpu-core/src/kernels.rs:1772` | `OperationFamily::Lifecycle` | Exit lifecycle action represented by the typed one-free-per-device executor transition. |
| `EliminatedVendorWorkspaceBinding` | `gpu_blas_workspace` from `gpu-core/src/kernels.rs:1738` | `OperationFamily::Workspace` | Prohibited vendor-library workspace setter eliminated; owned primitive scratch is fixed during preparation. |

The source strings above are historical source identities retained by the
compatibility manifest. They are not claims that those legacy directories are
part of this workspace. The dependency-clean implementations are the current
`src`, `text`, `ingest`, `executor`, and `native-executor` paths cited below.

The classifier is intentionally small and source-visible:

```rust
match symbol {
    "Data" | "Model" | "Train" | "Infer" => FacadeDeclaration,
    "encode" => TextTokenization,
    "parse_safetensors" => ModelContainerParsing,
    "render_chat" | "render_template" => ChatTemplateRendering,
    "gpu_shutdown" => RunShutdown,
    "gpu_blas_workspace" => EliminatedVendorWorkspaceBinding,
    _ if source.contains("safetensors") => ModelContainerParsing,
    _ => return None,
}
```

This is `NonCalculationRecipe::for_entry` (`ops/src/non_calculation.rs:16-29`).
The exact symbol arms run before the source substring arm. A different symbol
whose recorded source contains `safetensors` therefore receives the same
parsing classification, while an otherwise unknown entry returns `None` and
continues through the registry's other classifiers.

## Registry construction and lowering behavior

### Inventory to descriptor

`ops/build.rs` reads the root `operation-surface.txt`, rejects malformed rows,
preserves row order, and emits `RAW_OPERATION_SURFACE` plus
`RAW_OPERATION_COUNT` into Cargo's `OUT_DIR` (`ops/build.rs:3-53`).
`ops/src/registry.rs:198-208` includes that generated fragment. The immutable
`OperationRegistry` iterates the generated prefix and then the two explicit
Recipe-owned max-pool extensions (`ops/src/registry.rs:210-280`). A descriptor's
`OperationId` retains ordinal, source line, duplicate occurrence, and total
occurrence count (`ops/src/registry.rs:8-35`). Thus a compatibility entry keeps
its exact legacy identity even when its implementation is now owned elsewhere.

`describe` derives one `OperationDescriptor` from a raw row
(`ops/src/registry.rs:326-344`). The descriptor carries:

- the source-qualified identity and symbol;
- `OperationFamily`;
- `CanonicalDTypeContract`;
- `LoweringAvailability`;
- the human definition string;
- alias and determinism contracts; and
- any explicitly recognized legacy dtype.

The public facade exposes the same registry without exposing the build script.
`operations::all` preserves normative order, `operations::resolve` requires a
unique symbol, and `operations::resolve_exact` requires an exact symbol/source
pair (`src/facade.rs:63-82`). `OperationRegistry::resolve_unique` returns
`UnknownOperation` when absent and `AmbiguousSymbol` when duplicate source
rows exist; `resolve_exact` has the corresponding exact-pair checks
(`ops/src/registry.rs:283-323`). A caller cannot silently select a nearby
implementation for a duplicate legacy symbol.

### Classification order

`registry::lowering` applies one ordered decision tree
(`ops/src/registry.rs:347-373`):

1. owned scalar recipe;
2. owned direct primitive recipe;
3. checked owned workspace formula;
4. `NonCalculationRecipe::for_entry`;
5. finite structured composition;
6. explicit legacy-dtype exclusion;
7. pending dynamic conversion;
8. pending host behavior for non-`gpu_` symbols; or
9. pending dedicated primitive composition for an otherwise unknown GPU symbol.

Consequently `NonCalculation` is a positive, named result, not an accidental
fallthrough. It also cannot mask a scalar or primitive implementation: those
owned registries are consulted first.

For a non-calculation lowering, the registry derives
`CanonicalDTypeContract::NonNumericHostData` (`ops/src/registry.rs:375-413`),
maps the enum to its family (`ops/src/registry.rs:459-471`), and marks it
`DeterminismContract::HostDeterministic` (`ops/src/registry.rs:643-660`). These
fields describe metadata behavior. They do not authorize a payload dtype or a
host numerical kernel. Alias metadata still follows the general symbol rule:
non-GPU names default to `NoAlias`, while a `gpu_*` compatibility name is
`OperationSpecific` unless an explicit in-place rule applies
(`ops/src/registry.rs:626-640`).

### Wrong-kind failures are deliberate

The operations facade has separate entry points for scalar lowering, primitive
lowering, composition validation/materialization, and workspace evaluation
(`src/facade.rs:84-124`). Every one checks the descriptor's lowering kind.

- `lower_scalar` rejects `NonCalculation` with
  `OperationErrorKind::WrongLoweringKind` and the detail that the operation has
  no scalar elementwise recipe (`ops/src/scalar.rs:346-365`).
- `lower_primitive` does the same for direct primitive lowering
  (`ops/src/primitive.rs:388-415`).
- `validate_composition` requires `LoweringAvailability::Composition`
  (`ops/src/composition.rs:126-140`).
- `evaluate_workspace` requires `LoweringAvailability::Workspace`
  (`ops/src/workspace.rs:270-284`).
- `materialize_composition` rejects anything without a concrete composition
  materializer before it can build a graph (`ops/src/materialize.rs:413-445`).
  `remaining_composition_manifest` explicitly filters `NonCalculation` out
  (`ops/src/materialize.rs:482-505`).

The common `OperationError` retains the operation identity and renders the
kind, detail, and ordinal (`ops/src/error.rs:5-66`). There is no alternate CPU
implementation, retry, or vendor fallback after a wrong-kind error.

Training and inference graph compilers use the owned path directly. Their
`emit_owned_scalar` helpers resolve a descriptor and immediately call
`lower_scalar` (`training/src/compile.rs:10848-10858`,
`training/src/inference.rs:1973-1982`). Structured calls resolve a descriptor
and pass it to `materialize_composition`
(`training/src/compile.rs:10937-10996`, `training/src/inference.rs:2008-2076`).
A non-calculation descriptor therefore stops at a typed wrong-kind error rather
than becoming a graph node.

## `FacadeDeclaration`: declarations are state, not work

The four root symbols are the public declaration boundary. Their current
implementations are in `src/api.rs` and are deliberately inert until a terminal
run method is called.

### `Data`

`Data` stores source paths, target names, column and row exclusions, an optional
train fraction, normalization choice, and one deferred declaration error
(`src/api.rs:364-378`). `set`, `target`, `exclude`, `split`, and `norm` only
validate local values, update the returned immutable builder, and remember a
clone in the facade sequence (`src/api.rs:393-468`). They do not read a file,
parse a container, infer semantics, normalize a value, probe hardware, or
allocate a device buffer. `Data::validate` rejects a deferred error or an empty
source list (`src/api.rs:470-480`).

The actual source boundary is `src/data_prepare.rs`. `prepare_data` applies
finite ingest limits, requires targets and an explicit split, distills the
ordered source set, infers vector semantics, and prepares a rectangular typed
dataset (`src/data_prepare.rs:79-172`). `distill_data` is the shared target-free
inference boundary and returns a `DistilledDataset` without training-only
selection or normalization (`src/data_prepare.rs:97-129`). Every failure is
typed as declaration, ingest, source, semantic, or preparation failure; no
partial dataset is returned (`src/data_prepare.rs:20-77`).

### `Model`

`Model` stores layer declarations, Bayesian dependencies, an objective, gradient
policy, an optional checkpoint path, and a deferred error. It contains no
runtime handles, loaded weights, allocations, or mutable registry entries
(`src/api.rs:1005-1018`). `Model::load` records one path and rejects an empty
path, duplicate checkpoint source, or mixing a checkpoint with an inline layer
definition (`src/api.rs:1033-1058`). Layer and embedding builders validate their
local shape and relationship rules, then remember the updated declaration
(`src/api.rs:1061-1105` and following builders).

The checkpoint bytes and semantic graph are read later by training or inference
compilation. A model declaration therefore names future preparation work; it is
not a model-container parser and it is not payload arithmetic.

### `Train`

`Train` is static policy: epochs, learning rate and decay, optimizer, metrics,
plots, resume paths, save paths, and deferred configuration errors
(`src/api.rs:1845-1888`). Builder calls only store policy. Resume and save
extensions are checked at declaration time, including the semantic `.ogdl`
first argument and optional `.cubin` or `.hsaco` second argument
(`src/api.rs:1991-2079`); filesystem existence and native compilation happen
later.

`Train::run` consumes the preceding data/model pair and routes to
`try_run_with` (`src/training.rs:848-869`). The terminal then either builds the
specialized Bayesian or KNN artifact, or compiles a dense graph, installs the
graceful-stop source, executes native preparation and the run, and writes only
the declared artifacts (`src/training.rs:869-917`). The declaration itself is
not a loop and does not alter the graph's task vocabulary.

### `Infer`

`Infer` stores logging declarations and deferred policy errors
(`src/api.rs:2181-2253`). `resolve_declaration` consumes exactly one preceding
data/model sequence, validates both declarations, and builds an
`InferenceDeclaration`; it performs no native work (`src/api.rs:2219-2231`).
`evaluate` then calls the inference implementation
(`src/api.rs:2234-2240`).

The inference compiler reads a semantic `.ogdl` or supported `.gguf` model,
distills and selects data, and builds a typed inference graph before native
preparation (`src/inference.rs:482-543`). Native execution uses the same
measured-profile and `init -> loop -> exit` path as training. The report is
written only after exit and teardown (`src/inference.rs:432-479` and
`src/inference.rs:602-658`).

### Facade state machine

`src/facade.rs:135-190` owns a thread-local `RecipeSequence` with one optional
`Data` and one optional `Model`. A new data declaration replaces the data slot
and clears the model slot; model builders replace only the model slot. The
training and inference terminals take both slots and remove them. Missing data
or model produces a specific terminal error instead of guessing an earlier
declaration. This small state machine is the runtime role represented by
`FacadeDeclaration`; it is not a calculation graph state.

## `TextTokenization`: host metadata before admission

The inventory preserves the old `encode` identity, while the current bounded
implementation is `recipe-text` (`text/src/lib.rs`). The crate-level contract
states that tokenization converts raw text to checked `int32` identifiers and
chat rendering produces raw text before the single init admission. Neither
scores tokens, transforms payloads, reads files in the loop, nor retains a file
handle (`text/src/lib.rs:4-9`). The facade re-exports it as the dependency-clean
engine module (`src/facade.rs:17-42`).

### Construction and identity

`TextLimits` gives nonzero bounds for model bytes, input bytes, output token
count, vocabulary and merge sizes, template bytes, message count, and rendered
bytes (`text/src/lib.rs:26-97`). `Tokenizer::from_json` bounds and parses one
JSON snapshot, validates vocabulary size, and records a canonical
`TokenizerIdentity` (`text/src/lib.rs:429-450`). `from_file` snapshots a file
through the bounded ingest source reader before calling the same JSON path
(`text/src/lib.rs:452-469`). `from_vocabulary` validates duplicate pieces,
merge or SentencePiece score rules, unknown-token bounds, and int32 identity
limits before constructing the tokenizer (`text/src/lib.rs:471-493`,
`text/src/lib.rs:742-828`).

The identity is the complete canonical tokenizer serialization, not a lossy
hash or process-local handle (`text/src/lib.rs:169-185`). A prepared batch
therefore cannot be decoded with a different tokenizer configuration.

### Encoding contract

`Tokenizer::encode` checks input bytes, calls the tokenizer library, checks the
output-token bound, and converts every unsigned token ID to `i32`
(`text/src/lib.rs:496-530`). It returns `InvalidTokenId` when an ID exceeds the
int32 domain and `Tokenization` for tokenizer failures. This is metadata
construction. It does not calculate logits, probabilities, scores, or loss.

`prepare_batch` turns those IDs into a fixed, batch-major `[sequences,
sequence_length]` representation (`text/src/lib.rs:575-661`):

- empty batches, sequence counts, sequence width, role IDs, and flat layout are
  bounded before allocation;
- each row applies the declared reject, keep-start, or keep-end truncation
  policy;
- padding is placed on the declared side;
- the attention mask is filled from retained positions, not inferred from the
  pad, unknown, or special token values; and
- original and retained lengths are retained as metadata.

`decode_batch_row` first compares the exact tokenizer identity, then removes
padding exclusively through the stored validity mask before decoding
(`text/src/lib.rs:663-693`). Negative IDs, invalid roles, mismatched tokenizer
identity, overflow, and configured limits are typed `TextError` failures. A
valid token equal to the pad ID remains valid because the mask, not the numeric
ID, defines occupancy.

The resulting IDs are an int32 external input when a model declares an
embedding. The compiler checks exact token positions and vocabulary bounds and
creates a GPU gather into the embedding table (`training/src/compile.rs:1643-1709`,
`training/src/compile.rs:5339-5350`). The bytes cross the run boundary as part
of the one init data image, then all embedding and subsequent model arithmetic
is represented by `CalculationNode`s. Tokenization itself never becomes one.

## `ModelContainerParsing`: validate bytes, do not decode arithmetic

The explicit compatibility symbol is `parse_safetensors`. The current parser is
`recipe-ingest::parse_safetensors` and its module starts by stating the critical
rule: encoded element formats are retained as bytes, and conversion to Recipe
f32/int32 payload is a separately scheduled GPU operation
(`ingest/src/safetensors.rs:9-13`).

### Archive and entry types

`SafeTensorDType` describes the encoded format (`BOOL`, integer widths, `F16`,
`BF16`, `F32`, and `F64`) and only supplies the encoded element width
(`ingest/src/safetensors.rs:14-65`). `SafeTensorLimits` stores nonzero bounds for
header bytes, data bytes, tensor count, rank, and name bytes
(`ingest/src/safetensors.rs:67-113`).

`SafeTensorEntry` stores a name, encoded dtype, shape, and half-open byte span;
`SafeTensorArchive` borrows the complete data section and exposes metadata,
sorted entries, and `encoded_tensor` slices without copying or decoding values
(`ingest/src/safetensors.rs:115-183`).

### Parse invariants and failures

`parse_safetensors` validates the complete image before returning an archive
(`ingest/src/safetensors.rs:229-390`):

1. The eight-byte little-endian header length must exist and fit the configured
   header bound.
2. Header and data offsets must fit `u64` and `usize`, and the image must contain
   the complete header and bounded data section.
3. Header fields, metadata keys, and tensor fields are parsed as unique,
   well-formed maps. Unknown or duplicate fields fail closed.
4. Tensor count, name bytes, and rank obey their limits.
5. The dtype must be one of the supported encoded forms.
6. Shape products and `elements * element_bytes` use checked arithmetic.
7. Offsets cannot reverse, exceed the data section, or disagree with the exact
   shape-derived byte count.
8. All entries must cover the data section contiguously, with no overlap, gap,
   or trailing unowned bytes (`validate_contiguous_data`,
   `ingest/src/safetensors.rs:419-441`).

Errors are `SafeTensorErrorKind` values such as `Truncated`, configured-limit
violations, `MalformedHeader`, `DuplicateField`, `UnsupportedDType`,
`InvalidShape`, `InvalidOffset`, `NonContiguousData`, or
`ArithmeticOverflow` (`ingest/src/safetensors.rs:185-227`). No partial archive
is exposed after an error, and no host f32 model value is produced.

### Dataset caller

The ordered dataset reader recognizes a `.safetensors` leaf in
`ingest/src/dataset.rs:825-870` and calls `parse_safetensor_tables`
(`ingest/src/dataset.rs:1503-1592`). That adapter derives bounded parser limits
from the complete source byte count, wraps parser errors as
`DatasetSourceErrorKind::MalformedFormat`, and emits ordinary logical tables:

- metadata keys and values when metadata exists;
- tensor name, encoded type, shape text, rank as exact int32, encoded byte count
  as exact int32, and the opaque binary tensor span; or
- one binary payload table when the archive has no metadata or tensor entries.

The source bytes remain ordered and bounded by `distill_datasets`; aggregate
source limits cannot be bypassed by splitting or nesting files
(`ingest/src/dataset.rs:488-540`). `Data` preparation then infers semantic
vectors and prepares only the requested representation
(`src/data_prepare.rs:140-172`). The parser's metadata table is therefore an
ingestion result, not a host-side numerical model conversion.

## `ChatTemplateRendering`: raw text before tokenization

Both legacy `render_chat` and `render_template` map to
`ChatTemplateRendering`. The current function is
`recipe_text::render_template` (`text/src/lib.rs:941-1011`). It accepts a
template, ordered `Message { role, content }` values, a generation-prompt
flag, BOS/EOS strings, and the same immutable `TextLimits` used by tokenization.

The function checks template bytes, message count, nonempty roles, and each
message's input-byte bound. It compiles the Hugging Face template, renders the
message context, generation flag, and special-token values, and then checks the
rendered-byte bound. Compilation or rendering errors are `TextErrorKind::Template`;
invalid messages are `InvalidMessage`; bound failures are `LimitExceeded`.
The output is raw text. The caller must pass it to `Tokenizer::encode` if token
IDs are required. Rendering does not read model weights, score a token, or
perform a GPU transfer. It is deterministic host preparation under
`OperationFamily::Encoding`.

## `RunShutdown`: lifecycle transition, not a kernel

The compatibility row `gpu_shutdown` represents the old teardown operation. The
current executor uses typed lifecycle transitions and never schedules a
shutdown kernel.

### Exit phase and external egress

The executor's `ExitedLoop::exit_recoverable` first runs the prepared exit phase,
then tears down resources, records `LogicalEvent::Exited`, and returns the
completed run with its exit images (`executor/src/executor.rs:1321-1371`). Exit
images are collected only from prepared external exit transfers; the executor
checks byte capacity, allocates the precomputed image, calls the backend's
`collect_exit`, and records the source identity
(`executor/src/executor.rs:2496-2665`). This is data egress, not shutdown work.

The planner and worker enforce the endpoint distinction. A logical
`Device -> External` transfer is legal only in `RunPhase::Exit`, while
`External -> Device` admission is legal only in `RunPhase::Init`
(`core/src/plan.rs:634-712`). Worker projection rejects an external admission
outside init and external egress outside exit
(`executor/src/worker.rs:667-714`). Device-to-device transfers are assigned
`InternalTransfer` in init/loop and `ExitTransfer` in exit
(`executor/src/worker.rs:719-736`).

### One free per device, then destroy runtime resources

`WorkerExecution::begin_exit` refuses to leave the loop while any loop task is
incomplete (`executor/src/worker.rs:2114-2134`). `release_arena` is legal only
after all exit tasks complete or after cancellation; it calls
`release_one_arena` for one exact device (`executor/src/worker.rs:2149-2170`).
That helper rejects a previously released image, removes exactly one arena,
calls the backend, marks the image released, zeroes the image buffer, and
journals `ArenaReleased` (`executor/src/worker.rs:2533-2561`).

`finish` requires the lifecycle to be `Exit` or `Cancelling`, requires the arena
map to be empty, takes the worker resource exactly once, calls
`destroy_resources`, marks the worker `Finished`, and journals `Exited`
(`executor/src/worker.rs:2172-2192`). A failed quiesce deliberately preserves
arenas because native work may still reference them; fatal cleanup then attempts
ordered release and destruction while retaining the first failure
(`executor/src/worker.rs:2194-2263`).

The local backend releases a host partition, CUDA arena, or HSA arena only when
the device and backend class match, then separately destroys bridge, HSA, CUDA,
and optional host resources (`native-executor/src/local.rs:2030-2100`). Bridge
legs close their worker, staging, and stream resources and retain the first
error (`native-executor/src/bridge.rs:602-612`,
`native-executor/src/bridge.rs:1725-1753`). Runtime-object destruction is
therefore accounted separately from the logical one-free-per-device transition,
as required by the contract (`system-contract.md:64-69`,
`system-contract.md:138-143`).

Lifecycle failures remain visible and typed: wrong lifecycle, incomplete loop or
exit phase, duplicate arena release, missing resources, backend failure, journal
failure, and exit-image allocation or collection errors. None causes a host
calculation fallback or silently skips a device. `RunShutdown` is metadata for
this state transition and does not appear in `CalculationGraph` or `TaskKind`.

## `EliminatedVendorWorkspaceBinding`: owned scratch replaces a setter

The old `gpu_blas_workspace` row names a vendor-library workspace setter. The
registry deliberately classifies it as
`EliminatedVendorWorkspaceBinding`; no call to a vendor BLAS workspace API is
possible. The system contract prohibits HIP and vendor math libraries and
requires Recipe-owned scalar programs, primitives, transfers, and control edges
(`system-contract.md:30-34`).

### What owns workspace now

`WorkspaceFormula::for_symbol` recognizes only explicit Recipe-owned static
workspace queries such as fixed-tree reductions, scans, stable sorts, and
solver formulas (`ops/src/workspace.rs:18-105`). `gpu_blas_workspace` is not one
of those symbols, so its lowering remains `NonCalculation`, not
`Workspace`. Calling `operations::evaluate_workspace` on its descriptor returns
`WrongLoweringKind`, rather than querying a library or inventing a size.

Structured owned operations reserve scratch as part of graph materialization.
`WorkspaceAllocation` contains exact `WorkspaceObject` entries and a byte total
(`ops/src/materialize.rs:256-315`). `GraphBuilder::intermediate` checks the
reserved identity range, checked-adds each intermediate's storage bytes, rejects
the configured workspace limit, and records the object. `finish` validates the
complete `CalculationGraph` and returns the graph plus allocation
(`ops/src/materialize.rs:712-846`). This is a typed preparation result, not a
runtime setter.

The native artifact carries `scratch_bytes_per_dispatch`; the planner turns
per-task scratch into peak per-device resource usage and rejects a candidate
that exceeds usable capacity (`planner/src/planner.rs:2978-3030`,
`planner/src/planner.rs:3550-3570`). The finalized core resource manifest stores
scratch as `DeviceBytes` alongside queues, completions, metrics, and pinned
staging (`core/src/schedule.rs:476-484`). Scratch is therefore measured,
reserved, hashed, and fixed before init.

If an operation needs a workspace formula, the formula is evaluated with checked
`u64` arithmetic and returns `WorkspaceArithmeticOverflow` on overflow
(`ops/src/workspace.rs:287-346`). If an operation's graph emits more scratch
than its bound, materialization returns `WorkspaceLimitExceeded`. The old
vendor-binding symbol has no fallback path, and no vendor-library workspace
state can leak into the immutable schedule.

## End-to-end role of the classification

The complete path for a public run is deliberately split at the boundary:

1. `recipe.data(...)`, `recipe.model()`, `recipe.train()`, and `recipe.infer()`
   construct immutable declarations. The facade sequence records only the
   latest data/model pair.
2. Training or inference consumes that pair. Ingestion snapshots files, parses
   format metadata, renders or tokenizes text when a text model requires it,
   infers semantic vectors, and prepares typed values. These are host metadata
   stages and fail before native execution when their bounds or invariants fail.
3. The compiler resolves owned operation descriptors. Scalar symbols become
   `ScalarProgram`s, direct primitives become `PrimitiveKind`s, and structured
   operations become finite `CalculationGraph`s with explicit intermediate
   values and workspace. A non-calculation descriptor cannot pass any of those
   lowering boundaries.
4. Preparation discovers hardware, realizes artifacts, allocates resources, and
   finalizes immutable arena locations and schedule windows. No non-calculation
   entry adds a task; workspace and lifecycle choices are represented by typed
   preparation state.
5. Init admits one packed external data image per required device through an
   `External -> Device` `TransferTask`. Token IDs, opaque encoded bytes, model
   parameters, and other inputs are admitted as bytes with their declared
   f32/int32 or non-payload metadata contracts.
6. The loop submits only GPU `CalculationTask`s, run-owned internal
   `TransferTask`s, and four-byte `MetricTask` readbacks. It cannot render a chat
   template, tokenize a new string, parse a file, call a vendor workspace
   setter, or destroy a runtime object. External ingress and egress are absent.
7. Exit performs the finalized device-to-external transfers, collects typed exit
   images, releases each device arena once, destroys runtime resources, and
   records the authoritative terminal journal event.

The classification thus preserves compatibility visibility without weakening
the executable ontology. It tells registry consumers why an entry is present,
where its host implementation lives, which family and determinism contract it
has, and why attempting to lower it as payload arithmetic is an error.

## Failure and invariant matrix

| Boundary | Invariant | Observable failure |
| --- | --- | --- |
| Inventory/build | Every row has exactly one symbol and source; source order and duplicate occurrence are retained. | `ops/build.rs` panics on missing fields, extra fields, empty fields, invalid UTF-8, or checked occurrence overflow. |
| Registry lookup | A bare symbol is used only when exactly one source-qualified row exists. | `UnknownOperation` or `AmbiguousSymbol`; exact pairs use `resolve_exact`. |
| Non-calculation classification | Only the six enum cases and the explicit safetensors-source rule classify here. | Unmatched entries continue to `Unsupported` or another owned lowering instead of receiving a guessed host behavior. |
| Lowering | Non-calculation descriptors have `NonNumericHostData`, host determinism, and no scalar, primitive, composition, or workspace recipe. | `WrongLoweringKind` from the requested lowering API. |
| Text | Limits are nonzero and checked; token IDs are nonnegative and int32; batch identity is exact. | `InvalidLimit`, `LimitExceeded`, `InvalidTokenId`, `Tokenization`, `InvalidBatch`, or `TokenizerMismatch`. |
| Safetensors | Header, shape, dtype, offsets, and complete contiguous data coverage agree before archive exposure. | `SafeTensorErrorKind` structural, bound, dtype, offset, contiguity, or arithmetic error. |
| Facade | Declarations are inert; a terminal consumes one complete data/model pair. | Deferred `DeclarationError`, or missing preceding declaration at `run`/`evaluate`. |
| Graph | Only calculation, transfer, and metric tasks exist; calculations and metrics are loop-only; external admission/egress have init/exit phases. | Core plan validation errors such as invalid phase, placement, route, endpoint, or resource mismatch. |
| Lifecycle | Loop must complete before exit; exit must complete before arena release; every arena is released once; resources are destroyed once. | `PhaseIncomplete`, `InvalidLifecycle`, `ArenaAlreadyReleased`, backend, journal, or teardown errors. |
| Workspace | Owned graph intermediates and scratch formulas are checked and reserved before finalization. | `WorkspaceLimitExceeded`, `WorkspaceArithmeticOverflow`, `IdentityNamespaceExhausted`, or `WrongLoweringKind` for `gpu_blas_workspace`. |
| Runtime policy | No failed boundary silently becomes CPU arithmetic, device deselection, hidden ingress/egress, or a near-match vendor artifact. | The original typed failure reaches the public training/inference result. |

## Source map

The principal trace for this classification is:

| Role | Source |
| --- | --- |
| Enum, exact symbol classifier, definitions, families | `ops/src/non_calculation.rs:1-65` |
| Raw inventory parser and generated registry input | `ops/build.rs:3-53`, `operation-surface.txt:11-14,17,60,346,419,425-426` |
| Descriptor, lowering precedence, dtype/family/determinism metadata, lookup failures | `ops/src/registry.rs:8-35,113-196,198-344,347-413,459-571,626-661` |
| Public registry and wrong-kind facade calls | `src/facade.rs:44-124`, `ops/src/lib.rs:60-82` |
| Operation errors | `ops/src/error.rs:5-66` |
| Declaration builders and validation | `src/api.rs:364-523,1005-1092,1840-2170,2181-2276` |
| Thread-local declaration sequence | `src/facade.rs:127-190` |
| Training and inference terminal callers | `src/training.rs:848-917`, `src/inference.rs:432-658` |
| Bounded source and semantic preparation | `src/data_prepare.rs:79-172`, `ingest/src/dataset.rs:488-540,825-870,1503-1592` |
| Tokenization and template rendering | `text/src/lib.rs:4-9,26-97,411-715,717-928,941-1011` |
| Safetensors structural parser | `ingest/src/safetensors.rs:9-29,67-113,115-183,185-390,419-441` |
| Calculation/transfer/metric graph types | `core/src/schedule.rs:14-19,306-450` |
| Plan phase and endpoint validation | `core/src/plan.rs:416-804,900-960` |
| Exit images, exit transfer, and ordered teardown | `executor/src/executor.rs:1321-1371,2496-2665` |
| Worker lifecycle, one-free-per-device, and finish | `executor/src/worker.rs:2114-2263,2533-2561` |
| Native arena/resource release | `native-executor/src/local.rs:2030-2100`, `native-executor/src/bridge.rs:602-612,1725-1753` |
| Owned workspace reservation and materialization | `ops/src/workspace.rs:18-105,270-346`, `ops/src/materialize.rs:256-315,712-846` |
| Normative host, payload, upload/free, and lifecycle rules | `system-contract.md:42-69,138-179,224-279,833-860` |
