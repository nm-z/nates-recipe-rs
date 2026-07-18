#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"

extern "C" __global__ void ropex_qk_kernel(
    const double* q, const double* k, const double* pos,
    double* qo, double* ko, int n_rows, int d, const double* theta_p
) {
    double theta = *theta_p;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int half = d / 2;
    if (idx >= n_rows * half) return;
    int row = idx / half;
    int i = idx % half;
    double p = (double)(int)pos[row];
    double angle = p / pow(theta, (2.0 * (double)i) / (double)d);
    double c = cos(angle);
    double s = sin(angle);
    long base = (long)row * d;
    double qa = q[base + i];
    double qb = q[base + i + half];
    qo[base + i]        = qa * c - qb * s;
    qo[base + i + half] = qb * c + qa * s;
    double ka = k[base + i];
    double kb = k[base + i + half];
    ko[base + i]        = ka * c - kb * s;
    ko[base + i + half] = kb * c + ka * s;
}
extern "C" void launch_ropex_qk(
    const double* q, const double* k, const double* pos,
    double* qo, double* ko, int n_rows, int d, const double* theta, hipStream_t s
) {
    int total = n_rows * (d / 2);
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(ropex_qk_kernel, grid, block, 0, s, q, k, pos, qo, ko, n_rows, d, theta);
}

template<typename T> __global__ void ropex_qk_heads_kernel(
    T* q, T* k, int m, int d, int heads, int seq, const T* theta_p, const T* sgn_p
) {
    T theta = *theta_p;
    T sgn = *sgn_p;
    int hd = d / heads;
    int half = hd / 2;
    int total = m * heads * half;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= total) return;
    int j = idx % half;
    int t = idx / half;
    int h = t % heads;
    int row = t / heads;
    int pos = row % seq;
    T angle = (T)pos / pow(theta, (T)(2.0 * (double)j) / (T)hd);
    T c = cos(angle);
    T s = sgn * sin(angle);
    long base = (long)row * d + (long)h * hd;
    T qa = q[base + j], qb = q[base + j + half];
    q[base + j]        = qa * c - qb * s;
    q[base + j + half] = qb * c + qa * s;
    T ka = k[base + j], kb = k[base + j + half];
    k[base + j]        = ka * c - kb * s;
    k[base + j + half] = kb * c + ka * s;
}
extern "C" void launch_ropex_qk_heads(
    void* q, void* k, int m, int d, int heads, int seq, const void* theta, const void* sgn, hipStream_t st, int dtype
) {
    int half = (d / heads) / 2;
    if (half == 0) return;
    int total = m * heads * half;
    dim3 block(256); dim3 grid((total + 255) / 256);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(ropex_qk_heads_kernel<T>, grid, block, 0, st,
        (T*)q, (T*)k, m, d, heads, seq, (const T*)theta, (const T*)sgn));
}
