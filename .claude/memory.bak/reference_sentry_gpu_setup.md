---
name: Sentry GPU benchmark setup
description: GPU setup on sentry host — Tesla M60 CUDA, AMD V340 ROCm, build toolchains, patched binaries
type: reference
originSessionId: 1e3cdfaa-f09b-45eb-bed2-fa74d31661a6
---
## Hardware
- 2x Tesla M60 (Maxwell GM204, sm_52), driver 470.256.02
- 1x AMD Radeon Pro V340 (Vega, gfx900), ROCm
- 20-core Xeon, 46GB RAM

## CUDA Stack
- nvcc 13.0 at `/opt/cuda/` — does NOT support sm_52 (dropped in CUDA 12)
- nvcc 11.7 at `/opt/cuda-11.7/` — supports sm_52, use with gcc-11 as host compiler
- Host compilers: gcc-11, gcc-13, gcc-14, gcc-15 (system), clang 22 (system)
- Working combo for M60: `nvcc 11.7 + g++-11 + -arch=sm_52`
- Use clang as CUDA compiler for projects that support it (xgboost does)

## OpenCL
- `libnvidia-opencl.so.470.256.02` extracted from NVIDIA driver .run installer, installed to `/usr/lib/`
- `opencl-nvidia` pacman package (595) does NOT work with driver 470 — must use extracted 470 lib
- `clinfo` shows: NVIDIA CUDA platform, OpenCL 3.0, OpenCL C 1.2
- LightGBM GPU OpenCL kernels fail to compile on this driver (silent build failure, OpenCL C 1.2 limitation)

## Patched Binaries
- CatBoost 1.2.10 GPU: binary-patched `.version 7.8` → `.version 7.4` in `_catboost.so` PTX sections (386 occurrences). Driver 470 JITs sm_50 PTX for sm_52. Works.
- XGBoost 2.0.3 GPU: built from source at `~/Desktop/bench/xgboost/`. Patched `CMakeLists.txt:159` to use `/usr/bin/g++-11` as CUDA host compiler. `dmlc-core/CMakeLists.txt` cmake_minimum_required bumped to 3.18. Works.
- LightGBM 4.6.0 GPU: built from source at `~/Desktop/bench/LightGBM/`. Patched `sha1.hpp` for Boost 1.89 (`unsigned int[5]` → `unsigned char[20]`), removed `system` from Boost find_package. Builds but OpenCL kernel compile fails at runtime.

## Custom Shims (~/bin/)
- `cudart_unified.c` — LM Studio GPU unification shim (wraps device enumeration for CUDA+AMD)
- `nvml_shim.c`, `nvml_unified.c` — NVML wrappers
- `force_hip_backend.so` — forces HIP backend

## Benchmark Env
- Python venv: `/tmp/bench_venv/` (survives reboot if /tmp is persistent, but may be wiped)
- Persistent build dir: `~/Desktop/bench/` — use this, not /tmp
- CatBoost benchmark repo: clone to `~/Desktop/bench/` from github.com/catboost/catboost
- Datasets: need re-download after /tmp wipe. Store in `~/Desktop/bench/datasets/`

## Key Gotcha
- Never say CUDA version mismatch prevents GPU use. Compile targeting the right ISA (sm_52), link against a compatible runtime. The driver executes sm_52 — always has.
