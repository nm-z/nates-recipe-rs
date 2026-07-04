---
name: ROCm setup details
description: How ROCm/HIP is installed and configured on this machine
type: reference
originSessionId: 36551121-db6d-49ea-896e-8ae13f19d2f0
---
- ROCm 7.2.1 installed via `pacman -S rocm-hip-sdk rocm-device-libs hip-runtime-amd`
- `/opt/rocm` is a symlink to `/home/nate/.rocm-install/rocm` (root partition too small for 26GB install)
- hipcc was renamed to `amdclang++` in ROCm 7.x — use `/opt/rocm/bin/amdclang++` with `-x hip` flag
- Must pass `--rocm-path=/opt/rocm` to amdclang++ or it can't find device libs
- GPU arch for RX 7700/7800 XT: `gfx1101`
- GPU busy percent readable from `/sys/class/drm/card1/device/gpu_busy_percent`
- `rocm-smi` available for GPU monitoring
