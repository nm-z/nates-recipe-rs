---
name: Rust ML ecosystem gaps
description: What Rust ML crates cover vs what's missing — guides build-vs-import decisions
type: reference
originSessionId: 1f34447b-2e6b-4b5f-8bcf-649de253d6a4
---
As of 2026-04, Rust ML ecosystem findings from hands-on integration:

**Working crate integrations (verified in this project):**
- smelt-ml 1.3.0: ndarray 0.16, regression RF/ExtraTrees/GBR/Bagging/DecisionTree. No regression AdaBoost.
- linfa-elasticnet 0.8: Ridge (l1_ratio≈0), Lasso (l1_ratio=1), ElasticNet. Coordinate descent.
- linfa-svm 0.8: SVR via `linfa_regressor!` macro.
- linfa-nn 0.8: BallTree/KdTree with L1/L2/Lp. Search only, not a regressor.
- linfa-reduction 0.8: PCA via `linfa_transformer!` macro.
- linfa-clustering 0.8: KMeans (used for outlier detection wrapper).
- local-outlier-probabilities 1.0.1: LoOP (enhanced LOF) with ndarray Array2.
- burn 0.20: CNN regressor behind dl-burn feature flag.

**Crate limitations discovered:**
- linfa-trees 0.8: CLASSIFICATION ONLY. Label trait requires Eq+Hash, f64 doesn't qualify.
- linfa-ensemble 0.8: CLASSIFICATION ONLY (wraps linfa-trees).
- extended-isolation-forest 0.2: Const generic dimensions `Forest<f64, N>`. Unusable for runtime feature counts.
- smartcore 0.4: Locked to ndarray 0.15. Cannot interop with 0.16 without conversion.
- linfa-preprocessing 0.8: Wraps everything in DatasetBase<T>, incompatible with raw Array2 Transformer trait. ~40-60 lines bridge per scaler.

**Still no crate (must hand-roll):**
- Feature selection (VarianceThreshold, SelectKBest, RFE, Boruta, MI)
- Imputation (MedianImputer, KNNImputer, MICE)
- IsolationForest for f64 with dynamic dimensions
- Regression AdaBoost
- Scalers with GPU paths (StandardScaler, RobustScaler, MinMaxScaler, QuantileTransformer)
- Encoders (one-hot, ordinal, target)
- Pipeline/ColumnTransformer

**Dep compatibility notes:**
- rand_distr 0.6 depends on rand 0.10, conflicts with project's rand 0.9.
- automl crate pins serde = "=1.0.220", conflicts with everything.
- tch-rs and burn both link libtorch — can't coexist.
