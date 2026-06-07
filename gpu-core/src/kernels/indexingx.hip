#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>

// ── indexing-category GPU kernels (collision-safe indexingx_ / launch_indexingx_) ─
// All compute in f64. Index inputs are i32 (int*). Oracles = CPU.

// index_select / take (gather by i32 indices, row-major [rows,cols]) ──────────
// out[i,:] = src[idx[i], :]   (i in [0,n), cols-wide rows)
extern "C" __global__ void indexingx_index_select_kernel(
    const double* src, const int* idx, double* out, int n, int cols
) {
    int t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= n * cols) return;
    int i = t / cols, j = t % cols;
    out[t] = src[idx[i] * cols + j];
}
extern "C" void launch_indexingx_index_select(
    const double* src, const int* idx, double* out, int n, int cols, hipStream_t s
) {
    int total = n * cols;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(indexingx_index_select_kernel, g, b, 0, s, src, idx, out, n, cols);
}

// where(cond, a, b): out[i] = cond[i] != 0 ? a[i] : b[i] ─────────────────────
extern "C" __global__ void indexingx_where_kernel(
    const double* cond, const double* a, const double* b, double* out, int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = (cond[i] != 0.0) ? a[i] : b[i];
}
extern "C" void launch_indexingx_where(
    const double* cond, const double* a, const double* b, double* out, int n, hipStream_t s
) {
    dim3 b_(256); dim3 g((n + 255) / 256);
    hipLaunchKernelGGL(indexingx_where_kernel, g, b_, 0, s, cond, a, b, out, n);
}

// index_add: out (copy of base, [rows,cols]); out[idx[i],:] += src[i,:] ──────
// Accumulates with atomicAdd so duplicate indices sum correctly.
extern "C" __global__ void indexingx_index_add_kernel(
    double* out, const int* idx, const double* src, int n, int cols
) {
    int t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= n * cols) return;
    int i = t / cols, j = t % cols;
    atomicAdd(&out[idx[i] * cols + j], src[t]);
}
extern "C" void launch_indexingx_index_add(
    double* out, const int* idx, const double* src, int n, int cols, hipStream_t s
) {
    int total = n * cols;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(indexingx_index_add_kernel, g, b, 0, s, out, idx, src, n, cols);
}

// diagonal (extract main diagonal of [n,n] matrix): out[i] = m[i*n+i] ────────
extern "C" __global__ void indexingx_diagonal_kernel(const double* m, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = m[i * n + i];
}
extern "C" void launch_indexingx_diagonal(const double* m, double* out, int n, hipStream_t s) {
    dim3 b(256); dim3 g((n + 255) / 256);
    hipLaunchKernelGGL(indexingx_diagonal_kernel, g, b, 0, s, m, out, n);
}

// take_along_axis (torch.gather along last dim): src [rows,cols], idx [rows,k] ─
// out[i,j] = src[i, idx[i,j]]   (j in [0,k))
extern "C" __global__ void indexingx_take_along_axis_kernel(
    const double* src, const int* idx, double* out, int rows, int cols, int k
) {
    int t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= rows * k) return;
    int i = t / k, j = t % k;
    out[t] = src[i * cols + idx[t]];
}
extern "C" void launch_indexingx_take_along_axis(
    const double* src, const int* idx, double* out, int rows, int cols, int k, hipStream_t s
) {
    int total = rows * k;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(indexingx_take_along_axis_kernel, g, b, 0, s, src, idx, out, rows, cols, k);
}

// tril (lower triangle of [n,n], zero above main diag): out[i,j]= i>=j? m:0 ──
extern "C" __global__ void indexingx_tril_kernel(const double* m, double* out, int n) {
    int t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= n * n) return;
    int i = t / n, j = t % n;
    out[t] = (i >= j) ? m[t] : 0.0;
}
extern "C" void launch_indexingx_tril(const double* m, double* out, int n, hipStream_t s) {
    int total = n * n;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(indexingx_tril_kernel, g, b, 0, s, m, out, n);
}

// triu (upper triangle of [n,n], zero below main diag): out[i,j]= i<=j? m:0 ──
extern "C" __global__ void indexingx_triu_kernel(const double* m, double* out, int n) {
    int t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= n * n) return;
    int i = t / n, j = t % n;
    out[t] = (i <= j) ? m[t] : 0.0;
}
extern "C" void launch_indexingx_triu(const double* m, double* out, int n, hipStream_t s) {
    int total = n * n;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(indexingx_triu_kernel, g, b, 0, s, m, out, n);
}

// masked_select via hipcub::DeviceSelect::Flagged ───────────────────────────
// in [n], flags [n] (u8: nonzero=keep); out [n] (worst-case), num_out written
// to d_num (1 int). Caller allocates d_temp (size from _workspace_bytes) and
// reads d_num back to host after syncing the stream.
extern "C" size_t indexingx_masked_select_workspace_bytes(int n) {
    void* d_temp = nullptr;
    size_t temp_bytes = 0;
    hipcub::DeviceSelect::Flagged(
        d_temp, temp_bytes,
        (const double*)nullptr, (const unsigned char*)nullptr,
        (double*)nullptr, (int*)nullptr, n, (hipStream_t)0);
    return temp_bytes;
}
extern "C" void launch_indexingx_masked_select(
    const double* in, const unsigned char* flags, double* out, int* d_num,
    void* d_temp, size_t temp_bytes, int n, hipStream_t s
) {
    hipcub::DeviceSelect::Flagged(d_temp, temp_bytes, in, flags, out, d_num, n, s);
}
