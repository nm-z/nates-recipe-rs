---
name: gpu-wedge-and-staleness
description: "hipMallocAsync pool growth stochastically wedges (HSA spin, gdb-verified); fresh pool pages read back as stale ZEROS until flushed — both fixed by single-slab + one memset; watchdog for loud failure"
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Two more ROCm 7.2.4/gfx1101 driver diseases root-caused 2026-07-01 (siblings of the fa28a35 SDMA/L2 set):

1. **Stale-zero uploads (silent wrong results):** bf16 weights bounced into FRESH hipMallocAsync buffers read back as ALL ZEROS during step 0, real bytes visible by step 1. Signature: hidden-state hash bit-constant through 30 layers (zero-contribution layers + layer_scalar≈1 = identity), run-to-run output variance (which uploads were visible in time = the "attractor" chosen). THIS was the gemma4 nondeterminism — not kernels (det_probe: 16 op shapes bit-deterministic in- and cross-process).
2. **Pool-growth wedge (silent hang):** hipMallocAsync spins forever inside HSA during pool growth, ~stochastic per growth event, main thread R-state 100% CPU, survives SIGTERM sometimes. gdb-batch (run as gdb's own child — yama blocks attach) captured the stack. Keepalive/idle-gap and cascade-poison theories tested and dead.

**Why:** Both scale with the NUMBER of pool growth events; both are silent.

**How to apply:** ONE slab allocation for bulk weight storage ([[waterfall-memory-law]]), memset it once (commits pages → kills staleness), bump-place blobs inside. Never per-alloc memsets (hundreds of growth+sync events = wedge lottery). Loud-not-silent: staleness canary (readback head/tail vs shard bytes, bail on mismatch) + load watchdog thread (abort() after 20s without a beat). Oversize asks: exact pre-check per [[hip-oom-asserts]] — hip::sysfs_vram_free() + hip::pool_slack() shipped in 1105a2f.
