#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>


extern "C" size_t launch_setx_unique_consecutive_workspace_bytes(int n) {
      size_t tmp_bytes = 0;
      hipcub::DeviceSelect::Unique(
            nullptr, tmp_bytes,
            (const double*)nullptr, (double*)nullptr, (int*)nullptr, n, (hipStream_t)0);
      return tmp_bytes;
}

extern "C" void launch_setx_unique_consecutive(
      const double* x, double* out, int* out_count,
      void* tmp, size_t tmp_bytes, int n, hipStream_t s) {
      hipcub::DeviceSelect::Unique(tmp, tmp_bytes, x, out, out_count, n, s);
}

extern "C" size_t launch_setx_unique_workspace_bytes(int n) {
      size_t sort_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(
            nullptr, sort_bytes,
            (const double*)nullptr, (double*)nullptr, n, 0, (int)sizeof(double) * 8, (hipStream_t)0);
      size_t sel_bytes = 0;
      hipcub::DeviceSelect::Unique(
            nullptr, sel_bytes,
            (const double*)nullptr, (double*)nullptr, (int*)nullptr, n, (hipStream_t)0);
      return sort_bytes > sel_bytes ? sort_bytes : sel_bytes;
}

extern "C" void launch_setx_unique(
      const double* x, double* keys_sorted, double* out, int* out_count,
      void* tmp, size_t tmp_bytes, int n, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeys(tmp, tmp_bytes, x, keys_sorted, n, 0, sizeof(double) * 8, s);
      hipcub::DeviceSelect::Unique(tmp, tmp_bytes, keys_sorted, out, out_count, n, s);
}

extern "C" size_t launch_setx_unique_counts_workspace_bytes(int n) {
      size_t sort_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(
            nullptr, sort_bytes,
            (const double*)nullptr, (double*)nullptr, n, 0, (int)sizeof(double) * 8, (hipStream_t)0);
      size_t rle_bytes = 0;
      hipcub::DeviceRunLengthEncode::Encode(
            nullptr, rle_bytes,
            (const double*)nullptr, (double*)nullptr, (int*)nullptr, (int*)nullptr, n, (hipStream_t)0);
      return sort_bytes > rle_bytes ? sort_bytes : rle_bytes;
}

extern "C" void launch_setx_unique_counts(
      const double* x, double* keys_sorted, double* vals_out, int* counts_out, int* out_count,
      void* tmp, size_t tmp_bytes, int n, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeys(tmp, tmp_bytes, x, keys_sorted, n, 0, sizeof(double) * 8, s);
      hipcub::DeviceRunLengthEncode::Encode(tmp, tmp_bytes, keys_sorted, vals_out, counts_out, out_count, n, s);
}

extern "C" __global__ void setx_isin_kernel(
      const double* a, const double* b_sorted, double* mask, int na, int nb) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= na) return;
      double key = a[i];
      int lo = 0, hi = nb;
      while (lo < hi) {
            int mid = lo + ((hi - lo) >> 1);
            if (b_sorted[mid] < key) lo = mid + 1; else hi = mid;
      }
      mask[i] = (lo < nb && b_sorted[lo] == key) ? 1.0 : 0.0;
}

extern "C" size_t launch_setx_isin_workspace_bytes(int nb) {
      size_t sort_bytes = 0;
      hipcub::DeviceRadixSort::SortKeys(
            nullptr, sort_bytes,
            (const double*)nullptr, (double*)nullptr, nb, 0, (int)sizeof(double) * 8, (hipStream_t)0);
      return sort_bytes;
}

extern "C" void launch_setx_isin(
      const double* a, const double* b, double* b_sorted, double* mask,
      void* tmp, size_t tmp_bytes, int na, int nb, hipStream_t s) {
      hipcub::DeviceRadixSort::SortKeys(tmp, tmp_bytes, b, b_sorted, nb, 0, sizeof(double) * 8, s);
      dim3 block(256); dim3 grid((na + 255) / 256);
      hipLaunchKernelGGL(setx_isin_kernel, grid, block, 0, s, a, b_sorted, mask, na, nb);
}
