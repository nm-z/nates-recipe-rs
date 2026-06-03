#include <hip/hip_runtime.h>

// ── "special" category kernels, f64 (collision-safe specialx_/launch_specialx_) ─
// Unary ops share a macro; xlogy is binary and stands alone.
// erfinv, erfcx, i0(cyl_bessel_i0), i1(cyl_bessel_i1), normcdf(ndtr),
// normcdfinv(ndtri), sinpi are ROCm device builtins. digamma is hand-rolled.

// Digamma psi(x)=d/dx ln Gamma(x). Recurrence up to x>=6, then asymptotic series.
__device__ __forceinline__ double specialx_digamma_impl(double x) {
      double r = 0.0;
      while (x < 6.0) { r -= 1.0 / x; x += 1.0; }
      double f = 1.0 / (x * x);
      // psi(x) ~ ln x - 1/(2x) - f*(1/12 - f*(1/120 - f*(1/252 - f*(1/240 - f/132))))
      double t = f * (1.0/12.0 - f * (1.0/120.0 - f * (1.0/252.0 - f * (1.0/240.0 - f * (1.0/132.0)))));
      return r + log(x) - 0.5 / x - t;
}

#define SX(fn, expr)                                                              \
    extern "C" __global__ void specialx_##fn##_kernel(const double* x, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                            \
        if (i < n) { double v = x[i]; out[i] = (expr); }                          \
    }                                                                             \
    extern "C" void launch_specialx_##fn(const double* x, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                              \
        hipLaunchKernelGGL(specialx_##fn##_kernel, grid, block, 0, s, x, out, n); \
    }

SX(digamma, specialx_digamma_impl(v))
SX(expit,   1.0 / (1.0 + exp(-v)))                       // logistic sigmoid
SX(logit,   log(v / (1.0 - v)))                          // log(x/(1-x))
SX(sinc,    v == 0.0 ? 1.0 : sinpi(v) / (M_PI * v))      // sin(pi x)/(pi x)
SX(entr,    v > 0.0 ? -v * log(v) : (v == 0.0 ? 0.0 : -HUGE_VAL)) // -x ln x, 0 at 0
SX(erfinv,  erfinv(v))                                   // inverse erf (ocml)
SX(erfcx,   erfcx(v))                                    // exp(x^2) erfc(x) (ocml)
SX(i0,      cyl_bessel_i0(v))                            // modified Bessel I0
SX(i1,      cyl_bessel_i1(v))                            // modified Bessel I1
SX(i0e,     exp(-fabs(v)) * cyl_bessel_i0(v))            // exp-scaled I0
SX(i1e,     exp(-fabs(v)) * cyl_bessel_i1(v))            // exp-scaled I1
SX(j0,      j0(v))                                       // Bessel J0
SX(j1,      j1(v))                                       // Bessel J1
SX(y0,      y0(v))                                       // Bessel Y0
SX(y1,      y1(v))                                       // Bessel Y1
SX(ndtr,    normcdf(v))                                  // standard normal CDF
SX(ndtri,   normcdfinv(v))                               // inverse normal CDF
SX(sinpi,   sinpi(v))                                    // sin(pi x)

// xlogy(x,y) = x * log(y), with x==0 -> 0 (defining convention)
extern "C" __global__ void specialx_xlogy_kernel(const double* x, const double* y, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) { double a = x[i]; out[i] = (a == 0.0) ? 0.0 : a * log(y[i]); }
}
extern "C" void launch_specialx_xlogy(const double* x, const double* y, double* out, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(specialx_xlogy_kernel, grid, block, 0, s, x, y, out, n);
}
