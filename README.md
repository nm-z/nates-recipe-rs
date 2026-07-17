# recipe

GPU-native f64 NN training/inference, own HIP kernels, full-batch only. Root crate is the builder API (`Data`/`Model`/`Train`), backward, fit, save/resume, TUI, eval.

```bash
GIT_LFS_SKIP_SMUDGE=1 git clone https://github.com/nm-z/nates-recipe-rs   # source only; plain clone smudges ~3.3 GiB of datasets

cargo build --release                              # whole workspace; needs hipconfig on PATH; GPU_ARCH=gfx1101 default
cargo test all                                     # THE suite: every test in every crate, 60s SIGKILL each, log at suite.log
rg '^FAIL' suite.log                               # suite verdict
rm target/.suite_cache                             # force full re-run (suite skips fn-body-hash matches)

cargo run --release -- train.csv --target Price    # train straight from a csv
cargo run --release -- detect <path>               # column-type detection (csv / arff / dir / zip)

cargo run --release --example cookbook             # the e2e: NN/CNN/MLP/LLM scenarios
cargo run --release --example styles               # same model through all three import styles
cargo run --release --example train_detector       # retrain column-type detector; delete pantry/detector.ogdl first, rebuild after
cargo run --release --example gemma4 -- "prompt"   # gemma-26B f64 inference
cargo run --release --example det_probe            # determinism probe: every diffusion-forward op run twice, bit-compared
cargo run --release --example gpu_probe            # proof-of-life sgemm: correctness vs CPU ref, then f32 TFLOP/s
cargo run --release --example stress               # train over real datasets (bank/seeds/wine), delimiter sniffing included

cargo test --release --test dataset                # root tests standalone (all are members of `cargo test all`):
cargo test --release --test model                  #   likewise ooc, probe, wire, hygiene

recipe train.rs                                    # compile a script against librecipe, cache, run
recipe probe                                       # measure this machine (arch, GPUs, VRAM, RAM)
recipe serve                                       # training daemon on 7845
recipe peers                                       # live view of discoverable peers
```

One GPU process at a time; concurrent runs OOM at weight init.

## gpu-core

f64 `.hip` kernels, HIP/ROCm bindings, tagged memory ledger. Everything GPU sits on this.

```bash
cargo test -p gpu-core --release --test suite      # kernel proof suite (target named `suite`, not `all`; needs committed kernel_inventory.db)
cargo test -p gpu-core --release --test diffusion  # likewise moe, rope, tiered

cargo run --release -p gpu-core --bin probe        # raw device probe through gpu-core's own HIP bindings
cargo run --release -p gpu-core --bin asmrun       # load a .s/.hsaco and dispatch a kernel: asmrun <path.s> [kernel gridX blockX arg..]
```

## recipe-infer

Forward pass as pure tensor fns, ogdl/safetensors loading, owns GPU device lifecycle. Knows nothing of datasets.

```bash
cargo test -p recipe-infer --release               # forward / KV-cache / ogdl behavioral tests (GPU)
cargo test -p recipe-infer --release --test forward   # likewise gguf, safetensors, params, arch_dispatch, logit_ref, tokenizer_ref

cargo test --release -p recipe-infer --test archs_parity  -- --nocapture   # forward parity, GPU
cargo test --release -p recipe-infer --test vocabs_parity -- --nocapture   # tokenizer parity, CPU
# each prints one colored verdict row per model/vocab, then a summary line;
# fails until full parity (red by design)
```

## pantry

ALL parsing (csv/arff/zip/image dirs), encoding, the single NaN policy, trained column-type detector.

```bash
cargo build --release -p pantry                    # ships target/release/detect (standalone binary)
./target/release/detect <path>...
cargo test -p pantry --release --test data         # likewise encode
```

## vramspy

`LD_PRELOAD` cdylib interposing HSA alloc entry points; counts library-side VRAM beneath the ledger choke points.

```bash
cargo build --release -p vramspy
LD_PRELOAD=target/release/libvramspy.so <bin>
```

## catboost-rs, xgboost-rs, lightgbm-rs

Standalone GBDT trainers on `gpu-core` (ordered boosting; level-wise histogram trees; leaf-wise GOSS/EFB).

```bash
cargo build --release -p catboost-rs               # likewise xgboost-rs, lightgbm-rs
cargo run --release -p catboost-rs --bin bench     # train+predict bench on a csv; each crate ships its own `bench` bin
```

## ogdl, log

Leaf utility crates: OGDL tree graphs (weights/config format); flag-gated stderr and run-file logging.

```bash
cargo test -p ogdl --release --test corpus         # likewise doc, fill, full_probe, graph, host, path, reconcile, styles
```

`log` has no targets of its own; everything above is also a member of `cargo test all`.
