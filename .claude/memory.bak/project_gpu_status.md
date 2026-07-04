---
name: GPU primitive library status
description: nates_gpu Ruby/Lua GPU library — composable primitives, hipMallocManaged memory, ResNet blocks
type: project
originSessionId: f44be0ad-41ac-4569-b9ac-5e1062477260
---
GPU library exposes composable primitives via Ruby (magnus) and Lua (mlua). All ops stay on GPU — only `download` and `report` do D2H.

Memory: `hipMallocManaged` + `SetPreferredLocation(GPU)` + `SetAccessedBy(GPU)`. VRAM-primary with RAM overflow. No custom pool, no ALLOCATED tracking, no GC hooks. Drop calls `hipFree`.

Key primitives:
- `grad/hessian/tree_build/add_col/report` — fused GBM ops
- `linear/linear_backward` — rocBLAS dgemm fused with bias
- `layernorm_affine/gelu/dropout/softmax` — NN building blocks
- `adamw_update/grad_clip_norm` — optimizer ops
- `bernoulli/randn/rand_uniform` — GPU Philox RNG
- `solve/cholesky/tri_solve` — GPU-only via rocsolver

Design principle: if Ruby code goes past 2 tabs deep, stop and check if a primitive is missing.

**Why:** Fisher Price ML — readable scripts that look like the algorithm, not plumbing.
**How to apply:** New ML algorithms should compose existing primitives. If composing requires >2 indentation levels, write a new fused primitive in Rust.
