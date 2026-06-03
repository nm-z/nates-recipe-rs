#include <hip/hip_runtime.h>
#include <math.h>

// ── elementwise_unary TAIL gaps, f64 (collision-safe elementwise_unaryx_* names) ──
// Each op: out[i] = f(x[i]). Authoritative conventions:
//   sinc      : normalized, sin(pi*x)/(pi*x), sinc(0)=1            (torch/numpy)
//   frac      : x - trunc(x)                                       (torch)
//   heaviside : 0 for x<0, 0.5 for x==0, 1 for x>0  (half-max)     (math convention)
//   signbit   : 1.0 if sign bit set (incl -0.0), else 0.0          (torch/numpy)
//   isnan     : 1.0 if NaN else 0.0
//   isinf     : 1.0 if +/-inf else 0.0
//   isfinite  : 1.0 if finite (not nan, not inf) else 0.0
//   logit     : ln(p/(1-p))                                        (scipy.special.logit)
//   expit     : 1/(1+exp(-x))  (logistic sigmoid)                  (scipy.special.expit)
//   positive  : x  (identity)                                      (numpy/torch)
//   exp10     : 10^x

#define EU(fn, expr)                                                                       \
    extern "C" __global__ void elementwise_unaryx_##fn##_kernel(const double* x, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                                      \
        if (i < n) { double v = x[i]; out[i] = (expr); }                                    \
    }                                                                                       \
    extern "C" void launch_elementwise_unaryx_##fn(const double* x, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                                        \
        hipLaunchKernelGGL(elementwise_unaryx_##fn##_kernel, grid, block, 0, s, x, out, n); \
    }

__device__ __forceinline__ double eu_sinc(double v) {
    if (v == 0.0) return 1.0;
    double px = 3.14159265358979323846 * v;
    return sin(px) / px;
}

EU(sinc,      eu_sinc(v))
EU(frac,      v - trunc(v))
EU(heaviside, (v < 0.0) ? 0.0 : ((v > 0.0) ? 1.0 : 0.5))
EU(signbit,   signbit(v) ? 1.0 : 0.0)
EU(isnan,     isnan(v) ? 1.0 : 0.0)
EU(isinf,     isinf(v) ? 1.0 : 0.0)
EU(isfinite,  isfinite(v) ? 1.0 : 0.0)
EU(logit,     log(v / (1.0 - v)))
EU(expit,     1.0 / (1.0 + exp(-v)))
EU(positive,  v)
EU(exp10,     exp10(v))
