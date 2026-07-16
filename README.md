# recipe

GPU-native f64 NN training/inference in Rust. All GPU math is our own `.hip` kernels via `gpu-core`; no CPU ML crates, no vendor BLAS on the hot path. AMD RDNA3 is the primary target (gfx1101), NVIDIA runs through HIP-on-CUDA over the same kernel set.

## Setup

- Toolchain is pinned by `rust-toolchain.toml` (nightly); rustup picks it up on its own. Edition 2024, resolver 3.
- `hipconfig` must be on `PATH`. The build detects the backend from `hipconfig --platform` and locates the ROCm tree from `hipconfig --rocmpath`. Missing or unrecognized platform is a hard build failure that names the package to install; there is no fallback.
- `datasets/` is Git LFS (`.gitattributes`: `datasets/** filter=lfs`), about 3.3 GiB. A plain clone smudges all of it. If the machine only needs source (compute/serve node), skip the smudge and keep the ~110 MB tree of pointer files:

```bash
GIT_LFS_SKIP_SMUDGE=1 git clone https://github.com/nm-z/nates-recipe-rs
```

## Build

```bash
cargo build --release
```

Thin LTO, links the detected GPU backend. Kernels in `gpu-core/src/kernels/*.hip` compile through the `cc` crate with `hipcc` (`nvcc` on the NVIDIA path). Produces two binaries:

- `target/release/recipe` (training/eval/inference CLI)
- `target/release/detect` (standalone column-type detector)

New `.hip` files under `gpu-core/src/kernels` are discovered automatically (`build.rs` watches the directory). Adding a new crate target or a new `-Zscript` file triggers a full ~15 min workspace rebuild, so edit the existing examples in place instead of adding drivers.

### Env overrides

Each shows its default; `<rocm>` and `<cuda>` are the resolved roots.

| Variable | Default | Effect |
| --- | --- | --- |
| `GPU_ARCH` | `gfx1101` | AMD `--offload-arch` passed to the kernel compiler |
| `HIPCC` | `<rocm>/bin/hipcc` (falls back to `<rocm>/bin/amdclang++`) | Compiler for `.hip` kernels |
| `ROCM_EXTRA_INCLUDE` | `<rocm>/include` | Extra `-I` dir for AMD kernel compilation |
| `ROCM_EXTRA_LIB` | `<rocm>/lib` | Extra link-search dir |
| `CUDA_PATH` | `/opt/cuda` | CUDA toolkit root (NVIDIA backend) |
| `CUDA_ARCH` | `sm_86` | NVIDIA `-arch` target |
| `NVCC` | `<cuda>/bin/nvcc` | CUDA compiler (NVIDIA backend) |
| `ROCM_PATH` | `/opt/rocm` | Runtime only: ROCm lib search path when the CLI JIT-compiles a script |

## Test

```bash
cargo test all
```

That is THE suite: every test in every crate, one OS process per test, 60s SIGKILL deadline each (a kill is that test's FAIL, the suite continues), one log at `suite.log`. Verdict:

```bash
rg '^FAIL' suite.log
```

The suite skips tests whose fn-body hash matches `target/.suite_cache`; delete that file to force a full re-run.

Narrower runs:

```bash
cargo test -p recipe-infer --release            # forward / KV-cache / ogdl behavioral tests (GPU)
cargo test -p gpu-core --release --test suite   # kernel proof suite alone
```

The gpu-core test target is named `suite`, not `all`, and no test id may contain the substring "all" (cargo filters by substring, so it would collide with `cargo test all`).

`kernel_inventory.db` at the repo root (committed, ~3.7 MB SQLite) is live test data for the gpu-core proof suite; around 20 tests fail without it. Leave it alone.

## Run

Quick train from a CSV:

```bash
cargo run --release -- train.csv --target Price
```

Column-type detection on anything loadable (csv / arff / directory / zip, globs expand):

```bash
cargo run --release -- detect <path>    # or ./target/release/detect <path>
```

The `recipe` binary's other subcommands:

```bash
recipe train.rs    # compile the script against librecipe, cache the binary, run it
recipe probe       # measure this machine (arch, GPUs, VRAM, RAM)
recipe serve       # training daemon, listens on 7845
recipe peers       # live view of discoverable peers
```

A training script is plain Rust against the builder API:

```rust
use recipe::{Data, Model, Train, Loss, R2, mse};

fn main() {
      let data  = Data::load("train.csv").split(0.8).exclude("Id").target("SalePrice");
      let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.0001);
      Train::new().epochs(100).log([Loss, R2]).run(&data, &model).save(());
}
```

Examples:

```bash
cargo run --release --example cookbook              # the e2e: NN/CNN/MLP/LLM scenarios
cargo run --release --example styles                # same model through all three import styles
cargo run --release --example train_detector        # retrain the column-type detector, writes pantry/detector.ogdl
cargo run --release --example gemma4 -- "prompt"    # gemma-26B f64 inference
```

One GPU process at a time. Concurrent runs OOM at weight init.

## Workspace

| Crate | Role |
|-------|------|
| `gpu-core` | f64 HIP kernels, HIP/ROCm (plus CUDA shim) bindings, tagged memory ledger. Links ROCm. Depends on nothing GPU-side. |
| `recipe-infer` | Forward pass as pure tensor fns (weights + input matrix in, output matrix out), ogdl/safetensors loading, owns GPU device lifecycle. Knows nothing of datasets. |
| `pantry` | ALL parsing (csv/arff/zip/image dirs), encoding, the single NaN policy, trained column-type detector, ships `detect`. |
| `recipe` | Builder API (`Data`/`Model`/`Train`), backward, fit (in-VRAM and out-of-core), save/resume, TUI, eval. Delegates loading to `pantry`. |
| `vramspy` | `LD_PRELOAD` cdylib interposing HSA alloc entry points; counts library-side VRAM beneath the ledger choke points. |
| `ogdl`, `log` | Leaf utility crates (tree-graph format; stderr/run-file logging). |
| `catboost-rs`, `xgboost-rs`, `lightgbm-rs` | Standalone GBDT crates on `gpu-core`. |

Dependency chain for the NN path: `gpu-core`, then `recipe-infer`, then `pantry`, then `recipe`. Strict one-way, no cycles.

## Rules that break your build if you don't know them

- f64 only. No fp32/fp16/bf16/mixed precision anywhere.
- Full-batch only. No `batch_size`, no mini-batch, no gradient accumulation.
- No vendor math libs (rocBLAS/hipBLAS/cuBLAS etc.) in new code; the remaining `hipblas*` calls in kernels.rs are debt under migration, never add one.
- Exactly one call site per HIP memory API, in `gpu-core/src/memory.rs`. Never a raw `hip::` memory call anywhere else; every byte goes through the ledger.
- `hipMallocAsync`/`hipFreeAsync` only. The sync variants are rejected at compile time by the root `build.rs` scanner.
- NaN handling lives in one place (`pantry::encode::nan_clean`). Do not scatter new NaN checks.
- No caps or magic thresholds to "fix" size or speed: accelerate the code or fail clean with the size and culprit columns printed.
- Comment budget, enforced by test: `///` doc comments unlimited, `// SAFETY:` max 1 per unsafe block, any other `//` max 1 per function. No `/* */`.
- 6-space indentation, `anyhow::Result`, no `unwrap`, lowercase const aliases (`mse`, `w`, `b`). Progress and diagnostics go to stderr, never stdout.

## Debugging

- rocprofv3 hangs at teardown. Always `timeout 8 rocprofv3 --hip-trace --kernel-trace -d <dir> -- <bin>`; the SQLite DB survives the kill (query `rocpd_kernel_dispatch`; workgroups come from grid/workgroup dims).
- GPU crash forensics: `coredumpctl list` / `coredumpctl info <PID>` for the backtrace, `journalctl -k` for the amdgpu page-fault vs OOM truth. Inspect the core before rebuilding over the binary.
- A duplicate-symbol link error out of gpu-core is always a genuine duplicate `extern "C"` definition across two `.hip` sources (the build sweeps stale `.o`/`libhip*.a` first, so it is never staleness). Delete the redundant copy.
- FFI parity is the silent killer: C linkage matches names only. Check per-slot type parity between every `.hip` launcher and its Rust decl.
- rocPRIM in a `.hip` file needs `#include <rocprim/rocprim.hpp>` AND `#include <cstring>`; device temp is caller-owned (`*_workspace_bytes` query plus a launcher that takes tmp). See `reduce.hip`.
- Detector retrain: on a corpus change delete `pantry/detector.ogdl` first (the best-only save guard blocks incomparable scores), then rebuild after training (`include_str!` bakes the weights at compile time).
