---
name: feedback_backgrounded_means_impatient
description: A command the user manually backgrounds = impatience signal; the command is hung or under-utilizing the GPU/threads — stop running long blocking commands without a fast timeout
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9d550ed6-652b-46a9-a0d0-5432a864d444
---

When the user manually backgrounds one of my running commands (harness shows "Command was manually backgrounded by user"), it is an impatience signal: the command is likely hung, or worse, doing something that isn't using all 12 threads / the GPU. It means STOP launching more long blocking commands.

**Why:** A backgrounded command tells me the foreground run was taking too long with no payoff — Nate is at the terminal hands-on and the run is stealing the resource (one GPU process at a time) or just sitting idle/hung. Continuing to fire more blocking commands compounds the problem.

**How to apply:** The moment a command gets backgrounded, stop issuing new long-running commands. Never run a blocking command without a fast timeout — wrap with `timeout 1` (or a short bound) so a hang surfaces immediately instead of sitting silent. Prefer reading already-finished output over re-launching. See [[feedback_test_output_visible.md]] (hang must surface fast, tail don't void) and [[feedback_never_touch_processes.md]].
