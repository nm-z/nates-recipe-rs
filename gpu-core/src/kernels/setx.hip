#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>

// ── "set" category kernels (collision-safe setx_ / launch_setx_ names) ──
// Sorted-distinct (unique), adjacent-dedup (unique_consecutive), membership
// (isin), and run-length unique+counts. All dedup is delegated to hipcub
// (DeviceSelect::Unique / DeviceRunLengthEncode::Encode); sorting to
// DeviceRadixSort. Oracle for all = CPU set ops (see tests/prove_set.rs).
//
// Variable-length outputs: each launcher writes into a caller pre-allocated
// device buffer (capacity n) and reports the produced length via a device
// int* out_count that hipcub fills (d_num_selected). isin is fixed length na.

// ── unique_consecutive: collapse ADJACENT duplicates only (no sort) ──
// out[0..*out_count) = run-leaders of x in original order.
extern "C" void launch_setx_unique_consecutive(
      const double* x, double* out, int* out_count, int n, hipStream_t s) {
      double* in = nullptr;
      hipMalloc(&in, (size_t)n * sizeof(double));
      hipMemcpyAsync(in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t tmp_bytes = 0;
      hipcub::DeviceSelect::Unique(nullptr, tmp_bytes, in, out, out_count, n, s);
      void* tmp = nullptr;
      hipMalloc(&tmp, tmp_bytes);
      hipcub::DeviceSelect::Unique(tmp, tmp_bytes, in, out, out_count, n, s);
      hipFree(tmp);
      hipFree(in);
}

// ── unique: sorted distinct = radix sort then adjacent dedup ──
extern "C" void launch_setx_unique(
      const double* x, double* out, int* out_count, int n, hipStream_t s) {
      double* keys_in  = nullptr;
      double* keys_sorted = nullptr;
      hipMalloc(&keys_in,     (size_t)n * sizeof(double));
      hipMalloc(&keys_sorted, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      // sort ascending
      size_t sort_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, sort_bytes, keys_in, keys_sorted, n, 0, sizeof(double) * 8, s);
      void* sort_tmp = nullptr;
      hipMalloc(&sort_tmp, sort_bytes);
      hipcub::DeviceRadixSort::SortKeys(sort_tmp, sort_bytes, keys_in, keys_sorted, n, 0, sizeof(double) * 8, s);
      hipFree(sort_tmp);
      // adjacent dedup on the sorted array == global distinct
      size_t sel_bytes = 0;
      hipcub::DeviceSelect::Unique(nullptr, sel_bytes, keys_sorted, out, out_count, n, s);
      void* sel_tmp = nullptr;
      hipMalloc(&sel_tmp, sel_bytes);
      hipcub::DeviceSelect::Unique(sel_tmp, sel_bytes, keys_sorted, out, out_count, n, s);
      hipFree(sel_tmp);
      hipFree(keys_sorted);
      hipFree(keys_in);
}

// ── unique_counts: sorted distinct values + their multiplicities ──
// vals_out[0..*out_count) sorted distinct; counts_out[i] = # occurrences.
extern "C" void launch_setx_unique_counts(
      const double* x, double* vals_out, int* counts_out, int* out_count, int n, hipStream_t s) {
      double* keys_in     = nullptr;
      double* keys_sorted = nullptr;
      hipMalloc(&keys_in,     (size_t)n * sizeof(double));
      hipMalloc(&keys_sorted, (size_t)n * sizeof(double));
      hipMemcpyAsync(keys_in, x, (size_t)n * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t sort_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, sort_bytes, keys_in, keys_sorted, n, 0, sizeof(double) * 8, s);
      void* sort_tmp = nullptr;
      hipMalloc(&sort_tmp, sort_bytes);
      hipcub::DeviceRadixSort::SortKeys(sort_tmp, sort_bytes, keys_in, keys_sorted, n, 0, sizeof(double) * 8, s);
      hipFree(sort_tmp);
      size_t rle_bytes = 0;
      hipcub::DeviceRunLengthEncode::Encode(nullptr, rle_bytes, keys_sorted, vals_out, counts_out, out_count, n, s);
      void* rle_tmp = nullptr;
      hipMalloc(&rle_tmp, rle_bytes);
      hipcub::DeviceRunLengthEncode::Encode(rle_tmp, rle_bytes, keys_sorted, vals_out, counts_out, out_count, n, s);
      hipFree(rle_tmp);
      hipFree(keys_sorted);
      hipFree(keys_in);
}

// ── isin: mask[i] = (a[i] in b) ? 1.0 : 0.0, via sort(b) + binary search ──
// One thread per element of a; lower_bound on sorted b, exact-equality check.
extern "C" __global__ void setx_isin_kernel(
      const double* a, const double* b_sorted, double* mask, int na, int nb) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= na) return;
      double key = a[i];
      int lo = 0, hi = nb;            // lower_bound: first index with b_sorted[idx] >= key
      while (lo < hi) {
            int mid = lo + ((hi - lo) >> 1);
            if (b_sorted[mid] < key) lo = mid + 1; else hi = mid;
      }
      mask[i] = (lo < nb && b_sorted[lo] == key) ? 1.0 : 0.0;
}

extern "C" void launch_setx_isin(
      const double* a, const double* b, double* mask, int na, int nb, hipStream_t s) {
      double* b_in     = nullptr;
      double* b_sorted = nullptr;
      hipMalloc(&b_in,     (size_t)nb * sizeof(double));
      hipMalloc(&b_sorted, (size_t)nb * sizeof(double));
      hipMemcpyAsync(b_in, b, (size_t)nb * sizeof(double), hipMemcpyDeviceToDevice, s);
      size_t sort_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(nullptr, sort_bytes, b_in, b_sorted, nb, 0, sizeof(double) * 8, s);
      void* sort_tmp = nullptr;
      hipMalloc(&sort_tmp, sort_bytes);
      hipcub::DeviceRadixSort::SortKeys(sort_tmp, sort_bytes, b_in, b_sorted, nb, 0, sizeof(double) * 8, s);
      hipFree(sort_tmp);
      dim3 block(256); dim3 grid((na + 255) / 256);
      hipLaunchKernelGGL(setx_isin_kernel, grid, block, 0, s, a, b_sorted, mask, na, nb);
      hipFree(b_sorted);
      hipFree(b_in);
}
