#include <hip/hip_runtime.h>

extern "C" __global__ void relu_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] > 0.0 ? x[i] : 0.0;
}

extern "C" __global__ void relu_backward_kernel(const double* grad, const double* act, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = act[i] > 0.0 ? grad[i] : 0.0;
}

extern "C" __global__ void add_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] + b[i];
}

extern "C" __global__ void add_scalar_kernel(const double* x, double* out, int n, double s) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] + s;
}

extern "C" __global__ void div_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] / b[i];
}

extern "C" __global__ void fma_kernel(const double* x, const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] * a[i] + b[i];
}

// Broadcast-add bias (1D) to each row of a matrix (rows x cols)
extern "C" __global__ void bias_add_kernel(const double* x, const double* bias, double* out, int rows, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < rows * cols) {
        int col = i % cols;
        out[i] = x[i] + bias[col];
    }
}

// Element-wise multiply: out = a * b
extern "C" __global__ void mul_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] * b[i];
}

// Subtract: out = a - b
extern "C" __global__ void sub_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] - b[i];
}

// Row-wise softmax: out[i,j] = exp(x[i,j] - max_j) / sum_j(exp(x[i,j] - max_j))
// One block per row, threads cooperate on reduction via shared memory.
extern "C" __global__ void softmax_rows_kernel(const double* x, double* out, int rows, int cols) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* row_in = x + row * cols;
    double* row_out = out + row * cols;

    // Phase 1: find max (thread-local then reduce)
    double local_max = -1e308;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double v = row_in[j];
        if (v > local_max) local_max = v;
    }
    // Warp shuffle reduction for max
    for (int offset = warpSize / 2; offset > 0; offset /= 2) {
        double other = __shfl_down(local_max, offset);
        if (other > local_max) local_max = other;
    }
    // Cross-warp reduction via shared memory
    __shared__ double smax[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) smax[wid] = local_max;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_max = smax[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            double other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
        }
        if (threadIdx.x == 0) smax[0] = local_max;
    }
    __syncthreads();
    double row_max = smax[0];

    // Phase 2: compute exp(x - max) and sum
    double local_sum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double e = exp(row_in[j] - row_max);
        row_out[j] = e;
        local_sum += e;
    }
    // Reduce sum
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_sum += __shfl_down(local_sum, offset);
    __shared__ double ssum[32];
    if (lane == 0) ssum[wid] = local_sum;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_sum = ssum[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (threadIdx.x == 0) ssum[0] = local_sum;
    }
    __syncthreads();
    double row_sum = ssum[0];

    // Phase 3: normalize
    double inv_sum = 1.0 / row_sum;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        row_out[j] *= inv_sum;
    }
}

// Scale each element: out[i] = (x[i] - y[i]) / n  (for gradient = (softmax - onehot) / n)
extern "C" __global__ void sub_scale_kernel(const double* a, const double* b, double* out, int n, double scale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = (a[i] - b[i]) * scale;
}

extern "C" void launch_softmax_rows(const double* x, double* out, int rows, int cols, hipStream_t stream) {
    int threads = min(256, cols);
    if (threads < 32) threads = 32;
    hipLaunchKernelGGL(softmax_rows_kernel, dim3(rows), dim3(threads), 0, stream, x, out, rows, cols);
}

extern "C" void launch_sub_scale(const double* a, const double* b, double* out, int n, double scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(sub_scale_kernel, grid, block, 0, stream, a, b, out, n, scale);
}

// Sigmoid: out = 1 / (1 + exp(-x))
extern "C" __global__ void sigmoid_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = 1.0 / (1.0 + exp(-x[i]));
}

// Sigmoid backward: dL/dx = dL/dy * y * (1 - y), where y = sigmoid(x)
extern "C" __global__ void sigmoid_backward_kernel(const double* grad, const double* act, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = grad[i] * act[i] * (1.0 - act[i]);
}

// Tanh: out = tanh(x)
extern "C" __global__ void tanh_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = tanh(x[i]);
}

// Tanh backward: dL/dx = dL/dy * (1 - y^2), where y = tanh(x)
extern "C" __global__ void tanh_backward_kernel(const double* grad, const double* act, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = grad[i] * (1.0 - act[i] * act[i]);
}

// Scaled exp: out = exp(scale * x) — for RBF kernel exp(-gamma * dist²)
extern "C" __global__ void scaled_exp_kernel(const double* x, double* out, int n, double scale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = exp(scale * x[i]);
}

// Log-determinant from Cholesky factor L: logdet = 2 * sum(log(diag(L)))
// L is n x n, row-major. Output is single scalar.
extern "C" __global__ void log_det_cholesky_kernel(const double* L, double* out, int n) {
    // Single thread — n is small (covariance matrix dimension)
    if (threadIdx.x == 0 && blockIdx.x == 0) {
        double sum = 0.0;
        for (int i = 0; i < n; i++) {
            sum += log(L[i * n + i]);
        }
        out[0] = 2.0 * sum;
    }
}

// Reparameterization: out = mu + exp(0.5 * log_var) * eps, where eps ~ N(0,1)
// eps is pre-generated and passed in.
extern "C" __global__ void reparameterize_kernel(const double* mu, const double* log_var, const double* eps, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = mu[i] + exp(0.5 * log_var[i]) * eps[i];
}

// KL divergence per element: 0.5 * (mu^2 + exp(log_var) - log_var - 1)
extern "C" __global__ void kl_div_kernel(const double* mu, const double* log_var, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = 0.5 * (mu[i] * mu[i] + exp(log_var[i]) - log_var[i] - 1.0);
}

// Fused VAE backward: computes both grad_mu and grad_logvar in one kernel.
// grad_mu_out = grad_z + kl_weight * mu
// grad_lv_out = grad_z * 0.5 * exp(0.5 * log_var) * eps + kl_weight * 0.5 * (exp(log_var) - 1)
extern "C" __global__ void vae_backward_latent_kernel(
    const double* grad_z, const double* mu, const double* log_var, const double* eps,
    double* grad_mu_out, double* grad_lv_out, int n, double kl_weight
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        grad_mu_out[i] = grad_z[i] + kl_weight * mu[i];
        grad_lv_out[i] = grad_z[i] * 0.5 * exp(0.5 * log_var[i]) * eps[i]
                        + kl_weight * 0.5 * (exp(log_var[i]) - 1.0);
    }
}

// Add scalar to diagonal of n x n matrix (in-place)
extern "C" __global__ void add_diag_kernel(double* A, int n, double val) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) A[i * n + i] += val;
}

extern "C" void launch_add_diag(double* A, int n, double val, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(add_diag_kernel, grid, block, 0, stream, A, n, val);
}

extern "C" void launch_reparameterize(const double* mu, const double* log_var, const double* eps, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(reparameterize_kernel, grid, block, 0, stream, mu, log_var, eps, out, n);
}

extern "C" void launch_kl_div(const double* mu, const double* log_var, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(kl_div_kernel, grid, block, 0, stream, mu, log_var, out, n);
}

extern "C" void launch_vae_backward_latent(
    const double* grad_z, const double* mu, const double* log_var, const double* eps,
    double* grad_mu_out, double* grad_lv_out, int n, double kl_weight, hipStream_t stream
) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(vae_backward_latent_kernel, grid, block, 0, stream, grad_z, mu, log_var, eps, grad_mu_out, grad_lv_out, n, kl_weight);
}

extern "C" void launch_log_det_cholesky(const double* L, double* out, int n, hipStream_t stream) {
    hipLaunchKernelGGL(log_det_cholesky_kernel, dim3(1), dim3(1), 0, stream, L, out, n);
}

extern "C" void launch_scaled_exp(const double* x, double* out, int n, double scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(scaled_exp_kernel, grid, block, 0, stream, x, out, n, scale);
}

extern "C" void launch_sigmoid(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(sigmoid_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_sigmoid_backward(const double* grad, const double* act, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(sigmoid_backward_kernel, grid, block, 0, stream, grad, act, out, n);
}

extern "C" void launch_tanh_act(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(tanh_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_tanh_backward(const double* grad, const double* act, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(tanh_backward_kernel, grid, block, 0, stream, grad, act, out, n);
}

extern "C" void launch_relu(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(relu_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_relu_backward(const double* grad, const double* act, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(relu_backward_kernel, grid, block, 0, stream, grad, act, out, n);
}

extern "C" void launch_add(const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(add_kernel, grid, block, 0, stream, a, b, out, n);
}

extern "C" void launch_add_scalar(const double* x, double* out, int n, double s, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(add_scalar_kernel, grid, block, 0, stream, x, out, n, s);
}

extern "C" void launch_div(const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(div_kernel, grid, block, 0, stream, a, b, out, n);
}

extern "C" void launch_fma(const double* x, const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(fma_kernel, grid, block, 0, stream, x, a, b, out, n);
}

extern "C" void launch_mul(const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(mul_kernel, grid, block, 0, stream, a, b, out, n);
}

extern "C" __global__ void mul_inplace_kernel(double* a, const double* b, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) a[i] *= b[i];
}

extern "C" void launch_mul_inplace(double* a, const double* b, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(mul_inplace_kernel, grid, block, 0, stream, a, b, n);
}

// Fused grad + hessian for multiclass GBM, class k.
// grad_out[i] = (probs[i*nc+k] - (targets[i]==k)) * weights[i] * mask[i]
// hess_out[i] = clamp(probs[i*nc+k] * (1-probs[i*nc+k]) * weights[i] * mask[i], 0.001, 1e6)
extern "C" __global__ void grad_hess_kernel(const double* probs, const double* targets, const double* weights, const double* mask, double* grad_out, double* hess_out, int n, int nc, int k) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double pk = probs[i * nc + k];
        double yk = (targets[i] == (double)k) ? 1.0 : 0.0;
        double wi = weights[i];
        double mi = mask[i];
        grad_out[i] = (pk - yk) * wi * mi;
        double h = fmin(fmax(pk * (1.0 - pk) * wi, 0.001), 1e6);
        hess_out[i] = h * mi;
    }
}

extern "C" void launch_grad_hess(const double* probs, const double* targets, const double* weights, const double* mask, double* grad_out, double* hess_out, int n, int nc, int k, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(grad_hess_kernel, grid, block, 0, stream, probs, targets, weights, mask, grad_out, hess_out, n, nc, k);
}

extern "C" __global__ void softmax_ce_grad_kernel(const double* logits, const double* targets, const double* weights, double* grad_out, int n, int nc, double scale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    const double* l = logits + i * nc;
    double* g = grad_out + i * nc;
    double w = weights[i] * scale;
    int tgt = (int)targets[i];
    double mx = l[0];
    for (int k = 1; k < nc; k++) if (l[k] > mx) mx = l[k];
    double sum = 0;
    for (int k = 0; k < nc; k++) sum += exp(l[k] - mx);
    for (int k = 0; k < nc; k++) {
        g[k] = (exp(l[k] - mx) / sum - (k == tgt ? 1.0 : 0.0)) * w;
    }
}

extern "C" void launch_softmax_ce_grad(const double* logits, const double* targets, const double* weights, double* grad_out, int n, int nc, double scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(softmax_ce_grad_kernel, grid, block, 0, stream, logits, targets, weights, grad_out, n, nc, scale);
}

// MSE grad into scratch: grad[i] = pred[i] - target[i]  (f32)
extern "C" __global__ void mse_grad_kernel(const float* pred, const float* target, float* grad, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) grad[i] = pred[i] - target[i];
}

extern "C" void launch_mse_grad(const float* pred, const float* target, float* grad, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(mse_grad_kernel, grid, block, 0, stream, pred, target, grad, n);
}

// Softmax CE gradient for multi-class boosting with separate per-class f32 buffers.
// ptrs: device array of float* pointers, one per class (each [n])
// targets: [n] f32 (class labels, cast to int)
// grad: [n] f32 out — gradient for class k
// hess: [n] f32 out — hessian for class k
extern "C" __global__ void softmax_ce_class_grad_f32_kernel(
    const float** ptrs, const float* targets, float* grad, float* hess,
    int k, int n, int nc
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    float mx = -1e30f;
    for (int c = 0; c < nc; c++) { float v = ptrs[c][i]; if (v > mx) mx = v; }
    float sum = 0.0f;
    for (int c = 0; c < nc; c++) sum += expf(ptrs[c][i] - mx);
    float pk = expf(ptrs[k][i] - mx) / sum;
    grad[i] = pk - (targets[i] == (float)k ? 1.0f : 0.0f);
    hess[i] = fmaxf(pk * (1.0f - pk), 1e-6f);
}

extern "C" void launch_softmax_ce_class_grad_f32(
    const float** ptrs, const float* targets, float* grad, float* hess,
    int k, int n, int nc, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(softmax_ce_class_grad_f32_kernel, grid, block, 0, stream,
        ptrs, targets, grad, hess, k, n, nc);
}

// Logloss (binary CE) gradient for f32 boosting: grad = sigmoid(pred) - target
extern "C" __global__ void logloss_grad_f32_kernel(
    const float* pred, const float* target, float* grad, float* hess, int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    float p = 1.0f / (1.0f + expf(-pred[i]));
    grad[i] = p - target[i];
    hess[i] = fmaxf(p * (1.0f - p), 1e-6f);
}

extern "C" void launch_logloss_grad_f32(
    const float* pred, const float* target, float* grad, float* hess,
    int n, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(logloss_grad_f32_kernel, grid, block, 0, stream,
        pred, target, grad, hess, n);
}

// Argmax over flat f32 buffer, writes best index to out[0] as int
extern "C" __global__ void argmax_f32_kernel(const float* data, int* out, int n) {
    __shared__ float s_val[256];
    __shared__ int s_idx[256];
    int tid = threadIdx.x;
    float best = -1e30f;
    int best_i = 0;
    for (int i = tid; i < n; i += blockDim.x) {
        if (data[i] > best) { best = data[i]; best_i = i; }
    }
    s_val[tid] = best;
    s_idx[tid] = best_i;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s && s_val[tid + s] > s_val[tid]) {
            s_val[tid] = s_val[tid + s];
            s_idx[tid] = s_idx[tid + s];
        }
        __syncthreads();
    }
    if (tid == 0) out[0] = s_idx[0];
}

extern "C" void launch_argmax_f32(const float* data, int* out, int n, hipStream_t stream) {
    hipLaunchKernelGGL(argmax_f32_kernel, 1, 256, 0, stream, data, out, n);
}

// Fill f32 buffer with constant value
extern "C" __global__ void fill_f32_kernel(float* out, float val, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = val;
}

extern "C" void launch_fill_f32(float* out, float val, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(fill_f32_kernel, grid, block, 0, stream, out, val, n);
}

// Write split choice into device arrays at position d (kept for compat)
extern "C" __global__ void write_split_kernel(int* split_feat, unsigned char* split_bin, int feat, unsigned char bin, int d) {
    if (threadIdx.x == 0) { split_feat[d] = feat; split_bin[d] = bin; }
}

extern "C" void launch_write_split(int* split_feat, unsigned char* split_bin, int feat, unsigned char bin, int d, hipStream_t stream) {
    hipLaunchKernelGGL(write_split_kernel, 1, 1, 0, stream, split_feat, split_bin, feat, bin, d);
}

// Fused argmax over gain_buf + decode to (feat, bin) + write split at depth d
// Also writes the argmax index to best_idx[0] for optional host readback
extern "C" __global__ void argmax_write_split_kernel(
    const float* gain, int* split_feat, unsigned char* split_bin,
    int* best_idx_out, int n_features, int n_bins, int d
) {
    __shared__ float s_val[256];
    __shared__ int s_idx[256];
    int tid = threadIdx.x;
    int total = n_features * n_bins;
    float best = -1e30f;
    int best_i = 0;
    for (int i = tid; i < total; i += blockDim.x) {
        if (gain[i] > best) { best = gain[i]; best_i = i; }
    }
    s_val[tid] = best;
    s_idx[tid] = best_i;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s && s_val[tid + s] > s_val[tid]) {
            s_val[tid] = s_val[tid + s];
            s_idx[tid] = s_idx[tid + s];
        }
        __syncthreads();
    }
    if (tid == 0) {
        int idx = s_idx[0];
        int feat = idx / n_bins;
        int bin = idx % n_bins;
        split_feat[d] = feat;
        split_bin[d] = (unsigned char)bin;
        best_idx_out[0] = idx;
    }
}

extern "C" void launch_argmax_write_split(
    const float* gain, int* split_feat, unsigned char* split_bin,
    int* best_idx_out, int n_features, int n_bins, int d, hipStream_t stream
) {
    hipLaunchKernelGGL(argmax_write_split_kernel, 1, 256, 0, stream,
        gain, split_feat, split_bin, best_idx_out, n_features, n_bins, d);
}

extern "C" __global__ void add_col_scaled_kernel(double* matrix, const double* col, int n, int cols, int k, double scale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) matrix[i * cols + k] += col[i] * scale;
}

extern "C" void launch_add_col_scaled(double* matrix, const double* col, int n, int cols, int k, double scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(add_col_scaled_kernel, grid, block, 0, stream, matrix, col, n, cols, k, scale);
}

extern "C" void launch_sub(const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(sub_kernel, grid, block, 0, stream, a, b, out, n);
}

extern "C" void launch_bias_add(const double* x, const double* bias, double* out, int rows, int cols, hipStream_t stream) {
    int n = rows * cols;
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(bias_add_kernel, grid, block, 0, stream, x, bias, out, rows, cols);
}

// Fused LSTM cell: gates (n × 4*hs) + C (n × hs) → update C and H in-place.
// gates layout per sample: [forget(hs), input(hs), cell_cand(hs), output(hs)]
// gates already includes W_x * x_t + W_h @ h + b (pre-computed via gemm + bias_add).
extern "C" __global__ void lstm_cell_kernel(const double* gates, double* c, double* h, int n, int hs) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = n * hs;
    if (idx >= total) return;
    int i = idx / hs;  // sample index
    int j = idx % hs;  // hidden unit
    int base = i * 4 * hs;
    double f_g = 1.0 / (1.0 + exp(-gates[base + j]));           // forget
    double i_g = 1.0 / (1.0 + exp(-gates[base + hs + j]));      // input
    double g_g = tanh(gates[base + 2 * hs + j]);                  // cell candidate
    double o_g = 1.0 / (1.0 + exp(-gates[base + 3 * hs + j]));  // output
    double new_c = f_g * c[idx] + i_g * g_g;
    c[idx] = new_c;
    h[idx] = o_g * tanh(new_c);
}

extern "C" void launch_lstm_cell(const double* gates, double* c, double* h, int n, int hs, hipStream_t stream) {
    int total = n * hs;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(lstm_cell_kernel, grid, block, 0, stream, gates, c, h, n, hs);
}

// Gaussian log-likelihood matrix: out[i,c] = log_prior[c] - 0.5 * sum_j(log(var[c,j]) + (x[i,j]-mean[c,j])^2 / var[c,j])
// x: (n x p), means: (k x p), vars: (k x p), log_priors: (k), out: (n x k)
// One block per (i,c) pair, threads cooperate on the sum over p features.
extern "C" __global__ void gaussian_ll_kernel(
    const double* x, const double* means, const double* vars, const double* log_priors,
    double* out, int n, int k, int p
) {
    int ic = blockIdx.x;
    if (ic >= n * k) return;
    int i = ic / k;
    int c = ic % k;

    // Each thread handles a subset of features, then warp-reduce
    double local_sum = 0.0;
    for (int j = threadIdx.x; j < p; j += blockDim.x) {
        double diff = x[i * p + j] - means[c * p + j];
        double v = vars[c * p + j];
        local_sum += log(v) + diff * diff / v;
    }

    // Warp shuffle reduction
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_sum += __shfl_down(local_sum, offset);

    // Cross-warp reduction via shared memory
    __shared__ double sdata[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) sdata[wid] = local_sum;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_sum = sdata[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (threadIdx.x == 0) {
            out[i * k + c] = log_priors[c] - 0.5 * local_sum;
        }
    }
}

extern "C" void launch_gaussian_ll(
    const double* x, const double* means, const double* vars, const double* log_priors,
    double* out, int n, int k, int p, hipStream_t stream
) {
    int threads = min(256, ((p + 31) / 32) * 32);  // round up to warp size
    if (threads < 32) threads = 32;
    hipLaunchKernelGGL(gaussian_ll_kernel, dim3(n * k), dim3(threads), 0, stream,
        x, means, vars, log_priors, out, n, k, p);
}

// im2col for 1D convolution: X (n x p) → patches (n*out_len x ks)
// where out_len = p - ks + 1. One thread per output element.
extern "C" __global__ void im2col_1d_kernel(
    const double* x, double* patches, int n, int p, int ks, int out_len
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = n * out_len * ks;
    if (idx >= total) return;
    int k = idx % ks;
    int rem = idx / ks;
    int t = rem % out_len;
    int i = rem / out_len;
    patches[idx] = x[i * p + t + k];
}

extern "C" void launch_im2col_1d(
    const double* x, double* patches, int n, int p, int ks, int out_len, hipStream_t stream
) {
    int total = n * out_len * ks;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(im2col_1d_kernel, grid, block, 0, stream, x, patches, n, p, ks, out_len);
}

// Argmax per row: out[i] = argmax_j(x[i * cols + j]) — returns double index
extern "C" __global__ void argmax_rows_kernel(const double* x, double* out, int rows, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= rows) return;
    double best = -1e308;
    int best_j = 0;
    for (int j = 0; j < cols; j++) {
        double v = x[i * cols + j];
        if (v > best) { best = v; best_j = j; }
    }
    out[i] = (double)best_j;
}

extern "C" void launch_argmax_rows(const double* x, double* out, int rows, int cols, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((rows + 255) / 256);
    hipLaunchKernelGGL(argmax_rows_kernel, grid, block, 0, stream, x, out, rows, cols);
}

// ── Leaky ReLU ──────────────────────────────────────────────────────────────

extern "C" __global__ void leaky_relu_kernel(const double* x, double* out, int n, double alpha) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] > 0.0 ? x[i] : alpha * x[i];
}

extern "C" __global__ void leaky_relu_backward_kernel(const double* grad, const double* act, double* out, int n, double alpha) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = act[i] > 0.0 ? grad[i] : alpha * grad[i];
}

extern "C" void launch_leaky_relu(const double* x, double* out, int n, double alpha, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(leaky_relu_kernel, grid, block, 0, stream, x, out, n, alpha);
}

extern "C" void launch_leaky_relu_backward(const double* grad, const double* act, double* out, int n, double alpha, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(leaky_relu_backward_kernel, grid, block, 0, stream, grad, act, out, n, alpha);
}

// ── Layer Normalization ─────────────────────────────────────────────────────
// Row-wise: for each row, compute mean and var, normalize to (x - mean) / sqrt(var + eps)
// gamma/beta (learnable scale/shift) applied if non-null.

extern "C" __global__ void layernorm_kernel(const double* x, double* out,
    const double* gamma, const double* beta, int rows, int cols, double eps) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = x + row * cols;
    double* o = out + row * cols;
    // Compute mean
    double sum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) sum += r[j];
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        sum += __shfl_down(sum, offset);
    __shared__ double smem[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) smem[wid] = sum;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        sum = smem[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            sum += __shfl_down(sum, offset);
        if (threadIdx.x == 0) smem[0] = sum;
    }
    __syncthreads();
    double mean = smem[0] / cols;
    // Compute variance
    double var_acc = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double d = r[j] - mean;
        var_acc += d * d;
    }
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        var_acc += __shfl_down(var_acc, offset);
    if (lane == 0) smem[wid] = var_acc;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        var_acc = smem[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            var_acc += __shfl_down(var_acc, offset);
        if (threadIdx.x == 0) smem[0] = var_acc;
    }
    __syncthreads();
    double inv_std = 1.0 / sqrt(smem[0] / cols + eps);
    // Normalize
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double normed = (r[j] - mean) * inv_std;
        o[j] = gamma ? normed * gamma[j] + beta[j] : normed;
    }
}

extern "C" void launch_layernorm(const double* x, double* out,
    const double* gamma, const double* beta, int rows, int cols, double eps, hipStream_t stream) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(layernorm_kernel, dim3(rows), dim3(threads), 0, stream, x, out, gamma, beta, rows, cols, eps);
}

// ── Dropout ─────────────────────────────────────────────────────────────────
// Mask is pre-generated on CPU and uploaded (same pattern as VAE eps buffer).
// out[i] = mask[i] < p ? 0.0 : x[i] * scale, where scale = 1/(1-p)

extern "C" __global__ void dropout_kernel(const double* x, const double* mask, double* out, int n, double p, double scale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = mask[i] < p ? 0.0 : x[i] * scale;
}

extern "C" void launch_dropout(const double* x, const double* mask, double* out, int n, double p, double scale, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(dropout_kernel, grid, block, 0, stream, x, mask, out, n, p, scale);
}

// Dropout using u8 mask: out[i] = mask[i] ? x[i] * scale : 0.0
extern "C" __global__ void dropout_u8_kernel(const double* x, const unsigned char* mask, double* out, int n, double scale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = mask[i] ? x[i] * scale : 0.0;
}

extern "C" void launch_dropout_u8(const double* x, const unsigned char* mask, double* out, int n, double scale, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(dropout_u8_kernel, grid, block, 0, stream, x, mask, out, n, scale);
}

// ── Concat ──────────────────────────────────────────────────────────────────
// Concatenate two (rows, d1) and (rows, d2) matrices into (rows, d1+d2)

extern "C" __global__ void concat_kernel(const double* a, const double* b, double* out, int rows, int d1, int d2) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = rows * (d1 + d2);
    if (idx >= total) return;
    int row = idx / (d1 + d2);
    int col = idx % (d1 + d2);
    out[idx] = col < d1 ? a[row * d1 + col] : b[row * d2 + (col - d1)];
}

extern "C" void launch_concat(const double* a, const double* b, double* out, int rows, int d1, int d2, hipStream_t stream) {
    int total = rows * (d1 + d2);
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(concat_kernel, grid, block, 0, stream, a, b, out, rows, d1, d2);
}

// ── 2D im2col ───────────────────────────────────────────────────────────────
// Extract (kH*kW*C) patches from (H, W, C) images. Input: (N, C, H, W) in NCHW.
// Output: (N*outH*outW, kH*kW*C) patch matrix for GEMM-based convolution.

extern "C" __global__ void im2col_2d_kernel(
    const double* x, double* patches,
    int N, int C, int H, int W, int kH, int kW, int outH, int outW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * outH * outW * C * kH * kW;
    if (idx >= total) return;
    int patch_size = C * kH * kW;
    int patch_idx = idx / patch_size;  // which patch (N*outH*outW)
    int within = idx % patch_size;     // position within patch
    int n = patch_idx / (outH * outW);
    int hw = patch_idx % (outH * outW);
    int oh = hw / outW;
    int ow = hw % outW;
    int c = within / (kH * kW);
    int kk = within % (kH * kW);
    int kh = kk / kW;
    int kw = kk % kW;
    int ih = oh + kh;
    int iw = ow + kw;
    patches[idx] = x[((n * C + c) * H + ih) * W + iw];
}

extern "C" void launch_im2col_2d(
    const double* x, double* patches,
    int N, int C, int H, int W, int kH, int kW, int outH, int outW, hipStream_t stream
) {
    int total = N * outH * outW * C * kH * kW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(im2col_2d_kernel, grid, block, 0, stream, x, patches, N, C, H, W, kH, kW, outH, outW);
}

// ── Elementwise unary/binary ops ────────────────────────────────────────────

extern "C" __global__ void exp_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = exp(x[i]);
}

extern "C" __global__ void log_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = log(x[i]);
}

extern "C" __global__ void sqrt_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = sqrt(x[i]);
}

extern "C" __global__ void abs_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = fabs(x[i]);
}

extern "C" __global__ void neg_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = -x[i];
}

extern "C" __global__ void sign_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = (x[i] > 0.0) ? 1.0 : ((x[i] < 0.0) ? -1.0 : 0.0);
}

extern "C" __global__ void pow_kernel(const double* x, double* out, int n, double p) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = pow(x[i], p);
}

extern "C" __global__ void clamp_kernel(const double* x, double* out, int n, double lo, double hi) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = fmin(fmax(x[i], lo), hi);
}

extern "C" void launch_exp(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(exp_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_log(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(log_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_sqrt(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(sqrt_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_abs(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(abs_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_neg(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(neg_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_sign(const double* x, double* out, int n, hipStream_t stream) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(sign_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_pow(const double* x, double* out, int n, double p, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(pow_kernel, grid, block, 0, stream, x, out, n, p);
}

extern "C" void launch_clamp(const double* x, double* out, int n, double lo, double hi, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(clamp_kernel, grid, block, 0, stream, x, out, n, lo, hi);
}

// ── Transpose ──────────────────────────────────────────────────────────────
// out[c * rows + r] = in[r * cols + c]

extern "C" __global__ void transpose_kernel(const double* in, double* out, int rows, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < rows * cols) {
        int r = i / cols;
        int c = i % cols;
        out[c * rows + r] = in[r * cols + c];
    }
}

extern "C" void launch_transpose(const double* in, double* out, int rows, int cols, hipStream_t stream) {
    int n = rows * cols;
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(transpose_kernel, grid, block, 0, stream, in, out, rows, cols);
}

// ── Eye (identity matrix) ──────────────────────────────────────────────────

extern "C" __global__ void eye_kernel(double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n * n) out[i] = (i / n == i % n) ? 1.0 : 0.0;
}

extern "C" void launch_eye(double* out, int n, hipStream_t stream) {
    int total = n * n;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(eye_kernel, grid, block, 0, stream, out, n);
}

// ── Where mask (conditional selection) ─────────────────────────────────────
// out[i] = cond[i] != 0.0 ? a[i] : b[i]

extern "C" __global__ void where_mask_kernel(const double* cond, const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = (cond[i] != 0.0) ? a[i] : b[i];
}

extern "C" void launch_where_mask(const double* cond, const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(where_mask_kernel, grid, block, 0, stream, cond, a, b, out, n);
}

// ── Slice rows ─────────────────────────────────────────────────────────────
// Copy rows [start, start+count) from src[total_rows, cols] to dst[count, cols]

extern "C" __global__ void slice_rows_kernel(const double* src, double* dst, int start_row, int count, int cols, int total_n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < count * cols) {
        int r = i / cols;
        int c = i % cols;
        dst[i] = src[(start_row + r) * cols + c];
    }
}

extern "C" void launch_slice_rows(const double* src, double* dst, int start_row, int count, int cols, hipStream_t stream) {
    int n = count * cols;
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(slice_rows_kernel, grid, block, 0, stream, src, dst, start_row, count, cols, n);
}

// broadcast_sub: out[i] = X[i] - v[i % cols]  (subtract row vector from every row)
extern "C" __global__ void broadcast_sub_kernel(const double* x, const double* v, double* out, int n, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] - v[i % cols];
}

extern "C" void launch_broadcast_sub(const double* x, const double* v, double* out, int n, int cols, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(broadcast_sub_kernel, grid, block, 0, stream, x, v, out, n, cols);
}

// broadcast_mul: out[i] = X[i] * v[i % cols]  (multiply every row by row vector)
extern "C" __global__ void broadcast_mul_kernel(const double* x, const double* v, double* out, int n, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] * v[i % cols];
}

extern "C" void launch_broadcast_mul(const double* x, const double* v, double* out, int n, int cols, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(broadcast_mul_kernel, grid, block, 0, stream, x, v, out, n, cols);
}

// broadcast_div: out[i] = X[i] / v[i % cols]  (divide every row by row vector)
extern "C" __global__ void broadcast_div_kernel(const double* x, const double* v, double* out, int n, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] / v[i % cols];
}

extern "C" void launch_broadcast_div(const double* x, const double* v, double* out, int n, int cols, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(broadcast_div_kernel, grid, block, 0, stream, x, v, out, n, cols);
}

// ── Softmax backward ───────────────────────────────────────────────────────
// Row-wise: out_j = softmax_j * (grad_j - dot), where dot = sum_k(grad_k * softmax_k)
// One block per row, cooperative warp shuffle reduction.

extern "C" __global__ void softmax_backward_kernel(
    const double* grad, const double* sm, double* out, int rows, int cols
) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* g = grad + row * cols;
    const double* s = sm + row * cols;
    double* o = out + row * cols;

    double local_dot = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        local_dot += g[j] * s[j];

    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_dot += __shfl_down(local_dot, offset);
    __shared__ double sdot[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) sdot[wid] = local_dot;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_dot = sdot[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_dot += __shfl_down(local_dot, offset);
        if (threadIdx.x == 0) sdot[0] = local_dot;
    }
    __syncthreads();
    double dot = sdot[0];

    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        o[j] = s[j] * (g[j] - dot);
}

extern "C" void launch_softmax_backward(const double* grad, const double* sm, double* out, int rows, int cols, hipStream_t stream) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(softmax_backward_kernel, dim3(rows), dim3(threads), 0, stream, grad, sm, out, rows, cols);
}

// ── Log-softmax (numerically stable) ───────────────────────────────────────
// out_j = x_j - max - log(sum(exp(x - max)))

extern "C" __global__ void log_softmax_rows_kernel(const double* x, double* out, int rows, int cols) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = x + row * cols;
    double* o = out + row * cols;

    double local_max = -1e308;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double v = r[j];
        if (v > local_max) local_max = v;
    }
    for (int offset = warpSize / 2; offset > 0; offset /= 2) {
        double other = __shfl_down(local_max, offset);
        if (other > local_max) local_max = other;
    }
    __shared__ double smax[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) smax[wid] = local_max;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_max = smax[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            double other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
        }
        if (threadIdx.x == 0) smax[0] = local_max;
    }
    __syncthreads();
    double row_max = smax[0];

    double local_sum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        local_sum += exp(r[j] - row_max);
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_sum += __shfl_down(local_sum, offset);
    __shared__ double ssum[32];
    if (lane == 0) ssum[wid] = local_sum;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_sum = ssum[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (threadIdx.x == 0) ssum[0] = local_sum;
    }
    __syncthreads();
    double log_sum_exp = log(ssum[0]);

    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        o[j] = r[j] - row_max - log_sum_exp;
}

extern "C" void launch_log_softmax_rows(const double* x, double* out, int rows, int cols, hipStream_t stream) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(log_softmax_rows_kernel, dim3(rows), dim3(threads), 0, stream, x, out, rows, cols);
}

// ── Cross-entropy (fused log-softmax + NLL) ────────────────────────────────
// targets[i] = class index (as double). Per-sample loss:
// loss[i] = -(logits[i,target] - max - log(sum(exp(logits[i,:] - max))))

extern "C" __global__ void cross_entropy_kernel(
    const double* logits, const double* targets, double* losses, int rows, int cols
) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = logits + row * cols;
    int target_idx = (int)targets[row];

    double local_max = -1e308;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        if (r[j] > local_max) local_max = r[j];
    }
    for (int offset = warpSize / 2; offset > 0; offset /= 2) {
        double other = __shfl_down(local_max, offset);
        if (other > local_max) local_max = other;
    }
    __shared__ double smax[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) smax[wid] = local_max;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_max = smax[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            double other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
        }
        if (threadIdx.x == 0) smax[0] = local_max;
    }
    __syncthreads();
    double row_max = smax[0];

    double local_sum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        local_sum += exp(r[j] - row_max);
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_sum += __shfl_down(local_sum, offset);
    __shared__ double ssum[32];
    if (lane == 0) ssum[wid] = local_sum;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        local_sum = ssum[threadIdx.x];
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (threadIdx.x == 0) ssum[0] = local_sum;
    }
    __syncthreads();

    if (threadIdx.x == 0)
        losses[row] = -(r[target_idx] - row_max - log(ssum[0]));
}

extern "C" void launch_cross_entropy(
    const double* logits, const double* targets, double* losses, int rows, int cols, hipStream_t stream
) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(cross_entropy_kernel, dim3(rows), dim3(threads), 0, stream, logits, targets, losses, rows, cols);
}

// ── Gather rows ────────────────────────────────────────────────────────────
// out[i,:] = table[int(indices[i]),:]  — embedding lookup

extern "C" __global__ void gather_rows_kernel(
    const double* table, const double* indices, double* out, int n, int cols
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * cols) return;
    int i = idx / cols;
    int j = idx % cols;
    int row = (int)indices[i];
    out[idx] = table[row * cols + j];
}

extern "C" void launch_gather_rows(
    const double* table, const double* indices, double* out, int n, int cols, hipStream_t stream
) {
    int total = n * cols;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(gather_rows_kernel, grid, block, 0, stream, table, indices, out, n, cols);
}

// ── Scatter add ────────────────────────────────────────────────────────────
// target[int(indices[i]),:] += src[i,:]  — gradient write-back for embeddings

extern "C" __global__ void scatter_add_kernel(
    double* target, const double* indices, const double* src, int n, int cols
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * cols) return;
    int i = idx / cols;
    int j = idx % cols;
    int row = (int)indices[i];
    atomicAdd(&target[row * cols + j], src[idx]);
}

extern "C" void launch_scatter_add(
    double* target, const double* indices, const double* src, int n, int cols, hipStream_t stream
) {
    int total = n * cols;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(scatter_add_kernel, grid, block, 0, stream, target, indices, src, n, cols);
}

// ── col2im_1d ──────────────────────────────────────────────────────────────
// Reverse of im2col_1d: patches (n*out_len, ks) → image (n, p), out_len = p - ks + 1.
// Overlapping patches accumulate via atomicAdd.

extern "C" __global__ void col2im_1d_kernel(
    const double* patches, double* out, int n, int p, int ks, int out_len
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = n * out_len * ks;
    if (idx >= total) return;
    int k = idx % ks;
    int rem = idx / ks;
    int t = rem % out_len;
    int i = rem / out_len;
    atomicAdd(&out[i * p + t + k], patches[idx]);
}

extern "C" void launch_col2im_1d(
    const double* patches, double* out, int n, int p, int ks, int out_len, hipStream_t stream
) {
    hipMemsetAsync(out, 0, (size_t)n * p * sizeof(double), stream);
    int total = n * out_len * ks;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(col2im_1d_kernel, grid, block, 0, stream, patches, out, n, p, ks, out_len);
}

// ── col2im_2d ──────────────────────────────────────────────────────────────
// Reverse of im2col_2d: patches (N*outH*outW, C*kH*kW) → NCHW image.

extern "C" __global__ void col2im_2d_kernel(
    const double* patches, double* out,
    int N, int C, int H, int W, int kH, int kW, int outH, int outW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * outH * outW * C * kH * kW;
    if (idx >= total) return;
    int patch_size = C * kH * kW;
    int patch_idx = idx / patch_size;
    int within = idx % patch_size;
    int n = patch_idx / (outH * outW);
    int hw = patch_idx % (outH * outW);
    int oh = hw / outW;
    int ow = hw % outW;
    int c = within / (kH * kW);
    int kk = within % (kH * kW);
    int kh = kk / kW;
    int kw = kk % kW;
    int ih = oh + kh;
    int iw = ow + kw;
    atomicAdd(&out[((n * C + c) * H + ih) * W + iw], patches[idx]);
}

extern "C" void launch_col2im_2d(
    const double* patches, double* out,
    int N, int C, int H, int W, int kH, int kW, int outH, int outW, hipStream_t stream
) {
    hipMemsetAsync(out, 0, (size_t)N * C * H * W * sizeof(double), stream);
    int total = N * outH * outW * C * kH * kW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(col2im_2d_kernel, grid, block, 0, stream, patches, out, N, C, H, W, kH, kW, outH, outW);
}

// ── Max pool 1D (forward) ──────────────────────────────────────────────────
// Same convention as avg_pool_1d: input (n*out_len, n_filters) → (n, n_filters)
// Returns both values and argmax indices (as doubles).

extern "C" __global__ void max_pool_1d_kernel(
    const double* input, double* out_vals, double* out_idx, int n, int out_len, int n_filters
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * n_filters) return;
    int i = idx / n_filters;
    int fi = idx % n_filters;
    double best = -1e308;
    int best_t = 0;
    for (int t = 0; t < out_len; t++) {
        double v = input[(i * out_len + t) * n_filters + fi];
        if (v > best) { best = v; best_t = t; }
    }
    out_vals[idx] = best;
    out_idx[idx] = (double)best_t;
}

extern "C" void launch_max_pool_1d(
    const double* input, double* out_vals, double* out_idx, int n, int out_len, int n_filters, hipStream_t stream
) {
    int total = n * n_filters;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(max_pool_1d_kernel, grid, block, 0, stream, input, out_vals, out_idx, n, out_len, n_filters);
}

// ── Max pool 1D backward ───────────────────────────────────────────────────
// Scatter gradient to the max positions recorded during forward.

extern "C" __global__ void max_pool_1d_backward_kernel(
    const double* grad, const double* indices, double* out, int n, int out_len, int n_filters
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * n_filters) return;
    int i = idx / n_filters;
    int fi = idx % n_filters;
    int t = (int)indices[idx];
    out[(i * out_len + t) * n_filters + fi] = grad[idx];
}

extern "C" void launch_max_pool_1d_backward(
    const double* grad, const double* indices, double* out, int n, int out_len, int n_filters, hipStream_t stream
) {
    hipMemsetAsync(out, 0, (size_t)n * out_len * n_filters * sizeof(double), stream);
    int total = n * n_filters;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(max_pool_1d_backward_kernel, grid, block, 0, stream, grad, indices, out, n, out_len, n_filters);
}

// ── Avg pool 2D (forward) ──────────────────────────────────────────────────
// NCHW → NCHW with spatial pooling. outH = (H-kH)/sH+1, outW = (W-kW)/sW+1.

extern "C" __global__ void avg_pool_2d_kernel(
    const double* input, double* output,
    int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C * outH * outW) return;
    int ow = idx % outW; int tmp = idx / outW;
    int oh = tmp % outH; tmp /= outH;
    int c = tmp % C; int n = tmp / C;
    double sum = 0.0;
    int count = 0;
    for (int khi = 0; khi < kH; khi++) {
        for (int kwi = 0; kwi < kW; kwi++) {
            int ih = oh * sH + khi;
            int iw = ow * sW + kwi;
            if (ih < H && iw < W) { sum += input[((n * C + c) * H + ih) * W + iw]; count++; }
        }
    }
    output[idx] = sum / count;
}

extern "C" void launch_avg_pool_2d(
    const double* input, double* output,
    int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW, hipStream_t stream
) {
    int total = N * C * outH * outW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(avg_pool_2d_kernel, grid, block, 0, stream, input, output, N, C, H, W, kH, kW, sH, sW, outH, outW);
}

// ── Avg pool 2D backward ───────────────────────────────────────────────────

extern "C" __global__ void avg_pool_2d_backward_kernel(
    const double* grad_out, double* grad_in,
    int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C * outH * outW) return;
    int ow = idx % outW; int tmp = idx / outW;
    int oh = tmp % outH; tmp /= outH;
    int c = tmp % C; int n = tmp / C;
    int count = 0;
    for (int khi = 0; khi < kH; khi++)
        for (int kwi = 0; kwi < kW; kwi++)
            if (oh * sH + khi < H && ow * sW + kwi < W) count++;
    double val = grad_out[idx] / count;
    for (int khi = 0; khi < kH; khi++) {
        for (int kwi = 0; kwi < kW; kwi++) {
            int ih = oh * sH + khi;
            int iw = ow * sW + kwi;
            if (ih < H && iw < W)
                atomicAdd(&grad_in[((n * C + c) * H + ih) * W + iw], val);
        }
    }
}

extern "C" void launch_avg_pool_2d_backward(
    const double* grad_out, double* grad_in,
    int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW, hipStream_t stream
) {
    hipMemsetAsync(grad_in, 0, (size_t)N * C * H * W * sizeof(double), stream);
    int total = N * C * outH * outW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(avg_pool_2d_backward_kernel, grid, block, 0, stream, grad_out, grad_in, N, C, H, W, kH, kW, sH, sW, outH, outW);
}

// ── Max pool 2D (forward) ──────────────────────────────────────────────────
// Returns pooled values + flat index (ih*W+iw) within each channel's H×W plane.

extern "C" __global__ void max_pool_2d_kernel(
    const double* input, double* out_vals, double* out_idx,
    int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C * outH * outW) return;
    int ow = idx % outW; int tmp = idx / outW;
    int oh = tmp % outH; tmp /= outH;
    int c = tmp % C; int n = tmp / C;
    double best = -1e308;
    int best_pos = 0;
    for (int khi = 0; khi < kH; khi++) {
        for (int kwi = 0; kwi < kW; kwi++) {
            int ih = oh * sH + khi;
            int iw = ow * sW + kwi;
            if (ih < H && iw < W) {
                double v = input[((n * C + c) * H + ih) * W + iw];
                if (v > best) { best = v; best_pos = ih * W + iw; }
            }
        }
    }
    out_vals[idx] = best;
    out_idx[idx] = (double)best_pos;
}

extern "C" void launch_max_pool_2d(
    const double* input, double* out_vals, double* out_idx,
    int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW, hipStream_t stream
) {
    int total = N * C * outH * outW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(max_pool_2d_kernel, grid, block, 0, stream, input, out_vals, out_idx, N, C, H, W, kH, kW, sH, sW, outH, outW);
}

// ── Max pool 2D backward ───────────────────────────────────────────────────
// Scatter gradient back to the max position within each channel's H×W plane.

extern "C" __global__ void max_pool_2d_backward_kernel(
    const double* grad_out, const double* indices, double* grad_in,
    int N, int C, int outH, int outW, int HW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C * outH * outW) return;
    int tmp = idx / (outH * outW);
    int c = tmp % C;
    int n = tmp / C;
    int pos = (int)indices[idx];
    atomicAdd(&grad_in[(n * C + c) * HW + pos], grad_out[idx]);
}

extern "C" void launch_max_pool_2d_backward(
    const double* grad_out, const double* indices, double* grad_in,
    int N, int C, int outH, int outW, int H, int W, hipStream_t stream
) {
    hipMemsetAsync(grad_in, 0, (size_t)N * C * H * W * sizeof(double), stream);
    int total = N * C * outH * outW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(max_pool_2d_backward_kernel, grid, block, 0, stream, grad_out, indices, grad_in, N, C, outH, outW, H * W);
}

// ── Comparison ops ─────────────────────────────────────────────────────────

extern "C" __global__ void gt_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] > b[i] ? 1.0 : 0.0;
}
extern "C" __global__ void lt_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] < b[i] ? 1.0 : 0.0;
}
extern "C" __global__ void eq_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] == b[i] ? 1.0 : 0.0;
}
extern "C" __global__ void gt_scalar_kernel(const double* x, double* out, int n, double val) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] > val ? 1.0 : 0.0;
}
extern "C" __global__ void lt_scalar_kernel(const double* x, double* out, int n, double val) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] < val ? 1.0 : 0.0;
}

extern "C" void launch_gt(const double* a, const double* b, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gt_kernel, gr, bl, 0, s, a, b, out, n); }
extern "C" void launch_lt(const double* a, const double* b, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(lt_kernel, gr, bl, 0, s, a, b, out, n); }
extern "C" void launch_eq(const double* a, const double* b, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(eq_kernel, gr, bl, 0, s, a, b, out, n); }
extern "C" void launch_gt_scalar(const double* x, double* out, int n, double val, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gt_scalar_kernel, gr, bl, 0, s, x, out, n, val); }
extern "C" void launch_lt_scalar(const double* x, double* out, int n, double val, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(lt_scalar_kernel, gr, bl, 0, s, x, out, n, val); }

// ── GELU + backward ───────────────────────────────────────────────────────
// GELU(x) = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))

#define GELU_COEFF 0.7978845608028654  // sqrt(2/pi)
#define GELU_CUBIC 0.044715

extern "C" __global__ void gelu_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double v = x[i];
        double inner = GELU_COEFF * (v + GELU_CUBIC * v * v * v);
        out[i] = 0.5 * v * (1.0 + tanh(inner));
    }
}
extern "C" __global__ void gelu_backward_kernel(const double* grad, const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double v = x[i];
        double inner = GELU_COEFF * (v + GELU_CUBIC * v * v * v);
        double t = tanh(inner);
        double dtanh = 1.0 - t * t;
        double dinner = GELU_COEFF * (1.0 + 3.0 * GELU_CUBIC * v * v);
        out[i] = grad[i] * (0.5 * (1.0 + t) + 0.5 * v * dtanh * dinner);
    }
}
extern "C" void launch_gelu(const double* x, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gelu_kernel, gr, bl, 0, s, x, out, n); }
extern "C" void launch_gelu_backward(const double* grad, const double* x, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gelu_backward_kernel, gr, bl, 0, s, grad, x, out, n); }

// ── SiLU/Swish + backward ─────────────────────────────────────────────────
// SiLU(x) = x * sigmoid(x)

extern "C" __global__ void silu_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { double s = 1.0 / (1.0 + exp(-x[i])); out[i] = x[i] * s; }
}
extern "C" __global__ void silu_backward_kernel(const double* grad, const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double s = 1.0 / (1.0 + exp(-x[i]));
        out[i] = grad[i] * (s + x[i] * s * (1.0 - s));
    }
}
extern "C" void launch_silu(const double* x, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(silu_kernel, gr, bl, 0, s, x, out, n); }
extern "C" void launch_silu_backward(const double* grad, const double* x, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(silu_backward_kernel, gr, bl, 0, s, grad, x, out, n); }

// ── BatchNorm forward (training) ───────────────────────────────────────────
// One block per feature column. Threads cooperate over batch (rows).
// Saves mean and inv_std for backward.

extern "C" __global__ void batchnorm_forward_kernel(
    const double* x, const double* gamma, const double* beta,
    double* out, double* save_mean, double* save_inv_std,
    int N, int C, double eps
) {
    int c = blockIdx.x;
    if (c >= C) return;

    // Mean
    double lsum = 0.0;
    for (int i = threadIdx.x; i < N; i += blockDim.x) lsum += x[i * C + c];
    for (int off = warpSize/2; off > 0; off /= 2) lsum += __shfl_down(lsum, off);
    __shared__ double sm[32];
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
    if (lane == 0) sm[wid] = lsum;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        lsum = sm[threadIdx.x];
        for (int off = warpSize/2; off > 0; off /= 2) lsum += __shfl_down(lsum, off);
        if (threadIdx.x == 0) sm[0] = lsum;
    }
    __syncthreads();
    double mean = sm[0] / N;
    if (threadIdx.x == 0) save_mean[c] = mean;

    // Variance
    double lvar = 0.0;
    for (int i = threadIdx.x; i < N; i += blockDim.x) { double d = x[i*C+c] - mean; lvar += d*d; }
    for (int off = warpSize/2; off > 0; off /= 2) lvar += __shfl_down(lvar, off);
    if (lane == 0) sm[wid] = lvar;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        lvar = sm[threadIdx.x];
        for (int off = warpSize/2; off > 0; off /= 2) lvar += __shfl_down(lvar, off);
        if (threadIdx.x == 0) sm[0] = lvar;
    }
    __syncthreads();
    double inv_std = 1.0 / sqrt(sm[0] / N + eps);
    if (threadIdx.x == 0) save_inv_std[c] = inv_std;

    // Normalize + scale + shift
    double g = gamma[c], b = beta[c];
    for (int i = threadIdx.x; i < N; i += blockDim.x)
        out[i*C+c] = g * (x[i*C+c] - mean) * inv_std + b;
}

extern "C" void launch_batchnorm_forward(
    const double* x, const double* gamma, const double* beta,
    double* out, double* save_mean, double* save_inv_std,
    int N, int C, double eps, hipStream_t stream
) {
    int threads = min(256, max(32, N));
    hipLaunchKernelGGL(batchnorm_forward_kernel, dim3(C), dim3(threads), 0, stream,
        x, gamma, beta, out, save_mean, save_inv_std, N, C, eps);
}

// ── BatchNorm inference ────────────────────────────────────────────────────

extern "C" __global__ void batchnorm_inference_kernel(
    const double* x, const double* gamma, const double* beta,
    const double* run_mean, const double* run_var,
    double* out, int N, int C, double eps
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C) return;
    int c = idx % C;
    out[idx] = gamma[c] * (x[idx] - run_mean[c]) / sqrt(run_var[c] + eps) + beta[c];
}

extern "C" void launch_batchnorm_inference(
    const double* x, const double* gamma, const double* beta,
    const double* run_mean, const double* run_var,
    double* out, int N, int C, double eps, hipStream_t stream
) {
    int total = N * C;
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(batchnorm_inference_kernel, gr, bl, 0, stream, x, gamma, beta, run_mean, run_var, out, N, C, eps);
}

// ── BatchNorm backward ─────────────────────────────────────────────────────
// One block per feature. Two phases: reduce sums, then compute grad_x.

extern "C" __global__ void batchnorm_backward_kernel(
    const double* grad_y, const double* x, const double* save_mean, const double* save_inv_std,
    const double* gamma, double* grad_x, double* grad_gamma, double* grad_beta,
    int N, int C
) {
    int c = blockIdx.x;
    if (c >= C) return;
    double mean = save_mean[c], inv_s = save_inv_std[c], g = gamma[c];

    double ldy = 0.0, ldyx = 0.0;
    for (int i = threadIdx.x; i < N; i += blockDim.x) {
        double dy = grad_y[i*C+c];
        double xhat = (x[i*C+c] - mean) * inv_s;
        ldy += dy; ldyx += dy * xhat;
    }

    // Reduce both sums
    for (int off = warpSize/2; off > 0; off /= 2) { ldy += __shfl_down(ldy, off); ldyx += __shfl_down(ldyx, off); }
    __shared__ double s1[32], s2[32];
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
    if (lane == 0) { s1[wid] = ldy; s2[wid] = ldyx; }
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        ldy = s1[threadIdx.x]; ldyx = s2[threadIdx.x];
        for (int off = warpSize/2; off > 0; off /= 2) { ldy += __shfl_down(ldy, off); ldyx += __shfl_down(ldyx, off); }
        if (threadIdx.x == 0) { s1[0] = ldy; s2[0] = ldyx; }
    }
    __syncthreads();
    double sum_dy = s1[0], sum_dyx = s2[0];
    if (threadIdx.x == 0) { grad_gamma[c] = sum_dyx; grad_beta[c] = sum_dy; }

    double inv_N = 1.0 / N;
    for (int i = threadIdx.x; i < N; i += blockDim.x) {
        double dy = grad_y[i*C+c];
        double xhat = (x[i*C+c] - mean) * inv_s;
        grad_x[i*C+c] = g * inv_s * (dy - sum_dy * inv_N - xhat * sum_dyx * inv_N);
    }
}

extern "C" void launch_batchnorm_backward(
    const double* grad_y, const double* x, const double* save_mean, const double* save_inv_std,
    const double* gamma, double* grad_x, double* grad_gamma, double* grad_beta,
    int N, int C, hipStream_t stream
) {
    int threads = min(256, max(32, N));
    hipLaunchKernelGGL(batchnorm_backward_kernel, dim3(C), dim3(threads), 0, stream,
        grad_y, x, save_mean, save_inv_std, gamma, grad_x, grad_gamma, grad_beta, N, C);
}

// ── LayerNorm backward ─────────────────────────────────────────────────────
// One block per row. Recomputes mean/inv_std. Returns grad_x, accumulates grad_gamma/beta via atomic.

extern "C" __global__ void layernorm_backward_kernel(
    const double* grad_y, const double* x, const double* gamma,
    double* grad_x, double* grad_gamma, double* grad_beta,
    int rows, int cols, double eps
) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* xr = x + row * cols;
    const double* gr = grad_y + row * cols;
    double* gxr = grad_x + row * cols;
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;

    // Recompute mean
    double lm = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) lm += xr[j];
    for (int off = warpSize/2; off > 0; off /= 2) lm += __shfl_down(lm, off);
    __shared__ double smem[32];
    if (lane == 0) smem[wid] = lm;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        lm = smem[threadIdx.x];
        for (int off = warpSize/2; off > 0; off /= 2) lm += __shfl_down(lm, off);
        if (threadIdx.x == 0) smem[0] = lm;
    }
    __syncthreads();
    double mean = smem[0] / cols;

    // Recompute inv_std
    double lv = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) { double d = xr[j] - mean; lv += d*d; }
    for (int off = warpSize/2; off > 0; off /= 2) lv += __shfl_down(lv, off);
    if (lane == 0) smem[wid] = lv;
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        lv = smem[threadIdx.x];
        for (int off = warpSize/2; off > 0; off /= 2) lv += __shfl_down(lv, off);
        if (threadIdx.x == 0) smem[0] = lv;
    }
    __syncthreads();
    double inv_std = 1.0 / sqrt(smem[0] / cols + eps);

    // s1 = sum(dy*gamma), s2 = sum(dy*gamma*xhat), also accumulate grad_gamma/beta
    __shared__ double ss1[32], ss2[32];
    double ls1 = 0.0, ls2 = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double dy = gr[j], xhat = (xr[j] - mean) * inv_std;
        double dxh = dy * gamma[j];
        ls1 += dxh; ls2 += dxh * xhat;
        atomicAdd(&grad_gamma[j], dy * xhat);
        atomicAdd(&grad_beta[j], dy);
    }
    for (int off = warpSize/2; off > 0; off /= 2) { ls1 += __shfl_down(ls1, off); ls2 += __shfl_down(ls2, off); }
    if (lane == 0) { ss1[wid] = ls1; ss2[wid] = ls2; }
    __syncthreads();
    if (threadIdx.x < blockDim.x / warpSize) {
        ls1 = ss1[threadIdx.x]; ls2 = ss2[threadIdx.x];
        for (int off = warpSize/2; off > 0; off /= 2) { ls1 += __shfl_down(ls1, off); ls2 += __shfl_down(ls2, off); }
        if (threadIdx.x == 0) { ss1[0] = ls1; ss2[0] = ls2; }
    }
    __syncthreads();
    double sum1 = ss1[0], sum2 = ss2[0];

    // grad_x
    double inv_D = 1.0 / cols;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        double xhat = (xr[j] - mean) * inv_std;
        double dxh = gr[j] * gamma[j];
        gxr[j] = inv_std * (dxh - sum1 * inv_D - xhat * sum2 * inv_D);
    }
}

extern "C" void launch_layernorm_backward(
    const double* grad_y, const double* x, const double* gamma,
    double* grad_x, double* grad_gamma, double* grad_beta,
    int rows, int cols, double eps, hipStream_t stream
) {
    hipMemsetAsync(grad_gamma, 0, (size_t)cols * sizeof(double), stream);
    hipMemsetAsync(grad_beta, 0, (size_t)cols * sizeof(double), stream);
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(layernorm_backward_kernel, dim3(rows), dim3(threads), 0, stream,
        grad_y, x, gamma, grad_x, grad_gamma, grad_beta, rows, cols, eps);
}

// ── Adam update (fused, in-place) ──────────────────────────────────────────
// m = b1*m + (1-b1)*g, v = b2*v + (1-b2)*g^2
// m_hat = m/(1-b1^t), v_hat = v/(1-b2^t), w -= lr * m_hat / (sqrt(v_hat) + eps)

extern "C" __global__ void adam_update_kernel(
    double* w, double* m, double* v, const double* g,
    double lr, double b1, double b2, double eps, double bc1, double bc2, int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double gi = g[i];
    m[i] = b1 * m[i] + (1.0 - b1) * gi;
    v[i] = b2 * v[i] + (1.0 - b2) * gi * gi;
    double mh = m[i] / bc1;
    double vh = v[i] / bc2;
    w[i] -= lr * mh / (sqrt(vh) + eps);
}

extern "C" void launch_adam_update(
    double* w, double* m, double* v, const double* g,
    double lr, double b1, double b2, double eps, int t, int n, hipStream_t stream
) {
    double bc1 = 1.0 - pow(b1, (double)t);
    double bc2 = 1.0 - pow(b2, (double)t);
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(adam_update_kernel, gr, bl, 0, stream, w, m, v, g, lr, b1, b2, eps, bc1, bc2, n);
}

// ── AdamW update (Adam + decoupled weight decay) ───────────────────────────

extern "C" __global__ void adamw_update_kernel(
    double* w, double* m, double* v, const double* g,
    double lr, double b1, double b2, double eps, double wd, double bc1, double bc2, int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double gi = g[i];
    m[i] = b1 * m[i] + (1.0 - b1) * gi;
    v[i] = b2 * v[i] + (1.0 - b2) * gi * gi;
    double mh = m[i] / bc1;
    double vh = v[i] / bc2;
    w[i] = w[i] * (1.0 - lr * wd) - lr * mh / (sqrt(vh) + eps);
}

extern "C" void launch_adamw_update(
    double* w, double* m, double* v, const double* g,
    double lr, double b1, double b2, double eps, double wd, int t, int n, hipStream_t stream
) {
    double bc1 = 1.0 - pow(b1, (double)t);
    double bc2 = 1.0 - pow(b2, (double)t);
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(adamw_update_kernel, gr, bl, 0, stream, w, m, v, g, lr, b1, b2, eps, wd, bc1, bc2, n);
}

// ── GRU cell ───────────────────────────────────────────────────────────────
// gates layout: [z_pre(hs), r_pre(hs), n_x(hs), n_h(hs)] per sample
// z = sigmoid(z_pre), r = sigmoid(r_pre), n = tanh(n_x + r * n_h)
// h_new = (1-z)*n + z*h

extern "C" __global__ void gru_cell_kernel(const double* gates, const double* h, double* h_new, int n, int hs) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * hs) return;
    int i = idx / hs, j = idx % hs;
    int base = i * 4 * hs;
    double z = 1.0 / (1.0 + exp(-gates[base + j]));
    double r = 1.0 / (1.0 + exp(-gates[base + hs + j]));
    double n_tilde = tanh(gates[base + 2*hs + j] + r * gates[base + 3*hs + j]);
    h_new[idx] = (1.0 - z) * n_tilde + z * h[idx];
}

extern "C" void launch_gru_cell(const double* gates, const double* h, double* h_new, int n, int hs, hipStream_t stream) {
    int total = n * hs;
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(gru_cell_kernel, gr, bl, 0, stream, gates, h, h_new, n, hs);
}

// ── Structural ops ─────────────────────────────────────────────────────────

// Slice columns: dst[r,c] = src[r, start_col + c]
extern "C" __global__ void slice_cols_kernel(const double* src, double* dst, int rows, int src_cols, int start, int count) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * count) return;
    int r = idx / count, c = idx % count;
    dst[idx] = src[r * src_cols + start + c];
}

extern "C" void launch_slice_cols(const double* src, double* dst, int rows, int src_cols, int start, int count, hipStream_t s) {
    int n = rows * count;
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(slice_cols_kernel, gr, bl, 0, s, src, dst, rows, src_cols, start, count);
}

// Causal mask: n x n, lower tri = 0.0, upper tri = fill_val (e.g. -1e10)
extern "C" __global__ void tril_mask_kernel(double* out, int n, double fill_val) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * n) return;
    out[idx] = (idx % n <= idx / n) ? 0.0 : fill_val;
}

extern "C" void launch_tril_mask(double* out, int n, double fill_val, hipStream_t s) {
    int total = n * n;
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(tril_mask_kernel, gr, bl, 0, s, out, n, fill_val);
}

// Fill buffer with constant value
extern "C" __global__ void fill_kernel(double* out, int n, double val) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = val;
}

extern "C" void launch_fill(double* out, int n, double val, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(fill_kernel, gr, bl, 0, s, out, n, val);
}

// Repeat rows: tile (src_rows, cols) pattern vertically `repeats` times
extern "C" __global__ void repeat_rows_kernel(const double* src, double* dst, int src_n, int total) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < total) dst[i] = src[i % src_n];
}

extern "C" void launch_repeat_rows(const double* src, double* dst, int src_n, int total, hipStream_t s) {
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(repeat_rows_kernel, gr, bl, 0, s, src, dst, src_n, total);
}

// Nearest-neighbor upsample 2D: NCHW → NCHW with integer scale factors
extern "C" __global__ void upsample_nearest_2d_kernel(
    const double* input, double* output,
    int N, int C, int H, int W, int outH, int outW, int scaleH, int scaleW
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C * outH * outW) return;
    int ow = idx % outW; int tmp = idx / outW;
    int oh = tmp % outH; tmp /= outH;
    int c = tmp % C; int n = tmp / C;
    output[idx] = input[((n*C+c)*H + oh/scaleH)*W + ow/scaleW];
}

extern "C" void launch_upsample_nearest_2d(
    const double* input, double* output,
    int N, int C, int H, int W, int scaleH, int scaleW, hipStream_t stream
) {
    int outH = H * scaleH, outW = W * scaleW;
    int total = N * C * outH * outW;
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(upsample_nearest_2d_kernel, gr, bl, 0, stream, input, output, N, C, H, W, outH, outW, scaleH, scaleW);
}

// ── Gradient clipping by norm (two-pass) ───────────────────────────────────

extern "C" __global__ void reduce_sum_sq_kernel(const double* x, double* out, int n) {
    __shared__ double sdata[256];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    sdata[tid] = (i < n) ? x[i] * x[i] : 0.0;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s) sdata[tid] += sdata[tid + s];
        __syncthreads();
    }
    if (tid == 0) atomicAdd(out, sdata[0]);
}

extern "C" __global__ void clip_by_norm_kernel(double* x, const double* norm_sq, int n, double max_norm) {
    double norm = sqrt(*norm_sq);
    if (norm <= max_norm) return;
    double scale = max_norm / norm;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) x[i] *= scale;
}

extern "C" void launch_grad_clip_norm(double* x, double* tmp, int n, double max_norm, hipStream_t stream) {
    hipMemsetAsync(tmp, 0, sizeof(double), stream);
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(reduce_sum_sq_kernel, gr, bl, 0, stream, (const double*)x, tmp, n);
    hipLaunchKernelGGL(clip_by_norm_kernel, gr, bl, 0, stream, x, tmp, n, max_norm);
}

// ── Philox 2x32-10 GPU RNG ───────────────────────────────────────────────

__device__ uint2 philox2x32(uint32_t counter, uint32_t key) {
    uint2 state = {counter, key};
    for (int i = 0; i < 10; i++) {
        uint32_t lo = state.x * 0xD2511F53u;
        uint32_t hi = __umulhi(state.x, 0xD2511F53u);
        state.x = hi ^ state.y ^ (key * (i + 1));
        state.y = lo;
    }
    return state;
}

__device__ double philox_uniform(uint32_t idx, uint32_t seed) {
    uint2 r = philox2x32(idx, seed);
    return (double)(r.x) / 4294967296.0;
}

extern "C" __global__ void rand_uniform_kernel(double* out, int n, uint32_t seed) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = philox_uniform(i, seed);
}

extern "C" void launch_rand_uniform(double* out, int n, uint32_t seed, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(rand_uniform_kernel, grid, block, 0, stream, out, n, seed);
}

extern "C" __global__ void bernoulli_u8_kernel(unsigned char* mask, int n, uint32_t seed, double p) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        uint2 r = philox2x32(i, seed);
        double u = (double)(r.x) / 4294967296.0;
        mask[i] = u >= p ? (unsigned char)1 : (unsigned char)0;
    }
}

extern "C" void launch_bernoulli_u8(unsigned char* mask, int n, uint32_t seed, double p, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(bernoulli_u8_kernel, grid, block, 0, stream, mask, n, seed, p);
}

extern "C" __global__ void randn_kernel(double* out, int n, uint32_t seed) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double u1 = philox_uniform(2 * i, seed);
        double u2 = philox_uniform(2 * i + 1, seed);
        u1 = fmax(u1, 1e-30);
        out[i] = sqrt(-2.0 * log(u1)) * cos(2.0 * 3.14159265358979323846 * u2);
    }
}

extern "C" void launch_randn(double* out, int n, uint32_t seed, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(randn_kernel, grid, block, 0, stream, out, n, seed);
}

extern "C" __global__ void bernoulli_kernel(double* out, int n, double p, uint32_t seed) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = philox_uniform(i, seed) < p ? 1.0 : 0.0;
}

extern "C" void launch_bernoulli(double* out, int n, double p, uint32_t seed, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(bernoulli_kernel, grid, block, 0, stream, out, n, p, seed);
}

// In-place row-wise softmax over (n_rows × n_classes) f32 matrix
extern "C" __global__ void softmax_inplace_kernel(float* x, int n_rows, int n_classes) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    float* rp = x + row * n_classes;
    float mx = rp[0];
    for (int c = 1; c < n_classes; c++) if (rp[c] > mx) mx = rp[c];
    float s = 0.0f;
    for (int c = 0; c < n_classes; c++) { rp[c] = expf(rp[c] - mx); s += rp[c]; }
    for (int c = 0; c < n_classes; c++) rp[c] /= s;
}

extern "C" void launch_softmax_inplace(float* x, int n_rows, int n_classes, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(softmax_inplace_kernel, grid, block, 0, stream, x, n_rows, n_classes);
}

// Cross-entropy gradient for contiguous (n_rows × n_classes) pred (post-softmax)
// grad[row * n_classes + c] = pred[row * n_classes + c] - (tgt[row] == c)
// hess[row * n_classes + c] = pred[row * n_classes + c] * (1 - pred[row * n_classes + c])
extern "C" __global__ void logloss_grad_mc_kernel(
    const float* pred, const float* tgt, float* grad, float* hess,
    int n_rows, int n_classes
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    int label = (int)tgt[row];
    for (int c = 0; c < n_classes; c++) {
        float p = pred[row * n_classes + c];
        grad[row * n_classes + c] = p - (label == c ? 1.0f : 0.0f);
        hess[row * n_classes + c] = fmaxf(p * (1.0f - p), 1e-6f);
    }
}

extern "C" void launch_logloss_grad_mc(
    const float* pred, const float* tgt, float* grad, float* hess,
    int n_rows, int n_classes, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(logloss_grad_mc_kernel, grid, block, 0, stream,
        pred, tgt, grad, hess, n_rows, n_classes);
}

// Accuracy: argmax over (n_rows × n_classes) pred vs int tgt, atomicAdd to out[0]
extern "C" __global__ void accuracy_kernel(
    const float* pred, const float* tgt, float* out,
    int n_rows, int n_classes
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    const float* rp = pred + row * n_classes;
    int best = 0;
    for (int c = 1; c < n_classes; c++) if (rp[c] > rp[best]) best = c;
    if (best == (int)tgt[row]) atomicAdd(out, 1.0f);
}

extern "C" void launch_accuracy(
    const float* pred, const float* tgt, float* out,
    int n_rows, int n_classes, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(accuracy_kernel, grid, block, 0, stream,
        pred, tgt, out, n_rows, n_classes);
}

// scatter_add_by_leaf for one class column in (n_rows × n_classes) pred matrix
// pred[row * n_classes + col] += lr * leaf_value[leaf_idx[row]]
extern "C" __global__ void scatter_add_by_leaf_col_kernel(
    float* pred, const unsigned char* leaf_idx, const float* leaf_value,
    float lr, int n_rows, int n_classes, int col
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    pred[row * n_classes + col] += lr * leaf_value[leaf_idx[row]];
}

extern "C" void launch_scatter_add_by_leaf_col(
    float* pred, const unsigned char* leaf_idx, const float* leaf_value,
    float lr, int n_rows, int n_classes, int col, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(scatter_add_by_leaf_col_kernel, grid, block, 0, stream,
        pred, leaf_idx, leaf_value, lr, n_rows, n_classes, col);
}
