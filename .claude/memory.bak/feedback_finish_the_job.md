---
name: Finish the job completely
description: When told to show all metrics, show ALL metrics EVERYWHERE — output lines, labels, headers, final summary. Don't half-ass it.
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
When told to add/show something, do it completely across the entire output — not just the per-step lines. If 6 metrics are computed, all 6 show in per-step output AND final summary AND holdout output, with labels. Don't implement something in one place and forget the other 3 places that also need it.

**Why:** Repeatedly implementing something halfway (metrics in steps but not final output, GPU in one function but not the bottleneck, pruner wired but NopPruner, etc.) wastes time on rework.
**How to apply:** After making a change, grep for every place that displays/uses the same data and update all of them in the same edit.
