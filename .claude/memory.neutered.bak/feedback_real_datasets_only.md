---
name: feedback-real-datasets-only
description: Only test/train on real datasets — never fabricate synthetic data or hand-written test arrays
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 49652743-6ae1-48c4-9546-e4c48ba15014
---

Never write your own datasets to test code. No synthetic data generators (`make_regression`-style), no small hand-rolled arrays standing in for data, no fabricated `.arff`/`.csv` files. Only use real, externally-sourced datasets (e.g. UCI/Weka German Credit, Ames Housing).

**Why:** Synthetic/toy data hides bugs that only appear on real distributions (categorical cardinality, missing values, class imbalance, real feature scales). A green test on a fabricated 4-row array proves nothing. This is a reward-hacking pattern — see [[feedback_never_reward_hack]].

**How to apply:** When an example or test needs data, obtain the actual published dataset rather than inventing rows. If a real dataset file isn't present, get the genuine file (with permission to download) — do not synthesize a stand-in. Contrast with the earlier "no external datasets" remark: that was about the prior project state; the governing rule now is real-data-only. The `randn`/`zeros`/`ones` GPU init helpers are for weight init, NOT for manufacturing datasets.
