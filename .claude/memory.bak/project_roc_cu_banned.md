---
name: roc-cu-banned
description: "ROC AND CU LIBRARIES BANNED codebase-wide — no rocBLAS/hipBLAS/rocSOLVER/rocFFT/cuBLAS math anywhere; sole exception: exactly 2 cu shim call sites for hip→cu MEMORY ALLOC only (rg for cu must match 2)"
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Nate's standing law (re-stated angrily 2026-07-01 — "I've been fighting you this whole time"): **ROC and CU are BANNED.** No rocBLAS, hipBLAS, rocSOLVER, rocFFT, cuBLAS, or any vendor math library call anywhere in the codebase. All GPU math = our own .hip kernels through our own launch wrappers. The ONLY sanctioned vendor-library surface: **exactly 2 cu shim call sites, for hip→cu memory allocation only** (the NVIDIA-backend alloc mapping). Acceptance test: `rg` for cu matches exactly 2 instances.

**Why:** Vendor libraries are dark allocations ([[project_memory_ledger_law]] extension), opaque stream semantics (the hipblas-own-stream race), nondeterminism risk, wedge surface, and unaccountable VRAM — "our kernels ARE the hip runtime, we are making rocBLAS calls with OUR data and OUR arch." Precedent that we outperform them: custom split-K dW beat rocBLAS 7.8×.

**How to apply:** Never add a new roc*/hipblas*/cu* call — write the .hip kernel. Migration order for existing violations (kernels.rs is full of hipblasDgemm/Dscal/Dgemv; build.rs links rocblas+rocsolver+rocfft): (1) gemma4 inference forward (deblas-builder, custom gemm_bt_f64 — in flight), (2) training GEMMs (split-K precedent in math.hip), (3) rocsolver/rocfft users get custom kernels, (4) unlink the libs in gpu-core/build.rs. When done, the only cu tokens are the 2 alloc-shim sites. Related: [[waterfall-memory-law]], [[blocking-op-budget]].
