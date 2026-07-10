use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

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
	);
}

pub const ROPE_THETA: f64 = 10000.0;

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
	unsafe {
		launch_ropex_qk_heads(
			q.ptr_raw(),
			k.ptr_raw(),
			m as i32,
			d as i32,
			heads as i32,
			seq as i32,
			theta.ptr_raw() as *const c_void,
			sgn.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_ropex_qk(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			positions.ptr_raw() as *const c_void,
			qo.ptr_raw(),
			ko.ptr_raw(),
			n_rows as i32,
			dim as i32,
			theta.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
