#include <hip/hip_runtime.h>

// ── fused single-pass metric reductions ──────────────────────────────────────
// No temp buffers, no HIP memory APIs. Each kernel streams pred/y once, reduces
// warp -> shared -> block, and atomicAdds one block partial into out[0].
// out is a single scalar; the caller pre-zeroes it (atomicAdd accumulates).
// Per-element contributions are pre-scaled so out holds the final metric after
// the launch: ss_res = Σ(y-p)², mse = Σ(p-y)²/n, accuracy = Σ[round(p)==round(y)]/n.

#define BLOCK 256

__device__ __forceinline__ double block_reduce_sum(double val) {
      for (int offset = warpSize / 2; offset > 0; offset /= 2)
            val += __shfl_down(val, offset);

      __shared__ double s[64];
      int lane = threadIdx.x % warpSize;
      int wid  = threadIdx.x / warpSize;
      if (lane == 0) s[wid] = val;
      __syncthreads();

      int nwarps = (blockDim.x + warpSize - 1) / warpSize;
      double total = 0.0;
      if (wid == 0) {
            double v = (lane < nwarps) ? s[lane] : 0.0;
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                  v += __shfl_down(v, offset);
            total = v;
      }
      return total;
}

// ── residual sum of squares: Σ (y - pred)² ───────────────────────────────────
__global__ void ss_res_kernel(const double* pred, const double* y, double* out, int n) {
      double acc = 0.0;
      for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < n; i += gridDim.x * blockDim.x) {
            double d = y[i] - pred[i];
            acc += d * d;
      }
      double block_sum = block_reduce_sum(acc);
      if (threadIdx.x == 0)
            atomicAdd(out, block_sum);
}

extern "C" void launch_ss_res(const double* pred, const double* y, double* out, int n, void* stream) {
      int blocks = (n + BLOCK - 1) / BLOCK;
      if (blocks < 1) blocks = 1;
      if (blocks > 65535) blocks = 65535;
      hipLaunchKernelGGL(ss_res_kernel, dim3(blocks), dim3(BLOCK), 0, (hipStream_t)stream,
                         pred, y, out, n);
}

// ── mean squared error: Σ (pred - y)² / n ────────────────────────────────────
__global__ void mse_kernel(const double* pred, const double* y, double* out, int n) {
      double inv_n = (n > 0) ? 1.0 / (double)n : 0.0;
      double acc = 0.0;
      for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < n; i += gridDim.x * blockDim.x) {
            double d = pred[i] - y[i];
            acc += d * d * inv_n;
      }
      double block_sum = block_reduce_sum(acc);
      if (threadIdx.x == 0)
            atomicAdd(out, block_sum);
}

extern "C" void launch_mse(const double* pred, const double* y, double* out, int n, void* stream) {
      int blocks = (n + BLOCK - 1) / BLOCK;
      if (blocks < 1) blocks = 1;
      if (blocks > 65535) blocks = 65535;
      hipLaunchKernelGGL(mse_kernel, dim3(blocks), dim3(BLOCK), 0, (hipStream_t)stream,
                         pred, y, out, n);
}

// ── classification accuracy: Σ [round(pred) == round(y)] / n ──────────────────
__global__ void accuracy_kernel(const double* pred, const double* y, double* out, int n) {
      double inv_n = (n > 0) ? 1.0 / (double)n : 0.0;
      double acc = 0.0;
      for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < n; i += gridDim.x * blockDim.x) {
            acc += (llround(pred[i]) == llround(y[i])) ? inv_n : 0.0;
      }
      double block_sum = block_reduce_sum(acc);
      if (threadIdx.x == 0)
            atomicAdd(out, block_sum);
}

extern "C" void launch_acc_metric(const double* pred, const double* y, double* out, int n, void* stream) {
      int blocks = (n + BLOCK - 1) / BLOCK;
      if (blocks < 1) blocks = 1;
      if (blocks > 65535) blocks = 65535;
      hipLaunchKernelGGL(accuracy_kernel, dim3(blocks), dim3(BLOCK), 0, (hipStream_t)stream,
                         pred, y, out, n);
}
