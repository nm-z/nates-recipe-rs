#include <hip/hip_runtime.h>

// ── histogram_build ────────────────────────────────────────────────────────
// For each (sample, feature) pair where mask is active, accumulate gradient,
// hessian, and count into the bin indicated by bins[sample, feature].
// bins stored as doubles, cast to int. Histograms are (p, n_bins) each.

__global__ void histogram_build_kernel(
    const double* bins, const double* grad, const double* hess, const double* mask,
    double* grad_hist, double* hess_hist, double* count_hist,
    int n, int p, int n_bins
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * p) return;
    int i = idx / p;
    int f = idx % p;
    if (mask[i] == 0.0) return;
    int b = (int)bins[i * p + f];
    if (b < 0 || b >= n_bins) return;
    int h = f * n_bins + b;
    atomicAdd(&grad_hist[h], grad[i]);
    atomicAdd(&hess_hist[h], hess[i]);
    atomicAdd(&count_hist[h], 1.0);
}

// Caller must zero grad_hist, hess_hist, count_hist (each p*n_bins doubles) before calling.
extern "C" void launch_histogram_build(
    const double* bins, const double* grad, const double* hess, const double* mask,
    double* grad_hist, double* hess_hist, double* count_hist,
    int n, int p, int n_bins, hipStream_t stream
) {
    int total = n * p;
    dim3 bl(256); dim3 gr((total + 255) / 256);
    hipLaunchKernelGGL(histogram_build_kernel, gr, bl, 0, stream,
        bins, grad, hess, mask, grad_hist, hess_hist, count_hist, n, p, n_bins);
}

// ── split_eval ─────────────────────────────────────────────────────────────
// Per feature: serial prefix-sum over histogram, evaluate gain at each split.
// gain = G_L²/(H_L+λ) + G_R²/(H_R+λ) - G_parent²/(H_parent+λ)
// One thread per feature — n_bins is small (typically ≤256).

__global__ void split_eval_kernel(
    const double* grad_hist, const double* hess_hist,
    double* best_gain, double* best_bin,
    int p, int n_bins, double lambda, double min_child_weight
) {
    int f = blockIdx.x * blockDim.x + threadIdx.x;
    if (f >= p) return;

    const double* gh = grad_hist + f * n_bins;
    const double* hh = hess_hist + f * n_bins;

    double total_g = 0, total_h = 0;
    for (int b = 0; b < n_bins; b++) { total_g += gh[b]; total_h += hh[b]; }
    double parent = total_g * total_g / (total_h + lambda);

    double left_g = 0, left_h = 0;
    double bg = -1e308; int bb = -1;

    for (int b = 0; b < n_bins - 1; b++) {
        left_g += gh[b]; left_h += hh[b];
        double right_g = total_g - left_g, right_h = total_h - left_h;
        if (left_h < min_child_weight || right_h < min_child_weight) continue;
        double gain = left_g*left_g/(left_h+lambda) + right_g*right_g/(right_h+lambda) - parent;
        if (gain > bg) { bg = gain; bb = b; }
    }

    best_gain[f] = bg;
    best_bin[f] = (double)bb;
}

extern "C" void launch_split_eval(
    const double* grad_hist, const double* hess_hist,
    double* best_gain, double* best_bin,
    int p, int n_bins, double lambda, double min_child_weight, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((p + 255) / 256);
    hipLaunchKernelGGL(split_eval_kernel, gr, bl, 0, stream,
        grad_hist, hess_hist, best_gain, best_bin, p, n_bins, lambda, min_child_weight);
}

// ── data_partition ─────────────────────────────────────────────────────────
// Split active samples (mask=1) into left (bin ≤ threshold) and right (bin > threshold).

__global__ void data_partition_kernel(
    const double* bins, const double* node_mask,
    double* left_mask, double* right_mask,
    int n, int p, int split_feat, int split_bin
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    if (node_mask[i] == 0.0) { left_mask[i] = 0.0; right_mask[i] = 0.0; return; }
    int bin = (int)bins[i * p + split_feat];
    left_mask[i] = (bin <= split_bin) ? 1.0 : 0.0;
    right_mask[i] = (bin > split_bin) ? 1.0 : 0.0;
}

extern "C" void launch_data_partition(
    const double* bins, const double* node_mask,
    double* left_mask, double* right_mask,
    int n, int p, int split_feat, int split_bin, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n + 255) / 256);
    hipLaunchKernelGGL(data_partition_kernel, gr, bl, 0, stream,
        bins, node_mask, left_mask, right_mask, n, p, split_feat, split_bin);
}

// ── tree_build ────────────────────────────────────────────────────────────
// Full level-wise gradient boosted tree. Builds histogram, evaluates splits,
// repartitions samples at each depth — all on GPU. No host↔device sync in
// the depth loop. Returns per-sample leaf predictions for train AND test.

// Phase 1: Build histograms for ALL nodes at current depth level.
// Each thread handles one (sample, feature) pair. node_assign routes the
// sample's contribution to the correct node's histogram via atomicAdd.
__global__ void tb_histogram_kernel(
    const double* bins,       // [n, p]
    const double* grad,       // [n]
    const double* hess,       // [n]
    const int* node_assign,   // [n]
    double* grad_hist,        // [n_level_nodes * p * n_bins]
    double* hess_hist,        // [n_level_nodes * p * n_bins]
    int n, int p, int n_bins, int level_base
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * p) return;
    int i = idx / p;
    int f = idx % p;
    int node = node_assign[i];
    if (node < level_base) return;  // Sample already at a leaf from earlier depth
    int level_off = node - level_base;
    int b = (int)bins[i * p + f];
    if (b < 0 || b >= n_bins) return;
    int h = level_off * p * n_bins + f * n_bins + b;
    atomicAdd(&grad_hist[h], grad[i]);
    atomicAdd(&hess_hist[h], hess[i]);
}

// Phase 2: Find best split per node. One thread per node — loops over all
// features and bins (serial scan, same gain formula as split_eval_kernel).
__global__ void tb_split_eval_kernel(
    const double* grad_hist,  // [n_level_nodes * p * n_bins]
    const double* hess_hist,
    int* split_feat,          // [max_nodes] — global, indexed by node_id
    int* split_bin,           // [max_nodes]
    int n_level_nodes, int p, int n_bins,
    double lambda, double min_cw, int level_base
) {
    int node_off = blockIdx.x * blockDim.x + threadIdx.x;
    if (node_off >= n_level_nodes) return;
    int node_id = level_base + node_off;

    double best_gain = 0.0;   // Must beat 0 to split — else leaf
    int best_f = -1, best_b = -1;

    for (int f = 0; f < p; f++) {
        const double* gh = grad_hist + node_off * p * n_bins + f * n_bins;
        const double* hh = hess_hist + node_off * p * n_bins + f * n_bins;

        double total_g = 0, total_h = 0;
        for (int b = 0; b < n_bins; b++) { total_g += gh[b]; total_h += hh[b]; }
        double parent = total_g * total_g / (total_h + lambda);

        double left_g = 0, left_h = 0;
        for (int b = 0; b < n_bins - 1; b++) {
            left_g += gh[b]; left_h += hh[b];
            double right_g = total_g - left_g, right_h = total_h - left_h;
            if (left_h < min_cw || right_h < min_cw) continue;
            double gain = left_g*left_g/(left_h+lambda) + right_g*right_g/(right_h+lambda) - parent;
            if (gain > best_gain) { best_gain = gain; best_f = f; best_b = b; }
        }
    }

    split_feat[node_id] = best_f;
    split_bin[node_id] = best_b;
}

// Phase 3: Repartition — each sample follows its node's split to a child.
__global__ void tb_repartition_kernel(
    const double* bins,       // [n, p]
    int* node_assign,         // [n] — updated in-place
    const int* split_feat,    // [max_nodes]
    const int* split_bin,     // [max_nodes]
    int n, int p
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int node = node_assign[i];
    int f = split_feat[node];
    if (f < 0) return;  // Node is a leaf — don't repartition
    int b = split_bin[node];
    int bin_val = (int)bins[i * p + f];
    node_assign[i] = (bin_val <= b) ? (2 * node + 1) : (2 * node + 2);
}

// Phase 4a: Sum gradient and hessian per leaf via atomicAdd.
__global__ void tb_leaf_sum_kernel(
    const double* grad, const double* hess,
    const int* node_assign,
    double* node_sum_g, double* node_sum_h,
    int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int node = node_assign[i];
    atomicAdd(&node_sum_g[node], grad[i]);
    atomicAdd(&node_sum_h[node], hess[i]);
}

// Phase 4b: Compute leaf value = -G / (H + λ).
__global__ void tb_leaf_val_kernel(
    const double* node_sum_g, const double* node_sum_h,
    double* leaf_val,
    int n_leaves, int leaf_base, double lambda
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_leaves) return;
    int node = leaf_base + i;
    leaf_val[node] = -node_sum_g[node] / (node_sum_h[node] + lambda);
}

// Phase 5a: Scatter leaf predictions to train samples.
__global__ void tb_scatter_kernel(
    const int* node_assign, const double* leaf_val,
    double* predictions, int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    predictions[i] = leaf_val[node_assign[i]];
}

// Phase 5b: Apply built tree to test data — each thread traverses root→leaf.
__global__ void tb_apply_tree_kernel(
    const double* bins,       // [n_te, p]
    const int* split_feat,    // [max_nodes]
    const int* split_bin,     // [max_nodes]
    const double* leaf_val,   // [max_nodes]
    double* predictions,      // [n_te]
    int n, int p, int max_depth
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int node = 0;
    for (int d = 0; d < max_depth; d++) {
        int f = split_feat[node];
        if (f < 0) break;  // Early-stopped leaf
        int b = split_bin[node];
        int bin_val = (int)bins[i * p + f];
        node = (bin_val <= b) ? (2 * node + 1) : (2 * node + 2);
    }
    predictions[i] = leaf_val[node];
}

// Per-step launchers. The Rust caller allocates all scratch buffers, performs
// every zero/sentinel fill, runs the depth loop, and frees the scratch. Each
// wrapper below does nothing but hipLaunchKernelGGL.
//
// Caller-allocated scratch (max_nodes = (1<<(max_depth+1))-1,
//   max_level = max_depth<=1 ? 1 : 1<<(max_depth-1), hist_elems = max_level*p*n_bins):
//   node_assign : n_tr      ints     — fill 0
//   sf, sb      : max_nodes ints     — fill 0xFF (-1 leaf sentinel)
//   gh, hh      : hist_elems doubles — per level d: zero first n_level*p*n_bins doubles
//   sum_g,sum_h : max_nodes doubles  — fill 0 before leaf_sum
//   lv          : max_nodes doubles  — fill 0 before leaf_val
//
// Caller depth loop (d in 0..max_depth): level_base=(1<<d)-1, n_level=1<<d:
//   zero gh,hh (n_level*p*n_bins doubles) → launch_tb_histogram(level_base)
//   → launch_tb_split_eval(n_level, level_base) → launch_tb_repartition
// After loop: zero sum_g,sum_h,lv → launch_tb_leaf_sum → launch_tb_leaf_val(max_nodes,0,lambda)
//   → launch_tb_scatter (tr) → launch_tb_apply_tree (te).

extern "C" void launch_tb_histogram(
    const double* tr_bins, const double* grad, const double* hess,
    const int* node_assign, double* grad_hist, double* hess_hist,
    int n_tr, int p, int n_bins, int level_base, hipStream_t stream
) {
    int total_nf = n_tr * p;
    dim3 bl(256); dim3 gr((total_nf + 255) / 256);
    hipLaunchKernelGGL(tb_histogram_kernel, gr, bl, 0, stream,
        tr_bins, grad, hess, node_assign, grad_hist, hess_hist, n_tr, p, n_bins, level_base);
}

extern "C" void launch_tb_split_eval(
    const double* grad_hist, const double* hess_hist,
    int* split_feat, int* split_bin,
    int n_level, int p, int n_bins, double lambda, double min_cw, int level_base,
    hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n_level + 255) / 256);
    hipLaunchKernelGGL(tb_split_eval_kernel, gr, bl, 0, stream,
        grad_hist, hess_hist, split_feat, split_bin, n_level, p, n_bins, lambda, min_cw, level_base);
}

extern "C" void launch_tb_repartition(
    const double* tr_bins, int* node_assign,
    const int* split_feat, const int* split_bin,
    int n_tr, int p, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n_tr + 255) / 256);
    hipLaunchKernelGGL(tb_repartition_kernel, gr, bl, 0, stream,
        tr_bins, node_assign, split_feat, split_bin, n_tr, p);
}

extern "C" void launch_tb_leaf_sum(
    const double* grad, const double* hess, const int* node_assign,
    double* node_sum_g, double* node_sum_h, int n_tr, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n_tr + 255) / 256);
    hipLaunchKernelGGL(tb_leaf_sum_kernel, gr, bl, 0, stream,
        grad, hess, node_assign, node_sum_g, node_sum_h, n_tr);
}

extern "C" void launch_tb_leaf_val(
    const double* node_sum_g, const double* node_sum_h, double* leaf_val,
    int n_leaves, int leaf_base, double lambda, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n_leaves + 255) / 256);
    hipLaunchKernelGGL(tb_leaf_val_kernel, gr, bl, 0, stream,
        node_sum_g, node_sum_h, leaf_val, n_leaves, leaf_base, lambda);
}

extern "C" void launch_tb_scatter(
    const int* node_assign, const double* leaf_val, double* predictions,
    int n_tr, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n_tr + 255) / 256);
    hipLaunchKernelGGL(tb_scatter_kernel, gr, bl, 0, stream,
        node_assign, leaf_val, predictions, n_tr);
}

extern "C" void launch_tb_apply_tree(
    const double* te_bins, const int* split_feat, const int* split_bin,
    const double* leaf_val, double* predictions,
    int n_te, int p, int max_depth, hipStream_t stream
) {
    dim3 bl(256); dim3 gr((n_te + 255) / 256);
    hipLaunchKernelGGL(tb_apply_tree_kernel, gr, bl, 0, stream,
        te_bins, split_feat, split_bin, leaf_val, predictions, n_te, p, max_depth);
}

// ══════════════════════════════════════════════════════════════════════════════
// GPU oblivious tree kernels — u8 bins, f32 grad/hess, shared-mem histograms
// ══════════════════════════════════════════════════════════════════════════════

// ── kernel 1: oblivious histogram ─────────────────────────────────────────
// One block per (node, feature). Shared-mem [256] accumulators.
// bins_fm: feature-major [n_features, n_rows] u8
// node_idx: [n_rows] u8
extern "C" __global__
void oblivious_histogram_kernel(
    const unsigned char* bins_fm,
    const unsigned char* node_idx,
    const float* grad,
    const float* hess,
    float* grad_hist,
    float* hess_hist,
    int n_rows,
    int n_features,
    int n_bins,
    int n_nodes
) {
    int feat = blockIdx.x;
    int node = blockIdx.y;

    __shared__ float s_grad[256];
    __shared__ float s_hess[256];

    for (int b = threadIdx.x; b < n_bins; b += blockDim.x) {
        s_grad[b] = 0.0f;
        s_hess[b] = 0.0f;
    }
    __syncthreads();

    const unsigned char* feat_bins = bins_fm + feat * n_rows;
    for (int row = threadIdx.x; row < n_rows; row += blockDim.x) {
        if (node_idx[row] == (unsigned char)node) {
            unsigned char bin = feat_bins[row];
            atomicAdd(&s_grad[bin], grad[row]);
            atomicAdd(&s_hess[bin], hess[row]);
        }
    }
    __syncthreads();

    int base = ((node * n_features) + feat) * n_bins;
    for (int b = threadIdx.x; b < n_bins; b += blockDim.x) {
        grad_hist[base + b] = s_grad[b];
        hess_hist[base + b] = s_hess[b];
    }
}

extern "C" void launch_oblivious_histogram(
    const unsigned char* bins_fm, const unsigned char* node_idx,
    const float* grad, const float* hess,
    float* grad_hist, float* hess_hist,
    int n_rows, int n_features, int n_bins, int n_nodes,
    hipStream_t stream
) {
    dim3 grid(n_features, n_nodes);
    dim3 block(256);
    hipLaunchKernelGGL(oblivious_histogram_kernel, grid, block, 0, stream,
        bins_fm, node_idx, grad, hess, grad_hist, hess_hist,
        n_rows, n_features, n_bins, n_nodes);
}

// ── kernel 2a: oblivious route step ───────────────────────────────────────
// Update node assignment for one depth level.
// node_out[row] = node_in[row] | ((bins_rm[row * n_features + split_feat] > split_bin) << depth)
extern "C" __global__
void oblivious_route_step_kernel(
    const unsigned char* bins_rm,
    const unsigned char* node_in,
    unsigned char* node_out,
    int split_feat,
    unsigned char split_bin,
    int depth,
    int n_rows,
    int n_features
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    unsigned int bit = bins_rm[row * n_features + split_feat] > split_bin ? 1u : 0u;
    node_out[row] = node_in[row] | (unsigned char)(bit << depth);
}

extern "C" void launch_oblivious_route_step(
    const unsigned char* bins_rm, const unsigned char* node_in, unsigned char* node_out,
    int split_feat, unsigned char split_bin, int depth,
    int n_rows, int n_features, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(oblivious_route_step_kernel, grid, block, 0, stream,
        bins_rm, node_in, node_out, split_feat, split_bin, depth, n_rows, n_features);
}

extern "C" __global__
void oblivious_route_step_dev_kernel(
    const unsigned char* bins_rm,
    const unsigned char* node_in,
    unsigned char* node_out,
    const int* split_feat_arr,
    const unsigned char* split_bin_arr,
    int depth,
    int n_rows,
    int n_features
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    int sf = split_feat_arr[depth];
    unsigned char sb = split_bin_arr[depth];
    unsigned int bit = bins_rm[row * n_features + sf] > sb ? 1u : 0u;
    node_out[row] = node_in[row] | (unsigned char)(bit << depth);
}

extern "C" void launch_oblivious_route_step_dev(
    const unsigned char* bins_rm, const unsigned char* node_in, unsigned char* node_out,
    const int* split_feat_arr, const unsigned char* split_bin_arr, int depth,
    int n_rows, int n_features, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(oblivious_route_step_dev_kernel, grid, block, 0, stream,
        bins_rm, node_in, node_out, split_feat_arr, split_bin_arr, depth, n_rows, n_features);
}

// ── kernel 2b: oblivious route full ───────────────────────────────────────
// Compute final leaf index from all depth splits.
extern "C" __global__
void oblivious_route_full_kernel(
    const unsigned char* bins_rm,
    const int* split_feat,
    const unsigned char* split_bin,
    unsigned char* leaf_idx,
    int n_rows,
    int n_features,
    int depth
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    const unsigned char* rp = bins_rm + row * n_features;
    unsigned int leaf = 0;
    for (int d = 0; d < depth; d++) {
        unsigned int bit = rp[split_feat[d]] > split_bin[d] ? 1u : 0u;
        leaf |= (bit << d);
    }
    leaf_idx[row] = (unsigned char)leaf;
}

extern "C" void launch_oblivious_route_full(
    const unsigned char* bins_rm, const int* split_feat, const unsigned char* split_bin,
    unsigned char* leaf_idx, int n_rows, int n_features, int depth, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(oblivious_route_full_kernel, grid, block, 0, stream,
        bins_rm, split_feat, split_bin, leaf_idx, n_rows, n_features, depth);
}

// ── kernel 3: scatter-add by leaf ─────────────────────────────────────────
extern "C" __global__
void scatter_add_by_leaf_kernel(
    float* pred,
    const unsigned char* leaf_idx,
    const float* leaf_value,
    float lr,
    int n_rows
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    pred[row] += lr * leaf_value[leaf_idx[row]];
}

extern "C" void launch_scatter_add_by_leaf(
    float* pred, const unsigned char* leaf_idx, const float* leaf_value,
    float lr, int n_rows, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(scatter_add_by_leaf_kernel, grid, block, 0, stream,
        pred, leaf_idx, leaf_value, lr, n_rows);
}

// ── kernel 4: leaf reduce ─────────────────────────────────────────────────
// Scatter grad/hess into leaf buckets via atomics.
extern "C" __global__
void leaf_reduce_kernel(
    const unsigned char* leaf_idx,
    const float* grad,
    const float* hess,
    float* leaf_grad,
    float* leaf_hess,
    int n_rows
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= n_rows) return;
    unsigned char leaf = leaf_idx[row];
    atomicAdd(&leaf_grad[leaf], grad[row]);
    atomicAdd(&leaf_hess[leaf], hess[row]);
}

extern "C" void launch_leaf_reduce(
    const unsigned char* leaf_idx, const float* grad, const float* hess,
    float* leaf_grad, float* leaf_hess, int n_rows, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_rows + 255) / 256);
    hipLaunchKernelGGL(leaf_reduce_kernel, grid, block, 0, stream,
        leaf_idx, grad, hess, leaf_grad, leaf_hess, n_rows);
}

// ── kernel 5: leaf finalize ───────────────────────────────────────────────
// leaf_value[j] = -leaf_grad[j] / (leaf_hess[j] + lambda)
extern "C" __global__
void leaf_finalize_kernel(
    const float* leaf_grad,
    const float* leaf_hess,
    float* leaf_value,
    float lambda,
    int n_leaves
) {
    int j = blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n_leaves) return;
    leaf_value[j] = -leaf_grad[j] / (leaf_hess[j] + lambda);
}

extern "C" void launch_leaf_finalize(
    const float* leaf_grad, const float* leaf_hess, float* leaf_value,
    float lambda, int n_leaves, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((n_leaves + 255) / 256);
    hipLaunchKernelGGL(leaf_finalize_kernel, grid, block, 0, stream,
        leaf_grad, leaf_hess, leaf_value, lambda, n_leaves);
}

// ── kernel 6: oblivious split evaluation ──────────────────────────────────
// One block per feature. Shared-memory prefix sums over histogram bins.
// gain_out[feat * n_bins + bin] = sum over nodes of split gain at that bin.
extern "C" __global__
void oblivious_split_eval_kernel(
    const float* grad_hist,
    const float* hess_hist,
    float* gain_out,
    int n_nodes,
    int n_features,
    int n_bins,
    float lambda,
    float min_cw
) {
    int feat = blockIdx.x;
    if (feat >= n_features) return;

    __shared__ float s_pg[256];
    __shared__ float s_ph[256];

    for (int bin = threadIdx.x; bin < n_bins; bin += blockDim.x)
        gain_out[feat * n_bins + bin] = 0.0f;
    __syncthreads();

    for (int node = 0; node < n_nodes; node++) {
        int base = ((node * n_features) + feat) * n_bins;

        // Load into shared mem and compute prefix sums
        float g_val = (threadIdx.x < n_bins) ? grad_hist[base + threadIdx.x] : 0.0f;
        float h_val = (threadIdx.x < n_bins) ? hess_hist[base + threadIdx.x] : 0.0f;

        // Inclusive prefix sum via shared memory
        s_pg[threadIdx.x] = g_val;
        s_ph[threadIdx.x] = h_val;
        __syncthreads();

        for (int stride = 1; stride < n_bins; stride <<= 1) {
            float pg = (threadIdx.x >= stride) ? s_pg[threadIdx.x - stride] : 0.0f;
            float ph = (threadIdx.x >= stride) ? s_ph[threadIdx.x - stride] : 0.0f;
            __syncthreads();
            s_pg[threadIdx.x] += pg;
            s_ph[threadIdx.x] += ph;
            __syncthreads();
        }

        float tg = s_pg[n_bins - 1];
        float th = s_ph[n_bins - 1];
        if (th < 1e-10f) { __syncthreads(); continue; }
        float ps = tg * tg / (th + lambda);

        if (threadIdx.x < n_bins - 1) {
            float lg = s_pg[threadIdx.x];
            float lh = s_ph[threadIdx.x];
            float rg = tg - lg, rh = th - lh;
            if (lh >= min_cw && rh >= min_cw) {
                float gain = lg * lg / (lh + lambda) + rg * rg / (rh + lambda) - ps;
                gain_out[feat * n_bins + threadIdx.x] += gain;
            }
        }
        __syncthreads();
    }
}

extern "C" void launch_oblivious_split_eval(
    const float* grad_hist, const float* hess_hist, float* gain_out,
    int n_nodes, int n_features, int n_bins, float lambda, float min_cw,
    hipStream_t stream
) {
    dim3 grid(n_features);
    dim3 block(256);
    hipLaunchKernelGGL(oblivious_split_eval_kernel, grid, block, 0, stream,
        grad_hist, hess_hist, gain_out, n_nodes, n_features, n_bins, lambda, min_cw);
}
