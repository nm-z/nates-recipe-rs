#include <hip/hip_runtime.h>

// ── itemset_support ────────────────────────────────────────────────────────
// Count how many transactions contain each candidate itemset.
// transactions: (n_trans, n_items) binary matrix (1.0 = item present)
// candidates:   (n_cands, k) — each row is a k-itemset (item indices as doubles)
// counts:       (n_cands,) — output support count per candidate
//
// For k=2 this is equivalent to gemm(T,T,"T","N") on the binary matrix,
// but for k≥3 there's no matrix formulation — must check all k items.
// One thread per (transaction, candidate) pair.

__global__ void itemset_support_kernel(
    const double* trans, const double* cands, double* counts,
    int n_trans, int n_items, int n_cands, int k
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_trans * n_cands) return;
    int t = idx / n_cands;  // transaction
    int c = idx % n_cands;  // candidate

    // Check if transaction t contains all k items in candidate c
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
    hipMemsetAsync(counts, 0, (size_t)n_cands * sizeof(double), stream);
    int total = n_trans * n_cands;
    dim3 bl(256); dim3 gr((total + 255) / 256);
    hipLaunchKernelGGL(itemset_support_kernel, gr, bl, 0, stream,
        trans, cands, counts, n_trans, n_items, n_cands, k);
}

// ── candidate_generate ─────────────────────────────────────────────────────
// Apriori candidate generation: join frequent k-itemsets to produce (k+1)-candidates.
// Frequent itemsets must be sorted in each row (ascending item indices).
//
// Join rule: two k-itemsets that share the first (k-1) items produce a (k+1)-candidate.
// Prune: skip if any (k)-subset of the candidate is not in the frequent set.
//        For simplicity, this kernel does the join only. Pruning via support check after.
//
// One thread per (i, j) pair where j > i. If freq[i][0..k-2] == freq[j][0..k-2]
// and freq[i][k-1] < freq[j][k-1], emit candidate = freq[i] ∪ {freq[j][k-1]}.
//
// Two-pass: pass 1 counts how many candidates (atomic), pass 2 writes them.
// We expose both passes as one function — caller provides pre-allocated output.

__global__ void candidate_generate_count_kernel(
    const double* freq, int n_freq, int k, int* count
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int n_pairs = n_freq * (n_freq - 1) / 2;
    if (idx >= n_pairs) return;

    // Map linear idx to (i, j) with j > i
    // i = n - 2 - floor(sqrt(-8*idx + 4*n*(n-1) - 7) / 2.0 - 0.5)
    // Simpler: iterate
    int i = 0, j = 0;
    int rem = idx;
    for (i = 0; i < n_freq - 1; i++) {
        int span = n_freq - 1 - i;
        if (rem < span) { j = i + 1 + rem; break; }
        rem -= span;
    }

    const double* a = freq + i * k;
    const double* b = freq + j * k;

    // Check first k-1 items match
    for (int p = 0; p < k - 1; p++) {
        if ((int)a[p] != (int)b[p]) return;
    }
    // Last items must differ (a[k-1] < b[k-1] since sorted)
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

// Returns the number of candidates generated (host int).
// Caller must provide output buffer large enough (worst case: n_freq*(n_freq-1)/2 rows of k+1).
extern "C" int launch_candidate_generate(
    const double* freq, double* out,
    int n_freq, int k, hipStream_t stream
) {
    int n_pairs = n_freq * (n_freq - 1) / 2;
    if (n_pairs == 0) return 0;

    // Count pass
    int* d_count = nullptr;
    hipMalloc(&d_count, sizeof(int));
    hipMemsetAsync(d_count, 0, sizeof(int), stream);
    dim3 bl(256); dim3 gr((n_pairs + 255) / 256);
    hipLaunchKernelGGL(candidate_generate_count_kernel, gr, bl, 0, stream,
        freq, n_freq, k, d_count);

    int h_count = 0;
    hipMemcpyAsync(&h_count, d_count, sizeof(int), hipMemcpyDeviceToHost, stream);
    hipStreamSynchronize(stream);

    if (h_count == 0) { hipFree(d_count); return 0; }

    // Write pass — reuse d_count as write position (reset to 0)
    hipMemsetAsync(d_count, 0, sizeof(int), stream);
    hipLaunchKernelGGL(candidate_generate_write_kernel, gr, bl, 0, stream,
        freq, out, n_freq, k, d_count);
    hipStreamSynchronize(stream);
    hipFree(d_count);
    return h_count;
}
