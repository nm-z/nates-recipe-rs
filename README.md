# recipe

**ML training + inference for AMD/NVIDIA GPUs.**

`recipe` is a GPU-native neural-network framework written in Rust. Every arithmetic operation runs in `f64` on the GPU through the framework's own HIP/CUDA kernels — there is no vendor BLAS on the training or inference hot path, and no CPU ML crate does the compute. You describe a model with a chained builder API, point it at a dataset (CSV, ARFF, zip, or a directory of images), and train full-batch on the GPU. AMD RDNA3 is the primary target; an NVIDIA backend shares the same kernels. When a dataset is larger than VRAM, training streams it out-of-core (VRAM to RAM to disk) instead of failing, and the math stays full-batch throughout.

## Features

- **f64 end to end.** No fp32/fp16/bf16/mixed precision anywhere — all compute is double precision on the GPU.
- **Own GPU kernels.** All GPU math runs through hand-written HIP/CUDA compute kernels (`gpu-core`); no rocBLAS/hipBLAS/cuBLAS on the hot path.
- **AMD-first, NVIDIA-capable.** Built for AMD RDNA3 via HIP/ROCm, with a CUDA backend over the same kernel set.
- **Full-batch training.** No mini-batch SGD, no gradient accumulation — the whole dataset is one GEMM shape.
- **Out-of-core streaming.** Datasets bigger than VRAM stream VRAM→RAM→DISK with read-ahead/write-behind; the math stays full-batch.
- **One builder, three import styles.** `Model` / `Data` / `Train` / `Infer` chain the same methods whether you reach them as `recipe.model()`, `Model::new()`, or `recipe::model()`.
- **Model types in one API.** Dense nets, 1-D convolutional nets (`conv(filters, kernel, stride)`), MLPs, and transformer/LLM stacks (`layer(embed(dim))`, `layer(attn(heads))`) — see `examples/cookbook.rs` for NN, CNN, MLP, and LLM scenarios end to end.
- **Activations as chained methods.** `.relu() .leak() .gelu() .elu() .sigmoid()` and more, applied directly after a `.layer(n)`.
- **Losses & metrics as consts.** Losses `mse mae huber ce bce focal`; log metrics `Loss Accuracy R2 Lr Epoch Time hip`.
- **Data loading built in (`pantry`).** CSV / ARFF / zip / image-directory parsing, one-adapter NaN policy, feature hashing for text/high-cardinality columns, and a trained char-level column-type detector (`detect` binary).
- **Weight I/O.** Loads and saves weights in the project's `ogdl` format and reads `safetensors` for inference (`recipe-infer`).
- **Multi-node training.** `Train::net([...])` distributes a run across configured peer machines.
- **Tagged memory ledger.** Every GPU allocation, free, and transfer is tagged and accounted through a single set of choke points in `gpu-core`.
- **Save / resume.** `.run(&data, &model).save(path)` persists a model; `Train::resume(path)` continues a run, crashing with a diagnostic on a shape mismatch rather than silently reinitializing.

## Install

### Prerequisites

- **Rust nightly** — pinned by `rust-toolchain.toml` (`channel = "nightly"`, components `rustfmt`, `clippy`, `rust-src`); `rustup` selects it automatically inside the repo. Edition 2024, resolver 3.
- **A HIP runtime with `hipconfig` on `PATH`.** The backend is detected at build time from `hipconfig --platform`, and the ROCm tree is located via `hipconfig --rocmpath`. If `hipconfig` is missing or reports an unrecognized platform, the build fails immediately with the package to install:

  ```
  hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia
  ```

  - **AMD:** a ROCm install providing `hipcc`/`amdclang++`, `amdhip64`, and `hipblas`/`hipsolver`/`hipfft`.
  - **NVIDIA:** the HIP-on-CUDA runtime plus a CUDA toolkit (`nvcc`, `cudart`/`cublas`/`cusolver`/`cufft`); HIP's `hipblas`/`hipsolver`/`hipfft` wrap the CUDA libraries.
- **Git LFS** — datasets under `datasets/` are stored as LFS objects (`.gitattributes`: `datasets/** filter=lfs diff=lfs merge=lfs -text`).

### Clone

A plain clone smudges the full dataset tree (~3.3 GiB):

```bash
git clone https://github.com/nm-z/nates-recipe-rs
```

For a source-only checkout (compute/serve nodes that don't need the datasets), skip the LFS smudge — this leaves ~110 MB of pointer files instead:

```bash
GIT_LFS_SKIP_SMUDGE=1 git clone https://github.com/nm-z/nates-recipe-rs
```

### Build

```bash
cargo build --release
```

The release profile uses `lto = "thin"` and links the detected GPU backend. Kernels in `gpu-core/src/kernels/*.hip` are compiled through the `cc` crate with `hipcc` (or `nvcc` on the NVIDIA path). The build produces two binaries:

- `target/release/recipe` — the training/eval/inference CLI
- `target/release/detect` — the standalone column-type detector

### Environment overrides

The build reads the ROCm root from `hipconfig --rocmpath`; the variables below override individual pieces (each shows its default; `<rocm>` and `<cuda>` are the resolved ROCm/CUDA roots).

| Variable | Default | Effect |
| --- | --- | --- |
| `GPU_ARCH` | `gfx1101` | AMD `--offload-arch` target passed to the kernel compiler |
| `HIPCC` | `<rocm>/bin/hipcc` (falls back to `<rocm>/bin/amdclang++` if absent) | Compiler used for `.hip` kernels |
| `ROCM_EXTRA_INCLUDE` | `<rocm>/include` | Extra `-I` include dir for AMD kernel compilation |
| `ROCM_EXTRA_LIB` | `<rocm>/lib` | Extra link-search dir |
| `CUDA_PATH` | `/opt/cuda` | CUDA toolkit root (NVIDIA backend) |
| `CUDA_ARCH` | `sm_86` | NVIDIA `-arch` target |
| `NVCC` | `<cuda>/bin/nvcc` | CUDA compiler (NVIDIA backend) |
| `ROCM_PATH` | `/opt/rocm` | Runtime only: ROCm `lib` search path used when the CLI JIT-compiles a Rust script target |

No override installs a runtime for you — a missing or undecided HIP backend is always a hard build failure naming the package, never a silent fallback.

## Usage

### Three import styles

One crate, one set of methods — only the door differs. Past the constructor, the chaining, the `let` bindings, `train().run(&data, &model)`, and `infer().run(&model).eval(&data)` are identical. Each block below builds the same model on the same data.

```rust
use gpu_core::log::{loss, r2};
use recipe::{Data, Infer, Loss, Model, R2, Train, mse, recipe};
```

**Style 1 — static (`recipe.data(...)`, dot syntax):**

```rust
let data  = recipe.data("train.csv").split(0.8).exclude("Id").target("SalePrice");
let model = recipe.model().layer(64).leak().layer(1).loss(mse).lr(0.0001);
recipe.train().epochs(5).log([Loss, R2]).run(&data, &model);
recipe.infer().log([r2]).run(&model).eval(&data);
```

**Style 2 — struct (`Data::load(...)`, associated function):**

```rust
let data  = Data::load("train.csv").split(0.8).exclude("Id").target("SalePrice");
let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.0001);
Train::new().epochs(5).log([Loss, R2]).run(&data, &model).save("model.ogdl");
Infer::new().log([r2]).run(&model).eval(&data);
```

**Style 3 — free function (`recipe::data(...)`, crate path):**

```rust
let data  = recipe::data("train.csv").split(0.8).exclude("Id").target("SalePrice");
let model = recipe::model().layer(64).leak().layer(1).loss(mse).lr(0.0001);
recipe::train().epochs(5).log_every(1).log([Loss, R2]).run(&data, &model).save(());
recipe::infer().log([r2]).run(&model).eval(&data);
```

`recipe` is a unit static, so `recipe.data(...)` and `recipe::data(...)` coexist in the same scope.

- `Data`: `Data::load(path)` then `.set(...)`, `.split(f)`, `.exclude(col)`, `.target(col)` (or `.target([col, ...])` for multi-target), `.datasets()`. Loading is lazy; `.target()` describes, `run` materializes.
- `Model`: `.layer(n)` with a chained activation (`.relu()`, `.leak()`, `.gelu()`, `.sigmoid()`, …), plus `.layer(embed(dim))`, `.layer(attn(heads))`, `.conv(filters, kernel, stride)`, and `.loss(...)` / `.lr(...)`. Losses: `mse mae huber ce bce focal`.
- `Train`: `.epochs(n)`, `.log_every(n)`, `.log([...])`, `.net([...])`, `.resume(path)`; `.run(&data, &model)` returns `&Train` so `.save(path)` chains off it. `.save(())` writes `model.ogdl`; `.save("path.ogdl")` writes an explicit path. Log metrics: `Loss Accuracy R2 Lr Epoch Time hip`.
- `Infer`: forward-only; `.run(&model).eval(&data)`.

Runnable end-to-end versions of the above live in `examples/styles.rs` and `examples/cookbook.rs`:

```bash
cargo run --release --example styles
cargo run --release --example cookbook
```

### Command line

The two binaries built above expose the command-line surface.

**`recipe`** runs a training script and exposes the network/probe subcommands:

```bash
recipe train.rs            # compile the script against librecipe and run it
recipe probe               # measure this machine (arch, GPUs, VRAM, RAM)
recipe serve               # run the training daemon (listens on port 7845)
recipe peers               # live network view of discoverable peers
recipe -h                  # usage
```

A training run is a Rust script that uses the builder API above. Write it once, then hand it to `recipe`, which compiles it against the installed library, caches the compiled binary, and executes it:

```rust
// train.rs
use recipe::{Data, Model, Train, Loss, R2, mse};

fn main() {
      let data  = Data::load("train.csv").split(0.8).exclude("Id").target("SalePrice");
      let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.0001);
      Train::new().epochs(100).log([Loss, R2]).run(&data, &model).save(());
}
```

```bash
recipe train.rs
```

**`detect`** is a standalone column-type detector — no training framework linked — that prints each column's inferred datatype (`Numeric`, `Temporal`, `Categorical`, `Ordinal`, `Text`, or `Image`):

```bash
detect <path>...           # csv / arff / directory / zip; globs expand to many
```

```text
SalePrice -> Numeric
Neighborhood -> Categorical
YearBuilt -> Temporal
```

## Architecture

`recipe` is a Cargo workspace. The root crate (`.`) is `recipe`; the other members are listed in `Cargo.toml`:

| Crate | Role |
|-------|------|
| `gpu-core` | GPU-native f64 compute: the `.hip` kernels, the HIP/ROCm (and CUDA-shim) bindings, and the tagged memory ledger. Links ROCm. |
| `recipe-infer` | Forward-pass inference as pure tensor functions (weights + input matrix produce an output matrix), `ogdl`/safetensors weight loading, and the GPU device lifecycle (init/shutdown). Knows nothing of datasets. |
| `pantry` | All dataset parsing (CSV, ARFF, zip, image directories), encoding, the NaN policy, and a trained char-level column-type detector. Ships the `detect` binary. |
| `recipe` | The user-facing builder API: `Data`/`Model`/`Train`, backward pass, in-VRAM and out-of-core `fit`, save/resume, eval, and the TUI. Delegates loading to `pantry`. |
| `vramspy` | `LD_PRELOAD` cdylib that interposes HSA allocation entry points to count library-side VRAM by pool-owning agent, beneath the ledger's choke points. |
| `log` | Flag-gated stderr and run-file logging with self-erasing terminal wait lines. |
| `ogdl` | OGDL tree graphs: dotted-path selection and tab-indented serialization, used for weights and config. |
| `catboost-rs`, `xgboost-rs`, `lightgbm-rs` | Standalone gradient-boosted-decision-tree trainers (ordered boosting; level-wise histogram trees; leaf-wise GOSS/EFB), each on `gpu-core`. |

### Dependency DAG

Path dependencies form a strict one-way graph — no cycles. `ogdl` and `log` are leaf utility crates; `gpu-core` sits above them and everything GPU sits above `gpu-core`.

```text
ogdl ── log
  │      │
  └──► gpu-core ──► recipe-infer ──► pantry
          │              │             │
          │              └──────┐      │
          ▼                     ▼      ▼
   catboost-rs                    recipe
   xgboost-rs        (depends on gpu-core, recipe-infer, pantry, ogdl)
   lightgbm-rs

vramspy ── log        (standalone cdylib; not on the gpu-core path)
```

The neural-network chain is `gpu-core → recipe-infer → pantry → recipe`: kernels know nothing of inference, inference knows nothing of datasets, and `recipe` composes all three. The three GBDT crates depend only on `gpu-core`. `vramspy` is built separately as an interposer and links only `log`.

### Design commitments

These are structural, not tunables:

- **f64 only.** Every tensor and kernel is double precision — no fp32/fp16/bf16 or mixed precision anywhere.
- **Full-batch only.** There is no `batch_size`, mini-batch SGD, or gradient accumulation; the whole dataset is one matrix per step.
- **One call site per HIP memory API.** Alloc, free, transfer, and memset each funnel through exactly one choke point in `gpu-core/src/memory.rs`, where every byte is tagged and ledgered (live and peak per tag; cumulative H2D/D2H/D2D bytes and call counts). No raw HIP memory call exists elsewhere.
- **Waterfall placement.** VRAM fills completely before anything spills to RAM, and RAM before disk. Fit decisions go through `gpu_core::memory::claimable_bytes()`, never a raw device query, with a fixed per-tier reserve as the only headroom constant.

## Data

Loading and encoding live in the `pantry` crate. `pantry::load_groups(path)` dispatches on the path and returns typed table/image groups; the trainer above it consumes the encoded matrix and never touches parsing.

### Formats

- **CSV / delimited text** — the delimiter is sniffed per file (comma, semicolon, tab, or whitespace). The header row is decided structurally, not heuristically: if every cell of the first row parses as `f64` the row is data and columns are named `col_0..col_N`; otherwise the first row is the header.
- **ARFF** — `@attribute` types are read directly (`{a,b,c}` nominal sets become categorical, everything else numeric); `@data` rows follow.
- **Zip archives** — extracted to a temp directory (nested zips are unpacked recursively), then loaded as a directory.
- **Image directories** — each image becomes one RGB pixel row (`width * height * 3`) at the images' native resolution. A directory of images is a single matrix keyed by filename stem, joined to table rows that reference those filenames; class-labeled subdirectories are supported for labeled image sets.

Directories may mix tables and images; groups sharing a key column are joined on it.

### Column-type detection

Column types are inferred by a trained char-level model rather than hand-written rules. Each column's cells are read as a newline-joined byte stream (up to 256 bytes), and a fixed `embed → attention → dense → dense` network classifies the stream into one of six kinds: **Numeric, Temporal, Categorical, Ordinal, Text, Image**. Classification uses only structural signal from the raw bytes — no magic thresholds. The trained weights ship inline (`pantry/detector.ogdl`) and the forward pass runs on the GPU. These inferred types drive the encoding described next, and the standalone `detect` binary (see [Command line](#command-line)) surfaces them directly.

### Encoding

- **Numeric / Temporal** → one column (dates parsed to a day count).
- **Categorical** → one-hot, one column per distinct value.
- **Ordinal** → a single integer-coded column.
- **Text** → tokenized by splitting on non-alphanumeric characters and lowercasing; a sorted per-column vocabulary maps each row to a fixed-length sequence of token IDs (`0` = unknown/pad) for the embedding layer. The sequence length is bounded to 256 tokens, a no-op for ordinary short text and a truncation only for long-form outliers.
- **Image** columns hold filenames and act as join keys into the image matrix, contributing no feature columns themselves.

Cardinality is never capped to fit. If the encoded matrix exceeds the combined VRAM + RAM + disk ceiling, the run stops with a one-line autopsy naming the widest columns instead of silently shrinking the data.

### Missing values

One policy, one function (`nan_clean` applied through `clean_dataset`). Common missing markers — `""`, `NA`, `NaN`, `N/A`, `NULL`, `None`, `?`, `.`, `-` — are recognized on parse. Feature columns are **mean-imputed** (each NaN filled with that column's finite mean); rows whose **target** is missing are **dropped**, since a label cannot be invented. After this single pass the matrix contains no NaN, so nothing downstream re-handles missingness.

## License

Released under the MIT License.
