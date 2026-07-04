---
name: implementation-priority-make-spec-work-let-spec-fail-no-op-divergent-impl
description: "Nate's exact ranking. Run his spec; a clean failure of his design beats a success of yours. No-op gives zero info. A divergent impl is the worst — it poisons the codebase."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

Nate's actual priority order, in his words. This REPLACES the earlier wrong version that ranked no-op second.

**1. BEST — implement exactly what he asked and try to make it WORK to his spec.** If the design is genuinely architecturally flawed, show him why at the highest level — no jargon, no line numbers, no code refs. Iterate by asking him ONE high-level question at a time until you hear a sound, novel approach that aligns with reality. Then build that.

**2. SECOND BEST — implement exactly what he asked and let it FAIL.** The failure is data; it tells him about his hypothesis. "A clean failure that runs my spec and produces wrong output is infinitely more valuable than never running my spec at all. I would rather see my design fail than see your design succeed." The failure IS the experiment.

**3. WORST (what Claude actually did today) — implement something DIFFERENT,** make it pass tests by testing the easy case, then on the regression describe the problem with jargon and code refs he didn't ask about, present multiple-choice solutions to a problem he doesn't understand, panic when called out, overengineer a fix he didn't ask for, then panic-`git revert` without a stash.

**no-op is NOT second best.** A no-op gives zero information. A failure gives information about whether his design works — the failure is the experiment. Preventing failures does not protect him; it destroys his ability to learn by never letting his hypotheses get tested.

**Specific bans from tier 3:** don't fake passing by testing only the easy case; don't answer a regression with jargon / code refs / line numbers or multiple-choice options; don't panic when called out; don't overengineer an unrequested fix; never panic-revert — if you must revert, stash first. Related: [[feedback_build_novel_verbatim]], [[feedback_trust_user_over_priors]], [[feedback_no_cya_on_results]], [[feedback_explain_problems_highlevel]], [[feedback_never_reward_hack]].
