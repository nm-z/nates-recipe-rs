#include <cstring>
#include <hip/hip_runtime.h>
#include <rocprim/rocprim.hpp>
#include <float.h>

// ── Full-array reduction gaps (scalar out[0]) ──────────────────────────────
// Launchers do ZERO HIP memory APIs: no hipMalloc/hipFree/hipMemcpy/hipMemset,
// no host-side hipStreamSynchronize. rocprim temp storage and all internal
// device scalars are allocated by the Rust caller and passed in as pointers.
// For each rocprim-backed gap, query the temp size with the matching
// launch_*_workspace_bytes(...) function, allocate that many bytes, and pass
// the buffer + size to the launcher. After the launch, the Rust caller syncs
// its stream and downloads out[].
// rocprim::reduce signature: reduce(tmp, tmp_bytes, in, out, init, n, op, stream).

// rocprim temp-storage size query — pure size computation, launches nothing,
// touches no device memory (temp storage == nullptr path).
template <typename InIt, typename Op>
static inline size_t rp_reduce_bytes(InIt in, double* out, double init, int n, Op op) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, in, out, init, (size_t)n, op);
      return tmp;
}

// rocprim reduce into out[0] using caller-provided temp storage (d, d_bytes).
template <typename InIt, typename Op>
static inline void rp_reduce(InIt in, double* out, double init, int n, Op op, hipStream_t s, void* d, size_t d_bytes) {
      rocprim::reduce(d, d_bytes, in, out, init, (size_t)n, op, s);
}

// product = Π x_i
extern "C" size_t launch_reductionx_prod_workspace_bytes(const double* x, double* out, int n) {
      return rp_reduce_bytes(x, out, 1.0, n, rocprim::multiplies<double>());
}
extern "C" void launch_reductionx_prod(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, out, 1.0, n, rocprim::multiplies<double>(), s, workspace, workspace_bytes);
}

// nansum = Σ x_i treating NaN as 0
struct NanToZero { __host__ __device__ double operator()(double v) const { return v == v ? v : 0.0; } };
extern "C" size_t launch_reductionx_nansum_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, NanToZero{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_nansum(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, NanToZero{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

// mean_abs = (Σ |x_i|) / n  -> launcher returns Σ|x_i|; Rust divides by n
struct AbsOp { __host__ __device__ double operator()(double v) const { return v < 0.0 ? -v : v; } };
extern "C" size_t launch_reductionx_mean_abs_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, AbsOp{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_mean_abs(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, AbsOp{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

// sumsq ("sum of squares" / sqrSum) = Σ x_i²   (distinct from l2 = sqrt of this)
struct SqOp { __host__ __device__ double operator()(double v) const { return v * v; } };
extern "C" size_t launch_reductionx_sumsq_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, SqOp{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_sumsq(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, SqOp{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

// count_nonzero = number of x_i != 0  (returned as a double)
struct IsNonzero { __host__ __device__ double operator()(double v) const { return v != 0.0 ? 1.0 : 0.0; } };
extern "C" size_t launch_reductionx_count_nonzero_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_count_nonzero(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

// any = (∃ x_i != 0) ? 1 : 0    (logical OR over truthiness)
extern "C" size_t launch_reductionx_any_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::maximum<double>());
}
extern "C" void launch_reductionx_any(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 0.0, n, rocprim::maximum<double>(), s, workspace, workspace_bytes);
}

// all = (∀ x_i != 0) ? 1 : 0    (logical AND over truthiness)
extern "C" size_t launch_reductionx_all_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      return rp_reduce_bytes(it, out, 1.0, n, rocprim::minimum<double>());
}
extern "C" void launch_reductionx_all(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 1.0, n, rocprim::minimum<double>(), s, workspace, workspace_bytes);
}

// ptp ("peak to peak") = max(x) - min(x). Launcher writes max into out[0], min into out[1].
// One workspace reused for both reduces (serialized on stream s); size = max of the two.
extern "C" size_t launch_reductionx_ptp_workspace_bytes(const double* x, double* out, int n) {
      size_t a = rp_reduce_bytes(x, out,     -DBL_MAX, n, rocprim::maximum<double>());
      size_t b = rp_reduce_bytes(x, out + 1,  DBL_MAX, n, rocprim::minimum<double>());
      return a > b ? a : b;
}
extern "C" void launch_reductionx_ptp(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, out,     -DBL_MAX, n, rocprim::maximum<double>(), s, workspace, workspace_bytes);
      rp_reduce(x, out + 1,  DBL_MAX, n, rocprim::minimum<double>(), s, workspace, workspace_bytes);
}

// logsumexp = m + log(Σ exp(x_i - m)),  m = max(x). out[0]=m, out[1]=Σexp; Rust finishes m+log(se).
// Reduces write directly into out[0]/out[1]; the second reduce reads out[0] (=m) via the iterator,
// safe because both reduces run on stream s. One workspace reused for both reduces.
struct SubExp { const double* m; __host__ __device__ double operator()(double v) const { return exp(v - *m); } };
extern "C" size_t launch_reductionx_logsumexp_workspace_bytes(const double* x, double* out, int n) {
      size_t a = rp_reduce_bytes(x, out, -DBL_MAX, n, rocprim::maximum<double>());
      auto it = rocprim::make_transform_iterator(x, SubExp{out});
      size_t b = rp_reduce_bytes(it, out + 1, 0.0, n, rocprim::plus<double>());
      return a > b ? a : b;
}
extern "C" void launch_reductionx_logsumexp(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, out, -DBL_MAX, n, rocprim::maximum<double>(), s, workspace, workspace_bytes);   // out[0] = m = max(x)
      auto it = rocprim::make_transform_iterator(x, SubExp{out});
      rp_reduce(it, out + 1, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);      // out[1] = Σ exp(x - m)
}

// var/std: launcher returns Σ(x_i - μ)² into out[0]; Rust divides by n (pop) or n-1 (sample) and sqrt for std.
// Two-pass: mean μ first (into caller's `mu` scalar), then Σ(x-μ)². One workspace reused for both reduces.
struct SqDevOp { const double* mu; __host__ __device__ double operator()(double v) const { double d = v - *mu; return d * d; } };
__global__ void reductionx_scale1_kernel(double* p, double sc) { if (threadIdx.x == 0 && blockIdx.x == 0) p[0] *= sc; }
extern "C" size_t launch_reductionx_sumsqdev_workspace_bytes(const double* x, double* out, int n) {
      size_t a = rp_reduce_bytes(x, out, 0.0, n, rocprim::plus<double>());
      auto it = rocprim::make_transform_iterator(x, SqDevOp{out});
      size_t b = rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
      return a > b ? a : b;
}
extern "C" void launch_reductionx_sumsqdev(const double* x, double* out, double* mu, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, mu, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);          // mu = Σx
      hipLaunchKernelGGL(reductionx_scale1_kernel, dim3(1), dim3(1), 0, s, mu, 1.0 / (double)n); // mu /= n
      auto it = rocprim::make_transform_iterator(x, SqDevOp{mu});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);        // out = Σ(x-μ)²
}

// argmax/argmin: first-occurrence index of the global extremum, returned as a double in out[0].
// Mirrors reduce.hip argmin_rows_kernel (single block, value+index warp-shuffle), generalized to a
// grid-stride scan with an atomic tie-break that keeps the SMALLEST index among equal extrema.
// The packed extremum tracker is a file-scope __device__ scalar so the launcher needs no caller
// scratch — keeping the 4-arg (x, out, n, stream) shape shared by every other reductionx launcher.
__device__ unsigned long long g_reductionx_best;

__global__ void reductionx_argmax_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double lval = -DBL_MAX; int lidx = 0;
      for (int j = i; j < n; j += gridDim.x * blockDim.x) {
            if (x[j] > lval) { lval = x[j]; lidx = j; }
      }
      // Lock-free CAS loop on a packed best holding idx; compare via x[]. value dominates,
      // ties keep the smaller idx (atomicMax on order-preserving f64 bits is too fragile).
      unsigned long long cur = g_reductionx_best;
      while (true) {
            int cidx = (int)(cur & 0xffffffffULL);
            double cval = x[cidx];
            if (lval < cval || (lval == cval && lidx >= cidx)) break;
            unsigned long long want = (unsigned long long)(unsigned int)lidx;
            unsigned long long prev = atomicCAS(&g_reductionx_best, cur, want);
            if (prev == cur) break;
            cur = prev;
      }
}
__global__ void reductionx_argmin_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double lval = DBL_MAX; int lidx = 0;
      for (int j = i; j < n; j += gridDim.x * blockDim.x) {
            if (x[j] < lval) { lval = x[j]; lidx = j; }
      }
      unsigned long long cur = g_reductionx_best;
      while (true) {
            int cidx = (int)(cur & 0xffffffffULL);
            double cval = x[cidx];
            if (lval > cval || (lval == cval && lidx >= cidx)) break;
            unsigned long long want = (unsigned long long)(unsigned int)lidx;
            unsigned long long prev = atomicCAS(&g_reductionx_best, cur, want);
            if (prev == cur) break;
            cur = prev;
      }
}
__global__ void reductionx_idx_to_double(double* out) {
      if (threadIdx.x == 0) out[0] = (double)(int)(g_reductionx_best & 0xffffffffULL);
}
// resets the packed tracker to idx 0 (replaces the old HtoD memcpy of zero).
__global__ void reductionx_zero_u64() {
      if (threadIdx.x == 0 && blockIdx.x == 0) g_reductionx_best = 0;
}
extern "C" void launch_reductionx_argmax(const double* x, double* out, int n, hipStream_t s) {
      hipLaunchKernelGGL(reductionx_zero_u64, dim3(1), dim3(1), 0, s);
      dim3 b(256); dim3 g(min(256, (n + 255) / 256));
      hipLaunchKernelGGL(reductionx_argmax_kernel, g, b, 0, s, x, out, n);
      hipLaunchKernelGGL(reductionx_idx_to_double, dim3(1), dim3(1), 0, s, out);
}
extern "C" void launch_reductionx_argmin(const double* x, double* out, int n, hipStream_t s) {
      hipLaunchKernelGGL(reductionx_zero_u64, dim3(1), dim3(1), 0, s);
      dim3 b(256); dim3 g(min(256, (n + 255) / 256));
      hipLaunchKernelGGL(reductionx_argmin_kernel, g, b, 0, s, x, out, n);
      hipLaunchKernelGGL(reductionx_idx_to_double, dim3(1), dim3(1), 0, s, out);
}
