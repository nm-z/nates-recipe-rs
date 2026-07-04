---
name: project_image_group_join
description: "Image dir + CSV join in pantry — the removed \"wells\" skip, the empty-hash join-key bug, and what still must be fixed for 1:1 row-per-image"
metadata: 
  node_type: memory
  type: project
  originSessionId: c896e4fe-80e3-430b-84b6-81eb86fb789e
---

✅ **SHIPPED 2026-06-29 (commit ef76cfb).** The image join now works per Nate's single-list-of-vectors abstraction, verified e2e (examples/e2e_image.rs: cookbook CNN verbatim, image vector → 3072 features, loss 3.32→1.03, acc 0.14→0.91). Two changes: (1) `data.rs load_dir_groups` emits ONE `DirGroup::Image` for the whole dir, hashes = each file's STEM (was one degenerate single-image group per stem). (2) `encode.rs assemble` joins an Image group by matching a sample COLUMN's cell stems to the image vector's filename keys: pick the column with the most stem-matches (`file_stem` helper strips dir+ext), gather each row's image by that key. The Kind::Image filename column is the join key (0 feature width, no longer a panic). The "real fix Q2" below is what got implemented. Historical analysis kept below.

The image-dir → CSV join lives in `pantry/src/encode.rs` `assemble` + `pantry/src/data.rs` `group_and_hash`/`DirGroup`. Three findings:

**1. The `DirGroup::Image` skip — REMOVED 2026-06-29.**
Was `encode.rs:562-568`: `if matches!(g, DirGroup::Image{..}) { skipped.push("... image group (N wells) kept out of the feature matrix — one copy per well, not duplicated into rows"); continue; }`. Authored by Nate himself: today's blame shows `f2acfe84` but that's a *move* commit (recipe/dataset.rs → pantry/encode.rs); `git blame -w -C -C -C` traces the origin to **37788dd (2026-06-06)**, the trailing-stop-checkpoint commit that "also bundles in-progress dataset/data-loading changes." Then relocated verbatim through `4be70c4` (4-crate split) and `f2acfe84`.
- **Why it existed:** deliberate dedup guard for a **many-rows-share-one-image (fan-out)** topology — the plate/assay "well" mental model: one image per well, many measurement rows per well. Splatting a 3072-dim pixel block into every row sharing a hash would duplicate the image N times. NOT a conv-layer placeholder (nothing conv/TODO in it). The `all_one`/`aligns` test directly above it is the fan-out detector (`by_hash.len()` = distinct image join-keys).
- **Why it was wrong:** for 1:1 row-per-image (handwriting dataset, `image_id,label`, `train_0001.png`→`train_0001`), `by_hash.len()==n`, nothing is deduped, and the skip just throws all pixels out of the feature matrix. Removed per Nate's directive "REMOVE IMAGE SKIP".

**2. The join-key bug (still present, bites BEFORE the skip ever mattered).**
`group_and_hash` (data.rs:140) keys on the **filename**: split on `__` (`hash__rest`), else file stem. So an image dir keys rows as `train_0001`, `train_0002`, … But a **lone CSV** gets `let hashes = vec![String::new(); cells.len()];` (data.rs:369) — every row's join key is `""`. In `assemble` the first gate is `shares = by_hash.keys().any(|h| s_count.contains_key(h))`; `""` vs `train_0001` → **false**, so the image group is dropped at the *"shares no join key — separate table"* branch, one step earlier than the removed skip. Removing the skip alone does NOT make 1:1 work — the group never reaches it.

**3. The real fix (Q2, not yet done).** The row-level join key is the **`image_id` CELL value** (`train_0001.png`), which matches the image filename — the hash join uses *file-level* names (empty for a lone CSV, stems for images) and never looks at cell values, so they can't align for 1:1. Fix: detect the CSV column whose cells are filenames present in a sibling image group, stem them (strip ext: `train_0001.png`→`train_0001` to match `group_and_hash`), and populate that CSV group's `hashes` from those cell values. Then `shares`/`all_one` go true and pixels fan in 1:1.

See [[project_no_2d_3d_conv]] (only 1D conv; images are 32x32x3 flattened rows from `image_to_row`), [[project_crate_architecture]] (pantry owns all parsing/loaders), [[feature_never_delete_failing_example]].
