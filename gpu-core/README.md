# gpu-core

`gpu-core` is the low-level HIP/ROCm compute crate for this repository.  
It provides:

- Raw HIP runtime bindings (`hip.rs`)
- GPU memory ownership + spill behavior (`memory.rs`)
- A large kernel/math API over rocBLAS/rocSOLVER + custom HIP kernels (`kernels.rs`)

## Architecture

### 1) FFI layer (`src/hip.rs`)

- Defines `HipError` and `check()` for HIP status-code handling.
- Exposes core HIP runtime symbols (`hipMalloc`, `hipFree`, `hipMemcpy`, streams/events, sync, mem info).
- Provides small safe wrappers like:
  - `mem_info()`
  - `device_synchronize()`
  - `set_device()`

### 2) Memory layer (`src/memory.rs`)

Core type: `GpuBuffer`

- Owns or borrows a GPU pointer (via `GpuBuffer::borrow(...)` vs owned allocations).
- Allocation paths:
  - `hipMalloc` for normal device allocation
  - `hipMallocManaged` fallback when memory pressure is high
- Includes simple adaptive spill behavior:
  - enters spill mode when usage is high
  - exits spill mode when pressure drops
- Optional GC callback hook (`set_gc_hook`) for external memory cleanup before retry.
- Upload/download helpers for `f64`, `f32`, and `u8`.
- RAII `Drop` frees owned device memory with `hipFree`.

### 3) Compute layer (`src/kernels.rs`)

This layer combines:

- rocBLAS kernels (e.g. GEMM, AXPY/SCAL-style ops)
- rocSOLVER routines (e.g. Cholesky factorization / solves)
- Custom HIP kernels from `src/kernels/*.hip`:
  - `elementwise.hip`
  - `reduce.hip`
  - `distance.hip`
  - `argsort.hip`
  - `tree.hip`
  - `dtw.hip`
  - `apriori.hip`

It keeps a **thread-local rocBLAS handle** and exposes `gpu_shutdown()` to release handles and reset the device.

## Build/link pipeline

`gpu-core/build.rs`:

- Compiles HIP sources via `/opt/rocm/bin/hipcc` (or `/opt/rocm/bin/amdclang++` fallback).
- Archives compiled objects into `libhipkernels.a`.
- Links against ROCm libs:
  - `amdhip64`
  - `rocblas`
  - `rocsolver`
  - `stdc++`

## Feature surface (high level)

`kernels.rs` exposes a broad GPU API including:

- Linear algebra & solves (`gpu_gemm*`, `gpu_linear*`, `gpu_cholesky*`, `gpu_solve`, `gpu_tri_solve`)
- Elementwise ops and activations (relu/sigmoid/tanh/gelu/silu/leaky_relu + backward variants)
- Normalization & regularization (layernorm/batchnorm/dropout + backward)
- Loss/softmax utilities (softmax/log-softmax/cross-entropy, CE gradients)
- Tensor/data transforms (transpose, concat, slicing, im2col/col2im, pooling, broadcast ops)
- Reductions/stat ops (sum/mean/var/min/max, log-sum-exp, prefix sums)
- Optimizer updates (`gpu_sgd_update`, `gpu_adam_update`, `gpu_adamw_update`, grad clipping)
- Distance/nearest-neighbor helpers (`gpu_pairwise_l2`, argsort/top-k/argmin/argmax)
- Sequence/model primitives (LSTM/GRU cell helpers)
- Tree/GBM-style kernels (histogram build, split eval, partition, tree build, oblivious-tree helpers)
- Specialized kernels (DTW, Apriori support/candidate generation, GPU RNG uniform/normal/bernoulli)

## TODO (robust crate improvements)

- [ ] Replace hard-coded ROCm include/lib paths and fixed `--offload-arch=gfx1101` with environment-driven config.
- [ ] Add feature flags for optional kernel groups (core math vs tree/mining extras) to reduce compile/link footprint.
- [ ] Introduce richer error mapping (rocBLAS/rocSOLVER/HIP domains) instead of mostly raw status codes.
- [ ] Add shape/stride metadata wrappers around `GpuBuffer` to prevent dimension mismatch at call-sites.
- [ ] Add optional stream-aware APIs (explicit stream ownership, async copies, overlap patterns).
- [ ] Add lightweight smoke/integration tests that can be skipped when ROCm is unavailable, with clearer diagnostics.
- [ ] Add a crate-level API reference table (inputs/outputs/dtypes) for frequently used kernel entry points.

## Notes

- The crate is intentionally low-level and performance-oriented.
- Most APIs assume contiguous row-major logical layout on the Rust side.
- Many operations are allocation-returning; there are also `_into` variants for preallocated outputs in hot paths.
