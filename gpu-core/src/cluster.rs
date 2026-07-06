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
		eps: *const c_void,
		stream: *mut c_void,
	);

	fn launch_exclusive_scan_i32(
		count_in: *const c_void,
		row_ptr_out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);

	fn launch_fixed_radius_fill_csr(
		points: *const c_void,
		row_ptr: *const c_void,
		indices: *mut c_void,
		n: i32,
		dim: i32,
		eps: *const c_void,
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

	fn launch_masked_weight_sum(
		edge_w: *const c_void,
		in_mst: *const c_void,
		total: *mut c_void,
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

// ── Fixed-radius neighbor CSR: count → scan → fill schedulable ops ───────────

// Pass 1: per-point neighbor counts (one thread per point — no n×n buffer).
pub fn fixed_radius_count(
	points: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	dim: usize,
	count_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_fixed_radius_count(
			points.ptr_raw() as *const c_void,
			count_out.ptr_raw(),
			n as i32,
			dim as i32,
			eps.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// Exclusive prefix sum of the length-n count vector → CSR row offsets. Writes
// row_ptr_out[0..n] and row_ptr_out[n] = nnz.
pub fn exclusive_scan_i32(
	count_in: &GpuBuffer,
	n: usize,
	row_ptr_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_exclusive_scan_i32(
			count_in.ptr_raw() as *const c_void,
			row_ptr_out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// Pass 2: fill the compact adjacency (one thread per point owns its row).
pub fn fixed_radius_fill_csr(
	points: &GpuBuffer,
	row_ptr: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	dim: usize,
	indices_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_fixed_radius_fill_csr(
			points.ptr_raw() as *const c_void,
			row_ptr.ptr_raw() as *const c_void,
			indices_out.ptr_raw(),
			n as i32,
			dim as i32,
			eps.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// content_sized CSR (nnz unknown at plan time): composes count → scan → row_ptr[n]
// readback to size indices → fill; owns the eps upload and the nnz download.
// not-an-op: driver — count/scan/read/fill composite with a mid-flow nnz readback
pub fn gpu_fixed_radius_neighbors(
	points: &GpuBuffer,
	n: usize,
	dim: usize,
	eps: f64,
) -> Result<NeighborCsr, HipError> {
	let eps_buf = GpuBuffer::upload(&[eps])?;
	let count = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	fixed_radius_count(points, &eps_buf, n, dim, &count)?;

	let row_ptr_buf = GpuBuffer::alloc_bytes((n + 1) * std::mem::size_of::<i32>())?;
	exclusive_scan_i32(&count, n, &row_ptr_buf)?;

	// nnz = row_ptr[n], read back as a plan-time size query to size `indices`.
	let mut row_ptr_h = vec![0i32; n + 1];
	row_ptr_buf.download_i32(&mut row_ptr_h)?;
	let nnz = row_ptr_h[n] as usize;

	let indices = GpuBuffer::alloc_bytes(nnz.max(1) * std::mem::size_of::<i32>())?;
	fixed_radius_fill_csr(points, &row_ptr_buf, &eps_buf, n, dim, &indices)?;

	Ok(NeighborCsr { row_ptr: row_ptr_buf, indices, nnz })
}

// ── Union-Find connected components: per-step ops + convergence driver ───────

pub fn uf_init(n_nodes: usize, parent_out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_uf_init(parent_out.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn uf_hook(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	n_edges: usize,
	parent_out: &GpuBuffer,
	changed_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_uf_hook(
			edge_src.ptr_raw() as *const c_void,
			edge_dst.ptr_raw() as *const c_void,
			parent_out.ptr_raw(),
			changed_out.ptr_raw(),
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn uf_compress(n_nodes: usize, parent_out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_uf_compress(parent_out.ptr_raw(), n_nodes as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

// not-an-op: driver — hook→compress convergence loop branching on the `changed` readback
pub fn gpu_union_find_cc(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
) -> Result<GpuBuffer, HipError> {
	let labels = GpuBuffer::alloc_bytes(n_nodes * std::mem::size_of::<i32>())?;
	let changed = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

	uf_init(n_nodes, &labels)?;

	let mut flag = [0i32; 1];
	loop {
		changed.memset_zero(std::mem::size_of::<i32>())?;
		uf_hook(edge_src, edge_dst, n_edges, &labels, &changed)?;
		uf_compress(n_nodes, &labels)?;
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

// ── Borůvka MST: per-step ops + round/dedup/inner-UF driver ──────────────────

pub fn boruvka_init(
	n_nodes: usize,
	best_edge_out: &GpuBuffer,
	best_wkey_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_boruvka_init(
			best_edge_out.ptr_raw(),
			best_wkey_out.ptr_raw(),
			n_nodes as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn boruvka_min_w(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	edge_w: &GpuBuffer,
	parent: &GpuBuffer,
	n_edges: usize,
	best_wkey_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_boruvka_min_w(
			edge_src.ptr_raw() as *const c_void,
			edge_dst.ptr_raw() as *const c_void,
			edge_w.ptr_raw() as *const c_void,
			parent.ptr_raw() as *const c_void,
			best_wkey_out.ptr_raw(),
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn boruvka_min_e(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	edge_w: &GpuBuffer,
	parent: &GpuBuffer,
	best_wkey: &GpuBuffer,
	n_edges: usize,
	best_edge_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_boruvka_min_e(
			edge_src.ptr_raw() as *const c_void,
			edge_dst.ptr_raw() as *const c_void,
			edge_w.ptr_raw() as *const c_void,
			parent.ptr_raw() as *const c_void,
			best_wkey.ptr_raw() as *const c_void,
			best_edge_out.ptr_raw(),
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn boruvka_mark(
	best_edge: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
	in_mst_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_boruvka_mark(
			best_edge.ptr_raw() as *const c_void,
			in_mst_out.ptr_raw(),
			n_nodes as i32,
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// Device MST total-weight reduction over marked edges → 1-elem total_out.
pub fn masked_weight_sum(
	edge_w: &GpuBuffer,
	in_mst: &GpuBuffer,
	n_edges: usize,
	total_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_masked_weight_sum(
			edge_w.ptr_raw() as *const c_void,
			in_mst.ptr_raw() as *const c_void,
			total_out.ptr_raw(),
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// round loop with host edge-endpoint download, HashSet dedup of selected edges,
// selected-list upload, inner union-find convergence, and final weight download.
// not-an-op: driver — Borůvka round/dedup/inner-UF orchestration over the step ops
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

	uf_init(n_nodes, &parent)?;

	let mut best_edge_h = vec![0i32; n_nodes];
	let mut flag = [0i32; 1];
	loop {
		// Per-round reset then a two-pass race-free min: pass 1 atomicMin's the
		// monotonic weight encoding per component, pass 2 atomicMin's the edge
		// index among edges attaining that min (deterministic tie-break).
		boruvka_init(n_nodes, &best_edge, &best_w)?;
		boruvka_min_w(edge_src, edge_dst, edge_w, &parent, n_edges, &best_w)?;
		boruvka_min_e(edge_src, edge_dst, edge_w, &parent, &best_w, n_edges, &best_edge)?;
		boruvka_mark(&best_edge, n_nodes, n_edges, &in_mst)?;

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
			uf_hook(&sel_src_buf, &sel_dst_buf, n_sel, &parent, &changed)?;
			uf_compress(n_nodes, &parent)?;
			changed.download_i32(&mut flag)?;
			if flag[0] == 0 {
				break;
			}
		}
	}

	// Device MST total-weight reduction over the marked edges; driver reads it back.
	let total_buf = GpuBuffer::alloc(1)?;
	masked_weight_sum(edge_w, &in_mst, n_edges, &total_buf)?;
	let mut tw = [0.0f64; 1];
	total_buf.download(&mut tw)?;

	Ok(BoruvkaResult { in_mst, total_weight: tw[0] })
}

pub fn gpu_core_distance(
	points: &GpuBuffer,
	n: usize,
	dim: usize,
	min_pts: usize,
	core_dist_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_core_distance(
			points.ptr_raw() as *const c_void,
			core_dist_out.ptr_raw(),
			n as i32,
			dim as i32,
			min_pts as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
