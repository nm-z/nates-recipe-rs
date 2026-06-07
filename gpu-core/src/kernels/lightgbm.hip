#include <hip/hip_runtime.h>
#include <float.h>

// ══════════════════════════════════════════════════════════════════════════════
// LightGBM leaf-wise kernels — i32 leaf-slot index, f32 grad/hess/count histograms.
// EXCLUSIVE to lightgbm-rs. The shared oblivious_* / leaf_reduce kernels in tree.hip
// keep their u8 leaf index for catboost-rs + nates-gpu-ruby and are NOT touched here.
//
// Histogram layout (size_t throughout — slot*n_eff*n_bins can exceed 2^31):
//   base = ((size_t)slot * n_eff + feat) * n_bins + bin
// ══════════════════════════════════════════════════════════════════════════════

// ── lgbm_histogram ─────────────────────────────────────────────────────────
// Build grad/hess/count histograms for ONE leaf slot. One block per feature,
// shared-mem [256] accumulators. Scans all rows, accumulates only target_slot's.
extern "C" __global__
void lgbm_histogram_kernel(
      const unsigned char* bins_fm,   // [n_eff * n_rows] feature-major u8
      const int*           node_idx,  // [n_rows] i32 leaf slot per row
      const float*         grad,      // [n_rows]
      const float*         hess,      // [n_rows]
      float*               grad_hist,
      float*               hess_hist,
      float*               count_hist,
      int                  target_slot,
      int                  n_rows,
      int                  n_eff,
      int                  n_bins
) {
      int feat = blockIdx.x;
      int tid  = threadIdx.x;

      __shared__ float s_g[256];
      __shared__ float s_h[256];
      __shared__ float s_c[256];

      for (int b = tid; b < n_bins; b += blockDim.x) {
            s_g[b] = 0.0f; s_h[b] = 0.0f; s_c[b] = 0.0f;
      }
      __syncthreads();

      const unsigned char* feat_bins = bins_fm + (size_t)feat * n_rows;
      for (int row = tid; row < n_rows; row += blockDim.x) {
            if (node_idx[row] == target_slot) {
                  unsigned char bin = feat_bins[row];
                  if (bin < n_bins) {
                        atomicAdd(&s_g[bin], grad[row]);
                        atomicAdd(&s_h[bin], hess[row]);
                        atomicAdd(&s_c[bin], 1.0f);
                  }
            }
      }
      __syncthreads();

      size_t base = ((size_t)target_slot * n_eff + feat) * n_bins;
      for (int b = tid; b < n_bins; b += blockDim.x) {
            grad_hist[base + b]  = s_g[b];
            hess_hist[base + b]  = s_h[b];
            count_hist[base + b] = s_c[b];
      }
}

extern "C" void launch_lgbm_histogram(
      const unsigned char* bins_fm, const int* node_idx,
      const float* grad, const float* hess,
      float* grad_hist, float* hess_hist, float* count_hist,
      int target_slot, int n_rows, int n_eff, int n_bins,
      hipStream_t stream
) {
      dim3 grid(n_eff);
      dim3 block(256);
      hipLaunchKernelGGL(lgbm_histogram_kernel, grid, block, 0, stream,
            bins_fm, node_idx, grad, hess, grad_hist, hess_hist, count_hist,
            target_slot, n_rows, n_eff, n_bins);
}

// ── lgbm_hist_subtract ─────────────────────────────────────────────────────
// In-place histogram subtraction: hist[dst_slot] -= hist[src_slot] for all
// three (grad/hess/count). One block per feature. Used to derive the larger
// child (parent − smaller sibling); parent stays in dst_slot until this runs.
extern "C" __global__
void lgbm_hist_subtract_kernel(
      float* grad_hist,
      float* hess_hist,
      float* count_hist,
      int    dst_slot,
      int    src_slot,
      int    n_eff,
      int    n_bins
) {
      int feat = blockIdx.x;
      size_t bd = ((size_t)dst_slot * n_eff + feat) * n_bins;
      size_t bs = ((size_t)src_slot * n_eff + feat) * n_bins;
      for (int b = threadIdx.x; b < n_bins; b += blockDim.x) {
            grad_hist[bd + b]  -= grad_hist[bs + b];
            hess_hist[bd + b]  -= hess_hist[bs + b];
            count_hist[bd + b] -= count_hist[bs + b];
      }
}

extern "C" void launch_lgbm_hist_subtract(
      float* grad_hist, float* hess_hist, float* count_hist,
      int dst_slot, int src_slot, int n_eff, int n_bins,
      hipStream_t stream
) {
      dim3 grid(n_eff);
      dim3 block(256);
      hipLaunchKernelGGL(lgbm_hist_subtract_kernel, grid, block, 0, stream,
            grad_hist, hess_hist, count_hist, dst_slot, src_slot, n_eff, n_bins);
}

// ── lgbm_best_split ────────────────────────────────────────────────────────
// One block per evaluated slot. For each feature: Hillis-Steele inclusive prefix
// scan over bins (linear, NOT O(n_bins^2)), Newton gain gated by min_child_weight,
// per-thread best across features. Then a shared-array + thread-0 reduction (the
// codebase has no float atomicMax) picks the winning (feat,bin) and its left count.
// Outputs are packed by EVAL INDEX (slot_idx), not by slot id.
extern "C" __global__
void lgbm_best_split_kernel(
      const float* grad_hist,
      const float* hess_hist,
      const float* count_hist,
      const int*   slot_ids,        // [n_eval]
      float*       best_gain,       // [n_eval]
      int*         best_feat,       // [n_eval]
      int*         best_bin,        // [n_eval]
      int*         best_left_count, // [n_eval]
      int          n_eval,
      int          n_eff,
      int          n_bins,
      float        lambda,
      float        min_child_weight
) {
      int slot_idx = blockIdx.x;
      if (slot_idx >= n_eval) return;
      int slot = slot_ids[slot_idx];
      int tid  = threadIdx.x;

      __shared__ float s_g[256];
      __shared__ float s_h[256];
      __shared__ float s_c[256];

      float my_gain = -FLT_MAX;
      int   my_feat = -1, my_bin = -1;
      float my_lc   = 0.0f;

      for (int feat = 0; feat < n_eff; feat++) {
            size_t base = ((size_t)slot * n_eff + feat) * n_bins;
            s_g[tid] = (tid < n_bins) ? grad_hist[base + tid]  : 0.0f;
            s_h[tid] = (tid < n_bins) ? hess_hist[base + tid]  : 0.0f;
            s_c[tid] = (tid < n_bins) ? count_hist[base + tid] : 0.0f;
            __syncthreads();

            for (int stride = 1; stride < n_bins; stride <<= 1) {
                  float pg = (tid >= stride) ? s_g[tid - stride] : 0.0f;
                  float ph = (tid >= stride) ? s_h[tid - stride] : 0.0f;
                  float pc = (tid >= stride) ? s_c[tid - stride] : 0.0f;
                  __syncthreads();
                  s_g[tid] += pg; s_h[tid] += ph; s_c[tid] += pc;
                  __syncthreads();
            }

            float tg = s_g[n_bins - 1];
            float th = s_h[n_bins - 1];
            if (th < 1e-10f) { __syncthreads(); continue; }
            float parent = tg * tg / (th + lambda);

            if (tid < n_bins - 1) {
                  float lg = s_g[tid], lh = s_h[tid], lc = s_c[tid];
                  float rg = tg - lg, rh = th - lh;
                  if (lh >= min_child_weight && rh >= min_child_weight) {
                        float gain = lg * lg / (lh + lambda) + rg * rg / (rh + lambda) - parent;
                        if (isfinite(gain) && gain > my_gain) {
                              my_gain = gain; my_feat = feat; my_bin = tid; my_lc = lc;
                        }
                  }
            }
            __syncthreads();
      }

      __shared__ float r_gain[256];
      __shared__ int   r_feat[256];
      __shared__ int   r_bin[256];
      __shared__ float r_lc[256];
      r_gain[tid] = my_gain;
      r_feat[tid] = my_feat;
      r_bin[tid]  = my_bin;
      r_lc[tid]   = my_lc;
      __syncthreads();

      if (tid == 0) {
            float bg = -FLT_MAX; int bf = -1, bb = -1; float bc = 0.0f;
            for (int t = 0; t < blockDim.x; t++) {
                  if (r_gain[t] > bg) { bg = r_gain[t]; bf = r_feat[t]; bb = r_bin[t]; bc = r_lc[t]; }
            }
            best_gain[slot_idx]       = bg;
            best_feat[slot_idx]       = bf;
            best_bin[slot_idx]        = bb;
            best_left_count[slot_idx] = (int)(bc + 0.5f);
      }
}

extern "C" void launch_lgbm_best_split(
      const float* grad_hist, const float* hess_hist, const float* count_hist,
      const int* slot_ids,
      float* best_gain, int* best_feat, int* best_bin, int* best_left_count,
      int n_eval, int n_eff, int n_bins, float lambda, float min_child_weight,
      hipStream_t stream
) {
      dim3 grid(n_eval);
      dim3 block(256);
      hipLaunchKernelGGL(lgbm_best_split_kernel, grid, block, 0, stream,
            grad_hist, hess_hist, count_hist, slot_ids,
            best_gain, best_feat, best_bin, best_left_count,
            n_eval, n_eff, n_bins, lambda, min_child_weight);
}

// ── lgbm_leaf_reduce ───────────────────────────────────────────────────────
// Scatter grad/hess into per-slot buckets via atomics (i32 leaf slot). Pairs with
// the shared tree.hip leaf_finalize kernel (no node_idx) for value = -G/(H+λ).
extern "C" __global__
void lgbm_leaf_reduce_kernel(
      const int*   node_idx,
      const float* grad,
      const float* hess,
      float*       leaf_grad,
      float*       leaf_hess,
      int          n_rows
) {
      int row = blockIdx.x * blockDim.x + threadIdx.x;
      if (row >= n_rows) return;
      int slot = node_idx[row];
      atomicAdd(&leaf_grad[slot], grad[row]);
      atomicAdd(&leaf_hess[slot], hess[row]);
}

extern "C" void launch_lgbm_leaf_reduce(
      const int* node_idx, const float* grad, const float* hess,
      float* leaf_grad, float* leaf_hess, int n_rows, hipStream_t stream
) {
      dim3 block(256);
      dim3 grid((n_rows + 255) / 256);
      hipLaunchKernelGGL(lgbm_leaf_reduce_kernel, grid, block, 0, stream,
            node_idx, grad, hess, leaf_grad, leaf_hess, n_rows);
}

// ── goss_sample ────────────────────────────────────────────────────────────
// sorted_idx[0..top_k-1] = top-a rows by |grad| (argsorted on host).
// weights_out[row] = 1.0 (top-a), (1-a)/b if uniform_rand[row] < (1-a)/b (rest), else 0.
extern "C" __global__
void goss_sample_kernel(
      const int*   sorted_idx,
      float*       weights_out,
      const float* uniform_rand,
      int          n_rows,
      int          top_k,
      float        sample_rate,    // keep a rest row with prob = sample_rate (= goss_b)
      float        keep_weight     // amplification weight = (1-a)/b applied to kept rest rows
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n_rows) return;
      int row = sorted_idx[i];
      if (i < top_k) {
            weights_out[row] = 1.0f;
      } else if (uniform_rand[row] < sample_rate) {
            weights_out[row] = keep_weight;
      }
}

extern "C" void launch_goss_sample(
      const int* sorted_idx, float* weights_out, const float* uniform_rand,
      int n_rows, int top_k, float sample_rate, float keep_weight, hipStream_t stream
) {
      dim3 block(256);
      dim3 grid((n_rows + 255) / 256);
      hipLaunchKernelGGL(goss_sample_kernel, grid, block, 0, stream,
            sorted_idx, weights_out, uniform_rand, n_rows, top_k, sample_rate, keep_weight);
}

// ── leaf_split_apply ───────────────────────────────────────────────────────
// Route rows of target_leaf: bin <= split_bin → new_leaf_left, else new_leaf_right.
// node_idx is i32 (widened from u8 — lightgbm-exclusive, supports > 256 leaves).
extern "C" __global__
void leaf_split_apply_kernel(
      const unsigned char* bins_fm,
      int*                 node_idx,
      int                  target_leaf,
      int                  new_leaf_left,
      int                  new_leaf_right,
      int                  split_feature,
      unsigned char        split_bin,
      int                  n_rows,
      int                  n_features
) {
      int row = blockIdx.x * blockDim.x + threadIdx.x;
      if (row >= n_rows) return;
      if (node_idx[row] != target_leaf) return;
      unsigned char bin = bins_fm[(size_t)split_feature * n_rows + row];
      node_idx[row] = (bin <= split_bin) ? new_leaf_left : new_leaf_right;
}

extern "C" void launch_leaf_split_apply(
      const unsigned char* bins_fm, int* node_idx,
      int target_leaf, int new_leaf_left, int new_leaf_right,
      int split_feature, unsigned char split_bin,
      int n_rows, int n_features, hipStream_t stream
) {
      dim3 block(256);
      dim3 grid((n_rows + 255) / 256);
      hipLaunchKernelGGL(leaf_split_apply_kernel, grid, block, 0, stream,
            bins_fm, node_idx, target_leaf, new_leaf_left, new_leaf_right,
            split_feature, split_bin, n_rows, n_features);
}
