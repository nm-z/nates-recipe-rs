use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

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

pub fn gpu_bootstrap_sample(
	uniform_ws: &GpuBuffer,
	n: usize,
	n_samples: usize,
	seed: usize,
	idx_out: &GpuBuffer,
) -> Result<(), HipError> {
	let _ = seed;
	unsafe {
		launch_floor_scale_to_idx(
			uniform_ws.ptr_raw() as *const c_void,
			idx_out.ptr_raw(),
			n_samples as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_feature_subset(
	keys_ws: &GpuBuffer,
	n_features: usize,
	k: usize,
	seed: usize,
	idx_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_feature_subset(
			keys_ws.ptr_raw(),
			idx_out.ptr_raw(),
			n_features as i32,
			k as i32,
			seed as u32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_random_threshold_split(
	feature_col: &GpuBuffer,
	d_min_ws: &GpuBuffer,
	d_max_ws: &GpuBuffer,
	n: usize,
	seed: usize,
	threshold_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_random_threshold_split(
			feature_col.ptr_raw() as *const c_void,
			d_min_ws.ptr_raw(),
			d_max_ws.ptr_raw(),
			threshold_out.ptr_raw(),
			n as i32,
			seed as u32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_tree_ensemble_predict(
			bins.ptr_raw() as *const c_void,
			node_feature.ptr_raw() as *const c_void,
			node_thresh.ptr_raw() as *const c_void,
			node_left.ptr_raw() as *const c_void,
			node_right.ptr_raw() as *const c_void,
			node_is_leaf.ptr_raw() as *const c_void,
			node_value.ptr_raw() as *const c_void,
			tree_root.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			n_trees as i32,
			lr.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_oob_mask(
	bootstrap_idx: &GpuBuffer,
	used_ws: &GpuBuffer,
	n_samples: usize,
	n: usize,
	oob_out: &GpuBuffer,
) -> Result<(), HipError> {
	used_ws.memset_zero(n)?;
	unsafe {
		launch_oob_mask(
			bootstrap_idx.ptr_raw() as *const c_void,
			used_ws.ptr_raw(),
			oob_out.ptr_raw(),
			n_samples as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
