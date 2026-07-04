---
name: feedback_agents_never_git
description: Dispatched worktree/subagents must be told to NEVER run git — they land in the shared checkout and wreck state
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2cdc1e43-ae8c-445a-9556-af38f0a80ef8
---

When dispatching an Agent (even with `isolation: "worktree"`), the worktree isolation can silently FAIL and the agent runs in the shared checkout at the real repo path. A Phase-2 agent this session ran `git add -A` + commit there: it swept 30M lines of `datasets/` CSVs into a commit AND switched the main session's checkout onto a new branch. Recovery cost real time (reset to master, restore code as uncommitted, gitignore datasets/).

**Why:** the agent thinks it's isolated; it isn't. git is global mutable state shared with the main session.

**How to apply:** Every agent brief that edits files MUST include a hard rule: "NEVER run ANY git command (no add/commit/checkout/reset/branch/stash). Only edit/create/delete files; leave everything uncommitted; the main session handles ALL git." Phase 3 used this rule and the agent left a clean uncommitted working tree — no mess. The main session does the staging/commit/push after verifying the diff. Also: don't trust the agent's claim about git state — verify with `git status`/`git log`/`git worktree list` yourself ([[feedback_never_touch_processes.md]] cousin: stay in control of repo state). Related: gitignore data dirs so a stray `git add -A` can't balloon ([[project_crate_architecture.md]] — datasets/ is now ignored).
