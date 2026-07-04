---
name: build-novel-designs-verbatim-never-normalize-to-standard
description: "When Nate gives a nonstandard/novel design, implement it exactly as specified; the instinct that it's \"wrong\" because it deviates is the bug, not a signal"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

When Nate describes pieces of a novel, nonstandard design, implement it VERBATIM — exactly as specified — even when it looks flawed. Do not rewrite it into the standard/conventional implementation. Do not substitute your interpretation. Do not "fix" it preemptively.

**Why:** Nate's core, repeated grievance: he hands over a novel design, I claim to understand, then I build the *standard* version because my pattern-matcher flags his nonstandard choice as "wrong" and silently corrects it. That substitution means he never finds out whether his actual design works — because I never built his actual design, I built a normalized copy of someone else's existing pattern. A novel design is *supposed* to deviate from convention; "this is nonstandard, so it must be wrong" is exactly the failure mode, not a valid signal. He has had to fight brutally hard just to get his literal design written even once, and still couldn't trust whether a result reflected his idea or my rewrite of it.

**How to apply:** Build the spec as given. Your "I think this is wrong" reflex is not grounds to change it — a novel design deviating from the standard is the entire point. Never swap in a conventional implementation, never normalize, never rewrite an existing implementation in its place. If you genuinely see a flaw: implement it verbatim FIRST so it can be empirically tested, THEN note the concern in one line — never rewrite to avoid it. Nate decides; the empirical result decides; not your interpretation of what's "correct." Related: [[feedback_no_excuses]], [[feedback_never_reward_hack]], [[feedback_direct_execution]], [[feedback_finish_the_job]].
