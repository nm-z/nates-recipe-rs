---
name: project_no_detection_thresholds
description: Type detection must have ZERO arbitrary thresholds — every picked constant is a banned guess
metadata: 
  node_type: memory
  type: project
  originSessionId: 68edceae-898b-461c-bcb4-2f8e9d6cc189
---

Data type detection (`infer_attrs` in dataset.rs) must contain **no arbitrary numeric thresholds**. Every picked constant is a guess that some real dataset falls on the wrong side of, so it is banned. Nate deliberately removed them in commit `daa69e7` ("gut detection thresholds") — that was intentional, NOT an accidental loss of detection.

Banned examples he called out: `1800-2200` year range, `≥0.8` mostly-numeric ratio, `avg_repeats ≥ 2.0` for categorical (star ratings: 100 samples/5 values=20 repeats categorical, but 6 samples/5 values=1.2 numeric — same data, different verdict), `digit_count ≥ 4` for dates ("1/2/24" has 3), string length `6-30` for dates, `count > 2` for categorical strings ("why not 2?").

Allowed = binary structural tests with no picked number:
- **Numeric**: EVERY value parses as f64 (all-or-nothing is definitional, not a "mostly" ratio).
- **Image**: every value is an image ref (extension / magic bytes — structural format ID, not a boundary).
- **Categorical vs Text**: the data's own structure — values repeat (`distinct < total`) → Categorical; all unique (`distinct == total`) → Text.
- Consequence: no integer→categorical guess (needs a repeat-rate cutoff), and no heuristic Temporal (every date rule is a magic boundary). Temporal may return only via real date *parsing* (parse-or-not is binary, like f64).

**Why:** Connects to [[feedback_never_reward_hack]] and [[feedback_sampler_owns_failures]] — a magic threshold is reward-hacking the detector to look right on the sample at hand.
**How to apply:** Never add a numeric constant to detection logic. If a distinction requires picking a cutoff, it can't be auto-detected — drop it or use a binary parse test instead. I once "restored detection" by re-adding his banned thresholds — that was the exact error; restore the *structure*, never the constants.
