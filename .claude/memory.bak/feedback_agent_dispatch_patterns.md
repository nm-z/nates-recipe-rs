---
name: agent-dispatch-opus-4-8-sota-subagents-offload-research-testing-to-preserve-context
description: "In nates-recipe-rs use Opus 4.8 for subagents (SOTA, not a thinking-level requirement); be patient (slower-but-better is fine); freely offload research/testing/exploration to subagents or inline web search to keep the main context small, since the project is large, intertwined, scattered with multiple emerging architectures"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

**Model:** Use **Opus 4.8** for subagents in this project — the SOTA model — for both research and implementation. The huge gpu-core kernel surface and the project's scattered, multi-architecture sprawl make weaker models (Haiku/Sonnet) flail even on tight, well-scoped specs (two such subs had to be killed). "Max thinking" is NOT a literal requirement — Nate said that only to emphasize the complexity; the point is just "use the best model."

**Patience:** A sub that takes longer but returns a better result is the preferred trade. Do not optimize subs for speed; Nate is patient and wants the better output.

**Offload to preserve context:** It is completely fine — encouraged — to do a web search inline OR dispatch a subagent for research/testing, specifically to keep your OWN context window small over time. This project is large, intertwined, and scattered, with multiple architectures emerging (Lua runtime, the Rust builder, gpu-core, the tree/GBDT path) — confusing to navigate. To explore it or gather targeted information, dispatch a subagent rather than burning main-thread context spelunking.

**Mechanics:** Auto-dispatch in the background without asking; feed the agent the accumulated context you already have (kernel signatures, file:line, the plan) so it doesn't re-derive what's known. Supersedes the old Haiku/Sonnet split (that's for simpler projects). Consistent with [[feedback_reviews_use_opus]] and [[feedback_subagent_first]].
