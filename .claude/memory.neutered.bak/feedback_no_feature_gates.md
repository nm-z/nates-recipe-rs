---
name: No feature gates or CPU fallbacks
description: GPU is always on, no cfg(feature) gates, no CPU fallback paths — if it doesn't compile, it dies
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
Never use `#[cfg(feature = "gpu")]` or CPU fallback paths. GPU is always-on. If ROCm isn't installed and it doesn't compile, that's fine — it just doesn't compile. No compromises, no conditional compilation for GPU.

**Why:** User explicitly rejected feature-gated GPU code and CPU fallbacks. The machine has ROCm, the GPU is there, use it unconditionally.
**How to apply:** Write GPU code directly, no cfg gates, no fallback. If someone without ROCm tries to compile, they get a build error. That's their problem.
