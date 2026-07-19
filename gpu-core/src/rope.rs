use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

unsafe extern "C" {
	fn launch_ropex_qk(
		q: *const c_void,
		k: *const c_void,
		pos: *const c_void,
		qo: *mut c_void,
		ko: *mut c_void,
		n_rows: i32,
		d: i32,
		theta: *const c_void,
		stream: *mut c_void,
	);
	fn launch_ropex_qk_heads(
		q: *mut c_void,
		k: *mut c_void,
		m: i32,
		d: i32,
		heads: i32,
		seq: i32,
		theta: *const c_void,
		sgn: *const c_void,
		stream: *mut c_void,
		dtype: i32,
	);
}

pub const ROPE_THETA: f64 = 10000.0;

#[inline]
pub fn gpu_rope_qk_heads_inplace(
	sgn: &GpuBuffer,
	theta: &GpuBuffer,
	m: usize,
	d: usize,
	heads: usize,
	seq: usize,
	q: &GpuBuffer,
	k: &GpuBuffer,
) -> Result<(), HipError> {
	let mi = ci(m)?;
	let di = ci(d)?;
	let headsi = ci(heads)?;
	let seqi = ci(seq)?;
	// SAFETY: FFI launch; all buffer pointers are live GPU allocations and dims are range-checked.
	unsafe {
		launch_ropex_qk_heads(
			q.ptr_raw(),
			k.ptr_raw(),
			mi,
			di,
			headsi,
			seqi,
			theta.ptr_raw().cast_const(),
			sgn.ptr_raw().cast_const(),
			ptr::null_mut(),
			q.dtype().ffi(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_rope_qk(
	q: &GpuBuffer,
	k: &GpuBuffer,
	positions: &GpuBuffer,
	theta: &GpuBuffer,
	n_rows: usize,
	dim: usize,
	qo: &GpuBuffer,
	ko: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i = ci(n_rows)?;
	let dim_i = ci(dim)?;
	// SAFETY: FFI launch; all buffer pointers are live GPU allocations and dims are range-checked.
	unsafe {
		launch_ropex_qk(
			q.ptr_raw().cast_const(),
			k.ptr_raw().cast_const(),
			positions.ptr_raw().cast_const(),
			qo.ptr_raw(),
			ko.ptr_raw(),
			n_rows_i,
			dim_i,
			theta.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
