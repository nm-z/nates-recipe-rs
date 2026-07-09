#include <hip/hip_runtime.h>


__device__ __forceinline__ double specialx_digamma_impl(double x) {
      double r = 0.0;
      while (x < 6.0) { r -= 1.0 / x; x += 1.0; }
      double f = 1.0 / (x * x);
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
SX(expit,   1.0 / (1.0 + exp(-v)))
SX(logit,   log(v / (1.0 - v)))
SX(sinc,    v == 0.0 ? 1.0 : sinpi(v) / (M_PI * v))
SX(entr,    v > 0.0 ? -v * log(v) : (v == 0.0 ? 0.0 : -HUGE_VAL))
SX(erfinv,  erfinv(v))
SX(erfcx,   erfcx(v))
SX(i0,      cyl_bessel_i0(v))
SX(i1,      cyl_bessel_i1(v))
SX(i0e,     exp(-fabs(v)) * cyl_bessel_i0(v))
SX(i1e,     exp(-fabs(v)) * cyl_bessel_i1(v))
SX(j0,      j0(v))
SX(j1,      j1(v))
SX(y0,      y0(v))
SX(y1,      y1(v))
SX(ndtr,    normcdf(v))
SX(ndtri,   normcdfinv(v))
SX(sinpi,   sinpi(v))

extern "C" __global__ void specialx_xlogy_kernel(const double* x, const double* y, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) { double a = x[i]; out[i] = (a == 0.0) ? 0.0 : a * log(y[i]); }
}
extern "C" void launch_specialx_xlogy(const double* x, const double* y, double* out, int n, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(specialx_xlogy_kernel, grid, block, 0, s, x, y, out, n);
}
