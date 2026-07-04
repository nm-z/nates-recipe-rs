---
name: no-circular-roofline-claims-measure-flops-workgroups
description: "Before asserting %-of-peak or a kernel's root cause, pull actual FLOP count AND workgroup count from the dispatch — never back-fill an unknown dim with the conclusion"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 348dc130-5104-43d2-aaaf-ea9177c58526
---

When analyzing a profiling DB, do NOT claim a kernel runs at X% of peak by assuming peak to derive an unknown dimension, then citing that as proof of peak — that is circular and wrong. I claimed a forward GEMM was "~100% f64 peak" this way; actual was 46.5% (511 GFLOP/s of 1099). I also misattributed a slow backward dW GEMM to "transposed-B tile selection" when the real cause was occupancy starvation.

**Why:** the user does per-dispatch FLOP accounting and catches hand-waved roofline claims instantly. A wrong root cause sends optimization the wrong way (layout change vs split-K).

**How to apply:**
- Get the ACTUAL FLOP count for the dispatch (2*N*K*M for GEMM) and the ACTUAL workgroup count: `(grid_size_x/workgroup_size_x)*grid_size_y` from rocpd_kernel_dispatch. Never assume a dimension.
- Small-output GEMM with a huge reduction dim = occupancy starvation, not tile selection. Ex: weight-grad dW = Xᵀ·dZ, output K×M = 42×64 → 2×2 = 4 tiles → 4 workgroups on 54 CUs (50 idle), 56ms; same 2.56 GFLOP as the forward which packs 29,710 workgroups in 5ms. Fix = **split-K** (partition the N reduction across workgroups, reduce partials), NOT transposing.
- Forward/dX GEMMs (output N×K, large) fill the GPU; backward dW (output K×M, tiny) starves it. Different problem per direction even at identical FLOP count.
- Ceilings: [[reference_gfx1101_roofline]] (1.099 TFLOP/s f64, 432 GB/s).
