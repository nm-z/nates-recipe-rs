#include <hip/hip_runtime.h>

// ── creation-category generators: output-only from scalar params, f64 ──────────
// Unique names prefixed creationx_ / launch_creationx_ to avoid symbol collisions
// with existing eye/fill/iota kernels. Each writes out[i] from i and the params;
// no input buffer is read. Oracle in prove_creation.rs is the CPU sequence.

// arange: out[i] = start + i*step
extern "C" __global__ void creationx_arange_kernel(double* out, double start, double step, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = start + (double)i * step;
}
extern "C" void launch_creationx_arange(double* out, double start, double step, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_arange_kernel, grid, block, 0, s, out, start, step, n);
}

// linspace: endpoint-inclusive. out[i] = start + i*(stop-start)/(n-1); n==1 -> start.
extern "C" __global__ void creationx_linspace_kernel(double* out, double start, double stop, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            if (n == 1) { out[i] = start; return; }
            double step = (stop - start) / (double)(n - 1);
            out[i] = (i == n - 1) ? stop : start + (double)i * step;
      }
}
extern "C" void launch_creationx_linspace(double* out, double start, double stop, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_linspace_kernel, grid, block, 0, s, out, start, stop, n);
}

// logspace: out[i] = base^(linspace(start,stop,n)[i]). endpoint-inclusive exponent.
extern "C" __global__ void creationx_logspace_kernel(double* out, double start, double stop, double base, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double e;
            if (n == 1) e = start;
            else { double step = (stop - start) / (double)(n - 1); e = (i == n - 1) ? stop : start + (double)i * step; }
            out[i] = pow(base, e);
      }
}
extern "C" void launch_creationx_logspace(double* out, double start, double stop, double base, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_logspace_kernel, grid, block, 0, s, out, start, stop, base, n);
}

// geomspace: geometric progression from start to stop (both nonzero, same sign).
// out[i] = start * (stop/start)^(i/(n-1)); endpoint-inclusive.
extern "C" __global__ void creationx_geomspace_kernel(double* out, double start, double stop, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            if (n == 1) { out[i] = start; return; }
            if (i == n - 1) { out[i] = stop; return; }
            double t = (double)i / (double)(n - 1);
            out[i] = start * pow(stop / start, t);
      }
}
extern "C" void launch_creationx_geomspace(double* out, double start, double stop, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_geomspace_kernel, grid, block, 0, s, out, start, stop, n);
}

// tri: rows x cols lower-triangular ones matrix with diagonal offset k.
// out[r*cols + c] = (c <= r + k) ? 1.0 : 0.0  (row-major).
extern "C" __global__ void creationx_tri_kernel(double* out, int rows, int cols, int k) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = rows * cols;
      if (idx < total) {
            int r = idx / cols;
            int c = idx % cols;
            out[idx] = (c <= r + k) ? 1.0 : 0.0;
      }
}
extern "C" void launch_creationx_tri(double* out, int rows, int cols, int k, hipStream_t s) {
      int total = rows * cols;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(creationx_tri_kernel, grid, block, 0, s, out, rows, cols, k);
}

// eye_rect: rows x cols identity-like matrix with diagonal offset k (general, non-square).
// out[r*cols + c] = (c == r + k) ? 1.0 : 0.0
extern "C" __global__ void creationx_eye_rect_kernel(double* out, int rows, int cols, int k) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = rows * cols;
      if (idx < total) {
            int r = idx / cols;
            int c = idx % cols;
            out[idx] = (c == r + k) ? 1.0 : 0.0;
      }
}
extern "C" void launch_creationx_eye_rect(double* out, int rows, int cols, int k, hipStream_t s) {
      int total = rows * cols;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(creationx_eye_rect_kernel, grid, block, 0, s, out, rows, cols, k);
}

// arange_i32: integer sequence out[i] = start + i*step  (covers int-dtype iota/range/sequence)
extern "C" __global__ void creationx_arange_i32_kernel(int* out, int start, int step, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = start + i * step;
}
extern "C" void launch_creationx_arange_i32(int* out, int start, int step, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_arange_i32_kernel, grid, block, 0, s, out, start, step, n);
}
