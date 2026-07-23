use crate::HipError;
use crate::hip::device_synchronize;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::cmp::Ordering;
use core::ffi::c_void;
use core::mem;
use core::ptr;
use std::collections::HashSet;

unsafe extern "C" {
	fn launch_fixed_radius_count(
		points: *const c_void,
		count: *mut c_void,
		n: i32,
		dim: i32,
		eps: *const c_void,
		stream: *mut c_void,
	);

	fn launch_exclusive_scan_i32(count_in: *const c_void, row_ptr_out: *mut c_void, n: i32, stream: *mut c_void);

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

	fn launch_boruvka_init(best_edge: *mut c_void, best_wkey: *mut c_void, n_nodes: i32, stream: *mut c_void);

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

pub struct NeighborCsr {
	pub row_ptr: GpuBuffer,
	pub indices: GpuBuffer,
	pub nnz: usize,
}

#[inline]
pub fn fixed_radius_count(
	points: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	dim: usize,
	count_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointers come from live GpuBuffers valid for this launch; n/dim are range-checked.
	unsafe {
		launch_fixed_radius_count(
			points.ptr_raw().cast_const(),
			count_out.ptr_raw(),
			ci(n)?,
			ci(dim)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn exclusive_scan_i32(count_in: &GpuBuffer, n: usize, row_ptr_out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: pointers come from live GpuBuffers valid for this launch; n is range-checked.
	unsafe {
		launch_exclusive_scan_i32(
			count_in.ptr_raw().cast_const(),
			row_ptr_out.ptr_raw(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn fixed_radius_fill_csr(
	points: &GpuBuffer,
	row_ptr: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	dim: usize,
	indices_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointers come from live GpuBuffers valid for this launch; n/dim are range-checked.
	unsafe {
		launch_fixed_radius_fill_csr(
			points.ptr_raw().cast_const(),
			row_ptr.ptr_raw().cast_const(),
			indices_out.ptr_raw(),
			ci(n)?,
			ci(dim)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_fixed_radius_neighbors(points: &GpuBuffer, n: usize, dim: usize, eps: f64) -> Result<NeighborCsr, HipError> {
	let eps_buf = GpuBuffer::alloc(1)?;
	eps_buf.load(&[eps])?;
	let count = GpuBuffer::alloc_bytes(n * mem::size_of::<i32>())?;
	fixed_radius_count(points, &eps_buf, n, dim, &count)?;

	let row_ptr_buf = GpuBuffer::alloc_bytes((n + 1) * mem::size_of::<i32>())?;
	exclusive_scan_i32(&count, n, &row_ptr_buf)?;

	let mut row_ptr_h = vec![0i32; n + 1];
	row_ptr_buf.download_i32(&mut row_ptr_h)?;
	let nnz = usize::try_from(row_ptr_h[n]).map_err(|_err| return HipError(1))?;

	let indices = GpuBuffer::alloc_bytes(nnz.max(1) * mem::size_of::<i32>())?;
	fixed_radius_fill_csr(points, &row_ptr_buf, &eps_buf, n, dim, &indices)?;

	return Ok(NeighborCsr {
		row_ptr: row_ptr_buf,
		indices,
		nnz,
	});
}

#[inline]
pub fn uf_init(n_nodes: usize, parent_out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: parent_out is a live GpuBuffer and the node count is a checked i32.
	unsafe {
		launch_uf_init(parent_out.ptr_raw(), ci(n_nodes)?, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn uf_hook(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	n_edges: usize,
	parent_out: &GpuBuffer,
	changed_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffer pointers are live and n_edges is a checked i32.
	unsafe {
		launch_uf_hook(
			edge_src.ptr_raw().cast_const(),
			edge_dst.ptr_raw().cast_const(),
			parent_out.ptr_raw(),
			changed_out.ptr_raw(),
			ci(n_edges)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn uf_compress(n_nodes: usize, parent_out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: parent_out is a live GpuBuffer and the node count is a checked i32.
	unsafe {
		launch_uf_compress(parent_out.ptr_raw(), ci(n_nodes)?, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_union_find_cc(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
) -> Result<GpuBuffer, HipError> {
	let labels = GpuBuffer::alloc_bytes(n_nodes * mem::size_of::<i32>())?;
	let changed = GpuBuffer::alloc_bytes(mem::size_of::<i32>())?;

	uf_init(n_nodes, &labels)?;

	let mut flag = [0i32; 1];
	loop {
		changed.memset_zero(mem::size_of::<i32>())?;
		uf_hook(edge_src, edge_dst, n_edges, &labels, &changed)?;
		uf_compress(n_nodes, &labels)?;
		changed.download_i32(&mut flag)?;
		match flag[0].cmp(&0i32) {
			Ordering::Equal => break,
			Ordering::Less | Ordering::Greater => {}
		}
	}
	return Ok(labels);
}

pub struct BoruvkaResult {
	pub in_mst: GpuBuffer,
	pub total_weight: f64,
}

#[inline]
pub fn boruvka_init(n_nodes: usize, best_edge_out: &GpuBuffer, best_wkey_out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: the buffers outlive the call and n_nodes is range-checked into i32.
	unsafe {
		launch_boruvka_init(
			best_edge_out.ptr_raw(),
			best_wkey_out.ptr_raw(),
			ci(n_nodes)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn boruvka_min_w(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	edge_w: &GpuBuffer,
	parent: &GpuBuffer,
	n_edges: usize,
	best_wkey_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the buffers outlive the call and n_edges is range-checked into i32.
	unsafe {
		launch_boruvka_min_w(
			edge_src.ptr_raw().cast_const(),
			edge_dst.ptr_raw().cast_const(),
			edge_w.ptr_raw().cast_const(),
			parent.ptr_raw().cast_const(),
			best_wkey_out.ptr_raw(),
			ci(n_edges)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn boruvka_min_e(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	edge_w: &GpuBuffer,
	parent: &GpuBuffer,
	best_wkey: &GpuBuffer,
	n_edges: usize,
	best_edge_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the buffers outlive the call and n_edges is range-checked into i32.
	unsafe {
		launch_boruvka_min_e(
			edge_src.ptr_raw().cast_const(),
			edge_dst.ptr_raw().cast_const(),
			edge_w.ptr_raw().cast_const(),
			parent.ptr_raw().cast_const(),
			best_wkey.ptr_raw().cast_const(),
			best_edge_out.ptr_raw(),
			ci(n_edges)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn boruvka_mark(
	best_edge: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
	in_mst_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: raw device pointers come from live GpuBuffers valid for this launch.
	unsafe {
		launch_boruvka_mark(
			best_edge.ptr_raw().cast_const(),
			in_mst_out.ptr_raw(),
			ci(n_nodes)?,
			ci(n_edges)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn masked_weight_sum(
	edge_w: &GpuBuffer,
	in_mst: &GpuBuffer,
	n_edges: usize,
	total_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: raw device pointers come from live GpuBuffers valid for this launch.
	unsafe {
		launch_masked_weight_sum(
			edge_w.ptr_raw().cast_const(),
			in_mst.ptr_raw().cast_const(),
			total_out.ptr_raw(),
			ci(n_edges)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_boruvka_mst(
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	edge_w: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
) -> Result<BoruvkaResult, HipError> {
	let in_mst = GpuBuffer::alloc_bytes(n_edges)?;
	in_mst.memset_zero(n_edges)?;
	let parent = GpuBuffer::alloc_bytes(n_nodes * mem::size_of::<i32>())?;
	let best_edge = GpuBuffer::alloc_bytes(n_nodes * mem::size_of::<i32>())?;
	let best_w = GpuBuffer::alloc(n_nodes)?;
	let changed = GpuBuffer::alloc_bytes(mem::size_of::<i32>())?;

	let mut src_h = vec![0i32; n_edges];
	let mut dst_h = vec![0i32; n_edges];
	edge_src.download_i32(&mut src_h)?;
	edge_dst.download_i32(&mut dst_h)?;

	uf_init(n_nodes, &parent)?;

	let mut best_edge_h = vec![0i32; n_nodes];
	let mut flag = [0i32; 1];
	loop {
		boruvka_init(n_nodes, &best_edge, &best_w)?;
		boruvka_min_w(edge_src, edge_dst, edge_w, &parent, n_edges, &best_w)?;
		boruvka_min_e(
			edge_src, edge_dst, edge_w, &parent, &best_w, n_edges, &best_edge,
		)?;
		boruvka_mark(&best_edge, n_nodes, n_edges, &in_mst)?;

		best_edge.download_i32(&mut best_edge_h)?;

		let mut seen: HashSet<i32> = HashSet::new();
		let mut sel_src: Vec<i32> = Vec::new();
		let mut sel_dst: Vec<i32> = Vec::new();
		for &e in &best_edge_h {
			if let Some(i) = usize::try_from(e)
				.ok()
				.filter(|&idx| return idx < n_edges)
				.filter(|_idx| return seen.insert(e))
			{
				sel_src.push(src_h[i]);
				sel_dst.push(dst_h[i]);
			}
		}
		match sel_src.len().cmp(&0) {
			Ordering::Equal | Ordering::Less => break,
			Ordering::Greater => {
				let sel_src_buf = GpuBuffer::upload_i32(&sel_src)?;
				let sel_dst_buf = GpuBuffer::upload_i32(&sel_dst)?;
				let n_sel = sel_src.len();

				loop {
					changed.memset_zero(mem::size_of::<i32>())?;
					uf_hook(&sel_src_buf, &sel_dst_buf, n_sel, &parent, &changed)?;
					uf_compress(n_nodes, &parent)?;
					changed.download_i32(&mut flag)?;
					match flag[0].cmp(&0i32) {
						Ordering::Equal => break,
						Ordering::Less | Ordering::Greater => {}
					}
				}
			}
		}
	}

	let total_buf = GpuBuffer::alloc(1)?;
	masked_weight_sum(edge_w, &in_mst, n_edges, &total_buf)?;
	let mut tw = [0.0f64; 1];
	// SAFETY: tw is a live host buffer sized for the async download from total_buf.
	unsafe { total_buf.download_async(&mut tw, ptr::null_mut()) }?;
	device_synchronize()?;

	return Ok(BoruvkaResult {
		in_mst,
		total_weight: tw[0],
	});
}

#[inline]
pub fn gpu_core_distance(
	points: &GpuBuffer,
	n: usize,
	dim: usize,
	min_pts: usize,
	core_dist_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: raw device pointers come from live GpuBuffers valid for this launch.
	unsafe {
		launch_core_distance(
			points.ptr_raw().cast_const(),
			core_dist_out.ptr_raw(),
			ci(n)?,
			ci(dim)?,
			ci(min_pts)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
