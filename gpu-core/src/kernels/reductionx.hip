#include <cstring>
#include <hip/hip_runtime.h>
#include <rocprim/rocprim.hpp>
#include <float.h>


template <typename InIt, typename Op>
static inline size_t rp_reduce_bytes(InIt in, double* out, double init, int n, Op op) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, in, out, init, (size_t)n, op);
      return tmp;
}

template <typename InIt, typename Op>
static inline void rp_reduce(InIt in, double* out, double init, int n, Op op, hipStream_t s, void* d, size_t d_bytes) {
      rocprim::reduce(d, d_bytes, in, out, init, (size_t)n, op, s);
}

extern "C" size_t launch_reductionx_prod_workspace_bytes(const double* x, double* out, int n) {
      return rp_reduce_bytes(x, out, 1.0, n, rocprim::multiplies<double>());
}
extern "C" void launch_reductionx_prod(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, out, 1.0, n, rocprim::multiplies<double>(), s, workspace, workspace_bytes);
}

struct NanToZero { __host__ __device__ double operator()(double v) const { return v == v ? v : 0.0; } };
extern "C" size_t launch_reductionx_nansum_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, NanToZero{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_nansum(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, NanToZero{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

struct AbsOp { __host__ __device__ double operator()(double v) const { return v < 0.0 ? -v : v; } };
extern "C" size_t launch_reductionx_mean_abs_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, AbsOp{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_mean_abs(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, AbsOp{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

struct SqOp { __host__ __device__ double operator()(double v) const { return v * v; } };
extern "C" size_t launch_reductionx_sumsq_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, SqOp{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_sumsq(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, SqOp{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

struct IsNonzero { __host__ __device__ double operator()(double v) const { return v != 0.0 ? 1.0 : 0.0; } };
extern "C" size_t launch_reductionx_count_nonzero_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
}
extern "C" void launch_reductionx_count_nonzero(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

extern "C" size_t launch_reductionx_any_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      return rp_reduce_bytes(it, out, 0.0, n, rocprim::maximum<double>());
}
extern "C" void launch_reductionx_any(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 0.0, n, rocprim::maximum<double>(), s, workspace, workspace_bytes);
}

extern "C" size_t launch_reductionx_all_workspace_bytes(const double* x, double* out, int n) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      return rp_reduce_bytes(it, out, 1.0, n, rocprim::minimum<double>());
}
extern "C" void launch_reductionx_all(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      auto it = rocprim::make_transform_iterator(x, IsNonzero{});
      rp_reduce(it, out, 1.0, n, rocprim::minimum<double>(), s, workspace, workspace_bytes);
}

extern "C" size_t launch_reductionx_ptp_workspace_bytes(const double* x, double* out, int n) {
      size_t a = rp_reduce_bytes(x, out,     -DBL_MAX, n, rocprim::maximum<double>());
      size_t b = rp_reduce_bytes(x, out + 1,  DBL_MAX, n, rocprim::minimum<double>());
      return a > b ? a : b;
}
extern "C" void launch_reductionx_ptp(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, out,     -DBL_MAX, n, rocprim::maximum<double>(), s, workspace, workspace_bytes);
      rp_reduce(x, out + 1,  DBL_MAX, n, rocprim::minimum<double>(), s, workspace, workspace_bytes);
}

struct SubExp { const double* m; __host__ __device__ double operator()(double v) const { return exp(v - *m); } };
extern "C" size_t launch_reductionx_logsumexp_workspace_bytes(const double* x, double* out, int n) {
      size_t a = rp_reduce_bytes(x, out, -DBL_MAX, n, rocprim::maximum<double>());
      auto it = rocprim::make_transform_iterator(x, SubExp{out});
      size_t b = rp_reduce_bytes(it, out + 1, 0.0, n, rocprim::plus<double>());
      return a > b ? a : b;
}
extern "C" void launch_reductionx_logsumexp(const double* x, double* out, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, out, -DBL_MAX, n, rocprim::maximum<double>(), s, workspace, workspace_bytes);
      auto it = rocprim::make_transform_iterator(x, SubExp{out});
      rp_reduce(it, out + 1, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

struct SqDevOp { const double* mu; __host__ __device__ double operator()(double v) const { double d = v - *mu; return d * d; } };
__global__ void reductionx_scale1_kernel(double* p, double sc) { if (threadIdx.x == 0 && blockIdx.x == 0) p[0] *= sc; }
extern "C" size_t launch_reductionx_sumsqdev_workspace_bytes(const double* x, double* out, int n) {
      size_t a = rp_reduce_bytes(x, out, 0.0, n, rocprim::plus<double>());
      auto it = rocprim::make_transform_iterator(x, SqDevOp{out});
      size_t b = rp_reduce_bytes(it, out, 0.0, n, rocprim::plus<double>());
      return a > b ? a : b;
}
extern "C" void launch_reductionx_sumsqdev(const double* x, double* out, double* mu, int n, hipStream_t s, void* workspace, size_t workspace_bytes) {
      rp_reduce(x, mu, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
      hipLaunchKernelGGL(reductionx_scale1_kernel, dim3(1), dim3(1), 0, s, mu, 1.0 / (double)n);
      auto it = rocprim::make_transform_iterator(x, SqDevOp{mu});
      rp_reduce(it, out, 0.0, n, rocprim::plus<double>(), s, workspace, workspace_bytes);
}

__device__ unsigned long long g_reductionx_best;

__global__ void reductionx_argmax_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double lval = -DBL_MAX; int lidx = 0;
      for (int j = i; j < n; j += gridDim.x * blockDim.x) {
            if (x[j] > lval) { lval = x[j]; lidx = j; }
      }
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
