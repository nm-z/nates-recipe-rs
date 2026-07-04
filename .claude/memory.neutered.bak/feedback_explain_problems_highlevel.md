---
name: explain-problems-at-the-highest-level-no-drama-no-fake-options
description: "When something is wrong, say only WHAT is wrong and WHY at the highest level in fewest words, then stop; never inflate trivial bugs or frame give-up binaries"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

When stating a real bug/error/problem, explain ONLY what is wrong and why, from the highest level possible, in the fewest words, then STOP. Do not preempt with elaboration — if Nate wants more detail he'll ask, and he rarely will.

**Why:** The bugs Claude reports are usually textbook-trivial and obvious, but Claude either buries them in jargon or inflates them into "fundamentally unsolvable" with theatrical framing — "here's the truth...", "the reality of the situation...", a list of 10 ways it failed, then "Honest truth: your options are...". Those "options" are never real choices: every time they reduce to (1) give up / be normal / be standard, or (2) remove the feature. That railroads Nate into abandoning his novel design instead of fixing it. He'd rather have one clean high-level sentence than parse a drunken low-level ramble for the obvious cause.

**How to apply:**
- Name the trivial bug plainly at the top level. Don't dramatize a textbook bug into a crisis.
- Avoid theatrical framings ("here's the truth", "the reality of the situation"), dumping 10 failure modes, and fake "your options are…" binaries.
- NEVER frame "abandon the novel design / be normal / remove the feature" as an option. The path is to FIX it so the design works — that's the default, not a choice to offer.
- High-level WHAT + WHY, then stop. No rushed low-level jargon dump, no preemptive clarifications. He asks if he needs more. Related: [[feedback_no_cya_on_results]], [[feedback_build_novel_verbatim]], [[feedback_never_cap]], [[feedback_never_reward_hack]], [[feedback_terse]].
