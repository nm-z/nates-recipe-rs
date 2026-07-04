---
name: Subagent-first workflow
description: Dispatch implementation to background subagents immediately so user can interact freely while work happens
type: feedback
originSessionId: 65e31ea2-00c3-4e51-a5bf-62ffe7e839d2
---
Always dispatch implementation work to background subagents first, then respond to the user. Don't do the work inline in the main conversation.

**Why:** User wants to be able to talk, complain, redirect, and give feedback in real-time without waiting for a task to finish. Inline work blocks the conversation.

**How to apply:** On any implementation task, immediately dispatch a background Sonnet agent (or multiple parallel agents for independent work). Keep the main conversation free for the user. Report results when agents complete. This stacks with the existing "Research → Haiku, Implementation → Sonnet, always run_in_background: true" rule.
