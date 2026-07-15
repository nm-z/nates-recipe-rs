use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::mem;
use core::ptr;

unsafe extern "C" {
	fn launch_csr_spmv(
		values: *const c_void,
		col_idx: *const c_void,
		row_ptr: *const c_void,
		x: *const c_void,
		y: *mut c_void,
		n_rows: i32,
		stream: *mut c_void,
	);
	fn launch_csr_spmm(
		values: *const c_void,
		col_idx: *const c_void,
		row_ptr: *const c_void,
		b: *const c_void,
		c: *mut c_void,
		n_rows: i32,
		feat: i32,
		stream: *mut c_void,
	);
	fn launch_neighbor_aggregate(
		features: *const c_void,
		edge_src: *const c_void,
		edge_dst: *const c_void,
		agg: *mut c_void,
		deg: *const c_void,
		n_nodes: i32,
		feat: i32,
		n_edges: i32,
		mean_flag: i32,
		stream: *mut c_void,
	);
	fn launch_degree(
		edge_dst: *const c_void,
		deg: *mut c_void,
		n_edges: i32,
		stream: *mut c_void,
	);
	fn launch_gcn_norm(
		features: *mut c_void,
		deg: *const c_void,
		n_nodes: i32,
		feat: i32,
		stream: *mut c_void,
	);
}

/// # Errors
/// Returns [`HipError`] if `n_rows` overflows `i32`.
#[inline]
pub fn gpu_csr_spmv(
	values: &GpuBuffer,
	col_idx: &GpuBuffer,
	row_ptr: &GpuBuffer,
	x: &GpuBuffer,
	n_rows: usize,
	y_out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows = ci(n_rows)?;
	// SAFETY: buffer pointers are live device allocations and `rows` is range-checked.
	unsafe {
		launch_csr_spmv(
			values.ptr_raw().cast_const(),
			col_idx.ptr_raw().cast_const(),
			row_ptr.ptr_raw().cast_const(),
			x.ptr_raw().cast_const(),
			y_out.ptr_raw(),
			rows,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_rows` or `feat` overflows `i32`.
#[inline]
pub fn gpu_csr_spmm(
	values: &GpuBuffer,
	col_idx: &GpuBuffer,
	row_ptr: &GpuBuffer,
	dense_b: &GpuBuffer,
	n_rows: usize,
	feat: usize,
	c_out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows = ci(n_rows)?;
	let cols = ci(feat)?;
	// SAFETY: buffer pointers are live device allocations and the sizes are range-checked.
	unsafe {
		launch_csr_spmm(
			values.ptr_raw().cast_const(),
			col_idx.ptr_raw().cast_const(),
			row_ptr.ptr_raw().cast_const(),
			dense_b.ptr_raw().cast_const(),
			c_out.ptr_raw(),
			rows,
			cols,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a device memset fails or a size overflows `i32`.
#[inline]
pub fn gpu_neighbor_aggregate(
	features: &GpuBuffer,
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	deg_ws: &GpuBuffer,
	n_nodes: usize,
	feat: usize,
	n_edges: usize,
	mean: usize,
	agg_out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::kernels::{check_launch, ci};
	agg_out.memset_zero(n_nodes * feat * mem::size_of::<f64>())?;
	deg_ws.memset_zero(n_nodes * mem::size_of::<f64>())?;
	if mean != 0 {
		// SAFETY: edge_dst and deg_ws are live device buffers for the launch duration.
		unsafe {
			launch_degree(
				edge_dst.ptr_raw().cast_const(),
				deg_ws.ptr_raw(),
				ci(n_edges)?,
				ptr::null_mut(),
			);
		}
		check_launch();
	}

	// SAFETY: all argument buffers are live device allocations for the launch duration.
	unsafe {
		launch_neighbor_aggregate(
			features.ptr_raw().cast_const(),
			edge_src.ptr_raw().cast_const(),
			edge_dst.ptr_raw().cast_const(),
			agg_out.ptr_raw(),
			deg_ws.ptr_raw().cast_const(),
			ci(n_nodes)?,
			ci(feat)?,
			ci(n_edges)?,
			ci(mean)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] on a device failure or a size that overflows `i32`.
#[inline]
pub fn gpu_degree(
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
	deg_out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::kernels::ci;
	deg_out.memset_zero(n_nodes * mem::size_of::<f64>())?;
	// SAFETY: edge_dst is a live device buffer for the launch duration.
	unsafe {
		launch_degree(
			edge_dst.ptr_raw().cast_const(),
			deg_out.ptr_raw(),
			ci(n_edges)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when a size overflows `i32` or the kernel launch reports failure.
#[inline]
pub fn gpu_gcn_norm(
	deg: &GpuBuffer,
	n_nodes: usize,
	feat: usize,
	features: &GpuBuffer,
) -> Result<(), HipError> {
	let nodes = ci(n_nodes)?;
	let cols = ci(feat)?;
	// SAFETY: device pointers are live GpuBuffer allocations that outlive the launch.
	unsafe {
		launch_gcn_norm(
			features.ptr_raw(),
			deg.ptr_raw().cast_const(),
			nodes,
			cols,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
