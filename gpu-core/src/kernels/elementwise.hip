#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"

template<typename T> __global__ void relu_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] > 0.0 ? x[i] : 0.0;
}

extern "C" __global__ void relu_backward_kernel(const double* grad, const double* act, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = act[i] > 0.0 ? grad[i] : 0.0;
}

template<typename T> __global__ void add_kernel(const T* a, const T* b, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] + b[i];
}

template<typename T> __global__ void add_scalar_kernel(const T* x, T* out, int n, const T* s_ptr) {
    T s = *s_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] + s;
}

extern "C" __global__ void div_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] / b[i];
}

template<typename T> __global__ void fma_kernel(const T* x, const T* a, const T* b, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] * a[i] + b[i];
}

template<typename T> __global__ void bias_add_kernel(const T* x, const T* bias, T* out, int rows, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < rows * cols) {
        int col = i % cols;
        out[i] = x[i] + bias[col];
    }
}

extern "C" __global__ void mul_kernel(const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] * b[i];
}

template<typename T> __global__ void sub_kernel(const T* a, const T* b, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = a[i] - b[i];
}

template<typename T> __global__ void softmax_rows_kernel(const T* x, T* out, int rows, int cols) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const T* row_in = x + row * cols;
    T* row_out = out + row * cols;

    T local_max = -1e308;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        T v = row_in[j];
        if (v > local_max) local_max = v;
    }
    for (int offset = warpSize / 2; offset > 0; offset /= 2) {
        T other = __shfl_down(local_max, offset);
        if (other > local_max) local_max = other;
    }
    __shared__ T smax[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) smax[wid] = local_max;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_max = (lane < nwarps) ? smax[lane] : -1e308;
        for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            T other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
        }
        if (lane == 0) smax[0] = local_max;
    }
    __syncthreads();
    T row_max = smax[0];

    T local_sum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        T e = exp(row_in[j] - row_max);
        row_out[j] = e;
        local_sum += e;
    }
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_sum += __shfl_down(local_sum, offset);
    __shared__ T ssum[32];
    if (lane == 0) ssum[wid] = local_sum;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_sum = (lane < nwarps) ? ssum[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (lane == 0) ssum[0] = local_sum;
    }
    __syncthreads();
    T row_sum = ssum[0];

    T inv_sum = 1.0 / row_sum;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        row_out[j] *= inv_sum;
    }
}

template<typename T> __global__ void sub_scale_kernel(const T* a, const T* b, T* out, int n, const T* scale_p) {
    T scale = *scale_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = (a[i] - b[i]) * scale;
}

extern "C" void launch_softmax_rows(const double* x, double* out, int rows, int cols, hipStream_t stream, int dtype) {
    int threads = min(256, cols);
    if (threads < 32) threads = 32;
    DT_DISPATCH(dtype, softmax_rows_kernel<T><<<dim3(rows), dim3(threads), 0, stream>>>((const T*)x, (T*)out, rows, cols));
}

extern "C" void launch_sub_scale(const double* a, const double* b, double* out, int n, const double* scale, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, sub_scale_kernel<T><<<grid, block, 0, stream>>>((const T*)a, (const T*)b, (T*)out, n, (const T*)scale));
}

template<typename T> __global__ void sigmoid_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = 1.0 / (1.0 + exp(-x[i]));
}

extern "C" __global__ void sigmoid_backward_kernel(const double* grad, const double* act, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = grad[i] * act[i] * (1.0 - act[i]);
}

template<typename T> __global__ void tanh_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = tanh(x[i]);
}

extern "C" __global__ void tanh_backward_kernel(const double* grad, const double* act, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = grad[i] * (1.0 - act[i] * act[i]);
}

extern "C" __global__ void scaled_exp_kernel(const double* x, double* out, int n, const double* scale_ptr) {
    double scale = *scale_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = exp(scale * x[i]);
}

extern "C" __global__ void log_det_cholesky_kernel(const double* L, double* out, int n) {
    if (threadIdx.x == 0 && blockIdx.x == 0) {
        double sum = 0.0;
        for (int i = 0; i < n; i++) {
            sum += log(L[i * n + i]);
        }
        out[0] = 2.0 * sum;
    }
}

extern "C" __global__ void reparameterize_kernel(const double* mu, const double* log_var, const double* eps, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = mu[i] + exp(0.5 * log_var[i]) * eps[i];
}

extern "C" __global__ void kl_div_kernel(const double* mu, const double* log_var, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = 0.5 * (mu[i] * mu[i] + exp(log_var[i]) - log_var[i] - 1.0);
}

extern "C" __global__ void vae_backward_latent_kernel(
    const double* grad_z, const double* mu, const double* log_var, const double* eps,
    double* grad_mu_out, double* grad_lv_out, int n, const double* kl_weight_ptr
) {
    double kl_weight = *kl_weight_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        grad_mu_out[i] = grad_z[i] + kl_weight * mu[i];
        grad_lv_out[i] = grad_z[i] * 0.5 * exp(0.5 * log_var[i]) * eps[i]
                        + kl_weight * 0.5 * (exp(log_var[i]) - 1.0);
    }
}

extern "C" __global__ void add_diag_kernel(double* A, int n, const double* val_ptr) {
    double val = *val_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) A[i * n + i] += val;
}

extern "C" void launch_add_diag(double* A, int n, const double* val, hipStream_t stream) {
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
    double* grad_mu_out, double* grad_lv_out, int n, const double* kl_weight, hipStream_t stream
) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(vae_backward_latent_kernel, grid, block, 0, stream, grad_z, mu, log_var, eps, grad_mu_out, grad_lv_out, n, kl_weight);
}

extern "C" void launch_log_det_cholesky(const double* L, double* out, int n, hipStream_t stream) {
    hipLaunchKernelGGL(log_det_cholesky_kernel, dim3(1), dim3(1), 0, stream, L, out, n);
}

extern "C" void launch_scaled_exp(const double* x, double* out, int n, const double* scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(scaled_exp_kernel, grid, block, 0, stream, x, out, n, scale);
}

extern "C" void launch_sigmoid(const double* x, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, sigmoid_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n));
}

extern "C" void launch_sigmoid_backward(const double* grad, const double* act, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(sigmoid_backward_kernel, grid, block, 0, stream, grad, act, out, n);
}

extern "C" void launch_tanh_act(const double* x, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, tanh_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n));
}

extern "C" void launch_tanh_backward(const double* grad, const double* act, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(tanh_backward_kernel, grid, block, 0, stream, grad, act, out, n);
}

extern "C" void launch_relu(const double* x, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, relu_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n));
}

extern "C" void launch_relu_backward(const double* grad, const double* act, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(relu_backward_kernel, grid, block, 0, stream, grad, act, out, n);
}

extern "C" void launch_add(const double* a, const double* b, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, add_kernel<T><<<grid, block, 0, stream>>>((const T*)a, (const T*)b, (T*)out, n));
}

extern "C" void launch_add_scalar(const double* x, double* out, int n, const double* s, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, add_scalar_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n, (const T*)s));
}

extern "C" void launch_div(const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(div_kernel, grid, block, 0, stream, a, b, out, n);
}

extern "C" void launch_fma(const double* x, const double* a, const double* b, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, fma_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (const T*)a, (const T*)b, (T*)out, n));
}

extern "C" void launch_mul(const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(mul_kernel, grid, block, 0, stream, a, b, out, n);
}

template<typename T> __global__ void mul_inplace_kernel(T* a, const T* b, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) a[i] *= b[i];
}

extern "C" void launch_mul_inplace(double* a, const double* b, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, mul_inplace_kernel<T><<<grid, block, 0, stream>>>((T*)a, (const T*)b, n));
}

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

extern "C" __global__ void softmax_ce_grad_kernel(const double* logits, const double* targets, const double* weights, double* grad_out, int n, int nc, const double* scale_p) {
    double scale = *scale_p;
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

extern "C" void launch_softmax_ce_grad(const double* logits, const double* targets, const double* weights, double* grad_out, int n, int nc, const double* scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(softmax_ce_grad_kernel, grid, block, 0, stream, logits, targets, weights, grad_out, n, nc, scale);
}

extern "C" __global__ void mse_grad_kernel(const float* pred, const float* target, float* grad, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) grad[i] = pred[i] - target[i];
}

extern "C" void launch_mse_grad(const float* pred, const float* target, float* grad, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(mse_grad_kernel, grid, block, 0, stream, pred, target, grad, n);
}

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

extern "C" __global__ void fill_f32_kernel(float* out, const float* val_p, int n) {
    float val = *val_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = val;
}

extern "C" void launch_fill_f32(float* out, const float* val, int n, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(fill_f32_kernel, grid, block, 0, stream, out, val, n);
}

extern "C" __global__ void write_split_kernel(int* split_feat, unsigned char* split_bin, int feat, unsigned char bin, int d) {
    if (threadIdx.x == 0) { split_feat[d] = feat; split_bin[d] = bin; }
}

extern "C" void launch_write_split(int* split_feat, unsigned char* split_bin, int feat, unsigned char bin, int d, hipStream_t stream) {
    hipLaunchKernelGGL(write_split_kernel, 1, 1, 0, stream, split_feat, split_bin, feat, bin, d);
}

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

extern "C" __global__ void add_col_scaled_kernel(double* matrix, const double* col, int n, int cols, int k, const double* scale_p) {
    double scale = *scale_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) matrix[i * cols + k] += col[i] * scale;
}

extern "C" void launch_add_col_scaled(double* matrix, const double* col, int n, int cols, int k, const double* scale, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(add_col_scaled_kernel, grid, block, 0, stream, matrix, col, n, cols, k, scale);
}

extern "C" void launch_sub(const double* a, const double* b, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, sub_kernel<T><<<grid, block, 0, stream>>>((const T*)a, (const T*)b, (T*)out, n));
}

extern "C" void launch_bias_add(const double* x, const double* bias, double* out, int rows, int cols, hipStream_t stream, int dtype) {
    int n = rows * cols;
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, bias_add_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (const T*)bias, (T*)out, rows, cols));
}

extern "C" __global__ void lstm_cell_kernel(const double* gates, double* c, double* h, int n, int hs) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = n * hs;
    if (idx >= total) return;
    int i = idx / hs;
    int j = idx % hs;
    int base = i * 4 * hs;
    double f_g = 1.0 / (1.0 + exp(-gates[base + j]));
    double i_g = 1.0 / (1.0 + exp(-gates[base + hs + j]));
    double g_g = tanh(gates[base + 2 * hs + j]);
    double o_g = 1.0 / (1.0 + exp(-gates[base + 3 * hs + j]));
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

extern "C" __global__ void gaussian_ll_kernel(
    const double* x, const double* means, const double* vars, const double* log_priors,
    double* out, int n, int k, int p
) {
    int ic = blockIdx.x;
    if (ic >= n * k) return;
    int i = ic / k;
    int c = ic % k;

    double local_sum = 0.0;
    for (int j = threadIdx.x; j < p; j += blockDim.x) {
        double diff = x[i * p + j] - means[c * p + j];
        double v = vars[c * p + j];
        local_sum += log(v) + diff * diff / v;
    }

    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        local_sum += __shfl_down(local_sum, offset);

    __shared__ double sdata[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) sdata[wid] = local_sum;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_sum = (lane < nwarps) ? sdata[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (lane == 0) {
            out[i * k + c] = log_priors[c] - 0.5 * local_sum;
        }
    }
}

extern "C" void launch_gaussian_ll(
    const double* x, const double* means, const double* vars, const double* log_priors,
    double* out, int n, int k, int p, hipStream_t stream
) {
    int threads = min(256, ((p + 31) / 32) * 32);
    if (threads < 32) threads = 32;
    hipLaunchKernelGGL(gaussian_ll_kernel, dim3(n * k), dim3(threads), 0, stream,
        x, means, vars, log_priors, out, n, k, p);
}

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


template<typename T> __global__ void copy_f64_kernel(const T* __restrict__ src, T* __restrict__ dst, long long n) {
    long long i = (long long)blockIdx.x * blockDim.x + threadIdx.x;
    long long stride = (long long)gridDim.x * blockDim.x;
    for (; i < n; i += stride) dst[i] = src[i];
}

extern "C" void launch_copy_f64(const double* src, double* dst, long long n, hipStream_t stream, int dtype) {
    dim3 block(256);
    long long want = (n + 255) / 256;
    dim3 grid((unsigned)(want > 65535 ? 65535 : (want < 1 ? 1 : want)));
    DT_DISPATCH(dtype, copy_f64_kernel<T><<<grid, block, 0, stream>>>((const T*)src, (T*)dst, n));
}


template<typename T> __global__ void leaky_relu_kernel(const T* x, T* out, int n, const T* alpha_ptr) {
    T alpha = *alpha_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] > 0.0 ? x[i] : alpha * x[i];
}

extern "C" __global__ void leaky_relu_backward_kernel(const double* grad, const double* act, double* out, int n, const double* alpha_ptr) {
    double alpha = *alpha_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = act[i] > 0.0 ? grad[i] : alpha * grad[i];
}

extern "C" void launch_leaky_relu(const double* x, double* out, int n, const double* alpha, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, leaky_relu_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n, (const T*)alpha));
}

extern "C" void launch_leaky_relu_backward(const double* grad, const double* act, double* out, int n, const double* alpha, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(leaky_relu_backward_kernel, grid, block, 0, stream, grad, act, out, n, alpha);
}


template<typename T> __global__ void layernorm_kernel(const T* x, T* out,
    const T* gamma, const T* beta, int rows, int cols, const T* eps_p) {
    T eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    const T* r = x + row * cols;
    T* o = out + row * cols;
    T sum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) sum += r[j];
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        sum += __shfl_down(sum, offset);
    __shared__ T smem[32];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) smem[wid] = sum;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        sum = (lane < nwarps) ? smem[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            sum += __shfl_down(sum, offset);
        if (lane == 0) smem[0] = sum;
    }
    __syncthreads();
    T mean = smem[0] / cols;
    T var_acc = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        T d = r[j] - mean;
        var_acc += d * d;
    }
    for (int offset = warpSize / 2; offset > 0; offset /= 2)
        var_acc += __shfl_down(var_acc, offset);
    if (lane == 0) smem[wid] = var_acc;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        var_acc = (lane < nwarps) ? smem[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            var_acc += __shfl_down(var_acc, offset);
        if (lane == 0) smem[0] = var_acc;
    }
    __syncthreads();
    T inv_std = 1.0 / sqrt(smem[0] / cols + eps);
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        T normed = (r[j] - mean) * inv_std;
        o[j] = gamma ? normed * gamma[j] + (beta ? beta[j] : 0.0) : normed;
    }
}

extern "C" void launch_layernorm(const double* x, double* out,
    const double* gamma, const double* beta, int rows, int cols, const double* eps, hipStream_t stream, int dtype) {
    int threads = min(256, max(32, cols));
    DT_DISPATCH(dtype, layernorm_kernel<T><<<dim3(rows), dim3(threads), 0, stream>>>((const T*)x, (T*)out, (const T*)gamma, (const T*)beta, rows, cols, (const T*)eps));
}


extern "C" __global__ void dropout_kernel(const double* x, const double* mask, double* out, int n, const double* p_p, const double* scale_p) {
    double p = *p_p;
    double scale = *scale_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = mask[i] < p ? 0.0 : x[i] * scale;
}

extern "C" void launch_dropout(const double* x, const double* mask, double* out, int n, const double* p, const double* scale, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(dropout_kernel, grid, block, 0, stream, x, mask, out, n, p, scale);
}

extern "C" __global__ void dropout_u8_kernel(const double* x, const unsigned char* mask, double* out, int n, const double* scale_ptr) {
    double scale = *scale_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = mask[i] ? x[i] * scale : 0.0;
}

extern "C" void launch_dropout_u8(const double* x, const unsigned char* mask, double* out, int n, const double* scale, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(dropout_u8_kernel, grid, block, 0, stream, x, mask, out, n, scale);
}


template<typename T> __global__ void concat_kernel(const T* a, const T* b, T* out, int rows, int d1, int d2) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = rows * (d1 + d2);
    if (idx >= total) return;
    int row = idx / (d1 + d2);
    int col = idx % (d1 + d2);
    out[idx] = col < d1 ? a[row * d1 + col] : b[row * d2 + (col - d1)];
}

extern "C" void launch_concat(const double* a, const double* b, double* out, int rows, int d1, int d2, hipStream_t stream, int dtype) {
    int total = rows * (d1 + d2);
    dim3 block(256); dim3 grid((total + 255) / 256);
    DT_DISPATCH(dtype, concat_kernel<T><<<grid, block, 0, stream>>>((const T*)a, (const T*)b, (T*)out, rows, d1, d2));
}


template<typename T> __global__ void slice_lead_kernel(const T* src, T* out, int rows, int src_cols, int take) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = rows * take;
    if (idx >= total) return;
    int row = idx / take;
    int col = idx % take;
    out[idx] = src[row * src_cols + col];
}

extern "C" void launch_slice_lead(const double* src, double* out, int rows, int src_cols, int take, hipStream_t stream, int dtype) {
    int total = rows * take;
    dim3 block(256); dim3 grid((total + 255) / 256);
    DT_DISPATCH(dtype, slice_lead_kernel<T><<<grid, block, 0, stream>>>((const T*)src, (T*)out, rows, src_cols, take));
}


extern "C" __global__ void im2col_2d_kernel(
    const double* x, double* patches,
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


extern "C" __global__ void exp_kernel(const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = exp(x[i]);
}

template<typename T> __global__ void log_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = log(x[i]);
}

template<typename T> __global__ void sqrt_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = sqrt(x[i]);
}

template<typename T> __global__ void abs_kernel(const T* x, T* out, int n) {
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

extern "C" __global__ void pow_kernel(const double* x, double* out, int n, const double* p_ptr) {
    double p = *p_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = pow(x[i], p);
}

template<typename T> __global__ void clamp_kernel(const T* x, T* out, int n, const T* lo_ptr, const T* hi_ptr) {
    T lo = *lo_ptr;
    T hi = *hi_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = fmin(fmax(x[i], lo), hi);
}

extern "C" void launch_exp(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(exp_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_log(const double* x, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, log_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n));
}

extern "C" void launch_sqrt(const double* x, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, sqrt_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n));
}

extern "C" void launch_abs(const double* x, double* out, int n, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, abs_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n));
}

extern "C" void launch_neg(const double* x, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(neg_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_sign(const double* x, double* out, int n, hipStream_t stream) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(sign_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" void launch_pow(const double* x, double* out, int n, const double* p, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(pow_kernel, grid, block, 0, stream, x, out, n, p);
}

extern "C" void launch_clamp(const double* x, double* out, int n, const double* lo, const double* hi, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, clamp_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (T*)out, n, (const T*)lo, (const T*)hi));
}


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


extern "C" __global__ void eye_kernel(double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n * n) out[i] = (i / n == i % n) ? 1.0 : 0.0;
}

extern "C" void launch_eye(double* out, int n, hipStream_t stream) {
    int total = n * n;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(eye_kernel, grid, block, 0, stream, out, n);
}


extern "C" __global__ void where_mask_kernel(const double* cond, const double* a, const double* b, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = (cond[i] != 0.0) ? a[i] : b[i];
}

extern "C" void launch_where_mask(const double* cond, const double* a, const double* b, double* out, int n, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(where_mask_kernel, grid, block, 0, stream, cond, a, b, out, n);
}


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

template<typename T> __global__ void broadcast_sub_kernel(const T* x, const T* v, T* out, int n, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] - v[i % cols];
}

extern "C" void launch_broadcast_sub(const double* x, const double* v, double* out, int n, int cols, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, broadcast_sub_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (const T*)v, (T*)out, n, cols));
}

template<typename T> __global__ void broadcast_mul_kernel(const T* x, const T* v, T* out, int n, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] * v[i % cols];
}

extern "C" void launch_broadcast_mul(const double* x, const double* v, double* out, int n, int cols, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, broadcast_mul_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (const T*)v, (T*)out, n, cols));
}

template<typename T> __global__ void broadcast_div_kernel(const T* x, const T* v, T* out, int n, int cols) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] / v[i % cols];
}

extern "C" void launch_broadcast_div(const double* x, const double* v, double* out, int n, int cols, hipStream_t stream, int dtype) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    DT_DISPATCH(dtype, broadcast_div_kernel<T><<<grid, block, 0, stream>>>((const T*)x, (const T*)v, (T*)out, n, cols));
}


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
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_dot = (lane < nwarps) ? sdot[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_dot += __shfl_down(local_dot, offset);
        if (lane == 0) sdot[0] = local_dot;
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
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_max = (lane < nwarps) ? smax[lane] : -1e308;
        for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            double other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
        }
        if (lane == 0) smax[0] = local_max;
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
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_sum = (lane < nwarps) ? ssum[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (lane == 0) ssum[0] = local_sum;
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
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_max = (lane < nwarps) ? smax[lane] : -1e308;
        for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            double other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
        }
        if (lane == 0) smax[0] = local_max;
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
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        local_sum = (lane < nwarps) ? ssum[lane] : 0.0;
        for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
        if (lane == 0) ssum[0] = local_sum;
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


template<typename T> __global__ void gather_rows_kernel(
    const T* table, const T* indices, T* out, int n, int cols
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * cols) return;
    int i = idx / cols;
    int j = idx % cols;
    int row = (int)indices[i];
    out[idx] = table[row * cols + j];
}

extern "C" void launch_gather_rows(
    const double* table, const double* indices, double* out, int n, int cols, hipStream_t stream, int dtype
) {
    int total = n * cols;
    dim3 block(256); dim3 grid((total + 255) / 256);
    DT_DISPATCH(dtype, gather_rows_kernel<T><<<grid, block, 0, stream>>>((const T*)table, (const T*)indices, (T*)out, n, cols));
}


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
    int total = n * out_len * ks;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(col2im_1d_kernel, grid, block, 0, stream, patches, out, n, p, ks, out_len);
}


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
    int total = N * outH * outW * C * kH * kW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(col2im_2d_kernel, grid, block, 0, stream, patches, out, N, C, H, W, kH, kW, outH, outW);
}


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
    int total = n * n_filters;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(max_pool_1d_backward_kernel, grid, block, 0, stream, grad, indices, out, n, out_len, n_filters);
}


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
    int total = N * C * outH * outW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(avg_pool_2d_backward_kernel, grid, block, 0, stream, grad_out, grad_in, N, C, H, W, kH, kW, sH, sW, outH, outW);
}


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
    int total = N * C * outH * outW;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(max_pool_2d_backward_kernel, grid, block, 0, stream, grad_out, indices, grad_in, N, C, outH, outW, H * W);
}


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
extern "C" __global__ void gt_scalar_kernel(const double* x, double* out, int n, const double* val_p) {
    double val = *val_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] > val ? 1.0 : 0.0;
}
extern "C" __global__ void lt_scalar_kernel(const double* x, double* out, int n, const double* val_p) {
    double val = *val_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = x[i] < val ? 1.0 : 0.0;
}

extern "C" void launch_gt(const double* a, const double* b, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gt_kernel, gr, bl, 0, s, a, b, out, n); }
extern "C" void launch_lt(const double* a, const double* b, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(lt_kernel, gr, bl, 0, s, a, b, out, n); }
extern "C" void launch_eq(const double* a, const double* b, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(eq_kernel, gr, bl, 0, s, a, b, out, n); }
extern "C" void launch_gt_scalar(const double* x, double* out, int n, const double* val, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gt_scalar_kernel, gr, bl, 0, s, x, out, n, val); }
extern "C" void launch_lt_scalar(const double* x, double* out, int n, const double* val, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(lt_scalar_kernel, gr, bl, 0, s, x, out, n, val); }


#define GELU_COEFF 0.7978845608028654
#define GELU_CUBIC 0.044715

template<typename T> __global__ void gelu_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        T v = x[i];
        T inner = GELU_COEFF * (v + GELU_CUBIC * v * v * v);
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
extern "C" void launch_gelu(const double* x, double* out, int n, hipStream_t s, int dtype) {
    dim3 bl(256); dim3 gr((n+255)/256); DT_DISPATCH(dtype, gelu_kernel<T><<<gr, bl, 0, s>>>((const T*)x, (T*)out, n)); }
extern "C" void launch_gelu_backward(const double* grad, const double* x, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(gelu_backward_kernel, gr, bl, 0, s, grad, x, out, n); }


template<typename T> __global__ void silu_kernel(const T* x, T* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) { T s = 1.0 / (1.0 + exp(-x[i])); out[i] = x[i] * s; }
}
extern "C" __global__ void silu_backward_kernel(const double* grad, const double* x, double* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        double s = 1.0 / (1.0 + exp(-x[i]));
        out[i] = grad[i] * (s + x[i] * s * (1.0 - s));
    }
}
extern "C" void launch_silu(const double* x, double* out, int n, hipStream_t s, int dtype) {
    dim3 bl(256); dim3 gr((n+255)/256); DT_DISPATCH(dtype, silu_kernel<T><<<gr, bl, 0, s>>>((const T*)x, (T*)out, n)); }
extern "C" void launch_silu_backward(const double* grad, const double* x, double* out, int n, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256); hipLaunchKernelGGL(silu_backward_kernel, gr, bl, 0, s, grad, x, out, n); }


extern "C" __global__ void batchnorm_forward_kernel(
    const double* x, const double* gamma, const double* beta,
    double* out, double* save_mean, double* save_inv_std,
    int N, int C, const double* eps_p
) {
    double eps = *eps_p;
    int c = blockIdx.x;
    if (c >= C) return;

    double lsum = 0.0;
    for (int i = threadIdx.x; i < N; i += blockDim.x) lsum += x[i * C + c];
    for (int off = warpSize/2; off > 0; off /= 2) lsum += __shfl_down(lsum, off);
    __shared__ double sm[32];
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
    if (lane == 0) sm[wid] = lsum;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        lsum = (lane < nwarps) ? sm[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) lsum += __shfl_down(lsum, off);
        if (lane == 0) sm[0] = lsum;
    }
    __syncthreads();
    double mean = sm[0] / N;
    if (threadIdx.x == 0) save_mean[c] = mean;

    double lvar = 0.0;
    for (int i = threadIdx.x; i < N; i += blockDim.x) { double d = x[i*C+c] - mean; lvar += d*d; }
    for (int off = warpSize/2; off > 0; off /= 2) lvar += __shfl_down(lvar, off);
    if (lane == 0) sm[wid] = lvar;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        lvar = (lane < nwarps) ? sm[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) lvar += __shfl_down(lvar, off);
        if (lane == 0) sm[0] = lvar;
    }
    __syncthreads();
    double inv_std = 1.0 / sqrt(sm[0] / N + eps);
    if (threadIdx.x == 0) save_inv_std[c] = inv_std;

    double g = gamma[c], b = beta[c];
    for (int i = threadIdx.x; i < N; i += blockDim.x)
        out[i*C+c] = g * (x[i*C+c] - mean) * inv_std + b;
}

extern "C" void launch_batchnorm_forward(
    const double* x, const double* gamma, const double* beta,
    double* out, double* save_mean, double* save_inv_std,
    int N, int C, const double* eps, hipStream_t stream
) {
    int threads = min(256, max(32, N));
    hipLaunchKernelGGL(batchnorm_forward_kernel, dim3(C), dim3(threads), 0, stream,
        x, gamma, beta, out, save_mean, save_inv_std, N, C, eps);
}


extern "C" __global__ void batchnorm_inference_kernel(
    const double* x, const double* gamma, const double* beta,
    const double* run_mean, const double* run_var,
    double* out, int N, int C, const double* eps_p
) {
    double eps = *eps_p;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= N * C) return;
    int c = idx % C;
    out[idx] = gamma[c] * (x[idx] - run_mean[c]) / sqrt(run_var[c] + eps) + beta[c];
}

extern "C" void launch_batchnorm_inference(
    const double* x, const double* gamma, const double* beta,
    const double* run_mean, const double* run_var,
    double* out, int N, int C, const double* eps, hipStream_t stream
) {
    int total = N * C;
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(batchnorm_inference_kernel, gr, bl, 0, stream, x, gamma, beta, run_mean, run_var, out, N, C, eps);
}


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

    for (int off = warpSize/2; off > 0; off /= 2) { ldy += __shfl_down(ldy, off); ldyx += __shfl_down(ldyx, off); }
    __shared__ double s1[32], s2[32];
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
    if (lane == 0) { s1[wid] = ldy; s2[wid] = ldyx; }
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        ldy = (lane < nwarps) ? s1[lane] : 0.0; ldyx = (lane < nwarps) ? s2[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) { ldy += __shfl_down(ldy, off); ldyx += __shfl_down(ldyx, off); }
        if (lane == 0) { s1[0] = ldy; s2[0] = ldyx; }
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


extern "C" __global__ void layernorm_backward_kernel(
    const double* grad_y, const double* x, const double* gamma,
    double* grad_x, double* grad_gamma, double* grad_beta,
    int rows, int cols, const double* eps_p
) {
    double eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* xr = x + row * cols;
    const double* gr = grad_y + row * cols;
    double* gxr = grad_x + row * cols;
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;

    double lm = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) lm += xr[j];
    for (int off = warpSize/2; off > 0; off /= 2) lm += __shfl_down(lm, off);
    __shared__ double smem[32];
    if (lane == 0) smem[wid] = lm;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        lm = (lane < nwarps) ? smem[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) lm += __shfl_down(lm, off);
        if (lane == 0) smem[0] = lm;
    }
    __syncthreads();
    double mean = smem[0] / cols;

    double lv = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) { double d = xr[j] - mean; lv += d*d; }
    for (int off = warpSize/2; off > 0; off /= 2) lv += __shfl_down(lv, off);
    if (lane == 0) smem[wid] = lv;
    __syncthreads();
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        lv = (lane < nwarps) ? smem[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) lv += __shfl_down(lv, off);
        if (lane == 0) smem[0] = lv;
    }
    __syncthreads();
    double inv_std = 1.0 / sqrt(smem[0] / cols + eps);

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
    if (wid == 0) {
        int nwarps = (blockDim.x + warpSize - 1) / warpSize;
        ls1 = (lane < nwarps) ? ss1[lane] : 0.0; ls2 = (lane < nwarps) ? ss2[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) { ls1 += __shfl_down(ls1, off); ls2 += __shfl_down(ls2, off); }
        if (lane == 0) { ss1[0] = ls1; ss2[0] = ls2; }
    }
    __syncthreads();
    double sum1 = ss1[0], sum2 = ss2[0];

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
    int rows, int cols, const double* eps, hipStream_t stream
) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(layernorm_backward_kernel, dim3(rows), dim3(threads), 0, stream,
        grad_y, x, gamma, grad_x, grad_gamma, grad_beta, rows, cols, eps);
}


extern "C" __global__ void adam_update_kernel(
    double* w, double* m, double* v, const double* g,
    const double* lr_p, const double* b1_p, const double* b2_p, const double* eps_p, int t, int n
) {
    double lr = *lr_p, b1 = *b1_p, b2 = *b2_p, eps = *eps_p;
    double bc1 = 1.0 - pow(b1, (double)t);
    double bc2 = 1.0 - pow(b2, (double)t);
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
    const double* lr, const double* b1, const double* b2, const double* eps, int t, int n, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(adam_update_kernel, gr, bl, 0, stream, w, m, v, g, lr, b1, b2, eps, t, n);
}


extern "C" __global__ void adamw_update_kernel(
    double* w, double* m, double* v, const double* g,
    const double* lr_p, const double* b1_p, const double* b2_p, const double* eps_p, const double* wd_p, int t, int n
) {
    double lr = *lr_p, b1 = *b1_p, b2 = *b2_p, eps = *eps_p, wd = *wd_p;
    double bc1 = 1.0 - pow(b1, (double)t);
    double bc2 = 1.0 - pow(b2, (double)t);
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
    const double* lr, const double* b1, const double* b2, const double* eps, const double* wd, int t, int n, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(adamw_update_kernel, gr, bl, 0, stream, w, m, v, g, lr, b1, b2, eps, wd, t, n);
}


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


template<typename T> __global__ void slice_cols_kernel(const T* src, T* dst, int rows, int src_cols, int start, int count) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= rows * count) return;
    int r = idx / count, c = idx % count;
    dst[idx] = src[r * src_cols + start + c];
}

extern "C" void launch_slice_cols(const double* src, double* dst, int rows, int src_cols, int start, int count, hipStream_t s, int dtype) {
    int n = rows * count;
    dim3 bl(256); dim3 gr((n+255)/256);
    DT_DISPATCH(dtype, slice_cols_kernel<T><<<gr, bl, 0, s>>>((const T*)src, (T*)dst, rows, src_cols, start, count));
}

extern "C" __global__ void tril_mask_kernel(double* out, int n, const double* fill_val_p) {
    double fill_val = *fill_val_p;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * n) return;
    out[idx] = (idx % n <= idx / n) ? 0.0 : fill_val;
}

extern "C" void launch_tril_mask(double* out, int n, const double* fill_val, hipStream_t s) {
    int total = n * n;
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(tril_mask_kernel, gr, bl, 0, s, out, n, fill_val);
}

extern "C" __global__ void fill_kernel(double* out, int n, const double* val_p) {
    double val = *val_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = val;
}

extern "C" void launch_fill(double* out, int n, const double* val, hipStream_t s) {
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(fill_kernel, gr, bl, 0, s, out, n, val);
}

extern "C" __global__ void repeat_rows_kernel(const double* src, double* dst, int src_n, int total) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < total) dst[i] = src[i % src_n];
}

extern "C" void launch_repeat_rows(const double* src, double* dst, int src_n, int total, hipStream_t s) {
    dim3 bl(256); dim3 gr((total+255)/256);
    hipLaunchKernelGGL(repeat_rows_kernel, gr, bl, 0, s, src, dst, src_n, total);
}

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

extern "C" __global__ void clip_by_norm_kernel(double* x, const double* norm_sq, int n, const double* max_norm_p) {
    double max_norm = *max_norm_p;
    double norm = sqrt(*norm_sq);
    if (norm <= max_norm) return;
    double scale = max_norm / norm;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) x[i] *= scale;
}

extern "C" void launch_grad_clip_norm(double* x, double* tmp, int n, const double* max_norm, hipStream_t stream) {
    dim3 bl(256); dim3 gr((n+255)/256);
    hipLaunchKernelGGL(reduce_sum_sq_kernel, gr, bl, 0, stream, (const double*)x, tmp, n);
    hipLaunchKernelGGL(clip_by_norm_kernel, gr, bl, 0, stream, x, tmp, n, max_norm);
}


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

extern "C" __global__ void bernoulli_u8_kernel(unsigned char* mask, int n, uint32_t seed, const double* p_ptr) {
    double p = *p_ptr;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        uint2 r = philox2x32(i, seed);
        double u = (double)(r.x) / 4294967296.0;
        mask[i] = u >= p ? (unsigned char)1 : (unsigned char)0;
    }
}

extern "C" void launch_bernoulli_u8(unsigned char* mask, int n, uint32_t seed, const double* p, hipStream_t stream) {
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

extern "C" __global__ void bernoulli_kernel(double* out, int n, const double* p_p, uint32_t seed) {
    double p = *p_p;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = philox_uniform(i, seed) < p ? 1.0 : 0.0;
}

extern "C" void launch_bernoulli(double* out, int n, const double* p, uint32_t seed, hipStream_t stream) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(bernoulli_kernel, grid, block, 0, stream, out, n, p, seed);
}

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

extern "C" __global__ void scatter_add_by_leaf_col_kernel(
    float* pred, const unsigned char* leaf_idx, const float* leaf_value,
    const float* lr_p, int n_rows, int n_classes, int col
) {
    float lr = *lr_p;
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    pred[row * n_classes + col] += lr * leaf_value[leaf_idx[row]];
}

extern "C" void launch_scatter_add_by_leaf_col(
    float* pred, const unsigned char* leaf_idx, const float* leaf_value,
    const float* lr, int n_rows, int n_classes, int col, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(scatter_add_by_leaf_col_kernel, grid, block, 0, stream,
        pred, leaf_idx, leaf_value, lr, n_rows, n_classes, col);
}
