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
// Returns GpuBuffer of shape [cols * (n_bins + 1)] f64 (edges per column, equal-width).
pub fn gpu_bin_edges_uniform(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(cols * (n_bins + 1))?;
	unsafe {
		launch_bin_edges_uniform(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			rows as i32,
			cols as i32,
			n_bins as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

// gpu_bin_edges_quantile
// Returns GpuBuffer of shape [cols * (n_bins + 1)] f64 (edges per column, equal-frequency).
// Precondition: rows <= 1024 (bitonic sort shared-memory limit).
pub fn gpu_bin_edges_quantile(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
) -> Result<GpuBuffer, HipError> {
	let edges = GpuBuffer::alloc(cols * (n_bins + 1))?;
	let tmp_col = GpuBuffer::alloc(cols * rows)?;
	unsafe {
		launch_bin_edges_quantile(
			x.ptr_raw() as *const c_void,
			edges.ptr_raw(),
			tmp_col.ptr_raw(),
			rows as i32,
			cols as i32,
			n_bins as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(edges)
}

// gpu_quantize_features
// Returns GpuBuffer (byte-sized) of shape [rows * cols] u8.
// Each element is the bin index [0, n_bins-1] for the corresponding x value.
// edges must be the output of gpu_bin_edges_uniform or gpu_bin_edges_quantile
// with matching (cols, n_bins).
pub fn gpu_quantize_features(
	x: &GpuBuffer,
	edges: &GpuBuffer,
	rows: usize,
	cols: usize,
	n_bins: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc_bytes(rows * cols)?;
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
	Ok(out)
}

// gpu_one_hot
// labels_i32: GpuBuffer of i32[n].
// Returns GpuBuffer of f64[n * n_classes].
pub fn gpu_one_hot(
	labels_i32: &GpuBuffer,
	n: usize,
	n_classes: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n * n_classes)?;
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
	Ok(out)
}

// gpu_count_distinct
// x must be a sorted i32 GpuBuffer of length n.
// Returns the number of distinct values as usize.
pub fn gpu_count_distinct(x: &GpuBuffer, n: usize) -> Result<usize, HipError> {
	let out = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	let unique_vals = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	let run_counts = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	let temp_bytes = unsafe {
		count_distinct_workspace_bytes(
			x.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		)
	};
	let temp = GpuBuffer::alloc_bytes(temp_bytes)?;
	unsafe {
		launch_count_distinct(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			unique_vals.ptr_raw(),
			run_counts.ptr_raw(),
			temp.ptr_raw(),
			temp_bytes,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut v = [0i32; 1];
	out.download_i32(&mut v)?;
	Ok(v[0] as usize)
}

// gpu_run_length
// Precondition: x must be a sorted i32 GpuBuffer of length n.
// Returns (values GpuBuffer[i32], counts GpuBuffer[i32], n_runs usize).
// values and counts are valid for indices 0..n_runs.
pub fn gpu_run_length(x: &GpuBuffer, n: usize) -> Result<(GpuBuffer, GpuBuffer, usize), HipError> {
	let values = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	let counts = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	let n_runs_buf = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	let temp_bytes = unsafe {
		run_length_workspace_bytes(x.ptr_raw() as *const c_void, n as i32, std::ptr::null_mut())
	};
	let temp = GpuBuffer::alloc_bytes(temp_bytes)?;
	unsafe {
		launch_run_length(
			x.ptr_raw() as *const c_void,
			values.ptr_raw(),
			counts.ptr_raw(),
			n_runs_buf.ptr_raw(),
			temp.ptr_raw(),
			temp_bytes,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut nr = [0i32; 1];
	n_runs_buf.download_i32(&mut nr)?;
	Ok((values, counts, nr[0] as usize))
}

// gpu_pairwise_cosine
// query: GpuBuffer f64[nq * dim], train: GpuBuffer f64[nt * dim].
// Returns GpuBuffer f64[nq * nt] of cosine similarities.
pub fn gpu_pairwise_cosine(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(nq * nt)?;
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
	Ok(out)
}

// gpu_pairwise_l1
// query: GpuBuffer f64[nq * dim], train: GpuBuffer f64[nt * dim].
// Returns GpuBuffer f64[nq * nt] of L1 distances.
pub fn gpu_pairwise_l1(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(nq * nt)?;
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
	Ok(out)
}

// gpu_pairwise_hamming
// query_u8: GpuBuffer u8[nq * dim], train_u8: GpuBuffer u8[nt * dim].
// Returns GpuBuffer f64[nq * nt] of normalized Hamming distances (mismatches / dim).
pub fn gpu_pairwise_hamming(
	query_u8: &GpuBuffer,
	train_u8: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(nq * nt)?;
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
	Ok(out)
}
