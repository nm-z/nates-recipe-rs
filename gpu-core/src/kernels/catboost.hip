#include <hip/hip_runtime.h>
#include <float.h>

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

// ── Bitonic argsort (self-contained, ascending by f64 key) ───────────────────
// Keys padded with +INF sentinels so they sort to the tail.

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

__global__ void fill_sentinel_f64_kernel(double* data, int real_n, int padded_n, double sentinel) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= real_n && i < padded_n) data[i] = sentinel;
}

__global__ void init_int_idx_kernel(int* idx, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) idx[i] = i;
}

static int cb_next_pow2(int n) {
      int p = 1;
      while (p < n) p <<= 1;
      return p;
}

// Sorts keys ascending and returns the permutation in idx_out.
// keys_in: f64[n] (read-only device ptr), idx_out: int[n].
static void bitonic_argsort_f64(const double* keys_in, int* idx_out, int n, hipStream_t stream) {
      int pn = cb_next_pow2(n);
      double* keys = nullptr;
      int* vals = nullptr;
      hipMalloc(&keys, (size_t)pn * sizeof(double));
      hipMalloc(&vals, (size_t)pn * sizeof(int));
      hipMemcpyAsync(keys, keys_in, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      dim3 b(256), g((pn + 255) / 256);
      hipLaunchKernelGGL(init_int_idx_kernel, g, b, 0, stream, vals, pn);
      if (pn > n) {
            hipLaunchKernelGGL(fill_sentinel_f64_kernel, g, b, 0, stream, keys, n, pn, DBL_MAX);
      }
      for (int k = 2; k <= pn; k <<= 1) {
            for (int j = k >> 1; j > 0; j >>= 1) {
                  hipLaunchKernelGGL(bitonic_step_idx_kernel_cb, g, b, 0, stream, keys, vals, j, k, pn);
            }
      }
      hipMemcpyAsync(idx_out, vals, (size_t)n * sizeof(int), hipMemcpyDeviceToDevice, stream);
      hipStreamSynchronize(stream);
      hipFree(keys);
      hipFree(vals);
}

// ── random_permutation ────────────────────────────────────────────────────────
// Uses a simple LCG on device to fill rand_keys[i], then argsorts them.
// The resulting argsort indices ARE the permutation.

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

extern "C" void launch_random_permutation(
      int* perm_out, int n, unsigned int seed, hipStream_t stream
) {
      double* rand_keys = nullptr;
      hipMalloc(&rand_keys, (size_t)n * sizeof(double));

      dim3 block(256), grid((n + 255) / 256);
      hipLaunchKernelGGL(lcg_rand_kernel, grid, block, 0, stream, rand_keys, n, seed);

      // argsort rand_keys ascending => random permutation
      bitonic_argsort_f64(rand_keys, perm_out, n, stream);

      hipFree(rand_keys);
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
      int           n,
      int           n_categories,
      double        prior,
      double        smoothing,
      hipStream_t   stream
) {
      double* cat_sum = nullptr;
      double* cat_cnt = nullptr;
      hipMalloc(&cat_sum, (size_t)n_categories * sizeof(double));
      hipMalloc(&cat_cnt, (size_t)n_categories * sizeof(double));
      hipMemsetAsync(cat_sum, 0, (size_t)n_categories * sizeof(double), stream);
      hipMemsetAsync(cat_cnt, 0, (size_t)n_categories * sizeof(double), stream);
      hipStreamSynchronize(stream);

      hipLaunchKernelGGL(ordered_target_stats_kernel, dim3(1), dim3(1), 0, stream,
            cat_col, target, perm, encoded_out, cat_sum, cat_cnt,
            n, prior, smoothing);

      hipStreamSynchronize(stream);
      hipFree(cat_sum);
      hipFree(cat_cnt);
}
