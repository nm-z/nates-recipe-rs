use crate::hip::HipError;
use crate::kernels::{check_launch, gpu_gemm, gpu_softmax_rows};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// FFI declaration — slot-for-slot with the launcher in moex.hip
//
// C: launch_moex_weighted_accumulate(ye, gate, out, n, d, n_experts, e, stream)
//    const double*, const double*, double*, int, int, int, int, hipStream_t

unsafe extern "C" {
	fn launch_moex_weighted_accumulate(
		ye: *const c_void,
		gate: *const c_void,
		out: *mut c_void,
		n: i32,
		d: i32,
		n_experts: i32,
		e: i32,
		stream: *mut c_void,
	);
}

// ── gpu_moe_route ────────────────────────────────────────────────────────────
// Dense mixture-of-experts routing forward (all experts evaluated per token).
//   hidden:   f64 (n_tokens, d_model) row-major.
//   gate_w:   f64 (d_model, n_experts).
//   expert_w: f64 (n_experts, d_model, d_model) flattened — each expert a
//             d_model→d_model linear, contiguous d_model*d_model block per expert.
// router logits = hidden @ gate_w → (n_tokens, n_experts); softmax over experts
// per token → gate probs; for each expert e, Ye = hidden @ expert_w[e]; the
// output O = Σ_e gate[:,e] * Ye, shape (n_tokens, d_model) == hidden.
pub fn gpu_moe_route(
	hidden: &GpuBuffer,
	gate_w: &GpuBuffer,
	expert_w: &GpuBuffer,
	n_tokens: usize,
	d_model: usize,
	n_experts: usize,
) -> Result<GpuBuffer, HipError> {
	let logits = gpu_gemm(hidden, gate_w, n_tokens, n_experts, d_model)?;
	let gate = gpu_softmax_rows(&logits, n_tokens, n_experts)?;

	let out = GpuBuffer::zeros_bytes(n_tokens * d_model * std::mem::size_of::<f64>())?;
	let expert_stride = d_model * d_model;
	for e in 0..n_experts {
		let we = expert_w.view(e * expert_stride, expert_stride);
		let ye = gpu_gemm(hidden, &we, n_tokens, d_model, d_model)?;
		unsafe {
			launch_moex_weighted_accumulate(
				ye.ptr_raw() as *const c_void,
				gate.ptr_raw() as *const c_void,
				out.ptr_raw(),
				n_tokens as i32,
				d_model as i32,
				n_experts as i32,
				e as i32,
				std::ptr::null_mut(),
			);
		}
		check_launch();
	}
	Ok(out)
}
