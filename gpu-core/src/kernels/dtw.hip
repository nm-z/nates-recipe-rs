#include <hip/hip_runtime.h>

// ── DTW anti-diagonal kernel ───────────────────────────────────────────────
// Compute one anti-diagonal of the DTW DP table in parallel.
// Anti-diagonal d contains cells (i, j) where i + j == d, with
// 1 ≤ i ≤ m, 1 ≤ j ≤ n (1-indexed DP table with border row/col = +inf).
//
// dp is (m+1) x (n+1), row-major. dp[0][0] = 0, dp[0][j>0] = inf, dp[i>0][0] = inf.
// cost is (m) x (n), row-major. cost[i][j] = |a_i - b_j| or squared distance.
//
// Each element on anti-diagonal d only depends on (i-1,j), (i,j-1), (i-1,j-1)
// which are all from anti-diagonals d-1 and d-2 — already computed.

__global__ void dtw_antidiag_kernel(
    const double* cost, double* dp,
    int m, int n, int d  // d = anti-diagonal index, 0-based (d ranges 0..m+n-2)
) {
    int k = blockIdx.x * blockDim.x + threadIdx.x;
    // Map k to (i, j) on this anti-diagonal
    // d = (i-1) + (j-1) => i + j = d + 2, with 1 ≤ i ≤ m, 1 ≤ j ≤ n
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

// ── DTW init kernel ────────────────────────────────────────────────────────
// Fill the DP table border: dp[0][0] = 0, every other cell = +inf.
// Replaces the former host-buffer upload init (no HIP memory API here).
__global__ void dtw_init_kernel(double* dp, int dp_size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= dp_size) return;
    dp[idx] = (idx == 0) ? 0.0 : 1e308;
}

// Single launch: initialize the DP table on device.
// Caller allocates dp ((m+1)*(n+1) doubles) and passes dp_size = (m+1)*(n+1).
extern "C" void launch_dtw_init(
    double* dp, int dp_size, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((dp_size + 255) / 256);
    hipLaunchKernelGGL(dtw_init_kernel, gr, bl, 0, stream, dp, dp_size);
}

// Single launch: compute one anti-diagonal d of the DP table.
// Caller loops d from 0 to m+n-2 after calling launch_dtw_init once.
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
