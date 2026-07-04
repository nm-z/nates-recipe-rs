---
name: Parallel agent dispatch preference
description: User prefers parallel worktree agents for independent tasks over sequential planning/review cycles
type: feedback
originSessionId: 7fc0ede0-97ec-44ca-a78c-e3969cc1b37d
---
When tasks are independent (different files, no cross-deps), dispatch parallel agents in worktrees immediately. Don't gate on brainstorming or planning skills when the scope is clear.

**Why:** User said "just do it in parallel" when offered a planning step. Confirmed twice across separate feature sets (crate replacements and image support). Speed > process ceremony.

**How to apply:** For N independent file-level tasks, dispatch N agents in worktrees simultaneously, merge sequentially after all complete. Skip spec/quality review subagents unless the user asks — they slow things down and the user reads diffs directly.
