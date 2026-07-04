---
name: feedback_no_marathon_training_runs
description: "Don't launch multi-hour blocking training runs and wait on them — a 20000-epoch detector retrain ran 4h39m+ without exiting ('this will never exit')"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2cdc1e43-ae8c-445a-9556-af38f0a80ef8
---

Launching a long training run as a background command and blocking on its
completion is wrong when the run is multi-hour. A detector retrain at **20000
epochs** ran **4h 39m+ and still hadn't exited** — the user flagged it: "this
will never exit."

**Why:** GPU detector training throughput is ~1.6 epochs/s on this corpus (gfx1101),
so 10000 epochs ≈ 1.7h and 20000 ≈ 3.5h+ (it overran to 4h39m). A run that long
is, operationally, a hang — it ties up the GPU and the workflow with no return.
I had **raised epochs from 10000 to 20000** to chase a 0.94 accuracy gate; the
user's original spec said 10000 and "do not lower epochs" — raising it into
marathon territory was the mistake.

**How to apply:**
- Keep training-run epoch budgets bounded to what finishes in well under an hour.
  Do NOT raise epochs into multi-hour runs to chase a metric.
- If more training is genuinely needed, iterate in **short checkpointed runs**
  (resume from the saved checkpoint) rather than one giant blocking run — so each
  step returns fast and progress is visible.
- A run with no bounded ETA is a hang. Surface it fast; don't sit on a 3–5h
  background command waiting for it to "finish." Relates to
  [[feedback_test_output_visible]] (a hang must surface fast, never sit silent)
  and the detector's [[project_type_detector]] training.
