#include <cstring>
#include <hip/hip_runtime.h>
#include <rocprim/rocprim.hpp>
#include <float.h>

// ── Scalar all-reduce helpers ────────────────────────────────────────────────
// Signature: reduce(tmp_ptr, tmp_bytes, input, output, initial_value, size, op, stream)

extern "C" void launch_sum_all(const double* x, double* out, int n, hipStream_t stream) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, x, out, 0.0, (size_t)n, rocprim::plus<double>(), stream);
      void* d = nullptr;
      hipMalloc(&d, tmp);
      rocprim::reduce(d, tmp, x, out, 0.0, (size_t)n, rocprim::plus<double>(), stream);
      hipStreamSynchronize(stream);
      hipFree(d);
}

extern "C" void launch_max_all(const double* x, double* out, int n, hipStream_t stream) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, x, out, -DBL_MAX, (size_t)n, rocprim::maximum<double>(), stream);
      void* d = nullptr;
      hipMalloc(&d, tmp);
      rocprim::reduce(d, tmp, x, out, -DBL_MAX, (size_t)n, rocprim::maximum<double>(), stream);
      hipStreamSynchronize(stream);
      hipFree(d);
}

extern "C" void launch_min_all(const double* x, double* out, int n, hipStream_t stream) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, x, out, DBL_MAX, (size_t)n, rocprim::minimum<double>(), stream);
      void* d = nullptr;
      hipMalloc(&d, tmp);
      rocprim::reduce(d, tmp, x, out, DBL_MAX, (size_t)n, rocprim::minimum<double>(), stream);
      hipStreamSynchronize(stream);
      hipFree(d);
}

// mean_all: caller downloads out[0] and divides by n (avoids a second kernel)
extern "C" void launch_mean_all(const double* x, double* out, int n, hipStream_t stream) {
      launch_sum_all(x, out, n, stream);
}

// L2 norm: sum(x^2) — caller downloads and takes sqrt
__global__ void square_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[i] * x[i];
}

extern "C" void launch_l2_norm(const double* x, double* out, int n, hipStream_t stream) {
      double* sq = nullptr;
      hipMalloc(&sq, (size_t)n * sizeof(double));
      dim3 b(256), g((n + 255) / 256);
      hipLaunchKernelGGL(square_kernel, g, b, 0, stream, x, sq, n);
      launch_sum_all(sq, out, n, stream);
      hipFree(sq);
}

// Dot product: sum(a*b) — caller downloads out[0]
__global__ void mul_kernel_dot(const double* a, const double* b, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = a[i] * b[i];
}

extern "C" void launch_dot(const double* a, const double* b, double* out, int n, hipStream_t stream) {
      double* prod = nullptr;
      hipMalloc(&prod, (size_t)n * sizeof(double));
      dim3 b2(256), g2((n + 255) / 256);
      hipLaunchKernelGGL(mul_kernel_dot, g2, b2, 0, stream, a, b, prod, n);
      launch_sum_all(prod, out, n, stream);
      hipFree(prod);
}

// ── Bitonic sort ─────────────────────────────────────────────────────────────
// Pads to next power-of-two with +INF sentinels (ascending sort).
// Padded slots are filled with DBL_MAX so they sink to the tail.
// After sort, only the first n elements are valid.

__global__ void bitonic_step_kernel(double* data, int j, int k, int padded_n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= padded_n) return;
      int ixj = i ^ j;
      if (ixj > i) {
            int ascending = ((i & k) == 0) ? 1 : 0;
            double a = data[i], bv = data[ixj];
            if ((ascending && a > bv) || (!ascending && a < bv)) {
                  data[i] = bv;
                  data[ixj] = a;
            }
      }
}

__global__ void bitonic_step_idx_kernel(double* keys, int* vals, int j, int k, int padded_n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= padded_n) return;
      int ixj = i ^ j;
      if (ixj > i) {
            int ascending = ((i & k) == 0) ? 1 : 0;
            double a = keys[i], bv = keys[ixj];
            if ((ascending && a > bv) || (!ascending && a < bv)) {
                  keys[i] = bv; keys[ixj] = a;
                  int vi = vals[i], vj = vals[ixj];
                  vals[i] = vj; vals[ixj] = vi;
            }
      }
}

__global__ void init_idx_kernel(int* idx, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) idx[i] = i;
}

__global__ void fill_sentinel_kernel(double* data, int real_n, int padded_n, double sentinel) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= real_n && i < padded_n) data[i] = sentinel;
}

static int next_pow2(int n) {
      int p = 1;
      while (p < n) p <<= 1;
      return p;
}

extern "C" void launch_sort(const double* x, double* out, int n, hipStream_t stream) {
      int pn = next_pow2(n);
      double* work = nullptr;
      hipMalloc(&work, (size_t)pn * sizeof(double));
      hipMemcpyAsync(work, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      if (pn > n) {
            dim3 bg((pn + 255) / 256);
            hipLaunchKernelGGL(fill_sentinel_kernel, bg, dim3(256), 0, stream, work, n, pn, DBL_MAX);
      }
      dim3 b(256), g((pn + 255) / 256);
      for (int k = 2; k <= pn; k <<= 1) {
            for (int j = k >> 1; j > 0; j >>= 1) {
                  hipLaunchKernelGGL(bitonic_step_kernel, g, b, 0, stream, work, j, k, pn);
            }
      }
      hipMemcpyAsync(out, work, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      hipStreamSynchronize(stream);
      hipFree(work);
}

extern "C" void launch_argsort(const double* x, int* out_idx, int n, hipStream_t stream) {
      int pn = next_pow2(n);
      double* keys = nullptr;
      int* vals = nullptr;
      hipMalloc(&keys, (size_t)pn * sizeof(double));
      hipMalloc(&vals, (size_t)pn * sizeof(int));
      hipMemcpyAsync(keys, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      dim3 b(256), g((pn + 255) / 256);
      hipLaunchKernelGGL(init_idx_kernel, g, b, 0, stream, vals, pn);
      if (pn > n) {
            hipLaunchKernelGGL(fill_sentinel_kernel, g, b, 0, stream, keys, n, pn, DBL_MAX);
      }
      for (int k = 2; k <= pn; k <<= 1) {
            for (int j = k >> 1; j > 0; j >>= 1) {
                  hipLaunchKernelGGL(bitonic_step_idx_kernel, g, b, 0, stream, keys, vals, j, k, pn);
            }
      }
      hipMemcpyAsync(out_idx, vals, (size_t)n * sizeof(int), hipMemcpyDeviceToDevice, stream);
      hipStreamSynchronize(stream);
      hipFree(keys);
      hipFree(vals);
}

// Sort (double keys, double vals) pairs ascending by key.
// vals treated as raw bits via int* reinterpret — safe because sizeof(double)==sizeof(int64) != sizeof(int).
// We use a dedicated f64-f64 bitonic step instead.
__global__ void bitonic_step_dd_kernel(double* keys, double* vals, int j, int k, int padded_n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= padded_n) return;
      int ixj = i ^ j;
      if (ixj > i) {
            int ascending = ((i & k) == 0) ? 1 : 0;
            double ka = keys[i], kb = keys[ixj];
            if ((ascending && ka > kb) || (!ascending && ka < kb)) {
                  keys[i] = kb; keys[ixj] = ka;
                  double va = vals[i], vb = vals[ixj];
                  vals[i] = vb; vals[ixj] = va;
            }
      }
}

extern "C" void launch_sort_by_key(
      const double* keys, const double* vals, double* out_keys, double* out_vals,
      int n, hipStream_t stream
) {
      int pn = next_pow2(n);
      double* wk = nullptr;
      double* wv = nullptr;
      hipMalloc(&wk, (size_t)pn * sizeof(double));
      hipMalloc(&wv, (size_t)pn * sizeof(double));
      hipMemcpyAsync(wk, keys, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      hipMemcpyAsync(wv, vals, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      if (pn > n) {
            dim3 bg((pn + 255) / 256);
            hipLaunchKernelGGL(fill_sentinel_kernel, bg, dim3(256), 0, stream, wk, n, pn, DBL_MAX);
      }
      dim3 b(256), g((pn + 255) / 256);
      for (int k2 = 2; k2 <= pn; k2 <<= 1) {
            for (int j = k2 >> 1; j > 0; j >>= 1) {
                  hipLaunchKernelGGL(bitonic_step_dd_kernel, g, b, 0, stream, wk, wv, j, k2, pn);
            }
      }
      hipMemcpyAsync(out_keys, wk, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      hipMemcpyAsync(out_vals, wv, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, stream);
      hipStreamSynchronize(stream);
      hipFree(wk);
      hipFree(wv);
}

// Segment sort: sort within each contiguous segment.
// seg_offsets[s..s+1] are the start/end of segment s; seg_offsets[n_segs] = n.
// Uses insertion sort in shared memory — conservative, correct for segments <= 1024.
__global__ void segment_sort_kernel(
      const double* data, double* out, const int* seg_offsets, int n_segs
) {
      int s = blockIdx.x;
      if (s >= n_segs) return;
      int start = seg_offsets[s];
      int end   = seg_offsets[s + 1];
      int len   = end - start;
      if (len == 0) return;
      if (len == 1) { if (threadIdx.x == 0) out[start] = data[start]; return; }

      extern __shared__ double smem[];
      for (int i = threadIdx.x; i < len; i += blockDim.x) smem[i] = data[start + i];
      __syncthreads();

      if (threadIdx.x == 0) {
            for (int i = 1; i < len; i++) {
                  double v = smem[i];
                  int j = i - 1;
                  while (j >= 0 && smem[j] > v) { smem[j + 1] = smem[j]; j--; }
                  smem[j + 1] = v;
            }
      }
      __syncthreads();

      for (int i = threadIdx.x; i < len; i += blockDim.x) out[start + i] = smem[i];
}

extern "C" void launch_segment_sort(
      const double* data, const int* seg_offsets, double* out, int n, int n_segs, hipStream_t stream
) {
      (void)n;
      size_t smem = 1024 * sizeof(double);
      hipLaunchKernelGGL(segment_sort_kernel, dim3(n_segs), dim3(256), smem, stream,
            data, out, seg_offsets, n_segs);
}

// ── Inclusive scans ───────────────────────────────────────────────────────────

// cumsum_rows: for each row, inclusive prefix sum over columns.
// One block per row; sequential within thread 0, parallel across rows.
__global__ void cumsum_rows_kernel(const double* x, double* out, int rows, int cols) {
      int row = blockIdx.x;
      if (row >= rows) return;
      const double* rx = x + row * cols;
      double* ro = out + row * cols;
      if (threadIdx.x == 0) {
            double acc = 0.0;
            for (int c = 0; c < cols; c++) {
                  acc += rx[c];
                  ro[c] = acc;
            }
      }
}

extern "C" void launch_cumsum_rows(const double* x, double* out, int rows, int cols, hipStream_t stream) {
      hipLaunchKernelGGL(cumsum_rows_kernel, dim3(rows), dim3(1), 0, stream, x, out, rows, cols);
}

// cumsum_cols: for each column, inclusive prefix sum over rows.
__global__ void cumsum_cols_kernel(const double* x, double* out, int rows, int cols) {
      int col = blockIdx.x * blockDim.x + threadIdx.x;
      if (col >= cols) return;
      double acc = 0.0;
      for (int r = 0; r < rows; r++) {
            acc += x[r * cols + col];
            out[r * cols + col] = acc;
      }
}

extern "C" void launch_cumsum_cols(const double* x, double* out, int rows, int cols, hipStream_t stream) {
      dim3 b(256), g((cols + 255) / 256);
      hipLaunchKernelGGL(cumsum_cols_kernel, g, b, 0, stream, x, out, rows, cols);
}

// cumprod: inclusive product scan via rocprim
extern "C" void launch_cumprod(const double* x, double* out, int n, hipStream_t stream) {
      size_t tmp = 0;
      rocprim::inclusive_scan(nullptr, tmp, x, out, n, rocprim::multiplies<double>(), stream);
      void* d = nullptr;
      hipMalloc(&d, tmp);
      rocprim::inclusive_scan(d, tmp, x, out, n, rocprim::multiplies<double>(), stream);
      hipStreamSynchronize(stream);
      hipFree(d);
}

// cummax: inclusive max scan
struct MaxOp {
      __host__ __device__ double operator()(double a, double b) const { return a > b ? a : b; }
};

extern "C" void launch_cummax(const double* x, double* out, int n, hipStream_t stream) {
      size_t tmp = 0;
      rocprim::inclusive_scan(nullptr, tmp, x, out, n, MaxOp{}, stream);
      void* d = nullptr;
      hipMalloc(&d, tmp);
      rocprim::inclusive_scan(d, tmp, x, out, n, MaxOp{}, stream);
      hipStreamSynchronize(stream);
      hipFree(d);
}

// ── Segmented reduce by key ───────────────────────────────────────────────────
// seg_ids[i] = segment index for element i. Segments must be dense 0..n_segs-1.
// Elements need not be contiguous by segment; atomicAdd handles arbitrary order.

__global__ void segment_sum_kernel(const double* vals, const int* seg_ids, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) atomicAdd(&out[seg_ids[i]], vals[i]);
}

extern "C" void launch_segment_sum(
      const double* vals, const int* seg_ids, double* out, int n, int n_segs, hipStream_t stream
) {
      hipMemsetAsync(out, 0, (size_t)n_segs * sizeof(double), stream);
      dim3 b(256), g((n + 255) / 256);
      hipLaunchKernelGGL(segment_sum_kernel, g, b, 0, stream, vals, seg_ids, out, n);
}

// segment_max: atomic CAS loop for double (no native atomicMax for f64)
__global__ void segment_max_init_kernel(double* out, int n_segs) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n_segs) out[i] = -DBL_MAX;
}

__device__ void atomic_max_double(double* addr, double val) {
      unsigned long long* addr_ull = (unsigned long long*)addr;
      unsigned long long old_ull = *addr_ull;
      unsigned long long assumed;
      do {
            assumed = old_ull;
            if (__longlong_as_double(assumed) >= val) break;
            old_ull = atomicCAS(addr_ull, assumed, __double_as_longlong(val));
      } while (old_ull != assumed);
}

__global__ void segment_max_kernel(const double* vals, const int* seg_ids, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) atomic_max_double(&out[seg_ids[i]], vals[i]);
}

extern "C" void launch_segment_max(
      const double* vals, const int* seg_ids, double* out, int n, int n_segs, hipStream_t stream
) {
      dim3 b(256);
      dim3 g_segs((n_segs + 255) / 256);
      hipLaunchKernelGGL(segment_max_init_kernel, g_segs, b, 0, stream, out, n_segs);
      dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(segment_max_kernel, g, b, 0, stream, vals, seg_ids, out, n);
}

// ── S4/Mamba linear recurrence scan ──────────────────────────────────────────
// h_t = a_t * h_{t-1} + b_t  for each channel d in parallel.
// Layout: a[t*dim+d], b[t*dim+d], states[t*dim+d].
// Each thread owns one channel and loops over n_steps sequentially.
// Complexity: O(n_steps) sequential per channel; O(dim) parallel across threads.
// This is the conservative correct baseline. Parallel prefix (Blelloch) is possible
// with associative (a,b) pairs but requires 2x workspace and more complexity.
__global__ void scan_linear_recurrence_kernel(
      const double* a, const double* b, double* states, int n_steps, int dim
) {
      int d = blockIdx.x * blockDim.x + threadIdx.x;
      if (d >= dim) return;
      double h = 0.0;
      for (int t = 0; t < n_steps; t++) {
            int idx = t * dim + d;
            h = a[idx] * h + b[idx];
            states[idx] = h;
      }
}

extern "C" void launch_scan_linear_recurrence(
      const double* a, const double* b, double* states, int n_steps, int dim, hipStream_t stream
) {
      dim3 b2(256), g((dim + 255) / 256);
      hipLaunchKernelGGL(scan_linear_recurrence_kernel, g, b2, 0, stream, a, b, states, n_steps, dim);
}
