#include <cstring>
#include <hip/hip_runtime.h>
#include <rocprim/rocprim.hpp>
#include <float.h>
#include "dtype_dispatch.h"


extern "C" size_t launch_sum_all_workspace_bytes(int n) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, (const double*)nullptr, (double*)nullptr, 0.0, (size_t)n, rocprim::plus<double>(), hipStream_t(0));
      return tmp;
}

extern "C" void launch_sum_all(const double* x, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      size_t tmp = workspace_bytes;
      rocprim::reduce(workspace, tmp, x, out, 0.0, (size_t)n, rocprim::plus<double>(), stream);
}

extern "C" size_t launch_max_all_workspace_bytes(int n) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, (const double*)nullptr, (double*)nullptr, -DBL_MAX, (size_t)n, rocprim::maximum<double>(), hipStream_t(0));
      return tmp;
}

extern "C" void launch_max_all(const double* x, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      size_t tmp = workspace_bytes;
      rocprim::reduce(workspace, tmp, x, out, -DBL_MAX, (size_t)n, rocprim::maximum<double>(), stream);
}

extern "C" size_t launch_min_all_workspace_bytes(int n) {
      size_t tmp = 0;
      rocprim::reduce(nullptr, tmp, (const double*)nullptr, (double*)nullptr, DBL_MAX, (size_t)n, rocprim::minimum<double>(), hipStream_t(0));
      return tmp;
}

extern "C" void launch_min_all(const double* x, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      size_t tmp = workspace_bytes;
      rocprim::reduce(workspace, tmp, x, out, DBL_MAX, (size_t)n, rocprim::minimum<double>(), stream);
}

extern "C" size_t launch_mean_all_workspace_bytes(int n) {
      return launch_sum_all_workspace_bytes(n);
}

__global__ void scale_inv_n_kernel(double* out, int n) {
      if (blockIdx.x == 0 && threadIdx.x == 0) out[0] = out[0] / (double)n;
}

extern "C" void launch_mean_all(const double* x, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      launch_sum_all(x, out, workspace, workspace_bytes, n, stream);
      hipLaunchKernelGGL(scale_inv_n_kernel, dim3(1), dim3(1), 0, stream, out, n);
}

__global__ void square_kernel(const double* x, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = x[i] * x[i];
}

extern "C" size_t launch_l2_norm_workspace_bytes(int n) {
      return launch_sum_all_workspace_bytes(n);
}

__global__ void sqrt_scalar_kernel(double* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) out[0] = sqrt(out[0]);
}

extern "C" void launch_l2_norm(const double* x, double* sq, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      dim3 b(256), g((n + 255) / 256);
      hipLaunchKernelGGL(square_kernel, g, b, 0, stream, x, sq, n);
      launch_sum_all(sq, out, workspace, workspace_bytes, n, stream);
      hipLaunchKernelGGL(sqrt_scalar_kernel, dim3(1), dim3(1), 0, stream, out);
}

__global__ void mul_kernel_dot(const double* a, const double* b, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = a[i] * b[i];
}

extern "C" size_t launch_dot_workspace_bytes(int n) {
      return launch_sum_all_workspace_bytes(n);
}

extern "C" void launch_dot(const double* a, const double* b, double* prod, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      dim3 b2(256), g2((n + 255) / 256);
      hipLaunchKernelGGL(mul_kernel_dot, g2, b2, 0, stream, a, b, prod, n);
      launch_sum_all(prod, out, workspace, workspace_bytes, n, stream);
}


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

__global__ void fill_sentinel_kernel(double* data, int real_n, int padded_n, const double* sentinel) {
      double s = *sentinel;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= real_n && i < padded_n) data[i] = s;
}

extern "C" void launch_fill_sentinel(double* data, int real_n, int padded_n, const double* sentinel, hipStream_t stream) {
      dim3 b(256), g((padded_n + 255) / 256);
      hipLaunchKernelGGL(fill_sentinel_kernel, g, b, 0, stream, data, real_n, padded_n, sentinel);
}

extern "C" void launch_init_idx(int* idx, int n, hipStream_t stream) {
      dim3 b(256), g((n + 255) / 256);
      hipLaunchKernelGGL(init_idx_kernel, g, b, 0, stream, idx, n);
}

extern "C" void launch_bitonic_step(double* data, int j, int k, int padded_n, hipStream_t stream) {
      dim3 b(256), g((padded_n + 255) / 256);
      hipLaunchKernelGGL(bitonic_step_kernel, g, b, 0, stream, data, j, k, padded_n);
}

extern "C" void launch_bitonic_step_idx(double* keys, int* vals, int j, int k, int padded_n, hipStream_t stream) {
      dim3 b(256), g((padded_n + 255) / 256);
      hipLaunchKernelGGL(bitonic_step_idx_kernel, g, b, 0, stream, keys, vals, j, k, padded_n);
}

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

extern "C" void launch_bitonic_step_dd(double* keys, double* vals, int j, int k, int padded_n, hipStream_t stream) {
      dim3 b(256), g((padded_n + 255) / 256);
      hipLaunchKernelGGL(bitonic_step_dd_kernel, g, b, 0, stream, keys, vals, j, k, padded_n);
}

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

extern "C" size_t launch_cumprod_workspace_bytes(int n) {
      size_t tmp = 0;
      rocprim::inclusive_scan(nullptr, tmp, (const double*)nullptr, (double*)nullptr, n, rocprim::multiplies<double>(), hipStream_t(0));
      return tmp;
}

extern "C" void launch_cumprod(const double* x, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      size_t tmp = workspace_bytes;
      rocprim::inclusive_scan(workspace, tmp, x, out, n, rocprim::multiplies<double>(), stream);
}

struct MaxOp {
      __host__ __device__ double operator()(double a, double b) const { return a > b ? a : b; }
};

extern "C" size_t launch_cummax_workspace_bytes(int n) {
      size_t tmp = 0;
      rocprim::inclusive_scan(nullptr, tmp, (const double*)nullptr, (double*)nullptr, n, MaxOp{}, hipStream_t(0));
      return tmp;
}

extern "C" void launch_cummax(const double* x, double* out, void* workspace, size_t workspace_bytes, int n, hipStream_t stream) {
      size_t tmp = workspace_bytes;
      rocprim::inclusive_scan(workspace, tmp, x, out, n, MaxOp{}, stream);
}


__global__ void segment_sum_kernel(const double* vals, const int* seg_ids, double* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) atomicAdd(&out[seg_ids[i]], vals[i]);
}

extern "C" void launch_segment_sum(
      const double* vals, const int* seg_ids, double* out, int n, int n_segs, hipStream_t stream
) {
      (void)n_segs;
      dim3 b(256), g((n + 255) / 256);
      hipLaunchKernelGGL(segment_sum_kernel, g, b, 0, stream, vals, seg_ids, out, n);
}

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

// state_g (nullable, [dim], thread d owns state_g[d]=h): loaded at entry, written
// back at exit, so an incremental scan over only the NEW steps is bit-identical to
// a full-sequence scan. Null keeps the h=0 init and no write-back.
template<typename T> __global__ void scan_linear_recurrence_kernel(
      const T* a, const T* b, T* states, int n_steps, int dim, T* state_g
) {
      int d = blockIdx.x * blockDim.x + threadIdx.x;
      if (d >= dim) return;
      T h = state_g ? state_g[d] : (T)0.0;
      for (int t = 0; t < n_steps; t++) {
            int idx = t * dim + d;
            h = a[idx] * h + b[idx];
            states[idx] = h;
      }
      if (state_g) state_g[d] = h;
}

extern "C" void launch_scan_linear_recurrence(
      const double* a, const double* b, double* states, int n_steps, int dim, double* state, hipStream_t stream, int dtype
) {
      dim3 b2(256), g((dim + 255) / 256);
      DT_DISPATCH(dtype, scan_linear_recurrence_kernel<T><<<g, b2, 0, stream>>>((const T*)a, (const T*)b, (T*)states, n_steps, dim, (T*)state));
}
