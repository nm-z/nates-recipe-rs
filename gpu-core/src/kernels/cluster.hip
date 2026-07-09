#include <hip/hip_runtime.h>
#include <float.h>


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
      int total = n * n;
      int threads = 256;
      int blocks = (total + threads - 1) / threads;
      hipLaunchKernelGGL(fixed_radius_kernel, dim3(blocks), dim3(threads), 0, stream,
            points, mask, count, n, dim, eps2);
}


extern "C" __global__ void uf_init_kernel(int* parent, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) parent[i] = i;
}


extern "C" __global__ void uf_hook_kernel(
      const int* src, const int* dst, int* parent, int* changed, int n_edges
) {
      int e = blockIdx.x * blockDim.x + threadIdx.x;
      if (e >= n_edges) return;
      int u = src[e];
      int v = dst[e];
      while (parent[u] != u) u = parent[u];
      while (parent[v] != v) v = parent[v];
      if (u == v) return;
      int lo = u < v ? u : v;
      int hi = u < v ? v : u;
      int old = atomicCAS(&parent[lo], lo, hi);
      if (old == lo) atomicAdd(changed, 1);
}


extern "C" __global__ void uf_compress_kernel(int* parent, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      int root = i;
      while (parent[root] != root) root = parent[root];
      parent[i] = root;
}



extern "C" void launch_uf_init(
      int* parent, int n_nodes, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_nodes + threads - 1) / threads;
      hipLaunchKernelGGL(uf_init_kernel, dim3(blocks), dim3(threads), 0, stream,
            parent, n_nodes);
}

extern "C" void launch_uf_hook(
      const int* edge_src, const int* edge_dst, int* parent, int* changed,
      int n_edges, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_edges + threads - 1) / threads;
      hipLaunchKernelGGL(uf_hook_kernel, dim3(blocks), dim3(threads), 0, stream,
            edge_src, edge_dst, parent, changed, n_edges);
}

extern "C" void launch_uf_compress(
      int* parent, int n_nodes, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_nodes + threads - 1) / threads;
      hipLaunchKernelGGL(uf_compress_kernel, dim3(blocks), dim3(threads), 0, stream,
            parent, n_nodes);
}


__device__ static unsigned long long boruvka_wkey(double w) {
      unsigned long long b = __double_as_longlong(w);
      return (b & 0x8000000000000000ull) ? ~b : (b | 0x8000000000000000ull);
}

extern "C" __global__ void boruvka_init_kernel(
      int* best_edge, unsigned long long* best_wkey, int n_nodes
) {
      int c = blockIdx.x * blockDim.x + threadIdx.x;
      if (c >= n_nodes) return;
      best_edge[c] = 0x7FFFFFFF;
      best_wkey[c] = ~0ull;
}

extern "C" __global__ void boruvka_min_w_kernel(
      const int* src, const int* dst, const double* w,
      const int* parent, unsigned long long* best_wkey,
      int n_edges
) {
      int e = blockIdx.x * blockDim.x + threadIdx.x;
      if (e >= n_edges) return;
      int cu = parent[src[e]], cv = parent[dst[e]];
      if (cu == cv) return;
      unsigned long long k = boruvka_wkey(w[e]);
      atomicMin(&best_wkey[cu], k);
      atomicMin(&best_wkey[cv], k);
}

extern "C" __global__ void boruvka_min_e_kernel(
      const int* src, const int* dst, const double* w,
      const int* parent, const unsigned long long* best_wkey, int* best_edge,
      int n_edges
) {
      int e = blockIdx.x * blockDim.x + threadIdx.x;
      if (e >= n_edges) return;
      int cu = parent[src[e]], cv = parent[dst[e]];
      if (cu == cv) return;
      unsigned long long k = boruvka_wkey(w[e]);
      if (k == best_wkey[cu]) atomicMin(&best_edge[cu], e);
      if (k == best_wkey[cv]) atomicMin(&best_edge[cv], e);
}

extern "C" __global__ void boruvka_mark_kernel(
      const int* best_edge, unsigned char* in_mst, int n_nodes, int n_edges
) {
      int c = blockIdx.x * blockDim.x + threadIdx.x;
      if (c >= n_nodes) return;
      int e = best_edge[c];
      if (e >= 0 && e < n_edges) in_mst[e] = 1;
}


extern "C" void launch_boruvka_init(
      int* best_edge, unsigned long long* best_wkey, int n_nodes, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_nodes + threads - 1) / threads;
      hipLaunchKernelGGL(boruvka_init_kernel, dim3(blocks), dim3(threads), 0, stream,
            best_edge, best_wkey, n_nodes);
}

extern "C" void launch_boruvka_min_w(
      const int* edge_src, const int* edge_dst, const double* edge_w,
      const int* parent, unsigned long long* best_wkey,
      int n_edges, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_edges + threads - 1) / threads;
      hipLaunchKernelGGL(boruvka_min_w_kernel, dim3(blocks), dim3(threads), 0, stream,
            edge_src, edge_dst, edge_w, parent, best_wkey, n_edges);
}

extern "C" void launch_boruvka_min_e(
      const int* edge_src, const int* edge_dst, const double* edge_w,
      const int* parent, const unsigned long long* best_wkey, int* best_edge,
      int n_edges, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_edges + threads - 1) / threads;
      hipLaunchKernelGGL(boruvka_min_e_kernel, dim3(blocks), dim3(threads), 0, stream,
            edge_src, edge_dst, edge_w, parent, best_wkey, best_edge, n_edges);
}

extern "C" void launch_boruvka_mark(
      const int* best_edge, unsigned char* in_mst, int n_nodes, int n_edges, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n_nodes + threads - 1) / threads;
      hipLaunchKernelGGL(boruvka_mark_kernel, dim3(blocks), dim3(threads), 0, stream,
            best_edge, in_mst, n_nodes, n_edges);
}


extern "C" __global__ void core_distance_kernel(
      const double* points, double* core_dist,
      int n, int dim, int min_pts
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;















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


extern "C" __global__ void fixed_radius_count_kernel(
      const double* points, int* count, int n, int dim, const double* eps
) {
      double eps2 = (*eps) * (*eps);
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      int c = 0;
      for (int j = 0; j < n; j++) {
            double d2 = 0.0;
            for (int d = 0; d < dim; d++) {
                  double diff = points[i * dim + d] - points[j * dim + d];
                  d2 += diff * diff;
            }
            if (d2 <= eps2) c++;
      }
      count[i] = c;
}

extern "C" void launch_fixed_radius_count(
      const double* points, int* count, int n, int dim, const double* eps, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(fixed_radius_count_kernel, dim3(blocks), dim3(threads), 0, stream,
            points, count, n, dim, eps);
}

extern "C" __global__ void exclusive_scan_i32_kernel(
      const int* in, int* out, int n
) {
      if (blockIdx.x * blockDim.x + threadIdx.x != 0) return;
      int acc = 0;
      for (int i = 0; i < n; i++) { out[i] = acc; acc += in[i]; }
      out[n] = acc;
}

extern "C" void launch_exclusive_scan_i32(
      const int* in, int* out, int n, hipStream_t stream
) {
      hipLaunchKernelGGL(exclusive_scan_i32_kernel, dim3(1), dim3(1), 0, stream,
            in, out, n);
}

extern "C" __global__ void masked_weight_sum_kernel(
      const double* w, const unsigned char* in_mst, double* total, int n_edges
) {
      if (blockIdx.x * blockDim.x + threadIdx.x != 0) return;
      double s = 0.0;
      for (int e = 0; e < n_edges; e++) { if (in_mst[e]) s += w[e]; }
      total[0] = s;
}

extern "C" void launch_masked_weight_sum(
      const double* w, const unsigned char* in_mst, double* total, int n_edges, hipStream_t stream
) {
      hipLaunchKernelGGL(masked_weight_sum_kernel, dim3(1), dim3(1), 0, stream,
            w, in_mst, total, n_edges);
}

extern "C" __global__ void fixed_radius_fill_csr_kernel(
      const double* points, const int* row_ptr, int* indices, int n, int dim, const double* eps
) {
      double eps2 = (*eps) * (*eps);
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      int off = row_ptr[i];
      for (int j = 0; j < n; j++) {
            double d2 = 0.0;
            for (int d = 0; d < dim; d++) {
                  double diff = points[i * dim + d] - points[j * dim + d];
                  d2 += diff * diff;
            }
            if (d2 <= eps2) { indices[off++] = j; }
      }
}

extern "C" void launch_fixed_radius_fill_csr(
      const double* points, const int* row_ptr, int* indices, int n, int dim, const double* eps, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(fixed_radius_fill_csr_kernel, dim3(blocks), dim3(threads), 0, stream,
            points, row_ptr, indices, n, dim, eps);
}
