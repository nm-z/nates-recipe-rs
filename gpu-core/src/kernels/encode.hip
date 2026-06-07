#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>

// ── bin_edges_uniform ────────────────────────────────────────────────────────
// One thread per column. Finds per-column min/max, writes n_bins+1 edges
// (uniformly spaced). edges layout: col-major, edges[col*(n_bins+1)+b].
__global__ void bin_edges_uniform_kernel(
      const double* x, double* edges,
      int rows, int cols, int n_bins
) {
      int c = blockIdx.x * blockDim.x + threadIdx.x;
      if (c >= cols) return;

      double mn = x[c], mx = x[c];
      for (int r = 1; r < rows; r++) {
            double v = x[r * cols + c];
            if (v < mn) mn = v;
            if (v > mx) mx = v;
      }

      double* ep = edges + c * (n_bins + 1);
      double width = (n_bins > 0) ? (mx - mn) / (double)n_bins : 0.0;
      for (int b = 0; b <= n_bins; b++)
            ep[b] = mn + b * width;
}

extern "C" void launch_bin_edges_uniform(
      const double* x, double* edges,
      int rows, int cols, int n_bins, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (cols + threads - 1) / threads;
      hipLaunchKernelGGL(bin_edges_uniform_kernel, dim3(blocks), dim3(threads), 0, stream,
                         x, edges, rows, cols, n_bins);
}

// ── bitonic sort (ascending, in-place, power-of-two padded) ─────────────────
// One block sorts one column. blockDim.x == n_padded/2.
// Each thread handles two elements (tid, tid+n_padded/2) in its half of shared memory.
// The full sort is done via standard bitonic network over n_padded elements.
__global__ void bitonic_sort_kernel(double* data, int n, int n_padded) {
      extern __shared__ double sdata[];
      int tid = threadIdx.x;
      int base = blockIdx.x * n;

      // Load lower half
      sdata[tid]              = (tid < n)              ? data[base + tid]              : 1.0e300;
      // Load upper half
      sdata[tid + n_padded/2] = (tid + n_padded/2 < n) ? data[base + tid + n_padded/2] : 1.0e300;
      __syncthreads();

      // Standard bitonic network: each thread drives comparisons for its assigned index
      // Use sequential pair assignment: thread tid handles index 2*tid and 2*tid+1 per step.
      // Rewrite using the standard single-thread-per-element formulation over n_padded indices,
      // two elements per thread to keep blockDim == n_padded/2.
      for (int k = 2; k <= n_padded; k <<= 1) {
            for (int j = k >> 1; j >= 1; j >>= 1) {
                  // Process element at position tid
                  {
                        int i   = tid;
                        int ixj = i ^ j;
                        if (ixj > i) {
                              bool asc = ((i & k) == 0);
                              double a = sdata[i], b = sdata[ixj];
                              if ((asc && a > b) || (!asc && a < b)) {
                                    sdata[i] = b; sdata[ixj] = a;
                              }
                        }
                  }
                  // Process element at position tid + n_padded/2
                  {
                        int i   = tid + n_padded/2;
                        int ixj = i ^ j;
                        if (ixj > i) {
                              bool asc = ((i & k) == 0);
                              double a = sdata[i], b = sdata[ixj];
                              if ((asc && a > b) || (!asc && a < b)) {
                                    sdata[i] = b; sdata[ixj] = a;
                              }
                        }
                  }
                  __syncthreads();
            }
      }

      if (tid < n)              data[base + tid]              = sdata[tid];
      if (tid + n_padded/2 < n) data[base + tid + n_padded/2] = sdata[tid + n_padded/2];
}

// ── bin_edges_quantile ───────────────────────────────────────────────────────
// Copies each column into a transposed column-contiguous buffer, sorts each
// column with bitonic sort, then picks equally-spaced quantile values as edges.
// Precondition: rows <= 1024 (bitonic sort limit with shared memory).
__global__ void extract_column_kernel(
      const double* x, double* col_buf,
      int rows, int cols
) {
      int r = blockIdx.x * blockDim.x + threadIdx.x;
      int c = blockIdx.y;
      if (r < rows) col_buf[c * rows + r] = x[r * cols + c];
}

__global__ void bin_edges_from_sorted_kernel(
      const double* sorted_cols, double* edges,
      int rows, int cols, int n_bins
) {
      int c = blockIdx.x * blockDim.x + threadIdx.x;
      if (c >= cols) return;

      const double* sc = sorted_cols + c * rows;
      double* ep = edges + c * (n_bins + 1);
      ep[0] = sc[0];
      for (int b = 1; b < n_bins; b++) {
            // quantile index = b * (rows-1) / n_bins
            double fi = (double)b * (double)(rows - 1) / (double)n_bins;
            int lo = (int)fi;
            int hi = lo + 1 < rows ? lo + 1 : lo;
            double frac = fi - (double)lo;
            ep[b] = sc[lo] * (1.0 - frac) + sc[hi] * frac;
      }
      ep[n_bins] = sc[rows - 1];
}

extern "C" void launch_bin_edges_quantile(
      const double* x, double* edges, double* tmp_col_buf,
      int rows, int cols, int n_bins, hipStream_t stream
) {
      // Extract columns into col_buf (cols x rows, each row = one column of x)
      {
            dim3 block(256, 1);
            dim3 grid((rows + 255) / 256, cols);
            hipLaunchKernelGGL(extract_column_kernel, grid, block, 0, stream,
                               x, tmp_col_buf, rows, cols);
      }

      // Sort each column via bitonic sort (one block per column)
      // Requires rows power-of-two padded; handled by sentinel +inf fill above.
      int n_padded = 1;
      while (n_padded < rows) n_padded <<= 1;
      int threads = n_padded / 2;
      if (threads < 1) threads = 1;
      if (threads > 512) threads = 512;  // cap; bitonic works in two halves
      size_t smem = n_padded * sizeof(double);
      hipLaunchKernelGGL(bitonic_sort_kernel, dim3(cols), dim3(threads), smem, stream,
                         tmp_col_buf, rows, n_padded);

      // Extract edges from sorted columns
      {
            int threads2 = 256;
            int blocks2  = (cols + threads2 - 1) / threads2;
            hipLaunchKernelGGL(bin_edges_from_sorted_kernel, dim3(blocks2), dim3(threads2), 0, stream,
                               tmp_col_buf, edges, rows, cols, n_bins);
      }
}

// ── quantize_features ────────────────────────────────────────────────────────
// Binary search into pre-computed edges to assign each value a bin index [0, n_bins-1].
// Output is uint8_t (n_bins <= 255 assumed).
// edges layout: edges[col*(n_bins+1)+b], n_bins+1 values per column.
__global__ void quantize_features_kernel(
      const double* x, const double* edges, unsigned char* out,
      int rows, int cols, int n_bins
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = rows * cols;
      if (idx >= total) return;

      int r = idx / cols;
      int c = idx % cols;
      double v = x[r * cols + c];
      const double* ep = edges + c * (n_bins + 1);

      // Binary search: find largest b such that ep[b] <= v
      int lo = 0, hi = n_bins;
      while (lo < hi) {
            int mid = (lo + hi + 1) / 2;
            if (ep[mid] <= v) lo = mid;
            else              hi = mid - 1;
      }
      // Clamp to [0, n_bins-1]
      if (lo >= n_bins) lo = n_bins - 1;
      out[idx] = (unsigned char)lo;
}

extern "C" void launch_quantize_features(
      const double* x, const double* edges, unsigned char* out,
      int rows, int cols, int n_bins, hipStream_t stream
) {
      int total   = rows * cols;
      int threads = 256;
      int blocks  = (total + threads - 1) / threads;
      hipLaunchKernelGGL(quantize_features_kernel, dim3(blocks), dim3(threads), 0, stream,
                         x, edges, out, rows, cols, n_bins);
}

// ── one_hot ──────────────────────────────────────────────────────────────────
// labels_i32: int[n], out: double[n * n_classes] (row-major, one 1.0 per row).
__global__ void one_hot_kernel(
      const int* labels, double* out,
      int n, int n_classes
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;

      int base = i * n_classes;
      for (int c = 0; c < n_classes; c++) out[base + c] = 0.0;
      int lbl = labels[i];
      if (lbl >= 0 && lbl < n_classes) out[base + lbl] = 1.0;
}

extern "C" void launch_one_hot(
      const int* labels, double* out,
      int n, int n_classes, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(one_hot_kernel, dim3(blocks), dim3(threads), 0, stream,
                         labels, out, n, n_classes);
}

// ── count_distinct ────────────────────────────────────────────────────────────
// Counts distinct values in a sorted int32 array.
// Uses hipcub::DeviceRunLengthEncode to find run lengths, then counts the runs.
// Writes the number of runs directly into out[0] (no DtoD copy).
// Caller allocates unique_vals (n ints), run_counts (n ints), and temp
// (count_distinct_workspace_bytes() bytes). Caller owns any stream sync/readback.
extern "C" size_t count_distinct_workspace_bytes(
      const int* x, int n, hipStream_t stream
) {
      size_t temp_bytes = 0;
      hipcub::DeviceRunLengthEncode::Encode(nullptr, temp_bytes,
            x, (int*)nullptr, (int*)nullptr, (int*)nullptr, n, stream);
      return temp_bytes;
}

extern "C" void launch_count_distinct(
      const int* x, int* out, int* unique_vals, int* run_counts,
      void* temp, size_t temp_bytes, int n, hipStream_t stream
) {
      hipcub::DeviceRunLengthEncode::Encode(temp, temp_bytes,
            x, unique_vals, run_counts, out, n, stream);
}

// ── run_length ────────────────────────────────────────────────────────────────
// Run-length encoding of a SORTED int32 array.
// Precondition: x must be sorted ascending.
// Writes unique values into values_out, counts into counts_out (both pre-allocated,
// size n is sufficient), and the actual number of runs into n_runs_out[0].
// Caller allocates temp (run_length_workspace_bytes() bytes). Caller owns any
// stream sync/readback of n_runs_out.
extern "C" size_t run_length_workspace_bytes(
      const int* x, int n, hipStream_t stream
) {
      size_t temp_bytes = 0;
      hipcub::DeviceRunLengthEncode::Encode(nullptr, temp_bytes,
            x, (int*)nullptr, (int*)nullptr, (int*)nullptr, n, stream);
      return temp_bytes;
}

extern "C" void launch_run_length(
      const int* x, int* values_out, int* counts_out, int* n_runs_out,
      void* temp, size_t temp_bytes, int n, hipStream_t stream
) {
      hipcub::DeviceRunLengthEncode::Encode(temp, temp_bytes,
            x, values_out, counts_out, n_runs_out, n, stream);
}
