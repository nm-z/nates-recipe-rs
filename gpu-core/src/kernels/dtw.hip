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

// Full DTW: initialize DP table, then loop m+n-1 anti-diagonal launches.
// Returns the completed DP table; the DTW distance is dp[m][n].
extern "C" void launch_dtw(
    const double* cost, double* dp,
    int m, int n, hipStream_t stream
) {
    int stride = n + 1;
    int dp_size = (m + 1) * stride;

    // Init: dp[0][0] = 0, rest of border = +inf
    // Fill entire dp with inf, then set dp[0][0] = 0
    // Use a small kernel to fill
    double inf_val = 1e308;

    // Fill dp with inf
    {
        // Reuse fill pattern: one thread per element
        int total = dp_size;
        dim3 bl(256); dim3 gr((total + 255) / 256);
        // Inline fill kernel — set all to inf
        hipMemsetAsync(dp, 0, dp_size * sizeof(double), stream);
    }
    // Actually, hipMemset sets bytes to 0, which gives 0.0. We need inf.
    // Use a device-side fill. We'll launch a trivial kernel.
    // For simplicity: fill on host and upload? No, dp is already device memory.
    // Use the existing fill kernel from elementwise.hip via a function pointer? No.
    // Just write a local fill kernel here.

    // Workaround: fill dp with inf, then zero dp[0][0]
    // We'll do it with two hipMemcpy calls from host.
    double zero = 0.0;
    // First set all dp to inf using a simple kernel
    hipStreamSynchronize(stream);
    std::vector<double> init(dp_size, inf_val);
    init[0] = 0.0;
    hipMemcpy(dp, init.data(), dp_size * sizeof(double), hipMemcpyHostToDevice);

    // Launch one kernel per anti-diagonal
    int n_diags = m + n - 1;
    for (int d = 0; d < n_diags; d++) {
        // Number of cells on this anti-diagonal
        int i_min = max(1, d + 2 - n);
        int i_max = min(m, d + 1);
        int count = i_max - i_min + 1;
        if (count <= 0) continue;
        dim3 bl(256); dim3 gr((count + 255) / 256);
        hipLaunchKernelGGL(dtw_antidiag_kernel, gr, bl, 0, stream,
            cost, dp, m, n, d);
    }
}
