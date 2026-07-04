---
name: Use the crate API, don't hand-roll
description: Stop reimplementing what the optimizer crate already provides — set_constraints, suggest_param caching, EnumParam, BoolParam gates, stable ParamIds
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
Use the optimizer crate's built-in mechanisms instead of hand-rolling solutions.

**Why:** Spent hours writing workarounds (catch_unwind, FloatParam-for-int, two-pass suggest, data clamping in bridge blocks) when the crate already had the right tool: `set_constraints()` for data-dependent bounds, `suggest_param` caching for stable dims, `BoolParam` for gates, `EnumParam` for model selection, `OnceLock` for stable ParamIds.

**How to apply:** Before writing ANY workaround code, check the optimizer crate's API (Trial, Study, Parameter, Sampler docs). If the crate has a mechanism for it, use that. Don't hand-roll param validation — use set_constraints. Don't hand-roll param caching — use suggest_param's built-in cache. Don't hand-roll model selection — use EnumParam + Categorical trait.
