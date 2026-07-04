---
name: project_encoding_bloat_profile
description: "Measured f64-encoding footprint across all 611 dataset CSVs — bloat is concentrated in ONE file (LLM train.csv), seq-len max() padding barely fires corpus-wide, and f64 encodes SMALLER than CSV for high-precision numeric sets"
metadata: 
  node_type: memory
  type: project
  originSessionId: 9d550ed6-652b-46a9-a0d0-5432a864d444
---

Empirical bloat scan of all 611 CSVs in `datasets/` (analysis at `/home/nate/Desktop/bloat_analysis.csv`):
- raw total **3.396 GB** → f64 matrix total **22.380 GB** (+18.983 GB, +559%) if every file were encoded.
- **The bloat is one file.** `train.csv` (the LLM classification set) = **10.65 GB, 96.6% avoidable** — the whole 10 GB. The only other avoidable files: output.csv 0.01 GB (61.7%), loan.csv 0.31 GB (58.1%), test.csv ~0 (55.0%), handm.csv 0.07 GB (33.1%). That's **5 of 611**.
- **The `max()` seq-len padding I kept wanting to cap barely fires.** 606 of 611 files have NO row over 256 tokens, so a `.min(ctx)` / context cap would be a **no-op** for them. Capping text seq_len at a fixed window is the wrong generalization — it "fixes" a whole-corpus padding problem that is really one pathological file.
- **f64 is not universally bloat.** 11 files encode SMALLER than their CSV — high-precision numeric sets (VNA/SansEC predictors, submission/target files): sample_submission.csv -59.5% (ratio 0.41), sample_predictors.csv -58.7% (ratio 0.41, 3204 numeric cols), Predictors_..._Hold-2.csv -56.9% (0.43). Numeric cells stored as long decimal text cost more as ASCII than as 8-byte f64.

**Lesson (ties [[feedback_no_unproven_bug_claims]], [[project_no_detection_thresholds]]):** measure the corpus before generalizing a cap/fix. The 21 GB double-copy OOM on the LLM set is one file's 10 GB matrix copied twice in `select()`; the token-padding explosion is not a systemic issue and a magic-256 cap is the wrong lever. Verify against `bloat_analysis.csv`, don't assume.
