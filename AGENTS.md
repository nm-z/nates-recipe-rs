# Repository Guidelines

## Project Structure & Module Organization

The root `src/` directory contains the public declaration facade and the `recipe`
CLI. Implementation is divided into focused workspace crates:

- `recipe-core`, `recipe-language`, `recipe-primitives`, and `recipe-ops` define
  typed graphs and Recipe-owned calculations.
- `recipe-probe`, `recipe-cluster`, `recipe-planner`, `recipe-scheduler`, and
  `recipe-prepare` discover hardware and produce immutable AOT plans.
- `recipe-cuda`, `recipe-hsa`, `recipe-executor`, and
  `recipe-native-executor` own native asynchronous execution.
- `recipe-transport` and `recipe-remote` implement bounded master/worker
  communication.

Package integration tests live under each crate's `tests/`; smaller unit tests
usually live in `src/tests.rs`. Normative behavior is defined by
`system-contract.md`, `topology/contract.toml`, and `operation-surface.txt`.
Packaging files are under `pkg/`, while sample inputs belong in
`examples/datasets/`.

## Build, Test, and Development Commands

```bash
cargo build --workspace
cargo check --workspace --all-targets
cargo test --workspace
cargo test -p recipe-remote
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run --bin recipe -- probe --help
```

Use package-specific tests while iterating, then run the full workspace suite.
Ordinary builds and deterministic tests do not require GPU hardware. Live probe,
CUDA, HSA, or artifact-building tests require the corresponding drivers and
offline toolchains.

## Coding Style & Naming Conventions

This is a Rust 2024 workspace. Follow `rustfmt.toml`: hard tabs, 120-column
maximum, grouped imports, and formatted documentation examples. Use
`snake_case` for functions and test names, `UpperCamelCase` for types, and
`SCREAMING_SNAKE_CASE` for constants. Keep unsafe code confined to reviewed FFI
boundaries and document its invariants.

## Testing Guidelines

Name tests after observable behavior, for example
`shared_half_duplex_token_serializes_opposing_transfers`. Add deterministic
tests for success, rejection, cleanup, and lifecycle ordering. There is no
numeric coverage target; new behavior must exercise its public boundary.
Hardware-dependent tests should remain explicitly ignored or feature-gated.

## Architecture and Security Constraints

Do not introduce HIP, the CUDA Runtime API, or vendor math libraries. Production
schedules must consume measured profiles; values in `topology/contract.toml`
only seed bounded probing. Preserve GPU-only f32/int32 payload calculation and
the immutable `init -> loop -> exit` lifecycle. Kernel generation may use LLVM
IR directly or MLIR, provided Recipe continues to own operation semantics,
costs, scheduling, and equivalent AMD/NVIDIA lowering.

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
