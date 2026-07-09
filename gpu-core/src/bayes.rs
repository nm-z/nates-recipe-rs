use crate::hip::HipError;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_nb_count_table(
		x_counts: *const c_void,
		y: *const c_void,
		out: *mut c_void,
		n: i32,
		n_features: i32,
		n_classes: i32,
		stream: *mut c_void,
	);
	fn launch_nb_feature_log_prob(
		count_table: *const c_void,
		out: *mut c_void,
		n_classes: i32,
		n_features: i32,
		alpha: *const c_void,
		stream: *mut c_void,
	);
	fn launch_multinomial_nb_logprob(
		log_class_prior: *const c_void,
		feature_log_prob: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		n_features: i32,
		n_classes: i32,
		stream: *mut c_void,
	);
	fn launch_bernoulli_nb_logprob(
		log_class_prior: *const c_void,
		feature_log_prob: *const c_void,
		feature_log_neg: *const c_void,
		x_binary: *const c_void,
		out: *mut c_void,
		n: i32,
		n_features: i32,
		n_classes: i32,
		stream: *mut c_void,
	);
}

pub fn gpu_nb_count_table(
	x_counts: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_nb_count_table(
			x_counts.ptr_raw() as *const c_void,
			y.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			n_features as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

pub fn gpu_nb_feature_log_prob(
	count_table: &GpuBuffer,
	alpha: &GpuBuffer,
	n_classes: usize,
	n_features: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_nb_feature_log_prob(
			count_table.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n_classes as i32,
			n_features as i32,
			alpha.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

pub fn gpu_multinomial_nb_logprob(
	log_class_prior: &GpuBuffer,
	feature_log_prob: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_multinomial_nb_logprob(
			log_class_prior.ptr_raw() as *const c_void,
			feature_log_prob.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			n_features as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}

pub fn gpu_bernoulli_nb_logprob(
	log_class_prior: &GpuBuffer,
	feature_log_prob: &GpuBuffer,
	feature_log_neg: &GpuBuffer,
	x_binary: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bernoulli_nb_logprob(
			log_class_prior.ptr_raw() as *const c_void,
			feature_log_prob.ptr_raw() as *const c_void,
			feature_log_neg.ptr_raw() as *const c_void,
			x_binary.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			n_features as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(())
}
