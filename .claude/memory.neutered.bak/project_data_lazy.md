---
name: project_data_lazy
description: Data is lazy — .target() only describes; Train::run materializes+frees the dataset per run (Prepared enum). Fixes the cookbook OOM from all 4 datasets loading at construction.
metadata: 
  node_type: memory
  type: project
  originSessionId: c896e4fe-80e3-430b-84b6-81eb86fb789e
---

**Data describes, Train executes (2026-06-29, working tree, uncommitted).** `Data::target()` used to eagerly call `prepare()` (load_groups+assemble+select → full matrices) and store `self.set`/`self.test`. The cookbook builds all 4 `(Data,Model,Train)` up front, so 4 full matrices — incl. the ~10GB LLM text and the image pixels — were resident at construction → OOM before any training.

Fix: `.target()` now records config only (target_names/targets, raw-test read). Loading+encoding happens ONLY when `Train::run`/`eval` asks. Mechanism:
- `model.rs`: `RunData::dataset(&self)->&Dataset` replaced by `prepared(&self)->Prepared<'_>`. `pub enum Prepared{Owned(Dataset),Borrowed(&Dataset)}` + `.get()`. `run` does `let prepared=data.prepared(); let ds=prepared.get();` — an `Owned` dataset lives only for that run, freed on return. Bare `Dataset`/`Option<Dataset>` impls return `Borrowed` (zero-clone).
- `dataset.rs`: removed `pub set`/`pub test` fields. `RunData for Data::prepared()` → `Owned(self.datasets().0)` (drops the test split; training runs don't use it). New `pub fn datasets(&self)->(Dataset,Option<Dataset>)` materializes+prints summary; `print_summary`/`feature_type_counts`/`cat_cardinality_counts` now take `(train,test,attrs)` args. `main.rs` + the safetensors/CHURN tests call `.datasets()` to materialize explicitly.

**Verified by `cargo run --release --example cookbook`:** nn(594k rows) loaded→trained 5000ep→FREED, then cnn(image, 3072 feat) loaded→trained 5000ep(acc 1.0)→FREED, then mlp loaded. Only one resident at a time — **NO OOM**. (Eager would panic/ OOM at mlp_data construction before the loop → 0 models trained; lazy got 2 full trainings.)

**Separate blocker (NOT this goal, NOT OOM):** cookbook then panics at mlp/house-prices — `drop_nan_samples` (encode.rs:764) drops any row with a NaN target OR NaN feature; house-prices "NA-means-none" cols make all 1168 rows NaN → 0 rows → `dataset has 0 rows after NaN removal` (dataset.rs:373). Documented intended policy is impute-features/drop-only-NaN-target (see [[project_categorical_target_sparse]]) but this tree's handler doesn't impute. Blocks reaching model 4 (llm ~10GB). Pre-existing, orthogonal to lazy loading.
