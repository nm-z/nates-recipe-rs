---
name: waterfall-memory-law
description: "VRAM→RAM→DISK is a strict waterfall — fill VRAM completely before ONE byte pools in RAM; placement is the system's output, never per-call-site choice"
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Nate's memory invariant is a literal waterfall: the top basin (VRAM) fills to the brim before a drop lands in RAM; RAM fills before anything stays DISK-only. Water never pools in two layers at once. Violations he called out (2026-07-01): sizing a RAM pool while VRAM sat at 4.3/12GB committed, a guessed "1GB headroom" constant, and measuring fullness by pool RESERVATION instead of committed/touched pages.

**Why:** Placement scattered across hand-sized per-model pools (resident here, expert-cache there, host pool there) breaks the invariant invisibly and un-observably. One general placer + the byte ledger makes fill order provable.

**How to apply:** `gpu_core::waterfall::Waterfall` (shipped 1105a2f) is THE placer for weight blobs. Fill it LAST (arena/stage/rocBLAS workspace pre-allocated), ONE slab = min(hip_free, sysfs_vram_free) − pool_slack (exact pre-check — see [[hip-oom-asserts]]: oversize asks abort uncatchably), memset-committed once. RAM tier to pantry's 90%-of-available law. DISK blobs never read at fill. No new threshold constants — "full" is the driver's own numbers. Related: [[memory-ledger-law]], [[gemma-engine-state]].

**Reserve law (2026-07-02):** exactly 1 GB of each tier (VRAM/RAM/DISK) belongs to the user — the ONLY headroom constant anywhere (`ooc::USER_GB`); every ratio/floor (90%, /10, /2-remaining probe rules) is banned and was removed. Homes are placed PER WINDOW (~168 MB granularity) so VRAM fills to top−1GB before one RAM byte, RAM to its line before one disk byte — ALL ooc transient host bytes (read-ahead, write-behind, commit staging) come from a preallocated touched POOL_BUFS pool (structural max, panics loud on exhaustion) so RAM sits FLAT at top−1GB while disk churns (verified: 32.2GB flat, MemAvailable=1.0GiB); the old static self-overhead reserve fluctuated 3GB below the line and is gone. The alloc gate refuses inside the last VRAM GB, which also covers the driver counter over-report — per-ask vram_probe children were deleted. Verified live: VRAM 11.5/11.98 pinned. See [[feedback_perf_saturation_laws]].
