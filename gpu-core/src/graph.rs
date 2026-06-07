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
// x has n_cols elements (f64); returns y[n_rows] (f64).
pub fn gpu_csr_spmv(
	values: &GpuBuffer,
	col_idx: &GpuBuffer,
	row_ptr: &GpuBuffer,
	x: &GpuBuffer,
	n_rows: usize,
) -> Result<GpuBuffer, HipError> {
	let y = GpuBuffer::alloc(n_rows)?;
	unsafe {
		launch_csr_spmv(
			values.ptr_raw() as *const c_void,
			col_idx.ptr_raw() as *const c_void,
			row_ptr.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			y.ptr_raw(),
			n_rows as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(y)
}

// CSR sparse matrix times dense node-feature matrix: C = A * B.
// A: n_rows x n_cols (CSR). B: n_cols x feat (row-major f64). Returns C: n_rows x feat.
pub fn gpu_csr_spmm(
	values: &GpuBuffer,
	col_idx: &GpuBuffer,
	row_ptr: &GpuBuffer,
	dense_b: &GpuBuffer,
	n_rows: usize,
	feat: usize,
) -> Result<GpuBuffer, HipError> {
	let c = GpuBuffer::alloc(n_rows * feat)?;
	unsafe {
		launch_csr_spmm(
			values.ptr_raw() as *const c_void,
			col_idx.ptr_raw() as *const c_void,
			row_ptr.ptr_raw() as *const c_void,
			dense_b.ptr_raw() as *const c_void,
			c.ptr_raw(),
			n_rows as i32,
			feat as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(c)
}

// Scatter-based neighbor aggregation over edges given as (src, dst) i32 index lists.
// features: n_nodes x feat (f64). edge_src/edge_dst: n_edges (i32).
// mean=true divides each node's aggregated features by its in-degree.
// Returns aggregated: n_nodes x feat (f64).
pub fn gpu_neighbor_aggregate(
	features: &GpuBuffer,
	edge_src: &GpuBuffer,
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	feat: usize,
	n_edges: usize,
	mean: bool,
) -> Result<GpuBuffer, HipError> {
	let agg = GpuBuffer::alloc(n_nodes * feat)?;
	agg.memset_zero(n_nodes * feat * std::mem::size_of::<f64>())?;
	// Build degree vector (f64) from edge_dst so the mean kernel can read it.
	let deg = GpuBuffer::alloc(n_nodes)?;
	deg.memset_zero(n_nodes * std::mem::size_of::<f64>())?;
	if mean {
		unsafe {
			launch_degree(
				edge_dst.ptr_raw() as *const c_void,
				deg.ptr_raw(),
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
			agg.ptr_raw(),
			deg.ptr_raw() as *const c_void,
			n_nodes as i32,
			feat as i32,
			n_edges as i32,
			mean as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(agg)
}

// Compute in-degree for each node as f64 from an edge list (i32 dst indices).
// Returns deg[n_nodes] (f64).
pub fn gpu_degree(
	edge_dst: &GpuBuffer,
	n_nodes: usize,
	n_edges: usize,
) -> Result<GpuBuffer, HipError> {
	let deg = GpuBuffer::alloc(n_nodes)?;
	deg.memset_zero(n_nodes * std::mem::size_of::<f64>())?;
	unsafe {
		launch_degree(
			edge_dst.ptr_raw() as *const c_void,
			deg.ptr_raw(),
			n_edges as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(deg)
}

// Scale each row of a node-feature matrix by deg[node]^{-1/2} in place (GCN normalization).
// features: n_nodes x feat (f64, mutable). deg: n_nodes (f64).
pub fn gpu_gcn_norm(
	features: &GpuBuffer,
	deg: &GpuBuffer,
	n_nodes: usize,
	feat: usize,
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
