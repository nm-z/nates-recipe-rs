# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

GPU-native neural network training framework in Rust. Builder-pattern API for defining models, loading data (CSV/ARFF/zip/image dirs), training on AMD GPUs via HIP/ROCm, and evaluating. All compute runs on GPU through the `gpu-core` subcrate — no CPU ML crates.

Split into a 4-crate workspace along a strict one-way dependency DAG:

```
gpu-core                          HIP kernels, links ROCm, depends on nothing
recipe-infer → gpu-core           forward pass + ogdl load. pure tensor fns:
                                  weights + input matrix → output matrix. owns GPU device
                                  lifecycle (init/shutdown). knows nothing of Datasets/columns.
pantry       → recipe-infer       ALL data parsing (csv/arff/zip/dir loaders) + column-type
                                  detection (the trained char-level detector, embedded ogdl) +
                                  the standalone `detect` binary. no training.
nates-recipe → gpu-core,          Model/Train builder API, backward, fit, save/resume, TUI,
               recipe-infer,        eval. Data delegates loading+detection to pantry; runs the
               pantry               forward via recipe-infer. Holds the detector trainer.
```

`catboost-rs` / `lightgbm-rs` / `xgboost-rs` are workspace-`exclude`d, standalone, untouched.

## Build & Run

```bash
cargo build --release                    # thin LTO, links ROCm libs
cargo test --workspace                   # all crates (root tests nates-recipe only)
cargo test -p recipe-infer --release     # forward/KV-cache/ogdl behavioral tests (GPU)
cargo test -p nates-recipe model::metric_gpu_tests::   # GPU metric/gradient tests
cargo run --release -- train.csv --target Price        # nates-recipe CLI
cargo run --release -- detect <path>                   # column-type detection (CLI)
./target/release/detect <path>           # standalone GPU-only detector (pantry bin, no training fw)
cargo run --release --example cookbook   # API examples
cargo run --release --example train_detector           # retrain the detector → pantry/detector.ogdl
```

Requires ROCm (default `/opt/rocm`). Override with `ROCM_PATH`, `ROCM_EXTRA_LIB`, `ROCM_EXTRA_INCLUDE`, `GPU_ARCH` (default `gfx1101`), `HIPCC` env vars.

## Typical Usage (examples/cookbook.rs)

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

Examples live in `examples/` (`cookbook.rs`, `train_detector.rs`) — normal `cargo run --example` programs, not `-Zscript`. Datasets live in `datasets/` (gitignored). Edit examples in place rather than spawning new script files (each new crate target triggers a full rebuild).

## Architecture

```
recipe-infer (forward engine — tensors in, tensors out; deps gpu-core + ndarray only)
├── src/lib.rs         — re-exports; init()/shutdown() (GPU device lifecycle); human_bytes
├── src/enums.rs       — Activation, LayerSpec, LayerKind, Loss, Metric + user consts (mse/ce/R2/…)
├── src/params.rs      — Saved, LayerParams, Scaler, build_layer_params, sinusoidal_pe, concat_layer, pinned_vocab
├── src/scratch.rs     — Scratch (ping-pong activation/grad arena), vram_estimate
├── src/forward.rs     — forward_into (dense/embed/attn/conv + KV-cache flash-attn inference path),
│                        attn_forward(_cached), metric_gpu(_into), upload/zscore/nan_impute/download
└── src/ogdl.rs        — OGDL checkpoint codec: load_ogdl(_str)

pantry (all parsing + detection; deps recipe-infer only)
├── src/lib.rs         — Mat/Vec1 aliases, Kind enum, Attr struct, available_ram_bytes, re-exports
├── src/data.rs        — loaders: read_raw_csv, parse_arff, load_groups/zip/dir, image dirs, RAM guards
├── src/detect.rs      — char-level detector inference: tokenize_column, predict_kinds (runs the
│                        embedded ogdl through recipe-infer's forward), Kind/CONTEXT/VOCAB consts
├── src/main.rs        — standalone `detect` binary (init → load_groups → predict_kinds → print)
└── detector.ogdl      — trained detector weights, include_str!'d into the binary

nates-recipe (root crate — training framework; deps gpu-core, recipe-infer, pantry)
├── src/lib.rs         — type aliases, re-exports (incl. `pub use pantry::data`, recipe_infer enums)
├── src/main.rs        — CLI: recipe <train.csv> [--target <col>] | recipe detect <path>
└── src/utils/
    ├── dataset.rs     — Data builder: .load().set().exclude().test().split().target(); Dataset;
    │                    delegates loading+type-detection to pantry; encoding, column alignment
    ├── model.rs       — Model (layer stack + chained activations), Train (run/save/resume), TUI, preflight
    ├── train.rs       — backward_step, fit, eval (forward via recipe-infer)
    └── detect.rs      — the detector TRAINER: model() builder, SOURCES/MARCH labels, corpus_split,
                         instances, build_dataset (uses pantry::tokenize_column). Emits pantry/detector.ogdl.

gpu-core (path dep, HIP/ROCm)
├── src/lib.rs         — module declarations
├── src/hip.rs         — HIP FFI bindings, set_device, stream management
├── src/memory.rs      — GpuBuffer (async alloc via hipMallocAsync), upload/download
├── src/kernels.rs     — pub fns: gemm, activations, losses, metrics, optimizers, reductions
├── src/kernels/*.hip  — HIP kernel source files compiled to libhipkernels.a
└── src/{attention,bayes,catboost,cluster,encoding,forest,graph,linalg,losses,
         math_ops,nn_f32,optimizers,reductions,rl,sequence,svm}.rs — domain-specific GPU ops
```

## User API (re-exported from lib.rs — what train.rs scripts use)

- **`Data`** — builder for loading datasets. `.set(path)` accepts CSV, ARFF, or a directory of files. `.exclude("col")` to drop columns. `.test(path)` for separate test file. `.split(frac)` for random split. `.target("col")` or `.target(["a","b"])` for multi-target (terminal — triggers preparation). After building: `data.set` (train Dataset), `data.test` (Option\<Dataset\>), `data.target` (target column name).
- **`Dataset`** — the encoded numeric result: `x: Mat`, `y: Vec1`, `n_targets`, `text_cols`.
- **`Model`** — layer stack. `.layer(units)` for linear dense, then chain `.relu()`, `.leak()`, `.sigmoid()`, `.tanh()`, `.selu()`, `.gelu()`, `.silu()` for activation. `.layer(embed(dim))` for embeddings, `.layer(attn(heads))` for self-attention. `.loss(mse)` and `.lr(0.001)`. Embed behavior: with text columns → embed token ids; with no text but categoricals → embed categorical indices directly (one-hot groups collapsed to integer indices at runtime, each category gets a unique embedding vector); no embed → one-hot encode categoricals as usual.
- **`Train`** — run config. `.epochs()`, `.log([Loss, R2])`, `.plot([Loss, R2])`, `.resume(path)`. `.run(&model, &data)` trains, `.run(&model, &data.test)` infers. `.save([w, b], path)` saves params after training, `.save(["Id", data.target], path)` saves predictions after inference. Preflight checks VRAM, embed/text, and loss/output before any GPU work.
- **Consts** — losses (`mse`, `mae`, `huber`, `ce`, `bce`), metrics (`Loss`, `Accuracy`, `R2`, `Lr`, `Epoch`, `Time`), save selectors (`w`, `b`).

## Internals (not user-facing)

- **`GpuBuffer`** (`gpu-core/memory.rs`) — async-allocated GPU memory (`hipMallocAsync`/`hipFreeAsync`). Weights, activations, and scratch live here. Upload/download to/from host `&[f64]`.
- **`kernels`** (`gpu-core/kernels.rs`) — pub FFI wrappers: gemm, activations, loss gradients, fused metrics, reductions, optimizers. Called by `recipe-infer` (forward) and `nates-recipe` `train.rs` (backward) — never by user code.
- **`LayerParams`** / **`Scratch`** (`recipe-infer`) — per-layer GPU weight buffers and ping-pong activation/gradient scratch. Allocated once at fit, reused across epochs. Forward (`forward_into`) is in recipe-infer; backward (`backward_step`) stays in nates-recipe `train.rs` and reads the activations forward retained.
- **Domain modules** (`gpu-core/src/{attention,forest,cluster,svm,...}.rs`) — GPU ops for specific algorithm families. Wired through `kernels.rs` FFI. Note: `svm`/`cluster` neighbor / perm fns are unused prototypes (no production callers).

## Data Pipeline (lives in `pantry`)

1. Load CSV/ARFF/zip/dir, detect column types via the trained char-level detector (`pantry::predict_kinds`, embedded `pantry/detector.ogdl` run through recipe-infer). `Kind` enum:
   6 classes — **Numeric**, **Temporal**, **Categorical**, **Ordinal**, **Text**, **Image**. Detection is a trained char-level transformer (`embed(32).vocab(257) → attn(4) → 64.leak() → 6`, ~0.987 train acc), NOT magic-number thresholds (every picked constant is a banned guess — see the detector trainer in nates-recipe `detect.rs`). Each column's raw cells → byte-stream (CONTEXT=256) → the model picks the Kind.
   - **Text** → tokenize → token-id sequences for `embed` layers; **Categorical** → one-hot (or integer-index when an embed layer + no text cols); **Temporal** → encoded as days.
   - Missing markers (NA, NaN, N/A, NULL, None, ?, ., -) filtered before detection.
2. When `.test()` is set, align train/test to shared columns only.
3. NaN rows dropped after column selection (not before).
4. RAM guard (`pantry::available_ram_bytes`) panics if projected parse size exceeds 90% of available memory.

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
