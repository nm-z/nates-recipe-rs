#include <hip/hip_runtime.h>

#define TILE 16

// ── pairwise cosine ──────────────────────────────────────────────────────────
// query (nq x dim), train (nt x dim) → out (nq x nt), each entry = cosine similarity.
// Tiled: accumulates dot(q,t), ||q||², ||t||² in shared memory.
__global__ void pairwise_cosine_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim
) {
      int q = blockIdx.y * TILE + threadIdx.y;
      int t = blockIdx.x * TILE + threadIdx.x;

      __shared__ double sq[TILE][TILE];
      __shared__ double st[TILE][TILE];

      double dot  = 0.0;
      double norm_q = 0.0;
      double norm_t = 0.0;

      for (int tile_start = 0; tile_start < dim; tile_start += TILE) {
            int dq = tile_start + threadIdx.x;
            int dt = tile_start + threadIdx.y;

            double qv = (q < nq && dq < dim) ? query[q * dim + dq] : 0.0;
            double tv = (t < nt && dt < dim) ? train[t * dim + dt] : 0.0;

            sq[threadIdx.y][threadIdx.x] = qv;
            st[threadIdx.y][threadIdx.x] = tv;
            __syncthreads();

            int tile_end = min(TILE, dim - tile_start);
            for (int d = 0; d < tile_end; d++) {
                  double qd = sq[threadIdx.y][d];
                  double td = st[d][threadIdx.x];
                  dot    += qd * td;
                  norm_q += qd * qd;
                  norm_t += td * td;
            }
            __syncthreads();
      }

      if (q < nq && t < nt) {
            double denom = __dsqrt_rn(norm_q) * __dsqrt_rn(norm_t);
            out[q * nt + t] = (denom > 0.0) ? dot / denom : 0.0;
      }
}

extern "C" void launch_pairwise_cosine(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, hipStream_t stream
) {
      dim3 block(TILE, TILE);
      dim3 grid((nt + TILE - 1) / TILE, (nq + TILE - 1) / TILE);
      hipLaunchKernelGGL(pairwise_cosine_kernel, grid, block, 0, stream,
                         query, train, out, nq, nt, dim);
}

// ── pairwise L1 ──────────────────────────────────────────────────────────────
// query (nq x dim), train (nt x dim) → out (nq x nt), each entry = L1 distance.
// Tiled shared memory same pattern as L2.
__global__ void pairwise_l1_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim
) {
      int q = blockIdx.y * TILE + threadIdx.y;
      int t = blockIdx.x * TILE + threadIdx.x;

      __shared__ double sq[TILE][TILE];
      __shared__ double st[TILE][TILE];

      double sum = 0.0;

      for (int tile_start = 0; tile_start < dim; tile_start += TILE) {
            int dq = tile_start + threadIdx.x;
            int dt = tile_start + threadIdx.y;

            sq[threadIdx.y][threadIdx.x] = (q < nq && dq < dim) ? query[q * dim + dq] : 0.0;
            st[threadIdx.y][threadIdx.x] = (t < nt && dt < dim) ? train[t * dim + dt] : 0.0;
            __syncthreads();

            int tile_end = min(TILE, dim - tile_start);
            for (int d = 0; d < tile_end; d++) {
                  double diff = sq[threadIdx.y][d] - st[d][threadIdx.x];
                  sum += diff < 0.0 ? -diff : diff;
            }
            __syncthreads();
      }

      if (q < nq && t < nt)
            out[q * nt + t] = sum;
}

extern "C" void launch_pairwise_l1(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, hipStream_t stream
) {
      dim3 block(TILE, TILE);
      dim3 grid((nt + TILE - 1) / TILE, (nq + TILE - 1) / TILE);
      hipLaunchKernelGGL(pairwise_l1_kernel, grid, block, 0, stream,
                         query, train, out, nq, nt, dim);
}

// ── pairwise Hamming ─────────────────────────────────────────────────────────
// query_u8 (nq x dim), train_u8 (nt x dim) → out (nq x nt) as f64.
// Each entry = fraction of differing positions (Hamming distance / dim).
// Inputs are uint8 (e.g. quantized bins or binary features).
// Tiled with uint8 shared memory to maximise occupancy.
__global__ void pairwise_hamming_kernel(
      const unsigned char* query, const unsigned char* train, double* out,
      int nq, int nt, int dim
) {
      int q = blockIdx.y * TILE + threadIdx.y;
      int t = blockIdx.x * TILE + threadIdx.x;

      __shared__ unsigned char sq[TILE][TILE];
      __shared__ unsigned char st[TILE][TILE];

      int mismatches = 0;

      for (int tile_start = 0; tile_start < dim; tile_start += TILE) {
            int dq = tile_start + threadIdx.x;
            int dt = tile_start + threadIdx.y;

            sq[threadIdx.y][threadIdx.x] = (q < nq && dq < dim) ? query[q * dim + dq] : 0;
            st[threadIdx.y][threadIdx.x] = (t < nt && dt < dim) ? train[t * dim + dt] : 0;
            __syncthreads();

            int tile_end = min(TILE, dim - tile_start);
            for (int d = 0; d < tile_end; d++) {
                  mismatches += (sq[threadIdx.y][d] != st[d][threadIdx.x]) ? 1 : 0;
            }
            __syncthreads();
      }

      if (q < nq && t < nt)
            out[q * nt + t] = (dim > 0) ? (double)mismatches / (double)dim : 0.0;
}

extern "C" void launch_pairwise_hamming(
      const unsigned char* query, const unsigned char* train, double* out,
      int nq, int nt, int dim, hipStream_t stream
) {
      dim3 block(TILE, TILE);
      dim3 grid((nt + TILE - 1) / TILE, (nq + TILE - 1) / TILE);
      hipLaunchKernelGGL(pairwise_hamming_kernel, grid, block, 0, stream,
                         query, train, out, nq, nt, dim);
}
