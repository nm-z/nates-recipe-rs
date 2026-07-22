use gpu_core::memory::GpuBuffer;
use gpu_core::{cluster, graph, sequence};
use std::collections::HashSet;
use std::f64::consts;
use std::mem;
use std::ptr;

const EPS: f64 = 1e-5;

fn close(a: f64, b: f64) -> bool {
	(a - b).abs() < EPS || (a - b).abs() < EPS * a.abs().max(b.abs()).max(1.0)
}

// ── graph: CSR SpMV ────────────────────────────────────────────────────────

#[test]
fn test_csr_spmv() {
	let values_h: [f64; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
	let col_idx_h: [i32; 5] = [0, 1, 1, 2, 0];
	let row_ptr_h: [i32; 4] = [0, 2, 4, 5];
	let x_h: [f64; 3] = [1.0, 2.0, 3.0];

	let values = {
		let __up = &values_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let col_idx = GpuBuffer::upload_i32(&col_idx_h).unwrap();
	let row_ptr = GpuBuffer::upload_i32(&row_ptr_h).unwrap();
	let x = {
		let __up = &x_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let y_gpu = GpuBuffer::alloc(3).unwrap();
	graph::gpu_csr_spmv(&values, &col_idx, &row_ptr, &x, 3, &y_gpu).unwrap();

	let mut y = [0.0f64; 3];
	unsafe { y_gpu.download_async(&mut y, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("SpMV y = {:?}", y);
	assert!(y.iter().all(|v| v.is_finite()), "NaN/Inf in SpMV output");
	assert!(close(y[0], 5.0), "SpMV y[0]: expected 5.0, got {}", y[0]);
	assert!(close(y[1], 18.0), "SpMV y[1]: expected 18.0, got {}", y[1]);
	assert!(close(y[2], 5.0), "SpMV y[2]: expected 5.0, got {}", y[2]);
}

// ── graph: CSR SpMM ────────────────────────────────────────────────────────

#[test]
fn test_csr_spmm() {
	let values_h: [f64; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
	let col_idx_h: [i32; 5] = [0, 1, 1, 2, 0];
	let row_ptr_h: [i32; 4] = [0, 2, 4, 5];
	let b_h: [f64; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

	let values = {
		let __up = &values_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let col_idx = GpuBuffer::upload_i32(&col_idx_h).unwrap();
	let row_ptr = GpuBuffer::upload_i32(&row_ptr_h).unwrap();
	let b = {
		let __up = &b_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let c_gpu = GpuBuffer::alloc(6).unwrap();
	graph::gpu_csr_spmm(&values, &col_idx, &row_ptr, &b, 3, 2, &c_gpu).unwrap();

	let mut c = [0.0f64; 6];
	unsafe { c_gpu.download_async(&mut c, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("SpMM C = {:?}", c);
	let expected = [7.0, 10.0, 29.0, 36.0, 5.0, 10.0];
	assert!(c.iter().all(|v| v.is_finite()), "NaN/Inf in SpMM output");
	for i in 0..6 {
		assert!(
			close(c[i], expected[i]),
			"SpMM C[{}]: expected {}, got {}",
			i,
			expected[i],
			c[i]
		);
	}
}

// ── graph: neighbor_aggregate (sum) ───────────────────────────────────────

#[test]
fn test_neighbor_aggregate_sum() {
	let features_h: [f64; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
	let edge_src_h: [i32; 3] = [0, 2, 2];
	let edge_dst_h: [i32; 3] = [1, 1, 3];

	let features = {
		let __up = &features_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
	let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();

	let deg_ws = GpuBuffer::alloc(4).unwrap();
	let agg_gpu = GpuBuffer::alloc(8).unwrap();
	graph::gpu_neighbor_aggregate(
		&features, &edge_src, &edge_dst, &deg_ws, 4, 2, 3, 0, &agg_gpu,
	)
	.unwrap();

	let mut agg = [0.0f64; 8];
	unsafe { agg_gpu.download_async(&mut agg, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("NeighborAgg sum = {:?}", agg);
	let expected = [0.0, 0.0, 6.0, 8.0, 0.0, 0.0, 5.0, 6.0];
	assert!(
		agg.iter().all(|v| v.is_finite()),
		"NaN/Inf in neighbor_agg sum"
	);
	for i in 0..8 {
		assert!(
			close(agg[i], expected[i]),
			"NeighborAgg sum[{}]: expected {}, got {}",
			i,
			expected[i],
			agg[i]
		);
	}
}

// ── graph: neighbor_aggregate (mean) ──────────────────────────────────────

#[test]
fn test_neighbor_aggregate_mean() {
	let features_h: [f64; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
	let edge_src_h: [i32; 3] = [0, 2, 2];
	let edge_dst_h: [i32; 3] = [1, 1, 3];

	let features = {
		let __up = &features_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
	let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();

	let deg_ws = GpuBuffer::alloc(4).unwrap();
	let agg_gpu = GpuBuffer::alloc(8).unwrap();
	graph::gpu_neighbor_aggregate(
		&features, &edge_src, &edge_dst, &deg_ws, 4, 2, 3, 1, &agg_gpu,
	)
	.unwrap();

	let mut agg = [0.0f64; 8];
	unsafe { agg_gpu.download_async(&mut agg, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("NeighborAgg mean = {:?}", agg);
	let expected = [0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 5.0, 6.0];
	assert!(
		agg.iter().all(|v| v.is_finite()),
		"NaN/Inf in neighbor_agg mean"
	);
	for i in 0..8 {
		assert!(
			close(agg[i], expected[i]),
			"NeighborAgg mean[{}]: expected {}, got {}",
			i,
			expected[i],
			agg[i]
		);
	}
}

// ── graph: gpu_degree ─────────────────────────────────────────────────────

#[test]
fn test_gpu_degree() {
	let edge_dst_h: [i32; 3] = [1, 1, 3];

	let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();
	let deg_gpu = GpuBuffer::alloc(4).unwrap();
	graph::gpu_degree(&edge_dst, 4, 3, &deg_gpu).unwrap();

	let mut deg = [0.0f64; 4];
	unsafe { deg_gpu.download_async(&mut deg, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("Degree = {:?}", deg);
	let expected = [0.0, 2.0, 0.0, 1.0];
	assert!(deg.iter().all(|v| v.is_finite()), "NaN/Inf in degree");
	for i in 0..4 {
		assert!(
			close(deg[i], expected[i]),
			"Degree[{}]: expected {}, got {}",
			i,
			expected[i],
			deg[i]
		);
	}
}

// ── graph: gpu_gcn_norm ───────────────────────────────────────────────────

#[test]
fn test_gpu_gcn_norm() {
	let mut features_h: [f64; 8] = [1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0];
	let deg_h: [f64; 4] = [1.0, 4.0, 9.0, 16.0];

	let features = {
		let __up = &features_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let deg = {
		let __up = &deg_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	graph::gpu_gcn_norm(&deg, 4, 2, &features).unwrap();

	unsafe { features.download_async(&mut features_h, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("GCN norm features = {:?}", features_h);
	assert!(
		features_h.iter().all(|v| v.is_finite()),
		"NaN/Inf in gcn_norm"
	);
	for i in 0..4 {
		assert!(
			close(features_h[i * 2], 1.0),
			"GCN norm node{} col0: expected 1.0, got {}",
			i,
			features_h[i * 2]
		);
		assert!(
			close(features_h[i * 2 + 1], 0.0),
			"GCN norm node{} col1: expected 0.0, got {}",
			i,
			features_h[i * 2 + 1]
		);
	}
}

// ── sequence: forward-backward ────────────────────────────────────────────

#[test]
fn test_forward_backward() {
	let log_trans_h: [f64; 4] = [0.7f64.ln(), 0.3f64.ln(), 0.4f64.ln(), 0.6f64.ln()];
	let log_emit_h: [f64; 6] = [
		0.9f64.ln(),
		0.2f64.ln(),
		0.1f64.ln(),
		0.8f64.ln(),
		0.9f64.ln(),
		0.2f64.ln(),
	];

	let log_trans = {
		let __up = &log_trans_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let log_emit = {
		let __up = &log_emit_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let alpha_gpu = GpuBuffer::alloc(6).unwrap();
	let beta_gpu = GpuBuffer::alloc(6).unwrap();
	let gamma_gpu = GpuBuffer::alloc(6).unwrap();
	sequence::gpu_forward_backward(
		&log_trans, &log_emit, 2, 3, &alpha_gpu, &beta_gpu, &gamma_gpu,
	)
	.unwrap();

	let mut log_alpha = [0.0f64; 6];
	let mut log_beta = [0.0f64; 6];
	let mut log_gamma = [0.0f64; 6];

	unsafe { alpha_gpu.download_async(&mut log_alpha, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { beta_gpu.download_async(&mut log_beta, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { gamma_gpu.download_async(&mut log_gamma, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("log_alpha = {:?}", log_alpha);
	println!("log_beta  = {:?}", log_beta);
	println!("log_gamma = {:?}", log_gamma);

	assert!(
		log_alpha.iter().all(|v| v.is_finite()),
		"NaN/Inf in log_alpha"
	);
	assert!(
		log_beta.iter().all(|v| v.is_finite()),
		"NaN/Inf in log_beta"
	);
	assert!(
		log_gamma.iter().all(|v| v.is_finite()),
		"NaN/Inf in log_gamma"
	);

	let ref_alpha: [f64; 6] = [
		-0.798508,
		-consts::LN_10,
		-3.338223,
		-1.857899,
		-2.544338,
		-3.870401,
	];
	let ref_beta: [f64; 6] = [-1.810942, -1.354796, -0.371064, -0.733969, 0.0, 0.0];
	let ref_gamma: [f64; 6] = [
		-0.300595, -1.348526, -1.400432, -0.283014, -0.235484, -1.561547,
	];

	for i in 0..6 {
		assert!(
			close(log_alpha[i], ref_alpha[i]),
			"log_alpha[{}]: expected {:.6}, got {:.6}",
			i,
			ref_alpha[i],
			log_alpha[i]
		);
	}
	for i in 0..6 {
		assert!(
			close(log_beta[i], ref_beta[i]),
			"log_beta[{}]: expected {:.6}, got {:.6}",
			i,
			ref_beta[i],
			log_beta[i]
		);
	}
	for t in 0..3 {
		let p0 = log_gamma[t * 2].exp();
		let p1 = log_gamma[t * 2 + 1].exp();
		assert!(
			(p0 + p1 - 1.0).abs() < 1e-6,
			"gamma t={} prob sum: expected 1.0, got {}",
			t,
			p0 + p1
		);
	}
	for i in 0..6 {
		assert!(
			close(log_gamma[i], ref_gamma[i]),
			"log_gamma[{}]: expected {:.6}, got {:.6}",
			i,
			ref_gamma[i],
			log_gamma[i]
		);
	}
}

// ── sequence: viterbi ─────────────────────────────────────────────────────

#[test]
fn test_viterbi() {
	let log_trans_h: [f64; 4] = [0.7f64.ln(), 0.3f64.ln(), 0.4f64.ln(), 0.6f64.ln()];
	let log_emit_h: [f64; 6] = [
		0.9f64.ln(),
		0.2f64.ln(),
		0.1f64.ln(),
		0.8f64.ln(),
		0.9f64.ln(),
		0.2f64.ln(),
	];

	let log_trans = {
		let __up = &log_trans_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let log_emit = {
		let __up = &log_emit_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let delta = GpuBuffer::alloc(6).unwrap();
	let backptr = GpuBuffer::alloc_bytes(6 * mem::size_of::<i32>()).unwrap();
	let path_gpu = GpuBuffer::alloc_bytes(3 * mem::size_of::<i32>()).unwrap();
	sequence::gpu_viterbi(&log_trans, &log_emit, 2, 3, &delta, &backptr, &path_gpu).unwrap();

	let mut path = [0i32; 3];
	path_gpu.download_i32(&mut path).unwrap();

	println!("Viterbi best_path = {:?}", path);
	assert_eq!(path[0], 0, "Viterbi path[0]: expected 0, got {}", path[0]);
	assert_eq!(path[1], 1, "Viterbi path[1]: expected 1, got {}", path[1]);
	assert_eq!(path[2], 0, "Viterbi path[2]: expected 0, got {}", path[2]);
}

// ── cluster: union_find_cc ────────────────────────────────────────────────

#[test]
fn test_union_find_cc() {
	let edge_src_h: [i32; 3] = [0, 1, 3];
	let edge_dst_h: [i32; 3] = [1, 2, 4];

	let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
	let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();

	let labels_gpu = cluster::gpu_union_find_cc(&edge_src, &edge_dst, 5, 3).unwrap();

	let mut parent = [0i32; 5];
	labels_gpu.download_i32(&mut parent).unwrap();

	println!("UF parent = {:?}", parent);

	assert_eq!(
		parent[0], parent[1],
		"UF: nodes 0 and 1 should be in same component, got roots {} and {}",
		parent[0], parent[1]
	);
	assert_eq!(
		parent[1], parent[2],
		"UF: nodes 1 and 2 should be in same component, got roots {} and {}",
		parent[1], parent[2]
	);
	assert_eq!(
		parent[3], parent[4],
		"UF: nodes 3 and 4 should be in same component, got roots {} and {}",
		parent[3], parent[4]
	);
	assert_ne!(
		parent[0], parent[3],
		"UF: nodes 0 and 3 should be in different components, both got root {}",
		parent[0]
	);

	let mut roots: HashSet<i32> = HashSet::new();
	for &p in &parent {
		roots.insert(p);
	}
	assert_eq!(
		roots.len(),
		2,
		"UF: expected 2 components, got {} distinct roots: {:?}",
		roots.len(),
		roots
	);
}

// ── cluster: boruvka_mst ──────────────────────────────────────────────────

#[test]
fn test_boruvka_mst() {
	let edge_src_h: [i32; 4] = [0, 1, 2, 0];
	let edge_dst_h: [i32; 4] = [1, 2, 3, 3];
	let edge_w_h: [f64; 4] = [1.0, 5.0, 2.0, 10.0];

	let edge_src = GpuBuffer::upload_i32(&edge_src_h).unwrap();
	let edge_dst = GpuBuffer::upload_i32(&edge_dst_h).unwrap();
	let edge_w = {
		let __up = &edge_w_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let result = cluster::gpu_boruvka_mst(&edge_src, &edge_dst, &edge_w, 4, 4).unwrap();

	let mut in_mst = [0u8; 4];
	result.in_mst.download_u8(&mut in_mst).unwrap();

	println!("Boruvka in_mst = {:?}", in_mst);
	println!("Boruvka total_weight = {}", result.total_weight);

	assert!(
		result.total_weight.is_finite(),
		"NaN/Inf in Boruvka total_weight"
	);

	assert!(
		close(result.total_weight, 8.0),
		"FINDING: Boruvka total_weight: expected 8.0, got {}. Likely over-merging bug \
             (UF step uses all edges, not just MST edges)",
		result.total_weight
	);

	let mst_count = in_mst.iter().filter(|&&b| b != 0).count();
	assert_eq!(
		mst_count, 3,
		"FINDING: Boruvka MST edge count: expected 3, got {}. in_mst={:?}",
		mst_count, in_mst
	);
}

// ── cluster: core_distance ────────────────────────────────────────────────

#[test]
fn test_core_distance() {
	let points_h: [f64; 4] = [0.0, 1.0, 2.0, 10.0];

	let points = {
		let __up = &points_h;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let core_dist_gpu = GpuBuffer::alloc(4).unwrap();
	cluster::gpu_core_distance(&points, 4, 1, 2, &core_dist_gpu).unwrap();

	let mut cd = [0.0f64; 4];
	unsafe { core_dist_gpu.download_async(&mut cd, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	println!("core_distance = {:?}", cd);
	assert!(cd.iter().all(|v| v.is_finite()), "NaN/Inf in core_distance");

	let expected = [2.0, 1.0, 2.0, 9.0];
	for i in 0..4 {
		assert!(
			close(cd[i], expected[i]),
			"core_distance[{}]: expected {}, got {}",
			i,
			expected[i],
			cd[i]
		);
	}
}
