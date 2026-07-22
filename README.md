# recipe

GPU-native f64 NN training/inference, own HIP kernels, full-batch only. Root crate is the builder API (`Data`/`Model`/`Train`), backward, fit, save/resume, TUI, eval.

Source-only clone; a plain clone smudges ~3.3 GiB of LFS datasets:

```bash
GIT_LFS_SKIP_SMUDGE=1 git clone https://github.com/nm-z/nates-recipe-rs
```

Build the whole workspace. Needs `hipconfig` on `PATH`; default GPU arch is gfx1101, override with `GPU_ARCH`:

```bash
cargo build --release
```

THE suite: every test in every crate, one OS process per test, 60s SIGKILL each, one log at `suite.log`. PASS green, FAIL red, `[S]` status cyan; FAIL details show only the test's own output plus the panic (libtest ceremony is scrubbed). The second line is the verdict; the third forces a full re-run (the suite skips tests whose fn-body hash matches the cache):

```bash
cargo test
rg '^FAIL' suite.log
rm target/.suite_cache
```

Filter to matching test ids (`cargo test all` also still works, unfiltered). Any positional filter makes cargo run the root lib's empty 0-test harness too; that extra block is a cargo quirk, not the suite:

```bash
cargo test <substring>
```

Train straight from a csv:

```bash
cargo run --release -- train.csv --target Price
```

Column-type detection (csv / arff / directory / zip):

```bash
cargo run --release -- detect <path>
```

Examples. `cookbook` is the e2e (NN/CNN/MLP/LLM scenarios). `styles` builds the same model through all three import styles. `train_detector` retrains the column-type detector; delete `pantry/detector.ogdl` first and rebuild after, since `include_str!` bakes the weights at compile time. `gemma4` is gemma-26B f64 inference. `det_probe` runs every diffusion-forward op twice and bit-compares for determinism. `gpu_probe` is a proof-of-life sgemm with an f32 TFLOP/s measure. `stress` trains over real datasets (bank/seeds/wine):

```bash
cargo run --release --example cookbook
cargo run --release --example styles
cargo run --release --example train_detector
cargo run --release --example gemma4 -- "prompt"
cargo run --release --example det_probe
cargo run --release --example gpu_probe
cargo run --release --example stress
```

Root test targets standalone, all also members of `cargo test all`; same form for `ooc`, `probe`, `wire`, `hygiene`:

```bash
cargo test --release --test dataset
cargo test --release --test model
```

The installed binary. `recipe train.rs` compiles the script against librecipe, caches the binary, and runs it. `probe` measures the machine (arch, GPUs, VRAM, RAM). `serve` is the training daemon on 7845. `peers` is a live view of discoverable peers:

```bash
recipe train.rs
recipe probe
recipe serve
recipe peers
```

One GPU process at a time; concurrent runs OOM at weight init.

## gpu-core

f64 `.hip` kernels, HIP/ROCm bindings, tagged memory ledger. Everything GPU sits on this.

Kernel proof suite; the test target is named `suite`, not `all`, and it needs the committed `recipe.db`:

```bash
cargo test -p gpu-core --release --test suite
```

Other test targets; same form for `moe`, `rope`, `tiered`:

```bash
cargo test -p gpu-core --release --test diffusion
```

Raw device probe through gpu-core's own HIP bindings:

```bash
cargo run --release -p gpu-core --bin probe
```

Load a `.s`/`.hsaco` and dispatch one of its kernels from the command line; path alone lists the kernels:

```bash
cargo run --release -p gpu-core --bin asmrun -- <path.s> [kernel gridX blockX arg..]
```

## recipe-infer

Forward pass as pure tensor fns, ogdl/safetensors loading, owns GPU device lifecycle. Knows nothing of datasets.

All behavioral tests (forward / KV-cache / ogdl, GPU):

```bash
cargo test -p recipe-infer --release
```

Single test targets; same form for `gguf`, `safetensors`, `params`, `arch_dispatch`, `logit_ref`, `tokenizer_ref`:

```bash
cargo test -p recipe-infer --release --test forward
```

The apples-to-apples parity pair: forward parity on GPU, tokenizer parity on CPU. Each prints one colored verdict row per model/vocab, then a summary line, and fails until full parity (red by design):

```bash
cargo test --release -p recipe-infer --test archs_parity  -- --nocapture
cargo test --release -p recipe-infer --test vocabs_parity -- --nocapture
```

## pantry

ALL parsing (csv/arff/zip/image dirs), encoding, the single NaN policy, trained column-type detector.

Build ships the standalone `detect` binary:

```bash
cargo build --release -p pantry
./target/release/detect <path>...
```

Tests; same form for `encode`:

```bash
cargo test -p pantry --release --test data
```

## vramspy

`LD_PRELOAD` cdylib interposing HSA alloc entry points; counts library-side VRAM beneath the ledger choke points.

Build, then interpose under any GPU binary:

```bash
cargo build --release -p vramspy
LD_PRELOAD=target/release/libvramspy.so <bin>
```

## catboost-rs, xgboost-rs-broken, lightgbm-rs

GBDT trainer member crates on `gpu-core` (ordered boosting; level-wise histogram trees; leaf-wise GOSS/EFB).

Build; same form for `xgboost-rs-broken`, `lightgbm-rs`:

```bash
cargo build --release -p catboost-rs
```

Each crate ships its own `bench` bin, a train+predict bench on a csv:

```bash
cargo run --release -p catboost-rs --bin bench
```

## ogdl, log

Leaf utility crates: OGDL tree graphs (weights/config format); flag-gated stderr and run-file logging.

ogdl test targets; same form for `doc`, `fill`, `full_probe`, `graph`, `host`, `path`, `reconcile`, `styles`:

```bash
cargo test -p ogdl --release --test corpus
```

`log` has no targets of its own; everything above is also a member of `cargo test all`.
