#include <hip/hip_runtime.h>

// ── Embedding-category GPU kernels (collision-safe embeddingx_ / launch_embeddingx_) ─
//
// embeddingx_lookup   : gather rows of (V,D) f64 table by i32 indices -> (N,D)
// embeddingx_bag      : pooled lookup (sum/mean) over variable bags via i32 offsets -> (B,D)
// embeddingx_one_hot  : i32 class ids (N) -> f64 one-hot (N,C)
// embeddingx_rope     : NeoX-style rotary position embedding forward (x,cos,sin)->y
// embeddingx_rope_bwd : RoPE backward; transpose of the forward rotation (dy,cos,sin)->dx

// ── embedding lookup: out[i,j] = table[indices[i], j] ───────────────────────
extern "C" __global__ void embeddingx_lookup_kernel(
    const double* table, const int* indices, double* out, int n, int d
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * d) return;
    int i = idx / d;
    int j = idx % d;
    int row = indices[i];
    out[idx] = table[(long)row * d + j];
}
extern "C" void launch_embeddingx_lookup(
    const double* table, const int* indices, double* out, int n, int d, hipStream_t s
) {
    int total = n * d;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(embeddingx_lookup_kernel, grid, block, 0, s, table, indices, out, n, d);
}

// ── embedding bag: pool rows[offsets[b]..offsets[b+1]) into out[b,:] ─────────
// mode 0 = sum, mode 1 = mean. One thread per (bag, dim) output element.
extern "C" __global__ void embeddingx_bag_kernel(
    const double* table, const int* indices, const int* offsets,
    double* out, int n_bags, int d, int mode
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_bags * d) return;
    int b = idx / d;
    int j = idx % d;
    int start = offsets[b];
    int end   = offsets[b + 1];
    double acc = 0.0;
    for (int k = start; k < end; ++k) {
        int row = indices[k];
        acc += table[(long)row * d + j];
    }
    int count = end - start;
    if (mode == 1 && count > 0) acc /= (double)count;
    out[idx] = acc;
}
extern "C" void launch_embeddingx_bag(
    const double* table, const int* indices, const int* offsets,
    double* out, int n_bags, int d, int mode, hipStream_t s
) {
    int total = n_bags * d;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(embeddingx_bag_kernel, grid, block, 0, s, table, indices, offsets, out, n_bags, d, mode);
}

// ── one-hot: out[i, indices[i]] = 1, else 0 ─────────────────────────────────
extern "C" __global__ void embeddingx_one_hot_kernel(
    const int* indices, double* out, int n, int c
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * c) return;
    int i = idx / c;
    int j = idx % c;
    out[idx] = (indices[i] == j) ? 1.0 : 0.0;
}
extern "C" void launch_embeddingx_one_hot(
    const int* indices, double* out, int n, int c, hipStream_t s
) {
    int total = n * c;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(embeddingx_one_hot_kernel, grid, block, 0, s, indices, out, n, c);
}

// ── RoPE forward (NeoX half-split convention) ───────────────────────────────
// x:(n,d) row-major. cos,sin:(n, d/2) per-position rotation factors.
// Pair element k in [0,d/2) with element k+d/2:
//   y[k]      = x[k]*cos[k] - x[k+d/2]*sin[k]
//   y[k+d/2]  = x[k+d/2]*cos[k] + x[k]*sin[k]
extern "C" __global__ void embeddingx_rope_kernel(
    const double* x, const double* cosb, const double* sinb, double* y, int n, int d
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int half = d / 2;
    if (idx >= n * half) return;
    int i = idx / half;          // position row
    int k = idx % half;          // pair index
    long base = (long)i * d;
    double a = x[base + k];
    double b = x[base + k + half];
    double c = cosb[(long)i * half + k];
    double s = sinb[(long)i * half + k];
    y[base + k]        = a * c - b * s;
    y[base + k + half] = b * c + a * s;
}
extern "C" void launch_embeddingx_rope(
    const double* x, const double* cosb, const double* sinb, double* y, int n, int d, hipStream_t s
) {
    int total = n * (d / 2);
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(embeddingx_rope_kernel, grid, block, 0, s, x, cosb, sinb, y, n, d);
}

// ── RoPE backward: dx = R^T dy (rotation by -theta) ─────────────────────────
//   dx[k]      = dy[k]*cos[k] + dy[k+d/2]*sin[k]
//   dx[k+d/2]  = dy[k+d/2]*cos[k] - dy[k]*sin[k]
extern "C" __global__ void embeddingx_rope_bwd_kernel(
    const double* dy, const double* cosb, const double* sinb, double* dx, int n, int d
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int half = d / 2;
    if (idx >= n * half) return;
    int i = idx / half;
    int k = idx % half;
    long base = (long)i * d;
    double ga = dy[base + k];
    double gb = dy[base + k + half];
    double c = cosb[(long)i * half + k];
    double s = sinb[(long)i * half + k];
    dx[base + k]        = ga * c + gb * s;
    dx[base + k + half] = gb * c - ga * s;
}
extern "C" void launch_embeddingx_rope_bwd(
    const double* dy, const double* cosb, const double* sinb, double* dx, int n, int d, hipStream_t s
) {
    int total = n * (d / 2);
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(embeddingx_rope_bwd_kernel, grid, block, 0, s, dy, cosb, sinb, dx, n, d);
}
