#include <hip/hip_runtime.h>

// ── Philox 2x32-10 RNG (self-contained, matches elementwise.hip) ─────────

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

// ── floor(u * n) -> i32 bootstrap index kernel ───────────────────────────
// Input:  uniform[n_samples] in [0,1)
// Output: idx[n_samples] in [0, n-1] (with replacement)

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

// ── Feature subset: generate random keys, bitonic argsort (small n_features) ─

// Generate random f64 keys in [0,1) for n_features positions
__global__ void forest_rand_keys_kernel(double* keys, int n_features, uint32_t seed) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n_features) {
            keys[i] = forest_philox_uniform((uint32_t)i, seed);
      }
}

// Bitonic argsort: sort indices by keys ascending.
// Assumes n_features is padded to a power-of-two inside the kernel by treating
// out-of-range slots as key=2.0 (infinity). Works for n_features <= 1024
// (launched with blockDim.x = next_pow2(n_features), one block).
__global__ void bitonic_argsort_kernel(
      const double* keys, int* idx, int n_features
) {
      extern __shared__ char smem[];
      double* s_keys = (double*)smem;
      int*    s_idx  = (int*)(smem + blockDim.x * sizeof(double));

      int tid = threadIdx.x;

      // Load into shared memory; pad with sentinel
      s_keys[tid] = tid < n_features ? keys[tid] : 2.0;
      s_idx[tid]  = tid;
      __syncthreads();

      // Bitonic sort (ascending) on shared memory
      int sz = blockDim.x; // power-of-two
      for (int k = 2; k <= sz; k *= 2) {
            for (int j = k / 2; j >= 1; j /= 2) {
                  int ixj = tid ^ j;
                  if (ixj > tid) {
                        // Determine sort direction
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

      // Write first k sorted indices out; caller reads first k entries
      if (tid < n_features) {
            idx[tid] = s_idx[tid];
      }
}

// Host launcher: generates random keys then bitonic-sorts to give first k indices
extern "C" void launch_feature_subset(
      int* idx_out, int n_features, int k, uint32_t seed, hipStream_t stream
) {
      // Allocate temporary key buffer
      double* keys = nullptr;
      hipMalloc(&keys, n_features * sizeof(double));

      // Generate random keys
      dim3 block_k(256);
      dim3 grid_k((n_features + 255) / 256);
      hipLaunchKernelGGL(forest_rand_keys_kernel, grid_k, block_k, 0, stream,
                         keys, n_features, seed);

      // Compute next power-of-two for block size
      int np2 = 1;
      while (np2 < n_features) np2 *= 2;
      // Cap at 1024 (max HIP threads per block)
      if (np2 > 1024) np2 = 1024;

      size_t smem_bytes = (size_t)np2 * (sizeof(double) + sizeof(int));
      hipLaunchKernelGGL(bitonic_argsort_kernel, dim3(1), dim3(np2), smem_bytes, stream,
                         keys, idx_out, n_features);

      hipStreamSynchronize(stream);
      hipFree(keys);
}

// ── Random threshold split: find col min/max, draw one uniform in [min,max] ─

// Two-pass reduce: first find min and max of a column vector
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
      // Parallel reduction
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

// Draw one uniform threshold in [min, max] using Philox
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
      const double* col, double* threshold_out, int n, uint32_t seed, hipStream_t stream
) {
      // Temporary min/max scalars
      double* d_min = nullptr;
      double* d_max = nullptr;
      hipMalloc(&d_min, sizeof(double));
      hipMalloc(&d_max, sizeof(double));

      int threads = 256;
      hipLaunchKernelGGL(col_minmax_kernel, dim3(1), dim3(threads), 0, stream,
                         col, d_min, d_max, n);

      hipLaunchKernelGGL(draw_threshold_kernel, dim3(1), dim3(1), 0, stream,
                         d_min, d_max, threshold_out, seed);

      hipStreamSynchronize(stream);
      hipFree(d_min);
      hipFree(d_max);
}

// ── OOB mask: u8[n], 1 where index never appears in bootstrap ────────────

// Scatter 'used' flags: for each bootstrap index, mark used[bootstrap_idx[i]] = 1
__global__ void scatter_used_kernel(const int* bootstrap_idx, unsigned char* used, int n_samples) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n_samples) {
            used[bootstrap_idx[i]] = (unsigned char)1;
      }
}

// Invert: oob[i] = 1 - used[i]
__global__ void invert_mask_kernel(const unsigned char* used, unsigned char* oob, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) oob[i] = used[i] ? (unsigned char)0 : (unsigned char)1;
}

extern "C" void launch_oob_mask(
      const int* bootstrap_idx, unsigned char* oob_out,
      int n_samples, int n, hipStream_t stream
) {
      // Temporary 'used' array, zero-initialized
      unsigned char* used = nullptr;
      hipMalloc(&used, n * sizeof(unsigned char));
      hipMemsetAsync(used, 0, n * sizeof(unsigned char), stream);

      dim3 block(256);
      dim3 grid_s((n_samples + 255) / 256);
      hipLaunchKernelGGL(scatter_used_kernel, grid_s, block, 0, stream,
                         bootstrap_idx, used, n_samples);

      dim3 grid_n((n + 255) / 256);
      hipLaunchKernelGGL(invert_mask_kernel, grid_n, block, 0, stream,
                         used, oob_out, n);

      hipStreamSynchronize(stream);
      hipFree(used);
}
