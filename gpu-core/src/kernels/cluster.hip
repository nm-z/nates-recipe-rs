#include <hip/hip_runtime.h>
#include <float.h>

// ── Fixed-radius neighbors ──────────────────────────────────────────────────
// For each pair (i,j): compute squared L2, compare to eps2.
// mask[i*n+j] = 1 if dist2 <= eps2, else 0.
// count[i]    = number of j with mask[i*n+j]=1.
// Each thread handles one (i,j) pair in the upper+lower matrix.

extern "C" __global__ void fixed_radius_kernel(
      const double* points, unsigned char* mask, int* count,
      int n, int dim, double eps2
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = n * n;
      if (idx >= total) return;
      int i = idx / n;
      int j = idx % n;
      double d2 = 0.0;
      for (int d = 0; d < dim; d++) {
            double diff = points[i * dim + d] - points[j * dim + d];
            d2 += diff * diff;
      }
      unsigned char hit = (d2 <= eps2) ? 1 : 0;
      mask[idx] = hit;
      if (hit) atomicAdd(&count[i], 1);
}

extern "C" void launch_fixed_radius_neighbors(
      const double* points, unsigned char* mask, int* count,
      int n, int dim, double eps, hipStream_t stream
) {
      double eps2 = eps * eps;
      // zero count
      hipMemset(count, 0, (size_t)n * sizeof(int));
      int total = n * n;
      int threads = 256;
      int blocks = (total + threads - 1) / threads;
      hipLaunchKernelGGL(fixed_radius_kernel, dim3(blocks), dim3(threads), 0, stream,
            points, mask, count, n, dim, eps2);
}

// ── Union-Find: initialize ──────────────────────────────────────────────────
// parent[i] = i  (each node is its own root)

extern "C" __global__ void uf_init_kernel(int* parent, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) parent[i] = i;
}

// ── Union-Find: hook step ───────────────────────────────────────────────────
// For each edge (src,dst): if roots differ, hook smaller root under larger.
// changed[0] is set to 1 if any hook happened.

extern "C" __global__ void uf_hook_kernel(
      const int* src, const int* dst, int* parent, int* changed, int n_edges
) {
      int e = blockIdx.x * blockDim.x + threadIdx.x;
      if (e >= n_edges) return;
      int u = src[e];
      int v = dst[e];
      // walk to roots (bounded by n_nodes depth — safe for forests)
      while (parent[u] != u) u = parent[u];
      while (parent[v] != v) v = parent[v];
      if (u == v) return;
      int lo = u < v ? u : v;
      int hi = u < v ? v : u;
      // hook lo under hi
      int old = atomicCAS(&parent[lo], lo, hi);
      if (old == lo) atomicAdd(changed, 1);
}

// ── Union-Find: path compression ───────────────────────────────────────────
// Each node walks to its root and writes root directly.

extern "C" __global__ void uf_compress_kernel(int* parent, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      int root = i;
      while (parent[root] != root) root = parent[root];
      parent[i] = root;
}

// ── Union-Find: label relabeling ────────────────────────────────────────────
// After compression: parent[i] is the root. Map roots to compact 0..k-1 labels.
// We do this on host after downloading parent[].

// Host entry for launch_union_find_cc: loops until no change.
extern "C" void launch_union_find_cc(
      const int* edge_src, const int* edge_dst, int* parent_buf,
      int n_nodes, int n_edges, hipStream_t stream
) {
      // Initialize
      {
            int threads = 256;
            int blocks = (n_nodes + threads - 1) / threads;
            hipLaunchKernelGGL(uf_init_kernel, dim3(blocks), dim3(threads), 0, stream,
                  parent_buf, n_nodes);
      }
      hipStreamSynchronize(stream);

      // Allocate changed flag on GPU
      int* d_changed = nullptr;
      hipMalloc((void**)&d_changed, sizeof(int));

      int threads_e = 256;
      int blocks_e = (n_edges + threads_e - 1) / threads_e;
      int threads_n = 256;
      int blocks_n = (n_nodes + threads_n - 1) / threads_n;

      // Iterate until stable
      for (int iter = 0; iter < n_nodes + 1; iter++) {
            hipMemset(d_changed, 0, sizeof(int));
            hipLaunchKernelGGL(uf_hook_kernel, dim3(blocks_e), dim3(threads_e), 0, stream,
                  edge_src, edge_dst, parent_buf, d_changed, n_edges);
            hipLaunchKernelGGL(uf_compress_kernel, dim3(blocks_n), dim3(threads_n), 0, stream,
                  parent_buf, n_nodes);
            hipStreamSynchronize(stream);
            int h_changed = 0;
            hipMemcpy(&h_changed, d_changed, sizeof(int), hipMemcpyDeviceToHost);
            if (h_changed == 0) break;
      }
      hipFree(d_changed);
}

// ── Borůvka: find cheapest outgoing edge per component ─────────────────────
// For each edge e=(u,v,w): if comp[u] != comp[v], try to update cheapest[comp[u]] and cheapest[comp[v]].
// cheapest[c] = index of cheapest outgoing edge from component c (-1 = none).
// We store (best_w, best_e_idx) packed as two arrays.

extern "C" __global__ void boruvka_find_cheapest_kernel(
      const int* src, const int* dst, const double* w,
      const int* parent, int* best_edge, double* best_w,
      int n_edges
) {
      int e = blockIdx.x * blockDim.x + threadIdx.x;
      if (e >= n_edges) return;
      int u = src[e], v = dst[e];
      int cu = parent[u], cv = parent[v];
      if (cu == cv) return; // same component
      double ew = w[e];
      // Try update for component cu
      // We use a double-precision atomic via integer CAS trick
      // Simplification: use atomicMin on a long long encoding (limited to positive weights)
      // For generality we do two separate atomic operations on int arrays:
      // best_edge[cu] stores the edge index of the cheapest found so far
      // We loop with atomicCAS — this is O(contention) but correct.
      // Update cu's best:
      while (1) {
            int old_e = best_edge[cu];
            double old_w = (old_e < 0) ? DBL_MAX : best_w[cu];
            if (ew >= old_w) break;
            int got = atomicCAS(&best_edge[cu], old_e, e);
            if (got == old_e) { best_w[cu] = ew; break; }
      }
      // Update cv's best:
      while (1) {
            int old_e = best_edge[cv];
            double old_w = (old_e < 0) ? DBL_MAX : best_w[cv];
            if (ew >= old_w) break;
            int got = atomicCAS(&best_edge[cv], old_e, e);
            if (got == old_e) { best_w[cv] = ew; break; }
      }
}

// Mark MST edges found in this Borůvka round
extern "C" __global__ void boruvka_mark_kernel(
      const int* best_edge, unsigned char* in_mst, int n_nodes
) {
      int c = blockIdx.x * blockDim.x + threadIdx.x;
      if (c >= n_nodes) return;
      int e = best_edge[c];
      if (e >= 0) in_mst[e] = 1;
}

// Host-side Borůvka MST loop
extern "C" void launch_boruvka_mst(
      const int* edge_src, const int* edge_dst, const double* edge_w,
      unsigned char* in_mst, double* total_weight_buf,
      int n_nodes, int n_edges, hipStream_t stream
) {
      // Allocate union-find parent array
      int* parent = nullptr;
      hipMalloc((void**)&parent, (size_t)n_nodes * sizeof(int));
      {
            int threads = 256;
            int blocks = (n_nodes + threads - 1) / threads;
            hipLaunchKernelGGL(uf_init_kernel, dim3(blocks), dim3(threads), 0, stream, parent, n_nodes);
      }

      // Allocate best-edge arrays
      int* best_edge = nullptr;
      double* best_w = nullptr;
      hipMalloc((void**)&best_edge, (size_t)n_nodes * sizeof(int));
      hipMalloc((void**)&best_w, (size_t)n_nodes * sizeof(double));

      // Also need changed flag for union-find inner loop
      int* d_changed = nullptr;
      hipMalloc((void**)&d_changed, sizeof(int));

      hipMemset(in_mst, 0, (size_t)n_edges * sizeof(unsigned char));
      hipStreamSynchronize(stream);

      int threads_n = 256;
      int blocks_n  = (n_nodes + threads_n - 1) / threads_n;
      int threads_e = 256;
      int blocks_e  = (n_edges + threads_e - 1) / threads_e;

      // Borůvka: at most ceil(log2(n_nodes)) rounds
      int max_rounds = 0;
      { int tmp = n_nodes; while (tmp > 1) { tmp = (tmp + 1) / 2; max_rounds++; } if (max_rounds < 1) max_rounds = 1; }

      for (int round = 0; round < max_rounds; round++) {
            // Init best_edge = -1
            // hipMemset works on int: -1 in two's complement = 0xFF...FF = memset(ptr, 0xff, ...)
            hipMemset(best_edge, 0xff, (size_t)n_nodes * sizeof(int));
            hipStreamSynchronize(stream);

            // Find cheapest outgoing edge per component
            hipLaunchKernelGGL(boruvka_find_cheapest_kernel, dim3(blocks_e), dim3(threads_e), 0, stream,
                  edge_src, edge_dst, edge_w, parent, best_edge, best_w, n_edges);
            hipStreamSynchronize(stream);

            // Check if any component found an outgoing edge
            // Download best_edge to host and check
            int* h_best_edge = (int*)malloc((size_t)n_nodes * sizeof(int));
            hipMemcpy(h_best_edge, best_edge, (size_t)n_nodes * sizeof(int), hipMemcpyDeviceToHost);

            int any_found = 0;
            for (int c = 0; c < n_nodes; c++) {
                  if (h_best_edge[c] >= 0) { any_found = 1; break; }
            }
            if (!any_found) { free(h_best_edge); break; }

            // Mark edges in MST
            hipLaunchKernelGGL(boruvka_mark_kernel, dim3(blocks_n), dim3(threads_n), 0, stream,
                  best_edge, in_mst, n_nodes);
            hipStreamSynchronize(stream);

            // Merge components via union-find on ONLY the selected cheapest edges.
            // Build a deduplicated host list of selected edge indices from best_edge.
            {
                  // Download edge indices and full edge arrays to host
                  int* h_src = (int*)malloc((size_t)n_edges * sizeof(int));
                  int* h_dst = (int*)malloc((size_t)n_edges * sizeof(int));
                  hipMemcpy(h_src, edge_src, (size_t)n_edges * sizeof(int), hipMemcpyDeviceToHost);
                  hipMemcpy(h_dst, edge_dst, (size_t)n_edges * sizeof(int), hipMemcpyDeviceToHost);

                  // Collect unique selected edge indices
                  int* sel_src = (int*)malloc((size_t)n_nodes * sizeof(int));
                  int* sel_dst = (int*)malloc((size_t)n_nodes * sizeof(int));
                  int n_sel = 0;
                  // use a boolean visited array to deduplicate edge indices
                  unsigned char* seen = (unsigned char*)calloc(n_edges, 1);
                  for (int c = 0; c < n_nodes; c++) {
                        int e = h_best_edge[c];
                        if (e >= 0 && !seen[e]) {
                              seen[e] = 1;
                              sel_src[n_sel] = h_src[e];
                              sel_dst[n_sel] = h_dst[e];
                              n_sel++;
                        }
                  }
                  free(seen);
                  free(h_src);
                  free(h_dst);

                  if (n_sel > 0) {
                        // Upload selected edges and run UF only on them
                        int* d_sel_src = nullptr;
                        int* d_sel_dst = nullptr;
                        hipMalloc((void**)&d_sel_src, (size_t)n_sel * sizeof(int));
                        hipMalloc((void**)&d_sel_dst, (size_t)n_sel * sizeof(int));
                        hipMemcpy(d_sel_src, sel_src, (size_t)n_sel * sizeof(int), hipMemcpyHostToDevice);
                        hipMemcpy(d_sel_dst, sel_dst, (size_t)n_sel * sizeof(int), hipMemcpyHostToDevice);

                        int thr_sel = 256;
                        int blk_sel = (n_sel + thr_sel - 1) / thr_sel;
                        for (int iter = 0; iter < n_nodes + 1; iter++) {
                              hipMemset(d_changed, 0, sizeof(int));
                              hipLaunchKernelGGL(uf_hook_kernel, dim3(blk_sel), dim3(thr_sel), 0, stream,
                                    d_sel_src, d_sel_dst, parent, d_changed, n_sel);
                              hipLaunchKernelGGL(uf_compress_kernel, dim3(blocks_n), dim3(threads_n), 0, stream,
                                    parent, n_nodes);
                              hipStreamSynchronize(stream);
                              int h_changed = 0;
                              hipMemcpy(&h_changed, d_changed, sizeof(int), hipMemcpyDeviceToHost);
                              if (h_changed == 0) break;
                        }
                        hipFree(d_sel_src);
                        hipFree(d_sel_dst);
                  }
                  free(sel_src);
                  free(sel_dst);
            }
            free(h_best_edge);
      }

      // Compute total MST weight on host
      unsigned char* h_in_mst = (unsigned char*)malloc((size_t)n_edges * sizeof(unsigned char));
      double* h_w = (double*)malloc((size_t)n_edges * sizeof(double));
      hipMemcpy(h_in_mst, in_mst, (size_t)n_edges * sizeof(unsigned char), hipMemcpyDeviceToHost);
      hipMemcpy(h_w, edge_w, (size_t)n_edges * sizeof(double), hipMemcpyDeviceToHost);
      double total = 0.0;
      for (int e = 0; e < n_edges; e++) if (h_in_mst[e]) total += h_w[e];
      free(h_in_mst);
      free(h_w);
      // Write total weight to single-element output
      hipMemcpy(total_weight_buf, &total, sizeof(double), hipMemcpyHostToDevice);

      hipFree(parent);
      hipFree(best_edge);
      hipFree(best_w);
      hipFree(d_changed);
}

// ── Core distance: distance to min_pts-th nearest neighbor ─────────────────
// For each point i: compute all n distances, partial-sort to find the min_pts-th.
// One thread per point; inner loop over all j.
// This is O(n^2) but fully parallel across points.

extern "C" __global__ void core_distance_kernel(
      const double* points, double* core_dist,
      int n, int dim, int min_pts
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;

      // Partial selection: maintain a max-heap of size min_pts in registers.
      // Since min_pts can be large, use a simple insertion-sort style buffer in local array.
      // To avoid VLA/dynamic alloc on device, we use a fixed-size approach:
      // walk all j, maintain the min_pts smallest distances seen so far.
      // Implemented as a sorted array: track the max of the k-smallest.

      // We need min_pts elements. On GPU we can't allocate dynamic arrays,
      // so we use an outer loop approach: find the t-th order statistic via
      // counting how many distances are strictly less than a candidate.
      // Instead, we walk j and maintain a fixed local buffer of size min_pts
      // using a running max trick. This is O(n * min_pts) per point in the worst case.
      // For correctness without fixed-size registers, we implement a simple selection:
      // Walk j once to count distances, then find the min_pts-th smallest via
      // a threshold binary search would need shared memory. Use the direct O(n) pass
      // with a small in-register partial heap via a fixed upper bound cap.

      // Practical approach: one pass storing the max-of-kept in a register,
      // replacing when a smaller value is found and the buffer is not full.
      // We cap the in-register buffer at min_pts using atomics on shared memory would
      // require dynamic sizing. Since we need correctness, use two-pass:
      // Pass 1: find the min_pts-th smallest distance via partition.
      // Pass 2: done in one pass with running max.

      // Implementation: maintain running buffer of min_pts smallest via insertion.
      // Since min_pts is a runtime param, we must use shared memory or accept O(n*min_pts).
      // We allocate shared memory via extern __shared__ below — but this kernel uses
      // dynamic shared memory per block, which we pass as 0 (so not available here).
      // Fallback: use a simple O(n * min_pts) approach with a local max variable.
      // This is correct for any min_pts.

      // Use: keep a running "kth_max" = largest of the k-smallest seen so far.
      // count_le = number of distances <= kth_max.
      // On each new distance d:
      //   if d < kth_max → replace kth_max with d, scan again if needed.
      // Since a full re-scan is too expensive, we do:
      // 1. First pass: collect the true min_pts-th order stat using O(n) per pivot selection.
      // We use a simplified approach: find max of the (min_pts) smallest via one linear scan
      // with a tournament — just track the kth smallest directly.

      // Simplest correct O(n * min_pts) approach: iterate min_pts times to peel minimums.
      // This is too slow for large min_pts. For correctness we use a two-pass approach:
      // Pass 1: find the min_pts-th smallest distance d_k.
      // We use the standard "find kth smallest in unsorted array" via repeated partial min:

      // Practical O(n log n) alternative: sort distances. But no GPU sort here.
      // We implement: single pass maintaining a max-register of the smallest min_pts seen.
      // Use a fixed-size local array in registers. HIP allows local arrays (goes to VGPRs).

      // Local array approach — declared with a compile-time max. We use min_pts directly
      // via a simple linear scan maintaining the top-k as a sorted local buffer.
      // For correctness with arbitrary min_pts, we use the following approach:
      // Track a "threshold" = the largest of the k-smallest seen.
      // Initialize: scan first min_pts points to fill. Then for each remaining point,
      // if dist < threshold, do a linear scan in our local list to replace the max.
      // This is O(n * min_pts) worst case but O(n) if data is well-distributed.

      // Since GPU register files are limited, we use a hybrid:
      // keep a running max in a register, and track count of values <= max.
      // If count > min_pts, shrink threshold. This is the QuickSelect idea but single-pass.
      // For correctness: use a two-pass approach without local arrays:

      // PASS 1: find a threshold T such that exactly min_pts distances <= T.
      // We do this by: first scan to find global min (d_min), global max (d_max),
      // then binary search on T in [d_min, d_max] — but binary search needs log(n) passes.
      // Too many global reads.

      // FINAL CORRECT APPROACH: Use a partial sort register array.
      // On RDNA3, each thread has ~256 VGPRs. We allocate a local array of size min_pts
      // in local (L0 cache backed) memory. For moderate min_pts this is feasible.
      // We cap at min_pts via the algorithm only — no fixed compile-time limit.

      // Implementation: maintain sorted local buffer of the min_pts smallest distances.
      // Insert each distance into the sorted buffer (insertion sort style).
      // The core distance = buffer[min_pts - 1] at the end.

      // To avoid needing a compile-time size, we use the following single-pass algorithm:
      // Maintain kth_max = infinity, and a count of elements inserted.
      // Use a simple unsorted array stored in local memory (goes to LDS or scratch).
      // Since we can't dynamically allocate per-thread local arrays of runtime size in HIP,
      // we use the following 2-pass approach without local arrays:

      // PASS 1: find true kth smallest distance d_k.
      //   - find global min d_min, find global max d_max over all j!=i
      //   - binary search T in [d_min, d_max]:
      //     count = #{j : dist(i,j) <= T, j!=i}
      //     find smallest T where count >= min_pts
      //   - Need log2(n) binary search iterations, each O(n). Total O(n log n) per point.
      //   - Feasible for the problem.

      // We implement this:
      double d_min = DBL_MAX, d_max = 0.0;
      for (int j = 0; j < n; j++) {
            if (j == i) continue;
            double d2 = 0.0;
            for (int d = 0; d < dim; d++) {
                  double diff = points[i * dim + d] - points[j * dim + d];
                  d2 += diff * diff;
            }
            double dist = sqrt(d2);
            if (dist < d_min) d_min = dist;
            if (dist > d_max) d_max = dist;
      }

      if (min_pts <= 1) {
            core_dist[i] = d_min;
            return;
      }

      // Binary search for the min_pts-th smallest distance
      double lo = d_min, hi = d_max;
      for (int iter = 0; iter < 64; iter++) {
            double mid = lo + (hi - lo) * 0.5;
            int cnt = 0;
            for (int j = 0; j < n; j++) {
                  if (j == i) continue;
                  double d2 = 0.0;
                  for (int d = 0; d < dim; d++) {
                        double diff = points[i * dim + d] - points[j * dim + d];
                        d2 += diff * diff;
                  }
                  if (sqrt(d2) <= mid) cnt++;
            }
            if (cnt >= min_pts) hi = mid;
            else lo = mid;
            if (hi - lo < 1e-12) break;
      }
      core_dist[i] = hi;
}

extern "C" void launch_core_distance(
      const double* points, double* core_dist,
      int n, int dim, int min_pts, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(core_distance_kernel, dim3(blocks), dim3(threads), 0, stream,
            points, core_dist, n, dim, min_pts);
}
