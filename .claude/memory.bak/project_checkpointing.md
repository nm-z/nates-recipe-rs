---
name: project_checkpointing
description: Train::fit checkpoints on per-epoch R² increase; NaN OGDL cells randomized not rejected
metadata: 
  node_type: memory
  type: project
  originSessionId: 05a011e7-78e0-4285-b1a1-3aeb9f26cd41
---

Training lifecycle in `src/utils/model.rs` `Model::fit` / `Train::run`:

- **Checkpointing (default on)**: when `cfg.save` has non-empty parts, every epoch downloads preds, computes train R², and if `e>0 && r2 > r2_prev` writes the **pre-update** weights (the ones that produced that R²) to the resolved save path via `write_ogdl`/`dump_ogdl`. The headless log line for that epoch gets `← checkpoint` appended (green). Sensitivity = any increase vs the immediately previous epoch (NOT best-ever); `r2_prev` only updates on finite R². No config knob, no MA (user deferred that). Cost: a GPU→RAM preds download every epoch — user accepted this explicitly.
- Purpose is crash resilience: clean finish / SIGINT / `q` still do the end-of-run `save` in `Train::run` (overwrites with final weights), so the checkpoint only "wins" the file if the run is hard-killed mid-training.
- **NaN OGDL cells**: the old "all-NaN → reject, random init" logic was removed. Now `load_ogdl` results are scanned; each NaN weight/bias is replaced with `StandardNormal * sqrt(2/in_dim)` (He-scaled, seed `0xB1A5`), and it reports `{nans}/{total} weights+biases (X%) were NaN / randomized those, continuing`. Training never writes NaN, so this only triggers on a hand-edited file. Input-data NaNs are surfaced separately by `report_nans` (detection only, no stop). See [[project_data_autodetect]].
- `save()` was factored into `dump_ogdl(params, parts)` (OGDL string, W row-major `i*out_dim+j`) + `write_ogdl(path, out)` (mkdir -p then write); checkpoint reuses both.

**How to apply:** checkpoint path resolution uses `Train::resolve` (handles `*` = next-to-exe and `~`). Keep pre-update timing — saving after backprop would persist weights one SGD step off from the logged R².
