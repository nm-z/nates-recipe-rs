# nates-recipe-rs (what is actually novel here)

This repo is a mixed Rust + Ruby + Lua ML/GPU playground with a custom GPU core, language bridges, and Kaggle-focused pipelines.

The non-boring parts are:

- A custom `gpu-core` crate with HIP/rocBLAS/rocSOLVER bindings plus many custom `.hip` kernels.
- Language bridges:
  - Rust binary + Lua runtime (`src/lua_runtime.rs`)
  - Rust `cdylib` for Ruby (`nates-gpu-ruby`) exposing GPU ops and CatBoost bridge functions
- A local CatBoost implementation crate (`catboost-rs`) wired into Ruby.
- Kaggle pipeline scripts (`kaggle_s6e4/solve_v2.rb`) that run ensembles including XGBoost, LightGBM, CatBoost.

## Where things are

### Crates / modules tree

```text
nates-recipe-rs/
├── Cargo.toml                 # root crate: nates_recipe
├── src/
│   ├── main.rs                # CLI: runs Lua model scripts
│   ├── lib.rs                 # exports gpu-core + lua runtime
│   ├── lua_runtime.rs         # Lua<->GPU bridge functions
│   └── utils/
│       ├── data.rs
│       └── tests.rs
├── gpu-core/                  # core GPU crate (HIP + kernels)
│   ├── Cargo.toml
│   └── src/
│       ├── hip.rs
│       ├── memory.rs
│       ├── kernels.rs
│       └── kernels/*.hip
├── catboost-rs/               # local CatBoost implementation crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── nates-gpu-ruby/            # Ruby native extension (cdylib)
│   ├── Cargo.toml
│   └── src/lib.rs
└── nates-gpu-lua/             # Lua native module (cdylib)
    ├── Cargo.toml
    └── src/lib.rs
```

### Kernel sources

```text
gpu-core/src/kernels/
├── elementwise.hip
├── argsort.hip
├── reduce.hip
├── distance.hip
├── tree.hip
├── dtw.hip
└── apriori.hip
```

`gpu-core/src/kernels.rs` is the Rust FFI/launcher layer for these kernels plus rocBLAS/rocSOLVER calls.

### Data locations

```text
data/
└── arc/
    ├── arc-agi_training_challenges.json
    ├── arc-agi_training_solutions.json
    ├── arc-agi_test_challenges.json
    ├── arc-agi_evaluation_challenges.json
    ├── arc-agi_evaluation_solutions.json
    └── sample_submission.json

kaggle_s6e4/
├── train.csv
├── test.csv
├── sample_submission.csv
└── irrigation_prediction.csv

models/
├── X_vna2.csv
└── y_vna2.csv
```

## Especially: CatBoost / XGBoost / LightGBM

### CatBoost

- Local implementation crate: `catboost-rs/src/lib.rs`
  - Ordered boosting logic
  - Categorical handling + target statistics
  - Oblivious tree building and prediction
- Ruby bridge exposure: `nates-gpu-ruby/src/lib.rs`
  - `catboost_train(...)`
  - `catboost_predict(...)`
- Kaggle usage point: `kaggle_s6e4/solve_v2.rb` in `train_cb_gpu(...)`.

### XGBoost

- Used from Ruby in `kaggle_s6e4/solve_v2.rb`.
- Imported via `require "xgb"`.
- Training path: `train_xgb(...)` (builds `XGBoost::DMatrix`, calls `XGBoost.train`, predicts for fold/test).
- No direct Rust XGBoost bridge is used in the root crate right now.

### LightGBM

- Used from Ruby in `kaggle_s6e4/solve_v2.rb`.
- Imported via `require "lightgbm"`.
- Training path: `train_lgbm(...)` (builds `LightGBM::Dataset`, calls `LightGBM.train`, predicts for fold/test).
- No direct Rust LightGBM bridge is used in the root crate right now.

## Build/test caveat in this environment

`cargo test` currently fails here because `gpu-core` expects `hipcc` to exist at build time.
