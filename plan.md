# Reduce Recipe LOC and Replace Proxy Tests with Architectural Proof

## Objective

Preserve the short `API.ogdl` declaration surface and every normative Recipe contract while reducing duplicated
implementation code. Replace the current broad unit/mock/semantic test inventory with a small hardware-backed
acceptance suite that executes the same public training and inference paths as a real user and fails when Recipe's
architectural promises are not realized.

Tests are not treated as proof merely because they restate an enum, error message, graph shape, or implementation
detail. A surviving test must measure an externally meaningful result or a native-runtime invariant during real work.
Acceptance tests are allowed to remain red while an architectural requirement is incomplete. Do not weaken a gate,
increase its tolerance, add a skip, or substitute a mock merely to make the suite green.

## Authority, scope, and adaptation

The intended scope is workspace-wide and as large as needed to remove proxy tests and make a serious best-effort pass
at deduplication and unification. The binding requirement is the testing philosophy above; production LOC reduction,
deduplication, and unification are best-effort outcomes, not permission to damage working semantics or invent an
abstraction solely to satisfy a line-count target.

Every specific type, package, counter, threshold, sequence, file, and implementation technique below is a current
proposal based on the repository as inspected, not an order to enact it after contrary evidence appears. Reinspect the
live code before each change. If new information shows that a proposed detail is redundant, measures the wrong thing,
creates a worse abstraction, conflicts with real hardware behavior, or risks a working contract, change or omit that
detail and record why. Prefer the simplest implementation that proves the real invariant and removes genuine
duplication; never do something nonsensical merely because this plan happened to name it.

## Non-negotiable boundaries

- Do not shorten, remove, or reinterpret `API.ogdl` declarations to reduce implementation size.
- Preserve native AMD ROCr/HSA and NVIDIA CUDA Driver execution without HIP, the CUDA Runtime API, or vendor math
  libraries.
- Preserve GPU-only f32/int32 calculation, measured-system planning, immutable `init -> loop -> exit`, and one logical
  full-training-partition optimizer update per epoch.
- Preserve the exact public `.save(...)` and `.resume(...)` one- and two-path forms and the exported-artifact contract.
- Keep CUDA and HSA FFI ownership explicit; do not force superficial DRY abstractions across different native APIs.
- Preserve the user's existing uncommitted changes in `recipe-training/src/compile.rs`.

## 1. Establish an honest LOC baseline

Record three separate numbers before changing code:

1. all tracked Rust lines;
2. proxy-test Rust lines scheduled for deletion;
3. handwritten production Rust lines excluding examples, acceptance code, generated output, and tests.

Track the third number after every production refactor. Removing tests must not be reported as production-code
simplification. Reduce handwritten production lines as far as genuine deduplication and unification safely allow,
without imposing a numeric quota or changing public behavior or artifact bytes.

Also record change amplification for each model block: the number of match sites and conversion layers touched when a
new block is added. Reducing that multiplier is more important than reducing accessor or documentation lines.

### LOC ledger

The pre-change baseline is reconstructed from `HEAD`, not from the already-edited worktree. Test and production
counts are deliberately separate.

| Measure | Pre-change | Current (2026-08-02) | Delta |
|---|---:|---:|---:|
| All Rust lines | 227,853 tracked | 174,567 workspace | -53,286 |
| Proxy-test Rust scheduled for deletion | 53,035 (29,777 in 72 named files; 23,258 in 144 inline `cfg(test)` items) | 0 | -53,035 |
| Handwritten production Rust, excluding examples, acceptance, generated output, and tests | 174,024 | 171,987 | -2,037 |

The inline baseline uses the exact `HEAD` source spans beginning at each `#[cfg(test)]` item and ending at its balanced
item boundary. The current production count uses the same path exclusions and includes the new production evidence
and shared-forward modules, but excludes `recipe-acceptance`; acceptance LOC is evidence infrastructure, not a
production-code reduction.

Current change-amplification observations:

- Forward activation selection had two separate per-activation compiler matches; one catalog now owns the semantic
  selection, while training and inference contain only generic emission by lowering kind.
- Checkpoint and KNN activation/operation token parsing and writing had four repeated mapping sites; both now call the
  canonical `DenseActivation`/`DenseOperation` token conversions.
- Parameter traversal is shared across execution evidence for every trainable block. One canonical ordered checkpoint
  tape now drives planned-manifest tensor collection, resume compatibility, and loaded-image admission for every
  block, including recurrent and residual parameters.

The current ledger includes the NVIDIA TF32 lowering and native submission/evidence work needed to make the real
performance and lifecycle gates pass; those lines are not hidden as deduplication. The production reduction remains
2,037 lines independently of the 53,035 removed proxy-test lines. Within the latest §4 pass, canonical checkpoint
traversal removed 135 lines and shared recurrent forward lowering removed a net 30 lines across `compile.rs`,
`inference.rs`, and `forward.rs`.

### Current implementation and proof status (2026-08-02)

- Proxy tests, inline test modules, compile-fail fixtures, and their test-only support are removed; the workspace has
  zero Rust `#[test]`, `#[cfg(test)]`, or `cfg!(test)` markers.
- `recipe-acceptance` is an explicit CUDA hardware runner. UCI Airfoil proves the real 1,202-row training partition,
  one logical optimizer update, six real AdamW parameter submissions, no non-GPU calculation, one bundled CUBIN
  load, zero loop realization, and complete teardown. A separate real GRU train-save-infer workload proves a
  nine-row full-partition recurrent update, one bundled CUBIN load in each run, target-free saved-model inference over
  all 12 rows, exact artifact ownership, and complete teardown.
- All nine literal save/resume/artifact/SIGINT cases pass through `recipe run` on the RTX 2050.
- GGUF Llama correctness, lifecycle, and performance gates pass. Recipe-owned NVIDIA TF32 contraction lowering,
  cohort fault readback, and allocation-free same-stream loop submission reduced the checked decode loop without
  weakening the comparison. Recipe's post-unification seven-sample median is 64,762.972580 token/s versus pinned
  llama.cpp's 30,743.025296 token/s. The complete record is
  `target/acceptance/cuda-1785672520-540204.tsv`. Both timers cover the checked native decode/loop and exclude model
  construction and teardown.
- Before consolidation, all 21 cookbook workflows passed as separate release binaries on the RTX 2050. The latest
  exact record is
  `target/acceptance/cookbooks-cuda-1785670794-454449/results.tsv`; every source, binary, and consumed dataset is
  hashed. The first real tree/forest run exposed an invalid CUDA local-memory write. NVIDIA lowering now leaves
  allocas in generic address space for LLVM's NVPTX local-memory pass, while AMD retains explicit private address
  space 5. The repaired public tree train-save-infer workflow passes and NVIDIA Compute Sanitizer reports zero errors.
- AMD/HSA code remains implemented and buildable, but no AMD result is claimed because only NVIDIA hardware is
  available now. AMD matrix cells remain visibly `not run` rather than being inferred from compilation.
- Forward activation semantics, repeated scalar programs, attention index programs, activation/operation tokens, and
  parameter-state traversal have shared owners. RNN, GRU, and LSTM now use one typed forward sequence emitter for
  training, validation, and loaded-model inference; parameter sourcing and backward tapes remain explicit. Broader
  attention/dense graph construction remains parallel where a smaller safe interface has not yet been demonstrated.
- The existing `recipe-ops` registry already provides the proposed exact `(symbol, source)` catalog, generated
  `operation-surface.txt` verification, typed identity, and fail-closed resolution. A second ABI catalog was therefore
  rejected as redundant; remaining operation-specific shape equations and variant-dependent ABI declarations stay
  handwritten until a real workload proves a smaller abstraction.
- Native evidence counts the actual retained image, entry-point, queue, completion, and persistent-allocation resources
  immediately before the real backend destroys them, then reports successful teardown with zero live resources. The
  finalized loop interface admits only prepared calculation, transfer, and metric work: compilation, loading,
  filesystem access, allocation, queue creation, and completion-object creation are not representable after loop
  entry. Consequently its zero loop-realization count records a type-level production boundary rather than a test
  adapter's guess; adding shadow counters for operations the interface cannot express would add code without stronger
  evidence.
- The final static audit passes `cargo check --workspace --all-targets`, `cargo build --workspace --release`,
  `cargo build --examples --release`, the nightly format check with the installed formatter's explicit version
  override, and `recipe-audit` in `next` mode over the absolute workspace, locked metadata, exact root package ID, and
  release ELF. The external `.antigravitycli` editor symlink is hidden read-only from audit traversal; no repository
  source or binary is excluded.
- `cargo clippy --workspace --all-targets -- -D warnings` remains red on the 2026-07-30 nightly. Its first blocking
  results are pre-existing `result_large_err`/`large_enum_variant` layout advice in `recipe-executor` and
  `recipe-probe`, followed by newly enabled style lints in `recipe-ingest`. Boxing public recoverable failures or
  applying unrelated mechanical rewrites would change layouts and add churn without proving a runtime invariant, so
  this moving-toolchain lint debt is recorded rather than misrepresented as an architectural failure or silently
  suppressed.

## 2. Replace the current test inventory

### Delete proxy tests

Delete tests whose only evidence comes from constructed values, fake backends, repeated semantic wording, private
implementation shapes, getter behavior, exact diagnostic prose, or compiler behavior already enforced by
`cargo check`. This includes redundant unit modules, compile-fail API probes, mock execution suites, generated-looking
fixtures, and test-only helper APIs that exist solely to support them.

After removing a test, remove orphaned `#[cfg(test)]` branches, fake drivers, fixture builders, dependencies, manifest
targets, and Clippy test allowances. Simplify the production types exposed by that cleanup rather than leaving dead
instrumentation behind.

Do not delete datasets, model images, logits references, or cookbook programs used by real acceptance workloads.

### Create one explicit hardware acceptance package

Add a workspace package named `recipe-acceptance`. It is not part of ordinary `cargo test --workspace`; every command
requires an explicit hardware/backend selection and fails if the selected driver, measured profile, offline toolchain,
model, oracle executable, or observation capability is missing.

The package must invoke public Recipe declarations for the workload. It may consume read-only internal execution
evidence, described below, but it must not call a shortcut compiler or executor that bypasses `recipe.data`,
`recipe.model`, `recipe.train`, or `recipe.infer`.

Each result records:

- git commit and dirty state;
- Recipe release profile and compiler version;
- measured profile, device, driver, and toolchain identities;
- exact dataset/model/oracle digests;
- warm-up policy, sample count, individual timings, and median;
- correctness result before performance comparison;
- native lifecycle/resource counters;
- pass or failure reason.

Write results beneath `target/acceptance/`; never write them beside a saved model and never treat them as exported model
artifacts.

### Add production-derived native evidence

Add a bounded internal `NativeExecutionSummary` produced by the real native preparation/execution path. It is retained
in completed lower-level execution results for acceptance inspection but is not added to the public declaration
builders, root `TrainingReport`, root `InferenceReport`, semantic model, or exported artifacts.

The summary contains per-device monotonic counters for:

- native images compiled;
- CUDA modules or HSA executables loaded;
- logical kernel functions resolved from each image;
- device allocations and releases;
- queue/stream and completion-object creation;
- calculation, transfer, and metric submissions by lifecycle phase;
- module/executable loads, compilations, allocations, and filesystem reads observed after loop entry;
- maximum live instances and resources remaining after teardown.

Increment these counters at the actual compiler, loader, allocator, and submission boundaries, not in a test adapter.
Use fixed-size or preparation-bounded storage and no loop-time allocation. The evidence must be observational and must
not affect scheduling or execution decisions.

## 3. Architectural proof gates

### Llama GGUF correctness and speed against llama.cpp

Run Recipe and a pinned llama.cpp oracle against the same checked-in dense-F32 Llama GGUF, exact token IDs, output
positions, GPU, and driver environment.

1. Build both release binaries with recorded build commands.
2. Verify input model and token digests before either run.
3. Run one untimed warm-up for each implementation.
4. Run seven interleaved measured samples in ABBA order to reduce clock/order bias.
5. Compare identical raw-logit output. Recipe must retain the existing normalized mean-square-error bound of `1e-3`
   and identical output dimensions before throughput is considered.
6. Calculate tokens per second from the identical number of evaluated tokens and use the median sample.
7. Fail if Recipe's median tokens/second is lower than llama.cpp's median. Report both distributions; do not hide a
   slower result behind a percentage tolerance.

The oracle path and arguments are versioned in the acceptance package. An absent oracle is a failed acceptance run,
not a skipped or passing test.

### One native image load and no loop-time realization

For representative dense training, saved-model inference, recurrent training, and GGUF inference on each backend:

- Require exactly one distinct CUBIN module load or HSACO executable load per participating device for the finalized
  bundled image. Multiple logical entry points may be resolved from that one image.
- Require all compilation, linking, module/executable loading, function lookup, allocation, queue creation, and
  completion-object creation to occur before loop entry.
- Require zero recompilations, module/executable loads, filesystem reads, or device allocations during the loop.
- Require zero live modules, executables, queues, events/signals, and device allocations after teardown.
- Fail on any counter overflow or missing observation.

This gate directly proves the AOT and immutable-lifecycle claim. It must use the real CUDA/HSA loader calls and real
hardware.

### Full-partition training semantics

Run a real multi-row training declaration and inspect the native execution summary plus saved model:

- one logical optimizer update per epoch;
- no public or hidden minibatch/repeated-row/padding path;
- every training row contributes to the loss and update;
- no host f32/int32 calculation submission;
- metrics represent the full declared training or validation partition;
- Ctrl+C is accepted only at a complete epoch boundary and the resulting model is loadable.

Physical GPU tiling is allowed, but it must remain recorded as physical realization beneath one logical update.

### Artifact and resume contract

Use literal public source declarations through `recipe run` to execute these real cases in a fresh temporary directory:

- no `.save(...)`: no user-owned output files;
- model-only, kernel-only, and model-plus-kernel save: exactly the requested one or two files;
- missing `.resume("model.ogdl")`: fresh training followed by any independently declared save;
- existing model resume: continued weights and a loadable result;
- model-plus-kernel resume: authenticated reuse on the same measured system;
- absent or incompatible native kernel: normal recompilation from OGDL metadata without a third exported file;
- Ctrl+C during unbounded training: safe-boundary model save followed by complete native teardown.

Fail on journals, plans, profiles, caches, checkpoints, temporary fragments, or other files appearing beside declared
artifacts.

### Backend equivalence and cookbook coverage

Run every named workflow in `examples/cookbook.rs` on at least one measured AMD and one measured NVIDIA system. For the same
seed, prepared dataset, and model declaration, compare output shape, artifact schema, lifecycle/resource invariants,
and numerical results under the contract's documented tolerance. A backend-specific unsupported path is a failure if
the public declaration claims equivalent AMD/NVIDIA execution.

Keep a dated hardware matrix in `examples/README.md`. A cell is green only after the actual public workload completed;
compilation, a mock, or declaration construction cannot mark it green.

## 4. Reduce duplicated production code

### Measured reduction budget

The current high-yield target is not a new ontology. It is the five-file `recipe-training` model pipeline:
`checkpoint.rs` (10,591 lines), `compile.rs` (12,528), `inference.rs` (4,977), `model.rs` (2,842), and
`forward.rs` (610), or 31,548 lines total. The concrete reduction target is 2,020-2,770 handwritten production
lines from the first four changes below. That is 6.4-8.8% of this subsystem and 1.2-1.6% of the current 171,987-line
production workspace. A later primitive reverse pass could raise the total to approximately 2,370-3,420 lines.

These are deletion budgets, not quotas. Each range includes the replacement code and compatibility adapters; report
the actual net change after each stage. If a stage cannot reach the low end without obscuring the mathematics,
changing artifact bytes, or weakening native behavior, stop and retain the clearer code.

| Change | Repeated code measured now | Expected replacement | Estimated net reduction |
|---|---:|---:|---:|
| One payload-parameterized block/tensor model for live, planned, and decoded state | about 1,548 lines of parallel type and family-conversion bodies | 450-650 lines of canonical types, public aliases/accessors, and `try_map`/traversal | 900-1,100 lines |
| One forward block walker for training, validation, and saved-model inference | about 1,977 lines in validation and inference block lowering, in addition to the already shared recurrent equations | retain family equations once plus 250-400 lines of parameter/tape adapters | 750-1,050 lines |
| One internal graph emitter used by both compilers | 600-750 lines of duplicated tensor identity, primitive emission, materialization, and tensor-contract insertion | 300-450 shared lines and thin error/domain adapters | 250-400 lines |
| One materializer dispatch table instead of parallel `OPERATIONS`, `supports`, and symbol matches | roughly 250-350 lines across the concrete materializer modules | one provenance-keyed function table | 120-220 lines |
| Later: reverse the existing scalar SSA and typed primitive graph instead of hand-writing differentiable backward paths | about 1,966 lines in activation/normalization and block backward lowering | primitive VJPs plus explicit non-differentiable K-means, pool-winner, and tree handling | 350-650 lines |

The checkpoint estimate comes from three representations of the same eleven block families: public decoded images at
`checkpoint.rs:561`, live states at `model.rs:2296`, and planned checkpoint values at `checkpoint.rs:7334`; plus the
family-by-family construction and conversion bodies at `checkpoint.rs:8330-8650`, `8910-9143`, and `9205-9388`.
The forward estimate comes from the validation block path at `compile.rs:6944-7665` and the saved-model block path at
`inference.rs:2801-4055`. The backward estimate is the measured `compile.rs:3526-3759` and `8381-10112` body, not an
assumption that tree construction or optimizer scheduling can be differentiated away.

### Share forward compilation

Create one internal forward-lowering implementation in `recipe-training` and make training and inference use it.

- Move shared shape checks, tensor creation, identity allocation, alias rules, forward scalar programs, and block
  traversal out of the separate compilers.
- Use a typed parameter-source boundary: training supplies initialized/resumed parameters and optimizer state;
  inference supplies loaded model tensors.
- Return one forward result containing outputs, block states, and typed parameter bindings.
- Keep gradient construction, optimizer moments, AdamW, and training metrics exclusively in the training extension.
- Delete the duplicate helpers currently present in both `compile.rs` and `inference.rs`.

After each model family moves to the shared path, run its real train-save-infer cookbook and the one-image/lifecycle
gate before deleting the old implementation.

### Use one validated semantic model

Retain the public declaration as the unchecked user-facing form, then map it once into a canonical validated semantic
model used by compilation, saving, resume, digesting, and inference.

- Parameterize tensor-bearing structures by payload: planned tensors reference `ValueId`; loaded tensors own validated
  bytes.
- Use one typed block hierarchy for dense, convolution, pool, K-means, tree, embedding, attention, RNN, GRU, LSTM,
  and residual models.
- Provide shared typed block/parameter traversal for serialization, resume compatibility, digesting, native binding,
  and artifact construction.
- Remove parallel internal checkpoint, checkpoint-image, manifest, and inference representations where they encode
  the same model facts.
- Preserve every current OGDL root, version, field order, canonical byte representation, bound, and resume rule. Do
  not introduce a format version merely for the refactor.

Concretely, introduce one internal `ModelBlock<T, P>` hierarchy, where `T` is an unoptimized tensor and `P` is a
trainable parameter triple. Use `ValueId`-bearing payloads while compiling and byte-bearing payloads after decoding.
Implement block traversal and fallible payload conversion once. Keep the existing exported checkpoint and dense-state
names as aliases or compatibility wrappers so this does not buy LOC by breaking callers.

Validate this stage with actual save, byte inspection, load, resume, inference, missing-kernel recompilation, and
artifact-count acceptance runs.

### Collapse only the duplicated materializer dispatch

Do not create another operation ABI or semantic catalog. `OperationDescriptor` and the generated
`operation-surface.txt` registry already own provenance and lowering identity, while the 112 current
`require_exact_abi` calls state operation-specific facts beside the shape equations that use them. Moving those facts
elsewhere would add indirection rather than remove meaning.

Replace only each module's parallel `OPERATIONS` list, `supports` lookup, and symbol `match` with one static table from
exact `(symbol, source)` identity to its emitter function. Keep exact names, dtype/shape equations, scalar SSA,
workspace formulas, and graph emission in the emitter. This is a small 120-220-line target, not a foundational
generalization, and should be dropped if Rust function-pointer/error plumbing consumes the saving.

### Derive backward graphs from the calculation graph only after the structural pass

The existing `ScalarProgram` SSA and `PrimitiveKind` graph are sufficient objects on which to define reverse rules;
do not add a parallel differentiation language. Add reverse rules only for scalar opcodes and primitives exercised by
current differentiable model paths. Keep tree construction, discrete K-means assignment/update, pool winner indices,
loss seeding, clipping, scheduling, and AdamW explicit where they are not ordinary differentiable forward work.

This stage is justified primarily by reducing the number of handwritten sites required for a new differentiable
operation. Its expected immediate net deletion is only 350-650 lines because a correct reverse pass itself is real
code. Proceed only if real NVIDIA train-save-infer runs produce the same parameter tensors, one-image lifecycle, and
kernel-submission evidence as the current graph; otherwise retain the manual path.

### Consolidate repeated model machinery

- Share typed parameter/moment storage, affine-gate emission, recurrent initialization, and parameter traversal across
  RNN, GRU, and LSTM while keeping each forward/backward cell equation and gate order explicit.
- Define activations once and derive façade mapping, validated-model mapping, checkpoint token mapping, and
  forward/backward program selection from that catalog.
- Share artifact save/resume routing across dense, KNN, and Bayes only where the exported contract is identical; retain
  distinct semantic payloads and native-capability restrictions.
- Remove redundant accessors and adapters made obsolete by the canonical model, but preserve currently exported public
  types unless an explicit API decision authorizes a breaking change.

## 5. Permanent validation policy

Static commands remain mandatory but are described only as build hygiene:

```bash
cargo check --workspace --all-targets
cargo build --workspace --release
cargo build --examples --release
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The production `recipe-audit` CLI must pass against the absolute workspace, locked Cargo metadata, exact package IDs,
and release binaries.

Ordinary CI runs static checks and rejects newly added proxy tests. Labeled CUDA and HSA machines run the explicit
`recipe-acceptance` package. Performance results are compared only within the same recorded hardware/toolchain class.
The merge/release gate requires all applicable architectural proof tests to pass; a known red test remains visible and
blocks claims that the corresponding architecture is complete.

## Completion criteria

- The public declaration examples are unchanged except where an acceptance workload is added.
- The proxy-test inventory and its test-only production branches are removed.
- Every surviving test performs real public training or inference, real native loading/execution, or direct production
  artifact/audit inspection.
- Llama GGUF inference is numerically valid and not slower than the pinned llama.cpp oracle on the same GPU.
- Each participating GPU loads one bundled native image before the loop and none during it.
- Loop execution performs no realization, allocation, compilation, or filesystem I/O.
- Native teardown leaves no live resources.
- Save/resume produces only the declared artifacts in every real case.
- Every currently available hardware backend records actual successful workloads; unavailable AMD/HSA cells remain
  explicitly `not run` and cannot be inferred from compilation or historical results.
- Handwritten production Rust LOC is materially reduced through genuine deduplication and unification, independently
  of deleted test LOC; no fixed reduction quota justifies a worse design.
- Final reporting lists removed duplication, before/after LOC by subsystem, exact acceptance commands, hardware used,
  and any proof gate that remains red.
