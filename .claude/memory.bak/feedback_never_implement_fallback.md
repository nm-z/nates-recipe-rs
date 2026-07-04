---
name: feedback_never_implement_fallback
description: Never implement a fallback default for a query that can fail — crash with a clear cause instead. A swallowed wrong value (e.g. CU count = 1) hides the real failure.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: c71a10a1-a999-4982-b472-5c12ca8ab0be
---

Never write a fallback default when a runtime query (device property, capability probe, env lookup, etc.) fails. Let it fail loudly: `assert!`/panic with a message naming the cause. A silent fallback substitutes a plausible-but-wrong value that corrupts downstream behavior and hides the bug.

**Why:** I made `cu_count()` (hipGetDeviceProperties → multiProcessorCount) fall back to `1` when the query returned 0 (device not initialized). That would silently size every split-K launch to a single workgroup — slow, wrong, and invisible. The user: "delete the fallback... never implement a fallback." A crash pointing at "device not initialized" is strictly better than a 1-CU launch that looks fine.

**How to apply:** query → if invalid, panic with the cause (and how to fix). Cache only valid values. This is the same root as [[feedback_no_feature_gates]] (no CPU fallbacks, compile or die), [[feedback_sampler_owns_failures]] (never clamp/mean-fallback, never panic-hide), and the global rule "let things fail — a crash with a clear cause beats a swallowed error / no CYA fallbacks." Applies to ALL fallbacks, not just this one instance.
