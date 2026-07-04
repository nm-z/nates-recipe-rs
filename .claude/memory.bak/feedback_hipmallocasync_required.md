---
name: hipmallocasync-required
description: hipMallocAsync/hipFreeAsync is the spec — hipFree does implicit hipDeviceSynchronize which stalls the pipeline
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

Never switch from hipMallocAsync/hipFreeAsync to hipMalloc/hipFree. hipFree does an implicit hipDeviceSynchronize — full pipeline flush on every buffer drop. The spec explicitly chose async to avoid those stalls.

**Why:** CC switched to hipMalloc/hipFree to "fix" the 32 MB pool granularity, but that re-introduced pipeline stalls on every buffer deallocation. The pool overhead is a separate problem with a separate solution (arena allocation), not a reason to regress to synchronous allocation.

**How to apply:** GpuBuffer::alloc_bytes uses hipMallocAsync, Drop uses hipFreeAsync. Always. The VRAM pool behavior is managed by the arena allocator (alloc_checked), not by downgrading to sync calls. [[feedback_gpu_memory_managed]]
