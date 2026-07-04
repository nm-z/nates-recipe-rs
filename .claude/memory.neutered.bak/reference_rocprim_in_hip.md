---
name: reference_rocprim_in_hip
description: "Using rocPRIM device primitives inside a .hip kernel file (radix sort, segmented reduce) — required includes + the caller-owned-temp convention"
metadata: 
  node_type: memory
  type: reference
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

To call rocPRIM device functions (`rocprim::radix_sort_pairs`, `rocprim::segmented_reduce`, etc.) from a `gpu-core/src/kernels/*.hip` file:

- `#include <rocprim/rocprim.hpp>` **and** `#include <cstring>`. Without `<cstring>`, hipcc fails compiling `rocprim/iterator/texture_cache_iterator.hpp` with `no matching function for call to 'memset'`. `reduce.hip` is the working reference (it includes both).
- rocPRIM needs device temp storage. The codebase convention (see `reduce.hip`) is **caller-owned temp**, not `hipMalloc` inside the kernel: expose two `extern "C"` fns — a `*_workspace_bytes(...) -> size_t` that calls the primitive with `nullptr` temp to query the size, and a `launch_*(..., void* tmp, size_t tmp_bytes, ...)` that runs it. The Rust wrapper allocates a `GpuBuffer` for the temp and passes it (keeps the no-`hipMalloc`/`hipFree` rule, see [[feedback_hipmallocasync_required]]).
- rocPRIM's two-array overload `radix_sort_pairs(temp, bytes, keys_in, keys_out, vals_in, vals_out, n, begin_bit, end_bit, stream)` writes sorted results to the `*_out` arrays. f64 keys are supported (handles float bit-ordering). Used in `catboost.rs::gpu_random_permutation` (replaced the O(n log²n) bitonic argsort).

Many gpu-core ML prototype fns (`gpu_smo_train`, `gpu_kernel_matrix`, `gpu_fixed_radius_neighbors`, `gpu_random_permutation`) have **no callers in the main crate** — they're standalone GPU prototypes, so their signatures can be changed freely.
