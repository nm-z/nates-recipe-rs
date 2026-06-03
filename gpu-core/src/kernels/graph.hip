#include <hip/hip_runtime.h>

// CSR SpMV: y[i] = sum_{j in row i} values[k] * x[col_idx[k]]
// One thread per row.
extern "C" __global__ void csr_spmv_kernel(
    const double* values, const int* col_idx, const int* row_ptr,
    const double* x, double* y, int n_rows
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    double acc = 0.0;
    int start = row_ptr[row];
    int end   = row_ptr[row + 1];
    for (int k = start; k < end; k++) {
        acc += values[k] * x[col_idx[k]];
    }
    y[row] = acc;
}

extern "C" void launch_csr_spmv(
    const double* values, const int* col_idx, const int* row_ptr,
    const double* x, double* y, int n_rows, hipStream_t stream
) {
    int threads = 256;
    int blocks  = (n_rows + threads - 1) / threads;
    hipLaunchKernelGGL(csr_spmv_kernel, dim3(blocks), dim3(threads), 0, stream,
        values, col_idx, row_ptr, x, y, n_rows);
}

// CSR SpMM: C[i, f] = sum_{j in row i} values[k] * B[col_idx[k], f]
// Grid: (ceil(feat/TILE_F), ceil(n_rows/TILE_R)); block: (TILE_F, TILE_R).
#define TILE_F 16
#define TILE_R 16

extern "C" __global__ void csr_spmm_kernel(
    const double* values, const int* col_idx, const int* row_ptr,
    const double* B, double* C, int n_rows, int feat
) {
    int row = blockIdx.y * TILE_R + threadIdx.y;
    int f   = blockIdx.x * TILE_F + threadIdx.x;
    if (row >= n_rows || f >= feat) return;
    double acc = 0.0;
    int start = row_ptr[row];
    int end   = row_ptr[row + 1];
    for (int k = start; k < end; k++) {
        acc += values[k] * B[col_idx[k] * feat + f];
    }
    C[row * feat + f] = acc;
}

extern "C" void launch_csr_spmm(
    const double* values, const int* col_idx, const int* row_ptr,
    const double* B, double* C, int n_rows, int feat, hipStream_t stream
) {
    dim3 block(TILE_F, TILE_R);
    dim3 grid((feat + TILE_F - 1) / TILE_F, (n_rows + TILE_R - 1) / TILE_R);
    hipLaunchKernelGGL(csr_spmm_kernel, grid, block, 0, stream,
        values, col_idx, row_ptr, B, C, n_rows, feat);
}

// Neighbor aggregation via atomicAdd scatter: for each edge (src->dst), add features[src] to agg[dst].
// If mean=1, divide by in-degree at the end (second kernel).
extern "C" __global__ void neighbor_agg_scatter_kernel(
    const double* features, const int* edge_src, const int* edge_dst,
    double* agg, int feat, int n_edges
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_edges * feat) return;
    int e = idx / feat;
    int f = idx % feat;
    int src = edge_src[e];
    int dst = edge_dst[e];
    atomicAdd(&agg[dst * feat + f], features[src * feat + f]);
}

// Divide each node's aggregated features by its in-degree.
extern "C" __global__ void neighbor_agg_mean_kernel(
    double* agg, const double* deg, int n_nodes, int feat
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_nodes * feat) return;
    int node = idx / feat;
    double d = deg[node];
    if (d > 0.0) agg[idx] /= d;
}

extern "C" void launch_neighbor_aggregate(
    const double* features, const int* edge_src, const int* edge_dst,
    double* agg, const double* deg,
    int n_nodes, int feat, int n_edges, int mean_flag, hipStream_t stream
) {
    int threads = 256;
    int total   = n_edges * feat;
    int blocks  = (total + threads - 1) / threads;
    hipLaunchKernelGGL(neighbor_agg_scatter_kernel, dim3(blocks), dim3(threads), 0, stream,
        features, edge_src, edge_dst, agg, feat, n_edges);
    if (mean_flag) {
        int total2 = n_nodes * feat;
        int blocks2 = (total2 + threads - 1) / threads;
        hipLaunchKernelGGL(neighbor_agg_mean_kernel, dim3(blocks2), dim3(threads), 0, stream,
            agg, deg, n_nodes, feat);
    }
}

// Compute in-degree as f64: deg[dst] += 1 for each edge.
extern "C" __global__ void degree_kernel(
    const int* edge_dst, double* deg, int n_edges
) {
    int e = blockIdx.x * blockDim.x + threadIdx.x;
    if (e >= n_edges) return;
    atomicAdd(&deg[edge_dst[e]], 1.0);
}

extern "C" void launch_degree(
    const int* edge_dst, double* deg, int n_edges, hipStream_t stream
) {
    int threads = 256;
    int blocks  = (n_edges + threads - 1) / threads;
    hipLaunchKernelGGL(degree_kernel, dim3(blocks), dim3(threads), 0, stream,
        edge_dst, deg, n_edges);
}

// GCN normalization: scale features[node, f] by deg[node]^{-1/2}.
// Nodes with deg=0 are left unchanged.
extern "C" __global__ void gcn_norm_kernel(
    double* features, const double* deg, int n_nodes, int feat
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_nodes * feat) return;
    int node = idx / feat;
    double d = deg[node];
    if (d > 0.0) features[idx] *= 1.0 / __builtin_sqrt(d);
}

extern "C" void launch_gcn_norm(
    double* features, const double* deg, int n_nodes, int feat, hipStream_t stream
) {
    int threads = 256;
    int total   = n_nodes * feat;
    int blocks  = (total + threads - 1) / threads;
    hipLaunchKernelGGL(gcn_norm_kernel, dim3(blocks), dim3(threads), 0, stream,
        features, deg, n_nodes, feat);
}
