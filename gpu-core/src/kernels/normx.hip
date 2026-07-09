#include <hip/hip_runtime.h>


__device__ __forceinline__ double block_sum_bcast(double val, double* smem) {
    int lane = threadIdx.x % warpSize;
    int wid  = threadIdx.x / warpSize;
    for (int off = warpSize / 2; off > 0; off /= 2) val += __shfl_down(val, off);
    if (lane == 0) smem[wid] = val;
    __syncthreads();
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;
    if (wid == 0) {
        double v = (lane < nwarps) ? smem[lane] : 0.0;
        for (int off = warpSize / 2; off > 0; off /= 2) v += __shfl_down(v, off);
        if (lane == 0) smem[0] = v;
    }
    __syncthreads();
    double total = smem[0];
    __syncthreads();
    return total;
}

extern "C" __global__ void normx_groupnorm_kernel(
    const double* x, double* out, const double* gamma, const double* beta,
    int N, int C, int L, int G, double eps
) {
    int inst = blockIdx.x;
    int n = inst / G;
    int g = inst % G;
    int cpg = C / G;
    int m = cpg * L;
    long base = (long)n * C * L + (long)g * cpg * L;

    __shared__ double smem[32];
    double sum = 0.0;
    for (int i = threadIdx.x; i < m; i += blockDim.x) sum += x[base + i];
    double mean = block_sum_bcast(sum, smem) / m;

    double vacc = 0.0;
    for (int i = threadIdx.x; i < m; i += blockDim.x) { double d = x[base + i] - mean; vacc += d * d; }
    double inv_std = 1.0 / sqrt(block_sum_bcast(vacc, smem) / m + eps);

    for (int i = threadIdx.x; i < m; i += blockDim.x) {
        int c = g * cpg + i / L;
        double normed = (x[base + i] - mean) * inv_std;
        out[base + i] = gamma ? normed * gamma[c] + beta[c] : normed;
    }
}

extern "C" void launch_normx_groupnorm(
    const double* x, double* out, const double* gamma, const double* beta,
    int N, int C, int L, int G, double eps, hipStream_t s
) {
    int cpg = C / G;
    int threads = min(256, max(32, cpg * L));
    hipLaunchKernelGGL(normx_groupnorm_kernel, dim3(N * G), dim3(threads), 0, s,
        x, out, gamma, beta, N, C, L, G, eps);
}

extern "C" __global__ void normx_instancenorm_kernel(
    const double* x, double* out, const double* gamma, const double* beta,
    int N, int C, int L, double eps
) {
    int inst = blockIdx.x;
    int c = inst % C;
    long base = (long)inst * L;

    __shared__ double smem[32];
    double sum = 0.0;
    for (int i = threadIdx.x; i < L; i += blockDim.x) sum += x[base + i];
    double mean = block_sum_bcast(sum, smem) / L;

    double vacc = 0.0;
    for (int i = threadIdx.x; i < L; i += blockDim.x) { double d = x[base + i] - mean; vacc += d * d; }
    double inv_std = 1.0 / sqrt(block_sum_bcast(vacc, smem) / L + eps);

    for (int i = threadIdx.x; i < L; i += blockDim.x) {
        double normed = (x[base + i] - mean) * inv_std;
        out[base + i] = gamma ? normed * gamma[c] + beta[c] : normed;
    }
}

extern "C" void launch_normx_instancenorm(
    const double* x, double* out, const double* gamma, const double* beta,
    int N, int C, int L, double eps, hipStream_t s
) {
    int threads = min(256, max(32, L));
    hipLaunchKernelGGL(normx_instancenorm_kernel, dim3(N * C), dim3(threads), 0, s,
        x, out, gamma, beta, N, C, L, eps);
}

extern "C" __global__ void normx_l2_normalize_kernel(
    const double* x, double* out, int rows, int cols, double eps
) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = x + (long)row * cols;
    double* o = out + (long)row * cols;

    __shared__ double smem[32];
    double ss = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) { double v = r[j]; ss += v * v; }
    double sumsq = block_sum_bcast(ss, smem);
    double denom = sqrt(sumsq > eps ? sumsq : eps);
    double inv = 1.0 / denom;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) o[j] = r[j] * inv;
}

extern "C" void launch_normx_l2_normalize(
    const double* x, double* out, int rows, int cols, double eps, hipStream_t s
) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(normx_l2_normalize_kernel, dim3(rows), dim3(threads), 0, s,
        x, out, rows, cols, eps);
}

extern "C" __global__ void normx_rmsnorm_kernel(
    const double* x, double* out, const double* gamma, int rows, int cols, const double* eps_p
) {
    double eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = x + (long)row * cols;
    double* o = out + (long)row * cols;

    __shared__ double smem[32];
    double ss = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) { double v = r[j]; ss += v * v; }
    double ms = block_sum_bcast(ss, smem) / cols;
    double inv = 1.0 / sqrt(ms + eps);
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        o[j] = gamma ? r[j] * inv * gamma[j] : r[j] * inv;
}

extern "C" void launch_normx_rmsnorm(
    const double* x, double* out, const double* gamma, int rows, int cols, const double* eps, hipStream_t s
) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(normx_rmsnorm_kernel, dim3(rows), dim3(threads), 0, s,
        x, out, gamma, rows, cols, eps);
}
