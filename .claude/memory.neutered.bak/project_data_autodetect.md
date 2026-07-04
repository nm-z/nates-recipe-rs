---
name: project_data_autodetect
description: Data::prepare auto-detects features/target and aligns train↔test for any file/dir combo
metadata: 
  node_type: memory
  type: project
  originSessionId: 05a011e7-78e0-4285-b1a1-3aeb9f26cd41
---

`Data` (src/utils/dataset.rs) loads `.set`/`.test` from ANY file or directory, fails only when train/test share no columns. REWRITTEN 2026-06 — the old mean-collapse loader is gone.

**Loader model (data.rs `load_dir_groups` → DirGroup::{Table{headers,hashes,cells}, Image{hashes,pixels}}):**
- A directory is parsed into GROUPS by file type (`feature_group` = part after `__`, or extension like `png`). CSV files of one group STACK their rows (rows = samples, no collapse, no aggregation — ever). A single file = one un-grouped Table (group name `""`, no hash).
- A file's `read_raw_csv` keeps RAW string cells (nulls→""); type inference + encoding happen later in `prepare`, once roles are known.

**Assembly (`assemble` in dataset.rs):**
- The group owning `.target()` defines the samples — its rows at FULL resolution. Every OTHER group is hash-joined: 1-row-per-hash BROADCASTS onto sample rows; equal-rows-per-hash ALIGNS by within-hash position; mismatched row counts are REPORTED in `parsed → unjoined` and left out (NEVER averaged). `sample_hint` carries the set's sample-group to the (unlabeled) test.
- Encoding (`encode`/`infer_attrs`, replaced `materialize`): numeric col passes through (blank/unparse→NaN); nominal FEATURE → one-hot `col=cat`; nominal/numeric TARGET → label-index/parse (blank/unseen→NaN). Categories inferred from the SET; test reuses them BY NAME (a test with fewer cols is encoded by name, not position). Columns namespaced `group:col` (bare for a file).
- LAZY materialization: `Assembled` holds per-group matrices + per-sample gather indices; `select(feats)` builds ONLY kept columns. Critical — a dir with images broadcasts 3072 px/sample (~25GB on 1.5M rows) only if png survives alignment/`.exclude`; dropped/excluded groups cost nothing. Mismatched groups are skipped BEFORE encoding (never materialize a 5M-row group just to drop it).
- Alignment: feats = set.names ∩ test.names, minus `.exclude(pattern)` (exact / `group:*` / group / bare header). selection happens BEFORE `drop_nan_samples` (dropped cols never cause row drops). See [[project_checkpointing]] for the NaN/drop side.

**Target resolution** (`resolve_target`): `.target` wins (exact / trailing `:name`); else if a test exists and exactly ONE table column is train-only, that's the target; else None.

**Why:** "same header = same feature" is WRONG across groups (horizontal_well:GR ≠ typewell:GR — different instrument/depth); namespacing keeps them distinct. Rows are samples, not collapsed. Real datasets: rogii train dir → typewell defines 1,567,045 samples, horizontal_well unjoined (5M rows ≠ 1.5M), png broadcast then aligned out. Tested in tests.rs `pipeline_tests` (fast) + `dir_assembly_tests` (#[ignore], real dir, peak ~9GB).

**How to apply:** never aggregate to force alignment — report and let the user `.exclude()` or join. Keep encoding role-aware and category-consistent (set schema reused by name). Keep materialization lazy. See [[feedback_real_datasets_only]], [[feedback_never_reward_hack]].
