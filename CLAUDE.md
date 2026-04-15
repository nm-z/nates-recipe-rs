# CLAUDE.md

## Core Rules
- When debugging issues, NEVER speculate or guess at causes. Always read the actual logs, code, and docs FIRST before suggesting fixes. Do not try multiple random approaches — diagnose systematically.
- When given a clear directive, act on it immediately. Do NOT ask clarifying questions if the answer is already in context, memory, or the codebase. Do NOT propose multiple options when the choice is already made.
- Always TEST changes before reporting them as done. Run the build, run the tests, verify the output. Do not claim something works without evidence.
- When corrected, accept the correction immediately and move forward. Do not re-explain original reasoning or contradict the correction.

## Code Changes
- Do NOT make excessive or scattered changes across many files. Keep changes minimal and focused on what was requested. If a fix touches more than 3 files, pause and explain why before proceeding.

## Project Context
- This project primarily uses Rust. Secondary tools: Ruby, Shell/Makefile, Markdown. When working on the video pipeline (mvfx), understand that it involves Vulkan, VA-API, DMA-BUF, Wayland, and frame interpolation at 4K 240fps.

## ABSOLUTE RULES
- NEVER hand-roll any ML algorithm, math routine, or data structure that exists in a dependency.
- NEVER reimplement: distance metrics, scalers, encoders, splitters, loss functions, optimizers, matrix ops, stats functions, clustering, trees, linear models, kernels, decompositions, or any preprocessing step.
- Before writing ANY function, check this list. If a crate covers it, USE THAT CRATE'S API.
- When unsure, grep Cargo.toml and check the crate's docs.rs page before writing code.
- Use 6-space tab indentation everywhere.

## CRATE MAPPING

### Linear Models / Regression
- Ridge, Lasso, ElasticNet: `linfa-elasticnet`
- OLS, GLM: `linfa-linear`, `ferrolearn-linear`, `linreg-core`
- LARS/LASSO path: `linfa-lars`
- Logistic regression: `linfa-logistic`
- Isotonic regression: `sklears-isotonic`
- PLS: `linfa-pls`
- Gaussian process regression: `friedrich`
- Misc regression: `anofox-regression`

### Trees / Ensembles
- Decision trees: `linfa-trees`, `sklears-tree`
- Random forest, ExtraTrees, GBR, AdaBoost, Bagging: `smelt-ml`
- Pure Rust RF: `randomforest`
- Pure Rust GBDT: `gbdt`
- XGBoost: `xgb` (v3.0.5, marcomq/rust-xgboost, prebuilt binaries)
- LightGBM: `lightgbm3` (feature `lgbm`), `lightgbm-rust` (feature `lgbm-rust`)
- CatBoost: `catboost-rust` (feature `catboost`)

### SVM
- linfa SVM: `linfa-svm`
- sklears SVM: `sklears-svm`
- light-svm: `light-svm`

### Bayes
- Naive Bayes: `linfa-bayes`

### Neighbors / KNN
- KNN classifiers/regressors: `sklears-neighbors`, `ferrolearn-neighbors`
- Spatial indexing: `kiddo`, `ball-tree`
- Approximate NN: `hnsw`

### Clustering
- KMeans, DBSCAN, GMM, Spectral: `linfa-clustering`
- HDBSCAN: `hdbscan`
- KMedoids: `kmedoids`
- Misc clustering: `ferrolearn-cluster`

### Neural Networks / Deep Learning
- ELM: `elm`
- Perceptron: `perceptron`
- Neural nets (sklears): `sklears-neural`
- Burn framework: `burn`, `burn-ndarray`, `bimm` (feature `dl-burn`)
- Candle framework: `candle-core`, `candle-nn`, `candle-transformers` (feature `dl-candle`)
- Compile-time shape DL: `dfdx` (feature `dl-dfdx`)
- Vision models: `torsh-vision`, `microcnn`, `axonml-vision`
- Pretrained models: `torsh-models`

### Preprocessing
- Scalers, encoders, normalizers: `linfa-preprocessing`, `sklears-preprocessing`
- Feature selection: `sklears-feature-selection`
- Feature extraction: `sklears-feature-extraction`
- Kernel approximation: `sklears-kernel-approximation`
- Imbalanced resampling (SMOTE, ADASYN): `imbalanced-sampling`, `rust-imbalanced-learn`

### Dimensionality Reduction / Manifold
- PCA: `linfa-reduction`, `pca`
- ICA: `linfa-ica`
- t-SNE: `linfa-tsne`
- UMAP: `umap-rs`
- Manifold: `sklears-manifold`
- Embeddings: `annembed` (feature `embed`)

### Discriminant Analysis / Covariance
- LDA/QDA: `sklears-discriminant-analysis`
- Covariance: `sklears-covariance`
- Cross decomposition: `sklears-cross-decomposition`

### Semi-Supervised
- Label propagation/spreading: `sklears-semi-supervised`

### Outlier Detection
- Isolation forest: `extended-isolation-forest`, `isolation_forest`
- Local outlier: `local-outlier-probabilities`

### Sequence Models
- CRF: `crfs`
- HMM: `hmm`

### Conformal Prediction
- `conformal-prediction`

### Hyperparameter Optimization
- TPE sampler: `tpe`
- Pruners (Median, Percentile, Hyperband, etc): `optimizer::pruner::*`
- Bayesian/EGO: `egobox-ego`
- Samplers, trials, study: `optimizer`

### Statistics / Math
- Distributions, stats functions: `statrs`
- Array stats (mean, var, quantile): `ndarray-stats`
- Stats: `scirs2-stats`
- Correlation: `correlation`, `traquer`
- Causal discovery: `deep_causality_discovery`
- Special functions: `special`
- Log probability: `logp`

### Linear Algebra
- Dense: `ndarray`, `nalgebra`, `faer`, `faer-ext`
- Sparse: `sprs`
- LAPACK: `ndarray-linalg` (feature `lapack`)
- linfa linalg: `linfa-linalg`
- Optimization (L-BFGS, etc): `argmin`, `argmin-math`

### NLP / Text
- TF-IDF vectorizer: `tf-idf-vectorizer`
- BM25: `bm25`
- Text tokenization/vectorization: `vtext`
- Text analysis: `text_analysis`
- FastText embeddings: `fasttext`
- Word2Vec embeddings: `word2vec`
- HF tokenizers: `tokenizers` (feature `hf-tokenizers`)

### Image / Audio
- Image loading/transforms: `image`, `imageproc`
- CLAHE: `clahe`
- Spectrograms: `spectrograms`
- Mel spectrograms: `mel_spec`

### Time Series
- Forecasting, seasonality, ETS, MSTL: `augurs`
- Series utils: `scirs2-series`
- Transforms (FFT, wavelet): `scirs2-transform`
- Dates: `chrono`

### Graphs
- Graph structures: `petgraph`
- Graph algorithms: `scirs2-graph`

### Inference / Deployment
- ONNX Runtime: `ort` (feature `onnx-ort`)
- Tract ONNX: `tract-onnx` (feature `onnx-tract`)
- Model I/O (safetensors, HF hub): `hf-hub`, `safetensors` (feature `model-io`)

### Data Loading
- DataFrames: `polars`
- CSV: `csv`
- JSON: `serde_json`
- Binary serialization: `bincode`, `serde`

### Kernels
- Kernel methods: `linfa-kernel`

### Core Infra
- Parallelism: `rayon`, `crossbeam`
- RNG: `rand`, `rand08`, `rand_chacha`, `rand_xoshiro`, `rand_distr`
- Hashing: `ahash`
- CLI: `clap`
- Errors: `anyhow`
- Enums: `strum`
- Scripting: `mlua`
- Logging: `tracing`, `tracing-subscriber`
- TUI: `ratatui`, `crossterm`
- Progress: `indicatif`
- Plotting: `plotters` (feature `plot`)
- Geo/spatial: `geo`
- DB: `rusqlite`
- Floats: `noisy_float`
- Smartcore (misc): `smartcore`
- Ferrolearn core: `ferrolearn-core`
- Sklears core: `sklears-core`
- Scirs2 core: `scirs2-core`

## Agent Dispatch Rules

Research → Haiku. Implementation → Sonnet. No exceptions. Always `run_in_background: true`.

## What This Is

Rust AutoML system for regression tasks (Kaggle-oriented). MOEA/D multi-objective search over model/hyperparameter/preprocessor space, per-fold cross-validation with no data leakage, GPU-accelerated models, and post-search refinement with holdout evaluation.

## Build & Run

```bash
cargo build --release           # thin LTO, ~40s on AMD64
cargo run --release -- --trials 200 --seed 42
cargo test                      # 41 tests passing
cargo test --lib optimizer      # single module
cargo test test_kfold_5_splits  # single test
```

Default dataset: Ames Housing (index 1). Override with `cargo run --release -- 0` or `cargo run --release -- arc`.

## Architecture

All numeric computation uses `ndarray::Array2<f64>` (features) and `Array1<f64>` (targets).

```
src/
├── zoo.rs          — define_model! macro + Model enum (106 bridged variants)
│                     One macro invocation per model, bridge closure in zoo.rs.
├── bridge.rs       — Regressor/Transformer traits + ModelInstance (crate wiring)
│                     Bridge macros: fit_elasticnet!, fit_smelt!, fit_svm!, fit_linfa_classifier!
│                     GPU weight structs: GpuMlpWeights, GpuCnn1dWeights, GpuTransformerWeights,
│                       GpuVaeWeights, GcnWeights, GatWeights (zero-copy predict, weights stay on GPU)
│                     Preprocessors: StandardScaler, RobustScaler, MedianImputer (bridge Transformer trait)
│                     KNN graph construction + GCN/GAT message passing (real graph NNs)
│                     ndarray 0.16↔0.17 conversion (to_sklears_array2/from_sklears_array2)
├── preprocessor.rs — define_preprocessor! macro (19 variants: 9 scalers, 5 feat_sel, 3 augment, 2 imputers)
│                     FittedTransformer: fit on train fold, transform test with fitted params (no leakage)
│                     dispatch_fit_transform() returns (transformed_train, FittedTransformer)
│                     select_columns() helper for feature selection
├── optimizer.rs    — MOEA/D (MultiObjectiveStudy) + 7 pruners, RepeatedKFold, 6 metrics
│                     Static folds across all trials (same seed, apples-to-apples comparison)
│                     Per-fold preprocessing: scaler fit on train, feat_sel indices from train, augment train-only
│                     Shape guard: pred.len() != y_test.len() → NaN + prune (sampler's fault)
│                     refine_best(): 5×10 extended CV with per-fold preprocessing
├── lib.rs          — pub mod declarations only
├── main.rs         — CLI (clap) → headless or --tui, default dataset = housing (index 1)
├── gpu/
│   ├── memory.rs   — GpuBuffer (Send+Sync), BufferPool, hipMalloc/hipFree/hipMemcpy
│   ├── kernels.rs  — HIP/ROCm kernels: gemm, softmax, relu, gpu_sgd_update, pairwise_l2, im2col, etc.
│   └── kernels/    — .hip source files (elementwise, reduce, argsort, distance)
└── utils/
    ├── data.rs     — CSV loading (Polars→ndarray), frequency encoding for categoricals, train/test split
    ├── tui.rs      — Ratatui dashboard + headless stderr output, sigaction Ctrl+C handler
    │                 Post-search: refine → fit with per-fold preprocessing → holdout → save artifacts
    ├── tests.rs    — unit tests (41 passing)
    ├── run.nu      — Nushell orchestration wrapper
    └── report.rb   — Ruby report generator
```

## Data Flow

`main` → `tui::run_headless()`:
1. Load CSV via Polars, frequency-encode categoricals, split train/test (80/20)
2. Fit MedianImputer on train only, transform both with train's medians
3. `optimizer::search()` runs MOEA/D with static 3×4 RepeatedKFold:
   - Each trial suggests Model + scaler + feat_sel + augmentation via CategoricalParam
   - Per-fold: fit scaler on train fold → transform test with FittedTransformer
   - Per-fold: fit feat_sel on train fold → apply selected_indices to test
   - Augmentation (noise/dropout) applied to train only; PolynomialFeatures to both
   - Scores 6 objectives (R², RMSE, MAE, MAPE, LogCosh, Huber), all maximized (losses negated)
   - 7 pruners check `trial.should_prune()` every fold step
4. Post-search: reconstruct best preprocessor pipeline from serialized configs
5. `refine_best()`: 5×10 CV with per-fold preprocessing for refined score estimate
6. Final fit: fit preprocessors on full train, transform test with train's params, fit model, predict
7. Save artifacts: `results/best_stack.json` + `results/test_predictions.csv`

## Key Traits (bridge.rs)

- `Regressor` — `fit(&mut self, data: &Data)` + `predict(&self, x) -> Prediction`, requires `Send + Sync`
- `Transformer` — `fit(&mut self, x)` + `transform(&self, x) -> Mat` + default `fit_transform`
- `FittedTransformer` (preprocessor.rs) — captures fitted params in closure, `transform(&self, x) -> Mat` + `selected_indices()`

## GPU Models

All GPU models keep weights on GPU after fit. Predict only uploads X, single download at end.

- `GpuMlp`: 2-layer MLP with ReLU, full backprop via SGD
- `GpuCnn1d`: im2col + GEMM conv, ReLU, avg pool, linear head, full backprop
- `GpuTransformer`: self-attention (Q/K/V) + FFN + linear head, frozen attention + trained FFN/head
- `GpuVae`: encoder-decoder VAE with reparameterization trick, full backprop including KL
- `GpuLstm`: features-as-timesteps (seq_len=n_feat, input_dim=1), Ridge readout on final hidden
- `Gcn`: real GCN on KNN graph, multi-layer message passing H'=ReLU(Ã@H@W+b), full backprop
- `Gat`: multi-head attention on KNN graph, learned α_ij = softmax(a_l·Wh_i + a_r·Wh_j), full backprop

`gpu_sgd_update()`: Y = Y - α·X (gradient descent). NOT standard axpy. Named to prevent misuse.

## Conventions

- Edition 2024, Rust stable.
- `anyhow::Result` for fallible functions.
- Deterministic RNG via `ChaCha8Rng` seeded from CLI `--seed`.
- Progress output goes to stderr; structured output to stdout/files.
- Failed model fits return NaN predictions (pruner handles them), never y.mean() dummies.

## Crate Stack

Three pillars:

- **linfa** (forked) — traditional ML: SVM, naive Bayes, PLS, LARS, elasticnet. Bridged via macros in bridge.rs.
- **smelt-ml** — tree ensembles: DecisionTree, RF, ExtraTrees, GBR, AdaBoost, Bagging.
- **optimizer** (forked) — MultiObjectiveStudy with MOEA/D sampler + 7 pruners via CompositePruner.
- **Additional:** friedrich (GP), ferrolearn, sklears-*, smartcore, hdbscan, augurs, etc.

All models defined in `zoo.rs` via single `define_model!` macro. 106 bridged variants — no ALLOWLIST.

**Build enforcement (build.rs):** Scans all src/*.rs, bans 50+ patterns at compile time. No exemptions.

**Forked crates:** optimizer (pruner support on MultiObjectiveStudy), linfa (ndarray 0.16 compat), sprs, torsh-core, burn-import, xgboost_lib-sys, sklears-neural (println removal).

## Requirements

1. Allowed Languages:
	- `rust` / `lua` / `ruby` / `Nu`
2. Must have support for all existing on Earth as of the last 42 days:
	- Models (Regressors)
	- Preprocessors (Outlier/Transform)
	- Scalers
	- Feature Selection
	- Classifiers
3. **Crate-first**: Use existing crates for every algorithm possible. Index what they provide, wrap behind `Regressor`/`Transformer` traits, expose to optimizer. Only hand-roll when no crate exists.
4. **Full valid ranges for optimizer**: Every hyperparameter must expose its full mathematically valid range to the optimizer. No "standard" or "typical" ranges — that's reward hacking. If a parameter supports `-inf < 0 < +inf`, that's what the optimizer sees. If it's `(0, +inf)`, give it `(0, +inf)`. The optimizer decides what's good, not us. The Python recipe did this correctly.
5. **Macro schema for the zoo**: All models defined via single `define_model!` macro in `zoo.rs`. Each model = one line in the macro invocation. Each model has bridge closure in zoo.rs. Never write per-model files or structs. Models are data, not code.
6. **No hand-rolling what exists**: If an algorithm exists as a Rust crate, the only code written is the macro invocation to bridge it. Hand-rolling is permitted ONLY when no crate on Earth implements it. Must not exist anywhere before you implement it.
