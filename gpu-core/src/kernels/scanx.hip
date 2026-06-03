#include <hip/hip_runtime.h>
#include <float.h>
#include <math.h>

// ── scanx: cumulative / prefix-scan ops, f64, 1-D arrays ─────────────────────
// One block, thread 0 walks the array sequentially. Scans are inherently
// sequential (each output depends on all predecessors); the authoritative
// reference is a CPU running accumulate, which this reproduces exactly.
// Correctness, not throughput, is the contract these prove.

// cummin: inclusive running minimum.  out[i] = min(x[0..=i])
extern "C" __global__ void scanx_cummin_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = DBL_MAX;
            for (int i = 0; i < n; i++) { double v = x[i]; acc = v < acc ? v : acc; out[i] = acc; }
      }
}
extern "C" void launch_scanx_cummin(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_cummin_kernel, g, b, 0, s, x, out, n);
}

// cummax: inclusive running maximum.  out[i] = max(x[0..=i])
extern "C" __global__ void scanx_cummax_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = -DBL_MAX;
            for (int i = 0; i < n; i++) { double v = x[i]; acc = v > acc ? v : acc; out[i] = acc; }
      }
}
extern "C" void launch_scanx_cummax(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_cummax_kernel, g, b, 0, s, x, out, n);
}

// cumsum (inclusive): out[i] = sum(x[0..=i])
extern "C" __global__ void scanx_cumsum_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = 0.0;
            for (int i = 0; i < n; i++) { acc += x[i]; out[i] = acc; }
      }
}
extern "C" void launch_scanx_cumsum(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_cumsum_kernel, g, b, 0, s, x, out, n);
}

// cumprod (inclusive): out[i] = prod(x[0..=i])
extern "C" __global__ void scanx_cumprod_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = 1.0;
            for (int i = 0; i < n; i++) { acc *= x[i]; out[i] = acc; }
      }
}
extern "C" void launch_scanx_cumprod(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_cumprod_kernel, g, b, 0, s, x, out, n);
}

// exclusive cumsum (prefix sum, identity 0): out[i] = sum(x[0..i]); out[0] = 0
extern "C" __global__ void scanx_excl_sum_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = 0.0;
            for (int i = 0; i < n; i++) { out[i] = acc; acc += x[i]; }
      }
}
extern "C" void launch_scanx_excl_sum(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_excl_sum_kernel, g, b, 0, s, x, out, n);
}

// exclusive cumprod (prefix product, identity 1): out[i] = prod(x[0..i]); out[0] = 1
extern "C" __global__ void scanx_excl_prod_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = 1.0;
            for (int i = 0; i < n; i++) { out[i] = acc; acc *= x[i]; }
      }
}
extern "C" void launch_scanx_excl_prod(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_excl_prod_kernel, g, b, 0, s, x, out, n);
}

// logcumsumexp (inclusive): out[i] = log(sum_{k<=i} exp(x[k]))
// Numerically stable streaming: maintain running max m and running sum s of
// exp(x[k]-m).  lse_i = m + log(s).  Matches jax.lax.cumlogsumexp / torch.logcumsumexp.
extern "C" __global__ void scanx_logcumsumexp_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double m = -DBL_MAX;
            double s = 0.0;
            for (int i = 0; i < n; i++) {
                  double v = x[i];
                  if (v > m) {
                        // rescale running sum to new max
                        s = s * exp(m - v) + 1.0;
                        m = v;
                  } else {
                        s = s + exp(v - m);
                  }
                  out[i] = m + log(s);
            }
      }
}
extern "C" void launch_scanx_logcumsumexp(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_logcumsumexp_kernel, g, b, 0, s, x, out, n);
}

// associative_scan with combine = (+): inclusive scan under an associative binary
// op.  Generic jax.lax.associative_scan / tl.associative_scan primitive; we prove
// the canonical '+' instance (associativity is what the primitive guarantees).
extern "C" __global__ void scanx_assoc_add_kernel(const double* x, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double acc = 0.0;
            for (int i = 0; i < n; i++) { acc = acc + x[i]; out[i] = acc; }
      }
}
extern "C" void launch_scanx_assoc_add(const double* x, double* out, int n, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_assoc_add_kernel, g, b, 0, s, x, out, n);
}

// linear-recurrence associative scan: first-order h_t = a_t*h_{t-1} + b_t.
// This is jax.lax.associative_scan's flagship use (Mamba/S4 SSM, mamba2 cumsum).
// Pair-combine ((A1,B1),(A2,B2)) = (A1*A2, A2*B1 + B2) is associative; running it
// forward yields the recurrence.  out[i] = h_i with h_{-1}=0.
extern "C" __global__ void scanx_recur_kernel(const double* a, const double* b, double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            double h = 0.0;
            for (int i = 0; i < n; i++) { h = a[i] * h + b[i]; out[i] = h; }
      }
}
extern "C" void launch_scanx_recur(const double* a, const double* b, double* out, int n, hipStream_t s) {
      dim3 b2(1); dim3 g(1);
      hipLaunchKernelGGL(scanx_recur_kernel, g, b2, 0, s, a, b, out, n);
}
