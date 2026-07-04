---
name: Lua models are bindings not pipelines
description: Lua GPU model functions must be pure math recipes — weights as inputs, predictions as output, no init/bias/seed/state
type: feedback
originSessionId: b6825836-fa65-4148-9d29-0c0a9e07d7ba
---
Lua model functions are BINDINGS — pure mathematical recipes. They take ALL inputs (data + weights + hyperparams), run the math, return predictions.

**Why:** Bloated Lua functions that create weights, manage bias, handle seeds duplicate what Rust already does. The Lua function IS the algorithm and nothing else. Configuration, initialization, state management belong in Rust.

**How to apply:**
- Weights (W1, W2, etc.) are function INPUTS, not created inside
- No bias terms unless the algorithm requires them
- No Xavier init, no seed, no randn inside the model
- Use explicit primitives (`sub(W1, scale(g1, lr))`) not convenience wrappers (`sgd_update`)
- Return predictions directly, not weights
- The function should be as short as possible — all signal, no ceremony
