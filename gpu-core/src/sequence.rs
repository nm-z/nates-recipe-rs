use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

unsafe extern "C" {
	fn launch_forward_backward(
		log_trans: *const c_void,
		log_emit: *const c_void,
		log_alpha: *mut c_void,
		log_beta: *mut c_void,
		log_gamma: *mut c_void,
		n_states: i32,
		t_len: i32,
		stream: *mut c_void,
	);

	fn launch_viterbi(
		log_trans: *const c_void,
		log_emit: *const c_void,
		delta: *mut c_void,
		backptr: *mut c_void,
		best_path: *mut c_void,
		n_states: i32,
		t_len: i32,
		stream: *mut c_void,
	);
}

/// # Errors
/// Returns `HipError` when `n_states` or `t_len` overflows `i32`.
#[inline]
pub fn gpu_forward_backward(
	log_trans: &GpuBuffer,
	log_emit: &GpuBuffer,
	n_states: usize,
	t_len: usize,
	log_alpha_out: &GpuBuffer,
	log_beta_out: &GpuBuffer,
	log_gamma_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_states_i = ci(n_states)?;
	let t_len_i = ci(t_len)?;
	// SAFETY: all buffer pointers are live for the launch and dims are checked i32.
	unsafe {
		launch_forward_backward(
			log_trans.ptr_raw().cast_const(),
			log_emit.ptr_raw().cast_const(),
			log_alpha_out.ptr_raw(),
			log_beta_out.ptr_raw(),
			log_gamma_out.ptr_raw(),
			n_states_i,
			t_len_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n_states` or `t_len` overflows `i32`.
#[inline]
pub fn gpu_viterbi(
	log_trans: &GpuBuffer,
	log_emit: &GpuBuffer,
	n_states: usize,
	t_len: usize,
	delta: &GpuBuffer,
	backptr: &GpuBuffer,
	best_path_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_states_i = ci(n_states)?;
	let t_len_i = ci(t_len)?;
	// SAFETY: all buffer pointers are live for the launch and dims are checked i32.
	unsafe {
		launch_viterbi(
			log_trans.ptr_raw().cast_const(),
			log_emit.ptr_raw().cast_const(),
			delta.ptr_raw(),
			backptr.ptr_raw(),
			best_path_out.ptr_raw(),
			n_states_i,
			t_len_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
