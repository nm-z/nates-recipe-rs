use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// ── FFI: catboost.hip ─────────────────────────────────────────────────────────

unsafe extern "C" {
	fn launch_iota(out: *mut c_void, n: i32, stream: *mut c_void);

	// Random f64 keys (per-element LCG); argsorting them gives the permutation.
	fn launch_lcg_rand(out: *mut c_void, n: i32, seed: u32, stream: *mut c_void);

	// O(n) permutation via rocPRIM radix sort of (key, index) pairs. Caller owns
	// the temp (sized by radix_perm_workspace_bytes) and the double-buffer outputs.
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

// ── Public API ─────────────────────────────────────────────────────────────────

// gpu_iota
// Fills `out` (i32[n]) with [0, 1, 2, ..., n-1].
pub fn gpu_iota(n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_iota(out.ptr_raw(), n as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

// not-an-op: plan-time helper — rocPRIM radix-sort workspace byte-size query
pub fn gpu_random_permutation_workspace_bytes(n: usize) -> usize {
	unsafe { radix_perm_workspace_bytes(n as i32, std::ptr::null_mut()) }
}

// gpu_random_permutation
// Writes a random permutation of [0..n-1] (i32[n]) into `out`.
// seed determines the random draw; different seeds give different permutations.
// Implementation: draw uniform f64 keys via per-element LCG (launch_lcg_rand) into
// `keys`, an iota of indices into `iota_scratch`, then rocPRIM radix-sort the
// (key, index) pairs ascending — O(n), no power-of-two padding. `keys_out` receives
// the sorted keys (discarded) and `tmp` (tmp_bytes from
// gpu_random_permutation_workspace_bytes) is the rocPRIM scratch. All scratch is
// caller-owned.
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

// gpu_ordered_target_stats
// CatBoost ordered (leakage-free) target statistics encoder.
//
// cat_col_i32: GpuBuffer i32[n]   — category index per row (0-based, 0..n_categories-1)
// target:      GpuBuffer f64[n]   — regression target per row
// perm_i32:    GpuBuffer i32[n]   — random permutation from gpu_random_permutation
// prior:       1-elem device buffer — prior value added to numerator (prior * smoothing)
// smoothing:   1-elem device buffer — denominator additive term
// cat_sum/cat_cnt: caller-owned scratch (n_categories f64 each), zero-filled before launch
// n_categories: number of distinct categories
//
// Writes f64[n] into `out` where out[row] is the target statistic for that row,
// computed using only rows that appear BEFORE row in the permutation order, so the
// target never leaks into its own statistic.
//
// Formula (per position p in permutation, row = perm[p]):
//   TS = (sum_{j<p, cat_col[perm[j]]==cat} target[perm[j]] + prior * smoothing)
//        / (count_{j<p, cat_col[perm[j]]==cat} + smoothing)
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
