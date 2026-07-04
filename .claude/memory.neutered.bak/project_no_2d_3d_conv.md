---
name: project_no_2d_3d_conv
description: "No 2D/3D conv as a layer — it's just reshaping features into a grid (feature transformation), deferred to future"
metadata: 
  node_type: memory
  type: project
  originSessionId: 68edceae-898b-461c-bcb4-2f8e9d6cc189
---

There will be no 2D or 3D convolution as a conv layer type. The only conv layer is 1D conv (`conv(filters, kernel, stride)`).

The reason: 2D/3D conv is fundamentally NOT a different operation — it's just processing a grid. The "2D-ness" comes entirely from reshaping/tiling the input into an N-D grid first; the convolution itself is the same. That grid might be an image, but it might be anything (any data can be tiled/arranged into a box). So the real work is a **feature transformation** (reshape / tile / grid), not a new layer type.

**Why:** Treating 2D/3D conv as its own layer conflates the grid-arrangement (a data/feature concern) with the convolution (already done in 1D). The grid is the prerequisite, and it's general-purpose, not image-specific.
**How to apply:** Don't wire conv2d/conv3d as Model layers (their kernels exist in convx.hip but stay unwired). 2D/3D conv is deferred to the future and, when built, comes via the `reshape/tile/grid` feature-transformation path in TODO.md, not a `.conv2d()` layer. See [[project_pooling_kernel_only]].
