---
name: Kaggle S6E4 irrigation competition
description: Active Kaggle competition — multiclass classification, balanced accuracy metric, GPU GBM + NN ensemble
type: project
originSessionId: f44be0ad-41ac-4569-b9ac-5e1062477260
---
Kaggle Playground Series S6E4: predict Irrigation Need (High/Low/Medium) from agricultural features.
Metric: balanced accuracy. Heavily imbalanced — High is 3.3% of data.
630k train rows, 270k test rows, 19 raw features + target encoding + interaction features = 64 total.

Current approach (solve_v2.rb):
- 5-fold OOF with per-fold target encoding, label encoding, binning (leak-free)
- 5 models per fold: 1 GPU-GBM, 1 ResNet, 1 XGBoost, 1 LightGBM, 1 CatBoost
- ResNet: h=64, blocks=3, AdamW, LayerNorm→GELU→Linear residual blocks, VarianceThreshold + SelectKBest
- NN augments with 10K original dataset rows (nn_nft = 514K)
- PATIENCE=50, NN_PATIENCE=50
- Threshold optimization on full OOF

Best LB: 0.97102 (gem-based mega-ensemble). GPU GBM got 0.96910.
Top leaderboard: 0.98215 (Chris Deotte). Baseline cluster: 0.98114.

**Why:** Testing the GPU primitive library on a real competition.
**How to apply:** Use this as the benchmark for primitive design — if the training code looks ugly, a primitive is missing.
