use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::check_launch;

// ── FFI: catboost.hip ─────────────────────────────────────────────────────────

unsafe extern "C" {
      fn launch_iota(
            out: *mut c_void,
            n: i32,
            stream: *mut c_void,
      );

      fn launch_random_permutation(
            perm_out: *mut c_void,
            n: i32,
            seed: u32,
            stream: *mut c_void,
      );

      fn launch_ordered_target_stats(
            cat_col: *const c_void,
            target: *const c_void,
            perm: *const c_void,
            encoded_out: *mut c_void,
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
            launch_iota(
                  out.ptr_raw(),
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// gpu_random_permutation
// Returns GpuBuffer of i32[n] — a random permutation of [0..n-1].
// seed: u32 determines the random draw; different seeds give different permutations.
// Implementation: draw uniform f64 keys via LCG, bitonic argsort the keys,
// argsort indices = permutation.
pub fn gpu_random_permutation(n: usize, seed: u32) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
      unsafe {
            launch_random_permutation(
                  out.ptr_raw(),
                  n as i32,
                  seed,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
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
      unsafe {
            launch_ordered_target_stats(
                  cat_col_i32.ptr_raw() as *const c_void,
                  target.ptr_raw() as *const c_void,
                  perm_i32.ptr_raw() as *const c_void,
                  out.ptr_raw(),
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
