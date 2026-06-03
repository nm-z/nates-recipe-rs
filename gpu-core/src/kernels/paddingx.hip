#include <hip/hip_runtime.h>

// ── Padding kernels, f64. 1D (length L) and 2D (row-major H×W). ───────────────
// Each thread owns ONE output element and maps it back to a source index per the
// pad convention, or writes the constant value for constant-pad OOB.
//
//   constant : OOB -> cval
//   reflect  : mirror WITHOUT repeating the edge  (abcd, pad2 left -> cb|abcd)
//   replicate: clamp to [0,L-1] (edge)            (abcd, pad2 left -> aa|abcd)
//   circular : wrap modulo L                      (abcd, pad2 left -> cd|abcd)

// reflect a coordinate into [0, L-1] without repeating the boundary; loops so it
// works for pad >= L. L>=2 required for reflect (matches PyTorch).
__device__ __forceinline__ int reflect_idx(int p, int L) {
      if (L == 1) return 0;
      int period = 2 * L - 2;
      int q = p % period;
      if (q < 0) q += period;
      return (q < L) ? q : (period - q);
}
__device__ __forceinline__ int replicate_idx(int p, int L) {
      if (p < 0) return 0;
      if (p >= L) return L - 1;
      return p;
}
__device__ __forceinline__ int circular_idx(int p, int L) {
      int q = p % L;
      if (q < 0) q += L;
      return q;
}

// ─────────────────────────── 1D ───────────────────────────
// out length = L + lpad + rpad ; n = out length.
extern "C" __global__ void paddingx_constant1d_kernel(
            const double* x, double* out, int L, int lpad, int n, double cval) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int p = i - lpad;
            out[i] = (p >= 0 && p < L) ? x[p] : cval;
      }
}
extern "C" __global__ void paddingx_reflect1d_kernel(
            const double* x, double* out, int L, int lpad, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[reflect_idx(i - lpad, L)];
}
extern "C" __global__ void paddingx_replicate1d_kernel(
            const double* x, double* out, int L, int lpad, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[replicate_idx(i - lpad, L)];
}
extern "C" __global__ void paddingx_circular1d_kernel(
            const double* x, double* out, int L, int lpad, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[circular_idx(i - lpad, L)];
}

// ─────────────────────────── 2D ───────────────────────────
// input H×W row-major; output OH×OW with OH=H+tpad+bpad, OW=W+lpad+rpad.
// n = OH*OW ; thread index -> (or,oc).
extern "C" __global__ void paddingx_constant2d_kernel(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n, double cval) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int oc = i % OW, orr = i / OW;
            int r = orr - tpad, c = oc - lpad;
            out[i] = (r >= 0 && r < H && c >= 0 && c < W) ? x[r * W + c] : cval;
      }
}
extern "C" __global__ void paddingx_reflect2d_kernel(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int oc = i % OW, orr = i / OW;
            int r = reflect_idx(orr - tpad, H), c = reflect_idx(oc - lpad, W);
            out[i] = x[r * W + c];
      }
}
extern "C" __global__ void paddingx_replicate2d_kernel(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int oc = i % OW, orr = i / OW;
            int r = replicate_idx(orr - tpad, H), c = replicate_idx(oc - lpad, W);
            out[i] = x[r * W + c];
      }
}
extern "C" __global__ void paddingx_circular2d_kernel(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int oc = i % OW, orr = i / OW;
            int r = circular_idx(orr - tpad, H), c = circular_idx(oc - lpad, W);
            out[i] = x[r * W + c];
      }
}

// ─────────────────────────── launchers ───────────────────────────
#define G(n) dim3 grid(((n) + 255) / 256)
extern "C" void launch_paddingx_constant1d(
            const double* x, double* out, int L, int lpad, int n, double cval, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_constant1d_kernel, grid, block, 0, s, x, out, L, lpad, n, cval);
}
extern "C" void launch_paddingx_reflect1d(
            const double* x, double* out, int L, int lpad, int n, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_reflect1d_kernel, grid, block, 0, s, x, out, L, lpad, n);
}
extern "C" void launch_paddingx_replicate1d(
            const double* x, double* out, int L, int lpad, int n, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_replicate1d_kernel, grid, block, 0, s, x, out, L, lpad, n);
}
extern "C" void launch_paddingx_circular1d(
            const double* x, double* out, int L, int lpad, int n, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_circular1d_kernel, grid, block, 0, s, x, out, L, lpad, n);
}
extern "C" void launch_paddingx_constant2d(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n, double cval, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_constant2d_kernel, grid, block, 0, s, x, out, H, W, tpad, lpad, OH, OW, n, cval);
}
extern "C" void launch_paddingx_reflect2d(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_reflect2d_kernel, grid, block, 0, s, x, out, H, W, tpad, lpad, OH, OW, n);
}
extern "C" void launch_paddingx_replicate2d(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_replicate2d_kernel, grid, block, 0, s, x, out, H, W, tpad, lpad, OH, OW, n);
}
extern "C" void launch_paddingx_circular2d(
            const double* x, double* out, int H, int W, int tpad, int lpad,
            int OH, int OW, int n, hipStream_t s) {
      dim3 block(256); G(n);
      hipLaunchKernelGGL(paddingx_circular2d_kernel, grid, block, 0, s, x, out, H, W, tpad, lpad, OH, OW, n);
}
