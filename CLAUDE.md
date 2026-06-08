# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

GPU-native neural network training framework in Rust. Builder-pattern API for defining models, loading data (CSV/ARFF/image dirs), training on AMD GPUs via HIP/ROCm, and evaluating. All compute runs on GPU through the `gpu-core` subcrate — no CPU ML crates.

## Build & Run

```bash
cargo build --release                    # thin LTO, links ROCm libs
cargo test                               # 19 tests (15 pipeline, 4 GPU)
cargo test tests::pipeline_tests::       # data pipeline + type detection tests
cargo test model::metric_gpu_tests::     # GPU model tests only
cargo test numeric_blanks_drop_rows      # single test by name
cargo run --release -- train.csv --target Price
```

Requires ROCm (default `/opt/rocm`). Override with `ROCM_PATH`, `ROCM_EXTRA_LIB`, `ROCM_EXTRA_INCLUDE`, `GPU_ARCH` (default `gfx1101`), `HIPCC` env vars.

## Typical Usage (train.rs script)

```rust
use nates_recipe::*;

let data = Data::load()
      .set("train.csv")
      .split(0.8)
      .target("Price");

let model = Model::new()
      .layer(64).leak()
      .layer(32).leak()
      .layer(1)
      .loss(mse)
      .lr(0.001);

let train = Train::new()
      .epochs(100)
      .log([Loss, R2, Lr])
      .plot([Loss, R2]);

train.run(&model, &data);
train.save([w, b], "model.ogdl");
```

`train.rs` is a `cargo -Zscript` file — edit in place, don't create new script files (each triggers a full rebuild).

## Architecture

```
nates-recipe (root crate)
├── src/lib.rs         — type aliases (Mat=Array2<f64>, Vec1=Array1<f64>), re-exports
├── src/main.rs        — CLI: recipe <train.csv> [--target <col>]
└── src/utils/
    ├── dataset.rs     — Data builder: .load().set().exclude().test().split().target()
    │                    CSV/ARFF parsing, value-based type detection (Kind enum), encoding, column alignment
    ├── model.rs       — Model (layer stack + chained activations), Train (run/save/resume), TUI live chart
    │                    Forward/backward, loss gradients, OGDL checkpoint, preflight checks, eval
    ├── data.rs        — train_test_split, raw CSV reader, image-dir loading, RAM guards
    └── tests.rs       — 15 pipeline tests: type detection (5 kinds + 5 edge cases) + data pipeline (5)

gpu-core (path dep, HIP/ROCm)
├── src/lib.rs         — module declarations
├── src/hip.rs         — HIP FFI bindings, set_device, stream management
├── src/memory.rs      — GpuBuffer (async alloc via hipMallocAsync), upload/download
├── src/kernels.rs     — 215 pub fns: gemm, activations, losses, metrics, optimizers, reductions
├── src/kernels/*.hip  — 50 HIP kernel source files compiled to libhipkernels.a
└── src/{attention,bayes,catboost,cluster,encoding,forest,graph,linalg,losses,
         math_ops,nn_f32,optimizers,reductions,rl,sequence,svm}.rs — domain-specific GPU ops
```

## User API (re-exported from lib.rs — what train.rs scripts use)

- **`Data`** — builder for loading datasets. `.set(path)` accepts CSV, ARFF, or a directory of files. `.exclude("col")` to drop columns. `.test(path)` for separate test file. `.split(frac)` for random split. `.target("col")` or `.target(["a","b"])` for multi-target (terminal — triggers preparation). After building: `data.set` (train Dataset), `data.test` (Option\<Dataset\>), `data.target` (target column name).
- **`Dataset`** — the encoded numeric result: `x: Mat`, `y: Vec1`, `n_targets`, `text_cols`.
- **`Model`** — layer stack. `.layer(units)` for linear dense, then chain `.relu()`, `.leak()`, `.sigmoid()`, `.tanh()`, `.selu()`, `.gelu()`, `.silu()` for activation. `.layer(embed(dim))` for token embeddings, `.layer(attn(heads))` for self-attention. `.loss(mse)` and `.lr(0.001)`.
- **`Train`** — run config. `.epochs()`, `.log([Loss, R2])`, `.plot([Loss, R2])`, `.resume(path)`. `.run(&model, &data)` trains, `.run(&model, &data.test)` infers. `.save([w, b], path)` saves params after training, `.save(["Id", data.target], path)` saves predictions after inference. Preflight checks VRAM, embed/text, and loss/output before any GPU work.
- **Consts** — losses (`mse`, `mae`, `huber`, `ce`, `bce`), metrics (`Loss`, `Accuracy`, `R2`, `Lr`, `Epoch`, `Time`), save selectors (`w`, `b`).

## Internals (gpu-core — not user-facing)

- **`GpuBuffer`** (`gpu-core/memory.rs`) — async-allocated GPU memory (`hipMallocAsync`/`hipFreeAsync`). Weights, activations, and scratch live here. Upload/download to/from host `&[f64]`.
- **`kernels`** (`gpu-core/kernels.rs`) — 215 pub FFI wrappers: gemm, activations, loss gradients, fused metrics, reductions, optimizers. Called by `model.rs` forward/backward — never by user code.
- **`LayerParams`** / **`Scratch`** (`model.rs`, private) — per-layer GPU weight buffers and ping-pong activation/gradient scratch. Allocated once at fit, reused across epochs.
- **Domain modules** (`gpu-core/src/{attention,forest,cluster,svm,...}.rs`) — GPU ops for specific algorithm families. Wired through `kernels.rs` FFI.

## Data Pipeline

1. Load CSV/ARFF, detect column types from values (Kind enum):
   - **Numeric** — continuous f64 (non-integer or unique floats)
   - **Temporal** — date strings (YYYY-MM-DD, YYYY/MM/DD) → encoded as days
   - **Categorical** — strings with repeats, or integers where every value averages ≥2 occurrences
   - **Text** — all-unique strings → tokenize → token-id sequences (SEQ_LEN=32) for `embed` layers
   - **Image** — file paths (.png/.jpg/etc) or base64 (detected, encoding not yet implemented)
   - Missing markers (NA, NaN, N/A, NULL, None, ?, ., -) filtered before detection
   - Mostly-numeric columns (≥80% f64) treated as numeric with NaN for unparseable cells
2. Categorical columns → one-hot encoding
3. When `.test()` is set, align train/test to shared columns only
4. NaN rows dropped after column selection (not before)
5. RAM guard panics if projected parse size exceeds 90% of available memory

## Training Loop

Full-batch gradient descent on GPU. No mini-batching (banned). Weights uploaded once, stay on GPU. Per-epoch: forward all layers → loss gradient → backward all layers → SGD update. Metrics (R², loss, accuracy) computed via fused GPU kernels — single scalar download per metric, never downloading predictions.

Activations: relu, sigmoid, linear, leaky_relu, prelu (learnable slope), elu, selu, tanh, silu/swish, gelu.
Losses: mse, mae, huber, ce (softmax cross-entropy), bce (binary cross-entropy).

Checkpoint: saves weights in OGDL format on first R² drop (stop-loss). Resume loads and validates shapes — crashes on mismatch, never silently discards.

## GPU Architecture (gpu-core)

`gpu-core/build.rs` compiles all `.hip` files in `src/kernels/` with hipcc/amdclang++, archives into `libhipkernels.a`, links with `amdhip64`, `rocblas`, `rocsolver`, `rocfft`.

Memory: `hipMallocAsync`/`hipFreeAsync` only. Synchronous `hipMalloc`/`hipFree` are banned at compile time by the root `build.rs` scanner.

## Build Enforcement

Root `build.rs` scans all `src/*.rs` files and panics on banned patterns:
- `hipMalloc(` / `hipFree(` (must use async variants)

## Conventions

- Edition 2024, Rust stable
- 6-space tab indentation everywhere
- `anyhow::Result` for fallible functions
- `#![deny(clippy::unwrap_used)]` — no unwraps in library code
- `f64` everywhere — fp32/mixed precision not allowed
- Full-batch only — mini-batch/batch_size banned
- Lowercase const aliases for losses/metrics/save: `mse`, `bce`, `w`, `b`, `Loss`, `R2`, etc.
- Activations are chained methods: `.layer(64).leak()`, `.layer(1).sigmoid()`
- Progress/diagnostics to stderr, never stdout

## Rules

- Diagnose before fixing — read logs, code, and docs first. No guessing.
- Test changes before claiming success.
- Minimal, focused edits. If a fix touches >3 files, explain why.
- Never hand-roll what a crate provides. Check Cargo.toml and docs.rs first.
