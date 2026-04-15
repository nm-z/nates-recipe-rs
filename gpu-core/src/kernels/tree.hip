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

extern "C" void launch_histogram_build(
    const double* bins, const double* grad, const double* hess, const double* mask,
    double* grad_hist, double* hess_hist, double* count_hist,
    int n, int p, int n_bins, hipStream_t stream
) {
    size_t hb = (size_t)p * n_bins * sizeof(double);
    hipMemsetAsync(grad_hist, 0, hb, stream);
    hipMemsetAsync(hess_hist, 0, hb, stream);
    hipMemsetAsync(count_hist, 0, hb, stream);
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

// Launcher — orchestrates the full tree build on GPU. The ONLY host function
// called from Rust. Allocates temporaries, runs depth loop, frees temps.
extern "C" void launch_tree_build(
    const double* tr_bins, int n_tr,
    const double* grad, const double* hess,
    const double* te_bins, int n_te,
    int p, int n_bins, int max_depth, double lambda, double min_cw,
    double* tr_pred, double* te_pred,
    hipStream_t stream
) {
    int max_nodes = (1 << (max_depth + 1)) - 1;
    int n_leaves = 1 << max_depth;
    int leaf_base = (1 << max_depth) - 1;

    // Allocate temporaries
    int* node_assign;  hipMalloc((void**)&node_assign, (size_t)n_tr * sizeof(int));
    int* sf;           hipMalloc((void**)&sf, (size_t)max_nodes * sizeof(int));
    int* sb;           hipMalloc((void**)&sb, (size_t)max_nodes * sizeof(int));
    double* sum_g;     hipMalloc((void**)&sum_g, (size_t)max_nodes * sizeof(double));
    double* sum_h;     hipMalloc((void**)&sum_h, (size_t)max_nodes * sizeof(double));
    double* lv;        hipMalloc((void**)&lv, (size_t)max_nodes * sizeof(double));

    // Largest level determines histogram size
    size_t max_level = (size_t)(1 << (max_depth - 1));  // deepest split level
    if (max_depth <= 1) max_level = 1;
    size_t hist_elems = max_level * p * n_bins;
    double* gh;  hipMalloc((void**)&gh, hist_elems * sizeof(double));
    double* hh;  hipMalloc((void**)&hh, hist_elems * sizeof(double));

    hipMemsetAsync(node_assign, 0, (size_t)n_tr * sizeof(int), stream);
    hipMemsetAsync(sf, 0xFF, (size_t)max_nodes * sizeof(int), stream);  // -1 = leaf sentinel
    hipMemsetAsync(sb, 0xFF, (size_t)max_nodes * sizeof(int), stream);

    dim3 bl(256);

    // ── Depth loop ────────────────────────────────────────────────────────
    for (int d = 0; d < max_depth; d++) {
        int level_base = (1 << d) - 1;
        int n_level = 1 << d;
        size_t lh_bytes = (size_t)n_level * p * n_bins * sizeof(double);

        // Clear histograms for this level
        hipMemsetAsync(gh, 0, lh_bytes, stream);
        hipMemsetAsync(hh, 0, lh_bytes, stream);

        // Histogram build
        int total_nf = n_tr * p;
        dim3 gr_hist((total_nf + 255) / 256);
        hipLaunchKernelGGL(tb_histogram_kernel, gr_hist, bl, 0, stream,
            tr_bins, grad, hess, node_assign, gh, hh, n_tr, p, n_bins, level_base);

        // Split eval
        dim3 gr_eval((n_level + 255) / 256);
        hipLaunchKernelGGL(tb_split_eval_kernel, gr_eval, bl, 0, stream,
            gh, hh, sf, sb, n_level, p, n_bins, lambda, min_cw, level_base);

        // Repartition train
        dim3 gr_tr((n_tr + 255) / 256);
        hipLaunchKernelGGL(tb_repartition_kernel, gr_tr, bl, 0, stream,
            tr_bins, node_assign, sf, sb, n_tr, p);
    }

    // ── Leaf values ───────────────────────────────────────────────────────
    hipMemsetAsync(sum_g, 0, (size_t)max_nodes * sizeof(double), stream);
    hipMemsetAsync(sum_h, 0, (size_t)max_nodes * sizeof(double), stream);
    hipMemsetAsync(lv, 0, (size_t)max_nodes * sizeof(double), stream);

    dim3 gr_sum((n_tr + 255) / 256);
    hipLaunchKernelGGL(tb_leaf_sum_kernel, gr_sum, bl, 0, stream,
        grad, hess, node_assign, sum_g, sum_h, n_tr);

    dim3 gr_lv((max_nodes + 255) / 256);
    hipLaunchKernelGGL(tb_leaf_val_kernel, gr_lv, bl, 0, stream,
        sum_g, sum_h, lv, max_nodes, 0, lambda);

    // ── Scatter predictions ───────────────────────────────────────────────
    hipLaunchKernelGGL(tb_scatter_kernel, gr_sum, bl, 0, stream,
        node_assign, lv, tr_pred, n_tr);

    // Test: single pass traversal
    dim3 gr_te((n_te + 255) / 256);
    hipLaunchKernelGGL(tb_apply_tree_kernel, gr_te, bl, 0, stream,
        te_bins, sf, sb, lv, te_pred, n_te, p, max_depth);

    // Free temporaries
    hipFree(node_assign);
    hipFree(sf); hipFree(sb);
    hipFree(sum_g); hipFree(sum_h); hipFree(lv);
    hipFree(gh); hipFree(hh);
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
// One block per feature. For each bin, prefix-sum through histogram and
// compute total gain across all nodes.
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

    for (int bin = threadIdx.x; bin < n_bins; bin += blockDim.x) {
        float total_gain = 0.0f;
        for (int node = 0; node < n_nodes; node++) {
            int base = ((node * n_features) + feat) * n_bins;
            float tg = 0.0f, th = 0.0f;
            for (int b = 0; b < n_bins; b++) { tg += grad_hist[base + b]; th += hess_hist[base + b]; }
            if (th < 1e-10f) continue;
            float ps = tg * tg / (th + lambda);
            float lg = 0.0f, lh = 0.0f;
            for (int b = 0; b <= bin; b++) { lg += grad_hist[base + b]; lh += hess_hist[base + b]; }
            float rg = tg - lg, rh = th - lh;
            if (lh < min_cw || rh < min_cw) continue;
            total_gain += lg * lg / (lh + lambda) + rg * rg / (rh + lambda) - ps;
        }
        gain_out[feat * n_bins + bin] = total_gain;
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
