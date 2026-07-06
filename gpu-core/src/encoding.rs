use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// ── FFI: encode.hip ──────────────────────────────────────────────────────────
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

// ── FFI: metrics.hip ─────────────────────────────────────────────────────────
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

// ── Public API ────────────────────────────────────────────────────────────────

// gpu_bin_edges_uniform
// edges_out: caller-provided f64[cols * (n_bins + 1)] (edges per column, equal-width).
pub fn gpu_bin_edges_uniform(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
	edges_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bin_edges_uniform(
			x.ptr_raw() as *const c_void,
			edges_out.ptr_raw(),
			rows as i32,
			cols as i32,
			n_bins as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_bin_edges_quantile
// edges_out: caller-provided f64[cols * (n_bins + 1)] (edges per column, equal-frequency).
// tmp_col: caller-provided f64[cols * rows] workspace.
// Precondition: rows <= 1024 (bitonic sort shared-memory limit).
pub fn gpu_bin_edges_quantile(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
	tmp_col: &GpuBuffer,
	edges_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bin_edges_quantile(
			x.ptr_raw() as *const c_void,
			edges_out.ptr_raw(),
			tmp_col.ptr_raw(),
			rows as i32,
			cols as i32,
			n_bins as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_quantize_features
// out: caller-provided byte-sized u8[rows * cols].
// Each element is the bin index [0, n_bins-1] for the corresponding x value.
// edges must be the output of gpu_bin_edges_uniform or gpu_bin_edges_quantile
// with matching (cols, n_bins).
pub fn gpu_quantize_features(
	x: &GpuBuffer,
	edges: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_quantize_features(
			x.ptr_raw() as *const c_void,
			edges.ptr_raw() as *const c_void,
			out.ptr_raw(),
			rows as i32,
			cols as i32,
			n_bins as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_one_hot
// labels_i32: GpuBuffer of i32[n]. out: caller-provided f64[n * n_classes].
pub fn gpu_one_hot(
	labels_i32: &GpuBuffer,
	n: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_one_hot(
			labels_i32.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_count_distinct_workspace_bytes
// Plan-time size query for the `temp` workspace of gpu_count_distinct.
// not-an-op: plan-time helper — rocPRIM temp-size query needs the real device ptr
pub fn gpu_count_distinct_workspace_bytes(x: &GpuBuffer, n: usize) -> usize {
	unsafe { count_distinct_workspace_bytes(x.ptr_raw() as *const c_void, n as i32, std::ptr::null_mut()) }
}

// gpu_count_distinct
// x must be a sorted i32 GpuBuffer of length n.
// count_out: caller-provided 1-elem i32 buffer receiving the distinct-value count.
// unique_vals/run_counts: caller-provided n*i32 scratch.
// temp: caller-provided workspace sized via gpu_count_distinct_workspace_bytes.
pub fn gpu_count_distinct(
	x: &GpuBuffer,
	n: usize,
	unique_vals: &GpuBuffer,
	run_counts: &GpuBuffer,
	temp: &GpuBuffer,
	count_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_count_distinct(
			x.ptr_raw() as *const c_void,
			count_out.ptr_raw(),
			unique_vals.ptr_raw(),
			run_counts.ptr_raw(),
			temp.ptr_raw(),
			temp.len(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_run_length_workspace_bytes
// Plan-time size query for the `temp` workspace of gpu_run_length.
// not-an-op: plan-time helper — rocPRIM temp-size query needs the real device ptr
pub fn gpu_run_length_workspace_bytes(x: &GpuBuffer, n: usize) -> usize {
	unsafe { run_length_workspace_bytes(x.ptr_raw() as *const c_void, n as i32, std::ptr::null_mut()) }
}

// gpu_run_length
// Precondition: x must be a sorted i32 GpuBuffer of length n.
// values_out/counts_out: caller-provided max-sized n*i32 buffers, valid for 0..n_runs.
// n_runs_out: caller-provided 1-elem i32 buffer receiving the run count.
// temp: caller-provided workspace sized via gpu_run_length_workspace_bytes.
pub fn gpu_run_length(
	x: &GpuBuffer,
	n: usize,
	temp: &GpuBuffer,
	values_out: &GpuBuffer,
	counts_out: &GpuBuffer,
	n_runs_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_run_length(
			x.ptr_raw() as *const c_void,
			values_out.ptr_raw(),
			counts_out.ptr_raw(),
			n_runs_out.ptr_raw(),
			temp.ptr_raw(),
			temp.len(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_pairwise_cosine
// query: GpuBuffer f64[nq * dim], train: GpuBuffer f64[nt * dim].
// out: caller-provided f64[nq * nt] of cosine similarities.
pub fn gpu_pairwise_cosine(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_pairwise_cosine(
			query.ptr_raw() as *const c_void,
			train.ptr_raw() as *const c_void,
			out.ptr_raw(),
			nq as i32,
			nt as i32,
			dim as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_pairwise_l1
// query: GpuBuffer f64[nq * dim], train: GpuBuffer f64[nt * dim].
// out: caller-provided f64[nq * nt] of L1 distances.
pub fn gpu_pairwise_l1(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_pairwise_l1(
			query.ptr_raw() as *const c_void,
			train.ptr_raw() as *const c_void,
			out.ptr_raw(),
			nq as i32,
			nt as i32,
			dim as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// gpu_pairwise_hamming
// query_u8: GpuBuffer u8[nq * dim], train_u8: GpuBuffer u8[nt * dim].
// out: caller-provided f64[nq * nt] of normalized Hamming distances (mismatches / dim).
pub fn gpu_pairwise_hamming(
	query_u8: &GpuBuffer,
	train_u8: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_pairwise_hamming(
			query_u8.ptr_raw() as *const c_void,
			train_u8.ptr_raw() as *const c_void,
			out.ptr_raw(),
			nq as i32,
			nt as i32,
			dim as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
