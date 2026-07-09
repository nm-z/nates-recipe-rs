#include <hip/hip_runtime.h>


__device__ uint2 forest_philox2x32(uint32_t counter, uint32_t key) {
      uint2 state = {counter, key};
      for (int i = 0; i < 10; i++) {
            uint32_t lo = state.x * 0xD2511F53u;
            uint32_t hi = __umulhi(state.x, 0xD2511F53u);
            state.x = hi ^ state.y ^ (key * (uint32_t)(i + 1));
            state.y = lo;
      }
      return state;
}

__device__ double forest_philox_uniform(uint32_t idx, uint32_t seed) {
      uint2 r = forest_philox2x32(idx, seed);
      return (double)(r.x) / 4294967296.0;
}


__global__ void floor_scale_to_idx_kernel(
      const double* uniform, int* idx_out, int n_samples, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n_samples) {
            int v = (int)(uniform[i] * (double)n);
            if (v >= n) v = n - 1;
            idx_out[i] = v;
      }
}

extern "C" void launch_floor_scale_to_idx(
      const double* uniform, int* idx_out, int n_samples, int n, hipStream_t stream
) {
      dim3 block(256);
      dim3 grid((n_samples + 255) / 256);
      hipLaunchKernelGGL(floor_scale_to_idx_kernel, grid, block, 0, stream,
                         uniform, idx_out, n_samples, n);
}


__global__ void forest_rand_keys_kernel(double* keys, int n_features, uint32_t seed) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n_features) {
            keys[i] = forest_philox_uniform((uint32_t)i, seed);
      }
}

__global__ void bitonic_argsort_kernel(
      const double* keys, int* idx, int n_features
) {
      extern __shared__ char smem[];
      double* s_keys = (double*)smem;
      int*    s_idx  = (int*)(smem + blockDim.x * sizeof(double));

      int tid = threadIdx.x;

      s_keys[tid] = tid < n_features ? keys[tid] : 2.0;
      s_idx[tid]  = tid;
      __syncthreads();

      int sz = blockDim.x;
      for (int k = 2; k <= sz; k *= 2) {
            for (int j = k / 2; j >= 1; j /= 2) {
                  int ixj = tid ^ j;
                  if (ixj > tid) {
                        bool ascending = ((tid & k) == 0);
                        bool swap_cond = ascending
                              ? (s_keys[tid] > s_keys[ixj])
                              : (s_keys[tid] < s_keys[ixj]);
                        if (swap_cond) {
                              double tk = s_keys[tid]; s_keys[tid] = s_keys[ixj]; s_keys[ixj] = tk;
                              int    ti = s_idx[tid];  s_idx[tid]  = s_idx[ixj];  s_idx[ixj]  = ti;
                        }
                  }
                  __syncthreads();
            }
      }

      if (tid < n_features) {
            idx[tid] = s_idx[tid];
      }
}

extern "C" void launch_feature_subset(
      double* keys, int* idx_out, int n_features, int k, uint32_t seed, hipStream_t stream
) {
      dim3 block_k(256);
      dim3 grid_k((n_features + 255) / 256);
      hipLaunchKernelGGL(forest_rand_keys_kernel, grid_k, block_k, 0, stream,
                         keys, n_features, seed);

      int np2 = 1;
      while (np2 < n_features) np2 *= 2;
      if (np2 > 1024) np2 = 1024;

      size_t smem_bytes = (size_t)np2 * (sizeof(double) + sizeof(int));
      hipLaunchKernelGGL(bitonic_argsort_kernel, dim3(1), dim3(np2), smem_bytes, stream,
                         keys, idx_out, n_features);
}


__global__ void col_minmax_kernel(const double* col, double* out_min, double* out_max, int n) {
      __shared__ double s_min[256];
      __shared__ double s_max[256];
      int tid = threadIdx.x;
      double lmin = 1e308, lmax = -1e308;
      for (int i = tid; i < n; i += blockDim.x) {
            double v = col[i];
            if (v < lmin) lmin = v;
            if (v > lmax) lmax = v;
      }
      s_min[tid] = lmin;
      s_max[tid] = lmax;
      __syncthreads();
      for (int stride = blockDim.x / 2; stride > 0; stride /= 2) {
            if (tid < stride) {
                  if (s_min[tid + stride] < s_min[tid]) s_min[tid] = s_min[tid + stride];
                  if (s_max[tid + stride] > s_max[tid]) s_max[tid] = s_max[tid + stride];
            }
            __syncthreads();
      }
      if (tid == 0) {
            out_min[0] = s_min[0];
            out_max[0] = s_max[0];
      }
}

__global__ void draw_threshold_kernel(
      const double* col_min, const double* col_max,
      double* threshold_out, uint32_t seed
) {
      double mn = col_min[0];
      double mx = col_max[0];
      double u = forest_philox_uniform(0u, seed);
      threshold_out[0] = mn + u * (mx - mn);
}

extern "C" void launch_random_threshold_split(
      const double* col, double* d_min, double* d_max,
      double* threshold_out, int n, uint32_t seed, hipStream_t stream
) {
      int threads = 256;
      hipLaunchKernelGGL(col_minmax_kernel, dim3(1), dim3(threads), 0, stream,
                         col, d_min, d_max, n);

      hipLaunchKernelGGL(draw_threshold_kernel, dim3(1), dim3(1), 0, stream,
                         d_min, d_max, threshold_out, seed);
}


__global__ void scatter_used_kernel(const int* bootstrap_idx, unsigned char* used, int n_samples) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n_samples) {
            used[bootstrap_idx[i]] = (unsigned char)1;
      }
}

__global__ void invert_mask_kernel(const unsigned char* used, unsigned char* oob, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) oob[i] = used[i] ? (unsigned char)0 : (unsigned char)1;
}

extern "C" void launch_oob_mask(
      const int* bootstrap_idx, unsigned char* used, unsigned char* oob_out,
      int n_samples, int n, hipStream_t stream
) {
      dim3 block(256);
      dim3 grid_s((n_samples + 255) / 256);
      hipLaunchKernelGGL(scatter_used_kernel, grid_s, block, 0, stream,
                         bootstrap_idx, used, n_samples);

      dim3 grid_n((n + 255) / 256);
      hipLaunchKernelGGL(invert_mask_kernel, grid_n, block, 0, stream,
                         used, oob_out, n);
}

__global__ void tree_ensemble_predict_kernel(
      const unsigned char* bins,
      const int* node_feature, const int* node_thresh,
      const int* node_left, const int* node_right,
      const unsigned char* node_is_leaf, const double* node_value,
      const int* tree_root, double* out,
      int n, int n_trees, const double* lr
) {
      double lr_v = *lr;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double acc = 0.0;
      for (int t = 0; t < n_trees; t++) {
            int node = tree_root[t];
            while (node_is_leaf[node] == (unsigned char)0) {
                  int f = node_feature[node];
                  int thr = node_thresh[node];
                  int bv = (int)bins[f * n + i];
                  node = (bv <= thr) ? node_left[node] : node_right[node];
            }
            acc += node_value[node];
      }
      out[i] = lr_v * acc;
}

extern "C" void launch_tree_ensemble_predict(
      const unsigned char* bins,
      const int* node_feature, const int* node_thresh,
      const int* node_left, const int* node_right,
      const unsigned char* node_is_leaf, const double* node_value,
      const int* tree_root, double* out,
      int n, int n_trees, const double* lr, hipStream_t stream
) {
      dim3 block(256);
      dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(tree_ensemble_predict_kernel, grid, block, 0, stream,
                         bins, node_feature, node_thresh, node_left, node_right,
                         node_is_leaf, node_value, tree_root, out, n, n_trees, lr);
}
