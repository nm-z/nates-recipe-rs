use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_discounted_returns(
		rewards: *const c_void,
		returns: *mut c_void,
		gamma: *const c_void,
		t_len: i32,
		stream: *mut c_void,
	);

	fn launch_gae(
		rewards: *const c_void,
		values: *const c_void,
		advantages: *mut c_void,
		gamma: *const c_void,
		lam: *const c_void,
		t_len: i32,
		stream: *mut c_void,
	);

	fn launch_td_targets(
		rewards: *const c_void,
		values_next: *const c_void,
		done_mask: *const c_void,
		targets: *mut c_void,
		gamma: *const c_void,
		n: i32,
		stream: *mut c_void,
	);

	fn launch_categorical_logprob(
		logits: *const c_void,
		actions: *const c_void,
		logp: *mut c_void,
		n: i32,
		n_actions: i32,
		stream: *mut c_void,
	);

	fn launch_gaussian_logprob(
		mu: *const c_void,
		log_std: *const c_void,
		actions: *const c_void,
		logp: *mut c_void,
		n: i32,
		dim: i32,
		stream: *mut c_void,
	);
}

pub fn gpu_discounted_returns(
	rewards: &GpuBuffer,
	gamma: &GpuBuffer,
	t_len: usize,
	returns: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_discounted_returns(
			rewards.ptr_raw() as *const c_void,
			returns.ptr_raw(),
			gamma.ptr_raw() as *const c_void,
			t_len as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_gae(
	rewards: &GpuBuffer,
	values: &GpuBuffer,
	gamma: &GpuBuffer,
	lam: &GpuBuffer,
	t_len: usize,
	advantages: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gae(
			rewards.ptr_raw() as *const c_void,
			values.ptr_raw() as *const c_void,
			advantages.ptr_raw(),
			gamma.ptr_raw() as *const c_void,
			lam.ptr_raw() as *const c_void,
			t_len as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_td_targets(
	rewards: &GpuBuffer,
	values_next: &GpuBuffer,
	done_mask: &GpuBuffer,
	gamma: &GpuBuffer,
	n: usize,
	targets: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_td_targets(
			rewards.ptr_raw() as *const c_void,
			values_next.ptr_raw() as *const c_void,
			done_mask.ptr_raw() as *const c_void,
			targets.ptr_raw(),
			gamma.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_categorical_logprob(
	logits: &GpuBuffer,
	actions_i32: &GpuBuffer,
	n: usize,
	n_actions: usize,
	logp: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_categorical_logprob(
			logits.ptr_raw() as *const c_void,
			actions_i32.ptr_raw() as *const c_void,
			logp.ptr_raw(),
			n as i32,
			n_actions as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_gaussian_logprob(
	mu: &GpuBuffer,
	log_std: &GpuBuffer,
	actions: &GpuBuffer,
	n: usize,
	dim: usize,
	logp: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gaussian_logprob(
			mu.ptr_raw() as *const c_void,
			log_std.ptr_raw() as *const c_void,
			actions.ptr_raw() as *const c_void,
			logp.ptr_raw(),
			n as i32,
			dim as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
