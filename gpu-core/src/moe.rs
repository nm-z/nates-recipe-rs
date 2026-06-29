use crate::hip::HipError;
use crate::kernels::{
	check_launch, gpu_add_inplace, gpu_copy_into, gpu_gemm, gpu_gemm_at, gpu_gemm_bt,
	gpu_softmax_backward_into, gpu_softmax_rows,
};
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
	fn launch_moex_weighted_accumulate_backward(
		d_out: *const c_void,
		gate: *const c_void,
		ye: *const c_void,
		d_ye: *mut c_void,
		d_gate: *mut c_void,
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

// ── gpu_moe_backward ─────────────────────────────────────────────────────────
// Backward through every op of gpu_moe_route. Given d_out (n_tokens, d_model),
// returns (d_hidden, d_gate_w, d_expert_w) matching the forward inputs' shapes.
// Recomputes the forward intermediates (gate, per-expert Ye) so it is fully
// self-contained. Reuses the dense GEMM/softmax-backward kernels; the only new
// op is the weighted-accumulate backward.
//   per expert e: d_ye = gate[:,e]·d_out ; d_gate[:,e] = Σ_j Ye[:,j]·d_out[:,j]
//                 d_hidden += d_ye · Weᵀ ; d_We = hiddenᵀ · d_ye
//   router: d_logits = softmax_bwd(d_gate, gate)
//           d_hidden += d_logits · gate_wᵀ ; d_gate_w = hiddenᵀ · d_logits
pub fn gpu_moe_backward(
	hidden: &GpuBuffer,
	gate_w: &GpuBuffer,
	expert_w: &GpuBuffer,
	d_out: &GpuBuffer,
	n_tokens: usize,
	d_model: usize,
	n_experts: usize,
) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
	let logits = gpu_gemm(hidden, gate_w, n_tokens, n_experts, d_model)?;
	let gate = gpu_softmax_rows(&logits, n_tokens, n_experts)?;
	let expert_stride = d_model * d_model;

	let d_hidden = GpuBuffer::zeros_bytes(n_tokens * d_model * std::mem::size_of::<f64>())?;
	let d_gate = GpuBuffer::alloc(n_tokens * n_experts)?;
	let d_expert_w = GpuBuffer::alloc(n_experts * expert_stride)?;
	let d_ye = GpuBuffer::alloc(n_tokens * d_model)?;

	for e in 0..n_experts {
		let we = expert_w.view(e * expert_stride, expert_stride);
		let ye = gpu_gemm(hidden, &we, n_tokens, d_model, d_model)?;
		unsafe {
			launch_moex_weighted_accumulate_backward(
				d_out.ptr_raw() as *const c_void,
				gate.ptr_raw() as *const c_void,
				ye.ptr_raw() as *const c_void,
				d_ye.ptr_raw(),
				d_gate.ptr_raw(),
				n_tokens as i32,
				d_model as i32,
				n_experts as i32,
				e as i32,
				std::ptr::null_mut(),
			);
		}
		check_launch();
		// d_hidden += d_ye · Weᵀ
		let dh_e = gpu_gemm_bt(&d_ye, &we, n_tokens, d_model, d_model)?;
		gpu_add_inplace(&d_hidden, &dh_e, n_tokens * d_model);
		// d_We = hiddenᵀ · d_ye  →  d_expert_w[e]
		let dwe = gpu_gemm_at(hidden, &d_ye, d_model, d_model, n_tokens)?;
		gpu_copy_into(&dwe, &d_expert_w.view(e * expert_stride, expert_stride), expert_stride);
	}

	let d_logits = GpuBuffer::alloc(n_tokens * n_experts)?;
	gpu_softmax_backward_into(&d_gate, &gate, &d_logits, n_tokens, n_experts);
	let dh_r = gpu_gemm_bt(&d_logits, gate_w, n_tokens, d_model, n_experts)?;
	gpu_add_inplace(&d_hidden, &dh_r, n_tokens * d_model);
	let d_gate_w = gpu_gemm_at(hidden, &d_logits, d_model, n_experts, n_tokens)?;

	Ok((d_hidden, d_gate_w, d_expert_w))
}

#[cfg(test)]
mod tests {
	use super::*;

	// Backward through every MoE op must match a finite difference of the forward
	// for the loss L = Σ (out ⊙ G): the analytic d_gate_w / d_expert_w / d_hidden
	// from gpu_moe_backward (with d_out = G) equal (L(θ+ε) − L(θ−ε)) / 2ε.
	#[test]
	fn moe_backward_matches_finite_diff() {
		crate::hip::set_device(0).expect("set_device");
		let (n, d, e) = (5usize, 4usize, 3usize);
		let mk = |seed: usize, len: usize, scale: f64| -> Vec<f64> {
			(0..len)
				.map(|i| (((i * 1103515245 + seed * 12345) % 1000) as f64 / 1000.0 - 0.5) * scale)
				.collect()
		};
		let hidden = mk(1, n * d, 1.0);
		let gate_w = mk(2, d * e, 1.0);
		let expert_w = mk(3, e * d * d, 1.0);
		let g = mk(4, n * d, 1.0); // upstream grad = d_out

		let hb = GpuBuffer::upload(&hidden).expect("h");
		let gwb = GpuBuffer::upload(&gate_w).expect("gw");
		let ewb = GpuBuffer::upload(&expert_w).expect("ew");
		let gb = GpuBuffer::upload(&g).expect("g");
		let (d_hidden, d_gate_w, d_expert_w) =
			gpu_moe_backward(&hb, &gwb, &ewb, &gb, n, d, e).expect("bwd");
		let dl = |buf: &GpuBuffer, len: usize| -> Vec<f64> {
			let mut v = vec![0.0f64; len];
			buf.download(&mut v).expect("download");
			v
		};
		let dh = dl(&d_hidden, n * d);
		let dgw = dl(&d_gate_w, d * e);
		let dew = dl(&d_expert_w, e * d * d);

		let eps = 1e-6;
		let loss = |hh: &[f64], gw: &[f64], ew: &[f64]| -> f64 {
			let out = gpu_moe_route(
				&GpuBuffer::upload(hh).expect("h"),
				&GpuBuffer::upload(gw).expect("gw"),
				&GpuBuffer::upload(ew).expect("ew"),
				n,
				d,
				e,
			)
			.expect("fwd");
			let mut o = vec![0.0f64; n * d];
			out.download(&mut o).expect("download out");
			o.iter().zip(&g).map(|(a, b)| a * b).sum()
		};
		let fd = |base: &[f64], idx: usize, which: u8| -> f64 {
			let mut p = base.to_vec();
			let mut m = base.to_vec();
			p[idx] += eps;
			m[idx] -= eps;
			match which {
				0 => (loss(&p, &gate_w, &expert_w) - loss(&m, &gate_w, &expert_w)) / (2.0 * eps),
				1 => (loss(&hidden, &p, &expert_w) - loss(&hidden, &m, &expert_w)) / (2.0 * eps),
				_ => (loss(&hidden, &gate_w, &p) - loss(&hidden, &gate_w, &m)) / (2.0 * eps),
			}
		};
		let mut maxdiff = 0.0f64;
		for i in 0..n * d {
			maxdiff = maxdiff.max((fd(&hidden, i, 0) - dh[i]).abs());
		}
		for i in 0..d * e {
			maxdiff = maxdiff.max((fd(&gate_w, i, 1) - dgw[i]).abs());
		}
		for i in 0..e * d * d {
			maxdiff = maxdiff.max((fd(&expert_w, i, 2) - dew[i]).abs());
		}
		eprintln!("moe backward vs finite-diff: maxdiff = {maxdiff:e}");
		assert!(maxdiff < 1e-6, "moe backward != finite diff: {maxdiff:e}");
	}
}
