---
name: no-minibatching-full-batch-only-non-negotiable
description: In nates-recipe-rs, mini-batching is banned — not part of the design, non-negotiable. Training is full-batch gradient descent over the entire dataset every epoch. Never propose batch_size, mini-batch SGD, or shrinking N as a speed/convergence lever.
metadata:
  node_type: memory
  type: project
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

Mini-batching is **banned and non-negotiable** — it is not part of the design. Training is **full-batch**: every epoch processes the entire dataset (e.g. all 594,194 samples) as a single batch. There is no `batch_size`, no mini-batch SGD, no gradient accumulation over sub-batches.

**Why:** Nate stated it flatly as a hard design constraint, alongside [[project_f64_required]]. The standard instinct — mini-batch to fit memory, speed iterations, or "improve generalization" — is off the table here.

**How to apply:**
- Never propose `batch_size`, mini-batch/stochastic SGD, gradient accumulation, or dataset chunking as a performance, memory, or convergence lever.
- The GEMM shapes are full-batch by design (M = full sample count, e.g. 594194). Do **not** suggest shrinking N via batching to get "better-shaped"/higher-utilization GEMMs — the tall-skinny shape is intrinsic and stays. Combined with [[project_f64_required]], the fp64 full-batch DGEMM is the fixed compute floor; optimize via overlap/algorithm/custom-kernel work, never by changing batch size or precision.
- Memory sizing (e.g. an arena) must hold full-batch activations; that is the design, not a problem to "fix" with batching.
