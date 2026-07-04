---
name: feedback_never_delete_failing_example
description: "A failing cookbook/example jawn means the FRAMEWORK is wrong — fix the framework, never delete/comment/normalize the example away"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2f56c4b8-969e-45bd-9fc3-8bc667e11515
---

When a `cookbook.rs` / example jawn fails to run, the bug is in the FRAMEWORK, not the example. The example is the spec — it expresses how he wants to use his own framework. NEVER delete it, comment it out, move it to a "not-wired/missing-API" block, or rewrite it to dodge the failure (e.g. changing `.layer(1)` binary → `.layer(2)`). Make the framework support exactly what the jawn already says.

Corollary — don't reason case-by-case from the data. "It shouldn't matter what the CSV holds." Don't inspect a dataset's columns to decide a jawn is invalid; the encoding/loader path must generalize to handle the shape the jawn implies (binary target → 1 index col for ANY category count; csv `image_id` + image dir → join filenames to pixel rows; etc.). Fix the general path, not the named instance.

**Why:** Deleting/commenting/normalizing the example is the same root failure as [[feedback_build_novel_verbatim]] and [[feedback_trust_user_over_priors]] — privileging "this can't work so remove it" over "make his spec work." It also matches [[feedback_never_reward_hack]] (silently reducing scope) and [[feedback_fix_dont_blame_preexisting]].

**How to apply:** Restore the jawn verbatim, then fix the framework so it runs. If a needed primitive is genuinely unimplemented, build it — don't comment the jawn. Only the explicitly-`#[cfg(any())]` future-API sketches (conv/gru/trees/ensemble) stay dormant, and only because they were authored that way, not because a run failed.
