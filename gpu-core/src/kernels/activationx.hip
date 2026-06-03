#include <hip/hip_runtime.h>

// ── Remaining activation ops the swarm did not ship, f64 ────────────────────
// Unique symbol names (activationx_* / launch_activationx_*) to avoid any
// collision with existing kernels (gapact_*, actx_*, mx_*, etc.).
//
// Buckets:
//   same-size elementwise (x,out,n)            : relu_squared, shifted_softplus
//   same-size elementwise w/ runtime param p   : squareplus(b)
//   same-size elementwise w/ two params s,b    : star_relu
//   prelu: per-element alpha array (a,out,n)   : prelu  (scalar = broadcast array)
//   single input -> n/2 contiguous split       : glu, reglu  (out[i]=f(x[i],x[i+half]))
//   single input -> 2n concat                  : crelu (out=[relu(x) || relu(-x)])
//   row-wise softmax(-x)                        : softmin (rows,cols)

// ---- same-size elementwise, no params ----
#define AX0(fn, expr)                                                                 \
    extern "C" __global__ void activationx_##fn##_kernel(const double* x, double* out, int n) { \
        int i = blockIdx.x * blockDim.x + threadIdx.x;                                \
        if (i < n) { double v = x[i]; out[i] = (expr); }                             \
    }                                                                                 \
    extern "C" void launch_activationx_##fn(const double* x, double* out, int n, hipStream_t s) { \
        dim3 block(256); dim3 grid((n + 255) / 256);                                  \
        hipLaunchKernelGGL(activationx_##fn##_kernel, grid, block, 0, s, x, out, n);  \
    }

// relu_squared = relu(x)^2
AX0(relu_squared, (v > 0.0 ? v * v : 0.0))
// shifted_softplus = softplus(x) - ln2 = ln(0.5*e^x + 0.5); stable form.
AX0(shifted_softplus, ((v > 0.0 ? v : 0.0) + log1p(exp(-fabs(v))) - 0.6931471805599453))

// ---- same-size elementwise, one runtime param p ----
// squareplus(x,b) = (x + sqrt(x^2 + b)) / 2
extern "C" __global__ void activationx_squareplus_kernel(const double* x, double* out, int n, double b) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = 0.5 * (v + sqrt(v * v + b)); }
}
extern "C" void launch_activationx_squareplus(const double* x, double* out, int n, double b, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(activationx_squareplus_kernel, grid, block, 0, s, x, out, n, b);
}

// ---- same-size elementwise, two runtime params s, b ----
// star_relu(x; s, b) = s * relu(x)^2 + b
extern "C" __global__ void activationx_star_relu_kernel(const double* x, double* out, int n, double s, double b) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; double r = v > 0.0 ? v : 0.0; out[i] = s * r * r + b; }
}
extern "C" void launch_activationx_star_relu(const double* x, double* out, int n, double s, double b, hipStream_t st) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(activationx_star_relu_kernel, grid, block, 0, st, x, out, n, s, b);
}

// ---- prelu: per-element alpha array. prelu(x,a) = max(0,x) + a*min(0,x) ----
extern "C" __global__ void activationx_prelu_kernel(const double* x, const double* alpha, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = v > 0.0 ? v : alpha[i] * v; }
}
extern "C" void launch_activationx_prelu(const double* x, const double* alpha, double* out, int n, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(activationx_prelu_kernel, grid, block, 0, s, x, alpha, out, n);
}

// ---- glu: split contiguous halves. in length 2*half; out[i]=a[i]*sigmoid(b[i]) ----
extern "C" __global__ void activationx_glu_kernel(const double* x, double* out, int half) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < half) { double a = x[i]; double b = x[i + half]; out[i] = a * (1.0 / (1.0 + exp(-b))); }
}
extern "C" void launch_activationx_glu(const double* x, double* out, int half, hipStream_t s) {
    dim3 block(256); dim3 grid((half + 255) / 256);
    hipLaunchKernelGGL(activationx_glu_kernel, grid, block, 0, s, x, out, half);
}

// ---- reglu: split contiguous halves. out[i]=a[i]*relu(b[i]) ----
extern "C" __global__ void activationx_reglu_kernel(const double* x, double* out, int half) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < half) { double a = x[i]; double b = x[i + half]; out[i] = a * (b > 0.0 ? b : 0.0); }
}
extern "C" void launch_activationx_reglu(const double* x, double* out, int half, hipStream_t s) {
    dim3 block(256); dim3 grid((half + 255) / 256);
    hipLaunchKernelGGL(activationx_reglu_kernel, grid, block, 0, s, x, out, half);
}

// ---- crelu: concat(relu(x), relu(-x)). out length 2n. ----
extern "C" __global__ void activationx_crelu_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double v = x[i]; out[i] = v > 0.0 ? v : 0.0; out[i + n] = (-v) > 0.0 ? -v : 0.0; }
}
extern "C" void launch_activationx_crelu(const double* x, double* out, int n, hipStream_t s) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(activationx_crelu_kernel, grid, block, 0, s, x, out, n);
}

// ---- softmin: row-wise softmax(-x). One block per row, numerically stable. ----
extern "C" __global__ void activationx_softmin_kernel(const double* x, double* out, int rows, int cols) {
    int r = blockIdx.x;
    if (r >= rows) return;
    const double* xr = x + (size_t)r * cols;
    double* orow = out + (size_t)r * cols;
    // min of row (so -min is the max of -x) for stability
    double mn = xr[0];
    for (int j = 1; j < cols; ++j) { double v = xr[j]; if (v < mn) mn = v; }
    double sum = 0.0;
    for (int j = 0; j < cols; ++j) { double e = exp(-(xr[j] - mn)); orow[j] = e; sum += e; }
    double inv = 1.0 / sum;
    for (int j = 0; j < cols; ++j) orow[j] *= inv;
}
extern "C" void launch_activationx_softmin(const double* x, double* out, int rows, int cols, hipStream_t s) {
    dim3 block(1); dim3 grid(rows);
    hipLaunchKernelGGL(activationx_softmin_kernel, grid, block, 0, s, x, out, rows, cols);
}
