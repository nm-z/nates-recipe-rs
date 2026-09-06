# Repository Guidelines

## Project Structure & Module Organization

The root `src/` directory contains the public declaration facade and the `recipe`
CLI. Implementation is divided into focused workspace crates:

- `recipe-core`, `recipe-language`, `recipe-primitives`, and `recipe-ops` define
  typed graphs and Recipe-owned calculations.
- `recipe-probe`, `recipe-planner`, `recipe-scheduler`, and `recipe-prepare`
  discover hardware and produce immutable AOT plans.
- `recipe-cuda`, `recipe-hsa`, `recipe-executor`, and
  `recipe-native-executor` own native asynchronous execution.
- Root binaries `rpc` and `tokenize` own Ethernet worker communication and
  standalone text preprocessing without library crate boundaries.

Public real-data workloads live under `examples/`; the cookbook owns measured
correctness, performance, and lifecycle gates. Normative
behavior is defined by `system-contract.md`, `topology/contract.toml`, and
`operation-surface.txt`. Packaging files are under `pkg/`, while sample inputs
belong in `examples/datasets/`.

## Build, Test, and Development Commands

```bash
cargo build --workspace
cargo check --workspace --all-targets
cargo build --examples --release
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets --keep-going
cargo run --bin recipe -- probe --help
```

Compilation, formatting, linting, and auditing are structural build hygiene;
they are not runtime evidence. Acceptance runs execute public training or
inference declarations on real datasets and real CUDA or HSA hardware with the
corresponding drivers and offline toolchains.

Always run `cargo fmt --all` immediately before pushing and immediately before
counting anything. A count made before formatting is not valid.

Never promote Clippy warnings to errors. Do not pass `-D warnings` or an
equivalent deny flag to Clippy. Use `--keep-going` so warnings in one crate do
not prevent checking the rest of the workspace.

### Clippy Policy Is User-Owned

Clippy configuration is user-owned policy. Never change Clippy unless the user
asks for a Clippy change. A request to fix code, remove warnings, make a build
pass, continue working, or complete a goal does not authorize editing lint
levels. Do not add, remove, weaken, strengthen, rename, reorder, or relocate
allow, warn, deny, forbid, expectation, priority, lint-group, command-line,
workspace, crate, module, item, or environment settings. Do not reinterpret
frustration with a warning as permission to suppress it. Do not convert a
source problem into a configuration change because the source fix is
repetitive, difficult, ugly, contradictory, or time-consuming. Do not decide
that a lint is unreasonable and disable it. Do not insert attributes as a
substitute for changing Cargo.toml. Do not pass extra Clippy flags that alter
the configured diagnostic set unless the user requests those flags. When two
lints conflict, report the conflict and leave policy unchanged. When Clippy
cannot reach zero without a policy decision, preserve the diagnostics and state
what remains. The user chooses whether a lint is allowed, warned, denied, or
removed. Only an explicit request naming Clippy configuration, a lint, a lint
group, or an allow or deny authorizes that exact change. Apply no Clippy
changes.

## Coding Style & Naming Conventions

This is a Rust 2024 workspace. Follow `rustfmt.toml`: hard tabs, 120-column
maximum, grouped imports, and formatted documentation examples. Use
`snake_case` for functions and test names, `UpperCamelCase` for types, and
`SCREAMING_SNAKE_CASE` for constants. Keep unsafe code confined to reviewed FFI
boundaries and document its invariants.

This repository contains no Python. Run repository-wide searches, diffs,
reviews, and Git inspections normally without Python-specific exclusion globs
or pathspecs.

## Acceptance Guidelines

Do not add inline, unit, mock, synthetic-data, compile-pass, or compile-fail
tests. Runtime evidence must execute the public workflow end to end on a real
dataset and real CUDA or HSA hardware, and it must fail when the observed
correctness, performance, or architectural invariant is violated. Hardware is
required rather than ignored or feature-gated; an unavailable prerequisite is
an unsuccessful acceptance run, not a passing or skipped result.

Use the Rust compiler, formatter, and linter for structural validity. They do
not prove logical correctness. Prefer gates such as Recipe
inference beating its pinned llama.cpp oracle on identical work and a complete
training run proving one pre-loop native image load with no loop-time
realization.

## Architecture and Security Constraints

Do not introduce HIP, the CUDA Runtime API, or vendor math libraries. Production
schedules must consume measured profiles; values in `topology/contract.toml`
only seed bounded probing. Preserve GPU-only f32/int32 payload calculation and
the immutable `init -> loop -> exit` lifecycle. Kernel generation may use LLVM
IR directly or MLIR, provided Recipe continues to own operation semantics,
costs, scheduling, and equivalent AMD/NVIDIA lowering.

Every executable training or inference workload must lower completely to a
graph of calculations and transfers. `TaskKind::Metric` is a specialized
four-byte device readback transfer, not a third fundamental kind of work. Init
admission and output egress are transfers. Dependencies, routes, queues,
synchronization, and lifecycle phases order or realize calculations and
transfers; do not promote those implementation concerns into additional model
semantics. Discovery, compilation, allocation, and native-image loading are
pre-loop preparation and are not model work executed in the finalized loop.

When analyzing or reducing the architecture, begin with this existing
calculation/transfer reduction and the concrete `ScalarProgram`,
`PrimitiveKind`, and scheduled-task representations. Do not invent a parallel
ontology from repository nouns. A production-LOC proposal must identify the
duplicated source regions, estimate replacement code, and state the net
workspace percentage; reject abstractions that merely rename or relocate the
same code.

Do not write bureaucratic code. Do not split one operation into layers whose
purpose is validating handoffs to one another. Such divisions create bridges;
the bridges create validation; the validation creates error types; the error
types create match arms. None of those pieces justifies the original split.
Collapse them into the single operation unless each division has an independent
architectural reason to exist.

Training runs must never use a batch size smaller than the number of rows being
trained on. Concretely: if `.split(...)` is set, the effective batch size must be
at least the training split size; if no split is set, batch size must be at least
the full prepared dataset size. No user-facing partial-batch controls are allowed.

## Exported Artifact Contract

Training is valid without `.save(...)`; omitting it exports nothing. Public
training/model artifact export may produce at most two user-owned files: a
semantic model whose extension is `.ogdl`, and a realized native kernel whose
extension is `.cubin` for CUDA or `.hsaco` for HSA. Do not export journals,
plans, caches, profiles, intermediate checkpoints, or other execution files as
model artifacts.

`.resume(...)` and `.save(...)` are independent optional training
declarations. Omitting `.resume(...)` starts a new training run and must never
disable or reject a declared `.save(...)`; the requested artifacts are written
when that new run exits. Commenting out `.resume(...)` while retaining
`.save(...)` therefore means train from scratch, then save on exit.

A declared `.resume(MODEL_PATH)` is existence-conditional. If `MODEL_PATH`
exists when training starts, load its weights and continue training. If it does
not exist, start fresh; the missing model is not an error and must not prevent
the run or any independent `.save(...)` declaration.

For `.save(...)`, the extension selects the artifact. A one-path call may save
either the model or the native kernel alone. A literal two-path call such as
`.save("model.ogdl", "kernel.hsaco")` saves both.

`.resume(...)` always requires the semantic `.ogdl` model as its first
argument. Its optional second argument may be a `.cubin` or `.hsaco` kernel;
resuming from a kernel alone is invalid because a realized kernel does not
contain the semantic training state. Thus `.resume("model.ogdl")` and
`.resume("model.ogdl", "kernel.hsaco")` are valid, while
`.resume("kernel.hsaco")` is not. Preserve the literal one-argument and
two-argument method forms; do not replace the two arguments with a tuple,
array, macro, or repeated chained calls.

Whenever saving or resuming needs a native kernel and no `.cubin` or `.hsaco`
path was supplied, Recipe recompiles the kernel from the `.ogdl` model metadata
for the current measured system. A missing kernel file is a normal
recompilation path, not an error and not a reason to invent another exported
file.

## Commit & Pull Request Guidelines

Use concise imperative subjects consistent with history: `Add ...`, `Replace
...`, `Remove ...`, or `Complete ...`. Keep commits scoped to one coherent
change. Pull requests should explain the behavior and contract impact, identify
affected crates, link relevant issues, and list the exact validation commands
run. Call out required hardware or toolchain testing explicitly.
