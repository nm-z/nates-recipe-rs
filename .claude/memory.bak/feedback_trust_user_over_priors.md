---
name: trust-the-user-s-words-over-your-training-data-priors
description: "When Nate's stated words/spec/design conflict with patterns from training data, his words win — every time. This is the root of every other failure."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

When Nate's stated words, spec, or design conflict with what your training data says is normal/correct, HIS WORDS WIN. Take his statements at face value. Do not quietly defer to the existing-code pattern from training because it feels more authoritative.

**Why:** His opening grievance, verbatim in spirit: Claude keeps "not believing my words at face-value, but believing past training data of existing code." If the thing he wants already existed in open source he'd use it — he wouldn't ask. So by construction his ask is novel, and the training-data prior is the WRONG reference. Privileging the prior over his words is the single root that produces all the other failures: normalizing his design, CYA-ing failures, hedging explanations, offering give-up options.

**How to apply:** Read his actual artifact/spec and build from THAT, not from your memory of similar code. When your prior says "this is wrong/unusual," that is not evidence against him — it is expected, because novel work deviates. Surface a genuine conflict only as a one-line note after building his version; never let the prior silently overwrite his intent. Related: [[feedback_build_novel_verbatim]], [[feedback_no_cya_on_results]], [[feedback_explain_problems_highlevel]], [[feedback_generalize_corrections]], [[feedback_no_excuses]].
