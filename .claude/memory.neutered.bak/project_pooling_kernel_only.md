---
name: project_pooling_kernel_only
description: "Pooling is a kernel-level HW concern, never in the user API; downsampling in the API is stride"
metadata: 
  node_type: memory
  type: project
  originSessionId: 68edceae-898b-461c-bcb4-2f8e9d6cc189
---

Pooling stays a kernel-level capability (it's a hardware optimization), never exposed in the user-facing API. There is no `.pool()` method, no `pool` LayerSpec/LayerKind, and there must not be one. The pool GPU kernels exist and are proven (`gpu_max_pool_1d`, `gpu_avg_pool_1d`, `poolx.hip`: global/adaptive/lp, 2d variants; `prove_pool.rs`) but are not wired into any layer.

Downsampling in the user API is done via **stride**, which is the third parameter of the conv builder: `conv(filters, kernel, stride)` (model.rs ~1914). Stride is a conv param, NOT its own layer type — there is no `.stride()` method. Fully wired through conv1d forward + backward.

**Why:** Pooling is a HW optimization, not a modeling choice the user should make in the builder API.
**How to apply:** Don't add `.pool()` to Model. If asked to "wire pooling into the kernel," it means keep it at the kernel level / out of the API — not fuse it into conv. See [[project_gpu_status]].
