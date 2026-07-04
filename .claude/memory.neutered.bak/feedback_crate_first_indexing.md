---
name: Crate-first implementation — index don't reimplement
description: Maximize crate dependencies for models/preprocessors/scalers/feature-selection/classifiers, wrap and expose to optimizer rather than hand-rolling
type: feedback
originSessionId: 1f34447b-2e6b-4b5f-8bcf-649de253d6a4
---
Rely on as many crates as possible. Do NOT hand-roll implementations when a crate exists.

For each category — Models, Preprocessors, Scalers, Feature Selection, Classifiers — the job is to:
1. Find crates that implement the algorithm
2. Index what they provide (wrap their API behind our traits)
3. Expose it to the Optimizer

**Why:** The CLAUDE.md lists hundreds of algorithms across all categories. Hand-rolling each is unrealistic and wasteful when crates exist. The value is in the optimization pipeline, not reimplementing known algorithms.

**How to apply:** When adding any new algorithm, first search for a crate that implements it. Only hand-roll if no viable crate exists. The wrapper should be thin — just enough to satisfy `Regressor`/`Transformer` traits and expose hyperparameters to the optimizer's search space.
