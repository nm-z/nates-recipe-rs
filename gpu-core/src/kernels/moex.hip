#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"


template<typename T> __global__ void moex_weighted_accumulate_kernel(
    const T* ye, const T* gate, T* out,
    int n, int d, int n_experts, int e
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * d) return;
    int i = idx / d;
    T g = gate[(long)i * n_experts + e];
    out[idx] += g * ye[idx];
}
extern "C" void launch_moex_weighted_accumulate(
    const void* ye, const void* gate, void* out,
    int n, int d, int n_experts, int e, hipStream_t s, int dtype
) {
    int total = n * d;
    dim3 block(256); dim3 grid((total + 255) / 256);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(moex_weighted_accumulate_kernel<T>, grid, block, 0, s,
        (const T*)ye, (const T*)gate, (T*)out, n, d, n_experts, e));
}

extern "C" __global__ void moex_dye_kernel(
    const double* d_out, const double* gate, double* d_ye,
    int n, int d, int n_experts, int e
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * d) return;
    int i = idx / d;
    d_ye[idx] = gate[(long)i * n_experts + e] * d_out[idx];
}
extern "C" __global__ void moex_dgate_kernel(
    const double* d_out, const double* ye, double* d_gate,
    int n, int d, int n_experts, int e
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double acc = 0.0;
    long base = (long)i * d;
    for (int j = 0; j < d; j++) acc += ye[base + j] * d_out[base + j];
    d_gate[(long)i * n_experts + e] = acc;
}
extern "C" void launch_moex_weighted_accumulate_backward(
    const double* d_out, const double* gate, const double* ye,
    double* d_ye, double* d_gate, int n, int d, int n_experts, int e, hipStream_t s
) {
    int total = n * d;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(moex_dye_kernel, grid, block, 0, s, d_out, gate, d_ye, n, d, n_experts, e);
    dim3 grid_n((n + 255) / 256);
    hipLaunchKernelGGL(moex_dgate_kernel, grid_n, block, 0, s, d_out, ye, d_gate, n, d, n_experts, e);
}
