---
name: MOEA/D architecture decisions
description: Design rationale for optimizer — MOEA/D only, 6 objectives, static folds, per-fold preprocessing, no timeouts
type: project
originSessionId: 9c0cd41b-1fda-4d4f-b8a4-3675f2a2b08c
---
Key design decisions (2026-04-11, updated 2026-04-12):

**MOEA/D only**: No sampler selection. Neighborhood-based mating converges without thousands of trials.

**6 objectives**: R², RMSE, MAE, MAPE, LogCosh, Huber — all equal weight. No trial time objective.

**Pruning over timeouts**: `should_prune()` at each fold step. 7 pruners (Median, Percentile, Threshold, Patient, SHA, Hyperband, Wilcoxon) kill underperformers deterministically. No wallclock anything.

**Static folds**: Same 3x4 splits for every trial (single seed). Per-trial seed variation conflated hyperparam vs data-split variance. Static folds = clean comparison.

**Per-fold preprocessing**: Scalers fit on train fold, transform test with fitted params. Feature selection indices from train only. Augmentation (noise/dropout) structurally excluded from test. Eliminates global statistic leak.

**Failed fits → NaN, not y.mean()**: Sampler owns failures. NaN predictions trigger pruner. Dummy mean predictors polluted search with R²≈0 zombies.

**No ALLOWLIST**: Every bridged model is in the search space. Dataset-adaptive filtering handles inapplicable options.

**Why:** Previous architecture was flat — ALLOWLIST of 38 models, single-objective TPE on R², global preprocessing leaked validation stats, per-trial fold seeds conflated variance sources.
**How to apply:** When adding models or preprocessors, follow the macro pattern. Never filter the search space manually.
