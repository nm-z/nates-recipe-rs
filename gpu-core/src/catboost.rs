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
		prior: f64,
		smoothing: f64,
		stream: *mut c_void,
	);
}

// ── Public API ─────────────────────────────────────────────────────────────────

// gpu_iota
// Returns GpuBuffer of i32[n] containing [0, 1, 2, ..., n-1].
pub fn gpu_iota(n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	unsafe {
		launch_iota(out.ptr_raw(), n as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(out)
}

// gpu_random_permutation
// Returns GpuBuffer of i32[n] — a random permutation of [0..n-1].
// seed: u32 determines the random draw; different seeds give different permutations.
// Implementation: draw uniform f64 keys via per-element LCG (launch_lcg_rand) and an
// iota of indices, then rocPRIM radix-sort the (key, index) pairs ascending — O(n),
// no power-of-two padding. The reordered indices are the permutation.
pub fn gpu_random_permutation(n: usize, seed: u32) -> Result<GpuBuffer, HipError> {
	let stream = std::ptr::null_mut();
	let keys = GpuBuffer::alloc(n)?; // f64[n] random keys
	let keys_out = GpuBuffer::alloc(n)?; // f64[n] sorted keys (discarded)
	let vals_in = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?; // i32[n] iota
	let vals_out = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?; // i32[n] permutation
	unsafe {
		launch_lcg_rand(keys.ptr_raw(), n as i32, seed, stream);
		launch_iota(vals_in.ptr_raw(), n as i32, stream);
	}
	check_launch();
	let tmp_bytes = unsafe { radix_perm_workspace_bytes(n as i32, stream) };
	let tmp = GpuBuffer::alloc_bytes(tmp_bytes.max(1))?;
	unsafe {
		launch_radix_sort_perm(
			keys.ptr_raw(),
			keys_out.ptr_raw(),
			vals_in.ptr_raw() as *const c_void,
			vals_out.ptr_raw(),
			n as i32,
			tmp.ptr_raw(),
			tmp_bytes,
			stream,
		);
	}
	check_launch();
	Ok(vals_out)
}

// gpu_ordered_target_stats
// CatBoost ordered (leakage-free) target statistics encoder.
//
// cat_col_i32: GpuBuffer i32[n]   — category index per row (0-based, 0..n_categories-1)
// target:      GpuBuffer f64[n]   — regression target per row
// perm_i32:    GpuBuffer i32[n]   — random permutation from gpu_random_permutation
// n_categories: number of distinct categories
// prior:        prior value added to numerator (prior * smoothing)
// smoothing:    denominator additive term (avoids division by zero; controls strength)
//
// Returns GpuBuffer f64[n] where encoded[row] is the target statistic for that row,
// computed using only rows that appear BEFORE row in the permutation order, so the
// target never leaks into its own statistic.
//
// Formula (per position p in permutation, row = perm[p]):
//   TS = (sum_{j<p, cat_col[perm[j]]==cat} target[perm[j]] + prior * smoothing)
//        / (count_{j<p, cat_col[perm[j]]==cat} + smoothing)
//
// The kernel needs per-category running accumulators (cat_sum, cat_cnt). These are
// caller-owned scratch (n_categories f64 each) and must be zero-filled before launch.
pub fn gpu_ordered_target_stats(
	cat_col_i32: &GpuBuffer,
	target: &GpuBuffer,
	perm_i32: &GpuBuffer,
	n: usize,
	n_categories: usize,
	prior: f64,
	smoothing: f64,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	let cat_sum = GpuBuffer::zeros_bytes(n_categories * std::mem::size_of::<f64>())?;
	let cat_cnt = GpuBuffer::zeros_bytes(n_categories * std::mem::size_of::<f64>())?;
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
			prior,
			smoothing,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}
