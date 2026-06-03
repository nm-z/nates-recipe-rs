#include <hip/hip_runtime.h>

// ── histogram-category kernels (unique histogramx_ / launch_histogramx_ names) ──
//
// histc:    count f64 values into B equal-width bins over [lo, hi].
//           Convention: bin index = floor((x - lo) / w), w = (hi - lo) / B.
//           Out-of-range values (x < lo or x > hi) are dropped; x == hi maps to
//           the last bin (right edge closed, matches numpy/torch histc). Counts
//           accumulate via atomicAdd into a zero-initialized int buffer.
//
// bincount: count occurrences of non-negative int labels into out[0..n_bins).
//           out_len must be >= max(label)+1; labels >= out_len are dropped.

extern "C" __global__ void histogramx_histc_kernel(
      const double* x, int* counts, int n, double lo, double hi, int bins) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double v = x[i];
      if (v < lo || v > hi) return;
      double w = (hi - lo) / (double)bins;
      int b = (int)((v - lo) / w);
      if (b == bins) b = bins - 1;          // x == hi -> last bin
      if (b < 0) b = 0;
      if (b >= bins) b = bins - 1;
      atomicAdd(&counts[b], 1);
}

extern "C" void launch_histogramx_histc(
      const double* x, int* counts, int n, double lo, double hi, int bins, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(histogramx_histc_kernel, grid, block, 0, s, x, counts, n, lo, hi, bins);
}

extern "C" __global__ void histogramx_bincount_kernel(
      const int* labels, int* counts, int n, int out_len) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      int l = labels[i];
      if (l < 0 || l >= out_len) return;
      atomicAdd(&counts[l], 1);
}

extern "C" void launch_histogramx_bincount(
      const int* labels, int* counts, int n, int out_len, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(histogramx_bincount_kernel, grid, block, 0, s, labels, counts, n, out_len);
}
