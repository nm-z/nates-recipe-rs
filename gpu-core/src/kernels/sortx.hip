#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>

// ── "sort" category kernels (collision-safe sortx_ / launch_sortx_ names) ──
// Full sorts via hipcub::DeviceRadixSort (device radix sort on gfx1101).
// Oracle for all of these = CPU sort (see tests/prove_sort.rs).

// Fill out[i] = i (index iota) used as the values payload for pair sorts.
extern "C" __global__ void sortx_iota_kernel(int* out, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) out[i] = i;
}

// ── sort ascending: keys-only radix sort ──
extern "C" void launch_sortx_sort_asc(const double* x, double* out, int n, hipStream_t s) {
      double* keys_in = nullptr;
      hipMalloc(&keys_in, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, tmp_bytes, keys_in, out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortKeys(tmp, tmp_bytes, keys_in, out, n, 0, sizeof(double) * 8, s);
      hipFree(tmp);
      hipFree(keys_in);
}

// ── sort descending: keys-only radix sort, descending ──
extern "C" void launch_sortx_sort_desc(const double* x, double* out, int n, hipStream_t s) {
      double* keys_in = nullptr;
      hipMalloc(&keys_in, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeysDescending(nullptr, tmp_bytes, keys_in, out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortKeysDescending(tmp, tmp_bytes, keys_in, out, n, 0, sizeof(double) * 8, s);
      hipFree(tmp);
      hipFree(keys_in);
}

// ── argsort ascending: pair sort (key=value, val=index), output sorted indices ──
extern "C" void launch_sortx_argsort(const double* x, int* out_idx, int n, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_out = nullptr;
      int*    vals_in  = nullptr;
      hipMalloc(&keys_in,  (size_t)n * sizeof(double));
      hipMalloc(&keys_out, (size_t)n * sizeof(double));
      hipMalloc(&vals_in,  (size_t)n * sizeof(int));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(sortx_iota_kernel, g, b, 0, s, vals_in, n);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortPairs(nullptr, tmp_bytes, keys_in, keys_out, vals_in, out_idx, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortPairs(tmp, tmp_bytes, keys_in, keys_out, vals_in, out_idx, n, 0, sizeof(double) * 8, s);
      hipFree(tmp);
      hipFree(keys_in);
      hipFree(keys_out);
      hipFree(vals_in);
}

// ── topk: the k LARGEST values, in descending order (torch.topk convention) ──
extern "C" void launch_sortx_topk(const double* x, double* out, int n, int k, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_out = nullptr;
      hipMalloc(&keys_in,  (size_t)n * sizeof(double));
      hipMalloc(&keys_out, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeysDescending(nullptr, tmp_bytes, keys_in, keys_out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortKeysDescending(tmp, tmp_bytes, keys_in, keys_out, n, 0, sizeof(double) * 8, s);
      hipMemcpyAsync(out, keys_out, (size_t)k * sizeof(double), hipMemcpyDeviceToDevice, s);
      hipFree(tmp);
      hipFree(keys_in);
      hipFree(keys_out);
}

// ── kthvalue: the k-th SMALLEST value (1-based k, torch.kthvalue convention) ──
// out[0] = sorted_asc[k-1]
extern "C" __global__ void sortx_pick_kernel(const double* sorted, double* out, int idx) {
      if (threadIdx.x == 0 && blockIdx.x == 0) out[0] = sorted[idx];
}
extern "C" void launch_sortx_kthvalue(const double* x, double* out, int n, int k, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_out = nullptr;
      hipMalloc(&keys_in,  (size_t)n * sizeof(double));
      hipMalloc(&keys_out, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, tmp_bytes, keys_in, keys_out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortKeys(tmp, tmp_bytes, keys_in, keys_out, n, 0, sizeof(double) * 8, s);
      hipLaunchKernelGGL(sortx_pick_kernel, dim3(1), dim3(1), 0, s, keys_out, out, k - 1);
      hipFree(tmp);
      hipFree(keys_in);
      hipFree(keys_out);
}

// ── median: middle of ascending sort; even n => mean of the two central elems ──
extern "C" __global__ void sortx_median_kernel(const double* sorted, double* out, int n) {
      if (threadIdx.x == 0 && blockIdx.x == 0) {
            if (n & 1) out[0] = sorted[n / 2];
            else out[0] = 0.5 * (sorted[n / 2 - 1] + sorted[n / 2]);
      }
}
extern "C" void launch_sortx_median(const double* x, double* out, int n, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_out = nullptr;
      hipMalloc(&keys_in,  (size_t)n * sizeof(double));
      hipMalloc(&keys_out, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, tmp_bytes, keys_in, keys_out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortKeys(tmp, tmp_bytes, keys_in, keys_out, n, 0, sizeof(double) * 8, s);
      hipLaunchKernelGGL(sortx_median_kernel, dim3(1), dim3(1), 0, s, keys_out, out, n);
      hipFree(tmp);
      hipFree(keys_in);
      hipFree(keys_out);
}

// ── argmax: index of the maximum element (argsort desc, take first index) ──
extern "C" void launch_sortx_argmax(const double* x, int* out_idx, int n, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_out = nullptr;
      int*    vals_in  = nullptr;
      int*    vals_out = nullptr;
      hipMalloc(&keys_in,  (size_t)n * sizeof(double));
      hipMalloc(&keys_out, (size_t)n * sizeof(double));
      hipMalloc(&vals_in,  (size_t)n * sizeof(int));
      hipMalloc(&vals_out, (size_t)n * sizeof(int));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(sortx_iota_kernel, g, b, 0, s, vals_in, n);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortPairsDescending(nullptr, tmp_bytes, keys_in, keys_out, vals_in, vals_out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortPairsDescending(tmp, tmp_bytes, keys_in, keys_out, vals_in, vals_out, n, 0, sizeof(double) * 8, s);
      hipMemcpyAsync(out_idx, vals_out, sizeof(int), hipMemcpyDeviceToDevice, s);
      hipFree(tmp);
      hipFree(keys_in);
      hipFree(keys_out);
      hipFree(vals_in);
      hipFree(vals_out);
}

// ── argmin: index of the minimum element (argsort asc, take first index) ──
extern "C" void launch_sortx_argmin(const double* x, int* out_idx, int n, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_out = nullptr;
      int*    vals_in  = nullptr;
      int*    vals_out = nullptr;
      hipMalloc(&keys_in,  (size_t)n * sizeof(double));
      hipMalloc(&keys_out, (size_t)n * sizeof(double));
      hipMalloc(&vals_in,  (size_t)n * sizeof(int));
      hipMalloc(&vals_out, (size_t)n * sizeof(int));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(sortx_iota_kernel, g, b, 0, s, vals_in, n);
      size_t tmp_bytes = 0;
      hipcub::DeviceRadixSort::SortPairs(nullptr, tmp_bytes, keys_in, keys_out, vals_in, vals_out, n, 0, sizeof(double) * 8, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceRadixSort::SortPairs(tmp, tmp_bytes, keys_in, keys_out, vals_in, vals_out, n, 0, sizeof(double) * 8, s);
      hipMemcpyAsync(out_idx, vals_out, sizeof(int), hipMemcpyDeviceToDevice, s);
      hipFree(tmp);
      hipFree(keys_in);
      hipFree(keys_out);
      hipFree(vals_in);
      hipFree(vals_out);
}

// ── searchsorted (left side): for each query q, count of sorted[i] < q ──
// out[j] = number of elements in `sorted` strictly less than q[j] (numpy 'left').
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
