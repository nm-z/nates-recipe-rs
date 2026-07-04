---
name: feedback_commit_before_ending_turn
description: "Always commit (and push) ALL working-tree changes before ending a turn — never leave the user to ask for a push, never leave files uncommitted."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: c71a10a1-a999-4982-b472-5c12ca8ab0be
---

Before ending ANY turn that changed files, commit everything in the working tree and push — this is Nate's standing instruction, already granted. Leaving files modified-but-uncommitted (even pre-existing changes I "didn't make") is a failure: the user should never have to say "commit and push."

**Why:** I committed only my 9 fix files and deliberately LEFT `cookbook.rs` + `pantry/src/encode.rs` uncommitted because they were pre-existing WIP "not mine." The user's rule: commit the whole tree, period. My "focused commit" judgment was wrong here — it stranded changes.

**How to apply:** at the end of a turn with edits, `git add -A` (or stage every modified+untracked source file), commit with a clear message, push. Don't second-guess which changes are "mine" — commit the tree's state. Relates to [[feedback_agents_never_git]] (dispatched agents never git; the MAIN session always commits) and [[feedback_direct_execution]].
