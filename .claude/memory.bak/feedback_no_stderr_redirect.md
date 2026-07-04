---
name: feedback_no_stderr_redirect
description: Never use 2>&1 or /dev/null in this project; never merge or hide output streams
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 05a011e7-78e0-4285-b1a1-3aeb9f26cd41
---

In nates-recipe, NEVER use `2>&1` or `/dev/null` in any command. Don't merge stdout+stderr, don't pipe output through `grep` that hides lines, don't filter what's shown. Run commands plainly and let both streams flow so the user sees the full, unedited output.

**Why:** the training log + parsed tree go to STDERR and structured results to STDOUT; merging or grep-filtering them hid intermediate output (parsed tree, NaN report) and read as cherry-picking — the user called it cheating. Also extends the global "never discard output / never redirect to /dev/null" rule.

**How to apply:** invoke `cargo +nightly -Zscript <file>` etc. with no redirects and no output-filtering pipes. If output is long, show it long. Run only ONE GPU process at a time — concurrent `-Zscript` runs cause `HipError(2)` (GPU OOM) at weight init. See [[feedback_no_output_filtering]].
