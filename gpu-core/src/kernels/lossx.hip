#include <hip/hip_runtime.h>
#include <math.h>

// ── Per-element regression/classification loss kernels, f64 ───────────────────
// Each kernel computes the per-element loss contribution (the value BEFORE the
// mean/sum reduction), matching the exact textbook / framework definition.
// Inputs: pred (x) and target (y) arrays of length n. Output: out[i] = loss(x_i,y_i).
//
// Two-input macro: out[i] = EXPR over (a=pred[i], b=target[i]).
#define LOSS2(fn, expr)                                                           \
    extern "C" __global__ void lossx_##fn##_kernel(                              \
            const double* pred, const double* target, double* out, int n) {     \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                          \
        if (i < n) { double a = pred[i]; double b = target[i]; out[i] = (expr); } \
    }                                                                            \
    extern "C" void launch_lossx_##fn(                                          \
            const double* pred, const double* target, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                            \
        hipLaunchKernelGGL(lossx_##fn##_kernel, grid, block, 0, s, pred, target, out, n); \
    }

// MSE (squared error) per element: (pred - target)^2
LOSS2(mse, (a - b) * (a - b))

// MAE / L1 per element: |pred - target|
LOSS2(mae, fabs(a - b))

// log-cosh per element: log(cosh(pred - target))
// numerically stable: log(cosh(d)) = |d| + log1p(exp(-2|d|)) - log(2)
LOSS2(log_cosh, ({ double d = a - b; double ad = fabs(d); ad + log1p(exp(-2.0 * ad)) - 0.6931471805599453; }))

// BCE from probabilities per element: -(y*log(p) + (1-y)*log(1-p))
// p = pred clamped to (eps, 1-eps); y = target
LOSS2(bce, ({ double eps = 1e-12; double p = a < eps ? eps : (a > 1.0 - eps ? 1.0 - eps : a); -(b * log(p) + (1.0 - b) * log(1.0 - p)); }))

// Poisson NLL per element (torch default log_input=True): exp(input) - target*input
// here pred = input (log-rate), target = observed count
LOSS2(poisson_nll, exp(a) - b * a)

// KL divergence per element (torch convention, target as probabilities, pred = log-probabilities):
// out = target * (log(target) - pred); guard target<=0 -> 0
LOSS2(kl_div, (b > 0.0) ? b * (log(b) - a) : 0.0)

// ── Parameterized losses (extra scalar arg) ───────────────────────────────────

// Smooth L1 (torch) per element with beta:
//   |d| < beta : 0.5 * d^2 / beta
//   else       : |d| - 0.5 * beta
extern "C" __global__ void lossx_smooth_l1_kernel(
        const double* pred, const double* target, double* out, int n, double beta) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double d = pred[i] - target[i];
    double ad = fabs(d);
    out[i] = (ad < beta) ? (0.5 * d * d / beta) : (ad - 0.5 * beta);
}
extern "C" void launch_lossx_smooth_l1(
        const double* pred, const double* target, double* out, int n, double beta, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(lossx_smooth_l1_kernel, grid, block, 0, s, pred, target, out, n, beta);
}

// Huber (torch) per element with delta:
//   |d| <= delta : 0.5 * d^2
//   else         : delta * (|d| - 0.5 * delta)
extern "C" __global__ void lossx_huber_kernel(
        const double* pred, const double* target, double* out, int n, double delta) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double d = pred[i] - target[i];
    double ad = fabs(d);
    out[i] = (ad <= delta) ? (0.5 * d * d) : (delta * (ad - 0.5 * delta));
}
extern "C" void launch_lossx_huber(
        const double* pred, const double* target, double* out, int n, double delta, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(lossx_huber_kernel, grid, block, 0, s, pred, target, out, n, delta);
}

// Tweedie deviance loss per element with power p (1<p<2 typical), pred>0:
//   L = -y * pred^(1-p)/(1-p) + pred^(2-p)/(2-p)
// (unit deviance up to a factor; the standard regression objective form used in
//  sklearn TweedieRegressor / xgboost reg:tweedie)
extern "C" __global__ void lossx_tweedie_kernel(
        const double* pred, const double* target, double* out, int n, double power) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double y = target[i];
    double mu = pred[i];
    double a = pow(mu, 1.0 - power) / (1.0 - power);
    double c = pow(mu, 2.0 - power) / (2.0 - power);
    out[i] = -y * a + c;
}
extern "C" void launch_lossx_tweedie(
        const double* pred, const double* target, double* out, int n, double power, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(lossx_tweedie_kernel, grid, block, 0, s, pred, target, out, n, power);
}

// Quantile / pinball loss per element with quantile q in (0,1):
//   d = target - pred
//   L = max(q*d, (q-1)*d)   == d>=0 ? q*d : (q-1)*d
extern "C" __global__ void lossx_quantile_kernel(
        const double* pred, const double* target, double* out, int n, double q) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double d = target[i] - pred[i];
    double l = q * d;
    double r = (q - 1.0) * d;
    out[i] = l > r ? l : r;
}
extern "C" void launch_lossx_quantile(
        const double* pred, const double* target, double* out, int n, double q, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(lossx_quantile_kernel, grid, block, 0, s, pred, target, out, n, q);
}
