#include <hip/hip_runtime.h>

// ── More gap activations, f64 (collision-safe launch_actx_* names) ───────────
// Parameterized ones expose the full param range as a runtime arg.
#define ACTX0(fn, expr)                                                          \
    extern "C" __global__ void actx_##fn##_kernel(const double* x, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                           \
        if (i < n) { double v = x[i]; out[i] = (expr); }                         \
    }                                                                            \
    extern "C" void launch_actx_##fn(const double* x, double* out, int n, hipStream_t s) { \
        dim3 b(256); dim3 g((n + 255) / 256);                                    \
        hipLaunchKernelGGL(actx_##fn##_kernel, g, b, 0, s, x, out, n);           \
    }
#define ACTX1(fn, expr)                                                          \
    extern "C" __global__ void actx_##fn##_kernel(const double* x, double* out, int n, double p) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                           \
        if (i < n) { double v = x[i]; out[i] = (expr); }                         \
    }                                                                            \
    extern "C" void launch_actx_##fn(const double* x, double* out, int n, double p, hipStream_t s) { \
        dim3 b(256); dim3 g((n + 255) / 256);                                    \
        hipLaunchKernelGGL(actx_##fn##_kernel, g, b, 0, s, x, out, n, p);        \
    }

static __device__ __forceinline__ double clampd(double v, double lo, double hi) { return v < lo ? lo : (v > hi ? hi : v); }

ACTX0(relu6,        clampd(v, 0.0, 6.0))
ACTX0(hardsigmoid,  clampd(v / 6.0 + 0.5, 0.0, 1.0))
ACTX0(hardtanh,     clampd(v, -1.0, 1.0))
ACTX0(softsign,     v / (1.0 + fabs(v)))
ACTX0(tanhshrink,   v - tanh(v))
ACTX0(logsigmoid,   -((-v > 0.0 ? -v : 0.0) + log1p(exp(-fabs(v)))))   // log(sigmoid(v)) = -softplus(-v)
ACTX0(gelu_exact,   0.5 * v * (1.0 + erf(v * 0.7071067811865476)))
ACTX0(softshrink,   v > 0.5 ? v - 0.5 : (v < -0.5 ? v + 0.5 : 0.0))
ACTX1(celu,         (v > 0.0 ? v : 0.0) + fmin(0.0, p * (exp(v / p) - 1.0)))
ACTX1(hardshrink,   fabs(v) > p ? v : 0.0)
ACTX1(softshrink_p, v > p ? v - p : (v < -p ? v + p : 0.0))
ACTX1(thresholdedrelu, v > p ? v : 0.0)
