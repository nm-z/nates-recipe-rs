---
name: hip-atexit-doublefree
description: "Never call hipDeviceReset() in an atexit handler — double-frees with HIP's own __hip_module_dtor"
metadata: 
  node_type: memory
  type: project
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

Teardown heap corruption (`malloc(): unaligned tcache chunk detected`, `corrupted double-linked list`, SIGABRT or SIGSEGV — message varies by heap layout, so it's intermittent and size-dependent) was caused by `gpu_shutdown()` calling `hipDeviceReset()` from an `atexit` hook.

At process exit, atexit handlers run LIFO. libamdhip64 registers `__hip_module_dtor` early (on library load); our handler registers later → ours runs FIRST. `hipDeviceReset()` tears down the HIP context, then HIP's `__hip_module_dtor` frees the same structures again → double-free.

**Diagnosis method (worked):** `MALLOC_CHECK_=3 <bin>` forces glibc to abort at first corruption; `gdb -batch -ex run -ex bt <bin>` showed `exit() → __hip_module_dtor → libamdhip64 → libc free`. Build the cargo-script first (uncapped), then run the compiled binary directly under a short `timeout` — don't run the .rs under the timeout or the ~90s compile eats it.

**Rule:** never `hipDeviceReset()` at exit. The OS reclaims all VRAM when the process dies, so it's redundant; it only collides with HIP's own teardown. `gpu_shutdown()` may destroy the rocBLAS handle, nothing more. Fixed in commit 4195aa7. [[feedback_never_touch_processes]]
