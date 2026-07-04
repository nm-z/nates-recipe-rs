---
name: reference_kernel_inventory_db
description: "kernel_inventory.db is live test data for the gpu-core proof suite — deleting it breaks ~20 tests; it's gitignored, restore from git history if missing"
metadata: 
  node_type: memory
  type: reference
  originSessionId: 2cdc1e43-ae8c-445a-9556-af38f0a80ef8
---

`kernel_inventory.db` (repo root, ~3.7MB SQLite, 13016 kernels across 6 sources:
amd_rocm_libs / nvidia_cuda_libs / pytorch / rapids_vision_signal /
tensorflow_jax_xla / transformer_llm_zoo) is the **data source for the entire
gpu-core proof suite** — `tests/inventory_proof.rs` + ~20 `tests/prove_*.rs`, all
via `tests/common/mod.rs::inventory_dir()`, which opens the db and dumps one JSON
per source to a temp dir. It looks like repo-root clutter but is NOT — it's the
ground truth for the kernel-expansion completion gate ([[project_kernel_inventory_goal]]).

**Landmine:** deleting it makes every gpu-core proof test fail/skip — `common/mod.rs`
panics `no such table: kernels` (an empty 0-byte db gets recreated by rusqlite on
open). It was swept up in a root-clutter deletion (commit 4be70c4, "delete
README/model.ogdl/kernel_inventory.db/TODO") and silently broke `cargo test
--workspace` until restored 2026-06-23.

**State:** restored to the working tree from history (`git show 4be70c4^:kernel_inventory.db
> kernel_inventory.db`) and **gitignored** (`/kernel_inventory.db`) — keep the
binary out of git but present on disk for local test runs. If it's missing again,
restore the same way. Do not re-delete it expecting the proof tests to keep passing.

Separately: `prove_conv.rs` SIGABRTs on a GPU page fault ("Memory access fault by
GPU node-1") — a real preexisting gpu-core conv-kernel bug, independent of the db,
still unfixed as of 2026-06-23. So `cargo test --workspace` halts at prove_conv
even with the db present; test the changed crates directly
(`cargo test -p recipe-infer -p pantry -p nates-recipe`). See [[reference_gpucore_build_gotcha]].
