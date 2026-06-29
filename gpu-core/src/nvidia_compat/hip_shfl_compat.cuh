// Force-included (-include) when compiling the rocprim/hipcub kernels with
// plain nvcc on the NVIDIA backend. hipcc normally provides HIP's maskless
// warp-shuffle builtins (__shfl_down etc.); plain nvcc does not, so we map them
// onto CUDA's *_sync forms with a full mask. These kernels' reductions run with
// the whole warp converged (see the zero-padded block-reduce fix), so a full
// 0xffffffff mask is correct.
#pragma once
#ifndef HIP_SHFL_COMPAT_CUH
#define HIP_SHFL_COMPAT_CUH

template <class T>
__device__ __forceinline__ T __shfl_down(T v, unsigned int delta, int width = warpSize) {
      return __shfl_down_sync(0xffffffffu, v, delta, width);
}
template <class T>
__device__ __forceinline__ T __shfl_up(T v, unsigned int delta, int width = warpSize) {
      return __shfl_up_sync(0xffffffffu, v, delta, width);
}
template <class T>
__device__ __forceinline__ T __shfl_xor(T v, int mask, int width = warpSize) {
      return __shfl_xor_sync(0xffffffffu, v, mask, width);
}
template <class T>
__device__ __forceinline__ T __shfl(T v, int src, int width = warpSize) {
      return __shfl_sync(0xffffffffu, v, src, width);
}

#endif
