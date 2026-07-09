#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>


extern "C" __global__ void sortx_iota_kernel(int* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = i;
}

extern "C" __global__ void sortx_copy_kernel(const double* src, double* dst, int k) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < k) dst[i] = src[i];
}

extern "C" __global__ void sortx_pick_int_kernel(const int* src, int* dst, int idx) {
      if (threadIdx.x == 0 && blockIdx.x == 0) dst[0] = src[idx];
}

extern "C" size_t sortx_sort_asc_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_sort_asc(const double* x, double* out, int n, void* workspace, size_t workspace_bytes, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeys(workspace, workspace_bytes, x, out, n, 0, sizeof(double) * 8, s);
}

extern "C" size_t sortx_sort_desc_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeysDescending(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_sort_desc(const double* x, double* out, int n, void* workspace, size_t workspace_bytes, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeysDescending(workspace, workspace_bytes, x, out, n, 0, sizeof(double) * 8, s);
}

extern "C" size_t sortx_argsort_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortPairs(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, (const int*)nullptr, (int*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_argsort(const double* x, int* out_idx, int n, double* keys_out, int* vals_in, void* workspace, size_t workspace_bytes, hipStream_t s) {
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(sortx_iota_kernel, g, b, 0, s, vals_in, n);
      hipcub::DeviceRadixSort::SortPairs(workspace, workspace_bytes, x, keys_out, vals_in, out_idx, n, 0, sizeof(double) * 8, s);
}

extern "C" size_t sortx_topk_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeysDescending(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_topk(const double* x, double* out, int n, int k, double* keys_out, void* workspace, size_t workspace_bytes, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeysDescending(workspace, workspace_bytes, x, keys_out, n, 0, sizeof(double) * 8, s);
      dim3 b(256); dim3 g((k + 255) / 256);
      hipLaunchKernelGGL(sortx_copy_kernel, g, b, 0, s, keys_out, out, k);
}

extern "C" __global__ void sortx_pick_kernel(const double* sorted, double* out, int idx) {
      if (threadIdx.x == 0 && blockIdx.x == 0) out[0] = sorted[idx];
}
extern "C" size_t sortx_kthvalue_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_kthvalue(const double* x, double* out, int n, int k, double* keys_out, void* workspace, size_t workspace_bytes, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeys(workspace, workspace_bytes, x, keys_out, n, 0, sizeof(double) * 8, s);
      hipLaunchKernelGGL(sortx_pick_kernel, dim3(1), dim3(1), 0, s, keys_out, out, k - 1);
}

extern "C" __global__ void sortx_median_kernel(const double* sorted, double* out, int n) {
      if (threadIdx.x == 0 && blockIdx.x == 0) {
            if (n & 1) out[0] = sorted[n / 2];
            else out[0] = 0.5 * (sorted[n / 2 - 1] + sorted[n / 2]);
      }
}
extern "C" size_t sortx_median_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_median(const double* x, double* out, int n, double* keys_out, void* workspace, size_t workspace_bytes, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeys(workspace, workspace_bytes, x, keys_out, n, 0, sizeof(double) * 8, s);
      hipLaunchKernelGGL(sortx_median_kernel, dim3(1), dim3(1), 0, s, keys_out, out, n);
}

extern "C" size_t sortx_argmax_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortPairsDescending(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, (const int*)nullptr, (int*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_argmax(const double* x, int* out_idx, int n, double* keys_out, int* vals_in, int* vals_out, void* workspace, size_t workspace_bytes, hipStream_t s) {
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(sortx_iota_kernel, g, b, 0, s, vals_in, n);
      hipcub::DeviceRadixSort::SortPairsDescending(workspace, workspace_bytes, x, keys_out, vals_in, vals_out, n, 0, sizeof(double) * 8, s);
      hipLaunchKernelGGL(sortx_pick_int_kernel, dim3(1), dim3(1), 0, s, vals_out, out_idx, 0);
}

extern "C" size_t sortx_argmin_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortPairs(nullptr, tmp_bytes, (const double*)nullptr, (double*)nullptr, (const int*)nullptr, (int*)nullptr, n, 0, sizeof(double) * 8);
      return tmp_bytes;
}
extern "C" void launch_sortx_argmin(const double* x, int* out_idx, int n, double* keys_out, int* vals_in, int* vals_out, void* workspace, size_t workspace_bytes, hipStream_t s) {
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(sortx_iota_kernel, g, b, 0, s, vals_in, n);
      hipcub::DeviceRadixSort::SortPairs(workspace, workspace_bytes, x, keys_out, vals_in, vals_out, n, 0, sizeof(double) * 8, s);
      hipLaunchKernelGGL(sortx_pick_int_kernel, dim3(1), dim3(1), 0, s, vals_out, out_idx, 0);
}

extern "C" __global__ void sortx_searchsorted_kernel(const double* sorted, const double* q, int* out, int n, int nq) {
      int j = blockIdx.x * blockDim.x + threadIdx.x;
      if (j >= nq) return;
      double key = q[j];
      int lo = 0, hi = n;
      while (lo < hi) {
            int mid = (lo + hi) >> 1;
            if (sorted[mid] < key) lo = mid + 1;
            else hi = mid;
      }
      out[j] = lo;
}
extern "C" void launch_sortx_searchsorted(const double* sorted, const double* q, int* out, int n, int nq, hipStream_t s) {
      dim3 b(256); dim3 g((nq + 255) / 256);
      hipLaunchKernelGGL(sortx_searchsorted_kernel, g, b, 0, s, sorted, q, out, n, nq);
}
