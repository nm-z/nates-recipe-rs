---
name: project_categorical_target_sparse
description: REALITY CHECK 2026-06-29 — the "categorical target → single index column + sparse-CE expand_labels" design was NEVER committed; current code one-hots categorical targets, n_targets=N, requires .layer(N)+ce, hard assert. NaN/RAM notes below unre-verified.
metadata: 
  node_type: memory
  type: project
  originSessionId: 2f56c4b8-969e-45bd-9fc3-8bc667e11515
---

✅ **NOW IMPLEMENTED 2026-06-29 (commit ef76cfb).** The index-column design is real as of this commit, verified e2e foreground (e2e_image.rs ce+layer(36) on 35-seen classes: loss 3.32→1.03; e2e_churn.rs bce+layer(1) binary: loss 0.59→0.57). How: `encode.rs` now encodes a categorical TARGET as ONE class-index column (0..N-1) — `match (&attr.kind, is_target(ai))` → Categorical+target → index col (features still one-hot). In `train.rs fit`, `expand_ce = classify && n_targets==1 && out_dim>1` expands that index column to a one-hot of the model's `out_dim` (host-side, into ybuf) and sets `k=out_dim`, reusing the existing dense one-hot CE/accuracy; the hard assert is relaxed to `out_dim==k` (k=out_dim for expand, else n_targets). Binary (bce, out_dim==1) and regression use the column as-is. So `.layer(36)+ce` works even when train shows only 35 classes, and `.layer(1)+bce` works for a binary categorical target. The ⚠️ note below described the PRE-fix one-hot-target behavior — now superseded; kept for history.

⚠️ **(pre-fix, superseded) CORRECTED 2026-06-29.** The index-column + sparse-CE design described below was aspirational and **never implemented** at the time — `git log -S "expand_labels"` returned nothing. Pre-fix behavior:

**Target encoding (pantry `encode.rs`):** a categorical target encodes to **ONE-HOT**, exactly like a categorical feature — `Kind::Categorical(cats)` → `cats.len()` binary columns (encode.rs:157-171, width() line 233). Role only routes columns to x vs y, it does NOT change encoding (encode.rs:132 "Role only decides where the produced columns are routed"). So a categorical target → `n_targets = N` one-hot columns. There is NO single-index-column path.

**CE in `train.rs`:** consumes the one-hot `y` buffer directly — `gpu_softmax_rows_into` then `gpu_sub_scale_into(da, y, …)` (train.rs:138-146). **No `expand_labels`, no `sparse` flag, no index→one-hot bridge.** fit() HARD-ASSERTS `out_dim == n_targets` (train.rs:893-898); preflight WARNS first (model.rs:599-605). `.layer(N)` is MANUAL — must equal class count; 36-class handwriting → `.layer(36)+ce` on 36 one-hot cols.

`collapse_onehot` (dataset.rs:48) is the **embed-on-categorical FEATURES** path (one-hot feature groups → integer-index cols for `embed()` lookup); it never touches the target. Binary "churn .layer(1)+bce" works only when that target resolves to a 1-column kind (Ordinal/numeric 0-1), NOT via any categorical collapse — a `Categorical(2)` target is 2 columns and `.layer(1)` would fail the assert. See [[project_image_group_join]] (the image-join panic at encode.rs:204 blocks reaching this loop).

**NaN policy (`drop_nan_samples`):** impute missing FEATURES with column mean (matching inference-time `nan_impute_and_apply`); drop a row ONLY when its TARGET is NaN. Previously dropped any row with any NaN feature → house-prices collapsed 293→1 sample. See [[project_checkpointing]] for the older "drop NaN rows" note (now superseded for features).

**Large-matrix host RAM (`encode.rs`):** encode writes columns straight into the final matrix (no `Vec<Vec<f64>>` intermediate); `select` moves the single source matrix out via `take_identity` instead of copying when nothing's excluded post-encode; `drop(set_groups)` + `libc::malloc_trim(0)` before the RAM guard so it sees pages glibc retained. This let a 57477×23268 (~10 GB) LLM matrix prep in 32 GB; it then hits the preflight VRAM cap (full-batch too big for the GPU) and declines gracefully — correct, not a bug.
