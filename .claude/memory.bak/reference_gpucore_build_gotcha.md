---
name: reference_gpucore_build_gotcha
description: gpu-core duplicate-symbol link failure cause + fix (stale .o after build.rs naming change)
metadata: 
  node_type: memory
  type: reference
  originSessionId: cea6d039-8630-495c-9a35-b344784ca5fe
---

**Symptom:** `cargo build`/`test` of gpu-core (or anything linking it, e.g. lightgbm-rs) fails with `rust-lld: error: duplicate symbol: launch_goss_sample` (and others), each "defined at lightgbm.hip" in both `lightgbm.o` and `lightgbm_hip.o` inside `libgpu_core-*.rlib`.

**Cause:** gpu-core/build.rs was changed from a hardcoded `kernels = [...]` array (object files named `<name>.o`) to recursive `collect_hip_files()` auto-discovery (object files named `<relpath with . -> _>.o`, e.g. `lightgbm_hip.o`). `ar rcs libhipkernels.a <objs>` ADDS/REPLACES members but never REMOVES stale ones, so the OUT_DIR's archive accumulated BOTH the old `<name>.o` and new `<name>_hip.o` → every `launch_*` symbol defined twice.

**Fix (surgical, ~instant):** in `target/debug/build/gpu-core-*/out/`, delete the stale old-named `.o` (the ones NOT ending in `_hip.o`: apriori.o argsort.o distance.o dtw.o elementwise.o lightgbm.o reduce.o tree.o) and `libhipkernels.a`, then `touch` any `.hip` to force build.rs to re-archive cleanly. `cargo clean -p gpu-core` also works but recompiles all .hip (~slow).

This was the link error that stalled the concurrent expansion swarm's build (session 2f9db8e0). See [[project_kernel_inventory_goal]], [[project_gpu_core_expansion]].

**Second, DISTINCT cause (source-level dup, fixed 2026-06-23):** the same `duplicate symbol` error can come from TWO different `.hip` SOURCE files defining the same `extern "C"` launcher. Found `launch_bitonic_step_idx` defined in BOTH `catboost.hip` (kernel `bitonic_step_idx_kernel_cb`) and `scan.hip` (kernel `bitonic_step_idx_kernel`) — byte-identical kernels (only local var names differed). Fix is NOT deleting stale .o — it's removing the redundant SOURCE definition (kept scan.hip's, deleted catboost.hip's; `reductions.rs` binds the one symbol). Only bit gpu-core TEST binaries (which pull both .o); main binaries arbitrarily link-resolved one (fragile). After the fix `cargo test --workspace --no-run` links clean (38 binaries). So: dup symbol → check BOTH (a) stale old-named .o in OUT_DIR and (b) genuine two-source-file definition.
