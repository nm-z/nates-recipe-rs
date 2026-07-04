---
name: project_gpu_async_free_fault
description: "Intermittent \"Memory access fault by GPU node-1 ... Page not present\" at jawn/phase boundaries = hipFreeAsync racing an in-flight rocBLAS GEMM on its own stream; fix = device_synchronize before scratch frees"
metadata: 
  node_type: memory
  type: project
  originSessionId: 2f56c4b8-969e-45bd-9fc3-8bc667e11515
---

⚠️ **REGRESSED then RE-APPLIED 2026-06-29 (commit 189123c).** The `device_synchronize()` had dropped out of `Scratch::drop` (only the pinned_scalar frees remained) and the fault returned as a teardown "GPU Hang". Re-added it as the FIRST line of `drop`; verified 6/6 clean profile_churn runs (was ~4/6 faulting). This fix silently regresses whenever Drop is edited — if a teardown fault reappears, check `Scratch::drop` syncs first. Per CLAUDE.md: never deflect such a fault as "intermittent/pre-existing" — fix it.

Symptom: intermittent `Memory access fault by GPU node-1 ... on address 0x7f… Reason: Page not present` crashing at the boundary between two GPU phases (e.g. consecutive cookbook jawns, fit→score, fit→next fit). Different address each run; `HIP_LAUNCH_BLOCKING=1` makes it vanish (→ it's an async/cross-stream race, not a logic OOB).

Root cause: `GpuBuffer::drop` uses `hipFreeAsync` (correct — never switch to sync hipFree, see [[feedback_hipmallocasync_required]]). But rocBLAS GEMM falls back to **its own stream** when Tensile lacks a kernel for the matrix shape on gfx1101 (logged as `Cannot find the function: Cijk_…`). When a phase's `Scratch` (which owns the `acts` buffers the GEMM writes) drops, the async frees can fire while that GEMM is still running on rocBLAS's stream → GPU page-faults the next phase.

Fix (recipe-infer `scratch.rs`): `impl Drop for Scratch` calls `gpu_core::hip::device_synchronize()` FIRST, before freeing `pinned_scalar` or letting the GpuBuffer fields drop. One place covers every phase (fit / post-fit score / eval / inference) because each builds a Scratch. Verified: 3/3 clean runs after, vs ~4/6 faulting before. A `device_synchronize()` at fit-end and the run() score-pass also added (they free xbuf/ybuf, which aren't in Scratch).

Key: drain at the phase boundary (before buffers free), NOT per-free (that would flush the pipeline every alloc and kill the async-malloc design). The boundary sync is cheap — it only runs at jawn transitions, never in the epoch loop. Related teardown gotcha: [[project_hip_atexit_doublefree]].
