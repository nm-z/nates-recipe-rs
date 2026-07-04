---
name: rocprof-timeout
description: Always wrap rocprofv3 in a strict ~8s timeout — it hangs at teardown
metadata: 
  node_type: memory
  type: reference
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

rocprofv3 reliably hangs/aborts at teardown on this box (`corrupted double-linked list`, signal-handler stalls). Never run it bare — wrap every invocation in a strict timeout (~8s for short runs) so it can't sit blocking all day.

`timeout 8 rocprofv3 --hip-trace --kernel-trace --memory-allocation-trace -d <dir> -- <binary>`

The SQLite trace DB is written incrementally, so a killed-on-timeout run still leaves a queryable `<dir>/engi/<pid>_results.db`. Flags: `--hip-trace` = HIP API regions (memcpy direction/size, hipMemGetInfo, syncs); `--kernel-trace` = kernel dispatches (needed for GEMM/kernel counts + durations); `--memory-allocation-trace` = alloc sizes (zero-byte allocs). `--hip-trace` alone leaves the `kernels` table empty. [[reference_rocm_setup]]
