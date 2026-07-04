---
name: Sampler owns all failures
description: Never CYA for bad model outputs — NaN, div by zero, out of range are the sampler's fault. Return garbage metrics, let it learn. Never crash, never clamp, never constrain.
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
The sampler chose the configuration. If it produces NaN, zero variance, div by zero, out of range — that's the sampler's problem. Return the garbage metrics (mean fallback), the optimizer sees it's trash and learns.

**Why:** The whole point of AutoML is: define the search space, hand the optimizer every tool, let it figure out what works. If we clamp, constrain, or panic, we're hand-holding. Clamping lies to the optimizer (thinks it picked 15, model used 8). Panicking kills the study over one bad trial. Constraining limits exploration.

**How to apply:** 
- Never `.expect()` on model `.fit()` — use match, fall back to mean predictor on Err
- Never clamp hyperparams in bridge blocks — if the value is bad, the metrics say so
- Never panic on NaN/Inf in model output — return it, let metrics be terrible
- Use `set_constraints()` only for truly impossible configurations (n_clusters > n_samples), not for "might be bad"
- Even broken params: the sampler should learn not to use them via BoolParam gates
- Shift blame onto the sampler. Params are never the issue.
