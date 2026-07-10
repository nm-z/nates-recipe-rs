use crate::hip::{HipError, check};
use crate::infer_ops::gpu_gemm_bt_f64;
use crate::kernels::{
	gpu_add_inplace, gpu_copy_into, gpu_gemm, gpu_gemm_at,
	gpu_softmax_backward_into, gpu_softmax_rows_into,
};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

fn cl() -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	check(unsafe { crate::hip::hipGetLastError() })
}


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

pub fn gpu_moe_weighted_accumulate(
	ye: &GpuBuffer,
	gate: &GpuBuffer,
	n: usize,
	d: usize,
	n_experts: usize,
	e: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_moex_weighted_accumulate(
			ye.ptr_raw() as *const c_void,
			gate.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			d as i32,
			n_experts as i32,
			e as i32,
			std::ptr::null_mut(),
		);
	}
	cl()
}

pub fn gpu_moe_weighted_accumulate_backward(
	d_out: &GpuBuffer,
	gate: &GpuBuffer,
	ye: &GpuBuffer,
	n: usize,
	d: usize,
	n_experts: usize,
	e: usize,
	d_ye: &GpuBuffer,
	d_gate: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_moex_weighted_accumulate_backward(
			d_out.ptr_raw() as *const c_void,
			gate.ptr_raw() as *const c_void,
			ye.ptr_raw() as *const c_void,
			d_ye.ptr_raw(),
			d_gate.ptr_raw(),
			n as i32,
			d as i32,
			n_experts as i32,
			e as i32,
			std::ptr::null_mut(),
		);
	}
	cl()
}

pub fn gpu_moe_route(
	hidden: &GpuBuffer,
	gate_w: &GpuBuffer,
	expert_w: &GpuBuffer,
	n_tokens: usize,
	d_model: usize,
	n_experts: usize,
) -> Result<GpuBuffer, HipError> {
	let logits = GpuBuffer::alloc(n_tokens * n_experts)?;
	gpu_gemm(hidden, gate_w, n_tokens, n_experts, d_model, &logits)?;
	let gate = GpuBuffer::alloc(n_tokens * n_experts)?;
	gpu_softmax_rows_into(&logits, n_tokens, n_experts, &gate)?;

	let out = GpuBuffer::alloc_bytes(n_tokens * d_model * std::mem::size_of::<f64>())?;
	out.memset_zero(n_tokens * d_model * std::mem::size_of::<f64>())?;
	let expert_stride = d_model * d_model;
	let ye = GpuBuffer::alloc(n_tokens * d_model)?;
	for e in 0..n_experts {
		let we = expert_w.view(e * expert_stride, expert_stride);
		gpu_gemm(hidden, &we, n_tokens, d_model, d_model, &ye)?;
		gpu_moe_weighted_accumulate(&ye, &gate, n_tokens, d_model, n_experts, e, &out)?;
	}
	Ok(out)
}

pub fn gpu_moe_backward(
	hidden: &GpuBuffer,
	gate_w: &GpuBuffer,
	expert_w: &GpuBuffer,
	d_out: &GpuBuffer,
	n_tokens: usize,
	d_model: usize,
	n_experts: usize,
) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
	let logits = GpuBuffer::alloc(n_tokens * n_experts)?;
	gpu_gemm(hidden, gate_w, n_tokens, n_experts, d_model, &logits)?;
	let gate = GpuBuffer::alloc(n_tokens * n_experts)?;
	gpu_softmax_rows_into(&logits, n_tokens, n_experts, &gate)?;
	let expert_stride = d_model * d_model;

	let d_hidden = GpuBuffer::alloc_bytes(n_tokens * d_model * std::mem::size_of::<f64>())?;
	d_hidden.memset_zero(n_tokens * d_model * std::mem::size_of::<f64>())?;
	let d_gate = GpuBuffer::alloc(n_tokens * n_experts)?;
	let d_expert_w = GpuBuffer::alloc(n_experts * expert_stride)?;
	let d_ye = GpuBuffer::alloc(n_tokens * d_model)?;
	let ye = GpuBuffer::alloc(n_tokens * d_model)?;
	let dh_e = GpuBuffer::alloc(n_tokens * d_model)?;
	let dwe = GpuBuffer::alloc(expert_stride)?;

	for e in 0..n_experts {
		let we = expert_w.view(e * expert_stride, expert_stride);
		gpu_gemm(hidden, &we, n_tokens, d_model, d_model, &ye)?;
		gpu_moe_weighted_accumulate_backward(
			d_out, &gate, &ye, n_tokens, d_model, n_experts, e, &d_ye, &d_gate,
		)?;
		gpu_gemm_bt_f64(&d_ye, &we, n_tokens, d_model, d_model, &dh_e)?;
		gpu_add_inplace(&dh_e, n_tokens * d_model, &d_hidden)?;
		gpu_gemm_at(hidden, &d_ye, d_model, d_model, n_tokens, &dwe)?;
		gpu_copy_into(&dwe, expert_stride, &d_expert_w.view(e * expert_stride, expert_stride))?;
	}

	let d_logits = GpuBuffer::alloc(n_tokens * n_experts)?;
	gpu_softmax_backward_into(&d_gate, &gate, n_tokens, n_experts, &d_logits)?;
	let dh_r = GpuBuffer::alloc(n_tokens * d_model)?;
	gpu_gemm_bt_f64(&d_logits, gate_w, n_tokens, d_model, n_experts, &dh_r)?;
	gpu_add_inplace(&dh_r, n_tokens * d_model, &d_hidden)?;
	let d_gate_w = GpuBuffer::alloc(d_model * n_experts)?;
	gpu_gemm_at(hidden, &d_logits, d_model, n_experts, n_tokens, &d_gate_w)?;

	Ok((d_hidden, d_gate_w, d_expert_w))
}
