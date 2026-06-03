#include <cstring>
#include <hip/hip_runtime.h>
#include <rocprim/rocprim.hpp>
#include <float.h>

// ── Full-array reduction gaps (scalar out[0]) ──────────────────────────────
// All launchers take (const double* x, double* out, int n, hipStream_t s).
// out is a length-1 device buffer; the Rust side downloads out[0].
// rocprim::reduce signature: reduce(tmp, tmp_bytes, in, out, init, n, op, stream).

// generic rocprim reduce over a (possibly transformed) input iterator
template <typename InIt, typename Op>
static inline void rp_reduce(InIt in, double* out, double init, int n, Op op, hipStream_t s) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, in, out, init, (size_t)n, op, s);
      void* d = nullptr;
      hipMalloc(&d, tmp);
      rocprim::reduce(d, tmp, in, out, init, (size_t)n, op, s);
      hipStreamSynchronize(s);
      hipFree(d);
}

// product = Π x_i
extern "C" void launch_reductionx_prod(const double* x, double* out, int n, hipStream_t s) {
      rp_reduce(x, out, 1.0, n, rocprim::multiplies<double>(), s);
}

// nansum = Σ x_i treating NaN as 0
struct NanToZero { __host__ __device__ double operator()(double v) const { return v == v ? v : 0.0; } };
extern "C" void launch_reductionx_nansum(const double* x, double* out, int n, hipStream_t s) {
      auto it = rocprim::make_transform_iterator(x, NanToZero{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s);
}

// mean_abs = (Σ |x_i|) / n  -> launcher returns Σ|x_i|; Rust divides by n
struct AbsOp { __host__ __device__ double operator()(double v) const { return v < 0.0 ? -v : v; } };
extern "C" void launch_reductionx_mean_abs(const double* x, double* out, int n, hipStream_t s) {
      auto it = rocprim::make_transform_iterator(x, AbsOp{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s);
}

// sumsq ("sum of squares" / sqrSum) = Σ x_i²   (distinct from l2 = sqrt of this)
struct SqOp { __host__ __device__ double operator()(double v) const { return v * v; } };
extern "C" void launch_reductionx_sumsq(const double* x, double* out, int n, hipStream_t s) {
      auto it = rocprim::make_transform_iterator(x, SqOp{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s);
}

// count_nonzero = number of x_i != 0  (returned as a double)
struct IsNonzero { __host__ __device__ double operator()(double v) const { return v != 0.0 ? 1.0 : 0.0; } };
extern "C" void launch_reductionx_count_nonzero(const double* x, double* out, int n, hipStream_t s) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s);
}

// any = (∃ x_i != 0) ? 1 : 0    (logical OR over truthiness)
extern "C" void launch_reductionx_any(const double* x, double* out, int n, hipStream_t s) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 0.0, n, rocprim::maximum<double>(), s);
}

// all = (∀ x_i != 0) ? 1 : 0    (logical AND over truthiness)
extern "C" void launch_reductionx_all(const double* x, double* out, int n, hipStream_t s) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 1.0, n, rocprim::minimum<double>(), s);
}

// ptp ("peak to peak") = max(x) - min(x). Launcher writes max into out[0], min into out[1].
extern "C" void launch_reductionx_ptp(const double* x, double* out, int n, hipStream_t s) {
      rp_reduce(x, out,     -DBL_MAX, n, rocprim::maximum<double>(), s);
      rp_reduce(x, out + 1,  DBL_MAX, n, rocprim::minimum<double>(), s);
}

// logsumexp = m + log(Σ exp(x_i - m)),  m = max(x). out[0]=lse. Needs m first.
struct SubExp { const double* m; __host__ __device__ double operator()(double v) const { return exp(v - *m); } };
extern "C" void launch_reductionx_logsumexp(const double* x, double* out, int n, hipStream_t s) {
      double* m = nullptr; hipMalloc(&m, sizeof(double));
      rp_reduce(x, m, -DBL_MAX, n, rocprim::maximum<double>(), s);
      double* se = nullptr; hipMalloc(&se, sizeof(double));
      auto it = rocprim::make_transform_iterator(x, SubExp{m});
      rp_reduce(it, se, 0.0, n, rocprim::plus<double>(), s);
      // out[0] = m + log(se)  (host finishes: download m, se -> Rust). Write both into out.
      hipMemcpy(out,     m,  sizeof(double), hipMemcpyDeviceToDevice);
      hipMemcpy(out + 1, se, sizeof(double), hipMemcpyDeviceToDevice);
      hipFree(m); hipFree(se);
}

// var/std: launcher returns Σ(x_i - μ)² into out[0]; Rust divides by n (pop) or n-1 (sample) and sqrt for std.
// Two-pass: mean μ first, then Σ(x-μ)².
struct SqDevOp { const double* mu; __host__ __device__ double operator()(double v) const { double d = v - *mu; return d * d; } };
__global__ void reductionx_scale1_kernel(double* p, double sc) { if (threadIdx.x == 0 && blockIdx.x == 0) p[0] *= sc; }
extern "C" void launch_reductionx_sumsqdev(const double* x, double* out, int n, hipStream_t s) {
      double* mu = nullptr; hipMalloc(&mu, sizeof(double));
      rp_reduce(x, mu, 0.0, n, rocprim::plus<double>(), s);          // mu = Σx
      hipLaunchKernelGGL(reductionx_scale1_kernel, dim3(1), dim3(1), 0, s, mu, 1.0 / (double)n); // mu /= n
      hipStreamSynchronize(s);
      auto it = rocprim::make_transform_iterator(x, SqDevOp{mu});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s);
      hipFree(mu);
}

// argmax/argmin: first-occurrence index of the global extremum, returned as a double in out[0].
// Mirrors reduce.hip argmin_rows_kernel (single block, value+index warp-shuffle), generalized to a
// grid-stride scan with an atomic tie-break that keeps the SMALLEST index among equal extrema.
__global__ void reductionx_argmax_kernel(const double* x, double* out, unsigned long long* best, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double lval = -DBL_MAX; int lidx = 0;
      for (int j = i; j < n; j += gridDim.x * blockDim.x) {
            if (x[j] > lval) { lval = x[j]; lidx = j; }
      }
      // pack (value, idx) so a single atomicMax picks max value, and for ties the smaller idx.
      // value dominates; encode idx complemented so larger value OR (==value, smaller idx) wins.
      // Simpler: use atomicMax on a 64-bit pack of float-order-preserving value bits is fragile for f64.
      // Use a global lock-free CAS loop on a packed best holding idx, compare via x[].
      unsigned long long cur = *best;
      while (true) {
            int cidx = (int)(cur & 0xffffffffULL);
            double cval = x[cidx];
            if (lval < cval || (lval == cval && lidx >= cidx)) break;
            unsigned long long want = (unsigned long long)(unsigned int)lidx;
            unsigned long long prev = atomicCAS(best, cur, want);
            if (prev == cur) break;
            cur = prev;
      }
}
__global__ void reductionx_argmin_kernel(const double* x, double* out, unsigned long long* best, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double lval = DBL_MAX; int lidx = 0;
      for (int j = i; j < n; j += gridDim.x * blockDim.x) {
            if (x[j] < lval) { lval = x[j]; lidx = j; }
      }
      unsigned long long cur = *best;
      while (true) {
            int cidx = (int)(cur & 0xffffffffULL);
            double cval = x[cidx];
            if (lval > cval || (lval == cval && lidx >= cidx)) break;
            unsigned long long want = (unsigned long long)(unsigned int)lidx;
            unsigned long long prev = atomicCAS(best, cur, want);
            if (prev == cur) break;
            cur = prev;
      }
}
__global__ void reductionx_idx_to_double(const unsigned long long* best, double* out) {
      if (threadIdx.x == 0) out[0] = (double)(int)(*best & 0xffffffffULL);
}
extern "C" void launch_reductionx_argmax(const double* x, double* out, int n, hipStream_t s) {
      unsigned long long* best = nullptr; hipMalloc(&best, sizeof(unsigned long long));
      unsigned long long zero = 0; hipMemcpy(best, &zero, sizeof(zero), hipMemcpyHostToDevice);
      dim3 b(256); dim3 g(min(256, (n + 255) / 256));
      hipLaunchKernelGGL(reductionx_argmax_kernel, g, b, 0, s, x, out, best, n);
      hipLaunchKernelGGL(reductionx_idx_to_double, dim3(1), dim3(1), 0, s, best, out);
      hipStreamSynchronize(s);
      hipFree(best);
}
extern "C" void launch_reductionx_argmin(const double* x, double* out, int n, hipStream_t s) {
      unsigned long long* best = nullptr; hipMalloc(&best, sizeof(unsigned long long));
      unsigned long long zero = 0; hipMemcpy(best, &zero, sizeof(zero), hipMemcpyHostToDevice);
      dim3 b(256); dim3 g(min(256, (n + 255) / 256));
      hipLaunchKernelGGL(reductionx_argmin_kernel, g, b, 0, s, x, out, best, n);
      hipLaunchKernelGGL(reductionx_idx_to_double, dim3(1), dim3(1), 0, s, best, out);
      hipStreamSynchronize(s);
      hipFree(best);
}
