use crate::hip::HipError;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

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

// CSR sparse matrix-vector product: y = A * x, A stored in CSR format.
// values/col_idx have nnz elements; row_ptr has n_rows+1 elements (i32).
// x has n_cols elements (f64); y_out is n_rows (f64).
pub fn gpu_csr_spmv(
	values: &GpuBuffer,
	col_idx: &GpuBuffer,
	row_ptr: &GpuBuffer,
	x: &GpuBuffer,
	n_rows: usize,
	y_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_csr_spmv(
			values.ptr_raw() as *const c_void,
			col_idx.ptr_raw() as *const c_void,
			row_ptr.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			y_out.ptr_raw(),
			n_rows as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

// CSR sparse matrix times dense node-feature matrix: C = A * B.
// A: n_rows x n_cols (CSR). B: n_cols x feat (row-major f64). c_out: n_rows x feat.
pub fn gpu_csr_spmm(
	values: &GpuBuffer,
	col_idx: &GpuBuffer,
	row_ptr: &GpuBuffer,
	dense_b: &GpuBuffer,
	n_rows: usize,
	feat: usize,
	c_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_csr_spmm(
			values.ptr_raw() as *const c_void,
			col_idx.ptr_raw() as *const c_void,
			row_ptr.ptr_raw() as *const c_void,
			dense_b.ptr_raw() as *const c_void,
			c_out.ptr_raw(),
			n_rows as i32,
			feat as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

// Scatter-based neighbor aggregation over edges given as (src, dst) i32 index lists.
// features: n_nodes x feat (f64). edge_src/edge_dst: n_edges (i32).
// mean=1 divides each node's aggregated features by its in-degree.
// deg_ws: caller-provided n_nodes f64 scratch. agg_out: n_nodes x feat (f64).
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
	agg_out.memset_zero(n_nodes * feat * std::mem::size_of::<f64>())?;
	deg_ws.memset_zero(n_nodes * std::mem::size_of::<f64>())?;
	if mean != 0 {
		unsafe {
			launch_degree(
				edge_dst.ptr_raw() as *const c_void,
				deg_ws.ptr_raw(),
				n_edges as i32,
				std::ptr::null_mut(),
			);
		}
		crate::kernels::check_launch();
	}
	unsafe {
		launch_neighbor_aggregate(
			features.ptr_raw() as *const c_void,
			edge_src.ptr_raw() as *const c_void,
			edge_dst.ptr_raw() as *const c_void,
			agg_out.ptr_raw(),
			deg_ws.ptr_raw() as *const c_void,
			n_nodes as i32,
			feat as i32,
			n_edges as i32,
			mean as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

// Compute in-degree for each node as f64 from an edge list (i32 dst indices).
// deg_out: n_nodes (f64); zeroed internally before the scatter.
pub fn gpu_degree(
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
	deg_out: &GpuBuffer,
) -> Result<(), HipError> {
	deg_out.memset_zero(n_nodes * std::mem::size_of::<f64>())?;
	unsafe {
		launch_degree(
			edge_dst.ptr_raw() as *const c_void,
			deg_out.ptr_raw(),
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

// Scale each row of a node-feature matrix by deg[node]^{-1/2} in place (GCN normalization).
// features: n_nodes x feat (f64, in-place out). deg: n_nodes (f64).
// in-place: writes features (D^-1/2 A D^-1/2 normalization)
pub fn gpu_gcn_norm(
	deg: &GpuBuffer,
	n_nodes: usize,
	feat: usize,
	features: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gcn_norm(
			features.ptr_raw(),
			deg.ptr_raw() as *const c_void,
			n_nodes as i32,
			feat as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}
