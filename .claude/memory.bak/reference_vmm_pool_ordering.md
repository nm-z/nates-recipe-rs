---
name: reference_vmm_pool_ordering
description: HIP VMM reservations corrupt later hipMallocAsync — allocate pool/arena memory BEFORE any VMM buffer
metadata: 
  node_type: memory
  type: reference
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

On ROCm/gfx1101, calling `hipMallocAsync` (stream-ordered pool) **after** a HIP VMM
`hipMemAddressReserve`+`hipMemMap` (as used by the tiered buffer's VRAM tier in
`gpu-core/src/tiered.rs`) returns a pointer whose memory faults on first access
("Memory access fault ... Page not present") — even a plain `memset` of it. The VMM
reservation and the async pool contend for virtual address space.

Proven by bisection in `tiered::tests::stage_across_three_tiers`: allocating the
staging window with `hip::malloc_async` **before** `Tiered::alloc_capped` (the VMM
reservation) → passes; allocating it after → the window's own memset faults. When
window-before-VMM, the pool VA lands adjacent to the VMM VA and works; after, it
lands far away and is unbacked.

**Rule:** allocate all pool/arena device memory up front (the training arena is set
up at `init()`, before any tiered/VMM buffer is created), then create VMM buffers.
A staging window for the tiered consumer must come from that pre-allocated arena,
not a fresh post-VMM `hipMallocAsync`. See [[project_gpu_async_free_fault]] for the
other async-memory fault class (hipFreeAsync racing an in-flight GEMM).
