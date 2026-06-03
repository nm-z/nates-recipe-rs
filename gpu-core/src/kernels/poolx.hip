#include <hip/hip_runtime.h>

// ── Pool category gap kernels, f64 (collision-safe poolx_ / launch_poolx_ names) ─
// Layout convention mirrors existing pool kernels:
//   *_1d  : input laid out (rows * span, channels), pooled along the span dim,
//           producing (rows, channels). Matches avg_pool_1d's pre-windowed scheme.

// ── Global average pool 1D ──────────────────────────────────────────────────
// out[r,ch] = mean over span of input[(r*span+t)*channels+ch]
extern "C" __global__ void poolx_global_avg_pool_kernel(
    const double* x, double* out, int rows, int span, int channels
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * channels) return;
    int r = idx / channels;
    int ch = idx % channels;
    double sum = 0.0;
    for (int t = 0; t < span; t++) sum += x[(r * span + t) * channels + ch];
    out[idx] = sum / span;
}
extern "C" void launch_poolx_global_avg_pool(
    const double* x, double* out, int rows, int span, int channels, hipStream_t s
) {
    int total = rows * channels;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(poolx_global_avg_pool_kernel, g, b, 0, s, x, out, rows, span, channels);
}

// ── Global max pool 1D ──────────────────────────────────────────────────────
// out[r,ch] = max over span of input[(r*span+t)*channels+ch]
extern "C" __global__ void poolx_global_max_pool_kernel(
    const double* x, double* out, int rows, int span, int channels
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * channels) return;
    int r = idx / channels;
    int ch = idx % channels;
    double best = -1e308;
    for (int t = 0; t < span; t++) {
        double v = x[(r * span + t) * channels + ch];
        if (v > best) best = v;
    }
    out[idx] = best;
}
extern "C" void launch_poolx_global_max_pool(
    const double* x, double* out, int rows, int span, int channels, hipStream_t s
) {
    int total = rows * channels;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(poolx_global_max_pool_kernel, g, b, 0, s, x, out, rows, span, channels);
}

// ── Lp-norm pool 1D (global) ────────────────────────────────────────────────
// out[r,ch] = ( sum_t |x|^p )^(1/p)  over span. p=2 → L2 pool.
extern "C" __global__ void poolx_lp_pool_kernel(
    const double* x, double* out, int rows, int span, int channels, double p
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * channels) return;
    int r = idx / channels;
    int ch = idx % channels;
    double acc = 0.0;
    for (int t = 0; t < span; t++) {
        double v = x[(r * span + t) * channels + ch];
        acc += pow(fabs(v), p);
    }
    out[idx] = pow(acc, 1.0 / p);
}
extern "C" void launch_poolx_lp_pool(
    const double* x, double* out, int rows, int span, int channels, double p, hipStream_t s
) {
    int total = rows * channels;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(poolx_lp_pool_kernel, g, b, 0, s, x, out, rows, span, channels, p);
}

// ── Adaptive average pool 1D ────────────────────────────────────────────────
// Input length I along span, output length O. Each output bin o averages
// input[start..end) where start=floor(o*I/O), end=ceil((o+1)*I/O) (PyTorch).
// Layout: input (rows, I) row-major; output (rows, O) row-major.
extern "C" __global__ void poolx_adaptive_avg_pool_kernel(
    const double* x, double* out, int rows, int in_len, int out_len
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * out_len) return;
    int r = idx / out_len;
    int o = idx % out_len;
    int start = (int)((long)o * in_len / out_len);
    int end = (int)(((long)(o + 1) * in_len + out_len - 1) / out_len);  // ceil
    double sum = 0.0;
    for (int t = start; t < end; t++) sum += x[r * in_len + t];
    out[idx] = sum / (double)(end - start);
}
extern "C" void launch_poolx_adaptive_avg_pool(
    const double* x, double* out, int rows, int in_len, int out_len, hipStream_t s
) {
    int total = rows * out_len;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(poolx_adaptive_avg_pool_kernel, g, b, 0, s, x, out, rows, in_len, out_len);
}

// ── Adaptive max pool 1D ────────────────────────────────────────────────────
// Same bin layout as adaptive_avg, but max over the bin.
extern "C" __global__ void poolx_adaptive_max_pool_kernel(
    const double* x, double* out, int rows, int in_len, int out_len
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * out_len) return;
    int r = idx / out_len;
    int o = idx % out_len;
    int start = (int)((long)o * in_len / out_len);
    int end = (int)(((long)(o + 1) * in_len + out_len - 1) / out_len);  // ceil
    double best = -1e308;
    for (int t = start; t < end; t++) {
        double v = x[r * in_len + t];
        if (v > best) best = v;
    }
    out[idx] = best;
}
extern "C" void launch_poolx_adaptive_max_pool(
    const double* x, double* out, int rows, int in_len, int out_len, hipStream_t s
) {
    int total = rows * out_len;
    dim3 b(256); dim3 g((total + 255) / 256);
    hipLaunchKernelGGL(poolx_adaptive_max_pool_kernel, g, b, 0, s, x, out, rows, in_len, out_len);
}
