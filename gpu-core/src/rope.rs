use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// ── FFI: ropex.hip ───────────────────────────────────────────────────────────
// C: launch_ropex_qk(q, k, pos, qo, ko, n_rows, d, theta, stream)
//    const double*, const double*, const double*, double*, double*, int, int, double, hipStream_t
unsafe extern "C" {
	fn launch_ropex_qk(
		q: *const c_void,
		k: *const c_void,
		pos: *const c_void,
		qo: *mut c_void,
		ko: *mut c_void,
		n_rows: i32,
		d: i32,
		theta: f64,
		stream: *mut c_void,
	);
}

// ── gpu_rope_qk ──────────────────────────────────────────────────────────────
// Q, K: f64 (n_rows, dim) row-major. positions: f64 (n_rows,), cast to int in-kernel.
// NeoX half-split rotary: pair (i, i+dim/2) rotated by angle = pos / theta^(2i/dim).
// Same rotation applied to both Q and K. theta: base frequency (e.g. 10000.0).
// Returns freshly-allocated rotated (Q, K) of the same (n_rows, dim) shape.
pub fn gpu_rope_qk(
	q: &GpuBuffer,
	k: &GpuBuffer,
	positions: &GpuBuffer,
	n_rows: usize,
	dim: usize,
	theta: f64,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let qo = GpuBuffer::alloc(n_rows * dim)?;
	let ko = GpuBuffer::alloc(n_rows * dim)?;
	unsafe {
		launch_ropex_qk(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			positions.ptr_raw() as *const c_void,
			qo.ptr_raw(),
			ko.ptr_raw(),
			n_rows as i32,
			dim as i32,
			theta,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok((qo, ko))
}
