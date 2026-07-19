use crate::HipError;
use crate::kernels::{check_launch, ci, cu, safe_i32};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

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

#[inline]
pub fn gpu_iota(n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `out` is a valid device buffer of at least `n` elements; the null stream is the default stream.
	unsafe {
		launch_iota(out.ptr_raw(), n_i32, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

#[must_use]
#[inline]
pub fn gpu_random_permutation_workspace_bytes(n: usize) -> usize {
	let n_i32 = safe_i32(n);
	// SAFETY: `radix_perm_workspace_bytes` only reads `n` to size scratch; the null stream is the default stream.
	unsafe { return radix_perm_workspace_bytes(n_i32, ptr::null_mut()) }
}

#[inline]
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
	let stream = ptr::null_mut();
	let seed_u32 = cu(seed)?;
	let n_i32 = ci(n)?;
	// SAFETY: `keys` is a valid device buffer of at least `n` elements; the LCG fills it in place.
	unsafe {
		launch_lcg_rand(keys.ptr_raw(), n_i32, seed_u32, stream);
	}
	// SAFETY: `iota_scratch` is a valid device buffer of at least `n` elements.
	unsafe {
		launch_iota(iota_scratch.ptr_raw(), n_i32, stream);
	}
	check_launch();
	// SAFETY: every buffer is a valid device allocation sized for `n`; `tmp` holds `tmp_bytes` of scratch.
	unsafe {
		launch_radix_sort_perm(
			keys.ptr_raw(),
			keys_out.ptr_raw(),
			iota_scratch.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			tmp.ptr_raw(),
			tmp_bytes,
			stream,
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
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
	let n_i32 = ci(n)?;
	let n_categories_i32 = ci(n_categories)?;
	// SAFETY: all pointers are valid device buffers sized for `n`/`n_categories`; the null stream is the default stream.
	unsafe {
		launch_ordered_target_stats(
			cat_col_i32.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			perm_i32.ptr_raw().cast_const(),
			out.ptr_raw(),
			cat_sum.ptr_raw(),
			cat_cnt.ptr_raw(),
			n_i32,
			n_categories_i32,
			prior.ptr_raw().cast_const(),
			smoothing.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
