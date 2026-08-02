# `recipe-acceptance`

`recipe-acceptance` is Recipe's executable, real-data, real-hardware acceptance
gate. It is deliberately a binary rather than a Rust test harness. The binary
enters the public `recipe` declarations, executes the native CUDA path, and
checks independently observable results from the completed reports, journals,
native resource evidence, artifact directories, and a pinned external oracle.
Compilation, a successful process status, or emitted text alone is not a pass
condition.

This document describes the implementation in `acceptance/src/main.rs`, the
manifest in `acceptance/Cargo.toml`, and the pinned oracle under
`acceptance/oracle/`. The shorter user-facing recipe and build command is in
[`../README.md`](../README.md).

## Package and source layout

`acceptance/Cargo.toml` declares package `recipe-acceptance` version `0.1.0`,
Rust edition 2024, and one binary target:

| Path | Role |
| --- | --- |
| `src/main.rs` | The complete acceptance runner and all gates. |
| `Cargo.toml` | Binary package metadata and direct Rust dependencies. |
| `inputs.sha256` | The five required dataset and oracle-input SHA-256 records. |
| `llama.cpp-revision` | The exact llama.cpp commit required by the oracle. |
| `oracle/CMakeLists.txt` | CMake configuration for the pinned CUDA llama.cpp executable. |
| `oracle/main.cpp` | The standalone llama.cpp decode, timing, and `LGT0` writer. |
| `.docs/README.md` | This source-traced acceptance contract. |

The package sets `autotests = false` and the binary target sets `test = false`.
There are no acceptance unit tests, mocks, synthetic-data substitutes, or test
only execution paths. `recipe` is the path dependency at the workspace root.
The other direct dependencies are:

* `sha2 = "0.10"`, used to hash every manifest-named input before any gate;
* `tempfile = "3"`, used for one isolated scratch directory per invocation.

The root `recipe` crate supplies the public `recipe.data(...)`,
`recipe.model()`, `recipe.train().run()`, and `recipe.infer().evaluate()`
facade, plus the public evidence types re-exported from the engine crates. The
acceptance runner uses these evidence surfaces directly:

* `recipe-ingest` reads and prepares the delimited files, GGUF model, and token
  stream during preparation;
* `recipe-training` compiles dense and recurrent graphs, writes semantic OGDL
  checkpoints, writes realized native kernels, and exposes training evidence;
* `recipe-prepare`, `recipe-probe`, and `recipe-native-probe` reopen the exact
  measured local profile and realize the native target;
* `recipe-native-executor` and `recipe-cuda` own the CUDA resource, queue,
  completion, allocation, module, and teardown evidence;
* `recipe-executor` owns the immutable `Prepared -> Initialized -> Running ->
  Exited` journal and logical/physical call records;
* `recipe-core`, `recipe-language`, `recipe-ops`, `recipe-program`,
  `recipe-kernel`, `recipe-host`, and `recipe-hsa` are transitive parts of the
  graph, scheduling, artifact, host-transfer, and backend implementation.

No acceptance code calls HIP, the CUDA Runtime API, or a vendor math library.
Recipe's CUDA boundary is the CUDA Driver API. The acceptance record captures
the exact native profile and toolchain identities that the root runtime uses.

## Production entrypoint and command line

The production entrypoint is the `recipe-acceptance` binary, whose `main`
function creates an acceptance record, captures provenance, runs the complete
workflow, records either `pass` or `fail`, persists the record, and returns
the original gate result. The accepted command line is:

```text
recipe-acceptance cuda \
  --dataset-root PATH \
  --digest-manifest PATH \
  --recipe-cli PATH \
  --llama-oracle PATH \
  --samples 7
```

The first argument must be the literal `cuda`. The four path options are each
required exactly once. `--samples` is required and must be exactly `7`, the
runner's `REQUIRED_SAMPLES` constant. Unknown flags, duplicate path flags,
missing values, non-UTF-8 options, and any other sample count fail before a
workload begins. The runner does not accept `hsa` or an automatic backend
choice. This acceptance record therefore makes no AMD claim.

The supplied `recipe-cli` is invoked through its public `recipe run FILE.rs`
command for the artifact cases. The runner itself invokes the public Rust API
directly for the training, recurrent, and GGUF cases. The supplied oracle is
invoked as a separate executable with `--revision` and, for measured work,
`--model`, `--tokens`, and `--output`.

## Prerequisites

All prerequisites are hard requirements. A missing prerequisite is an
unsuccessful run, never a skipped gate.

### Recipe profile and native system

Run the normal bare-metal probe before the acceptance command:

```bash
cargo run --release --bin recipe -- probe
```

`recipe probe` discovers the current host and every visible native GPU, runs
bounded measurements, writes an identity-keyed measured profile, and installs
the active native receipt. The acceptance process does not perform a fresh
measurement as a fallback. Its `with_current_native_preparation` call reopens
the exact profile named by that receipt and verifies the current machine,
RAM/storage/GPU origins, native target identities, driver bindings, and pinned
tool binaries. The profile must contain at least one complete GPU calculation
device. Every gate then requires exactly one participating CUDA device.

The probe and runtime require a Linux bare-metal process, a usable CUDA Driver
library, the corresponding NVIDIA GPU, `/sys/bus/pci/devices`, and the current
host's private Recipe state directory. The state root is
`$XDG_CACHE_HOME/recipe-next` when `XDG_CACHE_HOME` is absolute, otherwise
`$HOME/.cache/recipe-next`. It and its profile, receipt, scratch, and run
subdirectories must be real, canonical, user-owned private paths. The probe
rejects container markers and PID namespaces.

The active measured configuration requires LLVM `opt` and `llc`; `ld.lld` and
`ptxas` are optional in the generic configuration but become required when the
measured CUDA target or artifact policy needs them. The runner records
`ptxas`, `opt`, `llc`, `ld.lld`, and `mold` versions for provenance. A
command being unavailable is recorded as `unavailable` during provenance
capture, but a later native preparation or compilation failure still fails the
gate.

### Dataset and manifest

`--dataset-root` is the directory that contains every relative path in
`--digest-manifest`. The checked-in manifest has exactly five non-comment
records, using the syntax `SHA256  relative/path`:

| Relative path | Required SHA-256 |
| --- | --- |
| `uci-airfoil/airfoil_self_noise.dat` | `74c75fd71783f1e6b71f8a622b993dc592897a97cd689c5090a07147a1b097b3` |
| `cookbook/binary.csv` | `27ec24779214658966d691b73784df1ac85fca8fbee7f9c5d14a35b1b9a24a40` |
| `llamacpp-archs-seed42/llama-dense.gguf` | `b4ca2f9ddb2137a434a555a4e89007469ac97b1ed08a7096b14c0e2e395a8c18` |
| `llamacpp-archs-seed42/llama-dense.logits` | `8f34febe8bf9c3d8298bbd94e6ecba7db56f0b6d793663f9ecf4625d8e05be99` |
| `llamacpp-archs-seed42/tokens.txt` | `400952c77831be04ac252671d54897cd5fce222ddaa5a4d903ee62c70555fb5c` |

The runner reads every non-comment line, requires exactly five records, reads
the corresponding real bytes, rejects Git LFS pointer text, computes SHA-256,
and compares the digest. It does not accept a missing file, malformed digest,
digest mismatch, or a manifest with a different record count. The `.dat` file
is a whitespace-delimited, headerless table with 1,503 rows. The CSV has one
header plus 12 rows and the target column `target`. The token file contains 128
integer token IDs. The GGUF and reference-logit files are the fixed dense-F32
instrument used by the final gate.

### Pinned llama.cpp oracle

`llama.cpp-revision` contains commit
`aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3`. Configure and build the oracle
from a checkout at exactly that commit:

```bash
git clone https://github.com/ggml-org/llama.cpp.git /path/to/llama.cpp
git -C /path/to/llama.cpp checkout aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3
cmake -S acceptance/oracle -B target/recipe-llama-oracle \
  -DLLAMA_CPP_ROOT=/path/to/llama.cpp -DCMAKE_BUILD_TYPE=Release
cmake --build target/recipe-llama-oracle --target recipe-llama-oracle -j
```

`acceptance/oracle/CMakeLists.txt` requires CMake 3.24 or newer, checks
`LLAMA_CPP_ROOT/include/llama.h`, checks `git rev-parse HEAD`, builds static
llama.cpp with `GGML_CUDA=ON` and `GGML_CUDA_GRAPHS=ON`, and disables unrelated
common, test, tool, example, server, app, and UI targets. The host therefore
needs a C/C++ compiler, the CUDA build toolchain, CUDA Driver support, and one
live GPU backend. The oracle itself rejects anything other than exactly one GPU
backend or a GGUF vocabulary other than 128.

### Build and invocation

Build the release Recipe CLI and acceptance binary, then run the complete gate:

```bash
cargo build --release --bin recipe
cargo run --release -p recipe-acceptance -- cuda \
  --dataset-root examples/datasets \
  --digest-manifest acceptance/inputs.sha256 \
  --recipe-cli target/release/recipe \
  --llama-oracle target/recipe-llama-oracle/recipe-llama-oracle \
  --samples 7
```

The `recipe run` child path locates a built `librecipe.rlib` beside the CLI
binary or in the normal target directories. It compiles each generated source
with `rustc`, runs it from its source directory, forwards live output, and
removes the temporary child binary. A missing library, source compilation
failure, child nonzero status, or cleanup error fails the artifact gate.

## Invocation lifecycle

`main` and `run_acceptance` execute these stages in order:

1. Create one `AcceptanceRecord` and capture static provenance. Provenance
   includes the package version, debug/release profile, Git commit and dirty
   state, `rustc -Vv`, `nvidia-smi` GPU identity, and the native tool versions
   listed above.
2. Parse the exact CUDA command line and record the supplied paths and sample
   count.
3. Verify all five manifest inputs, the regular-file Recipe CLI, and the
   regular-file llama.cpp oracle.
4. Execute the oracle with `--revision` and require the exact pinned revision.
5. Reopen the current measured native preparation and record profile schema,
   profile digest, topology digest, discovery digest, each device origin,
   target, and toolchain.
6. Create one `tempfile` directory with prefix
   `recipe-acceptance-cuda-`. Every gate writes only beneath this directory.
7. Run the four gates in fixed order: dense training, recurrent lifecycle,
   artifact contract, and GGUF llama comparison.
8. Print the final tab-delimited pass line only after all four gates succeed.

Each stage uses `?` and therefore stops at its first error. On success or any
gate error, `main` adds `result=pass` or `result=fail` and, for failure, the
error text as `failure_reason`, persists the record, and returns the outcome.
Record persistence itself is required; an inability to create or write the
record is also an invocation failure.

## Gate 1: UCI Airfoil dense training

`run_training_gate` uses the real
`uci-airfoil/airfoil_self_noise.dat` path. The public declaration is:

```rust
recipe.data(path)
    .target("col6")
    .norm(min_max)
    .split(0.8);
recipe.model()
    .perc(24)
    .silu()
    .layer(12)
    .huber()
    .layer(1)
    .loss(huber);
recipe.train()
    .optimizer(adamw)
    .epochs(1)
    .lr(0.0001)
    .cos()
    .log(Loss)
    .save("airfoil.ogdl")
    .run()?;
```

The data preparation boundary reads the source, infers its vectors, fits the
normalization on the train partition, and applies that immutable state to all
retained rows. The exact rationalized f32 split produces the measured
`TrainingBounds::train_rows`; the gate requires more than one row and records
the observed count rather than assuming a hard-coded count.

The gate requires all of these end-state facts:

* the report is `TrainingModelKind::Dense`;
* its journal contains `Prepared`, `Initialized`, `LoopStarted`,
  `LoopCompleted`, and `Exited`, exactly one `LoopIterationStarted`, exactly
  one `LoopIterationCompleted`, exactly one physical `BindResources`, and
  exactly one physical `DestroyResources`;
* its realized native set contains exactly one nonempty `Cubin` image with a
  nonempty logical entry table;
* native evidence describes exactly one CUDA device, exactly one module/image
  load, at least one entry lookup, queue, completion object, and persistent
  allocation, zero loop-time realization calls, completed teardown, and zero
  live resources after teardown;
* training evidence reports one logical optimizer update per epoch, at least
  one optimizer parameter task, equal optimizer-task and optimizer-submission
  counts, one loop iteration started and completed, zero non-GPU calculation
  tasks and submissions, and zero compacted logical events;
* retained `Loss` metrics are nonempty and every retained metric sample is for
  epoch 1; and
* the top-level scratch directory contains exactly the declared nonempty
  `airfoil.ogdl` file and no other file.

The pass line is
`acceptance workload uci-airfoil-training rows <observed> iterations 1
native_images 1 result pass` with tab separators. The acceptance record also
retains the training row count, optimizer task/submission counts, compaction
counter, native resource counters, and teardown counters.

## Gate 2: GRU train, save, and target-free inference

`run_recurrent_gate` creates `recurrent-lifecycle/` beneath the invocation
scratch directory and uses the real `cookbook/binary.csv` file:

```rust
recipe.data(path)
    .target("target")
    .norm(z_score)
    .split(0.75);
recipe.model().gru(8).layer(1).loss(bce);
recipe.train()
    .optimizer(adamw)
    .epochs(1)
    .lr(0.001)
    .cos()
    .log(Loss)
    .save("recurrent-lifecycle/gru.ogdl")
    .run()?;
```

The 12 real rows and 0.75 split must produce exactly 9 training rows. The
training report must be dense, have one complete native lifecycle, one CUDA
`Cubin`, the same one-device/one-image/zero-loop-realization/complete-teardown
evidence as Gate 1, one optimizer update, equal optimizer task/submission
counts, and zero non-GPU calculation work.

The saved model is then loaded through the public target-free inference path:

```rust
recipe.data(path).exclude("target");
recipe.model().load("recurrent-lifecycle/gru.ogdl");
let inference = recipe.infer().evaluate()?;
```

This deliberately omits `.target(...)`, `.split(...)`, and `.norm(...)` because
the saved semantic model owns interpretation and target-free inference covers
every retained row. The gate requires `InferenceModelKind::Dense`, exactly 12
f32 prediction values, one complete lifecycle, one nonempty CUDA `Cubin`, and
the same native evidence invariants. The recurrent directory must contain
exactly the one nonempty `gru.ogdl` file after both runs. The pass line names
`gru-train-save-infer`, 9 training rows, and one native image per run.

## Gate 3: save/resume artifact contract

`run_artifact_gate` canonicalizes the Airfoil dataset, creates fresh
`artifact-contract/sources/` and `artifact-contract/outputs/` directories, and
generates Rust sources that all use the exact production `recipe run` command.
Each generated training source uses the same real Airfoil declaration as Gate
1, one epoch, AdamW, and a scalar Huber objective. `run_training_source` checks
the child process status, then `verify_artifact_set` sorts the output names,
requires exact equality with the expected set, and rejects zero-byte files.

The source performs ten child invocations, grouped by the runner as nine
reported contract cases because the safe-stop and stopped-model resume form one
case:

| Case | Declaration and required output set |
| --- | --- |
| No save | No `.save(...)`; output directory remains empty. |
| Model only | `.save("model.ogdl")`; exactly `model.ogdl`. |
| Kernel only | `.save("kernel.cubin")`; exactly `kernel.cubin`. |
| Literal pair | `.save("model.ogdl", "kernel.cubin")`; exactly both files. |
| Missing model resume | `.resume("absent.ogdl")` plus `.save("fresh.ogdl")`; exactly `fresh.ogdl`, proving a missing semantic resume model starts fresh and does not disable an independent save. |
| Existing model resume | Resume the model-only `model.ogdl`, save `resumed.ogdl`; exactly `resumed.ogdl`. |
| Existing pair resume | `.resume("model.ogdl", "kernel.cubin")` and save a literal pair; exactly `resumed.cubin` and `resumed.ogdl`. |
| Missing kernel resume | Resume the existing model with a nonexistent `.cubin`, save a pair; exactly `recompiled.cubin` and `recompiled.ogdl`, proving the normal recompilation path. |
| Safe stop and resume | Run the child without an epoch bound, wait for a line containing `epoch`, send `SIGINT`, require graceful epoch-boundary stop and `safe.ogdl`, then resume that model and save exactly `resumed.ogdl`. |

The literal two-path forms are part of the public source contract. The root
`recipe run` source frontend lowers `.save(model, kernel)` and
`.resume(model, kernel)` to the checked pair methods before `rustc`; this gate
therefore tests the real syntax path rather than calling an internal helper.
The semantic first resume path must be `.ogdl`; a kernel-only resume is not a
valid declaration. An existing kernel is authenticated against the semantic
model's program and digest. A missing kernel is intentionally treated as
absence and causes current-system recompilation.

The safe-stop child source asserts `report.gracefully_stopped()`, exactly one
CUDA device and image, zero loop realization calls, zero live resources after
teardown, and prints `recipe-acceptance-safe-stop-ok`. The runner waits at most
60 seconds for an epoch line, then at most 60 seconds for process completion and
teardown. Failure to observe the line, deliver `kill -INT`, finish, return a
success status, or print the marker fails the case. The following stopped-model
resume must load the saved OGDL through the same `recipe run` path.

The gate records `artifact_cases_completed=9`, zero unexpected files, a true
safe-stop epoch-boundary fact, and a true loadable stopped-model fact. It does
not claim that unlisted malformed declarations or incompatible existing
checkpoint/kernel pairs are accepted; those remain Recipe error boundaries
outside this matrix.

## Gate 4: dense-F32 GGUF Llama versus pinned oracle

`run_llama_gate` uses `llamacpp-archs-seed42/llama-dense.gguf`,
`tokens.txt`, and `llama-dense.logits`. The reference is an `LGT0` file:

* bytes 0..4 are the ASCII magic `LGT0`;
* little-endian `u32` headers declare exactly 128 positions and 128 vocabulary
  values; and
* the payload contains exactly `128 * 128` little-endian binary32 values with
  no trailing or truncated bytes.

The token table must contain exactly 128 nonempty integer IDs. Recipe declares
the real token file as data, loads the real GGUF through
`recipe.model().load(...)`, requests `[Time, Device]` logging, and calls
`recipe.infer().evaluate()`. The report must dispatch to
`InferenceModelKind::GgufLlama`, expose exactly one participating device, one
complete lifecycle iteration, exactly one nonempty CUDA `Cubin`, complete
one-device native evidence, and a 128-by-128 f32 logit image. GGUF decoding is
fail-closed for unsupported architecture or metadata, non-F32 tensors, missing
tensors, and invalid token streams. The checked instrument is the ordinary
dense-F32, causal, equal query/KV-head, full-head RoPE `llama` graph.

The pinned oracle is first run once with `--revision`, then once for an
untimed warm-up and seven measured samples. Recipe receives the same one
untimed warm-up and seven measured samples. The measured runs are interleaved
in the fixed `recipe, oracle, oracle, recipe` order until both sets contain
seven samples. Recipe elapsed time is the completed immutable native decode
loop after admission and before output publication/teardown. Oracle elapsed
time surrounds its `llama_decode` and logit collection after model/context
creation and warm-up. Setup, parsing, compilation, realization, and teardown
are outside both throughput intervals.

For every warm-up and measured run, the emitted logits are checked against the
committed reference with normalized mean squared error:

```text
NMSE = sum((actual - reference)^2) / sum(reference^2)
```

The value must be finite and strictly below `1e-3`. The runner retains the
maximum Recipe and oracle NMSE. It converts each positive finite elapsed time
to tokens per second, sorts each seven-sample set, and uses the fourth sample
as the median. Recipe passes only when its median tokens/second is not lower
than the oracle median. A lower Recipe median, an invalid sample, a missing
`elapsed_ns=...` line, a zero oracle duration, an oracle process failure, or a
logit shape/correctness failure fails the gate.

The pass record contains the position and vocabulary dimensions, maximum NMSE
for both implementations, every throughput sample, both medians, and Recipe's
physical calculation, transfer, and metric submission counts. The oracle's
native counters are intentionally zero because its C++ process is an external
comparison implementation, not Recipe evidence.

## Native and lifecycle evidence definitions

The helper `verify_one_cuda_image` is intentionally narrow: the report's
realized kernel set must have one image, its format must be `Cubin`, and both
the bytes and logical entry table must be nonempty. It does not accept a
compilation status or a file merely named `.cubin`.

The helper `verify_cuda_evidence` checks evidence captured from real CUDA
resources immediately before teardown. It requires:

* no realization call in the running loop;
* completed teardown and zero live resources afterward;
* one and only one device entry with backend `Cuda`;
* one image/module load; and
* nonzero entry lookups, queues, completion objects, and persistent
  allocations.

The helper `verify_completed_lifecycle` independently counts journal logical
events and physical calls. It does not infer completion from a report status:
it requires one start and one completion for the requested loop count, the
five lifecycle events named by the gate, and exactly one resource bind and one
resource destroy. The one-epoch training gates additionally inspect
`TrainingExecutionEvidence`, including full-partition update count, optimizer
parameter submissions, GPU-only calculation placement, and event compaction.

## Outputs and observability

The process emits tab-delimited workload lines on standard output. A successful
invocation ends with:

```text
acceptance	backend	cuda	result	pass
```

The root `AcceptanceRecord` is written once per invocation beneath
`target/acceptance/`, or beneath the equivalent `CARGO_TARGET_DIR/acceptance`
when `CARGO_TARGET_DIR` is absolute. A relative target directory is resolved
against the workspace root. The filename is
`cuda-<unix-seconds>-<process-id>.tsv`. Records are written for failed as well
as successful invocations and contain escaped tab/newline values. They include
the schema `recipe-acceptance-v1`, provenance, all five input digests, profile
and device identities, gate measurements, timing samples, medians, and the
final result or failure reason. The record is an acceptance observation, not a
model artifact.

Runner-owned generated Rust sources, child output directories, oracle `LGT0`
outputs, saved test models, and saved test kernels are beneath the one
temporary scratch directory. `tempfile::TempDir` removes that tree when the
runner exits. The only intentionally persistent output produced by this
invocation is its tab-delimited observation record. The child Recipe CLI keeps
transient compiled binaries under the private Recipe state root and removes
those binaries and transformed source files while retaining only the declared
artifact files long enough for the exact artifact checks.

## Failure and boundary contract

The acceptance runner is fail-closed. It does not retry a gate, select a
different profile, substitute a different dataset or oracle, skip unavailable
hardware, or continue after a failed gate. Typical failure classes are:

* command-line, UTF-8, duplicate-option, or sample-count errors;
* missing, malformed, Git LFS, or digest-mismatched input bytes;
* missing Recipe CLI or oracle files;
* oracle revision, argument, GPU-count, model-load, decode, output, or timing
  errors;
* missing or stale measured profile, insecure private state, bare-metal
  violation, current-origin mismatch, changed driver/toolchain identity, or
  native preparation failure;
* public declaration, ingest, semantic preparation, compilation, artifact
  save, resume compatibility, native realization, executor, lifecycle,
  placement, or teardown errors;
* exact artifact-set mismatches, empty artifacts, child compiler failures,
  nonzero child statuses, SIGINT delivery failures, or either 60-second
  safe-stop timeout; and
* logit shape, non-finite NMSE, NMSE threshold, invalid duration, incomplete
  sample set, or Recipe-versus-oracle median throughput failures.

The acceptance command is intentionally narrower than the complete public API:
it proves only the four named workloads and their explicitly listed invariants.
The root build, formatter, linter, audit, and standalone probe are structural
or diagnostic checks. They are useful prerequisites but are not substitutes for
this real-data, real-CUDA acceptance run.
