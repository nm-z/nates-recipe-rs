// hipcub → CUDA CUB shim for the NVIDIA backend.
//
// ROCm's bundled hipcub is version-skewed against the system CCCL (it includes
// CUB headers that newer CCCL removed, e.g. device_spmv.cuh), so instead of
// ROCm's hipcub we expose the hipcub:: device API directly on top of CUDA's
// CUB. hipcub is a thin name-compatible wrapper over CUB, so the whole device
// algorithm surface this project uses — DeviceRadixSort (SortKeys/SortPairs and
// the *Descending variants), DeviceRunLengthEncode::Encode, DeviceSelect
// (Unique/Flagged) — is reachable via `using namespace cub`.
#pragma once
#ifndef HIPCUB_NV_SHIM_HPP
#define HIPCUB_NV_SHIM_HPP

#include <cub/cub.cuh>

namespace hipcub {
using namespace cub;
}

#endif
