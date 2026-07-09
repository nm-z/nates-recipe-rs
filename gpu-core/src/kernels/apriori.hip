#include <hip/hip_runtime.h>


__global__ void itemset_support_kernel(
    const double* trans, const double* cands, double* counts,
    int n_trans, int n_items, int n_cands, int k
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_trans * n_cands) return;
    int t = idx / n_cands;
    int c = idx % n_cands;

    const double* cand_row = cands + c * k;
    int present = 1;
    for (int ki = 0; ki < k; ki++) {
        int item = (int)cand_row[ki];
        if (item < 0 || item >= n_items || trans[t * n_items + item] == 0.0) {
            present = 0;
            break;
        }
    }
    if (present) atomicAdd(&counts[c], 1.0);
}

extern "C" void launch_itemset_support(
    const double* trans, const double* cands, double* counts,
    int n_trans, int n_items, int n_cands, int k, hipStream_t stream
) {
    int total = n_trans * n_cands;
    dim3 bl(256); dim3 gr((total + 255) / 256);
    hipLaunchKernelGGL(itemset_support_kernel, gr, bl, 0, stream,
        trans, cands, counts, n_trans, n_items, n_cands, k);
}


__global__ void candidate_generate_count_kernel(
    const double* freq, int n_freq, int k, int* count
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int n_pairs = n_freq * (n_freq - 1) / 2;
    if (idx >= n_pairs) return;

    int i = 0, j = 0;
    int rem = idx;
    for (i = 0; i < n_freq - 1; i++) {
        int span = n_freq - 1 - i;
        if (rem < span) { j = i + 1 + rem; break; }
        rem -= span;
    }

    const double* a = freq + i * k;
    const double* b = freq + j * k;

    for (int p = 0; p < k - 1; p++) {
        if ((int)a[p] != (int)b[p]) return;
    }
    if ((int)a[k-1] >= (int)b[k-1]) return;
    atomicAdd(count, 1);
}

__global__ void candidate_generate_write_kernel(
    const double* freq, double* out, int n_freq, int k, int* write_pos
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int n_pairs = n_freq * (n_freq - 1) / 2;
    if (idx >= n_pairs) return;

    int i = 0, j = 0;
    int rem = idx;
    for (i = 0; i < n_freq - 1; i++) {
        int span = n_freq - 1 - i;
        if (rem < span) { j = i + 1 + rem; break; }
        rem -= span;
    }

    const double* a = freq + i * k;
    const double* b = freq + j * k;

    for (int p = 0; p < k - 1; p++) {
        if ((int)a[p] != (int)b[p]) return;
    }
    if ((int)a[k-1] >= (int)b[k-1]) return;

    int pos = atomicAdd(write_pos, 1);
    double* row = out + pos * (k + 1);
    for (int p = 0; p < k; p++) row[p] = a[p];
    row[k] = b[k - 1];
}

extern "C" void launch_candidate_generate_count(
    const double* freq, int n_freq, int k, int* count, hipStream_t stream
) {
    int n_pairs = n_freq * (n_freq - 1) / 2;
    if (n_pairs == 0) return;
    dim3 bl(256); dim3 gr((n_pairs + 255) / 256);
    hipLaunchKernelGGL(candidate_generate_count_kernel, gr, bl, 0, stream,
        freq, n_freq, k, count);
}

extern "C" void launch_candidate_generate_write(
    const double* freq, double* out, int n_freq, int k, int* write_pos, hipStream_t stream
) {
    int n_pairs = n_freq * (n_freq - 1) / 2;
    if (n_pairs == 0) return;
    dim3 bl(256); dim3 gr((n_pairs + 255) / 256);
    hipLaunchKernelGGL(candidate_generate_write_kernel, gr, bl, 0, stream,
        freq, out, n_freq, k, write_pos);
}
