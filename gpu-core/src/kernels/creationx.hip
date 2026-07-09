#include <hip/hip_runtime.h>


extern "C" __global__ void creationx_arange_kernel(double* out, double start, double step, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = start + (double)i * step;
}
extern "C" void launch_creationx_arange(double* out, double start, double step, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_arange_kernel, grid, block, 0, s, out, start, step, n);
}

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

extern "C" __global__ void creationx_arange_i32_kernel(int* out, int start, int step, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = start + i * step;
}
extern "C" void launch_creationx_arange_i32(int* out, int start, int step, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(creationx_arange_i32_kernel, grid, block, 0, s, out, start, step, n);
}
