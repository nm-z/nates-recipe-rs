#include <hip/hip_runtime.h>

// Multiprocessor count of the current device via hipGetDeviceProperties →
// prop.multiProcessorCount (CUs in CU mode, half-CUs in WGP mode). Returned as a
// plain int so Rust never has to bind the ~800-byte hipDeviceProp_t struct.
extern "C" int hip_multiprocessor_count() {
      int dev = 0;
      if (hipGetDevice(&dev) != hipSuccess) return 0;
      hipDeviceProp_t prop;
      if (hipGetDeviceProperties(&prop, dev) != hipSuccess) return 0;
      return prop.multiProcessorCount;
}

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

// ── Split-K backward weight gradient ───────────────────────────────────────
// dW[i,j] = Σ_r X[r,i]·dZ[r,j], X is [m×k], dZ is [m×n], dW is [k×n] (all
// row-major). The reduction axis r (= batch rows, ~475K) is the only large dim;
// the output k×n is tiny (e.g. 42×64), so a plain GEMM launches ~4 workgroups and
// leaves 50 of 54 CUs idle. Split the reduction into P slices: pass 1 has one
// workgroup per (output tile, slice) accumulating a partial dW into a [P×k×n]
// scratch with register micro-tiling + LDS staging; pass 2 sums the P partials.
// Two-pass reduce (NOT f64 atomicAdd) keeps the sum order fixed → deterministic.
#define SK_BM 64
#define SK_BN 64
#define SK_BK 16
#define SK_TM 4
#define SK_TN 4
// threads = (SK_BM/SK_TM)*(SK_BN/SK_TN) = 16*16 = 256

__global__ void splitk_dw_pass1(
      const double* __restrict__ X,    // [m×k] row-major (a_prev / input)
      const double* __restrict__ dZ,   // [m×n] row-major (grad)
      double* __restrict__ partials,   // [P×k×n]
      int m, int n, int k, int P
) {
      int n_mtiles = (k + SK_BM - 1) / SK_BM;
      int tile = blockIdx.x;
      int tm = tile % n_mtiles;        // BM tile along k (output rows)
      int tn = tile / n_mtiles;        // BN tile along n (output cols)
      int p  = blockIdx.y;             // reduction slice
      int m0 = tm * SK_BM;
      int n0 = tn * SK_BN;

      long long rows_per = (long long)(m + P - 1) / P;
      long long r_begin  = (long long)p * rows_per;
      long long r_end    = r_begin + rows_per;
      if (r_end > (long long)m) r_end = (long long)m;

      __shared__ double As[SK_BK][SK_BM];   // As[kk][mm] = X[(r0+kk)*k + m0+mm]
      __shared__ double Bs[SK_BK][SK_BN];   // Bs[kk][nn] = dZ[(r0+kk)*n + n0+nn]

      int tid  = threadIdx.x;               // 0..255
      int tcol = tid % (SK_BN / SK_TN);     // 0..15
      int trow = tid / (SK_BN / SK_TN);     // 0..15

      double acc[SK_TM][SK_TN];
      #pragma unroll
      for (int i = 0; i < SK_TM; i++)
            #pragma unroll
            for (int j = 0; j < SK_TN; j++) acc[i][j] = 0.0;

      for (long long r0 = r_begin; r0 < r_end; r0 += SK_BK) {
            int bk = (int)(r_end - r0); if (bk > SK_BK) bk = SK_BK;
            for (int idx = tid; idx < SK_BK * SK_BM; idx += blockDim.x) {
                  int kk = idx / SK_BM, mm = idx % SK_BM, gm = m0 + mm;
                  As[kk][mm] = (kk < bk && gm < k) ? X[(r0 + kk) * (long long)k + gm] : 0.0;
            }
            for (int idx = tid; idx < SK_BK * SK_BN; idx += blockDim.x) {
                  int kk = idx / SK_BN, nn = idx % SK_BN, gn = n0 + nn;
                  Bs[kk][nn] = (kk < bk && gn < n) ? dZ[(r0 + kk) * (long long)n + gn] : 0.0;
            }
            __syncthreads();
            #pragma unroll
            for (int kk = 0; kk < SK_BK; kk++) {
                  double ar[SK_TM], br[SK_TN];
                  #pragma unroll
                  for (int i = 0; i < SK_TM; i++) ar[i] = As[kk][trow * SK_TM + i];
                  #pragma unroll
                  for (int j = 0; j < SK_TN; j++) br[j] = Bs[kk][tcol * SK_TN + j];
                  #pragma unroll
                  for (int i = 0; i < SK_TM; i++)
                        #pragma unroll
                        for (int j = 0; j < SK_TN; j++) acc[i][j] += ar[i] * br[j];
            }
            __syncthreads();
      }

      double* pp = partials + (long long)p * k * n;
      #pragma unroll
      for (int i = 0; i < SK_TM; i++) {
            int gi = m0 + trow * SK_TM + i;
            if (gi >= k) continue;
            #pragma unroll
            for (int j = 0; j < SK_TN; j++) {
                  int gj = n0 + tcol * SK_TN + j;
                  if (gj >= n) continue;
                  pp[(long long)gi * n + gj] = acc[i][j];
            }
      }
}

__global__ void splitk_dw_pass2(const double* __restrict__ partials, double* __restrict__ dW, int kn, int P) {
      int e = blockIdx.x * blockDim.x + threadIdx.x;
      if (e >= kn) return;
      double s = 0.0;
      for (int p = 0; p < P; p++) s += partials[(long long)p * kn + e];
      dW[e] = s;
}

extern "C" void launch_splitk_dw(
      const double* input, const double* grad, double* partials, double* grad_w,
      int m, int n, int k, int P, hipStream_t stream
) {
      int n_mtiles = (k + SK_BM - 1) / SK_BM;
      int n_ntiles = (n + SK_BN - 1) / SK_BN;
      dim3 grid1(n_mtiles * n_ntiles, P);
      hipLaunchKernelGGL(splitk_dw_pass1, grid1, dim3(256), 0, stream, input, grad, partials, m, n, k, P);
      int kn = k * n;
      hipLaunchKernelGGL(splitk_dw_pass2, dim3((kn + 255) / 256), dim3(256), 0, stream, partials, grad_w, kn, P);
}
