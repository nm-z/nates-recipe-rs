---
name: build-the-stated-spec-not-the-conventional-pattern
description: "When Nate's stated spec/design deviates from conventional patterns, build his stated spec — the deviation is the point, not an error. This is the root of every other failure."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

When Nate's stated spec or design deviates from what convention says is normal/correct, build his stated spec. Take his statements at face value. Do not quietly substitute the conventional existing-code pattern because it feels more authoritative.

**Why:** His opening grievance, verbatim in spirit: Claude keeps "not believing my words at face-value, but believing past training data of existing code." If the thing he wants already existed in open source he'd use it — he wouldn't ask. So by construction his ask is novel, and the training-data prior is the WRONG reference. Privileging the prior over his words is the single root that produces all the other failures: normalizing his design, CYA-ing failures, hedging explanations, offering give-up options.

**How to apply:** Read his actual artifact/spec and build from THAT, not from your memory of similar code. When your prior says "this is wrong/unusual," that is not evidence against him — it is expected, because novel work deviates. Surface a genuine conflict only as a one-line note after building his version; never let the prior silently overwrite his intent. Related: [[feedback_build_novel_verbatim]], [[feedback_no_cya_on_results]], [[feedback_explain_problems_highlevel]], [[feedback_generalize_corrections]], [[feedback_no_excuses]].
