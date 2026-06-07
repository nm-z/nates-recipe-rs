#include <hip/hip_runtime.h>

// ── Unary elementwise ──────────────────────────────────────────────────────

extern "C" __global__ void rsqrt_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = rsqrt(x[i]);
}

extern "C" __global__ void reciprocal_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = 1.0 / x[i];
}

extern "C" __global__ void sin_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = sin(x[i]);
}

extern "C" __global__ void cos_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = cos(x[i]);
}

extern "C" __global__ void tan_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = tan(x[i]);
}

extern "C" __global__ void log1p_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = log1p(x[i]);
}

extern "C" __global__ void expm1_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = expm1(x[i]);
}

extern "C" __global__ void floor_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = floor(x[i]);
}

extern "C" __global__ void ceil_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = ceil(x[i]);
}

extern "C" __global__ void round_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = round(x[i]);
}

extern "C" __global__ void trunc_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = trunc(x[i]);
}

// ── Binary elementwise ─────────────────────────────────────────────────────

extern "C" __global__ void emax_kernel(const double* a, const double* b, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = a[i] > b[i] ? a[i] : b[i];
}

extern "C" __global__ void emin_kernel(const double* a, const double* b, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = a[i] < b[i] ? a[i] : b[i];
}

extern "C" __global__ void atan2_kernel(const double* a, const double* b, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = atan2(a[i], b[i]);
}

extern "C" __global__ void fmod_kernel(const double* a, const double* b, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = fmod(a[i], b[i]);
}

// ── Scalar ops ─────────────────────────────────────────────────────────────

extern "C" __global__ void sub_scalar_kernel(const double* x, double* out, int n, double s) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[i] - s;
}

extern "C" __global__ void div_scalar_kernel(const double* x, double* out, int n, double s) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[i] / s;
}

extern "C" __global__ void rsub_scalar_kernel(const double* x, double* out, int n, double s) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = s - x[i];
}

extern "C" __global__ void rdiv_scalar_kernel(const double* x, double* out, int n, double s) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = s / x[i];
}

// ── Reduce: has_nan / isfinite_all ────────────────────────────────────────
// flag buffer is a single i32, initialised to 0 by caller.
// has_nan: sets flag to 1 if any element is NaN.
// isfinite_all: sets flag to 1 if any element is NOT finite.

extern "C" __global__ void has_nan_kernel(const double* x, int* flag, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n && isnan(x[i])) atomicOr(flag, 1);
}

extern "C" __global__ void isfinite_all_kernel(const double* x, int* flag, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n && !isfinite(x[i])) atomicOr(flag, 1);
}

// ── Launchers ──────────────────────────────────────────────────────────────

extern "C" void launch_rsqrt(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(rsqrt_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_reciprocal(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(reciprocal_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_emax(const double* a, const double* b, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(emax_kernel, dim3(blocks), dim3(threads), 0, stream, a, b, out, n);
}

extern "C" void launch_emin(const double* a, const double* b, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(emin_kernel, dim3(blocks), dim3(threads), 0, stream, a, b, out, n);
}

extern "C" void launch_sin(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(sin_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_cos(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(cos_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_tan(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(tan_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_atan2(const double* a, const double* b, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(atan2_kernel, dim3(blocks), dim3(threads), 0, stream, a, b, out, n);
}

extern "C" void launch_log1p(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(log1p_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_expm1(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(expm1_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_floor(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(floor_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_ceil(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(ceil_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_round(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(round_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_trunc(const double* x, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(trunc_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n);
}

extern "C" void launch_fmod(const double* a, const double* b, double* out, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(fmod_kernel, dim3(blocks), dim3(threads), 0, stream, a, b, out, n);
}

extern "C" void launch_sub_scalar(const double* x, double* out, int n, double s, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(sub_scalar_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n, s);
}

extern "C" void launch_div_scalar(const double* x, double* out, int n, double s, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(div_scalar_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n, s);
}

extern "C" void launch_rsub_scalar(const double* x, double* out, int n, double s, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(rsub_scalar_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n, s);
}

extern "C" void launch_rdiv_scalar(const double* x, double* out, int n, double s, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(rdiv_scalar_kernel, dim3(blocks), dim3(threads), 0, stream, x, out, n, s);
}

extern "C" void launch_has_nan(const double* x, int* flag, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(has_nan_kernel, dim3(blocks), dim3(threads), 0, stream, x, flag, n);
}

extern "C" void launch_isfinite_all(const double* x, int* flag, int n, hipStream_t stream) {
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(isfinite_all_kernel, dim3(blocks), dim3(threads), 0, stream, x, flag, n);
}

extern "C" __global__ void tall_skinny_dgemm_kernel(
      const double* __restrict__ X,
      const double* __restrict__ W,
      double* __restrict__ C,
      int M, int N, int K
) {
      int row = blockIdx.x * blockDim.y + threadIdx.y;
      int col = threadIdx.x;
      if (row >= M || col >= N) return;
      double acc = 0.0;
      for (int i = 0; i < K; i++)
            acc += X[row * K + i] * W[i * N + col];
      C[row * N + col] += acc;
}

extern "C" void launch_tall_skinny_dgemm(
      const double* X, const double* W, double* C,
      int M, int N, int K, hipStream_t stream
) {
      dim3 threads(N, min(256 / N, 64));
      dim3 blocks((M + (int)threads.y - 1) / (int)threads.y);
      hipLaunchKernelGGL(tall_skinny_dgemm_kernel, blocks, threads, 0, stream, X, W, C, M, N, K);
}
