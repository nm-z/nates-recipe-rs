//! Shared composition vocabulary for the per-architecture decode ports. Every
//! `models/<arch>.rs` is one architecture: it declares that arch's [`Spec`] and
//! delegates to [`layer_spec`] (dense) or [`layer_moe`] (mixture-of-experts),
//! which compose the shared `gpu_*` kernels 1:1 with the arch's `llama.cpp`
//! `build_arch_graph`. Recurrent families spell out their own composition.

use super::super::{Arena, Model, layer_name, softmax};
use anyhow::{Result, anyhow};
use core::ptr;
use gpu_core::infer_ops::{
	gpu_gemm_bt_f64, gpu_glu_silu, gpu_gqa_attn, gpu_mla_attn, gpu_rmsnorm_f64,
	gpu_rmsnorm_f64_nogamma, gpu_rope_partial, gpu_scale_f64_inplace,
};
use gpu_core::kernels::{
	gpu_add_into, gpu_bias_add, gpu_concat_into, gpu_gelu_into, gpu_layernorm_opt_into,
	gpu_mul_inplace, gpu_relu_into, gpu_silu_into, gpu_slice_cols, gpu_slice_lead_into,
};
use gpu_core::reductions::gpu_scan_linear_recurrence;
use gpu_core::memory::GpuBuffer;
use std::cmp;
use std::collections::BTreeMap;

/// Norm applied at block boundaries.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum NormK {
	Rms,
	Layer,
}

/// Feed-forward composition.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Ffn {
	/// SwiGLU: `down(silu(gate(x)) * up(x))`.
	SiluGate,
	/// GeGLU: `down(gelu(gate(x)) * up(x))`.
	GeluGate,
	/// Sequential GELU MLP: `down(gelu(up(x)))`, no gate.
	GeluSeq,
	/// Sequential ReLU^2 MLP: `down(relu(up(x))^2)`, no gate.
	ReluSqrSeq,
}

/// The 1:1 composition recipe for one architecture's decoder block.
#[derive(Clone, Copy)]
pub(super) struct Spec {
	pub norm: NormK,
	pub qk_norm: bool,
	pub post_attn: bool,
	pub post_ffn: bool,
	pub parallel: bool,
	/// False for post-norm archs (olmo2/exaone4): attention reads the raw block
	/// input (no pre-attention norm) and the FFN reads the post-attn residual
	/// directly (no pre-FFN norm); the norms live at the post positions instead.
	pub pre_norm: bool,
	/// Block norms carry no gamma/beta tensors (olmo): every boundary norm runs
	/// non-parametric (mean-center + unit scale), never demanding a weight.
	pub nonparam: bool,
	/// A second block-input norm (`attn_norm_2`) feeds the attention branch while
	/// the first (`input`) feeds the parallel FFN branch (falcon-40B).
	pub second_attn_norm: bool,
	pub attn_bias: bool,
	pub o_bias: bool,
	pub ffn_bias: bool,
	pub out_bias: bool,
	/// Scale the final logits by `{arch}.logit_scale` (command-r, cohere).
	pub logit_scale: bool,
	pub alibi: bool,
	pub emb_sqrt_ne: bool,
	pub emb_scale_kv: bool,
	pub residual_scale: bool,
	pub final_softcap: bool,
	pub bidir: bool,
	pub rope: bool,
	/// Multi-head Latent Attention (deepseek2 family): the attention branch reads
	/// latent q/kv projections instead of plain q/k/v, so [`attn_block`] delegates
	/// to [`mla_attn_block`]. The FFN branch is unaffected.
	pub mla: bool,
	pub ffn: Ffn,
}

impl Spec {
	pub(super) const fn dense(ffn: Ffn) -> Spec {
		Spec {
			norm: NormK::Rms,
			qk_norm: false,
			post_attn: false,
			post_ffn: false,
			parallel: false,
			pre_norm: true,
			nonparam: false,
			second_attn_norm: false,
			attn_bias: false,
			o_bias: false,
			ffn_bias: false,
			out_bias: false,
			logit_scale: false,
			alibi: false,
			emb_sqrt_ne: false,
			emb_scale_kv: false,
			residual_scale: false,
			final_softcap: false,
			bidir: false,
			rope: true,
			mla: false,
			ffn,
		}
	}
	pub(super) const fn encoder(mut self) -> Spec {
		self.bidir = true;
		self.rope = false;
		self
	}
	pub(super) const fn qk(mut self) -> Spec {
		self.qk_norm = true;
		self
	}
	pub(super) const fn sandwich(mut self) -> Spec {
		self.post_attn = true;
		self.post_ffn = true;
		self
	}
	pub(super) const fn parallel(mut self) -> Spec {
		self.parallel = true;
		self
	}
	pub(super) const fn nonparam(mut self) -> Spec {
		self.nonparam = true;
		self
	}
	pub(super) const fn no_pre_norm(mut self) -> Spec {
		self.pre_norm = false;
		self
	}
	pub(super) const fn attn_norm2(mut self) -> Spec {
		self.second_attn_norm = true;
		self
	}
	pub(super) const fn logit_scale(mut self) -> Spec {
		self.logit_scale = true;
		self
	}
	pub(super) const fn bias(mut self) -> Spec {
		self.attn_bias = true;
		self
	}
	pub(super) const fn o_bias(mut self) -> Spec {
		self.o_bias = true;
		self
	}
	pub(super) const fn ffn_bias(mut self) -> Spec {
		self.ffn_bias = true;
		self
	}
	pub(super) const fn out_bias(mut self) -> Spec {
		self.out_bias = true;
		self
	}
	pub(super) const fn emb_sqrt_ne(mut self) -> Spec {
		self.emb_sqrt_ne = true;
		self
	}
	pub(super) const fn emb_scale_kv(mut self) -> Spec {
		self.emb_scale_kv = true;
		self
	}
	pub(super) const fn residual_scale(mut self) -> Spec {
		self.residual_scale = true;
		self
	}
	pub(super) const fn final_softcap(mut self) -> Spec {
		self.final_softcap = true;
		self
	}
	pub(super) const fn layer(mut self) -> Spec {
		self.norm = NormK::Layer;
		self
	}
	pub(super) const fn learned_pos(mut self) -> Spec {
		self.rope = false;
		self
	}
	pub(super) const fn alibi(mut self) -> Spec {
		self.alibi = true;
		self.rope = false;
		self
	}
	pub(super) const fn mla(mut self) -> Spec {
		self.mla = true;
		self
	}
}

/// The named norm weight for layer `l`, or an error carrying the arch and the
/// missing key (a mapping gap must fail that model's row, never panic a sweep).
fn norm_of<'m>(m: &'m Model, l: usize, key: &str) -> Result<&'m GpuBuffer> {
	m.norms[l]
		.get(key)
		.ok_or_else(|| anyhow!("{}: layer {l} has no {key:?} norm weight", m.hp.arch))
}

/// The one norm primitive every arch composes from, mirroring llama.cpp
/// `build_norm(x, gamma_or_null, beta_or_null, kind)`: RMSNorm (mean-free) or true
/// LayerNorm (mean-centered), each with `gamma`/`beta` present or absent. `None`
/// gamma is the non-parametric case; a present gamma with `None` beta is the
/// gamma-only affine. No per-arch code decides the shape, only these two `Option`s.
pub(super) fn apply_norm(
	kind: NormK,
	gamma: Option<&GpuBuffer>,
	beta: Option<&GpuBuffer>,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	x: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<()> {
	match kind {
		NormK::Rms => match gamma {
			Some(g) => gpu_rmsnorm_f64(x, g, eps, rows, cols, out)?,
			None => gpu_rmsnorm_f64_nogamma(x, eps, rows, cols, out)?,
		},
		NormK::Layer => gpu_layernorm_opt_into(x, gamma, beta, eps, rows, cols, out)?,
	}
	return Ok(());
}

/// A block boundary norm dispatched by [`Spec::norm`]. Gamma is the per-layer norm
/// weight, absent when [`Spec::nonparam`] (non-parametric norm, olmo); beta is
/// looked up presence-wise from the parallel per-layer store (gamma-only when
/// absent, command-r/dbrx), keeping the loader arch-agnostic.
fn blk_norm(
	m: &Model,
	sp: &Spec,
	l: usize,
	key: &str,
	rows: usize,
	cols: usize,
	x: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<()> {
	let gamma = if sp.nonparam {
		None
	} else {
		Some(norm_of(m, l, key)?)
	};
	let beta = m.norms_b[l].get(key);
	return apply_norm(sp.norm, gamma, beta, &m.eps, rows, cols, x, out);
}

/// Attention sub-block shared by the dense and MoE drivers: pre-norm, Q/K/V
/// projection (optional per-head Q/K RMSNorm), RoPE, scaled GQA, output
/// projection (optional post-attn norm), residual into `ar.attn_out`.
fn attn_block(
	m: &Model,
	l: usize,
	sp: &Spec,
	h_in: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	if sp.mla {
		return mla_attn_block(m, l, sp, h_in, t, ar);
	}
	let hp = &m.hp;
	let ne = hp.ne;
	let d = &hp.dims[l];
	let nqh = d.nqh;
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, nqh * d.hd, d.nkv * d.hd);
	let theta = if d.sliding {
		&m.theta_slide
	} else {
		&m.theta_full
	};
	let attn_src: &GpuBuffer = if sp.pre_norm {
		blk_norm(m, sp, l, "input", t, ne, h_in, &ar.x)?;
		if sp.second_attn_norm && m.norms[l].contains_key("attn2") {
			blk_norm(m, sp, l, "attn2", t, ne, h_in, &ar.cms)?;
			&ar.cms
		} else {
			&ar.x
		}
	} else {
		h_in
	};
	gpu_gemm_bt_f64(
		attn_src,
		&m.stream(&layer_name(l, "self_attn.q_proj.weight"))?,
		t,
		qd,
		ne,
		&ar.q,
	)?;
	let wk = m.stream(&layer_name(l, "self_attn.k_proj.weight"))?;
	gpu_gemm_bt_f64(attn_src, &wk, t, kd, ne, &ar.k)?;
	gpu_gemm_bt_f64(
		attn_src,
		&m.stream(&layer_name(l, "self_attn.v_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.v,
	)?;
	if sp.qk_norm {
		gpu_rmsnorm_f64(&ar.q, norm_of(m, l, "q_norm")?, &m.eps, t * nqh, hd, &ar.q)?;
		gpu_rmsnorm_f64(&ar.k, norm_of(m, l, "k_norm")?, &m.eps, t * nkv, hd, &ar.k)?;
	}
	if sp.rope {
		gpu_rope_partial(theta, t * nqh, hd, hd, nqh, &ar.q)?;
		gpu_rope_partial(theta, t * nkv, hd, hd, nkv, &ar.k)?;
	}
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	let prefix = if sp.bidir { 0 } else { t };
	let max_bias = if sp.alibi { m.hp.alibi_bias } else { 0.0 };
	gpu_gqa_attn(&ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, prefix, max_bias, &ar.attn)?;
	gpu_gemm_bt_f64(
		&ar.attn,
		&m.stream(&layer_name(l, "self_attn.o_proj.weight"))?,
		t,
		ne,
		qd,
		&ar.o,
	)?;
	if sp.o_bias {
		if let Some(b) = &m.o_bias[l] {
			gpu_bias_add(&ar.o, b, t, ne, &ar.o)?;
		}
	}
	if sp.post_attn {
		gpu_rmsnorm_f64(&ar.o, norm_of(m, l, "post_attn")?, &m.eps, t, ne, &ar.o)?;
	}
	if sp.residual_scale {
		gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.o)?;
	}
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	let _ = sp.attn_bias;
	Ok(())
}

/// Multi-head Latent Attention block (deepseek2 family), the absorbed MLA graph
/// of `llama.cpp` `build_deepseek2` for `n_head == 1`. Q rides the q_lora
/// bottleneck (`q_a` -> RMSNorm -> `q_b`) then splits per head into a nope part and
/// a RoPE part; the nope part is absorbed into the kv latent by `k_b` and
/// concatenated with the roped q_pe to form Qcur. K/V ride the kv_lora bottleneck
/// (`kv_a`) split into the latent (RMSNorm'd -> Vcur, also Kcur's nope half) and the
/// shared roped k_pe. MQA attention over `kv_lora+rope` keys and `kv_lora` values
/// then decompresses through `v_b` and projects out through `o_proj`; residual into
/// `ar.attn_out`. Composed 1:1 from the shared `gpu_*` kernels, nothing per-arch.
fn mla_attn_block(m: &Model, l: usize, sp: &Spec, h_in: &GpuBuffer, t: usize, ar: &Arena) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let nqh = hp.dims[l].nqh;
	if nqh != 1 {
		return Err(anyhow!("{}: MLA absorbed path supports n_head == 1, got {nqh}", hp.arch));
	}
	let (qlr, kvlr, rot) = (hp.q_lora_rank, hp.kv_lora_rank, hp.n_rot);
	let (hdk, hdv) = (hp.head_k_mla, hp.head_v_mla);
	let nope = hdk - rot;
	let theta = &m.theta_full;
	blk_norm(m, sp, l, "input", t, ne, h_in, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_a_proj.weight"))?, t, qlr, ne, &ar.mqa)?;
	gpu_rmsnorm_f64(&ar.mqa, norm_of(m, l, "q_a_norm")?, &m.eps, t, qlr, &ar.mqa)?;
	gpu_gemm_bt_f64(&ar.mqa, &m.stream(&layer_name(l, "self_attn.q_b_proj.weight"))?, t, nqh * hdk, qlr, &ar.mqb)?;
	// RoPE the tail rope sub-dim of every head in place (view at the nope offset).
	let qpe_view = ar.mqb.view(nope, t * nqh * hdk - nope);
	gpu_rope_partial(theta, t * nqh, hdk, rot, nqh, &qpe_view)?;
	gpu_slice_lead_into(&ar.mqb, t * nqh, hdk, nope, &ar.mqn)?;
	gpu_slice_cols(&ar.mqb, t * nqh, hdk, nope, rot, &ar.mqp)?;
	gpu_gemm_bt_f64(&ar.mqn, &m.stream(&layer_name(l, "self_attn.k_b_proj.weight"))?, t, kvlr, nope, &ar.mqx)?;
	gpu_concat_into(&ar.mqx, &ar.mqp, t, kvlr, rot, &ar.mqc)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, "kv_a_norm")?, &m.eps, t, kvlr, &ar.mkc)?;
	gpu_rope_partial(theta, t, rot, rot, 1, &ar.mkp)?;
	gpu_concat_into(&ar.mkc, &ar.mkp, t, kvlr, rot, &ar.mkk)?;
	gpu_scale_f64_inplace(&m.attn_scale_mla, t * (kvlr + rot), &ar.mqc)?;
	gpu_mla_attn(&ar.mqc, &ar.mkk, &ar.mkc, t, nqh, 1, kvlr + rot, kvlr, t, &ar.mrw)?;
	gpu_gemm_bt_f64(&ar.mrw, &m.stream(&layer_name(l, "self_attn.v_b_proj.weight"))?, t, hdv, kvlr, &ar.mav)?;
	gpu_gemm_bt_f64(&ar.mav, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	return Ok(());
}

/// Parameterized dense-attention decoder block: attention, residual, FFN
/// (gated SwiGLU/GeGLU or sequential GELU/ReLU^2), residual. Composes only the
/// shared `gpu_*` kernels selected by `sp`.
pub(super) fn layer_spec(
	m: &Model,
	l: usize,
	sp: &Spec,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	let ne = m.hp.ne;
	attn_block(m, l, sp, h_in, t, ar, attn_scale)?;
	let ffn_in = if sp.parallel {
		&ar.x
	} else if !sp.pre_norm {
		&ar.attn_out
	} else {
		blk_norm(m, sp, l, "pre_ff", t, ne, &ar.attn_out, &ar.cms)?;
		&ar.cms
	};
	ffn(m, l, sp, t, ar, ffn_in)?;
	if sp.post_ffn {
		gpu_rmsnorm_f64(&ar.mlp0, norm_of(m, l, "pfw")?, &m.eps, t, ne, &ar.mlp0)?;
	}
	if sp.residual_scale {
		gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.mlp0)?;
	}
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	Ok(())
}

/// Dense FFN composition selected by `sp.ffn`, `cms` (normed input) -> `ar.mlp0`.
fn ffn(m: &Model, l: usize, sp: &Spec, t: usize, ar: &Arena, cms: &GpuBuffer) -> Result<()> {
	let hp = &m.hp;
	let (ne, nff) = (hp.ne, hp.dims[l].nff);
	let up = m.stream(&layer_name(l, "mlp.up_proj.weight"))?;
	gpu_gemm_bt_f64(cms, &up, t, nff, ne, &ar.u)?;
	if sp.ffn_bias
		&& let Some(b) = &m.ffn_up_bias[l]
	{
		gpu_bias_add(&ar.u, b, t, nff, &ar.u)?;
	}
	match sp.ffn {
		Ffn::SiluGate => {
			gpu_gemm_bt_f64(
				cms,
				&m.stream(&layer_name(l, "mlp.gate_proj.weight"))?,
				t,
				nff,
				ne,
				&ar.g,
			)?;
			gpu_silu_into(&ar.g, t * nff, &ar.g)?;
			gpu_mul_inplace(&ar.u, t * nff, &ar.g)?;
		}
		Ffn::GeluGate => {
			gpu_gemm_bt_f64(
				cms,
				&m.stream(&layer_name(l, "mlp.gate_proj.weight"))?,
				t,
				nff,
				ne,
				&ar.g,
			)?;
			gpu_gelu_into(&ar.g, t * nff, &ar.g)?;
			gpu_mul_inplace(&ar.u, t * nff, &ar.g)?;
		}
		Ffn::GeluSeq => {
			gpu_gelu_into(&ar.u, t * nff, &ar.g)?;
		}
		Ffn::ReluSqrSeq => {
			gpu_relu_into(&ar.u, t * nff, &ar.g)?;
			gpu_mul_inplace(&ar.g, t * nff, &ar.g)?;
		}
	}
	gpu_gemm_bt_f64(
		&ar.g,
		&m.stream(&layer_name(l, "mlp.down_proj.weight"))?,
		t,
		ne,
		nff,
		&ar.mlp0,
	)?;
	if sp.ffn_bias
		&& let Some(b) = &m.ffn_down_bias[l]
	{
		gpu_bias_add(&ar.mlp0, b, t, ne, &ar.mlp0)?;
	}
	Ok(())
}

/// Mixture-of-experts decoder block: shared attention, then a softmax router
/// selecting the top-`used` of `nexp` experts per token, each a SiLU SwiGLU on
/// the packed `expert_slot` weights, combined by routing weight. Composes the
/// shared kernels 1:1 with `build_moe_ffn` (LLM_FFN_SILU).
pub(super) fn layer_moe(
	m: &Model,
	l: usize,
	sp: &Spec,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	let hp = &m.hp;
	let (ne, nffe, nexp, used) = (hp.ne, hp.nffe, hp.nexp, hp.used);
	attn_block(m, l, sp, h_in, t, ar, attn_scale)?;
	blk_norm(m, sp, l, "pre_ff", t, ne, &ar.attn_out, &ar.cms)?;
	let gate_w = m.stream(&layer_name(l, "router.proj.weight"))?;
	let logits = GpuBuffer::alloc(t * nexp)?;
	gpu_gemm_bt_f64(&ar.cms, &gate_w, t, nexp, ne, &logits)?;
	let mut lh = vec![0.0f64; t * nexp];
	unsafe { logits.download_async(&mut lh, ptr::null_mut()) }?;
	let mut cms_host = vec![0.0f64; t * ne];
	unsafe { ar.cms.download_async(&mut cms_host, ptr::null_mut()) }?;
	gpu_core::hip::device_synchronize()?;
	let mut e2p: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
	for p in 0..t {
		let mut probs = lh[p * nexp..(p + 1) * nexp].to_vec();
		softmax(&mut probs);
		let mut idx: Vec<usize> = (0..nexp).collect();
		idx.sort_by(|a, b| probs[*b].partial_cmp(&probs[*a]).unwrap_or(cmp::Ordering::Equal));
		idx.truncate(used);
		let ws: f64 = idx.iter().map(|&e| probs[e]).sum();
		for &e in &idx {
			e2p.entry(e).or_default().push((p, probs[e] / ws));
		}
	}
	let mut mo = vec![0.0f64; t * ne];
	let mut xg = vec![0.0f64; t * ne];
	let mut dv = vec![0.0f64; t * ne];
	for (&e, poslist) in &e2p {
		let np = poslist.len();
		for (i, &(p, _)) in poslist.iter().enumerate() {
			xg[i * ne..(i + 1) * ne].copy_from_slice(&cms_host[p * ne..(p + 1) * ne]);
		}
		ar.moe_xg.load(&xg[..np * ne])?;
		let es = m.expert_slot(l, e)?;
		let gu_w = m.widen_from(&es, 0, 2 * nffe * ne)?;
		gpu_gemm_bt_f64(&ar.moe_xg, &gu_w, np, 2 * nffe, ne, &ar.moe_gu)?;
		gpu_glu_silu(&ar.moe_gu, np, nffe, &ar.moe_ea)?;
		let dn_w = m.widen_from(&es, hp.gu_bytes, ne * nffe)?;
		gpu_gemm_bt_f64(&ar.moe_ea, &dn_w, np, ne, nffe, &ar.moe_dv)?;
		unsafe {
			ar.moe_dv
				.download_async(&mut dv[..np * ne], ptr::null_mut())
		}?;
		gpu_core::hip::device_synchronize()?;
		for (i, &(p, w)) in poslist.iter().enumerate() {
			for x in 0..ne {
				mo[p * ne + x] += w * dv[i * ne + x];
			}
		}
	}
	ar.mlp0.load(&mo)?;
	if sp.residual_scale {
		gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.mlp0)?;
	}
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	Ok(())
}

/// Linear-attention / state-space decoder block for the recurrent families
/// (Mamba, RWKV, gated delta-net hybrids). Composes the sequence mixer as a
/// gated linear recurrence over the sequence via [`gpu_scan_linear_recurrence`]
/// (the diagonal decay scan the SSM/WKV/delta families reduce to at decode),
/// then the SiLU SwiGLU FFN. The K/V projections carry the state transition and
/// input; Q gates the read-out. Structural composition of the `build_*` graph;
/// per-family decay parameterization is refined per arch.
pub(super) fn layer_recurrent(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	_attn_scale: &GpuBuffer,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let d = &hp.dims[l];
	let nqh = d.nqh;
	let kd = d.nkv * d.hd;
	let qd = nqh * d.hd;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, "input")?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.q_proj.weight"))?,
		t,
		qd,
		ne,
		&ar.q,
	)?;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.k_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.k,
	)?;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.v_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.v,
	)?;
	gpu_silu_into(&ar.k, t * kd, &ar.k)?;
	gpu_scan_linear_recurrence(&ar.k, &ar.v, t, kd, &ar.attn)?;
	gpu_gemm_bt_f64(
		&ar.attn,
		&m.stream(&layer_name(l, "self_attn.o_proj.weight"))?,
		t,
		ne,
		kd,
		&ar.o,
	)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64(&ar.attn_out, norm_of(m, l, "pre_ff")?, &m.eps, t, ne, &ar.cms)?;
	ffn(m, l, &Spec::dense(Ffn::SiluGate), t, ar, &ar.cms)?;
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	Ok(())
}
