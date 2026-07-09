#include <cstring>
#include <hip/hip_runtime.h>
#include <float.h>
#include <rocprim/rocprim.hpp>


__global__ void iota_kernel(int* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = i;
}

extern "C" void launch_iota(int* out, int n, hipStream_t stream) {
      dim3 block(256);
      dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(iota_kernel, grid, block, 0, stream, out, n);
}


__global__ void fill_sentinel_f64_kernel(double* data, int real_n, int padded_n, double sentinel) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= real_n && i < padded_n) data[i] = sentinel;
}

extern "C" void launch_fill_sentinel_f64(double* data, int real_n, int padded_n, double sentinel, hipStream_t stream) {
      dim3 b(256), g((padded_n + 255) / 256);
      hipLaunchKernelGGL(fill_sentinel_f64_kernel, g, b, 0, stream, data, real_n, padded_n, sentinel);
}

__global__ void init_int_idx_kernel(int* idx, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) idx[i] = i;
}

extern "C" void launch_init_int_idx(int* idx, int n, hipStream_t stream) {
      dim3 b(256), g((n + 255) / 256);
      hipLaunchKernelGGL(init_int_idx_kernel, g, b, 0, stream, idx, n);
}


__global__ void lcg_rand_kernel(double* out, int n, unsigned int seed) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      unsigned int state = seed ^ ((unsigned int)i * 2654435761u);
      state ^= state >> 16;
      state *= 0x45d9f3b;
      state ^= state >> 16;
      state *= 0x45d9f3b;
      state ^= state >> 16;
      out[i] = (double)(state & 0x00FFFFFFu) / (double)0x01000000;
}

extern "C" void launch_lcg_rand(double* out, int n, unsigned int seed, hipStream_t stream) {
      dim3 block(256), grid((n + 255) / 256);
      hipLaunchKernelGGL(lcg_rand_kernel, grid, block, 0, stream, out, n, seed);
}


__global__ void ordered_target_stats_kernel(
      const int*    cat_col,
      const double* target,
      const int*    perm,
      double*       encoded_out,
      double*       cat_sum,
      double*       cat_cnt,
      int           n,
      const double* prior_p,
      const double* smoothing_p
) {
      double prior     = *prior_p;
      double smoothing = *smoothing_p;
      if (blockIdx.x != 0 || threadIdx.x != 0) return;

      for (int p = 0; p < n; p++) {
            int row = perm[p];
            int cat = cat_col[row];
            double s = cat_sum[cat];
            double c = cat_cnt[cat];
            encoded_out[row] = (s + prior * smoothing) / (c + smoothing);
            cat_sum[cat] = s + target[row];
            cat_cnt[cat] = c + 1.0;
      }
}

extern "C" void launch_ordered_target_stats(
      const int*    cat_col,
      const double* target,
      const int*    perm,
      double*       encoded_out,
      double*       cat_sum,
      double*       cat_cnt,
      int           n,
      int           n_categories,
      const double* prior,
      const double* smoothing,
      hipStream_t   stream
) {
      (void)n_categories;
      hipLaunchKernelGGL(ordered_target_stats_kernel, dim3(1), dim3(1), 0, stream,
            cat_col, target, perm, encoded_out, cat_sum, cat_cnt,
            n, prior, smoothing);
}

extern "C" size_t radix_perm_workspace_bytes(int n, hipStream_t stream) {
      size_t tmp_bytes = 0;
      rocprim::radix_sort_pairs(
            nullptr, tmp_bytes,
            (double*)nullptr, (double*)nullptr,
            (int*)nullptr, (int*)nullptr,
            (size_t)n, 0, sizeof(double) * 8, stream);
      return tmp_bytes;
}

extern "C" void launch_radix_sort_perm(
      double* keys, double* keys_out, const int* vals_in, int* vals_out,
      int n, void* tmp, size_t tmp_bytes, hipStream_t stream
) {
      rocprim::radix_sort_pairs(
            tmp, tmp_bytes,
            keys, keys_out,
            vals_in, vals_out,
            (size_t)n, 0, sizeof(double) * 8, stream);
}
