#include <hip/hip_runtime.h>
#include <math.h>


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
