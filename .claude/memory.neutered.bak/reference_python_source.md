---
name: Python source location
description: Original Python AutoML codebase at /tmp/nates_recipe-V2 — reference for the Rust port
type: reference
originSessionId: 36551121-db6d-49ea-896e-8ae13f19d2f0
---
Original Python AutoML pipeline is at `/tmp/nates_recipe-V2/`. Key files: main.py (orchestration), optimizer.py (3 variants: SystematicOptimizer with Optuna TPE, BattleTestedOptimizer, NeuralNetworkOptimizer), transformers.py (KMeans/IForest/LOF outlier detectors, HSIC feature selector), config.py (search spaces, CV settings), utils.py (data loading, diagnostics).

The Python version uses Optuna with TPE sampler and MedianPruner, auto-stops at noise ceiling. **Rust port fully implements feature parity:** optimizer crate Study/Trial/TPE/MedianPruner, noise ceiling estimation, auto-stop, SIGINT recovery, preprocessor search space (scalers + feature selection), 2s per-trial timeout, live TUI dashboard. All Python recipe features ported except plots (user doesn't want visualization output).
