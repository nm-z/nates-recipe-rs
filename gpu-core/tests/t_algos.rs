use gpu_core::memory::GpuBuffer;
use gpu_core::{graph, sequence, cluster};

const EPS: f64 = 1e-5;

fn close(a: f64, b: f64) -> bool {
      (a - b).abs() < EPS || (a - b).abs() < EPS * a.abs().max(b.abs()).max(1.0)
}

// ── graph: CSR SpMV ────────────────────────────────────────────────────────

#[test]
fn test_csr_spmv() {
      // 3x3 sparse A:
      //   row 0: (0,1.0),(1,2.0)
      //   row 1: (1,3.0),(2,4.0)
      //   row 2: (0,5.0)
      // x=[1,2,3] → y=[5,18,5]
      let values_h:  [f64; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
      let col_idx_h: [i32; 5] = [0, 1, 1, 2, 0];
      let row_ptr_h: [i32; 4] = [0, 2, 4, 5];
      let x_h:       [f64; 3] = [1.0, 2.0, 3.0];

      let values  = GpuBuffer::upload(&values_h).unwrap();
      let col_idx = GpuBuffer::upload_i32(&col_idx_h).unwrap();
      let row_ptr = GpuBuffer::upload_i32(&row_ptr_h).unwrap();
      let x       = GpuBuffer::upload(&x_h).unwrap();

      let y_gpu = graph::gpu_csr_spmv(&values, &col_idx, &row_ptr, &x, 3).unwrap();

      let mut y = [0.0f64; 3];
      y_gpu.download(&mut y).unwrap();

      println!("SpMV y = {:?}", y);
      assert!(y.iter().all(|v| v.is_finite()), "NaN/Inf in SpMV output");
      assert!(close(y[0], 5.0),  "SpMV y[0]: expected 5.0, got {}", y[0]);
      assert!(close(y[1], 18.0), "SpMV y[1]: expected 18.0, got {}", y[1]);
      assert!(close(y[2], 5.0),  "SpMV y[2]: expected 5.0, got {}", y[2]);
}

// ── graph: CSR SpMM ────────────────────────────────────────────────────────

#[test]
fn test_csr_spmm() {
      // Same A, B = [[1,2],[3,4],[5,6]] (3x2 row-major)
      // C[0] = [7,10], C[1] = [29,36], C[2] = [5,10]
      let values_h:  [f64; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
      let col_idx_h: [i32; 5] = [0, 1, 1, 2, 0];
      let row_ptr_h: [i32; 4] = [0, 2, 4, 5];
      let b_h: [f64; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

      let values  = GpuBuffer::upload(&values_h).unwrap();
      let col_idx = GpuBuffer::upload_i32(&col_idx_h).unwrap();
      let row_ptr = GpuBuffer::upload_i32(&row_ptr_h).unwrap();
      let b       = GpuBuffer::upload(&b_h).unwrap();

      let c_gpu = graph::gpu_csr_spmm(&values, &col_idx, &row_ptr, &b, 3, 2).unwrap();

      let mut c = [0.0f64; 6];
      c_gpu.download(&mut c).unwrap();

      println!("SpMM C = {:?}", c);
      let expected = [7.0, 10.0, 29.0, 36.0, 5.0, 10.0];
      assert!(c.iter().all(|v| v.is_finite()), "NaN/Inf in SpMM output");
      for i in 0..6 {
            assert!(close(c[i], expected[i]),
                  "SpMM C[{}]: expected {}, got {}", i, expected[i], c[i]);
      }
}

// ── graph: neighbor_aggregate (sum) ───────────────────────────────────────

#[test]
fn test_neighbor_aggregate_sum() {
      // 4 nodes, edges: 0->1, 2->1, 2->3 (node 1 has in-degree 2)
      // features: node0=[1,2], node1=[3,4], node2=[5,6], node3=[7,8]
      // agg[0]=[0,0], agg[1]=[6,8], agg[2]=[0,0], agg[3]=[5,6]
      let features_h: [f64; 8] = [1.0,2.0, 3.0,4.0, 5.0,6.0, 7.0,8.0];
      let edge_src_h: [i32; 3] = [0, 2, 2];
      let edge_dst_h: [i32; 3] = [1, 1, 3];

      let features = GpuBuffer::upload(&features_h).unwrap();
      let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
      let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();

      let agg_gpu = graph::gpu_neighbor_aggregate(&features, &edge_src, &edge_dst, 4, 2, 3, false).unwrap();

      let mut agg = [0.0f64; 8];
      agg_gpu.download(&mut agg).unwrap();

      println!("NeighborAgg sum = {:?}", agg);
      let expected = [0.0,0.0, 6.0,8.0, 0.0,0.0, 5.0,6.0];
      assert!(agg.iter().all(|v| v.is_finite()), "NaN/Inf in neighbor_agg sum");
      for i in 0..8 {
            assert!(close(agg[i], expected[i]),
                  "NeighborAgg sum[{}]: expected {}, got {}", i, expected[i], agg[i]);
      }
}

// ── graph: neighbor_aggregate (mean) ──────────────────────────────────────

#[test]
fn test_neighbor_aggregate_mean() {
      // Same graph: 0->1, 2->1, 2->3
      // agg[1] = [6/2, 8/2] = [3, 4] (in-degree 2)
      // agg[3] = [5/1, 6/1] = [5, 6]
      let features_h: [f64; 8] = [1.0,2.0, 3.0,4.0, 5.0,6.0, 7.0,8.0];
      let edge_src_h: [i32; 3] = [0, 2, 2];
      let edge_dst_h: [i32; 3] = [1, 1, 3];

      let features = GpuBuffer::upload(&features_h).unwrap();
      let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
      let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();

      let agg_gpu = graph::gpu_neighbor_aggregate(&features, &edge_src, &edge_dst, 4, 2, 3, true).unwrap();

      let mut agg = [0.0f64; 8];
      agg_gpu.download(&mut agg).unwrap();

      println!("NeighborAgg mean = {:?}", agg);
      let expected = [0.0,0.0, 3.0,4.0, 0.0,0.0, 5.0,6.0];
      assert!(agg.iter().all(|v| v.is_finite()), "NaN/Inf in neighbor_agg mean");
      for i in 0..8 {
            assert!(close(agg[i], expected[i]),
                  "NeighborAgg mean[{}]: expected {}, got {}", i, expected[i], agg[i]);
      }
}

// ── graph: gpu_degree ─────────────────────────────────────────────────────

#[test]
fn test_gpu_degree() {
      // Edges 0->1, 2->1, 2->3 → dst=[1,1,3]
      // deg[0]=0, deg[1]=2, deg[2]=0, deg[3]=1
      let edge_dst_h: [i32; 3] = [1, 1, 3];

      let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();
      let deg_gpu = graph::gpu_degree(&edge_dst, 4, 3).unwrap();

      let mut deg = [0.0f64; 4];
      deg_gpu.download(&mut deg).unwrap();

      println!("Degree = {:?}", deg);
      let expected = [0.0, 2.0, 0.0, 1.0];
      assert!(deg.iter().all(|v| v.is_finite()), "NaN/Inf in degree");
      for i in 0..4 {
            assert!(close(deg[i], expected[i]),
                  "Degree[{}]: expected {}, got {}", i, expected[i], deg[i]);
      }
}

// ── graph: gpu_gcn_norm ───────────────────────────────────────────────────

#[test]
fn test_gpu_gcn_norm() {
      // features = [[1,0],[2,0],[3,0],[4,0]], deg = [1,4,9,16]
      // scale[i] = 1/sqrt(deg[i])
      // result: [[1,0],[1,0],[1,0],[1,0]]
      let mut features_h: [f64; 8] = [1.0,0.0, 2.0,0.0, 3.0,0.0, 4.0,0.0];
      let deg_h: [f64; 4] = [1.0, 4.0, 9.0, 16.0];

      let features = GpuBuffer::upload(&features_h).unwrap();
      let deg      = GpuBuffer::upload(&deg_h).unwrap();

      graph::gpu_gcn_norm(&features, &deg, 4, 2).unwrap();

      features.download(&mut features_h).unwrap();

      println!("GCN norm features = {:?}", features_h);
      assert!(features_h.iter().all(|v| v.is_finite()), "NaN/Inf in gcn_norm");
      // every row's first column should be 1.0 (n*1/sqrt(n^2) = 1)
      // col 0: 1*1=1, 2*0.5=1, 3*(1/3)=1, 4*0.25=1
      for i in 0..4 {
            assert!(close(features_h[i*2], 1.0),
                  "GCN norm node{} col0: expected 1.0, got {}", i, features_h[i*2]);
            assert!(close(features_h[i*2+1], 0.0),
                  "GCN norm node{} col1: expected 0.0, got {}", i, features_h[i*2+1]);
      }
}

// ── sequence: forward-backward ────────────────────────────────────────────

#[test]
fn test_forward_backward() {
      // 2-state HMM, T=3
      // log_trans[s*2+s2]: 0->0=log(0.7), 0->1=log(0.3), 1->0=log(0.4), 1->1=log(0.6)
      // log_emit[t*2+s]: t=0 obs=0, t=1 obs=1, t=2 obs=0
      //   s=0: emit(0)=0.9, emit(1)=0.1; s=1: emit(0)=0.2, emit(1)=0.8
      let log_trans_h: [f64; 4] = [
            0.7f64.ln(), 0.3f64.ln(),
            0.4f64.ln(), 0.6f64.ln(),
      ];
      let log_emit_h: [f64; 6] = [
            0.9f64.ln(), 0.2f64.ln(),
            0.1f64.ln(), 0.8f64.ln(),
            0.9f64.ln(), 0.2f64.ln(),
      ];

      let log_trans = GpuBuffer::upload(&log_trans_h).unwrap();
      let log_emit  = GpuBuffer::upload(&log_emit_h).unwrap();

      let (alpha_gpu, beta_gpu, gamma_gpu) =
            sequence::gpu_forward_backward(&log_trans, &log_emit, 2, 3).unwrap();

      let mut log_alpha = [0.0f64; 6];
      let mut log_beta  = [0.0f64; 6];
      let mut log_gamma = [0.0f64; 6];

      alpha_gpu.download(&mut log_alpha).unwrap();
      beta_gpu.download(&mut log_beta).unwrap();
      gamma_gpu.download(&mut log_gamma).unwrap();

      println!("log_alpha = {:?}", log_alpha);
      println!("log_beta  = {:?}", log_beta);
      println!("log_gamma = {:?}", log_gamma);

      assert!(log_alpha.iter().all(|v| v.is_finite()), "NaN/Inf in log_alpha");
      assert!(log_beta.iter().all(|v| v.is_finite()),  "NaN/Inf in log_beta");
      assert!(log_gamma.iter().all(|v| v.is_finite()), "NaN/Inf in log_gamma");

      // CPU reference (computed with Python logsumexp):
      let ref_alpha: [f64; 6] = [
            -0.798508, -2.302585,
            -3.338223, -1.857899,
            -2.544338, -3.870401,
      ];
      let ref_beta: [f64; 6] = [
            -1.810942, -1.354796,
            -0.371064, -0.733969,
             0.0,       0.0,
      ];
      let ref_gamma: [f64; 6] = [
            -0.300595, -1.348526,
            -1.400432, -0.283014,
            -0.235484, -1.561547,
      ];

      for i in 0..6 {
            assert!(close(log_alpha[i], ref_alpha[i]),
                  "log_alpha[{}]: expected {:.6}, got {:.6}", i, ref_alpha[i], log_alpha[i]);
      }
      for i in 0..6 {
            assert!(close(log_beta[i], ref_beta[i]),
                  "log_beta[{}]: expected {:.6}, got {:.6}", i, ref_beta[i], log_beta[i]);
      }
      // gamma: check that each timestep's probabilities sum to 1
      for t in 0..3 {
            let p0 = log_gamma[t*2].exp();
            let p1 = log_gamma[t*2+1].exp();
            assert!((p0 + p1 - 1.0).abs() < 1e-6,
                  "gamma t={} prob sum: expected 1.0, got {}", t, p0 + p1);
      }
      // check actual gamma values against reference
      for i in 0..6 {
            assert!(close(log_gamma[i], ref_gamma[i]),
                  "log_gamma[{}]: expected {:.6}, got {:.6}", i, ref_gamma[i], log_gamma[i]);
      }
}

// ── sequence: viterbi ─────────────────────────────────────────────────────

#[test]
fn test_viterbi() {
      // Same HMM. Best path brute-forced over all 2^3=8 paths: [0, 1, 0]
      let log_trans_h: [f64; 4] = [
            0.7f64.ln(), 0.3f64.ln(),
            0.4f64.ln(), 0.6f64.ln(),
      ];
      let log_emit_h: [f64; 6] = [
            0.9f64.ln(), 0.2f64.ln(),
            0.1f64.ln(), 0.8f64.ln(),
            0.9f64.ln(), 0.2f64.ln(),
      ];

      let log_trans = GpuBuffer::upload(&log_trans_h).unwrap();
      let log_emit  = GpuBuffer::upload(&log_emit_h).unwrap();

      let path_gpu = sequence::gpu_viterbi(&log_trans, &log_emit, 2, 3).unwrap();

      let mut path = [0i32; 3];
      path_gpu.download_i32(&mut path).unwrap();

      println!("Viterbi best_path = {:?}", path);
      // Brute-force best path is [0, 1, 0]
      assert_eq!(path[0], 0, "Viterbi path[0]: expected 0, got {}", path[0]);
      assert_eq!(path[1], 1, "Viterbi path[1]: expected 1, got {}", path[1]);
      assert_eq!(path[2], 0, "Viterbi path[2]: expected 0, got {}", path[2]);
}

// ── cluster: fixed_radius_neighbors ──────────────────────────────────────

#[test]
fn test_fixed_radius_neighbors() {
      // 4 points in 2D: (0,0),(1,0),(5,0),(6,0), eps=1.5
      // Points 0&1 are within eps of each other, 2&3 are within eps of each other.
      // mask[i*4+j] = 1 if dist(i,j)<=eps (includes self i==i)
      // count[i] counts neighbors including self
      let points_h: [f64; 8] = [0.0,0.0, 1.0,0.0, 5.0,0.0, 6.0,0.0];

      let points = GpuBuffer::upload(&points_h).unwrap();

      let result = cluster::gpu_fixed_radius_neighbors(&points, 4, 2, 1.5).unwrap();

      let mut mask  = [0u8; 16];
      let mut count = [0i32; 4];

      result.within_mask.download_u8(&mut mask).unwrap();
      result.neighbor_count.download_i32(&mut count).unwrap();

      println!("FRN mask  = {:?}", mask);
      println!("FRN count = {:?}", count);

      // Expected mask (1 = within eps including self):
      // row 0: [1,1,0,0], row 1: [1,1,0,0], row 2: [0,0,1,1], row 3: [0,0,1,1]
      let expected_mask: [u8; 16] = [1,1,0,0, 1,1,0,0, 0,0,1,1, 0,0,1,1];
      for i in 0..16 {
            assert_eq!(mask[i], expected_mask[i],
                  "FRN mask[{}]: expected {}, got {}", i, expected_mask[i], mask[i]);
      }
      // count: each point has exactly 2 neighbors (self + partner)
      for i in 0..4 {
            assert_eq!(count[i], 2,
                  "FRN count[{}]: expected 2, got {}", i, count[i]);
      }
}

// ── cluster: union_find_cc ────────────────────────────────────────────────

#[test]
fn test_union_find_cc() {
      // 5 nodes, 2 components:
      //   Component A: 0,1,2 (edges 0-1, 1-2)
      //   Component B: 3,4   (edge  3-4)
      let edge_src_h: [i32; 3] = [0, 1, 3];
      let edge_dst_h: [i32; 3] = [1, 2, 4];

      let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
      let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();

      let labels_gpu = cluster::gpu_union_find_cc(&edge_src, &edge_dst, 5, 3).unwrap();

      let mut parent = [0i32; 5];
      labels_gpu.download_i32(&mut parent).unwrap();

      println!("UF parent = {:?}", parent);

      // After path compression: all nodes in same component share the same root.
      // Don't hard-code which root wins — just check partition structure.
      assert_eq!(parent[0], parent[1],
            "UF: nodes 0 and 1 should be in same component, got roots {} and {}", parent[0], parent[1]);
      assert_eq!(parent[1], parent[2],
            "UF: nodes 1 and 2 should be in same component, got roots {} and {}", parent[1], parent[2]);
      assert_eq!(parent[3], parent[4],
            "UF: nodes 3 and 4 should be in same component, got roots {} and {}", parent[3], parent[4]);
      assert_ne!(parent[0], parent[3],
            "UF: nodes 0 and 3 should be in different components, both got root {}", parent[0]);

      // Exactly 2 distinct component labels
      let mut roots: std::collections::HashSet<i32> = std::collections::HashSet::new();
      for &p in &parent { roots.insert(p); }
      assert_eq!(roots.len(), 2, "UF: expected 2 components, got {} distinct roots: {:?}", roots.len(), roots);
}

// ── cluster: boruvka_mst ──────────────────────────────────────────────────

#[test]
fn test_boruvka_mst() {
      // 4 nodes, 4 undirected edges (each stored once as directed):
      //   edge 0: 0->1 w=1.0
      //   edge 1: 1->2 w=5.0
      //   edge 2: 2->3 w=2.0
      //   edge 3: 0->3 w=10.0
      // Kruskal MST: pick (0,1,1), (2,3,2), (1,2,5) = total 8.0
      // The boruvka kernel runs UF on ALL edges (not just MST edges) between rounds,
      // which may over-merge. Report the actual weight as a finding if != 8.0.
      let edge_src_h: [i32; 4] = [0, 1, 2, 0];
      let edge_dst_h: [i32; 4] = [1, 2, 3, 3];
      let edge_w_h:   [f64; 4] = [1.0, 5.0, 2.0, 10.0];

      let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
      let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();
      let edge_w   = GpuBuffer::upload(&edge_w_h).unwrap();

      let result = cluster::gpu_boruvka_mst(&edge_src, &edge_dst, &edge_w, 4, 4).unwrap();

      let mut in_mst = [0u8; 4];
      result.in_mst.download_u8(&mut in_mst).unwrap();

      println!("Boruvka in_mst = {:?}", in_mst);
      println!("Boruvka total_weight = {}", result.total_weight);

      assert!(result.total_weight.is_finite(), "NaN/Inf in Boruvka total_weight");

      // Expected MST weight = 8.0 (Kruskal reference)
      assert!(close(result.total_weight, 8.0),
            "FINDING: Boruvka total_weight: expected 8.0, got {}. Likely over-merging bug \
             (UF step uses all edges, not just MST edges)", result.total_weight);

      // MST should have exactly n_nodes-1 = 3 edges
      let mst_count = in_mst.iter().filter(|&&b| b != 0).count();
      assert_eq!(mst_count, 3,
            "FINDING: Boruvka MST edge count: expected 3, got {}. in_mst={:?}", mst_count, in_mst);
}

// ── cluster: core_distance ────────────────────────────────────────────────

#[test]
fn test_core_distance() {
      // 4 points in 1D: [0.0, 1.0, 2.0, 10.0], min_pts=2
      // Core dist = 2nd smallest distance to other points (excluding self)
      // point 0: dists=[1,2,10] → 2nd = 2.0
      // point 1: dists=[1,1,9]  → 2nd = 1.0
      // point 2: dists=[1,2,8]  → 2nd = 2.0
      // point 3: dists=[8,9,10] → 2nd = 9.0
      let points_h: [f64; 4] = [0.0, 1.0, 2.0, 10.0];

      let points = GpuBuffer::upload(&points_h).unwrap();
      let core_dist_gpu = cluster::gpu_core_distance(&points, 4, 1, 2).unwrap();

      let mut cd = [0.0f64; 4];
      core_dist_gpu.download(&mut cd).unwrap();

      println!("core_distance = {:?}", cd);
      assert!(cd.iter().all(|v| v.is_finite()), "NaN/Inf in core_distance");

      let expected = [2.0, 1.0, 2.0, 9.0];
      for i in 0..4 {
            assert!(close(cd[i], expected[i]),
                  "core_distance[{}]: expected {}, got {}", i, expected[i], cd[i]);
      }
}
