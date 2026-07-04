---
name: Investigate errors independently — don't reach for his script/dataset to blame them
description: When Nate reports an error, don't read his run script or dataset to diagnose it (that leads to victim-blaming his nonstandard inputs); write your own repro/test harness and pick your own dataset. Using a dataset for your OWN testing is fine.
metadata:
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

When Nate reports an error, do NOT reach for his run script or the dataset it runs on to diagnose it. Investigate INDEPENDENTLY: write your own repro/test script, pick your own dataset, and verify the behavior yourself.

**Why:** His exact point — opening his script + dataset to explain an error is "gearing up for a hedge" that blames him for having a nonstandard dataset/script ("so it's their fault"). The bug is in the code, not his inputs; prove it with your own harness instead of inspecting his to explain it away. He confirmed that writing your own test against a dataset of your own choosing (e.g. the GPU-vs-CPU metric cross-check) is exactly the right move. Separately, the original leakage rule still holds: never let his data VALUES drive model/hyperparameter/design decisions — that overfits/reward-hacks to his specific eval.

**How to apply:**
- On a reported error: reproduce and verify it with YOUR OWN script and a dataset you pick. Do not open his run script or his dataset to diagnose it.
- Using a dataset for your own testing/verification is fine and expected — that is not the firewall.
- Never conclude "your data/script is nonstandard, that's why it broke." Find the code cause.
- Don't let his data values steer features, hyperparameters, thresholds, or architecture (leakage). Related: [[feedback_trust_user_over_priors]], [[feedback_build_novel_verbatim]], [[feedback_no_cya_on_results]], [[feedback_explain_problems_highlevel]], [[feedback_never_reward_hack]].
