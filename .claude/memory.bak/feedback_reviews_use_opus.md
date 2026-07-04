---
name: feedback-reviews-use-opus
description: "Code/design reviews must run on Opus 4.8 with max thinking, never Haiku"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2f9db8e0-f4bf-408f-bad8-f9ca7d4d43c9
---

Any review step — code review, design audit, adversarial verification of findings — must run on **Opus 4.8 with max thinking**. Haiku (and Sonnet) are not allowed to review.

**Why:** Reviews are the correctness gate; a weak reviewer rubber-stamps subtle GPU/FFI/algebra bugs (the exact class that slips past tolerance-based tests). The user wants the strongest model adjudicating correctness regardless of token cost.

**How to apply:** In Workflow/Agent review or verify phases, set `model: 'opus'` explicitly on every reviewer/verifier agent (do not omit it and rely on inheritance, and never use `haiku`/`sonnet`). This **overrides** the global CLAUDE.md "Research → Haiku" dispatch rule for the review case. The CLAUDE.md rule still applies to non-review research fan-out. See [[feedback_agent_dispatch_patterns]].
