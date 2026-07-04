---
name: feedback_perf_saturation_laws
description: "Nate's performance constraints — something must ALWAYS be happening at theoretical-bandwidth speed; CPU or GPU (or both) pinned at 100% at all times except waiting on user input"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Nate's saturation laws for ALL compute work (stated 2026-07-02 during the ooc waterfall build):

1. **Something is always happening, at theoretical-bandwidth speed.** Every front (CPU, GPU, RAM, PCIe, DISK, network) that a phase touches must run at its bandwidth ceiling. A device at 100% util that IS the bottleneck (e.g. NVMe during a disk-bound sweep) is acceptable; everything else idling alongside an unsaturated bottleneck is not.
2. **CPU or GPU (or both) pinned at 100% at any given time** — the only exception is blocking on user input. Idle-looking phases (init, loading, encoding, placement) are violations, not warm-up.
3. **If d(RAM)/dt ≠ 0 then CPU = 100%.** Any phase where memory is moving (loading, paging, memcpy, faulting, encoding, data ops) must be parallelized across ALL 12 threads (engi = 12 threads, `nproc`). A RAM climb at 8% CPU means a single-threaded memory path — find it and fan it out (par_copy in the pinned bounce, par_touch for lazy zero-pages, rayon for parse/encode).
4. **Blocking pattern vs bandwidth pattern:** alternating util graphs (GPU 100%/DISK 0% then GPU 0%/DISK 100%) = serialization = a bug. Overlap with read-ahead (depth ≥2 for device queue depth), write-behind worker pools, and stream/DMA overlap until fronts ride together.
5. **1 GB-per-tier user reserve is the ONLY headroom constant** (see the reserve law): fill VRAM/RAM/DISK to top−1GB. "If you are not at the top of those shits, we will see it."

**Why:** he watches `dev` (1 Hz CPU/RAM/GPU/VRAM/DISK line) and system monitors in real time; any front off its ceiling is immediately visible and reads as lazy engineering.

**How to apply:** after any perf-relevant change, run the workload under `dev` (from my shell: `script -qec 'bash -ic "dev <cmd>"' <capture>`) and read the tail myself: verify tiers at top−1GB, no alternating util sawtooth, no RAM-moving phase below CPU 100%. Known remaining lever for disk-bound sweeps: fewer disk bytes (recompute-over-store for 1W/1R buffers), not more threads.

Related: [[project_waterfall_memory_law]], [[feedback_dev_wrapper_and_60s_tests]], [[project_no_minibatching]].
