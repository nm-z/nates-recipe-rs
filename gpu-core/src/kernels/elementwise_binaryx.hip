#include <hip/hip_runtime.h>
#include <math.h>

// ── Missing elementwise-binary math ops, f64 (collision-safe names) ───────────
// Two input arrays a,b -> out. Authoritative numpy/torch/IEEE conventions baked
// into the expression so the CPU oracle in prove_elementwise_binary.rs matches.
#define EB(fn, expr)                                                                       \
    extern "C" __global__ void elementwise_binaryx_##fn##_kernel(                          \
            const double* a, const double* b, double* out, int n) {                        \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                                     \
        if (i < n) { double x = a[i]; double y = b[i]; out[i] = (expr); }                  \
    }                                                                                       \
    extern "C" void launch_elementwise_binaryx_##fn(                                       \
            const double* a, const double* b, double* out, int n, hipStream_t s) {         \
        dim3 block(256); dim3 grid((n + 255) / 256);                                       \
        hipLaunchKernelGGL(elementwise_binaryx_##fn##_kernel, grid, block, 0, s, a, b, out, n); \
    }

EB(pow,         pow(x, y))
EB(hypot,       hypot(x, y))
EB(fmax,        fmax(x, y))
EB(fmin,        fmin(x, y))
EB(copysign,    copysign(x, y))
EB(nextafter,   nextafter(x, y))
EB(logaddexp,   fmax(x, y) + log1p(exp(-fabs(x - y))))
EB(logaddexp2,  fmax(x, y) + log2(1.0 + exp2(-fabs(x - y))))
EB(remainder,   x - floor(x / y) * y)
EB(floor_divide, floor(x / y))
EB(heaviside,   x < 0.0 ? 0.0 : (x > 0.0 ? 1.0 : y))
EB(ldexp,       x * exp2(y))
EB(xlogy,       x == 0.0 ? 0.0 : x * log(y))
EB(xlog1py,     x == 0.0 ? 0.0 : x * log1p(y))
EB(squared_difference, (x - y) * (x - y))
EB(ge,          x >= y ? 1.0 : 0.0)
EB(le,          x <= y ? 1.0 : 0.0)
EB(ne,          x != y ? 1.0 : 0.0)
