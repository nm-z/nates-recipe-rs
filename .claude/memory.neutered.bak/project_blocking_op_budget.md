---
name: blocking-op-budget
description: "Hard budget per run: async transfers/allocs/frees unlimited; blocking ops capped at 2 allocations, 2 frees, unlimited blocking transfers"
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Nate's op budget for the engine (stated 2026-07-01):

- **Asynchronous** transfers, allocations, frees: **no limit**.
- **CPU-blocking allocations: max 2** per run.
- **CPU-blocking frees: max 2** per run.
- **CPU-blocking transfers: unlimited.**

**Why:** Blocking alloc/free are pipeline drains and (pool growth) wedge lottery tickets — they must be rare, deliberate events (e.g., the waterfall slab claim), not a per-buffer pattern. Transfers may block freely (a sync'd copy is sometimes the correct semantics); alloc/free must not.

**How to apply:** Current gemma4 (1105a2f) is 318 blocking allocs / 7 async frees / 29,553 blocking transfers — MET in c400f15: Waterfall::claim() = init→ONE async pool alloc of all free VRAM (11.23GB), registered as device arena via memory::set_device_arena — everything (norms/arena/stage/blobs/hipBLAS workspace via hipblasSetWorkspace) carves from it; skip_pool_warm(); exit→the claim's one async free. Ledger: allocs 1, frees 1, blocking allocs 0. The deeper spec: init→precalculated claim, exit→free ALL — one invocation per op per process lifetime, not just one call site. Related: [[waterfall-memory-law]], [[memory-ledger-law]].
