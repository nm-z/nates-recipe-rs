---
name: GPU memory allocation strategy
description: Two-threshold allocator with GC hook — do not deviate from the tested approach in gpu-core/src/memory.rs
type: feedback
originSessionId: d45109e4-23c5-4af6-a6ad-f6cadbc88354
---
GPU memory uses a two-threshold strategy in `gpu-core/src/memory.rs`:

1. **85% of total VRAM**: triggers registered GC hook (Ruby GC) to free dead buffers
2. **90% of total VRAM**: new allocations spill to `hipMallocManaged` (RAM) instead of `hipMalloc` (VRAM)
3. **Drop**: `hipFree` on owned buffers when Rust drops them

The check is `used + n_bytes > total * N / 100` — comparing against **total capacity**, not free space. This ensures small allocations (4 MB) spill at the same threshold as large ones (2 GB).

**Why:** Previous implementation checked `n_bytes <= free * 90 / 100` (per-request fit in remaining free). Small allocations always passed this check regardless of how full VRAM was. Dead buffers piled up to 10+ GB. RAM was never used. Four stub functions (`set_oom_hook`, `pool_flush`, `set_budget`, `allocated`) pretended memory management existed but were no-ops. Diagnosed via scientific method over a full session.

**How to apply:**
- `set_gc_hook` stores a real function pointer via `OnceLock`. Ruby registers `rb_gc_start` + `device_synchronize` at init.
- No custom pools, no budget tracking, no `allocated()` counter.
- Never add stub functions that look functional but do nothing.
- The GC hook is the only mechanism that forces dead buffer cleanup — do not remove it.
- `mem_report()` in solve_v2.rb prints RAM/VRAM per round for verification.
