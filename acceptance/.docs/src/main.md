# Recipe CUDA hardware acceptance runner

This document is the executable contract for the binary in
acceptance/src/main.rs. It describes the real command boundary, the inputs
that are admitted, the native preparation path, each workload, the measured
intervals, the independently checked end state, and the failure record. Source
locations below are implementation evidence, not a second specification.

## Parseable runner manifest

~~~toml
package = "recipe-acceptance"
binary = "recipe-acceptance"
entrypoint = "recipe-acceptance cuda"
backend = "cuda"
required_samples = 7
required_digest_entries = 5
llama_cpp_revision = "aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3"
llama_positions = 128
llama_vocabulary = 128
llama_nmse_must_be_less_than = 0.001
profile_record_schema = "recipe-acceptance-v1"
record_directory = "target/acceptance"
hardware = "one real CUDA GPU, one participating native device per run"
missing_prerequisite = "failure"
~~~

The constants are acceptance/src/main.rs:20-25. The package is an executable
with autotests disabled at acceptance/Cargo.toml:1-18. This is an explicit
runtime gate, not a cargo test and not a compile-only check.

The complete invocation is:

~~~text
cargo run --release -p recipe-acceptance -- cuda \
  --dataset-root examples/datasets \
  --digest-manifest acceptance/inputs.sha256 \
  --recipe-cli target/release/recipe \
  --llama-oracle target/recipe-llama-oracle/recipe-llama-oracle \
  --samples 7
~~~

The pinned oracle is built from an exact llama.cpp checkout. The command and
the CMake identity check are in acceptance/README.md:8-25 and
acceptance/oracle/CMakeLists.txt:1-36. The current runner intentionally accepts
only the CUDA backend. It does not establish an AMD HSA result.

## Production call graph

The top-level order is fixed by acceptance/src/main.rs:57-94:

1. main constructs an AcceptanceRecord and captures provenance.
2. run_acceptance parses the command, records the requested paths and sample
   count, verifies all five digest inputs, verifies the oracle revision, records
   the input manifest and current measured native profile, and creates one
   temporary scratch directory.
3. The four gates run in order: dense Airfoil training, recurrent
   train-save-infer, artifact save/resume, and Recipe versus llama.cpp logits
   and throughput.
4. A successful run prints the final acceptance line. Any error skips later
   gates and is returned to main.
5. main writes result=pass or result=fail and always attempts to persist the
   tab-delimited record before returning the gate result.

The scratch directory uses the prefix recipe-acceptance-cuda- and is owned by
the tempfile guard, so it is removed when run_acceptance returns. Oracle output
files are scratch observations and are not exported model artifacts.

The exact implementation symbols are:

| stage | source | authoritative operation |
| --- | --- | --- |
| process entry | acceptance/src/main.rs:57-70 | construct record, run gates, persist pass/fail |
| gate orchestration | acceptance/src/main.rs:72-94 | parse, verify, profile, create scratch, call four gates |
| record creation | acceptance/src/main.rs:102-225 | target path, schema, escaped fields, provenance, persistence |
| argument boundary | acceptance/src/main.rs:250-316 | require positional cuda, four paths, and exactly seven samples |
| input boundary | acceptance/src/main.rs:318-408 | SHA-256, LFS-pointer, file, and oracle-revision checks |
| dense gate | acceptance/src/main.rs:410-547 | Airfoil full-partition training and lifecycle evidence |
| recurrent gate | acceptance/src/main.rs:548-635 | GRU training, saved-model inference, and lifecycle evidence |
| artifact gate | acceptance/src/main.rs:636-803 | literal recipe run save/resume matrix |
| generated source path | acceptance/src/main.rs:805-840 | write a public Rust declaration and invoke recipe run |
| safe-stop path | acceptance/src/main.rs:842-965 | observe an epoch, send SIGINT, and require epoch-bound stop |
| common artifact checks | acceptance/src/main.rs:967-1017 | exact directory contents, nonempty images, CUDA format |
| common native checks | acceptance/src/main.rs:1019-1120 | one CUDA device, one image load, no loop realization, teardown |
| GGUF gate | acceptance/src/main.rs:1122-1226 | warm-up, seven samples, NMSE, median throughput |
| Recipe GGUF run | acceptance/src/main.rs:1228-1290 | public load/evaluate boundary and physical call accounting |
| oracle run | acceptance/src/main.rs:1292-1332 | pinned executable, elapsed_ns, LGT0 output |
| logit and sample checks | acceptance/src/main.rs:1334-1413 | LGT0 shape, normalized MSE, tokens/s, seven-sample median |

## Invocation and input admission

### Argument grammar

parse_arguments consumes the first argument as a backend name. It must be the
literal UTF-8 string cuda. Every remaining item is a flag followed by one
value. The accepted flags are:

~~~text
--dataset-root PATH
--digest-manifest PATH
--recipe-cli PATH
--llama-oracle PATH
--samples 7
~~~

The four path flags are stored in a BTreeMap. A duplicate flag fails. An
unknown flag, missing value, non-UTF-8 value, missing required flag, or a sample
value other than 7 fails with an InvalidInput error containing usage text.
Extra values cannot be silently ignored because they are parsed as another
flag. The parser is acceptance/src/main.rs:250-316.

### Digest manifest

verify_inputs reads the supplied manifest line by line. Blank lines and lines
starting with # are ignored. Every other line must contain exactly the
recognised two-space separator between a 64-character hexadecimal SHA-256 and a
relative path. The path is joined to dataset-root and read as bytes. The
following checks are made before any workload:

* the digest text is exactly 64 ASCII hexadecimal bytes;
* the path is readable;
* the bytes are not a Git LFS pointer beginning with
  version https://git-lfs.github.com/spec/v1;
* the computed SHA-256 exactly equals the manifest digest;
* exactly five non-comment entries were observed;
* the oracle path and Recipe CLI path are regular files.

The checked-in acceptance/inputs.sha256 currently names these five byte
identities:

| relative path | SHA-256 |
| --- | --- |
| uci-airfoil/airfoil_self_noise.dat | 74c75fd71783f1e6b71f8a622b993dc592897a97cd689c5090a07147a1b097b3 |
| cookbook/binary.csv | 27ec24779214658966d691b73784df1ac85fca8fbee7f9c5d14a35b1b9a24a40 |
| llamacpp-archs-seed42/llama-dense.gguf | b4ca2f9ddb2137a434a555a4e89007469ac97b1ed08a7096b14c0e2e395a8c18 |
| llamacpp-archs-seed42/llama-dense.logits | 8f34febe8bf9c3d8298bbd94e6ecba7db56f0b6d793663f9ecf4625d8e05be99 |
| llamacpp-archs-seed42/tokens.txt | 400952c77831be04ac252671d54897cd5fce222ddaa5a4d903ee62c70555fb5c |

capture_input_manifest repeats each verified digest as a record field named
input_sha256_ followed by the relative path with slash characters replaced by
underscores. It does not perform a second digest calculation. The verification
and record projection are acceptance/src/main.rs:318-388.

### Oracle revision

verify_oracle_revision runs the supplied executable with exactly --revision.
The stdout, trimmed, must equal the contents of
acceptance/llama.cpp-revision, currently
aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3. A failed process, invalid UTF-8,
missing output, or mismatch fails before scratch creation. The C++ oracle
accepts this one-argument mode at acceptance/oracle/main.cpp:116-120.

### Provenance and the acceptance record

AcceptanceRecord::new uses CARGO_MANIFEST_DIR's workspace parent. If
CARGO_TARGET_DIR is absolute, the record directory is
that path/acceptance. If it is relative, the directory is workspace joined
with the value and then acceptance. With no variable it is workspace/target/
acceptance. The file name is cuda-UNIX_SECONDS-PROCESS_ID.tsv.

Each line is name, a tab, and value. AcceptanceRecord::field escapes backslash,
tab, newline, and carriage return, in that order. The first fields are:

~~~text
schema                 recipe-acceptance-v1
unix_timestamp         seconds since the Unix epoch
recipe_version         CARGO_PKG_VERSION
recipe_profile         debug or release
git_commit             git rev-parse HEAD, or unavailable: ...
git_dirty              true, false, or unavailable: ...
rustc                  rustc -Vv, or unavailable: ...
nvidia_gpu             nvidia-smi CSV identity, or unavailable: ...
ptxas                  ptxas --version, or unavailable: ...
llvm_opt               opt --version, or unavailable: ...
llvm_llc               llc --version, or unavailable: ...
lld                    ld.lld --version, or unavailable: ...
mold                   mold --version, or unavailable: ...
~~~

command_stdout reports a command failure as an unavailable value during
provenance capture, rather than making optional tool identity a gate failure.
It uses stderr if a successful command has empty stdout. The record itself is
persisted with fs::write and the path is printed to stderr. These details are
acceptance/src/main.rs:102-240.

The result fields are appended after all gates:

~~~text
result                 pass
result                 fail
failure_reason         textual error from the first failed boundary
~~~

Only one result branch is appended. If persisting the record itself fails, that
I/O error is returned by main and can replace the original gate error. A failed
gate still attempts to persist the record.

## Measured native profile and hardware boundary

capture_native_profile calls recipe's
with_current_native_preparation, not a synthetic profile reader. The callback
records:

~~~text
measured_profile_schema
measured_profile_digest
measured_topology_digest
measured_discovery_digest
measured_device_N
measured_target_N
measured_toolchain_N
~~~

The fields come from the MeasuredProfile and NativePreparationScope, with
target and toolchain represented by their Debug forms. A device origin is the
measured scope target origin. This is acceptance/src/main.rs:174-211.

The current-native path is authoritative in src/native_prepare.rs:368-411 and
src/cli.rs:949-1009:

1. require_bare_metal rejects a non-bare-metal execution environment.
2. The private state root is discovered and the host RAM inventory is measured.
3. An active-native receipt is loaded when present. Its profile path, cache
   identity, RAM origin, PCI root, scratch directory, native libraries, LLVM
   tools, ptxas, PTX ISA, HSA code object version, release label, and FMA
   chain are reopened and digest-checked. A missing receipt falls back to the
   exact identity derived from topology/contract.toml and the current host and
   native probe configuration. A present but malformed, stale, or changed
   receipt fails closed and must be repaired by rerunning recipe probe.
4. ExplicitPathProfileCache loads the profile whose path and identity are
   specified. It never selects an arbitrary newest profile.
5. A thread-local NativeGpuProbe is opened once and reused only while its
   configuration is identical. Each callback still rebuilds the per-run
   native preparation scope.
6. NativeGpuProbe discovers all devices. The measured profile resolves against
   the current host inventory. The scope rejects a profile that would silently
   omit a measured calculation GPU outside the local machine, and rejects an
   empty local GPU set.
7. The callback receives exact CUDA/HSA bindings, host plan, and target build
   specifications. The acceptance record only observes these identities; the
   workload gates perform the actual preparation and execution.

An active profile can therefore fail because the machine is not bare metal,
the receipt or profile is absent, a path or tool digest changed, host RAM no
longer matches, discovery differs, profile identity is stale, a required native
library is unavailable, or a complete local GPU scope cannot be formed. These
are real prerequisite failures, not skipped cases.

The profile is measured by recipe probe. The probe uses
LocalSystemDiscovery, NativeGpuProbe, LocalHostBenchmarks, ProbeEngine, the
seed contract, and ExplicitPathProfileCache in src/cli.rs:876-947. Seed values
are only probe bounds; runtime preparation consumes the resulting measured
profile.

The cache identity is a schema number plus a canonical digest. Probe derives
the current identity from the seed, host fingerprint, exhaustive GPU
descriptors, and peer descriptors before looking for a profile,
probe/src/engine.rs:203-225 and 640-718. Profile validation resolves current
RAM, storage, and GPU origins by stable keys and machine fingerprint, never by
ordinal, product name, capacity, or benchmark similarity,
probe/src/resolve.rs:76-114. ExplicitPathProfileCache additionally requires an
absolute canonical private parent, mode and ownership checks, a non-symlink
regular profile file, bounded size, and a decoded cache identity equal to the
requested identity, probe/src/cache.rs:25-138. A profile that is present but
changed or stale is therefore an explicit failure boundary.

### Native execution handoff

Dense training enters src/training.rs:1278-1338. Inference enters
src/inference.rs:602-659. Both call with_current_native_preparation, derive
runtime tuning from the measured profile, create a production
LocalCandidateFactory and NativeExecutorDriver, obtain a deferred compiler from
the measured target scope, realize a native candidate, and pass it to a
Preparer. The training path additionally accepts an authenticated prebuilt
kernel for resume when one exists. No declaration builder performs this work.

The production lifecycle is:

~~~text
measured profile
  -> target and compiler preparation
  -> native artifact realization and validation
  -> resource bind and image load
  -> init admission
  -> one or more immutable loop iterations
  -> exit transfers and output collection
  -> worker quiescence and resource destruction
~~~

The training executor is
training/src/execute.rs:2176-2294. It prepares the finalized bundle,
rejects loop external transfers, builds images, maps exit values, binds and
initializes a PreparedRun, polls RunningRun until complete, drains metrics,
exits, collects output images, and returns the journal plus native evidence.
Inference follows the corresponding path in training/src/execute.rs:1201-1430
through src/inference.rs.

NativeExecutionEvidence is defined in native-executor/src/evidence.rs:10-57.
For each participating device it retains:

~~~text
device
backend                 Cuda or Hsa
image_loads
entry_lookups
queues
completion_objects
persistent_allocations
~~~

The run-level fields are loop_realization_calls,
teardown_completed, and live_resources_after_teardown. The local backend fills
completed evidence only after bridge, HSA, CUDA, and host resources have all
been destroyed, native-executor/src/local.rs:2064-2109. The acceptance checks
these facts rather than treating a status line as proof.

## Common end-state gates

### One bundled CUDA image

verify_one_cuda_image (acceptance/src/main.rs:996-1017) reads the retained
RealizedNativeKernelSet. It requires exactly one image, NativeKernelFormat::Cubin,
nonempty image bytes, and a nonempty logical entry table. The retained type is
defined in training/src/execute.rs:257-329. It is one exact image identity,
target, toolchain, digest, byte payload, and entry set from the successful
native realization.

### CUDA driver evidence and teardown

verify_cuda_evidence (acceptance/src/main.rs:1019-1062) requires all of:

~~~text
loop_realization_calls == 0
teardown_completed == true
live_resources_after_teardown == 0
devices.len() == 1
devices[0].backend == NativeBackendKind::Cuda
devices[0].image_loads == 1
devices[0].entry_lookups > 0
devices[0].queues > 0
devices[0].completion_objects > 0
devices[0].persistent_allocations > 0
~~~

The one-device assertion is made on every native training and inference run,
even though profile preparation can describe multiple local GPUs. A run using
more than one device fails this CUDA acceptance contract.

For CUDA specifically, the counters are direct projections of retained driver
resources in native-executor/src/cuda.rs:586-603: image_loads is the number of
loaded modules, entry_lookups is the number of loaded artifact entries,
queues is the number of nonblocking streams, completion_objects is the number
of completion events, and persistent_allocations is the number of metric
buffers plus an optional scratch allocation plus the required staging
allocation. CUDA realization creates queues, completion events, pinned
staging, modules, loaded artifacts, invocation parameter blocks, metric
buffers, and egress buffers before task submission,
native-executor/src/cuda.rs:1665-1904. Its destroy path first requires every
stream to be idle and every completion event to be available, then destroys
events and streams, unloads modules, frees metric buffers, staging, and scratch,
native-executor/src/cuda.rs:2203-2245. A nonzero counter in completed evidence
therefore describes resources that existed during the run, while
live_resources_after_teardown==0 describes ownership after those resources
were destroyed.

### Lifecycle journal

verify_completed_lifecycle (acceptance/src/main.rs:1064-1120) receives the real
RunJournal. For the requested iteration count it counts
LoopIterationStarted and LoopIterationCompleted events and requires both counts
to equal that count. It also requires at least one each of Prepared,
Initialized, LoopStarted, LoopCompleted, and Exited. Finally it counts
PhysicalCall::BindResources and PhysicalCall::DestroyResources and requires
one of each.

This helper intentionally checks presence and counts, not event ordering. The
underlying journal separates logical lifecycle events from backend physical
calls. LogicalEvent and PhysicalCall are defined in
executor/src/executor.rs:151-230 and executor/src/backend.rs:166-227.

RunJournal allocates its logical and retained-physical streams from fixed
JournalCapacity derived from the finalized bundle. Repeated loop events are
retained for iteration zero and compacted thereafter unless a caller requests
full detail. Repeated physical submit and poll calls are treated similarly:
the first pending poll marker for each task remains in physical_calls while an
exact u128 count is retained in pending_poll_counts. JournalSummary separately
counts all observed and compacted logical events and physical calls. This is
why physical_calls().iter() is the right source for actual first-iteration
submission counts, while physical_calls_compacted is an observation of omitted
detail rather than proof that no call happened. The implementation is
executor/src/executor.rs:383-613.

The executor accepts a graceful stop only at the terminal boundary of a
completed active iteration. RunningRun::poll_with_progress_or_stop records
LoopIterationCompleted, reads the stop flag, and records LoopStopAccepted and
LoopCompleted without starting another iteration when requested,
executor/src/executor.rs:1155-1248.

## Gate 1: UCI Airfoil dense training

run_training_gate is acceptance/src/main.rs:410-547. It uses the real file
uci-airfoil/airfoil_self_noise.dat and writes one requested model at
scratch/airfoil.ogdl.

### Public declaration

~~~rust
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
let report = recipe
    .train()
    .optimizer(adamw)
    .epochs(1)
    .lr(0.0001)
    .cos()
    .log(Loss)
    .save(model)
    .run()?;
~~~

The builder methods only record declarations. Data::split narrows the supplied
f64 to an f32 and requires a finite value strictly in (0,1),
src/api.rs:364-523. Preparation converts the exact f32 binary value to a
rational TrainFraction, computes floor(retained_rows * numerator /
denominator), fits schemas on those train rows, and applies them to all
retained rows, ingest/src/prepare.rs:52-133 and 798-838. The checked Airfoil
file currently has 1,503 data rows, so its current 0.8 declaration produces
1,202 train rows by that floor rule. The gate deliberately checks the
independent resulting bound as greater than one rather than hardcoding 1,202.

The model declarations map to a Perc block of 24 parallel perceptrons, SiLU
operation, a dense layer of 12 with Huber activation, a dense layer of 1, and
Huber loss. The API mappings are src/api.rs:713-778, 1141-1184, and
1355-1404.

### Training execution and checks

Train::run takes the immediately preceding data and model declarations,
compiles a dense package, installs the process SIGINT guard, executes the
native training, constructs a completed TrainingReport, and then writes the
declared model artifact, src/training.rs:848-917. The report must be the Dense
variant. KNN and Bayesian preparation are explicitly not accepted as a
substitute.

The gate then checks, in order:

1. journal lifecycle for exactly one complete loop iteration;
2. exactly one nonempty CUDA cubin image and logical entry table;
3. one-device CUDA native evidence, exactly one image load, nonzero entry
   lookups, queues, completion objects, persistent allocations, no loop
   realization, and complete teardown;
4. TrainingExecutionEvidence:

   ~~~text
   bounds.train_rows > 1
   logical_updates_per_epoch == 1
   optimizer_parameter_tasks > 0
   optimizer_parameter_submissions == optimizer_parameter_tasks
   loop_iterations_started == 1
   loop_iterations_completed == 1
   non_gpu_calculation_tasks == 0
   non_gpu_calculation_submissions == 0
   logical_events_compacted == 0
   ~~~

   The evidence joins compiled parameter-update task identities with actual
   PhysicalCall::SubmitCalculation records, and joins non-GPU task identities
   with actual submissions, training/src/execute.rs:385-439 and 2296-2401.
5. at least one retained final metric, with every metric sample present and
   its one-based epoch equal to 1;
6. exactly one scratch entry, the requested airfoil.ogdl file.

The record stores train rows, epoch count, accepted updates, optimizer task and
submission counts, host-calculation counts, native image/resource counters,
loop realization calls, live resources after teardown, and the journal's
physical call compaction count. The field name
training_pending_poll_calls_compacted is historical; its value is
TrainingExecutionEvidence::physical_calls_compacted, not a separate pending
poll count.

The printed pass line is:

~~~text
acceptance  workload  uci-airfoil-training  rows  <train_rows>
iterations  1          native_images  1     result  pass
~~~

The actual output uses tabs, not the spacing shown above.

## Gate 2: recurrent GRU train, save, and infer

run_recurrent_gate is acceptance/src/main.rs:548-635. It creates
scratch/recurrent-lifecycle, reads cookbook/binary.csv, and declares
scratch/recurrent-lifecycle/gru.ogdl.

### Training declaration and proof

~~~rust
recipe.data(path)
    .target("target")
    .norm(z_score)
    .split(0.75);
recipe.model().gru(8).layer(1).loss(bce);
let training = recipe
    .train()
    .optimizer(adamw)
    .epochs(1)
    .lr(0.001)
    .cos()
    .log(Loss)
    .save(model)
    .run()?;
~~~

The CSV has one header and 12 real rows. The exact rational 0.75 split yields
9 train rows, which this gate checks directly. The report must still be
TrainingModelKind::Dense because GRU is a dense native training graph, not a
reference-only model. The gate checks the same lifecycle and CUDA image
requirements as Airfoil, then requires:

~~~text
training.bounds.train_rows == 9
training.logical_updates_per_epoch() == 1
training.optimizer_parameter_tasks() > 0
training.optimizer_parameter_submissions() == training.optimizer_parameter_tasks()
training.non_gpu_calculation_tasks() == 0
training.non_gpu_calculation_submissions() == 0
~~~

The declared GRU is the first leading reset-before scalar-sequence block. For
hidden width H=8 it consumes one normalized numeric scalar per feature column,
starts every independent row at h0=0, and emits only the final hidden state.
The owned native graph has separate reset, update, and candidate input and
recurrent weights plus three biases, and uses

~~~text
r_t = sigmoid(x_t W_xr + h_(t-1) W_hr + b_r)
z_t = sigmoid(x_t W_xz + h_(t-1) W_hz + b_z)
n_t = tanh(x_t W_xn + (r_t * h_(t-1)) W_hn + b_n)
h_t = (1 - z_t) * n_t + z_t * h_(t-1)
~~~

Training unrolls this graph, accumulates all nine parameter gradients across
every step and supervised row, and performs the one full-partition AdamW
transition. The validation and target-free inference graphs replay the same
zero-state recurrence. These semantics are the C39 contract in
system-contract.md:715-734 and the compile paths are
training/src/compile.rs:5572-5645 and 7352-7395.

### Target-free saved-model inference

After training, the gate replaces the public data declaration with:

~~~rust
recipe.data(path).exclude("target");
recipe.model().load(model);
let inference = recipe.infer().evaluate()?;
~~~

Inference requires a checkpoint source with .ogdl or .gguf extension and
rejects target, split, and normalization declarations because the saved
semantic model owns that interpretation, src/inference.rs:500-581. The
inference report must be Dense and must expose exactly 12 f32 values, one for
every real row. It then receives the same one-iteration lifecycle, one cubin
image, one-device CUDA evidence, and teardown checks.

The root directory must contain exactly gru.ogdl and no native kernel or
journal file. The record stores training rows, native image loads for the
training and inference runs, and the 12 inference rows. The pass line is:

~~~text
acceptance  workload  gru-train-save-infer  rows  9
native_images_per_run  1  result  pass
~~~

## Gate 3: save and resume artifact contract

run_artifact_gate is acceptance/src/main.rs:636-803. It creates two
subdirectories, sources and outputs, under a fresh
scratch/artifact-contract root. The Airfoil path is canonicalized before being
embedded in generated source.

### Real production CLI boundary

run_training_source writes one Rust file in sources. Its body is a normal public
Recipe declaration:

~~~rust
use recipe::*;
fn main() -> TrainingResult<()> {
    recipe.data(DATA).target("col6").norm(min_max).split(0.8);
    recipe.model().layer(1).loss(huber);
    recipe.train().optimizer(adamw).epochs(1).lr(0.0001)
        RESUME SAVE
        .run()?;
    Ok(())
}
~~~

RESUME and SAVE are literal fragments or empty strings. The function then
executes the supplied binary as recipe run SOURCE, captures its complete
stdout/stderr with Command::output, and fails with both streams if the process
status is unsuccessful, acceptance/src/main.rs:805-840.

The CLI is not an internal function call. src/cli.rs:125-167 dispatches the
run subcommand to run_source. run_source canonicalizes and validates the
source file, locates the built Recipe library, lowers public syntax, invokes
rustc with the linked library, forwards compiler diagnostics, executes the
compiled binary in the source directory, removes the temporary binary, and
returns a failure containing the captured output tail when the child exits
unsuccessfully, src/cli.rs:272-361 and 433-602. Literal two-argument forms are
lowered by source_frontend.rs:373-517:

~~~text
.save(MODEL, KERNEL)   -> .__recipe_save_pair(MODEL, KERNEL)
.resume(MODEL, KERNEL) -> .__recipe_resume_pair(MODEL, KERNEL)
.run(MODEL, DATA)      -> .__recipe_run_with(MODEL, DATA)
~~~

The one-argument forms remain the public methods. This preserves the literal
API shape while giving rustc a typed implementation target.

### Exact cases and independently checked files

Each case uses a new output directory. verify_artifact_set lists the directory,
converts file names to UTF-8, sorts observed names, requires exact equality
with the expected list, and rejects zero-byte entries,
acceptance/src/main.rs:967-990.

| case label | declaration | expected directory |
| --- | --- | --- |
| no-save | no resume, no save | empty |
| model-only | .save(model.ogdl) | model.ogdl |
| kernel-only | .save(kernel.cubin) | kernel.cubin |
| model-kernel-pair | .save(model.ogdl, kernel.cubin) | kernel.cubin, model.ogdl |
| missing-resume | .resume(absent.ogdl), .save(fresh.ogdl) | fresh.ogdl |
| existing-resume | .resume(model.ogdl), .save(resumed.ogdl) | resumed.ogdl |
| existing-pair-resume | .resume(model.ogdl, kernel.cubin), .save(resumed.ogdl, resumed.cubin) | resumed.cubin, resumed.ogdl |
| absent-kernel-resume | .resume(pair-model.ogdl, missing.cubin), .save(recompiled.ogdl, recompiled.cubin) | recompiled.cubin, recompiled.ogdl |
| safe-stop | unbounded training, SIGINT after an observed epoch, save(safe.ogdl) | safe.ogdl |
| safe-stop-resume | .resume(safe.ogdl), .save(resumed.ogdl) | resumed.ogdl |

The source currently executes ten output-directory cases, including both
safe-stop and safe-stop-resume. It records and prints
artifact_cases_completed=9 and cases=9 at acceptance/src/main.rs:797-801.
That literal field is therefore a stale count; the authoritative behavior is
the ten calls and ten exact directory checks above. This mismatch does not
make an individual case pass without its own end-state check.

### Artifact semantics exercised by the cases

The public policy is implemented in src/api.rs:1991-2079 and executed in
src/training.rs:746-917:

* one-argument save accepts .ogdl for the semantic model and .cubin or .hsaco
  for the native kernel;
* the literal two-argument save form requires model first and native kernel
  second;
* one-argument resume requires a semantic .ogdl model;
* two-argument resume requires .ogdl first and .cubin or .hsaco second;
* omitting save writes no model or kernel;
* model save and kernel save are independent;
* an existing resume model is loaded and applied to the current compiled
  training graph;
* a missing resume model is treated as a fresh run;
* a missing resume kernel returns no prebuilt bundle and therefore exercises
  current-system recompilation;
* an existing resume kernel is authenticated against checkpoint program,
  topology, discovery, target, toolchain, and byte digest. A mismatch fails
  instead of falling back to an unrelated image.

Dense TrainingReport saving occurs only after native teardown. The extension
selects NativeKernelFormat::Cubin or Hsaco at src/training.rs:902-915. The
case directories consequently contain at most the two user-declared files,
never a journal, plan, profile, cache, or intermediate checkpoint.

### SIGINT safe-stop case

run_safe_stop_source generates a source with no .epochs call:

~~~rust
let report = recipe.train()
    .optimizer(adamw)
    .lr(0.0001)
    .log(Epoch)
    .save(SAFE_MODEL)
    .run()?;
assert!(report.gracefully_stopped());
assert_eq!(report.native_evidence().unwrap().devices().len(), 1);
assert_eq!(report.native_evidence().unwrap().devices()[0].image_loads, 1);
assert_eq!(report.native_evidence().unwrap().loop_realization_calls(), 0);
assert_eq!(report.native_evidence().unwrap().live_resources_after_teardown(), 0);
println!("recipe-acceptance-safe-stop-ok");
~~~

Without .epochs, the training horizon is unbounded. .lr supplies a constant
linear schedule for that horizon, and the SIGINT control is required for an
unbounded loop, src/training.rs:626-665 and training/src/execute.rs:90-128.
The .log(Epoch) presenter emits a line beginning with epoch after a completed
iteration, src/training.rs:1557-1677.

The acceptance process starts recipe run with stdout and stderr pipes and
reader threads. It waits up to 60 seconds for an stdout line containing
epoch. On timeout it kills and waits for the child and returns a failure with
both captured streams. After observing the line it executes the OS kill
program with -INT and the child PID, waits up to another 60 seconds for normal
exit, joins both readers, and requires a successful status plus the marker
recipe-acceptance-safe-stop-ok, acceptance/src/main.rs:842-965.

The CLI forwards one SIGINT from its own SigintGuard to the compiled source
child, src/cli.rs:433-507. The source process guard stores an atomic request;
TrainingExecutionControl reads it only after the active loop iteration reaches
terminal completion. The executor records LoopStopAccepted and exits without
starting another iteration, src/signal.rs:12-89 and
executor/src/executor.rs:1163-1232. The source-level assertions independently
check that the accepted stop has completed native teardown and left exactly one
image load.

## Gate 4: dense-F32 GGUF Llama correctness and speed

run_llama_gate is acceptance/src/main.rs:1122-1226. It uses:

~~~text
llamacpp-archs-seed42/tokens.txt
llamacpp-archs-seed42/llama-dense.gguf
llamacpp-archs-seed42/llama-dense.logits
~~~

The token file must contain exactly 128 nonempty lines. The reference logits
must be an LGT0 file with positions=128 and vocabulary=128, and therefore
16,384 little-endian f32 values. read_lgt0 performs the header, shape, exact
payload length, and chunk checks at acceptance/src/main.rs:1360-1386.

Recipe's GGUF boundary is stricter than the acceptance file checks. The decoder
requires GGUF v3 little-endian bytes, general.architecture=llama, nonzero
vocabulary/context/embedding/block/feed-forward/head metadata, equal query and
KV head counts, an even full-head rotary dimension, equal key/value head
widths, no experts, causal attention, and no parallel residual. It validates
positive finite RMS epsilon, RoPE base and scaling, attention scale, and Q/K/V
clamp metadata, requires zero SwiGLU clamps, and captures only F32 tensors with
the exact declared dimensions. Required tensors include token embedding,
output norm, every block norm and projection, and the optional output weight,
biases, scales, and RoPE factors. Any unconsumed tensor is rejected as an
unsupported variant. These checks are training/src/gguf_llama.rs:186-476.

The target-free table must have width one. Each cell is parsed as an exact
int32 token, the stream must be nonempty and no longer than the saved context,
and every token must be in 0..vocabulary,
training/src/gguf_llama.rs:485-565. The checked corpus therefore exercises
both the container/tensor contract and the separate token-stream contract
before native preparation.

### Recipe run

run_recipe_llama enters the public declaration boundary for every invocation:

~~~rust
recipe.data(tokens);
recipe.model().load(model);
let report = recipe.infer().log([Time, Device]).evaluate()?;
~~~

InferenceReport::evaluate resolves the preceding declarations, requires
target-free data and a .ogdl or .gguf model, loads the GGUF artifact, compiles
the immutable graph, performs native preparation and execution, writes the
public report after teardown, and returns the completed report,
src/api.rs:2234-2240 and src/inference.rs:482-659. The report must be
InferenceModelKind::GgufLlama and expose exactly one participating device.

The run then checks the common one-iteration lifecycle, one nonempty Cubin,
one-device CUDA evidence, and complete teardown. It counts physical calls in
the real journal:

~~~text
calculation_submissions
  PhysicalCall::SubmitCalculation
transfer_submissions
  SubmitInternalTransfer
  SubmitExitTransfer
  SubmitExternalIngress
  SubmitExternalEgress
metric_submissions
  PhysicalCall::SubmitMetric
~~~

The returned TimedLogits stores report.elapsed(), all f32 values from
report.values(), and native counters. InferenceReport::elapsed is documented
as one complete checked native inference loop, with setup, output collection,
and teardown outside the interval, src/inference.rs:264-307.

### Pinned llama.cpp oracle

run_oracle invokes:

~~~text
ORACLE --model MODEL --tokens TOKENS --output OUTPUT
~~~

It requires a successful process, finds a stdout line beginning
elapsed_ns=, parses a nonzero u64, and reads the output with the same LGT0
decoder. It assigns zero to Recipe-native counters because those counters are
not meaningful for the external process.

The oracle implementation is acceptance/oracle/main.cpp:116-198. It:

1. requires exactly 128 valid integer token IDs;
2. loads all GGML backends and requires GPU offload plus exactly one GPU
   backend;
3. loads the model with n_gpu_layers=-1 and requires a 128-token vocabulary;
4. creates a context with n_ctx, n_batch, and n_ubatch all 128;
5. performs one untimed llama_decode warm-up and clears llama memory;
6. measures one llama_decode plus logit collection with steady_clock;
7. writes LGT0 logits and frees batch, context, and model.

The CMake project requires the exact llama.cpp revision and enables static
CUDA and CUDA graphs, acceptance/oracle/CMakeLists.txt:8-36. The acceptance
runner itself verifies the revision before this gate.

### Warm-up, samples, and interval

The gate performs one untimed Recipe run and one untimed oracle run first. Both
must pass the same reference NMSE check. It records the Recipe warm-up native
image, entry, queue, completion, persistent-allocation, calculation-transfer-
metric counters. It then performs exactly seven measured runs per
implementation. The deterministic scheduling pattern is:

~~~text
recipe, oracle, oracle, recipe, recipe, oracle, oracle, recipe, ...
~~~

The loop repeats the four-item pattern until both sample vectors have length
seven. Every measured run independently checks logits before recording its
throughput. Oracle output paths are
llama-oracle-warmup.logits and llama-oracle-0.logits through
llama-oracle-6.logits under scratch.

tokens_per_second divides the 128 token count by the checked elapsed duration.
The duration must be positive and finite. median requires exactly seven
positive finite samples, sorts with f64::total_cmp, and selects index 3,
acceptance/src/main.rs:1388-1407.

Recipe's elapsed interval is report.elapsed, after immutable loop completion
and before output publication/teardown. The oracle's interval starts immediately
before decode_all and ends after logits have been copied. Parsing, model load,
compilation, native realization, warm-up, and teardown are outside both
throughput samples. They remain covered by lifecycle and image evidence.

### Correctness and performance thresholds

verify_logits requires actual length 128*128 and equality with reference
length. It computes:

~~~text
squared_error = sum((actual_f32_as_f64 - expected_f32_as_f64)^2)
reference_energy = sum(expected_f32_as_f64^2)
nmse = squared_error / reference_energy
~~~

Nonfinite NMSE and NMSE greater than or equal to 1.0e-3 fail. A finite NMSE
strictly below the threshold passes that run. The gate retains the maximum
NMSE across warm-up and all seven measured runs for each implementation.

After sampling it records:

~~~text
llama_positions = 128
llama_vocabulary = 128
recipe_maximum_nmse
llamacpp_maximum_nmse
recipe_tok_s_samples
llamacpp_tok_s_samples
recipe_median_tok_s
llamacpp_median_tok_s
~~~

It prints both sample vectors and fails when Recipe's median tokens per second
is lower than the oracle median. Equal medians pass. Correctness is therefore
an absolute reference gate and performance is a median comparison, not a
single-best sample comparison.

## Failure boundaries and observable output

The runner is fail-closed. Every gate returns AcceptanceResult and is called
with ?. The first error stops the remaining gates. Typical direct failure text
identifies:

~~~text
missing backend or option
acceptance requires exactly 7 measured samples
digest manifest syntax, digest mismatch, Git LFS pointer, or wrong entry count
pinned llama.cpp oracle does not exist
llama.cpp oracle revision mismatch
native profile, device, target, toolchain, or binding identity failure
wrong report model family
missing journal, image, training evidence, or native evidence
invalid lifecycle counts or missing lifecycle event
native loop realization or incomplete teardown
non-GPU calculation task or submission
missing or extra artifact
safe-stop epoch or teardown timeout
invalid LGT0 header, shape, payload, or logits
NMSE at or above 1e-3
invalid or incomplete throughput samples
Recipe median slower than llama.cpp
~~~

The error is stored as failure_reason after being converted to a string. A
successful workload emits a tab-separated acceptance line only after all of
its end-state checks. The final backend pass line is emitted only after all
four gates pass:

~~~text
acceptance  backend  cuda  result  pass
~~~

The process status is the result of run_acceptance unless record persistence
fails. The record path is printed to stderr even for a failed gate when
persistence succeeds. No pass or skip status is emitted for an unavailable
GPU, driver, profile, tool, dataset, oracle, or other prerequisite.

## Evidence interpretation

The acceptance record is an observation of one real machine and one exact
input revision. It records hardware and measured-profile identities, but it is
not a model artifact. A successful run proves the checked public declarations
traversed the real data preparation, measured native target, image
realization, CUDA driver execution, immutable lifecycle, output publication,
and teardown paths on that machine.

Compilation, rustc diagnostics, an executable status alone, a printed metric,
or a report kind alone is not sufficient. The gates independently inspect
resulting row counts, f32 output shape, NMSE, physical submission counts,
logical lifecycle events, retained native image bytes and entries, actual
driver-derived resource counters, exact artifact directory contents, and
post-teardown resource counts. The only intentionally historical observation
in the current source is the artifact_cases_completed=9 label while ten
artifact output directories are exercised.
