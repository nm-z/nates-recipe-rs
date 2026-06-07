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
		alpha: f64,
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

/// Accumulate per-class feature count table [n_classes * n_features] via
/// atomicAdd.  `x_counts` is [n * n_features] f64 feature counts; `y` is
/// [n] i32 class labels.  The output buffer must be zeroed by the caller.
pub fn gpu_nb_count_table(
	x_counts: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::zeros_bytes(n_classes * n_features * std::mem::size_of::<f64>())?;
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
	Ok(out)
}

/// Compute smoothed log P(feature|class) from a count table.
/// Returns [n_classes * n_features] with Laplace smoothing parameter `alpha`.
pub fn gpu_nb_feature_log_prob(
	count_table: &GpuBuffer,
	n_classes: usize,
	n_features: usize,
	alpha: f64,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n_classes * n_features)?;
	unsafe {
		launch_nb_feature_log_prob(
			count_table.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n_classes as i32,
			n_features as i32,
			alpha,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(out)
}

/// Multinomial NB log-posterior [n * n_classes].
/// out[i,c] = log_prior[c] + sum_f x[i,f] * feature_log_prob[c,f]
pub fn gpu_multinomial_nb_logprob(
	log_class_prior: &GpuBuffer,
	feature_log_prob: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n * n_classes)?;
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
	Ok(out)
}

/// Bernoulli NB log-posterior [n * n_classes] including the (1-x)*log(1-p) term.
/// out[i,c] = log_prior[c] + sum_f [ x[i,f]*log_p[c,f] + (1-x[i,f])*log_neg[c,f] ]
pub fn gpu_bernoulli_nb_logprob(
	log_class_prior: &GpuBuffer,
	feature_log_prob: &GpuBuffer,
	feature_log_neg: &GpuBuffer,
	x_binary: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n * n_classes)?;
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
	Ok(out)
}
