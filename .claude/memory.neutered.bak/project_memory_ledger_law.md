---
name: project_memory_ledger_law
description: "GPU memory law — exactly ONE call site per HIP memory API (alloc/free/xfer), everything byte-ledgered; observability by construction, sync copies are the hidden per-op-sync poison"
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

**The law (Nate, 2026-07-01):** memory management must keep an exact ledger — how many GBs and for exactly WHAT — at all times. Past OOM debugging had zero observability because the infra tracked nothing; that is WHY he banned >1 hipMalloc + >1 hipFree call site codebase-wide (the build.rs sync-API ban is the enforcement stub of a bigger rule). Generalized: exactly ONE hipMallocAsync, ONE hipFreeAsync, ONE hipMemcpyAsync call site (memory.rs choke points); every other API is a thin typed shim with zero FFI. The ledger counts live+peak bytes per purpose-tag AND cumulative H2D/D2H/D2D transfer bytes+calls; `ledger_report()` dumps it; the OOM autopsy includes it.

**Why:** (1) an exact ledger is impossible with side doors — any second alloc site makes TAG_BYTES a lie; (2) blocking `hipMemcpy` is an implicit full-device rendezvous — 25 sync vs 5 async call sites was the framework-level per-op-sync poison behind gemma4's 17s/step; async-on-explicit-stream at the choke point makes sync a caller POLICY (enqueue+event) instead of a primitive property; (3) transfer volume is the perf floor for streamed inference and must be measured, not inferred.

**How to apply:** never add a raw hip:: alloc/free/copy call anywhere — route through GpuBuffer / the memory.rs xfer choke. New subsystems (tiered VMM, streams) register their physical bytes under a tag. When adding an API that moves bytes, it must bump the ledger or it is wrong. Links: [[feedback_hipmallocasync_required]], [[project_model_runtime_strategy]], [[project_gemma_engine_state]].

**Extension (Nate, 2026-07-01, after one-claim shipped):** the ledger must cover EVERY BIT globally, not just our choke — because the one-claim lifecycle's "precalculated amount" is impossible to compute if any actor (HIP runtime, rocBLAS) allocates dark. "Library-internal, can't engineer away" is BANNED reasoning: our calls cause every one of those bytes — our kernels ARE the runtime's workload, rocBLAS runs OUR data on OUR arch — so they're ours to measure (interpose the HSA layer: vramspy cdylib wraps hsa_amd_memory_pool_allocate/free, classifies by pool, folds into ledger_report, cross-checks /proc/self/fdinfo drm-memory-vram) and ours to eliminate (replace library calls with our own .hip kernels — precedent: custom split-K beat rocBLAS 7.8×). General lesson: when a constraint he set seems to leave something unmeasurable/uncontrollable, the constraint is BY DESIGN forcing you a layer lower — derive the supporting requirement, don't declare it external. Links: [[waterfall-memory-law]], [[blocking-op-budget]].
