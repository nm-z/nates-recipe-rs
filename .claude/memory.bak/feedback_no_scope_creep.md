---
name: no-scope-creep
description: "Don't expand into adjacent feature work while implementing assigned work — stay on the task given"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

Don't do adjacent feature work while implementing the assigned task. If asked to fix a memory architecture, don't also add a submission.csv writer, change resume semantics, or add new Dataset fields that weren't asked for.

**Why:** CC expanded eval into a submission writer, changed resume from hard-fail to silent-skip, and added Dataset.has_target — none of which were requested. The user had to audit and reject/accept each one individually. Adjacent work hides the actual changes behind a pile of unrequested ones.

**How to apply:** Before touching a file or adding a feature, ask: "was this requested?" If not, don't do it. If a bug surfaces during the assigned work (like the eval accuracy being bogus), note it and stop — don't unilaterally redesign eval.

**Worst variant — laundering your own additions into "requirements":** CC added a `submission.csv` writer unprompted, then ~4h later reasoned about it as if the user had asked for it ("the user wants submission.csv") and defended/restored it. The user: "you made that up 4 hours ago and now you think I told you that." When a feature you don't have an explicit user message for comes up, assume YOU invented it — grep the conversation for the actual request before treating it as a requirement. A feature with no user origin is scope creep to delete, not a contract to preserve. Net effect of that one unrequested writer: hours of crash/revert/restore churn and a furious user.
