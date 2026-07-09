#include <hip/hip_runtime.h>


__global__ void dtw_antidiag_kernel(
    const double* cost, double* dp,
    int m, int n, int d
) {
    int k = blockIdx.x * blockDim.x + threadIdx.x;
    int i = k + max(1, d + 2 - n);
    int j = d + 2 - i;
    if (i < 1 || i > m || j < 1 || j > n) return;

    int stride = n + 1;
    double c = cost[(i-1) * n + (j-1)];
    double top  = dp[(i-1) * stride + j];
    double left = dp[i * stride + (j-1)];
    double diag = dp[(i-1) * stride + (j-1)];
    dp[i * stride + j] = c + fmin(fmin(top, left), diag);
}

__global__ void dtw_init_kernel(double* dp, int dp_size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= dp_size) return;
    dp[idx] = (idx == 0) ? 0.0 : 1e308;
}

extern "C" void launch_dtw_init(
    double* dp, int dp_size, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((dp_size + 255) / 256);
    hipLaunchKernelGGL(dtw_init_kernel, gr, bl, 0, stream, dp, dp_size);
}

extern "C" void launch_dtw_antidiag(
    const double* cost, double* dp,
    int m, int n, int d, hipStream_t stream
) {
    int i_min = max(1, d + 2 - n);
    int i_max = min(m, d + 1);
    int count = i_max - i_min + 1;
    if (count <= 0) return;
    dim3 bl(256); dim3 gr((count + 255) / 256);
    hipLaunchKernelGGL(dtw_antidiag_kernel, gr, bl, 0, stream,
        cost, dp, m, n, d);
}
