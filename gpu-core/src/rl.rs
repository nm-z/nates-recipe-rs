use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

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

#[inline]
pub fn gpu_discounted_returns(
	rewards: &GpuBuffer,
	gamma: &GpuBuffer,
	t_len: usize,
	returns: &GpuBuffer,
) -> Result<(), HipError> {
	let t_len_i32 = ci(t_len)?;
	// SAFETY: buffer pointers are valid device allocations and t_len bounds the launch grid.
	unsafe {
		launch_discounted_returns(
			rewards.ptr_raw().cast_const(),
			returns.ptr_raw(),
			gamma.ptr_raw().cast_const(),
			t_len_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_gae(
	rewards: &GpuBuffer,
	values: &GpuBuffer,
	gamma: &GpuBuffer,
	lam: &GpuBuffer,
	t_len: usize,
	advantages: &GpuBuffer,
) -> Result<(), HipError> {
	let t_len_i32 = ci(t_len)?;
	// SAFETY: buffer pointers are valid device allocations and t_len bounds the launch grid.
	unsafe {
		launch_gae(
			rewards.ptr_raw().cast_const(),
			values.ptr_raw().cast_const(),
			advantages.ptr_raw(),
			gamma.ptr_raw().cast_const(),
			lam.ptr_raw().cast_const(),
			t_len_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_td_targets(
	rewards: &GpuBuffer,
	values_next: &GpuBuffer,
	done_mask: &GpuBuffer,
	gamma: &GpuBuffer,
	n: usize,
	targets: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: buffer pointers are valid device allocations and n bounds the launch grid.
	unsafe {
		launch_td_targets(
			rewards.ptr_raw().cast_const(),
			values_next.ptr_raw().cast_const(),
			done_mask.ptr_raw().cast_const(),
			targets.ptr_raw(),
			gamma.ptr_raw().cast_const(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_categorical_logprob(
	logits: &GpuBuffer,
	actions_i32: &GpuBuffer,
	n: usize,
	n_actions: usize,
	logp: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let n_actions_i32 = ci(n_actions)?;
	// SAFETY: buffer pointers are valid device allocations and n bounds the launch grid.
	unsafe {
		launch_categorical_logprob(
			logits.ptr_raw().cast_const(),
			actions_i32.ptr_raw().cast_const(),
			logp.ptr_raw(),
			n_i32,
			n_actions_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_gaussian_logprob(
	mu: &GpuBuffer,
	log_std: &GpuBuffer,
	actions: &GpuBuffer,
	n: usize,
	dim: usize,
	logp: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let dim_i32 = ci(dim)?;
	// SAFETY: buffer pointers are valid device allocations and n bounds the launch grid.
	unsafe {
		launch_gaussian_logprob(
			mu.ptr_raw().cast_const(),
			log_std.ptr_raw().cast_const(),
			actions.ptr_raw().cast_const(),
			logp.ptr_raw(),
			n_i32,
			dim_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
