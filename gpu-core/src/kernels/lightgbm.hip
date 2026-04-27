#include <hip/hip_runtime.h>

// ── leaf_wise_best_split_kernel ────────────────────────────────────────────
// For each (leaf, feature, bin) triple where leaf_active[leaf]==1, compute
// the XGBoost split gain. Writes gains to gain_out[leaf * n_features * n_bins + feat * n_bins + bin].
// Leaves with leaf_active==0 get 0 gain.
//
// grad_hist / hess_hist layout: [n_leaves * n_features * n_bins]
// Indexed as: leaf * n_features * n_bins + feat * n_bins + bin

__global__ void leaf_wise_best_split_kernel(
      const float* grad_hist,
      const float* hess_hist,
      const int*   leaf_active,
      float        lambda,
      float        min_child_weight,
      float*       gain_out,
      int          n_leaves,
      int          n_features,
      int          n_bins
) {
      int leaf = blockIdx.z;
      int feat = blockIdx.x;
      int tid  = threadIdx.x;

      if (leaf >= n_leaves || feat >= n_features) return;
      if (!leaf_active[leaf]) {
            for (int b = tid; b < n_bins; b += blockDim.x)
                  gain_out[leaf * n_features * n_bins + feat * n_bins + b] = 0.0f;
            return;
      }

      __shared__ float s_g[256];
      __shared__ float s_h[256];

      int base = leaf * n_features * n_bins + feat * n_bins;
      for (int b = tid; b < n_bins; b += blockDim.x) {
            s_g[b] = (b < 256) ? grad_hist[base + b] : 0.0f;
            s_h[b] = (b < 256) ? hess_hist[base + b] : 0.0f;
      }
      __syncthreads();

      float total_g = 0.0f, total_h = 0.0f;
      for (int b = 0; b < n_bins; b++) { total_g += s_g[b]; total_h += s_h[b]; }
      float parent_score = (total_h + lambda > 0.0f) ? (total_g * total_g / (total_h + lambda)) : 0.0f;

      float left_g = 0.0f, left_h = 0.0f;
      for (int b = tid; b < n_bins - 1; b += blockDim.x) {
            float lg = 0.0f, lh = 0.0f;
            for (int i = 0; i <= b; i++) { lg += s_g[i]; lh += s_h[i]; }
            float rg = total_g - lg, rh = total_h - lh;
            float gain = 0.0f;
            if (lh >= min_child_weight && rh >= min_child_weight) {
                  gain = lg * lg / (lh + lambda) + rg * rg / (rh + lambda) - parent_score;
            }
            gain_out[base + b] = (gain > 0.0f) ? gain : 0.0f;
      }
      if (tid == 0) gain_out[base + n_bins - 1] = 0.0f;
}

extern "C" void launch_leaf_wise_best_split(
      const float* grad_hist,
      const float* hess_hist,
      const int*   leaf_active,
      float        lambda,
      float        min_child_weight,
      float*       gain_out,
      int          n_leaves,
      int          n_features,
      int          n_bins,
      hipStream_t  stream
) {
      dim3 grid(n_features, 1, n_leaves);
      dim3 block(256);
      size_t total = (size_t)n_leaves * n_features * n_bins * sizeof(float);
      hipMemsetAsync(gain_out, 0, total, stream);
      hipLaunchKernelGGL(leaf_wise_best_split_kernel, grid, block, 0, stream,
            grad_hist, hess_hist, leaf_active, lambda, min_child_weight,
            gain_out, n_leaves, n_features, n_bins);
}

// ── goss_sample_kernel ─────────────────────────────────────────────────────
// sorted_idx[0..top_k-1]  = indices of top-a rows by |grad| (already computed by argsort)
// sorted_idx[top_k..n-1]  = remaining rows
// weights_out[i] = 1.0 for i in top_k,
//                = (1-a)/b if sampled from rest (uniform_rand[sorted_idx[i]] < b),
//                = 0.0 otherwise.

__global__ void goss_sample_kernel(
      const int*   sorted_idx,
      float*       weights_out,
      const float* uniform_rand,
      int          n_rows,
      int          top_k,
      float        keep_weight
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n_rows) return;
      weights_out[i] = 0.0f;

      int row = sorted_idx[i];
      if (i < top_k) {
            weights_out[row] = 1.0f;
      } else {
            if (uniform_rand[row] < keep_weight) {
                  weights_out[row] = keep_weight;
            }
      }
}

extern "C" void launch_goss_sample(
      const int*   sorted_idx,
      float*       weights_out,
      const float* uniform_rand,
      int          n_rows,
      int          top_k,
      float        keep_weight,
      hipStream_t  stream
) {
      hipMemsetAsync(weights_out, 0, (size_t)n_rows * sizeof(float), stream);
      dim3 block(256);
      dim3 grid((n_rows + 255) / 256);
      hipLaunchKernelGGL(goss_sample_kernel, grid, block, 0, stream,
            sorted_idx, weights_out, uniform_rand, n_rows, top_k, keep_weight);
}

// ── leaf_split_apply_kernel ────────────────────────────────────────────────
// For rows in target_leaf: route left (bin <= split_bin) → new_leaf_left,
//                          route right (bin > split_bin) → new_leaf_right.
// Other rows: unchanged.

__global__ void leaf_split_apply_kernel(
      const unsigned char* bins_fm,
      unsigned char*       node_idx,
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
      if ((int)node_idx[row] != target_leaf) return;
      unsigned char bin = bins_fm[split_feature * n_rows + row];
      node_idx[row] = (unsigned char)((bin <= split_bin) ? new_leaf_left : new_leaf_right);
}

extern "C" void launch_leaf_split_apply(
      const unsigned char* bins_fm,
      unsigned char*       node_idx,
      int                  target_leaf,
      int                  new_leaf_left,
      int                  new_leaf_right,
      int                  split_feature,
      unsigned char        split_bin,
      int                  n_rows,
      int                  n_features,
      hipStream_t          stream
) {
      dim3 block(256);
      dim3 grid((n_rows + 255) / 256);
      hipLaunchKernelGGL(leaf_split_apply_kernel, grid, block, 0, stream,
            bins_fm, node_idx, target_leaf, new_leaf_left, new_leaf_right,
            split_feature, split_bin, n_rows, n_features);
}
