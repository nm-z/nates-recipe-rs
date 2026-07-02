#include <hip/hip_runtime.h>

// ── Fused scaled dot-product attention (f32, flash-style) ──────────────────
// One block per query row. Online softmax: running max m, running sum l,
// running output acc[dim]. On each new key j, rescale existing acc by
// exp(m_old - m_new) before accumulating. Scale = 1/sqrt(dim).
// causal=1: skip keys j > query_row (block index).
// Layout: Q,K,V are (n_rows, seq, dim); out is (n_rows, seq, dim).

__global__ void attn_sdp_kernel(
      const float* q, const float* k, const float* v,
      float* out,
      int seq, int dim, int causal
) {
      int batch_row = blockIdx.x;   // index in [0, n_rows*seq)
      int br = batch_row / seq;     // batch index
      int qi = batch_row % seq;     // query position in sequence

      const float* q_row = q + batch_row * dim;
      const float* k_base = k + br * seq * dim;
      const float* v_base = v + br * seq * dim;
      float* o_row = out + batch_row * dim;

      float scale = 1.0f / sqrtf((float)dim);
      int key_limit = causal ? (qi + 1) : seq;

      // shared mem: acc[dim] — one f32 per dim, cooperated over threads
      extern __shared__ float sh_acc[];
      // initialize acc to 0
      for (int d = threadIdx.x; d < dim; d += blockDim.x)
            sh_acc[d] = 0.0f;
      __syncthreads();

      float m = -1e30f;   // running max
      float l = 0.0f;     // running sum of exp(s - m)

      for (int j = 0; j < key_limit; j++) {
            const float* k_row = k_base + j * dim;
            // dot product q · k_j (thread-strided, then reduce)
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

            // online softmax update
            float m_new = (s > m) ? s : m;
            float rescale = expf(m - m_new);
            float e_s = expf(s - m_new);

            // rescale acc, add v_j * e_s
            const float* v_row = v_base + j * dim;
            for (int d = threadIdx.x; d < dim; d += blockDim.x)
                  sh_acc[d] = sh_acc[d] * rescale + v_row[d] * e_s;
            l = l * rescale + e_s;
            m = m_new;
            __syncthreads();
      }

      // normalize and write output
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

// ── Causal softmax (upper-triangular -inf mask, f32) ──────────────────────
// Row i: positions j > i are masked to -inf before softmax.

__global__ void attn_causal_softmax_kernel(float* x, int rows, int cols) {
      int row = blockIdx.x;
      if (row >= rows) return;
      float* row_ptr = x + row * cols;

      // Find max (only unmasked: j <= row)
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

      // Exp (masked positions get 0)
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

// ── MHA split: (seq, n_heads*head_dim) → (n_heads, seq, head_dim) ─────────
// Permute dim 2 from interleaved heads to head-first layout.

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
      // src: x[s, h*head_dim + d] = x[s * (n_heads*head_dim) + h*head_dim + d]
      // dst: out[h, s, d] = out[h*seq*head_dim + s*head_dim + d]
      out[h * seq * head_dim + s * head_dim + d] = x[s * n_heads * head_dim + h * head_dim + d];
}

extern "C" void launch_mha_split(
      const float* x, float* out, int seq, int n_heads, int head_dim, hipStream_t stream
) {
      int total = seq * n_heads * head_dim;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_mha_split_kernel, grid, block, 0, stream, x, out, seq, n_heads, head_dim);
}

// ── MHA merge: (n_heads, seq, head_dim) → (seq, n_heads*head_dim) ─────────

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
      // src: x[h, s, d]
      // dst: out[s, h*head_dim + d]
      out[s * n_heads * head_dim + h * head_dim + d] = x[h * seq * head_dim + s * head_dim + d];
}

extern "C" void launch_mha_merge(
      const float* x, float* out, int seq, int n_heads, int head_dim, hipStream_t stream
) {
      int total = seq * n_heads * head_dim;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_mha_merge_kernel, grid, block, 0, stream, x, out, seq, n_heads, head_dim);
}

// ── RoPE (rotary positional embedding, f32) ────────────────────────────────
// For each (s, d) pair, rotate pairs (d, d+dim/2) by angle theta = pos / base^(2d/dim).
// Self-contained: uses device sinf/cosf — no external math dependency.

__global__ void attn_rope_kernel(
      const float* x, float* out,
      int seq, int dim, float base
) {
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
      const float* x, float* out, int seq, int dim, float base, hipStream_t stream
) {
      int total = seq * (dim / 2);
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(attn_rope_kernel, grid, block, 0, stream, x, out, seq, dim, base);
}

// ── Sinusoidal positional encoding table (f32) ────────────────────────────
// out[s, d] = sin(s / 10000^(d/dim))  for d even
//           = cos(s / 10000^((d-1)/dim)) for d odd

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

// ── RMSNorm forward (f32) ─────────────────────────────────────────────────
// rms = sqrt(mean(x^2) + eps); out = (x / rms) * gamma
// One block per row; threads cooperate over cols.

__global__ void attn_rmsnorm_kernel(
      const float* x, const float* gamma, float* out,
      int rows, int cols, float eps
) {
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
      int rows, int cols, float eps, hipStream_t stream
) {
      hipLaunchKernelGGL(attn_rmsnorm_kernel, dim3(rows), dim3(128), 0, stream,
            x, gamma, out, rows, cols, eps);
}

// ── RMSNorm backward (f32) ────────────────────────────────────────────────
// grad_x[i] = gamma[c] * rms * (grad_out[i] - x_hat[i] * mean(grad_out * x_hat))
// grad_gamma[c] = sum_rows(grad_out * x_hat)   where x_hat = x / rms

__global__ void attn_rmsnorm_backward_kernel(
      const float* grad_out, const float* x, const float* gamma,
      float* grad_x, float* grad_gamma,
      int rows, int cols, float eps
) {
      int row = blockIdx.x;
      if (row >= rows) return;
      const float* go = grad_out + row * cols;
      const float* xr = x + row * cols;
      float* gx = grad_x + row * cols;

      // Recompute rms
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

      // dot(grad_out, x_hat) = sum(go * xr * rms_inv)
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
            // grad_gamma: accumulated with atomicAdd across rows
            atomicAdd(&grad_gamma[c], go[c] * x_hat);
      }
}

extern "C" void launch_rmsnorm_backward(
      const float* grad_out, const float* x, const float* gamma,
      float* grad_x, float* grad_gamma,
      int rows, int cols, float eps, hipStream_t stream
) {
      hipLaunchKernelGGL(attn_rmsnorm_backward_kernel, dim3(rows), dim3(128), 0, stream,
            grad_out, x, gamma, grad_x, grad_gamma, rows, cols, eps);
}

// ── im2col_2d_ext (f32, strided+padded+dilated) ────────────────────────────
// out_h = (h + 2*pad_h - dil_h*(kh-1) - 1) / sh + 1
// out_w = (w + 2*pad_w - dil_w*(kw-1) - 1) / sw + 1
// Output layout: (n * out_h * out_w, c * kh * kw) — same as im2col_2d but
// with stride, padding, dilation. Out-of-bounds sample → 0 (zero-padding).

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

// ── col2im_2d_ext (f32, inverse of im2col_2d_ext) ─────────────────────────
// Accumulates patch values back into image using atomicAdd.
// out_h = (h + 2*pad_h - dil_h*(kh-1) - 1) / sh + 1  (same formula as im2col)

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

// ── Embedding backward (f32, atomicAdd accumulate) ────────────────────────
// grad_out: (n, cols) — upstream gradient from embedding lookup
// indices:  (n,) int32 token ids in [0, vocab)
// grad_table: (vocab, cols) — accumulate gradient; caller must zero first
// Atomic path: each thread adds grad_out[i, d] into grad_table[indices[i], d].
// No dedup — concurrent writes from same token idx race on the same cell,
// atomicAdd guarantees correct accumulation (last-writer issue is absent for add).

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
      (void)vocab; // vocab used only for sizing by caller
}

// ── BN running stats update (f32) ─────────────────────────────────────────
// run_mean[c] = (1 - momentum) * run_mean[c] + momentum * save_mean[c]
// run_var[c]  = (1 - momentum) * run_var[c]  + momentum * save_var[c]
// In-place on run_mean / run_var.

__global__ void attn_bn_update_running_kernel(
      float* run_mean, float* run_var,
      const float* save_mean, const float* save_var,
      float momentum, int c
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= c) return;
      float one_minus_m = 1.0f - momentum;
      run_mean[idx] = one_minus_m * run_mean[idx] + momentum * save_mean[idx];
      run_var[idx]  = one_minus_m * run_var[idx]  + momentum * save_var[idx];
}

extern "C" void launch_bn_update_running(
      float* run_mean, float* run_var,
      const float* save_mean, const float* save_var,
      float momentum, int c, hipStream_t stream
) {
      dim3 block(256); dim3 grid((c + 255) / 256);
      hipLaunchKernelGGL(attn_bn_update_running_kernel, grid, block, 0, stream,
            run_mean, run_var, save_mean, save_var, momentum, c);
}

// ── Fused multi-head self-attention, f64, FlashAttention-style ─────────────
// Inference KV-cache attention: K,V live in GPU memory (the cache, O(L·d)) and
// are streamed through shared-memory tiles with an online softmax, so the L×L
// score matrix is NEVER materialized — not in global memory, not in the kernel.
// Bidirectional (every query attends every key), matching the training forward.
//
// Layout (matches model.rs a_q/a_k/a_v/a_ctx): q,k,v,out are [n, seq, d] row-major
// with d = heads*hd; head hh occupies columns [hh*hd, hh*hd+hd). scale = 1/sqrt(hd).
// Grid: (ceil(seq/FA_BQ), heads, n). Block: FA_BQ threads, thread t = one query row.
// Shared: Qs[FA_BQ*hd] | Ks[FA_BK*hd] | Vs[FA_BK*hd] | As[FA_BQ*hd] (accumulators).

#define FA_BQ 64
#define FA_BK 64

__global__ void flash_attn_f64_kernel(
      const double* q, const double* k, const double* v, double* out,
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

      double m = -1.0e308;   // running max
      double l = 0.0;        // running denom

      // Load this thread's query row + zero its accumulator.
      if (qpos < seq) {
            const double* qrow = q + ((long)bi * seq + qpos) * d + hh * hd;
            for (int c = 0; c < hd; c++) {
                  Qs[tx * hd + c] = qrow[c];
                  As[tx * hd + c] = 0.0;
            }
      }

      for (int kt = 0; kt < seq; kt += FA_BK) {
            // Cooperatively stage this K,V tile into shared memory.
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
      }
}

extern "C" void launch_flash_attention_f64(
      const double* q, const double* k, const double* v, double* out,
      int n, int seq, int d, int heads, hipStream_t stream
) {
      int hd = d / heads;
      int qtiles = (seq + FA_BQ - 1) / FA_BQ;
      dim3 grid(qtiles, heads, n);
      dim3 block(FA_BQ);
      size_t shared_bytes = (size_t)(2 * FA_BQ + 2 * FA_BK) * hd * sizeof(double);
      hipLaunchKernelGGL(flash_attn_f64_kernel, grid, block, shared_bytes, stream,
            q, k, v, out, n, seq, d, heads);
}

// ── FlashAttention f64 TRAINING ─────────────────────────────────────────────
// Forward is the same online-softmax stream as flash_attn_f64_kernel but also
// emits the per-row logsumexp L_i = m_i + ln(l_i)  (layout [n][heads][seq]).
// Backward never materializes the L×L score matrix: both passes recompute
// P_ij = exp(q_i·k_j·scale − L_i) tile by tile from Q,K + lse. dsum holds
// D_i = Σ_c dO_ic·O_ic (equals rowsum(dP∘P), the softmax-backward row term).
// Deterministic: each output row is owned by exactly one thread, fixed loop
// order, no atomics.

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

// D_i = Σ_c dctx_ic · ctx_ic per (sample, head, row) — the softmax-backward row
// term, computed once so both backward kernels read it instead of re-reducing.
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

// dQ_i = Σ_j P_ij (dO_i·V_j − D_i) · scale · K_j — query-major: one thread per
// query row, K/V tiles streamed through shared memory like the forward.
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
      double* Qs = sh;                    // FA_BQ*hd  own query rows
      double* Ds = Qs + FA_BQ * hd;       // FA_BQ*hd  own dO rows
      double* Ks = Ds + FA_BQ * hd;       // FA_BK*hd
      double* Vs = Ks + FA_BK * hd;       // FA_BK*hd
      double* Aq = Vs + FA_BK * hd;       // FA_BQ*hd  dq accumulators

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

// dK_j = Σ_i dS_ij Q_i, dV_j = Σ_i P_ij dO_i — key-major: one thread per key
// row, Q/dO tiles (FA_TQ rows) streamed with their lse/dsum scalars.
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
      double* Ks = sh;                    // FA_BK*hd  own key rows
      double* Vo = Ks + FA_BK * hd;       // FA_BK*hd  own value rows
      double* Ak = Vo + FA_BK * hd;       // FA_BK*hd  dk accumulators
      double* Av = Ak + FA_BK * hd;       // FA_BK*hd  dv accumulators
      double* Qs = Av + FA_BK * hd;       // FA_TQ*hd
      double* Ds = Qs + FA_TQ * hd;       // FA_TQ*hd  dO rows
      double* Ls = Ds + FA_TQ * hd;       // FA_TQ     lse
      double* Rs = Ls + FA_TQ;            // FA_TQ     dsum

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
