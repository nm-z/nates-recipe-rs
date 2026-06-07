#include <cstring>
#include <hip/hip_runtime.h>
#include <float.h>
#include <rocprim/rocprim.hpp>

// ── iota ──────────────────────────────────────────────────────────────────────
// Fills out[i] = i for i in [0, n).

__global__ void iota_kernel(int* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = i;
}

extern "C" void launch_iota(int* out, int n, hipStream_t stream) {
      dim3 block(256);
      dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(iota_kernel, grid, block, 0, stream, out, n);
}

// ── Bitonic argsort building blocks (ascending by f64 key) ───────────────────
// Keys padded with +INF sentinels so they sort to the tail. The caller drives
// the bitonic schedule: it allocates keys[pn] (f64) and vals[pn] (i32) with
// pn = next_pow2(n), fills keys[0..n) with the random keys, sets keys[n..pn) to
// the sentinel via launch_fill_sentinel_f64, inits vals via launch_init_int_idx,
// then runs the k/j double loop of launch_bitonic_step_idx. After the sort,
// vals[0..n) is the permutation (the caller copies it to its perm output).

__global__ void bitonic_step_idx_kernel_cb(double* keys, int* vals, int j, int k, int padded_n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= padded_n) return;
      int ixj = i ^ j;
      if (ixj > i) {
            int ascending = ((i & k) == 0) ? 1 : 0;
            double ka = keys[i], kb = keys[ixj];
            if ((ascending && ka > kb) || (!ascending && ka < kb)) {
                  keys[i] = kb; keys[ixj] = ka;
                  int vi = vals[i], vj = vals[ixj];
                  vals[i] = vj; vals[ixj] = vi;
            }
      }
}

extern "C" void launch_bitonic_step_idx(double* keys, int* vals, int j, int k, int padded_n, hipStream_t stream) {
      dim3 b(256), g((padded_n + 255) / 256);
      hipLaunchKernelGGL(bitonic_step_idx_kernel_cb, g, b, 0, stream, keys, vals, j, k, padded_n);
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

// ── random_permutation ────────────────────────────────────────────────────────
// Uses a simple LCG on device to fill rand_keys[i]; argsorting those keys (via
// the bitonic building blocks above) yields the permutation. The caller fills
// the keys with launch_lcg_rand, then runs the bitonic schedule.

__global__ void lcg_rand_kernel(double* out, int n, unsigned int seed) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      // Per-element LCG seeded from (seed, i)
      unsigned int state = seed ^ ((unsigned int)i * 2654435761u);
      state ^= state >> 16;
      state *= 0x45d9f3b;
      state ^= state >> 16;
      state *= 0x45d9f3b;
      state ^= state >> 16;
      // Map to [0, 1)
      out[i] = (double)(state & 0x00FFFFFFu) / (double)0x01000000;
}

extern "C" void launch_lcg_rand(double* out, int n, unsigned int seed, hipStream_t stream) {
      dim3 block(256), grid((n + 255) / 256);
      hipLaunchKernelGGL(lcg_rand_kernel, grid, block, 0, stream, out, n, seed);
}

// ── ordered_target_stats ──────────────────────────────────────────────────────
// Ordered (target-leakage-free) target statistics for CatBoost:
//   For each row r, walking along perm[0], perm[1], ..., perm[n-1] in order,
//   when we arrive at position p (row = perm[p]):
//     seen_sum[cat]   = sum of target[perm[0..p-1]] where cat_col[perm[j]] == cat
//     seen_cnt[cat]   = count of such rows
//     TS = (seen_sum[cat] + prior * smoothing) / (seen_cnt[cat] + smoothing)
//   encoded_out[ perm[p] ] = TS
//
// This is a single sequential scan along the permutation — no thread divergence
// on writes (each category's accumulators are written by the single thread that
// owns that position in the sweep). The kernel runs with 1 block of 1 thread so
// the sequential ordering is preserved exactly.
//
// Correctness note: we read perm[p] to get the original row index, look up its
// category, compute TS from accumulators built from rows perm[0..p-1] only (no
// current row in numerator/denominator), then update the accumulator AFTER
// writing. This is identical to what CatBoost's CPU path does and prevents the
// target from leaking into its own statistic.
//
// The cat_sum / cat_cnt accumulators (n_categories doubles each) are caller-owned
// scratch and must be zero-filled by the caller before launch.

__global__ void ordered_target_stats_kernel(
      const int*    cat_col,       // i32[n] — category index per row, 0-based
      const double* target,        // f64[n] — regression target per row
      const int*    perm,          // i32[n] — permutation order (from gpu_random_permutation)
      double*       encoded_out,   // f64[n] — output: TS value for each row
      double*       cat_sum,       // f64[n_categories] — scratch: per-category running sum
      double*       cat_cnt,       // f64[n_categories] — scratch: per-category running count
      int           n,
      double        prior,
      double        smoothing
) {
      // Sequential kernel: single thread walks perm order
      if (blockIdx.x != 0 || threadIdx.x != 0) return;

      for (int p = 0; p < n; p++) {
            int row = perm[p];
            int cat = cat_col[row];
            double s = cat_sum[cat];
            double c = cat_cnt[cat];
            // TS uses only history BEFORE current position
            encoded_out[row] = (s + prior * smoothing) / (c + smoothing);
            // Update accumulator with current row (available to subsequent rows)
            cat_sum[cat] = s + target[row];
            cat_cnt[cat] = c + 1.0;
      }
}

extern "C" void launch_ordered_target_stats(
      const int*    cat_col,
      const double* target,
      const int*    perm,
      double*       encoded_out,
      double*       cat_sum,       // scratch: n_categories doubles, caller zero-fills
      double*       cat_cnt,       // scratch: n_categories doubles, caller zero-fills
      int           n,
      int           n_categories,  // scratch sizing only; unused inside the wrapper
      double        prior,
      double        smoothing,
      hipStream_t   stream
) {
      (void)n_categories;
      hipLaunchKernelGGL(ordered_target_stats_kernel, dim3(1), dim3(1), 0, stream,
            cat_col, target, perm, encoded_out, cat_sum, cat_cnt,
            n, prior, smoothing);
}

// ── O(n) random permutation via rocPRIM radix sort ───────────────────────────
// Replaces the O(n log²n) bitonic argsort (and its power-of-two padding): sort
// (random f64 key, index) pairs by key; the reordered indices are the permutation.
// Caller owns the temp buffer (queried via radix_perm_workspace_bytes) and the
// double-buffer key/value outputs — matching the reduce.hip workspace convention.
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
