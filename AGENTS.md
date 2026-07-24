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

## Commit & Pull Request Guidelines

Use concise imperative subjects consistent with history: `Add ...`, `Replace
...`, `Remove ...`, or `Complete ...`. Keep commits scoped to one coherent
change. Pull requests should explain the behavior and contract impact, identify
affected crates, link relevant issues, and list the exact validation commands
run. Call out required hardware or toolchain testing explicitly.
