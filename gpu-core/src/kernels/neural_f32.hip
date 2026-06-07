#include <hip/hip_runtime.h>
#include <hip/hip_fp16.h>

// ── f32 neural network hot-path kernels ────────────────────────────────────
// All kernels use float (not double). Launchers take float* typed pointers.
// f16 kernels work on __half stored as uint16_t.

#define GRID256(n) dim3 block(256); dim3 grid(((n) + 255) / 256)

// ── ReLU f32 ───────────────────────────────────────────────────────────────
extern "C" __global__ void relu_f32_kernel(const float* x, float* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[i] > 0.0f ? x[i] : 0.0f;
}
// backward: grad * (act > 0)
extern "C" __global__ void relu_backward_f32_kernel(const float* grad, const float* act, float* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = act[i] > 0.0f ? grad[i] : 0.0f;
}
extern "C" void launch_relu_f32(const float* x, float* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(relu_f32_kernel, grid, block, 0, stream, x, out, n);
}
extern "C" void launch_relu_backward_f32(const float* grad, const float* act, float* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(relu_backward_f32_kernel, grid, block, 0, stream, grad, act, out, n);
}

// ── GELU f32 (tanh approximation matching the f64 version) ────────────────
#define GELU_COEFF_F 0.7978845608028654f
#define GELU_CUBIC_F 0.044715f

extern "C" __global__ void gelu_f32_kernel(const float* x, float* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            float v = x[i];
            float inner = GELU_COEFF_F * (v + GELU_CUBIC_F * v * v * v);
            out[i] = 0.5f * v * (1.0f + tanhf(inner));
      }
}
extern "C" __global__ void gelu_backward_f32_kernel(const float* grad, const float* x, float* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            float v = x[i];
            float inner = GELU_COEFF_F * (v + GELU_CUBIC_F * v * v * v);
            float t = tanhf(inner);
            float dtanh = 1.0f - t * t;
            float dinner = GELU_COEFF_F * (1.0f + 3.0f * GELU_CUBIC_F * v * v);
            out[i] = grad[i] * (0.5f * (1.0f + t) + 0.5f * v * dtanh * dinner);
      }
}
extern "C" void launch_gelu_f32(const float* x, float* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(gelu_f32_kernel, grid, block, 0, stream, x, out, n);
}
extern "C" void launch_gelu_backward_f32(const float* grad, const float* x, float* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(gelu_backward_f32_kernel, grid, block, 0, stream, grad, x, out, n);
}

// ── Bias add f32: out[i] = x[i] + bias[i % cols] ─────────────────────────
extern "C" __global__ void bias_add_f32_kernel(const float* x, const float* bias, float* out, int rows, int cols) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < rows * cols) out[i] = x[i] + bias[i % cols];
}
extern "C" void launch_bias_add_f32(const float* x, const float* bias, float* out, int rows, int cols, hipStream_t stream) {
      int n = rows * cols;
      GRID256(n); hipLaunchKernelGGL(bias_add_f32_kernel, grid, block, 0, stream, x, bias, out, rows, cols);
}

// ── Repeat rows f32: broadcast src[cols] into dst[total] (fill dst with repeating src) ──
// Used in gpu_linear_f32 to pre-fill bias before sgemm beta=1.
extern "C" __global__ void repeat_rows_f32_kernel(const float* src, float* dst, int cols, int total) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < total) dst[i] = src[i % cols];
}
extern "C" void launch_repeat_rows_f32(const float* src, float* dst, int cols, int total, hipStream_t stream) {
      GRID256(total); hipLaunchKernelGGL(repeat_rows_f32_kernel, grid, block, 0, stream, src, dst, cols, total);
}

// ── LayerNorm f32 ─────────────────────────────────────────────────────────
// One block per row. gamma/beta are required (never null in this API).
extern "C" __global__ void layernorm_f32_kernel(
      const float* x, float* out,
      const float* gamma, const float* beta,
      int rows, int cols, float eps
) {
      int row = blockIdx.x;
      if (row >= rows) return;
      const float* r = x + row * cols;
      float* o = out + row * cols;

      float sum = 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) sum += r[j];
      for (int off = warpSize / 2; off > 0; off /= 2) sum += __shfl_down(sum, off);
      __shared__ float smem[32];
      int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
      if (lane == 0) smem[wid] = sum;
      __syncthreads();
      if (threadIdx.x < blockDim.x / warpSize) {
            sum = smem[threadIdx.x];
            for (int off = warpSize / 2; off > 0; off /= 2) sum += __shfl_down(sum, off);
            if (threadIdx.x == 0) smem[0] = sum;
      }
      __syncthreads();
      float mean = smem[0] / cols;

      float var_acc = 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) { float d = r[j] - mean; var_acc += d * d; }
      for (int off = warpSize / 2; off > 0; off /= 2) var_acc += __shfl_down(var_acc, off);
      if (lane == 0) smem[wid] = var_acc;
      __syncthreads();
      if (threadIdx.x < blockDim.x / warpSize) {
            var_acc = smem[threadIdx.x];
            for (int off = warpSize / 2; off > 0; off /= 2) var_acc += __shfl_down(var_acc, off);
            if (threadIdx.x == 0) smem[0] = var_acc;
      }
      __syncthreads();
      float inv_std = 1.0f / sqrtf(smem[0] / cols + eps);

      for (int j = threadIdx.x; j < cols; j += blockDim.x) {
            float normed = (r[j] - mean) * inv_std;
            o[j] = gamma[j] * normed + beta[j];
      }
}
extern "C" void launch_layernorm_f32(
      const float* x, float* out,
      const float* gamma, const float* beta,
      int rows, int cols, float eps, hipStream_t stream
) {
      int threads = min(256, max(32, cols));
      hipLaunchKernelGGL(layernorm_f32_kernel, dim3(rows), dim3(threads), 0, stream,
            x, out, gamma, beta, rows, cols, eps);
}

// ── LayerNorm backward f32 ────────────────────────────────────────────────
// One block per row. Accumulates grad_gamma/beta via atomicAdd.
extern "C" __global__ void layernorm_backward_f32_kernel(
      const float* grad_y, const float* x, const float* gamma,
      float* grad_x, float* grad_gamma, float* grad_beta,
      int rows, int cols, float eps
) {
      int row = blockIdx.x;
      if (row >= rows) return;
      const float* xr = x + row * cols;
      const float* gr = grad_y + row * cols;
      float* gxr = grad_x + row * cols;
      int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;

      float lm = 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) lm += xr[j];
      for (int off = warpSize/2; off > 0; off /= 2) lm += __shfl_down(lm, off);
      __shared__ float smem[32];
      if (lane == 0) smem[wid] = lm;
      __syncthreads();
      if (threadIdx.x < blockDim.x / warpSize) {
            lm = smem[threadIdx.x];
            for (int off = warpSize/2; off > 0; off /= 2) lm += __shfl_down(lm, off);
            if (threadIdx.x == 0) smem[0] = lm;
      }
      __syncthreads();
      float mean = smem[0] / cols;

      float lv = 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) { float d = xr[j] - mean; lv += d*d; }
      for (int off = warpSize/2; off > 0; off /= 2) lv += __shfl_down(lv, off);
      if (lane == 0) smem[wid] = lv;
      __syncthreads();
      if (threadIdx.x < blockDim.x / warpSize) {
            lv = smem[threadIdx.x];
            for (int off = warpSize/2; off > 0; off /= 2) lv += __shfl_down(lv, off);
            if (threadIdx.x == 0) smem[0] = lv;
      }
      __syncthreads();
      float inv_std = 1.0f / sqrtf(smem[0] / cols + eps);

      __shared__ float ss1[32], ss2[32];
      float ls1 = 0.0f, ls2 = 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) {
            float dy = gr[j], xhat = (xr[j] - mean) * inv_std;
            float dxh = dy * gamma[j];
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
      float sum1 = ss1[0], sum2 = ss2[0];

      float inv_D = 1.0f / cols;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) {
            float xhat = (xr[j] - mean) * inv_std;
            float dxh = gr[j] * gamma[j];
            gxr[j] = inv_std * (dxh - sum1 * inv_D - xhat * sum2 * inv_D);
      }
}
extern "C" void launch_layernorm_backward_f32(
      const float* grad_y, const float* x, const float* gamma,
      float* grad_x, float* grad_gamma, float* grad_beta,
      int rows, int cols, float eps, hipStream_t stream
) {
      int threads = min(256, max(32, cols));
      hipLaunchKernelGGL(layernorm_backward_f32_kernel, dim3(rows), dim3(threads), 0, stream,
            grad_y, x, gamma, grad_x, grad_gamma, grad_beta, rows, cols, eps);
}

// ── Avg pool 2D f32 ───────────────────────────────────────────────────────
extern "C" __global__ void avg_pool_2d_f32_kernel(
      const float* input, float* output,
      int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= N * C * outH * outW) return;
      int ow = idx % outW; int tmp = idx / outW;
      int oh = tmp % outH; tmp /= outH;
      int c = tmp % C; int n = tmp / C;
      float sum = 0.0f; int count = 0;
      for (int khi = 0; khi < kH; khi++) {
            for (int kwi = 0; kwi < kW; kwi++) {
                  int ih = oh * sH + khi, iw = ow * sW + kwi;
                  if (ih < H && iw < W) { sum += input[((n * C + c) * H + ih) * W + iw]; count++; }
            }
      }
      output[idx] = sum / count;
}
extern "C" void launch_avg_pool_2d_f32(
      const float* input, float* output,
      int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW, hipStream_t stream
) {
      int total = N * C * outH * outW;
      GRID256(total); hipLaunchKernelGGL(avg_pool_2d_f32_kernel, grid, block, 0, stream,
            input, output, N, C, H, W, kH, kW, sH, sW, outH, outW);
}

// ── Avg pool 2D backward f32 ──────────────────────────────────────────────
extern "C" __global__ void avg_pool_2d_backward_f32_kernel(
      const float* grad_out, float* grad_in,
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
      float val = grad_out[idx] / count;
      for (int khi = 0; khi < kH; khi++) {
            for (int kwi = 0; kwi < kW; kwi++) {
                  int ih = oh * sH + khi, iw = ow * sW + kwi;
                  if (ih < H && iw < W)
                        atomicAdd(&grad_in[((n * C + c) * H + ih) * W + iw], val);
            }
      }
}
extern "C" void launch_avg_pool_2d_backward_f32(
      const float* grad_out, float* grad_in,
      int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW, hipStream_t stream
) {
      int total = N * C * outH * outW;
      GRID256(total); hipLaunchKernelGGL(avg_pool_2d_backward_f32_kernel, grid, block, 0, stream,
            grad_out, grad_in, N, C, H, W, kH, kW, sH, sW, outH, outW);
}

// ── Max pool 2D f32 ───────────────────────────────────────────────────────
// out_idx stores the flat index (ih*W+iw) as float bits encoding an int.
extern "C" __global__ void max_pool_2d_f32_kernel(
      const float* input, float* out_vals, float* out_idx,
      int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= N * C * outH * outW) return;
      int ow = idx % outW; int tmp = idx / outW;
      int oh = tmp % outH; tmp /= outH;
      int c = tmp % C; int n = tmp / C;
      float best = -3.402823466e+38f; int best_pos = 0;
      for (int khi = 0; khi < kH; khi++) {
            for (int kwi = 0; kwi < kW; kwi++) {
                  int ih = oh * sH + khi, iw = ow * sW + kwi;
                  if (ih < H && iw < W) {
                        float v = input[((n * C + c) * H + ih) * W + iw];
                        if (v > best) { best = v; best_pos = ih * W + iw; }
                  }
            }
      }
      out_vals[idx] = best;
      out_idx[idx] = (float)best_pos;
}
extern "C" void launch_max_pool_2d_f32(
      const float* input, float* out_vals, float* out_idx,
      int N, int C, int H, int W, int kH, int kW, int sH, int sW, int outH, int outW, hipStream_t stream
) {
      int total = N * C * outH * outW;
      GRID256(total); hipLaunchKernelGGL(max_pool_2d_f32_kernel, grid, block, 0, stream,
            input, out_vals, out_idx, N, C, H, W, kH, kW, sH, sW, outH, outW);
}

// ── Max pool 2D backward f32 ──────────────────────────────────────────────
extern "C" __global__ void max_pool_2d_backward_f32_kernel(
      const float* grad_out, const float* indices, float* grad_in,
      int N, int C, int outH, int outW, int HW
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= N * C * outH * outW) return;
      int tmp = idx / (outH * outW);
      int c = tmp % C, n = tmp / C;
      int pos = (int)indices[idx];
      atomicAdd(&grad_in[(n * C + c) * HW + pos], grad_out[idx]);
}
extern "C" void launch_max_pool_2d_backward_f32(
      const float* grad_out, const float* indices, float* grad_in,
      int N, int C, int outH, int outW, int H, int W, hipStream_t stream
) {
      int total = N * C * outH * outW;
      GRID256(total); hipLaunchKernelGGL(max_pool_2d_backward_f32_kernel, grid, block, 0, stream,
            grad_out, indices, grad_in, N, C, outH, outW, H * W);
}

// ── LSTM cell f32 ─────────────────────────────────────────────────────────
// gates layout per sample: [forget(hs), input(hs), cell_cand(hs), output(hs)]
extern "C" __global__ void lstm_cell_f32_kernel(const float* gates, float* c, float* h, int n, int hs) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= n * hs) return;
      int i = idx / hs, j = idx % hs;
      int base = i * 4 * hs;
      float f_g = 1.0f / (1.0f + expf(-gates[base + j]));
      float i_g = 1.0f / (1.0f + expf(-gates[base + hs + j]));
      float g_g = tanhf(gates[base + 2 * hs + j]);
      float o_g = 1.0f / (1.0f + expf(-gates[base + 3 * hs + j]));
      float new_c = f_g * c[idx] + i_g * g_g;
      c[idx] = new_c;
      h[idx] = o_g * tanhf(new_c);
}
extern "C" void launch_lstm_cell_f32(const float* gates, float* c, float* h, int n, int hs, hipStream_t stream) {
      int total = n * hs;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(lstm_cell_f32_kernel, grid, block, 0, stream, gates, c, h, n, hs);
}

// ── GRU cell f32 ──────────────────────────────────────────────────────────
// gates layout: [z_pre(hs), r_pre(hs), n_x(hs), n_h(hs)] per sample
extern "C" __global__ void gru_cell_f32_kernel(const float* gates, const float* h, float* h_new, int n, int hs) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= n * hs) return;
      int i = idx / hs, j = idx % hs;
      int base = i * 4 * hs;
      float z = 1.0f / (1.0f + expf(-gates[base + j]));
      float r = 1.0f / (1.0f + expf(-gates[base + hs + j]));
      float n_tilde = tanhf(gates[base + 2*hs + j] + r * gates[base + 3*hs + j]);
      h_new[idx] = (1.0f - z) * n_tilde + z * h[idx];
}
extern "C" void launch_gru_cell_f32(const float* gates, const float* h, float* h_new, int n, int hs, hipStream_t stream) {
      int total = n * hs;
      dim3 bl(256); dim3 gr((total + 255) / 256);
      hipLaunchKernelGGL(gru_cell_f32_kernel, gr, bl, 0, stream, gates, h, h_new, n, hs);
}

// ── f16 elementwise kernels ───────────────────────────────────────────────
// Scalar __half — load to float, compute, store back.

extern "C" __global__ void relu_f16_kernel(const __half* x, __half* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            float v = __half2float(x[i]);
            out[i] = __float2half(v > 0.0f ? v : 0.0f);
      }
}
extern "C" void launch_relu_f16(const __half* x, __half* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(relu_f16_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" __global__ void gelu_f16_kernel(const __half* x, __half* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            float v = __half2float(x[i]);
            float inner = GELU_COEFF_F * (v + GELU_CUBIC_F * v * v * v);
            out[i] = __float2half(0.5f * v * (1.0f + tanhf(inner)));
      }
}
extern "C" void launch_gelu_f16(const __half* x, __half* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(gelu_f16_kernel, grid, block, 0, stream, x, out, n);
}

extern "C" __global__ void add_f16_kernel(const __half* a, const __half* b, __half* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = __float2half(__half2float(a[i]) + __half2float(b[i]));
}
extern "C" void launch_add_f16(const __half* a, const __half* b, __half* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(add_f16_kernel, grid, block, 0, stream, a, b, out, n);
}

extern "C" __global__ void mul_f16_kernel(const __half* a, const __half* b, __half* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = __float2half(__half2float(a[i]) * __half2float(b[i]));
}
extern "C" void launch_mul_f16(const __half* a, const __half* b, __half* out, int n, hipStream_t stream) {
      GRID256(n); hipLaunchKernelGGL(mul_f16_kernel, grid, block, 0, stream, a, b, out, n);
}
