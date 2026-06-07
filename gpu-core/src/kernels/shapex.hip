#include <hip/hip_runtime.h>

// ── shape-category kernels, f64, output-indexed gather (out[i] = x[src(i)]) ───
// Every kernel writes each output element exactly once (no races). Dims are
// passed as plain ints; the 2D ops decompose the flat output index into (r,c).
// Names are collision-safe: shapex_ / launch_shapex_.

// pad(constant): out is (rows+pt+pb) x (cols+pl+pr); fill cval outside the
// source window, copy x[sr*cols+sc] inside. orow/ocol are output dims.
extern "C" __global__ void shapex_pad_constant_kernel(
      const double* x, double* out, int rows, int cols,
      int pt, int pl, int orow, int ocol, double cval, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int r = i / ocol, c = i % ocol;
            int sr = r - pt, sc = c - pl;
            if (sr >= 0 && sr < rows && sc >= 0 && sc < cols)
                  out[i] = x[sr * cols + sc];
            else
                  out[i] = cval;
      }
}
extern "C" void launch_shapex_pad_constant(
      const double* x, double* out, int rows, int cols,
      int pt, int pl, int orow, int ocol, double cval, hipStream_t s) {
      int n = orow * ocol;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_pad_constant_kernel, grid, block, 0, s,
            x, out, rows, cols, pt, pl, orow, ocol, cval, n);
}

// flip along axis: axis==0 flips rows (flipud), axis==1 flips cols (fliplr).
extern "C" __global__ void shapex_flip_kernel(
      const double* x, double* out, int rows, int cols, int axis, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int r = i / cols, c = i % cols;
            int sr = (axis == 0) ? (rows - 1 - r) : r;
            int sc = (axis == 1) ? (cols - 1 - c) : c;
            out[i] = x[sr * cols + sc];
      }
}
extern "C" void launch_shapex_flip(
      const double* x, double* out, int rows, int cols, int axis, hipStream_t s) {
      int n = rows * cols;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_flip_kernel, grid, block, 0, s, x, out, rows, cols, axis, n);
}

// roll on a 2D array by (sr, sc) using numpy-positive modulo. Source coord is
// (r - sr) mod rows, (c - sc) mod cols so element moves forward by the shift.
extern "C" __global__ void shapex_roll_kernel(
      const double* x, double* out, int rows, int cols, int sr, int sc, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int r = i / cols, c = i % cols;
            int rr = ((r - sr) % rows + rows) % rows;
            int cc = ((c - sc) % cols + cols) % cols;
            out[i] = x[rr * cols + cc];
      }
}
extern "C" void launch_shapex_roll(
      const double* x, double* out, int rows, int cols, int sr, int sc, hipStream_t s) {
      int n = rows * cols;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_roll_kernel, grid, block, 0, s, x, out, rows, cols, sr, sc, n);
}

// triu: keep upper triangle (c >= r + k), else 0.
extern "C" __global__ void shapex_triu_kernel(
      const double* x, double* out, int rows, int cols, int k, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int r = i / cols, c = i % cols;
            out[i] = (c >= r + k) ? x[i] : 0.0;
      }
}
extern "C" void launch_shapex_triu(
      const double* x, double* out, int rows, int cols, int k, hipStream_t s) {
      int n = rows * cols;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_triu_kernel, grid, block, 0, s, x, out, rows, cols, k, n);
}

// tril: keep lower triangle (c <= r + k), else 0.
extern "C" __global__ void shapex_tril_kernel(
      const double* x, double* out, int rows, int cols, int k, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int r = i / cols, c = i % cols;
            out[i] = (c <= r + k) ? x[i] : 0.0;
      }
}
extern "C" void launch_shapex_tril(
      const double* x, double* out, int rows, int cols, int k, hipStream_t s) {
      int n = rows * cols;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_tril_kernel, grid, block, 0, s, x, out, rows, cols, k, n);
}

// tile: out is (rows*rr) x (cols*rc); out[R][C] = x[R%rows][C%cols].
extern "C" __global__ void shapex_tile_kernel(
      const double* x, double* out, int rows, int cols, int rr, int rc, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int ocol = cols * rc;
            int R = i / ocol, C = i % ocol;
            int sr = R % rows, sc = C % cols;
            out[i] = x[sr * cols + sc];
      }
}
extern "C" void launch_shapex_tile(
      const double* x, double* out, int rows, int cols, int rr, int rc, hipStream_t s) {
      int n = (rows * rr) * (cols * rc);
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_tile_kernel, grid, block, 0, s, x, out, rows, cols, rr, rc, n);
}

// repeat (axis 0, interleave): out is (rows*reps) x cols; row R maps to source
// row R/reps. numpy.repeat([a,b], 2) -> [a,a,b,b] semantics row-wise.
extern "C" __global__ void shapex_repeat_kernel(
      const double* x, double* out, int rows, int cols, int reps, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int R = i / cols, c = i % cols;
            int sr = R / reps;
            out[i] = x[sr * cols + c];
      }
}
extern "C" void launch_shapex_repeat(
      const double* x, double* out, int rows, int cols, int reps, hipStream_t s) {
      int n = (rows * reps) * cols;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_repeat_kernel, grid, block, 0, s, x, out, rows, cols, reps, n);
}

// diagonal: extract out[d] = x[(d)*cols + (d+k)] for the k-th diagonal.
// Length is min(rows, cols-k) for k>=0 and min(rows+k, cols) for k<0.
extern "C" __global__ void shapex_diagonal_kernel(
      const double* x, double* out, int rows, int cols, int k, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            int r = (k >= 0) ? i : (i - k);
            int c = (k >= 0) ? (i + k) : i;
            out[i] = x[r * cols + c];
      }
}
extern "C" void launch_shapex_diagonal(
      const double* x, double* out, int rows, int cols, int k, int len, hipStream_t s) {
      int n = len;
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(shapex_diagonal_kernel, grid, block, 0, s, x, out, rows, cols, k, n);
}

// pack the upper triangle of a column-major (m×n, lda=m) factor into a packed
// column-major (n×n) R: R[j*n+i] = (i<=j) ? factor[j*m+i] : 0. Replaces a
// GPU→CPU→GPU round trip in gpu_qr. Each output element written exactly once.
extern "C" __global__ void shapex_pack_upper_tri_kernel(
      const double* factor, double* r, int m, int n) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = n * n;
      if (idx >= total) return;
      int j = idx / n;   // column
      int i = idx % n;   // row
      r[j * n + i] = (i <= j) ? factor[j * m + i] : 0.0;
}

extern "C" void launch_shapex_pack_upper_tri(
      const double* factor, double* r, int m, int n, hipStream_t s) {
      int total = n * n;
      dim3 block(256); dim3 grid((total + 255) / 256);
      hipLaunchKernelGGL(shapex_pack_upper_tri_kernel, grid, block, 0, s, factor, r, m, n);
}
