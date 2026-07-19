use crate::HipError;
use crate::kernels::{check_launch, ci, safe_i32};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

unsafe extern "C" {
	fn launch_bin_edges_uniform(
		x: *const c_void,
		edges: *mut c_void,
		rows: i32,
		cols: i32,
		n_bins: i32,
		stream: *mut c_void,
	);
	fn launch_bin_edges_quantile(
		x: *const c_void,
		edges: *mut c_void,
		tmp_col_buf: *mut c_void,
		rows: i32,
		cols: i32,
		n_bins: i32,
		stream: *mut c_void,
	);
	fn launch_quantize_features(
		x: *const c_void,
		edges: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		n_bins: i32,
		stream: *mut c_void,
	);
	fn launch_one_hot(
		labels: *const c_void,
		out: *mut c_void,
		n: i32,
		n_classes: i32,
		stream: *mut c_void,
	);
	fn count_distinct_workspace_bytes(x: *const c_void, n: i32, stream: *mut c_void) -> usize;
	fn launch_count_distinct(
		x: *const c_void,
		out: *mut c_void,
		unique_vals: *mut c_void,
		run_counts: *mut c_void,
		temp: *mut c_void,
		temp_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn run_length_workspace_bytes(x: *const c_void, n: i32, stream: *mut c_void) -> usize;
	fn launch_run_length(
		x: *const c_void,
		values_out: *mut c_void,
		counts_out: *mut c_void,
		n_runs_out: *mut c_void,
		temp: *mut c_void,
		temp_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
}

unsafe extern "C" {
	fn launch_pairwise_cosine(
		query: *const c_void,
		train: *const c_void,
		out: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		stream: *mut c_void,
	);
	fn launch_pairwise_l1(
		query: *const c_void,
		train: *const c_void,
		out: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		stream: *mut c_void,
	);
	fn launch_pairwise_hamming(
		query: *const c_void,
		train: *const c_void,
		out: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		stream: *mut c_void,
	);
}

#[inline]
pub fn gpu_bin_edges_uniform(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
	edges_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer refers to a live GpuBuffer allocation valid for this launch.
	unsafe {
		launch_bin_edges_uniform(
			x.ptr_raw().cast_const(),
			edges_out.ptr_raw(),
			ci(rows)?,
			ci(cols)?,
			ci(n_bins)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_bin_edges_quantile(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
	tmp_col: &GpuBuffer,
	edges_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer refers to a live GpuBuffer allocation valid for this launch.
	unsafe {
		launch_bin_edges_quantile(
			x.ptr_raw().cast_const(),
			edges_out.ptr_raw(),
			tmp_col.ptr_raw(),
			ci(rows)?,
			ci(cols)?,
			ci(n_bins)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_quantize_features(
	x: &GpuBuffer,
	edges: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer refers to a live GpuBuffer allocation valid for this launch.
	unsafe {
		launch_quantize_features(
			x.ptr_raw().cast_const(),
			edges.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(rows)?,
			ci(cols)?,
			ci(n_bins)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_one_hot(
	labels_i32: &GpuBuffer,
	n: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: labels_i32 and out are live GpuBuffers and the range-checked sizes bound the launcher's reads and writes.
	unsafe {
		launch_one_hot(
			labels_i32.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(n)?,
			ci(n_classes)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[must_use]
#[inline]
pub fn gpu_count_distinct_workspace_bytes(x: &GpuBuffer, n: usize) -> usize {
	// SAFETY: x is a live GpuBuffer and safe_i32 bounds n; the query only reads within it.
	unsafe {
		return count_distinct_workspace_bytes(
			x.ptr_raw().cast_const(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
}

#[inline]
pub fn gpu_count_distinct(
	x: &GpuBuffer,
	n: usize,
	unique_vals: &GpuBuffer,
	run_counts: &GpuBuffer,
	temp: &GpuBuffer,
	count_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer is a live GpuBuffer and the range-checked n bounds the launcher's accesses.
	unsafe {
		launch_count_distinct(
			x.ptr_raw().cast_const(),
			count_out.ptr_raw(),
			unique_vals.ptr_raw(),
			run_counts.ptr_raw(),
			temp.ptr_raw(),
			temp.len(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[must_use]
#[inline]
pub fn gpu_run_length_workspace_bytes(x: &GpuBuffer, n: usize) -> usize {
	// SAFETY: FFI workspace-size query over device buffer `x`, returning a usize byte count.
	unsafe {
		return run_length_workspace_bytes(
			x.ptr_raw().cast_const(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
}

#[inline]
pub fn gpu_run_length(
	x: &GpuBuffer,
	n: usize,
	temp: &GpuBuffer,
	values_out: &GpuBuffer,
	counts_out: &GpuBuffer,
	n_runs_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: all four buffers are valid device allocations sized for the run-length encode over `n` elements.
	unsafe {
		launch_run_length(
			x.ptr_raw().cast_const(),
			values_out.ptr_raw(),
			counts_out.ptr_raw(),
			n_runs_out.ptr_raw(),
			temp.ptr_raw(),
			temp.len(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_pairwise_cosine(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `query`, `train`, and `out` are valid device buffers sized for the pairwise-cosine kernel.
	unsafe {
		launch_pairwise_cosine(
			query.ptr_raw().cast_const(),
			train.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(nq)?,
			ci(nt)?,
			ci(dim)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_pairwise_l1(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `query`, `train`, and `out` are valid device buffers sized for the pairwise-L1 kernel.
	unsafe {
		launch_pairwise_l1(
			query.ptr_raw().cast_const(),
			train.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(nq)?,
			ci(nt)?,
			ci(dim)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_pairwise_hamming(
	query_u8: &GpuBuffer,
	train_u8: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI kernel launch; the buffer pointers stay valid for the call and the sizes are checked i32 casts.
	unsafe {
		launch_pairwise_hamming(
			query_u8.ptr_raw().cast_const(),
			train_u8.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(nq)?,
			ci(nt)?,
			ci(dim)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
