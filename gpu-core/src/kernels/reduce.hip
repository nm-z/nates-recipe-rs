#include <cstring>
#include <hip/hip_runtime.h>
#include <rocprim/rocprim.hpp>

// Column-transposed view: flat index i in [col*rows, (col+1)*rows) maps to x[(i%rows)*cols+(i/rows)].
// This lets device_segmented_reduce treat each column as a contiguous segment of length rows.
struct ColTransposeOp {
    const double* x;
    int rows;
    int cols;
    __host__ __device__ double operator()(int i) const {
        return x[(i % rows) * cols + (i / rows)];
    }
};

struct ScaleByRows {
    int rows;
    __host__ __device__ int operator()(int c) const { return c * rows; }
};

struct DivByRows {
    double inv_rows;
    __host__ __device__ double operator()(double v) const { return v * inv_rows; }
};

extern "C" size_t reduce_sum_cols_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColTransposeOp{x, rows, cols});
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});

    size_t tmp_bytes = 0;
    rocprim::segmented_reduce(nullptr, tmp_bytes,
        input_it, (double*)nullptr, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::plus<double>(), 0.0,
        stream);
    return tmp_bytes;
}

extern "C" void launch_reduce_sum_cols(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColTransposeOp{x, rows, cols});

    // begin_offsets[c] = c*rows, end_offsets[c] = (c+1)*rows
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});

    rocprim::segmented_reduce(tmp, tmp_bytes,
        input_it, out, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::plus<double>(), 0.0,
        stream);
}

extern "C" size_t reduce_sum_rows_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{cols});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{cols});

    size_t tmp_bytes = 0;
    rocprim::segmented_reduce(nullptr, tmp_bytes,
        x, (double*)nullptr, (unsigned int)rows,
        begin_offsets, end_offsets,
        rocprim::plus<double>(), 0.0,
        stream);
    return tmp_bytes;
}

extern "C" void launch_reduce_sum_rows(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    // Row-major: each row is contiguous, segment i = [i*cols, (i+1)*cols)
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{cols});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{cols});

    rocprim::segmented_reduce(tmp, tmp_bytes,
        x, out, (unsigned int)rows,
        begin_offsets, end_offsets,
        rocprim::plus<double>(), 0.0,
        stream);
}

extern "C" size_t reduce_mean_cols_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    return reduce_sum_cols_workspace_bytes(x, rows, cols, stream);
}

extern "C" void launch_reduce_mean_cols(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    launch_reduce_sum_cols(x, out, rows, cols, tmp, tmp_bytes, stream);
    rocprim::transform(out, out, (size_t)cols, DivByRows{1.0 / rows}, stream);
}

// Squared-deviation view for variance: index i (column-transposed, like
// ColTransposeOp) → (x[row,col] - means[col])². Feeding this through the same
// device_segmented_reduce that powers launch_reduce_sum_cols gives a fully
// parallel variance pass (one segment per column, fixed-order → deterministic),
// replacing the old one-thread-per-column serial scan.
struct ColDevSqOp {
    const double* x;
    const double* means;
    int rows;
    int cols;
    __host__ __device__ double operator()(int i) const {
        int col = i / rows;
        double d = x[(i % rows) * cols + col] - means[col];
        return d * d;
    }
};

// Workspace = the segmented-reduce scratch + a column-mean buffer carved from the
// 256B-aligned tail, so variance stays allocate-once (no per-call hipMalloc).
extern "C" size_t reduce_var_cols_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    size_t seg = reduce_sum_cols_workspace_bytes(x, rows, cols, stream);
    size_t off = (seg + 255) & ~((size_t)255);
    return off + (size_t)cols * sizeof(double);
}

extern "C" void launch_reduce_var_cols(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    size_t seg = reduce_sum_cols_workspace_bytes(x, rows, cols, stream);
    size_t off = (seg + 255) & ~((size_t)255);
    double* means = (double*)((char*)tmp + off);
    // Pass 1: column means (parallel segmented sum / rows).
    launch_reduce_sum_cols(x, means, rows, cols, tmp, seg, stream);
    rocprim::transform(means, means, (size_t)cols, DivByRows{1.0 / rows}, stream);
    // Pass 2: column variance = mean of squared deviations, same parallel reduce.
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColDevSqOp{x, means, rows, cols});
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});
    rocprim::segmented_reduce(tmp, seg,
        input_it, out, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::plus<double>(), 0.0, stream);
    rocprim::transform(out, out, (size_t)cols, DivByRows{1.0 / rows}, stream);
}

// Argmin per row with warp shuffle reduction.
// One warp (64 threads on AMD) cooperates per row. Each thread handles cols/warpSize elements,
// then warp shuffle reduces to find global min.
__global__ void argmin_rows_kernel(const double* dists, int* assignments, int rows, int cols) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = dists + row * cols;

    // Each thread in the block scans a strided subset of columns
    double local_min = 1e308;
    int local_idx = 0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) {
        if (r[j] < local_min) { local_min = r[j]; local_idx = j; }
    }

    // Warp shuffle reduction for min
    for (int offset = warpSize / 2; offset > 0; offset /= 2) {
        double other_val = __shfl_down(local_min, offset);
        int other_idx = __shfl_down(local_idx, offset);
        if (other_val < local_min) { local_min = other_val; local_idx = other_idx; }
    }

    // Cross-warp reduction via shared memory
    __shared__ double s_val[16];
    __shared__ int s_idx[16];
    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    if (lane == 0) { s_val[wid] = local_min; s_idx[wid] = local_idx; }
    __syncthreads();

    if (threadIdx.x == 0) {
        double best = s_val[0];
        int best_i = s_idx[0];
        int n_warps = (blockDim.x + warpSize - 1) / warpSize;
        for (int w = 1; w < n_warps; w++) {
            if (s_val[w] < best) { best = s_val[w]; best_i = s_idx[w]; }
        }
        assignments[row] = best_i;
    }
}

extern "C" void launch_argmin_rows(const double* dists, int* assignments, int rows, int cols, hipStream_t stream) {
    int threads = min(256, max(64, cols));
    hipLaunchKernelGGL(argmin_rows_kernel, dim3(rows), dim3(threads), 0, stream, dists, assignments, rows, cols);
}

// Centroid update: scatter-add approach.
// Phase 1: zero centroids + counts.
// Phase 2: each thread handles a chunk of points, atomically adds to centroid sums.
// Phase 3: divide by counts.
// Much better than scanning all n per cluster — each point visited exactly once.

__global__ void centroid_zero_kernel(double* centroids, int* counts, int k, int dim) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < k * dim) centroids[i] = 0.0;
    if (i < k) counts[i] = 0;
}

__global__ void centroid_scatter_add_kernel(const double* x, const int* assignments, double* centroids, int* counts, int n, int dim) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int c = assignments[i];
    atomicAdd(&counts[c], 1);
    for (int d = 0; d < dim; d++) {
        atomicAdd(&centroids[c * dim + d], x[i * dim + d]);
    }
}

__global__ void centroid_divide_kernel(double* centroids, const int* counts, int k, int dim) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= k * dim) return;
    int c = idx / dim;
    int cnt = counts[c];
    if (cnt > 0) centroids[idx] /= cnt;
}

// 1D average pool: input (n * out_len, n_filters) → output (n, n_filters)
// Each output[i, fi] = mean(input[i*out_len .. (i+1)*out_len, fi])
__global__ void avg_pool_1d_kernel(const double* input, double* output, int n, int out_len, int n_filters) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = n * n_filters;
    if (idx >= total) return;
    int i = idx / n_filters;
    int fi = idx % n_filters;
    double sum = 0.0;
    for (int t = 0; t < out_len; t++) {
        sum += input[(i * out_len + t) * n_filters + fi];
    }
    output[i * n_filters + fi] = sum / out_len;
}

// Pool gradient expand: replicate (n x n_filters) gradient back to (n*out_len x n_filters), divide by out_len
__global__ void pool_grad_expand_kernel(const double* grad_pool, double* grad_out, int n, int out_len, int n_filters) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = n * out_len * n_filters;
    if (idx >= total) return;
    int i = idx / (out_len * n_filters);
    int fi = idx % n_filters;
    grad_out[idx] = grad_pool[i * n_filters + fi] / out_len;
}

extern "C" void launch_avg_pool_1d(const double* input, double* output, int n, int out_len, int n_filters, hipStream_t stream) {
    int total = n * n_filters;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(avg_pool_1d_kernel, grid, block, 0, stream, input, output, n, out_len, n_filters);
}

extern "C" void launch_pool_grad_expand(const double* grad_pool, double* grad_out, int n, int out_len, int n_filters, hipStream_t stream) {
    int total = n * out_len * n_filters;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(pool_grad_expand_kernel, grid, block, 0, stream, grad_pool, grad_out, n, out_len, n_filters);
}

extern "C" void launch_centroid_update(const double* x, const int* assignments, double* centroids, int* counts, int n, int dim, int k, hipStream_t stream) {
    int kd = k * dim;
    dim3 b1(256); dim3 g1((max(kd, k) + 255) / 256);
    hipLaunchKernelGGL(centroid_zero_kernel, g1, b1, 0, stream, centroids, counts, k, dim);

    dim3 b2(256); dim3 g2((n + 255) / 256);
    hipLaunchKernelGGL(centroid_scatter_add_kernel, g2, b2, 0, stream, x, assignments, centroids, counts, n, dim);

    dim3 b3(256); dim3 g3((kd + 255) / 256);
    hipLaunchKernelGGL(centroid_divide_kernel, g3, b3, 0, stream, centroids, counts, k, dim);
}

// ── Reduce max/min ─────────────────────────────────────────────────────────
// Uses rocprim::segmented_reduce with maximum/minimum operators.
// Row reductions: contiguous segments of length cols.
// Column reductions: virtual transposed segments via ColTransposeOp.

extern "C" size_t reduce_max_rows_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{cols});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{cols});

    size_t tmp_bytes = 0;
    rocprim::segmented_reduce(nullptr, tmp_bytes,
        x, (double*)nullptr, (unsigned int)rows,
        begin_offsets, end_offsets,
        rocprim::maximum<double>(), -1e308, stream);
    return tmp_bytes;
}

extern "C" void launch_reduce_max_rows(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{cols});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{cols});

    rocprim::segmented_reduce(tmp, tmp_bytes,
        x, out, (unsigned int)rows,
        begin_offsets, end_offsets,
        rocprim::maximum<double>(), -1e308, stream);
}

extern "C" size_t reduce_max_cols_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColTransposeOp{x, rows, cols});
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});

    size_t tmp_bytes = 0;
    rocprim::segmented_reduce(nullptr, tmp_bytes,
        input_it, (double*)nullptr, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::maximum<double>(), -1e308, stream);
    return tmp_bytes;
}

extern "C" void launch_reduce_max_cols(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColTransposeOp{x, rows, cols});
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});

    rocprim::segmented_reduce(tmp, tmp_bytes,
        input_it, out, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::maximum<double>(), -1e308, stream);
}

extern "C" size_t reduce_min_rows_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{cols});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{cols});

    size_t tmp_bytes = 0;
    rocprim::segmented_reduce(nullptr, tmp_bytes,
        x, (double*)nullptr, (unsigned int)rows,
        begin_offsets, end_offsets,
        rocprim::minimum<double>(), 1e308, stream);
    return tmp_bytes;
}

extern "C" void launch_reduce_min_rows(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{cols});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{cols});

    rocprim::segmented_reduce(tmp, tmp_bytes,
        x, out, (unsigned int)rows,
        begin_offsets, end_offsets,
        rocprim::minimum<double>(), 1e308, stream);
}

extern "C" size_t reduce_min_cols_workspace_bytes(const double* x, int rows, int cols, hipStream_t stream) {
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColTransposeOp{x, rows, cols});
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});

    size_t tmp_bytes = 0;
    rocprim::segmented_reduce(nullptr, tmp_bytes,
        input_it, (double*)nullptr, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::minimum<double>(), 1e308, stream);
    return tmp_bytes;
}

extern "C" void launch_reduce_min_cols(const double* x, double* out, int rows, int cols, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    auto input_it = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0),
        ColTransposeOp{x, rows, cols});
    auto begin_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(0), ScaleByRows{rows});
    auto end_offsets = rocprim::make_transform_iterator(
        rocprim::make_counting_iterator<int>(1), ScaleByRows{rows});

    rocprim::segmented_reduce(tmp, tmp_bytes,
        input_it, out, (unsigned int)cols,
        begin_offsets, end_offsets,
        rocprim::minimum<double>(), 1e308, stream);
}

// ── Log-sum-exp per row ────────────────────────────────────────────────────
// out[i] = max_j(x[i,j]) + log(sum_j(exp(x[i,j] - max_j(x[i,j]))))
// Cooperative row-wise: one block per row.

extern "C" __global__ void log_sum_exp_rows_kernel(const double* x, double* out, int rows, int cols) {
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = x + row * cols;

    // Find max
    double lmax = -1e308;
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        if (r[j] > lmax) lmax = r[j];
    for (int off = warpSize/2; off > 0; off /= 2) { double o = __shfl_down(lmax, off); if (o > lmax) lmax = o; }
    __shared__ double sm[32];
    int lane = threadIdx.x % warpSize, wid = threadIdx.x / warpSize;
    if (lane == 0) sm[wid] = lmax;
    __syncthreads();
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;
    if (wid == 0) {
        lmax = (lane < nwarps) ? sm[lane] : -1e308;
        for (int off = warpSize/2; off > 0; off /= 2) { double o = __shfl_down(lmax, off); if (o > lmax) lmax = o; }
        if (lane == 0) sm[0] = lmax;
    }
    __syncthreads();
    double row_max = sm[0];

    // Sum exp(x - max)
    double lsum = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) lsum += exp(r[j] - row_max);
    for (int off = warpSize/2; off > 0; off /= 2) lsum += __shfl_down(lsum, off);
    __shared__ double ss[32];
    if (lane == 0) ss[wid] = lsum;
    __syncthreads();
    if (wid == 0) {
        lsum = (lane < nwarps) ? ss[lane] : 0.0;
        for (int off = warpSize/2; off > 0; off /= 2) lsum += __shfl_down(lsum, off);
        if (lane == 0) ss[0] = lsum;
    }
    __syncthreads();

    if (threadIdx.x == 0) out[row] = row_max + log(ss[0]);
}

// ── Prefix sum (parallel scan) ─────────────────────────────────────────────

extern "C" size_t prefix_sum_inclusive_workspace_bytes(const double* x, int n, hipStream_t stream) {
    size_t tmp_bytes = 0;
    rocprim::inclusive_scan(nullptr, tmp_bytes, x, (double*)nullptr, n, rocprim::plus<double>(), stream);
    return tmp_bytes;
}

extern "C" void launch_prefix_sum_inclusive(const double* x, double* out, int n, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    rocprim::inclusive_scan(tmp, tmp_bytes, x, out, n, rocprim::plus<double>(), stream);
}

extern "C" size_t prefix_sum_exclusive_workspace_bytes(const double* x, int n, hipStream_t stream) {
    size_t tmp_bytes = 0;
    rocprim::exclusive_scan(nullptr, tmp_bytes, x, (double*)nullptr, 0.0, (size_t)n, rocprim::plus<double>(), stream);
    return tmp_bytes;
}

extern "C" void launch_prefix_sum_exclusive(const double* x, double* out, int n, void* tmp, size_t tmp_bytes, hipStream_t stream) {
    rocprim::exclusive_scan(tmp, tmp_bytes, x, out, 0.0, (size_t)n, rocprim::plus<double>(), stream);
}

extern "C" void launch_log_sum_exp_rows(const double* x, double* out, int rows, int cols, hipStream_t stream) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(log_sum_exp_rows_kernel, dim3(rows), dim3(threads), 0, stream, x, out, rows, cols);
}
