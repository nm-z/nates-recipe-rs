use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_iota(out: *mut c_void, n: i32, stream: *mut c_void);

	fn launch_lcg_rand(out: *mut c_void, n: i32, seed: u32, stream: *mut c_void);

	fn radix_perm_workspace_bytes(n: i32, stream: *mut c_void) -> usize;

	fn launch_radix_sort_perm(
		keys: *mut c_void,
		keys_out: *mut c_void,
		vals_in: *const c_void,
		vals_out: *mut c_void,
		n: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);

	fn launch_ordered_target_stats(
		cat_col: *const c_void,
		target: *const c_void,
		perm: *const c_void,
		encoded_out: *mut c_void,
		cat_sum: *mut c_void,
		cat_cnt: *mut c_void,
		n: i32,
		n_categories: i32,
		prior: *const c_void,
		smoothing: *const c_void,
		stream: *mut c_void,
	);
}

pub fn gpu_iota(n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_iota(out.ptr_raw(), n as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_random_permutation_workspace_bytes(n: usize) -> usize {
	unsafe { radix_perm_workspace_bytes(n as i32, std::ptr::null_mut()) }
}

pub fn gpu_random_permutation(
	keys: &GpuBuffer,
	keys_out: &GpuBuffer,
	iota_scratch: &GpuBuffer,
	tmp: &GpuBuffer,
	n: usize,
	seed: usize,
	tmp_bytes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let stream = std::ptr::null_mut();
	unsafe {
		launch_lcg_rand(keys.ptr_raw(), n as i32, seed as u32, stream);
		launch_iota(iota_scratch.ptr_raw(), n as i32, stream);
	}
	check_launch();
	unsafe {
		launch_radix_sort_perm(
			keys.ptr_raw(),
			keys_out.ptr_raw(),
			iota_scratch.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			tmp.ptr_raw(),
			tmp_bytes,
			stream,
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_ordered_target_stats(
	cat_col_i32: &GpuBuffer,
	target: &GpuBuffer,
	perm_i32: &GpuBuffer,
	prior: &GpuBuffer,
	smoothing: &GpuBuffer,
	cat_sum: &GpuBuffer,
	cat_cnt: &GpuBuffer,
	n: usize,
	n_categories: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_ordered_target_stats(
			cat_col_i32.ptr_raw() as *const c_void,
			target.ptr_raw() as *const c_void,
			perm_i32.ptr_raw() as *const c_void,
			out.ptr_raw(),
			cat_sum.ptr_raw(),
			cat_cnt.ptr_raw(),
			n as i32,
			n_categories as i32,
			prior.ptr_raw() as *const c_void,
			smoothing.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
