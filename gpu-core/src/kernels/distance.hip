#include <hip/hip_runtime.h>

#define TILE 16

// Tiled pairwise L2: query (nq x dim), train (nt x dim) → out (nq x nt)
// Each block computes a TILE x TILE sub-matrix of distances.
// Shared memory tiles reduce global memory reads from O(dim) to O(dim/TILE) per thread.
extern "C" __global__ void pairwise_l2_kernel(
    const double* query, const double* train, double* out,
    int n_query, int n_train, int dim
) {
    int q = blockIdx.y * TILE + threadIdx.y;
    int t = blockIdx.x * TILE + threadIdx.x;

    __shared__ double sq[TILE][TILE];
    __shared__ double st[TILE][TILE];

    double sum = 0.0;

    for (int tile_start = 0; tile_start < dim; tile_start += TILE) {
        // Load query tile: sq[ty][d] = query[q, tile_start + tx]
        int d_q = tile_start + threadIdx.x;
        if (q < n_query && d_q < dim)
            sq[threadIdx.y][threadIdx.x] = query[q * dim + d_q];
        else
            sq[threadIdx.y][threadIdx.x] = 0.0;

        // Load train tile: st[ty][d] = train[t, tile_start + ty]
        // Transposed load: threadIdx.y indexes the dim offset
        int d_t = tile_start + threadIdx.y;
        if (t < n_train && d_t < dim)
            st[threadIdx.y][threadIdx.x] = train[t * dim + d_t];
        else
            st[threadIdx.y][threadIdx.x] = 0.0;

        __syncthreads();

        // Accumulate squared differences for this tile
        int tile_end = min(TILE, dim - tile_start);
        for (int d = 0; d < tile_end; d++) {
            double diff = sq[threadIdx.y][d] - st[d][threadIdx.x];
            sum += diff * diff;
        }

        __syncthreads();
    }

    if (q < n_query && t < n_train) {
        out[q * n_train + t] = sum;  // squared L2, no sqrt (callers use squared distances)
    }
}

extern "C" void launch_pairwise_l2(
    const double* query, const double* train, double* out,
    int n_query, int n_train, int dim, hipStream_t stream
) {
    dim3 block(TILE, TILE);
    dim3 grid((n_train + TILE - 1) / TILE, (n_query + TILE - 1) / TILE);
    hipLaunchKernelGGL(pairwise_l2_kernel, grid, block, 0, stream, query, train, out, n_query, n_train, dim);
}
