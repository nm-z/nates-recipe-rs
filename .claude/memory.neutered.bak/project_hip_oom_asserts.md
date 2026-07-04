---
name: hip-asserts-core-dumps-on-oversize-alloc-pre-check-and-inference-is-forward-only
description: "VmHeap assert gated at the alloc choke (counters − 1GB band, probes DELETED); stale POOL_VERIFIED after an ooc fit re-triggers it via fragmented slack — pool_trim() at ooc teardown"
metadata:
  node_type: memory
  type: project
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

Oversize device asks die in rocclr's uncatchable `VmHeap::MapPhysMemory` assert (vmheap.cpp:175 → SIGABRT), never a catchable error. Gated at the single `hipMallocAsync` choke: growth past the proven peak (`POOL_LIVE`/`POOL_VERIFIED` in memory.rs) is refused with a clean `HipError(2)` when `n_bytes > min(hip_free, sysfs_vram_free) − pool_slack − 1GB`. The 1 GB user band also covers the counters' over-report of the true VmHeap ceiling — **probe children were DELETED** under the reserve law (eviction-assisted child mappings never proved the heavy-parent regime anyway).

**Second bite (2026-07-02): the gate's below-high-water fast path is a lie after an ooc fit.** Freeing ~11.5 GB of ooc buffers leaves the pool full of slack with `POOL_VERIFIED` stale-high; the next differently-shaped allocation storm (run()'s post-fit scoring `Scratch::new`) skips the gate (projected < high-water) yet hipMallocAsync can't serve big contiguous asks from fragmented slack → maps new physical → assert. Fix, both halves shipped (6358e45):
1. `gpu_core::memory::pool_trim()` — device sync → `hipMemPoolTrimTo(0)` → `POOL_VERIFIED = POOL_LIVE`; called at the end of every ooc fit (train.rs).
2. run()'s post-fit score uses fit's own stashed score (`ModelInner.fit_score`) whenever `Scratch::vram_bytes(forward_only) > free_vram` — never allocate the buffer set that forced ooc in the first place.

Regression test: `gpu-core/tests/suite/oversize_oom.rs` — a child fills VRAM in 512 MB steps; only acceptable end is `try_alloc_bytes → None` (exit 0); signal 6 = the bug.

Eval layer (still present): eval/predict use `Scratch::new(.., forward_only=true)` and pre-check `Scratch::vram_bytes` against free before allocating. Skipping a too-big holdout stays the clean behavior — never minibatch/chunk eval ([[project_no_minibatching]]).

Related: [[project_waterfall_memory_law]], [[project_memory_ledger_law]], [[project_gpu_wedge_and_staleness]].
