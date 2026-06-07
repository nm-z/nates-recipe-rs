use crate::hip::HipError;
use crate::kernels::{check_launch, gpu_rand_uniform};
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
		lr: f64,
		stream: *mut c_void,
	);
}

pub fn gpu_bootstrap_sample(n: usize, n_samples: usize, seed: u32) -> Result<GpuBuffer, HipError> {
	let uniform = gpu_rand_uniform(n_samples, seed)?;
	let out = GpuBuffer::alloc_bytes(n_samples * 4)?;
	unsafe {
		launch_floor_scale_to_idx(
			uniform.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n_samples as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_feature_subset(n_features: usize, k: usize, seed: u32) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc_bytes(n_features * 4)?;
	let keys = GpuBuffer::alloc(n_features)?;
	unsafe {
		launch_feature_subset(
			keys.ptr_raw(),
			out.ptr_raw(),
			n_features as i32,
			k as i32,
			seed,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_random_threshold_split(
	feature_col: &GpuBuffer,
	n: usize,
	seed: u32,
) -> Result<f64, HipError> {
	let out = GpuBuffer::alloc(1)?;
	let d_min = GpuBuffer::alloc(1)?;
	let d_max = GpuBuffer::alloc(1)?;
	unsafe {
		launch_random_threshold_split(
			feature_col.ptr_raw() as *const c_void,
			d_min.ptr_raw(),
			d_max.ptr_raw(),
			out.ptr_raw(),
			n as i32,
			seed,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut result = [0.0f64];
	out.download(&mut result)?;
	Ok(result[0])
}

// GPU forest inference for leaf-wise trees with arbitrary (global) child
// indices. `bins_flat` is feature-major [n_eff*n] u8; node arrays are the
// concatenation of every tree's nodes; `tree_root` holds each tree's global
// root index. Returns lr * sum_t leaf_value per sample (length n).
pub fn gpu_tree_ensemble_predict(
	bins_flat: &[u8],
	node_feature: &[i32],
	node_thresh: &[i32],
	node_left: &[i32],
	node_right: &[i32],
	node_is_leaf: &[u8],
	node_value: &[f64],
	tree_root: &[i32],
	n: usize,
	lr: f64,
) -> Result<Vec<f64>, HipError> {
	let bins_gpu = GpuBuffer::upload_u8(bins_flat)?;
	let feat_gpu = GpuBuffer::upload_i32(node_feature)?;
	let thr_gpu = GpuBuffer::upload_i32(node_thresh)?;
	let left_gpu = GpuBuffer::upload_i32(node_left)?;
	let right_gpu = GpuBuffer::upload_i32(node_right)?;
	let leaf_gpu = GpuBuffer::upload_u8(node_is_leaf)?;
	let val_gpu = GpuBuffer::upload(node_value)?;
	let root_gpu = GpuBuffer::upload_i32(tree_root)?;
	let out_gpu = GpuBuffer::alloc(n)?;
	unsafe {
		launch_tree_ensemble_predict(
			bins_gpu.ptr_raw() as *const c_void,
			feat_gpu.ptr_raw() as *const c_void,
			thr_gpu.ptr_raw() as *const c_void,
			left_gpu.ptr_raw() as *const c_void,
			right_gpu.ptr_raw() as *const c_void,
			leaf_gpu.ptr_raw() as *const c_void,
			val_gpu.ptr_raw() as *const c_void,
			root_gpu.ptr_raw() as *const c_void,
			out_gpu.ptr_raw(),
			n as i32,
			tree_root.len() as i32,
			lr,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut out = vec![0.0f64; n];
	out_gpu.download(&mut out)?;
	Ok(out)
}

pub fn gpu_oob_mask(bootstrap_idx_i32: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let n_samples = bootstrap_idx_i32.len() / 4;
	let out = GpuBuffer::zeros_bytes(n)?;
	let used = GpuBuffer::zeros_bytes(n)?;
	unsafe {
		launch_oob_mask(
			bootstrap_idx_i32.ptr_raw() as *const c_void,
			used.ptr_raw(),
			out.ptr_raw(),
			n_samples as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}
