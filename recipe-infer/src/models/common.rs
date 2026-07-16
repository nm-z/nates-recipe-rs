//! Shared driver for the causal-dense architectures whose decode graph has been
//! verified 1:1 against `llama.cpp/src/models/<arch>.cpp` AND whose tensor names
//! resolve through the loader's `hf_name` mapping. Only such architectures are
//! dispatched (see [`super::dispatch`]); everything else is a hard error.
//!
//! Contract of every architecture routed here: RMSNorm, GQA + RoPE (no attention
//! bias), SiLU-gated SwiGLU FFN, one pre-attn and one pre-FFN norm, standard
//! residuals. The only per-arch variation is an optional per-head Q/K RMSNorm.
//! Anything outside this contract (LayerNorm, attention bias, sandwich norms,
//! MoE, recurrence, bidirectional/encoder attention, multimodal) is NOT handled
//! here and its architecture is not routed.

use super::super::{Arena, Model, layer_name};
use anyhow::Result;
use gpu_core::infer_ops::{
	gpu_gemm_bt_f64, gpu_gqa_attn, gpu_rmsnorm_f64, gpu_rope_partial, gpu_scale_f64_inplace,
};
use gpu_core::kernels::{gpu_add_into, gpu_mul_inplace, gpu_silu_into};
use gpu_core::memory::GpuBuffer;

/// One verified causal-dense decoder block: RMSNorm -> Q/K/V proj -> (optional
/// per-head Q/K RMSNorm) -> RoPE -> scaled GQA -> o_proj -> residual -> RMSNorm
/// -> SiLU SwiGLU -> residual. Matches the `build_arch_graph` of `llama` /
/// `xverse` (`qk_norm = false`) and `qwen3` (`qk_norm = true`).
pub(super) fn causal_silu(
	m: &Model,
	l: usize,
	qk_norm: bool,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let nff = hp.nff;
	let nqh = hp.nqh;
	let nm = &m.norms[l];
	let d = &hp.dims[l];
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, nqh * d.hd, d.nkv * d.hd);
	let theta = if d.sliding {
		&m.theta_slide
	} else {
		&m.theta_full
	};
	gpu_rmsnorm_f64(h_in, &nm["input"], &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.q_proj.weight"))?,
		t,
		qd,
		ne,
		&ar.q,
	)?;
	let wk = m.stream(&layer_name(l, "self_attn.k_proj.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &wk, t, kd, ne, &ar.k)?;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.v_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.v,
	)?;
	if qk_norm {
		gpu_rmsnorm_f64(&ar.q, &nm["q_norm"], &m.eps, t * nqh, hd, &ar.q)?;
		gpu_rmsnorm_f64(&ar.k, &nm["k_norm"], &m.eps, t * nkv, hd, &ar.k)?;
	}
	gpu_rope_partial(theta, t * nqh, hd, hd, nqh, &ar.q)?;
	gpu_rope_partial(theta, t * nkv, hd, hd, nkv, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	gpu_gqa_attn(&ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, t, &ar.attn)?;
	gpu_gemm_bt_f64(
		&ar.attn,
		&m.stream(&layer_name(l, "self_attn.o_proj.weight"))?,
		t,
		ne,
		qd,
		&ar.o,
	)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64(&ar.attn_out, &nm["pre_ff"], &m.eps, t, ne, &ar.cms)?;
	gpu_gemm_bt_f64(
		&ar.cms,
		&m.stream(&layer_name(l, "mlp.gate_proj.weight"))?,
		t,
		nff,
		ne,
		&ar.g,
	)?;
	gpu_gemm_bt_f64(
		&ar.cms,
		&m.stream(&layer_name(l, "mlp.up_proj.weight"))?,
		t,
		nff,
		ne,
		&ar.u,
	)?;
	gpu_silu_into(&ar.g, t * nff, &ar.g)?;
	gpu_mul_inplace(&ar.u, t * nff, &ar.g)?;
	gpu_gemm_bt_f64(
		&ar.g,
		&m.stream(&layer_name(l, "mlp.down_proj.weight"))?,
		t,
		ne,
		nff,
		&ar.mlp0,
	)?;
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	Ok(())
}
