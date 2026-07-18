#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"


__global__ void attn_sdp_kernel(
      const float* q, const float* k, const float* v,
      float* out,
      int seq, int dim, int causal
) {
      int batch_row = blockIdx.x;
      int br = batch_row / seq;
      int qi = batch_row % seq;

      const float* q_row = q + batch_row * dim;
      const float* k_base = k + br * seq * dim;
      const float* v_base = v + br * seq * dim;
      float* o_row = out + batch_row * dim;

      float scale = 1.0f / sqrtf((float)dim);
      int key_limit = causal ? (qi + 1) : seq;

      extern __shared__ float sh_acc[];
      for (int d = threadIdx.x; d < dim; d += blockDim.x)
            sh_acc[d] = 0.0f;
      __syncthreads();

      float m = -1e30f;
      float l = 0.0f;

      for (int j = 0; j < key_limit; j++) {
            const float* k_row = k_base + j * dim;
            float local_dot = 0.0f;
            for (int d = threadIdx.x; d < dim; d += blockDim.x)
                  local_dot += q_row[d] * k_row[d];
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                  local_dot += __shfl_down(local_dot, offset);
            __shared__ float s_dot[32];
            int lane = threadIdx.x % warpSize;
            int wid  = threadIdx.x / warpSize;
            if (lane == 0) s_dot[wid] = local_dot;
            __syncthreads();
            int nwarps = (blockDim.x + warpSize - 1) / warpSize;
            if (wid == 0) {
                  float v2 = (lane < nwarps) ? s_dot[lane] : 0.0f;
                  for (int offset = warpSize / 2; offset > 0; offset /= 2)
                        v2 += __shfl_down(v2, offset);
                  if (lane == 0) s_dot[0] = v2;
            }
            __syncthreads();
            float s = s_dot[0] * scale;

            float m_new = (s > m) ? s : m;
            float rescale = expf(m - m_new);
            float e_s = expf(s - m_new);

            const float* v_row = v_base + j * dim;
            for (int d = threadIdx.x; d < dim; d += blockDim.x)
                  sh_acc[d] = sh_acc[d] * rescale + v_row[d] * e_s;
            l = l * rescale + e_s;
            m = m_new;
            __syncthreads();
      }

      float inv_l = (l > 0.0f) ? 1.0f / l : 0.0f;
      for (int d = threadIdx.x; d < dim; d += blockDim.x)
            o_row[d] = sh_acc[d] * inv_l;
}

extern "C" void launch_scaled_dot_product_attention(
      const float* q, const float* k, const float* v, float* out,
      int n_rows, int seq, int dim, int causal, hipStream_t stream
) {
      int total_blocks = n_rows * seq;
      int threads = 128;
      size_t shared_bytes = (size_t)dim * sizeof(float);
      hipLaunchKernelGGL(
            attn_sdp_kernel,
            dim3(total_blocks), dim3(threads), shared_bytes, stream,
            q, k, v, out, seq, dim, causal
      );
}


__global__ void attn_causal_softmax_kernel(float* x, int rows, int cols) {
      int row = blockIdx.x;
      if (row >= rows) return;
      float* row_ptr = x + row * cols;

      float local_max = -1e30f;
      for (int j = threadIdx.x; j <= row && j < cols; j += blockDim.x) {
            float v = row_ptr[j];
            if (v > local_max) local_max = v;
      }
      for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            float other = __shfl_down(local_max, offset);
            if (other > local_max) local_max = other;
      }
      __shared__ float s_max[32];
      int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
      if (lane == 0) s_max[wid] = local_max;
      __syncthreads();
      int nwarps_max = (blockDim.x + warpSize - 1) / warpSize;
      if (wid == 0) {
            float v = (lane < nwarps_max) ? s_max[lane] : -1e30f;
            for (int offset = warpSize / 2; offset > 0; offset /= 2) {
                  float other = __shfl_down(v, offset);
                  if (other > v) v = other;
            }
            if (lane == 0) s_max[0] = v;
      }
      __syncthreads();
      float row_max = s_max[0];

      float local_sum = 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x) {
            float e = (j <= row) ? expf(row_ptr[j] - row_max) : 0.0f;
            row_ptr[j] = e;
            local_sum += e;
      }
      for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sum += __shfl_down(local_sum, offset);
      __shared__ float s_sum[32];
      if (lane == 0) s_sum[wid] = local_sum;
      __syncthreads();
      int nwarps_sum = (blockDim.x + warpSize - 1) / warpSize;
      if (wid == 0) {
            float v = (lane < nwarps_sum) ? s_sum[lane] : 0.0f;
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                  v += __shfl_down(v, offset);
            if (lane == 0) s_sum[0] = v;
      }
      __syncthreads();
      float inv_sum = (s_sum[0] > 0.0f) ? 1.0f / s_sum[0] : 0.0f;
      for (int j = threadIdx.x; j < cols; j += blockDim.x)
            row_ptr[j] *= inv_sum;
}

extern "C" void launch_causal_softmax_rows(
      float* x, int rows, int cols, hipStream_t stream
) {
      hipLaunchKernelGGL(attn_causal_softmax_kernel, dim3(rows), dim3(128), 0, stream, x, rows, cols);
}


__global__ void attn_mha_split_kernel(
      const float* x, float* out,
      int seq, int n_heads, int head_dim
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = seq * n_heads * head_dim;
      if (idx >= total) return;
      int s   = idx / (n_heads * head_dim);
      int rem = idx % (n_heads * head_dim);
      int h   = rem / head_dim;
      int d   = rem % head_dim;
      out[h * seq * head_dim + s * head_dim + d] = x[s * n_heads * head_dim + h * head_dim + d];
}

extern "C" void launch_mha_split(
      const float* x, float* out, int seq, int n_heads, int head_dim, hipStream_t stream
) {
      int total = seq * n_heads * head_dim;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_mha_split_kernel, grid, block, 0, stream, x, out, seq, n_heads, head_dim);
}


__global__ void attn_mha_merge_kernel(
      const float* x, float* out,
      int seq, int n_heads, int head_dim
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = seq * n_heads * head_dim;
      if (idx >= total) return;
      int h   = idx / (seq * head_dim);
      int rem = idx % (seq * head_dim);
      int s   = rem / head_dim;
      int d   = rem % head_dim;
      out[s * n_heads * head_dim + h * head_dim + d] = x[h * seq * head_dim + s * head_dim + d];
}

extern "C" void launch_mha_merge(
      const float* x, float* out, int seq, int n_heads, int head_dim, hipStream_t stream
) {
      int total = seq * n_heads * head_dim;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_mha_merge_kernel, grid, block, 0, stream, x, out, seq, n_heads, head_dim);
}


__global__ void attn_rope_kernel(
      const float* x, float* out,
      int seq, int dim, const float* base_ptr
) {
      float base = *base_ptr;
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int half_dim = dim / 2;
      int total = seq * half_dim;
      if (idx >= total) return;
      int s = idx / half_dim;
      int d = idx % half_dim;
      float theta = (float)s / powf(base, (float)(2 * d) / (float)dim);
      float cs = cosf(theta);
      float sn = sinf(theta);
      float x0 = x[s * dim + d];
      float x1 = x[s * dim + d + half_dim];
      out[s * dim + d]            = x0 * cs - x1 * sn;
      out[s * dim + d + half_dim] = x0 * sn + x1 * cs;
}

extern "C" void launch_rope(
      const float* x, float* out, int seq, int dim, const float* base, hipStream_t stream
) {
      int total = seq * (dim / 2);
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_rope_kernel, grid, block, 0, stream, x, out, seq, dim, base);
}


__global__ void attn_pos_enc_kernel(float* out, int seq, int dim) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= seq * dim) return;
      int s = idx / dim;
      int d = idx % dim;
      float denom = powf(10000.0f, (float)(d / 2 * 2) / (float)dim);
      float angle = (float)s / denom;
      out[idx] = (d % 2 == 0) ? sinf(angle) : cosf(angle);
}

extern "C" void launch_positional_encoding(
      float* out, int seq, int dim, hipStream_t stream
) {
      int total = seq * dim;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_pos_enc_kernel, grid, block, 0, stream, out, seq, dim);
}


__global__ void attn_rmsnorm_kernel(
      const float* x, const float* gamma, float* out,
      int rows, int cols, const float* eps_ptr
) {
      float eps = *eps_ptr;
      int row = blockIdx.x;
      if (row >= rows) return;
      const float* xr = x + row * cols;
      float* or_ = out + row * cols;

      float local_sq = 0.0f;
      for (int c = threadIdx.x; c < cols; c += blockDim.x)
            local_sq += xr[c] * xr[c];
      for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sq += __shfl_down(local_sq, offset);
      __shared__ float s_sq[32];
      int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
      if (lane == 0) s_sq[wid] = local_sq;
      __syncthreads();
      int nwarps_sq = (blockDim.x + warpSize - 1) / warpSize;
      if (wid == 0) {
            float v = (lane < nwarps_sq) ? s_sq[lane] : 0.0f;
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                  v += __shfl_down(v, offset);
            if (lane == 0) s_sq[0] = v;
      }
      __syncthreads();
      float rms = rsqrtf(s_sq[0] / (float)cols + eps);
      for (int c = threadIdx.x; c < cols; c += blockDim.x)
            or_[c] = xr[c] * rms * gamma[c];
}

extern "C" void launch_rmsnorm(
      const float* x, const float* gamma, float* out,
      int rows, int cols, const float* eps, hipStream_t stream
) {
      hipLaunchKernelGGL(attn_rmsnorm_kernel, dim3(rows), dim3(128), 0, stream,
            x, gamma, out, rows, cols, eps);
}


__global__ void attn_rmsnorm_backward_kernel(
      const float* grad_out, const float* x, const float* gamma,
      float* grad_x, float* grad_gamma,
      int rows, int cols, const float* eps_ptr
) {
      float eps = *eps_ptr;
      int row = blockIdx.x;
      if (row >= rows) return;
      const float* go = grad_out + row * cols;
      const float* xr = x + row * cols;
      float* gx = grad_x + row * cols;

      float local_sq = 0.0f;
      for (int c = threadIdx.x; c < cols; c += blockDim.x)
            local_sq += xr[c] * xr[c];
      for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_sq += __shfl_down(local_sq, offset);
      __shared__ float s_sq[32];
      int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
      if (lane == 0) s_sq[wid] = local_sq;
      __syncthreads();
      int nwarps_sq = (blockDim.x + warpSize - 1) / warpSize;
      if (wid == 0) {
            float v = (lane < nwarps_sq) ? s_sq[lane] : 0.0f;
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                  v += __shfl_down(v, offset);
            if (lane == 0) s_sq[0] = v;
      }
      __syncthreads();
      float rms_inv = rsqrtf(s_sq[0] / (float)cols + eps);

      float local_dot = 0.0f;
      for (int c = threadIdx.x; c < cols; c += blockDim.x)
            local_dot += go[c] * xr[c] * rms_inv;
      for (int offset = warpSize / 2; offset > 0; offset /= 2)
            local_dot += __shfl_down(local_dot, offset);
      __shared__ float s_dot[32];
      if (lane == 0) s_dot[wid] = local_dot;
      __syncthreads();
      int nwarps_dot = (blockDim.x + warpSize - 1) / warpSize;
      if (wid == 0) {
            float v = (lane < nwarps_dot) ? s_dot[lane] : 0.0f;
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                  v += __shfl_down(v, offset);
            if (lane == 0) s_dot[0] = v;
      }
      __syncthreads();
      float dot_val = s_dot[0] / (float)cols;

      for (int c = threadIdx.x; c < cols; c += blockDim.x) {
            float x_hat = xr[c] * rms_inv;
            gx[c] = gamma[c] * rms_inv * (go[c] - x_hat * dot_val);
            atomicAdd(&grad_gamma[c], go[c] * x_hat);
      }
}

extern "C" void launch_rmsnorm_backward(
      const float* grad_out, const float* x, const float* gamma,
      float* grad_x, float* grad_gamma,
      int rows, int cols, const float* eps, hipStream_t stream
) {
      hipLaunchKernelGGL(attn_rmsnorm_backward_kernel, dim3(rows), dim3(128), 0, stream,
            grad_out, x, gamma, grad_x, grad_gamma, rows, cols, eps);
}


__global__ void attn_im2col_2d_ext_kernel(
      const float* x, float* patches,
      int n, int c, int h, int w,
      int kh, int kw,
      int sh, int sw,
      int pad_h, int pad_w,
      int dil_h, int dil_w,
      int out_h, int out_w
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = n * out_h * out_w * c * kh * kw;
      if (idx >= total) return;

      int patch_size = c * kh * kw;
      int patch_idx = idx / patch_size;
      int within   = idx % patch_size;

      int ni  = patch_idx / (out_h * out_w);
      int hw  = patch_idx % (out_h * out_w);
      int oh  = hw / out_w;
      int ow  = hw % out_w;
      int ci  = within / (kh * kw);
      int kk  = within % (kh * kw);
      int khi = kk / kw;
      int kwi = kk % kw;

      int ih = oh * sh - pad_h + khi * dil_h;
      int iw = ow * sw - pad_w + kwi * dil_w;

      float val = 0.0f;
      if (ih >= 0 && ih < h && iw >= 0 && iw < w)
            val = x[((ni * c + ci) * h + ih) * w + iw];
      patches[idx] = val;
}

extern "C" void launch_im2col_2d_ext(
      const float* x, float* patches,
      int n, int c, int h, int w,
      int kh, int kw,
      int sh, int sw,
      int pad_h, int pad_w,
      int dil_h, int dil_w,
      int out_h, int out_w,
      hipStream_t stream
) {
      int total = n * out_h * out_w * c * kh * kw;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_im2col_2d_ext_kernel, grid, block, 0, stream,
            x, patches, n, c, h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w, out_h, out_w);
}


__global__ void attn_col2im_2d_ext_kernel(
      const float* patches, float* x,
      int n, int c, int h, int w,
      int kh, int kw,
      int sh, int sw,
      int pad_h, int pad_w,
      int dil_h, int dil_w,
      int out_h, int out_w
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = n * out_h * out_w * c * kh * kw;
      if (idx >= total) return;

      int patch_size = c * kh * kw;
      int patch_idx = idx / patch_size;
      int within   = idx % patch_size;

      int ni  = patch_idx / (out_h * out_w);
      int hw  = patch_idx % (out_h * out_w);
      int oh  = hw / out_w;
      int ow  = hw % out_w;
      int ci  = within / (kh * kw);
      int kk  = within % (kh * kw);
      int khi = kk / kw;
      int kwi = kk % kw;

      int ih = oh * sh - pad_h + khi * dil_h;
      int iw = ow * sw - pad_w + kwi * dil_w;

      if (ih >= 0 && ih < h && iw >= 0 && iw < w)
            atomicAdd(&x[((ni * c + ci) * h + ih) * w + iw], patches[idx]);
}

extern "C" void launch_col2im_2d_ext(
      const float* patches, float* x,
      int n, int c, int h, int w,
      int kh, int kw,
      int sh, int sw,
      int pad_h, int pad_w,
      int dil_h, int dil_w,
      int out_h, int out_w,
      hipStream_t stream
) {
      int total = n * out_h * out_w * c * kh * kw;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_col2im_2d_ext_kernel, grid, block, 0, stream,
            patches, x, n, c, h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w, out_h, out_w);
}


__global__ void attn_embedding_backward_kernel(
      const float* grad_out, const int* indices, float* grad_table,
      int n, int cols
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= n * cols) return;
      int row = idx / cols;
      int col = idx % cols;
      int vocab_row = indices[row];
      atomicAdd(&grad_table[vocab_row * cols + col], grad_out[idx]);
}

extern "C" void launch_embedding_backward(
      const float* grad_out, const int* indices, float* grad_table,
      int n, int cols, int vocab, hipStream_t stream
) {
      int total = n * cols;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_embedding_backward_kernel, grid, block, 0, stream,
            grad_out, indices, grad_table, n, cols);
      (void)vocab;
}


__global__ void attn_bn_update_running_kernel(
      float* run_mean, float* run_var,
      const float* save_mean, const float* save_var,
      const float* momentum_ptr, int c
) {
      float momentum = *momentum_ptr;
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= c) return;
      float one_minus_m = 1.0f - momentum;
      run_mean[idx] = one_minus_m * run_mean[idx] + momentum * save_mean[idx];
      run_var[idx]  = one_minus_m * run_var[idx]  + momentum * save_var[idx];
}

extern "C" void launch_bn_update_running(
      float* run_mean, float* run_var,
      const float* save_mean, const float* save_var,
      const float* momentum, int c, hipStream_t stream
) {
      dim3 block(256); dim3 grid((c + 255) / 256);
      hipLaunchKernelGGL(attn_bn_update_running_kernel, grid, block, 0, stream,
            run_mean, run_var, save_mean, save_var, momentum, c);
}


#define FA_BQ 64
#define FA_BK 64

template<typename T> __global__ void flash_attn_f64_kernel(
      const T* q, const T* k, const T* v, T* out,
      int n, int seq, int d, int heads
) {
      int hd = d / heads;
      int qtile = blockIdx.x;
      int hh = blockIdx.y;
      int bi = blockIdx.z;
      int tx = threadIdx.x;
      int qpos = qtile * FA_BQ + tx;

      T scale = 1.0 / sqrt((double)hd);

      HIP_DYNAMIC_SHARED(char, smem_raw);
      T* sh = reinterpret_cast<T*>(smem_raw);
      T* Qs = sh;
      T* Ks = Qs + FA_BQ * hd;
      T* Vs = Ks + FA_BK * hd;
      T* As = Vs + FA_BK * hd;

      T m = -1.0e308;
      T l = 0.0;

      if (qpos < seq) {
            const T* qrow = q + ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++) {
                  Qs[tx * hd + c] = qrow[c];
                  As[tx * hd + c] = 0.0;
            }
      }

      for (int kt = 0; kt < seq; kt += FA_BK) {
            for (int idx = tx; idx < FA_BK * hd; idx += FA_BQ) {
                  int kj = idx / hd;
                  int cc = idx % hd;
                  int kpos = kt + kj;
                  if (kpos < seq) {
                        long base = ((long)bi * seq + kpos) * d + hh * hd;
                        Ks[idx] = k[base + cc];
                        Vs[idx] = v[base + cc];
                  }
            }
            __syncthreads();

            if (qpos < seq) {
                  int tile = (seq - kt < FA_BK) ? (seq - kt) : FA_BK;
                  for (int kj = 0; kj < tile; kj++) {
                        T dot = 0.0;
                        for (int c = 0; c < hd; c++)
                              dot += Qs[tx * hd + c] * Ks[kj * hd + c];
                        T s = dot * scale;
                        T m_new = (s > m) ? s : m;
                        T resc = exp(m - m_new);
                        T e = exp(s - m_new);
                        for (int c = 0; c < hd; c++)
                              As[tx * hd + c] = As[tx * hd + c] * resc + Vs[kj * hd + c] * e;
                        l = l * resc + e;
                        m = m_new;
                  }
            }
            __syncthreads();
      }

      if (qpos < seq) {
            T inv = (l > 0.0) ? 1.0 / l : 0.0;
            T* orow = out + ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++)
                  orow[c] = As[tx * hd + c] * inv;
      }
}

extern "C" void launch_flash_attention_f64(
      const void* q, const void* k, const void* v, void* out,
      int n, int seq, int d, int heads, hipStream_t stream, int dtype
) {
      int hd = d / heads;
      int qtiles = (seq + FA_BQ - 1) / FA_BQ;
      dim3 grid(qtiles, heads, n);
      dim3 block(FA_BQ);
      size_t shared_bytes = (size_t)(2 * FA_BQ + 2 * FA_BK) * hd * dt_elem_size(dtype);
      DT_DISPATCH(dtype, hipLaunchKernelGGL(flash_attn_f64_kernel<T>, grid, block, shared_bytes, stream,
            (const T*)q, (const T*)k, (const T*)v, (T*)out, n, seq, d, heads));
}


__global__ void flash_attn_f64_train_fwd_kernel(
      const double* q, const double* k, const double* v, double* out, double* lse,
      int n, int seq, int d, int heads
) {
      int hd = d / heads;
      int qtile = blockIdx.x;
      int hh = blockIdx.y;
      int bi = blockIdx.z;
      int tx = threadIdx.x;
      int qpos = qtile * FA_BQ + tx;

      double scale = 1.0 / sqrt((double)hd);

      extern __shared__ double sh[];
      double* Qs = sh;
      double* Ks = Qs + FA_BQ * hd;
      double* Vs = Ks + FA_BK * hd;
      double* As = Vs + FA_BK * hd;

      double m = -1.0e308;
      double l = 0.0;

      if (qpos < seq) {
            const double* qrow = q + ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++) {
                  Qs[tx * hd + c] = qrow[c];
                  As[tx * hd + c] = 0.0;
            }
      }

      for (int kt = 0; kt < seq; kt += FA_BK) {
            for (int idx = tx; idx < FA_BK * hd; idx += FA_BQ) {
                  int kj = idx / hd;
                  int cc = idx % hd;
                  int kpos = kt + kj;
                  if (kpos < seq) {
                        long base = ((long)bi * seq + kpos) * d + hh * hd;
                        Ks[idx] = k[base + cc];
                        Vs[idx] = v[base + cc];
                  }
            }
            __syncthreads();

            if (qpos < seq) {
                  int tile = (seq - kt < FA_BK) ? (seq - kt) : FA_BK;
                  for (int kj = 0; kj < tile; kj++) {
                        double dot = 0.0;
                        for (int c = 0; c < hd; c++)
                              dot += Qs[tx * hd + c] * Ks[kj * hd + c];
                        double s = dot * scale;
                        double m_new = (s > m) ? s : m;
                        double resc = exp(m - m_new);
                        double e = exp(s - m_new);
                        for (int c = 0; c < hd; c++)
                              As[tx * hd + c] = As[tx * hd + c] * resc + Vs[kj * hd + c] * e;
                        l = l * resc + e;
                        m = m_new;
                  }
            }
            __syncthreads();
      }

      if (qpos < seq) {
            double inv = (l > 0.0) ? 1.0 / l : 0.0;
            double* orow = out + ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++)
                  orow[c] = As[tx * hd + c] * inv;
            lse[((long)bi * heads + hh) * seq + qpos] = (l > 0.0) ? m + log(l) : -1.0e308;
      }
}

extern "C" void launch_flash_attention_f64_train_fwd(
      const double* q, const double* k, const double* v, double* out, double* lse,
      int n, int seq, int d, int heads, hipStream_t stream
) {
      int hd = d / heads;
      int qtiles = (seq + FA_BQ - 1) / FA_BQ;
      dim3 grid(qtiles, heads, n);
      dim3 block(FA_BQ);
      size_t shared_bytes = (size_t)(2 * FA_BQ + 2 * FA_BK) * hd * sizeof(double);
      hipLaunchKernelGGL(flash_attn_f64_train_fwd_kernel, grid, block, shared_bytes, stream,
            q, k, v, out, lse, n, seq, d, heads);
}

__global__ void flash_attn_f64_dsum_kernel(
      const double* ctx, const double* dctx, double* dsum,
      int n, int seq, int d, int heads
) {
      int qpos = blockIdx.x * blockDim.x + threadIdx.x;
      int hh = blockIdx.y;
      int bi = blockIdx.z;
      if (qpos >= seq) return;
      int hd = d / heads;
      long base = ((long)bi * seq + qpos) * d + hh * hd;
      double acc = 0.0;
      for (int c = 0; c < hd; c++)
            acc += dctx[base + c] * ctx[base + c];
      dsum[((long)bi * heads + hh) * seq + qpos] = acc;
}

extern "C" void launch_flash_attention_f64_dsum(
      const double* ctx, const double* dctx, double* dsum,
      int n, int seq, int d, int heads, hipStream_t stream
) {
      dim3 grid((seq + 255) / 256, heads, n);
      dim3 block(256);
      hipLaunchKernelGGL(flash_attn_f64_dsum_kernel, grid, block, 0, stream,
            ctx, dctx, dsum, n, seq, d, heads);
}

__global__ void flash_attn_f64_bwd_dq_kernel(
      const double* q, const double* k, const double* v, const double* dctx,
      const double* lse, const double* dsum, double* dq,
      int n, int seq, int d, int heads
) {
      int hd = d / heads;
      int qtile = blockIdx.x;
      int hh = blockIdx.y;
      int bi = blockIdx.z;
      int tx = threadIdx.x;
      int qpos = qtile * FA_BQ + tx;
      double scale = 1.0 / sqrt((double)hd);

      extern __shared__ double sh[];
      double* Qs = sh;
      double* Ds = Qs + FA_BQ * hd;
      double* Ks = Ds + FA_BQ * hd;
      double* Vs = Ks + FA_BK * hd;
      double* Aq = Vs + FA_BK * hd;

      double L = 0.0, Drow = 0.0;
      if (qpos < seq) {
            long qb = ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++) {
                  Qs[tx * hd + c] = q[qb + c];
                  Ds[tx * hd + c] = dctx[qb + c];
                  Aq[tx * hd + c] = 0.0;
            }
            long sb = ((long)bi * heads + hh) * seq + qpos;
            L = lse[sb];
            Drow = dsum[sb];
      }

      for (int kt = 0; kt < seq; kt += FA_BK) {
            for (int idx = tx; idx < FA_BK * hd; idx += FA_BQ) {
                  int kj = idx / hd;
                  int cc = idx % hd;
                  int kpos = kt + kj;
                  if (kpos < seq) {
                        long b = ((long)bi * seq + kpos) * d + hh * hd;
                        Ks[idx] = k[b + cc];
                        Vs[idx] = v[b + cc];
                  }
            }
            __syncthreads();

            if (qpos < seq) {
                  int tile = (seq - kt < FA_BK) ? (seq - kt) : FA_BK;
                  for (int kj = 0; kj < tile; kj++) {
                        double dot = 0.0, dp = 0.0;
                        for (int c = 0; c < hd; c++) {
                              dot += Qs[tx * hd + c] * Ks[kj * hd + c];
                              dp += Ds[tx * hd + c] * Vs[kj * hd + c];
                        }
                        double p = exp(dot * scale - L);
                        double dsv = p * (dp - Drow) * scale;
                        for (int c = 0; c < hd; c++)
                              Aq[tx * hd + c] += dsv * Ks[kj * hd + c];
                  }
            }
            __syncthreads();
      }

      if (qpos < seq) {
            long qb = ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++)
                  dq[qb + c] = Aq[tx * hd + c];
      }
}

extern "C" void launch_flash_attention_f64_bwd_dq(
      const double* q, const double* k, const double* v, const double* dctx,
      const double* lse, const double* dsum, double* dq,
      int n, int seq, int d, int heads, hipStream_t stream
) {
      int hd = d / heads;
      int qtiles = (seq + FA_BQ - 1) / FA_BQ;
      dim3 grid(qtiles, heads, n);
      dim3 block(FA_BQ);
      size_t shared_bytes = (size_t)(3 * FA_BQ + 2 * FA_BK) * hd * sizeof(double);
      hipLaunchKernelGGL(flash_attn_f64_bwd_dq_kernel, grid, block, shared_bytes, stream,
            q, k, v, dctx, lse, dsum, dq, n, seq, d, heads);
}

#define FA_TQ 32

__global__ void flash_attn_f64_bwd_dkv_kernel(
      const double* q, const double* k, const double* v, const double* dctx,
      const double* lse, const double* dsum, double* dk, double* dv,
      int n, int seq, int d, int heads
) {
      int hd = d / heads;
      int ktile = blockIdx.x;
      int hh = blockIdx.y;
      int bi = blockIdx.z;
      int tx = threadIdx.x;
      int kpos = ktile * FA_BK + tx;
      double scale = 1.0 / sqrt((double)hd);

      extern __shared__ double sh[];
      double* Ks = sh;
      double* Vo = Ks + FA_BK * hd;
      double* Ak = Vo + FA_BK * hd;
      double* Av = Ak + FA_BK * hd;
      double* Qs = Av + FA_BK * hd;
      double* Ds = Qs + FA_TQ * hd;
      double* Ls = Ds + FA_TQ * hd;
      double* Rs = Ls + FA_TQ;

      if (kpos < seq) {
            long kb = ((long)bi * seq + kpos) * d + hh * hd;
            for (int c = 0; c < hd; c++) {
                  Ks[tx * hd + c] = k[kb + c];
                  Vo[tx * hd + c] = v[kb + c];
                  Ak[tx * hd + c] = 0.0;
                  Av[tx * hd + c] = 0.0;
            }
      }

      for (int qt = 0; qt < seq; qt += FA_TQ) {
            for (int idx = tx; idx < FA_TQ * hd; idx += FA_BK) {
                  int qi = idx / hd;
                  int cc = idx % hd;
                  int qpos = qt + qi;
                  if (qpos < seq) {
                        long b = ((long)bi * seq + qpos) * d + hh * hd;
                        Qs[idx] = q[b + cc];
                        Ds[idx] = dctx[b + cc];
                  }
            }
            for (int idx = tx; idx < FA_TQ; idx += FA_BK) {
                  int qpos = qt + idx;
                  if (qpos < seq) {
                        long sb = ((long)bi * heads + hh) * seq + qpos;
                        Ls[idx] = lse[sb];
                        Rs[idx] = dsum[sb];
                  }
            }
            __syncthreads();

            if (kpos < seq) {
                  int tile = (seq - qt < FA_TQ) ? (seq - qt) : FA_TQ;
                  for (int qi = 0; qi < tile; qi++) {
                        double dot = 0.0, dp = 0.0;
                        for (int c = 0; c < hd; c++) {
                              dot += Qs[qi * hd + c] * Ks[tx * hd + c];
                              dp += Ds[qi * hd + c] * Vo[tx * hd + c];
                        }
                        double p = exp(dot * scale - Ls[qi]);
                        double dsv = p * (dp - Rs[qi]) * scale;
                        for (int c = 0; c < hd; c++) {
                              Ak[tx * hd + c] += dsv * Qs[qi * hd + c];
                              Av[tx * hd + c] += p * Ds[qi * hd + c];
                        }
                  }
            }
            __syncthreads();
      }

      if (kpos < seq) {
            long kb = ((long)bi * seq + kpos) * d + hh * hd;
            for (int c = 0; c < hd; c++) {
                  dk[kb + c] = Ak[tx * hd + c];
                  dv[kb + c] = Av[tx * hd + c];
            }
      }
}

extern "C" void launch_flash_attention_f64_bwd_dkv(
      const double* q, const double* k, const double* v, const double* dctx,
      const double* lse, const double* dsum, double* dk, double* dv,
      int n, int seq, int d, int heads, hipStream_t stream
) {
      int hd = d / heads;
      int ktiles = (seq + FA_BK - 1) / FA_BK;
      dim3 grid(ktiles, heads, n);
      dim3 block(FA_BK);
      size_t shared_bytes = ((size_t)(4 * FA_BK + 2 * FA_TQ) * hd + 2 * FA_TQ) * sizeof(double);
      hipLaunchKernelGGL(flash_attn_f64_bwd_dkv_kernel, grid, block, shared_bytes, stream,
            q, k, v, dctx, lse, dsum, dk, dv, n, seq, d, heads);
}
