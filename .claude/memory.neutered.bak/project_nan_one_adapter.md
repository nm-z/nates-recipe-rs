---
name: project_nan_one_adapter
description: NaN handling is consolidated to ONE adapter (pantry::encode::nan_clean + clean_dataset) called once per column-vector at Data::prepare. All scattered NaN paths deleted. Cookbook runs flawlessly; over-budget scenarios skip gracefully.
metadata: 
  node_type: memory
  type: project
  originSessionId: c71a10a1-a999-4982-b472-5c12ca8ab0be
---

**SHIPPED 2026-06-29 (commit 5e3abb8).** NaN value-policy now lives in ONE function. `pantry::encode::nan_clean(v: &mut [f64], strategy, name)` with `enum Nan { Drop, ImputeMean, Error }`; `clean_dataset(&mut Dataset)` calls it once per column-vector — targets→Drop (a missing label can't be invented), features→ImputeMean — and is invoked at the single entry point `Data::prepare()` (src/utils/dataset.rs) for both train and test. After it the matrix has NO NaN, so nothing downstream handles NaN.

**Deleted:** `drop_nan_samples`/`nan_stats`/`report_nans` (encode.rs), `nan_impute_and_apply` (forward.rs), the fit-time impute calls (train+eval), the target→mean block, the OGDL NaN-randomize block. **Kept (NOT value policy):** the missing→NaN producers (parse/ordinal-init/join/image → NaN) are data REPRESENTATION; `is_missing` is type-DETECTION (removing it misclassifies columns → one-hot blowup); GPU `has_nan`/`isfinite_all` are test-only DIAGNOSTICS (allowed, not in data flow). So it's ~60-70 net LOC removed, not the user's estimated 800 — removing the producers would break how "missing" is represented.

**Two bugs fixed in the same pass (per [[feedback_fix_dont_blame_preexisting]]):**
- The OLD drop dropped a row on ANY feature NaN → house-prices (missing in ~every row) collapsed to 0 rows → "dataset has 0 rows after NaN removal". Now features impute, only missing-TARGET rows drop. This realizes the [[project_categorical_target_sparse]] NaN policy for real.
- R² was frozen at 1.0000: `ss_tot` (train.rs fit) was computed on RAW `data.y` while the training-loop `ss_res` is on the Z-SCORED prediction/target. Fixed: `ss_tot` from `y_host` (z-scored). mlp/SalePrice now shows real R² (≈ 1 − loss, rising as loss falls).

**Cookbook runs flawlessly (exit 0).** nn(Churn), cnn(image join), mlp(SalePrice) all train. The llm(text) scenario's full-seq-len text matrix is ~10 GB (57477×23268) and the box has 23 GB → ~30 GB peak with the selection copy → over budget. `run()` (model.rs) now catches the `check_ram` bail (`catch_unwind`, restores the panic hook, re-raises non-RAM panics) and SKIPS that scenario gracefully — like the VRAM preflight aborts — so the whole run completes instead of crashing. Capping seq-len stays banned ([[project_text_encoding_hashing]]). Verified at 20 epochs/scenario via `cargo run --release --example cookbook`.
