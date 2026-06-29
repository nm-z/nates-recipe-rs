use crate::hip::HipError;
use crate::kernels::{check_launch, gpu_copy_into, safe_i32};
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
	fn launch_diffusionx_commit(
		canvas: *mut c_void,
		accepted: *const c_void,
		renoise: *const c_void,
		committed: *mut c_void,
		n: i32,
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

// ── gpu_diffusion_sample ─────────────────────────────────────────────────────
// Block-autoregressive entropy-gated diffusion decoding loop. Each iteration:
//   1. `logits_fn(canvas)` produces the model's logits for the current canvas
//      (the committed tokens condition the still-open positions);
//   2. the entropy-gated step commits the confident positions of this block;
//   3. committed positions are FROZEN; the loop repeats on the rest.
// Terminates when every position is committed or `max_steps` is reached. Returns
// the final canvas and the number of steps taken. `initial_canvas` holds the
// starting token ids (any sentinel for "open"); it is not mutated.
pub fn gpu_diffusion_sample(
	mut logits_fn: impl FnMut(&GpuBuffer) -> Result<GpuBuffer, HipError>,
	initial_canvas: &GpuBuffer,
	entropy_bound: f64,
	max_steps: usize,
	n_positions: usize,
	vocab: usize,
) -> Result<(GpuBuffer, usize), HipError> {
	let canvas = GpuBuffer::alloc(n_positions)?;
	gpu_copy_into(initial_canvas, &canvas, n_positions);
	let committed = GpuBuffer::zeros_bytes(n_positions * std::mem::size_of::<f64>())?;
	let mut host = vec![0.0f64; n_positions];
	let mut steps = 0usize;
	for s in 0..max_steps {
		steps = s + 1;
		let logits = logits_fn(&canvas)?;
		let (accepted, renoise) =
			gpu_entropy_gated_step(&logits, &canvas, entropy_bound, n_positions, vocab)?;
		unsafe {
			launch_diffusionx_commit(
				canvas.ptr_raw(),
				accepted.ptr_raw() as *const c_void,
				renoise.ptr_raw() as *const c_void,
				committed.ptr_raw(),
				safe_i32(n_positions),
				std::ptr::null_mut(),
			);
		}
		check_launch();
		committed.download(&mut host)?;
		if host.iter().all(|&c| c != 0.0) {
			break;
		}
	}
	Ok((canvas, steps))
}

#[cfg(test)]
mod tests {
	use super::*;

	// The block-autoregressive loop must commit progressively: a position becomes
	// confident only once its predecessors are committed (its logits depend on the
	// canvas), so exactly one position commits per step and the loop converges in
	// `n` steps with every position decoded — proving iteration, the freeze of
	// committed positions, and convergence detection.
	#[test]
	fn diffusion_block_ar_progressive_commit() {
		crate::hip::set_device(0).expect("set_device");
		let (n, vocab) = (5usize, 4usize);
		let bound = 0.5; // 0 < bound < ln(vocab)=1.386 → peaked=confident, uniform=uncertain
		let initial = GpuBuffer::upload(&vec![-1.0f64; n]).expect("init"); // -1 = open slot

		// Position p is confident (peaked at class 0) iff p <= #committed; otherwise
		// uniform logits (entropy ln(vocab) > bound). Committed positions hold a real
		// token id (>= 0); open positions hold the -1 sentinel.
		let logits_fn = |canvas: &GpuBuffer| -> Result<GpuBuffer, HipError> {
			let mut c = vec![0.0f64; n];
			canvas.download(&mut c)?;
			let committed_count = c.iter().filter(|&&v| v >= 0.0).count();
			let mut logits = vec![0.0f64; n * vocab];
			for p in 0..n {
				if p <= committed_count {
					logits[p * vocab] = 10.0; // peaked → entropy ~0 < bound
				}
			}
			GpuBuffer::upload(&logits)
		};

		let (canvas, steps) =
			gpu_diffusion_sample(logits_fn, &initial, bound, 100, n, vocab).expect("sample");
		let mut out = vec![0.0f64; n];
		canvas.download(&mut out).expect("dl");
		eprintln!("diffusion block-AR: steps={steps} canvas={out:?}");
		assert_eq!(steps, n, "one position commits per step → n steps");
		assert!(out.iter().all(|&v| v == 0.0), "every position decoded to argmax class 0");
	}
}
