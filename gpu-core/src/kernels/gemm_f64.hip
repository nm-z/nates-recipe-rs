#include <hip/hip_runtime.h>

// ── custom f64 GEMM-BT (no hipBLAS) ──────────────────────────────────────────
// out(m,n) = A(m,k) . B(n,k)^T, all row-major f64. Shapes are bandwidth-bound:
// m is tiny (1..54), n/k are the big streamed dims. One block per B row (n
// blocks total, always >> 54 CUs for every real shape) stages that row into
// LDS once (the only global read of B — coalesced, vectorized double2 where k
// is even), then every warp in the block independently computes the dot
// product against a distinct subset of A's m rows (round-robin over warps) via
// warp-shuffle reduction. A is tiny (<=1.2MB across all shapes) and reread from
// global once per block, but stays resident in L2/Infinity Cache after warm-up.
// No atomics anywhere -> fixed shuffle reduction order -> bit-identical reruns.

extern "C" __global__ void gemm_bt_f64(
    const double* __restrict__ A,   // (m,k) row-major
    const double* __restrict__ B,   // (n,k) row-major
    double* __restrict__ C,         // (m,n) row-major
    int m, int n, int k
) {
    extern __shared__ double bs[];   // k doubles: this block's staged B row
    int j = blockIdx.x;
    const double* brow = B + (long)j * k;

    if ((k & 1) == 0) {
        const double2* brow2 = reinterpret_cast<const double2*>(brow);
        double2* bs2 = reinterpret_cast<double2*>(bs);
        int half = k / 2;
        for (int i = threadIdx.x; i < half; i += blockDim.x) bs2[i] = brow2[i];
    } else {
        for (int i = threadIdx.x; i < k; i += blockDim.x) bs[i] = brow[i];
    }
    __syncthreads();

    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;

    for (int i = wid; i < m; i += nwarps) {
        const double* arow = A + (long)i * k;
        double part = 0.0;
        for (int x = lane; x < k; x += warpSize) part += arow[x] * bs[x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            part += __shfl_down(part, offset);
        if (lane == 0) C[(long)i * n + j] = part;
    }
}

extern "C" void launch_gemm_bt_f64(
    const double* a, const double* b, double* c, int m, int n, int k, hipStream_t s
) {
    int threads = 256;
    size_t shmem = (size_t)k * sizeof(double);
    hipLaunchKernelGGL(gemm_bt_f64, dim3((unsigned)n), dim3(threads), shmem, s, a, b, c, m, n, k);
}

// x[i] *= scalar, elementwise. Replaces hipblasDscal.
extern "C" __global__ void scale_f64(double* x, double scalar, long n) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) x[i] *= scalar;
}
extern "C" void launch_scale_f64(double* x, double scalar, long n, hipStream_t s) {
    int t = 256;
    long blk = (n + t - 1) / t;
    hipLaunchKernelGGL(scale_f64, dim3((unsigned)blk), dim3(t), 0, s, x, scalar, n);
}
