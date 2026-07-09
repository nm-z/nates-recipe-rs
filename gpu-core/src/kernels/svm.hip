#include <hip/hip_runtime.h>
#include <math.h>


#define TILE 16

__global__ void kernel_matrix_kernel(
      const double* x,
      double*       k_out,
      int           n,
      int           dim,
      int           kind,
      const double* gamma_p,
      const double* coef0_p,
      const double* degree_p
) {
      double gamma  = *gamma_p;
      double coef0  = *coef0_p;
      double degree = *degree_p;
      int row = blockIdx.y * TILE + threadIdx.y;
      int col = blockIdx.x * TILE + threadIdx.x;

      __shared__ double sa[TILE][TILE];
      __shared__ double sb[TILE][TILE];

      double dot  = 0.0;
      double sq_a = 0.0;
      double sq_b = 0.0;

      for (int tile = 0; tile < (dim + TILE - 1) / TILE; tile++) {
            int da = tile * TILE + threadIdx.x;
            int db = tile * TILE + threadIdx.y;

            sa[threadIdx.y][threadIdx.x] = (row < n && da < dim) ? x[row * dim + da] : 0.0;
            sb[threadIdx.y][threadIdx.x] = (col < n && db < dim) ? x[col * dim + db] : 0.0;
            __syncthreads();

            int tile_end = min(TILE, dim - tile * TILE);
            for (int d = 0; d < tile_end; d++) {
                  dot  += sa[threadIdx.y][d] * sb[d][threadIdx.x];
                  if (kind == 1) {
                        double da_val = sa[threadIdx.y][d];
                        double db_val = sb[d][threadIdx.x];
                        sq_a += da_val * da_val;
                        sq_b += db_val * db_val;
                  }
            }
            __syncthreads();
      }

      if (row >= n || col >= n) return;

      double val;
      if (kind == 0) {
            val = dot;
      } else if (kind == 1) {
            double dist2 = sq_a + sq_b - 2.0 * dot;
            if (dist2 < 0.0) dist2 = 0.0;
            val = exp(-gamma * dist2);
      } else if (kind == 2) {
            double inner = gamma * dot + coef0;
            val = pow(inner, degree);
      } else {
            val = tanh(gamma * dot + coef0);
      }

      k_out[row * n + col] = val;
}

extern "C" void launch_kernel_matrix(
      const double* x,
      double*       k_out,
      int           n,
      int           dim,
      int           kind,
      const double* gamma,
      const double* coef0,
      const double* degree,
      hipStream_t   stream
) {
      dim3 block(TILE, TILE);
      dim3 grid((n + TILE - 1) / TILE, (n + TILE - 1) / TILE);
      hipLaunchKernelGGL(kernel_matrix_kernel, grid, block, 0, stream,
            x, k_out, n, dim, kind, gamma, coef0, degree);
}


__global__ void smo_update_gradient_kernel(
      double*       grad,
      const double* K,
      int           n,
      int           row_i,
      int           row_j,
      double        delta_i,
      double        delta_j
) {
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= n) return;
      grad[t] += delta_i * K[row_i * n + t] + delta_j * K[row_j * n + t];
}

extern "C" void launch_smo_update_gradient(
      double*     grad,
      const double* K,
      int         n,
      int         row_i,
      int         row_j,
      double      delta_i,
      double      delta_j,
      hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_update_gradient_kernel, dim3(blocks), dim3(threads), 0, stream,
            grad, K, n, row_i, row_j, delta_i, delta_j);
}


__global__ void smo_kkt_score_kernel(
      const double* grad,
      const double* alpha,
      const double* y,
      double*       score_i,
      double*       score_j,
      int           n,
      const double* C_p
) {
      double C = *C_p;
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= n) return;

      double yt    = y[t];
      double at    = alpha[t];
      double F_t   = yt * grad[t];

      bool in_up = (yt > 0.0) ? (at < C) : (at > 0.0);
      bool in_down = (yt > 0.0) ? (at > 0.0) : (at < C);

      score_i[t] = in_up   ? (-F_t) : (-1e308);
      score_j[t] = in_down ? (-F_t) : (-1e308);
}

extern "C" void launch_smo_kkt_score(
      const double* grad,
      const double* alpha,
      const double* y,
      double*       score_i,
      double*       score_j,
      int           n,
      const double* C,
      hipStream_t   stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_kkt_score_kernel, dim3(blocks), dim3(threads), 0, stream,
            grad, alpha, y, score_i, score_j, n, C);
}

__global__ void smo_kernel_row_kernel(
      const double* x, double* krow, int n, int dim, int row,
      int kind, const double* gamma_p, const double* coef0_p, const double* degree_p
) {
      double gamma  = *gamma_p;
      double coef0  = *coef0_p;
      double degree = *degree_p;
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= n) return;
      const double* xr = x + (long)row * dim;
      const double* xt = x + (long)t   * dim;
      double dot = 0.0, sa = 0.0, sb = 0.0;
      for (int d = 0; d < dim; d++) {
            double a = xr[d], b = xt[d];
            dot += a * b;
            if (kind == 1) { sa += a * a; sb += b * b; }
      }
      double val;
      if (kind == 0) {
            val = dot;
      } else if (kind == 1) {
            double dist2 = sa + sb - 2.0 * dot; if (dist2 < 0.0) dist2 = 0.0;
            val = exp(-gamma * dist2);
      } else if (kind == 2) {
            val = pow(gamma * dot + coef0, degree);
      } else {
            val = tanh(gamma * dot + coef0);
      }
      krow[t] = val;
}

extern "C" void launch_smo_kernel_row(
      const double* x, double* krow, int n, int dim, int row,
      int kind, const double* gamma, const double* coef0, const double* degree, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_kernel_row_kernel, dim3(blocks), dim3(threads), 0, stream,
            x, krow, n, dim, row, kind, gamma, coef0, degree);
}

__global__ void smo_argmax_kernel(const double* s, double* out, int n) {
      __shared__ double vbuf[256];
      __shared__ int    ibuf[256];
      int tid = threadIdx.x;
      double best = -1e308; int bi = 0;
      for (int t = tid; t < n; t += blockDim.x) {
            double v = s[t];
            if (v > best) { best = v; bi = t; }
      }
      vbuf[tid] = best; ibuf[tid] = bi;
      __syncthreads();
      for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
            if (tid < stride && vbuf[tid + stride] > vbuf[tid]) {
                  vbuf[tid] = vbuf[tid + stride];
                  ibuf[tid] = ibuf[tid + stride];
            }
            __syncthreads();
      }
      if (tid == 0) { out[0] = vbuf[0]; out[1] = (double)ibuf[0]; }
}

extern "C" void launch_smo_argmax(const double* s, double* out, int n, hipStream_t stream) {
      hipLaunchKernelGGL(smo_argmax_kernel, dim3(1), dim3(256), 0, stream, s, out, n);
}

__global__ void smo_update_gradient_rows_kernel(
      double* grad, const double* ki, const double* kj, int n, const double* di_p, const double* dj_p
) {
      double di = *di_p;
      double dj = *dj_p;
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= n) return;
      grad[t] += di * ki[t] + dj * kj[t];
}

extern "C" void launch_smo_update_gradient_rows(
      double* grad, const double* ki, const double* kj, int n, const double* di, const double* dj, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_update_gradient_rows_kernel, dim3(blocks), dim3(threads), 0, stream,
            grad, ki, kj, n, di, dj);
}
