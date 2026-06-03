#include <hip/hip_runtime.h>
#include <math.h>

// ── foreach_* ops: same elementwise op applied over a flattened batch ─────────
// torch._foreach_<op> applies <op> to a list of tensors. Flattening the list to
// one contiguous buffer makes this an ordinary elementwise op — a single launch
// over n = sum(numel) elements is exactly the per-tensor semantics. Unique
// foreachx_/launch_foreachx_ names avoid collision with the rest of the library.

// Unary: x -> out
#define FE1(fn, expr)                                                                 \
    extern "C" __global__ void foreachx_##fn##_kernel(const double* x, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                                \
        if (i < n) { double v = x[i]; out[i] = (expr); }                             \
    }                                                                                  \
    extern "C" void launch_foreachx_##fn(const double* x, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                                  \
        hipLaunchKernelGGL(foreachx_##fn##_kernel, grid, block, 0, s, x, out, n);     \
    }

// Binary: a,b -> out (elementwise tensorlist op)
#define FE2(fn, expr)                                                                 \
    extern "C" __global__ void foreachx_##fn##_kernel(const double* a, const double* b, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                                \
        if (i < n) { double x = a[i]; double y = b[i]; out[i] = (expr); }            \
    }                                                                                  \
    extern "C" void launch_foreachx_##fn(const double* a, const double* b, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                                  \
        hipLaunchKernelGGL(foreachx_##fn##_kernel, grid, block, 0, s, a, b, out, n);  \
    }

// ── core 9 (task-mandated) ──
FE2(add, x + y)
FE2(sub, x - y)
FE2(mul, x * y)
FE2(div, x / y)
FE1(neg, -v)
FE1(abs, fabs(v))
FE1(sqrt, sqrt(v))
FE1(exp, exp(v))
FE1(sigmoid, 1.0 / (1.0 + exp(-v)))

// ── extra exact-oracle elementwise foreach ops (honest coverage) ──
FE1(reciprocal, 1.0 / v)
FE1(log,    log(v))
FE1(log2,   log2(v))
FE1(log10,  log10(v))
FE1(log1p,  log1p(v))
FE1(expm1,  expm1(v))
FE1(floor,  floor(v))
FE1(ceil,   ceil(v))
FE1(round,  rint(v))
FE1(trunc,  trunc(v))
FE1(frac,   v - trunc(v))
FE1(sign,   (v > 0.0) ? 1.0 : ((v < 0.0) ? -1.0 : 0.0))
FE1(sin,    sin(v))
FE1(cos,    cos(v))
FE1(tan,    tan(v))
FE1(sinh,   sinh(v))
FE1(cosh,   cosh(v))
FE1(tanh,   tanh(v))
FE1(asin,   asin(v))
FE1(acos,   acos(v))
FE1(atan,   atan(v))
FE1(erf,    erf(v))
FE1(erfc,   erfc(v))
FE1(lgamma, lgamma(v))
FE2(maximum, fmax(x, y))
FE2(minimum, fmin(x, y))
FE2(pow, pow(x, y))
