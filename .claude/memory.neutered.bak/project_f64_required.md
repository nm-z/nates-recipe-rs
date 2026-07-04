---
name: f64-non-negotiable-project-wide
description: In nates-recipe-rs, f64/double precision is a hard non-negotiable requirement across the ENTIRE project — never propose fp32, fp16, bf16, TF32, or mixed precision as a speedup, even on consumer GPUs where fp64 is gimped
metadata:
  node_type: memory
  type: project
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

f64 (double precision) is a **non-negotiable, project-wide requirement**. All numeric compute is `Array2<f64>`/`Array1<f64>` and the GPU path is fp64 DGEMM. This is a hard constraint, not a current default.

**Why:** Nate stated it flatly as a requirement when reviewing GPU memory specs. The standard instinct on consumer AMD/NVIDIA silicon (RDNA3 7700 XT runs fp64 at ~1:32 of fp32) is to drop to fp32/mixed precision for a 10–30× throughput win. That door is **closed**. Numerical fidelity matters more than the fp64 throughput penalty.

**How to apply:**
- Never recommend fp32 / fp16 / bf16 / TF32 / mixed precision as a performance lever, anywhere, ever. Don't even list it as an option.
- Treat the fp64 GEMM cost as a **fixed compute floor**. The only legitimate speed levers are: compute/copy **overlap** (hide DMA, metric reductions, scalar downloads, checkpoint copies behind the fp64 GEMMs via multi-stream), removing pipeline-serializing stalls (e.g. `hipFree`'s implicit `hipDeviceSynchronize`), and **algorithmic** reduction of FLOPs — never precision reduction.
- When judging perf claims: any estimate that implicitly assumes fp32-class throughput for this workload is wrong by ~32×. The workload is fp64-compute-bound and stays that way. Related: [[feedback_no_cya_on_results]], [[feedback_trust_user_over_priors]], [[project_gpu_status]].
