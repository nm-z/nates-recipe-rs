---
name: feedback_fix_dont_blame_preexisting
description: "If you hit a preexisting issue, just fix it — never spend time proving it's preexisting"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2cdc1e43-ae8c-445a-9556-af38f0a80ef8
---

When you encounter any preexisting bug/error/breakage while doing other work, just **fix it**. Do NOT spend any time establishing, arguing, or noting that it was preexisting / not your fault / unrelated to your change.

**Why:** Proving "this was already broken" is pure CYA — it costs the user time, fixes nothing, and reads as deflecting blame. He doesn't care whose fault it is; he cares that it works. Spend 100% of the effort fixing, 0% on attribution.

**How to apply:** See a compile error, failing test, broken constructor, missing field, etc. that you didn't cause? Fix it inline and move on. Don't write "note: this was already failing before my change." Don't bisect to assign blame. Generalizes [[feedback_no_cya_on_results]] and [[feedback_never_read_data_or_run_script]] — same root: stop spending effort on blame/defense, spend it on the fix.
