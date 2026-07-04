---
name: resume-must-crash
description: "Resume arch mismatch must crash with a message, never silently discard trained weights"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

When resume weights don't match the script's architecture, crash with a diagnostic message and exit(1). Never silently skip the resume and fall through to random init.

**Why:** CC changed a hard failure to a silent skip. The user resumes expecting their trained weights. If the arch doesn't match, silently initializing random means they see garbage R² with no indication why — hours of training discarded without notice. A mismatch is a user error (wrong file or changed arch) and must be surfaced immediately, not papered over.

**How to apply:** The resume validation in Model::fit() prints the OGDL vs model neuron/feature counts and calls process::exit(1). Never clear the resumed weights and continue.
