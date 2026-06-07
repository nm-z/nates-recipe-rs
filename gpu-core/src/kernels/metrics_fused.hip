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

// ── classification accuracy: Σ [(pred>=0.5) == (y>=0.5)] / n ──────────────────
// Threshold at 0.5 to match the CPU reference (p>=0.5)==(y>=0.5). llround bins by
// nearest integer — wrong for raw logits and for anything outside [0,1].
__global__ void accuracy_kernel(const double* pred, const double* y, double* out, int n) {
      double inv_n = (n > 0) ? 1.0 / (double)n : 0.0;
      double acc = 0.0;
      for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < n; i += gridDim.x * blockDim.x) {
            acc += ((pred[i] >= 0.5) == (y[i] >= 0.5)) ? inv_n : 0.0;
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

// ── multi-class accuracy: Σ [argmax(pred_row) == argmax(y_row)] / n ───────────
// Each row is k-wide (one-hot target). Per row, argmax of pred vs argmax of y.
__global__ void argmax_acc_kernel(const double* pred, const double* y, double* out, int n, int k) {
      double inv_n = (n > 0) ? 1.0 / (double)n : 0.0;
      double acc = 0.0;
      for (int r = blockIdx.x * blockDim.x + threadIdx.x; r < n; r += gridDim.x * blockDim.x) {
            int pa = 0, ya = 0;
            double pbest = pred[r * k], ybest = y[r * k];
            for (int j = 1; j < k; ++j) {
                  double pv = pred[r * k + j];
                  if (pv > pbest) { pbest = pv; pa = j; }
                  double yv = y[r * k + j];
                  if (yv > ybest) { ybest = yv; ya = j; }
            }
            acc += (pa == ya) ? inv_n : 0.0;
      }
      double block_sum = block_reduce_sum(acc);
      if (threadIdx.x == 0)
            atomicAdd(out, block_sum);
}

extern "C" void launch_argmax_acc(const double* pred, const double* y, double* out, int n, int k, void* stream) {
      int blocks = (n + BLOCK - 1) / BLOCK;
      if (blocks < 1) blocks = 1;
      if (blocks > 65535) blocks = 65535;
      hipLaunchKernelGGL(argmax_acc_kernel, dim3(blocks), dim3(BLOCK), 0, (hipStream_t)stream,
                         pred, y, out, n, k);
}

// ── binary cross-entropy gradient: da = (p - y) / (p(1-p)) / n ────────────────
// Two-sided BCE: dL/dp = -y/p + (1-y)/(1-p) = (p-y)/(p(1-p)). Element-wise into
// da (no reduction). p clamped to [eps, 1-eps] so the division stays finite at
// the sigmoid tails. Paired with a sigmoid output this chains to dz = p-y.
__global__ void bce_grad_kernel(const double* pred, const double* y, double* da, int n) {
      double inv_n = (n > 0) ? 1.0 / (double)n : 0.0;
      const double eps = 1e-7;
      for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < n; i += gridDim.x * blockDim.x) {
            double p = pred[i];
            p = p < eps ? eps : (p > 1.0 - eps ? 1.0 - eps : p);
            da[i] = (p - y[i]) / (p * (1.0 - p)) * inv_n;
      }
}

extern "C" void launch_bce_grad(const double* pred, const double* y, double* da, int n, void* stream) {
      int blocks = (n + BLOCK - 1) / BLOCK;
      if (blocks < 1) blocks = 1;
      if (blocks > 65535) blocks = 65535;
      hipLaunchKernelGGL(bce_grad_kernel, dim3(blocks), dim3(BLOCK), 0, (hipStream_t)stream,
                         pred, y, da, n);
}
