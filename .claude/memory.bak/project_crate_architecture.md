---
name: project_crate_architecture
description: The shipped 4-crate workspace layout and one-way dependency DAG (gpu-core / recipe-infer / pantry / nates-recipe)
metadata: 
  node_type: memory
  type: project
  originSessionId: 2cdc1e43-ae8c-445a-9556-af38f0a80ef8
---

As of 2026-06-23 (committed 4be70c4, pushed to origin/master) the repo is a **4-crate workspace** with a strict one-way dependency DAG. This superseded the old single-crate `nates-recipe`. See [[project_detector_endstate.md]] for the phase-by-phase how.

```
gpu-core                          HIP kernels, links ROCm, depends on nothing
recipe-infer → gpu-core           forward pass + ogdl load. PURE TENSOR FNS: weights + input
                                  matrix → output matrix. owns GPU device lifecycle
                                  (recipe_infer::init() = set_device(0); recipe_infer::shutdown()).
                                  knows NOTHING of Datasets/columns/where data came from.
pantry       → recipe-infer       ALL data parsing (csv/arff/zip/dir loaders, src/data.rs) +
                                  column-type detection (src/detect.rs: predict_kinds, tokenize_column,
                                  Kind, embedded pantry/detector.ogdl) + standalone `detect` binary
                                  (src/main.rs, [[bin]] name="detect"). NO training. deps recipe-infer ONLY.
nates-recipe → gpu-core,          Model/Train builder, backward, fit, save/resume, TUI, eval, preflight.
               recipe-infer,        Data builder delegates loading+detection to pantry; forward via
               pantry               recipe-infer. Holds the detector TRAINER (src/utils/detect.rs).
catboost-rs / lightgbm-rs / xgboost-rs   workspace-excluded, standalone, untouched.
```

**File homes (where things moved):**
- recipe-infer/src: `enums.rs` (Activation/LayerSpec/LayerKind/Loss/Metric + user consts mse/ce/R2/…), `params.rs` (Saved/LayerParams/Scaler/build_layer_params/sinusoidal_pe/concat_layer/pinned_vocab), `scratch.rs` (Scratch + vram_estimate), `forward.rs` (forward_into, attn_forward(_cached), metric_gpu(_into), upload/zscore/nan_impute/download), `ogdl.rs` (load_ogdl(_str)), `lib.rs` (init/shutdown/human_bytes + re-exports). See [[project_scratch_ping_pong.md]].
- pantry/src: `data.rs` (was nates-recipe src/utils/data.rs — moved wholesale), `detect.rs` (detector INFERENCE half), `lib.rs` (Mat/Vec1 aliases, Kind enum, Attr struct, available_ram_bytes), `main.rs` (detect binary). `pantry/detector.ogdl` (weights, include_str'd).
- nates-recipe src/utils: `model.rs` (Model/Train builder + preflight), `train.rs` (backward_step/fit/eval), `dataset.rs` (Data/Dataset, delegates to pantry), `detect.rs` (detector TRAINER only: model() builder, SOURCES/MARCH, corpus_split, instances, build_dataset).

**Invariants:** builder API unchanged (`nates_recipe::*` re-exports recipe_infer enums + `pub use pantry::data`); examples compile unedited (the canary). recipe-infer must never reference Dataset/Model/Train/nates_recipe (grep-enforced). The Dataset→Mat conversion is the seam between "understanding data" (pantry) and "running math" (recipe-infer) — `collapse_onehot` stays up in nates-recipe.

**Verify commands:** `cargo build` (workspace), `cargo test --workspace` (root `cargo test` only tests nates-recipe!), `cargo test -p recipe-infer --release` (forward/KV-cache/ogdl), `./target/release/detect <path>` (standalone). [[feedback_test_output_visible.md]] — one GPU process at a time.
