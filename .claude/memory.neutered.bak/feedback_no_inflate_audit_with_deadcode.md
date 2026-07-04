---
name: feedback_no_inflate_audit_with_deadcode
description: "In capability/work-gap audits, never count f32 dead-mirrors or deferred-by-design items as forge candidates — classify them internal. Counting dead/by-design code as work inflates scope and muddies priorities."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 37002afc-9794-4d76-9e65-256d036b31fe
---

When auditing "what's left to build" / capability gaps, an item is NOT a forge candidate if it violates a standing project invariant or is explicitly deferred by design. Classify these `internal`/excluded, never `unwired`/forge:
- **f32 anything** — f64 is non-negotiable ([[project_f64_required]]); every f32 kernel/fn is a dead mirror, banned from the user path. Not work.
- **2D/3D conv + their padding** — deferred by design ([[project_no_2d_3d_conv]]); 2D/3D is reshape/tile-into-a-grid, not a layer. Not pending work.
- Same logic for any pooling-as-layer ([[project_pooling_kernel_only]]) or other by-design exclusions.

Concrete instance: the 41-agent user-API gap audit ([[project_userapi_forge_gap]]) reported 202 forge candidates, but 18 were f32 dead-mirrors + 9 were deferred-by-design 2D/3D = **27 inflation items (no overlap) → real forge ≈ 175.** The inflation concentrated in attention-rope (14 listed → ~2 real f64; the other 12 were f32 building blocks) and conv-pool (15 → 6; dropped 9 deferred 2D).

**Why:** an inflated count makes the work look larger than it is and buries the real priorities under dead code that will never ship. The number has to mean "things you could actually wire," not "every symbol that exists." **How to apply:** before counting any capability as a forge target, filter on the invariants first — f32→drop, 2D/3D→drop, by-design-kernel-only→drop — then count what remains. Same root class as [[feedback_no_handwave_capability_audits]]: report what's real, not what's nominally present.
