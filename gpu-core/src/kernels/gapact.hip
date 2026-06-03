#include <hip/hip_runtime.h>

// ── Gap activations the swarm did not ship (forward + backward), f64 ─────────
// Unique symbol names (gapact_*) to avoid any collision with existing kernels.
// Backward kernels take the PRE-activation x. Hyperparameters are runtime args.

extern "C" __global__ void gapact_elu_kernel(const double* x, double* out, int n, double alpha) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = v > 0.0 ? v : alpha * (exp(v) - 1.0); }
}
extern "C" __global__ void gapact_elu_backward_kernel(const double* grad, const double* x, double* out, int n, double alpha) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = grad[i] * (v > 0.0 ? 1.0 : alpha * exp(v)); }
}
extern "C" __global__ void gapact_selu_kernel(const double* x, double* out, int n, double alpha, double lambda) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = lambda * (v > 0.0 ? v : alpha * (exp(v) - 1.0)); }
}
extern "C" __global__ void gapact_selu_backward_kernel(const double* grad, const double* x, double* out, int n, double alpha, double lambda) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = grad[i] * lambda * (v > 0.0 ? 1.0 : alpha * exp(v)); }
}
extern "C" __global__ void gapact_mish_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; double sp = (v > 0.0 ? v : 0.0) + log1p(exp(-fabs(v))); out[i] = v * tanh(sp); }
}
extern "C" __global__ void gapact_mish_backward_kernel(const double* grad, const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double v = x[i];
        double sp = (v > 0.0 ? v : 0.0) + log1p(exp(-fabs(v)));
        double th = tanh(sp); double sig = 1.0 / (1.0 + exp(-v));
        out[i] = grad[i] * (th + v * (1.0 - th * th) * sig);
    }
}
extern "C" __global__ void gapact_softplus_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = (v > 0.0 ? v : 0.0) + log1p(exp(-fabs(v))); }
}
extern "C" __global__ void gapact_softplus_backward_kernel(const double* grad, const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = grad[i] * (1.0 / (1.0 + exp(-x[i])));
}
extern "C" __global__ void gapact_hardswish_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; double r = v + 3.0; r = r < 0.0 ? 0.0 : (r > 6.0 ? 6.0 : r); out[i] = v * r / 6.0; }
}
extern "C" __global__ void gapact_hardswish_backward_kernel(const double* grad, const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; double d = v <= -3.0 ? 0.0 : (v >= 3.0 ? 1.0 : (2.0 * v + 3.0) / 6.0); out[i] = grad[i] * d; }
}
extern "C" __global__ void gapact_swiglu_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double bb = b[i]; double s = bb / (1.0 + exp(-bb)); out[i] = a[i] * s; }
}
extern "C" __global__ void gapact_geglu_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double bb = b[i]; double g = 0.5 * bb * (1.0 + erf(bb * 0.7071067811865476)); out[i] = a[i] * g; }
}

#define GA_GRID(n) dim3 block(256); dim3 grid(((n) + 255) / 256)
extern "C" void launch_gapact_elu(const double* x, double* out, int n, double alpha, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_elu_kernel, grid, block, 0, s, x, out, n, alpha); }
extern "C" void launch_gapact_elu_backward(const double* g, const double* x, double* out, int n, double alpha, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_elu_backward_kernel, grid, block, 0, s, g, x, out, n, alpha); }
extern "C" void launch_gapact_selu(const double* x, double* out, int n, double alpha, double lambda, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_selu_kernel, grid, block, 0, s, x, out, n, alpha, lambda); }
extern "C" void launch_gapact_selu_backward(const double* g, const double* x, double* out, int n, double alpha, double lambda, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_selu_backward_kernel, grid, block, 0, s, g, x, out, n, alpha, lambda); }
extern "C" void launch_gapact_mish(const double* x, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_mish_kernel, grid, block, 0, s, x, out, n); }
extern "C" void launch_gapact_mish_backward(const double* g, const double* x, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_mish_backward_kernel, grid, block, 0, s, g, x, out, n); }
extern "C" void launch_gapact_softplus(const double* x, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_softplus_kernel, grid, block, 0, s, x, out, n); }
extern "C" void launch_gapact_softplus_backward(const double* g, const double* x, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_softplus_backward_kernel, grid, block, 0, s, g, x, out, n); }
extern "C" void launch_gapact_hardswish(const double* x, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_hardswish_kernel, grid, block, 0, s, x, out, n); }
extern "C" void launch_gapact_hardswish_backward(const double* g, const double* x, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_hardswish_backward_kernel, grid, block, 0, s, g, x, out, n); }
extern "C" void launch_gapact_swiglu(const double* a, const double* b, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_swiglu_kernel, grid, block, 0, s, a, b, out, n); }
extern "C" void launch_gapact_geglu(const double* a, const double* b, double* out, int n, hipStream_t s) { GA_GRID(n); hipLaunchKernelGGL(gapact_geglu_kernel, grid, block, 0, s, a, b, out, n); }
