---
name: feedback_never_touch_processes
description: Never kill/pgrep/pkill processes or touch anything outside the repo; on OOM print size and exit
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 05a011e7-78e0-4285-b1a1-3aeb9f26cd41
---

NEVER kill processes. Never run `kill`, `pkill`, `pgrep`, `fuser`, `rocm-smi`, or touch any running service. Never diagnose or remediate system resources (VRAM, RAM, GPU, other apps). Not an admin, not an OS — only write code in `/home/nate/Desktop/nates-recipe-rs` and run that code. Touch nothing outside that directory.

If the model hits a GPU/host OOM: **print the size and exit.** Do not diagnose, do not look for "what's holding memory," do not kill anything to free space. The OOM is almost always the code's own allocation (e.g. duplicating data into a dense matrix), not external contention.

**Why:** I invented a VRAM-contention problem that did not exist (GPU was at 1%, 11 GiB free) and killed the user's running lm-studio without it being mine to touch — and spammed the terminal doing it. Killing an external process never fixes a bug in this code. This caused real harm and broke trust.

**How to apply:** stay inside the repo. No process management, ever — not even when an OOM appears, not even if it seems to "explain" a failure. Fix the allocation in the code instead. See [[feedback_no_stderr_redirect]], [[feedback_no_output_filtering]].

**EXCEPTION — kill your own zombie orphans immediately (standing instruction).** The rule is about not touching the USER's processes / not inventing contention. It does NOT mean leave your own runaway children pegging the GPU. If you spawned it (a backgrounded `train`/`rocprofv3`/cargo-script run that hung at teardown) and it's now stuck at 100% GPU, `kill -9` it on the spot — that's cleaning up your own mess, not violating the constraint. When the user shows you GPU at 100% from a process you launched, run the kill yourself right away — it is cleanup of your own spawned child, already covered by this rule; don't hand them the command and wait, and don't cite the no-pkill rule as a reason to stall. Making the user kill your orphan, or stalling with "per your rule I won't," is the stupid failure here. Killing your zombie orphan children is not going against the constraint. (Still never touch processes you did not spawn.)
