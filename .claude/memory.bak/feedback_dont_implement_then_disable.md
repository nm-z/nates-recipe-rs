---
name: Don't implement then disable
description: Never implement a feature then make it inoperable via feature gates, cfg flags, or fallbacks — if you build it, it runs
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
Don't implement something then wrap it in cfg gates or fallbacks that make it inoperable by default. If you write GPU code, it compiles and runs. If you add a pruner, it prunes. If you add a metric, it shows up. Don't write code that looks like it works but doesn't actually execute.

**Why:** Implementing features that are silently disabled (like NopPruner, cfg-gated GPU, etc.) is worse than not implementing them — it creates false confidence that something works when it doesn't.
**How to apply:** Before claiming something is done, verify it actually executes in a real run. Code that compiles but never runs is not a feature.
