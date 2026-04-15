# nates-recipe-rs

This repository combines:

- a Rust GPU runtime (`gpu-core`)
- Rust-hosted scripting entrypoints (Lua and Ruby bridges)
- model/pipeline scripts for Kaggle-style workflows
- a local CatBoost implementation crate

## Repository map and responsibilities

```text
nates-recipe-rs/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── lua_runtime.rs
│   └── utils/
│       ├── data.rs
│       └── tests.rs
├── gpu-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── hip.rs
│       ├── memory.rs
│       ├── kernels.rs
│       └── kernels/
├── catboost-rs/
│   ├── Cargo.toml
│   └── src/lib.rs
├── nates-gpu-ruby/
│   ├── Cargo.toml
│   └── src/lib.rs
├── nates-gpu-lua/
│   ├── Cargo.toml
│   └── src/lib.rs
├── kaggle_s6e4/
└── models/
```

### What each major part does

- `src/main.rs`: command-line entrypoint that loads and runs a Lua script.
- `src/lua_runtime.rs`: registers GPU-backed tensor/buffer operations into Lua globals (upload/download, BLAS ops, activations, reductions, etc).
- `src/utils/data.rs`: data loading helpers (CSV via Polars, frequency encoding for categorical strings, train/test split, image folder loaders).

- `gpu-core/src/hip.rs`: raw HIP FFI bindings and error wrappers.
- `gpu-core/src/memory.rs`: GPU buffer allocation, upload/download, allocation counting, and spill behavior.
- `gpu-core/src/kernels.rs`: Rust launcher/FFI layer that wires high-level operations to rocBLAS/rocSOLVER and custom HIP kernels.
- `gpu-core/src/kernels/*.hip`: kernel implementations.

- `nates-gpu-ruby/src/lib.rs`: Ruby extension exposing GPU operations to Ruby and bridging CatBoost training/prediction.
- `nates-gpu-lua/src/lib.rs`: Lua module exposing GPU operations for Lua-based scripts.

- `catboost-rs/src/lib.rs`: local CatBoost implementation (ordered target statistics, oblivious trees, training and prediction APIs).

## Kernel locations

```text
gpu-core/src/kernels/
├── elementwise.hip   # elementwise math and activation primitives
├── argsort.hip       # partial sort/top-k/arg helpers
├── reduce.hip        # reductions (sum/mean/var/max/min/log-sum-exp)
├── distance.hip      # distance and nearest-neighbor related primitives
├── tree.hip          # histogram/split/tree-building kernels
├── dtw.hip           # dynamic time warping
└── apriori.hip       # association-rule support/candidate generation
```

## Data locations

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

## CatBoost / XGBoost / LightGBM paths

### CatBoost

- Implementation crate: `catboost-rs/src/lib.rs`
  - parameter struct (`Params`)
  - training entrypoint (`train`)
  - prediction entrypoint (`predict`)
  - quantization, ordered target stats, and oblivious-tree building internals
- Ruby bridge bindings: `nates-gpu-ruby/src/lib.rs`
  - `catboost_train(...)`
  - `catboost_predict(...)`
- Kaggle pipeline usage: `kaggle_s6e4/solve_v2.rb` via `train_cb_gpu(...)`.

### XGBoost

- Used in Ruby pipeline (`kaggle_s6e4/solve_v2.rb`) via `require "xgb"`.
- Primary wrapper function: `train_xgb(...)`.
- Flow in that function:
  - build `XGBoost::DMatrix`
  - call `XGBoost.train(...)` with config + optional early stopping
  - run prediction on validation/test matrix

### LightGBM

- Used in Ruby pipeline (`kaggle_s6e4/solve_v2.rb`) via `require "lightgbm"`.
- Primary wrapper function: `train_lgbm(...)`.
- Flow in that function:
  - build `LightGBM::Dataset`
  - call `LightGBM.train(...)` with config + optional early stopping
  - run prediction on validation/test features
