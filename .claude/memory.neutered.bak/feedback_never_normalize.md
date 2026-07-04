---
name: Never normalize targets internally
description: Never normalize y or manipulate data inside model implementations — hand the raw data to the optimizer, let it deal with trash
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
Never normalize y values inside model fit functions. If the MLP explodes because y is in the tens of thousands, that's the optimizer's problem — it should learn that this model config (learning rate, architecture) doesn't work and move on. Normalizing internally hides the failure from the optimizer.

**Why:** Internal normalization is the same as capping/limiting — it makes bad configs appear to work when they shouldn't. The optimizer needs to see the real failure to learn from it.
**How to apply:** Pass raw x and y to every model. If it produces NaN, the pruner kills it. The optimizer learns that config is bad.
