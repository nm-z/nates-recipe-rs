---
name: train-run-is-loopable-over-tuples
description: "Train::run takes &impl RunData, so cookbook examples can live in an array of (Data, Model, Train) tuples and run in one for-loop"
metadata: 
  node_type: memory
  type: reference
  originSessionId: 348dc130-5104-43d2-aaaf-ea9177c58526
---

`Train::run(&self, model: &Model, data: &impl RunData)` (src/utils/model.rs:255). `Data` implements `RunData`, so multiple examples can be stored as `[(Data, Model, Train); N]` and driven by a single `for (data, model, train) in &runs { train.run(model, data); }` — no per-example repeated blocks. Each tuple keeps its own epochs/loss/metrics. This is how examples/cookbook.rs loops NN/CNN/MLP/LLM. Don't forget this when asked to dedupe repeated train blocks.
