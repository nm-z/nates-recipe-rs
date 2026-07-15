use crate::HipError;
use crate::kernels::{check_launch, ci, cu};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

unsafe extern "C" {
	fn launch_floor_scale_to_idx(
		uniform: *const c_void,
		idx_out: *mut c_void,
		n_samples: i32,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_feature_subset(
		keys: *mut c_void,
		idx_out: *mut c_void,
		n_features: i32,
		k: i32,
		seed: u32,
		stream: *mut c_void,
	);
	fn launch_random_threshold_split(
		col: *const c_void,
		d_min: *mut c_void,
		d_max: *mut c_void,
		threshold_out: *mut c_void,
		n: i32,
		seed: u32,
		stream: *mut c_void,
	);
	fn launch_oob_mask(
		bootstrap_idx: *const c_void,
		used: *mut c_void,
		oob_out: *mut c_void,
		n_samples: i32,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_tree_ensemble_predict(
		bins: *const c_void,
		node_feature: *const c_void,
		node_thresh: *const c_void,
		node_left: *const c_void,
		node_right: *const c_void,
		node_is_leaf: *const c_void,
		node_value: *const c_void,
		tree_root: *const c_void,
		out: *mut c_void,
		n: i32,
		n_trees: i32,
		lr: *const c_void,
		stream: *mut c_void,
	);
}

/// # Errors
/// Returns `HipError` when `n` or `n_samples` overflows `i32`.
#[inline]
pub fn gpu_bootstrap_sample(
	uniform_ws: &GpuBuffer,
	n: usize,
	n_samples: usize,
	_seed: usize,
	idx_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_samples32 = ci(n_samples)?;
	let n32 = ci(n)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_floor_scale_to_idx(
			uniform_ws.ptr_raw().cast_const(),
			idx_out.ptr_raw(),
			n_samples32,
			n32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n_features`, `k`, or `seed` overflows its target width.
#[inline]
pub fn gpu_feature_subset(
	keys_ws: &GpuBuffer,
	n_features: usize,
	k: usize,
	seed: usize,
	idx_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_features32 = ci(n_features)?;
	let k32 = ci(k)?;
	let seed32 = cu(seed)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_feature_subset(
			keys_ws.ptr_raw(),
			idx_out.ptr_raw(),
			n_features32,
			k32,
			seed32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n` or `seed` overflows its target width.
#[inline]
pub fn gpu_random_threshold_split(
	feature_col: &GpuBuffer,
	d_min_ws: &GpuBuffer,
	d_max_ws: &GpuBuffer,
	n: usize,
	seed: usize,
	threshold_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n32 = ci(n)?;
	let seed32 = cu(seed)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_random_threshold_split(
			feature_col.ptr_raw().cast_const(),
			d_min_ws.ptr_raw(),
			d_max_ws.ptr_raw(),
			threshold_out.ptr_raw(),
			n32,
			seed32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n` or `n_trees` overflows `i32`.
#[inline]
pub fn gpu_tree_ensemble_predict(
	bins: &GpuBuffer,
	node_feature: &GpuBuffer,
	node_thresh: &GpuBuffer,
	node_left: &GpuBuffer,
	node_right: &GpuBuffer,
	node_is_leaf: &GpuBuffer,
	node_value: &GpuBuffer,
	tree_root: &GpuBuffer,
	lr: &GpuBuffer,
	n: usize,
	n_trees: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n32 = ci(n)?;
	let n_trees32 = ci(n_trees)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_tree_ensemble_predict(
			bins.ptr_raw().cast_const(),
			node_feature.ptr_raw().cast_const(),
			node_thresh.ptr_raw().cast_const(),
			node_left.ptr_raw().cast_const(),
			node_right.ptr_raw().cast_const(),
			node_is_leaf.ptr_raw().cast_const(),
			node_value.ptr_raw().cast_const(),
			tree_root.ptr_raw().cast_const(),
			out.ptr_raw(),
			n32,
			n_trees32,
			lr.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when the zero-fill fails or `n_samples` or `n` overflows `i32`.
#[inline]
pub fn gpu_oob_mask(
	bootstrap_idx: &GpuBuffer,
	used_ws: &GpuBuffer,
	n_samples: usize,
	n: usize,
	oob_out: &GpuBuffer,
) -> Result<(), HipError> {
	used_ws.memset_zero(n)?;
	let n_samples32 = ci(n_samples)?;
	let n32 = ci(n)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_oob_mask(
			bootstrap_idx.ptr_raw().cast_const(),
			used_ws.ptr_raw(),
			oob_out.ptr_raw(),
			n_samples32,
			n32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
