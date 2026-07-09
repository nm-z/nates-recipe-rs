use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

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

pub fn gpu_forward_backward(
	log_trans: &GpuBuffer,
	log_emit: &GpuBuffer,
	n_states: usize,
	t_len: usize,
	log_alpha_out: &GpuBuffer,
	log_beta_out: &GpuBuffer,
	log_gamma_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_forward_backward(
			log_trans.ptr_raw() as *const c_void,
			log_emit.ptr_raw() as *const c_void,
			log_alpha_out.ptr_raw(),
			log_beta_out.ptr_raw(),
			log_gamma_out.ptr_raw(),
			n_states as i32,
			t_len as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_viterbi(
	log_trans: &GpuBuffer,
	log_emit: &GpuBuffer,
	n_states: usize,
	t_len: usize,
	delta: &GpuBuffer,
	backptr: &GpuBuffer,
	best_path_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_viterbi(
			log_trans.ptr_raw() as *const c_void,
			log_emit.ptr_raw() as *const c_void,
			delta.ptr_raw(),
			backptr.ptr_raw(),
			best_path_out.ptr_raw(),
			n_states as i32,
			t_len as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
