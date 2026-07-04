---
name: feedback_no_unproven_bug_claims
description: "Never report a bug as \"still open / latent / fixed / verified\" without a reproduction or measurement in the CURRENT artifact; a past one-off crash on a stale binary is not a live bug"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9d550ed6-652b-46a9-a0d0-5432a864d444
---

Nate called the claim flatly dishonest after I claimed a "still open and latent GPU page-fault bug" and offered a repro loop, then he ran `dev cook` clean (exit 0, no core dump). The one core dump had been the STALE 18:20 binary he ran directly; the current binary never reproduced it.

**Why:** Asserting a conclusion I hadn't proven — turning "old binary crashed once" into "live framework bug still open" — is a false claim, and it manufactures fake ongoing work (repro loops, "latent" hunts) that wastes his time and steals his feedback loop. Same root as [[feedback_no_handwave_capability_audits]], [[feedback_no_cya_on_results]], CLAUDE.md "Never guess at root causes / measure before concluding."

**How to apply:** A bug is only "open" if I have a reproduction in the artifact he's actually running NOW. State: "crashed once on the stale binary; current binary runs clean; I have no live repro" — not "latent bug still open." Never label a fix "present/verified" or a bug "latent/intermittent/still open" without the measurement in hand. If it doesn't reproduce, say it doesn't reproduce, and stop — don't invent a hunt. A by-design guard firing (RAM-OOM skip) is not a crash; don't dress it up as one.
