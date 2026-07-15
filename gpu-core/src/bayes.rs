use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

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

/// Accumulates per-class feature count totals from `x_counts` and labels `y` into `out`.
///
/// # Errors
/// Returns [`HipError`] if `n`, `n_features`, or `n_classes` does not fit in [`i32`].
#[inline]
pub fn gpu_nb_count_table(
	x_counts: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let n_features_i32 = ci(n_features)?;
	let n_classes_i32 = ci(n_classes)?;
	// SAFETY: `x_counts`, `y`, and `out` are valid device buffers sized for the given dims.
	unsafe {
		launch_nb_count_table(
			x_counts.ptr_raw().cast_const(),
			y.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			n_features_i32,
			n_classes_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Computes smoothed per-class feature log-probabilities from `count_table` into `out`.
///
/// # Errors
/// Returns [`HipError`] if `n_classes` or `n_features` does not fit in [`i32`].
#[inline]
pub fn gpu_nb_feature_log_prob(
	count_table: &GpuBuffer,
	alpha: &GpuBuffer,
	n_classes: usize,
	n_features: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_classes_i32 = ci(n_classes)?;
	let n_features_i32 = ci(n_features)?;
	// SAFETY: `count_table`, `alpha`, and `out` are valid device buffers sized for the given dims.
	unsafe {
		launch_nb_feature_log_prob(
			count_table.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_classes_i32,
			n_features_i32,
			alpha.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Computes multinomial naive-Bayes class log-probabilities for each row of `x` into `out`.
///
/// # Errors
/// Returns [`HipError`] if `n`, `n_features`, or `n_classes` does not fit in [`i32`].
#[inline]
pub fn gpu_multinomial_nb_logprob(
	log_class_prior: &GpuBuffer,
	feature_log_prob: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	n_features: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let n_features_i32 = ci(n_features)?;
	let n_classes_i32 = ci(n_classes)?;
	// SAFETY: `log_class_prior`, `feature_log_prob`, `x`, and `out` are valid device buffers sized for the given dims.
	unsafe {
		launch_multinomial_nb_logprob(
			log_class_prior.ptr_raw().cast_const(),
			feature_log_prob.ptr_raw().cast_const(),
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			n_features_i32,
			n_classes_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Computes Bernoulli naive-Bayes class log-probabilities for each row of `x_binary` into `out`.
///
/// # Errors
/// Returns [`HipError`] if `n`, `n_features`, or `n_classes` does not fit in [`i32`].
#[inline]
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
	let n_i32 = ci(n)?;
	let n_features_i32 = ci(n_features)?;
	let n_classes_i32 = ci(n_classes)?;
	// SAFETY: `log_class_prior`, `feature_log_prob`, `feature_log_neg`, `x_binary`, and `out` are valid device buffers sized for the given dims.
	unsafe {
		launch_bernoulli_nb_logprob(
			log_class_prior.ptr_raw().cast_const(),
			feature_log_prob.ptr_raw().cast_const(),
			feature_log_neg.ptr_raw().cast_const(),
			x_binary.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			n_features_i32,
			n_classes_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
