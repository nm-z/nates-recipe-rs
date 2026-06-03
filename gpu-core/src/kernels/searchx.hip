#include <hip/hip_runtime.h>
#include <float.h>
#include <math.h>

// ── "search" category kernels, f64 in / i32 out (collision-safe searchx_/launch_searchx_) ─
// Binary-search family, membership, find, sortedness, arg-extrema, signal extrema.
// Index outputs are i32 and are proven by EXACT equality against a CPU oracle.
//
// searchsorted(side): parallel per-needle binary search into a sorted array `a`.
//   side==0 (left  / numpy side='left'  / lower_bound): first i with a[i] >= v
//   side==1 (right / numpy side='right' / upper_bound): first i with a[i] >  v
// This one kernel backs searchsorted, lower_bound, upper_bound, binary_search,
// digitize and bucketize; each gets its OWN CPU oracle in the Rust harness
// (the `right`/`side` conventions differ between numpy and torch).

extern "C" __global__ void searchx_searchsorted_kernel(
            const double* a, int na, const double* v, int* out, int nv, int side) {
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= nv) return;
      double key = v[t];
      int lo = 0, hi = na;          // search in [lo, hi)
      while (lo < hi) {
            int mid = lo + ((hi - lo) >> 1);
            double am = a[mid];
            // side==0: move right while a[mid] <  key  -> lower_bound
            // side==1: move right while a[mid] <= key  -> upper_bound
            bool go_right = (side == 0) ? (am < key) : (am <= key);
            if (go_right) lo = mid + 1; else hi = mid;
      }
      out[t] = lo;
}
extern "C" void launch_searchx_searchsorted(
            const double* a, int na, const double* v, int* out, int nv, int side, hipStream_t s) {
      dim3 block(256); dim3 grid((nv + 255) / 256);
      hipLaunchKernelGGL(searchx_searchsorted_kernel, grid, block, 0, s, a, na, v, out, nv, side);
}

// isin: for each x[i], 1 if present in sorted haystack `a` else 0. Binary search.
extern "C" __global__ void searchx_isin_kernel(
            const double* a, int na, const double* x, int* out, int nx) {
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= nx) return;
      double key = x[t];
      int lo = 0, hi = na;
      while (lo < hi) {
            int mid = lo + ((hi - lo) >> 1);
            if (a[mid] < key) lo = mid + 1; else hi = mid;
      }
      out[t] = (lo < na && a[lo] == key) ? 1 : 0;
}
extern "C" void launch_searchx_isin(
            const double* a, int na, const double* x, int* out, int nx, hipStream_t s) {
      dim3 block(256); dim3 grid((nx + 255) / 256);
      hipLaunchKernelGGL(searchx_isin_kernel, grid, block, 0, s, a, na, x, out, nx);
}

// nonzero: compact indices i where x[i] != 0, in increasing order.
// Single-block sequential scan (deterministic order). out[] holds the indices,
// cnt[0] holds how many were written. Mirrors the scanx.hip sequential contract.
extern "C" __global__ void searchx_nonzero_kernel(
            const double* x, int n, int* out, int* cnt) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int c = 0;
            for (int i = 0; i < n; i++) if (x[i] != 0.0) out[c++] = i;
            cnt[0] = c;
      }
}
extern "C" void launch_searchx_nonzero(
            const double* x, int n, int* out, int* cnt, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_nonzero_kernel, g, b, 0, s, x, n, out, cnt);
}

// find: first index i where x[i] == target, else n. Sequential (first occurrence).
extern "C" __global__ void searchx_find_kernel(
            const double* x, int n, double target, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int idx = n;
            for (int i = 0; i < n; i++) if (x[i] == target) { idx = i; break; }
            out[0] = idx;
      }
}
extern "C" void launch_searchx_find(
            const double* x, int n, double target, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_find_kernel, g, b, 0, s, x, n, target, out);
}

// find_n: first start index of a run of k consecutive elements == target, else n.
extern "C" __global__ void searchx_find_n_kernel(
            const double* x, int n, double target, int k, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int idx = n, run = 0;
            for (int i = 0; i < n; i++) {
                  if (x[i] == target) { run++; if (run >= k) { idx = i - k + 1; break; } }
                  else run = 0;
            }
            out[0] = idx;
      }
}
extern "C" void launch_searchx_find_n(
            const double* x, int n, double target, int k, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_find_n_kernel, g, b, 0, s, x, n, target, k, out);
}

// is_sorted: out[0] = 1 if x is non-decreasing, else 0.
extern "C" __global__ void searchx_is_sorted_kernel(const double* x, int n, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int ok = 1;
            for (int i = 1; i < n; i++) if (x[i] < x[i-1]) { ok = 0; break; }
            out[0] = ok;
      }
}
extern "C" void launch_searchx_is_sorted(const double* x, int n, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_is_sorted_kernel, g, b, 0, s, x, n, out);
}

// is_sorted_until: index of the first element that breaks non-decreasing order
// (the first i with x[i] < x[i-1]); n if fully sorted. Same value as
// thrust::is_sorted_until's iterator position. partition_point shares this kernel.
extern "C" __global__ void searchx_is_sorted_until_kernel(const double* x, int n, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int idx = n;
            for (int i = 1; i < n; i++) if (x[i] < x[i-1]) { idx = i; break; }
            out[0] = idx;
      }
}
extern "C" void launch_searchx_is_sorted_until(const double* x, int n, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_is_sorted_until_kernel, g, b, 0, s, x, n, out);
}

// partition_point: on a range partitioned so a prefix satisfies pred (x[i] < pivot)
// and the suffix does not, return the first index where the predicate is false.
// Binary search (the range is partitioned, not necessarily sorted by value).
extern "C" __global__ void searchx_partition_point_kernel(
            const double* x, int n, double pivot, int* out) {
      int lo = 0, hi = n;
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            while (lo < hi) {
                  int mid = lo + ((hi - lo) >> 1);
                  if (x[mid] < pivot) lo = mid + 1; else hi = mid;   // pred = (x<pivot)
            }
            out[0] = lo;
      }
}
extern "C" void launch_searchx_partition_point(
            const double* x, int n, double pivot, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_partition_point_kernel, g, b, 0, s, x, n, pivot, out);
}

// mismatch: first index i where a[i] != b[i], else n (thrust::mismatch).
extern "C" __global__ void searchx_mismatch_kernel(
            const double* a, const double* b, int n, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int idx = n;
            for (int i = 0; i < n; i++) if (a[i] != b[i]) { idx = i; break; }
            out[0] = idx;
      }
}
extern "C" void launch_searchx_mismatch(
            const double* a, const double* b, int n, int* out, hipStream_t s) {
      dim3 b1(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_mismatch_kernel, g, b1, 0, s, a, b, n, out);
}

// arg-extrema. mode selects the comparison; all return the index of the FIRST
//   occurrence of the extreme element (BLAS/thrust convention).
//   mode 0 argmax (max value)        1 argmin (min value)
//   mode 2 idamax (max |value|)      3 idamin (min |value|)
// max_element == argmax, min_element == argmin (same kernel, mode 0/1).
extern "C" __global__ void searchx_argextreme_kernel(
            const double* x, int n, int mode, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int best = 0;
            double bv = (mode >= 2) ? fabs(x[0]) : x[0];
            for (int i = 1; i < n; i++) {
                  double key = (mode >= 2) ? fabs(x[i]) : x[i];
                  bool better = (mode == 0 || mode == 2) ? (key > bv) : (key < bv);
                  if (better) { bv = key; best = i; }
            }
            out[0] = best;
      }
}
extern "C" void launch_searchx_argextreme(
            const double* x, int n, int mode, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_argextreme_kernel, g, b, 0, s, x, n, mode, out);
}

// minmax_element: out[0]=argmin index, out[1]=argmax index (first occurrences).
extern "C" __global__ void searchx_minmax_element_kernel(const double* x, int n, int* out) {
      if (blockIdx.x == 0 && threadIdx.x == 0) {
            int lo = 0, hi = 0; double mn = x[0], mx = x[0];
            for (int i = 1; i < n; i++) {
                  double v = x[i];
                  if (v < mn) { mn = v; lo = i; }
                  if (v > mx) { mx = v; hi = i; }
            }
            out[0] = lo; out[1] = hi;
      }
}
extern "C" void launch_searchx_minmax_element(const double* x, int n, int* out, hipStream_t s) {
      dim3 b(1); dim3 g(1);
      hipLaunchKernelGGL(searchx_minmax_element_kernel, g, b, 0, s, x, n, out);
}

// argrel/find_peaks: strict local extrema with order=1 (compare to both
// immediate neighbors). Produces a 0/1 i32 mask over interior + endpoints.
//   mode 0 argrelmax / find_peaks : x[i] > x[i-1] && x[i] > x[i+1]
//   mode 1 argrelmin              : x[i] < x[i-1] && x[i] < x[i+1]
//   mode 2 argrelextrema          : either of the above
// Endpoints (i==0, i==n-1) are never extrema (scipy.signal default, no wrap).
extern "C" __global__ void searchx_argrel_kernel(const double* x, int n, int mode, int* mask) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      int r = 0;
      if (i > 0 && i < n - 1) {
            double a = x[i-1], v = x[i], b = x[i+1];
            bool mx = (v > a) && (v > b);
            bool mn = (v < a) && (v < b);
            if (mode == 0) r = mx ? 1 : 0;
            else if (mode == 1) r = mn ? 1 : 0;
            else r = (mx || mn) ? 1 : 0;
      }
      mask[i] = r;
}
extern "C" void launch_searchx_argrel(const double* x, int n, int mode, int* mask, hipStream_t s) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(searchx_argrel_kernel, grid, block, 0, s, x, n, mode, mask);
}
