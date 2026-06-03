#include <hip/hip_runtime.h>

// ── Missing elementwise-unary math ops, f64 (collision-safe launch_mx_* names) ─
#define MX(fn, expr)                                                              \
    extern "C" __global__ void mx_##fn##_kernel(const double* x, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                            \
        if (i < n) { double v = x[i]; out[i] = (expr); }                          \
    }                                                                             \
    extern "C" void launch_mx_##fn(const double* x, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                              \
        hipLaunchKernelGGL(mx_##fn##_kernel, grid, block, 0, s, x, out, n);       \
    }

MX(square,  v * v)
MX(exp2,    exp2(v))
MX(log2,    log2(v))
MX(log10,   log10(v))
MX(cbrt,    cbrt(v))
MX(sinh,    sinh(v))
MX(cosh,    cosh(v))
MX(asin,    asin(v))
MX(acos,    acos(v))
MX(atan,    atan(v))
MX(asinh,   asinh(v))
MX(acosh,   acosh(v))
MX(atanh,   atanh(v))
MX(erf,     erf(v))
MX(erfc,    erfc(v))
MX(tgamma,  tgamma(v))
MX(lgamma,  lgamma(v))
MX(deg2rad, v * 0.017453292519943295)
MX(rad2deg, v * 57.29577951308232)
