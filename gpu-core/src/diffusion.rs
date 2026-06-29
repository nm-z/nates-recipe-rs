use crate::hip::HipError;
use crate::kernels::{check_launch, safe_i32};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// FFI declaration — slot-for-slot with launcher in diffusionx.hip
//
// C: launch_diffusionx_entropy_gated_step(logits, canvas, accepted, renoise, bound, n_positions, vocab, stream)
//    const double*, const double*, double*, double*, double, int, int, hipStream_t

unsafe extern "C" {
	fn launch_diffusionx_entropy_gated_step(
		logits: *const c_void,
		canvas: *const c_void,
		accepted: *mut c_void,
		renoise: *mut c_void,
		bound: f64,
		n_positions: i32,
		vocab: i32,
		stream: *mut c_void,
	);
}

// ── gpu_entropy_gated_step ────────────────────────────────────────────────
// Entropy-gated discrete diffusion sampler STEP. One fused thread per position.
// logits: f64 (n_positions, vocab) row-major. canvas: f64 (n_positions,) token ids.
// entropy_bound: scalar threshold on the natural-log Shannon entropy of softmax(logits[p,:]).
//   H_p <  bound -> commit argmax (renoise 0); H_p >= bound -> keep canvas (renoise 1).
// Returns (accepted_canvas f64 (n_positions,), renoise_mask f64 (n_positions,) 0/1).
pub fn gpu_entropy_gated_step(
	logits: &GpuBuffer,
	canvas: &GpuBuffer,
	entropy_bound: f64,
	n_positions: usize,
	vocab: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let accepted = GpuBuffer::alloc(n_positions)?;
	let renoise = GpuBuffer::alloc(n_positions)?;
	unsafe {
		launch_diffusionx_entropy_gated_step(
			logits.ptr_raw() as *const c_void,
			canvas.ptr_raw() as *const c_void,
			accepted.ptr_raw(),
			renoise.ptr_raw(),
			entropy_bound,
			safe_i32(n_positions),
			safe_i32(vocab),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok((accepted, renoise))
}
