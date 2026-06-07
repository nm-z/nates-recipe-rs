#include <hip/hip_runtime.h>
#include <math.h>

// ── Kernel matrix computation ────────────────────────────────────────────────
// kind: 0=linear  k(a,b) = a·b
//       1=rbf     k(a,b) = exp(-gamma * ||a-b||^2)
//       2=poly    k(a,b) = (gamma * a·b + coef0)^degree
//       3=sigmoid k(a,b) = tanh(gamma * a·b + coef0)
// Output: K[n*n], row-major.

#define TILE 16

__global__ void kernel_matrix_kernel(
      const double* x,
      double*       k_out,
      int           n,
      int           dim,
      int           kind,
      double        gamma,
      double        coef0,
      double        degree
) {
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
      double        gamma,
      double        coef0,
      double        degree,
      hipStream_t   stream
) {
      dim3 block(TILE, TILE);
      dim3 grid((n + TILE - 1) / TILE, (n + TILE - 1) / TILE);
      hipLaunchKernelGGL(kernel_matrix_kernel, grid, block, 0, stream,
            x, k_out, n, dim, kind, gamma, coef0, degree);
}

// ── SMO gradient vector update ────────────────────────────────────────────────
// After picking working pair (i, j) on host, update:
//   grad[t] += delta_i * K[i,t] + delta_j * K[j,t]   for all t
// where delta_i = y[i] * (new_alpha_i - old_alpha_i),
//       delta_j = y[j] * (new_alpha_j - old_alpha_j)
// The gradient here is the dual objective gradient: G[t] = sum_s alpha[s]*y[s]*K[s,t] - 1
// We accumulate the change from the two updated alphas.

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

// ── KKT violation scoring ─────────────────────────────────────────────────────
// For working-set selection we score each sample by its KKT violation magnitude.
// F[t] = y[t] * G[t]  (the functional margin).
// For the "i" side (upper bound candidates): score_i[t] = -F[t]
//   valid when alpha[t] < C (if y[t]=+1) or alpha[t] > 0 (if y[t]=-1)
// For the "j" side (lower bound candidates): score_j[t] = +F[t]
//   valid when alpha[t] > 0 (if y[t]=+1) or alpha[t] < C (if y[t]=-1)
// We write per-sample scores; host picks argmax for i and argmax for j.

__global__ void smo_kkt_score_kernel(
      const double* grad,    // G[t]: gradient of dual objective
      const double* alpha,   // current alphas
      const double* y,       // labels in {-1,+1}
      double*       score_i, // upper-set violation: -y[t]*G[t], -inf if inactive
      double*       score_j, // lower-set violation: +y[t]*G[t], -inf if inactive
      int           n,
      double        C
) {
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= n) return;

      double yt    = y[t];
      double at    = alpha[t];
      double F_t   = yt * grad[t];

      // Upper bound set I_up: {t : (y=+1 && a<C) || (y=-1 && a>0)}
      bool in_up = (yt > 0.0) ? (at < C) : (at > 0.0);
      // Lower bound set I_down: {t : (y=+1 && a>0) || (y=-1 && a<C)}
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
      double        C,
      hipStream_t   stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_kkt_score_kernel, dim3(blocks), dim3(threads), 0, stream,
            grad, alpha, y, score_i, score_j, n, C);
}

// ── Matrix-free SMO support ──────────────────────────────────────────────────
// Compute one kernel-matrix ROW on demand: krow[t] = K(x_row, x_t) for all t.
// Lets SMO run without ever materializing the n×n matrix (item 5: no O(n²) alloc).
__global__ void smo_kernel_row_kernel(
      const double* x, double* krow, int n, int dim, int row,
      int kind, double gamma, double coef0, double degree
) {
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
      int kind, double gamma, double coef0, double degree, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_kernel_row_kernel, dim3(blocks), dim3(threads), 0, stream,
            x, krow, n, dim, row, kind, gamma, coef0, degree);
}

// Single-block argmax over s[n] → out[0]=max value, out[1]=index (as double).
// Keeps working-set selection on the GPU (item 3: no per-iter O(n) score download).
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

// Gradient update from two precomputed kernel rows (matrix-free variant):
//   grad[t] += di * krow_i[t] + dj * krow_j[t]
__global__ void smo_update_gradient_rows_kernel(
      double* grad, const double* ki, const double* kj, int n, double di, double dj
) {
      int t = blockIdx.x * blockDim.x + threadIdx.x;
      if (t >= n) return;
      grad[t] += di * ki[t] + dj * kj[t];
}

extern "C" void launch_smo_update_gradient_rows(
      double* grad, const double* ki, const double* kj, int n, double di, double dj, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(smo_update_gradient_rows_kernel, dim3(blocks), dim3(threads), 0, stream,
            grad, ki, kj, n, di, dj);
}
