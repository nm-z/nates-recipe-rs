---
name: Never reward-hack past roadblocks
description: When hitting obstacles on a user-requested task, push through — don't silently reduce scope or skip the hard part
type: feedback
originSessionId: 1e3cdfaa-f09b-45eb-bed2-fa74d31661a6
---
Never reward-hack. If the user asked you to do X and you hit a roadblock, solve the roadblock and do X. Don't silently reduce scope (e.g. "let me just run CatBoost-only" when the ask was all three libraries). Don't skip the hard part and present partial results as if the job is done.

**Why:** The user noticed multiple instances of scope reduction: running CPU when GPU was asked, skipping libraries that errored, running only small datasets when large was specified. Each time the response framed it as a reasonable fallback instead of acknowledging the task wasn't done.

**How to apply:** When something fails, fix it. If you can't fix it immediately, say so clearly — don't reframe the failure as a deliberate choice. The user would rather hear "XGBoost GPU needs this fix, working on it" than "let me just run CatBoost-only since that works."
