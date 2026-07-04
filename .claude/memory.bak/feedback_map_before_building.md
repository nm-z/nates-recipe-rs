---
name: feedback_map_before_building
description: "On a large scattered build, fan out parallel background read-agents to map the whole space FIRST; don't serial-guess→inline-attempt→stall on first failure"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

The banned loop the user named: **guess one approach → implement that first guess inline → hit a wall → treat the failure as terminal and stop ("Holding.")**. Three faults in one: (1) single-hypothesis tunnel vision, (2) burning main context on serial grep-exploration that should be fanned out, (3) treating the first approach's failure as a wall instead of the next data point.

**Why:** For a large, scattered, multi-architecture build (e.g. wiring gemma-26B onto the GPU via gpu-core + tiered.rs streaming), the problem space is too big to hold or to probe serially. Serial guessing means you commit to approach #1 with partial information, and when it fails you have no #2 ready — so you stall. That stall reads as refusal.

**How to apply:** Before writing build code for anything non-trivial, dispatch parallel **background** read agents (Explore/general-purpose, `run_in_background: true`, or a Workflow when ultracode is on) — one per independent domain — to fully map the space and return conclusions (maps with file:line), not file dumps. Keep the main context free. Only after the maps land do you write the engine ONCE with complete information and multiple approaches in hand. A first approach failing is an experiment result, never a reason to stop — see [[feedback_implementation_priority]], [[feedback_no_excuses]], [[feedback_finish_the_job]]. Generalizes [[feedback_subagent_first]] / [[feedback_parallel_agents]] / [[feedback_agent_dispatch_patterns]] from "prefer subagents" to "map-before-building is mandatory, and first-failure is never terminal."
