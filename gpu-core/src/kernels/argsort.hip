#include <hip/hip_runtime.h>
#include <hipcub/hipcub.hpp>

// Fill indices 0..n-1 before sorting
__global__ void init_indices_kernel(int* indices, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) indices[i] = i;
}

// Workspace size (bytes) for the hipCUB radix sort over n pairs.
// Caller queries this, allocates the temp buffer, and passes it to launch_partial_argsort.
extern "C" size_t partial_argsort_workspace_bytes(int n) {
    size_t temp_bytes = 0;
    hipcub::DeviceRadixSort::SortPairs(
        nullptr, temp_bytes,
        (const double*)nullptr, (double*)nullptr,
        (const int*)nullptr, (int*)nullptr,
        n, 0, sizeof(double) * 8);
    return temp_bytes;
}

extern "C" void launch_partial_argsort(
    const double* data, int* indices,
    double* keys_out, int* vals_in, void* temp, size_t temp_bytes,
    int n, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(init_indices_kernel, grid, block, 0, stream, vals_in, n);

    // Sort ascending (smallest first = nearest neighbors).
    // data is the const key input; sorted value-indices land directly in `indices`.
    hipcub::DeviceRadixSort::SortPairs(
        temp, temp_bytes,
        data, keys_out,
        vals_in, indices,
        n, 0, sizeof(double) * 8, stream);
}

// Per-row top-k: for each row of (rows x cols) distance matrix,
// find the k column indices with smallest values.
// Output: (rows x k) int array of indices.
// One thread per row, insertion sort into k-sized buffer (k is small, typically ≤30).
__global__ void topk_per_row_kernel(const double* dists, int* out_indices, int rows, int cols, int k) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= rows) return;

    const double* r = dists + row * cols;
    int* out = out_indices + row * k;

    // Init with first k elements
    double vals[64]; // max k=64, stack allocated
    int actual_k = k < 64 ? k : 64;
    for (int i = 0; i < actual_k; i++) {
        vals[i] = r[i];
        out[i] = i;
    }
    // Insertion sort initial k
    for (int i = 1; i < actual_k; i++) {
        double v = vals[i];
        int idx = out[i];
        int j = i - 1;
        while (j >= 0 && vals[j] > v) {
            vals[j + 1] = vals[j];
            out[j + 1] = out[j];
            j--;
        }
        vals[j + 1] = v;
        out[j + 1] = idx;
    }
    // Scan remaining elements, replace max if smaller
    for (int i = actual_k; i < cols; i++) {
        if (r[i] < vals[actual_k - 1]) {
            vals[actual_k - 1] = r[i];
            out[actual_k - 1] = i;
            // Bubble down
            int j = actual_k - 2;
            while (j >= 0 && vals[j] > vals[j + 1]) {
                double tv = vals[j]; vals[j] = vals[j + 1]; vals[j + 1] = tv;
                int ti = out[j]; out[j] = out[j + 1]; out[j + 1] = ti;
                j--;
            }
        }
    }
}

extern "C" void launch_topk_per_row(const double* dists, int* out_indices, int rows, int cols, int k, hipStream_t stream) {
    dim3 block(256);
    dim3 grid((rows + 255) / 256);
    hipLaunchKernelGGL(topk_per_row_kernel, grid, block, 0, stream, dists, out_indices, rows, cols, k);
}
