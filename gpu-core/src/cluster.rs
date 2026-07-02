use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::collections::HashSet;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_fixed_radius_count(
		points: *const c_void,
		count: *mut c_void,
		n: i32,
		dim: i32,
		eps: f64,
		stream: *mut c_void,
	);

	fn launch_fixed_radius_fill_csr(
		points: *const c_void,
		row_ptr: *const c_void,
		indices: *mut c_void,
		n: i32,
		dim: i32,
		eps: f64,
		stream: *mut c_void,
	);

	fn launch_uf_init(parent: *mut c_void, n_nodes: i32, stream: *mut c_void);

	fn launch_uf_hook(
		edge_src: *const c_void,
		edge_dst: *const c_void,
		parent: *mut c_void,
		changed: *mut c_void,
		n_edges: i32,
		stream: *mut c_void,
	);

	fn launch_uf_compress(parent: *mut c_void, n_nodes: i32, stream: *mut c_void);

	fn launch_boruvka_init(
		best_edge: *mut c_void,
		best_wkey: *mut c_void,
		n_nodes: i32,
		stream: *mut c_void,
	);

	fn launch_boruvka_min_w(
		edge_src: *const c_void,
		edge_dst: *const c_void,
		edge_w: *const c_void,
		parent: *const c_void,
		best_wkey: *mut c_void,
		n_edges: i32,
		stream: *mut c_void,
	);

	fn launch_boruvka_min_e(
		edge_src: *const c_void,
		edge_dst: *const c_void,
		edge_w: *const c_void,
		parent: *const c_void,
		best_wkey: *const c_void,
		best_edge: *mut c_void,
		n_edges: i32,
		stream: *mut c_void,
	);

	fn launch_boruvka_mark(
		best_edge: *const c_void,
		in_mst: *mut c_void,
		n_nodes: i32,
		n_edges: i32,
		stream: *mut c_void,
	);

	fn launch_core_distance(
		points: *const c_void,
		core_dist: *mut c_void,
		n: i32,
		dim: i32,
		min_pts: i32,
		stream: *mut c_void,
	);
}

// Compact CSR adjacency: `row_ptr` (i32[n+1]) gives each point's [start,end) span in
// `indices` (i32[nnz]), the flattened neighbor j-indices (self included). Memory is
// O(nnz), never O(n²) — a dense mask at n=100k would be 10 GB; this is the actual
// neighbor count.
pub struct NeighborCsr {
	pub row_ptr: GpuBuffer,
	pub indices: GpuBuffer,
	pub nnz: usize,
}

pub fn gpu_fixed_radius_neighbors(
	points: &GpuBuffer,
	n: usize,
	dim: usize,
	eps: f64,
) -> Result<NeighborCsr, HipError> {
	// Pass 1: per-point neighbor counts (one thread per point — no n×n buffer).
	let count = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	unsafe {
		launch_fixed_radius_count(
			points.ptr_raw() as *const c_void,
			count.ptr_raw(),
			n as i32,
			dim as i32,
			eps,
			std::ptr::null_mut(),
		);
	}
	check_launch();

	// Exclusive prefix sum of the length-n count vector → CSR row offsets. Done on
	// the host: it's O(n), not O(n²), so it costs nothing next to the kernels.
	let mut counts = vec![0i32; n];
	count.download_i32(&mut counts)?;
	let mut row_ptr = vec![0i32; n + 1];
	let mut acc: i64 = 0;
	for i in 0..n {
		row_ptr[i] = acc as i32;
		acc += counts[i] as i64;
	}
	row_ptr[n] = acc as i32;
	let nnz = acc as usize;
	let row_ptr_buf = GpuBuffer::upload_i32(&row_ptr)?;

	// Pass 2: fill the compact adjacency. alloc at least 1 so nnz==0 is valid.
	let indices = GpuBuffer::alloc_bytes(nnz.max(1) * std::mem::size_of::<i32>())?;
	unsafe {
		launch_fixed_radius_fill_csr(
			points.ptr_raw() as *const c_void,
			row_ptr_buf.ptr_raw() as *const c_void,
			indices.ptr_raw(),
			n as i32,
			dim as i32,
			eps,
			std::ptr::null_mut(),
		);
	}
	check_launch();

	Ok(NeighborCsr { row_ptr: row_ptr_buf, indices, nnz })
}

// Per-step union-find launchers; this caller owns the parent/changed buffers and
// drives the hook→compress convergence loop with the device→host readback.
pub fn gpu_union_find_cc(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
) -> Result<GpuBuffer, HipError> {
	let labels = GpuBuffer::alloc_bytes(n_nodes * std::mem::size_of::<i32>())?;
	let changed = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

	unsafe {
		launch_uf_init(labels.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
	}
	check_launch();

	let mut flag = [0i32; 1];
	loop {
		changed.memset_zero(std::mem::size_of::<i32>())?;
		unsafe {
			launch_uf_hook(
				edge_src.ptr_raw() as *const c_void,
				edge_dst.ptr_raw() as *const c_void,
				labels.ptr_raw(),
				changed.ptr_raw(),
				n_edges as i32,
				std::ptr::null_mut(),
			);
		}
		check_launch();
		unsafe {
			launch_uf_compress(labels.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
		}
		check_launch();
		changed.download_i32(&mut flag)?;
		if flag[0] == 0 {
			break;
		}
	}
	Ok(labels)
}

pub struct BoruvkaResult {
	pub in_mst: GpuBuffer,
	pub total_weight: f64,
}

// Per-step Borůvka launchers; this caller owns parent/best_edge/best_w/in_mst/changed,
// the round loop, the best_edge reset to -1, the selected-edge dedup, the inner
// union-find loop that merges components, and the host-side MST weight reduction.
pub fn gpu_boruvka_mst(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	edge_w: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
) -> Result<BoruvkaResult, HipError> {
	let in_mst = GpuBuffer::zeros_bytes(n_edges)?;
	let parent = GpuBuffer::alloc_bytes(n_nodes * std::mem::size_of::<i32>())?;
	let best_edge = GpuBuffer::alloc_bytes(n_nodes * std::mem::size_of::<i32>())?;
	let best_w = GpuBuffer::alloc(n_nodes)?;
	let changed = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

	// Host copies of the edge endpoints: needed to materialize the selected-edge
	// list that drives the inter-round union-find.
	let mut src_h = vec![0i32; n_edges];
	let mut dst_h = vec![0i32; n_edges];
	edge_src.download_i32(&mut src_h)?;
	edge_dst.download_i32(&mut dst_h)?;

	unsafe {
		launch_uf_init(parent.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
	}
	check_launch();

	let mut best_edge_h = vec![0i32; n_nodes];
	let mut flag = [0i32; 1];
	loop {
		// Per-round reset (best_edge = INT_MAX sentinel, best_wkey = u64 max),
		// then a two-pass race-free min: pass 1 atomicMin's the monotonic
		// weight encoding per component, pass 2 atomicMin's the edge index
		// among edges attaining that min (deterministic tie-break).
		unsafe {
			launch_boruvka_init(best_edge.ptr_raw(), best_w.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
		}
		check_launch();
		unsafe {
			launch_boruvka_min_w(
				edge_src.ptr_raw() as *const c_void,
				edge_dst.ptr_raw() as *const c_void,
				edge_w.ptr_raw() as *const c_void,
				parent.ptr_raw() as *const c_void,
				best_w.ptr_raw(),
				n_edges as i32,
				std::ptr::null_mut(),
			);
		}
		check_launch();
		unsafe {
			launch_boruvka_min_e(
				edge_src.ptr_raw() as *const c_void,
				edge_dst.ptr_raw() as *const c_void,
				edge_w.ptr_raw() as *const c_void,
				parent.ptr_raw() as *const c_void,
				best_w.ptr_raw() as *const c_void,
				best_edge.ptr_raw(),
				n_edges as i32,
				std::ptr::null_mut(),
			);
		}
		check_launch();
		unsafe {
			launch_boruvka_mark(
				best_edge.ptr_raw() as *const c_void,
				in_mst.ptr_raw(),
				n_nodes as i32,
				n_edges as i32,
				std::ptr::null_mut(),
			);
		}
		check_launch();

		best_edge.download_i32(&mut best_edge_h)?;

		// Dedup selected edges: a single cheapest edge can be picked by both of
		// its endpoint components.
		let mut seen: HashSet<i32> = HashSet::new();
		let mut sel_src: Vec<i32> = Vec::new();
		let mut sel_dst: Vec<i32> = Vec::new();
		for &e in &best_edge_h {
			if e >= 0 && (e as usize) < n_edges && seen.insert(e) {
				sel_src.push(src_h[e as usize]);
				sel_dst.push(dst_h[e as usize]);
			}
		}
		if sel_src.is_empty() {
			break; // no outgoing edges → all components final
		}

		let sel_src_buf = GpuBuffer::upload_i32(&sel_src)?;
		let sel_dst_buf = GpuBuffer::upload_i32(&sel_dst)?;
		let n_sel = sel_src.len();

		// Merge the selected components via union-find until it stabilizes.
		loop {
			changed.memset_zero(std::mem::size_of::<i32>())?;
			unsafe {
				launch_uf_hook(
					sel_src_buf.ptr_raw() as *const c_void,
					sel_dst_buf.ptr_raw() as *const c_void,
					parent.ptr_raw(),
					changed.ptr_raw(),
					n_sel as i32,
					std::ptr::null_mut(),
				);
			}
			check_launch();
			unsafe {
				launch_uf_compress(parent.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
			}
			check_launch();
			changed.download_i32(&mut flag)?;
			if flag[0] == 0 {
				break;
			}
		}
	}

	// Host-side MST total-weight reduction over the marked edges.
	let mut in_mst_h = vec![0u8; n_edges];
	in_mst.download_u8(&mut in_mst_h)?;
	let mut w_h = vec![0.0f64; n_edges];
	edge_w.download(&mut w_h)?;
	let total_weight: f64 = (0..n_edges)
		.filter(|&e| in_mst_h[e] != 0)
		.map(|e| w_h[e])
		.sum();

	Ok(BoruvkaResult {
		in_mst,
		total_weight,
	})
}

pub fn gpu_core_distance(
	points: &GpuBuffer,
	n: usize,
	dim: usize,
	min_pts: usize,
) -> Result<GpuBuffer, HipError> {
	let core_dist = GpuBuffer::alloc(n)?;
	unsafe {
		launch_core_distance(
			points.ptr_raw() as *const c_void,
			core_dist.ptr_raw(),
			n as i32,
			dim as i32,
			min_pts as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(core_dist)
}
